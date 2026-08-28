import type {
    CemProducedElementBehavior,
    CemProducedElementBehaviorContext,
} from './cem-elements.js';
import {
    normalizeChoiceOptions,
    type NormalizedChoiceGroup as NormalizedGroup,
    type NormalizedChoiceOption as NormalizedOption,
} from './choice-options.js';

type SelectMode = 'dropdown' | 'single-listbox' | 'multiple-listbox';

interface RenderOption extends NormalizedOption {
    index: number;
    id: string;
    selected: boolean;
    active: boolean;
}

interface RenderGroup {
    label: string;
    disabled: boolean;
    options: RenderOption[];
}

interface SelectState {
    context?: CemProducedElementBehaviorContext;
    groups: NormalizedGroup[];
    options: NormalizedOption[];
    selection: Set<string>;
    defaultValues: string[];
    activeIndex: number;
    rangeAnchor: number;
    expanded: boolean;
    previewSelection: string[];
    mode: SelectMode;
    payloadSignature: string;
    authoredValue: string | null;
    initialized: boolean;
    formDisabled: boolean;
    pendingValues?: string[];
    typeahead: string;
    typeaheadAt: number;
    refocus: boolean;
    popupId: string;
    listboxId: string;
    labelId: string;
    connected: boolean;
    warnedPayloadSignature: string;
    onClick?: EventListener;
    onKeyDown?: EventListener;
    onFocusOut?: EventListener;
    onDocumentPointerDown?: EventListener;
}

const SELECT_STATES = new WeakMap<HTMLElement, SelectState>();
let selectSequence = 0;

export const CEM_CHOICE_SELECT_CAPABILITY: CemProducedElementBehavior = {
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

        state.onClick = (event) => handleClick(instance, state, event);
        state.onKeyDown = (event) => handleKeyDown(instance, state, event as KeyboardEvent);
        state.onFocusOut = () => {
            queueMicrotask(() => {
                if (state.expanded && !instance.contains(instance.ownerDocument.activeElement)) {
                    commitPreview(instance, state, true);
                }
            });
        };
        state.onDocumentPointerDown = (event) => {
            if (state.expanded && !instance.contains(event.target as Node | null)) {
                commitPreview(instance, state, true);
            }
        };
        instance.addEventListener('click', state.onClick);
        instance.addEventListener('keydown', state.onKeyDown);
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
        const listbox = instance.querySelector<HTMLElement>('.cem-select__listbox');
        if (listbox) {
            listbox.style.blockSize = `calc(var(--cem-list-row-height) * ${reflectedSize(instance)})`;
        }
        if (state.refocus) {
            state.refocus = false;
            controlFor(instance)?.focus({ preventScroll: true });
        }
        if (state.expanded || state.mode !== 'dropdown') {
            instance.querySelector<HTMLElement>(`#${cssEscape(activeOptionId(state))}`)?.scrollIntoView({ block: 'nearest' });
        }
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onClick) instance.removeEventListener('click', state.onClick);
        if (state.onKeyDown) instance.removeEventListener('keydown', state.onKeyDown);
        if (state.onFocusOut) instance.removeEventListener('focusout', state.onFocusOut);
        if (state.onDocumentPointerDown) {
            instance.ownerDocument.removeEventListener('pointerdown', state.onDocumentPointerDown, true);
        }
    },
    formDisabled(instance, disabled) {
        const state = stateFor(instance);
        state.formDisabled = disabled;
        state.context?.requestRender();
    },
    formReset(instance) {
        const state = stateFor(instance);
        state.selection = normalizedSelection(state, state.defaultValues, true);
        state.activeIndex = firstSelectedOrEnabledIndex(state);
        state.expanded = false;
        update(instance, state, false);
    },
    formStateRestore(instance, restored) {
        const state = stateFor(instance);
        const values =
            restored instanceof FormData
                ? restored.getAll(instance.getAttribute('name') ?? '').map(String)
                : restored === null || restored instanceof File
                  ? []
                  : [restored];
        state.selection = normalizedSelection(state, values, false);
        state.activeIndex = firstSelectedOrEnabledIndex(state);
        state.expanded = false;
        update(instance, state, false);
    },
};

function stateFor(instance: HTMLElement): SelectState {
    let state = SELECT_STATES.get(instance);
    if (state) return state;
    selectSequence += 1;
    state = {
        groups: [],
        options: [],
        selection: new Set(),
        defaultValues: [],
        activeIndex: -1,
        rangeAnchor: -1,
        expanded: false,
        previewSelection: [],
        mode: 'dropdown',
        payloadSignature: '',
        authoredValue: null,
        initialized: false,
        formDisabled: false,
        typeahead: '',
        typeaheadAt: 0,
        refocus: false,
        popupId: `cem-select-${selectSequence}-popup`,
        listboxId: `cem-select-${selectSequence}-listbox`,
        labelId: `cem-select-${selectSequence}-label`,
        connected: false,
        warnedPayloadSignature: '',
    };
    SELECT_STATES.set(instance, state);
    return state;
}

function synchronizeModel(instance: HTMLElement, state: SelectState): void {
    const snapshot = state.context?.snapshot();
    if (!snapshot) return;
    const mode = selectMode(instance);
    const authoredValue = instance.getAttribute('value');
    const signature = JSON.stringify(snapshot.payload.nodes);
    const payloadChanged = signature !== state.payloadSignature;
    const modeChanged = mode !== state.mode;

    if (payloadChanged) {
        const normalized = normalizeChoiceOptions(snapshot.payload.nodes);
        state.groups = normalized.groups;
        state.options = normalized.options;
        state.defaultValues = state.options.filter((option) => option.defaultSelected).map((option) => option.value);
        state.payloadSignature = signature;
        if (normalized.issue && state.warnedPayloadSignature !== signature) {
            state.warnedPayloadSignature = signature;
            instance.ownerDocument.defaultView?.console.warn(`[cem-select] ${normalized.issue}`);
        }
    }

    state.mode = mode;
    const authoredValueChanged = authoredValue !== state.authoredValue;
    state.authoredValue = authoredValue;
    if (!state.initialized || payloadChanged || modeChanged || authoredValueChanged) {
        const requested = state.pendingValues
            ?? (authoredValue !== null ? [authoredValue] : state.initialized ? [...state.selection] : state.defaultValues);
        state.pendingValues = undefined;
        state.selection = normalizedSelection(state, requested, !state.initialized);
        state.activeIndex = firstSelectedOrEnabledIndex(state);
        state.rangeAnchor = state.activeIndex;
        state.initialized = true;
    }

    if (isDisabled(instance, state)) state.expanded = false;
}

function selectMode(instance: HTMLElement): SelectMode {
    if (instance.hasAttribute('multiple')) return 'multiple-listbox';
    return reflectedSize(instance) > 1 ? 'single-listbox' : 'dropdown';
}

function reflectedSize(instance: HTMLElement): number {
    const parsed = Number.parseInt(instance.getAttribute('size') ?? '', 10);
    return Number.isFinite(parsed) && parsed > 0 ? parsed : instance.hasAttribute('multiple') ? 4 : 1;
}

function normalizedSelection(state: SelectState, values: readonly string[], applyFallback: boolean): Set<string> {
    const enabled = new Set(state.options.filter((option) => !option.disabled).map((option) => option.value));
    const selected = new Set(values.filter((value) => enabled.has(value)));
    if (state.mode !== 'multiple-listbox' && selected.size > 1) {
        return new Set([selected.values().next().value as string]);
    }
    if (selected.size === 0 && applyFallback && state.mode !== 'multiple-listbox') {
        const first = state.options.find((option) => !option.disabled);
        if (first) selected.add(first.value);
    }
    return selected;
}

function renderSlices(instance: HTMLElement, state: SelectState): Record<string, unknown> {
    let index = 0;
    const groups: RenderGroup[] = state.groups.map((group) => ({
        label: group.label,
        disabled: group.disabled,
        options: group.options.map((option) => {
            const rendered: RenderOption = {
                ...option,
                index,
                id: `${state.listboxId}-option-${index}`,
                selected: state.selection.has(option.value),
                active: index === state.activeIndex,
            };
            index += 1;
            return rendered;
        }),
    }));
    const selected = state.options.find((option) => state.selection.has(option.value));
    return {
        groups,
        value: selected?.value ?? '',
        selectedValues: state.options.filter((option) => state.selection.has(option.value)).map((option) => option.value),
        displayLabel: selected?.label || instance.getAttribute('placeholder') || 'Choose',
        selectedChildren: selected?.children ?? [],
        selectedRich: selected?.rich ?? false,
        activeOptionId: activeOptionId(state),
        expanded: state.expanded,
        mode: state.mode,
        popupId: state.popupId,
        listboxId: state.listboxId,
        labelId: state.labelId,
        visibleRows: reflectedSize(instance),
        behaviorDisabled: state.formDisabled,
    };
}

function activeOptionId(state: SelectState): string {
    return state.activeIndex >= 0 ? `${state.listboxId}-option-${state.activeIndex}` : '';
}

function handleClick(instance: HTMLElement, state: SelectState, event: Event): void {
    if (isDisabled(instance, state)) return;
    const target = event.target instanceof Element ? event.target : null;
    const option = target?.closest<HTMLElement>('[data-option-index]');
    if (option && instance.contains(option)) {
        const index = Number.parseInt(option.dataset.optionIndex ?? '', 10);
        const selectedOption = enabledOptionAt(state, index);
        if (!selectedOption) return;
        const previousSelection = selectionSignature(state);
        state.activeIndex = index;
        if (state.mode === 'multiple-listbox') {
            toggleIndex(state, index, event instanceof MouseEvent && event.shiftKey);
            update(instance, state, selectionSignature(state) !== previousSelection);
        } else {
            state.selection = new Set([selectedOption.value]);
            state.expanded = false;
            update(instance, state, selectionSignature(state) !== previousSelection);
        }
        return;
    }
    if (target?.closest('.cem-select__control') && state.mode === 'dropdown') {
        if (state.expanded) commitPreview(instance, state, true);
        else openDropdown(instance, state);
    }
}

function handleKeyDown(instance: HTMLElement, state: SelectState, event: KeyboardEvent): void {
    if (isDisabled(instance, state) || !controlFor(instance)?.contains(event.target as Node)) return;
    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey && event.key !== ' ') {
        event.preventDefault();
        typeahead(instance, state, event.key);
        return;
    }

    if (state.mode === 'dropdown') {
        handleDropdownKey(instance, state, event);
        return;
    }
    handleListboxKey(instance, state, event);
}

function handleDropdownKey(instance: HTMLElement, state: SelectState, event: KeyboardEvent): void {
    if (event.key === 'Escape' && state.expanded) {
        event.preventDefault();
        state.selection = normalizedSelection(state, state.previewSelection, false);
        state.expanded = false;
        update(instance, state, false);
        return;
    }
    if (event.key === 'Tab' && state.expanded) {
        commitPreview(instance, state, true);
        return;
    }
    if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        if (state.expanded) commitPreview(instance, state, true);
        else openDropdown(instance, state);
        return;
    }
    const movement = movementForKey(event.key);
    if (movement === null) return;
    event.preventDefault();
    if (!state.expanded) openDropdown(instance, state);
    moveActive(state, movement);
    update(instance, state, false);
}

function handleListboxKey(instance: HTMLElement, state: SelectState, event: KeyboardEvent): void {
    if (state.mode === 'multiple-listbox' && (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
        event.preventDefault();
        const previousSelection = selectionSignature(state);
        state.selection = new Set(state.options.filter((option) => !option.disabled).map((option) => option.value));
        update(instance, state, selectionSignature(state) !== previousSelection);
        return;
    }
    if (state.mode === 'multiple-listbox' && event.key === ' ') {
        event.preventDefault();
        const previousSelection = selectionSignature(state);
        toggleIndex(state, state.activeIndex, event.shiftKey);
        update(instance, state, selectionSignature(state) !== previousSelection);
        return;
    }
    const movement = movementForKey(event.key);
    if (movement === null) return;
    event.preventDefault();
    const previousSelection = selectionSignature(state);
    moveActive(state, movement);
    if (state.mode === 'multiple-listbox') {
        if (event.shiftKey) selectRange(state, state.activeIndex);
        update(instance, state, event.shiftKey && selectionSignature(state) !== previousSelection);
    } else {
        const activeOption = enabledOptionAt(state, state.activeIndex);
        if (!activeOption) return;
        state.selection = new Set([activeOption.value]);
        state.rangeAnchor = state.activeIndex;
        update(instance, state, selectionSignature(state) !== previousSelection);
    }
}

function movementForKey(key: string): number | 'first' | 'last' | null {
    if (key === 'ArrowDown') return 1;
    if (key === 'ArrowUp') return -1;
    if (key === 'PageDown') return 10;
    if (key === 'PageUp') return -10;
    if (key === 'Home') return 'first';
    if (key === 'End') return 'last';
    return null;
}

function moveActive(state: SelectState, movement: number | 'first' | 'last'): void {
    if (state.options.length === 0) return;
    const direction = movement === 'first' ? 1 : movement === 'last' ? -1 : Math.sign(movement);
    let index =
        movement === 'first'
            ? 0
            : movement === 'last'
              ? state.options.length - 1
              : Math.max(0, Math.min(state.options.length - 1, state.activeIndex + movement));
    while (index >= 0 && index < state.options.length && !isEnabledIndex(state, index)) index += direction;
    if (isEnabledIndex(state, index)) state.activeIndex = index;
}

function openDropdown(instance: HTMLElement, state: SelectState): void {
    state.previewSelection = [...state.selection];
    state.activeIndex = firstSelectedOrEnabledIndex(state);
    state.expanded = true;
    update(instance, state, false);
}

function commitPreview(instance: HTMLElement, state: SelectState, emit: boolean): void {
    const previousSelection = selectionSignature(state);
    const activeOption = enabledOptionAt(state, state.activeIndex);
    if (activeOption) state.selection = new Set([activeOption.value]);
    state.expanded = false;
    update(instance, state, emit && selectionSignature(state) !== previousSelection);
}

function toggleIndex(state: SelectState, index: number, range: boolean): void {
    const option = enabledOptionAt(state, index);
    if (!option) return;
    if (range) {
        selectRange(state, index);
        return;
    }
    const value = option.value;
    if (state.selection.has(value)) state.selection.delete(value);
    else state.selection.add(value);
    state.rangeAnchor = index;
}

function selectRange(state: SelectState, index: number): void {
    const anchor = isEnabledIndex(state, state.rangeAnchor) ? state.rangeAnchor : index;
    const [start, end] = anchor <= index ? [anchor, index] : [index, anchor];
    for (let current = start; current <= end; current += 1) {
        const option = enabledOptionAt(state, current);
        if (option) state.selection.add(option.value);
    }
}

function typeahead(instance: HTMLElement, state: SelectState, key: string): void {
    const now = Date.now();
    state.typeahead = now - state.typeaheadAt > 700 ? key : `${state.typeahead}${key}`;
    state.typeaheadAt = now;
    const query = state.typeahead.toLocaleLowerCase();
    const start = Math.max(0, state.activeIndex + 1);
    const ordered = [...state.options.slice(start), ...state.options.slice(0, start)];
    const match = ordered.find((option) => !option.disabled && option.label.toLocaleLowerCase().startsWith(query));
    if (!match) return;
    const previousSelection = selectionSignature(state);
    if (state.mode === 'dropdown') {
        if (!state.expanded) {
            state.previewSelection = [...state.selection];
            state.expanded = true;
        }
    } else if (state.mode === 'single-listbox') {
        state.selection = new Set([match.value]);
    }
    state.activeIndex = state.options.indexOf(match);
    update(
        instance,
        state,
        state.mode === 'single-listbox' && selectionSignature(state) !== previousSelection
    );
}

function firstSelectedOrEnabledIndex(state: SelectState): number {
    const selected = state.options.findIndex((option) => state.selection.has(option.value) && !option.disabled);
    return selected >= 0 ? selected : state.options.findIndex((option) => !option.disabled);
}

function isEnabledIndex(state: SelectState, index: number): boolean {
    return enabledOptionAt(state, index) !== null;
}

function selectionSignature(state: SelectState): string {
    return JSON.stringify(
        state.options.filter((option) => state.selection.has(option.value)).map((option) => option.value)
    );
}

function enabledOptionAt(state: SelectState, index: number): NormalizedOption | null {
    const option = index >= 0 && index < state.options.length ? state.options[index] : undefined;
    return option && !option.disabled ? option : null;
}

function update(instance: HTMLElement, state: SelectState, emit: boolean): void {
    state.refocus = instance.contains(instance.ownerDocument.activeElement);
    state.context?.setSlices(renderSlices(instance, state));
    synchronizeForm(instance, state);
    if (emit) {
        instance.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
        instance.dispatchEvent(new Event('change', { bubbles: true }));
    }
}

function synchronizeForm(instance: HTMLElement, state: SelectState): void {
    const internals = state.context?.internals;
    if (!internals) return;
    const disabled = isDisabled(instance, state);
    const name = instance.getAttribute('name') ?? '';
    const values = state.options.filter((option) => state.selection.has(option.value)).map((option) => option.value);
    if (disabled || !name) {
        internals.setFormValue(null);
    } else if (state.mode === 'multiple-listbox') {
        const data = new FormData();
        for (const value of values) data.append(name, value);
        internals.setFormValue(data, data);
    } else {
        const value = values[0] ?? '';
        internals.setFormValue(value, value);
    }

    const missing = instance.hasAttribute('required')
        && (values.length === 0 || (state.mode !== 'multiple-listbox' && values[0] === ''));
    if (disabled || !missing) {
        internals.setValidity({});
        return;
    }
    const anchor = controlFor(instance);
    internals.setValidity(
        { valueMissing: true },
        requiredValidationMessage(instance.ownerDocument, state.mode === 'multiple-listbox'),
        anchor ?? undefined
    );
}

function requiredValidationMessage(document: Document, multiple: boolean): string {
    const select = document.createElement('select');
    select.required = true;
    select.multiple = multiple;
    const option = document.createElement('option');
    option.value = '';
    option.selected = !multiple;
    select.append(option);
    return select.validationMessage || 'Please select an item in the list.';
}

function isDisabled(instance: HTMLElement, state: SelectState): boolean {
    return instance.hasAttribute('disabled') || state.formDisabled;
}

function controlFor(instance: HTMLElement): HTMLElement | null {
    return instance.querySelector<HTMLElement>('.cem-select__control');
}

function installHostApi(instance: HTMLElement, state: SelectState): void {
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
            get: () => state.options.find((option) => state.selection.has(option.value))?.value ?? '',
            set: (value: unknown) => setSelectedValues(instance, state, [String(value)], false),
        },
        selectedValues: {
            configurable: true,
            enumerable: true,
            get: () => state.options.filter((option) => state.selection.has(option.value)).map((option) => option.value),
            set: (values: unknown) =>
                setSelectedValues(instance, state, Array.isArray(values) ? values.map(String) : [String(values)], false),
        },
        selectedIndex: {
            configurable: true,
            enumerable: true,
            get: () => state.options.findIndex((option) => state.selection.has(option.value)),
            set: (value: unknown) => {
                const index = Number(value);
                const option = Number.isInteger(index) && index >= 0 ? state.options[index] : undefined;
                setSelectedValues(instance, state, option ? [option.value] : [], false);
            },
        },
        type: {
            configurable: true,
            enumerable: true,
            get: () => (instance.hasAttribute('multiple') ? 'select-multiple' : 'select-one'),
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
        multiple: reflectBoolean('multiple'),
        required: reflectBoolean('required'),
        name: reflectString('name'),
        autocomplete: reflectString('autocomplete'),
        size: {
            configurable: true,
            enumerable: true,
            get: () => reflectedSize(instance),
            set: (value: unknown) => instance.setAttribute('size', String(Math.max(0, Number(value) || 0))),
        },
        setSelectedValues: {
            configurable: true,
            value: (values: readonly unknown[]) => setSelectedValues(instance, state, values.map(String), false),
        },
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

function setSelectedValues(instance: HTMLElement, state: SelectState, values: string[], emit: boolean): void {
    if (!state.initialized) {
        state.pendingValues = values;
        return;
    }
    state.selection = normalizedSelection(state, values, false);
    state.activeIndex = firstSelectedOrEnabledIndex(state);
    state.expanded = false;
    update(instance, state, emit);
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
