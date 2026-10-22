// eslint-disable-next-line @nx/enforce-module-boundaries -- generated native bindings are vendored by the package build.
import * as legacy from '../../../../../cem_ql/dist/wasm/cem_ql.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- generated owned-factory contract is vendored with its bindings.
import type { CemQlWasmGeneration } from '../../../../../cem_ql/dist/wasm/cem_ql_factory.js';
import { CemNativeCheckpointDomain, cemMainThreadCheckpointDomain as domain } from './checkpoint-domain.js';

import type { CemNativeGenerationCandidate, CemNativeGenerationParticipant } from './native-generation.js';

export type CemFallbackNativeEntrypoint = typeof sharedEntrypoints[number];
export interface CemFreshFallbackGeneration {
    readonly kind: 'cem-fallback';
    readonly runtimeId: string;
    /** Trusted native embedding boundary. Bindings never switch physical heaps. */
    invoke(name: CemFallbackNativeEntrypoint, args: unknown[]): unknown;
    claimOwnership(): CemFallbackWasmOwnership;
    releaseOwnership(ownership: CemFallbackWasmOwnership): void;
    retirementConsent(ownership: readonly CemFallbackWasmOwnership[]): CemFallbackRetirementConsent;
    generationParticipant(): CemNativeGenerationParticipant;
}
export interface CemFallbackRetirementConsent { readonly kind: 'cem-fallback-retirement-consent' }
export interface CemFallbackWasmOwnership { readonly kind: 'cem-fallback-wasm-ownership' }
export interface CemDisposedFallbackGeneration { readonly runtimeId: string; readonly releasedOwnedReferences: true; readonly replacementAllowed: false }
export interface CemOwnedFallbackGeneration {
    readonly runtimeId: string;
    ready(): Promise<void>;
    generationParticipant(): CemNativeGenerationParticipant;
    claimOwnership(): CemFallbackWasmOwnership;
    releaseOwnership(ownership: CemFallbackWasmOwnership): void;
    retirementConsent(ownership: readonly CemFallbackWasmOwnership[]): CemFallbackRetirementConsent;
    /** Only after every managed host/helper and resource has been released.
     * This releases owned references, without claiming synchronous browser GC. */
    disposeEmpty(ownership: readonly CemFallbackWasmOwnership[], timeoutMs?: number): Promise<CemDisposedFallbackGeneration>;
}
let started = false;
let legacyInitialization: Promise<void> | undefined;
let owned: OwnedFallback | undefined;

/** Explicit pre-initialization opt-in; aliases never switch a live heap. */
export function isolateCemFallbackGeneration(): CemOwnedFallbackGeneration {
    if (started || owned) throw new Error('cem.runtime.generation.already-selected');
    owned = new OwnedFallback(domain);
    const selected = owned;
    return Object.freeze({ runtimeId: domain.issuedIdentity(), ready: () => initializeNativeRuntime(),
        generationParticipant: () => selected.participant(),
        claimOwnership: () => selected.claim(), releaseOwnership: (lease: CemFallbackWasmOwnership) => selected.release(lease),
        retirementConsent: (leases: readonly CemFallbackWasmOwnership[]) => selected.consent(leases),
        disposeEmpty: (leases: readonly CemFallbackWasmOwnership[], timeoutMs?: number) => selected.disposeEmpty(leases, timeoutMs) });
}
export function nativeRuntimeUsable(): boolean { return !owned || owned.usable(); }
export default function initializeNativeRuntime(): Promise<void> {
    started = true;
    if (owned) return owned.initialize();
    return legacyInitialization ??= legacy.default().then(native => { domain.bindNativeRuntime(native.memory); });
}

class OwnedFallback {
    constructor(private readonly domain = new CemNativeCheckpointDomain('main-thread')) {}
    participant(): CemNativeGenerationParticipant { return this.domain.generationParticipant(); }
    private native?: CemQlWasmGeneration;
    private initialization?: Promise<void>;
    private disposed = false;
    private failed = false;
    private disposing = false;
    private revision = 0;
    private readonly consents = new WeakMap<object, number>();
    private readonly owners = new Set<CemFallbackWasmOwnership>();

    usable(): boolean { return !this.disposed && !this.failed && !!this.domain.identity(); }
    initialize(signal?: AbortSignal): Promise<void> {
        if (signal?.aborted) return Promise.reject(new Error('cem.runtime.allocation.cancelled'));
        if (!this.usable()) return Promise.reject(new Error('cem.runtime.generation.retired'));
        // eslint-disable-next-line @nx/enforce-module-boundaries -- lazy generated factory; the package build rewrites this to its vendored copy.
        return this.initialization ??= import('../../../../../cem_ql/dist/wasm/cem_ql_factory.js').then(async module => {
            this.native = module.createCemQlWasmGeneration();
            await this.native.initialize(signal ? new Request(new URL('../../../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url), { signal }) : undefined);
            if (signal?.aborted) throw new Error('cem.runtime.allocation.cancelled');
            const memory = this.native.memory();
            if (!memory) throw new Error('cem.runtime.generation.identity');
            this.domain.bindNativeRuntime(memory);
            if (!this.domain.ownsNativeRuntime(memory)) throw new Error('cem.runtime.generation.identity');
            this.installRetirementEndpoint();
        }).catch(error => { this.failed = true; this.native = undefined; this.domain.revoke(); throw error; });
    }
    /** A separate callback scope never captures initialization's memory/module locals. */
    private installRetirementEndpoint(): void {
        this.domain.ownedRetirementEndpoint({
            allocate: (admitted, signal) => this.allocateFresh(admitted, signal),
            close: async () => { this.requireReady().call('closeCommandNativeAdmission', []); },
            authorize: token => {
                const revision = typeof token === 'object' && token !== null ? this.consents.get(token) : undefined;
                if (revision === undefined || revision !== this.revision || !this.usable()) throw new Error('cem.runtime.generation.ownership');
                return () => this.usable() && revision === this.revision;
            },
            commit: current => {
                if (this.disposing) throw new Error('cem.runtime.generation.pending');
                this.disposing = true;
                try { this.requireReady().dispose(current); this.released(); }
                finally { this.disposing = false; }
            },
        });
    }
    private async allocateFresh(admitted: () => boolean, signal: AbortSignal): Promise<CemNativeGenerationCandidate> {
        const next = new OwnedFallback();
        await next.initialize(signal);
        const identity = next.requireReady().memory();
        if (!identity) throw new Error('cem.runtime.generation.identity');
        const assert = () => { if (!admitted()) throw new Error('cem.runtime.allocation.closed'); };
        const runtime: CemFreshFallbackGeneration = Object.freeze({ kind: 'cem-fallback', runtimeId: next.domain.issuedIdentity(),
            invoke: (name: CemFallbackNativeEntrypoint, args: unknown[]) => {
                assert(); if (!(sharedEntrypoints as readonly string[]).includes(name)) throw new Error('cem.runtime.generation.entrypoint');
                return next.call(name, args);
            },
            claimOwnership: () => { assert(); return next.claim(); },
            releaseOwnership: (lease: CemFallbackWasmOwnership) => { assert(); next.release(lease); },
            retirementConsent: (leases: readonly CemFallbackWasmOwnership[]) => { assert(); return next.consent(leases); },
            generationParticipant: () => { assert(); return next.participant(); } });
        return { identity, runtimeId: runtime.runtimeId, runtime,
            current: () => {
                if (!next.usable()) return false;
                const text = next.call('runtimeMemoryStatus', []);
                if (typeof text !== 'string' || text.length > 2048) return false;
                const status = JSON.parse(text) as { softPressure?: unknown; hardPressure?: unknown };
                return status.softPressure === false && status.hardPressure === false;
            },
            discard: () => {
                try { next.native?.dispose(); } finally { next.failed = true; next.native = undefined; next.initialization = undefined; next.domain.discardAllocatedIdentity(); }
            } };
    }
    private requireReady(): CemQlWasmGeneration {
        if (!this.usable()) throw new Error('cem.runtime.generation.retired');
        if (!this.native || !this.domain.ownsNativeRuntime(this.native.memory())) throw new Error('cem.runtime.generation.unavailable');
        return this.native;
    }
    call(name: string, args: unknown[]): unknown { return this.requireReady().call(name, args); }
    claim(): CemFallbackWasmOwnership {
        this.requireReady();
        if (this.domain.pressureReason() || this.owners.size >= 256) throw new Error('cem.runtime.generation.ownership');
        const lease = Object.freeze({ kind: 'cem-fallback-wasm-ownership' as const });
        this.owners.add(lease); this.changed(); return lease;
    }
    release(lease: CemFallbackWasmOwnership): void {
        this.requireReady();
        if (!this.owners.delete(lease)) throw new Error('cem.runtime.generation.ownership');
        this.changed();
    }
    private changed(): void {
        if (!Number.isSafeInteger(++this.revision)) { this.failed = true; this.domain.revoke(); }
        this.domain.changed();
    }
    private ownershipRevision(leases: readonly CemFallbackWasmOwnership[]): number {
        const revision = this.revision;
        if (!Array.isArray(leases)) throw new Error('cem.runtime.generation.ownership');
        const count = Object.getOwnPropertyDescriptor(leases, 'length')?.value;
        if (!Number.isSafeInteger(count) || count < 1 || count > 256 || count !== this.owners.size) throw new Error('cem.runtime.generation.ownership');
        const seen = new Set<CemFallbackWasmOwnership>();
        for (let index = 0; index < count; index++) {
            // Read own data slots only: caller methods, iterators and
            // accessors cannot substitute consent or run during admission.
            const slot = Object.getOwnPropertyDescriptor(leases, String(index));
            if (!slot || !('value' in slot) || !this.owners.has(slot.value) || seen.has(slot.value)) throw new Error('cem.runtime.generation.ownership');
            seen.add(slot.value);
        }
        if (revision !== this.revision) throw new Error('cem.runtime.generation.ownership-stale');
        return revision;
    }
    consent(leases: readonly CemFallbackWasmOwnership[]): CemFallbackRetirementConsent {
        this.requireReady();
        if (this.disposing || this.domain.pressureReason()) throw new Error('cem.runtime.generation.ownership');
        this.disposing = true;
        try {
            const revision = this.ownershipRevision(leases);
            const token = Object.freeze({ kind: 'cem-fallback-retirement-consent' as const });
            this.consents.set(token, revision); return token;
        } finally { this.disposing = false; }
    }
    private released(): void { this.disposed = true; this.native = undefined; this.initialization = undefined; this.owners.clear(); }
    async disposeEmpty(leases: readonly CemFallbackWasmOwnership[], timeoutMs = 5000): Promise<CemDisposedFallbackGeneration> {
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) throw new Error('cem.runtime.generation.timeout-config');
        const native = this.requireReady();
        if (this.disposing) throw new Error('cem.runtime.generation.pending');
        this.disposing = true;
        try {
            const revision = this.ownershipRevision(leases);
            if (!this.domain.emptyForNativeDisposal()) throw new Error('cem.runtime.generation.peers');
            this.domain.closeAdmission('soft-pressure');
            const proof = await this.domain.drain({ timeoutMs });
            if (revision !== this.revision) throw new Error('cem.runtime.generation.ownership-stale');
            this.domain.releaseEmptyNative(proof, () => { native.dispose(() => revision === this.revision
                && this.domain.emptyForNativeDisposal() && this.domain.drainCurrent(proof)); });
            this.released();
            return Object.freeze({ runtimeId: this.domain.issuedIdentity(), releasedOwnedReferences: true, replacementAllowed: false });
        } finally { this.disposing = false; }
    }
}
function bind<K extends keyof typeof legacy>(name: K): typeof legacy[K] {
    return ((...args: unknown[]) => {
        started = true;
        if (owned) return owned.call(name, args);
        const operation = legacy[name];
        if (typeof operation !== 'function') throw new Error('cem.runtime.generation.entrypoint');
        return Reflect.apply(operation, undefined, args);
    }) as unknown as typeof legacy[K];
}

export const closeCommandNativeAdmission = bind('closeCommandNativeAdmission');
export const commandNativeResourceInventory = bind('commandNativeResourceInventory');
export const prepareCommandNativeRetirement = bind('prepareCommandNativeRetirement');
export const cemQlVersion = bind('cemQlVersion');
export const runtimeMemoryStatus = bind('runtimeMemoryStatus');
export const validateCheckpointControls = bind('validateCheckpointControls');
export const validateCheckpointRuntime = bind('validateCheckpointRuntime');
export const compileTemplate = bind('compileTemplate');
export const adoptDomStylesheets = bind('adoptDomStylesheets');
export const adoptInstanceStylesheets = bind('adoptInstanceStylesheets');
export const beginTemplateStylesheet = bind('beginTemplateStylesheet');
export const deliverTemplateStylesheet = bind('deliverTemplateStylesheet');
export const failTemplateStylesheet = bind('failTemplateStylesheet');
export const releaseTemplateStylesheets = bind('releaseTemplateStylesheets');
export const compileTemplateArtifact = bind('compileTemplateArtifact');
export const compileTemplateModuleClosure = bind('compileTemplateModuleClosure');
export const convertLegacyCustomElementTemplate = bind('convertLegacyCustomElementTemplate');
export const disposeTemplate = bind('disposeTemplate');
export const importTemplateArtifact = bind('importTemplateArtifact');
export const renderTemplateWithNativeValues = bind('renderTemplateWithNativeValues');
export const renderTemplateWithNativeValuesAndAdmission = bind('renderTemplateWithNativeValuesAndAdmission');
export const importCemDocumentValue = bind('importCemDocumentValue');
export const exportCemJsonValue = bind('exportCemJsonValue');
export const importNativeValueArtifact = bind('importNativeValueArtifact');
export const importReferenceReloadBundle = bind('importReferenceReloadBundle');
export const parseReferenceSource = bind('parseReferenceSource');
export const prepareNativeCapabilitySession = bind('prepareNativeCapabilitySession');
export const prepareNativeDatalist = bind('prepareNativeDatalist');
export const publishNativeDatalist = bind('publishNativeDatalist');
export const nativeCapabilitySessionLength = bind('nativeCapabilitySessionLength');
export const nativeSuggestionRowControls = bind('nativeSuggestionRowControls');
export const exportNativeCapabilityView = bind('exportNativeCapabilityView');
export const renderNativeCapabilityTemplate = bind('renderNativeCapabilityTemplate');
export const disposeNativeCapabilitySession = bind('disposeNativeCapabilitySession');
export const prepareNativeSuggestions = bind('prepareNativeSuggestions');
export const configureNativeSuggestionLabels = bind('configureNativeSuggestionLabels');
export const exportNativeSuggestionsView = bind('exportNativeSuggestionsView');
export const renderNativeSuggestionTemplate = bind('renderNativeSuggestionTemplate');
export const publishNativeSuggestions = bind('publishNativeSuggestions');
export const releaseNativeSuggestions = bind('releaseNativeSuggestions');
export const renderNativeSuggestionsFrame = bind('renderNativeSuggestionsFrame');
export const disposeReferenceSource = bind('disposeReferenceSource');
export const disposeNativeValueArtifact = bind('disposeNativeValueArtifact');
export const takeRenderValueArtifact = bind('takeRenderValueArtifact');
export const retainCemDocument = bind('retainCemDocument');
export const disposeCemDocument = bind('disposeCemDocument');
export const renderTemplateSource = bind('renderTemplateSource');
export const resolveModuleUrl = bind('resolveModuleUrl');
export const resolveNavigationLinks = bind('resolveNavigationLinks');
export const resolveResourceBase = bind('resolveResourceBase');
export const remapArtifactCss = bind('remapArtifactCss');
export const templateArtifactPayloadKey = bind('templateArtifactPayloadKey');
export const templateModuleImports = bind('templateModuleImports');
export const retainXsltComponent = bind('retainXsltComponent');
export const renderXsltComponentWithNativeValues = bind('renderXsltComponentWithNativeValues');
export const disposeXsltComponent = bind('disposeXsltComponent');
export const xsltStylesheetImports = bind('xsltStylesheetImports');
export const evaluateQuerySource = bind('evaluateQuerySource');
export const retainCemtXPathFunctions = bind('retainCemtXPathFunctions');
export const disposeCemtXPathFunctions = bind('disposeCemtXPathFunctions');

const sharedEntrypoints = [
    'closeCommandNativeAdmission', 'commandNativeResourceInventory', 'prepareCommandNativeRetirement',
    'cemQlVersion', 'runtimeMemoryStatus', 'validateCheckpointControls',
    'validateCheckpointRuntime', 'compileTemplate', 'adoptDomStylesheets',
    'adoptInstanceStylesheets', 'beginTemplateStylesheet', 'deliverTemplateStylesheet',
    'failTemplateStylesheet', 'releaseTemplateStylesheets', 'compileTemplateArtifact',
    'compileTemplateModuleClosure', 'convertLegacyCustomElementTemplate', 'disposeTemplate',
    'importTemplateArtifact', 'renderTemplateWithNativeValues', 'renderTemplateWithNativeValuesAndAdmission',
    'importCemDocumentValue', 'exportCemJsonValue', 'importNativeValueArtifact',
    'importReferenceReloadBundle', 'parseReferenceSource', 'prepareNativeCapabilitySession',
    'prepareNativeDatalist', 'publishNativeDatalist', 'nativeCapabilitySessionLength',
    'nativeSuggestionRowControls', 'exportNativeCapabilityView', 'renderNativeCapabilityTemplate',
    'disposeNativeCapabilitySession', 'prepareNativeSuggestions', 'configureNativeSuggestionLabels',
    'exportNativeSuggestionsView', 'renderNativeSuggestionTemplate', 'publishNativeSuggestions',
    'releaseNativeSuggestions', 'renderNativeSuggestionsFrame', 'disposeReferenceSource',
    'disposeNativeValueArtifact', 'takeRenderValueArtifact', 'retainCemDocument',
    'disposeCemDocument', 'renderTemplateSource', 'resolveModuleUrl',
    'resolveNavigationLinks', 'resolveResourceBase', 'remapArtifactCss',
    'templateArtifactPayloadKey', 'templateModuleImports', 'retainXsltComponent',
    'renderXsltComponentWithNativeValues', 'disposeXsltComponent', 'xsltStylesheetImports',
    'evaluateQuerySource', 'retainCemtXPathFunctions', 'disposeCemtXPathFunctions',
] as const;
