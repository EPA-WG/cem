import { afterEach, describe, expect, it, vi } from 'vitest';
import { DeclarationStyleOwnership } from '../../declaration-style-ownership.js';
import { CemCssDomPublicationQueue, type PreparedCssDomPublication } from './css-dom-publication-queue.js';

afterEach(() => vi.restoreAllMocks());

function fixture() {
    const element = { isConnected: true } as HTMLElement;
    const queue = CemCssDomPublicationQueue.forElement(element);
    const request = { entries: [], patch: { container: element }, currentRevision: () => ({}) } as PreparedCssDomPublication;
    const commit = vi.spyOn(DeclarationStyleOwnership, 'commitGroupWithPatch').mockReturnValue({ status: 'applied', diagnostics: [], errors: [] });
    return { element, queue, request, commit };
}

describe('CSS and DOM publication queue', () => {
    it('shares one queue per host and postpones reentrant preparation until publication returns', async () => {
        const f = fixture();
        expect(CemCssDomPublicationQueue.forElement(f.element)).toBe(f.queue);
        const events: string[] = [];
        let nested: ReturnType<typeof f.queue.publish> | undefined;
        f.commit.mockImplementationOnce(() => {
            nested = f.queue.publish(() => { events.push('prepare second'); return f.request; });
            expect(events).toEqual(['prepare first']);
            events.push('finish first');
            return { status: 'applied', diagnostics: [], errors: [] };
        });
        await f.queue.publish(() => { events.push('prepare first'); return f.request; });
        await nested;
        expect(events).toEqual(['prepare first', 'finish first', 'prepare second']);
    });

    it('blocks preparation after failure and only resumes after successful authoritative recovery', async () => {
        const f = fixture();
        f.commit.mockReturnValueOnce({ status: 'recovery-required', diagnostics: [], errors: [] });
        await f.queue.publish(() => f.request);
        const prepare = vi.fn(() => f.request);
        expect(await f.queue.publish(prepare)).toMatchObject({ status: 'rejected' });
        expect(prepare).not.toHaveBeenCalled();
        await expect(f.queue.recover(() => { throw new Error('restore failed'); })).rejects.toThrow('restore failed');
        expect(f.queue.recoveryRequired).toBe(true);
        let finish!: () => void;
        const held = new Promise<void>(resolve => { finish = resolve; });
        const recovery = f.queue.recover(() => held);
        const next = f.queue.publish(prepare);
        await Promise.resolve();
        expect(prepare).not.toHaveBeenCalled();
        finish(); await recovery;
        expect(await next).toMatchObject({ status: 'applied' });
        expect(f.queue.recoveryRequired).toBe(false);
    });

    it('keeps failed preparation separate from publication failure', async () => {
        const f = fixture();
        const error = new Error('load failed');
        expect(await f.queue.publish(() => { throw error; })).toEqual({ status: 'rejected', diagnostics: [], errors: [error] });
        expect(f.queue.recoveryRequired).toBe(false);
        expect(await f.queue.publish(() => f.request)).toMatchObject({ status: 'applied' });
    });

    it('routes foreign-host candidates through rejection cleanup', async () => {
        const f = fixture();
        await f.queue.publish(() => ({ ...f.request, patch: { ...f.request.patch, container: {} as Node } }));
        expect(f.commit.mock.calls[0][3]?.aborted).toBe(true);
    });

    it('requires recovery after unexpected publication exceptions and rejects disconnected recovery', async () => {
        const f = fixture();
        f.commit.mockImplementationOnce(() => { throw new Error('publication'); });
        expect(await f.queue.publish(() => f.request)).toMatchObject({ status: 'recovery-required' });
        Object.defineProperty(f.element, 'isConnected', { value: false });
        await expect(f.queue.recover(() => undefined)).rejects.toThrow('connected host');
        expect(f.queue.recoveryRequired).toBe(true);
    });
});
