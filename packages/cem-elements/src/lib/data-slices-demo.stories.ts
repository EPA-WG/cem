import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-data-slices-document';
const EXPECTED_LEGENDS = [
    'A1. inline slice initialization, change on event',
    'A2. slice initialization, change on event',
    'B. slice event data.',
    '1. slice change on event. 1:1 slice⮂value',
    '2. initial slice value, slice change on event. slice⮂value',
    '3. on input event. slice⮂value',
    '4. initial slice value from attribute',
    '5. slice value computed from event',
    '6. button ignored till change on click.',
    '7. initial slice value from SLICE element',
    '8. multiple slices by SLICE element',
    '9. slice in attribute',
    '10. multiple slices by same field',
    '11. slices and attribute',
    '12. checkbox use',
    '13. Radio group',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Data Slices Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        defineHtmlDemoElementFixture();

        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded data slices demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', new URL('../../demo/data-slices.html', import.meta.url).href);

        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG) as HTMLElement;
        await waitForCondition(
            () => host.querySelectorAll('html-demo-element[legend] article.demo-card').length === EXPECTED_LEGENDS.length,
            'all data-slices samples render from the HTML source',
            300
        );

        const actualLegends = Array.from(host.querySelectorAll('html-demo-element[legend]'), (sample) =>
            normalize(sample.getAttribute('legend') ?? '')
        );
        assertDeepEqual(actualLegends, [...EXPECTED_LEGENDS], 'data-slices sample inventory');

        await verifyInlineCounter(sampleByLegend(host, EXPECTED_LEGENDS[0]));
        await verifyDeclaredCounter(sampleByLegend(host, EXPECTED_LEGENDS[1]));
        await verifyEventPayload(sampleByLegend(host, EXPECTED_LEGENDS[2]));
        await verifyBasicSlice(sampleByLegend(host, EXPECTED_LEGENDS[3]));
        await verifyInitialChangeSlice(sampleByLegend(host, EXPECTED_LEGENDS[4]));
        await verifyInitialInputSlice(sampleByLegend(host, EXPECTED_LEGENDS[5]));
        await verifyAttributeInitialSlice(sampleByLegend(host, EXPECTED_LEGENDS[6]));
        await verifyTransformedSlice(sampleByLegend(host, EXPECTED_LEGENDS[7]));
        await verifyButtonSlice(sampleByLegend(host, EXPECTED_LEGENDS[8]));
        await verifyNestedInitialSlice(sampleByLegend(host, EXPECTED_LEGENDS[9]));
        await verifyMultipleNestedSlices(sampleByLegend(host, EXPECTED_LEGENDS[10]));
        await verifyAttributeSlice(sampleByLegend(host, EXPECTED_LEGENDS[11]));
        await verifyFanoutSlice(sampleByLegend(host, EXPECTED_LEGENDS[12]));
        await verifyAttributeFanoutSlice(sampleByLegend(host, EXPECTED_LEGENDS[13]));
        await verifyCheckboxSlices(sampleByLegend(host, EXPECTED_LEGENDS[14]));
        await verifyRadioSlice(sampleByLegend(host, EXPECTED_LEGENDS[15]));
    },
};

async function verifyInlineCounter(sample: HTMLElement): Promise<void> {
    assertEqual(inputValue(sample, 'input'), '0', 'A1 input-owned fallback initializes to zero');
    assert(normalizedText(requiredElement(sample, 'article.demo-card')).includes('0'), 'A1 exposes its initial value');

    click(sample, 'button:first-of-type');
    await waitForTextValue(sample, 'input', '1', 'A1 increment updates the inline slice');
    click(sample, 'button:nth-of-type(2)');
    await waitForTextValue(sample, 'input', '0', 'A1 decrement updates the inline slice');
}

async function verifyDeclaredCounter(sample: HTMLElement): Promise<void> {
    assertText(sample, 'output', '0', 'A2 declared slice initializes to zero');
    click(sample, 'button:first-of-type');
    await waitForText(sample, 'output', '1', 'A2 click/tap increment updates the slice');
    click(sample, 'button:nth-of-type(2)');
    await waitForText(sample, 'output', '0', 'A2 decrement updates the slice');
}

async function verifyEventPayload(sample: HTMLElement): Promise<void> {
    requiredElement(sample, 'textarea').dispatchEvent(
        new MouseEvent('mousemove', { bubbles: true, clientX: 42, clientY: 17 })
    );
    await waitForCondition(
        () => {
            const textarea = sample.querySelector('textarea');
            const offsetY = textValue(sample, 'p:nth-of-type(3) output');
            const inlineShadow = textarea?.style.boxShadow ?? '';
            return (
                textValue(sample, 'p:nth-of-type(1) output').startsWith('x:') &&
                textValue(sample, 'p:nth-of-type(1) output') !== 'x:' &&
                offsetY !== '' &&
                textValue(sample, 'p:nth-of-type(2) output') === 'mousemove' &&
                inlineShadow !== '' &&
                inlineShadow.includes(`${offsetY}px`) &&
                (textarea ? getComputedStyle(textarea).boxShadow : 'none') !== 'none'
            );
        },
        'B exposes the event payload and computes its coordinates into box-shadow'
    );
}

async function verifyBasicSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'output', '', 'sample 1 starts blank');
    setValueAndDispatch(sample, 'input', 'basic', 'change');
    await waitForText(sample, 'output', 'basic', 'sample 1 default change event updates the slice');
}

async function verifyInitialChangeSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'output', 'B', 'sample 2 exposes its initial slice');
    setValueAndDispatch(sample, 'input', 'changed', 'change');
    await waitForText(sample, 'output', 'changed', 'sample 2 change event updates the slice');
}

async function verifyInitialInputSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'output', 'B', 'sample 3 exposes its initial slice');
    setValueAndDispatch(sample, 'input', 'input event', 'input');
    await waitForText(sample, 'output', 'input event', 'sample 3 input event updates the slice');
}

async function verifyAttributeInitialSlice(sample: HTMLElement): Promise<void> {
    assertText(
        sample,
        'cem-slice-attribute-initial:first-of-type p:nth-of-type(1) output',
        '😁',
        'sample 4 uses the default attribute'
    );
    assertText(
        sample,
        'cem-slice-attribute-initial:last-of-type p:nth-of-type(1) output',
        '🤗',
        'sample 4 preserves a supplied attribute'
    );
    setValueAndDispatch(sample, 'cem-slice-attribute-initial:first-of-type input', 'qqq', 'keyup');
    await waitForText(
        sample,
        'cem-slice-attribute-initial:first-of-type p:nth-of-type(2) output',
        'qqq',
        'sample 4 keyup keeps the final event value'
    );
}

async function verifyTransformedSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'output', 'xB', 'sample 5 exposes the transformed initial slice');
    assertEqual(inputValue(sample, 'input'), 'B', 'sample 5 derives the input value from the slice');
    setValueAndDispatch(sample, 'input', 'C', 'change');
    await waitForText(sample, 'output', 'xC', 'sample 5 transforms the changed value into the slice');
}

async function verifyButtonSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'output', 'anonymous', 'sample 6 starts with the declared nickname');
    click(sample, 'button');
    await waitForText(sample, 'output', 'broccoli', 'sample 6 button supplies the explicit slice value');
}

async function verifyNestedInitialSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'output', '0', 'sample 7 nested slice directive initializes the value');
    click(sample, 'button');
    await waitForText(sample, 'output', '1', 'sample 7 nested click handler updates the value');
}

async function verifyMultipleNestedSlices(sample: HTMLElement): Promise<void> {
    assertText(sample, 'p:nth-of-type(1) output', '0', 'sample 8 initializes clicked');
    assertText(sample, 'p:nth-of-type(2) output', '0', 'sample 8 initializes focused');

    (requiredElement(sample, 'button') as HTMLButtonElement).focus();
    await waitForText(sample, 'p:nth-of-type(2) output', '1', 'sample 8 focus updates focused');
    click(sample, 'button');
    await waitForText(sample, 'p:nth-of-type(1) output', '1', 'sample 8 click updates clicked');
    (requiredElement(sample, 'button') as HTMLButtonElement).blur();
    await waitForText(sample, 'p:nth-of-type(2) output', '0', 'sample 8 blur clears focused');
}

async function verifyAttributeSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'cem-slice-emotion-attribute:first-of-type output', ':)', 'sample 9 keeps the supplied emotion');
    assertText(sample, 'cem-slice-emotion-attribute:last-of-type output', '😃', 'sample 9 supplies the default emotion');
    setValueAndDispatch(sample, 'cem-slice-emotion-attribute:last-of-type input', 'joyful', 'change');
    await waitForText(
        sample,
        'cem-slice-emotion-attribute:last-of-type output',
        'joyful',
        'sample 9 updates the attribute-backed slice'
    );
    assertEqual(
        requiredElement(sample, 'cem-slice-emotion-attribute:last-of-type').getAttribute('emotion'),
        'joyful',
        'sample 9 reflects the changed emotion'
    );
}

async function verifyFanoutSlice(sample: HTMLElement): Promise<void> {
    setValueAndDispatch(sample, 'input', 'mirrored', 'input');
    await waitForCondition(
        () =>
            textValue(sample, 'p:nth-of-type(2) output') === 'mirrored' &&
            textValue(sample, 'p:nth-of-type(3) output') === 'mirrored',
        'sample 10 fans one value out to both slices'
    );
}

async function verifyAttributeFanoutSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'p:nth-of-type(1) output', '😃', 'sample 11 initializes the attribute');
    setValueAndDispatch(sample, 'input', 'grinning', 'change');
    await waitForCondition(
        () =>
            textValue(sample, 'p:nth-of-type(1) output') === 'grinning' &&
            textValue(sample, 'p:nth-of-type(2) output') === 'grinning',
        'sample 11 fans one value out to the attribute and slice'
    );
    assertEqual(
        requiredElement(sample, 'cem-slice-attribute-fanout').getAttribute('emotion'),
        'grinning',
        'sample 11 reflects the changed emotion'
    );
}

async function verifyCheckboxSlices(sample: HTMLElement): Promise<void> {
    assertText(sample, 'p:nth-of-type(1) output', 'V0', 'sample 12 starts with the checked value attribute');
    setCheckedAndDispatch(sample, 'label:nth-of-type(1) input', false);
    await waitForText(sample, 'p:nth-of-type(1) output', '', 'sample 12 clears an unchecked value');
    setCheckedAndDispatch(sample, 'label:nth-of-type(2) input', true);
    await waitForText(sample, 'p:nth-of-type(3) output', 'V1', 'sample 12 resolves slice-value');
    setCheckedAndDispatch(sample, 'label:nth-of-type(3) input', true);
    await waitForText(sample, 'p:nth-of-type(4) output', 'V1', 'sample 12 resolves a variable value');
}

async function verifyRadioSlice(sample: HTMLElement): Promise<void> {
    assertText(sample, 'output', 'V1', 'sample 13 starts with the checked radio value');
    setCheckedAndDispatch(sample, 'label:first-of-type input', true);
    await waitForText(sample, 'output', 'V0', 'sample 13 propagates the newly checked radio value');
}

function defineHtmlDemoElementFixture(): void {
    if (customElements.get('html-demo-element')) return;

    class HtmlDemoElementFixture extends HTMLElement {
        connectedCallback(): void {
            if (this.querySelector(':scope > [slot="demo"]')) return;
            const template = Array.from(this.children).find(
                (child): child is HTMLTemplateElement => child instanceof HTMLTemplateElement
            );
            if (!template) return;
            const demo = document.createElement('div');
            demo.slot = 'demo';
            demo.append(template.content.cloneNode(true));
            this.append(demo);
        }
    }

    customElements.define('html-demo-element', HtmlDemoElementFixture);
}

function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('html-demo-element[legend]')).find(
        (candidate) => normalize(candidate.getAttribute('legend') ?? '') === legend
    );
    assert(sample, `expected sample ${legend}`);
    return sample;
}

function setValueAndDispatch(sample: ParentNode, selector: string, value: string, eventName: string): void {
    const input = requiredElement(sample, selector) as HTMLInputElement | HTMLTextAreaElement;
    input.value = value;
    input.dispatchEvent(new Event(eventName, { bubbles: true }));
}

function setCheckedAndDispatch(sample: ParentNode, selector: string, checked: boolean): void {
    const input = requiredElement(sample, selector) as HTMLInputElement;
    input.checked = checked;
    input.dispatchEvent(new Event('change', { bubbles: true }));
}

function click(sample: ParentNode, selector: string): void {
    (requiredElement(sample, selector) as HTMLButtonElement).click();
}

function inputValue(sample: ParentNode, selector: string): string {
    return (requiredElement(sample, selector) as HTMLInputElement).value;
}

function textValue(sample: ParentNode, selector: string): string {
    return normalize(requiredElement(sample, selector).textContent ?? '');
}

function assertText(sample: ParentNode, selector: string, expected: string, label: string): void {
    assertEqual(textValue(sample, selector), expected, label);
}

async function waitForText(sample: ParentNode, selector: string, expected: string, label: string): Promise<void> {
    await waitForCondition(() => textValue(sample, selector) === expected, label);
}

async function waitForTextValue(
    sample: ParentNode,
    selector: string,
    expected: string,
    label: string
): Promise<void> {
    await waitForCondition(() => inputValue(sample, selector) === expected, label);
}

function normalizedText(element: Element): string {
    return normalize(element.textContent ?? '');
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}

function requiredElement(root: ParentNode, selector: string): Element {
    const element = root.querySelector(selector);
    assert(element, `expected ${selector}`);
    return element;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${String(expected)}, got ${String(actual)}`);
    }
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    const actualJson = JSON.stringify(actual);
    const expectedJson = JSON.stringify(expected);
    if (actualJson !== expectedJson) {
        throw new Error(`${label}: expected ${expectedJson}, got ${actualJson}`);
    }
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function waitForCondition(predicate: () => boolean, message: string, frames = 180): Promise<void> {
    for (let attempt = 0; attempt < frames; attempt += 1) {
        if (predicate()) return;
        await nextFrame();
    }
    throw new Error(`${message} within ${frames} frames`);
}
