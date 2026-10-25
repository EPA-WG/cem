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
        expect(storybookCemRuntime().diagnosticsFor(host), JSON.stringify(storybookCemRuntime().diagnosticsFor(host))).toEqual([]);
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

export const NativeBulkPublication = meta.story({
    parameters: { docs: { description: { story: 'Browser-runner-only: one multiline native insertion publishes its final source once while preserving the native surface, focus, caret and undo. Explicit synthetic input remains observable.' } } },
    render: () => '<cem-source-editor value="initial" highlight="off"></cem-source-editor>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const host = canvasElement.querySelector('cem-source-editor') as Editor;
        await whenCemRendered(host);
        const control = editor(host), changes: string[] = [];
        host.addEventListener('input', () => changes.push(host.value));
        for (const lines of [16, 994]) {
            host.setAttribute('value', `initial ${lines}`); await whenCemRendered(host);
            changes.length = 0;
            const source = Array.from({ length: lines }, (_, index) => `"row${index}": "Café dispatch"`).join('\n');
            await userEvent.fill(control, source);
            expect(host.value).toBe(source);
            expect(changes).toEqual([source]);
            expect(editor(host)).toBe(control); expect(document.activeElement).toBe(control);
            expect([control.selectionStart, control.selectionEnd]).toEqual([source.length, source.length]);
            const markup = control.innerHTML;
            host.setAttribute('label', 'Updated status'); await whenCemRendered(host);
            expect(control.innerHTML).toBe(markup); expect(editor(host)).toBe(control);
            expect(document.activeElement).toBe(control);
            changes.length = 0;
            await userEvent.fill(control, source);
            expect(changes).toEqual([source]);
            await userEvent.keyboard('!');
            expect(host.value).toBe(`${source}!`); expect(changes.at(-1)).toBe(`${source}!`);
            await userEvent.keyboard('{Control>}z{/Control}');
            expect(host.value).toBe(source);
            await userEvent.keyboard('{Control>}y{/Control}');
            expect(host.value).toBe(`${source}!`);
        }
        changes.length = 0;
        host.value = 'Programmatic value';
        control.dispatchEvent(new InputEvent('input', { bubbles: true }));
        control.dispatchEvent(new InputEvent('input', { bubbles: true }));
        expect(changes).toEqual(['Programmatic value', 'Programmatic value']);
    },
});

export const TrailingNewlines = meta.story({
    parameters: { docs: { description: { story: 'Browser-runner-only: terminal caret placeholders must not enter source values after Enter, paste or rendering updates.' } } },
    render: () => '<cem-source-editor value="one"></cem-source-editor>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const host = canvasElement.querySelector('cem-source-editor') as Editor;
        await whenCemRendered(host);
        const control = editor(host);
        control.focus(); control.setSelectionRange(3, 3);
        await userEvent.keyboard('{Enter}');
        expect(host.value, control.innerHTML).toBe('one\n');
        await userEvent.keyboard('{Enter}');
        expect(host.value, control.innerHTML).toBe('one\n\n');
        expect(control.selectionStart).toBe(host.value.length);
        host.setAttribute('label', 'Build status changed'); await whenCemRendered(host);
        expect(host.value).toBe('one\n\n');
        expect(editor(host)).toBe(control);
        expect(control.selectionStart).toBe(host.value.length);
        await userEvent.keyboard('{Control>}z{/Control}');
        expect(host.value).not.toBe('one\n\n');
        await userEvent.keyboard('{Control>}y{/Control}');
        expect(host.value).toBe('one\n\n');
        const clipboard = document.createElement('textarea'); canvasElement.append(clipboard);
        for (const suffix of ['tail\n', 'tail\n\n', '\n', '\n\n']) {
            host.value = 'prefix'; clipboard.value = suffix;
            clipboard.focus(); clipboard.select(); await userEvent.copy();
            control.focus(); control.setSelectionRange(6, 6); await userEvent.paste();
            expect(host.value, control.innerHTML).toBe(`prefix${suffix}`);
            host.setAttribute('label', `Updated ${suffix.length}`); await whenCemRendered(host);
            expect(host.value).toBe(`prefix${suffix}`);
        }
        clipboard.remove();
        host.value = ''; control.focus(); control.setSelectionRange(0, 0);
        await userEvent.keyboard('{Enter}'); expect(host.value).toBe('\n');
        await userEvent.keyboard('{Backspace}'); expect(host.value).toBe('');
        host.value = 'literal\n\n'; host.setAttribute('label', 'Literal newlines'); await whenCemRendered(host);
        expect(host.value).toBe('literal\n\n');
    },
});

export const NativeWholeSourceReplacement = meta.story({
    parameters: { docs: { description: { story: 'Browser-runner-only: terminal source line endings remain selectable during whole-source clipboard replacement, including raw CRLF values and readonly rendering. Native undo restores the original source.' } } },
    render: () => '<cem-source-editor value="" highlight="off"></cem-source-editor>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const host = canvasElement.querySelector('cem-source-editor') as Editor;
        await whenCemRendered(host);
        const control = editor(host), clipboard = document.createElement('textarea');
        canvasElement.append(clipboard);
        try {
            for (const source of ['Café\n', 'Café\n\n', '\n', '\n\n', 'Café\r\n', 'Café\r\n\r\n']) {
                host.setAttribute('readonly', ''); host.value = source;
                await whenCemRendered(host);
                expect(host.value).toBe(source); expect(control.textContent).toBe(source);
                control.focus(); control.setSelectionRange(0, source.length);
                expect([control.selectionStart, control.selectionEnd]).toEqual([0, source.length]);
                host.setAttribute('label', `Source ${JSON.stringify(source)}`); await whenCemRendered(host);
                expect(host.value).toBe(source); expect(control.textContent).toBe(source);
                expect(editor(host)).toBe(control);
                host.removeAttribute('readonly'); await whenCemRendered(host);
                const replacement = 'Replacement Café\n';
                clipboard.value = replacement; clipboard.focus(); clipboard.select(); await userEvent.copy();
                control.focus(); await userEvent.keyboard('{Control>}a{/Control}');
                expect([control.selectionStart, control.selectionEnd]).toEqual([0, source.length]);
                await userEvent.paste();
                expect(host.value, control.innerHTML).toBe(replacement);
                expect(editor(host)).toBe(control); expect(document.activeElement).toBe(control);
                const markup = control.innerHTML;
                host.setAttribute('label', 'Replacement retained'); await whenCemRendered(host);
                expect(control.innerHTML).toBe(markup);
                await userEvent.keyboard('{Control>}z{/Control}'); expect(host.value, control.innerHTML).toBe(source);
                await userEvent.keyboard('{Control>}y{/Control}'); expect(host.value, control.innerHTML).toBe(replacement);
            }
        } finally { clipboard.remove(); }
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

const paintedRanges = (host: Element) => [...CSS.highlights.entries()]
    .filter(([name]) => name.startsWith('cem-source-')).flatMap(([, paint]) => [...paint])
    .filter(range => host.contains(range.startContainer));

export const LosslessColoring = meta.story({
    render: () => '<section aria-label="Colored source"></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `cem-colored-editor-${crypto.randomUUID()}`;
            await declare(declarationSource, tag);
            const host = document.createElement(tag) as Editor;
            host.setAttribute('content-type', 'text/html'); canvasElement.append(host);
            await runtime.whenRenderSettled(host);
            const control = editor(host), source = '\t<p title="é😀">unfinished<\r\n\n';
            host.value = source;
            const text = control.firstChild, markup = control.innerHTML;
            control.focus(); control.setSelectionRange(11, 14, 'backward');
            await waitFor(() => expect(paintedRanges(host).length).toBeGreaterThan(0));
            expect(host.value).toBe(source); expect(control.textContent).toBe(source);
            expect(control.firstChild).toBe(text); expect(control.innerHTML).toBe(markup);
            expect(control.childNodes).toHaveLength(2); expect(control.lastChild?.nodeName).toBe('BR');
            expect([control.selectionStart, control.selectionEnd, control.selectionDirection]).toEqual([11, 14, 'backward']);
            expect(host.querySelector('p')).toBeNull();
            host.setAttribute('label', 'Status update'); await runtime.whenRenderSettled(host);
            expect(editor(host)).toBe(control); expect(control.firstChild).toBe(text);
            expect(control.selectionStart).toBe(11); expect(document.activeElement).toBe(control);
            // Native child-language coloring and exact Unicode range boundaries.
            host.value = '😀é\r\n<style>.card { color: red }</style>';
            await waitFor(() => expect(paintedRanges(host).map(range => range.toString())).toContain('color'));
            for (const type of ['text/css', 'text/cem-ml']) {
                host.setAttribute('content-type', type);
                host.value = type === 'text/css' ? '\t.card { color: red; /* incomplete é😀\n\n' : '\t{p @title="é😀" | unfinished\n\n';
                const markup = control.innerHTML;
                await runtime.whenRenderSettled(host);
                await waitFor(() => expect(paintedRanges(host).length).toBeGreaterThan(0));
                expect(control.textContent).toBe(host.value); expect(control.innerHTML).toBe(markup);
                expect(control.childNodes).toHaveLength(2); expect(control.lastChild?.nodeName).toBe('BR');
            }
            host.setAttribute('content-type', 'application/json'); host.value = '{"plain":true}';
            await runtime.whenRenderSettled(host);
            await waitFor(() => expect(paintedRanges(host)).toHaveLength(0));
            expect(host.value).toBe('{"plain":true}');
            host.setAttribute('content-type', 'text/html'); host.value = '<b>Again</b>';
            await runtime.whenRenderSettled(host);
            await waitFor(() => expect(paintedRanges(host).length).toBeGreaterThan(0));
            host.setAttribute('highlight', 'off'); await runtime.whenRenderSettled(host);
            expect(paintedRanges(host)).toHaveLength(0); expect(host.value).toBe('<b>Again</b>');
            host.removeAttribute('highlight'); host.value = 'x'.repeat(262145); await runtime.whenRenderSettled(host);
            expect(control.textContent).toBe(host.value); expect(paintedRanges(host)).toHaveLength(0);
            host.remove(); scope.dispose();
        }
    },
});

export const ColoredEditingAndOwnership = meta.story({
    parameters: { docs: { description: { story: 'Browser-runner-only trusted typing, undo, paste, composition and disposal checks with native source paint.' } } },
    render: () => '<section class="cem-theme-light"><cem-source-editor content-type="text/html" value="&lt;p&gt;first&lt;/p&gt;"></cem-source-editor><cem-source-editor content-type="text/html" value="&lt;b&gt;second&lt;/b&gt;"></cem-source-editor></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const [host, other] = [...canvasElement.querySelectorAll('cem-source-editor')] as Editor[];
        await whenCemRendered(host); await whenCemRendered(other);
        await waitFor(() => expect(paintedRanges(host).length).toBeGreaterThan(0));
        await waitFor(() => expect(paintedRanges(other).length).toBeGreaterThan(0));
        const control = editor(host), originalText = control.firstChild;
        control.focus(); control.setSelectionRange(host.value.length, host.value.length);
        await userEvent.keyboard('{Enter}<b>next</b>');
        expect(host.value).toBe('<p>first</p>\n<b>next</b>');
        await waitFor(() => expect(paintedRanges(host).map(range => range.toString())).toContain('b'));
        host.setAttribute('fit', 'fill'); host.style.inlineSize = '8em'; host.style.blockSize = '5em';
        await whenCemRendered(host);
        control.scrollTop = control.scrollHeight; control.scrollLeft = control.scrollWidth;
        const scroll = [control.scrollTop, control.scrollLeft];
        const caret = control.selectionStart, nativeMarkup = control.innerHTML;
        host.setAttribute('label', 'Colored draft'); await whenCemRendered(host);
        expect(control.innerHTML).toBe(nativeMarkup); expect(control.firstChild).toBe(originalText);
        expect([control.scrollTop, control.scrollLeft]).toEqual(scroll);
        expect(control.selectionStart).toBe(caret); expect(document.activeElement).toBe(control);
        await userEvent.keyboard('{Control>}z{/Control}'); expect(host.value).not.toBe('<p>first</p>\n<b>next</b>');
        await userEvent.keyboard('{Control>}y{/Control}'); expect(host.value).toBe('<p>first</p>\n<b>next</b>');
        control.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        expect(paintedRanges(host)).toHaveLength(0); expect(paintedRanges(other).length).toBeGreaterThan(0);
        control.textContent = '<p>日本語</p>'; control.dispatchEvent(new InputEvent('input', { bubbles: true, isComposing: true }));
        host.value = '<p>Next file</p>'; host.setAttribute('source-id', 'next-resource'); await whenCemRendered(host);
        expect(control.textContent).toBe('<p>日本語</p>'); expect(paintedRanges(host)).toHaveLength(0);
        control.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
        await waitFor(() => expect(host.value).toBe('<p>Next file</p>'));
        await waitFor(() => expect(paintedRanges(host).length).toBeGreaterThan(0));
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            const section = host.parentElement!; section.className = `cem-theme-${mode}`;
            const context = document.createElement('canvas').getContext('2d')!;
            const luminance = (color: string) => {
                context.clearRect(0, 0, 1, 1); context.fillStyle = color; context.fillRect(0, 0, 1, 1);
                const rgb = [...context.getImageData(0, 0, 1, 1).data].slice(0, 3).map(channel => {
                    const value = channel / 255; return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
                });
                return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
            };
            const background = luminance(getComputedStyle(control).backgroundColor);
            for (const role of ['name', 'attribute', 'property', 'value', 'function', 'string', 'number', 'keyword', 'comment']) {
                const foreground = luminance(getComputedStyle(control, `::highlight(cem-source-${role})`).color);
                expect((Math.max(background, foreground) + 0.05) / (Math.min(background, foreground) + 0.05), `${mode} ${role}`).toBeGreaterThanOrEqual(4.5);
            }
            section.style.setProperty('--cem-palette-trust-x', 'rgb(12, 34, 56)');
            expect(getComputedStyle(control, '::highlight(cem-source-name)').color).toBe('rgb(12, 34, 56)');
            section.style.removeProperty('--cem-palette-trust-x');
        }
        host.value = '<i>old resource</i>'; host.setAttribute('source-id', 'older');
        host.value = '<strong>Current</strong>'; host.setAttribute('source-id', 'current'); await whenCemRendered(host);
        await waitFor(() => expect(paintedRanges(host).map(range => range.toString())).toContain('strong'));
        expect(paintedRanges(host).map(range => range.toString())).not.toContain('i');
        host.remove(); expect(paintedRanges(host)).toHaveLength(0);
        expect(paintedRanges(other).length).toBeGreaterThan(0);
        other.remove(); expect(paintedRanges(other)).toHaveLength(0);
    },
});
