import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, cemStoryRuntime, loadCemComponent, loadCemWorkflowDependencies, assertAccessibleName, assertAriaReferenceIntegrity, assertFocusVisible, assertLightDomRendered, nextRenderFrame, type CemElementRuntime, type DataIslandSnapshot, captureVisualSnapshot, type VisualSnapshot } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-nav.xhtml?raw';
import '../../styles.css';
const meta = preview.meta({ component: 'cem-nav', title: 'CEM Components/cem-nav', loaders: [async () => { await loadCemDeclaration('cem-nav', declarationSource); await loadCemComponent('cem-tabs');  await loadCemWorkflowDependencies(); return {}; }] });

export const HoverOwners = meta.story({
 name: "styles only navigation hover owners without changing current selection or component state",
 render: () => "\n            <cem-stack class=\"cem-theme-light\" gap=\"sm\">\n                <cem-nav label=\"Primary navigation\">\n                    <a href=\"#overview\">Overview</a>\n                    <a href=\"#current\" aria-current=\"page\">Current page</a>\n                    <a href=\"#unavailable\" aria-disabled=\"true\">Unavailable link</a>\n                    <button type=\"button\" disabled>Unavailable action</button>\n                </cem-nav>\n                <cem-nav label=\"Workspace navigation\" collapsible expanded>\n                    <a href=\"#workspace\">Workspace</a>\n                </cem-nav>\n                <cem-tabs label=\"Profile sections\" selected-index=\"1\">\n                    <cem-tab value=\"overview\" label=\"Overview tab\"><p>Overview panel</p></cem-tab>\n                    <cem-tab value=\"security\" label=\"Security tab\"><p>Security panel</p></cem-tab>\n                    <cem-tab value=\"billing\" label=\"Billing tab\" disabled><p>Billing panel</p></cem-tab>\n                </cem-tabs>\n            </cem-stack>\n        ",
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer, focus and native keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        const runtime = cemStoryRuntime();
        for (const wrapper of root.querySelectorAll<HTMLElement>('cem-stack, cem-table')) await whenCemRendered(wrapper);
        for (const host of root.querySelectorAll<HTMLElement>('cem-nav, cem-tabs, cem-list, cem-chip')) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});

export const KeyboardFocus = meta.story({
 name: "moves keyboard focus through navigation owners without changing selection or component state",
 render: () => "\n            <section class=\"cem-theme-light\">\n                <button id=\"navigation-focus-start\" type=\"button\">Start navigation focus sequence</button>\n                <cem-stack gap=\"sm\">\n                    <cem-nav label=\"Focus primary navigation\">\n                        <a id=\"focus-nav-default\" href=\"#overview\">Overview</a>\n                        <a id=\"focus-nav-current\" href=\"#current\" aria-current=\"page\">Current page</a>\n                        <button id=\"focus-nav-disabled\" type=\"button\" disabled>Unavailable action</button>\n                    </cem-nav>\n                    <cem-nav label=\"Focus workspace navigation\" collapsible expanded>\n                        <a id=\"focus-nav-content\" href=\"#workspace\">Workspace</a>\n                        <button id=\"focus-content-disabled\" type=\"button\" disabled>Unavailable workspace</button>\n                    </cem-nav>\n                    <cem-tabs label=\"Focus profile sections\" selected-index=\"1\">\n                        <cem-tab value=\"overview\" label=\"Overview tab\"><p>Overview panel</p></cem-tab>\n                        <cem-tab value=\"security\" label=\"Security tab\"><p>Security panel</p></cem-tab>\n                        <cem-tab value=\"disabled\" label=\"Disabled tab\" disabled><p>Disabled panel</p></cem-tab>\n                    </cem-tabs>\n                </cem-stack>\n                <button id=\"navigation-focus-end\" type=\"button\">End navigation focus sequence</button>\n            </section>\n        ",
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer, focus and native keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        const runtime = cemStoryRuntime();
        for (const wrapper of root.querySelectorAll<HTMLElement>('cem-stack, cem-table')) await whenCemRendered(wrapper);
        for (const host of root.querySelectorAll<HTMLElement>('cem-nav, cem-tabs, cem-list, cem-chip')) await whenCemRendered(host);
        await nextRenderFrame();
        await waitForStateSelector(
            root,
            'cem-tabs[label="Focus profile sections"] [data-tab-index="2"]:disabled',
        );

        const start = harness.query<HTMLButtonElement>('#navigation-focus-start');
        const end = harness.query<HTMLButtonElement>('#navigation-focus-end');
        const primaryHost = harness.query<HTMLElement>('cem-nav[label="Focus primary navigation"]');
        const primaryWrapper = harness.query<HTMLElement>('cem-nav[label="Focus primary navigation"] > nav');
        const disclosureHost = harness.query<HTMLElement>('cem-nav[label="Focus workspace navigation"]');
        const disclosureWrapper = harness.query<HTMLElement>('cem-nav[label="Focus workspace navigation"] > nav');
        const tabsHost = harness.query<HTMLElement>('cem-tabs[label="Focus profile sections"]');
        const tabsWrapper = harness.query<HTMLElement>('cem-tabs[label="Focus profile sections"] > [role="tablist"]');
        const selectedTabsPanel = harness.query<HTMLElement>(
            'cem-tabs[label="Focus profile sections"] > .cem-tabs__panels > [role="tabpanel"]:not([hidden])',
        );
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
                owner: harness.query<HTMLButtonElement>('cem-tabs[label="Focus profile sections"] [data-tab-index="1"]'),
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
            harness.query<HTMLButtonElement>('cem-tabs[label="Focus profile sections"] [data-tab-index="2"]'),
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
        expect(document.activeElement).toBe(selectedTabsPanel);
        expect(selectedTabsPanel.tabIndex).toBe(0);
        expect(lastCase.owner.matches(':focus-visible')).toBe(false);
        await userEvent.tab();
        await nextRenderFrame();
        expect(document.activeElement).toBe(end);
        const restoredLast = captureNavigationState(runtime, lastCase.host, lastCase.wrapper, lastCase.owner);
        expect(restoredLast.focusTreatment).toEqual(lastBaseline.focusTreatment);
        expectNavigationStructureAndGeometry(restoredLast, lastBaseline);

        expect(focusOrder).toEqual(cases.map(({ owner }) => owner.id || owner.className));
        expect(cases[1].owner.getAttribute('aria-current')).toBe('page');
        expect(cases[2].owner.getAttribute('aria-expanded')).toBe('true');
        expect(cases[4].owner.getAttribute('aria-selected')).toBe('true');
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

export const NativeActive = meta.story({
 name: "applies navigation active treatment during trusted pointer and native keyboard activation",
 render: () => "\n            <cem-stack class=\"cem-theme-light\" gap=\"sm\">\n                <cem-nav label=\"Active primary navigation\">\n                    <a id=\"active-nav-default\" href=\"#active-overview\">Overview</a>\n                    <a id=\"active-nav-current\" href=\"#active-current\" aria-current=\"page\">Current page</a>\n                    <a id=\"active-nav-aria-disabled\" href=\"#active-unavailable\" aria-disabled=\"true\">\n                        Unavailable link\n                    </a>\n                    <button id=\"active-nav-disabled\" type=\"button\" disabled>Unavailable action</button>\n                </cem-nav>\n                <cem-nav label=\"Active workspace navigation\" collapsible expanded>\n                    <a id=\"active-nav-content\" href=\"#active-workspace\">Workspace</a>\n                </cem-nav>\n                <cem-tabs label=\"Active profile sections\" selected-index=\"1\">\n                    <cem-tab value=\"overview\" label=\"Overview tab\"><p>Overview panel</p></cem-tab>\n                    <cem-tab value=\"security\" label=\"Security tab\"><p>Security panel</p></cem-tab>\n                    <cem-tab value=\"disabled\" label=\"Disabled tab\" disabled><p>Disabled panel</p></cem-tab>\n                </cem-tabs>\n            </cem-stack>\n        ",
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer, focus and native keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        const runtime = cemStoryRuntime();
        for (const wrapper of root.querySelectorAll<HTMLElement>('cem-stack, cem-table')) await whenCemRendered(wrapper);
        for (const host of root.querySelectorAll<HTMLElement>('cem-nav, cem-tabs, cem-list, cem-chip')) await whenCemRendered(host);
        await nextRenderFrame();
        await waitForStateSelector(root, 'cem-tabs[label="Active profile sections"] [data-tab-index="2"]:disabled');

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
                owner: harness.query<HTMLButtonElement>(
                    'cem-tabs[label="Active profile sections"] [data-tab-index="1"]',
                ),
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
                owner: harness.query<HTMLButtonElement>(
                    'cem-tabs[label="Active profile sections"] [data-tab-index="2"]',
                ),
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
        const selectedTab = activeCases[3].owner;
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
 },
});

export const DisabledDiscovery = meta.story({
 name: "keeps aria-disabled navigation owners discoverable while suppressing activation",
 render: () => "\n            <section class=\"cem-theme-light\">\n                <form id=\"disabled-navigation-form\">\n                    <button id=\"disabled-navigation-start\" type=\"button\">Start</button>\n                    <cem-nav label=\"Disabled primary navigation\">\n                        <a id=\"disabled-nav-enabled\" href=\"#enabled\">Enabled link</a>\n                        <a\n                            id=\"disabled-nav-current\"\n                            href=\"#disabled-current\"\n                            aria-current=\"page\"\n                            aria-disabled=\"true\"\n                        >\n                            <span id=\"disabled-nav-current-label\">Unavailable current page</span>\n                        </a>\n                        <button id=\"disabled-nav-submit\" type=\"submit\" aria-disabled=\"true\">\n                            Unavailable submit action\n                        </button>\n                        <button id=\"disabled-nav-native\" type=\"submit\" disabled>Native disabled action</button>\n                        <a id=\"disabled-nav-tail\" href=\"#tail\">Tail link</a>\n                        <div>\n                            <button id=\"disabled-nav-nested\" type=\"button\" aria-disabled=\"true\" tabindex=\"-1\">\n                                Nested component-owned control\n                            </button>\n                        </div>\n                    </cem-nav>\n                    <button id=\"disabled-navigation-end\" type=\"button\">End</button>\n                </form>\n            </section>\n        ",
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer, focus and native keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        const runtime = cemStoryRuntime();
        for (const wrapper of root.querySelectorAll<HTMLElement>('cem-stack, cem-table')) await whenCemRendered(wrapper);
        for (const host of root.querySelectorAll<HTMLElement>('cem-nav, cem-tabs, cem-list, cem-chip')) await whenCemRendered(host);
        await nextRenderFrame();
        await waitForStateSelector(root, '#disabled-nav-native:disabled');

        const form = harness.query<HTMLFormElement>('#disabled-navigation-form');
        const start = harness.query<HTMLButtonElement>('#disabled-navigation-start');
        const end = harness.query<HTMLButtonElement>('#disabled-navigation-end');
        const navHost = harness.query<HTMLElement>('cem-nav[label="Disabled primary navigation"]');
        const navWrapper = harness.query<HTMLElement>('cem-nav[label="Disabled primary navigation"] > nav');
        const ariaDisabledLink = harness.query<HTMLAnchorElement>('#disabled-nav-current');
        const ariaDisabledLinkLabel = harness.query<HTMLElement>('#disabled-nav-current-label');
        const ariaDisabledButton = harness.query<HTMLButtonElement>('#disabled-nav-submit');
        const nativeDisabledButton = harness.query<HTMLButtonElement>('#disabled-nav-native');
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
        ] as const;
        const nativeCases = [
            { host: navHost, owner: nativeDisabledButton, wrapper: navWrapper },
        ] as const;
        const focusOrder: readonly HTMLElement[] = [
            harness.query<HTMLAnchorElement>('#disabled-nav-enabled'),
            ariaDisabledLink,
            ariaDisabledButton,
            harness.query<HTMLAnchorElement>('#disabled-nav-tail'),
        ];
        const focusEvents: string[] = [];
        harness.root.addEventListener('focusin', (event) => {
            if (event.target instanceof HTMLElement && focusOrder.includes(event.target)) {
                focusEvents.push(event.target.id);
            }
        });

        assertStateHostsRendered(harness.root, 'cem-nav');
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
        expect(ariaOwners.map((owner) => owner.getAttribute('tabindex'))).toEqual([null, null]);
        expect(ariaDisabledLink.getAttribute('aria-current')).toBe('page');
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

export const DisclosureAndPassive = meta.story({
 name: "toggles collapsible navigation without changing passive landmark semantics",
 render: () => "\n            <form>\n                <cem-stack gap=\"sm\">\n                    <cem-nav label=\"Primary navigation\">\n                        <a href=\"#overview\">Overview</a>\n                    </cem-nav>\n                    <cem-nav label=\"Workspace navigation\" collapsible>\n                        <a href=\"#workspace\">Workspace</a>\n                    </cem-nav>\n                    <cem-nav label=\"Admin navigation\" collapsible expanded>\n                        <a href=\"#admin\">Admin</a>\n                    </cem-nav>\n                </cem-stack>\n            </form>\n        ",
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer, focus and native keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        const runtime = cemStoryRuntime();
        for (const wrapper of root.querySelectorAll<HTMLElement>('cem-stack, cem-table')) await whenCemRendered(wrapper);
        for (const host of root.querySelectorAll<HTMLElement>('cem-nav, cem-tabs, cem-list, cem-chip')) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});
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

export const MinimalSubstrate = meta.story({
 name: "registers and renders the exact minimal Phase 3 set through the accepted substrate",
 render: () => "\n            <div data-phase3-minimal-primitives>\n                <cem-surface label=\"Workspace\"><span>Surface content</span></cem-surface>\n                <cem-text>Ready</cem-text>\n                <cem-stack gap=\"sm\"><span>Stack content</span></cem-stack>\n                <cem-grid columns=\"2\"><span>Grid content</span></cem-grid>\n                <cem-list label=\"Tasks\"><li>Review</li></cem-list>\n                <cem-nav label=\"Sections\"><a href=\"#profile\">Profile</a></cem-nav>\n            </div>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement, runtime = cemStoryRuntime();
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        const substrateHarness = { ...harness, settle: (host: HTMLElement) => whenCemRendered(host), snapshot: (host: HTMLElement) => runtime.snapshotInstance(host) };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);
        expect(PHASE3_MINIMAL_PRIMITIVE_TAGS.every((tag) => Boolean(customElements.get(tag)))).toBe(true);


        const hosts = PHASE3_MINIMAL_PRIMITIVE_TAGS.map((tag) => substrateHarness.query<HTMLElement>(tag));
        await Promise.all(hosts.map((host) => substrateHarness.settle(host)));

        for (const host of hosts) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
            expect(host.querySelector(':scope > template[data-cem-island]')).not.toBeNull();
            expect(substrateHarness.snapshot(host).outputTarget).toBe('light-dom');
        }

        expect(assertAccessibleName(substrateHarness.query('cem-surface section'), 'Workspace')).toBe('Workspace');
        expect(assertAccessibleName(substrateHarness.query('cem-list ul'), 'Tasks')).toBe('Tasks');
        expect(assertAccessibleName(substrateHarness.query('cem-nav nav'), 'Sections')).toBe('Sections');
        expect(() => assertAriaReferenceIntegrity(substrateHarness.root)).not.toThrow();
 },
});

export const LandmarksAndTabs = meta.story({
 name: "renders navigation-family MVP primitives as accessible landmarks and tablists",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-app-bar label=\"CEM Admin\">\n                    <span slot=\"title\">CEM Admin</span>\n                    <button type=\"button\">Settings</button>\n                </cem-app-bar>\n                <cem-nav label=\"Primary\">\n                    <a href=\"#overview\">Overview</a>\n                    <a href=\"#assets\">Assets</a>\n                </cem-nav>\n                <cem-tabs label=\"Profile sections\">\n                    <cem-tab value=\"overview\" label=\"Overview\"><p>Overview panel</p></cem-tab>\n                    <cem-tab value=\"security\" label=\"Security\"><p>Security panel</p></cem-tab>\n                </cem-tabs>\n            </cem-stack>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);

        await waitForPrimitive(root, 'cem-tabs [role="tablist"]');

        const appBar = harness.query<HTMLElement>('cem-app-bar header');
        const appBarTitle = harness.query<HTMLElement>('cem-app-bar .cem-app-bar__title');
        const appBarAction = harness.query<HTMLButtonElement>('cem-app-bar button');
        const nav = harness.query<HTMLElement>('cem-nav nav');
        const tablist = harness.query<HTMLElement>('cem-tabs [role="tablist"]');
        const tabs = Array.from(harness.root.querySelectorAll<HTMLButtonElement>('cem-tabs [role="tab"]'));

        for (const host of Array.from(harness.root.querySelectorAll<HTMLElement>('cem-app-bar, cem-nav, cem-tabs'))) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
        }

        expect(appBar.getAttribute('role')).toBe('banner');
        expect(assertAccessibleName(appBar, 'CEM Admin')).toBe('CEM Admin');
        expect(appBarTitle.textContent?.trim()).toBe('CEM Admin');
        expect(assertAccessibleName(appBarAction, 'Settings')).toBe('Settings');
        expect(assertAccessibleName(nav, 'Primary')).toBe('Primary');
        expect(nav.querySelectorAll('a')).toHaveLength(2);
        expect(tablist.getAttribute('role')).toBe('tablist');
        expect(assertAccessibleName(tablist, 'Profile sections')).toBe('Profile sections');
        expect(tabs).toHaveLength(2);
        expect(tabs[0]?.getAttribute('aria-selected')).toBe('true');
        expect(tabs[1]?.getAttribute('aria-selected')).toBe('false');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

export const LayoutComposition = meta.story({
 name: "renders layout, list, navigation, and surface primitives",
 render: () => "\n            <cem-surface label=\"Account\">\n                <cem-grid columns=\"2\" gap=\"lg\">\n                    <cem-list label=\"Tasks\">\n                        <li>Review</li>\n                        <li>Approve</li>\n                    </cem-list>\n                    <cem-nav label=\"Sections\">\n                        <a href=\"#profile\">Profile</a>\n                    </cem-nav>\n                </cem-grid>\n            </cem-surface>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);

        await Promise.all([
            waitForPrimitive(root, 'cem-surface section'),
            waitForPrimitive(root, 'cem-grid .cem-grid'),
            waitForPrimitive(root, 'cem-list li:not(.cem-list__empty)'),
            waitForPrimitive(root, 'cem-nav nav'),
        ]);

        const surface = harness.query<HTMLElement>('cem-surface section');
        const grid = harness.query<HTMLElement>('cem-grid .cem-grid');
        const list = harness.query<HTMLUListElement>('cem-list ul');
        const nav = harness.query<HTMLElement>('cem-nav nav');

        expect(surface.getAttribute('aria-label')).toBe('Account');
        expect(grid.getAttribute('data-columns')).toBe('2');
        expect(grid.getAttribute('data-gap')).toBe('lg');
        expect(assertAccessibleName(list, 'Tasks')).toBe('Tasks');
        expect(list.querySelectorAll('li')).toHaveLength(2);
        expect(assertAccessibleName(nav, 'Sections')).toBe('Sections');
        expect(nav.querySelector('a')?.textContent).toBe('Profile');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

export const FamilyVisualSnapshots = meta.story({
 name: "captures primitive-family visual snapshots from rendered light DOM",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-surface label=\"Dashboard\">\n                    <cem-card label=\"Summary\">\n                        <span slot=\"title\">Summary</span>\n                        <cem-badge tone=\"success\">Ready</cem-badge>\n                    </cem-card>\n                </cem-surface>\n                <cem-nav label=\"Primary\">\n                    <a href=\"#home\">Home</a>\n                    <a href=\"#settings\">Settings</a>\n                </cem-nav>\n                <cem-alert tone=\"warning\" role=\"alert\">Check required fields.</cem-alert>\n            </cem-stack>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);

        await waitForPrimitive(root, 'cem-alert [role="alert"]');
        await waitForPrimitive(root, 'cem-card section');
        await waitForPrimitive(root, 'cem-badge .cem-badge');

        assertPrimitiveVisualSnapshot(captureVisualSnapshot(harness.query<HTMLElement>('cem-surface section')), {
            display: 'block',
            family: 'layout/content containers',
            htmlIncludes: ['class="cem-surface cem-surface--default"', 'aria-label="Dashboard"', 'cem-card'],
            tagName: 'section',
            text: 'Summary Ready',
        });
        assertPrimitiveVisualSnapshot(captureVisualSnapshot(harness.query<HTMLElement>('cem-nav nav')), {
            display: 'block',
            family: 'navigation landmarks',
            htmlIncludes: ['class="cem-nav"', 'aria-label="Primary"', 'href="#home"'],
            tagName: 'nav',
            text: 'HomeSettings',
        });
        assertPrimitiveVisualSnapshot(captureVisualSnapshot(harness.query<HTMLElement>('cem-alert [role="alert"]')), {
            display: 'block',
            family: 'feedback/status surfaces',
            htmlIncludes: ['class="cem-alert cem-alert--warning"', 'data-tone="warning"', 'role="alert"'],
            tagName: 'div',
            text: 'Check required fields.',
        });
 },
});
async function waitForPrimitive(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + 1000;

    while (Date.now() < deadline) {
        const element = root.querySelector(selector);

        if (element) {
            return element;
        }

        await nextRenderFrame();
    }

    throw new Error(`Expected primitive render output matching ${selector}`);
}

interface PrimitiveVisualSnapshotExpectation {
    display?: string;
    family: string;
    htmlIncludes: string[];
    tagName: string;
    text: string;
}

function assertPrimitiveVisualSnapshot(
    snapshot: VisualSnapshot,
    expectation: PrimitiveVisualSnapshotExpectation,
): void {
    expect(snapshot.tagName).toBe(expectation.tagName);
    expect(snapshot.text).toBe(expectation.text);
    expect(snapshot.rect.width).toBeGreaterThan(0);
    expect(snapshot.rect.height).toBeGreaterThan(0);
    expect(snapshot.styles.visibility).toBe('visible');
    expect(snapshot.styles.display).not.toBe('none');
    expect(snapshot.styles.display).not.toBe('contents');
    expect(snapshot.styles.color).not.toBe('');
    expect(snapshot.styles['font-size']).not.toBe('');

    if (expectation.display) {
        expect(snapshot.styles.display).toBe(expectation.display);
    }

    for (const html of expectation.htmlIncludes) {
        expect(snapshot.html).toContain(html);
    }
}
const PHASE3_MINIMAL_PRIMITIVE_TAGS = [
    'cem-surface',
    'cem-text',
    'cem-stack',
    'cem-grid',
    'cem-list',
    'cem-nav',
] as const;
