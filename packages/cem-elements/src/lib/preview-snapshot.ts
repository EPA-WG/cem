import { copyCemArtifactTree, registerCemArtifactTree, type CemArtifactTree, type CemPreviewArtifact } from './artifact-preview.js';

/** Selection facts only: no runtime, provider, credentials or execution permits. */
export interface CemPreviewSnapshotIdentity {
    label: string;
    ownerId?: string;
    configurationId?: string;
    runId?: string;
    projectRevision?: number;
    sourceRevision?: number;
    draft?: boolean;
    stale?: boolean;
    pinned?: boolean;
}
export type CemPreviewSnapshotValue =
    | { mode: 'text'; value: string; truncated?: boolean }
    | { mode: 'document'; value: string }
    | { mode: 'data'; format: 'json' | 'csv'; value: string };
export type CemPreviewSnapshotDescriptor = CemPreviewSnapshotValue
    | { mode: 'artifact-tree'; resourceSet: string; artifactPath: string };
export interface CemPreviewSnapshotInput {
    identity: CemPreviewSnapshotIdentity;
    preview: CemPreviewSnapshotDescriptor | { mode: 'artifact-tree'; artifacts: readonly CemPreviewArtifact[]; artifactPath: string };
}
export interface CemPreviewSnapshot {
    readonly identity: Readonly<CemPreviewSnapshotIdentity>;
    readonly preview: Readonly<CemPreviewSnapshotDescriptor>;
    readonly byteLength: number;
    readonly resourceCount: number;
    readonly signal: AbortSignal;
    retain(): CemPreviewSnapshot;
    dispose(): void;
}
export interface CemPreviewSnapshotStore {
    capture(input: CemPreviewSnapshotInput): CemPreviewSnapshot;
    usage(): Readonly<{ snapshots: number; leases: number; byteLength: number }>;
    dispose(): void;
}
export interface CemPreviewSnapshotStoreOptions { maxSnapshots?: number; maxBytes?: number }
export interface CemPreviewSnapshotEnvelope {
    schemaVersion: 1;
    identity: CemPreviewSnapshotIdentity;
    preview: CemPreviewSnapshotValue | { mode: 'artifact-tree'; artifacts: readonly CemPreviewArtifact[]; artifactPath: string };
    byteLength: number;
    resourceCount: number;
}
const maxBytes = 16 * 1024 * 1024, inlineBytes = 256 * 1024, encoder = new TextEncoder();
const exports = new WeakMap<CemPreviewSnapshot, () => CemPreviewSnapshotEnvelope>();
function fail(reason: string): never { throw new Error(`cem.preview.snapshot_${reason}`); }
function object(value: unknown): asserts value is Record<string, unknown> {
    if (!value || typeof value !== 'object' || ![Object.prototype, null].includes(Object.getPrototypeOf(value))) fail('invalid');
}
function keys(value: Record<string, unknown>, allowed: readonly string[]): void {
    if (Object.keys(value).some(key => !allowed.includes(key))) fail('invalid');
}
function string(value: unknown, limit = 4096): asserts value is string {
    if (typeof value !== 'string' || !value || value.length > limit) fail('invalid');
}
function identity(value: unknown): Readonly<CemPreviewSnapshotIdentity> {
    object(value);
    const strings = ['label', 'ownerId', 'configurationId', 'runId'];
    const numbers = ['projectRevision', 'sourceRevision'], flags = ['draft', 'stale', 'pinned'];
    keys(value, [...strings, ...numbers, ...flags]); string(value.label);
    for (const key of strings) if (value[key] !== undefined) string(value[key]);
    for (const key of numbers) if (value[key] !== undefined && (!Number.isSafeInteger(value[key]) || Number(value[key]) < 0)) fail('invalid');
    for (const key of flags) if (value[key] !== undefined && typeof value[key] !== 'boolean') fail('invalid');
    if (encoder.encode(JSON.stringify(value)).byteLength > 16 * 1024) fail('limit');
    return Object.freeze({ ...value }) as unknown as Readonly<CemPreviewSnapshotIdentity>;
}
interface RecordOwner {
    identity: Readonly<CemPreviewSnapshotIdentity>;
    preview?: Readonly<CemPreviewSnapshotDescriptor>;
    tree?: CemArtifactTree;
    byteLength: number;
    resourceCount: number;
    references: number;
    controller: AbortController;
}
/** A store owns bounded immutable copies. Leases share one charged registration. */
export function createCemPreviewSnapshotStore(options: CemPreviewSnapshotStoreOptions = {}): CemPreviewSnapshotStore {
    const capacity = options.maxSnapshots ?? 1, budget = options.maxBytes ?? maxBytes;
    if (!Number.isSafeInteger(capacity) || capacity < 1 || capacity > 8 || !Number.isSafeInteger(budget) || budget < 1 || budget > maxBytes) fail('limit');
    const records = new Set<RecordOwner>();
    let disposed = false, charged = 0;
    const release = (record: RecordOwner) => {
        if (!records.delete(record)) return;
        charged -= record.byteLength; record.tree?.dispose(); record.tree = undefined; record.preview = undefined;
        record.controller.abort(new Error('cem.preview.snapshot_released'));
    };
    function lease(record: RecordOwner): CemPreviewSnapshot {
        let active = true;
        const current = () => { const view = record.preview; if (!active || !records.has(record) || !view) fail('released'); return view; };
        record.references++;
        const held: CemPreviewSnapshot = Object.freeze({
            get identity() { current(); return record.identity; },
            get preview() { return current(); },
            get byteLength() { current(); return record.byteLength; },
            get resourceCount() { current(); return record.resourceCount; },
            get signal() { return record.controller.signal; },
            retain() { current(); return lease(record); },
            dispose() { if (!active) return; active = false; exports.delete(held); if (--record.references === 0) release(record); },
        });
        exports.set(held, () => {
            const view = current();
            return { schemaVersion: 1, identity: { ...record.identity }, byteLength: record.byteLength, resourceCount: record.resourceCount,
                preview: view.mode === 'artifact-tree' ? { mode: 'artifact-tree', artifactPath: view.artifactPath, artifacts: copyCemArtifactTree(view.resourceSet) } : { ...view } };
        });
        return held;
    }
    return Object.freeze({
        capture(input: CemPreviewSnapshotInput) {
            if (disposed) fail('released');
            if (records.size >= capacity) fail('capacity');
            object(input); keys(input as unknown as Record<string, unknown>, ['identity', 'preview']);
            const facts = identity(input.identity), view = input.preview;
            object(view);
            let bytes = 0, count = 1, tree: CemArtifactTree | undefined, output: CemPreviewSnapshotDescriptor;
            if (view.mode === 'artifact-tree') {
                keys(view, ['mode', 'resourceSet', 'artifactPath', 'artifacts']); string(view.artifactPath);
                if (('resourceSet' in view) === ('artifacts' in view)) fail('invalid');
                const artifacts = 'resourceSet' in view ? copyCemArtifactTree(view.resourceSet) : view.artifacts;
                if (!Array.isArray(artifacts) || !artifacts.length || artifacts.length > 128) fail('limit');
                count = artifacts.length;
                for (const item of artifacts) {
                    object(item); keys(item, ['path', 'contentType', 'bytes']); string(item.path); string(item.contentType, 128);
                    if (!ArrayBuffer.isView(item.bytes) || Object.prototype.toString.call(item.bytes) !== '[object Uint8Array]') fail('invalid');
                    bytes += item.bytes.byteLength;
                    if (charged + bytes > budget) fail('capacity');
                }
                const file = view.artifactPath.split(/[?#]/)[0], selected = artifacts.find(item => item.path === file);
                if (!selected || !['text/html', 'application/xhtml+xml'].includes(selected.contentType.split(';')[0].trim().toLowerCase())) fail('invalid');
                tree = registerCemArtifactTree(artifacts);
                output = { mode: 'artifact-tree', artifactPath: view.artifactPath, resourceSet: tree.resourceSet };
            } else {
                if (!['text', 'document', 'data'].includes(view.mode)) fail('invalid');
                keys(view, view.mode === 'text' ? ['mode', 'value', 'truncated'] : view.mode === 'data' ? ['mode', 'value', 'format'] : ['mode', 'value']);
                if (typeof view.value !== 'string') fail('invalid');
                if (view.value.length > inlineBytes) fail('limit');
                bytes = encoder.encode(view.value).byteLength;
                if (bytes > inlineBytes) fail('limit');
                if (view.mode === 'text' && view.truncated !== undefined && typeof view.truncated !== 'boolean') fail('invalid');
                if (view.mode === 'data' && !['json', 'csv'].includes(view.format)) fail('invalid');
                if (view.mode === 'data' && (view.value.length > 131072 || [...view.value].length > 65536)) fail('limit');
                if (charged + bytes > budget) fail('capacity');
                output = { ...view };
            }
            const record: RecordOwner = { identity: facts, preview: Object.freeze(output), tree, byteLength: bytes, resourceCount: count, references: 0, controller: new AbortController() };
            records.add(record); charged += bytes;
            return lease(record);
        },
        usage: () => Object.freeze({ snapshots: records.size, leases: [...records].reduce((sum, record) => sum + record.references, 0), byteLength: charged }),
        dispose() { if (disposed) return; disposed = true; for (const record of records) release(record); },
    });
}
/** Explicit byte/control transport boundary, not a document AST projection. */
export function exportCemPreviewSnapshot(snapshot: CemPreviewSnapshot): CemPreviewSnapshotEnvelope {
    const read = exports.get(snapshot);
    if (!read) fail('released');
    return read();
}
export function importCemPreviewSnapshot(store: CemPreviewSnapshotStore, value: unknown): CemPreviewSnapshot {
    object(value); keys(value, ['schemaVersion', 'identity', 'preview', 'byteLength', 'resourceCount']);
    if (value.schemaVersion !== 1 || !Number.isSafeInteger(value.byteLength) || !Number.isSafeInteger(value.resourceCount)) fail('invalid');
    object(value.preview);
    if (value.preview.mode === 'artifact-tree') {
        // A transport cannot borrow a receiver-local handle or hide uncharged
        // backing storage behind a short view. Exported copies have exact buffers.
        if ('resourceSet' in value.preview || !Array.isArray(value.preview.artifacts) || value.preview.artifacts.length > 128) fail('invalid');
        for (const artifact of value.preview.artifacts) {
            object(artifact);
            const bytes = artifact.bytes;
            if (!ArrayBuffer.isView(bytes) || Object.prototype.toString.call(bytes) !== '[object Uint8Array]'
                || bytes.byteOffset !== 0 || bytes.byteLength !== bytes.buffer.byteLength
                || Object.prototype.toString.call(bytes.buffer) !== '[object ArrayBuffer]') fail('invalid');
        }
    }
    const snapshot = store.capture({ identity: value.identity, preview: value.preview } as CemPreviewSnapshotInput);
    if (snapshot.byteLength !== value.byteLength || snapshot.resourceCount !== value.resourceCount) { snapshot.dispose(); fail('invalid'); }
    return snapshot;
}
