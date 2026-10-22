import { checkpointProcessingControl } from './internal/runtime-support/processing-host-runtime.js';
import { equalCheckpointState } from './checkpoint-form-resume.js';
import type { CheckpointSource } from './checkpoint-interactions.js';
import type { CemCheckpointRuntimeState } from './checkpoint-controls.js';
import type { CemProcessingHost } from './internal/runtime-support/processing-host.js';

/** Private original host adapter. Intent remains inert until every peer is bound. */
export interface CemProviderHandoff {
    readonly editor: HTMLElement;
    readonly presentation?: HTMLElement;
    current(): boolean;
    park(): () => void;
    renew(host: CemProcessingHost, saved: CemCheckpointRuntimeState): Promise<void>;
}
const factories = new WeakMap<HTMLElement, () => CemProviderHandoff | undefined>();
export function registerCemProviderHandoff(host: HTMLElement, factory: () => CemProviderHandoff | undefined): () => void {
    if (factories.has(host)) throw Error('cem.checkpoint.resume.provider-conflict');
    factories.set(host, factory);
    return () => { if (factories.get(host) === factory) factories.delete(host); };
}
export function prepareCemProviderHandoff(host: HTMLElement): CemProviderHandoff | undefined { return factories.get(host)?.(); }

/** Compare fresh native-validated capsules, including original bytes and policies. */
export async function validateCemProviderSource(host: CemProcessingHost, actual: CheckpointSource | undefined, expected: CheckpointSource) {
    if (!actual) throw Error('cem.checkpoint.resume.source');
    const state = JSON.parse(await checkpointProcessingControl(host, { action: 'runtime', text: JSON.stringify({
        version: 'cem-checkpoint-runtime/1', capability: null, references: actual, resources: [], writers: [], writeDisposition: 'suppress-until-new-input',
    }) })) as CemCheckpointRuntimeState;
    if (!equalCheckpointState(state.references, expected)) throw Error('cem.checkpoint.resume.source');
}
