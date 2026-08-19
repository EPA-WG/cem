import { CemElementRuntime } from '@epa-wg/cem-elements';

import {
    CEM_COMPONENT_PRIMITIVES,
    type CemComponentPrimitiveInstallResult,
    installCemComponentPrimitives,
} from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    assertLightDomRendered,
    captureVisualSnapshot,
    createComponentHarness,
    createSubstrateComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
    type SubstrateComponentHarness,
    type VisualSnapshot,
} from './testing/component-harness.js';

const PHASE3_MINIMAL_PRIMITIVE_TAGS = [
    'cem-action',
    'cem-field',
    'cem-surface',
    'cem-text',
    'cem-icon',
    'cem-stack',
    'cem-grid',
    'cem-list',
    'cem-nav',
    'cem-dialog-shell',
] as const;

describe('CEM component primitives', () => {
    let harness: ComponentHarness;
    let installResult: CemComponentPrimitiveInstallResult;
    let runtime: CemElementRuntime;

    beforeAll(async () => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-primitive-declaration' });
        installResult = await installCemComponentPrimitives(runtime);

        expect(installResult.diagnostics).toEqual([]);
        expect([...installResult.registered, ...installResult.skipped].sort()).toEqual(
            CEM_COMPONENT_PRIMITIVES.map((primitive) => primitive.tag).sort(),
        );
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('reports deterministic primitive install results', async () => {
        const primitiveTags = CEM_COMPONENT_PRIMITIVES.map((primitive) => primitive.tag);
        expect([...installResult.registered, ...installResult.skipped]).toEqual(primitiveTags);

        const runtime = new CemElementRuntime({ declarationTag: 'cem-components-primitive-reinstall-declaration' });
        const reinstallResult = await installCemComponentPrimitives(runtime);
        expect(reinstallResult.registered).toEqual([]);
        expect(reinstallResult.skipped).toEqual(primitiveTags);
        expect(reinstallResult.diagnostics).toEqual([]);
    });

    it('registers and renders the exact minimal Phase 3 set through the accepted substrate', async () => {
        expect(PHASE3_MINIMAL_PRIMITIVE_TAGS.every((tag) => Boolean(customElements.get(tag)))).toBe(true);

        const substrateHarness: SubstrateComponentHarness = createSubstrateComponentHarness(runtime);
        harness = substrateHarness;
        await substrateHarness.render(`
            <div data-phase3-minimal-primitives>
                <cem-action>Save</cem-action>
                <cem-field name="email" label="Email" value="ada@example.test"></cem-field>
                <cem-surface label="Workspace"><span>Surface content</span></cem-surface>
                <cem-text>Ready</cem-text>
                <cem-icon name="check" label="Complete"></cem-icon>
                <cem-stack gap="sm"><span>Stack content</span></cem-stack>
                <cem-grid columns="2"><span>Grid content</span></cem-grid>
                <cem-list label="Tasks"><li>Review</li></cem-list>
                <cem-nav label="Sections"><a href="#profile">Profile</a></cem-nav>
                <cem-dialog-shell label="Confirm"><p>Submit?</p></cem-dialog-shell>
            </div>
        `);
        const hosts = PHASE3_MINIMAL_PRIMITIVE_TAGS.map((tag) => substrateHarness.query<HTMLElement>(tag));
        await Promise.all(hosts.map((host) => substrateHarness.settle(host)));

        for (const host of hosts) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
            expect(host.querySelector(':scope > template[data-cem-island]')).not.toBeNull();
            expect(substrateHarness.snapshot(host).outputTarget).toBe('light-dom');
        }

        expect(assertAccessibleName(substrateHarness.query('cem-action button'), 'Save')).toBe('Save');
        expect(assertAccessibleName(substrateHarness.query('cem-field input'), 'Email')).toBe('Email');
        expect(assertAccessibleName(substrateHarness.query('cem-surface section'), 'Workspace')).toBe('Workspace');
        expect(assertAccessibleName(substrateHarness.query('cem-icon [role="img"]'), 'Complete')).toBe('Complete');
        expect(assertAccessibleName(substrateHarness.query('cem-list ul'), 'Tasks')).toBe('Tasks');
        expect(assertAccessibleName(substrateHarness.query('cem-nav nav'), 'Sections')).toBe('Sections');
        expect(assertAccessibleName(substrateHarness.query('cem-dialog-shell [role="dialog"]'), 'Confirm')).toBe('Confirm');
        expect(() => assertAriaReferenceIntegrity(substrateHarness.root)).not.toThrow();
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
        const selectHost = harness.query<HTMLElement & { value: string; selectedValues: string[] }>('cem-select');
        const select = harness.query<HTMLButtonElement>('cem-select .cem-select__control');
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
        expect(selectHost.getAttribute('name')).toBe('role');
        expect(select.getAttribute('role')).toBe('combobox');
        expect(selectHost.value).toBe('admin');
        expect(selectHost.selectedValues).toEqual(['admin']);
        expect(harness.root.querySelectorAll('cem-select [role="option"]')).toHaveLength(0);
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

    it('renders feedback-family MVP primitives with status and dialog semantics', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-dialog label="Confirm delete">
                    <p>Delete this asset?</p>
                </cem-dialog>
                <cem-sheet label="Filters">
                    <p>Filter options</p>
                </cem-sheet>
                <cem-toast>Saved</cem-toast>
                <cem-progress label="Upload progress" value="40" max="100"></cem-progress>
                <cem-skeleton label="Loading card"></cem-skeleton>
                <cem-alert tone="warning" role="alert">Check required fields.</cem-alert>
            </cem-stack>
        `);
        await waitForPrimitive(root, 'cem-alert [role="alert"]');

        const dialog = harness.query<HTMLElement>('cem-dialog [role="dialog"]');
        const sheet = harness.query<HTMLElement>('cem-sheet aside');
        const toast = harness.query<HTMLElement>('cem-toast [role="status"]');
        const progress = harness.query<HTMLProgressElement>('cem-progress progress');
        const skeleton = harness.query<HTMLElement>('cem-skeleton .cem-skeleton');
        const alert = harness.query<HTMLElement>('cem-alert [role="alert"]');

        for (const host of Array.from(
            harness.root.querySelectorAll<HTMLElement>(
                'cem-dialog, cem-sheet, cem-toast, cem-progress, cem-skeleton, cem-alert',
            ),
        )) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
        }

        expect(dialog.getAttribute('aria-modal')).toBe('true');
        expect(assertAccessibleName(dialog, 'Confirm delete')).toBe('Confirm delete');
        expect(dialog.textContent).toContain('Delete this asset?');
        expect(sheet.getAttribute('role')).toBe('region');
        expect(assertAccessibleName(sheet, 'Filters')).toBe('Filters');
        expect(toast.getAttribute('aria-live')).toBe('polite');
        expect(toast.textContent?.trim()).toBe('Saved');
        expect(assertAccessibleName(progress, 'Upload progress')).toBe('Upload progress');
        expect(progress.getAttribute('value')).toBe('40');
        expect(progress.getAttribute('max')).toBe('100');
        expect(skeleton.getAttribute('aria-hidden')).toBe('true');
        expect(skeleton.textContent?.trim()).toBe('Loading card');
        expect(alert.getAttribute('data-tone')).toBe('warning');
        expect(alert.textContent?.trim()).toBe('Check required fields.');
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
        await waitForPrimitive(root, 'cem-list li:not(.cem-list__empty)');

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

    it('captures primitive-family visual snapshots from rendered light DOM', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-action variant="primary">Save</cem-action>
                <cem-text-field name="email" value="a@b.test">
                    <span slot="label">Email</span>
                </cem-text-field>
                <cem-surface label="Dashboard">
                    <cem-card label="Summary">
                        <span slot="title">Summary</span>
                        <cem-badge tone="success">Ready</cem-badge>
                    </cem-card>
                </cem-surface>
                <cem-nav label="Primary">
                    <a href="#home">Home</a>
                    <a href="#settings">Settings</a>
                </cem-nav>
                <cem-alert tone="warning" role="alert">Check required fields.</cem-alert>
            </cem-stack>
        `);
        await waitForPrimitive(root, 'cem-alert [role="alert"]');
        await waitForPrimitive(root, 'cem-card section');
        await waitForPrimitive(root, 'cem-badge .cem-badge');

        assertPrimitiveVisualSnapshot(captureVisualSnapshot(harness.query<HTMLElement>('cem-action button')), {
            family: 'action controls',
            htmlIncludes: ['class="cem-action cem-action--primary"', 'type="button"'],
            tagName: 'button',
            text: 'Save',
        });
        assertPrimitiveVisualSnapshot(captureVisualSnapshot(harness.query<HTMLElement>('cem-text-field input')), {
            family: 'input controls',
            htmlIncludes: ['class="cem-text-field__control"', 'name="email"', 'value="a@b.test"'],
            tagName: 'input',
            text: '',
        });
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
