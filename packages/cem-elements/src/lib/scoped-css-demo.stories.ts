import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-scoped-css-demo-document';
const DEMO_URL = new URL('../../demo/scoped-css.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Private declaration CSS and ordinary outer cascade',
    '2. Component in a named scope shares default declaration styles with peers in the same scope',
    '3. Style can be scoped explicitly',
    '4. Mixed private and shared styles',
    '5. Invalid and mismatched scopes fail closed',
    '6. Payload style belongs to one instance',
    '7. Declaration styles must be static',
    '8. Fragment template CSS uses the effective produced tag',
    '9. Anonymous declaration CSS uses its generated tag',
    '10. uid-seed stabilizes keyframe names',
    '11. Descendant selectors stay inside the component',
    '12. CSS from an external template fragment',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Scoped CSS Demo',
    tags: ['test'],
};

export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => sourceLoadedDemo(),
    play: async ({ canvasElement }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await waitForCondition(
            () => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
            'all twelve scoped-CSS samples render from the HTML source',
            300
        );
        assertDeepEqual(sampleLegends(host), [...EXPECTED_LEGENDS], 'scoped-CSS sample inventory');

        const privateCss = sampleByLegend(host, EXPECTED_LEGENDS[0]);
        await styleEquals(privateCss, 'cem-css-private button', 'borderTopStyle', 'dashed');
        assertNotEqual(style(privateCss, '[slot="demo"] > button', 'borderTopStyle'), 'dashed', 'private selector does not leak');

        const bare = sampleByLegend(host, EXPECTED_LEGENDS[1]);
        await styleEquals(bare, 'cem-css-shared-bare b', 'color', 'rgb(0, 128, 0)');
        await styleEquals(bare, 'cem-css-shared-peer b', 'color', 'rgb(0, 128, 0)');

        const explicit = sampleByLegend(host, EXPECTED_LEGENDS[2]);
        await styleEquals(explicit, 'cem-css-shared-explicit .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)');
        await styleEquals(explicit, 'cem-css-explicit-peer .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)');

        const mixed = sampleByLegend(host, EXPECTED_LEGENDS[3]);
        await styleEquals(mixed, 'cem-css-mixed .sample-mixed', 'borderTopColor', 'rgb(0, 0, 255)');
        await styleEquals(mixed, 'cem-css-mixed-peer .sample-mixed-shared', 'color', 'rgb(255, 0, 0)');

        const invalid = sampleByLegend(host, EXPECTED_LEGENDS[4]);
        await waitForCondition(
            () => style(invalid, 'cem-css-unscoped-explicit .must-not-apply', 'color') !== 'rgb(255, 0, 0)'
                && style(invalid, 'cem-css-invalid-declaration .sample-invalid-declaration', 'color') === 'rgb(0, 0, 255)',
            'invalid scopes fail closed while the fallback private style remains'
        );

        const payload = sampleByLegend(host, EXPECTED_LEGENDS[5]);
        await styleEquals(payload, 'cem-css-instance:first-of-type button', 'borderTopColor', 'rgb(0, 0, 255)');
        await styleEquals(payload, 'cem-css-instance:last-of-type button', 'borderTopColor', 'rgb(255, 0, 0)');

        const dynamic = sampleByLegend(host, EXPECTED_LEGENDS[6]);
        await waitForCondition(
            () => dynamic.querySelector('cem-css-dynamic style') === null
                && style(dynamic, 'cem-css-dynamic .must-not-apply', 'color') !== 'rgb(255, 0, 0)',
            'dynamic declaration styles are rejected'
        );

        await styleEquals(
            sampleByLegend(host, EXPECTED_LEGENDS[7]),
            'cem-css-fragment .sample-fragment',
            'backgroundColor',
            'rgb(254, 243, 199)'
        );
        await styleEquals(
            sampleByLegend(host, EXPECTED_LEGENDS[8]),
            '.sample-anonymous',
            'color',
            'rgb(238, 130, 238)'
        );

        const keyframes = sampleByLegend(host, EXPECTED_LEGENDS[9]);
        await waitForCondition(
            () => style(keyframes, 'cem-css-keyframes [part~="indicator"]', 'animationName').includes('seeded-pulse')
                && style(keyframes, 'cem-css-keyframes [part~="indicator"]', 'animationName').includes('udemoz2fcssz2fkeyframes'),
            'uid-seed stabilizes the rewritten animation name'
        );

        const descendant = sampleByLegend(host, EXPECTED_LEGENDS[10]);
        await styleEquals(descendant, '[slot="demo"] label', 'color', 'rgb(0, 128, 0)');
        await styleEquals(descendant, '[slot="demo"] b', 'color', 'rgb(0, 0, 139)');

        const external = sampleByLegend(host, EXPECTED_LEGENDS[11]);
        await waitForCondition(
            () => normalize(external.querySelector('cem-css-external-fragment')?.textContent ?? '').includes('projected external template')
                && style(external, '.external-scoped-item', 'backgroundColor') === 'rgb(254, 243, 199)',
            'external fragment content keeps its scoped style artifact'
        );
    },
};

function sourceLoadedDemo(): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', 'source-loaded scoped CSS demo coverage');
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

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

function style(root: ParentNode, selector: string, property: keyof CSSStyleDeclaration): string {
    const element = root.querySelector<HTMLElement>(selector);
    return element ? String(getComputedStyle(element)[property]) : '';
}

async function styleEquals(
    root: ParentNode,
    selector: string,
    property: keyof CSSStyleDeclaration,
    expected: string
): Promise<void> {
    await waitForCondition(
        () => style(root, selector, property) === expected,
        `${selector} ${String(property)} should be ${expected}`
    );
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 200): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}

function assertNotEqual(actual: unknown, unexpected: unknown, label: string): void {
    if (actual === unexpected) throw new Error(`${label}: did not expect ${String(unexpected)}`);
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
