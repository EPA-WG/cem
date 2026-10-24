import { createHash } from 'node:crypto';
import { createRequire } from 'node:module';
import { cpSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { afterAll, expect, it, vi } from 'vitest';
import { CemSchemaAssetPackLoader, type CemSchemaAssetNativeRuntime } from './schema-asset-pack.js';
import type { CemHttpRequest, CemHttpResourceLoadResult } from './cem-elements.js';

const root = resolve(import.meta.dirname, '../../..');
const pack = join(root, 'cem-ml-npm/dist-schema-assets');
const modulePath = join(root, 'cem-ml-npm/dist-schema-assets-wasm/wasm/node/cem_ml.js');
const temporary = mkdtempSync(join(tmpdir(), 'cem-pack-activation-'));
afterAll(() => rmSync(temporary, { recursive: true, force: true }));
interface Native extends CemSchemaAssetNativeRuntime {
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
    return createRequire(import.meta.url)(join(path, 'cem_ml.js')) as Native;
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
        await loader.activatePackages([csv]);
        const bytes = new TextEncoder().encode('id,name\n1,"Ada\n'), uri = 'memory:command.csv';
        const version = { revision: 1, sha256: createHash('sha256').update(bytes).digest('hex') };
        const runPlan = JSON.parse(native.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [...new TextEncoder().encode(JSON.stringify({
            inputs: [{ uri, rootScope: { defaultContentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } }], outputs: [],
        }))] })));
        const request = { protocolVersion: 1, requestId: 'owned-csv-validation', project: { projectId: 'owned-fixture', revision: 1 },
            resourceVersions: { [uri]: version }, operation: { kind: 'validate', inputIds: ['input:0'], projection: 'text' }, runPlan, resources: {},
            policyStamp: { resolver: 'fixture', safety: 'fixture', budget: 'fixture' } };
        const forbidden = (): never => { throw Error('unexpected publication'); };
        const result = JSON.parse(await native.executeCommandServiceV1(JSON.stringify(request), JSON.stringify({ runtime: 'wasm-node',
            targetIdentity: 'fixture', abiIdentity: 'fixture', debugControlActive: false }),
            () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
            () => JSON.stringify({ version, bytes: [...bytes], identity: { contentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } }),
            forbidden, forbidden, () => undefined));
        expect(result.error).toBeUndefined();
        expect(JSON.stringify(result)).toContain('cem.csv.unclosed_quote');
        expect(owners(native)).toBe(1);
    } finally { loader.dispose(); }
});
