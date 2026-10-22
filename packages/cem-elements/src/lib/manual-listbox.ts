import type { CemElementRuntime } from './cem-elements.js';
import type { CemProcessingHost } from './internal/runtime-support/processing-host.js';
import { registerCemProviderHandoff, validateCemProviderSource } from './checkpoint-provider-handoff.js';
import type { CheckpointEndpoint, CheckpointManualListbox } from './checkpoint-interactions.js';
import { getCemEditorProvider, registerCemEditorCheckpoint, isCemEditorLeaseFor, subscribeCemEditorCheckpoint, type CemEditorLease } from './form-control-capability.js';
import { createCemSurfaceGeometryLease, nativeSurfaceGeometry } from './popup-controller.js';
import { observeInteractionReferences, reportInteractionReference } from './interaction-reference.js';
import { assertCemSurfaceOwnerAvailable, createCemSurfaceSession, observeCemSurfaceDismissalRegion, type CemSurfaceLifetime, type CemSurfaceSession } from './surface-session.js';

export interface CemManualListboxLifecycle {
    /** Trusted consumer's side-effect-free relationship/placement check; never inferred from IDs or adjacency. */
    current(): boolean;
    /** Source, query and row publication are ready at this consumer lifecycle stage. */
    ready(): boolean;
    /** Host opts into inert reset/reacquisition; callbacks remain current admission policy. */
    checkpoint?: boolean;
    /** Original host policy reacquires sources on the already-bound fresh owner.
     * Returning a lifecycle grants no old native handle or publication. */
    resume?(host: CemProcessingHost): Promise<CemManualListboxLifecycle>;
    /** Revoke any outstanding opening when the captured relationship/revision changes. */
    subscribe(invalidate: () => void): () => void;
}
interface Options {
    runtime?: CemElementRuntime;
    host: HTMLElement;
    editorHost: HTMLElement;
    lifecycle: CemManualListboxLifecycle;
    /** One attachment shares its provider lease with attributes, visibility and commits. */
    editorLease?: CemEditorLease;
    ancestors?: readonly CemSurfaceLifetime[];
    onVisibility?(open: boolean, reason: string): void;
}
export interface CemManualListboxController extends CemSurfaceSession { open(): boolean }
const pauses = new WeakMap<CemManualListboxController, () => () => void>();
export function parkCemManualListbox(controller: CemManualListboxController): () => void {
    const park = pauses.get(controller); if (!park) throw Error('cem.checkpoint.resume.listbox'); return park();
}
const checkpointDelegates = new WeakMap<HTMLElement, { editor: HTMLElement; panel: HTMLElement; capture(): Omit<CheckpointManualListbox, 'panel' | 'editor'> | undefined }>();
export function hasCemManualListboxCheckpoint(host: HTMLElement): boolean { return checkpointDelegates.has(host); }
export function cemManualListboxCheckpoint(host: HTMLElement, resolve: (node: HTMLElement) => CheckpointEndpoint | undefined): CheckpointManualListbox | undefined {
    const delegate = checkpointDelegates.get(host), state = delegate?.capture();
    const panel = delegate && resolve(delegate.panel), editor = delegate && resolve(delegate.editor);
    return state && panel && editor ? { ...state, panel, editor } : undefined;
}
const noninteractive = 'button,a[href],input,select,textarea,summary,iframe,object,embed,audio[controls],video[controls],[contenteditable]:not([contenteditable="false"]),[tabindex],[autofocus]';

/** Package-private semantic delegate. It neither selects rows nor writes values or editor ARIA. */
export function connectCemManualListbox(owner: HTMLElement, options: Options): CemManualListboxController {
    assertCemSurfaceOwnerAvailable(owner);
    if (!options.editorLease && options.lifecycle.checkpoint && checkpointDelegates.has(options.host)) throw new TypeError('Duplicate standalone delegate checkpoint owner');
    const { host, editorHost } = options;
    let lifecycle = options.lifecycle;
    let stopLifecycle: (() => void) | undefined;
    const provider = getCemEditorProvider(editorHost), editor = provider?.control;
    if (!provider || !(editor instanceof HTMLInputElement)) throw new TypeError('Manual listbox requires an exact native editor provider');
    if (options.editorLease && !isCemEditorLeaseFor(options.editorLease, provider)) throw new TypeError('Manual listbox requires a verified editor lease');
    const ancestors = Object.freeze([...(options.ancestors ?? [])]);
    const abort = new AbortController();
    // Acquire surface ownership before the editor claim: a competing surface must not perturb another provider.
    let session: CemSurfaceSession | undefined, paused = false;
    const geometry = createCemSurfaceGeometryLease(host, owner, () => session?.reconcile());
    const checkpointOwner = {};
    const lease = options.editorLease ?? provider.lease(checkpointOwner);
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
            profile: 'manual-listbox', ancestors, paused: () => paused,
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
        stopLifecycle = lifecycle.subscribe(() => { if (!paused) current.dismiss('revision'); });
        releases.push(() => stopLifecycle?.());
        releases.push(subscribeCemEditorCheckpoint(provider, lease, update => {
            if (paused) return;
            if ((update.cause === 'claims' || update.cause === 'availability'
                || update.cause === 'attribute-claims' && lease.attributes.valid) && lease.valid) { current.reconcile(); return; }
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
    if (!options.editorLease && lifecycle.checkpoint) {
        const capture = () => {
            if (disposed || lifecycle.checkpoint !== true || current.pending || !host.isConnected || !owner.isConnected || !editorHost.isConnected
                || !lease.valid || lease.attributes.valid || lease.datalist.valid || provider.composing
                || getCemEditorProvider(editorHost) !== provider || provider.control !== editor
                || !lifecycle.current() || !lifecycle.ready() || host.hasAttribute('anchor') || host.hasAttribute('boundary')
                || ancestors.length || editor.type !== 'text' || editor.hasAttribute('list') || host.hasAttribute('mode') || host.hasAttribute('kind') || placement() === 'center'
                || owner.matches('[tabindex],[autofocus],[contenteditable]:not([contenteditable="false"])') || owner.getAttribute('role') !== 'listbox' || owner.getAttribute('popover') !== 'manual'
                || owner.querySelector(noninteractive) || current.visible !== owner.matches(':popover-open')
                || current.visible && !check()) return undefined;
            return { providerRevision: provider.revision, visible: current.visible, presentation: 'reset-on-resume' as const, disposition: 'reacquire' as const };
        };
        const record = { editor: editorHost, panel: owner, capture };
        checkpointDelegates.set(host, record);
        releases.push(() => { if (checkpointDelegates.get(host) === record) checkpointDelegates.delete(host); });
        releases.push(registerCemEditorCheckpoint(provider, { host, owner: checkpointOwner, lease, current: () => !!capture(), claim: () => 'none' }));
        releases.push(registerCemProviderHandoff(host, () => {
            const captured = capture(), renew = lifecycle.resume?.bind(lifecycle);
            if (!captured || !renew) return undefined;
            return { editor: editorHost, presentation: owner, current: () => !disposed && (paused || !!capture()), park() {
                if (!capture() || paused) throw Error('cem.checkpoint.resume.listbox');
                return parkCemManualListbox(controller);
            }, async renew(freshHost, saved) {
                if (!paused || !saved.manualListbox || saved.manualListbox.providerRevision !== provider.revision)
                    throw Error('cem.checkpoint.resume.listbox');
                const next = await renew(freshHost);
                if (!next || next.checkpoint !== true || !next.current() || !next.ready()) throw Error('cem.checkpoint.resume.source-authority');
                if (saved.nativePublication) {
                    if (!options.runtime) throw Error('cem.checkpoint.resume.source-authority');
                    options.runtime.refreshElementReferences(host); await options.runtime.whenRenderSettled(host);
                    await validateCemProviderSource(freshHost, options.runtime.renderedSuggestionsFor(host)?.binding.checkpointSource(), saved.nativePublication.source);
                }
                stopLifecycle?.(); lifecycle = next;
                stopLifecycle = lifecycle.subscribe(() => { if (!paused) current.dismiss('revision'); });
                // This standalone delegate explicitly opted into picker reset.
                const closed = owner.matches(':popover-open') ? new Promise<void>(resolve => owner.addEventListener('toggle', () => resolve(), { once: true })) : undefined;
                current.dismiss('resume'); paused = false; current.reconcile();
                await closed;
                if (!capture()) throw Error('cem.checkpoint.resume.listbox');
            } };
        }));
    }
    const controller: CemManualListboxController = {
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
    pauses.set(controller, () => {
        if (disposed || paused || current.pending) throw Error('cem.checkpoint.resume.listbox');
        paused = true;
        return () => { if (!paused) return; paused = false; current.reconcile(); };
    });
    return controller;
}
