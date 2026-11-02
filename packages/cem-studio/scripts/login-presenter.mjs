import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { parseCemStudioProjectBundle } from '../src/file-system-provider.js';
import { executeNativeCatalogCommand, nativeCatalogReceipt } from './native-catalog-execution.mjs';

import { workspaceRoot, presenterPrivacy, presenterHash as hash, presenterJson as json,
    presenterText as text, presenterFile as file, presenterCommand as command, withPresenterProject } from './presenter-project.mjs';
export { workspaceRoot, presenterPrivacy } from './presenter-project.mjs';
export const presenterRoot = resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/login-presenter');
const archivePath = 'packages/cem-studio/generated/feature-tour/examples/login.cem-studio.json';
const definitionPath = 'examples/projects/login/example.json';
const outputExtension = configuration => configuration.expectedOutput || configuration.id === 'source-colors' ? 'html'
    : configuration.kind === 'validation' || configuration.expectedSemanticContract ? 'json' : 'cem';

/** The freeze contains only checked-in project data; never read a browser session. */
export async function readLoginPresenter(root = presenterRoot) {
    const manifest = JSON.parse(await readFile(file(root, 'manifest.json'), 'utf8'));
    assert.equal(manifest.schemaVersion, 1);
    assert.match(manifest.sourceRevision, /^[0-9a-f]{40}$/, 'Presenter source revision must be explicit');
    assert.deepEqual(manifest.privacy, presenterPrivacy, 'Presenter credential privacy policy changed');
    const verified = async descriptor => {
        const bytes = await readFile(file(root, descriptor.file));
        assert.equal(hash(bytes), descriptor.sha256, `Frozen presenter bytes changed: ${descriptor.file}`);
        return bytes;
    };
    const archive = await verified(manifest.archive), definitionBytes = await verified(manifest.definition);
    const definition = JSON.parse(text(definitionBytes)), bundle = parseCemStudioProjectBundle(archive);
    assert.equal(definition.id, 'login');
    assert.equal(bundle.project.id, 'login-seed');
    assert.equal(manifest.projectVersion, definition.version);
    assert.equal(bundle.project.revision, 1);
    assert.deepEqual(manifest.resources, bundle.project.resources.map(({ id, path, contentType, schema, sha256 }) =>
        ({ id, path, contentType, schema, sha256 })));
    for (const resource of bundle.project.resources) {
        file(root, resource.path);
        assert.equal(hash(bundle.contents[resource.id]), resource.sha256, `Archive resource changed: ${resource.id}`);
    }
    const expectedIds = definition.configurations.flatMap(config => [config.id, ...(config.repair ? [`${config.id}-repaired`] : [])]);
    assert.deepEqual(manifest.checkpoints.map(item => item.id), expectedIds, 'Presenter checkpoint coverage changed');
    for (const checkpoint of manifest.checkpoints) {
        const config = definition.configurations.find(item => item.id === checkpoint.configuration);
        assert.ok(config);
        assert.equal(checkpoint.source, config.source);
        assert.deepEqual(checkpoint.argv, command(bundle, bundle.project.entries.find(item => item.id === config.id)));
        await verified(checkpoint.output);
        await verified(checkpoint.diagnostics);
    }
    assert.deepEqual(archive, await readFile(resolve(workspaceRoot, archivePath)), 'Current Login archive differs from presenter freeze');
    assert.deepEqual(definitionBytes, await readFile(resolve(workspaceRoot, definitionPath)), 'Current Login definition differs from presenter freeze');
    for (const source of definition.sources) assert.deepEqual(Buffer.from(bundle.contents[source.id]),
        await readFile(resolve(workspaceRoot, source.source)), `Authored Login source differs: ${source.id}`);
    return { manifest, bundle, definition };
}

/** Replay the shared native CLI on ordinary project files in an isolated directory. */
async function replay(bundle, definition, visit) {
    return withPresenterProject(bundle, 'cem-login-presenter-', async directory => {
        for (const config of definition.configurations) {
            const argv = command(bundle, bundle.project.entries.find(item => item.id === config.id));
            const run = async repaired => {
                const id = repaired ? `${config.id}-repaired` : config.id;
                const result = await executeNativeCatalogCommand(resolve(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml'),
                    argv, directory, repaired ? 0 : config.expectedExit ?? 0);
                const expectation = repaired ? { errorCount: 0, warningCount: 3, warningCode: 'cem.ast.unresolved_reference' }
                    : config.expectedValidation;
                nativeCatalogReceipt('login', repaired ? `${config.id}:repair` : config.id, result, expectation);
                const output = config.expectedGraphOutput ? await readFile(file(directory, config.expectedGraphOutput)) : Buffer.from(result.stdout);
                if (config.expectedOutput) assert.deepEqual(output, Buffer.from(bundle.contents[config.expectedOutput]));
                for (const expected of config.expectedText ?? []) assert.ok(text(output).includes(expected));
                if (config.expectedSemanticContract) {
                    const report = JSON.parse(text(output));
                    const values = report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue)
                        : report.result.items.map(item => item.value);
                    assert.deepEqual(values, JSON.parse(text(bundle.contents[config.expectedSemanticContract])));
                }
                await visit({ id, configuration: config.id, source: config.source, argv, exitCode: result.exitCode },
                    config, output, Buffer.from(json(result.diagnostics)));
            };
            await run(false);
            if (config.repair) {
                const resource = bundle.project.resources.find(item => item.id === config.source);
                const original = text(bundle.contents[resource.id]);
                const repaired = original.replace(config.repair.find, config.repair.replace);
                assert.notEqual(original, repaired);
                await writeFile(file(directory, resource.path), repaired);
                await run(true);
                await writeFile(file(directory, resource.path), bundle.contents[resource.id]);
            }
        }
    });
}

export async function verifyLoginPresenter(root = presenterRoot) {
    const { manifest, bundle, definition } = await readLoginPresenter(root);
    await replay(bundle, definition, async (actual, _config, output, diagnostics) => {
        const frozen = manifest.checkpoints.find(item => item.id === actual.id);
        for (const [key, value] of Object.entries(actual)) assert.deepEqual(value, frozen[key], `${actual.id}: ${key} drifted`);
        assert.deepEqual(output, await readFile(file(root, frozen.output.file)), `${actual.id}: native output drifted`);
        assert.deepEqual(diagnostics, await readFile(file(root, frozen.diagnostics.file)), `${actual.id}: native diagnostics drifted`);
    });
    return manifest.checkpoints.length;
}

async function freezeLoginPresenter() {
    const archive = await readFile(resolve(workspaceRoot, archivePath));
    const definitionBytes = await readFile(resolve(workspaceRoot, definitionPath));
    const bundle = parseCemStudioProjectBundle(archive), definition = JSON.parse(text(definitionBytes));
    const manifest = { schemaVersion: 1, projectVersion: definition.version,
        sourceRevision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: workspaceRoot, encoding: 'utf8' }).trim(),
        privacy: presenterPrivacy, archive: { file: 'login.cem-studio.json', sha256: hash(archive) },
        definition: { file: 'example.json', sha256: hash(definitionBytes) },
        resources: bundle.project.resources.map(({ id, path, contentType, schema, sha256 }) => ({ id, path, contentType, schema, sha256 })),
        checkpoints: [] };
    const pending = new Map([[manifest.archive.file, archive], [manifest.definition.file, definitionBytes]]);
    await replay(bundle, definition, async (actual, config, output, diagnostics) => {
        const outputFile = `checkpoints/${actual.id}.${outputExtension(config)}`;
        const diagnosticsFile = `checkpoints/${actual.id}.diagnostics.json`;
        pending.set(outputFile, output); pending.set(diagnosticsFile, diagnostics);
        manifest.checkpoints.push({ ...actual, output: { file: outputFile, sha256: hash(output) },
            diagnostics: { file: diagnosticsFile, sha256: hash(diagnostics) } });
    });
    // Do not replace a prior freeze until every native checkpoint has succeeded.
    for (const [name, bytes] of pending) {
        await mkdir(dirname(file(presenterRoot, name)), { recursive: true });
        await writeFile(file(presenterRoot, name), bytes);
    }
    await writeFile(file(presenterRoot, 'manifest.json'), json(manifest));
    await readLoginPresenter();
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    assert.ok(process.argv.slice(2).every(arg => arg === '--write'), 'Use only --write to explicitly replace the presenter freeze');
    if (process.argv.includes('--write')) await freezeLoginPresenter();
    console.log(`Verified ${await verifyLoginPresenter()} frozen Login presenter checkpoints; browser storage was not read.`);
}
