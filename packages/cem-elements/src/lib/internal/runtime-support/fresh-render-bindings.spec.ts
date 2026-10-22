import { describe, expect, it, vi } from 'vitest';
import { CemNativeCheckpointDomain } from './checkpoint-domain.js';
import { CemXPathFunctionLibraries, identifyXPathFunctionLibrary } from './xpath-function-library.js';
import { createCemQlRenderBindings } from './cem-ql-render.js';
import type { CemQlNativeBindingContext } from './native-runtime.js';

vi.mock('./native-runtime.js', () => ({ default: vi.fn(), nativeRuntimeUsable: () => true }));
function fixture(label: string) {
    const domain = new CemNativeCheckpointDomain('main-thread');
    let usable = true, next = 0;
    const disposed: number[] = [], disposedXPath: number[] = [];
    const initialize = vi.fn(async () => undefined);
    const native = {
        runtimeMemoryStatus: () => JSON.stringify({ softPressure: false, hardPressure: false }),
        templateArtifactPayloadKey: () => JSON.stringify({ contentType: 'cem-template-artifact', sourceHash: label,
            cemMlVersion: '1', cemQlVersion: '1', sourceMapMode: 'dev' }),
        retainCemDocument: () => ++next,
        disposeCemDocument: (id: number) => { disposed.push(id); },
        retainCemtXPathFunctions: () => JSON.stringify({ companionId: ++next, sourceHash: label,
            contentType: 'application/vnd.cem.cemt-xpath-functions+cem-bin', formatVersion: 'cemt-xpath-functions/1' }),
        disposeCemtXPathFunctions: (id: number) => { disposedXPath.push(id); },
    } as unknown as CemQlNativeBindingContext['native'];
    const context = { domain, native, initialize, usable: () => usable };
    const bindings = createCemQlRenderBindings(context);
    return { bindings, context, initialize, disposed, disposedXPath, retire: () => { usable = false; domain.revoke(); } };
}
describe('explicit generation-bound render helpers', () => {
    it('keeps initialization, source IDs and helper identities independent across interleaved heaps', async () => {
        const old = fixture('old'), fresh = fixture('fresh');
        const [a, b] = await Promise.all([
            old.bindings.cemMlTemplateArtifactPayloadKey('a', 'dev'),
            fresh.bindings.cemMlTemplateArtifactPayloadKey('b', 'dev'),
        ]);
        expect([a.sourceHash, b.sourceHash]).toEqual(['old', 'fresh']);
        const bytes = new TextEncoder().encode('<doc>é😀</doc>').buffer;
        expect(await old.bindings.retainLoadedCemDocument(bytes, 'text/xml', 'https://example.test/a')).toBe(1);
        expect(await fresh.bindings.retainLoadedCemDocument(bytes, 'text/xml', 'https://example.test/b')).toBe(1);
        old.bindings.disposeLoadedCemDocument(1);
        expect(old.disposed).toEqual([1]); expect(fresh.disposed).toEqual([]);
        old.context.native = fresh.context.native;
        expect((await old.bindings.cemMlTemplateArtifactPayloadKey('a', 'dev')).sourceHash).toBe('old');
        old.retire();
        await expect(old.bindings.cemMlTemplateArtifactPayloadKey('b', 'dev')).rejects.toThrow();
        expect((await fresh.bindings.cemMlTemplateArtifactPayloadKey('b', 'dev')).sourceHash).toBe('fresh');
        expect(old.initialize).toHaveBeenCalledTimes(1); expect(fresh.initialize).toHaveBeenCalledTimes(1);
        fresh.bindings.disposeLoadedCemDocument(1);
    });
    it('reacquires executable XPath dependencies using only the selected fresh context', async () => {
        const old = fixture('old'), fresh = fixture('fresh');
        const a = new CemXPathFunctionLibraries(old.bindings), b = new CemXPathFunctionLibraries(fresh.bindings);
        const source = async (bindings: typeof old.bindings) => identifyXPathFunctionLibrary('library',
            'https://example.test/functions.cemt', 'policy', bindings.cemMlTemplateArtifactPayloadKey);
        const first = await a.acquire(await source(old.bindings));
        const second = await b.acquire(await source(fresh.bindings));
        expect(first.companionId).toBe(second.companionId);
        first.release(); expect(old.disposedXPath).toEqual([1]); expect(fresh.disposedXPath).toEqual([]);
        a.dispose(); old.retire();
        second.release(); expect(fresh.disposedXPath).toEqual([1]);
        const next = await b.acquire(await source(fresh.bindings)); expect(next.companionId).toBe(2);
        b.dispose(); expect(fresh.disposedXPath).toEqual([1, 2]);
    });

});
