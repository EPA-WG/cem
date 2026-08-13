export const WORKER_PROTOCOL_VERSION = 1;
export const OPERATION_PROTOCOL_VERSION = 1;
export const MAX_COORDINATED_WORKERS = 256;
export const DEFAULT_MAX_NODE_WORKERS = 8;
export const DEFAULT_MAX_BROWSER_WORKERS = 8;
export const DEFAULT_STARTUP_TIMEOUT_MS = 10_000;
export const MAX_STARTUP_TIMEOUT_MS = 60_000;
export const DEFAULT_HARD_CANCEL_GRACE_MS = 2_000;
export const MIN_HARD_CANCEL_GRACE_MS = 10;
export const MAX_HARD_CANCEL_GRACE_MS = 30_000;
export const WORK_PACKET_PROTOCOL_VERSION = 1;

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
    readonly kind: 'initialize' | 'run' | 'progress' | 'event' | 'result' | 'control';
    readonly operationId?: number;
    readonly sequence?: number;
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

export type OperationWorkDomain = 'transform' | 'query';

export interface OperationWorkStage {
    readonly domain: OperationWorkDomain;
    readonly ordinal: number;
    readonly label: string;
}

export interface OperationWorkPacket {
    readonly workProtocolVersion: number;
    readonly operationId: number;
    readonly taskId: number;
    readonly scopeId: number;
    readonly worker: WorkerAddress;
    readonly attempt: number;
    readonly commitSequence: number;
    readonly stage: OperationWorkStage;
    readonly payload: unknown;
    readonly transfers?: readonly never[];
}

export interface OperationWorkResult extends Omit<OperationWorkPacket, 'payload'> {
    readonly status: 'succeeded' | 'failed' | 'cancelled';
    readonly payload: unknown;
}

export interface WorkerWorkRequest {
    readonly type: 'cem-operation-work';
    readonly packet: OperationWorkPacket;
}

export interface WorkerWorkResultEnvelope extends WorkerEnvelope<OperationWorkResult> {
    readonly operation: {
        readonly protocolVersion: number;
        readonly kind: 'result';
        readonly operationId: number;
        readonly sequence: number;
        readonly payload: OperationWorkResult;
    };
}

export interface OperationSource {
    readonly uri: string;
    readonly bytes: readonly number[];
    readonly fromFormat?: 'cem' | 'html' | 'xml';
    readonly identity: {
        readonly contentType?: string;
        readonly schema?: string;
        readonly defaultNamespace?: string;
        readonly namespaces?: Readonly<Record<string, string>>;
        readonly baseUri?: string;
    };
    readonly rootScope?: Readonly<Record<string, unknown>>;
}

export interface TransformOperationRunRequest {
    readonly kind: 'transform';
    readonly data: OperationSource;
    readonly template: OperationSource;
    readonly params?: Readonly<Record<string, unknown>>;
    readonly templateEntrypoint?: { readonly name?: string };
    readonly target?: OperationSource['identity'];
    readonly targetScope?: Readonly<Record<string, unknown>>;
    readonly preserveSourceOffsets?: boolean;
}

export interface QueryOperationRunRequest {
    readonly kind: 'query';
    readonly data: OperationSource;
    readonly query: OperationSource;
}

export type ResumableOperationRunRequest = TransformOperationRunRequest | QueryOperationRunRequest;

export interface ResumableOperationTerminal {
    readonly status: 'succeeded' | 'failed' | 'cancelled' | 'fatal';
    readonly result?: unknown;
    readonly error?: { readonly code: string; readonly message: string };
    readonly reason?: string;
}

export interface ResumableWorkerReplacement {
    readonly previous: WorkerAddress;
    readonly replacement: WorkerAddress;
    readonly affectedOperationIds: readonly number[];
    readonly retryPackets: readonly OperationWorkPacket[];
}

export type ResumableOperationEvent =
    | {
          readonly kind: 'state';
          readonly operationId: string;
          readonly state: 'running' | 'pause-requested' | 'paused' | 'stepping' | 'cancelling' | 'terminal';
      }
    | {
          readonly kind: 'dispatch';
          readonly operationId: string;
          readonly taskId: string;
          readonly stage: OperationWorkStage;
          readonly worker: WorkerAddress;
      }
    | {
          readonly kind: 'commit';
          readonly operationId: string;
          readonly taskIds: readonly string[];
      }
    | {
          readonly kind: 'worker-replaced';
          readonly operationId: string;
          readonly previous: WorkerAddress;
          readonly replacement: WorkerAddress;
      }
    | {
          readonly kind: 'terminal';
          readonly operationId: string;
          readonly terminal: ResumableOperationTerminal;
      };
