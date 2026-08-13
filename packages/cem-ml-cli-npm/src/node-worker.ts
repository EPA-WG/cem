import { readFile } from 'node:fs/promises';
import { parentPort, threadId, workerData } from 'node:worker_threads';

import * as runtime from '@epa-wg/cem-ml/wasm';

import type {
    NodeWorkerBootstrap,
    NodeWorkerCapabilityManifest,
    NodeWorkerInitializeEnvelope,
    WorkerProtocolDescriptor,
} from './protocol.js';

const port = parentPort;
if (port === null) {
    throw new Error('CEM-ML Node worker host requires a worker-thread parent port');
}

const bootstrap = parseBootstrap(workerData as unknown);
const runtimeMetadataUrl = new URL(import.meta.resolve('@epa-wg/cem-ml/runtime.json'));
const runtimeMetadata = JSON.parse(await readFile(runtimeMetadataUrl, 'utf8')) as unknown;
if (!isRecord(runtimeMetadata) || !isRecord(runtimeMetadata.abi)) {
    throw new Error('CEM-ML runtime metadata is invalid');
}
const abiIdentity = runtimeMetadata.abi.identity;
if (typeof abiIdentity !== 'string' || abiIdentity.length === 0) {
    throw new Error('CEM-ML runtime metadata does not provide an ABI identity');
}

const protocol = JSON.parse(runtime.workerProtocolDescriptor()) as WorkerProtocolDescriptor;
const capabilityRequest = JSON.stringify({
    runtime: 'wasm-node',
    targetIdentity: 'wasm32-unknown-unknown:nodejs',
    abiIdentity,
    debugControlActive: false,
});
const capability = JSON.parse(
    runtime.nodeWorkerCapabilityManifest(capabilityRequest, bootstrap.effectiveWorkers),
) as NodeWorkerCapabilityManifest & { readonly error?: unknown };
if (capability.error !== undefined) {
    throw new Error('CEM-ML Node worker capability projection failed');
}

const commonVersion = runtime.version();
const envelope: NodeWorkerInitializeEnvelope = {
    workerProtocolVersion: protocol.workerProtocolVersion,
    worker: bootstrap.worker,
    sequence: 1,
    operation: {
        protocolVersion: protocol.operationProtocolVersion,
        kind: 'initialize',
        payload: {
            runtimeInstanceId: `node-thread-${threadId}:slot-${bootstrap.worker.slot}:generation-${bootstrap.worker.generation}`,
            threadId,
            commonVersion,
            protocol,
            capability,
        },
    },
    transfers: [],
};

port.postMessage(envelope);
port.on('message', (message: unknown) => {
    if (isRecord(message) && message.type === 'cem-worker-close') {
        port.close();
    }
});

function parseBootstrap(value: unknown): NodeWorkerBootstrap {
    if (
        !isRecord(value) ||
        !isRecord(value.worker) ||
        !isPositiveInteger(value.worker.slot) ||
        !isPositiveInteger(value.worker.generation) ||
        !isPositiveInteger(value.effectiveWorkers)
    ) {
        throw new Error('CEM-ML Node worker bootstrap is invalid');
    }
    return {
        worker: {
            slot: value.worker.slot,
            generation: value.worker.generation,
        },
        effectiveWorkers: value.effectiveWorkers,
    };
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isPositiveInteger(value: unknown): value is number {
    return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}
