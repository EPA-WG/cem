import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, analyzeDeclarationRegistrationIdentity, exportDataIslandSnapshotForEdge, writeDataIslandHydrationData } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { createBrowserModuleUrlContext, createBrowserModuleUrlRoot } from './internal/runtime-support/module-url-resolution.js';
import { executeNativeSsrInitialRenderFixture, executeNativeEdgeRenderUpdateFixture } from './edge-ssr-host-fixture.js';
import { CemEdgeSsrJobSequence, createCemEdgeSsrHostRequestEnvelope } from './edge-ssr-host.js';
import { InMemoryEdgeRenderStateStore, readTemplateSource, applyPatchFramesToRange, type PatchFrame } from './projection.js';
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

export const NativePayloadStylesFromInitialSsr = nativeInitialStylesStory(false);
export const NativeDeclarationStylesFromInitialSsr = nativeInitialStylesStory(true);

export const NativeStylesPreservedAcrossEdgeUpdates = nativeInitialStylesStory(true, true);

function nativeInitialStylesStory(includeDeclarations: boolean, includeUpdate = false): Story {
    return {
        render: () => '<section aria-label="Native SSR payload stylesheet hydration"></section>',
        play: async ({ canvasElement }) => {
            await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
            const root = canvasElement.querySelector('section');
            if (!root) throw new Error('missing SSR fixture');
            for (const fallback of [false, true]) {
                const scope = createCemDeclarationScope({ document });
                const tag = `native-ssr-${includeUpdate ? 'updates-' : includeDeclarations ? 'declarations-' : ''}${fallback ? 'fallback' : 'worker'}`;
                const importedCss = 'p {color:rgb(1,2,3);animation:pulse 20s infinite} @keyframes pulse {from{opacity:.5}to{opacity:1}}';
                const bytes = new TextEncoder().encode(importedCss).buffer;
                const response = { bytes, finalUrl: new URL('./ssr-child.css', document.baseURI).href, contentType: 'text/css' };
                const declarationSources = [{ css: '@import "./ssr-declaration.css";', scope: null },
                    { css: ':host {--shared-ready:yes}', scope: 'ssr-library' }];
                const declarationResponse = { bytes: new TextEncoder().encode('p {border-top:3px solid rgb(4,5,6)} :host {--asset:url("./ssr-icon.svg")}').buffer,
                    finalUrl: new URL('./ssr-declaration.css', document.baseURI).href, contentType: 'text/css' };
                const read = async (request: { url: string }) => request.url.endsWith('/ssr-declaration.css') ? declarationResponse : response;
                let reads = 0;
                let release: () => void = () => undefined;
                const gate = new Promise<void>(resolve => { release = resolve; });
                const runtime = new CemElementRuntime({ declarationTag: `declaration-${tag}`, declarationScope: scope,
                    moduleUrlRoot: { baseUrl: document.baseURI, importMap: {} },
                    retainedStylesheets: { read: async request => { reads++; await gate; return read(request); } },
                    processingWorkerFactory: request => {
                        if (fallback) throw new Error('forced fallback');
                        return new Worker(request.scriptUrl, { type: request.type, name: request.name });
                    },
                });
                try {
                    const declaration = document.createElement('div');
                    declaration.setAttribute('tag', tag); declaration.setAttribute('version', '1.0.0');
                    const template = document.createElement('template');
                    template.innerHTML = includeUpdate ? '<attribute name="label">Server content</attribute><p>${$label}</p>' : '<p>Server content</p>';
                    if (includeDeclarations) {
                        declaration.setAttribute('scope', 'ssr-library');
                        template.innerHTML = `<style>${declarationSources[0].css}</style><style scope="ssr-library">${declarationSources[1].css}</style>${template.innerHTML}`;
                    }
                    declaration.append(template); root.append(declaration);
                    runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
                    const authored = document.createElement(tag);
                    authored.setAttribute('data-cem-render-scope', `server-${tag}`);
                    authored.innerHTML = '<template><style>@import "./ssr-child.css";</style></template>';
                    if (includeDeclarations) authored.setAttribute('scope', 'ssr-library');
                    const snapshot = runtime.snapshotInstance(authored);
                    if (includeUpdate && fallback) snapshot.sourceMapMode = 'prod';
                    const rootContext = createBrowserModuleUrlRoot(document, snapshot.scopePolicyStamp,
                        { baseUrl: document.baseURI, importMap: {} });
                    const context = createBrowserModuleUrlContext(rootContext.context, 'server', document.baseURI,
                        `document:${document.baseURI}`, snapshot.scopePolicyStamp).wire;
                    const exported = exportDataIslandSnapshotForEdge(snapshot, { fields: {
                        hostAttributes: 'allow', dataset: 'allow', payload: 'allow', slices: 'allow', formData: 'allow',
                        validationState: 'allow', eventPayloads: 'allow',
                    } });
                    const declarationIdentity = analyzeDeclarationRegistrationIdentity({ tag, declarationVersion: '1.0.0',
                        resolvedTemplateSource: template.innerHTML, templateLanguage: 'dom', hasBehavior: false,
                        scopePolicyStamp: snapshot.scopePolicyStamp, sharedStyleScope: includeDeclarations ? 'ssr-library' : null,
                    }).registrationIdentity;
                    if (!declarationIdentity) throw new Error('missing declaration identity');
                    // The host supplies the render source after extracting static declaration CSS.
                    const renderTemplate = template.content.cloneNode(true) as DocumentFragment;
                    renderTemplate.querySelectorAll('style').forEach(style => style.remove());
                    const request = createCemEdgeSsrHostRequestEnvelope(new CemEdgeSsrJobSequence(), 'render-initial', {
                        template: { kind: 'serialized-template-source-v1', templateArtifactId: snapshot.templateArtifactId,
                            source: readTemplateSource(renderTemplate) },
                        snapshot: exported, scopeUid: `server-${tag}`, sourceMapMode: snapshot.sourceMapMode ?? 'dev',
                        revision: { instanceId: snapshot.instanceId, dataRevision: snapshot.dataRevision,
                            templateArtifactId: snapshot.templateArtifactId, scopePolicyStamp: snapshot.scopePolicyStamp,
                            outputTarget: snapshot.outputTarget, renderAttempt: snapshot.renderAttempt },
                    });
                    const store = new InMemoryEdgeRenderStateStore();
                    const stylesheetContext = { baseUrl: document.baseURI, context,
                        ...(includeDeclarations ? { declarations: [{ owner: { kind: 'declaration' as const, identity: declarationIdentity, tag },
                            sources: declarationSources, baseUrl: document.baseURI, context }] } : {}),
                    };
                    const result = await executeNativeSsrInitialRenderFixture(request, store, {
                        native: wasm, ...stylesheetContext, signal: new AbortController().signal, read,
                    });
                    expect(result.outcome).toBe('success');
                    if (result.outcome !== 'success') throw new Error(result.diagnostics.map(d => d.message).join('\n'));
                    expect(result.result.diagnostics).toEqual([]);
                    expect(result.result.renderedHtml).not.toContain('<style');
                    let serverDeclaration: HTMLElement | undefined;
                    let declarationNodes: HTMLStyleElement[] = [];
                    if (includeDeclarations) {
                        const batch = result.result.declarationStylesheets?.[0];
                        if (!batch) throw new Error('missing declaration sidecar');
                        declaration.remove();
                        serverDeclaration = declaration.cloneNode(true) as HTMLElement;
                        serverDeclaration.insertAdjacentHTML('beforeend', batch.html);
                        declarationNodes = Array.from(serverDeclaration.querySelectorAll(':scope > style'));
                        expect(declarationNodes).toHaveLength(2);
                        root.append(serverDeclaration);
                        runtime.registerDeclaration(serverDeclaration); await runtime.whenDeclarationSettled(serverDeclaration);
                    }
                    const island = authored.querySelector('template');
                    if (!island) throw new Error('missing authored island');
                    writeDataIslandHydrationData(island, snapshot);
                    const restored = document.createElement(tag);
                    restored.setAttribute('data-cem-render-scope', `server-${tag}`);
                    restored.innerHTML = `${island.outerHTML}<!--cem-render-start-->${result.result.renderedHtml}<!--cem-render-end-->${result.result.instanceStylesheetHtml}`;
                    if (includeDeclarations) {
                        restored.setAttribute('scope', 'ssr-library');
                        const marker = result.result.declarationStylesheets?.[0].contextMarker;
                        if (marker) restored.setAttribute('data-cem-css-context', marker);
                    }
                    const paragraph = restored.querySelector('p');
                    const style = restored.querySelector(':scope > style[data-cem-instance-style]');
                    if (!paragraph || !style) throw new Error('missing server nodes');
                    const serverCss = style.textContent;
                    root.append(restored);
                    let settled = false;
                    const ready = runtime.whenRenderSettled(restored).then(() => { settled = true; });
                    await waitFor(() => expect(reads).toBeGreaterThanOrEqual(1));
                    expect(settled).toBe(false);
                    expect(restored.querySelector('p')).toBe(paragraph);
                    expect(restored.querySelector(':scope > style[data-cem-instance-style]')).toBe(style);
                    expect(getComputedStyle(paragraph).color).toBe('rgb(1, 2, 3)');
                    release(); await ready;
                    expect(restored.querySelector('p')).toBe(paragraph);
                    expect(restored.querySelector(':scope > style[data-cem-instance-style]')).toBe(style);
                    expect(style.textContent).toBe(serverCss);
                    expect(restored.querySelectorAll(':scope > style[data-cem-instance-style]')).toHaveLength(1);
                    expect(getComputedStyle(paragraph).color).toBe('rgb(1, 2, 3)');
                    expect(reads).toBe(includeDeclarations ? 2 : 1);
                    if (serverDeclaration) {
                        expect(serverDeclaration.querySelectorAll(':scope > style')).toHaveLength(2);
                        declarationNodes.forEach((node, index) => expect(serverDeclaration?.querySelectorAll(':scope > style')[index]).toBe(node));
                        expect(getComputedStyle(paragraph).borderTopColor).toBe('rgb(4, 5, 6)');
                        expect(getComputedStyle(restored).getPropertyValue('--shared-ready').trim()).toBe('yes');
                        expect(restored.getAttribute('data-cem-css-context')).toBe(result.result.declarationStylesheets?.[0].contextMarker);
                    }
                    const animation = paragraph.getAnimations()[0] as CSSAnimation;
                    expect(serverCss).toContain(`@keyframes ${animation.animationName}`);
                    if (includeUpdate) {
                        const previous = result.result.renderState;
                        const next = structuredClone(request.payload);
                        next.snapshot.hostAttributes = { ...next.snapshot.hostAttributes, label: 'Updated on Edge' };
                        next.snapshot.dataRevision = 'next'; next.revision.dataRevision = 'next';
                        const update = createCemEdgeSsrHostRequestEnvelope(new CemEdgeSsrJobSequence(), 'render-update', {
                            ...next, previousRenderPlan: { stateKey: previous.stateKey, expectedEtag: previous.etag,
                                identity: previous.renderRevision, address: { ...previous.currentRenderPlan, kind: 'render-plan' } },
                        });
                        const frames: PatchFrame[] = [];
                        let completed = false;
                        for await (const response of executeNativeEdgeRenderUpdateFixture(update, store, stylesheetContext, new AbortController().signal)) {
                            if (response.outcome === 'progress') frames.push(response.result.frame);
                            else {
                                expect(response.outcome).toBe('success');
                                if (response.outcome !== 'success') throw new Error('native update failed');
                                expect(response.result.renderState.currentStylesheets).toEqual(previous.currentStylesheets);
                                completed = true;
                            }
                        }
                        expect(completed).toBe(true);
                        const comments = Array.from(restored.childNodes).filter((node): node is Comment => node.nodeType === Node.COMMENT_NODE);
                        const start = comments.find(node => node.data === 'cem-render-start');
                        const end = comments.find(node => node.data === 'cem-render-end');
                        if (!start || !end) throw new Error('missing render bounds');
                        const applied = applyPatchFramesToRange({ start, end }, frames, next.revision, document);
                        expect(applied.status, JSON.stringify(applied.diagnostics)).toBe('applied');
                        expect(restored.querySelector('p')).toBe(paragraph);
                        expect(paragraph.textContent).toBe('Updated on Edge');
                        expect(restored.querySelector(':scope > style[data-cem-instance-style]')).toBe(style);
                        declarationNodes.forEach((node, index) => expect(serverDeclaration?.querySelectorAll(':scope > style')[index]).toBe(node));
                        expect(paragraph.getAnimations()[0]).toBe(animation);
                        expect(getComputedStyle(paragraph).color).toBe('rgb(1, 2, 3)');
                        expect(getComputedStyle(paragraph).borderTopColor).toBe('rgb(4, 5, 6)');
                        expect(reads).toBe(2);
                    }
                    expect(runtime.diagnosticsFor(restored).filter(d => !fallback || d.code !== 'cem.processing_host.worker_startup_fallback')).toEqual([]);
                } finally { release(); root.replaceChildren(); scope.dispose(); }
            }
        },
    };
}
