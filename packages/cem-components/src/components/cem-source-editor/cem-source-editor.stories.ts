import { expect, waitFor, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, storybookCemRuntime, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-source-editor.xhtml?raw';

type Editor = HTMLElement & { value: string; selectionStart: number; selectionEnd: number; selectionDirection: string; setSelectionRange(start: number, end: number, direction?: string): void };
const meta = preview.meta({ component: 'cem-source-editor', title: 'CEM Components/cem-source-editor', loaders: [async () => { await loadCemDeclaration('cem-source-editor', declarationSource); return {}; }] });
const editor = (host: Element) => host.querySelector<Editor>('[part="control"]')!;

export const TextAndSemantics = meta.story({
    render: () => '<cem-source-editor label="Current file" aria-label="Source document" value="&lt;h1&gt;Hello&lt;/h1&gt;&#10;{p | World}"><strong slot="label">view.cemt</strong></cem-source-editor>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-source-editor') as Editor;
        await whenCemRendered(host);
        const control = editor(host);
        expect(within(host).getByRole('textbox', { name: 'Source document' })).toBe(control);
        expect(control.contentEditable).toBe('plaintext-only');
        expect(control.getAttribute('aria-multiline')).toBe('true');
        expect(control.getAttribute('spellcheck')).toBe('false');
        expect(control.innerText).toBe('<h1>Hello</h1>\n{p | World}');
        expect(host.querySelector('h1')).toBeNull();
        expect(host.querySelector('[part=label] strong')?.textContent).toBe('view.cemt');
        expect(host.shadowRoot).toBeNull();
        host.value = '<img src=x onerror="throw 1">\n🚲';
        expect(control.innerText).toBe(host.value);
        expect(host.querySelector('img')).toBeNull();
        host.setAttribute('label', 'Changed'); await whenCemRendered(host);
        expect(editor(host)).toBe(control);
        expect(control.innerText).toBe(host.value);
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    },
});

export const NativeEditing = meta.story({
    parameters: { docs: { description: { story: 'Trusted multiline input, paste and undo/redo run in the browser runner. Rendering status updates must retain the same native editable region and caret.' } } },
    render: () => '<cem-source-editor value="first"></cem-source-editor>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const host = canvasElement.querySelector('cem-source-editor') as Editor;
        await whenCemRendered(host);
        const control = editor(host);
        control.focus(); control.setSelectionRange(5, 5);
        await userEvent.keyboard('{Enter}second');
        expect(host.value).toBe('first\nsecond');
        const caret = control.selectionStart;
        host.setAttribute('label', 'Unsaved changes'); await whenCemRendered(host);
        expect(editor(host)).toBe(control);
        expect(control.selectionStart).toBe(caret);
        expect(document.activeElement).toBe(control);
        await userEvent.keyboard('{Control>}z{/Control}');
        expect(host.value).not.toBe('first\nsecond');
        await userEvent.keyboard('{Control>}y{/Control}');
        expect(host.value).toBe('first\nsecond');
        const clipboard = document.createElement('textarea');
        clipboard.value = '\n<b>plain text</b>'; canvasElement.append(clipboard);
        clipboard.focus(); clipboard.select(); await userEvent.copy();
        control.focus(); control.setSelectionRange(host.value.length, host.value.length);
        await userEvent.paste(); clipboard.remove();
        expect(host.value).toBe('first\nsecond\n<b>plain text</b>');
        expect(control.querySelector('b')).toBeNull();
    },
});

export const CompositionAndSelection = meta.story({
    render: () => '<cem-source-editor value="one&#10;🚲two"></cem-source-editor>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-source-editor') as Editor;
        await whenCemRendered(host);
        const control = editor(host);
        host.setAttribute('selection-start', '4'); host.setAttribute('selection-end', '6');
        host.setAttribute('selection-direction', 'backward'); host.setAttribute('selection-focus', '');
        host.setAttribute('selection-request', 'source-span'); await whenCemRendered(host);
        expect([control.selectionStart, control.selectionEnd, control.selectionDirection]).toEqual([4, 6, 'backward']);
        expect(document.getSelection()?.toString()).toBe('🚲');
        control.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        control.textContent = '日本語';
        control.dispatchEvent(new InputEvent('input', { bubbles: true, isComposing: true }));
        host.setAttribute('label', 'Composing'); await whenCemRendered(host);
        expect(control.textContent).toBe('日本語');
        expect(host.value).toBe('日本語');
        host.setAttribute('value', 'Next resource'); await whenCemRendered(host);
        expect(control.textContent).toBe('日本語');
        control.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
        await waitFor(() => expect(control.textContent).toBe('Next resource'));
        control.focus(); control.setSelectionRange(2, 4);
        host.setAttribute('label', 'Settled'); await whenCemRendered(host);
        expect([control.selectionStart, control.selectionEnd]).toEqual([2, 4]);
        host.setAttribute('readonly', 'false'); await whenCemRendered(host);
        expect(control.contentEditable).toBe('false');
        expect(control.getAttribute('aria-readonly')).toBe('true');
        expect(control.tabIndex).toBe(0);
        const before = host.value;
        expect(control.dispatchEvent(new InputEvent('beforeinput', { bubbles: true, cancelable: true, inputType: 'insertText', data: 'x' }))).toBe(false);
        control.textContent = 'Blocked edit'; control.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(host.value).toBe(before);
        expect(control.textContent).toBe(before);
        host.setAttribute('disabled', ''); await whenCemRendered(host);
        expect(control.getAttribute('aria-disabled')).toBe('true');
        expect(control.tabIndex).toBe(-1);
    },
});

export const ThemeAndReconnection = meta.story({
    render: () => '<section class="cem-theme-light"><cem-source-editor value="Retained source"></cem-source-editor></section>',
    play: async ({ canvasElement }) => {
        const section = canvasElement.querySelector('section')!;
        const host = section.querySelector('cem-source-editor') as Editor;
        await whenCemRendered(host);
        const control = editor(host);
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            section.className = `cem-theme-${mode}`;
            expect(getComputedStyle(control).fontFamily).toContain('monospace');
            expect(parseFloat(getComputedStyle(control).minBlockSize)).toBeGreaterThanOrEqual(48);
            section.style.setProperty('--cem-palette-comfort-text', 'rgb(12, 34, 56)');
            expect(getComputedStyle(control).color).toBe('rgb(12, 34, 56)');
            section.style.removeProperty('--cem-palette-comfort-text');
        }
        host.value = 'Unsaved draft'; host.remove(); section.append(host); await whenCemRendered(host);
        expect(host.value).toBe('Unsaved draft');
        expect(editor(host)).toBe(control);
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-source-editor-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const isolated = document.createElement(tag) as Editor;
        isolated.setAttribute('value', '<p>Standalone source</p>'); canvasElement.append(isolated);
        await runtime.whenRenderSettled(isolated);
        expect(editor(isolated).textContent).toBe('<p>Standalone source</p>');
        expect(isolated.querySelector('p')).toBeNull();
    },
});
