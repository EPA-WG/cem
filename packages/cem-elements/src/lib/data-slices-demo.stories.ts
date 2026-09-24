import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { whenCemSourceRendered } from '../../.storybook/preview.js';

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
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded data slices demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', new URL('../../demo/data-slices.html', import.meta.url).href);

        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG) as HTMLElement;
        await whenCemSourceRendered(host);
        for (const legend of EXPECTED_LEGENDS) {
            const instances = legend === '4. initial slice value from attribute'
                || legend === '9. slice in attribute' ? 2 : 1;
            assertEqual(sampleByLegend(host, legend).querySelectorAll('article.demo-card').length,
                instances, `${legend}: all authored instances render`);
        }

        const actualLegends = Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
            normalize(sample.getAttribute('legend') ?? '')
        );
        assertDeepEqual(actualLegends, [...EXPECTED_LEGENDS], 'data-slices sample inventory');

        const verifiers = [
            verifyInlineCounter, verifyDeclaredCounter, verifyEventPayload,
            verifyBasicSlice, verifyInitialChangeSlice, verifyInitialInputSlice,
            verifyAttributeInitialSlice, verifyTransformedSlice, verifyButtonSlice,
            verifyNestedInitialSlice, verifyMultipleNestedSlices, verifyAttributeSlice,
            verifyFanoutSlice, verifyAttributeFanoutSlice, verifyCheckboxSlices, verifyRadioSlice,
        ];
        for (const [index, verify] of verifiers.entries()) {
            const legend = EXPECTED_LEGENDS[index];
            await step(legend, () => verify(sampleByLegend(host, legend)));
        }
    },
};

async function verifyInlineCounter(sample: HTMLElement): Promise<void> {
    const count = () => normalize(Array.from(requiredElement(sample, 'article.demo-card').childNodes)
        .filter(node => node.nodeType === Node.TEXT_NODE).map(node => node.textContent ?? '').join(''));
    assertValues(sample, ['0'], [], 'A1 initial input');
    assertEqual(count(), '0', 'A1 initial displayed count');
    for (const [selector, value] of [['button:first-of-type', '1'], ['button:nth-of-type(2)', '0']]) {
        click(sample, selector);
        await waitForCondition(() => inputValue(sample, 'input') === value && count() === value,
            'A1 updates input and displayed count together');
    }
    setValueAndDispatch(sample, 'input', '5', 'change');
    await waitForCondition(() => inputValue(sample, 'input') === '5' && count() === '5',
        'A1 accepts a directly edited count');
}

async function verifyDeclaredCounter(sample: HTMLElement): Promise<void> {
    assertValues(sample, ['0'], ['0'], 'A2 initial count');
    click(sample, 'button:first-of-type');
    await waitForValues(sample, ['1'], ['1'], 'A2 click increments');
    requiredElement(sample, 'button:first-of-type').dispatchEvent(new Event('tap', { bubbles: true }));
    await waitForValues(sample, ['2'], ['2'], 'A2 tap increments');
    click(sample, 'button:nth-of-type(2)');
    await waitForValues(sample, ['1'], ['1'], 'A2 decrement');
}

async function verifyEventPayload(sample: HTMLElement): Promise<void> {
    assertDeepEqual(outputValues(sample), ['', '', ''], 'B starts without event metadata');
    assertEqual((requiredElement(sample, 'textarea') as HTMLTextAreaElement).value, '', 'B initial textarea');
    for (const [type, x, y] of [['mousemove', 42, 17], ['click', 21, 9]] as const) {
        const textarea = requiredElement(sample, 'textarea') as HTMLTextAreaElement;
        const coordinates = { pageX: Number.NaN, offsetX: Number.NaN, offsetY: Number.NaN };
        textarea.addEventListener(type, event => {
            const mouse = event as MouseEvent;
            Object.assign(coordinates, { pageX: mouse.pageX, offsetX: mouse.offsetX, offsetY: mouse.offsetY });
        }, { once: true });
        textarea.dispatchEvent(new MouseEvent(type, { bubbles: true, clientX: x, clientY: y }));
        assert(Number.isFinite(coordinates.pageX), 'B receives the dispatched pointer event');
        const expected = coordinates;
        await waitForCondition(() => {
            const current = requiredElement(sample, 'textarea') as HTMLTextAreaElement;
            return sameValues(outputValues(sample), [`x:${expected.pageX}`, type, String(expected.offsetY)])
                && current.style.boxShadow.includes(`${expected.offsetX}px ${expected.offsetY}px`)
                && getComputedStyle(current).boxShadow !== 'none';
        }, `B displays exact ${type} payload and both shadow offsets`);
    }
}

async function verifyBasicSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, [''], [''], 'sample 1 starts blank');
    setValueAndDispatch(sample, 'input', 'pending', 'input');
    await whenCemSourceRendered(sample);
    assertValues(sample, ['pending'], [''], 'sample 1 waits for change');
    setValueAndDispatch(sample, 'input', 'basic', 'change');
    await waitForValues(sample, ['basic'], ['basic'], 'sample 1 change commits');
    setValueAndDispatch(sample, 'input', '', 'change');
    await waitForValues(sample, [''], [''], 'sample 1 clears');
}

async function verifyInitialChangeSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, ['B'], ['B'], 'sample 2 initial value');
    setValueAndDispatch(sample, 'input', 'pending', 'input');
    await whenCemSourceRendered(sample);
    assertValues(sample, ['pending'], ['B'], 'sample 2 waits for change');
    setValueAndDispatch(sample, 'input', 'changed', 'change');
    await waitForValues(sample, ['changed'], ['changed'], 'sample 2 change commits');
    setValueAndDispatch(sample, 'input', '', 'change');
    await waitForValues(sample, [''], [''], 'sample 2 clears without restoring the default');
}

async function verifyInitialInputSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, ['B'], ['B'], 'sample 3 initial value');
    setValueAndDispatch(sample, 'input', 'input event', 'input');
    await waitForValues(sample, ['input event'], ['input event'], 'sample 3 input commits');
    setValueAndDispatch(sample, 'input', '', 'input');
    await waitForValues(sample, [''], [''], 'sample 3 clears');
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


function outputValues(sample: ParentNode): string[] {
    return Array.from(sample.querySelectorAll('output'), output => normalize(output.textContent ?? ''));
}

function inputValues(sample: ParentNode): string[] {
    return Array.from(sample.querySelectorAll<HTMLInputElement>('input'), input => input.value);
}

function sameValues(actual: readonly unknown[], expected: readonly unknown[]): boolean {
    return actual.length === expected.length && actual.every((value, index) => value === expected[index]);
}

function assertValues(sample: ParentNode, inputs: readonly string[], outputs: readonly string[], label: string): void {
    assertDeepEqual(inputValues(sample), inputs, `${label}: inputs`);
    assertDeepEqual(outputValues(sample), outputs, `${label}: outputs`);
}

async function waitForValues(sample: ParentNode, inputs: readonly string[], outputs: readonly string[], label: string): Promise<void> {
    await waitForCondition(() => sameValues(inputValues(sample), inputs)
        && sameValues(outputValues(sample), outputs), label);
}

function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
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

function assertDeepEqual(actual: readonly unknown[], expected: readonly unknown[], label: string): void {
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
