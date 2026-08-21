import type {
    CemProducedElementBehavior,
    CemProducedElementBehaviorContext,
    SerializedPayloadNode,
} from '@epa-wg/cem-elements';

export interface CemTabDetail {
    value: string;
    index: number;
    previousIndex: number;
}

interface NormalizedTab {
    value: string;
    label: string;
    children: SerializedPayloadNode[];
    disabled: boolean;
}

interface RenderTab extends NormalizedTab {
    index: number;
    buttonId: string;
    panelId: string;
    selected: boolean;
    tabIndex: number;
}

interface TabsState {
    connected: boolean;
    context?: CemProducedElementBehaviorContext;
    tabs: NormalizedTab[];
    payloadSignature: string;
    warnedPayloadSignature: string;
    issue: string | null;
    ownerId: string;
    valueIds: Map<string, string>;
    nextValueId: number;
    focusIndex: number;
    refocusIndex: number;
    onClickCapture?: EventListener;
    onFocusIn?: EventListener;
    onFocusOut?: EventListener;
    onKeyDown?: EventListener;
}

interface NormalizeTabsResult {
    tabs: NormalizedTab[];
    issue: string | null;
}

const TABS_STATES = new WeakMap<HTMLElement, TabsState>();
let tabsSequence = 0;

export const CEM_TABS_BEHAVIOR: CemProducedElementBehavior = {
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
        state.onClickCapture = (event) => handleClick(instance, state, event);
        state.onFocusIn = (event) => handleFocusIn(instance, state, event);
        state.onFocusOut = (event) => handleFocusOut(instance, state, event as FocusEvent);
        state.onKeyDown = (event) => handleKeyDown(instance, state, event as KeyboardEvent);
        instance.addEventListener('click', state.onClickCapture, true);
        instance.addEventListener('focusin', state.onFocusIn);
        instance.addEventListener('focusout', state.onFocusOut);
        instance.addEventListener('keydown', state.onKeyDown);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        state.context = context;
        const focusedPanelIndex = directPanelIndex(instance, instance.ownerDocument.activeElement);
        synchronizeTabs(instance, state, context.snapshot().payload.nodes);
        const selected = selectedIndex(instance, state);
        if (focusedPanelIndex >= 0 && focusedPanelIndex !== selected) {
            state.refocusIndex = selected;
        }
        context.setSlices(renderSlices(instance, state), { render: false });
    },
    rendered(instance) {
        const state = stateFor(instance);
        if (state.refocusIndex < 0) return;
        const index = state.refocusIndex;
        state.refocusIndex = -1;
        directTab(instance, index)?.focus({ preventScroll: true });
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onClickCapture) instance.removeEventListener('click', state.onClickCapture, true);
        if (state.onFocusIn) instance.removeEventListener('focusin', state.onFocusIn);
        if (state.onFocusOut) instance.removeEventListener('focusout', state.onFocusOut);
        if (state.onKeyDown) instance.removeEventListener('keydown', state.onKeyDown);
    },
};

function stateFor(instance: HTMLElement): TabsState {
    let state = TABS_STATES.get(instance);
    if (state) return state;
    tabsSequence += 1;
    state = {
        connected: false,
        tabs: [],
        payloadSignature: '',
        warnedPayloadSignature: '',
        issue: null,
        ownerId: `cem-tabs-${tabsSequence}`,
        valueIds: new Map(),
        nextValueId: 0,
        focusIndex: -1,
        refocusIndex: -1,
    };
    TABS_STATES.set(instance, state);
    return state;
}

function synchronizeTabs(
    instance: HTMLElement,
    state: TabsState,
    nodes: readonly SerializedPayloadNode[],
): void {
    const signature = JSON.stringify(nodes);
    if (signature !== state.payloadSignature) {
        const normalized = normalizeTabs(nodes);
        state.tabs = normalized.tabs;
        state.issue = normalized.issue;
        state.payloadSignature = signature;
        if (normalized.issue && state.warnedPayloadSignature !== signature) {
            state.warnedPayloadSignature = signature;
            instance.ownerDocument.defaultView?.console.warn(`[cem-tabs] ${normalized.issue}`);
        }
    }

    const selected = selectedIndex(instance, state);
    if (!focusableIndex(state, state.focusIndex)) state.focusIndex = selected;
}

function normalizeTabs(nodes: readonly SerializedPayloadNode[]): NormalizeTabsResult {
    const elements = nodes.filter(
        (node): node is Extract<SerializedPayloadNode, { kind: 'element' }> => node.kind === 'element',
    );
    const unexpected = nodes.some(
        (node) => node.kind !== 'comment' && (node.kind !== 'element' || node.tag !== 'cem-tab'),
    );
    if (unexpected || elements.some((node) => node.tag !== 'cem-tab')) {
        return { tabs: [], issue: 'Author only direct cem-tab element children.' };
    }
    if (elements.length === 0) return { tabs: [], issue: 'Author at least one direct cem-tab.' };

    const tabs: NormalizedTab[] = [];
    const values = new Set<string>();
    for (const node of elements) {
        const value = node.attributes.value?.trim() ?? '';
        const label = node.attributes.label?.trim() ?? '';
        if (!value || !label) {
            return { tabs: [], issue: 'Every cem-tab requires non-empty value and label attributes.' };
        }
        if (values.has(value)) return { tabs: [], issue: `Duplicate cem-tab value "${value}" is not allowed.` };
        values.add(value);
        tabs.push({
            value,
            label,
            children: node.children,
            disabled: Object.hasOwn(node.attributes, 'disabled'),
        });
    }
    if (tabs.every(({ disabled }) => disabled)) {
        return { tabs: [], issue: 'Author at least one enabled cem-tab.' };
    }
    return { tabs, issue: null };
}

function renderSlices(instance: HTMLElement, state: TabsState): Record<string, unknown> {
    const selected = selectedIndex(instance, state);
    return {
        authoringValid: state.issue === null,
        orientation: normalizedOrientation(instance),
        tabs: state.tabs.map((tab, index): RenderTab => {
            const ids = stableIds(state, tab.value);
            return {
                ...tab,
                index,
                buttonId: ids.button,
                panelId: ids.panel,
                selected: index === selected,
                tabIndex: !tab.disabled && index === state.focusIndex ? 0 : -1,
            };
        }),
    };
}

function stableIds(state: TabsState, value: string): { button: string; panel: string } {
    let id = state.valueIds.get(value);
    if (!id) {
        state.nextValueId += 1;
        id = String(state.nextValueId);
        state.valueIds.set(value, id);
    }
    return {
        button: `${state.ownerId}-tab-${id}`,
        panel: `${state.ownerId}-panel-${id}`,
    };
}

function normalizedOrientation(instance: HTMLElement): 'horizontal' | 'vertical' {
    return instance.getAttribute('orientation') === 'vertical' ? 'vertical' : 'horizontal';
}

function selectedIndex(instance: HTMLElement, state: TabsState): number {
    if (state.tabs.length === 0) return 0;
    const parsed = Number.parseInt(instance.getAttribute('selected-index') ?? '', 10);
    const requested = Number.isFinite(parsed) ? Math.min(Math.max(parsed, 0), state.tabs.length - 1) : 0;
    if (!state.tabs[requested]?.disabled) return requested;
    for (let offset = 1; offset <= state.tabs.length; offset += 1) {
        const candidate = (requested + offset) % state.tabs.length;
        if (!state.tabs[candidate]?.disabled) return candidate;
    }
    return 0;
}

function focusableIndex(state: TabsState, index: number): boolean {
    return index >= 0 && index < state.tabs.length && !state.tabs[index]?.disabled;
}

function firstFocusableIndex(state: TabsState): number {
    return state.tabs.findIndex(({ disabled }) => !disabled);
}

function lastFocusableIndex(state: TabsState): number {
    for (let index = state.tabs.length - 1; index >= 0; index -= 1) {
        if (!state.tabs[index]?.disabled) return index;
    }
    return -1;
}

function handleClick(instance: HTMLElement, state: TabsState, event: Event): void {
    if (event.defaultPrevented) return;
    const target = event.target instanceof Element ? event.target : null;
    const button = target?.closest<HTMLButtonElement>('button.cem-tabs__tab') ?? null;
    const index = button ? tabIndex(instance, button) : -1;
    if (index < 0 || state.tabs[index]?.disabled) return;

    const previousIndex = selectedIndex(instance, state);
    if (index === previousIndex) return;
    state.focusIndex = index;
    state.refocusIndex = index;
    instance.setAttribute('selected-index', String(index));
    const detail: CemTabDetail = {
        value: state.tabs[index]?.value ?? '',
        index,
        previousIndex,
    };
    instance.dispatchEvent(new CustomEvent<CemTabDetail>('cem-tab', {
        bubbles: true,
        composed: true,
        detail,
    }));
}

function handleKeyDown(instance: HTMLElement, state: TabsState, event: KeyboardEvent): void {
    const target = event.target instanceof Element
        ? event.target.closest<HTMLButtonElement>('button.cem-tabs__tab')
        : null;
    const current = target ? tabIndex(instance, target) : -1;
    if (current < 0) return;

    const orientation = normalizedOrientation(instance);
    let destination: number;
    if (event.key === 'Home') destination = firstFocusableIndex(state);
    else if (event.key === 'End') destination = lastFocusableIndex(state);
    else if ((orientation === 'horizontal' && event.key === 'ArrowRight')
        || (orientation === 'vertical' && event.key === 'ArrowDown')) {
        destination = adjacentFocusableIndex(state, current, 1);
    } else if ((orientation === 'horizontal' && event.key === 'ArrowLeft')
        || (orientation === 'vertical' && event.key === 'ArrowUp')) {
        destination = adjacentFocusableIndex(state, current, -1);
    } else {
        return;
    }

    if (destination < 0) return;
    event.preventDefault();
    state.focusIndex = destination;
    state.refocusIndex = destination;
    state.context?.setSlices(renderSlices(instance, state));
}

function adjacentFocusableIndex(state: TabsState, start: number, direction: 1 | -1): number {
    if (state.tabs.length === 0) return -1;
    for (let offset = 1; offset <= state.tabs.length; offset += 1) {
        const candidate = (start + direction * offset + state.tabs.length) % state.tabs.length;
        if (focusableIndex(state, candidate)) return candidate;
    }
    return -1;
}

function handleFocusIn(instance: HTMLElement, state: TabsState, event: Event): void {
    const target = event.target instanceof Element
        ? event.target.closest<HTMLButtonElement>('button.cem-tabs__tab')
        : null;
    const index = target ? tabIndex(instance, target) : -1;
    if (index < 0 || index === state.focusIndex) return;
    state.focusIndex = index;
    state.refocusIndex = index;
    state.context?.setSlices(renderSlices(instance, state));
}

function handleFocusOut(instance: HTMLElement, state: TabsState, event: FocusEvent): void {
    const related = event.relatedTarget;
    if (related instanceof Node && instance.contains(related)) return;
    const selected = selectedIndex(instance, state);
    if (state.focusIndex === selected) return;
    state.focusIndex = selected;
    state.context?.setSlices(renderSlices(instance, state));
}

function tabIndex(instance: HTMLElement, button: HTMLButtonElement): number {
    if (!isDirectTab(instance, button)) return -1;
    const index = Number.parseInt(button.dataset.tabIndex ?? '', 10);
    return Number.isFinite(index) ? index : -1;
}

function isDirectTab(instance: HTMLElement, button: HTMLButtonElement): boolean {
    const list = button.parentElement;
    return list?.classList.contains('cem-tabs__list') === true && list.parentElement === instance;
}

function directTab(instance: HTMLElement, index: number): HTMLButtonElement | null {
    const candidate = instance.querySelector<HTMLButtonElement>(
        `:scope > .cem-tabs__list > .cem-tabs__tab[data-tab-index="${index}"]`,
    );
    return candidate && isDirectTab(instance, candidate) ? candidate : null;
}

function directPanelIndex(instance: HTMLElement, activeElement: Element | null): number {
    const panel = activeElement?.closest<HTMLElement>('.cem-tabs__panel') ?? null;
    const panels = panel?.parentElement;
    if (!panel || panels?.classList.contains('cem-tabs__panels') !== true || panels.parentElement !== instance) {
        return -1;
    }
    const index = Number.parseInt(panel.dataset.tabIndex ?? '', 10);
    return Number.isFinite(index) ? index : -1;
}

function installHostApi(instance: HTMLElement, state: TabsState): void {
    if (Object.getOwnPropertyDescriptor(instance, 'selectedIndex')) return;
    Object.defineProperty(instance, 'selectedIndex', {
        configurable: true,
        enumerable: true,
        get: () => selectedIndex(instance, state),
        set: (value: unknown) => {
            const parsed = Number(value);
            const requested = Number.isFinite(parsed) ? Math.floor(parsed) : 0;
            const clamped = state.tabs.length > 0
                ? Math.min(Math.max(requested, 0), state.tabs.length - 1)
                : 0;
            const normalized = state.tabs[clamped]?.disabled
                ? firstEnabledFrom(state, clamped)
                : clamped;
            instance.setAttribute('selected-index', String(Math.max(normalized, 0)));
        },
    });
}

function firstEnabledFrom(state: TabsState, start: number): number {
    for (let offset = 1; offset <= state.tabs.length; offset += 1) {
        const candidate = (start + offset) % state.tabs.length;
        if (!state.tabs[candidate]?.disabled) return candidate;
    }
    return -1;
}
