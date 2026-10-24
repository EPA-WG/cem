import { expect, waitFor as storyWaitFor } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-datepicker.xhtml?raw';
const meta = preview.meta({ component: 'cem-datepicker', title: 'CEM Components/cem-datepicker', loaders: [async () => { await loadCemDeclaration('cem-datepicker', declarationSource); return {}; }] });

export const NativeOwners = meta.story({
    name: 'keeps one direct native text input and optional native toggle as the exact owners',
    render: () => fixtureFor('arrival-datepicker'),
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native keyboard and pointer contract.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        expect(datepickerContractFixture).not.toMatch(/<script\b/i);
        expect(datepickerContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-datepicker')) await whenCemRendered(host);
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
        expect(parts.input).toHaveAccessibleName('Arrival date');
        expect(requiredToggle(parts)).toHaveAccessibleName('Choose arrival date');
        expect(parts.host.hasAttribute('name')).toBe(false);
        expect(new FormData(requiredElement<HTMLFormElement>(root, '#datepicker-form')).get('arrival-date')).toBe('2026-08-11');
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    },
});

export const ValueValidationAndReset = meta.story({
    name: 'keeps canonical value, locale calendar, validation, form, and reset ownership on the input',
    render: () => fixtureFor('arrival-datepicker', 'localized-datepicker'),
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native keyboard and pointer contract.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-datepicker')) await whenCemRendered(host);
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
        expect(new FormData(requiredElement<HTMLFormElement>(root, '#datepicker-form')).get('arrival-date')).toBe('');
        requiredElement<HTMLButtonElement>(root, '#datepicker-reset').click();
        await waitForValue(arrival.input, '2026-08-11');
        expect(arrival.input.validity.valid).toBe(true);
    },
});

export const RovingCalendarKeyboard = meta.story({
    name: 'moves one roving grid focus owner through day, week, month, and year navigation before confirmation',
    render: () => fixtureFor('arrival-datepicker'),
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native keyboard and pointer contract.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-datepicker')) await whenCemRendered(host);
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
        await nextRenderFrame();
        expect(parts.input.value).toBe('2027-09-22');
        expect(document.activeElement).toBe(parts.input);
        expect(events).toEqual(['input:true:false', 'change:true:false']);
    },
});

export const DraftAndDismissal = meta.story({
    name: 'keeps pointer drafts and cancel, Escape, and backdrop dismissal silent until Apply commits',
    render: () => fixtureFor('arrival-datepicker'),
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native keyboard and pointer contract.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-datepicker')) await whenCemRendered(host);
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
    },
});

export const DisabledAndMalformedOwners = meta.story({
    name: 'suppresses every disabled route and rejects malformed owner vocabulary without substitution',
    render: () => fixtureFor('disabled-datepicker', 'malformed-datepicker'),
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native keyboard and pointer contract.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-datepicker')) await whenCemRendered(host);
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
    },
});

export const IndependentPaintAndGeometry = meta.story({
    name: 'keeps today, selected, focus, hover, and disabled paint independent and geometry-stable',
    render: () => fixtureFor('current-datepicker'),
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native keyboard and pointer contract.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-datepicker')) await whenCemRendered(host);
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
    },
});

export const LiveBoundsAndExpanded = meta.story({
    name: 'supports a silent expanded property while live bounds update validity and calendar suppression',
    render: () => fixtureFor('arrival-datepicker'),
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native keyboard and pointer contract.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-datepicker')) await whenCemRendered(host);
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
    },
});

export const SuperCompactCalendarGeometry = meta.story({
    render: () => `<section data-cem-density="super-compact" data-cem-spacing="normal">${fixtureFor('arrival-datepicker')}</section>`,
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted native calendar navigation and commit; intrinsic calendar columns remain visible when control/coupling minima are zero.' } } },
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const boundary = requiredElement<HTMLElement>(canvasElement, '[data-cem-density]');
        const theme = requiredElement<HTMLElement>(boundary, '[data-datepicker-contract-fixture]');
        await whenCemRendered(requiredElement<HTMLElement>(boundary, '#arrival-datepicker'));
        const parts = datepickerParts(boundary, '#arrival-datepicker');
        const events = nativeValueEvents(parts.input);
        parts.input.focus();
        await userEvent.keyboard('{ArrowDown}');
        await waitForDialog(parts.dialog, true);
        await waitForActiveDate(parts, '2026-08-11');

        const assertCalendarColumns = () => {
            const headers = [...parts.dialog.querySelectorAll<HTMLElement>('[role=columnheader]')];
            expect(headers).toHaveLength(7);
            const grid = requiredElement<HTMLElement>(parts.dialog, '[role=grid]');
            const gridBox = grid.getBoundingClientRect();
            for (const [index, header] of headers.entries()) {
                const box = header.getBoundingClientRect();
                expect(box.width).toBeGreaterThan(0);
                expect(box.left).toBeGreaterThanOrEqual(gridBox.left - 0.5);
                expect(box.right).toBeLessThanOrEqual(gridBox.right + 0.5);
                if (index > 0) expect(box.left).toBeGreaterThanOrEqual(headers[index - 1].getBoundingClientRect().right);
            }
            for (const cell of parts.dialog.querySelectorAll<HTMLElement>('[role=columnheader],button[role=gridcell]')) {
                const text = document.createRange(); text.selectNodeContents(cell);
                expect(text.getBoundingClientRect().width).toBeLessThanOrEqual(cell.clientWidth + 0.5);
                text.detach();
            }
            expect(gridBox.width).toBeGreaterThanOrEqual(7 * parseFloat(getComputedStyle(parts.dialog).lineHeight));
            expect(parts.dialog.getBoundingClientRect().width).toBeLessThanOrEqual(window.innerWidth + 0.5);
        };

        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            theme.className = `cem-theme-${mode}`; await nextRenderFrame();
            expect(getComputedStyle(parts.dialog).getPropertyValue('--cem-coupling-zone-min').trim()).toBe('0px');
            expect(getComputedStyle(dayFor(parts, '2026-08-11')).paddingBlockStart).toBe('0px');
            assertCalendarColumns();
        }
        boundary.style.cssText = '--cem-typography-ui-font-size:20px;--cem-typography-ui-line-height:1.5;--cem-typography-data-font-size:20px;--cem-typography-data-line-height:1.5;';
        await nextRenderFrame(); assertCalendarColumns();
        boundary.style.cssText = '';
        boundary.setAttribute('data-cem-density', 'normal'); await nextRenderFrame();
        expect(getComputedStyle(parts.dialog).getPropertyValue('--cem-coupling-zone-min').trim()).toBe('3rem');
        expect(parts.dialog.getBoundingClientRect().width).toBeGreaterThanOrEqual(7 * 48);
        assertCalendarColumns();
        boundary.setAttribute('data-cem-density', 'super-compact');
        boundary.style.cssText = '--cem-coupling-zone-min:56px;'; await nextRenderFrame();
        expect(parts.dialog.getBoundingClientRect().width).toBeGreaterThanOrEqual(7 * 56);
        assertCalendarColumns();
        // Exercise the existing viewport-relative cap through its public gutter token.
        boundary.style.cssText = '--cem-gap-block:calc(50vi - 90px);'; await nextRenderFrame();
        expect(parts.dialog.getBoundingClientRect().width).toBeGreaterThanOrEqual(179.5);
        expect(parts.dialog.getBoundingClientRect().width).toBeLessThanOrEqual(180.5);
        boundary.style.cssText = ''; await nextRenderFrame(); assertCalendarColumns();

        expect(parts.input.value).toBe('2026-08-11'); expect(events).toEqual([]);
        await userEvent.keyboard('{ArrowRight}'); await waitForActiveDate(parts, '2026-08-12');
        await userEvent.keyboard('{Enter}');
        await waitFor(() => dayFor(parts, '2026-08-12').getAttribute('aria-selected') === 'true', 'compact calendar draft selection');
        expect(events).toEqual([]);
        await userEvent.click(parts.apply); await waitForDialog(parts.dialog, false); await nextRenderFrame();
        expect(parts.input.value).toBe('2026-08-12');
        expect(events).toEqual(['input:true:false', 'change:true:false']);
        expect(document.activeElement).toBe(parts.input);
    },
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
        apply: requiredElement<HTMLButtonElement>(owner, '[data-calendar-action="apply"]'),
        cancel: requiredElement<HTMLButtonElement>(owner, '[data-calendar-action="cancel"]'),
        dialog: requiredElement<HTMLDialogElement>(owner, ':scope > dialog.cem-datepicker__dialog'),
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

async function waitForDialog(dialog: HTMLDialogElement, open: boolean): Promise<void> {
    await waitFor(() => dialog.open === open, `datepicker dialog open=${String(open)}`);
}

async function waitForActiveDate(parts: DatepickerParts, value: string): Promise<void> {
    await waitFor(() => activeDay(parts)?.dataset.date === value, `active calendar date ${value}`);
    await whenCemRendered(parts.host);
    expect(document.activeElement).toBe(activeDay(parts));
    expect(rovingDays(parts)).toHaveLength(1);
}

async function waitForValue(input: HTMLInputElement, value: string): Promise<void> {
    await waitFor(() => input.value === value, `datepicker value ${value}`);
}

async function waitFor(predicate: () => boolean, label: string): Promise<void> {
    await storyWaitFor(() => expect(predicate(), label).toBe(true), { timeout: 10000 });
}
async function nextRenderFrame(): Promise<void> {
    await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
}
function assertAriaReferenceIntegrity(root: ParentNode): void {
    for (const node of root.querySelectorAll('[aria-controls], [aria-labelledby], [aria-describedby]')) {
        for (const name of ['aria-controls', 'aria-labelledby', 'aria-describedby']) {
            for (const id of (node.getAttribute(name) ?? '').split(/\s+/).filter(Boolean)) {
                if (!node.ownerDocument.getElementById(id)) throw new Error(`Missing ARIA reference ${id}`);
            }
        }
    }
}

export const PresenceBoundaryYearsAndThemes = meta.story({
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `calendar-presence-${crypto.randomUUID()}`, source = await declare(declarationSource, tag);
            const host = document.createElement(tag) as HTMLElement & { expanded: boolean };
            host.innerHTML = '<input slot="input" type="text" name="date" value="0001-01-01" aria-label="Date"><button type="button" slot="toggle">Choose</button>';
            canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const input = requiredElement<HTMLInputElement>(host, 'input'), dialog = requiredElement<HTMLDialogElement>(host, 'dialog');
                for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
                    canvasElement.className = `cem-theme-${mode}`;
                    for (const value of ['', 'false']) {
                        host.setAttribute('disabled', value); host.setAttribute('required', value); host.setAttribute('invalid', value);
                        await runtime.whenRenderSettled(host);
                        expect(input.disabled).toBe(true); expect(input.required).toBe(true); expect(input.getAttribute('aria-invalid')).toBe('true');
                        host.expanded = true; expect(dialog.open).toBe(false);
                    }
                }
                for (const name of ['disabled', 'required', 'invalid']) host.removeAttribute(name);
                host.setAttribute('lang', 'not_a_locale'); await runtime.whenRenderSettled(host);
                for (const value of ['0001-01-01', '9999-12-31']) {
                    input.value = value; input.dispatchEvent(new Event('input', { bubbles: true }));
                    host.expanded = true; await runtime.whenRenderSettled(host);
                    expect(dialog.open).toBe(true); expect(input.validity.valid).toBe(true);
                    expect(dialog.querySelectorAll('[role=gridcell]')).toHaveLength(42);
                    expect(dialog.querySelectorAll('span[role=gridcell]').length).toBeGreaterThan(0);
                    expect(dialog.querySelectorAll('[role=gridcell][tabindex="0"]')).toHaveLength(1);
                    host.expanded = false; await runtime.whenRenderSettled(host);
                }
                expect(host.shadowRoot).toBeNull(); expect(host.querySelector('style')).toBeNull();
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); source.remove(); scope.dispose(); canvasElement.className = ''; }
        }
    },
});

function fixtureFor(...ids: string[]): string {
    const template = document.createElement('template');
    template.innerHTML = datepickerContractFixture;
    for (const host of template.content.querySelectorAll('cem-datepicker')) if (!ids.includes(host.id)) host.remove();
    return template.innerHTML;
}

const datepickerContractFixture = `<section class="cem-theme-light" data-datepicker-contract-fixture aria-label="Datepicker contract fixture">
  <button data-datepicker-focus-start type="button">Focus start</button>

  <form id="datepicker-form">
    <label for="arrival-date">Arrival date</label>
    <cem-datepicker
      id="arrival-datepicker"
      lang="en-US"
      min="2026-08-01"
      max="2027-12-31"
      required
    >
      <input
        id="arrival-date"
        slot="input"
        type="text"
        name="arrival-date"
        value="2026-08-11"
        autocomplete="off"
        aria-describedby="arrival-date-format"
      >
      <button id="arrival-date-toggle" slot="toggle" type="button" aria-label="Choose arrival date">Choose</button>
    </cem-datepicker>
    <span id="arrival-date-format">Use YYYY-MM-DD.</span>
    <button id="datepicker-submit" type="submit">Submit</button>
    <button id="datepicker-reset" type="reset">Reset</button>
  </form>

  <label for="localized-date">Localized date</label>
  <cem-datepicker id="localized-datepicker" lang="fr-FR" min="2026-12-01" max="2026-12-31">
    <input id="localized-date" slot="input" type="text" value="2026-12-24">
    <button id="localized-date-toggle" slot="toggle" type="button" aria-label="Choisir une date">Choisir</button>
  </cem-datepicker>

  <label for="current-date">Current date</label>
  <cem-datepicker id="current-datepicker" lang="en-US">
    <input id="current-date" slot="input" type="text">
    <button id="current-date-toggle" slot="toggle" type="button" aria-label="Choose current date">Choose</button>
  </cem-datepicker>

  <label for="disabled-date">Disabled date</label>
  <cem-datepicker id="disabled-datepicker" disabled>
    <input id="disabled-date" slot="input" type="text" value="2026-08-11">
    <button id="disabled-date-toggle" slot="toggle" type="button" aria-label="Choose disabled date">Choose</button>
  </cem-datepicker>

  <label for="invalid-date">Invalid date</label>
  <cem-datepicker id="invalid-datepicker" min="2026-01-01" max="2026-12-31" invalid>
    <input id="invalid-date" slot="input" type="text" value="2026-02-30">
  </cem-datepicker>

  <cem-datepicker id="malformed-datepicker">
    <input slot="input" type="text" value="2026-08-11" aria-label="First malformed date">
    <input slot="input" type="text" value="2026-08-12" aria-label="Second malformed date">
  </cem-datepicker>

  <button data-datepicker-focus-end type="button">Focus end</button>
</section>
`;
