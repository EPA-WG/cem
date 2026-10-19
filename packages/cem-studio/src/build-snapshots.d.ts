import type { CemStudioBuildRun, CemStudioWorkbenchProjection } from './workbench.js';
export const CEM_STUDIO_PIN_LIMITS: Readonly<{ count: number; totalBytes: number; artifactCount: number; artifactBytes: number; metadataBytes: number }>;
export interface CemStudioBuildSnapshot {
    readonly version: 1;
    readonly run: CemStudioBuildRun;
    readonly outputs: readonly { readonly uri: string; readonly bytes: ArrayBuffer | Uint8Array }[];
    readonly storageBytes?: number;
}
export interface CemStudioStoredBuildSnapshot extends CemStudioBuildSnapshot {
    readonly key: string;
    readonly projectId: string;
    readonly createdAt: string;
}
export interface CemStudioBuildSnapshotSummary {
    readonly key: string;
    readonly projectId: string;
    readonly createdAt: string;
    readonly runId: string;
    readonly configurationId: string;
    readonly name: string;
    readonly byteLength: number;
    readonly projectRevision: number;
}
export function captureCemStudioBuildSnapshot(projection: CemStudioWorkbenchProjection): CemStudioBuildSnapshot;
export function verifyCemStudioBuildSnapshot(value: unknown, crypto?: Crypto): Promise<CemStudioBuildSnapshot & { readonly storageBytes: number }>;
export function cemStudioBuildSnapshotSummary(record: CemStudioStoredBuildSnapshot): CemStudioBuildSnapshotSummary;
export type CemStudioPinnedOutput = Pick<import('./workbench.js').CemStudioBuildArtifact,
    'uri' | 'contentType' | 'byteLength' | 'sha256' | 'available'> & { readonly bytes: Uint8Array; readonly text?: string };
export type CemStudioPinnedBuildProjection = Omit<Partial<CemStudioWorkbenchProjection>, 'kind' | 'output' | 'outputs'> & {
    readonly kind: 'snapshot';
    readonly mode: 'pinned';
    readonly exitCode: 0;
    readonly buildRun: CemStudioBuildRun;
    readonly outputs: readonly CemStudioPinnedOutput[];
    readonly output?: CemStudioPinnedOutput;
    readonly diagnostics: readonly Readonly<Record<string, unknown>>[];
    readonly stale: false;
};
export function openCemStudioBuildSnapshot(record: CemStudioStoredBuildSnapshot): {
    snapshot(): { projection: CemStudioPinnedBuildProjection };
    selectArtifact(id: string): void;
    copyProjection(writeText?: (text: string) => Promise<void>): Promise<void>;
    downloadProjection(save?: (file: { filename: string; contentType: string; bytes: Uint8Array }) => Promise<void>): Promise<void>;
    dispose(): void;
};

export type CemStudioBuildComparisonStatus = 'added' | 'removed' | 'changed' | 'unchanged';
export interface CemStudioBuildComparison {
    readonly baselineRunId: string;
    readonly currentRunId: string;
    readonly baselineProjectRevision: number;
    readonly currentProjectRevision: number;
    readonly configurationChanged: boolean;
    readonly baselineConfiguration: CemStudioBuildRun['configuration'];
    readonly currentConfiguration: CemStudioBuildRun['configuration'];
    readonly executionIdentityChanged: boolean;
    readonly baselineExecutionIdentity: CemStudioBuildRun['executionIdentity'];
    readonly currentExecutionIdentity: CemStudioBuildRun['executionIdentity'];
    readonly counts: Readonly<Record<CemStudioBuildComparisonStatus, number>>;
    readonly artifacts: readonly {
        readonly path: string;
        readonly status: CemStudioBuildComparisonStatus;
        readonly bytesChanged: boolean;
        readonly metadataChanged: boolean;
        readonly provenanceChanged: boolean;
        readonly before?: CemStudioComparedArtifact;
        readonly after?: CemStudioComparedArtifact;
    }[];
    readonly inputs: readonly {
        readonly resourceId: string;
        readonly status: CemStudioBuildComparisonStatus;
        readonly before?: CemStudioBuildRun['inputs'][number];
        readonly after?: CemStudioBuildRun['inputs'][number];
    }[];
}
export interface CemStudioComparedArtifact {
    readonly sha256: string;
    readonly byteLength: number;
    readonly contentType: string;
    readonly provenance: readonly Readonly<Record<string, unknown>>[];
}
/** Inputs must be admitted, byte-verified successful manifests. Does not load outputs. */
export function compareCemStudioBuildRuns(baseline: CemStudioBuildRun, current: CemStudioBuildRun): CemStudioBuildComparison;
