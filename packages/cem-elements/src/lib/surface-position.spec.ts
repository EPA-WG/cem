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
