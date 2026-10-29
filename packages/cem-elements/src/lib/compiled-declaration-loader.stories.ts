import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, within } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

export default { title: 'CEM Elements/Declarative Compiled Loading', tags: ['test'] } satisfies Meta;
const generated = import.meta.glob<string>('../../../../dist/artifacts/cem_ql/compiled-form/*', { eager: true, query: '?url', import: 'default' });
function scenario(fallback: boolean): StoryObj {
    return { render: () => '<section aria-label="Declarative compiled loading"></section>', play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector('section'); if (!container) throw Error('Missing fixture');
        const htmlUrl = generated['../../../../dist/artifacts/cem_ql/compiled-form/declarations.html'];
        if (!htmlUrl) throw Error('Build native declaration fixtures first');
        const absoluteHtml = new URL(htmlUrl, location.href);
        const html = await (await fetch(absoluteHtml)).text();
        const frame = canvasElement.ownerDocument.createElement('iframe'); container.append(frame);
        const document = frame.contentDocument, host = frame.contentWindow;
        if (!document || !host) throw Error('Missing isolated document');
        const base = document.createElement('base'); base.href = absoluteHtml.href; document.head.append(base);
        const scope = createCemDeclarationScope({ document }); let sourceLoads = 0, workerJobs = 0;
        const kinds: string[] = [], formats: string[] = [], requests: string[] = [];
        const runtime = new CemElementRuntime({ declarationScope: scope, retainedStylesheets: {},
            moduleUrlRoot: { importMap: { imports: {
                'compiled-entry-metadata': new URL('./cem-compiled-form-app.json', absoluteHtml).href,
                'compiled-entry-template': new URL('./cem-compiled-form-app.bin', absoluteHtml).href,
            } } },
            loadSrcDocument: async () => { sourceLoads++; throw Error('Source loading forbidden'); },
            processingWorkerFactory: request => {
                if (fallback) throw Error('Forced fallback');
                const worker = new Worker(request.scriptUrl, { type: request.type, name: request.name });
                const post = worker.postMessage.bind(worker);
                worker.postMessage = (message: unknown) => {
                    const packet = message as { operation?: string; payload?: { language?: string; source?: { kind: string; artifact?: { formatVersion: string } } } };
                    if (packet.operation === 'compile' && packet.payload?.language === 'cem-ml') { kinds.push(packet.payload.source?.kind ?? 'missing'); formats.push(packet.payload.source?.artifact?.formatVersion ?? 'missing'); }
                    post(message);
                };
                worker.addEventListener('message', ({ data }) => { if (data.direction === 'response' && data.outcome === 'success') workerJobs++; });
                return worker;
            } });
        const originalFetch = globalThis.fetch;
        let refuseMetadata = false;
        globalThis.fetch = async (input, init) => {
            const url = String(input); requests.push(url);
            if (refuseMetadata && url.endsWith('/cem-compiled-form-app.json')) return new Response('Unavailable', { status: 503 });
            return originalFetch(input, init);
        };
        try {
            document.body.innerHTML = html;
            const declarations = [...document.querySelectorAll<HTMLElement>('cem-element')];
            expect(declarations.length).toBe(6);
            const entry = declarations.find(element => element.getAttribute('tag') === 'cem-compiled-form-app');
            if (!entry) throw Error('Missing generated entry');
            const hash = entry.getAttribute('compiled-hash'); if (!hash) throw Error('Missing build pin');
            entry.setAttribute('compiled-hash', `cem-bin/1+blake3:${'0'.repeat(64)}`);
            runtime.install(host);
            await expect(runtime.whenDeclarationSettled(entry)).rejects.toThrow('metadata');
            expect(host.customElements.get('cem-compiled-form-app')).toBeUndefined();
            expect(requests.some(url => url.endsWith('/cem-compiled-form-app.bin'))).toBe(false);
            entry.setAttribute('compiled-hash', hash);
            // A failed request is retryable, never an invitation to compile source.
            refuseMetadata = true; runtime.registerDeclaration(entry);
            await expect(runtime.whenDeclarationSettled(entry)).rejects.toThrow('HTTP 503');
            refuseMetadata = false;
            runtime.registerDeclaration(entry);
            entry.setAttribute('compiled-template-src', './changed.bin');
            await expect(runtime.whenDeclarationSettled(entry)).rejects.toThrow('changed');
            entry.setAttribute('compiled-src', 'compiled-entry-metadata');
            entry.setAttribute('compiled-template-src', 'compiled-entry-template');
            runtime.registerDeclaration(entry);
            await Promise.all(declarations.map(element => runtime.whenDeclarationSettled(element)));
            expect(requests.every(url => !url.includes('.xhtml'))).toBe(true); expect(sourceLoads).toBe(0);
            const app = document.createElement('cem-compiled-form-app'); document.body.append(app);
            await runtime.whenRenderSettled(app);
            expect(host.getComputedStyle(app).getPropertyValue('--compiled-import-proof').trim()).toBe('imported');
            expect(host.getComputedStyle(app).getPropertyValue('--compiled-nested-proof').trim()).toBe('nested');
            expect(requests.some(url => /\.css(?:\?|$)/.test(url))).toBe(false);
            const card = app.querySelector<HTMLElement>('cem-card'); if (!card) throw Error('Missing compiled card');
            await runtime.whenRenderSettled(card);
            const field = app.querySelector<HTMLElement>('cem-text-field'), form = app.querySelector('form');
            if (!field || !form) throw Error('Missing live form');
            await runtime.whenRenderSettled(field);
            expect([...document.querySelectorAll('style')].some(style => style.textContent?.includes('cem-text-field'))).toBe(true);
            const control = within(field).getByRole('textbox', { name: 'Text field' });
            expect(new FormData(form).get('item')).toBe('Original');
            await userEvent.clear(control); await userEvent.type(control, 'Edited'); await runtime.whenRenderSettled(field);
            expect(new FormData(form).get('item')).toBe('Edited');
            control.focus(); (control as HTMLInputElement).setSelectionRange(1, 4);
            field.setAttribute('label', 'Renamed'); await runtime.whenRenderSettled(field);
            expect(within(field).getByRole('textbox', { name: 'Renamed' })).toBe(control);
            expect(document.activeElement).toBe(control);
            expect([(control as HTMLInputElement).selectionStart, (control as HTMLInputElement).selectionEnd]).toEqual([1, 4]);
            entry.remove(); document.body.prepend(entry); await runtime.whenDeclarationSettled(entry);
            expect(new FormData(form).get('item')).toBe('Edited');
            if (!fallback) { expect(workerJobs).toBeGreaterThan(0); expect(kinds.length).toBeGreaterThan(0);
                expect(kinds.every(kind => kind === 'pinned-template-artifact-v1')).toBe(true);
                expect(formats.every(format => format === 'cem-template-artifact/3')).toBe(true); }
            // A disposed scope drains even a fetch which never resolves.
            const lateScope = createCemDeclarationScope({ document });
            const lateRuntime = new CemElementRuntime({ declarationScope: lateScope, declarationTag: 'cem-cancelled-loader' });
            lateRuntime.install(host);
            let started: (() => void) | undefined;
            const reading = new Promise<void>(resolve => { started = resolve; });
            globalThis.fetch = () => { started?.(); return new Promise(() => undefined); };
            const late = document.createElement('cem-cancelled-loader');
            for (const name of ['compiled-src', 'compiled-template-src', 'compiled-hash']) late.setAttribute(name, entry.getAttribute(name) ?? '');
            late.setAttribute('compiled-src', new URL('./cem-compiled-form-app.json', absoluteHtml).href);
            late.setAttribute('compiled-template-src', new URL('./cem-compiled-form-app.bin', absoluteHtml).href);
            late.setAttribute('tag', 'cem-cancelled-app'); document.body.append(late);
            const settled = lateRuntime.whenDeclarationSettled(late);
            await reading; lateScope.dispose();
            await expect(settled).rejects.toThrow(); expect(host.customElements.get('cem-cancelled-app')).toBeUndefined();
        } finally { globalThis.fetch = originalFetch; scope.dispose(); frame.remove(); }
    } };
}
export const WorkerClosure = scenario(false);
export const FallbackClosure = scenario(true);
