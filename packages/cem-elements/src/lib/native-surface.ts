import { captureCemMenuTaskRelay, type CemMenuTaskRelay } from './menu-task-relay.js';
import { getCemActionInvocation } from './action-command-capability.js';
import { interactionReference, observeInteractionReferences, reportInteractionReference } from './interaction-reference.js';
import { createCemSurfaceGeometryLease, nativeSurfaceGeometry, type CemSurfaceGeometryLease } from './popup-controller.js';
import { focusSurface, restoreSurfaceFocus } from './surface-references.js';
import { captureCemSurfaceInvocation, snapshotGeometryRect, type CemSurfaceInvocation } from './surface-invocation.js';
import type { CemProducedElementBehavior } from './cem-elements.js';
import { isCemEditorCompositionKey } from './form-control-capability.js';
import { CemSurfaceBody, ownsCemSurfaceBodyNode, surfaceExitAnimations } from './surface-body.js';
import { nativeSurfaceVisible as visible, registerCemSurfaceOwner } from './surface-session.js';
import { CemSurfaceControls } from './surface-controls.js';

export interface CemSurfacePreparation {
    readonly owner: HTMLElement;
    readonly host: HTMLElement;
    readonly invocation: CemSurfaceInvocation;
    readonly signal: AbortSignal;
}
export interface CemNativeSurfaceOptions {
    host?: HTMLElement;
    /** Shared host preparation; application data stays in its native lifecycle. */
    prepareBody?(preparation: CemSurfacePreparation): void | Promise<void>;
}
export type CemNativeSurfaceKind = 'dialog' | 'tooltip';
export interface CemNativeSurfaceController {
    readonly owner: HTMLElement;
    readonly host: HTMLElement;
    /** True means admitted; asynchronous preparation may still be pending. */
    open(invocation?: CemSurfaceInvocation): boolean;
    requestClose(reason?: string): boolean;
    disconnect(): void;
}
const controllers = new WeakMap<HTMLElement, { controller: NativeSurface; kind?: CemNativeSurfaceKind }>();
const defaultsInitialized = new WeakSet<HTMLElement>();
let sessions = 0;
function nativeDialog(node: HTMLElement): node is HTMLDialogElement { return node.localName === 'dialog'; }
function surfaceKind(owner: HTMLElement, host: HTMLElement): CemNativeSurfaceKind | undefined {
    const kind = host.getAttribute('kind');
    if (kind) return kind === 'dialog' || kind === 'tooltip' ? kind : undefined;
    return owner.getAttribute('role') === 'tooltip' ? 'tooltip' : nativeDialog(owner) || owner.getAttribute('role') === 'dialog' ? 'dialog' : undefined;
}
function retainInvocation(invocation: CemSurfaceInvocation): CemSurfaceInvocation {
    return Object.freeze({ ...invocation, geometry: invocation.geometry && Object.freeze({
        pointer: snapshotGeometryRect(invocation.geometry.pointer, true), selection: snapshotGeometryRect(invocation.geometry.selection),
    }) });
}
/** The stable native owner owns visibility; this adapter owns its transient session. */
class NativeSurface implements CemNativeSurfaceController {
    private readonly abort = new AbortController();
    private readonly releaseVisibility: () => void;
    private readonly releaseReferences: () => void;
    private readonly geometry: CemSurfaceGeometryLease;
    private readonly registeredKind?: CemNativeSurfaceKind;
    private readonly tooltipSources = new Map<HTMLElement, { abort: AbortController; focused: boolean; pointer: boolean; touch: boolean }>();
    private tooltipPointer = false;
    private tooltipDismissed = false;
    private tooltipAutomatic = false;
    private descriptionId?: string;
    private readonly describedSources = new Map<HTMLElement, string>();
    private invocation?: CemSurfaceInvocation;
    private relay?: CemMenuTaskRelay;
    private activation?: { source: HTMLElement; invocation: CemSurfaceInvocation };
    private phase = 'closed';
    private wasOpen = false;
    private closeReason = 'native';
    private restore = false;
    private prepared = false;
    private sessionId = '';
    private disposed = false;
    private timer?: ReturnType<typeof setTimeout>;
    private readonly body: CemSurfaceBody;
    readonly controls: CemSurfaceControls;
    private preparation?: AbortController;
    private generation = 0;
    private releaseBusy?: () => void;
    constructor(readonly owner: HTMLElement, readonly host: HTMLElement, private readonly preparationOptions: CemNativeSurfaceOptions) {
        this.controls = new CemSurfaceControls(owner, host);
        this.body = new CemSurfaceBody(owner, host, nodes => {
            const descendants = nodes.flatMap(node => node instanceof HTMLElement ? [node, ...node.querySelectorAll<HTMLElement>('*')] : []);
            for (const child of descendants.reverse()) controllers.get(child)?.controller.disconnect();
        }, node => this.controls.chrome(node));
        this.registeredKind = surfaceKind(owner, host);
        this.geometry = createCemSurfaceGeometryLease(host, owner, () => this.reposition());
        const options = { signal: this.abort.signal }, root = owner.getRootNode();
        try { this.releaseVisibility = registerCemSurfaceOwner(host, owner, this.registeredKind ?? 'invalid', () => { this.sync(); this.bindTooltipSources(); }); }
        catch (error) { this.geometry.release(); throw error; }
        this.releaseReferences = observeInteractionReferences(host, () => { this.sync(); this.bindTooltipSources(); this.reposition(); });
        owner.addEventListener('command', e => this.command(e as CommandEvent), options);
        if (host !== owner) host.addEventListener('command', e => { if (e.target === host) this.command(e as CommandEvent); }, options);
        owner.addEventListener('beforetoggle', e => {
            if (e.target !== owner) return;
            const event = e as ToggleEvent;
            if (event.newState === 'open') {
                if (!this.prepared) {
                    const invocation = this.fromSource(event.source instanceof HTMLElement ? event.source : undefined);
                    if (!this.prepare(invocation) || !this.prepareBody()) event.preventDefault();
                    else if (this.relay) {
                        // Native popovertarget opens through beforetoggle. Defer it
                        // so the same prepared handoff commits before visibility.
                        event.preventDefault();
                        const pending = this.preparation = new AbortController(), generation = this.generation;
                        queueMicrotask(() => {
                            if (pending.signal.aborted || generation !== this.generation || this.disposed) return;
                            this.preparation = undefined; this.show();
                        });
                    }
                }
            } else { this.phase = 'closing'; this.captureReturn(); }
            this.reflect(); queueMicrotask(() => this.sync());
        }, options);
        owner.addEventListener('cancel', e => {
            this.captureReturn();
            if (!this.emit('cem-before-close', true, this.closeReason)) { e.preventDefault(); this.phase = 'open'; this.restore = false; }
            queueMicrotask(() => this.sync());
        }, options);
        root.addEventListener('click', e => {
            const source = e.target instanceof Element ? e.target.closest('button,a') : null;
            if (!(source instanceof HTMLElement)) return;
            const activation = { source, invocation: captureCemSurfaceInvocation(source, e) };
            this.activation = activation; queueMicrotask(() => { if (this.activation === activation) this.activation = undefined; });
        }, { ...options, capture: true });
        root.addEventListener('click', e => {
            if (e.defaultPrevented) return;
            const source = e.target instanceof Element ? e.target.closest('button') : null;
            if (!(source instanceof HTMLElement)) return;
            const action = this.controls.popoverAction(source); if (!action) return;
            e.preventDefault();
            if (action === 'show' || action === 'toggle' && !visible(owner) && !this.preparation) this.open(this.fromSource(source));
            else this.requestClose(this.controls.isClose(source) ? 'close-control' : 'command');
        }, options);
        root.addEventListener('pointerdown', e => {
            if (visible(owner) && e.target instanceof Node && !owner.contains(e.target)) this.noteDismissalReason('outside');
        }, { ...options, capture: true });
        root.addEventListener('keydown', e => {
            const event = e as KeyboardEvent;
            if (event.defaultPrevented || isCemEditorCompositionKey(event)) return;
            if (this.preparation && event.key === 'Escape' && event.target instanceof Node &&
                (this.invocation?.source?.contains(event.target) || owner.contains(event.target))) {
                event.preventDefault(); event.stopPropagation(); this.requestClose('escape'); return;
            }
            if (this.kind() === 'tooltip' && event.key === 'Escape' && (visible(owner) || this.timer !== undefined)) {
                event.preventDefault(); event.stopPropagation(); this.requestClose('escape'); return;
            }
            if (!visible(owner)) return;
            if (event.key === 'Tab') this.noteDismissalReason('tab');
            if (event.key === 'Escape' && (owner.contains(owner.ownerDocument.activeElement) || this.kind() === 'tooltip')) {
                this.closeReason = 'escape';
                // Modal native cancellation remains browser-owned. Persistent
                // nonmodal dialogs and manual popovers need an explicit adapter.
                if (!owner.matches(':modal') && (nativeDialog(owner) && !owner.hasAttribute('popover') || owner.getAttribute('popover') === 'manual' || this.kind() === 'tooltip')) {
                    event.preventDefault(); event.stopPropagation(); this.requestClose('escape');
                }
            }
        }, options);
        owner.addEventListener('interest', e => {
            if (this.kind() !== 'tooltip') return;
            e.preventDefault(); const source = (e as Event & { source?: HTMLElement }).source;
            if (source instanceof HTMLElement && !this.tooltipSources.has(source)) this.open(this.fromSource(source));
        }, options);
        owner.addEventListener('loseinterest', e => {
            if (this.kind() !== 'tooltip') return;
            e.preventDefault(); this.tooltipDismissed = true; this.schedule(false, this.invocation, true);
        }, options);
        owner.addEventListener('pointerenter', event => {
            if (this.kind() !== 'tooltip' || event.pointerType === 'touch') return;
            this.tooltipPointer = true; this.tooltipInterest();
        }, options);
        owner.addEventListener('pointerleave', event => {
            if (this.kind() !== 'tooltip' || event.pointerType === 'touch') return;
            this.tooltipPointer = false; this.tooltipInterest();
        }, options);
        if (this.body.eager()) this.body.mount();
        this.bindTooltipSources(); this.sync();
    }
    private kind(): CemNativeSurfaceKind | undefined {
        return surfaceKind(this.owner, this.host);
    }
    private placement(): string { return this.host.getAttribute('placement') ?? (this.kind() === 'tooltip' ? 'block-start center' : 'center'); }
    private profile(): boolean {
        const kind = this.kind(), modal = this.host.getAttribute('mode') === 'modal' || this.owner.matches(':modal'), role = this.owner.getAttribute('role');
        const invalid = !kind || kind !== this.registeredKind || !this.owner.isConnected || !this.host.isConnected || this.owner.hidden
            || kind === 'dialog' && (!nativeDialog(this.owner) && !(role === 'dialog' && this.owner.hasAttribute('popover')) || role !== null && role !== 'dialog' && role !== 'alertdialog')
            || kind === 'tooltip' && (role !== 'tooltip' || !this.owner.hasAttribute('popover') || this.owner.querySelector('button,a[href],input,select,textarea,[contenteditable]:not([contenteditable="false"]),[tabindex]'))
            || modal && (kind !== 'dialog' || !nativeDialog(this.owner) || this.owner.hasAttribute('popover') || this.host.getAttribute('presentation') === 'local')
            || this.host.hasAttribute('mode') && !['modal', 'nonmodal'].includes(this.host.getAttribute('mode') ?? '')
            || nativeDialog(this.owner) && !this.owner.hasAttribute('popover') && this.owner.open && this.host.hasAttribute('mode')
                && (this.host.getAttribute('mode') === 'modal') !== this.owner.matches(':modal')
            || this.host.getAttribute('overflow') === 'hide' && kind === 'dialog';
        reportInteractionReference(this.host, invalid ? 'interaction-profile-conflict' : undefined, 'profile');
        return !invalid;
    }
    private fromSource(source?: HTMLElement): CemSurfaceInvocation {
        if (!source) return {};
        if (this.activation?.source === source) {
            const captured = source instanceof HTMLButtonElement ? getCemActionInvocation(source) : undefined;
            return captured ? { ...captured, geometry: this.activation.invocation.geometry, inputKind: this.activation.invocation.inputKind } : this.activation.invocation;
        }
        return captureCemSurfaceInvocation(source);
    }
    private prepare(invocation: CemSurfaceInvocation): boolean {
        const relay = this.kind() === 'dialog' ? captureCemMenuTaskRelay(invocation.source) : undefined;
        if (this.disposed || !this.profile() || !this.body.valid()) return false;
        if (visible(this.owner) && this.kind() === 'dialog' && (this.invocation?.contextKey ?? null) !== (invocation.contextKey ?? null)) {
            const policy = this.host.getAttribute('context-change') ?? 'reject';
            if (policy !== 'replace' && !(policy === 'request' && this.emit('cem-context-change', true, 'activate', invocation))) {
                reportInteractionReference(this.host, 'interaction-context-rejected', 'context'); return false;
            }
        }
        if (!nativeSurfaceGeometry(this.host, this.owner, invocation.source, invocation.geometry, this.placement())) return false;
        if (!visible(this.owner) && !this.emit('cem-before-open', true, 'activate', invocation)) return false;
        if (this.disposed || !this.profile()) return false;
        reportInteractionReference(this.host, undefined, 'context');
        this.invalidatePreparation();
        this.relay = relay;
        this.invocation = retainInvocation({ ...invocation, logicalParent: invocation.logicalParent ?? relay?.logicalParent,
            returnDestination: invocation.returnDestination ?? relay?.returnDestination }); this.closeReason = 'native'; this.restore = false;
        if (!visible(this.owner)) { this.phase = 'preparing'; this.sessionId = `cem-native-session-${++sessions}`; }
        this.prepared = true; this.reflect(); return true;
    }
    private invalidatePreparation(): void {
        this.relay = undefined;
        this.generation++; this.preparation?.abort(); this.preparation = undefined;
        this.releaseBusy?.(); this.releaseBusy = undefined;
    }
    private busy(): void {
        const nodes = [...new Set([this.host, this.invocation?.source].filter((node): node is HTMLElement => !!node))];
        const previous = nodes.map(node => node.getAttribute('aria-busy'));
        nodes.forEach(node => node.setAttribute('aria-busy', 'true'));
        this.releaseBusy = () => nodes.forEach((node, index) => {
            if (node.getAttribute('aria-busy') !== 'true') return;
            const value = previous[index]; if (value === null) node.removeAttribute('aria-busy'); else node.setAttribute('aria-busy', value);
        });
    }
    /** False defers or rejects a browser opening; completion uses the same native owner. */
    private prepareBody(): boolean {
        const pending = new AbortController(), generation = this.generation;
        this.preparation = pending;
        try {
            if (!this.body.mount()) { this.failed(); return false; }
            const result = this.preparationOptions.prepareBody?.({ owner: this.owner, host: this.host, invocation: this.invocation ?? {}, signal: pending.signal });
            if (pending.signal.aborted || generation !== this.generation || this.disposed) {
                if (result) void result.catch(() => undefined);
                return false;
            }
            if (result && typeof result.then === 'function') {
                this.busy();
                void result.then(() => {
                    if (pending.signal.aborted || generation !== this.generation || this.disposed) return;
                    this.preparation = undefined; this.releaseBusy?.(); this.releaseBusy = undefined;
                    this.show();
                }, () => {
                    if (!pending.signal.aborted && generation === this.generation && !this.disposed) this.failed();
                });
                return false;
            }
            this.preparation = undefined;
            return true;
        } catch { if (generation === this.generation && !this.disposed) this.failed(); return false; }
    }
    private cancelPreparation(): void {
        this.invalidatePreparation(); this.prepared = false; this.phase = 'closed'; this.invocation = undefined;
        this.closeNested(); this.body.close(); this.reflect();
    }
    private failed(): void {
        this.invalidatePreparation(); this.prepared = false; this.phase = 'closed'; this.invocation = undefined;
        this.body.close(); this.reflect(); reportInteractionReference(this.host, 'interaction-open-failed', 'open');
    }
    private show(): boolean {
        if (this.disposed || !this.profile() || !this.body.valid()) { this.failed(); return false; }
        const alreadyOpen = visible(this.owner);
        if (this.relay) {
            const relay = this.relay; this.relay = undefined;
            if (!relay.commit()) { this.cancelPreparation(); return false; }
        }
        try {
            if (!alreadyOpen) {
                if (this.owner.hasAttribute('popover')) this.owner.showPopover();
                else if (nativeDialog(this.owner)) {
                    if (this.host.getAttribute('mode') === 'modal' || this.invocation?.command === 'show-modal') this.owner.showModal(); else this.owner.show();
                }
            }
            this.sync();
            if (!visible(this.owner)) { this.failed(); return false; }
            reportInteractionReference(this.host, undefined, 'open');
            if (alreadyOpen) { this.reposition(); if (this.kind() === 'dialog') focusSurface(this.host, this.owner); }
            return true;
        } catch { this.failed(); return false; }
    }
    open(invocation: CemSurfaceInvocation = {}): boolean {
        this.cancelTimer();
        if (this.kind() === 'tooltip' && !this.tooltipAvailable(invocation.source)) return false;
        if (!this.prepare(invocation)) return false;
        if (visible(this.owner)) return this.show();
        return this.prepareBody() ? this.show() : !!this.preparation;
    }
    requestClose(reason = 'command'): boolean {
        this.cancelTimer();
        if (this.kind() === 'tooltip' && ['escape', 'command', 'close-control'].includes(reason)) this.tooltipDismissed = true;
        if (this.disposed) return false;
        if (this.preparation) {
            if (!this.emit('cem-before-close', true, reason)) return false;
            this.cancelPreparation(); return true;
        }
        if (!visible(this.owner)) return false;
        this.closeReason = reason; this.captureReturn(); this.phase = 'closing'; this.reflect();
        if (this.owner.hasAttribute('popover')) {
            if (!this.emit('cem-before-close', true, reason)) { this.phase = 'open'; this.restore = false; this.reflect(); return false; }
            this.owner.hidePopover();
        } else if (nativeDialog(this.owner)) this.owner.requestClose();
        this.sync(); return !visible(this.owner);
    }
    private command(event: CommandEvent): void {
        if (event.target !== this.owner && event.target !== this.host || event.defaultPrevented) return;
        if (event.source?.hasAttribute('commandfor') && event.source.hasAttribute('popovertarget')) {
            event.preventDefault(); reportInteractionReference(this.host, 'interaction-dual-route', 'command'); return;
        }
        if (event.command === 'show-modal' && (this.owner.hasAttribute('popover') || this.host.getAttribute('mode') === 'nonmodal' || this.host.getAttribute('presentation') === 'local')
            || ['close', 'request-close'].includes(event.command) && this.owner.hasAttribute('popover')
            || ['show-popover', 'hide-popover', 'toggle-popover'].includes(event.command) && !this.owner.hasAttribute('popover')) {
            event.preventDefault(); reportInteractionReference(this.host, 'interaction-command-incompatible', 'command'); return;
        }
        const opening = ['--cem-show', 'show-modal', 'show-popover'].includes(event.command) || event.command === 'toggle-popover' && !visible(this.owner) && !this.preparation;
        const closing = ['--cem-hide', 'request-close', 'close', 'hide-popover'].includes(event.command) || event.command === 'toggle-popover' && (visible(this.owner) || !!this.preparation);
        if (!opening && !closing) return;
        if (opening) {
            const invocation = { ...this.fromSource(event.source instanceof HTMLElement ? event.source : undefined), command: event.command };
            event.preventDefault(); this.open(invocation); return;
        } else {
            const reason = this.controls.isClose(event.source) ? 'close-control' : 'command';
            if (event.command === '--cem-hide' || this.preparation) { event.preventDefault(); this.requestClose(reason); return; }
            this.closeReason = reason; this.captureReturn();
            if (this.owner.hasAttribute('popover') && !this.emit('cem-before-close', true, reason)) event.preventDefault();
        }
        queueMicrotask(() => this.sync());
    }
    private captureReturn(): void {
        const active = this.owner.ownerDocument.activeElement;
        this.restore = !['outside', 'tab', 'disconnect', 'anchor-lost'].includes(this.closeReason)
            && (this.closeReason === 'escape' || !!active && this.owner.contains(active));
    }
    private noteDismissalReason(reason: string): void {
        this.closeReason = reason;
        queueMicrotask(() => { if (visible(this.owner) && this.closeReason === reason) this.closeReason = 'native'; });
    }
    private sync(): void {
        if (this.disposed) return;
        this.controls.refresh(this.kind() === 'dialog', visible(this.owner));
        if (this.preparation && this.relay && !this.relay.current()) { this.cancelPreparation(); return; }
        if (visible(this.owner) && this.wasOpen && !this.profile()) {
            this.restore = false; this.closeReason = 'native';
            if (this.owner.hasAttribute('popover')) this.owner.hidePopover(); else if (nativeDialog(this.owner)) this.owner.close();
        }
        if (this.preparation && (!this.owner.isConnected || !this.host.isConnected || !this.body.valid())) {
            this.failed(); return;
        }
        const open = visible(this.owner);
        if (open && !this.wasOpen) {
            if (!this.profile() || !this.reposition()) {
                if (this.owner.hasAttribute('popover')) this.owner.hidePopover(); else if (nativeDialog(this.owner)) this.owner.close();
                this.geometry.reset();
                this.failed(); return;
            }
            this.wasOpen = true; this.prepared = false; this.phase = 'open'; this.reflect();
            if (this.kind() === 'dialog') focusSurface(this.host, this.owner);
            else this.describe(this.invocation?.source);
            this.emit('cem-open', false, 'activate');
        } else if (!open && this.wasOpen) {
            this.wasOpen = false; this.phase = 'closing'; this.prepared = false; this.geometry.reset(); this.reflect();
            this.closeNested();
            if (this.kind() === 'dialog' && this.restore) restoreSurfaceFocus(this.host, this.invocation?.returnDestination ?? this.invocation?.source);
            const generation = ++this.generation;
            this.emit('cem-close', false, this.closeReason);
            if (generation !== this.generation || visible(this.owner)) return;
            this.invocation = undefined; this.restore = false;
            const finish = () => {
                if (this.disposed || generation !== this.generation || visible(this.owner)) return;
                this.body.close(); this.phase = 'closed'; this.reflect();
            };
            const exits = surfaceExitAnimations(this.owner);
            if (exits.length) void Promise.all(exits).then(finish); else finish();
        } else if (!open && !this.preparation && this.phase !== 'closing') {
            this.prepared = false; this.phase = 'closed'; this.invocation = undefined; this.reflect();
        } else if (open) { this.phase = 'open'; this.prepared = false; this.reflect(); }
    }
    private closeNested(): void {
        const descendants = [...this.owner.querySelectorAll<HTMLElement>('*')].reverse();
        for (const owner of descendants) {
            const child = controllers.get(owner)?.controller;
            if (!child) continue;
            const preparing = !!child.preparation;
            child.invalidatePreparation(); child.closeReason = 'ancestor-close'; child.restore = false;
            if (visible(owner)) { if (owner.hasAttribute('popover')) owner.hidePopover(); else if (nativeDialog(owner)) owner.close(); }
            child.sync(); if (preparing) child.body.close();
        }
    }
    private reposition(): boolean {
        if (this.disposed || !visible(this.owner)) return false;
        const ok = this.geometry.fit(this.invocation?.source, this.invocation?.geometry, this.placement());
        if (!ok && this.wasOpen && (this.host.getAttribute('anchor-lost') ?? (this.kind() === 'dialog' ? 'freeze' : 'close')) !== 'freeze') this.requestClose('anchor-lost');
        return ok;
    }
    private emit(name: string, cancelable: boolean, reason: string, invocation = this.invocation): boolean {
        return this.host.dispatchEvent(new CustomEvent(name, { bubbles: true, cancelable, detail: {
            source: invocation?.source, command: invocation?.command ?? null, contextKey: invocation?.contextKey ?? null,
            reason, sessionId: this.sessionId, ...(nativeDialog(this.owner) ? { returnValue: this.owner.returnValue } : {}),
        } }));
    }
    private reflect(): void {
        if (this.host.getAttribute('data-state') !== this.phase) this.host.setAttribute('data-state', this.phase);
        this.controls.refresh(this.kind() === 'dialog', visible(this.owner));
    }
    private describe(source?: HTMLElement): void {
        if (!source) return;
        if (this.describedSources.has(source) && this.describedSources.get(source) !== this.owner.id) this.forgetDescription(source);
        if (!this.owner.id) {
            const root = this.owner.getRootNode() as ParentNode;
            do { this.owner.id = `cem-native-description-${++sessions}`; } while ([...root.querySelectorAll('[id]')].some(node => node !== this.owner && node.id === this.owner.id));
            this.descriptionId = this.owner.id;
        }
        const tokens = source.getAttribute('aria-describedby')?.split(/\s+/).filter(Boolean) ?? [];
        if (!tokens.includes(this.owner.id)) { tokens.push(this.owner.id); source.setAttribute('aria-describedby', tokens.join(' ')); this.describedSources.set(source, this.owner.id); }
    }
    private forgetDescription(source: HTMLElement): void {
        const id = this.describedSources.get(source); if (!id) return;
        const remaining = source.getAttribute('aria-describedby')?.split(/\s+/).filter(token => token && token !== id) ?? [];
        if (remaining.length) source.setAttribute('aria-describedby', remaining.join(' ')); else source.removeAttribute('aria-describedby');
        this.describedSources.delete(source);
    }
    private pendingInterest?: { open: boolean; source?: HTMLElement; delay: number };
    private cancelTimer(): void {
        if (this.timer !== undefined) clearTimeout(this.timer);
        this.timer = undefined; this.pendingInterest = undefined;
    }
    private schedule(open: boolean, invocation?: CemSurfaceInvocation, pointer = false): void {
        const authored = this.host.getAttribute(open ? 'show-delay' : 'hide-delay');
        const delay = pointer ? authored === null ? open ? 500 : 100 : /^\d+$/.test(authored) ? Number(authored) : NaN : 0;
        if (!Number.isSafeInteger(delay) || delay < 0 || delay > 2147483647) { this.cancelTimer(); reportInteractionReference(this.host, 'interaction-profile-conflict', 'timing'); return; }
        if (this.timer !== undefined && this.pendingInterest?.open === open && this.pendingInterest.source === invocation?.source && this.pendingInterest.delay === delay) return;
        this.cancelTimer();
        if (!delay) { if (open) this.open(invocation); else this.requestClose('outside'); return; }
        this.pendingInterest = { open, source: invocation?.source, delay };
        this.timer = setTimeout(() => {
            this.timer = undefined; this.pendingInterest = undefined;
            if (open) this.open(invocation); else this.requestClose('outside');
        }, delay);
    }
    ownsDescriptionId(node: Element): boolean { return node === this.owner && !!this.descriptionId && node.id === this.descriptionId; }
    private tooltipAvailable(source?: HTMLElement): boolean {
        return !this.host.hasAttribute('disabled') && !this.host.closest('[hidden],[inert]')
            && !!this.owner.textContent?.trim() && (!source || source.isConnected && !source.matches(':disabled,[aria-disabled="true"]') && !source.closest('[hidden],[inert]'));
    }
    private tooltipInterest(): void {
        this.tooltipAutomatic = true;
        const entries = [...this.tooltipSources].filter(([source]) => this.tooltipAvailable(source));
        const focused = entries.find(([, state]) => state.focused && !state.touch);
        const hovered = entries.find(([, state]) => state.pointer && !state.touch);
        const source = focused?.[0] ?? hovered?.[0] ?? (this.tooltipPointer && this.invocation?.source && this.tooltipSources.has(this.invocation.source) ? this.invocation.source : undefined);
        const interested = !!source && this.tooltipAvailable(source);
        if (!interested && !this.tooltipPointer) this.tooltipDismissed = false;
        if (!interested || this.tooltipDismissed) {
            if (!this.tooltipAvailable(this.invocation?.source)) { this.cancelTimer(); this.requestClose('outside'); }
            else if (visible(this.owner) || this.timer !== undefined) this.schedule(false, this.invocation, true);
            return;
        }
        if (visible(this.owner) && this.invocation?.source === source) { this.cancelTimer(); return; }
        this.schedule(true, captureCemSurfaceInvocation(source), !focused);
    }
    private bindTooltipSources(): void {
        if (this.disposed) return;
        if (this.kind() !== 'tooltip') {
            for (const state of this.tooltipSources.values()) state.abort.abort();
            this.tooltipSources.clear();
            for (const source of this.describedSources.keys()) this.forgetDescription(source);
            return;
        }
        const select = (target?: Element): HTMLElement | undefined => {
            const selector = 'button,a[href],input,select,textarea,summary';
            const source = target?.matches(selector) ? target : target?.querySelector(':scope > [part~=control]');
            return source instanceof HTMLElement && source.matches(selector) ? source : undefined;
        };
        const next = new Set<HTMLElement>();
        const local = [...this.host.children].filter(node => node.getAttribute('slot') === 'trigger');
        const explicit = this.host.getAttribute('trigger-for');
        let code: string | undefined;
        if (local.length > 1 || explicit !== null && local.length) code = 'interaction-trigger-ambiguous';
        else if (explicit !== null) {
            const selected = interactionReference(this.host, explicit, 'trigger-for'), source = select(selected.target);
            code = selected.code ?? (!source ? 'interaction-control-unsupported' : undefined);
            if (source && !code) next.add(source);
        } else if (local.length) {
            const source = select(local[0]); if (source) next.add(source); else code = 'interaction-control-unsupported';
        }
        reportInteractionReference(this.host, code, 'trigger');
        const root = this.owner.getRootNode() as ParentNode;
        if (!code && this.owner.id) for (const source of root.querySelectorAll<HTMLElement>('[interestfor]')) {
            if (select(source) === source && source.getAttribute('interestfor') === this.owner.id && interactionReference(source, `#${this.owner.id}`).target === this.owner) next.add(source);
        }
        for (const [source, state] of this.tooltipSources) if (!next.has(source)) { state.abort.abort(); this.tooltipSources.delete(source); this.forgetDescription(source); }
        for (const source of next) {
            this.describe(source);
            if (this.tooltipSources.has(source)) continue;
            const state = { abort: new AbortController(), focused: this.owner.ownerDocument.activeElement === source, pointer: false, touch: false };
            this.tooltipSources.set(source, state); const options = { signal: state.abort.signal };
            source.addEventListener('pointerenter', event => {
                if (event.pointerType === 'touch') return;
                state.pointer = true; state.touch = false; this.tooltipInterest();
            }, options);
            source.addEventListener('pointerleave', event => { if (event.pointerType !== 'touch') { state.pointer = false; this.tooltipInterest(); } }, options);
            source.addEventListener('pointerdown', event => {
                if (event.pointerType === 'touch') { state.touch = true; state.focused = false; state.pointer = false; this.tooltipInterest(); }
            }, options);
            source.addEventListener('focus', () => { state.focused = true; this.tooltipInterest(); }, options);
            source.addEventListener('blur', () => { state.focused = false; state.touch = false; this.tooltipInterest(); }, options);
            source.addEventListener('keydown', event => {
                if (event.key !== 'Escape' && !event.defaultPrevented && !isCemEditorCompositionKey(event)) {
                    state.touch = false; state.focused = this.owner.ownerDocument.activeElement === source; this.tooltipInterest();
                }
            }, options);
            if (state.focused) this.tooltipAutomatic = true;
        }
        if (this.tooltipAutomatic) this.tooltipInterest();
        else if (!this.tooltipAvailable(this.invocation?.source)) this.requestClose('outside');
    }
    disconnect(): void {
        if (this.disposed) return;
        this.cancelTimer(); this.invalidatePreparation(); this.closeNested(); this.closeReason = 'disconnect'; this.restore = false;
        if (visible(this.owner)) { if (this.owner.hasAttribute('popover')) this.owner.hidePopover(); else if (nativeDialog(this.owner)) this.owner.close(); this.sync(); }
        this.body.close(); this.phase = 'closed'; this.reflect();
        this.disposed = true; this.abort.abort(); this.releaseVisibility(); this.releaseReferences(); this.geometry.release();
        this.controls.disconnect();
        for (const state of this.tooltipSources.values()) state.abort.abort(); this.tooltipSources.clear();
        for (const source of this.describedSources.keys()) this.forgetDescription(source);
        this.describedSources.clear(); this.invocation = undefined; this.activation = undefined; controllers.delete(this.owner);
    }
}
export function connectCemNativeSurface(owner: HTMLElement, options: CemNativeSurfaceOptions = {}): CemNativeSurfaceController {
    const host = options.host ?? owner, existing = controllers.get(owner);
    if (existing) {
        if (existing.controller.host !== host) throw new TypeError('Native surface already has a semantic owner');
        if (existing.kind !== surfaceKind(owner, host)) throw new TypeError('Native surface already has a different semantic profile');
        return existing.controller;
    }
    const controller = new NativeSurface(owner, host, options); controllers.set(owner, { controller, kind: surfaceKind(owner, host) }); return controller;
}
const providerControllers = new WeakMap<HTMLElement, CemNativeSurfaceController>();
export const CEM_NATIVE_SURFACE_CAPABILITY: CemProducedElementBehavior = {
    rendered(host, context) {
        const candidates = [...host.querySelectorAll<HTMLElement>(':scope > [part~="surface"]')];
        const current = providerControllers.get(host);
        if (candidates.length !== 1) { current?.disconnect(); providerControllers.delete(host); reportInteractionReference(host, 'interaction-surface-ambiguous', 'surface'); return; }
        const owner = candidates[0];
        if (current?.owner !== owner) { current?.disconnect(); providerControllers.set(host, connectCemNativeSurface(owner, { host, prepareBody: ({ signal }) => context.runtime.prepareSurfaceBody(host, owner, signal) })); }
        reportInteractionReference(host, undefined, 'surface');
        if (!defaultsInitialized.has(host)) { defaultsInitialized.add(host); if (!context.resumed && host.hasAttribute('default-open')) providerControllers.get(host)?.open(); }
    },
    preserveRenderedNode(host, current) {
        const owner = providerControllers.get(host)?.owner ?? (current.parentElement?.parentElement === host
            && current.parentElement.matches('[part~=surface]') ? current.parentElement : undefined);
        return !!owner && (ownsCemSurfaceBodyNode(owner, current) || controllers.get(owner)?.controller.controls.owns(current) === true);
    },
    preserveRenderedAttribute(host, current, _desired, attribute) {
        const owner = providerControllers.get(host)?.owner;
        return current === owner && ['open', 'style', 'data-placement'].includes(attribute.name)
            || !!owner && (controllers.get(owner)?.controller.controls.ownsAttribute(current, attribute.name) === true
                || attribute.name === 'id' && controllers.get(owner)?.controller.ownsDescriptionId(current) === true);
    },
    disconnected(host) { providerControllers.get(host)?.disconnect(); providerControllers.delete(host); },
};
