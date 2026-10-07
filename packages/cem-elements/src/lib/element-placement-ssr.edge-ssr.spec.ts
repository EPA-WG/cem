import { readFile } from 'node:fs/promises';
import { beforeAll, expect, it } from 'vitest';
import { CemSsrPlacementCoordinator } from './element-placement-ssr.js';
import { elementReferenceFixtureInputs, ELEMENT_REFERENCE_TEMPLATE } from './element-reference-lifecycle.fixtures.js';
import { processRetainedCemMlTemplate, retainCemMlTemplateSource, disposeRetainedCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';
import { serializeRenderPlanToHtmlFixture } from './edge-ssr-host-fixture.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- retained native SSR ingress without browser globals.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
import { EXPERIMENTAL_ARIA_REFERENCE_PROFILE } from './aria-reference-profile.js';
function required<T>(value: T | undefined): T { if (value === undefined) throw new Error('Missing committed SSR fixture'); return value; }
beforeAll(async () => { await wasm.default({ module_or_path: await readFile(new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url)) }); });
it('exports the selected draft profile through native SSR plans and resume hints', async () => {
    const source = `{cem:variable @name=target @select='data:read("<div/>", "xml").root.children'}{div @aria-details={#(target, target)}}{$target}`;
    const artifact = await retainCemMlTemplateSource(source);
    const coordinator = new CemSsrPlacementCoordinator({ ariaReferenceProfile: EXPERIMENTAL_ARIA_REFERENCE_PROFILE });
    try {
        const identity = { producedTag: 'profile', instanceId: 'profile', templateArtifactId: 'profile', dataRevision: '1', outputTarget: 'light-dom' as const, scopePolicyStamp: 'scope', ariaReferenceProfile: EXPERIMENTAL_ARIA_REFERENCE_PROFILE };
        const result = await processRetainedCemMlTemplate(artifact.artifactId, { source, identity, data: {} });
        expect(result.renderPlan.ariaReferenceProfile).toBe(EXPERIMENTAL_ARIA_REFERENCE_PROFILE);
        expect(serializeRenderPlanToHtmlFixture(result.renderPlan)).toContain('aria-details="profile-ref-1 profile-ref-1"');
        const transaction = coordinator.transaction(['profile'], { profile: '1' });
        coordinator.publishGroup(transaction, [{ stage: coordinator.stage(result.renderPlan) }]);
        expect(coordinator.resumeHints().profile).toBe(EXPERIMENTAL_ARIA_REFERENCE_PROFILE);
        expect(() => coordinator.stage({ ...result.renderPlan, ariaReferenceProfile: undefined })).toThrow('profile');
        await expect(processRetainedCemMlTemplate(artifact.artifactId, { source, identity: { ...identity, ariaReferenceProfile: undefined }, data: {} })).rejects.toThrow('cardinality');
    } finally { coordinator.dispose(); disposeRetainedCemMlTemplate(artifact.artifactId); }
});
it('commits an SSR producer group with original native references and exports authority-free resume hints', async () => {
    expect(typeof document).toBe('undefined');
    const ownerSource = '{slice @name=destination}{$destination}', consumerSource = ELEMENT_REFERENCE_TEMPLATE.replace('{$destination}', '');
    const ownerArtifact = await retainCemMlTemplateSource(ownerSource), consumerArtifact = await retainCemMlTemplateSource(consumerSource);
    const references = elementReferenceFixtureInputs(); const coordinator = new CemSsrPlacementCoordinator();
    const identity = (instanceId: string) => ({ producedTag: 'placement', instanceId, templateArtifactId: instanceId, dataRevision: '1', outputTarget: 'light-dom' as const, scopePolicyStamp: 'scope' });
    try {
        const transaction = coordinator.transaction(['owner', 'consumer'], { owner: '1', consumer: '1' });
        const original = await processRetainedCemMlTemplate(ownerArtifact.artifactId, { source: ownerSource, identity: identity('owner'), data: {}, elementReferenceInputs: references });
        const owner = coordinator.stage(original.renderPlan);
        const lease = coordinator.registerPrepared(transaction, owner, [0], references.sources[1], 'input.children');
        const revoke = coordinator.grant('consumer', lease, ['commandfor']);
        const input = coordinator.prepare('consumer', references, transaction);
        const result = await processRetainedCemMlTemplate(consumerArtifact.artifactId, { source: consumerSource, identity: identity('consumer'), data: {}, elementReferenceInputs: input });
        expect(coordinator.plans().size).toBe(0);
        const consumer = coordinator.stage(result.renderPlan);
        const plans = coordinator.publishGroup(transaction, [{ stage: owner }, { stage: consumer, inputs: input, uses: result.elementPlacementUses }]);
        expect(serializeRenderPlanToHtmlFixture(required(plans.get('owner')))).toContain('id="owner-ref-0"');
        expect(serializeRenderPlanToHtmlFixture(required(plans.get('consumer')))).toContain('commandfor="owner-ref-0"');
        expect(serializeRenderPlanToHtmlFixture(original.renderPlan)).not.toContain('id="owner-ref-0"');
        const hints = coordinator.resumeHints(); expect(hints.profile).toBe('wai-aria-1.2-rec-20230606');
        expect(JSON.stringify(hints)).not.toContain(lease.token); expect(JSON.stringify(hints)).not.toContain('grants');
        expect(hints.producers.map(p => [p.producer, p.revision])).toEqual([['owner', '1'], ['consumer', '1']]);
        const fresh = new CemSsrPlacementCoordinator(); expect(fresh.plans().size).toBe(0);
        expect(fresh.prepare('consumer', references).placements?.admissions).toEqual([]);
        revoke(); expect(serializeRenderPlanToHtmlFixture(required(coordinator.plans().get('consumer')))).not.toContain('commandfor=');
        fresh.dispose();
    } finally { coordinator.dispose(); disposeRetainedCemMlTemplate(ownerArtifact.artifactId); disposeRetainedCemMlTemplate(consumerArtifact.artifactId); }
});
it('rolls back coordinated SSR publication when a producer revision is superseded', async () => {
    const coordinator = new CemSsrPlacementCoordinator();
    const plan = (instanceId: string) => ({ producedTag: 'placement', instanceId, templateArtifactId: instanceId, dataRevision: '1', outputTarget: 'light-dom' as const, scopePolicyStamp: 'scope', nodes: [] });
    const transaction = coordinator.transaction(['owner', 'consumer'], { owner: '1', consumer: '1' });
    const owner = coordinator.stage(plan('owner')), consumer = coordinator.stage(plan('consumer'), () => false);
    expect(() => coordinator.publishGroup(transaction, [{ stage: owner }, { stage: consumer }])).toThrow();
    expect(coordinator.plans().size).toBe(0); transaction.cancel(); coordinator.dispose();
});
