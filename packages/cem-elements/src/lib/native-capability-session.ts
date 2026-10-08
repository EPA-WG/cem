import type { CemElementReferenceInputs } from './element-reference-inputs.js';
import type { NativeCemValue } from './native-values.js';
import type { CemProcessingHost, CemProcessingNativeSessionInput, CemProcessingNativeSessionResult } from './internal/runtime-support/processing-host.js';

/** Host-issued live authority. This is neither saved state nor a native value artifact. */
export interface CemNativeSessionHandle {
    sessionKey: string;
    instanceId: string;
    scopePolicyStamp: string;
    sourceRevision: string;
    queryRevision: number;
}
export type CemNativeSessionSources = Omit<CemElementReferenceInputs, 'kind' | 'placements'> & {
    kind: 'cem-native-session-sources-v1';
};
export interface CemNativeSessionView {
    handle: CemNativeSessionHandle;
    values: readonly NativeCemValue[];
}
/** Immutable revision lease. Consumers supply current attachment/query eligibility. */
export class CemNativeCapabilitySession {
    private released = false;
    private readonly ownerMode: CemProcessingHost['mode'];
    private readonly abort = () => { void this.release().catch(() => undefined); };
    private constructor(private readonly host: CemProcessingHost, readonly handle: CemNativeSessionHandle,
        readonly length: number, private readonly current: () => boolean, private readonly signal?: AbortSignal) {
        Object.freeze(handle);
        this.ownerMode = host.mode;
        signal?.addEventListener('abort', this.abort, { once: true });
    }
    static async prepare(host: CemProcessingHost, input: Extract<CemProcessingNativeSessionInput, { action: 'prepare' }>,
        current: () => boolean, signal?: AbortSignal): Promise<CemNativeCapabilitySession> {
        const handle = Object.freeze({ ...input.handle });
        const job = host.nativeSession({ ...input, handle });
        const cancel = () => { void host.cancel({ targetJobId: job.jobId, reason: 'superseded' }).result.catch(() => undefined); };
        signal?.addEventListener('abort', cancel, { once: true });
        try {
            if (signal?.aborted) cancel();
            const result = await job.result;
            if (result.status !== 'ready' || !current() || signal?.aborted) {
                await host.nativeSession({ action: 'release', handle }).result;
                throw new Error('Native capability preparation was superseded');
            }
            return new CemNativeCapabilitySession(host, handle, result.length, () => !signal?.aborted && current(), signal);
        } catch (error) {
            await host.nativeSession({ action: 'release', handle }).result.catch(() => undefined);
            throw error;
        } finally { signal?.removeEventListener('abort', cancel); }
    }
    get valid(): boolean { return !this.released && !this.host.ownerScope.disposed && this.host.mode === this.ownerMode && this.current(); }
    async view(expression: string, index?: number): Promise<CemNativeSessionView> {
        const result = await this.run({ action: 'view', handle: this.handle, expression, index });
        if (result.status !== 'view') throw new Error('Invalid native capability view reply');
        return { handle: this.handle, values: result.values };
    }
    async render(template: string, index?: number): Promise<Extract<CemProcessingNativeSessionResult, { status: 'rendered' }>> {
        const result = await this.run({ action: 'render', handle: this.handle, template, index });
        if (result.status !== 'rendered') throw new Error('Invalid native label reply');
        return result;
    }
    async release(): Promise<void> {
        if (this.released) return;
        this.released = true;
        this.signal?.removeEventListener('abort', this.abort);
        await this.host.nativeSession({ action: 'release', handle: this.handle }).result;
    }
    private async run(input: CemProcessingNativeSessionInput): Promise<CemProcessingNativeSessionResult> {
        if (!this.valid) {
            await this.release().catch(() => undefined);
            throw new Error(this.host.mode !== this.ownerMode ? 'Native session worker owner was lost; prepare a fresh session' : 'Native capability session is no longer current');
        }
        let result: CemProcessingNativeSessionResult;
        try { result = await this.host.nativeSession(input).result; }
        catch (error) {
            if (this.host.mode !== this.ownerMode || this.host.ownerScope.disposed) await this.release().catch(() => undefined);
            throw error;
        }
        if (!this.valid) { await this.release(); throw new Error('Native capability result was superseded'); }
        return result;
    }
}
