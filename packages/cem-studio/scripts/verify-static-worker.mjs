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

    const featureTour = await validateFeatureTour(page, initialFeatureTourStatus);
    assert.equal(featureTour.initialStatus, 'installed');
    assert.ok(['installed', 'preserved'].includes(featureTour.status));
    assert.equal(featureTour.seedId, 'cem-ml-feature-tour-seed');
    assert.equal(featureTour.projectId, 'feature-tour');
    assert.equal(featureTour.exampleCount, 30);
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
        onlineOfflineProjectProjections,
        recoveredWorkbench,
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
