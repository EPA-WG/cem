import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemComponent, cemComponentDeclarationSource, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-option-group.xhtml?raw';
const meta = preview.meta({ component: 'cem-option-group', title: 'CEM Components/cem-option-group', loaders: [async () => { await loadCemDeclaration('cem-option-group', declarationSource); await loadCemComponent('cem-option'); await loadCemComponent('cem-select'); return {}; }] });
function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
export const Semantics = meta.story({
    render: () => '<cem-option-group label="Example" value="one">Projected <strong>content</strong></cem-option-group>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-option-group'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]');
        expect(host.shadowRoot).toBeNull(); expect(owner.tabIndex).toBe(-1);
        expect(owner.textContent?.trim()).toBe('Projected content'); expect(owner.hasAttribute('role')).toBe(false); expect(owner.hasAttribute('tabindex')).toBe(false);
    },
});

export const WorkerRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-option-group-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.setAttribute('label', 'Original');
        host.innerHTML = '<strong>Retained payload</strong>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), payload = required(host, 'strong');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-tab', 'cem-step', 'cem-tree-activate']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('dir', 'rtl');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            owner.click(); expect(events).toEqual([]); expect(owner.tabIndex).toBe(-1);
            expect(getComputedStyle(owner).direction).toBe('rtl');
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const FallbackRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-option-group-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.setAttribute('label', 'Original');
        host.innerHTML = '<strong>Retained payload</strong>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), payload = required(host, 'strong');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-tab', 'cem-step', 'cem-tree-activate']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('dir', 'rtl');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            owner.click(); expect(events).toEqual([]); expect(owner.tabIndex).toBe(-1);
            expect(getComputedStyle(owner).direction).toBe('rtl');
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const ThemeInheritance = meta.story({
    render: () => '<div class="theme-audit"><cem-option-group label="Example" initials="EX">Example</cem-option-group></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, '.theme-audit'), host = required<HTMLElement>(container, 'cem-option-group'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-typography-ui-font-size', '19px');
            expect(getComputedStyle(owner).fontSize).toBe('19px');

        }
    },
});

export const ConsumedPayload = meta.story({
    render: () => '<cem-select size="3" label="Choice"><cem-option-group label="Group" disabled><cem-option value="one">One</cem-option></cem-option-group></cem-select>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-select'); await whenCemRendered(host);
        expect(host.querySelectorAll('[role=option]')).toHaveLength(1); expect(required(host, '[role=group]').getAttribute('aria-label')).toBe('Group'); expect(required(host, '[role=option]').getAttribute('aria-disabled')).toBe('true');
        expect(host.querySelector(':scope > cem-option-group')).toBeNull();
        const owners = [...host.querySelectorAll('button, input, [role=option], [role=treeitem]')];
        const input = host.querySelector('input:not([type=hidden])') as HTMLInputElement | null;
        if (input) { input.value = 'edited'; input.focus(); }
        host.setAttribute('label', 'Updated owner'); await whenCemRendered(host);
        expect([...host.querySelectorAll('button, input, [role=option], [role=treeitem]')]).toEqual(owners);
        if (input) { expect(input.value).toBe('edited'); expect(document.activeElement).toBe(input); }
    },
});

export const AllDisabledWorker = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `choice-list-${crypto.randomUUID()}`;
        const declaration = await declare(await cemComponentDeclarationSource('cem-select'), tag);
        const host = document.createElement(tag); host.setAttribute('size', '3'); host.setAttribute('label', 'Unavailable choices');
        host.innerHTML = '<cem-option-group label="Group" disabled><cem-option value="one">One</cem-option></cem-option-group>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[role=listbox]');
            expect(owner.getAttribute('aria-activedescendant') ?? '').toBe('');
            expect(host.querySelectorAll('[role=option]')).toHaveLength(1);
            expect(required(host, '[role=option]').getAttribute('aria-disabled')).toBe('true');
            host.setAttribute('label', 'Still unavailable'); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[role=listbox]')).toBe(owner);
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const AllDisabledFallback = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `choice-list-${crypto.randomUUID()}`;
        const declaration = await declare(await cemComponentDeclarationSource('cem-select'), tag);
        const host = document.createElement(tag); host.setAttribute('size', '3'); host.setAttribute('label', 'Unavailable choices');
        host.innerHTML = '<cem-option-group label="Group" disabled><cem-option value="one">One</cem-option></cem-option-group>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[role=listbox]');
            expect(owner.getAttribute('aria-activedescendant') ?? '').toBe('');
            expect(host.querySelectorAll('[role=option]')).toHaveLength(1);
            expect(required(host, '[role=option]').getAttribute('aria-disabled')).toBe('true');
            host.setAttribute('label', 'Still unavailable'); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[role=listbox]')).toBe(owner);
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});
