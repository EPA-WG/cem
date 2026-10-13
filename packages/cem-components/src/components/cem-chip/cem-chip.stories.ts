import { expect } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, cemStoryRuntime, loadCemComponent, loadCemWorkflowDependencies, assertAccessibleName, assertAriaReferenceIntegrity, assertFocusVisible, nextRenderFrame, type DataIslandSnapshot } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-chip.xhtml?raw';
import '../../styles.css';
const meta = preview.meta({ component: 'cem-chip', title: 'CEM Components/cem-chip', loaders: [async () => { await loadCemDeclaration('cem-chip', declarationSource); await loadCemComponent('cem-list');  await loadCemWorkflowDependencies(); return {}; }] });

export const CheckableAndPassive = meta.story({
 name: "toggles checkable content chips without changing passive chip semantics",
 render: () => "\n            <cem-stack gap=\"sm\">\n                <cem-chip label=\"Static topic\">Static topic</cem-chip>\n                <cem-chip label=\"Owner filter\" checkable>Owner</cem-chip>\n                <cem-chip label=\"Ready filter\" checkable checked>Ready</cem-chip>\n            </cem-stack>\n        ",
 parameters: { docs: { description: { story: 'Browser-runner-only: trusted pointer, focus and native keyboard assertions.' } } },
 play: async ({ canvasElement }) => {
  if (import.meta.env.MODE !== 'test') return;
        const root = canvasElement;
        const harness = { root, query: <T extends Element = Element>(selector: string): T => { const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node; } };
        const runtime = cemStoryRuntime();
        for (const wrapper of root.querySelectorAll<HTMLElement>('cem-stack, cem-table')) await whenCemRendered(wrapper);
        for (const host of root.querySelectorAll<HTMLElement>('cem-nav, cem-tabs, cem-list, cem-chip')) await whenCemRendered(host);
        await nextRenderFrame();
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
 },
});
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
