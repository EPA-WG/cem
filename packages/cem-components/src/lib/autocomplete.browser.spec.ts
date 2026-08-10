import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import autocompleteContractFixture from '../../tests/autocomplete/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

type TestCemAutocomplete = HTMLElement & {
    checkValidity(): boolean;
    disabled: boolean;
    displayValue: string;
    expanded: boolean;
    form: HTMLFormElement | null;
    readonly: boolean;
    reportValidity(): boolean;
    required: boolean;
    selectedIndex: number;
    value: string;
    validationMessage: string;
    validity: ValidityState;
    willValidate: boolean;
};

describe('autocomplete contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(() => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-autocomplete-declaration' });
        const result = installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('is declarative and exposes the accepted editable-combobox and form surface', async () => {
        expect(autocompleteContractFixture).not.toMatch(/<script\b/i);
        expect(autocompleteContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const form = requiredElement<HTMLFormElement>(root, '#autocomplete-form');
        const host = requiredElement<TestCemAutocomplete>(root, '#person-autocomplete');
        const input = requiredElement<HTMLInputElement>(host, '.cem-autocomplete__control');

        expect(input.localName).toBe('input');
        expect(input.type).toBe('text');
        expect(input.getAttribute('role')).toBe('combobox');
        expect(input.getAttribute('aria-autocomplete')).toBe('list');
        expect(input.getAttribute('aria-expanded')).toBe('false');
        expect(input.hasAttribute('aria-controls')).toBe(false);
        expect(input.hasAttribute('aria-activedescendant')).toBe(false);
        expect(input.hasAttribute('name')).toBe(false);
        expect(assertAccessibleName(input, 'Person')).toBe('Person');
        expect(host.value).toBe('ada');
        expect(host.displayValue).toBe('Ada Lovelace');
        expect(host.selectedIndex).toBe(0);
        expect(host.expanded).toBe(false);
        expect(host.form).toBe(form);
        expect(host.required).toBe(true);
        expect(host.checkValidity()).toBe(true);
        expect(host.willValidate).toBe(true);
        expect(new FormData(form).get('person')).toBe('ada');
        expect(host.querySelector('cem-option, cem-option-group')).toBeNull();
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('keeps native text input events singular while free-form editing owns the submitted value', async () => {
        const root = await renderFixture();
        const form = requiredElement<HTMLFormElement>(root, '#autocomplete-form');
        const host = requiredElement<TestCemAutocomplete>(root, '#person-autocomplete');
        const input = requiredElement<HTMLInputElement>(host, '.cem-autocomplete__control');
        const observations: Array<{ displayValue: string; target: EventTarget | null; type: string; value: string }> = [];
        for (const eventName of ['input', 'change']) {
            host.addEventListener(eventName, (event) => {
                observations.push({
                    displayValue: host.displayValue,
                    target: event.target,
                    type: event.type,
                    value: host.value,
                });
            });
        }

        await userEvent.clear(input);
        await userEvent.type(input, 'New person');
        await nextRenderFrame();

        expect(host.value).toBe('New person');
        expect(host.displayValue).toBe('New person');
        expect(host.selectedIndex).toBe(-1);
        expect(new FormData(form).get('person')).toBe('New person');
        expect(observations.filter((entry) => entry.type === 'input')).toHaveLength('New person'.length + 1);
        expect(observations.every((entry) => entry.target === input)).toBe(true);
        expect(observations.at(-1)).toMatchObject({ displayValue: 'New person', value: 'New person' });
    });

    it('navigates enabled suggestions with focus retained and commits one ordered event pair', async () => {
        const root = await renderFixture();
        const host = requiredElement<TestCemAutocomplete>(root, '#person-autocomplete');
        const input = requiredElement<HTMLInputElement>(host, '.cem-autocomplete__control');
        const events: string[] = [];
        for (const eventName of ['input', 'change']) host.addEventListener(eventName, () => events.push(eventName));

        input.focus();
        await nextRenderFrame();
        expect(document.activeElement).toBe(input);
        expect(host.expanded).toBe(true);
        expect(input.getAttribute('aria-expanded')).toBe('true');
        const popup = requiredElement<HTMLElement>(host, '.cem-autocomplete__popup');
        expect(input.getAttribute('aria-controls')).toBe(popup.id);
        expect(popup.getAttribute('role')).toBe('listbox');
        expect(popup.querySelector('[role="group"]')?.getAttribute('aria-label')).toBe('Engineering');
        expect(popup.querySelector('[role="option"] strong')?.textContent).toBe('Ada');
        expect(popup.querySelectorAll('[role="option"]')).toHaveLength(3);

        await userEvent.keyboard('{End}');
        expect(input.selectionStart).toBe(input.value.length);
        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();
        const active = requiredElement<HTMLElement>(host, `#${input.getAttribute('aria-activedescendant')}`);
        expect(active.textContent?.trim()).toBe('Grace Hopper');
        expect(active.getAttribute('aria-disabled')).toBe('false');
        expect(host.value).toBe('ada');
        expect(events).toEqual([]);
        expect(document.activeElement).toBe(input);

        await userEvent.keyboard('{Enter}');
        await nextRenderFrame();
        expect(host.value).toBe('grace');
        expect(host.displayValue).toBe('Grace Hopper');
        expect(host.selectedIndex).toBe(1);
        expect(host.expanded).toBe(false);
        expect(input.getAttribute('aria-expanded')).toBe('false');
        expect(input.hasAttribute('aria-controls')).toBe(false);
        expect(input.hasAttribute('aria-activedescendant')).toBe(false);
        expect(events).toEqual(['input', 'change']);
        expect(document.activeElement).toBe(input);
    });

    it('enforces require-selection on close without duplicating native events', async () => {
        const root = await renderFixture();
        const host = requiredElement<TestCemAutocomplete>(root, '#selection-autocomplete');
        const input = requiredElement<HTMLInputElement>(host, '.cem-autocomplete__control');
        const events: string[] = [];
        for (const eventName of ['input', 'change']) host.addEventListener(eventName, () => events.push(eventName));

        input.focus();
        await userEvent.type(input, 'unmatched');
        expect(host.displayValue).toBe('unmatched');
        expect(host.value).toBe('');
        expect(host.selectedIndex).toBe(-1);
        events.length = 0;

        await userEvent.keyboard('{Escape}');
        await nextRenderFrame();
        expect(host.displayValue).toBe('');
        expect(host.value).toBe('');
        expect(host.expanded).toBe(false);
        expect(events).toEqual(['input', 'change']);

        host.value = 'beta';
        await nextRenderFrame();
        expect(host.value).toBe('beta');
        expect(host.displayValue).toBe('Beta');
        expect(host.selectedIndex).toBe(1);
        expect(events).toEqual(['input', 'change']);

        host.value = 'missing';
        await nextRenderFrame();
        expect(host.value).toBe('');
        expect(host.displayValue).toBe('');
        expect(host.selectedIndex).toBe(-1);
        expect(events).toEqual(['input', 'change']);
    });

    it('supports alternate close paths, reverse navigation, pointer commit, and disabled rejection', async () => {
        const root = await renderFixture();
        const host = requiredElement<TestCemAutocomplete>(root, '#selection-autocomplete');
        const input = requiredElement<HTMLInputElement>(host, '.cem-autocomplete__control');
        const events: string[] = [];
        for (const eventName of ['input', 'change']) host.addEventListener(eventName, () => events.push(eventName));

        input.focus();
        await nextRenderFrame();
        await userEvent.keyboard('{Escape}');
        expect(host.expanded).toBe(false);
        expect(events).toEqual([]);

        await userEvent.keyboard('{Alt>}{ArrowDown}{/Alt}');
        await nextRenderFrame();
        expect(host.expanded).toBe(true);
        expect(input.hasAttribute('aria-activedescendant')).toBe(false);
        await userEvent.keyboard('{Alt>}{ArrowUp}{/Alt}');
        expect(host.expanded).toBe(false);
        expect(events).toEqual([]);

        await userEvent.keyboard('{Alt>}{ArrowDown}{/Alt}{ArrowUp}');
        await nextRenderFrame();
        const active = requiredElement<HTMLElement>(host, `#${input.getAttribute('aria-activedescendant')}`);
        expect(active.textContent?.trim()).toBe('Beta');
        await userEvent.keyboard('{Tab}');
        expect(host.expanded).toBe(false);
        expect(host.value).toBe('');
        expect(events).toEqual([]);

        input.focus();
        await nextRenderFrame();
        const beta = Array.from(host.querySelectorAll<HTMLElement>('[role="option"]')).find(
            (option) => option.textContent?.trim() === 'Beta',
        );
        if (!beta) throw new Error('Expected Beta option');
        await userEvent.click(beta);
        await nextRenderFrame();
        expect(document.activeElement).toBe(input);
        expect(host.value).toBe('beta');
        expect(host.displayValue).toBe('Beta');
        expect(events).toEqual(['input', 'change']);

        await userEvent.keyboard('{Alt>}{ArrowDown}{/Alt}{Enter}');
        expect(host.value).toBe('beta');
        expect(events).toEqual(['input', 'change', 'input', 'change']);

        const person = requiredElement<TestCemAutocomplete>(root, '#person-autocomplete');
        const personInput = requiredElement<HTMLInputElement>(person, '.cem-autocomplete__control');
        const personEvents: string[] = [];
        for (const eventName of ['input', 'change']) {
            person.addEventListener(eventName, () => personEvents.push(eventName));
        }
        personInput.focus();
        await nextRenderFrame();
        const disabledOption = requiredElement<HTMLElement>(person, '[role="option"][aria-disabled="true"]');
        disabledOption.click();
        expect(person.value).toBe('ada');
        expect(person.expanded).toBe(true);
        expect(personEvents).toEqual([]);
    });

    it('refreshes live payload without replacing focus, input identity, committed value, or events', async () => {
        const root = await renderFixture();
        const host = requiredElement<TestCemAutocomplete>(root, '#person-autocomplete');
        const input = requiredElement<HTMLInputElement>(host, '.cem-autocomplete__control');
        const island = requiredElement<HTMLTemplateElement>(host, 'template[data-cem-island="instance"]');
        const events: string[] = [];
        for (const eventName of ['input', 'change']) host.addEventListener(eventName, () => events.push(eventName));

        input.focus();
        await nextRenderFrame();
        island.content.querySelectorAll('cem-option, cem-option-group').forEach((node) => node.remove());
        const replacement = document.createElement('cem-option');
        replacement.setAttribute('value', 'lin');
        replacement.setAttribute('label', 'Lin Chen');
        replacement.textContent = 'Lin Chen';
        island.content.append(replacement);

        await waitFor(() => host.querySelectorAll('[role="option"]').length === 1, 'replacement option renders');
        expect(requiredElement(host, '[role="option"]').textContent?.trim()).toBe('Lin Chen');
        expect(requiredElement(host, '.cem-autocomplete__control')).toBe(input);
        expect(document.activeElement).toBe(input);
        expect(host.value).toBe('ada');
        expect(host.displayValue).toBe('Ada Lovelace');
        expect(host.selectedIndex).toBe(-1);
        expect(events).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('supports native migration and suppresses disabled, readonly, and busy state side effects', async () => {
        const root = await renderFixture();
        const native = requiredElement<TestCemAutocomplete>(root, '#native-autocomplete');
        const readonly = requiredElement<TestCemAutocomplete>(root, '#readonly-autocomplete');
        const disabled = requiredElement<TestCemAutocomplete>(root, '#disabled-autocomplete');
        const busy = requiredElement<TestCemAutocomplete>(root, '#busy-autocomplete');
        const nativeInput = requiredElement<HTMLInputElement>(native, '.cem-autocomplete__control');
        const readonlyInput = requiredElement<HTMLInputElement>(readonly, '.cem-autocomplete__control');
        const disabledInput = requiredElement<HTMLInputElement>(disabled, '.cem-autocomplete__control');
        const busyInput = requiredElement<HTMLInputElement>(busy, '.cem-autocomplete__control');
        const events: string[] = [];
        for (const host of [readonly, disabled, busy]) {
            for (const eventName of ['input', 'change']) host.addEventListener(eventName, () => events.push(eventName));
        }

        expect(native.value).toBe('two');
        expect(native.displayValue).toBe('Two');
        nativeInput.focus();
        await nextRenderFrame();
        expect(native.querySelectorAll('[role="option"]')).toHaveLength(2);

        readonlyInput.focus();
        await userEvent.keyboard('{ArrowDown}');
        expect(readonly.readonly).toBe(true);
        expect(readonly.expanded).toBe(false);
        expect(readonly.value).toBe('fixed');
        expect(readonlyInput.readOnly).toBe(true);

        const focusOwner = document.activeElement;
        disabledInput.focus();
        disabledInput.click();
        expect(disabled.disabled).toBe(true);
        expect(disabled.expanded).toBe(false);
        expect(disabled.value).toBe('fixed');
        expect(disabledInput.disabled).toBe(true);
        expect(document.activeElement).toBe(focusOwner);

        expect(busyInput.getAttribute('data-state')).toBe('loading');
        expect(busyInput.getAttribute('aria-busy')).toBe('true');
        expect(busyInput.disabled).toBe(false);
        expect(events).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness();
        const root = await harness.render(autocompleteContractFixture);
        await waitFor(
            () => root.querySelectorAll('cem-autocomplete .cem-autocomplete__control').length === 6,
            'autocomplete controls render',
        );
        return root;
    }
});

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Expected fixture to contain ${selector}`);
    return element;
}

async function waitFor(condition: () => boolean, message: string): Promise<void> {
    const deadline = performance.now() + 1_000;
    while (!condition()) {
        if (performance.now() >= deadline) throw new Error(`Timed out waiting for ${message}`);
        await nextRenderFrame();
    }
}
