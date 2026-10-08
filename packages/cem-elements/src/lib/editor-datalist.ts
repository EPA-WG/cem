import { isCemDatalistInput, isCemDatalistPlacements, type CemDatalistPlacements } from './suggestions-placements.js';

export interface CemEditorDatalistRequest {
    revision: number;
    current(): boolean;
    placements: CemDatalistPlacements;
}
export interface CemEditorDatalistClaim {
    readonly valid: boolean;
    set(request: CemEditorDatalistRequest): boolean;
    refresh(): void;
    clear(): void;
    dispose(): void;
    preserves(name: string): boolean;
}
/** The provider owns only `list`; it never installs custom popup semantics or edits the value. */
export function createCemEditorDatalistClaim(editor: HTMLElement, eligible: () => boolean, revision: () => number,
    invalidated: () => void): CemEditorDatalistClaim {
    let request: Readonly<CemEditorDatalistRequest> | undefined, applied: string | undefined;
    let stop: (() => void) | undefined, disposed = false, checking = false;
    // Even a later author write of the same ID takes ownership. Drain our own writes
    // synchronously, leaving all externally queued list mutations visible here.
    const authored = (records: MutationRecord[]) => {
        if (applied !== undefined && records.some(record => record.target === editor && record.attributeName === 'list')) applied = undefined;
    };
    const clear = () => {
        authored(observer.takeRecords());
        const previous = request; request = undefined; stop?.(); stop = undefined;
        previous?.placements.release();
        if (applied !== undefined && editor.getAttribute('list') === applied) editor.removeAttribute('list');
        applied = undefined; observer.takeRecords();
    };
    const admitted = (value: CemEditorDatalistRequest) => {
        try {
            return !disposed && eligible() && Number.isSafeInteger(value.revision) && revision() === value.revision
                && typeof value.current === 'function' && value.current() && isCemDatalistPlacements(value.placements)
                && value.placements.editor === editor && value.placements.current() && isCemDatalistInput(editor)
                && editor.getAttribute('role') !== 'combobox' && !editor.hasAttribute('aria-expanded')
                && !editor.hasAttribute('aria-activedescendant') && !editor.hasAttribute('aria-autocomplete');
        } catch { return false; }
    };
    const refresh = () => {
        if (!request || checking) return;
        checking = true;
        try {
            authored(observer.takeRecords());
            if (!admitted(request) || applied === undefined || editor.getAttribute('list') !== applied) { clear(); invalidated(); }
        } finally { checking = false; }
    };
    const observer = new MutationObserver(records => { authored(records); refresh(); });
    observer.observe(editor.getRootNode(), { subtree: true, childList: true, attributes: true, characterData: true,
        attributeFilter: ['list', 'id', 'type', 'multiple', 'disabled', 'readonly', 'value', 'label', 'hidden',
            'role', 'aria-expanded', 'aria-activedescendant', 'aria-autocomplete'] });
    return {
        get valid() { refresh(); return !!request; },
        refresh, clear,
        preserves(name) { refresh(); return name === 'list' && !!request; },
        set(next) {
            refresh();
            const captured = Object.freeze({ ...next });
            if (!admitted(captured) || (request ? editor.getAttribute('list') !== applied : editor.hasAttribute('list'))) {
                const active = !!request; clear(); if (active) invalidated(); return false;
            }
            const previous = request; stop?.(); stop = undefined;
            request = captured; applied = captured.placements.datalist.id;
            editor.setAttribute('list', applied); observer.takeRecords();
            if (previous && previous.placements !== captured.placements) previous.placements.release();
            stop = captured.placements.subscribe(refresh);
            refresh(); return !!request;
        },
        dispose() { if (disposed) return; disposed = true; clear(); observer.disconnect(); },
    };
}
