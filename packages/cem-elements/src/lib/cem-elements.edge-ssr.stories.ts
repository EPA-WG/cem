import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, exportDataIslandSnapshotForEdge, writeDataIslandHydrationData } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { createBrowserModuleUrlContext, createBrowserModuleUrlRoot } from './internal/runtime-support/module-url-resolution.js';
import { executeNativeSsrInitialRenderFixture } from './edge-ssr-host-fixture.js';
import { CemEdgeSsrJobSequence, createCemEdgeSsrHostRequestEnvelope } from './edge-ssr-host.js';
import { InMemoryEdgeRenderStateStore, readTemplateSource } from './projection.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- verify the same native bindings used by the Node SSR evidence host.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { edgeSsrStories } from './cem-elements.stories.js';

const meta: Meta = {
    title: 'CEM Elements/Edge SSR',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const SsrHydrationFromSerializedSnapshot: Story =
    edgeSsrStories.SsrHydrationFromSerializedSnapshot;
export const SsrHydrationRerendersIncompatibleDeclarationVersion: Story =
    edgeSsrStories.SsrHydrationRerendersIncompatibleDeclarationVersion;
export const SsrHydrationRejectsUnsupportedSnapshotVersion: Story =
    edgeSsrStories.SsrHydrationRejectsUnsupportedSnapshotVersion;
export const SsrHydrationRejectsIncompleteMarkup: Story =
    edgeSsrStories.SsrHydrationRejectsIncompleteMarkup;
export const EdgePatchFramesFromSerializedSnapshot: Story =
    edgeSsrStories.EdgePatchFramesFromSerializedSnapshot;
export const BrowserToEdgeSnapshotPrivacyPolicy: Story =
    edgeSsrStories.BrowserToEdgeSnapshotPrivacyPolicy;
export const EdgeRenderStateHybridStorageModel: Story =
    edgeSsrStories.EdgeRenderStateHybridStorageModel;

export const NativePayloadStylesFromInitialSsr: Story = {
    render: () => '<section aria-label="Native SSR payload stylesheet hydration"></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing SSR fixture');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document });
            const tag = `native-ssr-${fallback ? 'fallback' : 'worker'}`;
            const importedCss = 'p {color:rgb(1,2,3);animation:pulse 20s infinite} @keyframes pulse {from{opacity:.5}to{opacity:1}}';
            const bytes = new TextEncoder().encode(importedCss).buffer;
            const response = { bytes, finalUrl: new URL('./ssr-child.css', document.baseURI).href, contentType: 'text/css' };
            let reads = 0;
            let release: () => void = () => undefined;
            const gate = new Promise<void>(resolve => { release = resolve; });
            const runtime = new CemElementRuntime({ declarationTag: `declaration-${tag}`, declarationScope: scope,
                moduleUrlRoot: { baseUrl: document.baseURI, importMap: {} },
                retainedStylesheets: { read: async () => { reads++; await gate; return response; } },
                processingWorkerFactory: request => {
                    if (fallback) throw new Error('forced fallback');
                    return new Worker(request.scriptUrl, { type: request.type, name: request.name });
                },
            });
            try {
                const declaration = document.createElement('div');
                declaration.setAttribute('tag', tag); declaration.setAttribute('version', '1.0.0');
                const template = document.createElement('template'); template.innerHTML = '<p>Server content</p>';
                declaration.append(template); root.append(declaration);
                runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
                const authored = document.createElement(tag);
                authored.setAttribute('data-cem-render-scope', `server-${tag}`);
                authored.innerHTML = '<template><style>@import "./ssr-child.css";</style></template>';
                const snapshot = runtime.snapshotInstance(authored);
                const rootContext = createBrowserModuleUrlRoot(document, snapshot.scopePolicyStamp,
                    { baseUrl: document.baseURI, importMap: {} });
                const context = createBrowserModuleUrlContext(rootContext.context, 'server', document.baseURI,
                    `document:${document.baseURI}`, snapshot.scopePolicyStamp).wire;
                const exported = exportDataIslandSnapshotForEdge(snapshot, { fields: {
                    hostAttributes: 'allow', dataset: 'allow', payload: 'allow', slices: 'allow', formData: 'allow',
                    validationState: 'allow', eventPayloads: 'allow',
                } });
                const request = createCemEdgeSsrHostRequestEnvelope(new CemEdgeSsrJobSequence(), 'render-initial', {
                    template: { kind: 'serialized-template-source-v1', templateArtifactId: snapshot.templateArtifactId,
                        source: readTemplateSource(template.content) },
                    snapshot: exported, scopeUid: `server-${tag}`, sourceMapMode: 'dev',
                    revision: { instanceId: snapshot.instanceId, dataRevision: snapshot.dataRevision,
                        templateArtifactId: snapshot.templateArtifactId, scopePolicyStamp: snapshot.scopePolicyStamp,
                        outputTarget: snapshot.outputTarget, renderAttempt: snapshot.renderAttempt },
                });
                const result = await executeNativeSsrInitialRenderFixture(request, new InMemoryEdgeRenderStateStore(), {
                    native: wasm, baseUrl: document.baseURI, context, signal: new AbortController().signal,
                    read: async () => response,
                });
                expect(result.outcome).toBe('success');
                if (result.outcome !== 'success') throw new Error(result.diagnostics.map(d => d.message).join('\n'));
                expect(result.result.diagnostics).toEqual([]);
                expect(result.result.renderedHtml).not.toContain('<style');
                const island = authored.querySelector('template');
                if (!island) throw new Error('missing authored island');
                writeDataIslandHydrationData(island, snapshot);
                const restored = document.createElement(tag);
                restored.setAttribute('data-cem-render-scope', `server-${tag}`);
                restored.innerHTML = `${island.outerHTML}<!--cem-render-start-->${result.result.renderedHtml}<!--cem-render-end-->${result.result.instanceStylesheetHtml}`;
                const paragraph = restored.querySelector('p');
                const style = restored.querySelector(':scope > style[data-cem-instance-style]');
                if (!paragraph || !style) throw new Error('missing server nodes');
                const serverCss = style.textContent;
                root.append(restored);
                let settled = false;
                const ready = runtime.whenRenderSettled(restored).then(() => { settled = true; });
                await waitFor(() => expect(reads).toBe(1));
                expect(settled).toBe(false);
                expect(restored.querySelector('p')).toBe(paragraph);
                release(); await ready;
                expect(restored.querySelector('p')).toBe(paragraph);
                expect(restored.querySelector(':scope > style[data-cem-instance-style]')).toBe(style);
                expect(style.textContent).toBe(serverCss);
                expect(restored.querySelectorAll(':scope > style[data-cem-instance-style]')).toHaveLength(1);
                expect(getComputedStyle(paragraph).color).toBe('rgb(1, 2, 3)');
                const animation = paragraph.getAnimations()[0] as CSSAnimation;
                expect(serverCss).toContain(`@keyframes ${animation.animationName}`);
                expect(runtime.diagnosticsFor(restored).filter(d => !fallback || d.code !== 'cem.processing_host.worker_startup_fallback')).toEqual([]);
            } finally { release(); root.replaceChildren(); scope.dispose(); }
        }
    },
};
