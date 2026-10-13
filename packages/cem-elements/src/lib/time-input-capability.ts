import type {
    CemProducedElementBehavior,
    CemProducedElementBehaviorContext,
    SerializedPayloadNode,
} from './cem-elements.js';
import { isCemEditorCompositionKey } from './form-control-capability.js';
import { registerCemSurfaceOwner } from './surface-session.js';
import { normalizeChoiceOptions } from './choice-options.js';

interface TimeBounds {
    interval: number;
    max: number;
    maxValue: string;
    min: number;
    minValue: string;
    valid: boolean;
}

interface TimeOption {
    disabled: boolean;
    label: string;
    value: string;
}

interface RenderTimeOption extends TimeOption {
    active: boolean;
    id: string;
    index: number;
    selected: boolean;
}

interface TimeInputState {
    activeIndex: number;
    authoringValid: boolean;
    connected: boolean;
    context?: CemProducedElementBehaviorContext;
    expanded: boolean;
    input: HTMLInputElement | null;
    listboxId: string;
    onClick?: EventListener;
    onDocumentPointerDown?: EventListener;
    onDocumentReset?: EventListener;
    onFocusOut?: EventListener;
    onInput?: EventListener;
    onKeyDown?: EventListener;
    onPointerDown?: EventListener;
    options: TimeOption[];
    owner: HTMLElement | null;
    payloadSignature: string;
    popup: HTMLElement | null;
    toggle: HTMLButtonElement | null;
    value: string;
    warnedSignature: string;
    generation: number;
    composing: boolean;
    compositionPresses: Set<string>;
    compositionAbort?: AbortController;
    ownerSlots: string[];
    releaseSurface?: () => void;
}

const DEFAULT_INTERVAL = 30;
const DEFAULT_MAX = '23:59';
const DEFAULT_MIN = '00:00';
const TIME_INPUT_STATES = new WeakMap<HTMLElement, TimeInputState>();
let timeInputSequence = 0;

export const CEM_TIME_INPUT_CAPABILITY: CemProducedElementBehavior = {
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
        state.generation++;
        state.compositionAbort = new AbortController();
        const signal = state.compositionAbort.signal;
        instance.addEventListener('compositionstart', event => { if (event.target === state.input) state.composing = true; }, { signal });
        instance.addEventListener('compositionend', event => { if (event.target === state.input) state.composing = false; }, { signal });
        instance.addEventListener('keydown', event => {
            if (event.target !== state.input) return;
            if (state.composing || isCemEditorCompositionKey(event) || event.keyCode === 229 || event.key === 'Process') state.compositionPresses.add(event.code || event.key);
        }, { capture: true, signal });
        instance.addEventListener('keyup', event => { if (event.target === state.input) state.compositionPresses.delete(event.code || event.key); }, { signal });
        state.onInput = (event) => handleNativeValueEvent(instance, state, event);
        state.onClick = (event) => handleClick(instance, state, event);
        state.onPointerDown = (event) => handlePointerDown(instance, state, event);
        state.onKeyDown = (event) => handleKeyDown(instance, state, event as KeyboardEvent);
        state.onFocusOut = () => {
            state.composing = false;
            state.compositionPresses.clear();
            queueMicrotask(() => {
                if (state.connected && state.expanded && !instance.contains(instance.ownerDocument.activeElement)) {
                    setExpanded(instance, state, false);
                }
            });
        };
        state.onDocumentPointerDown = (event) => {
            if (state.expanded && !instance.contains(event.target as Node | null)) {
                setExpanded(instance, state, false);
            }
        };
        state.onDocumentReset = (event) => {
            if (event.target !== state.input?.form) return;
            const generation = state.generation;
            timerWindow(instance).setTimeout(() => {
                if (!event.defaultPrevented && state.connected && state.generation === generation) {
                    setExpanded(instance, state, false);
                    synchronizeNativeValue(instance, state);
                }
            }, 0);
        };
        instance.addEventListener('input', state.onInput, true);
        instance.addEventListener('change', state.onInput, true);
        instance.addEventListener('click', state.onClick);
        instance.addEventListener('pointerdown', state.onPointerDown);
        instance.addEventListener('keydown', state.onKeyDown);
        instance.addEventListener('focusout', state.onFocusOut);
        instance.ownerDocument.addEventListener('pointerdown', state.onDocumentPointerDown, true);
        instance.ownerDocument.addEventListener('reset', state.onDocumentReset, true);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        synchronizeModel(instance, state, context.snapshot().payload.nodes);
        context.setSlices(renderSlices(state), { render: false });
    },
    rendered(instance) {
        synchronizeOwners(instance, stateFor(instance));
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        state.generation++;
        state.expanded = false;
        state.activeIndex = -1;
        state.releaseSurface?.();
        state.releaseSurface = undefined;
        state.compositionAbort?.abort();
        state.composing = false;
        state.compositionPresses.clear();
        hidePopup(state);
        if (state.onInput) {
            instance.removeEventListener('input', state.onInput, true);
            instance.removeEventListener('change', state.onInput, true);
        }
        if (state.onClick) instance.removeEventListener('click', state.onClick);
        if (state.onPointerDown) instance.removeEventListener('pointerdown', state.onPointerDown);
        if (state.onKeyDown) instance.removeEventListener('keydown', state.onKeyDown);
        if (state.onFocusOut) instance.removeEventListener('focusout', state.onFocusOut);
        if (state.onDocumentPointerDown) {
            instance.ownerDocument.removeEventListener('pointerdown', state.onDocumentPointerDown, true);
        }
        if (state.onDocumentReset) {
            instance.ownerDocument.removeEventListener('reset', state.onDocumentReset, true);
        }
        state.input = null;
        state.owner = null;
        state.popup = null;
        state.toggle = null;
    },
};

function stateFor(instance: HTMLElement): TimeInputState {
    let state = TIME_INPUT_STATES.get(instance);
    if (state) return state;
    timeInputSequence += 1;
    state = {
        activeIndex: -1,
        authoringValid: false,
        connected: false,
        expanded: false,
        input: null,
        listboxId: `cem-time-input-${timeInputSequence}-listbox`,
        options: [],
        owner: null,
        payloadSignature: '',
        popup: null,
        toggle: null,
        value: '',
        warnedSignature: '',
        generation: 0,
        composing: false,
        compositionPresses: new Set(),
        ownerSlots: [],
    };
    TIME_INPUT_STATES.set(instance, state);
    return state;
}

function synchronizeModel(
    instance: HTMLElement,
    state: TimeInputState,
    nodes: readonly SerializedPayloadNode[],
): void {
    state.ownerSlots = [...new Set(nodes.flatMap(node => node.kind === 'element' && ['input', 'toggle'].includes(node.attributes.slot) ? [node.attributes.slot] : []))];
    const signature = JSON.stringify(nodes);
    const bounds = normalizedBounds(instance);
    const payload = validatePayload(nodes, bounds);
    state.authoringValid = bounds.valid && payload.valid;
    if (signature !== state.payloadSignature) {
        state.payloadSignature = signature;
        if (state.value === '') state.value = payload.initialValue;
    }
    state.options = state.authoringValid ? payload.options : [];
    if (!state.authoringValid || isDisabled(instance)) {
        state.expanded = false;
        state.activeIndex = -1;
    } else if (!enabledOptionAt(state, state.activeIndex)) {
        state.activeIndex = selectedIndex(state);
    }

    const issue = bounds.valid ? payload.issue : 'min, max, and interval must define one non-overnight time range.';
    const warningSignature = issue ? `${signature}|${bounds.minValue}|${bounds.maxValue}|${bounds.interval}|${issue}` : '';
    if (issue && warningSignature !== state.warnedSignature) {
        state.warnedSignature = warningSignature;
        instance.ownerDocument.defaultView?.console.warn(`[time-input] ${issue}`);
    } else if (!issue) {
        state.warnedSignature = '';
    }
}

function validatePayload(nodes: readonly SerializedPayloadNode[], bounds: TimeBounds): {
    initialValue: string;
    issue: string | null;
    options: TimeOption[];
    valid: boolean;
} {
    const elements = nodes.filter(
        (node): node is Extract<SerializedPayloadNode, { kind: 'element' }> => node.kind === 'element',
    );
    const inputs = elements.filter((node) => node.tag === 'input' && node.attributes.slot === 'input');
    const toggles = elements.filter((node) => node.tag === 'button' && node.attributes.slot === 'toggle');
    const options = elements.filter((node) => node.tag === 'cem-option');
    const allowed = new Set(['button', 'cem-option', 'input']);
    let issue: string | null = null;

    if (inputs.length !== 1 || inputs[0]?.attributes.type?.toLowerCase() !== 'text') {
        issue = 'Author exactly one direct input[slot="input"][type="text"].';
    } else if (toggles.length > 1 || (toggles[0] && toggles[0].attributes.type?.toLowerCase() !== 'button')) {
        issue = 'The optional direct button[slot="toggle"] must use type="button".';
    } else if (elements.some((node) => !allowed.has(node.tag))) {
        issue = 'Only the input owner, optional toggle, and direct cem-option choices are allowed.';
    } else if (elements.some((node) => node.tag === 'button' && node.attributes.slot !== 'toggle')) {
        issue = 'Only button[slot="toggle"] is allowed as the optional toggle owner.';
    } else if (elements.some((node) => node.tag === 'input' && node.attributes.slot !== 'input')) {
        issue = 'Only input[slot="input"] is allowed as the value owner.';
    }

    let normalizedOptions: TimeOption[] = [];
    if (!issue && options.length > 0) {
        const normalized = normalizeChoiceOptions(nodes);
        issue = normalized.issue;
        normalizedOptions = normalized.options.map((option) => {
            const minutes = parseTime(option.value);
            if (minutes === null) {
                issue ??= `Option value "${option.value}" must use canonical HH:mm.`;
            }
            return {
                disabled: option.disabled || minutes === null || minutes < bounds.min || minutes > bounds.max,
                label: option.label,
                value: option.value,
            };
        });
    } else if (!issue) {
        normalizedOptions = generatedOptions(bounds);
    }

    return {
        initialValue: inputs[0]?.attributes.value ?? '',
        issue,
        options: issue ? [] : normalizedOptions,
        valid: issue === null,
    };
}

function generatedOptions(bounds: TimeBounds): TimeOption[] {
    if (!bounds.valid) return [];
    const options: TimeOption[] = [];
    for (let minutes = bounds.min; minutes <= bounds.max; minutes += bounds.interval) {
        const value = formatTime(minutes);
        options.push({ disabled: false, label: value, value });
    }
    return options;
}

function normalizedBounds(instance: HTMLElement): TimeBounds {
    const minValue = instance.getAttribute('min') ?? DEFAULT_MIN;
    const maxValue = instance.getAttribute('max') ?? DEFAULT_MAX;
    const min = parseTime(minValue);
    const max = parseTime(maxValue);
    const intervalValue = Number(instance.getAttribute('interval') ?? DEFAULT_INTERVAL);
    const interval = Number.isInteger(intervalValue) && intervalValue > 0 && intervalValue <= 1440
        ? intervalValue
        : Number.NaN;
    return {
        interval,
        max: max ?? -1,
        maxValue,
        min: min ?? -1,
        minValue,
        valid: min !== null && max !== null && min <= max && Number.isFinite(interval),
    };
}

function renderSlices(state: TimeInputState): Record<string, unknown> {
    const options: RenderTimeOption[] = state.options.map((option, index) => ({
        ...option,
        active: state.expanded && index === state.activeIndex,
        id: `${state.listboxId}-option-${index}`,
        index,
        selected: option.value === state.value,
    }));
    return {
        activeOptionId: activeOptionId(state),
        ownerSlots: state.ownerSlots,
        expanded: state.expanded,
        listboxId: state.listboxId,
        mode: state.authoringValid ? 'valid' : 'invalid',
        options,
    };
}

function synchronizeOwners(instance: HTMLElement, state: TimeInputState): void {
    const owner = instance.querySelector<HTMLElement>(':scope > [part~="root"]');
    const popup = owner?.querySelector<HTMLElement>(':scope > [part~="popup"]') ?? null;
    const inputs = owner
        ? [...owner.querySelectorAll<HTMLInputElement>(':scope > input[slot="input"]')]
        : [];
    const toggles = owner
        ? [...owner.querySelectorAll<HTMLButtonElement>(':scope > button[slot="toggle"]')]
        : [];
    const input = inputs.length === 1 && inputs[0]?.type === 'text' ? inputs[0] : null;
    const toggle = toggles.length === 1 && toggles[0]?.type === 'button' ? toggles[0] : null;
    const valid = state.authoringValid && Boolean(owner && popup && input && toggles.length <= 1);

    state.owner = owner;
    if (state.popup !== popup || !state.releaseSurface) {
        state.releaseSurface?.();
        state.releaseSurface = popup ? registerCemSurfaceOwner(instance, popup, 'time-input', () => {
            synchronizeAvailability(instance, state);
            if (state.expanded && (!canInteract(instance, state) || instance.closest('[hidden]:not([hidden="until-found"]),[inert]'))) {
                setExpanded(instance, state, false);
            }
        }) : undefined;
    }
    state.popup = popup;
    state.input = valid ? input : null;
    state.toggle = valid ? toggle : null;
    if (owner) owner.dataset.mode = valid ? 'valid' : 'invalid';
    if (!valid || !input || !popup) {
        state.expanded = false;
        state.activeIndex = -1;
        hidePopup(state);
        return;
    }

    synchronizeAvailability(instance, state);
    if (isDisabled(instance) || input.matches(':disabled') || input.readOnly) { state.expanded = false; state.activeIndex = -1; }
    input.required = instance.hasAttribute('required');
    input.setAttribute('role', 'combobox');
    input.setAttribute('aria-autocomplete', 'list');
    input.setAttribute('aria-haspopup', 'listbox');
    input.setAttribute('aria-controls', state.listboxId);
    input.setAttribute('aria-expanded', String(state.expanded));
    if (state.expanded && activeOptionId(state)) input.setAttribute('aria-activedescendant', activeOptionId(state));
    else input.removeAttribute('aria-activedescendant');

    if (toggle) {
        toggle.setAttribute('aria-haspopup', 'listbox');
        toggle.setAttribute('aria-controls', state.listboxId);
        toggle.setAttribute('aria-expanded', String(state.expanded));
    }
    popup.setAttribute('aria-label', popupLabel(input));

    if (input.value !== state.value) {
        state.value = input.value;
        state.activeIndex = selectedIndex(state);
        state.context?.setSlices(renderSlices(state));
        return;
    }
    synchronizeValidity(instance, input);
    synchronizePopup(state);
}

// Native availability must not wait for asynchronous option rendering. The
// surface observer already tracks disabled ancestors and the exact live owners.
function synchronizeAvailability(instance: HTMLElement, state: TimeInputState): void {
    const disabled = isDisabled(instance);
    for (const control of [state.input, state.toggle]) {
        if (control && control.disabled !== disabled) control.disabled = disabled;
    }
}

function synchronizeNativeValue(instance: HTMLElement, state: TimeInputState): void {
    const input = state.input;
    if (!input) return;
    state.value = input.value;
    state.activeIndex = selectedIndex(state);
    synchronizeValidity(instance, input);
    state.context?.setSlices(renderSlices(state));
}

function synchronizeValidity(instance: HTMLElement, input: HTMLInputElement): void {
    const value = input.value;
    const bounds = normalizedBounds(instance);
    const minutes = value === '' ? null : parseTime(value);
    let message = '';
    if (value !== '' && minutes === null) message = 'Enter a time in HH:mm.';
    else if (minutes !== null && (minutes < bounds.min || minutes > bounds.max)) {
        message = `Choose a time from ${bounds.minValue} through ${bounds.maxValue}.`;
    }
    input.setCustomValidity(message);
    input.toggleAttribute(
        'aria-invalid',
        instance.hasAttribute('invalid') || Boolean(message) || (input.required && value === ''),
    );
    if (input.hasAttribute('aria-invalid')) input.setAttribute('aria-invalid', 'true');
}

function handleNativeValueEvent(instance: HTMLElement, state: TimeInputState, event: Event): void {
    if (event.target !== state.input) return;
    synchronizeNativeValue(instance, state);
}

function handlePointerDown(instance: HTMLElement, state: TimeInputState, event: Event): void {
    const option = optionElement(instance, event.target);
    if (!option) return;
    const index = Number.parseInt(option.dataset.optionIndex ?? '', 10);
    if (enabledOptionAt(state, index)) event.preventDefault();
}

function handleClick(instance: HTMLElement, state: TimeInputState, event: Event): void {
    if (!canInteract(instance, state)) return;
    const target = event.target;
    if (target === state.input) {
        setExpanded(instance, state, true);
        return;
    }
    if (target instanceof Node && state.toggle?.contains(target)) {
        const expanded = !state.expanded;
        state.input?.focus({ preventScroll: true });
        setExpanded(instance, state, expanded);
        return;
    }
    const option = optionElement(instance, target);
    if (!option) return;
    const index = Number.parseInt(option.dataset.optionIndex ?? '', 10);
    commitIndex(instance, state, index);
}

function handleKeyDown(instance: HTMLElement, state: TimeInputState, event: KeyboardEvent): void {
    if (event.target !== state.input || !canInteract(instance, state)) return;
    if (event.defaultPrevented || state.composing || state.compositionPresses.has(event.code || event.key) || isCemEditorCompositionKey(event) || event.keyCode === 229 || event.ctrlKey || event.metaKey || event.altKey || (event.shiftKey && event.key !== 'Tab')) return;
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        if (!hasEnabledOptions(state)) return;
        event.preventDefault();
        const direction = event.key === 'ArrowDown' ? 1 : -1;
        if (!state.expanded) {
            if (state.input) state.value = state.input.value;
            state.expanded = true;
            const selected = selectedIndex(state);
            state.activeIndex = selected >= 0 && enabledOptionAt(state, selected)
                ? selected
                : direction > 0 ? firstEnabledIndex(state) : lastEnabledIndex(state);
            update(instance, state);
            return;
        }
        moveActive(state, direction);
        update(instance, state);
        return;
    }
    if (event.key === 'Enter' && state.expanded && enabledOptionAt(state, state.activeIndex)) {
        event.preventDefault();
        commitIndex(instance, state, state.activeIndex);
        return;
    }
    if (event.key === 'Escape' && state.expanded) {
        event.preventDefault();
        setExpanded(instance, state, false);
        return;
    }
    if (event.key === 'Tab' && state.expanded) setExpanded(instance, state, false);
}

function setExpanded(instance: HTMLElement, state: TimeInputState, expanded: boolean): void {
    const next = expanded && canInteract(instance, state) && hasEnabledOptions(state);
    if (state.expanded === next) return;
    state.expanded = next;
    if (next) {
        if (state.input) state.value = state.input.value;
        const selected = selectedIndex(state);
        state.activeIndex = selected >= 0 && enabledOptionAt(state, selected) ? selected : firstEnabledIndex(state);
    } else {
        state.activeIndex = -1;
    }
    update(instance, state);
}

function commitIndex(instance: HTMLElement, state: TimeInputState, index: number): void {
    const option = enabledOptionAt(state, index);
    const input = state.input;
    if (!option || !input || !canInteract(instance, state)) return;
    input.value = option.value;
    state.value = option.value;
    state.activeIndex = index;
    state.expanded = false;
    synchronizeValidity(instance, input);
    update(instance, state);
    // Publish the closed native presentation before application observers run.
    hidePopup(state);
    input.setAttribute('aria-expanded', 'false');
    input.removeAttribute('aria-activedescendant');
    state.toggle?.setAttribute('aria-expanded', 'false');
    input.focus({ preventScroll: true });
    const generation = state.generation;
    input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
    if (state.connected && generation === state.generation) input.dispatchEvent(new Event('change', { bubbles: true }));
}

function update(_instance: HTMLElement, state: TimeInputState): void {
    state.context?.setSlices(renderSlices(state));
    // A retained, matching native list can change visibility without waiting for
    // another publication of the same options. Changed lists await rendering.
    const options = state.popup ? [...state.popup.querySelectorAll<HTMLElement>(':scope > [role=option]')] : [];
    if (!state.expanded || options.length === state.options.length && options.every((node, index) =>
        node.dataset.value === state.options[index].value && node.getAttribute('aria-disabled') === String(state.options[index].disabled))) {
        for (const [index, node] of options.entries()) {
            node.setAttribute('data-active', String(state.expanded && index === state.activeIndex));
            node.setAttribute('aria-selected', String(state.options[index].value === state.value));
        }
        synchronizePopup(state);
    }
}

function moveActive(state: TimeInputState, direction: 1 | -1): void {
    let index = state.activeIndex + direction;
    while (index >= 0 && index < state.options.length && !enabledOptionAt(state, index)) index += direction;
    if (enabledOptionAt(state, index)) state.activeIndex = index;
}

function selectedIndex(state: TimeInputState): number {
    return state.options.findIndex((option) => option.value === state.value);
}

function firstEnabledIndex(state: TimeInputState): number {
    return state.options.findIndex((option) => !option.disabled);
}

function lastEnabledIndex(state: TimeInputState): number {
    for (let index = state.options.length - 1; index >= 0; index -= 1) {
        if (!state.options[index]?.disabled) return index;
    }
    return -1;
}

function enabledOptionAt(state: TimeInputState, index: number): TimeOption | null {
    const option = index >= 0 && index < state.options.length ? state.options[index] : undefined;
    return option && !option.disabled ? option : null;
}

function hasEnabledOptions(state: TimeInputState): boolean {
    return state.options.some((option) => !option.disabled);
}

function activeOptionId(state: TimeInputState): string {
    return state.expanded && enabledOptionAt(state, state.activeIndex)
        ? `${state.listboxId}-option-${state.activeIndex}`
        : '';
}

function optionElement(instance: HTMLElement, target: EventTarget | null): HTMLElement | null {
    const element = target instanceof Element ? target.closest<HTMLElement>('[data-option-index]') : null;
    return element && element.parentElement === stateFor(instance).popup ? element : null;
}

function synchronizePopup(state: TimeInputState): void {
    if (!state.popup) return;
    state.input?.setAttribute('aria-expanded', String(state.expanded));
    state.toggle?.setAttribute('aria-expanded', String(state.expanded));
    if (state.expanded && activeOptionId(state)) state.input?.setAttribute('aria-activedescendant', activeOptionId(state));
    else state.input?.removeAttribute('aria-activedescendant');
    if (state.expanded && !state.popup.matches(':popover-open')) state.popup.showPopover();
    else if (!state.expanded && state.popup.matches(':popover-open')) state.popup.hidePopover();
    if (state.expanded && state.activeIndex >= 0) {
        state.popup.querySelector<HTMLElement>(`#${activeOptionId(state)}`)?.scrollIntoView({ block: 'nearest' });
    }
}

function hidePopup(state: TimeInputState): void {
    if (state.popup?.matches(':popover-open')) state.popup.hidePopover();
}

function canInteract(instance: HTMLElement, state: TimeInputState): boolean {
    return state.connected && state.authoringValid && !isDisabled(instance) && Boolean(state.input && !state.input.matches(':disabled') && !state.input.readOnly && state.popup);
}

function isDisabled(instance: HTMLElement): boolean {
    return instance.hasAttribute('disabled');
}

function popupLabel(input: HTMLInputElement): string {
    const explicit = input.getAttribute('aria-label')?.trim();
    if (explicit) return `${explicit} options`;
    const labelledBy = (input.getAttribute('aria-labelledby') ?? '').trim().split(/\s+/).filter(Boolean);
    const referenced = labelledBy
        .map((id) => input.ownerDocument.getElementById(id)?.textContent?.trim() ?? '')
        .filter(Boolean)
        .join(' ');
    if (referenced) return `${referenced} options`;
    const labels = [...(input.labels ?? [])].map((label) => label.textContent?.trim() ?? '').filter(Boolean).join(' ');
    return labels ? `${labels} options` : 'Time options';
}

function parseTime(value: string): number | null {
    const match = /^([01]\d|2[0-3]):([0-5]\d)$/.exec(value);
    return match ? Number(match[1]) * 60 + Number(match[2]) : null;
}

function formatTime(minutes: number): string {
    return `${String(Math.floor(minutes / 60)).padStart(2, '0')}:${String(minutes % 60).padStart(2, '0')}`;
}

function timerWindow(instance: HTMLElement): Window {
    const view = instance.ownerDocument.defaultView;
    if (!view) throw new Error('time-input requires an attached browser document');
    return view;
}

function installHostApi(instance: HTMLElement, state: TimeInputState): void {
    Object.defineProperty(instance, 'expanded', {
        configurable: true,
        enumerable: true,
        get: () => state.expanded,
        set: (value: unknown) => setExpanded(instance, state, Boolean(value)),
    });
}
