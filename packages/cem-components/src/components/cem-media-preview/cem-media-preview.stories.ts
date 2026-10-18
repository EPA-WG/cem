import { expect, waitFor, within } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemComponent, cemComponentPlaygroundSource, storybookCemRuntime, whenCemRendered, createCemStoryRuntime, assertAccessibleName } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-media-preview.xhtml?raw';
const meta = preview.meta({ component: 'cem-media-preview', title: 'CEM Components/cem-media-preview', loaders: [async () => { await loadCemDeclaration('cem-media-preview', declarationSource); return {}; }] });
function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
export const Semantics = meta.story({
    render: () => '<cem-media-preview label="Landscape"><img alt="Mountain at dawn" src="data:image/svg+xml,%3Csvg xmlns=%22http://www.w3.org/2000/svg%22/%3E"><strong slot="caption">Field notes</strong></cem-media-preview>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-media-preview'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]');
        expect(host.shadowRoot).toBeNull(); expect(owner.tabIndex).toBe(-1);
        expect(owner.localName).toBe('figure'); expect(assertAccessibleName(owner, 'Landscape')).toBe('Landscape'); expect(required(host, '[part=caption]').textContent).toBe('Field notes'); expect(required<HTMLImageElement>(host, 'img').alt).toBe('Mountain at dawn'); expect(required(host, '[part=media]').querySelector('[slot=caption]')).toBeNull();
    },
});

export const WorkerRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-media-preview-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.setAttribute('label', 'Original');
        host.innerHTML = '<strong>Retained payload</strong>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), payload = required(host, 'strong');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-tab', 'cem-step', 'cem-tree-activate']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('dir', 'rtl');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            owner.click(); expect(events).toEqual([]); expect(owner.tabIndex).toBe(-1);
            expect(getComputedStyle(owner).direction).toBe('rtl');
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const FallbackRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-media-preview-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.setAttribute('label', 'Original');
        host.innerHTML = '<strong>Retained payload</strong>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), payload = required(host, 'strong');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-tab', 'cem-step', 'cem-tree-activate']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('dir', 'rtl');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            owner.click(); expect(events).toEqual([]); expect(owner.tabIndex).toBe(-1);
            expect(getComputedStyle(owner).direction).toBe('rtl');
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const ThemeInheritance = meta.story({
    render: () => '<div class="theme-audit"><cem-media-preview label="Example" initials="EX">Example</cem-media-preview></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, '.theme-audit'), host = required<HTMLElement>(container, 'cem-media-preview'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-typography-ui-font-size', '19px');
            expect(getComputedStyle(owner).fontSize).toBe('19px');
            container.style.setProperty('--cem-palette-comfort', 'rgb(24, 36, 48)'); expect(getComputedStyle(owner).backgroundColor).toBe('rgb(24, 36, 48)'); container.style.setProperty('--cem-bend-media', '13px'); expect(getComputedStyle(owner).borderTopLeftRadius).toBe('13px');
        }
    },
});

async function explicitModes(canvasElement: HTMLElement, fallback: boolean) {
    const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
    const tag = `cem-media-output-${crypto.randomUUID()}`;
    const declaration = await declare(declarationSource, tag);
    const host = document.createElement(tag);
    const attack = '<img src="https://example.test/pixel" onerror="parent.__cemPreviewAttack=true"><script>parent.__cemPreviewAttack=true</script>';
    host.setAttribute('mode', 'text'); host.setAttribute('label', 'Exact output');
    host.setAttribute('value', `  ${attack}\n\tline two  `); host.setAttribute('truncated', '');
    canvasElement.append(host);
    try {
        await runtime.whenRenderSettled(host);
        const output = required<HTMLPreElement>(host, 'pre');
        expect(output.textContent).toBe(`  ${attack}\n\tline two  `);
        expect(host.querySelector('img, script')).toBeNull();
        expect(output.tabIndex).toBe(0);
        expect(assertAccessibleName(output, 'Exact output (truncated)')).toBe('Exact output (truncated)');
        output.focus(); host.setAttribute('label', 'Updated output');
        await runtime.whenRenderSettled(host);
        expect(host.querySelector('pre')).toBe(output); expect(document.activeElement).toBe(output);
        host.setAttribute('mode', 'document'); host.setAttribute('value', attack);
        // Attempts to widen the fixed document policy are not public options.
        host.setAttribute('sandbox', 'allow-scripts allow-same-origin');
        host.setAttribute('allow', '*'); host.setAttribute('referrerpolicy', 'unsafe-url');
        await runtime.whenRenderSettled(host);
        const frame = required<HTMLIFrameElement>(host, 'iframe');
        expect(frame.getAttribute('sandbox')).toBe(''); expect(frame.getAttribute('allow')).toBe('');
        expect(frame.referrerPolicy).toBe('no-referrer'); expect(frame.title).toBe('Updated output');
        const policy = "default-src 'none'; base-uri 'none'; connect-src 'none'; font-src 'none'; form-action 'none'; frame-src 'none'; img-src 'none'; media-src 'none'; object-src 'none'; script-src 'none'; style-src 'none'; worker-src 'none'";
        expect(frame.srcdoc).toContain(policy);
        expect(frame.srcdoc.indexOf(policy)).toBeLessThan(frame.srcdoc.indexOf(attack));
        await waitFor(() => expect(frame.contentDocument).toBeNull());
        expect((window as Window & { __cemPreviewAttack?: boolean }).__cemPreviewAttack).toBeUndefined();
        const mutations: MutationRecord[] = [];
        const observer = new MutationObserver(records => mutations.push(...records));
        observer.observe(frame, { attributes: true, attributeFilter: ['srcdoc'] });
        host.setAttribute('label', 'Retained document'); await runtime.whenRenderSettled(host);
        mutations.push(...observer.takeRecords()); observer.disconnect();
        expect(host.querySelector('iframe')).toBe(frame); expect(mutations).toHaveLength(0);
        host.setAttribute('mode', 'message'); host.setAttribute('value', 'Download the exact bytes.');
        await runtime.whenRenderSettled(host);
        expect(host.querySelector('iframe, pre')).toBeNull();
        expect(required(host, '[part=message]').textContent).toBe('Download the exact bytes.');
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
    } finally { host.remove(); declaration.remove(); scope.dispose(); }
}

export const ExplicitModesWorker = meta.story({ render: () => '<div></div>', play: async ({ canvasElement }) => explicitModes(canvasElement, false) });
export const ExplicitModesFallback = meta.story({ render: () => '<div></div>', play: async ({ canvasElement }) => explicitModes(canvasElement, true) });

export const OutputThemeInheritance = meta.story({
    render: () => '<div><cem-media-preview mode="text" label="Text output" value="Exact source"></cem-media-preview></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, 'div'), host = required<HTMLElement>(container, 'cem-media-preview');
        await whenCemRendered(host);
        const output = required<HTMLElement>(host, 'pre'), figure = required<HTMLElement>(host, 'figure');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `cem-theme-${mode}`;
            expect(getComputedStyle(figure).color).not.toBe(getComputedStyle(figure).backgroundColor);
            container.style.setProperty('--cem-typography-script-font-size', '19px');
            container.style.setProperty('--cem-inset-container', '17px');
            container.style.setProperty('--cem-stroke-boundary', '3px');
            expect(getComputedStyle(output).fontSize).toBe('19px');
            expect(getComputedStyle(output).paddingInlineStart).toBe('17px');
            expect(getComputedStyle(figure).borderTopWidth).toBe('3px');
            expect(getComputedStyle(output).maxBlockSize).toBe(`${innerHeight * 0.6}px`);
        }
    },
});

export const OutputPropertyControls = meta.story({
    loaders: [async () => {
        const source = await cemComponentPlaygroundSource('cem-media-preview');
        await Promise.all(['cem-field', 'cem-radio', 'cem-textarea', 'cem-checkbox'].map(loadCemComponent));
        const document = new DOMParser().parseFromString(source, 'text/html');
        const template = document.querySelector('cem-gallery-page > template')!.content.querySelector('cem-element.properties > template') as HTMLTemplateElement;
        return { propertyMarkup: `<cem-element>${template.outerHTML}</cem-element>` };
    }],
    render: (_args, { loaded }) => String(loaded.propertyMarkup),
    play: async ({ canvasElement }) => {
        const declaration = required<HTMLElement>(canvasElement, 'cem-element');
        const settle = async () => {
            await storybookCemRuntime().whenDeclarationSettled(declaration);
            await whenCemRendered(required<HTMLElement>(declaration, '[data-cem-anonymous-instance]'));
            await Promise.all([...canvasElement.querySelectorAll<HTMLElement>('cem-field, cem-radio, cem-textarea, cem-checkbox, cem-media-preview')].map(whenCemRendered));
        };
        await settle();
        const modes = within(canvasElement).getByRole('radiogroup', { name: 'mode' });
        within(modes).getByRole('radio', { name: 'text', exact: true }).click(); await settle();
        const editor = within(canvasElement).getByRole('textbox', { name: 'value', exact: true }) as HTMLTextAreaElement;
        const text = '  <p>Literal text</p>\n';
        editor.value = text; editor.dispatchEvent(new Event('input', { bubbles: true })); await settle();
        expect(required(canvasElement, 'cem-media-preview pre').textContent).toBe(text);
        within(canvasElement).getByRole('checkbox', { name: 'truncated' }).click(); await settle();
        expect(required(canvasElement, 'cem-media-preview pre').getAttribute('aria-label')).toBe('Policy preview (truncated)');
        within(modes).getByRole('radio', { name: 'document', exact: true }).click(); await settle();
        expect(required<HTMLIFrameElement>(canvasElement, 'cem-media-preview iframe').srcdoc).toContain(text);
        within(modes).getByRole('radio', { name: 'message', exact: true }).click(); await settle();
        expect(required(canvasElement, 'cem-media-preview [part=message]').textContent).toBe(text);
        editor.value = '{p | Live property edit}'; editor.dispatchEvent(new Event('input', { bubbles: true })); await settle();
        within(modes).getByRole('radio', { name: 'live', exact: true }).click(); await settle();
        await waitFor(() => expect(required<HTMLIFrameElement>(canvasElement, 'cem-media-preview iframe').contentDocument?.querySelector('p')?.textContent).toBe('Live property edit'));
        const owner = within(canvasElement).getByRole('textbox', { name: 'source-uri', exact: true }) as HTMLInputElement;
        owner.value = 'preview://changed/edited.cem'; owner.dispatchEvent(new Event('input', { bubbles: true })); await settle();
        expect(required(canvasElement, 'cem-media-preview').getAttribute('source-uri')).toBe(owner.value);
        expect(required(canvasElement, 'cem-media-preview').getAttribute('resource-set')).toBe('{"resources":[]}');
        expect(storybookCemRuntime().diagnosticsFor(declaration).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
    },
});

export const LiveCem = meta.story({
    render: () => '<cem-media-preview mode="live" label="Interactive example" source-uri="preview://example/view.cem"></cem-media-preview>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-media-preview');
        host.setAttribute('value', '{slice @name=message | Ready}{button @type=button @slice=message @slice-event=click @slice-value="\'Clicked\'" | Try it}{p | {$datadom.slices.message}}');
        await whenCemRendered(host);
        await waitFor(() => expect(host.querySelector('[role=status]')?.textContent).toBe('Live view ready'));
        const frame = required<HTMLIFrameElement>(host, 'iframe');
        expect(frame.sandbox.value).toBe('allow-same-origin');
        expect(frame.contentDocument?.querySelector('cem-live-view')?.shadowRoot).toBeNull();
        frame.contentDocument!.querySelector('button')!.click();
        await waitFor(() => expect(frame.contentDocument?.querySelector('p')?.textContent).toBe('Clicked'));
        host.setAttribute('value', '{p | Saved edit}');
        await waitFor(() => expect(frame.contentDocument?.querySelector('p')?.textContent).toBe('Saved edit'));
        host.setAttribute('mode', 'text'); await whenCemRendered(host);
        expect(host.querySelector('iframe')).toBeNull();
    },
});

export const FilledTextPreview = meta.story({
    render: () => '<div style="block-size:400px"><cem-media-preview fit="fill" mode="text" label="Output" value="' + 'Long output&#10;'.repeat(100) + '"></cem-media-preview></div>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-media-preview'); await whenCemRendered(host);
        expect(host.getBoundingClientRect().height).toBe(400);
        const root = required<HTMLElement>(host, '[part=root]');
        expect(root.getBoundingClientRect().height).toBeLessThanOrEqual(400);
        const text = required<HTMLElement>(host, '[part=text]');
        expect(text.scrollHeight).toBeGreaterThan(text.clientHeight);
    },
});
