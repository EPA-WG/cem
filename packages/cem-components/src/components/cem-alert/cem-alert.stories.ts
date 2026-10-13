import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-alert.xhtml?raw';
const meta = preview.meta({ component: 'cem-alert', title: 'CEM Components/cem-alert', loaders: [async () => { await loadCemDeclaration('cem-alert', declarationSource); return {}; }] });
function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}

export const Semantics = meta.story({
    render: () => '<cem-alert></cem-alert><cem-alert label="Fallback"><strong>Projected message</strong></cem-alert>',
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-alert')];
        for (const host of hosts) await whenCemRendered(host);
        const host = hosts[0], owner = required<HTMLElement>(host, '[part=root]');
        expect(owner.textContent?.trim()).toBe('Alert');
        expect(host.shadowRoot).toBeNull(); expect(owner.tabIndex).toBe(-1);
        expect(owner.getAttribute('role')).toBe('status'); expect(owner.getAttribute('data-tone')).toBe('info');
        for (const [tone, role, expectedTone, expectedRole] of [['warning', 'alert', 'warning', 'alert'], ['danger', 'status', 'danger', 'status'], ['success', 'alert', 'success', 'alert'], ['unsupported', 'button', 'info', 'status']]) {
            host.setAttribute('tone', tone); host.setAttribute('role', role); await whenCemRendered(host);
            expect(owner.getAttribute('data-tone')).toBe(expectedTone); expect(owner.getAttribute('role')).toBe(expectedRole);
        }
        host.setAttribute('label', 'Updated message'); await whenCemRendered(host);
        expect(host.querySelector('[part=root]')).toBe(owner); expect(owner.textContent?.trim()).toBe('Updated message');
        expect(required(hosts[1], '[part=root]').textContent?.trim()).toBe('Projected message');
        expect(hosts[1].querySelector('strong')).not.toBeNull();
    },
});

export const WorkerRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-alert-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.innerHTML = '<strong>Projected message</strong><button type="button">Retained action</button>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), child = host.querySelector('strong'), button = host.querySelector('button'), svg = host.querySelector('svg');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
            if (button) button.focus();
            host.setAttribute('label', 'Updated'); host.setAttribute('tone', 'warning'); host.setAttribute('role', 'alert');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child); expect(host.querySelector('svg')).toBe(svg);
            expect(owner.getAttribute('role')).toBe('alert'); expect(owner.getAttribute('data-tone')).toBe('warning');
            if (button) expect(document.activeElement).toBe(button);
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
        const tag = `cem-alert-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.innerHTML = '<strong>Projected message</strong><button type="button">Retained action</button>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), child = host.querySelector('strong'), button = host.querySelector('button'), svg = host.querySelector('svg');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
            if (button) button.focus();
            host.setAttribute('label', 'Updated'); host.setAttribute('tone', 'warning'); host.setAttribute('role', 'alert');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child); expect(host.querySelector('svg')).toBe(svg);
            expect(owner.getAttribute('role')).toBe('alert'); expect(owner.getAttribute('data-tone')).toBe('warning');
            if (button) expect(document.activeElement).toBe(button);
            owner.click(); expect(events).toEqual([]);
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const ThemeInheritance = meta.story({
    render: () => '<div class="theme-audit"><cem-alert></cem-alert></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, '.theme-audit'), host = required<HTMLElement>(container, 'cem-alert'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]'), style = () => getComputedStyle(owner);
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-palette-trust', 'rgb(24, 36, 48)'); container.style.setProperty('--cem-inset-container', '24px'); expect(style().backgroundColor).toBe('rgb(24, 36, 48)'); expect(style().paddingTop).toBe('24px');
        }
    },
});
