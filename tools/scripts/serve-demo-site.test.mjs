import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtemp, mkdir, writeFile, symlink, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { createDemoSiteHandler } from './serve-demo-site.mjs';

async function fixture(t) {
    const directory = await mkdtemp(join(tmpdir(), 'cem-demo-host-'));
    const root = join(directory, 'site');
    await mkdir(join(root, 'studio/assets'), { recursive: true });
    await writeFile(join(root, 'index.html'), '<a href="./studio/">CEM Studio</a>');
    await writeFile(join(root, 'studio/index.html'), '<title>CEM Studio</title>');
    await writeFile(join(root, 'studio/security-headers.json'), JSON.stringify({ headers: {
        'Content-Security-Policy': "default-src 'none'", 'X-Content-Type-Options': 'nosniff',
    } }));
    await writeFile(join(root, 'studio/service-worker.js'), '// scoped worker');
    await writeFile(join(root, 'studio/manifest.webmanifest'), '{"scope":"./"}');
    await writeFile(join(root, 'studio/assets/runtime.wasm'), new Uint8Array([0, 97, 115, 109]));
    const server = createServer(createDemoSiteHandler({ root }));
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    t.after(async () => { server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); await rm(directory, { recursive: true, force: true }); });
    return { root, directory, url: `http://127.0.0.1:${server.address().port}` };
}

test('serves home and redirects Studio directories with query selection intact', async t => {
    const { url } = await fixture(t);
    const home = await fetch(url); assert.equal(home.status, 200);
    assert.equal(await home.text(), '<a href="./studio/">CEM Studio</a>');
    const redirect = await fetch(`${url}/studio?sample=login`, { redirect: 'manual' });
    assert.equal(redirect.status, 308); assert.equal(redirect.headers.get('location'), '/studio/?sample=login');
    const studio = await fetch(`${url}/studio/`); assert.equal(await studio.text(), '<title>CEM Studio</title>');
});

test('keeps Studio security headers scoped and does not broaden service-worker control', async t => {
    const { url } = await fixture(t);
    assert.equal((await fetch(url)).headers.get('content-security-policy'), null);
    const worker = await fetch(`${url}/studio/service-worker.js`);
    assert.equal(worker.headers.get('content-security-policy'), "default-src 'none'");
    assert.equal(worker.headers.get('x-content-type-options'), 'nosniff');
    assert.equal(worker.headers.get('service-worker-allowed'), null);
    assert.match(worker.headers.get('content-type'), /javascript/);
});

test('delivers WASM and manifest MIME types and honors HEAD requests', async t => {
    const { url } = await fixture(t);
    const wasm = await fetch(`${url}/studio/assets/runtime.wasm`);
    assert.equal(wasm.headers.get('content-type'), 'application/wasm');
    assert.deepEqual(new Uint8Array(await wasm.arrayBuffer()), new Uint8Array([0, 97, 115, 109]));
    assert.equal((await fetch(`${url}/studio/manifest.webmanifest`)).headers.get('content-type'), 'application/manifest+json');
    const head = await fetch(`${url}/studio/`, { method: 'HEAD' }); assert.equal(head.status, 200); assert.equal(await head.text(), '');
});

test('refuses encoded traversal, external symlinks, malformed paths and unsupported methods', async t => {
    const { url, root, directory } = await fixture(t);
    await writeFile(join(directory, 'secret.txt'), 'outside');
    await symlink(join(directory, 'secret.txt'), join(root, 'escape.txt'));
    assert.equal((await fetch(`${url}/%2e%2e%2fsecret.txt`)).status, 403);
    assert.equal((await fetch(`${url}/escape.txt`)).status, 403);
    assert.equal((await fetch(`${url}/%ZZ`)).status, 400);
    assert.equal((await fetch(`${url}/missing`)).status, 404);
    assert.equal((await fetch(url, { method: 'POST' })).status, 405);
});
