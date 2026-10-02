import { describe, expect, it } from 'vitest';
import { compositeIndex, popupPosition, popupLocalPosition } from './composite-navigation.js';

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
});
