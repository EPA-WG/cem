import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import tooltipContractFixture from '../../tests/tooltip/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('tooltip contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(() => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-tooltip-declaration' });
        const result = installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('keeps one named native trigger and a persistent supplemental description as the exact owners', async () => {
        expect(tooltipContractFixture).not.toMatch(/<script\b/i);
        expect(tooltipContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const save = tooltipParts(root, '#save-tooltip');
        const link = tooltipParts(root, '#link-tooltip');
        const malformed = tooltipParts(root, '#malformed-tooltip', false);
        if (!save.trigger || !link.trigger) throw new Error('Expected valid tooltip fixtures to have triggers');

        expect(save.owner.dataset.mode).toBe('valid');
        expect(save.owner.dataset.position).toBe('above');
        expect(save.trigger).toBe(requiredElement(root, '#save-trigger'));
        expect(save.trigger.parentElement).toBe(save.owner);
        expect(assertAccessibleName(save.trigger, 'Save')).toBe('Save');
        expect(save.trigger.getAttribute('aria-describedby')?.split(/\s+/)).toEqual([
            'save-help',
            save.description.id,
        ]);
        expect(save.description.textContent).toBe('Save the current document');
        expect(save.surface.textContent).toBe(save.description.textContent);
        expect(save.surface.getAttribute('role')).toBe('tooltip');
        expect(save.surface.getAttribute('popover')).toBe('manual');
        expect(save.surface.querySelector('a, button, input, select, textarea, [tabindex]')).toBeNull();
        expect(save.surface.matches(':popover-open')).toBe(false);

        expect(link.trigger.localName).toBe('a');
        expect(link.owner.dataset.position).toBe('after');
        expect(assertAccessibleName(link.trigger, 'Account')).toBe('Account');
        expect(malformed.owner.dataset.mode).toBe('invalid');
        expect(malformed.trigger).toBe(requiredElement(root, '#malformed-trigger'));
        expect(requiredElement(root, '#malformed-trigger').hasAttribute('aria-describedby')).toBe(false);
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
    });

    it('keeps pointer enter, tooltip hover continuity, and leave on the native trigger without mutation', async () => {
        const root = await renderFixture();
        const parts = tooltipParts(root, '#save-tooltip');
        const trigger = requiredTrigger(parts);
        const originalTrigger = trigger;
        const originalGeometry = geometry(trigger);
        const originalHostAttributes = attributes(parts.host);
        const originalTriggerAttributes = attributes(trigger);
        const originalOwnerHtml = parts.owner.innerHTML;
        const events: string[] = [];
        const mutations: MutationRecord[] = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'click', 'input', 'change', 'cem-tooltip-toggle']) {
            trigger.addEventListener(eventName, (event) => events.push(`${event.type}:${event.isTrusted}`));
        }
        const observer = new MutationObserver((records) => mutations.push(...records));
        observer.observe(parts.host, { attributes: true, childList: true, subtree: true });

        await userEvent.hover(trigger);
        await waitForPopover(parts.surface, true);
        expect(trigger.matches(':hover')).toBe(true);
        expect(events).toContain('pointerenter:true');
        expect(document.activeElement).not.toBe(parts.surface);
        expect(geometry(trigger)).toEqual(originalGeometry);

        await userEvent.hover(parts.surface);
        await nextRenderFrame();
        expect(parts.surface.matches(':popover-open')).toBe(true);
        expect(document.activeElement).not.toBe(parts.surface);
        expect(geometry(trigger)).toEqual(originalGeometry);

        await userEvent.unhover(parts.surface);
        await waitForPopover(parts.surface, false);
        observer.disconnect();
        expect(events).toContain('pointerleave:true');
        expect(events.filter((entry) => /^(click|input|change|cem-tooltip-toggle):/.test(entry))).toEqual([]);
        expect(mutations).toEqual([]);
        expect(requiredTrigger(tooltipParts(root, '#save-tooltip'))).toBe(originalTrigger);
        expect(attributes(parts.host)).toEqual(originalHostAttributes);
        expect(attributes(trigger)).toEqual(originalTriggerAttributes);
        expect(parts.owner.innerHTML).toBe(originalOwnerHtml);
        expect(geometry(trigger)).toEqual(originalGeometry);
    });

    it('composes focus and hover reasons while Escape and blur dismiss without moving focus', async () => {
        const root = await renderFixture();
        const parts = tooltipParts(root, '#save-tooltip');
        const trigger = requiredTrigger(parts);
        requiredElement<HTMLButtonElement>(root, '[data-tooltip-focus-start]').focus();

        await userEvent.keyboard('{Tab}');
        expect(document.activeElement).toBe(trigger);
        expect(trigger.matches(':focus-visible')).toBe(true);
        await waitForPopover(parts.surface, true);

        await userEvent.hover(trigger);
        expect(trigger.matches(':hover')).toBe(true);
        requiredElement<HTMLButtonElement>(root, '[data-tooltip-focus-end]').focus();
        expect(document.activeElement).toBe(requiredElement(root, '[data-tooltip-focus-end]'));
        expect(parts.surface.matches(':popover-open')).toBe(true);
        await userEvent.unhover(trigger);
        await waitForPopover(parts.surface, false);

        requiredElement<HTMLButtonElement>(root, '[data-tooltip-focus-start]').focus();
        await userEvent.keyboard('{Tab}');
        await waitForPopover(parts.surface, true);
        await userEvent.keyboard('{Escape}');
        expect(document.activeElement).toBe(trigger);
        await waitForPopover(parts.surface, false);
        await nextRenderFrame();
        expect(parts.surface.matches(':popover-open')).toBe(false);

        await userEvent.keyboard('{Tab}');
        requiredElement<HTMLButtonElement>(root, '[data-tooltip-focus-start]').focus();
        await userEvent.keyboard('{Tab}');
        await waitForPopover(parts.surface, true);
    });

    it('honors normalized delays and the declarative manual open API silently', async () => {
        const root = await renderFixture();
        const parts = tooltipParts(root, '#delayed-tooltip');
        const trigger = requiredTrigger(parts);
        const triggerIdentity = trigger;
        const componentEvents: string[] = [];
        parts.host.addEventListener('cem-tooltip-toggle', (event) => componentEvents.push(event.type));

        await userEvent.hover(trigger);
        expect(parts.surface.matches(':popover-open')).toBe(false);
        await waitForPopover(parts.surface, true);
        await userEvent.unhover(trigger);
        expect(parts.surface.matches(':popover-open')).toBe(true);
        await waitForPopover(parts.surface, false);

        parts.host.setAttribute('open', '');
        await waitForPopover(parts.surface, true);
        expect(document.activeElement).not.toBe(parts.surface);
        expect(requiredTrigger(tooltipParts(root, '#delayed-tooltip'))).toBe(triggerIdentity);
        parts.host.removeAttribute('open');
        await waitForPopover(parts.surface, false);

        parts.host.setAttribute('message', 'Updated delayed help');
        await waitForText(parts.description, 'Updated delayed help');
        expect(parts.surface.textContent).toBe('Updated delayed help');
        expect(requiredTrigger(tooltipParts(root, '#delayed-tooltip'))).toBe(triggerIdentity);
        expect(componentEvents).toEqual([]);
    });

    it('suppresses host and native-trigger disabled presentation without taking over the trigger', async () => {
        const root = await renderFixture();
        const disabled = tooltipParts(root, '#disabled-tooltip');
        const nativeDisabled = tooltipParts(root, '#native-disabled-tooltip');
        const disabledTrigger = requiredTrigger(disabled);
        const nativeDisabledTrigger = requiredTrigger(nativeDisabled) as HTMLButtonElement;

        expect(disabledTrigger.hasAttribute('disabled')).toBe(false);
        expect(disabledTrigger.hasAttribute('aria-describedby')).toBe(false);
        await userEvent.hover(disabledTrigger);
        await nextRenderFrame();
        expect(disabled.surface.matches(':popover-open')).toBe(false);

        expect(nativeDisabledTrigger.disabled).toBe(true);
        expect(nativeDisabledTrigger.hasAttribute('aria-describedby')).toBe(false);
        nativeDisabled.host.setAttribute('open', '');
        await nextRenderFrame();
        expect(nativeDisabled.surface.matches(':popover-open')).toBe(false);

        disabled.host.removeAttribute('disabled');
        disabled.host.setAttribute('open', '');
        await waitForPopover(disabled.surface, true);
        expect(disabledTrigger.getAttribute('aria-describedby')).toContain(disabled.description.id);
        disabled.host.setAttribute('disabled', '');
        await waitForPopover(disabled.surface, false);
        expect(disabled.host.hasAttribute('open')).toBe(true);
        expect(disabledTrigger.hasAttribute('aria-describedby')).toBe(false);
    });

    it('leaves touch pointer and native activation uncanceled and does not auto-present', async () => {
        const root = await renderFixture();
        const parts = tooltipParts(root, '#touch-tooltip');
        const trigger = requiredTrigger(parts);
        let clicks = 0;
        trigger.addEventListener('click', () => {
            clicks += 1;
        });

        const enter = new PointerEvent('pointerenter', { bubbles: false, cancelable: true, pointerType: 'touch' });
        const down = new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerType: 'touch' });
        const up = new PointerEvent('pointerup', { bubbles: true, cancelable: true, pointerType: 'touch' });
        expect(trigger.dispatchEvent(enter)).toBe(true);
        expect(trigger.dispatchEvent(down)).toBe(true);
        expect(trigger.dispatchEvent(up)).toBe(true);
        trigger.click();
        await nextRenderFrame();

        expect(clicks).toBe(1);
        expect(parts.surface.matches(':popover-open')).toBe(false);
        expect(getComputedStyle(trigger).touchAction).not.toBe('none');
        expect(getComputedStyle(trigger).userSelect).not.toBe('none');
    });

    it('uses logical CSS anchor placement in the top layer without changing trigger geometry', async () => {
        const root = await renderFixture();
        const above = tooltipParts(root, '#save-tooltip');
        const after = tooltipParts(root, '#link-tooltip');
        const aboveGeometry = geometry(requiredTrigger(above));
        const afterGeometry = geometry(requiredTrigger(after));

        above.host.setAttribute('open', '');
        await waitForPopover(above.surface, true);
        const aboveStyle = getComputedStyle(above.surface);
        expect(aboveStyle.position).toBe('fixed');
        expect(aboveStyle.positionAnchor).toBe('--_cem-tooltip-anchor');
        expect(aboveStyle.positionArea).toBe('block-end');
        expect(aboveStyle.positionTryOrder).toBe('most-height');
        expect(above.surface.matches(':popover-open')).toBe(true);
        expect(above.surface.getBoundingClientRect().top).toBeGreaterThanOrEqual(
            requiredTrigger(above).getBoundingClientRect().bottom,
        );

        after.host.setAttribute('open', '');
        await waitForPopover(after.surface, true);
        expect(['center end', 'inline-start']).toContain(getComputedStyle(after.surface).positionArea);
        expect(getComputedStyle(after.surface).positionTryOrder).toBe('most-width');
        expect(geometry(requiredTrigger(above))).toEqual(aboveGeometry);
        expect(geometry(requiredTrigger(after))).toEqual(afterGeometry);
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness();
        const root = await harness.render(tooltipContractFixture);
        await waitForSelector(root, '#save-tooltip > .cem-tooltip > [slot="trigger"]');
        return root;
    }
});

interface TooltipParts {
    description: HTMLElement;
    host: HTMLElement;
    owner: HTMLElement;
    surface: HTMLElement;
    trigger: HTMLElement | null;
}

function tooltipParts(root: ParentNode, selector: string, requireTrigger = true): TooltipParts {
    const host = requiredElement<HTMLElement>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-tooltip');
    const trigger = owner.querySelector<HTMLElement>(':scope > [slot="trigger"]');
    if (requireTrigger && !trigger) throw new Error(`Missing tooltip trigger: ${selector}`);
    return {
        description: requiredElement(owner, ':scope > .cem-tooltip__description'),
        host,
        owner,
        surface: requiredElement(owner, ':scope > .cem-tooltip__surface'),
        trigger,
    };
}

function requiredTrigger(parts: TooltipParts): HTMLElement {
    if (!parts.trigger) throw new Error('Missing required tooltip trigger');
    return parts.trigger;
}

function attributes(element: Element): Record<string, string> {
    return Object.fromEntries([...element.attributes].map((attribute) => [attribute.name, attribute.value]));
}

function geometry(element: Element): { height: number; width: number; x: number; y: number } {
    const rect = element.getBoundingClientRect();
    return { height: rect.height, width: rect.width, x: rect.x, y: rect.y };
}

function requiredElement<T extends Element>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Missing required fixture element: ${selector}`);
    return element;
}

async function waitForSelector(root: ParentNode, selector: string): Promise<void> {
    for (let attempt = 0; attempt < 80; attempt += 1) {
        if (root.querySelector(selector)) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for selector: ${selector}`);
}

async function waitForPopover(surface: HTMLElement, open: boolean): Promise<void> {
    for (let attempt = 0; attempt < 120; attempt += 1) {
        if (surface.matches(':popover-open') === open) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for tooltip popover open=${String(open)}`);
}

async function waitForText(element: HTMLElement, text: string): Promise<void> {
    for (let attempt = 0; attempt < 80; attempt += 1) {
        if (element.textContent === text) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for tooltip text: ${text}`);
}
