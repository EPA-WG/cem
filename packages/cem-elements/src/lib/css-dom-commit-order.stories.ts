import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { DeclarationStyleOwnership, type DeclarationStylesheetCommit } from './declaration-style-ownership.js';
import { nativeCssStoryHost, nativeCssStoryContext } from './native-css-story-fixture.js';
import { deferStylesheetNotifications } from './internal/runtime-support/stylesheet-notifications.js';
import { prepareRetainedStylesheets, type CemPreparedStylesheets } from './internal/runtime-support/stylesheet-installation.js';
import { prepareInstanceStylesheets, type PreparedInstanceStylesheets } from './internal/runtime-support/instance-stylesheet-installation.js';
import { applyRenderPlanToRange, diffRenderPlansToPatchFrames, preparePatchFramesForRange,
    renderPlanIdentity, type RenderPlan } from './projection.js';

export default { title: 'CEM Elements/CSS and DOM Commit Ordering Evidence', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

/** Publication ordering and the notification boundary; Edge integration remains separate. */
function publicationStory(orders: readonly string[]): Story { return {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing ordering fixture');
        for (const fallback of [false, true]) for (const order of orders) {
            const native = nativeCssStoryHost(fallback);
            const scope = createCemDeclarationScope({ document });
            const owner = new DeclarationStyleOwnership(document, scope);
            const declaration = document.createElement('div'); root.append(declaration); owner.add(declaration, scope);
            const tag = `ordering-host-${crypto.randomUUID()}`;
            const childTag = `ordering-child-${crypto.randomUUID()}`;
            const observed: Array<{ callback: string; text: string; asset: string }> = [];
            let recording = false;
            let onMarker: (() => void) | undefined;
            let onConnect: (() => void) | undefined;
            const released: string[] = [];
            const instanceObservations: string[] = [];
            const observe = (callback: string) => {
                if (recording) observed.push({ callback, text: host.querySelector('[data-content]')?.textContent ?? '',
                    asset: getComputedStyle(host).getPropertyValue('--asset') });
            };
            customElements.define(tag, class extends HTMLElement {
                static observedAttributes = ['data-cem-css-context'];
                attributeChangedCallback() { observe('marker'); const callback = onMarker; onMarker = undefined; callback?.(); }
            });
            customElements.define(childTag, class extends HTMLElement {
                connectedCallback() {
                    observe('connected');
                    instanceObservations.push(getComputedStyle(host).getPropertyValue('--instance').trim());
                    const callback = onConnect; onConnect = undefined; callback?.();
                }
            });
            const host = document.createElement(tag); root.append(host);
            const bounds = { start: document.createComment('start'), end: document.createComment('end') };
            host.append(bounds.start, bounds.end);
            const releases: Promise<unknown>[] = [];
            const preparations: CemPreparedStylesheets[] = [];
            const instancePreparations: PreparedInstanceStylesheets[] = [];
            try {
                const { artifact } = await native.compile(tag, [{ css: ':host { --asset: url(asset); }', scope: null }]);
                const candidate = async (variant: string): Promise<DeclarationStylesheetCommit> => {
                    const lease = owner.stageConsumer(host, scope);
                    const consumer = `ordering-${variant}`;
                    const context = { ...nativeCssStoryContext, frames: [{ frameId: 'page', baseUrl: 'https://example.test/',
                        scopes: [], specifiers: { imports: {}, resources: { asset: { target: `./${variant}.svg` } } } }] };
                    if (order.startsWith('prepared-') && variant === 'new') {
                        const prepared = prepareRetainedStylesheets({ host: native.host, artifact, consumer, lease, context,
                            baseUrl: 'https://example.test/main.css', occurrences: [{ index: 0, scope: { kind: 'private', tag } }],
                            read: async () => { throw new Error('unexpected import'); } });
                        preparations.push(prepared);
                        expect(await prepared.ready).toMatchObject({ status: 'prepared' });
                        const entry = prepared.takeCommit();
                        if (!entry) throw new Error('missing prepared commit');
                        expect(prepared.takeCommit()).toBeUndefined();
                        expect(prepared.commit()).toBe(false);
                        if (order === 'prepared-disposed') await prepared.dispose();
                        if (order === 'prepared-released') entry.release();
                        if (order === 'prepared-invalid') entry.isCurrent = () => false;
                        return { ...entry, release() { observe('release'); released.push(variant); entry.release(); } };
                    }
                    const output = await native.host.stylesheet({ action: 'begin', artifact, consumer, index: 0,
                        baseUrl: 'https://example.test/main.css', context, scope: { kind: 'private', tag } }).result;
                    if (output.status !== 'ready') throw new Error('expected ready CSS');
                    return { lease, outputs: [{ index: 0, scope: { kind: 'private' }, output }], release() {
                        observe('release');
                        released.push(variant);
                        releases.push(native.host.stylesheet({ action: 'release', artifact, consumer, loadId: output.loadId }).result);
                        if (order === 'cleanup-error' && variant === 'old') throw new Error('cleanup fixture');
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
                let instanceStyles: PreparedInstanceStylesheets | undefined;
                let instanceStyleNode: HTMLStyleElement | null = null;
                if (order.startsWith('instance-')) {
                    const prepare = (value: string, empty = false) => {
                        const result = prepareInstanceStylesheets({ host: native.host, element: host, instanceId: 'instance',
                            context: nativeCssStoryContext, baseUrl: 'https://example.test/',
                            sources: empty ? [] : [{ css: `:host { --instance: ${value}; }`, scope: null }],
                            artifactId: `instance-${value}`, scopePolicyStamp: 'native-story', signal: new AbortController().signal,
                            read: async () => { throw new Error('unexpected instance import'); } });
                        instancePreparations.push(result);
                        return result;
                    };
                    const active = prepare('old');
                    expect(await active.ready).toMatchObject({ status: 'prepared' });
                    expect(active.commit()).toBe(true);
                    instanceStyleNode = host.querySelector('style[data-cem-instance-style]');
                    instanceStyles = prepare('new', order === 'instance-clear');
                    expect(await instanceStyles.ready).toMatchObject({ status: 'prepared' });
                    expect(instanceStyles.check()).toBe(true);
                    expect(getComputedStyle(host).getPropertyValue('--instance').trim()).toBe('old');
                    if (order === 'instance-reused') instanceStyles = active;
                }
                let patch = preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, after), renderPlanIdentity(after), document);
                expect(patch.check(renderPlanIdentity(after)).status).toBe('ready');
                recording = true;
                if (!['css-first', 'dom-first', 'deferred-cleanup'].includes(order)) {
                    const controller = new AbortController();
                    let current = renderPlanIdentity(after);
                    if (['reject-patch', 'prepared-rejected', 'instance-rejected'].includes(order)) patch.cancel();
                    if (order === 'instance-disposed') instanceStyles?.dispose();
                    if (order === 'instance-invalidated') onMarker = () => instanceStyles?.dispose();
                    if (order === 'instance-failure') onMarker = () => {
                        const append = host.append;
                        host.append = () => { host.append = append; throw new Error('instance publication fixture'); };
                    };
                    if (order === 'reject-css') css.outputs = [...css.outputs, ...css.outputs];
                    if (order === 'cancel-before') controller.abort();
                    if (order === 'cancel-during') onMarker = () => controller.abort();
                    if (order === 'supersede') onMarker = () => { current = { ...current, dataRevision: '3' }; };
                    if (order === 'supersede-child') onConnect = () => { current = { ...current, dataRevision: '3' }; };
                    if (order === 'disconnect-child') onConnect = () => host.remove();
                    if (order === 'mutate-target') onMarker = () => {
                        const target = host.querySelector('[data-content]');
                        if (target) target.replaceWith(target.cloneNode(true));
                    };
                    if (order === 'foreign-host') {
                        const foreign = document.createElement('div'); root.append(foreign);
                        const range = { start: document.createComment('start'), end: document.createComment('end') };
                        foreign.append(range.start, range.end);
                        applyRenderPlanToRange(range, before, document);
                        patch = preparePatchFramesForRange(range, diffRenderPlansToPatchFrames(before, after), current, document);
                    }
                    const commit = () => DeclarationStyleOwnership.commitGroupWithPatch([css], patch, () => current, controller.signal, instanceStyles);
                    const result = order === 'nested' ? deferStylesheetNotifications(commit) : commit();
                    const rejected = ['reject-patch', 'reject-css', 'cancel-before', 'foreign-host', 'nested',
                        'prepared-rejected', 'prepared-disposed', 'prepared-invalid', 'prepared-released',
                        'instance-rejected', 'instance-disposed', 'instance-reused'].includes(order);
                    const completed = ['joint', 'cleanup-error', 'supersede-child', 'disconnect-child', 'prepared-joint',
                        'instance-joint', 'instance-clear'].includes(order);
                    expect(result.status).toBe(rejected ? 'rejected' : ['joint', 'prepared-joint', 'instance-joint', 'instance-clear'].includes(order) ? 'applied' : 'recovery-required');
                    expect(host.querySelector('[data-content]')?.textContent).toBe(completed ? 'After' : 'Before');
                    if (order !== 'disconnect-child') expect(getComputedStyle(host).getPropertyValue('--asset')).toContain(rejected ? '/old.svg' : '/new.svg');
                    expect(old.lease.signal.aborted).toBe(!rejected);
                    expect(released).toEqual([rejected ? 'new' : 'old']);
                    expect(patch.check(current).status).toBe('aborted');
                    expect(result.errors.length).toBe(['cleanup-error', 'instance-failure'].includes(order) ? 1 : 0);
                    if (instanceStyles) {
                        const expected = order === 'instance-clear' ? '' : ['instance-joint', 'instance-failure'].includes(order) ? 'new' : 'old';
                        expect(getComputedStyle(host).getPropertyValue('--instance').trim()).toBe(expected);
                        expect(host.querySelector('style[data-cem-instance-style]')).toBe(order === 'instance-clear' ? null : instanceStyleNode);
                        expect(instanceStyles.isPublished()).toBe(completed);
                        expect(instanceStyles.check()).toBe(false);
                        if (completed) expect(instanceObservations).toEqual([expected]);
                    }
                    if (completed && order !== 'disconnect-child') for (const callback of ['release', 'abort', 'connected']) {
                        expect(observed).toContainEqual({ callback, text: 'After', asset: expect.stringContaining('/new.svg') });
                    }
                    expect(native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
                    continue;
                } else if (order === 'deferred-cleanup') {
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
                recording = false; scope.dispose(); await Promise.all(preparations.map(prepared => prepared.dispose()));
                for (const prepared of instancePreparations) prepared.dispose();
                await Promise.all(releases); native.dispose(); root.replaceChildren();
            }
        }
    },
}; }

export const SynchronousObserversSeeSequentialPublication = publicationStory(['css-first', 'dom-first', 'deferred-cleanup']);
export const JointAdmissionAndRecovery = publicationStory([
    'joint', 'reject-patch', 'reject-css', 'cancel-before', 'cancel-during',
    'mutate-target', 'supersede', 'cleanup-error', 'foreign-host',
    'nested', 'supersede-child', 'disconnect-child',
]);
export const PreparedLoadsJoinPublication = publicationStory([
    'prepared-joint', 'prepared-rejected', 'prepared-disposed', 'prepared-invalid', 'prepared-released',
]);
export const InstanceStylesJoinPublication = publicationStory([
    'instance-joint', 'instance-clear', 'instance-rejected', 'instance-disposed', 'instance-invalidated', 'instance-reused', 'instance-failure',
]);
