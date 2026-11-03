import assert from 'node:assert/strict';
import { test } from 'node:test';
import worker from '../cloudflare/demo-worker.mjs';

test('literal and encoded package URLs reach one asset without a browser redirect', async () => {
    for (const path of ['/studio/assets/@epa-wg/cem-studio/shell.js', '/studio/assets/%40epa-wg/cem-studio/shell.js']) {
        const response = await worker.fetch(new Request(`https://example.test${path}?edition=1`), {
            ASSETS: { fetch: request => new URL(request.url).pathname.includes('@')
                ? Response.redirect('https://example.test/studio/assets/%40epa-wg/cem-studio/shell.js', 307)
                : new Response('export const marker = 1;', { headers: { 'content-security-policy': "default-src 'none'" } }) },
        });
        assert.equal(response.status, 200);
        assert.equal(response.headers.get('location'), null);
        assert.equal(response.headers.get('content-security-policy'), "default-src 'none'");
        assert.equal(await response.text(), 'export const marker = 1;');
    }
});

test('asset forwarding retains method, query, headers and already encoded paths', async () => {
    let forwarded;
    const request = new Request('https://example.test/studio/assets/@scope/file%20name.js?mode=read', {
        method: 'HEAD', headers: { 'if-none-match': 'edition-one', 'accept-encoding': 'br' },
    });
    const response = new Response(null, { status: 304, headers: { etag: 'edition-one' } });
    assert.equal(await worker.fetch(request, { ASSETS: { fetch: value => { forwarded = value; return response; } } }), response);
    assert.equal(forwarded.url, 'https://example.test/studio/assets/%40scope/file%20name.js?mode=read');
    assert.equal(forwarded.method, 'HEAD');
    assert.equal(forwarded.headers.get('if-none-match'), 'edition-one');
    assert.equal(forwarded.headers.get('accept-encoding'), 'br');
    const ordinary = new Request('https://example.test/studio/');
    await worker.fetch(ordinary, { ASSETS: { fetch: value => { assert.equal(value, ordinary); return response; } } });
});
