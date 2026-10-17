import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import ts from 'typescript';

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

assert.ok(!declaredAssets.some(asset => /\/(?:browser|browser-recorded-output|browser-html-base|browser-stylesheets|browser-svg-computed)\.js$/.test(asset.target)),
    'Studio must deploy the command-service surface without live DOM/paint observation hosts');
const declaredPaths = new Set(declaredAssets.map(asset => resolve(outputRoot, asset.target)));
// This metadata is an explicit graph export in studio.cem, outside its module map.
declaredPaths.add(resolve(outputRoot, 'assets/@epa-wg/cem-theme/package.json'));
for (const asset of declaredAssets.filter(asset => asset.contentType === 'text/javascript')) {
    const filename = resolve(outputRoot, asset.target);
    const text = await readFile(filename, 'utf8');
    for (const { fileName } of ts.preProcessFile(text, true, true).importedFiles) {
        if (!fileName.startsWith('.')) continue;
        assert.ok(declaredPaths.has(resolve(dirname(filename), fileName)),
            `${asset.identity} imports undeclared deployed module ${fileName}`);
    }
}

const shellModule = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/shell.js'), 'utf8');
const shellView = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/shell.xhtml'), 'utf8');
assert.match(shellModule, /mountCemView/);
assert.doesNotMatch(shellModule, /shellMarkup|innerHTML|textContent|querySelector|toggleAttribute|classList/);
assert.match(shellView, /<template id="cem-studio-shell" type="text\/cem-ml">/);
assert.match(shellView, /theme-command=theme/);
assert.match(shellView, /host-command=install/);
assert.match(shellView, /host-command=update/);
assert.match(shellView, /\{slot\}/);
assert.doesNotMatch(shellView, /<script|\son[a-z]+=|javascript:/i);
const workbenchModule = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/workbench.js'), 'utf8');
const workbenchControls = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/workbench-controls.xhtml'), 'utf8');
assert.match(workbenchModule, /mountCemView/);
assert.doesNotMatch(workbenchModule, /editor\.focus\(|editor\.setSelectionRange\(|function statusLabel|function statusTone/);
assert.match(workbenchControls, /<template id="cem-studio-workbench-controls" type="text\/cem-ml">/);
for (const command of ['edit', 'save', 'reload', 'run', 'parse', 'inspect']) {
    assert.match(workbenchControls, new RegExp(`host-command=${command}(?:\\s|$)`));
}
assert.match(workbenchControls, /selection-request=/);
assert.doesNotMatch(workbenchControls, /@variant=(?:quiet|secondary)|<script|\son[a-z]+=|javascript:/i);

const navigationView = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/workbench-navigation.xhtml'), 'utf8');
for (const command of ['select-workbench', 'open-entity', 'search-projects', 'open-search', 'navigation-cancel']) {
    assert.ok(navigationView.includes(`host-command=${command}`));
}
assert.match(navigationView, /host-command-event=cem-tree-activate/);
assert.doesNotMatch(navigationView, /<script|\son[a-z]+=|javascript:/i);

const commandView = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/workbench-command.xhtml'), 'utf8');
assert.match(commandView, /<template id="cem-studio-workbench-command" type="text\/cem-ml">/);
for (const command of ['edit', 'copy', 'reset', 'target-mode', 'target-existing', 'target-name', 'target-parent',
    'apply', 'apply-run', 'confirm-replacement', 'use-new', 'cancel-replacement', 'dismiss-replacement']) {
    assert.match(commandView, new RegExp(`host-command=${command}(?:\\s|$)`));
}
assert.match(commandView, /host-command-event=cem-close/);
assert.doesNotMatch(commandView, /@variant=(?:quiet|secondary)|<script|\son[a-z]+=|javascript:/i);
assert.doesNotMatch(workbenchModule, /commandApplicationMarkup|renderCommandApplication|commandAlertLabel|commandChangesMarkup|commandEditor\.value|commandTargetName\.value/);

const resultsView = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/workbench-results.xhtml'), 'utf8');
const layoutView = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/workbench-layout.xhtml'), 'utf8');
assert.match(layoutView, /<template id="cem-studio-workbench-layout" type="text\/cem-ml">/);
assert.match(resultsView, /<template id="cem-studio-workbench-results" type="text\/cem-ml">/);
for (const command of ['copy-output', 'download-output', 'diagnostic', 'provenance']) {
    assert.match(resultsView, new RegExp(`host-command=${command}(?:\\s|$)`));
}
assert.match(resultsView, /cem-media-preview/);
assert.doesNotMatch(resultsView, /@variant=(?:quiet|secondary)|<script|\son[a-z]+=|javascript:/i);
assert.doesNotMatch(workbenchModule, /innerHTML|textContent|addEventListener|removeEventListener|workbenchMarkup|renderWorkbench|Markup\(/);

const html = await readFile(resolve(outputRoot, 'index.html'), 'utf8');
for (const [specifier, entry] of Object.entries(destinationMap.imports)) {
    assert.match(html, new RegExp(`${escapeRegex(specifier)}[^<]+${escapeRegex(entry.path)}`, 's'));
}
assert.doesNotMatch(html, /node_modules/);
assert.match(html, /import '@epa-wg\/cem-studio\/main'/);
assert.match(html, /data-cem-studio-root/);
assert.match(html, /<base href="\.\/" data-cem-studio-scope\s*\/?>/);
assert.doesNotMatch(html, /preview\.css/);

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
    securityHeaders,
    previewModule,
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
    readJson(resolve(outputRoot, 'security-headers.json')),
    readFile(resolve(outputRoot, 'assets/@epa-wg/cem-studio/preview.js'), 'utf8'),
]);
assert.equal(manifest.start_url, './');
assert.equal(manifest.icons[0].src, './icon.svg');
assert.equal(buildMetadata.package, packageMetadata.name);
assert.equal(buildMetadata.commonVersion, packageMetadata.version);
assert.equal(buildMetadata.buildIdentity, 'cem-studio-static-v1');
assert.equal(inventory.commonVersion, packageMetadata.version);
assert.equal(inventory.schemaVersion, 2);
assert.deepEqual(
    inventory.groups.map(({ id }) => id),
    ['shell', 'runtime', 'samples'],
);
assert.deepEqual(
    inventory.groups.find(({ id }) => id === 'samples'),
    {
        id: 'samples',
        strategy: 'cache-first',
        catalog: './samples/index.json',
    },
);
assert.equal(runtimeMetadata.commonVersion, packageMetadata.version);
assert.equal(themeMetadata.version, packageMetadata.dependencies['@epa-wg/cem-theme']);
assert.equal(sampleIndex.commonVersion, packageMetadata.version);
assert.equal(sampleIndex.projects.length, 2);
assert.equal(sampleIndex.projects[0].id, 'schema-examples');
assert.equal(sampleIndex.projects[1].id, 'login');
assert.equal(sampleIndex.projects[1].archive, './examples/login.cem-studio.json');
assert.ok(sampleIndex.cacheUrls.includes(sampleIndex.projects[1].archive));
assert.equal(featureTourCatalog.commonVersion, packageMetadata.version);
// The tour includes one passing example per canonical runtime schema package.
assert.ok(runtimeMetadata.schemaPackages.manifestCount > 0);
assert.equal(featureTourCatalog.packageCount, runtimeMetadata.schemaPackages.manifestCount);
assert.equal(featureTourCatalog.exampleCount, featureTourCatalog.packageCount);
assert.equal(featureTourCatalog.examples.length, featureTourCatalog.exampleCount);
assert.equal(featureTourCatalog.workbenchCount, 5);
assert.equal(featureTourCatalog.workbenches.length, 5);
assert.equal(featureTourProject.id, featureTourCatalog.seed.id);
assert.equal(featureTourProject.resources.length, featureTourCatalog.projectResourceCount);
assert.equal(
    featureTourCatalog.projectResourceCount,
    featureTourCatalog.exampleCount * 2 +
        featureTourCatalog.dependencyCount +
        featureTourCatalog.workbenchResourceCount,
);
assert.equal(sampleIndex.cacheUrls.length, featureTourCatalog.cacheUrlCount + 3);
assert.match(serviceWorker, /event\.data\?\.type === 'cem-studio-activate-update'/);
assert.match(serviceWorker, /event\.waitUntil\(self\.skipWaiting\(\)\)/);
const contentSecurityPolicy = securityHeaders.headers['Content-Security-Policy'];
const scriptPolicy = contentSecurityPolicy.split(';').map(part => part.trim()).find(part => part.startsWith('script-src '));
assert.ok(scriptPolicy);
assert.doesNotMatch(scriptPolicy, /'unsafe-inline'|'unsafe-eval'/);
assert.match(contentSecurityPolicy, /(?:^|;)\s*style-src 'self';/);
assert.match(contentSecurityPolicy, /(?:^|;)\s*style-src-elem 'self' 'unsafe-inline';/);
assert.match(contentSecurityPolicy, /(?:^|;)\s*style-src-attr 'none';/);
assert.match(contentSecurityPolicy, /'wasm-unsafe-eval'/);
assert.match(contentSecurityPolicy, /default-src 'none'/);
assert.match(contentSecurityPolicy, /frame-ancestors 'none'/);
const inlineScriptHashes = [...html.matchAll(/<script(?![^>]*\bsrc=)[^>]*>([\s\S]*?)<\/script>/g)].map(
    (match) => `'sha256-${createHash('sha256').update(match[1]).digest('base64')}'`,
);
assert.equal(inlineScriptHashes.length, 2);
for (const hash of inlineScriptHashes) assert.ok(contentSecurityPolicy.includes(hash), `app CSP is missing ${hash}`);
assert.doesNotMatch(previewModule, /createElement|innerHTML|textContent|srcdoc|mountCemStudioPreview/);
const componentBundle = await readFile(resolve(outputRoot, 'assets/@epa-wg/cem-components/dist/components.xhtml'), 'utf8');
const mediaPreview = componentBundle.match(/<cem-element\b[^>]*\btag="cem-media-preview"[\s\S]*?<\/cem-element>/)?.[0];
assert.ok(mediaPreview, 'canonical media-preview declaration is missing');
assert.match(mediaPreview, /default-src 'none'/);
assert.match(mediaPreview, /@sandbox=""/);
assert.doesNotMatch(mediaPreview, /allow-scripts|allow-same-origin/);
assert.equal(packageMetadata.exports['.'].import, './dist/static/assets/@epa-wg/cem-studio/bootstrap.js');
assert.equal(packageMetadata.exports['./shell'].import, './dist/static/assets/@epa-wg/cem-studio/shell.js');
assert.equal(
    packageMetadata.exports['./file-system-provider'].import,
    './dist/static/assets/@epa-wg/cem-studio/file-system-provider.js',
);
assert.equal(
    packageMetadata.exports['./feature-tour'].import,
    './dist/static/assets/@epa-wg/cem-studio/feature-tour.js',
);
assert.equal(packageMetadata.exports['./workbench'].import, './dist/static/assets/@epa-wg/cem-studio/workbench.js');
assert.equal(packageMetadata.exports['./preview'].import, './dist/static/assets/@epa-wg/cem-studio/preview.js');

const files = await filesUnder(outputRoot);
for (const required of [
    'index.html',
    'manifest.webmanifest',
    'build.json',
    'cache-inventory.json',
    'module-map.json',
    'security-headers.json',
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
