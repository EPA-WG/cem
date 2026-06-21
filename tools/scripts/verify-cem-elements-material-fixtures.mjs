#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const fixtureDir = join(repoRoot, 'packages/cem-elements/tests/parity/material');
const manifestPath = join(fixtureDir, 'manifest.json');

const expectedOrder = ['icon', 'icon-link', 'menu', 'badge', 'action', 'dropdown', 'input', 'autocomplete'];
const expectedTags = new Map([
    ['icon', 'cem-icon'],
    ['icon-link', 'cem-icon-link'],
    ['menu', 'cem-menu'],
    ['badge', 'cem-badge'],
    ['action', 'cem-action'],
    ['dropdown', 'cem-dropdown'],
    ['input', 'cem-input'],
    ['autocomplete', 'cem-autocomplete'],
]);
const expectedImports = new Map([
    ['icon', ['icon-link']],
    ['icon-link', []],
    ['menu', ['icon-link']],
    ['badge', ['icon-link', 'icon']],
    ['action', ['icon-link', 'icon']],
    ['dropdown', ['icon-link', 'menu']],
    ['input', ['icon-link', 'icon']],
    ['autocomplete', ['icon-link', 'input', 'menu']],
]);

function fail(message) {
    console.error(`error: ${message}`);
    process.exitCode = 1;
}

function readFixture(relativePath) {
    const filePath = join(fixtureDir, relativePath);
    if (!existsSync(filePath)) {
        fail(`missing fixture file ${relativePath}`);
        return '';
    }
    return readFileSync(filePath, 'utf8');
}

function sameArray(actual, expected) {
    return actual.length === expected.length && actual.every((value, index) => value === expected[index]);
}

if (!existsSync(manifestPath)) {
    fail(`missing manifest ${manifestPath}`);
    process.exit(1);
}

const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
if (manifest.version !== 1) fail('manifest version must be 1');
if (!sameArray(manifest.importOrder ?? [], expectedOrder)) {
    fail(`manifest importOrder must be ${expectedOrder.join(', ')}`);
}
if (!Array.isArray(manifest.fixtures)) fail('manifest fixtures must be an array');

const seen = new Set();
for (const fixture of manifest.fixtures ?? []) {
    if (!expectedTags.has(fixture.id)) fail(`unexpected fixture id ${fixture.id}`);
    if (seen.has(fixture.id)) fail(`duplicate fixture id ${fixture.id}`);
    seen.add(fixture.id);

    const expectedTag = expectedTags.get(fixture.id);
    if (fixture.tag !== expectedTag) fail(`${fixture.id}: expected tag ${expectedTag}`);
    if (!fixture.legacy) fail(`${fixture.id}: missing legacy file`);
    if (!fixture.cemMl) fail(`${fixture.id}: missing CEM-ML twin file`);
    if (!sameArray(fixture.imports ?? [], expectedImports.get(fixture.id) ?? [])) {
        fail(`${fixture.id}: imports do not match material inventory`);
    }

    const legacy = readFixture(fixture.legacy);
    const cemMl = readFixture(fixture.cemMl);
    if (legacy && !legacy.includes('<custom-element')) fail(`${fixture.legacy}: missing <custom-element declaration`);
    if (legacy && !legacy.includes(expectedTag)) fail(`${fixture.legacy}: missing ${expectedTag}`);
    if (cemMl && !cemMl.includes('<cem-element')) fail(`${fixture.cemMl}: missing <cem-element declaration`);
    if (cemMl && !cemMl.includes(`tag="${expectedTag}"`)) fail(`${fixture.cemMl}: missing tag="${expectedTag}"`);
    if (cemMl && !cemMl.includes('type="text/cem-ml"')) fail(`${fixture.cemMl}: missing text/cem-ml template`);

    for (const imported of fixture.imports ?? []) {
        const importedTag = expectedTags.get(imported);
        if (!legacy.includes(importedTag)) fail(`${fixture.id}: legacy fixture missing imported ${importedTag}`);
    }

    const surface = `${legacy}\n${cemMl}`;
    for (const marker of fixture.markers ?? []) {
        if (!surface.includes(marker)) {
            fail(`${fixture.id}: marker ${JSON.stringify(marker)} not found in paired fixtures`);
        }
    }
}

for (const id of expectedOrder) {
    if (!seen.has(id)) fail(`missing fixture for ${id}`);
}

if (process.exitCode) {
    process.exit(process.exitCode);
}

console.log(`cem-elements material parity fixtures verified (${seen.size} fixtures).`);
