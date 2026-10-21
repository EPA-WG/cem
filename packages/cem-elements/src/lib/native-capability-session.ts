import { checkpointPreparation, checkpointBase64, type CheckpointSource } from './checkpoint-interactions.js';
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
export type CemNativePublicationConfig = CemNativeSuggestionsConfig | { profile: 'native-datalist'; query?: never; queryRevision?: never; filter?: never };
export interface CemNativeSuggestionsConfig {
    profile?: never;
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
    private checkpointInput?: Extract<CemProcessingNativeSessionInput, { action: 'prepare' }>;
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
        current: () => boolean, signal?: AbortSignal, checkpoint = false): Promise<CemNativeCapabilitySession> {
        const handle = Object.freeze({ ...input.handle });
        let checkpointInput: Extract<CemProcessingNativeSessionInput, { action: 'prepare' }> | undefined;
        // Copy before submission, so async caller aliases cannot change admitted provenance.
        const sourceInputs = input.sources.kind === 'cem-native-session-import-v1' ? [input.sources.bytes] : input.sources.sources.map(source => source.bundle);
        const metadataBounded = input.sources.kind === 'cem-native-session-import-v1' ? input.sources.sourceUri.length <= 4096
            : input.sources.bindings.length <= 256 && input.sources.grants.length <= 256
                && input.sources.bindings.every(binding => binding.name.length <= 1024 && binding.select.length <= 4096);
        const preparation = checkpoint ? checkpointPreparation(input) : undefined;
        const preparationBytes = preparation ? new TextEncoder().encode(preparation.dataJson).byteLength
            + preparation.nativeBindings.artifacts.reduce((sum, artifact) => sum + atob(artifact.base64).length, 0) : 0;
        if (checkpoint && metadataBounded && input.select.length <= 4096 && handle.sourceRevision.length <= 1024
            && Object.values(input.labelTemplates ?? {}).every(value => value.length <= 4096) && sourceInputs.length <= 64 && sourceInputs.reduce((sum, bytes) => sum + bytes.byteLength, preparationBytes) <= 256 * 1024
            && preparation) checkpointInput = structuredClone(input);
        const job = host.nativeSession({ ...(checkpointInput ? structuredClone(checkpointInput) : input), handle });
        const cancel = () => { void host.cancel({ targetJobId: job.jobId, reason: 'superseded' }).result.catch(() => undefined); };
        signal?.addEventListener('abort', cancel, { once: true });
        try {
            if (signal?.aborted) cancel();
            const result = await job.result;
            if (result.status !== 'ready' || !current() || signal?.aborted) {
                await host.nativeSession({ action: 'release', handle }).result;
                throw new Error('Native capability preparation was superseded');
            }
            const session = new CemNativeCapabilitySession(host, handle, result.length, () => !signal?.aborted && current(), signal, result.suggestions);
            session.checkpointInput = checkpointInput;
            return session;
        } catch (error) {
            await host.nativeSession({ action: 'release', handle }).result.catch(() => undefined);
            throw error;
        } finally { signal?.removeEventListener('abort', cancel); }
    }
    checkpointSource(): CheckpointSource | undefined {
        const input = this.checkpointInput;
        if (!this.valid || !input || !this.suggestions || this.suggestions.rows > 1024) return undefined;
        const imported = input.sources.kind === 'cem-native-session-import-v1';
        const original = input.sources;
        const sources: CheckpointSource['sources'] = [];
        let bytes = 0;
        for (const source of imported ? [original as CemNativeSessionImport] : (original as CemNativeSessionSources).sources) {
            const buffer = 'bytes' in source ? source.bytes : source.bundle;
            bytes += buffer.byteLength; if (bytes > 256 * 1024) return undefined;
            const base64 = checkpointBase64(buffer); if (base64 === undefined) return undefined;
            sources.push({ codec: 'bytes' in source ? 'xml' : 'cemb', base64, sourceUri: 'bytes' in source ? source.sourceUri : null,
                primarySourceId: 'bytes' in source ? 1 : source.primarySourceId, context: 'bytes' in source ? true : source.context });
            // Non-default source traversal contracts require their own portable limits.
            if ('bundle' in source && (source.maxDepth !== undefined || source.maxWork !== undefined || source.unresolved !== undefined)) return undefined;
        }
        const preparation = checkpointPreparation(input); if (!preparation) return undefined;
        return { preparation, version: 'cem-native-source/1', revision: this.handle.sourceRevision, rowCount: this.suggestions.rows, select: input.select,
            resolve: input.resolve ?? true, requesting: imported ? 0 : (original as CemNativeSessionSources).requesting, sources,
            bindings: imported ? [] : (original as CemNativeSessionSources).bindings.map(binding => ({ ...binding })),
            crossings: imported ? [] : (original as CemNativeSessionSources).grants.map(([from, to]) => ({ from, to })),
            labels: { option: input.labelTemplates?.option ?? null, group: input.labelTemplates?.group ?? null }, disposition: 'reacquire' };
    }
    /** Original source intent for an inactive host-admitted consumer, never old rows or leases. */
    checkpointSourceIntent(): CheckpointSource | undefined {
        const source = this.checkpointSource();
        return source ? { ...source, rowCount: 0 } : undefined;
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
        this.released = true; this.checkpointInput = undefined; this.stopOwnerLoss();
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
        return this.publish(config, current);
    }
    async publishDatalist(controls: Record<string, never>, current: () => boolean): Promise<CemNativeSuggestionsPublication> {
        if (this.suggestions?.identity !== 'cem-native-datalist-v1') throw new Error('Native datalist adapter is required');
        return this.publish({ profile: 'native-datalist' }, current, structuredClone(controls));
    }
    private async publish(config: CemNativePublicationConfig, current: () => boolean, controls?: Record<string, never>): Promise<CemNativeSuggestionsPublication> {
        if (!this.suggestions || !current()) throw new Error('Native suggestions publication is unavailable');
        const publication = crypto.randomUUID(), captured = Object.freeze({ ...config });
        try {
            const result = await this.run(captured.profile === 'native-datalist'
                ? { action: 'publish-datalist', handle: this.handle, publication, controls: controls ?? {} }
                : { action: 'publish-suggestions', handle: this.handle, publication, suggestions: captured });
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
                }, (handle, index) => {
                    let source = this.rowSources.get(handle);
                    if (!source) {
                        source = createCemNativeSuggestionSource(() => this.valid, expire => { this.sourceExpiryListeners.add(expire); return () => { this.sourceExpiryListeners.delete(expire); }; }, index, () => this.checkpointSource());
                        this.rowSources.set(handle, source);
                    }
                    return source;
                }, () => this.checkpointSource());
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
