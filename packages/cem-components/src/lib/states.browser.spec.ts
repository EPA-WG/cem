import { CemElementRuntime, type DataIslandSnapshot } from '@epa-wg/cem-elements';

import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    assertFocusVisible,
    assertLightDomRendered,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

interface SerializedEventTarget {
    checked: boolean | null;
    name: string | null;
    tag: string;
    type: string | null;
    value: string | null;
}

interface SerializedEventPayload {
    bubbles: boolean;
    currentTarget: SerializedEventTarget | null;
    sliceValue: unknown;
    target: SerializedEventTarget | null;
    type: string;
}

describe('CEM component primitive states and ARIA behavior', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(() => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-state-declaration' });
        const result = installCemComponentPrimitives(runtime);

        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('reflects action, loading, disabled, expanded, selected, and focus states on native controls', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-action loading="true" expanded="false">Sync</cem-action>
                <cem-icon-button name="settings" label="Open settings" disabled></cem-icon-button>
                <cem-menu-item expanded="true">Advanced options</cem-menu-item>
                <cem-tabs label="Sections">
                    <button type="button" role="tab" aria-selected="true">Current</button>
                    <button type="button" role="tab" aria-selected="false">Later</button>
                </cem-tabs>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-tabs [role="tablist"]');

        const actionHost = harness.query<HTMLElement>('cem-action');
        const action = harness.query<HTMLButtonElement>('cem-action button');
        const iconButton = harness.query<HTMLButtonElement>('cem-icon-button button');
        const menuItem = harness.query<HTMLButtonElement>('cem-menu-item button');
        const tabs = Array.from(harness.root.querySelectorAll<HTMLButtonElement>('cem-tabs [role="tab"]'));

        assertStateHostsRendered(harness.root, 'cem-action, cem-icon-button, cem-menu-item, cem-tabs');
        expect(action.getAttribute('aria-busy')).toBe('true');
        expect(action.getAttribute('aria-expanded')).toBe('false');
        expect(assertAccessibleName(action, 'Sync')).toBe('Sync');
        expect(iconButton.disabled).toBe(true);
        expect(assertAccessibleName(iconButton, 'Open settings')).toBe('Open settings');
        expect(menuItem.getAttribute('aria-expanded')).toBe('true');
        expect(assertAccessibleName(menuItem, 'Advanced options')).toBe('Advanced options');
        expect(tabs.map((tab) => tab.getAttribute('aria-selected')).join('|')).toBe('true|false');
        await assertFocusVisible(action);

        action.click();
        await nextRenderFrame();

        const payload = eventPayload(runtime.snapshotInstance(actionHost), 'pressed');
        expect(payload.type).toBe('click');
        expect(payload.sliceValue).toBe('click');
        expect(payload.target?.tag).toBe('button');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('reflects form disabled, invalid, required, readonly, checked, and indeterminate states', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <section>
                <p id="email-help">Use a work address.</p>
                <p id="email-error">Email is required.</p>
                <p id="role-help">Choose the closest role.</p>
                <p id="role-error">Role is required.</p>
                <cem-stack gap="sm">
                    <cem-text-field
                        name="email"
                        value="a@b.test"
                        label="Email"
                        required
                        readonly
                        invalid="true"
                        describedby="email-help"
                        error="email-error"
                    ></cem-text-field>
                    <cem-textarea name="notes" label="Notes" disabled invalid="true"></cem-textarea>
                    <cem-select
                        name="role"
                        label="Role"
                        required
                        invalid="true"
                        describedby="role-help"
                        error="role-error"
                    >
                        <option value="admin">Admin</option>
                        <option value="viewer">Viewer</option>
                    </cem-select>
                    <cem-checkbox name="terms" checked required invalid="true">Accept terms</cem-checkbox>
                    <cem-checkbox name="partial" indeterminate="mixed">Partially selected</cem-checkbox>
                    <cem-radio name="plan" value="pro" checked disabled>Pro plan</cem-radio>
                    <cem-switch name="public-profile" checked>Public profile</cem-switch>
                </cem-stack>
            </section>
        `);
        await waitForStateSelector(root, 'cem-switch input');

        const textField = harness.query<HTMLInputElement>('cem-text-field input');
        const textarea = harness.query<HTMLTextAreaElement>('cem-textarea textarea');
        const select = harness.query<HTMLSelectElement>('cem-select select');
        const checkedBox = harness.query<HTMLInputElement>('cem-checkbox input[name="terms"]');
        const mixedBox = harness.query<HTMLInputElement>('cem-checkbox input[name="partial"]');
        const radio = harness.query<HTMLInputElement>('cem-radio input');
        const switchInput = harness.query<HTMLInputElement>('cem-switch input');

        assertStateHostsRendered(
            harness.root,
            'cem-text-field, cem-textarea, cem-select, cem-checkbox, cem-radio, cem-switch',
        );
        expect(textField.required).toBe(true);
        expect(textField.readOnly).toBe(true);
        expect(textField.getAttribute('aria-invalid')).toBe('true');
        expect(textField.getAttribute('aria-describedby')).toBe('email-help');
        expect(textField.getAttribute('aria-errormessage')).toBe('email-error');
        expect(assertAccessibleName(textField, 'Email')).toBe('Email');
        expect(textarea.disabled).toBe(true);
        expect(textarea.getAttribute('aria-invalid')).toBe('true');
        expect(select.required).toBe(true);
        expect(select.getAttribute('aria-describedby')).toBe('role-help');
        expect(select.getAttribute('aria-errormessage')).toBe('role-error');
        expect(checkedBox.checked).toBe(true);
        expect(checkedBox.required).toBe(true);
        expect(checkedBox.getAttribute('aria-invalid')).toBe('true');
        expect(mixedBox.getAttribute('aria-checked')).toBe('mixed');
        expect(radio.checked).toBe(true);
        expect(radio.disabled).toBe(true);
        expect(switchInput.checked).toBe(true);
        expect(switchInput.getAttribute('role')).toBe('switch');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('captures serializable slice-event payloads for text and boolean controls', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-text-field name="query" value="draft" label="Query"></cem-text-field>
                <cem-checkbox name="enabled">Enabled</cem-checkbox>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-checkbox input');

        const fieldHost = harness.query<HTMLElement>('cem-text-field');
        const checkboxHost = harness.query<HTMLElement>('cem-checkbox');
        const input = harness.query<HTMLInputElement>('cem-text-field input');

        input.value = 'published';
        input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
        await nextRenderFrame();

        const textSnapshot = runtime.snapshotInstance(fieldHost);
        const textPayload = eventPayload(textSnapshot, 'value');
        expect(textSnapshot.slices.value).toBe('published');
        expect(textPayload).toMatchObject({
            bubbles: true,
            sliceValue: 'published',
            type: 'input',
        });
        expect(textPayload.target).toMatchObject({
            name: 'query',
            tag: 'input',
            type: 'text',
            value: 'published',
        });

        const nextCheckbox = harness.query<HTMLInputElement>('cem-checkbox input');
        nextCheckbox.checked = true;
        nextCheckbox.dispatchEvent(new Event('change', { bubbles: true, composed: true }));
        await nextRenderFrame();

        const checkboxSnapshot = runtime.snapshotInstance(checkboxHost);
        const checkboxPayload = eventPayload(checkboxSnapshot, 'checked');
        expect(checkboxSnapshot.slices.checked).toBe(true);
        expect(checkboxPayload).toMatchObject({
            bubbles: true,
            sliceValue: true,
            type: 'change',
        });
        expect(checkboxPayload.target).toMatchObject({
            checked: true,
            name: 'enabled',
            tag: 'input',
            type: 'checkbox',
        });
        expect(nextCheckbox.isConnected).toBe(true);
        expect(harness.query<HTMLInputElement>('cem-checkbox input')).toBe(nextCheckbox);
    });

    it('preserves empty states, indeterminate progress, and live-region roles', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-list label="Empty tasks"></cem-list>
                <cem-table label="Empty table"></cem-table>
                <cem-progress label="Loading assets"></cem-progress>
                <cem-toast>Saved</cem-toast>
                <cem-alert tone="danger" role="alert">Resolve errors.</cem-alert>
                <cem-skeleton label="Loading card"></cem-skeleton>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-skeleton .cem-skeleton');

        const list = harness.query<HTMLUListElement>('cem-list ul');
        const table = harness.query<HTMLElement>('cem-table [role="table"]');
        const progress = harness.query<HTMLProgressElement>('cem-progress progress');
        const toast = harness.query<HTMLElement>('cem-toast [role="status"]');
        const alert = harness.query<HTMLElement>('cem-alert [role="alert"]');
        const skeleton = harness.query<HTMLElement>('cem-skeleton .cem-skeleton');

        assertStateHostsRendered(harness.root, 'cem-list, cem-table, cem-progress, cem-toast, cem-alert, cem-skeleton');
        expect(assertAccessibleName(list, 'Empty tasks')).toBe('Empty tasks');
        expect(list.querySelector('.cem-list__empty')?.textContent?.trim()).toBe('No items');
        expect(assertAccessibleName(table, 'Empty table')).toBe('Empty table');
        expect(table.querySelector('[role="cell"]')?.textContent?.trim()).toBe('No rows');
        expect(progress.hasAttribute('value')).toBe(false);
        expect(assertAccessibleName(progress, 'Loading assets')).toBe('Loading assets');
        expect(toast.getAttribute('aria-live')).toBe('polite');
        expect(toast.textContent?.trim()).toBe('Saved');
        expect(alert.getAttribute('data-tone')).toBe('danger');
        expect(alert.textContent?.trim()).toBe('Resolve errors.');
        expect(skeleton.getAttribute('aria-hidden')).toBe('true');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });
});

async function waitForStateSelector(root: ParentNode, selector: string): Promise<Element> {
    const deadline = Date.now() + 1000;

    while (Date.now() < deadline) {
        const element = root.querySelector(selector);

        if (element) {
            return element;
        }

        await nextRenderFrame();
    }

    throw new Error(`Expected state render output matching ${selector}`);
}

function assertStateHostsRendered(root: ParentNode, selector: string): void {
    for (const host of Array.from(root.querySelectorAll<HTMLElement>(selector))) {
        assertLightDomRendered(host);
        expect(host.shadowRoot).toBeNull();
    }
}

function eventPayload(snapshot: DataIslandSnapshot, name: string): SerializedEventPayload {
    const payload = snapshot.eventPayloads[name];

    if (!isSerializedEventPayload(payload)) {
        throw new Error(`Expected serialized event payload for ${name}`);
    }

    return payload;
}

function isSerializedEventPayload(value: unknown): value is SerializedEventPayload {
    if (!value || typeof value !== 'object') {
        return false;
    }

    const record = value as Partial<SerializedEventPayload>;

    return typeof record.type === 'string' && 'sliceValue' in record;
}
