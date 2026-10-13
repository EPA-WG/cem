import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, nextRenderFrame, whenCemRendered, assertLightDomRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-tabs.xhtml?raw';
const meta = preview.meta({ component: 'cem-tabs', title: 'CEM Components/cem-tabs', loaders: [async () => { await loadCemDeclaration('cem-tabs', declarationSource); return {}; }] });
interface TestCemTabs extends HTMLElement {
    selectedIndex: number;
}

interface TabsParts {
    host: TestCemTabs;
    list: HTMLElement;
    panels: HTMLElement[];
    panelsOwner: HTMLElement;
    tabs: HTMLButtonElement[];
}


export const NativeOwners = meta.story({
 name: "renders one labeled tablist with exact native tabs and stable reciprocal panels",
 render: () => tabsContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
        expect(tabsContractFixture).not.toMatch(/<script\b/i);
        expect(tabsContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-tabs')) await whenCemRendered(host);
        const parts = tabsParts(root, '#workspace-tabs');

        expect(parts.list.parentElement).toBe(parts.host);
        expect(parts.panelsOwner.parentElement).toBe(parts.host);
        expect(parts.list.getAttribute('role')).toBe('tablist');
        expect(parts.list.getAttribute('aria-orientation')).toBe('horizontal');
        expect(parts.list).toHaveAccessibleName('Workspace panes');
        expect(parts.tabs).toHaveLength(4);
        expect(parts.panels).toHaveLength(4);
        expect(parts.host.selectedIndex).toBe(1);
        expect(parts.tabs.filter((tab) => tab.getAttribute('aria-selected') === 'true')).toEqual([parts.tabs[1]]);
        expect(parts.panels.filter((panel) => !panel.hidden)).toEqual([parts.panels[1]]);
        expect(parts.tabs.filter((tab) => tab.tabIndex === 0)).toEqual([parts.tabs[1]]);
        expect(parts.tabs[2]?.disabled).toBe(true);
        expect(parts.tabs[2]?.tabIndex).toBe(-1);
        expect(parts.host.querySelectorAll(':scope > cem-tab')).toHaveLength(0);

        for (const [index, tab] of parts.tabs.entries()) {
            const panel = requiredItem(parts.panels, index);
            expect(tab).toBeInstanceOf(HTMLButtonElement);
            expect(tab.type).toBe('button');
            expect(tab.getAttribute('role')).toBe('tab');
            expect(tab.getAttribute('aria-controls')).toBe(panel.id);
            expect(panel.getAttribute('role')).toBe('tabpanel');
            expect(panel.getAttribute('aria-labelledby')).toBe(tab.id);
            expect(panel.tabIndex).toBe(0);
        }
        expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
 },
});

export const NativeActivation = meta.story({
 name: "commits pointer and native keyboard activation once while current and disabled paths stay silent",
 render: () => tabsContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-tabs')) await whenCemRendered(host);
        let parts = tabsParts(root, '#workspace-tabs');
        const editor = requiredElement<HTMLTextAreaElement>(parts.host, '#editor-source');
        const events: Array<{ type: string; detail?: unknown }> = [];
        for (const eventName of ['click', 'input', 'change', 'cem-tab']) {
            parts.host.addEventListener(eventName, (event) => events.push({
                type: event.type,
                detail: event instanceof CustomEvent ? event.detail : undefined,
            }));
        }

        editor.value = 'retained source';
        await userEvent.click(requiredItem(parts.tabs, 3));
        await waitForSelection(parts.host, 3);
        parts = tabsParts(root, '#workspace-tabs');
        expect(document.activeElement).toBe(parts.tabs[3]);
        expect(requiredElement(parts.host, '#editor-source')).toBe(editor);
        expect(editor.value).toBe('retained source');
        expect(events.filter(({ type }) => type === 'cem-tab')).toEqual([{
            type: 'cem-tab',
            detail: { value: 'console', index: 3, previousIndex: 1 },
        }]);

        await userEvent.click(requiredItem(parts.tabs, 3));
        parts.tabs[2]?.click();
        await nextRenderFrame();
        expect(parts.host.selectedIndex).toBe(3);
        expect(events.filter(({ type }) => type === 'cem-tab')).toHaveLength(1);
        expect(events.some(({ type }) => type === 'input' || type === 'change')).toBe(false);
 },
});

export const ManualRovingFocus = meta.story({
 name: "uses manual activation with orientation-aware wrapping roving focus and selected entry restoration",
 render: () => tabsContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
  const { userEvent } = await import('vitest/browser');
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-tabs')) await whenCemRendered(host);
        let horizontal = tabsParts(root, '#workspace-tabs');
        horizontal.tabs[1]?.focus();
        await userEvent.keyboard('{ArrowRight}');
        await nextRenderFrame();
        horizontal = tabsParts(root, '#workspace-tabs');
        expect(document.activeElement).toBe(horizontal.tabs[3]);
        expect(horizontal.host.selectedIndex).toBe(1);
        expect(horizontal.tabs.filter((tab) => tab.tabIndex === 0)).toEqual([horizontal.tabs[3]]);

        await userEvent.keyboard('{ArrowRight}');
        await nextRenderFrame();
        horizontal = tabsParts(root, '#workspace-tabs');
        expect(document.activeElement).toBe(horizontal.tabs[0]);
        await userEvent.keyboard('{End}');
        await nextRenderFrame();
        horizontal = tabsParts(root, '#workspace-tabs');
        expect(document.activeElement).toBe(horizontal.tabs[3]);
        await userEvent.keyboard('{Enter}');
        await waitForSelection(horizontal.host, 3);

        requiredElement<HTMLButtonElement>(root, '[data-tabs-focus-end]').focus();
        await waitFor(() => tabsParts(root, '#workspace-tabs').tabs[3]?.tabIndex === 0);

        let vertical = tabsParts(root, '#vertical-tabs');
        expect(vertical.list.getAttribute('aria-orientation')).toBe('vertical');
        vertical.tabs[0]?.focus();
        await userEvent.keyboard('{ArrowRight}');
        expect(document.activeElement).toBe(vertical.tabs[0]);
        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();
        vertical = tabsParts(root, '#vertical-tabs');
        expect(document.activeElement).toBe(vertical.tabs[2]);
        expect(vertical.host.selectedIndex).toBe(0);
        await userEvent.keyboard(' ');
        await waitForSelection(vertical.host, 2);
 },
});

export const ProgrammaticSelectionAndRecovery = meta.story({
 name: "keeps programmatic changes silent, stabilizes ids, recovers selection, and never hides focused content",
 render: () => tabsContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-tabs')) await whenCemRendered(host);
        let parts = tabsParts(root, '#dynamic-tabs');
        const events: unknown[] = [];
        parts.host.addEventListener('cem-tab', (event) => events.push((event as CustomEvent).detail));
        const stableIds = new Map(parts.tabs.slice(0, 2).map((tab, index) => [
            tab.textContent?.trim(),
            [tab.id, parts.panels[index]?.id],
        ]));
        const retained = requiredElement<HTMLInputElement>(parts.host, '#retained-value');

        retained.value = 'preserved';
        parts.host.selectedIndex = 0;
        await waitForSelection(parts.host, 0);
        expect(requiredElement(parts.host, '#retained-value')).toBe(retained);
        expect(retained.value).toBe('preserved');
        expect(events).toEqual([]);

        parts.host.selectedIndex = 1;
        await waitForSelection(parts.host, 1);
        parts = tabsParts(root, '#dynamic-tabs');
        const focusedPanelAction = requiredElement<HTMLButtonElement>(parts.host, '#focused-panel-action');
        focusedPanelAction.focus();
        parts.host.selectedIndex = 2;
        await waitForSelection(parts.host, 2);
        parts = tabsParts(root, '#dynamic-tabs');
        expect(document.activeElement).toBe(parts.tabs[2]);

        const island = dataIsland(parts.host);
        requiredElement<HTMLElement>(island.content, 'cem-tab[value="three"]').remove();
        await waitFor(() => tabsParts(root, '#dynamic-tabs').tabs.length === 2);
        parts = tabsParts(root, '#dynamic-tabs');
        expect(parts.host.selectedIndex).toBe(1);
        for (const [label, ids] of stableIds) {
            const index = parts.tabs.findIndex((tab) => tab.textContent?.trim() === label);
            expect([parts.tabs[index]?.id, parts.panels[index]?.id]).toEqual(ids);
        }

        const selectedPayload = requiredElement<HTMLElement>(dataIsland(parts.host).content, 'cem-tab[value="two"]');
        selectedPayload.setAttribute('disabled', '');
        await waitFor(() => tabsParts(root, '#dynamic-tabs').host.selectedIndex !== 1);
        parts = tabsParts(root, '#dynamic-tabs');
        expect(parts.host.selectedIndex).toBe(0);
        expect(parts.tabs[0]?.getAttribute('aria-selected')).toBe('true');
        expect(events).toEqual([]);
 },
});

export const MalformedAuthoring = meta.story({
 name: "fails closed for malformed authoring",
 render: () => tabsContractFixture,
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted native pointer and keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        for (const host of root.querySelectorAll<HTMLElement>('cem-tabs')) await whenCemRendered(host);
        const malformed = requiredElement<TestCemTabs>(root, '#malformed-tabs');
        expect(malformed.querySelector('.cem-tabs--invalid')?.hasAttribute('hidden')).toBe(true);
        expect(malformed.querySelector('[role="tablist"], [role="tab"], [role="tabpanel"]')).toBeNull();
 },
});

function tabsParts(root: ParentNode, selector: string): TabsParts {
    const host = requiredElement<TestCemTabs>(root, selector);
    const list = requiredElement<HTMLElement>(host, ':scope > .cem-tabs__list');
    const tabs = Array.from(list.querySelectorAll<HTMLButtonElement>(':scope > button.cem-tabs__tab'));
    const panelsOwner = requiredElement<HTMLElement>(host, ':scope > .cem-tabs__panels');
    const panels = Array.from(panelsOwner.querySelectorAll<HTMLElement>(':scope > .cem-tabs__panel'));
    return { host, list, panels, panelsOwner, tabs };
}

function dataIsland(host: HTMLElement): HTMLTemplateElement {
    return requiredElement<HTMLTemplateElement>(host, 'template[data-cem-island="instance"]');
}

async function waitForSelection(host: TestCemTabs, selected: number): Promise<void> {
    await waitFor(() => host.selectedIndex === selected
        && host.querySelector('[role="tab"][aria-selected="true"]')?.getAttribute('data-tab-index') === String(selected));
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

function assertAriaReferenceIntegrity(root: ParentNode): void {
    for (const node of root.querySelectorAll('[aria-controls], [aria-labelledby], [aria-describedby]')) {
        for (const name of ['aria-controls', 'aria-labelledby', 'aria-describedby']) {
            for (const id of (node.getAttribute(name) ?? '').split(/\s+/).filter(Boolean)) {
                if (!node.ownerDocument.getElementById(id)) throw new Error(`Missing ARIA reference ${id}`);
            }
        }
    }
}

const tabsContractFixture = "<section class=\"cem-theme-light\" aria-label=\"Tabs contract fixture\">\n<button type=\"button\" data-tabs-focus-start>Before tabs</button>\n\n<cem-tabs id=\"workspace-tabs\" label=\"Workspace panes\" selected-index=\"1\">\n    <cem-tab value=\"explorer\" label=\"Explorer\">\n        <input id=\"explorer-filter\" aria-label=\"Explorer filter\" value=\"initial\">\n    </cem-tab>\n    <cem-tab value=\"editor\" label=\"Editor\">\n        <textarea id=\"editor-source\" aria-label=\"Editor source\">source</textarea>\n    </cem-tab>\n    <cem-tab value=\"preview\" label=\"Preview\" disabled>\n        <p>Preview unavailable</p>\n    </cem-tab>\n    <cem-tab value=\"console\" label=\"Console\">\n        <button id=\"clear-console\" type=\"button\">Clear console</button>\n    </cem-tab>\n</cem-tabs>\n\n<cem-tabs id=\"vertical-tabs\" label=\"Result views\" orientation=\"vertical\">\n    <cem-tab value=\"data\" label=\"Data\"><p>Data result</p></cem-tab>\n    <cem-tab value=\"raw\" label=\"Raw\" disabled><p>Raw result</p></cem-tab>\n    <cem-tab value=\"report\" label=\"Report\"><p>Report result</p></cem-tab>\n</cem-tabs>\n\n<cem-tabs id=\"dynamic-tabs\" label=\"Dynamic views\" selected-index=\"1\">\n    <cem-tab value=\"one\" label=\"One\"><input id=\"retained-value\" aria-label=\"Retained value\" value=\"one\"></cem-tab>\n    <cem-tab value=\"two\" label=\"Two\"><button id=\"focused-panel-action\" type=\"button\">Focused panel action</button></cem-tab>\n    <cem-tab value=\"three\" label=\"Three\"><p>Three panel</p></cem-tab>\n</cem-tabs>\n\n<cem-tabs id=\"malformed-tabs\" label=\"Malformed tabs\">\n    <div>Not a tab payload</div>\n</cem-tabs>\n\n<button type=\"button\" data-tabs-focus-end>After tabs</button>\n</section>\n";


export const SelectedState = meta.story({
 name: "reflects selected state on native tabs",
 render: () => "\n            <div gap=\"sm\">\n                <cem-tabs label=\"Sections\">\n                    <cem-tab value=\"current\" label=\"Current\"><p>Current panel</p></cem-tab>\n                    <cem-tab value=\"later\" label=\"Later\"><p>Later panel</p></cem-tab>\n                </cem-tabs>\n            </div>\n        ",
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer, focus and native keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        for (const host of root.querySelectorAll<HTMLElement>('cem-nav, cem-tabs, cem-list, cem-chip')) await whenCemRendered(host);

        const tabs = Array.from(harness.root.querySelectorAll<HTMLButtonElement>('cem-tabs [role="tab"]'));

        assertStateHostsRendered(harness.root, 'cem-tabs');
        expect(tabs.map((tab) => tab.getAttribute('aria-selected')).join('|')).toBe('true|false');

        await nextRenderFrame();

        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
 },
});

function assertStateHostsRendered(root: ParentNode, selector: string): void {
    for (const host of root.querySelectorAll<HTMLElement>(selector)) { assertLightDomRendered(host); expect(host.shadowRoot).toBeNull(); }
}
