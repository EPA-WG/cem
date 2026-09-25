import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';

export default { title: 'CEM Elements/Supporting HTML Documents', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const demoUrl = new URL('../../demo/embed-1.html', import.meta.url);
const SVG_NS = 'http://www.w3.org/2000/svg';
const MATH_NS = 'http://www.w3.org/1998/Math/MathML';

function renderSource(path: string, tag: string, attributes: Record<string, string> = {}, payload = ''): HTMLElement {
    const root = document.createElement('section');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('src', new URL(path, demoUrl).href);
    declaration.setAttribute('link-base', 'source');
    const host = document.createElement(tag);
    for (const [name, value] of Object.entries(attributes)) host.setAttribute(name, value);
    host.textContent = payload;
    root.append(declaration, host);
    return root;
}

async function rendered(canvas: HTMLElement, tag: string): Promise<HTMLElement> {
    const host = required(canvas, tag);
    await whenCemSourceRendered(host);
    return host;
}

async function additionalSource(canvas: HTMLElement, path: string, tag: string): Promise<HTMLElement> {
    canvas.append(renderSource(path, tag));
    return rendered(canvas, tag);
}

async function additionalInstance(host: HTMLElement, payload?: string): Promise<HTMLElement> {
    const other = document.createElement(host.localName);
    if (payload !== undefined) {
        const strong = document.createElement('strong');
        strong.textContent = payload;
        other.append(strong);
    }
    host.after(other);
    await whenCemSourceRendered(other);
    return other;
}

function diagnostics(canvas: HTMLElement): void {
    for (const declaration of canvas.querySelectorAll<HTMLElement>('cem-element[tag]')) {
        expect(cemDiagnosticCodes(declaration)).toEqual([]);
        for (const host of canvas.querySelectorAll<HTMLElement>(declaration.getAttribute('tag') ?? '')) {
            expect(cemDiagnosticCodes(host)).toEqual([]);
        }
    }
}

function embeddedDocument(host: HTMLElement): void {
    expect(host.querySelectorAll('h4')).toHaveLength(1);
    expect(required(host, 'h4').textContent).toBe('embed-1.html');
    expect(host.querySelectorAll('[data-cem-anonymous-instance]')).toHaveLength(1);
    expect(normalized(required(host, '[data-cem-anonymous-instance]'))).toBe('🖖');
    // Full-document purity awaits the shared Storybook HTML-serving correction.
    expect(host.querySelectorAll('script')).toHaveLength(0);
}

async function embeddedLibrary(canvas: HTMLElement, tag: string, path: string): Promise<void> {
    const host = await rendered(canvas, tag);
    expect(normalized(host)).toBe('👋 from embed-lib-component');
    expect(host.querySelectorAll('h1, h4, a, img, article, script')).toHaveLength(0);
    const hash = await additionalSource(canvas, `${path}#embed-relative-hash`, `${tag}-hash`);
    expect(hash.querySelectorAll('a')).toHaveLength(1);
    const library = path.startsWith('lib-dir/') ? path : 'lib-dir/embed-lib.html';
    expect((required(hash, 'a') as HTMLAnchorElement).href).toBe(new URL(`${library}#embed-lib-component`, demoUrl).href);
    expect(normalized(required(hash, 'dce-embed-lib-component'))).toBe('👋 from embed-lib-component');
    expect(hash.querySelectorAll('img')).toHaveLength(1);
    const image = required(hash, 'img') as HTMLImageElement;
    expect(image.src).toBe(new URL('lib-dir/Smiley.svg', demoUrl).href);
    await waitFor(() => expect(image.complete && image.naturalWidth > 0).toBe(true));
    expect(image.alt).toBe('Library Smiley');
    expect(hash.querySelectorAll('h1, h4, article, script')).toHaveLength(0);
    const file = await additionalSource(canvas, `${path}#embed-relative-file`, `${tag}-file`);
    expect(file.querySelectorAll(':scope > a')).toHaveLength(1);
    expect((required(file, ':scope > a') as HTMLAnchorElement).href).toBe(new URL('embed-1.html', demoUrl).href);
    const nested = required(file, 'dce-embed-lib-file');
    await whenCemSourceRendered(nested);
    embeddedDocument(nested);
    expect(file.textContent).not.toContain('loading from');
    expect(normalized(host)).toBe('👋 from embed-lib-component');
    diagnostics(canvas);
}

export const EmbeddedDocument: Story = {
    render: () => renderSource('embed-1.html', 'story-support-embedded'),
    play: async ({ canvasElement }) => {
        embeddedDocument(await rendered(canvasElement, 'story-support-embedded'));
        diagnostics(canvasElement);
    },
};

export const EmbeddedLibrary: Story = {
    render: () => renderSource('embed-lib.html#embed-lib-component', 'story-support-library'),
    play: async ({ canvasElement }) => {
        await embeddedLibrary(canvasElement, 'story-support-library', 'embed-lib.html');
    },
};

export const RelativeLibrary: Story = {
    render: () => renderSource('lib-dir/embed-lib.html#embed-lib-component', 'story-support-relative-library'),
    play: async ({ canvasElement }) => {
        await embeddedLibrary(canvasElement, 'story-support-relative-library', 'lib-dir/embed-lib.html');
    },
};

export const ExternalDocument: Story = {
    render: () => renderSource('external-template-document.html', 'story-support-external-document'),
    play: async ({ canvasElement }) => {
        const host = await rendered(canvasElement, 'story-support-external-document');
        expect(host.querySelectorAll('article.external-document-template')).toHaveLength(1);
        expect(required(host, 'h2').textContent).toBe('External document');
        expect(required(host, 'article.external-document-template > p').textContent).toBe('External document fallback');
        const projected = await additionalInstance(host, 'Projected <fruit> & 🍒');
        expect(required(projected, 'h2').textContent).toBe('External document');
        expect(required(projected, 'article.external-document-template > p').textContent).toBe('Projected <fruit> & 🍒');
        expect(projected.querySelectorAll('article.external-document-template > p > strong')).toHaveLength(1);
        expect(required(host, 'article.external-document-template > p').textContent).toBe('External document fallback');
        expect(canvasElement.querySelectorAll('script, slot')).toHaveLength(0);
        diagnostics(canvasElement);
    },
};

export const ExternalTemplateLibrary: Story = {
    render: () => renderSource('external-template-templates.html#external-card-template', 'story-support-template',
        { title: 'Source-loaded card' }, 'Projected source content'),
    play: async ({ canvasElement, step }) => {
        const host = await rendered(canvasElement, 'story-support-template');
        const nodes = Array.from(host.querySelectorAll('article, h2, p, a'));
        expect(nodes).toHaveLength(4);
        expect(required(host, 'h2').textContent).toBe('Source-loaded card');
        expect(required(host, 'p').textContent).toBe('Projected source content');
        const target = new URL('external-template-templates.html#external-card-template', demoUrl).href;
        expect((required(host, 'a') as HTMLAnchorElement).href).toBe(target);
        expect(host.querySelectorAll('#external-subtree-template, .external-scoped-card, script')).toHaveLength(0);
        const fallback = await additionalInstance(host);
        expect(required(fallback, 'h2').textContent).toBe('External template');
        expect(required(fallback, 'p').textContent).toBe('External fallback');
        const projected = await additionalInstance(host, 'Projected <fruit> & 🍒');
        expect(required(projected, 'p > strong').textContent).toBe('Projected <fruit> & 🍒');
        for (const value of ['Edited title', '', '<Fruit & 🍒>', null]) {
            await step(`Title ${value === null ? 'removed' : JSON.stringify(value)}`, async () => {
                if (value === null) host.removeAttribute('title');
                else host.setAttribute('title', value);
                await waitFor(() => expect(required(host, 'h2').textContent).toBe(value ?? 'External template'));
                await whenCemSourceRendered(host);
                const current = Array.from(host.querySelectorAll('article, h2, p, a'));
                expect(current).toHaveLength(nodes.length);
                current.forEach((node, index) => expect(node).toBe(nodes[index]));
                expect(required(host, 'p').textContent).toBe('Projected source content');
                expect((required(host, 'a') as HTMLAnchorElement).href).toBe(target);
                expect(required(fallback, 'h2').textContent).toBe('External template');
                expect(required(projected, 'p > strong').textContent).toBe('Projected <fruit> & 🍒');
            });
        }
        const subtree = await additionalSource(canvasElement,
            'external-template-templates.html#external-subtree-template', 'story-support-subtree');
        expect(subtree.querySelectorAll('article')).toHaveLength(1);
        expect(required(subtree, '#external-subtree-template h2').textContent).toBe('External subtree');
        expect(required(subtree, 'p').textContent).toBe('External subtree fallback');
        expect(subtree.querySelectorAll('a, .external-scoped-card')).toHaveLength(0);
        const subtreePayload = await additionalInstance(subtree, 'Subtree payload 🍋');
        expect(required(subtreePayload, 'p > strong').textContent).toBe('Subtree payload 🍋');
        const scoped = await additionalSource(canvasElement,
            'external-template-templates.html#scoped-css-external-template', 'story-support-scoped');
        expect(scoped.querySelectorAll('section.external-scoped-card')).toHaveLength(1);
        expect(normalized(required(scoped, 'p'))).toBe('External scoped fallback ./external-template-templates.html#scoped-css-external-template');
        expect(getComputedStyle(required(scoped, 'p')).backgroundColor).toBe('rgb(254, 243, 199)');
        expect(getComputedStyle(required(scoped, 'p')).borderTopColor).toBe('rgb(180, 83, 9)');
        expect((required(scoped, 'a') as HTMLAnchorElement).href).toBe(new URL('external-template-templates.html', demoUrl).href);
        expect(scoped.querySelectorAll('article, h2, script')).toHaveLength(0);
        diagnostics(canvasElement);
    },
};

export const HtmlFragmentLibrary: Story = {
    render: () => renderSource('html-template.html', 'story-support-html-library'),
    play: async ({ canvasElement }) => {
        const host = await rendered(canvasElement, 'story-support-html-library');
        expect(host.querySelectorAll('b')).toHaveLength(2);
        expect(required(host, '#wave').textContent).toBe('👋');
        expect(required(host, '#ok').textContent).toBe('👌');
        checkSvg(required(host, '#dwc-logo'));
        checkMath(required(host, '#sophomores-dream'));
        expect(host.querySelector('script')).toBeNull();
        for (const [id, text] of [['wave', '👋'], ['ok', '👌'], ['dwc-logo', ''], ['sophomores-dream', '']]) {
            const fragment = await additionalSource(canvasElement, `html-template.html#${id}`, `story-support-fragment-${id}`);
            expect(fragment.querySelectorAll('b, svg, math')).toHaveLength(1);
            const element = required(fragment, `#${id}`);
            if (text) expect(element.textContent).toBe(text);
            if (id === 'dwc-logo') checkSvg(element);
            if (id === 'sophomores-dream') checkMath(element);
            expect(fragment.querySelector('script')).toBeNull();
        }
        diagnostics(canvasElement);
    },
};

function checkSvg(svg: HTMLElement): void {
    expect(svg.namespaceURI).toBe(SVG_NS);
    expect(Array.from(svg.querySelectorAll('*'), node => node.namespaceURI).every(ns => ns === SVG_NS)).toBe(true);
    expect(svg.getAttribute('viewBox')).toBe('0 0 216 209.18');
    expect(svg.querySelectorAll('polygon')).toHaveLength(1);
    expect(svg.querySelectorAll('path')).toHaveLength(21);
}

function checkMath(math: HTMLElement): void {
    expect(math.namespaceURI).toBe(MATH_NS);
    expect(Array.from(math.querySelectorAll('*'), node => node.namespaceURI).every(ns => ns === MATH_NS)).toBe(true);
    expect(math.getAttribute('display')).toBe('block');
    expect(math.querySelectorAll('msubsup')).toHaveLength(1);
    expect(math.querySelectorAll('munderover')).toHaveLength(1);
    expect(math.querySelectorAll('msup')).toHaveLength(3);
    expect(Array.from(math.querySelectorAll('mi'), node => node.textContent)).toEqual(['x', 'x', 'x', 'n', 'n', 'n', 'n']);
}

function required(root: ParentNode, selector: string): HTMLElement {
    const node = root.querySelector<HTMLElement>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}
function normalized(node: HTMLElement): string { return (node.textContent ?? '').replace(/\s+/gu, ' ').trim(); }
