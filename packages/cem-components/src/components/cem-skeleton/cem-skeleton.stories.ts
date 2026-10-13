import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-skeleton.xhtml?raw';
const meta = preview.meta({ component: 'cem-skeleton', title: 'CEM Components/cem-skeleton', loaders: [async () => { await loadCemDeclaration('cem-skeleton', declarationSource); return {}; }] });
function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}

export const Semantics = meta.story({
    render: () => '<cem-skeleton></cem-skeleton><cem-skeleton label="Fallback"><strong>Projected message</strong></cem-skeleton>',
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-skeleton')];
        for (const host of hosts) await whenCemRendered(host);
        const host = hosts[0], owner = required<HTMLElement>(host, '[part=root]');
        expect(owner.textContent?.trim()).toBe('Loading');
        expect(host.shadowRoot).toBeNull(); expect(owner.tabIndex).toBe(-1);
        expect(owner.getAttribute('aria-hidden')).toBe('true'); expect(owner.inert).toBe(true);
        expect(owner.hasAttribute('role')).toBe(false); expect(owner.hasAttribute('aria-live')).toBe(false);
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
        const tag = `cem-skeleton-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.innerHTML = '<strong>Projected message</strong><button type="button">Retained action</button>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), child = host.querySelector('strong'), svg = host.querySelector('svg');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child); expect(host.querySelector('svg')).toBe(svg);
            expect(owner.getAttribute('aria-hidden')).toBe('true'); expect(owner.inert).toBe(true);
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
        const tag = `cem-skeleton-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.innerHTML = '<strong>Projected message</strong><button type="button">Retained action</button>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), child = host.querySelector('strong'), svg = host.querySelector('svg');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-progress']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child); expect(host.querySelector('svg')).toBe(svg);
            expect(owner.getAttribute('aria-hidden')).toBe('true'); expect(owner.inert).toBe(true);
            owner.click(); expect(events).toEqual([]);
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(child);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const ThemeInheritance = meta.story({
    render: () => '<div class="theme-audit"><cem-skeleton></cem-skeleton></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, '.theme-audit'), host = required<HTMLElement>(container, 'cem-skeleton'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]'), style = () => getComputedStyle(owner);
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-palette-conservative', 'rgb(24, 36, 48)'); container.style.setProperty('--cem-control-height', '48px'); expect(style().backgroundColor).toBe('rgb(24, 36, 48)'); expect(style().minBlockSize).toBe('48px'); expect(style().animationName).toBe('none');
        }
    },
});

export const KeyboardNeutrality = meta.story({
    render: () => '<button>Before</button><cem-skeleton><button>Decorative payload</button></cem-skeleton><button>After</button>',
    parameters: { docs: { description: { story: 'Browser-runner-only: native tab order skips the entire decorative placeholder.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await whenCemRendered(required<HTMLElement>(canvasElement, 'cem-skeleton'));
        required<HTMLButtonElement>(canvasElement, ':scope > button').focus(); await userEvent.tab();
        expect(document.activeElement).toBe(canvasElement.lastElementChild);
    },
});
