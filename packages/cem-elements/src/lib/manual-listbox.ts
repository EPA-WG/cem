import { getCemEditorProvider, isCemEditorLeaseFor, type CemEditorLease } from './form-control-capability.js';
import { createCemSurfaceGeometryLease, nativeSurfaceGeometry } from './popup-controller.js';
import { observeInteractionReferences, reportInteractionReference } from './interaction-reference.js';
import { assertCemSurfaceOwnerAvailable, createCemSurfaceSession, observeCemSurfaceDismissalRegion, type CemSurfaceLifetime, type CemSurfaceSession } from './surface-session.js';

export interface CemManualListboxLifecycle {
    /** Trusted consumer's side-effect-free relationship/placement check; never inferred from IDs or adjacency. */
    current(): boolean;
    /** Source, query and row publication are ready at this consumer lifecycle stage. */
    ready(): boolean;
    /** Revoke any outstanding opening when the captured relationship/revision changes. */
    subscribe(invalidate: () => void): () => void;
}
interface Options {
    host: HTMLElement;
    editorHost: HTMLElement;
    lifecycle: CemManualListboxLifecycle;
    /** One attachment shares its provider lease with attributes, visibility and commits. */
    editorLease?: CemEditorLease;
    ancestors?: readonly CemSurfaceLifetime[];
    onVisibility?(open: boolean, reason: string): void;
}
export interface CemManualListboxController extends CemSurfaceSession { open(): boolean }
const noninteractive = 'button,a[href],input,select,textarea,summary,iframe,object,embed,audio[controls],video[controls],[contenteditable]:not([contenteditable="false"]),[tabindex],[autofocus]';

/** Package-private semantic delegate. It neither selects rows nor writes values or editor ARIA. */
export function connectCemManualListbox(owner: HTMLElement, options: Options): CemManualListboxController {
    assertCemSurfaceOwnerAvailable(owner);
    const { host, editorHost, lifecycle } = options;
    const provider = getCemEditorProvider(editorHost), editor = provider?.control;
    if (!provider || !(editor instanceof HTMLInputElement)) throw new TypeError('Manual listbox requires an exact native editor provider');
    if (options.editorLease && !isCemEditorLeaseFor(options.editorLease, provider)) throw new TypeError('Manual listbox requires a verified editor lease');
    const ancestors = Object.freeze([...(options.ancestors ?? [])]);
    const abort = new AbortController();
    // Acquire surface ownership before the editor claim: a competing surface must not perturb another provider.
    let session: CemSurfaceSession | undefined;
    const geometry = createCemSurfaceGeometryLease(host, owner, () => session?.reconcile());
    const lease = options.editorLease ?? provider.lease({});
    const releaseLease = () => { if (!options.editorLease) lease.release(); };
    const placement = () => host.getAttribute('placement') ?? 'block-end start';
    const admittedAncestors = (node: HTMLElement) => {
        for (let parent = node.parentElement; parent; parent = parent.parentElement) {
            if (parent.matches(':popover-open,dialog[open]') && !ancestors.some(a => a.owner === parent && a.current())) return false;
        }
        return true;
    };
    const check = () => {
        const profile = owner.localName !== 'dialog' && owner.getAttribute('popover') === 'manual'
            && typeof owner.showPopover === 'function' && owner.getAttribute('role') === 'listbox'
            && !owner.matches('[tabindex],[autofocus],[contenteditable]:not([contenteditable="false"])') && !owner.querySelector(noninteractive)
            && editor.type === 'text' && !editor.hasAttribute('list')
            && !host.hasAttribute('mode') && !host.hasAttribute('kind') && placement() !== 'center';
        reportInteractionReference(host, profile ? undefined : 'interaction-profile-conflict', 'listbox-profile');
        if (!profile || !geometry.valid || !lease.valid || getCemEditorProvider(editorHost) !== provider || provider.control !== editor
            || !provider.editable || provider.composing || editor.ownerDocument.activeElement !== editor
            || !editor.isConnected || !editorHost.isConnected || editor.ownerDocument !== owner.ownerDocument
            || editor.closest('[hidden]:not([hidden="until-found"]),[inert]') || !lifecycle.current() || !lifecycle.ready()
            || !admittedAncestors(editor) || !admittedAncestors(owner)) return false;
        const modal = editor.closest('dialog:modal');
        if (modal && !modal.contains(owner)) return false;
        const prepared = nativeSurfaceGeometry(host, owner, editor, undefined, placement());
        const rect = prepared?.rect, bounds = prepared?.bounds;
        const valid = !!prepared && prepared.anchor === editor && !!rect && !!bounds
            && rect.right > rect.left && rect.bottom > rect.top && bounds.right - bounds.left > 8 && bounds.bottom - bounds.top > 8
            && rect.right > bounds.left + 4 && rect.left < bounds.right - 4 && rect.bottom > bounds.top + 4 && rect.top < bounds.bottom - 4;
        if (!valid) reportInteractionReference(host, 'interaction-anchor-unavailable', 'geometry');
        return valid;
    };
    try {
        session = createCemSurfaceSession(host, owner, {
            profile: 'manual-listbox', ancestors,
            capture() {
                if (!check()) return;
                const revision = provider.revision;
                return () => provider.revision === revision && check();
            },
            fit() {
                if (!check() || !geometry.fit(editor, undefined, placement(), true)) return false;
                const rect = owner.getBoundingClientRect(), style = getComputedStyle(owner);
                return rect.width > 0 && rect.height > 0 && style.display !== 'none' && !['hidden', 'collapse'].includes(style.visibility);
            },
            reset: () => geometry.reset(), onVisibility: options.onVisibility,
        });
    } catch (error) { releaseLease(); geometry.release(); throw error; }
    const current = session;
    const releases: (() => void)[] = [];
    try {
        releases.push(observeInteractionReferences(host, () => current.reconcile()));
        releases.push(lifecycle.subscribe(() => current.dismiss('revision')));
        releases.push(provider.subscribe(update => {
            if (update.cause === 'claims' && lease.valid) return;
            current.dismiss(update.cause);
        }));
    } catch (error) {
        for (const release of releases) release();
        try { current.disconnect(); } finally { releaseLease(); geometry.release(); }
        throw error;
    }
    const inside = (target: EventTarget | null) => target instanceof Node && (target === editor || owner.contains(target));
    const stopPointers = observeCemSurfaceDismissalRegion(owner.ownerDocument, inside,
        () => current.visible || current.pending ? current.generation : undefined, () => current.dismiss('outside'));
    editorHost.addEventListener('keydown', event => {
        if (event.target !== editor || event.defaultPrevented || provider.compositionOwned(event) || !lease.valid) return;
        if (event.key === 'Escape' && (current.visible || current.pending) && lease.handlePress(event)) current.dismiss('escape');
        else if (event.key === 'Tab' && (current.visible || current.pending)) {
            queueMicrotask(() => { if (!event.defaultPrevented) current.dismiss('tab'); });
        }
    }, { signal: abort.signal });
    owner.ownerDocument.addEventListener('focusin', event => {
        if (event.target !== editor) current.dismiss('focus');
    }, { signal: abort.signal });
    editor.addEventListener('focusout', () => queueMicrotask(() => {
        if (editor.ownerDocument.activeElement !== editor) current.dismiss('focus');
    }), { signal: abort.signal });
    let disposed = false;
    return {
        get visible() { return current.visible; }, get pending() { return current.pending; }, get generation() { return current.generation; },
        prepare: () => current.prepare(), complete: attempt => current.complete(attempt),
        open() { const attempt = current.prepare(); return !!attempt && current.complete(attempt); },
        dismiss: reason => current.dismiss(reason), reconcile: () => current.reconcile(),
        disconnect() {
            if (disposed) return;
            disposed = true; abort.abort(); for (const release of releases) release(); stopPointers();
            try { current.disconnect(); } finally { releaseLease(); geometry.release(); }
        },
    };
}
