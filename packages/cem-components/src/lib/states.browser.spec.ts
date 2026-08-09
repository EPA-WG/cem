import { CemElementRuntime, type DataIslandSnapshot } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

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

    it('toggles checkable content chips without changing passive chip semantics', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-chip label="Static topic">Static topic</cem-chip>
                <cem-chip label="Owner filter" checkable>Owner</cem-chip>
                <cem-chip label="Ready filter" checkable checked>Ready</cem-chip>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-chip[checkable] button');

        const passive = harness.query<HTMLElement>('cem-chip:not([checkable]) .cem-chip');
        const uncheckedHost = harness.query<HTMLElement>('cem-chip[checkable]:not([checked])');
        const unchecked = harness.query<HTMLButtonElement>('cem-chip[checkable]:not([checked]) button');
        const checked = harness.query<HTMLButtonElement>('cem-chip[checkable][checked] button');

        expect(passive).toBeInstanceOf(HTMLSpanElement);
        expect(passive.hasAttribute('aria-pressed')).toBe(false);
        expect(passive.tabIndex).toBe(-1);
        expect(unchecked).toBeInstanceOf(HTMLButtonElement);
        expect(unchecked.type).toBe('button');
        expect(assertAccessibleName(unchecked, 'Owner filter')).toBe('Owner filter');
        expect(unchecked.getAttribute('aria-pressed')).toBe('false');
        expect(checked.getAttribute('aria-pressed')).toBe('true');
        await assertFocusVisible(unchecked);

        unchecked.click();
        await nextRenderFrame();

        const pressed = harness.query<HTMLButtonElement>('cem-chip[checkable]:not([checked]) button');
        const pressedSnapshot = runtime.snapshotInstance(uncheckedHost);
        const pressedPayload = eventPayload(pressedSnapshot, 'checked');
        expect(pressed.getAttribute('aria-pressed')).toBe('true');
        expect(pressedSnapshot.slices.checked).toBe(true);
        expect(pressedPayload).toMatchObject({
            bubbles: true,
            sliceValue: true,
            type: 'click',
        });
        expect(pressedPayload.target).toMatchObject({
            tag: 'button',
            type: 'button',
        });

        pressed.click();
        await nextRenderFrame();

        const released = harness.query<HTMLButtonElement>('cem-chip[checkable]:not([checked]) button');
        const releasedSnapshot = runtime.snapshotInstance(uncheckedHost);
        expect(released.getAttribute('aria-pressed')).toBe('false');
        expect(releasedSnapshot.slices.checked).toBe(false);
        expect(eventPayload(releasedSnapshot, 'checked').sliceValue).toBe(false);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('toggles collapsible navigation without changing passive landmark semantics', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <form>
                <cem-stack gap="sm">
                    <cem-nav label="Primary navigation">
                        <a href="#overview">Overview</a>
                    </cem-nav>
                    <cem-nav label="Workspace navigation" collapsible>
                        <a href="#workspace">Workspace</a>
                    </cem-nav>
                    <cem-nav label="Admin navigation" collapsible expanded>
                        <a href="#admin">Admin</a>
                    </cem-nav>
                </cem-stack>
            </form>
        `);
        await waitForStateSelector(root, 'cem-nav[collapsible] button');

        const form = harness.query<HTMLFormElement>('form');
        const passiveNav = harness.query<HTMLElement>('cem-nav:not([collapsible]) nav');
        const passiveLink = harness.query<HTMLAnchorElement>('cem-nav:not([collapsible]) a');
        const closedHost = harness.query<HTMLElement>('cem-nav[collapsible]:not([expanded])');
        const closedNav = harness.query<HTMLElement>('cem-nav[collapsible]:not([expanded]) nav');
        const closedButton = harness.query<HTMLButtonElement>('cem-nav[collapsible]:not([expanded]) button');
        const closedContent = harness.query<HTMLDivElement>(
            'cem-nav[collapsible]:not([expanded]) .cem-nav__content',
        );
        const closedLink = harness.query<HTMLAnchorElement>('cem-nav[collapsible]:not([expanded]) a');
        const openNav = harness.query<HTMLElement>('cem-nav[collapsible][expanded] nav');
        const openButton = harness.query<HTMLButtonElement>('cem-nav[collapsible][expanded] button');
        const openContent = harness.query<HTMLDivElement>('cem-nav[collapsible][expanded] .cem-nav__content');

        expect(passiveNav.children).toHaveLength(1);
        expect(passiveNav.firstElementChild).toBe(passiveLink);
        expect(passiveNav.querySelector('button, .cem-nav__content')).toBeNull();
        expect(assertAccessibleName(passiveNav, 'Primary navigation')).toBe('Primary navigation');
        expect(assertAccessibleName(closedNav, 'Workspace navigation')).toBe('Workspace navigation');
        expect(assertAccessibleName(openNav, 'Admin navigation')).toBe('Admin navigation');
        expect(closedButton).toBeInstanceOf(HTMLButtonElement);
        expect(closedButton.type).toBe('button');
        expect(closedButton.name).toBe('');
        expect(assertAccessibleName(closedButton, 'Workspace navigation')).toBe('Workspace navigation');
        expect(assertAccessibleName(openButton, 'Admin navigation')).toBe('Admin navigation');
        expect(closedButton.getAttribute('aria-expanded')).toBe('false');
        expect(openButton.getAttribute('aria-expanded')).toBe('true');
        expect(closedContent.hidden).toBe(true);
        expect(openContent.hidden).toBe(false);
        expect(Array.from(form.querySelectorAll('button')).every((button) => !button.hasAttribute('aria-controls'))).toBe(
            true,
        );
        expect(form.querySelector('[role="menu"], [role="menubar"], [role="menuitem"], [aria-haspopup]')).toBeNull();
        expect(form.querySelector('details, summary')).toBeNull();
        expect(Array.from(new FormData(form).entries())).toEqual([]);

        await userEvent.click(closedButton);
        await nextRenderFrame();

        const pointerButton = harness.query<HTMLButtonElement>('cem-nav[collapsible]:not([expanded]) button');
        const pointerContent = harness.query<HTMLDivElement>(
            'cem-nav[collapsible]:not([expanded]) .cem-nav__content',
        );
        const pointerSnapshot = runtime.snapshotInstance(closedHost);
        expect(pointerButton).toBe(closedButton);
        expect(pointerContent).toBe(closedContent);
        expect(pointerButton.getAttribute('aria-expanded')).toBe('true');
        expect(pointerContent.hidden).toBe(false);
        expect(document.activeElement).toBe(pointerButton);
        expect(pointerSnapshot.slices.expanded).toBe(true);
        expect(eventPayload(pointerSnapshot, 'expanded')).toMatchObject({
            bubbles: true,
            sliceValue: true,
            type: 'click',
        });
        expect(eventPayload(pointerSnapshot, 'expanded').target).toMatchObject({
            tag: 'button',
            type: 'button',
        });

        await userEvent.tab();
        expect(document.activeElement).toBe(closedLink);

        pointerButton.focus();
        await userEvent.keyboard('{Enter}');
        await nextRenderFrame();

        const enterButton = harness.query<HTMLButtonElement>('cem-nav[collapsible]:not([expanded]) button');
        const enterContent = harness.query<HTMLDivElement>('cem-nav[collapsible]:not([expanded]) .cem-nav__content');
        const enterSnapshot = runtime.snapshotInstance(closedHost);
        expect(enterButton).toBe(closedButton);
        expect(enterContent).toBe(closedContent);
        expect(enterButton.getAttribute('aria-expanded')).toBe('false');
        expect(enterContent.hidden).toBe(true);
        expect(document.activeElement).toBe(enterButton);
        expect(enterSnapshot.slices.expanded).toBe(false);
        expect(eventPayload(enterSnapshot, 'expanded').sliceValue).toBe(false);

        await userEvent.tab();
        expect(document.activeElement).toBe(openButton);

        enterButton.focus();
        await userEvent.keyboard(' ');
        await nextRenderFrame();

        const spaceButton = harness.query<HTMLButtonElement>('cem-nav[collapsible]:not([expanded]) button');
        const spaceContent = harness.query<HTMLDivElement>('cem-nav[collapsible]:not([expanded]) .cem-nav__content');
        const spaceSnapshot = runtime.snapshotInstance(closedHost);
        expect(spaceButton).toBe(closedButton);
        expect(spaceContent).toBe(closedContent);
        expect(spaceButton.getAttribute('aria-expanded')).toBe('true');
        expect(spaceContent.hidden).toBe(false);
        expect(document.activeElement).toBe(spaceButton);
        expect(spaceSnapshot.slices.expanded).toBe(true);
        expect(eventPayload(spaceSnapshot, 'expanded').sliceValue).toBe(true);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('selects declarative list options without changing passive list semantics', async () => {
        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-list label="Static topics">
                    <li>Static topic</li>
                </cem-list>
                <cem-list label="Asset type" selectable value="document" size="3">
                    <cem-list-option value="image" selected>Image</cem-list-option>
                    <cem-list-option value="document">Document</cem-list-option>
                    <cem-list-option value="archive" disabled>Archive</cem-list-option>
                    <span value="ignored">Ignored non-option</span>
                    <div><cem-list-option value="nested">Ignored nested option</cem-list-option></div>
                </cem-list>
                <cem-list label="Fallback type" selectable size="3">
                    <cem-list-option value="image" selected>Image</cem-list-option>
                    <cem-list-option value="document" selected>Document</cem-list-option>
                </cem-list>
            </cem-stack>
        `);
        await waitForStateSelector(root, 'cem-list[selectable] select');

        const passive = harness.query<HTMLUListElement>('cem-list:not([selectable]) ul');
        const listHost = harness.query<HTMLElement>('cem-list[selectable][value]');
        const listbox = harness.query<HTMLSelectElement>('cem-list[selectable][value] select');
        const fallback = harness.query<HTMLSelectElement>('cem-list[selectable]:not([value]) select');
        const options = Array.from(listbox.options);

        expect(passive).toBeInstanceOf(HTMLUListElement);
        expect(passive.textContent?.trim()).toBe('Static topic');
        expect(listbox).toBeInstanceOf(HTMLSelectElement);
        expect(listbox.size).toBe(3);
        expect(listbox.multiple).toBe(false);
        expect(listbox.name).toBe('');
        expect(assertAccessibleName(listbox, 'Asset type')).toBe('Asset type');
        expect(options.map((option) => `${option.value}:${option.text}`).join('|')).toBe(
            'image:Image|document:Document|archive:Archive',
        );
        expect(options.map((option) => option.selected).join('|')).toBe('false|true|false');
        expect(options.map((option) => option.getAttribute('aria-selected')).join('|')).toBe('false|true|false');
        expect(options[2]?.disabled).toBe(true);
        expect(fallback.value).toBe('document');
        expect(Array.from(fallback.options).map((option) => option.selected).join('|')).toBe('false|true');
        expect(
            Array.from(fallback.options).map((option) => option.getAttribute('aria-selected')).join('|'),
        ).toBe('false|true');

        await userEvent.selectOptions(listbox, 'image');
        await nextRenderFrame();

        const pointerSelected = harness.query<HTMLSelectElement>('cem-list[selectable][value] select');
        const pointerSnapshot = runtime.snapshotInstance(listHost);
        const pointerPayload = eventPayload(pointerSnapshot, 'value');
        expect(pointerSelected.value).toBe('image');
        expect(pointerSelected.options[0]?.selected).toBe(true);
        expect(pointerSelected.options[0]?.getAttribute('aria-selected')).toBe('true');
        expect(pointerSnapshot.slices.value).toBe('image');
        expect(pointerPayload).toMatchObject({
            bubbles: true,
            sliceValue: 'image',
            type: 'change',
        });
        expect(pointerPayload.target).toMatchObject({
            tag: 'select',
            value: 'image',
        });

        await assertFocusVisible(pointerSelected);
        expect(document.activeElement).toBe(pointerSelected);
        expect(pointerSelected.selectedOptions[0]?.matches(':focus')).toBe(false);
        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();

        const keyboardSelected = harness.query<HTMLSelectElement>('cem-list[selectable][value] select');
        const keyboardSnapshot = runtime.snapshotInstance(listHost);
        expect(keyboardSelected.value).toBe('document');
        expect(keyboardSelected.options[1]?.getAttribute('aria-selected')).toBe('true');
        expect(keyboardSnapshot.slices.value).toBe('document');
        expect(eventPayload(keyboardSnapshot, 'value').sliceValue).toBe('document');

        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();

        const disabledSkipped = harness.query<HTMLSelectElement>('cem-list[selectable][value] select');
        expect(disabledSkipped.value).toBe('document');
        expect(disabledSkipped.options[2]?.disabled).toBe(true);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('marks explicit busy cards without making nested content primitives loading owners', async () => {
        const authoredFallback = document.createElement('cem-card');
        authoredFallback.setAttribute('label', 'Assets');
        authoredFallback.setAttribute('busy', '');
        authoredFallback.innerHTML = `
            <span slot="title">Assets</span>
            <p>Loading assets…</p>
            <cem-skeleton label="Asset rows"></cem-skeleton>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('Loading assets…');
        expect(authoredFallback.querySelector('cem-skeleton')?.getAttribute('label')).toBe('Asset rows');

        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-card label="Ordinary card">
                    <span slot="title">Ordinary card</span>
                    <p>Ready</p>
                </cem-card>
                <cem-card label="Initial assets" busy>
                    <span slot="title">Initial assets</span>
                    <p class="loading-message">Loading assets…</p>
                    <cem-skeleton label="Asset rows"></cem-skeleton>
                    <cem-skeleton label="Asset preview"></cem-skeleton>
                </cem-card>
                <cem-card label="Profile" busy="false">
                    <span slot="title">Profile</span>
                    <p>Grace Hopper</p>
                    <button type="button">Edit profile</button>
                </cem-card>
                <cem-card label="Empty assets" busy>
                    <span slot="title">Empty assets</span>
                    <cem-list label="Assets"></cem-list>
                </cem-card>
                <cem-list id="busy-list" label="Standalone list" busy></cem-list>
                <cem-table id="busy-table" label="Standalone table" busy></cem-table>
                <cem-media-preview id="busy-preview" label="Standalone preview" busy></cem-media-preview>
            </cem-stack>
        `);
        await waitForStateSelector(root, '#busy-preview > .cem-media-preview');

        const ordinary = harness.query<HTMLElement>('cem-card[label="Ordinary card"] > section');
        const initialHost = harness.query<HTMLElement>('cem-card[label="Initial assets"]');
        const initial = harness.query<HTMLElement>('cem-card[label="Initial assets"] > section');
        const initialHeader = harness.query<HTMLElement>('cem-card[label="Initial assets"] > section > header');
        const initialBody = harness.query<HTMLElement>('cem-card[label="Initial assets"] .cem-card__body');
        const initialMessage = harness.query<HTMLParagraphElement>('cem-card[label="Initial assets"] .loading-message');
        const skeletons = Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-card[label="Initial assets"] cem-skeleton .cem-skeleton'),
        );
        const refreshHost = harness.query<HTMLElement>('cem-card[label="Profile"]');
        const refreshCard = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const refreshHeader = harness.query<HTMLElement>('cem-card[label="Profile"] > section > header');
        const refreshBody = harness.query<HTMLElement>('cem-card[label="Profile"] .cem-card__body');
        const refreshButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const emptyHost = harness.query<HTMLElement>('cem-card[label="Empty assets"]');
        const emptyCard = harness.query<HTMLElement>('cem-card[label="Empty assets"] > section');
        const emptyList = harness.query<HTMLUListElement>('cem-card[label="Empty assets"] cem-list ul');
        const standaloneList = harness.query<HTMLUListElement>('#busy-list > ul');
        const standaloneTable = harness.query<HTMLElement>('#busy-table > [role="table"]');
        const standalonePreview = harness.query<HTMLElement>('#busy-preview > .cem-media-preview');

        assertStateHostsRendered(
            harness.root,
            'cem-card, #busy-list, #busy-table, #busy-preview, cem-card cem-skeleton',
        );
        expect(ordinary.className).toBe('cem-card');
        expect(ordinary.getAttribute('aria-label')).toBe('Ordinary card');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.hasAttribute('aria-busy')).toBe(false);
        expect(ordinary.querySelector('.cem-card__header')?.textContent?.trim()).toBe('Ordinary card');
        expect(ordinary.querySelector('.cem-card__body')?.textContent?.trim()).toBe('Ready');

        expect(assertAccessibleName(initial, 'Initial assets')).toBe('Initial assets');
        expect(initial.getAttribute('data-state')).toBe('loading');
        expect(initial.getAttribute('aria-busy')).toBe('true');
        expect(initialHeader.className).toBe('cem-card__header');
        expect(initialBody.className).toBe('cem-card__body');
        expect(initialMessage.textContent?.trim()).toBe('Loading assets…');
        expect(initialMessage.getAttribute('role')).toBeNull();
        expect(initialMessage.getAttribute('aria-live')).toBeNull();
        expect(skeletons).toHaveLength(2);
        expect(skeletons.every((skeleton) => skeleton.getAttribute('aria-hidden') === 'true')).toBe(true);
        expect(initial.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(initial.hasAttribute('inert')).toBe(false);

        expect(refreshCard.getAttribute('data-state')).toBe('loading');
        expect(refreshCard.getAttribute('aria-busy')).toBe('true');
        expect(refreshCard.textContent).toContain('Grace Hopper');
        expect(refreshButton.disabled).toBe(false);
        expect(refreshButton.tabIndex).toBe(0);
        expect(emptyCard.getAttribute('data-state')).toBe('loading');
        expect(emptyList.querySelector('.cem-list__empty')?.textContent?.trim()).toBe('No items');

        for (const candidate of [standaloneList, standaloneTable, standalonePreview]) {
            expect(candidate.hasAttribute('data-state')).toBe(false);
            expect(candidate.hasAttribute('aria-busy')).toBe(false);
        }

        const refreshRect = refreshCard.getBoundingClientRect();
        expect(refreshRect.width).toBeGreaterThan(0);
        expect(refreshRect.height).toBeGreaterThan(0);
        const lifecycleEvents: string[] = [];
        for (const name of ['cem-loaded', 'cem-error', 'cem-cancel']) {
            refreshHost.addEventListener(name, () => lifecycleEvents.push(name));
        }

        await assertFocusVisible(refreshButton);
        refreshHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const settledCard = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const settledHeader = harness.query<HTMLElement>('cem-card[label="Profile"] > section > header');
        const settledBody = harness.query<HTMLElement>('cem-card[label="Profile"] .cem-card__body');
        const settledButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const settledRect = settledCard.getBoundingClientRect();
        expect(settledCard).toBe(refreshCard);
        expect(settledHeader).toBe(refreshHeader);
        expect(settledBody).toBe(refreshBody);
        expect(settledButton).toBe(refreshButton);
        expect(settledCard.hasAttribute('data-state')).toBe(false);
        expect(settledCard.hasAttribute('aria-busy')).toBe(false);
        expect(settledRect.width).toBe(refreshRect.width);
        expect(settledRect.height).toBe(refreshRect.height);
        expect(document.activeElement).toBe(settledButton);

        refreshHost.setAttribute('busy', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const pendingAgain = harness.query<HTMLElement>('cem-card[label="Profile"] > section');
        const pendingButton = harness.query<HTMLButtonElement>('cem-card[label="Profile"] button');
        const pendingRect = pendingAgain.getBoundingClientRect();
        expect(pendingAgain).toBe(refreshCard);
        expect(pendingButton).toBe(refreshButton);
        expect(pendingAgain.getAttribute('data-state')).toBe('loading');
        expect(pendingAgain.getAttribute('aria-busy')).toBe('true');
        expect(pendingRect.width).toBe(refreshRect.width);
        expect(pendingRect.height).toBe(refreshRect.height);
        expect(document.activeElement).toBe(pendingButton);

        emptyHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const settledEmptyCard = harness.query<HTMLElement>('cem-card[label="Empty assets"] > section');
        const settledEmptyList = harness.query<HTMLUListElement>('cem-card[label="Empty assets"] cem-list ul');
        expect(settledEmptyCard).toBe(emptyCard);
        expect(settledEmptyCard.hasAttribute('data-state')).toBe(false);
        expect(settledEmptyCard.hasAttribute('aria-busy')).toBe(false);
        expect(settledEmptyList).toBe(emptyList);
        expect(settledEmptyList.querySelector('.cem-list__empty')?.textContent?.trim()).toBe('No items');

        const initialSnapshot = runtime.snapshotInstance(initialHost);
        const refreshSnapshot = runtime.snapshotInstance(refreshHost);
        expect(initialSnapshot.slices).not.toHaveProperty('busy');
        expect(initialSnapshot.slices).not.toHaveProperty('loading');
        expect(initialSnapshot.eventPayloads).not.toHaveProperty('busy');
        expect(initialSnapshot.eventPayloads).not.toHaveProperty('loading');
        expect(refreshSnapshot.slices).not.toHaveProperty('busy');
        expect(refreshSnapshot.eventPayloads).not.toHaveProperty('busy');
        expect(lifecycleEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('marks explicit empty workflow surfaces without inferring layout emptiness', async () => {
        const authoredFallback = document.createElement('cem-surface');
        authoredFallback.setAttribute('label', 'Asset results');
        authoredFallback.setAttribute('empty', '');
        authoredFallback.innerHTML = `
            <h2>No assets yet</h2>
            <p>Upload an asset to begin building this collection.</p>
            <a href="#authored-upload">Upload an asset</a>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('No assets yet');
        expect(authoredFallback.querySelector('a')?.getAttribute('href')).toBe('#authored-upload');

        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-surface label="Dashboard">
                    <p>Ready</p>
                </cem-surface>
                <cem-surface label="Asset results" empty>
                    <h2>No assets yet</h2>
                    <p>Upload an asset to begin building this collection.</p>
                    <a href="#new-asset">Upload an asset</a>
                </cem-surface>
                <cem-surface label="False-token results" empty="false">
                    <p>No matching results.</p>
                    <a href="#clear-filters">Clear filters</a>
                </cem-surface>
                <cem-stack id="empty-stack" empty></cem-stack>
                <cem-grid id="empty-grid" empty></cem-grid>
            </cem-stack>
        `);
        await waitForStateSelector(root, '#empty-grid > .cem-grid');

        const ordinaryHost = harness.query<HTMLElement>('cem-surface:not([empty])');
        const ordinary = harness.query<HTMLElement>('cem-surface:not([empty]) > section');
        const emptyHost = harness.query<HTMLElement>('cem-surface[empty=""]');
        const emptySurface = harness.query<HTMLElement>('cem-surface[empty=""] > section');
        const falseTokenSurface = harness.query<HTMLElement>('cem-surface[empty="false"] > section');
        const emptyStack = harness.query<HTMLDivElement>('#empty-stack > .cem-stack');
        const emptyGrid = harness.query<HTMLDivElement>('#empty-grid > .cem-grid');
        const guidance = harness.query<HTMLParagraphElement>('cem-surface[empty=""] p');
        const recovery = harness.query<HTMLAnchorElement>('cem-surface[empty=""] a');

        assertStateHostsRendered(harness.root, 'cem-surface, #empty-stack, #empty-grid');
        expect(ordinary.className).toBe('cem-surface cem-surface--default');
        expect(ordinary.getAttribute('aria-label')).toBe('Dashboard');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.children).toHaveLength(1);
        expect(ordinary.textContent?.trim()).toBe('Ready');
        expect(assertAccessibleName(emptySurface, 'Asset results')).toBe('Asset results');
        expect(emptySurface.className).toBe('cem-surface cem-surface--default');
        expect(emptySurface.getAttribute('data-state')).toBe('empty');
        expect(emptySurface.children).toHaveLength(3);
        expect(emptySurface.querySelector('h2')?.textContent?.trim()).toBe('No assets yet');
        expect(guidance.textContent?.trim()).toBe('Upload an asset to begin building this collection.');
        expect(recovery).toBeInstanceOf(HTMLAnchorElement);
        expect(recovery.getAttribute('href')).toBe('#new-asset');
        expect(assertAccessibleName(recovery, 'Upload an asset')).toBe('Upload an asset');
        expect(falseTokenSurface.getAttribute('data-state')).toBe('empty');
        expect(emptySurface.getAttribute('role')).toBeNull();
        expect(emptySurface.getAttribute('aria-live')).toBeNull();
        expect(emptySurface.getAttribute('aria-atomic')).toBeNull();
        expect(emptySurface.getAttribute('tabindex')).toBeNull();
        expect(emptySurface.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(emptyStack.hasAttribute('data-state')).toBe(false);
        expect(emptyStack.getAttribute('role')).toBeNull();
        expect(emptyStack.childElementCount).toBe(0);
        expect(emptyStack.textContent?.trim()).toBe('');
        expect(emptyGrid.hasAttribute('data-state')).toBe(false);
        expect(emptyGrid.getAttribute('role')).toBeNull();
        expect(emptyGrid.childElementCount).toBe(0);
        expect(emptyGrid.textContent?.trim()).toBe('');

        await assertFocusVisible(recovery);
        emptyHost.removeAttribute('empty');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const ordinaryTransition = harness.query<HTMLElement>('cem-surface[label="Asset results"] > section');
        const recoveryAfterRemoval = harness.query<HTMLAnchorElement>('cem-surface[label="Asset results"] a');
        expect(ordinaryTransition).toBe(emptySurface);
        expect(recoveryAfterRemoval).toBe(recovery);
        expect(ordinaryTransition.hasAttribute('data-state')).toBe(false);
        expect(document.activeElement).toBe(recoveryAfterRemoval);

        emptyHost.setAttribute('empty', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(emptyHost);
        await nextRenderFrame();

        const emptyTransition = harness.query<HTMLElement>('cem-surface[label="Asset results"] > section');
        const recoveryAfterAddition = harness.query<HTMLAnchorElement>('cem-surface[label="Asset results"] a');
        const snapshot = runtime.snapshotInstance(emptyHost);
        expect(emptyTransition).toBe(emptySurface);
        expect(recoveryAfterAddition).toBe(recovery);
        expect(emptyTransition.getAttribute('data-state')).toBe('empty');
        expect(document.activeElement).toBe(recoveryAfterAddition);
        expect(snapshot.slices).not.toHaveProperty('empty');
        expect(snapshot.eventPayloads).not.toHaveProperty('empty');
        expect(runtime.snapshotInstance(ordinaryHost).slices).not.toHaveProperty('empty');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    });

    it('marks explicit busy workflow surfaces without making formatting containers loading owners', async () => {
        const authoredFallback = document.createElement('cem-surface');
        authoredFallback.setAttribute('label', 'Asset workspace');
        authoredFallback.setAttribute('busy', '');
        authoredFallback.innerHTML = `
            <h2>Asset workspace</h2>
            <p>Loading filters and results…</p>
            <cem-stack gap="md">
                <cem-skeleton label="Asset filters"></cem-skeleton>
                <cem-skeleton label="Asset results"></cem-skeleton>
            </cem-stack>
        `;
        expect(authoredFallback.childElementCount).toBe(3);
        expect(authoredFallback.textContent).toContain('Loading filters and results…');
        expect(authoredFallback.querySelectorAll('cem-skeleton')).toHaveLength(2);

        harness = createComponentHarness();
        const root = await harness.render(`
            <cem-stack gap="sm">
                <cem-surface label="Ordinary workspace">
                    <p>Ready</p>
                </cem-surface>
                <cem-surface label="Initial workspace" busy>
                    <h2>Asset workspace</h2>
                    <p class="loading-message">Loading filters and results…</p>
                    <cem-stack gap="md">
                        <cem-skeleton label="Asset filters"></cem-skeleton>
                        <cem-skeleton label="Asset results"></cem-skeleton>
                    </cem-stack>
                    <cem-surface label="Nested ordinary surface">
                        <p>Nested content</p>
                    </cem-surface>
                </cem-surface>
                <cem-surface label="Profile workspace" busy="false">
                    <h2>Profile workspace</h2>
                    <cem-grid columns="2" gap="lg">
                        <cem-card label="Profile details">
                            <p>Grace Hopper</p>
                            <button type="button">Edit profile</button>
                        </cem-card>
                        <cem-card label="Preferences">
                            <p>Daily summaries</p>
                        </cem-card>
                    </cem-grid>
                </cem-surface>
                <cem-surface label="Empty transition" busy empty>
                    <h2>No assets yet</h2>
                    <p>Upload an asset to begin building this collection.</p>
                    <a href="#new-asset">Upload an asset</a>
                </cem-surface>
                <cem-stack id="busy-stack" gap="sm" busy>
                    <p>Formatting stack</p>
                </cem-stack>
                <cem-grid id="busy-grid" columns="2" gap="md" busy>
                    <p>First cell</p>
                    <p>Second cell</p>
                </cem-grid>
            </cem-stack>
        `);
        await waitForStateSelector(root, '#busy-grid > .cem-grid');

        const ordinary = harness.query<HTMLElement>('cem-surface[label="Ordinary workspace"] > section');
        const initialHost = harness.query<HTMLElement>('cem-surface[label="Initial workspace"]');
        const initial = harness.query<HTMLElement>('cem-surface[label="Initial workspace"] > section');
        const initialMessage = harness.query<HTMLParagraphElement>(
            'cem-surface[label="Initial workspace"] > section > .loading-message',
        );
        const initialStack = harness.query<HTMLDivElement>(
            'cem-surface[label="Initial workspace"] > section > cem-stack > .cem-stack',
        );
        const skeletons = Array.from(
            harness.root.querySelectorAll<HTMLElement>(
                'cem-surface[label="Initial workspace"] > section > cem-stack cem-skeleton > .cem-skeleton',
            ),
        );
        const nestedSurface = harness.query<HTMLElement>(
            'cem-surface[label="Initial workspace"] cem-surface[label="Nested ordinary surface"] > section',
        );
        const refreshHost = harness.query<HTMLElement>('cem-surface[label="Profile workspace"]');
        const refreshSurface = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const refreshGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const refreshCards = Array.from(
            harness.root.querySelectorAll<HTMLElement>('cem-surface[label="Profile workspace"] cem-card > section'),
        );
        const refreshButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        const transitionHost = harness.query<HTMLElement>('cem-surface[label="Empty transition"]');
        const transitionSurface = harness.query<HTMLElement>('cem-surface[label="Empty transition"] > section');
        const transitionRecovery = harness.query<HTMLAnchorElement>('cem-surface[label="Empty transition"] a');
        const busyStack = harness.query<HTMLDivElement>('#busy-stack > .cem-stack');
        const busyGrid = harness.query<HTMLDivElement>('#busy-grid > .cem-grid');

        assertStateHostsRendered(
            harness.root,
            'cem-surface, #busy-stack, #busy-grid, cem-surface cem-stack, cem-surface cem-grid, cem-surface cem-card, cem-surface cem-skeleton',
        );
        expect(ordinary.className).toBe('cem-surface cem-surface--default');
        expect(ordinary.getAttribute('aria-label')).toBe('Ordinary workspace');
        expect(ordinary.hasAttribute('data-state')).toBe(false);
        expect(ordinary.hasAttribute('aria-busy')).toBe(false);

        expect(assertAccessibleName(initial, 'Initial workspace')).toBe('Initial workspace');
        expect(initial.getAttribute('data-state')).toBe('loading');
        expect(initial.getAttribute('aria-busy')).toBe('true');
        expect(initialMessage.textContent?.trim()).toBe('Loading filters and results…');
        expect(initialMessage.getAttribute('role')).toBeNull();
        expect(initialMessage.getAttribute('aria-live')).toBeNull();
        expect(initialStack.getAttribute('data-gap')).toBe('md');
        expect(initialStack.hasAttribute('data-state')).toBe(false);
        expect(initialStack.hasAttribute('aria-busy')).toBe(false);
        expect(skeletons).toHaveLength(2);
        expect(skeletons.every((skeleton) => skeleton.getAttribute('aria-hidden') === 'true')).toBe(true);
        expect(nestedSurface.hasAttribute('data-state')).toBe(false);
        expect(nestedSurface.hasAttribute('aria-busy')).toBe(false);
        expect(initial.querySelector('[role="status"], [role="alert"], [aria-live]')).toBeNull();
        expect(initial.hasAttribute('inert')).toBe(false);

        expect(refreshSurface.getAttribute('data-state')).toBe('loading');
        expect(refreshSurface.getAttribute('aria-busy')).toBe('true');
        expect(refreshGrid.getAttribute('data-columns')).toBe('2');
        expect(refreshGrid.getAttribute('data-gap')).toBe('lg');
        expect(refreshCards).toHaveLength(2);
        expect(refreshCards.every((card) => !card.hasAttribute('data-state'))).toBe(true);
        expect(refreshCards.every((card) => !card.hasAttribute('aria-busy'))).toBe(true);
        expect(refreshButton.disabled).toBe(false);
        expect(refreshButton.tabIndex).toBe(0);

        expect(transitionSurface.getAttribute('data-state')).toBe('loading');
        expect(transitionSurface.getAttribute('aria-busy')).toBe('true');
        expect(transitionSurface.textContent).toContain('No assets yet');
        for (const candidate of [busyStack, busyGrid]) {
            expect(candidate.hasAttribute('data-state')).toBe(false);
            expect(candidate.hasAttribute('aria-busy')).toBe(false);
        }

        const refreshRect = refreshSurface.getBoundingClientRect();
        const refreshGridRect = refreshGrid.getBoundingClientRect();
        const refreshChildren = Array.from(refreshGrid.children);
        const refreshChildPositions = refreshChildren.map((child) => {
            const rect = child.getBoundingClientRect();
            return [rect.x, rect.y, rect.width, rect.height];
        });
        expect(refreshRect.width).toBeGreaterThan(0);
        expect(refreshRect.height).toBeGreaterThan(0);
        expect(refreshGridRect.width).toBeGreaterThan(0);
        expect(refreshGridRect.height).toBeGreaterThan(0);
        const lifecycleEvents: string[] = [];
        for (const name of ['cem-loaded', 'cem-error', 'cem-cancel']) {
            refreshHost.addEventListener(name, () => lifecycleEvents.push(name));
        }

        await assertFocusVisible(refreshButton);
        refreshHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const settledSurface = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const settledGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const settledButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        const settledRect = settledSurface.getBoundingClientRect();
        const settledGridRect = settledGrid.getBoundingClientRect();
        const settledChildren = Array.from(settledGrid.children);
        const settledChildPositions = settledChildren.map((child) => {
            const rect = child.getBoundingClientRect();
            return [rect.x, rect.y, rect.width, rect.height];
        });
        expect(settledSurface).toBe(refreshSurface);
        expect(settledGrid).toBe(refreshGrid);
        expect(settledButton).toBe(refreshButton);
        expect(settledChildren[0]).toBe(refreshChildren[0]);
        expect(settledChildren[1]).toBe(refreshChildren[1]);
        expect(settledSurface.hasAttribute('data-state')).toBe(false);
        expect(settledSurface.hasAttribute('aria-busy')).toBe(false);
        expect([settledRect.width, settledRect.height]).toEqual([refreshRect.width, refreshRect.height]);
        expect([settledGridRect.x, settledGridRect.y, settledGridRect.width, settledGridRect.height]).toEqual([
            refreshGridRect.x,
            refreshGridRect.y,
            refreshGridRect.width,
            refreshGridRect.height,
        ]);
        expect(settledChildPositions).toEqual(refreshChildPositions);
        expect(document.activeElement).toBe(settledButton);

        refreshHost.setAttribute('busy', '');
        await nextRenderFrame();
        await runtime.whenRenderSettled(refreshHost);
        await nextRenderFrame();

        const pendingAgain = harness.query<HTMLElement>('cem-surface[label="Profile workspace"] > section');
        const pendingGrid = harness.query<HTMLDivElement>(
            'cem-surface[label="Profile workspace"] > section > cem-grid > .cem-grid',
        );
        const pendingButton = harness.query<HTMLButtonElement>('cem-surface[label="Profile workspace"] button');
        expect(pendingAgain).toBe(refreshSurface);
        expect(pendingGrid).toBe(refreshGrid);
        expect(pendingButton).toBe(refreshButton);
        expect(pendingAgain.getAttribute('data-state')).toBe('loading');
        expect(pendingAgain.getAttribute('aria-busy')).toBe('true');
        expect(document.activeElement).toBe(pendingButton);

        await assertFocusVisible(transitionRecovery);
        transitionHost.removeAttribute('busy');
        await nextRenderFrame();
        await runtime.whenRenderSettled(transitionHost);
        await nextRenderFrame();

        const settledEmpty = harness.query<HTMLElement>('cem-surface[label="Empty transition"] > section');
        const settledRecovery = harness.query<HTMLAnchorElement>('cem-surface[label="Empty transition"] a');
        expect(settledEmpty).toBe(transitionSurface);
        expect(settledRecovery).toBe(transitionRecovery);
        expect(settledEmpty.getAttribute('data-state')).toBe('empty');
        expect(settledEmpty.hasAttribute('aria-busy')).toBe(false);
        expect(document.activeElement).toBe(settledRecovery);

        for (const host of [initialHost, refreshHost, transitionHost]) {
            const snapshot = runtime.snapshotInstance(host);
            expect(snapshot.slices).not.toHaveProperty('busy');
            expect(snapshot.slices).not.toHaveProperty('loading');
            expect(snapshot.slices).not.toHaveProperty('empty');
            expect(snapshot.eventPayloads).not.toHaveProperty('busy');
            expect(snapshot.eventPayloads).not.toHaveProperty('loading');
            expect(snapshot.eventPayloads).not.toHaveProperty('empty');
        }
        expect(lifecycleEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
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
