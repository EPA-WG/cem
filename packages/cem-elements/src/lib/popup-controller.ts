import { popupPosition, popupLocalPosition, popupBounds } from './composite-navigation.js';
import { interactionReference, reportInteractionReference } from './interaction-reference.js';

/** Shared browser popup geometry; declarations retain ownership of panel paint. */
export function positionPopup(trigger: HTMLElement, panel: HTMLElement, column = false, host?: HTMLElement, boundaryHost = host): boolean {
    const view = trigger.ownerDocument.defaultView;
    if (!view || panel.hidden) return false;
    const selected = (source: HTMLElement | undefined, name: string, fallback: string, defaultTarget?: HTMLElement) => {
        const value = source?.getAttribute(name) ?? fallback;
        const target = !source || !source.hasAttribute(`data-cem-node-ref-${name}`) && value === fallback ? defaultTarget : interactionReference(source, value, name).target;
        return target instanceof Element && target.isConnected && target.getClientRects().length &&
            !target.closest('[hidden]:not([hidden="until-found"])') && !['hidden', 'collapse'].includes(getComputedStyle(target).visibility) ? target : undefined;
    };
    const anchor = selected(host, 'anchor', 'invoker', trigger);
    const boundary = selected(boundaryHost, 'boundary', 'viewport');
    const explicitBoundary = !!boundaryHost && (boundaryHost.hasAttribute('data-cem-node-ref-boundary') ||
        boundaryHost.hasAttribute('boundary') && boundaryHost.getAttribute('boundary') !== 'viewport');
    const viewport = view.visualViewport;
    const bounds = popupBounds({ left: viewport?.offsetLeft ?? 0, top: viewport?.offsetTop ?? 0,
        right: (viewport?.offsetLeft ?? 0) + (viewport?.width ?? view.innerWidth), bottom: (viewport?.offsetTop ?? 0) + (viewport?.height ?? view.innerHeight) }, boundary?.getBoundingClientRect());
    if (!anchor || explicitBoundary && !boundary || !bounds) {
        if (host) reportInteractionReference(host, 'interaction-anchor-unavailable', 'geometry');
        return false;
    }
    if (host) {
        reportInteractionReference(host, undefined, 'geometry');
        geometryOwners.set(panel, host);
        watchGeometry(host, [anchor, panel, ...(boundary ? [boundary] : [])]);
    }
    panel.style.maxWidth = `${bounds.right - bounds.left - 8}px`;
    panel.style.maxHeight = `min(var(--_cem-popup-max-height, ${bounds.bottom - bounds.top - 8}px), ${bounds.bottom - bounds.top - 8}px)`;
    const rect = panel.getBoundingClientRect();
    const { left, top } = popupPosition(anchor.getBoundingClientRect(), rect.width, rect.height, view.innerWidth, view.innerHeight, column, getComputedStyle(anchor).direction === 'rtl', bounds);
    const parent = panel.offsetParent;
    const local = getComputedStyle(panel).position === 'absolute' && parent instanceof HTMLElement
        ? popupLocalPosition({ left, top }, parent.getBoundingClientRect(), parent.scrollLeft, parent.scrollTop, parent.clientLeft, parent.clientTop)
        : { left, top };
    panel.style.left = `${local.left}px`;
    panel.style.top = `${local.top}px`;
    return true;
}
const styles = new WeakMap<HTMLElement, Map<string, { value: string; priority: string }>>();
const geometryOwners = new WeakMap<Element, HTMLElement>();
const ownedStyles = ['box-sizing', 'position', 'z-index', 'max-width', 'max-height', 'overflow', 'left', 'top'];
export function releasePopupGeometry(panel: HTMLElement): void {
    const owner = geometryOwners.get(panel);
    if (owner) watchGeometry(owner, []);
    geometryOwners.delete(panel);
    const previous = styles.get(panel);
    if (!previous) return;
    for (const [name, { value, priority }] of previous) {
        if (value) panel.style.setProperty(name, value, priority); else panel.style.removeProperty(name);
    }
    styles.delete(panel);
}
export function showPopup(trigger: HTMLElement, panel: HTMLElement, column = false, host?: HTMLElement, boundaryHost = host): boolean {
    if (!styles.has(panel)) styles.set(panel, new Map(ownedStyles.map(name => [name, { value: panel.style.getPropertyValue(name), priority: panel.style.getPropertyPriority(name) }])));
    if (panel.hidden) panel.hidden = false;
    panel.style.removeProperty('display');
    if (getComputedStyle(panel).display === 'none') panel.style.display = 'block';
    panel.style.boxSizing = 'border-box';
    if (getComputedStyle(panel).position !== 'absolute') panel.style.position = 'fixed';
    if (getComputedStyle(panel).zIndex === 'auto') panel.style.zIndex = '1000';
    panel.style.maxWidth = 'calc(100vw - 8px)';
    panel.style.maxHeight = 'min(var(--_cem-popup-max-height, 100vh), calc(100vh - 8px))';
    panel.style.overflow = 'auto';
    // CSS-anchored panels start at their declared position, even offscreen.
    // Explicit activation and resize can request viewport collision handling.
    if (host?.hasAttribute('anchor') || boundaryHost?.hasAttribute('boundary') || getComputedStyle(panel).position !== 'absolute') return positionPopup(trigger, panel, column, host, boundaryHost);
    return true;
}
export function hidePopup(panel: HTMLElement): void {
    if (!panel.hidden) panel.hidden = true;
    panel.style.removeProperty('display');
    releasePopupGeometry(panel);
}
const geometry = new WeakMap<HTMLElement, { observer: ResizeObserver; mutations: MutationObserver; targets: Set<Element> }>();
function watchGeometry(host: HTMLElement, targets: Element[]): void {
    const state = geometry.get(host);
    if (!state) return;
    const next = new Set(targets);
    for (const target of state.targets) if (!next.has(target)) state.observer.unobserve(target);
    for (const target of next) if (!state.targets.has(target)) state.observer.observe(target);
    if (next.size !== state.targets.size || [...next].some(target => !state.targets.has(target))) {
        state.mutations.disconnect();
        // Panel inline styles are owned by fitting itself; observing them would
        // feed each position update back into the geometry loop.
        for (const target of targets.filter(target => geometryOwners.get(target) !== host)) state.mutations.observe(target, { attributes: true, attributeFilter: ['style', 'class'] });
    }
    state.targets = next;
}
export function observePopupGeometry(host: HTMLElement, update: () => void): () => void {
    const observer = new ResizeObserver(update);
    const mutations = new MutationObserver(update);
    geometry.set(host, { observer, mutations, targets: new Set() });
    return () => { observer.disconnect(); mutations.disconnect(); geometry.delete(host); };
}
export function firstPopupControl(panel: HTMLElement): HTMLElement | undefined {
    const available = (control: HTMLElement) => !control.closest('[hidden],[inert],[disabled],[aria-disabled="true"]') &&
        control.getClientRects().length > 0 && getComputedStyle(control).display !== 'none' && !['hidden', 'collapse'].includes(getComputedStyle(control).visibility);
    return Array.from(panel.querySelectorAll<HTMLElement>('button,a[href],input,select,textarea')).find(available) ??
        Array.from(panel.querySelectorAll<HTMLElement>('[tabindex]')).find(available);
}
