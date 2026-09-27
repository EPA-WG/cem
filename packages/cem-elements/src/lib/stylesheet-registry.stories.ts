import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { DeclarationStyleOwnership, type DeclarationStylesheetCommit } from './declaration-style-ownership.js';
import { CemStylesheetRegistry, type CemStylesheetSource } from './internal/runtime-support/stylesheet-registry.js';
import type { CemStylesheetResponse } from './internal/runtime-support/stylesheet-installation.js';
import { CemCssDomPublicationQueue } from './internal/runtime-support/css-dom-publication-queue.js';
import { applyRenderPlanToRange, diffRenderPlansToPatchFrames, preparePatchFramesForRange,
    renderPlanIdentity, type RenderPlan } from './projection.js';
import { nativeCssStoryHost, nativeCssStoryContext } from './native-css-story-fixture.js';

export default { title: 'CEM Elements/Retained Stylesheet Registry', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

function context(variant: string) {
    return { ...nativeCssStoryContext, frames: [{ frameId: variant, baseUrl: 'https://example.test/', scopes: [],
        specifiers: { imports: {}, resources: { icon: { target: `./${variant}.svg` } } } }] };
}
function fixture(root: HTMLElement, fallback: boolean) {
    const native = nativeCssStoryHost(fallback);
    const registry = new CemStylesheetRegistry(document);
    const scope = createCemDeclarationScope({ document });
    const consumers = createCemDeclarationScope({ document });
    function instance(tag: string, sharedScope = 'controls') {
        const element = document.createElement(tag);
        element.setAttribute('scope', sharedScope);
        element.innerHTML = '<template data-cem-island="instance"></template><p>Retained content</p>';
        root.append(element);
        return element;
    }
    async function source(tag: string, css: Array<{ css: string; scope: string | null }>, read?: CemStylesheetSource['read']) {
        const element = document.createElement('div'); root.append(element);
        const ownership = new DeclarationStyleOwnership(document, scope); ownership.add(element, scope);
        const { artifact } = await native.compile(tag, css);
        const options: CemStylesheetSource = { declaration: {}, scope, ownership, host: native.host, artifact,
            baseUrl: 'https://example.test/main.css', occurrences: css.map((sheet, index) => ({ index,
                scope: sheet.scope === null ? { kind: 'private', tag } : { kind: 'shared', name: sheet.scope } })),
            read: read ?? (async () => { throw new Error('unexpected fixture request'); }) };
        return { element, options };
    }
    return { native, registry, scope, consumers, instance, source,
        async dispose() { await registry.dispose(); scope.dispose(); consumers.dispose(); native.dispose(); root.replaceChildren(); } };
}
function heldResponse() {
    let resolve: (response: CemStylesheetResponse) => void = () => { throw new Error('missing response resolver'); };
    const promise = new Promise<CemStylesheetResponse>(done => { resolve = done; });
    return { promise, resolve: (css: string) => resolve({ bytes: new TextEncoder().encode(css).buffer,
        finalUrl: 'https://example.test/child.css', contentType: 'text/css' }) };
}

export const QueuedConnectionActivation: Story = {
    render: () => '<section aria-label="Queued registry activation"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing queued registry fixture');
        for (const fallback of [false, true]) for (const mode of ['replace', 'empty', 'reject', 'wrong-group', 'activation-failed', 'cleanup-cancelled']) {
            const f = fixture(root, fallback);
            try {
                const element = f.instance('registry-queued');
                const own = await f.source(element.localName, [{ css: ':host { --private: url(icon); }', scope: null }]);
                if (mode !== 'empty') f.registry.register(own.options);
                const options = { element, declaration: own.options.declaration, sharedScope: 'controls',
                    scope: f.consumers, context: context('old') };
                const initial = f.registry.prepareReplacement(options);
                await initial.ready;
                const oldEntries = initial.takeCommits();
                if (!oldEntries) throw new Error('missing initial entries');
                if (oldEntries.length) expect(DeclarationStyleOwnership.commitGroup(oldEntries)).toBe(true);
                const active = initial.activate();
                if (!active) throw new Error('missing initial connection');
                const bounds = { start: document.createComment('start'), end: document.createComment('end') };
                element.append(bounds.start, bounds.end);
                const plan = (text: string): RenderPlan => ({ producedTag: element.localName, instanceId: 'instance',
                    templateArtifactId: 'template', dataRevision: text, outputTarget: 'light-dom', scopePolicyStamp: 'policy',
                    nodes: [{ kind: 'text', text, renderNodeId: 'text' }] });
                const before = plan('Before'), after = plan('After');
                applyRenderPlanToRange(bounds, before, document);
                const queue = CemCssDomPublicationQueue.forElement(element);
                const observations: Array<{ retired: boolean; text: string | null }> = [];
                let pending: ReturnType<typeof f.registry.prepareReplacement> | undefined;
                let transferred: readonly DeclarationStylesheetCommit[] = [];
                oldEntries[0]?.lease.signal.addEventListener('abort', () => {
                    observations.push({ retired: active.signal.aborted, text: bounds.start.nextSibling?.textContent ?? null });
                    if (mode === 'cleanup-cancelled') pending?.release();
                }, { once: true });
                const result = await queue.publish(async () => {
                    pending = f.registry.prepareReplacement({ ...options, context: context('new') });
                    expect(await pending.ready).toMatchObject({ status: 'prepared' });
                    const entries = pending.takeCommits();
                    if (!entries) throw new Error('missing replacement entries');
                    transferred = entries;
                    const revision = renderPlanIdentity(after);
                    const patch = preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(before, after), revision, document);
                    if (mode === 'reject') patch.cancel();
                    if (mode === 'activation-failed') pending.activate = () => undefined;
                    return { entries: mode === 'wrong-group' ? [...entries] : entries, registryConnection: pending,
                        patch, currentRevision: () => revision };
                });
                const rejected = mode === 'reject' || mode === 'wrong-group';
                const recovery = mode === 'activation-failed' || mode === 'cleanup-cancelled';
                expect(result.status).toBe(rejected ? 'rejected' : recovery ? 'recovery-required' : 'applied');
                expect(bounds.start.nextSibling?.textContent).toBe(rejected ? 'Before' : 'After');
                expect(queue.recoveryRequired).toBe(recovery);
                if (rejected) {
                    expect(pending?.signal.aborted).toBe(true);
                    expect(active.signal.aborted).toBe(false);
                    expect(getComputedStyle(element).getPropertyValue('--private')).toContain('/old.svg');
                } else if (!recovery) {
                    expect(active.signal.aborted).toBe(true);
                    expect(pending?.signal.aborted).toBe(false);
                    expect(observations).toEqual(mode === 'empty' ? [] : [{ retired: true, text: 'After' }]);
                    // Reusing an adopted handle must reject without tearing it down.
                    const revision = renderPlanIdentity(after);
                    expect(await queue.publish(() => ({ entries: transferred, registryConnection: pending,
                        patch: preparePatchFramesForRange(bounds, diffRenderPlansToPatchFrames(after, after), revision, document),
                        currentRevision: () => revision }))).toMatchObject({ status: 'rejected' });
                    expect(pending?.isPublished()).toBe(true);
                    expect(pending?.signal.aborted).toBe(false);
                }
                if (recovery) {
                    let prepared = false;
                    await queue.publish(() => { prepared = true; throw new Error('blocked factory'); });
                    expect(prepared).toBe(false);
                    await queue.recover(async () => {
                        pending?.release(); active.release();
                        const restored = f.registry.connect(options);
                        await restored.whenReady();
                        applyRenderPlanToRange(bounds, before, document);
                    });
                    expect(queue.recoveryRequired).toBe(false);
                    expect(getComputedStyle(element).getPropertyValue('--private')).toContain('/old.svg');
                }
                expect(f.native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { await f.dispose(); }
        }
    },
};

export const PreparedConnectionReplacement: Story = {
    render: () => '<section aria-label="Prepared registry replacement"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing prepared registry fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const element = f.instance('registry-prepared');
                let gate = heldResponse();
                gate.resolve(':host { --private: url(icon); }');
                let reads = 0; let fail = false;
                const own = await f.source(element.localName, [{ css: '@import "./child.css";', scope: null }], async () => {
                    reads++;
                    if (fail) throw new Error('replacement import failed');
                    return gate.promise;
                });
                const shared = await f.source('registry-shared', [{ css: ':host { --shared: url(icon); }', scope: 'controls' }]);
                f.registry.register(own.options); f.registry.register(shared.options);
                const options = { element, declaration: own.options.declaration, sharedScope: 'controls',
                    scope: f.consumers, context: context('old') };
                const active = f.registry.connect(options);
                expect(await active.whenReady()).toMatchObject({ installed: 2 });
                const marker = element.getAttribute('data-cem-css-context');
                const oldStyle = own.element.querySelector('style');
                gate = heldResponse();
                const pending = f.registry.prepareReplacement({ ...options, context: context('new') });
                expect(pending.takeCommits()).toBeUndefined();
                await waitFor(() => expect(reads).toBe(2));
                expect(active.signal.aborted).toBe(false);
                expect(element.getAttribute('data-cem-css-context')).toBe(marker);
                expect(own.element.querySelector('style')).toBe(oldStyle);
                expect(getComputedStyle(element).getPropertyValue('--private')).toContain('/old.svg');
                gate.resolve(':host { --private: url(icon); }');
                expect(await pending.ready).toMatchObject({ status: 'prepared' });
                const entries = pending.takeCommits();
                if (!entries) throw new Error('missing prepared entries');
                expect(entries).toHaveLength(2);
                expect(pending.takeCommits()).toBeUndefined();
                expect(pending.activate()).toBeUndefined();
                expect(DeclarationStyleOwnership.commitGroup(entries)).toBe(true);
                const current = pending.activate();
                if (!current) throw new Error('missing activated connection');
                expect(active.signal.aborted).toBe(true);
                expect(await current.whenReady()).toMatchObject({ installed: 2 });
                active.release();
                for (const name of ['--private', '--shared']) expect(getComputedStyle(element).getPropertyValue(name)).toContain('/new.svg');

                fail = true;
                const failed = f.registry.prepareReplacement({ ...options, context: context('failed') });
                expect(await failed.ready).toMatchObject({ status: 'cancelled' });
                expect(current.signal.aborted).toBe(false);
                expect(getComputedStyle(element).getPropertyValue('--private')).toContain('/new.svg');
                fail = false;
                const extra = await f.source('registry-extra', [{ css: ':host { --extra: yes; }', scope: 'unused' }]);
                const invalidated = f.registry.prepareReplacement({ ...options, context: context('obsolete') });
                await invalidated.ready;
                const stale = invalidated.takeCommits();
                if (!stale) throw new Error('missing stale candidates');
                f.registry.register(extra.options);
                expect(invalidated.signal.aborted).toBe(true);
                expect(DeclarationStyleOwnership.commitGroup(stale)).toBe(false);
                expect(getComputedStyle(element).getPropertyValue('--private')).toContain('/new.svg');

                const clearShared = f.registry.prepareReplacement({ ...options, sharedScope: 'other', context: context('new') });
                expect(await clearShared.ready).toMatchObject({ status: 'prepared' });
                const clearing = clearShared.takeCommits();
                if (!clearing) throw new Error('missing clearing entries');
                expect(clearing.some(entry => entry.outputs.length === 0)).toBe(true);
                expect(DeclarationStyleOwnership.commitGroup(clearing)).toBe(true);
                const remaining = clearShared.activate();
                if (!remaining) throw new Error('missing remaining connection');
                expect(await remaining.whenReady()).toMatchObject({ installed: 1 });
                expect(getComputedStyle(element).getPropertyValue('--shared').trim()).toBe('');
                expect(getComputedStyle(element).getPropertyValue('--private')).toContain('/new.svg');
                const abandoned = f.registry.prepareReplacement({ ...options, context: context('abandoned') });
                await abandoned.ready;
                abandoned.release();
                expect(remaining.signal.aborted).toBe(false);
                expect(getComputedStyle(element).getPropertyValue('--private')).toContain('/new.svg');
                gate = heldResponse();
                const previousReads = reads;
                const cancelled = f.registry.prepareReplacement({ ...options, context: context('cancelled') });
                await waitFor(() => expect(reads).toBeGreaterThan(previousReads));
                remaining.release();
                expect(await cancelled.ready).toMatchObject({ status: 'cancelled' });
                expect(cancelled.signal.aborted).toBe(true);
                gate.resolve(':host { --private: late; }');
                await f.registry.flush();
                expect(f.native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { await f.dispose(); }
        }
    },
};

export const SharedSourcesAndContextReadiness: Story = {
    render: () => '<section aria-label="Shared stylesheet source registry"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing registry fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const a = f.instance('registry-a'), b = f.instance('registry-b');
                const excluded = f.instance('registry-excluded', 'other');
                const gate = heldResponse(); let requested = 0;
                const first = await f.source('registry-a', [
                    { css: ':host { --private: own; }', scope: null },
                    { css: '@import "./child.css";', scope: 'controls' },
                ], async () => { requested++; return gate.promise; });
                const connect = (element: HTMLElement, declaration: object, variant: string, sharedScope = 'controls') => f.registry.connect({
                    element, declaration, sharedScope, scope: f.consumers, context: context(variant) });
                const ca = connect(a, first.options.declaration, 'a');
                const cb = connect(b, {}, 'b');
                const ce = connect(excluded, {}, 'a', 'other');
                expect(await ca.whenReady()).toMatchObject({ status: 'ready', installed: 0 });
                const registration = f.registry.register(first.options);
                let settled = false;
                const pending = ca.whenReady().then(result => { settled = true; return result; });
                await waitFor(() => expect(requested).toBe(2));
                expect(settled).toBe(false);
                // This source has no produced instances, but must style both shared consumers.
                const second = await f.source('registry-source-only', [{ css: ':host { --second: url(icon); }', scope: 'controls' }]);
                const otherRegistration = f.registry.register(second.options);
                gate.resolve(':host { --shared: url(icon); }');
                expect(await pending).toMatchObject({ status: 'ready', installed: 3, diagnostics: [] });
                expect(await cb.whenReady()).toMatchObject({ status: 'ready', installed: 2, diagnostics: [] });
                expect(await ce.whenReady()).toMatchObject({ status: 'ready', installed: 0 });
                expect(getComputedStyle(a).getPropertyValue('--private').trim()).toBe('own');
                expect(getComputedStyle(b).getPropertyValue('--private').trim()).toBe('');
                expect(getComputedStyle(excluded).getPropertyValue('--private').trim()).toBe('');
                for (const [element, variant] of [[a, 'a'], [b, 'b']] as const) {
                    expect(getComputedStyle(element).getPropertyValue('--shared')).toContain(`/${variant}.svg`);
                    expect(getComputedStyle(element).getPropertyValue('--second')).toContain(`/${variant}.svg`);
                }
                expect(first.element.querySelectorAll('style')).toHaveLength(3);
                expect(second.element.querySelectorAll('style')).toHaveLength(2);
                registration.release();
                expect(await ca.whenReady()).toMatchObject({ installed: 1 });
                expect(getComputedStyle(a).getPropertyValue('--private').trim()).toBe('');
                expect(a.hasAttribute('data-cem-css-context')).toBe(true);
                otherRegistration.release(); await f.registry.flush();
                expect(a.hasAttribute('data-cem-css-context')).toBe(false);
                expect(b.hasAttribute('data-cem-css-context')).toBe(false);
                expect(f.native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { await f.dispose(); }
        }
    },
};

export const PendingRemovalAndContextReplacement: Story = {
    render: () => '<section aria-label="Registry source replacement"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing replacement fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const element = f.instance('registry-reconnect');
                const content = element.querySelector('p');
                const gate = heldResponse(); let signal: AbortSignal | undefined;
                const source = await f.source('registry-reconnect', [{ css: '@import "./child.css";', scope: 'controls' }],
                    async (_request, current) => { signal = current; return gate.promise; });
                const registration = f.registry.register(source.options);
                const options = { element, declaration: {}, scope: f.consumers, sharedScope: 'controls', context: context('old') };
                const first = f.registry.connect(options);
                const pending = first.whenReady();
                await waitFor(() => expect(signal).toBeDefined());
                registration.release();
                expect(await pending).toMatchObject({ status: 'ready', installed: 0 });
                expect(signal?.aborted).toBe(true);
                await f.registry.flush();
                f.registry.register({ ...source.options, read: async () => ({
                    bytes: new TextEncoder().encode(':host { --current: url(icon); }').buffer,
                    finalUrl: 'https://example.test/child.css', contentType: 'text/css' }) });
                const current = f.registry.connect({ ...options, context: context('new') });
                expect(await first.whenReady()).toMatchObject({ status: 'cancelled' });
                expect(await current.whenReady()).toMatchObject({ status: 'ready', installed: 1, diagnostics: [] });
                gate.resolve(':host { --stale: yes; }');
                registration.release(); // An old handle cannot remove the replacement source.
                await f.registry.flush();
                expect(getComputedStyle(element).getPropertyValue('--current')).toContain('/new.svg');
                expect(getComputedStyle(element).getPropertyValue('--stale').trim()).toBe('');
                element.remove(); root.append(element);
                expect(await current.whenReady()).toMatchObject({ status: 'cancelled' });
                const reconnect = f.registry.connect({ ...options, context: context('reconnected') });
                expect(await reconnect.whenReady()).toMatchObject({ status: 'ready', installed: 1 });
                expect(getComputedStyle(element).getPropertyValue('--current')).toContain('/reconnected.svg');
                expect(element.querySelector('p')).toBe(content);
                reconnect.release(); await f.registry.flush();
                expect(source.element.querySelectorAll('style')).toHaveLength(0);
                expect(element.hasAttribute('data-cem-css-context')).toBe(false);
            } finally { await f.dispose(); }
        }
    },
};

export const AncestorDisposalAndDisconnectedConsumers: Story = {
    render: () => '<section aria-label="Registry scope ownership"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing disposal fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const a = f.instance('registry-scope'), b = f.instance('registry-scope');
                const child = createCemDeclarationScope({ document, parent: f.consumers });
                const source = await f.source('registry-scope', [{ css: ':host { --asset: url(icon); }', scope: 'controls' }]);
                f.registry.register(source.options);
                const connect = (element: HTMLElement) => f.registry.connect({ element, declaration: {},
                    scope: child, sharedScope: 'controls', context: context('a') });
                const ca = connect(a), cb = connect(b);
                await Promise.all([ca.whenReady(), cb.whenReady()]);
                expect(source.element.querySelectorAll('style')).toHaveLength(1);
                a.remove();
                expect(await ca.whenReady()).toMatchObject({ status: 'cancelled' });
                expect(source.element.querySelectorAll('style')).toHaveLength(1);
                f.consumers.dispose();
                expect(await cb.whenReady()).toMatchObject({ status: 'cancelled' });
                await f.registry.flush();
                expect(source.element.querySelectorAll('style')).toHaveLength(0);
                expect(b.hasAttribute('data-cem-css-context')).toBe(false);
                const parent = createCemDeclarationScope({ document });
                const sourceChild = createCemDeclarationScope({ document, parent });
                const other = await f.source('registry-scope-other', [{ css: ':host { --other: yes; }', scope: 'controls' }]);
                f.registry.register({ ...other.options, scope: sourceChild });
                const survivor = f.registry.connect({ element: b, declaration: {}, scope: f.scope,
                    sharedScope: 'controls', context: context('b') });
                expect(await survivor.whenReady()).toMatchObject({ installed: 2 });
                parent.dispose();
                expect(await survivor.whenReady()).toMatchObject({ installed: 1 });
                expect(getComputedStyle(b).getPropertyValue('--other').trim()).toBe('');
                sourceChild.dispose();
            } finally { await f.dispose(); }
        }
    },
};

export const ReentrantConnectionReplacement: Story = {
    render: () => '<section aria-label="Registry reentrant cancellation"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing reentrant fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const element = f.instance('registry-reentrant');
                const options = { element, declaration: {}, scope: f.consumers, sharedScope: 'controls', context: context('old') };
                const gate = heldResponse(); let reads = 0;
                let reentrant: ReturnType<CemStylesheetRegistry['connect']> | undefined;
                const source = await f.source('registry-reentrant', [{ css: '@import "./child.css";', scope: 'controls' }],
                    async (_request, signal) => {
                        if (++reads === 1) {
                            signal.addEventListener('abort', () => {
                                reentrant = f.registry.connect({ ...options, context: context('reentrant') });
                            }, { once: true });
                            return gate.promise;
                        }
                        return { bytes: new TextEncoder().encode(':host { --asset: url(icon); }').buffer,
                            finalUrl: 'https://example.test/child.css', contentType: 'text/css' };
                    });
                f.registry.register(source.options);
                const old = f.registry.connect(options);
                await waitFor(() => expect(reads).toBe(1));
                const superseded = f.registry.connect({ ...options, context: context('superseded') });
                expect(await old.whenReady()).toMatchObject({ status: 'cancelled' });
                expect(await superseded.whenReady()).toMatchObject({ status: 'cancelled' });
                expect(reentrant).toBeDefined();
                expect(await reentrant?.whenReady()).toMatchObject({ status: 'ready', installed: 1, diagnostics: [] });
                gate.resolve(':host { --late: yes; }');
                await f.registry.flush();
                expect(reads).toBe(2);
                expect(getComputedStyle(element).getPropertyValue('--asset')).toContain('/reentrant.svg');
                expect(getComputedStyle(element).getPropertyValue('--late').trim()).toBe('');
                reentrant?.release(); await f.registry.flush();
                expect(source.element.querySelectorAll('style')).toHaveLength(0);
            } finally { await f.dispose(); }
        }
    },
};
