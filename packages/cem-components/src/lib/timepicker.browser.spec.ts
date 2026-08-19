import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import timepickerContractFixture from '../../tests/timepicker/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('timepicker contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(async () => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-timepicker-declaration' });
        const result = await installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('keeps one direct native text input and optional native toggle as the exact owners', async () => {
        expect(timepickerContractFixture).not.toMatch(/<script\b/i);
        expect(timepickerContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const parts = timepickerParts(root, '#meeting-timepicker');

        expect(parts.owner.dataset.mode).toBe('valid');
        expect(parts.input).toBe(requiredElement(root, '#meeting-time'));
        expect(parts.input.parentElement).toBe(parts.owner);
        expect(parts.input.type).toBe('text');
        expect(parts.input.getAttribute('role')).toBe('combobox');
        expect(parts.input.getAttribute('aria-autocomplete')).toBe('list');
        expect(parts.input.getAttribute('aria-haspopup')).toBe('listbox');
        expect(parts.input.getAttribute('aria-controls')).toBe(parts.popup.id);
        expect(parts.input.getAttribute('aria-expanded')).toBe('false');
        expect(parts.input.hasAttribute('aria-activedescendant')).toBe(false);
        expect(parts.popup.getAttribute('role')).toBe('listbox');
        expect(parts.popup.getAttribute('popover')).toBe('manual');
        expect(parts.popup.matches(':popover-open')).toBe(false);
        expect(parts.toggle?.type).toBe('button');
        expect(parts.toggle?.getAttribute('aria-controls')).toBe(parts.popup.id);
        expect(assertAccessibleName(parts.input, 'Meeting time')).toBe('Meeting time');
        expect(assertAccessibleName(requiredToggle(parts), 'Choose meeting time')).toBe('Choose meeting time');
        expect(parts.host.hasAttribute('name')).toBe(false);
        expect(new FormData(requiredElement<HTMLFormElement>(root, '#timepicker-form')).get('meeting-time')).toBe('09:30');
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('normalizes generated and authored choices while native value, validation, and reset stay on the input', async () => {
        const root = await renderFixture();
        const meeting = timepickerParts(root, '#meeting-timepicker');
        const generated = timepickerParts(root, '#generated-timepicker');

        expect(optionValues(meeting)).toEqual(['09:00', '09:30', '10:00', '10:30', '11:00', '17:30']);
        expect(optionLabels(meeting)).toEqual(['9:00 AM', '9:30 AM', '10:00 AM', '10:30 AM', '11:00 AM', '5:30 PM']);
        expect(optionFor(meeting, '09:30').getAttribute('aria-selected')).toBe('true');
        expect(optionFor(meeting, '10:30').getAttribute('aria-disabled')).toBe('true');
        expect(optionFor(meeting, '17:30').getAttribute('aria-disabled')).toBe('true');
        expect(optionValues(generated)).toEqual(['08:00', '08:45', '09:30']);
        expect(optionFor(generated, '08:45').getAttribute('aria-selected')).toBe('true');

        setNativeValue(meeting.input, '9:30');
        expect(meeting.input.validity.customError).toBe(true);
        expect(meeting.input.getAttribute('aria-invalid')).toBe('true');
        setNativeValue(meeting.input, '18:00');
        expect(meeting.input.validity.rangeOverflow).toBe(false);
        expect(meeting.input.validity.customError).toBe(true);
        setNativeValue(meeting.input, '10:00');
        expect(meeting.input.validity.valid).toBe(true);
        expect(meeting.input.hasAttribute('aria-invalid')).toBe(false);
        await waitFor(() => optionFor(meeting, '10:00').getAttribute('aria-selected') === 'true', 'typed selection');
        expect(optionFor(meeting, '10:00').getAttribute('aria-selected')).toBe('true');

        meeting.input.value = '';
        meeting.input.dispatchEvent(new Event('input', { bubbles: true }));
        expect(meeting.input.validity.valueMissing).toBe(true);
        expect(new FormData(requiredElement<HTMLFormElement>(root, '#timepicker-form')).get('meeting-time')).toBe('');
        requiredElement<HTMLButtonElement>(root, '#timepicker-reset').click();
        await waitForValue(meeting.input, '09:30');
        expect(meeting.input.validity.valid).toBe(true);
        await waitFor(() => optionFor(meeting, '09:30').getAttribute('aria-selected') === 'true', 'reset selection');
        expect(optionFor(meeting, '09:30').getAttribute('aria-selected')).toBe('true');
    });

    it('keeps focus on the combobox while keyboard navigation skips disabled choices and commits in order', async () => {
        const root = await renderFixture();
        const parts = timepickerParts(root, '#meeting-timepicker');
        const events: string[] = [];
        for (const name of ['input', 'change']) {
            parts.input.addEventListener(name, (event) => events.push(`${event.type}:${event.target === parts.input}:${event.isTrusted}`));
        }
        parts.input.focus();

        await userEvent.keyboard('{ArrowDown}');
        await waitForPopover(parts.popup, true);
        expect(document.activeElement).toBe(parts.input);
        expect(activeOption(parts)?.dataset.value).toBe('09:30');
        expect(activeOption(parts)?.getAttribute('aria-selected')).toBe('true');

        await userEvent.keyboard('{ArrowDown}');
        expect(activeOption(parts)?.dataset.value).toBe('10:00');
        await userEvent.keyboard('{ArrowDown}');
        expect(activeOption(parts)?.dataset.value).toBe('11:00');
        await userEvent.keyboard('{Enter}');
        await waitForPopover(parts.popup, false);
        expect(parts.input.value).toBe('11:00');
        expect(document.activeElement).toBe(parts.input);
        expect(events).toEqual(['input:true:false', 'change:true:false']);

        await userEvent.keyboard('{ArrowUp}');
        await waitForPopover(parts.popup, true);
        const valueBeforeEscape = parts.input.value;
        await userEvent.keyboard('{Escape}');
        await waitForPopover(parts.popup, false);
        expect(parts.input.value).toBe(valueBeforeEscape);
        expect(document.activeElement).toBe(parts.input);
    });

    it('opens through native input and toggle clicks, commits pointer choice on the input, and closes outside', async () => {
        const root = await renderFixture();
        const parts = timepickerParts(root, '#meeting-timepicker');
        const toggle = requiredToggle(parts);
        const events: string[] = [];
        toggle.addEventListener('click', (event) => events.push(`toggle:${event.isTrusted}`));
        for (const name of ['input', 'change']) {
            parts.input.addEventListener(name, (event) => events.push(`${event.type}:${event.target === parts.input}`));
        }

        await userEvent.click(toggle);
        await waitForPopover(parts.popup, true);
        expect(events).toEqual(['toggle:true']);
        expect(document.activeElement).toBe(parts.input);
        await userEvent.click(optionFor(parts, '10:00'));
        await waitForPopover(parts.popup, false);
        expect(parts.input.value).toBe('10:00');
        expect(events).toEqual(['toggle:true', 'input:true', 'change:true']);

        await userEvent.click(parts.input);
        await waitForPopover(parts.popup, true);
        await userEvent.click(requiredElement(root, '[data-timepicker-focus-start]'));
        await waitForPopover(parts.popup, false);
        expect(parts.input.value).toBe('10:00');
        expect(events).toEqual(['toggle:true', 'input:true', 'change:true']);
    });

    it('suppresses every disabled route and rejects malformed owner vocabulary without substitution', async () => {
        const root = await renderFixture();
        const disabled = timepickerParts(root, '#disabled-timepicker');
        const malformed = timepickerParts(root, '#malformed-timepicker', false);

        expect(disabled.input.disabled).toBe(true);
        expect(requiredToggle(disabled).disabled).toBe(true);
        disabled.input.click();
        requiredToggle(disabled).click();
        await nextRenderFrame();
        expect(disabled.popup.matches(':popover-open')).toBe(false);

        disabled.host.removeAttribute('disabled');
        await waitFor(() => !disabled.input.disabled && !requiredToggle(disabled).disabled, 'disabled state clears');
        disabled.input.click();
        await waitForPopover(disabled.popup, true);
        disabled.host.setAttribute('disabled', '');
        await waitForPopover(disabled.popup, false);
        expect(disabled.input.disabled).toBe(true);

        expect(malformed.owner.dataset.mode).toBe('invalid');
        expect(malformed.owner.querySelectorAll(':scope > input[slot="input"]')).toHaveLength(2);
        expect(malformed.popup.querySelectorAll('[role="option"]')).toHaveLength(0);
        expect(malformed.inputs.every((input) => input.getAttribute('role') !== 'combobox')).toBe(true);
    });

    it('keeps hover, focus-visible, expanded, active, selected, and disabled paint independent and geometry-stable', async () => {
        const root = await renderFixture();
        const parts = timepickerParts(root, '#meeting-timepicker');
        const inputIdentity = parts.input;
        const hostAttributes = attributes(parts.host);
        const baselineGeometry = geometry(parts.input);
        const baselineValue = parts.input.value;
        const mutationEvents: string[] = [];
        for (const name of ['input', 'change', 'cem-timepicker-toggle']) {
            parts.host.addEventListener(name, (event) => mutationEvents.push(event.type));
        }

        await userEvent.hover(parts.input);
        expect(parts.input.matches(':hover')).toBe(true);
        expect(indicatorLayers(parts.input)[0]?.color).toBe(
            resolveTokenColor(parts.input, '--cem-input-indicator-anchor-hover-color'),
        );
        expect(geometry(parts.input)).toEqual(baselineGeometry);
        await userEvent.unhover(parts.input);

        requiredElement<HTMLButtonElement>(root, '[data-timepicker-focus-start]').focus();
        await userEvent.keyboard('{Tab}');
        expect(document.activeElement).toBe(parts.input);
        expect(parts.input.matches(':focus-visible')).toBe(true);
        expect(geometry(parts.input)).toEqual(baselineGeometry);

        await userEvent.keyboard('{ArrowDown}');
        await waitForPopover(parts.popup, true);
        const selectedActive = optionFor(parts, '09:30');
        expect(selectedActive.getAttribute('aria-selected')).toBe('true');
        expect(getComputedStyle(selectedActive).backgroundColor).toBe(
            resolveTokenColor(selectedActive, '--cem-select-option-active-background'),
        );
        const selectedState = semanticOptionState(parts);
        const hover = optionFor(parts, '10:00');
        const hoverGeometry = geometry(hover);
        await userEvent.hover(hover);
        expect(getComputedStyle(hover).backgroundColor).toBe(
            resolveTokenColor(hover, '--cem-select-option-hover-background'),
        );
        expect(semanticOptionState(parts)).toEqual(selectedState);
        expect(geometry(hover)).toEqual(hoverGeometry);
        await userEvent.unhover(hover);

        expect(parts.input.value).toBe(baselineValue);
        expect(requiredElement(root, '#meeting-time')).toBe(inputIdentity);
        expect(attributes(parts.host)).toEqual(hostAttributes);
        expect(geometry(parts.input)).toEqual(baselineGeometry);
        expect(mutationEvents).toEqual([]);
    });

    it('supports a silent expanded property while live range changes update validity and option suppression', async () => {
        const root = await renderFixture();
        const parts = timepickerParts(root, '#meeting-timepicker');
        const host = parts.host as HTMLElement & { expanded: boolean };
        const events: string[] = [];
        for (const name of ['input', 'change', 'cem-timepicker-toggle']) host.addEventListener(name, () => events.push(name));

        host.expanded = true;
        await waitForPopover(parts.popup, true);
        expect(host.expanded).toBe(true);
        expect(host.hasAttribute('expanded')).toBe(false);
        host.expanded = false;
        await waitForPopover(parts.popup, false);

        parts.host.setAttribute('max', '09:15');
        await waitFor(() => optionFor(parts, '09:30').getAttribute('aria-disabled') === 'true', 'live max suppression');
        expect(parts.input.validity.customError).toBe(true);
        expect(parts.input.getAttribute('aria-invalid')).toBe('true');
        expect(events).toEqual([]);
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness();
        const root = await harness.render(timepickerContractFixture);
        await waitForSelector(root, '#meeting-timepicker > .cem-timepicker > input[slot="input"]');
        await waitFor(() => optionValues(timepickerParts(root, '#meeting-timepicker')).length === 6, 'time options');
        return root;
    }
});

interface TimepickerParts {
    host: HTMLElement;
    input: HTMLInputElement;
    inputs: HTMLInputElement[];
    options: HTMLElement[];
    owner: HTMLElement;
    popup: HTMLElement;
    toggle: HTMLButtonElement | null;
}

function timepickerParts(root: ParentNode, selector: string, requireSingleInput = true): TimepickerParts {
    const host = requiredElement<HTMLElement>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-timepicker');
    const inputs = [...owner.querySelectorAll<HTMLInputElement>(':scope > input[slot="input"]')];
    if (requireSingleInput && inputs.length !== 1) throw new Error(`Expected one timepicker input: ${selector}`);
    return {
        host,
        input: inputs[0] ?? document.createElement('input'),
        inputs,
        options: [...owner.querySelectorAll<HTMLElement>('.cem-timepicker__option')],
        owner,
        popup: requiredElement(owner, ':scope > .cem-timepicker__popup'),
        toggle: owner.querySelector<HTMLButtonElement>(':scope > button[slot="toggle"]'),
    };
}

function requiredToggle(parts: TimepickerParts): HTMLButtonElement {
    if (!parts.toggle) throw new Error('Missing required timepicker toggle');
    return parts.toggle;
}

function optionValues(parts: TimepickerParts): string[] {
    return parts.options.map((option) => option.dataset.value ?? '');
}

function optionLabels(parts: TimepickerParts): string[] {
    return parts.options.map((option) => option.textContent ?? '');
}

function optionFor(parts: TimepickerParts, value: string): HTMLElement {
    const option = parts.owner.querySelector<HTMLElement>(`.cem-timepicker__option[data-value="${value}"]`);
    if (!option) throw new Error(`Missing timepicker option: ${value}`);
    return option;
}

function activeOption(parts: TimepickerParts): HTMLElement | null {
    return parts.owner.querySelector<HTMLElement>('.cem-timepicker__option[data-active="true"]');
}

function semanticOptionState(parts: TimepickerParts): string[] {
    return parts.options.map((option) => [
        option.dataset.value,
        option.dataset.active,
        option.getAttribute('aria-selected'),
        option.getAttribute('aria-disabled'),
    ].join(':'));
}

function setNativeValue(input: HTMLInputElement, value: string): void {
    input.value = value;
    input.dispatchEvent(new Event('input', { bubbles: true }));
}

function attributes(element: Element): Record<string, string> {
    return Object.fromEntries([...element.attributes].map((attribute) => [attribute.name, attribute.value]));
}

function geometry(element: Element): { height: number; width: number; x: number; y: number } {
    const rect = element.getBoundingClientRect();
    return { height: rect.height, width: rect.width, x: rect.x, y: rect.y };
}

function indicatorLayers(element: Element): Array<{ color: string }> {
    return [...getComputedStyle(element).boxShadow.matchAll(/rgba?\([^)]*\)/g)]
        .map((match) => ({ color: match[0] }));
}

function resolveTokenColor(owner: HTMLElement, token: string): string {
    const probe = owner.ownerDocument.createElement('span');
    probe.style.color = `var(${token})`;
    (owner.parentElement ?? owner.ownerDocument.body).append(probe);
    const value = getComputedStyle(probe).color;
    probe.remove();
    return value;
}

function requiredElement<T extends Element>(root: ParentNode | T | null, selector: string): T {
    if (root instanceof Element && root.matches(selector)) return root as T;
    const element = root?.querySelector<T>(selector);
    if (!element) throw new Error(`Missing required fixture element: ${selector}`);
    return element;
}

async function waitForSelector(root: ParentNode, selector: string): Promise<void> {
    await waitFor(() => Boolean(root.querySelector(selector)), selector);
}

async function waitForPopover(popup: HTMLElement, open: boolean): Promise<void> {
    await waitFor(() => popup.matches(':popover-open') === open, `timepicker popup open=${String(open)}`);
}

async function waitForValue(input: HTMLInputElement, value: string): Promise<void> {
    await waitFor(() => input.value === value, `timepicker value ${value}`);
}

async function waitFor(predicate: () => boolean, label: string): Promise<void> {
    for (let attempt = 0; attempt < 120; attempt += 1) {
        if (predicate()) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for ${label}`);
}
