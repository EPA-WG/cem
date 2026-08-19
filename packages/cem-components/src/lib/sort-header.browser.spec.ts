import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import sortHeaderContractFixture from '../../tests/sort-header/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import type { CemSortDetail } from './sort-header-behavior.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('sort header contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(async () => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-sort-header-declaration' });
        const result = await installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('renders exact column-header, native action, direction, and accessible-name ownership', async () => {
        expect(sortHeaderContractFixture).not.toMatch(/<script\b/i);
        expect(sortHeaderContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const name = sortParts(root, '#name-sort');
        const created = sortParts(root, '#created-sort');
        const disabled = sortParts(root, '#disabled-sort');
        const invalid = sortParts(root, '#invalid-sort');

        expect(requiredElement(root, '#records-table > .cem-table').getAttribute('role')).toBe('table');
        expect(name.owner.parentElement).toBe(name.host);
        expect(name.button.parentElement).toBe(name.owner);
        expect(name.owner.getAttribute('role')).toBe('columnheader');
        expect(name.button).toBeInstanceOf(HTMLButtonElement);
        expect(name.button.type).toBe('button');
        expect(name.host.tabIndex).toBe(-1);
        expect(name.owner.tabIndex).toBe(-1);
        expect(assertAccessibleName(name.button, 'Sort by Name')).toBe('Sort by Name');
        expect(name.label.textContent).toBe('Name');
        expect(name.indicator.textContent).toBe('◇');
        expect(name.indicator.getAttribute('aria-hidden')).toBe('true');
        expect(name.owner.hasAttribute('aria-sort')).toBe(false);

        expect(created.owner.getAttribute('aria-sort')).toBe('ascending');
        expect(created.indicator.textContent).toBe('▲');
        expect(disabled.button.disabled).toBe(true);
        expect(disabled.owner.hasAttribute('aria-disabled')).toBe(false);
        expect(invalid.host.getAttribute('direction')).toBe('sideways');
        expect(invalid.owner.hasAttribute('aria-sort')).toBe(false);
        expect(invalid.indicator.textContent).toBe('◇');
        expect(root.querySelectorAll('[role="status"][aria-live="polite"]')).toHaveLength(1);
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('cycles pointer, Enter, and Space once while coordinating only the nearest table', async () => {
        const root = await renderFixture();
        const name = sortParts(root, '#name-sort');
        const created = sortParts(root, '#created-sort');
        const other = sortParts(root, '#other-sort');
        const rows = () => [...requiredElement(root, '#records-table > .cem-table').querySelectorAll(':scope > [role="row"]')];
        const authoredRows = rows();
        const details: CemSortDetail[] = [];
        const eventNames: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-sort']) {
            name.host.addEventListener(eventName, (event) => {
                eventNames.push(`${event.type}:${event.isTrusted}`);
                if (event instanceof CustomEvent && event.type === 'cem-sort') details.push(event.detail);
            });
        }

        await userEvent.click(name.button);
        await waitForDirection(name.host, 'ascending');
        expectDirection(name, 'ascending', '▲');
        expect(created.host.hasAttribute('direction')).toBe(false);
        expect(created.owner.hasAttribute('aria-sort')).toBe(false);
        expect(other.host.getAttribute('direction')).toBe('descending');
        expect(other.owner.getAttribute('aria-sort')).toBe('descending');
        expect(rows()).toEqual(authoredRows);

        name.button.focus();
        await userEvent.keyboard('{Enter}');
        await waitForDirection(name.host, 'descending');
        expectDirection(name, 'descending', '▼');

        name.button.focus();
        await userEvent.keyboard(' ');
        await waitForDirection(name.host, 'none');
        expectDirection(name, 'none', '◇');
        expect(name.host.hasAttribute('direction')).toBe(false);

        expect(details).toEqual([
            { direction: 'ascending', name: 'name', previousDirection: 'none' },
            { direction: 'descending', name: 'name', previousDirection: 'ascending' },
            { direction: 'none', name: 'name', previousDirection: 'descending' },
        ]);
        expect(eventNames.filter((entry) => entry.startsWith('cem-sort:'))).toEqual([
            'cem-sort:false',
            'cem-sort:false',
            'cem-sort:false',
        ]);
        expect(eventNames.filter((entry) => /input|change/.test(entry))).toEqual([]);
        expect(eventNames.filter((entry) => entry === 'click:true')).toHaveLength(3);
    });

    it('suppresses disabled activation and keeps programmatic state silent with stable owners', async () => {
        const root = await renderFixture();
        const disabled = sortParts(root, '#disabled-sort');
        const invalid = sortParts(root, '#invalid-sort');
        const original = { ...invalid };
        const events: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-sort']) {
            disabled.host.addEventListener(eventName, (event) => events.push(event.type));
            invalid.host.addEventListener(eventName, (event) => events.push(event.type));
        }

        disabled.button.focus();
        expect(document.activeElement).not.toBe(disabled.button);
        disabled.button.click();
        await nextRenderFrame();
        expect(disabled.host.hasAttribute('direction')).toBe(false);
        expect(events).toEqual([]);

        invalid.host.setAttribute('direction', 'descending');
        await waitForDirection(invalid.host, 'descending');
        const changed = sortParts(root, '#invalid-sort');
        expect(changed.owner).toBe(original.owner);
        expect(changed.button).toBe(original.button);
        expect(changed.label).toBe(original.label);
        expect(changed.indicator).toBe(original.indicator);
        expectDirection(changed, 'descending', '▼');
        expect(events).toEqual([]);

        invalid.host.setAttribute('label', 'Updated');
        await waitForText(changed.label, 'Updated');
        expect(assertAccessibleName(changed.button, 'Sort by Updated')).toBe('Sort by Updated');
        expect(changed.owner).toBe(original.owner);
        expect(events).toEqual([]);

        await userEvent.click(changed.button);
        await waitForDirection(invalid.host, 'none');
        expect(events.filter((entry) => entry === 'cem-sort')).toEqual(['cem-sort']);
    });

    it('keeps pointer, hover, focus-visible, and active paint on the native button with stable geometry and state', async () => {
        const root = await renderFixture();
        const parts = sortParts(root, '#name-sort');
        const pointerEvents: string[] = [];
        const componentEvents: string[] = [];
        await userEvent.hover(requiredElement(root, '[data-sort-focus-start]'));
        parts.button.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
        parts.button.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
        for (const eventName of ['input', 'change', 'cem-sort']) {
            parts.host.addEventListener(eventName, (event) => componentEvents.push(event.type));
        }
        const baselineHtml = parts.host.innerHTML;
        const baseline = captureTransientState(parts);

        expect(baseline.backgroundColor).toBe(
            resolveTokenColor(parts.button, '--cem-action-contextual-default-background'),
        );
        expect(baseline.color).toBe(resolveTokenColor(parts.button, '--cem-action-contextual-default-text'));
        expect(baseline.height).toBeGreaterThanOrEqual(resolveTokenLength(parts.button, '--cem-table-row-height'));
        expect(baseline.indicatorWidth).toBeCloseTo(
            resolveTokenLength(parts.button, '--cem-icon-button-icon-size'),
            4,
        );

        await userEvent.hover(parts.button);
        await nextRenderFrame();
        const hovered = captureTransientState(parts);
        expect(hovered.backgroundColor).toBe(
            resolveTokenColor(parts.button, '--cem-action-contextual-hover-background'),
        );
        expect(hovered.color).toBe(resolveTokenColor(parts.button, '--cem-action-contextual-hover-text'));
        expectTransientGeometry(hovered, baseline);
        expect(parts.owner.matches(':hover')).toBe(true);

        requiredElement<HTMLButtonElement>(root, '[data-sort-focus-start]').focus();
        await userEvent.tab();
        expect(document.activeElement).toBe(parts.button);
        expect(parts.button.matches(':focus-visible')).toBe(true);
        const focusedHovered = captureTransientState(parts);
        expect(focusedHovered.outlineWidth).toBe(resolveTokenLength(parts.button, '--cem-stroke-focus'));
        expect(focusedHovered.backgroundColor).toBe(hovered.backgroundColor);
        expectTransientGeometry(focusedHovered, baseline);

        await userEvent.keyboard('[Space>]');
        await waitForPseudoClass(parts.button, ':active');
        const active = captureTransientState(parts);
        expect(active.backgroundColor).toBe(
            resolveTokenColor(parts.button, '--cem-action-contextual-active-background'),
        );
        expect(active.color).toBe(resolveTokenColor(parts.button, '--cem-action-contextual-active-text'));
        expect(active.ariaSort).toBeNull();
        expect(parts.host.hasAttribute('direction')).toBe(false);
        expect(parts.host.innerHTML).toBe(baselineHtml);
        expectTransientGeometry(active, baseline);

        await userEvent.keyboard('[/Space]');
        await waitForDirection(parts.host, 'ascending');
        await userEvent.unhover(parts.button);
        await nextRenderFrame();
        expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        expect(componentEvents).toEqual(['cem-sort']);
        expect(parts.host.getAttribute('style')).toBeNull();
        expect(parts.owner.getAttribute('style')).toBeNull();
    });

    it('leaves row reordering and polite result announcement to the application event consumer', async () => {
        const root = await renderFixture();
        const parts = sortParts(root, '#name-sort');
        const table = requiredElement(root, '#records-table > .cem-table');
        const rowA = requiredElement(table, '#row-a');
        const rowB = requiredElement(table, '#row-b');
        const status = requiredElement(root, '#sort-announcement');

        await userEvent.click(parts.button);
        await waitForDirection(parts.host, 'ascending');
        expect(rowA.nextElementSibling).toBe(rowB);
        expect(status.textContent).toBe('Rows retain authored order.');

        table.addEventListener('cem-sort', (event) => {
            const detail = (event as CustomEvent<CemSortDetail>).detail;
            table.insertBefore(rowB, rowA);
            status.textContent = `${detail.name} sorted ${detail.direction}.`;
        }, { once: true });
        await userEvent.click(parts.button);
        await waitForDirection(parts.host, 'descending');
        expect(rowB.nextElementSibling).toBe(rowA);
        expect(status.textContent).toBe('name sorted descending.');
        expect(root.querySelectorAll('[role="status"], [aria-live]')).toHaveLength(1);
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness({ runtime });
        const root = await harness.render(sortHeaderContractFixture);
        await waitForSelector(root, '#invalid-sort > .cem-sort-header > .cem-sort-header__button');
        return root;
    }
});

interface SortParts {
    button: HTMLButtonElement;
    host: HTMLElement;
    indicator: HTMLElement;
    label: HTMLElement;
    owner: HTMLElement;
}

function sortParts(root: ParentNode, selector: string): SortParts {
    const host = requiredElement<HTMLElement>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-sort-header');
    return {
        button: requiredElement<HTMLButtonElement>(owner, ':scope > .cem-sort-header__button'),
        host,
        indicator: requiredElement<HTMLElement>(owner, '.cem-sort-header__indicator'),
        label: requiredElement<HTMLElement>(owner, '.cem-sort-header__label'),
        owner,
    };
}

function expectDirection(parts: SortParts, direction: 'none' | 'ascending' | 'descending', indicator: string): void {
    expect(parts.owner.getAttribute('aria-sort')).toBe(direction === 'none' ? null : direction);
    expect(parts.indicator.textContent).toBe(indicator);
}

function captureTransientState(parts: SortParts) {
    const rect = parts.button.getBoundingClientRect();
    const indicatorRect = parts.indicator.getBoundingClientRect();
    const style = getComputedStyle(parts.button);
    return {
        ariaSort: parts.owner.getAttribute('aria-sort'),
        backgroundColor: style.backgroundColor,
        color: style.color,
        height: rect.height,
        indicatorHeight: indicatorRect.height,
        indicatorWidth: indicatorRect.width,
        outlineWidth: pixels(style.outlineWidth),
        width: rect.width,
    };
}

function expectTransientGeometry(
    actual: ReturnType<typeof captureTransientState>,
    baseline: ReturnType<typeof captureTransientState>,
): void {
    expect(actual.height).toBeCloseTo(baseline.height, 4);
    expect(actual.width).toBeCloseTo(baseline.width, 4);
    expect(actual.indicatorHeight).toBeCloseTo(baseline.indicatorHeight, 4);
    expect(actual.indicatorWidth).toBeCloseTo(baseline.indicatorWidth, 4);
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

async function waitForDirection(host: HTMLElement, direction: 'none' | 'ascending' | 'descending'): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        const owner = host.querySelector<HTMLElement>(':scope > .cem-sort-header');
        const indicator = host.querySelector<HTMLElement>('.cem-sort-header__indicator');
        const rendered = owner?.getAttribute('aria-sort') ?? 'none';
        const expectedIndicator = direction === 'ascending' ? '▲' : direction === 'descending' ? '▼' : '◇';
        if (rendered === direction && indicator?.textContent === expectedIndicator) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for sort direction ${direction}`);
}

async function waitForText(owner: Element, expected: string): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        if (owner.textContent === expected) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for text ${expected}`);
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
