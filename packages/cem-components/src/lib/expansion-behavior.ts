import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';

interface ExpansionState {
    connected: boolean;
    headingId: string;
    onClick?: EventListener;
    panelId: string;
    summaryId: string;
}

const EXPANSION_STATES = new WeakMap<HTMLElement, ExpansionState>();
const HEADING_LEVELS = new Set(['1', '2', '3', '4', '5', '6']);
let expansionSequence = 0;

export const CEM_EXPANSION_BEHAVIOR: CemProducedElementBehavior = {
    connected(instance) {
        const state = stateFor(instance);
        if (state.connected) return;
        state.connected = true;
        state.onClick = (event) => handleClick(instance, event);
        instance.addEventListener('click', state.onClick);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        context.setSlices({
            headingId: state.headingId,
            headingLevel: normalizedHeadingLevel(instance),
            panelId: state.panelId,
            summaryId: state.summaryId,
        }, { render: false });
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onClick) instance.removeEventListener('click', state.onClick);
    },
};

function stateFor(instance: HTMLElement): ExpansionState {
    let state = EXPANSION_STATES.get(instance);
    if (state) return state;
    expansionSequence += 1;
    state = {
        connected: false,
        headingId: `cem-expansion-${expansionSequence}-heading`,
        panelId: `cem-expansion-${expansionSequence}-panel`,
        summaryId: `cem-expansion-${expansionSequence}-summary`,
    };
    EXPANSION_STATES.set(instance, state);
    return state;
}

function normalizedHeadingLevel(instance: HTMLElement): string {
    const authored = instance.getAttribute('heading-level');
    return authored && HEADING_LEVELS.has(authored) ? authored : '3';
}

function handleClick(instance: HTMLElement, event: Event): void {
    if (event.defaultPrevented) return;
    const target = event.target instanceof Element ? event.target : null;
    const button = target?.closest<HTMLButtonElement>('button.cem-expansion__header') ?? null;
    if (!button || button.disabled || !isDirectHeader(instance, button)) return;
    instance.toggleAttribute('expanded');
}

function isDirectHeader(instance: HTMLElement, button: HTMLButtonElement): boolean {
    const heading = button.parentElement;
    const surface = heading?.parentElement;
    return (
        heading?.classList.contains('cem-expansion__heading') === true
        && surface?.classList.contains('cem-expansion') === true
        && surface.parentElement === instance
    );
}
