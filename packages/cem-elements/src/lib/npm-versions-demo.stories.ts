import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-npm-versions-demo-document';
const DEMO_URL = new URL('../../demo/npm-versions-demo.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Default to the latest version',
    '2. Preselect a version and show dates',
    '3. Propagate the selected value',
    '4. Override the label slot',
    '5. Synchronize the selected version with the URL',
] as const;

const meta: Meta = {
    title: 'CEM Elements/NPM Versions Demo',
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
                'all five npm-version samples render from the HTML source',
                300
            );
            assertDeepEqual(sampleLegends(host), [...EXPECTED_LEGENDS], 'npm-version sample inventory');

            const defaults = sampleByLegend(host, EXPECTED_LEGENDS[0]);
            await waitForCondition(
                () => defaults.querySelector<HTMLSelectElement>('select')?.value === '0.1.0',
                'the latest fixture version is selected by default'
            );

            const preselected = sampleByLegend(host, EXPECTED_LEGENDS[1]);
            await waitForCondition(
                () => preselected.querySelector<HTMLSelectElement>('select')?.value === '0.0.22'
                    && optionText(preselected, '0.0.22').includes('2024-04-20'),
                `initialversion preselects 0.0.22 and dates are visible; rendered=${normalize(preselected.textContent ?? '')}`
            );

            const propagated = sampleByLegend(host, EXPECTED_LEGENDS[2]);
            await waitForCondition(
                () => propagated.querySelector('select') !== null,
                'the propagated picker finishes rendering'
            );
            choose(propagated, '0.0.25');
            await waitForCondition(
                () => normalize(propagated.querySelector('output')?.textContent ?? '') === '0.0.25'
                    && propagated.querySelector('cem-npm-version-propagated')?.getAttribute('value') === '0.0.25',
                'a selection reaches the wrapper slice and picker value attribute'
            );

            const label = sampleByLegend(host, EXPECTED_LEGENDS[3]);
            await waitForCondition(
                () => normalize(label.querySelector('label')?.textContent ?? '').startsWith('Select a release:'),
                'the projected label replaces the package fallback'
            );
            choose(label, '0.0.21');
            await waitForCondition(
                () => normalize(label.querySelector('output')?.textContent ?? '') === '0.0.21',
                'the slot override does not interfere with value propagation'
            );

            const url = sampleByLegend(host, EXPECTED_LEGENDS[4]);
            buttonByText(url, 'Set URL to 0.0.22').click();
            await waitForCondition(
                () => location.hash === '#version=0.0.22'
                    && url.querySelector<HTMLSelectElement>('select')?.value === '0.0.22',
                'the live location slice preselects the URL version'
            );
            choose(url, '0.1.0');
            await waitForCondition(
                () => Array.from(url.querySelectorAll('output')).some(
                    (output) => normalize(output.textContent ?? '') === '0.1.0'
                ),
                'the wrapper observes the later picker selection'
            );
            buttonByText(url, 'Apply selection to URL').click();
            await waitForCondition(
                () => location.hash === '#version=0.1.0'
                    && Array.from(url.querySelectorAll('output')).some(
                        (output) => normalize(output.textContent ?? '') === '#version=0.1.0'
                    ),
                'Apply selection writes the later picker value to the page hash'
            );
        } finally {
            history.replaceState({}, '', originalUrl);
        }
    },
};

function sourceLoadedDemo(): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded npm versions demo coverage');
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

function requiredSelect(root: ParentNode): HTMLSelectElement {
    const select = root.querySelector('select');
    if (!(select instanceof HTMLSelectElement)) throw new Error('expected version select');
    return select;
}

function choose(root: ParentNode, value: string): void {
    const select = requiredSelect(root);
    select.value = value;
    select.dispatchEvent(new Event('change', { bubbles: true }));
}

function optionText(root: ParentNode, value: string): string {
    const option = Array.from(requiredSelect(root).options).find((candidate) => candidate.value === value);
    if (!option) throw new Error(`expected ${value} option`);
    return normalize(option.textContent ?? '');
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

async function waitForCondition(condition: () => boolean, message: string, attempts = 200): Promise<void> {
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
