import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, writeDataIslandHydrationData, type CemElementRuntimeOptions } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

export default { title: 'CEM Elements/Retained Stylesheet Runtime', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
type Reader = NonNullable<NonNullable<CemElementRuntimeOptions['retainedStylesheets']>['read']>;
function response(css: string) {
    return { bytes: new TextEncoder().encode(css).buffer, finalUrl: 'https://example.test/cdn/child.css', contentType: 'text/css' };
}
function held() {
    let resolve: (value: ReturnType<typeof response>) => void = () => { throw new Error('missing resolver'); };
    const promise = new Promise<ReturnType<typeof response>>(done => { resolve = done; });
    return { promise, resolve };
}
let sequence = 0;
function fixture(root: HTMLElement, fallback: boolean, read?: Reader) {
    const parent = createCemDeclarationScope({ document });
    const scope = createCemDeclarationScope({ document, parent });
    const suffix = `native-${++sequence}-${fallback ? 'fallback' : 'worker'}`;
    let workerCalls = 0;
    const runtime = new CemElementRuntime({ declarationTag: `declaration-${suffix}`, declarationScope: scope,
        retainedStylesheets: { read },
        moduleUrlRoot: { baseUrl: 'https://example.test/', importMap: {} },
        processingWorkerFactory: request => {
            workerCalls++;
            if (fallback) throw new Error('fixture selects fallback');
            return new Worker(request.scriptUrl, { type: request.type, name: request.name });
        } });
    function declare(name: string, source: string, mode: 'dom' | 'cem-ml' = 'cem-ml', sharedScope?: string) {
        const tag = `${name}-${suffix}`;
        const declaration = document.createElement('div'); declaration.setAttribute('tag', tag); declaration.setAttribute('version', '1.0.0');
        if (sharedScope) declaration.setAttribute('scope', sharedScope);
        const template = document.createElement('template');
        if (mode === 'cem-ml') { template.setAttribute('type', 'text/cem-ml'); template.textContent = source; }
        else template.innerHTML = source;
        declaration.append(template); root.append(declaration);
        runtime.registerDeclaration(declaration);
        return { declaration, tag, ready: runtime.whenDeclarationSettled(declaration).then(() => { expect(runtime.diagnosticsFor(declaration).filter(d => d.code !== 'cem.processing_host.worker_startup_fallback')).toEqual([]); }) };
    }
    return { runtime, scope, parent, suffix, declare, workerCalls: () => workerCalls,
        dispose() { root.replaceChildren(); scope.dispose(); parent.dispose(); } };
}
const css = (value: string, scope = '') => `{style ${scope ? `@scope=${scope}` : ''} |\`\`\`\n${value} \`\`\`\n}`;

export const PayloadReplacementPreservesActiveStyles: Story = {
    render: () => '<section aria-label="Staged runtime payload styles"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing staged payload fixture');
        for (const fallback of [false, true]) {
            let gate = held(); let reads = 0;
            const f = fixture(root, fallback, async () => { reads++; return gate.promise; });
            try {
                const declaration = f.declare('staged-payload', '{p | Content}');
                await declaration.ready;
                const instance = document.createElement(declaration.tag);
                const payload = document.createElement('template');
                payload.innerHTML = '<style>p { --instance: old; }</style>';
                instance.append(payload); root.append(instance);
                await f.runtime.whenRenderSettled(instance);
                const island = instance.querySelector<HTMLTemplateElement>('template[data-cem-island="instance"]');
                const source = island?.content.querySelector('style');
                const style = instance.querySelector('style[data-cem-instance-style]');
                if (!island || !source || !style) throw new Error('missing initial payload stylesheet');
                const value = () => getComputedStyle(instance.querySelector('p') as Element).getPropertyValue('--instance').trim();
                expect(value()).toBe('old');
                source.textContent = '@import "./replacement.css";';
                await waitFor(() => expect(reads).toBe(1));
                expect(instance.querySelector('style[data-cem-instance-style]')).toBe(style);
                expect(value()).toBe('old');
                gate.resolve(response('p { --instance: new; }'));
                await waitFor(() => expect(value()).toBe('new'));
                await f.runtime.whenRenderSettled(instance);
                expect(instance.querySelector('style[data-cem-instance-style]')).toBe(style);

                gate = held();
                source.textContent = '@import "./failed.css"; p { --instance: partial; }';
                await waitFor(() => expect(reads).toBe(2));
                expect(value()).toBe('new');
                gate.resolve({ ...response('invalid import'), contentType: 'text/html' });
                await f.runtime.whenRenderSettled(instance);
                expect(value()).toBe('new');
                expect(instance.querySelector('style[data-cem-instance-style]')).toBe(style);
                expect(f.runtime.diagnosticsFor(instance).some(d => d.code === 'cem.css.import_content_type')).toBe(true);

                gate = held();
                source.textContent = '@import "./superseded.css";';
                await waitFor(() => expect(reads).toBe(3));
                expect(value()).toBe('new');
                source.textContent = 'p { --instance: latest; }';
                await waitFor(() => expect(value()).toBe('latest'));
                gate.resolve(response('p { --instance: obsolete; }'));
                await f.runtime.whenRenderSettled(instance);
                expect(value()).toBe('latest');
                expect(instance.querySelectorAll('style[data-cem-instance-style]')).toHaveLength(1);

                const payloadParent = source.parentNode;
                if (!payloadParent) throw new Error('missing retained payload parent');
                source.remove();
                await waitFor(() => expect(instance.querySelectorAll('style[data-cem-instance-style]')).toHaveLength(0));
                await f.runtime.whenRenderSettled(instance);
                expect(value()).toBe('');
                source.textContent = 'p { --instance: restored; }';
                payloadParent.appendChild(source);
                await waitFor(() => expect(value()).toBe('restored'));
                gate = held();
                source.textContent = '@import "./disconnect.css";';
                await waitFor(() => expect(reads).toBe(4));
                expect(value()).toBe('restored');
                instance.remove();
                gate.resolve(response('p { --instance: disconnected; }'));
                await f.runtime.whenRenderSettled(instance);
                expect(instance.querySelectorAll('style[data-cem-instance-style]')).toHaveLength(0);
                expect(f.workerCalls()).toBeGreaterThan(0);
            } finally { gate.resolve(response('')); f.dispose(); }
        }
    },
};

export const RecoveryPreservesRetainedInstanceStyles: Story = {
    render: () => '<section aria-label="Retained CSS recovery"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing recovery fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const declaration = f.declare('recovery-card', '{attribute @name=label | Before}{p | {$label}}');
                await declaration.ready;
                const instance = document.createElement(declaration.tag);
                const payload = document.createElement('template');
                payload.innerHTML = '<style>p { --instance: retained; }</style>';
                instance.append(payload); root.append(instance);
                await f.runtime.whenRenderSettled(instance);
                const style = instance.querySelector('style[data-cem-instance-style]');
                const paragraph = instance.querySelector('p');
                if (!style || !paragraph) throw new Error('missing initial instance output');
                expect(paragraph.textContent).toBe('Before');
                expect(instance.querySelectorAll('style')).toHaveLength(1);
                paragraph.remove();
                instance.setAttribute('label', 'After');
                await waitFor(() => expect(instance.querySelector('p')?.textContent).toBe('After'));
                await f.runtime.whenRenderSettled(instance);
                expect(instance.querySelector('p')).not.toBe(paragraph);
                expect(instance.querySelectorAll('style')).toHaveLength(1);
                expect(instance.querySelector('style[data-cem-instance-style]')).toBe(style);
                expect(getComputedStyle(instance.querySelector('p') as HTMLElement).getPropertyValue('--instance').trim()).toBe('retained');
                expect(f.runtime.diagnosticsFor(instance).some(d => d.code === 'cem-element.processing_host_render_failed')).toBe(false);
                expect(f.workerCalls()).toBeGreaterThan(0);
            } finally { f.dispose(); }
        }
    },
};

export const RenderAndHydrationWaitForImports: Story = {
    render: () => '<section aria-label="Runtime native CSS readiness"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing runtime fixture');
        for (const fallback of [false, true]) for (const mode of ['dom', 'cem-ml'] as const) {
            let gate = held(); let reads = 0;
            const f = fixture(root, fallback, async () => { reads++; return gate.promise; });
            try {
                const source = mode === 'dom' ? '<attribute name=label>Ready</attribute><style>@import "./child.css";</style><p>${$label}</p>'
                    : `{attribute @name=label | Ready}${css('@import "./child.css";')}{p | {$label}}`;
                const declaration = f.declare('ready-card', source, mode); await declaration.ready;
                const instance = document.createElement(declaration.tag); root.append(instance);
                let settled = false;
                const ready = f.runtime.whenRenderSettled(instance).then(() => { settled = true; });
                await waitFor(() => expect(reads).toBe(1));
                expect(settled).toBe(false); expect(instance.querySelector('p')).toBeNull();
                gate.resolve(response(':host { --ready: url(./ready.svg); }'));
                await ready;
                expect(instance.querySelector('p')?.textContent).toBe('Ready');
                expect(getComputedStyle(instance).getPropertyValue('--ready')).toContain('/cdn/ready.svg');
                const snapshot = f.runtime.snapshotInstance(instance);
                expect(snapshot.hostAttributes['data-cem-css-context']).toBeUndefined();
                expect(snapshot.dataset.cemCssContext).toBeUndefined();
                const paragraph = instance.querySelector('p');
                const revision = paragraph?.getAttribute('data-cem-data-revision');
                if (!revision) throw new Error('missing render revision');
                snapshot.dataRevision = revision;
                const markup = instance.innerHTML; instance.remove();
                gate = held();
                const restored = document.createElement(declaration.tag);
                for (const [name, value] of Object.entries(snapshot.hostAttributes)) restored.setAttribute(name, value);
                restored.innerHTML = markup;
                const retained = restored.querySelector('p');
                const island = restored.querySelector<HTMLTemplateElement>('template[data-cem-island="instance"]');
                if (!island) throw new Error('missing hydration island');
                writeDataIslandHydrationData(island, snapshot);
                root.append(restored);
                let hydrated = false;
                const hydration = f.runtime.whenRenderSettled(restored).then(() => { hydrated = true; });
                await waitFor(() => expect(reads).toBe(2));
                expect(hydrated).toBe(false); expect(restored.querySelector('p')).toBe(retained);
                gate.resolve(response(':host { --ready: url(./restored.svg); }'));
                await hydration;
                expect(restored.querySelector('p')).toBe(retained);
                expect(getComputedStyle(restored).getPropertyValue('--ready')).toContain('/cdn/restored.svg');
                const marker = restored.getAttribute('data-cem-css-context');
                restored.setAttribute('data-cem-css-context', 'tampered');
                await waitFor(() => expect(restored.getAttribute('data-cem-css-context')).toBe(marker));
                expect(restored.querySelector('p')).toBe(retained);
                expect(f.runtime.diagnosticsFor(restored)).toEqual([]);
                restored.setAttribute('label', 'Updated');
                await waitFor(() => expect(restored.querySelector('p')?.textContent).toBe('Updated'));
                await f.runtime.whenRenderSettled(restored);
                expect(reads).toBe(2);
                expect(f.workerCalls()).toBeGreaterThan(0);
            } finally { f.dispose(); }
        }
    },
};

export const SharedSourcesAndConsumingModuleMaps: Story = {
    render: () => '<section aria-label="Runtime shared CSS module contexts"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing shared fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback, async () => { throw new Error('unexpected import'); });
            try {
                const sharedScope = `shared-${f.suffix}`;
                const consumer = f.declare('shared-consumer', '{p | Consumer}', 'cem-ml', sharedScope);
                await consumer.ready;
                const hosts: HTMLElement[] = [];
                for (const variant of ['a', 'b']) {
                    const wrapper = f.declare(`map-${variant}`, `{module-map | {resource @specifier=icon @target="https://example.test/${variant}.svg"}}{slot}`);
                    await wrapper.ready;
                    const parent = document.createElement(wrapper.tag);
                    const payload = document.createElement('template');
                    payload.innerHTML = `<${consumer.tag}></${consumer.tag}>`; parent.append(payload); root.append(parent);
                    await f.runtime.whenRenderSettled(parent);
                    const child = parent.querySelector<HTMLElement>(consumer.tag);
                    if (!child) throw new Error('missing mapped consumer');
                    await f.runtime.whenRenderSettled(child); hosts.push(child);
                }
                // No instances of this declaration: registration alone contributes shared sources.
                const source = f.declare('shared-source', css(':host { --asset: url(icon); }', sharedScope), 'cem-ml', sharedScope);
                await source.ready;
                await Promise.all(hosts.map(host => f.runtime.whenRenderSettled(host)));
                for (const [index, host] of hosts.entries()) {
                    expect(getComputedStyle(host).getPropertyValue('--asset')).toContain(index === 0 ? '/a.svg' : '/b.svg');
                }
                expect(source.declaration.querySelectorAll('style[data-cem-declaration-style]')).toHaveLength(2);
                const parent = hosts[0].parentElement;
                if (!parent) throw new Error('missing consumer parent');
                const otherParent = hosts[1].parentElement;
                if (!otherParent) throw new Error('missing second parent');
                otherParent.append(hosts[0]); await f.runtime.whenRenderSettled(hosts[0]);
                expect(getComputedStyle(hosts[0]).getPropertyValue('--asset')).toContain('/b.svg');
                expect(source.declaration.querySelectorAll('style[data-cem-declaration-style]')).toHaveLength(1);
            } finally { f.dispose(); }
        }
    },
};

export const DisconnectAndSourceDiagnostics: Story = {
    render: () => '<section aria-label="Runtime stylesheet cancellation and diagnostics"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing cancellation fixture');
        for (const fallback of [false, true]) {
            const gate = held(); let signal: AbortSignal | undefined; let reads = 0;
            const f = fixture(root, fallback, async (_request, current) => {
                if (++reads === 1) { signal = current; return gate.promise; }
                throw new Error('network offline');
            });
            try {
                const declaration = f.declare('cancel-card', `${css('@import "./child.css";')}${css(':host { --sibling: yes; }')}{p | Content}`);
                await declaration.ready;
                const instance = document.createElement(declaration.tag); root.append(instance);
                const pending = f.runtime.whenRenderSettled(instance);
                await waitFor(() => expect(signal).toBeDefined());
                instance.remove(); await pending;
                expect(signal?.aborted).toBe(true);
                root.append(instance); await f.runtime.whenRenderSettled(instance);
                expect(getComputedStyle(instance).getPropertyValue('--sibling').trim()).toBe('yes');
                expect(instance.querySelector('p')?.textContent).toBe('Content');
                const diagnostic = f.runtime.diagnosticsFor(instance).find(d => d.code === 'cem.css.import_load_failed');
                expect(diagnostic).toMatchObject({ offset: 1, message: 'network offline', sourceUri: expect.stringMatching(/^urn:cem:template-style:/) });
                expect(diagnostic?.stylesheetUrl).toBe(document.baseURI);
                gate.resolve(response(':host { --stale: no; }'));
                await f.runtime.whenRenderSettled(instance);
                expect(getComputedStyle(instance).getPropertyValue('--stale').trim()).toBe('');
                expect(f.runtime.diagnosticsFor(instance).filter(d => d.code === 'cem.css.import_load_failed')).toHaveLength(1);
                f.scope.dispose();
                expect(declaration.declaration.querySelectorAll('style[data-cem-declaration-style]')).toHaveLength(0);
                expect(instance.hasAttribute('data-cem-css-context')).toBe(false);
            } finally { f.dispose(); }
        }
    },
};

export const AncestorDisposalStopsPendingRender: Story = {
    render: () => '<section aria-label="Pending CSS ancestor disposal"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing ancestor fixture');
        for (const fallback of [false, true]) {
            const gate = held(); let signal: AbortSignal | undefined;
            const f = fixture(root, fallback, async (_request, current) => { signal = current; return gate.promise; });
            try {
                const declaration = f.declare('disposed-card', `${css('@import "./child.css";')}{p | Must not render}`);
                await declaration.ready;
                const instance = document.createElement(declaration.tag); root.append(instance);
                const pending = f.runtime.whenRenderSettled(instance);
                await waitFor(() => expect(signal).toBeDefined());
                f.parent.dispose(); await pending;
                expect(signal?.aborted).toBe(true);
                expect(instance.querySelector('p')).toBeNull();
                gate.resolve(response(':host { --late: no; }'));
                await f.runtime.whenRenderSettled(instance);
                expect(declaration.declaration.querySelectorAll('style[data-cem-declaration-style]')).toHaveLength(0);
                expect(instance.hasAttribute('data-cem-css-context')).toBe(false);
            } finally { f.dispose(); }
        }
    },
};

export const DefaultBrowserByteTransport: Story = {
    render: () => '<section aria-label="Runtime default CSS byte transport"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing default transport fixture');
        const url = new URL('./retained-stylesheet-runtime-fixture.css?direct', import.meta.url).href;
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const declaration = f.declare('fetched-card', `${css(`@import "${url}";`)}{p | Fetched}`);
                await declaration.ready;
                const instance = document.createElement(declaration.tag); root.append(instance);
                await f.runtime.whenRenderSettled(instance);
                expect(instance.querySelector('p')?.textContent).toBe('Fetched');
                expect(getComputedStyle(instance).getPropertyValue('--default-reader').trim()).toBe('loaded');
                expect(f.runtime.diagnosticsFor(instance)).toEqual([]);
                expect(declaration.declaration.querySelectorAll('style[data-cem-declaration-style]')).toHaveLength(1);
            } finally { f.dispose(); }
        }
    },
};

export const DefaultReaderPreservesNativeByteDiagnostic: Story = {
    render: () => '<section aria-label="Default CSS reader native byte limit"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing byte limit fixture');
        const original = globalThis.fetch;
        const url = 'https://example.test/oversized-stylesheet-fixture.css';
        let cancellations = 0;
        globalThis.fetch = async (input, init) => {
            if (input !== url) return original(input, init);
            const response = new Response(new ReadableStream<Uint8Array>({
                pull(controller) { controller.enqueue(new Uint8Array(16 * 1024 * 1024 + 100)); },
                cancel() { cancellations++; },
            }), { headers: { 'content-type': 'text/css' } });
            Object.defineProperty(response, 'url', { value: url });
            return response;
        };
        try {
            for (const fallback of [false, true]) {
                const f = fixture(root, fallback);
                try {
                    const declaration = f.declare('oversized-card', `${css(`@import "${url}";`)}{p | Independent content}`);
                    await declaration.ready;
                    const instance = document.createElement(declaration.tag); root.append(instance);
                    await f.runtime.whenRenderSettled(instance);
                    expect(instance.querySelector('p')?.textContent).toBe('Independent content');
                    expect(f.runtime.diagnosticsFor(instance)).toEqual(expect.arrayContaining([expect.objectContaining({
                        code: 'cem.css.import_byte_limit', offset: 1, stylesheetUrl: document.baseURI,
                    })]));
                    expect(declaration.declaration.querySelectorAll('style[data-cem-declaration-style]')).toHaveLength(0);
                } finally { f.dispose(); }
            }
            expect(cancellations).toBe(2);
        } finally { globalThis.fetch = original; }
    },
};

export const LateEventAfterDisconnect: Story = {
    render: () => '<section aria-label="Detached CSS consumer events"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing detached-event fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const declaration = f.declare('late-event', '<slice name="loaded">before</slice><style>p { color: green; }</style><img slice="loaded" slice-event="load" slice-value="after"><p>${$loaded}</p>', 'dom');
                await declaration.ready;
                const instance = document.createElement(declaration.tag); root.append(instance);
                await f.runtime.whenRenderSettled(instance);
                const paragraph = instance.querySelector('p');
                const image = instance.querySelector('img');
                if (!paragraph || !image) throw new Error('missing event fixture output');
                expect(paragraph.textContent).toBe('before');
                instance.remove();
                image.dispatchEvent(new Event('load'));
                await f.runtime.whenRenderSettled(instance);
                expect(instance.querySelector('p')).toBe(paragraph);
                expect(paragraph.textContent).toBe('before');
                root.append(instance);
                await f.runtime.whenRenderSettled(instance);
                const restored = instance.querySelector('p');
                if (!restored) throw new Error('missing reconnected output');
                expect(getComputedStyle(restored).color).toBe('rgb(0, 128, 0)');
                expect(f.runtime.diagnosticsFor(instance)).toEqual([]);
            } finally { f.dispose(); }
        }
    },
};

export const PayloadCssReadinessAndHydration: Story = {
    render: () => '<section aria-label="Native payload CSS readiness"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing payload CSS fixture');
        for (const fallback of [false, true]) for (const mode of ['dom', 'cem-ml'] as const) {
            let gate = held(); let reads = 0;
            const f = fixture(root, fallback, async () => { reads++; return gate.promise; });
            try {
                const declaration = f.declare('payload-card', mode === 'dom' ? '<p>Ready</p>' : '{p | Ready}', mode);
                await declaration.ready;
                const instance = document.createElement(declaration.tag);
                const payload = document.createElement('template');
                payload.innerHTML = '<style>@import "./child.css";</style><style>p {--independent: yes}</style>';
                instance.append(payload); root.append(instance);
                let settled = false;
                const ready = f.runtime.whenRenderSettled(instance).then(() => { settled = true; });
                await waitFor(() => expect(reads).toBe(1));
                expect(settled).toBe(false); expect(instance.querySelector('p')).toBeNull();
                gate.resolve(response('p {color: rgb(1, 2, 3); animation: pulse 20s infinite} @keyframes pulse {from{opacity:0.5}to{opacity:1}}'));
                await ready;
                const paragraph = instance.querySelector('p');
                if (!paragraph) throw new Error('missing payload-styled content');
                expect(getComputedStyle(paragraph).color).toBe('rgb(1, 2, 3)');
                expect(getComputedStyle(paragraph).getPropertyValue('--independent').trim()).toBe('yes');
                expect(instance.querySelectorAll(':scope > style[data-cem-instance-style]')).toHaveLength(2);
                expect(declaration.declaration.querySelector('style')).toBeNull();
                const name = (paragraph.getAnimations()[0] as CSSAnimation).animationName;
                const snapshot = f.runtime.snapshotInstance(instance);
                const revision = paragraph.getAttribute('data-cem-data-revision');
                if (!revision) throw new Error('missing payload revision');
                snapshot.dataRevision = revision;
                const markup = instance.innerHTML; instance.remove();
                gate = held();
                const restored = document.createElement(declaration.tag); restored.innerHTML = markup;
                const island = restored.querySelector<HTMLTemplateElement>('template[data-cem-island="instance"]');
                const retainedParagraph = restored.querySelector('p');
                if (!island || !retainedParagraph) throw new Error('missing payload hydration DOM');
                writeDataIslandHydrationData(island, snapshot);
                root.append(restored);
                let hydrated = false;
                const hydration = f.runtime.whenRenderSettled(restored).then(() => { hydrated = true; });
                await waitFor(() => expect(reads).toBe(2));
                expect(hydrated).toBe(false);
                expect(restored.querySelector('p')).toBe(retainedParagraph);
                gate.resolve(response('p {color: rgb(1, 2, 3); animation: pulse 20s infinite} @keyframes pulse {from{opacity:0.5}to{opacity:1}}'));
                await hydration;
                expect(restored.querySelector('p')).toBe(retainedParagraph);
                expect((retainedParagraph.getAnimations()[0] as CSSAnimation).animationName).toBe(name);
                expect(restored.querySelectorAll(':scope > style[data-cem-instance-style]')).toHaveLength(2);
                expect(f.runtime.diagnosticsFor(restored)).toEqual([]);
            } finally { f.dispose(); }
        }
    },
};

export const PayloadCssDisconnectAndFailure: Story = {
    render: () => '<section aria-label="Native payload CSS cancellation"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing payload cancellation fixture');
        for (const fallback of [false, true]) {
            let gate = held(); let reads = 0;
            const signals: AbortSignal[] = [];
            const f = fixture(root, fallback, async (_request, signal) => { reads++; signals.push(signal); return gate.promise; });
            try {
                const declaration = f.declare('payload-cancel', '{p | Ready}'); await declaration.ready;
                const instance = document.createElement(declaration.tag);
                const payload = document.createElement('template');
                payload.innerHTML = '<style>@import "./child.css";</style><style>p {color:green}</style>';
                instance.append(payload); root.append(instance);
                const ready = f.runtime.whenRenderSettled(instance);
                await waitFor(() => expect(reads).toBe(1));
                instance.remove(); await ready;
                expect(signals[0].aborted).toBe(true);
                expect(instance.querySelector('p')).toBeNull();
                const stale = gate; gate = held(); root.append(instance);
                const reconnected = f.runtime.whenRenderSettled(instance);
                await waitFor(() => expect(reads).toBe(2));
                stale.resolve(response('p {color:red}'));
                gate.resolve({ ...response('not CSS'), contentType: 'text/html' });
                await reconnected;
                const paragraph = instance.querySelector('p');
                if (!paragraph) throw new Error('missing surviving payload content');
                expect(getComputedStyle(paragraph).color).toBe('rgb(0, 128, 0)');
                expect(instance.querySelectorAll(':scope > style[data-cem-instance-style]')).toHaveLength(1);
                const diagnostic = f.runtime.diagnosticsFor(instance).find(d => d.code === 'cem.css.import_content_type');
                expect(diagnostic).toMatchObject({ source: 'instance', offset: 0, length: '@import "./child.css";'.length });
                expect(diagnostic?.sourceUri).toMatch(/^urn:cem:template-style:/);
                f.parent.dispose();
                expect(instance.querySelector('style[data-cem-instance-style]')).toBeNull();
            } finally { f.dispose(); }
        }
    },
};

export const PayloadCssContextAndSourceChanges: Story = {
    render: () => '<section aria-label="Payload CSS contexts and edits"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('missing payload contexts fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const parents: HTMLElement[] = [];
                for (const variant of ['a', 'b']) {
                    const declaration = f.declare(`payload-map-${variant}`, `{module-map | {resource @specifier=icon @target="https://example.test/${variant}.svg"}}{p | Context}`);
                    await declaration.ready;
                    const parent = document.createElement(declaration.tag); root.append(parent);
                    await f.runtime.whenRenderSettled(parent); parents.push(parent);
                }
                const declaration = f.declare('payload-context', '{p | Ready}'); await declaration.ready;
                const instance = document.createElement(declaration.tag);
                const payload = document.createElement('template');
                payload.innerHTML = '<style>p {--asset:url(icon)}</style>';
                instance.append(payload); parents[0].append(instance);
                await f.runtime.whenRenderSettled(instance);
                const first = instance.querySelector('p');
                if (!first) throw new Error('missing mapped payload content');
                expect(getComputedStyle(first).getPropertyValue('--asset')).toContain('/a.svg');
                expect(instance.hasAttribute('data-cem-css-context')).toBe(false);
                const identity = f.runtime.snapshotInstance(instance).instanceId;
                parents[1].append(instance);
                await f.runtime.whenRenderSettled(instance);
                const moved = instance.querySelector('p');
                if (!moved) throw new Error('missing moved payload content');
                expect(getComputedStyle(moved).getPropertyValue('--asset')).toContain('/b.svg');
                expect(f.runtime.snapshotInstance(instance).instanceId).toBe(identity);
                const island = instance.querySelector<HTMLTemplateElement>('template[data-cem-island="instance"]');
                const source = island?.content.querySelector('style');
                if (!source) throw new Error('missing retained payload source');
                source.textContent = 'p {color:blue}';
                await waitFor(() => expect(getComputedStyle(instance.querySelector('p') as Element).color).toBe('rgb(0, 0, 255)'));
                await f.runtime.whenRenderSettled(instance);
                expect(instance.querySelectorAll(':scope > style[data-cem-instance-style]')).toHaveLength(1);
                expect(instance.querySelector('style[data-cem-instance-style]')?.textContent).not.toContain('/b.svg');
                expect(f.runtime.diagnosticsFor(instance)).toEqual([]);
            } finally { f.dispose(); }
        }
    },
};
