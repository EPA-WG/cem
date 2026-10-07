import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime, writeDataIslandHydrationData } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemElementPlacementCoordinator } from './element-placement-coordinator.js';
import { CemSsrPlacementCoordinator } from './element-placement-ssr.js';
import { ELEMENT_REFERENCE_TEMPLATE, elementReferenceFixtureInputs } from './element-reference-lifecycle.fixtures.js';
import { processRetainedCemMlTemplate, retainCemMlTemplateSource, disposeRetainedCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';
import { materializeRenderPlan, scopeRenderPlan } from './projection.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- SSR and hydration share the original native owners.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
export default { title: 'CEM Elements/Edge SSR Granted Placements', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T>(value: T | null | undefined): T { if (value == null) throw new Error('Missing retained hydration fixture'); return value; }
export const ResumeHintsRequireFreshClientAuthorityAndReconnectGrants: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = canvasElement.querySelector('section'); if (!root) throw new Error('Missing fixture');
        for (const fallback of [false, true]) {
            const references = elementReferenceFixtureInputs('{dialog @id=authored-foreign-part}');
            const scope = createCemDeclarationScope({ document }), client = new CemElementPlacementCoordinator(document), server = new CemSsrPlacementCoordinator();
            const tag = `cem-placement-hydration-${crypto.randomUUID()}`, declarationTag = `declaration-${tag}`;
            const templateSource = ELEMENT_REFERENCE_TEMPLATE.replace('{$destination}', ''), ownerSource = '{slice @name=destination}{$destination}';
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope, placementCoordinator: client,
                elementReferenceInputs: () => references, ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback'); } } : {}) });
            runtime.install(window);
            const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag); declaration.setAttribute('version', '1.0.0');
            const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = templateSource;
            declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
            const uid = `hydrated-${tag}`, authored = document.createElement(tag); authored.setAttribute('data-cem-render-scope', uid);
            const snapshot = runtime.snapshotInstance(authored), artifact = await retainCemMlTemplateSource(templateSource), ownerArtifact = await retainCemMlTemplateSource(ownerSource);
            const consumerId = snapshot.instanceId, ownerId = `owner-${tag}`;
            let restored: HTMLElement | undefined, owner: HTMLElement | undefined;
            try {
                const transaction = server.transaction([ownerId, consumerId], { [ownerId]: '1', [consumerId]: snapshot.dataRevision });
                const ownerResult = await processRetainedCemMlTemplate(ownerArtifact.artifactId, { source: ownerSource, data: {}, elementReferenceInputs: references,
                    identity: { producedTag: 'owner', instanceId: ownerId, templateArtifactId: 'owner', dataRevision: '1', outputTarget: 'light-dom', scopePolicyStamp: 'server-scope' } });
                const ownerStage = server.stage(ownerResult.renderPlan), lease = server.registerPrepared(transaction, ownerStage, [0], references.sources[1], 'input.children');
                server.grant(consumerId, lease, ['commandfor']);
                const inputs = server.prepare(consumerId, references, transaction);
                const consumerResult = await processRetainedCemMlTemplate(artifact.artifactId, { source: templateSource, data: {}, elementReferenceInputs: inputs, identity: {
                    producedTag: tag, instanceId: consumerId, templateArtifactId: snapshot.templateArtifactId, dataRevision: snapshot.dataRevision, renderAttempt: snapshot.renderAttempt,
                    outputTarget: snapshot.outputTarget, scopePolicyStamp: snapshot.scopePolicyStamp } });
                const plans = server.publishGroup(transaction, [{ stage: ownerStage }, { stage: server.stage(consumerResult.renderPlan), inputs, uses: consumerResult.elementPlacementUses }]);
                const hints = server.resumeHints(); await expect(hints.profile).toBe('wai-aria-1.2-rec-20230606');
                owner = document.createElement('div'); owner.append(materializeRenderPlan(required(plans.get(ownerId)), document));
                restored = document.createElement(tag); restored.setAttribute('data-cem-render-scope', uid);
                const island = document.createElement('template'); island.setAttribute('data-cem-island', 'instance'); writeDataIslandHydrationData(island, snapshot);
                restored.append(island, document.createComment('cem-render-start'), materializeRenderPlan(scopeRenderPlan(required(plans.get(consumerId)), uid).renderPlan, document), document.createComment('cem-render-end'));
                const button = required(restored.querySelector('button')), target = required(owner.querySelector('dialog'));
                await expect(button.getAttribute('commandfor')).toBe(target.id);
                root.append(owner, restored); await runtime.whenRenderSettled(restored);
                await expect(button.hasAttribute('commandfor')).toBe(false); await expect(button.hasAttribute('data-cem-placement-ref-commandfor')).toBe(false);
                await expect(target.id).toBe('authored-foreign-part');
                // A stale server revision descriptor supplies no registry authority.
                hints.producers[0].revision = 'stale';
                runtime.refreshElementReferences(restored); await runtime.whenRenderSettled(restored); await expect(button.hasAttribute('commandfor')).toBe(false);
                const fresh = client.register({ producer: ownerId, revision: 'client-2', path: [0], element: target, source: references.sources[1], select: 'input.children', currentRevision: () => 'client-2' });
                client.grant(consumerId, fresh, ['commandfor']); runtime.refreshElementReferences(restored); await runtime.whenRenderSettled(restored);
                await expect(restored.querySelector('button')?.getAttribute('commandfor')).toBe(target.id);
                restored.remove(); root.append(restored); await runtime.whenRenderSettled(restored);
                await expect(restored.querySelector('button')?.hasAttribute('commandfor')).toBe(false);
                client.grant(consumerId, fresh, ['commandfor']); runtime.refreshElementReferences(restored); await runtime.whenRenderSettled(restored);
                await expect(restored.querySelector('button')?.getAttribute('commandfor')).toBe(target.id);
                fresh.dispose();
            } finally { restored?.remove(); owner?.remove(); declaration.remove(); client.dispose(); server.dispose(); scope.dispose(); disposeRetainedCemMlTemplate(artifact.artifactId); disposeRetainedCemMlTemplate(ownerArtifact.artifactId); }
        }
    },
};
