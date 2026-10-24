import { assertNativeGenerationResourceTarget, registerNativeGenerationResource, type CemNativeGenerationParticipant, type CemNativeGenerationBindingContext } from './internal/runtime-support/native-generation.js';
import type { CemHttpRequest, CemHttpResourceLoadResult, CemHttpResourcePolicy,
    CemResourceResolution, CemResourceResolutionRequest, CemResourceResolutionResult } from './cem-elements.js';
import { openCemHttpResource } from './http-resource-transport.js';

/** Optional native ABI only. These calls release all temporary native owners. */
export interface CemSchemaAssetNativeRuntime {
    schemaAssetPackBootstrapV1(): string;
    schemaAssetPackSummaryV1(index: Uint8Array, pin: string): string;
    schemaAssetPackImportV1(index: Uint8Array, pin: string, packageId: string, version: string, path: string, bytes: Uint8Array): string;
    commandNativeMemoryIdentity(): WebAssembly.Memory;
    schemaAssetOwnerOpenV1?(index: Uint8Array, pin: string, hostConfiguration: string): number;
    schemaAssetOwnerAdmitV1?(owner: number, packageId: string, version: string, path: string, bytes: Uint8Array): void;
    schemaAssetOwnerActivateV1?(owner: number, selection: string): string;
    schemaAssetOwnerDisposeV1?(owner: number): boolean;
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
    /** Explicit trusted embedding configuration, never taken from a template or URL.
     * Required for activation; native exact replacement grants remain authoritative. */
    activationHostConfiguration?: string;
}
export interface CemSchemaAssetSourceIdentity { packageId: string; version: string; path: string }
export interface CemSchemaAssetPackageIdentity { packageId: string; version: string }
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

/** Bounded original-byte acquisition and explicit embedding-owned activation.
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
    private epoch = 0;
    private detached = false;
    private owner?: number;
    private activating = false;
    private activationQueue: Promise<void> = Promise.resolve();
    private queuedActivations = 0;
    private readonly activated = new Map<string, CemSchemaAssetPackageIdentity>();

    constructor(options: CemSchemaAssetPackOptions) {
        const { runtime, ...transport } = options;
        const native = runtime.native;
        const methods = ['schemaAssetPackBootstrapV1', 'schemaAssetPackSummaryV1', 'schemaAssetPackImportV1', 'commandNativeMemoryIdentity',
            'schemaAssetOwnerOpenV1', 'schemaAssetOwnerAdmitV1', 'schemaAssetOwnerActivateV1', 'schemaAssetOwnerDisposeV1'] as const;
        this.endpoint = { native: Object.freeze(Object.fromEntries(methods.map(name => [name,
            typeof native[name] === 'function' ? native[name].bind(native) : undefined]))) as unknown as CemSchemaAssetNativeRuntime,
            current: runtime.current.bind(runtime) };
        this.base = new URL(options.baseUrl);
        if (!['https:', 'http:'].includes(this.base.protocol) || this.base.username || this.base.password
            || this.base.search || this.base.hash || !this.base.pathname.endsWith('/')) fail('policy');
        this.policy = { allowCrossOrigin: false, maxResponseBytes: MAX_SOURCE, timeoutMs: 30_000, redirect: 'error', ...options.policy };
        this.capacity = options.maxResidentBytes ?? MAX_RESIDENT;
        if (!Number.isSafeInteger(this.capacity) || this.capacity < 1 || this.capacity > MAX_RESIDENT
            || !Number.isSafeInteger(this.policy.maxResponseBytes) || this.policy.maxResponseBytes < 1 || this.policy.maxResponseBytes > MAX_SOURCE
            || !Number.isFinite(this.policy.timeoutMs) || this.policy.timeoutMs <= 0 || this.policy.timeoutMs > 30_000
            || this.policy.redirect !== 'error') fail('policy');
        Object.freeze(this.policy);
        this.options = { ...transport, policy: this.policy };
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

    /** Fetch original bodies, then let native code publish the complete selection.
     * This loader owns the sole package context in its physical runtime. It must
     * be disposed before that runtime can pass the existing empty-inventory gate. */
    async activatePackages(packages: readonly CemSchemaAssetPackageIdentity[], signal?: AbortSignal): Promise<void> {
        this.guard(signal);
        if (!Array.isArray(packages) || !packages.length || packages.length > 64
            || packages.some(identity => !identity || typeof identity.packageId !== 'string' || typeof identity.version !== 'string'
                || identity.packageId.length > 256 || identity.version.length > 64)) fail('identity');
        if (this.queuedActivations >= 8) fail('busy');
        this.queuedActivations++;
        // Copy authored identities before any wait. No caller can change the
        // selection while another declaration owns the publication boundary.
        const selection = packages.map(identity => ({ ...identity }));
        const work = this.activationQueue.then(() => this.activateSelection(selection, signal))
            .finally(() => { this.queuedActivations--; });
        this.activationQueue = work.catch(() => undefined);
        if (signal) await abortable(work, signal); else await work;
    }

    private async activateSelection(packages: readonly CemSchemaAssetPackageIdentity[], signal?: AbortSignal): Promise<void> {
        this.guard(signal);
        if (!packages.length || packages.length > 64
            || new Set(packages.map(p => p.packageId)).size !== packages.length) fail('identity');
        const native = this.runtime.native;
        const openOwner = native.schemaAssetOwnerOpenV1, admit = native.schemaAssetOwnerAdmitV1,
            activate = native.schemaAssetOwnerActivateV1;
        if (!openOwner || !admit || !activate || !native.schemaAssetOwnerDisposeV1) fail('unsupported');
        const hostConfiguration = this.options.activationHostConfiguration;
        if (typeof hostConfiguration !== 'string') fail('policy');
        this.activating = true;
        try {
            await this.operation(signal, async active => {
                const index = await this.loadIndex(active);
                for (const identity of packages) {
                    const descriptors = [...index.sources.values()].filter(source => source.packageId === identity.packageId && source.version === identity.version);
                    if (!descriptors.length) fail('missing');
                    for (const descriptor of descriptors) await this.loadSource(descriptor, active);
                }
                this.guard(active);
                const owner = this.owner ?? nativeCall(() => openOwner(index.bytes, index.pin.indexDigest, hostConfiguration));
                if (!Number.isSafeInteger(owner) || owner < 1) fail('response');
                this.owner = owner;
                for (const { descriptor, bytes } of this.retained.values()) {
                    this.guard(active);
                    nativeCall(() => admit(owner, descriptor.packageId, descriptor.version, descriptor.path, bytes));
                }
                this.guard(active);
                const result = nativeControl<{ ready: boolean; indexDigest: string }>(() =>
                    activate(owner, JSON.stringify(packages)));
                if (result.indexDigest !== index.pin.indexDigest || result.ready !== true) fail('activation');
                this.guard(active);
                for (const identity of packages) this.activated.set(identity.packageId, { ...identity });
            });
        } finally { this.activating = false; }
    }

    /** Shared declarative demand through an existing http-request resource.
     * Install as loadHttpResource in a host that owns this same runtime; ordinary
     * requests use the configured transport. The returned JSON is a named status
     * DTO, not a schema/document AST or a replacement-authorization grant. */
    async loadResource(request: CemHttpRequest): Promise<CemHttpResourceLoadResult> {
        const prefix = new URL('demand/', this.base).href;
        if (!request.resolvedUrl.startsWith(prefix)) {
            return (this.options.loadHttpResource ?? openCemHttpResource)(request);
        }
        this.guard(request.signal);
        const url = new URL(request.resolvedUrl);
        if (request.method !== 'GET' || url.search || url.hash || url.username || url.password) fail('policy');
        const parts = url.href.slice(prefix.length).split('/');
        if (parts.length !== 2 || parts.some(part => !part)) fail('identity');
        let identity: CemSchemaAssetPackageIdentity;
        try { identity = { packageId: decodeURIComponent(parts[0]), version: decodeURIComponent(parts[1]) }; }
        catch { return fail('identity'); }
        await this.activatePackages([identity], request.signal);
        this.guard(request.signal);
        const body = new TextEncoder().encode(JSON.stringify({ kind: 'cem-schema-package-status/1', state: 'ready', ...identity }));
        return { response: { url: url.href, status: 200, statusText: 'OK', ok: true, redirected: false,
            contentType: 'application/json', headers: { 'content-type': 'application/json' } },
            body: { async *[Symbol.asyncIterator]() { yield body; } } };
    }

    /** Explicit byte-only preparation, independent of checkpoint/credential authority.
     * Each retained source is admitted/imported again on the distinct target heap.
     * The caller still owns coordinated retirement and final publication.
     */
    async prepareFreshRuntime(runtime: CemSchemaAssetRuntime, signal?: AbortSignal): Promise<CemSchemaAssetPackLoader> {
        this.lifetime.signal.throwIfAborted(); signal?.throwIfAborted();
        if (this.active || this.activating || this.queuedActivations) fail('busy');
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
            if (this.activated.size) await fresh.activatePackages([...this.activated.values()], signal);
            this.lifetime.signal.throwIfAborted(); signal?.throwIfAborted();
            return fresh;
        } catch (error) { fresh.dispose(); throw error; }
    }

    /** @internal Registration must describe this exact initialized heap. */
    assertRecoveryParticipant(participant: CemNativeGenerationParticipant): void {
        this.guard();
        if (!this.memory) fail('stale');
        assertNativeGenerationResourceTarget(participant, undefined, this.memory);
    }

    /** @internal Byte-only source lease for an original generation-resource adapter.
     * release is invoked only within checked checkpoint/embedding cleanup. It
     * drops every old native reference while preserving immutable replay bytes. */
    prepareRecoverySource(): Readonly<{ current(): boolean; release(): void;
        prepare(runtime: CemSchemaAssetRuntime, signal: AbortSignal): Promise<CemSchemaAssetPackLoader> }> {
        this.guard();
        if (this.active || this.activating || this.queuedActivations || this.detached) fail('busy');
        const epoch = this.epoch;
        const current = () => !this.lifetime.signal.aborted && epoch === this.epoch && !this.active && !this.activating && !this.queuedActivations;
        return Object.freeze({ current, release: () => {
            if (!current()) fail('stale');
            if (this.detached) return;
            if (this.endpoint?.native.commandNativeMemoryIdentity() !== this.memory) fail('stale');
            if (this.owner !== undefined) {
                const dispose = this.endpoint?.native.schemaAssetOwnerDisposeV1, owner = this.owner;
                if (!dispose || nativeCall(() => dispose(owner)) !== true) fail('owner');
                this.owner = undefined;
            }
            this.detached = true; this.endpoint = undefined; this.memory = undefined;
        }, prepare: (runtime, signal) => {
            if (!current() || !this.detached) fail('stale');
            return this.prepareFreshRuntime(runtime, signal);
        } });
    }

    dispose(): void {
        this.lifetime.abort(new CemSchemaAssetLoadError('cem.schema_asset.disposed'));
        this.index = undefined; this.retained.clear(); this.residentBytes = 0;
        this.activated.clear();
        // Cleanup stays available under closed native admission. A failed cleanup
        // leaves its owner visible in inventory and cannot authorize retirement.
        if (this.owner !== undefined && this.endpoint) {
            const owner = this.owner;
            nativeCall(() => this.endpoint?.native.schemaAssetOwnerDisposeV1?.(owner));
            this.owner = undefined;
        }
        this.endpoint = undefined; this.memory = undefined;
    }

    private get runtime(): CemSchemaAssetRuntime {
        if (!this.endpoint) fail(this.detached ? 'stale' : 'disposed');
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
        this.active++; this.epoch++;
        try { const result = await work(controller.signal); this.guard(controller.signal); return result; }
        finally {
            this.active--; clearTimeout(timeout);
            external?.removeEventListener('abort', cancel); this.lifetime.signal.removeEventListener('abort', cancel);
        }
    }
}

export interface CemSchemaAssetPackRecoveryOptions {
    loader: CemSchemaAssetPackLoader;
    participant: CemNativeGenerationParticipant;
    /** Original shared host adapter resolving the new physical runtime. */
    target(runtime: object): { runtime: CemSchemaAssetRuntime; participant: CemNativeGenerationParticipant };
    /** Recovery policy fence, including credentials; ordinary reads retain native admission. */
    current(): boolean;
}

/** Stable HTTP/source route participating in complete native binding transactions.
 * Native owners and replay buffers never enter checkpoint/status DTOs. */
export class CemSchemaAssetPackRecoveryBinding {
    private loader: CemSchemaAssetPackLoader;
    private closed = false;
    private registration?: Readonly<{ remove(): void }>;
    private readonly target: CemSchemaAssetPackRecoveryOptions['target'];
    private readonly admitted: () => boolean;
    constructor(options: CemSchemaAssetPackRecoveryOptions) {
        this.loader = options.loader; this.target = options.target; this.admitted = options.current;
        if (typeof this.target !== 'function' || typeof this.admitted !== 'function' || this.admitted() !== true) fail('stale');
        this.loader.assertRecoveryParticipant(options.participant);
        this.enroll(options.participant);
    }
    loadResource = (request: CemHttpRequest): Promise<CemHttpResourceLoadResult> => {
        this.guard(); return this.loader.loadResource(request);
    };
    loadSource = (identity: CemSchemaAssetSourceIdentity, signal?: AbortSignal): Promise<CemLoadedSchemaAssetSource> => {
        this.guard(); return this.loader.loadSource(identity, signal);
    };
    private guard(recovery = false): void { if (this.closed || recovery && this.admitted() !== true) fail('stale'); }
    private enroll(participant: CemNativeGenerationParticipant): void {
        this.registration = registerNativeGenerationResource(participant, () => {
            this.guard(true);
            const previous = this.loader, source = previous.prepareRecoverySource();
            const current = () => !this.closed && this.loader === previous && this.admitted() === true && source.current();
            return { current, release: () => { if (!current()) fail('stale'); source.release(); },
                prepare: async (context: CemNativeGenerationBindingContext) => {
                    if (!current() || context.signal.aborted) fail('stale');
                    const target = this.target(context.generation.runtime);
                    assertNativeGenerationResourceTarget(target.participant, context.generation.runtimeId, target.runtime.native.commandNativeMemoryIdentity());
                    const fresh = await source.prepare(target.runtime, context.signal);
                    let discarded = false;
                    const discard = () => {
                        if (discarded) return; discarded = true;
                        if (this.loader === fresh || this.loader === previous) { this.registration?.remove(); this.closed = true; }
                        fresh.dispose(); previous.dispose();
                    };
                    if (!current() || context.signal.aborted) { discard(); fail('stale'); }
                    return { owner: this, current: () => !discarded && current() && !context.signal.aborted,
                        bind: () => {
                            if (!context.active() || !current() || discarded || context.signal.aborted) fail('stale');
                            this.registration?.remove(); this.loader = fresh;
                            this.enroll(target.participant);
                        }, bound: () => { previous.dispose(); }, discard };
                } };
        }, () => this.dispose());
    }
    dispose(): void {
        if (this.closed) return;
        this.closed = true; this.registration?.remove(); this.loader.dispose();
    }
}

/** Structural interface supplied only by an original owned host runtime. */
export interface CemSchemaAssetNativeBinding {
    current(): boolean;
    memory(): WebAssembly.Memory | undefined;
    call(name: string, args: unknown[]): unknown;
}
export function cemSchemaAssetRuntimeFromBinding(binding: CemSchemaAssetNativeBinding): CemSchemaAssetRuntime {
    const call = binding.call.bind(binding), memory = binding.memory.bind(binding), current = binding.current.bind(binding);
    return { current, native: {
        schemaAssetPackBootstrapV1: () => call('schemaAssetPackBootstrapV1', []) as string,
        schemaAssetPackSummaryV1: (...args) => call('schemaAssetPackSummaryV1', args) as string,
        schemaAssetPackImportV1: (...args) => call('schemaAssetPackImportV1', args) as string,
        schemaAssetOwnerOpenV1: (...args) => call('schemaAssetOwnerOpenV1', args) as number,
        schemaAssetOwnerAdmitV1: (...args) => { call('schemaAssetOwnerAdmitV1', args); },
        schemaAssetOwnerActivateV1: (...args) => call('schemaAssetOwnerActivateV1', args) as string,
        schemaAssetOwnerDisposeV1: (...args) => call('schemaAssetOwnerDisposeV1', args) as boolean,
        commandNativeMemoryIdentity: () => { const identity = memory(); if (!identity) fail('stale'); return identity; },
    } };
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
    return nativeCall(() => control<T>(read()));
}
function nativeCall<T>(read: () => T): T {
    try { return read(); }
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
