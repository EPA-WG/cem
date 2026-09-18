import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({ retain: vi.fn(), dispose: vi.fn(), ready: vi.fn(), hash: vi.fn() }));
vi.mock('../../../../../cem_ql/dist/wasm/cem_ql.js', () => ({
    retainCemtXPathFunctions: mocks.retain, disposeCemtXPathFunctions: mocks.dispose,
}));
vi.mock('./cem-ql-render.js', () => ({
    ensureRuntimeReady: mocks.ready, cemMlTemplateArtifactPayloadKey: mocks.hash,
}));
import { CemXPathFunctionLibraries, identifyXPathFunctionLibrary } from './xpath-function-library.js';

beforeEach(() => {
    vi.resetAllMocks();
    mocks.hash.mockImplementation(async (source: string) => ({ sourceHash: `hash:${source}`, cemMlVersion: '1', cemQlVersion: '1' }));
    let id = 0;
    mocks.retain.mockImplementation((source: string) => JSON.stringify({ companionId: ++id,
        contentType: 'application/vnd.cem.cemt-xpath-functions+cem-bin', formatVersion: 'cemt-xpath-functions/1', sourceHash: `hash:${source}` }));
});
const source = () => identifyXPathFunctionLibrary('library', 'https://example.test/functions.cemt', 'policy');

describe('independently retained XPath library dependencies', () => {
    it('shares concurrent consumers and releases only after the last reference', async () => {
        const cache = new CemXPathFunctionLibraries();
        const dependency = await source();
        const [first, second] = await Promise.all([cache.acquire(dependency), cache.acquire(dependency)]);
        expect(first.companionId).toBe(second.companionId);
        expect(mocks.retain).toHaveBeenCalledTimes(1);
        first.release(); first.release();
        expect(mocks.dispose).not.toHaveBeenCalled();
        second.release();
        expect(mocks.dispose).toHaveBeenCalledExactlyOnceWith(first.companionId);
        const replacement = await cache.acquire(dependency);
        expect(replacement.companionId).not.toBe(first.companionId);
        cache.dispose(); replacement.release();
        expect(mocks.dispose).toHaveBeenCalledTimes(2);
        await expect(cache.acquire(dependency)).rejects.toThrow('disposed');
    });
    it('keys URL, policy, source hash and source bytes independently of templates', async () => {
        const cache = new CemXPathFunctionLibraries();
        const dependency = await source();
        const first = await cache.acquire(dependency);
        for (const changed of [
            { ...dependency, uri: 'https://other.test/functions.cemt' },
            { ...dependency, resolverPolicyStamp: 'another policy' },
            await identifyXPathFunctionLibrary('changed', dependency.uri, dependency.resolverPolicyStamp),
        ]) {
            const other = await cache.acquire(changed);
            expect(other.companionId).not.toBe(first.companionId);
        }
        await expect(cache.acquire({ ...dependency, source: 'forged' })).rejects.toThrow('identity mismatch');
        await expect(cache.acquire({ ...dependency, cemQlVersion: 'other' })).rejects.toThrow('identity mismatch');
        expect(mocks.retain).toHaveBeenCalledTimes(4);
        cache.dispose();
        expect(mocks.dispose).toHaveBeenCalledTimes(4);
    });
    it('retries failed compilation and cleans up metadata mismatch', async () => {
        const cache = new CemXPathFunctionLibraries();
        const dependency = await source();
        mocks.retain.mockImplementationOnce(() => { throw new Error('invalid XPath'); });
        await expect(cache.acquire(dependency)).rejects.toThrow('invalid XPath');
        mocks.retain.mockReturnValueOnce(JSON.stringify({ companionId: 42, sourceHash: 'wrong' }));
        await expect(cache.acquire(dependency)).rejects.toThrow('identity mismatch');
        expect(mocks.dispose).toHaveBeenCalledWith(42);
        const valid = await cache.acquire(dependency);
        valid.release();
        expect(mocks.dispose).toHaveBeenCalledTimes(2);
    });
    it('cleans up a pending import when its host is disposed', async () => {
        const cache = new CemXPathFunctionLibraries();
        const dependency = await source();
        let resume!: () => void;
        mocks.ready.mockImplementationOnce(() => new Promise<void>((resolve) => { resume = resolve; }));
        const pending = cache.acquire(dependency);
        await vi.waitFor(() => expect(resume).toBeTypeOf('function'));
        cache.dispose(); resume();
        await expect(pending).rejects.toThrow('disposed');
        expect(mocks.dispose).toHaveBeenCalledTimes(1);
    });
    it('bounds source bytes, URLs, and retained dependency count', async () => {
        await expect(identifyXPathFunctionLibrary('🍒'.repeat(9000), 'https://example.test/a', 'p')).rejects.toThrow('source limit');
        await expect(identifyXPathFunctionLibrary('x', './relative', 'p')).rejects.toThrow();
        const cache = new CemXPathFunctionLibraries();
        const dependency = await source();
        for (let index = 0; index < 64; index++) await cache.acquire({ ...dependency, uri: `https://example.test/${index}` });
        await expect(cache.acquire(dependency)).rejects.toThrow('count exceeds limit');
        cache.dispose();
        expect(mocks.dispose).toHaveBeenCalledTimes(64);
    });
});
