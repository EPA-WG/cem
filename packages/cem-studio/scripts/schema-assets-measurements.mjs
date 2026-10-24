import assert from 'node:assert/strict';

// Measurement control DTOs only. Source semantics and memory policy stay native.
export function distribution(values) {
    assert.ok(values.length > 0, 'measurement needs samples');
    assert.ok(values.every(value => Number.isFinite(value) && value >= 0), 'invalid measurement');
    const sorted = [...values].sort((a, b) => a - b), middle = Math.floor(sorted.length / 2);
    return { count: sorted.length, min: sorted[0], median: sorted.length % 2 ? sorted[middle] : (sorted[middle - 1] + sorted[middle]) / 2,
        max: sorted.at(-1) };
}

export function memoryObservation(status) {
    const { config, sample, reservedBytes, activeOperations } = status;
    for (const value of [config.softLimitBytes, config.hardLimitBytes, config.recoveryHeadroomBytes, config.maxConcurrentOperations,
        sample.allocatorLiveBytes, sample.allocatorPeakBytes, sample.linearMemoryBytes, reservedBytes, activeOperations]) {
        assert.ok(Number.isSafeInteger(value) && value >= 0, 'invalid native memory observation');
    }
    assert.ok(config.hardLimitBytes > config.recoveryHeadroomBytes && config.recoveryHeadroomBytes > 0);
    assert.ok(sample.allocatorPeakBytes >= sample.allocatorLiveBytes);
    // Native admission observes max(live requested allocation, linear capacity).
    // The lifetime allocator peak is a separate observation, not additive RSS.
    const observedBytes = Math.max(sample.allocatorLiveBytes, sample.linearMemoryBytes);
    const workLimitBytes = config.hardLimitBytes - config.recoveryHeadroomBytes;
    const chargedBytes = observedBytes + reservedBytes;
    return { ...status, observedBytes, workLimitBytes, workHeadroomBytes: Math.max(0, workLimitBytes - chargedBytes),
        softHeadroomBytes: Math.max(0, config.softLimitBytes - chargedBytes),
        recoveryReservePreserved: chargedBytes <= workLimitBytes };
}

export function summarize(samples) {
    assert.ok(samples.length > 0);
    const keys = Object.keys(samples[0].timings);
    assert.ok(samples.every(sample => JSON.stringify(Object.keys(sample.timings)) === JSON.stringify(keys)), 'unequal workload stages');
    return Object.fromEntries(keys.map(key => [key, distribution(samples.map(sample => sample.timings[key]))]));
}
