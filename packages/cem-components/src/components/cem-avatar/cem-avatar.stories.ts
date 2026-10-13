import { expect } from 'storybook/test';
import preview, { assertLightDomRendered, assertAriaReferenceIntegrity, nextRenderFrame, loadCemWorkflowDependencies, loadCemDeclaration, whenCemRendered, createCemStoryRuntime, assertAccessibleName } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-avatar.xhtml?raw';
const meta = preview.meta({ component: 'cem-avatar', title: 'CEM Components/cem-avatar', loaders: [async () => { await loadCemDeclaration('cem-avatar', declarationSource); return {}; }] });
function required<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
export const Semantics = meta.story({
    render: () => '<cem-avatar label="Ada Lovelace" initials="AL"></cem-avatar>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-avatar'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]');
        expect(host.shadowRoot).toBeNull(); expect(owner.tabIndex).toBe(-1);
        expect(assertAccessibleName(owner, 'Ada Lovelace')).toBe('Ada Lovelace'); expect(owner.getAttribute('role')).toBe('img'); expect(owner.textContent).toBe('AL'); host.setAttribute('initials', 'AB'); host.setAttribute('label', 'Ada Byron'); await whenCemRendered(host); expect(host.querySelector('[part=root]')).toBe(owner); expect(owner.textContent).toBe('AB'); expect(assertAccessibleName(owner, 'Ada Byron')).toBe('Ada Byron');
    },
});

export const WorkerRetention = meta.story({
    render: () => '<div></div>',
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `cem-avatar-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
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
        const tag = `cem-avatar-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
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
    render: () => '<div class="theme-audit"><cem-avatar label="Example" initials="EX">Example</cem-avatar></div>',
    play: async ({ canvasElement }) => {
        const container = required<HTMLElement>(canvasElement, '.theme-audit'), host = required<HTMLElement>(container, 'cem-avatar'); await whenCemRendered(host);
        const owner = required<HTMLElement>(host, '[part=root]');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-typography-initialism-font-size', '19px');
            expect(getComputedStyle(owner).fontSize).toBe('19px');
            container.style.setProperty('--cem-palette-comfort', 'rgb(24, 36, 48)'); expect(getComputedStyle(owner).backgroundColor).toBe('rgb(24, 36, 48)'); container.style.setProperty('--cem-bend-avatar', '13px'); expect(getComputedStyle(owner).borderTopLeftRadius).toBe('13px');
        }
    },
});

const profileEditorWorkflowMarkup = `<cem-card label="Profile editor">
  <span slot="title">Profile editor</span>
  <cem-stack gap="sm">
    <cem-avatar label="Ada Lovelace" initials="AL"></cem-avatar>
    <cem-text-field label="Display name" name="display-name" value="Ada Lovelace"></cem-text-field>
    <cem-textarea label="Bio" name="bio" value="Analytical engine notes"></cem-textarea>
    <cem-switch name="public-profile" value="yes">Public profile</cem-switch>
    <cem-alert tone="info" role="status">Profile changes are saved locally until submitted.</cem-alert>
    <cem-action variant="primary">Save profile</cem-action>
  </cem-stack>
</cem-card>
`;
export const ProfileEditorWorkflow = meta.story({
    name: 'renders the profile editor workflow without app JavaScript',
    loaders: [async () => { await loadCemWorkflowDependencies(); return {}; }],
    render: () => `<section aria-label="workflow fixture">${profileEditorWorkflowMarkup}</section>`,
    play: async ({ canvasElement }) => {
        expect(profileEditorWorkflowMarkup).not.toMatch(/<script\b/i);
        expect(profileEditorWorkflowMarkup).not.toMatch(/\son[a-z]+\s*=/i);
        expect(profileEditorWorkflowMarkup).toMatch(/<cem-[a-z-]+/);
        await nextRenderFrame();
        await waitForWorkflowSelector(canvasElement, 'cem-action button');
        await waitForWorkflowText(canvasElement, 'cem-switch .cem-switch__label', 'Public profile');

        const avatar = workflowQuery<HTMLElement>(canvasElement, 'cem-avatar [role="img"]');
        const displayName = workflowQuery<HTMLInputElement>(canvasElement, 'cem-text-field[name="display-name"] input');
        const bio = workflowQuery<HTMLTextAreaElement>(canvasElement, 'cem-textarea[name="bio"] textarea');
        const publicProfile = workflowQuery<HTMLInputElement>(canvasElement, 'cem-switch input[name="public-profile"]');
        const alert = workflowQuery<HTMLElement>(canvasElement, 'cem-alert [role="status"]');
        const save = workflowQuery<HTMLButtonElement>(canvasElement, 'cem-action button');

        expect(assertAccessibleName(avatar, 'Ada Lovelace')).toBe('Ada Lovelace');
        expect(avatar.textContent?.trim()).toBe('AL');
        expect(displayName.getAttribute('value')).toBe('Ada Lovelace');
        expect(assertAccessibleName(displayName, 'Display name')).toBe('Display name');
        expect(bio.value).toBe('Analytical engine notes');
        expect(assertAccessibleName(bio, 'Bio')).toBe('Bio');
        expect(publicProfile.getAttribute('role')).toBe('switch');
        expect(assertAccessibleName(publicProfile, 'Public profile')).toBe('Public profile');
        expect(alert.textContent?.trim()).toBe('Profile changes are saved locally until submitted.');
        expect(assertAccessibleName(save, 'Save profile')).toBe('Save profile');
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

async function waitForWorkflowText(root: ParentNode, selector: string, text: string): Promise<Element> {
    const deadline = Date.now() + WORKFLOW_RENDER_TIMEOUT_MS;
    while (Date.now() < deadline) {
        const found = root.querySelector(selector);
        if (found?.textContent?.includes(text)) {
            return found;
        }
        await nextRenderFrame();
    }
    throw new Error(`Expected workflow render output matching ${selector} to contain ${text}`);
}

function assertWorkflowIntegrity(root: ParentNode): void {
    for (const host of Array.from(root.querySelectorAll<HTMLElement>('cem-card, cem-list, cem-action, cem-text-field, cem-textarea, cem-checkbox, cem-radio, cem-switch, cem-app-bar, cem-tabs, cem-table, cem-media-preview, cem-alert, cem-toast, cem-progress, cem-skeleton, cem-dialog, cem-sheet, cem-avatar, cem-icon-button, cem-chip, cem-badge'))) {
        assertLightDomRendered(host);
        expect(host.shadowRoot).toBeNull();
    }
    expect(() => assertAriaReferenceIntegrity(root)).not.toThrow();
}
