import type {
    CemProducedElementBehavior,
    CemProducedElementBehaviorContext,
    SerializedPayloadNode,
} from '@epa-wg/cem-elements';

interface CalendarBounds {
    max: CalendarDate;
    maxValue: string;
    min: CalendarDate;
    minValue: string;
    valid: boolean;
}

interface CalendarDate {
    day: number;
    month: number;
    serial: number;
    value: string;
    year: number;
}

interface RenderCalendarDay {
    active: boolean;
    current: boolean;
    disabled: boolean;
    fullLabel: string;
    id: string;
    number: string;
    outside: boolean;
    selected: boolean;
    tabIndex: number;
    value: string;
}

interface RenderCalendarWeek {
    days: RenderCalendarDay[];
}

interface RenderWeekday {
    full: string;
    short: string;
}

type PendingFocus = 'apply' | 'cancel' | 'day' | 'next' | 'previous' | null;

interface DatepickerState {
    activeValue: string;
    authoringValid: boolean;
    bounds: CalendarBounds;
    connected: boolean;
    context?: CemProducedElementBehaviorContext;
    dialog: HTMLDialogElement | null;
    dialogId: string;
    displayMonth: number;
    displayYear: number;
    draftValue: string;
    expanded: boolean;
    firstDay: number;
    headingId: string;
    input: HTMLInputElement | null;
    locale: string | undefined;
    onClick?: EventListener;
    onDialogCancel?: EventListener;
    onDialogClose?: EventListener;
    onDocumentReset?: EventListener;
    onInput?: EventListener;
    onKeyDown?: EventListener;
    owner: HTMLElement | null;
    payloadSignature: string;
    pendingCommit: boolean;
    pendingFocus: PendingFocus;
    returnFocus: boolean;
    toggle: HTMLButtonElement | null;
    value: string;
    warnedSignature: string;
}

const DAY_MS = 86_400_000;
const DEFAULT_MAX = '9999-12-31';
const DEFAULT_MIN = '0001-01-01';
const DATEPICKER_STATES = new WeakMap<HTMLElement, DatepickerState>();
let datepickerSequence = 0;

export const CEM_DATEPICKER_BEHAVIOR: CemProducedElementBehavior = {
    constructed(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        installHostApi(instance, state);
    },
    connected(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        if (state.connected) return;
        state.connected = true;
        state.onInput = (event) => handleNativeValueEvent(instance, state, event);
        state.onClick = (event) => handleClick(instance, state, event as MouseEvent);
        state.onKeyDown = (event) => handleKeyDown(instance, state, event as KeyboardEvent);
        state.onDocumentReset = (event) => {
            if (event.target !== state.input?.form) return;
            timerWindow(instance).setTimeout(() => synchronizeNativeValue(instance, state), 0);
        };
        instance.addEventListener('input', state.onInput, true);
        instance.addEventListener('change', state.onInput, true);
        instance.addEventListener('click', state.onClick);
        instance.addEventListener('keydown', state.onKeyDown);
        instance.ownerDocument.addEventListener('reset', state.onDocumentReset, true);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        synchronizeModel(instance, state, context.snapshot().payload.nodes);
        context.setSlices(renderSlices(instance, state), { render: false });
    },
    rendered(instance) {
        synchronizeOwners(instance, stateFor(instance));
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.dialog?.open) state.dialog.close();
        detachDialogListeners(state);
        if (state.onInput) {
            instance.removeEventListener('input', state.onInput, true);
            instance.removeEventListener('change', state.onInput, true);
        }
        if (state.onClick) instance.removeEventListener('click', state.onClick);
        if (state.onKeyDown) instance.removeEventListener('keydown', state.onKeyDown);
        if (state.onDocumentReset) {
            instance.ownerDocument.removeEventListener('reset', state.onDocumentReset, true);
        }
        state.dialog = null;
        state.input = null;
        state.owner = null;
        state.toggle = null;
    },
};

function stateFor(instance: HTMLElement): DatepickerState {
    let state = DATEPICKER_STATES.get(instance);
    if (state) return state;
    datepickerSequence += 1;
    const min = requiredDate(DEFAULT_MIN);
    const max = requiredDate(DEFAULT_MAX);
    state = {
        activeValue: '',
        authoringValid: false,
        bounds: { max, maxValue: DEFAULT_MAX, min, minValue: DEFAULT_MIN, valid: true },
        connected: false,
        dialog: null,
        dialogId: `cem-datepicker-${datepickerSequence}-dialog`,
        displayMonth: 1,
        displayYear: 1970,
        draftValue: '',
        expanded: false,
        firstDay: 0,
        headingId: `cem-datepicker-${datepickerSequence}-heading`,
        input: null,
        locale: undefined,
        owner: null,
        payloadSignature: '',
        pendingCommit: false,
        pendingFocus: null,
        returnFocus: false,
        toggle: null,
        value: '',
        warnedSignature: '',
    };
    DATEPICKER_STATES.set(instance, state);
    return state;
}

function synchronizeModel(
    instance: HTMLElement,
    state: DatepickerState,
    nodes: readonly SerializedPayloadNode[],
): void {
    const signature = JSON.stringify(nodes);
    const payload = validatePayload(nodes);
    const bounds = normalizedBounds(instance);
    state.bounds = bounds;
    state.locale = localeFor(instance);
    state.firstDay = localeFirstDay(state.locale);
    state.authoringValid = payload.valid && bounds.valid;
    if (signature !== state.payloadSignature) {
        state.payloadSignature = signature;
        if (state.value === '') state.value = payload.initialValue;
    }
    if (!state.authoringValid || isDisabled(instance)) {
        state.expanded = false;
        state.pendingFocus = null;
    }
    if (state.expanded) {
        const active = clampDate(parseDate(state.activeValue) ?? todayDate(), bounds);
        state.activeValue = active.value;
        const draft = parseDate(state.draftValue);
        state.draftValue = draft ? clampDate(draft, bounds).value : active.value;
        state.displayYear = active.year;
        state.displayMonth = active.month;
    } else if (state.displayYear < 1 || state.displayYear > 9999) {
        const initial = openingDate(state);
        state.displayYear = initial.year;
        state.displayMonth = initial.month;
    }

    const issue = bounds.valid ? payload.issue : 'min and max must define one canonical ascending date range.';
    const warningSignature = issue ? `${signature}|${bounds.minValue}|${bounds.maxValue}|${issue}` : '';
    if (issue && warningSignature !== state.warnedSignature) {
        state.warnedSignature = warningSignature;
        instance.ownerDocument.defaultView?.console.warn(`[cem-datepicker] ${issue}`);
    } else if (!issue) {
        state.warnedSignature = '';
    }
}

function validatePayload(nodes: readonly SerializedPayloadNode[]): {
    initialValue: string;
    issue: string | null;
    valid: boolean;
} {
    const elements = nodes.filter(
        (node): node is Extract<SerializedPayloadNode, { kind: 'element' }> => node.kind === 'element',
    );
    const inputs = elements.filter((node) => node.tag === 'input' && node.attributes.slot === 'input');
    const toggles = elements.filter((node) => node.tag === 'button' && node.attributes.slot === 'toggle');
    let issue: string | null = null;
    if (inputs.length !== 1 || inputs[0]?.attributes.type?.toLowerCase() !== 'text') {
        issue = 'Author exactly one direct input[slot="input"][type="text"].';
    } else if (toggles.length > 1 || (toggles[0] && toggles[0].attributes.type?.toLowerCase() !== 'button')) {
        issue = 'The optional direct button[slot="toggle"] must use type="button".';
    } else if (elements.some((node) => node.tag !== 'input' && node.tag !== 'button')) {
        issue = 'Only the input owner and optional toggle are allowed.';
    } else if (elements.some((node) => node.tag === 'button' && node.attributes.slot !== 'toggle')) {
        issue = 'Only button[slot="toggle"] is allowed as the optional toggle owner.';
    } else if (elements.some((node) => node.tag === 'input' && node.attributes.slot !== 'input')) {
        issue = 'Only input[slot="input"] is allowed as the value owner.';
    }
    return { initialValue: inputs[0]?.attributes.value ?? '', issue, valid: issue === null };
}

function normalizedBounds(instance: HTMLElement): CalendarBounds {
    const minValue = instance.getAttribute('min') ?? DEFAULT_MIN;
    const maxValue = instance.getAttribute('max') ?? DEFAULT_MAX;
    const min = parseDate(minValue);
    const max = parseDate(maxValue);
    const valid = Boolean(min && max && min.serial <= max.serial);
    return {
        max: max ?? requiredDate(DEFAULT_MAX),
        maxValue,
        min: min ?? requiredDate(DEFAULT_MIN),
        minValue,
        valid,
    };
}

function renderSlices(instance: HTMLElement, state: DatepickerState): Record<string, unknown> {
    const heading = formatMonth(state.displayYear, state.displayMonth, state.locale);
    return {
        applyDisabled: !inBounds(parseDate(state.draftValue), state.bounds),
        dialogId: state.dialogId,
        expanded: state.expanded,
        heading,
        headingId: state.headingId,
        mode: state.authoringValid ? 'valid' : 'invalid',
        nextDisabled: !monthInBounds(shiftMonth(monthDate(state), 1), state.bounds),
        previousDisabled: !monthInBounds(shiftMonth(monthDate(state), -1), state.bounds),
        weekdays: renderWeekdays(state.locale, state.firstDay),
        weeks: renderWeeks(instance, state),
    };
}

function renderWeeks(instance: HTMLElement, state: DatepickerState): RenderCalendarWeek[] {
    const first = createDate(state.displayYear, state.displayMonth, 1) ?? state.bounds.min;
    const offset = (weekday(first) - state.firstDay + 7) % 7;
    const startSerial = first.serial - offset;
    const today = todayDate().value;
    const weeks: RenderCalendarWeek[] = [];
    for (let weekIndex = 0; weekIndex < 6; weekIndex += 1) {
        const days: RenderCalendarDay[] = [];
        for (let dayIndex = 0; dayIndex < 7; dayIndex += 1) {
            const date = dateFromSerial(startSerial + weekIndex * 7 + dayIndex);
            if (!date) {
                days.push({
                    active: false,
                    current: false,
                    disabled: true,
                    fullLabel: '',
                    id: `${state.dialogId}-blank-${weekIndex}-${dayIndex}`,
                    number: '',
                    outside: true,
                    selected: false,
                    tabIndex: -1,
                    value: '',
                });
                continue;
            }
            const disabled = !inBounds(date, state.bounds) || isDisabled(instance);
            const active = state.expanded && date.value === state.activeValue && !disabled;
            days.push({
                active,
                current: date.value === today,
                disabled,
                fullLabel: formatFullDate(date, state.locale),
                id: `${state.dialogId}-day-${date.value}`,
                number: String(date.day),
                outside: date.year !== state.displayYear || date.month !== state.displayMonth,
                selected: date.value === state.draftValue,
                tabIndex: active ? 0 : -1,
                value: date.value,
            });
        }
        weeks.push({ days });
    }
    return weeks;
}

function synchronizeOwners(instance: HTMLElement, state: DatepickerState): void {
    const owner = instance.querySelector<HTMLElement>(':scope > .cem-datepicker');
    const dialog = owner?.querySelector<HTMLDialogElement>(':scope > dialog.cem-datepicker__dialog') ?? null;
    const inputs = owner ? [...owner.querySelectorAll<HTMLInputElement>(':scope > input[slot="input"]')] : [];
    const toggles = owner ? [...owner.querySelectorAll<HTMLButtonElement>(':scope > button[slot="toggle"]')] : [];
    const input = inputs.length === 1 && inputs[0]?.type === 'text' ? inputs[0] : null;
    const toggle = toggles.length === 1 && toggles[0]?.type === 'button' ? toggles[0] : null;
    const valid = state.authoringValid && Boolean(owner && dialog && input && toggles.length <= 1);

    state.owner = owner;
    if (state.dialog !== dialog) {
        detachDialogListeners(state);
        state.dialog = dialog;
        attachDialogListeners(instance, state);
    }
    state.input = valid ? input : null;
    state.toggle = valid ? toggle : null;
    if (owner) owner.dataset.mode = valid ? 'valid' : 'invalid';
    if (!valid || !input || !dialog) {
        state.expanded = false;
        state.pendingFocus = null;
        if (dialog?.open) dialog.close();
        return;
    }

    const disabled = isDisabled(instance);
    input.disabled = disabled;
    input.required = instance.hasAttribute('required');
    input.setAttribute('role', 'combobox');
    input.setAttribute('aria-autocomplete', 'none');
    input.setAttribute('aria-haspopup', 'dialog');
    input.setAttribute('aria-controls', state.dialogId);
    input.setAttribute('aria-expanded', String(state.expanded));
    if (toggle) {
        toggle.disabled = disabled;
        toggle.setAttribute('aria-haspopup', 'dialog');
        toggle.setAttribute('aria-controls', state.dialogId);
        toggle.setAttribute('aria-expanded', String(state.expanded));
    }
    dialog.setAttribute('aria-label', `${controlLabel(input)} calendar`);

    if (input.value !== state.value) {
        state.value = input.value;
        synchronizeValidity(instance, input);
        state.context?.setSlices(renderSlices(instance, state));
        return;
    }
    synchronizeValidity(instance, input);
    synchronizeDialog(instance, state);
}

function attachDialogListeners(instance: HTMLElement, state: DatepickerState): void {
    if (!state.dialog) return;
    state.onDialogCancel = (event) => {
        event.preventDefault();
        closePicker(instance, state);
    };
    state.onDialogClose = () => {
        if (!state.expanded) return;
        state.expanded = false;
        state.pendingFocus = null;
        state.returnFocus = true;
        update(instance, state);
    };
    state.dialog.addEventListener('cancel', state.onDialogCancel);
    state.dialog.addEventListener('close', state.onDialogClose);
}

function detachDialogListeners(state: DatepickerState): void {
    if (state.dialog && state.onDialogCancel) state.dialog.removeEventListener('cancel', state.onDialogCancel);
    if (state.dialog && state.onDialogClose) state.dialog.removeEventListener('close', state.onDialogClose);
    state.onDialogCancel = undefined;
    state.onDialogClose = undefined;
}

function synchronizeDialog(instance: HTMLElement, state: DatepickerState): void {
    const dialog = state.dialog;
    const input = state.input;
    if (!dialog || !input) return;
    if (state.expanded && !dialog.open) dialog.showModal();
    else if (!state.expanded && dialog.open) dialog.close();

    if (state.expanded && dialog.open && state.pendingFocus) {
        const selector = state.pendingFocus === 'day'
            ? '[role="gridcell"][data-active="true"]'
            : `[data-datepicker-action="${state.pendingFocus}"]`;
        dialog.querySelector<HTMLElement>(selector)?.focus({ preventScroll: true });
        state.pendingFocus = null;
    }
    if (!state.expanded && state.returnFocus) {
        state.returnFocus = false;
        queueMicrotask(() => {
            if (input.isConnected) input.focus({ preventScroll: true });
        });
    }
    if (!state.expanded && state.pendingCommit) {
        state.pendingCommit = false;
        input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
        input.dispatchEvent(new Event('change', { bubbles: true }));
    }
}

function synchronizeNativeValue(instance: HTMLElement, state: DatepickerState): void {
    const input = state.input;
    if (!input) return;
    state.value = input.value;
    synchronizeValidity(instance, input);
    state.context?.setSlices(renderSlices(instance, state));
}

function synchronizeValidity(instance: HTMLElement, input: HTMLInputElement): void {
    const value = input.value;
    const bounds = normalizedBounds(instance);
    const date = value === '' ? null : parseDate(value);
    let message = '';
    if (value !== '' && !date) message = 'Enter a real date in YYYY-MM-DD.';
    else if (date && !inBounds(date, bounds)) {
        message = `Choose a date from ${bounds.minValue} through ${bounds.maxValue}.`;
    }
    input.setCustomValidity(message);
    input.toggleAttribute(
        'aria-invalid',
        instance.hasAttribute('invalid') || Boolean(message) || (input.required && value === ''),
    );
    if (input.hasAttribute('aria-invalid')) input.setAttribute('aria-invalid', 'true');
}

function handleNativeValueEvent(instance: HTMLElement, state: DatepickerState, event: Event): void {
    if (event.target !== state.input) return;
    synchronizeNativeValue(instance, state);
}

function handleClick(instance: HTMLElement, state: DatepickerState, event: MouseEvent): void {
    const target = event.target;
    if (target === state.toggle) {
        if (!canInteract(instance, state)) return;
        if (state.expanded) closePicker(instance, state);
        else openPicker(instance, state);
        return;
    }
    if (target === state.dialog && state.dialog) {
        const rect = state.dialog.getBoundingClientRect();
        const outside = event.clientX < rect.left || event.clientX > rect.right
            || event.clientY < rect.top || event.clientY > rect.bottom;
        if (outside) closePicker(instance, state);
        return;
    }
    const element = target instanceof Element ? target.closest<HTMLElement>('[data-datepicker-action], [data-date]') : null;
    if (!element || !state.dialog?.contains(element) || !state.expanded) return;
    const action = element.dataset.datepickerAction;
    if (action === 'cancel') {
        closePicker(instance, state);
        return;
    }
    if (action === 'apply') {
        commitDraft(instance, state);
        return;
    }
    if (action === 'previous' || action === 'next') {
        if (element instanceof HTMLButtonElement && element.disabled) return;
        moveDisplayedMonth(instance, state, action === 'next' ? 1 : -1, action);
        return;
    }
    if (element instanceof HTMLButtonElement && element.dataset.date && !element.disabled) {
        state.activeValue = element.dataset.date;
        state.draftValue = element.dataset.date;
        const date = requiredDate(element.dataset.date);
        state.displayYear = date.year;
        state.displayMonth = date.month;
        state.pendingFocus = 'day';
        update(instance, state);
    }
}

function handleKeyDown(instance: HTMLElement, state: DatepickerState, event: KeyboardEvent): void {
    if (event.target === state.input) {
        if (event.key === 'ArrowDown' && canInteract(instance, state)) {
            event.preventDefault();
            openPicker(instance, state);
        }
        return;
    }
    const day = event.target instanceof HTMLButtonElement && event.target.matches('[role="gridcell"][data-date]')
        ? event.target
        : null;
    if (!day || !state.expanded || day.disabled) return;
    const active = parseDate(state.activeValue);
    if (!active) return;
    let next: CalendarDate | null;
    if (event.key === 'ArrowLeft') next = dateFromSerial(active.serial - 1);
    else if (event.key === 'ArrowRight') next = dateFromSerial(active.serial + 1);
    else if (event.key === 'ArrowUp') next = dateFromSerial(active.serial - 7);
    else if (event.key === 'ArrowDown') next = dateFromSerial(active.serial + 7);
    else if (event.key === 'Home' || event.key === 'End') {
        const offset = (weekday(active) - state.firstDay + 7) % 7;
        next = dateFromSerial(active.serial + (event.key === 'Home' ? -offset : 6 - offset));
    } else if (event.key === 'PageUp' || event.key === 'PageDown') {
        const direction = event.key === 'PageDown' ? 1 : -1;
        next = shiftMonth(active, direction * (event.shiftKey || event.altKey ? 12 : 1));
    } else if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        state.draftValue = active.value;
        state.pendingFocus = 'day';
        update(instance, state);
        return;
    } else {
        return;
    }
    event.preventDefault();
    if (next) setActiveDate(instance, state, next);
}

function openPicker(instance: HTMLElement, state: DatepickerState): void {
    if (!canInteract(instance, state)) return;
    const opening = openingDate(state);
    state.activeValue = opening.value;
    state.draftValue = opening.value;
    state.displayYear = opening.year;
    state.displayMonth = opening.month;
    state.expanded = true;
    state.pendingFocus = 'day';
    state.returnFocus = false;
    update(instance, state);
}

function closePicker(instance: HTMLElement, state: DatepickerState): void {
    if (!state.expanded) return;
    state.expanded = false;
    state.pendingFocus = null;
    state.returnFocus = true;
    if (state.dialog?.open) state.dialog.close();
    update(instance, state);
}

function commitDraft(instance: HTMLElement, state: DatepickerState): void {
    const draft = parseDate(state.draftValue);
    const input = state.input;
    if (!draft || !input || !inBounds(draft, state.bounds)) return;
    input.value = draft.value;
    state.value = draft.value;
    synchronizeValidity(instance, input);
    state.pendingCommit = true;
    closePicker(instance, state);
}

function moveDisplayedMonth(
    instance: HTMLElement,
    state: DatepickerState,
    delta: number,
    focus: 'next' | 'previous',
): void {
    const active = parseDate(state.activeValue) ?? monthDate(state);
    const next = shiftMonth(active, delta);
    if (!next || !monthInBounds(next, state.bounds)) return;
    const bounded = clampDate(next, state.bounds);
    state.activeValue = bounded.value;
    state.displayYear = bounded.year;
    state.displayMonth = bounded.month;
    state.pendingFocus = focus;
    update(instance, state);
}

function setActiveDate(instance: HTMLElement, state: DatepickerState, date: CalendarDate): void {
    const bounded = clampDate(date, state.bounds);
    state.activeValue = bounded.value;
    state.displayYear = bounded.year;
    state.displayMonth = bounded.month;
    state.pendingFocus = 'day';
    update(instance, state);
}

function update(instance: HTMLElement, state: DatepickerState): void {
    state.context?.setSlices(renderSlices(instance, state));
}

function openingDate(state: DatepickerState): CalendarDate {
    const committed = parseDate(state.value);
    return committed && inBounds(committed, state.bounds) ? committed : clampDate(todayDate(), state.bounds);
}

function monthDate(state: DatepickerState): CalendarDate {
    return createDate(state.displayYear, state.displayMonth, 1) ?? state.bounds.min;
}

function monthInBounds(date: CalendarDate | null, bounds: CalendarBounds): boolean {
    if (!date) return false;
    const start = createDate(date.year, date.month, 1);
    const end = createDate(date.year, date.month, daysInMonth(date.year, date.month));
    return Boolean(start && end && end.serial >= bounds.min.serial && start.serial <= bounds.max.serial);
}

function inBounds(date: CalendarDate | null, bounds: CalendarBounds): date is CalendarDate {
    return Boolean(date && bounds.valid && date.serial >= bounds.min.serial && date.serial <= bounds.max.serial);
}

function clampDate(date: CalendarDate, bounds: CalendarBounds): CalendarDate {
    if (date.serial < bounds.min.serial) return bounds.min;
    if (date.serial > bounds.max.serial) return bounds.max;
    return date;
}

function parseDate(value: string): CalendarDate | null {
    const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
    if (!match) return null;
    return createDate(Number(match[1]), Number(match[2]), Number(match[3]));
}

function requiredDate(value: string): CalendarDate {
    const date = parseDate(value);
    if (!date) throw new Error(`Invalid internal calendar date: ${value}`);
    return date;
}

function createDate(year: number, month: number, day: number): CalendarDate | null {
    if (!Number.isInteger(year) || year < 1 || year > 9999 || month < 1 || month > 12 || day < 1 || day > 31) {
        return null;
    }
    const date = new Date(0);
    date.setUTCHours(0, 0, 0, 0);
    date.setUTCFullYear(year, month - 1, day);
    if (date.getUTCFullYear() !== year || date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day) return null;
    const value = `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
    return { day, month, serial: Math.floor(date.getTime() / DAY_MS), value, year };
}

function dateFromSerial(serial: number): CalendarDate | null {
    const date = new Date(serial * DAY_MS);
    return createDate(date.getUTCFullYear(), date.getUTCMonth() + 1, date.getUTCDate());
}

function shiftMonth(date: CalendarDate, delta: number): CalendarDate | null {
    const monthIndex = (date.year - 1) * 12 + date.month - 1 + delta;
    if (monthIndex < 0 || monthIndex >= 9999 * 12) return null;
    const year = Math.floor(monthIndex / 12) + 1;
    const month = monthIndex % 12 + 1;
    return createDate(year, month, Math.min(date.day, daysInMonth(year, month)));
}

function daysInMonth(year: number, month: number): number {
    const date = new Date(0);
    date.setUTCHours(0, 0, 0, 0);
    date.setUTCFullYear(year, month, 0);
    return date.getUTCDate();
}

function weekday(date: CalendarDate): number {
    return new Date(date.serial * DAY_MS).getUTCDay();
}

function todayDate(): CalendarDate {
    const today = new Date();
    return createDate(today.getFullYear(), today.getMonth() + 1, today.getDate()) ?? requiredDate('1970-01-01');
}

function renderWeekdays(locale: string | undefined, firstDay: number): RenderWeekday[] {
    const sunday = requiredDate('2024-01-07');
    return Array.from({ length: 7 }, (_, index) => {
        const date = dateFromSerial(sunday.serial + (firstDay + index) % 7) ?? sunday;
        return {
            full: formatDate(date, locale, { weekday: 'long' }),
            short: formatDate(date, locale, { weekday: 'short' }),
        };
    });
}

function formatMonth(year: number, month: number, locale: string | undefined): string {
    const date = createDate(year, month, 1) ?? requiredDate('1970-01-01');
    return formatDate(date, locale, { month: 'long', year: 'numeric' });
}

function formatFullDate(date: CalendarDate, locale: string | undefined): string {
    return formatDate(date, locale, { dateStyle: 'full' });
}

function formatDate(
    date: CalendarDate,
    locale: string | undefined,
    options: Intl.DateTimeFormatOptions,
): string {
    return new Intl.DateTimeFormat(locale, { ...options, timeZone: 'UTC' }).format(new Date(date.serial * DAY_MS));
}

function localeFor(instance: HTMLElement): string | undefined {
    return instance.lang
        || instance.closest<HTMLElement>('[lang]')?.lang
        || instance.ownerDocument.documentElement.lang
        || undefined;
}

function localeFirstDay(locale: string | undefined): number {
    const localeValue = new Intl.Locale(locale ?? 'en-US') as Intl.Locale & {
        getWeekInfo?: () => { firstDay: number };
        weekInfo?: { firstDay: number };
    };
    const firstDay = localeValue.getWeekInfo?.().firstDay ?? localeValue.weekInfo?.firstDay ?? 7;
    return firstDay % 7;
}

function controlLabel(input: HTMLInputElement): string {
    const explicit = input.getAttribute('aria-label')?.trim();
    if (explicit) return explicit;
    const labelledBy = (input.getAttribute('aria-labelledby') ?? '').trim().split(/\s+/).filter(Boolean);
    const referenced = labelledBy
        .map((id) => input.ownerDocument.getElementById(id)?.textContent?.trim() ?? '')
        .filter(Boolean)
        .join(' ');
    if (referenced) return referenced;
    const labels = [...(input.labels ?? [])].map((label) => label.textContent?.trim() ?? '').filter(Boolean).join(' ');
    return labels || 'Choose date';
}

function canInteract(instance: HTMLElement, state: DatepickerState): boolean {
    return state.authoringValid && !isDisabled(instance) && Boolean(state.input && state.dialog);
}

function isDisabled(instance: HTMLElement): boolean {
    return instance.hasAttribute('disabled');
}

function timerWindow(instance: HTMLElement): Window {
    const view = instance.ownerDocument.defaultView;
    if (!view) throw new Error('cem-datepicker requires an attached browser document');
    return view;
}

function installHostApi(instance: HTMLElement, state: DatepickerState): void {
    Object.defineProperty(instance, 'expanded', {
        configurable: true,
        enumerable: true,
        get: () => state.expanded,
        set: (value: unknown) => {
            if (value) openPicker(instance, state);
            else closePicker(instance, state);
        },
    });
}
