import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor, userEvent } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemElementPlacementCoordinator } from './element-placement-coordinator.js';
import { elementReferenceFixtureInputs, ELEMENT_REFERENCE_TEMPLATE } from './element-reference-lifecycle.fixtures.js';
import { processRetainedCemMlTemplate, retainCemMlTemplateSource, disposeRetainedCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- original CEM owners enter through the explicit debug reload boundary.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
export default { title: 'CEM Elements/Granted Placements', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const foreignTemplate = ELEMENT_REFERENCE_TEMPLATE.replace('{$destination}', '');
export const CommittedOwnerWorkerFallbackRevocationAndFreshReconnect: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = canvasElement.querySelector('section'); if (!root) throw new Error('Missing fixture');
        for (const fallback of [false, true]) {
            const coordinator = new CemElementPlacementCoordinator(document);
            const references = elementReferenceFixtureInputs(); const scope = createCemDeclarationScope({ document });
            const tag = `cem-foreign-${crypto.randomUUID()}`, declarationTag = `declaration-${tag}`;
            let revision = '1', ready = true;
            const dialog = document.createElement('dialog'); root.append(dialog);
            const lease = coordinator.register({ producer: 'foreign-owner', path: [0], revision: '1', element: dialog, source: references.sources[1], select: 'input.children', currentRevision: () => ready ? revision : undefined });
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope, placementCoordinator: coordinator,
                elementReferenceInputs: () => references, ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback'); } } : {}) });
            runtime.install(window);
            const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag);
            const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = foreignTemplate;
            declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration);
            const instance = document.createElement(tag); root.append(instance);
            try {
                await runtime.whenRenderSettled(instance); await expect(instance.querySelector('button')).toBeNull();
                const id = runtime.snapshotInstance(instance).instanceId;
                let revoke = coordinator.grant(id, lease, ['commandfor']); runtime.refreshElementReferences(instance); await runtime.whenRenderSettled(instance);
                const button = instance.querySelector('button'); if (!button) throw new Error('Missing admitted invoker');
                await expect(button.getAttribute('commandfor')).toBe('foreign-owner-ref-0');
                await expect(dialog.id).toBe('foreign-owner-ref-0'); await userEvent.click(button); await expect(dialog.open).toBe(true); dialog.close();
                revoke(); await expect(button.hasAttribute('commandfor')).toBe(false);
                await expect(button.hasAttribute('data-cem-placement-ref-commandfor')).toBe(false);
                await runtime.whenRenderSettled(instance); await expect(instance.querySelector('button')).toBe(button);
                revoke = coordinator.grant(id, lease, ['commandfor']); runtime.refreshElementReferences(instance); await runtime.whenRenderSettled(instance);
                await expect(instance.querySelector('button')?.getAttribute('commandfor')).toBe(dialog.id);
                const admitted = instance.querySelector('button'); dialog.remove();
                await waitFor(() => expect(admitted?.hasAttribute('commandfor')).toBe(false));
                root.append(dialog); runtime.refreshElementReferences(instance); await runtime.whenRenderSettled(instance);
                await expect(instance.querySelector('button')?.hasAttribute('commandfor')).toBe(false);
                revoke(); lease.dispose(); revision = '2'; ready = false;
            } finally { instance.remove(); dialog.remove(); declaration.remove(); scope.dispose(); coordinator.dispose(); }
        }
    },
};
export const PreparedGroupHasNoPartialRelationshipsAndRejectsStalePublication: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = canvasElement.querySelector('section'); if (!root) throw new Error('Missing fixture');
        const owner = document.createElement('div'), consumer = document.createElement('div'); root.append(owner, consumer);
        const coordinator = new CemElementPlacementCoordinator(document); const references = elementReferenceFixtureInputs();
        const artifact = await retainCemMlTemplateSource(foreignTemplate);
        const ownerArtifact = await retainCemMlTemplateSource('{slice @name=destination}{$destination}');
        const identity = (instanceId: string) => ({ producedTag: 'fixture', instanceId, templateArtifactId: 'placement', dataRevision: '2', outputTarget: 'light-dom' as const, scopePolicyStamp: 'scope' });
        let current = true;
        try {
            const transaction = coordinator.transaction([{ producer: 'owner', host: owner }, { producer: 'consumer', host: consumer }], { owner: '2', consumer: '2' });
            const ownerResult = await processRetainedCemMlTemplate(ownerArtifact.artifactId, { source: '{slice @name=destination}{$destination}', identity: identity('owner'), data: {}, elementReferenceInputs: references });
            const ownerStage = coordinator.stage(owner, ownerResult.renderPlan, () => current);
            const lease = coordinator.registerPrepared(transaction, ownerStage, [0], references.sources[1], 'input.children');
            coordinator.grant('consumer', lease, ['commandfor']);
            const input = coordinator.prepare(consumer, 'consumer', references, transaction);
            const result = await processRetainedCemMlTemplate(artifact.artifactId, { source: foreignTemplate, identity: identity('consumer'), data: {}, elementReferenceInputs: input });
            const consumerStage = coordinator.stage(consumer, result.renderPlan, () => current);
            await expect(consumer.querySelector('button')).toBeNull(); await expect(owner.querySelector('dialog')).toBeNull();
            current = false;
            await expect(() => coordinator.publishGroup(transaction, [{ stage: ownerStage }, { stage: consumerStage, inputs: input, uses: result.elementPlacementUses }])).toThrow();
            await expect(owner.children.length).toBe(0); await expect(consumer.children.length).toBe(0);
            current = true;
            coordinator.publishGroup(transaction, [{ stage: ownerStage }, { stage: consumerStage, inputs: input, uses: result.elementPlacementUses }]);
            const target = owner.querySelector('dialog'), invoker = consumer.querySelector('button');
            await expect(target?.id).toBe('owner-ref-0'); await expect(invoker?.getAttribute('commandfor')).toBe(target?.id);
            await expect(invoker?.hasAttribute('data-cem-placement-ref-commandfor')).toBe(true);
            lease.dispose(); await expect(invoker?.hasAttribute('commandfor')).toBe(false);
        } finally { disposeRetainedCemMlTemplate(artifact.artifactId); disposeRetainedCemMlTemplate(ownerArtifact.artifactId); coordinator.dispose(); owner.remove(); consumer.remove(); }
    },
};
export const NestedPreparedProducerAndReservationConflict: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = canvasElement.querySelector('section'); if (!root) throw new Error('Missing fixture');
        const parent = document.createElement('div'), consumer = document.createElement('div'); root.append(parent, consumer);
        const coordinator = new CemElementPlacementCoordinator(document), references = elementReferenceFixtureInputs();
        const source = '{slice @name=destination}{$destination}', outerSource = '{div {div}}';
        const ownerArtifact = await retainCemMlTemplateSource(source), outerArtifact = await retainCemMlTemplateSource(outerSource), consumerArtifact = await retainCemMlTemplateSource(foreignTemplate);
        const identity = (instanceId: string) => ({ producedTag: 'fixture', instanceId, templateArtifactId: instanceId, dataRevision: '3', outputTarget: 'light-dom' as const, scopePolicyStamp: 'scope' });
        try {
            const outer = await processRetainedCemMlTemplate(outerArtifact.artifactId, { source: outerSource, identity: identity('outer'), data: {} });
            const outerStage = coordinator.stage(parent, outer.renderPlan, () => true);
            const nested = outerStage.fragment.querySelector<HTMLElement>('div div'); if (!nested) throw new Error('Missing nested producer host');
            const transaction = coordinator.transaction([{ producer: 'outer', host: parent }, { producer: 'nested-owner', host: nested }, { producer: 'consumer', host: consumer }], { outer: '3', 'nested-owner': '3', consumer: '3' });
            coordinator.enlist(transaction, outerStage);
            const owner = await processRetainedCemMlTemplate(ownerArtifact.artifactId, { source, identity: identity('nested-owner'), data: {}, elementReferenceInputs: references });
            const ownerStage = coordinator.stage(nested, owner.renderPlan, () => true, transaction);
            const lease = coordinator.registerPrepared(transaction, ownerStage, [0], references.sources[1], 'input.children'); coordinator.grant('consumer', lease, ['commandfor']);
            const input = coordinator.prepare(consumer, 'consumer', references, transaction);
            const result = await processRetainedCemMlTemplate(consumerArtifact.artifactId, { source: foreignTemplate, identity: identity('consumer'), data: {}, elementReferenceInputs: input });
            const consumerStage = coordinator.stage(consumer, result.renderPlan, () => true, transaction);
            const conflict = document.createElement('span'); conflict.id = 'nested-owner-ref-0'; root.append(conflict);
            await expect(() => coordinator.publishGroup(transaction, [{ stage: outerStage }, { stage: ownerStage }, { stage: consumerStage, inputs: input, uses: result.elementPlacementUses }])).toThrow();
            await expect(parent.children.length).toBe(0); await expect(consumer.children.length).toBe(0);
            conflict.remove(); coordinator.publishGroup(transaction, [{ stage: outerStage }, { stage: ownerStage }, { stage: consumerStage, inputs: input, uses: result.elementPlacementUses }]);
            await expect(nested.isConnected).toBe(true); await expect(consumer.querySelector('button')?.getAttribute('commandfor')).toBe(nested.querySelector('dialog')?.id);
            const shadowHost = document.createElement('div'); root.append(shadowHost); const shadow = shadowHost.attachShadow({ mode: 'open' }); const foreign = document.createElement('dialog'); shadow.append(foreign);
            await expect(() => coordinator.register({ producer: 'outside-root', revision: '3', path: [0], element: foreign, source: references.sources[1], select: 'input.children', currentRevision: () => '3' })).toThrow();
            shadowHost.remove(); lease.dispose();
        } finally { coordinator.dispose(); parent.remove(); consumer.remove(); disposeRetainedCemMlTemplate(ownerArtifact.artifactId); disposeRetainedCemMlTemplate(outerArtifact.artifactId); disposeRetainedCemMlTemplate(consumerArtifact.artifactId); }
    },
};
export const ActivationRevocationRestoresOldForestsBeforeRetry: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = canvasElement.querySelector('section'); if (!root) throw new Error('Missing fixture');
        const owner = document.createElement('div'), consumer = document.createElement('div'); owner.textContent = 'Old owner'; consumer.textContent = 'Old consumer'; root.append(owner, consumer);
        const coordinator = new CemElementPlacementCoordinator(document), references = elementReferenceFixtureInputs();
        let revoke: () => void = () => undefined; let race = true;
        const reactionTag = `cem-placement-reaction-${crypto.randomUUID()}`;
        customElements.define(reactionTag, class extends HTMLElement {
            static observedAttributes = ['commandfor'];
            attributeChangedCallback(_name: string, _old: string | null, value: string | null) { if (value !== null && race) { race = false; revoke(); } }
        });
        const source = '{slice @name=destination}{$destination}', consumerSource = foreignTemplate.replace('button ', `${reactionTag} `);
        const ownerArtifact = await retainCemMlTemplateSource(source), artifact = await retainCemMlTemplateSource(consumerSource);
        const identity = (instanceId: string) => ({ producedTag: 'fixture', instanceId, templateArtifactId: instanceId, dataRevision: '4', outputTarget: 'light-dom' as const, scopePolicyStamp: 'scope' });
        try {
            const transaction = coordinator.transaction([{ producer: 'owner', host: owner }, { producer: 'consumer', host: consumer }], { owner: '4', consumer: '4' });
            const producer = await processRetainedCemMlTemplate(ownerArtifact.artifactId, { source, identity: identity('owner'), data: {}, elementReferenceInputs: references });
            const ownerStage = coordinator.stage(owner, producer.renderPlan, () => true, transaction);
            const lease = coordinator.registerPrepared(transaction, ownerStage, [0], references.sources[1], 'input.children'); revoke = coordinator.grant('consumer', lease, ['commandfor']);
            const inputs = coordinator.prepare(consumer, 'consumer', references, transaction);
            const result = await processRetainedCemMlTemplate(artifact.artifactId, { source: consumerSource, identity: identity('consumer'), data: {}, elementReferenceInputs: inputs });
            const consumerStage = coordinator.stage(consumer, result.renderPlan, () => true, transaction);
            await expect(() => coordinator.publishGroup(transaction, [{ stage: ownerStage }, { stage: consumerStage, inputs, uses: result.elementPlacementUses }])).toThrow();
            await expect(owner.textContent).toBe('Old owner'); await expect(consumer.textContent).toBe('Old consumer');
            await expect(ownerStage.fragment.querySelector('dialog')?.hasAttribute('id')).toBe(false);
            await expect(consumerStage.fragment.querySelector(reactionTag)?.hasAttribute('commandfor')).toBe(false);
            await expect(consumerStage.fragment.querySelector(reactionTag)?.hasAttribute('data-cem-placement-ref-commandfor')).toBe(false);
            revoke = coordinator.grant('consumer', lease, ['commandfor']); const retry = coordinator.prepare(consumer, 'consumer', references, transaction);
            const retried = await processRetainedCemMlTemplate(artifact.artifactId, { source: consumerSource, identity: identity('consumer'), data: {}, elementReferenceInputs: retry });
            coordinator.publishGroup(transaction, [{ stage: ownerStage }, { stage: consumerStage, inputs: retry, uses: retried.elementPlacementUses }]);
            await expect(consumer.querySelector(reactionTag)?.getAttribute('commandfor')).toBe(owner.querySelector('dialog')?.id);
            lease.dispose();
        } finally { coordinator.dispose(); owner.remove(); consumer.remove(); disposeRetainedCemMlTemplate(ownerArtifact.artifactId); disposeRetainedCemMlTemplate(artifact.artifactId); }
    },
};
