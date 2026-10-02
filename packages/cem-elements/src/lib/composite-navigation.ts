/** Browser-independent contracts used by declarative composite capabilities. */
export function compositeIndex(count: number, active: number, action: 'next' | 'previous' | 'first' | 'last'): number {
    if (!count) return -1;
    if (action === 'first') return 0;
    if (action === 'last') return count - 1;
    if (active < 0) return action === 'previous' ? count - 1 : 0;
    return (active + (action === 'next' ? 1 : count - 1)) % count;
}

export function popupPosition(anchor: { left: number; right: number; top: number; bottom: number }, width: number, height: number, viewportWidth: number, viewportHeight: number, column: boolean, rtl: boolean): { left: number; top: number } {
    let left = column ? (rtl ? anchor.left - width : anchor.right) : (rtl ? anchor.right - width : anchor.left);
    let top = column ? anchor.top : anchor.bottom;
    if (column && (left + width > viewportWidth - 4 || left < 4)) left = rtl ? anchor.right : anchor.left - width;
    if (!column && top + height > viewportHeight - 4) top = anchor.top - height;
    left = Math.max(4, Math.min(left, viewportWidth - width - 4));
    top = Math.max(4, Math.min(top, viewportHeight - height - 4));
    return { left, top };
}

/** Convert viewport popup coordinates into an absolute containing block. */
export function popupLocalPosition(point: { left: number; top: number }, origin: { left: number; top: number }, scrollLeft: number, scrollTop: number, clientLeft: number, clientTop: number): { left: number; top: number } {
    return { left: point.left - origin.left + scrollLeft - clientLeft, top: point.top - origin.top + scrollTop - clientTop };
}
