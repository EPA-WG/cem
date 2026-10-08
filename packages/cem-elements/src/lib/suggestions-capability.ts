import type { DataIslandSnapshot, CemProducedElementBehavior, CemProducedElementBehaviorContext } from './cem-elements.js';
import { connectCemSuggestionsController, type CemSuggestionsController, type CemSuggestionsControllerOptions } from './suggestions-controller.js';
import { resolveCemSuggestionsEditor } from './suggestions-editor.js';
import { CemNativeCapabilitySession } from './native-capability-session.js';
import { CemSuggestionsPlacementCoordinator, type CemSuggestionsPlacementLease } from './suggestions-placements.js';
import type { CemNativeSuggestionsBinding, CemNativeSuggestionsPublication } from './native-suggestions-publication.js';
import type { CemProcessingHost } from './internal/runtime-support/processing-host.js';
import type { CemValueArtifactLimits } from './native-values.js';
import { observeInteractionReferences, reportInteractionReference } from './interaction-reference.js';

interface State {
    context: CemProducedElementBehaviorContext;
    options?: CemSuggestionsControllerOptions;
    controller?: CemSuggestionsController;
    observer?: MutationObserver;
    stopReferences?: () => void;
    queued: boolean;
    configuration?: string;
    local?: LocalState;
    localBlocked?: boolean;
}
export interface CemLocalSuggestionsEnvironment {
    snapshot: DataIslandSnapshot;
    owner: CemProcessingHost;
    limits: CemValueArtifactLimits;
    optionsSources: readonly Element[];
    sourceRoot?: Element;
    ownsEditor(editor: HTMLElement): boolean;
    current(): boolean;
}
interface LocalState {
    options: CemSuggestionsControllerOptions;
    template?: HTMLTemplateElement;
    sourceTemplate?: HTMLTemplateElement;
    provider: object;
    abort: AbortController;
    binding?: CemNativeSuggestionsBinding;
    publication?: CemNativeSuggestionsPublication;
    release(): void;
}
const states = new WeakMap<HTMLElement, State>();
/** Package-private read-only view of the active shared controller. */
export function cemSuggestionsControllerFor(instance: HTMLElement): CemSuggestionsController | undefined { return states.get(instance)?.controller; }
function stop(state: State): void { state.controller?.disconnect(); state.controller = undefined; state.options = undefined; }
/** Host revocation is synchronous; authorization and leases never enter a snapshot. */
export function refreshCemSuggestionsAuthorization(instance: HTMLElement): void {
    const state = states.get(instance); if (!state) return;
    if (state.local) { stop(state); state.local.release(); state.local = undefined; }
    state.localBlocked = false;
    synchronize(instance, state);
}
export function localCemSuggestionsBinding(instance: HTMLElement, snapshot: DataIslandSnapshot): CemNativeSuggestionsBinding | undefined {
    const local = states.get(instance)?.local;
    if (!local?.binding?.valid || !local.options.current?.()) return undefined;
    // Each render acquires its own lease; the source/publication stay native.
    return local.publication?.bind({ instanceId: snapshot.instanceId, scopePolicyStamp: snapshot.scopePolicyStamp,
        revision: snapshot.dataRevision, current: local.options.current });
}
function releaseLocal(state: State): void { state.local?.release(); state.local = undefined; }

function localInputs(instance: HTMLElement, state: State): CemSuggestionsControllerOptions | undefined {
    const runtime = state.context.runtime, environment = runtime.localSuggestionsEnvironmentFor(instance);
    if (!environment || state.localBlocked) { releaseLocal(state); return undefined; }
    const endpoint = resolveCemSuggestionsEditor(instance);
    if (!endpoint.host || !endpoint.provider) throw new Error('Local suggestions requires an exact editor provider');
    const { host: editorHost, provider } = endpoint;
    const templates = [...instance.children].filter(child => child.getAttribute('slot') === 'options');
    const surfaces = [...instance.children].filter(child => child.getAttribute('part')?.split(/\s+/).includes('surface'));
    if (endpoint.code || instance.hasAttribute('editor-for') || instance.hasAttribute('options')
        || environment.snapshot.nativeAttributes?.some(attribute => ['options', 'editor-for'].includes(attribute.name))
        || environment.optionsSources.length !== templates.length
        || environment.optionsSources.some(source => !(source instanceof HTMLTemplateElement))
        || templates.length > 1 || templates.length === 1 && !(templates[0] instanceof HTMLTemplateElement)
        || surfaces.length !== 1 || !(surfaces[0] instanceof HTMLElement) || surfaces[0].getAttribute('role') !== 'listbox'
        || surfaces[0].getAttribute('popover') !== 'manual' || surfaces[0].hasAttribute('slot') || !environment.ownsEditor(editorHost)) {
        throw new Error('Local suggestions requires exact local slots without explicit input conflicts');
    }
    const template = templates[0] as HTMLTemplateElement | undefined, listbox = surfaces[0] as HTMLElement;
    const sourceTemplate = environment.optionsSources[0] as HTMLTemplateElement | undefined;
    const previous = state.local;
    if (previous && previous.template === template && previous.sourceTemplate === sourceTemplate && previous.provider === provider
        && previous.options.editorHost === editorHost && previous.options.listbox === listbox && previous.options.current?.()) return previous.options;
    stop(state); releaseLocal(state);
    const captureSource = () => {
        const wrapper = instance.ownerDocument.createElementNS('http://www.w3.org/1999/xhtml', 'options');
        if (sourceTemplate) {
            const walker = instance.ownerDocument.createTreeWalker(sourceTemplate.content, NodeFilter.SHOW_ALL); let count = 0, bytes = 0;
            const depths = new WeakMap<Node, number>(); depths.set(sourceTemplate.content, 0);
            for (let node = walker.nextNode(); node; node = walker.nextNode()) {
                const depth = (node.parentNode ? depths.get(node.parentNode) ?? 0 : 0) + 1; depths.set(node, depth);
                if (++count > environment.limits.maxValues || depth > environment.limits.maxDepth) throw new RangeError('Local options capture node/depth limit exceeded');
                if (node instanceof HTMLTemplateElement) throw new Error('Local options cannot contain nested source templates');
                if ((node.nodeValue?.length ?? 0) > environment.limits.maxBytes) throw new RangeError('Local options capture byte limit exceeded');
                bytes += new TextEncoder().encode(node.nodeValue ?? '').byteLength;
                if (node instanceof Element) for (const attribute of node.attributes) {
                    if (attribute.name.length + attribute.value.length > environment.limits.maxBytes) throw new RangeError('Local options capture byte limit exceeded');
                    bytes += new TextEncoder().encode(attribute.name + attribute.value).byteLength;
                }
                if (bytes > environment.limits.maxBytes) throw new RangeError('Local options capture byte limit exceeded');
            }
            // Namespace bindings belong to the original capture, including unused prefixes.
            const namespaces = new Map<string, string>();
            for (let ancestor: Element | null = sourceTemplate; ancestor; ancestor = ancestor.parentElement) {
                for (const attribute of ancestor.attributes) if ((attribute.namespaceURI === 'http://www.w3.org/2000/xmlns/' || attribute.name === 'xmlns' || attribute.name.startsWith('xmlns:')) && !namespaces.has(attribute.name)) namespaces.set(attribute.name, attribute.value);
            }
            for (const [name, value] of namespaces) wrapper.setAttributeNS('http://www.w3.org/2000/xmlns/', name, value);
            wrapper.append(sourceTemplate.content.cloneNode(true));
        }
        const text = new XMLSerializer().serializeToString(wrapper);
        if (new TextEncoder().encode(text).byteLength > environment.limits.maxBytes) throw new RangeError('Local options capture byte limit exceeded');
        return text;
    };
    const sourceText = captureSource();

    const abort = new AbortController(), coordinator = new CemSuggestionsPlacementCoordinator(instance.getRootNode() as Document | ShadowRoot, environment.limits);
    let session: Promise<CemNativeCapabilitySession> | undefined, publication: CemNativeSuggestionsPublication | undefined;
    let released = false;
    let latest = 0, leases: CemSuggestionsPlacementLease[] = [], stopOwner: (() => void) | undefined;
    const queue = () => { queueMicrotask(() => synchronize(instance, state)); };
    const sourceObserver = new MutationObserver(() => { if (state.local === local) { stop(state); releaseLocal(state); queue(); } });
    if (environment.sourceRoot) sourceObserver.observe(environment.sourceRoot, { childList: true });
    if (sourceTemplate) sourceObserver.observe(sourceTemplate, { attributes: true });
    if (sourceTemplate) sourceObserver.observe(sourceTemplate.content, { childList: true, subtree: true, attributes: true, characterData: true });
    const local: LocalState = { template, sourceTemplate, provider, abort,
        options: { editorHost, listbox,
            current: () => {
                if (sourceObserver.takeRecords().length) { abort.abort(); queue(); }
                return !abort.signal.aborted && state.local === local && environment.current()
                    && resolveCemSuggestionsEditor(instance).provider === provider
                    && editorHost.parentElement === instance && listbox.parentElement === instance
                    && !instance.hasAttribute('editor-for') && !instance.hasAttribute('options')
                    && [...instance.children].filter(child => child.getAttribute('slot') === 'options').length === (template ? 1 : 0)
                    && (!template || template.parentElement === instance)
                    && runtime.localSuggestionsSourceCurrent(instance, sourceTemplate);
            },
            async prepare(query, queryRevision) {
                if (!local.options.current?.()) throw new Error('Local suggestions authority is unavailable');
                const operation = ++latest;
                for (const lease of leases) lease.dispose(); leases = [];
                local.binding?.release(); local.binding = undefined; local.publication = undefined;
                void publication?.release().catch(() => undefined); publication = undefined;
                const ready = () => (instance.getAttribute('options-state') ?? 'ready') === 'ready';
                const controls = () => ['filter', 'filter-by', 'options-revision', 'options-query-revision', 'options-policy'].map(name => instance.getAttribute(name)).join('\u0000');
                const capturedControls = controls();
                const queryCurrent = () => local.options.current?.() === true && latest === operation && ready() && controls() === capturedControls;
                if (!ready()) throw new Error('Local suggestions source is pending, failed or has invalid readiness');
                if (!session) {
                    // Serialize the authored inert DOM only at the explicit XML import boundary.
                    const bytes = new TextEncoder().encode(sourceText);
                    session = CemNativeCapabilitySession.prepare(environment.owner, { action: 'prepare', adapter: 'suggestions-v1',
                        handle: { sessionKey: crypto.randomUUID(), instanceId: environment.snapshot.instanceId,
                            scopePolicyStamp: environment.snapshot.scopePolicyStamp, sourceRevision: crypto.randomUUID() },
                        sources: { kind: 'cem-native-session-import-v1', bytes: bytes.buffer as ArrayBuffer, contentType: 'application/xml', sourceUri: instance.baseURI },
                        data: {}, select: 'input.children.children', limits: environment.limits,
                    }, () => local.options.current?.() === true, abort.signal);
                }
                const source = await session;
                if (!queryCurrent()) throw new Error('Local suggestions preparation was superseded');
                const filter = instance.getAttribute('filter') ?? 'contains';
                if (!['contains', 'prefix', 'none'].includes(filter)) throw new Error('Local sources require a local filter mode');
                const next = await source.publishSuggestions({ query, queryRevision, filter: filter as 'contains' | 'prefix' | 'none',
                    ...(instance.hasAttribute('filter-by') ? { filterBy: instance.getAttribute('filter-by') ?? '' } : {}) }, queryCurrent);
                try {
                    if (!queryCurrent()) throw new Error('Local suggestions publication was superseded');
                    publication = next; local.publication = next;
                    local.binding = next.bind({ instanceId: environment.snapshot.instanceId, scopePolicyStamp: environment.snapshot.scopePolicyStamp,
                        revision: String(queryRevision), current: queryCurrent });
                    runtime.refreshElementReferences(instance); await runtime.whenRenderSettled(instance);
                    const mapped = runtime.renderedSuggestionsFor(instance);
                    if (!queryCurrent() || !mapped?.current()) throw new Error('Local suggestions has no current committed row placements');
                    const fieldProducer = runtime.snapshotInstance(editorHost).instanceId, producer = environment.snapshot.instanceId;
                    const current = () => queryCurrent() && mapped.current();
                    const editorLease = coordinator.register({ producer: fieldProducer, revision: String(provider.revision), current,
                        kind: 'editor', element: editorHost }); leases.push(editorLease);
                    const listboxLease = coordinator.register({ producer, revision: String(queryRevision), current, kind: 'listbox', element: listbox }); leases.push(listboxLease);
                    coordinator.grant(producer, editorLease, ['editor-for']); coordinator.grant(fieldProducer, listboxLease, ['aria-controls']);
                    const rows = mapped.rows.map(row => {
                        const lease = coordinator.register({ producer, revision: String(queryRevision), current, kind: 'row', element: row.element, row: row.native });
                        leases.push(lease); coordinator.grant(fieldProducer, lease, ['aria-activedescendant']); return lease;
                    });
                    return coordinator.prepare(mapped.binding, editorLease, listboxLease, rows);
                } catch (error) { await next.release().catch(() => undefined); throw error; }
            },
        },
        release() {
            if (released) return; released = true;
            abort.abort(); sourceObserver.disconnect(); stopOwner?.(); stopOwner = undefined;
            for (const lease of leases) lease.dispose(); leases = []; coordinator.dispose();
            local.binding?.release(); local.binding = undefined; local.publication = undefined; void publication?.release().catch(() => undefined);
            void session?.then(source => source.release(), () => undefined).catch(() => undefined);
        },
    };
    state.local = local;
    stopOwner = environment.owner.onNativeAuthorityLost?.(() => { state.localBlocked = true; stop(state); releaseLocal(state); queue(); });
    return local.options;
}
function synchronize(instance: HTMLElement, state: State): void {
    if (!instance.isConnected || !state.observer) return;
    const endpoint = resolveCemSuggestionsEditor(instance);
    let options: CemSuggestionsControllerOptions | undefined;
    try {
        options = state.context.runtime.suggestionsControllerInputsFor(instance);
        if (options) releaseLocal(state); else options = localInputs(instance, state);
    } catch { stop(state); releaseLocal(state); reportInteractionReference(instance, 'suggestions-local-input-conflict', 'suggestions-binding'); return; }
    if (endpoint.code || !options || endpoint.host !== options.editorHost || options.listbox.parentElement !== instance
        || !options.listbox.getAttribute('part')?.split(/\s+/).includes('surface') || options.listbox.hasAttribute('slot')) {
        stop(state); reportInteractionReference(instance, endpoint.code ?? 'suggestions-admission-required', 'suggestions-binding'); return;
    }
    if (!state.controller?.retained || state.options?.prepare !== options.prepare || state.options?.editorHost !== options.editorHost || state.options?.listbox !== options.listbox
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
            attributeFilter: ['require-selection', 'selection-message', 'filter', 'filter-by', 'options-state', 'options-revision', 'options-query-revision', 'options-policy', 'options', 'editor-for', 'slot', 'part', 'role', 'popover'] });
        state.stopReferences = observeInteractionReferences(instance, queue);
    },
    rendered(instance, context) { const state = states.get(instance); if (state) { state.context = context; synchronize(instance, state); } },
    disconnected(instance) {
        const state = states.get(instance); if (!state) return;
        state.observer?.disconnect(); state.observer = undefined; state.stopReferences?.(); state.stopReferences = undefined;
        stop(state); releaseLocal(state); state.localBlocked = false; state.configuration = undefined;
    },
};
