import type { CemHttpRequest, CemHttpResourceLoadResult, CemHttpResourcePolicy,
    CemResourceResolution, CemResourceResolutionRequest, CemResourceResolutionResult } from './cem-elements.js';
import { openCemHttpResource } from './http-resource-transport.js';

/** Optional native ABI only. These calls release all temporary native owners. */
export interface CemSchemaAssetNativeRuntime {
    schemaAssetPackBootstrapV1(): string;
    schemaAssetPackSummaryV1(index: Uint8Array, pin: string): string;
    schemaAssetPackImportV1(index: Uint8Array, pin: string, packageId: string, version: string, path: string, bytes: Uint8Array): string;
    commandNativeMemoryIdentity(): WebAssembly.Memory;
}
export interface CemSchemaAssetRuntime {
    native: CemSchemaAssetNativeRuntime;
    /** Host admission fence; a loader never grants recovery or runtime ownership. */
    current(): boolean;
}
export interface CemSchemaAssetPackOptions {
    baseUrl: string;
    runtime: CemSchemaAssetRuntime;
    resolveResourceUrl?: (request: CemResourceResolutionRequest) => CemResourceResolutionResult | Promise<CemResourceResolutionResult>;
    loadHttpResource?: (request: CemHttpRequest) => Promise<CemHttpResourceLoadResult>;
    policy?: Partial<CemHttpResourcePolicy>;
    /** May lower the shared 32 MiB admission bound. Includes retained index and pending reads. */
    maxResidentBytes?: number;
}
export interface CemSchemaAssetSourceIdentity { packageId: string; version: string; path: string }
interface SourceDescriptor extends CemSchemaAssetSourceIdentity { digest: string; byteLength: number; contentType: string }
interface Bootstrap { indexDigest: string; indexBytes: number; sourceCount: number; sourceBytes: number; packageCount: number }
interface ImportedSource extends CemSchemaAssetSourceIdentity {
    indexDigest: string; digest: string; sourceUri: string; sourceBytes: number; nodeCount: number;
}
export interface CemLoadedSchemaAssetSource extends ImportedSource { bytes: Uint8Array; contentType: string }
export class CemSchemaAssetLoadError extends Error {
    constructor(readonly code: string) { super(code); this.name = 'CemSchemaAssetLoadError'; }
}
function fail(code: string): never { throw new CemSchemaAssetLoadError(`cem.schema_asset.${code}`); }
const digestPattern = /^[a-f0-9]{64}$/u;
const key = (source: CemSchemaAssetSourceIdentity) => JSON.stringify([source.packageId, source.version, source.path]);
const mediaType = (value: string | null) => value?.split(';', 1)[0]?.trim().toLowerCase();
const MAX_INDEX = 4 * 1024 * 1024, MAX_SOURCE = 16 * 1024 * 1024, MAX_RESIDENT = 32 * 1024 * 1024;

/** Bounded original-byte acquisition, never a schema parser or activation API.
 * Only named native catalog/control DTOs cross JSON boundaries. Native code
 * authenticates the bootstrap/index and interprets every imported source.
 */
export class CemSchemaAssetPackLoader {
    private readonly options: Omit<CemSchemaAssetPackOptions, 'runtime'>;
    private endpoint?: CemSchemaAssetRuntime;
    private readonly base: URL;
    private readonly policy: CemHttpResourcePolicy;
    private readonly capacity: number;
    private readonly lifetime = new AbortController();
    private memory?: WebAssembly.Memory;
    private index?: { bytes: Uint8Array; pin: Bootstrap; sources: Map<string, SourceDescriptor> };
    private readonly retained = new Map<string, { descriptor: SourceDescriptor; bytes: Uint8Array }>();
    private residentBytes = 0;
    private pendingBytes = 0;
    private active = 0;

    constructor(options: CemSchemaAssetPackOptions) {
        const { runtime, ...transport } = options;
        this.options = transport;
        this.endpoint = { native: runtime.native, current: runtime.current.bind(runtime) };
        this.base = new URL(options.baseUrl);
        if (!['https:', 'http:'].includes(this.base.protocol) || this.base.username || this.base.password
            || this.base.search || this.base.hash || !this.base.pathname.endsWith('/')) fail('policy');
        this.policy = { allowCrossOrigin: false, maxResponseBytes: MAX_SOURCE, timeoutMs: 30_000, redirect: 'error', ...options.policy };
        this.capacity = options.maxResidentBytes ?? MAX_RESIDENT;
        if (!Number.isSafeInteger(this.capacity) || this.capacity < 1 || this.capacity > MAX_RESIDENT
            || !Number.isSafeInteger(this.policy.maxResponseBytes) || this.policy.maxResponseBytes < 1 || this.policy.maxResponseBytes > MAX_SOURCE
            || !Number.isFinite(this.policy.timeoutMs) || this.policy.timeoutMs <= 0 || this.policy.timeoutMs > 30_000
            || this.policy.redirect !== 'error') fail('policy');
        for (const name of ['schemaAssetPackBootstrapV1', 'schemaAssetPackSummaryV1', 'schemaAssetPackImportV1', 'commandNativeMemoryIdentity'] as const) {
            if (typeof this.runtime.native[name] !== 'function') fail('unsupported');
        }
        this.guard();
        this.memory = this.runtime.native.commandNativeMemoryIdentity();
        if (!this.memory) fail('unsupported');
    }

    /** No decoded source, native handle or mutable cache buffer is published. */
    async loadSource(identity: CemSchemaAssetSourceIdentity, signal?: AbortSignal): Promise<CemLoadedSchemaAssetSource> {
        return this.operation(signal, async active => {
            const index = await this.loadIndex(active);
            const descriptor = index.sources.get(key(identity));
            if (!descriptor) fail('missing');
            const entryKey = key(descriptor);
            const retained = this.retained.get(entryKey)?.bytes;
            const acquired = retained ? undefined
                : await this.read(`sources/${descriptor.digest}`, descriptor.byteLength, descriptor.contentType, active);
            const bytes = retained ?? acquired?.bytes;
            if (!bytes) fail('response');
            try {
                this.guard(active);
                const result = this.import(index, descriptor, bytes);
                this.guard(active);
                if (!this.retained.has(entryKey)) {
                    acquired?.release(); // Transfer the reservation into retained bytes.
                    if (this.residentBytes + this.pendingBytes + bytes.byteLength > this.capacity) fail('limit');
                    this.retained.set(entryKey, { descriptor, bytes });
                    this.residentBytes += bytes.byteLength;
                }
                return { ...result, contentType: descriptor.contentType, bytes: bytes.slice() };
            } finally { acquired?.release(); }
        });
    }

    /** Full immutable delivery list. Sidecars are inventory, never trust anchors. */
    async deploymentUrls(signal?: AbortSignal): Promise<readonly string[]> {
        return this.operation(signal, async active => {
            const index = await this.loadIndex(active);
            return Object.freeze([...new Set(['index.json', 'index.sha256', 'bootstrap.json', 'measurements.json', 'integrity.json',
                ...[...index.sources.values()].map(source => `sources/${source.digest}`)])].sort().map(path => new URL(path, this.base).href));
        });
    }

    /** Explicit byte-only preparation, independent of checkpoint/credential authority.
     * Each retained source is admitted/imported again on the distinct target heap.
     * The caller still owns coordinated retirement and final publication.
     */
    async prepareFreshRuntime(runtime: CemSchemaAssetRuntime, signal?: AbortSignal): Promise<CemSchemaAssetPackLoader> {
        this.lifetime.signal.throwIfAborted(); signal?.throwIfAborted();
        if (this.active) fail('busy');
        if (runtime.native.commandNativeMemoryIdentity() === this.memory) fail('generation');
        const fresh = new CemSchemaAssetPackLoader({ ...this.options, runtime });
        try {
            await fresh.operation(signal, async active => {
                const pin = fresh.bootstrap(active);
                if (!this.index) return;
                if (pin.indexDigest !== this.index.pin.indexDigest) fail('digest');
                const index = fresh.admitIndex(this.index.bytes, pin, active);
                for (const [sourceKey, entry] of this.retained) {
                    fresh.guard(active);
                    fresh.import(index, entry.descriptor, entry.bytes);
                    fresh.guard(active);
                    fresh.retained.set(sourceKey, entry);
                    fresh.residentBytes += entry.bytes.byteLength;
                }
            });
            return fresh;
        } catch (error) { fresh.dispose(); throw error; }
    }

    dispose(): void {
        this.lifetime.abort(new CemSchemaAssetLoadError('cem.schema_asset.disposed'));
        this.index = undefined; this.retained.clear(); this.residentBytes = 0;
        this.endpoint = undefined; this.memory = undefined;
    }

    private get runtime(): CemSchemaAssetRuntime {
        if (!this.endpoint) fail('disposed');
        return this.endpoint;
    }
    private guard(signal?: AbortSignal): void {
        this.lifetime.signal.throwIfAborted(); signal?.throwIfAborted();
        if (this.runtime.current() !== true) fail('stale');
        if (this.memory && this.runtime.native.commandNativeMemoryIdentity() !== this.memory) fail('stale');
    }
    private bootstrap(signal: AbortSignal): Bootstrap {
        this.guard(signal);
        const pin = nativeControl<Bootstrap>(() => this.runtime.native.schemaAssetPackBootstrapV1());
        if (!digestPattern.test(pin.indexDigest) || !Number.isSafeInteger(pin.indexBytes) || pin.indexBytes < 1 || pin.indexBytes > MAX_INDEX
            || !Number.isSafeInteger(pin.sourceCount) || pin.sourceCount < 1 || pin.sourceCount > 1024) fail('bootstrap');
        this.guard(signal); return pin;
    }
    private async loadIndex(signal: AbortSignal) {
        const pin = this.bootstrap(signal); // Native admission applies on cache hits too.
        if (this.index) {
            if (this.index.pin.indexDigest !== pin.indexDigest) fail('digest');
            return this.index;
        }
        const acquired = await this.read('index.json', pin.indexBytes, 'application/json', signal);
        try { return this.admitIndex(acquired.bytes, pin, signal, acquired.release); }
        finally { acquired.release(); }
    }
    private admitIndex(bytes: Uint8Array, pin: Bootstrap, signal: AbortSignal, release?: () => void) {
        this.guard(signal);
        const summary = nativeControl<Bootstrap>(() => this.runtime.native.schemaAssetPackSummaryV1(bytes, pin.indexDigest));
        for (const field of ['indexDigest', 'indexBytes', 'sourceCount', 'sourceBytes', 'packageCount'] as const) {
            if (summary[field] !== pin[field]) fail('bootstrap');
        }
        // This is the explicit native-validated catalog DTO, never a CEM AST.
        const dto = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)) as { sources: SourceDescriptor[] };
        if (!Array.isArray(dto.sources) || dto.sources.length !== pin.sourceCount) fail('response');
        const sources = new Map<string, SourceDescriptor>();
        for (const source of dto.sources) {
            if (typeof source.packageId !== 'string' || typeof source.version !== 'string' || typeof source.path !== 'string'
                || typeof source.contentType !== 'string' || !digestPattern.test(source.digest)
                || !Number.isSafeInteger(source.byteLength) || source.byteLength < 1 || source.byteLength > MAX_SOURCE) fail('response');
            sources.set(key(source), source);
        }
        this.guard(signal);
        if (!this.index) {
            release?.();
            if (this.residentBytes + this.pendingBytes + bytes.byteLength > this.capacity) fail('limit');
            this.index = { bytes, pin, sources }; this.residentBytes += bytes.byteLength;
        }
        return this.index;
    }
    private import(index: NonNullable<CemSchemaAssetPackLoader['index']>, descriptor: SourceDescriptor, bytes: Uint8Array): ImportedSource {
        const result = nativeControl<ImportedSource>(() => this.runtime.native.schemaAssetPackImportV1(index.bytes, index.pin.indexDigest,
            descriptor.packageId, descriptor.version, descriptor.path, bytes));
        if (key(result) !== key(descriptor) || result.indexDigest !== index.pin.indexDigest || result.digest !== descriptor.digest
            || result.sourceUri !== descriptor.path || result.sourceBytes !== descriptor.byteLength
            || !Number.isSafeInteger(result.nodeCount) || result.nodeCount < 1) fail('response');
        return result;
    }

    private async read(path: string, length: number, contentType: string, signal: AbortSignal): Promise<{ bytes: Uint8Array; release(): void }> {
        if (length > this.policy.maxResponseBytes || this.residentBytes + this.pendingBytes + length > this.capacity) fail('limit');
        this.pendingBytes += length;
        let held = true, delivered = false;
        const release = () => { if (held) { held = false; this.pendingBytes -= length; } };
        let iterator: AsyncIterator<Uint8Array> | undefined;
        try {
            const authoredUrl = new URL(path, this.base).href;
            const request: CemResourceResolutionRequest = { kind: 'http-request', authoredUrl, baseUrl: this.base.href,
                declarationScopeId: 'cem-schema-assets', contextIdentity: 'cem-schema-assets/1', method: 'GET',
                headers: { Accept: contentType }, expectedContentTypes: [contentType, 'application/octet-stream'] };
            const resolved = this.options.resolveResourceUrl
                ? await abortable(Promise.resolve(this.options.resolveResourceUrl(request)), signal) : authoredUrl;
            this.guard(signal);
            const resolution: CemResourceResolution = typeof resolved === 'string'
                ? { authoredUrl, resolvedUrl: resolved, resolverIdentity: 'cem-schema-assets/1', resourcePolicyStamp: 'immutable-pinned' } : resolved;
            const url = new URL(resolution.resolvedUrl);
            if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.hash
                || !this.policy.allowCrossOrigin && url.origin !== this.base.origin) fail('policy');
            const loaded = await abortable((this.options.loadHttpResource ?? openCemHttpResource)({ ...request, ...resolution,
                method: 'GET', credentials: 'omit', cache: 'default', policy: { ...this.policy, maxResponseBytes: length }, signal }), signal,
                late => { void Promise.resolve(late.body[Symbol.asyncIterator]().return?.()).catch(() => undefined); });
            iterator = loaded.body[Symbol.asyncIterator]();
            this.guard(signal);
            const response = loaded.response;
            if (!response.ok || response.status < 200 || response.status >= 300) fail('http');
            if (response.redirected || new URL(response.url).href !== url.href) fail('policy');
            const mime = mediaType(response.contentType);
            // Digest-named static files may use an opaque binary transport MIME.
            // Semantic source MIME comes from the native-authenticated index.
            if (mime !== contentType && mime !== 'application/octet-stream') fail('mime');
            const body = new Uint8Array(length);
            let offset = 0, chunks = 0;
            for (;;) {
                const chunk = await abortable(Promise.resolve(iterator.next()), signal);
                this.guard(signal);
                if (chunk.done) break;
                // Also bound zero-byte/microtask-only producers, which can
                // otherwise starve the deadline timer without growing a body.
                if (++chunks > 65_536) fail('limit');
                if (!(chunk.value instanceof Uint8Array) || offset + chunk.value.byteLength > length) fail('limit');
                body.set(chunk.value, offset); offset += chunk.value.byteLength;
            }
            if (offset !== length) fail('limit');
            delivered = true;
            return { bytes: body, release };
        } catch (error) {
            this.guard(signal);
            if (error instanceof CemSchemaAssetLoadError) throw error;
            throw new CemSchemaAssetLoadError('cem.schema_asset.unavailable');
        } finally {
            if (!delivered) release();
            closeBody(iterator);
        }
    }
    private async operation<T>(external: AbortSignal | undefined, work: (signal: AbortSignal) => Promise<T>): Promise<T> {
        this.guard(external);
        if (this.active >= 8) fail('busy');
        const controller = new AbortController();
        const cancel = () => controller.abort(external?.aborted ? external.reason : this.lifetime.signal.reason);
        external?.addEventListener('abort', cancel, { once: true });
        this.lifetime.signal.addEventListener('abort', cancel, { once: true });
        const timeout = setTimeout(() => controller.abort(new CemSchemaAssetLoadError('cem.schema_asset.timeout')), this.policy.timeoutMs);
        this.active++;
        try { const result = await work(controller.signal); this.guard(controller.signal); return result; }
        finally {
            this.active--; clearTimeout(timeout);
            external?.removeEventListener('abort', cancel); this.lifetime.signal.removeEventListener('abort', cancel);
        }
    }
}

function control<T>(text: string): T {
    if (typeof text !== 'string' || text.length > 16 * 1024) fail('response');
    let value: unknown;
    try { value = JSON.parse(text); } catch { fail('response'); }
    if (!value || typeof value !== 'object' || Array.isArray(value)) fail('response');
    return value as T;
}
function closeBody(iterator: AsyncIterator<Uint8Array> | undefined): void {
    try { void Promise.resolve(iterator?.return?.()).catch(() => undefined); }
    catch { /* A broken provider cannot retain loader admission or replace its result. */ }
}
function nativeControl<T>(read: () => string): T {
    try { return control<T>(read()); }
    catch (error) {
        if (typeof error === 'string' && /^cem\.[a-z0-9_.]+$/u.test(error)) throw new CemSchemaAssetLoadError(error);
        throw error;
    }
}
function abortable<T>(promise: Promise<T>, signal: AbortSignal, abandoned?: (value: T) => void): Promise<T> {
    signal.throwIfAborted();
    return new Promise<T>((resolve, reject) => {
        const cancel = () => reject(signal.reason);
        signal.addEventListener('abort', cancel, { once: true });
        promise.then(value => {
            if (signal.aborted) { try { abandoned?.(value); } catch { /* no authority survives cancellation */ } }
            resolve(value);
        }, reject).finally(() => signal.removeEventListener('abort', cancel));
    });
}
