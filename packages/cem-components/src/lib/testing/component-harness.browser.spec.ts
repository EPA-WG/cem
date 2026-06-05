import { page } from 'vitest/browser';

import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    assertFocusVisible,
    assertLightDomRendered,
    captureVisualSnapshot,
    createComponentHarness,
    expectComponentEvent,
    type ComponentHarness,
} from './component-harness.js';

interface ChangeDetail {
    name: string;
    valid: boolean;
    value: string;
}

describe('component test harness', () => {
    let harness: ComponentHarness;

    afterEach(() => {
        harness?.cleanup();
    });

    it('asserts light-DOM rendering, accessible names, references, and visual snapshots', async () => {
        harness = createComponentHarness();

        const host = await harness.render(`
            <cem-harness-action
                id="save-action"
                aria-labelledby="save-label"
                aria-describedby="save-hint"
                style="display: inline-block; padding: 8px; color: rgb(17, 24, 39); background: rgb(255, 255, 255);"
            >
                <span id="save-label">Save</span>
                <span id="save-hint">Persists the current draft.</span>
                <button type="button">Save</button>
            </cem-harness-action>
        `);

        assertLightDomRendered(host);
        expect(assertAccessibleName(host, 'Save')).toBe('Save');
        expect(() => assertAriaReferenceIntegrity(host)).not.toThrow();

        const snapshot = captureVisualSnapshot(host);
        expect(snapshot).toMatchObject({
            tagName: 'cem-harness-action',
            text: 'Save Persists the current draft. Save',
        });
        expect(snapshot.rect.width).toBeGreaterThan(0);
        expect(snapshot.rect.height).toBeGreaterThan(0);
        expect(snapshot.styles.display).toBe('inline-block');

        const screenshot = await page.screenshot({ element: host, save: false });
        expect(screenshot.length).toBeGreaterThan(100);
    });

    it('asserts component events bubble, compose, and carry serializable details', async () => {
        harness = createComponentHarness();
        const host = await harness.render('<cem-harness-field><input value="ready" /></cem-harness-field>');
        const detail = { name: 'status', valid: true, value: 'ready' } satisfies ChangeDetail;

        const event = await expectComponentEvent<ChangeDetail>(
            host,
            'cem-change',
            () => {
                host.dispatchEvent(
                    new CustomEvent<ChangeDetail>('cem-change', {
                        bubbles: true,
                        composed: true,
                        detail,
                    }),
                );
            },
            { detail },
        );

        expect(event.detail).toEqual(detail);
    });

    it('asserts focus reaches the target and keeps a visible indicator', async () => {
        harness = createComponentHarness();

        const button = (await harness.render(`
            <button
                type="button"
                style="outline: 3px solid rgb(37, 99, 235); outline-offset: 2px;"
            >
                Focusable
            </button>
        `)) as HTMLButtonElement;

        await assertFocusVisible(button);
    });
});
