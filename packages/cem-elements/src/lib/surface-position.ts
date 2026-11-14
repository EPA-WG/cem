import type { PopupRect } from './composite-navigation.js';

export function validSurfacePlacement(value: string): boolean {
    return value === 'center' || /^(block-start|block-end|inline-start|inline-end) (start|center|end)$/.test(value);
}
/** Logical placement and fitting for native surfaces; no DOM or focus ownership. */
export function surfacePosition(anchor: PopupRect | undefined, width: number, height: number, bounds: PopupRect,
    placement: string, rtl = false, writingMode = 'horizontal-tb', fallback: readonly string[] = [],
    options: { flip?: boolean; shift?: boolean; gap?: number } = {}): { left: number; top: number; placement: string } {
    const vertical = writingMode.startsWith('vertical') || writingMode.startsWith('sideways');
    const blockStart = vertical ? writingMode.endsWith('-rl') ? 'right' : 'left' : 'top';
    const inlineStart = vertical ? rtl !== (writingMode === 'sideways-lr') ? 'bottom' : 'top' : rtl ? 'right' : 'left';
    const opposite = (side: string) => ({ left: 'right', right: 'left', top: 'bottom', bottom: 'top' })[side as 'left'];
    const gap = options.gap ?? 4;
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
        return horizontal ? { left: physical === 'left' ? origin.left - width - gap : origin.right + gap, top: cross(origin.top, origin.bottom, height) }
            : { left: cross(origin.left, origin.right, width), top: physical === 'top' ? origin.top - height - gap : origin.bottom + gap };
    };
    const fits = (p: { left: number; top: number }) => p.left >= bounds.left + 4 && p.top >= bounds.top + 4 && p.left + width <= bounds.right - 4 && p.top + height <= bounds.bottom - 4;
    const alternatives = fallback.length ? fallback : [placement.replace(/(block|inline)-(start|end)/, (_, axis, side) => `${axis}-${side === 'start' ? 'end' : 'start'}`)];
    let selected = placement, point = coordinates(placement);
    if (options.flip !== false && !fits(point)) for (const choice of alternatives) { const candidate = coordinates(choice); if (fits(candidate)) { selected = choice; point = candidate; break; } }
    if (options.shift === false) return { ...point, placement: selected };
    return { left: Math.max(bounds.left + 4, Math.min(point.left, bounds.right - width - 4)),
        top: Math.max(bounds.top + 4, Math.min(point.top, bounds.bottom - height - 4)), placement: selected };
}

/** Translate the adopted placement grammar, without changing content direction. */
export function surfaceAnchorOptions(placement: string, rtl = false, writingMode = 'horizontal-tb', fallback: readonly string[] = [],
    options: { flip?: boolean; crossAlign?: boolean } = {}): { area: string; tries: string; gapAxis: 'x' | 'y'; choices: string[] } | undefined {
    if (!validSurfacePlacement(placement) || placement === 'center' || fallback.some(value => !validSurfacePlacement(value) || value === 'center')) return;
    const vertical = writingMode.startsWith('vertical') || writingMode.startsWith('sideways');
    const blockStart = vertical ? writingMode.endsWith('-rl') ? 'right' : 'left' : 'top';
    const inlineStart = vertical ? rtl !== (writingMode === 'sideways-lr') ? 'bottom' : 'top' : rtl ? 'right' : 'left';
    const opposite = (side: string) => ({ left: 'right', right: 'left', top: 'bottom', bottom: 'top' })[side as 'left'];
    const oppositePlacement = placement.replace(/(block|inline)-(start|end)/, (_, axis, side) => `${axis}-${side === 'start' ? 'end' : 'start'}`);
    const flippedAlign = placement.replace(/ (start|end)$/, (_, align) => ` ${align === 'start' ? 'end' : 'start'}`);
    const alternatives = options.flip === false ? [] : fallback.length ? [...fallback] : [oppositePlacement,
        ...(options.crossAlign && flippedAlign !== placement ? [flippedAlign, oppositePlacement.replace(/ (start|end)$/, (_, align) => ` ${align === 'start' ? 'end' : 'start'}`)] : [])];
    const choices = [...new Set([placement, ...alternatives])];
    // An area fallback cannot change its margins. Keep cross-axis policies on
    // the compatibility solver rather than silently changing their gap.
    if (choices.some(value => value.split('-')[0] !== placement.split('-')[0])) return;
    const area = (value: string) => {
        const [side, align] = value.split(' ');
        const physical = side.startsWith('block') ? side.endsWith('start') ? blockStart : opposite(blockStart)
            : side.endsWith('start') ? inlineStart : opposite(inlineStart);
        const crossStart = side.startsWith('block') ? inlineStart : blockStart;
        const span = align === 'start' ? opposite(crossStart) : crossStart;
        return `${physical} ${align === 'center' ? 'span-all' : `span-${span}`}`;
    };
    const preferred = area(placement);
    return { area: preferred, tries: choices.slice(1).map(area).join(', ') || 'none', gapAxis: /^(top|bottom) /.test(preferred) ? 'y' : 'x', choices };
}
