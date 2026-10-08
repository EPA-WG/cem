import type { CemNativePublicationConfig } from './native-capability-session.js';
import type { CemProcessingCompileInput, CemProcessingRenderDiffInput, CemProcessingRenderDiffResult, CemProcessingNativeSessionInput, CemProcessingNativeSessionResult } from './internal/runtime-support/processing-host.js';
import { assertProcessingBoundaryValue } from './projection.js';

/** Supplied by the host that admitted this consuming instance and captured revision. */
export interface CemNativeSuggestionsConsumer {
    instanceId: string;
    scopePolicyStamp: string;
    revision: string;
    current(): boolean;
}
type Rendered = Extract<CemProcessingNativeSessionResult, { status: 'rendered' }>;
export interface CemNativeSuggestionsComponentFrame {
    compile: CemProcessingCompileInput;
    render: Omit<CemProcessingRenderDiffInput, 'artifact' | 'nativeSuggestions'>;
}
export interface CemNativeSuggestionsBinding {
    readonly kind: 'cem-live-suggestions-binding-v1';
    readonly config: Readonly<CemNativePublicationConfig>;
    readonly valid: boolean;
    render(template: string, data: Record<string, unknown>): Promise<Rendered>;
    renderComponent(frame: CemNativeSuggestionsComponentFrame): Promise<CemProcessingRenderDiffResult>;
    rows(): Promise<readonly CemNativeSuggestionRow[]>;
    rowPlacements(result: CemProcessingRenderDiffResult): Promise<readonly { native: CemNativeSuggestionRow; renderNodeId: string }[]>;
    subscribe(invalidate: () => void): () => void;
    release(): void;
}
export interface CemNativeSuggestionsPublication {
    readonly valid: boolean;
    readonly config: Readonly<CemNativePublicationConfig>;
    bind(consumer: CemNativeSuggestionsConsumer): CemNativeSuggestionsBinding;
    release(): Promise<void>;
}
/** Exact native identity, session-local and excluded from durable state. */
export interface CemNativeSuggestionSource {
    readonly kind: 'cem-live-suggestion-source-v1';
    readonly valid: boolean;
    subscribe(invalidate: () => void): () => void;
}
/** Prepared scalar commit controls, not a projection of the source AST. */
export interface CemNativeSuggestionRow {
    readonly source: CemNativeSuggestionSource;
    readonly value: string;
    readonly eligible: boolean;
    readonly available: boolean;
    readonly valid: boolean;
}
const ROWS = Symbol.for('cem.live-suggestion-rows.v1');
const rowEnvironment = globalThis as typeof globalThis & { [ROWS]?: WeakMap<object, CemNativeSuggestionsBinding> };
const rows = rowEnvironment[ROWS] ??= new WeakMap();
export function isCemNativeSuggestionRow(value: unknown): value is CemNativeSuggestionRow {
    return typeof value === 'object' && value !== null && rows.has(value);
}
export function isCemNativeSuggestionRowFor(row: CemNativeSuggestionRow, binding: CemNativeSuggestionsBinding): boolean {
    return rows.get(row) === binding;
}
export function createCemNativeSuggestionSource(current: () => boolean, subscribe: (expire: () => void) => () => void): CemNativeSuggestionSource {
    let expired = false;
    const listeners = new Set<() => void>();
    const nativeSubscription: { stop(): void } = { stop: () => undefined };
    const expire = () => {
        if (expired) return;
        expired = true; nativeSubscription.stop();
        for (const listener of [...listeners]) { try { listener(); } catch { /* Revoke every retained proof. */ } }
        listeners.clear();
    };
    nativeSubscription.stop = subscribe(expire);
    if (expired) nativeSubscription.stop();
    const valid = () => { if (!expired && !current()) expire(); return !expired; };
    return Object.freeze({ kind: 'cem-live-suggestion-source-v1' as const,
        get valid() { return valid(); },
        subscribe(listener: () => void) { if (!valid()) { listener(); return () => undefined; } listeners.add(listener); return () => { listeners.delete(listener); }; },
        toJSON() { throw new TypeError('Live native source identity cannot be serialized'); } });
}

type Frame = Omit<Extract<CemProcessingNativeSessionInput, { action: 'render-suggestions-frame' }>, 'handle'>
    | Omit<Extract<CemProcessingNativeSessionInput, { action: 'suggestions-rows' }>, 'handle'>;
const BINDINGS = Symbol.for('cem.live-suggestions-bindings.v1');
const environment = globalThis as typeof globalThis & { [BINDINGS]?: WeakSet<object> };
const bindings = environment[BINDINGS] ??= new WeakSet();
/** Exact realm-local origin across runtime copies; availability still requires `.valid`. */
export function isCemNativeSuggestionsBinding(value: unknown): value is CemNativeSuggestionsBinding {
    return typeof value === 'object' && value !== null && bindings.has(value);
}
/** A live binding is exact transient authority, never a CEMV value or saved island. */
export function createCemNativeSuggestionsPublication(config: Readonly<CemNativePublicationConfig>, current: () => boolean,
    run: (input: Frame) => Promise<CemProcessingNativeSessionResult>, releaseOwner: () => Promise<void>, publication: string,
    runComponent: (frame: CemNativeSuggestionsComponentFrame) => Promise<CemProcessingRenderDiffResult>,
    sourceForHandle: (handle: string) => CemNativeSuggestionSource): CemNativeSuggestionsPublication {
    let released = false, expired = false;
    const publicationListeners = new Set<() => void>();
    const notify = (listeners: Set<() => void>) => { for (const listener of [...listeners]) { try { listener(); } catch { /* Revocation must reach every lease. */ } } };
    const valid = () => {
        if (!released && !expired && !current()) { expired = true; notify(publicationListeners); }
        return !released && !expired;
    };
    const nonportable = () => { throw new TypeError('Live suggestions authority cannot be serialized; reacquire on resume'); };
    const leases = new WeakMap<object, { consumer: CemNativeSuggestionsConsumer; released: boolean; expired: boolean }>();
    const publisher = Object.freeze({
        get valid() { return valid(); }, config: Object.freeze({ ...config }),
        bind(consumer: CemNativeSuggestionsConsumer): CemNativeSuggestionsBinding {
            if (this !== publisher || !valid()) throw new Error('Native suggestions publication is no longer current');
            if (!consumer || typeof consumer.current !== 'function' || [consumer.instanceId, consumer.scopePolicyStamp, consumer.revision]
                .some(value => typeof value !== 'string' || !value || value.length > 1024) || !consumer.current()) throw new TypeError('Invalid admitted native consumer');
            const state = { consumer: Object.freeze({ ...consumer }), released: false, expired: false };
            const listeners = new Set<() => void>();
            const results = new WeakMap<CemProcessingRenderDiffResult, readonly { handle: string; renderNodeId: string }[]>();
            const rowHandles = new WeakMap<CemNativeSuggestionRow, string>();
            const revoke = () => { if (state.expired) return; state.expired = true; notify(listeners); };
            publicationListeners.add(revoke);
            const admitted = (receiver: object) => {
                if (leases.get(receiver) !== state) throw new TypeError('A copied native binding has no consumer authority');
                if (state.released || state.expired || !valid()) return false;
                if (!state.consumer.current()) revoke();
                return !state.expired;
            };
            const binding: CemNativeSuggestionsBinding = Object.freeze({
                kind: 'cem-live-suggestions-binding-v1' as const,
                config: publisher.config,
                get valid() { return admitted(this); },
                async render(template: string, data: Record<string, unknown>) {
                    if (!admitted(this)) throw new Error('Native consumer binding is no longer current');
                    assertProcessingBoundaryValue(data, 'native consumer control frame');
                    const captured = structuredClone(data);
                    const result = await run({ action: 'render-suggestions-frame', publication, template, data: captured,
                        consumer: { instanceId: state.consumer.instanceId, scopePolicyStamp: state.consumer.scopePolicyStamp, revision: state.consumer.revision } });
                    if (!admitted(this)) throw new Error('Native consumer result was superseded');
                    if (result.status !== 'rendered') throw new Error('Invalid native consumer frame reply');
                    return result;
                },
                async renderComponent(frame: CemNativeSuggestionsComponentFrame) {
                    if (!admitted(this)) throw new Error('Native consumer binding is no longer current');
                    const { revision } = frame.render;
                    if (revision.instanceId !== state.consumer.instanceId || revision.scopePolicyStamp !== state.consumer.scopePolicyStamp
                        || revision.dataRevision !== state.consumer.revision || frame.compile.scopePolicyStamp !== revision.scopePolicyStamp
                        || frame.compile.templateArtifactId !== revision.templateArtifactId || 'nativeSuggestions' in frame.render) {
                        throw new TypeError('Native component frame differs from its admitted consumer');
                    }
                    assertProcessingBoundaryValue(frame, 'native component control frame');
                    const result = await runComponent(structuredClone(frame));
                    if (!admitted(this)) throw new Error('Native consumer result was superseded');
                    results.set(result, Object.freeze((result.suggestionPlacements ?? []).map(p => Object.freeze({ ...p }))));
                    return result;
                },
                async rows() {
                    if (!admitted(this)) throw new Error('Native consumer binding is no longer current');
                    if (config.profile === 'native-datalist') throw new Error('Native datalist has no row selection proof');
                    const result = await run({ action: 'suggestions-rows', publication });
                    if (!admitted(this)) throw new Error('Native row preparation was superseded');
                    if (result.status !== 'rows') throw new Error('Invalid native row preparation reply');
                    const prepared = result.rows.map(control => {
                        const source = sourceForHandle(control.handle);
                        const row: CemNativeSuggestionRow = Object.freeze({ source, value: control.value, eligible: control.eligible, available: control.available,
                            get valid() { return admitted(binding) && source.valid; }, toJSON: nonportable });
                        rows.set(row, binding); rowHandles.set(row, control.handle); return row;
                    });
                    return Object.freeze(prepared);
                },
                async rowPlacements(result: CemProcessingRenderDiffResult) {
                    if (!admitted(this)) throw new Error('Native consumer binding is no longer current');
                    const metadata = results.get(result);
                    if (config.profile === 'native-datalist') {
                        if (!metadata || metadata.length) throw new Error('Invalid native datalist frame');
                        return Object.freeze([]);
                    }
                    if (!metadata) throw new TypeError('Placement metadata requires the exact native render result');
                    if (!metadata.length) return Object.freeze([]);
                    const prepared = await binding.rows(), byHandle = new Map(prepared.map(row => [rowHandles.get(row), row]));
                    const targets = new Map<string, string>(), nodeIds = new Set<string>();
                    for (const { handle, renderNodeId } of metadata) {
                        if (!byHandle.has(handle) || targets.has(handle) || nodeIds.has(renderNodeId) || !renderNodeId) throw new TypeError('Invalid current native row placement');
                        targets.set(handle, renderNodeId); nodeIds.add(renderNodeId);
                    }
                    // Constructed order cannot change the native source-order navigation contract.
                    return Object.freeze(prepared.flatMap(native => {
                        const handle = rowHandles.get(native), renderNodeId = handle && targets.get(handle);
                        return renderNodeId ? [Object.freeze({ native, renderNodeId })] : [];
                    }));
                },
                subscribe(listener: () => void) {
                    if (leases.get(this) !== state) throw new TypeError('A copied native binding has no consumer authority');
                    if (!admitted(this)) { listener(); return () => undefined; }
                    listeners.add(listener); return () => { listeners.delete(listener); };
                },
                release() {
                    if (leases.get(this) !== state) throw new TypeError('A copied native binding has no consumer authority');
                    if (state.released) return;
                    state.released = true; publicationListeners.delete(revoke); notify(listeners); listeners.clear();
                },
                toJSON: nonportable,
            });
            leases.set(binding, state); bindings.add(binding); return binding;
        },
        async release() { if (this !== publisher) throw new TypeError('Invalid native publication owner'); if (released) return; released = true; notify(publicationListeners); publicationListeners.clear(); await releaseOwner(); },
        toJSON: nonportable,
    });
    return publisher;
}
