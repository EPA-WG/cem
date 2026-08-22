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

page.on('pageerror', (error) => browserErrors.push(error.message));
page.on('console', (message) => {
    if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) {
        browserErrors.push(message.text());
    }
});
page.on('response', (response) => {
    if (response.status() >= 400) httpFailures.push(`${response.status()} ${response.url()}`);
});

try {
    await page.goto(`${server.url}/index.html`, { waitUntil: 'load' });
    await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication));
    await page.evaluate(async () => {
        await navigator.serviceWorker.register('./service-worker.js', {
            scope: './',
            type: 'module',
            updateViaCache: 'none',
        });
        await navigator.serviceWorker.ready;
        if (!navigator.serviceWorker.controller) {
            await new Promise((resolveController) => {
                navigator.serviceWorker.addEventListener('controllerchange', resolveController, {
                    once: true,
                });
            });
        }
    });

    const shell = await inspectShell(page);
    assert.equal(shell.state, 'ready');
    assert.equal(shell.theme, 'cem-theme-contrast-dark');
    assert.equal(shell.controlsOnly, true);
    assert.equal(shell.componentsInstalled, true);

    const caches = await page.evaluate(() => globalThis.caches.keys());
    assert.deepEqual(caches.sort(), [
        'cem-studio:0.1.0:runtime',
        'cem-studio:0.1.0:samples',
        'cem-studio:0.1.0:shell',
    ]);

    const stored = await importOfflineProject(page);
    assert.equal(stored.repositoryRevision, 1);

    const online = await executeVersionCommand(page, 'static-worker-online');
    assert.equal(online.exitCode, 0);
    assert.equal(online.runtime, 'wasm-browser-worker');
    assert.equal(online.executorTopology, 'browser-worker-pool');

    await context.setOffline(true);
    await page.goto(`${server.url}/projects/offline-project`, { waitUntil: 'load' });
    await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication));
    assert.ok(await page.evaluate(() => navigator.serviceWorker.controller !== null));
    assert.equal(await page.getAttribute('[data-cem-studio-root]', 'data-theme'), 'cem-theme-contrast-dark');
    const recovered = await exportOfflineProject(page);
    assert.equal(recovered.content, 'Offline project bytes');
    assert.equal(recovered.projectId, 'offline-project');
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
        repository?.close();
        await repository?.deleteDatabase();
    }).catch(() => undefined);
    await browser.close();
    await server.close();
}

async function inspectShell(page) {
    return page.evaluate(async () => {
        const application = globalThis.__cemStudioApplication;
        application.shell.theme.setMode('cem-theme-contrast-dark');
        await Promise.all(['cem-app-bar', 'cem-action', 'cem-select', 'cem-badge', 'cem-alert'].map(
            (tag) => customElements.whenDefined(tag),
        ));
        return {
            state: document.querySelector('[data-cem-studio-root]')?.getAttribute('data-cem-studio-state'),
            theme: document.querySelector('[data-cem-studio-root]')?.getAttribute('data-theme'),
            controlsOnly: document.querySelectorAll('button:not(cem-action button):not(cem-select button)').length === 0,
            componentsInstalled: Boolean(
                document.querySelector('cem-app-bar header')
                && document.querySelector('cem-action button')
                && document.querySelector('cem-select .cem-select__control')
                && document.querySelector('cem-badge .cem-badge')
                && document.querySelector('cem-alert .cem-alert'),
            ),
        };
    });
}

async function importOfflineProject(page) {
    return page.evaluate(async () => {
        const { createCemStudioProjectRepository } = await import('@epa-wg/cem-studio/repository');
        const repository = createCemStudioProjectRepository({ validateProject: async (bundle) => bundle });
        globalThis.__cemStudioAcceptanceRepository = repository;
        const content = new TextEncoder().encode('Offline project bytes');
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
