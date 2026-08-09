#!/usr/bin/env node

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import ts from 'typescript';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const mvpPath = join(repoRoot, 'docs/component-mvp.md');
const inventoryPath = join(repoRoot, 'packages/cem-components/tests/state-matrix-coverage.json');
const reportDirectory = join(repoRoot, 'packages/cem-components/dist/reports');
const reportJsonPath = join(reportDirectory, 'component-state-matrix.json');
const reportMarkdownPath = join(reportDirectory, 'component-state-matrix.md');
const allowedStatuses = new Set(['covered', 'static-only', 'gap']);
const failures = [];

const markdown = readText(mvpPath);
const inventory = JSON.parse(readText(inventoryPath));
const componentTagsByCategory = parseComponentTagsByCategory(markdown);
const requiredStatesByCategory = parseCategoryStates(markdown);
const expectedIds = new Set(
    [...requiredStatesByCategory].flatMap(([category, states]) =>
        states.map((state) => `${category.toLowerCase()}:${state}`),
    ),
);
const records = Array.isArray(inventory.coverage) ? inventory.coverage : [];
const recordsById = new Map();

if (inventory.version !== 1) {
    fail('state matrix coverage inventory version must be 1');
}
if (inventory.source !== 'docs/component-mvp.md#category-state-coverage') {
    fail('state matrix coverage inventory must name docs/component-mvp.md#category-state-coverage as its source');
}

for (const [index, record] of records.entries()) {
    validateRecord(record, index);
}

for (const id of expectedIds) {
    if (!recordsById.has(id)) {
        fail(`missing state matrix coverage row ${id}`);
    }
}
for (const id of recordsById.keys()) {
    if (!expectedIds.has(id)) {
        fail(`state matrix coverage row ${id} is not required by docs/component-mvp.md`);
    }
}

const priority = Array.isArray(inventory.priority) ? inventory.priority : [];
for (const id of priority) {
    if (!expectedIds.has(id)) {
        fail(`priority row ${id} is not required by docs/component-mvp.md`);
    }
}
const expectedNext = priority.find((id) => recordsById.get(id)?.status !== 'covered');
if (!expectedNext) {
    fail('priority must contain at least one uncovered state requirement');
} else if (inventory.recommendedNext !== expectedNext) {
    fail(`recommendedNext must be the first uncovered priority row (${expectedNext})`);
}

if (failures.length > 0) {
    for (const failure of failures) {
        console.error(`error: ${failure}`);
    }
    process.exit(1);
}

const summary = {
    total: records.length,
    covered: records.filter((record) => record.status === 'covered').length,
    staticOnly: records.filter((record) => record.status === 'static-only').length,
    gaps: records.filter((record) => record.status === 'gap').length,
};
const report = {
    version: inventory.version,
    source: inventory.source,
    summary,
    recommendedNext: inventory.recommendedNext,
    coverage: records,
};

mkdirSync(reportDirectory, { recursive: true });
writeFileSync(reportJsonPath, `${JSON.stringify(report, null, 4)}\n`);
writeFileSync(reportMarkdownPath, renderMarkdownReport(report));

for (const record of records.filter((entry) => entry.status !== 'covered')) {
    console.log(`${record.status}: ${record.id} - ${record.reason}`);
}
console.log(
    `cem-components state matrix verified (${summary.total} requirements: ${summary.covered} browser-covered, ` +
        `${summary.staticOnly} static-only, ${summary.gaps} gaps; next: ${inventory.recommendedNext}).`,
);

function validateRecord(record, index) {
    if (!record || typeof record !== 'object' || Array.isArray(record)) {
        fail(`coverage row ${index} must be an object`);
        return;
    }

    const label = typeof record.id === 'string' ? record.id : `coverage row ${index}`;
    if (typeof record.id !== 'string' || !record.id) {
        fail(`coverage row ${index} must have an id`);
        return;
    }
    if (recordsById.has(record.id)) {
        fail(`duplicate state matrix coverage row ${record.id}`);
    } else {
        recordsById.set(record.id, record);
    }

    const expectedId =
        typeof record.category === 'string' && typeof record.state === 'string'
            ? `${record.category.toLowerCase()}:${record.state}`
            : '';
    if (record.id !== expectedId) {
        fail(`${label}: id must match the lowercased category and state (${expectedId || 'missing category/state'})`);
    }

    const allowedComponents = componentTagsByCategory.get(record.category);
    if (!allowedComponents) {
        fail(`${label}: unknown component category ${String(record.category)}`);
    }
    if (!requiredStatesByCategory.get(record.category)?.includes(record.state)) {
        fail(`${label}: state is not required for ${String(record.category)} by docs/component-mvp.md`);
    }
    if (!Array.isArray(record.components) || record.components.length === 0) {
        fail(`${label}: components must name at least one affected or evidenced component`);
    } else {
        for (const component of new Set(record.components)) {
            if (!allowedComponents?.has(component)) {
                fail(`${label}: ${component} is not a ${String(record.category)} component in docs/component-mvp.md`);
            }
        }
        if (new Set(record.components).size !== record.components.length) {
            fail(`${label}: components must not contain duplicates`);
        }
    }
    if (typeof record.interaction !== 'string' || !record.interaction.trim()) {
        fail(`${label}: interaction or transition description is required`);
    }
    if (!allowedStatuses.has(record.status)) {
        fail(`${label}: status must be covered, static-only, or gap`);
        return;
    }

    if (record.status === 'covered') {
        validateCoveredRecord(record);
    } else if (record.status === 'static-only') {
        validateStaticOnlyRecord(record);
    } else {
        validateGapRecord(record);
    }
}

function validateCoveredRecord(record) {
    const label = record.id;
    if (typeof record.owner !== 'string' || !record.owner.endsWith('.browser.spec.ts')) {
        fail(`${label}: covered owner must be an exact .browser.spec.ts path`);
        return;
    }
    if (typeof record.test !== 'string' || !record.test) {
        fail(`${label}: covered row must name an exact browser test`);
        return;
    }
    if (!Array.isArray(record.assertions) || record.assertions.length === 0) {
        fail(`${label}: covered row must name at least one exact assertion`);
        return;
    }

    const ownerPath = resolveRepoPath(record.owner, label);
    if (!ownerPath || !existsSync(ownerPath)) {
        fail(`${label}: browser owner does not exist at ${record.owner}`);
        return;
    }
    const testBody = findBrowserTestBody(ownerPath, record.test);
    if (!testBody) {
        fail(`${label}: browser test not found in ${record.owner}: ${record.test}`);
        return;
    }
    for (const assertion of record.assertions) {
        if (typeof assertion !== 'string' || !assertion) {
            fail(`${label}: assertion references must be non-empty strings`);
        } else if (!testBody.includes(assertion)) {
            fail(`${label}: stale assertion reference in ${record.test}: ${assertion}`);
        }
    }
    if ('reason' in record) {
        fail(`${label}: covered row must not carry a gap reason`);
    }
}

function validateStaticOnlyRecord(record) {
    const label = record.id;
    if (typeof record.owner !== 'string' || !record.owner.endsWith('.html')) {
        fail(`${label}: static-only owner must be an exact HTML fixture path`);
        return;
    }
    if (!Array.isArray(record.assertions) || record.assertions.length === 0) {
        fail(`${label}: static-only row must name the authored state markers`);
        return;
    }
    if (typeof record.reason !== 'string' || !record.reason.trim()) {
        fail(`${label}: static-only row must explain the missing browser assertion`);
    }
    if ('test' in record) {
        fail(`${label}: static-only row must not claim a browser test`);
    }

    const ownerPath = resolveRepoPath(record.owner, label);
    if (!ownerPath || !existsSync(ownerPath)) {
        fail(`${label}: static fixture owner does not exist at ${record.owner}`);
        return;
    }
    const markup = readText(ownerPath);
    for (const marker of record.assertions) {
        if (typeof marker !== 'string' || !marker || !markup.includes(marker)) {
            fail(`${label}: stale static fixture marker in ${record.owner}: ${String(marker)}`);
        }
    }
}

function validateGapRecord(record) {
    const label = record.id;
    if (typeof record.reason !== 'string' || !record.reason.trim()) {
        fail(`${label}: gap row must explain the missing executable evidence`);
    }
    for (const forbidden of ['owner', 'test', 'assertions']) {
        if (forbidden in record) {
            fail(`${label}: gap row must not claim ${forbidden}`);
        }
    }
}

function parseComponentTagsByCategory(source) {
    const result = new Map();
    let inTable = false;

    for (const line of source.split(/\r?\n/)) {
        if (line.startsWith('| Category | Component ID | Element name |')) {
            inTable = true;
            continue;
        }
        if (!inTable) {
            continue;
        }
        if (line.startsWith('| ---')) {
            continue;
        }
        if (!line.startsWith('|')) {
            break;
        }

        const cells = tableCells(line);
        const category = cells[0];
        const tag = stripCode(cells[2]);
        if (!category || !tag) {
            fail(`invalid component MVP row: ${line}`);
            continue;
        }
        const tags = result.get(category) ?? new Set();
        tags.add(tag);
        result.set(category, tags);
    }

    return result;
}

function parseCategoryStates(source) {
    const result = new Map();
    let inTable = false;

    for (const line of source.split(/\r?\n/)) {
        if (line.startsWith('| Category | Required MVP states |')) {
            inTable = true;
            continue;
        }
        if (!inTable) {
            continue;
        }
        if (line.startsWith('| ---')) {
            continue;
        }
        if (!line.startsWith('|')) {
            break;
        }

        const cells = tableCells(line);
        const category = cells[0];
        const states = [...(cells[1] ?? '').matchAll(/`([^`]+)`/g)].map((match) => match[1]);
        if (!category || states.length === 0) {
            fail(`invalid category state row: ${line}`);
            continue;
        }
        result.set(category, states);
    }

    return result;
}

function findBrowserTestBody(path, title) {
    const sourceText = readText(path);
    const source = ts.createSourceFile(path, sourceText, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
    let body;

    function visit(node) {
        if (body) {
            return;
        }
        if (
            ts.isCallExpression(node) &&
            ts.isIdentifier(node.expression) &&
            node.expression.text === 'it' &&
            (ts.isStringLiteral(node.arguments[0]) || ts.isNoSubstitutionTemplateLiteral(node.arguments[0])) &&
            node.arguments[0].text === title
        ) {
            const callback = node.arguments[1];
            if (callback && (ts.isArrowFunction(callback) || ts.isFunctionExpression(callback))) {
                body = callback.body.getText(source);
                return;
            }
        }
        ts.forEachChild(node, visit);
    }

    visit(source);
    return body;
}

function renderMarkdownReport(report) {
    const lines = [
        '# CEM Component State-Matrix Coverage',
        '',
        `Source: \`${report.source}\``,
        '',
        `Summary: ${report.summary.covered} browser-covered, ${report.summary.staticOnly} static-only, ` +
            `${report.summary.gaps} gaps across ${report.summary.total} requirements.`,
        '',
        `Recommended next requirement: \`${report.recommendedNext}\`.`,
        '',
        '| Requirement | Components | Interaction / transition | Status | Executable owner |',
        '| --- | --- | --- | --- | --- |',
    ];

    for (const record of report.coverage) {
        const owner = record.status === 'covered' ? `\`${record.owner}\` — ${record.test}` : record.reason;
        lines.push(
            `| \`${record.id}\` | ${record.components.map((component) => `\`${component}\``).join(', ')} | ` +
                `${escapeTableCell(record.interaction)} | ${record.status} | ${escapeTableCell(owner)} |`,
        );
    }

    return `${lines.join('\n')}\n`;
}

function tableCells(line) {
    return line
        .slice(1, -1)
        .split('|')
        .map((cell) => cell.trim());
}

function stripCode(value = '') {
    return value.replace(/^`|`$/g, '');
}

function escapeTableCell(value) {
    return String(value).replaceAll('|', '\\|').replaceAll('\n', ' ');
}

function resolveRepoPath(path, label) {
    const resolved = resolve(repoRoot, path);
    const relativePath = relative(repoRoot, resolved);
    if (relativePath.startsWith('..') || relativePath === '') {
        fail(`${label}: owner must resolve to a repository file`);
        return undefined;
    }
    return resolved;
}

function readText(path) {
    return readFileSync(path, 'utf8');
}

function fail(message) {
    failures.push(message);
}
