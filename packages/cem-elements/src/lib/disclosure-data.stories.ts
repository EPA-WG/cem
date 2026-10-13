import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemStoryRuntime, cemComponentDeclarationSource } from '../../.storybook/preview.js';
const meta: Meta = { title: 'CEM Elements/Disclosure And Data', tags: ['test'] };
export default meta;
type Story = StoryObj;
function required<T extends HTMLElement = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
export const DisclosureOwnershipAndLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `disclosure-owner-${crypto.randomUUID()}`;
            const source = await declare(await cemComponentDeclarationSource('cem-expansion'), tag);
            const host = document.createElement(tag);
            host.innerHTML = `<span slot="summary">Summary</span><input aria-label="Retained" value="default"><${tag} label="Nested"><button part="header" type="button">Payload</button></${tag}>`;
            canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const nested = required(host, tag); await runtime.whenRenderSettled(nested);
                const button = required<HTMLButtonElement>(host, '[part=header]'), panel = required(host, '[part=panel]');
                const input = required<HTMLInputElement>(host, 'input'); input.value = 'edited';
                const ids = [button.id, panel.id, button.getAttribute('aria-labelledby')];
                const events: string[] = []; for (const type of ['input', 'change', 'toggle']) host.addEventListener(type, () => events.push(type));
                for (const value of ['', 'false', 'true']) {
                    host.setAttribute('expanded', value); host.setAttribute('region', value); host.setAttribute('disabled', value);
                    await runtime.whenRenderSettled(host);
                    expect(button.disabled).toBe(true); expect(panel.hidden).toBe(false); expect(panel.getAttribute('role')).toBe('region');
                    button.click(); expect(host.hasAttribute('expanded')).toBe(true);
                    expect(required(host, 'input')).toBe(input); expect(input.value).toBe('edited');
                }
                host.removeAttribute('disabled'); host.removeAttribute('region'); await runtime.whenRenderSettled(host);
                expect(panel.getAttribute('role')).toBeNull();
                required<HTMLButtonElement>(nested, '[part=header]').click(); await runtime.whenRenderSettled(nested);
                expect(nested.hasAttribute('expanded')).toBe(true); expect(host.hasAttribute('expanded')).toBe(true);
                button.focus(); button.click(); await runtime.whenRenderSettled(host);
                expect(panel.hidden).toBe(true); expect(document.activeElement).toBe(button);
                expect([button.id, panel.id, button.getAttribute('aria-labelledby')]).toEqual(ids);
                host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
                button.click(); await runtime.whenRenderSettled(host); expect(panel.hidden).toBe(false);
                expect(required(host, '[part=header]')).toBe(button); expect(events).toEqual([]);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); source.remove(); scope.dispose(); }
        }
    },
};
export const SortCoordinationAndLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `sort-owner-${crypto.randomUUID()}`, source = await declare(await cemComponentDeclarationSource('cem-sort-header'), tag);
            const fixture = document.createElement('section');
            fixture.innerHTML = `<div role="table"><div role="row"><${tag} label="First" name="first"></${tag}><${tag} label="Peer" direction="descending" disabled="false"></${tag}></div><div role="table"><div role="row"><${tag} label="Nested" direction="ascending"></${tag}></div></div></div><div role="table"><div role="row"><${tag} label="Other" direction="ascending"></${tag}></div></div>`;
            canvasElement.append(fixture);
            const hosts = [...fixture.querySelectorAll<HTMLElement>(tag)];
            try {
                for (const host of hosts) await runtime.whenRenderSettled(host);
                const [host, peer, nested, other] = hosts, button = required<HTMLButtonElement>(host, 'button');
                const events: unknown[] = []; host.addEventListener('cem-sort', event => events.push((event as CustomEvent).detail));
                button.focus(); button.click(); for (const owner of hosts) await runtime.whenRenderSettled(owner);
                expect(peer.hasAttribute('direction')).toBe(false); expect(nested.getAttribute('direction')).toBe('ascending'); expect(other.getAttribute('direction')).toBe('ascending');
                expect(events).toEqual([{ name: 'first', direction: 'ascending', previousDirection: 'none' }]);
                expect(document.activeElement).toBe(button);
                for (const value of ['', 'false']) {
                    host.setAttribute('disabled', value); host.setAttribute('direction', 'descending'); await runtime.whenRenderSettled(host);
                    expect(button.disabled).toBe(true); button.click(); expect(events).toHaveLength(1);
                }
                host.removeAttribute('disabled'); host.remove(); fixture.append(host); await runtime.whenRenderSettled(host);
                button.click(); await runtime.whenRenderSettled(host); expect(host.hasAttribute('direction')).toBe(false); expect(events).toHaveLength(2);
                expect(required(host, 'button')).toBe(button); expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { fixture.remove(); source.remove(); scope.dispose(); }
        }
    },
};
export const PaginationPresenceAndLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `pagination-owner-${crypto.randomUUID()}`, source = await declare(await cemComponentDeclarationSource('cem-paginator'), tag);
            const host = document.createElement(tag); host.setAttribute('length', '120'); host.setAttribute('page-size', '0.5');
            canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const next = required<HTMLButtonElement>(host, '[data-page-action=next]'), range = required(host, '[part=range]');
                expect(range.textContent).toBe('1 – 50 of 120'); expect(host.getAttribute('page-size')).toBe('0.5');
                const events: unknown[] = []; host.addEventListener('cem-page', event => events.push((event as CustomEvent).detail));
                for (const value of ['', 'false']) {
                    host.setAttribute('show-first-last', value); host.setAttribute('hide-page-size', value); host.setAttribute('disabled', value);
                    await runtime.whenRenderSettled(host);
                    expect(host.querySelector('select')).toBeNull(); expect(host.querySelectorAll('[part=action]')).toHaveLength(4);
                    expect(next.disabled).toBe(true); next.click(); expect(events).toEqual([]);
                }
                host.removeAttribute('hide-page-size'); host.removeAttribute('disabled'); await runtime.whenRenderSettled(host);
                const select = required<HTMLSelectElement>(host, 'select'); expect(select.value).toBe('50');
                next.focus(); next.click(); await runtime.whenRenderSettled(host); expect(document.activeElement).toBe(next);
                next.click(); await runtime.whenRenderSettled(host); expect(range.textContent).toBe('101 – 120 of 120');
                expect(next.getAttribute('aria-disabled')).toBe('true'); expect(next.tabIndex).toBe(-1); expect(document.activeElement).toBe(next);
                let targetClicks = 0; next.addEventListener('click', () => targetClicks++); next.click(); expect(targetClicks).toBe(0); expect(events).toHaveLength(2);
                host.setAttribute('page-index', '0'); await runtime.whenRenderSettled(host); expect(events).toHaveLength(2);
                host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host); next.click(); await runtime.whenRenderSettled(host);
                expect(events).toHaveLength(3); expect(required(host, 'select')).toBe(select); expect(required(host, '[part=range]')).toBe(range);
                expect(required(host, '[data-page-action=next]')).toBe(next); expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); source.remove(); scope.dispose(); }
        }
    },
};
