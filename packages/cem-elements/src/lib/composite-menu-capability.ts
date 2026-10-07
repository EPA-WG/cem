import { interactionReference, interactionControl, reportInteractionReference, observeInteractionReferences } from './interaction-reference.js';
import { focusSurface, restoreSurfaceFocus } from './surface-references.js';
import type { CemProducedElementBehavior } from './cem-elements.js';
import { compositeIndex } from './composite-navigation.js';
import { showPopup, hidePopup, positionPopup, observePopupGeometry, releasePopupGeometry } from './popup-controller.js';

// Direct composite parts and projected submenu slots are the declaration contract.
const COMPOSITE = '[part~="composite"]';
const originals = new WeakMap<HTMLElement, { role: string | null; tabindex: string | null; disabled: string | null }>();
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
    active?: HTMLElement;
    open?: HTMLElement;
    openPanel?: HTMLElement;
    parent?: HTMLElement;
    relationshipHidden?: { value: HTMLElement['hidden'] };
    releaseReferences?: () => void;
    releaseGeometry?: () => void;
    geometryReady?: boolean;
    buffer: string;
    typedAt: number;
    abort?: AbortController;
    observer?: MutationObserver;
    syncing: boolean;
}
function stateFor(host: HTMLElement): State {
    let state = states.get(host);
    if (!state) { state = { buffer: '', typedAt: 0, syncing: false }; states.set(host, state); }
    return state;
}
function container(host: HTMLElement): HTMLElement | null { return host.querySelector<HTMLElement>(`:scope > ${COMPOSITE}`); }
function owner(panel: HTMLElement): HTMLElement | null {
    const nested = panel.matches('[slot="submenu"]') ? panel.parentElement : null;
    if (!panel.hasAttribute('parent-item')) return nested;
    const selected = interactionReference(panel, panel.getAttribute('parent-item') ?? '', 'parent-item');
    const control = interactionControl(selected.target);
    const parent = control?.closest(COMPOSITE)?.parentElement;
    const code = selected.code ?? (!control || !parent || !menus.has(parent) || !controls(parent).includes(control)
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
function enabled(host: HTMLElement): boolean { return rootMenu(host)?.getAttribute('keyboard') === 'menu'; }
function insideChain(host: HTMLElement, node: Node | null): boolean {
    if (!node) return false;
    const root = rootMenu(host);
    return (root ?? host).contains(node) || !!root && [...menus].some(menu => rootMenu(menu) === root && menu.contains(node));
}
function relationshipsChanged(host: HTMLElement): void {
    const state = stateFor(host);
    const parent = parentMenu(host) ?? undefined;
    if (state.parent && state.parent !== parent && stateFor(state.parent).openPanel === host) close(state.parent, true);
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
function items(host: HTMLElement): HTMLElement[] { return controls(host).filter(control => !unavailable(control)); }
function panelFor(control: HTMLElement): HTMLElement | null {
    const nested = control.parentElement?.querySelector<HTMLElement>(`:scope > [slot="submenu"]`) ?? null;
    const panels = new Set<HTMLElement>(nested && (!nested.hasAttribute('parent-item') || owner(nested) === control && rootMenu(nested)) ? [nested] : []);
    for (const panel of menus) if (panel.hasAttribute('parent-item') && owner(panel) === control && rootMenu(panel)) panels.add(panel);
    if (panels.size > 1) {
        for (const panel of panels) { claimPanel(panel); reportInteractionReference(panel, 'interaction-reference-conflict'); hidePopup(panel); }
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
function focus(host: HTMLElement, control: HTMLElement | undefined): void {
    if (!control) return;
    stateFor(host).active = control;
    synchronize(host);
    control.focus();
}
function close(host: HTMLElement, restore = false): void {
    const state = stateFor(host);
    const trigger = state.open;
    state.open = undefined;
    if (!trigger) return;
    const panel = state.openPanel ?? panelFor(trigger);
    state.openPanel = undefined;
    if (panel) {
        close(panel);
        hidePopup(panel);
    }
    setAttribute(trigger, 'aria-expanded', 'false');
    if (restore) restoreSurfaceFocus(panel ?? host, trigger);
}
function dismiss(host: HTMLElement): void { close(rootMenu(host) ?? host); }
function boundaryOwner(host: HTMLElement): HTMLElement {
    const visited = new Set<HTMLElement>();
    while (!host.hasAttribute('boundary') && !visited.has(host)) {
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
}
function open(host: HTMLElement, trigger: HTMLElement, last = false): void {
    if (!enabled(host) || unavailable(trigger)) return;
    const panel = panelFor(trigger);
    if (!panel) return;
    if (stateFor(host).open !== trigger) close(host);
    stateFor(host).open = trigger;
    stateFor(host).openPanel = panel;
    stateFor(panel).geometryReady = false;
    if (!showPopup(trigger, panel, column(host), panel, boundaryOwner(panel))) { close(host); return; }
    stateFor(panel).geometryReady = true;
    setAttribute(trigger, 'aria-expanded', 'true');
    synchronize(panel);
    position(host);
    const candidates = items(panel);
    focusSurface(panel, panel, (last ? candidates.at(-1) : candidates[0]) ?? container(panel) ?? undefined);
}
function synchronize(host: HTMLElement): void {
    const state = stateFor(host);
    const box = container(host);
    if (!box || state.syncing) return;
    state.syncing = true;
    try {
        const menuMode = enabled(host);
        setAttribute(box, 'role', menuMode ? (column(host) || owner(host) ? 'menu' : 'menubar') : 'group');
        setAttribute(box, 'aria-orientation', menuMode ? (column(host) ? 'vertical' : 'horizontal') : null);
        for (const control of controls(host)) {
            if (!originals.has(control)) originals.set(control, { role: control.getAttribute('role'), tabindex: control.getAttribute('tabindex'), disabled: control.getAttribute('aria-disabled') });
            if (control.matches('a')) setAttribute(control, 'aria-disabled', control.hasAttribute('disabled') || control.parentElement?.hasAttribute('disabled') ? 'true' : originals.get(control)?.disabled ?? null);
        }
        const candidates = items(host);
        setAttribute(box, 'tabindex', menuMode && candidates.length === 0 ? '-1' : null);
        if (!state.active || !candidates.includes(state.active)) state.active = candidates[0];
        if (state.open && (!candidates.includes(state.open) || !panelFor(state.open))) close(host);
        if (!menuMode) close(host);
        for (const control of controls(host)) {
            setAttribute(control, 'role', menuMode ? 'menuitem' : originals.get(control)?.role ?? null);
            const disabled = unavailable(control);
            setAttribute(control, 'tabindex', menuMode ? (control === state.active && !disabled ? '0' : '-1') : (disabled ? '-1' : originals.get(control)?.tabindex ?? null));
            const panel = panelFor(control);
            if (!panel) { releaseRelationship(control); continue; }
            if (!relationshipAttributes.has(control)) relationshipAttributes.set(control, new Map(['aria-haspopup', 'aria-controls', 'aria-expanded'].map(name => [name, control.getAttribute(name)])));
            if (!control.id) control.id = `cem-composite-trigger-${++sequence}`;
            if (!panel.id) panel.id = `cem-composite-panel-${++sequence}`;
            setAttribute(control, 'aria-haspopup', 'menu');
            setAttribute(control, 'aria-controls', panel.id);
            setAttribute(control, 'aria-expanded', state.open === control ? 'true' : 'false');
            const panelBox = container(panel);
            if (panelBox) setAttribute(panelBox, 'aria-labelledby', control.id);
            if (state.open !== control) setAttribute(panel, 'hidden', '');
        }
        position(host);
    } finally { state.syncing = false; }
}
function handleKey(host: HTMLElement, event: KeyboardEvent): void {
    if (!enabled(host)) return;
    const target = event.target as HTMLElement;
    if (target.closest(COMPOSITE) !== container(host)) return;
    const candidates = items(host);
    const current = candidates.indexOf(target);
    if (current < 0 && !['Escape', 'Tab'].includes(event.key)) return;
    const rtl = getComputedStyle(host).direction === 'rtl';
    const isColumn = column(host);
    const forward = rtl ? 'ArrowLeft' : 'ArrowRight';
    const backward = rtl ? 'ArrowRight' : 'ArrowLeft';
    const key = event.key;
    const parent = parentMenu(host);
    if (key === 'Tab') { dismiss(host); return; }
    if (key === 'Escape') {
        if (!stateFor(host).open && !parentMenu(host)) return;
        if (stateFor(host).open) close(host, true);
        else { const parent = parentMenu(host); if (parent) close(parent, true); }
    } else if ((isColumn && key === forward) || (!isColumn && (key === 'ArrowDown' || key === 'ArrowUp'))) {
        open(host, target, key === 'ArrowUp');
    } else if (isColumn && key === backward && parent) {
        close(parent, true);
    } else if (key === 'Home' || key === 'End' || key === (isColumn ? 'ArrowDown' : forward) || key === (isColumn ? 'ArrowUp' : backward)) {
        const action = key === 'Home' ? 'first' : key === 'End' ? 'last' : key === (isColumn ? 'ArrowDown' : forward) ? 'next' : 'previous';
        close(host);
        focus(host, candidates[compositeIndex(candidates.length, current, action)]);
    } else if (key.length === 1 && key !== ' ' && !event.altKey && !event.ctrlKey && !event.metaKey) {
        const state = stateFor(host);
        const now = Date.now();
        state.buffer = now - state.typedAt > 700 ? key : state.buffer + key;
        state.typedAt = now;
        const query = [...state.buffer].every(letter => letter === state.buffer[0]) ? key : state.buffer;
        const ordered = [...candidates.slice(current + 1), ...candidates.slice(0, current + 1)];
        const match = ordered.find(control => (control.getAttribute('aria-label') ?? control.textContent ?? '').trim().toLocaleLowerCase().startsWith(query.toLocaleLowerCase()));
        focus(host, match);
    } else return; // Enter and Space retain native button/link activation.
    event.preventDefault();
    event.stopPropagation();
}

export const CEM_COMPOSITE_MENU_CAPABILITY: CemProducedElementBehavior = {
    connected(host) {
        const view = host.ownerDocument.defaultView;
        if (!view) return;
        const state = stateFor(host);
        if (state.abort) return;
        const abort = new AbortController();
        state.abort = abort;
        menus.add(host);
        state.releaseReferences = observeInteractionReferences(host, () => relationshipsChanged(host));
        const options = { signal: abort.signal };
        host.addEventListener('keydown', event => handleKey(host, event), options);
        host.addEventListener('click', event => {
            const control = (event.target as Element).closest<HTMLElement>('button,a');
            if (!control || !controls(host).includes(control)) return;
            if (unavailable(control)) { event.preventDefault(); event.stopImmediatePropagation(); return; }
            if (panelFor(control)) {
                event.preventDefault();
                if (state.open === control) close(host, true); else open(host, control);
            } else if (enabled(host)) {
                // Let native link/command activation complete before dismissing.
                queueMicrotask(() => { dismiss(host); host.dispatchEvent(new Event('cem-popup-dismiss', { bubbles: true })); });
            }
        }, { ...options, capture: true });
        host.addEventListener('focusin', event => {
            const control = event.target as HTMLElement;
            if (items(host).includes(control)) { state.active = control; synchronize(host); }
        }, options);
        host.addEventListener('focusout', () => queueMicrotask(() => {
            if (!insideChain(host, host.ownerDocument.activeElement)) dismiss(host);
        }), options);
        host.ownerDocument.addEventListener('pointerdown', event => {
            if (!insideChain(host, event.target as Node)) dismiss(host);
        }, { ...options, capture: true });
        host.ownerDocument.addEventListener('click', event => {
            if (!insideChain(host, event.target as Node)) dismiss(host);
        }, { ...options, capture: true });
        const reflow = () => { const parent = parentMenu(host); if (parent) position(parent); position(host); };
        state.releaseGeometry = observePopupGeometry(host, reflow);
        view.visualViewport?.addEventListener('resize', reflow, options);
        view.visualViewport?.addEventListener('scroll', reflow, options);
        view.addEventListener('resize', reflow, options);
        view.addEventListener('scroll', reflow, { ...options, capture: true });
        state.observer = new MutationObserver(() => synchronize(host));
        state.observer.observe(host, { childList: true, subtree: true, attributes: true, attributeFilter: ['disabled', 'hidden', 'direction', 'keyboard', 'dir', 'href', 'parent-item', 'data-cem-node-ref-parent-item', 'focus-target', 'return-focus', 'anchor', 'boundary', 'anchor-lost', 'data-cem-node-ref-focus-target', 'data-cem-node-ref-return-focus', 'data-cem-node-ref-anchor', 'data-cem-node-ref-boundary'] });
    },
    rendered(host) { relationshipsChanged(host); },
    preserveRenderedAttribute(_host, _current, _desired, attribute) {
        return (_current.matches(COMPOSITE) || controls(_host).includes(_current as HTMLElement)) && ['role', 'tabindex', 'aria-orientation', 'aria-haspopup', 'aria-controls', 'aria-expanded', 'aria-labelledby', 'aria-disabled', 'id'].includes(attribute.name);
    },
    disconnected(host) {
        close(host);
        controls(host).forEach(releaseRelationship);
        const state = stateFor(host);
        state.abort?.abort();
        state.abort = undefined;
        state.observer?.disconnect();
        state.releaseReferences?.(); state.releaseReferences = undefined;
        state.releaseGeometry?.(); state.releaseGeometry = undefined; releasePopupGeometry(host); state.geometryReady = false; menus.delete(host);
        if (state.relationshipHidden) { host.hidden = state.relationshipHidden.value; state.relationshipHidden = undefined; }
        if (state.parent?.isConnected) synchronize(state.parent);
    },
};
