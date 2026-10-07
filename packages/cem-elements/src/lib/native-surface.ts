import { getCemActionInvocation } from './action-command-capability.js';
import { interactionReference, interactionControl, observeInteractionReferences, reportInteractionReference } from './interaction-reference.js';
import { fitNativeSurface, nativeSurfaceGeometry, observePopupGeometry, releasePopupGeometry } from './popup-controller.js';
import { focusSurface, restoreSurfaceFocus } from './surface-references.js';
import { captureCemSurfaceInvocation, snapshotGeometryRect, type CemSurfaceInvocation } from './surface-invocation.js';
import type { CemProducedElementBehavior } from './cem-elements.js';

export type CemNativeSurfaceKind = 'dialog' | 'tooltip';
export interface CemNativeSurfaceController {
    readonly owner: HTMLElement;
    readonly host: HTMLElement;
    open(invocation?: CemSurfaceInvocation): boolean;
    requestClose(reason?: string): boolean;
    disconnect(): void;
}
const controllers = new WeakMap<HTMLElement, NativeSurface>();
const defaultsInitialized = new WeakSet<HTMLElement>();
let sessions = 0;
function nativeDialog(node: HTMLElement): node is HTMLDialogElement { return node.localName === 'dialog'; }
function visible(node: HTMLElement): boolean { return node.hasAttribute('popover') ? node.matches(':popover-open') : nativeDialog(node) && node.open; }
function retainInvocation(invocation: CemSurfaceInvocation): CemSurfaceInvocation {
    return Object.freeze({ ...invocation, geometry: invocation.geometry && Object.freeze({
        pointer: snapshotGeometryRect(invocation.geometry.pointer, true), selection: snapshotGeometryRect(invocation.geometry.selection),
    }) });
}
/** An eager native owner owns visibility; this adapter owns its transient session. */
class NativeSurface implements CemNativeSurfaceController {
    private readonly abort = new AbortController();
    private readonly observer: MutationObserver;
    private readonly releaseReferences: () => void;
    private readonly releaseGeometry: () => void;
    private readonly tooltipSources = new Map<HTMLElement, AbortController>();
    private readonly describedSources = new Map<HTMLElement, string>();
    private invocation?: CemSurfaceInvocation;
    private activation?: { source: HTMLElement; invocation: CemSurfaceInvocation };
    private phase = 'closed';
    private wasOpen = false;
    private closeReason = 'native';
    private restore = false;
    private prepared = false;
    private sessionId = '';
    private disposed = false;
    private timer?: ReturnType<typeof setTimeout>;
    constructor(readonly owner: HTMLElement, readonly host: HTMLElement) {
        const options = { signal: this.abort.signal }, root = owner.getRootNode();
        this.observer = new MutationObserver(() => { this.sync(); this.bindTooltipSources(); });
        this.observer.observe(owner, { attributes: true, attributeFilter: ['open', 'popover', 'hidden', 'role'], childList: true, subtree: true });
        this.releaseReferences = observeInteractionReferences(host, () => { this.sync(); this.bindTooltipSources(); this.reposition(); });
        this.releaseGeometry = observePopupGeometry(host, () => this.reposition());
        owner.addEventListener('command', e => this.command(e as CommandEvent), options);
        owner.addEventListener('beforetoggle', e => {
            if (e.target !== owner) return;
            const event = e as ToggleEvent;
            if (event.newState === 'open') {
                if (!this.prepared && !this.prepare(this.fromSource(event.source instanceof HTMLElement ? event.source : undefined))) event.preventDefault();
            } else { this.phase = 'closing'; this.captureReturn(); }
            this.reflect(); queueMicrotask(() => this.sync());
        }, options);
        owner.addEventListener('toggle', () => this.sync(), options);
        owner.addEventListener('close', () => this.sync(), options);
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
        root.addEventListener('pointerdown', e => {
            if (visible(owner) && e.target instanceof Node && !owner.contains(e.target)) this.noteDismissalReason('outside');
        }, { ...options, capture: true });
        root.addEventListener('keydown', e => {
            const event = e as KeyboardEvent;
            if (!visible(owner) || event.defaultPrevented) return;
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
            e.preventDefault(); this.schedule(false, this.invocation, true);
        }, options);
        owner.addEventListener('pointerenter', () => this.cancelTimer(), options);
        owner.addEventListener('pointerleave', () => { if (this.kind() === 'tooltip') this.schedule(false, this.invocation, true); }, options);
        const view = owner.ownerDocument.defaultView;
        view?.addEventListener('resize', () => this.reposition(), options);
        view?.addEventListener('scroll', () => this.reposition(), { ...options, capture: true });
        view?.visualViewport?.addEventListener('resize', () => this.reposition(), options);
        view?.visualViewport?.addEventListener('scroll', () => this.reposition(), options);
        this.bindTooltipSources(); this.sync();
    }
    private kind(): CemNativeSurfaceKind | undefined {
        const kind = this.host.getAttribute('kind');
        if (kind) return kind === 'dialog' || kind === 'tooltip' ? kind : undefined;
        return this.owner.getAttribute('role') === 'tooltip' ? 'tooltip' : nativeDialog(this.owner) || this.owner.getAttribute('role') === 'dialog' ? 'dialog' : undefined;
    }
    private placement(): string { return this.host.getAttribute('placement') ?? (this.kind() === 'tooltip' ? 'block-start center' : 'center'); }
    private profile(): boolean {
        const kind = this.kind(), modal = this.host.getAttribute('mode') === 'modal' || this.owner.matches(':modal'), role = this.owner.getAttribute('role');
        const invalid = !kind || !this.owner.isConnected || !this.host.isConnected || this.owner.hidden
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
        if (this.disposed || !this.profile()) return false;
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
        this.invocation = retainInvocation(invocation); this.closeReason = 'native'; this.restore = false;
        if (!visible(this.owner)) { this.phase = 'preparing'; this.sessionId = `cem-native-session-${++sessions}`; }
        this.prepared = true; this.reflect(); return true;
    }
    open(invocation: CemSurfaceInvocation = {}): boolean {
        this.cancelTimer();
        const alreadyOpen = visible(this.owner);
        if (!this.prepare(invocation)) return false;
        try {
            if (!visible(this.owner)) {
                if (this.owner.hasAttribute('popover')) this.owner.showPopover();
                else if (nativeDialog(this.owner)) {
                    if (this.host.getAttribute('mode') === 'modal') this.owner.showModal(); else this.owner.show();
                }
            }
            this.sync();
            if (alreadyOpen && visible(this.owner)) {
                this.reposition(); if (this.kind() === 'dialog') focusSurface(this.host, this.owner);
            }
            return visible(this.owner);
        } catch { reportInteractionReference(this.host, 'interaction-open-failed', 'open'); this.prepared = false; this.phase = 'closed'; this.reflect(); return false; }
    }
    requestClose(reason = 'command'): boolean {
        this.cancelTimer();
        if (this.disposed || !visible(this.owner)) return false;
        this.closeReason = reason; this.captureReturn(); this.phase = 'closing'; this.reflect();
        if (this.owner.hasAttribute('popover')) {
            if (!this.emit('cem-before-close', true, reason)) { this.phase = 'open'; this.restore = false; this.reflect(); return false; }
            this.owner.hidePopover();
        } else if (nativeDialog(this.owner)) this.owner.requestClose();
        this.sync(); return !visible(this.owner);
    }
    private command(event: CommandEvent): void {
        if (event.target !== this.owner || event.defaultPrevented) return;
        if (event.command === 'show-modal' && (this.owner.hasAttribute('popover') || this.host.getAttribute('mode') === 'nonmodal' || this.host.getAttribute('presentation') === 'local')
            || ['close', 'request-close'].includes(event.command) && this.owner.hasAttribute('popover')) {
            event.preventDefault(); reportInteractionReference(this.host, 'interaction-command-incompatible', 'command'); return;
        }
        const opening = ['--cem-show', 'show-modal', 'show-popover'].includes(event.command) || event.command === 'toggle-popover' && !visible(this.owner);
        const closing = ['--cem-hide', 'request-close', 'close', 'hide-popover'].includes(event.command) || event.command === 'toggle-popover' && visible(this.owner);
        if (!opening && !closing) return;
        if (opening) {
            const invocation = { ...this.fromSource(event.source instanceof HTMLElement ? event.source : undefined), command: event.command };
            if (event.command === '--cem-show') { event.preventDefault(); this.open(invocation); return; }
            if (!this.prepare(invocation)) event.preventDefault();
        } else {
            if (event.command === '--cem-hide') { event.preventDefault(); this.requestClose('command'); return; }
            this.closeReason = 'command'; this.captureReturn();
            if (this.owner.hasAttribute('popover') && !this.emit('cem-before-close', true, 'command')) event.preventDefault();
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
        if (visible(this.owner) && this.wasOpen && !this.profile()) {
            this.restore = false; this.closeReason = 'native';
            if (this.owner.hasAttribute('popover')) this.owner.hidePopover(); else if (nativeDialog(this.owner)) this.owner.close();
        }
        const open = visible(this.owner);
        if (open && !this.wasOpen) {
            if (!this.profile() || !this.reposition()) {
                if (this.owner.hasAttribute('popover')) this.owner.hidePopover(); else if (nativeDialog(this.owner)) this.owner.close();
                releasePopupGeometry(this.owner);
                this.prepared = false; this.invocation = undefined; this.phase = 'closed'; this.reflect(); return;
            }
            this.wasOpen = true; this.prepared = false; this.phase = 'open'; this.reflect();
            if (this.kind() === 'dialog') focusSurface(this.host, this.owner);
            else this.describe(this.invocation?.source);
            this.emit('cem-open', false, 'activate');
        } else if (!open && this.wasOpen) {
            this.wasOpen = false; this.phase = 'closed'; this.prepared = false; releasePopupGeometry(this.owner); this.reflect();
            if (this.kind() === 'dialog' && this.restore) restoreSurfaceFocus(this.host, this.invocation?.returnDestination ?? this.invocation?.source);
            this.emit('cem-close', false, this.closeReason); this.invocation = undefined; this.restore = false;
        } else if (!open) { this.prepared = false; this.phase = 'closed'; this.invocation = undefined; this.reflect(); }
        else { this.phase = 'open'; this.prepared = false; this.reflect(); }
    }
    private reposition(): boolean {
        if (this.disposed || !visible(this.owner)) return false;
        const ok = fitNativeSurface(this.host, this.owner, this.invocation?.source, this.invocation?.geometry, this.placement());
        if (!ok && this.wasOpen && (this.host.getAttribute('anchor-lost') ?? (this.kind() === 'dialog' ? 'freeze' : 'close')) !== 'freeze') this.requestClose('anchor-lost');
        return ok;
    }
    private emit(name: string, cancelable: boolean, reason: string, invocation = this.invocation): boolean {
        return this.host.dispatchEvent(new CustomEvent(name, { bubbles: true, cancelable, detail: {
            source: invocation?.source, command: invocation?.command ?? null, contextKey: invocation?.contextKey ?? null,
            reason, sessionId: this.sessionId, ...(nativeDialog(this.owner) ? { returnValue: this.owner.returnValue } : {}),
        } }));
    }
    private reflect(): void { if (this.host.getAttribute('data-state') !== this.phase) this.host.setAttribute('data-state', this.phase); }
    private describe(source?: HTMLElement): void {
        if (!source) return;
        if (this.describedSources.has(source) && this.describedSources.get(source) !== this.owner.id) this.forgetDescription(source);
        if (!this.owner.id) {
            const root = this.owner.getRootNode() as ParentNode;
            do { this.owner.id = `cem-native-description-${++sessions}`; } while ([...root.querySelectorAll('[id]')].some(node => node !== this.owner && node.id === this.owner.id));
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
    private cancelTimer(): void { if (this.timer !== undefined) clearTimeout(this.timer); this.timer = undefined; }
    private schedule(open: boolean, invocation?: CemSurfaceInvocation, pointer = false): void {
        this.cancelTimer();
        const authored = this.host.getAttribute(open ? 'show-delay' : 'hide-delay');
        const delay = pointer ? authored === null ? open ? 500 : 100 : /^\d+$/.test(authored) ? Number(authored) : NaN : 0;
        if (!Number.isSafeInteger(delay) || delay < 0 || delay > 2147483647) { reportInteractionReference(this.host, 'interaction-profile-conflict', 'timing'); return; }
        if (!delay) { if (open) this.open(invocation); else this.requestClose('outside'); return; }
        this.timer = setTimeout(() => { this.timer = undefined; if (open) this.open(invocation); else this.requestClose('outside'); }, delay);
    }
    private bindTooltipSources(): void {
        if (this.disposed) return;
        if (this.kind() !== 'tooltip') {
            for (const abort of this.tooltipSources.values()) abort.abort();
            this.tooltipSources.clear();
            for (const source of this.describedSources.keys()) this.forgetDescription(source);
            return;
        }
        const next = new Set<HTMLElement>();
        const explicit = this.host.getAttribute('trigger-for');
        if (explicit !== null) {
            const selected = interactionReference(this.host, explicit, 'trigger-for'), source = interactionControl(selected.target);
            reportInteractionReference(this.host, selected.code ?? (!source ? 'interaction-control-unsupported' : undefined), 'trigger');
            if (source) next.add(source);
        }
        const root = this.owner.getRootNode() as ParentNode;
        if (this.owner.id) for (const source of root.querySelectorAll<HTMLElement>('button[interestfor],a[interestfor]')) {
            if (source.getAttribute('interestfor') === this.owner.id && interactionReference(source, `#${this.owner.id}`).target === this.owner) next.add(source);
        }
        for (const [source, abort] of this.tooltipSources) if (!next.has(source)) { abort.abort(); this.tooltipSources.delete(source); this.forgetDescription(source); }
        for (const source of next) if (!this.tooltipSources.has(source)) {
            const abort = new AbortController(); this.tooltipSources.set(source, abort); const options = { signal: abort.signal };
            this.describe(source);
            source.addEventListener('pointerenter', e => { if ((e as PointerEvent).pointerType !== 'touch') this.schedule(true, captureCemSurfaceInvocation(source, new MouseEvent('click', { detail: 1, clientX: e.clientX, clientY: e.clientY })), true); }, options);
            source.addEventListener('pointerleave', () => { if (this.owner.ownerDocument.activeElement !== source) this.schedule(false, this.invocation, true); }, options);
            source.addEventListener('focus', () => this.schedule(true, captureCemSurfaceInvocation(source, new KeyboardEvent('keydown')), false), options);
            source.addEventListener('blur', () => this.schedule(false, this.invocation), options);
        }
    }
    disconnect(): void {
        if (this.disposed) return;
        this.cancelTimer(); this.closeReason = 'disconnect'; this.restore = false;
        if (visible(this.owner)) { if (this.owner.hasAttribute('popover')) this.owner.hidePopover(); else if (nativeDialog(this.owner)) this.owner.close(); this.sync(); }
        this.disposed = true; this.abort.abort(); this.observer.disconnect(); this.releaseReferences(); this.releaseGeometry(); releasePopupGeometry(this.owner);
        for (const abort of this.tooltipSources.values()) abort.abort(); this.tooltipSources.clear();
        for (const source of this.describedSources.keys()) this.forgetDescription(source);
        this.describedSources.clear(); this.invocation = undefined; this.activation = undefined; controllers.delete(this.owner);
    }
}
export function connectCemNativeSurface(owner: HTMLElement, options: { host?: HTMLElement } = {}): CemNativeSurfaceController {
    const host = options.host ?? owner, existing = controllers.get(owner);
    if (existing) { if (existing.host !== host) throw new TypeError('Native surface already has a semantic owner'); return existing; }
    const controller = new NativeSurface(owner, host); controllers.set(owner, controller); return controller;
}
const providerControllers = new WeakMap<HTMLElement, CemNativeSurfaceController>();
export const CEM_NATIVE_SURFACE_CAPABILITY: CemProducedElementBehavior = {
    rendered(host) {
        const candidates = [...host.querySelectorAll<HTMLElement>(':scope > [part~="surface"]')];
        const current = providerControllers.get(host);
        if (candidates.length !== 1) { current?.disconnect(); providerControllers.delete(host); reportInteractionReference(host, 'interaction-surface-ambiguous', 'surface'); return; }
        const owner = candidates[0];
        if (current?.owner !== owner) { current?.disconnect(); providerControllers.set(host, connectCemNativeSurface(owner, { host })); }
        reportInteractionReference(host, undefined, 'surface');
        if (!defaultsInitialized.has(host)) { defaultsInitialized.add(host); if (host.hasAttribute('default-open')) providerControllers.get(host)?.open(); }
    },
    preserveRenderedAttribute(host, current, _desired, attribute) {
        return current === providerControllers.get(host)?.owner && ['open', 'style', 'data-placement'].includes(attribute.name);
    },
    disconnected(host) { providerControllers.get(host)?.disconnect(); providerControllers.delete(host); },
};
