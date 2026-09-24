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
    play: async ({ canvasElement, step }) => {
        const host = canvasElement.querySelector('story-attributes-document') as HTMLElement;
        await waitFor(() => expect(host.querySelectorAll('cem-demo-element')).toHaveLength(8), { timeout: 15000 });
        await waitFor(() => expect(host.querySelector('#defaults-1')?.getAttribute('p1')).toBe('default_P1'), { timeout: 15000 });

        await step('1. attributes definition', async () => {
            await expectDefaults(instance(host, 'defaults-1'), 'default_P1', 'def_P3');
        });

        await step('1a. External attribute changes', async () => {
            const changed = instance(host, 'defaults-2');
            const external = changed.closest('cem-demo-element') as HTMLElement;
            await expectDefaults(changed, '123', 'def_P3');
            button(external, 'set p1').click();
            await expectDefaults(changed, 'changed p1', 'def_P3');
            button(external, 'set p2').click();
            await expectDefaults(changed, 'changed p1', 'def_P3');
            button(external, 'set p3').click();
            await expectDefaults(changed, 'changed p1', 'changed p3');
            button(external, 'remove p3').click();
            await expectDefaults(changed, 'changed p1', 'def_P3');
            (external.querySelector('#p3-input') as HTMLInputElement).value = '';
            button(external, 'set p3').click();
            await expectDefaults(changed, 'changed p1', '');
        });

        await step('1b. Container attribute values', async () => {
            await expectDefaults(instance(host, 'defaults-3'), '123', 'qwe');
            // The reused declaration must not share another instance's edits.
            await expectDefaults(instance(host, 'defaults-1'), 'default_P1', 'def_P3');
        });

        await step('2. attribute from slice', async () => {
            const title = instance(host, 'title-from-slice');
            await expectState(title, { title: '😃' }, ['title attribute: 😃'], '😃');
            for (const value of ['Hover me', '']) {
                type(title, value);
                await expectState(title, { title: value }, [`title attribute: ${value}`], value);
            }
        });

        await step('3. V attribute matches input value', async () => {
            const value = instance(host, 'value-default');
            await expectValue(value, 'def', false);
            for (const text of ['From input', '']) {
                type(value, text);
                await expectValue(value, text, true);
            }
        });

        await step('3a. Container value before input', async () => {
            const value = instance(host, 'value-container');
            await expectValue(value, 'V1', false);
            for (const text of ['Container replaced', '']) {
                type(value, text);
                await expectValue(value, text, true);
            }
        });

        await step('4. attribute defaults, from container, and from slice', async () => {
            const precedence = instance(host, 'precedence-default');
            await expectPrecedence(precedence, 'def', false, '');
            for (const value of ['Own default-case value', '']) {
                type(precedence, value);
                await expectPrecedence(precedence, value, true, value);
            }
        });

        await step('4a. External changes versus user input', async () => {
            const precedence = instance(host, 'precedence-container');
            const control = button(precedence.closest('cem-demo-element') as HTMLElement, 'Set container value');
            await expectPrecedence(precedence, 'From Container', false, '');
            control.click();
            await expectPrecedence(precedence, 'External update', false, '');
            for (const value of ['Own value', '']) {
                type(precedence, value);
                await expectPrecedence(precedence, value, true, value);
                control.click();
                await expectPrecedence(precedence, value, true, value);
            }
        });
    },
};

function instance(root: HTMLElement, id: string): HTMLElement {
    const result = root.querySelector<HTMLElement>(`#${id}`);
    if (!result) throw new Error(`Missing authored attributes instance ${id}`);
    return result;
}

async function expectState(
    root: HTMLElement,
    attributes: Record<string, string>,
    paragraphs: string[],
    inputValue?: string,
): Promise<void> {
    const normalize = (value: string) => value.replace(/\s+/gu, ' ').trim();
    await waitFor(() => {
        for (const [name, value] of Object.entries(attributes)) expect(root).toHaveAttribute(name, value);
        expect(Array.from(root.querySelectorAll('article.demo-card p'), element => normalize(element.textContent ?? '')))
            .toEqual(paragraphs.map(normalize));
        if (inputValue !== undefined) expect(root.querySelector('input')).toHaveValue(inputValue);
    });
}

async function expectDefaults(root: HTMLElement, p1: string, p3: string): Promise<void> {
    await expectState(root, { p1, p2: 'always_p2', p3 }, [`p1: ${p1}`, 'p2: always_p2', `p3: ${p3}`]);
}

async function expectValue(root: HTMLElement, value: string, changed: boolean): Promise<void> {
    await expectState(root, { v: value, 'is-changed': String(changed) },
        [`v: ${value}`, `is-changed: ${changed}`], value);
}

async function expectPrecedence(root: HTMLElement, value: string, hasInput: boolean, inputValue: string): Promise<void> {
    await expectState(root, { v: value },
        [`datadom.attributes.v: ${value}`, `effective value: ${value}`, `has-input: ${hasInput}`], inputValue);
}

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
