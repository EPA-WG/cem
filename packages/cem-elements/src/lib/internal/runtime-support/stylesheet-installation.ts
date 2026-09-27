import type { CemStylesheetConsumerLease, CemOwnedStylesheet } from '../../declaration-style-ownership.js';
import type { CemModuleUrlContextWire } from './module-url-resolution.js';
import {
    cemProcessingFailureDiagnostics,
    type CemProcessingArtifactHandle,
    type CemProcessingHost,
    type CemProcessingStylesheetDiagnostic,
    type CemProcessingStylesheetInput,
    type CemProcessingStylesheetResult,
    type CemProcessingDiagnostic,
} from './processing-host.js';

type Begin = Extract<CemProcessingStylesheetInput, { action: 'begin' }>;
type Pending = Extract<CemProcessingStylesheetResult, { status: 'pending' }>;
type Diagnostic = CemProcessingDiagnostic | CemProcessingStylesheetDiagnostic;

export interface CemStylesheetResponse {
    bytes: ArrayBuffer;
    finalUrl: string;
    contentType?: string;
}

export interface CemStylesheetInstallationOptions<TScope extends Begin['scope'] = Exclude<Begin['scope'], { kind: 'instance' }>> {
    host: CemProcessingHost;
    artifact: CemProcessingArtifactHandle;
    consumer: string;
    lease: CemStylesheetConsumerLease<NoInfer<TScope>>;
    baseUrl: string;
    context: CemModuleUrlContextWire;
    occurrences: ReadonlyArray<{ index: number; scope: TScope }>;
    /** Transport supplies bytes; native import owns resolution and response policy. */
    read(request: Pending['request'], signal: AbortSignal): Promise<CemStylesheetResponse>;
}

export interface CemStylesheetInstallation {
    ready: Promise<{ status: 'ready' | 'cancelled'; diagnostics: Diagnostic[]; installed: number }>;
    /** Release this generation and wait for its submitted cleanup operations. */
    dispose(): Promise<void>;
}

const cancelled = Symbol('stylesheet installation cancelled');

/** Owns the async native load transaction; the browser never parses CSS here. */
export function installRetainedStylesheets<TScope extends Begin['scope']>(options: CemStylesheetInstallationOptions<TScope>): CemStylesheetInstallation {
    const { host, lease } = options;
    const { signal } = lease;
    const { artifact, consumer, baseUrl, context, occurrences } = structuredClone({
        artifact: options.artifact, consumer: options.consumer, baseUrl: options.baseUrl,
        context: options.context, occurrences: options.occurrences,
    });
    const generations = new Map<string, typeof host.mode>();
    const jobs = new Set<number>();
    const cleanup = new Set<Promise<void>>();
    const cleanupErrors: unknown[] = [];

    function clean(operation: () => Promise<unknown>, mode = host.mode): void {
        const pending = Promise.resolve().then(operation).then(() => undefined, error => {
            // Host disposal or worker replacement already drops the old native owners.
            if (!host.ownerScope.disposed && host.mode === mode) cleanupErrors.push(error);
        }).finally(() => cleanup.delete(pending));
        cleanup.add(pending);
    }
    function releaseGeneration(loadId: string): void {
        const mode = generations.get(loadId);
        if (!generations.delete(loadId) || mode !== host.mode) return;
        clean(() => host.stylesheet({ action: 'release', artifact, consumer, loadId }).result, mode);
    }
    function release(): void {
        for (const loadId of generations.keys()) releaseGeneration(loadId);
    }
    function abort(): void {
        for (const jobId of jobs) clean(() => host.cancel({ targetJobId: jobId, reason: 'superseded' }).result);
        release();
    }
    signal.addEventListener('abort', abort, { once: true });

    function active<T>(promise: Promise<T>): Promise<T> {
        return new Promise<T>((resolve, reject) => {
            const onAbort = () => reject(cancelled);
            if (signal.aborted) reject(cancelled);
            else signal.addEventListener('abort', onAbort, { once: true });
            promise.then(resolve, reject).finally(() => signal.removeEventListener('abort', onAbort));
        });
    }
    async function operation(input: CemProcessingStylesheetInput): Promise<CemProcessingStylesheetResult> {
        if (signal.aborted) throw cancelled;
        const job = host.stylesheet(input);
        jobs.add(job.jobId);
        // Observe late successful responses too; never retain a cancelled owner.
        const result = job.result.then(result => {
            if ('loadId' in result) {
                generations.set(result.loadId, host.mode);
                if (signal.aborted) releaseGeneration(result.loadId);
            }
            return result;
        }).finally(() => jobs.delete(job.jobId));
        return active(result);
    }

    const ready: CemStylesheetInstallation['ready'] = (async () => {
        const outputs: CemOwnedStylesheet<TScope>[] = [];
        const diagnostics: Diagnostic[] = [];
        try {
            for (const occurrence of occurrences) {
                let loadId: string | undefined;
                try {
                    let result = await operation({ action: 'begin', artifact, consumer, baseUrl, context, ...occurrence });
                    if ('loadId' in result) loadId = result.loadId;
                    while (result.status === 'pending') {
                        const request = result.request;
                        let response: CemStylesheetResponse;
                        try {
                            response = await active(Promise.resolve().then(() => {
                                if (signal.aborted) throw cancelled;
                                return options.read(request, signal);
                            }));
                        } catch (error) {
                            if (signal.aborted || error === cancelled) throw cancelled;
                            // Native admission owns the requesting import's source location.
                            await operation({ action: 'fail', artifact, consumer, loadId: result.loadId,
                                requestId: request.id, message: error instanceof Error ? error.message : String(error) });
                            throw new Error('native stylesheet failure did not stop the load', { cause: error });
                        }
                        result = await operation({ action: 'deliver', artifact, consumer, loadId: result.loadId,
                            requestId: result.request.id, ...response });
                    }
                    if (result.status !== 'ready') throw new Error('expected a completed native stylesheet');
                    diagnostics.push(...result.diagnostics);
                    outputs.push({ index: occurrence.index, scope: occurrence.scope, output: result });
                } catch (error) {
                    if (signal.aborted || error === cancelled) throw cancelled;
                    if (loadId !== undefined) releaseGeneration(loadId);
                    diagnostics.push(...cemProcessingFailureDiagnostics(error));
                }
            }
            if (signal.aborted) throw cancelled;
            if (!lease.commit(outputs, release)) {
                if (!signal.aborted) throw new Error('native stylesheet lease rejected the complete set');
                throw cancelled;
            }
            return { status: 'ready', diagnostics, installed: outputs.length };
        } catch (error) {
            if (error !== cancelled) diagnostics.push(...cemProcessingFailureDiagnostics(error));
            lease.release();
            release();
            return { status: 'cancelled', diagnostics, installed: 0 };
        }
    })();
    return {
        ready,
        async dispose() {
            lease.release();
            release();
            await ready;
            while (cleanup.size) await Promise.all(cleanup);
            signal.removeEventListener('abort', abort);
            if (cleanupErrors.length) throw new AggregateError(cleanupErrors, 'native stylesheet cleanup failed');
        },
    };
}
