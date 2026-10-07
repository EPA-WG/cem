import { interactionReference, interactionControl, reportInteractionReference, observeInteractionReferences } from './interaction-reference.js';
import { focusSurface, restoreSurfaceFocus } from './surface-references.js';
import type { CemProducedElementBehavior } from './cem-elements.js';
import { hidePopup, positionPopup, showPopup, observePopupGeometry, releasePopupGeometry } from './popup-controller.js';

interface State { abort?: AbortController; observer?: MutationObserver; trigger?: HTMLElement; panel?: HTMLElement; open: boolean; releaseReferences?: () => void; releaseGeometry?: () => void; geometryReady?: boolean; authoredBase?: boolean; generatedBase?: { node: HTMLElement; hidden: HTMLElement['hidden'] }; external?: { trigger: HTMLElement; attributes: Map<string, string | null> }; authoredDisabled?: boolean; authoredHasPopup?: string; }
const states = new WeakMap<HTMLElement, State>();
let sequence = 0;
const popups = new Set<HTMLElement>();
function releaseExternal(host: HTMLElement): void {
    const state = stateFor(host);
    if (state.external) {
        for (const [name, value] of state.external.attributes) setAttribute(state.external.trigger, name, value);
        state.external = undefined;
    }
    if (state.generatedBase) { state.generatedBase.node.hidden = state.generatedBase.hidden; state.generatedBase = undefined; }
}
function selectedTrigger(host: HTMLElement): HTMLElement | undefined {
    const state = stateFor(host);
    if (!host.hasAttribute('trigger-for')) { releaseExternal(host); reportInteractionReference(host); return host.querySelector<HTMLElement>(':scope > [part~="base"] > :is(button,a,[tabindex])') ?? undefined; }
    const selected = interactionReference(host, host.getAttribute('trigger-for') ?? '', 'trigger-for');
    const trigger = interactionControl(selected.target);
    let code = selected.code ?? (!trigger || state.authoredBase || host.contains(trigger) ? 'interaction-reference-conflict' : undefined);
    if (trigger && [...popups].some(other => other !== host && other.hasAttribute('trigger-for') && interactionControl(interactionReference(other, other.getAttribute('trigger-for') ?? '', 'trigger-for').target) === trigger)) code = 'interaction-reference-conflict';
    if (code || !trigger) { releaseExternal(host); reportInteractionReference(host, code); return; }
    reportInteractionReference(host);
    if (state.external?.trigger !== trigger) {
        releaseExternal(host);
        state.external = { trigger, attributes: new Map(['aria-controls', 'aria-haspopup', 'aria-expanded', 'aria-disabled'].map(name => [name, trigger.getAttribute(name)])) };
    }
    const base = host.querySelector<HTMLElement>(':scope > [part~="base"]');
    if (base && state.generatedBase?.node !== base) {
        if (state.generatedBase) state.generatedBase.node.hidden = state.generatedBase.hidden;
        state.generatedBase = { node: base, hidden: base.hidden };
    }
    if (base && !base.hidden) base.hidden = true;
    return trigger;
}
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
    const previousPanel = state.panel;
    const wasOpen = state.open;
    state.trigger = selectedTrigger(host);
    state.panel = host.querySelector<HTMLElement>(':scope > [part~="popup"]') ?? undefined;
    const { trigger, panel } = state;
    if (previousPanel && previousPanel !== panel) releasePopupGeometry(previousPanel);
    if (!trigger || !panel) { if (panel) hidePopup(panel); state.open = false; return; }
    if (!panel.id) panel.id = `cem-popup-panel-${++sequence}`;
    if (!trigger.id) trigger.id = `cem-popup-trigger-${++sequence}`;
    const disabled = host.hasAttribute('disabled') || !!state.authoredDisabled || state.external?.attributes.get('aria-disabled') === 'true';
    if (!state.external && trigger instanceof HTMLButtonElement && trigger.disabled !== disabled) trigger.disabled = disabled;
    setAttribute(trigger, 'aria-disabled', String(disabled));
    setAttribute(trigger, 'aria-controls', panel.id);
    setAttribute(trigger, 'aria-haspopup', panel.querySelector('[role="menu"],[role="menubar"]') ? 'menu' : state.authoredHasPopup ?? null);
    setAttribute(panel, 'aria-labelledby', trigger.id);
    state.open = host.getAttribute('open') !== 'false' && !host.hasAttribute('disabled') && !host.hidden;
    setAttribute(trigger, 'aria-expanded', String(state.open));
    if (state.open) {
        const ready = !wasOpen || previousPanel !== panel || panel.hidden ? showPopup(trigger, panel, false, host) :
            host.hasAttribute('anchor') || host.hasAttribute('boundary') || getComputedStyle(panel).position !== 'absolute' ? positionPopup(trigger, panel, false, host) : true;
        if (ready) state.geometryReady = true;
        else if (!wasOpen || !state.geometryReady || host.getAttribute('anchor-lost') !== 'freeze') {
            state.open = false; host.setAttribute('open', 'false'); hidePopup(panel); setAttribute(trigger, 'aria-expanded', 'false');
        }
    } else hidePopup(panel);
}
function setOpen(host: HTMLElement, open: boolean, restore = false): void {
    const state = stateFor(host);
    if (host.hasAttribute('disabled') && open) return;
    host.setAttribute('open', String(open));
    synchronize(host);
    if (state.open && state.panel && state.trigger) {
        positionPopup(state.trigger, state.panel, false, host);
        focusSurface(host, state.panel);
    }
    else if (restore) restoreSurfaceFocus(host, state.trigger);
}
export const CEM_POPUP_CAPABILITY: CemProducedElementBehavior = {
    beforeRender(host, context) {
        const base = context.snapshot().payload.slots.base?.find(node => node.kind === 'element');
        stateFor(host).authoredBase = !!base;
        stateFor(host).authoredDisabled = base?.kind === 'element' && Object.hasOwn(base.attributes, 'disabled');
        stateFor(host).authoredHasPopup = base?.kind === 'element' ? base.attributes['aria-haspopup'] : undefined;
    },
    connected(host) {
        if (!host.ownerDocument.defaultView) return;
        const state = stateFor(host);
        if (state.abort) return;
        state.abort = new AbortController();
        popups.add(host);
        state.releaseReferences = observeInteractionReferences(host, () => synchronize(host));
        const options = { signal: state.abort.signal };
        host.addEventListener('click', event => {
            synchronize(host);
            if (state.trigger?.contains(event.target as Node) && !state.external) {
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
            if (state.open && !host.contains(host.ownerDocument.activeElement) && host.ownerDocument.activeElement !== state.trigger) setOpen(host, false);
        }), options);
        host.ownerDocument.addEventListener('click', event => {
            synchronize(host);
            if (!state.external || !state.trigger?.contains(event.target as Node)) return;
            event.preventDefault();
            if (host.hasAttribute('disabled') || state.trigger.matches(':disabled,[aria-disabled="true"]')) return;
            setOpen(host, !state.open, state.open);
        }, { ...options, capture: true });
        host.ownerDocument.addEventListener('keydown', event => {
            if (state.external && event.target === state.trigger && event.key === 'ArrowDown' && !state.trigger?.matches(':disabled,[aria-disabled="true"]')) { event.preventDefault(); setOpen(host, true); }
        }, options);
        for (const eventName of ['pointerdown', 'click']) host.ownerDocument.addEventListener(eventName, event => {
            if (state.open && !host.contains(event.target as Node) && !state.trigger?.contains(event.target as Node)) setOpen(host, false);
        }, { ...options, capture: true });
        const view = host.ownerDocument.defaultView;
        const position = () => {
            if (!state.open) return;
            synchronize(host);
            if (state.open && state.trigger && state.panel && getComputedStyle(state.panel).position === 'absolute' &&
                !host.hasAttribute('anchor') && !host.hasAttribute('boundary') &&
                !positionPopup(state.trigger, state.panel, false, host) && host.getAttribute('anchor-lost') !== 'freeze') setOpen(host, false);
        };
        state.releaseGeometry = observePopupGeometry(host, position);
        view.visualViewport?.addEventListener('resize', position, options);
        view.visualViewport?.addEventListener('scroll', position, options);
        view.addEventListener('resize', position, options);
        // Absolute panels follow their containing block during scrolling. Re-clamping
        // their coordinates on every scroll would pin them to the viewport edge.
        view.addEventListener('scroll', () => {
            if (host.hasAttribute('anchor') || host.hasAttribute('boundary') || state.panel && getComputedStyle(state.panel).position !== 'absolute') position();
        }, { ...options, capture: true });
        state.observer = new MutationObserver(records => {
            synchronize(host);
            if (records.some(record => record.attributeName === 'dir')) position();
        });
        state.observer.observe(host, { childList: true, subtree: true, attributes: true, attributeFilter: ['open', 'disabled', 'hidden', 'dir', 'trigger-for', 'data-cem-node-ref-trigger-for', 'focus-target', 'return-focus', 'anchor', 'boundary', 'anchor-lost', 'data-cem-node-ref-focus-target', 'data-cem-node-ref-return-focus', 'data-cem-node-ref-anchor', 'data-cem-node-ref-boundary'] });
    },
    rendered(host) { synchronize(host); },
    preserveRenderedAttribute(_host, _current, _desired, attribute) {
        const state = stateFor(_host);
        return (_current === state.panel || _current === state.trigger) && ['id', 'aria-controls', 'aria-haspopup', 'aria-expanded', 'aria-labelledby', 'hidden', 'style'].includes(attribute.name);
    },
    disconnected(host) {
        const state = stateFor(host);
        state.abort?.abort(); state.abort = undefined; state.observer?.disconnect();
        state.releaseReferences?.(); state.releaseReferences = undefined;
        state.releaseGeometry?.(); state.releaseGeometry = undefined;
        if (state.panel) releasePopupGeometry(state.panel); state.geometryReady = false; popups.delete(host); releaseExternal(host);
    },
};
