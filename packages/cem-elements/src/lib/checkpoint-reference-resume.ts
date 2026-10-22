import { checkpointReferenceSource, snapshotCheckpointReferences, type CheckpointSource } from './checkpoint-interactions.js';
import { equalCheckpointState } from './checkpoint-form-resume.js';
import type { CemElementReferenceInputs } from './element-reference-inputs.js';
import type { CemElementPlacementCoordinator } from './element-placement-coordinator.js';

function portable(inputs: CemElementReferenceInputs, revision: string) {
    const snapshot = snapshotCheckpointReferences(inputs);
    return snapshot && checkpointReferenceSource(snapshot, revision);
}
/** Native-validated bytes are intent. The original host callback and current
 * placement registry must independently authorize a fresh invocation. */
export async function prepareCemReferenceResume(host: HTMLElement, instanceId: string, expected: CheckpointSource,
    raw: CemElementReferenceInputs, validate: (source: CheckpointSource) => Promise<CheckpointSource>, coordinator?: CemElementPlacementCoordinator) {
    const original = portable(raw, expected.revision);
    if (!original) throw new Error('cem.checkpoint.resume.source');
    const admitted = coordinator?.prepare(host, instanceId, raw, undefined, true) ?? snapshotCheckpointReferences(raw);
    if (!admitted) throw new Error('cem.checkpoint.resume.source');
    const renewed = coordinator?.renew(admitted);
    const inputs = renewed?.inputs ?? admitted;
    const current = () => host.isConnected && (!renewed || renewed.current())
        && equalCheckpointState(original, portable(raw, expected.revision));
    try {
        const source = checkpointReferenceSource(inputs, expected.revision);
        if (!source || !current() || !equalCheckpointState(expected, await validate(source)) || !current())
            throw new Error('cem.checkpoint.resume.source-authority');
    } catch (error) { renewed?.discard(); throw error; }
    return { inputs, current, discard: () => renewed?.discard() };
}
