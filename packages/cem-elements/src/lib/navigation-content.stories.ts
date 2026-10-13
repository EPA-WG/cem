import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemStoryRuntime, cemComponentDeclarationSource } from '../../.storybook/preview.js';

const meta: Meta = { title: 'CEM Elements/Navigation And Content', tags: ['test'] };
export default meta;
type Story = StoryObj;
function required<T extends HTMLElement = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
function key(owner: HTMLElement, value: string, modifiers: KeyboardEventInit = {}): KeyboardEvent {
    const event = new KeyboardEvent('keydown', { key: value, bubbles: true, cancelable: true, ...modifiers }); owner.dispatchEvent(event); return event;
}
export const PanelOwnershipAndLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) for (const name of ['tabs', 'stepper']) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `panel-owner-${crypto.randomUUID()}`, item = name === 'tabs' ? 'cem-tab' : 'cem-step';
            const source = await declare(await cemComponentDeclarationSource(`cem-${name}`), tag);
            const fixture = document.createElement('section'); fixture.innerHTML = `<button type="button">Outside</button><${tag} label="Views"><${item} value="one" label="One"><input aria-label="Retained" value="default"></${item}><${item} value="two" label="Two"><button type="button">Panel action</button></${item}><${item} value="three" label="Three" disabled="false"><p>Unavailable</p></${item}></${tag}>`;
            canvasElement.append(fixture);
            const host = required<HTMLElement & { selectedIndex: number }>(fixture, tag);
            try {
                await runtime.whenRenderSettled(host);
                const buttons = [...host.querySelectorAll<HTMLButtonElement>(name === 'tabs' ? '[part=tab]' : '[part=header]')];
                const input = required<HTMLInputElement>(host, 'input'); input.value = 'edited';
                const events: unknown[] = []; host.addEventListener(name === 'tabs' ? 'cem-tab' : 'cem-step', event => events.push((event as CustomEvent).detail));
                expect(buttons[2].disabled).toBe(true); buttons[0].focus();
                for (const modifiers of [{ ctrlKey: true }, { altKey: true }, { metaKey: true }, { isComposing: true }, { keyCode: 229 }]) {
                    expect(key(buttons[0], 'ArrowRight', modifiers).defaultPrevented).toBe(false);
                    await runtime.whenRenderSettled(host); expect(document.activeElement).toBe(buttons[0]);
                }
                key(buttons[0], 'ArrowRight'); await runtime.whenRenderSettled(host);
                expect(document.activeElement).toBe(buttons[1]); expect(host.selectedIndex).toBe(0); expect(events).toEqual([]);
                buttons[1].click(); await runtime.whenRenderSettled(host);
                expect(events).toEqual([{ value: 'two', index: 1, previousIndex: 0 }]);
                required<HTMLButtonElement>(host, '[part=panel]:not([hidden]) button').focus();
                host.selectedIndex = 0; await runtime.whenRenderSettled(host);
                expect(document.activeElement).toBe(buttons[0]); expect(required(host, 'input')).toBe(input); expect(input.value).toBe('edited');
                const outside = required<HTMLButtonElement>(fixture, ':scope > button'); outside.focus(); host.selectedIndex = 1; await runtime.whenRenderSettled(host); expect(document.activeElement).toBe(outside);
                host.remove(); fixture.append(host); await runtime.whenRenderSettled(host);
                expect(document.activeElement).toBe(outside); expect(required(host, 'input')).toBe(input);
                buttons[0].click(); await runtime.whenRenderSettled(host); expect(events).toHaveLength(2);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { fixture.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const TreeRecoveryAndLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `hierarchy-owner-${crypto.randomUUID()}`, source = await declare(await cemComponentDeclarationSource('cem-tree'), tag);
            const fixture = document.createElement('section'); fixture.innerHTML = `<button type="button">Outside</button><${tag} label="Resources" expanded-values="parent" selection="single" selected-values="child"><cem-tree-item value="parent" label="Parent"><cem-tree-item value="child" label="Child"></cem-tree-item><cem-tree-item value="blocked" label="Blocked" disabled="false"></cem-tree-item></cem-tree-item><cem-tree-item value="next" label="Next" expandable="false" loading="false"></cem-tree-item></${tag}>`;
            canvasElement.append(fixture); const host = required(fixture, tag);
            try {
                await runtime.whenRenderSettled(host);
                const parent = required<HTMLButtonElement>(host, '[data-tree-value=parent]'), child = required<HTMLButtonElement>(host, '[data-tree-value=child]');
                const next = required<HTMLButtonElement>(host, '[data-tree-value=next]');
                expect(child.tabIndex).toBe(0); expect(next.getAttribute('aria-busy')).toBe('true'); expect(next.getAttribute('aria-expanded')).toBe('false');
                const events: string[] = []; host.addEventListener('cem-tree-toggle', () => events.push('toggle')); host.addEventListener('cem-tree-activate', () => events.push('activate'));
                child.focus(); host.setAttribute('expanded-values', ''); await runtime.whenRenderSettled(host);
                expect(document.activeElement).toBe(parent); expect(events).toEqual([]);
                const outside = required<HTMLButtonElement>(fixture, ':scope > button');
                host.setAttribute('expanded-values', 'parent'); await runtime.whenRenderSettled(host); child.focus();
                host.setAttribute('expanded-values', ''); outside.focus(); await runtime.whenRenderSettled(host); await Promise.resolve();
                expect(document.activeElement).toBe(outside);
                parent.focus(); expect(key(parent, 'n', { isComposing: true }).defaultPrevented).toBe(false); await runtime.whenRenderSettled(host); expect(document.activeElement).toBe(parent);
                host.remove(); fixture.append(host); await runtime.whenRenderSettled(host); next.click(); await runtime.whenRenderSettled(host); expect(events).toEqual(['toggle']);
                expect(host.getAttribute('selected-values')).toBe('child'); expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { fixture.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const PresenceAndNativeState: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) for (const name of ['nav', 'list', 'chip']) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `content-owner-${crypto.randomUUID()}`, source = await declare(await cemComponentDeclarationSource(`cem-${name}`), tag);
            const host = document.createElement(tag); host.setAttribute('label', 'Content');
            const mode = name === 'nav' ? 'collapsible' : name === 'list' ? 'selectable' : 'checkable';
            host.innerHTML = name === 'nav' ? '<a href="#allowed">Allowed</a><a href="#blocked" aria-disabled="true"><strong>Blocked</strong></a><section><a href="#nested" aria-disabled="true">Nested content</a></section>' : name === 'list' ? '<cem-list-option value="one" selected>One</cem-list-option><cem-list-option value="two" selected>Two</cem-list-option><cem-list-option value="blocked" disabled="false">Blocked</cem-list-option><div><cem-list-option value="nested">Nested</cem-list-option></div>' : '<strong>Visible content</strong>';
            canvasElement.append(host);
            try {
                for (const value of ['', 'false', 'true']) {
                    host.setAttribute(mode, value); if (name !== 'list') host.setAttribute(name === 'nav' ? 'expanded' : 'checked', value);
                    await runtime.whenRenderSettled(host);
                    const owner = required<HTMLButtonElement | HTMLSelectElement>(host, name === 'list' ? 'select' : 'button');
                    expect(owner).toHaveAccessibleName('Content');
                    if (name === 'list') {
                        const select = owner as HTMLSelectElement; expect(select.value).toBe('two'); expect(select.options).toHaveLength(3); expect(select.options[2].disabled).toBe(true);
                    } else expect(owner.getAttribute(name === 'nav' ? 'aria-expanded' : 'aria-pressed')).toBe('true');
                    host.setAttribute('label', 'Renamed'); await runtime.whenRenderSettled(host); expect(required(host, name === 'list' ? 'select' : 'button')).toBe(owner); host.setAttribute('label', 'Content');
                }
                if (name === 'nav') {
                    const nestedTarget = required(host, 'a[href="#blocked"] strong'); const blocked = new MouseEvent('click', { bubbles: true, cancelable: true }); nestedTarget.dispatchEvent(blocked); expect(blocked.defaultPrevented).toBe(true);
                    let suppressed: boolean | undefined;
                    required(host, 'section a').addEventListener('click', event => {
                        suppressed = event.defaultPrevented;
                        event.preventDefault(); // Keep the browser fixture on its test page.
                    }, { once: true });
                    required(host, 'section a').dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
                    expect(suppressed).toBe(false);
                } else if (name === 'list') {
                    const select = required<HTMLSelectElement>(host, 'select'); select.value = 'one'; select.dispatchEvent(new Event('change', { bubbles: true })); await runtime.whenRenderSettled(host);
                    expect(runtime.snapshotInstance(host).slices.value).toBe('one'); expect(select.options[0].getAttribute('aria-selected')).toBe('true');
                    host.setAttribute('disabled', 'false'); await runtime.whenRenderSettled(host); expect(select.disabled).toBe(true);
                } else {
                    const button = required<HTMLButtonElement>(host, 'button'); button.focus(); button.click(); await runtime.whenRenderSettled(host); expect(button.getAttribute('aria-pressed')).toBe('false'); expect(document.activeElement).toBe(button);
                    host.setAttribute('disabled', 'false'); await runtime.whenRenderSettled(host); expect(button.disabled).toBe(true);
                }
                host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host); expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const NestedPanelOwners: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) for (const name of ['tabs', 'stepper']) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `nested-panel-${crypto.randomUUID()}`, item = name === 'tabs' ? 'cem-tab' : 'cem-step';
            const source = await declare(await cemComponentDeclarationSource(`cem-${name}`), tag);
            const host = document.createElement(tag) as HTMLElement & { selectedIndex: number };
            host.innerHTML = `<${item} value="outer-one" label="Outer one"><${tag}><${item} value="inner-one" label="Inner one">First</${item}><${item} value="inner-two" label="Inner two">Second</${item}></${tag}></${item}><${item} value="outer-two" label="Outer two">Other</${item}>`;
            canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const inner = required<HTMLElement & { selectedIndex: number }>(host, tag);
                await runtime.whenRenderSettled(inner);
                const selector = name === 'tabs' ? '[part=tab]' : '[part=header]';
                const buttons = [...inner.querySelectorAll<HTMLButtonElement>(selector)];
                const targets: EventTarget[] = [];
                host.addEventListener(name === 'tabs' ? 'cem-tab' : 'cem-step', event => { if (event.target) targets.push(event.target); });
                buttons[0].focus(); key(buttons[0], 'ArrowRight'); await runtime.whenRenderSettled(inner);
                expect(document.activeElement).toBe(buttons[1]); expect(host.selectedIndex).toBe(0);
                buttons[1].click(); await runtime.whenRenderSettled(inner); await runtime.whenRenderSettled(host);
                expect(inner.selectedIndex).toBe(1); expect(host.selectedIndex).toBe(0); expect(targets).toEqual([inner]);
                expect(document.activeElement).toBe(buttons[1]); expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); source.remove(); scope.dispose(); }
        }
    },
};
