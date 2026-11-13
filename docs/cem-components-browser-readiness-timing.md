# Browser-side fixture readiness timing

The benchmark now records `browserReadyMs` separately from the original
`readyMs` assertion wall time. The new value uses the renderer's
`performance.now()` after a complete fixture predicate passes in two consecutive
animation-frame callbacks with the same nodes. It is opt-in through
`--record=browser-readiness`; historical startup and phase archives are preserved.

The [40-observation report](cem-components-wasm-startup-browser-readiness.md) and
[per-run evidence](cem-components-wasm-startup-browser-readiness.json) retain both
metrics, first-use results, original native hashes, worker counts and memory.
For example, data/fallback has a median browser fixture timestamp of 876 ms and
an assertion wall time of 1365 ms; editor/fallback has 789/1250 ms. These are
independently summarized measurements with different origins and criteria,
not interchangeable values or an optimization comparison with older runs.

## Acceptance and publication

- Controls require one visible, enabled Apply button and one editable text input
  holding Sample, with its associated Item label and the same owning form.
- The empty JSON data view requires its Inventory group and complete No data
  notice. A partially rendered root does not qualify.
- The source editor requires its Source label and textbox, multiline semantics,
  an empty editable control and enabled/read-write attributes.
- The selected compiled entry has no control DOM. It samples its original native
  initialization flag, which the fixture sets after its pinned identity check.

Missing, hidden, zero-sized, transparent, inert, busy, disabled or incomplete
controls refuse. A changed predicate or node identity invalidates the candidate
and starts a new two-frame observation. The original role/name/locator assertions
remain unchanged. After those assertions and the existing network fence, final
publication rechecks current predicates and identities and stops frame sampling
before memory snapshots. Failed assertions or cancellation close the recorder.

The original first-use workflows, network/error checks, native hash checks and
worker success checks still run. A result becomes publishable only after all
of them pass. Summaries require both the browser confirmation and the final
workflow-verification marker. Missing or invalid proofs produce unavailable
metrics rather than zero or partial-sample medians. The original Node-clock
assertion wall time is captured before any browser confirmation/fence work.

This is a benchmark-only DOM observation. It wraps no browser/runtime APIs,
performs no native operation and installs no product/component behavior.
Declarations, theme tokens, resource policies and native ownership are unchanged.

## Sampling limits and verification

This metric describes the fixed benchmark predicates, not general application
readiness, accessibility-tree publication, a paint timestamp or settlement of
every native resource. Two animation-frame callbacks do not imply a fixed
elapsed interval. Busy renderers and scheduling affect sampling cadence; an
original assertion can complete before the second qualifying callback.
The record retains the first qualifying sample, confirmation gap, invalidations,
frame count and post-assertion confirmation timestamp.

Across the 40 observations, wall time inside frame predicate checks totals
0.9–17.2 ms per observation (median 8.35 ms); the largest individual check is
0.3–13.5 ms (median 3.5 ms). Confirmation gaps range from 0.4 to 16.7 ms
(median 6.8 ms). These counters include synchronous query/style/layout work;
they do not measure all scheduling effects and must not be subtracted to invent
an uninstrumented time. Memory is sampled after recorder shutdown, and remains
an observed browser-family PSS snapshot rather than a peak or minimum.

All 29 serial collector/readiness tests pass. New coverage exercises all four
fixture predicates, hidden/transparent/inert/disabled/wrong-name/partial refusal,
node replacement and disconnection, failed original assertions, cancellation,
cleanup, proof validation, separate summaries and incomplete-sample refusal.
All 40 fresh-browser observations pass original first use with one WASM
acquisition; all 120 process-family PSS snapshots are complete. Full/selected
native binaries retain their earlier hashes and sizes.

One initial smoke invocation hit the original controls/fallback Apply-button
visibility deadline before detailed failure capture was added. No cause was
retained. The separate four-observation controls run and later complete smoke
and 40-observation run pass with the same deadline; these passes do not explain
that failure. Future failures now retain recorder state, page errors, native
diagnostics and bounded fixture markup while rethrowing the original failure.

```bash
yarn nx run cem_ql:measure:browser-startup -- --record=browser-readiness --runs=5 --write-doc
```

Next measure explicit all-capabilities delivery/offline totals, then clean
npm-consumer loading and memory before choosing fixed budgets. The grammar
notation/Nelumbo, Schematron parity and Studio schema-constraints authoring
workstreams and their contact follow-ups are recorded separately in the roadmap.
