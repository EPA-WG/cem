import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { DeclarationStyleOwnership, type DeclarationStylesheetCommit } from './declaration-style-ownership.js';
import { nativeCssStoryHost, nativeCssStoryContext } from './native-css-story-fixture.js';
import { deferStylesheetNotifications } from './internal/runtime-support/stylesheet-notifications.js';
import { applyRenderPlanToRange, diffRenderPlansToPatchFrames, preparePatchFramesForRange,
    renderPlanIdentity, type RenderPlan } from './projection.js';

export default { title: 'CEM Elements/CSS and DOM Commit Ordering Evidence', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

/** Publication ordering and the notification boundary; Edge integration remains separate. */
export const SynchronousObserversSeeSequentialPublication: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing ordering fixture');
        for (const fallback of [false, true]) for (const order of ['css-first', 'dom-first', 'deferred-cleanup']) {
            const native = nativeCssStoryHost(fallback);
            const scope = createCemDeclarationScope({ document });
            const owner = new DeclarationStyleOwnership(document, scope);
            const declaration = document.createElement('div'); root.append(declaration); owner.add(declaration, scope);
            const tag = `ordering-host-${crypto.randomUUID()}`;
            const childTag = `ordering-child-${crypto.randomUUID()}`;
            const observed: Array<{ callback: string; text: string; asset: string }> = [];
            let recording = false;
            const observe = (callback: string) => {
                if (recording) observed.push({ callback, text: host.querySelector('[data-content]')?.textContent ?? '',
                    asset: getComputedStyle(host).getPropertyValue('--asset') });
            };
            customElements.define(tag, class extends HTMLElement {
                static observedAttributes = ['data-cem-css-context'];
                attributeChangedCallback() { observe('marker'); }
            });
            customElements.define(childTag, class extends HTMLElement {
                connectedCallback() { observe('connected'); }
            });
            const host = document.createElement(tag); root.append(host);
            const bounds = { start: document.createComment('start'), end: document.createComment('end') };
            host.append(bounds.start, bounds.end);
            const releases: Promise<unknown>[] = [];
            try {
                const { artifact } = await native.compile(tag, [{ css: ':host { --asset: url(asset); }', scope: null }]);
                const candidate = async (variant: string): Promise<DeclarationStylesheetCommit> => {
                    const lease = owner.stageConsumer(host, scope);
                    const consumer = `ordering-${variant}`;
                    const context = { ...nativeCssStoryContext, frames: [{ frameId: 'page', baseUrl: 'https://example.test/',
                        scopes: [], specifiers: { imports: {}, resources: { asset: { target: `./${variant}.svg` } } } }] };
                    const output = await native.host.stylesheet({ action: 'begin', artifact, consumer, index: 0,
                        baseUrl: 'https://example.test/main.css', context, scope: { kind: 'private', tag } }).result;
                    if (output.status !== 'ready') throw new Error('expected ready CSS');
                    return { lease, outputs: [{ index: 0, scope: { kind: 'private' }, output }], release() {
                        observe('release');
                        releases.push(native.host.stylesheet({ action: 'release', artifact, consumer, loadId: output.loadId }).result);
                    } };
                };
                const plan = (after: boolean): RenderPlan => ({ producedTag: tag, instanceId: 'instance',
                    templateArtifactId: 'template', dataRevision: after ? '2' : '1', outputTarget: 'light-dom', scopePolicyStamp: 'policy',
                    nodes: [{ kind: 'element', namespace: null, tag: after ? childTag : 'div', renderNodeId: 'content',
                        attributes: [{ name: 'data-content', value: '' }], children: [{ kind: 'text', text: after ? 'After' : 'Before', renderNodeId: 'text' }] }] });
                const before = plan(false); const after = plan(true);
                const old = await candidate('old');
                expect(DeclarationStyleOwnership.commitGroup([old])).toBe(true);
                old.lease.signal.addEventListener('abort', () => observe('abort'), { once: true });
                applyRenderPlanToRange(bounds, before, document);
                const css = await candidate('new');
                const patch = preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, after), renderPlanIdentity(after), document);
                expect(patch.check(renderPlanIdentity(after)).status).toBe('ready');
                recording = true;
                if (order === 'deferred-cleanup') {
                    deferStylesheetNotifications(() => {
                        expect(DeclarationStyleOwnership.commitGroup([css])).toBe(true);
                        expect(old.lease.signal.aborted).toBe(false);
                        expect(observed.some(event => event.callback === 'release' || event.callback === 'abort')).toBe(false);
                        expect(patch.commit(renderPlanIdentity(after)).status).toBe('applied');
                    });
                    expect(old.lease.signal.aborted).toBe(true);
                    for (const callback of ['release', 'abort', 'connected']) {
                        expect(observed).toContainEqual({ callback, text: 'After', asset: expect.stringContaining('/new.svg') });
                    }
                    // Browser attribute callbacks still run synchronously during CSS publication.
                    expect(observed).toContainEqual({ callback: 'marker', text: 'Before', asset: expect.stringContaining('/new.svg') });
                } else if (order === 'css-first') {
                    expect(DeclarationStyleOwnership.commitGroup([css])).toBe(true);
                    expect(patch.commit(renderPlanIdentity(after)).status).toBe('applied');
                    expect(observed).toContainEqual({ callback: 'marker', text: 'Before', asset: expect.stringContaining('/new.svg') });
                } else {
                    expect(patch.commit(renderPlanIdentity(after)).status).toBe('applied');
                    expect(DeclarationStyleOwnership.commitGroup([css])).toBe(true);
                    expect(observed).toContainEqual({ callback: 'connected', text: 'After', asset: expect.stringContaining('/old.svg') });
                }
                expect(host.querySelector('[data-content]')?.textContent).toBe('After');
                expect(getComputedStyle(host).getPropertyValue('--asset')).toContain('/new.svg');
                expect(native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally {
                recording = false; scope.dispose(); await Promise.all(releases); native.dispose(); root.replaceChildren();
            }
        }
    },
};
