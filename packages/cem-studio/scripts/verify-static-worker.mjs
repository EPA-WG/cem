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

    const online = await executeVersionCommand(page, 'static-worker-online');
    assert.equal(online.exitCode, 0);
    assert.equal(online.runtime, 'wasm-browser-worker');
    assert.equal(online.executorTopology, 'browser-worker-pool');

    await context.setOffline(true);
    await page.reload({ waitUntil: 'load' });
    assert.ok(await page.evaluate(() => navigator.serviceWorker.controller !== null));
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
        online,
        offline,
    };
    await mkdir(dirname(reportPath), { recursive: true });
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.log('Verified graph-emitted CEM-ML CLI worker and WASM command online and offline.');
} finally {
    await context.setOffline(false).catch(() => undefined);
    await browser.close();
    await server.close();
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
