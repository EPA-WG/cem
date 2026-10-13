import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemWorkflowDependencies, whenCemRendered, nextRenderFrame, assertAccessibleName, assertAriaReferenceIntegrity } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-paginator.xhtml?raw';
const meta = preview.meta({ component: 'cem-paginator', title: 'CEM Components/cem-paginator', loaders: [async () => { await loadCemDeclaration('cem-paginator', declarationSource); await loadCemWorkflowDependencies(); return {}; }] });
const paginatorContractFixture = "<section class=\"cem-theme-light\" data-paginator-contract-fixture aria-label=\"Paginator contract fixture\">\n  <button data-paginator-focus-start type=\"button\">Focus start</button>\n\n  <cem-paginator\n    id=\"records-paginator\"\n    name=\"records\"\n    label=\"Record pages\"\n    length=\"120\"\n    page-index=\"1\"\n    page-size=\"25\"\n    page-size-options=\"100 10 25 50 25 invalid 0\"\n    show-first-last\n  ></cem-paginator>\n\n  <div id=\"records-result\" aria-label=\"Rendered records\">Authored data remains external.</div>\n\n  <cem-paginator id=\"empty-paginator\" name=\"empty\" label=\"Empty pages\"></cem-paginator>\n\n  <cem-paginator\n    id=\"invalid-paginator\"\n    name=\"invalid\"\n    label=\"Invalid input pages\"\n    length=\"not-a-number\"\n    page-index=\"-4\"\n    page-size=\"0\"\n    page-size-options=\"20 10 20 nope 0.5\"\n    show-first-last\n  ></cem-paginator>\n\n  <cem-paginator\n    id=\"disabled-paginator\"\n    name=\"disabled\"\n    label=\"Disabled pages\"\n    length=\"90\"\n    page-index=\"1\"\n    page-size=\"30\"\n    page-size-options=\"15 30 45\"\n    show-first-last\n    disabled\n  ></cem-paginator>\n\n  <cem-paginator\n    id=\"compact-paginator\"\n    name=\"compact\"\n    label=\"Compact pages\"\n    length=\"30\"\n    page-size=\"10\"\n    hide-page-size\n  ></cem-paginator>\n\n  <button data-paginator-focus-end type=\"button\">Focus end</button>\n</section>\n";
import type { CemPageDetail } from '../../../../cem-elements/.storybook/preview.js';

export const NativeOwners = meta.story({
 name: "renders exact landmark, page-size, range, action, normalization, and optional-control semantics",
 render: () => paginatorContractFixture,
 parameters: { docs: { description: { story: "Browser-runner-only: trusted native input assertions." } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== "test") return;
        expect(paginatorContractFixture).not.toMatch(/<script\b/i);
        expect(paginatorContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});

export const NativeActivation = meta.story({
 name: "navigates once through pointer, Enter, and Space with focus-stable suppressed boundaries",
 render: () => paginatorContractFixture,
 parameters: { docs: { description: { story: "Browser-runner-only: trusted native input assertions." } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== "test") return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});

export const PageSizeChange = meta.story({
 name: "preserves the first visible item on native page-size change with one ordered component event",
 render: () => paginatorContractFixture,
 parameters: { docs: { description: { story: "Browser-runner-only: trusted native input assertions." } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== "test") return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});

export const AuthorControl = meta.story({
 name: "keeps global disabled and live programmatic control silent while retaining surviving owners",
 render: () => paginatorContractFixture,
 parameters: { docs: { description: { story: "Browser-runner-only: trusted native input assertions." } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== "test") return;
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});

export const TransientPaint = meta.story({
 name: "keeps hover, focus-visible, and active paint on native controls with stable transient geometry",
 render: () => paginatorContractFixture,
 parameters: { docs: { description: { story: "Browser-runner-only: trusted native input assertions." } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== "test") return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});

export const ApplicationDataOwnership = meta.story({
 name: "updates only requested range until the application consumes cem-page for data rendering",
 render: () => paginatorContractFixture,
 parameters: { docs: { description: { story: "Browser-runner-only: trusted native input assertions." } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== "test") return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});

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

export const InheritedThemeContract = meta.story({
 render: () => '<section><cem-paginator label="Audit" length="120" page-size="25"><p>Content</p></cem-paginator></section>',
 play: async ({ canvasElement }) => {
    const container = requiredElement<HTMLElement>(canvasElement, 'section');
    const host = requiredElement<HTMLElement>(container, 'cem-paginator'); await whenCemRendered(host);
    const owner = requiredElement<HTMLElement>(host, '[part=range]');
    for (const mode of ['light','dark','contrast-light','contrast-dark','native']) {
        container.className = `cem-theme-${mode}`;
        container.style.cssText = '--cem-typography-data-font-size: 19px; --cem-typography-data-line-height: 1.7; --cem-typography-data-font-variant-numeric: lining-nums proportional-nums; --cem-coupling-zone-min: 52px; --cem-table-row-height: 56px; --cem-control-height: 20px; --cem-icon-button-size: 24px; --cem-gap-related: 3px; --cem-gap-group: 2px; --cem-coupling-guard-min: 11px; --cem-action-contextual-default-background: rgb(11, 31, 51); --cem-action-contextual-default-text: rgb(241, 242, 243); --cem-palette-comfort: rgb(23, 33, 43); --cem-palette-comfort-text: rgb(231, 232, 233);';
        await nextRenderFrame();
        expect(getComputedStyle(owner).fontSize).toBe('19px');
        expect(getComputedStyle(owner).lineHeight).toBe('32.3px');
        expect(getComputedStyle(owner).fontVariantNumeric).toBe('lining-nums proportional-nums');
        expect(requiredElement(host, 'select').getBoundingClientRect().height).toBeGreaterThanOrEqual(52);
        expect(requiredElement(host, '[part=action]').getBoundingClientRect().height).toBeGreaterThanOrEqual(52);
        expect(requiredElement(host, '[part=action]').getBoundingClientRect().width).toBeGreaterThanOrEqual(52);
        expect(requiredElement(host, 'select').getBoundingClientRect().width).toBeGreaterThanOrEqual(52);
        expect(getComputedStyle(requiredElement<HTMLElement>(host, '[part=range-actions]')).gap).toBe('11px');
        expect(getComputedStyle(requiredElement<HTMLElement>(host, 'nav')).gap).toBe('11px');
        expect(getComputedStyle(requiredElement<HTMLElement>(host, 'nav')).backgroundColor).toBe('rgb(23, 33, 43)');
        expect(host.querySelector('[part=range]')).toBe(owner);
    }
 },
});
