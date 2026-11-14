import { expect, it } from 'vitest';
import { surfacePosition, validSurfacePlacement } from './surface-position.js';
const bounds = { left: 0, right: 300, top: 0, bottom: 200 };
const anchor = { left: 100, right: 120, top: 80, bottom: 100 };
it('fits centered and attached logical placements with direction and vertical writing', () => {
    expect(surfacePosition(undefined, 100, 40, bounds, 'center')).toEqual({ left: 100, top: 80, placement: 'center' });
    expect(surfacePosition(anchor, 50, 20, bounds, 'block-end start').left).toBe(100);
    expect(surfacePosition(anchor, 50, 20, bounds, 'block-end start', true).left).toBe(70);
    expect(surfacePosition(anchor, 50, 20, bounds, 'block-start start', false, 'vertical-rl')).toEqual({ left: 124, top: 80, placement: 'block-start start' });
});
it('flips before shifting and only accepts the adopted complete grammar', () => {
    expect(surfacePosition({ ...anchor, top: 180, bottom: 200 }, 50, 40, bounds, 'block-end start').placement).toBe('block-start start');
    const unfitted = surfacePosition({ ...anchor, top: 180, bottom: 200 }, 50, 40, bounds, 'block-end start', false, 'horizontal-tb', [], { flip: false, shift: false });
    expect(unfitted).toEqual({ left: 100, top: 204, placement: 'block-end start' });
    for (const invalid of ['bottom', 'inline-end', 'block-end left', '', 'center start']) expect(validSurfacePlacement(invalid)).toBe(false);
    expect(validSurfacePlacement('inline-start end')).toBe(true);
});

it('maps logical attachment and ordered fallbacks to physical CSS areas', async () => {
    const { surfaceAnchorOptions } = await import('./surface-position.js');
    expect(surfaceAnchorOptions('block-end start')).toMatchObject({ area: 'bottom span-right', tries: 'top span-right', gapAxis: 'y' });
    expect(surfaceAnchorOptions('block-end start', true)).toMatchObject({ area: 'bottom span-left', tries: 'top span-left' });
    expect(surfaceAnchorOptions('inline-end start', false, 'vertical-rl')).toMatchObject({ area: 'bottom span-left', gapAxis: 'y' });
    expect(surfaceAnchorOptions('block-start end', false, 'vertical-rl')).toMatchObject({ area: 'right span-top', gapAxis: 'x' });
    expect(surfaceAnchorOptions('block-end center', false, 'horizontal-tb', ['block-start center', 'block-end end']))
        .toMatchObject({ area: 'bottom span-all', tries: 'top span-all, bottom span-left' });
    expect(surfaceAnchorOptions('block-end start', false, 'horizontal-tb', [], { flip: false })).toMatchObject({ tries: 'none' });
    expect(surfaceAnchorOptions('block-end start', false, 'horizontal-tb', [], { crossAlign: true }))
        .toMatchObject({ tries: 'top span-right, bottom span-left, top span-left' });
    expect(surfaceAnchorOptions('center')).toBeUndefined();
    expect(surfaceAnchorOptions('bottom')).toBeUndefined();
    expect(surfaceAnchorOptions('block-end start', false, 'horizontal-tb', ['inline-end start'])).toBeUndefined();
});
