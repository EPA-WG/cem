import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';

const DEFAULT_PAGE_SIZE = 50;

export interface CemPageDetail {
    length: number;
    name: string;
    pageIndex: number;
    pageSize: number;
    previousPageIndex: number;
}

interface PaginatorState {
    connected: boolean;
    onChange?: EventListener;
    onClickCapture?: EventListener;
}

interface NormalizedPaginator {
    length: number;
    nextDisabled: boolean;
    pageCount: number;
    pageIndex: number;
    pageSize: number;
    previousDisabled: boolean;
}

type PageAction = 'first' | 'previous' | 'next' | 'last';

const PAGINATOR_STATES = new WeakMap<HTMLElement, PaginatorState>();

export const CEM_PAGINATOR_BEHAVIOR: CemProducedElementBehavior = {
    connected(instance) {
        const state = stateFor(instance);
        if (state.connected) return;
        state.connected = true;
        state.onClickCapture = (event) => handleClick(instance, event);
        state.onChange = (event) => handleChange(instance, event);
        instance.addEventListener('click', state.onClickCapture, true);
        instance.addEventListener('change', state.onChange);
    },
    beforeRender(instance, context) {
        const normalized = normalizePaginator(instance);
        const disabled = instance.hasAttribute('disabled');
        const ofLabel = labelValue(instance, 'of-label', 'of');
        const rangeStart = normalized.length === 0 ? 0 : normalized.pageIndex * normalized.pageSize + 1;
        const rangeEnd = Math.min((normalized.pageIndex + 1) * normalized.pageSize, normalized.length);
        context.setSlices(
            {
                firstPageLabel: labelValue(instance, 'first-page-label', 'First page'),
                itemsPerPageLabel: labelValue(instance, 'items-per-page-label', 'Items per page'),
                lastPageLabel: labelValue(instance, 'last-page-label', 'Last page'),
                length: normalized.length,
                nextDisabled: normalized.nextDisabled || disabled,
                nextPageLabel: labelValue(instance, 'next-page-label', 'Next page'),
                pageIndex: normalized.pageIndex,
                pageSize: normalized.pageSize,
                pageSizeOptions: normalizedPageSizeOptions(instance, normalized.pageSize).map((value) => ({
                    label: String(value),
                    selected: value === normalized.pageSize,
                    value,
                })),
                previousDisabled: normalized.previousDisabled || disabled,
                previousPageLabel: labelValue(instance, 'previous-page-label', 'Previous page'),
                rangeLabel: `${rangeStart} – ${rangeEnd} ${ofLabel} ${normalized.length}`,
            },
            { render: false },
        );
    },
    disconnected(instance) {
        const state = stateFor(instance);
        if (!state.connected) return;
        state.connected = false;
        if (state.onClickCapture) instance.removeEventListener('click', state.onClickCapture, true);
        if (state.onChange) instance.removeEventListener('change', state.onChange);
    },
};

function stateFor(instance: HTMLElement): PaginatorState {
    let state = PAGINATOR_STATES.get(instance);
    if (state) return state;
    state = { connected: false };
    PAGINATOR_STATES.set(instance, state);
    return state;
}

function labelValue(instance: HTMLElement, name: string, fallback: string): string {
    return instance.getAttribute(name)?.trim() || fallback;
}

function normalizePaginator(instance: HTMLElement): NormalizedPaginator {
    const length = nonNegativeInteger(instance.getAttribute('length'), 0);
    const pageSize = positiveInteger(instance.getAttribute('page-size'), DEFAULT_PAGE_SIZE);
    const pageCount = Math.ceil(length / pageSize);
    const maximumPageIndex = Math.max(0, pageCount - 1);
    const pageIndex = Math.min(nonNegativeInteger(instance.getAttribute('page-index'), 0), maximumPageIndex);
    return {
        length,
        nextDisabled: pageCount === 0 || pageIndex >= maximumPageIndex,
        pageCount,
        pageIndex,
        pageSize,
        previousDisabled: pageIndex === 0,
    };
}

function normalizedPageSizeOptions(instance: HTMLElement, pageSize: number): number[] {
    const options = (instance.getAttribute('page-size-options') ?? '')
        .trim()
        .split(/\s+/)
        .filter(Boolean)
        .map((value) => Number(value))
        .filter((value) => Number.isFinite(value) && value > 0)
        .map((value) => Math.floor(value))
        .filter((value) => value > 0);
    return [...new Set([...options, pageSize])].sort((first, second) => first - second);
}

function nonNegativeInteger(authored: string | null, fallback: number): number {
    const parsed = Number(authored ?? '');
    return Number.isFinite(parsed) && parsed >= 0 ? Math.floor(parsed) : fallback;
}

function positiveInteger(authored: string | null, fallback: number): number {
    const parsed = Number(authored ?? '');
    return Number.isFinite(parsed) && parsed > 0 ? Math.floor(parsed) : fallback;
}

function handleClick(instance: HTMLElement, event: Event): void {
    if (event.defaultPrevented) return;
    const target = event.target instanceof Element ? event.target : null;
    const button = target?.closest<HTMLButtonElement>('button.cem-paginator__action') ?? null;
    if (!button || !isDirectAction(instance, button)) return;

    const action = button.dataset.pageAction as PageAction | undefined;
    const normalized = normalizePaginator(instance);
    if (!action || instance.hasAttribute('disabled') || actionUnavailable(normalized, action)) {
        event.preventDefault();
        event.stopImmediatePropagation();
        return;
    }

    const pageIndex = pageIndexForAction(normalized, action);
    commitPage(instance, normalized, pageIndex, normalized.pageSize);
}

function handleChange(instance: HTMLElement, event: Event): void {
    const target = event.target;
    if (!(target instanceof HTMLSelectElement) || !isDirectPageSize(instance, target)) return;
    if (target.disabled || instance.hasAttribute('disabled')) return;

    const normalized = normalizePaginator(instance);
    const pageSize = positiveInteger(target.value, normalized.pageSize);
    if (pageSize === normalized.pageSize) return;
    const firstVisibleIndex = normalized.pageIndex * normalized.pageSize;
    const maximumPageIndex = Math.max(0, Math.ceil(normalized.length / pageSize) - 1);
    const pageIndex = Math.min(Math.floor(firstVisibleIndex / pageSize), maximumPageIndex);
    commitPage(instance, normalized, pageIndex, pageSize);
}

function isDirectAction(instance: HTMLElement, button: HTMLButtonElement): boolean {
    const actions = button.parentElement;
    const owner = actions?.parentElement;
    return (
        actions?.classList.contains('cem-paginator__range-actions') === true
        && owner?.classList.contains('cem-paginator') === true
        && owner.parentElement === instance
    );
}

function isDirectPageSize(instance: HTMLElement, select: HTMLSelectElement): boolean {
    const field = select.parentElement;
    const owner = field?.parentElement;
    return (
        field?.classList.contains('cem-paginator__page-size') === true
        && owner?.classList.contains('cem-paginator') === true
        && owner.parentElement === instance
    );
}

function actionUnavailable(normalized: NormalizedPaginator, action: PageAction): boolean {
    return action === 'first' || action === 'previous' ? normalized.previousDisabled : normalized.nextDisabled;
}

function pageIndexForAction(normalized: NormalizedPaginator, action: PageAction): number {
    if (action === 'first') return 0;
    if (action === 'previous') return normalized.pageIndex - 1;
    if (action === 'next') return normalized.pageIndex + 1;
    return Math.max(0, normalized.pageCount - 1);
}

function commitPage(
    instance: HTMLElement,
    previous: NormalizedPaginator,
    pageIndex: number,
    pageSize: number,
): void {
    if (pageSize !== previous.pageSize) instance.setAttribute('page-size', String(pageSize));
    instance.setAttribute('page-index', String(pageIndex));
    const detail: CemPageDetail = {
        length: previous.length,
        name: instance.getAttribute('name') ?? '',
        pageIndex,
        pageSize,
        previousPageIndex: previous.pageIndex,
    };
    instance.dispatchEvent(new CustomEvent<CemPageDetail>('cem-page', {
        bubbles: true,
        composed: true,
        detail,
    }));
}
