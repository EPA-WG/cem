import { describe, expect, it } from 'vitest';

import { consumeLocationWriteTrigger } from './location-write-trigger.js';

describe('location write triggers', () => {
    it('preserves continuous writes when no trigger is authored', () => {
        const consumed = new Map<number, string>();
        expect(consumeLocationWriteTrigger(consumed, 0, undefined)).toBe(true);
        expect(consumeLocationWriteTrigger(consumed, 0, undefined)).toBe(true);
    });

    it('consumes a trigger before a live reader can rerender the writer', () => {
        const consumed = new Map<number, string>();
        expect(consumeLocationWriteTrigger(consumed, 0, '1')).toBe(true);
        expect(consumeLocationWriteTrigger(consumed, 0, '1')).toBe(false);
        expect(consumeLocationWriteTrigger(consumed, 0, '2')).toBe(true);
        expect(consumed.size).toBe(1);
    });

    it('keeps writers independent and treats an empty trigger as unarmed', () => {
        const consumed = new Map<number, string>();
        expect(consumeLocationWriteTrigger(consumed, 0, '')).toBe(false);
        expect(consumeLocationWriteTrigger(consumed, 0, '1')).toBe(true);
        expect(consumeLocationWriteTrigger(consumed, 1, '1')).toBe(true);
        expect(consumeLocationWriteTrigger(new Map(), 0, '1')).toBe(true);
    });
});
