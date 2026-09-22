import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, type CemSrcDocumentLoadResult } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

const meta: Meta = { title: 'CEM Elements/Declaration Source Retry', tags: ['test'] };
export default meta;
type Story = StoryObj;
const SOURCE = '<template id="card" type="text/cem-ml">{p | Ready}</template>';

function deferred<T>() {
    let resolve!: (value: T) => void;
    let reject!: (reason: Error) => void;
    const promise = new Promise<T>((accept, fail) => { resolve = accept; reject = fail; });
    return { promise, resolve, reject };
}

function mount(root: HTMLElement, runtime: CemElementRuntime, tag: string, src: string) {
    const declaration = document.createElement(runtime.declarationTag);
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('src', src);
    const instance = document.createElement(tag);
    root.append(declaration, instance);
    return { declaration, instance };
}

function recoveryStory(failure: 'transport' | 'stream'): Story {
    return {
        render: () => document.createElement('section'),
        play: async ({ canvasElement }) => {
            const scope = createCemDeclarationScope({ document });
            const firstLoad = deferred<string | CemSrcDocumentLoadResult>();
            const retryLoad = deferred<string>();
            let requests = 0;
            const runtime = new CemElementRuntime({
                declarationTag: `cem-retry-${failure}-declaration`, declarationScope: scope,
                loadSrcDocument: () => ++requests === 1 ? firstLoad.promise : retryLoad.promise,
            });
            runtime.install(window);
            const src = `/retry-${failure}.html#card`;
            try {
                const first = mount(canvasElement, runtime, `story-retry-${failure}-first`, src);
                const second = mount(canvasElement, runtime, `story-retry-${failure}-second`, src);
                runtime.registerDeclaration(first.declaration);
                expect(requests).toBe(1);
                expect(customElements.get(first.instance.localName)).toBeUndefined();

                if (failure === 'transport') {
                    firstLoad.reject(new Error('HTTP 503: controlled source failure'));
                } else {
                    const reading = deferred<void>();
                    let controller: ReadableStreamDefaultController<Uint8Array> | undefined;
                    let sent = false;
                    firstLoad.resolve({
                        resolvedUrl: new URL(src, document.baseURI).href,
                        contentType: 'text/html',
                        body: new ReadableStream<Uint8Array>({
                            start(value) { controller = value; },
                            pull(value) {
                                if (sent) return;
                                sent = true;
                                value.enqueue(new TextEncoder().encode('<template id="card"'));
                                reading.resolve();
                            },
                        }),
                    });
                    await reading.promise;
                    if (!controller) throw new Error('response stream did not start');
                    controller.error(new Error('controlled interrupted response'));
                }
                await Promise.all([first, second].map(owner => runtime.whenDeclarationSettled(owner.declaration)));
                for (const owner of [first, second]) {
                    expect(runtime.diagnosticsFor(owner.declaration).map(diagnostic => diagnostic.code))
                        .toEqual(['cem-element.src_load_failed']);
                    expect(customElements.get(owner.instance.localName)).toBeUndefined();
                    expect(owner.instance.querySelector('p')).toBeNull();
                }
                expect(requests).toBe(1); // Failure itself does not schedule a retry.

                // Reconnection, explicit registration and a fresh declaration share
                // one new acquisition while it is pending.
                first.declaration.remove();
                canvasElement.append(first.declaration);
                runtime.registerDeclaration(second.declaration);
                const replacement = mount(canvasElement, runtime, `story-retry-${failure}-replacement`, src);
                expect(requests).toBe(2);
                retryLoad.resolve(SOURCE);
                for (const owner of [first, second, replacement]) {
                    await runtime.whenDeclarationSettled(owner.declaration);
                    await waitFor(() => expect(owner.instance.querySelector('p')).toHaveTextContent('Ready'));
                }
                // Diagnostics remain attempt history, rather than disappearing on success.
                expect(runtime.diagnosticsFor(first.declaration).map(diagnostic => diagnostic.code))
                    .toEqual(['cem-element.src_load_failed']);
                expect(runtime.diagnosticsFor(replacement.declaration)).toEqual([]);
                const cached = mount(canvasElement, runtime, `story-retry-${failure}-cached`, src);
                await runtime.whenDeclarationSettled(cached.declaration);
                await waitFor(() => expect(cached.instance.querySelector('p')).toHaveTextContent('Ready'));
                expect(requests).toBe(2);
                expect(runtime.diagnosticsFor(cached.declaration)).toEqual([]);
            } finally {
                firstLoad.resolve(SOURCE);
                retryLoad.resolve(SOURCE);
                canvasElement.replaceChildren();
                scope.dispose();
            }
        },
    };
}

export const FailedTransportCanRetry = recoveryStory('transport');
export const InterruptedBodyCanRetry = recoveryStory('stream');

export const RetryCannotReviveDisposedScope: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const disposal of ['direct', 'ancestor']) {
            const parent = createCemDeclarationScope({ document });
            const scope = createCemDeclarationScope({ document, parent });
            let requests = 0;
            const runtime = new CemElementRuntime({
                declarationTag: `cem-retry-disposed-${disposal}`, declarationScope: scope,
                loadSrcDocument: async () => { requests += 1; throw new Error('controlled failure'); },
            });
            runtime.install(window);
            try {
                const first = mount(canvasElement, runtime, `story-retry-disposed-${disposal}`, '/retry-disposed.html#card');
                await runtime.whenDeclarationSettled(first.declaration);
                expect(requests).toBe(1);
                (disposal === 'direct' ? scope : parent).dispose();
                first.declaration.remove();
                canvasElement.append(first.declaration);
                expect(runtime.registerDeclaration(first.declaration)).toBe(false);
                const replacement = mount(canvasElement, runtime, `story-retry-disposed-new-${disposal}`, '/retry-disposed.html#card');
                await runtime.whenDeclarationSettled(replacement.declaration);
                const code = disposal === 'direct' ? 'cem-element.scope_disposed' : 'cem-element.scope_ancestor_disposed';
                expect(runtime.diagnosticsFor(first.declaration).map(diagnostic => diagnostic.code)).toContain(code);
                expect(runtime.diagnosticsFor(replacement.declaration).map(diagnostic => diagnostic.code)).toContain(code);
                expect(requests).toBe(1);
                expect(customElements.get(first.instance.localName)).toBeUndefined();
                expect(customElements.get(replacement.instance.localName)).toBeUndefined();
            } finally {
                canvasElement.replaceChildren();
                scope.dispose();
                parent.dispose();
            }
        }
    },
};

export const DisposalDuringRetryPreventsRegistration: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const scope = createCemDeclarationScope({ document });
        const retry = deferred<string>();
        let requests = 0;
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-dispose-pending-retry', declarationScope: scope,
            loadSrcDocument: () => ++requests === 1 ? Promise.reject(new Error('controlled failure')) : retry.promise,
        });
        runtime.install(window);
        try {
            const first = mount(canvasElement, runtime, 'story-dispose-pending-retry', '/retry-pending.html#card');
            await runtime.whenDeclarationSettled(first.declaration);
            runtime.registerDeclaration(first.declaration);
            expect(requests).toBe(2);
            scope.dispose();
            retry.resolve(SOURCE);
            await runtime.whenDeclarationSettled(first.declaration);
            expect(customElements.get(first.instance.localName)).toBeUndefined();
            expect(first.instance.querySelector('p')).toBeNull();
            expect(runtime.diagnosticsFor(first.declaration).map(diagnostic => diagnostic.code))
                .toContain('cem-element.scope_disposed');
        } finally {
            retry.resolve(SOURCE);
            canvasElement.replaceChildren();
            scope.dispose();
        }
    },
};
