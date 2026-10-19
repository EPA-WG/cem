import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { installInstanceStylesheets, stageInstanceStylesheets, prepareInstanceStylesheets, type InstanceStylesheetInstallationOptions } from './internal/runtime-support/instance-stylesheet-installation.js';
import type { CemStylesheetResponse } from './internal/runtime-support/stylesheet-installation.js';
import { nativeCssStoryHost, nativeCssStoryContext } from './native-css-story-fixture.js';

export default { title: 'CEM Elements/Instance Stylesheet Staging', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function held<T>() {
    let resolve!: (value: T) => void;
    const promise = new Promise<T>(done => { resolve = done; });
    return { promise, resolve };
}
function response(css: string): CemStylesheetResponse {
    return { bytes: new TextEncoder().encode(css).buffer, finalUrl: 'https://example.test/child.css', contentType: 'text/css' };
}
function stage(options: InstanceStylesheetInstallationOptions) {
    return stageInstanceStylesheets({ ...options, artifactId: `instance-stage:${crypto.randomUUID()}` });
}
function fixture(root: HTMLElement, fallback: boolean) {
    const native = nativeCssStoryHost(fallback);
    const element = document.createElement('native-instance-card');
    const style = document.createElement('style');
    style.setAttribute('data-cem-instance-style', '0');
    style.textContent = 'native-instance-card { --value: server; }';
    const content = document.createElement('p'); content.textContent = 'Retained content';
    element.append(style, content); root.append(element);
    const controller = new AbortController();
    const options: InstanceStylesheetInstallationOptions = {
        host: native.host, context: { ...nativeCssStoryContext, frames: [{ frameId: 'page', baseUrl: 'https://example.test/',
            scopes: [], specifiers: { imports: {}, resources: {} } }] }, element, instanceId: 'staged-card',
        sources: [{ css: ':host { --value: client; }', scope: null }],
        artifactId: 'staged-instance', scopePolicyStamp: 'native-story',
        signal: controller.signal, baseUrl: 'https://example.test/main.css',
        read: async () => response(':host { --value: imported; }'),
    };
    return { native, element, style, content, controller, options,
        value: () => getComputedStyle(element).getPropertyValue('--value').trim(),
        dispose() { controller.abort(); native.dispose(); root.replaceChildren(); } };
}

export const ServerAndCommittedNodesSurviveStaging: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing instance fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const invalidServer = stage({ ...f.options, sources: [{ css: '}', scope: null }] });
                expect((await invalidServer.ready).some(item => item.severity === 'error' || item.severity === 'fatal')).toBe(true);
                expect(f.element.querySelector('style')).toBe(f.style);
                expect(f.value()).toBe('server');
                invalidServer.dispose();
                const gate = held<CemStylesheetResponse>();
                let requested = false;
                const first = stage({ ...f.options,
                    sources: [{ css: '@import "./child.css";', scope: null }],
                    read: async () => { requested = true; return gate.promise; } });
                expect(f.value()).toBe('server');
                await waitFor(() => expect(requested).toBe(true), { timeout: 10000 });
                expect(f.style.isConnected).toBe(true);
                expect(f.value()).toBe('server');
                gate.resolve(response(':host { --value: first; }'));
                expect(await first.ready).toEqual([]);
                expect(f.value()).toBe('first');
                expect(f.element.querySelector('style')).toBe(f.style);
                const text = f.style.firstChild;
                const equal = stage({ ...f.options,
                    sources: [{ css: '@import "./child.css";', scope: null }],
                    read: async () => response(':host { --value: first; }') });
                await equal.ready;
                first.dispose();
                expect(f.element.querySelector('style')).toBe(f.style);
                expect(f.style.firstChild).toBe(text);
                expect(f.value()).toBe('first');
                const failed = stage({ ...f.options,
                    sources: [{ css: ':host { --value: partial; }', scope: null }, { css: '@import "./missing.css";', scope: null }],
                    read: async () => { throw new Error('missing import'); } });
                expect((await failed.ready).some(item => item.severity === 'error' || item.severity === 'fatal')).toBe(true);
                expect(f.value()).toBe('first');
                expect(f.element.querySelectorAll('style')).toHaveLength(1);
                const cleared = stage({ ...f.options, sources: [] });
                expect(await cleared.ready).toEqual([]);
                equal.dispose(); failed.dispose();
                expect(f.element.querySelectorAll('style')).toHaveLength(0);
                expect(f.element.querySelector('p')).toBe(f.content);
                expect(f.native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
                cleared.dispose();
            } finally { f.dispose(); }
        }
    },
};

export const CancellationSupersessionAndCompilationFailure: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing instance fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const active = installInstanceStylesheets(f.options);
                await active.ready;
                expect(f.value()).toBe('client');
                const pending = () => {
                    const gate = held<CemStylesheetResponse>();
                    let signal: AbortSignal | undefined;
                    const load = stage({ ...f.options,
                        sources: [{ css: '@import "./child.css";', scope: null }],
                        read: async (_request, requestSignal) => { signal = requestSignal; return gate.promise; } });
                    return { gate, load, signal: () => signal };
                };
                const cancelled = pending();
                await waitFor(() => expect(cancelled.signal()).toBeDefined(), { timeout: 10000 });
                cancelled.load.dispose();
                await cancelled.load.ready;
                expect(cancelled.signal()?.aborted).toBe(true);
                expect(f.value()).toBe('client');
                const stale = pending();
                await waitFor(() => expect(stale.signal()).toBeDefined(), { timeout: 10000 });
                const replacement = stage({ ...f.options,
                    sources: [{ css: ':host { --value: replacement; }', scope: null }] });
                await replacement.ready;
                await stale.load.ready;
                expect(stale.signal()?.aborted).toBe(true);
                cancelled.gate.resolve(response(':host { --value: cancelled; }'));
                stale.gate.resolve(response(':host { --value: stale; }'));
                active.dispose();
                expect(f.value()).toBe('replacement');
                const invalid = stage({ ...f.options,
                    sources: [{ css: ':host { --value: partial; }', scope: null }, { css: '}', scope: null }] });
                expect((await invalid.ready).some(item => item.severity === 'error' || item.severity === 'fatal')).toBe(true);
                expect(f.value()).toBe('replacement');
                const disconnected = pending();
                await waitFor(() => expect(disconnected.signal()).toBeDefined(), { timeout: 10000 });
                f.element.remove();
                disconnected.gate.resolve(response(':host { --value: disconnected; }'));
                await disconnected.load.ready;
                root.append(f.element);
                expect(f.value()).toBe('replacement');
                disconnected.load.dispose(); invalid.dispose();
                f.controller.abort();
                expect(f.element.querySelectorAll('style')).toHaveLength(0);
            } finally { f.dispose(); }
        }
    },
};

export const CompilationCancellationAndInputSnapshot: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing instance fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const gate = held<void>();
                let compileSubmitted = false; let cancelled = false; let read = false;
                const host = new Proxy(f.native.host, { get(target, key) {
                    if (key === 'compile') return (input: Parameters<typeof target.compile>[0]) => {
                        const job = target.compile(input); compileSubmitted = true;
                        return { ...job, result: job.result.then(async result => { await gate.promise; return result; }) };
                    };
                    if (key === 'cancel') return (input: Parameters<typeof target.cancel>[0]) => {
                        cancelled = true; return target.cancel(input);
                    };
                    const value = Reflect.get(target, key);
                    return typeof value === 'function' ? value.bind(target) : value;
                } });
                const abort = new AbortController();
                const load = stage({ ...f.options, host, signal: abort.signal,
                    sources: [{ css: '@import "./child.css";', scope: null }],
                    read: async () => { read = true; return response(':host { --value: late; }'); } });
                expect(compileSubmitted).toBe(true);
                abort.abort();
                expect(await load.ready).toEqual([]);
                expect(cancelled).toBe(true);
                expect(f.value()).toBe('server');
                gate.resolve();
                const options = { ...f.options,
                    sources: [{ css: ':host { --asset: url(asset); --value: captured; }', scope: null }],
                    context: { ...nativeCssStoryContext, frames: [{ frameId: 'page', baseUrl: 'https://example.test/',
                        scopes: [], specifiers: { imports: {}, resources: { asset: { target: './original.svg' } } } }] } };
                const snapshot = stage(options);
                options.sources[0].css = ':host { --value: mutated; }';
                options.context.frames[0].specifiers.resources.asset.target = './mutated.svg';
                expect(await snapshot.ready).toEqual([]);
                expect(f.value()).toBe('captured');
                expect(getComputedStyle(f.element).getPropertyValue('--asset')).toContain('/original.svg');
                expect(read).toBe(false);
                load.dispose(); snapshot.dispose();
            } finally { f.dispose(); }
        }
    },
};


export const ExplicitPublication: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing instance fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            const prepare = (sources = f.options.sources) => prepareInstanceStylesheets({ ...f.options,
                artifactId: crypto.randomUUID(), sources });
            try {
                const first = prepare([{ css: '@import "./child.css";', scope: null }]);
                expect(first.commit()).toBe(false);
                const result = await first.ready;
                expect(result.status).toBe('prepared');
                expect(result.diagnostics.some(item => item.severity === 'error' || item.severity === 'fatal')).toBe(false);
                expect(f.native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
                expect(f.value()).toBe('server');
                expect(f.element.querySelector('style')).toBe(f.style);
                expect(first.commit()).toBe(true);
                expect(first.commit()).toBe(false);
                expect(f.value()).toBe('imported');
                const stale = prepare();
                await stale.ready;
                expect(f.value()).toBe('imported');
                const current = prepare([{ css: ':host { --value: current; }', scope: null }]);
                await current.ready;
                expect(stale.commit()).toBe(false);
                expect(current.commit()).toBe(true);
                first.dispose(); stale.dispose();
                expect(f.value()).toBe('current');
                const cancelled = prepare();
                await cancelled.ready;
                cancelled.dispose();
                expect(cancelled.commit()).toBe(false);
                expect(f.value()).toBe('current');
                const invalid = prepare([{ css: '}', scope: null }]);
                expect(await invalid.ready).toMatchObject({ status: 'cancelled' });
                expect(invalid.commit()).toBe(false);
                invalid.dispose();
                expect(f.value()).toBe('current');
                const empty = prepare([]);
                expect(empty.commit()).toBe(false);
                expect(await empty.ready).toEqual({ status: 'prepared', diagnostics: [] });
                expect(f.value()).toBe('current');
                expect(empty.commit()).toBe(true);
                expect(f.element.querySelectorAll('style')).toHaveLength(0);
                current.dispose(); empty.dispose();
                expect(f.element.querySelector('p')).toBe(f.content);
            } finally { f.dispose(); }
        }
    },
};

export const UnchangedCssomAndFreshImports: Story = {
    render: () => '<section aria-label="Unchanged instance stylesheet installation"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                let reads = 0;
                let imported = ':host { --value: var(--fixture-reuse-theme, first); animation: steady 60s linear infinite; } @keyframes steady { from { opacity: .9; } to { opacity: 1; } }';
                const options = { ...f.options, sources: [{ css: '@import "./child.css";', scope: null }],
                    read: async () => { reads++; return response(imported); } };
                let active = stageInstanceStylesheets(options);
                expect(await active.ready).toEqual(fallback ? [expect.objectContaining({
                    code: 'cem.processing_host.worker_startup_fallback', severity: 'warning' })] : []);
                expect(f.value()).toBe('first');
                const animation = f.element.getAnimations()[0];
                expect(animation !== undefined).toBe(true);
                const style = f.element.querySelector('style')!;
                const sheet = style.sheet;
                const text = style.firstChild;
                const observer = new MutationObserver(() => undefined);
                observer.observe(f.element, { childList: true, subtree: true, attributes: true, characterData: true });
                try {
                    for (let i = 0; i < 4; i++) {
                        const next = prepareInstanceStylesheets(options);
                        expect(await next.ready).toMatchObject({ status: 'prepared' });
                        expect(observer.takeRecords()).toHaveLength(0);
                        expect(next.commit()).toBe(true);
                        active.dispose();
                        active = { ready: Promise.resolve([]), dispose: next.dispose };
                        expect(observer.takeRecords()).toHaveLength(0);
                        expect(style.sheet === sheet).toBe(true);
                        expect(f.element.getAnimations()[0] === animation).toBe(true);
                        expect(style.firstChild === text).toBe(true);
                    }
                    expect(reads).toBe(5); // Dependency freshness is checked on every attempt.
                    root.style.setProperty('--fixture-reuse-theme', 'theme');
                    expect(f.value()).toBe('theme');
                    const themed = prepareInstanceStylesheets(options);
                    await themed.ready; expect(themed.commit()).toBe(true);
                    active.dispose(); active = { ready: Promise.resolve([]), dispose: themed.dispose };
                    expect(observer.takeRecords()).toHaveLength(0);
                    expect(style.sheet === sheet).toBe(true);
                    expect(f.element.getAnimations()[0] === animation).toBe(true);
                    const cancelled = prepareInstanceStylesheets(options);
                    await cancelled.ready; cancelled.cancelPreparation();
                    expect(observer.takeRecords()).toHaveLength(0);
                    expect(style.sheet === sheet).toBe(true);
                    imported = ':host { --value: changed; }';
                    const changed = prepareInstanceStylesheets(options);
                    await changed.ready;
                    expect(changed.commit()).toBe(true);
                    active.dispose();
                    expect(f.value()).toBe('changed');
                    expect(style.sheet === sheet).toBe(false);
                    expect(observer.takeRecords().length).toBeGreaterThan(0);
                    changed.dispose();
                    expect(f.element.querySelector('style')).toBeNull();
                } finally { observer.disconnect(); active.dispose(); }
                expect(f.native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { root.style.removeProperty('--fixture-reuse-theme'); f.dispose(); }
        }
    },
};

export const NativeIdentitySiblingsAndDomRepair: Story = {
    render: () => '<section aria-label="Instance stylesheet reuse admission"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                await f.native.compile('reuse-admission-warmup', []);
                let active = stageInstanceStylesheets(f.options);
                expect(await active.ready).toEqual([]);
                const first = f.element.querySelector('style')!;
                let sheet = first.sheet;
                const replace = async (options: InstanceStylesheetInstallationOptions) => {
                    const next = prepareInstanceStylesheets({ ...options, artifactId: `reuse:${crypto.randomUUID()}` });
                    expect(await next.ready).toMatchObject({ status: 'prepared' });
                    expect(next.commit()).toBe(true);
                    active.dispose();
                    active = { ready: Promise.resolve([]), dispose: next.dispose };
                };
                const extra = { ...f.options, sources: [...f.options.sources, { css: ':host { --value: second; }', scope: null }] };
                await replace(extra);
                expect(first.sheet === sheet).toBe(true);
                expect(f.element.querySelectorAll('style')).toHaveLength(2);
                expect(f.value()).toBe('second');
                const second = f.element.querySelectorAll('style')[1];
                f.element.insertBefore(second, first);
                await replace(extra);
                expect(f.element.querySelector('style') === first).toBe(true);
                expect(first.sheet === sheet).toBe(true);
                expect(f.value()).toBe('second');
                await replace(f.options);
                expect(first.sheet === sheet).toBe(true);
                expect(f.element.querySelectorAll('style')).toHaveLength(1);
                const text = first.textContent;
                const changedBase = { ...f.options, baseUrl: 'https://example.test/other/main.css' };
                await replace(changedBase);
                expect(first.textContent).toBe(text); // Same bytes are insufficient if native identity changed.
                expect(first.sheet === sheet).toBe(false);
                sheet = first.sheet;
                first.setAttribute('data-cem-instance-style', 'tampered');
                await replace(changedBase);
                expect(first.getAttribute('data-cem-instance-style')).toBe('0');
                expect(first.sheet === sheet).toBe(true);
                first.textContent = ':host { --value: tampered; }';
                await replace(changedBase);
                expect(f.value()).toBe('client');
                expect(first.sheet === sheet).toBe(false);
                sheet = first.sheet;
                f.element.append(f.content);
                await replace(changedBase);
                expect(first === f.element.lastChild).toBe(true);
                expect(first.sheet === sheet).toBe(false);
                expect(f.element.querySelector('p') === f.content).toBe(true);
                active.dispose();
                expect(f.element.querySelector('style')).toBeNull();
                expect(f.native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
            } finally { f.dispose(); }
        }
    },
};
