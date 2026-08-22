import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';

import { chromium } from 'playwright';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const outputRoot = resolve(projectRoot, 'dist/static');
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/static-worker.json');
const server = await startStaticServer(outputRoot);
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ serviceWorkers: 'allow' });
const page = await context.newPage();
const browserErrors = [];
const httpFailures = [];
const serviceWorkerErrors = [];

context.on('serviceworker', (worker) => {
    worker.on('console', (message) => {
        if (message.type() === 'error') serviceWorkerErrors.push(message.text());
    });
});

page.on('pageerror', (error) => browserErrors.push(error.message));
page.on('console', (message) => {
    if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) {
        browserErrors.push(message.text());
    } else if (message.text().startsWith('[cem-studio:feature-tour]')) {
        console.log(message.text());
    }
});
page.on('response', (response) => {
    if (response.status() >= 400) httpFailures.push(`${response.status()} ${response.url()}`);
});

try {
    await page.goto(`${server.url}/index.html`, { waitUntil: 'load' });
    await waitForApplication(page, browserErrors, httpFailures);
    console.log('[cem-studio:static-worker] application ready');
    const initialFeatureTourStatus = await page.evaluate(
        () => globalThis.__cemStudioApplication.featureTour.status,
    );
    const serviceWorker = await page.evaluate(async () => {
        const registration = await navigator.serviceWorker.register('./service-worker.js', {
            scope: './',
            type: 'module',
            updateViaCache: 'none',
        });
        const ready = await Promise.race([
            navigator.serviceWorker.ready.then(() => true),
            new Promise((resolveReady) => setTimeout(() => resolveReady(false), 30_000)),
        ]);
        return {
            ready,
            controlled: navigator.serviceWorker.controller !== null,
            active: registration.active?.state,
            installing: registration.installing?.state,
            waiting: registration.waiting?.state,
        };
    });
    assert.equal(
        serviceWorker.ready,
        true,
        `service worker did not become ready: ${JSON.stringify({ serviceWorker, serviceWorkerErrors, httpFailures })}`,
    );
    if (!serviceWorker.controlled) {
        await page.reload({ waitUntil: 'load' });
        await waitForApplication(page, browserErrors, httpFailures);
    }
    assert.ok(await page.evaluate(() => navigator.serviceWorker.controller !== null));
    console.log('[cem-studio:static-worker] service worker controlling page');

    const shell = await inspectShell(page);
    assert.equal(shell.state, 'ready');
    assert.equal(shell.theme, 'cem-theme-contrast-dark');
    assert.equal(shell.controlsOnly, true);
    assert.equal(shell.componentsInstalled, true);
    console.log('[cem-studio:static-worker] shell verified');

    const caches = await page.evaluate(() => globalThis.caches.keys());
    assert.deepEqual(caches.sort(), [
        'cem-studio:0.1.0:runtime',
        'cem-studio:0.1.0:samples',
        'cem-studio:0.1.0:shell',
    ]);
    console.log('[cem-studio:static-worker] deployment caches verified');

    const projections = await exerciseProjections(page);
    assert.deepEqual(projections.parse.map(({ mode }) => mode), ['ast', 'events']);
    assert.deepEqual(
        projections.inspect.map(({ mode }) => mode),
        ['summary', 'ast', 'events', 'diagnostics', 'source-offsets', 'tree'],
    );
    for (const projection of [...projections.parse, ...projections.inspect]) {
        assert.equal(projection.status, 'projected');
        assert.equal(projection.projectRevision, 1);
        assert.equal(projection.resourceRevision, 1);
        assert.equal(projection.runtime, 'wasm-browser-worker');
        assert.equal(projection.nativeKind, projection.kind);
        assert.equal(projection.stale, false);
        assert.ok(projection.contentType.includes('cem'));
        assert.ok(projection.byteLength > 0);
        assert.equal(projection.sha256.length, 64);
        assert.ok(projection.text.includes('@doc cem-ml 1'));
    }
    assert.equal(projections.componentsOnly, true);
    assert.equal(projections.controlsInstalled, true);

    const commandView = await exerciseCommandView(page);
    assert.equal(commandView.projection, 'studio');
    assert.equal(commandView.initialStatus, 'current');
    assert.equal(commandView.changedStatus, 'changed');
    assert.ok(commandView.changeCount > 0);
    assert.ok(commandView.changeCategories.includes('operation'));
    assert.equal(commandView.copyStatus, 'success');
    assert.equal(commandView.copiedExactDraft, true);
    assert.equal(commandView.unresolvedCode, 'cem.browser_command.resolve');
    assert.equal(commandView.invalidStatus, 'invalid');
    assert.equal(commandView.invalidCode, 'cem.command.unknown_option');
    assert.equal(commandView.resetStatus, 'current');
    assert.equal(commandView.repositoryUnchanged, true);
    assert.equal(commandView.componentsOnly, true);
    assert.equal(commandView.controlsInstalled, true);

    const commandApply = await exerciseCommandApplyAndRun(page);
    assert.equal(commandApply.recommendedTarget, 'new');
    assert.equal(commandApply.applyStatus, 'applied');
    assert.equal(commandApply.applyExecuted, false);
    assert.equal(commandApply.runStatus, 'ran');
    assert.equal(commandApply.disposition, 'updated');
    assert.equal(commandApply.projectRevision, 3);
    assert.equal(commandApply.resourceRevision, 2);
    assert.equal(commandApply.executionProjectRevision, commandApply.projectRevision);
    assert.equal(commandApply.executionResourceRevision, commandApply.resourceRevision);
    assert.equal(commandApply.exactReload, true);
    assert.equal(commandApply.runtime, 'wasm-browser-worker');
    assert.equal(commandApply.componentsOnly, true);
    assert.equal(commandApply.controlsInstalled, true);

    const portableOperations = await exercisePortableOperations(page);
    assert.deepEqual(portableOperations.results.map(({ kind }) => kind), [
        'convert',
        'query',
        'transform',
        'trace',
        'transform',
    ]);
    assert.ok(portableOperations.results.every(({ expectedMatches }) => expectedMatches));
    assert.ok(portableOperations.results.every(({ runtime }) => runtime === 'wasm-browser-worker'));
    assert.ok(portableOperations.results.every(({ stale }) => !stale));
    assert.ok(portableOperations.results.every(({ copiedExact, downloadedExact }) => copiedExact && downloadedExact));
    assert.ok(portableOperations.results.find(({ mode }) => mode === 'graph').graphCount > 0);
    assert.ok(portableOperations.results.find(({ kind }) => kind === 'trace').traceCount > 0);
    assert.equal(portableOperations.componentsOnly, true);
    assert.equal(portableOperations.controlsInstalled, true);

    const featureTour = await validateFeatureTour(page, initialFeatureTourStatus);
    assert.equal(featureTour.initialStatus, 'installed');
    assert.ok(['installed', 'preserved'].includes(featureTour.status));
    assert.equal(featureTour.seedId, 'cem-ml-feature-tour-seed');
    assert.equal(featureTour.projectId, 'feature-tour');
    assert.equal(featureTour.exampleCount, 31);
    assert.equal(featureTour.validatedCount, featureTour.exampleCount);
    assert.equal(featureTour.cachedSampleResponses, featureTour.cacheUrlCount);

    const workbench = await exerciseWorkbench(page);
    assert.equal(workbench.status, 'invalid');
    assert.equal(workbench.projectRevision, 2);
    assert.equal(workbench.resourceRevision, 2);
    assert.equal(workbench.exactReload, true);
    assert.ok(workbench.diagnosticCount > 0);
    assert.ok(workbench.hardViolationCount > 0);
    assert.ok(workbench.provenanceCount > 0);
    assert.equal(workbench.runtime, 'wasm-browser-worker');
    assert.equal(workbench.componentsOnly, true);
    assert.equal(workbench.reportVisible, true);
    assert.equal(workbench.diagnosticSelection.kind, 'diagnostic');
    assert.ok(workbench.diagnosticSelection.byteLength > 0);
    assert.equal(workbench.provenanceSelection.kind, 'provenance');

    const stored = await importOfflineProject(page);
    assert.equal(stored.repositoryRevision, 3);
    const onlineOfflineProjectProjections = await projectOfflineResource(page);
    assert.equal(onlineOfflineProjectProjections.parse.status, 'projected');
    assert.equal(onlineOfflineProjectProjections.inspect.status, 'projected');

    const online = await executeVersionCommand(page, 'static-worker-online');
    assert.equal(online.exitCode, 0);
    assert.equal(online.runtime, 'wasm-browser-worker');
    assert.equal(online.executorTopology, 'browser-worker-pool');

    await context.setOffline(true);
    await page.goto(`${server.url}/projects/offline-project`, { waitUntil: 'load' });
    await waitForApplication(page, browserErrors, httpFailures);
    assert.ok(await page.evaluate(() => navigator.serviceWorker.controller !== null));
    assert.equal(await page.getAttribute('[data-cem-studio-root]', 'data-theme'), 'cem-theme-contrast-dark');
    const recovered = await exportOfflineProject(page);
    assert.equal(recovered.content, '{main | Offline project bytes}\n');
    assert.equal(recovered.projectId, 'offline-project');
    const recoveredWorkbench = await recoverWorkbenchOffline(page, workbench.content);
    assert.equal(recoveredWorkbench.status, 'invalid');
    assert.equal(recoveredWorkbench.projectRevision, 2);
    assert.equal(recoveredWorkbench.resourceRevision, 2);
    assert.equal(recoveredWorkbench.content, workbench.content);
    assert.equal(recoveredWorkbench.exactContent, true);
    assert.ok(recoveredWorkbench.diagnosticCount > 0);
    assert.ok(recoveredWorkbench.provenanceCount > 0);
    assert.equal(recoveredWorkbench.runtime, 'wasm-browser-worker');
    const recoveredCommandView = await exerciseOfflineCommandView(page);
    assert.equal(recoveredCommandView.status, 'changed');
    assert.ok(recoveredCommandView.changeCount > 0);
    assert.equal(recoveredCommandView.projection, 'studio');
    assert.equal(recoveredCommandView.resetStatus, 'current');
    const recoveredPortableOperations = await exercisePortableOperations(page);
    assert.deepEqual(
        recoveredPortableOperations.results.map(({ sha256 }) => sha256),
        portableOperations.results.map(({ sha256 }) => sha256),
    );
    assert.ok(recoveredPortableOperations.results.every(({ expectedMatches }) => expectedMatches));
    const recoveredProjections = await projectOfflineResource(page);
    assert.equal(recoveredProjections.parse.sha256, onlineOfflineProjectProjections.parse.sha256);
    assert.equal(recoveredProjections.parse.text, onlineOfflineProjectProjections.parse.text);
    assert.equal(recoveredProjections.inspect.sha256, onlineOfflineProjectProjections.inspect.sha256);
    assert.equal(recoveredProjections.inspect.text, onlineOfflineProjectProjections.inspect.text);
    assert.equal(recoveredProjections.parse.runtime, 'wasm-browser-worker');
    assert.equal(recoveredProjections.inspect.runtime, 'wasm-browser-worker');
    const offline = await executeVersionCommand(page, 'static-worker-offline');
    assert.equal(offline.exitCode, 0);
    assert.equal(offline.runtime, 'wasm-browser-worker');
    assert.equal(offline.executorTopology, 'browser-worker-pool');
    assert.deepEqual(browserErrors, []);
    assert.deepEqual(httpFailures, []);

    const report = {
        schemaVersion: 1,
        project: '@epa-wg/cem-studio',
        source: 'graph-emitted-static-output',
        shell,
        caches,
        indexedDbSurvival: recovered,
        featureTour,
        workbench,
        projections,
        commandView,
        commandApply,
        portableOperations,
        onlineOfflineProjectProjections,
        recoveredWorkbench,
        recoveredCommandView,
        recoveredPortableOperations,
        recoveredProjections,
        offlineNavigation: '/projects/offline-project',
        online,
        offline,
    };
    await mkdir(dirname(reportPath), { recursive: true });
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.log('Verified graph-emitted CEM-ML CLI worker and WASM command online and offline.');
} finally {
    await context.setOffline(false).catch(() => undefined);
    await page.evaluate(async () => {
        const repository = globalThis.__cemStudioAcceptanceRepository;
        globalThis.__cemStudioApplication?.repository.close();
        await globalThis.__cemStudioApplication?.validator.close();
        repository?.close();
        await repository?.deleteDatabase();
    }).catch(() => undefined);
    await browser.close();
    await server.close();
}

async function waitForApplication(page, browserErrors, httpFailures) {
    try {
        await page.waitForFunction(
            () => Boolean(globalThis.__cemStudioApplication),
            undefined,
            { timeout: 30_000 },
        );
    } catch (error) {
        const diagnostic = await page.evaluate(async () => ({
            documentReadyState: document.readyState,
            rootState: document.querySelector('[data-cem-studio-root]')?.getAttribute('data-cem-studio-state'),
            rootMounted: document.querySelector('[data-cem-studio-root]')?.getAttribute('data-cem-studio-mounted'),
            cacheNames: await caches.keys(),
            serviceWorkers: (await navigator.serviceWorker.getRegistrations()).map((registration) => ({
                active: registration.active?.state,
                installing: registration.installing?.state,
                waiting: registration.waiting?.state,
            })),
            indexedDbDatabases: typeof indexedDB.databases === 'function'
                ? await indexedDB.databases()
                : [],
            resources: performance.getEntriesByType('resource').map(({ name }) => name),
        }));
        const details = [
            ...browserErrors.map((message) => `browser: ${message}`),
            ...httpFailures.map((message) => `http: ${message}`),
            `diagnostic: ${JSON.stringify(diagnostic)}`,
        ];
        throw new Error(`${error.message}${details.length > 0 ? `\n${details.join('\n')}` : ''}`, { cause: error });
    }
}

async function validateFeatureTour(page, initialStatus) {
    return page.evaluate(async (startupStatus) => {
        const application = globalThis.__cemStudioApplication;
        const { catalog, bundle } = application.seed;
        let validatedCount = 0;
        for (const [index, example] of catalog.examples.entries()) {
            console.log(`[cem-studio:feature-tour] validating ${index + 1}/${catalog.exampleCount}: ${example.id}`);
            const controller = new AbortController();
            const timeout = setTimeout(
                () => controller.abort(`Feature Tour validation timed out: ${example.id}`),
                60_000,
            );
            try {
                await application.validator.validateResource({
                    bytes: bundle.contents[example.resourceId],
                    contentType: example.contentType,
                    schema: example.schema,
                    uri: example.path,
                    dependencies: example.dependencies.map((dependency) => ({
                        bytes: bundle.contents[dependency.resourceId],
                        contentType: dependency.contentType,
                        schema: dependency.schema,
                        path: dependency.path,
                    })),
                    signal: controller.signal,
                });
            } finally {
                clearTimeout(timeout);
            }
            validatedCount += 1;
        }
        const sampleCache = await caches.open('cem-studio:0.1.0:samples');
        return {
            initialStatus: startupStatus,
            status: application.featureTour.status,
            seedId: catalog.seed.id,
            projectId: application.featureTour.projectId,
            exampleCount: catalog.exampleCount,
            dependencyCount: catalog.dependencyCount,
            cacheUrlCount: catalog.cacheUrlCount,
            validatedCount,
            cachedSampleResponses: (await sampleCache.keys()).length,
        };
    }, initialStatus);
}

async function exerciseWorkbench(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const invalid = '@doc cem-ml 1\n\n{article @id="broken" |\n    {h1 | Missing article close}\n';
        application.workbench.updateDraft(invalid);
        const snapshot = await application.workbench.saveAndValidate();
        const exported = await application.repository.query({
            protocolVersion: 1,
            repository: 'studio-projects',
            operation: 'export-project',
            requestRevision: 1,
            parameters: { projectId: application.featureTour.projectId },
        });
        const diagnosticIndex = snapshot.validation.diagnostics.findIndex(({ range }) => range.len > 0);
        if (diagnosticIndex < 0) throw new Error('saved revision diagnostics have no navigable source range');
        const diagnosticSelection = application.workbench.navigateDiagnostic(diagnosticIndex);
        const provenanceSelection = application.workbench.navigateProvenance(0);
        await application.workbenchView.whenSettled();
        const resource = exported.value.project.resources.find(({ id }) => id === snapshot.resourceId);
        const persisted = new TextDecoder().decode(exported.value.contents[snapshot.resourceId]);
        return {
            status: snapshot.status,
            projectRevision: snapshot.projectRevision,
            resourceRevision: snapshot.resourceRevision,
            repositoryRevision: snapshot.repositoryRevision,
            content: invalid,
            exactReload: persisted === invalid
                && exported.value.project.revision === snapshot.projectRevision
                && resource.revision === snapshot.resourceRevision,
            diagnosticCount: snapshot.validation.diagnostics.length,
            hardViolationCount: snapshot.validation.hardViolationCount,
            provenanceCount: snapshot.validation.provenance.length,
            runtime: snapshot.validation.executionIdentity.runtime,
            diagnosticSelection,
            provenanceSelection,
            componentsOnly: document.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
            ).length === 0,
            reportVisible: Boolean(document.querySelector('[data-cem-studio-workbench] cem-table [role="table"]')),
        };
    });
}

async function recoverWorkbenchOffline(page, expectedContent) {
    return page.evaluate(async (content) => {
        const application = globalThis.__cemStudioApplication;
        const snapshot = await application.workbench.validatePersisted();
        await application.workbenchView.whenSettled();
        return {
            status: snapshot.status,
            projectRevision: snapshot.projectRevision,
            resourceRevision: snapshot.resourceRevision,
            content: snapshot.persistedText,
            exactContent: snapshot.persistedText === content,
            diagnosticCount: snapshot.validation.diagnostics.length,
            provenanceCount: snapshot.validation.provenance.length,
            runtime: snapshot.validation.executionIdentity.runtime,
        };
    }, expectedContent);
}

async function exerciseProjections(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const project = async (kind, mode) => {
            const snapshot = kind === 'parse'
                ? await application.workbench.parsePersisted(mode)
                : await application.workbench.inspectPersisted(mode);
            const projection = snapshot.projection;
            const nativeOperation = projection.nativeResult.result?.storage === 'inline'
                ? projection.nativeResult.result.value
                : undefined;
            return {
                kind,
                mode,
                status: snapshot.status,
                projectRevision: projection.revision.projectRevision,
                resourceRevision: projection.revision.resourceRevision,
                runtime: projection.executionIdentity.runtime,
                nativeKind: nativeOperation?.kind,
                stale: projection.stale,
                contentType: projection.output.contentType,
                byteLength: projection.output.byteLength,
                sha256: projection.output.sha256,
                text: projection.output.text,
                diagnosticCount: projection.diagnostics.length,
                provenanceCount: projection.provenance.length,
            };
        };
        const parse = [];
        for (const mode of ['ast', 'events']) parse.push(await project('parse', mode));
        const inspect = [];
        for (const mode of ['summary', 'ast', 'events', 'diagnostics', 'source-offsets', 'tree']) {
            inspect.push(await project('inspect', mode));
        }
        await application.workbenchView.whenSettled();
        return {
            parse,
            inspect,
            componentsOnly: document.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
            ).length === 0,
            controlsInstalled: Boolean(
                document.querySelector('[data-cem-studio-workbench] cem-select .cem-select__control')
                && document.querySelector('[data-cem-studio-workbench] cem-action button')
                && document.querySelector('[data-cem-studio-workbench] cem-textarea[readonly] textarea')
                && document.querySelector('[data-cem-studio-workbench] cem-tabs [role="tablist"]')
                && document.querySelector('[data-cem-studio-workbench] cem-table [role="table"]')
            ),
        };
    });
}

async function exercisePortableOperations(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const initialWorkbenchId = application.workbench.snapshot().workbenchId;
        const results = [];
        for (const scenario of application.seed.catalog.workbenches) {
            await application.workbench.selectWorkbench(scenario.id);
            const snapshot = await application.workbench.runPersistedOperation();
            const projection = snapshot.projection;
            let copied;
            let downloaded;
            await application.workbench.copyProjection(async (text) => {
                copied = text;
            });
            await application.workbench.downloadProjection(async (file) => {
                downloaded = file;
            });
            results.push({
                scenario: scenario.id,
                kind: projection.kind,
                mode: projection.mode,
                projectRevision: projection.revision.projectRevision,
                resourceRevision: projection.revision.resourceRevision,
                runtime: projection.executionIdentity.runtime,
                stale: projection.stale,
                expectedMatches: projection.expectedMatches,
                summary: projection.summary,
                outputContentType: projection.output.contentType,
                outputByteLength: projection.output.byteLength,
                sha256: projection.output.sha256,
                traceCount: projection.trace.length,
                graphCount: projection.graph.length,
                copiedExact: copied === projection.output.text,
                downloadedExact: downloaded?.contentType === projection.output.contentType
                    && downloaded?.bytes.byteLength === projection.output.byteLength
                    && downloaded.bytes.every((byte, index) => byte === projection.output.bytes[index]),
            });
        }
        await application.workbench.selectWorkbench(initialWorkbenchId);
        await application.workbenchView.whenSettled();
        return {
            results,
            componentsOnly: document.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
            ).length === 0,
            controlsInstalled: Boolean(
                document.querySelector('[data-cem-studio-workbench-select] .cem-select__control')
                && document.querySelector('[data-cem-studio-operation-run] button')
                && document.querySelector('[data-cem-studio-projection-copy] button')
                && document.querySelector('[data-cem-studio-projection-download] button')
                && document.querySelector('[data-cem-studio-projection-expected]')
                && document.querySelector('[data-cem-studio-projection-trace]')
                && document.querySelector('[data-cem-studio-projection-graph]')
            ),
        };
    });
}

async function exerciseCommandView(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const before = await application.repository.query({
            protocolVersion: 1,
            repository: 'studio-projects',
            operation: 'export-project',
            requestRevision: 1,
            parameters: { projectId: application.featureTour.projectId },
        });
        const initial = application.workbench.snapshot().command;
        const changedText = initial.current.text.replace('--show tree', '--show summary');
        if (changedText === initial.current.text) throw new Error('command fixture could not select inspect summary');
        const editor = document.querySelector('cem-textarea[data-cem-studio-command-editor] textarea');
        editor.value = changedText;
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await application.workbenchView.whenSettled();
        const changed = application.workbench.snapshot().command;
        const changesTableVisible = Boolean(
            document.querySelector('cem-table[label="CLI Command semantic changes"] [role="table"]'),
        );

        let copied = '';
        Object.defineProperty(navigator.clipboard, 'writeText', {
            configurable: true,
            value: async (text) => {
                copied = text;
            },
        });
        document.querySelector('cem-action[data-cem-studio-command-copy] button').click();
        await application.workbenchView.whenSettled();
        const copiedState = application.workbench.snapshot().command;

        const inputUri = Object.values(initial.current.parsed.positionals)
            .find((value) => typeof value === 'string' && value.startsWith('studio:'));
        if (!inputUri) throw new Error('generated command has no Studio input URI');
        editor.value = changedText.replace(inputUri, 'studio://feature-tour/missing.cem');
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await application.workbenchView.whenSettled();
        const unresolved = application.workbench.snapshot().command;

        editor.value = `${changedText} --not-a-cem-option`;
        editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await application.workbenchView.whenSettled();
        const invalid = application.workbench.snapshot().command;
        document.querySelector('cem-action[data-cem-studio-command-reset] button').click();
        await application.workbenchView.whenSettled();
        const reset = application.workbench.snapshot().command;
        const after = await application.repository.query({
            protocolVersion: 1,
            repository: 'studio-projects',
            operation: 'export-project',
            requestRevision: 1,
            parameters: { projectId: application.featureTour.projectId },
        });
        return {
            projection: initial.projection,
            initialStatus: initial.status,
            changedStatus: changed.status,
            changeCount: changed.changes.length,
            changeCategories: [...new Set(changed.changes.map(({ category }) => category))],
            copyStatus: copiedState.copy.status,
            copiedExactDraft: copied === changedText,
            unresolvedCode: unresolved.diagnostic.code,
            invalidStatus: invalid.status,
            invalidCode: invalid.diagnostic.code,
            resetStatus: reset.status,
            repositoryUnchanged: before.repositoryRevision === after.repositoryRevision
                && before.value.project.revision === after.value.project.revision,
            componentsOnly: document.querySelectorAll(
                '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
            ).length === 0,
            controlsInstalled: Boolean(
                document.querySelector('cem-textarea[data-cem-studio-command-editor] textarea')
                && document.querySelector('cem-action[data-cem-studio-command-copy] button')
                && document.querySelector('cem-action[data-cem-studio-command-reset] button')
                && changesTableVisible
            ),
        };
    });
}

async function exerciseCommandApplyAndRun(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const [{ createCemStudioProjectRepository }, { createCemStudioFeatureTourCopy }, { createCemStudioFeatureTourWorkbench }] = await Promise.all([
            import('@epa-wg/cem-studio/repository'),
            import('@epa-wg/cem-studio/feature-tour'),
            import('@epa-wg/cem-studio/workbench'),
        ]);
        const repository = createCemStudioProjectRepository({
            databaseName: `cem-studio-command-apply-${crypto.randomUUID()}`,
            validateProject: application.validator.validateProject,
            now: () => '2026-08-22T00:00:00Z',
        });
        let workbench;
        try {
            const projectId = 'command-apply-acceptance';
            await repository.execute({
                protocolVersion: 1,
                repository: 'studio-projects',
                operation: 'import-project',
                requestRevision: 1,
                parameters: {
                    bundle: createCemStudioFeatureTourCopy(application.seed, {
                        projectId,
                        now: '2026-08-22T00:00:00Z',
                    }),
                    mode: 'create',
                },
            });
            workbench = await createCemStudioFeatureTourWorkbench({
                repository,
                validator: application.validator,
                seed: application.seed,
                projectId,
            });
            const recommendedTarget = workbench.snapshot().command.application.target.mode;
            const applied = await workbench.applyCommand();
            const applyExecuted = Boolean(applied.projection);
            const ran = await workbench.applyAndRun();
            const result = ran.command.application.result;
            const exported = await repository.query({
                protocolVersion: 1,
                repository: 'studio-projects',
                operation: 'export-project',
                requestRevision: 1,
                parameters: { projectId },
            });
            const storedBytes = exported.value.contents[result.commandResource.id];
            const exactReload = storedBytes.byteLength === result.commandBytes.byteLength
                && new Uint8Array(storedBytes).every((byte, index) =>
                    byte === new Uint8Array(result.commandBytes)[index]);
            return {
                recommendedTarget,
                applyStatus: applied.command.application.status,
                applyExecuted,
                runStatus: ran.command.application.status,
                disposition: result.disposition,
                projectRevision: result.projectRevision,
                resourceRevision: result.resourceRevision,
                executionProjectRevision: ran.command.application.execution.projectRevision,
                executionResourceRevision: ran.command.application.execution.resourceRevision,
                exactReload,
                runtime: ran.projection.executionIdentity.runtime,
                componentsOnly: document.querySelectorAll(
                    '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
                ).length === 0,
                controlsInstalled: Boolean(
                    document.querySelector('[data-cem-studio-workbench] cem-text-field input')
                    && document.querySelector('[data-cem-studio-workbench] cem-select .cem-select__control')
                    && document.querySelector('[data-cem-studio-workbench] cem-action button')
                ),
            };
        } finally {
            workbench?.dispose();
            repository.close();
            await repository.deleteDatabase();
        }
    });
}

async function exerciseOfflineCommandView(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const initial = application.workbench.snapshot().command;
        const changedText = initial.current.text.replace('--format ast', '--format events');
        const changed = await application.workbench.updateCommandDraft(changedText);
        const command = changed.command;
        const reset = await application.workbench.resetCommandDraft();
        await application.workbenchView.whenSettled();
        return {
            projection: command.projection,
            status: command.status,
            changeCount: command.changes.length,
            resetStatus: reset.command.status,
        };
    });
}

async function projectOfflineResource(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const { createCemStudioFeatureTourWorkbench } = await import('@epa-wg/cem-studio/workbench');
        const workbench = await createCemStudioFeatureTourWorkbench({
            repository: application.repository,
            validator: application.validator,
            seed: application.seed,
            projectId: 'offline-project',
            example: {
                resourceId: 'source',
                path: 'source.cem',
                contentType: 'application/cem',
                schema: 'https://cem.dev/ns/cem-ml/1',
                dependencies: [],
            },
        });
        const parseSnapshot = await workbench.parsePersisted('ast');
        const parse = {
            status: parseSnapshot.status,
            runtime: parseSnapshot.projection.executionIdentity.runtime,
            sha256: parseSnapshot.projection.output.sha256,
            text: parseSnapshot.projection.output.text,
        };
        const inspectSnapshot = await workbench.inspectPersisted('tree');
        const inspect = {
            status: inspectSnapshot.status,
            runtime: inspectSnapshot.projection.executionIdentity.runtime,
            sha256: inspectSnapshot.projection.output.sha256,
            text: inspectSnapshot.projection.output.text,
        };
        workbench.dispose();
        return { parse, inspect };
    });
}

async function inspectShell(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        application.shell.theme.setMode('cem-theme-contrast-dark');
        await Promise.all([
            'cem-app-bar',
            'cem-action',
            'cem-select',
            'cem-badge',
            'cem-alert',
            'cem-textarea',
            'cem-tabs',
            'cem-list',
            'cem-table',
        ].map(
            (tag) => customElements.whenDefined(tag),
        ));
        return {
            state: document.querySelector('[data-cem-studio-root]')?.getAttribute('data-cem-studio-state'),
            theme: document.querySelector('[data-cem-studio-root]')?.getAttribute('data-theme'),
            controlsOnly: document.querySelectorAll(
                'button:not(cem-action button):not(cem-select button):not(cem-tabs button)',
            ).length === 0,
            componentsInstalled: Boolean(
                document.querySelector('cem-app-bar header')
                && document.querySelector('cem-action button')
                && document.querySelector('cem-select .cem-select__control')
                && document.querySelector('cem-badge .cem-badge')
                && document.querySelector('cem-alert .cem-alert')
                && document.querySelector('cem-textarea .cem-textarea__control')
                && document.querySelector('cem-tabs [role="tablist"]')
            ),
        };
    });
}

async function importOfflineProject(page) {
    return page.evaluate(async () => {
        const { createCemStudioProjectRepository } = await import('@epa-wg/cem-studio/repository');
        const repository = createCemStudioProjectRepository({ validateProject: async (bundle) => bundle });
        globalThis.__cemStudioAcceptanceRepository = repository;
        const content = new TextEncoder().encode('{main | Offline project bytes}\n');
        const digest = await crypto.subtle.digest('SHA-256', content);
        const sha256 = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
        const result = await repository.execute({
            protocolVersion: 1,
            repository: 'studio-projects',
            operation: 'import-project',
            requestRevision: 1,
            parameters: {
                bundle: {
                    project: {
                        $schema: 'https://cem.dev/ns/studio/project/1',
                        schemaVersion: 1,
                        id: 'offline-project',
                        name: 'Offline survival fixture',
                        rootUri: 'studio://offline-project/',
                        revision: 1,
                        createdAt: '2026-08-21T00:00:00Z',
                        updatedAt: '2026-08-21T00:00:00Z',
                        entries: [],
                        resources: [{
                            id: 'source',
                            role: 'data',
                            sourceKind: 'project-file',
                            path: 'source.cem',
                            contentType: 'application/cem',
                            schema: 'https://cem.dev/ns/cem-ml/1',
                            revision: 1,
                            sha256,
                        }],
                    },
                    contents: { source: content },
                },
            },
        });
        return { repositoryRevision: result.repositoryRevision };
    });
}

async function exportOfflineProject(page) {
    return page.evaluate(async () => {
        const { createCemStudioProjectRepository } = await import('@epa-wg/cem-studio/repository');
        const repository = createCemStudioProjectRepository({ validateProject: async (bundle) => bundle });
        globalThis.__cemStudioAcceptanceRepository = repository;
        const result = await repository.query({
            protocolVersion: 1,
            repository: 'studio-projects',
            operation: 'export-project',
            requestRevision: 1,
            parameters: { projectId: 'offline-project' },
        });
        return {
            projectId: result.value.project.id,
            content: new TextDecoder().decode(result.value.contents.source),
            repositoryRevision: result.repositoryRevision,
        };
    });
}

async function executeVersionCommand(page, requestId) {
    return page.evaluate(async (id) => {
        const api = await import('@epa-wg/cem-ml-cli/browser');
        const meta = api.parseCemMlCommand(['version'], { runtime: 'wasm-browser-worker' });
        const command = {
            schemaVersion: meta.schemaVersion,
            commonVersion: meta.commonVersion,
            commandPath: ['version'],
            globalOptions: meta.globalOptions,
            options: {},
            positionals: {},
        };
        const invocation = await api.buildBrowserCommandInvocation(
            command,
            async () => {
                throw new Error('the version command must not request a project resource');
            },
            { requestId: id },
        );
        const host = {
            currentRevision: async ({ project }) => ({ project, resourceVersions: {} }),
            readResource: async () => {
                throw new Error('the version command must not read a project resource');
            },
            prepareWrite: async () => {
                throw new Error('the version command must not prepare a write');
            },
            commitWrite: async () => {
                throw new Error('the version command must not commit a write');
            },
            rollbackWrite: async () => undefined,
        };
        const client = await api.createBrowserCommandServiceClient({ host });
        try {
            const result = await client.execute(invocation.request).result();
            api.projectBrowserCommandPresentation(invocation.presentation, result);
            return {
                requestId: result.requestId,
                exitCode: result.exitCode,
                status: result.status,
                runtime: client.capability.runtime,
                executorTopology: client.capability.executorTopology,
                commonVersion: client.worker.commonVersion,
            };
        } finally {
            await client.close();
        }
    }, requestId);
}

function startStaticServer(root) {
    return new Promise((resolveServer, reject) => {
        const server = createServer(async (request, response) => {
            try {
                const requestPath = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
                const candidate = resolve(root, `.${requestPath === '/' ? '/index.html' : requestPath}`);
                const rootPrefix = root.endsWith(sep) ? root : `${root}${sep}`;
                if (candidate !== root && !candidate.startsWith(rootPrefix)) {
                    response.writeHead(403).end('Forbidden');
                    return;
                }
                const metadata = await stat(candidate);
                const path = metadata.isDirectory() ? resolve(candidate, 'index.html') : candidate;
                const bytes = await readFile(path);
                response.writeHead(200, {
                    'content-type': contentType(path),
                    'cache-control': 'no-store',
                    'service-worker-allowed': '/',
                });
                response.end(bytes);
            } catch {
                response.writeHead(404).end('Not found');
            }
        });
        server.on('error', reject);
        server.listen(0, '127.0.0.1', () => {
            const address = server.address();
            if (!address || typeof address === 'string') {
                reject(new Error('static server did not expose a TCP address'));
                return;
            }
            resolveServer({
                url: `http://127.0.0.1:${address.port}`,
                close: () =>
                    new Promise((resolveClose, rejectClose) => {
                        server.close((error) => (error ? rejectClose(error) : resolveClose()));
                    }),
            });
        });
    });
}

function contentType(path) {
    return (
        {
            '.css': 'text/css; charset=utf-8',
            '.html': 'text/html; charset=utf-8',
            '.js': 'text/javascript; charset=utf-8',
            '.json': 'application/json; charset=utf-8',
            '.svg': 'image/svg+xml',
            '.wasm': 'application/wasm',
            '.webmanifest': 'application/manifest+json',
        }[extname(path)] ?? 'application/octet-stream'
    );
}
