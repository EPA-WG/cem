import { popupPosition, popupLocalPosition, popupBounds } from './composite-navigation.js';
import { interactionReference, reportInteractionReference } from './interaction-reference.js';
import { snapshotGeometryRect, type CemInvocationGeometry } from './surface-invocation.js';
import { surfaceAnchorOptions, surfacePosition, validSurfacePlacement } from './surface-position.js';
import { checkpointSurfaceGeometryCurrent, claimSurfaceGeometry, ownsSurfaceGeometry, releaseSurfaceGeometryClaim, releaseSurfaceGeometry,
    bindCssSurfaceAnchor, clearCssSurfaceAnchor, rememberCssSurfaceAnchor, freezeCssSurfaceAnchor, writeGeometryStyle, restoreGeometryStyle, writeGeometryPlacement, watchSurfaceGeometry, observePopupGeometry, queueSurfaceGeometry } from './surface-geometry.js';
export { observePopupGeometry } from './surface-geometry.js';

function popupGeometryInputs(trigger: HTMLElement, panel: HTMLElement, host?: HTMLElement, boundaryHost = host) {
    const view = trigger.ownerDocument.defaultView;
    if (!view || panel.hidden) return undefined;
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
    return anchor && (!explicitBoundary || boundary) && bounds ? { view, anchor, boundary, bounds } : undefined;
}
const popupGeometrySnapshots = new WeakMap<HTMLElement, { host: HTMLElement; boundaryHost?: HTMLElement; trigger: HTMLElement; column: boolean; anchor: Element; boundary?: Element; signature: string }>();
function popupGeometrySignature(trigger: HTMLElement, panel: HTMLElement, inputs: NonNullable<ReturnType<typeof popupGeometryInputs>>): string {
    const rect = (element?: Element | null) => {
        if (!element) return null;
        const { left, top, right, bottom, width, height } = element.getBoundingClientRect();
        return [left, top, right, bottom, width, height];
    };
    const parent = panel.offsetParent, style = getComputedStyle(panel), anchorStyle = getComputedStyle(inputs.anchor);
    const viewport = inputs.view.visualViewport;
    return JSON.stringify([rect(trigger), rect(inputs.anchor), rect(inputs.boundary), rect(panel), rect(parent), inputs.bounds,
        inputs.view.innerWidth, inputs.view.innerHeight, viewport && [viewport.offsetLeft, viewport.offsetTop, viewport.width, viewport.height, viewport.scale],
        parent instanceof HTMLElement && [parent.scrollLeft, parent.scrollTop, parent.clientLeft, parent.clientTop],
        style.position, style.direction, style.writingMode, anchorStyle.direction, anchorStyle.writingMode]);
}
/** Fresh-layout evidence only. No style writes, focus moves, observer drains or serialized pixels. */
export function checkpointPopupGeometry(trigger: HTMLElement, panel: HTMLElement, column: boolean, host: HTMLElement, boundaryHost = host): boolean {
    const saved = popupGeometrySnapshots.get(panel), inputs = popupGeometryInputs(trigger, panel, host, boundaryHost);
    return !!saved && !!inputs && checkpointSurfaceGeometryCurrent(host, panel) && saved.host === host && saved.boundaryHost === boundaryHost
        && saved.trigger === trigger && saved.column === column && saved.anchor === inputs.anchor && saved.boundary === inputs.boundary
        && saved.signature === popupGeometrySignature(trigger, panel, inputs);
}
/** Declarations opt into this bridge; unsupported profiles retain pixel fitting. */
function cssAnchorPolicy(panel: HTMLElement): boolean {
    return getComputedStyle(panel).getPropertyValue('--_cem-surface-anchor-policy').trim() === 'area'
        && CSS.supports('anchor-name', '--cem-probe') && CSS.supports('position-area', 'bottom span-right')
        && CSS.supports('position-try-fallbacks', 'top span-right');
}
function tryCssAnchor(host: HTMLElement | undefined, panel: HTMLElement, anchor: Element | undefined, bounds: { left: number; top: number; right: number; bottom: number },
    placement: string, fallback: readonly string[], options: { flip: boolean; shift: boolean }, gap: number, boundary?: Element): string | undefined {
    const viewport = panel.ownerDocument.defaultView?.visualViewport;
    const style = getComputedStyle(anchor ?? panel), rtl = style.direction === 'rtl';
    const policy = surfaceAnchorOptions(placement, rtl, style.writingMode, fallback, { flip: options.flip, crossAlign: options.shift });
    const eligible = cssAnchorPolicy(panel) && anchor instanceof HTMLElement && !anchor.contains(panel) && !panel.contains(anchor)
        && anchor.getRootNode() === panel.getRootNode() && !boundary && policy
        && (!viewport || viewport.scale === 1 && viewport.offsetLeft === 0 && viewport.offsetTop === 0)
        && (panel.matches(':popover-open,:modal') || !!(anchor.compareDocumentPosition(panel) & Node.DOCUMENT_POSITION_FOLLOWING));
    if (!eligible) { clearCssSurfaceAnchor(panel); return; }
    const name = bindCssSurfaceAnchor(panel, anchor);
    writeGeometryStyle(panel, 'position-anchor', name);
    writeGeometryStyle(panel, '--_cem-surface-position-mode', 'fixed');
    writeGeometryStyle(panel, '--_cem-surface-area', policy.area);
    writeGeometryStyle(panel, '--_cem-surface-tries', policy.tries);
    writeGeometryStyle(panel, 'inset', 'auto');
    writeGeometryStyle(panel, 'margin', policy.gapAxis === 'y' ? `${gap}px 0px` : `0px ${gap}px`);
    const box = panel.getBoundingClientRect(), origin = anchor.getBoundingClientRect();
    const visible = box.left >= bounds.left + 3.5 && box.top >= bounds.top + 3.5
        && box.right <= bounds.right - 3.5 && box.bottom <= bounds.bottom - 3.5;
    for (const choice of policy.choices) {
        const expected = surfacePosition(origin, box.width, box.height, bounds, choice, rtl, style.writingMode, [], { flip: false, shift: false, gap });
        if ((!options.shift || visible) && Math.abs(box.left - expected.left) < 0.75 && Math.abs(box.top - expected.top) < 0.75) {
            rememberCssSurfaceAnchor(panel, { left: box.left, top: box.top, placement: choice });
            if (host) writeGeometryPlacement(panel, choice);
            return choice;
        }
    }
    // No CSS alternative can satisfy this boundary (or the anchor is outside
    // its CSS scope). Release the bridge before the compatibility solver shifts.
    clearCssSurfaceAnchor(panel);
    return;
}

/** Shared browser popup geometry; declarations retain ownership of panel paint. */
export function positionPopup(trigger: HTMLElement, panel: HTMLElement, column = false, host?: HTMLElement, boundaryHost = host): boolean {
    if (!ownsSurfaceGeometry(panel)) return false;
    const inputs = popupGeometryInputs(trigger, panel, host, boundaryHost);
    if (!inputs) {
        if (host?.getAttribute('anchor-lost') === 'freeze') freezeCssSurfaceAnchor(panel);
        if (host) reportInteractionReference(host, 'interaction-anchor-unavailable', 'geometry');
        return false;
    }
    const { view, anchor, boundary, bounds } = inputs;
    if (host) {
        reportInteractionReference(host, undefined, 'geometry');
        watchSurfaceGeometry(host, panel, [anchor, panel, ...(boundary ? [boundary] : [])]);
    }
    writeGeometryStyle(panel, 'max-width', `${bounds.right - bounds.left - 8}px`);
    writeGeometryStyle(panel, 'max-height', `min(var(--_cem-popup-max-height, ${bounds.bottom - bounds.top - 8}px), ${bounds.bottom - bounds.top - 8}px)`);
    const placement = column ? 'inline-end start' : 'block-end start';
    if (tryCssAnchor(host, panel, anchor, bounds, placement, [], { flip: true, shift: true }, 0, boundary)) {
        if (host) popupGeometrySnapshots.set(panel, { host, boundaryHost, trigger, column, anchor, boundary, signature: popupGeometrySignature(trigger, panel, inputs) });
        return true;
    }
    const rect = panel.getBoundingClientRect();
    const anchorStyle = getComputedStyle(anchor);
    const { left, top } = /^(vertical|sideways)/.test(anchorStyle.writingMode)
        ? surfacePosition(anchor.getBoundingClientRect(), rect.width, rect.height, bounds, placement, anchorStyle.direction === 'rtl', anchorStyle.writingMode, [], { gap: 0 })
        : popupPosition(anchor.getBoundingClientRect(), rect.width, rect.height, view.innerWidth, view.innerHeight, column, anchorStyle.direction === 'rtl', bounds);
    const parent = panel.offsetParent;
    const local = getComputedStyle(panel).position === 'absolute' && parent instanceof HTMLElement
        ? popupLocalPosition({ left, top }, parent.getBoundingClientRect(), parent.scrollLeft, parent.scrollTop, parent.clientLeft, parent.clientTop)
        : { left, top };
    writeGeometryStyle(panel, 'left', `${local.left}px`);
    writeGeometryStyle(panel, 'top', `${local.top}px`);
    if (host) popupGeometrySnapshots.set(panel, { host, boundaryHost, trigger, column, anchor, boundary, signature: popupGeometrySignature(trigger, panel, inputs) });
    return true;
}

/** Validate independent anchor/boundary roles before native opening changes focus. */
export function nativeSurfaceGeometry(host: HTMLElement, panel: HTMLElement, trigger: HTMLElement | undefined,
    geometry: CemInvocationGeometry | undefined, placement: string) {
    const view = panel.ownerDocument.defaultView;
    const selected = (name: string, fallback: string, defaultTarget?: HTMLElement) => {
        const value = host.getAttribute(name) ?? fallback;
        const target = !host.hasAttribute(`data-cem-node-ref-${name}`) && value === fallback ? defaultTarget : interactionReference(host, value, name).target;
        return target instanceof HTMLElement && target.isConnected && target.getClientRects().length
            && !target.closest('[hidden]:not([hidden="until-found"])') && !['hidden', 'collapse'].includes(getComputedStyle(target).visibility) ? target : undefined;
    };
    const value = host.getAttribute('anchor') ?? 'invoker', typed = host.hasAttribute('data-cem-node-ref-anchor');
    const centered = placement === 'center' && !host.hasAttribute('anchor') && !typed;
    const captured = !typed && (value === 'pointer' || value === 'selection');
    const anchor = centered || captured ? undefined : selected('anchor', 'invoker', trigger);
    const rect = centered ? undefined : captured ? snapshotGeometryRect(geometry?.[value as 'pointer' | 'selection'], value === 'pointer') : anchor?.getBoundingClientRect();
    const boundary = selected('boundary', 'viewport'), explicitBoundary = host.hasAttribute('data-cem-node-ref-boundary') || host.hasAttribute('boundary') && host.getAttribute('boundary') !== 'viewport';
    const viewport = view?.visualViewport;
    const bounds = view && popupBounds({ left: viewport?.offsetLeft ?? 0, top: viewport?.offsetTop ?? 0,
        right: (viewport?.offsetLeft ?? 0) + (viewport?.width ?? view.innerWidth), bottom: (viewport?.offsetTop ?? 0) + (viewport?.height ?? view.innerHeight) }, boundary?.getBoundingClientRect());
    const overflow = (host.getAttribute('overflow') ?? 'flip shift resize').trim().split(/\s+/);
    if (!validSurfacePlacement(placement) || host.getAttribute('fallback')?.split(',').some(value => !validSurfacePlacement(value.trim()))
        || overflow.some(token => !['flip', 'shift', 'resize', 'hide'].includes(token)) || overflow.includes('hide') && overflow.length !== 1) {
        reportInteractionReference(host, 'interaction-profile-conflict', 'geometry'); return;
    }
    if (!view || !bounds || !centered && !rect || explicitBoundary && !boundary) {
        reportInteractionReference(host, 'interaction-anchor-unavailable', 'geometry'); return;
    }
    reportInteractionReference(host, undefined, 'geometry');
    return { anchor, rect, boundary, bounds, overflow };
}

/** Fit an already-open native owner without changing its visibility or paint. */
export function fitNativeSurface(host: HTMLElement, panel: HTMLElement, trigger: HTMLElement | undefined,
    geometry: CemInvocationGeometry | undefined, placement: string): boolean {
    return fitSurface(host, panel, trigger, geometry, placement);
}
function fitSurface(host: HTMLElement, panel: HTMLElement, trigger: HTMLElement | undefined,
    geometry: CemInvocationGeometry | undefined, placement: string, token?: object, matchAnchorInlineSize = false): boolean {
    if (!ownsSurfaceGeometry(panel, token)) return false;
    const prepared = nativeSurfaceGeometry(host, panel, trigger, geometry, placement);
    if (!prepared) { if (host.getAttribute('anchor-lost') === 'freeze') freezeCssSurfaceAnchor(panel); return false; }
    const { bounds, rect, anchor, boundary, overflow } = prepared;
    watchSurfaceGeometry(host, panel, [panel, ...(anchor ? [anchor] : []), ...(boundary ? [boundary] : [])]);
    writeGeometryStyle(panel, 'box-sizing', 'border-box');
    writeGeometryStyle(panel, 'position', panel.matches(':popover-open,:modal') ? 'fixed' : 'absolute');
    writeGeometryStyle(panel, 'margin', '0'); writeGeometryStyle(panel, 'right', 'auto'); writeGeometryStyle(panel, 'bottom', 'auto');
    if (overflow.includes('resize')) {
        writeGeometryStyle(panel, 'max-width', `${bounds.right - bounds.left - 8}px`);
        writeGeometryStyle(panel, 'max-height', `${bounds.bottom - bounds.top - 8}px`); writeGeometryStyle(panel, 'overflow', 'auto');
    } else {
        for (const name of ['max-width', 'max-height', 'overflow']) restoreGeometryStyle(panel, name);
    }
    const style = getComputedStyle(anchor ?? host);
    if (matchAnchorInlineSize && rect) {
        const vertical = style.writingMode.startsWith('vertical') || style.writingMode.startsWith('sideways');
        restoreGeometryStyle(panel, vertical ? 'min-width' : 'min-height');
        writeGeometryStyle(panel, vertical ? 'min-height' : 'min-width', `${Math.max(0, Math.min(
            vertical ? rect.bottom - rect.top : rect.right - rect.left,
            vertical ? bounds.bottom - bounds.top - 8 : bounds.right - bounds.left - 8))}px`);
    } else { restoreGeometryStyle(panel, 'min-width'); restoreGeometryStyle(panel, 'min-height'); }
    if (tryCssAnchor(host, panel, anchor, bounds, placement, host.getAttribute('fallback')?.split(',').map(value => value.trim()) ?? [],
        { flip: overflow.includes('flip'), shift: overflow.includes('shift') || overflow.includes('hide') }, 4, boundary)) return true;
    writeGeometryStyle(panel, 'margin', '0'); writeGeometryStyle(panel, 'right', 'auto'); writeGeometryStyle(panel, 'bottom', 'auto');
    const size = panel.getBoundingClientRect();
    const point = surfacePosition(rect, size.width, size.height, bounds, placement, style.direction === 'rtl', style.writingMode,
        host.getAttribute('fallback')?.split(',').map(value => value.trim()), { flip: overflow.includes('flip'), shift: overflow.includes('shift') });
    if (overflow.includes('hide') && (point.left < bounds.left || point.top < bounds.top || point.left + size.width > bounds.right || point.top + size.height > bounds.bottom)) return false;
    const parent = panel.offsetParent;
    const local = panel.style.position === 'absolute' && parent instanceof HTMLElement
        ? popupLocalPosition(point, parent.getBoundingClientRect(), parent.scrollLeft, parent.scrollTop, parent.clientLeft, parent.clientTop) : point;
    writeGeometryStyle(panel, 'left', `${local.left}px`); writeGeometryStyle(panel, 'top', `${local.top}px`);
    writeGeometryPlacement(panel, point.placement);
    return true;
}
export function releasePopupGeometry(panel: HTMLElement): void {
    popupGeometrySnapshots.delete(panel);
    releaseSurfaceGeometry(panel);
}
export function showPopup(trigger: HTMLElement, panel: HTMLElement, column = false, host?: HTMLElement, boundaryHost = host): boolean {
    if (!ownsSurfaceGeometry(panel)) return false;
    if (panel.hidden) panel.hidden = false;
    panel.style.removeProperty('display');
    if (getComputedStyle(panel).display === 'none') panel.style.display = 'block';
    writeGeometryStyle(panel, 'box-sizing', 'border-box');
    if (getComputedStyle(panel).position !== 'absolute') writeGeometryStyle(panel, 'position', 'fixed');
    if (getComputedStyle(panel).zIndex === 'auto') writeGeometryStyle(panel, 'z-index', '1000');
    writeGeometryStyle(panel, 'max-width', 'calc(100vw - 8px)');
    writeGeometryStyle(panel, 'max-height', 'min(var(--_cem-popup-max-height, 100vh), calc(100vh - 8px))');
    writeGeometryStyle(panel, 'overflow', 'auto');
    // Initially visible offscreen examples keep their natural containing-block
    // position. Explicit activation still requests fitting, as before.
    const natural = getComputedStyle(panel).position === 'absolute' && !host?.hasAttribute('anchor') && !boundaryHost?.hasAttribute('boundary');
    const anchorBox = trigger.getBoundingClientRect(), view = trigger.ownerDocument.defaultView;
    if (natural && view && (anchorBox.bottom <= 0 || anchorBox.top >= view.innerHeight || anchorBox.right <= 0 || anchorBox.left >= view.innerWidth)) return true;
    if (host?.hasAttribute('anchor') || boundaryHost?.hasAttribute('boundary') || cssAnchorPolicy(panel) || getComputedStyle(panel).position !== 'absolute') return positionPopup(trigger, panel, column, host, boundaryHost);
    return true;
}
export function hidePopup(panel: HTMLElement): void {
    if (!ownsSurfaceGeometry(panel)) return;
    if (!panel.hidden) panel.hidden = true;
    panel.style.removeProperty('display');
    releasePopupGeometry(panel);
}
export interface CemSurfaceGeometryLease {
    readonly valid: boolean;
    fit(trigger: HTMLElement | undefined, geometry: CemInvocationGeometry | undefined, placement: string, matchAnchorInlineSize?: boolean): boolean;
    /** End this fit's claims and queued work while keeping the registration. */
    reset(): void;
    release(): void;
}
/** Each actual surface has one geometry owner; a host may own several surfaces. */
export function createCemSurfaceGeometryLease(host: HTMLElement, panel: HTMLElement, update: () => void): CemSurfaceGeometryLease {
    const token = claimSurfaceGeometry(panel), abort = new AbortController();
    let released = false;
    const valid = () => !released && ownsSurfaceGeometry(panel, token);
    let stop: () => void;
    try { stop = observePopupGeometry(host, () => { if (valid()) update(); }, panel); }
    catch (error) { releaseSurfaceGeometryClaim(panel, token); throw error; }
    const view = host.ownerDocument.defaultView, queue = () => { if (valid()) queueSurfaceGeometry(host, panel); }, options = { signal: abort.signal };
    view?.addEventListener('resize', queue, options); view?.addEventListener('scroll', queue, { ...options, capture: true });
    view?.visualViewport?.addEventListener('resize', queue, options); view?.visualViewport?.addEventListener('scroll', queue, options);
    return {
        get valid() { return valid(); },
        fit(trigger, geometry, placement, matchAnchorInlineSize = false) { return valid() && host.isConnected && panel.isConnected && fitSurface(host, panel, trigger, geometry, placement, token, matchAnchorInlineSize); },
        reset() { if (valid()) releaseSurfaceGeometry(panel, token); },
        release() {
            if (!valid()) return;
            releaseSurfaceGeometry(panel, token); stop(); abort.abort(); released = true; releaseSurfaceGeometryClaim(panel, token);
        },
    };
}
export function firstPopupControl(panel: HTMLElement): HTMLElement | undefined {
    const available = (control: HTMLElement) => !control.closest('[hidden],[inert],[disabled],[aria-disabled="true"]') &&
        control.getClientRects().length > 0 && getComputedStyle(control).display !== 'none' && !['hidden', 'collapse'].includes(getComputedStyle(control).visibility);
    return Array.from(panel.querySelectorAll<HTMLElement>('button,a[href],input,select,textarea')).find(available) ??
        Array.from(panel.querySelectorAll<HTMLElement>('[tabindex]')).find(available);
}
