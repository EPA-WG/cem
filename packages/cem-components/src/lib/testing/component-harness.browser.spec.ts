import type { CemProducedElementBehavior } from '@epa-wg/cem-elements';
import { page } from 'vitest/browser';

import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    assertFocusVisible,
    assertLightDomRendered,
    captureFormSnapshot,
    captureVisualSnapshot,
    createSubstrateComponentHarness,
    expectComponentEvent,
    type SubstrateComponentHarness,
} from './component-harness.js';

interface ChangeDetail {
    name: string;
    valid: boolean;
    value: string;
}

const ACTION_TAG = 'cem-harness-substrate-action';
const FIELD_TAG = 'cem-harness-substrate-field';
const activationListeners = new WeakMap<HTMLElement, EventListener>();

const ACTION_BEHAVIOR: CemProducedElementBehavior = {
    connected(instance) {
        const listener: EventListener = (event) => {
            const target = event.target instanceof Element ? event.target.closest('button') : null;
            if (!target || !instance.contains(target)) return;
            instance.dispatchEvent(
                new CustomEvent<ChangeDetail>('cem-activate', {
                    bubbles: true,
                    composed: true,
                    detail: {
                        name: instance.getAttribute('name') ?? '',
                        valid: true,
                        value: event.type,
                    },
                }),
            );
        };
        activationListeners.set(instance, listener);
        instance.addEventListener('click', listener);
    },
    disconnected(instance) {
        const listener = activationListeners.get(instance);
        if (listener) instance.removeEventListener('click', listener);
        activationListeners.delete(instance);
    },
};

describe('component test harness', () => {
    let harness: SubstrateComponentHarness;

    afterEach(() => {
        harness?.cleanup();
    });

    it('proves action and field contracts through real CEM-ML substrate declarations', async () => {
        harness = createSubstrateComponentHarness();
        await harness.register({
            tag: ACTION_TAG,
            behavior: ACTION_BEHAVIOR,
            behaviorIdentity: 'cem-harness-action-behavior-v1',
            cemMl:
                '{attribute @name=label | Action}' +
                '{button @type=button @class=cem-harness-action @aria-label="{$label}" @style="display:inline-block;width:120px;height:32px;color:rgb(17, 24, 39);background-color:rgb(255, 255, 255);outline:3px solid rgb(37, 99, 235)" @slice=pressed @slice-event=click @slice-value="$event.type" | {$label}}',
        });
        await harness.register({
            tag: FIELD_TAG,
            cemMl:
                '{attribute @name=label | Field}' +
                '{label @class=cem-harness-field | {span | {$label}} {input @name="{$datadom.attributes.name}" @value={datadom.slices.value ?? datadom.attributes.value} @required={datadom.attributes.required} @slice=value @slice-event=input @slice-value="{$target.value}" | }}',
        });

        const form = await harness.render(`
            <form>
                <${ACTION_TAG} name="save" label="Save"></${ACTION_TAG}>
                <${FIELD_TAG} name="email" label="Email" value="draft@example.test" required></${FIELD_TAG}>
            </form>
        `) as HTMLFormElement;
        const actionHost = harness.query<HTMLElement>(ACTION_TAG);
        const fieldHost = harness.query<HTMLElement>(FIELD_TAG);
        await Promise.all([harness.settle(actionHost), harness.settle(fieldHost)]);

        const action = harness.query<HTMLButtonElement>(`${ACTION_TAG} button`);
        let input = harness.query<HTMLInputElement>(`${FIELD_TAG} input`);
        assertLightDomRendered(actionHost);
        assertLightDomRendered(fieldHost);
        expect(actionHost.shadowRoot).toBeNull();
        expect(fieldHost.shadowRoot).toBeNull();
        expect(assertAccessibleName(action, 'Save')).toBe('Save');
        expect(assertAccessibleName(input, 'Email')).toBe('Email');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();

        actionHost.setAttribute('label', 'Publish');
        await harness.settle(actionHost);
        expect(action.textContent?.trim()).toBe('Publish');

        const event = await expectComponentEvent<ChangeDetail>(
            actionHost,
            'cem-activate',
            () => action.click(),
            { detail: { name: 'save', valid: true, value: 'click' } },
        );
        await harness.settle(actionHost);
        expect(event.detail.value).toBe('click');
        expect(harness.snapshot(actionHost).slices.pressed).toBe('click');

        expect(captureFormSnapshot(form)).toEqual({
            entries: [['email', 'draft@example.test']],
            valid: true,
        });
        input.value = 'temporary@example.test';
        expect(captureFormSnapshot(form).entries).toEqual([['email', 'temporary@example.test']]);
        expect(await harness.resetForm(form, [fieldHost])).toEqual({
            entries: [['email', 'draft@example.test']],
            valid: true,
        });

        input = harness.query<HTMLInputElement>(`${FIELD_TAG} input`);
        input.value = '';
        expect(captureFormSnapshot(form).valid).toBe(false);
        input.value = 'published@example.test';
        input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
        await harness.settle(fieldHost);
        expect(harness.snapshot(fieldHost).slices.value).toBe('published@example.test');
        expect(captureFormSnapshot(form)).toEqual({
            entries: [['email', 'published@example.test']],
            valid: true,
        });

        await assertFocusVisible(action);
        expect(captureVisualSnapshot(action, [
            'background-color',
            'color',
            'display',
            'height',
            'width',
        ])).toMatchInlineSnapshot(`
          {
            "html": "<button type="button" class="cem-harness-action" aria-label="Publish" style="display:inline-block;width:120px;height:32px;color:rgb(17, 24, 39);background-color:rgb(255, 255, 255);outline:3px solid rgb(37, 99, 235)">Publish</button>",
            "rect": {
              "height": 32,
              "width": 120,
            },
            "styles": {
              "background-color": "rgb(255, 255, 255)",
              "color": "rgb(17, 24, 39)",
              "display": "inline-block",
              "height": "32px",
              "width": "120px",
            },
            "tagName": "button",
            "text": "Publish",
          }
        `);

        const screenshot = await page.screenshot({ element: form, save: false });
        expect(screenshot.length).toBeGreaterThan(100);
    });
});
