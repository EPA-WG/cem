import type { CemElementReferenceInputs } from './element-reference-inputs.js';
import type { NativeCemValue } from './native-values.js';
import type { CemProcessingHost, CemProcessingNativeSessionInput, CemProcessingNativeSessionResult } from './internal/runtime-support/processing-host.js';
import { createCemNativeSuggestionsPublication, createCemNativeSuggestionSource, type CemNativeSuggestionSource, type CemNativeSuggestionsPublication } from './native-suggestions-publication.js';

/** Host-issued live authority. This is neither saved state nor a native value artifact. */
export interface CemNativeSessionHandle {
    sessionKey: string;
    instanceId: string;
    scopePolicyStamp: string;
    sourceRevision: string;
}
export type CemNativeSessionSources = Omit<CemElementReferenceInputs, 'kind' | 'placements'> & {
    kind: 'cem-native-session-sources-v1';
};
/** Explicit format import at the native boundary, with no implicit crossing grants. */
export interface CemNativeSessionImport {
    kind: 'cem-native-session-import-v1';
    bytes: ArrayBuffer;
    contentType: 'application/xml';
    sourceUri: string;
}
export interface CemNativeSessionView {
    handle: CemNativeSessionHandle;
    values: readonly NativeCemValue[];
}
/** Scalar control only. Sources, content and row identity stay in the native owner. */
export interface CemNativeSuggestionsConfig {
    query: string;
    queryRevision: number;
    filter?: 'contains' | 'prefix' | 'external' | 'none';
    filterBy?: string;
    expanded?: boolean;
    active?: number;
    committed?: number;
}
/** Retained source lease. Query freshness belongs to each immutable publication. */
export class CemNativeCapabilitySession {
    private released = false;
    private releasePromise?: Promise<void>;
    private ownerLost = false;
    private readonly stopOwnerLoss: () => void;
    private readonly publications = new Set<CemNativeSuggestionsPublication>();
    private readonly sourceExpiryListeners = new Set<() => void>();
    private readonly rowSources = new Map<string, CemNativeSuggestionSource>();
    private readonly ownerMode: CemProcessingHost['mode'];
    private readonly abort = () => { void this.release().catch(() => undefined); };
    private constructor(private readonly host: CemProcessingHost, readonly handle: CemNativeSessionHandle,
        readonly length: number, private readonly current: () => boolean, private readonly signal?: AbortSignal,
        readonly suggestions?: Extract<CemProcessingNativeSessionResult, { status: 'ready' }>['suggestions']) {
        Object.freeze(handle);
        this.ownerMode = host.mode;
        this.stopOwnerLoss = host.onNativeAuthorityLost?.(() => { this.ownerLost = true; void this.release().catch(() => undefined); }) ?? (() => undefined);
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
            return new CemNativeCapabilitySession(host, handle, result.length, () => !signal?.aborted && current(), signal, result.suggestions);
        } catch (error) {
            await host.nativeSession({ action: 'release', handle }).result.catch(() => undefined);
            throw error;
        } finally { signal?.removeEventListener('abort', cancel); }
    }
    get valid(): boolean { return !this.released && !this.host.ownerScope.disposed && this.host.mode === this.ownerMode && this.current(); }
    async view(expression: string, index?: number, suggestions?: CemNativeSuggestionsConfig): Promise<CemNativeSessionView> {
        const result = await this.run({ action: 'view', handle: this.handle, expression, index, suggestions });
        if (result.status !== 'view') throw new Error('Invalid native capability view reply');
        return { handle: this.handle, values: result.values };
    }
    async render(template: string, index?: number, suggestions?: CemNativeSuggestionsConfig, groupLabel?: boolean): Promise<Extract<CemProcessingNativeSessionResult, { status: 'rendered' }>> {
        const result = await this.run({ action: 'render', handle: this.handle, template, index, suggestions, groupLabel });
        if (result.status !== 'rendered') throw new Error('Invalid native label reply');
        return result;
    }
    release(): Promise<void> {
        if (this.releasePromise) return this.releasePromise;
        let resolve!: () => void, reject!: (error: unknown) => void;
        this.releasePromise = new Promise<void>((complete, failed) => { resolve = complete; reject = failed; });
        this.released = true; this.stopOwnerLoss();
        for (const expire of [...this.sourceExpiryListeners]) expire(); this.sourceExpiryListeners.clear();
        this.signal?.removeEventListener('abort', this.abort);
        void Promise.allSettled([...this.publications].map(publication => publication.release())).then(async () => {
            this.publications.clear(); this.rowSources.clear();
            await this.host.nativeSession({ action: 'release', handle: this.handle }).result.catch(error => { if (!this.ownerLost) throw error; });
        }).then(resolve, reject);
        return this.releasePromise;
    }
    /** Host-owned publication; consumers route to this session's original processing owner. */
    async publishSuggestions(config: CemNativeSuggestionsConfig, current: () => boolean): Promise<CemNativeSuggestionsPublication> {
        if (!this.suggestions || !current()) throw new Error('Native suggestions publication is unavailable');
        const publication = crypto.randomUUID(), captured = Object.freeze({ ...config });
        try {
            const result = await this.run({ action: 'publish-suggestions', handle: this.handle, publication, suggestions: captured });
            if (result.status !== 'published' || !current()) throw new Error('Native suggestions publication was superseded');
            const admitted = createCemNativeSuggestionsPublication(captured, () => this.valid && current(),
                input => this.run({ ...input, handle: this.handle }),
                () => { this.publications.delete(admitted); return this.host.nativeSession({ action: 'release-suggestions', handle: this.handle, publication }).result.then(() => undefined); }, publication,
                async frame => {
                    if (!this.valid || !current()) throw new Error('Native publication is no longer current');
                    const compiled = await this.host.compile(frame.compile).result;
                    if (!this.valid || !current()) throw new Error('Native publication compile was superseded');
                    if (compiled.diagnostics.some(diagnostic => diagnostic.severity === 'error' || diagnostic.severity === 'fatal')) {
                        throw new Error(compiled.diagnostics.map(diagnostic => diagnostic.message).join('; '));
                    }
                    const result = await this.host.renderDiff({ ...frame.render, artifact: compiled.artifact,
                        nativeSuggestions: { handle: this.handle, publication } }).result;
                    if (!this.valid || !current()) throw new Error('Native publication render was superseded');
                    return result;
                }, handle => {
                    let source = this.rowSources.get(handle);
                    if (!source) {
                        source = createCemNativeSuggestionSource(() => this.valid, expire => { this.sourceExpiryListeners.add(expire); return () => { this.sourceExpiryListeners.delete(expire); }; });
                        this.rowSources.set(handle, source);
                    }
                    return source;
                });
            this.publications.add(admitted);
            return admitted;
        } catch (error) {
            await this.host.nativeSession({ action: 'release-suggestions', handle: this.handle, publication }).result.catch(() => undefined);
            throw error;
        }
    }
    private async run(input: CemProcessingNativeSessionInput): Promise<CemProcessingNativeSessionResult> {
        if (!this.valid) {
            await this.release().catch(() => undefined);
            throw new Error(this.ownerLost || this.host.mode !== this.ownerMode ? 'Native session worker owner was lost; prepare a fresh session' : 'Native capability session is no longer current');
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
