import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';

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
assert.equal(catalog.packageCount, 31);
assert.equal(catalog.exampleCount, catalog.packageCount);
assert.equal(catalog.capability.operation, 'validate');
assert.equal(catalog.capability.availability, 'available');
assert.equal(project.id, catalog.seed.id);
assert.equal(project.entries.filter(({ kind }) => kind === 'validation').length, catalog.exampleCount);
assert.equal(project.resources.length, catalog.projectResourceCount);
assert.equal(
    catalog.projectResourceCount,
    catalog.exampleCount * 2 + catalog.dependencyCount,
);
assert.equal(sampleIndex.samples[0].id, catalog.seed.id);
assert.equal(sampleIndex.cacheUrls.length, catalog.cacheUrlCount);
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

const report = {
    schemaVersion: 1,
    project: '@epa-wg/cem-studio',
    seed: catalog.seed,
    selection: catalog.selection,
    packageCount: catalog.packageCount,
    exampleCount: catalog.exampleCount,
    dependencyCount: catalog.dependencyCount,
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
