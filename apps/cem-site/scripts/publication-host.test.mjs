import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';
import { publicationOrigin, verifyPublishedAssets } from './publication-host.mjs';

test('public Site acceptance preserves local default and normalizes only an HTTPS origin', () => {
    assert.equal(publicationOrigin(undefined), undefined);
    assert.equal(publicationOrigin('https://site.example/'), 'https://site.example');
    assert.equal(publicationOrigin('https://site.example'), 'https://site.example');
});
for (const value of ['http://site.example/', 'https://user:secret@site.example/', 'https://site.example/site/',
    'https://site.example/?q=test', 'https://site.example/#guide', 'https://site.example:8443/']) {
    test(`refuses ambiguous public Site target ${value.replace('secret', 'redacted')}`, () => assert.throws(() => publicationOrigin(value), /HTTPS origin/));
}
const bytes = Buffer.from('native publication');
const inventory = { aggregateSha256: 'fixture', files: [{ path: 'index.html', bytes: bytes.length,
    sha256: createHash('sha256').update(bytes).digest('hex') }] };
test('graph-emitted HTML is verified at its canonical public route without a redirect', async () => {
    const result = await verifyPublishedAssets('https://site.example', inventory, {
        routes: [{ kind: 'page', output: 'index.html', route: '/' }],
        fetchImpl: async url => { assert.equal(url.href, 'https://site.example/'); return new Response(bytes); },
    });
    assert.equal(result.files[0].requestPath, '/');
});
test('public asset checks require original bytes and a successful response', async () => {
    const result = await verifyPublishedAssets('https://site.example', inventory, { fetchImpl: async () => new Response(bytes) });
    assert.equal(result.files.length, 1);
    assert.equal(result.files[0].sha256, inventory.files[0].sha256);
    await assert.rejects(verifyPublishedAssets('https://site.example', inventory, {
        fetchImpl: async () => new Response('different content!', { status: 200 }),
    }), /byte verification failed/);
    await assert.rejects(verifyPublishedAssets('https://site.example', inventory, {
        fetchImpl: async () => new Response(bytes, { status: 404 }),
    }), /byte verification failed/);
});
