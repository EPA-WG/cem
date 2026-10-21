import { checkpointScalarPortable } from './checkpoint-controls.js';
import type { CemValueArtifactLimits, NativeCemAttributeBinding, NativeCemSliceBinding, NativeCemValue } from './native-values.js';
import type { CemCheckpointRelationship } from './committed-checkpoint.js';
/** Named scalar lifecycle contracts. Original source bytes enter native import. */
export type CheckpointEndpoint = CemCheckpointRelationship['provider'];
export interface CheckpointSource {
    preparation?: CheckpointPreparation;
    version: 'cem-native-source/1'; revision: string; rowCount: number; select: string; resolve: boolean; requesting: number;
    sources: { codec: 'xml' | 'cemb'; base64: string; sourceUri: string | null; primarySourceId: number; context: boolean; digest?: string | null }[];
    bindings: { source: number; name: string; select: string }[];
    crossings: { from: number; to: number }[]; labels: { option: string | null; group: string | null }; disposition: 'reacquire';
}
export interface CheckpointReadySuggestions {
    source: CheckpointSource; providerRevision: number; queryRevision: number; preparedRevision: number; query: string;
    publication: { filter: string; filterBy: string | null; expanded: boolean | null; active: number | null; committed: number | null };
    intent: boolean; feedbackIntent: boolean; feedbackState: 'ready'; activeIndex: number; committedIndex: number; committedValue: string | null;
    visible: boolean; scrollTop: number; scrollLeft: number;
    rows: { index: number; value: string; eligible: boolean; available: boolean; nodeId: string }[];
}
/** Source-backed declarative frame without a stateful editor controller. */
export interface CheckpointNativePublication {
    profile: 'listbox'; source: CheckpointSource; query: string; queryRevision: number;
    publication: CheckpointReadySuggestions['publication']; rows: CheckpointReadySuggestions['rows']; disposition: 'reacquire';
}
export interface CheckpointInactiveSuggestions {
    phase: 'inactive'; profile: 'listbox' | 'native-datalist'; source: CheckpointSource;
    providerRevision: number; queryRevision: number; query: string;
    feedbackState: 'idle' | 'failed' | 'unavailable'; feedbackIntent: boolean; editable: boolean;
    proof?: { source: CheckpointSource; index: number; value: string; disposition: 'revalidate' };
}
export interface CheckpointDatalist {
    phase: 'native-datalist'; source: CheckpointSource; providerRevision: number; targetNode: string;
    claim: 'attached' | 'unavailable'; presentation: 'reset-on-resume';
}
export type CheckpointSuggestions = CheckpointReadySuggestions | CheckpointInactiveSuggestions | CheckpointDatalist;
/** Original XML at an explicit source boundary; this envelope confers no authority. */
export function checkpointXmlSource(text: string, sourceUri: string, revision: string, select: string,
    labels: CheckpointSource['labels'] = { option: null, group: null }): CheckpointSource | undefined {
    if (sourceUri.length > 4096 || !revision || revision.length > 1024 || select.length > 4096
        || Object.values(labels).some(value => value !== null && value.length > 4096) || text.length > 256 * 1024) return undefined;
    const base64 = checkpointBase64(new TextEncoder().encode(text).buffer); if (base64 === undefined) return undefined;
    return { version: 'cem-native-source/1', revision, rowCount: 0, select, resolve: false, requesting: 0,
        sources: [{ codec: 'xml', base64, sourceUri, primarySourceId: 1, context: true }], bindings: [], crossings: [], labels: { ...labels }, disposition: 'reacquire' };
}
export interface CheckpointPlacement {
    requesterNode: string; property: string; producer: string; revision: string; providerNode: string;
    sourceIndex: number; path: number[]; select: string; source: CheckpointSource; disposition: 'reacquire';
}
export function checkpointBase64(buffer: ArrayBuffer): string | undefined {
    if (buffer.byteLength > 256 * 1024) return undefined;
    let binary = ''; for (const byte of new Uint8Array(buffer)) binary += String.fromCharCode(byte);
    return btoa(binary);
}

export interface CheckpointSurface {
    body?: CheckpointDeferredBody;
    returnValue: string | null; modal: boolean; focusWithin: boolean; closeReason: string; disposition: 'reacquire';
    invocation: null | { source: CheckpointEndpoint | null; returnDestination: CheckpointEndpoint | null; logicalParent: CheckpointEndpoint | null;
        command: string | null; contextKey: string | null; context?: CheckpointNativeCapsule; inputKind: 'pointer' | 'keyboard' | 'programmatic';
        pointer: { left: number; right: number; top: number; bottom: number } | null;
        selection: { left: number; right: number; top: number; bottom: number } | null };
}
export interface CheckpointMenu { parent: CheckpointEndpoint | null; launcher: CheckpointEndpoint | null; panel: CheckpointEndpoint | null; disposition: 'reacquire'; policies?: CheckpointMenuPolicies }
export interface CheckpointMenuRole { mode: 'target' | 'invoker' | 'viewport' | 'auto' | 'none'; target: CheckpointEndpoint | null }
export interface CheckpointMenuPolicies {
    compositeNode: string; anchor: CheckpointMenuRole; boundary: CheckpointMenuRole; boundaryOwner: CheckpointEndpoint | null;
    focusTarget: CheckpointMenuRole; returnFocus: CheckpointMenuRole; anchorLost: 'close' | 'freeze'; geometry: 'recompute';
}
export interface CheckpointPopup { trigger: string; panel: string; scrollTop: number; scrollLeft: number; disposition: 'reacquire' }

// Exact invocation snapshots are private host evidence, never inferred from markup.
const referenceSnapshots = new WeakSet<object>();
import type { CemElementReferenceInputs } from './element-reference-inputs.js';
export function snapshotCheckpointReferences(input: CemElementReferenceInputs): CemElementReferenceInputs | undefined {
    if (input.sources.length > 64 || input.bindings.length > 256 || input.grants.length > 256
        || input.sources.reduce((sum, source) => sum + source.bundle.byteLength, 0) > 256 * 1024
        || input.bindings.some(binding => binding.name.length > 1024 || binding.select.length > 4096)
        || input.sources.some(source => source.maxDepth !== undefined || source.maxWork !== undefined || source.unresolved !== undefined)) return undefined;
    const snapshot: CemElementReferenceInputs = Object.freeze({ ...input,
        sources: Object.freeze(input.sources.map(source => Object.freeze({ ...source, bundle: source.bundle.slice(0) }))),
        bindings: Object.freeze(input.bindings.map(binding => Object.freeze({ ...binding }))), grants: Object.freeze(input.grants.map(([from, to]) => Object.freeze([from, to] as const))) });
    referenceSnapshots.add(snapshot); return snapshot;
}
export function checkpointReferenceSource(input: CemElementReferenceInputs, revision: string): CheckpointSource | undefined {
    if (!referenceSnapshots.has(input)) return undefined;
    const sources: CheckpointSource['sources'] = [];
    for (const source of input.sources) {
        const base64 = checkpointBase64(source.bundle); if (base64 === undefined) return undefined;
        sources.push({ codec: 'cemb', base64, sourceUri: null, primarySourceId: source.primarySourceId, context: source.context });
    }
    return { version: 'cem-native-source/1', revision, rowCount: 0, select: 'input.children', resolve: false, requesting: input.requesting,
        sources, bindings: input.bindings.map(binding => ({ ...binding })), crossings: input.grants.map(([from, to]) => ({ from, to })),
        labels: { option: null, group: null }, disposition: 'reacquire' };
}

export interface CheckpointDeferredBody {
    templateNode: string; mode: 'retain' | 'dispose'; materialized: boolean; roots: string[]; disposition: 'rematerialize';
    projection?: CheckpointRenderProjection; reuseOnOpen?: boolean; owners?: { hostNode: string; owner: CheckpointEndpoint }[];
}
export interface CheckpointManualListbox {
    panel: CheckpointEndpoint; editor: CheckpointEndpoint; providerRevision: number; visible: boolean;
    presentation: 'reset-on-resume'; disposition: 'reacquire';
}

/** Named renderer checkpoint transport. Never authored data or an evaluation AST. */
export interface CheckpointRenderProjection {
    version: 'cem-render-projection/1';
    nativeArtifacts?: { contentHash: string; base64: string }[];
    origin: { instanceId: string; templateArtifactId: string; dataRevision: string; scopePolicyStamp: string };
    entries: (
        | { kind: 'element'; nodeId: string; tag: string; namespace: string | null; attributes: { name: string; value: string; nativeValue?: { artifact: number; index: number } }[] }
        | { kind: 'text' | 'comment'; nodeId: string; text: string }
        | { kind: 'end' }
        | { kind: 'range-start' | 'range-end'; nodeId: string; rangeId: string }
    )[];
}

/** Exact standalone attachment intent; contains no live lease, list ID or picker state. */
export interface CheckpointStandaloneDatalist {
    editor: CheckpointEndpoint; target: CheckpointEndpoint; providerRevision: number;
    claim: 'attached' | 'unavailable'; source: CheckpointSource;
    presentation: 'reset-on-resume'; disposition: 'reacquire';
}

/** Explicit native-session control frame and opaque values, never native authority. */
export interface CheckpointNativeCapsule { artifact: { contentHash: string; base64: string }; index: number }
export interface CheckpointNativeBindings {
    version: 'cem-native-bindings/1'; artifacts: CheckpointNativeCapsule['artifact'][];
    bindings: { target: 'attribute' | 'slice'; name: string; artifact: number; index: number; attribute?: string }[];
}
export interface CheckpointPreparation { dataJson: string; limits: CemValueArtifactLimits; nativeBindings: CheckpointNativeBindings }
function portableNativeValue(value: NativeCemValue): boolean {
    if (!value || typeof value !== 'object' || Reflect.ownKeys(value).length !== 4) return false;
    for (const key of ['kind', 'artifact', 'contentHash', 'index']) {
        const property = Object.getOwnPropertyDescriptor(value, key);
        if (!property || !Object.hasOwn(property, 'value') || !property.enumerable) return false;
    }
    if (value.kind !== 'cem-native-value-v1' || !(value.artifact instanceof ArrayBuffer) || value.artifact.byteLength > 256 * 1024
        || typeof value.contentHash !== 'string' || value.contentHash.length > 128 || !Number.isSafeInteger(value.index) || value.index < 0
        || Reflect.ownKeys(value).some(key => !['kind', 'artifact', 'contentHash', 'index'].includes(key as string))) return false;
    return true;
}
export function snapshotCheckpointNativeValue(value: NativeCemValue): NativeCemValue | undefined {
    if (!portableNativeValue(value)) return undefined;
    return Object.freeze({ kind: value.kind, artifact: value.artifact.slice(0), contentHash: value.contentHash, index: value.index });
}
export function sameCheckpointNativeValue(value: NativeCemValue, snapshot: NativeCemValue): boolean {
    const witness = new Uint8Array(snapshot.artifact);
    return portableNativeValue(value) && value.kind === snapshot.kind && value.index === snapshot.index && value.contentHash === snapshot.contentHash
        && value.artifact.byteLength === snapshot.artifact.byteLength && new Uint8Array(value.artifact).every((byte, i) => byte === witness[i]);
}
export function checkpointNativeCapsule(value: NativeCemValue): CheckpointNativeCapsule | undefined {
    const snapshot = snapshotCheckpointNativeValue(value); if (!snapshot) return undefined;
    const base64 = checkpointBase64(snapshot.artifact);
    return base64 === undefined ? undefined : { artifact: { contentHash: snapshot.contentHash, base64 }, index: snapshot.index };
}
export function checkpointNativeBindings(attributes: readonly NativeCemAttributeBinding[] = [], slices: readonly NativeCemSliceBinding[] = []): CheckpointNativeBindings | undefined {
    if (attributes.length + slices.length > 256) return undefined;
    const artifacts: CheckpointNativeBindings['artifacts'] = [], bindings: CheckpointNativeBindings['bindings'] = [];
    const buffers = new Map<ArrayBuffer, number>();
    let bytes = 0;
    for (const [target, values] of [['attribute', attributes], ['slice', slices]] as const) for (const binding of values) {
        if (!binding.name || binding.name.length > 1024) return undefined;
        const attribute = target === 'slice' ? (binding as NativeCemSliceBinding).attribute : undefined;
        if (attribute !== undefined && (!attribute || attribute.length > 1024)) return undefined;
        const value = binding.value; if (!portableNativeValue(value)) return undefined;
        let artifact = buffers.get(value.artifact);
        if (artifact === undefined) {
            bytes += value.artifact.byteLength; if (bytes > 256 * 1024) return undefined;
            const capsule = checkpointNativeCapsule(value); if (!capsule) return undefined;
            artifact = artifacts.findIndex(item => item.contentHash === capsule.artifact.contentHash && item.base64 === capsule.artifact.base64);
            if (artifact < 0) {
                if (artifacts.length >= 64) return undefined;
                artifact = artifacts.length; artifacts.push(capsule.artifact);
            }
            buffers.set(value.artifact, artifact);
        } else if (artifacts[artifact].contentHash !== value.contentHash) return undefined;
        bindings.push({ target, name: binding.name, artifact, index: value.index, ...(attribute === undefined ? {} : { attribute }) });
    }
    return { version: 'cem-native-bindings/1', artifacts, bindings };
}
export function checkpointPreparation(input: { data: Record<string, unknown>; limits: CemValueArtifactLimits;
    nativeAttributes?: readonly NativeCemAttributeBinding[]; nativeSlices?: readonly NativeCemSliceBinding[] }): CheckpointPreparation | undefined {
    if (!checkpointScalarPortable(input.data)) return undefined;
    const dataJson = JSON.stringify(input.data); if (new TextEncoder().encode(dataJson).byteLength > 256 * 1024) return undefined;
    const nativeBindings = checkpointNativeBindings(input.nativeAttributes, input.nativeSlices);
    return nativeBindings ? { dataJson, limits: { maxBytes: input.limits.maxBytes, maxValues: input.limits.maxValues, maxDepth: input.limits.maxDepth }, nativeBindings } : undefined;
}
