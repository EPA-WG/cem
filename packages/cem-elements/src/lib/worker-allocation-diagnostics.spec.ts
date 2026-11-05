import { expect, it } from 'vitest';
import { createWorkerAllocationDiagnostics } from './retained-media.fixtures.js';

class WorkerProbe extends EventTarget {
    calls: unknown[][] = [];
    stopped = false;
    postMessage(...args: unknown[]) { this.calls.push(args); }
    terminate() { this.stopped = true; }
}

it('observes the original worker without changing payloads/transfers or retaining their contents', () => {
    let now = 100;
    const diagnostics = createWorkerAllocationDiagnostics(() => now), worker = new WorkerProbe();
    diagnostics.observe(worker as unknown as Worker, true);
    diagnostics.phase('allocation-start');
    const secret = { credential: 'must-not-be-logged' }, transfer = [new ArrayBuffer(4)];
    worker.postMessage(secret, transfer);
    expect(worker.calls[0][0]).toBe(secret); expect(worker.calls[0][1]).toBe(transfer);
    now = 110; worker.dispatchEvent(new MessageEvent('message', { data: { direction: 'ready' } }));
    const request = { protocol: 'cem-rendering-retirement-v2', direction: 'request', action: 'inspect' };
    now = 115; worker.postMessage(request);
    now = 130; worker.dispatchEvent(new MessageEvent('message', { data: { ...request, direction: 'response', ok: true, secret } }));
    diagnostics.phase('allocation-admitted');
    now = 140; worker.terminate(); expect(worker.stopped).toBe(true);
    expect(diagnostics.snapshot().workers[0]).toEqual({ fresh: true, events: [
        { phase: 'created', atMs: 0 }, { phase: 'transport-ready', atMs: 10 },
        { phase: 'headroom-request', atMs: 15 }, { phase: 'headroom-ready', atMs: 30 }, { phase: 'terminated', atMs: 40 },
    ] });
    expect(JSON.stringify(diagnostics.snapshot())).not.toContain('credential');
    expect(JSON.stringify(diagnostics.snapshot())).not.toContain('must-not-be-logged');
    expect(diagnostics.snapshot().allocations).toEqual([{ phase: 'allocation-start', atMs: 0 }, { phase: 'allocation-admitted', atMs: 30 }]);
    diagnostics.snapshot().workers[0].events[0].phase = 'forged';
    diagnostics.snapshot().allocations[0].phase = 'forged';
    expect(diagnostics.snapshot().workers[0].events[0].phase).toBe('created');
    expect(diagnostics.snapshot().allocations[0].phase).toBe('allocation-start');
});

it('bounds diagnostic retention while preserving errors, refusal and original worker delivery', () => {
    const diagnostics = createWorkerAllocationDiagnostics(() => 0);
    for (let i = 0; i < 17; i++) {
        const worker = new WorkerProbe(); diagnostics.observe(worker as unknown as Worker, false);
        worker.dispatchEvent(new Event('error')); worker.dispatchEvent(new Event('messageerror'));
        worker.dispatchEvent(new MessageEvent('message', { data: { protocol: 'cem-rendering-retirement-v2', direction: 'response', action: 'inspect', ok: false } }));
        for (let n = 0; n < 20; n++) worker.postMessage({ protocol: 'cem-rendering-retirement-v2', direction: 'request', action: 'inspect' });
        expect(worker.calls).toHaveLength(20);
    }
    const snapshot = diagnostics.snapshot();
    expect(snapshot.workers).toHaveLength(16); expect(snapshot.dropped).toBeGreaterThan(0);
    expect(snapshot.workers.every(worker => worker.events.length <= 16)).toBe(true);
    expect(snapshot.workers[0].events.slice(1, 4).map(event => event.phase)).toEqual(['worker-error', 'message-error', 'headroom-refused']);
    for (let i = 0; i < 20; i++) diagnostics.phase('allocation-start');
    expect(diagnostics.snapshot().allocations).toHaveLength(16);
});
