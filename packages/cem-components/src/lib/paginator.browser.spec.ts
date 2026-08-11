import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import paginatorContractFixture from '../../tests/paginator/contract.html?raw';
import type { CemPageDetail } from './paginator-behavior.js';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('paginator contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(() => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-paginator-declaration' });
        const result = installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('renders exact landmark, page-size, range, action, normalization, and optional-control semantics', async () => {
        expect(paginatorContractFixture).not.toMatch(/<script\b/i);
        expect(paginatorContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const records = paginatorParts(root, '#records-paginator');
        const empty = paginatorParts(root, '#empty-paginator');
        const invalid = paginatorParts(root, '#invalid-paginator');
        const compact = paginatorParts(root, '#compact-paginator');

        expect(records.owner).toBeInstanceOf(HTMLElement);
        expect(records.owner.tagName).toBe('NAV');
        expect(records.owner.parentElement).toBe(records.host);
        expect(assertAccessibleName(records.owner, 'Record pages')).toBe('Record pages');
        expect(records.select).toBeInstanceOf(HTMLSelectElement);
        expect(assertAccessibleName(requiredSelect(records), 'Items per page')).toBe('Items per page');
        expect([...requiredSelect(records).options].map((option) => option.value)).toEqual(['10', '25', '50', '100']);
        expect(requiredSelect(records).value).toBe('25');
        expect(records.range.textContent).toBe('26 – 50 of 120');
        expect(records.range.getAttribute('role')).toBe('status');
        expect(records.range.getAttribute('aria-live')).toBe('polite');
        expect(records.range.getAttribute('aria-atomic')).toBe('true');
        expect([...records.actions.keys()]).toEqual(['first', 'previous', 'next', 'last']);
        expectAction(records, 'first', 'First page', '«', false);
        expectAction(records, 'previous', 'Previous page', '‹', false);
        expectAction(records, 'next', 'Next page', '›', false);
        expectAction(records, 'last', 'Last page', '»', false);

        expect(empty.range.textContent).toBe('0 – 0 of 0');
        expect([...requiredSelect(empty).options].map((option) => option.value)).toEqual(['50']);
        expect([...empty.actions.keys()]).toEqual(['previous', 'next']);
        expectAction(empty, 'previous', 'Previous page', '‹', true);
        expectAction(empty, 'next', 'Next page', '›', true);

        expect(invalid.host.getAttribute('length')).toBe('not-a-number');
        expect(invalid.host.getAttribute('page-index')).toBe('-4');
        expect(invalid.host.getAttribute('page-size')).toBe('0');
        expect(invalid.range.textContent).toBe('0 – 0 of 0');
        expect([...requiredSelect(invalid).options].map((option) => option.value)).toEqual(['10', '20', '50']);
        expect(requiredSelect(invalid).value).toBe('50');

        expect(compact.select).toBeNull();
        expect(compact.owner.querySelector('.cem-paginator__page-size')).toBeNull();
        expect([...compact.actions.keys()]).toEqual(['previous', 'next']);
        expect(compact.range.textContent).toBe('1 – 10 of 30');
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('navigates once through pointer, Enter, and Space with focus-stable suppressed boundaries', async () => {
        const root = await renderFixture();
        const parts = paginatorParts(root, '#records-paginator');
        const details: CemPageDetail[] = [];
        const events: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-page']) {
            parts.host.addEventListener(eventName, (event) => {
                events.push(`${event.type}:${event.isTrusted}`);
                if (event instanceof CustomEvent && event.type === 'cem-page') details.push(event.detail);
            });
        }

        const next = requiredAction(parts, 'next');
        await userEvent.click(next);
        await waitForPage(parts.host, 2, 25, '51 – 75 of 120');
        expect(document.activeElement).toBe(next);

        const last = requiredAction(parts, 'last');
        last.focus();
        await userEvent.keyboard('{Enter}');
        await waitForPage(parts.host, 4, 25, '101 – 120 of 120');
        expect(document.activeElement).toBe(last);
        expect(last.getAttribute('aria-disabled')).toBe('true');
        expect(last.tabIndex).toBe(-1);

        const beforeBoundary = [...events];
        last.click();
        await nextRenderFrame();
        expect(events).toEqual(beforeBoundary);
        expect(parts.host.getAttribute('page-index')).toBe('4');

        const first = requiredAction(parts, 'first');
        first.focus();
        await userEvent.keyboard(' ');
        await waitForPage(parts.host, 0, 25, '1 – 25 of 120');
        expect(document.activeElement).toBe(first);
        expect(first.getAttribute('aria-disabled')).toBe('true');
        expect(first.tabIndex).toBe(-1);

        const previous = requiredAction(parts, 'previous');
        const beforePreviousBoundary = [...events];
        previous.click();
        await nextRenderFrame();
        expect(events).toEqual(beforePreviousBoundary);

        next.focus();
        await userEvent.keyboard('{Enter}');
        await waitForPage(parts.host, 1, 25, '26 – 50 of 120');

        expect(details).toEqual([
            { length: 120, name: 'records', pageIndex: 2, pageSize: 25, previousPageIndex: 1 },
            { length: 120, name: 'records', pageIndex: 4, pageSize: 25, previousPageIndex: 2 },
            { length: 120, name: 'records', pageIndex: 0, pageSize: 25, previousPageIndex: 4 },
            { length: 120, name: 'records', pageIndex: 1, pageSize: 25, previousPageIndex: 0 },
        ]);
        expect(events.filter((entry) => entry === 'cem-page:false')).toHaveLength(4);
        expect(events.filter((entry) => entry === 'click:true')).toHaveLength(4);
        expect(events.some((entry) => /input|change/.test(entry))).toBe(false);

        parts.host.setAttribute('page-size', 'bad');
        parts.host.setAttribute('page-index', '0');
        await waitForRenderedRange(parts.host, '1 – 50 of 120');
        await userEvent.click(next);
        await waitForRenderedRange(parts.host, '51 – 100 of 120');
        expect(parts.host.getAttribute('page-index')).toBe('1');
        expect(parts.host.getAttribute('page-size')).toBe('bad');
        expect(details.at(-1)).toEqual({
            length: 120,
            name: 'records',
            pageIndex: 1,
            pageSize: 50,
            previousPageIndex: 0,
        });
    });

    it('preserves the first visible item on native page-size change with one ordered component event', async () => {
        const root = await renderFixture();
        const parts = paginatorParts(root, '#records-paginator');
        const events: string[] = [];
        const details: CemPageDetail[] = [];
        for (const eventName of ['input', 'change', 'cem-page']) {
            parts.host.addEventListener(eventName, (event) => {
                events.push(`${event.type}:${event.isTrusted}`);
                if (event instanceof CustomEvent && event.type === 'cem-page') details.push(event.detail);
            });
        }

        await userEvent.selectOptions(requiredSelect(parts), '10');
        await waitForPage(parts.host, 2, 10, '21 – 30 of 120');
        expect(requiredSelect(parts).value).toBe('10');
        expect(events).toEqual(['input:false', 'cem-page:false', 'change:false']);
        expect(details).toEqual([
            { length: 120, name: 'records', pageIndex: 2, pageSize: 10, previousPageIndex: 1 },
        ]);

        const eventCount = events.length;
        await userEvent.selectOptions(requiredSelect(parts), '10');
        await nextRenderFrame();
        expect(events.slice(eventCount)).toEqual(['input:false', 'change:false']);
        expect(details).toHaveLength(1);
        expect(parts.host.getAttribute('page-index')).toBe('2');
        expect(parts.host.getAttribute('page-size')).toBe('10');
    });

    it('keeps global disabled and live programmatic control silent while retaining surviving owners', async () => {
        const root = await renderFixture();
        const disabled = paginatorParts(root, '#disabled-paginator');
        const records = paginatorParts(root, '#records-paginator');
        const empty = paginatorParts(root, '#empty-paginator');
        const events: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-page']) {
            disabled.host.addEventListener(eventName, (event) => events.push(event.type));
            records.host.addEventListener(eventName, (event) => events.push(event.type));
        }

        expect(requiredSelect(disabled).disabled).toBe(true);
        for (const button of disabled.actions.values()) expect(button.disabled).toBe(true);
        requiredAction(disabled, 'next').click();
        await nextRenderFrame();
        expect(events).toEqual([]);
        expect(disabled.host.getAttribute('page-index')).toBe('1');

        const boundaryTargetEvents: string[] = [];
        const emptyPrevious = requiredAction(empty, 'previous');
        emptyPrevious.addEventListener('click', () => boundaryTargetEvents.push('target'));
        empty.host.addEventListener('click', () => boundaryTargetEvents.push('host'));
        emptyPrevious.click();
        await nextRenderFrame();
        expect(boundaryTargetEvents).toEqual([]);

        const original = {
            next: requiredAction(records, 'next'),
            owner: records.owner,
            range: records.range,
            select: records.select,
        };
        records.host.setAttribute('page-index', '99');
        await waitForRenderedRange(records.host, '101 – 120 of 120');
        expect(records.host.getAttribute('page-index')).toBe('99');
        records.host.setAttribute('length', '30');
        await waitForRenderedRange(records.host, '26 – 30 of 30');
        records.host.setAttribute('page-size', 'bad');
        await waitForRenderedRange(records.host, '1 – 30 of 30');
        const updated = paginatorParts(root, '#records-paginator');
        expect(updated.owner).toBe(original.owner);
        expect(updated.range).toBe(original.range);
        expect(updated.select).toBe(original.select);
        expect(requiredAction(updated, 'next')).toBe(original.next);
        expect(records.host.getAttribute('page-size')).toBe('bad');
        expect(events).toEqual([]);
    });

    it('keeps hover, focus-visible, and active paint on native controls with stable transient geometry', async () => {
        const root = await renderFixture();
        const parts = paginatorParts(root, '#records-paginator');
        const next = requiredAction(parts, 'next');
        const disabled = paginatorParts(root, '#disabled-paginator');
        const pointerEvents: string[] = [];
        const stateEvents: string[] = [];
        await userEvent.hover(requiredElement(root, '[data-paginator-focus-start]'));
        next.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
        next.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
        for (const eventName of ['input', 'change', 'cem-page']) {
            parts.host.addEventListener(eventName, (event) => stateEvents.push(event.type));
        }
        const baselineHtml = parts.host.innerHTML;
        const baseline = captureActionState(next);
        const selectBaseline = captureSelectState(requiredSelect(parts));

        expect(baseline.backgroundColor).toBe(
            resolveTokenColor(next, '--cem-action-contextual-default-background'),
        );
        expect(baseline.color).toBe(resolveTokenColor(next, '--cem-action-contextual-default-text'));
        expect(baseline.height).toBeCloseTo(resolveTokenLength(next, '--cem-icon-button-size'), 4);
        expect(selectBaseline.borderColor).toBe(
            resolveTokenColor(requiredSelect(parts), '--cem-input-indicator-anchor-color'),
        );

        await userEvent.hover(next);
        await nextRenderFrame();
        const hovered = captureActionState(next);
        expect(hovered.backgroundColor).toBe(
            resolveTokenColor(next, '--cem-action-contextual-hover-background'),
        );
        expect(hovered.color).toBe(resolveTokenColor(next, '--cem-action-contextual-hover-text'));
        expectActionGeometry(hovered, baseline);

        requiredElement<HTMLButtonElement>(root, '[data-paginator-focus-start]').focus();
        await userEvent.tab();
        expect(document.activeElement).toBe(parts.select);
        expect(requiredSelect(parts).matches(':focus-visible')).toBe(true);
        const focusedSelect = captureSelectState(requiredSelect(parts));
        expect(focusedSelect.outlineWidth).toBe(resolveTokenLength(requiredSelect(parts), '--cem-stroke-focus'));
        expectSelectGeometry(focusedSelect, selectBaseline);

        await userEvent.tab();
        await userEvent.tab();
        await userEvent.tab();
        expect(document.activeElement).toBe(next);
        expect(next.matches(':focus-visible')).toBe(true);
        await userEvent.hover(next);
        const focusedHovered = captureActionState(next);
        expect(focusedHovered.outlineWidth).toBe(resolveTokenLength(next, '--cem-stroke-focus'));
        expect(focusedHovered.backgroundColor).toBe(hovered.backgroundColor);

        await userEvent.keyboard('[Space>]');
        await waitForPseudoClass(next, ':active');
        const active = captureActionState(next);
        expect(active.backgroundColor).toBe(
            resolveTokenColor(next, '--cem-action-contextual-active-background'),
        );
        expect(parts.host.getAttribute('page-index')).toBe('1');
        expect(parts.host.innerHTML).toBe(baselineHtml);
        expectActionGeometry(active, baseline);

        await userEvent.keyboard('[/Space]');
        await waitForPage(parts.host, 2, 25, '51 – 75 of 120');
        await userEvent.unhover(next);
        expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        expect(stateEvents).toEqual(['cem-page']);
        expect(captureActionState(requiredAction(disabled, 'next')).color).toBe(
            resolveTokenColor(requiredAction(disabled, 'next'), '--cem-action-contextual-disabled-text'),
        );
        expect(parts.host.getAttribute('style')).toBeNull();
        expect(parts.owner.getAttribute('style')).toBeNull();
    });

    it('updates only requested range until the application consumes cem-page for data rendering', async () => {
        const root = await renderFixture();
        const parts = paginatorParts(root, '#records-paginator');
        const result = requiredElement(root, '#records-result');

        await userEvent.click(requiredAction(parts, 'next'));
        await waitForPage(parts.host, 2, 25, '51 – 75 of 120');
        expect(result.textContent).toBe('Authored data remains external.');

        parts.host.addEventListener('cem-page', (event) => {
            const detail = (event as CustomEvent<CemPageDetail>).detail;
            result.textContent = `Application rendered page ${detail.pageIndex}.`;
        }, { once: true });
        await userEvent.click(requiredAction(parts, 'next'));
        await waitForPage(parts.host, 3, 25, '76 – 100 of 120');
        expect(result.textContent).toBe('Application rendered page 3.');
        expect(parts.owner.querySelectorAll('[role="status"][aria-live="polite"]')).toHaveLength(1);
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness();
        const root = await harness.render(paginatorContractFixture);
        await waitForSelector(root, '#compact-paginator > .cem-paginator > .cem-paginator__range-actions');
        return root;
    }
});

type PageAction = 'first' | 'previous' | 'next' | 'last';

interface PaginatorParts {
    actions: Map<string, HTMLButtonElement>;
    host: HTMLElement;
    owner: HTMLElement;
    range: HTMLElement;
    select: HTMLSelectElement | null;
}

function paginatorParts(root: ParentNode, selector: string): PaginatorParts {
    const host = requiredElement<HTMLElement>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-paginator');
    return {
        actions: new Map(
            [...owner.querySelectorAll<HTMLButtonElement>('.cem-paginator__action')].map((button) => [
                button.dataset.pageAction ?? '',
                button,
            ]),
        ),
        host,
        owner,
        range: requiredElement<HTMLElement>(owner, '.cem-paginator__range'),
        select: owner.querySelector<HTMLSelectElement>('.cem-paginator__page-size-control'),
    };
}

function requiredAction(parts: PaginatorParts, action: PageAction): HTMLButtonElement {
    const button = parts.actions.get(action);
    if (!button) throw new Error(`Missing paginator ${action} action`);
    return button;
}

function requiredSelect(parts: PaginatorParts): HTMLSelectElement {
    if (!parts.select) throw new Error('Missing paginator page-size control');
    return parts.select;
}

function expectAction(
    parts: PaginatorParts,
    action: PageAction,
    label: string,
    icon: string,
    unavailable: boolean,
): void {
    const button = requiredAction(parts, action);
    expect(button.type).toBe('button');
    expect(assertAccessibleName(button, label)).toBe(label);
    expect(button.querySelector('.cem-paginator__icon')?.textContent).toBe(icon);
    expect(button.querySelector('.cem-paginator__icon')?.getAttribute('aria-hidden')).toBe('true');
    expect(button.getAttribute('aria-disabled')).toBe(unavailable ? 'true' : null);
    expect(button.tabIndex).toBe(unavailable ? -1 : 0);
}

function captureActionState(button: HTMLButtonElement) {
    const rect = button.getBoundingClientRect();
    const iconRect = requiredElement(button, '.cem-paginator__icon').getBoundingClientRect();
    const style = getComputedStyle(button);
    return {
        backgroundColor: style.backgroundColor,
        color: style.color,
        height: rect.height,
        iconHeight: iconRect.height,
        iconWidth: iconRect.width,
        outlineWidth: pixels(style.outlineWidth),
        width: rect.width,
    };
}

function captureSelectState(select: HTMLSelectElement) {
    const rect = select.getBoundingClientRect();
    const style = getComputedStyle(select);
    return {
        borderColor: style.borderBlockEndColor,
        height: rect.height,
        outlineWidth: pixels(style.outlineWidth),
        width: rect.width,
    };
}

function expectActionGeometry(actual: ReturnType<typeof captureActionState>, expected: ReturnType<typeof captureActionState>): void {
    expect(actual.height).toBeCloseTo(expected.height, 4);
    expect(actual.width).toBeCloseTo(expected.width, 4);
    expect(actual.iconHeight).toBeCloseTo(expected.iconHeight, 4);
    expect(actual.iconWidth).toBeCloseTo(expected.iconWidth, 4);
}

function expectSelectGeometry(actual: ReturnType<typeof captureSelectState>, expected: ReturnType<typeof captureSelectState>): void {
    expect(actual.height).toBeCloseTo(expected.height, 4);
    expect(actual.width).toBeCloseTo(expected.width, 4);
}

function resolveTokenColor(owner: HTMLElement, token: string): string {
    const probe = owner.ownerDocument.createElement('span');
    probe.style.color = `var(${token})`;
    owner.append(probe);
    const value = getComputedStyle(probe).color;
    probe.remove();
    return value;
}

function resolveTokenLength(owner: HTMLElement, token: string): number {
    const probe = owner.ownerDocument.createElement('span');
    probe.style.display = 'block';
    probe.style.inlineSize = `var(${token})`;
    probe.style.position = 'absolute';
    owner.append(probe);
    const value = pixels(getComputedStyle(probe).inlineSize);
    probe.remove();
    return value;
}

function pixels(value: string): number {
    const parsed = Number.parseFloat(value);
    if (!Number.isFinite(parsed)) throw new Error(`Expected a resolved CSS length, received ${value}`);
    return parsed;
}

async function waitForPage(host: HTMLElement, pageIndex: number, pageSize: number, range: string): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        const owner = host.querySelector<HTMLElement>(':scope > .cem-paginator');
        const select = owner?.querySelector<HTMLSelectElement>('.cem-paginator__page-size-control');
        const currentRange = owner?.querySelector<HTMLElement>('.cem-paginator__range');
        if (
            host.getAttribute('page-index') === String(pageIndex)
            && host.getAttribute('page-size') === String(pageSize)
            && (!select || select.value === String(pageSize))
            && currentRange?.textContent === range
        ) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for page ${pageIndex}, size ${pageSize}, range ${range}`);
}

async function waitForRenderedRange(host: HTMLElement, range: string): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        if (host.querySelector('.cem-paginator__range')?.textContent === range) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for paginator range ${range}`);
}

async function waitForSelector(root: ParentNode, selector: string): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        if (root.querySelector(selector)) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for ${selector}`);
}

async function waitForPseudoClass(owner: Element, selector: string): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        if (owner.matches(selector)) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for ${selector}`);
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Missing required fixture element: ${selector}`);
    return element;
}
