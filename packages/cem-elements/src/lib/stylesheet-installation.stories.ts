import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { DeclarationStyleOwnership } from './declaration-style-ownership.js';
import { installRetainedStylesheets, type CemStylesheetResponse } from './internal/runtime-support/stylesheet-installation.js';
import { nativeCssStoryHost, nativeCssStoryContext } from './native-css-story-fixture.js';

export default { title: 'CEM Elements/Retained Stylesheet Installation', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

function fixture(root: HTMLElement, fallback: boolean) {
    const native = nativeCssStoryHost(fallback);
    const scope = createCemDeclarationScope({ document });
    const owner = new DeclarationStyleOwnership(document, scope);
    const declaration = document.createElement('div');
    const instance = document.createElement('native-install-card');
    const content = document.createElement('p'); content.textContent = 'Retained server content';
    instance.append(content); root.append(declaration, instance); owner.add(declaration, scope);
    const context = { ...nativeCssStoryContext, frames: [{ frameId: 'page', baseUrl: 'https://example.test/',
        scopes: [], specifiers: { imports: {}, resources: { asset: { target: './original.svg' } } } }] };
    return { native, scope, owner, declaration, instance, content, context,
        dispose() { scope.dispose(); native.dispose(); root.replaceChildren(); } };
}
function response(css: string, contentType = 'text/css'): CemStylesheetResponse {
    return { bytes: new TextEncoder().encode(css).buffer, finalUrl: 'https://example.test/cdn/child.css', contentType };
}
function held<T>() {
    let resolve: (value: T) => void = () => { throw new Error('missing held resolver'); };
    const promise = new Promise<T>(done => { resolve = done; });
    return { promise, resolve };
}

export const ReadinessAndContextSnapshot: Story = {
    render: () => '<section aria-label="Native CSS readiness"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing installation fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const { artifact } = await f.native.compile('native-install-card', [
                    { css: '@import "./child.css"; :host { --root: yes; }', scope: null },
                    { css: ':host { --independent: yes; }', scope: null },
                ]);
                const gate = held<CemStylesheetResponse>();
                let requested = false; let settled = false;
                const options = { host: f.native.host, artifact, consumer: 'card',
                    lease: f.owner.beginConsumer(f.instance, f.scope), context: f.context,
                    baseUrl: 'https://example.test/main.css', occurrences: [0, 1].map(index =>
                        ({ index, scope: { kind: 'private' as const, tag: 'native-install-card' } })),
                    read: async (request: { url: string }) => {
                        expect(request.url).toBe('https://example.test/child.css');
                        requested = true; return gate.promise;
                    } };
                const load = installRetainedStylesheets(options);
                void load.ready.then(() => { settled = true; });
                await waitFor(() => expect(requested).toBe(true));
                expect(settled).toBe(false);
                expect(f.declaration.querySelectorAll('style')).toHaveLength(0);
                f.context.frames[0].specifiers.resources.asset.target = './changed.svg';
                gate.resolve(response(':host { --asset: url(asset); --relative: url(./icon.svg); }'));
                expect(await load.ready).toMatchObject({ status: 'ready', installed: 2 });
                expect(f.native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
                expect(f.instance.querySelector('p')).toBe(f.content);
                expect(getComputedStyle(f.instance).getPropertyValue('--asset')).toContain('/original.svg');
                expect(getComputedStyle(f.instance).getPropertyValue('--relative')).toContain('/cdn/icon.svg');
                expect(getComputedStyle(f.instance).getPropertyValue('--independent').trim()).toBe('yes');
                expect(f.declaration.querySelectorAll('style')).toHaveLength(2);
                await load.dispose();
                expect(f.declaration.querySelectorAll('style')).toHaveLength(0);
                expect(f.instance.hasAttribute('data-cem-css-context')).toBe(false);
            } finally { f.dispose(); }
        }
    },
};

export const CancellationAndReconnect: Story = {
    render: () => '<section aria-label="Native CSS cancellation and reconnect"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing cancellation fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const { artifact } = await f.native.compile('native-install-card', [{ css: '@import "./child.css";', scope: null }]);
                const options = { host: f.native.host, artifact, consumer: 'same-consumer', context: f.context,
                    baseUrl: 'https://example.test/main.css', occurrences: [{ index: 0,
                        scope: { kind: 'private' as const, tag: 'native-install-card' } }] };
                const gate = held<CemStylesheetResponse>();
                let readerSignal: AbortSignal | undefined;
                const old = installRetainedStylesheets({ ...options, lease: f.owner.beginConsumer(f.instance, f.scope),
                    read: async (_request, signal) => { readerSignal = signal; return gate.promise; } });
                await waitFor(() => expect(readerSignal).toBeDefined());
                f.instance.remove();
                expect(await old.ready).toMatchObject({ status: 'cancelled', installed: 0 });
                expect(readerSignal?.aborted).toBe(true);
                root.append(f.instance);
                const current = installRetainedStylesheets({ ...options, lease: f.owner.beginConsumer(f.instance, f.scope),
                    read: async () => response(':host { --current: yes; }') });
                expect(await current.ready).toMatchObject({ status: 'ready', installed: 1 });
                gate.resolve(response(':host { --stale: yes; }'));
                await old.dispose();
                expect(getComputedStyle(f.instance).getPropertyValue('--current').trim()).toBe('yes');
                expect(getComputedStyle(f.instance).getPropertyValue('--stale').trim()).toBe('');
                f.scope.dispose();
                await current.dispose();
                expect(f.declaration.querySelectorAll('style')).toHaveLength(0);
            } finally { f.dispose(); }
        }
    },
};

export const FailuresKeepIndependentStyles: Story = {
    render: () => '<section aria-label="Native CSS import failures"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing failure fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const { artifact } = await f.native.compile('native-install-card', [
                    { css: '@import "./child.css"; :host { --failed: no; }', scope: null },
                    { css: ':host { --independent: yes; }', scope: null },
                ]);
                const load = installRetainedStylesheets({ host: f.native.host, artifact, consumer: 'failure',
                    lease: f.owner.beginConsumer(f.instance, f.scope), context: f.context,
                    baseUrl: 'https://example.test/main.css', occurrences: [0, 1].map(index =>
                        ({ index, scope: { kind: 'private' as const, tag: 'native-install-card' } })),
                    read: async () => response('<html/>', 'text/html') });
                const result = await load.ready;
                expect(result).toMatchObject({ status: 'ready', installed: 1 });
                expect(result.diagnostics).toEqual(expect.arrayContaining([expect.objectContaining({
                    code: 'cem.css.import_content_type', stylesheetUrl: 'https://example.test/main.css',
                    sourceUri: expect.stringMatching(/^urn:cem:template-style:/), offset: 0,
                })]));
                expect(getComputedStyle(f.instance).getPropertyValue('--failed').trim()).toBe('');
                expect(getComputedStyle(f.instance).getPropertyValue('--independent').trim()).toBe('yes');
                await load.dispose();
            } finally { f.dispose(); }
        }
    },
};
