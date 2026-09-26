import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, writeDataIslandHydrationData } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { createCemProcessingFailureEnvelope, type CemProcessingRequestEnvelope } from './internal/runtime-support/processing-host.js';

export default { title: 'CEM Elements/DOM Stylesheet Adoption', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
let sequence = 0;

/** Delay the real worker's CSS request; native adoption still runs in WASM. */
class HeldCssWorker extends EventTarget {
    requests: CemProcessingRequestEnvelope<'compile'>[] = [];
    terminated = false;
    constructor(private worker: Worker) {
        super();
        for (const type of ['message', 'error', 'messageerror']) worker.addEventListener(type, event => {
            this.dispatchEvent(type === 'message' ? new MessageEvent(type, { data: (event as MessageEvent).data })
                : new Event(type));
        });
    }
    postMessage(request: CemProcessingRequestEnvelope): void {
        if (request.operation === 'compile' && request.payload.language === 'css') this.requests.push(request);
        else this.worker.postMessage(request);
    }
    release(fail = false): void {
        for (const request of this.requests) {
            if (fail) this.dispatchEvent(new MessageEvent('message', { data: createCemProcessingFailureEnvelope(request, 'failure',
                [{ code: 'fixture.css.failure', severity: 'error', message: 'native adoption fixture failure' }]) }));
            else this.worker.postMessage(request);
        }
    }
    terminate(): void { this.terminated = true; this.worker.terminate(); }
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
            let hydrationSettled = false;
            const hydration = f.runtime.whenRenderSettled(hydrated).then(() => { hydrationSettled = true; });
            let declarationSettled = false; let renderSettled = false;
            const declaration = f.runtime.whenDeclarationSettled(f.declaration).then(() => { declarationSettled = true; });
            const render = f.runtime.whenRenderSettled(f.instance).then(() => { renderSettled = true; });
            await waitFor(() => expect(f.workers[0]?.requests.length).toBe(1), { timeout: 15000 });
            expect(declarationSettled).toBe(false); expect(renderSettled).toBe(false);
            expect(hydrationSettled).toBe(false); expect(hydrated.querySelector('p')).toBe(serverContent);
            expect(f.instance.querySelector('p')).toBeNull(); expect(second.querySelector('p')).toBeNull();
            expect(f.declaration.querySelector('style[data-cem-declaration-style]')).toBeNull();
            f.instance.remove(); root.append(f.instance);
            f.workers[0].release();
            await Promise.all([declaration, render, hydration, f.runtime.whenRenderSettled(f.instance), f.runtime.whenRenderSettled(second)]);
            expect(f.workers[0].requests).toHaveLength(1);
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
                expect(f.workers[0].terminated).toBe(true);
            } else {
                expect(f.instance.querySelector('p')?.textContent).toBe('Settled');
                expect(f.runtime.diagnosticsFor(f.declaration).some(d => d.code === 'cem-element.stylesheet_adoption_failed')).toBe(true);
                f.scope.dispose();
            }
        }
    },
};
