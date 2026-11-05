# Linked preparation source provenance — 2026-11-04

- [x] Diagnose the three linked package preparation source-map failures without
  dropping provenance or changing native evidence admission.
- [x] Correct the fixture's map expectation and add regressions for independent
  retained owners, exact original frames/spans and existing origin boundaries.

The shared `CheckCandidate` fixture required the query candidate's source map to
equal the original lexical parser map. These are intentionally different when a
retained document has an owner URI and no existing source boundary. The retained
query view appends one `SourceBoundary` frame; private lexical evidence continues
to require the exact original parser map.

The existing [native evidence test](../../packages/cem_ql/tests/datatype_validation/preparation_evidence.rs)
already documents this distinction and rejects substituting the enriched query
map as lexical evidence. The [retained tree](../../packages/cem_ml/src/parser/tree.rs)
and [source-map enrichment](../../packages/cem_ml/src/source_map.rs) implement it.
No preparation, source-map or validation runtime code changed.

The corrected [linked fixture](../../packages/cem_ml_transform_cem_ql/tests/schema_package_lifecycle/attribute_datatypes.rs)
requires every original frame unchanged, followed by exactly one boundary whose
URI is the retained owner's URI and whose source ID/span equal the original
current frame. An empty map, absent URI or pre-existing boundary must retain the
entire original map unchanged. Boundaries are neither stripped nor normalized
away for comparison.

Two [native regressions](../../packages/cem_ml_transform_cem_ql/tests/schema_package_lifecycle/pretyped_attributes.rs)
exercise preparation and prepared-value consumption. Independent owners receive
their own exact URI boundary, with the expected frame stack built explicitly in
the test. A source that already has an authored boundary retains that boundary
even when its wrapper has a different URI. Both cases verify that the parser map
and retained query map remain unchanged after preparation/consumption. The first
also verifies one preparer callback with no replay during consumption.

Candidate ancestor/field restrictions, original node/tree ownership, context and
publication invalidation, cancellation and cumulative preparation budgets retain
their existing assertions. The [native datatype contract](../cem-datatype-compilation-design.md)
and evidence-map forgery refusals are unchanged.

## Verification

Before the fixture correction, the unchanged linked suite reproduces the same
three failures: pretyped handoff, callback context/publication changes and
checked preparation replacement (46/49 pass). Of the two new regressions, the
URI-enriched case fails the old equality assertion and the existing-boundary
case passes. Both pass after correcting that assertion.

The full corrected linked lifecycle suite passes **51/51**, including all three
previously failing cases and both new regressions (252.91 seconds). An earlier
post-correction full attempt ended with exit 143 without a final test summary;
it is not counted as a passing invocation.

The unchanged canonical suites pass: all 11 native preparation-evidence checks
(including projected-map/cleared-map/text forgery refusals) and all seven retained
source-provenance checks (including existing-boundary priority and empty maps).
The frozen Amsterdam delivery archive still matches its recorded byte count and
SHA-256. No public deployment or frozen material changed.

Focused verification used prepared dependencies and Linux x64 / Node 24.16.0.
Commands from the repository root, with `NX_DAEMON=false`:

```sh
yarn nx run cem_ml_transform_cem_ql:test --test=schema_package_lifecycle --args=pretyped_candidate_ --excludeTaskDependencies --skip-nx-cache
yarn nx run cem_ml_transform_cem_ql:test --test=schema_package_lifecycle --excludeTaskDependencies --skip-nx-cache
yarn nx run cem_ql:test --test=datatype_validation --args=preparation_evidence --excludeTaskDependencies --skip-nx-cache
yarn nx run cem_ml:test --test=source_provenance --excludeTaskDependencies --skip-nx-cache
```

The next unfinished follow-up isolates optional Studio sample generation from
the strict default Feature Tour generated-file set.
