import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, cemStoryRuntime, loadCemComponent, loadCemWorkflowDependencies, assertAccessibleName, assertAriaReferenceIntegrity, assertFocusVisible, assertLightDomRendered, nextRenderFrame, type CemElementRuntime, type DataIslandSnapshot } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-list.xhtml?raw';
import '../../styles.css';
const meta = preview.meta({ component: 'cem-list', title: 'CEM Components/cem-list', loaders: [async () => { await loadCemDeclaration('cem-list', declarationSource); await loadCemComponent('cem-chip');  await loadCemWorkflowDependencies(); return {}; }] });

export const HoverOwners = meta.story({
 name: "styles only interactive content hover owners without changing selection or component state",
 render: () => "\n            <cem-stack class=\"cem-theme-light\" gap=\"sm\">\n                <cem-list id=\"passive-content-list\" label=\"Static topics\">\n                    <li>Static topic</li>\n                </cem-list>\n                <cem-list id=\"interactive-content-list\" label=\"Asset type\" selectable value=\"document\" size=\"3\">\n                    <cem-list-option value=\"image\">Image</cem-list-option>\n                    <cem-list-option value=\"document\" selected>Document</cem-list-option>\n                    <cem-list-option value=\"archive\" disabled>Archive</cem-list-option>\n                </cem-list>\n                <cem-list id=\"disabled-content-list\" label=\"Unavailable asset type\" selectable size=\"2\">\n                    <cem-list-option value=\"image\" selected>Image</cem-list-option>\n                    <cem-list-option value=\"document\">Document</cem-list-option>\n                </cem-list>\n                <cem-chip id=\"passive-content-chip\">Passive chip</cem-chip>\n                <cem-chip id=\"unchecked-content-chip\" checkable>Unchecked chip</cem-chip>\n                <cem-chip id=\"checked-content-chip\" checkable checked>Checked chip</cem-chip>\n                <cem-chip id=\"disabled-content-chip\" checkable checked>Unavailable chip</cem-chip>\n                <cem-table id=\"passive-content-table\" label=\"Static comparison\">\n                    <div role=\"row\"><span role=\"cell\">Static cell</span></div>\n                </cem-table>\n            </cem-stack>\n        ",
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
        await nextRenderFrame();

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
 },
});

export const KeyboardFocus = meta.story({
 name: "moves keyboard focus through interactive content owners without changing selection or checked state",
 render: () => "\n            <section class=\"cem-theme-light\">\n                <button id=\"content-focus-start\" type=\"button\">Start content focus sequence</button>\n                <cem-stack gap=\"sm\">\n                    <cem-list id=\"focus-passive-list\" label=\"Static topics\">\n                        <li>Static topic</li>\n                    </cem-list>\n                    <cem-list id=\"focus-selectable-list\" label=\"Asset type\" selectable value=\"document\" size=\"3\">\n                        <cem-list-option value=\"image\">Image</cem-list-option>\n                        <cem-list-option value=\"document\" selected>Document</cem-list-option>\n                        <cem-list-option value=\"archive\" disabled>Archive</cem-list-option>\n                    </cem-list>\n                    <cem-list id=\"focus-disabled-list\" label=\"Unavailable asset type\" selectable size=\"2\">\n                        <cem-list-option value=\"image\" selected>Image</cem-list-option>\n                        <cem-list-option value=\"document\">Document</cem-list-option>\n                    </cem-list>\n                    <cem-chip id=\"focus-passive-chip\">Passive chip</cem-chip>\n                    <cem-chip id=\"focus-unchecked-chip\" checkable>Unchecked chip</cem-chip>\n                    <cem-chip id=\"focus-checked-chip\" checkable checked>Checked chip</cem-chip>\n                    <cem-chip id=\"focus-disabled-chip\" checkable checked>Unavailable chip</cem-chip>\n                    <cem-table id=\"focus-passive-table\" label=\"Static comparison\">\n                        <div role=\"row\"><span role=\"cell\">Static cell</span></div>\n                    </cem-table>\n                </cem-stack>\n                <button id=\"content-focus-end\" type=\"button\">End content focus sequence</button>\n            </section>\n        ",
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
        await waitForStateSelector(root, '#focus-passive-table > .cem-table');

        const start = harness.query<HTMLButtonElement>('#content-focus-start');
        const end = harness.query<HTMLButtonElement>('#content-focus-end');
        await userEvent.hover(start);
        const listHost = harness.query<HTMLElement>('#focus-selectable-list');
        const listOwner = harness.query<HTMLSelectElement>('#focus-selectable-list > select');
        const uncheckedHost = harness.query<HTMLElement>('#focus-unchecked-chip');
        const uncheckedOwner = harness.query<HTMLButtonElement>('#focus-unchecked-chip > button');
        const checkedHost = harness.query<HTMLElement>('#focus-checked-chip');
        const checkedOwner = harness.query<HTMLButtonElement>('#focus-checked-chip > button');
        const disabledListOwner = harness.query<HTMLSelectElement>('#focus-disabled-list > select');
        const disabledChipOwner = harness.query<HTMLButtonElement>('#focus-disabled-chip > button');
        disabledListOwner.disabled = true;
        disabledChipOwner.disabled = true;
        await nextRenderFrame();

        const cases = [
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
        const passiveCases = [
            {
                host: harness.query<HTMLElement>('#focus-passive-list'),
                owner: harness.query<HTMLElement>('#focus-passive-list > ul'),
            },
            {
                host: harness.query<HTMLElement>('#focus-passive-chip'),
                owner: harness.query<HTMLElement>('#focus-passive-chip > span'),
            },
            {
                host: harness.query<HTMLElement>('#focus-passive-table'),
                owner: harness.query<HTMLElement>('#focus-passive-table > .cem-table'),
            },
        ] as const;
        const baselines = cases.map(({ host, owner }) => captureContentInteractionState(runtime, host, owner));
        const passiveBaselines = passiveCases.map(({ host, owner }) =>
            captureContentInteractionState(runtime, host, owner),
        );
        const focusOrder: string[] = [];
        const focusOwnerNames = new Map<HTMLElement, string>([
            [listOwner, 'selectable-list'],
            [uncheckedOwner, 'unchecked-chip'],
            [checkedOwner, 'checked-chip'],
            [end, 'end'],
        ]);
        const mutationEvents: string[] = [];
        harness.root.addEventListener('focusin', (event) => {
            if (event.target instanceof HTMLElement) {
                const ownerName = focusOwnerNames.get(event.target);
                if (ownerName) {
                    focusOrder.push(ownerName);
                }
            }
        });
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => mutationEvents.push(eventName));
        }

        for (const [index, contentCase] of cases.entries()) {
            const { owner, tokens } = contentCase;
            const baseline = baselines[index];
            if (!baseline) {
                throw new Error(`Expected content focus baseline ${index}`);
            }
            expect(baseline.backgroundColor).toBe(resolveTokenColor(owner, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(owner, tokens.defaultText));
        }

        start.focus();
        expect(document.activeElement).toBe(start);

        for (const [index, contentCase] of cases.entries()) {
            const { host, owner, tokens } = contentCase;
            const baseline = baselines[index];
            if (!baseline) {
                throw new Error(`Expected content focus baseline ${index}`);
            }

            await userEvent.tab();
            await nextRenderFrame();

            const focused = captureContentInteractionState(runtime, host, owner);
            expect(document.activeElement).toBe(owner);
            expect(owner.matches(':focus-visible')).toBe(true);
            expect(paintedColor(focused.focusTreatment[0])).toBe(resolveTokenColor(owner, '--cem-zebra-color-1'));
            expect(focused.focusTreatment[1]).toBe('solid');
            expect(focused.focusTreatment[2]).toBe(`${resolveTokenLength(owner, '--cem-stroke-focus')}px`);
            expect(focused.focusTreatment[3]).toBe(
                `${resolveTokenLength(owner, '--cem-stroke-indicator-offset')}px`,
            );
            expect(focused.backgroundColor).toBe(baseline.backgroundColor);
            expect(focused.color).toBe(baseline.color);
            expect(focused.hostFocusTreatment).toEqual(baseline.hostFocusTreatment);
            expectContentInteractionStructureAndGeometry(focused, baseline);

            if (index > 0) {
                const previousCase = cases[index - 1];
                const previousBaseline = baselines[index - 1];
                if (!previousCase || !previousBaseline) {
                    throw new Error(`Expected previous content focus case ${index - 1}`);
                }
                const restoredPrevious = captureContentInteractionState(
                    runtime,
                    previousCase.host,
                    previousCase.owner,
                );
                expect(previousCase.owner.matches(':focus-visible')).toBe(false);
                expect(restoredPrevious.focusTreatment).toEqual(previousBaseline.focusTreatment);
                expectContentInteractionStructureAndGeometry(restoredPrevious, previousBaseline);
            }

            await userEvent.hover(owner);
            await nextRenderFrame();
            const hoveredFocused = captureContentInteractionState(runtime, host, owner);
            expect(owner.matches(':hover')).toBe(true);
            expect(owner.matches(':focus-visible')).toBe(true);
            expect(hoveredFocused.backgroundColor).toBe(resolveTokenColor(owner, tokens.hoverBackground));
            expect(hoveredFocused.color).toBe(resolveTokenColor(owner, tokens.hoverText));
            expect(hoveredFocused.focusTreatment).toEqual(focused.focusTreatment);
            expectContentInteractionStructureAndGeometry(hoveredFocused, focused);

            await userEvent.unhover(owner);
            await nextRenderFrame();
            const restoredFocus = captureContentInteractionState(runtime, host, owner);
            expect(restoredFocus.backgroundColor).toBe(focused.backgroundColor);
            expect(restoredFocus.color).toBe(focused.color);
            expect(restoredFocus.focusTreatment).toEqual(focused.focusTreatment);
            expectContentInteractionStructureAndGeometry(restoredFocus, focused);

            expect(document.activeElement).not.toBe(disabledListOwner);
            expect(document.activeElement).not.toBe(disabledChipOwner);
            expect(disabledListOwner.matches(':focus-visible')).toBe(false);
            expect(disabledChipOwner.matches(':focus-visible')).toBe(false);
        }

        await userEvent.tab();
        await nextRenderFrame();
        expect(document.activeElement).toBe(end);
        expect(checkedOwner.matches(':focus-visible')).toBe(false);
        const lastBaseline = baselines.at(-1);
        if (!lastBaseline) {
            throw new Error('Expected final content focus baseline');
        }
        const restoredLast = captureContentInteractionState(runtime, checkedHost, checkedOwner);
        expect(restoredLast.focusTreatment).toEqual(lastBaseline.focusTreatment);
        expectContentInteractionStructureAndGeometry(restoredLast, lastBaseline);

        for (const [index, passiveCase] of passiveCases.entries()) {
            const { host, owner } = passiveCase;
            const baseline = passiveBaselines[index];
            if (!baseline) {
                throw new Error(`Expected passive content focus baseline ${index}`);
            }
            owner.focus();
            expect(document.activeElement).toBe(end);
            expect(owner.tabIndex).toBe(-1);
            expect(owner.matches(':focus-visible')).toBe(false);
            expectContentInteractionStructureAndGeometry(
                captureContentInteractionState(runtime, host, owner),
                baseline,
            );
        }

        expect(focusOrder).toEqual([
            'selectable-list',
            'unchecked-chip',
            'checked-chip',
            'end',
        ]);
        expect(listOwner.value).toBe('document');
        expect(listOwner.options[1]?.getAttribute('aria-selected')).toBe('true');
        expect(listOwner.options[2]?.disabled).toBe(true);
        expect(uncheckedOwner.getAttribute('aria-pressed')).toBe('false');
        expect(checkedOwner.getAttribute('aria-pressed')).toBe('true');
        expect(mutationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

export const SelectionAndPassive = meta.story({
 name: "selects declarative list options without changing passive list semantics",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-list label=\"Static topics\">\n                    <li>Static topic</li>\n                </cem-list>\n                <cem-list label=\"Asset type\" selectable value=\"document\" size=\"3\">\n                    <cem-list-option value=\"image\" selected>Image</cem-list-option>\n                    <cem-list-option value=\"document\">Document</cem-list-option>\n                    <cem-list-option value=\"archive\" disabled>Archive</cem-list-option>\n                    <span value=\"ignored\">Ignored non-option</span>\n                    <div><cem-list-option value=\"nested\">Ignored nested option</cem-list-option></div>\n                </cem-list>\n                <cem-list label=\"Fallback type\" selectable size=\"3\">\n                    <cem-list-option value=\"image\" selected>Image</cem-list-option>\n                    <cem-list-option value=\"document\" selected>Document</cem-list-option>\n                </cem-list>\n            </cem-stack>\n        ",
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

interface ContentInteractionStateSnapshot {
    backgroundColor: string;
    color: string;
    focusTreatment: readonly string[];
    forcedColorAdjust: string;
    hostAttributes: readonly string[];
    hostBackgroundColor: string;
    hostFocusTreatment: readonly string[];
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
        hostFocusTreatment: [
            hostStyles.outlineColor,
            hostStyles.outlineStyle,
            hostStyles.outlineWidth,
            hostStyles.outlineOffset,
            hostStyles.boxShadow,
        ],
        // snapshotInstance itself advances the hydration revision. Compare every
        // other byte, including live DOM and the remaining retained island.
        hostHtml: host.outerHTML.replace(/(<cem-hydration:field name="dataRevision" kind="string">)[^<]*/g, '$1'),
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
    expect(actual.hostFocusTreatment).toEqual(expected.hostFocusTreatment);
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

export const ContentFamily = meta.story({
 name: "renders content-family MVP primitives as accessible light DOM",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-card label=\"Profile summary\">\n                    <span slot=\"title\">Profile summary</span>\n                    <p>Updated today</p>\n                </cem-card>\n                <cem-list label=\"Assets\">\n                    <li>Policy.pdf</li>\n                    <li>Invoice.csv</li>\n                </cem-list>\n                <cem-table label=\"Asset table\">\n                    <div role=\"row\">\n                        <span role=\"columnheader\">Name</span>\n                        <span role=\"columnheader\">Status</span>\n                    </div>\n                    <div role=\"row\">\n                        <span role=\"cell\">Policy.pdf</span>\n                        <span role=\"cell\">Ready</span>\n                    </div>\n                </cem-table>\n                <cem-chip label=\"Filtered by owner\">Owner</cem-chip>\n                <cem-badge tone=\"success\">Ready</cem-badge>\n                <cem-avatar label=\"Ada Lovelace\" initials=\"AL\"></cem-avatar>\n                <cem-media-preview label=\"Policy preview\">\n                    <img src=\"/policy.png\" alt=\"Policy thumbnail\" />\n                    <span slot=\"caption\">Policy preview</span>\n                </cem-media-preview>\n            </cem-stack>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);

        await waitForPrimitive(root, 'cem-media-preview figure');

        const card = harness.query<HTMLElement>('cem-card section');
        const cardTitle = harness.query<HTMLElement>('cem-card .cem-card__header');
        const list = harness.query<HTMLUListElement>('cem-list ul');
        const table = harness.query<HTMLElement>('cem-table [role="table"]');
        const chip = harness.query<HTMLElement>('cem-chip .cem-chip');
        const badge = harness.query<HTMLElement>('cem-badge .cem-badge');
        const avatar = harness.query<HTMLElement>('cem-avatar .cem-avatar');
        const mediaPreview = harness.query<HTMLElement>('cem-media-preview figure');
        const mediaImage = harness.query<HTMLImageElement>('cem-media-preview img');
        const mediaCaption = harness.query<HTMLElement>('cem-media-preview figcaption');

        for (const host of Array.from(
            harness.root.querySelectorAll<HTMLElement>(
                'cem-card, cem-list, cem-table, cem-chip, cem-badge, cem-avatar, cem-media-preview',
            ),
        )) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
        }

        expect(assertAccessibleName(card, 'Profile summary')).toBe('Profile summary');
        expect(cardTitle.textContent?.trim()).toBe('Profile summary');
        expect(card.textContent).toContain('Updated today');
        expect(assertAccessibleName(list, 'Assets')).toBe('Assets');
        expect(list.querySelectorAll('li')).toHaveLength(2);
        expect(table.getAttribute('role')).toBe('table');
        expect(assertAccessibleName(table, 'Asset table')).toBe('Asset table');
        expect(table.querySelectorAll('[role="row"]')).toHaveLength(2);
        expect(table.querySelectorAll('[role="cell"]')).toHaveLength(2);
        expect(assertAccessibleName(chip, 'Filtered by owner')).toBe('Filtered by owner');
        expect(chip.textContent?.trim()).toBe('Owner');
        expect(badge.getAttribute('data-tone')).toBe('success');
        expect(badge.textContent?.trim()).toBe('Ready');
        expect(avatar.getAttribute('role')).toBe('img');
        expect(assertAccessibleName(avatar, 'Ada Lovelace')).toBe('Ada Lovelace');
        expect(avatar.textContent?.trim()).toBe('AL');
        expect(assertAccessibleName(mediaPreview, 'Policy preview')).toBe('Policy preview');
        expect(mediaImage.getAttribute('alt')).toBe('Policy thumbnail');
        expect(mediaCaption.textContent?.trim()).toBe('Policy preview');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

export const PassiveEmptyStates = meta.story({
 name: "preserves empty states, indeterminate progress, and live-region roles",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-list label=\"Empty tasks\"></cem-list>\n                <cem-table label=\"Empty table\"></cem-table>\n                <cem-progress label=\"Loading assets\"></cem-progress>\n                <cem-toast>Saved</cem-toast>\n                <cem-alert tone=\"danger\" role=\"alert\">Resolve errors.</cem-alert>\n                <cem-skeleton label=\"Loading card\"></cem-skeleton>\n            </cem-stack>\n        ",
 play: async ({ canvasElement }) => {
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('*')) if (host.localName.startsWith('cem-') && customElements.get(host.localName)) await whenCemRendered(host);

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



export const PassiveLoadingBoundary = meta.story({
    render: () => '<section aria-busy="true"><cem-list label="Assets"></cem-list></section><cem-list label="Standalone list" busy></cem-list>',
    play: async ({ canvasElement }) => {
        const section = canvasElement.querySelector('section');
        if (!section) throw new Error('Missing busy ancestor');
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-list')) await whenCemRendered(host);
        const emptyList = section.querySelector('ul');
        if (!emptyList) throw new Error('Missing passive list');
        expect(emptyList.querySelector('.cem-list__empty')?.textContent?.trim()).toBe('No items');
        for (const candidate of canvasElement.querySelectorAll('ul')) {
            expect(candidate.hasAttribute('data-state')).toBe(false);
            expect(candidate.hasAttribute('aria-busy')).toBe(false);
        }
        section.removeAttribute('aria-busy'); await nextRenderFrame();
        const settledEmptyList = section.querySelector('ul');
        expect(settledEmptyList).toBe(emptyList);
        expect(settledEmptyList?.querySelector('.cem-list__empty')?.textContent?.trim()).toBe('No items');
    },
});
