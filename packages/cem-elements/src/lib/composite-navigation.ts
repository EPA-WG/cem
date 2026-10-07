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
