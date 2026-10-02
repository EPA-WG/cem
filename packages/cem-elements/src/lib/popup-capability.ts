import type { CemProducedElementBehavior } from './cem-elements.js';
import { firstPopupControl, hidePopup, positionPopup, showPopup } from './popup-controller.js';

interface State { abort?: AbortController; observer?: MutationObserver; trigger?: HTMLElement; panel?: HTMLElement; open: boolean; authoredDisabled?: boolean; authoredHasPopup?: string; }
const states = new WeakMap<HTMLElement, State>();
let sequence = 0;
function stateFor(host: HTMLElement): State {
    let state = states.get(host);
    if (!state) { state = { open: false }; states.set(host, state); }
    return state;
}
function setAttribute(node: Element, name: string, value: string | null): void {
    if (value === null) { if (node.hasAttribute(name)) node.removeAttribute(name); return; }
    if (node.getAttribute(name) !== value) node.setAttribute(name, value);
}
function synchronize(host: HTMLElement): void {
    const state = stateFor(host);
    state.trigger = host.querySelector<HTMLElement>(':scope > [part~="base"] > :is(button,a,[tabindex])') ?? undefined;
    state.panel = host.querySelector<HTMLElement>(':scope > [part~="popup"]') ?? undefined;
    const { trigger, panel } = state;
    if (!trigger || !panel) return;
    if (!panel.id) panel.id = `cem-popup-panel-${++sequence}`;
    if (!trigger.id) trigger.id = `cem-popup-trigger-${++sequence}`;
    const disabled = host.hasAttribute('disabled') || !!state.authoredDisabled;
    if (trigger instanceof HTMLButtonElement && trigger.disabled !== disabled) trigger.disabled = disabled;
    setAttribute(trigger, 'aria-disabled', String(host.hasAttribute('disabled') || !!state.authoredDisabled));
    setAttribute(trigger, 'aria-controls', panel.id);
    setAttribute(trigger, 'aria-haspopup', panel.querySelector('[role="menu"],[role="menubar"]') ? 'menu' : state.authoredHasPopup ?? null);
    setAttribute(panel, 'aria-labelledby', trigger.id);
    state.open = host.getAttribute('open') !== 'false' && !host.hasAttribute('disabled') && !host.hidden;
    setAttribute(trigger, 'aria-expanded', String(state.open));
    if (state.open) showPopup(trigger, panel); else hidePopup(panel);
}
function setOpen(host: HTMLElement, open: boolean, restore = false): void {
    const state = stateFor(host);
    if (host.hasAttribute('disabled') && open) return;
    host.setAttribute('open', String(open));
    synchronize(host);
    if (open && state.panel) firstPopupControl(state.panel)?.focus();
    else if (restore) state.trigger?.focus();
}
export const CEM_POPUP_CAPABILITY: CemProducedElementBehavior = {
    beforeRender(host, context) {
        const base = context.snapshot().payload.slots.base?.find(node => node.kind === 'element');
        stateFor(host).authoredDisabled = base?.kind === 'element' && Object.hasOwn(base.attributes, 'disabled');
        stateFor(host).authoredHasPopup = base?.kind === 'element' ? base.attributes['aria-haspopup'] : undefined;
    },
    connected(host) {
        if (!host.ownerDocument.defaultView) return;
        const state = stateFor(host);
        if (state.abort) return;
        state.abort = new AbortController();
        const options = { signal: state.abort.signal };
        host.addEventListener('click', event => {
            synchronize(host);
            if (state.trigger?.contains(event.target as Node)) {
                event.preventDefault();
                if (host.hasAttribute('disabled')) { event.stopImmediatePropagation(); return; }
                setOpen(host, !state.open, state.open);
            }
        }, { ...options, capture: true });
        host.addEventListener('keydown', event => {
            if (event.defaultPrevented) return;
            if (event.key === 'Escape' && state.open) { event.preventDefault(); event.stopPropagation(); setOpen(host, false, true); }
            else if (event.key === 'Tab' && state.open) setOpen(host, false);
            else if (event.target === state.trigger && event.key === 'ArrowDown') { event.preventDefault(); setOpen(host, true); }
        }, options);
        host.addEventListener('cem-popup-dismiss', () => setOpen(host, false), options);
        host.addEventListener('focusout', () => queueMicrotask(() => {
            if (state.open && !host.contains(host.ownerDocument.activeElement)) setOpen(host, false);
        }), options);
        for (const eventName of ['pointerdown', 'click']) host.ownerDocument.addEventListener(eventName, event => {
            if (state.open && !host.contains(event.target as Node)) setOpen(host, false);
        }, { ...options, capture: true });
        const view = host.ownerDocument.defaultView;
        const position = () => { if (state.trigger && state.panel && state.open) positionPopup(state.trigger, state.panel); };
        view.addEventListener('resize', position, options);
        view.addEventListener('scroll', position, { ...options, capture: true });
        state.observer = new MutationObserver(() => synchronize(host));
        state.observer.observe(host, { childList: true, subtree: true, attributes: true, attributeFilter: ['open', 'disabled', 'hidden', 'dir'] });
    },
    rendered(host) { synchronize(host); },
    preserveRenderedAttribute(_host, _current, _desired, attribute) {
        const state = stateFor(_host);
        return (_current === state.panel || _current === state.trigger) && ['id', 'aria-controls', 'aria-haspopup', 'aria-expanded', 'aria-labelledby', 'hidden', 'style'].includes(attribute.name);
    },
    disconnected(host) {
        const state = stateFor(host);
        state.abort?.abort(); state.abort = undefined; state.observer?.disconnect();
    },
};
