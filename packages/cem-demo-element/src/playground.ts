import './index.js';
import type { CemDemoElement } from './cem-demo-element.js';
import { getPlaygroundFormats, renderPlaygroundSource } from './playground-runtime.js';
import { playgroundExamples } from './playground-examples.js';

export interface PlaygroundState {
    title: string;
    description: string;
    source: string;
    input: string;
    output: string;
}

/** Query parameters carry authored text, never highlighted HTML or a serialized AST. */
export function readPlaygroundState(search: string, defaults: PlaygroundState): PlaygroundState {
    const parameters = new URLSearchParams(search);
    return {
        title: parameters.get('title') ?? defaults.title,
        description: parameters.get('description') ?? defaults.description,
        source: parameters.get('source') ?? defaults.source,
        input: parameters.get('input') ?? defaults.input,
        output: parameters.get('output') ?? defaults.output,
    };
}

export function playgroundLink(baseUrl: string, state: PlaygroundState): string {
    const url = new URL(baseUrl);
    url.searchParams.set('title', state.title);
    url.searchParams.set('description', state.description);
    url.searchParams.set('source', state.source);
    url.searchParams.set('input', state.input);
    url.searchParams.set('output', state.output);
    return url.href;
}

export async function initializePlayground(playground: HTMLElement): Promise<{ render(): Promise<void> }> {
    const form = required<HTMLFormElement>(playground, 'form[aria-label="Render settings"]');
    const sourceDemo = required<CemDemoElement>(playground, 'cem-demo-element[aria-label="Editable source"]');
    const outputDemo = required<CemDemoElement>(playground, 'cem-demo-element[aria-label="Rendered output"]');
    const title = required<HTMLInputElement>(playground, 'input[name="title"]');
    const description = required<HTMLTextAreaElement>(playground, 'textarea[name="description"]');
    const input = required<HTMLSelectElement>(playground, 'select[name="input"]');
    const output = required<HTMLSelectElement>(playground, 'select[name="output"]');
    const submit = required<HTMLButtonElement>(playground, 'button[type="submit"]');
    const copy = required<HTMLButtonElement>(playground, 'button[name="copy-link"]');
    const status = required<HTMLElement>(playground, ':scope > [role="status"]');
    const linkDetails = required<HTMLDetailsElement>(playground, '.playground-link');
    const linkField = required<HTMLInputElement>(playground, 'input[name="share-link"]');
    const examples = required<HTMLUListElement>(playground, 'ul[aria-label="Examples and copied links"]');
    const window = playground.ownerDocument.defaultView;
    if (!window) throw new Error('The playground needs a browser window.');

    function announce(message: string, state = 'ready'): void {
        status.textContent = message;
        status.dataset.state = state;
        playground.dataset.state = state;
    }

    function appendExample(state: PlaygroundState, url: string): void {
        const item = playground.ownerDocument.createElement('li');
        const link = playground.ownerDocument.createElement('a');
        link.href = url;
        link.textContent = state.title.trim() || 'Untitled example';
        item.append(link);
        if (state.description) {
            const summary = playground.ownerDocument.createElement('p');
            summary.textContent = state.description;
            item.append(summary);
        }
        examples.append(item);
    }

    try {
        await Promise.all([sourceDemo.updateComplete, outputDemo.updateComplete]);
        const code = required<HTMLElement>(sourceDemo, '[slot="text"] code');
        const preview = required<HTMLElement>(outputDemo, '[slot="demo"]');
        const formats = await getPlaygroundFormats();
        for (const format of formats.inputs) input.add(new Option(format.label, format.id));
        output.replaceChildren(...formats.outputs.map(format => new Option(format.label, format.id)));
        const defaults: PlaygroundState = {
            title: title.value, description: description.value,
            source: code.textContent ?? '', input: 'auto', output: 'html',
        };
        const state = readPlaygroundState(window.location.search, defaults);
        title.value = state.title;
        description.value = state.description;
        code.textContent = state.source;
        code.contentEditable = 'plaintext-only';
        code.setAttribute('role', 'textbox');
        code.setAttribute('aria-label', 'Source code');
        code.setAttribute('aria-multiline', 'true');
        code.setAttribute('spellcheck', 'false');
        code.setAttribute('autocapitalize', 'off');
        code.tabIndex = 0;
        for (const example of playgroundExamples) {
            appendExample(example, playgroundLink(window.location.href, example));
        }

        // Keep unsupported URL choices visible: silently selecting another parser
        // would change the meaning of a shared document.
        for (const [select, value] of [[input, state.input], [output, state.output]] as const) {
            if (!Array.from(select.options).some(option => option.value === value)) {
                select.add(new Option(`Unsupported: ${value}`, value));
            }
            select.value = value;
        }

        function currentState(): PlaygroundState {
            // Unlike textContent, innerText retains newlines created by Enter
            // (<div>/<br>) in a contenteditable code region.
            return {
                title: title.value, description: description.value,
                source: code.innerText, input: input.value, output: output.value,
            };
        }

        let rendering = false;
        async function render(): Promise<void> {
            if (rendering) return;
            rendering = true;
            const state = currentState();
            submit.disabled = true;
            input.disabled = true;
            output.disabled = true;
            code.contentEditable = 'false';
            playground.setAttribute('aria-busy', 'true');
            announce('Rendering…', 'loading');
            preview.replaceChildren();
            try {
                const result = await renderPlaygroundSource(state.source, state.input, state.output);
                // Only the engine's escaped HTML colorer output enters the editor.
                // Its text remains the source consumed by the next Render/Copy Link.
                code.innerHTML = result.sourceHtml;
                code.dataset.language = result.inputFormat;
                code.className = `cem-source-code language-${result.inputFormat}`;
                if (result.status === 'error') {
                    announce(result.diagnostics.map(item => item.message).join('\n') || 'The source could not be rendered.', 'error');
                    return;
                }
                if (state.output === 'html') {
                    preview.innerHTML = result.outputHtml;
                } else {
                    const pre = playground.ownerDocument.createElement('pre');
                    const outputCode = playground.ownerDocument.createElement('code');
                    outputCode.className = `cem-source-code language-${state.output}`;
                    outputCode.innerHTML = result.outputHtml;
                    pre.append(outputCode);
                    preview.append(pre);
                }
                const inputLabel = formats.inputs.find(format => format.id === result.inputFormat)?.label ?? result.inputFormat;
                const outputLabel = formats.outputs.find(format => format.id === state.output)?.label ?? state.output;
                const notices = result.diagnostics.filter(item => item.severity === 'warning').map(item => item.message);
                announce([`${inputLabel} → ${outputLabel}`, ...notices].join('\n'));
            } catch (error) {
                announce(error instanceof Error ? error.message : String(error), 'error');
            } finally {
                rendering = false;
                submit.disabled = false;
                input.disabled = false;
                output.disabled = false;
                code.contentEditable = 'plaintext-only';
                playground.removeAttribute('aria-busy');
            }
        }

        form.addEventListener('submit', event => {
            event.preventDefault();
            void render();
        });
        copy.addEventListener('click', async () => {
            const state = currentState();
            const url = playgroundLink(window.location.href, state);
            linkField.value = url;
            linkDetails.hidden = false;
            appendExample(state, url);
            try {
                await window.navigator.clipboard.writeText(url);
                announce('Link copied and added to the list.');
            } catch {
                linkDetails.open = true;
                linkField.focus();
                linkField.select();
                announce('Link added to the list. Copy the selected link to share the source.');
            }
        });
        code.addEventListener('input', () => {
            announce('Source changed. Render to update the output.');
            linkDetails.hidden = true;
        });
        submit.disabled = false;
        copy.disabled = false;

        if (new URLSearchParams(window.location.search).has('source')) {
            // A shared URL fills the editor without executing its HTML on load.
            preview.textContent = 'Render the source to see the result here.';
            announce('Source restored from the link. Render to preview.');
        } else {
            await render();
        }
        return { render };
    } catch (error) {
        announce(error instanceof Error ? error.message : String(error), 'error');
        throw error;
    }
}

function required<T extends Element>(parent: ParentNode, selector: string): T {
    const element = parent.querySelector<T>(selector);
    if (!element) throw new Error(`Missing playground control: ${selector}`);
    return element;
}
