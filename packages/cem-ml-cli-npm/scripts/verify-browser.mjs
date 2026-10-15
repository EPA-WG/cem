import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, extname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';
import { build } from 'vite';
import { verifyHtmlBaseParity, emitNativeHtmlBaseEvidence } from './verify-html-base.mjs';
import { verifyHtmlBaseFreezeEvidence } from './verify-html-base-evidence.mjs';
import { verifyStylesheetSvg } from './verify-stylesheet-svg.mjs';
import { verifyRecordedOutput } from './verify-recorded-output.mjs';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const packageMetadata = JSON.parse(readFileSync(resolve(projectRoot, 'package.json'), 'utf8'));
const temporaryRoot = mkdtempSync(resolve(tmpdir(), 'cem-ml-browser-worker-'));
const bundleRoot = resolve(temporaryRoot, 'bundle');
let browser;
let server;
let inactiveCssReleased = false;
const pendingInactiveCss = new Set();
const sendInactiveCss = response => {
    response.writeHead(200, { 'content-type': 'text/css; charset=utf-8' });
    response.end('@import "grand.css"; p{color:blue}');
};

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
                input: { entry: resolve(projectRoot, 'tests/browser-worker-pool.fixture.mjs'), 'image-worker': resolve(projectRoot, 'tests/image-delivery-worker.fixture.mjs') },
                output: {
                    entryFileNames: '[name].js',
                    chunkFileNames: 'assets/[name]-[hash].js',
                    assetFileNames: 'assets/[name]-[hash][extname]',
                },
            },
        },
    });

    const deliveryRequests = new Set();
    const replayRequests = new Set();
    let selectionMode = 'normal';
    const pendingSelection = new Set();
    const sendNight = response => { response.writeHead(200, { 'content-type': 'text/css; charset=utf-8', 'cache-control': 'no-store' }); response.end('p{color:rgb(40,30,20)}'); };
    server = createServer((request, response) => {
        const pathname = new URL(request.url ?? '/', 'http://127.0.0.1').pathname;
        if (pathname.startsWith('/recorded-images/')) {
            const body = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAYAAACqaXHeAAAAl0lEQVR4nO3QoREAIBDAsNdoNPtvCWNEUBHf66x97s9GB2gN0AFaA3SA1gAdoDVAB2gN0AFaA3SA1gAdoDVAB2gN0AFaA3SA1gAdoDVAB2gN0AFaA3SA1gAdoDVAB2gN0AFaA3SA1gAdoDVAB2gN0AFaA3SA1gAdoDVAB2gN0AFaA3SA1gAdoDVAB2gN0AFaA3SA1gAdoD0H/LEefIE4owAAAABJRU5ErkJggg==', 'base64');
            const acquisition = request.headers['sec-fetch-dest'] !== 'image';
            if (pathname.endsWith('/replay.png')) {
                const bytes = replayRequests.has(request.url) ? Buffer.concat([body, Buffer.from('replayed-body')]) : body;
                replayRequests.add(request.url);
                response.writeHead(200, { 'content-type': 'image/png', 'cache-control': 'max-age=3600' }).end(bytes); return;
            }
            if (pathname.endsWith('/delivery.png')) {
                if (deliveryRequests.has(request.url)) { response.writeHead(404).end(); return; }
                deliveryRequests.add(request.url);
            }
            if (pathname.endsWith('/delivery-error.png')) { response.writeHead(404).end(); return; }
            if (pathname.endsWith('/delivery-redirect.png')) { response.writeHead(302, { location: '/recorded-images/a.png' }).end(); return; }
            if (pathname.endsWith('/delivery-oversized.png')) { response.writeHead(200, { 'content-type': 'image/png' }).end(Buffer.alloc(4*1024*1024+1)); return; }

            if (pathname.endsWith('/missing.png') || (pathname.endsWith('/acquisition-error.png') && acquisition)) {
                response.writeHead(404).end(); return;
            }
            if (pathname.endsWith('/redirect.png') && acquisition) {
                response.writeHead(302, { location: '/recorded-images/a.png' }).end(); return;
            }
            if (pathname.endsWith('/oversized.png') && acquisition) {
                response.writeHead(200, { 'content-type': 'image/png' }).end(Buffer.alloc(4 * 1024 * 1024 + 1)); return;
            }
            // A different body at the same URL must never establish historical identity.
            if (pathname.endsWith('/changed.png') && acquisition) {
                response.writeHead(200, { 'content-type': 'image/png', 'cache-control': 'no-store' }).end(Buffer.concat([body, Buffer.from('different response')])); return;
            }
            const send = () => response.writeHead(200, { 'content-type': 'image/png', 'cache-control': 'no-store' }).end(body);
            if ((pathname.endsWith('/pending.png') && acquisition) || pathname.endsWith('/delivery-pending.png')) setTimeout(send, 250); else send();
            return;
        }
        if (pathname === '/styles/recorded-error.css') {
            response.writeHead(404).end(); return;
        }
        if (pathname === '/styles/recorded-pending.css') {
            setTimeout(() => { response.writeHead(200, { 'content-type': 'text/css' }).end('p{color:rgb(20,30,40)}'); }, 250);
            return;
        }
        if (pathname === '/docs/browser-recorded-output.html') {
            response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
            response.end('<!doctype html><html><head></head><body></body></html>');
            return;
        }
        if (pathname === '/docs/browser-stylesheets.html') {
            response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
            response.end(`<!doctype html>${readFileSync(resolve(workspaceRoot, 'packages/cem_ml/tests/fixtures/browser-stylesheets.xhtml'), 'utf8').trim()}`);
            return;
        }
        if (['selection', 'active', 'inactive', 'missing', 'omitted'].some(kind => pathname === `/docs/browser-stylesheet-${kind}.html`)) {
            const kind = pathname.split('-').at(-1).replace('.html', '');
            const source = readFileSync(resolve(workspaceRoot, `packages/cem_ml/tests/fixtures/browser-stylesheet-${['selection', 'active'].includes(kind) ? kind : 'inactive'}.xhtml`), 'utf8').trim();
            response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
            response.end(`<!doctype html>${kind === 'missing' ? source.replace(' print;', ' screen;').replace('unloaded.css', 'missing.css') : kind === 'omitted' ? source.replace('print;', 'supports(display: cem-unimplemented);') : source}`);
            return;
        }
        if (pathname === '/styles/unloaded.css') {
            if (inactiveCssReleased) sendInactiveCss(response);
            else pendingInactiveCss.add(response);
            return;
        }
        if (pathname === '/release-inactive-css') {
            inactiveCssReleased = true;
            for (const pending of pendingInactiveCss) sendInactiveCss(pending);
            pendingInactiveCss.clear();
            response.writeHead(204).end();
            return;
        }
        if (pathname === '/styles/grand.css') {
            response.writeHead(200, { 'content-type': 'text/css; charset=utf-8' });
            response.end('p{border-style:solid}');
            return;
        }
        if (pathname === '/selection-mode') {
            selectionMode = new URL(request.url, 'http://localhost').searchParams.get('mode');
            if (selectionMode === 'normal') { for (const pending of pendingSelection) sendNight(pending); pendingSelection.clear(); }
            response.writeHead(204).end(); return;
        }
        if (pathname === '/styles/night.css') {
            if (selectionMode === 'slow') { pendingSelection.add(response); response.on('close', () => pendingSelection.delete(response)); }
            else if (selectionMode === 'error') response.writeHead(500, { 'cache-control': 'no-store' }).end();
            else if (selectionMode === 'changed') response.writeHead(200, { 'content-type': 'text/css', 'cache-control': 'no-store' }).end('p{color:red}');
            else sendNight(response);
            return;
        }
        if (pathname === '/styles/day.css' || pathname === '/styles/night-extra.css') {
            response.writeHead(200, { 'content-type': 'text/css; charset=utf-8', 'cache-control': 'no-store' });
            response.end(pathname.endsWith('/day.css') ? 'p{color:rgb(20,30,40)}' : '@import "grand.css" screen; p{background-color:rgb(4,5,6)}');
            return;
        }
        if (pathname === '/styles/theme.css' || pathname === '/styles/child.css') {
            response.writeHead(200, { 'content-type': 'text/css; charset=utf-8' });
            response.end(pathname.endsWith('/theme.css') ? '@import "child.css" supports(display: grid) screen and (min-width: 200px); p{background-color:rgb(4,5,6)}' : 'p{border-style:solid}');
            return;
        }
        if (pathname === '/a/html-base-host.html') {
            response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
            response.end(`<!doctype html>${readFileSync(resolve(workspaceRoot, 'packages/cem_ml/tests/fixtures/html-base-host.xhtml'), 'utf8').trim()}`);
            return;
        }
        if (pathname === '/docs/html-execution-inventory.html') {
            response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
            response.end(`<!doctype html>${readFileSync(resolve(workspaceRoot, 'packages/cem_ml/tests/fixtures/html-execution-inventory.xhtml'), 'utf8').trim()}`);
            return;
        }
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

    const hostConfiguration = await runFixture(page, 'command-host-configuration');
    assert.deepEqual(hostConfiguration, {
        rejected: ['cem.command.host_configuration_invalid', 'cem.command.host_configuration_invalid'],
        before: 0, callbacks: 1, status: 'succeeded',
    });

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

    const htmlBase = await verifyHtmlBaseParity(browser, workspaceRoot);
    const stylesheetSvg = await verifyStylesheetSvg(browser, workspaceRoot);
    const htmlBaseFreeze = await verifyHtmlBaseFreezeEvidence(browser, workspaceRoot);
    const browserStylesheets = await page.evaluate((version) => globalThis.runBrowserStylesheetsFixture(version), browser.version());
    assert.equal(browserStylesheets.cases.length, 18);
    console.log(`Verified ${browserStylesheets.cases.length} shared browser stylesheet host cases.`);
    const compiledStylesheets = await page.evaluate(() => globalThis.runCompiledStylesheetsFixture());
    assert.equal(compiledStylesheets.cases.length, 20);
    const recordedPreparations = await page.evaluate(version => globalThis.runRecordedOutputFixture(version), browser.version());
    assert.equal(recordedPreparations.cases.length, 124);
    console.log(`Verified ${recordedPreparations.cases.length} production native preparation host cases.`);
    const recordedOutput = await verifyRecordedOutput(browser, workspaceRoot);
    const stylesheetSelection = await page.evaluate((version) => globalThis.runStylesheetSelectionFixture(version), browser.version());
    assert.equal(stylesheetSelection.cases.length, 9);
    const activeStylesheets = await page.evaluate((version) => globalThis.runActiveStylesheetFixture(version), browser.version());
    assert.equal(activeStylesheets.cases.length, 19);
    console.log(`Verified ${activeStylesheets.cases.length} active stylesheet transition cases.`);
    console.log(`Verified ${compiledStylesheets.cases.length} compiled installation and ${stylesheetSelection.cases.length} selection cases.`);
    const htmlBaseHost = await page.evaluate((version) => globalThis.runHtmlBaseHostFixture(version), browser.version());
    assert.equal(htmlBaseHost.cases.length, 18);
    const htmlExecutionInventory = await emitNativeHtmlBaseEvidence(workspaceRoot, '--verify-execution-inventory', [htmlBaseHost.inventoryObservation]);
    assert.equal(htmlExecutionInventory[0].body_analysis_required, true);
    console.log(`Verified ${htmlBaseHost.cases.length} packaged native HTML base host cases.`);
    const parityReport = resolve(workspaceRoot, 'dist/reports/cem-ml-platform-parity.browser.json');
    mkdirSync(dirname(parityReport), { recursive: true });
    writeFileSync(
        parityReport,
        `${JSON.stringify(
            {
                schemaVersion: 1,
                host: 'wasm-browser-worker',
                capability: command.capability,
                identity: command.resultIdentity,
                successProgress: command.progress,
                operations: allOperations.summaries,
                cancellation,
                htmlBase,
                htmlBaseFreeze,
                htmlBaseHost,
                htmlExecutionInventory,
                stylesheetSvg,
                browserStylesheets,
                compiledStylesheets,
                stylesheetSelection,
                activeStylesheets,
                recordedOutput,
                recordedPreparations,
            },
            null,
            2,
        )}\n`,
    );

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
