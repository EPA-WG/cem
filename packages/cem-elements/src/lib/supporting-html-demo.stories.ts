import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { whenCemSourceRendered } from '../../.storybook/preview.js';

export default { title: 'CEM Elements/Supporting HTML Documents', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const demoUrl = new URL('../../demo/embed-1.html', import.meta.url);

function renderSource(path: string, tag: string, attributes: Record<string, string> = {}, payload = ''): HTMLElement {
    const root = document.createElement('section');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('src', new URL(path, demoUrl).href);
    const host = document.createElement(tag);
    for (const [name, value] of Object.entries(attributes)) host.setAttribute(name, value);
    host.textContent = payload;
    root.append(declaration, host);
    return root;
}

async function rendered(canvas: HTMLElement, tag: string): Promise<HTMLElement> {
    const host = canvas.querySelector<HTMLElement>(tag);
    if (!host) throw new Error(`Missing ${tag} source host`);
    await whenCemSourceRendered(host);
    return host;
}

export const EmbeddedDocument: Story = {
    render: () => renderSource('embed-1.html', 'story-support-embedded'),
    play: async ({ canvasElement }) => {
        const host = await rendered(canvasElement, 'story-support-embedded');
        expect(host.querySelector('h4')).toHaveTextContent('embed-1.html');
        expect(host).toHaveTextContent('🖖');
    },
};

export const EmbeddedLibrary: Story = {
    render: () => renderSource('embed-lib.html#embed-lib-component', 'story-support-library'),
    play: async ({ canvasElement }) => {
        expect(await rendered(canvasElement, 'story-support-library')).toHaveTextContent('👋 from embed-lib-component');
    },
};

export const RelativeLibrary: Story = {
    render: () => renderSource('lib-dir/embed-lib.html#embed-lib-component', 'story-support-relative-library'),
    play: async ({ canvasElement }) => {
        expect(await rendered(canvasElement, 'story-support-relative-library')).toHaveTextContent('👋 from embed-lib-component');
    },
};

export const ExternalDocument: Story = {
    render: () => renderSource('external-template-document.html', 'story-support-external-document'),
    play: async ({ canvasElement }) => {
        const host = await rendered(canvasElement, 'story-support-external-document');
        expect(host.querySelector('article.external-document-template h2')).toHaveTextContent('External document');
        expect(host.querySelector('article.external-document-template p')).toHaveTextContent('External document fallback');
    },
};

export const ExternalTemplateLibrary: Story = {
    render: () => renderSource('external-template-templates.html#external-card-template', 'story-support-template',
        { title: 'Source-loaded card' }, 'Projected source content'),
    play: async ({ canvasElement }) => {
        const host = await rendered(canvasElement, 'story-support-template');
        expect(host.querySelector('h2')).toHaveTextContent('Source-loaded card');
        expect(host.querySelector('p')).toHaveTextContent('Projected source content');
        expect(host.querySelector('a')).toHaveAttribute('href', './external-template-templates.html#external-card-template');
    },
};

export const HtmlFragmentLibrary: Story = {
    render: () => renderSource('html-template.html', 'story-support-html-library'),
    play: async ({ canvasElement }) => {
        const host = await rendered(canvasElement, 'story-support-html-library');
        expect(host.querySelector('#wave')).toHaveTextContent('👋');
        expect(host.querySelector('#ok')).toHaveTextContent('👌');
        expect(host.querySelector('#dwc-logo')?.namespaceURI).toBe('http://www.w3.org/2000/svg');
        expect(host.querySelector('#sophomores-dream')?.namespaceURI).toBe('http://www.w3.org/1998/Math/MathML');
        expect(host.querySelector('script')).toBeNull();
    },
};
