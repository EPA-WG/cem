import { describe, expect, it } from 'vitest';
import { compositeIndex, popupPosition, popupLocalPosition, popupBounds } from './composite-navigation.js';

describe('composite navigation contract', () => {
    it('wraps, handles missing active items and empty collections', () => {
        expect(compositeIndex(3, 2, 'next')).toBe(0);
        expect(compositeIndex(3, 0, 'previous')).toBe(2);
        expect(compositeIndex(3, -1, 'next')).toBe(0);
        expect(compositeIndex(0, -1, 'last')).toBe(-1);
        expect(compositeIndex(3, 1, 'first')).toBe(0);
        expect(compositeIndex(3, 1, 'last')).toBe(2);
    });
    it('converts viewport coordinates to a scrolled, bordered containing block', () => {
        expect(popupLocalPosition({ left: 150, top: 240 }, { left: 100, top: 180 }, 20, 30, 2, 3)).toEqual({ left: 68, top: 87 });
    });
    it('flips row and column popups and clamps oversized panels', () => {
        const anchor = { left: 180, right: 200, top: 170, bottom: 190 };
        expect(popupPosition(anchor, 80, 90, 220, 220, false, false)).toEqual({ left: 136, top: 80 });
        expect(popupPosition(anchor, 80, 90, 220, 220, true, false)).toEqual({ left: 100, top: 126 });
        expect(popupPosition(anchor, 500, 500, 220, 220, true, true)).toEqual({ left: 4, top: 4 });
    });
    it('fits against the boundary intersected with the visual viewport', () => {
        const viewport = { left: 20, top: 30, right: 220, bottom: 230 };
        const bounds = popupBounds(viewport, { left: 100, top: 10, right: 300, bottom: 150 });
        expect(bounds).toEqual({ left: 100, top: 30, right: 220, bottom: 150 });
        expect(popupBounds(viewport, { left: 250, top: 0, right: 300, bottom: 300 })).toBeUndefined();
        expect(popupPosition({ left: 180, right: 200, top: 100, bottom: 120 }, 80, 60, 220, 230, false, false, bounds)).toEqual({ left: 136, top: 40 });
        expect(popupPosition({ left: 180, right: 200, top: 100, bottom: 120 }, 500, 500, 220, 230, true, true, bounds)).toEqual({ left: 104, top: 34 });
        expect(popupLocalPosition({ left: 104, top: 34 }, { left: 80, top: 20 }, 10, 5, 2, 1)).toEqual({ left: 32, top: 18 });
    });
});
