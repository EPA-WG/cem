import { isDomInstance } from './dom-realm.js';
import { hasCemSurfaceOwner, nativeSurfaceVisible } from './surface-session.js';

type Root = Document | ShadowRoot;
type Invoker = HTMLButtonElement | HTMLInputElement;
interface Route { target: HTMLElement; action: string }
interface Session {
    owner: HTMLElement;
    source: Invoker;
    parent?: Session;
    order: number;
    closing?: { inside: boolean; blocked: boolean; input: number };
}
interface Subscription { users: number; stop(): void }
const ROOTS = Symbol.for('cem.native-invoker-focus.v1');
const environment = globalThis as typeof globalThis & { [ROOTS]?: WeakMap<Root, Subscription> };
const roots = environment[ROOTS] ??= new WeakMap();

/** Read the native relationship, including IDL element references and shadow scopes. */
function route(node: unknown): Route | undefined {
    if (!isDomInstance(node, 'HTMLButtonElement') && !isDomInstance(node, 'HTMLInputElement')) return;
    if (node.disabled || node.matches(':disabled') || isDomInstance(node, 'HTMLInputElement') && node.type !== 'button') return;
    const command = isDomInstance(node, 'HTMLButtonElement') ? node.commandForElement : null;
    const commandName = isDomInstance(node, 'HTMLButtonElement') ? (node.command ?? node.getAttribute('command') ?? '').toLowerCase() : '';
    const popover = node.popoverTargetElement;
    if (command && popover || node.hasAttribute('commandfor') && node.hasAttribute('popovertarget')) return;
    const target = command ?? popover;
    const action = command ? commandName.replace(/-popover$/, '') : node.popoverTargetAction;
    if (command && !['toggle-popover', 'show-popover', 'hide-popover'].includes(commandName)) return;
    return isDomInstance(target, 'HTMLElement') && target.hasAttribute('popover') ? { target, action } : undefined;
}
function usable(source: Invoker, owner: HTMLElement, root: Root): boolean {
    if (!source.isConnected || !owner.isConnected || source.getRootNode() !== root || owner.getRootNode() !== root
        || source.closest('[hidden]:not([hidden="until-found"]),[inert],[aria-disabled="true"]')) return false;
    const binding = route(source), style = source.ownerDocument.defaultView?.getComputedStyle(source);
    return binding?.target === owner && binding.action !== 'hide' && source.getClientRects().length > 0
        && style?.visibility !== 'hidden' && style?.visibility !== 'collapse';
}
function active(root: Root): Element | null {
    return root.activeElement ?? (isDomInstance(root, 'ShadowRoot') ? root.ownerDocument.activeElement : null);
}

/**
 * Supplement focus return for native invokers. Visibility, entry focus, roles,
 * ARIA and navigation remain native or belong to an existing semantic owner.
 * Root delegation covers invokers/targets added after connection; no tag registry.
 */
export function connectCemNativeInvokerFocus(root: Root): () => void {
    if (!isDomInstance(root, 'Document') && !isDomInstance(root, 'ShadowRoot')) throw new TypeError('Native invoker focus requires a Document or ShadowRoot');
    const document = isDomInstance(root, 'Document') ? root : root.ownerDocument;
    const view = document.defaultView;
    if (!view || !Object.hasOwn(view.HTMLElement.prototype, 'popover')) return () => undefined;
    let subscription = roots.get(root);
    if (!subscription) {
        const abort = new view.AbortController(), options = { capture: true, signal: abort.signal };
        const sessions = new Map<HTMLElement, Session>();
        let observer: MutationObserver;
        let reconcile: (records: MutationRecord[]) => void;
        let live = true, order = 0, input = 0;
        let intent: { kind: 'pointer' | 'tab' | 'escape' | 'activation'; target: Node | null } | undefined;
        let activation: { source: Invoker; binding: Route } | undefined;
        const timers = new Set<number>();
        const afterDispatch = (callback: () => void) => {
            const timer = view.setTimeout(() => { timers.delete(timer); if (live) callback(); }, 0);
            timers.add(timer);
        };
        const deferClear = () => {
            const previousIntent = intent, previousActivation = activation;
            afterDispatch(() => {
                if (intent === previousIntent) intent = undefined;
                if (activation === previousActivation) activation = undefined;
            });
        };
        root.addEventListener('pointerdown', event => {
            input++; intent = { kind: 'pointer', target: event.composedPath()[0] as Node }; activation = undefined;
        }, options);
        root.addEventListener('pointerup', deferClear, options);
        root.addEventListener('keydown', event => {
            input++; activation = undefined;
            const key = (event as KeyboardEvent).key;
            intent = key === 'Tab' || key === 'Escape' ? { kind: key === 'Tab' ? 'tab' : 'escape', target: event.composedPath()[0] as Node } : undefined;
            deferClear();
        }, options);
        root.addEventListener('click', event => {
            input++; intent = { kind: 'activation', target: event.composedPath()[0] as Node };
            const source = event.composedPath().find(node => route(node)) as Invoker | undefined;
            const binding = source && route(source);
            activation = source && binding ? { source, binding } : undefined;
            deferClear();
        }, options);
        root.addEventListener('beforetoggle', event => {
            const owner = event.target;
            if (!event.isTrusted || !isDomInstance(owner, 'HTMLElement') || !owner.hasAttribute('popover')) return;
            if (hasCemSurfaceOwner(owner)) { sessions.delete(owner); return; }
            const toggle = event as ToggleEvent;
            if (toggle.newState === 'open') {
                reconcile(observer.takeRecords());
                const nativeSource = toggle.source;
                const source = route(nativeSource)?.target === owner ? nativeSource as Invoker
                    : !('source' in toggle) && activation?.binding.target === owner && activation.binding.action !== 'hide' ? activation.source : undefined;
                if (!source || route(source)?.action === 'hide') { sessions.delete(owner); return; }
                const parent = [...sessions.values()].filter(session => nativeSurfaceVisible(session.owner)
                    && (session.owner.contains(source) || session.owner.contains(owner.parentNode)))
                    .sort((a, b) => b.order - a.order)[0];
                const session: Session = { owner, source, parent, order: ++order };
                sessions.set(owner, session);
                // beforetoggle is cancelable; its capture phase cannot assume admission.
                afterDispatch(() => {
                    if (sessions.get(owner) === session && (event.defaultPrevented || !nativeSurfaceVisible(owner) && !session.closing)) sessions.delete(owner);
                });
            } else {
                const session = sessions.get(owner);
                if (!session) return;
                const descendsFrom = (child: Session) => {
                    for (let parent = child.parent; parent; parent = parent.parent) if (parent === session) return true;
                    return false;
                };
                const inside = owner.contains(active(root)) || [...sessions.values()].some(child => child.closing?.inside
                    && !child.closing.blocked && child.closing.input === input && descendsFrom(child));
                session.closing = { inside, input,
                    blocked: intent?.kind === 'tab' || !!intent && ['pointer', 'activation'].includes(intent.kind) && !owner.contains(intent.target) };
            }
        }, options);
        root.addEventListener('toggle', event => {
            const owner = event.target;
            if (!event.isTrusted || !isDomInstance(owner, 'HTMLElement') || (event as ToggleEvent).newState !== 'closed') return;
            const session = sessions.get(owner), closing = session?.closing;
            if (!session || !closing || nativeSurfaceVisible(owner)) return;
            sessions.delete(owner);
            if (!live || hasCemSurfaceOwner(owner) || !closing.inside || closing.blocked || closing.input !== input || !usable(session.source, owner, root)) return;
            // A native cascade restores only the outer surviving invoker, never
            // successive child invokers that have themselves become hidden.
            for (let parent = session.parent; parent; parent = parent.parent) if (parent.closing && !nativeSurfaceVisible(parent.owner)) return;
            const focused = active(root);
            if (focused === session.source) return; // Preserve correct native restoration.
            if (!focused || focused === document.body || focused === document.documentElement || owner.contains(focused)) session.source.focus();
        }, options);
        reconcile = records => {
            if (!sessions.size) return;
            const changed = new Set<MutationRecord>();
            const after = new Map<Node, Map<string, string | null>>();
            for (const record of [...records].reverse()) if (record.type === 'attributes' && record.attributeName) {
                const name = record.attributeName, target = record.target as Element;
                let values = after.get(target);
                if (!values) { values = new Map(); after.set(target, values); }
                const value = values.has(name) ? values.get(name) : target.getAttribute(name);
                if (record.oldValue !== value) changed.add(record);
                values.set(name, record.oldValue);
            }
            for (const [owner, session] of sessions) {
                const removed = records.some(record => record.type === 'childList' && [...record.removedNodes].some(node => node.contains(owner) || node.contains(session.source)));
                const rebound = records.some(record => changed.has(record)
                    && (record.target === session.source && ['popovertarget', 'popovertargetaction', 'commandfor', 'command'].includes(record.attributeName ?? '')
                        || record.target === owner && record.attributeName === 'id'));
                if (removed || rebound || !owner.isConnected || !session.source.isConnected || route(session.source)?.target !== owner || hasCemSurfaceOwner(owner)) sessions.delete(owner);
            }
        };
        observer = new view.MutationObserver(reconcile);
        observer.observe(root, { subtree: true, childList: true, attributes: true, attributeOldValue: true,
            attributeFilter: ['id', 'popovertarget', 'popovertargetaction', 'commandfor', 'command', 'popover', 'disabled', 'type'] });
        subscription = { users: 0, stop: () => { live = false; abort.abort(); observer.disconnect(); for (const timer of timers) view.clearTimeout(timer); timers.clear(); sessions.clear(); activation = undefined; intent = undefined; } };
        roots.set(root, subscription);
    }
    const current = subscription;
    current.users++;
    let released = false;
    return () => {
        if (released) return; released = true;
        if (--current.users === 0) { current.stop(); roots.delete(root); }
    };
}
