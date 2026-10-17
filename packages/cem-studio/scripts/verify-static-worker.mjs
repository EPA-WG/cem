import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { dirname, extname, resolve, sep } from 'node:path';

import { chromium } from 'playwright';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const outputRoot = resolve(projectRoot, 'dist/static');
const lifecycleOnly = process.argv.includes('--lifecycle-only');
const shellOnly = process.argv.includes('--shell-only');
const presentationOnly = process.argv.includes('--presentation-only');
const reportPath = resolve(workspaceRoot, `dist/reports/cem-studio/${presentationOnly ? 'presentation-samples' : lifecycleOnly ? 'workspace-delivery' : shellOnly ? 'shell-delivery' : 'static-worker'}.json`);
const server = await startStaticServer(outputRoot);
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const context = await browser.newContext({ serviceWorkers: 'allow' });
const page = await context.newPage();
page.setDefaultTimeout(30_000);
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
    const initialResponse = await page.goto(`${server.url}/index.html`, { waitUntil: 'load' });
    assert.ok(initialResponse);
    await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication));
    assert.equal(await page.evaluate(async () => (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'list-projects', parameters: {} })).value.length), 0);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projectId), undefined);
    await waitForApplication(page, browserErrors, httpFailures);
    console.log('[cem-studio:static-worker] application ready');
    const initialFeatureTourStatus = await page.evaluate(() => globalThis.__cemStudioFixture.featureTour.status);
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
    assert.equal(shell.fileSystemControlsInstalled, true);
    assert.equal(shell.explorerStyled, true);
    assert.equal(shell.themesStyled, true, JSON.stringify({ themeStyles: shell.themeStyles, styleDiagnostics: shell.styleDiagnostics, styles: shell.styles, browserErrors, httpFailures }));
    assert.equal(shell.workbenchStyled, true, JSON.stringify(shell.workbenchStyles));
    assert.equal(shell.workbenchOverrides, true, JSON.stringify(shell.overrideStyles));
    assert.equal(shell.commandStyled, true, JSON.stringify(shell.commandStyles));
    assert.equal(shell.commandOverrides, true, JSON.stringify(shell.overrideStyles));
    assert.equal(shell.previewStyled, true, JSON.stringify(shell.previewStyles));
    assert.equal(shell.previewOverrides, true, JSON.stringify(shell.overrideStyles));
    assert.equal(shell.inheritedOverrides, true, JSON.stringify(shell.overrideStyles));
    assert.deepEqual(shell.errors, []);
    assert.ok(['unbound', 'unsupported'].includes(shell.fileSystemState));
    console.log('[cem-studio:static-worker] shell verified');
    const retainedEditor = await page.locator('[data-cem-studio-editor] textarea').elementHandle();
    const retainedCommand = await page.locator('[data-cem-studio-command-editor] textarea').elementHandle();
    for (const width of [390, 1280]) {
        await page.setViewportSize({ width, height: 900 });
        await retainedEditor.evaluate(element => { element.focus(); element.setSelectionRange(1, 4); });
        await page.getByRole('tab', { name: 'Results', exact: true }).click();
        await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
        assert.equal(await retainedEditor.evaluate(element => element.isConnected && element.closest('[role="tabpanel"]').hidden), true);
        await page.getByRole('tab', { name: 'Source and command', exact: true }).click();
        await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
        assert.deepEqual(await retainedEditor.evaluate(element => [element === document.querySelector('[data-cem-studio-editor] textarea'), element.selectionStart, element.selectionEnd]), [true, 1, 4]);
        assert.equal(await retainedCommand.evaluate(element => element === document.querySelector('[data-cem-studio-command-editor] textarea')), true);
    }
    await retainedEditor.dispose(); await retainedCommand.dispose();
    if (!shellOnly && !lifecycleOnly && !presentationOnly) {
        await page.getByRole('textbox', { name: 'Project name', exact: true }).fill('Deployed starter');
        await page.getByRole('button', { name: 'Create CEM-ML project', exact: true }).click();
        await page.waitForFunction(() => globalThis.__cemStudioApplication.workbench.snapshot().home.projects.some(project => project.name === 'Deployed starter'));
        const starter = await page.evaluate(async () => {
            const app = globalThis.__cemStudioApplication;
            const choice = app.workbench.snapshot().home.projects.find(project => project.name === 'Deployed starter');
            const projectId = JSON.parse(choice.id).projectId;
            const response = await app.repository.query({ protocolVersion: 1, repository: 'studio-projects', operation: 'export-project', requestRevision: 1, parameters: { projectId } });
            return { id: projectId, resources: response.value.project.resources.length, entries: response.value.project.entries.length };
        });
        assert.ok(starter.id.startsWith('project-'));
        assert.equal(starter.resources, 1);
        assert.equal(starter.entries, 0);
    }

    if (presentationOnly) {
        await exerciseExampleProjects(page, context);
        assert.deepEqual(browserErrors, []); assert.deepEqual(httpFailures, []);
        await mkdir(dirname(reportPath), { recursive: true });
        await writeFile(reportPath, `${JSON.stringify({ schemaVersion: 1, exampleProjects: true, validationRepair: true, offline: true, configurations: 7 }, null, 2)}\n`);
        console.log('Verified deployed example project copies, schema failure/repair, source selection, Save/reload and seven offline configurations.');
    } else if (lifecycleOnly) {
        const lifecycle = await exerciseProjectLifecycle(page, context);
        assert.deepEqual(browserErrors, []); assert.deepEqual(httpFailures, []);
        await mkdir(dirname(reportPath), { recursive: true });
        await writeFile(reportPath, `${JSON.stringify({ schemaVersion: 1, source: 'graph-emitted-static-output', browser: { engine: 'chromium', version: browser.version(), platform: process.platform }, shell, lifecycle }, null, 2)}\n`, 'utf8');
        console.log('Verified deployed two-project lifecycle and automated accessibility acceptance.');
    } else if (shellOnly) {
        const hardening = await inspectHardening(page, initialResponse);
        assert.equal(hardening.shellForcedColors, true);
        assert.equal(hardening.cspDefaultDeny, true);
        assert.equal(hardening.cspNoUnsafeScript, true);
        assert.equal(hardening.cspStyleAttributesBlocked, true);
        await context.setOffline(true);
        await page.reload({ waitUntil: 'load' });
        await waitForApplication(page, browserErrors, httpFailures);
        assert.equal(await page.getAttribute('[data-cem-studio-shell]', 'data-theme'), 'cem-theme-contrast-dark');
        const offline = await inspectShell(page);
        assert.equal(offline.state, 'ready');
        assert.equal(offline.componentsInstalled, true);
        assert.equal(offline.themesStyled, true);
        assert.equal(offline.inheritedOverrides, true);
        assert.deepEqual(offline.errors, []);
        assert.deepEqual(browserErrors, []);
        assert.deepEqual(httpFailures, []);
        await mkdir(dirname(reportPath), { recursive: true });
        await writeFile(reportPath, `${JSON.stringify({ schemaVersion: 1, shell, hardening, offline }, null, 2)}\n`, 'utf8');
        console.log('Verified declarative shell delivery, CEM theme inheritance, forced colors and offline reload.');
    } else {
    const caches = await page.evaluate(() => globalThis.caches.keys());
    assert.deepEqual(caches.sort(), ['cem-studio:0.1.0:runtime', 'cem-studio:0.1.0:samples', 'cem-studio:0.1.0:shell']);
    console.log('[cem-studio:static-worker] deployment caches verified');

    const projections = await exerciseProjections(page);
    assert.deepEqual(
        projections.parse.map(({ mode }) => mode),
        ['ast', 'events'],
    );
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
    assert.deepEqual(
        portableOperations.results.map(({ kind }) => kind),
        ['convert', 'query', 'transform', 'trace', 'transform'],
    );
    assert.ok(portableOperations.results.every(({ expectedMatches }) => expectedMatches));
    assert.ok(portableOperations.results.every(({ runtime }) => runtime === 'wasm-browser-worker'));
    assert.ok(portableOperations.results.every(({ stale }) => !stale));
    assert.ok(portableOperations.results.every(({ copiedExact, downloadedExact }) => copiedExact && downloadedExact));
    assert.ok(portableOperations.results.every(({ traceRows, traceCount, graphRows, graphCount }) =>
        traceRows === Math.min(100, traceCount) && graphRows === Math.min(100, graphCount)), JSON.stringify(portableOperations.results));
    assert.ok(portableOperations.results.find(({ mode }) => mode === 'graph').graphCount > 0);
    assert.ok(portableOperations.results.find(({ kind }) => kind === 'trace').traceCount > 0);
    assert.equal(portableOperations.componentsOnly, true);
    assert.equal(portableOperations.controlsInstalled, true);

    const featureTour = await validateFeatureTour(page, initialFeatureTourStatus);
    assert.equal(featureTour.initialStatus, 'installed');
    assert.ok(['installed', 'preserved'].includes(featureTour.status));
    assert.equal(featureTour.seedId, 'cem-ml-feature-tour-seed');
    assert.equal(featureTour.projectId, 'feature-tour');
    assert.equal(featureTour.exampleCount, 32);
    assert.equal(featureTour.validatedCount, featureTour.exampleCount);
    assert.equal(featureTour.cachedSampleResponses, featureTour.cacheUrlCount + 3);
    assert.equal(featureTour.exampleProjectCached, true);

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

    const hardening = await inspectHardening(page, initialResponse);
    assert.equal(hardening.cspDefaultDeny, true);
    assert.equal(hardening.cspNoUnsafeScript, true);
    assert.equal(hardening.cspStyleAttributesBlocked, true);
    assert.equal(hardening.forcedColors, true);
    assert.equal(hardening.workbenchForcedColors, true);
    assert.equal(hardening.commandForcedColors, true);
    assert.equal(hardening.shellForcedColors, true);
    assert.equal(hardening.reducedMotion, true);
    assert.equal(hardening.previewReflows, true);
    assert.equal(hardening.liveStatusNamed, true);
    assert.equal(hardening.nonColorStateCue, true);
    assert.equal(hardening.keyboardReachable, true);

    const updateBarrier = await exerciseUpdateBarrier(page);
    assert.equal(updateBarrier.blockedWithActiveWork, true);
    assert.equal(updateBarrier.dirtyBeforeActivation, true);
    assert.equal(updateBarrier.persistedExactDraft, true);
    assert.equal(updateBarrier.revisionAdvanced, true);
    assert.deepEqual(updateBarrier.messages, [{ type: 'cem-studio-activate-update' }]);

    const stored = await importOfflineProject(page);
    assert.equal(stored.repositoryRevision, updateBarrier.repositoryRevisionAfter + 1);
    const onlineFileSystemFallback = await exerciseFileSystemFallback(page, 'offline-project');
    assert.equal(onlineFileSystemFallback.projectId, 'offline-project');
    assert.equal(onlineFileSystemFallback.exactContent, true);
    assert.equal(onlineFileSystemFallback.providerStateExcluded, true);
    assert.equal(onlineFileSystemFallback.importedExact, true);
    assert.equal(onlineFileSystemFallback.contentType, 'application/vnd.cem.studio-project-bundle+json');
    const onlineOfflineProjectProjections = await projectOfflineResource(page);
    assert.equal(onlineOfflineProjectProjections.parse.status, 'projected');
    assert.equal(onlineOfflineProjectProjections.inspect.status, 'projected');

    const online = await executeVersionCommand(page, 'static-worker-online');
    assert.equal(online.exitCode, 0);
    assert.equal(online.runtime, 'wasm-browser-worker');
    assert.equal(online.executorTopology, 'browser-worker-pool');

    const recoveredSelection = { projectId: 'offline-project', resourceId: 'source' };
    await page.evaluate(async selection => {
        const app = globalThis.__cemStudioApplication;
        await app.session.select(selection);
        if (app.session.snapshot().transition) throw new Error('Recovery fixture navigation was not admitted');
        await app.workbenchView.whenSettled();
    }, recoveredSelection);
    await context.setOffline(true);
    await page.goto(`${server.url}/projects/offline-project`, { waitUntil: 'load' });
    await waitForApplication(page, browserErrors, httpFailures);
    assert.ok(await page.evaluate(() => navigator.serviceWorker.controller !== null));
    assert.equal(await page.getAttribute('[data-cem-studio-shell]', 'data-theme'), 'cem-theme-contrast-dark');
    const recovered = await exportOfflineProject(page);
    assert.equal(recovered.content, '{main | Offline project bytes}\n');
    assert.equal(recovered.projectId, 'offline-project');
    assert.deepEqual(await page.evaluate(() => globalThis.__cemStudioApplication.session.snapshot().selection), recoveredSelection);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().persistedText), '{main | Offline project bytes}\n');
    assert.deepEqual(await page.evaluate(() => {
        const status = globalThis.__cemStudioApplication.shell.fileSystem.status();
        return { projectId: status.projectId, resourceId: status.resourceId };
    }), recoveredSelection);
    await page.evaluate(async selection => {
        const app = globalThis.__cemStudioApplication;
        await app.session.select(selection); await app.workbenchView.whenSettled();
    }, updateBarrier.selection);
    const recoveredWorkbench = await recoverWorkbenchOffline(page, updateBarrier.persistedContent);
    assert.equal(recoveredWorkbench.status, 'invalid');
    assert.equal(recoveredWorkbench.projectRevision, updateBarrier.projectRevisionAfter);
    assert.equal(recoveredWorkbench.resourceRevision, updateBarrier.resourceRevisionAfter);
    assert.equal(recoveredWorkbench.content, updateBarrier.persistedContent);
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
    const offlineFileSystemFallback = await exerciseFileSystemFallback(page, 'offline-project');
    assert.equal(offlineFileSystemFallback.sha256, onlineFileSystemFallback.sha256);
    assert.equal(offlineFileSystemFallback.importedExact, true);
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
        hardening,
        updateBarrier,
        projections,
        commandView,
        commandApply,
        portableOperations,
        onlineOfflineProjectProjections,
        onlineFileSystemFallback,
        recoveredWorkbench,
        recoveredCommandView,
        recoveredPortableOperations,
        recoveredProjections,
        offlineNavigation: '/projects/offline-project',
        online,
        offline,
        offlineFileSystemFallback,
    };
    await mkdir(dirname(reportPath), { recursive: true });
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.log('Verified graph-emitted CEM-ML CLI worker and WASM command online and offline.');
    }
} finally {
    await context.setOffline(false).catch(() => undefined);
    await page
        .evaluate(async () => {
            const repository = globalThis.__cemStudioAcceptanceRepository;
            globalThis.__cemStudioApplication?.repository.close();
            await globalThis.__cemStudioApplication?.validator.close();
            repository?.close();
            await repository?.deleteDatabase();
        })
        .catch(() => undefined);
    await browser.close();
    await server.close();
}

async function exerciseProjectLifecycle(page, context) {
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const snapshot = () => page.evaluate(() => {
        const state = globalThis.__cemStudioApplication.workbench.snapshot();
        return { projectId: state.projectId, projectRevision: state.projectRevision, entryId: state.entryId, resourceId: state.resourceId, draft: state.draft, dirty: state.dirty, persistedText: state.persistedText,
            navigationError: state.navigationError, projection: state.projection && { stale: state.projection.stale, exitCode: state.projection.exitCode, summary: state.projection.summary, output: { text: state.projection.output.text } },
            command: state.command && { parsed: { options: state.command.parsed?.options }, application: { status: state.command.application.status, result: state.command.application.result && { entry: state.command.application.result.entry } } } };
    });
    const choose = async (selector, name) => {
        const host = page.locator(selector);
        await host.getByRole('combobox').click();
        await host.getByRole('option', { name, exact: true }).click();
        await settle();
    };
    const click = async name => { console.log(`[cem-studio:lifecycle] ${name}`); await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const source = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const names = ['Acceptance Alpha', 'Acceptance Beta'];
    await context.setOffline(false);
    await page.getByRole('textbox', { name: 'Project name', exact: true }).fill(names[0]);
    await click('Create CEM-ML project');
    const alpha = await page.evaluate(async name => {
        const app = globalThis.__cemStudioApplication;
        const id = JSON.parse(app.workbench.snapshot().home.projects.find(project => project.name === name).id).projectId;
        return (await app.repository.query({ protocolVersion: 1, repository: 'studio-projects', operation: 'export-project', requestRevision: 1, parameters: { projectId: id } })).value;
    }, names[0]);
    const betaId = 'acceptance-beta';
    const archive = await page.evaluate(async ({ alpha, betaId, name }) => {
        const app = globalThis.__cemStudioApplication;
        const bundle = (await app.repository.query({ protocolVersion: 1, repository: 'studio-projects', operation: 'export-project', requestRevision: 1, parameters: { projectId: alpha } })).value;
        bundle.project = { ...bundle.project, id: betaId, name, rootUri: `studio://${betaId}/` };
        const { serializeCemStudioProjectBundle } = await import('@epa-wg/cem-studio/file-system-provider');
        return Array.from(serializeCemStudioProjectBundle(bundle));
    }, { alpha: alpha.project.id, betaId, name: names[1] });
    const chooser = page.waitForEvent('filechooser');
    await page.getByRole('button', { name: 'Import project archive', exact: true }).click();
    await (await chooser).setFiles({ name: 'acceptance.cem-studio.json', mimeType: 'application/vnd.cem.studio-project-bundle+json', buffer: Buffer.from(archive) });
    await page.waitForFunction(name => globalThis.__cemStudioApplication.workbench.snapshot().home.projects.some(project => project.name === name), names[1]);
    await settle();
    const resource = alpha.project.resources[0];
    const openResource = name => choose('[data-cem-studio-workbench-select]', `${name} / ${resource.path} —`);
    await choose('form[host-command="home-open"] cem-select', names[0]); await click('Open project');
    assert.equal((await snapshot()).projectId, alpha.project.id);
    await openResource(names[0]);
    const texts = ['{article | {h1 | Alpha saved} {p | Café α}}\n', '{article | {h1 | Beta saved offline} {p | Café β}}\n'];
    const entries = [];
    const createPage = async (name, text) => {
        await source.fill(text); await settle(); await click('Save and validate');
        assert.equal((await snapshot()).persistedText, text);
        await click('Parse persisted revision');
        if (name === 'Alpha inspection') {
            const revision = await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projectRevision);
            await page.getByRole('combobox', { name: 'Operation option', exact: true }).click();
            await page.getByRole('option', { name: 'format', exact: true }).click(); await settle();
            const fieldForm = page.locator('form[host-command="edit-field"]');
            await fieldForm.waitFor({ state: 'attached' });
            assert.equal(await fieldForm.count(), 1);
            await page.getByRole('combobox', { name: 'format', exact: true }).click();
            await page.getByRole('option', { name: 'events', exact: true }).click();
            await click('Set format');
            assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().command.parsed.options.format), 'events');
            await click('Reset format');
            assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().command.parsed.options.format), 'ast');
            await page.getByRole('combobox', { name: 'Operation option', exact: true }).click();
            await page.getByRole('option', { name: 'Operation options', exact: true }).click(); await settle();
            const groupForm = page.locator('form[host-command="edit-group"]');
            assert.equal(await groupForm.count(), 1);
            assert.equal(await page.locator('form[host-command="edit-field"]').count(), 0);
            const setChoice = async (label, option) => {
                await groupForm.getByRole('combobox', { name: label, exact: true }).click();
                await page.getByRole('option', { name: option, exact: true }).click();
            };
            await setChoice('Change format', 'Set');
            await setChoice('format', 'events');
            await setChoice('Change preserve-source-offsets', 'Set');
            await setChoice('preserve-source-offsets', 'True');
            await click('Set grouped options');
            const grouped = (await snapshot()).command;
            assert.equal(grouped.parsed.options.format, 'events');
            assert.equal(grouped.parsed.options.preserve_source_offsets, true);
            assert.equal((await snapshot()).projectRevision, revision);
            await inspectWorkspaceAccessibility(page, context);
            await page.getByRole('tab', { name: 'Studio command', exact: true }).click(); await settle();
        }
        await choose('[data-cem-studio-command-target-mode]', 'New configuration (recommended when incompatible)');
        await page.getByRole('textbox', { name: 'New configuration name', exact: true }).fill(name);
        await settle(); await click('Apply');
        const state = await snapshot();
        assert.equal(state.command.application.status, 'applied');
        return state.command.application.result.entry.id;
    };
    entries.push(await createPage('Alpha inspection', texts[0]));
    // Open the command-created page through the public search form.
    await page.getByRole('textbox', { name: 'Search projects', exact: true }).fill('Alpha inspection');
    await click('Search'); await click('Open result');
    assert.equal((await snapshot()).entryId, entries[0]);
    assert.equal((await snapshot()).command.parsed.options.format, 'events');
    assert.equal((await snapshot()).command.parsed.options.preserve_source_offsets, true);
    await click('Run configuration');
    assert.equal((await snapshot()).projection.stale, false);
    const editor = await source.elementHandle();
    await source.fill(`${texts[0]}unsaved`); await settle();
    await source.evaluate(element => element.setSelectionRange(2, 7));
    await openResource(names[1]);
    const dialog = page.getByRole('dialog', { name: 'Resolve drafts before navigation', exact: true });
    await dialog.waitFor({ state: 'visible' });
    for (let index = 0; index < 8; index += 1) {
        await page.keyboard.press('Tab');
        const focus = await dialog.evaluate(element => ({ contained: element.contains(document.activeElement), modal: element.matches(':modal'), documentFocused: document.hasFocus(), browserChrome: document.activeElement === document.body }));
        assert.equal(focus.modal && (focus.contained || (!focus.documentFocused && focus.browserChrome)), true, `modal admitted background focus at Tab ${index}: ${JSON.stringify(focus)}`);
    }
    assert.equal(await source.evaluate(element => { element.focus(); return document.activeElement === element; }), false, 'modal background must be inert');
    await page.keyboard.press('Escape'); await settle();
    assert.equal(await dialog.count(), 0);
    assert.equal(await editor.evaluate(element => element === document.querySelector('[data-cem-studio-editor] textarea')), true);
    assert.deepEqual(await editor.evaluate(element => [element.selectionStart, element.selectionEnd]), [2, 7]);
    assert.equal((await snapshot()).draft, `${texts[0]}unsaved`);
    assert.equal(await page.locator('[data-cem-studio-workbench-select] [role="combobox"]').evaluate(element => document.activeElement === element), true);
    await openResource(names[1]); await click('Discard draft');
    assert.equal((await snapshot()).projectId, betaId);
    await context.setOffline(true);
    entries.push(await createPage('Beta inspection', texts[1]));
    // Browser downloads and file-picker import exercise the real archive boundary.
    const downloadEvent = page.waitForEvent('download');
    await click('Export project backup');
    const download = await downloadEvent;
    const backup = await readFile(await download.path());
    const exported = await page.evaluate(async ({ bytes, resourceId }) => {
        const { parseCemStudioProjectBundle } = await import('@epa-wg/cem-studio/file-system-provider');
        const bundle = parseCemStudioProjectBundle(new Uint8Array(bytes));
        return { id: bundle.project.id, entries: bundle.project.entries.map(entry => entry.id), text: new TextDecoder().decode(bundle.contents[resourceId]) };
    }, { bytes: Array.from(backup), resourceId: resource.id });
    assert.equal(exported.id, betaId); assert.ok(exported.entries.includes(entries[1])); assert.equal(exported.text, texts[1]);
    await choose('form[host-command="home-trash"] cem-select', names[1]); await click('Move to Trash');
    assert.equal((await snapshot()).navigationError.code, 'cem.studio.selection.unavailable');
    await choose('form[host-command="home-restore"] cem-select', names[1]); await click('Restore project');
    await click('Reload persisted revision');
    assert.equal((await snapshot()).persistedText, texts[1]);
    await page.reload({ waitUntil: 'load' }); await waitForApplication(page, browserErrors, httpFailures); await settle();
    assert.equal((await snapshot()).projectId, betaId); assert.equal((await snapshot()).persistedText, texts[1]);
    await page.getByRole('textbox', { name: 'Search projects', exact: true }).fill('Beta inspection');
    await click('Search'); await click('Open result'); await click('Run configuration');
    assert.equal((await snapshot()).entryId, entries[1]); assert.equal((await snapshot()).projection.stale, false);
    const accessibility = await inspectWorkspaceAccessibility(page, context);
    await page.getByRole('textbox', { name: 'Search projects', exact: true }).fill('Alpha inspection');
    await click('Search'); await click('Open result');
    assert.equal((await snapshot()).command.parsed.options.format, 'events');
    assert.equal((await snapshot()).command.parsed.options.preserve_source_offsets, true);
    await openResource(names[0]); assert.equal((await snapshot()).persistedText, texts[0]);
    const recoveryText = '{article | Recovery 🙂 remains a draft}\n';
    await source.fill(recoveryText); await settle();
    const recoveryDownload = page.waitForEvent('download');
    await click('Download draft recovery backup');
    const recoveryFile = await recoveryDownload;
    assert.equal(JSON.parse(await readFile(await recoveryFile.path(), 'utf8')).draft, recoveryText);
    await page.reload({ waitUntil: 'load' }); await waitForApplication(page, browserErrors, httpFailures); await settle();
    assert.equal((await snapshot()).draft, recoveryText);
    assert.equal((await snapshot()).persistedText, texts[0]);
    assert.equal(await source.inputValue(), recoveryText);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.session.snapshot().draftRecovery.status), 'recovered');
    // Corrupt only the explicit local recovery boundary; persisted data is untouched.
    await page.evaluate(() => {
        const key = Object.keys(localStorage).find(key => key.includes(':draft-v1:') && localStorage.getItem(key)?.includes('Recovery 🙂'));
        const record = JSON.parse(localStorage.getItem(key));
        record.persistedText = 'outdated baseline';
        localStorage.setItem(key, JSON.stringify(record));
    });
    await page.reload({ waitUntil: 'load' }); await waitForApplication(page, browserErrors, httpFailures); await settle();
    assert.equal((await snapshot()).draft, texts[0]);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.session.snapshot().draftRecovery.status), 'conflict');
    await click('Clear stored recovery and keep current editor drafts');
    assert.equal((await snapshot()).draft, texts[0]);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.session.draftBackup()), '');
    const structure = () => page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().structure);
    const pick = async (label, option) => {
        await page.getByRole('combobox', { name: label, exact: true }).click();
        await page.getByRole('option', { name: option, exact: true }).click();
        await settle();
    };
    await page.getByRole('textbox', { name: 'Project display name', exact: true }).fill('Acceptance Alpha edited');
    await click('Rename project');
    assert.equal((await structure()).name, 'Acceptance Alpha edited');
    await page.getByRole('textbox', { name: 'New subproject name', exact: true }).fill('Acceptance section');
    await click('Create subproject');
    assert.ok((await structure()).entries.some(entry => entry.name === 'Acceptance section'));
    await pick('Configuration to edit', 'Acceptance section');
    await pick('Configuration edit', 'Reorder among siblings');
    await click('Edit configuration');
    assert.equal((await structure()).entries[0].name, 'Acceptance section');
    await click('Undo structure edit');
    assert.equal((await structure()).entries[0].name, 'Alpha inspection');
    await click('Redo structure edit');
    assert.equal((await structure()).entries[0].name, 'Acceptance section');
    await page.getByRole('textbox', { name: 'New resource path', exact: true }).fill('../escaping.cem');
    await click('Apply resource edit');
    assert.ok((await structure()).error);
    await page.getByRole('textbox', { name: 'New resource path', exact: true }).fill('data/structure-source.cem');
    await page.getByRole('textbox', { name: 'Initial CEM-ML source', exact: true }).fill('{p | Created offline}');
    await click('Apply resource edit');
    assert.ok((await structure()).resources.some(resource => resource.path === 'data/structure-source.cem'));
    await click('Undo structure edit');
    assert.ok(!(await structure()).resources.some(resource => resource.path === 'data/structure-source.cem'));
    await click('Redo structure edit');
    assert.ok((await structure()).resources.some(resource => resource.path === 'data/structure-source.cem'));
    assert.equal((await snapshot()).persistedText, texts[0]);
    const activeResource = (await snapshot()).resourceId;
    const oldPath = (await structure()).resources.find(item => item.id === activeResource).path;
    await pick('Resource edit', 'Rename resource');
    await pick('Resource to edit', oldPath);
    await page.getByRole('textbox', { name: 'New resource path', exact: true }).fill('data/renamed-active.cem');
    await click('Apply resource edit');
    assert.equal((await structure()).error, undefined);
    assert.equal((await structure()).resources.find(item => item.id === activeResource).path, 'data/renamed-active.cem');
    assert.equal((await snapshot()).persistedText, texts[0]);
    await click('Delete unreferenced resource');
    assert.match((await structure()).error.message, /referenced/);
    await pick('Resource to edit', 'data/structure-source.cem');
    await click('Delete unreferenced resource');
    assert.equal((await structure()).error, undefined);
    assert.ok(!(await structure()).resources.some(item => item.path === 'data/structure-source.cem'));
    await click('Undo structure edit');
    assert.ok((await structure()).resources.some(item => item.path === 'data/structure-source.cem'));
    await page.getByRole('textbox', { name: 'Search projects', exact: true }).fill('Alpha inspection');
    await click('Search'); await click('Open result'); await click('Run configuration');
    assert.equal((await snapshot()).projection.stale, false);
    assert.equal((await snapshot()).entryId, entries[0]);
    await page.reload({ waitUntil: 'load' }); await waitForApplication(page, browserErrors, httpFailures); await settle();
    assert.equal((await structure()).resources.find(item => item.id === activeResource).path, 'data/renamed-active.cem');
    assert.equal((await structure()).entries[0].name, 'Acceptance section');
    assert.equal((await structure()).canUndo, false);
    const stableResourceReferences = await page.evaluate(async template => {
        const { repository, validator } = globalThis.__cemStudioApplication;
        const { parseCemMlCommandText, parseCemMlCommandResource, serializeCemMlCommandResource, CEM_ML_CLI_COMMAND_CONTENT_TYPE, CEM_ML_CLI_COMMAND_SCHEMA } = await import('@epa-wg/cem-ml-cli/browser-commands');
        const root = 'studio://acceptance-references/';
        const encoder = new TextEncoder(), decoder = new TextDecoder();
        const graph = '// café native graph\n{run | {import @src="../assets/old.svg" @opaque=true @content-type=image/svg+xml | {export @out="../output/result.svg"}}}\n';
        const contents = { asset: encoder.encode('<svg xmlns="http://www.w3.org/2000/svg"><title>Exact café</title></svg>'), graph: encoder.encode(graph),
            config: encoder.encode(`{ "inputs": [{"uri":"${root}assets/old.svg"}] }\n`),
            command: encoder.encode(serializeCemMlCommandResource(parseCemMlCommandText(`cem-ml transform --config ${root}graphs/run.cem`))) };
        const specs = [ ['asset', 'data', 'assets/old.svg', 'image/svg+xml', 'http://www.w3.org/2000/svg'],
            ['graph', 'transform-config', 'graphs/run.cem', 'text/cem-ml', 'https://cem.dev/ns/cli/transform-config/1'],
            ['config', 'run-config', 'run.json', 'application/json', 'https://cem.dev/ns/cli/run-config/1'],
            ['command', 'run-config', 'command.json', CEM_ML_CLI_COMMAND_CONTENT_TYPE, CEM_ML_CLI_COMMAND_SCHEMA] ];
        const resources = await Promise.all(specs.map(async ([id, role, path, contentType, schema]) => ({ id, role, path, contentType, schema, sourceKind: 'project-file', revision: 1,
            sha256: [...new Uint8Array(await crypto.subtle.digest('SHA-256', contents[id]))].map(x => x.toString(16).padStart(2, '0')).join('') })));
        const project = { ...template, id: 'acceptance-references', name: 'Stable reference acceptance', rootUri: root, revision: 1, resources,
            entries: [{ id: 'graph-page', kind: 'transformation-graph', name: 'Opaque copy', runConfigResourceId: 'command', resourceIds: ['asset', 'graph', 'command'] }] };
        const request = (operation, parameters) => ({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation, parameters });
        const edit = (revision, edit) => repository.execute(request('edit-project', { projectId: project.id, expectedRevision: revision, edit }));
        const read = async () => (await repository.query(request('export-project', { projectId: project.id }))).value;
        const ensure = (condition, message) => { if (!condition) throw new Error(message); };
        await repository.execute(request('import-project', { bundle: { project, contents } }));
        const changed = await edit(1, { kind: 'rename-resource', id: 'asset', path: 'assets/new.svg' });
        const rebound = await read();
        ensure(decoder.decode(rebound.contents.graph) === graph, 'graph bytes were not preserved around the native slot');
        ensure(JSON.parse(decoder.decode(rebound.contents.config)).inputs[0].uri === root + 'assets/old.svg', 'run-config reference changed');
        ensure(rebound.project.resources.find(x => x.id === 'graph').revision === 1, 'consumer revision changed during rename');
        let stale = false, referenced = false;
        try { await edit(1, { kind: 'rename-resource', id: 'asset', path: 'stale.svg' }); } catch (error) { stale = error.code === 'cem.studio.repository.revision_conflict'; }
        try { await edit(2, { kind: 'delete-resource', id: 'asset' }); } catch (error) { referenced = error.message.includes('referenced'); }
        ensure(stale && referenced, 'native edit guards failed');
        const outcome = await validator.executeAuthoredResourceCommand({ bytes: new Uint8Array(rebound.contents.graph), contentType: 'text/cem-ml', schema: 'https://cem.dev/ns/cli/transform-config/1', uri: root + 'graphs/run.cem', projectId: project.id, projectRevision: rebound.project.revision, resourceRevision: rebound.project.resources.find(x => x.id === 'graph').revision,
            commandResource: new Uint8Array(rebound.contents.command), dependencies: [{ bytes: new Uint8Array(rebound.contents.asset), contentType: 'image/svg+xml', schema: 'http://www.w3.org/2000/svg', path: root + 'assets/old.svg' }] });
        ensure(outcome.result?.exitCode === 0, 'graph execution after stable references failed');
        ensure(outcome.output.text === decoder.decode(contents.asset), 'graph output lost exact asset bytes');
        await edit(2, { kind: 'restore', bundle: changed.value.before });
        const restored = await read();
        for (const id of ['asset', 'graph', 'config', 'command']) ensure(decoder.decode(restored.contents[id]) === decoder.decode(contents[id]), `undo changed ${id} bytes`);
        ensure(restored.project.resources.find(x => x.id === 'graph').revision === 2, 'undo rewound revisions');
        let collision = false;
        try { await edit(3, { kind: 'rename-resource', id: 'graph', path: 'assets/old.svg' }); } catch { collision = true; }
        ensure(collision && (await read()).project.revision === 3, 'configuration/output collision was committed');
        const relocated = await edit(3, { kind: 'rename-resource', id: 'graph', path: 'moved/deep/run.cem' });
        const moved = await read();
        const movedUri = root + moved.project.resources.find(x => x.id === 'graph').referencePath;
        ensure(decoder.decode(moved.contents.graph) === graph, 'graph relocation changed resolved bindings or unaffected bytes');
        ensure(moved.project.resources.find(x => x.id === 'graph').revision === 3, 'graph relocation incremented revision more than once');
        ensure(JSON.stringify(moved.project.entries) === JSON.stringify(project.entries), 'graph relocation changed stable page links');
        ensure(parseCemMlCommandResource(new Uint8Array(moved.contents.command)).command.options.config === movedUri, 'incoming transform reference changed');
        let staleMove = false, deletion = false;
        try { await edit(3, { kind: 'rename-resource', id: 'graph', path: 'stale.cem' }); } catch (error) { staleMove = error.code === 'cem.studio.repository.revision_conflict'; }
        try { await edit(4, { kind: 'delete-resource', id: 'graph' }); } catch { deletion = true; }
        ensure(staleMove && deletion, 'relocation stale/deletion guards failed');
        const movedOutcome = await validator.executeAuthoredResourceCommand({ bytes: new Uint8Array(moved.contents.graph),
            contentType: 'text/cem-ml', schema: 'https://cem.dev/ns/cli/transform-config/1', uri: movedUri, projectId: project.id,
            projectRevision: moved.project.revision, resourceRevision: moved.project.resources.find(x => x.id === 'graph').revision, commandResource: new Uint8Array(moved.contents.command),
            dependencies: [{ bytes: new Uint8Array(moved.contents.asset), contentType: 'image/svg+xml', schema: 'http://www.w3.org/2000/svg', path: root + 'assets/old.svg' }] });
        ensure(movedOutcome.result?.exitCode === 0 && movedOutcome.output.text === decoder.decode(contents.asset), 'relocated graph execution lost exact bytes');
        const configMove = await edit(4, { kind: 'rename-resource', id: 'config', path: 'moved/run.json' });
        ensure(decoder.decode((await read()).contents.config) === decoder.decode(contents.config), 'absolute run-config relocation changed bytes');
        await edit(5, { kind: 'restore', bundle: configMove.value.before });
        await edit(6, { kind: 'restore', bundle: relocated.value.before });
        const undone = await read();
        for (const id of ['asset', 'graph', 'config', 'command']) ensure(decoder.decode(undone.contents[id]) === decoder.decode(contents[id]), `relocation undo changed ${id} bytes`);
        ensure(undone.project.resources.find(x => x.id === 'graph').path === 'graphs/run.cem', 'relocation undo lost original graph path');

        return true;
    }, alpha.project);
    assert.equal(stableResourceReferences, true);
    await context.setOffline(false);
    await context.setOffline(true);
    assert.equal(await page.evaluate(() => navigator.onLine), false);
    await page.evaluate(async () => {
        const { repository, session } = globalThis.__cemStudioApplication;
        const { parseCemMlCommandText, serializeCemMlCommandResource } = await import('@epa-wg/cem-ml-cli/browser-commands');
        const request = (operation, parameters) => ({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation, parameters });
        const bundle = (await repository.query(request('export-project', { projectId: 'acceptance-references' }))).value;
        bundle.project = { ...bundle.project, id: 'acceptance-graph-authoring', name: 'Graph authoring acceptance', rootUri: 'studio://acceptance-graph-authoring/', revision: 1 };
        bundle.project.resources = bundle.project.resources.filter(resource => resource.id !== 'config');
        const encoder = new TextEncoder();
        bundle.contents.graph = encoder.encode('{run | {import @id=asset @src="../assets/old.svg" @opaque=true @content-type=image/svg+xml | {export @id=result @out="../output/result.svg"}} {import @id=backup @src="../assets/old.svg" @opaque=true @content-type=image/svg+xml | {export @id=backup-result @out="../output/backup.svg"}}}\n');
        bundle.contents.command = encoder.encode(serializeCemMlCommandResource(parseCemMlCommandText('cem-ml transform --config studio://acceptance-graph-authoring/graphs/run.cem')));
        delete bundle.contents.config;
        for (const resource of bundle.project.resources) {
            resource.revision = 1;
            resource.sha256 = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bundle.contents[resource.id]))].map(byte => byte.toString(16).padStart(2, '0')).join('');
        }
        await repository.execute(request('import-project', { bundle }));
        await session.select({ projectId: bundle.project.id, entryId: 'graph-page' });
    });
    await settle();
    const graphBefore = await snapshot();
    const graphDescription = await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().graphConfiguration);
    assert.ok(graphDescription, 'The imported graph must expose native authoring metadata');
    assert.equal(graphDescription.error, undefined);
    assert.equal(graphDescription.parents.some(parent => parent.id === 'backup'), true);
    const graphRecords = await page.evaluate(async () => {
        const { repository } = globalThis.__cemStudioApplication;
        const value = (await repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'export-project', parameters: { projectId: 'acceptance-graph-authoring' } })).value;
        return value.project.resources.filter(resource => resource.id !== 'graph');
    });
    await pick('New graph node parent', 'asset (import)'); await pick('New graph node kind', 'export');
    await page.getByRole('textbox', { name: 'Graph node ID', exact: true }).fill('copy');
    await page.getByRole('textbox', { name: 'out', exact: true }).fill('../output/copy.svg');
    await click('Reset new graph node');
    assert.equal(await page.getByRole('textbox', { name: 'Graph node ID', exact: true }).inputValue(), '');
    assert.equal((await snapshot()).dirty, false);
    await page.getByRole('textbox', { name: 'Graph node ID', exact: true }).fill('copy');
    await page.getByRole('textbox', { name: 'out', exact: true }).fill('../output/copy.svg');
    await click('Create graph node');
    assert.equal((await snapshot()).projectRevision, 1);
    assert.equal((await snapshot()).projection, undefined);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().graphConfiguration.nodes.some(node => node.id === 'copy')), true);
    const graphWithNode = (await snapshot()).draft;
    await click('Undo graph edit'); assert.equal((await snapshot()).draft, graphBefore.draft);
    await click('Redo graph edit'); assert.equal((await snapshot()).draft, graphWithNode);
    await pick('Graph node', 'result (export)'); await pick('Graph attribute', 'out');
    await page.getByRole('textbox', { name: 'Graph attribute value', exact: true }).fill('../output/authored.svg');
    await click('Set graph attribute');
    assert.equal((await snapshot()).projectRevision, 1);
    assert.equal((await snapshot()).dirty, true);
    assert.equal((await snapshot()).projection, undefined);
    const graphAttributeDraft = (await snapshot()).draft;
    await click('Undo graph edit'); assert.equal((await snapshot()).draft, graphWithNode);
    await click('Redo graph edit'); assert.equal((await snapshot()).draft, graphAttributeDraft);
    await pick('Graph edge target', 'copy (export)'); await pick('Graph edge role', 'parent');
    await pick('Graph edge source', 'backup (import)');
    await click('Reset staged edge change');
    assert.equal((await snapshot()).draft, graphAttributeDraft);
    await pick('Graph edge source', 'backup (import)'); await click('Apply graph edge');
    assert.equal((await snapshot()).projectRevision, 1);
    assert.equal((await snapshot()).projection, undefined);
    const graphDraft = (await snapshot()).draft;
    assert.notEqual(graphDraft, graphAttributeDraft);
    await click('Undo graph edit'); assert.equal((await snapshot()).draft, graphAttributeDraft);
    await click('Redo graph edit'); assert.equal((await snapshot()).draft, graphDraft);
    await inspectWorkspaceAccessibility(page, context);
    const graphPaint = await page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const modes = ['cem-theme-light', 'cem-theme-dark', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native'];
        const paint = [];
        for (const mode of modes) {
            application.shell.theme.setMode(mode); await application.shell.whenSettled(); await application.workbenchView.whenSettled();
            for (const label of ['Graph configuration', 'New graph node', 'Graph edge']) {
                const fieldset = document.querySelector(`cem-fieldset[label="${label}"] fieldset`);
                const style = getComputedStyle(fieldset);
                paint.push({ mode, label, background: style.backgroundColor, color: style.color, inset: parseFloat(style.paddingInlineStart), border: parseFloat(style.borderTopWidth) });
            }
        }
        const inheritedInsets = ['Graph configuration', 'New graph node', 'Graph edge'].map(label => {
            const host = document.querySelector(`cem-fieldset[label="${label}"]`);
            host.style.setProperty('--cem-inset-container', '23px');
            const inset = getComputedStyle(host.querySelector('fieldset')).paddingInlineStart;
            host.style.removeProperty('--cem-inset-container'); return inset;
        });
        application.shell.theme.setMode('cem-theme-contrast-dark'); await application.shell.whenSettled();
        return { paint, inheritedInsets };
    });
    assert.deepEqual(graphPaint.inheritedInsets, ['23px', '23px', '23px']);
    for (const paint of graphPaint.paint) assert.ok(paint.background !== 'rgba(0, 0, 0, 0)' && paint.color !== paint.background && paint.inset > 0 && paint.border > 0, JSON.stringify(paint));
    await page.emulateMedia({ forcedColors: 'active' });
    for (const label of ['Graph configuration', 'New graph node', 'Graph edge']) assert.equal(await page.locator(`cem-fieldset[label="${label}"] fieldset`).evaluate(fieldset => {
        const style = getComputedStyle(fieldset);
        return style.color !== style.backgroundColor && style.borderTopColor === style.color;
    }), true);
    await page.emulateMedia({ forcedColors: 'none' });
    await click('Save and validate');
    assert.equal((await snapshot()).projectRevision, 2);
    assert.equal((await snapshot()).persistedText, graphDraft);
    await page.reload({ waitUntil: 'load' }); await waitForApplication(page, browserErrors, httpFailures); await settle();
    assert.equal((await snapshot()).persistedText, graphDraft);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'copy').edges.find(field => field.id === 'parent').source), 'backup');
    await pick('New graph node parent', 'asset (import)'); await pick('New graph node kind', 'export');
    await page.getByRole('textbox', { name: 'Graph node ID', exact: true }).fill('recovered-copy');
    await page.getByRole('textbox', { name: 'out', exact: true }).fill('../output/recovered.svg');
    await click('Create graph node');
    await pick('Graph edge target', 'copy (export)'); await pick('Graph edge role', 'parent');
    await pick('Graph edge source', 'asset (import)'); await click('Apply graph edge');
    const recoveredGraph = (await snapshot()).draft;
    const graphBackupEvent = page.waitForEvent('download'); await click('Download draft recovery backup');
    assert.equal(JSON.parse(await readFile(await (await graphBackupEvent).path(), 'utf8')).draft, recoveredGraph);
    await page.reload({ waitUntil: 'load' }); await waitForApplication(page, browserErrors, httpFailures); await settle();
    assert.equal((await snapshot()).draft, recoveredGraph);
    assert.equal((await snapshot()).persistedText, graphDraft);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().graphConfiguration.nodes.find(node => node.id === 'copy').edges.find(field => field.id === 'parent').source), 'asset');
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.session.snapshot().draftRecovery.status), 'recovered');
    await context.setOffline(false);
    await context.setOffline(true);
    assert.equal(await page.evaluate(() => navigator.onLine), false);
    await click('Reload persisted revision'); await click('Run configuration');
    const graphRun = await snapshot();
    assert.equal(graphRun.projection.exitCode, 0);
    assert.equal(graphRun.projection.stale, false);
    assert.equal(graphRun.projection.summary.artifactCount, 3);
    assert.equal(graphRun.projection.output.text.includes('Exact café'), true);
    assert.equal(await page.evaluate(() => navigator.onLine), false);
    assert.deepEqual(await page.evaluate(async () => {
        const { repository } = globalThis.__cemStudioApplication;
        const value = (await repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'export-project', parameters: { projectId: 'acceptance-graph-authoring' } })).value;
        return value.project.resources.filter(resource => resource.id !== 'graph');
    }), graphRecords);
    await exerciseExampleProjects(page, context);
    await editor.dispose();
    return { exampleProjects: true, projectSemanticComparison: true, projectValidationRepair: true, graphEdgeAuthoring: true, graphNodeCreation: true, graphConfigurationAuthoring: true, stableResourceReferences, resourcePathEditing: true, typedOperationForms: true, groupedOperationForms: true, structuralEditing: true, draftRecovery: true, projects: [alpha.project.id, betaId], entries, onlineCreateAndImport: true, offlineSaveApplyRun: true,
        guardedDraftAndFocusRetained: true, exportedExactBytes: true, trashRestore: true, offlineReopen: true, accessibility };
}

async function exerciseExampleProjects(page, context) {
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const snapshot = () => page.evaluate(() => {
        const state = globalThis.__cemStudioApplication.workbench.snapshot();
        return { draft: state.draft, status: state.status, projectId: state.projectId, projectRevision: state.projectRevision,
            projection: state.projection && { kind: state.projection.kind, exitCode: state.projection.exitCode, text: state.projection.output.text },
            diagnostics: state.validation?.diagnostics, validationStale: state.validation?.stale, validation: state.validation?.reportSummary };
    });
    const click = async name => { console.log(`[cem-studio:examples] ${name}`); await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const pick = async (name, option) => { await page.getByRole('combobox', { name, exact: true }).click(); await page.getByRole('option', { name: option, exact: true }).click(); await settle(); };
    const sampleName = 'Login';
    const copyId = name => page.evaluate(name => JSON.parse(globalThis.__cemStudioApplication.workbench.snapshot().home.projects.find(project => project.name === name).id).projectId, name);
    const selectConfiguration = async (projectId, entryId) => {
        const label = await page.evaluate(({ projectId, entryId }) => {
            const choice = globalThis.__cemStudioApplication.workbench.snapshot().workbenches.find(choice => {
                const selected = JSON.parse(choice.id); return selected.projectId === projectId && selected.entryId === entryId;
            });
            return `${choice.name} — ${choice.operation}`;
        }, { projectId, entryId });
        await pick('Workbench', label);
        const admitted = await page.evaluate(() => {
            const state = globalThis.__cemStudioApplication.workbench.snapshot();
            return { entryId: state.entryId, navigationError: state.navigationError, error: state.error, sessionStatus: state.sessionStatus, transition: state.transition };
        });
        assert.equal(admitted.entryId, entryId, JSON.stringify(admitted));
    };
    await context.setOffline(false); await context.setOffline(true);
    assert.equal(await page.evaluate(() => navigator.onLine), false);
    const before = await snapshot();
    await pick('Example projects', sampleName); await click('Create project from example');
    assert.deepEqual(await snapshot(), before);
    const firstId = await copyId(sampleName);
    await pick('Projects', sampleName); await click('Open project');
    const readme = await page.evaluate(projectId => {
        const choices = globalThis.__cemStudioApplication.workbench.snapshot().workbenches;
        const choice = choices.find(choice => { const selection = JSON.parse(choice.id); return selection.projectId === projectId && selection.resourceId === 'readme' && !selection.entryId; });
        return `${choice.name} — ${choice.operation}`;
    }, firstId);
    await pick('Workbench', readme);
    assert.ok((await snapshot()).draft.includes('demo.login.method_required'));
    assert.ok((await snapshot()).draft.includes('Intentional validation failure and repair'));
    const configurations = await page.evaluate(async projectId => (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', operation: 'get-project', requestRevision: 1, parameters: { projectId } })).value.entries, firstId);
    assert.equal(configurations.length, 7);
    assert.ok(configurations.every(configuration => !/^\d+\./.test(configuration.name)));
    await selectConfiguration(firstId, 'read-edit');
    const original = (await snapshot()).draft;
    await click('Run configuration');
    assert.equal((await snapshot()).projection.kind, 'parse');
    assert.equal((await snapshot()).projection.exitCode, 0);
    const contract = await page.evaluate(async projectId => {
        const bundle = (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', operation: 'export-project', requestRevision: 1, parameters: { projectId } })).value;
        return JSON.parse(new TextDecoder().decode(bundle.contents['login-semantic-contract']));
    }, firstId);
    const semanticValues = async () => JSON.parse((await snapshot()).projection.text).result.sequence.items.map(item => item.value.lexicalValue);
    for (const configuration of ['semantics-html', 'semantics-cem']) {
        await selectConfiguration(firstId, configuration); await click('Run configuration');
        assert.equal((await snapshot()).projection.kind, 'query');
        assert.deepEqual(await semanticValues(), contract);
    }
    await selectConfiguration(firstId, 'read-edit');
    const edited = original.replace('Sign in}', 'Amsterdam sign in}');
    assert.notEqual(edited, original);
    await page.getByRole('textbox', { name: 'CEM source', exact: true }).fill(edited); await settle();
    await click('Save and validate');
    assert.equal((await snapshot()).validation.errorCount, 0);
    assert.equal((await snapshot()).validation.warningCount, 3);
    await click('Run configuration');
    assert.ok((await snapshot()).projection.text.includes('Amsterdam sign in'));
    await selectConfiguration(firstId, 'validate');
    const revision = (await snapshot()).projectRevision;
    await click('Validate persisted revision');
    assert.equal((await snapshot()).validation.errorCount, 0);
    assert.equal((await snapshot()).validation.warningCount, 3);
    assert.equal((await snapshot()).projectRevision, revision);
    await selectConfiguration(firstId, 'inspect'); await click('Run configuration');
    assert.equal((await snapshot()).projection.kind, 'inspect');
    assert.equal((await snapshot()).projection.exitCode, 0);
    assert.ok((await snapshot()).projection.text.includes('Amsterdam sign in'));
    await selectConfiguration(firstId, 'html-conversion'); await click('Run configuration');
    const converted = (await snapshot()).projection;
    assert.equal(converted.kind, 'convert'); assert.equal(converted.exitCode, 0);
    for (const field of ['@name=email', '@name=password', '@method=post']) assert.ok(converted.text.includes(field), field);
    await selectConfiguration(firstId, 'semantics-cem'); await click('Run configuration');
    assert.deepEqual(await semanticValues(), contract.map(value => value === 'title=Sign in' ? 'title=Amsterdam sign in' : value));
    await selectConfiguration(firstId, 'semantics-html');
    const originalHtml = (await snapshot()).draft;
    const changedHtml = originalHtml.replace('method="post"', 'method="get"');
    await page.getByRole('textbox', { name: 'CEM source', exact: true }).fill(changedHtml); await settle();
    await click('Run configuration');
    assert.deepEqual(await semanticValues(), contract);
    assert.equal((await snapshot()).draft, changedHtml);
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projection.stale), true);
    await click('Save and validate'); await click('Run configuration');
    assert.deepEqual(await semanticValues(), contract.map(value => value === 'method=post' ? 'method=get' : value));
    await click('Reload persisted revision'); assert.equal((await snapshot()).draft, changedHtml);
    await page.getByRole('textbox', { name: 'CEM source', exact: true }).fill(originalHtml); await settle();
    await click('Save and validate'); await click('Run configuration');
    assert.deepEqual(await semanticValues(), contract);
    await selectConfiguration(firstId, 'validation-repair');
    assert.ok(await page.getByText('Validate the form method using the project-owned teaching schema.', { exact: false }).isVisible());
    const invalid = (await snapshot()).draft;
    assert.equal(invalid, original.replace(' @method=post', ''));
    await click('Validate persisted revision');
    const failed = await snapshot();
    assert.equal(failed.status, 'invalid');
    assert.equal(failed.validation.errorCount, 1); assert.equal(failed.validation.warningCount, 3);
    const diagnosticIndex = failed.diagnostics.findIndex(diagnostic => diagnostic.code === 'demo.login.method_required');
    assert.ok(diagnosticIndex >= 0);
    assert.deepEqual(failed.diagnostics[diagnosticIndex].range, { start: invalid.indexOf('{form'), len: 5 });
    await page.getByRole('tab', { name: 'Results', exact: true }).click(); await settle();
    await page.getByRole('listbox', { name: 'Diagnostics', exact: true }).selectOption(String(diagnosticIndex)); await settle();
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    assert.equal(await editor.evaluate(element => document.activeElement === element), true);
    assert.equal(await editor.evaluate(element => element.value.slice(element.selectionStart, element.selectionEnd)), '{form');
    const repaired = invalid.replace('@cem:form="sign-in"', '@cem:form="sign-in" @method=post');
    assert.equal(repaired, original);
    await editor.fill(repaired); await settle();
    await click('Validate persisted revision');
    assert.equal((await snapshot()).validation.errorCount, 1);
    assert.equal((await snapshot()).validationStale, true);
    assert.equal((await snapshot()).draft, repaired);
    await click('Save and validate');
    assert.equal((await snapshot()).status, 'valid');
    assert.equal((await snapshot()).validation.errorCount, 0); assert.equal((await snapshot()).validation.warningCount, 3);
    await click('Reload persisted revision');
    assert.equal((await snapshot()).draft, repaired);
    await click('Validate persisted revision'); assert.equal((await snapshot()).validation.errorCount, 0);
    await editor.fill(invalid); await settle(); await click('Save and validate');
    assert.equal((await snapshot()).validation.errorCount, 1);
    await editor.fill(repaired); await settle(); await click('Save and validate');
    assert.equal((await snapshot()).validation.errorCount, 0);
    await selectConfiguration(firstId, 'read-edit');
    await page.reload({ waitUntil: 'load' }); await waitForApplication(page, browserErrors, httpFailures); await settle();
    assert.equal((await snapshot()).draft, edited);
    await pick('Example projects', sampleName); await click('Create project from example');
    const secondId = await copyId(`${sampleName} (2)`);
    assert.notEqual(firstId, secondId);
    assert.equal((await snapshot()).projectId, firstId);
    await pick('Projects', `${sampleName} (2)`); await click('Open project');
    await selectConfiguration(secondId, 'read-edit');
    assert.equal((await snapshot()).draft, original);
    await selectConfiguration(secondId, 'validation-repair');
    assert.equal((await snapshot()).draft, invalid);
    await click('Validate persisted revision');
    assert.equal((await snapshot()).validation.errorCount, 1);
    for (const configuration of ['semantics-html', 'semantics-cem']) {
        await selectConfiguration(secondId, configuration); await click('Run configuration');
        assert.deepEqual(await semanticValues(), contract);
    }
    assert.equal(await page.evaluate(() => navigator.onLine), false);
}

async function inspectWorkspaceAccessibility(page, context) {
    const wasOffline = !await page.evaluate(() => navigator.onLine);
    const cdp = await context.newCDPSession(page);
    const { nodes } = await cdp.send('Accessibility.getFullAXTree');
    const interactiveRoles = ['button', 'combobox', 'textbox', 'tab', 'treeitem'];
    const exposed = nodes.filter(node => !node.ignored && interactiveRoles.includes(node.role?.value));
    assert.ok(exposed.length > 15);
    assert.deepEqual(exposed.filter(node => !node.name?.value?.trim()).map(node => node.role.value), [], 'unnamed accessible control');
    const relationships = await page.evaluate(() => {
        const ids = Array.from(document.querySelectorAll('[id]'), element => element.id);
        const duplicates = ids.filter((id, index) => ids.indexOf(id) !== index);
        const missing = [];
        for (const element of document.querySelectorAll('[aria-labelledby], [aria-describedby], [aria-controls], [aria-activedescendant], [for]')) {
            for (const name of ['aria-labelledby', 'aria-describedby', 'aria-controls', 'aria-activedescendant', 'for']) {
                for (const id of (element.getAttribute(name) ?? '').split(/\s+/).filter(Boolean)) if (!document.getElementById(id)) missing.push({ name, id });
            }
        }
        return { duplicates, missing };
    });
    assert.deepEqual(relationships, { duplicates: [], missing: [] });
    const layouts = [];
    for (const width of [320, 390, 1280]) {
        await page.setViewportSize({ width, height: 900 });
        const layout = await page.evaluate(() => ({ viewport: innerWidth, content: document.documentElement.scrollWidth,
            overflow: Array.from(document.querySelectorAll('body *')).filter(element => element.getBoundingClientRect().right > innerWidth + 1 && element.getBoundingClientRect().width > 0).slice(-12).map(element => ({ tag: element.localName, part: element.getAttribute('part'), text: element.textContent.slice(0, 90), right: element.getBoundingClientRect().right })) }));
        assert.ok(layout.content <= layout.viewport + 1, `horizontal page overflow at ${width}: ${JSON.stringify(layout)}`);
        layouts.push(layout);
    }
    const field = page.getByRole('textbox', { name: 'Project name', exact: true });
    await field.focus(); await page.keyboard.press('Tab');
    assert.equal(await page.getByRole('button', { name: 'Create CEM-ML project', exact: true }).evaluate(element => document.activeElement === element), true);
    const focus = await page.evaluate(() => {
        const style = getComputedStyle(document.activeElement);
        return { width: parseFloat(style.outlineWidth), style: style.outlineStyle, color: style.outlineColor, shadow: style.boxShadow };
    });
    assert.ok(focus.width > 0 && focus.style !== 'none' && (focus.color !== 'rgba(0, 0, 0, 0)' || focus.shadow !== 'none'), 'keyboard focus must be visible');
    await cdp.detach();
    // Preserve offline emulation across the auxiliary CDP session. Toggle both
    // states because Playwright skips an unchanged cached network setting.
    if (wasOffline) { await context.setOffline(false); await context.setOffline(true); }
    return { namedControls: exposed.length, validRelationships: true, layouts, keyboardFocus: focus,
        scope: 'Automated Chromium accessibility tree and keyboard checks; physical devices and spoken assistive technology remain separately open.' };
}

async function waitForApplication(page, browserErrors, httpFailures) {
    try {
        await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication), undefined, { timeout: 30_000 });
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
            indexedDbDatabases: typeof indexedDB.databases === 'function' ? await indexedDB.databases() : [],
            resources: performance.getEntriesByType('resource').map(({ name }) => name),
        }));
        const details = [
            ...browserErrors.map((message) => `browser: ${message}`),
            ...httpFailures.map((message) => `http: ${message}`),
            `diagnostic: ${JSON.stringify(diagnostic)}`,
        ];
        throw new Error(`${error.message}${details.length > 0 ? `\n${details.join('\n')}` : ''}`, { cause: error });
    }
    await page.evaluate(async () => {
        if (globalThis.__cemStudioFixture) return;
        const app = globalThis.__cemStudioApplication;
        const { loadCemStudioFeatureTour, installCemStudioFeatureTour } = await import('@epa-wg/cem-studio');
        const seed = await loadCemStudioFeatureTour({ baseUrl: new URL('./samples/feature-tour/', document.baseURI), validator: app.validator });
        const generic = await app.exampleProjects.examples.find(project => project.id === 'schema-examples').load();
        // Legacy seed-upgrade regression data shares portable command resources with the ordinary example archive.
        for (const scenario of seed.catalog.workbenches) {
            const id = scenario.runConfigResourceId;
            Object.assign(seed.bundle.project.resources.find(resource => resource.id === id), generic.project.resources.find(resource => resource.id === id));
            seed.bundle.contents[id] = generic.contents[id];
        }
        const featureTour = await installCemStudioFeatureTour(app.repository, seed);
        globalThis.__cemStudioFixture = { seed, featureTour };
        if (!app.session.snapshot().selection) {
            const example = seed.catalog.examples.find(item => item.packageId === 'cem-ml' && item.contentType.includes('cem'));
            const entry = seed.bundle.project.entries.find(item => item.runConfigResourceId === example.runConfigResourceId);
            await app.session.select({ projectId: featureTour.projectId, entryId: entry.id });
        }
        await app.workbench.refreshNavigation();
        await app.workbenchView.whenSettled();
    });
}

async function inspectHardening(page, initialResponse) {
    const policy = await initialResponse.headerValue('content-security-policy');
    assert.ok(policy, 'static deployment did not return Content-Security-Policy');
    await page.emulateMedia({ forcedColors: 'active', reducedMotion: 'reduce' });
    await page.setViewportSize({ width: 360, height: 800 });
    await page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        await application.workbench.parsePersisted('ast');
        await application.workbenchView.whenSettled();
        const previewApi = await import('@epa-wg/cem-studio/preview');
        const previewRoot = document.createElement('section');
        previewRoot.setAttribute('data-cem-studio-preview', '');
        previewRoot.setAttribute('data-cem-studio-hardening-preview', '');
        document.body.append(previewRoot);
        const components = await (await import('@epa-wg/cem-studio/shell')).installCemStudioShellComponents();
        const preview = previewApi.createCemStudioPreview({
            bytes: new TextEncoder().encode('Keyboard and reflow preview'), contentType: 'text/plain',
        });
        const viewer = document.createElement('cem-media-preview');
        viewer.setAttribute('mode', 'text'); viewer.setAttribute('label', preview.label);
        viewer.setAttribute('value', preview.text); previewRoot.append(viewer);
        await components.runtime.whenRenderSettled(viewer);
        const output = viewer.querySelector('pre');
        const probe = document.createElement('span');
        probe.tabIndex = 0;
        probe.setAttribute('data-cem-studio-keyboard-probe', '');
        output.before(probe);
        probe.focus();
    });
    await page.keyboard.press('Tab');
    const keyboardReachable = await page.evaluate(
        () => document.activeElement === document.querySelector('[data-cem-studio-hardening-preview] pre'),
    );
    await page.evaluate(() => document.querySelector('[data-cem-studio-keyboard-probe]')?.remove());
    const result = await page.evaluate(
        async ({ contentSecurityPolicy, keyboardReached }) => {
            const preview = document.querySelector('[data-cem-studio-hardening-preview]');
            const previewOutput = preview?.querySelector('pre');
            previewOutput?.focus();
            const previewRect = preview?.getBoundingClientRect();
            const status = document.querySelector('[data-cem-studio-workbench-content] [role="status"]');
            const statusBadge = document.querySelector('[data-cem-studio-workbench-status]');
            const previewApi = await import('@epa-wg/cem-studio/preview');
            const frameRoot = document.createElement('section');
            frameRoot.setAttribute('data-cem-studio-preview', '');
            document.body.append(frameRoot);
            const components = await (await import('@epa-wg/cem-studio/shell')).installCemStudioShellComponents();
            const model = previewApi.createCemStudioPreview({
                bytes: new TextEncoder().encode('<p>Forced color preview</p>'), contentType: 'text/html',
            });
            const viewer = document.createElement('cem-media-preview');
            viewer.setAttribute('mode', 'document'); viewer.setAttribute('label', model.label);
            viewer.setAttribute('value', model.source); frameRoot.append(viewer);
            await components.runtime.whenRenderSettled(viewer);
            const frame = viewer.querySelector('iframe');
            const frameStyles = getComputedStyle(frame);
            const forcedColors =
                matchMedia('(forced-colors: active)').matches &&
                frameStyles.borderTopStyle === 'solid' &&
                frame.getAttribute('sandbox') === '';
            const header = getComputedStyle(document.querySelector('cem-app-bar header'));
            const action = getComputedStyle(document.querySelector('[data-cem-studio-install] button'));
            const editor = getComputedStyle(document.querySelector('[data-cem-studio-editor] textarea'));
            const workbenchAction = getComputedStyle(document.querySelector('[data-cem-studio-reload] button'));
            const workbenchForcedColors = editor.outlineStyle === 'solid' && parseFloat(editor.outlineWidth) > 0 &&
                editor.forcedColorAdjust === 'auto' && workbenchAction.borderTopStyle === 'solid' &&
                workbenchAction.forcedColorAdjust === 'auto';
            const commandEditor = getComputedStyle(document.querySelector('[data-cem-studio-command-editor] textarea'));
            const commandAction = getComputedStyle(document.querySelector('[data-cem-studio-command-copy] button'));
            const commandForcedColors = commandEditor.outlineStyle === 'solid' && parseFloat(commandEditor.outlineWidth) > 0 &&
                commandEditor.forcedColorAdjust === 'auto' && commandAction.borderTopStyle === 'solid' &&
                commandAction.forcedColorAdjust === 'auto';
            const system = document.createElement('span');
            system.style.cssText = 'background: Canvas; color: CanvasText';
            document.body.append(system);
            const systemColors = getComputedStyle(system);
            const treePaint = getComputedStyle(document.querySelector('cem-tree [role="treeitem"]'));
            const panePaint = getComputedStyle(document.querySelector('cem-tabs[label="Workbench panes"] [role="tab"]'));
            const shellForcedColors = treePaint.forcedColorAdjust === 'auto' && treePaint.color !== treePaint.backgroundColor && panePaint.forcedColorAdjust === 'auto' && panePaint.color !== panePaint.backgroundColor && header.backgroundColor === systemColors.backgroundColor &&
                header.color === systemColors.color && header.borderTopStyle === 'solid' &&
                action.borderTopStyle === 'solid' && action.forcedColorAdjust === 'auto';
            system.remove();
            frameRoot.remove();
            return {
                cspDefaultDeny: contentSecurityPolicy.includes("default-src 'none'"),
                cspNoUnsafeScript:
                    contentSecurityPolicy.split(';').some(part => part.trim().startsWith('script-src ') &&
                        !part.includes("'unsafe-inline'") && !part.includes("'unsafe-eval'")),
                cspStyleAttributesBlocked: contentSecurityPolicy.includes("style-src-attr 'none'"),
                forcedColors,
                workbenchForcedColors,
                commandForcedColors,
                shellForcedColors,
                reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
                previewReflows: Boolean(previewRect && previewRect.left >= 0 && previewRect.right <= innerWidth),
                liveStatusNamed:
                    status?.getAttribute('aria-live') === 'polite' && status?.getAttribute('aria-atomic') === 'true',
                nonColorStateCue: Boolean(statusBadge?.getAttribute('label')),
                keyboardReachable: keyboardReached && previewOutput?.tabIndex === 0,
            };
        },
        { contentSecurityPolicy: policy, keyboardReached: keyboardReachable },
    );
    await page.evaluate(() => document.querySelector('[data-cem-studio-hardening-preview]')?.remove());
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.emulateMedia({ forcedColors: 'none', reducedMotion: 'no-preference' });
    return result;
}

async function exerciseUpdateBarrier(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const workbench = application.workbench;
        const update = application.shell.update;
        const messages = [];
        update.setWaitingWorker({ postMessage: (message) => messages.push(message) });
        update.setActiveRequestCount(1);
        const blocked = await update.activateUpdate();
        update.setActiveRequestCount(0);
        const before = await application.repository.query({
            protocolVersion: 1,
            repository: 'studio-projects',
            operation: 'export-project',
            requestRevision: 1,
            parameters: { projectId: globalThis.__cemStudioFixture.featureTour.projectId },
        });
        const draft = `${workbench.snapshot().draft}\n{p | Persisted before update}\n`;
        workbench.updateDraft(draft);
        const dirtyBeforeActivation = update.status().dirty;
        const dirtyBlocked = await update.activateUpdate();
        if (dirtyBlocked.reason !== 'unsaved-work' || messages.length) throw new Error('Update crossed an unresolved draft');
        await workbench.saveSource();
        if (workbench.snapshot().command) {
            const command = workbench.snapshot().command.current.text;
            await workbench.updateCommandDraft(`${command}\n`);
            const commandBlocked = await update.activateUpdate();
            if (commandBlocked.reason !== 'unsaved-work' || messages.length) throw new Error('Update crossed a command draft');
            await workbench.resetCommandDraft();
        }
        await update.activateUpdate();
        const after = await application.repository.query({
            protocolVersion: 1,
            repository: 'studio-projects',
            operation: 'export-project',
            requestRevision: 1,
            parameters: { projectId: globalThis.__cemStudioFixture.featureTour.projectId },
        });
        const bytes = after.value.contents[workbench.snapshot().resourceId];
        const resource = after.value.project.resources.find(({ id }) => id === workbench.snapshot().resourceId);
        return {
            blockedWithActiveWork: blocked.state === 'blocked' && blocked.reason === 'active-work',
            dirtyBeforeActivation,
            persistedExactDraft: new TextDecoder().decode(bytes) === draft,
            revisionAdvanced: after.value.project.revision === before.value.project.revision + 1,
            persistedContent: draft,
            selection: application.session.snapshot().selection,
            projectRevisionAfter: after.value.project.revision,
            resourceRevisionAfter: resource.revision,
            repositoryRevisionAfter: after.repositoryRevision,
            messages,
        };
    });
}

async function validateFeatureTour(page, initialStatus) {
    return page.evaluate(async (startupStatus) => {
        const application = globalThis.__cemStudioApplication;
        const { catalog, bundle } = globalThis.__cemStudioFixture.seed;
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
            status: globalThis.__cemStudioFixture.featureTour.status,
            seedId: catalog.seed.id,
            projectId: globalThis.__cemStudioFixture.featureTour.projectId,
            exampleCount: catalog.exampleCount,
            dependencyCount: catalog.dependencyCount,
            cacheUrlCount: catalog.cacheUrlCount,
            validatedCount,
            cachedSampleResponses: (await sampleCache.keys()).length,
            exampleProjectCached: Boolean(await sampleCache.match(new URL('./samples/examples/login.cem-studio.json', document.baseURI))),
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
            parameters: { projectId: globalThis.__cemStudioFixture.featureTour.projectId },
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
            exactReload:
                persisted === invalid &&
                exported.value.project.revision === snapshot.projectRevision &&
                resource.revision === snapshot.resourceRevision,
            diagnosticCount: snapshot.validation.diagnostics.length,
            hardViolationCount: snapshot.validation.hardViolationCount,
            provenanceCount: snapshot.validation.provenance.length,
            runtime: snapshot.validation.executionIdentity.runtime,
            diagnosticSelection,
            provenanceSelection,
            componentsOnly:
                document.querySelectorAll(
                    '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button):not(cem-tree button)',
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
            const snapshot =
                kind === 'parse'
                    ? await application.workbench.parsePersisted(mode)
                    : await application.workbench.inspectPersisted(mode);
            const projection = snapshot.projection;
            const nativeOperation =
                projection.nativeResult.result?.storage === 'inline' ? projection.nativeResult.result.value : undefined;
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
            componentsOnly:
                document.querySelectorAll(
                    '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button):not(cem-tree button)',
                ).length === 0,
            controlsInstalled: Boolean(
                document.querySelector('[data-cem-studio-workbench] cem-select .cem-select__control') &&
                document.querySelector('[data-cem-studio-workbench] cem-action button') &&
                document.querySelector('[data-cem-studio-workbench] [data-cem-studio-preview] pre') &&
                document.querySelector('[data-cem-studio-workbench] cem-tabs [role="tablist"]') &&
                document.querySelector('[data-cem-studio-workbench] cem-table [role="table"]'),
            ),
        };
    });
}

async function exercisePortableOperations(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const initialWorkbenchId = application.workbench.snapshot().workbenchId;
        const results = [];
        for (const scenario of globalThis.__cemStudioFixture.seed.catalog.workbenches) {
            const project = (await application.repository.query({ protocolVersion: 1, repository: 'studio-projects', operation: 'get-project', requestRevision: 1, parameters: { projectId: globalThis.__cemStudioFixture.featureTour.projectId } })).value;
            const entry = project.entries.find(item => item.runConfigResourceId === scenario.runConfigResourceId);
            await application.session.select({ projectId: project.id, entryId: entry.id });
            if (application.session.snapshot().transition) throw new Error('Portable scenario navigation was not admitted');
            const snapshot = await application.workbench.runPersistedOperation();
            const projection = snapshot.projection;
            await application.workbenchView.whenSettled();
            const recordRows = selector => [...document.querySelectorAll(`${selector} [role="row"]`)]
                .filter(row => row.querySelector('[role="cell"]')).length;
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
                traceRows: recordRows('[data-cem-studio-projection-trace]'),
                graphRows: recordRows('[data-cem-studio-projection-graph]'),
                copiedExact: copied === projection.output.text,
                downloadedExact:
                    downloaded?.contentType === projection.output.contentType &&
                    downloaded?.bytes.byteLength === projection.output.byteLength &&
                    downloaded.bytes.every((byte, index) => byte === projection.output.bytes[index]),
            });
        }
        await application.workbench.selectWorkbench(initialWorkbenchId);
        await application.workbenchView.whenSettled();
        return {
            results,
            componentsOnly:
                document.querySelectorAll(
                    '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button):not(cem-tree button)',
                ).length === 0,
            controlsInstalled: Boolean(
                document.querySelector('[data-cem-studio-workbench-select] .cem-select__control') &&
                document.querySelector('[data-cem-studio-operation-run] button') &&
                document.querySelector('[data-cem-studio-projection-copy] button') &&
                document.querySelector('[data-cem-studio-projection-download] button') &&
                document.querySelector('[data-cem-studio-projection-expected]') &&
                document.querySelector('[data-cem-studio-projection-trace]') &&
                document.querySelector('[data-cem-studio-projection-graph]'),
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
            parameters: { projectId: globalThis.__cemStudioFixture.featureTour.projectId },
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

        const inputUri = Object.values(initial.current.parsed.positionals).find(
            (value) => typeof value === 'string' && value.startsWith('studio:'),
        );
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
            parameters: { projectId: globalThis.__cemStudioFixture.featureTour.projectId },
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
            repositoryUnchanged:
                before.repositoryRevision === after.repositoryRevision &&
                before.value.project.revision === after.value.project.revision,
            componentsOnly:
                document.querySelectorAll(
                    '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button):not(cem-tree button)',
                ).length === 0,
            controlsInstalled: Boolean(
                document.querySelector('cem-textarea[data-cem-studio-command-editor] textarea') &&
                document.querySelector('cem-action[data-cem-studio-command-copy] button') &&
                document.querySelector('cem-action[data-cem-studio-command-reset] button') &&
                changesTableVisible,
            ),
        };
    });
}

async function exerciseCommandApplyAndRun(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        const [
            { createCemStudioProjectRepository },
            { createCemStudioFeatureTourCopy },
            { createCemStudioFeatureTourWorkbench },
        ] = await Promise.all([
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
                    bundle: createCemStudioFeatureTourCopy(globalThis.__cemStudioFixture.seed, {
                        projectId,
                        now: '2026-08-22T00:00:00Z',
                    }),
                    mode: 'create',
                },
            });
            workbench = await createCemStudioFeatureTourWorkbench({
                repository,
                validator: application.validator,
                seed: globalThis.__cemStudioFixture.seed,
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
            const exactReload =
                storedBytes.byteLength === result.commandBytes.byteLength &&
                new Uint8Array(storedBytes).every((byte, index) => byte === new Uint8Array(result.commandBytes)[index]);
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
                componentsOnly:
                    document.querySelectorAll(
                        '[data-cem-studio-workbench] button:not(cem-action button):not(cem-select button):not(cem-tabs button):not(cem-tree button)',
                    ).length === 0,
                controlsInstalled: Boolean(
                    document.querySelector('[data-cem-studio-workbench] cem-text-field input') &&
                    document.querySelector('[data-cem-studio-workbench] cem-select .cem-select__control') &&
                    document.querySelector('[data-cem-studio-workbench] cem-action button'),
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
        await application.workbench.parsePersisted('ast');
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
            seed: globalThis.__cemStudioFixture.seed,
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
        // A page without a persisted command does not synthesize one at load.
        // Exercise the enabled command editor through an explicit native action.
        await application.workbench.parsePersisted('ast');
        await application.workbenchView.whenSettled();
        const components = await (await import('@epa-wg/cem-studio/shell')).installCemStudioShellComponents();
        let themesStyled = true;
        const themeStyles = [];
        let workbenchStyled = true;
        const workbenchStyles = [];
        let explorerStyled = true;
        let commandStyled = true;
        const commandStyles = [];
        const previewProbe = document.createElement('cem-media-preview');
        previewProbe.setAttribute('mode', 'text'); previewProbe.setAttribute('label', 'Theme verification');
        previewProbe.setAttribute('value', 'Exact output'); application.shell.view.element.append(previewProbe);
        await components.runtime.whenRenderSettled(previewProbe);
        let previewStyled = true;
        const previewStyles = [];
        const header = document.querySelector('cem-app-bar header');
        for (const mode of ['cem-theme-light', 'cem-theme-dark', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native']) {
            application.shell.theme.setMode(mode);
            await application.shell.whenSettled();
            const style = getComputedStyle(header);
            themeStyles.push({ mode, background: style.backgroundColor, color: style.color, padding: style.paddingInlineStart });
            themesStyled &&= style.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
                style.color !== style.backgroundColor && parseFloat(style.paddingInlineStart) > 0;
            const treeStyle = getComputedStyle(document.querySelector('cem-tree [role="treeitem"]'));
            const paneStyle = getComputedStyle(document.querySelector('cem-tabs[label="Workbench panes"] [role="tab"]'));
            explorerStyled &&= treeStyle.backgroundColor === 'rgba(0, 0, 0, 0)' && treeStyle.color !== treeStyle.backgroundColor && parseFloat(treeStyle.minBlockSize) > 0 && paneStyle.color !== paneStyle.backgroundColor;
            const editor = getComputedStyle(document.querySelector('[data-cem-studio-editor] textarea'));
            const action = getComputedStyle(document.querySelector('[data-cem-studio-reload] button'));
            workbenchStyles.push({ mode, background: editor.backgroundColor, color: editor.color,
                indicator: editor.boxShadow, actionBorder: action.borderTopWidth });
            workbenchStyled &&= editor.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
                editor.color !== editor.backgroundColor && editor.boxShadow !== 'none' && parseFloat(action.borderTopWidth) > 0;
            const commandEditor = getComputedStyle(document.querySelector('[data-cem-studio-command-editor] textarea'));
            const commandAction = getComputedStyle(document.querySelector('[data-cem-studio-command-copy] button'));
            commandStyles.push({ mode, background: commandEditor.backgroundColor, color: commandEditor.color,
                indicator: commandEditor.boxShadow, actionBorder: commandAction.borderTopWidth });
            commandStyled &&= commandEditor.backgroundColor !== 'rgba(0, 0, 0, 0)' &&
                commandEditor.color !== commandEditor.backgroundColor && commandEditor.boxShadow !== 'none' &&
                parseFloat(commandAction.borderTopWidth) > 0;
            const figure = getComputedStyle(previewProbe.querySelector('figure'));
            const text = getComputedStyle(previewProbe.querySelector('pre'));
            previewStyles.push({ mode, background: figure.backgroundColor, color: figure.color,
                font: text.fontFamily, size: text.fontSize, border: figure.borderTopWidth });
            previewStyled &&= figure.backgroundColor !== 'rgba(0, 0, 0, 0)' && figure.color !== figure.backgroundColor &&
                parseFloat(figure.borderTopWidth) > 0 && parseFloat(text.fontSize) > 0;


        }
        const shellHost = application.shell.view.element;
        shellHost.style.setProperty('--cem-content-interaction-default-background', 'rgb(12, 34, 56)');
        shellHost.style.setProperty('--cem-inset-container', '19px');
        shellHost.style.setProperty('--cem-typography-script-font-size', '21px');
        shellHost.style.setProperty('--cem-bend-media', '11px');
        shellHost.style.setProperty('--cem-action-border-radius', '13px');
        shellHost.style.setProperty('--cem-input-background-color', 'rgb(31, 47, 63)');
        const actionStyle = getComputedStyle(document.querySelector('[data-cem-studio-install] button'));
        const overrideStyles = { treeBackground: getComputedStyle(document.querySelector('cem-tree [role="treeitem"]')).backgroundColor, padding: getComputedStyle(header).paddingInlineStart,
            radius: actionStyle.borderTopLeftRadius, couplingBorder: actionStyle.borderTopWidth,
            editorBackground: getComputedStyle(document.querySelector('[data-cem-studio-editor] textarea')).backgroundColor,
            workbenchRadius: getComputedStyle(document.querySelector('[data-cem-studio-reload] button')).borderTopLeftRadius,
            workbenchBorder: getComputedStyle(document.querySelector('[data-cem-studio-reload] button')).borderTopWidth,
            commandBackground: getComputedStyle(document.querySelector('[data-cem-studio-command-editor] textarea')).backgroundColor,
            commandRadius: getComputedStyle(document.querySelector('[data-cem-studio-command-copy] button')).borderTopLeftRadius,
            commandBorder: getComputedStyle(document.querySelector('[data-cem-studio-command-copy] button')).borderTopWidth,
            previewInset: getComputedStyle(previewProbe.querySelector('pre')).paddingInlineStart,
            previewFontSize: getComputedStyle(previewProbe.querySelector('pre')).fontSize,
            previewBend: getComputedStyle(previewProbe.querySelector('figure')).borderTopLeftRadius };
        const previewOverrides = overrideStyles.previewInset === '19px' && overrideStyles.previewFontSize === '21px' &&
            overrideStyles.previewBend === '11px';
        const commandOverrides = overrideStyles.commandBackground === 'rgb(31, 47, 63)' &&
            parseFloat(overrideStyles.commandRadius) === 13 + parseFloat(overrideStyles.commandBorder);
        const workbenchOverrides = overrideStyles.editorBackground === 'rgb(31, 47, 63)' &&
            parseFloat(overrideStyles.workbenchRadius) === 13 + parseFloat(overrideStyles.workbenchBorder);
        const inheritedOverrides = overrideStyles.treeBackground === 'rgb(12, 34, 56)' && overrideStyles.padding === '19px' &&
            parseFloat(overrideStyles.radius) === 13 + parseFloat(overrideStyles.couplingBorder);
        shellHost.style.removeProperty('--cem-content-interaction-default-background');
        shellHost.style.removeProperty('--cem-inset-container');
        shellHost.style.removeProperty('--cem-typography-script-font-size');
        shellHost.style.removeProperty('--cem-bend-media');
        previewProbe.remove();
        shellHost.style.removeProperty('--cem-action-border-radius');
        shellHost.style.removeProperty('--cem-input-background-color');
        application.shell.theme.setMode('cem-theme-contrast-dark');
        await application.shell.whenSettled();
        await Promise.all(
            [
                'cem-app-bar',
                'cem-action',
                'cem-select',
                'cem-badge',
                'cem-alert',
                'cem-textarea',
                'cem-tabs',
                'cem-list',
                'cem-table',
            ].map((tag) => customElements.whenDefined(tag)),
        );
        return {
            themesStyled, explorerStyled,
            previewStyled,
            previewStyles,
            previewOverrides,
            commandStyled,
            commandStyles,
            commandOverrides,
            workbenchStyled,
            workbenchStyles,
            workbenchOverrides,
            themeStyles,
            inheritedOverrides,
            overrideStyles,
            errors: application.shell.view.errors.map(error => error.message),
            styleDiagnostics: [...components.result.diagnostics, ...components.runtime.diagnosticsFor(document.querySelector('cem-app-bar'))],
            styles: [...document.querySelectorAll('style')].map(style => style.textContent.slice(0, 180)),
            state: document.querySelector('[data-cem-studio-shell]')?.getAttribute('data-cem-studio-state'),
            theme: document.querySelector('[data-cem-studio-shell]')?.getAttribute('data-theme'),
            controlsOnly:
                document.querySelectorAll('button:not(cem-action button):not(cem-icon-button button):not(cem-select button):not(cem-tabs button):not(cem-tree button)')
                    .length === 0,
            componentsInstalled: Boolean(
                document.querySelector('cem-app-bar header') &&
                document.querySelector('cem-action button') &&
                document.querySelector('cem-select .cem-select__control') &&
                document.querySelector('cem-badge .cem-badge') &&
                document.querySelector('cem-alert .cem-alert') &&
                document.querySelector('cem-textarea .cem-textarea__control') &&
                document.querySelector('cem-tabs [role="tablist"]'),
            ),
            fileSystemState: application.shell.fileSystem.status().state,
            fileSystemControlsInstalled: Boolean(
                document.querySelector('cem-action[data-cem-studio-open-project] button') &&
                document.querySelector('cem-action[data-cem-studio-provider-reconnect] button') &&
                document.querySelector('cem-action[data-cem-studio-provider-write] button') &&
                document.querySelector('cem-action[data-cem-studio-import-fallback] button') &&
                document.querySelector('cem-action[data-cem-studio-export-fallback] button') &&
                document.querySelector('[data-cem-studio-provider-alert]'),
            ),
        };
    });
}

async function exerciseFileSystemFallback(page, projectId) {
    return page.evaluate(async (id) => {
        const application = globalThis.__cemStudioApplication;
        const [
            { CEM_STUDIO_PROJECT_BUNDLE_CONTENT_TYPE, createCemStudioFileSystemProvider, parseCemStudioProjectBundle },
            { createCemStudioProjectRepository },
        ] = await Promise.all([
            import('@epa-wg/cem-studio/file-system-provider'),
            import('@epa-wg/cem-studio/repository'),
        ]);
        const fallback = await application.fileSystemProvider.exportFallback({ projectId: id });
        const decoded = parseCemStudioProjectBundle(fallback.archive.bytes);
        const digest = await crypto.subtle.digest('SHA-256', fallback.archive.bytes);
        const sha256 = [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, '0')).join('');
        const importedRepository = createCemStudioProjectRepository({
            databaseName: `cem-studio-fallback-${crypto.randomUUID()}`,
            validateProject: application.validator.validateProject,
        });
        try {
            const provider = createCemStudioFileSystemProvider({
                repository: importedRepository,
                showOpenFilePicker: null,
                showDirectoryPicker: null,
            });
            await provider.importFallback({ archive: fallback.archive.bytes });
            const imported = await importedRepository.query({
                protocolVersion: 1,
                repository: 'studio-projects',
                operation: 'export-project',
                requestRevision: 1,
                parameters: { projectId: id },
            });
            const source = decoded.project.resources.find(({ id: resourceId }) => resourceId === 'source');
            const decodedBytes = decoded.contents.source;
            const importedBytes = imported.value.contents.source;
            return {
                projectId: decoded.project.id,
                contentType: fallback.archive.contentType,
                sha256,
                exactContent: source.sha256.length === 64 && decodedBytes.byteLength > 0,
                providerStateExcluded: !JSON.stringify(decoded.project).includes('provider'),
                importedExact:
                    imported.value.project.id === decoded.project.id &&
                    importedBytes.byteLength === decodedBytes.byteLength &&
                    new Uint8Array(importedBytes).every((byte, index) => byte === decodedBytes[index]),
                expectedContentType: CEM_STUDIO_PROJECT_BUNDLE_CONTENT_TYPE,
            };
        } finally {
            importedRepository.close();
            await importedRepository.deleteDatabase();
        }
    }, projectId);
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
                        resources: [
                            {
                                id: 'source',
                                role: 'data',
                                sourceKind: 'project-file',
                                path: 'source.cem',
                                contentType: 'application/cem',
                                schema: 'https://cem.dev/ns/cem-ml/1',
                                revision: 1,
                                sha256,
                            },
                        ],
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
        const api = await import('@epa-wg/cem-ml-cli/browser-commands');
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

async function startStaticServer(root) {
    const security = JSON.parse(await readFile(resolve(root, 'security-headers.json'), 'utf8'));
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
                    ...security.headers,
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
