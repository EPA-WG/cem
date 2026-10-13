import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, nextRenderFrame, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-stepper.xhtml?raw';
const meta = preview.meta({ component: 'cem-stepper', title: 'CEM Components/cem-stepper', loaders: [async () => { await loadCemDeclaration('cem-stepper', declarationSource); return {}; }] });
interface TestCemStepper extends HTMLElement {
    selectedIndex: number;
}

interface StepperParts {
    host: TestCemStepper;
    owner: HTMLElement;
    list: HTMLOListElement;
    headers: HTMLButtonElement[];
    items: HTMLLIElement[];
    panels: HTMLElement[];
}


export const NativeOwners = meta.story({
 name: "renders one labeled ordered workflow with exact native headers and persistent linked regions",
 render: () => stepperContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
        expect(stepperContractFixture).not.toMatch(/<script\b/i);
        expect(stepperContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-stepper')) await whenCemRendered(host);
        const parts = stepperParts(root, '#workflow-stepper');

        expect(parts.owner.parentElement).toBe(parts.host);
        expect(parts.owner.tagName).toBe('SECTION');
        expect(parts.owner).toHaveAccessibleName('Create account');
        expect(parts.owner.dataset.orientation).toBe('horizontal');
        expect(parts.list.parentElement).toBe(parts.owner);
        expect(parts.list.tagName).toBe('OL');
        expect(parts.headers).toHaveLength(4);
        expect(parts.panels).toHaveLength(4);
        expect(parts.host.querySelector('[role="tablist"], [role="tab"], [role="tabpanel"]')).toBeNull();

        for (const [index, header] of parts.headers.entries()) {
            expect(header).toBeInstanceOf(HTMLButtonElement);
            expect(header.type).toBe('button');
            expect(header.parentElement).toBe(parts.items[index]);
            expect(header.getAttribute('aria-controls')).toBe(parts.panels[index]?.id);
            expect(parts.panels[index]?.getAttribute('aria-labelledby')).toBe(header.id);
            expect(parts.panels[index]?.getAttribute('role')).toBe('region');
        }

        expect(parts.host.selectedIndex).toBe(1);
        expect(parts.headers[1]?.getAttribute('aria-current')).toBe('step');
        expect(parts.headers[1]?.hasAttribute('aria-selected')).toBe(false);
        expect(parts.panels[1]?.hidden).toBe(false);
        expect(parts.panels.filter((panel) => panel.hidden)).toHaveLength(3);
        expect(parts.headers[0]?.textContent).toContain('Complete');
        expect(parts.headers[1]?.textContent).toContain('Optional');
        expect(parts.headers[3]?.textContent).toContain('Error');
        expect(parts.headers[3]?.getAttribute('aria-invalid')).toBe('true');
        expect(parts.headers[2]?.disabled).toBe(true);
        expect(parts.headers.filter((header) => header.tabIndex === 0)).toEqual([parts.headers[1]]);
        expect(root.querySelectorAll('cem-stepper cem-step')).toHaveLength(0);
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
 },
});

export const NativeActivation = meta.story({
 name: "commits one pointer/native-button path while current and disabled activation stay silent",
 render: () => stepperContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-stepper')) await whenCemRendered(host);
        let parts = stepperParts(root, '#workflow-stepper');
        const accountInput = requiredElement<HTMLInputElement>(parts.host, '#account-name');
        const events: Array<{ type: string; trusted: boolean; detail?: unknown }> = [];
        for (const eventName of ['click', 'input', 'change', 'cem-step']) {
            parts.host.addEventListener(eventName, (event) => events.push({
                type: event.type,
                trusted: event.isTrusted,
                detail: event instanceof CustomEvent ? event.detail : undefined,
            }));
        }

        await userEvent.click(requiredItem(parts.headers, 0));
        await waitForSelection(parts.host, 0);
        parts = stepperParts(root, '#workflow-stepper');
        expect(parts.host.getAttribute('selected-index')).toBe('0');
        expect(parts.headers[0]?.getAttribute('aria-current')).toBe('step');
        expect(document.activeElement).toBe(parts.headers[0]);
        expect(events.filter((event) => event.type === 'cem-step')).toEqual([{
            type: 'cem-step',
            trusted: false,
            detail: { value: 'account', index: 0, previousIndex: 1 },
        }]);
        expect(events.filter((event) => event.type === 'click')).toHaveLength(1);

        accountInput.value = 'Retained';
        await userEvent.click(requiredItem(parts.headers, 3));
        await waitForSelection(parts.host, 3);
        parts = stepperParts(root, '#workflow-stepper');
        expect(requiredElement(parts.host, '#account-name')).toBe(accountInput);
        expect(accountInput.value).toBe('Retained');
        expect(events.filter((event) => event.type === 'cem-step')).toHaveLength(2);

        await userEvent.click(requiredItem(parts.headers, 3));
        parts.headers[2]?.click();
        await nextRenderFrame();
        expect(parts.host.selectedIndex).toBe(3);
        expect(events.filter((event) => event.type === 'cem-step')).toHaveLength(2);
        expect(events.some((event) => event.type === 'input' || event.type === 'change')).toBe(false);
 },
});

export const RovingFocus = meta.story({
 name: "moves roving focus by orientation, wraps, skips native-disabled steps, and activates with Enter",
 render: () => stepperContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-stepper')) await whenCemRendered(host);
        let horizontal = stepperParts(root, '#workflow-stepper');
        horizontal.headers[1]?.focus();
        await userEvent.keyboard('{ArrowRight}');
        await nextRenderFrame();
        horizontal = stepperParts(root, '#workflow-stepper');
        expect(document.activeElement).toBe(horizontal.headers[3]);
        expect(horizontal.host.selectedIndex).toBe(1);
        expect(horizontal.headers.filter((header) => header.tabIndex === 0)).toEqual([horizontal.headers[3]]);

        await userEvent.keyboard('{ArrowRight}');
        await nextRenderFrame();
        horizontal = stepperParts(root, '#workflow-stepper');
        expect(document.activeElement).toBe(horizontal.headers[0]);
        await userEvent.keyboard('{End}');
        await nextRenderFrame();
        horizontal = stepperParts(root, '#workflow-stepper');
        expect(document.activeElement).toBe(horizontal.headers[3]);
        await userEvent.keyboard('{Enter}');
        await waitForSelection(horizontal.host, 3);
        horizontal = stepperParts(root, '#workflow-stepper');
        expect(horizontal.headers[3]?.getAttribute('aria-current')).toBe('step');

        let vertical = stepperParts(root, '#vertical-stepper');
        vertical.headers[0]?.focus();
        await userEvent.keyboard('{ArrowRight}');
        expect(document.activeElement).toBe(vertical.headers[0]);
        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();
        vertical = stepperParts(root, '#vertical-stepper');
        expect(document.activeElement).toBe(vertical.headers[2]);
        expect(vertical.host.selectedIndex).toBe(0);
        await userEvent.keyboard(' ');
        await waitForSelection(vertical.host, 2);
        vertical = stepperParts(root, '#vertical-stepper');
        expect(vertical.headers[2]?.getAttribute('aria-current')).toBe('step');
 },
});

export const ApplicationWorkflowFacts = meta.story({
 name: "enforces authored linear, optional, invalid, and editable facts without inspecting panel controls",
 render: () => stepperContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-stepper')) await whenCemRendered(host);
        let parts = stepperParts(root, '#linear-stepper');
        const events: unknown[] = [];
        parts.host.addEventListener('cem-step', (event) => events.push((event as CustomEvent).detail));

        expect(parts.headers[1]?.getAttribute('aria-disabled')).toBe('true');
        parts.headers[0]?.focus();
        await userEvent.keyboard('{ArrowRight}');
        await nextRenderFrame();
        parts = stepperParts(root, '#linear-stepper');
        expect(document.activeElement).toBe(parts.headers[1]);
        await userEvent.keyboard('{Enter}');
        await nextRenderFrame();
        expect(parts.host.selectedIndex).toBe(0);
        expect(events).toEqual([]);

        const island = dataIsland(parts.host);
        const contact = requiredElement<HTMLElement>(island.content, 'cem-step[value="contact"]');
        const delivery = requiredElement<HTMLElement>(island.content, 'cem-step[value="delivery"]');
        contact.setAttribute('completed', '');
        await waitFor(() => stepperParts(root, '#linear-stepper').headers[1]?.getAttribute('aria-disabled') !== 'true');
        parts = stepperParts(root, '#linear-stepper');
        await userEvent.click(requiredItem(parts.headers, 2));
        await waitForSelection(parts.host, 2);
        expect(events).toEqual([{ value: 'payment', index: 2, previousIndex: 0 }]);

        delivery.setAttribute('invalid', '');
        parts.host.selectedIndex = 0;
        await waitForSelection(parts.host, 0);
        parts = stepperParts(root, '#linear-stepper');
        expect(parts.headers[2]?.getAttribute('aria-disabled')).toBe('true');
        parts.headers[2]?.click();
        expect(parts.host.selectedIndex).toBe(0);

        delivery.removeAttribute('invalid');
        parts.host.selectedIndex = 2;
        await waitForSelection(parts.host, 2);
        parts = stepperParts(root, '#linear-stepper');
        expect(parts.headers[0]?.getAttribute('aria-disabled')).toBe('true');
        parts.headers[0]?.click();
        expect(parts.host.selectedIndex).toBe(2);
        contact.setAttribute('editable', '');
        await waitFor(() => stepperParts(root, '#linear-stepper').headers[0]?.getAttribute('aria-disabled') !== 'true');
        parts = stepperParts(root, '#linear-stepper');
        await userEvent.click(requiredItem(parts.headers, 0));
        await waitForSelection(parts.host, 0);
        expect(events.at(-1)).toEqual({ value: 'contact', index: 0, previousIndex: 2 });
 },
});

export const ProgrammaticAndDisabled = meta.story({
 name: "keeps programmatic control silent, clamps selection, and rejects malformed or globally disabled owners",
 render: () => stepperContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-stepper')) await whenCemRendered(host);
        let workflow = stepperParts(root, '#workflow-stepper');
        const events: string[] = [];
        workflow.host.addEventListener('cem-step', () => events.push('cem-step'));

        workflow.host.selectedIndex = 99;
        await waitForSelection(workflow.host, 3);
        expect(workflow.host.getAttribute('selected-index')).toBe('3');
        workflow.host.setAttribute('selected-index', '-4');
        await waitForSelection(workflow.host, 0);
        workflow = stepperParts(root, '#workflow-stepper');
        expect(workflow.host.selectedIndex).toBe(0);
        expect(workflow.host.getAttribute('selected-index')).toBe('-4');
        expect(events).toEqual([]);

        const disabled = stepperParts(root, '#disabled-stepper');
        expect(disabled.host.selectedIndex).toBe(1);
        expect(disabled.headers.every((header) => header.disabled && header.tabIndex === -1)).toBe(true);
        expect(disabled.headers[1]?.getAttribute('aria-current')).toBe('step');
        expect(disabled.headers[1]?.matches(':hover')).toBe(false);
        disabled.headers[0]?.click();
        expect(disabled.host.selectedIndex).toBe(1);

        const malformedHost = requiredElement<TestCemStepper>(root, '#malformed-stepper');
        expect(malformedHost.querySelector('.cem-stepper--invalid')?.hasAttribute('hidden')).toBe(true);
        expect(malformedHost.querySelector('button, ol, [role="region"]')).toBeNull();
        expect(malformedHost.selectedIndex).toBe(0);
 },
});

export const StatePaintAndGeometry = meta.story({
 name: "keeps hover and focus-visible on the exact header with stable geometry and zero transient mutation",
 render: () => stepperContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-stepper')) await whenCemRendered(host);
        let parts = stepperParts(root, '#workflow-stepper');
        const header = requiredItem(parts.headers, 3);
        const wrapper = requiredItem(parts.items, 3);
        const pointerEvents: string[] = [];
        header.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
        header.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
        const baseline = transientSnapshot(parts.host, header, wrapper);
        const focusBaseline = transientSnapshot(
            parts.host,
            requiredItem(parts.headers, 1),
            requiredItem(parts.items, 1),
        );

        await userEvent.hover(header);
        await nextRenderFrame();
        expect(wrapper.matches(':hover')).toBe(true);
        expect(header.matches(':hover')).toBe(true);
        const hovered = transientSnapshot(parts.host, header, wrapper);
        expect(hovered.backgroundColor).toBe(resolveTokenColor(header, '--cem-navigation-item-hover-background'));
        expect(hovered.color).toBe(resolveTokenColor(header, '--cem-navigation-item-hover-text'));
        expectStableTransient(hovered, baseline);

        requiredElement<HTMLButtonElement>(root, '[data-stepper-focus-start]').focus();
        await userEvent.tab();
        parts = stepperParts(root, '#workflow-stepper');
        expect(document.activeElement).toBe(parts.headers[1]);
        expect(parts.headers[1]?.matches(':focus-visible')).toBe(true);
        expect(parts.headers[3]?.matches(':hover')).toBe(true);
        const focusedHeader = requiredItem(parts.headers, 1);
        const focused = transientSnapshot(parts.host, focusedHeader, requiredItem(parts.items, 1));
        expect(focused.outlineWidth).toBe(resolveTokenLength(focusedHeader, '--cem-stroke-focus'));
        expectStableTransient(focused, focusBaseline);

        await userEvent.keyboard('[Space>]');
        expect(parts.headers[1]?.matches(':active')).toBe(true);
        const active = transientSnapshot(
            parts.host,
            requiredItem(parts.headers, 1),
            requiredItem(parts.items, 1),
        );
        expect(active.backgroundColor).toBe(
            resolveTokenColor(requiredItem(parts.headers, 1), '--cem-navigation-item-current-active-background'),
        );
        expectStableTransient(active, focusBaseline);
        await userEvent.keyboard('[/Space]');

        await userEvent.unhover(requiredItem(parts.headers, 3));
        await nextRenderFrame();
        expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        expect(parts.host.selectedIndex).toBe(1);
        expect(parts.host.getAttribute('selected-index')).toBe('1');
 },
});

function stepperParts(root: ParentNode, selector: string): StepperParts {
    const host = requiredElement<TestCemStepper>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-stepper:not(.cem-stepper--invalid)');
    const list = requiredElement<HTMLOListElement>(owner, ':scope > .cem-stepper__steps');
    const items = Array.from(list.querySelectorAll<HTMLLIElement>(':scope > .cem-stepper__item'));
    const headers = items.map((item) => requiredElement<HTMLButtonElement>(item, ':scope > .cem-stepper__header'));
    const panelsOwner = requiredElement<HTMLElement>(owner, ':scope > .cem-stepper__panels');
    const panels = Array.from(panelsOwner.querySelectorAll<HTMLElement>(':scope > .cem-stepper__panel'));
    return { host, owner, list, headers, items, panels };
}

function dataIsland(host: HTMLElement): HTMLTemplateElement {
    return requiredElement<HTMLTemplateElement>(host, 'template[data-cem-island="instance"]');
}

async function waitForSelection(host: TestCemStepper, selected: number): Promise<void> {
    await waitFor(() => host.selectedIndex === selected && host.querySelector('[aria-current="step"]')?.getAttribute('data-step-index') === String(selected));
}

async function waitFor(predicate: () => boolean, label = 'condition'): Promise<void> {
    for (let attempt = 0; attempt < 40; attempt += 1) {
        if (predicate()) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for ${label}`);
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Expected element matching ${selector}`);
    return element;
}

function requiredItem<T>(items: readonly T[], index: number): T {
    const item = items[index];
    if (item === undefined) throw new Error(`Expected item at index ${index}`);
    return item;
}

function rectTuple(element: Element): [number, number, number, number] {
    const rect = element.getBoundingClientRect();
    return [rect.x, rect.y, rect.width, rect.height];
}

function transientSnapshot(host: TestCemStepper, header: HTMLButtonElement, wrapper: HTMLLIElement) {
    const style = getComputedStyle(header);
    return {
        selectedIndex: host.selectedIndex,
        hostAttributes: Array.from(host.attributes).map((attribute) => `${attribute.name}=${attribute.value}`).join('|'),
        headerRect: rectTuple(header),
        wrapperRect: rectTuple(wrapper),
        backgroundColor: style.backgroundColor,
        color: style.color,
        outlineWidth: Number.parseFloat(style.outlineWidth),
    };
}

function expectStableTransient(
    actual: ReturnType<typeof transientSnapshot>,
    expected: ReturnType<typeof transientSnapshot>,
): void {
    expect(actual.selectedIndex).toBe(expected.selectedIndex);
    expect(actual.hostAttributes).toBe(expected.hostAttributes);
    expect(actual.headerRect).toEqual(expected.headerRect);
    expect(actual.wrapperRect).toEqual(expected.wrapperRect);
}

function resolveTokenColor(owner: Element, token: string): string {
    const probe = document.createElement('span');
    probe.style.color = `var(${token})`;
    owner.append(probe);
    const value = getComputedStyle(probe).color;
    probe.remove();
    return value;
}

function resolveTokenLength(owner: Element, token: string): number {
    const probe = document.createElement('span');
    probe.style.inlineSize = `var(${token})`;
    owner.append(probe);
    const value = Number.parseFloat(getComputedStyle(probe).inlineSize);
    probe.remove();
    return value;
}

function assertAriaReferenceIntegrity(root: ParentNode): void {
    for (const node of root.querySelectorAll('[aria-controls], [aria-labelledby], [aria-describedby]')) {
        for (const name of ['aria-controls', 'aria-labelledby', 'aria-describedby']) {
            for (const id of (node.getAttribute(name) ?? '').split(/\s+/).filter(Boolean)) {
                if (!node.ownerDocument.getElementById(id)) throw new Error(`Missing ARIA reference ${id}`);
            }
        }
    }
}

const stepperContractFixture = "<section class=\"cem-theme-light\" data-stepper-contract-fixture aria-label=\"Stepper contract fixture\">\n  <button data-stepper-focus-start type=\"button\">Focus start</button>\n\n  <cem-stepper id=\"workflow-stepper\" label=\"Create account\" selected-index=\"1\">\n    <cem-step value=\"account\" label=\"Account\" completed editable>\n      <label>Account name <input id=\"account-name\" value=\"Ada\"></label>\n    </cem-step>\n    <cem-step value=\"shipping\" label=\"Shipping\" optional>\n      <button id=\"shipping-action\" type=\"button\">Use saved address</button>\n    </cem-step>\n    <cem-step value=\"review\" label=\"Review\" completed disabled>\n      <p id=\"review-copy\">Review details</p>\n    </cem-step>\n    <cem-step value=\"confirm\" label=\"Confirm\" invalid>\n      <p id=\"confirm-copy\">Resolve the error before submitting.</p>\n    </cem-step>\n  </cem-stepper>\n\n  <cem-stepper id=\"linear-stepper\" label=\"Linear checkout\" linear>\n    <cem-step value=\"contact\" label=\"Contact\">\n      <p>Contact details</p>\n    </cem-step>\n    <cem-step value=\"delivery\" label=\"Delivery\" optional>\n      <p>Delivery details</p>\n    </cem-step>\n    <cem-step value=\"payment\" label=\"Payment\">\n      <p>Payment details</p>\n    </cem-step>\n  </cem-stepper>\n\n  <cem-stepper id=\"vertical-stepper\" label=\"Vertical workflow\" orientation=\"vertical\">\n    <cem-step value=\"one\" label=\"One\"><p>One panel</p></cem-step>\n    <cem-step value=\"two\" label=\"Two\" disabled><p>Two panel</p></cem-step>\n    <cem-step value=\"three\" label=\"Three\"><p>Three panel</p></cem-step>\n  </cem-stepper>\n\n  <cem-stepper id=\"disabled-stepper\" label=\"Disabled workflow\" selected-index=\"1\" disabled>\n    <cem-step value=\"disabled-one\" label=\"Disabled one\"><p>First disabled panel</p></cem-step>\n    <cem-step value=\"disabled-two\" label=\"Disabled two\" completed><p>Second disabled panel</p></cem-step>\n  </cem-stepper>\n\n  <cem-stepper id=\"malformed-stepper\" label=\"Malformed workflow\">\n    <cem-step value=\"duplicate\" label=\"Duplicate one\"><p>One</p></cem-step>\n    <cem-step value=\"duplicate\" label=\"Duplicate two\"><p>Two</p></cem-step>\n  </cem-stepper>\n\n  <button data-stepper-focus-end type=\"button\">Focus end</button>\n</section>\n";
