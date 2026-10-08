import type { CemSurfaceLifetime } from './surface-session.js';

export interface CemPlatformCloseAuthority {
    current(): boolean;
    subscribe(revoke: () => void): () => void;
}
export interface CemPlatformCloseContext {
    readonly event: KeyboardEvent;
    readonly editor: HTMLInputElement;
    readonly ancestors: readonly CemSurfaceLifetime[];
}
export interface CemPlatformCloseLease {
    readonly current: boolean;
    reconcile(): void;
    dispose(): void;
}
export interface CemPlatformCloseCoordinator {
    /** Host calls this before any other watcher/native surface created by this event. */
    consume(event: Event): void;
    reserve(context: CemPlatformCloseContext, current: () => boolean, dismiss: (reason: string) => void): CemPlatformCloseLease | undefined;
}
interface NativeWatcher extends EventTarget { destroy(): void }
const USED = Symbol.for('cem.platform-close.used-events.v1');
const environment = globalThis as typeof globalThis & { [USED]?: WeakSet<Event> };
const used = environment[USED] ??= new WeakSet<Event>();

/** Trusted host boundary: admit attests verified browser grouping and control of
 * all relevant window/ancestor watcher creation. DOM markup cannot grant this.
 * First integration admits fresh, unmodified ArrowUp/ArrowDown openings only. */
export function createCemPlatformCloseCoordinator(view: Window,
    admit: (context: CemPlatformCloseContext) => CemPlatformCloseAuthority | undefined): CemPlatformCloseCoordinator {
    return Object.freeze({
        consume(event: Event) { used.add(event); },
        reserve(context: CemPlatformCloseContext, current: () => boolean, dismiss: (reason: string) => void) {
            const { event, editor } = context;
            const editorAvailable = () => editor.isConnected && editor.type === 'text' && !editor.hasAttribute('list')
                && !editor.disabled && !editor.readOnly && !editor.closest('[hidden]:not([hidden="until-found"]),[inert]')
                && editor.ownerDocument.defaultView === view && view.document.activeElement === editor;
            const Constructor = (view as Window & { CloseWatcher?: new () => NativeWatcher }).CloseWatcher;
            if (!Constructor || used.has(event) || !event.isTrusted || event.eventPhase === Event.NONE
                || event.type !== 'keydown' || event.target !== editor || event.defaultPrevented || event.repeat || event.isComposing
                || !['ArrowDown', 'ArrowUp'].includes(event.key) || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey
                || !editorAvailable()
                || !view.navigator.userActivation?.isActive || !current()) return;
            const ancestors = Object.freeze([...context.ancestors]);
            const parentsCurrent = () => {
                if (!ancestors.every(parent => parent.current())) return false;
                for (let parent = editor.parentElement; parent; parent = parent.parentElement) {
                    if (parent.matches(':popover-open,dialog[open]') && !ancestors.some(a => a.owner === parent)) return false;
                }
                return true;
            };
            if (!parentsCurrent()) return;
            // Consume before host code, so reentrant admission cannot duplicate a watcher.
            used.add(event);
            let authority: CemPlatformCloseAuthority | undefined;
            try { authority = admit(Object.freeze({ event, editor, ancestors })); } catch { return; }
            try { if (!authority?.current() || !parentsCurrent() || !current()) return; } catch { return; }
            let watcher: NativeWatcher;
            try { watcher = new Constructor(); } catch { return; }
            let disposed = false;
            const observer = new MutationObserver(() => { if (!valid()) revoke(); });
            const releases: (() => void)[] = [];
            const valid = () => {
                try { return !disposed && editorAvailable() && authority.current() && parentsCurrent() && current(); }
                catch { return false; }
            };
            const dispose = () => {
                if (disposed) return;
                disposed = true; observer.disconnect(); watcher.removeEventListener('close', closed); watcher.destroy();
                for (const release of releases.splice(0)) release();
            };
            const revoke = () => { if (!disposed) { dispose(); dismiss('platform-authority'); } };
            const closed = (event: Event) => {
                if (!event.isTrusted || disposed) return;
                const accepted = valid(); dispose(); dismiss(accepted ? 'platform-close' : 'platform-authority');
            };
            watcher.addEventListener('close', closed);
            // Subscribe can synchronously revoke. Release late subscriptions too.
            const subscribe = (source: CemPlatformCloseAuthority) => {
                const release = source.subscribe(revoke);
                if (disposed) release(); else releases.push(release);
            };
            try { subscribe(authority); for (const parent of ancestors) subscribe(parent); }
            catch { revoke(); return; }
            if (!valid()) { revoke(); return; }
            observer.observe(view.document, { childList: true, subtree: true, attributes: true,
                attributeFilter: ['hidden', 'inert', 'disabled', 'readonly', 'open', 'popover', 'type', 'list'] });
            return Object.freeze({ get current() { return valid(); }, reconcile() { if (!valid()) revoke(); }, dispose });
        },
    });
}
