import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtempSync, readFileSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, extname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';
import { build } from 'vite';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const packageMetadata = JSON.parse(readFileSync(resolve(projectRoot, 'package.json'), 'utf8'));
const temporaryRoot = mkdtempSync(resolve(tmpdir(), 'cem-ml-browser-worker-'));
const bundleRoot = resolve(temporaryRoot, 'bundle');
let browser;
let server;

try {
    await build({
        configFile: false,
        logLevel: 'warn',
        root: projectRoot,
        resolve: {
            conditions: ['browser', 'import', 'module', 'default'],
        },
        build: {
            emptyOutDir: true,
            outDir: bundleRoot,
            target: 'es2022',
            rollupOptions: {
                input: resolve(projectRoot, 'tests/browser-worker-pool.fixture.mjs'),
                output: {
                    entryFileNames: 'entry.js',
                    chunkFileNames: 'assets/[name]-[hash].js',
                    assetFileNames: 'assets/[name]-[hash][extname]',
                },
            },
        },
    });

    server = createServer((request, response) => {
        const pathname = new URL(request.url ?? '/', 'http://127.0.0.1').pathname;
        if (pathname === '/') {
            response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
            response.end('<!doctype html><script type="module" src="/entry.js"></script>');
            return;
        }
        const path = resolve(bundleRoot, `.${decodeURIComponent(pathname)}`);
        const relativePath = relative(bundleRoot, path);
        if (
            relativePath === '..' ||
            relativePath.startsWith(`..${sep}`) ||
            isAbsolute(relativePath) ||
            !statSync(path, { throwIfNoEntry: false })?.isFile()
        ) {
            response.writeHead(404).end();
            return;
        }
        response.writeHead(200, { 'content-type': contentType(path) });
        response.end(readFileSync(path));
    });
    await new Promise((resolveListening, rejectListening) => {
        server.once('error', rejectListening);
        server.listen(0, '127.0.0.1', resolveListening);
    });
    const address = server.address();
    assert.ok(address && typeof address === 'object');

    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    page.on('pageerror', (error) => console.error(`browser page error: ${error.message}`));
    await page.goto(`http://127.0.0.1:${address.port}/`);
    await page.waitForFunction(() => typeof globalThis.runCemMlBrowserFixture === 'function');

    const pool = await runFixture(page, 'pool');
    assert.equal(pool.mode, 'pool');
    assert.equal(pool.fallbackReason, undefined);
    assert.equal(pool.size, 2);
    assert.equal(pool.topology, 'browser-worker-pool');
    assert.equal(pool.effectiveMaxWorkers, 2);
    assert.equal(pool.commonVersion, packageMetadata.version);
    assert.equal(pool.workers.length, 2);
    assert.equal(new Set(pool.workers.map(({ runtimeInstanceId }) => runtimeInstanceId)).size, 2);
    assert.deepEqual(
        pool.workers.map(({ slot, generation }) => ({ slot, generation })),
        [
            { slot: 1, generation: 1 },
            { slot: 2, generation: 1 },
        ],
    );
    assert.equal(pool.mainThread, undefined);
    assert.equal(pool.sharedArrayBufferAvailable, false);
    assert.equal(pool.hardCancelAvailability, 'available');

    const single = await runFixture(page, 'single-worker');
    assert.equal(single.mode, 'single-worker');
    assert.equal(single.size, 1);
    assert.equal(single.effectiveMaxWorkers, 1);
    assert.equal(single.workers.length, 1);

    const singleFallback = await runFixture(page, 'single-worker-fallback');
    assert.equal(singleFallback.mode, 'single-worker-fallback');
    assert.equal(singleFallback.fallbackReason, 'pool-initialization-failed');
    assert.equal(singleFallback.constructionAttempts, 2);
    assert.equal(singleFallback.workers.length, 1);
    assert.equal(singleFallback.topology, 'browser-worker-pool');

    const mainFallback = await runFixture(page, 'main-thread-fallback');
    assert.equal(mainFallback.mode, 'main-thread-fallback');
    assert.equal(mainFallback.fallbackReason, 'worker-initialization-failed');
    assert.equal(mainFallback.constructionAttempts, 2);
    assert.equal(mainFallback.workers.length, 0);
    assert.equal(mainFallback.size, 1);
    assert.equal(mainFallback.topology, 'sequential');
    assert.equal(mainFallback.effectiveMaxWorkers, 1);
    assert.equal(mainFallback.mainThread.runtimeInstanceId, 'browser-main-thread');
    assert.equal(mainFallback.hardCancelAvailability, 'unavailable');

    const unavailable = await runFixture(page, 'workers-unavailable');
    assert.equal(unavailable.mode, 'main-thread-fallback');
    assert.equal(unavailable.fallbackReason, 'workers-unavailable');
    assert.equal(unavailable.constructionAttempts, 0);
    assert.equal(unavailable.topology, 'sequential');

    const bounds = await runFixture(page, 'bounds');
    assert.match(bounds[0], /workerCount=0/);
    assert.match(bounds[1], /workerCount=5/);
    assert.match(bounds[2], /maxWorkers=257/);
    assert.match(bounds[3], /startupTimeoutMs=0/);

    const operationPool = await runFixture(page, 'operation-pool');
    assert.equal(operationPool.mode, 'pool');
    assert.equal(operationPool.transformedItemCount, 2);
    assert.equal(operationPool.queriedItemCount, 2);
    assert.deepEqual(operationPool.commits, ['1', '2', '3', '4']);
    assert.equal(operationPool.queryTerminal.status, 'succeeded');
    assert.equal(operationPool.cancelled.status, 'cancelled');
    assert.equal(operationPool.awaitErrorName, 'AbortError');
    assert.ok(operationPool.replacementEvents.length > 0);
    assert.ok(operationPool.workers.some(({ generation }) => generation > 1));

    const operationSingle = await runFixture(page, 'operation-single-worker');
    assert.equal(operationSingle.mode, 'single-worker');
    assert.equal(operationSingle.transformedItemCount, 2);
    assert.equal(operationSingle.queriedItemCount, 2);
    assert.equal(operationSingle.queryTerminal.status, 'succeeded');
    assert.equal(operationSingle.cancelled.status, 'cancelled');

    const operationMain = await runFixture(page, 'operation-main-thread');
    assert.equal(operationMain.mode, 'main-thread-fallback');
    assert.equal(operationMain.transformedItemCount, 2);
    assert.equal(operationMain.queriedItemCount, 2);
    assert.equal(operationMain.queryTerminal.status, 'succeeded');
    assert.equal(operationMain.cancelled.status, 'cancelled');
    assert.equal(operationMain.replacementEvents.length, 0);

    const command = await runFixture(page, 'command-service');
    assert.equal(command.worker.slot, 1);
    assert.equal(command.worker.generation, 1);
    assert.equal(command.worker.commonVersion, packageMetadata.version);
    assert.match(command.worker.runtimeInstanceId, /^browser-command-\d+:slot-1:generation-1$/);
    assert.equal(command.runtime, 'wasm-browser-worker');
    assert.equal(command.topology, 'browser-worker-pool');
    assert.equal(command.effectiveMaxWorkers, 1);
    assert.equal(command.abiIdentity, command.resultIdentity.abiIdentity);
    assert.equal(command.resultIdentity.runtime, 'wasm-browser-worker');
    assert.equal(command.status, 'succeeded');
    assert.equal(command.originalArtifactCount, 2);
    assert.ok(command.artifactByteLength > 16);
    assert.equal(command.firstReadByteLength, 16);
    assert.equal(command.copiedArtifactBytes, true);
    assert.deepEqual(command.disposeDispositions, ['disposed', 'already-disposed', 'disposed']);
    assert.deepEqual(command.progress, [
        [1, 'accepted', undefined],
        [2, 'prepared', undefined],
        [3, 'executing', undefined],
        [4, 'terminal', 'succeeded'],
    ]);
    assert.deepEqual(command.subscribedProgress, [1, 2, 3, 4]);
    assert.deepEqual(
        command.events.slice(0, 7).map(([kind]) => kind),
        ['ledger', 'read', 'prepare', 'prepare', 'ledger', 'commit', 'commit'],
    );
    assert.deepEqual(
        new Set(command.writes.map(({ uri }) => uri)),
        new Set(command.outputUris),
    );
    assert.ok(command.writes.every(({ byteLength, committed }) => byteLength > 0 && committed));
    assert.deepEqual(command.concurrent, [
        { requestId: 'browser-command-version-a', status: 'succeeded' },
        { requestId: 'browser-command-version-b', status: 'succeeded' },
    ]);
    assert.deepEqual(command.concurrentProgress, {
        'browser-command-version-a': [1, 2, 3, 4],
        'browser-command-version-b': [1, 2, 3, 4],
    });
    assert.equal(command.callbackFailure.resolved, false);
    assert.equal(command.callbackFailure.code, 'cem.command_service.ledger_read');
    assert.match(command.callbackFailure.message, /fixture revision ledger unavailable/);

    const allOperations = await runFixture(page, 'command-all-operations');
    assert.deepEqual(
        allOperations.summaries.map(({ operation }) => operation),
        [
            'parse',
            'validate',
            'check',
            'inspect',
            'convert',
            'query',
            'transform',
            'transform',
            'trace',
            'version-capabilities',
        ],
    );
    assert.deepEqual(
        allOperations.summaries
            .filter(({ operation }) => operation === 'transform')
            .map(({ sourceKind }) => sourceKind),
        ['direct', 'graph'],
    );
    assert.ok(
        allOperations.summaries.every(
            ({ status, exitCode, runtime, diagnostics, hasResult }) =>
                status === 'succeeded' &&
                exitCode === 0 &&
                runtime === 'wasm-browser-worker' &&
                diagnostics === 0 &&
                hasResult,
        ),
    );
    assert.ok(allOperations.summaries.some(({ hasReport }) => hasReport));
    assert.ok(allOperations.summaries.some(({ sourceMaps }) => sourceMaps > 0));
    assert.ok(allOperations.summaries.some(({ presentationTargets }) => presentationTargets.length > 0));
    assert.ok(allOperations.committedWrites > 0);

    const cancellation = await runFixture(page, 'command-cancellation');
    assert.equal(cancellation.acknowledgement.requestId, 'browser-command-cancel');
    assert.equal(cancellation.acknowledgement.disposition, 'accepted');
    assert.equal(cancellation.status, 'cancelled');
    assert.equal(cancellation.exitCode, 130);
    assert.deepEqual(cancellation.progress, [
        [1, 'accepted', undefined],
        [2, 'terminal', 'cancelled'],
    ]);

    const close = await runFixture(page, 'command-close');
    assert.deepEqual(close.failure, {
        resolved: false,
        code: 'cem.browser_command.client_closed',
        message: 'CEM-ML browser command-service client was closed',
    });
    assert.equal(close.postCloseCode, 'cem.browser_command.client_closed');

    const workerFailure = await runFixture(page, 'command-worker-failure');
    assert.equal(workerFailure.corrupted, true);
    assert.deepEqual(workerFailure.terminal, {
        resolved: false,
        code: 'cem.browser_command.worker_failed',
    });
    assert.equal(workerFailure.failures.length, 1);
    assert.equal(workerFailure.failures[0].code, 'worker-error');
    assert.equal(workerFailure.failures[0].worker.slot, 1);

    const workerUnavailable = await runFixture(page, 'command-worker-unavailable');
    assert.deepEqual(workerUnavailable, {
        accepted: false,
        code: 'cem.browser_command.worker_unavailable',
        callbacks: 0,
    });

    console.log(
        `Verified ${packageMetadata.name}@${packageMetadata.version} in Chromium: worker-pool control plus dedicated-worker command callbacks, progress, cancellation, artifacts, cleanup, and capability identity.`,
    );
} finally {
    await browser?.close();
    await new Promise((resolveClose) => server?.close(resolveClose) ?? resolveClose());
    assert.ok(temporaryRoot.startsWith(`${tmpdir()}${sep}cem-ml-browser-worker-`));
    rmSync(temporaryRoot, { recursive: true, force: true });
}

async function runFixture(page, scenario) {
    return page.evaluate(async (selectedScenario) => globalThis.runCemMlBrowserFixture(selectedScenario), scenario);
}

function contentType(path) {
    switch (extname(path)) {
        case '.js':
            return 'text/javascript; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        default:
            return 'application/octet-stream';
    }
}
