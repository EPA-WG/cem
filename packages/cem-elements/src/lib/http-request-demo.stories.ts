import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-http-request-demo-document';
const DEMO_URL = new URL('../../demo/http-request.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '0. URL from text to http-request',
    '1. Simplest http-request',
    '2. http-request response and headers',
] as const;

const meta: Meta = {
    title: 'CEM Elements/HTTP Request Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded HTTP request demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', DEMO_URL.href);

        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await waitForCondition(
            () => host.querySelectorAll('cem-demo-element[legend] article').length === EXPECTED_LEGENDS.length,
            'all three HTTP request samples render from the HTML source',
            300
        );

        assertDeepEqual(
            Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
                normalize(sample.getAttribute('legend') ?? '')
            ),
            [...EXPECTED_LEGENDS],
            'HTTP request sample inventory'
        );

        await verifyRuntimeUrlSelection(sampleByLegend(host, EXPECTED_LEGENDS[0]));
        await verifySimplestRequest(sampleByLegend(host, EXPECTED_LEGENDS[1]));
        await verifyRequestEnvelope(sampleByLegend(host, EXPECTED_LEGENDS[2]));
    },
};

async function verifyRuntimeUrlSelection(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => textList(sample, 'li').join('|') === 'alpha: ready|beta: loaded',
        'the initial selected URL loads the full response',
        300
    );

    click(buttonByText(sample, 'Compact records'));
    await waitForCondition(
        () => requiredInput(sample).value === './http-data-compact.json',
        'the compact preset updates the editable URL'
    );
    assertText(sample, 'article', 'Selected URL: ./http-data-compact.json', 'the selected URL is visible', true);

    click(buttonByText(sample, 'GET'));
    await waitForCondition(
        () => textList(sample, 'li').join('|') === 'solo: compact',
        'GET replaces the resource with the compact response',
        300
    );
    assertText(sample, 'article', 'Requested URL: ./http-data-compact.json', 'the requested URL is visible', true);
    assertText(sample, 'article', 'Request state: loaded', 'the selected request reaches loaded state', true);
}

async function verifySimplestRequest(sample: HTMLElement): Promise<void> {
    const expectedNames = [
        'bulbasaur',
        'ivysaur',
        'venusaur',
        'charmander',
        'charmeleon',
        'charizard',
    ];
    await waitForCondition(
        () => Array.from(sample.querySelectorAll('.result-buttons button'), (button) =>
            button.getAttribute('aria-label')
        ).join('|') === expectedNames.join('|'),
        'the fixed request renders all six Pokémon buttons',
        300
    );
    assertEqual(sample.querySelectorAll('.result-buttons img').length, 6, 'each Pokémon button has an image');
    assertEqual(sample.querySelectorAll('.result-buttons span').length, 0, 'the buttons do not repeat names as text');
    for (const [index, image] of Array.from(sample.querySelectorAll<HTMLImageElement>('.result-buttons img')).entries()) {
        assertEqual(
            image.src,
            `https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/${index + 1}.svg`,
            `${expectedNames[index]} sprite URL`
        );
        assertEqual(image.alt, expectedNames[index], 'each Pokémon image has useful alternative text');
    }
    assertText(sample, 'article', 'Request state: loaded', 'the fixed request reaches loaded state', true);
}

async function verifyRequestEnvelope(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => definitionValue(sample, 'State') === 'loaded',
        'the inspection request reaches loaded state',
        300
    );

    assertDefinitionValue(sample, 'Method', 'GET');
    assertDefinitionValue(sample, 'Authored URL', './http-data.json');
    assertDefinitionValue(sample, 'Accept header', 'application/json');
    assertDefinitionValue(sample, 'X-Demo header', 'ported-from-legacy');
    assertDefinitionValue(sample, 'Status', '200');
    assert(
        definitionValue(sample, 'Content type').startsWith('application/json'),
        'the response content type is available in the envelope'
    );

    const resolvedLink = requiredElement(sample, 'dd a') as HTMLAnchorElement;
    assert(
        new URL(resolvedLink.href).pathname.endsWith('/demo/http-data.json'),
        `resolved URL uses the demo source base: ${resolvedLink.href}`
    );
    assertEqual(resolvedLink.title, resolvedLink.href, 'the complete resolved URL remains available');
    assert((resolvedLink.textContent?.trim().length ?? 0) <= 32, 'the displayed resolved URL stays compact');
    assert(resolvedLink.textContent?.includes('…'), 'the compact resolved URL uses a middle ellipsis');
}


function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    if (!sample) throw new Error(`expected sample ${legend}`);
    return sample;
}

function requiredInput(root: ParentNode): HTMLInputElement {
    const input = root.querySelector('input');
    if (!(input instanceof HTMLInputElement)) throw new Error('expected the request URL input');
    return input;
}

function buttonByText(root: ParentNode, expected: string): HTMLButtonElement {
    const button = Array.from(root.querySelectorAll('button')).find(
        (candidate) => normalize(candidate.textContent ?? '') === expected
    );
    if (!button) throw new Error(`expected ${expected} button`);
    return button;
}

function click(element: HTMLElement): void {
    element.click();
}

function textList(root: ParentNode, selector: string): string[] {
    return Array.from(root.querySelectorAll(selector), (element) => normalize(element.textContent ?? ''));
}

function definitionValue(root: ParentNode, term: string): string {
    const definitionTerm = Array.from(root.querySelectorAll('dt')).find(
        (candidate) => normalize(candidate.textContent ?? '') === term
    );
    const value = definitionTerm?.nextElementSibling;
    if (!(value instanceof HTMLElement) || value.localName !== 'dd') {
        throw new Error(`expected definition value for ${term}`);
    }
    return normalize(value.textContent ?? '');
}

function assertDefinitionValue(root: ParentNode, term: string, expected: string): void {
    assertEqual(definitionValue(root, term), expected, term);
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 160): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}

function assertText(root: ParentNode, selector: string, expected: string, label: string, contains = false): void {
    const actual = normalize(requiredElement(root, selector).textContent ?? '');
    if (contains ? !actual.includes(expected) : actual !== expected) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) throw new Error(message);
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
