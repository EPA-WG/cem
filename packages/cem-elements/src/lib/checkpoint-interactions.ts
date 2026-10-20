import type { CemCheckpointRelationship } from './committed-checkpoint.js';
/** Named scalar lifecycle contracts. Original source bytes enter native import. */
export type CheckpointEndpoint = CemCheckpointRelationship['provider'];
export interface CheckpointSource {
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
        command: string | null; contextKey: string | null; inputKind: 'pointer' | 'keyboard' | 'programmatic';
        pointer: { left: number; right: number; top: number; bottom: number } | null;
        selection: { left: number; right: number; top: number; bottom: number } | null };
}
export interface CheckpointMenu { parent: CheckpointEndpoint | null; launcher: CheckpointEndpoint | null; panel: CheckpointEndpoint | null; disposition: 'reacquire' }
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
    origin: { instanceId: string; templateArtifactId: string; dataRevision: string; scopePolicyStamp: string };
    entries: (
        | { kind: 'element'; nodeId: string; tag: string; namespace: string | null; attributes: { name: string; value: string }[] }
        | { kind: 'text' | 'comment'; nodeId: string; text: string }
        | { kind: 'end' }
        | { kind: 'range-start' | 'range-end'; nodeId: string; rangeId: string }
    )[];
}
