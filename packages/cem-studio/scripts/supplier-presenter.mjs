import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { access, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseCemStudioProjectBundle } from '../src/file-system-provider.js';
import { executeNativeCatalogCommand, nativeCatalogReceipt } from './native-catalog-execution.mjs';
import { workspaceRoot, presenterPrivacy, presenterHash as hash, presenterJson as json,
    presenterText as text, presenterFile as file, presenterCommand as command, withPresenterProject } from './presenter-project.mjs';

export const supplierPresenterRoot = resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/supplier-presenter');
const archivePath = 'packages/cem-studio/generated/feature-tour/examples/supplier-pipeline.cem-studio.json';
const definitionPath = 'examples/projects/supplier-pipeline/example.json';
function scenarios(definition) {
    return [...definition.configurations.flatMap(config => [
        { id: config.id, configuration: config.id },
        ...(config.repair ? [{ id: `${config.id}-repaired`, configuration: config.id, repair: true }] : []),
    ]),
    { id: 'availability-zero-stock', configuration: 'availability', stock: 'zero' },
    { id: 'availability-invalid-stock', configuration: 'availability', stock: 'invalid' },
    { id: 'availability-restored-stock', configuration: 'availability', stock: 'restored' }];
}
export async function readSupplierPresenter(root = supplierPresenterRoot) {
    const manifest = JSON.parse(await readFile(file(root, 'manifest.json'), 'utf8'));
    assert.equal(manifest.schemaVersion, 1);
    assert.match(manifest.sourceRevision, /^[0-9a-f]{40}$/);
    assert.deepEqual(manifest.privacy, presenterPrivacy, 'Supplier presenter privacy policy changed');
    const verified = async descriptor => {
        const bytes = await readFile(file(root, descriptor.file));
        assert.equal(hash(bytes), descriptor.sha256, `Frozen Supplier bytes changed: ${descriptor.file}`);
        return bytes;
    };
    const archive = await verified(manifest.archive), definitionBytes = await verified(manifest.definition);
    assert.deepEqual(archive, await readFile(resolve(workspaceRoot, archivePath)), 'Supplier archive differs');
    assert.deepEqual(definitionBytes, await readFile(resolve(workspaceRoot, definitionPath)), 'Supplier definition differs');
    const bundle = parseCemStudioProjectBundle(archive), definition = JSON.parse(text(definitionBytes));
    assert.equal(bundle.project.id, 'supplier-pipeline-seed');
    assert.equal(definition.id, 'supplier-pipeline'); assert.equal(manifest.projectVersion, definition.version);
    for (const source of definition.sources) assert.deepEqual(Buffer.from(bundle.contents[source.id]),
        await readFile(resolve(workspaceRoot, source.source)), `Supplier authored source differs: ${source.id}`);
    for (const resource of bundle.project.resources) assert.equal(hash(bundle.contents[resource.id]), resource.sha256);
    assert.deepEqual(manifest.checkpoints.map(item => item.scenario), scenarios(definition), 'Supplier checkpoint coverage changed');
    for (const checkpoint of manifest.checkpoints) {
        const config = definition.configurations.find(item => item.id === checkpoint.scenario.configuration);
        assert.deepEqual(checkpoint.argv, command(bundle, bundle.project.entries.find(item => item.id === config.id)));
        assert.deepEqual(checkpoint.artifacts.map(item => item.path), checkpoint.scenario.stock === 'invalid' ? [] : Object.keys(config.expectedGraphOutputs ?? {}), 'Supplier output set changed');
        await verified(checkpoint.output); await verified(checkpoint.diagnostics);
        for (const artifact of checkpoint.artifacts) await verified(artifact);
    }
    return { manifest, definition, bundle };
}

async function replay(bundle, definition, visit) {
    return withPresenterProject(bundle, 'cem-supplier-presenter-', async directory => {
        for (const scenario of scenarios(definition)) {
            const config = definition.configurations.find(item => item.id === scenario.configuration);
            const source = bundle.project.resources.find(item => item.id === config.source);
            if (scenario.repair) {
                const original = text(bundle.contents[source.id]);
                const repaired = original.replace(config.repair.find, config.repair.replace);
                assert.notEqual(repaired, original); await writeFile(file(directory, source.path), repaired);
            }
            if (scenario.stock) {
                const resource = bundle.project.resources.find(item => item.id === 'supplier-stock');
                const original = text(bundle.contents[resource.id]);
                const replacement = scenario.stock === 'zero' ? 'SKU-A,0' : scenario.stock === 'invalid' ? 'SKU-A,many' : 'SKU-A,7';
                await writeFile(file(directory, resource.path), original.replace('SKU-A,7', replacement));
            }
            // A failed run must not inherit previous successful output files.
            for (const path of Object.keys(config.expectedGraphOutputs ?? {})) {
                await rm(file(directory, path), { force: true });
                await rm(file(directory, `${path}.map`), { force: true });
            }
            const invalidStock = scenario.stock === 'invalid';
            const expectedExit = invalidStock ? 1 : scenario.repair ? 0 : config.expectedExit ?? 0;
            const argv = command(bundle, bundle.project.entries.find(item => item.id === config.id));
            const result = await executeNativeCatalogCommand(resolve(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml'), argv, directory, expectedExit);
            const expectedValidation = invalidStock ? { errorCount: 2, warningCount: 0, errorCode: 'cem.transform_runtime.export_representation_unsupported' }
                : scenario.repair ? { errorCount: 0, warningCount: 0 } : config.expectedValidation;
            nativeCatalogReceipt('supplier-pipeline', scenario.id, result, expectedValidation);
            const artifacts = [];
            for (const [path, expected] of Object.entries(config.expectedGraphOutputs ?? {})) {
                if (invalidStock) {
                    assert.equal(await access(file(directory, path)).then(() => true, () => false), false, 'Failed Supplier run published an artifact');
                    continue;
                }
                const bytes = await readFile(file(directory, path));
                if (scenario.stock !== 'zero') assert.deepEqual(bytes, Buffer.from(bundle.contents[expected]));
                else if (path.endsWith('.json')) {
                    const expected = JSON.parse(text(bundle.contents['supplier-expected-json']));
                    Object.assign(expected[0], { quantity: 0, available: false, status: 'out-of-stock' });
                    assert.deepEqual(JSON.parse(text(bytes)), expected);
                }
                if (path.endsWith('.html')) {
                    const map = JSON.parse(await readFile(file(directory, `${path}.map`), 'utf8'));
                    assert.ok(map.outputSpans.length > 0 && JSON.stringify(map).includes('/templates/view.cemt'));
                }
                artifacts.push({ path, bytes });
            }
            if (config.expectedSemanticContract) {
                const report = JSON.parse(result.stdout);
                assert.deepEqual(report.result.sequence.items.map(item => item.value.lexicalValue), JSON.parse(text(bundle.contents[config.expectedSemanticContract])));
            }
            await visit({ scenario, argv, exitCode: result.exitCode }, config,
                artifacts.find(item => item.path === config.entryArtifact)?.bytes ?? Buffer.from(result.stdout),
                Buffer.from(json(result.diagnostics)), artifacts);
            if (scenario.repair) await writeFile(file(directory, source.path), bundle.contents[source.id]);
        }
    });
}

export async function verifySupplierPresenter(root = supplierPresenterRoot) {
    const { manifest, definition, bundle } = await readSupplierPresenter(root);
    await replay(bundle, definition, async (actual, _config, output, diagnostics, artifacts) => {
        const frozen = manifest.checkpoints.find(item => item.scenario.id === actual.scenario.id);
        for (const [key, value] of Object.entries(actual)) assert.deepEqual(value, frozen[key]);
        assert.deepEqual(output, await readFile(file(root, frozen.output.file)), 'Supplier native output drifted');
        assert.deepEqual(diagnostics, await readFile(file(root, frozen.diagnostics.file)), 'Supplier native diagnostics drifted');
        for (const artifact of artifacts) assert.deepEqual(artifact.bytes,
            await readFile(file(root, frozen.artifacts.find(item => item.path === artifact.path).file)), `Supplier ${artifact.path} drifted`);
    });
    return manifest.checkpoints.length;
}
async function freezeSupplierPresenter() {
    const archive = await readFile(resolve(workspaceRoot, archivePath)), definitionBytes = await readFile(resolve(workspaceRoot, definitionPath));
    const bundle = parseCemStudioProjectBundle(archive), definition = JSON.parse(text(definitionBytes));
    const manifest = { schemaVersion: 1, projectVersion: definition.version,
        sourceRevision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: workspaceRoot, encoding: 'utf8' }).trim(), privacy: presenterPrivacy,
        archive: { file: 'supplier-pipeline.cem-studio.json', sha256: hash(archive) },
        definition: { file: 'example.json', sha256: hash(definitionBytes) }, checkpoints: [] };
    const pending = new Map([[manifest.archive.file, archive], [manifest.definition.file, definitionBytes]]);
    const save = (name, bytes) => { pending.set(name, bytes); return { file: name, sha256: hash(bytes) }; };
    await replay(bundle, definition, async (actual, config, output, diagnostics, artifacts) => {
        const id = actual.scenario.id;
        const saved = artifacts.map(item => ({ path: item.path, ...save(`checkpoints/${id}/${item.path}`, item.bytes) }));
        const extension = config.kind === 'validation' || config.expectedSemanticContract ? 'json' : 'cem';
        manifest.checkpoints.push({ ...actual, output: saved.find(item => item.path === config.entryArtifact) ?? save(`checkpoints/${id}.${extension}`, output),
            diagnostics: save(`checkpoints/${id}.diagnostics.json`, diagnostics), artifacts: saved });
    });
    for (const [path, bytes] of pending) { await mkdir(dirname(file(supplierPresenterRoot, path)), { recursive: true }); await writeFile(file(supplierPresenterRoot, path), bytes); }
    await writeFile(file(supplierPresenterRoot, 'manifest.json'), json(manifest));
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    assert.ok(process.argv.slice(2).every(arg => arg === '--write'));
    if (process.argv.includes('--write')) await freezeSupplierPresenter();
    console.log(`Verified ${await verifySupplierPresenter()} frozen Supplier presenter checkpoints.`);
}
