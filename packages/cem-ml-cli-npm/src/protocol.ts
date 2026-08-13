export const WORKER_PROTOCOL_VERSION = 1;
export const OPERATION_PROTOCOL_VERSION = 1;
export const MAX_COORDINATED_WORKERS = 256;
export const DEFAULT_MAX_NODE_WORKERS = 8;
export const DEFAULT_MAX_BROWSER_WORKERS = 8;
export const DEFAULT_STARTUP_TIMEOUT_MS = 10_000;
export const MAX_STARTUP_TIMEOUT_MS = 60_000;

export interface WorkerAddress {
    readonly slot: number;
    readonly generation: number;
}

export interface WorkerProtocolLimits {
    readonly maxWorkers: number;
    readonly maxTransferBuffersPerMessage: number;
    readonly maxTransferBytesPerMessage: number;
}

export interface WorkerProtocolDescriptor {
    readonly workerProtocolVersion: number;
    readonly operationProtocolVersion: number;
    readonly limits: WorkerProtocolLimits;
}

export interface NodeWorkerCapabilityManifest {
    readonly contractVersion: number;
    readonly commonVersion: string;
    readonly runtime: 'wasm-node';
    readonly targetIdentity: string;
    readonly abiIdentity: string;
    readonly executorTopology: 'node-worker-pool';
    readonly effectiveMaxWorkers: number;
    readonly [field: string]: unknown;
}

export interface BrowserWorkerCapabilityManifest {
    readonly contractVersion: number;
    readonly commonVersion: string;
    readonly runtime: 'wasm-browser-worker';
    readonly targetIdentity: string;
    readonly abiIdentity: string;
    readonly executorTopology: 'browser-worker-pool';
    readonly effectiveMaxWorkers: number;
    readonly [field: string]: unknown;
}

export interface BrowserMainThreadCapabilityManifest {
    readonly contractVersion: number;
    readonly commonVersion: string;
    readonly runtime: 'wasm-browser-worker';
    readonly targetIdentity: string;
    readonly abiIdentity: string;
    readonly executorTopology: 'sequential';
    readonly effectiveMaxWorkers: 1;
    readonly [field: string]: unknown;
}

export interface NodeWorkerInitializePayload {
    readonly runtimeInstanceId: string;
    readonly threadId: number;
    readonly commonVersion: string;
    readonly protocol: WorkerProtocolDescriptor;
    readonly capability: NodeWorkerCapabilityManifest;
}

export interface BrowserWorkerInitializePayload {
    readonly runtimeInstanceId: string;
    readonly commonVersion: string;
    readonly protocol: WorkerProtocolDescriptor;
    readonly capability: BrowserWorkerCapabilityManifest;
}

export interface OperationHostEnvelope<T> {
    readonly protocolVersion: number;
    readonly kind: 'initialize';
    readonly payload: T;
}

export interface WorkerEnvelope<T> {
    readonly workerProtocolVersion: number;
    readonly worker: WorkerAddress;
    readonly sequence: number;
    readonly operation: OperationHostEnvelope<T>;
    readonly transfers: readonly never[];
}

export interface NodeWorkerBootstrap {
    readonly worker: WorkerAddress;
    readonly effectiveWorkers: number;
}

export interface BrowserWorkerBootstrap {
    readonly type: 'cem-worker-initialize';
    readonly worker: WorkerAddress;
    readonly effectiveWorkers: number;
    readonly runtimeHostId: string;
    readonly abiIdentity: string;
}

export type NodeWorkerInitializeEnvelope = WorkerEnvelope<NodeWorkerInitializePayload>;
export type BrowserWorkerInitializeEnvelope = WorkerEnvelope<BrowserWorkerInitializePayload>;
