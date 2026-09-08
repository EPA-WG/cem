import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-module-url-referrer-document';
const DEMO_URL = new URL('../../demo/module-url-referrer.html', import.meta.url);
const MATRIX_LEGEND = 'src by scalar referrer matrix';

const meta: Meta = {
    title: 'CEM Elements/Module URL Referrer Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded module URL referrer demo coverage');

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
            () => host.querySelectorAll('cem-demo-element[legend]').length === 1,
            'the module-URL referrer sample renders from the HTML source'
        );

        const sample = requiredElement(host, `cem-demo-element[legend="${MATRIX_LEGEND}"]`);
        const cells = () => Array.from(
            sample.querySelectorAll('tbody td'),
            (cell) => normalize(cell.textContent ?? '')
        );
        const smileyUrl = new URL('./lib-dir/Smiley.svg', DEMO_URL);
        const confusedUrl = new URL('./confused.svg', DEMO_URL);
        const squareUrl = new URL('./wc-square.svg', DEMO_URL);
        const expected = [
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
            () => cells().length === expected.length
                && cells().every((value, index) => value === expected[index]),
            'all scalar src-by-referrer combinations publish their expected URLs'
        );
        assert(
            host.querySelector('cem-module-url') === null,
            'transient cem-module-url controls are removed from rendered output'
        );
    },
};


function withSearch(input: URL, name: string, value: string): string {
    const url = new URL(input);
    url.searchParams.set(name, value);
    return url.href;
}

async function waitForCondition(
    condition: () => boolean,
    message: string,
    attempts = 120
): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    assert(element, `expected ${selector}`);
    return element;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) throw new Error(message);
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
