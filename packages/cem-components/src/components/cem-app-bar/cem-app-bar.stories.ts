import { expect } from 'storybook/test';
import preview, { assertLightDomRendered, assertAriaReferenceIntegrity, nextRenderFrame, loadCemWorkflowDependencies, loadCemComponent, loadCemDeclaration, whenCemRendered, createCemStoryRuntime, assertAccessibleName } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-app-bar.xhtml?raw';
const meta = preview.meta({ component: 'cem-app-bar', title: 'CEM Components/cem-app-bar', loaders: [async () => { await loadCemDeclaration('cem-app-bar', declarationSource); return {}; }] });

export const CompactHeading = meta.story({
    render: () => '<cem-app-bar appearance="compact" label="Studio"><h1 slot="title">Website · content/article.md - CEM Studio</h1><button type="button">Theme</button></cem-app-bar>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-app-bar'); await whenCemRendered(host);
        host.style.cssText = '--cem-control-padding-x:6px;--cem-control-padding-y:2px;--cem-inset-container:33px';
        const owner = required<HTMLElement>(host, '[part=root]'), heading = required<HTMLElement>(host, 'h1');
        expect(getComputedStyle(owner).paddingBlockStart).toBe('2px');
        expect(getComputedStyle(owner).paddingInlineStart).toBe('6px');
        expect(getComputedStyle(heading).marginBlockStart).toBe('0px');
        expect(getComputedStyle(heading).fontSize).toBe(getComputedStyle(owner).fontSize);
        expect(heading.textContent).toContain('content/article.md');
        host.setAttribute('label', 'Updated Studio'); await whenCemRendered(host);
        expect(host.querySelector('h1')).toBe(heading);
        host.removeAttribute('appearance'); await whenCemRendered(host);
        expect(getComputedStyle(owner).paddingBlockStart).toBe('33px');
    },
});

export const SuperCompactActions = meta.story({
    loaders: [async () => { await loadCemComponent('cem-icon'); await loadCemComponent('cem-icon-button'); return {}; }],
    render: () => '<section data-theme="cem-theme-light" data-cem-density="super-compact" data-cem-spacing="normal"><cem-app-bar appearance="compact" label="Studio"><h1 slot="title">Site · content/article.md - CEM Studio</h1><cem-icon-button image="◐" aria-label="Choose theme" variant="contextual"></cem-icon-button></cem-app-bar></section>',
    play: async ({ canvasElement }) => {
        const boundary = required<HTMLElement>(canvasElement, '[data-cem-density]');
        const host = required<HTMLElement>(boundary, 'cem-app-bar'); await whenCemRendered(host);
        const action = required<HTMLElement>(host, 'cem-icon-button'); await whenCemRendered(action);
        await whenCemRendered(required<HTMLElement>(action, 'cem-icon'));
        const header = required<HTMLElement>(host, '[part=root]');
        const actions = required<HTMLElement>(host, '[part=actions]');
        const title = required<HTMLElement>(host, '[part=title]');
        const button = required<HTMLButtonElement>(action, 'button');
        const expectContentHeight = () => {
            const style = getComputedStyle(header);
            const contentHeight = Math.max(title.getBoundingClientRect().height, actions.getBoundingClientRect().height);
            const chrome = Number.parseFloat(style.paddingBlockStart) + Number.parseFloat(style.paddingBlockEnd)
                + Number.parseFloat(style.borderTopWidth) + Number.parseFloat(style.borderBottomWidth);
            expect(Math.abs(header.getBoundingClientRect().height - contentHeight - chrome)).toBeLessThan(0.1);
        };
        for (const theme of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            boundary.setAttribute('data-theme', `cem-theme-${theme}`); await nextRenderFrame();
            const style = getComputedStyle(button);
            expect(assertAccessibleName(button, 'Choose theme')).toBe('Choose theme');
            expectContentHeight();
            expect(style.getPropertyValue('--cem-control-height').trim()).toBe('0px');
            expect(style.getPropertyValue('--cem-coupling-zone-min').trim()).toBe('0px');
            expect(style.getPropertyValue('--cem-control-padding-y').trim()).toBe('0px');
            expect(style.minBlockSize).toBe('0px');
            expect(style.paddingBlockStart).toBe('0px');
            expect(style.paddingBlockEnd).toBe('0px');
            expect(style.paddingInlineStart).toBe('0px');
            expect(style.borderTopWidth).toBe('0px');
            expect(getComputedStyle(header).paddingBlockStart).toBe('0px');
            expect(getComputedStyle(header).paddingBlockEnd).toBe('0px');
            const contentHeight = required<HTMLElement>(action, 'cem-icon').getBoundingClientRect().height;
            expect(Math.abs(button.getBoundingClientRect().height - contentHeight)).toBeLessThan(0.1);
            expect(button.getBoundingClientRect().height).toBeLessThan(48);
            expect(header.getBoundingClientRect().height).toBeLessThan(48);
        }
        boundary.style.cssText = '--cem-control-height:42px;--cem-coupling-zone-min:46px;--cem-control-padding-y:3px';
        await nextRenderFrame();
        expect(getComputedStyle(button).minBlockSize).toBe('46px');
        expect(getComputedStyle(button).paddingBlockStart).toBe('3px');
        expect(getComputedStyle(header).paddingBlockStart).toBe('3px');
        expect(button.getBoundingClientRect().height).toBeGreaterThanOrEqual(46);
        expectContentHeight();
        boundary.removeAttribute('style');
        action.setAttribute('size', 'large'); await whenCemRendered(action);
        await whenCemRendered(required<HTMLElement>(action, 'cem-icon'));
        const largeHeight = 4 * Number.parseFloat(getComputedStyle(document.documentElement).fontSize);
        expect(Number.parseFloat(getComputedStyle(button).minBlockSize)).toBeCloseTo(largeHeight);
        expect(button.getBoundingClientRect().height).toBeGreaterThanOrEqual(largeHeight);
        expectContentHeight();
        action.removeAttribute('size'); await whenCemRendered(action);
        await whenCemRendered(required<HTMLElement>(action, 'cem-icon'));
        expect(action.querySelector('button')).toBe(button);
        expect(getComputedStyle(button).minBlockSize).toBe('0px');
        expect(button.getBoundingClientRect().height).toBeLessThan(48);
        expectContentHeight();
    },
});
function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
export const Semantics = meta.story({
    render: () => '<cem-app-bar label="Workspace"><strong slot="title">Project</strong><button type="button">Account</button></cem-app-bar>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-app-bar'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]');
        expect(host.shadowRoot).toBeNull(); expect(owner.tabIndex).toBe(-1);
        expect(owner.localName).toBe('header'); expect(owner.getAttribute('role')).toBe('banner'); expect(assertAccessibleName(owner, 'Workspace')).toBe('Workspace'); expect(required(host, '[part=title]').textContent).toBe('Project'); expect(required(host, '[part=actions] button').textContent).toBe('Account');
    },
});

export const WorkerRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-app-bar-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.setAttribute('label', 'Original');
        host.innerHTML = '<strong>Retained payload</strong>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), payload = required(host, 'strong');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-tab', 'cem-step', 'cem-tree-activate']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('dir', 'rtl');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            owner.click(); expect(events).toEqual([]); expect(owner.tabIndex).toBe(-1);
            expect(getComputedStyle(owner).direction).toBe('rtl');
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const FallbackRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, true);
        const tag = `cem-app-bar-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
        const host = document.createElement(tag); host.setAttribute('label', 'Original');
        host.innerHTML = '<strong>Retained payload</strong>'; canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const owner = required<HTMLElement>(host, '[part=root]'), payload = required(host, 'strong');
            const events: string[] = []; for (const type of ['input', 'change', 'cem-tab', 'cem-step', 'cem-tree-activate']) host.addEventListener(type, () => events.push(type));
            host.setAttribute('label', 'Updated'); host.setAttribute('dir', 'rtl');
            await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            owner.click(); expect(events).toEqual([]); expect(owner.tabIndex).toBe(-1);
            expect(getComputedStyle(owner).direction).toBe('rtl');
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            expect(host.querySelector('[part=root]')).toBe(owner); expect(host.querySelector('strong')).toBe(payload);
            host.hidden = true; await runtime.whenRenderSettled(host); expect(getComputedStyle(host).display).toBe('none');
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
});

export const ThemeInheritance = meta.story({
    render: () => '<div class="theme-audit"><cem-app-bar label="Example" initials="EX">Example</cem-app-bar></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, '.theme-audit'), host = required<HTMLElement>(container, 'cem-app-bar'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-typography-ui-font-size', '19px');
            expect(getComputedStyle(owner).fontSize).toBe('19px');
            container.style.setProperty('--cem-palette-comfort', 'rgb(24, 36, 48)'); expect(getComputedStyle(owner).backgroundColor).toBe('rgb(24, 36, 48)');
        }
    },
});

const assetBrowserWorkflowMarkup = `<cem-app-bar label="Asset browser">
  <span slot="title">Asset browser</span>
  <cem-icon-button image="settings" label="Open settings"></cem-icon-button>
</cem-app-bar>

<cem-tabs label="Asset views">
  <cem-tab value="files" label="Files"><p>Files view</p></cem-tab>
  <cem-tab value="activity" label="Activity"><p>Activity view</p></cem-tab>
</cem-tabs>

<cem-card label="Assets">
  <span slot="title">Assets</span>
  <cem-chip label="Filtered by owner">Owner</cem-chip>
  <cem-badge tone="success">Ready</cem-badge>

  <cem-table label="Asset table">
    <div role="row">
      <span role="columnheader">Name</span>
      <span role="columnheader">Status</span>
    </div>
    <div role="row">
      <span role="cell">Policy.pdf</span>
      <span role="cell">Ready</span>
    </div>
  </cem-table>

  <cem-media-preview label="Policy preview">
    <img src="/policy.png" alt="Policy thumbnail">
    <span slot="caption">Policy preview</span>
  </cem-media-preview>
</cem-card>
`;
export const AssetBrowserWorkflow = meta.story({
    name: 'renders the asset browser workflow without app JavaScript',
    loaders: [async () => { await loadCemWorkflowDependencies(); return {}; }],
    render: () => `<section aria-label="workflow fixture">${assetBrowserWorkflowMarkup}</section>`,
    play: async ({ canvasElement }) => {
        expect(assetBrowserWorkflowMarkup).not.toMatch(/<script\b/i);
        expect(assetBrowserWorkflowMarkup).not.toMatch(/\son[a-z]+\s*=/i);
        expect(assetBrowserWorkflowMarkup).toMatch(/<cem-[a-z-]+/);
        await nextRenderFrame();
        await waitForWorkflowSelector(canvasElement, 'cem-media-preview figure');

        const appBar = workflowQuery<HTMLElement>(canvasElement, 'cem-app-bar header');
        const settings = workflowQuery<HTMLButtonElement>(canvasElement, 'cem-icon-button button');
        const tablist = workflowQuery<HTMLElement>(canvasElement, 'cem-tabs [role="tablist"]');
        const table = workflowQuery<HTMLElement>(canvasElement, 'cem-table [role="table"]');
        const badge = workflowQuery<HTMLElement>(canvasElement, 'cem-badge .cem-badge');
        const media = workflowQuery<HTMLElement>(canvasElement, 'cem-media-preview figure');
        const image = workflowQuery<HTMLImageElement>(canvasElement, 'cem-media-preview img');

        expect(assertAccessibleName(appBar, 'Asset browser')).toBe('Asset browser');
        expect(assertAccessibleName(settings, 'Open settings')).toBe('Open settings');
        expect(assertAccessibleName(tablist, 'Asset views')).toBe('Asset views');
        expect(table.querySelectorAll('[role="row"]')).toHaveLength(2);
        expect(assertAccessibleName(table, 'Asset table')).toBe('Asset table');
        expect(badge.getAttribute('data-tone')).toBe('success');
        expect(badge.textContent?.trim()).toBe('Ready');
        expect(assertAccessibleName(media, 'Policy preview')).toBe('Policy preview');
        expect(image.getAttribute('alt')).toBe('Policy thumbnail');
        assertWorkflowIntegrity(canvasElement);
    },
});

const WORKFLOW_RENDER_TIMEOUT_MS = 3000;
function workflowQuery<T extends Element = Element>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Expected workflow fixture to contain ${selector}`);
    return element;
}
async function waitForWorkflowSelector(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + WORKFLOW_RENDER_TIMEOUT_MS;
    while (Date.now() < deadline) {
        const found = root.querySelector(selector);
        if (found) {
            return found;
        }
        await nextRenderFrame();
    }
    throw new Error(`Expected workflow render output matching ${selector}`);
}

function assertWorkflowIntegrity(root: ParentNode): void {
    for (const host of Array.from(root.querySelectorAll<HTMLElement>('cem-card, cem-list, cem-action, cem-text-field, cem-textarea, cem-checkbox, cem-radio, cem-switch, cem-app-bar, cem-tabs, cem-table, cem-media-preview, cem-alert, cem-toast, cem-progress, cem-skeleton, cem-dialog, cem-sheet, cem-avatar, cem-icon-button, cem-chip, cem-badge'))) {
        assertLightDomRendered(host);
        expect(host.shadowRoot).toBeNull();
    }
    expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
}
