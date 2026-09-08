import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-dom-merge-document';
const DEMO_URL = new URL('../../demo/dom-merge.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Textarea word count',
    '2. Input word and character count',
] as const;

const meta: Meta = {
    title: 'CEM Elements/DOM Merge Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded DOM merge demo coverage');

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
            'both DOM merge samples render from the HTML source'
        );

        assertDeepEqual(
            Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
                normalize(sample.getAttribute('legend') ?? '')
            ),
            [...EXPECTED_LEGENDS],
            'DOM merge sample inventory'
        );

        const textareaSample = sampleByLegend(host, EXPECTED_LEGENDS[0]);
        const textarea = requiredElement(textareaSample, 'textarea') as HTMLTextAreaElement;
        textarea.focus();
        setValueAndDispatch(textarea, 'one two three');
        await waitForText(textareaSample, 'form > p strong', '3', 'textarea word count updates');
        assert(
            requiredElement(textareaSample, 'textarea') === textarea,
            'DOM merge preserves the edited textarea node'
        );
        assert(document.activeElement === textarea, 'DOM merge preserves textarea focus');

        const inputSample = sampleByLegend(host, EXPECTED_LEGENDS[1]);
        const input = requiredElement(inputSample, 'input') as HTMLInputElement;
        input.focus();
        setValueAndDispatch(input, 'two words');
        await waitForText(inputSample, 'output', 'two words', 'input value updates the displayed slice');
        assertText(inputSample, 'form > p:first-of-type strong', '9', 'character count follows the input slice');
        assertText(inputSample, 'form > p:nth-of-type(2) strong', '2', 'word count follows the input slice');
        assert(requiredElement(inputSample, 'input') === input, 'DOM merge preserves the edited input node');
        assert(document.activeElement === input, 'DOM merge preserves input focus');
    },
};


function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    if (!sample) throw new Error(`expected sample ${legend}`);
    return sample;
}

function setValueAndDispatch(control: HTMLInputElement | HTMLTextAreaElement, value: string): void {
    control.value = value;
    control.dispatchEvent(new Event('input', { bubbles: true }));
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForText(root: ParentNode, selector: string, expected: string, label: string): Promise<void> {
    await waitForCondition(() => textValue(root, selector) === expected, label);
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 120): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}

function assertText(root: ParentNode, selector: string, expected: string, label: string): void {
    const actual = textValue(root, selector);
    if (actual !== expected) throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
}

function textValue(root: ParentNode, selector: string): string {
    return normalize(requiredElement(root, selector).textContent ?? '');
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) throw new Error(message);
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
