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
    reconcile(): void;
    bind(snapshot: DataIslandSnapshot): CemNativeSuggestionsBinding | undefined;
    release(): void;
}
export function nativeDatalistConfiguration(instance: HTMLElement): string {
    return JSON.stringify(['profile', 'options-state', 'options-revision', 'options-error', 'options', 'editor-for', 'filter', 'filter-by',
        'require-selection', 'selection-message', 'options-query-revision', 'options-policy']
        .map(name => instance.getAttribute(name)));
}
/** Local host policy admits original sources; the native owner renders options into the declaration. */
export function createLocalDatalist(instance: HTMLElement, context: CemProducedElementBehaviorContext,
    environment: CemLocalSuggestionsEnvironment, changed: () => void, ownerLost: () => void): CemNativeDatalistAttachment {
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
    const runtime = context.runtime, datalist = surfaces[0] as HTMLDataListElement;
    const producer = environment.snapshot.instanceId, editorProducer = runtime.snapshotInstance(editorHost).instanceId;
    const source = environment.optionsSources[0] as HTMLTemplateElement | undefined;
    const configuration = nativeDatalistConfiguration(instance), captured = source ? [source] : [];
    const namespaceStamp = localSuggestionsNamespaceStamp(captured, environment.limits);
    const sourceText = captureLocalSuggestions(instance.ownerDocument, source, [], environment.limits);
    const abort = new AbortController(), lease = provider.lease({});
    const coordinator = new CemSuggestionsPlacementCoordinator(instance.getRootNode() as Document | ShadowRoot, environment.limits);
    let session: CemNativeCapabilitySession | undefined, publication: CemNativeSuggestionsPublication | undefined;
    let released = false, failed = false, preparing = true, queued = false, editorPlacement: CemSuggestionsPlacementLease | undefined, targetPlacement: CemSuggestionsPlacementLease | undefined;
    const queue = () => {
        if (queued || released) return;
        queued = true; queueMicrotask(() => { queued = false; if (!released) attachment.reconcile(); });
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
    const stopProvider = provider.subscribe(queue);
    const attachment: CemNativeDatalistAttachment = {
        current,
        reconcile() {
            if (released || failed) return;
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
                const placements = coordinator.prepareDatalist(editorPlacement, targetPlacement);
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
            lease.release(); coordinator.dispose(); void publication?.release().catch(() => undefined); void session?.release().catch(() => undefined);
            publication = undefined;
        },
    };
    const stopOwner = environment.owner.onNativeAuthorityLost?.(() => { attachment.release(); ownerLost();
        reportInteractionReference(instance, 'suggestions-native-owner-lost', 'suggestions-binding'); });
    // Wait until the caller installs this attachment before submitting its native frame.
    queueMicrotask(() => { void (async () => {
        try {
            const bytes = new TextEncoder().encode(sourceText);
            session = await CemNativeCapabilitySession.prepare(environment.owner, { action: 'prepare', adapter: 'native-datalist-v1',
                handle: { sessionKey: crypto.randomUUID(), instanceId: environment.snapshot.instanceId,
                    scopePolicyStamp: environment.snapshot.scopePolicyStamp, sourceRevision: crypto.randomUUID() },
                sources: { kind: 'cem-native-session-import-v1', bytes: bytes.buffer as ArrayBuffer, contentType: 'application/xml', sourceUri: instance.baseURI },
                data: {}, select: 'seq:where(input.children.children, fn(n) => n.name == "options").children', limits: environment.limits }, current, abort.signal);
            publication = await session.publishDatalist({}, current);
            if (!current()) throw new Error('Native datalist publication superseded');
            runtime.refreshElementReferences(instance); await runtime.whenRenderSettled(instance);
            if (!current() || runtime.diagnosticsFor(instance).some(d => d.severity === 'error' || d.severity === 'fatal')
                || runtime.renderedSuggestionsFor(instance)?.binding.config.profile !== 'native-datalist') throw new Error('Native datalist frame superseded or failed');
            preparing = false; attachment.reconcile();
        } catch {
            if (released) return;
            failed = true; lease.release(); coordinator.dispose(); void publication?.release().catch(() => undefined);
            void session?.release().catch(() => undefined); publication = undefined;
            reportInteractionReference(instance, 'suggestions-native-source-unavailable', 'suggestions-binding');
            // No automatic retry of an unchanged invalid source. A source/configuration
            // mutation drives fresh preparation through the outer lifecycle.
        }
    })(); });
    return attachment;
}
