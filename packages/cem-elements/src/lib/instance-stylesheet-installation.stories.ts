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
