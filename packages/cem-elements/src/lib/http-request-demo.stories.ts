import { verifyExternalFilePreviews } from '../../.storybook/external-file-previews.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { userEvent } from 'storybook/test';
import { cemDiagnosticCodes, traceCemReadiness, whenCemRendered } from '../../.storybook/preview.js';
import { readinessCheckpoint, readinessWait } from '../../.storybook/readiness-timing.js';

const SOURCE_TAG = 'story-http-request-demo-document';
const DEMO_URL = new URL('../../demo/http-request.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '0. URL from text to http-request',
    '1. Simplest http-request',
    '2. http-request response and headers',
] as const;
const PREVIEW_FILES = ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json'] as const;

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
        traceCemReadiness(root, 'http/EveryAuthoredSample');
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        const articleCounts = () => Array.from(host.querySelectorAll('cem-demo-element[legend]'), sample =>
            ({ legend: sample.getAttribute('legend'), articles: sample.querySelectorAll('article').length }));
        try {
            await waitForCondition(
                () => host.querySelectorAll('cem-demo-element[legend] article').length === EXPECTED_LEGENDS.length,
                'all three HTTP request samples render from the HTML source',
                300
            );
        } catch (error) {
            readinessCheckpoint('initial-articles-failed', { samples: articleCounts() });
            throw error;
        }
        readinessCheckpoint('initial-articles-ready', { samples: articleCounts() });

        assertDeepEqual(
            Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
                normalize(sample.getAttribute('legend') ?? '')
            ),
            [...EXPECTED_LEGENDS, ...PREVIEW_FILES],
            'HTTP request sample inventory'
        );

        const checks = [verifyRuntimeUrlSelection, verifySimplestRequest, verifyRequestEnvelope];
        for (const [index, check] of checks.entries()) {
            await step(EXPECTED_LEGENDS[index], async () => {
                const sample = sampleByLegend(host, EXPECTED_LEGENDS[index]);
                await check(sample);
                assertDeepEqual(cemDiagnosticCodes(requiredElement(sample, 'article').parentElement as HTMLElement),
                    index === 0 ? ['cem-element.http_request_parse_failed'] : [],
                    'only the deliberately malformed response produces a diagnostic');
            });
        }
        for (const file of PREVIEW_FILES) {
            await step(file, async () => verifyExternalFilePreviews(host, DEMO_URL, [file]));
        }
        assertDeepEqual(cemDiagnosticCodes(host), [], 'source document diagnostics');
    },
};

async function verifyRuntimeUrlSelection(sample: HTMLElement): Promise<void> {
    const owner = requiredElement(sample, 'article').parentElement as HTMLElement;
    const input = requiredInput(sample);
    const get = buttonByName(sample, 'GET');
    const full = ['alpha: ready', 'beta: loaded'];
    const compact = ['solo: compact'];
    const state = async (selected: string, requested: string, status: string, rows: readonly string[]) => {
        await whenCemRendered(owner);
        await waitForCondition(() => textList(sample, 'article > p output').join('|') === [selected, requested, status].join('|')
            && textList(sample, 'li').join('|') === rows.join('|'),
        `selected=${selected}, requested=${requested}, state=${status}, rows=${rows.join('|')}`, 300);
        assertEqual(requiredInput(sample), input, 'live URL input identity');
        assertEqual(input.value, selected, 'editable URL value');
        assertEqual(buttonByName(sample, 'GET'), get, 'live GET button identity');
        assertEqual(get.value, selected, 'GET copies the current draft');
        assertEqual(normalize(requiredElement(sample, 'article').textContent ?? '').includes('This URL did not provide an accepted JSON response.'),
            status === 'failed', 'failure guidance appears only for a failed request');
    };
    const preset = async (name: string) => { await userEvent.click(buttonByName(sample, name)); };
    const edit = async (value: string) => {
        await userEvent.clear(input);
        if (value) await userEvent.type(input, value);
        await whenCemRendered(owner);
        assertEqual(document.activeElement, input, 'editing retains input focus');
        assertEqual(input.selectionStart, value.length, 'editing retains the caret');
    };
    const submit = async () => { await userEvent.click(get); };

    await state('./http-data.json', '', 'idle', []);
    // Preset-after-typing refresh is pending the shared-value decision in TODO.
    // Keep the preset sequence before direct edits while that case is open.
    await submit();
    await state('./http-data.json', './http-data.json', 'loaded', full);
    await preset('Compact records');
    await state('./http-data-compact.json', './http-data.json', 'loaded', full);
    await submit();
    await state('./http-data-compact.json', './http-data-compact.json', 'loaded', compact);

    await preset('Invalid JSON response');
    await state('./http-data-invalid.json', './http-data-compact.json', 'loaded', compact);
    await submit();
    await state('./http-data-invalid.json', './http-data-invalid.json', 'failed', []);
    await preset('All records');
    await state('./http-data.json', './http-data-invalid.json', 'failed', []);
    await submit();
    await state('./http-data.json', './http-data.json', 'loaded', full);

    await preset('Empty URL');
    await state('', './http-data.json', 'loaded', full);
    await submit();
    await state('', '', 'idle', []);
    // A typed URL can restart the removed resource; Enter activates native GET.
    await edit('./http-data-compact.json');
    await state('./http-data-compact.json', '', 'idle', []);
    get.focus();
    await userEvent.keyboard('{Enter}');
    await state('./http-data-compact.json', './http-data-compact.json', 'loaded', compact);
    await edit('./http-data.json');
    await state('./http-data.json', './http-data-compact.json', 'loaded', compact);
    await submit();
    await state('./http-data.json', './http-data.json', 'loaded', full);
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
        const button = image.parentElement as HTMLButtonElement;
        assertEqual(button.title, expectedNames[index], 'button title');
        assertEqual(button.type, 'button', 'catalog buttons do not submit a form');
        assertEqual(normalize(button.textContent ?? ''), '', 'image button has no duplicate visible name');
        button.focus();
        assertEqual(document.activeElement, button, 'catalog button accepts keyboard focus');
        await waitForCondition(() => image.complete && image.naturalWidth > 0, 'Pokémon sprite loads', 300);
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
        resolvedLink.href === new URL('./http-data.json', DEMO_URL).href,
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

function buttonByName(root: ParentNode, expected: string): HTMLButtonElement {
    const button = Array.from(root.querySelectorAll('button')).find(
        (candidate) => normalize(candidate.getAttribute('aria-label') ?? candidate.textContent ?? '') === expected
    );
    if (!button) throw new Error(`expected ${expected} button`);
    return button;
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
    const mark = readinessWait(message, attempts);
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) {
            mark('ready', attempt);
            return;
        }
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    mark('timeout', attempts);
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
