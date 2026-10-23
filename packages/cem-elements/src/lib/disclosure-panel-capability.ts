import type { CemProducedElementBehavior } from './cem-elements.js';

interface ExpansionState {
    connected: boolean;
    headingId: string;
    onClick?: EventListener;
    panelId: string;
    summaryId: string;
    refocusHeader?: boolean;
}

const EXPANSION_STATES = new WeakMap<HTMLElement, ExpansionState>();
const HEADING_LEVELS = new Set(['1', '2', '3', '4', '5', '6']);
let expansionSequence = 0;

export const CEM_DISCLOSURE_PANEL_CAPABILITY: CemProducedElementBehavior = {
    connected(instance) {
        const state = stateFor(instance);
        if (state.connected) return;
        state.connected = true;
        state.onClick = (event) => handleClick(instance, event);
        instance.addEventListener('click', state.onClick);
    },
    beforeRender(instance, context) {
        const state = stateFor(instance);
        const panel = instance.querySelector<HTMLElement>(':scope > [part~="root"] > [part~="panel"]');
        if (!instance.hasAttribute('expanded') && panel && !panel.hidden && panel.contains(instance.ownerDocument.activeElement)) {
            state.refocusHeader = true;
        }
        context.setSlices({
            headingId: state.headingId,
            headingLevel: normalizedHeadingLevel(instance),
            panelId: state.panelId,
            summaryId: state.summaryId,
        }, { render: false });
    },
    rendered(instance) {
        const state = stateFor(instance);
        if (!state.refocusHeader) return;
        state.refocusHeader = false;
        instance.querySelector<HTMLButtonElement>(':scope > [part~="root"] > [part~="heading"] > button[part~="header"]')?.focus({ preventScroll: true });
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        state.refocusHeader = false;
        if (state.onClick) instance.removeEventListener('click', state.onClick);
    },
};

function stateFor(instance: HTMLElement): ExpansionState {
    let state = EXPANSION_STATES.get(instance);
    if (state) return state;
    expansionSequence += 1;
    state = {
        connected: false,
        headingId: `cem-disclosure-${expansionSequence}-heading`,
        panelId: `cem-disclosure-${expansionSequence}-panel`,
        summaryId: `cem-disclosure-${expansionSequence}-summary`,
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
    const button = target?.closest<HTMLButtonElement>('button[part~=header]') ?? null;
    if (!button || button.disabled || instance.hasAttribute('disabled') || !isDirectHeader(instance, button)) return;
    const expanded = instance.toggleAttribute('expanded');
    const value = instance.getAttribute('value')?.trim();
    if (instance.hasAttribute('notify-toggle') && value && !/\s/u.test(value)) {
        instance.dispatchEvent(new CustomEvent('cem-disclosure-toggle', {
            bubbles: true, composed: true, detail: { value, expanded },
        }));
    }
}

function isDirectHeader(instance: HTMLElement, button: HTMLButtonElement): boolean {
    const heading = button.parentElement;
    const surface = heading?.parentElement;
    return (
        heading?.matches('[part~=heading][role=heading]') === true
        && surface?.matches('[part~=root]') === true
        && surface.parentElement === instance
    );
}

/** Private generated IDs and retained expanded intent, inspected without creating state. */
export function cemDisclosurePanelCheckpoint(instance: HTMLElement) {
    const state = EXPANSION_STATES.get(instance);
    if (!state?.connected || !state.onClick || state.refocusHeader || !instance.isConnected) return;
    const heading = instance.querySelector<HTMLElement>(':scope > [part~="root"] > [part~="heading"]');
    const button = heading?.querySelector<HTMLButtonElement>(':scope > button[part~="header"]');
    const summary = button?.querySelector<HTMLElement>('[part~="summary"]');
    const panel = instance.querySelector<HTMLElement>(':scope > [part~="root"] > [part~="panel"]');
    const expanded = instance.hasAttribute('expanded'), headingLevel = normalizedHeadingLevel(instance);
    if (!button || !summary || !panel || button.id !== state.headingId || panel.id !== state.panelId || summary.id !== state.summaryId
        || heading?.getAttribute('aria-level') !== headingLevel || button.getAttribute('aria-controls') !== panel.id
        || button.getAttribute('aria-labelledby') !== summary.id || panel.getAttribute('aria-labelledby') !== button.id
        || button.getAttribute('aria-expanded') !== String(expanded) || panel.hidden === expanded) return;
    const root = instance.getRootNode() as ParentNode;
    for (const node of [button, summary, panel]) if (root.querySelectorAll(`[id="${CSS.escape(node.id)}"]`).length !== 1) return;
    return { kind: 'disclosure-panel' as const, headingId: state.headingId, panelId: state.panelId, summaryId: state.summaryId, headingLevel, expanded };
}
export function prepareCemDisclosurePanelGuard(host: HTMLElement) {
    const state = EXPANSION_STATES.get(host), nodes = [...host.querySelectorAll('[part~=root],[part~=heading],[part~=header],[part~=summary],[part~=panel]')];
    if (!cemDisclosurePanelCheckpoint(host)) throw Error('cem.checkpoint.resume.disclosure-stale');
    const listener = state?.onClick;
    return () => EXPANSION_STATES.get(host) === state && state?.onClick === listener && !!cemDisclosurePanelCheckpoint(host)
        && nodes.length === host.querySelectorAll('[part~=root],[part~=heading],[part~=header],[part~=summary],[part~=panel]').length
        && nodes.every(node => node.isConnected && host.contains(node));
}
