import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';

interface NavigationBehaviorState {
    connected: boolean;
    onAuxClick?: EventListener;
    onClick?: EventListener;
    onKeyDown?: EventListener;
    onKeyUp?: EventListener;
}

const NAVIGATION_STATES = new WeakMap<HTMLElement, NavigationBehaviorState>();

export const CEM_NAVIGATION_BEHAVIOR: CemProducedElementBehavior = {
    connected(instance) {
        const state = stateFor(instance);
        if (state.connected) return;
        state.connected = true;

        state.onClick = (event) => suppressDisabledClick(instance, event);
        state.onAuxClick = (event) => suppressDisabledClick(instance, event);
        state.onKeyDown = (event) => suppressDisabledKey(instance, event as KeyboardEvent);
        state.onKeyUp = (event) => suppressDisabledKey(instance, event as KeyboardEvent);
        instance.addEventListener('click', state.onClick, true);
        instance.addEventListener('auxclick', state.onAuxClick, true);
        instance.addEventListener('keydown', state.onKeyDown, true);
        instance.addEventListener('keyup', state.onKeyUp, true);
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onClick) instance.removeEventListener('click', state.onClick, true);
        if (state.onAuxClick) instance.removeEventListener('auxclick', state.onAuxClick, true);
        if (state.onKeyDown) instance.removeEventListener('keydown', state.onKeyDown, true);
        if (state.onKeyUp) instance.removeEventListener('keyup', state.onKeyUp, true);
    },
};

function stateFor(instance: HTMLElement): NavigationBehaviorState {
    let state = NAVIGATION_STATES.get(instance);
    if (state) return state;
    state = { connected: false };
    NAVIGATION_STATES.set(instance, state);
    return state;
}

function suppressDisabledClick(instance: HTMLElement, event: Event): void {
    if (disabledOwnerForEvent(instance, event)) suppress(event);
}

function suppressDisabledKey(instance: HTMLElement, event: KeyboardEvent): void {
    const owner = disabledOwnerForEvent(instance, event);
    if (!owner) return;
    if (event.key === 'Enter' || (event.key === ' ' && owner.localName === 'button')) {
        suppress(event);
    }
}

function disabledOwnerForEvent(instance: HTMLElement, event: Event): HTMLElement | null {
    const target = event.target instanceof Element ? event.target : null;
    const owner = target?.closest<HTMLElement>('a[href], button') ?? null;
    if (!owner || owner.getAttribute('aria-disabled') !== 'true') return null;
    return isDirectNavigationOwner(instance, owner) ? owner : null;
}

function isDirectNavigationOwner(instance: HTMLElement, owner: HTMLElement): boolean {
    const parent = owner.parentElement;
    if (!parent) return false;

    if (instance.localName === 'cem-nav') {
        if (parent.localName === 'nav' && parent.parentElement === instance) return true;
        const nav = parent.parentElement;
        return parent.classList.contains('cem-nav__content') && nav?.localName === 'nav' && nav.parentElement === instance;
    }

    return false;
}

function suppress(event: Event): void {
    event.preventDefault();
    event.stopImmediatePropagation();
}
