import { CemElementRuntime } from '@epa-wg/cem-elements';

import {
    CEM_COMPONENT_PRIMITIVES,
    installCemComponentPrimitives,
} from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    assertLightDomRendered,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

describe('CEM component primitives', () => {
    let harness: ComponentHarness;

    beforeAll(() => {
        const runtime = new CemElementRuntime({ declarationTag: 'cem-components-primitive-declaration' });
        const result = installCemComponentPrimitives(runtime);

        expect(result.diagnostics).toEqual([]);
        expect([...result.registered, ...result.skipped].sort()).toEqual(
            CEM_COMPONENT_PRIMITIVES.map((primitive) => primitive.tag).sort(),
        );
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('renders action-family primitives as accessible light DOM', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-action variant="primary">Save</cem-action>
                <cem-icon-button name="settings" label="Open settings"></cem-icon-button>
                <cem-menu-item>Archive</cem-menu-item>
            </cem-stack>
        `);
        await waitForPrimitive(root, 'cem-menu-item button');

        const action = harness.query<HTMLButtonElement>('cem-action button');
        const iconButton = harness.query<HTMLButtonElement>('cem-icon-button button');
        const menuItem = harness.query<HTMLButtonElement>('cem-menu-item button');
        const icon = harness.query<HTMLElement>('cem-icon-button .cem-icon');

        for (const host of Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-action, cem-icon-button, cem-menu-item'),
        )) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
        }

        expect(action.type).toBe('button');
        expect(action.className).toContain('cem-action--primary');
        expect(assertAccessibleName(action, 'Save')).toBe('Save');
        expect(iconButton.type).toBe('button');
        expect(iconButton.className).toContain('cem-icon-button--quiet');
        expect(assertAccessibleName(iconButton, 'Open settings')).toBe('Open settings');
        expect(icon.getAttribute('aria-hidden')).toBe('true');
        expect(icon.textContent?.trim()).toBe('settings');
        expect(menuItem.type).toBe('button');
        expect(menuItem.getAttribute('role')).toBe('menuitem');
        expect(assertAccessibleName(menuItem, 'Archive')).toBe('Archive');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('renders field, text, and icon primitives as accessible light DOM', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-field name="email" value="a@b.test">
                    <span slot="label">Email</span>
                    <span slot="help">Use a work address.</span>
                </cem-field>
                <cem-text variant="caption">Ready</cem-text>
                <cem-icon name="check" label="Complete"></cem-icon>
            </cem-stack>
        `);
        await waitForPrimitive(root, 'cem-icon span');

        const stack = harness.query<HTMLElement>('cem-stack');
        const input = harness.query<HTMLInputElement>('cem-field input');
        const text = harness.query<HTMLElement>('cem-text .cem-text');
        const icon = harness.query<HTMLElement>('cem-icon .cem-icon');

        for (const host of Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-stack, cem-field, cem-text, cem-icon'),
        )) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
        }

        expect(stack.querySelector('.cem-stack')?.getAttribute('data-gap')).toBe('sm');
        expect(input.getAttribute('name')).toBe('email');
        expect(input.getAttribute('value')).toBe('a@b.test');
        expect(assertAccessibleName(input, 'Email')).toBe('Email');
        expect(text.textContent?.trim()).toBe('Ready');
        expect(icon.getAttribute('role')).toBe('img');
        expect(assertAccessibleName(icon, 'Complete')).toBe('Complete');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('renders input-family MVP primitives as native accessible controls', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-text-field name="email" value="a@b.test" placeholder="name@example.com">
                    <span slot="label">Email</span>
                    <span slot="help">Use a work address.</span>
                </cem-text-field>
                <cem-textarea name="bio" value="Short bio">
                    <span slot="label">Bio</span>
                </cem-textarea>
                <cem-select name="role">
                    <span slot="label">Role</span>
                    <option value="admin">Admin</option>
                    <option value="viewer">Viewer</option>
                </cem-select>
                <cem-checkbox name="terms" value="accepted">Accept terms</cem-checkbox>
                <cem-radio name="plan" value="pro">Pro plan</cem-radio>
                <cem-switch name="notifications">Notifications</cem-switch>
            </cem-stack>
        `);
        await waitForPrimitive(root, 'cem-switch input');

        const textField = harness.query<HTMLInputElement>('cem-text-field input');
        const textarea = harness.query<HTMLTextAreaElement>('cem-textarea textarea');
        const select = harness.query<HTMLSelectElement>('cem-select select');
        const checkbox = harness.query<HTMLInputElement>('cem-checkbox input');
        const radio = harness.query<HTMLInputElement>('cem-radio input');
        const switchInput = harness.query<HTMLInputElement>('cem-switch input');

        for (const host of Array.from(
            harness.root.querySelectorAll<HTMLElement>(
                'cem-text-field, cem-textarea, cem-select, cem-checkbox, cem-radio, cem-switch',
            ),
        )) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
        }

        expect(textField.type).toBe('text');
        expect(textField.getAttribute('name')).toBe('email');
        expect(textField.getAttribute('value')).toBe('a@b.test');
        expect(textField.getAttribute('placeholder')).toBe('name@example.com');
        expect(assertAccessibleName(textField, 'Email')).toBe('Email');
        expect(textarea.getAttribute('name')).toBe('bio');
        expect(textarea.value).toBe('Short bio');
        expect(assertAccessibleName(textarea, 'Bio')).toBe('Bio');
        expect(select.getAttribute('name')).toBe('role');
        expect(select.querySelectorAll('option')).toHaveLength(2);
        expect(assertAccessibleName(select, 'Role')).toBe('Role');
        expect(checkbox.type).toBe('checkbox');
        expect(checkbox.getAttribute('name')).toBe('terms');
        expect(checkbox.getAttribute('value')).toBe('accepted');
        expect(assertAccessibleName(checkbox, 'Accept terms')).toBe('Accept terms');
        expect(radio.type).toBe('radio');
        expect(radio.getAttribute('name')).toBe('plan');
        expect(radio.getAttribute('value')).toBe('pro');
        expect(assertAccessibleName(radio, 'Pro plan')).toBe('Pro plan');
        expect(switchInput.type).toBe('checkbox');
        expect(switchInput.getAttribute('role')).toBe('switch');
        expect(switchInput.getAttribute('name')).toBe('notifications');
        expect(assertAccessibleName(switchInput, 'Notifications')).toBe('Notifications');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('renders navigation-family MVP primitives as accessible landmarks and tablists', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-app-bar label="CEM Admin">
                    <span slot="title">CEM Admin</span>
                    <button type="button">Settings</button>
                </cem-app-bar>
                <cem-nav label="Primary">
                    <a href="#overview">Overview</a>
                    <a href="#assets">Assets</a>
                </cem-nav>
                <cem-tabs label="Profile sections">
                    <button type="button" role="tab" aria-selected="true">Overview</button>
                    <button type="button" role="tab" aria-selected="false">Security</button>
                </cem-tabs>
            </cem-stack>
        `);
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
    });

    it('renders content-family MVP primitives as accessible light DOM', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-card label="Profile summary">
                    <span slot="title">Profile summary</span>
                    <p>Updated today</p>
                </cem-card>
                <cem-list label="Assets">
                    <li>Policy.pdf</li>
                    <li>Invoice.csv</li>
                </cem-list>
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
                <cem-chip label="Filtered by owner">Owner</cem-chip>
                <cem-badge tone="success">Ready</cem-badge>
                <cem-avatar label="Ada Lovelace" initials="AL"></cem-avatar>
                <cem-media-preview label="Policy preview">
                    <img src="/policy.png" alt="Policy thumbnail" />
                    <span slot="caption">Policy preview</span>
                </cem-media-preview>
            </cem-stack>
        `);
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
    });

    it('renders layout, list, navigation, surface, and dialog shell primitives', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-surface label="Account">
                <cem-grid columns="2" gap="lg">
                    <cem-list label="Tasks">
                        <li>Review</li>
                        <li>Approve</li>
                    </cem-list>
                    <cem-nav label="Sections">
                        <a href="#profile">Profile</a>
                    </cem-nav>
                </cem-grid>
                <cem-dialog-shell label="Confirm">
                    <p>Submit the change?</p>
                </cem-dialog-shell>
            </cem-surface>
        `);
        await waitForPrimitive(root, 'cem-dialog-shell [role="dialog"]');

        const surface = harness.query<HTMLElement>('cem-surface section');
        const grid = harness.query<HTMLElement>('cem-grid .cem-grid');
        const list = harness.query<HTMLUListElement>('cem-list ul');
        const nav = harness.query<HTMLElement>('cem-nav nav');
        const dialog = harness.query<HTMLElement>('cem-dialog-shell [role="dialog"]');

        expect(surface.getAttribute('aria-label')).toBe('Account');
        expect(grid.getAttribute('data-columns')).toBe('2');
        expect(grid.getAttribute('data-gap')).toBe('lg');
        expect(assertAccessibleName(list, 'Tasks')).toBe('Tasks');
        expect(list.querySelectorAll('li')).toHaveLength(2);
        expect(assertAccessibleName(nav, 'Sections')).toBe('Sections');
        expect(nav.querySelector('a')?.textContent).toBe('Profile');
        expect(dialog.getAttribute('aria-modal')).toBe('true');
        expect(assertAccessibleName(dialog, 'Confirm')).toBe('Confirm');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });
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
