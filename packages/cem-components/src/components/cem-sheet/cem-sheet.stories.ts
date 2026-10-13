import { expect, userEvent } from 'storybook/test';
import preview, { assertLightDomRendered, assertAriaReferenceIntegrity, assertAccessibleName, nextRenderFrame, loadCemWorkflowDependencies, loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-sheet.xhtml?raw';

const meta = preview.meta({ component: 'cem-sheet', title: 'CEM Components/cem-sheet', loaders: [async () => { await loadCemDeclaration('cem-sheet', declarationSource); return {}; }] });
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}

export const PersistentRegionAndNames = meta.story({
    render: () => '<section class="cem-theme-light"><cem-sheet><p>Body content</p></cem-sheet><cem-sheet label="Filters" expanded><label>Search <input value="Saved"></label></cem-sheet><cem-sheet label="Fallback" aria-labelledby="sheet-heading" aria-describedby="sheet-help"><strong slot="label" id="sheet-heading">Projected filters</strong><p id="sheet-help">Choose a filter.</p></cem-sheet></section>',
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-sheet')];
        for (const host of hosts) await whenCemRendered(host);
        expect(required(hosts[0], 'aside')).toHaveAccessibleName('Sheet');
        const host = hosts[1], owner = required(host, 'aside'), input = required<HTMLInputElement>(owner, 'input');
        expect(owner.getAttribute('role')).toBe('region');
        expect(owner.hidden).toBe(false); expect(owner).toHaveAccessibleName('Filters');
        expect(required(owner, '[part=heading]').textContent).toBe('Filters');
        input.value = 'Draft'; host.removeAttribute('expanded'); await whenCemRendered(host);
        expect(owner.hidden).toBe(false); expect(required(host, 'aside')).toBe(owner); expect(required(owner, 'input')).toBe(input); expect(input.value).toBe('Draft');
        host.setAttribute('expanded', 'false'); host.setAttribute('label', 'Renamed filters'); await whenCemRendered(host);
        expect(owner.hidden).toBe(false); expect(owner).toHaveAccessibleName('Renamed filters');
        host.setAttribute('aria-label', 'Accessible filters'); await whenCemRendered(host);
        expect(owner).toHaveAccessibleName('Accessible filters'); expect(required(owner, '[part=heading]').textContent).toBe('Renamed filters');
        expect(required(hosts[2], 'aside')).toHaveAccessibleName('Projected filters');
        expect(required(hosts[2], 'aside')).toHaveAccessibleDescription('Choose a filter.');
        expect(required(hosts[2], '[part=heading] > strong').textContent).toBe('Projected filters');
        expect(host.querySelector('style')).toBeNull(); expect(host.shadowRoot).toBeNull();
    },
});

export const TransientFocusAndKeyboard = meta.story({
    render: () => '<section class="cem-theme-light"><button type="button" aria-controls="sheet-task" aria-expanded="false">Application opener</button><cem-sheet id="sheet-task" label="Task" transient><label>Draft <input value="Saved"></label></cem-sheet><button type="button">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-sheet'); await whenCemRendered(host);
        const owner = required(host, 'aside'), input = required<HTMLInputElement>(owner, 'input'), opener = required<HTMLButtonElement>(canvasElement, 'button');
        const events: string[] = []; const record = (event: Event) => events.push(event.type);
        for (const type of ['cem-dismiss', 'cem-open', 'cem-close']) host.addEventListener(type, record);
        expect(owner.hidden).toBe(true); opener.focus(); host.setAttribute('expanded', ''); opener.setAttribute('aria-expanded', 'true'); await whenCemRendered(host);
        expect(owner.hidden).toBe(false); expect(document.activeElement).toBe(opener);
        expect(owner.hasAttribute('aria-modal')).toBe(false); expect(owner.hasAttribute('tabindex')).toBe(false); expect(host.hasAttribute('tabindex')).toBe(false);
        expect(host.querySelector('dialog, [popover], [inert]')).toBeNull();
        await userEvent.type(input, ' draft'); const draft = input.value; const width = owner.getBoundingClientRect().width;
        host.setAttribute('label', 'Renamed task'); await whenCemRendered(host);
        expect(required(host, 'aside')).toBe(owner); expect(required(owner, 'input')).toBe(input); expect(input.value).toBe(draft); expect(document.activeElement).toBe(input);
        expect(owner.getBoundingClientRect().width).toBe(width);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        await native.keyboard('{ArrowRight}'); expect(input.matches(':focus-visible')).toBe(true);
        await native.keyboard('{Escape}'); expect(owner.hidden).toBe(false); expect(document.activeElement).toBe(input);
        await native.keyboard('{Tab}'); expect(document.activeElement).toBe(required(canvasElement, 'section > button:last-child')); expect(owner.hidden).toBe(false);
        opener.focus(); host.removeAttribute('expanded'); opener.setAttribute('aria-expanded', 'false'); await whenCemRendered(host);
        expect(owner.hidden).toBe(true); expect(document.activeElement).toBe(opener); expect(events).toEqual([]);
        for (const type of ['cem-dismiss', 'cem-open', 'cem-close']) host.removeEventListener(type, record);
    },
    parameters: { docs: { description: { story: 'Browser-runner-only trusted keyboard checks cover native focus-visible, Escape and document tab order. The application controls visibility and focus.' } } },
});

export const PresenceFormsAndReconnectWorkerAndFallback = meta.story({
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(root, fallback);
            const tag = `cem-sheet-contract-${crypto.randomUUID()}`, declaration = await declare(declarationSource, tag);
            const host = document.createElement(tag);
            host.innerHTML = '<form><label>Required draft <input name="draft" value="Saved" required></label><button type="reset">Reset</button></form>';
            root.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const owner = required(host, 'aside'), input = required<HTMLInputElement>(owner, 'input'), form = required<HTMLFormElement>(owner, 'form');
                input.value = 'Draft';
                for (const transient of [null, '', 'false', 'true']) {
                    if (transient === null) host.removeAttribute('transient'); else host.setAttribute('transient', transient);
                    for (const expanded of [null, '', 'false', 'true']) {
                        if (expanded === null) host.removeAttribute('expanded'); else host.setAttribute('expanded', expanded);
                        await runtime.whenRenderSettled(host);
                        expect(owner.hidden).toBe(transient !== null && expanded === null);
                        expect(required(host, 'aside')).toBe(owner); expect(required(owner, 'input')).toBe(input); expect(input.value).toBe('Draft');
                        expect(new FormData(form).get('draft')).toBe('Draft');
                    }
                }
                input.value = ''; expect(form.checkValidity()).toBe(false); input.value = 'Draft'; expect(form.checkValidity()).toBe(true);
                await userEvent.click(required(form, 'button')); expect(input.value).toBe('Saved');
                input.value = 'Reconnected'; host.remove(); root.append(host); await runtime.whenRenderSettled(host);
                expect(required(host, 'aside')).toBe(owner); expect(required(owner, 'input')).toBe(input); expect(input.value).toBe('Reconnected');
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); declaration.remove(); scope.dispose(); }
        }
    },
});

const settingsWorkflowMarkup = `<cem-card label="Notification settings">
  <span slot="title">Notification settings</span>
  <cem-switch name="email-notifications">Email notifications</cem-switch>
  <cem-radio name="frequency" value="daily">Daily summary</cem-radio>
  <cem-radio name="frequency" value="weekly">Weekly summary</cem-radio>
  <cem-action variant="primary">Save settings</cem-action>
</cem-card>

<cem-alert tone="warning" role="alert">Review required notification preferences.</cem-alert>
<cem-toast>Settings saved</cem-toast>
<cem-progress label="Sync progress" value="40" max="100"></cem-progress>
<cem-skeleton label="Loading settings"></cem-skeleton>

<cem-dialog label="Confirm changes" trigger="Review changes" close-label="Done">
  <p>Apply these settings to all devices?</p>
  <cem-action variant="primary">Apply</cem-action>
</cem-dialog>

<cem-sheet label="Advanced filters">
  <cem-checkbox name="include-archived" value="yes">Include archived assets</cem-checkbox>
</cem-sheet>
`;
export const SettingsWorkflow = meta.story({
    name: 'renders the settings workflow without app JavaScript',
    loaders: [async () => { await loadCemWorkflowDependencies(); return {}; }],
    render: () => `<section aria-label="workflow fixture">${settingsWorkflowMarkup}</section>`,
    play: async ({ canvasElement }) => {
        expect(settingsWorkflowMarkup).not.toMatch(/<script\b/i);
        expect(settingsWorkflowMarkup).not.toMatch(/\son[a-z]+\s*=/i);
        expect(settingsWorkflowMarkup).toMatch(/<cem-[a-z-]+/);
        await nextRenderFrame();
        await waitForWorkflowSelector(canvasElement, 'cem-sheet aside');

        const card = workflowQuery<HTMLElement>(canvasElement, 'cem-card section');
        const email = workflowQuery<HTMLInputElement>(canvasElement, 'cem-switch input[name="email-notifications"]');
        const radios = Array.from(canvasElement.querySelectorAll<HTMLInputElement>('cem-radio input[name="frequency"]'));
        const alert = workflowQuery<HTMLElement>(canvasElement, 'cem-alert [role="alert"]');
        const toast = workflowQuery<HTMLElement>(canvasElement, 'cem-toast [role="status"]');
        const progress = workflowQuery<HTMLProgressElement>(canvasElement, 'cem-progress progress');
        const skeleton = workflowQuery<HTMLElement>(canvasElement, 'cem-skeleton .cem-skeleton');
        const dialog = workflowQuery<HTMLElement>(canvasElement, 'cem-dialog dialog');
        const sheet = workflowQuery<HTMLElement>(canvasElement, 'cem-sheet aside');
        const archived = workflowQuery<HTMLInputElement>(canvasElement, 'cem-sheet cem-checkbox input[name="include-archived"]');

        expect(assertAccessibleName(card, 'Notification settings')).toBe('Notification settings');
        expect(email.getAttribute('role')).toBe('switch');
        expect(assertAccessibleName(email, 'Email notifications')).toBe('Email notifications');
        expect(radios).toHaveLength(2);
        expect(radios.map((radio) => assertAccessibleName(radio)).join('|')).toBe('Daily summary|Weekly summary');
        expect(alert.textContent?.trim()).toBe('Review required notification preferences.');
        expect(toast.textContent?.trim()).toBe('Settings saved');
        expect(assertAccessibleName(progress, 'Sync progress')).toBe('Sync progress');
        expect(progress.getAttribute('value')).toBe('40');
        expect(skeleton.getAttribute('aria-hidden')).toBe('true');
        workflowQuery<HTMLButtonElement>(canvasElement, 'cem-dialog [part=trigger]').click();
        await nextRenderFrame();
        expect(assertAccessibleName(dialog, 'Confirm changes')).toBe('Confirm changes');
        expect(assertAccessibleName(sheet, 'Advanced filters')).toBe('Advanced filters');
        expect(assertAccessibleName(archived, 'Include archived assets')).toBe('Include archived assets');
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
