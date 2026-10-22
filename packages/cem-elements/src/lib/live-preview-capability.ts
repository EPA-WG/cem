import type { CemCheckpointRuntimeState } from './checkpoint-controls.js';
import type { CemProducedElementBehavior } from './cem-elements.js';
import { mountCemLivePreview, type CemLivePreview, type CemLivePreviewResources } from './live-preview.js';
import { mountCemArtifactPreview, type CemArtifactPreview } from './artifact-preview.js';

type Context = Parameters<NonNullable<CemProducedElementBehavior['rendered']>>[1];
interface State { key: string; frame: HTMLIFrameElement; context: Context; controller?: CemLivePreview | CemArtifactPreview }
const states = new WeakMap<HTMLElement, State>();
/** Browser-only retained controller intent. Native checkpoint validates the scalar policy. */
export function cemMediaPreviewCheckpoint(instance: HTMLElement): CemCheckpointRuntimeState['capability'] | undefined {
    const state = states.get(instance), mode = instance.getAttribute('mode');
    if (!state?.controller?.checkpoint()?.() || !instance.contains(state.frame) || !['live', 'artifact-tree'].includes(mode ?? '')) return undefined;
    return { kind: 'media-preview', mode: mode as 'live' | 'artifact-tree', disposition: 'retain-controller' };
}
export function prepareCemMediaPreviewGuard(instance: HTMLElement): (() => boolean) | undefined {
    const state = states.get(instance), witness = state?.controller?.checkpoint();
    if (!state || !witness || !witness()) return undefined;
    return () => instance.isConnected && states.get(instance) === state && instance.contains(state.frame) && witness();
}
export const CEM_LIVE_PREVIEW_CAPABILITY: CemProducedElementBehavior = {
    rendered(instance, context) {
        const previous = states.get(instance);
        const artifactMode = instance.getAttribute('mode') === 'artifact-tree';
        const frame = instance.querySelector<HTMLIFrameElement>('iframe[part~="live-document"], iframe[part~="artifact-document"]');
        if (!['live', 'artifact-tree'].includes(instance.getAttribute('mode') ?? '') || !frame) {
            previous?.controller?.dispose(); states.delete(instance); return;
        }
        const source = instance.getAttribute('value') ?? '', sourceUri = instance.getAttribute('source-uri') ?? '';
        const envelope = instance.getAttribute('resource-set') ?? '{"resources":[]}';
        const path = instance.getAttribute('artifact-path') ?? '';
        const key = JSON.stringify(artifactMode ? ['artifact', envelope, path.split(/[?#]/)[0]] : ['live', source, sourceUri, envelope]);
        if (previous?.key === key && previous.frame === frame) {
            previous.context = context;
            if (artifactMode && previous.controller && 'setLocation' in previous.controller) previous.controller.setLocation(path);
            return;
        }
        previous?.controller?.dispose();
        const state: State = { key, frame, context };
        states.set(instance, state);
        state.context.setSlices({ previewStatus: artifactMode ? 'Loading artifact view…' : 'Loading live view…' });
        try {
            if (artifactMode) {
                state.controller = mountCemArtifactPreview(frame, { resourceSet: envelope, path,
                    onStatus(message) { if (states.get(instance) === state) state.context.setSlices({ previewStatus: message }); },
                    onNavigate(value) { if (states.get(instance) === state) instance.dispatchEvent(new CustomEvent('cem-preview-navigate', { bubbles: true, composed: true, detail: { value } })); },
                });
                void state.controller.ready.then(() => {
                    if (states.get(instance) === state) context.runtime.checkpointInteractionSettled(instance);
                }).catch(error => {
                    if (states.get(instance) === state) { state.controller?.dispose(); state.context.setSlices({ previewStatus: String(error) }); }
                });
                return;
            }
            const resources = JSON.parse(envelope) as CemLivePreviewResources;
            state.controller = mountCemLivePreview(frame, { ...resources, source, sourceUri,
                runtimeOptions: context.runtime.previewRuntimeOptions(), onError(message) {
                if (states.get(instance) === state) state.context.setSlices({ previewStatus: message });
            } });
            void state.controller.ready.then(() => {
                if (states.get(instance) === state) { state.context.setSlices({ previewStatus: 'Live view ready' }); context.runtime.checkpointInteractionSettled(instance); }
            }, error => {
                if (states.get(instance) === state) { state.controller?.dispose(); state.context.setSlices({ previewStatus: String(error) }); }
            });
        } catch (error) { state.context.setSlices({ previewStatus: String(error) }); }
    },
    preserveRenderedAttribute(instance, current, _desired, attribute) {
        return ['srcdoc', 'height'].includes(attribute.name) && states.get(instance)?.frame === current;
    },
    disconnected(instance) { states.get(instance)?.controller?.dispose(); states.delete(instance); },
};
