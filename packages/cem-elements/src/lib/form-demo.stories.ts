import type { Meta, StoryObj } from '@storybook/web-components-vite';

const SOURCE_TAG = 'story-form-demo-document';
const DEMO_URL = new URL('../../demo/form.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Simple validation',
    '2. Form lifecycle',
    '3. Native control validity message',
    '4. Form custom validity message',
    '5. DCE as a form input',
] as const;

const meta: Meta = {
    title: 'CEM Elements/Form Demo',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        defineHtmlDemoElementFixture();

        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded form demo coverage');

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
            () => host.querySelectorAll('html-demo-element[legend] article').length === EXPECTED_LEGENDS.length,
            'all five form samples render from the HTML source',
            300
        );

        assertDeepEqual(
            Array.from(host.querySelectorAll('html-demo-element[legend]'), (sample) =>
                normalize(sample.getAttribute('legend') ?? '')
            ),
            [...EXPECTED_LEGENDS],
            'form sample inventory'
        );

        await verifySimpleValidation(sampleByLegend(host, EXPECTED_LEGENDS[0]));
        await verifyFormLifecycle(sampleByLegend(host, EXPECTED_LEGENDS[1]));
        await verifyNativeMessage(sampleByLegend(host, EXPECTED_LEGENDS[2]));
        await verifyFormMessage(sampleByLegend(host, EXPECTED_LEGENDS[3]));
        await verifyFormAssociatedDce(sampleByLegend(host, EXPECTED_LEGENDS[4]));
    },
};

async function verifySimpleValidation(sample: HTMLElement): Promise<void> {
    setValueAndDispatch(requiredControl(sample, 'input[name="username"]'), 'long-username');
    await waitForCondition(
        () => sample.querySelector('input[name="password"]') !== null && buttonText(sample).includes('Sign in'),
        'a long username reveals the password step'
    );

    setValueAndDispatch(requiredControl(sample, 'input[name="password"]'), 'secret');
    await waitForText(sample, 'form > p:nth-of-type(2) output', 'true', 'both values satisfy simple validation');
    assertText(sample, 'form > p:nth-of-type(3) output', '', 'the simple form message clears');
}

async function verifyFormLifecycle(sample: HTMLElement): Promise<void> {
    setValueAndDispatch(requiredControl(sample, 'input[name="username"]'), 'long-username');
    click(requiredElement(sample, 'input[value="sms"]'));
    await waitForCondition(
        () => normalize(requiredElement(sample, 'fieldset').textContent ?? '').includes('Message and data rates may apply.'),
        'the SMS lifecycle branch renders its warning'
    );

    click(requiredElement(sample, 'input[value="password"]'));
    await waitForCondition(
        () => sample.querySelector('input[name="password"]') !== null,
        'the password lifecycle branch renders its control'
    );
    setValueAndDispatch(requiredControl(sample, 'input[name="password"]'), 'secret');
    await waitForText(sample, 'form > p:nth-of-type(4) output', 'true', 'the lifecycle form becomes valid');
    assertText(sample, 'form > p:nth-of-type(3) output', 'password', 'form data records the confirmation choice');
    assertText(sample, 'form > p:first-of-type output', '', 'the username control message clears');
}

async function verifyNativeMessage(sample: HTMLElement): Promise<void> {
    const email = requiredControl(sample, 'input[name="email"]');
    setValueAndDispatch(email, '');
    await waitForCondition(
        () => textValue(sample, 'form > p:nth-of-type(1) output') === 'false'
            && textValue(sample, 'form > p:nth-of-type(2) output') !== '',
        'clearing the required control exposes its native validation message'
    );
    assertEqual(
        textValue(sample, 'form > p:nth-of-type(2) output'),
        email.validationMessage,
        'validation state mirrors the native control message'
    );
}

async function verifyFormMessage(sample: HTMLElement): Promise<void> {
    const email = requiredControl(sample, 'input[name="email"]');
    setValueAndDispatch(email, 'abc');
    await waitForText(sample, 'form > p:first-of-type output', 'abc', 'the form sample exposes its current slice');
    assertText(sample, 'form > p:nth-of-type(2) output', '3', 'the form sample exposes the current length');
    assertText(sample, 'form > p:nth-of-type(3) output', 'false', 'three characters keep the form invalid');
    assertText(
        sample,
        'form > p:nth-of-type(4) output',
        'Use more than 3 characters',
        'the form exposes its custom message'
    );

    setValueAndDispatch(requiredControl(sample, 'input[name="email"]'), 'abcd');
    await waitForText(sample, 'form > p:nth-of-type(3) output', 'true', 'four characters satisfy the form');
    assertText(sample, 'form > p:nth-of-type(4) output', '', 'the custom form message clears');
}

async function verifyFormAssociatedDce(sample: HTMLElement): Promise<void> {
    await waitForCondition(
        () => sample.querySelectorAll('cem-form-fruit-choice button').length === 6,
        'both form-associated DCE choices render their options'
    );

    chooseFruit(sample, 0, 'Apple');
    chooseFruit(sample, 1, 'Banana');
    await waitForText(sample, 'form > p:nth-of-type(2) output', 'false', 'different fruit choices remain invalid');
    assertText(sample, 'form > p:nth-of-type(3) output', 'Choose the same fruit', 'the mismatch message is visible');

    chooseFruit(sample, 1, 'Apple');
    await waitForText(sample, 'form > p:nth-of-type(2) output', 'true', 'matching DCE values satisfy the form');
    assertDeepEqual(
        Array.from(sample.querySelectorAll('form > p:first-of-type output'), (output) => normalize(output.textContent ?? '')),
        ['🍏', '🍏'],
        'both form-associated values reach parent form data'
    );
    assertText(sample, 'form > p:nth-of-type(3) output', '', 'the fruit mismatch message clears');
}

function chooseFruit(sample: ParentNode, index: number, label: string): void {
    const choices = sample.querySelectorAll<HTMLElement>('cem-form-fruit-choice');
    const choice = choices.item(index);
    if (!choice) throw new Error(`expected fruit choice ${index + 1}`);
    const button = Array.from(choice.querySelectorAll<HTMLButtonElement>('button')).find(
        (candidate) => normalize(candidate.textContent ?? '') === label
    );
    if (!button) throw new Error(`expected ${label} option in fruit choice ${index + 1}`);
    click(button);
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
    if (!sample) throw new Error(`expected sample ${legend}`);
    return sample;
}

function requiredControl(root: ParentNode, selector: string): HTMLInputElement {
    const element = requiredElement(root, selector);
    if (!(element instanceof HTMLInputElement)) throw new Error(`expected input ${selector}`);
    return element;
}

function setValueAndDispatch(control: HTMLInputElement, value: string): void {
    control.value = value;
    control.dispatchEvent(new Event('input', { bubbles: true }));
}

function click(element: HTMLElement): void {
    element.click();
}

function buttonText(root: ParentNode): string[] {
    return Array.from(root.querySelectorAll('button'), (button) => normalize(button.textContent ?? ''));
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

async function waitForText(root: ParentNode, selector: string, expected: string, label: string): Promise<void> {
    await waitForCondition(() => textValue(root, selector) === expected, label);
}

async function waitForCondition(condition: () => boolean, message: string, attempts = 160): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}

function assertText(root: ParentNode, selector: string, expected: string, label: string): void {
    assertEqual(textValue(root, selector), expected, label);
}

function textValue(root: ParentNode, selector: string): string {
    return normalize(requiredElement(root, selector).textContent ?? '');
}

function assertDeepEqual(actual: readonly string[], expected: readonly string[], label: string): void {
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function assertEqual(actual: string, expected: string, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`);
    }
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
