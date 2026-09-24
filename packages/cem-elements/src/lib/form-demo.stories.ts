import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemRendered, whenCemSourceRendered } from '../../.storybook/preview.js';

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
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded form demo coverage');

        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', DEMO_URL.href);

        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        await whenCemSourceRendered(host);
        assertEqual(
            host.querySelectorAll('cem-demo-element[legend] article').length,
            EXPECTED_LEGENDS.length,
            'all five form samples render from the HTML source'
        );

        assertDeepEqual(
            Array.from(host.querySelectorAll('cem-demo-element[legend]'), (sample) =>
                normalize(sample.getAttribute('legend') ?? '')
            ),
            [...EXPECTED_LEGENDS],
            'form sample inventory'
        );

        await step(EXPECTED_LEGENDS[0], () => verifySimpleValidation(sampleByLegend(host, EXPECTED_LEGENDS[0])));
        await step(EXPECTED_LEGENDS[1], () => verifyFormLifecycle(sampleByLegend(host, EXPECTED_LEGENDS[1])));
        await step(EXPECTED_LEGENDS[2], () => verifyNativeMessage(sampleByLegend(host, EXPECTED_LEGENDS[2])));
        await step(EXPECTED_LEGENDS[3], () => verifyFormMessage(sampleByLegend(host, EXPECTED_LEGENDS[3])));
        await step(EXPECTED_LEGENDS[4], () => verifyFormAssociatedDce(sampleByLegend(host, EXPECTED_LEGENDS[4])));
    },
};

async function verifySimpleValidation(sample: HTMLElement): Promise<void> {
    const username = requiredControl(sample, 'input[name="username"]');
    click(requiredElement(sample, 'button'));
    if (username.validity.valid || !username.validationMessage) throw new Error('empty Next must expose native validation');
    setValueAndDispatch(username, 'short');
    await waitForCondition(() => normalize(sample.textContent ?? '').includes('short'), 'short username is captured');
    click(requiredElement(sample, 'button'));
    setValueAndDispatch(requiredControl(sample, 'input[name="username"]'), 'long-username');
    await waitForCondition(() => normalize(sample.textContent ?? '').includes('long-username'), 'username is captured');
    if (sample.querySelector('input[name="password"]')) throw new Error('typing alone must not advance the step');
    await waitForText(sample, 'form > p:nth-of-type(2) output', 'false', 'a missing password keeps Next from submitting');
    click(requiredElement(sample, 'button'));
    await waitForCondition(
        () => sample.querySelector('input[name="password"]') !== null && buttonNames(sample).includes('Sign in'),
        'a long username reveals the password step'
    );

    setValueAndDispatch(requiredControl(sample, 'input[name="password"]'), 'secret');
    await waitForText(sample, 'form > p:nth-of-type(2) output', 'true', 'both values satisfy simple validation');
    assertText(sample, 'form > p:nth-of-type(3) output', '', 'the simple form message clears');
}

async function verifyFormLifecycle(sample: HTMLElement): Promise<void> {
    const instance = requiredElement(sample, 'article').parentElement as HTMLElement;
    const form = requiredElement(sample, 'form');
    const username = requiredControl(sample, 'input[name="username"]');
    const methods = ['email', 'sms', 'password'];
    const radios = methods.map(method => requiredControl(sample, `input[value="${method}"]`));
    const state = async (method: string, valid: boolean, message = '') => {
        await whenCemRendered(instance);
        await waitFor(() => {
            expect(requiredElement(sample, 'form')).toBe(form);
            expect(requiredControl(sample, 'input[name="username"]')).toBe(username);
            expect(username.isConnected).toBe(true);
            expect(radios.map(radio => radio.checked)).toEqual(methods.map(value => value === method));
            expect(radios.every(radio => radio.isConnected)).toBe(true);
            expect(textValue(sample, 'form > p:nth-of-type(2) output')).toBe(username.value);
            expect(textValue(sample, 'form > p:nth-of-type(3) output')).toBe(method);
            expect(textValue(sample, 'form > p:nth-of-type(4) output')).toBe(String(valid));
            expect(textValue(sample, 'form > p:nth-of-type(5) output')).toBe(message);
            expect(sample.querySelectorAll('input[name="password"]')).toHaveLength(method === 'password' ? 1 : 0);
            expect(Array.from(sample.querySelectorAll('fieldset > p'), node => normalize(node.textContent ?? '')))
                .toEqual(method === 'sms' ? ['Message and data rates may apply.'] : method ? [] : ['Select a confirmation method.']);
        });
        expect(cemDiagnosticCodes(instance)).toEqual([]);
    };
    const choose = async (method: string, keyboard = false) => {
        const radio = radios[methods.indexOf(method)];
        if (keyboard) {
            radio.focus();
            await userEvent.keyboard(' ');
        } else await userEvent.click(radio);
    };
    const edit = async (control: HTMLInputElement, value: string) => {
        await userEvent.clear(control);
        if (value) await userEvent.type(control, value);
        await whenCemRendered(instance);
        expect(control.isConnected).toBe(true);
        expect(document.activeElement).toBe(control);
        expect(control.value).toBe(value);
    };

    await whenCemRendered(instance);
    expect(textValue(sample, 'fieldset > p')).toBe('Select a confirmation method.');
    expect(radios.map(radio => radio.checked)).toEqual([false, false, false]);
    expect(sample.querySelector('input[name="password"]')).toBeNull();
    expect(cemDiagnosticCodes(instance)).toEqual([]);
    await edit(username, 'short');
    await state('', false, 'Complete the username and confirmation method');
    expect(username.validationMessage).toBe('Use at least 10 characters');
    expect(textValue(sample, 'form > p:first-of-type output')).toBe(username.validationMessage);
    await edit(username, 'abcdefghij');
    await state('', false, 'Complete the username and confirmation method');
    expect(username.validationMessage).toBe('');
    await choose('email');
    await state('email', true);
    await choose('sms', true);
    await state('sms', true);
    await choose('password');
    await whenCemRendered(instance);
    const password = requiredControl(sample, 'input[name="password"]');
    expect(password.value).toBe('');
    expect(password.validity.valid).toBe(false);
    // The initial form-valid output after this conditional insertion is a
    // recorded shared-refresh investigation in docs/todo.md.

    for (const [value, valid] of [['abc', false], ['abcd', true], ['', false], ['secret', true]] as const) {
        await edit(password, value);
        await state('password', valid, valid ? '' : 'Complete the username and confirmation method');
        expect(requiredControl(sample, 'input[name="password"]')).toBe(password);
        if (value === 'abc') expect(password.validationMessage).toBe('Password is too short');
    }
    await choose('email', true);
    await state('email', true);
    await choose('password');
    await state('password', true);
    expect(requiredControl(sample, 'input[name="password"]').value).toBe('secret');
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
        (candidate) => candidate.getAttribute('aria-label') === label
    );
    if (!button) throw new Error(`expected ${label} option in fruit choice ${index + 1}`);
    assertEqual(normalize(button.textContent ?? ''), label === 'Apple' ? '🍏' : '🍌', `${label} uses its fruit symbol`);
    assertEqual(button.title, label, `${label} retains its tooltip`);
    click(button);
}


function sampleByLegend(host: ParentNode, legend: string): HTMLElement {
    const sample = Array.from(host.querySelectorAll<HTMLElement>('cem-demo-element[legend]')).find(
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

function buttonNames(root: ParentNode): string[] {
    return Array.from(root.querySelectorAll('button'), (button) => normalize(button.getAttribute('aria-label') ?? button.textContent ?? ''));
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
