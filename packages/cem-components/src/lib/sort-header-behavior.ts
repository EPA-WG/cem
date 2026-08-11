import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';

export type CemSortDirection = 'none' | 'ascending' | 'descending';

export interface CemSortDetail {
    direction: CemSortDirection;
    name: string;
    previousDirection: CemSortDirection;
}

interface SortHeaderState {
    connected: boolean;
    onClick?: EventListener;
}

const SORT_HEADER_STATES = new WeakMap<HTMLElement, SortHeaderState>();

export const CEM_SORT_HEADER_BEHAVIOR: CemProducedElementBehavior = {
    connected(instance) {
        const state = stateFor(instance);
        if (state.connected) return;
        state.connected = true;
        state.onClick = (event) => handleClick(instance, event);
        instance.addEventListener('click', state.onClick);
    },
    beforeRender(instance, context) {
        const direction = normalizedDirection(instance.getAttribute('direction'));
        const label = instance.getAttribute('label')?.trim() || 'Column';
        context.setSlices(
            {
                actionLabel: `Sort by ${label}`,
                direction,
                indicator: indicatorFor(direction),
            },
            { render: false },
        );
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onClick) instance.removeEventListener('click', state.onClick);
    },
};

function stateFor(instance: HTMLElement): SortHeaderState {
    let state = SORT_HEADER_STATES.get(instance);
    if (state) return state;
    state = { connected: false };
    SORT_HEADER_STATES.set(instance, state);
    return state;
}

function normalizedDirection(authored: string | null): CemSortDirection {
    return authored === 'ascending' || authored === 'descending' ? authored : 'none';
}

function nextDirection(direction: CemSortDirection): CemSortDirection {
    if (direction === 'none') return 'ascending';
    if (direction === 'ascending') return 'descending';
    return 'none';
}

function indicatorFor(direction: CemSortDirection): string {
    if (direction === 'ascending') return '▲';
    if (direction === 'descending') return '▼';
    return '◇';
}

function handleClick(instance: HTMLElement, event: Event): void {
    if (event.defaultPrevented) return;
    const target = event.target instanceof Element ? event.target : null;
    const button = target?.closest<HTMLButtonElement>('button.cem-sort-header__button') ?? null;
    if (!button || button.disabled || !isDirectButton(instance, button)) return;

    const previousDirection = normalizedDirection(instance.getAttribute('direction'));
    const direction = nextDirection(previousDirection);
    if (direction !== 'none') clearTablePeers(instance);
    if (direction === 'none') instance.removeAttribute('direction');
    else instance.setAttribute('direction', direction);

    const detail: CemSortDetail = {
        direction,
        name: instance.getAttribute('name') ?? '',
        previousDirection,
    };
    instance.dispatchEvent(new CustomEvent<CemSortDetail>('cem-sort', {
        bubbles: true,
        composed: true,
        detail,
    }));
}

function isDirectButton(instance: HTMLElement, button: HTMLButtonElement): boolean {
    const owner = button.parentElement;
    return owner?.classList.contains('cem-sort-header') === true && owner.parentElement === instance;
}

function clearTablePeers(instance: HTMLElement): void {
    const table = instance.closest('cem-table');
    if (!table) return;
    for (const peer of table.querySelectorAll<HTMLElement>('cem-sort-header[direction]')) {
        if (peer !== instance && peer.closest('cem-table') === table) peer.removeAttribute('direction');
    }
}
