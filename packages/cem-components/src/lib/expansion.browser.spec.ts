import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import expansionContractFixture from '../../tests/expansion/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('expansion contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(async () => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-expansion-declaration' });
        const result = await installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('exposes one exact heading, button, and persistent controlled-panel relationship', async () => {
        expect(expansionContractFixture).not.toMatch(/<script\b/i);
        expect(expansionContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const account = expansionParts(root, '#account-expansion');
        const open = expansionParts(root, '#open-expansion');
        const fallback = expansionParts(root, '#fallback-expansion');

        expect(account.surface.parentElement).toBe(account.host);
        expect(account.heading.parentElement).toBe(account.surface);
        expect(account.heading.children).toHaveLength(1);
        expect(account.heading.firstElementChild).toBe(account.button);
        expect(account.heading.getAttribute('role')).toBe('heading');
        expect(account.heading.getAttribute('aria-level')).toBe('2');
        expect(account.button).toBeInstanceOf(HTMLButtonElement);
        expect(account.button.type).toBe('button');
        expect(account.button.disabled).toBe(false);
        expect(assertAccessibleName(account.button, 'Account details')).toBe('Account details');
        expect(account.button.getAttribute('aria-labelledby')).toBe(
            requiredElement(account.button, '.cem-expansion__summary').id,
        );
        expect(account.button.getAttribute('aria-expanded')).toBe('false');
        expect(account.button.getAttribute('aria-controls')).toBe(account.panel.id);
        expect(account.panel.getAttribute('aria-labelledby')).toBe(account.button.id);
        expect(account.panel.getAttribute('role')).toBe('region');
        expect(account.panel.hidden).toBe(true);
        expect(account.host.querySelector('#account-content')).not.toBeNull();

        expect(open.host.hasAttribute('expanded')).toBe(true);
        expect(open.button.getAttribute('aria-expanded')).toBe('true');
        expect(open.panel.hidden).toBe(false);
        expect(open.panel.hasAttribute('role')).toBe(false);
        expect(assertAccessibleName(open.button, 'Open summary')).toBe('Open summary');
        expect(open.button.querySelector('strong')?.textContent).toBe('summary');
        expect(fallback.heading.getAttribute('aria-level')).toBe('3');
        expect(root.querySelector('details, summary, nav, [role="tablist"]')).toBeNull();
        expect(new Set([account.button.id, account.panel.id, open.button.id, open.panel.id]).size).toBe(4);
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('uses one native click path for pointer, Enter, and Space while preserving owner identity', async () => {
        const root = await renderFixture();
        const parts = expansionParts(root, '#account-expansion');
        const content = requiredElement<HTMLElement>(parts.panel, '#account-content');
        const events: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-toggle', 'cem-expansion-toggle']) {
            parts.host.addEventListener(eventName, (event) => events.push(`${event.type}:${event.isTrusted}`));
        }

        await userEvent.click(parts.button);
        await waitForExpanded(parts.host, true);
        expectExpansionState(parts, true);
        expect(parts.host.querySelector('.cem-expansion__header')).toBe(parts.button);
        expect(parts.host.querySelector('.cem-expansion__panel')).toBe(parts.panel);
        expect(parts.host.querySelector('#account-content')).toBe(content);
        expect(document.activeElement).toBe(parts.button);
        expect(events).toEqual(['click:true']);

        await userEvent.tab();
        expect(document.activeElement).toBe(requiredElement(parts.panel, '#account-link'));
        parts.button.focus();
        await userEvent.keyboard('{Enter}');
        await waitForExpanded(parts.host, false);
        expectExpansionState(parts, false);
        expect(events).toEqual(['click:true', 'click:true']);

        parts.button.focus();
        await userEvent.keyboard(' ');
        await waitForExpanded(parts.host, true);
        expectExpansionState(parts, true);
        expect(events).toEqual(['click:true', 'click:true', 'click:true']);
        expect(events.some((entry) => /input|change|toggle/.test(entry))).toBe(false);
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('keeps disabled activation suppressed while allowing silent programmatic state control', async () => {
        const root = await renderFixture();
        const disabled = expansionParts(root, '#disabled-expansion');
        const disabledOpen = expansionParts(root, '#disabled-open-expansion');
        const events: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-toggle', 'cem-expansion-toggle']) {
            disabled.host.addEventListener(eventName, (event) => events.push(event.type));
        }

        expect(disabled.button.disabled).toBe(true);
        expectExpansionState(disabled, false);
        disabled.button.focus();
        expect(document.activeElement).not.toBe(disabled.button);
        disabled.button.click();
        await nextRenderFrame();
        expectExpansionState(disabled, false);
        expect(events).toEqual([]);

        expect(disabledOpen.button.disabled).toBe(true);
        expectExpansionState(disabledOpen, true);
        expect(requiredElement(disabledOpen.panel, '#disabled-open-action').tabIndex).toBe(0);

        disabled.host.setAttribute('expanded', '');
        await waitForExpanded(disabled.host, true);
        expectExpansionState(disabled, true);
        disabled.host.removeAttribute('expanded');
        await waitForExpanded(disabled.host, false);
        expectExpansionState(disabled, false);
        expect(events).toEqual([]);
    });

    it('keeps hover, focus-visible, and active paint on the header with stable transient geometry', async () => {
        const root = await renderFixture();
        const parts = expansionParts(root, '#account-expansion');
        const pointerEvents: string[] = [];
        await userEvent.hover(requiredElement(root, '[data-expansion-focus-start]'));
        await nextRenderFrame();
        parts.button.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
        parts.button.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
        const baseline = captureTransientState(parts);

        expect(baseline.backgroundColor).toBe(
            resolveTokenColor(parts.button, '--cem-action-contextual-default-background'),
        );
        expect(baseline.color).toBe(resolveTokenColor(parts.button, '--cem-action-contextual-default-text'));
        expect(baseline.height).toBeGreaterThanOrEqual(resolveTokenLength(parts.button, '--cem-coupling-zone-min'));

        await userEvent.hover(parts.button);
        await nextRenderFrame();
        const hovered = captureTransientState(parts);
        expect(hovered.backgroundColor).toBe(
            resolveTokenColor(parts.button, '--cem-action-contextual-hover-background'),
        );
        expect(hovered.color).toBe(resolveTokenColor(parts.button, '--cem-action-contextual-hover-text'));
        expectTransientGeometry(hovered, baseline);

        requiredElement<HTMLButtonElement>(root, '[data-expansion-focus-start]').focus();
        await userEvent.tab();
        expect(document.activeElement).toBe(parts.button);
        expect(parts.button.matches(':focus-visible')).toBe(true);
        const focused = captureTransientState(parts);
        expect(focused.outlineWidth).toBe(resolveTokenLength(parts.button, '--cem-stroke-focus'));
        expectTransientGeometry(focused, baseline);

        await userEvent.keyboard('[Space>]');
        await waitForPseudoClass(parts.button, ':active');
        const active = captureTransientState(parts);
        expect(active.backgroundColor).toBe(
            resolveTokenColor(parts.button, '--cem-action-contextual-active-background'),
        );
        expect(active.color).toBe(resolveTokenColor(parts.button, '--cem-action-contextual-active-text'));
        expect(active.ariaExpanded).toBe('false');
        expectTransientGeometry(active, baseline);

        await userEvent.keyboard('[/Space]');
        await waitForExpanded(parts.host, true);
        const expanded = captureTransientState(parts);
        expect(expanded.ariaExpanded).toBe('true');
        expect(expanded.outlineWidth).toBe(focused.outlineWidth);
        expectTransientGeometry(expanded, baseline);

        await userEvent.unhover(parts.button);
        await nextRenderFrame();
        expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        expect(parts.heading.matches(':hover')).toBe(false);
        expect(parts.surface.getAttribute('style')).toBeNull();
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness();
        const root = await harness.render(expansionContractFixture);
        await waitForSelector(root, '#fallback-expansion > .cem-expansion .cem-expansion__panel');
        return root;
    }
});

interface ExpansionParts {
    button: HTMLButtonElement;
    heading: HTMLElement;
    host: HTMLElement;
    panel: HTMLElement;
    surface: HTMLElement;
}

function expansionParts(root: ParentNode, selector: string): ExpansionParts {
    const host = requiredElement<HTMLElement>(root, selector);
    return {
        button: requiredElement<HTMLButtonElement>(host, ':scope > .cem-expansion > .cem-expansion__heading > .cem-expansion__header'),
        heading: requiredElement<HTMLElement>(host, ':scope > .cem-expansion > .cem-expansion__heading'),
        host,
        panel: requiredElement<HTMLElement>(host, ':scope > .cem-expansion > .cem-expansion__panel'),
        surface: requiredElement<HTMLElement>(host, ':scope > .cem-expansion'),
    };
}

function expectExpansionState(parts: ExpansionParts, expanded: boolean): void {
    expect(parts.host.hasAttribute('expanded')).toBe(expanded);
    expect(parts.button.getAttribute('aria-expanded')).toBe(String(expanded));
    expect(parts.panel.hidden).toBe(!expanded);
    expect(parts.button.getAttribute('aria-controls')).toBe(parts.panel.id);
    expect(parts.panel.getAttribute('aria-labelledby')).toBe(parts.button.id);
}

function captureTransientState(parts: ExpansionParts) {
    const rect = parts.button.getBoundingClientRect();
    const style = getComputedStyle(parts.button);
    return {
        ariaExpanded: parts.button.getAttribute('aria-expanded'),
        backgroundColor: style.backgroundColor,
        color: style.color,
        height: rect.height,
        outlineOffset: style.outlineOffset,
        outlineWidth: pixels(style.outlineWidth),
        width: rect.width,
    };
}

function expectTransientGeometry(actual: ReturnType<typeof captureTransientState>, baseline: ReturnType<typeof captureTransientState>): void {
    expect(actual.height).toBeCloseTo(baseline.height, 4);
    expect(actual.width).toBeCloseTo(baseline.width, 4);
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

async function waitForExpanded(host: HTMLElement, expanded: boolean): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        const button = host.querySelector<HTMLButtonElement>('.cem-expansion__header');
        const panel = host.querySelector<HTMLElement>('.cem-expansion__panel');
        if (
            host.hasAttribute('expanded') === expanded
            && button?.getAttribute('aria-expanded') === String(expanded)
            && panel?.hidden === !expanded
        ) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for expansion state ${expanded}`);
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
