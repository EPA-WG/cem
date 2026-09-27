import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { serializeDeclarationStylesheets } from './declaration-style-markup.js';
import { DeclarationStyleOwnership, type DeclarationStylesheetCommit } from './declaration-style-ownership.js';
import { nativeCssStoryHost, nativeCssStoryContext } from './native-css-story-fixture.js';

export default { title: 'CEM Elements/Native Stylesheet Ownership', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

export const ContextConsumersAndDisposal: Story = {
    render: () => '<section aria-label="Native CSS ownership"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing ownership fixture root');
        for (const fallback of [false, true]) {
            const native = nativeCssStoryHost(fallback);
            const processingScope = createCemDeclarationScope({ document });
            const contextA = createCemDeclarationScope({ document, parent: processingScope });
            const contextB = createCemDeclarationScope({ document, parent: processingScope });
            const ownership = new DeclarationStyleOwnership(document, processingScope);
            const declaration = document.createElement('div');
            root.append(declaration);
            ownership.add(declaration, processingScope);
            const releases: Promise<unknown>[] = [];
            try {
                const { artifact } = await native.compile('native-owned-card', [
                    { css: ':host { --asset: url(icon); --order: first; } span { animation: pulse 20s infinite; } @keyframes pulse { to { opacity: .5; } }', scope: null },
                    { css: ':host { --order: last; }', scope: null },
                ]);
                const instances = Array.from({ length: 3 }, () => {
                    const instance = document.createElement('native-owned-card');
                    instance.innerHTML = '<template data-cem-island="instance"></template><span>Owned</span>';
                    root.append(instance);
                    return instance;
                });
                let sequence = 0;
                async function prepare(instance: HTMLElement, variant: 'a' | 'b') {
                    const lease = ownership.beginConsumer(instance, variant === 'a' ? contextA : contextB);
                    const consumer = `owned-${++sequence}`;
                    const context = { ...nativeCssStoryContext, frames: [{ frameId: variant,
                        baseUrl: 'https://example.test/', scopes: [],
                        specifiers: { imports: {}, resources: { icon: { target: `./${variant}.svg` } } } }] };
                    const outputs = await Promise.all([0, 1].map(async index => {
                        const output = await native.host.stylesheet({ action: 'begin', artifact, consumer, index,
                            baseUrl: 'https://example.test/main.css', context,
                            scope: { kind: 'private', tag: 'native-owned-card' } }).result;
                        if (output.status !== 'ready') throw new Error('expected ready stylesheet');
                        return { index, scope: { kind: 'private' as const }, output };
                    }));
                    expect(native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
                    const release = () => {
                        for (const { output } of outputs) releases.push(native.host.stylesheet({ action: 'release',
                            artifact, consumer, loadId: output.loadId }).result);
                    };
                    return { lease, outputs, release };
                }
                async function install(instance: HTMLElement, variant: 'a' | 'b') {
                    const pending = await prepare(instance, variant);
                    expect(pending.lease.commit(pending.outputs, pending.release)).toBe(true);
                    return pending;
                }
                const a = await install(instances[0], 'a');
                await install(instances[1], 'a');
                await install(instances[2], 'b');
                expect(declaration.querySelectorAll('style')).toHaveLength(3);
                expect(instances[0].getAttribute('data-cem-css-context')).toBe(instances[1].getAttribute('data-cem-css-context'));
                expect(instances[0].getAttribute('data-cem-css-context')).not.toBe(instances[2].getAttribute('data-cem-css-context'));
                for (const [index, instance] of instances.entries()) {
                    expect(getComputedStyle(instance).getPropertyValue('--asset')).toContain(index === 2 ? '/b.svg' : '/a.svg');
                    expect(getComputedStyle(instance).getPropertyValue('--order').trim()).toBe('last');
                    expect(instance.querySelectorAll('style')).toHaveLength(0);
                    expect(instance.querySelector('span')?.getAnimations()).toHaveLength(1);
                }
                const sharedStyle = declaration.querySelector('style');
                instances[0].remove();
                await waitFor(() => expect(instances[0].hasAttribute('data-cem-css-context')).toBe(false));
                expect(sharedStyle?.isConnected).toBe(true);
                expect(declaration.querySelectorAll('style')).toHaveLength(3);
                root.append(instances[0]);
                const stale = await prepare(instances[0], 'a');
                await install(instances[0], 'b');
                expect(stale.lease.commit(stale.outputs, stale.release)).toBe(false);
                expect(getComputedStyle(instances[0]).getPropertyValue('--asset')).toContain('/b.svg');
                a.lease.release(); // An old lease cannot release the replacement.
                expect(instances[0].hasAttribute('data-cem-css-context')).toBe(true);
                contextA.dispose();
                expect(instances[1].hasAttribute('data-cem-css-context')).toBe(false);
                expect(declaration.querySelectorAll('style')).toHaveLength(2);
                expect(getComputedStyle(instances[2]).getPropertyValue('--asset')).toContain('/b.svg');
                const late = await prepare(instances[2], 'b');
                processingScope.dispose();
                expect(late.lease.commit(late.outputs, late.release)).toBe(false);
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                for (const instance of instances) expect(instance.hasAttribute('data-cem-css-context')).toBe(false);
                await Promise.all(releases);
            } finally {
                processingScope.dispose();
                await Promise.all(releases);
                native.dispose(); root.replaceChildren();
            }
        }
    },
};

export const SharedMarkersAndLateCommits: Story = {
    render: () => '<section aria-label="Shared native CSS ownership"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing shared ownership fixture root');
        const native = nativeCssStoryHost();
        const scope = createCemDeclarationScope({ document });
        const releases: Promise<unknown>[] = [];
        try {
            const owners = [new DeclarationStyleOwnership(document, scope), new DeclarationStyleOwnership(document, scope)];
            for (const owner of owners) {
                const declaration = document.createElement('div'); root.append(declaration); owner.add(declaration, scope);
            }
            const instance = document.createElement('native-shared-owner');
            instance.setAttribute('scope', 'owned-shared');
            instance.innerHTML = '<template data-cem-island="instance"></template><span>Shared</span>';
            root.append(instance);
            const { artifact } = await native.compile('native-shared-owner', [
                { css: ':host { --first: url(./icon.svg); }', scope: 'owned-shared' },
                { css: ':host { --second: url(./icon.svg); }', scope: 'owned-shared' },
            ]);
            let sequence = 0;
            async function prepare(owner: DeclarationStyleOwnership, index: number, identity = 'shared') {
                const lease = owner.beginConsumer(instance, scope);
                const consumer = `shared-${++sequence}`;
                const output = await native.host.stylesheet({ action: 'begin', artifact, consumer, index,
                    baseUrl: 'https://example.test/main.css', context: { ...nativeCssStoryContext, identity, frames: [{ frameId: identity,
                        baseUrl: 'https://example.test/', scopes: [], specifiers: { imports: {}, resources: {} } }] },
                    scope: { kind: 'shared', name: 'owned-shared' } }).result;
                if (output.status !== 'ready') throw new Error('expected ready shared stylesheet');
                expect(output.diagnostics).toEqual([]);
                const release = () => { releases.push(native.host.stylesheet({ action: 'release', artifact, consumer, loadId: output.loadId }).result); };
                return { lease, outputs: [{ index, scope: { kind: 'shared' as const, name: 'owned-shared' }, output }], release };
            }
            const first = await prepare(owners[0], 0);
            const second = await prepare(owners[1], 1);
            expect(first.lease.commit(first.outputs, first.release)).toBe(true);
            expect(second.lease.commit(second.outputs, second.release)).toBe(true);
            const marker = instance.getAttribute('data-cem-css-context');
            expect(getComputedStyle(instance).getPropertyValue('--first')).toContain('/icon.svg');
            expect(getComputedStyle(instance).getPropertyValue('--second')).toContain('/icon.svg');
            first.lease.release();
            expect(instance.getAttribute('data-cem-css-context')).toBe(marker);
            expect(getComputedStyle(instance).getPropertyValue('--second')).toContain('/icon.svg');
            expect(root.querySelectorAll('style[data-cem-style-scope="owned-shared"]')).toHaveLength(1);
            const conflict = await prepare(owners[0], 0, 'different');
            expect(conflict.lease.commit(conflict.outputs, conflict.release)).toBe(false);
            expect(instance.getAttribute('data-cem-css-context')).toBe(marker);
            expect(root.querySelectorAll('style')).toHaveLength(1);
            const late = await prepare(owners[0], 0);
            instance.remove(); root.append(instance);
            // Flush pending removal records inside commit, before the observer callback.
            expect(late.lease.commit(late.outputs, late.release)).toBe(false);
            expect(instance.hasAttribute('data-cem-css-context')).toBe(false);
            expect(root.querySelectorAll('style')).toHaveLength(0);
            const reconnect = await prepare(owners[0], 0);
            expect(reconnect.lease.commit(reconnect.outputs, reconnect.release)).toBe(true);
            expect(instance.getAttribute('data-cem-css-context')).toBe(marker);
            scope.dispose();
            expect(instance.hasAttribute('data-cem-css-context')).toBe(false);
            expect(root.querySelectorAll('style')).toHaveLength(0);
            const results = await Promise.all(releases);
            expect(results).toHaveLength(5);
            for (const result of results) expect(result).toMatchObject({ status: 'released', count: 1 });
        } finally { scope.dispose(); await Promise.all(releases); native.dispose(); root.replaceChildren(); }
    },
};

export const PendingImportCancellation: Story = {
    render: () => '<section aria-label="Native pending CSS cancellation"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing pending ownership fixture root');
        for (const fallback of [false, true]) {
            const native = nativeCssStoryHost(fallback);
            const scope = createCemDeclarationScope({ document });
            const owner = new DeclarationStyleOwnership(document, scope);
            const declaration = document.createElement('div');
            const instance = document.createElement('native-pending-card');
            root.append(declaration, instance); owner.add(declaration, scope);
            const releases: Promise<unknown>[] = [];
            try {
                const { artifact } = await native.compile('native-pending-card', [{ css: '@import "./child.css";', scope: null }]);
                const begin = () => native.host.stylesheet({ action: 'begin', artifact, consumer: 'pending', index: 0,
                    baseUrl: 'https://example.test/main.css', scope: { kind: 'private', tag: 'native-pending-card' },
                    context: { ...nativeCssStoryContext, frames: [{ frameId: 'page', baseUrl: 'https://example.test/',
                        scopes: [], specifiers: { imports: {}, resources: {} } }] } }).result;
                const first = owner.beginConsumer(instance, scope);
                const pending = await begin();
                if (pending.status !== 'pending') throw new Error('expected pending import');
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                let cancelled = false;
                first.signal.addEventListener('abort', () => { cancelled = true; }, { once: true });
                const replacement = owner.beginConsumer(instance, scope);
                expect(cancelled).toBe(true);
                expect(first.signal.aborted).toBe(true);
                expect(replacement.signal.aborted).toBe(false);
                const next = await begin();
                if (next.status !== 'pending') throw new Error('expected replacement import');
                // Old network cleanup may arrive after a newer load has started.
                expect(await native.host.stylesheet({ action: 'release', artifact, consumer: 'pending', loadId: pending.loadId }).result)
                    .toMatchObject({ status: 'released', count: 0 });
                const output = await native.host.stylesheet({ action: 'deliver', artifact, consumer: 'pending',
                    loadId: next.loadId, requestId: next.request.id, bytes: new TextEncoder().encode(':host { --imported: yes; }').buffer,
                    finalUrl: 'https://example.test/cdn/child.css', contentType: 'text/css' }).result;
                if (output.status !== 'ready') throw new Error('expected completed import');
                expect(replacement.commit([{ index: 0, scope: { kind: 'private' }, output }], () => {
                    releases.push(native.host.stylesheet({ action: 'release', artifact, consumer: 'pending', loadId: output.loadId }).result);
                })).toBe(true);
                expect(getComputedStyle(instance).getPropertyValue('--imported').trim()).toBe('yes');
                const loading = owner.beginConsumer(instance, scope);
                const unfinished = await begin();
                if (unfinished.status !== 'pending') throw new Error('expected unfinished import');
                loading.signal.addEventListener('abort', () => {
                    releases.push(native.host.stylesheet({ action: 'release', artifact, consumer: 'pending', loadId: unfinished.loadId }).result);
                }, { once: true });
                scope.dispose();
                expect(replacement.signal.aborted).toBe(true);
                expect(loading.signal.aborted).toBe(true);
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                expect(await Promise.all(releases)).toEqual([
                    { status: 'released', count: 1 }, { status: 'released', count: 1 },
                ]);
            } finally { scope.dispose(); await Promise.all(releases); native.dispose(); root.replaceChildren(); }
        }
    },
};

export const ReentrantCancellation: Story = {
    render: () => '<section aria-label="Reentrant CSS cancellation"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing cancellation fixture root');
        const scope = createCemDeclarationScope({ document });
        const owner = new DeclarationStyleOwnership(document, scope);
        const instance = document.createElement('native-reentrant-card'); root.append(instance);
        try {
            const first = owner.beginConsumer(instance, scope);
            let nestedSignal: AbortSignal | undefined;
            first.signal.addEventListener('abort', () => {
                nestedSignal = owner.beginConsumer(instance, scope).signal;
            }, { once: true });
            const superseded = owner.beginConsumer(instance, scope);
            expect(first.signal.aborted).toBe(true);
            expect(superseded.signal.aborted).toBe(true);
            expect(nestedSignal?.aborted).toBe(false);
            scope.dispose();
            expect(nestedSignal?.aborted).toBe(true);
        } finally { scope.dispose(); root.replaceChildren(); }
    },
};

export const ServerDeclarationStyleReuse: Story = {
    render: () => '<section aria-label="Server declaration stylesheet reuse"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing SSR style fixture');
        for (const fallback of [false, true]) {
            const native = nativeCssStoryHost(fallback);
            const scope = createCemDeclarationScope({ document });
            const owner = new DeclarationStyleOwnership(document, scope);
            const declaration = document.createElement('div');
            const consumers = [document.createElement('native-server-card'), document.createElement('native-server-card')];
            consumers.forEach(element => {
                element.setAttribute('scope', 'server-library');
                element.innerHTML = '<template data-cem-island="instance"></template>';
            });
            root.append(declaration, ...consumers);
            try {
                const { artifact } = await native.compile('native-server-card', [
                    { css: ':host { --private: yes }', scope: null },
                    { css: ':host { --shared: yes }', scope: 'server-library' },
                ]);
                const outputs = await Promise.all([0, 1].map(async index => {
                    const styleScope = index === 0 ? { kind: 'private' as const, tag: 'native-server-card' }
                        : { kind: 'shared' as const, name: 'server-library' };
                    const output = await native.host.stylesheet({ action: 'begin', artifact, consumer: 'server', index,
                        scope: styleScope, baseUrl: 'https://example.test/main.css', context: nativeCssStoryContext }).result;
                    if (output.status !== 'ready') throw new Error('expected ready native output');
                    return { index, scope: styleScope, output };
                }));
                const serialized = serializeDeclarationStylesheets(outputs);
                declaration.innerHTML = serialized.html;
                const serverNodes = Array.from(declaration.querySelectorAll('style'));
                const serverText = serverNodes.map(node => node.firstChild);
                // Discard duplicate and corrupted candidates once native output is available.
                const duplicate = serverNodes[0].cloneNode(true) as HTMLStyleElement;
                const corrupt = serverNodes[1].cloneNode(true) as HTMLStyleElement;
                corrupt.textContent = ':root { --corrupt: yes }';
                declaration.prepend(corrupt); declaration.append(duplicate);
                owner.add(declaration, scope);
                const first = owner.beginConsumer(consumers[0], scope);
                expect(first.commit(outputs, () => undefined)).toBe(true);
                expect(Array.from(declaration.querySelectorAll('style'))).toEqual(serverNodes);
                serverNodes.forEach((node, index) => expect(declaration.querySelectorAll('style')[index]).toBe(node));
                serverNodes.forEach((node, index) => expect(node.firstChild).toBe(serverText[index]));
                expect(duplicate.isConnected).toBe(false); expect(corrupt.isConnected).toBe(false);
                expect(consumers[0].getAttribute('data-cem-css-context')).toBe(serialized.contextMarker);
                const second = owner.beginConsumer(consumers[1], scope);
                expect(second.commit(outputs, () => undefined)).toBe(true);
                first.release();
                expect(Array.from(declaration.querySelectorAll('style'))).toEqual(serverNodes);
                serverNodes.forEach((node, index) => expect(declaration.querySelectorAll('style')[index]).toBe(node));
                expect(getComputedStyle(consumers[1]).getPropertyValue('--private').trim()).toBe('yes');
                expect(getComputedStyle(consumers[1]).getPropertyValue('--shared').trim()).toBe('yes');
                second.release();
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                // A mismatched occurrence must not be reused, even with the correct cache key.
                owner.remove(declaration);
                declaration.innerHTML = serialized.html;
                const wrong = declaration.querySelector('style');
                if (!wrong) throw new Error('missing mismatch candidate');
                wrong.setAttribute('data-cem-style-index', '99');
                const pendingContext = wrong.cloneNode(true) as HTMLStyleElement;
                pendingContext.setAttribute('data-cem-style-key', 'another-context');
                declaration.append(pendingContext);
                const restricted = declaration.querySelectorAll('style')[1];
                restricted.setAttribute('media', 'not all');
                owner.add(declaration, scope);
                const third = owner.beginConsumer(consumers[0], scope);
                expect(third.commit(outputs, () => undefined)).toBe(true);
                expect(wrong.isConnected).toBe(false);
                expect(restricted.isConnected).toBe(false);
                expect(pendingContext.isConnected).toBe(true);
                expect(declaration.querySelectorAll('style')).toHaveLength(3);
                scope.dispose();
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                expect(native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { scope.dispose(); native.dispose(); root.replaceChildren(); }
        }
    },
};

export const StagedReplacementOwnership: Story = {
    render: () => '<section aria-label="Staged declaration CSS replacement"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing staged fixture');
        for (const fallback of [false, true]) {
            const native = nativeCssStoryHost(fallback);
            const scope = createCemDeclarationScope({ document });
            const owner = new DeclarationStyleOwnership(document, scope);
            const otherOwner = new DeclarationStyleOwnership(document, scope);
            const declaration = document.createElement('div');
            const otherDeclaration = document.createElement('div');
            const element = document.createElement('native-staged-card');
            const peer = document.createElement('native-staged-card');
            root.append(declaration, otherDeclaration, element, peer);
            owner.add(declaration, scope); otherOwner.add(otherDeclaration, scope);
            const releases: Promise<unknown>[] = [];
            try {
                const { artifact } = await native.compile('native-staged-card', [{ css: ':host {--asset:url(icon)}', scope: null }]);
                let sequence = 0;
                const prepare = async (variant: string) => {
                    const consumer = `staged-${variant}-${++sequence}`;
                    const output = await native.host.stylesheet({ action: 'begin', artifact, consumer, index: 0,
                        scope: { kind: 'private', tag: 'native-staged-card' }, baseUrl: 'https://example.test/',
                        context: { ...nativeCssStoryContext, frames: [{ frameId: variant, baseUrl: 'https://example.test/', scopes: [],
                            specifiers: { imports: {}, resources: { icon: { target: `./${variant}.svg` } } } }] },
                    }).result;
                    if (output.status !== 'ready') throw new Error('expected native output');
                    return { outputs: [{ index: 0, scope: { kind: 'private' as const }, output }], release: () => {
                        releases.push(native.host.stylesheet({ action: 'release', artifact, consumer, loadId: output.loadId }).result);
                    } };
                };
                const a = await prepare('a');
                const b = await prepare('b');
                const first = owner.beginConsumer(element, scope);
                expect(first.commit(a.outputs, a.release)).toBe(true);
                const shared = owner.beginConsumer(peer, scope);
                const peerOutput = await prepare('a');
                expect(shared.commit(peerOutput.outputs, peerOutput.release)).toBe(true);
                const original = declaration.querySelector('style');
                const marker = element.getAttribute('data-cem-css-context');
                const cancelled = owner.stageConsumer(element, scope);
                expect(original?.isConnected).toBe(true);
                expect(first.signal.aborted).toBe(false);
                cancelled.release();
                expect(original?.isConnected).toBe(true);
                expect(element.getAttribute('data-cem-css-context')).toBe(marker);
                const stale = owner.stageConsumer(element, scope);
                const current = owner.stageConsumer(element, scope);
                expect(stale.signal.aborted).toBe(true);
                expect(stale.commit(b.outputs, () => undefined)).toBe(false);
                expect(first.signal.aborted).toBe(false);
                expect(current.commit([...b.outputs, ...b.outputs], () => undefined)).toBe(false);
                expect(first.signal.aborted).toBe(false);
                const blocking = otherOwner.beginConsumer(element, scope);
                const blockingOutput = await prepare('a');
                expect(blocking.commit(blockingOutput.outputs, blockingOutput.release)).toBe(true);
                const conflict = owner.stageConsumer(element, scope);
                expect(conflict.commit(b.outputs, () => undefined)).toBe(false);
                expect(element.getAttribute('data-cem-css-context')).toBe(marker);
                expect(original?.isConnected).toBe(true);
                blocking.release();
                const replacement = owner.stageConsumer(element, scope);
                expect(replacement.commit(b.outputs, b.release)).toBe(true);
                expect(first.signal.aborted).toBe(true);
                expect(getComputedStyle(element).getPropertyValue('--asset')).toContain('/b.svg');
                expect(getComputedStyle(peer).getPropertyValue('--asset')).toContain('/a.svg');
                expect(original?.isConnected).toBe(true);
                shared.release();
                expect(original?.isConnected).toBe(false);
                const retained = declaration.querySelector('style');
                const same = owner.stageConsumer(element, scope);
                const sameOutput = await prepare('b');
                expect(same.commit(sameOutput.outputs, sameOutput.release)).toBe(true);
                expect(declaration.querySelector('style')).toBe(retained);
                let nested: ReturnType<DeclarationStyleOwnership['stageConsumer']> | undefined;
                same.signal.addEventListener('abort', () => { nested = owner.stageConsumer(element, scope); }, { once: true });
                const reentrant = owner.stageConsumer(element, scope);
                const c = await prepare('c');
                expect(reentrant.commit(c.outputs, c.release)).toBe(true);
                expect(nested?.signal.aborted).toBe(false);
                nested?.release();
                expect(reentrant.signal.aborted).toBe(false);
                const pending = owner.stageConsumer(element, scope);
                element.remove(); root.append(element);
                expect(pending.commit(b.outputs, () => undefined)).toBe(false);
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                expect(element.hasAttribute('data-cem-css-context')).toBe(false);
                const resumed = owner.beginConsumer(element, scope);
                const resumedOutput = await prepare('c');
                expect(resumed.commit(resumedOutput.outputs, resumedOutput.release)).toBe(true);
                const abandoned = owner.stageConsumer(element, scope);
                const immediate = owner.beginConsumer(element, scope);
                expect(abandoned.signal.aborted).toBe(true);
                expect(resumed.signal.aborted).toBe(true);
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                const d = await prepare('d');
                expect(immediate.commit(d.outputs, d.release)).toBe(true);
                const disposed = owner.stageConsumer(element, scope);
                scope.dispose();
                expect(immediate.signal.aborted).toBe(true);
                expect(disposed.signal.aborted).toBe(true);
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                expect(native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { scope.dispose(); await Promise.all(releases); native.dispose(); root.replaceChildren(); }
        }
    },
};


export const CoordinatedContextReplacement: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing grouped ownership fixture');
        for (const fallback of [false, true]) {
            const native = nativeCssStoryHost(fallback);
            const scope = createCemDeclarationScope({ document });
            const owners = [0, 1].map(() => new DeclarationStyleOwnership(document, scope));
            const declarations = owners.map(owner => {
                const element = document.createElement('div'); root.append(element); owner.add(element, scope); return element;
            });
            const tag = `native-group-${crypto.randomUUID()}`;
            let observeMarker: (() => void) | undefined;
            customElements.define(tag, class extends HTMLElement {
                static observedAttributes = ['data-cem-css-context'];
                attributeChangedCallback() { observeMarker?.(); }
            });
            const instance = document.createElement(tag); root.append(instance);
            const releases: Promise<unknown>[] = [];
            let sequence = 0;
            try {
                const artifacts = await Promise.all([0, 1].map(index => native.compile(`native-group-source-${index}`, [
                    { css: `:host { --asset-${index}: url(asset); }`, scope: null },
                ])));
                async function prepare(index: number, variant: string): Promise<DeclarationStylesheetCommit> {
                    const lease = owners[index].stageConsumer(instance, scope);
                    const consumer = `group-${++sequence}`;
                    const artifact = artifacts[index].artifact;
                    const context = { ...nativeCssStoryContext, frames: [{ frameId: 'page',
                        baseUrl: 'https://example.test/', scopes: [],
                        specifiers: { imports: {}, resources: { asset: { target: `./${variant}.svg` } } } }] };
                    const output = await native.host.stylesheet({ action: 'begin', artifact, consumer, index: 0,
                        baseUrl: 'https://example.test/main.css', context,
                        scope: { kind: 'private', tag } }).result;
                    if (output.status !== 'ready') throw new Error('expected ready group output');
                    return { lease, outputs: [{ index: 0, scope: { kind: 'private' }, output }],
                        release: () => { releases.push(native.host.stylesheet({ action: 'release', artifact, consumer, loadId: output.loadId }).result); } };
                }
                const assertAssets = (variant: string) => {
                    for (const index of [0, 1]) expect(getComputedStyle(instance).getPropertyValue(`--asset-${index}`)).toContain(`/${variant}.svg`);
                };
                const initial = await Promise.all([prepare(0, 'old'), prepare(1, 'old')]);
                expect(DeclarationStyleOwnership.commitGroup(initial)).toBe(true);
                assertAssets('old');
                const oldNodes = declarations.map(element => element.querySelector('style'));
                const oldMarker = instance.getAttribute('data-cem-css-context');
                // Omitting another owner that still holds the old marker must fail closed.
                const omitted = await prepare(0, 'new');
                expect(DeclarationStyleOwnership.commitGroup([omitted])).toBe(false);
                assertAssets('old');
                expect(instance.getAttribute('data-cem-css-context')).toBe(oldMarker);
                const conflicting = await Promise.all([prepare(0, 'new'), prepare(1, 'other')]);
                expect(DeclarationStyleOwnership.commitGroup(conflicting)).toBe(false);
                assertAssets('old');
                const invalid = await Promise.all([prepare(0, 'new'), prepare(1, 'new')]);
                invalid[1].outputs = [...invalid[1].outputs, ...invalid[1].outputs];
                expect(DeclarationStyleOwnership.commitGroup(invalid)).toBe(false);
                expect(declarations.map(element => element.querySelector('style'))).toEqual(oldNodes);
                const stale = await Promise.all([prepare(0, 'new'), prepare(1, 'new')]);
                const newer = owners[1].stageConsumer(instance, scope);
                expect(DeclarationStyleOwnership.commitGroup(stale)).toBe(false);
                newer.release(); assertAssets('old');
                const next = await Promise.all([prepare(0, 'new'), prepare(1, 'new')]);
                let observedComplete = false;
                let markerObservedComplete = false;
                observeMarker = () => {
                    assertAssets('new');
                    const reentrant = owners[0].stageConsumer(instance, scope);
                    reentrant.release();
                    markerObservedComplete = true;
                };
                initial[0].lease.signal.addEventListener('abort', () => {
                    assertAssets('new');
                    expect(instance.getAttribute('data-cem-css-context')).not.toBe(oldMarker);
                    const reentrant = owners[1].stageConsumer(instance, scope);
                    reentrant.release();
                    observedComplete = true;
                }, { once: true });
                expect(DeclarationStyleOwnership.commitGroup(next)).toBe(true);
                expect(observedComplete).toBe(true);
                expect(markerObservedComplete).toBe(true);
                assertAssets('new');
                const releaseCount = releases.length;
                expect(DeclarationStyleOwnership.commitGroup(next)).toBe(false);
                expect(releases).toHaveLength(releaseCount);
                initial.forEach(entry => entry.lease.release());
                assertAssets('new');
                const same = await Promise.all([prepare(0, 'new'), prepare(1, 'new')]);
                const currentNodes = declarations.map(element => element.querySelector('style'));
                expect(DeclarationStyleOwnership.commitGroup(same)).toBe(true);
                expect(declarations.map(element => element.querySelector('style'))).toEqual(currentNodes);
                // Clearing one owner preserves the other owner's context marker.
                const clear = { lease: owners[0].stageConsumer(instance, scope), outputs: [], release: () => undefined };
                expect(DeclarationStyleOwnership.commitGroup([clear])).toBe(true);
                expect(instance.hasAttribute('data-cem-css-context')).toBe(true);
                expect(declarations[0].querySelectorAll('style')).toHaveLength(0);
                const disposed = await Promise.all([prepare(0, 'later'), prepare(1, 'later')]);
                observeMarker = undefined;
                scope.dispose();
                expect(DeclarationStyleOwnership.commitGroup(disposed)).toBe(false);
                expect(instance.hasAttribute('data-cem-css-context')).toBe(false);
                expect(declarations[1].querySelectorAll('style')).toHaveLength(0);
                expect(native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally {
                observeMarker = undefined;
                scope.dispose(); await Promise.all(releases); native.dispose(); root.replaceChildren();
            }
        }
    },
};
