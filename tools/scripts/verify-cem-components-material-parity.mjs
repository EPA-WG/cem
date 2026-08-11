#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const inventoryPath = resolve(repoRoot, 'packages/cem-components/tests/angular-material-parity.json');
const primitivesPath = resolve(repoRoot, 'packages/cem-components/src/lib/primitives.ts');
const failures = [];

const EXPECTED_BENCHMARK = {
    product: 'Angular Material',
    version: '22.1.1',
    tag: 'v22.1.1',
    commit: '0b67c3c38141049657b1167479accc80e455d2bd',
    capturedAt: '2026-08-10',
    catalogUrl: 'https://material.angular.dev/components/categories',
    sourceUrl:
        'https://github.com/angular/components/blob/v22.1.1/docs/src/app/shared/documentation-items/documentation-items.ts',
};
const EXPECTED_CATALOG = [
    ['autocomplete', 'Autocomplete'],
    ['badge', 'Badge'],
    ['bottom-sheet', 'Bottom Sheet'],
    ['button', 'Button'],
    ['button-toggle', 'Button Toggle'],
    ['card', 'Card'],
    ['checkbox', 'Checkbox'],
    ['chips', 'Chips'],
    ['core', 'Core'],
    ['datepicker', 'Datepicker'],
    ['dialog', 'Dialog'],
    ['divider', 'Divider'],
    ['expansion', 'Expansion Panel'],
    ['form-field', 'Form Field'],
    ['grid-list', 'Grid List'],
    ['icon', 'Icon'],
    ['input', 'Input'],
    ['list', 'List'],
    ['menu', 'Menu'],
    ['paginator', 'Paginator'],
    ['progress-bar', 'Progress Bar'],
    ['progress-spinner', 'Progress Spinner'],
    ['radio', 'Radio Button'],
    ['ripple', 'Ripples'],
    ['select', 'Select'],
    ['sidenav', 'Sidenav'],
    ['slide-toggle', 'Slide Toggle'],
    ['slider', 'Slider'],
    ['snack-bar', 'Snackbar'],
    ['sort', 'Sort Header'],
    ['stepper', 'Stepper'],
    ['table', 'Table'],
    ['tabs', 'Tabs'],
    ['timepicker', 'Timepicker'],
    ['toolbar', 'Toolbar'],
    ['tooltip', 'Tooltip'],
    ['tree', 'Tree'],
];
const ALLOWED_STATUSES = new Set(['unreviewed', 'gap', 'partial', 'covered']);
const ALLOWED_MAPPING_KINDS = new Set(['component', 'behavior', 'gap']);
const EXPECTED_IMPLEMENTATION_PRIORITY = {
    id: 'paginator',
    acceptedAt: '2026-08-11',
    completedAt: '2026-08-11',
    contract: 'packages/cem-components/docs/paginator-contract.md',
    state: 'completed',
    targetStatus: 'covered',
};

const inventory = JSON.parse(readFileSync(inventoryPath, 'utf8'));
const primitiveSource = readFileSync(primitivesPath, 'utf8');
const primitiveTags = new Set([...primitiveSource.matchAll(/\btag:\s*'([^']+)'/g)].map((match) => match[1]));
const records = Array.isArray(inventory.components) ? inventory.components : [];

if (inventory.version !== 1) {
    fail('Angular Material parity inventory version must be 1');
}
for (const [field, expectedValue] of Object.entries(EXPECTED_BENCHMARK)) {
    if (inventory.benchmark?.[field] !== expectedValue) {
        fail(`benchmark.${field} must equal ${expectedValue}`);
    }
}
if (records.length !== EXPECTED_CATALOG.length) {
    fail(`inventory must contain exactly ${EXPECTED_CATALOG.length} pinned catalog rows, received ${records.length}`);
}

const seen = new Set();
for (const [index, [expectedId, expectedName]] of EXPECTED_CATALOG.entries()) {
    const record = records[index];
    if (!record || typeof record !== 'object' || Array.isArray(record)) {
        fail(`catalog row ${index} must be an object for ${expectedId}`);
        continue;
    }
    if (record.id !== expectedId) {
        fail(`catalog row ${index} must be ${expectedId}, received ${String(record.id)}`);
    }
    if (record.name !== expectedName) {
        fail(`${expectedId}: name must be ${expectedName}, received ${String(record.name)}`);
    }
    if (seen.has(record.id)) {
        fail(`duplicate catalog row ${String(record.id)}`);
    }
    seen.add(record.id);
    validateRecord(record);
}

const expectedRecommendedAudit = records.find((record) => record?.status === 'unreviewed')?.id ?? null;
if (inventory.recommendedAudit !== expectedRecommendedAudit) {
    fail(`recommendedAudit must be the first unreviewed row (${String(expectedRecommendedAudit)})`);
}
if (records.some((record) => record?.status === 'unreviewed')) {
    fail('the pinned v22.1.1 catalog audit must not contain unreviewed rows');
}
validateImplementationPriority(inventory.implementationPriority);

if (failures.length > 0) {
    for (const failure of failures) {
        console.error(`error: ${failure}`);
    }
    process.exit(1);
}

const counts = Object.fromEntries(
    [...ALLOWED_STATUSES].map((status) => [status, records.filter((record) => record.status === status).length]),
);
console.log(
    `cem-components Angular Material parity inventory verified (${records.length} entries pinned to ` +
        `${inventory.benchmark.tag}: ${counts.covered} covered, ${counts.partial} partial, ${counts.gap} gaps, ` +
        `${counts.unreviewed} unreviewed; next audit: ${inventory.recommendedAudit ?? 'none'}; next implementation: ` +
        `${inventory.implementationPriority.state === 'completed' ? 'none (selection required)' : inventory.implementationPriority.id}).`,
);

function validateImplementationPriority(priority) {
    if (!priority || typeof priority !== 'object' || Array.isArray(priority)) {
        fail('implementationPriority must be an accepted priority object');
        return;
    }
    for (const [field, expectedValue] of Object.entries(EXPECTED_IMPLEMENTATION_PRIORITY)) {
        if (priority[field] !== expectedValue) {
            fail(`implementationPriority.${field} must equal ${expectedValue}`);
        }
    }
    const record = records.find((candidate) => candidate?.id === priority.id);
    if (!record || record.status !== priority.targetStatus || record.mapping?.kind === 'gap') {
        fail(`completed implementationPriority ${String(priority.id)} must achieve its non-gap target status`);
    }
    const contractPath = resolve(repoRoot, String(priority.contract ?? ''));
    if (!existsSync(contractPath)) {
        fail(`implementationPriority contract does not exist: ${String(priority.contract)}`);
        return;
    }
    const contractSource = readFileSync(contractPath, 'utf8');
    for (const heading of [
        '# Paginator Contract',
        '## Owner and author vocabulary',
        '## State and range contract',
        '## Event and keyboard contract',
        '## Accessibility contract',
        '## Theme-token audit',
        '## Forced-colors boundary',
        '## Focused fixture and assertion matrix',
    ]) {
        if (!contractSource.includes(heading)) {
            fail(`implementationPriority contract must contain ${heading}`);
        }
    }
}

function validateRecord(record) {
    const label = typeof record.id === 'string' && record.id ? record.id : 'unknown row';
    if (!ALLOWED_STATUSES.has(record.status)) {
        fail(`${label}: status must be unreviewed, gap, partial, or covered`);
        return;
    }
    if (record.status === 'unreviewed') {
        if (record.mapping !== null) {
            fail(`${label}: unreviewed row must keep mapping null`);
        }
        return;
    }
    if (!record.mapping || typeof record.mapping !== 'object' || Array.isArray(record.mapping)) {
        fail(`${label}: reviewed row must provide a mapping object`);
        return;
    }

    const mapping = record.mapping;
    if (!ALLOWED_MAPPING_KINDS.has(mapping.kind)) {
        fail(`${label}: mapping kind must be component, behavior, or gap`);
    }
    for (const field of ['owners', 'states', 'keyboard', 'accessibility', 'evidence']) {
        if (!Array.isArray(mapping[field])) {
            fail(`${label}: mapping.${field} must be an array`);
        }
    }
    if (typeof mapping.notes !== 'string' || !mapping.notes.trim()) {
        fail(`${label}: reviewed mapping must explain its semantic boundary in notes`);
    }

    const owners = Array.isArray(mapping.owners) ? mapping.owners : [];
    const evidence = Array.isArray(mapping.evidence) ? mapping.evidence : [];
    if (mapping.kind === 'gap') {
        if (record.status !== 'gap') {
            fail(`${label}: gap mapping kind requires gap status`);
        }
        if (owners.length > 0 || evidence.length > 0) {
            fail(`${label}: gap mapping must not claim owners or executable evidence`);
        }
    } else {
        if (record.status === 'gap') {
            fail(`${label}: component or behavior mapping cannot use gap status`);
        }
        if (owners.length === 0) {
            fail(`${label}: ${mapping.kind} mapping must name at least one owner`);
        }
        for (const owner of owners) {
            if (mapping.kind === 'component' && !primitiveTags.has(owner)) {
                fail(`${label}: component owner ${String(owner)} is not a public CEM_COMPONENT_PRIMITIVES tag`);
            }
            if (mapping.kind === 'behavior' && (typeof owner !== 'string' || !owner.startsWith('behavior:'))) {
                fail(`${label}: behavior owner ${String(owner)} must use a behavior: identity`);
            }
        }
        if (evidence.length === 0) {
            fail(`${label}: ${record.status} mapping must name executable product-layer evidence`);
        }
        for (const reference of evidence) {
            validateEvidence(label, reference);
        }
    }

    for (const field of ['states', 'keyboard', 'accessibility']) {
        if (!Array.isArray(mapping[field]) || mapping[field].length === 0) {
            fail(`${label}: reviewed mapping must document ${field}`);
        }
    }
}

function validateEvidence(label, reference) {
    if (typeof reference !== 'string' || !reference.startsWith('packages/cem-components/')) {
        fail(`${label}: evidence must be rooted in packages/cem-components, received ${String(reference)}`);
        return;
    }
    const separatorIndex = reference.indexOf('::');
    const path = separatorIndex === -1 ? reference : reference.slice(0, separatorIndex);
    const locator = separatorIndex === -1 ? '' : reference.slice(separatorIndex + 2);
    const absolutePath = resolve(repoRoot, path);
    if (!existsSync(absolutePath)) {
        fail(`${label}: evidence path does not exist: ${path}`);
        return;
    }
    if (separatorIndex !== -1 && !locator) {
        fail(`${label}: evidence locator must not be empty: ${reference}`);
        return;
    }
    if (locator && !readFileSync(absolutePath, 'utf8').includes(locator)) {
        fail(`${label}: evidence locator does not exist in ${path}: ${locator}`);
    }
}

function fail(message) {
    failures.push(message);
}
