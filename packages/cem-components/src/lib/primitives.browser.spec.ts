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
