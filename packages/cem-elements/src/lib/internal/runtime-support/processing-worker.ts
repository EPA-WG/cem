/// <reference lib="webworker" />
import { runtimeMemoryStatus } from './native-runtime.js';

import { processCheckpointControl, ensureRuntimeReady, closeRenderingNativeAdmission, renderingNativeResourceInventory, prepareRenderingNativeRetirement } from './cem-ql-render.js';
import { CemProcessingEngine } from './processing-engine.js';
import { cemMainThreadCheckpointDomain } from './checkpoint-domain.js';
import {
    CEM_RENDERING_RETIREMENT_PROTOCOL, isCemRenderingRetirementControl, type CemRenderingRetirementControl,
    CemProcessingCancellationRegistry,
    cemProcessingFailureDiagnostics,
    assertCemProcessingEnvelope,
    createCemProcessingFailureEnvelope,
    createCemProcessingReadyEnvelope,
    createCemProcessingPressureEnvelope,
    createCemProcessingSuccessEnvelope,
    type CemProcessingDiagnostic,
    type CemProcessingRequestEnvelope,
} from './processing-host.js';

const workerScope = self as unknown as DedicatedWorkerGlobalScope;
const engine = new CemProcessingEngine();
const jobs = new CemProcessingCancellationRegistry();
let activeRequests = 0;
let nativeClosed = false;
let permit: ReturnType<typeof prepareRenderingNativeRetirement> | undefined;
let preparing = false;
let prepared = false;


workerScope.addEventListener('message', (event: MessageEvent<unknown>) => {
    if (isCemRenderingRetirementControl(event.data) && event.data.direction === 'request') void retirementControl(event.data);
    else void handleMessage(event.data);
});

workerScope.postMessage(createCemProcessingReadyEnvelope('worker'));
cemMainThreadCheckpointDomain.onPressure(reason => workerScope.postMessage(createCemProcessingPressureEnvelope(reason)));

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

    activeRequests++;
    try {
        if (request.operation === 'cancel') {
            workerScope.postMessage(createCemProcessingSuccessEnvelope(request, {
                targetJobId: request.payload.targetJobId,
                accepted: jobs.cancel(request.payload.targetJobId),
            }));
            return;
        }
        jobs.start(request.jobId);
        if (request.operation === 'native-session') {
            const result = await engine.nativeSession(request.payload);
            if (jobs.isCancelled(request.jobId)) {
                if (request.payload.action === 'prepare') await engine.nativeSession({ action: 'release', handle: request.payload.handle });
                workerScope.postMessage(createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()]));
            } else workerScope.postMessage(createCemProcessingSuccessEnvelope(request, result));
            return;
        }
        if (request.operation === 'checkpoint') {
            const result = await engine.checkpoint(request.payload);
            workerScope.postMessage(jobs.isCancelled(request.jobId)
                ? createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()])
                : createCemProcessingSuccessEnvelope(request, result));
            return;
        }
        if (request.operation === 'value') {
            const result = await engine.value(request.payload);
            workerScope.postMessage(jobs.isCancelled(request.jobId)
                ? createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()])
                : createCemProcessingSuccessEnvelope(request, result));
            return;
        }
        if (request.operation === 'stylesheet') {
            const result = await engine.stylesheet(request.payload);
            if (jobs.isCancelled(request.jobId)) {
                if ('loadId' in result) await engine.stylesheet({ action: 'release',
                    artifact: request.payload.artifact, consumer: request.payload.consumer, loadId: result.loadId });
                workerScope.postMessage(createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()]));
            } else {
                workerScope.postMessage(createCemProcessingSuccessEnvelope(request, result));
            }
            return;
        }
        if (request.operation === 'document') {
            const result = await engine.document(request.payload);
            if (jobs.isCancelled(request.jobId) && request.payload.action === 'retain') {
                await engine.document({ action: 'release', handle: request.payload.handle });
                workerScope.postMessage(createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()]));
            } else {
                workerScope.postMessage(createCemProcessingSuccessEnvelope(request, result));
            }
            return;
        }
        if (request.operation === 'compile') {
            const result = await engine.compile(request.payload);
            if (jobs.isCancelled(request.jobId)) {
                workerScope.postMessage(createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()]));
            } else {
                workerScope.postMessage(createCemProcessingSuccessEnvelope(request, result));
            }
            return;
        }
        if (request.operation === 'render-diff') {
            const result = await engine.renderDiff(request.payload);
            if (jobs.isCancelled(request.jobId)) {
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
        workerScope.postMessage(
            jobs.isCancelled(request.jobId)
                ? createCemProcessingFailureEnvelope(request, 'cancelled', [cancelledDiagnostic()])
                : createCemProcessingFailureEnvelope(request, 'failure', cemProcessingFailureDiagnostics(error))
        );
    } finally {
        activeRequests--;
        if (request.operation !== 'cancel') {
            jobs.finish(request.jobId);
        }
    }
}

function cancelledDiagnostic(): CemProcessingDiagnostic {
    return {
        code: 'cem.processing_host.job_cancelled',
        severity: 'info',
        message: 'the CEM processing job was cancelled',
    };
}


async function retirementControl(request: CemRenderingRetirementControl): Promise<void> {
    try {
        if (preparing || activeRequests) throw new Error('cem.runtime.retirement.busy');
        let confirmation: unknown;
        if (request.action === 'checkpoint') {
            if (prepared || !request.input) throw new Error('cem.runtime.retirement.unavailable');
            preparing = true;
            try { confirmation = await processCheckpointControl(request.input); }
            finally { preparing = false; }
        } else if (request.action === 'inspect') {
            if (nativeClosed || prepared) throw new Error('cem.runtime.allocation.closed');
            preparing = true;
            try {
                await ensureRuntimeReady();
                const text = runtimeMemoryStatus();
                if (text.length > 2048) throw new Error('cem.runtime.allocation.headroom');
                const status = JSON.parse(text);
                if (status.softPressure !== false || status.hardPressure !== false) throw new Error('cem.runtime.allocation.headroom');
                confirmation = { softPressure: false, hardPressure: false };
            } finally { preparing = false; }
        } else if (request.action === 'close') {
            preparing = true;
            try { await ensureRuntimeReady(); closeRenderingNativeAdmission(); nativeClosed = true; }
            finally { preparing = false; }
        } else if (request.action === 'prepare') {
            if (!nativeClosed || prepared) throw new Error('cem.runtime.retirement.unavailable');
            prepared = true;
            engine.dispose({ reason: 'runtime-disposed' });
            // The existing complete linked native census refuses unmanaged roots
            // even after every managed render resource has been released.
            const inventory = renderingNativeResourceInventory();
            if (inventory.length > 65536) throw new Error('cem.runtime.retirement.inventory');
            const revision = JSON.parse(inventory).revision;
            permit = prepareRenderingNativeRetirement(revision);
            const text = permit.confirmation();
            if (text.length > 512) throw new Error('cem.runtime.retirement.confirmation');
            confirmation = JSON.parse(text);
        } else {
            permit?.free(); permit = undefined;
        }
        workerScope.postMessage({ protocol: CEM_RENDERING_RETIREMENT_PROTOCOL, direction: 'response',
            id: request.id, action: request.action, ok: true, ...(confirmation ? { confirmation } : {}) });
    } catch (error) {
        workerScope.postMessage({ protocol: CEM_RENDERING_RETIREMENT_PROTOCOL, direction: 'response',
            id: request.id, action: request.action, ok: false, error: String(error).slice(0, 512) });
    }
}
