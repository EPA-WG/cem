import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { createRequire } from 'node:module';
import ts from 'typescript';
import { ownedWasmFactory } from '../../../tools/scripts/cem-ql-wasm-factory.mjs';

// Build configuration only. CEM's native graph emits every deployment byte;
// this script never copies a deployment tree or interprets application sources.
const project = resolve(import.meta.dirname, '..'), workspace = resolve(project, '../..');
const generated = resolve(project, 'generated/schema-assets');
const output = resolve(project, 'dist/schema-assets');
const pack = resolve(workspace, 'packages/cem-ml-npm/dist-schema-assets');
const native = resolve(workspace, 'packages/cem-ml-npm/dist-schema-assets-wasm/wasm/browser');
const canonical = resolve(project, 'runtime');
const json = async path => JSON.parse(await readFile(path, 'utf8'));
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const source = await json(resolve(canonical, 'source.module-map.json'));
const target = await json(resolve(canonical, 'destination.module-map.json'));
const baseline = await treeHashes(resolve(project, 'dist/static'));
if (!baseline.length) throw Error('Build the ordinary Studio deployment first');
const pin = await json(resolve(pack, 'bootstrap.json'));
await mkdir(generated, { recursive: true });
await rm(output, { recursive: true, force: true });

const glue = await readFile(resolve(native, 'cem_ml.js'), 'utf8');
if (!glue.includes('schemaAssetOwnerActivateV1')) throw Error('Build the optional native WASM profile first');
const probe = createRequire(import.meta.url)(resolve(native, '../node/cem_ml.js'));
const nativePin = JSON.parse(probe.schemaAssetPackBootstrapV1());
const embeddingProfile = JSON.parse(probe.schemaAssetEmbeddedSourcesV1());
if (embeddingProfile.externalBodies !== true || !embeddingProfile.externalPackages.includes('csv')) throw Error('Build the external-source optional WASM profile first');
if (Object.keys(pin).some(key => nativePin[key] !== pin[key])
    || hash(await readFile(resolve(native, 'cem_ml_bg.wasm'))) !== hash(await readFile(resolve(native, '../node/cem_ml_bg.wasm')))) {
    throw Error('Optional runtime and emitted pack are not the same pinned deployment');
}
const rendererGlue = rendererBindings(glue);
await writeFile(resolve(generated, 'cem_ql.js'), rendererGlue);
await writeFile(resolve(generated, 'cem_ql_factory.js'), ownedWasmFactory(rendererGlue, 'cem_ql', 'CemQl'));
for (const group of ['imports', 'resources']) {
    for (const entry of Object.values(source[group])) {
        entry.path = relative(generated, resolve(canonical, entry.path)).replaceAll('\\', '/');
    }
}
const entry = id => source.imports[id] ?? source.resources[id];
for (const [id, file] of [
    ['@epa-wg/cem-ml/wasm', 'cem_ml.js'], ['@epa-wg/cem-ml/wasm-binary', 'cem_ml_bg.wasm'],
    ['@epa-wg/cem-ml/wasm/factory', 'cem_ml_factory.js'],
    ['@epa-wg/cem-elements/cem-ql-wasm', 'cem_ml_bg.wasm'],
]) entry(id).path = relative(generated, resolve(native, file)).replaceAll('\\', '/');
entry('@epa-wg/cem-elements/cem-ql').path = './cem_ql.js';
entry('@epa-wg/cem-elements/cem-ql-factory').path = './cem_ql_factory.js';
const assetHashes = [];
for (const group of ['imports', 'resources']) for (const [id, asset] of Object.entries(source[group])) {
    assetHashes.push([id, hash(await readFile(resolve(generated, asset.path)))]);
}
// The ordinary native output receipt also covers HTML, shell metadata, icons
// and sample/fixture outputs that are outside the module map. Opt-in assets
// replace selected modules only; hash both complete sets to avoid stale caches.
const deploymentDigest = hash(JSON.stringify([source, target, pin.indexDigest, assetHashes, baseline,
    hash(await readFile(resolve(project, 'studio.cem'))), hash(await readFile(import.meta.filename))]));
const build = await json(resolve(project, 'src/studio.build.json'));
Object.assign(build, { buildIdentity: 'cem-studio-schema-assets-v1', deploymentProfile: 'schema-assets', deploymentDigest,
    schemaAssetBase: './schema-assets/', schemaAssetIndexDigest: pin.indexDigest, embeddedBodies: 'bootstrap-only', embeddingProfile,
    packagePublication: 'explicit-host-admission', automaticRecovery: false,
    schemaPackageHostConfiguration: { rootQueueSize: 1024, schemaPackageReplacementGrants: [{ packageId: 'csv',
        expectedOrigin: { kind: 'builtin' }, replacementManifestUri: 'schema-packages/csv/v1/package.cem' }] } });
const inventory = await json(resolve(project, 'src/cache-inventory.json'));
Object.assign(inventory, { deploymentProfile: 'schema-assets', deploymentDigest });
inventory.groups.push({ id: 'schema-assets', strategy: 'cache-first', inventory: './schema-assets/integrity.json' });
for (const [name, value] of [['source.module-map.json', source], ['destination.module-map.json', target], ['build.json', build], ['cache-inventory.json', inventory]]) {
    await writeFile(resolve(generated, name), `${JSON.stringify(value, null, 2)}\n`);
}

// The canonical graph remains the source template. Only explicit build input /
// output literals change; all HTML/module/byte processing remains native.
let graph = await readFile(resolve(project, 'studio.cem'), 'utf8');
for (const [from, to] of [
    ['dist/static/', 'dist/schema-assets/'],
    ['runtime/source.module-map.json', 'generated/schema-assets/source.module-map.json'],
    ['runtime/destination.module-map.json', 'generated/schema-assets/destination.module-map.json'],
    ['src/studio.build.json', 'generated/schema-assets/build.json'],
    ['src/cache-inventory.json', 'generated/schema-assets/cache-inventory.json'],
]) {
    if (!graph.includes(from)) throw Error(`Canonical Studio graph no longer declares ${from}`);
    graph = graph.replaceAll(from, to);
}
graph = graph.replace('@source-map="generated/schema-assets/source.module-map.json"',
    '@source-map="generated/schema-assets/source.module-map.json" @authored-map="runtime/source.module-map.json"');
graph = graph.trimEnd();
if (!graph.endsWith('}')) throw Error('Canonical Studio graph must contain one run declaration');
graph = graph.slice(0, -1) + `\n  {import @id=schema-asset-pack @src="../cem-ml-npm/dist-schema-assets/**/*"\n      @root="../cem-ml-npm/dist-schema-assets" @opaque=true |\n    {export @id=schema-asset-delivery @out="dist/schema-assets/schema-assets/{path}"}\n  }\n}\n`;
const graphPath = resolve(project, 'studio.schema-assets.generated.cem');
const samplesPath = resolve(project, 'generated/feature-tour/feature-tour.schema-assets.cem');
await writeFile(graphPath, graph);
await writeFile(samplesPath, (await readFile(resolve(project, 'generated/feature-tour/feature-tour.cem'), 'utf8'))
    .replaceAll('../../dist/static/', '../../dist/schema-assets/'));
for (const [config, report] of [[samplesPath, 'schema-assets-samples'], [graphPath, 'schema-assets-build']]) {
    const run = spawnSync(resolve(workspace, 'dist/target/cem_ml_cli/debug/cem-ml'),
        ['transform', '--config', config, '--report-json', resolve(workspace, `dist/reports/cem-studio/${report}.json`)],
        { cwd: workspace, stdio: 'inherit' });
    if (run.error) throw run.error;
    if (run.status !== 0) throw Error(`Native optional Studio graph failed: ${run.status}`);
}
const after = await treeHashes(resolve(project, 'dist/static'));
if (JSON.stringify(after) !== JSON.stringify(baseline)) throw Error('Optional profile modified the default deployment');
console.log(`Built opt-in Studio at ${output}; default deployment unchanged; index ${pin.indexDigest}.`);

async function treeHashes(root, prefix = '') {
    const files = [];
    for (const entry of (await readdir(resolve(root, prefix), { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
        const path = prefix ? `${prefix}/${entry.name}` : entry.name;
        if (entry.isDirectory()) files.push(...await treeHashes(root, path));
        else files.push([path, hash(await readFile(resolve(root, path)))]);
    }
    return files;
}

function rendererBindings(source) {
    const ast = ts.createSourceFile('cem_ml.js', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    const edits = [];
    const visit = node => {
        if (ts.isNewExpression(node) && node.expression.getText(ast) === 'URL'
            && node.arguments?.length === 2 && ts.isStringLiteral(node.arguments[0])
            && node.arguments[0].text === 'cem_ml_bg.wasm' && node.arguments[1].getText(ast) === 'import.meta.url') {
            edits.push([node.arguments[0].getStart(ast), node.arguments[0].end]);
        }
        ts.forEachChild(node, visit);
    };
    visit(ast);
    if (edits.length !== 1) throw Error('Unexpected native binding binary URL shape');
    const [start, end] = edits[0];
    return source.slice(0, start) + "'cem_ql_bg.wasm'" + source.slice(end);
}
