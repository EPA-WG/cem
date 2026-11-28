import { isDomInstance } from './dom-realm.js';

/** Browser-independent contracts used by declarative composite capabilities. */
export function compositeIndex(count: number, active: number, action: 'next' | 'previous' | 'first' | 'last'): number {
    if (!count) return -1;
    if (action === 'first') return 0;
    if (action === 'last') return count - 1;
    if (active < 0) return action === 'previous' ? count - 1 : 0;
    return (active + (action === 'next' ? 1 : count - 1)) % count;
}

export interface PopupRect { left: number; right: number; top: number; bottom: number }
export function popupBounds(viewport: PopupRect, boundary: PopupRect = viewport): PopupRect | undefined {
    const result = { left: Math.max(viewport.left, boundary.left), top: Math.max(viewport.top, boundary.top), right: Math.min(viewport.right, boundary.right), bottom: Math.min(viewport.bottom, boundary.bottom) };
    return result.right > result.left + 8 && result.bottom > result.top + 8 ? result : undefined;
}
export function popupPosition(anchor: PopupRect, width: number, height: number, viewportWidth: number, viewportHeight: number, column: boolean, rtl: boolean, bounds: PopupRect = { left: 0, top: 0, right: viewportWidth, bottom: viewportHeight }): { left: number; top: number } {
    let left = column ? (rtl ? anchor.left - width : anchor.right) : (rtl ? anchor.right - width : anchor.left);
    let top = column ? anchor.top : anchor.bottom;
    if (column && (left + width > bounds.right - 4 || left < bounds.left + 4)) left = rtl ? anchor.right : anchor.left - width;
    if (!column && top + height > bounds.bottom - 4) top = anchor.top - height;
    left = Math.max(bounds.left + 4, Math.min(left, bounds.right - width - 4));
    top = Math.max(bounds.top + 4, Math.min(top, bounds.bottom - height - 4));
    return { left, top };
}

/** Convert viewport popup coordinates into an absolute containing block. */
export function popupLocalPosition(point: { left: number; top: number }, origin: { left: number; top: number }, scrollLeft: number, scrollTop: number, clientLeft: number, clientTop: number): { left: number; top: number } {
    return { left: point.left - origin.left + scrollLeft - clientLeft, top: point.top - origin.top + scrollTop - clientTop };
}

const ACTION_BUTTON = 'button,input[type="button"],input[type="submit"],input[type="reset"],input[type="image"]';
const GROUP = 'fieldset,form,[part~="composite"]';
const hosts = new WeakMap<HTMLElement, { defaultMarker: string | null; listener: (event: KeyboardEvent) => void }>();

/** A projected submenu can continue scope lookup at its admitted launcher. */
export function keynavEnabled(origin: HTMLElement, parent?: (node: HTMLElement) => HTMLElement | null, fallback = false): boolean {
    const app = origin.closest('cem-application');
    const visited = new Set<HTMLElement>();
    for (let node: HTMLElement | null = origin; node && !visited.has(node);) {
        visited.add(node);
        const marker = node.getAttribute('keynav') ?? hosts.get(node)?.defaultMarker;
        if (marker != null) return marker === '' || marker === 'on';
        if (node === app) break;
        const launcher: HTMLElement | null | undefined = parent?.(node);
        if (launcher && launcher.closest('cem-application') !== app) break;
        node = launcher ?? node.parentElement;
    }
    return fallback;
}

function formActionAvailable(button: HTMLElement, group: HTMLElement): boolean {
    return button.closest(GROUP) === group && button.closest('cem-application') === group.closest('cem-application') &&
        !button.isContentEditable && !button.matches('[popovertarget]') &&
        !button.closest('[inert],[hidden],[aria-disabled="true"]') && !button.matches(':disabled') &&
        button.getClientRects().length > 0 && getComputedStyle(button).visibility === 'visible' && keynavEnabled(button);
}

export function connectKeynav(host: HTMLElement, defaultMarker: string | null, menuKey?: (host: HTMLElement, event: KeyboardEvent) => void): void {
    if (hosts.has(host)) return;
    const listener = (event: KeyboardEvent) => {
        if (event.defaultPrevented || event.isComposing || event.altKey || event.ctrlKey || event.metaKey) return;
        const target = event.target;
        if (!isDomInstance(target, 'HTMLElement') || target.isContentEditable) return;
        const group = target.closest<HTMLElement>(GROUP);
        if (!group || group.closest('cem-application') !== target.closest('cem-application')) return;
        if (group.matches('[part~="composite"]')) {
            if (group.parentElement === host) menuKey?.(host, event);
            return;
        }
        if (event.shiftKey || !['ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight'].includes(event.key) ||
            !target.matches(ACTION_BUTTON) || !formActionAvailable(target, group)) return;
        // Only the nearest runtime host containing this form or fieldset delegates for it.
        let delegate: HTMLElement | null = group;
        while (delegate && !hosts.has(delegate) && delegate.localName !== 'cem-application') delegate = delegate.parentElement;
        if (delegate !== host) return;
        const buttons = Array.from(group.querySelectorAll<HTMLElement>(ACTION_BUTTON)).filter(button => formActionAvailable(button, group));
        const forward = getComputedStyle(group).direction === 'rtl' ? 'ArrowLeft' : 'ArrowRight';
        const delta = event.key === 'ArrowDown' || event.key === forward ? 1 : -1;
        const destination = buttons[buttons.indexOf(target) + delta];
        event.preventDefault();
        destination?.focus();
    };
    hosts.set(host, { defaultMarker, listener });
    host.addEventListener('keydown', listener);
}

export function disconnectKeynav(host: HTMLElement): void {
    const binding = hosts.get(host);
    if (binding) host.removeEventListener('keydown', binding.listener);
    hosts.delete(host);
}
