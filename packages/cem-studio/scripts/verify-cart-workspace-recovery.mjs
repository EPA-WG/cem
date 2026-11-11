import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';
import { createInteractiveCartService } from './verify-interactive-cart.mjs';

// Actual Studio/native workbench plus a second shared production media view.
// Only this trusted Node driver sends browser input; application markup remains
// script-free. Fresh pages own their heaps, storage and temporary repositories.
const service = await createInteractiveCartService();
const workspace = resolve(import.meta.dirname, '../../..');
const server = createServer(async (request, response) => {
    try {
        if (await service.handle(request, response)) return;
        if (request.url === '/workspace-recovery') {
            response.writeHead(200, { 'content-type': 'text/html' });
            response.end('<!doctype html><title>Studio recovery acceptance</title>'); return;
        }
        const path = resolve(workspace, '.' + decodeURIComponent(new URL(request.url, 'http://local').pathname));
        if (!path.startsWith(workspace + sep)) throw Error('outside workspace');
        const bytes = await readFile(path);
        response.writeHead(200, { 'content-type': ({ '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm',
            '.json': 'application/json', '.xhtml': 'application/xhtml+xml', '.css': 'text/css' })[extname(path)] ?? 'text/plain' });
        response.end(bytes);
    } catch (error) { response.writeHead(404); response.end(String(error)); }
});
let browser;
try {
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ executablePath: process.env.CEM_STUDIO_CHROME });
    const modes = ['worker', 'fallback'].filter(mode => !process.env.CEM_RECOVERY_MODE || process.env.CEM_RECOVERY_MODE === mode);
    assert.ok(modes.length, 'Unknown CEM_RECOVERY_MODE');
    for (const mode of modes) {
        service.reset(); service.requests.length = 0;
        const page = await browser.newPage();
        page.setDefaultTimeout(15_000);
        const attempts = [];
        page.on('request', request => { if (new URL(request.url()).pathname === '/graphql') attempts.push(request.method()); });
        page.on('console', message => { if (message.type() === 'error' || message.text().startsWith('[studio-cart:')) console.log(message.text()); });
        page.on('pageerror', error => console.error(error));
        let baseline;
        const cart = () => page.locator('cem-media-preview[label="Retained interactive cart"]').frameLocator('iframe');
        const quantity = () => cart().getByRole('spinbutton', { name: 'Quantity' }).first();
        await page.exposeBinding('studioCartPhase', async (_, phase) => {
            console.log(`[studio-cart:${mode}] ${phase}`);
            if (phase === 'ready') {
                assert.equal(await quantity().inputValue(), '2');
                await quantity().fill('3'); await quantity().press('Tab');
                await cart().getByText('USD 27.00', { exact: true }).waitFor();
                baseline = { requests: structuredClone(service.requests), attempts: attempts.length };
            } else if (phase === 'credential-refused') {
                assert.deepEqual(service.requests, baseline.requests); assert.equal(attempts.length, baseline.attempts);
                assert.equal(await quantity().inputValue(), '3');
            } else if (phase.startsWith('before ')) {
                const cycle = Number(phase.slice(7));
                await quantity().fill(String(4 + cycle)); await quantity().press('Tab');
                await cart().getByText(`USD ${((4 + cycle) * 9).toFixed(2)}`, { exact: true }).waitFor();
                baseline = { requests: structuredClone(service.requests), attempts: attempts.length };
            } else if (phase.startsWith('captured ')) {
                // The response total may precede native field publication.
                // Original complete capture/drain establishes quiescence;
                // retain controls at that boundary before any retirement.
                await page.locator('cem-media-preview[label="Retained interactive cart"] iframe').evaluate((frame, expected) => {
                    const document = frame.contentDocument, input = document.querySelector('input[type=number]');
                    if (!input || input.value !== expected) throw Error('Sealed cart lacks its accepted quantity control');
                    globalThis.originalCartFrame = { frame, document, input };
                }, String(4 + Number(phase.slice(9))));
            } else if (phase.startsWith('after ')) {
                assert.deepEqual(service.requests, baseline.requests); assert.equal(attempts.length, baseline.attempts);
                assert.equal(await quantity().inputValue(), String(4 + Number(phase.slice(6))));
                assert.deepEqual(await page.evaluate(() => {
                    const original = globalThis.originalCartFrame;
                    return { connected: original.frame.isConnected, document: original.frame.contentDocument === original.document,
                        input: original.document.querySelector('input[type=number]') === original.input };
                }), { connected: true, document: true, input: true });
            } else if (phase === 'complete') {
                await cart().getByRole('button', { name: 'Remove', exact: true }).click();
                await cart().getByText('Your cart is empty', { exact: true }).waitFor();
                assert.equal(service.requests.at(-1).operation, 'RemoveCartLine');
                assert.equal(service.requests.filter(r => r.method === 'POST').length, 5);
                assert.equal(new Set(service.deliveries.map(d => d.key)).size, 5);
            }
        });
        await page.goto(`${origin}/workspace-recovery`);
        let deadline;
        const result = await Promise.race([page.evaluate(async ({ mode, origin }) => {
            const { studioRecovery } = await import('/packages/cem-studio/src/recovery.fixture.js');
            const baseUrl = origin + '/packages/cem-studio/dist/cart-preview/';
            const deployment = await fetch(baseUrl + 'build.json').then(response => response.json());
            return studioRecovery(mode, stage => console.info(`[studio-cart:${mode}] ${stage}`), true, true,
                { baseUrl, commandUrl: baseUrl + 'assets/@epa-wg/cem-ml-cli/dist/browser-commands.js', grant: deployment.previewApplications[0], bundleUrl: baseUrl + 'samples/examples/shopping-cart.cem-studio.json',
                    phase: phase => globalThis.studioCartPhase(phase) });
        }, { mode, origin }), new Promise((_, reject) => { deadline = setTimeout(() => reject(Error(`${mode}: complete Studio recovery exceeded 110 seconds`)), 110_000); })]).finally(() => clearTimeout(deadline));
        assert.equal(result.renderingDomains, 4); assert.ok(result.sourceRetainedBytes > 16 * 1024);
        assert.equal(result.recoveries, 3); assert.equal(result.edits, 12); assert.equal(result.privacyRefusals, 2);
        assert.deepEqual(result.physicalGroups, mode === 'worker' ? [7, 7, 7] : [4, 4, 4]);
        assert.ok(result.groupBytes > result.checkpointBytes && result.groupBytes <= 4 * 1024 * 1024);
        console.log(`Studio cart ${mode}: ${JSON.stringify(result)}`);
        await page.close();
    }
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
