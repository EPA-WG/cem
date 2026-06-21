import { CemElementRuntime } from '@epa-wg/cem-elements';

import assetBrowserFixture from '../../tests/workflows/asset-browser.html?raw';
import authFormFixture from '../../tests/workflows/auth-form.html?raw';
import discussionThreadFixture from '../../tests/workflows/discussion-thread.html?raw';
import profileEditorFixture from '../../tests/workflows/profile-editor.html?raw';
import settingsFixture from '../../tests/workflows/settings.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    assertLightDomRendered,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

const workflowFixtures = [
    ['auth form', authFormFixture],
    ['profile editor', profileEditorFixture],
    ['asset browser', assetBrowserFixture],
    ['discussion thread', discussionThreadFixture],
    ['settings', settingsFixture],
] as const;

describe('CEM component workflow fixtures', () => {
    let harness: ComponentHarness;

    beforeAll(() => {
        const runtime = new CemElementRuntime({ declarationTag: 'cem-components-workflow-declaration' });
        const result = installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it.each(workflowFixtures)('%s fixture is declarative markup only', (_, fixture) => {
        expect(fixture).not.toMatch(/<script\b/i);
        expect(fixture).not.toMatch(/\son[a-z]+\s*=/i);
        expect(fixture).toMatch(/<cem-[a-z-]+/);
    });

    it('renders the auth form workflow without app JavaScript', async () => {
        harness = createComponentHarness();
        await renderWorkflow(harness, authFormFixture, 'cem-action button');

        const card = harness.query<HTMLElement>('cem-card section');
        const form = harness.query<HTMLFormElement>('form');
        const email = harness.query<HTMLInputElement>('cem-text-field input[name="email"]');
        const password = harness.query<HTMLInputElement>('cem-text-field input[name="password"]');
        const remember = harness.query<HTMLInputElement>('cem-checkbox input[name="remember"]');
        const submit = harness.query<HTMLButtonElement>('cem-action button');

        expect(assertAccessibleName(card, 'Sign in')).toBe('Sign in');
        expect(form).toBeInstanceOf(HTMLFormElement);
        expect(email.getAttribute('placeholder')).toBe('name@example.com');
        expect(assertAccessibleName(email, 'Email')).toBe('Email');
        expect(password.type).toBe('password');
        expect(assertAccessibleName(password, 'Password')).toBe('Password');
        expect(remember.type).toBe('checkbox');
        expect(remember.getAttribute('value')).toBe('yes');
        expect(assertAccessibleName(remember, 'Remember this device')).toBe('Remember this device');
        expect(submit.type).toBe('button');
        expect(assertAccessibleName(submit, 'Continue')).toBe('Continue');
        assertWorkflowIntegrity(harness.root);
    });

    it('renders the profile editor workflow without app JavaScript', async () => {
        harness = createComponentHarness();
        await renderWorkflow(harness, profileEditorFixture, 'cem-action button');
        await waitForWorkflowText(harness.root, 'cem-switch .cem-switch__label', 'Public profile');

        const avatar = harness.query<HTMLElement>('cem-avatar [role="img"]');
        const displayName = harness.query<HTMLInputElement>('cem-text-field input[name="display-name"]');
        const bio = harness.query<HTMLTextAreaElement>('cem-textarea textarea[name="bio"]');
        const publicProfile = harness.query<HTMLInputElement>('cem-switch input[name="public-profile"]');
        const alert = harness.query<HTMLElement>('cem-alert [role="status"]');
        const save = harness.query<HTMLButtonElement>('cem-action button');

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
        assertWorkflowIntegrity(harness.root);
    });

    it('renders the asset browser workflow without app JavaScript', async () => {
        harness = createComponentHarness();
        await renderWorkflow(harness, assetBrowserFixture, 'cem-media-preview figure');

        const appBar = harness.query<HTMLElement>('cem-app-bar header');
        const settings = harness.query<HTMLButtonElement>('cem-icon-button button');
        const tablist = harness.query<HTMLElement>('cem-tabs [role="tablist"]');
        const table = harness.query<HTMLElement>('cem-table [role="table"]');
        const badge = harness.query<HTMLElement>('cem-badge .cem-badge');
        const media = harness.query<HTMLElement>('cem-media-preview figure');
        const image = harness.query<HTMLImageElement>('cem-media-preview img');

        expect(assertAccessibleName(appBar, 'Asset browser')).toBe('Asset browser');
        expect(assertAccessibleName(settings, 'Open settings')).toBe('Open settings');
        expect(assertAccessibleName(tablist, 'Asset views')).toBe('Asset views');
        expect(table.querySelectorAll('[role="row"]')).toHaveLength(2);
        expect(assertAccessibleName(table, 'Asset table')).toBe('Asset table');
        expect(badge.getAttribute('data-tone')).toBe('success');
        expect(badge.textContent?.trim()).toBe('Ready');
        expect(assertAccessibleName(media, 'Policy preview')).toBe('Policy preview');
        expect(image.getAttribute('alt')).toBe('Policy thumbnail');
        assertWorkflowIntegrity(harness.root);
    });

    it('renders the discussion thread workflow without app JavaScript', async () => {
        harness = createComponentHarness();
        await renderWorkflow(harness, discussionThreadFixture, 'cem-action button');
        await waitForWorkflowText(harness.root, 'cem-list', 'The verification gate is green.');

        const thread = harness.query<HTMLElement>('cem-card section');
        const messages = harness.query<HTMLUListElement>('cem-list ul');
        const reply = harness.query<HTMLTextAreaElement>('cem-textarea textarea[name="reply"]');
        const toast = harness.query<HTMLElement>('cem-toast [role="status"]');
        const alert = harness.query<HTMLElement>('cem-alert [role="alert"]');
        const post = harness.query<HTMLButtonElement>('cem-action button');

        expect(assertAccessibleName(thread, 'Discussion thread')).toBe('Discussion thread');
        expect(assertAccessibleName(messages, 'Messages')).toBe('Messages');
        expect(messages.querySelectorAll('li')).toHaveLength(2);
        expect(messages.textContent).toContain('The verification gate is green.');
        expect(assertAccessibleName(reply, 'Reply')).toBe('Reply');
        expect(toast.textContent?.trim()).toBe('Draft saved');
        expect(alert.textContent?.trim()).toBe('Resolve mentions before posting.');
        expect(assertAccessibleName(post, 'Post reply')).toBe('Post reply');
        assertWorkflowIntegrity(harness.root);
    });

    it('renders the settings workflow without app JavaScript', async () => {
        harness = createComponentHarness();
        await renderWorkflow(harness, settingsFixture, 'cem-sheet aside');

        const card = harness.query<HTMLElement>('cem-card section');
        const email = harness.query<HTMLInputElement>('cem-switch input[name="email-notifications"]');
        const radios = Array.from(harness.root.querySelectorAll<HTMLInputElement>('cem-radio input[name="frequency"]'));
        const alert = harness.query<HTMLElement>('cem-alert [role="alert"]');
        const toast = harness.query<HTMLElement>('cem-toast [role="status"]');
        const progress = harness.query<HTMLProgressElement>('cem-progress progress');
        const skeleton = harness.query<HTMLElement>('cem-skeleton .cem-skeleton');
        const dialog = harness.query<HTMLElement>('cem-dialog [role="dialog"]');
        const sheet = harness.query<HTMLElement>('cem-sheet aside');
        const archived = harness.query<HTMLInputElement>('cem-sheet cem-checkbox input[name="include-archived"]');

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
        expect(assertAccessibleName(dialog, 'Confirm changes')).toBe('Confirm changes');
        expect(assertAccessibleName(sheet, 'Advanced filters')).toBe('Advanced filters');
        expect(assertAccessibleName(archived, 'Include archived assets')).toBe('Include archived assets');
        assertWorkflowIntegrity(harness.root);
    });
});

async function renderWorkflow(harness: ComponentHarness, markup: string, readySelector: string): Promise<void> {
    await harness.render(`<section aria-label="workflow fixture">${markup}</section>`);
    await waitForWorkflowSelector(harness.root, readySelector);
}

async function waitForWorkflowSelector(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + 1000;
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
    const deadline = Date.now() + 1000;
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
