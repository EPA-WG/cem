import { nativeSurfaceVisible } from './surface-session.js';

/** Prepared native publication supplies endpoints; this hook rechecks its host-issued authority. */
export interface CemEditorAttributeRequest {
    revision: number;
    current(): boolean;
    listbox: HTMLElement;
    row?: HTMLElement;
    expanded: boolean;
    autocomplete: 'list' | 'none';
}
export interface CemEditorAttributeClaim {
    readonly valid: boolean;
    set(request: CemEditorAttributeRequest): boolean;
    refresh(): void;
    clear(): void;
    dispose(): void;
    preserves(name: string): boolean;
}
type ScalarClaim = { saved: string | null; applied: string | null };
const names = ['role', 'aria-haspopup', 'aria-autocomplete', 'aria-expanded', 'aria-activedescendant'];
const token = (value: string) => !!value && !/[\s\p{Cc}]/u.test(value);
const tokens = (value: string | null) => value?.split(/\s+/).filter(Boolean) ?? [];
function write(node: Element, name: string, value: string | null): void {
    if (node.getAttribute(name) === value) return;
    if (value === null) node.removeAttribute(name); else node.setAttribute(name, value);
}
function endpoint(node: HTMLElement, editor: HTMLElement): boolean {
    if (!(node instanceof HTMLElement) || !token(node.id) || !node.isConnected || node.getRootNode() !== editor.getRootNode()) return false;
    const same = [...(editor.getRootNode() as ParentNode).querySelectorAll('[id]')].filter(other => other.id === node.id);
    return same.length === 1 && same[0] === node;
}
/** Provider-only writes: scalar claims preserve newer authors; controls owns only added tokens. */
export function createCemEditorAttributeClaim(editor: HTMLElement, eligible: () => boolean, revision: () => number,
    invalidated: () => void): CemEditorAttributeClaim {
    const scalars = new Map<string, ScalarClaim>();
    let controls: { saved: string | null; applied: string; added?: string } | undefined;
    let request: CemEditorAttributeRequest | undefined, closingPanel: HTMLElement | undefined, disposed = false, checking = false;
    const clear = () => {
        request = undefined;
        for (const [name, claim] of scalars) if (editor.getAttribute(name) === claim.applied) write(editor, name, claim.saved);
        scalars.clear();
        if (controls) {
            const current = editor.getAttribute('aria-controls');
            if (current === controls.applied) write(editor, 'aria-controls', controls.saved);
            else if (controls.added && tokens(current).includes(controls.added)) {
                const remaining = tokens(current).filter(value => value !== controls?.added);
                write(editor, 'aria-controls', remaining.length ? remaining.join(' ') : null);
            }
        }
        controls = undefined;
    };
    const admitted = (value: CemEditorAttributeRequest) => {
        const { listbox, row } = value;
        return !disposed && typeof value.expanded === 'boolean' && Number.isSafeInteger(value.revision)
            && typeof value.current === 'function' && eligible() && revision() === value.revision && value.current()
            && editor instanceof HTMLInputElement && editor.type === 'text' && !editor.hasAttribute('list')
            && endpoint(listbox, editor) && listbox.getAttribute('role') === 'listbox' && listbox.getAttribute('popover') === 'manual'
            && ['list', 'none'].includes(value.autocomplete)
            && (value.expanded ? nativeSurfaceVisible(listbox) && editor.ownerDocument.activeElement === editor
                : !nativeSurfaceVisible(listbox) || closingPanel === listbox)
            && (!row || value.expanded && endpoint(row, editor) && listbox.contains(row) && row.getAttribute('role') === 'option'
                && !row.closest('[hidden],[inert],[disabled],[aria-disabled="true"]') && row.getClientRects().length > 0
                && !['hidden', 'collapse'].includes(getComputedStyle(row).visibility));
    };
    const matches = () => [...scalars].every(([name, claim]) => editor.getAttribute(name) === claim.applied)
        && scalars.get('aria-activedescendant')?.applied === (request?.row?.id ?? null)
        && (!controls || tokens(editor.getAttribute('aria-controls')).includes(request?.listbox.id ?? ''));
    const refresh = () => {
        if (!request || checking) return;
        checking = true;
        try { if (!admitted(request) || !matches()) { clear(); invalidated(); } }
        finally { checking = false; }
    };
    const observer = new MutationObserver(refresh);
    observer.observe(editor.ownerDocument, { subtree: true, childList: true, attributes: true,
        attributeFilter: [...names, 'aria-controls', 'id', 'hidden', 'inert', 'disabled', 'readonly', 'type', 'list', 'style', 'class', 'aria-disabled', 'popover'] });
    const abort = new AbortController();
    editor.ownerDocument.addEventListener('focusin', refresh, { signal: abort.signal });
    // Native hide releases the active relationship in the same event turn.
    editor.ownerDocument.addEventListener('beforetoggle', event => {
        if (!request || !event.isTrusted || event.target !== request.listbox) return;
        if ((event as ToggleEvent).newState === 'closed') {
            closingPanel = request.listbox;
            request = Object.freeze({ ...request, expanded: false, row: undefined });
            for (const [name, value] of [['aria-expanded', 'false'], ['aria-activedescendant', null]] as const) {
                const claim = scalars.get(name);
                if (claim && editor.getAttribute(name) === claim.applied) { claim.applied = value; write(editor, name, value); }
            }
        }
        queueMicrotask(() => { closingPanel = undefined; refresh(); });
    }, { signal: abort.signal, capture: true });
    return {
        get valid() { refresh(); return !!request; },
        refresh, clear,
        preserves(name) { refresh(); return !!request && (scalars.has(name) || name === 'aria-controls' && !!controls); },
        set(next) {
            const captured = Object.freeze({ ...next });
            const values: Record<string, string | null> = { role: 'combobox', 'aria-haspopup': 'listbox', 'aria-autocomplete': captured.autocomplete,
                'aria-expanded': String(captured.expanded), 'aria-activedescendant': captured.row?.id ?? null };
            const allowed = admitted(captured) && (!request || matches()) && names.every(name => {
                const value = editor.getAttribute(name), previous = scalars.get(name);
                return previous ? value === previous.applied : value === null || value === values[name];
            });
            if (!allowed) { const active = !!request; clear(); if (active) invalidated(); return false; }
            const old = editor.getAttribute('aria-controls');
            const clean = tokens(old).filter(value => value !== controls?.added);
            const saved = controls && old === controls.applied ? controls.saved : controls?.added ? clean.length ? clean.join(' ') : null : old;
            const id = captured.listbox.id, added = clean.includes(id) ? undefined : id;
            if (added) clean.push(added);
            for (const name of names) {
                const previous = scalars.get(name);
                scalars.set(name, { saved: previous ? previous.saved : editor.getAttribute(name), applied: values[name] });
            }
            controls = { saved, applied: clean.join(' '), added }; request = captured;
            for (const name of names) write(editor, name, values[name]);
            write(editor, 'aria-controls', controls.applied);
            // Revocation/reentrant host hooks cannot leave a partly admitted claim visible.
            refresh(); return !!request;
        },
        dispose() { if (disposed) return; disposed = true; clear(); observer.disconnect(); abort.abort(); },
    };
}
