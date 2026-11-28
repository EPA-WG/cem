import type { CheckpointEndpoint, CheckpointMenu, CheckpointMenuPolicies, CheckpointMenuRole } from './checkpoint-interactions.js';
import { registerCemMenuRelay, resetCemTaskActivation, consumeCemTaskActivation } from './menu-task-relay.js';
import { interactionReference, interactionControl, reportInteractionReference, observeInteractionReferences } from './interaction-reference.js';
import { focusSurface, surfaceElementAvailable, surfaceFocusTargetEligible } from './surface-references.js';
import type { CemProducedElementBehavior } from './cem-elements.js';
import { compositeIndex, keynavEnabled } from './composite-navigation.js';
import { checkpointPopupGeometry, preparePopupGeometry, positionPopup, observePopupGeometry, releasePopupGeometry } from './popup-controller.js';

import { CemNativePopupBinding } from './native-popup-binding.js';

// Direct composite parts and projected submenu slots are the declaration contract.
const COMPOSITE = '[part~="composite"]';
const originals = new WeakMap<HTMLElement, { owner: HTMLElement; role: string | null; tabindex: string | null; disabled: string | null }>();
const relationshipAttributes = new WeakMap<HTMLElement, Map<string, string | null>>();
function releaseRelationship(control: HTMLElement): void {
    const attributes = relationshipAttributes.get(control);
    if (!attributes) return;
    for (const [name, value] of attributes) setAttribute(control, name, value);
    relationshipAttributes.delete(control);
}
const states = new WeakMap<HTMLElement, State>();
let sequence = 0;
const menus = new Set<HTMLElement>();
interface State {
    settled?: () => void;
    native?: CemNativePopupBinding;
    entryLast?: boolean;
    pendingEntry?: { last: boolean; origin: Element | null };
    controls: Set<HTMLElement>;
    relayRevision: number;
    queuedActivations: number;
    active?: HTMLElement;
    open?: HTMLElement;
    openPanel?: HTMLElement;
    parent?: HTMLElement;
    relationshipHidden?: { value: HTMLElement['hidden'] };
    releaseReferences?: () => void;
    releaseRelay?: () => void;
    releaseGeometry?: () => void;
    geometryReady?: boolean;
    buffer: string;
    typedAt: number;
    language?: string;
    abort?: AbortController;
    observer?: MutationObserver;
    syncing: boolean;
}
function stateFor(host: HTMLElement): State {
    let state = states.get(host);
    if (!state) { state = { controls: new Set(), relayRevision: 0, queuedActivations: 0, buffer: '', typedAt: 0, syncing: false }; states.set(host, state); }
    return state;
}
function container(host: HTMLElement): HTMLElement | null { return host.querySelector<HTMLElement>(`:scope > ${COMPOSITE}`); }
function owner(panel: HTMLElement): HTMLElement | null {
    const nested = panel.matches('[slot="submenu"]') ? panel.parentElement : null;
    if (!panel.hasAttribute('parent-item')) return nested;
    const selected = interactionReference(panel, panel.getAttribute('parent-item') ?? '', 'parent-item');
    const control = interactionControl(selected.target);
    const parent = control?.closest(COMPOSITE)?.parentElement;
    const code = selected.code ?? (!control || !parent || control.closest('cem-application') !== panel.closest('cem-application') || !menus.has(parent) || !controls(parent).includes(control)
        || (nested && !nested.contains(control)) ? 'interaction-reference-conflict' : undefined);
    if (code) reportInteractionReference(panel, code);
    return code ? null : control ?? null;
}
function parentMenu(host: HTMLElement): HTMLElement | null { return owner(host)?.closest(COMPOSITE)?.parentElement ?? null; }
function rootMenu(host: HTMLElement): HTMLElement | null {
    const visited = new Set<HTMLElement>();
    while (true) {
        if (visited.has(host)) { reportInteractionReference(host, 'interaction-reference-conflict'); return null; }
        visited.add(host);
        const parent = parentMenu(host);
        if (!parent) return host.hasAttribute('parent-item') && !owner(host) ? null : host;
        host = parent;
    }
}
function enabled(host: HTMLElement, origin = container(host) ?? host): boolean {
    if (!rootMenu(host)) return false;
    return keynavEnabled(origin, node => menus.has(node) && node.hasAttribute('parent-item') ? owner(node) : null);
}
function insideChain(host: HTMLElement, node: Node | null): boolean {
    if (!node) return false;
    const root = rootMenu(host);
    return (root ?? host).contains(node) || !!root && [...menus].some(menu => rootMenu(menu) === root && menu.contains(node));
}
function relationshipsChanged(host: HTMLElement): void {
    const state = stateFor(host);
    const parent = parentMenu(host) ?? undefined;
    if (state.parent && state.parent !== parent && stateFor(state.parent).openPanel === host) close(state.parent, true);
    if (state.parent !== parent) { state.native?.disconnect(); state.native = undefined; state.buffer = ''; }
    state.parent = parent;
    const trigger = owner(host);
    if (rootMenu(host) && (!parent || trigger && panelFor(trigger) === host)) reportInteractionReference(host);
    if (!parent && !host.hasAttribute('parent-item') && !host.matches('[slot="submenu"]') && state.relationshipHidden) {
        host.hidden = state.relationshipHidden.value;
        state.relationshipHidden = undefined;
    }
    synchronize(host);
    if (parent) synchronize(parent);
}
function column(host: HTMLElement): boolean {
    return (host.getAttribute('direction') ?? (owner(host) ? 'column' : 'row')) === 'column';
}
function controls(host: HTMLElement): HTMLElement[] {
    const box = container(host);
    if (!box) return [];
    return Array.from(box.children).flatMap(child => {
        if (child.matches('a,button')) return [child as HTMLElement];
        return Array.from(child.querySelectorAll<HTMLElement>(':scope > [part~="control"]'));
    });
}
function unavailable(control: HTMLElement): boolean {
    return !!control.closest('[hidden]:not([hidden="until-found"]),[inert],[disabled],[aria-disabled="true"]') ||
        control.getClientRects().length === 0 || getComputedStyle(control).visibility === 'hidden';
}
function items(host: HTMLElement): HTMLElement[] {
    return controls(host).filter(control => !control.matches(':disabled') && !control.closest('[inert],[hidden]') &&
        control.getClientRects().length > 0 && getComputedStyle(control).visibility !== 'hidden' && enabled(host, control));
}
function panelFor(control: HTMLElement): HTMLElement | null {
    const nested = control.parentElement?.querySelector<HTMLElement>(`:scope > [slot="submenu"]`) ?? null;
    const panels = new Set<HTMLElement>(nested && (!nested.hasAttribute('parent-item') || owner(nested) === control && rootMenu(nested)) ? [nested] : []);
    for (const panel of menus) if (panel.hasAttribute('parent-item') && owner(panel) === control && rootMenu(panel)) panels.add(panel);
    if (panels.size > 1) {
        for (const panel of panels) { claimPanel(panel); reportInteractionReference(panel, 'interaction-reference-conflict'); stateFor(panel).native?.request(false); }
        return null;
    }
    const panel = [...panels][0];
    if (panel) claimPanel(panel);
    return panel ?? null;
}
function claimPanel(panel: HTMLElement): void {
    stateFor(panel).relationshipHidden ??= { value: panel.hidden };
}
function setAttribute(node: Element, name: string, value: string | null): void {
    if (value === null) { if (node.hasAttribute(name)) node.removeAttribute(name); }
    else if (node.getAttribute(name) !== value) node.setAttribute(name, value);
}
function releaseControl(host: HTMLElement, control: HTMLElement): void {
    const original = originals.get(control);
    if (original?.owner !== host) return;
    const state = stateFor(host);
    if (state.open === control) close(host, !!state.openPanel?.contains(host.ownerDocument.activeElement));
    const panel = panelFor(control);
    if (panel && stateFor(panel).native?.trigger === control) { stateFor(panel).native?.disconnect(); stateFor(panel).native = undefined; }
    releaseRelationship(control);
    setAttribute(control, 'role', original.role);
    setAttribute(control, 'tabindex', original.tabindex);
    originals.delete(control);
    state.controls.delete(control);
}
function focus(host: HTMLElement, control: HTMLElement | undefined): void {
    if (!control) return;
    stateFor(host).active = control;
    synchronize(host);
    control.focus();
}
function close(host: HTMLElement, restore = false): void {
    const state = stateFor(host), trigger = state.open, panel = state.openPanel;
    if (!trigger || !panel) return;
    state.open = undefined; state.openPanel = undefined; state.relayRevision++;
    close(panel);
    stateFor(panel).native?.request(false, false, restore);
    setAttribute(trigger, 'aria-expanded', 'false');
}
function dismiss(host: HTMLElement): void { close(rootMenu(host) ?? host); }
function boundaryOwner(host: HTMLElement): HTMLElement {
    const visited = new Set<HTMLElement>();
    while (!host.hasAttribute('boundary') && !host.hasAttribute('data-cem-node-ref-boundary') && !visited.has(host)) {
        visited.add(host); const parent = parentMenu(host); if (!parent) break; host = parent;
    }
    return host;
}
function position(host: HTMLElement): void {
    const trigger = stateFor(host).open;
    const panel = trigger && panelFor(trigger);
    if (!trigger || !panel || panel.hidden) return;
    const ready = positionPopup(trigger, panel, column(host), panel, boundaryOwner(panel));
    if (ready) stateFor(panel).geometryReady = true;
    else if (!stateFor(panel).geometryReady || panel.getAttribute('anchor-lost') !== 'freeze') close(host);
    stateFor(panel).settled?.(); stateFor(host).settled?.();
}
function enterWhenReady(panel: HTMLElement): void {
    const state = stateFor(panel);
    const pending = state.pendingEntry;
    if (!pending || !state.native?.open) return;
    const box = container(panel);
    const active = panel.ownerDocument.activeElement;
    if (active !== pending.origin && active !== box && active !== panel.ownerDocument.body) {
        state.pendingEntry = undefined;
        return;
    }
    const entry = items(panel).at(pending.last ? -1 : 0);
    // Keep the requested end until an asynchronously rendered control arrives.
    if (entry || panel.hasAttribute('focus-target') || panel.hasAttribute('data-cem-node-ref-focus-target')) state.pendingEntry = undefined;
    if (entry?.matches('[aria-disabled="true"]') && !panel.hasAttribute('focus-target') && !panel.hasAttribute('data-cem-node-ref-focus-target')) entry.focus();
    else focusSurface(panel, panel, entry ?? box ?? undefined);
}
function bindPanel(host: HTMLElement, trigger: HTMLElement, panel: HTMLElement): CemNativePopupBinding {
    const state = stateFor(host), child = stateFor(panel);
    if (child.native && child.native.trigger !== trigger) { child.native.disconnect(); child.native = undefined; }
    if (!child.native) child.native = new CemNativePopupBinding(panel, trigger, panel, {
        admitted: () => enabled(host) && controls(host).includes(trigger) && panelFor(trigger) === panel && !unavailable(trigger),
        prepare: () => preparePopupGeometry(trigger, panel, panel, boundaryOwner(panel)),
        changed(open) {
            setAttribute(trigger, 'aria-expanded', String(open));
            if (open) {
                state.open = trigger; state.openPanel = panel;
                child.geometryReady = positionPopup(trigger, panel, column(host), panel, boundaryOwner(panel));
                if (!child.geometryReady) child.native?.request(false);
                synchronize(panel);
            } else {
                if (state.openPanel === panel) { state.open = undefined; state.openPanel = undefined; state.relayRevision++; }
                child.pendingEntry = undefined;
                releasePopupGeometry(panel); child.geometryReady = false;
            }
            child.settled?.(); state.settled?.();
        },
        entered() {
            child.pendingEntry = { last: !!child.entryLast, origin: panel.ownerDocument.activeElement };
            enterWhenReady(panel);
            child.entryLast = false;
        },
        closing: () => { state.relayRevision++; },
    });
    reportInteractionReference(panel, child.native.valid ? undefined : 'interaction-control-unsupported', 'native-invoker');
    return child.native;
}
function open(host: HTMLElement, trigger: HTMLElement, last = false, enter = true): void {
    if (!enabled(host) || unavailable(trigger)) return;
    const panel = panelFor(trigger);
    if (!panel) return;
    const binding = bindPanel(host, trigger, panel);
    stateFor(panel).entryLast = last;
    if (enter) binding.request(true, true);
    else binding.previewBranch();
}
function synchronize(host: HTMLElement): void {
    const state = stateFor(host);
    const box = container(host);
    if (state.syncing) return;
    const ownedControls = controls(host);
    for (const control of state.controls) if (!ownedControls.includes(control)) releaseControl(host, control);
    if (!box) return;
    state.syncing = true;
    try {
        const menuMode = enabled(host);
        setAttribute(box, 'role', menuMode ? (popupLevel(host) ? 'menu' : 'menubar') : 'group');
        setAttribute(box, 'aria-orientation', menuMode ? (column(host) ? 'vertical' : 'horizontal') : null);
        for (const control of ownedControls) {
            const prior = originals.get(control);
            if (prior && prior.owner !== host) releaseControl(prior.owner, control);
            if (!originals.has(control)) originals.set(control, { owner: host, role: control.getAttribute('role'), tabindex: control.getAttribute('tabindex'), disabled: control.getAttribute('aria-disabled') });
            state.controls.add(control);
            if (control.matches('a')) setAttribute(control, 'aria-disabled', control.hasAttribute('disabled') || control.parentElement?.matches('[disabled],[aria-disabled="true"]') ? 'true' : originals.get(control)?.disabled ?? null);
        }
        const candidates = items(host);
        setAttribute(box, 'tabindex', menuMode && candidates.length === 0 ? '-1' : null);
        if (!state.active || !candidates.includes(state.active)) state.active = candidates[0];
        if (!menuMode) close(host, !!state.openPanel?.contains(host.ownerDocument.activeElement));
        else if (state.open && (!candidates.includes(state.open) || !panelFor(state.open))) close(host);
        for (const control of ownedControls) {
            const controlMode = menuMode && enabled(host, control);
            const authoredRole = originals.get(control)?.role;
            setAttribute(control, 'role', controlMode ? (authoredRole === 'menuitemradio' || authoredRole === 'menuitemcheckbox' ? authoredRole : 'menuitem') : authoredRole ?? null);
            const disabled = control.matches(':disabled') || !controlMode && unavailable(control);
            setAttribute(control, 'tabindex', controlMode ? (control === state.active && !disabled && !popupLevel(host) ? '0' : '-1') : (disabled ? '-1' : originals.get(control)?.tabindex ?? null));
            const panel = panelFor(control);
            if (!panel) { releaseRelationship(control); continue; }
            if (!controlMode) {
                stateFor(panel).native?.disconnect(); stateFor(panel).native = undefined;
                releaseRelationship(control);
                setAttribute(panel, 'hidden', '');
                synchronize(panel);
                continue;
            }
            if (!relationshipAttributes.has(control)) relationshipAttributes.set(control, new Map(['aria-haspopup', 'aria-controls', 'aria-expanded'].map(name => [name, control.getAttribute(name)])));
            if (!control.id) control.id = `cem-composite-trigger-${++sequence}`;
            if (!panel.id) panel.id = `cem-composite-panel-${++sequence}`;
            setAttribute(control, 'aria-haspopup', 'menu');
            setAttribute(control, 'aria-controls', panel.id);
            const panelBox = container(panel);
            if (panelBox) setAttribute(panelBox, 'aria-labelledby', control.id);
            const binding = bindPanel(host, control, panel);
            binding.sync();
            setAttribute(control, 'aria-expanded', String(binding.open));
            synchronize(panel);
        }
        position(host);
        enterWhenReady(host);
    } finally { state.syncing = false; }
}
function standalone(host: HTMLElement): boolean {
    return !!host.closest('[part~="popup"]');
}
function popupLevel(host: HTMLElement): boolean { return !!parentMenu(host) || standalone(host); }
export function popupMenuEntry(panel: HTMLElement, last: boolean): HTMLElement | undefined {
    const menu = Array.from(menus).find(menu => menu.closest('[part~="popup"]') === panel && !parentMenu(menu) && enabled(menu));
    if (!menu) return;
    return items(menu).at(last ? -1 : 0) ?? container(menu) ?? undefined;
}
function moveBranch(root: HTMLElement, launcher: HTMLElement, step: 'next' | 'previous', preview: boolean): void {
    const bar = items(root);
    const sibling = bar[compositeIndex(bar.length, bar.indexOf(launcher), step)];
    close(root);
    focus(root, sibling);
    if (preview && sibling && !unavailable(sibling)) open(root, sibling, false, false);
}

export function handleMenuKeydown(host: HTMLElement, event: KeyboardEvent): void {
    const target = event.target as HTMLElement;
    const box = container(host);
    if (!box || target.closest(COMPOSITE) !== box || !enabled(host, target)) return;
    const candidates = items(host);
    const current = candidates.indexOf(target);
    if (current < 0 && target !== box) return;
    const key = event.key;
    const root = rootMenu(host);
    if (!root) return;
    if (key === 'Tab') {
        const launcher = stateFor(root).open;
        if (launcher) { dismiss(host); launcher.focus(); }
        return;
    }
    if (event.shiftKey || key === 'Escape' || current < 0) return;
    if (key === 'Enter' || key === ' ') {
        if (event.repeat || unavailable(target)) event.preventDefault();
        else if (key === ' ' && target.matches('a[href]')) { event.preventDefault(); target.click(); }
        return;
    }
    const vertical = column(host);
    const forward = getComputedStyle(box).direction === 'rtl' ? 'ArrowLeft' : 'ArrowRight';
    const backward = forward === 'ArrowRight' ? 'ArrowLeft' : 'ArrowRight';
    const next = vertical ? 'ArrowDown' : forward;
    const previous = vertical ? 'ArrowUp' : backward;
    const toward = vertical ? forward : 'ArrowDown';
    const away = vertical ? backward : 'ArrowUp';
    if (key === next || key === previous) {
        const step = key === next ? 'next' : 'previous';
        if (host === root && stateFor(root).open) moveBranch(root, target, step, true);
        else { close(host); focus(host, candidates[compositeIndex(candidates.length, current, step)]); }
    } else if (key === toward || key === away) {
        const parent = parentMenu(host);
        if (panelFor(target) && (key === toward || !popupLevel(host))) open(host, target, key === away);
        else if (!popupLevel(host)) moveBranch(root, target, key === toward ? 'next' : 'previous', false);
        else if (key === away && parent && (parent !== root || standalone(root))) close(parent, true);
        else if (!standalone(root)) {
            const launcher = stateFor(root).open;
            if (launcher) moveBranch(root, launcher, key === toward ? 'next' : 'previous', true);
        }
    } else if (key === 'Home' || key === 'End') {
        close(host);
        focus(host, key === 'Home' ? candidates[0] : candidates.at(-1));
    } else if (key.length === 1) {
        const state = stateFor(host);
        const now = Date.now();
        const language = target.closest('[lang]')?.getAttribute('lang') || undefined;
        state.buffer = now - state.typedAt > 700 || language !== state.language ? key : state.buffer + key;
        state.language = language;
        state.typedAt = now;
        const query = [...state.buffer].every(letter => letter === state.buffer[0]) ? key : state.buffer;
        const ordered = [...candidates.slice(current + 1), ...candidates.slice(0, current + 1)];
        let collator: Intl.Collator;
        try { collator = new Intl.Collator(language, { usage: 'search', sensitivity: 'base' }); }
        catch { collator = new Intl.Collator(undefined, { usage: 'search', sensitivity: 'base' }); }
        const match = ordered.find(control => {
            const label = (control.getAttribute('aria-label') ?? control.textContent ?? '').trim();
            return collator.compare(label.slice(0, query.length), query) === 0;
        });
        if (!match) return;
        focus(host, match);
    } else return;
    event.preventDefault();
}

export const CEM_COMPOSITE_MENU_CAPABILITY: CemProducedElementBehavior = {
    connected(host, context) {
        const view = host.ownerDocument.defaultView;
        if (!view) return;
        const state = stateFor(host);
        if (state.abort) return;
        state.settled = () => context?.runtime.checkpointInteractionSettled(host);
        const abort = new AbortController();
        state.abort = abort;
        menus.add(host);
        state.releaseRelay = registerCemMenuRelay({ host,
            owns: node => controls(host).some(control => control === node || control.contains(node)),
            parent: () => parentMenu(host) ?? undefined, launcher: () => interactionControl(owner(host) ?? undefined),
            revision: () => state.relayRevision,
            active: () => host.isConnected && enabled(host) && !host.closest('[hidden],[inert]')
                && (!parentMenu(host) || stateFor(parentMenu(host)!).openPanel === host),
            dismiss: () => close(host),
        });
        state.releaseReferences = observeInteractionReferences(host, () => relationshipsChanged(host));
        const options = { signal: abort.signal };
        host.addEventListener('click', event => {
            const control = (event.target as Element).closest<HTMLElement>('button,a');
            if (!control || !controls(host).includes(control)) return;
            if (unavailable(control)) { event.preventDefault(); event.stopImmediatePropagation(); return; }
            resetCemTaskActivation(control);
            if (panelFor(control)) {
                // The native invoker's default action is the only click toggle.
                return;
            } else if (enabled(host)) {
                // Let native link/command activation complete before dismissing.
                state.queuedActivations++;
                queueMicrotask(() => {
                    state.queuedActivations--;
                    if (consumeCemTaskActivation(control) || event.defaultPrevented || !host.isConnected) return;
                    const root = rootMenu(host);
                    const launcher = root && stateFor(root).open;
                    const restore = root && insideChain(root, host.ownerDocument.activeElement);
                    dismiss(host);
                    if (restore && launcher?.isConnected) launcher.focus();
                    host.dispatchEvent(new Event('cem-popup-dismiss', { bubbles: true }));
                });
            }
        }, { ...options, capture: true });
        host.addEventListener('focusin', event => {
            const control = event.target as HTMLElement;
            if (items(host).includes(control)) { state.active = control; synchronize(host); }
        }, options);
        host.addEventListener('focusout', event => {
            // Native focusout can precede activeElement moving to relatedTarget.
            if (insideChain(host, event.relatedTarget as Node | null)) return;
            view.setTimeout(() => {
                const active = host.ownerDocument.activeElement;
                if (!abort.signal.aborted && active && active !== host.ownerDocument.body
                    && active !== host.ownerDocument.documentElement && !insideChain(host, active)) dismiss(host);
            }, 0);
        }, options);
        const reflow = () => { const parent = parentMenu(host); if (parent) position(parent); position(host); };
        state.releaseGeometry = observePopupGeometry(host, reflow);
        view.visualViewport?.addEventListener('resize', reflow, options);
        view.visualViewport?.addEventListener('scroll', reflow, options);
        view.addEventListener('resize', reflow, options);
        view.addEventListener('scroll', reflow, { ...options, capture: true });
        state.observer = new MutationObserver(() => synchronize(host));
        state.observer.observe(host, { childList: true, subtree: true, attributes: true, attributeFilter: ['aria-expanded', 'disabled', 'hidden', 'direction', 'keynav', 'dir', 'href', 'parent-item', 'data-cem-node-ref-parent-item', 'focus-target', 'return-focus', 'anchor', 'boundary', 'anchor-lost', 'data-cem-node-ref-focus-target', 'data-cem-node-ref-return-focus', 'data-cem-node-ref-anchor', 'data-cem-node-ref-boundary'] });
    },
    rendered(host) { relationshipsChanged(host); },
    preserveRenderedAttribute(_host, _current, _desired, attribute) {
        return (_current.matches(COMPOSITE) || controls(_host).includes(_current as HTMLElement)) && ['role', 'tabindex', 'aria-orientation', 'aria-haspopup', 'aria-controls', 'aria-expanded', 'aria-labelledby', 'aria-disabled', 'id'].includes(attribute.name);
    },
    disconnected(host) {
        const state = stateFor(host); state.settled = undefined; state.pendingEntry = undefined;
        close(host);
        for (const control of state.controls) releaseControl(host, control);
        state.releaseRelay?.(); state.releaseRelay = undefined;
        state.abort?.abort();
        state.abort = undefined;
        state.observer?.disconnect();
        state.native?.disconnect(); state.native = undefined;
        state.releaseReferences?.(); state.releaseReferences = undefined;
        state.releaseGeometry?.(); state.releaseGeometry = undefined; releasePopupGeometry(host); state.geometryReady = false; menus.delete(host);
        if (state.relationshipHidden) { host.hidden = state.relationshipHidden.value; state.relationshipHidden = undefined; }
        if (state.parent?.isConnected) synchronize(state.parent);
    },
};

/** Resolve current semantic policy, never retain authored IDs or browser geometry. */
function checkpointMenuPolicies(instance: HTMLElement, parentHost: HTMLElement | undefined, resolve: (node: HTMLElement) => CheckpointEndpoint | undefined): CheckpointMenuPolicies | null | undefined {
    const names = ['anchor', 'boundary', 'focus-target', 'return-focus', 'anchor-lost'];
    const boundaryHost = boundaryOwner(instance);
    const inherited = boundaryHost !== instance && (boundaryHost.hasAttribute('boundary') || boundaryHost.hasAttribute('data-cem-node-ref-boundary'));
    if (!inherited && !names.some(name => instance.hasAttribute(name) || instance.hasAttribute(`data-cem-node-ref-${name}`))) return null;
    const box = container(instance), composite = box && resolve(box), self = resolve(instance);
    if (!composite || !self || composite.instanceId !== self.instanceId || composite.dataRevision !== self.dataRevision || composite.nodeId === 'host'
        || (box as HTMLElement & { cemRenderNodeId?: string }).cemRenderNodeId !== composite.nodeId) return undefined;
    const boundary = inherited ? boundaryHost : instance;
    const boundaryEndpoint = inherited ? resolve(boundaryHost) : null;
    if (boundaryEndpoint === undefined || boundaryEndpoint?.nodeId !== undefined && boundaryEndpoint.nodeId !== 'host') return undefined;
    const role = (source: HTMLElement, name: string, fallback: 'invoker' | 'viewport' | 'auto'): CheckpointMenuRole | undefined => {
        const value = source.getAttribute(name) ?? fallback, typed = source.hasAttribute(`data-cem-node-ref-${name}`);
        if (!typed && (value === fallback || fallback === 'auto' && value === 'none')) return { mode: value as CheckpointMenuRole['mode'], target: null };
        const reference = interactionReference(source, value, name);
        const node = reference.target;
        if (reference.code || !(node instanceof HTMLElement) || !node.isConnected || node.getRootNode() !== instance.getRootNode()) return undefined;
        if (fallback === 'auto' && (!surfaceFocusTargetEligible(node) || name === 'focus-target' && !instance.contains(node)
            || !instance.hidden && !surfaceElementAvailable(node))) return undefined;
        const target = resolve(node);
        return target && (target.nodeId === 'host' || (node as HTMLElement & { cemRenderNodeId?: string }).cemRenderNodeId === target.nodeId)
            ? { mode: 'target', target } : undefined;
    };
    const anchor = role(instance, 'anchor', 'invoker'), boundaryRole = role(boundary, 'boundary', 'viewport');
    const focusTarget = role(instance, 'focus-target', 'auto'), returnFocus = role(instance, 'return-focus', 'auto');
    const anchorLost = instance.getAttribute('anchor-lost') ?? 'close';
    if (!anchor || !boundaryRole || !focusTarget || !returnFocus || !['close', 'freeze'].includes(anchorLost)) return undefined;
    if (parentHost && stateFor(parentHost).openPanel === instance) {
        const trigger = owner(instance);
        if (!trigger || !checkpointPopupGeometry(trigger, instance, column(parentHost), instance, boundaryHost)) return undefined;
    }
    return { compositeNode: composite.nodeId, anchor, boundary: boundaryRole, boundaryOwner: boundaryEndpoint,
        focusTarget, returnFocus, anchorLost: anchorLost as 'close' | 'freeze', geometry: 'recompute' };
}

/** Read-only portable state; listeners, generated IDs and grants are reacquired. */
export function cemCompositeMenuCheckpoint(instance: HTMLElement, resolve: (node: HTMLElement) => CheckpointEndpoint | undefined) {
    const state = states.get(instance);
    if (!instance.isConnected || !state?.abort || state.abort.signal.aborted || state.syncing || state.queuedActivations) return undefined;
    if (enabled(instance)) for (const control of controls(instance)) {
        const panel = panelFor(control), binding = panel && stateFor(panel).native;
        if (panel && (!binding?.valid || binding.trigger !== control)) return undefined;
    }
    const parentHost = parentMenu(instance) ?? undefined, launcherControl = owner(instance) ?? undefined;
    if (state.parent !== parentHost || !!parentHost !== !!launcherControl || !rootMenu(instance)) return undefined;
    if (parentHost && (panelFor(launcherControl!) !== instance || (stateFor(parentHost).openPanel === instance) === !!instance.hidden)) return undefined;
    const policies = checkpointMenuPolicies(instance, parentHost, resolve);
    if (policies === undefined) return undefined;
    const parent = parentHost ? resolve(parentHost) : null, launcher = launcherControl ? resolve(launcherControl) : null;
    const panelHost = state.open && panelFor(state.open);
    if (state.open && unavailable(state.open) || !!state.open !== !!state.openPanel || state.openPanel !== (panelHost ?? undefined)
        || state.openPanel && (!stateFor(state.openPanel).native?.open || state.openPanel.hidden || !stateFor(state.openPanel).geometryReady)) return undefined;
    const panel = state.openPanel ? resolve(state.openPanel) : null;
    if (parent === undefined || launcher === undefined || panel === undefined) return undefined;
    const activeNode = state.active?.getAttribute('data-cem-render-node-id') ?? null;
    if (state.active && (!activeNode || !controls(instance).includes(state.active))) return undefined;
    const contract: CheckpointMenu = { parent, launcher, panel, disposition: 'reacquire', ...(policies ? { policies } : {}) };
    return { kind: 'composite-menu' as const, activeNode, typeahead: state.buffer, typeaheadAt: state.typedAt, open: !!state.open, state: contract };
}

/** Recompute retained submenu geometry; reopening would replay focus/task policy. */
export function resumeCemMenuGeometry(host: HTMLElement): void {
    const state = states.get(host);
    if (!state?.abort || state.abort.signal.aborted) throw new Error('cem.checkpoint.resume.menu');
    if (state.open && state.openPanel && !positionPopup(state.open, state.openPanel, column(host), state.openPanel, boundaryOwner(state.openPanel)))
        throw new Error('cem.checkpoint.resume.geometry');
}
