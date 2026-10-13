import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime, loadCemWorkflowDependencies } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-stack.xhtml?raw';
import '../../styles.css';
const meta = preview.meta({ component: 'cem-stack', title: 'CEM Components/cem-stack', loaders: [async () => { await loadCemDeclaration('cem-stack', declarationSource); await loadCemWorkflowDependencies(); return {}; }] });

export const WorkerRetention = meta.story({
 render: () => '<div></div>',
 play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-stack-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "<label>Retained field <input value=\"Initial\"></label>";
        canvasElement.append(host);
        await runtime.whenRenderSettled(host);
        const owner = host.querySelector<HTMLElement>('[part=root]')!;
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
            expect(getComputedStyle(owner).display).toBe('flex'); expect(getComputedStyle(owner).flexDirection).toBe('column');
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
        const tag = `cem-stack-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('label', 'Retained content');
        host.innerHTML = "<label>Retained field <input value=\"Initial\"></label>";
        canvasElement.append(host);
        await runtime.whenRenderSettled(host);
        const owner = host.querySelector<HTMLElement>('[part=root]')!;
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
            expect(getComputedStyle(owner).display).toBe('flex'); expect(getComputedStyle(owner).flexDirection).toBe('column');
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

export const LayoutGeometry = meta.story({
 render: () => '<cem-stack><p>One</p><p>Two</p><p>Three</p><p>Four</p></cem-stack>',
 play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-stack')!;
        await whenCemRendered(host);
        const owner = host.querySelector<HTMLElement>('[part=root]')!;
        const children = Array.from(owner.children);
        expect(children).toHaveLength(4);
        for (const [gap, token] of [['sm', '--cem-gap-related'], ['md', '--cem-layout-stack-gap'], ['lg', '--cem-gap-section'], ['unsupported', '--cem-layout-stack-gap']]) {
            host.setAttribute('gap', gap); await whenCemRendered(host);
            const probe = document.createElement('div'); probe.style.width = `var(${token})`; host.append(probe);
            expect(getComputedStyle(owner).gap).toBe(getComputedStyle(probe).width); probe.remove();
        }
        expect(getComputedStyle(owner).flexDirection).toBe('column'); expect(children[1].getBoundingClientRect().top).toBeGreaterThan(children[0].getBoundingClientRect().top);
        expect(Array.from(owner.children)).toEqual(children);
        expect(owner.hasAttribute('role')).toBe(false);
        expect(owner.hasAttribute('aria-busy')).toBe(false);

 },
});

export const ThemeInheritance = meta.story({
 render: () => '<div class="theme-audit"><cem-stack><p>One</p><p>Two</p><p>Three</p><p>Four</p></cem-stack></div>',
 play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector<HTMLElement>('.theme-audit');
        const host = container?.querySelector<HTMLElement>('cem-stack');
        if (!container || !host) throw new Error('Missing theme audit fixture');
        await whenCemRendered(host);
        const owner = host.querySelector<HTMLElement>('[part=root]');
        if (!owner) throw new Error('Missing component root');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-layout-stack-gap', '29px');
            container.style.setProperty('--cem-gap-related', '13px'); container.style.setProperty('--cem-gap-section', '37px');
            for (const [gap, expected] of [['md','29px'], ['sm','13px'], ['lg','37px']]) {
                host.setAttribute('gap', gap); await whenCemRendered(host); expect(getComputedStyle(owner).gap).toBe(expected);
            }
        }

 },
});
