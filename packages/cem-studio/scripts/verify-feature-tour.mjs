import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, readdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
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
const inventory = await readFile(resolve(workspaceRoot, 'docs/studio-content-types.md'), 'utf8');
const registry = await readJson(resolve(workspaceRoot, 'packages/cem_ml/schema/schema-assets-registry-v1.json'));
const runtime = await readJson(resolve(workspaceRoot, 'packages/cem-ml-npm/dist/cem-ml-runtime.json'));
const documentedPackages = inventoryRows('native-identities').map(row => row.map(inlineCodes));
assert.deepEqual(documentedPackages, registry.map(item => [
    [item.packageId], [item.version], [item.schemaUri],
    item.contentTypes.filter(type => type.primary).map(type => type.value),
    item.contentTypes.filter(type => !type.primary).map(type => type.value),
]), 'Studio identity inventory drifted from the native-emitted registry');
assert.deepEqual(inventoryRows('browser-operations').map(row => row.map(cell => inlineCodes(cell)[0])),
    runtime.capabilities.browser.operations.map(item => [item.operation, item.availability]),
    'Studio browser operation inventory drifted from native capabilities');
const documentedSamples = inventoryRows('existing-samples');
const sampleBundles = new Map();

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
const authoredRoot = resolve(workspaceRoot, 'examples/projects');
const authoredDefinitions = await Promise.all((await readdir(authoredRoot, { withFileTypes: true }))
    .filter(item => item.isDirectory()).sort((left, right) => left.name.localeCompare(right.name))
    .map(item => readJson(resolve(authoredRoot, item.name, 'example.json'))));
assert.deepEqual(sampleIndex.projects.map(project => project.id), ['schema-examples', ...authoredDefinitions.map(project => project.id)]);
assert.equal(sampleIndex.cacheUrls.length, catalog.cacheUrlCount + sampleIndex.projects.length + 1);
assert.deepEqual(sampleIndex.collections, await readJson(resolve(workspaceRoot, 'examples/projects/collections.json')));
const categoryIds = ['content-types-data', 'content-types-documents-ui', 'content-types-queries-transformations', 'content-types-schema-authoring'];
assert.deepEqual(sampleIndex.collections.filter(collection => collection.parentId === 'content-types-and-schemas').map(collection => collection.id), categoryIds);
for (const id of categoryIds) assert.ok(sampleIndex.projects.some(project => project.collections.includes(id)), `${id} has no existing project`);
const definition = await readJson(resolve(workspaceRoot, 'examples/projects/login/example.json'));
for (const record of [...sampleIndex.projects, sampleIndex.starter]) {
    assert.ok(sampleIndex.cacheUrls.includes(record.archive));
    const bytes = await readFile(resolve(projectRoot, 'dist/static/samples', record.archive.slice(2)));
    assert.equal(sha256(bytes), record.sha256);
    assert.deepEqual(bytes, await readFile(resolve(generatedRoot, record.archive.slice(2))));
    const bundle = parseCemStudioProjectBundle(bytes);
    sampleBundles.set(record.id, bundle);
    assert.equal(bundle.project.resources.length, Object.keys(bundle.contents).length);
    assert.ok(bundle.project.entries.every(entry => !/^\d+\./.test(entry.name)), 'configurations are not tutorial steps');
    await mkdir(dirname(reportPath), { recursive: true });
    const manifest = resolve(dirname(reportPath), `${record.id}.project.json`);
    await writeFile(manifest, JSON.stringify(bundle.project));
    run(cli, ['validate', manifest, '--format', 'json', '--content-type', 'application/vnd.cem.studio-project+json', '--schema', 'https://cem.dev/ns/studio/project/1'], workspaceRoot);
    if (record.id === 'login') {
        assert.deepEqual(record.collections, definition.collections);
        assert.equal(record.description, definition.description);
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
    } else if (['json-yaml-report', 'csv-inventory', 'supplier-pipeline', 'xhtml-document', 'html-cem-card', 'svg-diagram', 'mathml-formula', 'css-stylesheet', 'css-selector', 'scss-stylesheet', 'cemql-orders', 'xpath-catalog', 'cem-transform', 'xslt-transform', 'cem-schema', 'json-schema', 'relax-ng', 'module-maps', 'portable-config', 'native-projections'].includes(record.id)) {
        const definition = authoredDefinitions.find(project => project.id === record.id);
        assert.deepEqual(record.collections, definition.collections);
        assert.equal(record.description, definition.description);
        assert.equal(record.version, definition.version);
        assert.equal(bundle.project.entries.length, definition.configurations.length);
        for (const source of definition.sources) assert.deepEqual(bundle.contents[source.id], new Uint8Array(await readFile(resolve(workspaceRoot, source.source))));
        if (record.id === 'supplier-pipeline') {
            const pipeline = bundle.project.entries.find(entry => entry.id === 'availability');
            assert.equal(pipeline.kind, 'transformation-graph');
            for (const id of ['supplier-stock', 'supplier-catalog', 'supplier-reconcile', 'supplier-view']) assert.ok(pipeline.resourceIds.includes(id));
            assert.ok(new TextDecoder().decode(bundle.contents.readme).includes('Failure and repair'));
            const command = JSON.parse(new TextDecoder().decode(bundle.contents[pipeline.runConfigResourceId]));
            assert.ok(command.argv.includes('graphs/availability.cem'));
        }
        const directory = await mkdtemp(resolve(tmpdir(), `cem-studio-${record.id}-`));
        try {
            for (const resource of bundle.project.resources) {
                const path = resolve(directory, resource.path);
                await mkdir(dirname(path), { recursive: true });
                await writeFile(path, bundle.contents[resource.id]);
            }
            for (const configuration of definition.configurations) {
                const entry = bundle.project.entries.find(entry => entry.id === configuration.id);
                const configResource = bundle.project.resources.find(resource => resource.id === entry.runConfigResourceId);
                const command = JSON.parse(new TextDecoder().decode(bundle.contents[configResource.id]));
                const authoredArgs = command.argv ?? ['validate', '--config', configResource.path, '--format', 'json', '--no-color'];
                // The CLI's explicit, bounded local grant is a host concern.
                // Portable project commands keep relative resource references;
                // Studio instead supplies its owned virtual project resources.
                const args = record.id === 'scss-stylesheet'
                    ? [...authoredArgs.map(value => bundle.project.resources.some(resource => resource.path === value)
                        ? pathToFileURL(resolve(directory, value)).href : value),
                    '--resolver-read-map', `${pathToFileURL(`${directory}/`).href}=${directory}`,
                    '--base-uri', pathToFileURL(`${directory}/styles/`).href]
                    : authoredArgs;
                const moduleReport = configuration.expectedNativeAssets && resolve(directory, `native-${configuration.id}.report.json`);
                if (moduleReport) args.push('--report-json', moduleReport);
                if (configuration.expectedOutputBytes) {
                    assert.deepEqual(new Uint8Array(runExpected(cli, args, directory, configuration.expectedExit ?? 0, null)),
                        bundle.contents[configuration.expectedOutputBytes], `${configuration.id} binary output drifted`);
                    continue;
                }
                const actual = runExpected(cli, args, directory, configuration.expectedExit ?? 0);
                if (configuration.expectedSemanticContract) {
                    const expected = JSON.parse(new TextDecoder().decode(bundle.contents[configuration.expectedSemanticContract]));
                    const report = JSON.parse(actual);
                    const values = report.language === 'css-selector'
                        ? report.matches.map(({ localName, namespaceUri, documentOrder, nodeId }) => ({ localName, namespaceUri, documentOrder, nodeId }))
                        : report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue) : report.result.items.map(item => item.value);
                    assert.deepEqual(values, expected, `${configuration.id} semantic result drifted`);
                }
                if (configuration.expectedNativeItems || configuration.expectedNativeNodes || configuration.expectedNativeRanges) {
                    const report = JSON.parse(actual);
                    const items = report.language === 'xpath' ? report.result.sequence.items : report.result.items;
                    const fixture = id => JSON.parse(new TextDecoder().decode(bundle.contents[id]));
                    const ownedSource = (uri, source) => typeof uri === 'string'
                        && new URL(uri, pathToFileURL(`${directory}/`)).href === pathToFileURL(resolve(directory, source.path)).href;
                    if (configuration.expectedNativeItems) assert.deepEqual(items, fixture(configuration.expectedNativeItems));
                    if (configuration.expectedNativeNodes) {
                        assert.deepEqual(items.map(({ kind, nodeId, nodeKind, expandedName, sourceRange }) => ({ kind, nodeId, nodeKind, expandedName, sourceRange })), fixture(configuration.expectedNativeNodes));
                        for (const item of items) assert.ok(ownedSource(item.sourceUri, bundle.project.resources.find(resource => resource.id === configuration.source)));
                    }
                    if (configuration.expectedNativeRanges) {
                        const source = bundle.project.resources.find(resource => resource.id === configuration.source);
                        assert.deepEqual(items.map(item => {
                            assert.equal(item.kind, 'native'); assert.equal(item.representation, 'cem.ql.imported-cem-ast');
                            const frame = item.sourceMap.frames.find(frame => frame.transform.kind === 'SourceBoundary');
                            assert.ok(ownedSource(frame.transform.uri, source)); return frame.span.ranges;
                        }), fixture(configuration.expectedNativeRanges));
                    }
                }
                if (configuration.expectedOutput) {
                    const expected = new TextDecoder().decode(bundle.contents[configuration.expectedOutput]);
                    const exact = ['csv-inventory', 'svg-diagram', 'mathml-formula', 'css-stylesheet', 'css-selector', 'scss-stylesheet', 'cemql-orders', 'xpath-catalog', 'cem-transform', 'xslt-transform', 'cem-schema', 'json-schema', 'relax-ng', 'module-maps', 'portable-config', 'native-projections'].includes(record.id);
                    assert.equal(exact ? actual : actual.trim(), exact ? expected : expected.trim(), `${configuration.id} output drifted`);
                }
                for (const text of configuration.expectedText ?? []) assert.ok(actual.includes(text), `${configuration.id} misses ${text}`);
                if (configuration.expectedValidation) {
                    const report = JSON.parse(actual);
                    const expected = configuration.expectedValidation;
                    assert.equal(report.summary.errorCount, expected.errorCount);
                    assert.equal(report.summary.warningCount, expected.warningCount);
                    if (configuration.repair) {
                        const code = expected.errorCode ?? expected.warningCode;
                        const diagnostic = report.diagnostics.find(item => item.code === code);
                        assert.equal(diagnostic?.severity, expected.errorCode ? 'error' : 'warning');
                        if (!expected.location) assert.ok(diagnostic.line > 0 && diagnostic.column > 0 && diagnostic.byteOffset >= 0, `${configuration.id} needs native source coordinates`);
                        const source = bundle.project.resources.find(resource => resource.id === configuration.source);
                        const diagnosticSource = bundle.project.resources.find(resource => resource.id === (expected.source ?? configuration.source));
                        assert.ok(diagnostic.uri?.endsWith(diagnosticSource.path), `${configuration.id} diagnostic must identify its source`);
                        if (expected.location) {
                            for (const [key, value] of Object.entries(expected.location)) assert.equal(diagnostic[key] ?? null, value, `${configuration.id} diagnostic ${key} drifted`);
                        }
                        const original = new TextDecoder().decode(bundle.contents[source.id]);
                        const repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                        assert.notEqual(repaired, original, `${configuration.id} repair must change the invalid source`);
                        await writeFile(resolve(directory, source.path), repaired);
                        const repairedReport = JSON.parse(run(cli, args, directory));
                        assert.equal(repairedReport.summary.errorCount, 0);
                        assert.equal(repairedReport.summary.warningCount, 0);
                        assert.equal(repairedReport.summary.fatalCount, 0);
                        await writeFile(resolve(directory, source.path), bundle.contents[source.id]);
                        await writeFile(resolve(directory, configResource.path), bundle.contents[configResource.id]);
                    }
                }
                if (configuration.expectedGraphOutputs) {
                    assert.ok(Object.hasOwn(configuration.expectedGraphOutputs, entry.entryArtifact));
                    const baseline = new Map();
                    for (const [path, expectedId] of Object.entries(configuration.expectedGraphOutputs)) {
                        const bytes = new Uint8Array(await readFile(resolve(directory, path)));
                        baseline.set(path, bytes);
                        const actual = new TextDecoder().decode(bytes);
                        const expected = new TextDecoder().decode(bundle.contents[expectedId]);
                        if (path.endsWith('.json')) assert.deepEqual(JSON.parse(actual), JSON.parse(expected));
                        else assert.equal(actual.trim(), expected.trim());
                        if (record.id !== 'supplier-pipeline') assert.deepEqual(bytes, bundle.contents[expectedId], `${path} output fixture drifted`);
                    }
                    for (const path of configuration.expectedUnmappedOutputs ?? []) {
                        assert.ok(Object.hasOwn(configuration.expectedGraphOutputs, path));
                        await assert.rejects(readFile(resolve(directory, `${path}.map`)), { code: 'ENOENT' });
                    }
                    if (moduleReport) {
                        const manifest = (await readJson(moduleReport)).reportAst.transformGraph.moduleAssetManifest;
                        assert.equal(manifest.hashScheme, 'sha256'); assert.equal(manifest.hash, configuration.expectedNativeAssetHash);
                        assert.equal(manifest.assetCount, configuration.expectedNativeAssets.length); assert.equal(manifest.assets.length, manifest.assetCount);
                        const owned = (uri, path) => new URL(uri, pathToFileURL(`${directory}/`)).href === pathToFileURL(resolve(directory, path)).href;
                        let changed = false;
                        for (const expected of configuration.expectedNativeAssets) {
                            const asset = manifest.assets.find(item => item.specifier === expected.specifier);
                            const source = bundle.project.resources.find(item => item.id === expected.source), map = bundle.project.resources.find(item => item.id === expected.sourceMap);
                            const output = await readFile(resolve(directory, expected.destination));
                            assert.ok(asset); assert.equal(asset.target, expected.target); assert.equal(asset.contentType, expected.contentType);
                            assert.ok(owned(asset.sourceUri, source.path)); assert.ok(owned(asset.sourceMapUri, map.path)); assert.ok(owned(asset.destination, expected.destination));
                            assert.equal(asset.sourceByteLength, bundle.contents[source.id].byteLength); assert.equal(asset.sourceSha256, sha256(bundle.contents[source.id]));
                            assert.equal(asset.byteLength, output.byteLength); assert.equal(asset.sha256, sha256(output));
                            assert.deepEqual(new Uint8Array(output), bundle.contents[expected.expectedOutput]);
                            changed ||= asset.sourceSha256 !== asset.sha256 || asset.sourceByteLength !== asset.byteLength;
                        }
                        assert.equal(manifest.contractVersion, changed ? 2 : 1);
                    }
                    if (!configuration.expectedUnmappedOutputs?.includes(entry.entryArtifact)) {
                        const map = await readJson(resolve(directory, `${entry.entryArtifact}.map`));
                        assert.ok(map.outputSpans.length > 0);
                        for (const path of configuration.expectedProvenance ?? ['templates/view.cemt']) assert.ok(JSON.stringify(map).includes(`/${path}`));
                    } else assert.ok(!configuration.expectedProvenance?.length, 'Unmapped output must not claim template/input spans');
                    run(cli, args, directory);
                    for (const [path, expectedId] of Object.entries(configuration.expectedGraphOutputs)) {
                        const bytes = new Uint8Array(await readFile(resolve(directory, path)));
                        assert.deepEqual(bytes, baseline.get(path), `${path} repeated build drifted`);
                        if (record.id !== 'supplier-pipeline') assert.deepEqual(bytes, bundle.contents[expectedId], `${path} repeated output fixture drifted`);
                    }
                    const failures = configuration.expectedGraphFailures ?? (record.id === 'json-yaml-report'
                        ? [['api-payload', 'invalid-payload'], ['report-settings', 'invalid-settings']] : []);
                    for (const [sourceId, invalidId] of failures) {
                        const source = bundle.project.resources.find(resource => resource.id === sourceId);
                        await rm(resolve(directory, 'generated'), { recursive: true, force: true });
                        await writeFile(resolve(directory, source.path), bundle.contents[invalidId]);
                        assert.equal(runExpected(cli, args, directory, 1).trim(), '', `${invalidId} must not publish output`);
                        for (const path of Object.keys(configuration.expectedGraphOutputs)) {
                            await assert.rejects(readFile(resolve(directory, path)), { code: 'ENOENT' });
                        }
                        await writeFile(resolve(directory, source.path), bundle.contents[sourceId]);
                        run(cli, args, directory);
                        for (const [path, expectedId] of Object.entries(configuration.expectedGraphOutputs)) {
                            const bytes = new Uint8Array(await readFile(resolve(directory, path)));
                            assert.deepEqual(bytes, baseline.get(path), `${path} repaired build drifted`);
                            if (record.id !== 'supplier-pipeline') assert.deepEqual(bytes, bundle.contents[expectedId], `${path} repaired output fixture drifted`);
                        }
                    }
                    for (const refusal of configuration.expectedGraphRefusals ?? []) {
                        const source = bundle.project.resources.find(item => item.id === refusal.source);
                        await rm(resolve(directory, 'generated'), { recursive: true, force: true });
                        await writeFile(resolve(directory, source.path), bundle.contents[refusal.replacement]);
                        const syntax = JSON.parse(run(cli, ['validate', source.path, '--content-type', source.contentType, '--schema', source.schema, '--format', 'json'], directory));
                        assert.equal(syntax.summary.hardViolationCount, 0, 'Source syntax validation is separate from deployment planning');
                        const denied = spawnSync(cli, args, { cwd: directory, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
                        if (denied.error) throw denied.error;
                        assert.equal(denied.status, refusal.exitCode); assert.equal(denied.stdout, ''); assert.ok(denied.stderr.includes(refusal.code), denied.stderr);
                        for (const path of Object.keys(configuration.expectedGraphOutputs)) await assert.rejects(readFile(resolve(directory, path)), { code: 'ENOENT' });
                        await writeFile(resolve(directory, source.path), bundle.contents[refusal.source]); run(cli, args, directory);
                        for (const [path, bytes] of baseline) assert.deepEqual(new Uint8Array(await readFile(resolve(directory, path))), bytes);
                    }
                }
            }
        } finally { await rm(directory, { recursive: true, force: true }); }
    } else if (record.id === 'markdown-site') {
        const site = await readJson(resolve(workspaceRoot, 'examples/projects/markdown-site/example.json'));
        assert.deepEqual(record.collections, site.collections);
        assert.equal(record.description, site.description);
        assert.equal(record.version, site.version);
        assert.equal(bundle.project.entries.length, site.configurations.length);
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
            for (const configuration of site.configurations) {
                const entry = bundle.project.entries.find(item => item.id === configuration.id);
                const configResource = bundle.project.resources.find(resource => resource.id === entry.runConfigResourceId);
                const command = JSON.parse(new TextDecoder().decode(bundle.contents[entry.runConfigResourceId]));
                const args = command.argv ?? ['validate', '--config', configResource.path, '--format', 'json', '--no-color'];
                const actual = runExpected(cli, args, directory, configuration.expectedExit ?? 0);
                if (configuration.expectedOutput) {
                    assert.equal(actual, new TextDecoder().decode(bundle.contents[configuration.expectedOutput]), `${configuration.id} article output drifted`);
                }
                for (const text of configuration.expectedText ?? []) assert.ok(actual.includes(text), `${configuration.id} misses ${text}`);
                if (configuration.expectedSemanticContract) {
                    const report = JSON.parse(actual);
                    const values = report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue) : report.result.items.map(item => item.value);
                    assert.deepEqual(values, JSON.parse(new TextDecoder().decode(bundle.contents[configuration.expectedSemanticContract])));
                }
                if (configuration.expectedValidation) {
                    const expected = configuration.expectedValidation, report = JSON.parse(actual);
                    assert.equal(report.summary.errorCount, expected.errorCount); assert.equal(report.summary.warningCount, expected.warningCount);
                    if (configuration.repair) {
                        const source = bundle.project.resources.find(resource => resource.id === configuration.source);
                        const diagnostic = report.diagnostics.find(item => item.code === expected.errorCode);
                        assert.equal(diagnostic?.severity, 'error'); assert.ok(diagnostic.uri.endsWith(source.path));
                        for (const [key, value] of Object.entries(expected.location ?? {})) assert.equal(diagnostic[key] ?? null, value);
                        const original = new TextDecoder().decode(bundle.contents[source.id]);
                        const repaired = original.replace(configuration.repair.find, configuration.repair.replace);
                        assert.notEqual(repaired, original); await writeFile(resolve(directory, source.path), repaired);
                        const repairedReport = JSON.parse(run(cli, args, directory));
                        assert.equal(repairedReport.summary.errorCount, 0); assert.equal(repairedReport.summary.warningCount, 0); assert.equal(repairedReport.summary.fatalCount, 0);
                        await writeFile(resolve(directory, source.path), bundle.contents[source.id]);
                    }
                }
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
for (const [projectCell, resourceCell, configurationCell] of documentedSamples) {
    const projectId = inlineCodes(projectCell)[0];
    const resourceId = inlineCodes(resourceCell)[0];
    const configurationId = inlineCodes(configurationCell)[0];
    const bundle = sampleBundles.get(projectId);
    assert.ok(bundle, `Inventory references missing project ${projectId}`);
    assert.ok(bundle.project.resources.some(resource => resource.id === resourceId),
        `Inventory references missing resource ${projectId}/${resourceId}`);
    const configuration = bundle.project.entries.find(entry => entry.id === configurationId);
    assert.ok(configuration?.resourceIds.includes(resourceId),
        `Inventory configuration ${projectId}/${configurationId} does not use ${resourceId}`);
}
assert.deepEqual(catalog.examples.map(example => example.packageId), registry.map(item => item.packageId));
const schemaExamples = sampleBundles.get('schema-examples').project;
for (const item of registry) {
    const example = catalog.examples.find(example => example.packageId === item.packageId);
    assert.equal(example.packageVersion, item.version);
    assert.equal(example.schema, item.schemaUri);
    const essence = example.contentType.split(';')[0].trim().toLowerCase();
    assert.ok(item.contentTypes.some(type => type.value === essence));
    assert.equal(example.resourceId, `example-${item.packageId}`);
    assert.equal(example.runConfigResourceId, `run-${item.packageId}`);
    const configuration = schemaExamples.entries.find(entry => entry.id === `validate-${item.packageId}`);
    assert.equal(configuration?.runConfigResourceId, example.runConfigResourceId);
    assert.ok(configuration.resourceIds.includes(example.resourceId));
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
    contentTypeInventory: {
        packageCount: registry.length,
        contentTypeClaims: registry.reduce((count, item) => count + item.contentTypes.length, 0),
        distinctContentTypes: new Set(registry.flatMap(item => item.contentTypes.map(type => type.value))).size,
        browserOperationCount: runtime.capabilities.browser.operations.length,
        existingWorkflowCount: documentedSamples.length,
    },
    graphAssembly: 'cem-ml-transform-graph',
};
await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`Verified Feature Tour seed and ${catalog.exampleCount} native schema-package examples.`);

function run(command, args, cwd) {
    return runExpected(command, args, cwd, 0);
}

function runExpected(command, args, cwd, expectedStatus, encoding = 'utf8') {
    const result = spawnSync(command, args, { cwd, encoding, maxBuffer: 16 * 1024 * 1024 });
    if (result.error) throw result.error;
    if (result.status !== expectedStatus) {
        throw new Error(`${command} ${args.join(' ')} exited ${result.status}, expected ${expectedStatus}:\n${result.stderr || result.stdout}`);
    }
    return result.stdout;
}

function sha256(bytes) {
    return createHash('sha256').update(bytes).digest('hex');
}

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}

function inventoryRows(name) {
    const start = `<!-- ${name}:start -->`;
    const end = `<!-- ${name}:end -->`;
    assert.equal(inventory.split(start).length, 2, `Inventory needs one ${name} table`);
    assert.equal(inventory.split(end).length, 2, `Inventory needs one ${name} table end`);
    const body = inventory.split(start)[1].split(end)[0];
    const rows = body.trim().split('\n').map(line => line.trim().split('|').slice(1, -1).map(cell => cell.trim()));
    assert.ok(rows.length > 2 && rows.every(row => row.length === rows[0].length),
        `Inventory has malformed ${name} rows`);
    return rows.slice(2);
}

function inlineCodes(cell) {
    return [...cell.matchAll(/`([^`]+)`/g)].map(match => match[1]);
}
