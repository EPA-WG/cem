import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { DeclarationStyleOwnership } from './declaration-style-ownership.js';
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
