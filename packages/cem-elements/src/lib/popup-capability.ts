import { keynavEnabled } from './composite-navigation.js';
import { popupMenuEntry } from './composite-menu-capability.js';
import type { CheckpointEndpoint, CheckpointMenuRole, CheckpointPopup } from './checkpoint-interactions.js';
import { registerCemPopupRelay, containsCemMenuChain } from './menu-task-relay.js';
import { interactionReference, interactionControl, reportInteractionReference, observeInteractionReferences } from './interaction-reference.js';
import { focusSurface, surfaceElementAvailable, surfaceFocusTargetEligible } from './surface-references.js';
import type { CemProducedElementBehavior } from './cem-elements.js';
import { checkpointPopupGeometry, positionPopup, preparePopupGeometry, observePopupGeometry, releasePopupGeometry } from './popup-controller.js';

import { CemNativePopupBinding } from './native-popup-binding.js';

interface State { settled?: () => void; native?: CemNativePopupBinding; projected?: string; relayRevision: number; releaseRelay?: () => void; abort?: AbortController; observer?: MutationObserver; trigger?: HTMLElement; panel?: HTMLElement; open: boolean; releaseReferences?: () => void; releaseGeometry?: () => void; geometryReady?: boolean; authoredBase?: boolean; generatedBase?: { node: HTMLElement; hidden: HTMLElement['hidden'] }; external?: { trigger: HTMLElement; attributes: Map<string, string | null> }; authoredDisabled?: boolean; authoredHasPopup?: string; }
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
    if (!state) { state = { open: false, relayRevision: 0 }; states.set(host, state); }
    return state;
}
function setAttribute(node: Element, name: string, value: string | null): void {
    if (value === null) { if (node.hasAttribute(name)) node.removeAttribute(name); return; }
    if (node.getAttribute(name) !== value) node.setAttribute(name, value);
}
function synchronize(host: HTMLElement): void {
    const state = stateFor(host), trigger = selectedTrigger(host);
    const panel = host.querySelector<HTMLElement>(':scope > [part~="popup"]') ?? undefined;
    if (state.trigger !== trigger || state.panel !== panel) {
        state.native?.disconnect(); state.native = undefined;
        if (state.panel) releasePopupGeometry(state.panel);
        state.trigger = trigger; state.panel = panel; state.projected = undefined;
    }
    if (!trigger || !panel) { if (panel) panel.hidden = true; state.open = false; return; }
    if (!trigger.id) trigger.id = `cem-popup-trigger-${++sequence}`;
    const disabled = host.hasAttribute('disabled') || !!state.authoredDisabled || state.external?.attributes.get('aria-disabled') === 'true';
    if (!state.external && trigger instanceof HTMLButtonElement && trigger.disabled !== disabled) trigger.disabled = disabled;
    setAttribute(trigger, 'aria-disabled', String(disabled));
    const initial = !state.native;
    if (!state.native) state.native = new CemNativePopupBinding(host, trigger, panel, {
        admitted: () => !host.hidden && !host.hasAttribute('disabled') && !trigger.matches(':disabled,[aria-disabled="true"]'),
        prepare: () => preparePopupGeometry(trigger, panel, host),
        changed(open) {
            const previous = state.open; state.open = open;
            state.projected = String(open); setAttribute(host, 'open', state.projected);
            setAttribute(trigger, 'aria-expanded', state.projected);
            if (open) {
                state.geometryReady = positionPopup(trigger, panel, false, host);
                if (!state.geometryReady) state.native?.request(false);
            } else { if (previous) state.relayRevision++; releasePopupGeometry(panel); state.geometryReady = false; }
            state.settled?.();
        },
        entered: () => { if (state.open) focusSurface(host, panel, popupMenuEntry(panel, false)); },
        closing: () => { state.relayRevision++; },
    });
    reportInteractionReference(host, state.native.valid ? undefined : 'interaction-control-unsupported', 'native-invoker');
    setAttribute(trigger, 'aria-controls', panel.id);
    setAttribute(trigger, 'aria-haspopup', panel.querySelector('[role="menu"],[role="menubar"]') ? 'menu' : state.authoredHasPopup ?? null);
    setAttribute(panel, 'aria-labelledby', trigger.id);
    const requested = host.getAttribute('open') !== 'false' && !disabled && !host.hidden;
    if (initial || disabled || host.hidden || state.projected !== (host.getAttribute('open') ?? 'true')) {
        state.projected = host.getAttribute('open') ?? 'true';
        state.native.request(requested);
    }
    else state.native.sync();
    state.open = state.native.open;
    setAttribute(trigger, 'aria-expanded', String(state.open));
}
function setOpen(host: HTMLElement, open: boolean, restore = false): void {
    const state = stateFor(host);
    synchronize(host);
    state.native?.request(open, open, restore);
}
export const CEM_POPUP_CAPABILITY: CemProducedElementBehavior = {
    beforeRender(host, context) {
        const base = context.snapshot().payload.slots.base?.find(node => node.kind === 'element');
        stateFor(host).authoredBase = !!base;
        stateFor(host).authoredDisabled = base?.kind === 'element' && Object.hasOwn(base.attributes, 'disabled');
        stateFor(host).authoredHasPopup = base?.kind === 'element' ? base.attributes['aria-haspopup'] : undefined;
    },
    connected(host, context) {
        if (!host.ownerDocument.defaultView) return;
        const state = stateFor(host);
        if (state.abort) return;
        state.settled = () => context?.runtime.checkpointInteractionSettled(host);
        state.abort = new AbortController();
        popups.add(host);
        state.releaseRelay = registerCemPopupRelay({ host, panel: () => state.panel, launcher: () => state.trigger,
            revision: () => state.relayRevision,
            active: () => host.isConnected && state.open && host.getAttribute('open') !== 'false' && !!state.panel && !state.panel.hidden,
            dismiss: () => setOpen(host, false),
        });
        state.releaseReferences = observeInteractionReferences(host, () => { synchronize(host); position(); });
        const abort = state.abort, options = { signal: abort.signal };
        const keydown = (event: KeyboardEvent) => {
            if (event.defaultPrevented || event.isComposing || event.ctrlKey || event.metaKey || event.altKey) return;
            if (event.key === 'Tab' && state.open) {
                const inside = state.panel?.contains(host.ownerDocument.activeElement);
                setOpen(host, false);
                if (inside) state.trigger?.focus();
            } else if (event.target === state.trigger && !event.shiftKey && (event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
                if (!state.native?.valid || !state.panel || !state.trigger || !keynavEnabled(state.trigger, undefined, true) || state.trigger.matches(':disabled,[aria-disabled="true"]')) return;
                event.preventDefault();
                state.native.request(true);
                if (state.native.open) focusSurface(host, state.panel, popupMenuEntry(state.panel, event.key === 'ArrowUp'));
            }
        };
        host.addEventListener('keydown', keydown, options);
        host.addEventListener('cem-popup-dismiss', () => setOpen(host, false, true), options);
        host.addEventListener('focusout', event => {
            if (containsCemMenuChain(host, event.relatedTarget as Node | null) || event.relatedTarget === state.trigger) return;
            host.ownerDocument.defaultView?.setTimeout(() => {
                const active = host.ownerDocument.activeElement;
                // Reconciliation/blur can leave focus on the document, without
                // an outside destination. Native light-dismiss owns that case.
                if (state.abort === abort && !abort.signal.aborted && state.open && active
                    && active !== host.ownerDocument.body && active !== host.ownerDocument.documentElement
                    && !containsCemMenuChain(host, active) && active !== state.trigger) setOpen(host, false);
            }, 0);
        }, options);
        host.ownerDocument.addEventListener('keydown', event => {
            if (state.external && event.target === state.trigger) keydown(event);
        }, options);
        const view = host.ownerDocument.defaultView;
        const position = () => {
            if (state.open && state.trigger && state.panel
                && !positionPopup(state.trigger, state.panel, false, host) && host.getAttribute('anchor-lost') !== 'freeze') setOpen(host, false);
            state.settled?.();
        };
        state.releaseGeometry = observePopupGeometry(host, position);
        view.visualViewport?.addEventListener('resize', position, options);
        view.visualViewport?.addEventListener('scroll', position, options);
        view.addEventListener('resize', position, options);
        view.addEventListener('scroll', position, { ...options, capture: true });
        state.observer = new MutationObserver(records => {
            synchronize(host);
            if (records.some(record => ['dir', 'anchor', 'boundary', 'anchor-lost', 'data-cem-node-ref-anchor', 'data-cem-node-ref-boundary'].includes(record.attributeName ?? ''))) position();
        });
        state.observer.observe(host, { childList: true, subtree: true, attributes: true, attributeFilter: ['open', 'disabled', 'hidden', 'dir', 'trigger-for', 'data-cem-node-ref-trigger-for', 'focus-target', 'return-focus', 'anchor', 'boundary', 'anchor-lost', 'data-cem-node-ref-focus-target', 'data-cem-node-ref-return-focus', 'data-cem-node-ref-anchor', 'data-cem-node-ref-boundary'] });
    },
    rendered(host) { synchronize(host); },
    preserveRenderedAttribute(_host, _current, _desired, attribute) {
        const state = stateFor(_host);
        return (_current === state.panel || _current === state.trigger) && ['id', 'aria-controls', 'aria-haspopup', 'aria-expanded', 'aria-labelledby', 'hidden', 'style', 'popover', 'popovertarget', 'popovertargetaction', 'type'].includes(attribute.name);
    },
    disconnected(host) {
        const state = stateFor(host);
        state.settled = undefined;
        state.releaseRelay?.(); state.releaseRelay = undefined;
        state.abort?.abort(); state.abort = undefined; state.observer?.disconnect();
        state.releaseReferences?.(); state.releaseReferences = undefined;
        state.releaseGeometry?.(); state.releaseGeometry = undefined;
        state.native?.disconnect(); state.native = undefined; state.projected = undefined;
        if (state.panel) releasePopupGeometry(state.panel); state.geometryReady = false; popups.delete(host); releaseExternal(host);
    },
};

/** Read-only portable intent; selectors and serialized endpoints confer no authority. */
export function cemPopupCheckpoint(instance: HTMLElement, resolve?: (node: HTMLElement) => CheckpointEndpoint | undefined) {
    const state = states.get(instance);
    if (!instance.isConnected || !state?.abort || state.abort.signal.aborted || !state.panel || !state.trigger
        || !popups.has(instance) || !state.native?.valid || state.native.open !== state.open || !state.trigger.isConnected || !state.panel.isConnected
        || instance.querySelector(':scope > [part~="popup"]') !== state.panel || state.open && !state.geometryReady) return undefined;
    // Every open native owner needs current geometry, including fallback fitting.
    if (state.open && !checkpointPopupGeometry(state.trigger, state.panel, false, instance)) return undefined;
    const trigger = state.trigger.getAttribute('data-cem-render-node-id'), panel = state.panel.getAttribute('data-cem-render-node-id');
    if (!trigger || !panel) return undefined;
    const endpoint = (node: HTMLElement) => {
        const target = resolve?.(node);
        return target && (target.nodeId === 'host' || (node as HTMLElement & { cemRenderNodeId?: string }).cemRenderNodeId === target.nodeId) ? target : undefined;
    };
    const externalTrigger = state.external ? endpoint(state.trigger) : null;
    if (externalTrigger === undefined || externalTrigger && externalTrigger.nodeId !== trigger) return undefined;
    if (state.external) {
        const selected = interactionReference(instance, instance.getAttribute('trigger-for') ?? '', 'trigger-for');
        if (selected.code || interactionControl(selected.target) !== state.trigger || state.external.trigger !== state.trigger
            || [...popups].some(other => other !== instance && other.hasAttribute('trigger-for')
                && interactionControl(interactionReference(other, other.getAttribute('trigger-for') ?? '', 'trigger-for').target) === state.trigger)) return undefined;
    } else if (instance.hasAttribute('trigger-for') || instance.querySelector(':scope > [part~="base"] > :is(button,a,[tabindex])') !== state.trigger) return undefined;
    const role = (name: string, fallback: 'invoker' | 'viewport' | 'auto'): CheckpointMenuRole | undefined => {
        const value = instance.getAttribute(name) ?? fallback;
        if (!instance.hasAttribute(`data-cem-node-ref-${name}`) && (value === fallback || fallback === 'auto' && value === 'none'))
            return { mode: value as CheckpointMenuRole['mode'], target: null };
        const selected = interactionReference(instance, value, name), node = selected.target;
        if (selected.code || !(node instanceof HTMLElement) || !node.isConnected || node.getRootNode() !== instance.getRootNode()) return undefined;
        if (fallback === 'auto' && (!surfaceFocusTargetEligible(node) || name === 'focus-target' && !state.panel!.contains(node)
            || state.open && !surfaceElementAvailable(node))) return undefined;
        const target = endpoint(node); return target ? { mode: 'target', target } : undefined;
    };
    const anchor = role('anchor', 'invoker'), boundary = role('boundary', 'viewport');
    const focusTarget = role('focus-target', 'auto'), returnFocus = role('return-focus', 'auto');
    const anchorLost = instance.getAttribute('anchor-lost') ?? 'close';
    if (!anchor || !boundary || !focusTarget || !returnFocus || !['close', 'freeze'].includes(anchorLost)) return undefined;
    const contract: CheckpointPopup = { trigger, panel, scrollTop: state.panel.scrollTop, scrollLeft: state.panel.scrollLeft, disposition: 'reacquire',
        policies: { externalTrigger, anchor, boundary, focusTarget, returnFocus, anchorLost: anchorLost as 'close' | 'freeze', geometry: 'recompute' } };
    return { kind: 'popup' as const, open: state.open, state: contract };
}

/** Original controller/DOM evidence, including the control outside the popup subtree. */
export function prepareCemPopupGuard(host: HTMLElement, resolve: (node: HTMLElement) => CheckpointEndpoint | undefined): () => boolean {
    const state = states.get(host), saved = cemPopupCheckpoint(host, resolve);
    if (!state || !saved) throw Error('cem.checkpoint.resume.unsupported');
    const { abort, trigger, panel, external, generatedBase } = state;
    const root = host.getRootNode(), scope = host.closest('[interaction-scope]');
    const names = ['trigger-for', 'anchor', 'boundary', 'focus-target', 'return-focus', 'anchor-lost'];
    const policy = () => JSON.stringify(names.map(name => [host.getAttribute(name), host.getAttribute(`data-cem-node-ref-${name}`)]));
    const attributes = () => JSON.stringify(['id', 'aria-controls', 'aria-haspopup', 'aria-expanded', 'aria-disabled', 'disabled', 'tabindex'].map(name => trigger!.getAttribute(name)));
    const intent = JSON.stringify(saved), authored = policy(), control = attributes();
    return () => states.get(host) === state && state.abort === abort && !abort?.signal.aborted && state.trigger === trigger && state.panel === panel
        && state.external === external && state.generatedBase === generatedBase && host.getRootNode() === root && host.closest('[interaction-scope]') === scope
        && policy() === authored && attributes() === control && JSON.stringify(cemPopupCheckpoint(host, resolve)) === intent;
}

/** Renew geometry after fresh-owner admission without replaying open/focus actions. */
export function resumeCemPopupGeometry(host: HTMLElement): void {
    const state = states.get(host);
    if (!state?.trigger || !state.panel) throw new Error('cem.checkpoint.resume.popup');
    if (state.open && !positionPopup(state.trigger, state.panel, false, host)) throw new Error('cem.checkpoint.resume.geometry');
}
