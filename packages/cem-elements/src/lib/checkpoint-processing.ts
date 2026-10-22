import type { CemCheckpointResumeContext, CemPreparedCheckpointOwner } from './checkpoint-resume.js';
import type { CemCheckpointControlState, CemCheckpointRuntimeState } from './checkpoint-controls.js';
import type { CemDeclarationScope } from './declaration-scope.js';
import { createFreshCemProcessingHost } from './internal/runtime-support/processing-host-runtime.js';
import { nativeCheckpointIdentity } from './internal/runtime-support/checkpoint-domain.js';
import { consumeLocationWriteTrigger } from './internal/runtime-support/location-write-trigger.js';
import type { CemProcessingCompileInput, CemProcessingDocumentHandle, CemProcessingHost } from './internal/runtime-support/processing-host.js';

/** Host event sequence is authoritative; resource refreshes/renders are not input. */
export class CemCheckpointWriteSuppression {
    private readonly consumed = new Map<number, string>();
    private sequence: number;
    constructor(private readonly captured: number, writers: CemCheckpointRuntimeState['writers']) {
        this.assertSequence(captured); this.sequence = captured;
        for (const writer of writers) if (writer.trigger !== null) this.consumed.set(writer.index, writer.trigger);
    }
    consume(sequence: number, writer?: number, trigger?: string): boolean {
        this.assertSequence(sequence);
        if (sequence < this.sequence) throw new Error('cem.checkpoint.resume.input-regression');
        if (writer !== undefined && (!Number.isSafeInteger(writer) || writer < 0 || writer >= 256)) throw new Error('cem.checkpoint.resume.writer');
        if (trigger !== undefined && (writer === undefined || new TextEncoder().encode(trigger).byteLength > 1024
            || Array.from(trigger).some(char => { const code = char.charCodeAt(0); return code < 32 || code >= 127 && code <= 159; })))
            throw new Error('cem.checkpoint.resume.writer');
        this.sequence = sequence;
        // Covers even a newly exposed/conditional writer and implicit storage writes.
        if (sequence <= this.captured) return false;
        if (writer === undefined) return true;
        return consumeLocationWriteTrigger(this.consumed, writer, trigger);
    }
    private assertSequence(sequence: number): void {
        if (!Number.isSafeInteger(sequence) || sequence < 0) throw new Error('cem.checkpoint.resume.input');
    }
}

export interface CemCheckpointProcessingOptions {
    scope: CemDeclarationScope;
    sourceUri: string;
    scopePolicyStamp: string;
    /** Explicit named control DTOs extracted by the trusted semantic island adapter. */
    controls: string;
    runtime: string;
    /** Trusted DOM adapter serialization of the same sealed canonical island.
     * Namespace-qualified island parts/HTML void payload require XMLSerializer;
     * the original context island remains the transaction integrity authority. */
    islandXml?: string;
    /** Complete immutable authoring inputs; never a retained native handle. */
    compilations?: readonly CemProcessingCompileInput[];
    acceptedEventSequence(): number;
    /** Current host policy must explicitly admit each original external source.
     * Called while inactive; it must not fetch, subscribe or perform a write. */
    allowSource(resource: Readonly<CemCheckpointRuntimeState['resources'][number]>, signal: AbortSignal): boolean | Promise<boolean>;
}
export interface CemResumedProcessingOwner {
    /** Unavailable until the complete resume transaction admits every peer. */
    readonly host: CemProcessingHost;
    readonly sources: readonly { name: string; handle: CemProcessingDocumentHandle }[];
    readonly runtimeState: CemCheckpointRuntimeState;
    /** Call before starting any external write/journal operation. */
    consumeWrite(writer?: number, trigger?: string): boolean;
}

const preparedBindings = new WeakMap<CemResumedProcessingOwner, { host: CemProcessingHost; state: CemCheckpointRuntimeState; controls: CemCheckpointControlState; sources: readonly { name: string; handle: CemProcessingDocumentHandle }[] }>();
/** @internal Reversible native preparation for built-in semantic adapters only. */
export function preparedCheckpointProcessing(owner: CemResumedProcessingOwner) {
    const prepared = preparedBindings.get(owner);
    if (!prepared) throw new Error('cem.checkpoint.resume.binding');
    return { host: prepared.host, state: structuredClone(prepared.state), controls: structuredClone(prepared.controls), sources: structuredClone(prepared.sources) };
}

/** Shared processing/source preparation for trusted semantic owner adapters.
 * This prepares native state only; it never installs widgets, mutates live DOM,
 * subscribes sources, evaluates a template or replays an external action. */
export async function prepareCemCheckpointProcessing(context: CemCheckpointResumeContext,
    options: CemCheckpointProcessingOptions): Promise<CemPreparedCheckpointOwner & { owner: CemResumedProcessingOwner }> {
    if (context.signal.aborted) throw new Error('cem.checkpoint.resume.cancelled');
    if ((options.compilations?.length ?? 0) > 64) throw new Error('cem.checkpoint.resume.compilations');
    options = { ...options, acceptedEventSequence: options.acceptedEventSequence.bind(options), allowSource: options.allowSource.bind(options),
        compilations: (options.compilations ?? []).map(input => structuredClone(input)) };
    const host = createFreshCemProcessingHost(options.scope, context.runtime);
    const sources: { name: string; handle: CemProcessingDocumentHandle }[] = [];
    let disposed = false;
    const current = () => {
        const sequence = options.acceptedEventSequence();
        return !disposed && !context.signal.aborted && !options.scope.disposed && !!host.checkpointClient
            && Number.isSafeInteger(sequence) && (sequence === context.receipt.acceptedEventSequence
                || sequence > context.receipt.acceptedEventSequence && context.active())
            && nativeCheckpointIdentity(host.checkpointClient) === context.runtimeId;
    };
    const assert = () => { if (!current()) throw new Error('cem.checkpoint.resume.stale'); };
    const admitted = () => { assert(); if (!context.active()) throw new Error('cem.checkpoint.resume.inactive'); };
    // Every owner on the root shares one fresh host. Release only this owner's
    // source imports on rollback; physical-generation cleanup owns host disposal.
    const discard = () => {
        if (disposed) return; disposed = true;
        for (const source of sources) void host.document({ action: 'release', handle: source.handle }).result.catch(() => undefined);
        sources.length = 0;
    };
    const abort = () => discard(); context.signal.addEventListener('abort', abort, { once: true });
    try {
        if (!host.checkpoint) throw new Error('cem.checkpoint.resume.processing');
        const result = await host.checkpoint({ controls: options.controls, runtime: options.runtime, island: options.islandXml ?? context.island,
            ...(options.islandXml === undefined ? {} : { islandContentType: 'application/xml' as const }),
            sourceUri: options.sourceUri, scopePolicyStamp: options.scopePolicyStamp }).result;
        assert();
        // Decode only the native-validated named lifecycle DTO, never a source AST.
        const state = JSON.parse(result.runtime) as CemCheckpointRuntimeState;
        const controls = JSON.parse(options.controls) as CemCheckpointControlState;
        const writes = new CemCheckpointWriteSuppression(context.receipt.acceptedEventSequence, state.writers);
        for (const resource of state.resources) {
            if (!await options.allowSource(structuredClone(resource), context.signal)) throw new Error('cem.checkpoint.resume.source-policy');
            assert();
            if (!resource.document) continue;
            const document = resource.document;
            const bytes = Uint8Array.from(atob(document.base64), c => c.charCodeAt(0)).buffer;
            const handle: CemProcessingDocumentHandle = { documentKey: crypto.randomUUID(),
                instanceId: context.receipt.instanceId, scopePolicyStamp: options.scopePolicyStamp };
            sources.push({ name: resource.name, handle });
            await host.document({ action: 'retain', handle, bytes, contentType: document.contentType, sourceUri: document.sourceUri }).result;
            assert();
        }
        for (const compilation of options.compilations ?? []) {
            if (compilation.scopePolicyStamp !== options.scopePolicyStamp) throw new Error('cem.checkpoint.resume.scope');
            await host.compile(structuredClone(compilation)).result; assert();
        }
        const owner: CemResumedProcessingOwner = Object.freeze({
            get host() { admitted(); return host; },
            get sources() { admitted(); return structuredClone(sources); },
            get runtimeState() { admitted(); return structuredClone(state); },
            consumeWrite: (writer?: number, trigger?: string) => { admitted(); return writes.consume(options.acceptedEventSequence(), writer, trigger); },
        });
        preparedBindings.set(owner, { host, state, controls, sources });
        return { owner, current, discard: () => { preparedBindings.delete(owner); context.signal.removeEventListener('abort', abort); discard(); } };
    } catch (error) { context.signal.removeEventListener('abort', abort); discard(); throw error; }
}
