import type { PopupRect } from './composite-navigation.js';

export function validSurfacePlacement(value: string): boolean {
    return value === 'center' || /^(block-start|block-end|inline-start|inline-end) (start|center|end)$/.test(value);
}
/** Logical placement and fitting for native surfaces; no DOM or focus ownership. */
export function surfacePosition(anchor: PopupRect | undefined, width: number, height: number, bounds: PopupRect,
    placement: string, rtl = false, writingMode = 'horizontal-tb', fallback: readonly string[] = [],
    options: { flip?: boolean; shift?: boolean } = {}): { left: number; top: number; placement: string } {
    const vertical = writingMode.startsWith('vertical') || writingMode.startsWith('sideways');
    const blockStart = vertical ? writingMode.endsWith('-rl') ? 'right' : 'left' : 'top';
    const inlineStart = vertical ? rtl !== (writingMode === 'sideways-lr') ? 'bottom' : 'top' : rtl ? 'right' : 'left';
    const opposite = (side: string) => ({ left: 'right', right: 'left', top: 'bottom', bottom: 'top' })[side as 'left'];
    const coordinates = (choice: string) => {
        if (choice === 'center') return { left: bounds.left + (bounds.right - bounds.left - width) / 2, top: bounds.top + (bounds.bottom - bounds.top - height) / 2 };
        const [side, align] = choice.split(' ');
        const physical = side.startsWith('block') ? side.endsWith('start') ? blockStart : opposite(blockStart)
            : side.endsWith('start') ? inlineStart : opposite(inlineStart);
        const origin = anchor ?? bounds;
        const horizontal = physical === 'left' || physical === 'right';
        const crossStart = side.startsWith('block') ? inlineStart : blockStart;
        const endStart = horizontal ? crossStart === 'bottom' : crossStart === 'right';
        const cross = (start: number, end: number, size: number) => align === 'center' ? (start + end - size) / 2
            : (align === 'end') !== endStart ? end - size : start;
        return horizontal ? { left: physical === 'left' ? origin.left - width - 4 : origin.right + 4, top: cross(origin.top, origin.bottom, height) }
            : { left: cross(origin.left, origin.right, width), top: physical === 'top' ? origin.top - height - 4 : origin.bottom + 4 };
    };
    const fits = (p: { left: number; top: number }) => p.left >= bounds.left + 4 && p.top >= bounds.top + 4 && p.left + width <= bounds.right - 4 && p.top + height <= bounds.bottom - 4;
    const alternatives = fallback.length ? fallback : [placement.replace(/(block|inline)-(start|end)/, (_, axis, side) => `${axis}-${side === 'start' ? 'end' : 'start'}`)];
    let selected = placement, point = coordinates(placement);
    if (options.flip !== false && !fits(point)) for (const choice of alternatives) { const candidate = coordinates(choice); if (fits(candidate)) { selected = choice; point = candidate; break; } }
    if (options.shift === false) return { ...point, placement: selected };
    return { left: Math.max(bounds.left + 4, Math.min(point.left, bounds.right - width - 4)),
        top: Math.max(bounds.top + 4, Math.min(point.top, bounds.bottom - height - 4)), placement: selected };
}
