#!/usr/bin/env node

import { existsSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const fixtureDir = join(repoRoot, 'packages/cem-elements/tests/parity/legacy');
const manifestPath = join(fixtureDir, 'manifest.json');

const requiredBehaviors = [
    'declaration-registration',
    'inline-template-shape',
    'local-src',
    'external-src',
    'payload-capture',
    'attribute-defaults-overrides',
    'attribute-invalidation',
    'slots',
    'slice-events',
    'datadom-access-migration',
    'conditionals',
    'legacy-bridge',
];

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

if (!existsSync(manifestPath)) {
    fail(`missing manifest ${manifestPath}`);
    process.exit(1);
}

const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
if (manifest.version !== 1) fail('manifest version must be 1');
if (!Array.isArray(manifest.fixtures)) fail('manifest fixtures must be an array');

const declaredRequired = new Set(manifest.requiredBehaviors ?? []);
for (const behavior of requiredBehaviors) {
    if (!declaredRequired.has(behavior)) fail(`manifest requiredBehaviors missing ${behavior}`);
}

const seen = new Set();
for (const fixture of manifest.fixtures ?? []) {
    if (!requiredBehaviors.includes(fixture.id)) fail(`unexpected fixture id ${fixture.id}`);
    if (seen.has(fixture.id)) fail(`duplicate fixture id ${fixture.id}`);
    seen.add(fixture.id);
    if (!fixture.behavior) fail(`${fixture.id}: missing behavior text`);
    if (!fixture.legacy) fail(`${fixture.id}: missing legacy file`);
    if (!fixture.cemMl) fail(`${fixture.id}: missing CEM-ML twin file`);

    const legacy = readFixture(fixture.legacy);
    const cemMl = readFixture(fixture.cemMl);
    const support = (fixture.supportFiles ?? []).map((file) => readFixture(file)).join('\n');
    const legacySurface = `${legacy}\n${support}`;
    const cemMlSurface = `${cemMl}\n${support}`;
    if (legacy && !legacy.includes('<cem-element')) fail(`${fixture.legacy}: missing <cem-element declaration`);
    if (legacySurface && !legacySurface.includes('<template')) fail(`${fixture.id}: missing template`);
    if (cemMlSurface && !cemMlSurface.includes('type="text/cem-ml"')) fail(`${fixture.id}: missing text/cem-ml template`);

    const legacyTemplateTags = Array.from(
        legacySurface.matchAll(/<template\b[^>]*>/g),
        (match) => match[0]
    ).filter((tag) => !tag.includes('type="text/cem-ml"'));
    if (legacyTemplateTags.length === 0) fail(`${fixture.id}: missing legacy template`);
    for (const tag of legacyTemplateTags) {
        if (!tag.includes('lang="custom-element-v0"')) {
            fail(`${fixture.id}: legacy template must opt in with lang="custom-element-v0"`);
        }
    }

    for (const marker of fixture.markers ?? []) {
        if (!legacySurface.includes(marker) && !cemMlSurface.includes(marker)) {
            fail(`${fixture.id}: marker ${JSON.stringify(marker)} not found in paired fixtures`);
        }
    }
}

for (const behavior of requiredBehaviors) {
    if (!seen.has(behavior)) fail(`missing fixture for ${behavior}`);
}

if (process.exitCode) {
    process.exit(process.exitCode);
}

console.log(`cem-elements legacy parity fixtures verified (${seen.size} fixtures).`);
