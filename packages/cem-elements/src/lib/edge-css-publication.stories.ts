import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { DeclarationStyleOwnership } from './declaration-style-ownership.js';
import { exportDataIslandSnapshotForEdge } from './cem-elements.js';
import { edgeSsrSnapshotFixture } from './processing-boundary.fixtures.js';
import { CemEdgeSsrJobSequence, createCemEdgeSsrHostRequestEnvelope, type CemEdgeSsrRenderUpdateResult } from './edge-ssr-host.js';
import { executeNativeSsrInitialRenderFixture, executeNativeEdgeRenderUpdateFixture } from './edge-ssr-host-fixture.js';
import { publishEdgeCssDomUpdate } from './edge-css-publication.js';
import { InMemoryEdgeRenderStateStore, diffRenderPlansToPatchFrames, readEdgeRenderStateContents, type PatchFrame } from './projection.js';
import { nativeCssStoryContext } from './native-css-story-fixture.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- use the same native emission as the DOM-free Edge host.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';

export default { title: 'CEM Elements/Edge CSS Publication', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

export const ChangedCssAndDomPublishTogether: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing Edge CSS fixture');
        const scope = createCemDeclarationScope({ document });
        const controller = new AbortController();
        let onMarker: (() => void) | undefined;
        const tag = `edge-css-${crypto.randomUUID()}`;
        customElements.define(tag, class extends HTMLElement {
            static observedAttributes = ['data-cem-css-context'];
            attributeChangedCallback() { const callback = onMarker; onMarker = undefined; callback?.(); }
        });
        const host = document.createElement(tag); host.setAttribute('scope', 'library'); root.append(host);
        const owners = new Map<string, DeclarationStyleOwnership>();
        for (const identity of ['private', 'shared']) {
            const element = document.createElement('div'); root.append(element);
            const owner = new DeclarationStyleOwnership(document, scope); owner.add(element, scope); owners.set(identity, owner);
        }
        try {
            const snapshot = edgeSsrSnapshotFixture(); snapshot.producedTag = tag;
            snapshot.scopePolicyStamp += ':retained-declaration-css:retained-instance-css';
            snapshot.payload.nodes = [{ kind: 'element', key: 'css', tag: 'style', namespace: null, attributes: {}, slot: null,
                children: [{ kind: 'text', key: 'css/text', text: 'p { --instance: old; }' }] }];
            snapshot.payload.slots = {};
            const exported = exportDataIslandSnapshotForEdge(snapshot, { fields: { hostAttributes: 'allow', dataset: 'allow',
                payload: 'allow', slices: 'allow', formData: 'allow', validationState: 'allow', eventPayloads: 'allow' } });
            const initial = createCemEdgeSsrHostRequestEnvelope(new CemEdgeSsrJobSequence(), 'render-initial', {
                template: { kind: 'serialized-template-source-v1', templateArtifactId: snapshot.templateArtifactId,
                    source: [{ kind: 'element', namespace: null, tag: 'p', attributes: [], children: [{ kind: 'text', text: '${$label}' }] }] },
                snapshot: exported, sourceMapMode: 'dev', scopeUid: 'edge-css', revision: {
                    instanceId: snapshot.instanceId, dataRevision: snapshot.dataRevision, renderAttempt: snapshot.renderAttempt,
                    templateArtifactId: snapshot.templateArtifactId, scopePolicyStamp: snapshot.scopePolicyStamp, outputTarget: snapshot.outputTarget },
            });
            const store = new InMemoryEdgeRenderStateStore();
            const transport = { native: wasm, baseUrl: 'https://example.test/', context: nativeCssStoryContext,
                signal: controller.signal, read: async () => { throw new Error('unexpected import'); } };
            const seeded = await executeNativeSsrInitialRenderFixture(initial, store, transport);
            if (seeded.outcome !== 'success') throw new Error('initial native SSR failed');
            let current = seeded.result.renderState;
            host.innerHTML = `${seeded.result.instanceStylesheetHtml}<template data-cem-island="instance"></template><!--start-->${seeded.result.renderedHtml}<!--end-->`;
            const comments = Array.from(host.childNodes).filter((node): node is Comment => node.nodeType === Node.COMMENT_NODE);
            const bounds = { start: comments[0], end: comments[1] };
            const instanceStyle = host.querySelector('style');
            for (const variant of ['first', 'recover', 'unchanged']) {
                const cssVariant = variant === 'unchanged' ? 'recover' : variant;
                const payload = structuredClone(initial.payload);
                payload.snapshot.hostAttributes.label = variant;
                payload.snapshot.dataRevision = variant; payload.revision.dataRevision = variant;
                const source = payload.snapshot.payload.nodes[0];
                if (source.kind !== 'element') throw new Error('missing CSS payload');
                source.children = [{ kind: 'text', key: 'css/text', text: `p { --instance: ${cssVariant}; }` }];
                const context = { ...nativeCssStoryContext, frames: [{ frameId: cssVariant, baseUrl: 'https://example.test/', scopes: [],
                    specifiers: { imports: {}, resources: { icon: { target: `./${cssVariant}.svg` } } } }] };
                const request = createCemEdgeSsrHostRequestEnvelope(new CemEdgeSsrJobSequence(), 'render-update', { ...payload,
                    previousRenderPlan: { stateKey: current.stateKey, expectedEtag: current.etag,
                        address: { ...current.currentRenderPlan, kind: 'render-plan' }, identity: current.renderRevision } });
                const frames: PatchFrame[] = []; let result: CemEdgeSsrRenderUpdateResult | undefined;
                for await (const response of executeNativeEdgeRenderUpdateFixture(request, store, { ...transport, context,
                    declarations: ['private', 'shared'].map(identity => ({ owner: { kind: 'declaration' as const, identity, tag },
                        sources: [{ css: `:host { --${identity}: url(icon); }`, scope: identity === 'shared' ? 'library' : null }],
                        baseUrl: transport.baseUrl, context })) }, controller.signal)) {
                    if (response.outcome === 'progress') frames.push(response.result.frame);
                    else if (response.outcome === 'success') result = response.result;
                    else throw new Error(response.diagnostics.map(d => d.message).join('\n'));
                }
                if (!result) throw new Error('missing terminal Edge success');
                const previousEtag = current.etag;
                const options = { element: host, bounds, scope, declarations: owners, frames, result, previousEtag,
                    currentState: () => current, adopt: (state: typeof current) => { current = state; }, signal: controller.signal };
                const corrupt = structuredClone(result);
                if (!corrupt.stylesheets) throw new Error('missing emitted batch');
                corrupt.stylesheets.batch.instance[0].css += 'p {color:red}';
                expect(await publishEdgeCssDomUpdate({ ...options, result: corrupt })).toMatchObject({ status: 'rejected' });
                expect(current.etag).toBe(previousEtag);
                if (variant === 'recover') onMarker = () => host.querySelector('p')?.remove();
                const applied = await publishEdgeCssDomUpdate(options);
                if (variant === 'recover') {
                    expect(applied.status).toBe('recovery-required');
                    expect(current.etag).toBe(previousEtag);
                    const retained = readEdgeRenderStateContents(store, result.renderState);
                    if (!retained.ok) throw new Error('missing authoritative recovery plan');
                    expect(await publishEdgeCssDomUpdate({ ...options, recovery: true,
                        frames: diffRenderPlansToPatchFrames(null, retained.contents.renderPlan) })).toMatchObject({ status: 'applied' });
                } else expect(applied.status).toBe('applied');
                expect(host.querySelector('p')?.textContent).toBe(variant);
                expect(getComputedStyle(host.querySelector('p') as Element).getPropertyValue('--instance').trim()).toBe(cssVariant);
                expect(getComputedStyle(host).getPropertyValue('--private')).toContain(`/${cssVariant}.svg`);
                expect(getComputedStyle(host).getPropertyValue('--shared')).toContain(`/${cssVariant}.svg`);
                expect(host.querySelector('style')).toBe(instanceStyle);
                expect(await publishEdgeCssDomUpdate(options)).toMatchObject({ status: 'rejected' });
                expect(current.etag).toBe(result.renderState.etag);
            }
            controller.abort();
            expect(host.querySelector('style[data-cem-instance-style]')).toBeNull();
            expect(root.querySelectorAll('style[data-cem-declaration-style]')).toHaveLength(0);
        } finally { controller.abort(); scope.dispose(); root.replaceChildren(); }
    },
};
