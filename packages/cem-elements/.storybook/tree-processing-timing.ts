import type { CemElementRuntimeOptions } from '../src/index.js';
import {
    defaultCemProcessingWorkerFactory,
    type CemProcessingEnvelope,
    type CemProcessingRequestEnvelope,
    type CemProcessingWorkerFactory,
} from '../src/lib/internal/runtime-support/processing-host.js';

const enabled = import.meta.env.STORYBOOK_CEM_TREE_TRACE === '1';
let active = false;
let started = 0;
let viewer: HTMLElement | undefined;
let disconnect: (() => void) | undefined;
let story = 'tree/EditingSelectionAndDisclosure';

// Explicit diagnostic control records only. Never serialize source documents,
// native artifacts, render subtrees or the complete runtime snapshot.
function emit(event: string, detail: object = {}): void {
    if (!active) return;
    const now = performance.now();
    console.warn('[cem-tree-processing]', JSON.stringify({
        event, story, at: new Date(performance.timeOrigin + now).toISOString(),
        elapsedMs: Math.round((now - started) * 10) / 10,
        ...detail,
    }));
}

function viewerState(): object {
    const article = viewer?.querySelector('article');
    return {
        connected: viewer?.isConnected ?? false,
        scopeUid: article?.getAttribute('data-cem-render-scope'),
        revision: article?.getAttribute('data-cem-data-revision'),
        selected: viewer?.querySelector('output[aria-label="Selected branches"]')?.textContent,
        checked: Array.from(viewer?.querySelectorAll<HTMLInputElement>('input:checked') ?? [],
            input => input.getAttribute('aria-label')),
    };
}

const workerFactory: CemProcessingWorkerFactory = input => {
    const worker = defaultCemProcessingWorkerFactory(input);
    const send = worker.postMessage.bind(worker);
    const jobs = new Map<number, object>();
    emit('worker-created', { worker: input.name });
    worker.postMessage = (message: CemProcessingRequestEnvelope, options?: StructuredSerializeOptions | Transferable[]) => {
        const detail = message.operation === 'render-diff' ? {
            tag: message.payload.snapshot.producedTag,
            revision: message.payload.revision,
            scopeUid: message.payload.scopeUid,
            branchSlices: Object.fromEntries(Object.entries(message.payload.snapshot.slices)
                .filter(([key]) => key.startsWith('branch.'))
                .map(([key, value]) => [key, typeof value === 'string' || typeof value === 'boolean' || value === null
                    ? value : '[non-scalar]'])),
        } : message.operation === 'compile' ? {
            tag: message.payload.producedTag,
            artifact: message.payload.templateArtifactId,
        } : message.operation === 'cancel' ? message.payload : {};
        jobs.set(message.jobId, { story, ...detail });
        emit('worker-send', { worker: input.name, jobId: message.jobId, operation: message.operation, ...detail });
        if (Array.isArray(options)) send(message, options);
        else send(message, options);
    };
    worker.addEventListener('message', ({ data }: MessageEvent<CemProcessingEnvelope>) => {
        if (data.direction === 'ready') {
            emit('worker-ready', { worker: input.name });
            return;
        }
        if (data.direction !== 'response') return;
        const detail = jobs.get(data.jobId);
        jobs.delete(data.jobId);
        const result = data.outcome === 'success' && data.operation === 'render-diff' ? {
            revision: data.result.revision,
            frames: data.result.frames.length,
            patchOperations: data.result.frames.reduce((count, frame) =>
                count + (frame.type === 'ops' ? frame.ops.length : 0), 0),
            diagnostics: data.result.diagnostics.map(item => item.code),
        } : data.outcome !== 'success' ? { diagnostics: data.diagnostics.map(item => item.code) } : {};
        emit('worker-response', {
            worker: input.name, jobId: data.jobId, operation: data.operation, outcome: data.outcome,
            ...detail, ...result,
        });
    });
    return worker;
};

export const treeProcessingTimingOptions: Pick<CemElementRuntimeOptions, 'processingWorkerFactory' | 'onProcessingTrace'> = enabled ? {
    processingWorkerFactory: workerFactory,
    onProcessingTrace: event => emit('schedule', event),
} : {};

/** Start before mounting the existing authored page; stop at its actual removal. */
export function startTreeProcessingTiming(root: HTMLElement): void {
    startProcessingTiming(root, 'tree/EditingSelectionAndDisclosure');
}

/** Shared opt-in transport tracing; never changes worker scheduling or inputs. */
export function startProcessingTiming(root: HTMLElement, label: string): void {
    if (!enabled) return;
    disconnect?.();
    viewer = undefined;
    story = label;
    started = performance.now();
    active = true;
    let mounted = false;
    let previous = '';
    const observer = new MutationObserver(() => {
        mounted ||= root.isConnected;
        const state = viewerState();
        const serialized = JSON.stringify(state);
        if (viewer && serialized !== previous) {
            previous = serialized;
            emit('dom-state', state);
        }
        if (mounted && !root.isConnected) {
            emit('root-detached', state);
            disconnect?.();
        }
    });
    observer.observe(root.ownerDocument, { subtree: true, childList: true, attributes: true, characterData: true });
    const onInput = (event: Event): void => {
        if (!(event.target instanceof HTMLInputElement) || !viewer?.contains(event.target)) return;
        emit(`input-${event.type}`, {
            label: event.target.getAttribute('aria-label'), checked: event.target.checked,
            state: viewerState(),
        });
    };
    root.addEventListener('click', onInput, true);
    root.addEventListener('change', onInput, true);
    disconnect = () => {
        observer.disconnect();
        root.removeEventListener('click', onInput, true);
        root.removeEventListener('change', onInput, true);
        active = false;
        disconnect = undefined;
    };
    emit('start');
}

/** Observe the XML viewer without asking the runtime to create a new snapshot/revision. */
export function watchTreeSelection(instance: HTMLElement): void {
    if (!active) return;
    viewer = instance;
    emit('watch-viewer', viewerState());
}
