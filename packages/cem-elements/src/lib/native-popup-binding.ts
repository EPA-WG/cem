import { isDomInstance } from './dom-realm.js';
import { nativeSurfaceVisible } from './surface-session.js';
import { registerCemNativeInvokerReturnPolicy } from './native-invoker-focus.js';
import { restoreSurfaceFocus } from './surface-references.js';

let sequence = 0;
const BINDINGS = Symbol.for('cem.native-popup-bindings.v1');
const environment = globalThis as typeof globalThis & { [BINDINGS]?: WeakMap<HTMLElement, CemNativePopupBinding> };
const bindings = environment[BINDINGS] ??= new WeakMap();
/** Only the original live runtime route can authorize retention/checkpointing. */
export function nativePopupBinding(control: HTMLElement): CemNativePopupBinding | undefined {
    const binding = bindings.get(control); return binding?.valid ? binding : undefined;
}
export function preserveNativePopupAttribute(current: Element, attribute: Attr): boolean {
    return isDomInstance(current, 'HTMLElement') && (bindings.get(current)?.ownsAttribute(current, attribute) ?? false);
}
interface Options {
    admitted(): boolean;
    prepare(): boolean;
    changed(open: boolean): void;
    entered(): void;
    closing(): void;
}
/** Native routes own activation and dismissal; semantics request entry separately. */
export class CemNativePopupBinding {
    private readonly abort: AbortController;
    private readonly policy;
    private readonly attributes = new Map<HTMLElement, Map<string, { before: string | null; value: string }>>();
    private readonly hidden: HTMLElement['hidden'];
    private last = false;
    private enter = false;
    private requesting = false;
    private opening = false;
    private closing = false;
    private previousFocus?: Element | null;
    private live = true;
    private readonly timers = new Set<number>();
    readonly supported: boolean;
    constructor(readonly host: HTMLElement, readonly trigger: HTMLElement, readonly panel: HTMLElement, private readonly options: Options) {
        const view = panel.ownerDocument.defaultView;
        if (!view) throw new TypeError('Native popup requires a browser document');
        this.abort = new view.AbortController(); this.hidden = panel.hidden;
        this.supported = Object.hasOwn(view.HTMLElement.prototype, 'popover') && isDomInstance(trigger, 'HTMLButtonElement')
            && (!trigger.hasAttribute('type') && !trigger.form || trigger.type === 'button')
            && trigger.getRootNode() === panel.getRootNode()
            && !trigger.hasAttribute('commandfor') && !trigger.hasAttribute('popovertarget');
        this.policy = registerCemNativeInvokerReturnPolicy(panel, host);
        if (!this.supported) { panel.hidden = true; return; }
        if (!panel.id) do { panel.id = `cem-native-popup-${++sequence}`; }
            while ((panel.getRootNode() as ParentNode).querySelectorAll(`[id="${panel.id}"]`).length > 1);
        this.claim(panel, 'popover', 'auto');
        this.claim(trigger, 'type', 'button');
        this.claim(trigger, 'popovertarget', panel.id);
        this.claim(trigger, 'popovertargetaction', 'toggle');
        bindings.set(trigger, this); bindings.set(panel, this);
        panel.hidden = !nativeSurfaceVisible(panel);
        const signal = this.abort.signal;
        trigger.addEventListener('click', () => { if (!this.requesting) this.enter = true; }, { capture: true, signal });
        panel.addEventListener('beforetoggle', event => {
            if (!event.isTrusted || event.target !== panel) return;
            if ((event as ToggleEvent).newState === 'open') {
                this.closing = false;
                this.opening = true;
                this.previousFocus = panel.ownerDocument.activeElement;
                this.policy.reset();
                panel.hidden = false;
                if (!this.valid || !options.admitted() || !options.prepare()) {
                    event.preventDefault(); panel.hidden = true; this.enter = false;
                }
            } else { this.closing = true; options.closing(); }
            // Observe the completed native transition, not its capture phase.
            const timer = view.setTimeout(() => { this.timers.delete(timer); this.opening = false; this.closing = false; this.sync(); }, 0);
            this.timers.add(timer);
        }, { signal });
        panel.addEventListener('toggle', event => { if (event.isTrusted && event.target === panel) { this.opening = false; this.closing = false; this.sync(); } }, { signal });
    }
    private claim(node: HTMLElement, name: string, value: string): void {
        let attributes = this.attributes.get(node);
        if (!attributes) { attributes = new Map(); this.attributes.set(node, attributes); }
        attributes.set(name, { before: node.getAttribute(name), value }); node.setAttribute(name, value);
    }
    get valid(): boolean {
        return this.live && this.supported && this.host.isConnected && this.trigger.isConnected && this.panel.isConnected
            && isDomInstance(this.trigger, 'HTMLButtonElement') && this.trigger.popoverTargetElement === this.panel
            && this.trigger.type === 'button' && this.trigger.popoverTargetAction === 'toggle' && !this.trigger.hasAttribute('commandfor') && this.panel.popover === 'auto';
    }
    ownsAttribute(node: HTMLElement, attribute: Attr): boolean {
        return this.valid && (node === this.panel && attribute.name === 'hidden'
            || this.attributes.get(node)?.get(attribute.name)?.value === attribute.value);
    }
    get open(): boolean { return this.valid && nativeSurfaceVisible(this.panel); }
    /** Arrows activate the invoker; compatibility intent uses the native API without replaying a click. */
    request(open: boolean, enter = false, restore = false): void {
        if (!this.live || this.closing) return;
        this.enter = enter;
        if (!open) {
            const inside = this.panel.contains(this.panel.ownerDocument.activeElement);
            if (!restore) this.policy.suppress();
            this.closing = true;
            try { if (nativeSurfaceVisible(this.panel)) this.panel.hidePopover(); }
            finally { this.closing = false; }
            const active = this.panel.ownerDocument.activeElement;
            if (restore && inside && (!active || active === this.trigger || active === this.previousFocus
                || active === this.panel.ownerDocument.body || this.panel.contains(active))) restoreSurfaceFocus(this.host, this.trigger);
        } else if (this.valid && this.options.admitted() && !this.open) {
            if (enter) {
                this.requesting = true;
                try { this.trigger.click(); } finally { this.requesting = false; }
            } else this.panel.showPopover({ source: this.trigger });
        }
        this.sync();
        if (enter && this.open && this.enter) { this.enter = false; this.options.entered(); }
    }
    sync(): void {
        if (!this.live || this.closing) return;
        if (this.opening && !nativeSurfaceVisible(this.panel)) return;
        if (!this.valid && nativeSurfaceVisible(this.panel)) { this.request(false); return; }
        const open = this.open;
        // A new authored hidden request must not be overwritten by the mirror.
        if (open && this.last && this.panel.hidden) { this.request(false); return; }
        if (this.panel.hidden !== !open) this.panel.hidden = !open;
        if (this.last !== open) { this.last = open; this.options.changed(open); }
        if (open && this.enter) { this.enter = false; this.options.entered(); }
    }
    disconnect(): void {
        if (!this.live) return;
        this.request(false); this.live = false; this.abort.abort(); this.policy.release();
        if (bindings.get(this.trigger) === this) bindings.delete(this.trigger);
        if (bindings.get(this.panel) === this) bindings.delete(this.panel);
        const view = this.panel.ownerDocument.defaultView;
        for (const timer of this.timers) view?.clearTimeout(timer);
        this.timers.clear();
        for (const [node, attributes] of this.attributes) for (const [name, claim] of attributes) if (node.getAttribute(name) === claim.value) {
            if (claim.before === null) node.removeAttribute(name); else node.setAttribute(name, claim.before);
        }
        this.attributes.clear(); this.panel.hidden = this.hidden;
    }
}
