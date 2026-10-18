import { expect, within } from 'storybook/test';
import preview, { createCemStoryRuntime, loadCemDeclaration, loadCemComponent, cemComponentPlaygroundSource, storybookCemRuntime, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-workspace.xhtml?raw';

const meta = preview.meta({ component: 'cem-workspace', title: 'CEM Components/cem-workspace', loaders: [async () => { await loadCemDeclaration('cem-workspace', declarationSource); return {}; }] });

export const WorkerRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-workspace-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.innerHTML = '<nav slot="navigation">Project files</nav><label slot="editor">Current file <textarea>Original source</textarea></label><article slot="preview">Rendered output</article><section slot="tools">Diagnostics</section>';
        canvasElement.append(host); await runtime.whenRenderSettled(host);
        const editor = host.querySelector('textarea')!;
        const owner = host.querySelector('[part=root]');
        editor.value = 'Unsaved draft'; editor.focus(); editor.setSelectionRange(2, 7);
        host.setAttribute('label', 'Updated workspace'); host.setAttribute('preview-label', 'Live preview');
        await runtime.whenRenderSettled(host);
        expect(host.querySelector('[part=root]')).toBe(owner);
        expect(host.querySelector('textarea')).toBe(editor);
        expect(editor.value).toBe('Unsaved draft');
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([2, 7]);
        expect(document.activeElement).toBe(editor);
        expect(host.querySelectorAll('[part~=region]')).toHaveLength(4);
        expect(host.querySelector('[part~=preview]')?.getAttribute('aria-label')).toBe('Live preview');
        expect(host.shadowRoot).toBeNull();
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
        host.remove();
    },
});

export const FallbackRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-workspace-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.innerHTML = '<nav slot="navigation">Files</nav><label slot="editor">Current file <input value="Draft"></label><article slot="preview">Output</article><section slot="tools">Commands</section>';
        canvasElement.append(host); await runtime.whenRenderSettled(host);
        const input = host.querySelector('input');
        for (const mode of ['cem-theme-light', 'cem-theme-dark', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native']) {
            host.className = mode; host.setAttribute('editor-label', `Editor ${mode}`); await runtime.whenRenderSettled(host);
            expect(host.querySelector('input')).toBe(input);
            for (const region of host.querySelectorAll<HTMLElement>('[part~=region]')) {
                const style = getComputedStyle(region);
                expect(style.color).not.toBe(style.backgroundColor);
            }
        }
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.remove();
    },
});

export const ResponsiveRegions = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-workspace-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.innerHTML = '<nav slot="navigation">Files</nav><label slot="editor">Source <textarea>Retained draft</textarea></label><article slot="preview">Preview</article><section slot="tools">Tools</section>';
        host.style.setProperty('--cem-inset-container', '23px');
        canvasElement.append(host); await runtime.whenRenderSettled(host);
        const input = host.querySelector('textarea');
        const root = host.querySelector<HTMLElement>('[part=root]')!;
        const area = (name: string) => host.querySelector<HTMLElement>(`[part~="${name}"]`)!;
        for (const [width, columns] of [[1280, 3], [900, 2], [390, 1], [320, 1]] as const) {
            host.style.inlineSize = `${width}px`;
            expect(getComputedStyle(root).gridTemplateColumns.split(' ')).toHaveLength(columns);
            expect(root.scrollWidth).toBeLessThanOrEqual(width);
            expect(host.querySelector('textarea')).toBe(input);
            if (columns === 3) {
                expect(area('navigation').getBoundingClientRect().right).toBeLessThan(area('editor').getBoundingClientRect().left);
                expect(area('editor').getBoundingClientRect().right).toBeLessThan(area('preview').getBoundingClientRect().left);
                expect(area('tools').getBoundingClientRect().top).toBeGreaterThanOrEqual(area('editor').getBoundingClientRect().bottom);
            }
        }
        expect(getComputedStyle(host.querySelector<HTMLElement>('[part=content]')!).paddingInlineStart).toBe('23px');
        host.remove();
    },
});

export const PublicRegions = meta.story({
    render: () => '<cem-workspace label="Authoring" aria-label="Supplier workspace"><nav slot="navigation">Files</nav><span slot="editor-heading">Source <strong>view.cemt</strong></span><p slot="editor">Current file</p><p slot="preview">Output</p><p slot="tools">Diagnostics</p></cem-workspace>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-workspace')!;
        await whenCemRendered(host);
        expect(host.querySelector('[part=root]')?.getAttribute('aria-label')).toBe('Supplier workspace');
        expect(host.querySelector('[part~=editor] [part=heading] strong')?.textContent).toBe('view.cemt');
        expect(host.querySelectorAll('[part~=region]')).toHaveLength(4);
    },
});

export const FilledRegions = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-workspace-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const container = document.createElement('div');
        container.style.cssText = 'inline-size:1280px;block-size:800px';
        const host = document.createElement(tag); host.setAttribute('fit', 'fill');
        host.innerHTML = '<nav slot="navigation">Files</nav><textarea slot="editor">Draft</textarea><article slot="preview">Output</article><section slot="tools">' + '<p>Diagnostic</p>'.repeat(80) + '</section>';
        container.append(host); canvasElement.append(container); await runtime.whenRenderSettled(host);
        const root = host.querySelector<HTMLElement>('[part=root]')!;
        expect(root.getBoundingClientRect().height).toBe(800);
        const content = host.querySelector<HTMLElement>('[part~=tools] [part=content]')!;
        expect(content.scrollHeight).toBeGreaterThan(content.clientHeight);
        expect(getComputedStyle(content).overflowY).toBe('auto');
        expect(host.getBoundingClientRect().bottom).toBe(root.getBoundingClientRect().bottom);
        container.remove();
    },
});

export const PropertyControls = meta.story({
    loaders: [async () => {
        const source = await cemComponentPlaygroundSource('cem-workspace');
        await Promise.all(['cem-field', 'cem-select', 'cem-checkbox'].map(loadCemComponent));
        const document = new DOMParser().parseFromString(source, 'text/html');
        const template = document.querySelector('cem-gallery-page > template')!.content.querySelector('cem-element.properties > template') as HTMLTemplateElement;
        return { propertyMarkup: `<cem-element>${template.outerHTML}</cem-element>` };
    }],
    render: (_args, { loaded }) => String(loaded.propertyMarkup),
    play: async ({ canvasElement }) => {
        const declaration = canvasElement.querySelector<HTMLElement>('cem-element')!;
        const settle = async () => {
            await storybookCemRuntime().whenDeclarationSettled(declaration);
            await whenCemRendered(declaration.querySelector<HTMLElement>('[data-cem-anonymous-instance]')!);
            await Promise.all([...canvasElement.querySelectorAll<HTMLElement>('cem-field, cem-select, cem-checkbox, cem-workspace')].map(whenCemRendered));
        };
        await settle();
        const field = within(canvasElement).getByRole('textbox', { name: 'editor-label', exact: true }) as HTMLInputElement;
        field.value = 'Selected source'; field.dispatchEvent(new Event('input', { bubbles: true })); await settle();
        const host = canvasElement.querySelector('cem-workspace')!;
        expect(host.querySelector('[part~=editor]')?.getAttribute('aria-label')).toBe('Selected source');
        const select = canvasElement.querySelector<HTMLElement & { value: string }>('cem-select[name=fit]')!;
        select.value = 'fill'; select.dispatchEvent(new Event('change', { bubbles: true })); await settle();
        expect(host.getAttribute('fit')).toBe('fill');
        expect(storybookCemRuntime().diagnosticsFor(declaration).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
    },
});

export const BrowserLayout = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-workspace-${crypto.randomUUID()}`;
        await declare(declarationSource, tag);
        const host = document.createElement(tag);
        host.setAttribute('layout', 'browser');
        host.innerHTML = '<nav slot="navigation">Samples and files</nav><article slot="editor">Folder contents</article>';
        canvasElement.append(host);
        for (const width of [1280, 900, 390, 320]) {
            host.style.width = `${width}px`;
            await runtime.whenRenderSettled(host);
            const navigation = host.querySelector<HTMLElement>('[part~=navigation]')!;
            const content = host.querySelector<HTMLElement>('[part~=editor]')!;
            const a = navigation.getBoundingClientRect(), b = content.getBoundingClientRect();
            expect(a.width).toBeGreaterThan(0); expect(b.width).toBeGreaterThan(0);
            expect(getComputedStyle(host.querySelector('[part~=preview]')!).display).toBe('none');
            expect(getComputedStyle(host.querySelector('[part~=tools]')!).display).toBe('none');
            if (width >= 840) expect(b.left).toBeGreaterThan(a.right); else expect(b.top).toBeGreaterThan(a.bottom);
        }
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
        host.remove();
    },
});
