import { createHash } from 'node:crypto';
import { access, readFile } from 'node:fs/promises';
import { dirname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptRoot = dirname(fileURLToPath(import.meta.url));
const projectRoot = dirname(scriptRoot);
const workspaceRoot = resolve(projectRoot, '../..');
const manifest = JSON.parse(
    await readFile(resolve(projectRoot, 'test-fixtures/external-reference-corpus.json'), 'utf8')
);

const expectedSource = {
    repository: 'git@github.com:EPA-WG/custom-element-dist.git',
    main: '9887eec720704ec33e5c37e73a07a7437b2ed0f1',
    develop: '49d20ab3d1faf9659eb57493eb81abc148a61ec4',
    npmGitHead: '63ef0cfd11f4cba027fc5e3da83b682aa210f42c',
    inventoryCommit: '49d20ab3d1faf9659eb57493eb81abc148a61ec4',
    inventoryTree: 'fcc5992c2b6a8b38fcc6a41c35ede81311b1511f',
};
const expectedBrowserCategoryCounts = {
    attributes: 9,
    'scoped-css': 3,
    'dom-merge': 5,
    'external-templates': 12,
    'form-validity': 5,
    'http-request': 5,
    'local-storage': 4,
    location: 2,
    'module-url': 4,
    'set-url': 2,
    'slice-events': 9,
    slots: 7,
    'version-selection': 1,
    'xslt-conditionals': 15,
    'xslt-for-each': 3,
    'xslt-if-regression': 1,
    'import-map-frame': 1,
};
const expectedDispositionCounts = {
    accepted: 61,
    'package-adapter': 29,
    'rejected-bridge': 1,
};
const expectedInventoryDigest = '1e34cf6c782a36f87e0f6b3ac0d32b47d4fe7b49c79425f1a4dda665bb9d737e';
const projectConfigPaths = {
    '@epa-wg/custom-element': 'packages/custom-element/project.json',
    'cem-elements': 'packages/cem-elements/project.json',
};
const projectConfigs = new Map();

assertEqual(manifest.schemaVersion, 1, 'manifest schema version');
for (const [field, expected] of Object.entries(expectedSource)) {
    assertEqual(manifest.source?.[field], expected, `source ${field}`);
}
assertEqual(manifest.expected?.browserCases, 88, 'expected browser cases');
assertEqual(manifest.expected?.unitCases, 3, 'expected unit cases');
assertEqual(manifest.expected?.sourceFiles, 18, 'expected source files');
assertObjectEqual(
    manifest.expected?.browserCategoryCounts,
    expectedBrowserCategoryCounts,
    'expected browser category counts'
);

const sourceFiles = manifest.source?.sourceFiles;
assertPlainObject(sourceFiles, 'source files');
assertEqual(Object.keys(sourceFiles).length, manifest.expected.sourceFiles, 'source file count');
for (const [path, blob] of Object.entries(sourceFiles)) {
    assertSafeRelativePath(path, `external source path ${path}`);
    assert(/^[0-9a-f]{40}$/u.test(blob), `${path}: expected a 40-character Git blob id`);
}

const evidence = manifest.evidence;
assertPlainObject(evidence, 'evidence catalog');
assert(Object.keys(evidence).length > 0, 'evidence catalog must not be empty');
for (const [id, item] of Object.entries(evidence)) {
    assert(/^[a-z][a-z0-9-]*$/u.test(id), `invalid evidence id ${id}`);
    assert(
        ['accepted', 'package-adapter', 'rejected-bridge'].includes(item.disposition),
        `${id}: invalid disposition ${item.disposition}`
    );
    const allowedStates = {
        accepted: ['verified'],
        'package-adapter': ['required', 'verified'],
        'rejected-bridge': ['rejected'],
    }[item.disposition];
    assert(allowedStates.includes(item.state), `${id}: invalid ${item.disposition} state ${item.state}`);
    assertNonEmptyString(item.summary, `${id}: summary`);
    assert(Array.isArray(item.paths) && item.paths.length > 0, `${id}: paths must not be empty`);
    for (const path of item.paths) {
        await assertWorkspacePathExists(path, `${id}: evidence path`);
    }
    assert(Array.isArray(item.nxTargets), `${id}: nxTargets must be an array`);
    if (item.disposition === 'rejected-bridge') {
        assertEqual(item.nxTargets.length, 0, `${id}: rejected evidence target count`);
        assertNonEmptyString(item.rationale, `${id}: rejection rationale`);
    } else {
        assert(item.nxTargets.length > 0, `${id}: evidence must name at least one Nx target`);
        for (const target of item.nxTargets) {
            await assertNxTargetExists(target, `${id}: Nx target`);
        }
    }
}

assert(Array.isArray(manifest.browserCategories), 'browserCategories must be an array');
const categoryIds = new Set();
const caseIds = new Set();
const referencedSources = new Set();
const dispositionCounts = { accepted: 0, 'package-adapter': 0, 'rejected-bridge': 0 };
let browserCaseCount = 0;

for (const category of manifest.browserCategories) {
    assertNonEmptyString(category.id, 'browser category id');
    assert(!categoryIds.has(category.id), `duplicate browser category ${category.id}`);
    categoryIds.add(category.id);
    assert(
        Object.hasOwn(expectedBrowserCategoryCounts, category.id),
        `unexpected browser category ${category.id}`
    );
    assert(['storybook', 'browser-harness'].includes(category.sourceKind), `${category.id}: invalid sourceKind`);
    assertSourceRegistered(category.source, sourceFiles, `${category.id}: source`);
    referencedSources.add(category.source);
    assert(Array.isArray(category.cases), `${category.id}: cases must be an array`);
    assertEqual(
        category.cases.length,
        expectedBrowserCategoryCounts[category.id],
        `${category.id}: case count`
    );
    for (const item of category.cases) {
        assertNonEmptyString(item.symbol, `${category.id}: case symbol`);
        const caseId = `browser:${category.id}:${item.symbol}`;
        assert(!caseIds.has(caseId), `duplicate case ${caseId}`);
        caseIds.add(caseId);
        countEvidence(item.evidence, caseId, evidence, dispositionCounts);
        browserCaseCount += 1;
    }
}

assertArrayEqual([...categoryIds].sort(), Object.keys(expectedBrowserCategoryCounts).sort(), 'browser category ids');
assertEqual(browserCaseCount, manifest.expected.browserCases, 'browser case count');

assert(Array.isArray(manifest.unitCases), 'unitCases must be an array');
for (const item of manifest.unitCases) {
    assertNonEmptyString(item.symbol, 'unit case symbol');
    const caseId = `unit:helpers:${item.symbol}`;
    assert(!caseIds.has(caseId), `duplicate case ${caseId}`);
    caseIds.add(caseId);
    assertSourceRegistered(item.source, sourceFiles, `${caseId}: source`);
    referencedSources.add(item.source);
    assert(Array.isArray(item.helpers) && item.helpers.length > 0, `${caseId}: helpers must not be empty`);
    assertEqual(new Set(item.helpers).size, item.helpers.length, `${caseId}: unique helpers`);
    for (const helper of item.helpers) {
        assertNonEmptyString(helper, `${caseId}: helper`);
    }
    countEvidence(item.evidence, caseId, evidence, dispositionCounts);
}
assertEqual(manifest.unitCases.length, manifest.expected.unitCases, 'unit case count');
assertArrayEqual([...referencedSources].sort(), Object.keys(sourceFiles).sort(), 'referenced external source files');
assertObjectEqual(dispositionCounts, expectedDispositionCounts, 'evidence disposition counts');

const inventoryDigest = createHash('sha256').update(JSON.stringify(stableValue(manifest))).digest('hex');
assertEqual(inventoryDigest, expectedInventoryDigest, 'locked inventory digest');

console.log(
    `Verified external custom-element reference corpus: ${browserCaseCount} browser cases, ` +
        `${manifest.unitCases.length} unit cases, ${dispositionCounts.accepted} accepted, ` +
        `${dispositionCounts['package-adapter']} package-adapter, ` +
        `${dispositionCounts['rejected-bridge']} rejected-bridge.`
);

function countEvidence(evidenceId, caseId, catalog, counts) {
    assertNonEmptyString(evidenceId, `${caseId}: evidence`);
    const item = catalog[evidenceId];
    assert(item !== undefined, `${caseId}: unknown evidence ${evidenceId}`);
    counts[item.disposition] += 1;
}

function assertSourceRegistered(path, registered, label) {
    assertNonEmptyString(path, label);
    assert(Object.hasOwn(registered, path), `${label}: ${path} is not provenance-locked`);
}

async function assertWorkspacePathExists(path, label) {
    assertSafeRelativePath(path, label);
    const absolute = resolve(workspaceRoot, path);
    assert(
        absolute.startsWith(`${workspaceRoot}${sep}`),
        `${label}: ${path} resolves outside the workspace`
    );
    await access(absolute);
}

async function assertNxTargetExists(reference, label) {
    assertNonEmptyString(reference, label);
    const project = Object.keys(projectConfigPaths)
        .sort((a, b) => b.length - a.length)
        .find((name) => reference.startsWith(`${name}:`));
    assert(project !== undefined, `${label}: unknown project in ${reference}`);
    const target = reference.slice(project.length + 1);
    assertNonEmptyString(target, `${label}: target name`);
    let config = projectConfigs.get(project);
    if (!config) {
        config = JSON.parse(await readFile(resolve(workspaceRoot, projectConfigPaths[project]), 'utf8'));
        projectConfigs.set(project, config);
    }
    assert(config.targets?.[target] !== undefined, `${label}: ${reference} does not exist`);
}

function assertSafeRelativePath(path, label) {
    assertNonEmptyString(path, label);
    assert(!path.startsWith('/') && !path.split('/').includes('..'), `${label}: expected a safe relative path`);
}

function stableValue(value) {
    if (Array.isArray(value)) {
        return value.map(stableValue);
    }
    if (value && typeof value === 'object') {
        return Object.fromEntries(
            Object.keys(value)
                .sort()
                .map((key) => [key, stableValue(value[key])])
        );
    }
    return value;
}

function assertPlainObject(value, label) {
    assert(value && typeof value === 'object' && !Array.isArray(value), `${label}: expected an object`);
}

function assertNonEmptyString(value, label) {
    assert(typeof value === 'string' && value.trim().length > 0, `${label}: expected a non-empty string`);
}

function assertArrayEqual(actual, expected, label) {
    assertEqual(JSON.stringify(actual), JSON.stringify(expected), label);
}

function assertObjectEqual(actual, expected, label) {
    assertEqual(JSON.stringify(stableValue(actual)), JSON.stringify(stableValue(expected)), label);
}

function assertEqual(actual, expected, label) {
    assert(actual === expected, `${label}: expected ${expected}, got ${actual}`);
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
