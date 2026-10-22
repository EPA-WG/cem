import type { CemProcessingHost } from './internal/runtime-support/processing-host.js';
import { registerCemProviderHandoff, validateCemProviderSource } from './checkpoint-provider-handoff.js';
import type { CemElementRuntime } from './cem-elements.js';
import type { CheckpointEndpoint, CheckpointStandaloneDatalist } from './checkpoint-interactions.js';
import { getCemEditorProvider, registerCemEditorCheckpoint, subscribeCemEditorCheckpoint } from './form-control-capability.js';
import { checkpointRenderedDatalistOrigin } from './projection.js';
import { CemSuggestionsPlacementCoordinator, isCemDatalistInput } from './suggestions-placements.js';

export interface CemStandaloneDatalistLifecycle {
    current(): boolean;
    ready(): boolean;
    checkpoint?: boolean;
    /** Trusted original source policy, invoked only after every owner is bound. */
    resume?(host: CemProcessingHost): Promise<CemStandaloneDatalistLifecycle>;
    subscribe(invalidate: () => void): () => void;
}
interface Options {
    runtime: CemElementRuntime;
    host: HTMLElement;
    editorHost: HTMLElement;
    target: HTMLDataListElement;
    lifecycle: CemStandaloneDatalistLifecycle;
}
export interface CemStandaloneDatalistAttachment { reconcile(): void; disconnect(): void }
const delegates = new WeakMap<HTMLElement, { editor: HTMLElement; target: HTMLDataListElement; capture(): Omit<CheckpointStandaloneDatalist, 'editor' | 'target'> | undefined }>();
export function hasCemStandaloneDatalistCheckpoint(host: HTMLElement): boolean { return delegates.has(host); }
export function cemStandaloneDatalistCheckpoint(host: HTMLElement, resolve: (node: HTMLElement) => CheckpointEndpoint | undefined): CheckpointStandaloneDatalist | undefined {
    const delegate = delegates.get(host), state = delegate?.capture();
    const editor = delegate && resolve(delegate.editor), target = delegate && resolve(delegate.target);
    return state && editor && target ? { ...state, editor, target } : undefined;
}
/** Trusted host attachment to a committed native datalist frame. Owns only the list relationship. */
export function connectCemStandaloneDatalist({ runtime, host, editorHost, target, lifecycle }: Options): CemStandaloneDatalistAttachment {
    if (delegates.has(host)) throw new TypeError('Duplicate standalone datalist checkpoint owner');
    const provider = getCemEditorProvider(editorHost), editor = provider?.control;
    if (!provider || !isCemDatalistInput(editor) || !host.isConnected || !editorHost.isConnected || !host.contains(target) || host === editorHost) throw new TypeError('Standalone datalist requires an exact native editor and owned target');
    const producer = runtime.readInstanceSnapshot(host).instanceId, editorProducer = runtime.readInstanceSnapshot(editorHost).instanceId;
    const owner = {}, lease = provider.lease(owner);
    const coordinator = new CemSuggestionsPlacementCoordinator(host.getRootNode() as Document | ShadowRoot);
    let disposed = false, queued = false, paused = false;
    let stopLifecycle: (() => void) | undefined;
    const current = () => !disposed && host.isConnected && editorHost.isConnected && target.isConnected && host.contains(target)
        && host.getRootNode() === editorHost.getRootNode() && lease.valid && getCemEditorProvider(editorHost) === provider
        && provider.control === editor && isCemDatalistInput(editor) && !provider.composing && lifecycle.current() && lifecycle.ready();
    const frame = () => {
        if (!current()) return undefined;
        const mapped = runtime.renderedSuggestionsFor(host), origin = checkpointRenderedDatalistOrigin(target);
        const snapshot = runtime.readInstanceSnapshot(host);
        return mapped?.binding.config.profile === 'native-datalist' && origin?.instanceId === snapshot.instanceId
            // Unchanged DOM retains its original revision; mapped.current() proves the latest committed frame.
            && origin.templateArtifactId === snapshot.templateArtifactId
            && origin.scopePolicyStamp === snapshot.scopePolicyStamp ? mapped : undefined;
    };
    let editorPlacement: ReturnType<typeof coordinator.register>, targetPlacement: ReturnType<typeof coordinator.register>;
    try {
        editorPlacement = coordinator.register({ producer: editorProducer, revision: 'current', current, kind: 'editor', element: editorHost });
        targetPlacement = coordinator.register({ producer, revision: 'current', current: () => !!frame(), kind: 'datalist', element: target });
        coordinator.grant(editorProducer, targetPlacement, ['list']); coordinator.grant(producer, editorPlacement, ['editor-for']);
    } catch (error) { coordinator.dispose(); lease.release(); throw error; }
    const settled = () => { runtime.checkpointInteractionSettled(host); runtime.checkpointInteractionSettled(editorHost); };
    const reconcile = () => {
        if (disposed || paused) return;
        coordinator.refresh();
        if (!frame() || !provider.editable) { lease.datalist.clear(); return; }
        if (lease.datalist.valid || lease.attributes.valid) return;
        try {
            const placements = coordinator.prepareDatalist(editorPlacement, targetPlacement, lease);
            if (!lease.datalist.set({ revision: provider.revision, current: () => !!frame(), placements })) placements.release();
        } catch { /* Conflicting or authored relationships are preserved and block capture. */ }
    };
    const queue = () => {
        if (disposed || queued || paused) return;
        queued = true; queueMicrotask(() => { queued = false; if (!disposed) { reconcile(); settled(); } });
    };
    const releases: (() => void)[] = [];
    const observer = new MutationObserver(queue);
    const abort = new AbortController();
    try {
        releases.push(subscribeCemEditorCheckpoint(provider, lease, queue));
        stopLifecycle = lifecycle.subscribe(queue); releases.push(() => stopLifecycle?.());
        observer.observe(target, { attributes: true, childList: true, subtree: true, characterData: true });
        editor.addEventListener('focus', queue, { signal: abort.signal }); editor.addEventListener('blur', queue, { signal: abort.signal });
        if (lifecycle.checkpoint) {
            const capture = () => {
                const mapped = frame(), source = mapped?.binding.checkpointSource();
                if (lifecycle.checkpoint !== true || queued || !mapped || !source || lease.attributes.valid
                    || editorHost.contains(host.ownerDocument.activeElement) || editorHost.hasAttribute('list')
                    || (provider.editable ? !lease.datalist.valid || editor.list !== target : lease.datalist.valid || editor.hasAttribute('list'))
                    || target.options.length !== source.rowCount) return undefined;
                return { source, providerRevision: provider.revision, claim: provider.editable ? 'attached' as const : 'unavailable' as const,
                    presentation: 'reset-on-resume' as const, disposition: 'reacquire' as const };
            };
            const record = { editor: editorHost, target, capture }; delegates.set(host, record);
            releases.push(() => { if (delegates.get(host) === record) delegates.delete(host); });
            releases.push(registerCemEditorCheckpoint(provider, { host, owner, lease, current: () => !!capture(),
                claim: () => capture()?.claim === 'attached' ? 'datalist' : capture() ? 'none' : undefined }));
            releases.push(registerCemProviderHandoff(host, () => {
                const captured = capture(), renew = lifecycle.resume?.bind(lifecycle);
                if (!captured || !renew) return undefined;
                return { editor: editorHost, current: () => !disposed && (paused || !!capture()), park() {
                    if (!capture() || paused) throw Error('cem.checkpoint.resume.datalist');
                    paused = true;
                    return () => { if (paused) { paused = false; reconcile(); } };
                }, async renew(freshHost, saved) {
                    if (!paused || !saved.standaloneDatalist || saved.standaloneDatalist.providerRevision !== provider.revision)
                        throw Error('cem.checkpoint.resume.datalist');
                    const next = await renew(freshHost);
                    if (!next || next.checkpoint !== true || !next.current() || !next.ready()) throw Error('cem.checkpoint.resume.source-authority');
                    stopLifecycle?.(); lifecycle = next; stopLifecycle = lifecycle.subscribe(queue);
                    runtime.refreshElementReferences(host); await runtime.whenRenderSettled(host);
                    await validateCemProviderSource(freshHost, runtime.renderedSuggestionsFor(host)?.binding.checkpointSource(), saved.standaloneDatalist.source);
                    lease.datalist.clear(); paused = false; reconcile();
                    if (!capture()) throw Error('cem.checkpoint.resume.datalist');
                } };
            }));
        }
        reconcile(); settled();
    } catch (error) {
        disposed = true; observer.disconnect(); abort.abort(); for (const release of releases) release(); coordinator.dispose(); lease.release(); throw error;
    }
    return { reconcile() { reconcile(); settled(); }, disconnect() {
        if (disposed) return;
        disposed = true; observer.disconnect(); abort.abort(); for (const release of releases) release();
        coordinator.dispose(); lease.release(); settled();
    } };
}
