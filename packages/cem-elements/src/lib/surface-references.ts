import { interactionReference, reportInteractionReference } from './interaction-reference.js';
import { firstPopupControl } from './popup-controller.js';

/** Live browser roles for already-published native relationships. */
export function surfaceElement(host: HTMLElement, name: string, fallback: string): HTMLElement | undefined {
    const value = host.getAttribute(name) ?? fallback;
    if (!host.hasAttribute(`data-cem-node-ref-${name}`) && value === fallback) return;
    const target = interactionReference(host, value, name).target;
    return target instanceof HTMLElement ? target : undefined;
}
export function surfaceElementAvailable(target?: HTMLElement): target is HTMLElement {
    return !!target?.isConnected && !target.closest('[hidden]:not([hidden="until-found"]),[inert],[disabled],[aria-disabled="true"]') &&
        target.getClientRects().length > 0 && !['hidden', 'collapse'].includes(getComputedStyle(target).visibility);
}
function focusable(target?: HTMLElement): target is HTMLElement {
    return surfaceElementAvailable(target) && target.matches('button,input:not([type="hidden"]),select,textarea,a[href],[tabindex],[contenteditable]:not([contenteditable="false"])');
}
export function focusSurface(host: HTMLElement, surface: HTMLElement, fallback?: HTMLElement): void {
    const value = host.getAttribute('focus-target') ?? 'auto';
    const typed = host.hasAttribute('data-cem-node-ref-focus-target');
    if (!typed && value === 'none') { reportInteractionReference(host, undefined, 'focus-target'); return; }
    let target: HTMLElement | undefined;
    if (typed || value !== 'auto') {
        target = surfaceElement(host, 'focus-target', 'auto');
        const valid = focusable(target) && surface.contains(target);
        reportInteractionReference(host, valid ? undefined : 'interaction-focus-target-invalid', 'focus-target');
        if (!valid) target = undefined;
    } else reportInteractionReference(host, undefined, 'focus-target');
    target ??= [...surface.querySelectorAll<HTMLElement>('[autofocus]')].find(focusable) ?? fallback ?? firstPopupControl(surface);
    if (focusable(target) && surface.contains(target)) target.focus();
}
export function restoreSurfaceFocus(host: HTMLElement, invoker?: HTMLElement): void {
    const value = host.getAttribute('return-focus') ?? 'auto';
    const typed = host.hasAttribute('data-cem-node-ref-return-focus');
    if (!typed && value === 'none') { reportInteractionReference(host, undefined, 'return-focus'); return; }
    let target = invoker;
    if (typed || value !== 'auto') {
        target = surfaceElement(host, 'return-focus', 'auto');
        reportInteractionReference(host, focusable(target) ? undefined : 'interaction-return-focus-invalid', 'return-focus');
        if (!focusable(target)) target = invoker;
    } else reportInteractionReference(host, undefined, 'return-focus');
    if (focusable(target)) target.focus();
}
