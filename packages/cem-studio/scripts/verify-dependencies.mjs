import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';

import { createProjectGraphAsync } from '@nx/devkit';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/dependencies.json');
const studio = await readJson(resolve(projectRoot, 'package.json'));
const cli = await readJson(resolve(workspaceRoot, 'packages/cem-ml-cli-npm/package.json'));
const runtime = await readJson(resolve(workspaceRoot, 'packages/cem-ml-npm/package.json'));
const buildMetadata = await readJson(resolve(projectRoot, 'src/studio.build.json'));
const cargoManifest = await readFile(resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml'), 'utf8');
const commonVersion = cargoManifest.match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];

assert.ok(commonVersion, 'cannot read the common CEM-ML version authority');
assert.equal(studio.version, commonVersion);
assert.equal(cli.version, commonVersion);
assert.equal(runtime.version, commonVersion);
assert.deepEqual(Object.keys(studio.dependencies).sort(), [
    '@epa-wg/cem-components',
    '@epa-wg/cem-ml-cli',
    '@epa-wg/cem-theme',
]);
assert.equal(studio.dependencies['@epa-wg/cem-ml-cli'], commonVersion);
assert.equal(cli.dependencies['@epa-wg/cem-ml'], commonVersion);
assert.equal(studio.dependencies['@epa-wg/cem-ml'], undefined);
assert.equal(buildMetadata.commonVersion, commonVersion);
assert.deepEqual(buildMetadata.dependencies, studio.dependencies);

const graph = await createProjectGraphAsync({ exitOnError: false });
for (const project of [
    '@epa-wg/cem-studio',
    '@epa-wg/cem-ml-cli',
    '@epa-wg/cem-ml',
    '@epa-wg/cem-components',
    '@epa-wg/cem-theme',
]) {
    assert.ok(graph.nodes[project], `resolved Nx graph is missing ${project}`);
}

const directDependencies = new Set(
    (graph.dependencies['@epa-wg/cem-studio'] ?? []).map(({ target }) => target),
);
for (const dependency of ['@epa-wg/cem-ml-cli', '@epa-wg/cem-components', '@epa-wg/cem-theme']) {
    assert.ok(directDependencies.has(dependency), `Studio Nx dependencies are missing ${dependency}`);
}
assert.equal(
    directDependencies.has('@epa-wg/cem-ml'),
    false,
    'Studio must receive @epa-wg/cem-ml transitively through the CLI package',
);

const runtimePaths = uniquePaths(pathsBetween(graph.dependencies, '@epa-wg/cem-studio', '@epa-wg/cem-ml'));
assert.deepEqual(runtimePaths, [['@epa-wg/cem-studio', '@epa-wg/cem-ml-cli', '@epa-wg/cem-ml']]);

const report = {
    schemaVersion: 1,
    project: studio.name,
    commonVersion,
    exactCliDependency: studio.dependencies['@epa-wg/cem-ml-cli'],
    runtimePaths,
    directDependencies: [...directDependencies].sort(),
};
await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`Verified ${studio.name}@${commonVersion}: exact CLI dependency and one transitive runtime path.`);

function pathsBetween(dependencies, source, target, visited = new Set()) {
    if (source === target) return [[target]];
    if (visited.has(source)) return [];
    const nextVisited = new Set(visited).add(source);
    return (dependencies[source] ?? []).flatMap(({ target: dependency }) =>
        pathsBetween(dependencies, dependency, target, nextVisited).map((path) => [source, ...path]),
    );
}

function uniquePaths(paths) {
    return [...new Map(paths.map((path) => [path.join('\0'), path])).values()].sort((left, right) =>
        left.join('\0').localeCompare(right.join('\0')),
    );
}

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}
