import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { dirname, extname, join, relative, resolve } from 'node:path';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const schemaPackagesRoot = resolve(workspaceRoot, 'packages/cem_ml/schema-packages');
const runtimeMetadataPath = resolve(workspaceRoot, 'packages/cem-ml-npm/dist/cem-ml-runtime.json');
const generatedRoot = resolve(projectRoot, 'generated/feature-tour');
const operationFixturesRoot = resolve(projectRoot, 'fixtures/portable-operations');
const check = process.argv.includes('--check');
const seed = Object.freeze({
    id: 'cem-ml-feature-tour-seed',
    name: 'CEM-ML Feature Tour',
    version: '1.1.0',
    timestamp: '2026-08-22T00:00:00Z',
});

const runtime = await readJson(runtimeMetadataPath);
const browserCapability = runtime.capabilities?.browser;
const validateCapability = browserCapability?.operations?.find(({ operation }) => operation === 'validate');
if (validateCapability?.availability !== 'available') {
    throw new Error('Feature Tour generation requires the browser validate capability');
}

const manifestPaths = (await findPackageManifests(schemaPackagesRoot)).sort();
if (manifestPaths.length !== runtime.schemaPackages?.manifestCount) {
    throw new Error(
        `runtime advertises ${runtime.schemaPackages?.manifestCount} schema packages but ${manifestPaths.length} manifests were found`,
    );
}

const selected = [];
for (const manifestPath of manifestPaths) {
    const manifest = parseManifest(await readFile(manifestPath, 'utf8'));
    const example = manifest.examples.find(({ expectedResult }) => expectedResult === 'pass');
    if (!example) throw new Error(`${workspacePath(manifestPath)} has no passing example`);
    const sourcePath = resolve(dirname(manifestPath), example.path);
    const bytes = await readFile(sourcePath);
    const dependencies = await findExampleDependencies(sourcePath, bytes);
    selected.push({
        packageId: manifest.id,
        packageVersion: manifest.version,
        manifestPath,
        sourcePath,
        bytes,
        dependencies,
        ...example,
    });
}
selected.sort((left, right) => left.packageId.localeCompare(right.packageId));

const generated = new Map();
const examples = [];
const projectResources = [];
const projectEntries = [
    {
        id: 'schema-package-examples',
        kind: 'subproject',
        name: 'Schema package examples',
        description: 'One manifest-declared passing example from every browser-validatable schema package.',
        tags: ['tour', 'generated', 'schema-packages'],
    },
];
const graphNodes = [];

for (const item of selected) {
    const packageSlug = stableId(item.packageId);
    const exampleSlug = stableId(item.id);
    const extension = safeExtension(item.sourcePath);
    const resourceId = `example-${packageSlug}`;
    const runConfigResourceId = `run-${packageSlug}`;
    const logicalPath = `data/${packageSlug}/${exampleSlug}${extension}`;
    const deployedPath = `resources/${packageSlug}-${exampleSlug}${extension}`;
    const runConfigFile = `${packageSlug}.validate.json`;
    const runConfigPath = `config/${packageSlug}.validate.json`;
    const dependencies = item.dependencies.map((dependency, index) => {
        const resourceId = `dependency-${packageSlug}-${index + 1}`;
        const logicalDependencyPath = `data/${packageSlug}/${dependency.relativePath}`;
        const deployedDependencyPath = `resources/dependencies/${packageSlug}/${dependency.relativePath}`;
        return {
            ...dependency,
            resourceId,
            path: logicalDependencyPath,
            asset: `./${deployedDependencyPath}`,
            deployedPath: deployedDependencyPath,
        };
    });
    const runConfig = {
        inputs: [
            {
                uri: logicalPath,
                rootScope: {
                    defaultContentType: item.contentType,
                    schema: item.schema,
                },
            },
        ],
        outputs: [],
        schemaPackages: [],
        resolvers: [],
        scheduler: {},
    };
    const runConfigBytes = jsonBytes(runConfig);
    generated.set(`run-configs/${runConfigFile}`, runConfigBytes);

    examples.push({
        id: `${packageSlug}-${exampleSlug}`,
        packageId: item.packageId,
        packageVersion: item.packageVersion,
        exampleId: item.id,
        operation: 'validate',
        capabilityAvailability: validateCapability.availability,
        contentType: item.contentType,
        schema: item.schema,
        expectedResult: item.expectedResult,
        resourceId,
        runConfigResourceId,
        path: logicalPath,
        asset: `./${deployedPath}`,
        runConfig: `./run-configs/${runConfigFile}`,
        sha256: sha256(item.bytes),
        source: workspacePath(item.sourcePath),
        manifest: workspacePath(item.manifestPath),
        dependencies: dependencies.map((dependency) => ({
            resourceId: dependency.resourceId,
            path: dependency.relativePath,
            asset: dependency.asset,
            contentType: dependency.contentType,
            schema: dependency.schema,
            sha256: sha256(dependency.bytes),
            source: workspacePath(dependency.sourcePath),
        })),
    });
    projectEntries.push({
        id: `validate-${packageSlug}`,
        parentId: 'schema-package-examples',
        kind: 'validation',
        name: `${humanize(item.packageId)}: ${humanize(item.id)}`,
        description: `Validate ${item.id} from ${item.packageId}@${item.packageVersion}.`,
        runConfigResourceId,
        resourceIds: [resourceId, runConfigResourceId, ...dependencies.map(({ resourceId: id }) => id)],
        tags: ['tour', 'validation', packageSlug],
    });
    projectResources.push(
        {
            id: resourceId,
            role: 'data',
            sourceKind: 'project-file',
            path: logicalPath,
            contentType: item.contentType,
            schema: item.schema,
            revision: 1,
            sha256: sha256(item.bytes),
        },
        {
            id: runConfigResourceId,
            role: 'run-config',
            sourceKind: 'project-file',
            path: runConfigPath,
            contentType: 'application/json',
            schema: 'https://cem.dev/ns/cli/run-config/1',
            revision: 1,
            sha256: sha256(runConfigBytes),
        },
        ...dependencies.map((dependency) => ({
            id: dependency.resourceId,
            role: 'data',
            sourceKind: 'project-file',
            path: dependency.path,
            contentType: dependency.contentType,
            schema: dependency.schema,
            revision: 1,
            sha256: sha256(dependency.bytes),
        })),
    );
    graphNodes.push(graphCopyNode(
        `sample-${packageSlug}`,
        workspacePath(item.sourcePath, generatedRoot),
        `../../dist/static/samples/feature-tour/${deployedPath}`,
        item.contentType,
    ));
    graphNodes.push(...dependencies.map((dependency, index) => graphCopyNode(
        `sample-${packageSlug}-dependency-${index + 1}`,
        workspacePath(dependency.sourcePath, generatedRoot),
        `../../dist/static/samples/feature-tour/${dependency.deployedPath}`,
        dependency.contentType,
    )));
}

const operationFixtureDefinitions = {
    input: {
        file: 'input.cem',
        role: 'data',
        path: 'workbenches/input.cem',
        contentType: 'application/cem',
        schema: 'https://cem.dev/ns/cem-ml/1',
    },
    data: {
        file: 'data.xml',
        role: 'data',
        path: 'workbenches/data.xml',
        contentType: 'application/xml',
        schema: 'https://cem.dev/ns/data/xml/1',
    },
    template: {
        file: 'template.xpath',
        role: 'query',
        path: 'workbenches/template.xpath',
        contentType: 'application/vnd.cem.xpath',
        schema: 'https://cem.dev/ns/query/xpath/1',
    },
    graph: {
        file: 'graph.cem',
        role: 'graph',
        path: 'workbenches/graph.cem',
        contentType: 'application/vnd.cem.transform+cem',
        schema: 'https://cem.dev/ns/transform/cem/1',
    },
};
const operationFixtures = {};
for (const [id, definition] of Object.entries(operationFixtureDefinitions)) {
    const resourceId = `operation-${id}`;
    const sourcePath = resolve(operationFixturesRoot, definition.file);
    const bytes = await readFile(sourcePath);
    const deployedPath = `resources/portable-operations/${definition.file}`;
    operationFixtures[id] = { ...definition, resourceId, sourcePath, bytes, deployedPath };
    projectResources.push({
        id: resourceId,
        role: definition.role,
        sourceKind: 'project-file',
        path: definition.path,
        contentType: definition.contentType,
        schema: definition.schema,
        revision: 1,
        sha256: sha256(bytes),
    });
    graphNodes.push(graphCopyNode(
        resourceId,
        workspacePath(sourcePath, generatedRoot),
        `../../dist/static/samples/feature-tour/${deployedPath}`,
        definition.contentType,
    ));
}

const operationScenarioDefinitions = [
    {
        id: 'conversion',
        expectedFile: 'convert.json',
        operation: 'convert',
        kind: 'conversion',
        name: 'CEM-ML to DOM projection',
        primary: 'input',
        dependencies: [],
        commandArguments: [
            'convert', '$input', '--content-type', 'application/cem', '--schema',
            'https://cem.dev/ns/cem-ml/1', '--to-format', 'dom-json', '--preserve-source-offsets',
        ],
    },
    {
        id: 'query',
        operation: 'query',
        kind: 'query',
        name: 'XPath item query',
        primary: 'data',
        dependencies: [],
        commandArguments: [
            'query', '$input', '--content-type', 'application/xml', '--schema',
            'https://cem.dev/ns/data/xml/1', '--query', '//item', '--query-content-type',
            'application/vnd.cem.xpath', '--output', 'json',
        ],
    },
    {
        id: 'transformation',
        expectedFile: 'transform-direct.json',
        operation: 'transform',
        kind: 'transformation',
        name: 'Direct XPath transformation',
        primary: 'data',
        dependencies: ['template'],
        commandArguments: [
            'transform', '$input', '--data-content-type', 'application/xml', '--data-schema',
            'https://cem.dev/ns/data/xml/1', '--template', '$resource:operation-template',
            '--template-content-type', 'application/vnd.cem.xpath', '--template-schema',
            'https://cem.dev/ns/query/xpath/1', '--to-content-type',
            'application/vnd.cem.xpath-result+json', '--to-schema', 'https://cem.dev/ns/query/xpath/1',
        ],
    },
    {
        id: 'trace',
        operation: 'trace',
        kind: 'trace',
        name: 'Parser and validation trace',
        primary: 'input',
        dependencies: [],
        commandArguments: [
            'trace', '$input', '--content-type', 'application/cem', '--schema',
            'https://cem.dev/ns/cem-ml/1', '--format', 'json',
        ],
    },
    {
        id: 'transformation-graph',
        expectedFile: 'transform-graph.json',
        operation: 'transform',
        kind: 'transformation-graph',
        name: 'Transformation graph',
        primary: 'graph',
        dependencies: ['data', 'template'],
        commandArguments: ['transform', '--config', '$input'],
    },
];
projectEntries.push({
    id: 'portable-operation-workbenches',
    kind: 'subproject',
    name: 'Portable operation workbenches',
    description: 'Browser-capable CEM-ML operations executed through the shared command service.',
    tags: ['tour', 'generated', 'workbenches'],
});
const workbenches = [];
for (const scenario of operationScenarioDefinitions) {
    const primary = operationFixtures[scenario.primary];
    const dependencies = scenario.dependencies.map((id) => operationFixtures[id]);
    const expectedSourcePath = resolve(operationFixturesRoot, `expected/${scenario.expectedFile ?? `${scenario.id}.json`}`);
    const expectedBytes = await readFile(expectedSourcePath);
    const expectedResourceId = `operation-${scenario.id}-expected`;
    const expectedPath = `expected/${scenario.id}.json`;
    const expectedDeployedPath = `resources/portable-operations/${expectedPath}`;
    const runConfigResourceId = `operation-${scenario.id}-run`;
    const runConfigPath = `config/portable-operations/${scenario.id}.json`;
    const runConfigFile = `portable-${scenario.id}.json`;
    const runConfigBytes = jsonBytes({
        inputs: [{
            uri: primary.path,
            rootScope: { defaultContentType: primary.contentType, schema: primary.schema },
        }],
        outputs: [],
        schemaPackages: [],
        resolvers: [],
        scheduler: {},
    });
    generated.set(`run-configs/${runConfigFile}`, runConfigBytes);
    projectResources.push(
        {
            id: expectedResourceId,
            role: 'expected',
            sourceKind: 'project-file',
            path: expectedPath,
            contentType: 'application/json',
            schema: 'https://cem.dev/ns/data/json/1',
            revision: 1,
            sha256: sha256(expectedBytes),
        },
        {
            id: runConfigResourceId,
            role: 'run-config',
            sourceKind: 'project-file',
            path: runConfigPath,
            contentType: 'application/json',
            schema: 'https://cem.dev/ns/cli/run-config/1',
            revision: 1,
            sha256: sha256(runConfigBytes),
        },
    );
    projectEntries.push({
        id: `workbench-${scenario.id}`,
        parentId: 'portable-operation-workbenches',
        kind: scenario.kind,
        name: scenario.name,
        description: `Execute the ${scenario.id} fixture through the browser command worker.`,
        runConfigResourceId,
        resourceIds: [
            primary.resourceId,
            ...dependencies.map(({ resourceId }) => resourceId),
            expectedResourceId,
            runConfigResourceId,
        ],
        tags: ['tour', 'workbench', scenario.kind],
    });
    graphNodes.push(graphCopyNode(
        expectedResourceId,
        workspacePath(expectedSourcePath, generatedRoot),
        `../../dist/static/samples/feature-tour/${expectedDeployedPath}`,
        'application/json',
    ));
    workbenches.push({
        ...scenario,
        expectedFile: undefined,
        entryId: `workbench-${scenario.id}`,
        resourceId: primary.resourceId,
        runConfigResourceId,
        expectedResourceId,
        path: primary.path,
        asset: `./${primary.deployedPath}`,
        runConfig: `./run-configs/${runConfigFile}`,
        contentType: primary.contentType,
        schema: primary.schema,
        sha256: sha256(primary.bytes),
        expected: `./${expectedDeployedPath}`,
        expectedSha256: sha256(expectedBytes),
        dependencies: dependencies.map((dependency) => ({
            resourceId: dependency.resourceId,
            path: relative(dirname(primary.path), dependency.path).replaceAll('\\', '/'),
            asset: `./${dependency.deployedPath}`,
            contentType: dependency.contentType,
            schema: dependency.schema,
            sha256: sha256(dependency.bytes),
        })),
    });
}

const project = {
    $schema: 'https://cem.dev/ns/studio/project/1',
    schemaVersion: 1,
    id: seed.id,
    name: seed.name,
    description: `Read-only generated seed ${seed.id}@${seed.version}; create an editable copy before changes.`,
    rootUri: `studio://${seed.id}/`,
    revision: 1,
    createdAt: seed.timestamp,
    updatedAt: seed.timestamp,
    entries: projectEntries,
    resources: projectResources,
};
const projectBytes = jsonBytes(project);
generated.set('feature-tour.project.json', projectBytes);

const dependencyCount = examples.reduce((count, example) => count + example.dependencies.length, 0);
const operationCacheUrls = unique([
    ...Object.values(operationFixtures).map(({ deployedPath }) => `./${deployedPath}`),
    ...workbenches.flatMap(({ runConfigResourceId, expected }) => [
        `./run-configs/portable-${runConfigResourceId.slice('operation-'.length, -'-run'.length)}.json`,
        expected,
    ]),
]);
const cacheUrlCount = 3 + examples.length * 2 + dependencyCount + operationCacheUrls.length;
const catalog = {
    schemaVersion: 1,
    commonVersion: runtime.commonVersion,
    seed: {
        id: seed.id,
        name: seed.name,
        version: seed.version,
        project: './feature-tour.project.json',
        projectSha256: sha256(projectBytes),
    },
    capability: {
        contractVersion: browserCapability.contractVersion,
        runtime: browserCapability.runtime,
        targetIdentity: browserCapability.targetIdentity,
        abiIdentity: browserCapability.abiIdentity,
        operation: 'validate',
        availability: validateCapability.availability,
    },
    selection: 'first-manifest-declared-pass-example-per-registered-package',
    packageCount: selected.length,
    exampleCount: examples.length,
    dependencyCount,
    workbenchCount: workbenches.length,
    workbenchResourceCount: projectResources.length - (examples.length * 2 + dependencyCount),
    projectResourceCount: projectResources.length,
    cacheUrlCount,
    examples,
    workbenches,
};
generated.set('catalog.json', jsonBytes(catalog));

const sampleCacheUrls = [
    './index.json',
    './feature-tour/catalog.json',
    './feature-tour/feature-tour.project.json',
    ...examples.flatMap(({ asset, runConfig, dependencies }) => [
        `./feature-tour/${asset.slice(2)}`,
        `./feature-tour/${runConfig.slice(2)}`,
        ...dependencies.map((dependency) => `./feature-tour/${dependency.asset.slice(2)}`),
    ]),
    ...operationCacheUrls.map((path) => `./feature-tour/${path.slice(2)}`),
].sort();
const index = {
    schemaVersion: 1,
    commonVersion: runtime.commonVersion,
    cacheUrls: sampleCacheUrls,
    samples: [
        {
            id: seed.id,
            name: seed.name,
            version: seed.version,
            catalog: './feature-tour/catalog.json',
            project: './feature-tour/feature-tour.project.json',
        },
    ],
};
generated.set('index.json', jsonBytes(index));
const graph = `{run |\n${[
    graphCopyNode('sample-index', 'index.json', '../../dist/static/samples/index.json', 'application/json'),
    graphCopyNode(
        'feature-tour-catalog',
        'catalog.json',
        '../../dist/static/samples/feature-tour/catalog.json',
        'application/json',
    ),
    graphCopyNode(
        'feature-tour-project',
        'feature-tour.project.json',
        '../../dist/static/samples/feature-tour/feature-tour.project.json',
        'application/vnd.cem.studio-project+json',
    ),
    '  {import @id=feature-tour-run-configs @src="run-configs/*.json" @content-type="application/json" @opaque=true |\n'
        + '    {export @id=feature-tour-run-config-output @out="../../dist/static/samples/feature-tour/run-configs/{file}" @content-type="application/json"}\n'
        + '  }',
    ...graphNodes,
].join('\n\n')}\n}\n`;
generated.set('feature-tour.cem', Buffer.from(graph));

if (check) {
    await verifyGeneratedFiles(generated);
    console.log(`Verified generated Feature Tour: ${selected.length} schema packages and ${examples.length} examples.`);
} else {
    await rm(generatedRoot, { recursive: true, force: true });
    for (const [path, bytes] of generated) {
        const output = resolve(generatedRoot, path);
        await mkdir(dirname(output), { recursive: true });
        await writeFile(output, bytes);
    }
    console.log(`Generated Feature Tour: ${selected.length} schema packages and ${examples.length} examples.`);
}

async function verifyGeneratedFiles(expected) {
    const actualPaths = (await filesUnder(generatedRoot)).sort();
    const expectedPaths = [...expected.keys()].sort();
    if (JSON.stringify(actualPaths) !== JSON.stringify(expectedPaths)) {
        throw new Error(`generated Feature Tour file set drifted\nexpected: ${expectedPaths.join(', ')}\nactual: ${actualPaths.join(', ')}`);
    }
    for (const [path, bytes] of expected) {
        const actual = await readFile(resolve(generatedRoot, path));
        if (!actual.equals(bytes)) throw new Error(`generated Feature Tour drifted: ${path}`);
    }
}

async function findPackageManifests(root) {
    const manifests = [];
    for (const packageEntry of await readdir(root, { withFileTypes: true })) {
        if (!packageEntry.isDirectory() || packageEntry.name === 'scripts') continue;
        const candidate = resolve(root, packageEntry.name, 'v1/package.cem');
        try {
            await readFile(candidate);
            manifests.push(candidate);
        } catch (error) {
            if (error?.code !== 'ENOENT') throw error;
        }
    }
    return manifests;
}

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}

async function findExampleDependencies(sourcePath, bytes) {
    if (extname(sourcePath).toLowerCase() !== '.cem') return [];
    const exampleRoot = dirname(sourcePath);
    const dependencies = new Map();
    const pending = [{ sourcePath, bytes }];
    while (pending.length > 0) {
        const current = pending.shift();
        for (const block of findBlocksByName(current.bytes.toString('utf8'), 'schema')) {
            const reference = parseBlockHeaderAttributes(block).source;
            if (!reference || hasUriScheme(reference) || reference.startsWith('/')) continue;
            const dependencyPath = resolve(dirname(current.sourcePath), reference);
            const relativePath = relative(exampleRoot, dependencyPath).replaceAll('\\', '/');
            if (relativePath.startsWith('../') || relativePath === '..') {
                throw new Error(`${workspacePath(sourcePath)} references a dependency outside its example directory`);
            }
            if (dependencies.has(relativePath)) continue;
            const dependency = {
                relativePath,
                sourcePath: dependencyPath,
                bytes: await readFile(dependencyPath),
                contentType: 'application/vnd.cem.schema+cem',
                schema: 'https://cem.dev/ns/schema/1',
            };
            dependencies.set(relativePath, dependency);
            pending.push(dependency);
        }
    }
    return [...dependencies.values()]
        .sort((left, right) => left.relativePath.localeCompare(right.relativePath));
}

function hasUriScheme(value) {
    return /^[a-z][a-z0-9+.-]*:/i.test(value);
}

function parseManifest(source) {
    const packageBlock = findBlocksByName(source, 'package')[0];
    if (!packageBlock) throw new Error('package.cem does not contain a package block');
    const packageAttrs = parseBlockHeaderAttributes(packageBlock);
    const examples = findBlocksByName(source, 'example').map((block) => {
        const attrs = parseBlockHeaderAttributes(block);
        for (const key of ['id', 'path', 'content-type', 'schema', 'expected-result']) {
            if (!attrs[key]) throw new Error(`manifest example is missing @${key}`);
        }
        return {
            id: attrs.id,
            path: attrs.path,
            contentType: attrs['content-type'],
            schema: attrs.schema,
            expectedResult: attrs['expected-result'],
        };
    });
    return { id: packageAttrs.id, version: packageAttrs.version, examples };
}

function findBlocksByName(source, name) {
    const blocks = [];
    for (let index = 0; index < source.length; index += 1) {
        if (source[index] !== '{') continue;
        let cursor = index + 1;
        while (/\s/.test(source[cursor] ?? '')) cursor += 1;
        const nameStart = cursor;
        while (/[A-Za-z0-9_-]/.test(source[cursor] ?? '')) cursor += 1;
        if (source.slice(nameStart, cursor) !== name) continue;
        const end = findMatchingBrace(source, index);
        blocks.push(source.slice(index, end + 1));
        index = end;
    }
    return blocks;
}

function findMatchingBrace(source, start) {
    let depth = 0;
    let inString = false;
    let escaped = false;
    for (let index = start; index < source.length; index += 1) {
        const char = source[index];
        if (inString) {
            if (escaped) escaped = false;
            else if (char === '\\') escaped = true;
            else if (char === '"') inString = false;
        } else if (char === '"') inString = true;
        else if (char === '{') depth += 1;
        else if (char === '}' && --depth === 0) return index;
    }
    throw new Error(`unterminated CEM block at byte ${start}`);
}

function parseBlockHeaderAttributes(block) {
    const header = block.slice(0, block.indexOf('|') === -1 ? block.indexOf('}') : block.indexOf('|'));
    const attrs = {};
    const pattern = /@([A-Za-z0-9_-]+)\s*=\s*(?:"((?:\\.|[^"\\])*)"|([^\s|}]+))/g;
    for (const match of header.matchAll(pattern)) {
        attrs[match[1]] = match[2] === undefined ? match[3] : match[2].replace(/\\(["\\])/g, '$1');
    }
    return attrs;
}

function graphCopyNode(id, source, output, contentType) {
    return `  {import @id=${cemAttribute(id)} @src=${cemAttribute(source)} @content-type=${cemAttribute(contentType)} @opaque=true |\n`
        + `    {export @id=${cemAttribute(`${id}-output`)} @out=${cemAttribute(output)} @content-type=${cemAttribute(contentType)}}\n`
        + '  }';
}

function cemAttribute(value) {
    return `"${String(value).replaceAll('\\', '\\\\').replaceAll('"', '\\"')}"`;
}

function stableId(value) {
    const normalized = value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    if (!/^[a-z0-9]/.test(normalized) || normalized.length > 48) throw new Error(`cannot form stable id from ${value}`);
    return normalized;
}

function humanize(value) {
    return value.split(/[-_]+/).map((word) => word[0]?.toUpperCase() + word.slice(1)).join(' ');
}

function safeExtension(path) {
    const extension = extname(path).toLowerCase();
    return /^\.[a-z0-9.-]{1,24}$/.test(extension) ? extension : '.bin';
}

function jsonBytes(value) {
    return Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
}

function sha256(bytes) {
    return createHash('sha256').update(bytes).digest('hex');
}

function unique(values) {
    return [...new Set(values)];
}

function workspacePath(path, base = workspaceRoot) {
    return relative(base, path).replaceAll('\\', '/');
}

async function filesUnder(directory) {
    const files = [];
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) files.push(...(await filesUnder(path)));
        else files.push(relative(generatedRoot, path).replaceAll('\\', '/'));
    }
    return files;
}
