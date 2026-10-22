import { describe, expect, it, vi } from 'vitest';
import { prepareCemCheckpointProcessing, CemCheckpointWriteSuppression } from './checkpoint-processing.js';
import { CemNativeCheckpointDomain } from './internal/runtime-support/checkpoint-domain.js';
import { createFreshCemProcessingHost } from './internal/runtime-support/processing-host-runtime.js';
import type { CemProcessingHost } from './internal/runtime-support/processing-host.js';
import type { CemCheckpointResumeContext } from './checkpoint-resume.js';
import type { CemCheckpointRuntimeState } from './checkpoint-controls.js';
vi.mock('./internal/runtime-support/processing-host-runtime.js', () => ({ createFreshCemProcessingHost: vi.fn() }));
const writers = [{ index: 0, trigger: 'saved:1', implicit: false }, { index: 1, trigger: null, implicit: true }];
describe('consumed checkpoint writes', () => {
    it('suppresses every writer until accepted input, without consuming a newly exposed token', () => {
        const writes = new CemCheckpointWriteSuppression(3, writers);
        for (let n = 0; n < 3; n++) {
            expect(writes.consume(3)).toBe(false);
            expect(writes.consume(3, 0, 'saved:1')).toBe(false);
            expect(writes.consume(3, 0, 'saved:2')).toBe(false);
            expect(writes.consume(3, 1)).toBe(false);
            expect(writes.consume(3, 77, 'conditional')).toBe(false);
        }
        expect(writes.consume(4, 0, 'saved:1')).toBe(false);
        expect(writes.consume(4, 0, 'saved:2')).toBe(true);
        expect(writes.consume(4, 0, 'saved:2')).toBe(false);
        expect(writes.consume(4, 1)).toBe(true); expect(writes.consume(4)).toBe(true);
        expect(writes.consume(4, 77, 'conditional')).toBe(true);
        expect(() => writes.consume(3)).toThrow('regression');
        expect(() => writes.consume(Number.NaN)).toThrow('input');
        expect(() => writes.consume(4, 256, 'too-many')).toThrow('writer');
        expect(() => writes.consume(4, 2, 'x'.repeat(1025))).toThrow('writer');
        expect(() => writes.consume(4, undefined, 'unknown')).toThrow('writer');
    });
});
function fixture() {
    const domain = new CemNativeCheckpointDomain('worker'), controller = new AbortController();
    let active = false, sequence = 3;
    const context: CemCheckpointResumeContext = { previousRuntimeId: 'old', runtimeId: domain.issuedIdentity(), runtime: {},
        receipt: { instanceId: 'editor', declarationIdentity: 'template:1', dataRevision: '2', acceptedEventSequence: 3 },
        island: '<template>opaque é😀</template>', active: () => active, signal: controller.signal };
    const state = { version: 'cem-checkpoint-runtime/1', capability: null, writers, writeDisposition: 'suppress-until-new-input',
        resources: [{ name: 'catalog', kind: 'http', disposition: 'reimport-and-read-again', state: 'loaded', revision: 1, cursor: 0, live: false,
            url: 'https://example.test/data.xml', method: 'GET', document: { base64: btoa('<data>original</data>'), contentType: 'application/xml', sourceUri: 'https://example.test/data.xml', digest: 'native' } }] } as CemCheckpointRuntimeState;
    const checkpoint = vi.fn(input => ({ jobId: 1, result: Promise.resolve({ runtime: JSON.stringify(state) }) }));
    const document = vi.fn(input => ({ jobId: 2, result: Promise.resolve({ handle: input.handle, retained: input.action === 'retain' }) }));
    const host = { checkpointClient: domain.client(), checkpoint, document, compile: vi.fn(input => ({ jobId: 3, result: Promise.resolve({}) })) } as unknown as CemProcessingHost;
    vi.mocked(createFreshCemProcessingHost).mockReturnValue(host);
    const options = { scope: {} as CemProcessingHost['ownerScope'], sourceUri: 'https://example.test/owner', scopePolicyStamp: 'scope',
        controls: 'native controls', runtime: 'native runtime', acceptedEventSequence: () => sequence, allowSource: vi.fn(async () => true) };
    return { context, options, document, checkpoint, host, controller, domain, activate: () => { active = true; }, input: () => sequence++ };
}
describe('shared fresh checkpoint processing', () => {
    it('prepares native islands and original sources before exposing evaluation, and cleans exactly once', async () => {
        const f = fixture(), prepared = await prepareCemCheckpointProcessing(f.context, f.options);
        expect(f.checkpoint).toHaveBeenCalledWith(expect.objectContaining({ island: f.context.island }));
        expect(f.options.allowSource).toHaveBeenCalledOnce();
        expect(f.document).toHaveBeenCalledWith(expect.objectContaining({ action: 'retain', bytes: new TextEncoder().encode('<data>original</data>').buffer }));
        expect(() => prepared.owner.host).toThrow('inactive'); expect(() => prepared.owner.consumeWrite()).toThrow('inactive');
        f.activate(); expect(prepared.owner.host).toBe(f.host); expect(prepared.owner.sources).toHaveLength(1);
        const exported = prepared.owner.runtimeState; exported.writers[0].trigger = 'modified';
        expect(prepared.owner.consumeWrite(0, 'saved:2')).toBe(false);
        f.input(); expect(prepared.owner.consumeWrite(0, 'saved:1')).toBe(false); expect(prepared.owner.consumeWrite(0, 'saved:2')).toBe(true);
        prepared.discard(); prepared.discard();
        expect(f.document.mock.calls.filter(([i]) => i.action === 'release')).toHaveLength(1);
        expect(prepared.current()).toBe(false); expect(() => prepared.owner.host).toThrow('stale');
    });
    it('refuses denied source policy before importing or evaluating, preserving input bytes', async () => {
        const f = fixture(); f.options.allowSource.mockResolvedValue(false);
        await expect(prepareCemCheckpointProcessing(f.context, f.options)).rejects.toThrow('source-policy');
        expect(f.document).not.toHaveBeenCalled(); expect(f.context.island).toContain('é😀');
    });
    it('cleans a late import after cancellation and never exposes the host', async () => {
        const f = fixture(); let finish!: () => void;
        f.document.mockImplementationOnce(input => ({ jobId: 2, result: new Promise(resolve => { finish = () => resolve({ handle: input.handle, retained: true }); }) }));
        const pending = prepareCemCheckpointProcessing(f.context, f.options);
        await vi.waitFor(() => expect(finish).toBeTypeOf('function')); f.controller.abort(); finish();
        await expect(pending).rejects.toThrow('stale');
        expect(f.document.mock.calls.filter(([i]) => i.action === 'release')).toHaveLength(1);
    });
    it('refuses newer accepted input during source-policy preparation before importing', async () => {
        const f = fixture(); f.options.allowSource.mockImplementationOnce(async () => { f.input(); return true; });
        await expect(prepareCemCheckpointProcessing(f.context, f.options)).rejects.toThrow('stale');
        expect(f.document).not.toHaveBeenCalled();
    });
    it('rejects native validation failures before policy approval or source import', async () => {
        const f = fixture(); f.checkpoint.mockImplementationOnce(() => ({ jobId: 1, result: Promise.reject(Error('invalid native controls')) }));
        await expect(prepareCemCheckpointProcessing(f.context, f.options)).rejects.toThrow('native controls');
        expect(f.options.allowSource).not.toHaveBeenCalled(); expect(f.document).not.toHaveBeenCalled();
    });
});
