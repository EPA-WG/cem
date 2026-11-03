import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { demoHostingMode, assertDemoStudioPath, verifyRemoteDemoAssets } from './demo-hosting-verification.mjs';

const critical = ['packages/cem-demo-element/demo/playground.html', 'studio/module-map.json', 'studio/cache-inventory.json',
    'studio/samples/examples/login.cem-studio.json', 'studio/samples/examples/supplier-pipeline.cem-studio.json',
    'packages/cem-ml-npm/dist/wasm/browser/cem_ml_factory.js', 'packages/cem_ql/dist/wasm/cem_ql_factory.js', 'studio/runtime.wasm'];
async function fixture(t) {
    const destination = await mkdtemp(join(tmpdir(), 'cem-demo-public-'));
    t.after(() => rm(destination, { recursive: true, force: true }));
    const files = critical.map(path => ({ path, content: Buffer.from(path) }));
    const inventory = Buffer.from(JSON.stringify({ schemaVersion: 1, producer: 'cem-demo-site', fileCount: files.length + 1,
        files: files.map(({path, content}) => ({path, bytes: content.length, sha256: createHash('sha256').update(content).digest('hex')})) }));
    await writeFile(join(destination, 'demo-deployment.json'), inventory);
    const remote = new Map(files.map(({path,content}) => [path, content])); remote.set('demo-deployment.json', inventory);
    const requests = [];
    const request = async (url, options) => {
        requests.push({path:url.pathname.slice(1),options});
        const bytes = remote.get(url.pathname.slice(1));
        return new Response(bytes ?? 'Not found', { status: bytes ? 200 : 404 });
    };
    return { destination, remote, request, requests, url: 'https://example.test/' };
}
test('production and local preview each require their own exact Studio subtree', () => {
    assert.equal(demoHostingMode(), 'local-preview'); assert.equal(demoHostingMode('production'), 'production');
    assert.throws(() => demoHostingMode('any'), /CEM_DEMO_HOSTING/);
    assertDemoStudioPath('/studio/', 'production');
    assertDemoStudioPath('/studio/__cem_preview__/0123456789abcdef/', 'local-preview');
    for (const path of ['/studio', '/other/', '/studio/__cem_preview__/0123456789abcdef/']) assert.throws(() => assertDemoStudioPath(path, 'production'));
    for (const path of ['/studio/', '/studio/__cem_preview__/bad/', '/other/__cem_preview__/0123456789abcdef/']) assert.throws(() => assertDemoStudioPath(path, 'local-preview'));
});
test('matching public inventory plus original critical bytes still does not claim interaction acceptance', async t => {
    const f = await fixture(t), result = await verifyRemoteDemoAssets(f);
    assert.equal(result.fileCount, 9); assert.equal(result.checkedRemoteFiles.length, 9);
    assert.equal(result.interactionAcceptance, 'not-established-by-byte-checks');
    assert.ok(f.requests.every(item => item.options.cache === 'no-store' && item.options.signal));
});
test('stale inventory stops before fetching critical assets', async t => {
    const f = await fixture(t); f.remote.set('demo-deployment.json', Buffer.from('{}'));
    await assert.rejects(verifyRemoteDemoAssets(f), /Stale public demo/);
    assert.equal(f.requests.length, 1);
});
test('matching inventory cannot hide a same-size changed critical asset', async t => {
    const f = await fixture(t); const path = critical[0], bytes = Buffer.from(f.remote.get(path)); bytes[0] ^= 1; f.remote.set(path, bytes);
    await assert.rejects(verifyRemoteDemoAssets(f), /Stale public demo hash/);
});
test('HTTP 200 fallback HTML cannot stand in for an actual asset', async t => {
    const f = await fixture(t); f.remote.set(critical[1], Buffer.from('<html>Fallback</html>'));
    await assert.rejects(verifyRemoteDemoAssets(f), /Stale public demo/);
});
test('missing public assets and non-root origins are refused', async t => {
    const f = await fixture(t); f.remote.delete(critical[0]);
    await assert.rejects(verifyRemoteDemoAssets(f), /Public demo HTTP failure/);
    await assert.rejects(verifyRemoteDemoAssets({...f,url:'https://example.test/studio/'}), /deployment root/);
});
