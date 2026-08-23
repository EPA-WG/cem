import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';
import { createProjectGraphAsync } from '@nx/devkit';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const projectRoot = resolve(workspaceRoot, 'apps/cem-site');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-site');
const manifest = JSON.parse(await readFile(resolve(projectRoot, 'site.routes.json'), 'utf8'));
const publicationGraph = await readFile(resolve(projectRoot, 'site.cem'), 'utf8');
const reportText = await readFile(join(outputRoot, 'site.report.json'), 'utf8');
const projectGraph = await createProjectGraphAsync({ exitOnError: false });
const siteProject = projectGraph.nodes['cem-site']?.data;
const siteBuildTarget = siteProject?.targets?.build;

if (!siteProject || !siteBuildTarget) {
    throw new Error('the resolved Nx graph does not contain cem-site:build');
}

if (manifest.version !== 1 || !Array.isArray(manifest.entries)) {
    throw new Error('site.routes.json must declare version 1 and an entries array');
}

const requiredExclusions = [
    'docs/archive/**',
    'docs/todo.md',
    'roadmap.md',
    '**/*.tmp.md',
    '**/figma/**',
    '**/cem.tokens.intermediate.json',
    '**/cem.tokens.resolved.json',
].sort();
if (JSON.stringify([...manifest.exclusions].sort()) !== JSON.stringify(requiredExclusions)) {
    throw new Error('site publication exclusions drifted from the accepted boundary');
}

const forbiddenSources = [
    /^docs\/archive\//,
    /^docs\/todo\.md$/,
    /^roadmap\.md$/,
    /\.tmp\.md$/,
    /(^|\/)figma(\/|$)/,
    /cem\.tokens\.(intermediate|resolved)\.json$/,
];
const entriesByRoute = new Map();
const entriesByOutput = new Map();
const importIds = new Set();
const exportIds = new Set();
const ownersByRoute = new Map();
const canonicalSourceText = new Map();
const evidenceSourceText = new Map();
const buildInputs = new Set(siteBuildTarget.inputs);
const allowedContentRoles = new Set([
    'landing',
    'guide-index',
    'package-index',
    'guide',
    'authored-reference',
    'generated-reference',
    'catalog',
    'parity-comparison',
    'example-index',
    'interactive-example',
    'search',
    'release-notes',
]);
const allowedRelativeLinkPolicies = new Set(['none', 'site-routes', 'canonical-source']);
const buildDependencies = new Set(
    siteBuildTarget.dependsOn
        .filter((dependency) => typeof dependency === 'object')
        .map((dependency) => `${dependency.projects[0]}:${dependency.target}`),
);

function markdownLinks(source) {
    return [...source.matchAll(/\]\(([^)]+)\)/g)].map((match) => match[1].trim());
}

function isRelativeRepositoryLink(href) {
    return !href.startsWith('#') && !href.startsWith('/') && !/^[a-z][a-z0-9+.-]*:/i.test(href);
}

function normalizedHtmlText(source) {
    return source
        .replace(/<[^>]*>/g, ' ')
        .replaceAll('&amp;', '&')
        .replaceAll('&lt;', '<')
        .replaceAll('&gt;', '>')
        .replaceAll('&quot;', '"')
        .replaceAll('&#39;', "'")
        .replace(/\s+/g, ' ')
        .trim();
}

function expectedCanonicalSourceBase(source) {
    const separator = source.lastIndexOf('/');
    const directory = separator === -1 ? '' : source.slice(0, separator + 1);
    return `https://github.com/EPA-WG/cem/blob/develop/${directory}`;
}

function canonicalOwner(source) {
    const candidates = Object.values(projectGraph.nodes)
        .map((node) => ({
            name: node.name,
            root: node.data.root.replaceAll('\\', '/').replace(/^\.\/$/, '.'),
        }))
        .filter(({ root }) => (root === '.' ? true : source === root || source.startsWith(`${root}/`)))
        .sort((left, right) => right.root.length - left.root.length);
    if (candidates.length === 0) {
        throw new Error(`${source} has no owning Nx project root`);
    }
    const deepest = candidates[0].root.length;
    const owners = candidates.filter(({ root }) => root.length === deepest);
    if (owners.length !== 1) {
        throw new Error(`${source} has ambiguous Nx owners: ${owners.map(({ name }) => name).join(', ')}`);
    }
    return owners[0];
}

async function loadRuntimeContract(runtime, outputs) {
    if (
        typeof runtime?.id !== 'string' ||
        runtime?.schema !== 'https://cem.dev/ns/data/module-map/2' ||
        typeof runtime.sourceMap !== 'string' ||
        typeof runtime.destinationMap !== 'string' ||
        typeof runtime.entrySpecifier !== 'string' ||
        !Array.isArray(runtime.routes) ||
        runtime.routes.length === 0 ||
        new Set(runtime.routes).size !== runtime.routes.length
    ) {
        throw new Error('each site runtime must declare one exact module-map v2 contract');
    }
    const sourceMapPath = resolve(workspaceRoot, runtime.sourceMap);
    const destinationMapPath = resolve(workspaceRoot, runtime.destinationMap);
    const [sourceMap, destinationMap] = await Promise.all([
        readFile(sourceMapPath, 'utf8').then(JSON.parse),
        readFile(destinationMapPath, 'utf8').then(JSON.parse),
    ]);
    for (const [label, moduleMap] of [
        ['source', sourceMap],
        ['destination', destinationMap],
    ]) {
        if (
            moduleMap.$schema !== runtime.schema ||
            !moduleMap.imports ||
            Array.isArray(moduleMap.imports) ||
            !moduleMap.resources ||
            Array.isArray(moduleMap.resources)
        ) {
            throw new Error(`${label} runtime module map does not implement schema v2`);
        }
    }
    const sourceImportKeys = Object.keys(sourceMap.imports).sort();
    const destinationImportKeys = Object.keys(destinationMap.imports).sort();
    const sourceResourceKeys = Object.keys(sourceMap.resources).sort();
    const destinationResourceKeys = Object.keys(destinationMap.resources).sort();
    if (
        JSON.stringify(sourceImportKeys) !== JSON.stringify(destinationImportKeys) ||
        JSON.stringify(sourceResourceKeys) !== JSON.stringify(destinationResourceKeys) ||
        sourceImportKeys.some((key) => sourceResourceKeys.includes(key)) ||
        !sourceImportKeys.includes(runtime.entrySpecifier)
    ) {
        throw new Error(`${runtime.id} runtime module-map source/destination identities drifted`);
    }

    const declarations = [
        ...sourceImportKeys.map((specifier) => ({
            specifier,
            source: sourceMap.imports[specifier],
            target: destinationMap.imports[specifier],
            contentType: 'text/javascript',
        })),
        ...sourceResourceKeys.map((specifier) => {
            const source = sourceMap.resources[specifier];
            const destination = destinationMap.resources[specifier];
            if (
                !source ||
                !destination ||
                Object.keys(source).sort().join(',') !== 'contentType,path' ||
                Object.keys(destination).sort().join(',') !== 'contentType,path' ||
                source.contentType !== destination.contentType
            ) {
                throw new Error(`runtime resource ${specifier} is not an exact paired declaration`);
            }
            return {
                specifier,
                source: source.path,
                target: destination.path,
                contentType: source.contentType,
            };
        }),
    ];
    if (runtime.id === 'interactive') {
        if (
            declarations.filter(({ contentType }) => contentType === 'application/wasm').length !== 1 ||
            declarations.filter(({ contentType }) => contentType === 'text/css').length !== 2 ||
            !declarations.some(({ specifier }) => specifier.endsWith('/processing-worker'))
        ) {
            throw new Error('interactive runtime must declare one WASM, two stylesheets, and its worker');
        }
    } else if (
        runtime.id === 'search' &&
        (!sourceImportKeys.includes('@epa-wg/cem-site/components-runtime') ||
            !sourceImportKeys.includes('@epa-wg/custom-element') ||
            !sourceImportKeys.includes('@epa-wg/cem-components/primitives'))
    ) {
        throw new Error('search runtime must declare the production CEM component stack');
    }

    const upstreamByOwner = new Map([
        ['@epa-wg/custom-element', '@epa-wg/custom-element:build'],
        ['@epa-wg/cem-components', '@epa-wg/cem-components:build'],
        ['@epa-wg/cem-theme', '@epa-wg/cem-theme:build:tokens'],
    ]);
    const assets = [];
    for (const declaration of declarations) {
        if (
            typeof declaration.source !== 'string' ||
            typeof declaration.target !== 'string' ||
            !declaration.target.startsWith('./')
        ) {
            throw new Error(`runtime declaration ${declaration.specifier} has invalid paths`);
        }
        const sourcePath = resolve(dirname(sourceMapPath), declaration.source);
        const source = relative(workspaceRoot, sourcePath).replaceAll('\\', '/');
        if (source.startsWith('../')) {
            throw new Error(`runtime declaration ${declaration.specifier} escapes the workspace`);
        }
        const owner = canonicalOwner(source);
        if (owner.name !== 'cem-site') {
            const upstreamTarget = upstreamByOwner.get(owner.name);
            if (!upstreamTarget || !buildDependencies.has(upstreamTarget)) {
                throw new Error(
                    `runtime declaration ${declaration.specifier} does not schedule ${owner.name}'s production output`,
                );
            }
        }
        for (const route of runtime.routes) {
            if (!route.startsWith('/') || !route.endsWith('/')) {
                throw new Error(`${runtime.id} runtime route ${route} is not canonical`);
            }
            const output = new URL(declaration.target, `https://cem.invalid${route}`).pathname.slice(1);
            if (outputs.has(output)) {
                throw new Error(`runtime declarations collide at ${output}`);
            }
            outputs.add(output);
            assets.push({ ...declaration, source, sourcePath, route, output });
        }
    }
    return {
        runtime,
        assets,
        assetOutputs: new Set(assets.map(({ output }) => output)),
        declarationCount: declarations.length,
        sourceMap,
        destinationMap,
    };
}

if (!Array.isArray(manifest.runtimes) || manifest.runtimes.length === 0) {
    throw new Error('site.routes.json must declare a non-empty runtimes array');
}
const runtimeIds = manifest.runtimes.map(({ id }) => id);
if (
    new Set(runtimeIds).size !== runtimeIds.length ||
    JSON.stringify([...runtimeIds].sort()) !== JSON.stringify(['interactive', 'search'])
) {
    throw new Error('site runtimes must declare unique interactive and search contracts');
}
const runtimeOutputs = new Set();
const runtimeContracts = [];
for (const runtime of manifest.runtimes) {
    runtimeContracts.push(await loadRuntimeContract(runtime, runtimeOutputs));
}
const runtimeById = new Map(runtimeContracts.map((contract) => [contract.runtime.id, contract]));
const interactiveRuntimeContract = runtimeById.get('interactive');
const searchRuntimeContract = runtimeById.get('search');
const runtimeContract = {
    assets: runtimeContracts.flatMap(({ assets }) => assets),
    assetOutputs: runtimeOutputs,
};
if (
    JSON.stringify(JSON.parse(manifest.searchImportMap)) !==
    JSON.stringify({ imports: searchRuntimeContract.sourceMap.imports })
) {
    throw new Error('search import-map placeholder drifted from its source module map');
}

async function loadDeclaredSources(entry, field, destination) {
    if (entry[field] === undefined) {
        return;
    }
    if (!Array.isArray(entry[field]) || entry[field].length === 0) {
        throw new Error(`${entry.route} ${field} must be a non-empty array`);
    }
    if (new Set(entry[field]).size !== entry[field].length) {
        throw new Error(`${entry.route} has duplicate ${field}`);
    }
    for (const source of entry[field]) {
        if (
            typeof source !== 'string' ||
            /[*?[]/.test(source) ||
            forbiddenSources.some((pattern) => pattern.test(source))
        ) {
            throw new Error(`${entry.route} has invalid ${field} entry ${source}`);
        }
        destination.set(source, await readFile(resolve(workspaceRoot, source), 'utf8'));
    }
}

for (const entry of manifest.entries) {
    if (!['page', 'resource'].includes(entry.kind)) {
        throw new Error(`unsupported site entry kind: ${entry.kind}`);
    }
    if (!entry.route.startsWith('/') || !entry.output || !entry.source) {
        throw new Error(`invalid site entry: ${JSON.stringify(entry)}`);
    }
    const expectedOutput =
        entry.kind === 'page'
            ? entry.route === '/'
                ? 'index.html'
                : `${entry.route.slice(1)}index.html`
            : entry.route.slice(1);
    if (entry.output !== expectedOutput) {
        throw new Error(`${entry.route} must publish to ${expectedOutput}, got ${entry.output}`);
    }
    if (entriesByRoute.has(entry.route) || entriesByOutput.has(entry.output)) {
        throw new Error(`duplicate site route or output: ${entry.route}`);
    }
    if (importIds.has(entry.importId) || exportIds.has(entry.exportId)) {
        throw new Error(`duplicate graph identity for ${entry.route}`);
    }
    if (forbiddenSources.some((pattern) => pattern.test(entry.source))) {
        throw new Error(`${entry.route} publishes excluded source ${entry.source}`);
    }
    if (!allowedContentRoles.has(entry.contentRole)) {
        throw new Error(`${entry.route} has unknown content role ${entry.contentRole}`);
    }
    if (!allowedRelativeLinkPolicies.has(entry.relativeLinkPolicy)) {
        throw new Error(`${entry.route} has unknown relative-link policy ${entry.relativeLinkPolicy}`);
    }
    if (
        entry.route.startsWith('/reference/') &&
        !['authored-reference', 'generated-reference'].includes(entry.contentRole)
    ) {
        throw new Error(`${entry.route} does not declare an explicit reference role`);
    }
    if (entry.contentRole === 'authored-reference' && entry.sourceKind !== 'authored') {
        throw new Error(`${entry.route} presents generated content as authored reference`);
    }
    if (entry.contentRole === 'generated-reference' && entry.sourceKind !== 'generated') {
        throw new Error(`${entry.route} presents authored content as generated reference`);
    }
    if (entry.route.startsWith('/examples/') && !['example-index', 'interactive-example'].includes(entry.contentRole)) {
        throw new Error(`${entry.route} does not declare an explicit example role`);
    }
    if (entry.route.startsWith('/releases/') && entry.contentRole !== 'release-notes') {
        throw new Error(`${entry.route} does not declare the release-notes role`);
    }
    await loadDeclaredSources(entry, 'canonicalSources', canonicalSourceText);
    await loadDeclaredSources(entry, 'evidenceSources', evidenceSourceText);
    if (!entry.owner) {
        throw new Error(`${entry.route} has no canonical Nx owner`);
    }
    const resolvedOwner = canonicalOwner(entry.source);
    if (entry.owner !== resolvedOwner.name) {
        throw new Error(
            `${entry.route} declares owner ${entry.owner}, but ${entry.source} belongs to ${resolvedOwner.name}`,
        );
    }
    ownersByRoute.set(entry.route, resolvedOwner);
    if (entry.validationTarget !== undefined) {
        const ownerTargetPrefix = `${entry.owner}:`;
        if (
            typeof entry.validationTarget !== 'string' ||
            !entry.validationTarget.startsWith(ownerTargetPrefix) ||
            !buildDependencies.has(entry.validationTarget)
        ) {
            throw new Error(`${entry.route} does not schedule an owner-scoped validation target`);
        }
        const targetName = entry.validationTarget.slice(ownerTargetPrefix.length);
        if (!projectGraph.nodes[entry.owner].data.targets?.[targetName]) {
            throw new Error(`${entry.route} validation target ${entry.validationTarget} is absent from the Nx graph`);
        }
    }
    if (entry.sourceKind === 'generated') {
        if (!entry.upstreamTarget || !buildDependencies.has(entry.upstreamTarget)) {
            throw new Error(`${entry.route} does not schedule ${entry.upstreamTarget}`);
        }
        const ownerTargetPrefix = `${entry.owner}:`;
        if (!entry.upstreamTarget.startsWith(ownerTargetPrefix)) {
            throw new Error(`${entry.route} upstream target ${entry.upstreamTarget} is not owned by ${entry.owner}`);
        }
        const targetName = entry.upstreamTarget.slice(ownerTargetPrefix.length);
        if (!projectGraph.nodes[entry.owner].data.targets?.[targetName]) {
            throw new Error(
                `${entry.route} upstream target ${entry.upstreamTarget} is absent from the resolved Nx graph`,
            );
        }
    } else if (entry.sourceKind === 'authored') {
        if (entry.upstreamTarget !== null) {
            throw new Error(`${entry.route} gives an authored source a generation target`);
        }
        if (!entry.source.startsWith('apps/cem-site/') && !buildInputs.has(`{workspaceRoot}/${entry.source}`)) {
            throw new Error(`${entry.route} source is absent from the Nx build hash`);
        }
    } else {
        throw new Error(`${entry.route} has unknown source kind ${entry.sourceKind}`);
    }

    const sourceText = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
    const relativeSourceLinks = markdownLinks(sourceText).filter(isRelativeRepositoryLink);
    if (entry.relativeLinkPolicy === 'canonical-source') {
        const expectedBase = expectedCanonicalSourceBase(entry.source);
        if (
            entry.sourceKind !== 'authored' ||
            relativeSourceLinks.length === 0 ||
            entry.canonicalSourceBase !== expectedBase
        ) {
            throw new Error(
                `${entry.route} canonical-source policy must map authored relative links to ${expectedBase}`,
            );
        }
    } else {
        if (entry.canonicalSourceBase !== undefined) {
            throw new Error(`${entry.route} declares an unused canonicalSourceBase`);
        }
        if (entry.relativeLinkPolicy === 'none' && relativeSourceLinks.length !== 0) {
            throw new Error(`${entry.route} leaves repository-relative links without a policy`);
        }
        if (entry.relativeLinkPolicy === 'site-routes' && !entry.source.startsWith('apps/cem-site/')) {
            throw new Error(`${entry.route} applies site-route policy outside site-owned content`);
        }
    }
    const graphSource = relative(projectRoot, resolve(workspaceRoot, entry.source)).replaceAll('\\', '/');
    const graphOutput = `../../dist/apps/cem-site/${entry.output}`;
    for (const token of [
        `@id=${entry.importId}`,
        `@src="${graphSource}"`,
        `@id=${entry.exportId}`,
        `@out="${graphOutput}"`,
    ]) {
        if (!publicationGraph.includes(token)) {
            throw new Error(`${entry.route} is missing graph token ${token}`);
        }
    }
    if (entry.relativeLinkPolicy === 'canonical-source') {
        for (const token of ['@name="canonicalSourceBase"', `@value="${entry.canonicalSourceBase}"`]) {
            if (!publicationGraph.includes(token)) {
                throw new Error(`${entry.route} is missing canonical-source graph token ${token}`);
            }
        }
    }
    if (!reportText.includes(`${entry.importId}:import`) || !reportText.includes(entry.output)) {
        throw new Error(`${entry.route} is missing from the transform report`);
    }

    entriesByRoute.set(entry.route, entry);
    entriesByOutput.set(entry.output, entry);
    importIds.add(entry.importId);
    exportIds.add(entry.exportId);
}

if (!Array.isArray(manifest.publicSurfaces) || manifest.publicSurfaces.length === 0) {
    throw new Error('site.routes.json must declare public package and crate surfaces');
}

const requiredInventoryInputs = [
    '{workspaceRoot}/Cargo.toml',
    '{workspaceRoot}/packages/**/Cargo.toml',
    '{workspaceRoot}/packages/*/package.json',
    '{workspaceRoot}/packages/*/README.md',
];
for (const input of requiredInventoryInputs) {
    if (!buildInputs.has(input)) {
        throw new Error(`public-surface inventory input is absent from the Nx build hash: ${input}`);
    }
}

const expectedPublicSurfaces = [];
const packageDirectories = (await readdir(resolve(workspaceRoot, 'packages'), { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
for (const directory of packageDirectories) {
    const source = `packages/${directory}/package.json`;
    let packageManifest;
    try {
        packageManifest = JSON.parse(await readFile(resolve(workspaceRoot, source), 'utf8'));
    } catch (error) {
        if (error.code === 'ENOENT') {
            continue;
        }
        throw error;
    }
    if (packageManifest.private !== true) {
        expectedPublicSurfaces.push({
            kind: 'npm',
            id: packageManifest.name,
            manifest: source,
            nxProject: canonicalOwner(source).name,
        });
    }
}

const workspaceCargo = await readFile(resolve(workspaceRoot, 'Cargo.toml'), 'utf8');
const workspaceMembers = workspaceCargo.match(/members\s*=\s*\[([\s\S]*?)\]/)?.[1];
if (!workspaceMembers) {
    throw new Error('the Cargo workspace has no explicit members inventory');
}
for (const match of workspaceMembers.matchAll(/['"]([^'"]+)['"]/g)) {
    const source = `${match[1]}/Cargo.toml`;
    const cargoManifest = await readFile(resolve(workspaceRoot, source), 'utf8');
    const packageSection = cargoManifest.match(/^\[package\]\s*([\s\S]*?)(?=^\[|(?![\s\S]))/m)?.[1];
    if (!packageSection || !/^publish\s*=\s*true\s*$/m.test(packageSection)) {
        continue;
    }
    const name = packageSection.match(/^name\s*=\s*['"]([^'"]+)['"]\s*$/m)?.[1];
    if (!name) {
        throw new Error(`${source} has no Cargo package name`);
    }
    expectedPublicSurfaces.push({
        kind: 'cargo',
        id: name,
        manifest: source,
        nxProject: canonicalOwner(source).name,
    });
}

const publicSurfaceKeys = new Set();
const publicSurfaceManifests = new Set();
for (const surface of manifest.publicSurfaces) {
    const key = `${surface.kind}:${surface.id}`;
    if (
        !['npm', 'cargo'].includes(surface.kind) ||
        typeof surface.id !== 'string' ||
        !surface.id ||
        typeof surface.manifest !== 'string' ||
        !surface.manifest ||
        typeof surface.nxProject !== 'string' ||
        !surface.nxProject ||
        typeof surface.route !== 'string' ||
        !surface.route ||
        typeof surface.summary !== 'string' ||
        !surface.summary.trim()
    ) {
        throw new Error(`incomplete public-surface declaration: ${JSON.stringify(surface)}`);
    }
    if (publicSurfaceKeys.has(key) || publicSurfaceManifests.has(surface.manifest)) {
        throw new Error(`duplicate public-surface declaration: ${key}`);
    }
    const route = entriesByRoute.get(surface.route);
    if (!route || route.kind !== 'page') {
        throw new Error(`${key} has no published documentation route ${surface.route}`);
    }
    if (route.owner !== surface.nxProject && surface.route !== '/packages/') {
        throw new Error(`${key} route ${surface.route} is not owned by ${surface.nxProject}`);
    }
    if (canonicalOwner(surface.manifest).name !== surface.nxProject) {
        throw new Error(`${key} manifest is not owned by ${surface.nxProject}`);
    }
    publicSurfaceKeys.add(key);
    publicSurfaceManifests.add(surface.manifest);
}

const comparablePublicSurface = ({ kind, id, manifest: source, nxProject }) => ({
    kind,
    id,
    manifest: source,
    nxProject,
});
const sortPublicSurfaces = (surfaces) =>
    surfaces
        .map(comparablePublicSurface)
        .sort((left, right) => `${left.kind}:${left.id}`.localeCompare(`${right.kind}:${right.id}`));
if (
    JSON.stringify(sortPublicSurfaces(manifest.publicSurfaces)) !==
    JSON.stringify(sortPublicSurfaces(expectedPublicSurfaces))
) {
    throw new Error('public package/crate documentation drifted from publication metadata');
}

if (!Array.isArray(manifest.searchDocuments) || manifest.searchDocuments.length === 0) {
    throw new Error('site.routes.json must declare searchable route documents');
}
const searchableRoutes = manifest.entries
    .filter(({ kind, contentRole }) => kind === 'page' && contentRole !== 'search')
    .map(({ route }) => route)
    .sort();
const searchDocumentRoutes = manifest.searchDocuments.map(({ route }) => route).sort();
if (
    new Set(searchDocumentRoutes).size !== searchDocumentRoutes.length ||
    JSON.stringify(searchDocumentRoutes) !== JSON.stringify(searchableRoutes)
) {
    throw new Error('search documents must cover every non-search page route exactly once');
}
for (const document of manifest.searchDocuments) {
    if (
        typeof document.title !== 'string' ||
        !document.title.trim() ||
        typeof document.summary !== 'string' ||
        !document.summary.trim() ||
        !Array.isArray(document.headings) ||
        document.headings.length === 0
    ) {
        throw new Error(`search document ${document.route} is incomplete`);
    }
    const headingIds = document.headings.map(({ id }) => id);
    if (new Set(headingIds).size !== headingIds.length) {
        throw new Error(`search document ${document.route} has duplicate heading fragments`);
    }
    for (const heading of document.headings) {
        if (
            typeof heading.id !== 'string' ||
            !heading.id ||
            !Number.isInteger(heading.level) ||
            heading.level < 1 ||
            heading.level > 6 ||
            typeof heading.text !== 'string' ||
            !heading.text.trim()
        ) {
            throw new Error(`search document ${document.route} has an invalid heading`);
        }
    }
}

async function filesUnder(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    const files = [];
    for (const entry of entries) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) {
            files.push(...(await filesUnder(path)));
        } else {
            files.push(relative(outputRoot, path).replaceAll('\\', '/'));
        }
    }
    return files.sort();
}

const expectedFiles = [
    ...manifest.entries.flatMap((entry) => [entry.output, `${entry.output}.map`]),
    ...runtimeContract.assets.map(({ output }) => output),
    'site.report.json',
].sort();
const actualFiles = await filesUnder(outputRoot);
if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
    throw new Error(
        `CEM Site output is not clean.\nExpected: ${expectedFiles.join(', ')}\nActual: ${actualFiles.join(', ')}`,
    );
}

const idsByRoute = new Map();
const headingsByRoute = new Map();
for (const entry of manifest.entries.filter(({ kind }) => kind === 'page')) {
    const output = await readFile(join(outputRoot, entry.output), 'utf8');
    const ids = [...output.matchAll(/\sid="([^"]+)"/g)].map((match) => match[1]);
    if (new Set(ids).size !== ids.length) {
        throw new Error(`${entry.output} has duplicate fragment identifiers`);
    }
    idsByRoute.set(entry.route, new Set(ids));
    const headings = [...output.matchAll(/<h([1-6])([^>]*)>([\s\S]*?)<\/h\1>/g)].map((match) => {
        const id = match[2].match(/\sid="([^"]+)"/)?.[1];
        if (!id) {
            throw new Error(`${entry.output} has a heading without a stable fragment identifier`);
        }
        return { id, level: Number(match[1]), text: normalizedHtmlText(match[3]) };
    });
    headingsByRoute.set(entry.route, new Map(headings.map((heading) => [heading.id, heading])));
}
for (const document of manifest.searchDocuments) {
    const renderedHeadings = headingsByRoute.get(document.route);
    for (const heading of document.headings) {
        const rendered = renderedHeadings.get(heading.id);
        if (!rendered || rendered.level !== heading.level || rendered.text !== heading.text) {
            throw new Error(`${document.route} search heading ${heading.id} drifted from rendered HTML`);
        }
    }
}

const verification = {
    entries: [],
    exclusions: manifest.exclusions,
    publicSurfaces: {
        total: manifest.publicSurfaces.length,
        npm: manifest.publicSurfaces.filter(({ kind }) => kind === 'npm').length,
        cargo: manifest.publicSurfaces.filter(({ kind }) => kind === 'cargo').length,
        surfaces: manifest.publicSurfaces,
    },
    report: 'site.report.json',
};
for (const entry of manifest.entries) {
    const output = await readFile(join(outputRoot, entry.output), 'utf8');
    const sourceMapText = await readFile(join(outputRoot, `${entry.output}.map`), 'utf8');
    const sourceMap = JSON.parse(sourceMapText);

    if (entry.kind === 'page') {
        if (!output.includes('<nav aria-label="Primary">')) {
            throw new Error(`${entry.output} does not contain the shared primary navigation`);
        }
        if (output.includes('&lt;h1') || /<(?:script|link)\b[^>]*(?:src|href)="[^"]*node_modules/i.test(output)) {
            throw new Error(`${entry.output} contains an escaped HTML bridge or source-only runtime asset path`);
        }
        if (!Array.isArray(sourceMap.outputSpans) || sourceMap.outputSpans.length === 0) {
            throw new Error(`${entry.output}.map has no native output spans`);
        }
        if (!sourceMapText.includes('InterpreterRender')) {
            throw new Error(`${entry.output}.map does not retain CEMT render provenance`);
        }

        const links = [...output.matchAll(/href="([^"]+)"/g)].map((match) => match[1]);
        if (entry.relativeLinkPolicy === 'canonical-source') {
            const source = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
            for (const href of markdownLinks(source).filter(isRelativeRepositoryLink)) {
                const rewritten = `${entry.canonicalSourceBase}${href}`;
                if (!links.includes(rewritten)) {
                    throw new Error(`${entry.output} does not canonically rewrite ${href}`);
                }
            }
        }
        for (const href of links) {
            const target = new URL(href, `https://cem.invalid${entry.route}`);
            if (
                target.origin === 'https://cem.invalid' &&
                !entriesByRoute.has(target.pathname) &&
                !runtimeContract.assetOutputs.has(target.pathname.slice(1))
            ) {
                throw new Error(`${entry.output} links to unpublished route ${target.pathname}`);
            }
            if (target.origin === 'https://cem.invalid' && target.hash) {
                const fragment = decodeURIComponent(target.hash.slice(1));
                if (!idsByRoute.get(target.pathname)?.has(fragment)) {
                    throw new Error(`${entry.output} links to missing fragment ${target.pathname}#${fragment}`);
                }
            }
        }
        const verificationEntry = {
            route: entry.route,
            kind: entry.kind,
            contentRole: entry.contentRole,
            relativeLinkPolicy: entry.relativeLinkPolicy,
            owner: entry.owner,
            ownerRoot: ownersByRoute.get(entry.route).root,
            upstreamTarget: entry.upstreamTarget,
            validationTarget: entry.validationTarget ?? null,
            links,
            outputSpans: sourceMap.outputSpans.length,
        };

        if (entry.contentRole === 'package-index') {
            if (
                entry.route !== '/packages/' ||
                entry.source !== 'apps/cem-site/site.routes.json' ||
                output.includes('<script')
            ) {
                throw new Error('the public package index must be a static projection of the route manifest');
            }
            const renderedSurfaces = [...output.matchAll(/<li\b([^>]*)>/g)]
                .map(([, attributes]) => ({
                    kind: attributes.match(/\bdata-public-kind="([^"]+)"/)?.[1],
                    id: attributes.match(/\bdata-public-id="([^"]+)"/)?.[1],
                }))
                .filter(({ kind, id }) => kind && id)
                .map(({ kind, id }) => `${kind}:${id}`);
            const declaredSurfaces = manifest.publicSurfaces.map(({ kind, id }) => `${kind}:${id}`);
            if (JSON.stringify(renderedSurfaces) !== JSON.stringify(declaredSurfaces)) {
                throw new Error('the public package index does not render every declared surface exactly once');
            }
            const renderedText = normalizedHtmlText(output);
            for (const surface of manifest.publicSurfaces) {
                if (!output.includes(`href="${surface.route}"`) || !renderedText.includes(surface.summary)) {
                    throw new Error(`the public package index does not explain ${surface.kind}:${surface.id}`);
                }
            }
            Object.assign(verificationEntry, {
                publicSurfaceCount: manifest.publicSurfaces.length,
                npmPackageCount: manifest.publicSurfaces.filter(({ kind }) => kind === 'npm').length,
                cargoCrateCount: manifest.publicSurfaces.filter(({ kind }) => kind === 'cargo').length,
                javascript: false,
            });
        }

        if (entry.route === '/tokens/') {
            if (entry.source !== 'packages/cem-theme/dist/lib/tokens/cem.tokens.catalog.json') {
                throw new Error('the token browser must consume the public theme token catalog');
            }
            if (output.includes('<script')) {
                throw new Error('the static token browser must not load JavaScript');
            }

            const catalog = JSON.parse(await readFile(resolve(workspaceRoot, entry.source), 'utf8'));
            if (!Array.isArray(catalog.tokens) || catalog.tokens.length === 0) {
                throw new Error('the public theme token catalog has no tokens');
            }
            const tokenNames = new Set(catalog.tokens.map((token) => token.name));
            if (tokenNames.size !== catalog.tokens.length) {
                throw new Error('the public theme token catalog has duplicate token names');
            }
            const renderedRows = [...output.matchAll(/data-token-name="([^"]+)"/g)];
            if (renderedRows.length !== catalog.tokens.length) {
                throw new Error(
                    `token browser rendered ${renderedRows.length} of ${catalog.tokens.length} catalog records`,
                );
            }
            for (const token of catalog.tokens) {
                const canonicalSource = entry.canonicalSources.find((source) => source.endsWith(`/${token.spec}.md`));
                if (!canonicalSource) {
                    throw new Error(`${token.name} has undeclared canonical spec ${token.spec}`);
                }
                if (!canonicalSourceText.get(canonicalSource).includes(`###### ${token.sourceTable}`)) {
                    throw new Error(`${token.name} has unknown source table ${token.spec}#${token.sourceTable}`);
                }
                if (!output.includes(`data-token-name="${token.name}"`)) {
                    throw new Error(`${token.name} is absent from the rendered token browser`);
                }
            }

            const canonicalSpecs = entry.canonicalSources.map((source) =>
                source.slice(source.lastIndexOf('/') + 1, -3),
            );
            if (JSON.stringify(catalog.$generated?.sourceSpecs) !== JSON.stringify(canonicalSpecs)) {
                throw new Error('token catalog source specs drifted from canonicalSources');
            }
            const buckets = [...new Set(catalog.tokens.map((token) => token.bucket))].sort();
            if (JSON.stringify(buckets) !== JSON.stringify(['visual', 'voice'])) {
                throw new Error(`token catalog bucket coverage drifted: ${buckets.join(', ')}`);
            }

            Object.assign(verificationEntry, {
                canonicalSources: entry.canonicalSources,
                tokenCount: catalog.tokens.length,
                buckets,
                javascript: false,
            });
        }

        if (entry.route === '/components/') {
            if (entry.source !== 'packages/cem-components/dist/catalog/cem.components.catalog.json') {
                throw new Error('the component gallery must consume the public component catalog');
            }
            if (output.includes('<script')) {
                throw new Error('the static component gallery must not load JavaScript');
            }

            const catalogText = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
            const catalog = JSON.parse(catalogText);
            if (/figma/i.test(catalogText)) {
                throw new Error('the Phase 6 component catalog must not consume Figma projections');
            }
            if (!Array.isArray(catalog.components) || catalog.components.length === 0) {
                throw new Error('the public component catalog has no components');
            }
            if (catalog.components.some((component) => Object.prototype.hasOwnProperty.call(component, 'cemMl'))) {
                throw new Error('the component catalog must not copy executable CEM-ML fixtures');
            }

            const componentTags = new Set(catalog.components.map((component) => component.tag));
            if (componentTags.size !== catalog.components.length) {
                throw new Error('the public component catalog has duplicate component tags');
            }
            const renderedRows = [...output.matchAll(/data-component-tag="([^"]+)"/g)];
            if (renderedRows.length !== catalog.components.length) {
                throw new Error(
                    `component gallery rendered ${renderedRows.length} of ${catalog.components.length} catalog records`,
                );
            }

            const componentMvp = canonicalSourceText.get('docs/component-mvp.md');
            const primitiveSource = canonicalSourceText.get('packages/cem-components/src/lib/primitives.ts');
            for (const component of catalog.components) {
                if (
                    !component.tag?.startsWith('cem-') ||
                    !Array.isArray(component.tokenFamilies) ||
                    component.tokenFamilies.length === 0 ||
                    !Array.isArray(component.categoryStates) ||
                    component.categoryStates.length === 0
                ) {
                    throw new Error(`component catalog record is incomplete: ${component.tag}`);
                }
                if (!componentMvp.includes(`| \`${component.tag}\` |`)) {
                    throw new Error(`${component.tag} is absent from canonical component semantics`);
                }
                if (!primitiveSource.includes(`tag: '${component.tag}'`)) {
                    throw new Error(`${component.tag} is absent from the executable primitive inventory`);
                }
                if (!output.includes(`data-component-tag="${component.tag}"`)) {
                    throw new Error(`${component.tag} is absent from the rendered component gallery`);
                }
                if (!output.includes(`href="${component.documentation.referenceHref}"`)) {
                    throw new Error(`${component.tag} is missing its package-owned reference link`);
                }
            }

            if (
                JSON.stringify(catalog.$generated?.canonicalSources) !== JSON.stringify(entry.canonicalSources) ||
                JSON.stringify(catalog.$generated?.evidenceSources) !== JSON.stringify(entry.evidenceSources)
            ) {
                throw new Error('component catalog provenance drifted from the route allowlist');
            }
            const stateReportSource = entry.evidenceSources.find((source) =>
                source.endsWith('/component-state-matrix.json'),
            );
            const stateReport = JSON.parse(evidenceSourceText.get(stateReportSource));
            if (JSON.stringify(catalog.stateCoverage?.summary) !== JSON.stringify(stateReport.summary)) {
                throw new Error('component catalog state coverage drifted from its Nx report');
            }

            const storybook = catalog.relatedSurfaces?.storybook;
            if (
                storybook?.owner !== 'cem-elements' ||
                storybook?.availability !== 'local-build' ||
                storybook?.devTarget !== 'cem-elements:storybook' ||
                storybook?.buildTarget !== 'cem-elements:build-storybook' ||
                !output.includes(`href="${storybook.sourceHref}"`)
            ) {
                throw new Error('component gallery Storybook ownership or source link drifted');
            }
            const examples = catalog.relatedSurfaces?.examples;
            if (!Array.isArray(examples) || examples.length === 0) {
                throw new Error('component catalog must link package-owned examples');
            }
            for (const example of examples) {
                if (
                    example.owner !== '@epa-wg/cem-components' ||
                    !example.source?.startsWith('packages/cem-components/examples/') ||
                    !output.includes(`href="${example.sourceHref}"`)
                ) {
                    throw new Error(`component example ownership or link drifted: ${example.name}`);
                }
            }

            Object.assign(verificationEntry, {
                canonicalSources: entry.canonicalSources,
                evidenceSources: entry.evidenceSources,
                componentCount: catalog.components.length,
                stateCoverage: catalog.stateCoverage.summary,
                exampleLinks: examples.length,
                storybookAvailability: storybook.availability,
                javascript: false,
            });
        }

        if (entry.contentRole === 'parity-comparison') {
            const expectedSource = 'packages/cem-components/tests/angular-material-parity.json';
            const expectedValidationTarget = '@epa-wg/cem-components:verify-material-parity';
            const expectedCanonicalSources = [
                'packages/cem-components/docs/angular-material-parity.md',
                'packages/cem-components/src/lib/primitives.ts',
            ];
            if (
                entry.route !== '/components/angular-material/' ||
                entry.source !== expectedSource ||
                entry.validationTarget !== expectedValidationTarget ||
                JSON.stringify(entry.canonicalSources) !== JSON.stringify(expectedCanonicalSources)
            ) {
                throw new Error('the Angular Material comparison provenance contract drifted');
            }
            for (const source of [expectedSource, ...expectedCanonicalSources]) {
                if (!buildInputs.has(`{workspaceRoot}/${source}`)) {
                    throw new Error(`Angular Material comparison source is absent from the Nx build hash: ${source}`);
                }
            }
            if (output.includes('<script') || output.includes('node_modules') || output.includes('@angular/')) {
                throw new Error(
                    'the static Angular Material comparison must not load an Angular or JavaScript runtime',
                );
            }

            const inventoryText = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
            const inventory = JSON.parse(inventoryText);
            const records = inventory.components;
            const benchmark = inventory.benchmark;
            if (
                inventory.version !== 1 ||
                benchmark?.product !== 'Angular Material' ||
                benchmark?.version !== '22.1.1' ||
                benchmark?.tag !== 'v22.1.1' ||
                benchmark?.commit !== '0b67c3c38141049657b1167479accc80e455d2bd' ||
                !Array.isArray(records) ||
                records.length !== 37
            ) {
                throw new Error('the Angular Material comparison lost its exact pinned benchmark');
            }
            const counts = Object.fromEntries(
                ['covered', 'partial', 'gap', 'unreviewed'].map((status) => [
                    status,
                    records.filter((record) => record.status === status).length,
                ]),
            );
            if (
                counts.covered !== 18 ||
                counts.partial !== 19 ||
                counts.gap !== 0 ||
                counts.unreviewed !== 0 ||
                inventory.recommendedAudit !== null
            ) {
                throw new Error(`Angular Material comparison coverage drifted: ${JSON.stringify(counts)}`);
            }
            const renderedRows = [...output.matchAll(/data-parity-id="([^"]+)"/g)].map((match) => match[1]);
            if (
                renderedRows.length !== records.length ||
                JSON.stringify(renderedRows) !== JSON.stringify(records.map(({ id }) => id)) ||
                !output.includes(`data-parity-total="${records.length}"`) ||
                !output.includes(`data-parity-covered="${counts.covered}"`) ||
                !output.includes(`data-parity-partial="${counts.partial}"`)
            ) {
                throw new Error('the Angular Material comparison does not retain every row and exact coverage total');
            }
            const renderedText = normalizedHtmlText(output);
            const primitiveSource = canonicalSourceText.get('packages/cem-components/src/lib/primitives.ts');
            for (const record of records) {
                const mapping = record.mapping;
                if (
                    !mapping ||
                    !['component', 'behavior'].includes(mapping.kind) ||
                    !['covered', 'partial'].includes(record.status) ||
                    !output.includes(`id="angular-material-${record.id}"`) ||
                    !output.includes(`data-parity-status="${record.status}"`) ||
                    !output.includes(`data-mapping-kind="${mapping.kind}"`)
                ) {
                    throw new Error(`Angular Material comparison record is incomplete: ${record.id}`);
                }
                for (const owner of mapping.owners) {
                    if (
                        (mapping.kind === 'component' && !primitiveSource.includes(`tag: '${owner}'`)) ||
                        (mapping.kind === 'behavior' && !owner.startsWith('behavior:'))
                    ) {
                        throw new Error(`${record.id} has an unknown rendered CEM owner ${owner}`);
                    }
                }
                for (const value of [
                    record.name,
                    ...mapping.owners,
                    ...mapping.states,
                    ...mapping.keyboard,
                    ...mapping.accessibility,
                    ...mapping.evidence,
                    mapping.notes,
                ]) {
                    if (!renderedText.includes(value)) {
                        throw new Error(`${record.id} does not render its complete parity evidence: ${value}`);
                    }
                }
            }

            Object.assign(verificationEntry, {
                canonicalSources: entry.canonicalSources,
                benchmark: {
                    product: benchmark.product,
                    version: benchmark.version,
                    tag: benchmark.tag,
                    commit: benchmark.commit,
                    capturedAt: benchmark.capturedAt,
                },
                coverage: { total: records.length, ...counts },
                javascript: false,
                angularRuntime: false,
            });
        }

        if (entry.contentRole === 'interactive-example') {
            if (!interactiveRuntimeContract.runtime.routes.includes(entry.route)) {
                throw new Error(`${entry.route} is not declared as an interactive runtime consumer`);
            }
            const fixture = JSON.parse(await readFile(resolve(workspaceRoot, entry.source), 'utf8'));
            const tokenCatalogSource = entry.evidenceSources.find((source) =>
                source.endsWith('/cem.tokens.catalog.json'),
            );
            const componentCatalogSource = entry.evidenceSources.find((source) =>
                source.endsWith('/cem.components.catalog.json'),
            );
            const tokenCatalog = JSON.parse(evidenceSourceText.get(tokenCatalogSource));
            const componentCatalog = JSON.parse(evidenceSourceText.get(componentCatalogSource));
            const tokenNames = new Set(tokenCatalog.tokens.map(({ name }) => name));
            const componentTags = new Set(componentCatalog.components.map(({ tag }) => tag));
            const fixtureTokenNames = fixture.tokens.map(({ name }) => name);
            const fixtureComponentTags = fixture.components.map(({ tag }) => tag);
            if (
                new Set(fixtureTokenNames).size !== fixtureTokenNames.length ||
                fixtureTokenNames.some((name) => !tokenNames.has(name)) ||
                new Set(fixtureComponentTags).size !== fixtureComponentTags.length ||
                fixtureComponentTags.some((tag) => !componentTags.has(tag))
            ) {
                throw new Error('interactive fixture references unknown or duplicate package identities');
            }
            const tokenSource = canonicalSourceText.get('packages/cem-theme/src/lib/tokens/cem-colors.md');
            const componentSource = canonicalSourceText.get('packages/cem-components/src/lib/primitives.ts');
            for (const name of fixtureTokenNames) {
                if (!tokenSource.includes(`\`${name}\``) || !output.includes(name)) {
                    throw new Error(`interactive token example ${name} lost canonical provenance`);
                }
            }
            for (const tag of fixtureComponentTags) {
                if (!componentSource.includes(`tag: '${tag}'`) || !output.includes(`<${tag}`)) {
                    throw new Error(`interactive component example ${tag} lost canonical provenance`);
                }
            }
            for (const token of [
                '<script type="importmap">',
                'import "@epa-wg/cem-site/runtime";',
                '<custom-element tag="cem-site-greeting">',
                '<pre><code data-native-output>',
                '@epa-wg/cem-site/runtime',
                '@epa-wg/custom-element',
                '@epa-wg/cem-components/primitives',
            ]) {
                if (!output.includes(token)) {
                    throw new Error(`${entry.output} is missing interactive contract token ${token}`);
                }
            }
            for (const graphToken of [
                '@source-map="runtime/source.module-map.json"',
                '@target-map="runtime/destination.module-map.json"',
            ]) {
                if (!publicationGraph.includes(graphToken)) {
                    throw new Error(`interactive publication graph is missing ${graphToken}`);
                }
            }
            if (output.includes('node_modules')) {
                throw new Error('interactive output leaks a source-only dependency path');
            }
            Object.assign(verificationEntry, {
                canonicalSources: entry.canonicalSources,
                evidenceSources: entry.evidenceSources,
                tokenExamples: fixtureTokenNames,
                componentExamples: fixtureComponentTags,
                cemFixtureTag: fixture.cemFixture.tag,
                runtimeAssetCount: interactiveRuntimeContract.declarationCount,
                javascript: true,
            });
        }

        if (entry.contentRole === 'search') {
            if (!searchRuntimeContract.runtime.routes.includes(entry.route)) {
                throw new Error(`${entry.route} is not declared as a search runtime consumer`);
            }
            for (const token of [
                '<form action="/search/" method="get">',
                'data-search-index="route-manifest"',
                '<ol data-search-results',
                '<cem-field data-search-field',
                '<cem-action data-search-action',
                '<script type="importmap">',
                'import "@epa-wg/cem-site/search";',
                '@epa-wg/cem-site/search',
                '@epa-wg/cem-site/components-runtime',
                '@epa-wg/custom-element',
                '@epa-wg/cem-components/primitives',
            ]) {
                if (!output.includes(token)) {
                    throw new Error(`${entry.output} is missing search contract token ${token}`);
                }
            }
            if ([...output.matchAll(/data-search-document="/g)].length !== manifest.searchDocuments.length) {
                throw new Error(`${entry.output} does not render every searchable document without JavaScript`);
            }
            for (const graphToken of [
                '@source-map="runtime/source.module-map.json"',
                '@target-map="runtime/destination.module-map.json"',
            ]) {
                if (!publicationGraph.includes(graphToken)) {
                    throw new Error(`search publication graph is missing ${graphToken}`);
                }
            }
            Object.assign(verificationEntry, {
                searchDocuments: manifest.searchDocuments.length,
                runtimeAssetCount: searchRuntimeContract.declarationCount,
                javascript: true,
            });
        }

        verification.entries.push(verificationEntry);
    } else {
        const source = await readFile(resolve(workspaceRoot, entry.source), 'utf8');
        if (output !== source) {
            throw new Error(`${entry.output} is not a byte-stable publication of ${entry.source}`);
        }
        JSON.parse(output);
        verification.entries.push({
            route: entry.route,
            kind: entry.kind,
            owner: entry.owner,
            ownerRoot: ownersByRoute.get(entry.route).root,
            upstreamTarget: entry.upstreamTarget,
            bytes: Buffer.byteLength(output),
        });
    }
}

for (const route of interactiveRuntimeContract.runtime.routes) {
    if (entriesByRoute.get(route)?.contentRole !== 'interactive-example') {
        throw new Error(`runtime route ${route} is not an allowlisted interactive example`);
    }
}

for (const route of searchRuntimeContract.runtime.routes) {
    if (entriesByRoute.get(route)?.contentRole !== 'search') {
        throw new Error(`runtime route ${route} is not an allowlisted search page`);
    }
}

for (const asset of runtimeContract.assets) {
    const [source, output] = await Promise.all([readFile(asset.sourcePath), readFile(join(outputRoot, asset.output))]);
    if (!source.equals(output)) {
        throw new Error(`${asset.output} is not an exact publication of ${asset.source}`);
    }
}

for (const contract of runtimeContracts) {
    const runtimeImports = new Set(Object.keys(contract.destinationMap.imports));
    for (const asset of contract.assets.filter(({ contentType }) => contentType === 'text/javascript')) {
        const source = await readFile(join(outputRoot, asset.output), 'utf8');
        const moduleSpecifiers = [
            ...source.matchAll(/(?:import|export)\s+(?:[^'";]*?\sfrom\s*)?['"]([^'"]+)['"]/g),
        ].map((match) => match[1]);
        for (const specifier of moduleSpecifiers) {
            if (specifier.startsWith('.')) {
                const dependency = new URL(specifier, `https://cem.invalid/${asset.output}`).pathname.slice(1);
                if (!contract.assetOutputs.has(dependency)) {
                    throw new Error(`${asset.output} has undeclared relative dependency ${specifier}`);
                }
            } else if (!runtimeImports.has(specifier)) {
                throw new Error(`${asset.output} has undeclared bare dependency ${specifier}`);
            }
        }
        const urlSpecifiers = [...source.matchAll(/new URL\(\s*['"]([^'"]+)['"]\s*,\s*import\.meta\.url\s*\)/g)].map(
            (match) => match[1],
        );
        for (const specifier of urlSpecifiers) {
            if (specifier.startsWith('/') || /^[a-z][a-z\d+.-]*:/i.test(specifier)) {
                throw new Error(`${asset.output} has non-relative module URL dependency ${specifier}`);
            }
            const dependency = new URL(specifier, `https://cem.invalid/${asset.output}`).pathname.slice(1);
            if (!contract.assetOutputs.has(dependency)) {
                throw new Error(`${asset.output} has undeclared module URL dependency ${specifier}`);
            }
        }
    }
}

const generatedReference = await readFile(join(outputRoot, 'reference/cem-ml/transform-config/index.html'), 'utf8');
if (!generatedReference.includes('CEM-ML CLI Transform Config Schema')) {
    throw new Error('generated CEM-ML documentation was not ingested into its stable route');
}

await mkdir(reportRoot, { recursive: true });
const reportPath = join(reportRoot, 'verification.json');
await writeFile(reportPath, `${JSON.stringify(verification, null, 2)}\n`, 'utf8');
console.log(`CEM Site verification passed: ${relative(workspaceRoot, reportPath)}`);
