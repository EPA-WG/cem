import type { CemNativeSuggestionsConfig } from './native-capability-session.js';
import type { CemProcessingNativeSessionInput, CemProcessingNativeSessionResult } from './internal/runtime-support/processing-host.js';
import { assertProcessingBoundaryValue } from './projection.js';

/** Supplied by the host that admitted this consuming instance and captured revision. */
export interface CemNativeSuggestionsConsumer {
    instanceId: string;
    scopePolicyStamp: string;
    revision: string;
    current(): boolean;
}
type Rendered = Extract<CemProcessingNativeSessionResult, { status: 'rendered' }>;
export interface CemNativeSuggestionsBinding {
    readonly kind: 'cem-live-suggestions-binding-v1';
    readonly valid: boolean;
    render(template: string, data: Record<string, unknown>): Promise<Rendered>;
    release(): void;
}
export interface CemNativeSuggestionsPublication {
    readonly valid: boolean;
    readonly config: Readonly<CemNativeSuggestionsConfig>;
    bind(consumer: CemNativeSuggestionsConsumer): CemNativeSuggestionsBinding;
    release(): Promise<void>;
}
type Frame = Omit<Extract<CemProcessingNativeSessionInput, { action: 'render-suggestions-frame' }>, 'handle'>;
const BINDINGS = Symbol.for('cem.live-suggestions-bindings.v1');
const environment = globalThis as typeof globalThis & { [BINDINGS]?: WeakSet<object> };
const bindings = environment[BINDINGS] ??= new WeakSet();
/** Exact realm-local origin across runtime copies; availability still requires `.valid`. */
export function isCemNativeSuggestionsBinding(value: unknown): value is CemNativeSuggestionsBinding {
    return typeof value === 'object' && value !== null && bindings.has(value);
}
/** A live binding is exact transient authority, never a CEMV value or saved island. */
export function createCemNativeSuggestionsPublication(config: Readonly<CemNativeSuggestionsConfig>, current: () => boolean,
    run: (input: Frame) => Promise<CemProcessingNativeSessionResult>, releaseOwner: () => Promise<void>, publication: string): CemNativeSuggestionsPublication {
    let released = false, expired = false;
    const valid = () => !released && !(expired ||= !current());
    const nonportable = () => { throw new TypeError('Live suggestions authority cannot be serialized; reacquire on resume'); };
    const leases = new WeakMap<object, { consumer: CemNativeSuggestionsConsumer; released: boolean; expired: boolean }>();
    const publisher = Object.freeze({
        get valid() { return valid(); }, config: Object.freeze({ ...config }),
        bind(consumer: CemNativeSuggestionsConsumer): CemNativeSuggestionsBinding {
            if (this !== publisher || !valid()) throw new Error('Native suggestions publication is no longer current');
            if (!consumer || typeof consumer.current !== 'function' || [consumer.instanceId, consumer.scopePolicyStamp, consumer.revision]
                .some(value => typeof value !== 'string' || !value || value.length > 1024) || !consumer.current()) throw new TypeError('Invalid admitted native consumer');
            const state = { consumer: Object.freeze({ ...consumer }), released: false, expired: false };
            const admitted = (receiver: object) => {
                if (leases.get(receiver) !== state) throw new TypeError('A copied native binding has no consumer authority');
                if (state.released || state.expired || !valid()) return false;
                state.expired ||= !state.consumer.current();
                return !state.expired;
            };
            const binding: CemNativeSuggestionsBinding = Object.freeze({
                kind: 'cem-live-suggestions-binding-v1' as const,
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
                release() { if (leases.get(this) !== state) throw new TypeError('A copied native binding has no consumer authority'); state.released = true; },
                toJSON: nonportable,
            });
            leases.set(binding, state); bindings.add(binding); return binding;
        },
        async release() { if (this !== publisher) throw new TypeError('Invalid native publication owner'); if (released) return; released = true; await releaseOwner(); },
        toJSON: nonportable,
    });
    return publisher;
}
