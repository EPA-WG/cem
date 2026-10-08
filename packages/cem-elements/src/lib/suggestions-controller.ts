import { getCemEditorProvider } from './form-control-capability.js';
import { connectCemManualListbox, type CemManualListboxController } from './manual-listbox.js';
import { reportInteractionReference } from './interaction-reference.js';
import type { CemNativeSuggestionSource } from './native-suggestions-publication.js';
import type { CemSuggestionsPlacements, CemSuggestionsPlacedRow } from './suggestions-placements.js';
import type { CemSurfaceLifetime } from './surface-session.js';

export interface CemSuggestionsControllerOptions {
    editorHost: HTMLElement;
    listbox: HTMLElement;
    /** The trusted host renders and grants a native publication before returning its placements. */
    prepare(query: string, queryRevision: number): Promise<CemSuggestionsPlacements>;
    ancestors?: readonly CemSurfaceLifetime[];
    current?(): boolean;
    onError?(error: unknown): void;
}
export interface CemSuggestionsController {
    readonly visible: boolean;
    readonly pending: boolean;
    readonly active: CemNativeSuggestionSource | undefined;
    readonly committed: CemNativeSuggestionSource | undefined;
    refresh(): Promise<void>;
    reconcile(): void;
    dismiss(reason?: string): void;
    disconnect(): void;
}

/** One shared native-row/controller route. The field provider owns all value writes. */
export function connectCemSuggestionsController(host: HTMLElement, options: CemSuggestionsControllerOptions): CemSuggestionsController {
    const provider = getCemEditorProvider(options.editorHost), editor = provider?.control;
    if (!provider || !(editor instanceof HTMLInputElement) || editor.type !== 'text' || editor.hasAttribute('list')) {
        throw new TypeError('Suggestions requires an exact supported editor provider');
    }
    const abort = new AbortController(), lease = provider.lease({}), validity = provider.validity({});
    let disposed = false, generation = 0, queryRevision = 0, preparedRevision = -1, query = '', intent = false;
    let placements: CemSuggestionsPlacements | undefined, unsubscribePlacement: (() => void) | undefined;
    let active: CemSuggestionsPlacedRow | undefined, committed: { source: CemNativeSuggestionSource; value: string } | undefined;
    let stopProof: (() => void) | undefined;
    const clearProof = () => { stopProof?.(); stopProof = undefined; committed = undefined; };
    let popup: CemManualListboxController | undefined, pending = false, reconciling = false;
    let pointer: { row: CemSuggestionsPlacedRow; generation: number; id: number; x: number; y: number; ended: boolean } | undefined;
    const focused = () => editor.ownerDocument.activeElement === editor;
    const bound = () => !disposed && host.isConnected && options.editorHost.isConnected && lease.valid && provider.control === editor && (options.current?.() ?? true);
    const current = () => bound() && provider.editable && !provider.composing && !!placements?.current()
        && preparedRevision === provider.revision && query === editor.value;
    const eligibleRow = (row: CemSuggestionsPlacedRow) => row.native.valid && row.native.eligible
        && options.listbox.contains(row.element) && !row.element.closest('[hidden],[inert],[disabled],[aria-disabled="true"]');
    const eligible = (row: CemSuggestionsPlacedRow) => current() && eligibleRow(row);
    const available = () => current() ? placements?.rows.filter(eligibleRow) ?? [] : [];
    const updateValidity = () => {
        if (committed && !committed.source.valid) clearProof();
        validity.set(host.hasAttribute('require-selection') && !!editor.value && !committed
            ? host.getAttribute('selection-message')?.trim() || 'Select a suggestion.' : '');
    };
    const claim = () => {
        if (reconciling) return;
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
        } finally { reconciling = false; }
    };
    const dismiss = (reason = 'dismiss') => { intent = false; pointer = undefined; active = undefined; popup?.dismiss(reason); claim(); };
    const invalidate = () => { for (const listener of [...lifecycleListeners]) listener(); dismiss('authority'); lease.attributes.clear(); updateValidity(); };
    const releases: (() => void)[] = [];
    const lifecycleListeners = new Set<() => void>();
    const request = async (opening: boolean) => {
        if (!bound() || !provider.editable || provider.composing) { dismiss('unavailable'); return; }
        const attempt = ++generation, revision = provider.revision, value = editor.value, nextQuery = ++queryRevision;
        intent = opening && focused(); pending = true; active = undefined; pointer = undefined;
        popup?.dismiss('query'); lease.attributes.clear();
        unsubscribePlacement?.(); unsubscribePlacement = undefined; placements?.release(); placements = undefined;
        try {
            const prepared = await options.prepare(value, nextQuery);
            if (!bound() || attempt !== generation || revision !== provider.revision || editor.value !== value || provider.composing) {
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
            updateValidity(); reportInteractionReference(host, undefined, 'suggestions-source');
            claim();
            if (intent && focused() && available().length) intent = !!popup?.open();
            else if (!available().length) dismiss('empty');
            claim();
        } catch (error) {
            if (!disposed && attempt === generation) {
                pending = false; dismiss('source'); lease.attributes.clear();
                reportInteractionReference(host, 'suggestions-source-unavailable', 'suggestions-source'); options.onError?.(error);
            }
        } finally { if (attempt === generation) pending = false; }
    };
    try {
        popup = connectCemManualListbox(options.listbox, { host, editorHost: options.editorHost, editorLease: lease,
            ancestors: options.ancestors, lifecycle: { current, ready: () => available().length > 0,
                subscribe(listener) { lifecycleListeners.add(listener); return () => { lifecycleListeners.delete(listener); }; } },
            onVisibility(open, reason) { if (!open) { if (reason !== 'query') intent = false; active = undefined; pointer = undefined; } claim(); },
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
                stopProof = row.native.source.subscribe(() => { clearProof(); updateValidity(); }); dismiss('commit'); updateValidity(); },
        });
    };
    const move = (direction: number) => {
        const ready = available(); if (!ready.length || !popup?.open()) return false;
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
    releases.push(provider.subscribe(update => {
        if (disposed) return;
        if (update.cause === 'attribute-claims') return;
        if (update.cause === 'claims' || update.cause === 'availability') {
            if (!bound() || !provider.editable) invalidate(); else claim();
            return;
        }
        if (update.cause === 'composition-start') { ++generation; pending = false; dismiss('composition'); lease.attributes.clear(); return; }
        if (update.cause !== 'commit' && update.cause !== 'composition-end') clearProof();
        updateValidity();
        if (update.cause === 'commit') { void request(false); return; }
        const opening = update.cause === 'input' || update.cause === 'composition-end';
        void request(opening);
    }));
    editor.addEventListener('focus', () => { if (current()) { intent = true; active = undefined; if (available().length) popup?.open(); claim(); } else void request(true); }, { signal: abort.signal });
    options.editorHost.addEventListener('keydown', event => {
        if (event.target !== editor || event.defaultPrevented || !bound() || provider.compositionOwned(event)) return;
        if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey && event.key !== 'Tab') return;
        if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
            if (move(event.key === 'ArrowDown' ? 1 : -1)) event.preventDefault(); else if (!pending) void request(true);
        } else if (event.key === 'Enter' && active && eligible(active) && popup?.visible) {
            if (lease.handlePress(event)) commit(active);
        } else if (event.key === 'Escape' && (intent || popup?.visible || popup?.pending)) {
            if (lease.handlePress(event)) dismiss('escape');
        } else if (event.key === 'Tab') dismiss('tab');
        else if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) { active = undefined; claim(); }
    }, { signal: abort.signal, capture: true });
    const rowAt = (target: EventTarget | null) => target instanceof Node && current() ? placements?.rows.find(row => row.element.contains(target) && eligibleRow(row)) : undefined;
    options.listbox.addEventListener('pointerdown', event => {
        pointer = undefined;
        if (event.defaultPrevented || !event.isPrimary || event.pointerType !== 'mouse' || event.button !== 0 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || !focused()) return;
        const row = rowAt(event.target); if (!row) return;
        pointer = { row, generation, id: event.pointerId, x: event.clientX, y: event.clientY, ended: false }; event.preventDefault();
    }, { signal: abort.signal });
    options.listbox.addEventListener('pointermove', event => {
        if (pointer && (event.pointerId !== pointer.id || Math.hypot(event.clientX - pointer.x, event.clientY - pointer.y) > 8)) pointer = undefined;
    }, { signal: abort.signal });
    options.listbox.addEventListener('pointerup', event => {
        if (pointer && event.pointerId === pointer.id && rowAt(event.target) === pointer.row && pointer.generation === generation) pointer.ended = true;
        else pointer = undefined;
    }, { signal: abort.signal });
    for (const event of ['pointercancel', 'lostpointercapture', 'scroll']) options.listbox.addEventListener(event, () => { pointer = undefined; }, { signal: abort.signal, capture: true });
    options.listbox.addEventListener('click', event => {
        const row = rowAt(event.target), armed = pointer; pointer = undefined;
        if (event.defaultPrevented || event.button !== 0 || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || !row || !focused()) return;
        if (event.detail === 0 || armed?.ended && armed.generation === generation && armed.row === row) { event.preventDefault(); commit(row); }
    }, { signal: abort.signal });
    editor.addEventListener('select', () => { active = undefined; claim(); }, { signal: abort.signal });
    editor.addEventListener('focusout', () => { dismiss('focus'); }, { signal: abort.signal });
    void request(false); updateValidity();
    return {
        get visible() { return !!popup?.visible; }, get pending() { return pending; },
        get active() { return active?.native.source; }, get committed() { updateValidity(); return committed?.source; },
        refresh: () => request(false), reconcile: () => { popup?.reconcile(); claim(); updateValidity(); }, dismiss,
        disconnect() {
            if (disposed) return; disposed = true; ++generation; pending = false;
            abort.abort(); clearProof(); unsubscribePlacement?.(); placements?.release();
            for (const release of [...releases]) release(); releases.length = 0;
            popup?.disconnect(); lease.release(); validity.release();
        },
    };
}
