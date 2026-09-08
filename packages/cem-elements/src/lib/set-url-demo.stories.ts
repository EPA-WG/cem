import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-set-url-demo-document';
const DEMO_URL = new URL('../../demo/set-url.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Set the page hash',
    '2. Select the URL write method',
    '3. Conditionally inject a URL writer',
    '4. Set URL from form controls',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Set URL Demo',
    tags: ['test'],
};

export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => sourceLoadedDemo(),
    play: async ({ canvasElement }) => {
        const originalUrl = location.href;
        const host = requiredElement(canvasElement, SOURCE_TAG);
        try {
            await waitForCondition(
                () => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
                'all four set-url samples render from the HTML source'
            );
            assertDeepEqual(sampleLegends(host), [...EXPECTED_LEGENDS], 'set-url sample inventory');

            const hash = sampleByLegend(host, EXPECTED_LEGENDS[0]);
            await waitForCondition(
                () => Array.from(hash.querySelectorAll('button')).some(
                    (button) => normalize(button.textContent ?? '') === '#hash-two'
                ),
                'the hash writer sample finishes rendering'
            );
            buttonByText(hash, '#hash-two').click();
            await waitForText(hash, 'Selected target: #hash-two', 'the button value updates the target slice');
            await waitForText(hash, 'Current hash: #hash-two', 'location.hash writer updates the live reader');
            hash.remove();

            const methods = sampleByLegend(host, EXPECTED_LEGENDS[1]);
            buttonByText(methods, 'history.pushState').click();
            await waitForText(
                methods,
                'Selected method: history.pushState Current hash: #history.pushState',
                'the selected history method controls the writer'
            );
            methods.remove();

            const conditional = sampleByLegend(host, EXPECTED_LEGENDS[2]);
            buttonByText(conditional, 'Set').click();
            await waitForText(
                conditional,
                'Current hash: #conditional-writer',
                'the event conditionally injects the location writer'
            );
            conditional.remove();

            const form = sampleByLegend(host, EXPECTED_LEGENDS[3]);
            const input = requiredElement(form, 'input[type="text"]') as HTMLInputElement;
            input.value = '#form-verified';
            input.dispatchEvent(new Event('input', { bubbles: true }));
            buttonByText(form, 'Set').click();
            await waitForText(
                form,
                'Pending: history.pushState = #form-verified Current hash: #form-verified',
                'form controls supply the method and URL'
            );
        } finally {
            history.replaceState({}, '', originalUrl);
        }
    },
};

function sourceLoadedDemo(): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded set-url demo coverage');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', SOURCE_TAG);
    declaration.setAttribute('src', DEMO_URL.href);
    root.append(declaration, document.createElement(SOURCE_TAG));
    return root;
}


function sampleLegends(host: ParentNode): string[] {
    return Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
        normalize(sample.getAttribute('legend') ?? '')
    );
}

function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    if (!sample) throw new Error(`expected sample ${legend}`);
    return sample;
}

function buttonByText(root: ParentNode, expected: string): HTMLButtonElement {
    const button = Array.from(root.querySelectorAll('button')).find(
        (candidate) => normalize(candidate.textContent ?? '') === expected
    );
    if (!button) throw new Error(`expected ${expected} button`);
    return button;
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForText(root: ParentNode, expected: string, label: string): Promise<void> {
    await waitForCondition(
        () => normalize(root.textContent ?? '').includes(expected),
        `${label}; location=${location.href}; rendered=${normalize(root.textContent ?? '')}`
    );
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 180): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
