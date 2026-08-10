import '@epa-wg/cem-theme/styles.css';
import '../styles.css';

import { CemElementRuntime, type DataIslandSnapshot } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    assertFocusVisible,
    assertLightDomRendered,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

interface SerializedEventTarget {
    checked: boolean | null;
    name: string | null;
    tag: string;
    type: string | null;
    value: string | null;
}

interface SerializedEventPayload {
    bubbles: boolean;
    currentTarget: SerializedEventTarget | null;
    sliceValue: unknown;
    target: SerializedEventTarget | null;
    type: string;
}

type TestCemSelect = HTMLElement & {
    checkValidity(): boolean;
    disabled: boolean;
    form: HTMLFormElement | null;
    multiple: boolean;
    reportValidity(): boolean;
    required: boolean;
    selectedValues: string[];
    setSelectedValues(values: readonly string[]): void;
    size: number;
    type: 'select-multiple' | 'select-one';
    validationMessage: string;
    value: string;
    validity: ValidityState;
};

describe('CEM component primitive states and ARIA behavior', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(() => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-state-declaration' });
        const result = installCemComponentPrimitives(runtime);

        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('reflects action, loading, disabled, expanded, selected, and focus states on native controls', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-action loading="true" expanded="false">Sync</cem-action>
                <cem-icon-button name="settings" label="Open settings" disabled></cem-icon-button>
                <cem-menu-item expanded="true">Advanced options</cem-menu-item>
                <cem-tabs label="Sections">
                    <button type="button" role="tab" aria-selected="true">Current</button>
                    <button type="button" role="tab" aria-selected="false">Later</button>
                </cem-tabs>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-tabs [role="tablist"]');

        const actionHost = harness.query<HTMLElement>('cem-action');
        const action = harness.query<HTMLButtonElement>('cem-action button');
        const iconButton = harness.query<HTMLButtonElement>('cem-icon-button button');
        const menuItem = harness.query<HTMLButtonElement>('cem-menu-item button');
        const tabs = Array.from(harness.root.querySelectorAll<HTMLButtonElement>('cem-tabs [role="tab"]'));

        assertStateHostsRendered(harness.root, 'cem-action, cem-icon-button, cem-menu-item, cem-tabs');
        expect(action.getAttribute('aria-busy')).toBe('true');
        expect(action.getAttribute('aria-expanded')).toBe('false');
        expect(assertAccessibleName(action, 'Sync')).toBe('Sync');
        expect(iconButton.disabled).toBe(true);
        expect(assertAccessibleName(iconButton, 'Open settings')).toBe('Open settings');
        expect(menuItem.getAttribute('aria-expanded')).toBe('true');
        expect(assertAccessibleName(menuItem, 'Advanced options')).toBe('Advanced options');
        expect(tabs.map((tab) => tab.getAttribute('aria-selected')).join('|')).toBe('true|false');
        await assertFocusVisible(action);

        action.click();
        await nextRenderFrame();

        const payload = eventPayload(runtime.snapshotInstance(actionHost), 'pressed');
        expect(payload.type).toBe('click');
        expect(payload.sliceValue).toBe('click');
        expect(payload.target?.tag).toBe('button');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('applies shared native hover treatment without changing action geometry or semantics', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack class="cem-theme-light" gap="sm">
                <cem-action>Save changes</cem-action>
                <cem-icon-button name="settings" label="Open settings"></cem-icon-button>
                <cem-menu-item>Open menu</cem-menu-item>
                <cem-action disabled>Disabled save</cem-action>
                <cem-icon-button name="settings" label="Disabled settings" disabled></cem-icon-button>
                <cem-menu-item disabled>Disabled menu</cem-menu-item>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-menu-item[disabled] button');

        const actionCases = [
            {
                host: harness.query<HTMLElement>('cem-action:not([disabled])'),
                button: harness.query<HTMLButtonElement>('cem-action:not([disabled]) button'),
                name: 'Save changes',
                role: null,
                tokens: {
                    defaultBackground: '--cem-action-primary-default-background',
                    defaultText: '--cem-action-primary-default-text',
                    hoverBackground: '--cem-action-primary-hover-background',
                    hoverText: '--cem-action-primary-hover-text',
                },
            },
            {
                host: harness.query<HTMLElement>('cem-icon-button:not([disabled])'),
                button: harness.query<HTMLButtonElement>('cem-icon-button:not([disabled]) button'),
                name: 'Open settings',
                role: null,
                tokens: {
                    defaultBackground: '--cem-action-contextual-default-background',
                    defaultText: '--cem-action-contextual-default-text',
                    hoverBackground: '--cem-action-contextual-hover-background',
                    hoverText: '--cem-action-contextual-hover-text',
                },
            },
            {
                host: harness.query<HTMLElement>('cem-menu-item:not([disabled])'),
                button: harness.query<HTMLButtonElement>('cem-menu-item:not([disabled]) button'),
                name: 'Open menu',
                role: 'menuitem',
                tokens: {
                    defaultBackground: '--cem-action-contextual-default-background',
                    defaultText: '--cem-action-contextual-default-text',
                    hoverBackground: '--cem-action-contextual-hover-background',
                    hoverText: '--cem-action-contextual-hover-text',
                },
            },
        ] as const;
        const disabledCases = [
            {
                host: harness.query<HTMLElement>('cem-action[disabled]'),
                button: harness.query<HTMLButtonElement>('cem-action[disabled] button'),
                name: 'Disabled save',
                role: null,
                tokens: actionCases[0].tokens,
            },
            {
                host: harness.query<HTMLElement>('cem-icon-button[disabled]'),
                button: harness.query<HTMLButtonElement>('cem-icon-button[disabled] button'),
                name: 'Disabled settings',
                role: null,
                tokens: actionCases[1].tokens,
            },
            {
                host: harness.query<HTMLElement>('cem-menu-item[disabled]'),
                button: harness.query<HTMLButtonElement>('cem-menu-item[disabled] button'),
                name: 'Disabled menu',
                role: 'menuitem',
                tokens: actionCases[2].tokens,
            },
        ] as const;
        const activationEvents: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => activationEvents.push(eventName));
        }

        assertStateHostsRendered(harness.root, 'cem-action, cem-icon-button, cem-menu-item');

        for (const actionCase of actionCases) {
            const { button, host, name, role, tokens } = actionCase;
            expect(button.type).toBe('button');
            expect(button.disabled).toBe(false);
            expect(button.getAttribute('role')).toBe(role);
            expect(assertAccessibleName(button, name)).toBe(name);
            await assertFocusVisible(button);

            const baseline = captureActionState(runtime, host, button);
            expect(baseline.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(button, tokens.defaultText));

            await userEvent.hover(button);
            await nextRenderFrame();

            const hovered = captureActionState(runtime, host, button);
            expect(hovered.backgroundColor).toBe(resolveTokenColor(button, tokens.hoverBackground));
            expect(hovered.color).toBe(resolveTokenColor(button, tokens.hoverText));
            expect(hovered.backgroundColor).not.toBe(baseline.backgroundColor);
            expectActionStructureAndGeometry(hovered, baseline);
            expect(hovered.focusTreatment).toEqual(baseline.focusTreatment);
            expect(document.activeElement).toBe(button);

            await userEvent.unhover(button);
            await nextRenderFrame();

            const restored = captureActionState(runtime, host, button);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectActionStructureAndGeometry(restored, baseline);
            expect(restored.focusTreatment).toEqual(baseline.focusTreatment);
            expect(document.activeElement).toBe(button);
        }

        for (const actionCase of disabledCases) {
            const { button, host, name, role, tokens } = actionCase;
            expect(button.type).toBe('button');
            expect(button.disabled).toBe(true);
            expect(button.getAttribute('role')).toBe(role);
            expect(assertAccessibleName(button, name)).toBe(name);

            const focusOwner = document.activeElement;
            button.focus();
            expect(document.activeElement).toBe(focusOwner);

            const baseline = captureActionState(runtime, host, button);
            expect(baseline.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(button, tokens.defaultText));
            expect(baseline.backgroundColor).not.toBe(resolveTokenColor(button, tokens.hoverBackground));

            await userEvent.hover(button);
            await nextRenderFrame();

            const hovered = captureActionState(runtime, host, button);
            expect(hovered.backgroundColor).toBe(baseline.backgroundColor);
            expect(hovered.color).toBe(baseline.color);
            expectActionStructureAndGeometry(hovered, baseline);
            expect(document.activeElement).toBe(focusOwner);

            await userEvent.unhover(button);
            await nextRenderFrame();

            const restored = captureActionState(runtime, host, button);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectActionStructureAndGeometry(restored, baseline);
            expect(document.activeElement).toBe(focusOwner);
        }

        expect(activationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('styles only navigation hover owners without changing current selection or component state', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack class="cem-theme-light" gap="sm">
                <cem-nav label="Primary navigation">
                    <a href="#overview">Overview</a>
                    <a href="#current" aria-current="page">Current page</a>
                    <a href="#unavailable" aria-disabled="true">Unavailable link</a>
                    <button type="button" disabled>Unavailable action</button>
                </cem-nav>
                <cem-nav label="Workspace navigation" collapsible expanded>
                    <a href="#workspace">Workspace</a>
                </cem-nav>
                <cem-tabs label="Profile sections">
                    <button type="button" role="tab" aria-selected="false">Overview tab</button>
                    <button type="button" role="tab" aria-selected="true">Security tab</button>
                    <button type="button" role="tab" aria-selected="false" aria-disabled="true">
                        Billing tab
                    </button>
                    <button type="button" role="tab" aria-selected="false" disabled>Disabled tab</button>
                </cem-tabs>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-tabs button[role="tab"]:disabled');

        const primaryHost = harness.query<HTMLElement>('cem-nav[label="Primary navigation"]');
        const primaryWrapper = harness.query<HTMLElement>('cem-nav[label="Primary navigation"] > nav');
        const disclosureHost = harness.query<HTMLElement>('cem-nav[collapsible]');
        const disclosureWrapper = harness.query<HTMLElement>('cem-nav[collapsible] > nav');
        const tabsHost = harness.query<HTMLElement>('cem-tabs');
        const tabsWrapper = harness.query<HTMLElement>('cem-tabs > [role="tablist"]');
        const navigationCases = [
            {
                host: primaryHost,
                owner: harness.query<HTMLAnchorElement>('cem-nav[label="Primary navigation"] a[href="#overview"]'),
                state: null,
                tokens: {
                    defaultBackground: '--cem-navigation-item-default-background',
                    defaultText: '--cem-navigation-item-default-text',
                    hoverBackground: '--cem-navigation-item-hover-background',
                    hoverText: '--cem-navigation-item-hover-text',
                },
                wrapper: primaryWrapper,
            },
            {
                host: primaryHost,
                owner: harness.query<HTMLAnchorElement>('cem-nav[label="Primary navigation"] a[aria-current="page"]'),
                state: { attribute: 'aria-current', value: 'page' },
                tokens: {
                    defaultBackground: '--cem-navigation-item-current-background',
                    defaultText: '--cem-navigation-item-current-text',
                    hoverBackground: '--cem-navigation-item-current-hover-background',
                    hoverText: '--cem-navigation-item-current-hover-text',
                },
                wrapper: primaryWrapper,
            },
            {
                host: disclosureHost,
                owner: harness.query<HTMLButtonElement>('cem-nav[collapsible] > nav > .cem-nav__disclosure'),
                state: { attribute: 'aria-expanded', value: 'true' },
                tokens: {
                    defaultBackground: '--cem-navigation-item-default-background',
                    defaultText: '--cem-navigation-item-default-text',
                    hoverBackground: '--cem-navigation-item-hover-background',
                    hoverText: '--cem-navigation-item-hover-text',
                },
                wrapper: disclosureWrapper,
            },
            {
                host: disclosureHost,
                owner: harness.query<HTMLAnchorElement>('cem-nav[collapsible] > nav > .cem-nav__content > a[href]'),
                state: null,
                tokens: {
                    defaultBackground: '--cem-navigation-item-default-background',
                    defaultText: '--cem-navigation-item-default-text',
                    hoverBackground: '--cem-navigation-item-hover-background',
                    hoverText: '--cem-navigation-item-hover-text',
                },
                wrapper: disclosureWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('cem-tabs button[role="tab"][aria-selected="false"]'),
                state: { attribute: 'aria-selected', value: 'false' },
                tokens: {
                    defaultBackground: '--cem-navigation-item-default-background',
                    defaultText: '--cem-navigation-item-default-text',
                    hoverBackground: '--cem-navigation-item-hover-background',
                    hoverText: '--cem-navigation-item-hover-text',
                },
                wrapper: tabsWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('cem-tabs button[role="tab"][aria-selected="true"]'),
                state: { attribute: 'aria-selected', value: 'true' },
                tokens: {
                    defaultBackground: '--cem-navigation-item-current-background',
                    defaultText: '--cem-navigation-item-current-text',
                    hoverBackground: '--cem-navigation-item-current-hover-background',
                    hoverText: '--cem-navigation-item-current-hover-text',
                },
                wrapper: tabsWrapper,
            },
        ] as const;
        const disabledCases = [
            {
                host: primaryHost,
                owner: harness.query<HTMLAnchorElement>('cem-nav a[aria-disabled="true"]'),
                wrapper: primaryWrapper,
            },
            {
                host: primaryHost,
                owner: harness.query<HTMLButtonElement>('cem-nav button:disabled'),
                wrapper: primaryWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('cem-tabs button[aria-disabled="true"]'),
                wrapper: tabsWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('cem-tabs button:disabled'),
                wrapper: tabsWrapper,
            },
        ] as const;
        const mutationEvents: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        assertStateHostsRendered(harness.root, 'cem-nav, cem-tabs');

        for (const navigationCase of navigationCases) {
            const { host, owner, state, tokens, wrapper } = navigationCase;
            const pointerEvents: string[] = [];
            owner.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
            owner.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
            await assertFocusVisible(owner);

            const baseline = captureNavigationState(runtime, host, wrapper, owner);
            expect(baseline.backgroundColor).toBe(resolveTokenColor(owner, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(owner, tokens.defaultText));
            if (state) {
                expect(owner.getAttribute(state.attribute)).toBe(state.value);
            }

            await userEvent.hover(owner);
            await nextRenderFrame();

            const hovered = captureNavigationState(runtime, host, wrapper, owner);
            expect(owner.matches(':hover')).toBe(true);
            expect(hovered.backgroundColor).toBe(resolveTokenColor(owner, tokens.hoverBackground));
            expect(hovered.color).toBe(resolveTokenColor(owner, tokens.hoverText));
            expect(hovered.backgroundColor).not.toBe(baseline.backgroundColor);
            expect(contrastRatio(hovered.backgroundColor, hovered.color)).toBeGreaterThanOrEqual(4.5);
            expectNavigationStructureAndGeometry(hovered, baseline);
            expect(hovered.focusTreatment).toEqual(baseline.focusTreatment);
            expect(hovered.wrapperBackgroundColor).toBe(baseline.wrapperBackgroundColor);
            expect(document.activeElement).toBe(owner);
            if (state) {
                expect(owner.getAttribute(state.attribute)).toBe(state.value);
            }

            await userEvent.unhover(owner);
            await nextRenderFrame();

            const restored = captureNavigationState(runtime, host, wrapper, owner);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectNavigationStructureAndGeometry(restored, baseline);
            expect(restored.focusTreatment).toEqual(baseline.focusTreatment);
            expect(restored.wrapperBackgroundColor).toBe(baseline.wrapperBackgroundColor);
            expect(document.activeElement).toBe(owner);
            expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        }

        for (const disabledCase of disabledCases) {
            const { host, owner, wrapper } = disabledCase;
            const pointerEvents: string[] = [];
            owner.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
            owner.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
            const focusOwner = document.activeElement;
            const baseline = captureNavigationState(runtime, host, wrapper, owner);
            expect(baseline.backgroundColor).toBe(
                resolveTokenColor(owner, '--cem-navigation-item-disabled-background'),
            );
            expect(baseline.color).toBe(resolveTokenColor(owner, '--cem-navigation-item-disabled-text'));

            await userEvent.hover(owner);
            await nextRenderFrame();

            const hovered = captureNavigationState(runtime, host, wrapper, owner);
            expect(owner.matches(':hover')).toBe(true);
            expect(hovered.backgroundColor).toBe(baseline.backgroundColor);
            expect(hovered.color).toBe(baseline.color);
            expectNavigationStructureAndGeometry(hovered, baseline);
            expect(hovered.wrapperBackgroundColor).toBe(baseline.wrapperBackgroundColor);
            expect(document.activeElement).toBe(focusOwner);

            await userEvent.unhover(owner);
            await nextRenderFrame();

            const restored = captureNavigationState(runtime, host, wrapper, owner);
            expectNavigationStructureAndGeometry(restored, baseline);
            expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        }

        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('moves keyboard focus through navigation owners without changing selection or component state', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <section class="cem-theme-light">
                <button id="navigation-focus-start" type="button">Start navigation focus sequence</button>
                <cem-stack gap="sm">
                    <cem-nav label="Focus primary navigation">
                        <a id="focus-nav-default" href="#overview">Overview</a>
                        <a id="focus-nav-current" href="#current" aria-current="page">Current page</a>
                        <button id="focus-nav-disabled" type="button" disabled>Unavailable action</button>
                    </cem-nav>
                    <cem-nav label="Focus workspace navigation" collapsible expanded>
                        <a id="focus-nav-content" href="#workspace">Workspace</a>
                        <button id="focus-content-disabled" type="button" disabled>Unavailable workspace</button>
                    </cem-nav>
                    <cem-tabs label="Focus profile sections">
                        <button id="focus-tab-default" type="button" role="tab" aria-selected="false">
                            Overview tab
                        </button>
                        <button id="focus-tab-selected" type="button" role="tab" aria-selected="true">
                            Security tab
                        </button>
                        <button id="focus-tab-disabled" type="button" role="tab" aria-selected="false" disabled>
                            Disabled tab
                        </button>
                    </cem-tabs>
                </cem-stack>
                <button id="navigation-focus-end" type="button">End navigation focus sequence</button>
            </section>
        `);
        await waitForStateSelector(root, '#focus-tab-disabled:disabled');

        const start = harness.query<HTMLButtonElement>('#navigation-focus-start');
        const end = harness.query<HTMLButtonElement>('#navigation-focus-end');
        const primaryHost = harness.query<HTMLElement>('cem-nav[label="Focus primary navigation"]');
        const primaryWrapper = harness.query<HTMLElement>('cem-nav[label="Focus primary navigation"] > nav');
        const disclosureHost = harness.query<HTMLElement>('cem-nav[label="Focus workspace navigation"]');
        const disclosureWrapper = harness.query<HTMLElement>('cem-nav[label="Focus workspace navigation"] > nav');
        const tabsHost = harness.query<HTMLElement>('cem-tabs[label="Focus profile sections"]');
        const tabsWrapper = harness.query<HTMLElement>('cem-tabs[label="Focus profile sections"] > [role="tablist"]');
        const cases = [
            {
                host: primaryHost,
                owner: harness.query<HTMLAnchorElement>('#focus-nav-default'),
                state: null,
                tokens: {
                    defaultBackground: '--cem-navigation-item-default-background',
                    defaultText: '--cem-navigation-item-default-text',
                    hoverBackground: '--cem-navigation-item-hover-background',
                    hoverText: '--cem-navigation-item-hover-text',
                },
                wrapper: primaryWrapper,
            },
            {
                host: primaryHost,
                owner: harness.query<HTMLAnchorElement>('#focus-nav-current'),
                state: { attribute: 'aria-current', value: 'page' },
                tokens: {
                    defaultBackground: '--cem-navigation-item-current-background',
                    defaultText: '--cem-navigation-item-current-text',
                    hoverBackground: '--cem-navigation-item-current-hover-background',
                    hoverText: '--cem-navigation-item-current-hover-text',
                },
                wrapper: primaryWrapper,
            },
            {
                host: disclosureHost,
                owner: harness.query<HTMLButtonElement>(
                    'cem-nav[label="Focus workspace navigation"] > nav > .cem-nav__disclosure',
                ),
                state: { attribute: 'aria-expanded', value: 'true' },
                tokens: {
                    defaultBackground: '--cem-navigation-item-default-background',
                    defaultText: '--cem-navigation-item-default-text',
                    hoverBackground: '--cem-navigation-item-hover-background',
                    hoverText: '--cem-navigation-item-hover-text',
                },
                wrapper: disclosureWrapper,
            },
            {
                host: disclosureHost,
                owner: harness.query<HTMLAnchorElement>('#focus-nav-content'),
                state: null,
                tokens: {
                    defaultBackground: '--cem-navigation-item-default-background',
                    defaultText: '--cem-navigation-item-default-text',
                    hoverBackground: '--cem-navigation-item-hover-background',
                    hoverText: '--cem-navigation-item-hover-text',
                },
                wrapper: disclosureWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('#focus-tab-default'),
                state: { attribute: 'aria-selected', value: 'false' },
                tokens: {
                    defaultBackground: '--cem-navigation-item-default-background',
                    defaultText: '--cem-navigation-item-default-text',
                    hoverBackground: '--cem-navigation-item-hover-background',
                    hoverText: '--cem-navigation-item-hover-text',
                },
                wrapper: tabsWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('#focus-tab-selected'),
                state: { attribute: 'aria-selected', value: 'true' },
                tokens: {
                    defaultBackground: '--cem-navigation-item-current-background',
                    defaultText: '--cem-navigation-item-current-text',
                    hoverBackground: '--cem-navigation-item-current-hover-background',
                    hoverText: '--cem-navigation-item-current-hover-text',
                },
                wrapper: tabsWrapper,
            },
        ] as const;
        const disabled = [
            harness.query<HTMLButtonElement>('#focus-nav-disabled'),
            harness.query<HTMLButtonElement>('#focus-content-disabled'),
            harness.query<HTMLButtonElement>('#focus-tab-disabled'),
        ];
        const baselines = cases.map(({ host, owner, wrapper }) =>
            captureNavigationState(runtime, host, wrapper, owner),
        );
        const focusOrder: string[] = [];
        const mutationEvents: string[] = [];
        harness.root.addEventListener('focusin', (event) => {
            const target = event.target;
            if (target instanceof HTMLElement && cases.some(({ owner }) => owner === target)) {
                focusOrder.push(target.id || target.className);
            }
        });
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        assertStateHostsRendered(harness.root, 'cem-nav, cem-tabs');
        expect(disabled.every((owner) => owner.disabled)).toBe(true);
        start.focus();
        expect(document.activeElement).toBe(start);

        for (const [index, navigationCase] of cases.entries()) {
            await userEvent.tab();
            await nextRenderFrame();

            const { owner, state, tokens } = navigationCase;
            const focused = captureNavigationState(
                runtime,
                navigationCase.host,
                navigationCase.wrapper,
                owner,
            );
            expect(document.activeElement).toBe(owner);
            expect(owner.matches(':focus-visible')).toBe(true);
            expect(paintedColor(focused.focusTreatment[0])).toBe(resolveTokenColor(owner, '--cem-zebra-color-1'));
            expect(focused.focusTreatment[1]).toBe('solid');
            expect(focused.focusTreatment[2]).toBe(`${resolveTokenLength(owner, '--cem-stroke-focus')}px`);
            expect(focused.focusTreatment[3]).toBe(
                `${resolveTokenLength(owner, '--cem-stroke-indicator-offset')}px`,
            );
            expect(focused.backgroundColor).toBe(resolveTokenColor(owner, tokens.defaultBackground));
            expect(focused.color).toBe(resolveTokenColor(owner, tokens.defaultText));
            expect(focused.wrapperFocusTreatment).toEqual(baselines[index].wrapperFocusTreatment);
            expectNavigationStructureAndGeometry(focused, baselines[index]);
            if (state) {
                expect(owner.getAttribute(state.attribute)).toBe(state.value);
            }

            if (index > 0) {
                const previous = cases[index - 1];
                const restored = captureNavigationState(
                    runtime,
                    previous.host,
                    previous.wrapper,
                    previous.owner,
                );
                expect(previous.owner.matches(':focus-visible')).toBe(false);
                expect(restored.focusTreatment).toEqual(baselines[index - 1].focusTreatment);
                expect(restored.backgroundColor).toBe(
                    resolveTokenColor(previous.owner, previous.tokens.defaultBackground),
                );
                expect(restored.color).toBe(resolveTokenColor(previous.owner, previous.tokens.defaultText));
                expectNavigationStructureAndGeometry(restored, baselines[index - 1]);
            }

            await userEvent.hover(owner);
            await nextRenderFrame();
            const hoveredFocused = captureNavigationState(
                runtime,
                navigationCase.host,
                navigationCase.wrapper,
                owner,
            );
            expect(owner.matches(':hover')).toBe(true);
            expect(owner.matches(':focus-visible')).toBe(true);
            expect(hoveredFocused.backgroundColor).toBe(resolveTokenColor(owner, tokens.hoverBackground));
            expect(hoveredFocused.color).toBe(resolveTokenColor(owner, tokens.hoverText));
            expect(hoveredFocused.focusTreatment).toEqual(focused.focusTreatment);
            expectNavigationStructureAndGeometry(hoveredFocused, focused);
            if (state) {
                expect(owner.getAttribute(state.attribute)).toBe(state.value);
            }
            await userEvent.unhover(owner);
            await nextRenderFrame();
            const restoredFocus = captureNavigationState(
                runtime,
                navigationCase.host,
                navigationCase.wrapper,
                owner,
            );
            expect(restoredFocus.backgroundColor).toBe(focused.backgroundColor);
            expect(restoredFocus.color).toBe(focused.color);
            expect(restoredFocus.focusTreatment).toEqual(focused.focusTreatment);
            expectNavigationStructureAndGeometry(restoredFocus, focused);

            for (const disabledOwner of disabled) {
                expect(document.activeElement).not.toBe(disabledOwner);
                expect(disabledOwner.matches(':focus-visible')).toBe(false);
            }
        }

        const lastCase = cases.at(-1);
        const lastBaseline = baselines.at(-1);
        if (!lastCase || !lastBaseline) {
            throw new Error('Expected the final navigation focus case');
        }
        await userEvent.tab();
        await nextRenderFrame();
        expect(document.activeElement).toBe(end);
        expect(lastCase.owner.matches(':focus-visible')).toBe(false);
        const restoredLast = captureNavigationState(runtime, lastCase.host, lastCase.wrapper, lastCase.owner);
        expect(restoredLast.focusTreatment).toEqual(lastBaseline.focusTreatment);
        expectNavigationStructureAndGeometry(restoredLast, lastBaseline);

        expect(focusOrder).toEqual(cases.map(({ owner }) => owner.id || owner.className));
        expect(cases[1].owner.getAttribute('aria-current')).toBe('page');
        expect(cases[2].owner.getAttribute('aria-expanded')).toBe('true');
        expect(cases[4].owner.getAttribute('aria-selected')).toBe('false');
        expect(cases[5].owner.getAttribute('aria-selected')).toBe('true');
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('applies navigation active treatment during trusted pointer and native keyboard activation', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack class="cem-theme-light" gap="sm">
                <cem-nav label="Active primary navigation">
                    <a id="active-nav-default" href="#active-overview">Overview</a>
                    <a id="active-nav-current" href="#active-current" aria-current="page">Current page</a>
                    <a id="active-nav-aria-disabled" href="#active-unavailable" aria-disabled="true">
                        Unavailable link
                    </a>
                    <button id="active-nav-disabled" type="button" disabled>Unavailable action</button>
                </cem-nav>
                <cem-nav label="Active workspace navigation" collapsible expanded>
                    <a id="active-nav-content" href="#active-workspace">Workspace</a>
                </cem-nav>
                <cem-tabs label="Active profile sections">
                    <button id="active-tab-default" type="button" role="tab" aria-selected="false">
                        Overview tab
                    </button>
                    <button id="active-tab-selected" type="button" role="tab" aria-selected="true">
                        Security tab
                    </button>
                    <button
                        id="active-tab-aria-disabled"
                        type="button"
                        role="tab"
                        aria-selected="false"
                        aria-disabled="true"
                    >
                        Billing tab
                    </button>
                    <button
                        id="active-tab-disabled"
                        type="button"
                        role="tab"
                        aria-selected="false"
                        disabled
                    >
                        Disabled tab
                    </button>
                </cem-tabs>
            </cem-stack>
        `);
        await waitForStateSelector(root, '#active-tab-disabled:disabled');

        const primaryHost = harness.query<HTMLElement>('cem-nav[label="Active primary navigation"]');
        const primaryWrapper = harness.query<HTMLElement>('cem-nav[label="Active primary navigation"] > nav');
        const disclosureHost = harness.query<HTMLElement>('cem-nav[label="Active workspace navigation"]');
        const disclosureWrapper = harness.query<HTMLElement>('cem-nav[label="Active workspace navigation"] > nav');
        const tabsHost = harness.query<HTMLElement>('cem-tabs[label="Active profile sections"]');
        const tabsWrapper = harness.query<HTMLElement>(
            'cem-tabs[label="Active profile sections"] > [role="tablist"]',
        );
        const activeTokens = {
            activeBackground: '--cem-navigation-item-active-background',
            activeText: '--cem-navigation-item-active-text',
            defaultBackground: '--cem-navigation-item-default-background',
            defaultText: '--cem-navigation-item-default-text',
            hoverBackground: '--cem-navigation-item-hover-background',
            hoverText: '--cem-navigation-item-hover-text',
        } as const;
        const currentActiveTokens = {
            activeBackground: '--cem-navigation-item-current-active-background',
            activeText: '--cem-navigation-item-current-active-text',
            defaultBackground: '--cem-navigation-item-current-background',
            defaultText: '--cem-navigation-item-current-text',
            hoverBackground: '--cem-navigation-item-current-hover-background',
            hoverText: '--cem-navigation-item-current-hover-text',
        } as const;
        const disclosure = harness.query<HTMLButtonElement>(
            'cem-nav[label="Active workspace navigation"] > nav > .cem-nav__disclosure',
        );
        const activeCases = [
            {
                host: primaryHost,
                owner: harness.query<HTMLAnchorElement>('#active-nav-default'),
                state: null,
                tokens: activeTokens,
                wrapper: primaryWrapper,
            },
            {
                host: primaryHost,
                owner: harness.query<HTMLAnchorElement>('#active-nav-current'),
                state: { attribute: 'aria-current', value: 'page' },
                tokens: currentActiveTokens,
                wrapper: primaryWrapper,
            },
            {
                host: disclosureHost,
                owner: harness.query<HTMLAnchorElement>('#active-nav-content'),
                state: null,
                tokens: activeTokens,
                wrapper: disclosureWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('#active-tab-default'),
                state: { attribute: 'aria-selected', value: 'false' },
                tokens: activeTokens,
                wrapper: tabsWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('#active-tab-selected'),
                state: { attribute: 'aria-selected', value: 'true' },
                tokens: currentActiveTokens,
                wrapper: tabsWrapper,
            },
            {
                host: disclosureHost,
                owner: disclosure,
                state: { attribute: 'aria-expanded', value: 'true' },
                tokens: activeTokens,
                wrapper: disclosureWrapper,
            },
        ] as const;
        const disabledCases = [
            {
                host: primaryHost,
                owner: harness.query<HTMLAnchorElement>('#active-nav-aria-disabled'),
                wrapper: primaryWrapper,
            },
            {
                host: primaryHost,
                owner: harness.query<HTMLButtonElement>('#active-nav-disabled'),
                wrapper: primaryWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('#active-tab-aria-disabled'),
                wrapper: tabsWrapper,
            },
            {
                host: tabsHost,
                owner: harness.query<HTMLButtonElement>('#active-tab-disabled'),
                wrapper: tabsWrapper,
            },
        ] as const;
        const clickEvents: string[] = [];
        const mutationEvents: string[] = [];
        harness.root.addEventListener('click', (event) => {
            const target = event.target;
            if (target instanceof HTMLAnchorElement) {
                event.preventDefault();
            }
            if (target instanceof HTMLElement) {
                clickEvents.push(`${target.id || target.className}:${event.isTrusted}`);
            }
        });
        for (const eventName of ['input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        assertStateHostsRendered(harness.root, 'cem-nav, cem-tabs');
        for (const navigationCase of activeCases) {
            const { host, owner, state, tokens, wrapper } = navigationCase;
            await assertFocusVisible(owner);
            await userEvent.hover(owner);
            await nextRenderFrame();

            const hovered = captureNavigationState(runtime, host, wrapper, owner);
            const clickCount = clickEvents.length;
            expect(hovered.backgroundColor).toBe(resolveTokenColor(owner, tokens.hoverBackground));
            expect(hovered.color).toBe(resolveTokenColor(owner, tokens.hoverText));
            expect(hovered.forcedColorAdjust).toBe('auto');

            const pointerDown = nextTrustedPointerDown(owner);
            const click = userEvent.click(owner, { delay: 200 });
            const downEvent = await eventBeforeInteractionCompletes(pointerDown, click, 'navigation pointerdown');
            expect(downEvent.isTrusted).toBe(true);
            await waitForPseudoClass(owner, ':active');
            await nextRenderFrame();

            const active = captureNavigationState(runtime, host, wrapper, owner);
            expect(owner.matches(':active')).toBe(true);
            expectPaintedColorToResolveFromToken(active.backgroundColor, owner, tokens.activeBackground);
            expectPaintedColorToResolveFromToken(active.color, owner, tokens.activeText);
            expect(active.backgroundColor).not.toBe(hovered.backgroundColor);
            expect(contrastRatio(active.backgroundColor, active.color)).toBeGreaterThanOrEqual(4.5);
            expectNavigationStructureAndGeometry(active, hovered);
            expect(active.focusTreatment).toEqual(hovered.focusTreatment);
            expect(active.forcedColorAdjust).toBe('auto');
            expect(document.activeElement).toBe(owner);
            expect(clickEvents).toHaveLength(clickCount);
            if (state) {
                expect(owner.getAttribute(state.attribute)).toBe(state.value);
            }

            await click;
            await runtime.whenRenderSettled(host);
            await nextRenderFrame();
            const released = captureNavigationState(runtime, host, wrapper, owner);
            expect(owner.matches(':active')).toBe(false);
            expect(released.backgroundColor).toBe(resolveTokenColor(owner, tokens.hoverBackground));
            expect(released.color).toBe(resolveTokenColor(owner, tokens.hoverText));
            expect(released.focusTreatment).toEqual(hovered.focusTreatment);
            expect(released.forcedColorAdjust).toBe('auto');
            expect(document.activeElement).toBe(owner);
            expect(clickEvents).toHaveLength(clickCount + 1);
            expect(clickEvents.at(-1)).toBe(`${owner.id || owner.className}:true`);

            if (owner === disclosure) {
                expect(
                    harness.query<HTMLButtonElement>(
                        'cem-nav[label="Active workspace navigation"] > nav > .cem-nav__disclosure',
                    ),
                ).toBe(disclosure);
                expect(owner.getAttribute('aria-expanded')).toBe('false');
                expect(harness.query<HTMLDivElement>('#active-nav-content').parentElement?.hidden).toBe(true);
                const releaseSnapshot = runtime.snapshotInstance(host);
                expect(releaseSnapshot.slices.expanded).toBe(false);
                expect(eventPayload(releaseSnapshot, 'expanded')).toMatchObject({
                    sliceValue: false,
                    type: 'click',
                });
            } else {
                expectNavigationStructureAndGeometry(released, hovered);
                if (state) {
                    expect(owner.getAttribute(state.attribute)).toBe(state.value);
                }
            }

            await userEvent.unhover(owner);
            await nextRenderFrame();
            const restored = captureNavigationState(runtime, host, wrapper, owner);
            expect(restored.backgroundColor).toBe(resolveTokenColor(owner, tokens.defaultBackground));
            expect(restored.color).toBe(resolveTokenColor(owner, tokens.defaultText));
            expect(restored.focusTreatment).toEqual(released.focusTreatment);
            expect(restored.forcedColorAdjust).toBe('auto');
        }

        for (const disabledCase of disabledCases) {
            const { host, owner, wrapper } = disabledCase;
            const baseline = captureNavigationState(runtime, host, wrapper, owner);
            const clickCount = clickEvents.length;
            expect(baseline.backgroundColor).toBe(
                resolveTokenColor(owner, '--cem-navigation-item-disabled-background'),
            );
            expect(baseline.color).toBe(resolveTokenColor(owner, '--cem-navigation-item-disabled-text'));

            const pointerDown = nextTrustedPointerDown(owner);
            const click = userEvent.click(owner, { delay: 200, force: true });
            const downEvent = await eventBeforeInteractionCompletes(
                pointerDown,
                click,
                'disabled navigation pointerdown',
            );
            expect(downEvent.isTrusted).toBe(true);
            await nextRenderFrame();

            const held = captureNavigationState(runtime, host, wrapper, owner);
            expect(held.backgroundColor).toBe(baseline.backgroundColor);
            expect(held.color).toBe(baseline.color);
            expect(held.backgroundColor).not.toBe(
                resolveTokenColor(owner, '--cem-navigation-item-active-background'),
            );
            expectNavigationStructureAndGeometry(held, baseline);
            expect(clickEvents).toHaveLength(clickCount);

            await click;
            await nextRenderFrame();
            expect(clickEvents).toHaveLength(clickCount);
            const restored = captureNavigationState(runtime, host, wrapper, owner);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectNavigationStructureAndGeometry(restored, baseline);
            await userEvent.unhover(owner);
        }

        const defaultLink = activeCases[0].owner;
        const currentLink = activeCases[1].owner;
        const selectedTab = activeCases[4].owner;
        await userEvent.unhover(defaultLink);
        await userEvent.keyboard('{Tab}');
        await assertFocusVisible(defaultLink);
        let keyboardClickCount = clickEvents.length;
        await userEvent.keyboard('{Enter}');
        await nextRenderFrame();
        expect(clickEvents).toHaveLength(keyboardClickCount + 1);
        expect(defaultLink.matches(':active')).toBe(false);
        expect(document.activeElement).toBe(defaultLink);

        await assertFocusVisible(selectedTab);
        keyboardClickCount = clickEvents.length;
        await userEvent.keyboard('{Enter}');
        await nextRenderFrame();
        expect(clickEvents).toHaveLength(keyboardClickCount + 1);
        expect(selectedTab.getAttribute('aria-selected')).toBe('true');
        expect(document.activeElement).toBe(selectedTab);

        await assertFocusVisible(currentLink);
        keyboardClickCount = clickEvents.length;
        await userEvent.keyboard(' ');
        await nextRenderFrame();
        expect(clickEvents).toHaveLength(keyboardClickCount);
        expect(currentLink.getAttribute('aria-current')).toBe('page');

        await assertFocusVisible(selectedTab);
        const selectedKeyboardBaseline = captureNavigationState(runtime, tabsHost, tabsWrapper, selectedTab);
        keyboardClickCount = clickEvents.length;
        await userEvent.keyboard('[Space>]');
        await waitForPseudoClass(selectedTab, ':active');
        await nextRenderFrame();
        const selectedKeyboardActive = captureNavigationState(runtime, tabsHost, tabsWrapper, selectedTab);
        expectPaintedColorToResolveFromToken(
            selectedKeyboardActive.backgroundColor,
            selectedTab,
            '--cem-navigation-item-current-active-background',
        );
        expectPaintedColorToResolveFromToken(
            selectedKeyboardActive.color,
            selectedTab,
            '--cem-navigation-item-current-active-text',
        );
        expect(selectedKeyboardActive.focusTreatment).toEqual(selectedKeyboardBaseline.focusTreatment);
        expectNavigationStructureAndGeometry(selectedKeyboardActive, selectedKeyboardBaseline);
        expect(clickEvents).toHaveLength(keyboardClickCount);
        await userEvent.keyboard('[/Space]');
        await nextRenderFrame();
        expect(clickEvents).toHaveLength(keyboardClickCount + 1);
        expect(selectedTab.getAttribute('aria-selected')).toBe('true');

        await assertFocusVisible(disclosure);
        const disclosureKeyboardBaseline = captureNavigationState(
            runtime,
            disclosureHost,
            disclosureWrapper,
            disclosure,
        );
        keyboardClickCount = clickEvents.length;
        expect(disclosure.getAttribute('aria-expanded')).toBe('false');
        await userEvent.keyboard('[Space>]');
        await waitForPseudoClass(disclosure, ':active');
        await nextRenderFrame();
        const disclosureKeyboardActive = captureNavigationState(
            runtime,
            disclosureHost,
            disclosureWrapper,
            disclosure,
        );
        expectPaintedColorToResolveFromToken(
            disclosureKeyboardActive.backgroundColor,
            disclosure,
            '--cem-navigation-item-active-background',
        );
        expectPaintedColorToResolveFromToken(
            disclosureKeyboardActive.color,
            disclosure,
            '--cem-navigation-item-active-text',
        );
        expectNavigationStructureAndGeometry(disclosureKeyboardActive, disclosureKeyboardBaseline);
        expect(clickEvents).toHaveLength(keyboardClickCount);
        expect(disclosure.getAttribute('aria-expanded')).toBe('false');
        await userEvent.keyboard('[/Space]');
        await runtime.whenRenderSettled(disclosureHost);
        await nextRenderFrame();
        expect(clickEvents).toHaveLength(keyboardClickCount + 1);
        expect(disclosure.getAttribute('aria-expanded')).toBe('true');
        expect(harness.query<HTMLDivElement>('#active-nav-content').parentElement?.hidden).toBe(false);
        expect(runtime.snapshotInstance(disclosureHost).slices.expanded).toBe(true);
        expect(document.activeElement).toBe(disclosure);

        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('keeps aria-disabled navigation owners discoverable while suppressing activation', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <section class="cem-theme-light">
                <form id="disabled-navigation-form">
                    <button id="disabled-navigation-start" type="button">Start</button>
                    <cem-nav label="Disabled primary navigation">
                        <a id="disabled-nav-enabled" href="#enabled">Enabled link</a>
                        <a
                            id="disabled-nav-current"
                            href="#disabled-current"
                            aria-current="page"
                            aria-disabled="true"
                        >
                            <span id="disabled-nav-current-label">Unavailable current page</span>
                        </a>
                        <button id="disabled-nav-submit" type="submit" aria-disabled="true">
                            Unavailable submit action
                        </button>
                        <button id="disabled-nav-native" type="submit" disabled>Native disabled action</button>
                        <a id="disabled-nav-tail" href="#tail">Tail link</a>
                        <div>
                            <button id="disabled-nav-nested" type="button" aria-disabled="true" tabindex="-1">
                                Nested component-owned control
                            </button>
                        </div>
                    </cem-nav>
                    <cem-tabs label="Disabled profile sections">
                        <button id="disabled-tab-enabled" type="button" role="tab" aria-selected="false">
                            Enabled tab
                        </button>
                        <button
                            id="disabled-tab-selected"
                            type="button"
                            role="tab"
                            aria-selected="true"
                            aria-disabled="true"
                        >
                            Unavailable selected tab
                        </button>
                        <button
                            id="disabled-tab-native"
                            type="button"
                            role="tab"
                            aria-selected="false"
                            disabled
                        >
                            Native disabled tab
                        </button>
                    </cem-tabs>
                    <button id="disabled-navigation-end" type="button">End</button>
                </form>
            </section>
        `);
        await waitForStateSelector(root, '#disabled-tab-native:disabled');

        const form = harness.query<HTMLFormElement>('#disabled-navigation-form');
        const start = harness.query<HTMLButtonElement>('#disabled-navigation-start');
        const end = harness.query<HTMLButtonElement>('#disabled-navigation-end');
        const navHost = harness.query<HTMLElement>('cem-nav[label="Disabled primary navigation"]');
        const navWrapper = harness.query<HTMLElement>('cem-nav[label="Disabled primary navigation"] > nav');
        const tabsHost = harness.query<HTMLElement>('cem-tabs[label="Disabled profile sections"]');
        const tabsWrapper = harness.query<HTMLElement>(
            'cem-tabs[label="Disabled profile sections"] > [role="tablist"]',
        );
        const ariaDisabledLink = harness.query<HTMLAnchorElement>('#disabled-nav-current');
        const ariaDisabledLinkLabel = harness.query<HTMLElement>('#disabled-nav-current-label');
        const ariaDisabledButton = harness.query<HTMLButtonElement>('#disabled-nav-submit');
        const ariaDisabledTab = harness.query<HTMLButtonElement>('#disabled-tab-selected');
        const nativeDisabledButton = harness.query<HTMLButtonElement>('#disabled-nav-native');
        const nativeDisabledTab = harness.query<HTMLButtonElement>('#disabled-tab-native');
        const nestedDisabledButton = harness.query<HTMLButtonElement>('#disabled-nav-nested');
        const ariaCases = [
            {
                host: navHost,
                owner: ariaDisabledLink,
                pointerTarget: ariaDisabledLinkLabel,
                state: { attribute: 'aria-current', value: 'page' },
                activationKeys: ['Enter'],
                wrapper: navWrapper,
            },
            {
                host: navHost,
                owner: ariaDisabledButton,
                pointerTarget: ariaDisabledButton,
                state: null,
                activationKeys: ['Enter', ' '],
                wrapper: navWrapper,
            },
            {
                host: tabsHost,
                owner: ariaDisabledTab,
                pointerTarget: ariaDisabledTab,
                state: { attribute: 'aria-selected', value: 'true' },
                activationKeys: ['Enter', ' '],
                wrapper: tabsWrapper,
            },
        ] as const;
        const nativeCases = [
            { host: navHost, owner: nativeDisabledButton, wrapper: navWrapper },
            { host: tabsHost, owner: nativeDisabledTab, wrapper: tabsWrapper },
        ] as const;
        const focusOrder: readonly HTMLElement[] = [
            harness.query<HTMLAnchorElement>('#disabled-nav-enabled'),
            ariaDisabledLink,
            ariaDisabledButton,
            harness.query<HTMLAnchorElement>('#disabled-nav-tail'),
            harness.query<HTMLButtonElement>('#disabled-tab-enabled'),
            ariaDisabledTab,
        ];
        const focusEvents: string[] = [];
        harness.root.addEventListener('focusin', (event) => {
            if (event.target instanceof HTMLElement && focusOrder.includes(event.target)) {
                focusEvents.push(event.target.id);
            }
        });

        assertStateHostsRendered(harness.root, 'cem-nav, cem-tabs');
        start.focus();
        for (const owner of focusOrder) {
            await userEvent.tab();
            await nextRenderFrame();
            expect(document.activeElement).toBe(owner);
            expect(owner.matches(':focus-visible')).toBe(true);
            if (owner.getAttribute('aria-disabled') === 'true') {
                const styles = getComputedStyle(owner);
                expect(paintedColor(styles.backgroundColor)).toBe(
                    resolveTokenColor(owner, '--cem-navigation-item-disabled-background'),
                );
                expect(paintedColor(styles.color)).toBe(
                    resolveTokenColor(owner, '--cem-navigation-item-disabled-text'),
                );
                expect(paintedColor(styles.outlineColor)).toBe(resolveTokenColor(owner, '--cem-zebra-color-1'));
            }
            expect(document.activeElement).not.toBe(nativeDisabledButton);
            expect(document.activeElement).not.toBe(nativeDisabledTab);
        }
        await userEvent.tab();
        expect(document.activeElement).toBe(end);
        expect(focusEvents).toEqual(focusOrder.map((owner) => owner.id));

        const capturedClicks: MouseEvent[] = [];
        const capturedKeys: KeyboardEvent[] = [];
        const leakedActivationEvents: string[] = [];
        const targetActivationEvents: string[] = [];
        const linkSpaceEvents: string[] = [];
        const mutationEvents: string[] = [];
        let submissions = 0;
        const ariaOwners: readonly HTMLElement[] = ariaCases.map(({ owner }) => owner);
        const eventOwner = (event: Event): HTMLElement | undefined => {
            const target = event.target;
            return target instanceof Node
                ? ariaOwners.find((owner) => owner === target || owner.contains(target))
                : undefined;
        };
        const isActivationKey = (owner: HTMLElement | undefined, event: KeyboardEvent): boolean =>
            Boolean(owner && (event.key === 'Enter' || (event.key === ' ' && owner instanceof HTMLButtonElement)));

        harness.root.addEventListener(
            'click',
            (event) => {
                if (eventOwner(event) && event instanceof MouseEvent) capturedClicks.push(event);
            },
            true,
        );
        for (const eventName of ['keydown', 'keyup'] as const) {
            harness.root.addEventListener(
                eventName,
                (event) => {
                    if (event instanceof KeyboardEvent && eventOwner(event)) capturedKeys.push(event);
                },
                true,
            );
            harness.root.addEventListener(eventName, (event) => {
                const owner = eventOwner(event);
                if (event instanceof KeyboardEvent && isActivationKey(owner, event)) {
                    leakedActivationEvents.push(`${owner?.id}:${event.type}:${event.key}`);
                }
            });
        }
        harness.root.addEventListener('click', (event) => {
            const owner = eventOwner(event);
            if (owner) leakedActivationEvents.push(`${owner.id}:click`);
        });
        for (const owner of ariaOwners) {
            owner.addEventListener('click', () => targetActivationEvents.push(`${owner.id}:click`));
            for (const eventName of ['keydown', 'keyup'] as const) {
                owner.addEventListener(eventName, (event) => {
                    if (event instanceof KeyboardEvent && isActivationKey(owner, event)) {
                        targetActivationEvents.push(`${owner.id}:${event.type}:${event.key}`);
                    }
                });
            }
        }
        for (const eventName of ['keydown', 'keyup'] as const) {
            ariaDisabledLink.addEventListener(eventName, (event) => {
                if (event.key === ' ') linkSpaceEvents.push(`${event.type}:${event.defaultPrevented}`);
            });
        }
        form.addEventListener('submit', (event) => {
            submissions += 1;
            event.preventDefault();
        });
        for (const eventName of ['input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        const baselines = new Map<HTMLElement, NavigationStateSnapshot>();
        for (const navigationCase of [...ariaCases, ...nativeCases]) {
            const baseline = captureNavigationState(
                runtime,
                navigationCase.host,
                navigationCase.wrapper,
                navigationCase.owner,
            );
            baselines.set(navigationCase.owner, baseline);
            expect(baseline.backgroundColor).toBe(
                resolveTokenColor(navigationCase.owner, '--cem-navigation-item-disabled-background'),
            );
            expect(baseline.color).toBe(
                resolveTokenColor(navigationCase.owner, '--cem-navigation-item-disabled-text'),
            );
        }

        for (const navigationCase of ariaCases) {
            const { host, owner, pointerTarget, state, activationKeys, wrapper } = navigationCase;
            const baseline = baselines.get(owner);
            if (!baseline) throw new Error(`Missing disabled navigation baseline for ${owner.id}`);

            let clickCount = capturedClicks.length;
            let targetCount = targetActivationEvents.length;
            let leakedCount = leakedActivationEvents.length;
            await userEvent.click(pointerTarget, { force: true });
            await nextRenderFrame();
            expect(capturedClicks).toHaveLength(clickCount + 1);
            expect(capturedClicks.at(-1)?.isTrusted).toBe(true);
            expect(capturedClicks.at(-1)?.defaultPrevented).toBe(true);
            expect(targetActivationEvents).toHaveLength(targetCount);
            expect(leakedActivationEvents).toHaveLength(leakedCount);
            expect(submissions).toBe(0);

            clickCount = capturedClicks.length;
            targetCount = targetActivationEvents.length;
            leakedCount = leakedActivationEvents.length;
            owner.click();
            await nextRenderFrame();
            expect(capturedClicks).toHaveLength(clickCount + 1);
            expect(capturedClicks.at(-1)?.isTrusted).toBe(false);
            expect(capturedClicks.at(-1)?.defaultPrevented).toBe(true);
            expect(targetActivationEvents).toHaveLength(targetCount);
            expect(leakedActivationEvents).toHaveLength(leakedCount);
            expect(submissions).toBe(0);

            await userEvent.keyboard('{Tab}');
            await assertFocusVisible(owner);
            for (const key of activationKeys) {
                clickCount = capturedClicks.length;
                targetCount = targetActivationEvents.length;
                leakedCount = leakedActivationEvents.length;
                const keyCount = capturedKeys.length;
                await userEvent.keyboard(key === 'Enter' ? '{Enter}' : ' ');
                await nextRenderFrame();
                const keyEvents = capturedKeys.slice(keyCount);
                expect(keyEvents.map((event) => event.type)).toEqual(['keydown', 'keyup']);
                expect(keyEvents.every((event) => event.defaultPrevented)).toBe(true);
                expect(capturedClicks).toHaveLength(clickCount);
                expect(targetActivationEvents).toHaveLength(targetCount);
                expect(leakedActivationEvents).toHaveLength(leakedCount);
                expect(document.activeElement).toBe(owner);
                expect(submissions).toBe(0);
            }

            const after = captureNavigationState(runtime, host, wrapper, owner);
            expectNavigationStructureAndGeometry(after, baseline);
            expect(after.backgroundColor, `${owner.id} changed disabled fill`).toBe(baseline.backgroundColor);
            expect(after.color, `${owner.id} changed disabled text`).toBe(baseline.color);
            if (state) expect(owner.getAttribute(state.attribute)).toBe(state.value);
        }

        await userEvent.keyboard('{Tab}');
        await assertFocusVisible(ariaDisabledLink);
        const linkClickCount = capturedClicks.length;
        const linkKeyCount = capturedKeys.length;
        const linkTargetCount = targetActivationEvents.length;
        const linkLeakedCount = leakedActivationEvents.length;
        await userEvent.keyboard(' ');
        await nextRenderFrame();
        expect(capturedKeys.slice(linkKeyCount).map((event) => `${event.type}:${event.defaultPrevented}`)).toEqual([
            'keydown:false',
            'keyup:false',
        ]);
        expect(capturedClicks).toHaveLength(linkClickCount);
        expect(targetActivationEvents).toHaveLength(linkTargetCount);
        expect(leakedActivationEvents).toHaveLength(linkLeakedCount);
        expect(linkSpaceEvents).toEqual(['keydown:false', 'keyup:false']);

        for (const navigationCase of nativeCases) {
            const { host, owner, wrapper } = navigationCase;
            const baseline = baselines.get(owner);
            if (!baseline) throw new Error(`Missing native-disabled navigation baseline for ${owner.id}`);
            const clickCount = capturedClicks.length;
            owner.focus();
            expect(document.activeElement).not.toBe(owner);
            owner.click();
            await userEvent.click(owner, { force: true });
            await nextRenderFrame();
            expect(capturedClicks).toHaveLength(clickCount);
            expect(submissions).toBe(0);
            expectNavigationStructureAndGeometry(
                captureNavigationState(runtime, host, wrapper, owner),
                baseline,
            );
        }

        let nestedClicks = 0;
        nestedDisabledButton.addEventListener('click', (event) => {
            nestedClicks += 1;
            event.preventDefault();
        });
        await userEvent.click(nestedDisabledButton, { force: true });
        expect(nestedClicks).toBe(1);

        await userEvent.keyboard('{Tab}');

        expect(Array.from(new FormData(form).entries())).toEqual([]);
        expect(ariaOwners.map((owner) => owner.getAttribute('tabindex'))).toEqual([null, null, null]);
        expect(ariaDisabledLink.getAttribute('aria-current')).toBe('page');
        expect(ariaDisabledTab.getAttribute('aria-selected')).toBe('true');
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('applies shared native active treatment during pointer and keyboard activation', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack class="cem-theme-light" gap="sm">
                <cem-action>Save changes</cem-action>
                <cem-icon-button name="settings" label="Open settings"></cem-icon-button>
                <cem-menu-item>Open menu</cem-menu-item>
                <cem-action disabled>Disabled save</cem-action>
                <cem-icon-button name="settings" label="Disabled settings" disabled></cem-icon-button>
                <cem-menu-item disabled>Disabled menu</cem-menu-item>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-menu-item[disabled] button');

        const actionCases = [
            {
                host: harness.query<HTMLElement>('cem-action:not([disabled])'),
                button: harness.query<HTMLButtonElement>('cem-action:not([disabled]) button'),
                name: 'Save changes',
                role: null,
                slice: 'pressed',
                targetTag: 'button',
                tokens: {
                    activeBackground: '--cem-action-primary-active-background',
                    activeText: '--cem-action-primary-active-text',
                    defaultBackground: '--cem-action-primary-default-background',
                    defaultText: '--cem-action-primary-default-text',
                    hoverBackground: '--cem-action-primary-hover-background',
                    hoverText: '--cem-action-primary-hover-text',
                },
            },
            {
                host: harness.query<HTMLElement>('cem-icon-button:not([disabled])'),
                button: harness.query<HTMLButtonElement>('cem-icon-button:not([disabled]) button'),
                name: 'Open settings',
                role: null,
                slice: 'pressed',
                targetTag: 'span',
                tokens: {
                    activeBackground: '--cem-action-contextual-active-background',
                    activeText: '--cem-action-contextual-active-text',
                    defaultBackground: '--cem-action-contextual-default-background',
                    defaultText: '--cem-action-contextual-default-text',
                    hoverBackground: '--cem-action-contextual-hover-background',
                    hoverText: '--cem-action-contextual-hover-text',
                },
            },
            {
                host: harness.query<HTMLElement>('cem-menu-item:not([disabled])'),
                button: harness.query<HTMLButtonElement>('cem-menu-item:not([disabled]) button'),
                name: 'Open menu',
                role: 'menuitem',
                slice: 'selected',
                targetTag: 'button',
                tokens: {
                    activeBackground: '--cem-action-contextual-active-background',
                    activeText: '--cem-action-contextual-active-text',
                    defaultBackground: '--cem-action-contextual-default-background',
                    defaultText: '--cem-action-contextual-default-text',
                    hoverBackground: '--cem-action-contextual-hover-background',
                    hoverText: '--cem-action-contextual-hover-text',
                },
            },
        ] as const;
        const disabledCases = [
            {
                host: harness.query<HTMLElement>('cem-action[disabled]'),
                button: harness.query<HTMLButtonElement>('cem-action[disabled] button'),
                name: 'Disabled save',
                role: null,
                tokens: actionCases[0].tokens,
            },
            {
                host: harness.query<HTMLElement>('cem-icon-button[disabled]'),
                button: harness.query<HTMLButtonElement>('cem-icon-button[disabled] button'),
                name: 'Disabled settings',
                role: null,
                tokens: actionCases[1].tokens,
            },
            {
                host: harness.query<HTMLElement>('cem-menu-item[disabled]'),
                button: harness.query<HTMLButtonElement>('cem-menu-item[disabled] button'),
                name: 'Disabled menu',
                role: 'menuitem',
                tokens: actionCases[2].tokens,
            },
        ] as const;
        const activationEvents: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => activationEvents.push(eventName));
        }

        assertStateHostsRendered(harness.root, 'cem-action, cem-icon-button, cem-menu-item');

        for (const [index, actionCase] of actionCases.entries()) {
            const { button, host, name, role, slice, targetTag, tokens } = actionCase;
            expect(button.type).toBe('button');
            expect(button.disabled).toBe(false);
            expect(button.getAttribute('role')).toBe(role);
            expect(assertAccessibleName(button, name)).toBe(name);
            await assertFocusVisible(button);
            await userEvent.hover(button);
            await nextRenderFrame();

            const hovered = captureActionState(runtime, host, button);
            expect(hovered.backgroundColor).toBe(resolveTokenColor(button, tokens.hoverBackground));
            expect(hovered.color).toBe(resolveTokenColor(button, tokens.hoverText));
            expect(hovered.forcedColorAdjust).toBe('auto');

            const pointerDown = nextTrustedPointerDown(button);
            const click = userEvent.click(button, { delay: 200 });
            const downEvent = await eventBeforeInteractionCompletes(pointerDown, click, 'pointerdown');
            expect(downEvent.isTrusted).toBe(true);
            await waitForPseudoClass(button, ':active');
            await nextRenderFrame();

            const active = captureActionState(runtime, host, button);
            expect(button.matches(':active')).toBe(true);
            expectPaintedColorToResolveFromToken(active.backgroundColor, button, tokens.activeBackground);
            expectPaintedColorToResolveFromToken(active.color, button, tokens.activeText);
            expect(active.backgroundColor).not.toBe(hovered.backgroundColor);
            expect(contrastRatio(active.backgroundColor, active.color)).toBeGreaterThanOrEqual(4.5);
            expectActionStructureAndGeometry(active, hovered);
            expect(active.focusTreatment).toEqual(hovered.focusTreatment);
            expect(active.forcedColorAdjust).toBe(hovered.forcedColorAdjust);
            expect(document.activeElement).toBe(button);
            expect(activationEvents).toHaveLength(index);

            await click;
            await nextRenderFrame();

            const released = captureActionState(runtime, host, button);
            expect(button.matches(':active')).toBe(false);
            expect(released.backgroundColor).toBe(resolveTokenColor(button, tokens.hoverBackground));
            expect(released.color).toBe(resolveTokenColor(button, tokens.hoverText));
            expectActionStructureAndGeometryAfterActivation(released, hovered);
            expect(released.focusTreatment).toEqual(hovered.focusTreatment);
            expect(released.forcedColorAdjust).toBe('auto');
            expect(released.runtime).not.toBe(active.runtime);
            expect(document.activeElement).toBe(button);
            expect(activationEvents).toEqual(Array.from({ length: index + 1 }, () => 'click'));

            const releaseSnapshot = runtime.snapshotInstance(host);
            const releasePayload = eventPayload(releaseSnapshot, slice);
            expect(releaseSnapshot.slices[slice]).toBe('click');
            expect(releasePayload.type).toBe('click');
            expect(releasePayload.sliceValue).toBe('click');
            expect(releasePayload.currentTarget?.tag).toBe('button');
            expect(releasePayload.target?.tag).toBe(targetTag);

            await userEvent.unhover(button);
            await nextRenderFrame();

            const restored = captureActionState(runtime, host, button);
            expect(restored.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));
            expect(restored.color).toBe(resolveTokenColor(button, tokens.defaultText));
            expectActionStructureAndGeometryAfterActivation(restored, hovered);
            expect(restored.focusTreatment).toEqual(hovered.focusTreatment);
            expect(restored.forcedColorAdjust).toBe('auto');
            expect(document.activeElement).toBe(button);
        }

        for (const actionCase of disabledCases) {
            const { button, host, name, role, tokens } = actionCase;
            expect(button.type).toBe('button');
            expect(button.disabled).toBe(true);
            expect(button.getAttribute('role')).toBe(role);
            expect(assertAccessibleName(button, name)).toBe(name);

            const focusOwner = document.activeElement;
            button.focus();
            expect(document.activeElement).toBe(focusOwner);
            const baseline = captureActionState(runtime, host, button);
            const eventCount = activationEvents.length;
            expect(baseline.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(button, tokens.defaultText));
            expect(baseline.forcedColorAdjust).toBe('auto');

            const pointerDown = nextTrustedPointerDown(button);
            const click = userEvent.click(button, { delay: 200, force: true });
            const downEvent = await eventBeforeInteractionCompletes(pointerDown, click, 'disabled pointerdown');
            expect(downEvent.isTrusted).toBe(true);
            await nextRenderFrame();

            const held = captureActionState(runtime, host, button);
            expect(held.backgroundColor).toBe(baseline.backgroundColor);
            expect(held.color).toBe(baseline.color);
            expect(held.backgroundColor).not.toBe(resolveTokenColor(button, tokens.activeBackground));
            expectActionStructureAndGeometry(held, baseline);
            expect(held.forcedColorAdjust).toBe('auto');
            expect(document.activeElement).not.toBe(button);
            expect(activationEvents).toHaveLength(eventCount);

            await click;
            await nextRenderFrame();

            const restored = captureActionState(runtime, host, button);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectActionStructureAndGeometry(restored, baseline);
            expect(restored.forcedColorAdjust).toBe('auto');
            expect(document.activeElement).not.toBe(button);
            expect(activationEvents).toHaveLength(eventCount);
            await userEvent.unhover(button);
        }

        const keyboardCase = actionCases[0];
        const { button, host, slice, tokens } = keyboardCase;
        await userEvent.unhover(button);
        await assertFocusVisible(button);
        const keyboardBaseline = captureActionState(runtime, host, button);
        const keyboardEventCount = activationEvents.length;
        expect(keyboardBaseline.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));

        await userEvent.keyboard('[Space>]');
        await waitForPseudoClass(button, ':active');
        await nextRenderFrame();

        const keyboardActive = captureActionState(runtime, host, button);
        expect(button.matches(':active')).toBe(true);
        expectPaintedColorToResolveFromToken(keyboardActive.backgroundColor, button, tokens.activeBackground);
        expectPaintedColorToResolveFromToken(keyboardActive.color, button, tokens.activeText);
        expect(keyboardActive.backgroundColor).not.toBe(keyboardBaseline.backgroundColor);
        expect(contrastRatio(keyboardActive.backgroundColor, keyboardActive.color)).toBeGreaterThanOrEqual(4.5);
        expectActionStructureAndGeometry(keyboardActive, keyboardBaseline);
        expect(keyboardActive.focusTreatment).toEqual(keyboardBaseline.focusTreatment);
        expect(keyboardActive.forcedColorAdjust).toBe('auto');
        expect(document.activeElement).toBe(button);
        expect(activationEvents).toHaveLength(keyboardEventCount);

        await userEvent.keyboard('[/Space]');
        await nextRenderFrame();

        const keyboardReleased = captureActionState(runtime, host, button);
        expect(button.matches(':active')).toBe(false);
        expect(keyboardReleased.backgroundColor).toBe(keyboardBaseline.backgroundColor);
        expect(keyboardReleased.color).toBe(keyboardBaseline.color);
        expectActionStructureAndGeometryAfterActivation(keyboardReleased, keyboardBaseline);
        expect(keyboardReleased.focusTreatment).toEqual(keyboardBaseline.focusTreatment);
        expect(keyboardReleased.forcedColorAdjust).toBe('auto');
        expect(keyboardReleased.runtime).toBe(keyboardActive.runtime);
        expect(document.activeElement).toBe(button);
        expect(activationEvents).toEqual(Array.from({ length: keyboardEventCount + 1 }, () => 'click'));

        const keyboardReleaseSnapshot = runtime.snapshotInstance(host);
        const keyboardReleasePayload = eventPayload(keyboardReleaseSnapshot, slice);
        expect(keyboardReleaseSnapshot.slices[slice]).toBe('click');
        expect(keyboardReleasePayload.type).toBe('click');
        expect(keyboardReleasePayload.sliceValue).toBe('click');
        expect(keyboardReleasePayload.currentTarget?.tag).toBe('button');
        expect(keyboardReleasePayload.target?.tag).toBe('button');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('composes tokenized input indicators across appearance, hover, focus, validation, and selection states', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack class="cem-theme-light" gap="sm">
                <cem-field name="field" label="Field" value="alpha"></cem-field>
                <cem-text-field name="outlined" label="Outlined field" indicator="outline"></cem-text-field>
                <cem-textarea name="readonly" label="Readonly notes" readonly>Notes</cem-textarea>
                <cem-select name="invalid-select" label="Invalid role" invalid="true">
                    <option value="admin">Admin</option>
                </cem-select>
                <cem-checkbox name="checked" checked>Checked option</cem-checkbox>
                <cem-radio name="fallback" indicator="unsupported">Fallback option</cem-radio>
                <cem-switch name="invalid-switch" indicator="underline" checked invalid="true">
                    Invalid switch
                </cem-switch>
                <cem-select name="disabled-select" label="Disabled role" disabled>
                    <option value="viewer">Viewer</option>
                </cem-select>
                <cem-radio name="disabled-radio" disabled>Disabled option</cem-radio>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-radio[name="disabled-radio"] input');

        const cases = [
            {
                appearance: 'underline',
                control: harness.query<HTMLInputElement>('cem-field input'),
                host: harness.query<HTMLElement>('cem-field'),
                target: harness.query<HTMLElement>('cem-field input'),
                baselineToken: '--cem-input-indicator-anchor-color',
                hoverToken: '--cem-input-indicator-anchor-hover-color',
            },
            {
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-text-field input'),
                host: harness.query<HTMLElement>('cem-text-field'),
                target: harness.query<HTMLElement>('cem-text-field input'),
                baselineToken: '--cem-input-indicator-anchor-color',
                hoverToken: '--cem-input-indicator-anchor-hover-color',
            },
            {
                appearance: 'underline',
                control: harness.query<HTMLTextAreaElement>('cem-textarea textarea'),
                host: harness.query<HTMLElement>('cem-textarea'),
                target: harness.query<HTMLElement>('cem-textarea textarea'),
                baselineToken: '--cem-input-indicator-anchor-readonly-color',
                hoverToken: '--cem-input-indicator-anchor-readonly-color',
            },
            {
                appearance: 'underline',
                control: harness.query<HTMLButtonElement>('cem-select[name="invalid-select"] .cem-select__control'),
                host: harness.query<HTMLElement>('cem-select[name="invalid-select"]'),
                target: harness.query<HTMLElement>('cem-select[name="invalid-select"] .cem-select__control'),
                baselineToken: '--cem-input-indicator-anchor-invalid-color',
                hoverToken: '--cem-input-indicator-anchor-invalid-hover-color',
            },
            {
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-checkbox input'),
                host: harness.query<HTMLElement>('cem-checkbox'),
                selection: true,
                target: harness.query<HTMLLabelElement>('cem-checkbox > label'),
                baselineToken: '--cem-input-indicator-anchor-color',
                hoverToken: '--cem-input-indicator-anchor-hover-color',
            },
            {
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-radio[name="fallback"] input'),
                host: harness.query<HTMLElement>('cem-radio[name="fallback"]'),
                target: harness.query<HTMLLabelElement>('cem-radio[name="fallback"] > label'),
                baselineToken: '--cem-input-indicator-anchor-color',
                hoverToken: '--cem-input-indicator-anchor-hover-color',
            },
            {
                appearance: 'underline',
                control: harness.query<HTMLInputElement>('cem-switch input'),
                host: harness.query<HTMLElement>('cem-switch'),
                selection: true,
                target: harness.query<HTMLLabelElement>('cem-switch > label'),
                baselineToken: '--cem-input-indicator-anchor-invalid-color',
                hoverToken: '--cem-input-indicator-anchor-invalid-hover-color',
            },
        ] as const;
        const disabledCases = [
            {
                appearance: 'underline',
                control: harness.query<HTMLButtonElement>('cem-select[name="disabled-select"] .cem-select__control'),
                host: harness.query<HTMLElement>('cem-select[name="disabled-select"]'),
                target: harness.query<HTMLElement>('cem-select[name="disabled-select"] .cem-select__control'),
            },
            {
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-radio[name="disabled-radio"] input'),
                host: harness.query<HTMLElement>('cem-radio[name="disabled-radio"]'),
                target: harness.query<HTMLLabelElement>('cem-radio[name="disabled-radio"] > label'),
            },
        ] as const;
        const mutationEvents: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        assertStateHostsRendered(
            harness.root,
            'cem-field, cem-text-field, cem-textarea, cem-select, cem-checkbox, cem-radio, cem-switch',
        );

        for (const indicatorCase of cases) {
            const { appearance, baselineToken, control, host, hoverToken, target } = indicatorCase;
            const selection = 'selection' in indicatorCase && indicatorCase.selection;
            const baseline = captureInputIndicatorState(runtime, host, control, target);

            expectInputIndicatorGeometry(baseline, target, appearance, { selection });
            expectPaintedColorToResolveFromToken(baseline.layers[0].color, target, baselineToken);
            if (selection) {
                expectPaintedColorToResolveFromToken(
                    baseline.layers[2].color,
                    target,
                    '--cem-input-indicator-selection-color',
                );
            }

            await userEvent.hover(control);
            await nextRenderFrame();

            const hovered = captureInputIndicatorState(runtime, host, control, target);
            expect(control.matches(':hover')).toBe(true);
            if (target !== control) {
                expect(target.matches(':hover')).toBe(true);
            }
            expectPaintedColorToResolveFromToken(hovered.layers[0].color, target, hoverToken);
            expectInputIndicatorGeometry(hovered, target, appearance, { selection });
            expectInputIndicatorStructureAndGeometry(hovered, baseline);

            await userEvent.unhover(control);
            await nextRenderFrame();

            const restored = captureInputIndicatorState(runtime, host, control, target);
            expectPaintedColorToResolveFromToken(restored.layers[0].color, target, baselineToken);
            expect(restored.boxShadow).toBe(baseline.boxShadow);
            expectInputIndicatorStructureAndGeometry(restored, baseline);
        }

        for (const indicatorCase of disabledCases) {
            const { appearance, control, host, target } = indicatorCase;
            expect(control.disabled).toBe(true);

            const baseline = captureInputIndicatorState(runtime, host, control, target);
            expectInputIndicatorGeometry(baseline, target, appearance);
            expectPaintedColorToResolveFromToken(
                baseline.layers[0].color,
                target,
                '--cem-input-indicator-anchor-disabled-color',
            );

            await userEvent.hover(control);
            await nextRenderFrame();

            const hovered = captureInputIndicatorState(runtime, host, control, target);
            expectPaintedColorToResolveFromToken(
                hovered.layers[0].color,
                target,
                '--cem-input-indicator-anchor-disabled-color',
            );
            expect(hovered.boxShadow).toBe(baseline.boxShadow);
            expectInputIndicatorStructureAndGeometry(hovered, baseline);

            await userEvent.unhover(control);
        }

        const fieldHost = cases[0].host;
        const field = cases[0].control;
        fieldHost.style.setProperty(
            '--cem-input-indicator-appearance',
            'var(--cem-indicator-appearance-outline)',
        );
        await nextRenderFrame();
        expectInputIndicatorGeometry(captureInputIndicatorState(runtime, fieldHost, field, field), field, 'outline');
        fieldHost.style.removeProperty('--cem-input-indicator-appearance');
        await nextRenderFrame();
        expectInputIndicatorGeometry(captureInputIndicatorState(runtime, fieldHost, field, field), field, 'underline');

        field.focus();
        await nextRenderFrame();
        const focusedField = captureInputIndicatorState(runtime, fieldHost, field, field);
        expect(document.activeElement).toBe(field);
        expect(field.matches(':focus-visible')).toBe(true);
        expectInputIndicatorGeometry(focusedField, field, 'underline', { focus: true });
        expectPaintedColorToResolveFromToken(focusedField.layers[1].color, field, '--cem-zebra-color-1');

        const switchCase = cases[6];
        switchCase.control.focus();
        await nextRenderFrame();
        await userEvent.hover(switchCase.control);
        await nextRenderFrame();
        const focusedInvalidSelection = captureInputIndicatorState(
            runtime,
            switchCase.host,
            switchCase.control,
            switchCase.target,
        );
        expect(document.activeElement).toBe(switchCase.control);
        expect(switchCase.control.matches(':focus-visible')).toBe(true);
        expectInputIndicatorGeometry(focusedInvalidSelection, switchCase.target, 'underline', {
            focus: true,
            selection: true,
        });
        expectPaintedColorToResolveFromToken(
            focusedInvalidSelection.layers[0].color,
            switchCase.target,
            '--cem-input-indicator-anchor-invalid-hover-color',
        );
        expectPaintedColorToResolveFromToken(
            focusedInvalidSelection.layers[1].color,
            switchCase.target,
            '--cem-zebra-color-1',
        );
        expectPaintedColorToResolveFromToken(
            focusedInvalidSelection.layers[2].color,
            switchCase.target,
            '--cem-input-indicator-selection-color',
        );
        await userEvent.unhover(switchCase.control);

        expect(cases[0].control.value).toBe('alpha');
        expect(cases[2].control.readOnly).toBe(true);
        expect(cases[3].control.getAttribute('aria-invalid')).toBe('true');
        expect(cases[4].control.checked).toBe(true);
        expect(cases[6].control.checked).toBe(true);
        expect(cases[6].control.getAttribute('role')).toBe('switch');
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('moves keyboard focus through every enabled input indicator without changing component state', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <section class="cem-theme-light">
                <button id="input-focus-start" type="button">Start input focus sequence</button>
                <cem-stack gap="sm">
                    <cem-field name="focus-field" label="Generic field" value="alpha"></cem-field>
                    <cem-text-field name="disabled-field" label="Disabled field" disabled></cem-text-field>
                    <cem-text-field
                        name="focus-text-field"
                        label="Outlined text field"
                        indicator="outline"
                    ></cem-text-field>
                    <cem-textarea name="focus-textarea" label="Readonly notes" readonly>Notes</cem-textarea>
                    <cem-select name="focus-select" label="Invalid role" invalid="true">
                        <option value="admin">Admin</option>
                    </cem-select>
                    <cem-checkbox name="focus-checkbox" indeterminate="mixed">Mixed option</cem-checkbox>
                    <cem-checkbox name="disabled-checkbox" disabled>Disabled option</cem-checkbox>
                    <cem-radio name="focus-radio" indicator="underline" checked>Selected radio</cem-radio>
                    <cem-switch name="focus-switch" checked invalid="true">Invalid switch</cem-switch>
                </cem-stack>
                <button id="input-focus-end" type="button">End input focus sequence</button>
            </section>
        `);
        await waitForStateSelector(root, 'cem-switch[name="focus-switch"] input');

        const start = harness.query<HTMLButtonElement>('#input-focus-start');
        const end = harness.query<HTMLButtonElement>('#input-focus-end');
        const cases = [
            {
                anchorToken: '--cem-input-indicator-anchor-color',
                appearance: 'underline',
                control: harness.query<HTMLInputElement>('cem-field[name="focus-field"] input'),
                host: harness.query<HTMLElement>('cem-field[name="focus-field"]'),
                target: harness.query<HTMLElement>('cem-field[name="focus-field"] input'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-color',
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-text-field[name="focus-text-field"] input'),
                host: harness.query<HTMLElement>('cem-text-field[name="focus-text-field"]'),
                target: harness.query<HTMLElement>('cem-text-field[name="focus-text-field"] input'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-readonly-color',
                appearance: 'underline',
                control: harness.query<HTMLTextAreaElement>('cem-textarea[name="focus-textarea"] textarea'),
                host: harness.query<HTMLElement>('cem-textarea[name="focus-textarea"]'),
                target: harness.query<HTMLElement>('cem-textarea[name="focus-textarea"] textarea'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-invalid-color',
                appearance: 'underline',
                control: harness.query<HTMLButtonElement>('cem-select[name="focus-select"] .cem-select__control'),
                host: harness.query<HTMLElement>('cem-select[name="focus-select"]'),
                target: harness.query<HTMLElement>('cem-select[name="focus-select"] .cem-select__control'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-color',
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-checkbox[name="focus-checkbox"] input'),
                host: harness.query<HTMLElement>('cem-checkbox[name="focus-checkbox"]'),
                selectionToken: '--cem-input-indicator-indeterminate-color',
                target: harness.query<HTMLLabelElement>('cem-checkbox[name="focus-checkbox"] > label'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-color',
                appearance: 'underline',
                control: harness.query<HTMLInputElement>('cem-radio[name="focus-radio"] input'),
                host: harness.query<HTMLElement>('cem-radio[name="focus-radio"]'),
                selectionToken: '--cem-input-indicator-selection-color',
                target: harness.query<HTMLLabelElement>('cem-radio[name="focus-radio"] > label'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-invalid-color',
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-switch[name="focus-switch"] input'),
                host: harness.query<HTMLElement>('cem-switch[name="focus-switch"]'),
                selectionToken: '--cem-input-indicator-selection-color',
                target: harness.query<HTMLLabelElement>('cem-switch[name="focus-switch"] > label'),
            },
        ] as const;
        const disabledControls = [
            harness.query<HTMLInputElement>('cem-text-field[name="disabled-field"] input'),
            harness.query<HTMLInputElement>('cem-checkbox[name="disabled-checkbox"] input'),
        ];
        const baselines = cases.map(({ control, host, target }) =>
            captureInputIndicatorState(runtime, host, control, target),
        );
        const focusOrder: string[] = [];
        const mutationEvents: string[] = [];
        harness.root.addEventListener('focusin', (event) => {
            const target = event.target;
            if (
                target instanceof HTMLInputElement
                || target instanceof HTMLSelectElement
                || target instanceof HTMLTextAreaElement
                || (target instanceof HTMLButtonElement && target.matches('cem-select .cem-select__control'))
            ) {
                focusOrder.push(target.closest<HTMLElement>('cem-select')?.getAttribute('name') ?? target.getAttribute('name') ?? '');
            }
        });
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        assertStateHostsRendered(
            harness.root,
            'cem-field, cem-text-field, cem-textarea, cem-select, cem-checkbox, cem-radio, cem-switch',
        );
        for (const control of disabledControls) {
            expect(control.disabled).toBe(true);
        }

        start.focus();
        expect(document.activeElement).toBe(start);

        for (const [index, indicatorCase] of cases.entries()) {
            await userEvent.tab();
            await nextRenderFrame();

            const { anchorToken, appearance, control, host, target } = indicatorCase;
            const selectionToken = 'selectionToken' in indicatorCase ? indicatorCase.selectionToken : null;
            const focused = captureInputIndicatorState(runtime, host, control, target);

            expect(document.activeElement).toBe(control);
            expect(control.matches(':focus-visible')).toBe(true);
            expectInputIndicatorGeometry(focused, target, appearance, {
                focus: true,
                selection: selectionToken !== null,
            });
            expectPaintedColorToResolveFromToken(focused.layers[0].color, target, anchorToken);
            expectPaintedColorToResolveFromToken(focused.layers[1].color, target, '--cem-zebra-color-1');
            if (selectionToken) {
                expectPaintedColorToResolveFromToken(focused.layers[2].color, target, selectionToken);
            }
            expectInputIndicatorStructureAndGeometry(focused, baselines[index]);

            if (index > 0) {
                const previous = cases[index - 1];
                const previousSelectionToken =
                    'selectionToken' in previous ? previous.selectionToken : null;
                const restored = captureInputIndicatorState(
                    runtime,
                    previous.host,
                    previous.control,
                    previous.target,
                );
                expect(previous.control.matches(':focus-visible')).toBe(false);
                expectInputIndicatorGeometry(restored, previous.target, previous.appearance, {
                    selection: previousSelectionToken !== null,
                });
                expectPaintedColorToResolveFromToken(
                    restored.layers[0].color,
                    previous.target,
                    previous.anchorToken,
                );
                if (previousSelectionToken) {
                    expectPaintedColorToResolveFromToken(
                        restored.layers[2].color,
                        previous.target,
                        previousSelectionToken,
                    );
                }
                expectInputIndicatorStructureAndGeometry(restored, baselines[index - 1]);
            }
            for (const disabled of disabledControls) {
                expect(document.activeElement).not.toBe(disabled);
            }
        }

        const switchCase = cases.at(-1);
        const switchBaseline = baselines.at(-1);
        if (!switchCase || !switchBaseline) {
            throw new Error('Expected the switch focus case');
        }
        const focusedSwitch = captureInputIndicatorState(
            runtime,
            switchCase.host,
            switchCase.control,
            switchCase.target,
        );
        await userEvent.hover(switchCase.control);
        await nextRenderFrame();
        const focusedHoveredSwitch = captureInputIndicatorState(
            runtime,
            switchCase.host,
            switchCase.control,
            switchCase.target,
        );
        expectInputIndicatorGeometry(focusedHoveredSwitch, switchCase.target, 'outline', {
            focus: true,
            selection: true,
        });
        expectPaintedColorToResolveFromToken(
            focusedHoveredSwitch.layers[0].color,
            switchCase.target,
            '--cem-input-indicator-anchor-invalid-hover-color',
        );
        expectInputIndicatorStructureAndGeometry(focusedHoveredSwitch, focusedSwitch);
        await userEvent.unhover(switchCase.control);
        await nextRenderFrame();
        expect(
            captureInputIndicatorState(runtime, switchCase.host, switchCase.control, switchCase.target).boxShadow,
        ).toBe(focusedSwitch.boxShadow);

        await userEvent.tab();
        await nextRenderFrame();
        expect(document.activeElement).toBe(end);
        expect(switchCase.control.matches(':focus-visible')).toBe(false);
        const restoredSwitch = captureInputIndicatorState(
            runtime,
            switchCase.host,
            switchCase.control,
            switchCase.target,
        );
        expect(restoredSwitch.boxShadow).toBe(switchBaseline.boxShadow);
        expectInputIndicatorStructureAndGeometry(restoredSwitch, switchBaseline);

        expect(focusOrder).toEqual(
            cases.map(({ control }) =>
                control.closest<HTMLElement>('cem-select')?.getAttribute('name') ?? control.getAttribute('name') ?? '',
            ),
        );
        expect(cases[0].control.value).toBe('alpha');
        expect(cases[2].control.readOnly).toBe(true);
        expect(cases[3].control.getAttribute('aria-invalid')).toBe('true');
        expect(cases[4].control.getAttribute('aria-checked')).toBe('mixed');
        expect(cases[5].control.checked).toBe(true);
        expect(cases[6].control.checked).toBe(true);
        expect(cases[6].control.getAttribute('role')).toBe('switch');
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('projects explicit busy state across every input without taking over its lifecycle', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack class="cem-theme-light" gap="sm">
                <cem-field name="busy-field" label="Generic field" value="alpha"></cem-field>
                <cem-text-field
                    name="busy-text-field"
                    label="Outlined field"
                    value="bravo"
                    indicator="outline"
                    busy="false"
                ></cem-text-field>
                <cem-textarea name="busy-textarea" label="Readonly notes" value="charlie" readonly busy></cem-textarea>
                <cem-select name="busy-select" label="Role" busy>
                    <option value="delta" selected>Delta</option>
                </cem-select>
                <cem-checkbox name="busy-checkbox" busy checked>Checked option</cem-checkbox>
                <cem-radio name="busy-radio" indicator="underline" busy checked>Selected radio</cem-radio>
                <cem-switch name="busy-switch" busy checked invalid="true">Invalid switch</cem-switch>
                <cem-text-field name="busy-disabled" label="Disabled field" busy disabled></cem-text-field>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-text-field[name="busy-disabled"] input');

        const fieldHost = harness.query<HTMLElement>('cem-field[name="busy-field"]');
        const fieldControl = harness.query<HTMLInputElement>('cem-field[name="busy-field"] input');
        const fieldBaseline = captureInputIndicatorState(runtime, fieldHost, fieldControl, fieldControl);
        const cases = [
            {
                anchorToken: '--cem-input-indicator-anchor-pending-color',
                anchorWidthToken: '--cem-stroke-pending',
                appearance: 'underline',
                control: fieldControl,
                host: fieldHost,
                label: 'Generic field',
                target: fieldControl,
            },
            {
                anchorToken: '--cem-input-indicator-anchor-pending-color',
                anchorWidthToken: '--cem-stroke-pending',
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-text-field[name="busy-text-field"] input'),
                host: harness.query<HTMLElement>('cem-text-field[name="busy-text-field"]'),
                label: 'Outlined field',
                target: harness.query<HTMLInputElement>('cem-text-field[name="busy-text-field"] input'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-pending-color',
                anchorWidthToken: '--cem-stroke-pending',
                appearance: 'underline',
                control: harness.query<HTMLTextAreaElement>('cem-textarea[name="busy-textarea"] textarea'),
                host: harness.query<HTMLElement>('cem-textarea[name="busy-textarea"]'),
                label: 'Readonly notes',
                target: harness.query<HTMLTextAreaElement>('cem-textarea[name="busy-textarea"] textarea'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-pending-color',
                anchorWidthToken: '--cem-stroke-pending',
                appearance: 'underline',
                control: harness.query<HTMLButtonElement>('cem-select[name="busy-select"] .cem-select__control'),
                host: harness.query<HTMLElement>('cem-select[name="busy-select"]'),
                label: 'Role',
                target: harness.query<HTMLButtonElement>('cem-select[name="busy-select"] .cem-select__control'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-pending-color',
                anchorWidthToken: '--cem-stroke-pending',
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-checkbox[name="busy-checkbox"] input'),
                host: harness.query<HTMLElement>('cem-checkbox[name="busy-checkbox"]'),
                label: 'Checked option',
                selection: true,
                target: harness.query<HTMLLabelElement>('cem-checkbox[name="busy-checkbox"] > label'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-pending-color',
                anchorWidthToken: '--cem-stroke-pending',
                appearance: 'underline',
                control: harness.query<HTMLInputElement>('cem-radio[name="busy-radio"] input'),
                host: harness.query<HTMLElement>('cem-radio[name="busy-radio"]'),
                label: 'Selected radio',
                selection: true,
                target: harness.query<HTMLLabelElement>('cem-radio[name="busy-radio"] > label'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-invalid-color',
                anchorWidthToken: '--cem-stroke-boundary',
                appearance: 'outline',
                control: harness.query<HTMLInputElement>('cem-switch[name="busy-switch"] input'),
                host: harness.query<HTMLElement>('cem-switch[name="busy-switch"]'),
                label: 'Invalid switch',
                selection: true,
                target: harness.query<HTMLLabelElement>('cem-switch[name="busy-switch"] > label'),
            },
            {
                anchorToken: '--cem-input-indicator-anchor-disabled-color',
                anchorWidthToken: '--cem-stroke-boundary',
                appearance: 'underline',
                control: harness.query<HTMLInputElement>('cem-text-field[name="busy-disabled"] input'),
                host: harness.query<HTMLElement>('cem-text-field[name="busy-disabled"]'),
                label: 'Disabled field',
                target: harness.query<HTMLInputElement>('cem-text-field[name="busy-disabled"] input'),
            },
        ] as const;
        const mutationEvents: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        expect(fieldControl.hasAttribute('data-state')).toBe(false);
        expect(fieldControl.hasAttribute('aria-busy')).toBe(false);
        fieldControl.focus();
        expect(document.activeElement).toBe(fieldControl);
        fieldHost.setAttribute('busy', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(fieldHost);
        await nextRenderFrame();

        const busyField = captureInputIndicatorState(runtime, fieldHost, fieldControl, fieldControl);
        expect(harness.query<HTMLInputElement>('cem-field[name="busy-field"] input')).toBe(fieldControl);
        expect(document.activeElement).toBe(fieldControl);
        expect(busyField.controlRect).toEqual(fieldBaseline.controlRect);
        expect(busyField.hostRect).toEqual(fieldBaseline.hostRect);
        expect(busyField.targetRect).toEqual(fieldBaseline.targetRect);
        expect(busyField.runtime).toBe(fieldBaseline.runtime);

        for (const indicatorCase of cases) {
            const { anchorToken, anchorWidthToken, appearance, control, host, label, target } = indicatorCase;
            const focus = control === fieldControl;
            const selection = 'selection' in indicatorCase && indicatorCase.selection;
            const busy = captureInputIndicatorState(runtime, host, control, target);

            expect(control.getAttribute('data-state')).toBe('loading');
            expect(control.getAttribute('aria-busy')).toBe('true');
            expect(assertAccessibleName(control, label)).toBe(label);
            expect(control.hasAttribute('inert')).toBe(false);
            expectInputIndicatorGeometry(busy, target, appearance, {
                anchorWidthToken,
                focus,
                selection,
            });
            expectPaintedColorToResolveFromToken(busy.layers[0].color, target, anchorToken);
            if (focus) {
                expectPaintedColorToResolveFromToken(busy.layers[1].color, target, '--cem-zebra-color-1');
            }
            if (selection) {
                expectPaintedColorToResolveFromToken(
                    busy.layers[2].color,
                    target,
                    '--cem-input-indicator-selection-color',
                );
            }

            const snapshot = runtime.snapshotInstance(host);
            expect(snapshot.slices).not.toHaveProperty('busy');
            expect(snapshot.slices).not.toHaveProperty('loading');
            expect(snapshot.eventPayloads).not.toHaveProperty('busy');
            expect(snapshot.eventPayloads).not.toHaveProperty('loading');
        }

        expect(cases[1].host.getAttribute('busy')).toBe('false');
        expect(cases[2].control.readOnly).toBe(true);
        expect(cases[6].control.getAttribute('aria-invalid')).toBe('true');
        expect(cases[7].control.disabled).toBe(true);
        expect(cases[0].control.value).toBe('alpha');
        expect(cases[1].control.value).toBe('bravo');
        expect(cases[2].control.value).toBe('charlie');
        expect(cases[3].control.value).toBe('delta');
        expect(cases[4].control.checked).toBe(true);
        expect(cases[5].control.checked).toBe(true);
        expect(cases[6].control.checked).toBe(true);
        expect(harness.root.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();

        for (const { host } of cases) {
            host.removeAttribute('busy');
        }
        await nextRenderFrame();
        for (const { host } of cases) {
            await runtime.whenRenderSettled(host);
        }
        await nextRenderFrame();

        for (const { control, host } of cases) {
            expect(control.hasAttribute('data-state')).toBe(false);
            expect(control.hasAttribute('aria-busy')).toBe(false);
            expect(host.querySelector('input, textarea, select, .cem-select__control')).toBe(control);
        }
        expect(document.activeElement).toBe(fieldControl);
        const settledField = captureInputIndicatorState(runtime, fieldHost, fieldControl, fieldControl);
        expectInputIndicatorGeometry(settledField, fieldControl, 'underline', { focus: true });
        expect(settledField.controlRect).toEqual(fieldBaseline.controlRect);
        expect(settledField.hostRect).toEqual(fieldBaseline.hostRect);
        expect(settledField.targetRect).toEqual(fieldBaseline.targetRect);
        expect(settledField.runtime).toBe(fieldBaseline.runtime);
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('renders rich cem-option content with native-like dropdown, listbox, and form behavior', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <form id="select-form">
                <cem-select id="role-select" name="role" required>
                    <span slot="label">Role</span>
                    <cem-option value="">Choose a role</cem-option>
                    <cem-option-group label="People">
                        <cem-option value="ada" selected><strong>Ada</strong> Lovelace</cem-option>
                        <cem-option value="grace"><strong>Grace</strong> Hopper</cem-option>
                        <cem-option value="disabled" disabled>Unavailable</cem-option>
                    </cem-option-group>
                </cem-select>
                <cem-select id="tier-select" name="tier" size="3" label="Tier">
                    <cem-option value="one">One</cem-option>
                    <cem-option value="two" selected>Two</cem-option>
                    <cem-option value="three">Three</cem-option>
                </cem-select>
                <cem-select id="tag-select" name="tag" multiple size="4" label="Tags">
                    <cem-option value="alpha" selected>Alpha</cem-option>
                    <cem-option value="beta">Beta</cem-option>
                    <cem-option value="gamma" selected>Gamma</cem-option>
                </cem-select>
                <cem-select id="legacy-select" name="legacy" label="Legacy">
                    <option value="admin">Admin</option>
                    <option value="viewer" selected>Viewer</option>
                </cem-select>
                <button id="select-submit" type="submit">Submit</button>
            </form>
        `);
        await waitForStateSelector(root, '#tag-select [role="listbox"]');

        const form = harness.query<HTMLFormElement>('#select-form');
        const role = harness.query<TestCemSelect>('#role-select');
        const roleControl = harness.query<HTMLButtonElement>('#role-select .cem-select__control');
        const tier = harness.query<TestCemSelect>('#tier-select');
        const tierControl = harness.query<HTMLElement>('#tier-select [role="listbox"]');
        const tags = harness.query<TestCemSelect>('#tag-select');
        const tagsControl = harness.query<HTMLElement>('#tag-select [role="listbox"]');
        const legacy = harness.query<TestCemSelect>('#legacy-select');
        const mutationEvents: string[] = [];
        for (const eventName of ['input', 'change']) {
            role.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        expect(role.type).toBe('select-one');
        expect(role.value).toBe('ada');
        expect(role.selectedValues).toEqual(['ada']);
        expect(role.required).toBe(true);
        expect(role.checkValidity()).toBe(true);
        expect(role.form).toBe(form);
        expect(Array.from(new FormData(form).entries())).toEqual([
            ['role', 'ada'],
            ['tier', 'two'],
            ['tag', 'alpha'],
            ['tag', 'gamma'],
            ['legacy', 'viewer'],
        ]);
        expect(assertAccessibleName(roleControl, 'Role')).toBe('Role');
        expect(roleControl.getAttribute('aria-expanded')).toBe('false');
        expect(roleControl.hasAttribute('aria-controls')).toBe(false);
        expect(role.querySelector('cem-option')).toBeNull();

        await userEvent.click(roleControl);
        await nextRenderFrame();
        await runtime.whenRenderSettled(role);
        const popup = harness.query<HTMLElement>('#role-select .cem-select__popup');
        expect(roleControl.getAttribute('aria-expanded')).toBe('true');
        expect(roleControl.getAttribute('aria-controls')).toBe(popup.id);
        expect(popup.querySelector('[role="group"]')?.getAttribute('aria-label')).toBe('People');
        expect(popup.querySelector('[role="option"] strong')?.textContent).toBe('Ada');
        expect(popup.querySelectorAll('[role="option"]')).toHaveLength(4);
        const disabledOption = popup.querySelector<HTMLElement>('[role="option"][aria-disabled="true"]');
        expect(disabledOption?.textContent?.trim()).toBe('Unavailable');
        await userEvent.keyboard('{End}');
        await nextRenderFrame();
        const activeOption = role.querySelector<HTMLElement>(`#${roleControl.getAttribute('aria-activedescendant')}`);
        expect(activeOption?.textContent?.trim()).toBe('Grace Hopper');
        expect(role.value).toBe('ada');
        expect(roleControl.getAttribute('aria-expanded')).toBe('true');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();

        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();
        expect(role.value).toBe('ada');
        await userEvent.keyboard('{Escape}');
        await nextRenderFrame();
        expect(role.value).toBe('ada');
        expect(roleControl.getAttribute('aria-expanded')).toBe('false');
        expect(mutationEvents).toEqual([]);

        await userEvent.click(roleControl);
        await userEvent.keyboard('{ArrowDown}');
        await userEvent.keyboard('{Tab}');
        await nextRenderFrame();
        expect(role.value).toBe('grace');
        expect(mutationEvents).toEqual(['input', 'change']);

        role.value = 'ada';
        await nextRenderFrame();
        expect(role.value).toBe('ada');
        expect(mutationEvents).toEqual(['input', 'change']);

        await userEvent.click(roleControl);
        await userEvent.keyboard('{ArrowDown}');
        await userEvent.keyboard('{Enter}');
        await nextRenderFrame();
        expect(role.value).toBe('grace');
        expect(role.selectedValues).toEqual(['grace']);
        expect(mutationEvents).toEqual(['input', 'change', 'input', 'change']);
        expect(new FormData(form).get('role')).toBe('grace');

        role.setSelectedValues(['']);
        await nextRenderFrame();
        expect(role.value).toBe('');
        expect(role.validity.valueMissing).toBe(true);
        expect(role.validationMessage).not.toBe('');
        expect(mutationEvents).toEqual(['input', 'change', 'input', 'change']);
        form.reset();
        await nextRenderFrame();
        expect(role.value).toBe('ada');
        expect(role.checkValidity()).toBe(true);

        expect(tier.type).toBe('select-one');
        expect(tier.size).toBe(3);
        expect(tierControl.getAttribute('aria-multiselectable')).toBeNull();
        tierControl.focus();
        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();
        expect(tier.value).toBe('three');

        expect(tags.type).toBe('select-multiple');
        expect(tags.multiple).toBe(true);
        expect(tags.selectedValues).toEqual(['alpha', 'gamma']);
        expect(tagsControl.getAttribute('aria-multiselectable')).toBe('true');
        tagsControl.focus();
        await userEvent.keyboard('{ArrowDown}');
        await userEvent.keyboard(' ');
        await nextRenderFrame();
        expect(tags.selectedValues).toEqual(['alpha', 'beta', 'gamma']);
        expect(new FormData(form).getAll('tag')).toEqual(['alpha', 'beta', 'gamma']);

        expect(legacy.value).toBe('viewer');
        expect(legacy.selectedValues).toEqual(['viewer']);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('reflects form disabled, invalid, required, readonly, checked, and indeterminate states', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <section>
                <p id="email-help">Use a work address.</p>
                <p id="email-error">Email is required.</p>
                <p id="role-help">Choose the closest role.</p>
                <p id="role-error">Role is required.</p>
                <cem-stack gap="sm">
                    <cem-text-field
                        name="email"
                        value="a@b.test"
                        label="Email"
                        required
                        readonly
                        invalid="true"
                        describedby="email-help"
                        error="email-error"
                    ></cem-text-field>
                    <cem-textarea name="notes" label="Notes" disabled invalid="true"></cem-textarea>
                    <cem-select
                        name="role"
                        label="Role"
                        required
                        invalid="true"
                        describedby="role-help"
                        error="role-error"
                    >
                        <option value="admin">Admin</option>
                        <option value="viewer">Viewer</option>
                    </cem-select>
                    <cem-checkbox name="terms" checked required invalid="true">Accept terms</cem-checkbox>
                    <cem-checkbox name="partial" indeterminate="mixed">Partially selected</cem-checkbox>
                    <cem-radio name="plan" value="pro" checked disabled>Pro plan</cem-radio>
                    <cem-switch name="public-profile" checked>Public profile</cem-switch>
                </cem-stack>
            </section>
        `);
        await waitForStateSelector(root, 'cem-switch input');

        const textField = harness.query<HTMLInputElement>('cem-text-field input');
        const textarea = harness.query<HTMLTextAreaElement>('cem-textarea textarea');
        const select = harness.query<HTMLButtonElement>('cem-select .cem-select__control');
        const selectHost = harness.query<HTMLElement & { required: boolean }>('cem-select');
        const checkedBox = harness.query<HTMLInputElement>('cem-checkbox input[name="terms"]');
        const mixedBox = harness.query<HTMLInputElement>('cem-checkbox input[name="partial"]');
        const radio = harness.query<HTMLInputElement>('cem-radio input');
        const switchInput = harness.query<HTMLInputElement>('cem-switch input');

        assertStateHostsRendered(
            harness.root,
            'cem-text-field, cem-textarea, cem-select, cem-checkbox, cem-radio, cem-switch',
        );
        expect(textField.required).toBe(true);
        expect(textField.readOnly).toBe(true);
        expect(textField.getAttribute('aria-invalid')).toBe('true');
        expect(textField.getAttribute('aria-describedby')).toBe('email-help');
        expect(textField.getAttribute('aria-errormessage')).toBe('email-error');
        expect(assertAccessibleName(textField, 'Email')).toBe('Email');
        expect(textarea.disabled).toBe(true);
        expect(textarea.getAttribute('aria-invalid')).toBe('true');
        expect(selectHost.required).toBe(true);
        expect(select.getAttribute('aria-describedby')).toBe('role-help');
        expect(select.getAttribute('aria-errormessage')).toBe('role-error');
        expect(checkedBox.checked).toBe(true);
        expect(checkedBox.required).toBe(true);
        expect(checkedBox.getAttribute('aria-invalid')).toBe('true');
        expect(mixedBox.getAttribute('aria-checked')).toBe('mixed');
        expect(radio.checked).toBe(true);
        expect(radio.disabled).toBe(true);
        expect(switchInput.checked).toBe(true);
        expect(switchInput.getAttribute('role')).toBe('switch');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('captures serializable slice-event payloads for text and boolean controls', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-text-field name="query" value="draft" label="Query"></cem-text-field>
                <cem-checkbox name="enabled">Enabled</cem-checkbox>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-checkbox input');

        const fieldHost = harness.query<HTMLElement>('cem-text-field');
        const checkboxHost = harness.query<HTMLElement>('cem-checkbox');
        const input = harness.query<HTMLInputElement>('cem-text-field input');

        input.value = 'published';
        input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
        await nextRenderFrame();

        const textSnapshot = runtime.snapshotInstance(fieldHost);
        const textPayload = eventPayload(textSnapshot, 'value');
        expect(textSnapshot.slices.value).toBe('published');
        expect(textPayload).toMatchObject({
            bubbles: true,
            sliceValue: 'published',
            type: 'input',
        });
        expect(textPayload.target).toMatchObject({
            name: 'query',
            tag: 'input',
            type: 'text',
            value: 'published',
        });

        const nextCheckbox = harness.query<HTMLInputElement>('cem-checkbox input');
        nextCheckbox.checked = true;
        nextCheckbox.dispatchEvent(new Event('change', { bubbles: true, composed: true }));
        await nextRenderFrame();

        const checkboxSnapshot = runtime.snapshotInstance(checkboxHost);
        const checkboxPayload = eventPayload(checkboxSnapshot, 'checked');
        expect(checkboxSnapshot.slices.checked).toBe(true);
        expect(checkboxPayload).toMatchObject({
            bubbles: true,
            sliceValue: true,
            type: 'change',
        });
        expect(checkboxPayload.target).toMatchObject({
            checked: true,
            name: 'enabled',
            tag: 'input',
            type: 'checkbox',
        });
        expect(nextCheckbox.isConnected).toBe(true);
        expect(harness.query<HTMLInputElement>('cem-checkbox input')).toBe(nextCheckbox);
    });

    it('toggles checkable content chips without changing passive chip semantics', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-chip label="Static topic">Static topic</cem-chip>
                <cem-chip label="Owner filter" checkable>Owner</cem-chip>
                <cem-chip label="Ready filter" checkable checked>Ready</cem-chip>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-chip[checkable] button');

        const passive = harness.query<HTMLElement>('cem-chip:not([checkable]) .cem-chip');
        const uncheckedHost = harness.query<HTMLElement>('cem-chip[checkable]:not([checked])');
        const unchecked = harness.query<HTMLButtonElement>('cem-chip[checkable]:not([checked]) button');
        const checked = harness.query<HTMLButtonElement>('cem-chip[checkable][checked] button');

        expect(passive).toBeInstanceOf(HTMLSpanElement);
        expect(passive.hasAttribute('aria-pressed')).toBe(false);
        expect(passive.tabIndex).toBe(-1);
        expect(unchecked).toBeInstanceOf(HTMLButtonElement);
        expect(unchecked.type).toBe('button');
        expect(assertAccessibleName(unchecked, 'Owner filter')).toBe('Owner filter');
        expect(unchecked.getAttribute('aria-pressed')).toBe('false');
        expect(checked.getAttribute('aria-pressed')).toBe('true');
        await assertFocusVisible(unchecked);

        unchecked.click();
        await nextRenderFrame();

        const pressed = harness.query<HTMLButtonElement>('cem-chip[checkable]:not([checked]) button');
        const pressedSnapshot = runtime.snapshotInstance(uncheckedHost);
        const pressedPayload = eventPayload(pressedSnapshot, 'checked');
        expect(pressed.getAttribute('aria-pressed')).toBe('true');
        expect(pressedSnapshot.slices.checked).toBe(true);
        expect(pressedPayload).toMatchObject({
            bubbles: true,
            sliceValue: true,
            type: 'click',
        });
        expect(pressedPayload.target).toMatchObject({
            tag: 'button',
            type: 'button',
        });

        pressed.click();
        await nextRenderFrame();

        const released = harness.query<HTMLButtonElement>('cem-chip[checkable]:not([checked]) button');
        const releasedSnapshot = runtime.snapshotInstance(uncheckedHost);
        expect(released.getAttribute('aria-pressed')).toBe('false');
        expect(releasedSnapshot.slices.checked).toBe(false);
        expect(eventPayload(releasedSnapshot, 'checked').sliceValue).toBe(false);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('styles only interactive content hover owners without changing selection or component state', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack class="cem-theme-light" gap="sm">
                <cem-list id="passive-content-list" label="Static topics">
                    <li>Static topic</li>
                </cem-list>
                <cem-list id="interactive-content-list" label="Asset type" selectable value="document" size="3">
                    <cem-list-option value="image">Image</cem-list-option>
                    <cem-list-option value="document" selected>Document</cem-list-option>
                    <cem-list-option value="archive" disabled>Archive</cem-list-option>
                </cem-list>
                <cem-list id="disabled-content-list" label="Unavailable asset type" selectable size="2">
                    <cem-list-option value="image" selected>Image</cem-list-option>
                    <cem-list-option value="document">Document</cem-list-option>
                </cem-list>
                <cem-chip id="passive-content-chip">Passive chip</cem-chip>
                <cem-chip id="unchecked-content-chip" checkable>Unchecked chip</cem-chip>
                <cem-chip id="checked-content-chip" checkable checked>Checked chip</cem-chip>
                <cem-chip id="disabled-content-chip" checkable checked>Unavailable chip</cem-chip>
                <cem-table id="passive-content-table" label="Static comparison">
                    <div role="row"><span role="cell">Static cell</span></div>
                </cem-table>
            </cem-stack>
        `);
        await waitForStateSelector(root, '#passive-content-table > .cem-table');

        const listHost = harness.query<HTMLElement>('#interactive-content-list');
        const listOwner = harness.query<HTMLSelectElement>('#interactive-content-list > select');
        const uncheckedHost = harness.query<HTMLElement>('#unchecked-content-chip');
        const uncheckedOwner = harness.query<HTMLButtonElement>('#unchecked-content-chip > button');
        const checkedHost = harness.query<HTMLElement>('#checked-content-chip');
        const checkedOwner = harness.query<HTMLButtonElement>('#checked-content-chip > button');
        const disabledListHost = harness.query<HTMLElement>('#disabled-content-list');
        const disabledListOwner = harness.query<HTMLSelectElement>('#disabled-content-list > select');
        const disabledChipHost = harness.query<HTMLElement>('#disabled-content-chip');
        const disabledChipOwner = harness.query<HTMLButtonElement>('#disabled-content-chip > button');
        disabledListOwner.disabled = true;
        disabledChipOwner.disabled = true;

        const interactiveCases = [
            {
                host: listHost,
                owner: listOwner,
                tokens: {
                    defaultBackground: '--cem-content-interaction-default-background',
                    defaultText: '--cem-content-interaction-default-text',
                    hoverBackground: '--cem-content-interaction-hover-background',
                    hoverText: '--cem-content-interaction-hover-text',
                },
            },
            {
                host: uncheckedHost,
                owner: uncheckedOwner,
                tokens: {
                    defaultBackground: '--cem-content-interaction-default-background',
                    defaultText: '--cem-content-interaction-default-text',
                    hoverBackground: '--cem-content-interaction-hover-background',
                    hoverText: '--cem-content-interaction-hover-text',
                },
            },
            {
                host: checkedHost,
                owner: checkedOwner,
                tokens: {
                    defaultBackground: '--cem-content-interaction-selected-background',
                    defaultText: '--cem-content-interaction-selected-text',
                    hoverBackground: '--cem-content-interaction-selected-hover-background',
                    hoverText: '--cem-content-interaction-selected-hover-text',
                },
            },
        ] as const;
        const disabledCases = [
            { host: disabledListHost, owner: disabledListOwner },
            { host: disabledChipHost, owner: disabledChipOwner },
        ] as const;
        const passiveCases = [
            {
                host: harness.query<HTMLElement>('#passive-content-list'),
                owner: harness.query<HTMLElement>('#passive-content-list > ul'),
            },
            {
                host: harness.query<HTMLElement>('#passive-content-chip'),
                owner: harness.query<HTMLElement>('#passive-content-chip > span'),
            },
            {
                host: harness.query<HTMLElement>('#passive-content-table'),
                owner: harness.query<HTMLElement>('#passive-content-table > .cem-table'),
            },
        ] as const;
        const mutationEvents: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        assertStateHostsRendered(harness.root, 'cem-list, cem-chip, cem-table');
        expect(listOwner.value).toBe('document');
        expect(listOwner.options[1]?.getAttribute('aria-selected')).toBe('true');
        expect(listOwner.options[2]?.disabled).toBe(true);
        expect(uncheckedOwner.getAttribute('aria-pressed')).toBe('false');
        expect(checkedOwner.getAttribute('aria-pressed')).toBe('true');

        for (const contentCase of interactiveCases) {
            const { host, owner, tokens } = contentCase;
            const pointerEvents: string[] = [];
            owner.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
            owner.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
            await assertFocusVisible(owner);

            const baseline = captureContentInteractionState(runtime, host, owner);
            expect(baseline.backgroundColor).toBe(resolveTokenColor(owner, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(owner, tokens.defaultText));

            await userEvent.hover(owner);
            await nextRenderFrame();

            const hovered = captureContentInteractionState(runtime, host, owner);
            expect(owner.matches(':hover')).toBe(true);
            expect(hovered.backgroundColor).toBe(resolveTokenColor(owner, tokens.hoverBackground));
            expect(hovered.color).toBe(resolveTokenColor(owner, tokens.hoverText));
            expect(hovered.backgroundColor).not.toBe(baseline.backgroundColor);
            expect(contrastRatio(hovered.backgroundColor, hovered.color)).toBeGreaterThanOrEqual(4.5);
            expectContentInteractionStructureAndGeometry(hovered, baseline);
            expect(hovered.focusTreatment).toEqual(baseline.focusTreatment);
            expect(hovered.hostBackgroundColor).toBe(baseline.hostBackgroundColor);
            expect(document.activeElement).toBe(owner);

            await userEvent.unhover(owner);
            await nextRenderFrame();

            const restored = captureContentInteractionState(runtime, host, owner);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectContentInteractionStructureAndGeometry(restored, baseline);
            expect(restored.focusTreatment).toEqual(baseline.focusTreatment);
            expect(restored.hostBackgroundColor).toBe(baseline.hostBackgroundColor);
            expect(document.activeElement).toBe(owner);
            expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        }

        for (const disabledCase of disabledCases) {
            const { host, owner } = disabledCase;
            const pointerEvents: string[] = [];
            owner.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
            owner.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
            const focusOwner = document.activeElement;
            owner.focus();
            expect(document.activeElement).toBe(focusOwner);

            const baseline = captureContentInteractionState(runtime, host, owner);
            expect(baseline.backgroundColor).toBe(
                resolveTokenColor(owner, '--cem-content-interaction-disabled-background'),
            );
            expect(baseline.color).toBe(resolveTokenColor(owner, '--cem-content-interaction-disabled-text'));

            await userEvent.hover(owner);
            await nextRenderFrame();

            const hovered = captureContentInteractionState(runtime, host, owner);
            expect(owner.matches(':hover')).toBe(true);
            expect(hovered.backgroundColor).toBe(baseline.backgroundColor);
            expect(hovered.color).toBe(baseline.color);
            expectContentInteractionStructureAndGeometry(hovered, baseline);
            expect(hovered.hostBackgroundColor).toBe(baseline.hostBackgroundColor);
            expect(document.activeElement).toBe(focusOwner);

            await userEvent.unhover(owner);
            await nextRenderFrame();

            const restored = captureContentInteractionState(runtime, host, owner);
            expectContentInteractionStructureAndGeometry(restored, baseline);
            expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        }

        for (const passiveCase of passiveCases) {
            const { host, owner } = passiveCase;
            const pointerEvents: string[] = [];
            owner.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
            owner.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
            const baseline = captureContentInteractionState(runtime, host, owner);

            await userEvent.hover(owner);
            await nextRenderFrame();
            const hovered = captureContentInteractionState(runtime, host, owner);
            expect(owner.matches(':hover')).toBe(true);
            expect(hovered.backgroundColor).toBe(baseline.backgroundColor);
            expect(hovered.color).toBe(baseline.color);
            expectContentInteractionStructureAndGeometry(hovered, baseline);

            await userEvent.unhover(owner);
            await nextRenderFrame();
            expectContentInteractionStructureAndGeometry(
                captureContentInteractionState(runtime, host, owner),
                baseline,
            );
            expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        }

        expect(listOwner.value).toBe('document');
        expect(listOwner.options[1]?.getAttribute('aria-selected')).toBe('true');
        expect(listOwner.options[2]?.disabled).toBe(true);
        expect(uncheckedOwner.getAttribute('aria-pressed')).toBe('false');
        expect(checkedOwner.getAttribute('aria-pressed')).toBe('true');
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('toggles collapsible navigation without changing passive landmark semantics', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <form>
                <cem-stack gap="sm">
                    <cem-nav label="Primary navigation">
                        <a href="#overview">Overview</a>
                    </cem-nav>
                    <cem-nav label="Workspace navigation" collapsible>
                        <a href="#workspace">Workspace</a>
                    </cem-nav>
                    <cem-nav label="Admin navigation" collapsible expanded>
                        <a href="#admin">Admin</a>
                    </cem-nav>
                </cem-stack>
            </form>
        `);
        await waitForStateSelector(root, 'cem-nav[collapsible] button');

        const form = harness.query<HTMLFormElement>('form');
        const passiveNav = harness.query<HTMLElement>('cem-nav:not([collapsible]) nav');
        const passiveLink = harness.query<HTMLAnchorElement>('cem-nav:not([collapsible]) a');
        const closedHost = harness.query<HTMLElement>('cem-nav[collapsible]:not([expanded])');
        const closedNav = harness.query<HTMLElement>('cem-nav[collapsible]:not([expanded]) nav');
        const closedButton = harness.query<HTMLButtonElement>('cem-nav[collapsible]:not([expanded]) button');
        const closedContent = harness.query<HTMLDivElement>('cem-nav[collapsible]:not([expanded]) .cem-nav__content');
        const closedLink = harness.query<HTMLAnchorElement>('cem-nav[collapsible]:not([expanded]) a');
        const openNav = harness.query<HTMLElement>('cem-nav[collapsible][expanded] nav');
        const openButton = harness.query<HTMLButtonElement>('cem-nav[collapsible][expanded] button');
        const openContent = harness.query<HTMLDivElement>('cem-nav[collapsible][expanded] .cem-nav__content');

        expect(passiveNav.children).toHaveLength(1);
        expect(passiveNav.firstElementChild).toBe(passiveLink);
        expect(passiveNav.querySelector('button, .cem-nav__content')).toBeNull();
        expect(assertAccessibleName(passiveNav, 'Primary navigation')).toBe('Primary navigation');
        expect(assertAccessibleName(closedNav, 'Workspace navigation')).toBe('Workspace navigation');
        expect(assertAccessibleName(openNav, 'Admin navigation')).toBe('Admin navigation');
        expect(closedButton).toBeInstanceOf(HTMLButtonElement);
        expect(closedButton.type).toBe('button');
        expect(closedButton.name).toBe('');
        expect(assertAccessibleName(closedButton, 'Workspace navigation')).toBe('Workspace navigation');
        expect(assertAccessibleName(openButton, 'Admin navigation')).toBe('Admin navigation');
        expect(closedButton.getAttribute('aria-expanded')).toBe('false');
        expect(openButton.getAttribute('aria-expanded')).toBe('true');
        expect(closedContent.hidden).toBe(true);
        expect(openContent.hidden).toBe(false);
        expect(
            Array.from(form.querySelectorAll('button')).every((button) => !button.hasAttribute('aria-controls')),
        ).toBe(true);
        expect(form.querySelector('[role="menu"], [role="menubar"], [role="menuitem"], [aria-haspopup]')).toBeNull();
        expect(form.querySelector('details, summary')).toBeNull();
        expect(Array.from(new FormData(form).entries())).toEqual([]);

        await userEvent.click(closedButton);
        await nextRenderFrame();

        const pointerButton = harness.query<HTMLButtonElement>('cem-nav[collapsible]:not([expanded]) button');
        const pointerContent = harness.query<HTMLDivElement>('cem-nav[collapsible]:not([expanded]) .cem-nav__content');
        const pointerSnapshot = runtime.snapshotInstance(closedHost);
        expect(pointerButton).toBe(closedButton);
        expect(pointerContent).toBe(closedContent);
        expect(pointerButton.getAttribute('aria-expanded')).toBe('true');
        expect(pointerContent.hidden).toBe(false);
        expect(document.activeElement).toBe(pointerButton);
        expect(pointerSnapshot.slices.expanded).toBe(true);
        expect(eventPayload(pointerSnapshot, 'expanded')).toMatchObject({
            bubbles: true,
            sliceValue: true,
            type: 'click',
        });
        expect(eventPayload(pointerSnapshot, 'expanded').target).toMatchObject({
            tag: 'button',
            type: 'button',
        });

        await userEvent.tab();
        expect(document.activeElement).toBe(closedLink);

        pointerButton.focus();
        await userEvent.keyboard('{Enter}');
        await nextRenderFrame();

        const enterButton = harness.query<HTMLButtonElement>('cem-nav[collapsible]:not([expanded]) button');
        const enterContent = harness.query<HTMLDivElement>('cem-nav[collapsible]:not([expanded]) .cem-nav__content');
        const enterSnapshot = runtime.snapshotInstance(closedHost);
        expect(enterButton).toBe(closedButton);
        expect(enterContent).toBe(closedContent);
        expect(enterButton.getAttribute('aria-expanded')).toBe('false');
        expect(enterContent.hidden).toBe(true);
        expect(document.activeElement).toBe(enterButton);
        expect(enterSnapshot.slices.expanded).toBe(false);
        expect(eventPayload(enterSnapshot, 'expanded').sliceValue).toBe(false);

        await userEvent.tab();
        expect(document.activeElement).toBe(openButton);

        enterButton.focus();
        await userEvent.keyboard(' ');
        await nextRenderFrame();

        const spaceButton = harness.query<HTMLButtonElement>('cem-nav[collapsible]:not([expanded]) button');
        const spaceContent = harness.query<HTMLDivElement>('cem-nav[collapsible]:not([expanded]) .cem-nav__content');
        const spaceSnapshot = runtime.snapshotInstance(closedHost);
        expect(spaceButton).toBe(closedButton);
        expect(spaceContent).toBe(closedContent);
        expect(spaceButton.getAttribute('aria-expanded')).toBe('true');
        expect(spaceContent.hidden).toBe(false);
        expect(document.activeElement).toBe(spaceButton);
        expect(spaceSnapshot.slices.expanded).toBe(true);
        expect(eventPayload(spaceSnapshot, 'expanded').sliceValue).toBe(true);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('selects declarative list options without changing passive list semantics', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-list label="Static topics">
                    <li>Static topic</li>
                </cem-list>
                <cem-list label="Asset type" selectable value="document" size="3">
                    <cem-list-option value="image" selected>Image</cem-list-option>
                    <cem-list-option value="document">Document</cem-list-option>
                    <cem-list-option value="archive" disabled>Archive</cem-list-option>
                    <span value="ignored">Ignored non-option</span>
                    <div><cem-list-option value="nested">Ignored nested option</cem-list-option></div>
                </cem-list>
                <cem-list label="Fallback type" selectable size="3">
                    <cem-list-option value="image" selected>Image</cem-list-option>
                    <cem-list-option value="document" selected>Document</cem-list-option>
                </cem-list>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-list[selectable] select');

        const passive = harness.query<HTMLUListElement>('cem-list:not([selectable]) ul');
        const listHost = harness.query<HTMLElement>('cem-list[selectable][value]');
        const listbox = harness.query<HTMLSelectElement>('cem-list[selectable][value] select');
        const fallback = harness.query<HTMLSelectElement>('cem-list[selectable]:not([value]) select');
        const options = Array.from(listbox.options);

        expect(passive).toBeInstanceOf(HTMLUListElement);
        expect(passive.textContent?.trim()).toBe('Static topic');
        expect(listbox).toBeInstanceOf(HTMLSelectElement);
        expect(listbox.size).toBe(3);
        expect(listbox.multiple).toBe(false);
        expect(listbox.name).toBe('');
        expect(assertAccessibleName(listbox, 'Asset type')).toBe('Asset type');
        expect(options.map((option) => `${option.value}:${option.text}`).join('|')).toBe(
            'image:Image|document:Document|archive:Archive',
        );
        expect(options.map((option) => option.selected).join('|')).toBe('false|true|false');
        expect(options.map((option) => option.getAttribute('aria-selected')).join('|')).toBe('false|true|false');
        expect(options[2]?.disabled).toBe(true);
        expect(fallback.value).toBe('document');
        expect(
            Array.from(fallback.options)
                .map((option) => option.selected)
                .join('|'),
        ).toBe('false|true');
        expect(
            Array.from(fallback.options)
                .map((option) => option.getAttribute('aria-selected'))
                .join('|'),
        ).toBe('false|true');

        await userEvent.selectOptions(listbox, 'image');
        await nextRenderFrame();

        const pointerSelected = harness.query<HTMLSelectElement>('cem-list[selectable][value] select');
        const pointerSnapshot = runtime.snapshotInstance(listHost);
        const pointerPayload = eventPayload(pointerSnapshot, 'value');
        expect(pointerSelected.value).toBe('image');
        expect(pointerSelected.options[0]?.selected).toBe(true);
        expect(pointerSelected.options[0]?.getAttribute('aria-selected')).toBe('true');
        expect(pointerSnapshot.slices.value).toBe('image');
        expect(pointerPayload).toMatchObject({
            bubbles: true,
            sliceValue: 'image',
            type: 'change',
        });
        expect(pointerPayload.target).toMatchObject({
            tag: 'select',
            value: 'image',
        });

        await assertFocusVisible(pointerSelected);
        expect(document.activeElement).toBe(pointerSelected);
        expect(pointerSelected.selectedOptions[0]?.matches(':focus')).toBe(false);
        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();

        const keyboardSelected = harness.query<HTMLSelectElement>('cem-list[selectable][value] select');
        const keyboardSnapshot = runtime.snapshotInstance(listHost);
        expect(keyboardSelected.value).toBe('document');
        expect(keyboardSelected.options[1]?.getAttribute('aria-selected')).toBe('true');
        expect(keyboardSnapshot.slices.value).toBe('document');
        expect(eventPayload(keyboardSnapshot, 'value').sliceValue).toBe('document');

        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();

        const disabledSkipped = harness.query<HTMLSelectElement>('cem-list[selectable][value] select');
        expect(disabledSkipped.value).toBe('document');
        expect(disabledSkipped.options[2]?.disabled).toBe(true);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('marks explicit busy cards without making nested content primitives loading owners', async () => {
        const authoredFallback = document.createElement('cem-card');
        authoredFallback.setAttribute('label', 'Assets');
        authoredFallback.setAttribute('busy', '');
        authoredFallback.innerHTML = `
            <span slot="title">Assets</span>
            <p>Loading assets…</p>
            <cem-skeleton label="Asset rows"></cem-skeleton>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('Loading assets…');
        expect(authoredFallback.querySelector('cem-skeleton')?.getAttribute('label')).toBe('Asset rows');

        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-card label="Ordinary card">
                    <span slot="title">Ordinary card</span>
                    <p>Ready</p>
                </cem-card>
                <cem-card label="Initial assets" busy>
                    <span slot="title">Initial assets</span>
                    <p class="loading-message">Loading assets…</p>
                    <cem-skeleton label="Asset rows"></cem-skeleton>
                    <cem-skeleton label="Asset preview"></cem-skeleton>
                </cem-card>
                <cem-card label="Profile" busy="false">
                    <span slot="title">Profile</span>
                    <p>Grace Hopper</p>
                    <button type="button">Edit profile</button>
                </cem-card>
                <cem-card label="Empty assets" busy>
                    <span slot="title">Empty assets</span>
                    <cem-list label="Assets"></cem-list>
                </cem-card>
                <cem-list id="busy-list" label="Standalone list" busy></cem-list>
                <cem-table id="busy-table" label="Standalone table" busy></cem-table>
                <cem-media-preview id="busy-preview" label="Standalone preview" busy></cem-media-preview>
            </cem-stack>
        `);
        await waitForStateSelector(root, '#busy-preview > .cem-media-preview');

        const ordinary = harness.query<HTMLElement>('cem-card[label="Ordinary card"] > section');
        const initialHost = harness.query<HTMLElement>('cem-card[label="Initial assets"]');
        const initial = harness.query<HTMLElement>('cem-card[label="Initial assets"] > section');
        const initialHeader = harness.query<HTMLElement>('cem-card[label="Initial assets"] > section > header');
        const initialBody = harness.query<HTMLElement>('cem-card[label="Initial assets"] .cem-card__body');
        const initialMessage = harness.query<HTMLParagraphElement>('cem-card[label="Initial assets"] .loading-message');
        const skeletons = Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-card[label="Initial assets"] cem-skeleton .cem-skeleton'),
        );
        const refreshHost = harness.query<HTMLElement>('cem-card[label="Profile"]');
        const refreshCard = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const refreshHeader = harness.query<HTMLElement>('cem-card[label="Profile"] > section > header');
        const refreshBody = harness.query<HTMLElement>('cem-card[label="Profile"] .cem-card__body');
        const refreshButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const emptyHost = harness.query<HTMLElement>('cem-card[label="Empty assets"]');
        const emptyCard = harness.query<HTMLElement>('cem-card[label="Empty assets"] > section');
        const emptyList = harness.query<HTMLUListElement>('cem-card[label="Empty assets"] cem-list ul');
        const standaloneList = harness.query<HTMLUListElement>('#busy-list > ul');
        const standaloneTable = harness.query<HTMLElement>('#busy-table > [role="table"]');
        const standalonePreview = harness.query<HTMLElement>('#busy-preview > .cem-media-preview');

        assertStateHostsRendered(
            harness.root,
            'cem-card, #busy-list, #busy-table, #busy-preview, cem-card cem-skeleton',
        );
        expect(ordinary.className).toBe('cem-card');
        expect(ordinary.getAttribute('aria-label')).toBe('Ordinary card');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.hasAttribute('aria-busy')).toBe(false);
        expect(ordinary.querySelector('.cem-card__header')?.textContent?.trim()).toBe('Ordinary card');
        expect(ordinary.querySelector('.cem-card__body')?.textContent?.trim()).toBe('Ready');

        expect(assertAccessibleName(initial, 'Initial assets')).toBe('Initial assets');
        expect(initial.getAttribute('data-state')).toBe('loading');
        expect(initial.getAttribute('aria-busy')).toBe('true');
        expect(initialHeader.className).toBe('cem-card__header');
        expect(initialBody.className).toBe('cem-card__body');
        expect(initialMessage.textContent?.trim()).toBe('Loading assets…');
        expect(initialMessage.getAttribute('role')).toBeNull();
        expect(initialMessage.getAttribute('aria-live')).toBeNull();
        expect(skeletons).toHaveLength(2);
        expect(skeletons.every((skeleton) => skeleton.getAttribute('aria-hidden') === 'true')).toBe(true);
        expect(initial.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(initial.hasAttribute('inert')).toBe(false);

        expect(refreshCard.getAttribute('data-state')).toBe('loading');
        expect(refreshCard.getAttribute('aria-busy')).toBe('true');
        expect(refreshCard.textContent).toContain('Grace Hopper');
        expect(refreshButton.disabled).toBe(false);
        expect(refreshButton.tabIndex).toBe(0);
        expect(emptyCard.getAttribute('data-state')).toBe('loading');
        expect(emptyList.querySelector('.cem-list__empty')?.textContent?.trim()).toBe('No items');

        for (const candidate of [standaloneList, standaloneTable, standalonePreview]) {
            expect(candidate.hasAttribute('data-state')).toBe(false);
            expect(candidate.hasAttribute('aria-busy')).toBe(false);
        }

        const refreshRect = refreshCard.getBoundingClientRect();
        expect(refreshRect.width).toBeGreaterThan(0);
        expect(refreshRect.height).toBeGreaterThan(0);
        const lifecycleEvents: string[] = [];
        for (const name of ['cem-loaded', 'cem-error', 'cem-cancel']) {
            refreshHost.addEventListener(name, () => lifecycleEvents.push(name));
        }

        await assertFocusVisible(refreshButton);
        refreshHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const settledCard = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const settledHeader = harness.query<HTMLElement>('cem-card[label="Profile"] > section > header');
        const settledBody = harness.query<HTMLElement>('cem-card[label="Profile"] .cem-card__body');
        const settledButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const settledRect = settledCard.getBoundingClientRect();
        expect(settledCard).toBe(refreshCard);
        expect(settledHeader).toBe(refreshHeader);
        expect(settledBody).toBe(refreshBody);
        expect(settledButton).toBe(refreshButton);
        expect(settledCard.hasAttribute('data-state')).toBe(false);
        expect(settledCard.hasAttribute('aria-busy')).toBe(false);
        expect(settledRect.width).toBe(refreshRect.width);
        expect(settledRect.height).toBe(refreshRect.height);
        expect(document.activeElement).toBe(settledButton);

        refreshHost.setAttribute('busy', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const pendingAgain = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const pendingButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const pendingRect = pendingAgain.getBoundingClientRect();
        expect(pendingAgain).toBe(refreshCard);
        expect(pendingButton).toBe(refreshButton);
        expect(pendingAgain.getAttribute('data-state')).toBe('loading');
        expect(pendingAgain.getAttribute('aria-busy')).toBe('true');
        expect(pendingRect.width).toBe(refreshRect.width);
        expect(pendingRect.height).toBe(refreshRect.height);
        expect(document.activeElement).toBe(pendingButton);

        emptyHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const settledEmptyCard = harness.query<HTMLElement>('cem-card[label="Empty assets"] > section');
        const settledEmptyList = harness.query<HTMLUListElement>('cem-card[label="Empty assets"] cem-list ul');
        expect(settledEmptyCard).toBe(emptyCard);
        expect(settledEmptyCard.hasAttribute('data-state')).toBe(false);
        expect(settledEmptyCard.hasAttribute('aria-busy')).toBe(false);
        expect(settledEmptyList).toBe(emptyList);
        expect(settledEmptyList.querySelector('.cem-list__empty')?.textContent?.trim()).toBe('No items');

        const initialSnapshot = runtime.snapshotInstance(initialHost);
        const refreshSnapshot = runtime.snapshotInstance(refreshHost);
        expect(initialSnapshot.slices).not.toHaveProperty('busy');
        expect(initialSnapshot.slices).not.toHaveProperty('loading');
        expect(initialSnapshot.eventPayloads).not.toHaveProperty('busy');
        expect(initialSnapshot.eventPayloads).not.toHaveProperty('loading');
        expect(refreshSnapshot.slices).not.toHaveProperty('busy');
        expect(refreshSnapshot.eventPayloads).not.toHaveProperty('busy');
        expect(lifecycleEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('marks explicit empty workflow surfaces without inferring layout emptiness', async () => {
        const authoredFallback = document.createElement('cem-surface');
        authoredFallback.setAttribute('label', 'Asset results');
        authoredFallback.setAttribute('empty', '');
        authoredFallback.innerHTML = `
            <h2>No assets yet</h2>
            <p>Upload an asset to begin building this collection.</p>
            <a href="#authored-upload">Upload an asset</a>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('No assets yet');
        expect(authoredFallback.querySelector('a')?.getAttribute('href')).toBe('#authored-upload');

        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-surface label="Dashboard">
                    <p>Ready</p>
                </cem-surface>
                <cem-surface label="Asset results" empty>
                    <h2>No assets yet</h2>
                    <p>Upload an asset to begin building this collection.</p>
                    <a href="#new-asset">Upload an asset</a>
                </cem-surface>
                <cem-surface label="False-token results" empty="false">
                    <p>No matching results.</p>
                    <a href="#clear-filters">Clear filters</a>
                </cem-surface>
                <cem-stack id="empty-stack" empty></cem-stack>
                <cem-grid id="empty-grid" empty></cem-grid>
            </cem-stack>
        `);
        await waitForStateSelector(root, '#empty-grid > .cem-grid');

        const ordinaryHost = harness.query<HTMLElement>('cem-surface:not([empty])');
        const ordinary = harness.query<HTMLElement>('cem-surface:not([empty]) > section');
        const emptyHost = harness.query<HTMLElement>('cem-surface[empty=""]');
        const emptySurface = harness.query<HTMLElement>('cem-surface[empty=""] > section');
        const falseTokenSurface = harness.query<HTMLElement>('cem-surface[empty="false"] > section');
        const emptyStack = harness.query<HTMLDivElement>('#empty-stack > .cem-stack');
        const emptyGrid = harness.query<HTMLDivElement>('#empty-grid > .cem-grid');
        const guidance = harness.query<HTMLParagraphElement>('cem-surface[empty=""] p');
        const recovery = harness.query<HTMLAnchorElement>('cem-surface[empty=""] a');

        assertStateHostsRendered(harness.root, 'cem-surface, #empty-stack, #empty-grid');
        expect(ordinary.className).toBe('cem-surface cem-surface--default');
        expect(ordinary.getAttribute('aria-label')).toBe('Dashboard');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.children).toHaveLength(1);
        expect(ordinary.textContent?.trim()).toBe('Ready');
        expect(assertAccessibleName(emptySurface, 'Asset results')).toBe('Asset results');
        expect(emptySurface.className).toBe('cem-surface cem-surface--default');
        expect(emptySurface.getAttribute('data-state')).toBe('empty');
        expect(emptySurface.children).toHaveLength(3);
        expect(emptySurface.querySelector('h2')?.textContent?.trim()).toBe('No assets yet');
        expect(guidance.textContent?.trim()).toBe('Upload an asset to begin building this collection.');
        expect(recovery).toBeInstanceOf(HTMLAnchorElement);
        expect(recovery.getAttribute('href')).toBe('#new-asset');
        expect(assertAccessibleName(recovery, 'Upload an asset')).toBe('Upload an asset');
        expect(falseTokenSurface.getAttribute('data-state')).toBe('empty');
        expect(emptySurface.getAttribute('role')).toBeNull();
        expect(emptySurface.getAttribute('aria-live')).toBeNull();
        expect(emptySurface.getAttribute('aria-atomic')).toBeNull();
        expect(emptySurface.getAttribute('tabindex')).toBeNull();
        expect(emptySurface.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(emptyStack.hasAttribute('data-state')).toBe(false);
        expect(emptyStack.getAttribute('role')).toBeNull();
        expect(emptyStack.childElementCount).toBe(0);
        expect(emptyStack.textContent?.trim()).toBe('');
        expect(emptyGrid.hasAttribute('data-state')).toBe(false);
        expect(emptyGrid.getAttribute('role')).toBeNull();
        expect(emptyGrid.childElementCount).toBe(0);
        expect(emptyGrid.textContent?.trim()).toBe('');

        await assertFocusVisible(recovery);
        emptyHost.removeAttribute('empty');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const ordinaryTransition = harness.query<HTMLElement>('cem-surface[label="Asset results"] > section');
        const recoveryAfterRemoval = harness.query<HTMLAnchorElement>('cem-surface[label="Asset results"] a');
        expect(ordinaryTransition).toBe(emptySurface);
        expect(recoveryAfterRemoval).toBe(recovery);
        expect(ordinaryTransition.hasAttribute('data-state')).toBe(false);
        expect(document.activeElement).toBe(recoveryAfterRemoval);

        emptyHost.setAttribute('empty', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const emptyTransition = harness.query<HTMLElement>('cem-surface[label="Asset results"] > section');
        const recoveryAfterAddition = harness.query<HTMLAnchorElement>('cem-surface[label="Asset results"] a');
        const snapshot = runtime.snapshotInstance(emptyHost);
        expect(emptyTransition).toBe(emptySurface);
        expect(recoveryAfterAddition).toBe(recovery);
        expect(emptyTransition.getAttribute('data-state')).toBe('empty');
        expect(document.activeElement).toBe(recoveryAfterAddition);
        expect(snapshot.slices).not.toHaveProperty('empty');
        expect(snapshot.eventPayloads).not.toHaveProperty('empty');
        expect(runtime.snapshotInstance(ordinaryHost).slices).not.toHaveProperty('empty');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('marks explicit busy workflow surfaces without making formatting containers loading owners', async () => {
        const authoredFallback = document.createElement('cem-surface');
        authoredFallback.setAttribute('label', 'Asset workspace');
        authoredFallback.setAttribute('busy', '');
        authoredFallback.innerHTML = `
            <h2>Asset workspace</h2>
            <p>Loading filters and results…</p>
            <cem-stack gap="md">
                <cem-skeleton label="Asset filters"></cem-skeleton>
                <cem-skeleton label="Asset results"></cem-skeleton>
            </cem-stack>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('Loading filters and results…');
        expect(authoredFallback.querySelectorAll('cem-skeleton')).toHaveLength(2);

        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-surface label="Ordinary workspace">
                    <p>Ready</p>
                </cem-surface>
                <cem-surface label="Initial workspace" busy>
                    <h2>Asset workspace</h2>
                    <p class="loading-message">Loading filters and results…</p>
                    <cem-stack gap="md">
                        <cem-skeleton label="Asset filters"></cem-skeleton>
                        <cem-skeleton label="Asset results"></cem-skeleton>
                    </cem-stack>
                    <cem-surface label="Nested ordinary surface">
                        <p>Nested content</p>
                    </cem-surface>
                </cem-surface>
                <cem-surface label="Profile workspace" busy="false">
                    <h2>Profile workspace</h2>
                    <cem-grid columns="2" gap="lg">
                        <cem-card label="Profile details">
                            <p>Grace Hopper</p>
                            <button type="button">Edit profile</button>
                        </cem-card>
                        <cem-card label="Preferences">
                            <p>Daily summaries</p>
                        </cem-card>
                    </cem-grid>
                </cem-surface>
                <cem-surface label="Empty transition" busy empty>
                    <h2>No assets yet</h2>
                    <p>Upload an asset to begin building this collection.</p>
                    <a href="#new-asset">Upload an asset</a>
                </cem-surface>
                <cem-stack id="busy-stack" gap="sm" busy>
                    <p>Formatting stack</p>
                </cem-stack>
                <cem-grid id="busy-grid" columns="2" gap="md" busy>
                    <p>First cell</p>
                    <p>Second cell</p>
                </cem-grid>
            </cem-stack>
        `);
        await waitForStateSelector(root, '#busy-grid > .cem-grid');

        const ordinary = harness.query<HTMLElement>('cem-surface[label="Ordinary workspace"] > section');
        const initialHost = harness.query<HTMLElement>('cem-surface[label="Initial workspace"]');
        const initial = harness.query<HTMLElement>('cem-surface[label="Initial workspace"] > section');
        const initialMessage = harness.query<HTMLParagraphElement>(
            'cem-surface[label="Initial workspace"] > section > .loading-message',
        );
        const initialStack = harness.query<HTMLDivElement>(
            'cem-surface[label="Initial workspace"] > section > cem-stack > .cem-stack',
        );
        const skeletons = Array.from(
            harness.root.querySelectorAll<HTMLElement>(
                'cem-surface[label="Initial workspace"] > section > cem-stack cem-skeleton > .cem-skeleton',
            ),
        );
        const nestedSurface = harness.query<HTMLElement>(
            'cem-surface[label="Initial workspace"] cem-surface[label="Nested ordinary surface"] > section',
        );
        const refreshHost = harness.query<HTMLElement>('cem-surface[label="Profile workspace"]');
        const refreshSurface = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const refreshGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const refreshCards = Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-surface[label="Profile workspace"] cem-card > section'),
        );
        const refreshButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        const transitionHost = harness.query<HTMLElement>('cem-surface[label="Empty transition"]');
        const transitionSurface = harness.query<HTMLElement>('cem-surface[label="Empty transition"] > section');
        const transitionRecovery = harness.query<HTMLAnchorElement>('cem-surface[label="Empty transition"] a');
        const busyStack = harness.query<HTMLDivElement>('#busy-stack > .cem-stack');
        const busyGrid = harness.query<HTMLDivElement>('#busy-grid > .cem-grid');

        assertStateHostsRendered(
            harness.root,
            'cem-surface, #busy-stack, #busy-grid, cem-surface cem-stack, cem-surface cem-grid, cem-surface cem-card, cem-surface cem-skeleton',
        );
        expect(ordinary.className).toBe('cem-surface cem-surface--default');
        expect(ordinary.getAttribute('aria-label')).toBe('Ordinary workspace');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.hasAttribute('aria-busy')).toBe(false);

        expect(assertAccessibleName(initial, 'Initial workspace')).toBe('Initial workspace');
        expect(initial.getAttribute('data-state')).toBe('loading');
        expect(initial.getAttribute('aria-busy')).toBe('true');
        expect(initialMessage.textContent?.trim()).toBe('Loading filters and results…');
        expect(initialMessage.getAttribute('role')).toBeNull();
        expect(initialMessage.getAttribute('aria-live')).toBeNull();
        expect(initialStack.getAttribute('data-gap')).toBe('md');
        expect(initialStack.hasAttribute('data-state')).toBe(false);
        expect(initialStack.hasAttribute('aria-busy')).toBe(false);
        expect(skeletons).toHaveLength(2);
        expect(skeletons.every((skeleton) => skeleton.getAttribute('aria-hidden') === 'true')).toBe(true);
        expect(nestedSurface.hasAttribute('data-state')).toBe(false);
        expect(nestedSurface.hasAttribute('aria-busy')).toBe(false);
        expect(initial.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(initial.hasAttribute('inert')).toBe(false);

        expect(refreshSurface.getAttribute('data-state')).toBe('loading');
        expect(refreshSurface.getAttribute('aria-busy')).toBe('true');
        expect(refreshGrid.getAttribute('data-columns')).toBe('2');
        expect(refreshGrid.getAttribute('data-gap')).toBe('lg');
        expect(refreshCards).toHaveLength(2);
        expect(refreshCards.every((card) => !card.hasAttribute('data-state'))).toBe(true);
        expect(refreshCards.every((card) => !card.hasAttribute('aria-busy'))).toBe(true);
        expect(refreshButton.disabled).toBe(false);
        expect(refreshButton.tabIndex).toBe(0);

        expect(transitionSurface.getAttribute('data-state')).toBe('loading');
        expect(transitionSurface.getAttribute('aria-busy')).toBe('true');
        expect(transitionSurface.textContent).toContain('No assets yet');
        for (const candidate of [busyStack, busyGrid]) {
            expect(candidate.hasAttribute('data-state')).toBe(false);
            expect(candidate.hasAttribute('aria-busy')).toBe(false);
        }

        const refreshRect = refreshSurface.getBoundingClientRect();
        const refreshGridRect = refreshGrid.getBoundingClientRect();
        const refreshChildren = Array.from(refreshGrid.children);
        const refreshChildPositions = refreshChildren.map((child) => {
            const rect = child.getBoundingClientRect();
            return [rect.x, rect.y, rect.width, rect.height];
        });
        expect(refreshRect.width).toBeGreaterThan(0);
        expect(refreshRect.height).toBeGreaterThan(0);
        expect(refreshGridRect.width).toBeGreaterThan(0);
        expect(refreshGridRect.height).toBeGreaterThan(0);
        const lifecycleEvents: string[] = [];
        for (const name of ['cem-loaded', 'cem-error', 'cem-cancel']) {
            refreshHost.addEventListener(name, () => lifecycleEvents.push(name));
        }

        await assertFocusVisible(refreshButton);
        refreshHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const settledSurface = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const settledGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const settledButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        const settledRect = settledSurface.getBoundingClientRect();
        const settledGridRect = settledGrid.getBoundingClientRect();
        const settledChildren = Array.from(settledGrid.children);
        const settledChildPositions = settledChildren.map((child) => {
            const rect = child.getBoundingClientRect();
            return [rect.x, rect.y, rect.width, rect.height];
        });
        expect(settledSurface).toBe(refreshSurface);
        expect(settledGrid).toBe(refreshGrid);
        expect(settledButton).toBe(refreshButton);
        expect(settledChildren[0]).toBe(refreshChildren[0]);
        expect(settledChildren[1]).toBe(refreshChildren[1]);
        expect(settledSurface.hasAttribute('data-state')).toBe(false);
        expect(settledSurface.hasAttribute('aria-busy')).toBe(false);
        expect([settledRect.width, settledRect.height]).toEqual([refreshRect.width, refreshRect.height]);
        expect([settledGridRect.x, settledGridRect.y, settledGridRect.width, settledGridRect.height]).toEqual([
            refreshGridRect.x,
            refreshGridRect.y,
            refreshGridRect.width,
            refreshGridRect.height,
        ]);
        expect(settledChildPositions).toEqual(refreshChildPositions);
        expect(document.activeElement).toBe(settledButton);

        refreshHost.setAttribute('busy', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const pendingAgain = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const pendingGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const pendingButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        expect(pendingAgain).toBe(refreshSurface);
        expect(pendingGrid).toBe(refreshGrid);
        expect(pendingButton).toBe(refreshButton);
        expect(pendingAgain.getAttribute('data-state')).toBe('loading');
        expect(pendingAgain.getAttribute('aria-busy')).toBe('true');
        expect(document.activeElement).toBe(pendingButton);

        await assertFocusVisible(transitionRecovery);
        transitionHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(transitionHost);
        await nextRenderFrame();

        const settledEmpty = harness.query<HTMLElement>('cem-surface[label="Empty transition"] > section');
        const settledRecovery = harness.query<HTMLAnchorElement>('cem-surface[label="Empty transition"] a');
        expect(settledEmpty).toBe(transitionSurface);
        expect(settledRecovery).toBe(transitionRecovery);
        expect(settledEmpty.getAttribute('data-state')).toBe('empty');
        expect(settledEmpty.hasAttribute('aria-busy')).toBe(false);
        expect(document.activeElement).toBe(settledRecovery);

        for (const host of [initialHost, refreshHost, transitionHost]) {
            const snapshot = runtime.snapshotInstance(host);
            expect(snapshot.slices).not.toHaveProperty('busy');
            expect(snapshot.slices).not.toHaveProperty('loading');
            expect(snapshot.slices).not.toHaveProperty('empty');
            expect(snapshot.eventPayloads).not.toHaveProperty('busy');
            expect(snapshot.eventPayloads).not.toHaveProperty('loading');
            expect(snapshot.eventPayloads).not.toHaveProperty('empty');
        }
        expect(lifecycleEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('preserves empty states, indeterminate progress, and live-region roles', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-list label="Empty tasks"></cem-list>
                <cem-table label="Empty table"></cem-table>
                <cem-progress label="Loading assets"></cem-progress>
                <cem-toast>Saved</cem-toast>
                <cem-alert tone="danger" role="alert">Resolve errors.</cem-alert>
                <cem-skeleton label="Loading card"></cem-skeleton>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-skeleton .cem-skeleton');

        const list = harness.query<HTMLUListElement>('cem-list ul');
        const table = harness.query<HTMLElement>('cem-table [role="table"]');
        const progress = harness.query<HTMLProgressElement>('cem-progress progress');
        const toast = harness.query<HTMLElement>('cem-toast [role="status"]');
        const alert = harness.query<HTMLElement>('cem-alert [role="alert"]');
        const skeleton = harness.query<HTMLElement>('cem-skeleton .cem-skeleton');

        assertStateHostsRendered(harness.root, 'cem-list, cem-table, cem-progress, cem-toast, cem-alert, cem-skeleton');
        expect(assertAccessibleName(list, 'Empty tasks')).toBe('Empty tasks');
        expect(list.querySelector('.cem-list__empty')?.textContent?.trim()).toBe('No items');
        expect(assertAccessibleName(table, 'Empty table')).toBe('Empty table');
        expect(table.querySelector('[role="cell"]')?.textContent?.trim()).toBe('No rows');
        expect(progress.hasAttribute('value')).toBe(false);
        expect(assertAccessibleName(progress, 'Loading assets')).toBe('Loading assets');
        expect(toast.getAttribute('aria-live')).toBe('polite');
        expect(toast.textContent?.trim()).toBe('Saved');
        expect(alert.getAttribute('data-tone')).toBe('danger');
        expect(alert.textContent?.trim()).toBe('Resolve errors.');
        expect(skeleton.getAttribute('aria-hidden')).toBe('true');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });
});

async function waitForStateSelector(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + 1000;

    while (Date.now() < deadline) {
        const element = root.querySelector(selector);

        if (element) {
            return element;
        }

        await nextRenderFrame();
    }

    throw new Error(`Expected state render output matching ${selector}`);
}

function assertStateHostsRendered(root: ParentNode, selector: string): void {
    for (const host of Array.from(root.querySelectorAll<HTMLElement>(selector))) {
        assertLightDomRendered(host);
        expect(host.shadowRoot).toBeNull();
    }
}

function eventPayload(snapshot: DataIslandSnapshot, name: string): SerializedEventPayload {
    const payload = snapshot.eventPayloads[name];

    if (!isSerializedEventPayload(payload)) {
        throw new Error(`Expected serialized event payload for ${name}`);
    }

    return payload;
}

function isSerializedEventPayload(value: unknown): value is SerializedEventPayload {
    if (!value || typeof value !== 'object') {
        return false;
    }

    const record = value as Partial<SerializedEventPayload>;

    return typeof record.type === 'string' && 'sliceValue' in record;
}

interface InputIndicatorLayer {
    color: string;
    geometry: readonly number[];
}

interface InputIndicatorStateSnapshot {
    boxShadow: string;
    controlHtml: string;
    controlRect: readonly number[];
    hostAttributes: readonly string[];
    hostRect: readonly number[];
    layers: readonly InputIndicatorLayer[];
    runtime: string;
    targetHtml: string;
    targetRect: readonly number[];
}

function captureInputIndicatorState(
    runtime: CemElementRuntime,
    host: HTMLElement,
    control: HTMLButtonElement | HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement,
    target: HTMLElement,
): InputIndicatorStateSnapshot {
    const boxShadow = getComputedStyle(target).boxShadow;
    const runtimeSnapshot = runtime.snapshotInstance(host);

    return {
        boxShadow,
        controlHtml: control.outerHTML,
        controlRect: rectTuple(control),
        hostAttributes: Array.from(host.attributes, ({ name, value }) => `${name}=${value}`),
        hostRect: rectTuple(host),
        layers: parseInputIndicatorLayers(boxShadow),
        runtime: JSON.stringify({
            eventPayloads: runtimeSnapshot.eventPayloads,
            formData: runtimeSnapshot.formData,
            payload: runtimeSnapshot.payload,
            slices: runtimeSnapshot.slices,
            validationState: runtimeSnapshot.validationState,
        }),
        targetHtml: target.outerHTML,
        targetRect: rectTuple(target),
    };
}

function parseInputIndicatorLayers(boxShadow: string): readonly InputIndicatorLayer[] {
    if (boxShadow === 'none') {
        throw new Error('Expected a composed input indicator box shadow');
    }

    return splitTopLevel(boxShadow).map((layer) => {
        const color = layer.match(/(?:rgba?|hsla?|color)\([^)]*\)/)?.[0];
        const lengths = Array.from(layer.matchAll(/(-?\d*\.?\d+)px/g), (match) => Number(match[1]));

        if (!color || lengths.length < 4) {
            throw new Error(`Expected an input indicator shadow layer, received ${layer}`);
        }

        return {
            color: paintedColor(color),
            geometry: lengths.slice(-4),
        };
    });
}

function expectInputIndicatorGeometry(
    snapshot: InputIndicatorStateSnapshot,
    target: HTMLElement,
    appearance: 'outline' | 'underline',
    states: { anchorWidthToken?: string; focus?: boolean; selection?: boolean } = {},
): void {
    const boundary = resolveTokenLength(target, states.anchorWidthToken ?? '--cem-stroke-boundary');
    const stripe = resolveTokenLength(target, '--cem-zebra-strip-size');
    const cumulativeWidths = [
        boundary,
        boundary + (states.focus ? stripe : 0),
        boundary + (states.focus ? stripe : 0) + (states.selection ? stripe : 0),
    ];

    expect(snapshot.layers).toHaveLength(3);
    for (const [index, layer] of snapshot.layers.entries()) {
        const expected =
            appearance === 'underline'
                ? [0, cumulativeWidths[index], 0, 0]
                : [0, 0, 0, cumulativeWidths[index]];
        expect(layer.geometry).toEqual(expected);
    }
}

function expectInputIndicatorStructureAndGeometry(
    actual: InputIndicatorStateSnapshot,
    expected: InputIndicatorStateSnapshot,
): void {
    expect(actual.controlHtml).toBe(expected.controlHtml);
    expect(actual.controlRect).toEqual(expected.controlRect);
    expect(actual.hostAttributes).toEqual(expected.hostAttributes);
    expect(actual.hostRect).toEqual(expected.hostRect);
    expect(actual.runtime).toBe(expected.runtime);
    expect(actual.targetHtml).toBe(expected.targetHtml);
    expect(actual.targetRect).toEqual(expected.targetRect);
}

interface ContentInteractionStateSnapshot {
    backgroundColor: string;
    color: string;
    focusTreatment: readonly string[];
    forcedColorAdjust: string;
    hostAttributes: readonly string[];
    hostBackgroundColor: string;
    hostHtml: string;
    hostRect: readonly number[];
    ownerHtml: string;
    ownerRect: readonly number[];
    runtime: string;
    semanticState: string;
}

function captureContentInteractionState(
    runtime: CemElementRuntime,
    host: HTMLElement,
    owner: HTMLElement,
): ContentInteractionStateSnapshot {
    const hostStyles = getComputedStyle(host);
    const ownerStyles = getComputedStyle(owner);
    const runtimeSnapshot = runtime.snapshotInstance(host);
    const semanticState =
        owner instanceof HTMLSelectElement
            ? JSON.stringify({
                  disabled: owner.disabled,
                  options: Array.from(owner.options, (option) => ({
                      ariaSelected: option.getAttribute('aria-selected'),
                      disabled: option.disabled,
                      selected: option.selected,
                      value: option.value,
                  })),
                  value: owner.value,
              })
            : JSON.stringify({
                  ariaPressed: owner.getAttribute('aria-pressed'),
                  disabled: owner instanceof HTMLButtonElement ? owner.disabled : null,
              });

    return {
        backgroundColor: paintedColor(ownerStyles.backgroundColor),
        color: paintedColor(ownerStyles.color),
        focusTreatment: [
            ownerStyles.outlineColor,
            ownerStyles.outlineStyle,
            ownerStyles.outlineWidth,
            ownerStyles.outlineOffset,
            ownerStyles.boxShadow,
        ],
        forcedColorAdjust: ownerStyles.forcedColorAdjust,
        hostAttributes: Array.from(host.attributes, ({ name, value }) => `${name}=${value}`),
        hostBackgroundColor: paintedColor(hostStyles.backgroundColor),
        hostHtml: host.outerHTML,
        hostRect: sizeTuple(host),
        ownerHtml: owner.outerHTML,
        ownerRect: sizeTuple(owner),
        runtime: JSON.stringify({
            eventPayloads: runtimeSnapshot.eventPayloads,
            formData: runtimeSnapshot.formData,
            payload: runtimeSnapshot.payload,
            slices: runtimeSnapshot.slices,
            validationState: runtimeSnapshot.validationState,
        }),
        semanticState,
    };
}

function expectContentInteractionStructureAndGeometry(
    actual: ContentInteractionStateSnapshot,
    expected: ContentInteractionStateSnapshot,
): void {
    expect(actual.hostAttributes).toEqual(expected.hostAttributes);
    expect(actual.hostHtml).toBe(expected.hostHtml);
    expect(actual.hostRect).toEqual(expected.hostRect);
    expect(actual.ownerHtml).toBe(expected.ownerHtml);
    expect(actual.ownerRect).toEqual(expected.ownerRect);
    expect(actual.runtime).toBe(expected.runtime);
    expect(actual.semanticState).toBe(expected.semanticState);
}

function resolveTokenLength(element: Element, tokenName: string): number {
    const value = getComputedStyle(element).getPropertyValue(tokenName).trim();
    const match = value.match(/^(-?\d*\.?\d+)px$/);

    if (!match) {
        throw new Error(`Expected generated length token ${tokenName}, received ${value || '<empty>'}`);
    }

    return Number(match[1]);
}

interface NavigationStateSnapshot {
    backgroundColor: string;
    color: string;
    focusTreatment: readonly string[];
    forcedColorAdjust: string;
    hostAttributes: readonly string[];
    hostRect: readonly number[];
    ownerHtml: string;
    ownerRect: readonly number[];
    runtime: string;
    wrapperBackgroundColor: string;
    wrapperFocusTreatment: readonly string[];
    wrapperHtml: string;
    wrapperRect: readonly number[];
}

function captureNavigationState(
    runtime: CemElementRuntime,
    host: HTMLElement,
    wrapper: HTMLElement,
    owner: HTMLElement,
): NavigationStateSnapshot {
    const ownerStyles = getComputedStyle(owner);
    const wrapperStyles = getComputedStyle(wrapper);
    const runtimeSnapshot = runtime.snapshotInstance(host);

    return {
        backgroundColor: paintedColor(ownerStyles.backgroundColor),
        color: paintedColor(ownerStyles.color),
        focusTreatment: [
            ownerStyles.outlineColor,
            ownerStyles.outlineStyle,
            ownerStyles.outlineWidth,
            ownerStyles.outlineOffset,
            ownerStyles.boxShadow,
        ],
        forcedColorAdjust: ownerStyles.forcedColorAdjust,
        hostAttributes: Array.from(host.attributes, ({ name, value }) => `${name}=${value}`),
        hostRect: rectTuple(host),
        ownerHtml: owner.outerHTML,
        ownerRect: rectTuple(owner),
        runtime: JSON.stringify({
            eventPayloads: runtimeSnapshot.eventPayloads,
            formData: runtimeSnapshot.formData,
            payload: runtimeSnapshot.payload,
            slices: runtimeSnapshot.slices,
            validationState: runtimeSnapshot.validationState,
        }),
        wrapperBackgroundColor: paintedColor(wrapperStyles.backgroundColor),
        wrapperFocusTreatment: [
            wrapperStyles.outlineColor,
            wrapperStyles.outlineStyle,
            wrapperStyles.outlineWidth,
            wrapperStyles.outlineOffset,
            wrapperStyles.boxShadow,
        ],
        wrapperHtml: wrapper.outerHTML,
        wrapperRect: rectTuple(wrapper),
    };
}

function expectNavigationStructureAndGeometry(
    actual: NavigationStateSnapshot,
    expected: NavigationStateSnapshot,
): void {
    expect(actual.hostAttributes).toEqual(expected.hostAttributes);
    expect(actual.hostRect).toEqual(expected.hostRect);
    expect(actual.ownerHtml).toBe(expected.ownerHtml);
    expect(actual.ownerRect).toEqual(expected.ownerRect);
    expect(actual.runtime).toBe(expected.runtime);
    expect(actual.wrapperHtml).toBe(expected.wrapperHtml);
    expect(actual.wrapperRect).toEqual(expected.wrapperRect);
}

interface ActionStateSnapshot {
    backgroundColor: string;
    buttonHtml: string;
    buttonRect: readonly number[];
    color: string;
    focusTreatment: readonly string[];
    forcedColorAdjust: string;
    hostAttributes: readonly string[];
    hostRect: readonly number[];
    runtime: string;
}

function captureActionState(
    runtime: CemElementRuntime,
    host: HTMLElement,
    button: HTMLButtonElement,
): ActionStateSnapshot {
    const styles = getComputedStyle(button);
    const runtimeSnapshot = runtime.snapshotInstance(host);

    return {
        backgroundColor: paintedColor(styles.backgroundColor),
        buttonHtml: button.outerHTML,
        buttonRect: rectTuple(button),
        color: paintedColor(styles.color),
        focusTreatment: [styles.outlineColor, styles.outlineStyle, styles.outlineWidth, styles.boxShadow],
        forcedColorAdjust: styles.getPropertyValue('forced-color-adjust'),
        hostAttributes: Array.from(host.attributes, ({ name, value }) => `${name}=${value}`),
        hostRect: rectTuple(host),
        runtime: JSON.stringify({
            eventPayloads: runtimeSnapshot.eventPayloads,
            formData: runtimeSnapshot.formData,
            payload: runtimeSnapshot.payload,
            slices: runtimeSnapshot.slices,
            validationState: runtimeSnapshot.validationState,
        }),
    };
}

function expectActionStructureAndGeometry(actual: ActionStateSnapshot, expected: ActionStateSnapshot): void {
    expectActionStructureAndGeometryAfterActivation(actual, expected);
    expect(actual.runtime).toBe(expected.runtime);
}

function expectActionStructureAndGeometryAfterActivation(
    actual: ActionStateSnapshot,
    expected: ActionStateSnapshot,
): void {
    expect(actual.buttonHtml).toBe(expected.buttonHtml);
    expect(actual.buttonRect).toEqual(expected.buttonRect);
    expect(actual.hostAttributes).toEqual(expected.hostAttributes);
    expect(actual.hostRect).toEqual(expected.hostRect);
}

function nextTrustedPointerDown(button: HTMLElement): Promise<PointerEvent> {
    return new Promise((resolve, reject) => {
        const timeout = window.setTimeout(() => {
            button.removeEventListener('pointerdown', onPointerDown);
            reject(new Error('Expected a trusted pointerdown before the provider interaction completed'));
        }, 1000);
        const onPointerDown = (event: PointerEvent): void => {
            window.clearTimeout(timeout);
            resolve(event);
        };

        button.addEventListener('pointerdown', onPointerDown, { once: true });
    });
}

async function eventBeforeInteractionCompletes<T extends Event>(
    event: Promise<T>,
    interaction: Promise<void>,
    label: string,
): Promise<T> {
    return Promise.race([
        event,
        interaction.then(() => {
            throw new Error(`Expected ${label} while the provider interaction was still held`);
        }),
    ]);
}

async function waitForPseudoClass(element: Element, pseudoClass: string): Promise<void> {
    const deadline = Date.now() + 1000;

    while (Date.now() < deadline) {
        if (element.matches(pseudoClass)) {
            return;
        }
        await nextRenderFrame();
    }

    throw new Error(`Expected ${element.tagName.toLowerCase()} to match ${pseudoClass}`);
}

function contrastRatio(first: string, second: string): number {
    const firstLuminance = relativeLuminance(first);
    const secondLuminance = relativeLuminance(second);
    return (Math.max(firstLuminance, secondLuminance) + 0.05) / (Math.min(firstLuminance, secondLuminance) + 0.05);
}

function relativeLuminance(painted: string): number {
    const channels = painted.split(',').map(Number);
    if (channels.length !== 4 || channels.some((channel) => !Number.isFinite(channel)) || channels[3] !== 255) {
        throw new Error(`Expected an opaque painted RGBA color, received ${painted}`);
    }

    const [red, green, blue] = channels.slice(0, 3).map((channel) => {
        const normalized = channel / 255;
        return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
    });

    return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function rectTuple(element: Element): readonly number[] {
    const rect = element.getBoundingClientRect();
    return [rect.x, rect.y, rect.width, rect.height];
}

function sizeTuple(element: Element): readonly number[] {
    const rect = element.getBoundingClientRect();
    return [rect.width, rect.height];
}

function resolveTokenColor(element: Element, tokenName: string): string {
    const styles = getComputedStyle(element);
    const tokenValue = styles.getPropertyValue(tokenName).trim();

    if (!tokenValue) {
        throw new Error(`Expected generated theme token ${tokenName}`);
    }

    return paintedColor(resolveLightDark(tokenValue, styles.colorScheme));
}

function expectPaintedColorToResolveFromToken(actual: string, element: Element, tokenName: string): void {
    const expected = resolveTokenColor(element, tokenName);
    const actualChannels = actual.split(',').map(Number);
    const expectedChannels = expected.split(',').map(Number);

    expect(actualChannels).toHaveLength(4);
    expect(expectedChannels).toHaveLength(4);
    expect(actualChannels.every((channel) => Number.isFinite(channel))).toBe(true);
    expect(expectedChannels.every((channel) => Number.isFinite(channel))).toBe(true);
    for (const [index, channel] of actualChannels.entries()) {
        expect(Math.abs(channel - expectedChannels[index])).toBeLessThanOrEqual(1);
    }
}

function resolveLightDark(value: string, colorScheme: string): string {
    let resolved = value;
    let start = resolved.indexOf('light-dark(');

    while (start >= 0) {
        const open = start + 'light-dark'.length;
        const close = matchingParen(resolved, open);
        const choices = splitTopLevel(resolved.slice(open + 1, close));

        if (choices.length !== 2) {
            throw new Error(`Expected light-dark() token value to contain two colors: ${value}`);
        }

        const choice = colorScheme.includes('dark') ? choices[1] : choices[0];
        resolved = `${resolved.slice(0, start)}${choice.trim()}${resolved.slice(close + 1)}`;
        start = resolved.indexOf('light-dark(');
    }

    return resolved;
}

function matchingParen(value: string, open: number): number {
    let depth = 0;

    for (let index = open; index < value.length; index += 1) {
        if (value[index] === '(') {
            depth += 1;
        } else if (value[index] === ')') {
            depth -= 1;
            if (depth === 0) {
                return index;
            }
        }
    }

    throw new Error(`Unclosed CSS function in token value: ${value}`);
}

function splitTopLevel(value: string): string[] {
    const values: string[] = [];
    let depth = 0;
    let start = 0;

    for (let index = 0; index < value.length; index += 1) {
        if (value[index] === '(') {
            depth += 1;
        } else if (value[index] === ')') {
            depth -= 1;
        } else if (value[index] === ',' && depth === 0) {
            values.push(value.slice(start, index));
            start = index + 1;
        }
    }

    values.push(value.slice(start));
    return values;
}

function paintedColor(value: string): string {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });

    if (!context) {
        throw new Error('Expected a 2D canvas context for computed color comparison');
    }

    context.fillStyle = value;
    context.fillRect(0, 0, 1, 1);
    return Array.from(context.getImageData(0, 0, 1, 1).data).join(',');
}
