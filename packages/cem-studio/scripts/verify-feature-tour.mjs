import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { parseCemStudioProjectBundle } from '../src/file-system-provider.js';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const generatedRoot = resolve(projectRoot, 'generated/feature-tour');
const outputRoot = resolve(projectRoot, 'dist/static/samples/feature-tour');
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/feature-tour.json');
const cli = resolve(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');

run(process.execPath, ['scripts/generate-feature-tour.mjs', '--check'], projectRoot);

const catalog = await readJson(resolve(generatedRoot, 'catalog.json'));
const project = await readJson(resolve(generatedRoot, 'feature-tour.project.json'));
const sampleIndex = await readJson(resolve(projectRoot, 'dist/static/samples/index.json'));
const featureTourBuild = await readJson(resolve(workspaceRoot, 'dist/reports/cem-studio/feature-tour-build.json'));

assert.equal(catalog.selection, 'first-manifest-declared-pass-example-per-registered-package');
assert.equal(catalog.packageCount, 32);
assert.ok(catalog.examples.some(({ packageId }) => packageId === 'cem-data-island'));
assert.equal(catalog.exampleCount, catalog.packageCount);
assert.equal(catalog.workbenchCount, 5);
assert.deepEqual(catalog.workbenches.map(({ kind }) => kind), [
    'conversion',
    'query',
    'transformation',
    'trace',
    'transformation-graph',
]);
assert.equal(catalog.capability.operation, 'validate');
assert.equal(catalog.capability.availability, 'available');
assert.equal(project.id, catalog.seed.id);
assert.equal(project.entries.filter(({ kind }) => kind === 'validation').length, catalog.exampleCount);
assert.equal(project.resources.length, catalog.projectResourceCount);
assert.equal(
    catalog.projectResourceCount,
    catalog.exampleCount * 2 + catalog.dependencyCount + catalog.workbenchResourceCount,
);
assert.deepEqual(sampleIndex.projects.map(project => project.id), ['schema-examples', 'login', 'markdown-site', 'supplier-pipeline']);
assert.equal(sampleIndex.cacheUrls.length, catalog.cacheUrlCount + sampleIndex.projects.length + 1);
assert.deepEqual(sampleIndex.collections, [{ id: 'amsterdam-2026', name: 'Amsterdam 2026' }]);
const definition = await readJson(resolve(workspaceRoot, 'examples/projects/login/example.json'));
for (const record of [...sampleIndex.projects, sampleIndex.starter]) {
    assert.ok(sampleIndex.cacheUrls.includes(record.archive));
    const bytes = await readFile(resolve(projectRoot, 'dist/static/samples', record.archive.slice(2)));
    assert.equal(sha256(bytes), record.sha256);
    assert.deepEqual(bytes, await readFile(resolve(generatedRoot, record.archive.slice(2))));
    const bundle = parseCemStudioProjectBundle(bytes);
    assert.equal(bundle.project.resources.length, Object.keys(bundle.contents).length);
    assert.ok(bundle.project.entries.every(entry => !/^\d+\./.test(entry.name)), 'configurations are not tutorial steps');
    await mkdir(dirname(reportPath), { recursive: true });
    const manifest = resolve(dirname(reportPath), `${record.id}.project.json`);
    await writeFile(manifest, JSON.stringify(bundle.project));
    run(cli, ['validate', manifest, '--format', 'json', '--content-type', 'application/vnd.cem.studio-project+json', '--schema', 'https://cem.dev/ns/studio/project/1'], workspaceRoot);
    if (record.id === 'login') {
        assert.deepEqual(record.collections, ['amsterdam-2026']);
        assert.equal(bundle.project.entries.length, definition.configurations.length);
        for (const source of definition.sources) assert.deepEqual(bundle.contents[source.id], new Uint8Array(await readFile(resolve(workspaceRoot, source.source))));
        assert.equal(record.version, '2.8.0');
        const contract = JSON.parse(new TextDecoder().decode(bundle.contents['login-semantic-contract']));
        assert.equal(contract.length, 21);
        for (const id of ['semantics-html', 'semantics-cem']) {
            const configuration = bundle.project.entries.find(entry => entry.id === id);
            assert.equal(configuration.kind, 'query');
            assert.ok(configuration.resourceIds.includes('login-semantics-query'));
            assert.ok(configuration.resourceIds.includes('login-semantic-contract'));
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[configuration.runConfigResourceId]));
            assert.ok(command.argv.includes('queries/semantics.xpath'));
        }
        for (const id of ['semantics-html-cemql', 'semantics-cem-cemql']) {
            const configuration = bundle.project.entries.find(entry => entry.id === id);
            assert.equal(configuration.kind, 'query');
            assert.ok(configuration.resourceIds.includes('login-semantics-cemql'));
            assert.ok(configuration.resourceIds.includes('login-semantic-contract'));
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[configuration.runConfigResourceId]));
            assert.ok(command.argv.includes('queries/semantics.cemql'));
            assert.ok(command.argv.includes('--ready-names'));
            assert.ok(command.argv.includes('queueSize=1024'));
        }
        assert.equal(bundle.project.entries.length, 20);
        const expression = bundle.project.resources.find(resource => resource.id === 'login-expression');
        assert.equal(expression.path, 'queries/summary.cem-ql');
        assert.equal(expression.role, 'query');
        assert.equal(expression.contentType, 'application/vnd.cem.query-expression+cem-ql');
        assert.equal(expression.schema, 'https://cem.dev/ns/query/cem-ql/1#expression');
        for (const id of ['expression-html', 'expression-cem', 'expression-format-pretty', 'expression-format-tabular']) {
            const entry = bundle.project.entries.find(entry => entry.id === id);
            assert.ok(entry.resourceIds.includes(expression.id));
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[entry.runConfigResourceId]));
            assert.ok(command.argv.includes(expression.contentType));
            assert.ok(command.argv.includes(expression.schema));
        }
        const template = bundle.project.resources.find(resource => resource.id === 'login-view-template');
        assert.equal(template.role, 'template');
        assert.equal(template.contentType, 'application/vnd.cem.template+cem');
        assert.equal(template.schema, 'https://cem.dev/ns/template/cem-native/1');
        const expectedView = new TextDecoder().decode(bundle.contents['login-view-expected']);
        assert.ok(expectedView.includes('aria-describedby="login-help"'));
        for (const id of ['render-html', 'render-cem']) {
            const configuration = bundle.project.entries.find(entry => entry.id === id);
            assert.equal(configuration.kind, 'transformation');
            assert.ok(configuration.resourceIds.includes('login-view-template'));
            assert.ok(configuration.resourceIds.includes('login-view-expected'));
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[configuration.runConfigResourceId]));
            assert.ok(command.argv.includes('templates/view.cemt'));
            assert.ok(command.argv.includes('text/html'));
            assert.ok(command.argv.includes('--template-entrypoint'));
        }
        const graph = bundle.project.resources.find(resource => resource.id === 'login-view-graph');
        assert.equal(graph.role, 'transform-config');
        assert.equal(graph.path, 'graphs/view.cem');
        assert.equal(graph.schema, 'https://cem.dev/ns/cli/transform-config/1');
        assert.ok(bundle.project.entries.find(entry => entry.id === 'live-view').tags.includes('live-preview'));
        assert.ok(bundle.project.resources.find(resource => resource.id === 'login-live-view').role === 'template');
        const graphConfiguration = bundle.project.entries.find(entry => entry.id === 'render-graph');
        assert.equal(graphConfiguration.kind, 'transformation-graph');
        for (const id of ['login-view-graph', 'login-cem', 'login-view-template', 'login-view-expected']) assert.ok(graphConfiguration.resourceIds.includes(id));
        const graphCommand = JSON.parse(new TextDecoder().decode(bundle.contents[graphConfiguration.runConfigResourceId]));
        assert.ok(graphCommand.argv.includes('--config'));
        assert.ok(graphCommand.argv.includes('graphs/view.cem'));
        const repair = bundle.project.entries.find(entry => entry.id === 'validation-repair');
        assert.deepEqual(repair.resourceIds, ['login-invalid', 'command-validation-repair', 'login-schema-package', 'login-schema']);
        assert.ok(new TextDecoder().decode(bundle.contents.readme).includes('demo.login.method_required'));
        assert.deepEqual(JSON.parse(new TextDecoder().decode(bundle.contents[repair.runConfigResourceId])).schemaPackages, [{ uri: 'schemas/login-schema-package.cem' }]);
    } else if (record.id === 'supplier-pipeline') {
        const supplier = await readJson(resolve(workspaceRoot, 'examples/projects/supplier-pipeline/example.json'));
        assert.deepEqual(record.collections, ['amsterdam-2026']);
        assert.equal(record.version, supplier.version);
        assert.equal(bundle.project.entries.length, supplier.configurations.length);
        for (const source of supplier.sources) assert.deepEqual(bundle.contents[source.id], new Uint8Array(await readFile(resolve(workspaceRoot, source.source))));
        const pipeline = bundle.project.entries.find(entry => entry.id === 'availability');
        assert.equal(pipeline.kind, 'transformation-graph');
        for (const id of ['supplier-stock', 'supplier-catalog', 'supplier-reconcile', 'supplier-view']) assert.ok(pipeline.resourceIds.includes(id));
        assert.ok(new TextDecoder().decode(bundle.contents.readme).includes('Failure and repair'));
        const command = JSON.parse(new TextDecoder().decode(bundle.contents[pipeline.runConfigResourceId]));
        assert.ok(command.argv.includes('graphs/availability.cem'));
        const directory = await mkdtemp(resolve(tmpdir(), 'cem-studio-supplier-'));
        try {
            for (const resource of bundle.project.resources) {
                const path = resolve(directory, resource.path);
                await mkdir(dirname(path), { recursive: true });
                await writeFile(path, bundle.contents[resource.id]);
            }
            for (const entry of bundle.project.entries) {
                const command = JSON.parse(new TextDecoder().decode(bundle.contents[entry.runConfigResourceId]));
                run(cli, command.argv, directory);
            }
            const outputs = supplier.configurations.find(configuration => configuration.id === 'availability').expectedGraphOutputs;
            for (const [path, expectedId] of Object.entries(outputs)) {
                const actual = await readFile(resolve(directory, path), 'utf8');
                const expected = new TextDecoder().decode(bundle.contents[expectedId]);
                if (path.endsWith('.json')) assert.deepEqual(JSON.parse(actual), JSON.parse(expected));
                else assert.equal(actual.trim(), expected.trim());
            }
        } finally { await rm(directory, { recursive: true, force: true }); }
    } else if (record.id === 'markdown-site') {
        const site = await readJson(resolve(workspaceRoot, 'examples/projects/markdown-site/example.json'));
        assert.deepEqual(record.collections, ['amsterdam-2026']);
        assert.equal(bundle.project.entries.length, 3);
        for (const source of site.sources) assert.deepEqual(bundle.contents[source.id], new Uint8Array(await readFile(resolve(workspaceRoot, source.source))));
        const build = site.configurations.find(configuration => configuration.id === 'build-site');
        const entry = bundle.project.entries.find(entry => entry.id === build.id);
        assert.equal(entry.kind, 'transformation-graph');
        assert.equal(entry.entryArtifact, 'generated/index.html');
        const directory = await mkdtemp(resolve(tmpdir(), 'cem-studio-markdown-site-'));
        try {
            for (const resource of bundle.project.resources) {
                const path = resolve(directory, resource.path);
                await mkdir(dirname(path), { recursive: true });
                await writeFile(path, bundle.contents[resource.id]);
            }
            for (const entry of bundle.project.entries) {
                const command = JSON.parse(new TextDecoder().decode(bundle.contents[entry.runConfigResourceId]));
                run(cli, command.argv ?? ['validate', '--config', bundle.project.resources.find(resource => resource.id === entry.runConfigResourceId).path], directory);
            }
            const outputs = async () => Promise.all(Object.keys(build.expectedGraphOutputs).map(async path => [path, sha256(await readFile(resolve(directory, path)))]));
            const baseline = await outputs();
            for (const [path, expectedId] of Object.entries(build.expectedGraphOutputs)) {
                assert.deepEqual(new Uint8Array(await readFile(resolve(directory, path))), bundle.contents[expectedId]);
            }
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[entry.runConfigResourceId]));
            run(cli, command.argv, directory);
            assert.deepEqual(await outputs(), baseline);
            for (const path of ['index', 'articles/index', 'articles/first-build', 'articles/shared-layouts']) {
                const map = await readJson(resolve(directory, `generated/${path}.html.map`));
                const serialized = JSON.stringify(map);
                assert.ok(serialized.includes(`content/${path}.md`), `content origin missing for ${path}`);
                assert.ok(serialized.includes('partials/chrome.cemt'));
                assert.ok(serialized.includes('partials/content.cemt'));
            }
        } finally { await rm(directory, { recursive: true, force: true }); }
    } else if (record.id === 'schema-examples') {
        for (const configuration of catalog.workbenches) {
            const resource = bundle.project.resources.find(item => item.id === configuration.runConfigResourceId);
            assert.equal(resource.schema, 'https://cem.dev/ns/cli/command/1');
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[resource.id]));
            assert.ok(command.argv.every(argument => !argument.startsWith('$')), 'commands own their concrete project addresses');
        }
    }
}
assert.equal(featureTourBuild.summary.errorCount, 0);
assert.equal(featureTourBuild.summary.fatalCount, 0);

run(cli, [
    'validate',
    '--format',
    'json',
    '--content-type',
    'application/vnd.cem.studio-project+json',
    '--schema',
    'https://cem.dev/ns/studio/project/1',
    'packages/cem-studio/generated/feature-tour/feature-tour.project.json',
], workspaceRoot);
let nativeValidationBytes = 0;
for (const example of catalog.examples) {
    assert.equal(example.operation, 'validate');
    assert.equal(example.expectedResult, 'pass');
    const [source, deployed] = await Promise.all([
        readFile(resolve(workspaceRoot, example.source)),
        readFile(resolve(outputRoot, example.asset.slice(2))),
    ]);
    assert.deepEqual(deployed, source, `${example.id} was not copied byte-for-byte by the generated graph`);
    assert.equal(sha256(deployed), example.sha256, `${example.id} hash drifted`);
    await readFile(resolve(outputRoot, example.runConfig.slice(2)));
    for (const dependency of example.dependencies) {
        const [dependencySource, dependencyDeployed] = await Promise.all([
            readFile(resolve(workspaceRoot, dependency.source)),
            readFile(resolve(outputRoot, dependency.asset.slice(2))),
        ]);
        assert.deepEqual(
            dependencyDeployed,
            dependencySource,
            `${dependency.resourceId} was not copied byte-for-byte by the generated graph`,
        );
        assert.equal(sha256(dependencyDeployed), dependency.sha256, `${dependency.resourceId} hash drifted`);
    }
    nativeValidationBytes += Buffer.byteLength(run(cli, [
        'validate',
        '--format',
        'json',
        '--content-type',
        example.contentType,
        '--schema',
        example.schema,
        example.source,
    ], workspaceRoot));
}
for (const workbench of catalog.workbenches) {
    const [asset, expected, runConfig] = await Promise.all([
        readFile(resolve(outputRoot, workbench.asset.slice(2))),
        readFile(resolve(outputRoot, workbench.expected.slice(2))),
        readFile(resolve(outputRoot, workbench.runConfig.slice(2))),
    ]);
    assert.equal(sha256(asset), workbench.sha256, `${workbench.id} input hash drifted`);
    assert.equal(sha256(expected), workbench.expectedSha256, `${workbench.id} expected result hash drifted`);
    assert.ok(JSON.parse(expected).kind, `${workbench.id} expected result has no kind`);
    assert.ok(JSON.parse(runConfig).inputs?.length > 0, `${workbench.id} run config has no input`);
    for (const dependency of workbench.dependencies) {
        const bytes = await readFile(resolve(outputRoot, dependency.asset.slice(2)));
        assert.equal(sha256(bytes), dependency.sha256, `${workbench.id} dependency ${dependency.resourceId} drifted`);
    }
}

const report = {
    schemaVersion: 1,
    project: '@epa-wg/cem-studio',
    seed: catalog.seed,
    selection: catalog.selection,
    packageCount: catalog.packageCount,
    exampleCount: catalog.exampleCount,
    dependencyCount: catalog.dependencyCount,
    workbenchCount: catalog.workbenchCount,
    workbenchResourceCount: catalog.workbenchResourceCount,
    nativeValidationBytes,
    graphAssembly: 'cem-ml-transform-graph',
};
await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`Verified Feature Tour seed and ${catalog.exampleCount} native schema-package examples.`);

function run(command, args, cwd) {
    const result = spawnSync(command, args, { cwd, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
    if (result.error) throw result.error;
    if (result.status !== 0) {
        throw new Error(`${command} ${args.join(' ')} failed (${result.status}):\n${result.stderr || result.stdout}`);
    }
    return result.stdout;
}

function sha256(bytes) {
    return createHash('sha256').update(bytes).digest('hex');
}

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}
