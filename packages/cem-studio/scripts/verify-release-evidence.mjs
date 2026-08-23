import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve } from 'node:path';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-studio');
const reportPath = resolve(reportRoot, 'release-evidence.json');
const packageRoot = resolve(workspaceRoot, 'dist/packages/cem-studio');

const [metadata, build, deterministic, dependencies, consumer, packageReport, nx, cargo, archive] = await Promise.all([
    readJson(resolve(projectRoot, 'package.json')),
    readJson(resolve(projectRoot, 'dist/static/build.json')),
    readJson(resolve(reportRoot, 'determinism.json')),
    readJson(resolve(reportRoot, 'dependencies.json')),
    readJson(resolve(reportRoot, 'consumer.json')),
    readJson(resolve(packageRoot, 'package.json')),
    readJson(resolve(workspaceRoot, 'nx.json')),
    readFile(resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml'), 'utf8'),
    readFile(resolve(packageRoot, 'package.tgz')),
]);

const releaseGroup = nx.release.groups['cem-ml-platform'];
const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(cargo)?.[1];
assert.equal(releaseGroup.projectsRelationship, 'fixed');
assert.ok(releaseGroup.projects.includes(metadata.name));
for (const value of [
    build.commonVersion,
    dependencies.commonVersion,
    consumer.version,
    packageReport.version,
    cargoVersion,
]) {
    assert.equal(value, metadata.version);
}
assert.equal(build.package, metadata.name);
assert.equal(dependencies.project, metadata.name);
assert.equal(consumer.package, metadata.name);
assert.equal(packageReport.package, metadata.name);
assert.equal(packageReport.sha256, sha256(archive));
assert.equal(consumer.runtimeCopies, 1);
assert.equal(consumer.exactCliDependency, metadata.dependencies['@epa-wg/cem-ml-cli']);
assert.equal(dependencies.exactCliDependency, metadata.dependencies['@epa-wg/cem-ml-cli']);
assert.equal(deterministic.cleanBuilds, 2);
assert.equal(deterministic.aggregateSha256.length, 64);

const sourcePaths = gitLines([
    'ls-files',
    '--cached',
    '--others',
    '--exclude-standard',
    '--',
    'packages/cem-studio',
    'packages/cem_ml/Cargo.toml',
    'packages/cem-ml-npm/package.json',
    'packages/cem-ml-cli-npm/package.json',
    'nx.json',
    'yarn.lock',
]);
const sourceDigest = createHash('sha256');
for (const path of sourcePaths) {
    const bytes = await readFile(resolve(workspaceRoot, path));
    sourceDigest.update(`${path}\0${bytes.byteLength}\0`);
    sourceDigest.update(bytes);
}
const sourceDirty = gitLines(['status', '--porcelain=v1', '--', ...sourcePaths]).length > 0;
const sourceRevision = git(['rev-parse', 'HEAD']);
const repository = git(['config', '--get', 'remote.origin.url'], true) || 'local';
const dependencyInventory = Object.entries(metadata.dependencies)
    .map(([name, version]) => Object.freeze({ name, version }))
    .sort((left, right) => left.name.localeCompare(right.name));

const report = {
    schemaVersion: 1,
    subject: {
        package: metadata.name,
        version: metadata.version,
        archive: packageReport.filename,
        bytes: packageReport.bytes,
        sha256: packageReport.sha256,
    },
    synchronization: {
        releaseGroup: 'cem-ml-platform',
        relationship: releaseGroup.projectsRelationship,
        commonVersion: metadata.version,
        cargoVersion,
        cliVersion: dependencies.exactCliDependency,
    },
    source: {
        repository,
        revision: sourceRevision,
        treeSha256: sourceDigest.digest('hex'),
        fileCount: sourcePaths.length,
        dirty: sourceDirty,
    },
    build: {
        identity: build.buildIdentity,
        assembly: build.assembly,
        cleanBuilds: deterministic.cleanBuilds,
        staticFileCount: deterministic.fileCount,
        staticSha256: deterministic.aggregateSha256,
    },
    sbom: {
        format: 'cem-studio-dependency-inventory-v1',
        dependencies: dependencyInventory,
        runtimePaths: dependencies.runtimePaths,
    },
    verification: {
        dependencyAudit: relative(workspaceRoot, resolve(reportRoot, 'dependencies.json')),
        cleanConsumer: relative(workspaceRoot, resolve(reportRoot, 'consumer.json')),
        deterministicOutput: relative(workspaceRoot, resolve(reportRoot, 'determinism.json')),
        packageInstallVerified: consumer.staticBootstrap && consumer.importSideEffects === false,
        runtimeCopies: consumer.runtimeCopies,
    },
    provenance: {
        buildType: 'https://cem.dev/build/cem-studio-static-v1',
        builder: 'nx:@epa-wg/cem-studio:check',
        publicationAndSigningDeferredToPhase: 9,
        publicationReady: !sourceDirty,
    },
};

await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(
    `Verified synchronized Studio release evidence for ${metadata.name}@${metadata.version}` +
        ` (${sourceDirty ? 'working tree evidence' : 'publication-ready source'}).`,
);

function sha256(value) {
    return createHash('sha256').update(value).digest('hex');
}

function git(args, allowEmpty = false) {
    const result = spawnSync('git', args, { cwd: workspaceRoot, encoding: 'utf8' });
    if (result.status !== 0 && !allowEmpty) {
        throw new Error(`git ${args.join(' ')} failed: ${result.stderr || result.stdout || result.error}`);
    }
    return result.stdout.trim();
}

function gitLines(args) {
    const value = git(args);
    return value ? value.split(/\r?\n/u) : [];
}

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}
