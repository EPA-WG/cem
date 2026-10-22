import type { CemCheckpointControlState, CemCheckpointRuntimeState } from './checkpoint-controls.js';
import { cemFormControlCheckpoint } from './form-control-capability.js';
import { cemCheckableControlCheckpoint } from './checkable-control.js';
import { cemChoiceSelectCheckpoint } from './choice-select-capability.js';
import { cemEditableChoiceCheckpoint } from './editable-choice-capability.js';

type Capability = NonNullable<CemCheckpointRuntimeState['capability']>;
export type CemResumableFormKind = 'form-control' | 'checkable-control' | 'choice-select' | 'editable-choice';

/** Private controller metadata only; provider relationships require separate renewed grants. */
export function cemRetainedFormCheckpoint(host: HTMLElement, kind: CemResumableFormKind) {
    switch (kind) {
        case 'form-control': return cemFormControlCheckpoint(host);
        case 'checkable-control': return cemCheckableControlCheckpoint(host);
        case 'choice-select': return cemChoiceSelectCheckpoint(host);
        case 'editable-choice': return cemEditableChoiceCheckpoint(host);
    }
}

// Compare named, native-validated scalar DTOs independently of serde key order.
function equal(a: unknown, b: unknown): boolean {
    if (Object.is(a, b)) return true;
    if (!a || !b || typeof a !== 'object' || typeof b !== 'object' || Array.isArray(a) !== Array.isArray(b)) return false;
    const entries = Object.entries(a).filter(([, value]) => value !== undefined);
    const other = Object.entries(b).filter(([, value]) => value !== undefined);
    return entries.length === other.length && entries.every(([key, value]) => Object.hasOwn(b, key) && equal(value, (b as Record<string, unknown>)[key]));
}

/** Retain live controls and private reset/consumption state; never replay checkpoint edits. */
export function prepareCemRetainedFormResume(host: HTMLElement, kind: CemResumableFormKind,
    saved: Capability | null | undefined, controls: CemCheckpointControlState,
    readControls: () => CemCheckpointControlState | undefined) {
    const ids = new Set(controls.controls.map(control => control.id));
    const nodes = Array.from(host.querySelectorAll<HTMLElement>('[data-cem-render-node-id]'))
        .filter(node => ids.has(node.getAttribute('data-cem-render-node-id')!));
    const forms = nodes.map(node => (node as HTMLInputElement).form);
    const disabled = nodes.map(node => node.matches(':disabled'));
    const form = (host as HTMLInputElement).form, parent = host.parentNode;
    const current = () => host.isConnected && saved?.kind === kind
        && host.parentNode === parent && (host as HTMLInputElement).form === form
        && nodes.length === ids.size && nodes.every((node, index) => host.contains(node) && node.isConnected
            && node.getAttribute('data-cem-render-node-id') === controls.controls[index]?.id
            && (node as HTMLInputElement).form === forms[index] && node.matches(':disabled') === disabled[index])
        && equal(saved, cemRetainedFormCheckpoint(host, kind)) && equal(controls, readControls());
    if (!current()) throw new Error('cem.checkpoint.resume.form-stale');
    return { current, activate() { if (!current()) throw new Error('cem.checkpoint.resume.form-stale'); } };
}
