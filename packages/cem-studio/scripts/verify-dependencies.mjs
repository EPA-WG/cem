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
    '@epa-wg/cem-elements',
    '@epa-wg/cem-ml-cli',
    '@epa-wg/cem-theme',
]);
assert.equal(studio.dependencies['@epa-wg/cem-ml-cli'], commonVersion);
assert.equal(cli.dependencies['@epa-wg/cem-ml'], commonVersion);
assert.equal(studio.dependencies['@epa-wg/cem-ml'], undefined);
assert.equal(buildMetadata.commonVersion, commonVersion);
assert.deepEqual(buildMetadata.dependencies, studio.dependencies);

process.chdir(workspaceRoot);
const graph = await createProjectGraphAsync({ exitOnError: false });
for (const project of [
    '@epa-wg/cem-studio',
    '@epa-wg/cem-ml-cli',
    '@epa-wg/cem-ml',
    '@epa-wg/cem-components',
    'cem-elements',
    '@epa-wg/cem-theme',
]) {
    assert.ok(graph.nodes[project], `resolved Nx graph is missing ${project}`);
}

const directDependencies = new Set((graph.dependencies['@epa-wg/cem-studio'] ?? []).map(({ target }) => target));
for (const dependency of ['@epa-wg/cem-ml-cli', '@epa-wg/cem-components', 'cem-elements', '@epa-wg/cem-theme']) {
    assert.ok(directDependencies.has(dependency), `Studio Nx dependencies are missing ${dependency}`);
}
assert.equal(
    directDependencies.has('@epa-wg/cem-ml'),
    false,
    'Studio must receive @epa-wg/cem-ml transitively through the CLI package',
);

// Nx also tracks story imports and build prerequisites. Audit the published
// dependency routes separately; the installed consumer must still have exactly
// one physical runtime, even though the public component gallery also uses it.
const workspacePaths = uniquePaths(pathsBetween(graph.dependencies, '@epa-wg/cem-studio', '@epa-wg/cem-ml'));
const galleryPrefixes = [
    ['@epa-wg/cem-studio', '@epa-wg/cem-components', '@epa-wg/cem-demo-element'],
    ['@epa-wg/cem-studio', '@epa-wg/cem-components', 'cem-elements', '@epa-wg/cem-demo-element'],
    ['@epa-wg/cem-studio', '@epa-wg/custom-element', 'cem-elements', '@epa-wg/cem-demo-element'],
    ['@epa-wg/cem-studio', 'cem-elements', '@epa-wg/cem-demo-element'],
];
assert.deepEqual(workspacePaths, uniquePaths([
    ['@epa-wg/cem-studio', '@epa-wg/cem-ml-cli', '@epa-wg/cem-ml'],
    ...galleryPrefixes.flatMap(prefix => [
        [...prefix, '@epa-wg/cem-ml'], [...prefix, '@epa-wg/cem-ml-cli', '@epa-wg/cem-ml'],
    ]),
]));
const manifests = new Map(await Promise.all([...new Set(workspacePaths.flat())].map(async project => [
    project, await readJson(resolve(workspaceRoot, graph.nodes[project].data.root, 'package.json')),
])));
const publishedDependencies = Object.fromEntries([...manifests].map(([project, manifest]) => {
    const names = new Set(Object.keys({ ...manifest.dependencies, ...manifest.optionalDependencies, ...manifest.peerDependencies }));
    return [project, (graph.dependencies[project] ?? []).filter(({ target }) => names.has(manifests.get(target)?.name))];
}));
const runtimePaths = uniquePaths(pathsBetween(publishedDependencies, '@epa-wg/cem-studio', '@epa-wg/cem-ml'));
assert.deepEqual(runtimePaths, uniquePaths([
    ['@epa-wg/cem-studio', '@epa-wg/cem-ml-cli', '@epa-wg/cem-ml'],
    ['@epa-wg/cem-studio', '@epa-wg/cem-components', '@epa-wg/cem-demo-element', '@epa-wg/cem-ml'],
    ['@epa-wg/cem-studio', '@epa-wg/cem-components', '@epa-wg/cem-demo-element', '@epa-wg/cem-ml-cli', '@epa-wg/cem-ml'],
]));

const report = {
    schemaVersion: 1,
    project: studio.name,
    commonVersion,
    exactCliDependency: studio.dependencies['@epa-wg/cem-ml-cli'],
    runtimePaths,
    workspacePaths,
    directDependencies: [...directDependencies].sort(),
};
await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`Verified ${studio.name}@${commonVersion}: exact CLI dependency and ${runtimePaths.length} published runtime paths (${workspacePaths.length} workspace paths).`);

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
