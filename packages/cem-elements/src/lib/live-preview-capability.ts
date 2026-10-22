import type { CemCheckpointRuntimeState } from './checkpoint-controls.js';
import type { CemProducedElementBehavior } from './cem-elements.js';
import { mountCemLivePreview, type CemLivePreview, type CemLivePreviewResources } from './live-preview.js';
import { mountCemArtifactPreview, type CemArtifactPreview } from './artifact-preview.js';

type Context = Parameters<NonNullable<CemProducedElementBehavior['rendered']>>[1];
interface State { key: string; frame: HTMLIFrameElement; context: Context; controller?: CemLivePreview | CemArtifactPreview }
const states = new WeakMap<HTMLElement, State>();
type StaticMode = 'document' | 'text' | 'media' | 'message';
interface Presentation { context: Context; mode: StaticMode; root: Element; frame?: HTMLIFrameElement; ready: boolean; srcdoc?: string; revision: number; stop(): void }
const presentations = new WeakMap<HTMLElement, Presentation>();
const staticMode = (mode: string | null): mode is StaticMode => ['document', 'text', 'media', 'message'].includes(mode ?? '');
function presentationGuard(instance: HTMLElement): (() => boolean) | undefined {
    const state = presentations.get(instance);
    if (!state?.ready || !instance.contains(state.root)) return undefined;
    const revision = state.revision, frame = state.frame;
    if (frame && (!state.root.contains(frame) || frame.getAttribute('sandbox') !== '' || frame.getAttribute('allow') !== ''
        || frame.getAttribute('referrerpolicy') !== 'no-referrer' || frame.hasAttribute('src') || frame.srcdoc !== state.srcdoc)) return undefined;
    // Retain browser objects in place, including opaque sandbox documents. No
    // DOM serialization grants frame access or imports presentation as data.
    const nodes: Node[] = [], walker = instance.ownerDocument.createTreeWalker(state.root, 0xffffffff);
    nodes.push(state.root);
    while (walker.nextNode()) { if (nodes.length >= 4096) return undefined; nodes.push(walker.currentNode); }
    const attributes = (node: Node) => node.nodeType === 1
        ? JSON.stringify([...(node as Element).attributes].map(a => [a.namespaceURI, a.name, a.value])) : node.nodeValue;
    const snapshots = nodes.map(node => ({ node, parent: node.parentNode, next: node.nextSibling, first: node.firstChild, value: attributes(node) }));
    return () => instance.isConnected && presentations.get(instance) === state && state.ready && state.revision === revision
        && instance.getAttribute('mode') === state.mode && instance.contains(state.root)
        && snapshots.every(s => s.node.parentNode === s.parent && s.node.nextSibling === s.next && s.node.firstChild === s.first && attributes(s.node) === s.value);
}
function retainPresentation(instance: HTMLElement, context: Context, mode: StaticMode): void {
    const root = instance.querySelector('[part~="root"]'), previous = presentations.get(instance);
    const frame = mode === 'document' ? instance.querySelector<HTMLIFrameElement>('iframe[part~="document"]') ?? undefined : undefined;
    if (previous?.root === root && previous.mode === mode && previous.frame === frame && previous.srcdoc === frame?.srcdoc) { previous.context = context; return; }
    previous?.stop(); presentations.delete(instance);
    if (!root || mode === 'document' && !frame) return;
    const state: Presentation = { context, mode, root, frame, ready: !frame, srcdoc: frame?.srcdoc, revision: 0, stop() { frame?.removeEventListener('load', loaded); } };
    const loaded = (event: Event) => { if (!event.isTrusted) return; state.ready = true; state.revision++; state.context.runtime.checkpointInteractionSettled(instance); };
    frame?.addEventListener('load', loaded); presentations.set(instance, state);
}
/** Browser-only retained controller intent. Native checkpoint validates the scalar policy. */
export function cemMediaPreviewCheckpoint(instance: HTMLElement): CemCheckpointRuntimeState['capability'] | undefined {
    const state = states.get(instance), mode = instance.getAttribute('mode');
    if (staticMode(mode)) return presentationGuard(instance)?.() ? { kind: 'media-preview', mode, disposition: 'retain-presentation', revision: presentations.get(instance)!.revision } : undefined;
    if (!state?.controller?.checkpoint()?.() || !instance.contains(state.frame) || !['live', 'artifact-tree'].includes(mode ?? '')) return undefined;
    return { kind: 'media-preview', mode: mode as 'live' | 'artifact-tree', disposition: 'retain-controller' };
}
export function prepareCemMediaPreviewGuard(instance: HTMLElement): (() => boolean) | undefined {
    if (staticMode(instance.getAttribute('mode'))) return presentationGuard(instance);
    const state = states.get(instance), witness = state?.controller?.checkpoint();
    if (!state || !witness || !witness()) return undefined;
    return () => instance.isConnected && states.get(instance) === state && instance.contains(state.frame) && witness();
}
export const CEM_LIVE_PREVIEW_CAPABILITY: CemProducedElementBehavior = {
    rendered(instance, context) {
        const previous = states.get(instance), mode = instance.getAttribute('mode');
        if (staticMode(mode)) {
            previous?.controller?.dispose(); states.delete(instance); retainPresentation(instance, context, mode); return;
        }
        presentations.get(instance)?.stop(); presentations.delete(instance);
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
    disconnected(instance) { states.get(instance)?.controller?.dispose(); states.delete(instance); presentations.get(instance)?.stop(); presentations.delete(instance); },
};
