import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, writeDataIslandHydrationData } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { ELEMENT_REFERENCE_TEMPLATE, elementReferenceFixtureInputs } from './element-reference-lifecycle.fixtures.js';
import { processRetainedCemMlTemplate, retainCemMlTemplateSource, disposeRetainedCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';
import { materializeRenderPlan, scopeRenderPlan } from './projection.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- SSR fixture uses the same native retained entry point as the client.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
export default { title: 'CEM Elements/Edge SSR Reference IDs', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T extends Element>(parent: ParentNode, selector: string): T {
    const node = parent.querySelector<T>(selector);
    if (!node) throw new Error(`Missing fixture ${selector}`);
    return node;
}
export const NativeIdsHydrationReplacementAndReconnect: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = required<HTMLElement>(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document });
            const tag = `cem-reference-hydration-${crypto.randomUUID()}`; const declarationTag = `declaration-${tag}`;
            const references = elementReferenceFixtureInputs(fallback ? '{dialog @id=authored-hydration-part | Authored}' : '{dialog | Generated}');
            let ready = true;
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope,
                ...(fallback ? { processingWorkerFactory: () => { throw new Error('forced fallback'); } } : {}),
                elementReferenceInputs: () => ({ ...references, grants: ready ? references.grants : [] }) });
            runtime.install(window);
            const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag); declaration.setAttribute('version', '1.0.0');
            const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = ELEMENT_REFERENCE_TEMPLATE;
            declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const authored = document.createElement(tag); authored.setAttribute('tone', 'server');
            const uid = `server-${tag}`; authored.setAttribute('data-cem-render-scope', uid);
            const snapshot = runtime.snapshotInstance(authored);
            const artifact = await retainCemMlTemplateSource(ELEMENT_REFERENCE_TEMPLATE, ['tone']);
            try {
                const server = await processRetainedCemMlTemplate(artifact.artifactId, { source: ELEMENT_REFERENCE_TEMPLATE, data: { tone: 'server' }, elementReferenceInputs: references,
                    identity: { producedTag: tag, instanceId: snapshot.instanceId, templateArtifactId: snapshot.templateArtifactId, dataRevision: snapshot.dataRevision,
                        renderAttempt: snapshot.renderAttempt, outputTarget: snapshot.outputTarget, scopePolicyStamp: snapshot.scopePolicyStamp } });
                const plan = scopeRenderPlan(server.renderPlan, uid).renderPlan;
                const restored = document.createElement(tag); restored.setAttribute('tone', 'server'); restored.setAttribute('data-cem-render-scope', uid);
                const island = document.createElement('template'); island.setAttribute('data-cem-island', 'instance'); writeDataIslandHydrationData(island, snapshot);
                restored.append(island, document.createComment('cem-render-start'), materializeRenderPlan(plan, document), document.createComment('cem-render-end'));
                const button = required<HTMLButtonElement>(restored, 'button'), dialog = required<HTMLDialogElement>(restored, 'dialog'), id = dialog.id;
                root.append(restored); await runtime.whenRenderSettled(restored);
                await expect(restored.querySelector('button')).toBe(button);
                await expect(restored.querySelector('dialog')).toBe(dialog);
                await expect(button.getAttribute('commandfor')).toBe(id);
                ready = false; restored.setAttribute('tone', 'incomplete'); await runtime.whenRenderSettled(restored);
                await expect(restored.querySelector('button')).toBe(button);
                await expect(restored.querySelector('dialog')).toBe(dialog);
                await expect(button.getAttribute('commandfor')).toBe(id);
                ready = true; restored.setAttribute('tone', 'client'); await runtime.whenRenderSettled(restored);
                await expect(required<HTMLDialogElement>(restored, 'dialog').id).toBe(id);
                restored.remove(); root.append(restored); await runtime.whenRenderSettled(restored);
                await waitFor(() => expect(required<HTMLButtonElement>(restored, 'button').getAttribute('commandfor')).toBe(id));
                await expect(runtime.snapshotInstance(restored).instanceId).toBe(snapshot.instanceId);
                restored.remove();
            } finally { disposeRetainedCemMlTemplate(artifact.artifactId); declaration.remove(); root.replaceChildren(); scope.dispose(); }
        }
    },
};
