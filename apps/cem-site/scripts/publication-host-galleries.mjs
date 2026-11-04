import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, readdir, lstat } from 'node:fs/promises';
import { resolve } from 'node:path';
import { readDemoInventory } from '../../../tools/scripts/demo-deployment.mjs';

export const galleryManifestPath = 'component-gallery-publication.json';
export const publicationHash = bytes => createHash('sha256').update(bytes).digest('hex');
export const assetPageRoute = path => '/' + (path.endsWith('/index.html') ? path.slice(0, -10)
    : path === 'index.html' ? '' : path.endsWith('.html') ? path.slice(0, -5) : path);

// Preserve the complete prepared package trees, including their relative imports.
// The demo's root page, hosting headers and Studio remain separately owned.
export async function collectGalleryPublication(root, { sourceRoot, reservedPaths = [] } = {}) {
    const inventory = await readDemoInventory(root);
    assert.ok(inventory, 'Prepare the demo first with yarn copy:demo, or set CEM_SITE_GALLERY_ROOT to its verified folder');
    const reserved = new Set([...reservedPaths, galleryManifestPath]);
    const files = inventory.files.filter(file => file.path.startsWith('packages/') || file.path.startsWith('docs/') || file.path === 'LICENSE');
    const names = files.filter(file => /^packages\/cem-components\/playgrounds\/cem-[\w-]+-gallery\.html$/u.test(file.path))
        .map(file => file.path.split('/').at(-1)).sort();
    assert.ok(names.length && files.some(file => file.path === 'packages/cem-components/index.html'), 'Missing component gallery index or gallery pages');
    for (const file of files) {
        assert.ok(!reserved.has(file.path), `Gallery path collides with native Site output: ${file.path}`);
        const path = resolve(root, file.path);
        assert.ok((await lstat(path)).isFile(), `Gallery asset must be a regular file: ${file.path}`);
        const bytes = await readFile(path);
        assert.ok(bytes.length === file.bytes && publicationHash(bytes) === file.sha256, `Stale prepared gallery asset: ${file.path}`);
        if (sourceRoot && ((file.path.startsWith('packages/cem-components/') && !file.path.startsWith('packages/cem-components/dist/'))
            || file.path.startsWith('packages/cem-demo-element/dist/'))) {
            assert.ok(bytes.equals(await readFile(resolve(sourceRoot, file.path))), `Gallery differs from current source: ${file.path}; refresh with yarn copy:demo`);
        }
    }
    if (sourceRoot) {
        const current = (await readdir(resolve(sourceRoot, 'packages/cem-components/playgrounds')))
            .filter(name => /^cem-[\w-]+-gallery\.html$/u.test(name)).sort();
        assert.deepEqual(names, current, 'Prepared demo must include every current component gallery');
    }
    return {
        schemaVersion: 1, kind: 'cem-component-galleries',
        sourceInventorySha256: publicationHash(await readFile(resolve(root, 'demo-deployment.json'))),
        fileCount: files.length, galleryCount: names.length,
        galleryRoutes: names.map(name => assetPageRoute('packages/cem-components/playgrounds/' + name)),
        files,
        routes: files.filter(file => file.path.endsWith('.html'))
            .map(file => ({ kind: 'page', output: file.path, route: assetPageRoute(file.path) })),
    };
}

export function combinedPublication(native, galleries, manifestBytes) {
    const files = [...native.files, ...galleries.files, {
        path: galleryManifestPath, bytes: manifestBytes.length, sha256: publicationHash(manifestBytes),
    }].sort((a, b) => a.path.localeCompare(b.path));
    assert.equal(new Set(files.map(file => file.path)).size, files.length, 'Duplicate combined publication paths');
    assert.ok(files.length <= 999, 'Combined Site publication exceeds the 999-file budget');
    return { schemaVersion: 1, nativeAggregateSha256: native.aggregateSha256,
        gallerySourceInventorySha256: galleries.sourceInventorySha256,
        fileCount: files.length, aggregateSha256: publicationHash(JSON.stringify(files)),
        files, galleryRoutes: galleries.routes };
}

export async function readPublicationInventory(prepared, native) {
    assert.ok(prepared.publicationInventory, 'Prepare the combined Site/gallery publication first');
    const bytes = await readFile(prepared.publicationInventory.path);
    assert.equal(publicationHash(bytes), prepared.publicationInventory.sha256, 'Prepared publication inventory changed');
    const publication = JSON.parse(bytes);
    assert.equal(publication.nativeAggregateSha256, native.aggregateSha256, 'Combined publication uses a different native release');
    assert.equal(publication.aggregateSha256, publicationHash(JSON.stringify(publication.files)), 'Combined publication inventory drifted');
    assert.equal(publication.aggregateSha256, prepared.publicationInventory.aggregateSha256);
    assert.equal(publication.files.length, prepared.publicationInventory.fileCount);
    assert.equal(new Set(publication.files.map(file => file.path)).size, publication.files.length, 'Duplicate publication paths');
    const byPath = new Map(publication.files.map(file => [file.path, file]));
    for (const file of native.files) assert.deepEqual(byPath.get(file.path), file, `Combined publication lost native output: ${file.path}`);
    return publication;
}
