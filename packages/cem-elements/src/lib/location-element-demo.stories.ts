import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-location-element-demo-document';
const DEMO_URL = new URL('../../demo/location-element.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Window location live update',
    '2. Window location initial read',
    '3. External URL from href',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Location Element Demo',
    tags: ['test'],
};

export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => sourceLoadedDemo(SOURCE_TAG, DEMO_URL),
    play: async ({ canvasElement }) => {
        const originalUrl = location.href;
        const host = requiredElement(canvasElement, SOURCE_TAG);
        try {
            await waitForCondition(
                () => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
                'all three location-element samples render from the HTML source'
            );
            assertDeepEqual(sampleLegends(host), [...EXPECTED_LEGENDS], 'location sample inventory');

            const live = sampleByLegend(host, EXPECTED_LEGENDS[0]);
            await waitForCondition(
                () => Array.from(live.querySelectorAll('button')).some(
                    (button) => normalize(button.textContent ?? '') === 'history.pushState'
                ),
                'the live location sample finishes rendering'
            );
            buttonByText(live, 'history.pushState').click();
            await waitForCondition(
                () => definitionValue(live, 'hash') === '#checked'
                    && normalize(live.querySelector('ul')?.textContent ?? '').includes('mode = history.pushState')
                    && normalize(live.querySelector('ul')?.textContent ?? '').includes('tag = one,two'),
                'the live reader observes the history write and repeated parameters'
            );

            const initial = sampleByLegend(host, EXPECTED_LEGENDS[1]);
            await waitForCondition(
                () => definitionValue(initial, 'source') === 'window'
                    && definitionValue(initial, 'origin') === location.origin,
                'the initial reader publishes current URL fields'
            );

            const external = sampleByLegend(host, EXPECTED_LEGENDS[2]);
            await waitForCondition(
                () => definitionValue(external, 'hostname') === 'my.example'
                    && definitionValue(external, 'pathname') === '/docs'
                    && definitionValue(external, 'hash') === '#details'
                    && normalize(external.querySelector('ul')?.textContent ?? '').includes('b = 2,3'),
                'the href reader parses an external URL and repeated parameters'
            );
        } finally {
            history.replaceState({}, '', originalUrl);
        }
    },
};

function sourceLoadedDemo(tag: string, url: URL): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded location-element demo coverage');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('src', url.href);
    root.append(declaration, document.createElement(tag));
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

function definitionValue(root: ParentNode, term: string): string {
    const dt = Array.from(root.querySelectorAll('dt')).find(
        (candidate) => normalize(candidate.textContent ?? '') === term
    );
    const dd = dt?.nextElementSibling;
    if (!(dd instanceof HTMLElement) || dd.localName !== 'dd') throw new Error(`expected ${term} value`);
    return normalize(dd.textContent ?? '');
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
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
