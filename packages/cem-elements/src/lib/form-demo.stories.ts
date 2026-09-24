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
        await waitFor(() => expect(host.querySelectorAll('cem-demo-element[legend] article')).toHaveLength(5));
        expect(Array.from(host.querySelectorAll('cem-demo-element[legend]'), sample => sample.getAttribute('legend')))
            .toEqual(EXPECTED_LEGENDS);
        const checks = [verifySimpleValidation, verifyFormLifecycle, verifyNativeMessage, verifyFormMessage, verifyFormAssociatedDce];
        for (const [index, check] of checks.entries()) {
            const sample = sampleByLegend(host, EXPECTED_LEGENDS[index]);
            await step(EXPECTED_LEGENDS[index], async () => {
                await settle(sample);
                await check(sample);
                expect(cemDiagnosticCodes(requiredElement(sample, 'article').parentElement as HTMLElement)).toEqual([]);
            });
        }
        expect(cemDiagnosticCodes(host)).toEqual([]);
    },
};

async function verifySimpleValidation(sample: HTMLElement): Promise<void> {
    const form = requiredElement(sample, 'form') as HTMLFormElement;
    const username = requiredControl(sample, 'input[name="username"]');
    const submissions: boolean[] = [];
    // Observe runtime cancellation, then prevent the demo's native GET navigation.
    const onSubmit = (event: Event) => { submissions.push(event.defaultPrevented); event.preventDefault(); };
    sample.addEventListener('submit', onSubmit);
    try {
        expect(username.value).toBe('');
        expect(buttonNames(sample)).toEqual(['Next']);
        expect(textList(sample, 'form > p output')).toEqual(['', 'false', 'Enter a long username and password']);
        await userEvent.click(requiredElement(sample, 'button'));
        expect(username.validity.valid).toBe(false);
        expect(submissions).toEqual([]);
        await edit(sample, username, 'abcdefghij');
        await userEvent.click(requiredElement(sample, 'button'));
        await settle(sample);
        expect(sample.querySelector('input[name="password"]')).toBeNull();
        expect(submissions.every(cancelled => cancelled)).toBe(true);
        await edit(sample, username, 'abcdefghijk');
        expect(textList(sample, 'form > p output')).toEqual(['abcdefghijk', 'false', 'Enter a long username and password']);
        expect(sample.querySelector('input[name="password"]')).toBeNull();
        await userEvent.click(requiredElement(sample, 'button'));
        await settle(sample);
        expect(buttonNames(sample)).toEqual(['Sign in']);
        const password = requiredControl(sample, 'input[name="password"]');
        for (const [value, valid] of [['abc', false], ['abcd', true], ['', false], ['secret', true]] as const) {
            await edit(sample, password, value);
            expect(textList(sample, 'form > p output')).toEqual([
                'abcdefghijk', String(valid), valid ? '' : 'Enter a long username and password',
            ]);
            expect(requiredElement(sample, 'form')).toBe(form);
            expect(requiredControl(sample, 'input[name="username"]')).toBe(username);
            expect(requiredControl(sample, 'input[name="password"]')).toBe(password);
        }
        await userEvent.click(requiredElement(sample, 'button'));
        expect(submissions.at(-1)).toBe(false);
        expect(Array.from(new FormData(form).entries())).toEqual([['username', 'abcdefghijk'], ['password', 'secret']]);
    } finally { sample.removeEventListener('submit', onSubmit); }
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
    await state('password', false, 'Complete the username and confirmation method');
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
    expect(email.value).toBe('person@example.test');
    expect(textList(sample, 'output')).toEqual(['true', '']);
    for (const value of ['', 'not-an-email', 'reader@example.test', '', 'person@example.test']) {
        await edit(sample, email, value);
        expect(requiredControl(sample, 'input[name="email"]')).toBe(email);
        expect(textList(sample, 'output')).toEqual([String(email.validity.valid), email.validationMessage]);
        expect(email.validity.valueMissing).toBe(value === '');
        expect(email.validity.typeMismatch).toBe(value === 'not-an-email');
    }
}

async function verifyFormMessage(sample: HTMLElement): Promise<void> {
    const email = requiredControl(sample, 'input[name="email"]');
    expect(email.value).toBe('');
    expect(textList(sample, 'output')).toEqual(['', '0', 'false', 'Use more than 3 characters']);
    for (const value of ['abc', 'abcd', '🍒ab', '🍒abc', '', 'reader']) {
        await edit(sample, email, value);
        const length = [...value].length;
        expect(requiredControl(sample, 'input[name="email"]')).toBe(email);
        expect(textList(sample, 'output')).toEqual([
            value, String(length), String(length > 3), length > 3 ? '' : 'Use more than 3 characters',
        ]);
    }
}

async function verifyFormAssociatedDce(sample: HTMLElement): Promise<void> {
    const form = requiredElement(sample, 'form') as HTMLFormElement;
    const choices = Array.from(sample.querySelectorAll<HTMLElement>('cem-form-fruit-choice'));
    expect(choices).toHaveLength(2);
    const labels = ['Choose fruit', 'Apple', 'Banana'];
    for (const choice of choices) {
        await whenCemRendered(choice);
        expect(textList(choice, 'button')).toEqual(['Choose fruit', '🍏', '🍌']);
        expect(Array.from(choice.querySelectorAll('button'), button => [button.getAttribute('aria-label'), button.title]))
            .toEqual(labels.map(label => [label, label]));
    }
    const state = async (first: string, second: string) => {
        await Promise.all(choices.map(whenCemRendered));
        await settle(sample);
        const valid = first !== '' && first === second;
        await waitFor(() => expect(textList(sample, 'form > p output')).toEqual([
            first, second, String(valid), valid ? '' : 'Choose the same fruit',
        ]));
        expect(Array.from(new FormData(form).entries())).toEqual([['firstFruit', first], ['secondFruit', second]]);
        expect(Array.from(sample.querySelectorAll('cem-form-fruit-choice'))).toEqual(choices);
        for (const [index, value] of [first, second].entries()) {
            const choice = choices[index];
            expect(textList(choice, 'output')).toEqual([value]);
            expect(Array.from(choice.querySelectorAll('button'), button => button.getAttribute('aria-pressed')))
                .toEqual(['', '🍏', '🍌'].map(option => String(option === value)));
            expect(cemDiagnosticCodes(choice)).toEqual([]);
        }
    };
    const choose = async (index: number, label: string) => {
        const button = requiredElement(choices[index], `button[aria-label="${label}"]`);
        await userEvent.click(button);
    };
    await state('', '');
    await choose(0, 'Apple');
    await state('🍏', '');
    await choose(1, 'Banana');
    await state('🍏', '🍌');
    await choose(1, 'Apple');
    await state('🍏', '🍏');
    await choose(0, 'Choose fruit');
    await state('', '🍏');
    await choose(0, 'Banana');
    await state('🍌', '🍏');
    await choose(1, 'Banana');
    await state('🍌', '🍌');

    // The choice-select capability uses dropdown navigation: open, move, commit.
    const selected = requiredElement(choices[1], 'button[aria-pressed="true"]');
    selected.focus();
    await userEvent.keyboard(' {ArrowUp} ');
    await state('🍌', '🍏');
    const submissions: boolean[] = [];
    const onSubmit = (event: Event) => { submissions.push(event.defaultPrevented); event.preventDefault(); };
    sample.addEventListener('submit', onSubmit);
    try {
        await userEvent.click(requiredElement(sample, 'button[type="submit"]'));
        expect(submissions).toEqual([true]);
        await choose(1, 'Banana');
        await state('🍌', '🍌');
        await userEvent.click(requiredElement(sample, 'button[type="submit"]'));
        expect(submissions).toEqual([true, false]);
    } finally { sample.removeEventListener('submit', onSubmit); }
}

async function settle(sample: HTMLElement): Promise<void> {
    await whenCemRendered(requiredElement(sample, 'article').parentElement as HTMLElement);
}

async function edit(sample: HTMLElement, control: HTMLInputElement, value: string): Promise<void> {
    await userEvent.clear(control);
    if (value) await userEvent.type(control, value);
    await settle(sample);
    expect(control.isConnected).toBe(true);
    expect(document.activeElement).toBe(control);
    expect(control.value).toBe(value);
}

function textList(sample: ParentNode, selector: string): string[] {
    return Array.from(sample.querySelectorAll(selector), element => normalize(element.textContent ?? ''));
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

function buttonNames(root: ParentNode): string[] {
    return Array.from(root.querySelectorAll('button'), (button) => normalize(button.getAttribute('aria-label') ?? button.textContent ?? ''));
}

function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}

function textValue(root: ParentNode, selector: string): string {
    return normalize(requiredElement(root, selector).textContent ?? '');
}

function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
