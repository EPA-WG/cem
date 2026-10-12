import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, spyOn, userEvent, waitFor, within } from 'storybook/test';

import playgroundDocument from '../demo/playground.html?raw';
import playgroundCss from '../demo/playground.css?raw';
import {
    initializePlayground,
    playgroundLink,
    readPlaygroundState,
} from './playground.js';

const meta: Meta = {
    title: 'CEM Demo Element/Playground',
    tags: ['test'],
    render: () => {
        const parsed = new DOMParser().parseFromString(playgroundDocument, 'text/html');
        const playground = parsed.querySelector('main[aria-label="CEM playground"]');
        if (!playground) throw new Error('Expected the authored playground page');
        return `<div class="playground" data-theme="cem-theme-light"><style>${playgroundCss}</style>${playground.outerHTML}</div>`;
    },
};

export default meta;
type Story = StoryObj;

export const HtmlAndCemRendering: Story = {
    play: async ({ canvasElement }) => {
        const playground = requiredPlayground(canvasElement);
        const controller = await initializePlayground(playground);
        const canvas = within(playground);
        const input = canvas.getByRole('combobox', { name: 'Input format' });
        const output = canvas.getByRole('combobox', { name: 'Output format' });

        await expect(input).toHaveValue('auto');
        await expect(output).toHaveValue('html');
        await expect(canvas.getByRole('textbox', { name: 'Title', exact: true })).toHaveValue('Hello, CEM');
        await expect(canvas.getByRole('textbox', { name: 'Description', exact: true })).not.toHaveValue('');
        await expect(requiredEditor(playground)).toHaveAttribute('contenteditable', 'plaintext-only');
        await expect(requiredEditor(playground).closest('pre')).not.toBeNull();
        await expect(canvas.getByRole('option', { name: 'Auto' })).toBeVisible();
        await expect(Array.from((input as HTMLSelectElement).options, ({ value }) => value))
            .toEqual(expect.arrayContaining(['auto', 'html', 'cem', 'json', 'yaml', 'csv', 'xml', 'css', 'markdown']));

        setSource(playground, '<article><h2>Edited HTML</h2><p>Keep &amp; display this text.</p></article>');
        await userEvent.click(canvas.getByRole('button', { name: 'Render', exact: true }));
        await waitFor(() => expect(within(requiredOutput(playground)).getByRole('heading', { name: 'Edited HTML' })).toBeVisible());
        await expect(requiredOutput(playground)).toHaveTextContent('Keep & display this text.');
        const name = requiredEditor(playground).querySelector('[data-role="syntax.name"]');
        const text = requiredEditor(playground).querySelector('[data-role="syntax.text"]');
        if (!name || !text) throw new Error('Expected colored HTML name and text tokens');
        await expect(getComputedStyle(name).fontWeight).toBe('700');
        await expect(getComputedStyle(name).color).not.toBe(getComputedStyle(text).color);
        await expect(requiredEditor(playground).textContent).toContain('<article>');
        await expect(requiredEditor(playground).querySelector('article')).toBeNull();
        await expect(requiredEditor(playground)).toHaveAttribute('contenteditable', 'plaintext-only');

        await userEvent.selectOptions(input, 'cem');
        setSource(playground, '{article @class=message | {strong | Hello from CEM-ML}}');
        await controller.render();
        await expect(requiredOutput(playground).querySelector('article.message strong')).toHaveTextContent('Hello from CEM-ML');
        await expect(requiredEditor(playground).querySelector('b')).not.toBeNull();
    },
};

export const CrossFormatOutput: Story = {
    play: async ({ canvasElement }) => {
        const playground = requiredPlayground(canvasElement);
        const controller = await initializePlayground(playground);
        const canvas = within(playground);
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Input format' }), 'json');
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Output format' }), 'yaml');
        setSource(playground, '{"name":"CEM","count":3,"enabled":true}');

        await controller.render();

        const outputCode = requiredOutput(playground).querySelector('pre code');
        await expect(outputCode).not.toBeNull();
        await expect(outputCode?.textContent).toMatch(/name:\s*["']?CEM/);
        await expect(outputCode?.textContent).toMatch(/count:\s*3/);
        await expect(outputCode?.textContent).toMatch(/enabled:\s*true/);
        await expect(outputCode?.children.length).toBeGreaterThan(0);
        const name = outputCode?.querySelector('[data-role="syntax.name"]');
        const number = outputCode?.querySelector('[data-role="syntax.number"]');
        if (!name || !number) throw new Error('Expected colored YAML name and number tokens');
        await expect(getComputedStyle(name).fontWeight).toBe('700');
        await expect(getComputedStyle(name).color).not.toBe(getComputedStyle(number).color);
        await expect(requiredEditor(playground).children.length).toBeGreaterThan(0);
        await expect(requiredEditor(playground).textContent).toContain('"name"');
        await expect(requiredOutput(playground).querySelector('article')).toBeNull();

        // Empty JSON is still a JSON document when Auto selects the parser.
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Input format' }), 'auto');
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Output format' }), 'json');
        setSource(playground, '{}');
        await controller.render();
        await expect(playground).toHaveAttribute('data-state', 'ready');
        await expect(requiredOutput(playground).querySelector('code')?.textContent?.trim()).toBe('{}');
    },
};

export const InvalidSourceDiagnostics: Story = {
    play: async ({ canvasElement }) => {
        const playground = requiredPlayground(canvasElement);
        const controller = await initializePlayground(playground);
        const canvas = within(playground);
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Input format' }), 'json');
        setSource(playground, '{"unfinished":');

        await controller.render();

        const status = playground.querySelector(':scope > [role="status"]');
        await expect(status).toHaveAttribute('data-state', 'error');
        await expect(status?.textContent?.trim()).not.toBe('');
        await expect(requiredEditor(playground).textContent).toBe('{"unfinished":');
        await expect(canvas.getByRole('button', { name: 'Render', exact: true })).toBeEnabled();
    },
};

export const ShareableState: Story = {
    play: async ({ canvasElement }) => {
        const defaults = {
            title: 'Default title', description: 'Default description',
            source: '<p>Default</p>', input: 'auto', output: 'html',
        };
        const state = {
            title: '<b>Ideas 🧬 + & #</b>',
            description: 'A café example with <em>literal markup</em>.\nA second line + & #.',
            source: '<p>Plus +, ampersand &, fragment #, café 🧬</p>\n\n<p>Second paragraph</p>',
            input: 'html',
            output: 'cem',
        };
        const url = new URL(playgroundLink('https://cem.example/demo/playground.html', state));
        await expect(readPlaygroundState(url.search, defaults)).toEqual(state);
        await expect(readPlaygroundState('', defaults)).toEqual(defaults);
        const emptyState = { ...state, title: '', description: '', source: '' };
        const emptyUrl = new URL(playgroundLink(url.href, emptyState));
        await expect(readPlaygroundState(emptyUrl.search, defaults)).toEqual(emptyState);

        const playground = requiredPlayground(canvasElement);
        await initializePlayground(playground);
        const canvas = within(playground);
        await setMetadata(playground, state.title, state.description);
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Input format' }), 'html');
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Output format' }), 'cem');
        // Browsers may leave block/BR nodes after editing colored source. The
        // copied source must retain visible line boundaries, never token HTML.
        const editor = requiredEditor(playground);
        editor.innerHTML = '<span>&lt;p&gt;One + &amp; # 🧬&lt;/p&gt;</span><div><br></div><div>&lt;p&gt;Two&lt;/p&gt;</div>';
        editor.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertParagraph' }));
        const visibleSource = editor.innerText;
        await expect(visibleSource).toContain('\n\n');

        const clipboard = spyOn(navigator.clipboard, 'writeText')
            .mockRejectedValue(new DOMException('Clipboard unavailable', 'NotAllowedError'));
        try {
            await userEvent.click(canvas.getByRole('button', { name: 'Copy Link', exact: true }));
            const link = canvas.getByRole('textbox', { name: 'Link to this source' }) as HTMLInputElement;
            await waitFor(() => expect(link).toBeVisible());
            await expect(link).toHaveAttribute('readonly');
            const restored = readPlaygroundState(new URL(link.value).search, defaults);
            await expect(restored).toEqual({ ...state, source: visibleSource });
            await expect(restored.source).not.toContain('<span>');
            await expect(clipboard).toHaveBeenCalledWith(link.value);
            const examples = canvas.getByRole('list', { name: 'Examples and copied links' });
            const entries = within(examples).getAllByRole('listitem');
            await expect(entries).toHaveLength(7);
            const appended = entries.at(-1);
            if (!appended) throw new Error('Expected the copied example at the end of the list');
            await expect(within(appended).getByRole('link', { name: state.title })).toHaveAttribute('href', link.value);
            await expect(appended.querySelector('p')?.textContent).toBe(state.description);
            await expect(appended.querySelector('b, em')).toBeNull();
        } finally {
            clipboard.mockRestore();
        }
    },
};

export const PresetExamples: Story = {
    play: async ({ canvasElement }) => {
        const playground = requiredPlayground(canvasElement);
        const controller = await initializePlayground(playground);
        const canvas = within(playground);
        const examples = canvas.getByRole('list', { name: 'Examples and copied links' });
        const links = within(examples).getAllByRole('link') as HTMLAnchorElement[];
        await expect(links).toHaveLength(6);
        await expect(links.map(link => link.textContent)).toEqual([
            'Hello, CEM', 'Hello, CEM-ML', 'Login form CEM-ML', 'Markdown cem-ml', 'YML to JSON', 'CSV to CEM-ML',
        ]);
        const defaults = { title: '', description: '', source: '', input: 'auto', output: 'html' };
        const states = links.map(link => readPlaygroundState(new URL(link.href).search, defaults));
        await expect(states.map(state => state.input)).toEqual(['html', 'cem', 'cem', 'markdown', 'yaml', 'csv']);
        await expect(states.map(state => state.output)).toEqual(['html', 'html', 'html', 'html', 'json', 'cem']);

        for (const [index, state] of states.entries()) {
            await expect(state.title).toBe(links[index].textContent);
            await expect(state.description.trim()).not.toBe('');
            await expect(state.source.trim()).not.toBe('');
            await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Input format' }), state.input);
            await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Output format' }), state.output);
            setSource(playground, state.source);
            await controller.render();
            if (state.input === 'csv') {
                await expect(playground).toHaveAttribute('data-state', 'error');
                await expect(playground.querySelector(':scope > [role="status"]'))
                    .toHaveTextContent('without a registered CSV AST export adapter');
                await expect(requiredEditor(playground)).toHaveTextContent('Orders');
                continue;
            }
            await expect(playground).toHaveAttribute('data-state', 'ready');
            if (index < 2) {
                await expect(requiredOutput(playground)).toHaveTextContent(/Hello, CEM/);
            } else if (index === 2) {
                const login = requiredOutput(playground).querySelector('form');
                await expect(login).not.toBeNull();
                await expect(login?.parentElement?.closest('form')).toBeNull();
                await expect(login?.querySelector('input[type="password"]')).not.toBeNull();
                await expect(login?.querySelector('button[type="submit"]')).not.toBeNull();
                await expect(login?.querySelectorAll('label').length).toBeGreaterThanOrEqual(2);
            } else if (index === 3) {
                const preview = requiredOutput(playground);
                await expect(preview.querySelector('h1')).toHaveTextContent('Markdown meets CEM-ML');
                await expect(preview.querySelector('strong')).toHaveTextContent('content in Markdown');
                await expect(preview.querySelector('svg title')).toHaveTextContent('CEM-ML inline SVG');
                await expect(preview.querySelector('svg path')).toHaveAttribute('stroke', 'teal');
                await expect(preview.querySelector('pre')).toBeNull();
            } else {
                const code = requiredOutput(playground).querySelector('pre code');
                await expect(code).not.toBeNull();
                await expect(code?.children.length).toBeGreaterThan(0);
                await expect(JSON.parse(code?.textContent ?? '')).toEqual({
                    name: 'CEM', version: 1, enabled: true, formats: ['YAML', 'JSON'],
                });
            }
        }
    },
};

export const CopyLinkAppendsCurrentExample: Story = {
    play: async ({ canvasElement }) => {
        const playground = requiredPlayground(canvasElement);
        await initializePlayground(playground);
        const canvas = within(playground);
        const examples = canvas.getByRole('list', { name: 'Examples and copied links' });
        const presetLinks = within(examples).getAllByRole('link') as HTMLAnchorElement[];
        const presetUrls = presetLinks.map(link => link.href);
        const state = {
            title: '  <b>My café 🧬 + & #</b>  ',
            description: '<em>A literal description</em>\nWith a second line.',
            source: '<p>Copied + & # 🧬</p>', input: 'html', output: 'cem',
        };
        await setMetadata(playground, state.title, state.description);
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Input format' }), state.input);
        await userEvent.selectOptions(canvas.getByRole('combobox', { name: 'Output format' }), state.output);
        setSource(playground, state.source);
        const clipboard = spyOn(navigator.clipboard, 'writeText').mockResolvedValue(undefined);

        try {
            await userEvent.click(canvas.getByRole('button', { name: 'Copy Link', exact: true }));
            const links = within(examples).getAllByRole('link') as HTMLAnchorElement[];
            await expect(links).toHaveLength(7);
            await expect(links.slice(0, presetUrls.length).map(link => link.href)).toEqual(presetUrls);
            const appended = links[presetUrls.length];
            const expectedUrl = playgroundLink(window.location.href, state);
            await expect(appended).toHaveTextContent(state.title.trim());
            await expect(appended).toHaveAttribute('href', expectedUrl);
            await expect(clipboard).toHaveBeenCalledWith(expectedUrl);
            await expect(readPlaygroundState(new URL(appended.href).search, state)).toEqual(state);
            await expect(appended.parentElement?.querySelector('p')?.textContent).toBe(state.description);
            await expect(appended.parentElement?.querySelector('b, em')).toBeNull();

            await setMetadata(playground, '   ', '');
            await userEvent.click(canvas.getByRole('button', { name: 'Copy Link', exact: true }));
            const updatedLinks = within(examples).getAllByRole('link') as HTMLAnchorElement[];
            await expect(updatedLinks).toHaveLength(8);
            await expect(updatedLinks[presetUrls.length].href).toBe(expectedUrl);
            await expect(updatedLinks[presetUrls.length + 1]).toHaveTextContent('Untitled example');
            await expect(updatedLinks[presetUrls.length + 1].parentElement?.querySelector('p')).toBeNull();
            await expect(clipboard).toHaveBeenLastCalledWith(updatedLinks[presetUrls.length + 1].href);
        } finally {
            clipboard.mockRestore();
        }
    },
};

function requiredPlayground(root: ParentNode): HTMLElement {
    const playground = root.querySelector<HTMLElement>('main[aria-label="CEM playground"]');
    if (!playground) throw new Error('Expected the authored playground page');
    return playground;
}

function requiredEditor(playground: HTMLElement): HTMLElement {
    const editor = within(playground).getByRole('textbox', { name: 'Source code' });
    if (!(editor instanceof HTMLElement)) throw new Error('Expected editable source');
    return editor;
}

function requiredOutput(playground: HTMLElement): HTMLElement {
    const output = playground.querySelector<HTMLElement>('cem-demo-element[aria-label="Rendered output"] > [slot="demo"]');
    if (!output) throw new Error('Expected the output region');
    return output;
}

function setSource(playground: HTMLElement, source: string): void {
    const editor = requiredEditor(playground);
    editor.textContent = source;
    editor.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText' }));
}

async function setMetadata(playground: HTMLElement, title: string, description: string): Promise<void> {
    const canvas = within(playground);
    const titleField = canvas.getByRole('textbox', { name: 'Title', exact: true });
    const descriptionField = canvas.getByRole('textbox', { name: 'Description', exact: true });
    await userEvent.clear(titleField);
    if (title) await userEvent.type(titleField, title);
    await userEvent.clear(descriptionField);
    if (description) await userEvent.type(descriptionField, description);
}
