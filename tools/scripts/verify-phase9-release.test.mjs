import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import test from 'node:test';

import { gatherWorkspaceState, validatePhase9Contract } from './verify-phase9-release.mjs';

const workspaceRoot = resolve(import.meta.dirname, '../..');
const contract = JSON.parse(
    await readFile(resolve(workspaceRoot, 'tools/fitness/phase9-release-contract.json'), 'utf8'),
);
const fixtures = JSON.parse(
    await readFile(resolve(workspaceRoot, 'tools/fitness/phase9-release-invalid-fixtures.json'), 'utf8'),
);
const baseline = await gatherWorkspaceState(contract, workspaceRoot);

test('credential-free Phase 9 readiness passes with public publication pending', () => {
    const result = validatePhase9Contract(contract, structuredClone(baseline), 'readiness');
    assert.deepEqual(result.errors, []);
    assert.ok(result.blockers.some((blocker) => blocker.includes('public release evidence pending')));
});

for (const fixture of fixtures.cases) {
    test(`rejects ${fixture.id}`, () => {
        const state = structuredClone(baseline);
        applyFixture(state, fixture);
        const result = validatePhase9Contract(contract, state, fixture.mode ?? 'readiness');
        assert.ok(
            result.errors.some((error) => error.includes(fixture.expected)),
            `expected ${JSON.stringify(result.errors)} to include ${JSON.stringify(fixture.expected)}`,
        );
    });
}

function applyFixture(state, fixture) {
    switch (fixture.kind) {
        case 'set-family-version':
            state.familyVersions[fixture.family][fixture.member] = fixture.value;
            return;
        case 'remove-export':
            state.packageExports[fixture.manifest] = state.packageExports[fixture.manifest].filter(
                (value) => value !== fixture.export,
            );
            return;
        case 'remove-deprecation-source':
            state.existingPaths = state.existingPaths.filter((path) => path !== fixture.path);
            return;
        case 'remove-workflow-text':
            state.workflowTexts[fixture.workflow] = state.workflowTexts[fixture.workflow].replace(fixture.text, '');
            return;
        case 'set-publication-evidence':
            state.publicationEvidence = fixture.value;
            return;
        default:
            throw new Error(`unsupported fixture kind ${fixture.kind}`);
    }
}
