import { expect } from 'storybook/test';
import type { CemProducedElementBehavior } from '../index.js';
import preview, { createCemStoryRuntime, nextRenderFrame, assertAccessibleName, assertAriaReferenceIntegrity, assertFocusVisible, assertLightDomRendered, captureFormSnapshot, captureVisualSnapshot, expectComponentEvent } from '../../.storybook/preview.js';
const meta = preview.meta({ title: 'Runtime/Native substrate contracts' });
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


export const AccessibleNames = meta.story({ render: () => `<section>
            <label>Email <span aria-hidden="true">*</span><span hidden>Hidden</span><input aria-describedby="harness-help"></label>
            <p id="harness-help">Enter an address.</p>
            <span id="harness-explicit" hidden>Referenced name</span>
            <input aria-label="Lower priority" aria-labelledby="harness-explicit">
        </section>`, play: async ({ canvasElement }) => {
        const [email, referenced] = canvasElement.querySelectorAll('input');
        expect(assertAccessibleName(email, 'Email')).toBe('Email');
        expect(assertAccessibleName(referenced, 'Referenced name')).toBe('Referenced name');
        email.closest('label')?.querySelector('[aria-hidden]')?.removeAttribute('aria-hidden');
        expect(assertAccessibleName(email, 'Email *')).toBe('Email *');
} });

export const NativeActionAndField = meta.story({
    render: () => '<div></div>',
    parameters: { docs: { description: { story: 'Browser-runner-only screenshot; behavior callback is a runtime extension fixture.' } } },
    play: async ({ canvasElement }) => {
        const { runtime, scope } = createCemStoryRuntime(canvasElement, false);
        const register = async ({ tag, cemMl, behavior, behaviorIdentity }: { tag: string; cemMl: string; behavior?: CemProducedElementBehavior; behaviorIdentity?: string }) => {
            const declaration = document.createElement('div'); declaration.setAttribute('tag', tag);
            const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = cemMl; declaration.append(template);
            expect(runtime.registerDeclaration(declaration, { behavior, behaviorIdentity })).toBe(true);
            await runtime.whenDeclarationSettled(declaration); expect(runtime.diagnosticsFor(declaration)).toEqual([]);
        };
        try {
        await register({
            tag: ACTION_TAG,
            behavior: ACTION_BEHAVIOR,
            behaviorIdentity: 'cem-harness-action-behavior-v1',
            cemMl:
                '{attribute @name=label | Action}' +
                '{button @type=button @class=cem-harness-action @aria-label="{$label}" @style="display:inline-block;width:120px;height:32px;color:rgb(17, 24, 39);background-color:rgb(255, 255, 255);outline:3px solid rgb(37, 99, 235)" @slice=pressed @slice-event=click @slice-value="$event.type" | {$label}}',
        });
        await register({
            tag: FIELD_TAG,
            cemMl:
                '{attribute @name=label | Field}' +
                '{label @class=cem-harness-field | {span | {$label}} {input @name="{$datadom.attributes.name}" @value={datadom.slices.value ?? datadom.attributes.value} @required={if seq:count(datadom.attributes.required) > 0 { true } else { null }} @slice=value @slice-event=input @slice-value="{$target.value}" | }}',
        });

        canvasElement.innerHTML = `
            <form>
                <${ACTION_TAG} name="save" label="Save"></${ACTION_TAG}>
                <${FIELD_TAG} name="email" label="Email" value="draft@example.test" required></${FIELD_TAG}>
            </form>
        `;
        const form = substrateQuery<HTMLFormElement>(canvasElement, "form");
        await nextRenderFrame();
        const actionHost = substrateQuery<HTMLElement>(canvasElement, ACTION_TAG);
        const fieldHost = substrateQuery<HTMLElement>(canvasElement, FIELD_TAG);
        await Promise.all([runtime.whenRenderSettled(actionHost), runtime.whenRenderSettled(fieldHost)]);

        const action = substrateQuery<HTMLButtonElement>(canvasElement, `${ACTION_TAG} button`);
        let input = substrateQuery<HTMLInputElement>(canvasElement, `${FIELD_TAG} input`);
        assertLightDomRendered(actionHost);
        assertLightDomRendered(fieldHost);
        expect(actionHost.shadowRoot).toBeNull();
        expect(fieldHost.shadowRoot).toBeNull();
        expect(assertAccessibleName(action, 'Save')).toBe('Save');
        expect(assertAccessibleName(input, 'Email')).toBe('Email');
        expect(input.required).toBe(true);
        expect(() => assertAriaReferenceIntegrity(canvasElement)).not.toThrow();

        actionHost.setAttribute('label', 'Publish');
        await runtime.whenRenderSettled(actionHost);
        expect(action.textContent?.trim()).toBe('Publish');

        const event = await expectComponentEvent<ChangeDetail>(
            actionHost,
            'cem-activate',
            () => action.click(),
            { detail: { name: 'save', valid: true, value: 'click' } },
        );
        await runtime.whenRenderSettled(actionHost);
        expect(event.detail.value).toBe('click');
        expect(runtime.snapshotInstance(actionHost).slices.pressed).toBe('click');

        expect(captureFormSnapshot(form)).toEqual({
            entries: [['email', 'draft@example.test']],
            valid: true,
        });
        input.value = 'temporary@example.test';
        expect(captureFormSnapshot(form).entries).toEqual([['email', 'temporary@example.test']]);
        form.reset(); await runtime.whenRenderSettled(fieldHost);
        expect(captureFormSnapshot(form)).toEqual({
            entries: [['email', 'draft@example.test']],
            valid: true,
        });

        input = substrateQuery<HTMLInputElement>(canvasElement, `${FIELD_TAG} input`);
        input.value = '';
        expect(captureFormSnapshot(form).valid).toBe(false);

        // Native boolean attributes are enabled by presence, including "false".
        // Exercise live removal/reintroduction without losing the native owner.
        for (const required of [null, '', 'false', 'true', null, '']) {
            if (required === null) fieldHost.removeAttribute('required');
            else fieldHost.setAttribute('required', required);
            await runtime.whenRenderSettled(fieldHost);
            expect(substrateQuery<HTMLInputElement>(canvasElement, `${FIELD_TAG} input`)).toBe(input);
            expect(input.required).toBe(required !== null);
            input.value = '';
            expect(input.validity.valueMissing).toBe(required !== null);
            expect(captureFormSnapshot(form).valid).toBe(required === null);
        }

        input.value = 'published@example.test';
        input.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
        await runtime.whenRenderSettled(fieldHost);
        expect(runtime.snapshotInstance(fieldHost).slices.value).toBe('published@example.test');
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
        ])).toEqual(
          {
            "html": '<button type="button" class="cem-harness-action" aria-label="Publish" style="display:inline-block;width:120px;height:32px;color:rgb(17, 24, 39);background-color:rgb(255, 255, 255);outline:3px solid rgb(37, 99, 235)">Publish</button>',
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
        );

        if (import.meta.env.MODE === 'test') {
            const { page } = await import('vitest/browser');
            const screenshot = await page.screenshot({ element: form, save: false });
            expect(screenshot.length).toBeGreaterThan(100);
        }
        } finally { canvasElement.replaceChildren(); scope.dispose(); }
    },
});
function substrateQuery<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Expected substrate fixture to contain ${selector}`);
    return element;
}
