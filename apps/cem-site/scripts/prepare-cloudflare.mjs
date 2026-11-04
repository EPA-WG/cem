import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile, mkdir, rm, cp } from 'node:fs/promises';
import { gzipSync, gunzipSync } from 'node:zlib';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { compressedSitePaths } from '../../../tools/cloudflare/site-worker.mjs';
import { collectGalleryPublication, combinedPublication, galleryManifestPath, studioHostingHeaders, assertLocalDemoLinks } from './publication-host-galleries.mjs';

const workspace = resolve(import.meta.dirname, '../../..');
const nativeRoot = resolve(workspace, 'dist/apps/cem-site');
const output = resolve(workspace, 'dist/hosting/cem-site');
const release = resolve(workspace, 'dist/releases/cem-site-cloudflare');
const archive = resolve(workspace, 'dist/releases/cem-site-cloudflare.tgz');
const inventoryBytes = await readFile(resolve(workspace, 'dist/reports/cem-site/determinism.json'));
const inventory = JSON.parse(inventoryBytes);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const galleryRoot = resolve(process.env.CEM_SITE_GALLERY_ROOT ?? resolve(workspace, '../cem-bin'));
const galleries = await collectGalleryPublication(galleryRoot, { sourceRoot: workspace,
    reservedPaths: inventory.files.map(file => file.path) });
const galleryBytes = Buffer.from(JSON.stringify(galleries, null, 2) + '\n');
const publication = combinedPublication(inventory, galleries, galleryBytes);
const publicationBytes = Buffer.from(JSON.stringify(publication, null, 2) + '\n');
const hosted = [];
const headerBytes = await studioHostingHeaders(galleryRoot);
await rm(output, { recursive: true, force: true });
for (const file of inventory.files) {
    const bytes = await readFile(resolve(nativeRoot, file.path));
    assertLocalDemoLinks(bytes, file.path);
    assert.ok(bytes.length === file.bytes && hash(bytes) === file.sha256, `Stale native Site artifact: ${file.path}`);
    const compressed = compressedSitePaths.includes(file.path);
    const content = compressed ? gzipSync(bytes, { level: 9 }) : bytes;
    if (compressed) assert.ok(gunzipSync(content).equals(bytes), `Compressed source-map round trip failed: ${file.path}`);
    assert.ok(content.length <= 25 * 1024 * 1024, `Cloudflare asset exceeds 25 MiB: ${file.path}`);
    const path = file.path + (compressed ? '.gz' : '');
    const destination = resolve(output, path); await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, content);
    hosted.push({ path, bytes: content.length, sha256: hash(content), nativePath: file.path, compressed });
}
assert.ok(compressedSitePaths.every(path => inventory.files.some(file => file.path === path)), 'Compressed route no longer belongs to the native publication');
for (const file of galleries.files) {
    const bytes = await readFile(resolve(galleryRoot, file.path));
    assert.ok(bytes.length === file.bytes && hash(bytes) === file.sha256, `Gallery changed during packaging: ${file.path}`);
    assert.ok(bytes.length <= 25 * 1024 * 1024, `Cloudflare asset exceeds 25 MiB: ${file.path}`);
    const destination = resolve(output, file.path); await mkdir(dirname(destination), { recursive: true });
    await writeFile(destination, bytes);
    hosted.push({ ...file, nativePath: null, compressed: false });
}
await writeFile(resolve(output, galleryManifestPath), galleryBytes);
hosted.push({ path: galleryManifestPath, bytes: galleryBytes.length, sha256: hash(galleryBytes), nativePath: null, compressed: false });
// Asset-service control files are verified separately; they are not public URLs.
await writeFile(resolve(output, '_headers'), headerBytes);
const hostingControls = [{ path: '_headers', bytes: headerBytes.length, sha256: hash(headerBytes) }];
await rm(release, { recursive: true, force: true });
await mkdir(resolve(release, 'dist/hosting'), { recursive: true });
await cp(output, resolve(release, 'dist/hosting/cem-site'), { recursive: true });
const sources = [];
for (const path of ['wrangler.site.jsonc', 'tools/cloudflare/site-worker.mjs', 'tools/cloudflare/demo-worker.mjs']) {
    const bytes = await readFile(resolve(workspace, path)); const destination = resolve(release, path);
    await mkdir(dirname(destination), { recursive: true }); await writeFile(destination, bytes);
    sources.push({ path, bytes: bytes.length, sha256: hash(bytes) });
}
await writeFile(resolve(release, 'native-publication-inventory.json'), inventoryBytes);
await writeFile(resolve(release, 'publication-inventory.json'), publicationBytes);
await writeFile(resolve(release, 'README.txt'), 'CEM presentation Site with local Studio, Playground, galleries and colored snippets\n\nThis bundle deploys cem-amsterdam-2026, including /studio/, /packages/cem-demo-element/demo/playground and every component gallery. All former demo-host links are relative.\n\nFrom this extracted folder, deploy using your existing Cloudflare login:\n\n  npx wrangler@4 deploy --config wrangler.site.jsonc\n\nShare the actual public URL and Wrangler version ID for public verification.\nNo native build or separate demo deployment is needed for this deploy-only bundle.\n');
const packed = spawnSync('tar', ['-czf', archive, '-C', release, '.'], { stdio: 'inherit' });
assert.equal(packed.status, 0, 'Could not package the Cloudflare deploy-only bundle');
const archiveBytes = await readFile(archive);
const report = { schemaVersion: 1, preparedAt: new Date().toISOString(), status: 'prepared; not publicly verified',
    nativeAggregateSha256: inventory.aggregateSha256, nativeFileCount: inventory.fileCount,
    publicationInventory: { path: resolve(release, 'publication-inventory.json'), sha256: hash(publicationBytes),
        aggregateSha256: publication.aggregateSha256, fileCount: publication.fileCount },
    galleries: { sourceRoot: galleryRoot, sourceInventorySha256: galleries.sourceInventorySha256,
        fileCount: galleries.fileCount, galleryCount: galleries.galleryCount, studioFileCount: galleries.studioFileCount, manifestPath: galleryManifestPath },
    hostedFileCount: hosted.length, hostingControls, compressed: hosted.filter(file => file.compressed),
    hostedAggregateSha256: hash(JSON.stringify(hosted)), sources,
    archive: { path: archive, bytes: archiveBytes.length, sha256: hash(archiveBytes) },
    output, release, publicAcceptance: 'pending owner deployment and public checks' };
await writeFile(resolve(workspace, 'dist/reports/cem-site/cloudflare-preparation.json'), JSON.stringify(report, null, 2) + '\n');
console.log(JSON.stringify(report, null, 2));
