import { describe, expect, it, vi } from 'vitest';

vi.mock('./cem-ql-render.js', () => ({
    cemMlTemplateArtifactPayloadKey: vi.fn(async (_source: string, sourceMapMode: 'dev' | 'prod') => ({
        contentType: 'cem-template-artifact',
        sourceHash: 'cem-bin/1+blake3:fixture-source',
        cemMlVersion: '0.1.0',
        cemQlVersion: '0.1.0',
        sourceMapMode,
    })),
    compileCemMlTemplateArtifact: vi.fn(async (source: string) => new TextEncoder().encode(source)),
    retainCemMlTemplateSource: vi.fn(async () => ({ artifactId: 1, diagnostics: [] })),
    retainCemMlTemplateArtifact: vi.fn(async () => ({
        artifactId: 1,
        contentHash: 'cem-bin/1+blake3:fixture',
        formatVersion: 'cem-template-artifact/1',
        diagnostics: [],
    })),
    processNativeCemValue: vi.fn(async () => ({ text: 'null' })),
    retainLoadedCemDocument: vi.fn(async () => 201),
    disposeLoadedCemDocument: vi.fn(),
    disposeRetainedCemMlTemplate: vi.fn(() => true),
    processRetainedCemMlTemplate: vi.fn(async () => ({
        diagnostics: [],
        renderPlan: {
            producedTag: 'cem-fallback',
            instanceId: 'fixture',
            templateArtifactId: 'fixture',
            dataRevision: '1',
            outputTarget: 'light-dom',
            scopePolicyStamp: 'scope-policy-v1',
            nodes: [],
        },
    })),
}));

import { processNativeCemValue, retainLoadedCemDocument, disposeLoadedCemDocument, processRetainedCemMlTemplate } from './cem-ql-render.js';
import type { DataIslandSnapshot } from '../../cem-elements.js';
import { createCemDeclarationScope } from '../../declaration-scope.js';
import { CemProcessingEngine } from './processing-engine.js';
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

    respondDocument(): void {
        const index = this.pending.findIndex((request) => request.operation === 'document');
        const request = index >= 0 ? this.pending.splice(index, 1)[0] : undefined;
        if (!request || request.operation !== 'document') throw new Error('expected document request');
        queueMicrotask(() => this.emit('message', createCemProcessingSuccessEnvelope(request, {
            handle: request.payload.handle, retained: request.payload.action === 'retain',
        })));
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

    protected emit(type: string, data: CemProcessingResponseEnvelope | ReturnType<typeof createCemProcessingReadyEnvelope>): void {
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


class DocumentFallbackWorker extends ControlledProcessingWorker {
    private readonly engine = new CemProcessingEngine();
    override terminate(): void { this.engine.dispose({}); super.terminate(); }
    override postMessage(request: CemProcessingRequestEnvelope): void {
        if (request.operation === 'render-diff') throw new Error('worker failed after document import');
        if (request.operation === 'compile') {
            this.requests.push(request);
            void this.engine.compile(request.payload).then((result) =>
                this.emit('message', createCemProcessingSuccessEnvelope(request, result)));
            return;
        }
        super.postMessage(request);
        if (request.operation === 'document') this.respondDocument();
    }
}

it('replays retained response bytes through native import after worker loss and releases them', async () => {
    const worker = new DocumentFallbackWorker();
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, {
        workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => worker as unknown as Worker,
    });
    const { artifact } = await host.compile(compileInput('fixture')).result;
    const handle = { documentKey: 'retained-response', instanceId: 'fixture', scopePolicyStamp: 'scope-policy-v1' };
    const bytes = new TextEncoder().encode('{"qty":3}').buffer;
    const nativeAttributes = [{ name: 'label', value: { kind: 'cem-native-value-v1' as const,
        artifact: new Uint8Array([67, 69, 77, 86, 1]).buffer, contentHash: 'opaque-fixture', index: 0 } }];
    await host.document({ action: 'retain', handle, bytes, contentType: 'application/json', sourceUri: 'https://example.test/data' }).result;
    vi.mocked(retainLoadedCemDocument).mockClear();
    const revision = { instanceId: 'fixture', dataRevision: '1', templateArtifactId: 'fixture',
        scopePolicyStamp: 'scope-policy-v1', outputTarget: 'light-dom' as const };
    await host.renderDiff({ artifact, revision, data: {}, snapshot: { ...revision } as DataIslandSnapshot,
        nativeAttributes,
        nativeSlices: [{ ...nativeAttributes[0], name: 'stored', attribute: 'slice-value' }],
        documents: [{ slice: 'response', handle }], scopeUid: 'scope-one' }).result;
    expect(host.mode).toBe('main-thread');
    expect(retainLoadedCemDocument).toHaveBeenCalledWith(bytes, 'application/json', 'https://example.test/data');
    expect(processRetainedCemMlTemplate).toHaveBeenLastCalledWith(expect.any(Number), expect.objectContaining({
        documents: [{ slice: 'response', documentId: 201 }],
        nativeAttributes,
        nativeSlices: [{ ...nativeAttributes[0], name: 'stored', attribute: 'slice-value' }],
    }));
    await host.document({ action: 'release', handle }).result;
    expect(disposeLoadedCemDocument).toHaveBeenCalledWith(201);
    await expect(host.renderDiff({ artifact, revision, data: {}, snapshot: { ...revision } as DataIslandSnapshot,
        documents: [{ slice: 'response', handle }], scopeUid: 'scope-one' }).result).rejects.toThrow('missing the retained document bytes');
    await host.dispose({ reason: 'scope-disposed' }).result;
});

it('releases a native import cancelled while the main-thread fallback is awaiting it', async () => {
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, {
        workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => { throw new Error('worker unavailable'); },
    });
    let finish!: (id: number) => void;
    vi.mocked(retainLoadedCemDocument).mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; }));
    const handle = { documentKey: 'cancelled-import', instanceId: 'fixture', scopePolicyStamp: 'scope-policy-v1' };
    const job = host.document({ action: 'retain', handle, bytes: new TextEncoder().encode('<p/>').buffer,
        contentType: 'application/xml', sourceUri: 'https://example.test/data' });
    const rejected = expect(job.result).rejects.toThrow('cancelled');
    await vi.waitFor(() => expect(finish).toBeTypeOf('function'));
    await host.cancel({ targetJobId: job.jobId }).result;
    finish(207);
    await rejected;
    expect(disposeLoadedCemDocument).toHaveBeenCalledWith(207);
    await host.dispose({}).result;
});

it('releases only the disposed root documents when roots share one worker', async () => {
    const worker = new DocumentFallbackWorker();
    const document = {} as Document;
    const firstRoot = createCemDeclarationScope({ document });
    const secondRoot = createCemDeclarationScope({ document });
    const options = { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => worker as unknown as Worker, poolPolicy: { workerCount: 1, maxWorkers: 1 } };
    const first = cemProcessingHostForScope(firstRoot, options);
    const second = cemProcessingHostForScope(secondRoot, options);
    const firstHandle = { documentKey: 'first-root', instanceId: 'one', scopePolicyStamp: 'scope-policy-v1' };
    const secondHandle = { ...firstHandle, documentKey: 'second-root' };
    const source = { bytes: new TextEncoder().encode('<p/>').buffer,
        contentType: 'application/xml', sourceUri: 'https://example.test/data' };
    await first.document({ action: 'retain', handle: firstHandle, ...source }).result;
    await second.document({ action: 'retain', handle: secondHandle, ...source }).result;
    await first.dispose({ reason: 'scope-disposed' }).result;
    expect(worker.terminated).toBe(false);
    expect(worker.requests.filter((request) => request.operation === 'document' && request.payload.action === 'release')
        .map((request) => request.payload)).toEqual([{ action: 'release', handle: firstHandle }]);
    await second.dispose({ reason: 'scope-disposed' }).result;
    expect(worker.terminated).toBe(true);
});

it('replays stateless native value I/O on worker loss and suppresses cancelled results', async () => {
    const root = createCemDeclarationScope({ document: {} as Document });
    const worker = new ThrowingProcessingWorker();
    const host = cemProcessingHostForScope(root, { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => worker as unknown as Worker });
    const input = { action: 'export-json' as const, scopePolicyStamp: 'storage',
        limits: { maxBytes: 1000, maxValues: 100, maxDepth: 10 }, attribute: 'slice-value',
        value: { kind: 'cem-native-value-v1' as const, artifact: new Uint8Array([67, 69, 77]).buffer, contentHash: 'opaque', index: 0 } };
    expect(await host.value(input).result).toEqual({ text: 'null' });
    expect(host.mode).toBe('main-thread');
    expect(processNativeCemValue).toHaveBeenLastCalledWith(input);
    let finish!: (value: { text: string }) => void;
    vi.mocked(processNativeCemValue).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
    const job = host.value(input);
    const rejected = expect(job.result).rejects.toThrow('cancelled');
    await vi.waitFor(() => expect(finish).toBeTypeOf('function'));
    await host.cancel({ targetJobId: job.jobId, reason: 'superseded' }).result;
    finish({ text: 'old' });
    await rejected;
    root.dispose();
    await expect(host.value(input).result).rejects.toThrow();
});
