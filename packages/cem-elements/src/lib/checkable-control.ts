import type { CemProducedElementBehavior } from './cem-elements.js';

interface CheckableState { control: HTMLInputElement; indeterminate: boolean }
const states = new WeakMap<HTMLElement, CheckableState>();

/** Native checked/defaultChecked, radio grouping, events and forms remain browser-owned. */
export const CEM_CHECKABLE_CONTROL_CAPABILITY: CemProducedElementBehavior = {
    rendered(instance) {
        const controls = [...instance.querySelectorAll<HTMLInputElement>('input[part~="control"]')].filter(control => {
            for (let parent = control.parentElement; parent && parent !== instance; parent = parent.parentElement) {
                if (parent.localName.includes('-')) return false;
            }
            return true;
        });
        if (controls.length !== 1) return;
        const control = controls[0];
        if (control.type !== 'checkbox' || control.getAttribute('role') === 'switch') return;
        const previous = states.get(instance), indeterminate = instance.hasAttribute('indeterminate');
        // Reapply only a changed authored presence or a new native owner. Activation
        // clears mixed state natively and unrelated rerenders must preserve that edit.
        if (previous?.control !== control || previous.indeterminate !== indeterminate) {
            control.indeterminate = indeterminate;
        }
        states.set(instance, { control, indeterminate });
    },
};
