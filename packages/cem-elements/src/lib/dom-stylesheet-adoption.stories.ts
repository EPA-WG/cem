import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, writeDataIslandHydrationData } from './cem-elements.js';
import { nativeCssStoryHost, nativeCssStoryContext } from './native-css-story-fixture.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { createCemProcessingFailureEnvelope, type CemProcessingRequestEnvelope, type CemProcessingResponseEnvelope, type CemProcessingReadyEnvelope } from './internal/runtime-support/processing-host.js';

export default { title: 'CEM Elements/DOM Stylesheet Adoption', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
let sequence = 0;

/** Hold initial CSS adoption until release, then permit native stylesheet preflights. */
class HeldCssWorker extends EventTarget {
    requests: CemProcessingRequestEnvelope<'compile'>[] = [];
    terminated = false;
    private released = false;
    cssReplays = 0;
    cssReplaysMatch = true;
    private readonly trace: { kind: string; operation?: CemProcessingRequestEnvelope['operation']; outcome?: string }[] = [];
    private record(kind: string, operation?: CemProcessingRequestEnvelope['operation'], outcome?: string): void {
        if (this.trace.length < 32) this.trace.push({ kind, operation, outcome });
    }
    inspect() { return { held: this.requests.length, released: this.released, cssReplays: this.cssReplays,
        cssReplaysMatch: this.cssReplaysMatch, terminated: this.terminated, trace: [...this.trace] }; }
    constructor(private worker: Worker) {
        super();
        for (const type of ['message', 'error', 'messageerror']) worker.addEventListener(type, event => {
            if (type === 'message') {
                const data = (event as MessageEvent<CemProcessingResponseEnvelope | CemProcessingReadyEnvelope>).data;
                if (data.direction === 'response') this.record('reply', data.operation, data.outcome);
                else if (data.direction === 'ready') this.record('ready');
            }
            this.dispatchEvent(type === 'message' ? new MessageEvent(type, { data: (event as MessageEvent).data })
                : new Event(type));
        });
    }
    postMessage(request: CemProcessingRequestEnvelope): void {
        if (request.operation === 'compile' && request.payload.language === 'css') {
            if (!this.released) {
                this.record('hold CSS', request.operation);
                this.requests.push(request);
                return;
            }
            this.cssReplays++;
            // Compare transport inputs only; never parse CSS or expose source in diagnostics.
            this.cssReplaysMatch &&= this.requests.some(held =>
                JSON.stringify(held.payload) === JSON.stringify(request.payload));
            this.record('replay CSS', request.operation);
        } else this.record('send', request.operation);
        this.worker.postMessage(request);
    }
    release(fail = false): void {
        if (this.released) return;
        // Open before forwarding: successful adoption can immediately schedule preflights.
        this.released = true;
        this.record(fail ? 'fail' : 'release');
        for (const request of this.requests) {
            if (fail) this.dispatchEvent(new MessageEvent('message', { data: createCemProcessingFailureEnvelope(request, 'failure',
                [{ code: 'fixture.css.failure', severity: 'error', message: 'native adoption fixture failure' }]) }));
            else this.worker.postMessage(request);
        }
    }
    terminate(): void { this.record('terminate'); this.terminated = true; this.worker.terminate(); }
}

function fixture(root: HTMLElement, markup: string) {
    const tag = `css-adoption-${++sequence}`;
    const scope = createCemDeclarationScope({ document });
    const workers: HeldCssWorker[] = [];
    const runtime = new CemElementRuntime({ declarationTag: `${tag}-definition`, declarationScope: scope,
        processingWorkerFactory: ({ scriptUrl, name, type }) => {
            const worker = new HeldCssWorker(new Worker(scriptUrl, { name, type }));
            workers.push(worker); return worker as unknown as Worker;
        } });
    const declaration = document.createElement(`${tag}-definition`);
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('version', '1.0.0');
    const template = document.createElement('template'); template.innerHTML = markup;
    declaration.append(template); root.append(declaration);
    runtime.registerDeclaration(declaration);
    const instance = document.createElement(tag); root.append(instance);
    return { runtime, declaration, instance, scope, workers, tag };
}

export const ReadinessAndNativeDiagnostics: Story = {
    render: () => '<section aria-label="DOM CSS adoption readiness"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('stylesheet fixture root is missing');
        const f = fixture(root, '<style>:scope { --adopted: yes; }</style><style>a { color: red</style>'
            + '<style type="text/less">a { color:red; }</style><style type="{$type}">a {color:red;}</style><p>Ready</p>');
        let phase = 'setup';
        let hydrationSettled = false; let declarationSettled = false; let renderSettled = false;
        if (import.meta.env.MODE === 'test') {
            const { onTestFailed, onTestFinished } = await import('vitest');
            onTestFailed(() => { throw new Error(`held CSS readiness: ${JSON.stringify({ phase,
                hydrationSettled, declarationSettled, renderSettled,
                rendered: !!f.instance.querySelector('p'),
                styles: f.declaration.querySelectorAll('style[data-cem-declaration-style]').length,
                workers: f.workers.slice(0, 4).map(worker => worker.inspect()),
            })}`); });
            onTestFinished(() => f.scope.dispose());
        }
        try {
            const second = document.createElement(f.tag); root.append(second);
            const hydrated = document.createElement(f.tag);
            const snapshot = f.runtime.snapshotInstance(document.createElement(f.tag));
            const island = document.createElement('template');
            island.setAttribute('data-cem-island', 'instance');
            writeDataIslandHydrationData(island, snapshot);
            const serverContent = document.createElement('p'); serverContent.textContent = 'Server retained';
            serverContent.setAttribute('data-cem-template-artifact-id', snapshot.templateArtifactId);
            serverContent.setAttribute('data-cem-data-revision', snapshot.dataRevision);
            serverContent.setAttribute('data-cem-source-fidelity', 'dom-canonical');
            hydrated.append(island, document.createComment('cem-render-start'), serverContent, document.createComment('cem-render-end'));
            root.append(hydrated);
            const hydration = f.runtime.whenRenderSettled(hydrated).then(() => { hydrationSettled = true; });
            const declaration = f.runtime.whenDeclarationSettled(f.declaration).then(() => { declarationSettled = true; });
            const render = f.runtime.whenRenderSettled(f.instance).then(() => { renderSettled = true; });
            phase = 'held CSS';
            await waitFor(() => expect(f.workers[0]?.requests.length).toBe(1), { timeout: 15000 });
            expect(declarationSettled).toBe(false); expect(renderSettled).toBe(false);
            expect(hydrationSettled).toBe(false); expect(hydrated.querySelector('p')).toBe(serverContent);
            expect(f.instance.querySelector('p')).toBeNull(); expect(second.querySelector('p')).toBeNull();
            expect(f.declaration.querySelector('style[data-cem-declaration-style]')).toBeNull();
            f.instance.remove(); root.append(f.instance);
            f.workers[0].release();
            phase = 'released CSS';
            await Promise.all([declaration, render, hydration, f.runtime.whenRenderSettled(f.instance), f.runtime.whenRenderSettled(second)]);
            expect(f.workers[0].requests).toHaveLength(1);
            expect(f.workers[0].cssReplays).toBeGreaterThan(0);
            expect(f.workers[0].cssReplaysMatch).toBe(true);
            expect(hydrated.querySelector('p')).toBe(serverContent);
            expect(f.instance.querySelector('p')?.textContent).toBe('Ready');
            expect(second.querySelector('p')?.textContent).toBe('Ready');
            expect(getComputedStyle(f.instance).getPropertyValue('--adopted').trim()).toBe('yes');
            expect(f.declaration.querySelectorAll('style[data-cem-declaration-style]')).toHaveLength(1);
            const codes = f.runtime.diagnosticsFor(f.declaration).map(d => d.code);
            expect(codes).toContain('cem.ql.template.stylesheet_parse_failed');
            expect(codes).toContain('cem.ql.template.stylesheet_content_type_unsupported');
            expect(codes).toContain('cem-element.stylesheet_dynamic_unsupported');
        } finally { f.scope.dispose(); }
    },
};

export const FailureDisposalAndNoStyle: Story = {
    render: () => '<section aria-label="DOM CSS adoption failures"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('stylesheet fixture root is missing');
        const plain = fixture(root, '<p>Immediate</p>');
        await plain.runtime.whenRenderSettled(plain.instance);
        expect(plain.instance.querySelector('p')?.textContent).toBe('Immediate');
        expect(plain.workers).toHaveLength(0); plain.scope.dispose();
        for (const dispose of [false, true]) {
            const f = fixture(root, '<style>:scope { --adopted: yes; }</style><p>Settled</p>');
            await waitFor(() => expect(f.workers[0]?.requests.length).toBe(1), { timeout: 15000 });
            if (dispose) f.scope.dispose(); else f.workers[0].release(true);
            await Promise.all([f.runtime.whenDeclarationSettled(f.declaration), f.runtime.whenRenderSettled(f.instance)]);
            expect(f.declaration.querySelector('style[data-cem-declaration-style]')).toBeNull();
            if (dispose) {
                expect(f.instance.querySelector('p')).toBeNull();
                // Readiness cancels on disposal; physical termination drains admitted native work.
                f.workers[0].release();
                await waitFor(() => expect(f.workers[0].terminated).toBe(true));
                expect(f.instance.querySelector('p')).toBeNull();
                expect(f.declaration.querySelector('style[data-cem-declaration-style]')).toBeNull();
            } else {
                expect(f.instance.querySelector('p')?.textContent).toBe('Settled');
                expect(f.runtime.diagnosticsFor(f.declaration).some(d => d.code === 'cem-element.stylesheet_adoption_failed')).toBe(true);
                f.scope.dispose();
            }
        }
    },
};

/** Same retained source and owner must produce the same CSS across host modes. */
export const NativeOutputParity: Story = {
    render: () => '<section aria-label="Native stylesheet output parity"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('native stylesheet fixture root is missing');
        const css = [
            ':host { --adopted: yes; }',
            'span { animation: pulse 20s linear infinite; }',
            '@keyframes pulse { from { opacity: 0.25; } to { opacity: 0.75; } }',
            '@font-face { font-family: forbidden; src: url("./forbidden.woff2"); }',
            '@scope (.escape) { span { color: red; } }',
        ].join('\n');
        let workerCss: string | undefined;
        let workerOwner: string | undefined;
        for (const fallback of [false, true]) {
            const native = nativeCssStoryHost(fallback);
            try {
                const { artifact, diagnostics } = await native.compile('native-parity-card', [
                    { css, scope: 'native-parity' },
                    { css: 'a { color: red', scope: null },
                    { css: 'a {}', scope: null, contentType: 'text/less' },
                ]);
                expect(diagnostics.map(d => d.code)).toEqual(expect.arrayContaining([
                    'cem.ql.template.stylesheet_parse_failed', 'cem.ql.template.stylesheet_content_type_unsupported',
                ]));
                const result = await native.host.stylesheet({ action: 'begin', artifact,
                    consumer: 'parity', index: 0, baseUrl: document.baseURI,
                    context: nativeCssStoryContext, scope: { kind: 'shared', name: 'native-parity' } }).result;
                if (result.status !== 'ready') throw new Error('expected native CSS ready');
                expect(native.host.mode).toBe(fallback ? 'main-thread' : 'worker');
                expect(result.diagnostics.map(d => d.code)).toEqual(expect.arrayContaining([
                    'cem.scoped_css.global_construct_unsupported', 'cem.scoped_css.authored_scope_unsupported',
                ]));
                expect(result.css).not.toContain('@font-face');
                expect(result.css).not.toContain('.escape');
                if (fallback) {
                    expect(result.css).toBe(workerCss);
                    expect(result.identity.ownerKey).toBe(workerOwner);
                } else { workerCss = result.css; workerOwner = result.identity.ownerKey; }
                root.innerHTML = '<native-parity-card scope="native-parity"><template data-cem-island="instance"></template><span>First</span></native-parity-card>' +
                    '<native-parity-other scope="native-parity"><template data-cem-island="instance"></template><span>Second</span></native-parity-other>' +
                    '<table><tbody><tr><th scope="native-parity"><span>Unmanaged</span></th></tr></tbody></table>';
                if (result.identity.contextMarker) {
                    for (const host of root.querySelectorAll('[scope]')) {
                        host.setAttribute('data-cem-css-context', result.identity.contextMarker);
                    }
                }
                const style = document.createElement('style'); style.textContent = result.css; root.append(style);
                const spans = root.querySelectorAll('span');
                const first = getComputedStyle(spans[0]);
                expect(first.getPropertyValue('--adopted').trim()).toBe('yes');
                expect(getComputedStyle(spans[1]).getPropertyValue('--adopted').trim()).toBe('yes');
                expect(getComputedStyle(spans[2]).getPropertyValue('--adopted').trim()).toBe('');
                expect(first.animationName).not.toBe('pulse');
                expect(first.animationName).not.toBe('none');
                expect(getComputedStyle(spans[1]).animationName).toBe(first.animationName);
                expect(getComputedStyle(spans[2]).animationName).toBe('none');
                expect(spans[0].getAnimations()).toHaveLength(1);
                expect(await native.host.stylesheet({ action: 'release', artifact, consumer: 'parity', loadId: result.loadId }).result)
                    .toMatchObject({ status: 'released', count: 1 });
            } finally { root.replaceChildren(); native.dispose(); }
        }
    },
};
