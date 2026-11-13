import assert from 'node:assert/strict';
import test from 'node:test';
import { startupPhaseEvidence } from './startup-phase-evidence.mjs';

test('phase publication excludes diagnostic memory and binds original source/timing evidence', () => {
    const report = { phaseCohort: 'code-sharing', summary: [], methods: { memory: 'invalidated diagnostic counters' },
        results: [{ kind: 'controls', mode: 'fallback', run: 1, readyMs: 900, firstUseMs: 10,
            coldMemory: { browserFamily: { pssBytes: 123 } },
            responses: [{ path: '/runtime.js', sha256: 'original-source' }],
            startupEvidence: { visibleAt: 600, assertionAt: 900, overflow: false, events: [{ label: 'wasm.compile', start: 200, end: 220 }],
                resources: [{ path: '/runtime.js', responseEnd: 100 }, { path: '/native.wasm', responseEnd: 150 }] } }] };
    const published = startupPhaseEvidence(report, 'original-raw-record');
    assert.equal(published.rawSha256, 'original-raw-record');
    assert.equal(published.sourceHashes['/runtime.js'], 'original-source');
    assert.equal(published.results[0].startupEvidence.events[0].end, 220);
    assert.deepEqual(published.results[0].startupEvidence.wasmResources, [{ path: '/native.wasm', responseEnd: 150 }]);
    assert.equal(published.results[0].startupEvidence.lastScriptEnd, 100);
    assert(!JSON.stringify(published).includes('pssBytes'));
    assert.throws(() => startupPhaseEvidence({ ...report, summary: [{}] }, 'x'), /diagnostic/);
    report.results[0].startupEvidence.overflow = true;
    assert.throws(() => startupPhaseEvidence(report, 'x'));
});
