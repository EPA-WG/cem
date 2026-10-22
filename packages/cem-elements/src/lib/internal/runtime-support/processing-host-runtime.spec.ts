import { nativeCheckpointSnapshot, nativeCheckpointIdentity, nativeCheckpointSourceProof, nativeCheckpointSubscribe } from './checkpoint-domain.js';
import { closeNativeRenderingGeneration, retireNativeRenderingGeneration, nativeCheckpointDrain, nativeCheckpointDrainCurrent } from './checkpoint-domain.js';
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
    prepareRetainedNativeSession: vi.fn(async () => ({ id: 701, length: 2 })),
    releaseRetainedNativeSession: vi.fn(),
    exportRetainedNativeSessionView: vi.fn((_id, input) => ({ status: 'view', handle: input.handle, values: [] })),
    renderRetainedNativeSessionLabel: vi.fn(async (_id, input) => ({ status: 'rendered', handle: input.handle, nodes: [], diagnostics: [] })),
    retainLoadedCemDocument: vi.fn(async () => 201),
    processRetainedTemplateStylesheet: vi.fn(async (_id, input) => input.action === 'release'
        ? { status: 'released', count: 1 }
        : { status: 'pending', loadId: 1, request: { id: 1, url: 'https://example.test/child.css', contentType: null, integrity: null } }),
    releaseRetainedTemplateStylesheets: vi.fn(() => ({ status: 'released', count: 1 })),
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

import { processRetainedTemplateStylesheet, processNativeCemValue, retainLoadedCemDocument, disposeLoadedCemDocument, processRetainedCemMlTemplate } from './cem-ql-render.js';
import type { DataIslandSnapshot } from '../../cem-elements.js';
import { createCemDeclarationScope } from '../../declaration-scope.js';
import { CemProcessingEngine } from './processing-engine.js';
import { cemProcessingHostForScope } from './processing-host-runtime.js';
import {
    CEM_RENDERING_RETIREMENT_PROTOCOL, isCemRenderingRetirementControl, type CemRenderingRetirementControl,
    CemProcessingDiagnosticError,
    cemProcessingFailureDiagnostics,
    createCemProcessingFailureEnvelope,
    createCemProcessingReadyEnvelope,
    createCemProcessingPressureEnvelope,
    createCemProcessingSuccessEnvelope,
    createCemProcessingTextSource,
    type CemProcessingRequestEnvelope,
    type CemProcessingResponseEnvelope,
    type CemProcessingWorkerFactory,
} from './processing-host.js';
import type { CemProcessingSchedulingTraceEvent } from './processing-scheduler.js';

describe('Phase 3B processing-host runtime', () => {
    it('keeps stylesheet I/O in a pressure drain and permits explicit release after closure', async () => {
        class StylesheetWorker extends ControlledProcessingWorker {
            override postMessage(request: CemProcessingRequestEnvelope): void {
                if (request.operation === 'stylesheet') {
                    this.requests.push(request);
                    const response = request.payload.action === 'begin'
                        ? { status: 'pending' as const, loadId: 'load', request: { id: 1, url: 'https://example.test/pending.css', contentType: null, integrity: null } }
                        : { status: 'released' as const, count: 1 };
                    queueMicrotask(() => this.emit('message', createCemProcessingSuccessEnvelope(request, response)));
                } else super.postMessage(request);
            }
        }
        const worker = new StylesheetWorker(), scope = createCemDeclarationScope({ document: {} as Document });
        const host = cemProcessingHostForScope(scope, { workerScriptUrl: new URL('https://example.test/pressure.js'),
            workerFactory: () => worker as unknown as Worker });
        const job = host.compile(compileInput('stylesheet')); await flushMicrotasks(); worker.respondNext(); const { artifact } = await job.result;
        // Begin performs the normal preflight, which the controlled worker holds.
        const load = host.stylesheet({ ...stylesheetBegin, artifact, consumer: 'menu' });
        await flushMicrotasks(); worker.respondNext(); await load.result;
        worker.pressure(); let settled = false;
        const drained = nativeCheckpointDrain(host.checkpointClient!).then(proof => { settled = true; return proof; });
        await flushMicrotasks(); expect(settled).toBe(false);
        await host.stylesheet({ action: 'release', artifact, consumer: 'menu' }).result;
        expect(nativeCheckpointDrainCurrent(await drained)).toBe(true);
        await host.dispose({ reason: 'runtime-disposed' }).result; scope.dispose();
    });
    it('pressure-closes every pooled root without fallback or replay and drains queued jobs', async () => {
        const worker = new ControlledProcessingWorker(), document = {} as Document;
        const a = createCemDeclarationScope({ document }), b = createCemDeclarationScope({ document });
        const options = { workerScriptUrl: new URL('https://example.test/pressure.js'),
            workerFactory: () => worker as unknown as Worker, poolPolicy: { workerCount: 1, maxWorkers: 1, queueSize: 8 } };
        const first = cemProcessingHostForScope(a, options), second = cemProcessingHostForScope(b, options);
        const running = first.compile(compileInput('active')), queued = second.compile(compileInput('queued'));
        await flushMicrotasks(); worker.pressure();
        const drained = nativeCheckpointDrain(first.checkpointClient!);
        await expect(second.compile(compileInput('refused')).result).rejects.toThrow('cem.runtime.memory.closed');
        expect(worker.requests).toHaveLength(1);
        worker.respondNext(); await running.result; await flushMicrotasks(); worker.respondNext(); await queued.result;
        const proof = await drained; expect(nativeCheckpointDrainCurrent(proof)).toBe(true);
        expect(first.mode).toBe('worker'); expect(second.mode).toBe('worker'); expect(worker.terminated).toBe(false);
        await first.dispose({ reason: 'runtime-disposed' }).result;
        expect(nativeCheckpointDrainCurrent(proof)).toBe(false);
        await second.dispose({ reason: 'runtime-disposed' }).result; a.dispose(); b.dispose();
    });
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

    fail(): void {
        for (const listener of this.listeners.get('error') ?? []) listener({
            preventDefault() { return undefined; }, error: new Error('lost fresh worker'),
        } as unknown as MessageEvent<unknown>);
    }

    pressure(): void {
        for (const listener of this.listeners.get('message') ?? []) {
            listener({ data: createCemProcessingPressureEnvelope('soft-pressure') } as MessageEvent<unknown>);
        }
    }

    protected emit(type: string, data: CemProcessingResponseEnvelope | ReturnType<typeof createCemProcessingReadyEnvelope> | CemRenderingRetirementControl): void {
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


class StylesheetProcessingWorker extends ControlledProcessingWorker {
    private readonly engine: CemProcessingEngine;
    constructor(maxArtifactEntries?: number) {
        super();
        this.engine = new CemProcessingEngine({ maxArtifactEntries });
    }
    failDelivery = false;
    override terminate(): void { this.engine.dispose({ reason: 'runtime-disposed' }); super.terminate(); }
    override postMessage(request: CemProcessingRequestEnvelope): void {
        if (request.operation === 'stylesheet' && request.payload.action === 'deliver' && this.failDelivery) {
            throw new Error('worker failed during CSS delivery');
        }
        if (request.operation === 'compile' || request.operation === 'stylesheet') {
            this.requests.push(request);
            const result = request.operation === 'compile' ? this.engine.compile(request.payload) : this.engine.stylesheet(request.payload);
            void result.then((value) => this.emit('message', createCemProcessingSuccessEnvelope(request, value)),
                (error: unknown) => this.emit('message', createCemProcessingFailureEnvelope(request, 'failure', cemProcessingFailureDiagnostics(error))));
            return;
        }
        super.postMessage(request);
    }
}

const stylesheetBegin = { action: 'begin' as const, consumer: 'card', index: 0,
    baseUrl: 'https://example.test/main.css', scope: { kind: 'private' as const, tag: 'cem-card' },
    context: { identity: 'page', resolverIdentity: 'test', resourcePolicyStamp: 'test', frames: [] } };

it('routes stylesheet bytes and rejects worker handles after fallback, including after a new begin', async () => {
    const worker = new StylesheetProcessingWorker();
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => worker as unknown as Worker });
    const { artifact } = await host.compile(compileInput('css')).result;
    const pending = await host.stylesheet({ ...stylesheetBegin, artifact }).result;
    if (pending.status !== 'pending') throw new Error('expected pending');
    const delivery = { action: 'deliver' as const, artifact, consumer: 'card', loadId: pending.loadId,
        requestId: pending.request.id, bytes: new TextEncoder().encode('.card{}').buffer,
        finalUrl: 'https://example.test/child.css', contentType: 'text/css' };
    await host.stylesheet(delivery).result;
    expect(worker.requests.at(-1)?.payload).toEqual({ ...delivery, consumer: expect.stringContaining('card') });
    worker.failDelivery = true;
    await expect(host.stylesheet(delivery).result).rejects.toThrow('restart the load');
    expect(host.mode).toBe('main-thread');
    const restarted = await host.stylesheet({ ...stylesheetBegin, artifact }).result;
    expect(restarted).toHaveProperty('loadId');
    await expect(host.stylesheet(delivery).result).rejects.toThrow('owner changed');
    await host.dispose({ reason: 'runtime-disposed' }).result;
});

it('releases the exact stylesheet generation cancelled during fallback processing', async () => {
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => { throw new Error('unavailable'); } });
    const { artifact } = await host.compile(compileInput('css')).result;
    let finish!: (value: { status: 'pending'; loadId: number; request: { id: number; url: string; contentType: null; integrity: null } }) => void;
    vi.mocked(processRetainedTemplateStylesheet).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
    const job = host.stylesheet({ ...stylesheetBegin, artifact });
    const rejected = expect(job.result).rejects.toThrow('cancelled');
    await vi.waitFor(() => expect(finish).toBeTypeOf('function'));
    await host.cancel({ targetJobId: job.jobId, reason: 'superseded' }).result;
    finish({ status: 'pending', loadId: 17, request: { id: 1, url: 'https://example.test/child.css', contentType: null, integrity: null } });
    await rejected;
    expect(processRetainedTemplateStylesheet).toHaveBeenLastCalledWith(1,
        expect.objectContaining({ action: 'release', loadId: expect.stringMatching(/:17$/) }),
        expect.any(String), 'registration:css', 17);
    await host.dispose({ reason: 'runtime-disposed' }).result;
});

it('releases stylesheet consumers for a disposed root while keeping the pooled worker alive', async () => {
    const worker = new StylesheetProcessingWorker();
    const document = {} as Document;
    const roots = [createCemDeclarationScope({ document }), createCemDeclarationScope({ document })];
    const options = { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => worker as unknown as Worker, poolPolicy: { workerCount: 1, maxWorkers: 1 } };
    const hosts = roots.map(root => cemProcessingHostForScope(root, options));
    const { artifact } = await hosts[0].compile(compileInput('css')).result;
    await hosts[0].stylesheet({ ...stylesheetBegin, artifact }).result;
    const other = await hosts[1].compile(compileInput('css')).result;
    await hosts[1].stylesheet({ ...stylesheetBegin, artifact: other.artifact }).result;
    const consumers = worker.requests.filter(request => request.operation === 'stylesheet').map(request => request.payload.consumer);
    expect(new Set(consumers).size).toBe(2);
    await hosts[0].dispose({ reason: 'scope-disposed' }).result;
    expect(worker.terminated).toBe(false);
    expect(worker.requests.at(-1)).toMatchObject({ operation: 'stylesheet', payload: { action: 'release', artifact, consumer: expect.stringContaining('card') } });
    await hosts[1].dispose({ reason: 'scope-disposed' }).result;
});


it('releases a late successful worker stylesheet response after host cancellation', async () => {
    const worker = new StylesheetProcessingWorker();
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => worker as unknown as Worker });
    const { artifact } = await host.compile(compileInput('css')).result;
    let finish!: (value: { status: 'pending'; loadId: number; request: { id: number; url: string; contentType: null; integrity: null } }) => void;
    vi.mocked(processRetainedTemplateStylesheet).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
    const job = host.stylesheet({ ...stylesheetBegin, artifact });
    const rejected = expect(job.result).rejects.toThrow('cancelled');
    await vi.waitFor(() => expect(finish).toBeTypeOf('function'));
    const cancellation = host.cancel({ targetJobId: job.jobId, reason: 'superseded' });
    await vi.waitFor(() => expect(worker.requests.at(-1)?.operation).toBe('cancel'));
    worker.respondCancel(false);
    await cancellation.result;
    finish({ status: 'pending', loadId: 23, request: { id: 1, url: 'https://example.test/child.css', contentType: null, integrity: null } });
    await rejected;
    expect(worker.requests.at(-1)).toMatchObject({ operation: 'stylesheet',
        payload: { action: 'release', loadId: expect.stringMatching(/:23$/) } });
    await host.dispose({ reason: 'runtime-disposed' }).result;
});


it.each(['worker', 'fallback'])('preserves imported stylesheet diagnostic locations through %s transport', async (mode) => {
    const worker = new StylesheetProcessingWorker();
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => { if (mode === 'fallback') throw new Error('unavailable'); return worker as unknown as Worker; } });
    const { artifact } = await host.compile(compileInput('css')).result;
    const diagnostic = { code: 'cem.scoped_css.id_selector_unsupported', severity: 'warning' as const,
        message: 'ID selectors are suppressed', sourceUri: 'https://example.test/cdn/child.css',
        stylesheetUrl: 'https://example.test/cdn/child.css', sheet: 1, line: 2, column: 1, offset: 16, length: 8 };
    vi.mocked(processRetainedTemplateStylesheet).mockResolvedValueOnce({ status: 'ready', loadId: 7,
        css: '.card {color:green}', identity: { ownerKey: 'owner', cacheKey: 'cache', contextMarker: null },
        diagnostics: [diagnostic] });
    const result = await host.stylesheet({ ...stylesheetBegin, artifact }).result;
    expect(result.status).toBe('ready');
    if (result.status !== 'ready') throw new Error('expected ready stylesheet');
    expect(result.diagnostics).toContainEqual(diagnostic);
    expect(structuredClone(result).diagnostics).toContainEqual(diagnostic);
    await host.dispose({ reason: 'runtime-disposed' }).result;
});


it.each(['worker', 'fallback'])('preserves structured stylesheet failures through %s transport', async (mode) => {
    const worker = new StylesheetProcessingWorker();
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => { if (mode === 'fallback') throw new Error('unavailable'); return worker as unknown as Worker; } });
    const { artifact } = await host.compile(compileInput('css')).result;
    const diagnostic = { code: 'cem.css.import_content_type', severity: 'error' as const,
        message: 'CSS imports require text/css', sourceUri: 'https://example.test/cdn/child.css',
        stylesheetUrl: 'https://example.test/cdn/child.css', line: 2, column: 1, offset: 9, length: 16 };
    vi.mocked(processRetainedTemplateStylesheet).mockRejectedValueOnce(new CemProcessingDiagnosticError([diagnostic]));
    await expect(host.stylesheet({ ...stylesheetBegin, artifact }).result).rejects.toMatchObject({ diagnostics: [diagnostic] });
    expect(host.mode).toBe(mode === 'worker' ? 'worker' : 'main-thread');
    expect(cemProcessingFailureDiagnostics(new Error('stale handle'))).toEqual([
        { code: 'cem.processing_host.execution_failed', severity: 'error', message: 'stale handle' },
    ]);
    await host.dispose({ reason: 'runtime-disposed' }).result;
});

it('releases stylesheet generations outside a saturated worker work queue', async () => {
    class CleanupWorker extends ControlledProcessingWorker {
        override postMessage(request: CemProcessingRequestEnvelope): void {
            if (request.operation === 'stylesheet' && request.payload.action === 'release') {
                this.requests.push(request);
                queueMicrotask(() => this.emit('message', createCemProcessingSuccessEnvelope(request, { status: 'released', count: 1 })));
            } else super.postMessage(request);
        }
    }
    const worker = new CleanupWorker();
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, {
        workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => worker as unknown as Worker,
        poolPolicy: { workerCount: 1, maxWorkers: 1, queueSize: 1 },
    });
    const initial = host.compile(compileInput('owner')); await flushMicrotasks();
    worker.respondNext(); const { artifact } = await initial.result;
    const active = host.compile(compileInput('active')); await flushMicrotasks();
    const queued = host.compile(compileInput('queued')); await flushMicrotasks();
    try {
        const releases = Array.from({ length: 80 }, (_, index) => host.stylesheet({
            action: 'release', artifact, consumer: `consumer-${index}`, loadId: `load-${index}`,
        }).result);
        await expect(Promise.all(releases)).resolves.toHaveLength(80);
        expect(worker.requests.filter(request => request.operation === 'stylesheet')).toHaveLength(80);
    } finally {
        worker.respondNext(); await active.result; await flushMicrotasks();
        worker.respondNext(); await queued.result;
        root.dispose();
    }
});

it('cancels a native session after fallback import and releases its owner before retry', async () => {
    const native = await import('./cem-ql-render.js');
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => { throw new Error('unavailable'); } });
    const handle = { sessionKey: 'cancel-native', instanceId: 'field', scopePolicyStamp: 'scope', sourceRevision: 'one' };
    let finish!: (value: { id: number; length: number }) => void;
    vi.mocked(native.prepareRetainedNativeSession).mockImplementationOnce(() => new Promise(resolve => { finish = resolve; }));
    const request = { action: 'prepare' as const, handle, sources: { kind: 'cem-native-session-sources-v1' as const,
        requesting: 0, sources: [], bindings: [], grants: [] }, data: {}, select: '()', limits: { maxBytes: 1024, maxValues: 100, maxDepth: 20 } };
    const job = host.nativeSession(request);
    const rejected = expect(job.result).rejects.toThrow('cancelled');
    await vi.waitFor(() => expect(finish).toBeTypeOf('function'));
    await host.cancel({ targetJobId: job.jobId, reason: 'superseded' }).result;
    finish({ id: 719, length: 1 });
    await rejected;
    expect(native.releaseRetainedNativeSession).toHaveBeenCalledWith(719);
    await expect(host.nativeSession({ action: 'view', handle, expression: 'input' }).result).rejects.toThrow();
    await host.nativeSession({ ...request, handle: { ...handle, sessionKey: 'fresh' } }).result;
    await host.dispose({ reason: 'runtime-disposed' }).result;
    expect(native.releaseRetainedNativeSession).toHaveBeenCalledWith(701);
});


it('reacquires stylesheet source after bounded worker artifact eviction', async () => {
    const worker = new StylesheetProcessingWorker(1);
    const root = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(root, { workerScriptUrl: new URL('https://example.test/worker.js'),
        workerFactory: () => worker as unknown as Worker });
    try {
        const first = await host.compile(compileInput('evicted-css')).result;
        await host.compile(compileInput('replacement-css')).result;
        const result = await host.stylesheet({ ...stylesheetBegin, artifact: first.artifact }).result;
        expect(result).toHaveProperty('loadId');
        expect(host.mode).toBe('worker');
        expect(worker.requests.slice(-2).map(request => request.operation)).toEqual(['compile', 'stylesheet']);
        await host.stylesheet({ action: 'release', artifact: first.artifact, consumer: stylesheetBegin.consumer }).result;
    } finally { await host.dispose({ reason: 'runtime-disposed' }).result; }
});


it('accounts for actual pooled clients and host-only queued work before checkpoint coverage', async () => {
    let worker = new ControlledProcessingWorker();
    const document = {} as Document;
    const options = { workerScriptUrl: new URL('https://example.test/worker.js'), workerFactory: () => worker as unknown as Worker,
        poolPolicy: { workerCount: 1, maxWorkers: 1, queueSize: 8 } };
    const root = createCemDeclarationScope({ document }), peerRoot = createCemDeclarationScope({ document });
    const host = cemProcessingHostForScope(root, options), peer = cemProcessingHostForScope(peerRoot, options);
    const bindings = new Map([[host.checkpointClient!, ['editor']]]);
    expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    bindings.set(peer.checkpointClient!, ['peer']);
    const initial = nativeCheckpointSnapshot(bindings)!;
    expect(initial).toHaveLength(1); expect(initial[0].clients).toHaveLength(2);
    const job = peer.compile(compileInput('host-only'));
    expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    await flushMicrotasks(); worker.respondNext(); await job.result;
    expect(nativeCheckpointSnapshot(bindings)![0].revision).toBeGreaterThan(initial[0].revision);
    const release = host.checkpointInstance!('not-enrolled');
    expect(nativeCheckpointSnapshot(bindings)).toBeUndefined(); release();
    expect(nativeCheckpointSnapshot(bindings)).toBeDefined();
    const generation = nativeCheckpointIdentity(host.checkpointClient!);
    await peer.dispose({}).result; bindings.delete(peer.checkpointClient!);
    expect(nativeCheckpointSnapshot(bindings)![0].clients).toHaveLength(1);
    await host.dispose({}).result; expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    worker = new ControlledProcessingWorker();
    const replacement = cemProcessingHostForScope(createCemDeclarationScope({ document }), options);
    expect(nativeCheckpointIdentity(replacement.checkpointClient!)).not.toBe(generation);
    await replacement.dispose({}).result;
});


it('requires exact retained-document capsules even for a published instance and revokes uncertain cleanup', async () => {
    class FailedReleaseWorker extends ControlledProcessingWorker {
        override postMessage(request: CemProcessingRequestEnvelope): void {
            super.postMessage(request);
            if (request.operation === 'document' && request.payload.action === 'release') queueMicrotask(() =>
                this.emit('message', createCemProcessingFailureEnvelope(request, 'failure', [
                    { code: 'fixture.cleanup', severity: 'error', message: 'cleanup failed' },
                ])));
        }
    }
    const worker = new FailedReleaseWorker();
    const scope = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(scope, { workerScriptUrl: new URL('https://example.test/worker.js'), workerFactory: () => worker as unknown as Worker });
    const handle = { documentKey: 'doc', instanceId: 'editor', scopePolicyStamp: 'policy' };
    const input = { action: 'retain' as const, handle: { ...handle }, bytes: new TextEncoder().encode('{}').buffer,
        contentType: 'application/json', sourceUri: 'https://example.test/doc.json' };
    const pending = host.document(input); input.handle.instanceId = 'mutated-after-admission';
    await flushMicrotasks(); worker.respondDocument(); await pending.result;
    expect(worker.requests[0].payload).toMatchObject({ handle });
    const bindings = new Map([[host.checkpointClient!, ['editor']]]);
    expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    const capsule = {}; host.witnessCheckpointDocument!(handle, capsule);
    const proof = nativeCheckpointSourceProof(capsule)!;
    expect(proof.current()).toBe(true);
    expect(nativeCheckpointSnapshot(bindings, new Map([[proof.client, new Set([proof.resource])]]))).toBeDefined();
    await expect(host.document({ action: 'release', handle }).result).rejects.toThrow('cleanup failed');
    expect(nativeCheckpointIdentity(host.checkpointClient!)).toBeUndefined();
    expect(proof.current()).toBe(false);
    await host.dispose({}).result; scope.dispose();
});


it('keeps a host-only stylesheet load in coverage while it waits for host I/O', async () => {
    const worker = new StylesheetProcessingWorker(), scope = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(scope, { workerScriptUrl: new URL('https://example.test/worker.js'), workerFactory: () => worker as unknown as Worker });
    const { artifact } = await host.compile(compileInput('host-css')).result;
    const bindings = new Map([[host.checkpointClient!, ['view']]]);
    expect(nativeCheckpointSnapshot(bindings)).toBeDefined();
    const load = await host.stylesheet({ ...stylesheetBegin, artifact }).result;
    expect(load.status).toBe('pending');
    expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    await host.stylesheet({ action: 'release', artifact, consumer: stylesheetBegin.consumer }).result;
    expect(nativeCheckpointSnapshot(bindings)).toBeDefined();
    await host.dispose({}).result; scope.dispose();
});


it('rejects a prepared native result when its worker generation is lost before acknowledgement', async () => {
    class LostBeforeAcknowledgement extends ControlledProcessingWorker {
        private failure?: (event: MessageEvent<unknown>) => void;
        override addEventListener(type: string, listener: (event: MessageEvent<unknown>) => void): void {
            if (type === 'error') this.failure = listener;
            super.addEventListener(type, listener);
        }
        readySession(): void {
            const request = this.requests.find(request => request.operation === 'native-session');
            if (!request || request.operation !== 'native-session') throw new Error('missing native preparation');
            this.emit('message', createCemProcessingSuccessEnvelope(request, { status: 'ready', handle: request.payload.handle, length: 1 }));
        }
        lose(): void { this.failure?.({ preventDefault() { return undefined; }, error: new Error('lost native generation') } as unknown as MessageEvent<unknown>); }
    }
    const worker = new LostBeforeAcknowledgement(), scope = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(scope, { workerScriptUrl: new URL('https://example.test/worker.js'), workerFactory: () => worker as unknown as Worker });
    const job = host.nativeSession({ action: 'prepare', handle: { sessionKey: 'race', instanceId: 'source', sourceRevision: '1', scopePolicyStamp: 'policy' },
        sources: { kind: 'cem-native-session-sources-v1', requesting: 0, sources: [], bindings: [], grants: [] }, data: {}, select: 'input',
        limits: { maxBytes: 1000, maxValues: 100, maxDepth: 10 } });
    await flushMicrotasks();
    let responding = true;
    const stop = nativeCheckpointSubscribe(host.checkpointClient!, () => { if (responding) { responding = false; worker.lose(); } });
    worker.readySession();
    await expect(job.result).rejects.toThrow('generation changed');
    stop(); await host.dispose({}).result; scope.dispose();
});


it('retires every pooled root without fallback, repeated cleanup or stale native authority', async () => {
    class RetirementWorker extends ControlledProcessingWorker {
        preparations = 0;
        override postMessage(request: CemProcessingRequestEnvelope | CemRenderingRetirementControl): void {
            if (!isCemRenderingRetirementControl(request)) { super.postMessage(request); return; }
            if (request.action === 'prepare') this.preparations++;
            queueMicrotask(() => this.emit('message', { protocol: CEM_RENDERING_RETIREMENT_PROTOCOL,
                direction: 'response', id: request.id, action: request.action, ok: true,
                ...(request.action === 'prepare' ? { confirmation: { schemaVersion: 1, frozen: true, inventoryRevision: 3,
                    frozenRevision: 4, replacementAllowed: false } } : {}) }));
        }
    }
    const worker = new RetirementWorker(), document = {} as Document;
    const a = createCemDeclarationScope({ document }), b = createCemDeclarationScope({ document });
    const options = { workerScriptUrl: new URL('https://example.test/retirement.js'), workerFactory: () => worker as unknown as Worker,
        poolPolicy: { workerCount: 1, maxWorkers: 1, queueSize: 8 } };
    const first = cemProcessingHostForScope(a, options), second = cemProcessingHostForScope(b, options);
    await Promise.all([first.ready, second.ready]);
    const lost = vi.fn(); first.onNativeAuthorityLost!(lost); second.onNativeAuthorityLost!(lost);
    const proof = await closeNativeRenderingGeneration(first.checkpointClient!);
    const handoff = { current: () => true, retain: () => new Uint8Array([1]) };
    await expect(retireNativeRenderingGeneration(new Map([[first.checkpointClient!, ['a']]]), [], proof, handoff)).rejects.toThrow('coverage');
    expect(worker.preparations).toBe(0);
    await retireNativeRenderingGeneration(new Map([[first.checkpointClient!, ['a']], [second.checkpointClient!, ['b']]]), [], proof, handoff);
    expect(worker.preparations).toBe(1); expect(worker.terminated).toBe(true); expect(lost).toHaveBeenCalledTimes(2);
    expect(first.mode).toBe('worker'); expect(second.mode).toBe('worker');
    await expect(first.compile(compileInput('retired')).result).rejects.toThrow('retired');
    await Promise.all([first.dispose({ reason: 'runtime-disposed' }).result, second.dispose({ reason: 'runtime-disposed' }).result]);
    expect(worker.requests).toHaveLength(0); a.dispose(); b.dispose();
});


it('fences retirement on actual worker loss and never moves the closed root to fallback', async () => {
    class LostRetirementWorker extends ControlledProcessingWorker {
        private failure?: (event: MessageEvent<unknown>) => void;
        override addEventListener(type: string, listener: (event: MessageEvent<unknown>) => void): void {
            if (type === 'error') this.failure = listener;
            super.addEventListener(type, listener);
        }
        override postMessage(request: CemProcessingRequestEnvelope | CemRenderingRetirementControl): void {
            if (!isCemRenderingRetirementControl(request)) { super.postMessage(request); return; }
            queueMicrotask(() => {
                if (request.action === 'prepare') this.failure?.({ preventDefault() { return undefined; }, error: new Error('lost rendering worker') } as unknown as MessageEvent<unknown>);
                else this.emit('message', { protocol: CEM_RENDERING_RETIREMENT_PROTOCOL, direction: 'response', id: request.id, action: request.action, ok: true });
            });
        }
    }
    const worker = new LostRetirementWorker(), scope = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(scope, { workerScriptUrl: new URL('https://example.test/loss.js'), workerFactory: () => worker as unknown as Worker });
    await host.ready;
    const proof = await closeNativeRenderingGeneration(host.checkpointClient!), bytes = new Uint8Array([7]);
    await expect(retireNativeRenderingGeneration(new Map([[host.checkpointClient!, ['a']]]), [], proof,
        { current: () => true, retain: () => bytes })).rejects.toThrow('lost rendering worker');
    expect([...bytes]).toEqual([7]); expect(host.mode).toBe('worker'); expect(worker.requests).toHaveLength(0);
    expect(nativeCheckpointDrainCurrent(proof)).toBe(false);
    await expect(host.cancel({ targetJobId: 1 }).result).rejects.toThrow('retired');
    expect(host.mode).toBe('worker');
    await host.dispose({ reason: 'runtime-disposed' }).result; scope.dispose();
});

it.each(['revoke', 'worker-loss'])('fences generation-bound fresh roots without fallback on %s', async scenario => {
    class FreshWorker extends ControlledProcessingWorker {
        override postMessage(request: CemProcessingRequestEnvelope | CemRenderingRetirementControl): void {
            if (!isCemRenderingRetirementControl(request)) { super.postMessage(request); return; }
            queueMicrotask(() => this.emit('message', { protocol: CEM_RENDERING_RETIREMENT_PROTOCOL,
                direction: 'response', id: request.id, action: request.action, ok: true,
                ...(request.action === 'inspect' ? { confirmation: { softPressure: false, hardPressure: false } } : {}),
                ...(request.action === 'prepare' ? { confirmation: { schemaVersion: 1, frozen: true, inventoryRevision: 3,
                    frozenRevision: 4, replacementAllowed: false } } : {}) }));
        }
    }
    const { nativeGenerationParticipant } = await import('./checkpoint-domain.js');
    const { CemNativeGenerationGroup } = await import('./native-generation.js');
    const { createFreshCemProcessingHost } = await import('./processing-host-runtime.js');
    const workers: FreshWorker[] = [], scope = createCemDeclarationScope({ document: {} as Document });
    const host = cemProcessingHostForScope(scope, { workerScriptUrl: 'https://example.test/fresh.js', workerFactory: () => {
        const worker = new FreshWorker(); workers.push(worker); return worker as unknown as Worker;
    } });
    await host.ready;
    const client = host.checkpointClient;
    if (!client) throw new Error('missing native client');
    const group = new CemNativeGenerationGroup([nativeGenerationParticipant(client)]);
    const proof = await closeNativeRenderingGeneration(client);
    const retired = await retireNativeRenderingGeneration(new Map([[client, ['editor']]]), [], proof,
        { current: () => true, retain: () => new Uint8Array([1]) });
    const admission = await group.allocate([retired]);
    try {
        const fresh = admission.generations[0].runtime as import('./processing-host-runtime.js').CemFreshRenderingGeneration;
        expect(() => createFreshCemProcessingHost(scope, { ...fresh })).toThrow('binding');
        const first = createFreshCemProcessingHost(scope, fresh), otherScope = createCemDeclarationScope({ document: scope.document });
        const second = createFreshCemProcessingHost(otherScope, fresh);
        expect(first).not.toBe(host); expect(second).not.toBe(first);
        expect(createFreshCemProcessingHost(scope, fresh)).toBe(first);
        expect(nativeCheckpointIdentity(first.checkpointClient!)).toBe(admission.generations[0].runtimeId);
        const pending = first.compile(compileInput('fresh-pending')).result;
        const peer = second.compile(compileInput('peer-pending')).result;
        const rejected = expect(pending).rejects.toThrow(scenario === 'revoke' ? /retired|disposed|closed/ : /fresh-failure/);
        const peerRejected = expect(peer).rejects.toThrow(scenario === 'revoke' ? /retired|disposed|closed/ : /fresh-failure/);
        await flushMicrotasks(); expect(workers[1].requests).toHaveLength(2);
        expect(new Set(workers[1].requests.map(request => request.jobId)).size).toBe(2);
        if (scenario === 'revoke') admission.revoke();
        else workers[1].fail();
        await rejected; await peerRejected;
        expect(first.mode).toBe('worker'); expect(second.mode).toBe('worker');
        admission.revoke(); otherScope.dispose();
        expect(workers[1].terminated).toBe(true);
    } finally { admission.revoke(); scope.dispose(); }
});
