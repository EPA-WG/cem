import type {
    CemProducedElementBehavior,
    CemProducedElementBehaviorContext,
} from '@epa-wg/cem-elements';
import {
    normalizeChoiceOptions,
    type NormalizedChoiceGroup,
    type NormalizedChoiceOption,
} from './choice-options.js';

interface RenderOption extends NormalizedChoiceOption {
    active: boolean;
    hasChildren: boolean;
    id: string;
    index: number;
    selected: boolean;
}

interface RenderGroup {
    disabled: boolean;
    label: string;
    options: RenderOption[];
}

interface AutocompleteState {
    activeIndex: number;
    authoredValue: string | null;
    committedOptionValue: string | null;
    connected: boolean;
    context?: CemProducedElementBehaviorContext;
    defaultCommittedOptionValue: string | null;
    defaultDisplayValue: string;
    defaultValue: string;
    displayValue: string;
    edited: boolean;
    expanded: boolean;
    formDisabled: boolean;
    groups: NormalizedChoiceGroup[];
    initialized: boolean;
    labelId: string;
    listboxId: string;
    onChange?: EventListener;
    onClick?: EventListener;
    onDocumentPointerDown?: EventListener;
    onFocusIn?: EventListener;
    onFocusOut?: EventListener;
    onInput?: EventListener;
    onKeyDown?: EventListener;
    onPointerDown?: EventListener;
    options: NormalizedChoiceOption[];
    payloadSignature: string;
    pendingValue?: string;
    value: string;
    warnedPayloadSignature: string;
}

const AUTOCOMPLETE_STATES = new WeakMap<HTMLElement, AutocompleteState>();
let autocompleteSequence = 0;

export const CEM_AUTOCOMPLETE_BEHAVIOR: CemProducedElementBehavior = {
    formAssociated: true,
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

        state.onInput = (event) => handleInput(instance, state, event);
        state.onChange = (event) => handleChange(instance, state, event);
        state.onClick = (event) => handleClick(instance, state, event);
        state.onPointerDown = (event) => handlePointerDown(instance, event);
        state.onKeyDown = (event) => handleKeyDown(instance, state, event as KeyboardEvent);
        state.onFocusIn = (event) => {
            if (event.target === inputFor(instance)) open(instance, state, false);
        };
        state.onFocusOut = () => {
            queueMicrotask(() => {
                if (!instance.contains(instance.ownerDocument.activeElement)) close(instance, state, true);
            });
        };
        state.onDocumentPointerDown = (event) => {
            if (state.expanded && !instance.contains(event.target as Node | null)) close(instance, state, true);
        };

        instance.addEventListener('input', state.onInput, true);
        instance.addEventListener('change', state.onChange, true);
        instance.addEventListener('click', state.onClick);
        instance.addEventListener('pointerdown', state.onPointerDown);
        instance.addEventListener('keydown', state.onKeyDown);
        instance.addEventListener('focusin', state.onFocusIn);
        instance.addEventListener('focusout', state.onFocusOut);
        instance.ownerDocument.addEventListener('pointerdown', state.onDocumentPointerDown, true);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        synchronizeModel(instance, state);
        context.setSlices(renderSlices(instance, state), { render: false });
        synchronizeForm(instance, state);
    },
    rendered(instance) {
        const state = stateFor(instance);
        const input = inputFor(instance);
        if (input && input.value !== state.displayValue) input.value = state.displayValue;
        if (state.expanded && state.activeIndex >= 0) {
            instance
                .querySelector<HTMLElement>(`#${cssEscape(activeOptionId(state))}`)
                ?.scrollIntoView({ block: 'nearest' });
        }
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onInput) instance.removeEventListener('input', state.onInput, true);
        if (state.onChange) instance.removeEventListener('change', state.onChange, true);
        if (state.onClick) instance.removeEventListener('click', state.onClick);
        if (state.onPointerDown) instance.removeEventListener('pointerdown', state.onPointerDown);
        if (state.onKeyDown) instance.removeEventListener('keydown', state.onKeyDown);
        if (state.onFocusIn) instance.removeEventListener('focusin', state.onFocusIn);
        if (state.onFocusOut) instance.removeEventListener('focusout', state.onFocusOut);
        if (state.onDocumentPointerDown) {
            instance.ownerDocument.removeEventListener('pointerdown', state.onDocumentPointerDown, true);
        }
    },
    formDisabled(instance, disabled) {
        const state = stateFor(instance);
        state.formDisabled = disabled;
        if (disabled) state.expanded = false;
        state.context?.requestRender();
    },
    formReset(instance) {
        const state = stateFor(instance);
        state.value = state.defaultValue;
        state.displayValue = state.defaultDisplayValue;
        state.committedOptionValue = state.defaultCommittedOptionValue;
        state.activeIndex = selectedIndex(state);
        state.edited = false;
        state.expanded = false;
        update(instance, state, false);
    },
    formStateRestore(instance, restored) {
        const state = stateFor(instance);
        const value = restored === null || restored instanceof File || restored instanceof FormData ? '' : restored;
        applyProgrammaticValue(instance, state, String(value));
    },
};

function stateFor(instance: HTMLElement): AutocompleteState {
    let state = AUTOCOMPLETE_STATES.get(instance);
    if (state) return state;
    autocompleteSequence += 1;
    state = {
        activeIndex: -1,
        authoredValue: null,
        committedOptionValue: null,
        connected: false,
        defaultCommittedOptionValue: null,
        defaultDisplayValue: '',
        defaultValue: '',
        displayValue: '',
        edited: false,
        expanded: false,
        formDisabled: false,
        groups: [],
        initialized: false,
        labelId: `cem-autocomplete-${autocompleteSequence}-label`,
        listboxId: `cem-autocomplete-${autocompleteSequence}-listbox`,
        options: [],
        payloadSignature: '',
        value: '',
        warnedPayloadSignature: '',
    };
    AUTOCOMPLETE_STATES.set(instance, state);
    return state;
}

function synchronizeModel(instance: HTMLElement, state: AutocompleteState): void {
    const snapshot = state.context?.snapshot();
    if (!snapshot) return;
    const signature = JSON.stringify(snapshot.payload.nodes);
    const payloadChanged = signature !== state.payloadSignature;
    if (payloadChanged) {
        const normalized = normalizeChoiceOptions(snapshot.payload.nodes);
        state.groups = normalized.groups;
        state.options = normalized.options;
        state.payloadSignature = signature;
        if (normalized.issue && state.warnedPayloadSignature !== signature) {
            state.warnedPayloadSignature = signature;
            instance.ownerDocument.defaultView?.console.warn(`[cem-autocomplete] ${normalized.issue}`);
        }
        if (state.activeIndex >= state.options.length || !enabledOptionAt(state, state.activeIndex)) {
            state.activeIndex = initialActiveIndex(instance, state);
        }
        if (!hasEnabledOptions(state)) state.expanded = false;
    }

    const authoredValue = instance.getAttribute('value');
    const authoredValueChanged = authoredValue !== state.authoredValue;
    state.authoredValue = authoredValue;
    if (!state.initialized) {
        const selected = state.options.find((option) => option.defaultSelected && !option.disabled);
        const initialValue = state.pendingValue ?? authoredValue ?? selected?.value ?? '';
        state.pendingValue = undefined;
        resolveValue(instance, state, initialValue);
        state.defaultValue = state.value;
        state.defaultDisplayValue = state.displayValue;
        state.defaultCommittedOptionValue = state.committedOptionValue;
        state.initialized = true;
    } else if (authoredValueChanged) {
        resolveValue(instance, state, authoredValue ?? '');
        state.edited = false;
        state.expanded = false;
    }

    if (isDisabled(instance, state) || instance.hasAttribute('readonly')) state.expanded = false;
}

function resolveValue(instance: HTMLElement, state: AutocompleteState, requested: string): void {
    const option = state.options.find((candidate) => candidate.value === requested && !candidate.disabled);
    if (option) {
        state.value = option.value;
        state.displayValue = option.label;
        state.committedOptionValue = option.value;
    } else if (instance.hasAttribute('require-selection')) {
        state.value = '';
        state.displayValue = '';
        state.committedOptionValue = null;
    } else {
        state.value = requested;
        state.displayValue = requested;
        state.committedOptionValue = null;
    }
    state.activeIndex = selectedIndex(state);
}

function renderSlices(instance: HTMLElement, state: AutocompleteState): Record<string, unknown> {
    let index = 0;
    const groups: RenderGroup[] = state.groups.map((group) => ({
        disabled: group.disabled,
        label: group.label,
        options: group.options.map((option) => {
            const rendered: RenderOption = {
                ...option,
                active: index === state.activeIndex,
                hasChildren: option.children.length > 0,
                id: `${state.listboxId}-option-${index}`,
                index,
                selected: state.committedOptionValue === option.value,
            };
            index += 1;
            return rendered;
        }),
    }));
    return {
        activeOptionId: activeOptionId(state),
        behaviorDisabled: state.formDisabled,
        displayValue: state.displayValue,
        expanded: state.expanded,
        groups,
        labelId: state.labelId,
        listboxId: state.listboxId,
    };
}

function handleInput(instance: HTMLElement, state: AutocompleteState, event: Event): void {
    const input = inputFor(instance);
    if (!input || event.target !== input || input.readOnly || input.disabled) return;
    state.displayValue = input.value;
    state.value = instance.hasAttribute('require-selection') ? '' : input.value;
    state.committedOptionValue = null;
    state.activeIndex = instance.hasAttribute('auto-active-first') ? firstEnabledIndex(state) : -1;
    state.edited = true;
    state.expanded = hasEnabledOptions(state);
    update(instance, state, false);
}

function handleChange(instance: HTMLElement, state: AutocompleteState, event: Event): void {
    if (event.target !== inputFor(instance) || !instance.hasAttribute('require-selection')) return;
    event.stopImmediatePropagation();
    if (state.edited) clearInvalidEdit(instance, state, true);
}

function handlePointerDown(instance: HTMLElement, event: Event): void {
    const target = event.target instanceof Element ? event.target : null;
    if (target?.closest('[data-option-index]') && instance.contains(target)) event.preventDefault();
}

function handleClick(instance: HTMLElement, state: AutocompleteState, event: Event): void {
    if (!canInteract(instance, state)) return;
    const target = event.target instanceof Element ? event.target : null;
    const optionElement = target?.closest<HTMLElement>('[data-option-index]');
    if (!optionElement || !instance.contains(optionElement)) return;
    const index = Number.parseInt(optionElement.dataset.optionIndex ?? '', 10);
    commitIndex(instance, state, index);
}

function handleKeyDown(instance: HTMLElement, state: AutocompleteState, event: KeyboardEvent): void {
    if (event.target !== inputFor(instance) || !canInteract(instance, state)) return;
    if (event.altKey && event.key === 'ArrowDown') {
        if (hasEnabledOptions(state)) {
            event.preventDefault();
            open(instance, state, false);
        }
        return;
    }
    if (event.altKey && event.key === 'ArrowUp') {
        if (state.expanded) {
            event.preventDefault();
            close(instance, state, true);
        }
        return;
    }
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        if (!hasEnabledOptions(state)) return;
        event.preventDefault();
        const direction = event.key === 'ArrowDown' ? 1 : -1;
        const wasExpanded = state.expanded;
        if (!wasExpanded) open(instance, state, false);
        moveActive(state, direction);
        update(instance, state, false);
        return;
    }
    if (event.key === 'Enter') {
        if (state.expanded && enabledOptionAt(state, state.activeIndex)) {
            event.preventDefault();
            commitIndex(instance, state, state.activeIndex);
        }
        return;
    }
    if (event.key === 'Escape') {
        if (state.expanded) {
            event.preventDefault();
            close(instance, state, true);
        }
        return;
    }
    if (event.key === 'Tab' && state.expanded) close(instance, state, true);
}

function open(instance: HTMLElement, state: AutocompleteState, render: boolean): void {
    if (!canInteract(instance, state) || !hasEnabledOptions(state)) return;
    if (!state.expanded) {
        state.expanded = true;
        state.activeIndex = initialActiveIndex(instance, state);
    }
    if (render) update(instance, state, false);
    else state.context?.setSlices(renderSlices(instance, state));
}

function close(instance: HTMLElement, state: AutocompleteState, emitInvalidClear: boolean): void {
    if (!state.expanded && !(instance.hasAttribute('require-selection') && state.edited)) return;
    state.expanded = false;
    state.activeIndex = -1;
    if (instance.hasAttribute('require-selection') && state.edited) {
        clearInvalidEdit(instance, state, emitInvalidClear);
        return;
    }
    update(instance, state, false);
}

function clearInvalidEdit(instance: HTMLElement, state: AutocompleteState, emit: boolean): void {
    const changed = state.value !== '' || state.displayValue !== '' || state.committedOptionValue !== null;
    state.value = '';
    state.displayValue = '';
    state.committedOptionValue = null;
    state.activeIndex = -1;
    state.expanded = false;
    state.edited = false;
    update(instance, state, emit && changed);
}

function commitIndex(instance: HTMLElement, state: AutocompleteState, index: number): void {
    const option = enabledOptionAt(state, index);
    if (!option) return;
    state.value = option.value;
    state.displayValue = option.label;
    state.committedOptionValue = option.value;
    state.activeIndex = index;
    state.edited = false;
    state.expanded = false;
    update(instance, state, true);
}

function moveActive(state: AutocompleteState, direction: 1 | -1): void {
    if (state.options.length === 0) return;
    let index = state.activeIndex < 0
        ? direction > 0 ? 0 : state.options.length - 1
        : state.activeIndex + direction;
    while (index >= 0 && index < state.options.length && !enabledOptionAt(state, index)) index += direction;
    if (enabledOptionAt(state, index)) state.activeIndex = index;
}

function initialActiveIndex(instance: HTMLElement, state: AutocompleteState): number {
    const committed = selectedIndex(state);
    if (committed >= 0 && enabledOptionAt(state, committed)) return committed;
    return instance.hasAttribute('auto-active-first') ? firstEnabledIndex(state) : -1;
}

function firstEnabledIndex(state: AutocompleteState): number {
    return state.options.findIndex((option) => !option.disabled);
}

function selectedIndex(state: AutocompleteState): number {
    return state.committedOptionValue === null
        ? -1
        : state.options.findIndex((option) => option.value === state.committedOptionValue);
}

function enabledOptionAt(state: AutocompleteState, index: number): NormalizedChoiceOption | null {
    const option = index >= 0 && index < state.options.length ? state.options[index] : undefined;
    return option && !option.disabled ? option : null;
}

function hasEnabledOptions(state: AutocompleteState): boolean {
    return state.options.some((option) => !option.disabled);
}

function activeOptionId(state: AutocompleteState): string {
    return state.expanded && enabledOptionAt(state, state.activeIndex)
        ? `${state.listboxId}-option-${state.activeIndex}`
        : '';
}

function update(instance: HTMLElement, state: AutocompleteState, emit: boolean): void {
    state.context?.setSlices(renderSlices(instance, state));
    synchronizeForm(instance, state);
    if (emit) {
        instance.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
        instance.dispatchEvent(new Event('change', { bubbles: true }));
    }
}

function synchronizeForm(instance: HTMLElement, state: AutocompleteState): void {
    const internals = state.context?.internals;
    if (!internals) return;
    const disabled = isDisabled(instance, state);
    const name = instance.getAttribute('name') ?? '';
    if (disabled || !name) internals.setFormValue(null);
    else internals.setFormValue(state.value, state.value);

    const missing = instance.hasAttribute('required') && state.value === '';
    if (disabled || !missing) {
        internals.setValidity({});
        return;
    }
    const anchor = inputFor(instance);
    internals.setValidity(
        { valueMissing: true },
        requiredValidationMessage(instance.ownerDocument),
        anchor ?? undefined,
    );
}

function requiredValidationMessage(document: Document): string {
    const input = document.createElement('input');
    input.required = true;
    return input.validationMessage || 'Please fill out this field.';
}

function canInteract(instance: HTMLElement, state: AutocompleteState): boolean {
    return !isDisabled(instance, state) && !instance.hasAttribute('readonly');
}

function isDisabled(instance: HTMLElement, state: AutocompleteState): boolean {
    return instance.hasAttribute('disabled') || state.formDisabled;
}

function inputFor(instance: HTMLElement): HTMLInputElement | null {
    return instance.querySelector<HTMLInputElement>('.cem-autocomplete__control');
}

function installHostApi(instance: HTMLElement, state: AutocompleteState): void {
    const reflectBoolean = (name: string) => ({
        configurable: true,
        enumerable: true,
        get: () => instance.hasAttribute(name),
        set: (value: boolean) => instance.toggleAttribute(name, Boolean(value)),
    });
    const reflectString = (name: string) => ({
        configurable: true,
        enumerable: true,
        get: () => instance.getAttribute(name) ?? '',
        set: (value: unknown) => instance.setAttribute(name, String(value)),
    });

    Object.defineProperties(instance, {
        value: {
            configurable: true,
            enumerable: true,
            get: () => state.value,
            set: (value: unknown) => applyProgrammaticValue(instance, state, String(value)),
        },
        displayValue: {
            configurable: true,
            enumerable: true,
            get: () => state.displayValue,
            set: (value: unknown) => {
                const displayValue = String(value);
                state.displayValue = displayValue;
                state.value = instance.hasAttribute('require-selection') ? '' : displayValue;
                state.committedOptionValue = null;
                state.activeIndex = -1;
                state.edited = false;
                update(instance, state, false);
            },
        },
        selectedIndex: {
            configurable: true,
            enumerable: true,
            get: () => selectedIndex(state),
            set: (value: unknown) => {
                const index = Number(value);
                const option = Number.isInteger(index) ? enabledOptionAt(state, index) : null;
                applyProgrammaticValue(instance, state, option?.value ?? '');
            },
        },
        expanded: {
            configurable: true,
            enumerable: true,
            get: () => state.expanded,
            set: (value: unknown) => {
                if (value) open(instance, state, true);
                else close(instance, state, false);
            },
        },
        form: { configurable: true, enumerable: true, get: () => state.context?.internals?.form ?? null },
        validity: { configurable: true, enumerable: true, get: () => state.context?.internals?.validity },
        validationMessage: {
            configurable: true,
            enumerable: true,
            get: () => state.context?.internals?.validationMessage ?? '',
        },
        willValidate: {
            configurable: true,
            enumerable: true,
            get: () => state.context?.internals?.willValidate ?? false,
        },
        labels: { configurable: true, enumerable: true, get: () => labelsFor(instance) },
        disabled: reflectBoolean('disabled'),
        required: reflectBoolean('required'),
        readonly: reflectBoolean('readonly'),
        busy: reflectBoolean('busy'),
        requireSelection: reflectBoolean('require-selection'),
        autoActiveFirst: reflectBoolean('auto-active-first'),
        name: reflectString('name'),
        placeholder: reflectString('placeholder'),
        autocomplete: reflectString('autocomplete'),
        indicator: reflectString('indicator'),
        checkValidity: {
            configurable: true,
            value: () => state.context?.internals?.checkValidity() ?? true,
        },
        reportValidity: {
            configurable: true,
            value: () => state.context?.internals?.reportValidity() ?? true,
        },
    });
}

function applyProgrammaticValue(instance: HTMLElement, state: AutocompleteState, value: string): void {
    if (!state.initialized) {
        state.pendingValue = value;
        return;
    }
    resolveValue(instance, state, value);
    state.edited = false;
    state.expanded = false;
    update(instance, state, false);
}

function labelsFor(instance: HTMLElement): HTMLLabelElement[] {
    const labels: HTMLLabelElement[] = [];
    const parent = instance.closest('label');
    if (parent instanceof HTMLLabelElement) labels.push(parent);
    if (instance.id) {
        for (const label of instance.ownerDocument.querySelectorAll<HTMLLabelElement>('label[for]')) {
            if (label.htmlFor === instance.id && !labels.includes(label)) labels.push(label);
        }
    }
    return labels;
}

function cssEscape(value: string): string {
    return globalThis.CSS?.escape ? globalThis.CSS.escape(value) : value.replace(/[^a-zA-Z0-9_-]/g, '\\$&');
}
