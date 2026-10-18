import type { CemProducedElementBehavior } from './cem-elements.js';
import { mountCemLivePreview, type CemLivePreview, type CemLivePreviewResources } from './live-preview.js';

const states = new WeakMap<HTMLElement, { key: string; frame: HTMLIFrameElement; controller?: CemLivePreview }>();
export const CEM_LIVE_PREVIEW_CAPABILITY: CemProducedElementBehavior = {
    rendered(instance, context) {
        const previous = states.get(instance);
        const frame = instance.querySelector<HTMLIFrameElement>('iframe[part~="live-document"]');
        if (instance.getAttribute('mode') !== 'live' || !frame) {
            previous?.controller?.dispose(); states.delete(instance); return;
        }
        const source = instance.getAttribute('value') ?? '', sourceUri = instance.getAttribute('source-uri') ?? '';
        const envelope = instance.getAttribute('resource-set') ?? '{"resources":[]}';
        const key = JSON.stringify([source, sourceUri, envelope]);
        if (previous?.key === key && previous.frame === frame) return;
        previous?.controller?.dispose();
        const state: { key: string; frame: HTMLIFrameElement; controller?: CemLivePreview } = { key, frame };
        states.set(instance, state);
        context.setSlices({ previewStatus: 'Loading live view…' });
        try {
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
