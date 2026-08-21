import assert from 'node:assert/strict';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const classificationPath = resolve(projectRoot, 'tests/ui-classification.json');
const parityPath = resolve(workspaceRoot, 'packages/cem-components/tests/angular-material-parity.json');
const primitivesPath = resolve(workspaceRoot, 'packages/cem-components/src/lib/primitives.ts');
const documentationPath = resolve(workspaceRoot, 'docs/cem-studio-phase6.5-ui-classification.md');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-studio');
const requiredBehaviorIds = [
    'shell-command-bar',
    'responsive-workspace-layout',
    'compact-pane-navigation',
    'home-and-projects',
    'project-hierarchy',
    'explorer-reordering',
    'explorer-context-actions',
    'project-search',
    'source-editor-frame',
    'resource-identity-form',
    'run-workbench-form',
    'cli-command-roundtrip',
    'result-view-navigation',
    'structured-data-inspector',
    'diagnostics-panel',
    'report-event-source-trace',
    'safe-preview-frame',
    'transformation-graph-view',
    'run-status-and-cancellation',
    'clipboard-download-feedback',
    'confirmation-and-destructive-actions',
    'project-settings',
    'storage-offline-update-status',
];
const allowedDispositions = new Set([
    'reuse-general',
    'complete-general-first',
    'studio-export',
    'application-orchestration',
]);

const classification = await readJson(classificationPath);
const parity = await readJson(parityPath);
const primitivesSource = await readFile(primitivesPath, 'utf8');
const documentation = await readFile(documentationPath, 'utf8');
const parityById = new Map(parity.components.map((component) => [component.id, component]));
const generalComponents = new Set(
    [...primitivesSource.matchAll(/\btag:\s*'([^']+)'/g)].map((match) => match[1]),
);

assert.equal(classification.version, 1);
assert.equal(classification.scope, 'initial-studio-shell-and-workbench');
for (const field of ['product', 'version', 'tag', 'commit']) {
    assert.equal(
        classification.benchmark[field],
        parity.benchmark[field],
        `Studio UI classification benchmark ${field} drifted from the pinned parity inventory`,
    );
}
assert.ok(classification.principles.length >= 4, 'classification must preserve all application UI ownership principles');

const behaviorIds = classification.behaviors.map(({ id }) => id);
assert.deepEqual(
    [...new Set(behaviorIds)].sort(),
    [...requiredBehaviorIds].sort(),
    'Studio UI classification must contain every required behavior exactly once',
);

for (const behavior of classification.behaviors) {
    assert.ok(allowedDispositions.has(behavior.disposition), `${behavior.id} has an unknown disposition`);
    assert.ok(['open', 'blocked'].includes(behavior.compositionGate), `${behavior.id} has an unknown composition gate`);
    assert.ok(behavior.requirement.trim(), `${behavior.id} must state its required behavior`);
    assert.ok(behavior.surface.trim(), `${behavior.id} must name its Studio surface`);
    assert.ok(behavior.contractBounds.length > 0, `${behavior.id} must preserve an explicit component boundary`);
    assert.ok(behavior.applicationOwns.length > 0, `${behavior.id} must state application-owned orchestration`);
    assert.ok(behavior.evidence.length > 0, `${behavior.id} must cite contract evidence`);

    assert.deepEqual(
        behavior.generalComponents,
        [...new Set(behavior.generalComponents)],
        `${behavior.id} repeats a general component owner`,
    );
    for (const component of behavior.generalComponents) {
        assert.ok(generalComponents.has(component), `${behavior.id} references unknown general component ${component}`);
    }
    for (const component of behavior.studioComponents) {
        assert.match(component, /^cem-studio-[a-z0-9-]+$/, `${behavior.id} has an invalid /studio component identity`);
        assert.equal(
            generalComponents.has(component),
            false,
            `${behavior.id} cannot replace an existing general CEM component with a /studio owner`,
        );
    }

    assert.deepEqual(
        behavior.materialReferences,
        [...new Set(behavior.materialReferences)],
        `${behavior.id} repeats an Angular Material reference`,
    );
    for (const materialId of behavior.materialReferences) {
        const material = parityById.get(materialId);
        assert.ok(material, `${behavior.id} references unknown Angular Material row ${materialId}`);
        assert.ok(
            ['covered', 'partial'].includes(material.status),
            `${behavior.id} references unresolved Angular Material row ${materialId}`,
        );
    }

    if (behavior.disposition === 'complete-general-first') {
        assert.equal(behavior.compositionGate, 'blocked', `${behavior.id} must remain blocked until general parity lands`);
        assert.ok(
            behavior.materialReferences.some((id) => parityById.get(id).status === 'partial'),
            `${behavior.id} must identify the partial general parity row that blocks composition`,
        );
    } else {
        assert.equal(behavior.compositionGate, 'open', `${behavior.id} has no recorded general parity blocker`);
    }

    if (behavior.disposition === 'studio-export') {
        assert.ok(behavior.studioComponents.length > 0, `${behavior.id} must reserve a /studio component`);
    } else {
        assert.deepEqual(behavior.studioComponents, [], `${behavior.id} cannot declare a /studio component`);
    }

    for (const evidence of behavior.evidence) {
        const evidencePath = resolve(workspaceRoot, evidence.split('#')[0]);
        assert.ok((await stat(evidencePath)).isFile(), `${behavior.id} evidence does not exist: ${evidence}`);
    }
    assert.ok(
        documentation.includes(`| \`${behavior.id}\` |`),
        `classification documentation is missing behavior ${behavior.id}`,
    );
}

const blockedBehaviorIds = classification.behaviors
    .filter(({ compositionGate }) => compositionGate === 'blocked')
    .map(({ id }) => id)
    .sort();
const gatedBehaviorIds = [];
const gateMaterialIds = new Set();
for (const gate of classification.generalParityGates) {
    const material = parityById.get(gate.materialId);
    assert.ok(material, `general parity gate references unknown row ${gate.materialId}`);
    assert.equal(material.status, 'partial', `general parity gate ${gate.materialId} no longer matches a partial row`);
    assert.ok(gate.missingBehavior.length > 0, `general parity gate ${gate.materialId} must state missing behavior`);
    assert.ok(gate.nextAction.trim(), `general parity gate ${gate.materialId} must state its next action`);
    assert.equal(gateMaterialIds.has(gate.materialId), false, `duplicate general parity gate ${gate.materialId}`);
    gateMaterialIds.add(gate.materialId);
    gatedBehaviorIds.push(...gate.blocks);
}
assert.deepEqual(
    [...new Set(gatedBehaviorIds)].sort(),
    blockedBehaviorIds,
    'every blocked Studio behavior must be owned by one recorded general parity gate',
);

const deferredMaterialIds = new Set();
for (const deferred of classification.deferredGeneralCapabilities) {
    const material = parityById.get(deferred.materialId);
    assert.ok(material, `deferred capability references unknown row ${deferred.materialId}`);
    assert.equal(material.status, 'partial', `deferred capability ${deferred.materialId} no longer matches a partial row`);
    assert.ok(deferred.initialBoundary.trim(), `deferred capability ${deferred.materialId} must preserve its initial boundary`);
    assert.equal(deferredMaterialIds.has(deferred.materialId), false, `duplicate deferred capability ${deferred.materialId}`);
    assert.equal(gateMaterialIds.has(deferred.materialId), false, `${deferred.materialId} cannot be both blocking and deferred`);
    deferredMaterialIds.add(deferred.materialId);
}

const dispositionCounts = Object.fromEntries(
    [...allowedDispositions].map((disposition) => [
        disposition,
        classification.behaviors.filter((behavior) => behavior.disposition === disposition).length,
    ]),
);
const report = {
    schemaVersion: 1,
    scope: classification.scope,
    benchmark: classification.benchmark,
    behaviorCount: classification.behaviors.length,
    dispositionCounts,
    openBehaviorCount: classification.behaviors.length - blockedBehaviorIds.length,
    blockedBehaviors: blockedBehaviorIds,
    generalParityGates: [...gateMaterialIds].sort(),
    deferredGeneralCapabilities: [...deferredMaterialIds].sort(),
    reservedStudioComponents: classification.behaviors
        .flatMap(({ studioComponents }) => studioComponents)
        .sort(),
};
const markdown = `# CEM Studio UI Classification Report

- Benchmark: ${report.benchmark.product} ${report.benchmark.version} (${report.benchmark.commit})
- Classified behaviors: ${report.behaviorCount}
- Open classifications: ${report.openBehaviorCount}
- Blocked behaviors: ${report.blockedBehaviors.join(', ')}
- General parity gates: ${report.generalParityGates.join(', ')}
- Reserved \`/studio\` composites: ${report.reservedStudioComponents.length}

| Disposition | Count |
| --- | ---: |
${Object.entries(report.dispositionCounts)
    .map(([disposition, count]) => `| ${disposition} | ${count} |`)
    .join('\n')}
`;

await mkdir(reportRoot, { recursive: true });
await writeFile(resolve(reportRoot, 'ui-classification.json'), `${JSON.stringify(report, null, 2)}\n`, 'utf8');
await writeFile(resolve(reportRoot, 'ui-classification.md'), markdown, 'utf8');
console.log(
    `Verified ${report.behaviorCount} Studio UI classifications: ${report.openBehaviorCount} open, ` +
        `${report.blockedBehaviors.length} blocked by ${report.generalParityGates.join(', ') || 'none'}.`,
);

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}
