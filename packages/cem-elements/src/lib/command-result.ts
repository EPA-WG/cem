/** A published command artifact; this module has no DOM or CLI runtime dependency. */
export interface CommandOutputMetadata {
    readonly handleId?: number;
    readonly sourceMapId?: string;
    readonly uri: string;
    readonly kind: string;
    readonly contentType: string;
    readonly byteLength: number;
    readonly sha256: string;
}

export interface CommandOutput {
    readonly handleId?: number;
    readonly sourceMapId?: string;
    /** False means a failed command published no output, including no empty artifact. */
    readonly available: boolean;
    readonly uri: string;
    readonly contentType: string;
    readonly byteLength: number;
    readonly sha256: string;
    readonly bytes: Uint8Array;
    readonly text: string;
}

export interface CommandOutputReadOptions {
    readonly requestedUri?: string;
    readonly maxBytes: number;
    readonly chunkBytes?: number;
    /** Disable compatibility fallback when reading a particular publication. */
    readonly fallbackToFirstOutput?: boolean;
    readonly committedPublications?: readonly {
        readonly request: CommandOutputMetadata;
        readonly bytes: Uint8Array;
    }[];
}

/** Read and verify bounded command bytes without inventing an output on failure. */
export async function readCommandOutput(
    handle: {
        readArtifact(artifact: CommandOutputMetadata, range: { offset: number; maxBytes: number }): Promise<{
            metadata: { offset: number };
            bytes: Uint8Array;
        }>;
    },
    result: { exitCode: number; artifacts?: { items?: readonly CommandOutputMetadata[] } },
    options: CommandOutputReadOptions,
): Promise<CommandOutput> {
    const { requestedUri = 'cem-stdio://stdout', maxBytes, chunkBytes = 1024 * 1024,
        committedPublications = [] } = options;
    if (!Number.isSafeInteger(maxBytes) || maxBytes < 0 || !Number.isSafeInteger(chunkBytes) || chunkBytes <= 0) {
        throw new Error('Command output limits must be safe nonnegative byte counts with a positive chunk size');
    }
    const artifacts = result.artifacts?.items ?? [];
    const artifact = artifacts.find(({ uri }) => uri === requestedUri)
        ?? (options.fallbackToFirstOutput !== false ? artifacts.find(({ kind }) => kind === 'output') : undefined);
    const publication = artifact ? undefined : committedPublications.find(({ request }) => request.uri === requestedUri)
        ?? (options.fallbackToFirstOutput !== false ? committedPublications.find(({ request }) => request.kind === 'output') : undefined);
    const metadata = artifact ?? publication?.request;
    if (!metadata) {
        if (!Number.isInteger(result.exitCode) || result.exitCode === 0) {
            throw new Error(`CEM-ML command omitted its ${requestedUri} output publication`);
        }
        return output(false, requestedUri, 'text/plain', new Uint8Array(0));
    }
    if (!Number.isSafeInteger(metadata.byteLength) || metadata.byteLength < 0 || metadata.byteLength > maxBytes) {
        throw new Error(`Command output length exceeds its ${maxBytes} byte limit or is invalid`);
    }
    let bytes: Uint8Array;
    if (artifact) {
        bytes = new Uint8Array(metadata.byteLength);
        let offset = 0;
        while (offset < metadata.byteLength) {
            const requestedBytes = Math.min(chunkBytes, maxBytes, metadata.byteLength - offset);
            const read = await handle.readArtifact(artifact, { offset, maxBytes: requestedBytes });
            if (read.metadata.offset !== offset || read.bytes.byteLength === 0 || read.bytes.byteLength > requestedBytes) {
                throw new Error(`CEM-ML command returned an invalid artifact range at byte ${offset}`);
            }
            bytes.set(read.bytes, offset);
            offset += read.bytes.byteLength;
        }
    } else if (publication) {
        bytes = publication.bytes;
    } else {
        throw new Error('Command output publication disappeared');
    }
    if (bytes.byteLength !== metadata.byteLength) {
        throw new Error(`CEM-ML command output length ${bytes.byteLength} does not match ${metadata.byteLength}`);
    }
    const verified = await output(true, metadata.uri, metadata.contentType, bytes);
    if (verified.sha256 !== metadata.sha256) {
        throw new Error(`CEM-ML command output hash ${verified.sha256} does not match ${metadata.sha256}`);
    }
    return Object.freeze({ ...verified, handleId: metadata.handleId, sourceMapId: metadata.sourceMapId });
}

/** Verify the complete output set before returning any of it to the consumer. */
export async function readCommandOutputs(
    handle: Parameters<typeof readCommandOutput>[0],
    result: Parameters<typeof readCommandOutput>[1] & { artifacts?: { items?: readonly CommandOutputMetadata[]; originalCount?: number } },
    options: CommandOutputReadOptions & { readonly maxArtifacts: number },
): Promise<readonly CommandOutput[]> {
    if (!Number.isSafeInteger(options.maxArtifacts) || options.maxArtifacts < 0
        || !Number.isSafeInteger(options.maxBytes) || options.maxBytes < 0) {
        throw new Error('Command output set limits must be safe nonnegative counts');
    }
    const retained = result.artifacts?.items ?? [];
    if ((result.artifacts?.originalCount ?? retained.length) > retained.length) {
        throw new Error('Command artifact manifest is truncated');
    }
    const artifacts = retained.filter(({ kind }) => kind === 'output');
    const uris = new Set<string>();
    const publications = options.committedPublications ?? [];
    for (const artifact of artifacts) {
        if (!artifact.uri || uris.has(artifact.uri)) throw new Error('Command output manifest has a missing or duplicate URI');
        uris.add(artifact.uri);
    }
    // Retained manifests are authoritative; committed bytes are a fallback for
    // transports without retained handles, not a second set of publications.
    const committed = artifacts.length ? [] : publications.filter(({ request }) => request.kind === 'output');
    const metadata = [...artifacts, ...committed.map(({ request }) => request)];
    uris.clear();
    let total = 0;
    if (metadata.length > options.maxArtifacts) throw new Error('Command output count exceeds its artifact limit');
    for (const artifact of metadata) {
        if (!artifact.uri || uris.has(artifact.uri)) throw new Error('Command output manifest has a missing or duplicate URI');
        uris.add(artifact.uri);
        if (!Number.isSafeInteger(artifact.byteLength) || artifact.byteLength < 0) throw new Error('Command output length is invalid');
        total += artifact.byteLength;
        if (!Number.isSafeInteger(total) || total > options.maxBytes) throw new Error('Command output set exceeds its total byte limit');
    }
    const outputs: CommandOutput[] = [];
    for (const artifact of metadata) {
        outputs.push(await readCommandOutput(handle, result, { ...options, requestedUri: artifact.uri, fallbackToFirstOutput: false }));
    }
    if (!outputs.length && result.exitCode === 0) throw new Error('CEM-ML command omitted its output publications');
    return Object.freeze(outputs);
}

async function output(available: boolean, uri: string, contentType: string, source: Uint8Array): Promise<CommandOutput> {
    const bytes = source.slice();
    const digest = await crypto.subtle.digest('SHA-256', bytes.buffer);
    return Object.freeze({ available, uri, contentType, byteLength: bytes.byteLength, bytes,
        sha256: [...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, '0')).join(''),
        text: new TextDecoder().decode(bytes) });
}
