import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(packageRoot, '../..');
const outputPath = join(packageRoot, 'dist/catalog/cem.components.catalog.json');
const canonicalSources = [
    'docs/component-mvp.md',
    'packages/cem-components/declarative-migration.json',
    'packages/cem-components/src/lib/primitives.ts',
    'packages/cem-components/docs/component-reference.md',
    'packages/cem-components/docs/conventions.md',
    'packages/cem-components/docs/accessibility.md',
];
const evidenceSources = [
    'packages/cem-components/dist/reports/component-state-matrix.json',
];
const repositoryRevision = 'develop';
const referenceAnchors = {
    Action: 'actions',
    Input: 'inputs',
    Layout: 'layout',
    Content: 'content',
    Navigation: 'navigation',
    Feedback: 'feedback',
};

const packageJson = JSON.parse(await readWorkspaceFile('packages/cem-components/package.json'));
const componentMvp = await readWorkspaceFile(canonicalSources[0]);
const migration = JSON.parse(await readWorkspaceFile(canonicalSources[1]));
const primitiveSource = await readWorkspaceFile(canonicalSources[2]);
const stateReport = JSON.parse(await readWorkspaceFile(evidenceSources[0]));
const examplesReadme = await readWorkspaceFile('packages/cem-components/examples/README.md');
const cemElementsProject = JSON.parse(
    await readWorkspaceFile('packages/cem-elements/project.json'),
);
const repositoryBase = normalizeRepositoryUrl(packageJson.repository?.url);
const components = parseMvpComponents(componentMvp);
const declarativeTags = new Set(
    (await readdir(join(packageRoot, migration.componentRoot), { withFileTypes: true }))
        .filter((entry) => entry.isDirectory())
        .map((entry) => entry.name),
);
const examples = await parseExamples(examplesReadme);

validateComponentSources(components, primitiveSource, declarativeTags, stateReport);
validateRelatedSurfaceTargets(cemElementsProject);

const stateCategories = [...new Set(components.map(({ category }) => category))].map(
    (category) => ({
        category,
        states: stateReport.coverage
            .filter((record) => record.category === category)
            .map(({ state, status }) => ({ name: state, status })),
    }),
);
const stateCategoriesByName = new Map(
    stateCategories.map((category) => [category.category, category]),
);

const catalog = {
    version: 1,
    $generated: {
        packageVersion: packageJson.version,
        generator: 'packages/cem-components/scripts/build-component-catalog.mjs',
        canonicalSources,
        evidenceSources,
        componentCount: components.length,
    },
    components: components.map((component) => ({
        ...component,
        implementation: declarativeTags.has(component.tag)
            ? {
                  kind: 'cem-element-xhtml',
                  source: `packages/cem-components/src/components/${component.tag}/${component.tag}.xhtml`,
              }
            : {
                  kind: 'legacy-registry',
                  source: 'packages/cem-components/src/lib/primitives.ts',
              },
        categoryStates: stateCategoriesByName.get(component.category).states,
        documentation: {
            referenceHref: sourceHref(
                'packages/cem-components/docs/component-reference.md',
                referenceAnchors[component.category],
            ),
            semanticsHref: sourceHref('docs/component-mvp.md', 'component-list'),
        },
    })),
    stateCoverage: {
        source: stateReport.source,
        summary: stateReport.summary,
        recommendedNext: stateReport.recommendedNext,
        categories: stateCategories,
    },
    guidance: canonicalSources.slice(3).map((source) => ({
        source,
        href: sourceHref(source),
    })),
    relatedSurfaces: {
        storybook: {
            owner: 'cem-elements',
            availability: 'local-build',
            devTarget: 'cem-elements:storybook',
            buildTarget: 'cem-elements:build-storybook',
            source: 'packages/cem-elements/.storybook',
            sourceHref: sourceHref('packages/cem-elements/.storybook', undefined, 'tree'),
        },
        examples: examples.map((example) => ({
            ...example,
            owner: '@epa-wg/cem-components',
            sourceHref: sourceHref(example.source),
        })),
    },
};

await mkdir(dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(catalog, null, 2)}\n`, 'utf8');
console.log(
    `cem-components catalog built (${catalog.components.length} components, ` +
        `${catalog.relatedSurfaces.examples.length} source-linked examples).`,
);

async function readWorkspaceFile(path) {
    return readFile(resolve(workspaceRoot, path), 'utf8');
}

function normalizeRepositoryUrl(url) {
    if (typeof url !== 'string' || !url) {
        throw new Error('package repository.url must name the catalog source repository');
    }
    return url.replace(/^git\+/, '').replace(/\.git$/, '');
}

function sourceHref(path, anchor, view = 'blob') {
    const suffix = anchor ? `#${anchor}` : '';
    return `${repositoryBase}/${view}/${repositoryRevision}/${path}${suffix}`;
}

function parseMvpComponents(markdown) {
    const parsed = [];
    let inComponentTable = false;

    for (const line of markdown.split(/\r?\n/)) {
        if (line.startsWith('| Category | Component ID | Element name |')) {
            inComponentTable = true;
            continue;
        }
        if (!inComponentTable || line.startsWith('| ---')) {
            continue;
        }
        if (!line.startsWith('|')) {
            break;
        }

        const cells = tableCells(line);
        if (cells.length !== 5) {
            throw new Error(`component MVP row must have 5 cells: ${line}`);
        }
        const [category, idCell, tagCell, primaryUse, tokenFamiliesCell] = cells;
        const id = stripCode(idCell);
        const tag = stripCode(tagCell);
        const tokenFamilies = tokenFamiliesCell
            .split(',')
            .map((family) => family.trim())
            .filter(Boolean);
        parsed.push({ id, tag, category, primaryUse, tokenFamilies });
    }

    if (parsed.length === 0) {
        throw new Error('docs/component-mvp.md must contain component catalog rows');
    }
    return parsed;
}

async function parseExamples(markdown) {
    const parsed = [];
    let inExamplesTable = false;

    for (const line of markdown.split(/\r?\n/)) {
        if (line === '| Example | Purpose |') {
            inExamplesTable = true;
            continue;
        }
        if (!inExamplesTable || line.startsWith('| ---')) {
            continue;
        }
        if (!line.startsWith('|')) {
            break;
        }

        const cells = tableCells(line);
        const link = cells[0]?.match(/^\[`([^`]+)`\]\(\.\/([^)]+)\)$/);
        if (cells.length !== 2 || !link) {
            throw new Error(`component example row must contain one local source link: ${line}`);
        }
        parsed.push({
            name: link[1],
            source: `packages/cem-components/examples/${link[2]}`,
            purpose: cells[1],
        });
    }

    const exampleFiles = (await readdir(join(packageRoot, 'examples')))
        .filter((name) => name.endsWith('.html'))
        .sort();
    const documentedFiles = parsed.map(({ name }) => name).sort();
    if (JSON.stringify(exampleFiles) !== JSON.stringify(documentedFiles)) {
        throw new Error(
            `component example inventory drifted: files=${exampleFiles.join(', ')}; ` +
                `documented=${documentedFiles.join(', ')}`,
        );
    }
    return parsed;
}

function tableCells(line) {
    return line
        .slice(1, -1)
        .split('|')
        .map((cell) => cell.trim());
}

function stripCode(value) {
    return value.replace(/^`|`$/g, '');
}

function validateComponentSources(componentRows, source, declarativeTags, report) {
    const tags = componentRows.map(({ tag }) => tag);
    if (new Set(tags).size !== tags.length) {
        throw new Error('component catalog contains duplicate tags');
    }
    for (const component of componentRows) {
        if (!component.id || !component.tag || !component.primaryUse) {
            throw new Error(`component catalog row is incomplete: ${JSON.stringify(component)}`);
        }
        if (!component.tag.startsWith('cem-') || component.tokenFamilies.length === 0) {
            throw new Error(`${component.tag} must have a CEM tag and token families`);
        }
        if (!source.includes(`tag: '${component.tag}'`) && !declarativeTags.has(component.tag)) {
            throw new Error(`${component.tag} has neither a legacy registry entry nor a declarative XHTML folder`);
        }
        if (!referenceAnchors[component.category]) {
            throw new Error(`${component.tag} has unsupported category ${component.category}`);
        }
    }
    if (
        report.version !== 1 ||
        !Array.isArray(report.coverage) ||
        typeof report.summary !== 'object'
    ) {
        throw new Error('component state-matrix report has an unsupported shape');
    }
    for (const category of new Set(componentRows.map(({ category }) => category))) {
        if (!report.coverage.some((record) => record.category === category)) {
            throw new Error(`state-matrix report has no ${category} coverage`);
        }
    }
}

function validateRelatedSurfaceTargets(project) {
    if (!project.targets?.storybook || !project.targets?.['build-storybook']) {
        throw new Error('cem-elements must own storybook and build-storybook targets');
    }
}
