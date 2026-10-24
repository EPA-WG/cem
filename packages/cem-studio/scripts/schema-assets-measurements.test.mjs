import assert from 'node:assert/strict';
import { test } from 'node:test';
import { distribution, memoryObservation, summarize } from './schema-assets-measurements.mjs';

const status = (live = 30, linear = 60, reserved = 0) => ({ config: {
    softLimitBytes: 80, hardLimitBytes: 100, recoveryHeadroomBytes: 10, maxConcurrentOperations: 4,
}, sample: { allocatorLiveBytes: live, allocatorPeakBytes: 95, linearMemoryBytes: linear }, reservedBytes: reserved,
activeOperations: reserved ? 1 : 0, softPressure: false, hardPressure: false });

test('reports observed samples without ordering or timing threshold assumptions', () => {
    const values = [9, 1, 5];
    assert.deepEqual(distribution(values), { count: 3, min: 1, median: 5, max: 9 });
    assert.deepEqual(values, [9, 1, 5]);
    assert.equal(distribution([1, 2, 7, 8]).median, 4.5);
    assert.equal(distribution([0]).median, 0);
    for (const values of [[], [NaN], [Infinity], [-1]]) assert.throws(() => distribution(values));
});

test('charges max of allocator live and linear capacity plus reservations, separately from peak', () => {
    const result = memoryObservation(status(30, 60, 5));
    assert.equal(result.observedBytes, 60);
    assert.equal(result.workHeadroomBytes, 25);
    assert.equal(result.softHeadroomBytes, 15);
    assert.equal(result.sample.allocatorPeakBytes, 95);
    assert.equal(result.recoveryReservePreserved, true);
    assert.equal(memoryObservation(status(70, 60)).workHeadroomBytes, 20);
});

test('reports exhausted headroom without negative budgets or invented observations', () => {
    const result = memoryObservation(status(30, 110));
    assert.equal(result.workHeadroomBytes, 0);
    assert.equal(result.recoveryReservePreserved, false);
    for (const patch of [{ sample: { allocatorLiveBytes: 30, allocatorPeakBytes: 20, linearMemoryBytes: 60 } },
        { sample: { allocatorLiveBytes: 30, allocatorPeakBytes: 95, linearMemoryBytes: null } }, { reservedBytes: -1 }]) {
        assert.throws(() => memoryObservation({ ...status(), ...patch }));
    }
});

test('keeps phase distributions separate and rejects mixed workload schemas', () => {
    const sample = (setup, work) => ({ timings: { setup, work } });
    assert.deepEqual(summarize([sample(2, 9), sample(4, 3)]), {
        setup: { count: 2, min: 2, median: 3, max: 4 }, work: { count: 2, min: 3, median: 6, max: 9 },
    });
    assert.throws(() => summarize([sample(1, 2), { timings: { other: 1 } }]));
    assert.throws(() => summarize([]));
});
