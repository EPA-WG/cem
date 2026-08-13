import * as runtime from '@epa-wg/cem-ml/wasm';

import type {
    BrowserWorkerBootstrap,
    BrowserWorkerCapabilityManifest,
    BrowserWorkerInitializeEnvelope,
    WorkerProtocolDescriptor,
    WorkerWorkRequest,
    WorkerWorkResultEnvelope,
} from './protocol.js';
import type { ResumableRuntime } from './operation.js';

const workerScope = globalThis as unknown as DedicatedWorkerGlobalScope;
const initializeRuntime = (runtime as unknown as { readonly default: () => Promise<unknown> }).default;
let lifecycle: 'initializing' | 'loading' | 'ready' = 'initializing';
let bootstrapState: BrowserWorkerBootstrap | undefined;
let protocolState: WorkerProtocolDescriptor | undefined;
let nextSequence = 2;

workerScope.addEventListener('message', (event: MessageEvent<unknown>) => {
    if (isRecord(event.data) && event.data.type === 'cem-worker-close') {
        workerScope.close();
        return;
    }
    if (lifecycle === 'ready' && isWorkRequest(event.data)) {
        executeWork(event.data);
        return;
    }
    if (lifecycle !== 'initializing') {
        reportFailure(new Error('CEM-ML browser worker accepts exactly one initialization message'));
        return;
    }
    lifecycle = 'loading';
    void initialize(parseBootstrap(event.data)).catch(reportFailure);
});

async function initialize(bootstrap: BrowserWorkerBootstrap): Promise<void> {
    await initializeRuntime();
    const protocol = JSON.parse(runtime.workerProtocolDescriptor()) as WorkerProtocolDescriptor;
    const capabilityRequest = JSON.stringify({
        runtime: 'wasm-browser-worker',
        targetIdentity: 'wasm32-unknown-unknown:web',
        abiIdentity: bootstrap.abiIdentity,
        debugControlActive: false,
    });
    const capability = JSON.parse(
        runtime.browserWorkerCapabilityManifest(capabilityRequest, bootstrap.effectiveWorkers),
    ) as BrowserWorkerCapabilityManifest & { readonly error?: unknown };
    if (capability.error !== undefined) {
        throw new Error('CEM-ML browser worker capability projection failed');
    }

    const envelope: BrowserWorkerInitializeEnvelope = {
        workerProtocolVersion: protocol.workerProtocolVersion,
        worker: bootstrap.worker,
        sequence: 1,
        operation: {
            protocolVersion: protocol.operationProtocolVersion,
            kind: 'initialize',
            payload: {
                runtimeInstanceId: `${bootstrap.runtimeHostId}:slot-${bootstrap.worker.slot}:generation-${bootstrap.worker.generation}`,
                commonVersion: runtime.version(),
                protocol,
                capability,
            },
        },
        transfers: [],
    };

    lifecycle = 'ready';
    bootstrapState = bootstrap;
    protocolState = protocol;
    workerScope.postMessage(envelope);
}

function executeWork(message: WorkerWorkRequest): void {
    const bootstrap = bootstrapState;
    const protocol = protocolState;
    if (bootstrap === undefined || protocol === undefined) {
        throw new Error('CEM-ML browser worker received work before initialization');
    }
    const packet = message.packet;
    if (
        packet.worker.slot !== bootstrap.worker.slot ||
        packet.worker.generation !== bootstrap.worker.generation
    ) {
        throw new Error('CEM-ML work packet targets a different browser worker generation');
    }
    const result = parseRuntimeResult(
        (runtime as unknown as ResumableRuntime).executeOperationWork(JSON.stringify(packet)),
    );
    const response: WorkerWorkResultEnvelope = {
        workerProtocolVersion: protocol.workerProtocolVersion,
        worker: bootstrap.worker,
        sequence: nextSequence++,
        operation: {
            protocolVersion: protocol.operationProtocolVersion,
            kind: 'result',
            operationId: packet.operationId,
            sequence: packet.taskId,
            payload: result,
        },
        transfers: [],
    };
    workerScope.postMessage(response);
}

function parseRuntimeResult(json: string): WorkerWorkResultEnvelope['operation']['payload'] {
    const value = JSON.parse(json) as unknown;
    if (isRecord(value) && isRecord(value.error)) {
        throw new Error(
            typeof value.error.message === 'string' ? value.error.message : 'CEM-ML worker execution failed',
        );
    }
    return value as WorkerWorkResultEnvelope['operation']['payload'];
}

function isWorkRequest(value: unknown): value is WorkerWorkRequest {
    return isRecord(value) && value.type === 'cem-operation-work' && isRecord(value.packet);
}

function parseBootstrap(value: unknown): BrowserWorkerBootstrap {
    if (
        !isRecord(value) ||
        value.type !== 'cem-worker-initialize' ||
        !isRecord(value.worker) ||
        !isPositiveInteger(value.worker.slot) ||
        !isPositiveInteger(value.worker.generation) ||
        !isPositiveInteger(value.effectiveWorkers) ||
        typeof value.runtimeHostId !== 'string' ||
        value.runtimeHostId.length === 0 ||
        typeof value.abiIdentity !== 'string' ||
        value.abiIdentity.length === 0
    ) {
        throw new Error('CEM-ML browser worker bootstrap is invalid');
    }
    return {
        type: 'cem-worker-initialize',
        worker: {
            slot: value.worker.slot,
            generation: value.worker.generation,
        },
        effectiveWorkers: value.effectiveWorkers,
        runtimeHostId: value.runtimeHostId,
        abiIdentity: value.abiIdentity,
    };
}

function reportFailure(error: unknown): void {
    const failure = error instanceof Error ? error : new Error(String(error));
    setTimeout(() => {
        throw failure;
    });
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isPositiveInteger(value: unknown): value is number {
    return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}
