import type { CemProducedElementBehavior, CemProducedElementBehaviorContext } from './cem-elements.js';
import { connectCemSuggestionsController, type CemSuggestionsController, type CemSuggestionsControllerOptions } from './suggestions-controller.js';
import { resolveCemSuggestionsEditor } from './suggestions-editor.js';
import { observeInteractionReferences, reportInteractionReference } from './interaction-reference.js';

interface State {
    context: CemProducedElementBehaviorContext;
    options?: CemSuggestionsControllerOptions;
    controller?: CemSuggestionsController;
    observer?: MutationObserver;
    stopReferences?: () => void;
    queued: boolean;
    configuration?: string;
}
const states = new WeakMap<HTMLElement, State>();
/** Package-private read-only view of the active shared controller. */
export function cemSuggestionsControllerFor(instance: HTMLElement): CemSuggestionsController | undefined { return states.get(instance)?.controller; }
function stop(state: State): void { state.controller?.disconnect(); state.controller = undefined; state.options = undefined; }
function synchronize(instance: HTMLElement, state: State): void {
    if (!instance.isConnected || !state.observer) return;
    const endpoint = resolveCemSuggestionsEditor(instance);
    const options = state.context.runtime.suggestionsControllerInputsFor(instance);
    if (endpoint.code || !options || endpoint.host !== options.editorHost || options.listbox.parentElement !== instance
        || !options.listbox.getAttribute('part')?.split(/\s+/).includes('surface') || options.listbox.hasAttribute('slot')) {
        stop(state); reportInteractionReference(instance, endpoint.code ?? 'suggestions-admission-required', 'suggestions-binding'); return;
    }
    if (state.options?.prepare !== options.prepare || state.options?.editorHost !== options.editorHost || state.options?.listbox !== options.listbox
        || state.options?.current !== options.current || state.options?.ancestors !== options.ancestors) {
        stop(state);
        const current = () => resolveCemSuggestionsEditor(instance).host === options.editorHost && (options.current?.() ?? true);
        try { state.controller = connectCemSuggestionsController(instance, { ...options, current }); state.options = options; }
        catch { reportInteractionReference(instance, 'suggestions-binding-unavailable', 'suggestions-binding'); return; }
    }
    reportInteractionReference(instance, undefined, 'suggestions-binding');
    const configuration = ['filter', 'filter-by', 'options-state', 'options-revision', 'options-query-revision', 'options-policy']
        .map(name => instance.getAttribute(name)).join('\u0000');
    if (state.configuration !== undefined && state.configuration !== configuration) void state.controller?.refresh();
    state.configuration = configuration;
    state.controller?.reconcile();
}

/** Declarative wiring uses trusted lifecycle preparation; attributes and slots alone issue no grants. */
export const CEM_SUGGESTIONS_CAPABILITY: CemProducedElementBehavior = {
    constructed(instance, context) { states.set(instance, { context, queued: false }); },
    connected(instance, context) {
        const state = states.get(instance); if (!state || state.observer) return;
        state.context = context;
        const queue = () => {
            if (state.queued) return;
            state.queued = true; queueMicrotask(() => { state.queued = false; synchronize(instance, state); });
        };
        state.observer = new MutationObserver(queue);
        state.observer.observe(instance, { childList: true, subtree: true, attributes: true,
            attributeFilter: ['require-selection', 'selection-message', 'filter', 'filter-by', 'options-state', 'options-revision', 'options-query-revision', 'options-policy'] });
        state.stopReferences = observeInteractionReferences(instance, queue);
    },
    rendered(instance, context) { const state = states.get(instance); if (state) { state.context = context; synchronize(instance, state); } },
    disconnected(instance) {
        const state = states.get(instance); if (!state) return;
        state.observer?.disconnect(); state.observer = undefined; state.stopReferences?.(); state.stopReferences = undefined;
        stop(state); state.configuration = undefined;
    },
};
