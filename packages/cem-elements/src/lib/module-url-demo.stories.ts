import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-module-url-document';
const MODULE_URL_DEMO_URL = new URL('../../demo/module-url.html', import.meta.url);
const EXPECTED_LEGENDS = [
    'this page import maps',
    '1. module path by symbolic name',
    '2. src forms: relative URL',
    '3. src forms: absolute URL',
    '5. component-local map: naked',
    '6. component-local map: wrapper override',
    '7. component-local map: node referrer',
    'image-link',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Module URL Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        defineHtmlDemoElementFixture();

        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded module URL demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', MODULE_URL_DEMO_URL.href);

        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await waitForCondition(
            () => host.querySelectorAll('html-demo-element[legend]').length === EXPECTED_LEGENDS.length,
            'all module-url samples render from the HTML source'
        );

        const actualLegends = Array.from(host.querySelectorAll('html-demo-element[legend]'), (sample) =>
            normalize(sample.getAttribute('legend') ?? '')
        );
        assertDeepEqual(actualLegends, [...EXPECTED_LEGENDS], 'module-url sample inventory');

        const symbolic = sampleByLegend(host, EXPECTED_LEGENDS[1]);
        const squareUrl = new URL('./wc-square.svg', MODULE_URL_DEMO_URL).href;
        await waitForCondition(
            () => symbolic.querySelector('image-link')?.getAttribute('src') === squareUrl
                && symbolic.querySelector('image-link img')?.getAttribute('src') === squareUrl
                && symbolic.querySelector('image-link a')?.getAttribute('href') === squareUrl,
            () => `resolved package-subpath slice reaches image-link src; observed ${symbolic.querySelector('image-link')?.outerHTML ?? 'no image-link'}`
        );
        assertEqual(
            requiredElement(symbolic, 'image-link img').getAttribute('src'),
            squareUrl,
            'image-link renders the package-subpath image URL'
        );
        assertEqual(
            requiredElement(symbolic, 'image-link a').getAttribute('href'),
            squareUrl,
            'image-link retains the package-subpath URL as its link target'
        );

        const smileyUrl = new URL('./lib-dir/Smiley.svg', MODULE_URL_DEMO_URL);
        const relativeSrcUrl = withSearch(smileyUrl, 'src', 'relative');
        const relative = sampleByLegend(host, EXPECTED_LEGENDS[2]);
        await waitForCondition(
            () => relative.querySelector('image-link')?.getAttribute('src') === relativeSrcUrl,
            'anonymous relative-URL sample passes its resolved slice to image-link'
        );

        const absolute = sampleByLegend(host, EXPECTED_LEGENDS[3]);
        await waitForCondition(
            () => {
                const source = absolute.querySelector('image-link')?.getAttribute('src');
                return source?.startsWith('data:image/svg+xml,') === true
                    && absolute.querySelector('image-link img')?.getAttribute('src') === source
                    && absolute.querySelector('image-link a')?.getAttribute('href') === source;
            },
            'absolute src passes unchanged through image-link'
        );

        const confusedUrl = new URL('./confused.svg', MODULE_URL_DEMO_URL);
        const squareReferrerUrl = new URL('./wc-square.svg', MODULE_URL_DEMO_URL);
        const nakedUrl = withSearch(smileyUrl, 'owner', 'component');
        const wrappedUrl = withSearch(confusedUrl, 'owner', 'wrapper');
        const nodeReferrerUrl = withSearch(squareReferrerUrl, 'owner', 'component');

        const naked = sampleByLegend(host, EXPECTED_LEGENDS[4]);
        await waitForCondition(
            () => naked.querySelector('cem-local-map-naked-image img')?.getAttribute('src') === nakedUrl
                && naked.querySelector('cem-local-map-naked-image a')?.getAttribute('href') === nakedUrl,
            'naked component resolves through its own module map'
        );

        const override = sampleByLegend(host, EXPECTED_LEGENDS[5]);
        await waitForCondition(
            () => override.querySelector('cem-local-map-override-image img')?.getAttribute('src') === wrappedUrl
                && override.querySelector('cem-local-map-override-image a')?.getAttribute('href') === wrappedUrl,
            'wrapper module map overrides the child mapping'
        );

        const nodeReferrer = sampleByLegend(host, EXPECTED_LEGENDS[6]);
        const nodeReferrerCells = () => Array.from(
            nodeReferrer.querySelectorAll('cem-local-map-referrer-demo table.node-referrer-matrix td expando-link a'),
            (link) => link.getAttribute('href')
        );
        const expectedNodeReferrerCells = [
            withSearch(smileyUrl, 'referrer', 'node'),
            nodeReferrerUrl,
            'https://assets.example.test/logo.svg',
        ];
        await waitForCondition(
            () => nodeReferrer.querySelector('cem-local-map-referrer-demo img.node-referrer-image')?.getAttribute('src') === nodeReferrerUrl &&
                nodeReferrerCells().length === expectedNodeReferrerCells.length &&
                nodeReferrerCells().every((value, index) => value === expectedNodeReferrerCells[index]),
            () => `descendant node-referrer values resolve; observed image ${nodeReferrer.querySelector('img.node-referrer-image')?.getAttribute('src') ?? 'none'}, cells ${JSON.stringify(nodeReferrerCells())}`
        );
        assertEqual(
            nodeReferrerCells()[1],
            nodeReferrerUrl,
            'node referrer publishes the inner-only child mapping'
        );
        assertEqual(
            nodeReferrer.querySelector('cem-local-map-referrer-demo image-link.node-referrer-image a')?.getAttribute('href'),
            nodeReferrerUrl,
            'node-referrer image-link retains the full resolved URL'
        );

        const helper = sampleByLegend(host, EXPECTED_LEGENDS[7]);
        const helperLink = requiredElement(helper, 'image-link a');
        assertEqual(
            normalize(helperLink.textContent ?? ''),
            shortenMiddle(confusedUrl.href, 32),
            'relative helper URL resolves from the source file and is shortened'
        );
        assertEqual(helperLink.getAttribute('href'), confusedUrl.href, 'helper link retains its full resolved URL');
        assert(
            host.querySelector('cem-module-url') === null,
            'transient cem-module-url controls are removed from rendered output'
        );
    },
};

function defineHtmlDemoElementFixture(): void {
    if (customElements.get('html-demo-element')) return;

    class HtmlDemoElementFixture extends HTMLElement {
        connectedCallback(): void {
            if (this.querySelector(':scope > [slot="demo"]')) return;
            const template = Array.from(this.children).find(
                (child): child is HTMLTemplateElement => child instanceof HTMLTemplateElement
            );
            if (!template) return;
            const demo = document.createElement('div');
            demo.slot = 'demo';
            demo.append(template.content.cloneNode(true));
            this.append(demo);
        }
    }

    customElements.define('html-demo-element', HtmlDemoElementFixture);
}

function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('html-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    assert(sample, `expected sample ${legend}`);
    return sample;
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    assert(element, `expected ${selector}`);
    return element;
}

function withSearch(input: URL, name: string, value: string): string {
    const url = new URL(input);
    url.searchParams.set(name, value);
    return url.href;
}

function shortenMiddle(input: string, maxLength: number): string {
    const codepoints = Array.from(input);
    if (codepoints.length <= maxLength) return input;
    const prefixLength = Math.floor((maxLength - 1) / 2);
    const suffixLength = maxLength - 1 - prefixLength;
    return `${codepoints.slice(0, prefixLength).join('')}…${codepoints.slice(-suffixLength).join('')}`;
}

async function waitForCondition(
    condition: () => boolean,
    message: string | (() => string),
    attempts = 120
): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(typeof message === 'string' ? message : message());
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, message: string): void {
    if (actual !== expected) {
        throw new Error(`${message}: expected ${String(expected)}, got ${String(actual)}`);
    }
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], message: string): void {
    if (actual.length !== expected.length || actual.some((value, index) => value !== expected[index])) {
        throw new Error(`${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
}
