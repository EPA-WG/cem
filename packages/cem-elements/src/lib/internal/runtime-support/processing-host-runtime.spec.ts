import { describe, expect, it, vi } from 'vitest';

vi.mock('./cem-ql-render.js', () => ({
    compileCemMlTemplate: vi.fn(async () => []),
}));

import { createCemDeclarationScope } from '../../declaration-scope.js';
import { cemProcessingHostForScope } from './processing-host-runtime.js';
import {
    createCemProcessingReadyEnvelope,
    createCemProcessingSuccessEnvelope,
    createCemProcessingTextSource,
    type CemProcessingRequestEnvelope,
    type CemProcessingResponseEnvelope,
    type CemProcessingWorkerFactory,
} from './processing-host.js';
import type { CemProcessingSchedulingTraceEvent } from './processing-scheduler.js';

describe('Phase 3B processing-host runtime', () => {
    it('shares a bounded worker slot across roots with FIFO-per-root fair dispatch and deterministic traces', async () => {
        const worker = new ControlledProcessingWorker();
        const workerFactory: CemProcessingWorkerFactory = vi.fn(() => worker as unknown as Worker);
        const document = {} as Document;
        const rootOne = createCemDeclarationScope({ document });
        const rootTwo = createCemDeclarationScope({ document });
        const trace: CemProcessingSchedulingTraceEvent[] = [];
        const options = {
            workerScriptUrl: new URL('https://example.test/cem-processing-worker.js'),
            workerFactory,
            poolPolicy: { workerCount: 1, maxWorkers: 1, queueSize: 8 },
            onTrace: (event: CemProcessingSchedulingTraceEvent) => trace.push(event),
        };
        const hostOne = cemProcessingHostForScope(rootOne, options);
        const hostTwo = cemProcessingHostForScope(rootTwo, options);

        const jobs = [
            hostOne.compile(compileInput('one-a')),
            hostOne.compile(compileInput('one-b')),
            hostTwo.compile(compileInput('two-a')),
            hostTwo.compile(compileInput('two-b')),
        ];
        await flushMicrotasks();

        expect(workerFactory).toHaveBeenCalledTimes(1);
        expect(worker.requests.map((request) => request.jobId)).toEqual([1]);
        worker.respondNext();
        await flushMicrotasks();
        expect(worker.requests.map((request) => request.jobId)).toEqual([1, 3]);
        worker.respondNext();
        await flushMicrotasks();
        expect(worker.requests.map((request) => request.jobId)).toEqual([1, 3, 2]);
        worker.respondNext();
        await flushMicrotasks();
        expect(worker.requests.map((request) => request.jobId)).toEqual([1, 3, 2, 4]);
        worker.respondNext();
        await Promise.all(jobs.map((job) => job.result));

        expect(trace.filter((event) => event.kind === 'dispatch').map((event) => event.jobId)).toEqual([
            1, 3, 2, 4,
        ]);
        expect(trace.map((event) => event.sequence)).toEqual(
            trace.map((_event, index) => index + 1)
        );
        expect(trace.every((event) => event.workerSlot === 1)).toBe(true);
        expect(structuredClone(trace)).toEqual(trace);

        rootOne.dispose();
        rootTwo.dispose();
        await flushMicrotasks();
        expect(worker.terminated).toBe(true);
    });

    it('preserves an accepted cancellation when the target was removed before worker dispatch', async () => {
        const worker = new ControlledProcessingWorker();
        const document = {} as Document;
        const root = createCemDeclarationScope({ document });
        const host = cemProcessingHostForScope(root, {
            workerScriptUrl: new URL('https://example.test/cem-processing-worker.js'),
            workerFactory: () => worker as unknown as Worker,
            poolPolicy: { workerCount: 1, maxWorkers: 1, queueSize: 8 },
        });
        const active = host.compile(compileInput('active'));
        const queued = host.compile(compileInput('queued'));
        await flushMicrotasks();

        expect(worker.requests.map((request) => request.jobId)).toEqual([active.jobId]);
        const cancellation = host.cancel({ targetJobId: queued.jobId });
        await flushMicrotasks();
        expect(worker.requests.map((request) => request.operation)).toEqual(['compile', 'cancel']);

        worker.respondCancel(false);
        await expect(cancellation.result).resolves.toEqual({
            targetJobId: queued.jobId,
            accepted: true,
        });
        await expect(queued.result).rejects.toThrow(`processing job ${queued.jobId} was cancelled`);
        worker.respondNext();
        await active.result;
        root.dispose();
    });

    it('moves every same-root queued job to fallback after worker execution failure', async () => {
        const worker = new ThrowingProcessingWorker();
        const document = {} as Document;
        const root = createCemDeclarationScope({ document });
        const host = cemProcessingHostForScope(root, {
            workerScriptUrl: new URL('https://example.test/cem-processing-worker.js'),
            workerFactory: () => worker as unknown as Worker,
            poolPolicy: { workerCount: 1, maxWorkers: 1, queueSize: 8 },
        });
        const jobs = [
            host.compile(compileInput('fallback-a')),
            host.compile(compileInput('fallback-b')),
            host.compile(compileInput('fallback-c')),
        ];

        const results = await Promise.all(jobs.map((job) => job.result));
        expect(results.map((result) => result.artifact.artifactId)).toEqual([
            'fallback-a',
            'fallback-b',
            'fallback-c',
        ]);
        expect(host.mode).toBe('main-thread');
        expect(worker.terminated).toBe(true);
        root.dispose();
    });
});

class ControlledProcessingWorker {
    readonly requests: CemProcessingRequestEnvelope[] = [];
    terminated = false;

    private readonly listeners = new Map<string, Set<(event: MessageEvent<unknown>) => void>>();
    private readonly pending: CemProcessingRequestEnvelope[] = [];

    constructor() {
        queueMicrotask(() => this.emit('message', createCemProcessingReadyEnvelope('worker')));
    }

    addEventListener(type: string, listener: (event: MessageEvent<unknown>) => void): void {
        let listeners = this.listeners.get(type);
        if (!listeners) {
            listeners = new Set();
            this.listeners.set(type, listeners);
        }
        listeners.add(listener);
    }

    removeEventListener(type: string, listener: (event: MessageEvent<unknown>) => void): void {
        this.listeners.get(type)?.delete(listener);
    }

    postMessage(request: CemProcessingRequestEnvelope): void {
        this.requests.push(request);
        this.pending.push(request);
    }

    respondNext(): void {
        const index = this.pending.findIndex((request) => request.operation === 'compile');
        const request = index >= 0 ? this.pending.splice(index, 1)[0] : undefined;
        if (!request || request.operation !== 'compile') {
            throw new Error('expected one pending compile request');
        }
        const response = createCemProcessingSuccessEnvelope(request, {
            artifact: {
                kind: 'template-artifact-handle',
                artifactId: request.payload.templateArtifactId,
                cacheKey: `cache:${request.payload.templateArtifactId}`,
                registrationIdentity: request.payload.registrationIdentity,
                scopePolicyStamp: request.payload.scopePolicyStamp,
                sourceMapMode: request.payload.sourceMapMode,
            },
            declaredAttributes: [],
            observedAttributes: [],
            invalidationScopes: [],
            diagnostics: [],
        });
        queueMicrotask(() => this.emit('message', response));
    }

    respondCancel(accepted: boolean): void {
        const index = this.pending.findIndex((request) => request.operation === 'cancel');
        const request = index >= 0 ? this.pending.splice(index, 1)[0] : undefined;
        if (!request || request.operation !== 'cancel') {
            throw new Error('expected one pending cancel request');
        }
        queueMicrotask(() => this.emit('message', createCemProcessingSuccessEnvelope(request, {
            targetJobId: request.payload.targetJobId,
            accepted,
        })));
    }

    terminate(): void {
        this.terminated = true;
    }

    private emit(type: string, data: CemProcessingResponseEnvelope | ReturnType<typeof createCemProcessingReadyEnvelope>): void {
        for (const listener of this.listeners.get(type) ?? []) {
            listener({ data } as MessageEvent<unknown>);
        }
    }
}

class ThrowingProcessingWorker extends ControlledProcessingWorker {
    override postMessage(_request: CemProcessingRequestEnvelope): void {
        throw new Error('fixture worker execution failed');
    }
}

function compileInput(templateArtifactId: string) {
    return {
        language: 'cem-ml' as const,
        producedTag: `cem-${templateArtifactId}`,
        templateArtifactId,
        registrationIdentity: `registration:${templateArtifactId}`,
        source: createCemProcessingTextSource(`{span | ${templateArtifactId}}`),
        sourceRef: { kind: 'inline' as const, value: templateArtifactId },
        resolverIdentity: 'document:https://example.test/',
        scopePolicyStamp: 'scope-policy-v1',
        sourceMapMode: 'dev' as const,
    };
}

async function flushMicrotasks(): Promise<void> {
    await Promise.resolve();
    await Promise.resolve();
    await Promise.resolve();
}
