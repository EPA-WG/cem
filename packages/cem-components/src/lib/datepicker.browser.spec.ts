import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import datepickerContractFixture from '../../tests/datepicker/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('datepicker contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(() => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-datepicker-declaration' });
        const result = installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('keeps one direct native text input and optional native toggle as the exact owners', async () => {
        expect(datepickerContractFixture).not.toMatch(/<script\b/i);
        expect(datepickerContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const parts = datepickerParts(root, '#arrival-datepicker');

        expect(parts.owner.dataset.mode).toBe('valid');
        expect(parts.input).toBe(requiredElement(root, '#arrival-date'));
        expect(parts.input.parentElement).toBe(parts.owner);
        expect(parts.input.type).toBe('text');
        expect(parts.input.getAttribute('role')).toBe('combobox');
        expect(parts.input.getAttribute('aria-autocomplete')).toBe('none');
        expect(parts.input.getAttribute('aria-haspopup')).toBe('dialog');
        expect(parts.input.getAttribute('aria-controls')).toBe(parts.dialog.id);
        expect(parts.input.getAttribute('aria-expanded')).toBe('false');
        expect(parts.dialog.open).toBe(false);
        expect(parts.toggle?.type).toBe('button');
        expect(parts.toggle?.getAttribute('aria-controls')).toBe(parts.dialog.id);
        expect(parts.cancel.type).toBe('button');
        expect(parts.apply.type).toBe('button');
        expect(assertAccessibleName(parts.input, 'Arrival date')).toBe('Arrival date');
        expect(assertAccessibleName(requiredToggle(parts), 'Choose arrival date')).toBe('Choose arrival date');
        expect(parts.host.hasAttribute('name')).toBe(false);
        expect(new FormData(requiredElement(root, '#datepicker-form')).get('arrival-date')).toBe('2026-08-11');
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('keeps canonical value, locale calendar, validation, form, and reset ownership on the input', async () => {
        const root = await renderFixture();
        const arrival = datepickerParts(root, '#arrival-datepicker');
        const localized = datepickerParts(root, '#localized-datepicker');

        await userEvent.click(requiredToggle(arrival));
        await waitForDialog(arrival.dialog, true);
        expect(arrival.heading.textContent).toBe('August 2026');
        expect(weekdayLabels(arrival)).toEqual(['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']);
        expect(dayFor(arrival, '2026-08-11').getAttribute('aria-selected')).toBe('true');
        expect(dayFor(arrival, '2026-07-26').dataset.outside).toBe('true');
        expect(dayFor(arrival, '2026-07-31').getAttribute('aria-disabled')).toBe('true');
        await userEvent.click(arrival.cancel);
        await waitForDialog(arrival.dialog, false);

        await userEvent.click(requiredToggle(localized));
        await waitForDialog(localized.dialog, true);
        expect(localized.heading.textContent).toBe(formatMonth('fr-FR', '2026-12-24'));
        expect(weekdayLabels(localized)).toEqual(localizedWeekdays('fr-FR'));
        await userEvent.click(localized.cancel);

        setNativeValue(arrival.input, '2026-02-30');
        expect(arrival.input.validity.customError).toBe(true);
        expect(arrival.input.getAttribute('aria-invalid')).toBe('true');
        setNativeValue(arrival.input, '2025-12-31');
        expect(arrival.input.validity.customError).toBe(true);
        setNativeValue(arrival.input, '2026-08-12');
        expect(arrival.input.validity.valid).toBe(true);
        expect(arrival.input.hasAttribute('aria-invalid')).toBe(false);

        arrival.input.value = '';
        arrival.input.dispatchEvent(new Event('input', { bubbles: true }));
        expect(arrival.input.validity.valueMissing).toBe(true);
        expect(new FormData(requiredElement(root, '#datepicker-form')).get('arrival-date')).toBe('');
        requiredElement<HTMLButtonElement>(root, '#datepicker-reset').click();
        await waitForValue(arrival.input, '2026-08-11');
        expect(arrival.input.validity.valid).toBe(true);
    });

    it('moves one roving grid focus owner through day, week, month, and year navigation before confirmation', async () => {
        const root = await renderFixture();
        const parts = datepickerParts(root, '#arrival-datepicker');
        const events = nativeValueEvents(parts.input);
        parts.input.focus();

        await userEvent.keyboard('{ArrowDown}');
        await waitForDialog(parts.dialog, true);
        expect(activeDay(parts)?.dataset.date).toBe('2026-08-11');
        expect(document.activeElement).toBe(activeDay(parts));
        expect(rovingDays(parts)).toHaveLength(1);

        await userEvent.keyboard('{ArrowRight}{ArrowDown}{Home}');
        await waitForActiveDate(parts, '2026-08-16');
        await userEvent.keyboard('{End}');
        await waitForActiveDate(parts, '2026-08-22');
        await userEvent.keyboard('{PageDown}');
        await waitForActiveDate(parts, '2026-09-22');
        await userEvent.keyboard('{Shift>}{PageDown}{/Shift}');
        await waitForActiveDate(parts, '2027-09-22');

        await userEvent.keyboard('{Enter}');
        await waitFor(() => dayFor(parts, '2027-09-22').getAttribute('aria-selected') === 'true', 'draft selection');
        expect(parts.input.value).toBe('2026-08-11');
        expect(events).toEqual([]);
        await userEvent.click(parts.apply);
        await waitForDialog(parts.dialog, false);
        expect(parts.input.value).toBe('2027-09-22');
        expect(document.activeElement).toBe(parts.input);
        expect(events).toEqual(['input:true:false', 'change:true:false']);
    });

    it('keeps pointer drafts and cancel, Escape, and backdrop dismissal silent until Apply commits', async () => {
        const root = await renderFixture();
        const parts = datepickerParts(root, '#arrival-datepicker');
        const toggleEvents: string[] = [];
        const events = nativeValueEvents(parts.input);
        requiredToggle(parts).addEventListener('click', (event) => toggleEvents.push(`click:${event.isTrusted}`));

        await userEvent.click(requiredToggle(parts));
        await waitForDialog(parts.dialog, true);
        expect(toggleEvents).toEqual(['click:true']);
        await userEvent.click(dayFor(parts, '2026-08-12'));
        expect(parts.input.value).toBe('2026-08-11');
        expect(events).toEqual([]);
        await userEvent.click(parts.cancel);
        await waitForDialog(parts.dialog, false);
        expect(parts.input.value).toBe('2026-08-11');

        await userEvent.click(requiredToggle(parts));
        await waitForDialog(parts.dialog, true);
        await userEvent.keyboard('{Escape}');
        await waitForDialog(parts.dialog, false);
        expect(events).toEqual([]);

        await userEvent.click(requiredToggle(parts));
        await waitForDialog(parts.dialog, true);
        parts.dialog.dispatchEvent(new MouseEvent('click', { bubbles: true, clientX: 0, clientY: 0 }));
        await waitForDialog(parts.dialog, false);
        expect(parts.input.value).toBe('2026-08-11');
        expect(events).toEqual([]);
    });

    it('suppresses every disabled route and rejects malformed owner vocabulary without substitution', async () => {
        const root = await renderFixture();
        const disabled = datepickerParts(root, '#disabled-datepicker');
        const malformed = datepickerParts(root, '#malformed-datepicker', false);

        expect(disabled.input.disabled).toBe(true);
        expect(requiredToggle(disabled).disabled).toBe(true);
        disabled.input.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, key: 'ArrowDown' }));
        disabled.input.click();
        requiredToggle(disabled).click();
        await nextRenderFrame();
        expect(disabled.dialog.open).toBe(false);

        disabled.host.removeAttribute('disabled');
        await waitFor(() => !disabled.input.disabled && !requiredToggle(disabled).disabled, 'datepicker enabled');
        expect(disabled.input.disabled).toBe(false);

        expect(malformed.owner.dataset.mode).toBe('invalid');
        expect(malformed.owner.querySelectorAll(':scope > input[slot="input"]')).toHaveLength(2);
        expect(malformed.input.getAttribute('role')).not.toBe('combobox');
        expect(malformed.dialog.open).toBe(false);
    });

    it('keeps today, selected, focus, hover, and disabled paint independent and geometry-stable', async () => {
        const root = await renderFixture();
        const parts = datepickerParts(root, '#current-datepicker');
        const today = localToday();
        setNativeValue(parts.input, today);
        const inputIdentity = parts.input;
        const hostAttributes = attributes(parts.host);
        const baselineInputGeometry = geometry(parts.input);
        const mutationEvents = nativeValueEvents(parts.input);

        await userEvent.hover(parts.input);
        expect(parts.input.matches(':hover')).toBe(true);
        expect(geometry(parts.input)).toEqual(baselineInputGeometry);
        await userEvent.unhover(parts.input);

        parts.input.focus();
        await userEvent.keyboard('{ArrowDown}');
        await waitForDialog(parts.dialog, true);
        const currentSelected = dayFor(parts, today);
        expect(currentSelected.getAttribute('aria-current')).toBe('date');
        expect(currentSelected.getAttribute('aria-selected')).toBe('true');
        expect(currentSelected.matches(':focus-visible')).toBe(true);
        const baselineDayGeometry = size(currentSelected);
        expect(getComputedStyle(currentSelected).borderColor).toBe(
            resolveTokenColor(currentSelected, '--cem-content-interaction-current-indicator-color'),
        );
        expect(getComputedStyle(currentSelected).backgroundColor).toBe(
            resolveTokenColor(currentSelected, '--cem-content-interaction-selected-background'),
        );

        const hover = nextEnabledDay(parts, currentSelected);
        const semanticState = daySemanticState(parts);
        const hoverGeometry = size(hover);
        await userEvent.hover(hover);
        expect(getComputedStyle(hover).backgroundColor).toBe(
            resolveTokenColor(hover, '--cem-content-interaction-hover-background'),
        );
        expect(daySemanticState(parts)).toEqual(semanticState);
        expect(size(hover)).toEqual(hoverGeometry);
        expect(size(currentSelected)).toEqual(baselineDayGeometry);
        await userEvent.unhover(hover);

        await userEvent.keyboard('{Escape}');
        await waitForDialog(parts.dialog, false);
        expect(parts.input.value).toBe(today);
        expect(requiredElement(root, '#current-date')).toBe(inputIdentity);
        expect(attributes(parts.host)).toEqual(hostAttributes);
        expect(geometry(parts.input)).toEqual(baselineInputGeometry);
        expect(mutationEvents).toEqual([]);
    });

    it('supports a silent expanded property while live bounds update validity and calendar suppression', async () => {
        const root = await renderFixture();
        const parts = datepickerParts(root, '#arrival-datepicker');
        const host = parts.host as HTMLElement & { expanded: boolean };
        const events = nativeValueEvents(parts.input);

        host.expanded = true;
        await waitForDialog(parts.dialog, true);
        expect(host.expanded).toBe(true);
        expect(host.hasAttribute('expanded')).toBe(false);

        host.setAttribute('max', '2026-08-10');
        await waitFor(() => parts.input.validity.customError, 'live date range validity');
        expect(parts.input.getAttribute('aria-invalid')).toBe('true');
        expect(dayFor(parts, '2026-08-11').getAttribute('aria-disabled')).toBe('true');
        expect(events).toEqual([]);
        host.expanded = false;
        await waitForDialog(parts.dialog, false);
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness(runtime);
        const root = await harness.render(datepickerContractFixture);
        await waitForSelector(root, '#arrival-datepicker > .cem-datepicker > input[slot="input"]');
        await waitForSelector(root, '#arrival-datepicker > .cem-datepicker > dialog.cem-datepicker__dialog');
        return root;
    }
});

interface DatepickerParts {
    apply: HTMLButtonElement;
    cancel: HTMLButtonElement;
    dialog: HTMLDialogElement;
    heading: HTMLElement;
    host: HTMLElement;
    input: HTMLInputElement;
    inputs: HTMLInputElement[];
    owner: HTMLElement;
    toggle: HTMLButtonElement | null;
}

function datepickerParts(root: ParentNode, selector: string, requireSingleInput = true): DatepickerParts {
    const host = requiredElement<HTMLElement>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-datepicker');
    const inputs = [...owner.querySelectorAll<HTMLInputElement>(':scope > input[slot="input"]')];
    if (requireSingleInput && inputs.length !== 1) throw new Error(`Expected one datepicker input: ${selector}`);
    return {
        apply: requiredElement(owner, '[data-datepicker-action="apply"]'),
        cancel: requiredElement(owner, '[data-datepicker-action="cancel"]'),
        dialog: requiredElement(owner, ':scope > dialog.cem-datepicker__dialog'),
        heading: requiredElement(owner, '.cem-datepicker__heading'),
        host,
        input: inputs[0] ?? document.createElement('input'),
        inputs,
        owner,
        toggle: owner.querySelector<HTMLButtonElement>(':scope > button[slot="toggle"]'),
    };
}

function requiredToggle(parts: DatepickerParts): HTMLButtonElement {
    if (!parts.toggle) throw new Error('Missing required datepicker toggle');
    return parts.toggle;
}

function weekdayLabels(parts: DatepickerParts): string[] {
    return [...parts.dialog.querySelectorAll<HTMLElement>('[role="columnheader"]')]
        .map((element) => element.getAttribute('aria-label') ?? '');
}

function dayFor(parts: DatepickerParts, value: string): HTMLButtonElement {
    const day = parts.dialog.querySelector<HTMLButtonElement>(`[role="gridcell"][data-date="${value}"]`);
    if (!day) throw new Error(`Missing calendar date: ${value}`);
    return day;
}

function activeDay(parts: DatepickerParts): HTMLButtonElement | null {
    return parts.dialog.querySelector<HTMLButtonElement>('[role="gridcell"][data-active="true"]');
}

function rovingDays(parts: DatepickerParts): HTMLButtonElement[] {
    return [...parts.dialog.querySelectorAll<HTMLButtonElement>('[role="gridcell"][tabindex="0"]')];
}

function nextEnabledDay(parts: DatepickerParts, day: HTMLButtonElement): HTMLButtonElement {
    const days = [...parts.dialog.querySelectorAll<HTMLButtonElement>('[role="gridcell"]')];
    const index = days.indexOf(day);
    const result = days.slice(index + 1).find((candidate) => !candidate.disabled);
    if (!result) throw new Error('Missing next enabled date');
    return result;
}

function daySemanticState(parts: DatepickerParts): string[] {
    return [...parts.dialog.querySelectorAll<HTMLElement>('[role="gridcell"][data-date]')].map((day) => [
        day.dataset.date,
        day.dataset.active,
        day.getAttribute('aria-current'),
        day.getAttribute('aria-selected'),
        day.getAttribute('aria-disabled'),
    ].join(':'));
}

function nativeValueEvents(input: HTMLInputElement): string[] {
    const events: string[] = [];
    for (const name of ['input', 'change']) {
        input.addEventListener(name, (event) => events.push(`${event.type}:${event.target === input}:${event.isTrusted}`));
    }
    return events;
}

function setNativeValue(input: HTMLInputElement, value: string): void {
    input.value = value;
    input.dispatchEvent(new Event('input', { bubbles: true }));
}

function formatMonth(locale: string, value: string): string {
    return new Intl.DateTimeFormat(locale, { month: 'long', timeZone: 'UTC', year: 'numeric' })
        .format(dateFromValue(value));
}

function localizedWeekdays(locale: string): string[] {
    const firstDay = localeFirstDay(locale);
    const sunday = Date.UTC(2024, 0, 7);
    return Array.from({ length: 7 }, (_, index) =>
        new Intl.DateTimeFormat(locale, { timeZone: 'UTC', weekday: 'long' })
            .format(new Date(sunday + ((firstDay + index) % 7) * 86_400_000)),
    );
}

function localeFirstDay(locale: string): number {
    const localeValue = new Intl.Locale(locale) as Intl.Locale & {
        getWeekInfo?: () => { firstDay: number };
        weekInfo?: { firstDay: number };
    };
    const firstDay = localeValue.getWeekInfo?.().firstDay ?? localeValue.weekInfo?.firstDay ?? 7;
    return firstDay % 7;
}

function localToday(): string {
    const today = new Date();
    return [today.getFullYear(), today.getMonth() + 1, today.getDate()]
        .map((value, index) => String(value).padStart(index === 0 ? 4 : 2, '0'))
        .join('-');
}

function dateFromValue(value: string): Date {
    const [year, month, day] = value.split('-').map(Number);
    const date = new Date(0);
    date.setUTCHours(0, 0, 0, 0);
    date.setUTCFullYear(year, month - 1, day);
    return date;
}

function attributes(element: Element): Record<string, string> {
    return Object.fromEntries([...element.attributes].map((attribute) => [attribute.name, attribute.value]));
}

function geometry(element: Element): { height: number; width: number; x: number; y: number } {
    const rect = element.getBoundingClientRect();
    return { height: rect.height, width: rect.width, x: rect.x, y: rect.y };
}

function size(element: Element): { height: number; width: number } {
    const rect = element.getBoundingClientRect();
    return { height: rect.height, width: rect.width };
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

async function waitForDialog(dialog: HTMLDialogElement, open: boolean): Promise<void> {
    await waitFor(() => dialog.open === open, `datepicker dialog open=${String(open)}`);
}

async function waitForActiveDate(parts: DatepickerParts, value: string): Promise<void> {
    await waitFor(() => activeDay(parts)?.dataset.date === value, `active calendar date ${value}`);
    expect(document.activeElement).toBe(activeDay(parts));
    expect(rovingDays(parts)).toHaveLength(1);
}

async function waitForValue(input: HTMLInputElement, value: string): Promise<void> {
    await waitFor(() => input.value === value, `datepicker value ${value}`);
}

async function waitFor(predicate: () => boolean, label: string): Promise<void> {
    for (let attempt = 0; attempt < 120; attempt += 1) {
        if (predicate()) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for ${label}`);
}
