import type {
    CemProducedElementBehavior,
    CemProducedElementBehaviorContext,
    SerializedPayloadNode,
} from '@epa-wg/cem-elements';

export interface CemTreeToggleDetail {
    value: string;
    expanded: boolean;
}

export interface CemTreeActivateDetail {
    value: string;
}

type SelectionMode = 'none' | 'single' | 'multiple';

interface NormalizedTreeItem {
    value: string;
    label: string;
    id: string;
    groupId: string;
    level: number;
    position: number;
    setSize: number;
    expandable: boolean;
    disabled: boolean;
    loading: boolean;
    parent: NormalizedTreeItem | null;
    children: NormalizedTreeItem[];
}

interface RenderTreeItem {
    value: string;
    label: string;
    id: string;
    groupId: string;
    level: number;
    position: number;
    setSize: number;
    branch: boolean;
    hasGroup: boolean;
    expanded: boolean;
    selected: boolean;
    selectionEnabled: boolean;
    disabled: boolean;
    loading: boolean;
    loadingLabel: string;
    tabIndex: number;
    marker: string;
    selectedMarker: string;
    children: RenderTreeItem[];
}

interface TreeState {
    connected: boolean;
    context?: CemProducedElementBehaviorContext;
    roots: NormalizedTreeItem[];
    flatItems: NormalizedTreeItem[];
    itemsByValue: Map<string, NormalizedTreeItem>;
    itemIds: Map<string, string>;
    ownerId: string;
    inputSignature: string;
    warnedInputSignature: string;
    issue: string | null;
    focusValue: string;
    refocusValue: string;
    typeaheadBuffer: string;
    lastTypeaheadAt: number;
    onClickCapture?: EventListener;
    onFocusOut?: EventListener;
    onKeyDown?: EventListener;
}

interface NormalizeTreeResult {
    roots: NormalizedTreeItem[];
    flatItems: NormalizedTreeItem[];
    itemsByValue: Map<string, NormalizedTreeItem>;
    issue: string | null;
}

const TREE_STATES = new WeakMap<HTMLElement, TreeState>();
const SELECTION_MODES = new Set<SelectionMode>(['none', 'single', 'multiple']);
const TYPEAHEAD_TIMEOUT_MS = 700;
let treeSequence = 0;

export const CEM_TREE_BEHAVIOR: CemProducedElementBehavior = {
    constructed(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        installHostApi(instance);
    },
    connected(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        if (state.connected) return;
        state.connected = true;
        state.onClickCapture = (event) => handleClick(instance, state, event);
        state.onFocusOut = (event) => handleFocusOut(instance, state, event);
        state.onKeyDown = (event) => handleKeyDown(instance, state, event as KeyboardEvent);
        instance.addEventListener('click', state.onClickCapture, true);
        instance.addEventListener('focusout', state.onFocusOut);
        instance.addEventListener('keydown', state.onKeyDown);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        synchronizeTree(instance, state, context.snapshot().payload.nodes);
        context.setSlices(renderSlices(instance, state), { render: false });
    },
    rendered(instance) {
        const state = stateFor(instance);
        if (!state.refocusValue) return;
        const value = state.refocusValue;
        state.refocusValue = '';
        directTreeItem(instance, value)?.focus({ preventScroll: true });
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onClickCapture) instance.removeEventListener('click', state.onClickCapture, true);
        if (state.onFocusOut) instance.removeEventListener('focusout', state.onFocusOut);
        if (state.onKeyDown) instance.removeEventListener('keydown', state.onKeyDown);
    },
};

function stateFor(instance: HTMLElement): TreeState {
    let state = TREE_STATES.get(instance);
    if (state) return state;
    treeSequence += 1;
    state = {
        connected: false,
        roots: [],
        flatItems: [],
        itemsByValue: new Map(),
        itemIds: new Map(),
        ownerId: `cem-tree-${treeSequence}`,
        inputSignature: '',
        warnedInputSignature: '',
        issue: null,
        focusValue: '',
        refocusValue: '',
        typeaheadBuffer: '',
        lastTypeaheadAt: 0,
    };
    TREE_STATES.set(instance, state);
    return state;
}

function synchronizeTree(
    instance: HTMLElement,
    state: TreeState,
    nodes: readonly SerializedPayloadNode[],
): void {
    const signature = [
        JSON.stringify(nodes),
        instance.getAttribute('label') ?? '',
        instance.getAttribute('selection') ?? '',
    ].join('\u0000');
    if (signature !== state.inputSignature) {
        const normalized = normalizeTree(instance, state, nodes);
        state.roots = normalized.roots;
        state.flatItems = normalized.flatItems;
        state.itemsByValue = normalized.itemsByValue;
        state.issue = normalized.issue;
        state.inputSignature = signature;
        if (normalized.issue && state.warnedInputSignature !== signature) {
            state.warnedInputSignature = signature;
            instance.ownerDocument.defaultView?.console.warn(`[cem-tree] ${normalized.issue}`);
        }
    }

    synchronizeFocus(instance, state);
}

function normalizeTree(
    instance: HTMLElement,
    state: TreeState,
    nodes: readonly SerializedPayloadNode[],
): NormalizeTreeResult {
    const empty = (): NormalizeTreeResult => ({
        roots: [],
        flatItems: [],
        itemsByValue: new Map(),
        issue: null,
    });
    const label = instance.getAttribute('label')?.trim() ?? '';
    if (!label) return { ...empty(), issue: 'Author a non-empty label attribute.' };
    if (!selectionMode(instance)) {
        return { ...empty(), issue: 'selection must be none, single, or multiple.' };
    }

    const values = new Set<string>();
    const flatItems: NormalizedTreeItem[] = [];
    const itemsByValue = new Map<string, NormalizedTreeItem>();
    let issue: string | null = null;

    const visit = (
        candidates: readonly SerializedPayloadNode[],
        parent: NormalizedTreeItem | null,
        level: number,
    ): NormalizedTreeItem[] => {
        if (issue) return [];
        const elements = candidates.filter(
            (node): node is Extract<SerializedPayloadNode, { kind: 'element' }> => node.kind === 'element',
        );
        const unexpected = candidates.some(
            (node) => node.kind !== 'comment' && (node.kind !== 'element' || node.tag !== 'cem-tree-item'),
        );
        if (unexpected || elements.some((node) => node.tag !== 'cem-tree-item')) {
            issue = 'Author only recursive cem-tree-item element children.';
            return [];
        }

        const setSize = elements.length;
        return elements.map((node, index) => {
            const value = node.attributes.value?.trim() ?? '';
            const itemLabel = node.attributes.label?.trim() ?? '';
            if (!value || !itemLabel) {
                issue = 'Every cem-tree-item requires non-empty value and label attributes.';
            } else if (/\s/u.test(value)) {
                issue = `cem-tree-item value "${value}" may not contain whitespace.`;
            } else if (values.has(value)) {
                issue = `Duplicate cem-tree-item value "${value}" is not allowed.`;
            }
            values.add(value);
            const id = itemId(state, value || `invalid-${level}-${index}`);
            const item: NormalizedTreeItem = {
                value,
                label: itemLabel,
                id,
                groupId: `${id}-group`,
                level,
                position: index + 1,
                setSize,
                expandable: Object.hasOwn(node.attributes, 'expandable'),
                disabled: Object.hasOwn(node.attributes, 'disabled'),
                loading: Object.hasOwn(node.attributes, 'loading'),
                parent,
                children: [],
            };
            flatItems.push(item);
            itemsByValue.set(value, item);
            item.children = visit(node.children, item, level + 1);
            return item;
        });
    };

    const roots = visit(nodes, null, 1);
    if (!issue && roots.length === 0) issue = 'Author at least one direct cem-tree-item.';
    if (issue) return { ...empty(), issue };
    return { roots, flatItems, itemsByValue, issue: null };
}

function itemId(state: TreeState, value: string): string {
    const existing = state.itemIds.get(value);
    if (existing) return existing;
    const id = `${state.ownerId}-item-${state.itemIds.size}`;
    state.itemIds.set(value, id);
    return id;
}

function renderSlices(instance: HTMLElement, state: TreeState): Record<string, unknown> {
    const mode = selectionMode(instance) ?? 'none';
    const expanded = new Set(tokenList(instance, 'expanded-values'));
    const selected = selectedSet(instance, state, mode);
    const loadingLabel = instance.getAttribute('loading-label')?.trim() || 'Loading';
    const hostDisabled = instance.hasAttribute('disabled');

    const renderItems = (
        items: readonly NormalizedTreeItem[],
        visible: boolean,
        ancestorDisabled: boolean,
    ): RenderTreeItem[] => items.map((item) => {
        const branch = item.expandable || item.children.length > 0;
        const itemExpanded = branch && expanded.has(item.value);
        const itemDisabled = hostDisabled || ancestorDisabled || item.disabled;
        const itemSelected = selected.has(item.value);
        return {
            value: item.value,
            label: item.label,
            id: item.id,
            groupId: item.groupId,
            level: item.level,
            position: item.position,
            setSize: item.setSize,
            branch,
            hasGroup: branch,
            expanded: itemExpanded,
            selected: itemSelected,
            selectionEnabled: mode !== 'none',
            disabled: itemDisabled,
            loading: item.loading,
            loadingLabel,
            tabIndex: visible && !itemDisabled && state.focusValue === item.value ? 0 : -1,
            marker: branch ? (itemExpanded ? '▾' : '▸') : '',
            selectedMarker: itemSelected ? '✓' : '',
            children: renderItems(item.children, visible && itemExpanded, itemDisabled),
        };
    });

    return {
        authoringValid: state.issue === null,
        multiple: mode === 'multiple',
        items: renderItems(state.roots, true, false),
    };
}

function synchronizeFocus(instance: HTMLElement, state: TreeState): void {
    if (state.issue) {
        state.focusValue = '';
        return;
    }
    const visible = visibleEnabledItems(instance, state);
    if (visible.length === 0) {
        state.focusValue = '';
        return;
    }

    const active = instance.ownerDocument.activeElement;
    const activeButton = active instanceof HTMLButtonElement && isDirectTreeItem(instance, active)
        ? active
        : null;
    const activeValue = activeButton?.dataset.treeValue ?? '';
    if (activeValue && visible.some((item) => item.value === activeValue)) {
        state.focusValue = activeValue;
        return;
    }
    if (visible.some((item) => item.value === state.focusValue)) return;

    const previous = state.itemsByValue.get(activeValue || state.focusValue);
    let ancestor = previous?.parent ?? null;
    while (ancestor) {
        if (visible.some((item) => item.value === ancestor?.value)) {
            state.focusValue = ancestor.value;
            if (activeButton) state.refocusValue = ancestor.value;
            return;
        }
        ancestor = ancestor.parent;
    }

    const mode = selectionMode(instance) ?? 'none';
    const selected = selectedSet(instance, state, mode);
    const preferred = visible.find((item) => selected.has(item.value)) ?? visible[0];
    state.focusValue = preferred?.value ?? '';
    if (activeButton) state.refocusValue = state.focusValue;
}

function visibleEnabledItems(instance: HTMLElement, state: TreeState): NormalizedTreeItem[] {
    const expanded = new Set(tokenList(instance, 'expanded-values'));
    const hostDisabled = instance.hasAttribute('disabled');
    const visible: NormalizedTreeItem[] = [];
    const visit = (
        items: readonly NormalizedTreeItem[],
        ancestorVisible: boolean,
        ancestorDisabled: boolean,
    ): void => {
        for (const item of items) {
            const itemDisabled = hostDisabled || ancestorDisabled || item.disabled;
            if (ancestorVisible && !itemDisabled) visible.push(item);
            const branchVisible = ancestorVisible && expanded.has(item.value);
            visit(item.children, branchVisible, itemDisabled);
        }
    };
    visit(state.roots, true, false);
    return visible;
}

function selectedSet(instance: HTMLElement, state: TreeState, mode: SelectionMode): Set<string> {
    if (mode === 'none') return new Set();
    const requested = new Set(tokenList(instance, 'selected-values'));
    if (mode === 'multiple') {
        return new Set(state.flatItems.filter((item) => requested.has(item.value)).map((item) => item.value));
    }
    const first = state.flatItems.find((item) => requested.has(item.value));
    return new Set(first ? [first.value] : []);
}

function selectionMode(instance: HTMLElement): SelectionMode | null {
    const value = instance.getAttribute('selection')?.trim() || 'none';
    return SELECTION_MODES.has(value as SelectionMode) ? value as SelectionMode : null;
}

function handleClick(instance: HTMLElement, state: TreeState, event: Event): void {
    if (event.defaultPrevented) return;
    const target = event.target instanceof Element ? event.target : null;
    const button = target?.closest<HTMLButtonElement>('button.cem-tree__node') ?? null;
    if (!button || !isDirectTreeItem(instance, button)) return;
    const item = state.itemsByValue.get(button.dataset.treeValue ?? '');
    if (!item || button.getAttribute('aria-disabled') === 'true') {
        event.preventDefault();
        event.stopImmediatePropagation();
        return;
    }

    state.focusValue = item.value;
    state.refocusValue = item.value;
    if (isBranch(item)) toggleItem(instance, state, item);
    else activateItem(instance, item);
}

function handleFocusOut(instance: HTMLElement, state: TreeState, event: Event): void {
    const target = event.target instanceof HTMLButtonElement ? event.target : null;
    if (!target || !isDirectTreeItem(instance, target)) return;
    const value = target.dataset.treeValue ?? '';
    queueMicrotask(() => {
        if (instance.contains(instance.ownerDocument.activeElement)) return;
        const item = state.itemsByValue.get(value);
        const visible = visibleEnabledItems(instance, state);
        if (!item || visible.some((candidate) => candidate.value === value)) return;
        let ancestor = item.parent;
        while (ancestor && !visible.some((candidate) => candidate.value === ancestor?.value)) {
            ancestor = ancestor.parent;
        }
        if (!ancestor) return;
        state.focusValue = ancestor.value;
        for (const button of instance.querySelectorAll<HTMLButtonElement>('button.cem-tree__node')) {
            if (isDirectTreeItem(instance, button)) button.tabIndex = button.dataset.treeValue === ancestor.value ? 0 : -1;
        }
        directTreeItem(instance, ancestor.value)?.focus({ preventScroll: true });
    });
}

function handleKeyDown(instance: HTMLElement, state: TreeState, event: KeyboardEvent): void {
    const target = event.target instanceof HTMLButtonElement ? event.target : null;
    if (!target || !isDirectTreeItem(instance, target) || target.getAttribute('aria-disabled') === 'true') return;
    const item = state.itemsByValue.get(target.dataset.treeValue ?? '');
    if (!item) return;

    const visible = visibleEnabledItems(instance, state);
    const current = visible.findIndex((candidate) => candidate.value === item.value);
    if (current < 0) return;

    let destination: NormalizedTreeItem | undefined;
    if (event.key === 'ArrowDown') destination = visible[current + 1];
    else if (event.key === 'ArrowUp') destination = visible[current - 1];
    else if (event.key === 'Home') destination = visible[0];
    else if (event.key === 'End') destination = visible.at(-1);
    else if (event.key === 'ArrowRight') {
        if (!isBranch(item)) {
            event.preventDefault();
            return;
        }
        if (!expandedSet(instance).has(item.value)) {
            event.preventDefault();
            toggleItem(instance, state, item, true);
            return;
        }
        destination = item.children.find((child) => visible.some((candidate) => candidate.value === child.value));
    } else if (event.key === 'ArrowLeft') {
        if (isBranch(item) && expandedSet(instance).has(item.value)) {
            event.preventDefault();
            toggleItem(instance, state, item, false);
            return;
        }
        let ancestor = item.parent;
        while (ancestor && !visible.some((candidate) => candidate.value === ancestor?.value)) {
            ancestor = ancestor.parent;
        }
        destination = ancestor ?? undefined;
    } else if (isTypeaheadKey(event)) {
        event.preventDefault();
        destination = typeaheadDestination(state, visible, current, event.key);
    } else {
        return;
    }

    event.preventDefault();
    if (destination) focusItem(instance, state, destination.value);
}

function typeaheadDestination(
    state: TreeState,
    visible: readonly NormalizedTreeItem[],
    current: number,
    key: string,
): NormalizedTreeItem | undefined {
    const now = Date.now();
    const normalizedKey = key.toLocaleLowerCase();
    if (now - state.lastTypeaheadAt > TYPEAHEAD_TIMEOUT_MS) state.typeaheadBuffer = '';
    const repeated = state.typeaheadBuffer.length > 0
        && [...state.typeaheadBuffer].every((character) => character === normalizedKey);
    state.typeaheadBuffer = repeated ? normalizedKey : `${state.typeaheadBuffer}${normalizedKey}`;
    state.lastTypeaheadAt = now;

    for (let offset = 1; offset <= visible.length; offset += 1) {
        const candidate = visible[(current + offset) % visible.length];
        if (candidate?.label.toLocaleLowerCase().startsWith(state.typeaheadBuffer)) return candidate;
    }
    return undefined;
}

function isTypeaheadKey(event: KeyboardEvent): boolean {
    return event.key.length === 1
        && event.key !== ' '
        && !event.altKey
        && !event.ctrlKey
        && !event.metaKey;
}

function focusItem(instance: HTMLElement, state: TreeState, value: string): void {
    state.focusValue = value;
    state.refocusValue = value;
    directTreeItem(instance, value)?.focus({ preventScroll: true });
    state.context?.setSlices(renderSlices(instance, state));
}

function toggleItem(
    instance: HTMLElement,
    state: TreeState,
    item: NormalizedTreeItem,
    forced?: boolean,
): void {
    if (!isBranch(item)) return;
    const values = tokenList(instance, 'expanded-values');
    const expanded = new Set(values);
    const next = forced ?? !expanded.has(item.value);
    if (next) expanded.add(item.value);
    else expanded.delete(item.value);
    const ordered = [...values.filter((value) => expanded.has(value) && value !== item.value)];
    if (next) ordered.push(item.value);
    instance.setAttribute('expanded-values', ordered.join(' '));
    state.focusValue = item.value;
    state.refocusValue = item.value;
    instance.dispatchEvent(new CustomEvent<CemTreeToggleDetail>('cem-tree-toggle', {
        bubbles: true,
        composed: true,
        detail: { value: item.value, expanded: next },
    }));
}

function activateItem(instance: HTMLElement, item: NormalizedTreeItem): void {
    instance.dispatchEvent(new CustomEvent<CemTreeActivateDetail>('cem-tree-activate', {
        bubbles: true,
        composed: true,
        detail: { value: item.value },
    }));
}

function isBranch(item: NormalizedTreeItem): boolean {
    return item.expandable || item.children.length > 0;
}

function expandedSet(instance: HTMLElement): Set<string> {
    return new Set(tokenList(instance, 'expanded-values'));
}

function tokenList(instance: HTMLElement, attribute: string): string[] {
    const values = (instance.getAttribute(attribute) ?? '').trim().split(/\s+/u).filter(Boolean);
    return [...new Set(values)];
}

function isDirectTreeItem(instance: HTMLElement, button: HTMLButtonElement): boolean {
    return button.classList.contains('cem-tree__node') && button.closest('cem-tree') === instance;
}

function directTreeItem(instance: HTMLElement, value: string): HTMLButtonElement | null {
    for (const button of instance.querySelectorAll<HTMLButtonElement>('button.cem-tree__node')) {
        if (button.dataset.treeValue === value && isDirectTreeItem(instance, button)) return button;
    }
    return null;
}

function installHostApi(instance: HTMLElement): void {
    installTokenListProperty(instance, 'expandedValues', 'expanded-values');
    installTokenListProperty(instance, 'selectedValues', 'selected-values');
}

function installTokenListProperty(instance: HTMLElement, property: string, attribute: string): void {
    if (Object.getOwnPropertyDescriptor(instance, property)) return;
    Object.defineProperty(instance, property, {
        configurable: true,
        enumerable: true,
        get: () => tokenList(instance, attribute),
        set: (value: unknown) => {
            const source = typeof value === 'string' ? value.split(/\s+/u) : Array.isArray(value) ? value : [];
            const tokens = source.map(String).map((token) => token.trim()).filter((token) => token && !/\s/u.test(token));
            instance.setAttribute(attribute, [...new Set(tokens)].join(' '));
        },
    });
}
