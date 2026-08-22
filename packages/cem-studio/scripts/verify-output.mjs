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

assert.equal(sourceMap.$schema, 'https://cem.dev/ns/data/module-map/3');
assert.equal(destinationMap.$schema, sourceMap.$schema);
assert.deepEqual(Object.keys(destinationMap.imports).sort(), Object.keys(sourceMap.imports).sort());
assert.deepEqual(Object.keys(destinationMap.resources).sort(), Object.keys(sourceMap.resources).sort());
assert.equal(buildReport.summary.errorCount, 0);
assert.equal(buildReport.summary.fatalCount, 0);
assert.match(buildScript, /'packages\/cem-studio\/generated\/feature-tour\/feature-tour\.cem'/);
assert.match(buildScript, /'packages\/cem-studio\/studio\.cem'/);
assert.match(buildScript, /'--config',\s*config/);
assert.doesNotMatch(buildScript, /copyFile|\bcp\b|vite|rollup|webpack/);

const declaredAssets = [
    ...Object.entries(sourceMap.imports).map(([identity, source]) => ({
        identity,
        source: source.path,
        target: destinationMap.imports[identity].path,
        contentType: source.contentType,
        moduleImports: source.moduleImports ?? {},
    })),
    ...Object.entries(sourceMap.resources).map(([identity, resource]) => ({
        identity,
        source: resource.path,
        target: destinationMap.resources[identity].path,
        contentType: resource.contentType,
        moduleImports: resource.moduleImports ?? {},
    })),
];
const assetsByIdentity = new Map(declaredAssets.map((asset) => [asset.identity, asset]));
const moduleAssetManifest = findModuleAssetManifest(buildReport);
assert.ok(moduleAssetManifest, 'build report is missing the CEM-ML module-asset manifest');
assert.equal(moduleAssetManifest.contractVersion, 2);
assert.equal(moduleAssetManifest.assetCount, declaredAssets.length);
assert.equal(moduleAssetManifest.hashScheme, 'sha256');
assert.equal(moduleAssetManifest.hash.length, 64);
const manifestAssets = new Map(moduleAssetManifest.assets.map((asset) => [asset.specifier, asset]));

for (const asset of declaredAssets) {
    assert.ok(asset.target.startsWith('./'), `${asset.identity} target is not app-relative`);
    const sourceBytes = await readFile(resolve(runtimeRoot, asset.source));
    const outputBytes = await readFile(resolve(outputRoot, asset.target.slice(2)));
    const sourceEntry = sourceMap.imports[asset.identity] ?? sourceMap.resources[asset.identity];
    const destinationEntry = destinationMap.imports[asset.identity] ?? destinationMap.resources[asset.identity];
    assert.equal(destinationEntry.contentType, sourceEntry.contentType);
    assert.deepEqual(destinationEntry.moduleImports ?? {}, asset.moduleImports);

    const manifestAsset = manifestAssets.get(asset.identity);
    assert.ok(manifestAsset, `${asset.identity} is missing from the module-asset manifest`);
    assert.equal(manifestAsset.sourceByteLength, sourceBytes.byteLength);
    assert.equal(manifestAsset.sourceSha256, sha256(sourceBytes));
    assert.equal(manifestAsset.byteLength, outputBytes.byteLength);
    assert.equal(manifestAsset.sha256, sha256(outputBytes));

    if (Object.keys(asset.moduleImports).length === 0) {
        assert.deepEqual(outputBytes, sourceBytes, `${asset.identity} changed without a declared module edge`);
        continue;
    }

    assert.notDeepEqual(
        outputBytes,
        sourceBytes,
        `${asset.identity} declared module edges but remained byte-identical`,
    );
    const outputText = outputBytes.toString('utf8');
    for (const [specifier, targetIdentity] of Object.entries(asset.moduleImports)) {
        const targetAsset = assetsByIdentity.get(targetIdentity);
        assert.ok(targetAsset, `${asset.identity} points to undeclared asset ${targetIdentity}`);
        const rewritten = relativeModuleTarget(asset.target, targetAsset.target);
        assert.match(
            outputText,
            new RegExp(`['"]${escapeRegex(rewritten)}['"]`),
            `${asset.identity} did not rewrite ${specifier} to ${rewritten}`,
        );
        assert.doesNotMatch(
            outputText,
            moduleSpecifierRegex(specifier),
            `${asset.identity} retained bare specifier ${specifier}`,
        );
    }
}

const html = await readFile(resolve(outputRoot, 'index.html'), 'utf8');
for (const [specifier, entry] of Object.entries(destinationMap.imports)) {
    assert.match(html, new RegExp(`${escapeRegex(specifier)}[^<]+${escapeRegex(entry.path)}`, 's'));
}
assert.doesNotMatch(html, /node_modules/);
assert.match(html, /import '@epa-wg\/cem-studio\/main'/);
assert.match(html, /data-cem-studio-root/);
assert.match(html, /<base href="\.\/" data-cem-studio-scope\s*\/?>/);

const [
    manifest,
    buildMetadata,
    inventory,
    runtimeMetadata,
    themeMetadata,
    sampleIndex,
    featureTourCatalog,
    featureTourProject,
    serviceWorker,
] = await Promise.all([
    readJson(resolve(outputRoot, 'manifest.webmanifest')),
    readJson(resolve(outputRoot, 'build.json')),
    readJson(resolve(outputRoot, 'cache-inventory.json')),
    readJson(resolve(outputRoot, 'assets/@epa-wg/cem-ml/dist/cem-ml-runtime.json')),
    readJson(resolve(outputRoot, 'assets/@epa-wg/cem-theme/package.json')),
    readJson(resolve(outputRoot, 'samples/index.json')),
    readJson(resolve(outputRoot, 'samples/feature-tour/catalog.json')),
    readJson(resolve(outputRoot, 'samples/feature-tour/feature-tour.project.json')),
    readFile(resolve(outputRoot, 'service-worker.js'), 'utf8'),
]);
assert.equal(manifest.start_url, './');
assert.equal(manifest.icons[0].src, './icon.svg');
assert.equal(buildMetadata.package, packageMetadata.name);
assert.equal(buildMetadata.commonVersion, packageMetadata.version);
assert.equal(inventory.commonVersion, packageMetadata.version);
assert.equal(inventory.schemaVersion, 2);
assert.deepEqual(inventory.groups.map(({ id }) => id), ['shell', 'runtime', 'samples']);
assert.deepEqual(inventory.groups.find(({ id }) => id === 'samples'), {
    id: 'samples',
    strategy: 'cache-first',
    catalog: './samples/index.json',
});
assert.equal(runtimeMetadata.commonVersion, packageMetadata.version);
assert.equal(themeMetadata.version, packageMetadata.dependencies['@epa-wg/cem-theme']);
assert.equal(sampleIndex.commonVersion, packageMetadata.version);
assert.equal(sampleIndex.samples.length, 1);
assert.equal(sampleIndex.samples[0].id, 'cem-ml-feature-tour-seed');
assert.equal(featureTourCatalog.commonVersion, packageMetadata.version);
assert.equal(featureTourCatalog.packageCount, 30);
assert.equal(featureTourCatalog.exampleCount, 30);
assert.equal(featureTourCatalog.examples.length, 30);
assert.equal(featureTourProject.id, featureTourCatalog.seed.id);
assert.equal(featureTourProject.resources.length, featureTourCatalog.projectResourceCount);
assert.equal(
    featureTourCatalog.projectResourceCount,
    featureTourCatalog.exampleCount * 2 + featureTourCatalog.dependencyCount,
);
assert.equal(sampleIndex.cacheUrls.length, featureTourCatalog.cacheUrlCount);
assert.match(serviceWorker, /event\.data\?\.type === 'cem-studio-activate-update'/);
assert.match(serviceWorker, /event\.waitUntil\(self\.skipWaiting\(\)\)/);
assert.equal(packageMetadata.exports['.'].import, './dist/static/assets/@epa-wg/cem-studio/bootstrap.js');
assert.equal(packageMetadata.exports['./shell'].import, './dist/static/assets/@epa-wg/cem-studio/shell.js');
assert.equal(
    packageMetadata.exports['./feature-tour'].import,
    './dist/static/assets/@epa-wg/cem-studio/feature-tour.js',
);
assert.equal(
    packageMetadata.exports['./workbench'].import,
    './dist/static/assets/@epa-wg/cem-studio/workbench.js',
);

const files = await filesUnder(outputRoot);
for (const required of [
    'index.html',
    'manifest.webmanifest',
    'build.json',
    'cache-inventory.json',
    'module-map.json',
    'icon.svg',
    'service-worker.js',
    'samples/index.json',
    ...sampleIndex.cacheUrls.map((path) => `samples/${path.slice(2)}`),
    ...declaredAssets.map(({ target }) => target.slice(2)),
]) {
    assert.ok(files.includes(required), `static output is missing ${required}`);
}

const report = {
    schemaVersion: 1,
    project: packageMetadata.name,
    commonVersion: packageMetadata.version,
    assembly: 'cem-ml-transform-graph',
    moduleAssetContractVersion: moduleAssetManifest.contractVersion,
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

function relativeModuleTarget(fromTarget, toTarget) {
    const path = relative(dirname(fromTarget.slice(2)), toTarget.slice(2)).replaceAll('\\', '/');
    return path.startsWith('.') ? path : `./${path}`;
}

function moduleSpecifierRegex(specifier) {
    return new RegExp(`\\b(?:from\\s*|import\\s*(?:\\(\\s*)?)['"]${escapeRegex(specifier)}['"]`);
}

function sha256(bytes) {
    return createHash('sha256').update(bytes).digest('hex');
}

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}
