/// <reference lib="webworker" />

import { CemProcessingEngine } from './processing-engine.js';
import {
    assertCemProcessingEnvelope,
    createCemProcessingFailureEnvelope,
    createCemProcessingReadyEnvelope,
    createCemProcessingSuccessEnvelope,
    type CemProcessingDiagnostic,
    type CemProcessingRequestEnvelope,
} from './processing-host.js';

const workerScope = self as unknown as DedicatedWorkerGlobalScope;
const engine = new CemProcessingEngine();
const cancelledJobs = new Set<number>();

workerScope.addEventListener('message', (event: MessageEvent<unknown>) => {
    void handleMessage(event.data);
});

workerScope.postMessage(createCemProcessingReadyEnvelope('worker'));

async function handleMessage(message: unknown): Promise<void> {
    let request: CemProcessingRequestEnvelope;
    try {
        assertCemProcessingEnvelope(message);
        if (message.direction !== 'request') {
            throw new TypeError('the CEM processing worker accepts request envelopes only');
        }
        request = message;
    } catch {
        return;
    }

    try {
        if (request.operation === 'cancel') {
            cancelledJobs.add(request.payload.targetJobId);
            workerScope.postMessage(createCemProcessingSuccessEnvelope(request, {
                targetJobId: request.payload.targetJobId,
                accepted: true,
            }));
            return;
        }
        if (cancelledJobs.has(request.jobId)) {
            workerScope.postMessage(createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()]));
            return;
        }
        if (request.operation === 'compile') {
            const result = await engine.compile(request.payload);
            if (cancelledJobs.has(request.jobId)) {
                workerScope.postMessage(createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()]));
            } else {
                workerScope.postMessage(createCemProcessingSuccessEnvelope(request, result));
            }
            return;
        }
        if (request.operation === 'render-diff') {
            const result = await engine.renderDiff(request.payload);
            if (cancelledJobs.has(request.jobId)) {
                workerScope.postMessage(createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()]));
            } else {
                workerScope.postMessage(createCemProcessingSuccessEnvelope(request, result));
            }
            return;
        }
        const result = engine.dispose(request.payload);
        workerScope.postMessage(createCemProcessingSuccessEnvelope(request, result));
        workerScope.close();
    } catch (error) {
        workerScope.postMessage(createCemProcessingFailureEnvelope(request, 'failure', [failureDiagnostic(error)]));
    }
}

function cancelledDiagnostic(): CemProcessingDiagnostic {
    return {
        code: 'cem.processing_host.job_cancelled',
        severity: 'info',
        message: 'the CEM processing job was cancelled',
    };
}

function failureDiagnostic(error: unknown): CemProcessingDiagnostic {
    return {
        code: 'cem.processing_host.execution_failed',
        severity: 'error',
        message: error instanceof Error ? error.message : 'the CEM processing job failed',
    };
}
