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

    it('renders action, field, text, and icon primitives as accessible light DOM', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-action variant="primary">Save</cem-action>
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
        const button = harness.query<HTMLButtonElement>('cem-action button');
        const input = harness.query<HTMLInputElement>('cem-field input');
        const text = harness.query<HTMLElement>('cem-text .cem-text');
        const icon = harness.query<HTMLElement>('cem-icon .cem-icon');

        for (const host of Array.from(harness.root.querySelectorAll<HTMLElement>('cem-stack, cem-action, cem-field, cem-text, cem-icon'))) {
            assertLightDomRendered(host);
            expect(host.shadowRoot).toBeNull();
        }

        expect(stack.querySelector('.cem-stack')?.getAttribute('data-gap')).toBe('sm');
        expect(button.type).toBe('button');
        expect(assertAccessibleName(button, 'Save')).toBe('Save');
        expect(input.getAttribute('name')).toBe('email');
        expect(input.getAttribute('value')).toBe('a@b.test');
        expect(assertAccessibleName(input, 'Email')).toBe('Email');
        expect(text.textContent?.trim()).toBe('Ready');
        expect(icon.getAttribute('role')).toBe('img');
        expect(assertAccessibleName(icon, 'Complete')).toBe('Complete');
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
