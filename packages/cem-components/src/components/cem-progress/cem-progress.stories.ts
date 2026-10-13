import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-progress.xhtml?raw';
const meta = preview.meta({ component: 'cem-progress', title: 'CEM Components/cem-progress', loaders: [async () => { await loadCemDeclaration('cem-progress', declarationSource); return {}; }] });
function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}

export const NativeSemantics = meta.story({
    render: () => '<cem-progress label="Upload progress" value="40" max="100"></cem-progress>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-progress'); await whenCemRendered(host);
        const owner = required<HTMLProgressElement>(host, 'progress');
        expect(owner).toHaveAccessibleName('Upload progress'); expect(owner.value).toBe(40); expect(owner.max).toBe(100);
        expect(owner.getAttribute('value')).toBe('40'); expect(owner.getAttribute('max')).toBe('100');
        expect(host.shadowRoot).toBeNull(); expect(owner.hasAttribute('aria-live')).toBe(false);
        for (const [value, max, actualValue, actualMax] of [['120', '80', 80, 80], ['-2', '100', 0, 100], ['invalid', '0', 0, 1], ['', '', 0, 1]] as const) {
            host.setAttribute('value', value); host.setAttribute('max', max); await whenCemRendered(host);
            expect(host.querySelector('progress')).toBe(owner); expect(owner.value).toBe(actualValue); expect(owner.max).toBe(actualMax);
            expect(host.getAttribute('value')).toBe(value); expect(owner.hasAttribute('value')).toBe(true);
        }
        host.removeAttribute('value'); host.removeAttribute('max'); await whenCemRendered(host);
        expect(owner.hasAttribute('value')).toBe(false); expect(owner.position).toBe(-1); expect(owner.max).toBe(100);
        expect(owner.hasAttribute('role')).toBe(false); expect(owner.tabIndex).toBe(-1);
    },
});

export const WorkerRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-progress-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.innerHTML = ''; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), child = host.querySelector('strong'), svg = host.querySelector('svg');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('value', '30'); host.setAttribute('max', '12');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child); expect(host.querySelector('svg')).toBe(svg);
            expect((owner as HTMLProgressElement).value).toBe(12); expect((owner as HTMLProgressElement).max).toBe(12);
            owner.click(); expect(events).toEqual([]);
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const FallbackRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-progress-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.innerHTML = ''; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), child = host.querySelector('strong'), svg = host.querySelector('svg');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('value', '30'); host.setAttribute('max', '12');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child); expect(host.querySelector('svg')).toBe(svg);
            expect((owner as HTMLProgressElement).value).toBe(12); expect((owner as HTMLProgressElement).max).toBe(12);
            owner.click(); expect(events).toEqual([]);
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const ThemeInheritance = meta.story({
    render: () => '<div class="theme-audit"><cem-progress></cem-progress></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, '.theme-audit'), host = required<HTMLElement>(container, 'cem-progress'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]'), style = () => getComputedStyle(owner);
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-progress-indicator-color', 'rgb(24, 36, 48)'); expect(style().accentColor).toBe('rgb(24, 36, 48)');
        }
    },
});
