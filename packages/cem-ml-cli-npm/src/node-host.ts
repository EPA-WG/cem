import { randomUUID } from 'node:crypto';
import { mkdir, open, readFile, readdir, rename, rm } from 'node:fs/promises';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import type {
    CommandPreparedWriteTokenV1,
    CommandResolvedResourceV1,
    CommandResolvedWriteV1,
    CommandResourceReadRequestV1,
    CommandResourceWriteRequestV1,
    CommandRevisionLedgerRequestV1,
    CommandRevisionLedgerV1,
    CommandServiceHostCapabilitiesV1,
    CommandServiceRequestV1,
    FormatIdentity,
} from '@epa-wg/cem-ml/wasm';

const DEFAULT_MAX_HTTPS_BYTES = 32 * 1024 * 1024;
const STDOUT_URI = 'cem-stdio://stdout';
const STDERR_URI = 'cem-stdio://stderr';

export type NodeCommandReadable = AsyncIterable<Uint8Array | string>;

export interface NodeCommandWritable {
    write(chunk: Uint8Array): boolean;
    once(event: 'drain', listener: () => void): unknown;
}

export interface NodeCommandResolverMap {
    readonly uriPrefix: string;
    readonly localRoot: string;
}

export interface NodeCommandHostOptions {
    readonly cwd?: string;
    readonly fetch?: typeof globalThis.fetch;
    readonly maxHttpsBytes?: number;
    readonly readStreams?: Readonly<Record<string, NodeCommandReadable>>;
    readonly writeStreams?: Readonly<Record<string, NodeCommandWritable>>;
    readonly readMaps?: readonly NodeCommandResolverMap[];
    readonly writeMaps?: readonly NodeCommandResolverMap[];
    readonly stdout?: NodeCommandWritable;
    readonly stderr?: NodeCommandWritable;
    readonly deferStreamCommits?: boolean;
}

export interface NodeCommandPresentationWrite {
    readonly target: 'stdout' | 'stderr' | 'file';
    readonly uri?: string | null;
    readonly bytes: readonly number[] | Uint8Array;
}

type PreparedWrite =
    | {
          readonly kind: 'file';
          readonly requestId: string;
          readonly uri: string;
          readonly destination: string;
          readonly temporary: string;
          committed: boolean;
          backup?: string;
      }
    | {
          readonly kind: 'stream';
          readonly requestId: string;
          readonly uri: string;
          readonly stream: NodeCommandWritable;
          readonly bytes: Uint8Array;
          committed: boolean;
      };

export class NodeCommandHost implements CommandServiceHostCapabilitiesV1 {
    readonly cwd: string;

    #fetch: typeof globalThis.fetch;
    #maxHttpsBytes: number;
    #readStreams: Readonly<Record<string, NodeCommandReadable>>;
    #writeStreams: Readonly<Record<string, NodeCommandWritable>>;
    #readMaps: readonly NodeCommandResolverMap[];
    #writeMaps: readonly NodeCommandResolverMap[];
    #deferStreamCommits: boolean;
    #committedStreams = new Map<string, Array<{ readonly token: string; readonly bytes: Uint8Array }>>();
    #ledgers = new Map<string, CommandRevisionLedgerV1>();
    #prepared = new Map<string, PreparedWrite>();

    constructor(options: NodeCommandHostOptions = {}) {
        this.cwd = resolve(options.cwd ?? process.cwd());
        this.#fetch = options.fetch ?? globalThis.fetch;
        this.#maxHttpsBytes = options.maxHttpsBytes ?? DEFAULT_MAX_HTTPS_BYTES;
        if (!Number.isSafeInteger(this.#maxHttpsBytes) || this.#maxHttpsBytes <= 0) {
            throw new RangeError('maxHttpsBytes must be a positive safe integer');
        }
        this.#readStreams = options.readStreams ?? {};
        this.#writeStreams = {
            ...(options.writeStreams ?? {}),
            ...(options.stdout === undefined ? {} : { [STDOUT_URI]: options.stdout }),
            ...(options.stderr === undefined ? {} : { [STDERR_URI]: options.stderr }),
        };
        this.#readMaps = sortedMaps(options.readMaps ?? []);
        this.#writeMaps = sortedMaps(options.writeMaps ?? []);
        this.#deferStreamCommits = options.deferStreamCommits ?? false;
    }

    trackRequest(request: CommandServiceRequestV1): void {
        this.#ledgers.set(request.requestId, {
            project: request.project,
            resourceVersions: request.resourceVersions,
        });
    }

    async releaseRequest(requestId: string): Promise<void> {
        this.#ledgers.delete(requestId);
        const cleanup: Promise<void>[] = [];
        for (const [token, prepared] of this.#prepared) {
            if (prepared.requestId !== requestId) continue;
            this.#prepared.delete(token);
            if (prepared.kind === 'file') {
                if (prepared.backup !== undefined) cleanup.push(rm(prepared.backup, { force: true }));
                if (!prepared.committed) cleanup.push(rm(prepared.temporary, { force: true }));
            }
        }
        await Promise.all(cleanup);
    }

    currentRevision = (request: CommandRevisionLedgerRequestV1): CommandRevisionLedgerV1 => {
        const ledger = this.#ledgers.get(request.requestId);
        if (ledger === undefined) {
            throw hostError(
                'cem.node_host.revision_unknown',
                `no revision ledger is registered for request ${request.requestId}`,
            );
        }
        return ledger;
    };

    readResource = async (request: CommandResourceReadRequestV1): Promise<CommandResolvedResourceV1> => {
        const bytes = await this.read(request.uri);
        return {
            version: request.expected,
            bytes: [...bytes],
            identity: resourceIdentity(request),
        };
    };

    prepareWrite = async (
        request: CommandResourceWriteRequestV1,
        bytes: Uint8Array,
    ): Promise<CommandPreparedWriteTokenV1> => {
        const token = randomUUID();
        const stream = this.#writeStreams[request.uri];
        if (stream !== undefined) {
            this.#prepared.set(token, {
                kind: 'stream',
                requestId: request.requestId,
                uri: request.uri,
                stream,
                bytes: new Uint8Array(bytes),
                committed: false,
            });
            return { token };
        }
        const destination = this.#writePath(request.uri);
        await mkdir(dirname(destination), { recursive: true });
        const temporary = `${destination}.cem-ml-${token}.tmp`;
        const handle = await open(temporary, 'wx');
        try {
            await handle.writeFile(bytes);
            await handle.sync();
        } finally {
            await handle.close();
        }
        this.#prepared.set(token, {
            kind: 'file',
            requestId: request.requestId,
            uri: request.uri,
            destination,
            temporary,
            committed: false,
        });
        return { token };
    };

    commitWrite = async (token: string): Promise<CommandResolvedWriteV1> => {
        const prepared = this.#prepared.get(token);
        if (prepared === undefined) {
            throw hostError('cem.node_host.write_token_unknown', `unknown write token ${token}`);
        }
        if (prepared.kind === 'file') {
            const backup = `${prepared.temporary}.previous`;
            try {
                await rename(prepared.destination, backup);
                prepared.backup = backup;
            } catch (error) {
                if (!isNodeError(error, 'ENOENT')) throw error;
            }
            try {
                await rename(prepared.temporary, prepared.destination);
            } catch (error) {
                if (prepared.backup !== undefined) {
                    await rename(prepared.backup, prepared.destination);
                    prepared.backup = undefined;
                }
                throw error;
            }
        } else if (this.#deferStreamCommits) {
            const prior = this.#committedStreams.get(prepared.uri) ?? [];
            prior.push({ token, bytes: new Uint8Array(prepared.bytes) });
            this.#committedStreams.set(prepared.uri, prior);
        } else {
            await writeStream(prepared.stream, prepared.bytes);
        }
        prepared.committed = true;
        return { uri: prepared.uri };
    };

    rollbackWrite = async (token: string): Promise<void> => {
        const prepared = this.#prepared.get(token);
        if (prepared === undefined) return;
        this.#prepared.delete(token);
        if (prepared.kind === 'file') {
            if (prepared.committed) {
                await rm(prepared.destination, { force: true });
                if (prepared.backup !== undefined) await rename(prepared.backup, prepared.destination);
            } else {
                await rm(prepared.temporary, { force: true });
            }
        } else if (prepared.committed && this.#deferStreamCommits) {
            const retained = (this.#committedStreams.get(prepared.uri) ?? []).filter(
                (chunk) => chunk.token !== token,
            );
            if (retained.length === 0) this.#committedStreams.delete(prepared.uri);
            else this.#committedStreams.set(prepared.uri, retained);
        } else if (prepared.committed) {
            throw hostError(
                'cem.node_host.stream_rollback_unavailable',
                `committed stream ${prepared.uri} cannot be rolled back`,
            );
        }
    };

    async read(uri: string): Promise<Uint8Array> {
        const stream = this.#readStreams[uri];
        if (stream !== undefined) return readStream(stream, this.#maxHttpsBytes);
        if (uri.startsWith('https://')) return this.#readHttps(uri);
        if (uri.startsWith('http://')) {
            throw hostError('cem.node_host.https_required', `insecure HTTP resource ${uri} is not allowed`);
        }
        return new Uint8Array(await readFile(this.#readPath(uri)));
    }

    async expandGlob(uri: string): Promise<readonly string[]> {
        if (uri.startsWith('https://') || uri.startsWith('http://')) {
            throw hostError('cem.node_host.glob_unavailable', `URL glob listing is unavailable for ${uri}`);
        }
        const mapping = this.#readMaps.find(({ uriPrefix }) => uri.startsWith(uriPrefix));
        const localPattern =
            mapping === undefined ? localPath(uri, this.cwd) : mappedPath(uri, [mapping], true);
        if (localPattern === undefined) return [];
        const root = globRoot(localPattern);
        const matcher = globMatcher(localPattern);
        const matches: string[] = [];
        await collectFiles(root, matches);
        const selected = matches.filter((candidate) => matcher.test(candidate)).sort();
        if (selected.length === 0) {
            throw hostError('cem.node_host.glob_empty', `resource glob ${uri} matched no files`);
        }
        if (selected.length > 1024) {
            throw hostError('cem.node_host.glob_too_many', `resource glob ${uri} matched more than 1024 files`);
        }
        if (mapping === undefined) return selected.map((candidate) => candidate.replaceAll(sep, '/'));
        return selected.map((candidate) => {
            const suffix = relative(resolve(mapping.localRoot), candidate).replaceAll(sep, '/');
            return `${mapping.uriPrefix.replace(/\/$/, '')}/${suffix}`;
        });
    }

    takeCommittedStream(uri: string): Uint8Array {
        const chunks = this.#committedStreams.get(uri) ?? [];
        this.#committedStreams.delete(uri);
        return concatenate(chunks.map(({ bytes }) => bytes));
    }

    async publishPresentation(writes: readonly NodeCommandPresentationWrite[]): Promise<void> {
        const fileWrites = writes.filter((write) => write.target === 'file');
        const prepared: Array<{ readonly destination: string; readonly temporary: string }> = [];
        try {
            for (const write of fileWrites) {
                if (typeof write.uri !== 'string' || write.uri.length === 0) {
                    throw hostError('cem.node_host.presentation_uri', 'presentation file write requires a URI');
                }
                const destination = this.#writePath(write.uri);
                await mkdir(dirname(destination), { recursive: true });
                const temporary = `${destination}.cem-ml-${randomUUID()}.tmp`;
                const handle = await open(temporary, 'wx');
                try {
                    await handle.writeFile(new Uint8Array(write.bytes));
                    await handle.sync();
                } finally {
                    await handle.close();
                }
                prepared.push({ destination, temporary });
            }
            for (const write of prepared) await rename(write.temporary, write.destination);
        } catch (error) {
            await Promise.all(prepared.map(({ temporary }) => rm(temporary, { force: true })));
            throw error;
        }
        for (const write of writes.filter((candidate) => candidate.target !== 'file')) {
            const uri = write.target === 'stdout' ? STDOUT_URI : STDERR_URI;
            const stream = this.#writeStreams[uri];
            if (stream === undefined) {
                throw hostError('cem.node_host.stream_unavailable', `${write.target} stream is unavailable`);
            }
            await writeStream(stream, new Uint8Array(write.bytes));
        }
    }

    #readPath(uri: string): string {
        return mappedPath(uri, this.#readMaps) ?? localPath(uri, this.cwd);
    }

    #writePath(uri: string): string {
        return mappedPath(uri, this.#writeMaps) ?? localPath(uri, this.cwd);
    }

    async #readHttps(uri: string): Promise<Uint8Array> {
        const response = await this.#fetch(uri, { redirect: 'follow' });
        if (!response.ok) {
            throw hostError(
                'cem.node_host.https_status',
                `HTTPS resource ${uri} returned ${response.status} ${response.statusText}`,
            );
        }
        const declared = Number(response.headers.get('content-length'));
        if (Number.isFinite(declared) && declared > this.#maxHttpsBytes) {
            throw hostError('cem.node_host.resource_too_large', `HTTPS resource ${uri} exceeds the byte limit`);
        }
        const bytes = new Uint8Array(await response.arrayBuffer());
        if (bytes.byteLength > this.#maxHttpsBytes) {
            throw hostError('cem.node_host.resource_too_large', `HTTPS resource ${uri} exceeds the byte limit`);
        }
        return bytes;
    }
}

function resourceIdentity(request: CommandResourceReadRequestV1): FormatIdentity | null {
    const contentType = [...request.contentTypeHints].sort()[0];
    return contentType === undefined
        ? null
        : {
              contentType,
              schema: null,
              defaultNamespace: null,
              namespaces: {},
              baseUri: request.uri,
          };
}

function localPath(uri: string, cwd: string): string {
    if (uri.startsWith('file://')) return fileURLToPath(uri);
    if (hasScheme(uri)) {
        throw hostError('cem.node_host.resolver_unavailable', `no Node resolver is installed for ${uri}`);
    }
    return isAbsolute(uri) ? uri : resolve(cwd, uri);
}

function mappedPath(
    uri: string,
    maps: readonly NodeCommandResolverMap[],
    allowGlob = false,
): string | undefined {
    const mapping = maps.find(({ uriPrefix }) => uri.startsWith(uriPrefix));
    if (mapping === undefined) return undefined;
    const suffix = uri.slice(mapping.uriPrefix.length).replace(/^\/+/, '');
    const segments = suffix.split('/').filter((segment) => segment.length > 0 && segment !== '.');
    if (
        segments.includes('..') ||
        suffix.includes('\\') ||
        (!allowGlob && suffix.includes('?')) ||
        suffix.includes('#')
    ) {
        throw hostError('cem.node_host.resolver_escape', `mapped URI ${uri} escapes its local root`);
    }
    return resolve(mapping.localRoot, ...segments);
}

function sortedMaps(maps: readonly NodeCommandResolverMap[]): readonly NodeCommandResolverMap[] {
    return [...maps].sort((left, right) => right.uriPrefix.length - left.uriPrefix.length);
}

async function readStream(stream: NodeCommandReadable, maximum: number): Promise<Uint8Array> {
    const chunks: Uint8Array[] = [];
    let length = 0;
    for await (const chunk of stream) {
        const bytes = typeof chunk === 'string' ? new TextEncoder().encode(chunk) : new Uint8Array(chunk);
        length += bytes.byteLength;
        if (length > maximum) throw hostError('cem.node_host.resource_too_large', 'stream exceeds the byte limit');
        chunks.push(bytes);
    }
    const output = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) {
        output.set(chunk, offset);
        offset += chunk.byteLength;
    }
    return output;
}

function writeStream(stream: NodeCommandWritable, bytes: Uint8Array): Promise<void> {
    if (stream.write(bytes)) return Promise.resolve();
    return new Promise((resolve) => stream.once('drain', resolve));
}

function hasScheme(uri: string): boolean {
    return /^[A-Za-z][A-Za-z0-9+.-]*:/.test(uri);
}

function globRoot(pattern: string): string {
    const first = pattern.search(/[?*{]/);
    if (first < 0) return dirname(pattern);
    const prefix = pattern.slice(0, first);
    return prefix.endsWith('/') || prefix.endsWith('\\') ? prefix.slice(0, -1) : dirname(prefix);
}

function globMatcher(pattern: string): RegExp {
    let source = '^';
    for (let index = 0; index < pattern.length; index += 1) {
        const character = pattern[index] ?? '';
        if (character === '*' && pattern[index + 1] === '*') {
            source += '.*';
            index += 1;
        } else if (character === '*') {
            source += '[^/\\\\]*';
        } else if (character === '?') {
            source += '[^/\\\\]';
        } else if (character === '{') {
            const close = pattern.indexOf('}', index + 1);
            if (close < 0) throw hostError('cem.node_host.glob_invalid', `unclosed binding in ${pattern}`);
            source += '[^/\\\\]+';
            index = close;
        } else {
            source += character.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        }
    }
    return new RegExp(`${source}$`);
}

async function collectFiles(directory: string, files: string[]): Promise<void> {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const path = resolve(directory, entry.name);
        if (entry.isDirectory()) await collectFiles(path, files);
        else if (entry.isFile()) files.push(path);
    }
}

function concatenate(chunks: readonly Uint8Array[]): Uint8Array {
    const output = new Uint8Array(chunks.reduce((sum, chunk) => sum + chunk.byteLength, 0));
    let offset = 0;
    for (const chunk of chunks) {
        output.set(chunk, offset);
        offset += chunk.byteLength;
    }
    return output;
}

function hostError(code: string, message: string): Error & { readonly code: string } {
    return Object.assign(new Error(message), { code });
}

function isNodeError(error: unknown, code: string): boolean {
    return error instanceof Error && 'code' in error && error.code === code;
}

export function createNodeCommandHost(options: NodeCommandHostOptions = {}): NodeCommandHost {
    return new NodeCommandHost(options);
}
