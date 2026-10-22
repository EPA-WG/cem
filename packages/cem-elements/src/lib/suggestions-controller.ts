import type { CemProcessingHost } from './internal/runtime-support/processing-host.js';
import type { CheckpointSuggestions, CheckpointSource, CheckpointReadySuggestions } from './checkpoint-interactions.js';
import { checkpointSuggestionIndex, checkpointSuggestionSource } from './native-suggestions-publication.js';
import type { CemPlatformCloseCoordinator, CemPlatformCloseLease } from './platform-close.js';
import { getCemEditorProvider, registerCemEditorCheckpoint, subscribeCemEditorCheckpoint } from './form-control-capability.js';
import { connectCemManualListbox, parkCemManualListbox, type CemManualListboxController } from './manual-listbox.js';
import { reportInteractionReference } from './interaction-reference.js';
import type { CemNativeSuggestionSource } from './native-suggestions-publication.js';
import type { CemSuggestionsPlacements, CemSuggestionsPlacedRow } from './suggestions-placements.js';
import type { CemSurfaceLifetime } from './surface-session.js';

export interface CemSuggestionsControllerOptions {
    editorHost: HTMLElement;
    listbox: HTMLElement;
    /** The trusted host renders and grants a native publication before returning its placements. */
    prepare(query: string, queryRevision: number): Promise<CemSuggestionsPlacements>;
    /** Trusted original source intent for settled inactive states; not live authority. */
    checkpointSource?(): CheckpointSource | undefined;
    /** Original host source policy; only called after every participating owner is bound. */
    resume?(host: CemProcessingHost): Promise<CemSuggestionsControllerOptions>;
    ancestors?: readonly CemSurfaceLifetime[];
    /** Explicit trusted host admission; never enabled by local markup. */
    platformClose?: CemPlatformCloseCoordinator;
    current?(): boolean;
    onError?(error: unknown): void;
    /** Presentation-only lifecycle; never supplies source or placement authority. */
    onFeedback?(feedback: CemSuggestionsFeedback): void;
}
export interface CemSuggestionsFeedback {
    readonly state: 'idle' | 'pending' | 'ready' | 'failed';
    readonly queryRevision: number;
    readonly eligibleCount: number;
    readonly qualifying: boolean;
}
export interface CemSuggestionsController {
    /** Current editor/host lease, independent of query readiness. */
    readonly retained: boolean;
    readonly visible: boolean;
    readonly pending: boolean;
    readonly active: CemNativeSuggestionSource | undefined;
    readonly committed: CemNativeSuggestionSource | undefined;
    readonly feedback: CemSuggestionsFeedback;
    checkpoint(): CheckpointSuggestions | undefined;
    refresh(): Promise<void>;
    reconcile(): void;
    dismiss(reason?: string): void;
    disconnect(): void;
}

const handoffs = new WeakMap<CemSuggestionsController, {
    park(): () => void;
    resume(options: CemSuggestionsControllerOptions, saved: CheckpointSuggestions): Promise<void>;
}>();
export function cemSuggestionsControllerHandoff(controller: CemSuggestionsController) {
    const handoff = handoffs.get(controller); if (!handoff) throw Error('cem.checkpoint.resume.suggestions'); return handoff;
}
/** One shared native-row/controller route. The field provider owns all value writes. */
export function connectCemSuggestionsController(host: HTMLElement, options: CemSuggestionsControllerOptions): CemSuggestionsController {
    const provider = getCemEditorProvider(options.editorHost), editor = provider?.control;
    if (!provider || !(editor instanceof HTMLInputElement) || editor.type !== 'text' || editor.hasAttribute('list')) {
        throw new TypeError('Suggestions requires an exact supported editor provider');
    }
    const checkpointOwner = {};
    const abort = new AbortController(), lease = provider.lease(checkpointOwner), validity = provider.validity(checkpointOwner);
    let paused = false, resumePopup: (() => void) | undefined;
    let disposed = false, generation = 0, queryRevision = 0, preparedRevision = -1, query = '', intent = false;
    let placements: CemSuggestionsPlacements | undefined, unsubscribePlacement: (() => void) | undefined;
    let active: CemSuggestionsPlacedRow | undefined, committed: { source: CemNativeSuggestionSource; value: string } | undefined;
    let stopProof: (() => void) | undefined;
    const clearProof = () => { stopProof?.(); stopProof = undefined; committed = undefined; };
    let platformClose: CemPlatformCloseLease | undefined;
    const releasePlatform = () => { platformClose?.dispose(); platformClose = undefined; };
    let popup: CemManualListboxController | undefined, pending = false, reconciling = false;
    let feedbackIntent = false, feedbackState: CemSuggestionsFeedback['state'] = 'idle';
    let lastFeedback = '';
    let pointer: { row: CemSuggestionsPlacedRow; generation: number; id: number; x: number; y: number; ended: boolean } | undefined;
    const focused = () => editor.ownerDocument.activeElement === editor;
    const retained = () => !disposed && host.isConnected && options.editorHost.isConnected && lease.current && provider.control === editor && (options.current?.() ?? true);
    const bound = () => retained() && lease.valid;
    const current = () => feedbackState === 'ready' && bound() && provider.editable && !provider.composing && !!placements?.current()
        && preparedRevision === provider.revision && query === editor.value;
    const eligibleRow = (row: CemSuggestionsPlacedRow) => row.native.valid && row.native.eligible
        && options.listbox.contains(row.element) && !row.element.closest('[hidden],[inert],[disabled],[aria-disabled="true"]');
    const eligible = (row: CemSuggestionsPlacedRow) => current() && eligibleRow(row);
    const available = () => current() ? placements?.rows.filter(eligibleRow) ?? [] : [];
    const feedback = (): CemSuggestionsFeedback => {
        // A placement check may synchronously revoke the prior view and change readiness.
        const count = feedbackState === 'ready' ? available().length : 0;
        return Object.freeze({ state: feedbackState, queryRevision, eligibleCount: feedbackState === 'ready' ? count : 0,
            qualifying: feedbackIntent && bound() && focused() && provider.editable && !provider.composing });
    };
    const notifyFeedback = () => {
        if (paused) return;
        const next = feedback(), key = JSON.stringify(next);
        if (key !== lastFeedback) { lastFeedback = key; options.onFeedback?.(next); }
    };
    const updateValidity = () => {
        if (paused) return;
        if (committed && !committed.source.valid) clearProof();
        validity.set(host.hasAttribute('require-selection') && !!editor.value && !committed
            ? host.getAttribute('selection-message')?.trim() || 'Select a suggestion.' : '');
    };
    const claim = () => {
        if (reconciling || paused) return;
        reconciling = true;
        try {
            const visible = !!popup?.visible;
            if (active && (!visible || !eligible(active) || !focused())) active = undefined;
            for (const row of placements?.rows ?? []) row.element.setAttribute('aria-selected', String(row === active && visible));
            if (!current()) { lease.attributes.clear(); return; }
            const accepted = lease.attributes.set({ revision: provider.revision, current,
                listbox: options.listbox, expanded: visible, ...(active ? { row: active.element } : {}),
                autocomplete: placements?.binding.config.filter === 'none' ? 'none' : 'list' });
            if (!accepted && visible) popup?.dismiss('attribute-conflict');
        } finally { reconciling = false; notifyFeedback(); }
    };
    const dismiss = (reason = 'dismiss') => { releasePlatform(); intent = false; feedbackIntent = false; pointer = undefined; active = undefined; popup?.dismiss(reason); claim(); };
    const closePresentation = (reason: string) => { releasePlatform(); intent = false; pointer = undefined; active = undefined; popup?.dismiss(reason); claim(); };
    const reservePlatform = (event: KeyboardEvent) => {
        if (!options.platformClose) return;
        const attempt = generation, revision = provider.revision;
        platformClose = options.platformClose.reserve({ event, editor, ancestors: options.ancestors ?? [] },
            () => intent && generation === attempt && provider.revision === revision && bound() && focused()
                && provider.editable && !provider.composing,
            reason => { if (generation === attempt) dismiss(reason); });
        reportInteractionReference(host, platformClose ? undefined : 'interaction-platform-close-unavailable', 'platform-close');
    };
    const invalidate = () => {
        if (paused) return;
        const readiness = host.getAttribute('options-state');
        const retainFeedback = bound() && feedbackIntent && focused() && ['pending', 'failed'].includes(readiness ?? '');
        feedbackState = retainFeedback ? readiness === 'pending' ? 'pending' : 'failed' : 'idle';
        // Suspend the row/surface first; scalar readiness only retains presentation intent.
        const qualified = retainFeedback && feedbackIntent;
        for (const listener of [...lifecycleListeners]) listener();
        if (qualified) { feedbackIntent = true; closePresentation('source'); } else dismiss('authority');
        lease.attributes.clear(); updateValidity();
    };
    const releases: (() => void)[] = [];
    const lifecycleListeners = new Set<() => void>();
    const request = async (opening: boolean, preserveFeedback = false, event?: KeyboardEvent, restored?: CheckpointSuggestions) => {
        if (paused) return;
        releasePlatform();
        if (!bound() || !provider.editable && !(restored && 'phase' in restored && restored.phase === 'inactive' && restored.proof) || provider.composing) { dismiss('unavailable'); return; }
        const attempt = ++generation, revision = provider.revision, value = editor.value, nextQuery = ++queryRevision;
        feedbackIntent = focused() && (opening || preserveFeedback && feedbackIntent);
        intent = opening && focused(); pending = true; feedbackState = 'pending'; active = undefined; pointer = undefined;
        popup?.dismiss('query'); lease.attributes.clear();
        if (intent && event) reservePlatform(event);
        notifyFeedback();
        unsubscribePlacement?.(); unsubscribePlacement = undefined; placements?.release(); placements = undefined;
        try {
            const prepared = await options.prepare(value, nextQuery);
            if (!bound() || attempt !== generation || revision !== provider.revision || editor.value !== value || provider.composing) {
                if (attempt === generation) { feedbackState = 'idle'; feedbackIntent = false; }
                prepared.release(); return;
            }
            if (!prepared.current() || prepared.provider !== provider || prepared.editorHost !== options.editorHost || prepared.listbox !== options.listbox
                || prepared.binding.config.query !== value || prepared.binding.config.queryRevision !== nextQuery) {
                prepared.release(); throw new Error('Suggestions preparation differs from its captured editor/query');
            }
            placements = prepared;
            unsubscribePlacement = prepared.subscribe(invalidate); preparedRevision = revision; query = value; pending = false;
            if (committed && host.getAttribute('options-policy') === 'vocabulary'
                && !prepared.rows.some(row => row.native.source === committed?.source && row.native.available)) clearProof();
            if (restored) {
                const proof = 'phase' in restored ? restored.phase === 'inactive' ? restored.proof : undefined : restored.committedIndex < 0 ? undefined
                    : { index: restored.committedIndex, value: restored.committedValue };
                clearProof();
                if (proof) {
                    const fresh = (await prepared.binding.rows()).find(row => checkpointSuggestionIndex(row.source) === proof.index);
                    if (!fresh?.valid || !fresh.available || fresh.value !== proof.value || editor.value !== proof.value)
                        throw Error('cem.checkpoint.resume.proof');
                    committed = { source: fresh.source, value: fresh.value };
                    stopProof = fresh.source.subscribe(() => { if (!paused) { clearProof(); updateValidity(); } });
                }
                if (!('phase' in restored)) {
                    if (prepared.rows.length !== restored.rows.length || prepared.rows.some((row, index) => {
                        const expected = restored.rows[index]; return checkpointSuggestionIndex(row.native.source) !== expected.index
                            || row.native.value !== expected.value || eligibleRow(row) !== expected.eligible || row.native.available !== expected.available
                            || row.element.getAttribute('data-cem-render-node-id') !== expected.nodeId;
                    })) throw Error('cem.checkpoint.resume.rows');
                    intent = restored.intent && focused(); feedbackIntent = restored.feedbackIntent;
                }
            }
            feedbackState = 'ready'; updateValidity(); reportInteractionReference(host, undefined, 'suggestions-source');
            claim();
            let presented: Promise<void> | undefined;
            if (intent && focused() && available().length) {
                if (restored && !options.listbox.matches(':popover-open')) presented = new Promise<void>(resolve => options.listbox.addEventListener('toggle', () => resolve(), { once: true }));
                intent = !!popup?.open();
            }
            else if (!available().length) closePresentation('empty');
            if (restored && !('phase' in restored)) {
                active = restored.activeIndex < 0 ? undefined : prepared.rows.find(row => checkpointSuggestionIndex(row.native.source) === restored.activeIndex);
                if (restored.activeIndex >= 0 && (!active || !eligible(active))) throw Error('cem.checkpoint.resume.active');
                options.listbox.scrollTop = restored.scrollTop; options.listbox.scrollLeft = restored.scrollLeft;
            }
            claim();
            if (presented && intent) await presented;
        } catch (error) {
            if (!disposed && attempt === generation) {
                pending = false; feedbackState = host.getAttribute('options-state') === 'pending' ? 'pending' : 'failed';
                closePresentation('source'); lease.attributes.clear();
                reportInteractionReference(host, 'suggestions-source-unavailable', 'suggestions-source'); options.onError?.(error);
                if (restored) throw error;
            }
        } finally { if (attempt === generation) { pending = false; notifyFeedback(); } }
    };
    try {
        popup = connectCemManualListbox(options.listbox, { host, editorHost: options.editorHost, editorLease: lease,
            ancestors: options.ancestors, lifecycle: { current, ready: () => available().length > 0,
                subscribe(listener) { lifecycleListeners.add(listener); return () => { lifecycleListeners.delete(listener); }; } },
            onVisibility(open, reason) { if (!open) {
                releasePlatform();
                if (reason !== 'query') intent = false;
                // Source readiness may close the old view before its next
                // committed render; focused pending/failure feedback survives.
                const sourceUnavailable = reason === 'unavailable' && ['pending', 'failed'].includes(host.getAttribute('options-state') ?? '')
                    && bound() && focused() && provider.editable && !provider.composing;
                if (!['query', 'empty', 'source'].includes(reason) && !sourceUnavailable) feedbackIntent = false;
                active = undefined; pointer = undefined;
            } claim(); },
        });
    } catch (error) { lease.release(); validity.release(); throw error; }
    const commit = (row: CemSuggestionsPlacedRow) => {
        if (!eligible(row) || !focused() || !popup?.visible) return false;
        if (committed?.source === row.native.source && committed.value === row.native.value && editor.value === row.native.value) {
            dismiss('repeat'); return true;
        }
        const attempt = generation;
        return lease.commit(row.native.value, { revision: provider.revision,
            current: () => attempt === generation && eligible(row) && focused() && !!popup?.visible,
            applied() { clearProof(); committed = { source: row.native.source, value: row.native.value };
                stopProof = row.native.source.subscribe(() => { if (!paused) { clearProof(); updateValidity(); } }); dismiss('commit'); updateValidity(); },
        });
    };
    const move = (direction: number) => {
        const ready = available(); if (!ready.length || !popup?.open()) return false;
        feedbackIntent = true;
        const index = active ? ready.indexOf(active) : -1;
        active = ready[index < 0 ? direction > 0 ? 0 : ready.length - 1 : Math.max(0, Math.min(ready.length - 1, index + direction))];
        claim();
        if (!active || !lease.attributes.valid) return false;
        const rect = active.element.getBoundingClientRect(), bounds = options.listbox.getBoundingClientRect();
        const top = bounds.top + options.listbox.clientTop, bottom = top + options.listbox.clientHeight;
        if (rect.top < top) options.listbox.scrollTop += rect.top - top;
        else if (rect.bottom > bottom) options.listbox.scrollTop += rect.bottom - bottom;
        return true;
    };
    releases.push(subscribeCemEditorCheckpoint(provider, lease, update => {
        if (disposed || paused) return;
        if (update.cause === 'attribute-claims') return;
        if (update.cause === 'claims' || update.cause === 'availability') {
            if (!bound() || !provider.editable) invalidate();
            else if (!pending && placements && !current()) void request(false);
            else claim();
            return;
        }
        if (update.cause === 'composition-start') { ++generation; pending = false; dismiss('composition'); lease.attributes.clear(); return; }
        if (update.cause !== 'commit' && update.cause !== 'composition-end') clearProof();
        updateValidity();
        if (update.cause === 'commit') { void request(false); return; }
        const opening = update.cause === 'input' || update.cause === 'composition-end';
        void request(opening);
    }));
    editor.addEventListener('focus', () => { if (current()) { intent = true; feedbackIntent = true; active = undefined; if (available().length) popup?.open(); claim(); } else void request(true); }, { signal: abort.signal });
    options.editorHost.addEventListener('keydown', event => {
        if (event.target !== editor || event.defaultPrevented || !bound() || provider.compositionOwned(event)) return;
        if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey && event.key !== 'Tab') return;
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            if (!popup?.visible && !intent) {
                if (!available().length) { void request(true, false, event); return; }
                intent = true; reservePlatform(event);
                if (!intent) return; // Synchronous host revocation canceled this opening.
            }
            if (move(event.key === 'ArrowDown' ? 1 : -1)) event.preventDefault();
            else if (!pending) void request(true, false, event);
        } else if (event.key === 'Enter' && active && eligible(active) && popup?.visible) {
            if (lease.handlePress(event)) commit(active);
        } else if (event.key === 'Escape' && (intent || feedbackIntent || popup?.visible || popup?.pending)) {
            if (lease.handlePress(event)) dismiss('escape');
        } else if (event.key === 'Tab') dismiss('tab');
        else if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) { active = undefined; claim(); }
    }, { signal: abort.signal, capture: true });
    const rowAt = (target: EventTarget | null) => target instanceof Node && current() ? placements?.rows.find(row => row.element.contains(target) && eligibleRow(row)) : undefined;
    options.listbox.addEventListener('pointerdown', event => {
        pointer = undefined;
        if (event.defaultPrevented || !event.isPrimary || !['mouse', 'touch', 'pen'].includes(event.pointerType) || event.button !== 0 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || !focused()) return;
        const row = rowAt(event.target); if (!row) return;
        pointer = { row, generation, id: event.pointerId, x: event.clientX, y: event.clientY, ended: false };
        if (event.pointerType === 'mouse') event.preventDefault();
    }, { signal: abort.signal });
    // Compatibility mouse focus is suppressed only for this exact armed row.
    // Touch/pen pointerdown remains native so gestures may become scrolling.
    options.listbox.addEventListener('mousedown', event => {
        if (!event.defaultPrevented && event.button === 0 && !event.altKey && !event.ctrlKey && !event.metaKey && !event.shiftKey
            && focused() && pointer?.generation === generation && rowAt(event.target) === pointer.row) event.preventDefault();
    }, { signal: abort.signal });
    options.listbox.addEventListener('pointermove', event => {
        if (pointer && (event.pointerId !== pointer.id || Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) > 8)) pointer = undefined;
    }, { signal: abort.signal });
    options.listbox.addEventListener('pointerup', event => {
        if (pointer && event.pointerId === pointer.id && rowAt(event.target) === pointer.row && pointer.generation === generation) pointer.ended = true;
        else pointer = undefined;
    }, { signal: abort.signal });
    for (const event of ['pointercancel', 'scroll']) options.listbox.addEventListener(event, () => { pointer = undefined; }, { signal: abort.signal, capture: true });
    options.listbox.addEventListener('lostpointercapture', event => {
        // Native touch releases implicit capture after pointerup, before click.
        // An already completed exact-row press remains armed for that click.
        if (pointer?.id === event.pointerId && !pointer.ended) pointer = undefined;
    }, { signal: abort.signal, capture: true });
    options.listbox.addEventListener('click', event => {
        const row = rowAt(event.target), armed = pointer; pointer = undefined;
        if (event.defaultPrevented || event.button !== 0 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || !row || !focused()) return;
        if (event.detail === 0 || armed?.ended && armed.generation === generation && armed.row === row) { event.preventDefault(); commit(row); }
    }, { signal: abort.signal });
    editor.addEventListener('select', () => { active = undefined; claim(); }, { signal: abort.signal });
    editor.addEventListener('focusout', () => { dismiss('focus'); }, { signal: abort.signal });
    const checkpoint = (): CheckpointSuggestions | undefined => {
        if (!bound() || pending || pointer || reconciling || platformClose || popup?.pending || provider.composing
            || options.ancestors?.length || host.getAttribute('options-state') === 'pending') return undefined;
        if (feedbackState !== 'ready' || !provider.editable) {
            // A retained proof carries its own exact source, independent of current feedback.
            if (feedbackState === 'pending' || active || popup?.visible) return undefined;
            const source = options.checkpointSource?.();
            if (!source || source.rowCount !== 0) return undefined;
            const proofSource = committed && checkpointSuggestionSource(committed.source);
            const index = committed && checkpointSuggestionIndex(committed.source);
            if (committed && (!proofSource || index === undefined || index >= proofSource.rowCount || committed.value !== editor.value)) return undefined;
            const proof = committed && proofSource && index !== undefined ? { source: proofSource, index, value: committed.value, disposition: 'revalidate' as const } : undefined;
            return { ...(proof ? { proof } : {}), phase: 'inactive', profile: 'listbox', source, providerRevision: provider.revision, queryRevision,
                query: editor.value, feedbackState: !provider.editable ? 'unavailable' : feedbackState === 'failed' ? 'failed' : 'idle',
                feedbackIntent: provider.editable && feedbackIntent, editable: provider.editable };
        }
        if (!current()) return undefined;
        const config = placements?.binding.config;
        if (!config || config.profile === 'native-datalist') return undefined;
        const source = placements?.binding.checkpointSource();
        if (!source || !placements || placements.rows.length > 1024) return undefined;
        let size = query.length + (committed?.value.length ?? 0);
        const rows: CheckpointReadySuggestions['rows'] = [];
        for (const row of placements.rows) {
            const index = checkpointSuggestionIndex(row.native.source), nodeId = row.element.getAttribute('data-cem-render-node-id');
            size += row.native.value.length + (nodeId?.length ?? 0);
            if (size > 256 * 1024 || index === undefined || !nodeId || !row.native.valid) return undefined;
            rows.push({ index, nodeId, value: row.native.value, eligible: eligibleRow(row), available: row.native.available });
        }
        const activeIndex = active ? checkpointSuggestionIndex(active.native.source) : -1;
        const committedIndex = committed ? checkpointSuggestionIndex(committed.source) : -1;
        if (activeIndex === undefined || committedIndex === undefined || committed && committed.value !== editor.value) return undefined;
        return { source, publication: { filter: config.filter ?? 'contains', filterBy: config.filterBy ?? null, expanded: config.expanded ?? null, active: config.active ?? null, committed: config.committed ?? null }, providerRevision: provider.revision, queryRevision, preparedRevision, query, intent, feedbackIntent,
            feedbackState: 'ready', activeIndex, committedIndex, committedValue: committed?.value ?? null,
            visible: !!popup?.visible, scrollTop: options.listbox.scrollTop, scrollLeft: options.listbox.scrollLeft, rows };
    };
    releases.push(registerCemEditorCheckpoint(provider, { host, owner: checkpointOwner, lease, current: () => !!checkpoint(),
        claim: () => { const state = checkpoint(); return state ? 'phase' in state ? 'none' : 'attributes' : undefined; } }));
    void request(false); updateValidity();
    const controller: CemSuggestionsController = {
        checkpoint,
        get retained() { return retained(); },
        get visible() { return !!popup?.visible; }, get pending() { return pending; },
        get active() { return active?.native.source; }, get committed() { updateValidity(); return committed?.source; },
        get feedback() { return feedback(); },
        refresh: () => request(false, true), reconcile: () => { if (paused) return; platformClose?.reconcile(); popup?.reconcile(); claim(); updateValidity(); }, dismiss,
        disconnect() {
            if (disposed) return; releasePlatform(); disposed = true; ++generation; pending = false; feedbackIntent = false; feedbackState = 'idle';
            notifyFeedback();
            abort.abort(); clearProof(); unsubscribePlacement?.(); placements?.release();
            for (const release of [...releases]) release(); releases.length = 0;
            popup?.disconnect(); lease.release(); validity.release();
        },
    };
    handoffs.set(controller, {
        park() {
            if (paused || !checkpoint()) throw Error('cem.checkpoint.resume.suggestions');
            const unparkPopup = parkCemManualListbox(popup!); resumePopup = unparkPopup; paused = true;
            let released = false;
            return () => { if (released) return; released = true; paused = false; unparkPopup(); };
        },
        async resume(next, saved) {
            if ('phase' in saved && saved.phase === 'native-datalist') throw Error('cem.checkpoint.resume.profile');
            if (!paused || next.editorHost !== options.editorHost || next.listbox !== options.listbox || provider.composing
                || saved.providerRevision !== provider.revision) throw Error('cem.checkpoint.resume.provider');
            options = { ...next, onFeedback: options.onFeedback };
            // Expired native rows are never current again. Rebuild from the fresh
            // source; the browser lease retains its consumed-change/default state.
            paused = false; resumePopup?.(); resumePopup = undefined; queryRevision = saved.queryRevision - 1;
            await request(!('phase' in saved) && saved.intent, false, undefined, saved);
            if (!checkpoint()) throw Error('cem.checkpoint.resume.publication');
        },
    });
    return controller;
}
