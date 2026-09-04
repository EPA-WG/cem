import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-module-url-document';
const MODULE_URL_DEMO_URL = new URL('../../demo/module-url.html', import.meta.url);
const EXPECTED_LEGENDS = [
    'this page import maps',
    '4. module path by symbolic name',
    '5. src forms: relative URL, module path, and absolute URL',
    '6. src by scalar referrer matrix',
    '7. component-local map: naked, wrapper override, and node referrer',
    '8. str:shorten query/result matrix',
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
        await waitForCondition(
            () => symbolic.querySelector('cem-module-link a')?.getAttribute('href') ===
                'https://storybook.example.test/material/README.md',
            () => `symbolic package URL resolves through the Storybook root map; observed ${symbolic.querySelector('cem-module-link')?.outerHTML ?? 'no host'}`
        );
        assertEqual(
            requiredElement(symbolic, 'cem-module-link img').getAttribute('src'),
            new URL('./lib-dir/Smiley.svg', MODULE_URL_DEMO_URL).href,
            'symbolic logo URL resolves through the Storybook root map'
        );

        const srcForms = sampleByLegend(host, EXPECTED_LEGENDS[2]);
        const smileyUrl = new URL('./lib-dir/Smiley.svg', MODULE_URL_DEMO_URL);
        const relativeSrcUrl = withSearch(smileyUrl, 'src', 'relative');
        const moduleSrcUrl = withSearch(smileyUrl, 'src', 'module');
        const srcFormValues = () => Array.from(
            srcForms.querySelectorAll('cem-module-src-forms img'),
            (image) => image.getAttribute('src')
        );
        const srcFormLinks = () => Array.from(
            srcForms.querySelectorAll('cem-module-src-forms image-link a'),
            (link) => link.getAttribute('href')
        );
        await waitForCondition(
            () => {
                const values = srcFormValues();
                const links = srcFormLinks();
                return values[0] === relativeSrcUrl
                    && values[1] === moduleSrcUrl
                    && values[2]?.startsWith('data:image/svg+xml,')
                    && links.length === values.length
                    && links.every((value, index) => value === values[index]);
            },
            () => `relative, mapped module, and absolute src forms resolve through image-link; observed images ${JSON.stringify(srcFormValues())}, links ${JSON.stringify(srcFormLinks())}`
        );

        const matrix = sampleByLegend(host, EXPECTED_LEGENDS[3]);
        const matrixCells = () => Array.from(
            matrix.querySelectorAll('cem-module-referrer-matrix tbody td'),
            (cell) => normalize(cell.textContent ?? '')
        );
        const confusedUrl = new URL('./confused.svg', MODULE_URL_DEMO_URL);
        const squareUrl = new URL('./wc-square.svg', MODULE_URL_DEMO_URL);
        const expectedMatrix = [
            withSearch(smileyUrl, 'case', 'relative-relative'),
            withSearch(smileyUrl, 'referrer', 'relative'),
            'https://assets.example.test/logo.svg',
            withSearch(smileyUrl, 'case', 'relative-module'),
            withSearch(confusedUrl, 'referrer', 'module'),
            'https://assets.example.test/logo.svg',
            'https://referrer.example.test/lib-dir/Smiley.svg?case=relative-absolute',
            withSearch(squareUrl, 'referrer', 'absolute'),
            'https://assets.example.test/logo.svg',
        ];
        await waitForCondition(
            () => matrixCells().length === expectedMatrix.length &&
                matrixCells().every((value, index) => value === expectedMatrix[index]),
            'all scalar src by referrer combinations publish their expected URLs'
        );

        const localMaps = sampleByLegend(host, EXPECTED_LEGENDS[4]);
        const componentImages = () => Array.from(
            localMaps.querySelectorAll<HTMLElement>('cem-local-map-image img.component-owned-image')
        );
        const componentLinks = () => Array.from(
            localMaps.querySelectorAll<HTMLAnchorElement>('cem-local-map-image image-link a')
        );
        const nakedUrl = withSearch(smileyUrl, 'owner', 'component');
        const wrappedUrl = withSearch(confusedUrl, 'owner', 'wrapper');
        const nodeReferrerUrl = withSearch(squareUrl, 'owner', 'component');
        const nodeReferrerCells = () => Array.from(
            localMaps.querySelectorAll('cem-local-map-wrapper table.node-referrer-matrix td'),
            (cell) => normalize(cell.textContent ?? '')
        );
        const expectedNodeReferrerCells = [
            withSearch(smileyUrl, 'referrer', 'node'),
            nodeReferrerUrl,
            'https://assets.example.test/logo.svg',
        ];
        await waitForCondition(
            () => componentImages().length === 2 &&
                componentImages()[0].getAttribute('src') === nakedUrl &&
                componentImages()[1].getAttribute('src') === wrappedUrl &&
                componentLinks().length === 2 &&
                normalize(componentLinks()[0].textContent ?? '') === shortenMiddle(nakedUrl, 32) &&
                normalize(componentLinks()[1].textContent ?? '') === shortenMiddle(wrappedUrl, 32) &&
                localMaps.querySelector('cem-local-map-wrapper img.node-referrer-image')?.getAttribute('src') === nodeReferrerUrl &&
                nodeReferrerCells().length === expectedNodeReferrerCells.length &&
                nodeReferrerCells().every((value, index) => value === expectedNodeReferrerCells[index]),
            'local component map, wrapper override, and all descendant node-referrer src forms resolve'
        );
        assertEqual(
            nodeReferrerCells()[1],
            nodeReferrerUrl,
            'node referrer publishes the inner-only child mapping'
        );
        assertEqual(
            localMaps.querySelector('cem-local-map-wrapper image-link.node-referrer-image a')?.getAttribute('href'),
            nodeReferrerUrl,
            'node-referrer image-link retains the full resolved URL'
        );
        assertEqual(componentLinks()[0].getAttribute('href'), nakedUrl, 'naked component link retains its full URL');
        assertEqual(componentLinks()[1].getAttribute('href'), wrappedUrl, 'wrapped component link retains its full URL');

        const shortenMatrix = sampleByLegend(host, EXPECTED_LEGENDS[5]);
        await waitForCondition(
            () => shortenMatrix.querySelectorAll('cem-str-shorten-matrix tbody tr').length === 7,
            'all str:shorten matrix rows render'
        );
        const shortenQueries = Array.from(
            shortenMatrix.querySelectorAll('cem-str-shorten-matrix tbody td:first-of-type'),
            (cell) => normalize(cell.textContent ?? '')
        );
        const shortenResults = Array.from(
            shortenMatrix.querySelectorAll('cem-str-shorten-matrix tbody td:nth-of-type(2)'),
            (cell) => normalize(cell.textContent ?? '')
        );
        assertDeepEqual(shortenQueries, [
            'str:shorten("short", 8)',
            'str:shorten("abcdefghij", 7)',
            'str:shorten("abcdefghij", 8)',
            'str:shorten("abcdefghij", 8, "...")',
            'str:shorten("abcdefghij", 6, "")',
            'str:shorten("αβ😀δεζη", 5, "💠")',
            'str:shorten( "https://example.test/lib/semantic-card.cem" , 32)',
        ], 'str:shorten query matrix');
        assertDeepEqual(
            shortenResults,
            ['short', 'abc…hij', 'abc…ghij', 'ab...hij', 'abchij', 'αβ💠ζη', 'https://example…emantic-card.cem'],
            'str:shorten result matrix'
        );

        const helper = sampleByLegend(host, EXPECTED_LEGENDS[6]);
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
