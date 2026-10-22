import { checkpointXmlSource, type CheckpointDatalist, type CheckpointInactiveSuggestions, type CheckpointSource } from './checkpoint-interactions.js';
import { registerCemEditorCheckpoint, subscribeCemEditorCheckpoint } from './form-control-capability.js';
import type { DataIslandSnapshot, CemProducedElementBehaviorContext } from './cem-elements.js';
import type { CemLocalSuggestionsEnvironment } from './suggestions-capability.js';
import { captureLocalSuggestions, localSuggestionsNamespaceStamp } from './local-suggestions-capture.js';
import { CemNativeCapabilitySession } from './native-capability-session.js';
import type { CemNativeSuggestionsBinding, CemNativeSuggestionsPublication } from './native-suggestions-publication.js';
import { resolveCemSuggestionsEditor } from './suggestions-editor.js';
import { CemSuggestionsPlacementCoordinator, type CemSuggestionsPlacementLease } from './suggestions-placements.js';
import { reportInteractionReference } from './interaction-reference.js';

export interface CemNativeDatalistAttachment {
    current(): boolean;
    park(): () => void;
    ready(): Promise<void>;
    checkpoint(): CheckpointDatalist | undefined;
    editorHost: HTMLElement;
    readonly failed: boolean;
    reconcile(): void;
    bind(snapshot: DataIslandSnapshot): CemNativeSuggestionsBinding | undefined;
    release(): void;
}
export function nativeDatalistConfiguration(instance: HTMLElement): string {
    return JSON.stringify(['profile', 'options-state', 'options-revision', 'options-error', 'options', 'editor-for', 'filter', 'filter-by',
        'require-selection', 'selection-message', 'options-query-revision', 'options-policy']
        .map(name => instance.getAttribute(name)));
}
function localDatalistInputs(instance: HTMLElement, environment: CemLocalSuggestionsEnvironment) {
    const endpoint = resolveCemSuggestionsEditor(instance, 'native-datalist');
    if (endpoint.code || !endpoint.host || !endpoint.provider || !environment.ownsEditor(endpoint.host)) throw new Error('Native datalist requires a local original field');
    const { host: editorHost, provider } = endpoint;
    const surfaces = [...instance.children].filter(node => node.getAttribute('part')?.split(/\s+/).includes('datalist'));
    const templates = [...instance.children].filter(node => node.getAttribute('slot') === 'options');
    if (['options', 'editor-for', 'filter', 'filter-by', 'require-selection', 'selection-message', 'options-query-revision', 'options-policy'].some(name => instance.hasAttribute(name))
        || environment.snapshot.nativeAttributes?.some(attribute => ['options', 'editor-for'].includes(attribute.name))
        || instance.querySelector(':scope > [part~="surface"], :scope > [slot="option"], :scope > [slot="group-label"]')
        || environment.labelSources.length || templates.length > 1 || templates.length !== environment.optionsSources.length
        || templates.some(node => !(node instanceof HTMLTemplateElement)) || environment.optionsSources.some(node => !(node instanceof HTMLTemplateElement))
        || surfaces.length !== 1 || !(surfaces[0] instanceof HTMLDataListElement) || surfaces[0].hasAttribute('slot')) {
        throw new Error('Native datalist rejects custom filtering, selection, labels and surface configuration');
    }
    return { editorHost, provider, datalist: surfaces[0] as HTMLDataListElement, templates };
}
const inactiveSources = new WeakMap<HTMLElement, { identity: string; source: CheckpointSource }>();
/** A failed local source has no live picker/claim. Preserve original input, never retry it here. */
export function checkpointInactiveDatalist(instance: HTMLElement, context: CemProducedElementBehaviorContext, environment: CemLocalSuggestionsEnvironment): CheckpointInactiveSuggestions | undefined {
    if (!environment.current() || !context.runtime.localSuggestionsSourceCurrent(instance, environment.optionsSources[0] as HTMLTemplateElement | undefined, [])) return undefined;
    try {
        const { editorHost, provider } = localDatalistInputs(instance, environment);
        if (provider.composing || editorHost.contains(instance.ownerDocument.activeElement)) return undefined;
        const text = captureLocalSuggestions(instance.ownerDocument, environment.optionsSources[0] as HTMLTemplateElement | undefined, [], environment.limits);
        const identity = JSON.stringify([text, instance.baseURI, environment.snapshot.scopePolicyStamp]);
        let captured = inactiveSources.get(instance);
        if (captured?.identity !== identity) {
            const source = checkpointXmlSource(text, instance.baseURI, crypto.randomUUID(), 'seq:where(input.children.children, fn(n) => n.name == "options").children');
            if (!source) return undefined;
            captured = { identity, source }; inactiveSources.set(instance, captured);
        }
        if (!captured) return undefined;
        return { phase: 'inactive', profile: 'native-datalist', source: structuredClone(captured.source), providerRevision: provider.revision,
            queryRevision: 0, query: provider.control?.value ?? '', feedbackState: provider.editable ? 'failed' : 'unavailable', feedbackIntent: false, editable: provider.editable };
    } catch { return undefined; }
}
/** Local host policy admits original sources; the native owner renders options into the declaration. */
export function createLocalDatalist(instance: HTMLElement, context: CemProducedElementBehaviorContext,
    environment: CemLocalSuggestionsEnvironment, changed: () => void, ownerLost: () => void, resumeRevision?: string): CemNativeDatalistAttachment {
    const { editorHost, provider, datalist, templates } = localDatalistInputs(instance, environment);
    inactiveSources.delete(instance);
    const runtime = context.runtime;
    const producer = environment.snapshot.instanceId, editorProducer = (runtime.checkpointCaptureEnabled ? runtime.readInstanceSnapshot(editorHost) : runtime.snapshotInstance(editorHost)).instanceId;
    const source = environment.optionsSources[0] as HTMLTemplateElement | undefined;
    const configuration = nativeDatalistConfiguration(instance), captured = source ? [source] : [];
    const namespaceStamp = localSuggestionsNamespaceStamp(captured, environment.limits);
    const sourceText = captureLocalSuggestions(instance.ownerDocument, source, [], environment.limits);
    const checkpointOwner = {};
    const abort = new AbortController(), lease = provider.lease(checkpointOwner);
    const coordinator = new CemSuggestionsPlacementCoordinator(instance.getRootNode() as Document | ShadowRoot, environment.limits);
    let paused = false;
    let readyResolve!: () => void, readyReject!: (error: unknown) => void;
    const ready = new Promise<void>((resolve, reject) => { readyResolve = resolve; readyReject = reject; });
    void ready.catch(() => undefined);
    let session: CemNativeCapabilitySession | undefined, publication: CemNativeSuggestionsPublication | undefined;
    let released = false, failed = false, preparing = true, queued = false, editorPlacement: CemSuggestionsPlacementLease | undefined, targetPlacement: CemSuggestionsPlacementLease | undefined;
    const queue = () => {
        if (queued || released) return;
        queued = true; queueMicrotask(() => { queued = false; if (!released) {
            if (paused) return;
            attachment.reconcile();
            if (runtime.checkpointCaptureEnabled) { runtime.checkpointInteractionSettled(instance); runtime.checkpointInteractionSettled(editorHost); }
        } });
    };
    const sourceObserver = new MutationObserver(() => { attachment.release(); changed(); });
    if (environment.sourceRoot) sourceObserver.observe(environment.sourceRoot, { childList: true });
    if (source) {
        sourceObserver.observe(source, { attributes: true });
        sourceObserver.observe(source.content, { childList: true, subtree: true, attributes: true, characterData: true });
    }
    let sourceChanged = false;
    const current = () => {
        try {
            if (!sourceChanged && (sourceObserver.takeRecords().length || namespaceStamp !== localSuggestionsNamespaceStamp(captured, environment.limits))) {
                sourceChanged = true;
                queueMicrotask(() => { if (!released) { attachment.release(); changed(); } });
            }
            return !released && !abort.signal.aborted && lease.current && environment.current()
                && nativeDatalistConfiguration(instance) === configuration && (instance.getAttribute('options-state') ?? 'ready') === 'ready'
                && resolveCemSuggestionsEditor(instance, 'native-datalist').provider === provider
                && editorHost.parentElement === instance && datalist.parentElement === instance
                && !sourceChanged
                && runtime.localSuggestionsSourceCurrent(instance, source, []) && templates.every(node => node.parentElement === instance);
        } catch { return false; }
    };
    const stopProvider = subscribeCemEditorCheckpoint(provider, lease, queue);
    const attachment: CemNativeDatalistAttachment = {
        current, editorHost, ready: () => ready,
        park() { if (paused || !attachment.checkpoint()) throw Error('cem.checkpoint.resume.datalist'); paused = true;
            return () => { if (paused) { paused = false; attachment.reconcile(); } }; }, get failed() { return !released && failed; },
        checkpoint() {
            if (released || failed || preparing || queued || !current() || !lease.valid || provider.composing
                || editorHost.contains(instance.ownerDocument.activeElement)) return undefined;
            // Native picker visibility/selection is not observable: capture only the settled, unfocused model.
            const mapped = runtime.renderedSuggestionsFor(instance), source = session?.checkpointSource();
            const targetNode = datalist.getAttribute('data-cem-render-node-id');
            if (!source || !targetNode || !mapped?.current() || mapped.binding.config.profile !== 'native-datalist'
                || lease.attributes.valid || (provider.editable ? !lease.datalist.valid : lease.datalist.valid)) return undefined;
            return { phase: 'native-datalist', source, providerRevision: provider.revision, targetNode,
                claim: provider.editable ? 'attached' : 'unavailable', presentation: 'reset-on-resume' };
        },
        reconcile() {
            if (released || failed || paused) return;
            if (!current()) { attachment.release(); changed(); return; }
            if (preparing || !publication?.valid) return;
            coordinator.refresh();
            if (lease.datalist.valid) return;
            if (!lease.valid || !provider.editable) { reportInteractionReference(instance, 'suggestions-native-editor-unavailable', 'suggestions-binding'); return; }
            try {
                const revision = String(provider.revision);
                if (!editorPlacement) editorPlacement = coordinator.register({ producer: editorProducer, revision, current, kind: 'editor', element: editorHost });
                if (!targetPlacement) {
                    targetPlacement = coordinator.register({ producer, revision, current: () => current() && publication?.valid === true, kind: 'datalist', element: datalist });
                    coordinator.grant(producer, editorPlacement, ['editor-for']); coordinator.grant(editorProducer, targetPlacement, ['list']);
                }
                const placements = coordinator.prepareDatalist(editorPlacement, targetPlacement, lease);
                if (!lease.datalist.set({ revision: provider.revision, current, placements })) { placements.release(); throw new Error('Native list relationship conflicts'); }
                reportInteractionReference(instance, undefined, 'suggestions-binding');
            } catch { reportInteractionReference(instance, 'suggestions-native-claim-conflict', 'suggestions-binding'); }
        },
        bind(snapshot) {
            if (!current() || !publication?.valid) return undefined;
            return publication.bind({ instanceId: snapshot.instanceId, scopePolicyStamp: snapshot.scopePolicyStamp,
                revision: snapshot.dataRevision, current });
        },
        release() {
            if (released) return; released = true; abort.abort(); sourceObserver.disconnect(); stopProvider(); stopOwner?.();
            stopCheckpoint(); lease.release(); coordinator.dispose(); void publication?.release().catch(() => undefined); void session?.release().catch(() => undefined);
            publication = undefined;
        },
    };
    const stopCheckpoint = registerCemEditorCheckpoint(provider, { host: instance, owner: checkpointOwner, lease,
        current: () => !!attachment.checkpoint(), claim: () => attachment.checkpoint()?.claim === 'attached' ? 'datalist' : attachment.checkpoint() ? 'none' : undefined });
    const stopOwner = environment.owner.onNativeAuthorityLost?.(() => { if (paused) return; attachment.release(); ownerLost();
        reportInteractionReference(instance, 'suggestions-native-owner-lost', 'suggestions-binding'); });
    // Wait until the caller installs this attachment before submitting its native frame.
    queueMicrotask(() => { void (async () => {
        try {
            const bytes = new TextEncoder().encode(sourceText);
            session = await CemNativeCapabilitySession.prepare(environment.owner, { action: 'prepare', adapter: 'native-datalist-v1',
                handle: { sessionKey: crypto.randomUUID(), instanceId: environment.snapshot.instanceId,
                    scopePolicyStamp: environment.snapshot.scopePolicyStamp, sourceRevision: resumeRevision ?? crypto.randomUUID() },
                sources: { kind: 'cem-native-session-import-v1', bytes: bytes.buffer as ArrayBuffer, contentType: 'application/xml', sourceUri: instance.baseURI },
                data: {}, select: 'seq:where(input.children.children, fn(n) => n.name == "options").children', limits: environment.limits }, current, abort.signal, runtime.checkpointCaptureEnabled);
            publication = await session.publishDatalist({}, current);
            if (!current()) throw new Error('Native datalist publication superseded');
            runtime.refreshElementReferences(instance); await runtime.whenRenderSettled(instance);
            if (!current() || runtime.diagnosticsFor(instance).some(d => d.severity === 'error' || d.severity === 'fatal')
                || runtime.renderedSuggestionsFor(instance)?.binding.config.profile !== 'native-datalist') throw new Error('Native datalist frame superseded or failed');
            preparing = false; attachment.reconcile(); readyResolve();
            if (runtime.checkpointCaptureEnabled) { runtime.checkpointInteractionSettled(instance); runtime.checkpointInteractionSettled(editorHost); }
        } catch (error) {
            readyReject(error);
            if (released) return;
            failed = true; stopProvider(); stopCheckpoint(); lease.release(); coordinator.dispose(); void publication?.release().catch(() => undefined);
            void session?.release().catch(() => undefined); publication = undefined;
            reportInteractionReference(instance, 'suggestions-native-source-unavailable', 'suggestions-binding');
            if (runtime.checkpointCaptureEnabled) { runtime.checkpointInteractionSettled(instance); runtime.checkpointInteractionSettled(editorHost); }
            // No automatic retry of an unchanged invalid source. A source/configuration
            // mutation drives fresh preparation through the outer lifecycle.
        }
    })(); });
    return attachment;
}
