import { popupPosition, popupLocalPosition } from './composite-navigation.js';

/** Shared browser popup geometry; declarations retain ownership of panel paint. */
export function positionPopup(trigger: HTMLElement, panel: HTMLElement, column = false): void {
    const view = trigger.ownerDocument.defaultView;
    if (!view || panel.hidden) return;
    const rect = panel.getBoundingClientRect();
    const { left, top } = popupPosition(trigger.getBoundingClientRect(), rect.width, rect.height, view.innerWidth, view.innerHeight, column, getComputedStyle(trigger).direction === 'rtl');
    const parent = panel.offsetParent;
    const local = getComputedStyle(panel).position === 'absolute' && parent instanceof HTMLElement
        ? popupLocalPosition({ left, top }, parent.getBoundingClientRect(), parent.scrollLeft, parent.scrollTop, parent.clientLeft, parent.clientTop)
        : { left, top };
    panel.style.left = `${local.left}px`;
    panel.style.top = `${local.top}px`;
}
export function showPopup(trigger: HTMLElement, panel: HTMLElement, column = false): void {
    if (panel.hidden) panel.hidden = false;
    panel.style.display = 'block';
    panel.style.boxSizing = 'border-box';
    if (getComputedStyle(panel).position !== 'absolute') panel.style.position = 'fixed';
    panel.style.zIndex = '1000';
    panel.style.maxWidth = 'calc(100vw - 8px)';
    panel.style.maxHeight = 'min(var(--_cem-popup-max-height, 100vh), calc(100vh - 8px))';
    panel.style.overflow = 'auto';
    positionPopup(trigger, panel, column);
}
export function hidePopup(panel: HTMLElement): void {
    if (!panel.hidden) panel.hidden = true;
    panel.style.removeProperty('display');
}
export function firstPopupControl(panel: HTMLElement): HTMLElement | undefined {
    const available = (control: HTMLElement) => !control.closest('[hidden],[inert],[disabled],[aria-disabled="true"]') &&
        getComputedStyle(control).display !== 'none' && getComputedStyle(control).visibility !== 'hidden';
    return Array.from(panel.querySelectorAll<HTMLElement>('button,a[href],input,select,textarea')).find(available) ??
        Array.from(panel.querySelectorAll<HTMLElement>('[tabindex]')).find(available);
}
