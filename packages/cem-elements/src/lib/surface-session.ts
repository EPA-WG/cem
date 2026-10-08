/** Shared transient native visibility ownership. Semantic delegates own admission and focus. */
export function nativeSurfaceVisible(owner: HTMLElement): boolean {
    if (!owner.hasAttribute('popover')) return owner.localName === 'dialog' && (owner as HTMLDialogElement).open;
    try { return owner.matches(':popover-open'); } catch { return false; }
}
function available(node: HTMLElement): boolean {
    return node.isConnected && !node.closest('[hidden]:not([hidden="until-found"]),[inert]');
}
interface Registration {
    host: HTMLElement;
    owner: HTMLElement;
    profile: string;
    epoch: number;
    open: boolean;
    released: boolean;
    listeners: Set<() => void>;
}
const OWNERS = Symbol.for('cem.surface-sessions.v1');
const environment = globalThis as typeof globalThis & { [OWNERS]?: WeakMap<HTMLElement, Registration> };
const owners = environment[OWNERS] ??= new WeakMap<HTMLElement, Registration>();
export interface CemSurfaceLifetime {
    readonly owner: HTMLElement;
    /** A captured open lifetime never becomes current again after closing or disposal. */
    current(): boolean;
    subscribe(invalidate: () => void): () => void;
}
function invalidate(record: Registration): void {
    record.epoch++;
    for (const listener of [...record.listeners]) listener();
}
/** Registration is ownership, not native-reference/placement authority. */
export function assertCemSurfaceOwnerAvailable(owner: HTMLElement): void {
    if (owners.has(owner)) throw new TypeError('Native surface already has a semantic owner');
}
export function registerCemSurfaceOwner(host: HTMLElement, owner: HTMLElement, profile: string, reconcile: () => void): () => void {
    assertCemSurfaceOwnerAvailable(owner);
    const record: Registration = { host, owner, profile, epoch: 0, open: nativeSurfaceVisible(owner), released: false, listeners: new Set() };
    owners.set(owner, record);
    const abort = new AbortController(), options = { signal: abort.signal };
    const sync = () => {
        if (record.released) return;
        const open = available(host) && available(owner) && nativeSurfaceVisible(owner);
        if (record.open && !open) invalidate(record);
        record.open = open; reconcile();
    };
    owner.addEventListener('beforetoggle', event => {
        if (event.target !== owner || !event.isTrusted) return;
        if ((event as ToggleEvent).newState === 'closed') invalidate(record);
        queueMicrotask(sync);
    }, options);
    for (const name of ['toggle', 'close']) owner.addEventListener(name, event => { if (event.target === owner) sync(); }, options);
    const observer = new MutationObserver(records => {
        // Dialog close/show can occur before mutation delivery or a coalesced event.
        const opens = records.filter(r => r.target === owner && r.attributeName === 'open');
        if (opens.some((r, i) => r.oldValue !== null && (i + 1 < opens.length ? opens[i + 1].oldValue : owner.getAttribute('open')) === null)) invalidate(record);
        sync();
    });
    observer.observe(owner.ownerDocument, { attributes: true, attributeOldValue: true,
        attributeFilter: ['hidden', 'inert', 'open', 'popover', 'role', 'tabindex', 'autofocus', 'disabled', 'readonly', 'class', 'style',
            'type', 'list', 'href', 'controls', 'contenteditable'],
        childList: true, subtree: true });
    return () => {
        if (record.released) return;
        record.released = true; abort.abort(); observer.disconnect(); owners.delete(owner); invalidate(record); record.listeners.clear();
    };
}
/** Supplied explicitly by a host that has admitted this ancestor relationship. */
export function captureCemSurfaceLifetime(owner: HTMLElement): CemSurfaceLifetime | undefined {
    const record = owners.get(owner);
    if (!record || record.released || !available(record.host) || !available(owner) || !nativeSurfaceVisible(owner)) return;
    const epoch = record.epoch; let expired = false;
    const current = () => {
        expired ||= record.released || owners.get(owner) !== record || epoch !== record.epoch
            || !available(record.host) || !available(owner) || !nativeSurfaceVisible(owner);
        return !expired;
    };
    return Object.freeze({ owner, current, subscribe(listener: () => void) {
        record.listeners.add(listener); return () => { record.listeners.delete(listener); };
    } });
}

export interface CemSurfaceOpening { readonly generation: number }
export interface CemSurfaceSession {
    readonly visible: boolean;
    readonly pending: boolean;
    readonly generation: number;
    prepare(): CemSurfaceOpening | undefined;
    complete(opening: CemSurfaceOpening): boolean;
    dismiss(reason?: string): boolean;
    reconcile(): boolean;
    disconnect(): void;
}
interface SessionOptions {
    profile: string;
    /** Capture consumer-owned readiness/revision/admission; no DOM strings grant authority. */
    capture(): (() => boolean) | undefined;
    fit(): boolean;
    reset(): void;
    ancestors?: readonly CemSurfaceLifetime[];
    onVisibility?(open: boolean, reason: string): void;
}
/** Native manual presentation with generation-fenced openings and exactly one close route. */
export function createCemSurfaceSession(host: HTMLElement, owner: HTMLElement, options: SessionOptions): CemSurfaceSession {
    let generation = 0, opening: CemSurfaceOpening | undefined, proof: (() => boolean) | undefined;
    let accepted = false, disposed = false, syncing = false, closing = false, reason = 'native';
    const ancestors = Object.freeze([...(options.ancestors ?? [])]);
    const abort = new AbortController();
    const notify = (open: boolean) => {
        if (accepted === open) return;
        accepted = open; options.onVisibility?.(open, open ? 'open' : reason);
    };
    const current = () => !disposed && available(host) && available(owner) && ancestors.every(a => a.current()) && !!proof?.();
    const reset = () => { generation++; opening = undefined; proof = undefined; options.reset(); };
    const dismiss = (cause = 'dismiss') => {
        if (disposed || closing) return false;
        const active = accepted || !!opening || nativeSurfaceVisible(owner);
        reason = cause; closing = true; reset();
        try { if (nativeSurfaceVisible(owner)) owner.hidePopover(); }
        finally { try { notify(false); } finally { closing = false; } }
        return active;
    };
    const reconcile = () => {
        if (disposed || closing || syncing) return accepted && nativeSurfaceVisible(owner);
        syncing = true;
        try {
            if (!nativeSurfaceVisible(owner)) {
                if (accepted) { reason = 'native'; reset(); notify(false); }
                else if (opening && !current()) dismiss('unavailable');
                return false;
            }
            if (!current() || !options.fit() || !current() || !nativeSurfaceVisible(owner)) { dismiss('unavailable'); return false; }
            opening = undefined; notify(true); return accepted && nativeSurfaceVisible(owner);
        } finally { syncing = false; }
    };
    const prepare = () => {
        if (disposed || closing) return;
        generation++; opening = undefined;
        const captured = options.capture();
        if (!captured || !available(host) || !available(owner) || !ancestors.every(a => a.current()) || !captured()) { dismiss('unavailable'); return; }
        proof = captured; reason = 'native'; opening = Object.freeze({ generation }); return opening;
    };
    const releaseOwner = registerCemSurfaceOwner(host, owner, options.profile, reconcile);
    const releases = ancestors.map(a => a.subscribe(() => dismiss('ancestor')));
    owner.addEventListener('beforetoggle', event => {
        if (event.target !== owner || !event.isTrusted) return;
        if ((event as ToggleEvent).newState === 'open') {
            if ((!opening || !current()) && !prepare() || !current()) event.preventDefault();
            const attempt = opening;
            queueMicrotask(() => {
                if (attempt && opening === attempt && !nativeSurfaceVisible(owner)) dismiss('native');
            });
        } else {
            // Closing beforetoggle is noncancelable. Drop claims before delayed toggle delivery.
            const wasClosing = closing; closing = true;
            try { reset(); notify(false); } finally { closing = wasClosing; }
        }
    }, { signal: abort.signal });
    const session: CemSurfaceSession = {
        get visible() { return reconcile(); },
        get pending() { if (opening && !current()) dismiss('unavailable'); return !!opening; },
        get generation() { return generation; },
        prepare,
        complete(attempt) {
            if (disposed || attempt !== opening || attempt.generation !== generation) return false;
            if (!current()) { dismiss('unavailable'); return false; }
            try { if (!nativeSurfaceVisible(owner)) owner.showPopover(); }
            catch { dismiss('unavailable'); return false; }
            if (!nativeSurfaceVisible(owner)) { dismiss('native'); return false; }
            return reconcile();
        },
        dismiss, reconcile,
        disconnect() {
            if (disposed) return;
            try { dismiss('disconnect'); }
            finally { disposed = true; abort.abort(); for (const release of releases) release(); releaseOwner(); }
        },
    };
    reconcile(); return session;
}

/** A complete outside pointer sequence; cancellation, drag and scrolling never imply dismissal. */
export function observeCemSurfaceDismissalRegion(root: Document, inside: (target: EventTarget | null) => boolean,
    active: () => number | undefined, dismiss: () => void): () => void {
    const abort = new AbortController(), options = { signal: abort.signal, capture: true };
    type Press = { down: PointerEvent; x: number; y: number; cancelled: boolean; generation: number };
    const presses = new Map<number, Press>();
    root.addEventListener('pointerdown', event => {
        const generation = active(); presses.delete(event.pointerId);
        if (generation !== undefined && event.isPrimary && event.button === 0 && !inside(event.target))
            presses.set(event.pointerId, { down: event, x: event.clientX, y: event.clientY, cancelled: false, generation });
    }, options);
    root.addEventListener('pointermove', event => {
        const press = presses.get(event.pointerId);
        if (press && Math.hypot(event.clientX - press.x, event.clientY - press.y) > 8) press.cancelled = true;
    }, options);
    root.addEventListener('pointercancel', event => { presses.delete(event.pointerId); }, options);
    for (const name of ['scroll', 'dragstart', 'lostpointercapture']) root.addEventListener(name, () => {
        for (const press of presses.values()) press.cancelled = true;
    }, options);
    root.addEventListener('pointerup', event => {
        const press = presses.get(event.pointerId); presses.delete(event.pointerId);
        if (!press) return;
        queueMicrotask(() => {
            if (!abort.signal.aborted && !press.cancelled && !press.down.defaultPrevented && !event.defaultPrevented
                && Math.hypot(event.clientX - press.x, event.clientY - press.y) <= 8 && active() === press.generation && !inside(event.target)) dismiss();
        });
    }, options);
    return () => { abort.abort(); presses.clear(); };
}
