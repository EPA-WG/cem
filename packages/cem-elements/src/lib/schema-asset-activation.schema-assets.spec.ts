import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { cpSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { afterAll, afterEach, expect, it, vi } from 'vitest';
import { CemSchemaAssetPackLoader, type CemSchemaAssetNativeRuntime } from './schema-asset-pack.js';
import type { CemHttpRequest, CemHttpResourceLoadResult } from './cem-elements.js';

const root = resolve(import.meta.dirname, '../../..');
const pack = join(root, 'cem-ml-npm/dist-schema-assets');
const modulePath = join(root, 'cem-ml-npm/dist-schema-assets-wasm/wasm/node/cem_ml.js');
const temporary = mkdtempSync(join(tmpdir(), 'cem-pack-activation-'));
const nativeRequire = createRequire(import.meta.url);
afterEach(() => {
    vi.restoreAllMocks();
    // Each raw CJS fixture has its own real heap. Do not let Node's module cache
    // retain every completed case's WASM instance for the entire test file.
    for (const path of Object.keys(nativeRequire.cache)) if (path.startsWith(`${temporary}/`)) {
        const parent = nativeRequire.cache[path]?.parent;
        if (parent) parent.children = parent.children.filter(child => !child.filename.startsWith(`${temporary}/`));
        delete nativeRequire.cache[path];
    }
});
afterAll(() => rmSync(temporary, { recursive: true, force: true }));
interface Native extends CemSchemaAssetNativeRuntime {
    schemaAssetDemandPlanV1(request: string): string;
    schemaAssetEmbeddedSourcesV1(): string;
    commandNativeResourceInventory(): string;
    closeCommandNativeAdmission(): string;
    validateCommandNativeClosure(revision: number): string;
    retainCemDocument(bytes: Uint8Array, contentType: string, uri: string): number;
    disposeCemDocument(id: number): boolean;
    normalizeCommandRunPlanV1(request: string): string;
    executeCommandServiceV1(request: string, capability: string, ledger: (request: string) => string,
        read: (request: string) => string, prepare: () => never, commit: () => never, rollback: () => void): Promise<string>;
}
// Real package compilation and fresh-heap reactivation are integration work.
const nativeCase = (name: string, run: () => void | Promise<void>) => it(name, run, 30_000);
let sequence = 0;
function freshNative(): Native {
    const path = join(temporary, String(++sequence));
    cpSync(dirname(modulePath), path, { recursive: true });
    return nativeRequire(join(path, 'cem_ml.js')) as Native;
}
const baseUrl = 'https://assets.example.test/schema-assets/';
const csv = { packageId: 'csv', version: '1.0.0' };
const hostConfiguration = JSON.stringify({ schemaPackageReplacementGrants: [{ packageId: 'csv',
    expectedOrigin: { kind: 'builtin' }, replacementManifestUri: 'schema-packages/csv/v1/package.cem' }] });
const transport = async (request: CemHttpRequest): Promise<CemHttpResourceLoadResult> => {
    const path = new URL(request.resolvedUrl).pathname.slice('/schema-assets/'.length);
    const bytes = new Uint8Array(readFileSync(join(pack, path)));
    return { response: { url: request.resolvedUrl, status: 200, statusText: 'OK', ok: true, redirected: false,
        contentType: 'application/octet-stream', headers: {} }, body: { async *[Symbol.asyncIterator]() { yield bytes; } } };
};
const census = (native: Native) => JSON.parse(native.commandNativeResourceInventory()) as {
    complete: boolean; revision: number; blockingResources: number; replacementAllowed: boolean;
    registries: Array<{ name: string; live: number }>;
};
const owners = (native: Native) => census(native).registries.find(entry => entry.name === 'schema-package-owners')?.live;
const makeLoader = (native: Native, current = () => true, configuration = hostConfiguration) => new CemSchemaAssetPackLoader({
    baseUrl, runtime: { native, current }, loadHttpResource: transport, activationHostConfiguration: configuration,
});
const demand = (signal = new AbortController().signal): CemHttpRequest => ({
    authoredUrl: `${baseUrl}demand/csv/1.0.0`, resolvedUrl: `${baseUrl}demand/csv/1.0.0`, baseUrl,
    resolverIdentity: 'fixture', resourcePolicyStamp: 'fixture', contextIdentity: 'declaration:fixture',
    method: 'GET', headers: {}, policy: { allowCrossOrigin: false, maxResponseBytes: 4096, timeoutMs: 30_000, redirect: 'error' }, signal,
});

nativeCase('publishes native ownership through the existing declarative HTTP resource contract', async () => {
    const native = freshNative(), loader = makeLoader(native);
    try {
        expect(owners(native)).toBe(0);
        expect(JSON.parse(native.schemaAssetEmbeddedSourcesV1())).toMatchObject({ externalBodies: true, catalogPackageCount: 32,
            externalPackages: expect.arrayContaining(['csv', 'markdown']) });
        for (const contentType of ['text/csv', 'csv', 'TEXT/CSV;header=present']) {
            expect(() => native.retainCemDocument(new TextEncoder().encode('id,name\n1,Ada\n'), contentType, 'memory:not-admitted.csv'))
                .toThrow('cem.schema_asset.missing');
        }
        const loaded = await loader.loadResource(demand());
        const chunks = [];
        for await (const bytes of loaded.body) chunks.push(bytes);
        expect(loaded.response.contentType).toBe('application/json');
        expect(JSON.parse(new TextDecoder().decode(chunks[0]))).toEqual({ kind: 'cem-schema-package-status/1', state: 'ready', ...csv });
        expect(owners(native)).toBe(1);
        expect(census(native).replacementAllowed).toBe(false);
        const document = native.retainCemDocument(new TextEncoder().encode('id,name\n1,Ada\n'), 'text/csv', 'memory:owned.csv');
        expect(native.disposeCemDocument(document)).toBe(true);
        expect(() => native.retainCemDocument(new TextEncoder().encode('id,name\n1,"Ada\n'), 'text/csv', 'memory:invalid.csv'))
            .toThrow('cem.csv.unclosed_quote');
        await Promise.all([loader.activatePackages([csv]), loader.activatePackages([csv])]);
        expect(owners(native)).toBe(1);
    } finally { loader.dispose(); }
    expect(owners(native)).toBe(0);
});

nativeCase('refuses a missing optional artifact without publishing or falling back to embedded sources', async () => {
    const native = freshNative();
    const index = JSON.parse(readFileSync(join(pack, 'index.json'), 'utf8')) as { sources: Array<{ packageId: string; path: string; digest: string }> };
    const missing = index.sources.find(source => source.packageId === 'csv' && source.path.endsWith('/formatters/compact.cemt'));
    if (!missing) throw Error('missing native CSV fixture');
    const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: { native, current: () => true }, activationHostConfiguration: hostConfiguration,
        loadHttpResource: request => request.resolvedUrl.endsWith(`sources/${missing.digest}`) ? Promise.reject(Error('offline missing body')) : transport(request) });
    try {
        await expect(loader.loadResource(demand())).rejects.toThrow('cem.schema_asset.unavailable');
        expect(owners(native)).toBe(0);
        expect(() => native.retainCemDocument(new TextEncoder().encode('id,name\n1,Ada\n'), 'text/csv', 'memory:still-missing.csv')).toThrow('cem.schema_asset.missing');
    } finally { loader.dispose(); }
});

nativeCase('refuses ungranted and unknown demand and never grants authority from a URL', async () => {
    const native = freshNative(), loader = makeLoader(native, () => true, '{}');
    try {
        await expect(loader.loadResource(demand())).rejects.toThrow('cem.schema_asset.activation');
        await expect(loader.activatePackages([{ ...csv, version: 'unknown' }])).rejects.toThrow('cem.schema_asset.missing');
        await expect(loader.loadResource({ ...demand(), resolvedUrl: `${baseUrl}demand/csv/1.0.0?grant=builtin` })).rejects.toThrow('cem.schema_asset.policy');
        await expect(loader.loadResource({ ...demand(), method: 'HEAD' })).rejects.toThrow('cem.schema_asset.policy');
    } finally { loader.dispose(); }
    expect(owners(native)).toBe(0);
});

nativeCase('counts an owner as a retirement blocker, closes raw admission and permits exact cleanup', async () => {
    const native = freshNative(), loader = makeLoader(native);
    await loader.activatePackages([csv]);
    native.closeCommandNativeAdmission();
    const before = census(native);
    expect(before.blockingResources).toBeGreaterThan(0);
    expect(() => native.validateCommandNativeClosure(before.revision)).toThrow();
    await expect(loader.activatePackages([csv])).rejects.toThrow('cem.runtime.admission.closed');
    loader.dispose();
    expect(owners(native)).toBe(0);
    const after = census(native);
    expect(after.blockingResources).toBe(0);
    expect(JSON.parse(native.validateCommandNativeClosure(after.revision)).closed).toBe(true);
});

nativeCase('fences raw owner identities and source lengths and keeps every mutation behind native closure', () => {
    const native = freshNative();
    const indexBytes = new Uint8Array(readFileSync(join(pack, 'index.json')));
    const pin = JSON.parse(native.schemaAssetPackBootstrapV1()) as { indexDigest: string };
    const descriptor = (JSON.parse(new TextDecoder().decode(indexBytes)) as {
        sources: Array<{ packageId: string; version: string; path: string; byteLength: number }>;
    }).sources.find(source => source.packageId === 'csv');
    if (!descriptor) throw Error('missing CSV source');
    const open = native.schemaAssetOwnerOpenV1, admit = native.schemaAssetOwnerAdmitV1,
        activate = native.schemaAssetOwnerActivateV1, dispose = native.schemaAssetOwnerDisposeV1;
    if (!open || !admit || !activate || !dispose) throw Error('missing native owner ABI');
    const refusal = (call: () => unknown) => { try { call(); return undefined; } catch (error) { return error; } };
    const first = open(indexBytes, pin.indexDigest, hostConfiguration);
    expect(refusal(() => open(indexBytes, pin.indexDigest, hostConfiguration))).toBe('cem.schema_asset.owner_busy');
    expect(dispose(first + 1)).toBe(false);
    expect(owners(native)).toBe(1);
    expect(dispose(first)).toBe(true);
    const second = open(indexBytes, pin.indexDigest, hostConfiguration);
    expect(second).not.toBe(first);
    expect(dispose(first)).toBe(false);
    const wrongLength = new Uint8Array(4 * 1024 * 1024);
    const memoryBytes = native.commandNativeMemoryIdentity().buffer.byteLength;
    expect(refusal(() => admit(second, descriptor.packageId, descriptor.version, descriptor.path, wrongLength))).toBe('cem.schema_asset.limit');
    expect(native.commandNativeMemoryIdentity().buffer.byteLength).toBe(memoryBytes);
    native.closeCommandNativeAdmission();
    for (const call of [() => open(indexBytes, pin.indexDigest, hostConfiguration),
        () => admit(second, descriptor.packageId, descriptor.version, descriptor.path, new Uint8Array()),
        () => activate(second, JSON.stringify([csv]))]) {
        expect(refusal(call)).toBe('cem.runtime.admission.closed');
    }
    expect(dispose(first)).toBe(false);
    expect(dispose(second)).toBe(true);
    expect(owners(native)).toBe(0);
});

nativeCase('reimports and reactivates original bytes on a distinct heap without publishing a recovery grant', async () => {
    const native = freshNative(), target = freshNative();
    const open = vi.fn(transport);
    let current = true;
    const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: { native, current: () => current },
        loadHttpResource: open, activationHostConfiguration: hostConfiguration });
    let fresh: CemSchemaAssetPackLoader | undefined;
    try {
        await loader.activatePackages([csv]);
        const calls = open.mock.calls.length;
        current = false; // Original bytes survive; no old native invocation is allowed.
        fresh = await loader.prepareFreshRuntime({ native: target, current: () => true });
        expect(open.mock.calls.length).toBe(calls);
        expect(target.commandNativeMemoryIdentity()).not.toBe(native.commandNativeMemoryIdentity());
        expect(owners(native)).toBe(1);
        expect(owners(target)).toBe(1);
        expect(census(target).replacementAllowed).toBe(false);
        await expect(loader.activatePackages([csv])).rejects.toThrow('cem.schema_asset.stale');
    } finally { fresh?.dispose(); loader.dispose(); }
});

nativeCase('fences cancelled demand and entered-credential host admission without opening an owner', async () => {
    const native = freshNative();
    let credentialEntered = false;
    const loader = makeLoader(native, () => !credentialEntered);
    try {
        credentialEntered = true;
        await expect(loader.activatePackages([csv])).rejects.toThrow('cem.schema_asset.stale');
        credentialEntered = false;
        const controller = new AbortController(); controller.abort();
        await expect(loader.loadResource(demand(controller.signal))).rejects.toThrow();
        expect(owners(native)).toBe(0);
    } finally { loader.dispose(); }
});

nativeCase('runs ordinary command-service validation with the persistent package context', async () => {
    const native = freshNative(), loader = makeLoader(native);
    try {
        const bytes = new TextEncoder().encode('id,name\n1,"Ada\n'), uri = 'memory:command.csv';
        const version = { revision: 1, sha256: createHash('sha256').update(bytes).digest('hex') };
        const runPlan = JSON.parse(native.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [...new TextEncoder().encode(JSON.stringify({
            inputs: [{ uri, rootScope: { defaultContentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } }], outputs: [],
        }))] })));
        const request = { protocolVersion: 1, requestId: 'owned-csv-validation', project: { projectId: 'owned-fixture', revision: 1 },
            resourceVersions: { [uri]: version }, operation: { kind: 'validate', inputIds: ['input:0'], projection: 'text' }, runPlan, resources: {},
            policyStamp: { resolver: 'fixture', safety: 'fixture', budget: 'fixture' } };
        expect(JSON.parse(native.schemaAssetDemandPlanV1(JSON.stringify({ ...request,
            resources: { [uri]: { bytes: [...bytes], identity: { contentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } } },
        }))))
            .toEqual({ kind: 'cem-schema-asset-demand-plan/1', packages: [csv] });
        expect(owners(native)).toBe(0);
        const forbidden = (): never => { throw Error('unexpected publication'); };
        const execute = async (requestId: string) => JSON.parse(await native.executeCommandServiceV1(JSON.stringify({ ...request, requestId }), JSON.stringify({ runtime: 'wasm-node',
            targetIdentity: 'fixture', abiIdentity: 'fixture', debugControlActive: false }),
            () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
            () => JSON.stringify({ version, bytes: [...bytes], identity: { contentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } }),
            forbidden, forbidden, () => undefined));
        const missing = await execute('missing-csv-validation');
        expect(JSON.stringify(missing)).toContain('cem.schema_asset.missing');
        expect(owners(native)).toBe(0);
        await loader.activatePackages([csv]);
        const result = await execute('owned-csv-validation');
        expect(result.error).toBeUndefined();
        expect(JSON.stringify(result)).toContain('cem.csv.unclosed_quote');
        expect(owners(native)).toBe(1);
    } finally { loader.dispose(); }
});

nativeCase('recompiles a captured package resource with the complete original binding group', async () => {
    const { CemSchemaAssetPackRecoveryBinding } = await import('./schema-asset-pack.js');
    const { registerNativeGeneration, releaseNativeGenerationResources, completeNativeRetirement,
        CemNativeGenerationGroup, issueNativeGenerationBindingTicket, resumeNativeGenerationBindings } = await import('./internal/runtime-support/native-generation.js');
    const native = freshNative(), target = freshNative();
    const open = vi.fn(transport), loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: { native, current: () => true }, loadHttpResource: open,
        activationHostConfiguration: hostConfiguration });
    await loader.activatePackages([csv]);
    const participant = registerNativeGeneration(native.commandNativeMemoryIdentity(), 'source', async () => ({ identity: target.commandNativeMemoryIdentity(),
        runtimeId: 'target', runtime: target, current: () => true, discard() { /* Native owner cleanup is asserted through the binding. */ } }));
    const binding = new CemSchemaAssetPackRecoveryBinding({ loader, participant, current: () => true,
        target: runtime => {
            expect(runtime).toBe(target);
            return { runtime: { native: target, current: () => true },
                participant: registerNativeGeneration(target.commandNativeMemoryIdentity(), 'target', async () => { throw Error('unused'); }) };
        } });
    try {
        const group = new CemNativeGenerationGroup([participant]);
        const ticket = issueNativeGenerationBindingTicket(participant, () => true, async () => ({ owner: {}, current: () => true, discard() { /* Native owner cleanup is asserted through the binding. */ }, bind() { /* This fixture has no separate host route. */ } }));
        expect(() => releaseNativeGenerationResources({ ...participant }, () => true)).toThrow('resource');
        expect(() => releaseNativeGenerationResources(participant, () => false)).toThrow('resource');
        expect(owners(native)).toBe(1);
        native.closeCommandNativeAdmission();
        releaseNativeGenerationResources(participant, () => true);
        expect(owners(native)).toBe(0);
        expect(census(native).blockingResources).toBe(0);
        const calls = open.mock.calls.length;
        const admission = await group.allocate([completeNativeRetirement(participant, new Uint8Array([1]))]);
        const resumed = await resumeNativeGenerationBindings(admission, [ticket]);
        expect(open.mock.calls.length).toBe(calls);
        expect(owners(target)).toBe(1);
        expect(census(target).replacementAllowed).toBe(false);
        const source = (JSON.parse(readFileSync(join(pack, 'index.json'), 'utf8')) as { sources: Array<{ packageId: string; version: string; path: string }> }).sources.find(source => source.packageId === 'csv');
        if (!source) throw Error('missing CSV fixture');
        expect((await binding.loadSource(source)).sourceUri).toBe(source.path);
        resumed.revoke();
        expect(owners(target)).toBe(0);
        expect(() => binding.loadSource(source)).toThrow('stale');
    } finally { binding.dispose(); loader.dispose(); }
});

nativeCase('refuses changed source and credential witnesses without releasing a package owner', async () => {
    const { CemSchemaAssetPackRecoveryBinding } = await import('./schema-asset-pack.js');
    const { registerNativeGeneration, releaseNativeGenerationResources, CemNativeGenerationGroup } = await import('./internal/runtime-support/native-generation.js');
    const native = freshNative(), loader = makeLoader(native);
    await loader.activatePackages([csv]);
    let allowed = true;
    const participant = registerNativeGeneration(native.commandNativeMemoryIdentity(), crypto.randomUUID(), async () => { throw Error('unused'); });
    const foreign = registerNativeGeneration({}, crypto.randomUUID(), async () => { throw Error('unused'); });
    expect(() => new CemSchemaAssetPackRecoveryBinding({ loader, participant: foreign, current: () => true,
        target: () => { throw Error('must not prepare'); } })).toThrow('resource-target');
    const binding = new CemSchemaAssetPackRecoveryBinding({ loader, participant, current: () => allowed, target: () => { throw Error('must not prepare'); } });
    try {
        new CemNativeGenerationGroup([participant]);
        allowed = false;
        expect(() => releaseNativeGenerationResources(participant, () => true)).toThrow('resource');
        expect(owners(native)).toBe(1);
        allowed = true;
        await loader.activatePackages([csv]);
        expect(() => releaseNativeGenerationResources(participant, () => true)).toThrow('resource');
        expect(owners(native)).toBe(1);
    } finally { binding.dispose(); }
});

nativeCase('snapshots native methods and normalized transport policy across fresh preparation', async () => {
    const native = freshNative(), target = freshNative(), methods = { ...native };
    const policy = { timeoutMs: 30_000, maxResponseBytes: 16 * 1024 * 1024 };
    const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: { native: methods, current: () => true }, policy,
        loadHttpResource: transport, activationHostConfiguration: hostConfiguration });
    let fresh: CemSchemaAssetPackLoader | undefined;
    try {
        await loader.activatePackages([csv]);
        methods.schemaAssetPackBootstrapV1 = () => { throw Error('mutated control'); };
        methods.schemaAssetOwnerDisposeV1 = () => { throw Error('mutated cleanup'); };
        policy.timeoutMs = 0; policy.maxResponseBytes = 1;
        fresh = await loader.prepareFreshRuntime({ native: target, current: () => true });
        expect(owners(native)).toBe(1); expect(owners(target)).toBe(1);
    } finally { fresh?.dispose(); loader.dispose(); }
    expect(owners(native)).toBe(0); expect(owners(target)).toBe(0);
});


nativeCase('refuses replay after an async activation loses its acknowledgment', async () => {
    const native = freshNative();
    const open = native.schemaAssetOwnerOpenV1, admit = native.schemaAssetOwnerAdmitV1,
        activate = native.schemaAssetOwnerActivateV1, dispose = native.schemaAssetOwnerDisposeV1;
    if (!open || !admit || !activate || !dispose) throw Error('missing native owner ABI');
    let owner: number | undefined, started!: () => void, finish!: () => void;
    const publicationStarted = new Promise<void>(resolve => { started = resolve; });
    const runtime = { current: () => true, identity: () => native.commandNativeMemoryIdentity(),
        bootstrap: async () => native.schemaAssetPackBootstrapV1(),
        summary: async (index: Uint8Array, pin: string) => native.schemaAssetPackSummaryV1(index, pin),
        importSource: async (index: Uint8Array, pin: string, id: string, version: string, path: string, bytes: Uint8Array) =>
            native.schemaAssetPackImportV1(index, pin, id, version, path, bytes),
        activate: async (index: Uint8Array, pin: string, sources: readonly { packageId: string; version: string; path: string; bytes: Uint8Array }[], selection: string) => {
            const activeOwner = open(index, pin, hostConfiguration); owner = activeOwner;
            for (const source of sources) admit(activeOwner, source.packageId, source.version, source.path, source.bytes);
            const result = activate(activeOwner, selection);
            started();
            return new Promise<string>(resolve => { finish = () => resolve(result); });
        }, release: async () => {
            if (owner !== undefined) dispose(owner);
            owner = undefined; return true;
        } };
    const controller = new AbortController();
    const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime, loadHttpResource: transport });
    try {
        const pending = loader.activatePackages([csv], controller.signal);
        await publicationStarted;
        expect(owners(native)).toBe(1);
        controller.abort(Error('lost activation reply'));
        await expect(pending).rejects.toThrow('lost activation reply');
        await vi.waitFor(() => expect(() => loader.prepareRecoverySource()).toThrow('cem.schema_asset.uncertain'));
        await expect(loader.prepareFreshRuntime(runtime)).rejects.toThrow('cem.schema_asset.uncertain');
        finish();
        await expect(loader.loadResource(demand())).rejects.toThrow('cem.schema_asset.uncertain');
    } finally { loader.dispose(); }
    expect(owners(native)).toBe(0);
});

nativeCase('coordinates demand across independent heaps and lazy clients without duplicating owners', async () => {
    const { CemSchemaAssetDemandGroup } = await import('./schema-asset-pack.js');
    const { registerNativeGeneration, discardNativeGenerationResources } = await import('./internal/runtime-support/native-generation.js');
    const group = new CemSchemaAssetDemandGroup({ baseUrl, loadHttpResource: transport, activationHostConfiguration: hostConfiguration });
    const endpoint = (native: Native) => ({ runtime: { native, current: () => true }, current: () => true,
        participant: registerNativeGeneration(native.commandNativeMemoryIdentity(), crypto.randomUUID(), async () => { throw Error('unused'); }),
        target: () => { throw Error('must use the complete recovery group'); } });
    const first = freshNative(), second = freshNative(), lazy = freshNative();
    const a = endpoint(first), b = endpoint(second), c = endpoint(lazy);
    try {
        await group.run(() => undefined, () => a);
        await group.run(() => undefined, () => b);
        expect(owners(first)).toBe(0);
        let finish!: () => void;
        const active = group.run(() => new Promise<void>(resolve => { finish = resolve; }));
        await vi.waitFor(() => expect(finish).toBeTypeOf('function'));
        const loading = group.loadResource(demand());
        expect(() => group.assertReady()).toThrow('busy');
        expect(owners(first)).toBe(0);
        finish(); await active;
        const response = await loading;
        expect(response.response.contentType).toBe('application/json');
        expect([owners(first), owners(second), owners(lazy)]).toEqual([1, 1, 0]);
        await group.run(() => expect(owners(lazy)).toBe(1), () => c);
        await group.run(() => undefined, () => a);
        await expect(group.run(() => undefined, () => ({ ...a, participant: { ...a.participant } }))).rejects.toThrow('resource-target');
        expect([owners(first), owners(second), owners(lazy)]).toEqual([1, 1, 1]);
        group.assertReady();
        discardNativeGenerationResources(c.participant);
        expect(owners(lazy)).toBe(0);
        expect(group.status()).toMatchObject({ runtimeCount: 2, demandCount: 1 });
    } finally { group.dispose(); }
    expect([owners(first), owners(second), owners(lazy)]).toEqual([0, 0, 0]);
});

nativeCase('prepares bounded native requirements without startup demand, duplicate downloads or mutable plans', async () => {
    const { CemSchemaAssetDemandGroup } = await import('./schema-asset-pack.js');
    const { registerNativeGeneration } = await import('./internal/runtime-support/native-generation.js');
    const open = vi.fn(transport);
    const group = new CemSchemaAssetDemandGroup({ baseUrl, loadHttpResource: open, activationHostConfiguration: hostConfiguration });
    const natives = [freshNative(), freshNative()];
    try {
        await group.prepare({ kind: 'cem-schema-asset-demand-plan/1', packages: [] });
        expect(open).not.toHaveBeenCalled();
        for (const native of natives) await group.run(() => undefined, () => ({ runtime: { native, current: () => true },
            participant: registerNativeGeneration(native.commandNativeMemoryIdentity(), crypto.randomUUID(), async () => { throw Error('unused'); }),
            current: () => true, target: () => { throw Error('unused'); } }));
        expect(natives.map(owners)).toEqual([0, 0]);
        const cancelled = new AbortController(); cancelled.abort();
        await expect(group.prepare({ kind: 'cem-schema-asset-demand-plan/1', packages: [csv] }, cancelled.signal)).rejects.toThrow('stale');
        await expect(group.prepare({ kind: 'cem-schema-asset-demand-plan/1', packages: Array(65).fill(csv) })).rejects.toThrow('response');
        expect(open).not.toHaveBeenCalled();
        const plan = { kind: 'cem-schema-asset-demand-plan/1' as const, packages: [{ ...csv }, { ...csv }] };
        const preparing = group.prepare(plan);
        plan.packages[0].packageId = 'ungranted'; plan.packages.push({ packageId: 'scss', version: '1.0.0' });
        await preparing;
        await group.run(() => expect(natives.map(owners)).toEqual([1, 1]));
        expect(group.status()).toMatchObject({ state: 'idle', runtimeCount: 2, demandCount: 1 });
        const reads = open.mock.calls.length;
        await group.prepare({ kind: 'cem-schema-asset-demand-plan/1', packages: [csv, csv] });
        expect(open).toHaveBeenCalledTimes(reads);
    } finally { group.dispose(); }
    expect(natives.map(owners)).toEqual([0, 0]);
    await expect(group.prepare({ kind: 'cem-schema-asset-demand-plan/1', packages: [] })).rejects.toThrow('stale');
});

nativeCase('finishes publication after its waiter cancels and a completed physical host closes', async () => {
    const { CemSchemaAssetDemandGroup } = await import('./schema-asset-pack.js');
    const { registerNativeGeneration, discardNativeGenerationResources } = await import('./internal/runtime-support/native-generation.js');
    let release!: () => void, started!: () => void, indexes = 0;
    const paused = new Promise<void>(resolve => { release = resolve; });
    const publishing = new Promise<void>(resolve => { started = resolve; });
    const group = new CemSchemaAssetDemandGroup({ baseUrl, activationHostConfiguration: hostConfiguration,
        loadHttpResource: async request => {
            if (request.resolvedUrl.endsWith('/index.json') && ++indexes === 2) { started(); await paused; }
            return transport(request);
        } });
    const natives = [freshNative(), freshNative(), freshNative()];
    const participants = natives.map(native => registerNativeGeneration(native.commandNativeMemoryIdentity(), crypto.randomUUID(), async () => { throw Error('unused'); }));
    try {
        for (const native of natives) await group.run(() => undefined, () => ({ runtime: { native, current: () => true },
            participant: participants[natives.indexOf(native)],
            current: () => true, target: () => { throw Error('unused'); } }));
        const controller = new AbortController();
        const preparing = group.prepare({ kind: 'cem-schema-asset-demand-plan/1', packages: [csv] }, controller.signal);
        await publishing;
        expect(natives.map(owners)).toEqual([1, 0, 0]);
        controller.abort(Error('command cancelled'));
        await expect(preparing).rejects.toThrow('command cancelled');
        expect(() => group.assertReady()).toThrow('busy');
        discardNativeGenerationResources(participants[0]);
        const nextCommand = group.run(() => expect(natives.map(owners)).toEqual([0, 1, 1]));
        release(); await nextCommand;
        expect(group.status()).toMatchObject({ state: 'idle', demandCount: 1 });
    } finally { release(); group.dispose(); }
    expect(natives.map(owners)).toEqual([0, 0, 0]);
});

nativeCase('fences a partially activated demand group and preserves entered-credential recovery refusal', async () => {
    const { CemSchemaAssetDemandGroup, cemSchemaAssetTransportFromBinding } = await import('./schema-asset-pack.js');
    const { registerNativeGeneration, CemNativeGenerationGroup, releaseNativeGenerationResources } = await import('./internal/runtime-support/native-generation.js');
    const first = freshNative(), denied = freshNative();
    let credentialsClear = true;
    const participant = registerNativeGeneration(first.commandNativeMemoryIdentity(), crypto.randomUUID(), async () => { throw Error('unused'); });
    const group = new CemSchemaAssetDemandGroup({ baseUrl, loadHttpResource: transport, activationHostConfiguration: hostConfiguration });
    try {
        await group.run(() => undefined, () => ({ runtime: { native: first, current: () => true }, participant,
            current: () => credentialsClear, target: () => { throw Error('must not prepare'); } }));
        await group.loadResource(demand());
        credentialsClear = false;
        expect(() => new CemNativeGenerationGroup([participant])).toThrow('stale');
        expect(owners(first)).toBe(1);
        credentialsClear = true;
        const native = denied;
        const runtime = cemSchemaAssetTransportFromBinding({ current: () => true,
            memory: () => native.commandNativeMemoryIdentity(),
            call: (name, args) => Reflect.apply(native[name as keyof Native] as (...args: unknown[]) => unknown, native, args) }, '{}');
        await expect(group.run(() => { throw Error('must not publish'); }, () => ({ runtime,
            participant: registerNativeGeneration(native.commandNativeMemoryIdentity(), crypto.randomUUID(), async () => { throw Error('unused'); }),
            current: () => true, target: () => { throw Error('unused'); } }))).rejects.toThrow('activation');
        await expect(group.run(() => undefined)).rejects.toThrow('stale');
        expect(() => releaseNativeGenerationResources(participant, () => true)).toThrow('resource');
        await group.cleanup(() => undefined);
    } finally { group.dispose(); }
    expect([owners(first), owners(denied)]).toEqual([0, 0]);
});

nativeCase('requires the complete demand cohort and reuses staged owners during fresh host publication', async () => {
    const { CemSchemaAssetDemandGroup } = await import('./schema-asset-pack.js');
    const { registerNativeGeneration, CemNativeGenerationGroup, releaseNativeGenerationResources,
        completeNativeRetirement, issueNativeGenerationBindingTicket, resumeNativeGenerationBindings } = await import('./internal/runtime-support/native-generation.js');
    const originals = [freshNative(), freshNative()], targets = [freshNative(), freshNative()];
    const targetImports = targets.map(native => vi.spyOn(native, 'schemaAssetPackImportV1'));
    const ids = targets.map(() => crypto.randomUUID());
    const targetParticipants: import('./internal/runtime-support/native-generation.js').CemNativeGenerationParticipant[] = [];
    const participants = originals.map((native, index) => registerNativeGeneration(native.commandNativeMemoryIdentity(), crypto.randomUUID(), async () => ({
        identity: targets[index].commandNativeMemoryIdentity(), runtimeId: ids[index], runtime: targets[index], current: () => true, discard() { /* Binding owns cleanup. */ },
    })));
    const open = vi.fn(transport);
    const group = new CemSchemaAssetDemandGroup({ baseUrl, loadHttpResource: open, activationHostConfiguration: JSON.stringify({ schemaPackageReplacementGrants: [{ packageId: 'cem-element-template', expectedOrigin: { kind: 'builtin' }, replacementManifestUri: 'schema-packages/cem-element-template/v1/package.cem' }] }) });
    const endpoint = (native: Native, index: number, participant = participants[index]) => ({
        runtime: { native, current: () => true }, participant, current: () => true,
        target: (runtime: object) => {
            expect(runtime).toBe(targets[index]);
            targetParticipants[index] ??= registerNativeGeneration(targets[index].commandNativeMemoryIdentity(), ids[index], async () => { throw Error('unused'); });
            return { runtime: { native: targets[index], current: () => true }, participant: targetParticipants[index] };
        },
    });
    try {
        for (let index = 0; index < originals.length; index++) await group.run(() => undefined, () => endpoint(originals[index], index));
        await group.loadResource({ ...demand(), resolvedUrl: `${baseUrl}demand/cem-element-template/1.0.0` });
        expect(() => new CemNativeGenerationGroup([participants[0]])).toThrow('coverage');
        expect(originals.map(owners)).toEqual([1, 1]);
        expect(() => releaseNativeGenerationResources(participants[0], () => true)).toThrow('resource');
        const complete = new CemNativeGenerationGroup(participants);
        const tickets = participants.map((participant, index) => issueNativeGenerationBindingTicket(participant, () => true, async () => {
            await group.run(() => expect(owners(targets[index])).toBe(1), () => endpoint(targets[index], index, targetParticipants[index]));
            return { owner: {}, current: () => true, discard() { /* Binding owns cleanup. */ }, bind() { /* No separate host route in this fixture. */ } };
        }));
        for (let index = 0; index < originals.length; index++) {
            originals[index].closeCommandNativeAdmission(); releaseNativeGenerationResources(participants[index], () => true);
        }
        expect(originals.map(owners)).toEqual([0, 0]);
        const retirements = participants.map(participant => completeNativeRetirement(participant, new Uint8Array([1])));
        await expect(complete.allocate(retirements.slice(0, 1))).rejects.toThrow('coverage');
        const reads = open.mock.calls.length;
        const admission = await complete.allocate(retirements);
        const resumed = await resumeNativeGenerationBindings(admission, tickets);
        expect(open.mock.calls.length).toBe(reads);
        for (const imported of targetImports) expect(imported).toHaveBeenCalledTimes(2);
        expect(group.status()).toEqual({ state: 'idle', runtimeCount: 2, demandCount: 1 });
        for (let index = 0; index < targets.length; index++) await group.run(() => expect(owners(targets[index])).toBe(1), () => endpoint(targets[index], index, targetParticipants[index]));
        expect(() => new CemNativeGenerationGroup([targetParticipants[0]])).toThrow('coverage');
        resumed.revoke(); expect(targets.map(owners)).toEqual([0, 0]);
    } finally { group.dispose(); }
});
