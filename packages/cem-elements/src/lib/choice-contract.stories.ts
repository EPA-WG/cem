import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemComponentDeclarationSource, createCemStoryRuntime } from '../../.storybook/preview.js';

export default { title: 'CEM Elements/Independent Choice Contract', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
type Choice = HTMLElement & { value: string; reportValidity(): boolean };
function required<T extends Element>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing independent choice ${selector}`);
    return node;
}

async function contract(root: HTMLElement, fallback: boolean, radio: boolean) {
    if (import.meta.env.MODE !== 'test') return; // Trusted input requires the browser runner.
    const { userEvent } = await import('vitest/browser');
    const { runtime, scope, declare } = createCemStoryRuntime(root, fallback);
    const suffix = crypto.randomUUID(), tag = `independent-choice-${suffix}`, radioTag = `independent-radio-${suffix}`;
    const declarations: HTMLElement[] = [];
    const form = document.createElement('form');
    form.className = 'cem-theme-light';
    form.style.setProperty('--cem-list-row-height', '42px');
    const host = document.createElement(tag) as Choice;
    host.setAttribute('name', 'choice'); host.setAttribute('required', ''); host.setAttribute('value', 'b');
    host.innerHTML = '<option value="a">Alpha</option><option value="blocked" disabled>Blocked</option><option value="b" selected>Beta</option>';
    try {
        declarations.push(await declare(await cemComponentDeclarationSource('cem-radio'), radioTag));
        const source = (await cemComponentDeclarationSource('cem-select'))
            .replaceAll('cem-radio', radioTag).replaceAll('cem-select__', 'independent-choice__');
        declarations.push(await declare(source, tag));
        form.append(host); root.append(form);
        const settle = async () => {
            await runtime.whenRenderSettled(host);
            await Promise.all([...host.querySelectorAll<HTMLElement>(radioTag)].map(node => runtime.whenRenderSettled(node)));
        };
        await settle();
        expect(host.querySelector('.cem-select__control,cem-radio')).toBeNull();
        const control = required<HTMLButtonElement>(host, '[part~=root] > [part~=control]');
        if (radio) {
            control.focus();
            host.setAttribute('radio-threshold', '7'); await settle();
            const selected = required<HTMLInputElement>(host, '[part~=radios] input:checked');
            await waitFor(() => expect(document.activeElement).toBe(selected));
            await userEvent.keyboard('{ArrowUp}'); await settle();
            expect(host.value).toBe('a');
            expect(document.activeElement).toBe(required(host, '[part~=radios] input:checked'));
        } else {
            await userEvent.click(control); await settle();
            expect(control.getAttribute('aria-expanded')).toBe('true');
            await userEvent.keyboard('{Home}{Enter}'); await settle();
            expect(host.value).toBe('a'); expect(document.activeElement).toBe(control);
            host.value = ''; await settle();
            expect(host.reportValidity()).toBe(false); expect(document.activeElement).toBe(control);
            host.value = 'b'; await settle();
            host.setAttribute('size', '3'); await settle();
            const listbox = required<HTMLElement>(host, '[part~=control][part~=listbox]');
            expect(listbox.style.blockSize).toBe(''); // The declaration owns the row-height token.
            expect(parseFloat(getComputedStyle(listbox).blockSize)).toBe(126);
            listbox.focus(); await userEvent.keyboard('{Home}'); await settle();
            expect(host.value).toBe('a'); expect(document.activeElement).toBe(listbox);
        }
        expect([...new FormData(form).entries()]).toEqual([['choice', 'a']]);
        form.reset(); await settle();
        expect(host.value).toBe('b');
        expect([...new FormData(form).entries()]).toEqual([['choice', 'b']]);
        if (radio) {
            const outside = document.createElement('button');
            outside.type = 'button'; outside.textContent = 'Outside destination'; form.append(outside);
            host.setAttribute('radio-threshold', '0'); await settle();
            required<HTMLButtonElement>(host, '[part~=control]').focus();
            host.setAttribute('radio-threshold', '7');
            await runtime.whenRenderSettled(host);
            outside.focus(); await settle();
            expect(document.activeElement).toBe(outside);
            // Disconnect/reconnect must retire the prior connection's focus work.
            host.remove(); form.prepend(host); outside.focus(); await settle();
            expect(document.activeElement).toBe(outside);
        }
        expect(runtime.diagnosticsFor(host)).toEqual([]);
        form.remove(); scope.dispose();
        expect(host.isConnected).toBe(false);
        expect(document.activeElement).not.toBe(host);
    } finally { form.remove(); declarations.forEach(node => node.remove()); scope.dispose(); }
}
export const PartsWorker: Story = { render: () => '<section></section>', play: ({ canvasElement }) => contract(canvasElement, false, false) };
export const PartsFallback: Story = { render: () => '<section></section>', play: ({ canvasElement }) => contract(canvasElement, true, false) };
export const NativeRadioWorker: Story = { render: () => '<section></section>', play: ({ canvasElement }) => contract(canvasElement, false, true) };
export const NativeRadioFallback: Story = { render: () => '<section></section>', play: ({ canvasElement }) => contract(canvasElement, true, true) };
