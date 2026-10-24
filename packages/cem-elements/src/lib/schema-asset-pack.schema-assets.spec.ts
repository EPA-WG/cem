import { createRequire } from 'node:module';
import { cpSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { afterAll, expect, it, vi } from 'vitest';
import { CemSchemaAssetPackLoader, type CemSchemaAssetNativeRuntime, type CemSchemaAssetPackOptions } from './schema-asset-pack.js';
import type { CemHttpRequest, CemHttpResourceLoadResult } from './cem-elements.js';

// Real native optional ABI and its emitted original bytes. JSON here is the
// explicitly named catalog/control boundary, never an AST/source substitute.
const root = resolve(import.meta.dirname, '../../..');
const pack = join(root, 'cem-ml-npm/dist-schema-assets');
const modulePath = join(root, 'cem-ml-npm/dist-schema-assets-wasm/wasm/node/cem_ml.js');
const native = createRequire(import.meta.url)(modulePath) as CemSchemaAssetNativeRuntime;
const index = JSON.parse(readFileSync(join(pack, 'index.json'), 'utf8')) as {
    sources: Array<{ packageId: string; version: string; path: string; digest: string; byteLength: number; contentType: string }>;
};
const source = index.sources.find(item => item.packageId === 'csv' && item.path.endsWith('/formatters/compact.cemt'));
if (!source) throw new Error('Missing native CSV fixture');
const selected = source;
const baseUrl = 'https://assets.example.test/schema-assets/';
const diskRead = (path: string) => new Uint8Array(readFileSync(join(pack, path)));
const requests: CemHttpRequest[] = [];
const response = (request: CemHttpRequest, bytes: Uint8Array, mime = 'application/octet-stream'): CemHttpResourceLoadResult => ({
    response: { url: request.resolvedUrl, ok: true, status: 200, statusText: 'OK', redirected: false, headers: {}, contentType: mime },
    body: { async *[Symbol.asyncIterator]() { yield bytes; } },
});
const read = async (request: CemHttpRequest) => {
    requests.push(request);
    return response(request, diskRead(new URL(request.authoredUrl).pathname.split('/schema-assets/')[1] ?? 'missing'));
};
const options = (overrides: Partial<CemSchemaAssetPackOptions> = {}): CemSchemaAssetPackOptions => ({
    baseUrl, runtime: { native, current: () => true }, loadHttpResource: read, ...overrides,
});
const temporary = mkdtempSync(join(tmpdir(), 'cem-pack-loader-'));
afterAll(() => rmSync(temporary, { recursive: true, force: true }));

it('loads original bytes lazily, preserves native provenance and revalidates cached reads', async () => {
    requests.length = 0;
    const importSource = vi.fn(native.schemaAssetPackImportV1);
    const loader = new CemSchemaAssetPackLoader(options({ runtime: { native: { ...native, schemaAssetPackImportV1: importSource }, current: () => true } }));
    try {
        const first = await loader.loadSource(selected);
        expect(first.bytes).toEqual(diskRead(`sources/${selected.digest}`));
        expect(first.sourceUri).toBe(selected.path);
        expect(first.nodeCount).toBeGreaterThan(0);
        first.bytes.fill(0);
        expect((await loader.loadSource(selected)).bytes).toEqual(diskRead(`sources/${selected.digest}`));
        expect(requests.map(request => request.authoredUrl)).toEqual([`${baseUrl}index.json`, `${baseUrl}sources/${selected.digest}`]);
        expect(requests.every(request => request.credentials === 'omit' && request.policy.redirect === 'error')).toBe(true);
        expect(importSource).toHaveBeenCalledTimes(2);
    } finally { loader.dispose(); }
});

it('returns the complete native delivery inventory without fetching source bodies', async () => {
    requests.length = 0;
    const loader = new CemSchemaAssetPackLoader(options());
    try {
        const urls = await loader.deploymentUrls();
        const emitted = JSON.parse(readFileSync(join(pack, 'integrity.json'), 'utf8')) as { offlineUrls: string[] };
        expect([...urls].sort()).toEqual(emitted.offlineUrls.map(path => new URL(path, baseUrl).href).sort());
        expect(requests).toHaveLength(1);
    } finally { loader.dispose(); }
});

it('refuses an offline/missing body without selecting an embedded source and permits retry', async () => {
    let offline = true;
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: async request => {
        if (offline && !request.authoredUrl.endsWith('index.json')) throw new Error('offline');
        return read(request);
    } }));
    try {
        await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.unavailable');
        offline = false;
        expect((await loader.loadSource(selected)).sourceUri).toBe(selected.path);
    } finally { loader.dispose(); }
});

it('authenticates the downloaded index against the native pin before catalog use', async () => {
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: async request => {
        const bytes = diskRead('index.json'); bytes[bytes.length - 2] ^= 1;
        return response(request, bytes, 'application/json');
    } }));
    try { await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.digest'); }
    finally { loader.dispose(); }
});

it('refuses tampered source bytes natively and never admits the failed read to the cache', async () => {
    let corrupt = true;
    let bodyReads = 0;
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: async request => {
        if (request.authoredUrl.endsWith('index.json')) return read(request);
        bodyReads++;
        const bytes = diskRead(`sources/${selected.digest}`);
        if (corrupt) bytes[0] ^= 1;
        return response(request, bytes);
    } }));
    try {
        await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.digest');
        corrupt = false;
        await loader.loadSource(selected);
        expect(bodyReads).toBe(2);
    } finally { loader.dispose(); }
});

it.each(['text/html', null])('refuses transport MIME %s and closes the opened body', async mime => {
    const returned = vi.fn(async () => ({ done: true as const, value: undefined }));
    const next = vi.fn(async () => ({ done: true as const, value: undefined }));
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: async request => ({
        ...response(request, diskRead('index.json')), response: { ...response(request, new Uint8Array()).response, contentType: mime },
        body: { [Symbol.asyncIterator]: () => ({ next, return: returned }) },
    }) }));
    try {
        await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.mime');
        expect(next).not.toHaveBeenCalled(); expect(returned).toHaveBeenCalledOnce();
    } finally { loader.dispose(); }
});

it.each([-1, 1])('refuses a source length offset of %s without publishing', async offset => {
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: async request => request.authoredUrl.endsWith('index.json')
        ? read(request) : response(request, new Uint8Array(selected.byteLength + offset)) }));
    try { await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.limit'); }
    finally { loader.dispose(); }
});

it('refuses unknown source identities before acquiring a body', async () => {
    const loader = new CemSchemaAssetPackLoader(options());
    requests.length = 0;
    try {
        await expect(loader.loadSource({ ...selected, path: '../unlisted.cemt' })).rejects.toThrow('cem.schema_asset.missing');
        expect(requests).toHaveLength(1);
    } finally { loader.dispose(); }
});

it('applies aggregate and individual transport bounds before opening a source', async () => {
    const pin = JSON.parse(native.schemaAssetPackBootstrapV1()) as { indexBytes: number };
    const readResource = vi.fn(read);
    const loader = new CemSchemaAssetPackLoader(options({ maxResidentBytes: pin.indexBytes + selected.byteLength - 1, loadHttpResource: readResource }));
    try {
        await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.limit');
        expect(readResource).toHaveBeenCalledTimes(1);
    } finally { loader.dispose(); }
    const small = new CemSchemaAssetPackLoader(options({ policy: { maxResponseBytes: 1 }, loadHttpResource: readResource }));
    try { await expect(small.loadSource(selected)).rejects.toThrow('cem.schema_asset.limit'); }
    finally { small.dispose(); }
    expect(readResource).toHaveBeenCalledTimes(1);
});

it('aborts an uncooperative resolver without waiting for it or entering native import', async () => {
    const controller = new AbortController();
    const load = vi.fn(read);
    const loader = new CemSchemaAssetPackLoader(options({ resolveResourceUrl: () => new Promise(() => undefined), loadHttpResource: load }));
    try {
        const result = loader.loadSource(selected, controller.signal);
        controller.abort(new Error('user cancelled'));
        await expect(result).rejects.toThrow('user cancelled');
        expect(load).not.toHaveBeenCalled();
    } finally { loader.dispose(); }
});

it('cancels a stalled body without awaiting its uncooperative iterator return', async () => {
    const controller = new AbortController();
    const returned = vi.fn(() => new Promise<IteratorResult<Uint8Array>>(() => undefined));
    let started!: () => void;
    const ready = new Promise<void>(resolveReady => { started = resolveReady; });
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: async request => ({
        ...response(request, new Uint8Array()),
        body: { [Symbol.asyncIterator]: () => ({ next: () => { started(); return new Promise(() => undefined); }, return: returned }) },
    }) }));
    try {
        const result = loader.loadSource(selected, controller.signal);
        await ready; controller.abort(new Error('cancel body'));
        await expect(result).rejects.toThrow('cancel body');
        expect(returned).toHaveBeenCalledOnce();
    } finally { loader.dispose(); }
});

it('times out an uncooperative byte opener and fences disposal', async () => {
    const loader = new CemSchemaAssetPackLoader(options({ policy: { timeoutMs: 10 }, loadHttpResource: () => new Promise(() => undefined) }));
    await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.timeout');
    const pending = loader.loadSource(selected);
    loader.dispose();
    await expect(pending).rejects.toThrow('cem.schema_asset.disposed');
});

it('fences stale-runtime completion before native source import or cache publication', async () => {
    let current = true;
    const importSource = vi.fn(native.schemaAssetPackImportV1);
    const loader = new CemSchemaAssetPackLoader(options({ runtime: { native: { ...native, schemaAssetPackImportV1: importSource }, current: () => current },
        loadHttpResource: async request => { const loaded = await read(request); current = false; return loaded; } }));
    try {
        await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.stale');
        expect(importSource).not.toHaveBeenCalled();
    } finally { loader.dispose(); }
});

it('reimports original bytes on a distinct heap without network, native handles or retirement authority', async () => {
    const freshRoot = join(temporary, 'fresh');
    cpSync(dirname(modulePath), freshRoot, { recursive: true });
    const freshNative = createRequire(import.meta.url)(join(freshRoot, 'cem_ml.js')) as CemSchemaAssetNativeRuntime;
    const imported = vi.fn(freshNative.schemaAssetPackImportV1);
    let current = true;
    const load = vi.fn(read);
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: load, runtime: { native, current: () => current } }));
    try {
        const before = await loader.loadSource(selected);
        expect(load).toHaveBeenCalledTimes(2);
        current = false; // The old endpoint is retired; only its original bytes survive.
        const fresh = await loader.prepareFreshRuntime({ native: { ...freshNative, schemaAssetPackImportV1: imported }, current: () => true });
        loader.dispose();
        try {
            expect(imported).toHaveBeenCalledOnce();
            expect(await fresh.loadSource(selected)).toEqual(before);
            expect(imported).toHaveBeenCalledTimes(2);
            expect(load).toHaveBeenCalledTimes(2);
            (freshNative as CemSchemaAssetNativeRuntime & { closeCommandNativeAdmission(): void }).closeCommandNativeAdmission();
            await expect(fresh.loadSource(selected)).rejects.toThrow('cem.runtime.admission.closed');
        } finally { fresh.dispose(); }
    } finally { loader.dispose(); }
});

it('closes a response arriving after cancellation without running its iterator', async () => {
    const controller = new AbortController();
    let deliver!: (value: CemHttpResourceLoadResult) => void;
    const opened = new Promise<CemHttpResourceLoadResult>(resolveLoaded => { deliver = resolveLoaded; });
    const returned = vi.fn(async () => ({ done: true as const, value: undefined }));
    const next = vi.fn();
    const opener = vi.fn(() => opened);
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: opener }));
    try {
        const pending = loader.loadSource(selected, controller.signal);
        await vi.waitFor(() => expect(opener).toHaveBeenCalledOnce());
        controller.abort(new Error('cancel opener'));
        await expect(pending).rejects.toThrow('cancel opener');
        deliver({ ...response({ resolvedUrl: baseUrl } as CemHttpRequest, new Uint8Array()),
            body: { [Symbol.asyncIterator]: () => ({ next, return: returned }) } });
        await vi.waitFor(() => expect(returned).toHaveBeenCalledOnce());
        expect(next).not.toHaveBeenCalled();
    } finally { loader.dispose(); }
});

it('keeps independently cancelled loads isolated', async () => {
    const first = new AbortController();
    const loader = new CemSchemaAssetPackLoader(options());
    try {
        const cancelled = loader.loadSource(selected, first.signal);
        const accepted = loader.loadSource(selected);
        first.abort(new Error('cancel first'));
        await expect(cancelled).rejects.toThrow('cancel first');
        expect((await accepted).sourceUri).toBe(selected.path);
    } finally { loader.dispose(); }
});

it('refuses a rebound memory even when the host callback still reports current', async () => {
    let memory = native.commandNativeMemoryIdentity();
    const loader = new CemSchemaAssetPackLoader(options({ runtime: {
        native: { ...native, commandNativeMemoryIdentity: () => memory }, current: () => true,
    } }));
    try {
        await loader.loadSource(selected);
        memory = new WebAssembly.Memory({ initial: 1 });
        await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.stale');
    } finally { loader.dispose(); }
});

it('bounds concurrent pending requests and releases them all on disposal', async () => {
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: () => new Promise(() => undefined) }));
    const pending = Array.from({ length: 8 }, () => loader.loadSource(selected));
    const settled = Promise.allSettled(pending);
    await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.busy');
    loader.dispose();
    expect((await settled).every(result => result.status === 'rejected')).toBe(true);
});

it('bounds a microtask-only zero-byte producer and ignores a throwing iterator return', async () => {
    const returned = vi.fn(() => { throw Error('broken return'); });
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: async request => ({
        ...response(request, new Uint8Array()),
        body: { [Symbol.asyncIterator]: () => ({ next: async () => ({ done: false, value: new Uint8Array() }), return: returned }) },
    }) }));
    try {
        await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.limit');
        expect(returned).toHaveBeenCalledOnce();
    } finally { loader.dispose(); }
});

it('rejects same-heap fresh preparation and unsupported/default ABI before network work', async () => {
    const loader = new CemSchemaAssetPackLoader(options());
    try { await expect(loader.prepareFreshRuntime({ native, current: () => true })).rejects.toThrow('cem.schema_asset.generation'); }
    finally { loader.dispose(); }
    expect(() => new CemSchemaAssetPackLoader(options({ runtime: { native: {} as CemSchemaAssetNativeRuntime, current: () => true } })))
        .toThrow('cem.schema_asset.unsupported');
});

it('rejects resolver origin/protocol changes and redirects even with a custom opener', async () => {
    for (const resolvedUrl of ['https://foreign.example.test/index.json', 'file:///tmp/index.json', 'https://user:secret@assets.example.test/index.json']) {
        const opener = vi.fn(read);
        const loader = new CemSchemaAssetPackLoader(options({ resolveResourceUrl: () => resolvedUrl, loadHttpResource: opener }));
        try {
            await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.policy');
            expect(opener).not.toHaveBeenCalled();
        } finally { loader.dispose(); }
    }
    const loader = new CemSchemaAssetPackLoader(options({ loadHttpResource: async request => {
        const result = await read(request); result.response.redirected = true; return result;
    } }));
    try { await expect(loader.loadSource(selected)).rejects.toThrow('cem.schema_asset.policy'); }
    finally { loader.dispose(); }
});


it('cancels an asynchronous package endpoint without waiting for a missing worker response', async () => {
    const identity = {}, controller = new AbortController();
    const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: { current: () => true, identity: () => identity,
        bootstrap: () => new Promise<string>(() => { /* Simulate a missing response. */ }),
        summary: () => '', importSource: () => '', activate: () => '', release: () => true } });
    try {
        const pending = loader.loadSource(selected, controller.signal);
        controller.abort(Error('cancel pending worker'));
        await expect(pending).rejects.toThrow('cancel pending worker');
    } finally { loader.dispose(); }
});
