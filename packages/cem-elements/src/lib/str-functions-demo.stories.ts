import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-str-functions-document';
const STR_FUNCTIONS_DEMO_URL = new URL('../../demo/functions/str.html', import.meta.url);
const MATRIX_LEGEND = 'str:shorten query/result matrix';

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
            () => host.querySelectorAll('cem-demo-element[legend]').length === 1,
            'the string-functions sample renders from the HTML source'
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
    },
};


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
