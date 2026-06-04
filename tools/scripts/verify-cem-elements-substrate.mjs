#!/usr/bin/env node

/**
 * C2.6 verification gate — structural parse + roundtrip leg for `<cem-element>`
 * substrate templates.
 *
 * cem-element substrate templates use the cem-element authoring vocabulary
 * (`cem:if`/`cem:choose`, `<slot>`, `<attribute>`, `<data>`/`<option>`, `{$datadom…}`
 * host expressions). That vocabulary is not Tier-A HTML/SVG and its host bindings
 * resolve only at render time, so the semantic `validate` gate intentionally does
 * not accept it. This leg instead rides every substrate fixture through the real
 * `cem-ml` CLI `convert cem->cem` projection and asserts:
 *
 *   1. structural success — the fixture tokenizes, builds an AST, and serializes
 *      back to canonical CEM-ML (CLI exit 0); and
 *   2. roundtrip stability — the canonical form is idempotent (convert(convert(x))
 *      equals convert(x)), the same property `cem_ml_cli:e2e` checks for base
 *      fixtures.
 *
 * Semantic/render correctness of the same constructs is covered by the cem_ql
 * render tests and the `@epa-wg/cem-elements` Storybook parity stories.
 */

import { spawnSync } from 'node:child_process';
import { readdirSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const cli = join(repoRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const fixtureDir = join(repoRoot, 'examples/cem-elements');

function convertToCanonical(inputPath) {
    const result = spawnSync(cli, ['convert', '--from-format', 'cem', '--to-format', 'cem', inputPath], {
        cwd: repoRoot,
        encoding: 'utf8',
    });
    if (result.status !== 0) {
        throw new Error(`convert exited ${result.status} for ${inputPath}\n${result.stderr ?? ''}`);
    }
    const parsed = JSON.parse(result.stdout);
    if (typeof parsed.content !== 'string') {
        throw new Error(`convert produced no canonical content for ${inputPath}`);
    }
    return parsed.content;
}

const fixtures = readdirSync(fixtureDir)
    .filter((name) => name.endsWith('.cem'))
    .sort();

if (fixtures.length === 0) {
    console.error(`No substrate fixtures found in ${fixtureDir}`);
    process.exit(1);
}

const scratch = mkdtempSync(join(tmpdir(), 'cem-substrate-'));
const failures = [];

try {
    for (const fixture of fixtures) {
        const path = join(fixtureDir, fixture);
        try {
            const first = convertToCanonical(path);
            const roundtripPath = join(scratch, fixture);
            writeFileSync(roundtripPath, first);
            const second = convertToCanonical(roundtripPath);
            if (first !== second) {
                failures.push(`${fixture}: canonical roundtrip is not idempotent`);
                continue;
            }
            console.log(`  ok  ${fixture} — parses + roundtrips idempotently`);
        } catch (error) {
            failures.push(`${fixture}: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
} finally {
    rmSync(scratch, { recursive: true, force: true });
}

if (failures.length > 0) {
    console.error(`\ncem-element substrate verification failed:`);
    for (const failure of failures) {
        console.error(`  - ${failure}`);
    }
    process.exit(1);
}

console.log(`\ncem-element substrate verification passed (${fixtures.length} fixtures).`);
