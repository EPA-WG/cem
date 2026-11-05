import { describe, expect, it } from 'vitest';
import { createJourneyCapture } from '../../.storybook/journey-diagnostics.js';

describe('browser journey failure metadata', () => {
    it('keeps the last bounded monotonic phases and counts discarded phases', () => {
        let now = 10;
        const capture = createJourneyCapture('fixture', () => ({ connected: true }), () => now);
        for (let index = 0; index < 40; index++) { now += 2; capture.mark(`step:${index}`); }
        const result = capture.freeze();
        expect(result.records).toHaveLength(32);
        expect(result.dropped).toBe(8);
        expect(result.records[0]).toEqual({ phase: 'step:8', elapsedMs: 18 });
        expect(result.records.at(-1)).toEqual({ phase: 'step:39', elapsedMs: 80 });
        expect(result.elapsedMs).toBe(80);
    });

    it('copies failure state before cleanup and ignores late phases and caller mutations', () => {
        let reads = 0;
        const state = { connected: true, rows: [3, 2] };
        const capture = createJourneyCapture('fixture', () => { reads++; return state; });
        capture.mark('awaiting rows');
        const result = capture.freeze();
        state.connected = false; state.rows.length = 0;
        result.records.length = 0;
        (result.state as typeof state).rows.length = 0;
        capture.mark('teardown');
        expect(capture.freeze().records.map(record => record.phase)).toEqual(['awaiting rows']);
        expect(capture.freeze().state).toEqual({ connected: true, rows: [3, 2] });
        expect(reads).toBe(1);
    });

    it('retains phases when a detached fixture inspection is unavailable', () => {
        const capture = createJourneyCapture('fixture', () => { throw new Error('unavailable state'); });
        capture.mark('native tap');
        expect(capture.freeze().state).toEqual({ unavailable: true });
        expect(capture.freeze().records[0].phase).toBe('native tap');
    });
});
