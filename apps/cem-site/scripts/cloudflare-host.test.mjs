import assert from 'node:assert/strict';
import { gzipSync, gunzipSync } from 'node:zlib';
import { test } from 'node:test';
import worker from '../../../tools/cloudflare/site-worker.mjs';

const original = Buffer.from('{"sources":["original native publication"]}');
const compressed = gzipSync(original);
const url = 'https://site.example/tokens/index.html.map?revision=one';
test('gzip transport retains the public source-map URL, original bytes and cache validators', async () => {
    let received;
    const response = await worker.fetch(new Request(url, { headers: { 'Accept-Encoding': 'br, gzip', Range: 'bytes=0-10' } }), {
        ASSETS: { fetch: async request => { received = request; return new Response(compressed, {
            headers: { ETag: 'native-map-v1', 'Cache-Control': 'public, max-age=0', 'Content-Type': 'application/gzip', 'Content-Length': String(compressed.length) },
        }); } },
    });
    assert.equal(received.url, url.replace('.map?', '.map.gz?'));
    assert.equal(received.headers.get('accept-encoding'), 'identity');
    assert.equal(received.headers.get('range'), null);
    assert.equal(response.headers.get('content-encoding'), 'gzip');
    assert.equal(response.headers.get('content-type'), 'application/json; charset=utf-8');
    assert.equal(response.headers.get('etag'), 'native-map-v1');
    assert.equal(response.headers.get('content-length'), null);
    assert.deepEqual(gunzipSync(Buffer.from(await response.arrayBuffer())), original);
});
test('identity-only clients receive original map bytes through streaming decompression', async () => {
    const response = await worker.fetch(new Request(url, { headers: { 'Accept-Encoding': 'gzip;q=0, identity' } }), {
        ASSETS: { fetch: async () => new Response(compressed) },
    });
    assert.equal(response.headers.get('content-encoding'), null);
    assert.deepEqual(Buffer.from(await response.arrayBuffer()), original);
});
for (const [method, status] of [['HEAD', 200], ['GET', 304], ['GET', 404]]) {
    test(`source-map ${method}/${status} preserves bodyless/cache/missing responses`, async () => {
        const response = await worker.fetch(new Request(url, { method }), { ASSETS: { fetch: async () => new Response(null, { status }) } });
        assert.equal(response.status, status); assert.equal(await response.text(), '');
    });
}
test('ordinary assets retain the existing redirect-free forwarding adapter', async () => {
    let received; const expected = new Response('native module');
    const response = await worker.fetch(new Request('https://site.example/assets/@epa-wg/module.js'), { ASSETS: { fetch: async request => { received = request; return expected; } } });
    assert.equal(response, expected); assert.equal(new URL(received.url).pathname, '/assets/%40epa-wg/module.js');
});
