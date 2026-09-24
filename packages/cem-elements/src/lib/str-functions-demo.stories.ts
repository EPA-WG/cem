import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-str-functions-document';
const STR_FUNCTIONS_DEMO_URL = new URL('../../demo/functions/str.html', import.meta.url);
const MATRIX_LEGEND = 'str:shorten query/result matrix';
const METHODS = ['split', 'trim', 'trim_start', 'trim_end', 'char_at', 'at', 'index_of', 'last_index_of'] as const;

const meta: Meta = {
    title: 'CEM Elements/CEM-QL String Functions Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded CEM-QL string-functions demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', STR_FUNCTIONS_DEMO_URL.href);

        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await waitForCondition(
            () => host.querySelectorAll('cem-demo-element[legend]').length === 12,
            'all string-functions samples render from the HTML source'
        );
        assertDeepEqual(
            Array.from(host.querySelectorAll('cem-demo-element'), (sample) => sample.getAttribute('legend') ?? ''),
            [MATRIX_LEGEND, 'URL ID with a string chain', ...METHODS.map((method) => `str:${method}`), 'XPath normalize-space', 'XPath tokenize and string-join'],
            'string-function sample inventory'
        );

        const sample = requiredElement(host, `cem-demo-element[legend="${MATRIX_LEGEND}"]`);
        await waitForCondition(
            () => sample.querySelectorAll('cem-str-shorten-matrix tbody tr').length === 7,
            'all str:shorten matrix rows render'
        );

        const queries = Array.from(
            sample.querySelectorAll('cem-str-shorten-matrix tbody td:first-of-type'),
            (cell) => normalize(cell.textContent ?? '')
        );
        const results = Array.from(
            sample.querySelectorAll('cem-str-shorten-matrix tbody td:nth-of-type(2)'),
            (cell) => normalize(cell.textContent ?? '')
        );
        assertDeepEqual(queries, [
            'str:shorten("short", 8)',
            'str:shorten("abcdefghij", 7)',
            'str:shorten("abcdefghij", 8)',
            'str:shorten("abcdefghij", 8, "...")',
            'str:shorten("abcdefghij", 6, "")',
            'str:shorten("αβ😀δεζη", 5, "💠")',
            'str:shorten( "https://example.test/lib/semantic-card.cem" , 32)',
        ], 'str:shorten query matrix');
        assertDeepEqual(
            results,
            ['short', 'abc…hij', 'abc…ghij', 'ab...hij', 'abchij', 'αβ💠ζη', 'https://example…emantic-card.cem'],
            'str:shorten result matrix'
        );
        const chain = requiredElement(host, 'cem-demo-element[legend="URL ID with a string chain"]');
        await waitForOutput(chain, '1');
        await edit(chain, 'Pokémon URL', 'https://pokeapi.co/api/v2/pokemon/10/');
        await waitForOutput(chain, '10');
        await edit(chain, 'Pokémon URL', '/short/');
        await waitForOutput(chain, 'No ID');
        await verifySimpleMethods(host);
        const normalized = requiredElement(host, 'cem-demo-element[legend="XPath normalize-space"]');
        await waitForOutput(normalized, '🍒 🍋');
        await edit(normalized, 'Text', ' a\ta\n🍒  ');
        await waitForOutput(normalized, 'a a 🍒');
        await edit(normalized, 'Text', ' a\u00a0\u2003b ');
        await waitForOutput(normalized, 'a\u00a0\u2003b');
        await edit(normalized, 'Text', ' \t\n');
        await waitForOutput(normalized, '');
        const joined = requiredElement(host, 'cem-demo-element[legend="XPath tokenize and string-join"]');
        await waitForOutput(joined, '🍒/🍒/🍋');
        await edit(joined, 'Text', 'a\ta\nb');
        await waitForOutput(joined, 'a/a/b');
        await edit(joined, 'Separator', '🍒');
        await waitForOutput(joined, 'a🍒a🍒b');
        await edit(joined, 'Text', 'a\u00a0b');
        await waitForOutput(joined, 'a\u00a0b');
        await edit(joined, 'Text', '');
        await waitForOutput(joined, '');
    },
};

async function verifySimpleMethods(host: HTMLElement): Promise<void> {
    const sample = (method: string) => requiredElement(host, `cem-demo-element[legend="str:${method}"]`);
    const split = sample('split');
    await waitForOutput(split, '4');
    assertDeepEqual(parts(split), ['“🍒”', '“🍋”', '“”', '“🍌”'], 'split preserves empty fields');
    await edit(split, 'Text', 'a::b::::');
    await edit(split, 'Separator', '::');
    await waitForCondition(() => parts(split).join('|') === '“a”|“b”|“”|“”', 'literal multi-character separator');
    await edit(split, 'Text', '🍒🍋');
    await edit(split, 'Separator', '');
    await waitForOutput(split, '4');
    assertDeepEqual(parts(split), ['“”', '“🍒”', '“🍋”', '“”'], 'Rust splitting keeps codepoints and boundary empties');
    await edit(split, 'Text', '');
    await waitForOutput(split, '2');
    await edit(split, 'Separator', ',');
    await waitForOutput(split, '1');
    assertDeepEqual(parts(split), ['“”'], 'empty input has one empty field with a nonempty separator');

    for (const [method, initial, edited] of [
        ['trim', '🍒  🍋', '🍌  🍒'],
        ['trim_start', '🍒  🍋  ', '🍌  🍒 \uFEFF'],
        ['trim_end', '  🍒  🍋', ' \u00a0🍌  🍒'],
    ]) {
        const root = sample(method);
        await waitForOutput(root, initial);
        await edit(root, 'Text', ' \u00a0🍌  🍒 \uFEFF');
        await waitForOutput(root, edited);
        await edit(root, 'Text', '');
        await waitForOutput(root, '');
    }

    for (const method of ['char_at', 'at']) {
        const root = sample(method);
        await waitForOutput(root, method === 'at' ? '🍌' : '🍋');
        await edit(root, 'Index', '99');
        await waitForOutput(root, method === 'at' ? '∅' : '');
        await edit(root, 'Index', '-1');
        await waitForOutput(root, method === 'at' ? '🍌' : '');
        await edit(root, 'Index', '0');
        await waitForOutput(root, '🍒');
    }

    for (const method of ['index_of', 'last_index_of']) {
        const root = sample(method);
        await waitForOutput(root, method === 'index_of' ? '0' : '2');
        await edit(root, 'Position', '1');
        await waitForOutput(root, method === 'index_of' ? '2' : '0');
        await edit(root, 'Find', '🍋');
        await waitForOutput(root, '1');
        await edit(root, 'Find', '🥦');
        await waitForOutput(root, '-1');
        await edit(root, 'Find', '');
        await waitForOutput(root, '1');
        await edit(root, 'Position', '99');
        await waitForOutput(root, '3');
        await edit(root, 'Text', 'aaaa');
        await edit(root, 'Find', 'aa');
        await edit(root, 'Position', '1');
        await waitForOutput(root, '1');
    }
}

function parts(root: HTMLElement): string[] {
    return Array.from(root.querySelectorAll('li'), (item) => item.textContent ?? '');
}

async function edit(root: HTMLElement, label: string, value: string): Promise<void> {
    const field = Array.from(root.querySelectorAll('label')).find((element) => normalize(element.firstChild?.textContent ?? '') === label);
    const input = field?.querySelector<HTMLInputElement | HTMLTextAreaElement>('input, textarea');
    if (!input) throw new Error(`expected input labelled ${label}`);
    input.value = value;
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await waitForCondition(
        () => (input instanceof HTMLTextAreaElement ? input.textContent : input.getAttribute('value') ?? '') === value,
        `${label} should commit its new value before the next edit`
    );
}

async function waitForOutput(root: HTMLElement, expected: string): Promise<void> {
    await waitForCondition(
        () => root.querySelector('output')?.textContent === expected,
        `${root.getAttribute('legend')} output should be ${JSON.stringify(expected)}`
    );
}

async function waitForCondition(
    condition: () => boolean,
    message: string,
    attempts = 600
): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
