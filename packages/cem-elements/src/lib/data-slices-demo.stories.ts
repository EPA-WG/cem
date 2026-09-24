import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';

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
    for (const owner of sample.querySelectorAll<HTMLElement>('cem-element, cem-slice-attribute-initial')) {
        assertDeepEqual(cemDiagnosticCodes(owner), [], 'sample 4 has no declaration or instance diagnostics');
    }
    assertValues(sample, ['😁', '🤗'], ['😁', '', '🤗', ''], 'sample 4 attribute fallbacks');
    setValueAndDispatch(sample, 'cem-slice-attribute-initial:first-of-type input', 'qqq', 'keyup');
    await waitForValues(sample, ['qqq', '🤗'], ['😁', 'qqq', '🤗', ''], 'sample 4 first instance changes independently');
    setValueAndDispatch(sample, 'cem-slice-attribute-initial:last-of-type input', 'second', 'keyup');
    await waitForValues(sample, ['qqq', 'second'], ['😁', 'qqq', '🤗', 'second'], 'sample 4 second instance changes independently');
    setValueAndDispatch(sample, 'cem-slice-attribute-initial:first-of-type input', '', 'keyup');
    await waitForValues(sample, ['', 'second'], ['😁', '', '🤗', 'second'], 'sample 4 empty slice is distinct from an absent slice');
    assertEqual(requiredElement(sample, 'cem-slice-attribute-initial:first-of-type').getAttribute('a'), '😁', 'sample 4 keeps the default attribute');
    assertEqual(requiredElement(sample, 'cem-slice-attribute-initial:last-of-type').getAttribute('a'), '🤗', 'sample 4 keeps the supplied attribute');
}

async function verifyTransformedSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, ['B'], ['xB'], 'sample 5 initial transformed value');
    setValueAndDispatch(sample, 'input', 'C', 'change');
    await waitForValues(sample, ['C'], ['xC'], 'sample 5 transforms the event value');
    setValueAndDispatch(sample, 'input', '', 'change');
    await waitForValues(sample, [''], ['x'], 'sample 5 transforms empty input');
}

async function verifyButtonSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, ['anonymous'], ['anonymous'], 'sample 6 ignores the button until click');
    click(sample, 'button');
    await waitForValues(sample, ['broccoli'], ['broccoli'], 'sample 6 button updates input and slice');
}

async function verifyNestedInitialSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, [], ['0'], 'sample 7 nested default');
    click(sample, 'button');
    await waitForValues(sample, [], ['1'], 'sample 7 click increments');
    requiredElement(sample, 'button').dispatchEvent(new Event('tap', { bubbles: true }));
    await waitForValues(sample, [], ['2'], 'sample 7 tap increments');
}

async function verifyMultipleNestedSlices(sample: HTMLElement): Promise<void> {
    assertValues(sample, [], ['0', '0'], 'sample 8 initial slices');
    (requiredElement(sample, 'button') as HTMLButtonElement).focus();
    await waitForValues(sample, [], ['0', '1'], 'sample 8 focus changes only focused');
    click(sample, 'button');
    await waitForValues(sample, [], ['1', '1'], 'sample 8 click changes only clicked');
    requiredElement(sample, 'button').dispatchEvent(new Event('tap', { bubbles: true }));
    await waitForValues(sample, [], ['2', '1'], 'sample 8 tap changes only clicked');
    (requiredElement(sample, 'button') as HTMLButtonElement).blur();
    await waitForValues(sample, [], ['2', '0'], 'sample 8 blur changes only focused');
}

async function verifyAttributeSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, [':)', '😃'], [':)', '😃'], 'sample 9 supplied and default attributes');
    setValueAndDispatch(sample, 'cem-slice-emotion-attribute:first-of-type input', 'supplied change', 'change');
    await waitForValues(sample, ['supplied change', '😃'], ['supplied change', '😃'], 'sample 9 first instance updates independently');
    assertEqual(requiredElement(sample, 'cem-slice-emotion-attribute:first-of-type').getAttribute('emotion'), 'supplied change', 'sample 9 first reflection');
    setValueAndDispatch(sample, 'cem-slice-emotion-attribute:last-of-type input', 'joyful', 'change');
    await waitForValues(sample, ['supplied change', 'joyful'], ['supplied change', 'joyful'], 'sample 9 second instance updates independently');
    assertEqual(requiredElement(sample, 'cem-slice-emotion-attribute:last-of-type').getAttribute('emotion'), 'joyful', 'sample 9 second reflection');
    setValueAndDispatch(sample, 'cem-slice-emotion-attribute:first-of-type input', '', 'change');
    await waitForValues(sample, ['', 'joyful'], ['', 'joyful'], 'sample 9 clears one attribute without affecting the other');
    assertEqual(requiredElement(sample, 'cem-slice-emotion-attribute:first-of-type').getAttribute('emotion'), '', 'sample 9 reflects an empty attribute');
}

async function verifyFanoutSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, [''], ['', ''], 'sample 10 initial fanout');
    setValueAndDispatch(sample, 'input', 'mirrored', 'input');
    await waitForValues(sample, ['mirrored'], ['mirrored', 'mirrored'], 'sample 10 updates both slices');
    setValueAndDispatch(sample, 'input', '', 'input');
    await waitForValues(sample, [''], ['', ''], 'sample 10 clears both slices');
}

async function verifyAttributeFanoutSlice(sample: HTMLElement): Promise<void> {
    assertValues(sample, ['😃'], ['😃', ''], 'sample 11 initial attribute and absent slice');
    setValueAndDispatch(sample, 'input', 'grinning', 'change');
    await waitForValues(sample, ['grinning'], ['grinning', 'grinning'], 'sample 11 updates attribute and slice');
    assertEqual(requiredElement(sample, 'cem-slice-attribute-fanout').getAttribute('emotion'), 'grinning', 'sample 11 reflects changed emotion');
    setValueAndDispatch(sample, 'input', '', 'change');
    await waitForValues(sample, [''], ['', ''], 'sample 11 clears attribute and slice');
    assertEqual(requiredElement(sample, 'cem-slice-attribute-fanout').getAttribute('emotion'), '', 'sample 11 reflects empty emotion');
}

async function verifyCheckboxSlices(sample: HTMLElement): Promise<void> {
    assertCheckedValues(sample, [true, false, false], ['V0', '', ''], 'sample 12 initial checked state');
    for (const [index, checked, outputs] of [
        [1, [false, false, false], ['', '', '']],
        [1, [true, false, false], ['V0', '', '']],
        [2, [true, true, false], ['V0', 'V1', '']],
        [3, [true, true, true], ['V0', 'V1', 'V1']],
        [2, [true, false, true], ['V0', '', 'V1']],
        [3, [true, false, false], ['V0', '', '']],
    ] as const) {
        click(sample, `label:nth-of-type(${index}) input`);
        await waitForCondition(() => sameValues(checkedValues(sample), checked)
            && sameValues(outputValues(sample), outputs), 'sample 12 keeps checkbox state and all slices together');
    }
}

async function verifyRadioSlice(sample: HTMLElement): Promise<void> {
    assertCheckedValues(sample, [false, true], ['V1'], 'sample 13 initial radio choice');
    click(sample, 'label:first-of-type input');
    await waitForCondition(() => sameValues(checkedValues(sample), [true, false])
        && sameValues(outputValues(sample), ['V0']), 'sample 13 selects only V0');
    click(sample, 'label:last-of-type input');
    await waitForCondition(() => sameValues(checkedValues(sample), [false, true])
        && sameValues(outputValues(sample), ['V1']), 'sample 13 returns to only V1');
}

function outputValues(sample: ParentNode): string[] {
    return Array.from(sample.querySelectorAll('output'), output => normalize(output.textContent ?? ''));
}

function inputValues(sample: ParentNode): string[] {
    return Array.from(sample.querySelectorAll<HTMLInputElement>('input'), input => input.value);
}

function checkedValues(sample: ParentNode): boolean[] {
    return Array.from(sample.querySelectorAll<HTMLInputElement>('input'), input => input.checked);
}

function sameValues(actual: readonly unknown[], expected: readonly unknown[]): boolean {
    return actual.length === expected.length && actual.every((value, index) => value === expected[index]);
}

function assertValues(sample: ParentNode, inputs: readonly string[], outputs: readonly string[], label: string): void {
    assertDeepEqual(inputValues(sample), inputs, `${label}: inputs`);
    assertDeepEqual(outputValues(sample), outputs, `${label}: outputs`);
}

function assertCheckedValues(sample: ParentNode, checked: readonly boolean[], outputs: readonly string[], label: string): void {
    assertDeepEqual(checkedValues(sample), checked, `${label}: checked`);
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

function click(sample: ParentNode, selector: string): void {
    (requiredElement(sample, selector) as HTMLElement).click();
}

function inputValue(sample: ParentNode, selector: string): string {
    return (requiredElement(sample, selector) as HTMLInputElement).value;
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
