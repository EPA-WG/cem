import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { nativeCssStoryHost, nativeCssStoryContext } from './native-css-story-fixture.js';

export default { title: 'CEM Elements/Native Instance Stylesheets', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

export const WorkerAndFallbackOwners: Story = {
    render: () => '<section aria-label="Instance stylesheet ownership"></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const native = nativeCssStoryHost(fallback);
            const animations: string[] = [];
            try {
                const sources = [{ css: '@import "child.css"; @keyframes pulse {from {opacity:0.4} to {opacity:0.8}} .card {animation: pulse 20s infinite}', scope: null }];
                for (const identity of ['instance-one', 'instance-two']) {
                    const { artifact } = await native.compile(identity, sources, identity);
                    const begin = { action: 'begin' as const, artifact, consumer: 'lease', index: 0,
                        baseUrl: 'https://example.test/payload.css', context: { ...nativeCssStoryContext,
                            frames: [{ frameId: 'page', baseUrl: 'https://example.test/page.html', scopes: [], specifiers: { imports: {}, resources: {} } }] },
                        scope: { kind: 'instance' as const } };
                    await expect(native.host.stylesheet({ ...begin, scope: { kind: 'private', tag: 'cem-card' } }).result).rejects.toThrow('ownership');
                    const load = async () => {
                        const pending = await native.host.stylesheet(begin).result;
                        if (pending.status !== 'pending') throw new Error('expected pending instance import');
                        expect(pending.request.url).toBe('https://example.test/child.css');
                        const ready = await native.host.stylesheet({ action: 'deliver', artifact, consumer: 'lease',
                            loadId: pending.loadId, requestId: pending.request.id,
                            bytes: new TextEncoder().encode('.card {color:rgb(1, 2, 3); --asset:url(icon.svg)}').buffer,
                            finalUrl: 'https://example.test/cdn/child.css', contentType: 'text/css' }).result;
                        if (ready.status !== 'ready') throw new Error('expected ready instance stylesheet');
                        expect(ready.diagnostics).toEqual([]);
                        return ready;
                    };
                    const first = await load();
                    const host = document.createElement('div');
                    const style = document.createElement('style'); style.textContent = first.css;
                    const card = document.createElement('p'); card.className = 'card'; card.textContent = identity;
                    host.append(style, card); canvasElement.append(host);
                    expect(getComputedStyle(card).color).toBe('rgb(1, 2, 3)');
                    expect(getComputedStyle(card).getPropertyValue('--asset')).toContain('https://example.test/cdn/icon.svg');
                    expect(card.getAnimations()).toHaveLength(1);
                    animations.push((card.getAnimations()[0] as CSSAnimation).animationName);
                    expect(host.hasAttribute('data-cem-css-context')).toBe(false);
                    await native.host.stylesheet({ action: 'release', artifact, consumer: 'lease', loadId: first.loadId }).result;
                    const resumed = await load();
                    expect(resumed.identity).toEqual(first.identity);
                    expect(resumed.css).toBe(first.css);
                    const pending = await native.host.stylesheet(begin).result;
                    if (pending.status !== 'pending') throw new Error('expected cancellable import');
                    await native.host.stylesheet({ action: 'release', artifact, consumer: 'lease', loadId: pending.loadId }).result;
                    await expect(native.host.stylesheet({ action: 'fail', artifact, consumer: 'lease', loadId: pending.loadId,
                        requestId: pending.request.id, message: 'late failure' }).result).rejects.toThrow();
                }
                expect(new Set(animations).size).toBe(2);
                expect(native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { native.dispose(); canvasElement.replaceChildren(); }
        }
    },
};
