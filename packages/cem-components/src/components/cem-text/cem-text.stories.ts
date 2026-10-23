import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime, loadCemWorkflowDependencies, assertAriaReferenceIntegrity, assertLightDomRendered, nextRenderFrame } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-text.xhtml?raw';
import '../../styles.css';
const meta = preview.meta({ component: 'cem-text', title: 'CEM Components/cem-text', loaders: [async () => { await loadCemDeclaration('cem-text', declarationSource); await loadCemWorkflowDependencies(); return {}; }] });

export const ResourceKinds = meta.story({
    render: () => '<h1><cem-text variant="caption" kind="project">Website</cem-text> · <cem-text variant="caption" kind="resource">content/article.md</cem-text> · <cem-text variant="caption" kind="output">generated/article.html</cem-text></h1>',
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-text')];
        await Promise.all(hosts.map(whenCemRendered));
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            canvasElement.className = `cem-theme-${mode}`;
            for (const [index, role] of ['project', 'file', 'output'].entries()) {
                const host = hosts[index], node = host.querySelector<HTMLElement>('[part=root]');
                if (!node) throw Error('Missing text owner');
                const probe = document.createElement('span'); probe.style.color = `var(--cem-resource-${role}-color)`; canvasElement.append(probe);
                expect(getComputedStyle(node).color).toBe(getComputedStyle(probe).color);
                probe.remove();
                expect(node.textContent).not.toBe('');
            }
        }
    },
});

export const LightDomText = meta.story({
 name: "renders text primitives as accessible light DOM",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-text variant=\"caption\">Ready</cem-text>\n            </cem-stack>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();


        await waitForPrimitive(root, 'cem-text span');
        const stack = harness.query<HTMLElement>('cem-stack');
        const text = harness.query<HTMLElement>('cem-text .cem-text');

        for (const host of Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-stack, cem-text'),
        )) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
        }

        expect(stack.querySelector('.cem-stack')?.getAttribute('data-gap')).toBe('sm');
        expect(text.textContent?.trim()).toBe('Ready');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

async function waitForPrimitive(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + 1000;

    while (Date.now() < deadline) {
        const element = root.querySelector(selector);

        if (element) {
            return element;
        }

        await nextRenderFrame();
    }

    throw new Error(`Expected primitive render output matching ${selector}`);
}

export const WorkerRetention = meta.story({
 render: () => '<div></div>',
 play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-text-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "<label>Retained field <input value=\"Initial\"></label>";
        canvasElement.append(host);
        await runtime.whenRenderSettled(host);
        const owner = host.querySelector<HTMLElement>('[part=root]');
        if (!owner) throw Error('Missing text owner');
        const authored = Array.from(owner.querySelectorAll('[slot], label')), input = host.querySelector('input');
        expect(owner).not.toBeNull();
        expect(owner.tabIndex).toBe(-1);
        expect(owner.hasAttribute('inert')).toBe(false);
        expect(owner.hasAttribute('aria-live')).toBe(false);
        expect(host.shadowRoot).toBeNull();
        if (input) { input.value = 'Edited'; input.focus(); }
        for (const value of ['', 'false']) {
            host.setAttribute('busy', value); host.setAttribute('empty', value); host.setAttribute('decorative', value);
            host.setAttribute('gap', 'lg'); host.setAttribute('columns', '2'); host.setAttribute('orientation', 'vertical');
            host.setAttribute('label', 'Updated label'); host.setAttribute('variant', 'caption');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.getAttribute('aria-busy')).toBe(null);
            if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); expect(document.activeElement).toBe(input); }
            for (const child of authored) expect(host.contains(child)).toBe(true);

            host.removeAttribute('busy'); await runtime.whenRenderSettled(host);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.hasAttribute('aria-busy')).toBe(false);
            host.removeAttribute('empty'); host.removeAttribute('decorative');
            await runtime.whenRenderSettled(host);
            expect(owner.hasAttribute('data-state')).toBe(false);
        }
        host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
        expect(host.querySelector('[part=root]')).toBe(owner);
        if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); }
        const snapshot = runtime.snapshotInstance(host);
        for (const state of ['busy', 'empty', 'decorative']) {
            expect(snapshot.slices).not.toHaveProperty(state); expect(snapshot.eventPayloads).not.toHaveProperty(state);
        }
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.hidden = true; await runtime.whenRenderSettled(host);
        expect(getComputedStyle(host).display).toBe('none');
        host.remove();

 },
});

export const FallbackRetention = meta.story({
 render: () => '<div></div>',
 play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-text-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "<label>Retained field <input value=\"Initial\"></label>";
        canvasElement.append(host);
        await runtime.whenRenderSettled(host);
        const owner = host.querySelector<HTMLElement>('[part=root]');
        if (!owner) throw Error('Missing text owner');
        const authored = Array.from(owner.querySelectorAll('[slot], label')), input = host.querySelector('input');
        expect(owner).not.toBeNull();
        expect(owner.tabIndex).toBe(-1);
        expect(owner.hasAttribute('inert')).toBe(false);
        expect(owner.hasAttribute('aria-live')).toBe(false);
        expect(host.shadowRoot).toBeNull();
        if (input) { input.value = 'Edited'; input.focus(); }
        for (const value of ['', 'false']) {
            host.setAttribute('busy', value); host.setAttribute('empty', value); host.setAttribute('decorative', value);
            host.setAttribute('gap', 'lg'); host.setAttribute('columns', '2'); host.setAttribute('orientation', 'vertical');
            host.setAttribute('label', 'Updated label'); host.setAttribute('variant', 'caption');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.getAttribute('aria-busy')).toBe(null);
            if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); expect(document.activeElement).toBe(input); }
            for (const child of authored) expect(host.contains(child)).toBe(true);

            host.removeAttribute('busy'); await runtime.whenRenderSettled(host);
            expect(owner.getAttribute('data-state')).toBe(null);
            expect(owner.hasAttribute('aria-busy')).toBe(false);
            host.removeAttribute('empty'); host.removeAttribute('decorative');
            await runtime.whenRenderSettled(host);
            expect(owner.hasAttribute('data-state')).toBe(false);
        }
        host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
        expect(host.querySelector('[part=root]')).toBe(owner);
        if (input) { expect(host.querySelector('input')).toBe(input); expect(input.value).toBe('Edited'); }
        const snapshot = runtime.snapshotInstance(host);
        for (const state of ['busy', 'empty', 'decorative']) {
            expect(snapshot.slices).not.toHaveProperty(state); expect(snapshot.eventPayloads).not.toHaveProperty(state);
        }
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.hidden = true; await runtime.whenRenderSettled(host);
        expect(getComputedStyle(host).display).toBe('none');
        host.remove();

 },
});

export const ThemeInheritance = meta.story({
 render: () => '<div class="theme-audit"><cem-text>123.45</cem-text></div>',
 play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector<HTMLElement>('.theme-audit');
        const host = container?.querySelector<HTMLElement>('cem-text');
        if (!container || !host) throw new Error('Missing theme audit fixture');
        await whenCemRendered(host);
        const owner = host.querySelector<HTMLElement>('[part=root]');
        if (!owner) throw new Error('Missing component root');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            host.setAttribute('variant', 'data'); await whenCemRendered(host);
            container.style.removeProperty('--cem-typography-data-font-variant-numeric');
            expect(getComputedStyle(owner).fontVariantNumeric).toContain('tabular-nums');
            expect(getComputedStyle(owner).fontVariantNumeric).toContain('lining-nums');
            container.style.setProperty('--cem-typography-data-font-variant-numeric', 'proportional-nums oldstyle-nums');
            expect(getComputedStyle(owner).fontVariantNumeric).toContain('proportional-nums');
            expect(getComputedStyle(owner).fontVariantNumeric).toContain('oldstyle-nums');
            host.setAttribute('variant', 'body'); await whenCemRendered(host);
            container.style.setProperty('--cem-typography-feature-optical-sizing', 'none');
            expect(getComputedStyle(owner).fontOpticalSizing).toBe('none');
        }

 },
});
