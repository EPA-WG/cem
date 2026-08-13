import * as runtime from '@epa-wg/cem-ml/wasm';

import type {
    BrowserWorkerBootstrap,
    BrowserWorkerCapabilityManifest,
    BrowserWorkerInitializeEnvelope,
    WorkerProtocolDescriptor,
} from './protocol.js';

const workerScope = globalThis as unknown as DedicatedWorkerGlobalScope;
const initializeRuntime = (runtime as unknown as { readonly default: () => Promise<unknown> }).default;
let lifecycle: 'initializing' | 'loading' | 'ready' = 'initializing';

workerScope.addEventListener('message', (event: MessageEvent<unknown>) => {
    if (isRecord(event.data) && event.data.type === 'cem-worker-close') {
        workerScope.close();
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
    workerScope.postMessage(envelope);
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
