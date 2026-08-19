import { CemElementRuntime } from '@epa-wg/cem-elements';

import assetBrowserFixture from '../../tests/workflows/asset-browser.html?raw';
import authFormFixture from '../../tests/workflows/auth-form.html?raw';
import discussionThreadFixture from '../../tests/workflows/discussion-thread.html?raw';
import passwordResetFixture from '../../tests/workflows/password-reset.html?raw';
import profileEditorFixture from '../../tests/workflows/profile-editor.html?raw';
import registrationFixture from '../../tests/workflows/registration.html?raw';
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

const WORKFLOW_RENDER_TIMEOUT_MS = 3000;

const workflowFixtures = [
    ['auth form', authFormFixture],
    ['registration', registrationFixture],
    ['password reset', passwordResetFixture],
    ['profile editor', profileEditorFixture],
    ['asset browser', assetBrowserFixture],
    ['discussion thread', discussionThreadFixture],
    ['settings', settingsFixture],
] as const;

describe('CEM component workflow fixtures', () => {
    let harness: ComponentHarness;

    beforeAll(async () => {
        const runtime = new CemElementRuntime({ declarationTag: 'cem-components-workflow-declaration' });
        const result = await installCemComponentPrimitives(runtime);
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

    it('renders the registration auth workflow with required, invalid, and loading states', async () => {
        harness = createComponentHarness();
        await renderWorkflow(harness, registrationFixture, 'cem-progress progress');

        const card = harness.query<HTMLElement>('cem-card section');
        const fullName = harness.query<HTMLInputElement>('cem-text-field input[name="full-name"]');
        const email = harness.query<HTMLInputElement>('cem-text-field input[name="email"]');
        const password = harness.query<HTMLInputElement>('cem-text-field input[name="password"]');
        const terms = harness.query<HTMLInputElement>('cem-checkbox input[name="terms"]');
        const alert = harness.query<HTMLElement>('cem-alert [role="alert"]');
        const progress = harness.query<HTMLProgressElement>('cem-progress progress');
        const submit = harness.query<HTMLButtonElement>('cem-action button');

        expect(assertAccessibleName(card, 'Create account')).toBe('Create account');
        expect(fullName.required).toBe(true);
        expect(assertAccessibleName(fullName, 'Full name')).toBe('Full name');
        expect(email.required).toBe(true);
        expect(email.getAttribute('aria-invalid')).toBe('true');
        expect(email.getAttribute('aria-describedby')).toBe('registration-email-help');
        expect(email.getAttribute('aria-errormessage')).toBe('registration-email-error');
        expect(assertAccessibleName(email, 'Email')).toBe('Email');
        expect(password.type).toBe('password');
        expect(password.required).toBe(true);
        expect(password.readOnly).toBe(true);
        expect(assertAccessibleName(password, 'Password')).toBe('Password');
        expect(terms.required).toBe(true);
        expect(terms.getAttribute('aria-invalid')).toBe('true');
        expect(assertAccessibleName(terms, 'Accept terms')).toBe('Accept terms');
        expect(alert.textContent?.trim()).toBe('Complete required fields before continuing.');
        expect(assertAccessibleName(progress, 'Registration progress')).toBe('Registration progress');
        expect(progress.getAttribute('value')).toBe('65');
        expect(submit.disabled).toBe(true);
        expect(submit.getAttribute('aria-busy')).toBe('true');
        expect(assertAccessibleName(submit, 'Create account')).toBe('Create account');
        assertWorkflowIntegrity(harness.root);
    });

    it('renders the password reset workflow with help, error, and loading feedback', async () => {
        harness = createComponentHarness();
        await renderWorkflow(harness, passwordResetFixture, 'cem-progress progress');

        const card = harness.query<HTMLElement>('cem-card section');
        const email = harness.query<HTMLInputElement>('cem-text-field input[name="email"]');
        const alert = harness.query<HTMLElement>('cem-alert [role="status"]');
        const progress = harness.query<HTMLProgressElement>('cem-progress progress');
        const submit = harness.query<HTMLButtonElement>('cem-action button');

        expect(assertAccessibleName(card, 'Reset password')).toBe('Reset password');
        expect(email.required).toBe(true);
        expect(email.getAttribute('placeholder')).toBe('name@example.com');
        expect(email.getAttribute('aria-invalid')).toBe('true');
        expect(email.getAttribute('aria-describedby')).toBe('password-reset-email-help');
        expect(email.getAttribute('aria-errormessage')).toBe('password-reset-email-error');
        expect(assertAccessibleName(email, 'Email')).toBe('Email');
        expect(alert.textContent?.trim()).toBe('We will send a secure reset link.');
        expect(assertAccessibleName(progress, 'Reset request progress')).toBe('Reset request progress');
        expect(progress.getAttribute('value')).toBe('25');
        expect(submit.disabled).toBe(true);
        expect(submit.getAttribute('aria-busy')).toBe('true');
        expect(assertAccessibleName(submit, 'Send reset link')).toBe('Send reset link');
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
