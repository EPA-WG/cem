import { parseCemStudioProjectBundle } from './file-system-provider.js';
import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';

const answerKeys = ['sourceFiles', 'targetFiles', 'sourceFolder', 'outputFolder', 'layoutFolder', 'cssFolder', 'staticFolder', 'inputFormat', 'outputFormat'];

/** Ephemeral creation transaction. Interpreting paths, producing stubs and
 * describing hierarchies belong exclusively to the shared native producer.
 * This service transports form values and publishes one reviewed archive. */
export function createCemStudioProjectBootstrap({ validator, repository, changed = () => undefined }) {
    let state, epoch = 0, controller, disposed = false;
    const publish = () => changed(state);
    const active = token => !disposed && state && token === epoch && !controller?.signal.aborted;
    async function operation(work) {
        if (!state || state.busy || disposed) return;
        const token = ++epoch;
        controller = new AbortController();
        const signal = controller.signal;
        state = { ...state, busy: true, error: undefined }; publish();
        const current = () => { signal.throwIfAborted(); if (!active(token)) throw new DOMException('Project creation was cancelled', 'AbortError'); };
        try { return await work(signal, current); }
        catch (error) {
            if (active(token) && error?.name !== 'AbortError') state = { ...state, error: { message: error.message, code: error.code, fieldPath: error.fieldPath } };
        } finally { if (active(token)) { state = { ...state, busy: false }; publish(); } }
    }
    const planRequest = answers => ({ action: 'plan', preset: state.preset, projectId: state.projectId,
        name: state.name, createdAt: state.createdAt, ...(answers && { answers }) });
    const cancel = () => { ++epoch; controller?.abort(); controller = undefined; state = undefined; publish(); };
    return Object.freeze({
        snapshot: () => state,
        async open() {
            if (disposed) return;
            cancel();
            state = { phase: 'type', name: 'New project', preset: 'markdown-site',
                projectId: `project-${crypto.randomUUID()}`, createdAt: new Date().toISOString(), presets: [], busy: false };
            return operation(async (signal, current) => {
                const catalog = await validator.bootstrapProject({ action: 'catalog' }, { signal }); current();
                state = { ...state, presets: catalog.presets };
            });
        },
        async choose(fields) {
            if (!state || state.busy) return;
            const name = fields.name?.[0]?.trim() ?? '';
            const preset = fields.preset?.[0];
            if (!name || name.length > 120) { state = { ...state, error: { message: 'Enter a project name of 1–120 characters.' } }; publish(); return; }
            const answers = preset === state.preset ? state.answers : undefined;
            state = { ...state, name, preset };
            return operation(async (signal, current) => {
                const planned = await validator.bootstrapProject(planRequest(answers), { signal }); current();
                state = { ...state, ...planned, phase: 'structure', resourceId: planned.configuration.resources[0]?.id };
            });
        },
        async review(fields) {
            if (!state || state.busy) return;
            // These are questionnaire control values, not source-document parsing.
            const answers = Object.fromEntries(answerKeys.filter(key => fields[key]).map(key => [key, fields[key][0]]));
            state = { ...state, answers: { ...state.answers, ...answers } };
            return operation(async (signal, current) => {
                const planned = await validator.bootstrapProject(planRequest(state.answers), { signal }); current();
                state = { ...state, ...planned, phase: 'review', resourceId: planned.configuration.resources[0]?.id };
            });
        },
        back(phase) { if (state && !state.busy && ['type', 'structure'].includes(phase)) { state = { ...state, phase, error: undefined }; publish(); } },
        inspect(resourceId) { if (state && !state.busy && state.configuration?.resources.some(r => r.id === resourceId)) { state = { ...state, resourceId }; publish(); } },
        async create() {
            if (!state || state.busy || state.phase !== 'review') return;
            return operation(async (signal, current) => {
                const configuration = structuredClone(state.configuration);
                const generated = await validator.bootstrapProject({ action: 'generate', configuration }, { signal }); current();
                const bundle = parseCemStudioProjectBundle(new TextEncoder().encode(JSON.stringify(generated.bundle)));
                const result = await repository.execute({ protocolVersion: 1, repository: CEM_STUDIO_REPOSITORY_ID, requestRevision: 1, operation: 'import-project', parameters: { bundle, mode: 'create' } }, signal);
                current();
                state = { ...state, phase: 'complete', project: result.value };
                return result.value;
            });
        },
        cancel,
        dispose() { cancel(); disposed = true; },
    });
}
