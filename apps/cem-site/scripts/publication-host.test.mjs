import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { test } from 'node:test';
import { publicationOrigin, verifyPublishedAssets } from './publication-host.mjs';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { collectGalleryPublication, combinedPublication, publicationHash, assetPageRoute, readPublicationInventory, siteFileBudget, studioHostingHeaders, assertLocalDemoLinks } from './publication-host-galleries.mjs';

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
test('presentation HTML refuses former demo-host links while preserving relative URLs', () => {
    assertLocalDemoLinks(Buffer.from('<a href="/studio/">Studio</a>'), 'index.html');
    assert.throws(() => assertLocalDemoLinks(Buffer.from('<a href="https://green-morning-fd6e.trans-acf.workers.dev/studio/">Studio</a>'), 'index.html'), /Former demo-host link remains/);
    assert.throws(() => assertLocalDemoLinks(Buffer.from('<script src="//green-morning-fd6e.trans-acf.workers.dev/runtime.js"></script>'), 'page.xhtml'), /Former demo-host link remains/);
    assert.throws(() => assertLocalDemoLinks(Buffer.from('<a href=https://green-morning-fd6e.trans-acf.workers.dev/studio/>Studio</a>'), 'page.html'), /Former demo-host link remains/);
});
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

async function galleryFixture(t) {
    const root = await mkdtemp(join(tmpdir(), 'cem-site-gallery-'));
    t.after(() => rm(root, { recursive: true, force: true }));
    const demo = join(root, 'demo'), source = join(root, 'source');
    const content = {
        'packages/cem-components/index.html': 'component index',
        'packages/cem-components/playgrounds/cem-icon-gallery.html': 'icon gallery',
        'packages/cem-components/playgrounds/cem-gallery-nav.xhtml': 'shared header',
        'packages/cem-theme/dist/lib/tokens/catalog.json': 'complete public theme metadata',
        'packages/cem-elements/dist/runtime.js': 'shared runtime',
        'packages/cem-demo-element/dist/index.js': 'shared source viewer',
        'studio/index.html': 'same-origin Studio',
        'studio/build.json': '{}', 'studio/cache-inventory.json': '{}', 'studio/service-worker.js': 'scoped worker',
        'studio/security-headers.json': JSON.stringify({ headers: { 'Content-Security-Policy': "default-src 'self'", 'X-Content-Type-Options': 'nosniff' } }),
        'index.html': 'separate demo root', '_headers': 'separate headers',
    };
    const files = [];
    for (const [path, value] of Object.entries(content)) {
        for (const destination of [join(demo, path), join(source, path.startsWith('studio/') ? 'packages/cem-studio/dist/static/' + path.slice(7) : path)]) {
            await mkdir(dirname(destination), { recursive: true });
            await writeFile(destination, value);
        }
        files.push({ path, bytes: Buffer.byteLength(value), sha256: publicationHash(value) });
    }
    await writeFile(join(demo, 'demo-deployment.json'), JSON.stringify({ schemaVersion: 1, producer: 'cem-demo-site', fileCount: files.length + 1, files }));
    return { demo, source };
}
test('component publication preserves complete package trees and canonical same-origin routes', async t => {
    const { demo, source } = await galleryFixture(t);
    const gallery = await collectGalleryPublication(demo, { sourceRoot: source });
    assert.equal(gallery.fileCount, 11);
    assert.equal(gallery.galleryCount, 1);
    assert.ok(gallery.files.some(file => file.path.endsWith('tokens/catalog.json')));
    assert.equal(gallery.studioFileCount, 5);
    assert.ok(gallery.files.some(file => file.path === 'studio/index.html'));
    assert.ok(!gallery.files.some(file => file.path === 'index.html' || file.path === '_headers'));
    assert.deepEqual(gallery.galleryRoutes, ['/packages/cem-components/playgrounds/cem-icon-gallery']);
    assert.equal(assetPageRoute('packages/cem-components/index.html'), '/packages/cem-components/');
});
test('component publication refuses stale bytes and current-source drift', async t => {
    const { demo, source } = await galleryFixture(t);
    const path = 'packages/cem-components/playgrounds/cem-icon-gallery.html';
    await writeFile(join(source, path), 'updated canonical gallery');
    await assert.rejects(collectGalleryPublication(demo, { sourceRoot: source }), /differs from current source/);
    await writeFile(join(demo, path), 'changed deployment bytes');
    await assert.rejects(collectGalleryPublication(demo), /Stale prepared gallery asset/);
    await writeFile(join(demo, path), 'icon gallery');
    await writeFile(join(source, path), 'icon gallery');
    await writeFile(join(source, 'packages/cem-demo-element/dist/index.js'), 'updated source viewer');
    await assert.rejects(collectGalleryPublication(demo, { sourceRoot: source }), /differs from current source/);
});
test('component publication refuses native path collisions and omitted current galleries', async t => {
    const { demo, source } = await galleryFixture(t);
    await assert.rejects(collectGalleryPublication(demo, { reservedPaths: ['packages/cem-components/index.html'] }), /collides/);
    await writeFile(join(source, 'packages/cem-components/playgrounds/cem-action-gallery.html'), 'new gallery');
    await assert.rejects(collectGalleryPublication(demo, { sourceRoot: source }), /every current component gallery/);
});
test('combined inventory covers its gallery manifest and refuses duplicates or excessive assets', () => {
    const native = { aggregateSha256: 'native', files: [{ path: 'index.html', bytes: 3, sha256: publicationHash('one') }] };
    const gallery = { sourceInventorySha256: 'demo', files: [{ path: 'packages/gallery.html', bytes: 3, sha256: publicationHash('two') }], routes: [] };
    const bytes = Buffer.from(JSON.stringify(gallery));
    const combined = combinedPublication(native, gallery, bytes);
    assert.equal(combined.fileCount, 3);
    assert.equal(combined.files.find(file => file.path === 'component-gallery-publication.json').sha256, publicationHash(bytes));
    assert.throws(() => combinedPublication(native, { ...gallery, files: native.files }, bytes), /Duplicate/);
    assert.throws(() => combinedPublication(native, { ...gallery, files: Array.from({ length: siteFileBudget - 1 }, (_, i) => ({ path: `package/${i}` })) }, bytes), /1999-file budget/);
});
test('Studio publication refuses missing build assets, stale builds and incomplete current trees', async t => {
    const { demo, source } = await galleryFixture(t);
    await writeFile(join(source, 'packages/cem-studio/dist/static/index.html'), 'new Studio');
    await assert.rejects(collectGalleryPublication(demo, { sourceRoot: source }), /Studio differs from current build/);
    await writeFile(join(source, 'packages/cem-studio/dist/static/index.html'), 'same-origin Studio');
    await writeFile(join(source, 'packages/cem-studio/dist/static/new-module.js'), 'new module');
    await assert.rejects(collectGalleryPublication(demo, { sourceRoot: source }), /every current Studio asset/);
    const path = join(demo, 'demo-deployment.json');
    const inventory = JSON.parse(await readFile(path));
    inventory.files = inventory.files.filter(file => file.path !== 'studio/service-worker.js');
    inventory.fileCount--;
    await writeFile(path, JSON.stringify(inventory));
    await assert.rejects(collectGalleryPublication(demo), /Missing Studio asset: service-worker.js/);
});
test('Studio security headers apply only to its subtree and refuse malformed values', async t => {
    const { demo } = await galleryFixture(t);
    assert.equal((await studioHostingHeaders(demo)).toString(), "/studio/*\n  Content-Security-Policy: default-src 'self'\n  X-Content-Type-Options: nosniff\n");
    await writeFile(join(demo, 'studio/security-headers.json'), JSON.stringify({ headers: { 'Content-Security-Policy': "default-src 'self'\nInjected: value" } }));
    await assert.rejects(studioHostingHeaders(demo), /Invalid Studio security header/);
});
test('public verification refuses a changed combined manifest and lost native members', async t => {
    const { demo } = await galleryFixture(t);
    const native = { aggregateSha256: 'native', files: [{ path: 'index.html', bytes: 3, sha256: publicationHash('one') }] };
    const galleries = await collectGalleryPublication(demo);
    const publication = combinedPublication(native, galleries, Buffer.from('manifest'));
    const path = join(demo, 'publication.json');
    const bytes = Buffer.from(JSON.stringify(publication)); await writeFile(path, bytes);
    const prepared = { publicationInventory: { path, sha256: publicationHash(bytes), aggregateSha256: publication.aggregateSha256, fileCount: publication.fileCount } };
    assert.equal((await readPublicationInventory(prepared, native)).fileCount, publication.fileCount);
    await writeFile(path, bytes + '\n');
    await assert.rejects(readPublicationInventory(prepared, native), /inventory changed/);
    await writeFile(path, bytes);
    await assert.rejects(readPublicationInventory(prepared, { ...native, files: [{ ...native.files[0], sha256: 'changed' }] }), /lost native output/);
});
