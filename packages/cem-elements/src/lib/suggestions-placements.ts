import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS, type CemValueArtifactLimits } from './native-values.js';
import { getCemEditorProvider, type CemEditorProvider } from './form-control-capability.js';
import { isCemNativeSuggestionsBinding, isCemNativeSuggestionRow, isCemNativeSuggestionRowFor, type CemNativeSuggestionRow, type CemNativeSuggestionsBinding } from './native-suggestions-publication.js';

export interface CemSuggestionsPlacementLease { dispose(): void }
export interface CemSuggestionsPlacementDescription {
    producer: string;
    revision: string;
    current(): boolean;
    element: HTMLElement;
    kind: 'editor' | 'listbox' | 'row';
    row?: CemNativeSuggestionRow;
}
export interface CemSuggestionsPlacedRow { readonly native: CemNativeSuggestionRow; readonly element: HTMLElement }
export interface CemSuggestionsPlacements {
    readonly binding: CemNativeSuggestionsBinding;
    readonly provider: CemEditorProvider;
    readonly editorHost: HTMLElement;
    readonly listbox: HTMLElement;
    readonly rows: readonly CemSuggestionsPlacedRow[];
    current(): boolean;
    subscribe(invalidate: () => void): () => void;
    release(): void;
}
type Property = 'editor-for' | 'aria-controls' | 'aria-activedescendant';
interface Entry { description: Readonly<CemSuggestionsPlacementDescription>; id: string; active: boolean }
interface Grant { requester: string; entry: Entry; properties: Set<Property>; active: boolean }
const token = (value: string) => typeof value === 'string' && !!value && value.length <= 1024 && !/[\s\p{Cc}]/u.test(value);

/** Host-only original-row bridge. Names and IDs never create its grants. */
export class CemSuggestionsPlacementCoordinator {
    private readonly entries = new Map<CemSuggestionsPlacementLease, Entry>();
    private readonly grants = new Set<Grant>();
    private readonly admissions = new Set<{ check(): boolean; invalidate(): void }>();
    private readonly observer: MutationObserver;
    private sequence = 0;
    private disposed = false;
    constructor(private readonly root: Document | ShadowRoot, private readonly limits: CemValueArtifactLimits = DEFAULT_CEM_VALUE_ARTIFACT_LIMITS) {
        this.observer = new MutationObserver(() => this.refresh());
        this.observer.observe(root, { subtree: true, childList: true, attributes: true,
            attributeFilter: ['id', 'role', 'popover', 'type', 'list'] });
    }
    register(description: CemSuggestionsPlacementDescription): CemSuggestionsPlacementLease {
        if (this.disposed || this.entries.size >= this.limits.maxValues || !token(description.producer) || !token(description.revision)
            || typeof description.current !== 'function' || !['editor', 'listbox', 'row'].includes(description.kind)
            || description.kind === 'row' && (!isCemNativeSuggestionRow(description.row) || !description.row.valid)
            || description.kind !== 'row' && description.row !== undefined) throw new TypeError('Invalid suggestions placement registration');
        for (const entry of this.entries.values()) if (entry.active && entry.description.element === description.element) {
            throw new Error('A suggestions placement already owns this endpoint');
        }
        const entry: Entry = { description: Object.freeze({ ...description }),
            id: description.element.getAttribute('id') ?? `${description.producer}-suggestion-ref-${++this.sequence}`, active: true };
        if (!this.ready(entry)) throw new Error('Suggestions placements require exact committed endpoints');
        const lease = Object.freeze({ dispose: () => {
            if (!entry.active) return;
            entry.active = false; this.entries.delete(lease);
            for (const grant of this.grants) if (grant.entry === entry) { grant.active = false; this.grants.delete(grant); }
            this.refresh();
        } });
        this.entries.set(lease, entry); return lease;
    }
    grant(requester: string, lease: CemSuggestionsPlacementLease, properties: readonly Property[]): () => void {
        const entry = this.entries.get(lease);
        const allowed: Record<CemSuggestionsPlacementDescription['kind'], Property> = {
            editor: 'editor-for', listbox: 'aria-controls', row: 'aria-activedescendant',
        };
        if (this.disposed || !entry?.active || !token(requester) || !properties.length || this.grants.size >= this.limits.maxValues
            || properties.some(property => property !== allowed[entry.description.kind])) throw new TypeError('Invalid suggestions placement grant');
        const grant: Grant = { requester, entry, properties: new Set(properties), active: true }; this.grants.add(grant);
        return () => { if (grant.active) { grant.active = false; this.grants.delete(grant); this.refresh(); } };
    }
    prepare(binding: CemNativeSuggestionsBinding, editorLease: CemSuggestionsPlacementLease, listboxLease: CemSuggestionsPlacementLease,
        rowLeases: readonly CemSuggestionsPlacementLease[]): CemSuggestionsPlacements {
        const editor = this.entries.get(editorLease), listbox = this.entries.get(listboxLease), rowEntries = rowLeases.map(lease => this.entries.get(lease));
        if (this.disposed || !isCemNativeSuggestionsBinding(binding) || !binding.valid || !editor || !listbox || rowLeases.length > this.limits.maxValues
            || editor.description.kind !== 'editor' || listbox.description.kind !== 'listbox'
            || rowEntries.some(entry => !entry || entry.description.kind !== 'row' || !entry.description.row
                || !isCemNativeSuggestionRowFor(entry.description.row, binding))
            || new Set(rowEntries.map(entry => entry?.description.row?.source)).size !== rowEntries.length) {
            throw new Error('Invalid original-row placement mapping');
        }
        const provider = getCemEditorProvider(editor.description.element);
        if (!provider) throw new Error('Suggestions placement has no exact editor provider');
        const control = provider.control;
        const selected = [editor, listbox, ...rowEntries as Entry[]];
        const rows = (rowEntries as Entry[]).map(entry => Object.freeze({ native: entry.description.row as CemNativeSuggestionRow, element: entry.description.element }));
        const check = () => {
            try {
                const ids = this.idIndex();
                const editorTargets = new Set<Entry>();
                let reverse = false;
                for (const grant of this.grants) if (grant.active) {
                    if (grant.requester === editor.description.producer) editorTargets.add(grant.entry);
                    if (grant.requester === listbox.description.producer && grant.entry === editor) reverse = true;
                }
                return !!ids && !this.disposed && binding.valid && selected.every(entry => this.ready(entry, ids))
                    && getCemEditorProvider(editor.description.element) === provider && provider.control === control && control instanceof HTMLInputElement
                    && reverse && editorTargets.has(listbox)
                    && (rowEntries as Entry[]).every(entry => editorTargets.has(entry)
                        && listbox.description.element.contains(entry.description.element));
            } catch { return false; }
        };
        if (!check()) throw new Error('Suggestions requires current placement grants in both directions');
        const inserted: Entry[] = [];
        try {
            for (const entry of selected) if (!entry.description.element.hasAttribute('id')) {
                entry.description.element.id = entry.id; inserted.push(entry);
            }
            if (!check()) throw new Error('Suggestions placements changed during ID reservation');
        } catch (error) {
            for (const entry of inserted) if (entry.description.element.id === entry.id) entry.description.element.removeAttribute('id');
            throw error;
        }
        let expired = false, released = false;
        const listeners = new Set<() => void>();
        const invalidate = () => {
            if (expired) return;
            expired = true; for (const listener of [...listeners]) { try { listener(); } catch { /* Revoke every consumer. */ } }
        };
        const stopBinding = binding.subscribe(invalidate);
        const admission = { check, invalidate }; this.admissions.add(admission);
        return Object.freeze({ binding, provider, editorHost: editor.description.element, listbox: listbox.description.element, rows: Object.freeze(rows),
            current: () => { if (!expired && !check()) invalidate(); return !expired && !released; },
            subscribe: (listener: () => void) => { if (expired || released) { listener(); return () => undefined; } listeners.add(listener); return () => listeners.delete(listener); },
            release: () => { if (released) return; released = true; stopBinding(); this.admissions.delete(admission); invalidate(); listeners.clear(); },
        });
    }
    refresh(): void { for (const admission of [...this.admissions]) if (!admission.check()) admission.invalidate(); }
    dispose(): void {
        if (this.disposed) return;
        this.disposed = true; this.observer.disconnect(); this.refresh(); this.entries.clear(); this.grants.clear(); this.admissions.clear();
    }
    private idIndex(): Map<string, Element | null> | undefined {
        const ids = new Map<string, Element | null>(); let visited = 0;
        for (const node of this.root.querySelectorAll('[id]')) {
            if (++visited > this.limits.maxValues) return;
            ids.set(node.id, ids.has(node.id) ? null : node);
        }
        return ids;
    }
    private ready(entry: Entry, ids = this.idIndex()): boolean {
        const { element, kind, row, current } = entry.description;
        if (!ids || !entry.active || !token(entry.id) || !current() || !(element instanceof HTMLElement) || !element.isConnected
            || element.getRootNode() !== this.root || element.hasAttribute('id') && element.id !== entry.id
            || ids.has(entry.id) && ids.get(entry.id) !== element) return false;
        if (kind === 'row') return !!row?.valid && element.getAttribute('role') === 'option';
        if (kind === 'listbox') return element.getAttribute('role') === 'listbox' && element.getAttribute('popover') === 'manual';
        const control = getCemEditorProvider(element)?.control;
        return control instanceof HTMLInputElement && control.type === 'text' && !control.hasAttribute('list') && !element.hasAttribute('list');
    }
}
