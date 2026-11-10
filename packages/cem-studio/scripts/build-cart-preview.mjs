import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { computeStudioDeploymentRevision, deploymentRevisionModule } from './deployment-revision.mjs';

// Host configuration and native graph inputs. The native graph emits assets.
const project = resolve(import.meta.dirname, '..');
const workspace = resolve(project, '../..');
const generated = resolve(project, 'dist/cart-preview-inputs');
const output = resolve(project, 'dist/cart-preview');
const canonical = resolve(project, 'runtime');
const fixture = resolve(workspace, 'packages/cem_ql/fixtures/component-application/shopping-cart');
const wasm = resolve(workspace, 'packages/cem_ql/dist/wasm-selected-application');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const json = async path => JSON.parse(await readFile(path, 'utf8'));
const profile = await json(resolve(fixture, 'profile.json'));
const source = await json(resolve(canonical, 'source.module-map.json'));
const target = await json(resolve(canonical, 'destination.module-map.json'));
await mkdir(generated, { recursive: true });
await rm(output, { recursive: true, force: true });
for (const group of ['imports', 'resources']) {
    for (const entry of Object.values(source[group])) entry.path = relative(generated, resolve(canonical, entry.path)).replaceAll('\\', '/');
}
(source.imports['@epa-wg/cem-studio/deployment-revision'] ?? source.resources['@epa-wg/cem-studio/deployment-revision']).path = './revision.js';
const runtimeAssets = [
    ['cart-factory', 'cem_ql_factory.js', 'text/javascript'],
    ['cart-wasm', 'cem_ql_bg.wasm', 'application/wasm'],
];
for (const [id, file, contentType] of runtimeAssets) {
    source.resources[id] = { path: relative(generated, resolve(wasm, file)).replaceAll('\\', '/'), contentType };
    target.resources[id] = { path: `./applications/shopping-cart/${file}`, contentType };
}
const build = await json(resolve(project, 'src/studio.build.json'));
build.previewApplications = [{
    id: profile.id, endpoint: profile.graphql.endpoint,
    factoryUrl: './applications/shopping-cart/cem_ql_factory.js',
    wasmUrl: './applications/shopping-cart/cem_ql_bg.wasm',
    wasmSha256: hash(await readFile(resolve(wasm, 'cem_ql_bg.wasm'))),
    sourceSha256: hash(await readFile(resolve(fixture, 'profile.json'))),
    resources: await Promise.all([
        ['views/cart.cem', profile.entryTemplate],
        ['graphql/schema.graphql', profile.graphql.schema],
        ['graphql/operations.graphql', profile.graphql.operations],
    ].map(async ([path, file]) => ({ path, sha256: hash(await readFile(resolve(fixture, file))) }))),
    initialRead: profile.graphql.initialRead, actions: profile.graphql.actions,
    bindings: { cartId: profile.graphql.initialRead.variables.id },
}];
build.deploymentProfile = 'cart-preview';
for (const [file, value] of [['source.module-map.json', source], ['destination.module-map.json', target], ['build.json', build]]) {
    await writeFile(resolve(generated, file), JSON.stringify(value, null, 2) + '\n');
}
await writeFile(resolve(generated, 'revision.js'), deploymentRevisionModule(''));
let graph = await readFile(resolve(project, 'studio.cem'), 'utf8');
for (const [from, to] of [
    ['dist/static/', 'dist/cart-preview/'],
    ['runtime/source.module-map.json', 'dist/cart-preview-inputs/source.module-map.json'],
    ['runtime/destination.module-map.json', 'dist/cart-preview-inputs/destination.module-map.json'],
    ['src/studio.build.json', 'dist/cart-preview-inputs/build.json'],
]) graph = graph.replaceAll(from, to);
graph = graph.replace('@source-map="dist/cart-preview-inputs/source.module-map.json"',
    '@source-map="dist/cart-preview-inputs/source.module-map.json" @authored-map="runtime/source.module-map.json"');
const graphPath = resolve(project, 'studio.cart-preview.generated.cem');
await writeFile(graphPath, graph);
const sampleGeneration = spawnSync(process.execPath, [resolve(project, 'scripts/generate-feature-tour.mjs'), '--cart-preview-graph'],
    { cwd: workspace, stdio: 'inherit' });
if (sampleGeneration.error) throw sampleGeneration.error;
if (sampleGeneration.status !== 0) throw Error('Cart preview sample graph failed');
const samplesPath = resolve(generated, 'feature-tour.cem');
function emit(config) {
    const result = spawnSync(resolve(workspace, 'dist/target/cem_ml_cli/debug/cem-ml'), ['transform', '--config', config],
        { cwd: workspace, stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) throw Error('Native cart preview deployment failed');
}
emit(samplesPath);
emit(graphPath);
const revision = await computeStudioDeploymentRevision(output);
await writeFile(resolve(generated, 'revision.js'), deploymentRevisionModule(revision.revision));
emit(graphPath);
console.log(`Built Studio cart preview at ${output} with same-origin ${profile.graphql.endpoint}.`);
