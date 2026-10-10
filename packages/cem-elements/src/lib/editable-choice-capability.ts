import type {
    CemProducedElementBehavior,
    CemProducedElementBehaviorContext,
} from './cem-elements.js';
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

interface EditableChoiceState {
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

const EDITABLE_CHOICE_STATES = new WeakMap<HTMLElement, EditableChoiceState>();
let editableChoiceSequence = 0;

export const CEM_EDITABLE_CHOICE_CAPABILITY: CemProducedElementBehavior = {
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
        instance.addEventListener('mousedown', state.onPointerDown);
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
        synchronizeForm(instance, state);
        const input = inputFor(instance);
        if (input && input.value !== state.displayValue) input.value = state.displayValue;
        if (state.expanded && state.activeIndex >= 0) {
            const option = instance.querySelector<HTMLElement>(`#${cssEscape(activeOptionId(state))}`);
            const viewport = option?.closest<HTMLElement>('[part~=popup][role=listbox]');
            if (option && viewport) {
                const bounds = viewport.getBoundingClientRect(), row = option.getBoundingClientRect();
                const top = bounds.top + viewport.clientTop, bottom = top + viewport.clientHeight;
                if (row.top < top) viewport.scrollTop += row.top - top;
                else if (row.bottom > bottom) viewport.scrollTop += row.bottom - bottom;
            }
        }
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onInput) instance.removeEventListener('input', state.onInput, true);
        if (state.onChange) instance.removeEventListener('change', state.onChange, true);
        if (state.onClick) instance.removeEventListener('click', state.onClick);
        if (state.onPointerDown) {
            instance.removeEventListener('pointerdown', state.onPointerDown);
            instance.removeEventListener('mousedown', state.onPointerDown);
        }
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

function stateFor(instance: HTMLElement): EditableChoiceState {
    let state = EDITABLE_CHOICE_STATES.get(instance);
    if (state) return state;
    editableChoiceSequence += 1;
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
        labelId: `cem-editable-choice-${editableChoiceSequence}-label`,
        listboxId: `cem-editable-choice-${editableChoiceSequence}-listbox`,
        options: [],
        payloadSignature: '',
        value: '',
        warnedPayloadSignature: '',
    };
    EDITABLE_CHOICE_STATES.set(instance, state);
    return state;
}

function synchronizeModel(instance: HTMLElement, state: EditableChoiceState): void {
    const snapshot = state.context?.snapshot();
    if (!snapshot) return;
    const signature = JSON.stringify(snapshot.payload.nodes);
    const payloadChanged = signature !== state.payloadSignature;
    if (payloadChanged) {
        const activeValue = enabledOptionAt(state, state.activeIndex)?.value;
        const normalized = normalizeChoiceOptions(snapshot.payload.nodes);
        state.groups = normalized.groups;
        state.options = normalized.options;
        state.payloadSignature = signature;
        if (normalized.issue && state.warnedPayloadSignature !== signature) {
            state.warnedPayloadSignature = signature;
            instance.ownerDocument.defaultView?.console.warn(`[editable-choice] ${normalized.issue}`);
        }
        state.activeIndex = activeValue === undefined ? -1 : state.options.findIndex(option => option.value === activeValue && !option.disabled);
        if (state.activeIndex < 0) {
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

function resolveValue(instance: HTMLElement, state: EditableChoiceState, requested: string): void {
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

function renderSlices(instance: HTMLElement, state: EditableChoiceState): Record<string, unknown> {
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

function handleInput(instance: HTMLElement, state: EditableChoiceState, event: Event): void {
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

function handleChange(instance: HTMLElement, state: EditableChoiceState, event: Event): void {
    if (event.target !== inputFor(instance) || !instance.hasAttribute('require-selection')) return;
    event.stopImmediatePropagation();
    if (state.edited) clearInvalidEdit(instance, state, true);
}

function handlePointerDown(instance: HTMLElement, event: Event): void {
    const target = event.target instanceof Element ? event.target : null;
    const option = target?.closest('[part~=option][data-option-index]');
    if (option && owns(instance, option)) event.preventDefault();
}

function handleClick(instance: HTMLElement, state: EditableChoiceState, event: Event): void {
    if (!canInteract(instance, state)) return;
    const target = event.target instanceof Element ? event.target : null;
    const optionElement = target?.closest<HTMLElement>('[part~=option][data-option-index]');
    if (!optionElement || !owns(instance, optionElement)) return;
    const index = Number.parseInt(optionElement.dataset.optionIndex ?? '', 10);
    commitIndex(instance, state, index);
}

function handleKeyDown(instance: HTMLElement, state: EditableChoiceState, event: KeyboardEvent): void {
    if (event.target !== inputFor(instance) || !canInteract(instance, state)) return;
    // Composition and modified editing shortcuts belong to the native editor.
    if (event.isComposing || event.keyCode === 229 || event.ctrlKey || event.metaKey || event.shiftKey) return;
    if (event.altKey && event.key === 'ArrowDown') {
        if (hasEnabledOptions(state)) {
            event.preventDefault();
            open(instance, state, false);
        }
        return;
    }
    if (event.altKey && event.key === 'ArrowUp') {
        if (state.expanded || (instance.hasAttribute('require-selection') && state.edited)) {
            event.preventDefault();
            close(instance, state, true);
        }
        return;
    }
    if (event.altKey) return;
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
        if (state.expanded || (instance.hasAttribute('require-selection') && state.edited)) {
            event.preventDefault();
            close(instance, state, true);
        }
        return;
    }
    if (event.key === 'Tab' && state.expanded) close(instance, state, true);
}

function open(instance: HTMLElement, state: EditableChoiceState, render: boolean): void {
    if (!canInteract(instance, state) || !hasEnabledOptions(state)) return;
    if (!state.expanded) {
        state.expanded = true;
        state.activeIndex = initialActiveIndex(instance, state);
    }
    if (render) update(instance, state, false);
    else state.context?.setSlices(renderSlices(instance, state));
}

function close(instance: HTMLElement, state: EditableChoiceState, emitInvalidClear: boolean): void {
    if (!state.expanded && !(instance.hasAttribute('require-selection') && state.edited)) return;
    state.expanded = false;
    state.activeIndex = -1;
    if (instance.hasAttribute('require-selection') && state.edited) {
        clearInvalidEdit(instance, state, emitInvalidClear);
        return;
    }
    update(instance, state, false);
}

function clearInvalidEdit(instance: HTMLElement, state: EditableChoiceState, emit: boolean): void {
    const changed = state.value !== '' || state.displayValue !== '' || state.committedOptionValue !== null;
    state.value = '';
    state.displayValue = '';
    state.committedOptionValue = null;
    state.activeIndex = -1;
    state.expanded = false;
    state.edited = false;
    update(instance, state, emit && changed);
}

function commitIndex(instance: HTMLElement, state: EditableChoiceState, index: number): void {
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

function moveActive(state: EditableChoiceState, direction: 1 | -1): void {
    if (state.options.length === 0) return;
    let index = state.activeIndex < 0
        ? direction > 0 ? 0 : state.options.length - 1
        : state.activeIndex + direction;
    while (index >= 0 && index < state.options.length && !enabledOptionAt(state, index)) index += direction;
    if (enabledOptionAt(state, index)) state.activeIndex = index;
}

function initialActiveIndex(instance: HTMLElement, state: EditableChoiceState): number {
    const committed = selectedIndex(state);
    if (committed >= 0 && enabledOptionAt(state, committed)) return committed;
    return instance.hasAttribute('auto-active-first') ? firstEnabledIndex(state) : -1;
}

function firstEnabledIndex(state: EditableChoiceState): number {
    return state.options.findIndex((option) => !option.disabled);
}

function selectedIndex(state: EditableChoiceState): number {
    return state.committedOptionValue === null
        ? -1
        : state.options.findIndex((option) => option.value === state.committedOptionValue);
}

function enabledOptionAt(state: EditableChoiceState, index: number): NormalizedChoiceOption | null {
    const option = index >= 0 && index < state.options.length ? state.options[index] : undefined;
    return option && !option.disabled ? option : null;
}

function hasEnabledOptions(state: EditableChoiceState): boolean {
    return state.options.some((option) => !option.disabled);
}

function activeOptionId(state: EditableChoiceState): string {
    return state.expanded && enabledOptionAt(state, state.activeIndex)
        ? `${state.listboxId}-option-${state.activeIndex}`
        : '';
}

function update(instance: HTMLElement, state: EditableChoiceState, emit: boolean): void {
    const input = inputFor(instance);
    if (input && input.value !== state.displayValue) input.value = state.displayValue;
    state.context?.setSlices(renderSlices(instance, state));
    synchronizeForm(instance, state);
    if (emit) {
        instance.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
        instance.dispatchEvent(new Event('change', { bubbles: true }));
    }
}

function synchronizeForm(instance: HTMLElement, state: EditableChoiceState): void {
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

function canInteract(instance: HTMLElement, state: EditableChoiceState): boolean {
    return !isDisabled(instance, state) && !instance.hasAttribute('readonly');
}

function isDisabled(instance: HTMLElement, state: EditableChoiceState): boolean {
    return instance.hasAttribute('disabled') || state.formDisabled;
}

function owns(instance: HTMLElement, element: Element): boolean {
    for (let parent = element.parentElement; parent && parent !== instance; parent = parent.parentElement) {
        if (parent.localName.includes('-')) return false;
    }
    return instance.contains(element);
}

function inputFor(instance: HTMLElement): HTMLInputElement | null {
    const controls = [...instance.querySelectorAll<HTMLInputElement>('input[part~=control][role=combobox]')].filter(control => owns(instance, control));
    return controls.length === 1 ? controls[0] : null;
}

function installHostApi(instance: HTMLElement, state: EditableChoiceState): void {
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

function applyProgrammaticValue(instance: HTMLElement, state: EditableChoiceState, value: string): void {
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
