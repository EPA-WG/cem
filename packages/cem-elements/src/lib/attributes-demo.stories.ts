import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';

const meta: Meta = { title: 'CEM Elements/Attributes Demo', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const ExternalChangesAndInputPrecedence: Story = {
    render: () => {
        const root = document.createElement('section');
        const declaration = document.createElement('cem-element');
        declaration.setAttribute('tag', 'story-attributes-document');
        declaration.setAttribute('src', new URL('../../demo/attributes.html', import.meta.url).href);
        root.append(declaration, document.createElement('story-attributes-document'));
        return root;
    },
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('story-attributes-document') as HTMLElement;
        await waitFor(() => expect(host.querySelectorAll('cem-demo-element')).toHaveLength(8), { timeout: 15000 });
        await waitFor(() => expect(host.querySelector('#defaults-1')?.getAttribute('p1')).toBe('default_P1'), { timeout: 15000 });
        const changed = host.querySelector('#defaults-2') as HTMLElement;
        const external = changed.closest('cem-demo-element') as HTMLElement;
        button(external, 'set p1').click();
        await waitFor(() => expect(changed.getAttribute('p1')).toBe('changed p1'));
        button(external, 'set p2').click();
        await waitFor(() => expect(changed.getAttribute('p2')).toBe('always_p2'));
        button(external, 'set p3').click();
        await waitFor(() => expect(changed.getAttribute('p3')).toBe('changed p3'));
        button(external, 'remove p3').click();
        await waitFor(() => expect(changed.getAttribute('p3')).toBe('def_P3'));
        (external.querySelector('#p3-input') as HTMLInputElement).value = '';
        button(external, 'set p3').click();
        await waitFor(() => expect(changed.getAttribute('p3')).toBe(''));
        expect(host.querySelector('#defaults-3')?.getAttribute('p3')).toBe('qwe');

        const title = host.querySelector('#title-from-slice') as HTMLElement;
        type(title, 'Hover me');
        await waitFor(() => expect(title.title).toBe('Hover me'));
        for (const id of ['value-default', 'value-container']) {
            const value = host.querySelector(`#${id}`) as HTMLElement;
            type(value, '');
            await waitFor(() => {
                expect(value.getAttribute('v')).toBe('');
                expect(value.getAttribute('is-changed')).toBe('true');
            });
        }
        const precedence = host.querySelector('#precedence-container') as HTMLElement;
        const control = button(precedence.closest('cem-demo-element') as HTMLElement, 'Set container value');
        control.click();
        await waitFor(() => expect(precedence.getAttribute('v')).toBe('External update'));
        type(precedence, 'Own value');
        await waitFor(() => expect(precedence.getAttribute('v')).toBe('Own value'));
        control.click();
        await waitFor(() => expect(precedence.getAttribute('v')).toBe('Own value'));
        type(precedence, '');
        await waitFor(() => expect(precedence.getAttribute('v')).toBe(''));
        control.click();
        await waitFor(() => expect(precedence.getAttribute('v')).toBe(''));
    },
};

function button(root: HTMLElement, label: string): HTMLButtonElement {
    const result = Array.from(root.querySelectorAll('button')).find((candidate) => (candidate.getAttribute('aria-label') ?? candidate.textContent)?.trim() === label);
    if (!result) throw new Error(`Missing ${label} button`);
    return result;
}

function type(root: HTMLElement, value: string): void {
    const input = root.querySelector('input') as HTMLInputElement;
    input.value = value;
    input.dispatchEvent(new Event('input', { bubbles: true }));
}
