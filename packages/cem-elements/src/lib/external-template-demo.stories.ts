import { verifyExternalFilePreviews } from '../../.storybook/external-file-previews.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemRendered } from '../../.storybook/preview.js';
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
    play: async ({ canvasElement, step }) => {
        const host = canvasElement.querySelector('story-external-template-document') as HTMLElement;
        await whenCemRendered(host);
        const audited: string[] = [];
        const audit = async (legend: string, check: (sample: HTMLElement) => void, disclosure = false) => {
            audited.push(legend);
            await step(legend, async () => {
                const sample = host.querySelector<HTMLElement>(`cem-demo-element[legend="${legend}"]`);
                if (!sample) throw new Error(`Missing sample: ${legend}`);
                await waitFor(() => check(sample), { timeout: 15000 });
                if (disclosure) await toggleLiveDisclosure(sample);
            });
        };
        await audit('1. reference the template in page DOM', sample => {
            expect(Array.from(sample.querySelectorAll('dce-internal'), normalizedText))
                .toEqual(['👋 World!', 'Hello World!']);
        });
        await audit('2. without TAG, inline instantiation', sample => {
            const declarations = sample.querySelectorAll('cem-element[src="#template2"]');
            expect(declarations).toHaveLength(2);
            for (const declaration of declarations) {
                expect(declaration.hasAttribute('data-cem-anonymous-declaration')).toBe(true);
                expect(declaration.querySelectorAll('[data-cem-anonymous-instance]')).toHaveLength(1);
                expect(normalizedText(declaration.querySelector('[data-cem-anonymous-instance]'))).toBe('🏗️ construction');
            }
        });
        for (const [legend, selector] of [
            ['3. external SVG file', 'dce-external'],
            ['3a. Anonymous external SVG', 'cem-element[src="confused.svg"] > [data-cem-anonymous-instance]'],
        ]) await audit(legend, sample => {
            const instance = sample.querySelector(selector);
            expect(instance).not.toBeNull();
            expect(instance?.querySelectorAll('svg')).toHaveLength(1);
            const svg = instance?.querySelector('svg') as SVGElement;
            expect(svg.namespaceURI).toBe('http://www.w3.org/2000/svg');
            expect(Array.from(svg.querySelectorAll('use'), use => use.getAttributeNS('http://www.w3.org/1999/xlink', 'href')))
                .toEqual(['#h', '#j']);
            for (const id of ['h', 'j']) expect(svg.querySelector(`[id="${id}"]`)).not.toBeNull();
            expect(instance?.querySelector('i')).toBeNull();
        });
        await audit('3b. Missing source fallback', sample => {
            expect(normalizedText(sample.querySelector('dce-external-missing'))).toBe('fallback for missing image');
            expect(cemDiagnosticCodes(sample.querySelector('cem-element') as HTMLElement)).toContain('cem-element.src_load_failed');
            expect(sample.querySelector('svg')).toBeNull();
        });
        for (const [legend, selector, evidence] of [
            ['4. external CEM-ML template file', 'dce-external-4', [
                'Payload comment: explicit inert envelope follows', 'DCE with complete external CEMT island',
                'wrapped-payload', 'slot="heading"', 'slot=""', 'data-fruit="🍌"', 'aria-label="Fruit choice"',
                'Every element, attribute, dataset entry, and text node is data.',
            ]],
            ['4a. Live HTML payload capture', 'dce-external-4-inline', [
                'A second external-CEMT data island', 'DCE with live payload capture',
                'name="data-smile"', 'name="data-basket"', 'data-kind="live-payload"', '👼', '🍒',
            ]],
            ['4b. CEM-ML source payload', 'dce-external-4-cem-ml', [
                'content-type="text/cem-ml"', 'schema="https://cem.dev/ns/cem-ml/1"',
                '{payload:item @name=fruit @slot=default | Banana from CEM-ML payload source}',
            ]],
        ] as const) await audit(legend, sample => {
            const tree = sample.querySelector(selector);
            expect(normalizedText(tree?.querySelector('h2') ?? null)).toBe('External CEMT data-island transformation');
            for (const value of [
                'template[data-cem-island="instance"]', 'cem-island:context-root', 'cem-hydration:data',
                'cem-attributes:attributes', 'cem-dataset:dataset', 'cem-payload:payload', 'cem-slices:slices',
                'cem-resources:resources', 'cem-form:form-state', 'cem-validation:validation-state', 'cem-events:event-state',
                ...evidence,
            ]) expect(tree?.textContent).toContain(value);
            expect(tree?.querySelectorAll('details').length).toBeGreaterThan(20);
        }, true);
        for (const [legend, selector] of [
            ['5. external HTML template', 'dce-external-5'],
            ['5a. Anonymous external HTML', '#dce-external-5-inline > [data-cem-anonymous-instance]'],
        ]) await audit(legend, sample => {
            const instance = sample.querySelector(selector);
            expect(instance).not.toBeNull();
            expect(normalizedText(instance?.querySelector('#wave') ?? null)).toBe('👋');
            expect(normalizedText(instance?.querySelector('#ok') ?? null)).toBe('👌');
            expect(instance?.querySelectorAll('svg')).toHaveLength(1);
            expect(instance?.querySelector('svg')?.namespaceURI).toBe('http://www.w3.org/2000/svg');
            expect(instance?.querySelectorAll('math')).toHaveLength(1);
            expect(instance?.querySelector('math')?.namespaceURI).toBe('http://www.w3.org/1998/Math/MathML');
            expect(instance?.querySelector('script, i')).toBeNull();
        });
        await audit('6. HTML, SVG by ID within external file', sample => {
            const instance = sample.querySelector('dce-html-wave');
            expect(normalizedText(instance)).toBe('👋');
            expect(instance?.querySelectorAll('b')).toHaveLength(1);
            expect(instance?.querySelector('svg, math, #ok, i, script')).toBeNull();
        });
        for (const [legend, hostTag, tag, namespace] of [
            ['6a. SVG fragment by ID', 'dce-html-logo', 'svg', 'http://www.w3.org/2000/svg'],
            ['6b. MathML fragment by ID', 'dce-html-formula', 'math', 'http://www.w3.org/1998/Math/MathML'],
        ]) await audit(legend, sample => {
            const instance = sample.querySelector(hostTag);
            expect(instance?.querySelectorAll('svg, math')).toHaveLength(1);
            expect(instance?.querySelector(tag)?.namespaceURI).toBe(namespace);
            expect(instance?.querySelector('#wave, #ok, i, script')).toBeNull();
        });
        for (const [legend, heading, root, name, code, payload] of [
            ['7a. external CEM-ML data-island tree template', 'CEM-ML data island tree', 'cem-elements', 'alpha', 'a1', 'Leaf text from cem-elements data island'],
            ['7b. External XSLT XML payload tree', 'XSLT XML payload tree', 'cem-elements-xslt', 'beta', 'b1', 'Leaf text from cem-elements XSLT data island'],
        ]) await audit(legend, sample => expectPayloadTree(sample, heading, root, name, code, payload), true);
        await audit('7c. Missing fragment fallback', sample => {
            expect(normalizedText(sample.querySelector('dce-missing-none'))).toBe('element with id=none is missing in template');
            expect(cemDiagnosticCodes(sample.querySelector('cem-element') as HTMLElement)).toContain('cem-element.src_target_missing');
            expect(sample.querySelector('svg, math, #wave, #ok')).toBeNull();
        });
        await audit('7d. Anonymous external XSLT', sample => {
            expect(sample.querySelector('cem-element')?.hasAttribute('data-cem-anonymous-declaration')).toBe(true);
            expectPayloadTree(sample, 'XSLT XML payload tree', 'anonymous-xslt', 'fruit', 'cherry', '🍒 from anonymous XSLT');
        }, true);
        await audit('7e. Embedded XSLT fragment', sample => {
            expect(normalizedText(sample.querySelector('article h2'))).toBe('Embedded XSLT fruit tree');
            expect(sample.querySelectorAll('details')).toHaveLength(1);
            expect(normalizedText(sample.querySelector('summary'))).toBe('basket');
            expect(Array.from(sample.querySelectorAll('li'), normalizedText)).toEqual(['🍒', '🍋']);
            expect(sample.querySelector('svg, math, #wave, #ok, script, xsl\\:stylesheet')).toBeNull();
            expect(sample.querySelector('article')?.textContent).not.toContain('embedded-xsl data island tree');
        }, true);
        await audit('8. external file with embedding of another external DCE', sample => {
            expect(normalizedText(sample.querySelector('dce-embed-1 h4'))).toBe('embed-1.html');
            expect(normalizedText(sample.querySelector('dce-embed-1 [data-cem-anonymous-instance]'))).toBe('🖖');
        });
        const demoUrl = new URL('../../demo/external-template.html', import.meta.url);
        await audit('9. external file with invoking of relative template as hash by enclosed custom-element', sample => {
            const instance = sample.querySelector('dce-embed-relative-hash');
            expect(instance?.textContent).toContain('👌 from embed-relative-hash invoking');
            expect(normalizedText(instance?.querySelector('dce-embed-lib-component') ?? null)).toBe('👋 from embed-lib-component');
            expect(instance?.querySelector('a')?.href).toBe(new URL('./lib-dir/embed-lib.html#embed-lib-component', demoUrl).href);
            const image = instance?.querySelector('img');
            expect(image?.src).toBe(new URL('./lib-dir/Smiley.svg', demoUrl).href);
            expect(image?.complete && image.naturalWidth > 0).toBe(true);
        });
        await audit('10. external file with invoking of template in another relative path file by enclosed custom-element', sample => {
            const instance = sample.querySelector('dce-embed-relative-file');
            expect(instance?.textContent).toContain('👍 from embed-relative-file invoking');
            expect(instance?.querySelector('a')?.href).toBe(new URL('./embed-1.html', demoUrl).href);
            expect(normalizedText(instance?.querySelector('dce-embed-lib-file h4') ?? null)).toBe('embed-1.html');
            expect(normalizedText(instance?.querySelector('dce-embed-lib-file [data-cem-anonymous-instance]') ?? null)).toBe('🖖');
        });
        for (const [legend, file] of [
            ['embed-1.html external file', 'embed-1.html'],
            ['embed-lib.html with multiple templates', 'embed-lib.html'],
        ]) {
            audited.push(legend);
            await step(legend, () => verifyExternalFilePreviews(host, demoUrl, [file]));
        }
        expect(Array.from(host.querySelectorAll('cem-demo-element[legend]'), sample => sample.getAttribute('legend'))).toEqual(audited);
        expect(audited).toHaveLength(23);
    },
};

function normalizedText(element: Element | null): string {
    return element?.textContent?.replace(/\s+/gu, ' ').trim() ?? '';
}

function expectPayloadTree(sample: HTMLElement, heading: string, root: string, name: string, code: string, payload: string): void {
    expect(normalizedText(sample.querySelector('article h2'))).toBe(heading);
    expect(sample.querySelectorAll('details')).toHaveLength(4);
    expect(Array.from(sample.querySelectorAll('summary'), summary =>
        Array.from(summary.querySelectorAll('b, code'), normalizedText))).toEqual([
        ['catalog', `data-root="${root}"`], ['section', 'data-level="1"', `name="${name}"`],
        ['item', 'data-level="2"', `code="${code}"`], ['leaf', 'data-level="3"'],
    ]);
    expect(normalizedText(sample.querySelector('article p'))).toBe(payload);
}

async function toggleLiveDisclosure(sample: HTMLElement): Promise<void> {
    const disclosure = sample.querySelector('article > details') as HTMLDetailsElement;
    expect(disclosure.open).toBe(true);
    for (const open of [false, true]) {
        await userEvent.click(disclosure.querySelector('summary') as HTMLElement);
        expect(disclosure.isConnected).toBe(true);
        expect(sample.querySelector('article > details')).toBe(disclosure);
        expect((sample.querySelector('article > details') as HTMLDetailsElement).open).toBe(open);
    }
}
