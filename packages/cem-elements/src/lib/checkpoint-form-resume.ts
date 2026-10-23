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
export function equalCheckpointState(a: unknown, b: unknown): boolean {
    if (Object.is(a, b)) return true;
    if (!a || !b || typeof a !== 'object' || typeof b !== 'object' || Array.isArray(a) !== Array.isArray(b)) return false;
    const entries = Object.entries(a).filter(([, value]) => value !== undefined);
    const other = Object.entries(b).filter(([, value]) => value !== undefined);
    return entries.length === other.length && entries.every(([key, value]) => Object.hasOwn(b, key) && equalCheckpointState(value, (b as Record<string, unknown>)[key]));
}

/** Retain live controls and private reset/consumption state; never replay checkpoint edits. */
export function prepareCemRetainedFormResume(host: HTMLElement, kind: CemResumableFormKind,
    saved: Capability | null | undefined, controls: CemCheckpointControlState,
    readControls: () => CemCheckpointControlState | undefined, owns: (node: HTMLElement) => boolean) {
    return prepareCemRetainedWidgetResume(host, saved, controls, readControls,
        () => cemRetainedFormCheckpoint(host, kind), owns, 'form-stale');
}

/** Browser-owned state survives native retirement, but never supplies fresh native authority. */
export function prepareCemRetainedWidgetResume(host: HTMLElement, saved: Capability | null | undefined,
    controls: CemCheckpointControlState, readControls: () => CemCheckpointControlState | undefined,
    readCapability: () => Capability | null | undefined, owns: (node: HTMLElement) => boolean, error = 'widget-stale') {
    const ids = new Set(controls.controls.map(control => control.id));
    const nodes = Array.from(host.querySelectorAll<HTMLElement>('[data-cem-render-node-id]'))
        .filter(node => owns(node) && ids.has(node.getAttribute('data-cem-render-node-id')!));
    const forms = nodes.map(node => (node as HTMLInputElement).form);
    const disabled = nodes.map(node => node.matches(':disabled'));
    const form = (host as HTMLInputElement).form, parent = host.parentNode;
    const current = () => host.isConnected && !!saved
        && host.parentNode === parent && (host as HTMLInputElement).form === form
        && nodes.length === ids.size && nodes.every((node, index) => owns(node) && host.contains(node) && node.isConnected
            && node.getAttribute('data-cem-render-node-id') === controls.controls[index]?.id
            && (node as HTMLInputElement).form === forms[index] && node.matches(':disabled') === disabled[index])
        && equalCheckpointState(saved, readCapability()) && equalCheckpointState(controls, readControls());
    if (!current()) throw new Error(`cem.checkpoint.resume.${error}`);
    return { current, activate() { if (!current()) throw new Error(`cem.checkpoint.resume.${error}`); } };
}
