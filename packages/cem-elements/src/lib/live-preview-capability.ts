import type { CemProducedElementBehavior } from './cem-elements.js';
import { mountCemLivePreview, type CemLivePreview, type CemLivePreviewResources } from './live-preview.js';
import { mountCemArtifactPreview, type CemArtifactPreview } from './artifact-preview.js';

const states = new WeakMap<HTMLElement, { key: string; frame: HTMLIFrameElement; controller?: CemLivePreview | CemArtifactPreview }>();
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
            if (artifactMode && previous.controller && 'setLocation' in previous.controller) previous.controller.setLocation(path);
            return;
        }
        previous?.controller?.dispose();
        const state: { key: string; frame: HTMLIFrameElement; controller?: CemLivePreview | CemArtifactPreview } = { key, frame };
        states.set(instance, state);
        context.setSlices({ previewStatus: artifactMode ? 'Loading artifact view…' : 'Loading live view…' });
        try {
            if (artifactMode) {
                state.controller = mountCemArtifactPreview(frame, { resourceSet: envelope, path,
                    onStatus(message) { if (states.get(instance) === state) context.setSlices({ previewStatus: message }); },
                    onNavigate(value) { if (states.get(instance) === state) instance.dispatchEvent(new CustomEvent('cem-preview-navigate', { bubbles: true, composed: true, detail: { value } })); },
                });
                void state.controller.ready.catch(error => {
                    if (states.get(instance) === state) { state.controller?.dispose(); context.setSlices({ previewStatus: String(error) }); }
                });
                return;
            }
            const resources = JSON.parse(envelope) as CemLivePreviewResources;
            state.controller = mountCemLivePreview(frame, { ...resources, source, sourceUri, onError(message) {
                if (states.get(instance) === state) context.setSlices({ previewStatus: message });
            } });
            void state.controller.ready.then(() => {
                if (states.get(instance) === state) context.setSlices({ previewStatus: 'Live view ready' });
            }, error => {
                if (states.get(instance) === state) { state.controller?.dispose(); context.setSlices({ previewStatus: String(error) }); }
            });
        } catch (error) { context.setSlices({ previewStatus: String(error) }); }
    },
    preserveRenderedAttribute(instance, current, _desired, attribute) {
        return ['srcdoc', 'height'].includes(attribute.name) && states.get(instance)?.frame === current;
    },
    disconnected(instance) { states.get(instance)?.controller?.dispose(); states.delete(instance); },
};
