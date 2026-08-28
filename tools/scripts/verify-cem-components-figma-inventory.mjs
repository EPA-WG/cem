#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import ts from 'typescript';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const inventoryPath = join(repoRoot, 'examples/figma/component-library.json');
const fixturePath = join(repoRoot, 'examples/figma/component-library-fixture.md');
const mvpPath = join(repoRoot, 'docs/component-mvp.md');
const referencePath = join(repoRoot, 'packages/cem-components/docs/component-reference.md');
const primitivesPath = join(repoRoot, 'packages/cem-components/src/lib/primitives.ts');
const declarativeComponentsPath = join(repoRoot, 'packages/cem-components/src/components');
const stateMatrixPath = join(repoRoot, 'packages/cem-components/tests/state-matrix-coverage.json');
const reportDirectory = join(repoRoot, 'packages/cem-components/dist/reports');
const reportJsonPath = join(reportDirectory, 'figma-component-library.json');
const reportMarkdownPath = join(reportDirectory, 'figma-component-library.md');

const expectedSources = {
    implementations: 'packages/cem-components/declarative-migration.json',
    components: 'docs/component-mvp.md#component-list',
    states: 'packages/cem-components/tests/state-matrix-coverage.json',
    documentation: 'packages/cem-components/docs/component-reference.md',
};
const expectedModes = ['Light', 'Dark', 'Contrast Light', 'Contrast Dark', 'Native'];
const payloadTags = new Set(['cem-option', 'cem-option-group', 'cem-tab', 'cem-step', 'cem-tree-item']);
const structuralTags = new Set(['cem-stack', 'cem-grid', 'cem-table', 'cem-app-bar']);
const allowedRepresentations = new Set(['component-set', 'component', 'payload', 'structural']);
const allowedPropertyKinds = new Set(['variant', 'boolean', 'text', 'instance-swap', 'slot']);
const allowedReviewStatuses = new Set(['planned', 'reviewed']);
const categorySections = new Map([
    ['Action', 'Actions'],
    ['Input', 'Inputs'],
    ['Layout', 'Layout'],
    ['Content', 'Content'],
    ['Navigation', 'Navigation'],
    ['Feedback', 'Feedback'],
]);
const failures = [];

const mvp = parseMvpComponents(readText(mvpPath));
const primitives = parsePrimitiveDeclarations(readText(primitivesPath));
const primitiveByTag = new Map(primitives.map((primitive) => [primitive.tag, primitive]));
const declarations = mvp.map((component) => {
    const legacy = primitiveByTag.get(component.tag);
    if (legacy) return legacy;
    const path = join(declarativeComponentsPath, component.tag, `${component.tag}.xhtml`);
    return existsSync(path) ? { tag: component.tag, cemMl: readText(path) } : undefined;
});
const referenceRows = parseReferenceRows(readText(referencePath));
const stateMatrix = readJson(stateMatrixPath);
const inventory = readJson(inventoryPath);
const fixture = readText(fixturePath);
const coveredStates = coveredStatesByComponent(stateMatrix);

validateTopLevel();
validateEntries();
validateFixture();

if (failures.length > 0) {
    for (const failure of failures) console.error(`error: ${failure}`);
    process.exit(1);
}

const entries = inventory.components;
const summary = {
    total: entries.length,
    componentSets: entries.filter((entry) => entry.representation === 'component-set').length,
    components: entries.filter((entry) => entry.representation === 'component').length,
    payloads: entries.filter((entry) => entry.representation === 'payload').length,
    structural: entries.filter((entry) => entry.representation === 'structural').length,
    planned: entries.filter((entry) => entry.figma.status === 'planned').length,
    reviewed: entries.filter((entry) => entry.figma.status === 'reviewed').length,
};
const report = {
    version: inventory.version,
    sources: inventory.sources,
    figma: inventory.figma,
    summary,
    components: entries,
};

mkdirSync(reportDirectory, { recursive: true });
writeFileSync(reportJsonPath, `${JSON.stringify(report, null, 4)}\n`);
writeFileSync(reportMarkdownPath, renderMarkdownReport(report));

console.log(
    `cem-components Figma inventory verified (${summary.total} primitives: ` +
        `${summary.componentSets} component sets, ${summary.components} components, ` +
        `${summary.payloads} payloads, ${summary.structural} structural; ` +
        `${summary.reviewed} reviewed, ${summary.planned} planned).`,
);

function validateTopLevel() {
    if (!inventory || typeof inventory !== 'object' || Array.isArray(inventory)) {
        fail('Figma component inventory must be an object');
        return;
    }
    if (inventory.version !== 1) fail('Figma component inventory version must be 1');
    if (!sameJson(inventory.sources, expectedSources)) {
        fail('Figma component inventory sources must name the canonical primitive, component, state, and documentation owners');
    }
    if (!inventory.figma || typeof inventory.figma !== 'object' || Array.isArray(inventory.figma)) {
        fail('Figma component inventory must define its native library boundary');
    } else {
        if (inventory.figma.file !== 'https://www.figma.com/design/vLZUzjS7xHACjXgYLA9vtD/CEM-UI-Kit') {
            fail('Figma component inventory must target the canonical CEM UI Kit file');
        }
        if (inventory.figma.page !== '03 Components') {
            fail('Figma component inventory must target the 03 Components page');
        }
        if (!sameJson(inventory.figma.modes, expectedModes)) {
            fail(`Figma component inventory modes must be ${expectedModes.join(', ')}`);
        }
    }
    if (!Array.isArray(inventory.components)) fail('Figma component inventory components must be an array');
}

function validateEntries() {
    if (!Array.isArray(inventory.components)) return;

    const expectedTags = mvp.map((component) => component.tag);
    const primitiveTags = declarations.map((declaration) => declaration?.tag);
    const actualTags = inventory.components.map((entry) => entry?.tag);
    validateExactList('public primitive declarations', primitiveTags, expectedTags);
    validateExactList('Figma component inventory order', actualTags, expectedTags);

    const duplicates = duplicateValues(actualTags);
    for (const duplicate of duplicates) fail(`duplicate Figma component inventory entry ${duplicate}`);

    for (const [index, component] of mvp.entries()) {
        validateEntry(inventory.components[index], component, declarations[index]);
    }
}

function validateEntry(entry, component, primitive) {
    const label = component.tag;
    if (!entry || typeof entry !== 'object' || Array.isArray(entry)) {
        fail(`${label}: inventory entry must be an object`);
        return;
    }
    if (entry.tag !== component.tag) fail(`${label}: tag must match the public primitive order`);
    if (entry.category !== component.category) {
        fail(`${label}: category must be ${component.category}, found ${String(entry.category)}`);
    }
    if (!sameJson(entry.tokenFamilies, component.tokenFamilies)) {
        fail(`${label}: token families must exactly match docs/component-mvp.md (${component.tokenFamilies.join(', ')})`);
    }

    const expectedStates = expectedStatesFor(component.tag);
    if (!sameJson(entry.states, expectedStates)) {
        fail(`${label}: executable states must be ${expectedStates.join(', ') || '(none)'}`);
    }

    validateProperties(entry, component, primitive);
    const expectedRepresentation = representationFor(component.tag, expectedStates, entry.properties);
    if (!allowedRepresentations.has(entry.representation)) {
        fail(`${label}: unknown representation ${String(entry.representation)}`);
    } else if (entry.representation !== expectedRepresentation) {
        fail(`${label}: representation must be ${expectedRepresentation}, found ${entry.representation}`);
    }

    validateDocumentation(entry, component, primitive);
    validateFigmaEvidence(entry, component);
}

function validateProperties(entry, component, primitive) {
    const label = component.tag;
    if (!entry.properties || typeof entry.properties !== 'object' || Array.isArray(entry.properties)) {
        fail(`${label}: properties must be a source-to-Figma-kind object`);
        return;
    }
    const properties = Object.entries(entry.properties);
    if (properties.length === 0) fail(`${label}: properties must name at least one public authoring surface`);

    const referenceRow = referenceRows.get(label);
    const documentationText = documentationTextFor(entry.documentation, label);
    const evidenceText = [primitive?.cemMl ?? '', referenceRow?.line ?? '', documentationText].join('\n');

    for (const [source, kind] of properties) {
        if (!source || /\s/u.test(source)) fail(`${label}: property source ${JSON.stringify(source)} must be one token`);
        if (!allowedPropertyKinds.has(kind)) {
            fail(`${label}: ${source} has unsupported Figma property kind ${String(kind)}`);
        }
        if (source.startsWith('slot:') && kind !== 'slot') {
            fail(`${label}: ${source} must use the slot property kind`);
        }
        if (!source.startsWith('slot:') && kind === 'slot') {
            fail(`${label}: slot property ${source} must use a slot:* source`);
        }
        if (!propertyHasPublicEvidence(source, evidenceText)) {
            fail(`${label}: property ${source} is not traceable to its primitive or public documentation`);
        }
    }
}

function validateDocumentation(entry, component, primitive) {
    const label = component.tag;
    if (!Array.isArray(entry.documentation) || entry.documentation.length === 0) {
        fail(`${label}: documentation must name at least the component-reference section`);
        return;
    }
    const section = categorySections.get(component.category);
    const expectedPrimary = `packages/cem-components/docs/component-reference.md#${section?.toLowerCase()}`;
    if (entry.documentation[0] !== expectedPrimary) {
        fail(`${label}: primary documentation must be ${expectedPrimary}`);
    }
    if (!referenceRows.has(label)) {
        fail(`${label}: missing component-reference table row`);
    }
    for (const reference of entry.documentation) validateDocumentationReference(reference, label);

    const combined = documentationTextFor(entry.documentation, label);
    if (!combined.includes(label) && !(primitive?.cemMl ?? '').includes(label)) {
        fail(`${label}: documentation does not identify the public primitive`);
    }
}

function validateFigmaEvidence(entry, component) {
    const label = component.tag;
    const evidence = entry.figma;
    if (!evidence || typeof evidence !== 'object' || Array.isArray(evidence)) {
        fail(`${label}: figma evidence must be an object`);
        return;
    }
    if (!allowedReviewStatuses.has(evidence.status)) {
        fail(`${label}: Figma status must be planned or reviewed`);
    }
    const section = categorySections.get(component.category);
    const expectedLocator = `03 Components / ${section} / ${label}`;
    if (evidence.locator !== expectedLocator) {
        fail(`${label}: Figma locator must be ${expectedLocator}`);
    }
    if (evidence.status === 'planned' && evidence.revision !== null) {
        fail(`${label}: planned Figma evidence must use a null revision`);
    }
    if (evidence.status === 'reviewed') {
        if (typeof evidence.revision !== 'string' || !evidence.revision.trim()) {
            fail(`${label}: reviewed Figma evidence requires a revision or node URL`);
        } else if (!/^https:\/\/www\.figma\.com\/(design|file)\//u.test(evidence.revision) && !/^[a-z0-9._-]+$/iu.test(evidence.revision)) {
            fail(`${label}: reviewed Figma revision must be a Figma node URL or stable revision token`);
        }
    }
}

function validateFixture() {
    const requiredMarkers = [
        'component-library.json',
        'verify-figma-inventory',
        '`component-set`',
        '`component`',
        '`payload`',
        '`structural`',
        '`cem-action`',
        '`cem-icon`',
        '`cem-tree-item`',
        '`cem-stack`',
        'Light',
        'Dark',
        'Contrast Light',
        'Contrast Dark',
        'Native',
        'Deliberate rejection cases',
    ];
    for (const marker of requiredMarkers) {
        if (!fixture.includes(marker)) fail(`component-library-fixture.md missing review marker ${marker}`);
    }
}

function expectedStatesFor(tag) {
    if (payloadTags.has(tag) || structuralTags.has(tag)) return [];
    return coveredStates.get(tag) ?? [];
}

function representationFor(tag, states, properties) {
    if (payloadTags.has(tag)) return 'payload';
    if (structuralTags.has(tag)) return 'structural';
    if (states.length > 1 || Object.values(properties ?? {}).includes('variant')) return 'component-set';
    return 'component';
}

function coveredStatesByComponent(matrix) {
    const result = new Map();
    const records = Array.isArray(matrix?.coverage) ? matrix.coverage : [];
    for (const record of records) {
        if (record.status !== 'covered' || !Array.isArray(record.components)) continue;
        for (const tag of record.components) {
            const states = result.get(tag) ?? [];
            if (!states.includes(record.state)) states.push(record.state);
            result.set(tag, states);
        }
    }
    return result;
}

function parseMvpComponents(markdown) {
    const components = [];
    let inTable = false;
    for (const line of markdown.split(/\r?\n/u)) {
        if (line.startsWith('| Category | Component ID | Element name |')) {
            inTable = true;
            continue;
        }
        if (!inTable) continue;
        if (line.startsWith('| ---')) continue;
        if (!line.startsWith('|')) break;
        const cells = tableCells(line);
        if (cells.length !== 5) {
            fail(`component MVP row must have 5 cells: ${line}`);
            continue;
        }
        components.push({
            category: cells[0],
            id: stripCode(cells[1]),
            tag: stripCode(cells[2]),
            tokenFamilies: cells[4].split(',').map((family) => family.trim()).filter(Boolean),
        });
    }
    return components;
}

function parsePrimitiveDeclarations(sourceText) {
    const source = ts.createSourceFile(primitivesPath, sourceText, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
    const statement = source.statements.find(
        (candidate) =>
            ts.isVariableStatement(candidate) &&
            candidate.declarationList.declarations.some(
                (declaration) => ts.isIdentifier(declaration.name) && declaration.name.text === 'CEM_COMPONENT_PRIMITIVES',
            ),
    );
    const declaration = statement?.declarationList.declarations.find(
        (candidate) => ts.isIdentifier(candidate.name) && candidate.name.text === 'CEM_COMPONENT_PRIMITIVES',
    );
    const expression = unwrapExpression(declaration?.initializer);
    if (!expression || !ts.isArrayLiteralExpression(expression)) {
        fail('missing CEM_COMPONENT_PRIMITIVES array literal');
        return [];
    }
    return expression.elements.map((element, index) => parsePrimitive(element, index)).filter(Boolean);
}

function parsePrimitive(element, index) {
    if (!ts.isObjectLiteralExpression(element)) {
        fail(`primitive ${index} must be an object literal`);
        return undefined;
    }
    return {
        tag: propertyString(element, 'tag'),
        cemMl: propertyString(element, 'cemMl'),
    };
}

function parseReferenceRows(markdown) {
    const rows = new Map();
    for (const line of markdown.split(/\r?\n/u)) {
        if (!line.startsWith('| `cem-')) continue;
        const cells = tableCells(line);
        const tag = stripCode(cells[0] ?? '');
        if (rows.has(tag)) fail(`duplicate component-reference row ${tag}`);
        rows.set(tag, { line, cells });
    }
    return rows;
}

function propertyString(object, name) {
    const property = object.properties.find(
        (entry) =>
            ts.isPropertyAssignment(entry) &&
            ((ts.isIdentifier(entry.name) && entry.name.text === name) ||
                (ts.isStringLiteral(entry.name) && entry.name.text === name)),
    );
    return property && ts.isPropertyAssignment(property) ? expressionString(property.initializer) : '';
}

function expressionString(expression) {
    if (ts.isStringLiteral(expression) || ts.isNoSubstitutionTemplateLiteral(expression)) return expression.text;
    if (ts.isBinaryExpression(expression) && expression.operatorToken.kind === ts.SyntaxKind.PlusToken) {
        return `${expressionString(expression.left)}${expressionString(expression.right)}`;
    }
    return '';
}

function unwrapExpression(expression) {
    if (!expression) return undefined;
    if (ts.isSatisfiesExpression(expression) || ts.isAsExpression(expression)) {
        return unwrapExpression(expression.expression);
    }
    return expression;
}

function documentationTextFor(references, label) {
    if (!Array.isArray(references)) return '';
    return references
        .map((reference) => {
            if (typeof reference !== 'string') return '';
            const [path] = reference.split('#');
            const absolute = resolveRepoPath(path, label);
            return absolute && existsSync(absolute) ? readText(absolute) : '';
        })
        .join('\n');
}

function validateDocumentationReference(reference, label) {
    if (typeof reference !== 'string' || !reference.trim()) {
        fail(`${label}: documentation references must be non-empty strings`);
        return;
    }
    const [path, anchor] = reference.split('#');
    const absolute = resolveRepoPath(path, label);
    if (!absolute || !existsSync(absolute)) {
        fail(`${label}: documentation does not exist at ${path}`);
        return;
    }
    if (anchor) {
        const headings = readText(absolute)
            .split(/\r?\n/u)
            .filter((line) => /^#{1,6}\s+/u.test(line))
            .map((line) => markdownAnchor(line.replace(/^#{1,6}\s+/u, '')));
        if (!headings.includes(anchor)) fail(`${label}: stale documentation anchor ${reference}`);
    }
}

function propertyHasPublicEvidence(source, evidenceText) {
    if (source === 'slot:default') return evidenceText.includes('{slot') || /default slot|projects? authored|content/i.test(evidenceText);
    if (source.startsWith('slot:')) {
        const name = source.slice('slot:'.length);
        return evidenceText.includes(`slot:${name}`) || evidenceText.includes(`slot="${name}"`) || evidenceText.includes(`@name=${name}`);
    }
    return evidenceText.includes(source);
}

function resolveRepoPath(path, label) {
    if (!path || path.startsWith('/') || path.split('/').includes('..')) {
        fail(`${label}: repository path must be relative and may not traverse: ${String(path)}`);
        return null;
    }
    const absolute = resolve(repoRoot, path);
    if (!absolute.startsWith(`${repoRoot}/`)) {
        fail(`${label}: repository path escapes the workspace: ${path}`);
        return null;
    }
    return absolute;
}

function renderMarkdownReport(report) {
    const lines = [
        '# Figma Component Library Inventory',
        '',
        `- Public primitives: ${report.summary.total}`,
        `- Component sets: ${report.summary.componentSets}`,
        `- Components: ${report.summary.components}`,
        `- Inert payloads: ${report.summary.payloads}`,
        `- Structural owners: ${report.summary.structural}`,
        `- Reviewed: ${report.summary.reviewed}`,
        `- Planned: ${report.summary.planned}`,
        '',
        '| Primitive | Category | Representation | Properties | Executable states | Token families | Figma evidence |',
        '| --- | --- | --- | ---: | --- | --- | --- |',
    ];
    for (const entry of report.components) {
        lines.push(
            `| \`${entry.tag}\` | ${entry.category} | ${entry.representation} | ${Object.keys(entry.properties).length} | ` +
                `${entry.states.join(', ') || '—'} | ${entry.tokenFamilies.join(', ')} | ${entry.figma.status}: ${entry.figma.locator} |`,
        );
    }
    lines.push('', '> Generated by `verify-cem-components-figma-inventory.mjs`. Do not edit by hand.', '');
    return lines.join('\n');
}

function validateExactList(label, actual, expected) {
    if (!sameJson(actual, expected)) {
        fail(`${label} must exactly match: ${expected.join(', ')}`);
    }
}

function duplicateValues(values) {
    const seen = new Set();
    const duplicates = new Set();
    for (const value of values) {
        if (seen.has(value)) duplicates.add(value);
        seen.add(value);
    }
    return [...duplicates];
}

function tableCells(line) {
    return line.slice(1, -1).split('|').map((cell) => cell.trim());
}

function stripCode(value) {
    return value.replace(/^`|`$/gu, '');
}

function markdownAnchor(value) {
    return value
        .toLowerCase()
        .replace(/[`*_]/gu, '')
        .replace(/[^a-z0-9\s-]/gu, '')
        .trim()
        .replace(/\s+/gu, '-');
}

function sameJson(actual, expected) {
    return JSON.stringify(actual) === JSON.stringify(expected);
}

function readJson(path) {
    try {
        return JSON.parse(readText(path));
    } catch (error) {
        fail(`cannot parse ${relative(repoRoot, path)}: ${error.message}`);
        return {};
    }
}

function readText(path) {
    try {
        return readFileSync(path, 'utf8');
    } catch (error) {
        fail(`cannot read ${relative(repoRoot, path)}: ${error.message}`);
        return '';
    }
}

function fail(message) {
    failures.push(message);
}
