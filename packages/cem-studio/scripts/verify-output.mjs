import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const runtimeRoot = resolve(projectRoot, 'runtime');
const outputRoot = resolve(projectRoot, 'dist/static');
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/output.json');
const sourceMap = await readJson(resolve(runtimeRoot, 'source.module-map.json'));
const destinationMap = await readJson(resolve(runtimeRoot, 'destination.module-map.json'));
const buildReport = await readJson(resolve(workspaceRoot, 'dist/reports/cem-studio/build.json'));
const packageMetadata = await readJson(resolve(projectRoot, 'package.json'));
const buildScript = await readFile(resolve(projectRoot, 'scripts/build.mjs'), 'utf8');

assert.equal(sourceMap.$schema, 'https://cem.dev/ns/data/module-map/2');
assert.equal(destinationMap.$schema, sourceMap.$schema);
assert.deepEqual(Object.keys(destinationMap.imports).sort(), Object.keys(sourceMap.imports).sort());
assert.deepEqual(Object.keys(destinationMap.resources).sort(), Object.keys(sourceMap.resources).sort());
assert.equal(buildReport.summary.errorCount, 0);
assert.equal(buildReport.summary.fatalCount, 0);
assert.match(buildScript, /'--config',\s*'packages\/cem-studio\/studio\.cem'/);
assert.doesNotMatch(buildScript, /copyFile|\bcp\b|vite|rollup|webpack/);

const declaredAssets = [
    ...Object.entries(sourceMap.imports).map(([identity, source]) => ({
        identity,
        source,
        target: destinationMap.imports[identity],
        contentType: 'text/javascript',
    })),
    ...Object.entries(sourceMap.resources).map(([identity, resource]) => ({
        identity,
        source: resource.path,
        target: destinationMap.resources[identity].path,
        contentType: resource.contentType,
    })),
];

for (const asset of declaredAssets) {
    assert.ok(asset.target.startsWith('./'), `${asset.identity} target is not app-relative`);
    const sourceBytes = await readFile(resolve(runtimeRoot, asset.source));
    const outputBytes = await readFile(resolve(outputRoot, asset.target.slice(2)));
    assert.deepEqual(outputBytes, sourceBytes, `${asset.identity} graph emission changed opaque bytes`);
    if (sourceMap.resources[asset.identity]) {
        assert.equal(destinationMap.resources[asset.identity].contentType, asset.contentType);
    }
}

const html = await readFile(resolve(outputRoot, 'index.html'), 'utf8');
for (const [specifier, target] of Object.entries(destinationMap.imports)) {
    assert.match(html, new RegExp(`${escapeRegex(specifier)}[^<]+${escapeRegex(target)}`, 's'));
}
assert.doesNotMatch(html, /node_modules/);
assert.match(html, /import '@epa-wg\/cem-studio\/main'/);
assert.match(html, /data-cem-studio-root/);

const [manifest, buildMetadata, inventory, runtimeMetadata, themeMetadata] = await Promise.all([
    readJson(resolve(outputRoot, 'manifest.webmanifest')),
    readJson(resolve(outputRoot, 'build.json')),
    readJson(resolve(outputRoot, 'cache-inventory.json')),
    readJson(resolve(outputRoot, 'assets/@epa-wg/cem-ml/dist/cem-ml-runtime.json')),
    readJson(resolve(outputRoot, 'assets/@epa-wg/cem-theme/package.json')),
]);
assert.equal(manifest.start_url, './');
assert.equal(manifest.icons[0].src, './icon.svg');
assert.equal(buildMetadata.package, packageMetadata.name);
assert.equal(buildMetadata.commonVersion, packageMetadata.version);
assert.equal(inventory.commonVersion, packageMetadata.version);
assert.equal(runtimeMetadata.commonVersion, packageMetadata.version);
assert.equal(themeMetadata.version, packageMetadata.dependencies['@epa-wg/cem-theme']);
assert.equal(packageMetadata.exports['.'].import, './dist/static/assets/@epa-wg/cem-studio/bootstrap.js');

const moduleAssetManifest = findModuleAssetManifest(buildReport);
assert.ok(moduleAssetManifest, 'build report is missing the CEM-ML module-asset manifest');
assert.equal(moduleAssetManifest.assetCount, declaredAssets.length);
assert.equal(moduleAssetManifest.hashScheme, 'sha256');
assert.equal(moduleAssetManifest.hash.length, 64);

const files = await filesUnder(outputRoot);
for (const required of [
    'index.html',
    'manifest.webmanifest',
    'build.json',
    'cache-inventory.json',
    'icon.svg',
    'service-worker.js',
    ...declaredAssets.map(({ target }) => target.slice(2)),
]) {
    assert.ok(files.includes(required), `static output is missing ${required}`);
}

const report = {
    schemaVersion: 1,
    project: packageMetadata.name,
    commonVersion: packageMetadata.version,
    assembly: 'cem-ml-transform-graph',
    moduleAssetCount: declaredAssets.length,
    moduleAssetHash: moduleAssetManifest.hash,
    outputFileCount: files.length,
    aggregateSha256: createHash('sha256').update(JSON.stringify(files)).digest('hex'),
    files,
};
await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`Verified CEM Studio graph output: ${declaredAssets.length} declared assets, ${files.length} files.`);

function findModuleAssetManifest(value) {
    if (!value || typeof value !== 'object') return undefined;
    if (value.hashScheme === 'sha256' && Array.isArray(value.assets) && Number.isInteger(value.assetCount)) {
        return value;
    }
    for (const child of Object.values(value)) {
        const found = findModuleAssetManifest(child);
        if (found) return found;
    }
    return undefined;
}

async function filesUnder(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    const files = [];
    for (const entry of entries) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) files.push(...(await filesUnder(path)));
        else files.push(relative(outputRoot, path).replaceAll('\\', '/'));
    }
    return files.sort();
}

function escapeRegex(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}
