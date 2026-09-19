import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

const meta: Meta = {
    title: 'CEM Elements/External Template Demo',
    tags: ['test'],
};
export default meta;
type Story = StoryObj;

export const EmbeddedXsltFragmentBoundaries: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const response = await fetch(new URL('../../demo/html-template.xhtml', import.meta.url));
        expect(response.ok).toBe(true);
        const library = await response.text();
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-xslt-fragment-boundary',
            loadSrcDocument: async (path) => {
                let html = library;
                if (path.includes('unannotated')) html = html.replace('lang="custom-element-v0"', '');
                if (path.includes('unsupported')) html = html.replace(
                    '<h2>Embedded XSLT fruit tree</h2>',
                    '<xsl:script>never execute</xsl:script>',
                );
                return {
                    resolvedUrl: new URL(path, response.url).href,
                    resolverIdentity: 'xslt-fragment-boundary',
                    contentType: 'application/xhtml+xml',
                    body: (async function* () { yield new TextEncoder().encode(html); })(),
                };
            },
        });
        runtime.install(window);
        for (const [name, fragment] of [
            ['missing', 'no-such-fragment'],
            ['unannotated', 'embedded-xslt'],
            ['unsupported', 'embedded-xslt'],
        ]) {
            const declaration = document.createElement('cem-xslt-fragment-boundary');
            declaration.setAttribute('tag', `cem-xslt-${name}`);
            declaration.setAttribute('src', `./${name}.xhtml#${fragment}`);
            const instance = document.createElement(`cem-xslt-${name}`);
            instance.innerHTML = '<basket><fruit>🍒</fruit></basket>';
            canvasElement.append(declaration, instance);
            await runtime.whenDeclarationSettled(declaration);
            if (name === 'missing') {
                expect(runtime.diagnosticsFor(declaration).map((item) => item.code)).toContain('cem-element.src_target_missing');
                expect(instance.textContent).toBe('🍒');
                continue;
            }
            await runtime.whenRenderSettled(instance);
            if (name === 'unannotated') {
                expect(instance.querySelector('xsl\\:stylesheet')).not.toBeNull();
                expect(instance.querySelectorAll('li')).toHaveLength(1);
                expect(instance.querySelector('li')?.textContent).not.toContain('🍒');
            } else {
                await waitFor(() => expect(runtime.diagnosticsFor(declaration).map((item) => item.code))
                    .toContain('legacy_xslt.unsupported_construct'));
                expect(instance.textContent).not.toContain('never execute');
                expect(instance.querySelector('script, xsl\\:script')).toBeNull();
            }
        }
    },
};
export const AnonymousSourcesAndFallbacks: Story = {
    render: () => {
        const root = document.createElement('section');
        const declaration = document.createElement('cem-element');
        declaration.setAttribute('tag', 'story-external-template-document');
        declaration.setAttribute('src', new URL('../../demo/external-template.html', import.meta.url).href);
        root.append(declaration, document.createElement('story-external-template-document'));
        return root;
    },
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('story-external-template-document') as HTMLElement;
        await waitFor(() => expect(host.querySelectorAll('cem-demo-element')).toHaveLength(23), { timeout: 15000 });
        const anonymous = host.querySelector('cem-demo-element[legend^="2."]') as HTMLElement;
        await waitFor(() => {
            const declarations = anonymous.querySelectorAll('cem-element[src="#template2"]');
            expect(declarations).toHaveLength(2);
            for (const declaration of declarations) {
                expect(declaration.hasAttribute('data-cem-anonymous-declaration')).toBe(true);
                expect(declaration.querySelector('[data-cem-anonymous-instance]')?.textContent).toContain('construction');
            }
        }, { timeout: 15000 });
        await waitFor(() => expect(host.querySelector('cem-demo-element[legend^="3a."] svg')).not.toBeNull());
        await waitFor(() => expect(host.querySelector('dce-external-missing')?.textContent).toContain('fallback for missing image'));
        await waitFor(() => expect(host.querySelector('cem-demo-element[legend^="5a."] math')?.namespaceURI).toBe('http://www.w3.org/1998/Math/MathML'));
        await waitFor(() => expect(host.querySelector('dce-html-wave')?.textContent).toContain('👋'));
        await waitFor(() => expect(host.querySelector('dce-html-logo svg')?.namespaceURI).toBe('http://www.w3.org/2000/svg'));
        await waitFor(() => expect(host.querySelector('dce-html-formula math')?.namespaceURI).toBe('http://www.w3.org/1998/Math/MathML'));
        await waitFor(() => expect(host.querySelector('dce-missing-none')?.textContent).toContain('element with id=none is missing'));
        await waitFor(() => expect(host.querySelector('dce-embed-relative-hash')?.textContent).toContain('from embed-lib-component'), { timeout: 15000 });
        await waitFor(() => expect(host.querySelector('dce-embed-relative-file')?.textContent).toContain('🖖'), { timeout: 15000 });
        const tree = host.querySelector('dce-external-4') as HTMLElement;
        await waitFor(() => expect(tree.querySelectorAll('details').length).toBeGreaterThan(20));
        const branch = Array.from(tree.querySelectorAll('details')).find((detail) => detail.open);
        expect(branch).toBeDefined();
        branch?.querySelector('summary')?.click();
        expect(branch?.open).toBe(false);
        for (const [legend, heading, payload, details] of [
            ['7d. Anonymous external XSLT', 'XSLT XML payload tree', '🍒 from anonymous XSLT', 4],
            ['7e. Embedded XSLT fragment', 'Embedded XSLT fruit tree', '🍋', 1],
        ] as const) {
            const sample = host.querySelector(`cem-demo-element[legend="${legend}"]`) as HTMLElement;
            await waitFor(() => {
                expect(sample.querySelector('article h2')?.textContent).toBe(heading);
                expect(sample.querySelector('article')?.textContent).toContain(payload);
                expect(sample.querySelectorAll('details')).toHaveLength(details);
            }, { timeout: 15000 });
            const disclosure = sample.querySelector('details') as HTMLDetailsElement;
            expect(disclosure.open).toBe(true);
            disclosure.querySelector('summary')?.click();
            expect(disclosure.open).toBe(false);
            disclosure.querySelector('summary')?.click();
            expect(disclosure.open).toBe(true);
        }
        const fragment = host.querySelector('cem-demo-element[legend="7e. Embedded XSLT fragment"] article') as HTMLElement;
        expect(Array.from(fragment.querySelectorAll('li'), (item) => item.textContent?.trim())).toEqual(['🍒', '🍋']);
        expect(fragment.textContent).not.toContain('embedded-xsl data island tree');
    },
};
