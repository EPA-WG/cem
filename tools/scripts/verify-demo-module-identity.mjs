import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { createServer as createHttpsServer } from 'node:https';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { chromium } from 'playwright';
import worker from '../cloudflare/demo-worker.mjs';
import { createCemStudioPreviewHandler } from '../../packages/cem-studio/scripts/serve-preview.mjs';

const workspace = resolve(import.meta.dirname, '../..');
const tls = await mkdtemp(resolve(tmpdir(), 'cem-identity-tls-'));
execFileSync('openssl', ['req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', resolve(tls, 'key.pem'),
    '-out', resolve(tls, 'cert.pem'), '-days', '1', '-subj', '/CN=127.0.0.1', '-addext', 'subjectAltName=IP:127.0.0.1'], { stdio: 'ignore' });
const handler = createCemStudioPreviewHandler({ versionedAssets: false });
const backend = createServer((request, response) => {
    const url = new URL(request.url, 'http://localhost');
    if (url.pathname.includes('@')) {
        response.writeHead(307, { location: `${url.pathname.replaceAll('@', '%40')}${url.search}` }).end(); return;
    }
    if (!url.pathname.startsWith('/studio/')) { response.writeHead(404).end(); return; }
    request.url = `${url.pathname.slice('/studio'.length)}${url.search}`;
    void handler(request, response);
});
await new Promise(done => backend.listen(0, '127.0.0.1', done));
const environment = { ASSETS: { fetch: request => {
    const url = new URL(request.url);
    return fetch(`http://127.0.0.1:${backend.address().port}${url.pathname}${url.search}`, {
        method: request.method, headers: request.headers, redirect: 'manual',
    });
} } };
let adapter = false;
const frontend = createHttpsServer({ key: await readFile(resolve(tls, 'key.pem')), cert: await readFile(resolve(tls, 'cert.pem')) }, async (request, response) => {
    try {
        const input = new Request(`https://127.0.0.1:${frontend.address().port}${request.url}`, { method: request.method, headers: request.headers });
        const result = await (adapter ? worker.fetch(input, environment) : environment.ASSETS.fetch(input));
        response.writeHead(result.status, Object.fromEntries(result.headers));
        response.end(request.method === 'HEAD' ? undefined : Buffer.from(await result.arrayBuffer()));
    } catch (error) { response.writeHead(500).end(error.message); }
});
await new Promise(done => frontend.listen(0, '127.0.0.1', done));
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const report = { schemaVersion: 1, checkedAt: new Date().toISOString(), browser: browser.version(),
    isPublicAcceptance: false, fixture: 'Cloudflare-observed literal-at redirect; real Studio modules', results: [] };
try {
    for (const variant of ['redirect-baseline', 'asset-adapter']) {
        adapter = variant === 'asset-adapter';
        const context = await browser.newContext({ ignoreHTTPSErrors: true });
        try {
            const page = await context.newPage(), redirects = [], errors = [];
            const observation = { variant, redirects, errors };
            report.results.push(observation);
            page.on('pageerror', error => errors.push(error.message));
            page.on('requestfailed', request => errors.push(`${request.failure()?.errorText} ${request.url()}`));
            page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
            page.on('response', response => { if (response.status() === 307) redirects.push(response.url()); });
            await page.goto(`https://127.0.0.1:${frontend.address().port}/studio/`);
            await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication));
            const result = await page.evaluate(async () => {
                try {
                    const module = await import('@epa-wg/cem-studio/shell');
                    const first = await module.installCemStudioShellComponents();
                    const second = await module.installCemStudioShellComponents();
                    return { reused: first === second, errors: first.result.diagnostics.filter(item => ['error', 'fatal'].includes(item.severity)) };
                } catch (error) { return { error: error.message }; }
            });
            Object.assign(observation, result);
            if (adapter) { assert.deepEqual(result, { reused: true, errors: [] }); assert.deepEqual(redirects, []); }
            else { assert.ok(redirects.length > 0); assert.match(result.error, /already declared in this CEM scope/); }
            console.log(`${variant}: ${JSON.stringify(result)}; ${redirects.length} redirects`);
        } finally { await context.close(); }
    }
    report.status = 'passed';
} finally {
    const file = resolve(workspace, 'dist/reports/demo-site/module-identity.json');
    await mkdir(dirname(file), { recursive: true }); await writeFile(file, `${JSON.stringify(report, null, 2)}\n`);
    await browser.close(); frontend.closeAllConnections(); backend.closeAllConnections();
    await Promise.all([new Promise(done => frontend.close(done)), new Promise(done => backend.close(done))]);
    await rm(tls, { recursive: true, force: true });
}
