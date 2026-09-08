import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-for-each-document';
const DEMO_URL = new URL('../../demo/for-each.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Simple for-each',
    '2. for-each with position()',
    '3. Conditional for-each',
    '4. Nested for-each table',
    '5. for-each with attributes',
    '6. Dynamic table with toggle',
    '7. for-each over payload data',
    '8. for-each over location data',
    '9. for-each over HTTP JSON/XML data',
] as const;

const meta: Meta = {
    title: 'CEM Elements/For Each Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded for-each demo coverage');

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
            () => host.querySelectorAll('cem-demo-element[legend]').length === EXPECTED_LEGENDS.length,
            'all for-each samples render from the HTML source'
        );
        assertDeepEqual(
            Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
                normalize(sample.getAttribute('legend') ?? '')
            ),
            [...EXPECTED_LEGENDS],
            'for-each sample inventory'
        );

        const simple = sampleByLegend(host, EXPECTED_LEGENDS[0]);
        await waitForCondition(
            () => textList(simple, 'cem-loop-simple li').join('|') === 'Apple|Banana|Cherry',
            'simple loop renders all fruit',
            300
        );

        const positioned = sampleByLegend(host, EXPECTED_LEGENDS[1]);
        await waitForCondition(
            () => textList(positioned, 'cem-loop-position article > div > div').join('|') ===
                '1. Red|2. Green|3. Blue',
            'positioned loop renders numbered record fields'
        );

        const conditional = sampleByLegend(host, EXPECTED_LEGENDS[2]);
        await waitForCondition(
            () => conditional.querySelector('input') !== null,
            'conditional loop control renders'
        );
        const conditionalInput = requiredElement(conditional, 'input') as HTMLInputElement;
        conditionalInput.click();
        await waitForCondition(
            () => textList(conditional, 'article > div span').join('|') ===
                '1:First|2:Second|3:Third',
            () => `conditional loop renders after its checkbox changes; checked=${conditionalInput.checked}, spans=${JSON.stringify(textList(conditional, 'span'))}`
        );

        const nested = sampleByLegend(host, EXPECTED_LEGENDS[3]);
        await waitForCondition(
            () => textList(nested, 'tbody td').join('|') ===
                'A1|A2|A3|B1|B2|B3|C1|C2|C3',
            'nested loops render the complete table'
        );

        const attributes = sampleByLegend(host, EXPECTED_LEGENDS[4]);
        await waitForCondition(
            () => normalize(attributes.querySelector('article')?.textContent ?? '')
                .includes('#1 Alice (admin)')
                && normalize(attributes.querySelector('article')?.textContent ?? '')
                    .includes('#3 Charlie (viewer)'),
            'record fields render through loop variables'
        );

        const dynamic = sampleByLegend(host, EXPECTED_LEGENDS[5]);
        await waitForCondition(
            () => dynamic.querySelector('cem-loop-dynamic-table input') !== null,
            'dynamic table control renders'
        );
        const dynamicInput = requiredElement(dynamic, 'cem-loop-dynamic-table input') as HTMLInputElement;
        dynamicInput.click();
        await waitForCondition(
            () => textList(dynamic, 'cem-loop-dynamic-table tbody tr').length === 3
                && normalize(dynamic.querySelector('cem-loop-dynamic-table tbody')?.textContent ?? '')
                    .includes('Widget'),
            'dynamic table loop renders after its checkbox changes'
        );

        const payload = sampleByLegend(host, EXPECTED_LEGENDS[6]);
        await waitForCondition(
            () => textList(payload, 'cem-loop-payload .payload-feed li').join('|') ===
                '1. payload-alpha: Payload Alpha|2. payload-beta: Payload Beta',
            () => `payload loop renders serialized child elements; items=${JSON.stringify(textList(payload, 'cem-loop-payload .payload-feed li'))}`
        );

        const location = sampleByLegend(host, EXPECTED_LEGENDS[7]);
        await waitForCondition(
            () => textList(location, '.location-feed li').join('|') ===
                'topic = feeds|item = payload,resource',
            'location loop renders ordered query parameter entries'
        );

        const http = sampleByLegend(host, EXPECTED_LEGENDS[8]);
        await waitForCondition(
            () => normalize(http.querySelector('output[data-role="json-state"]')?.textContent ?? '') === 'loaded'
                && normalize(http.querySelector('output[data-role="xml-state"]')?.textContent ?? '') === 'loaded'
                && normalize(http.querySelector('.http-json-feed')?.textContent ?? '').includes('beta: loaded')
                && normalize(http.querySelector('.http-xml-feed')?.textContent ?? '').includes('delta: xml-loaded'),
            () => `HTTP JSON and XML loops render loaded resource data; json=${normalize(http.querySelector('output[data-role="json-state"]')?.textContent ?? '')}, xml=${normalize(http.querySelector('output[data-role="xml-state"]')?.textContent ?? '')}, jsonRows=${JSON.stringify(textList(http, '.http-json-feed li'))}, xmlRows=${JSON.stringify(textList(http, '.http-xml-feed li'))}`,
            300
        );
    },
};


function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    if (!sample) throw new Error(`expected sample ${legend}`);
    return sample;
}

function textList(root: ParentNode, selector: string): string[] {
    return Array.from(root.querySelectorAll(selector), (element) => normalize(element.textContent ?? ''));
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForCondition(
    condition: () => boolean,
    message: string | (() => string),
    attempts = 120
): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(typeof message === 'string' ? message : message());
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
