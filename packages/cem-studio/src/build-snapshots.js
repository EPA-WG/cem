import { createCemStudioPreview } from './preview.js';

// Derived control records only: no native tree, runtime handles or browser leases.
export const CEM_STUDIO_PIN_LIMITS = Object.freeze({ count: 8, totalBytes: 64 * 1024 * 1024,
    artifactCount: 128, artifactBytes: 16 * 1024 * 1024, metadataBytes: 1024 * 1024 });

function invalid(message) {
    const error = new Error(message);
    error.code = 'cem.studio.snapshot.invalid';
    throw error;
}
const nonempty = value => typeof value === 'string' && value.length > 0;
const digest = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
const revision = value => Number.isSafeInteger(value) && value > 0;

/** Select the portable build control boundary; never serialize the live projection. */
export function captureCemStudioBuildSnapshot(projection) {
    if (projection?.exitCode !== 0 || !projection.buildRun) invalid('Only successful complete builds can be pinned.');
    const { id, projectId, projectRootUri, projectRevision, configuration, inputs, status, executionIdentity, admission, byteLength } = projection.buildRun;
    const artifacts = projection.buildRun.artifacts.map(({ id, uri, path, contentType, byteLength, sha256, available, provenance, sourceMaps }) =>
        ({ id, uri, path, contentType, byteLength, sha256, available, provenance, sourceMaps }));
    return { version: 1, run: { id, projectId, projectRootUri, projectRevision, configuration, inputs, status,
        executionIdentity, admission, byteLength, artifacts }, outputs: projection.outputs.map(({ uri, bytes }) => ({ uri, bytes: new Uint8Array(bytes) })) };
}

/** Verify and clone all bytes before publishing any artifact. Called on write AND read. */
export async function verifyCemStudioBuildSnapshot(value, crypto = globalThis.crypto) {
    const input = structuredClone(value);
    const run = input?.run;
    if (input?.version !== 1 || !run || !nonempty(run.id) || !nonempty(run.projectId) || !nonempty(run.projectRootUri)
        || !revision(run.projectRevision) || run.status !== 'succeeded'
        || !run.executionIdentity || typeof run.executionIdentity !== 'object'
        || !nonempty(run.configuration?.kind) || !nonempty(run.configuration?.id) || !nonempty(run.configuration?.name) || !revision(run.configuration?.revision)
        || !digest(run.configuration?.sha256) || !Array.isArray(run.inputs) || run.inputs.length > 129
        || run.inputs.some(item => !nonempty(item.resourceId) || !nonempty(item.uri) || !nonempty(item.path) || !revision(item.revision) || !digest(item.sha256))
        || !Array.isArray(run.artifacts) || !run.artifacts.length || run.artifacts.length > CEM_STUDIO_PIN_LIMITS.artifactCount
        || !Array.isArray(input.outputs) || input.outputs.length !== run.artifacts.length) invalid('The pinned build manifest is incomplete or unsupported.');
    // JSON is the persisted control DTO, never an AST projection. Strip absent fields,
    // and reject runtime objects instead of silently reviving handles on reopening.
    const metadata = JSON.stringify(run, (_key, item) => {
        if (typeof item === 'number' && !Number.isFinite(item) || typeof item === 'bigint'
            || item && typeof item === 'object' && !Array.isArray(item) && Object.getPrototypeOf(item) !== Object.prototype) invalid('Pinned metadata must contain portable control values.');
        return item;
    });
    const metadataBytes = new TextEncoder().encode(JSON.stringify({ version: 1, run: JSON.parse(metadata), outputs: input.outputs.map(({ uri }) => ({ uri })) })).byteLength;
    if (metadataBytes > CEM_STUDIO_PIN_LIMITS.metadataBytes) invalid('Pinned build provenance exceeds the 1 MiB metadata limit.');
    const uris = new Set(), paths = new Set(), ids = new Set();
    let size = 0;
    const outputs = [];
    for (const artifact of run.artifacts) {
        if (!nonempty(artifact.id) || !nonempty(artifact.uri) || !nonempty(artifact.path) || !nonempty(artifact.contentType)
            || !digest(artifact.sha256) || artifact.available !== true || !Number.isSafeInteger(artifact.byteLength) || artifact.byteLength < 0
            || !Array.isArray(artifact.provenance) || !Array.isArray(artifact.sourceMaps)
            || uris.has(artifact.uri) || paths.has(artifact.path) || ids.has(artifact.id)) invalid('A pinned artifact is incomplete or ambiguous.');
        uris.add(artifact.uri); paths.add(artifact.path); ids.add(artifact.id);
        const output = input.outputs.find(item => item.uri === artifact.uri);
        if (!(output?.bytes instanceof Uint8Array || output?.bytes instanceof ArrayBuffer)) invalid('Pinned artifact bytes are missing.');
        const bytes = new Uint8Array(output.bytes);
        size += bytes.byteLength;
        if (size > CEM_STUDIO_PIN_LIMITS.artifactBytes || bytes.byteLength !== artifact.byteLength) invalid('Pinned artifact lengths exceed the complete-build limits.');
        const hash = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), byte => byte.toString(16).padStart(2, '0')).join('');
        if (hash !== artifact.sha256) invalid('Pinned artifact SHA-256 verification failed.');
        outputs.push({ uri: artifact.uri, bytes: bytes.buffer });
    }
    if (size !== run.byteLength) invalid('Pinned build total length does not match its artifacts.');
    return { version: 1, run: JSON.parse(metadata), outputs, storageBytes: size + metadataBytes };
}

export function cemStudioBuildSnapshotSummary(record) {
    return { key: record.key, projectId: record.projectId, createdAt: record.createdAt,
        runId: record.run.id, configurationId: record.run.configuration.id,
        name: `${record.run.configuration.name} · ${record.createdAt} · ${record.key.slice(-8)}`,
        byteLength: record.run.byteLength, projectRevision: record.run.projectRevision };
}

/** Restored run ownership: selection and transfers only, shared preview owns rendering. */
export function openCemStudioBuildSnapshot(record) {
    const outputs = record.outputs.map(output => {
        const artifact = record.run.artifacts.find(item => item.uri === output.uri);
        const bytes = new Uint8Array(output.bytes);
        return { ...artifact, bytes, text: undefined };
    });
    let disposed = false, projection = { kind: 'snapshot', mode: 'pinned', exitCode: 0, buildRun: record.run, outputs,
        executionIdentity: record.run.executionIdentity, diagnostics: [], stale: false };
    return {
        snapshot: () => ({ projection }),
        selectArtifact(id) {
            const artifact = record.run.artifacts.find(item => item.id === id);
            const output = outputs.find(item => item.uri === artifact?.uri);
            if (!output) throw new Error('The pinned artifact is unavailable.');
            projection = { ...projection, selectedArtifactId: id, output,
                preview: createCemStudioPreview({ ...output, label: artifact.path }), provenance: artifact.provenance, transfer: undefined };
        },
        async copyProjection(writeText) {
            const selected = projection;
            try {
                if (!selected.output || selected.preview.kind === 'download' || typeof writeText !== 'function') throw new Error('Exact text copying is unavailable for this artifact.');
                await writeText(new TextDecoder('utf-8', { fatal: true }).decode(selected.output.bytes));
                if (!disposed && projection === selected) projection = { ...selected, transfer: { status: 'copied', message: 'Exact pinned output copied.' } };
            } catch (error) {
                if (!disposed && projection === selected) projection = { ...selected, transfer: { status: 'failed', message: error.message } };
            }
        },
        async downloadProjection(save) {
            const selected = projection;
            try {
                if (!selected.output || typeof save !== 'function') throw new Error('Exact artifact download is unavailable.');
                const output = selected.output;
                const filename = output.uri.startsWith('cem-stdio:') ? 'pinned-output' : output.path.split('/').pop();
                await save({ filename, contentType: output.contentType, bytes: new Uint8Array(output.bytes) });
                if (!disposed && projection === selected) projection = { ...selected, transfer: { status: 'downloaded', message: `Downloaded ${filename}.` } };
            } catch (error) {
                if (!disposed && projection === selected) projection = { ...selected, transfer: { status: 'failed', message: error.message } };
            }
        },
        dispose() { disposed = true; },
    };
}
