# Native query source handoff — 2026-10-31

- [x] Add bounded, pinned query-source handoff capture from original live
  bindings and passive fresh-source verification. Test exact selectors,
  original provenance, payload exclusion, malformed/oversized packets,
  callback refusal, cancellation and stale owners. Require fresh host scopes,
  functions, adapters and requests; keep physical WASM endpoints and admission
  disabled until worker/fallback and credential acceptance is complete.

## Delivered behavior

The separate CEMQH1 source packet carries the original verified CEMQD1 bytes,
datatype occurrence and result/diagnostic adapter names from an actual live
pure-query binding. Capture checks original source/context/generation ownership,
holds native admission through serialization and rechecks before returning.
Native callbacks cannot be exported through this pure-query source adapter.

Verification requires an independent packet/declaration/program/source pin,
canonical bounded framing, exact ML/QL versions and a valid original datatype
selector. It shares the existing 100,000 work-check budget and cancellation
across the wrapper and nested verification. The packet is limited to 69,643
bytes: seven-byte magic, four-byte length, at most 4 KiB metadata and the
existing at-most-64-KiB declaration. Existing CEMQV1/CEMQD1 formats are unchanged.
Verified declarations retain one immutable bounded wire image, shared by their
owners; there is no previous-generation retention chain.

Reimport restores fresh passive CEM source arenas with original provenance and
typed instructions. It creates no host, grants, native registrations or request.
The embedding explicitly registers the exact restored source/lexical capture
and supplies its own context/policy, reviewed functions and checked result
adapter. The fresh binding owns new source/registry leases; a fresh invocation
must be created explicitly. Foreign/equal-but-different source allocations and
old requests/bindings fail. Failed linking drops staged leases without retiring
other bindings. Result-adapter names remain requirements; their schema/code
implementations and original profile ownership are separate dependencies.

Evaluation values/candidates/results, request budgets, callbacks and runtime
contexts never enter this source record. It is not an ownership receipt or
recovery authorization. The enclosing checkpoint policy must still refuse
entered passwords/OTPs until clearing, prove complete physical ownership and
drain all work. The 4 MiB/five-second limits and disabled automatic Studio
recovery are unchanged. No application/component JavaScript is added.

This completes the native source-adapter prerequisite only. Physical query
worker/owned-fallback transport, shutdown/replacement, imported rule/code and
registry owners, fresh-WASM handoff and credential acceptance remain in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
No WASM endpoint or reduced-runtime admission is enabled.

See the [contract](../cem-native-schema-capsules.md#query-source-handoff),
[packet verifier](../../packages/cem_ql/src/query_validator_capsule/handoff.rs),
[host adapter](../../packages/cem_ql/src/structural_receiver/query/handoff.rs),
[source fixtures](../../packages/cem_ql/tests/query_validator_capsule/handoff.rs)
and [native callback fixture](../../packages/cem_ql/tests/native_registrations/query_owners.rs).

## Verification

- Eight new native cases passed; the source API fixtures first failed for missing
  APIs before implementation. They cover
  fresh original source/capture reimport, native diagnostic provenance, owner
  drain lifetimes, missing functions/adapters, old/foreign handles, context/source
  loss, cancellation, malformed/noncanonical/oversized packets, payload exclusion,
  callback refusal and failed owner allocation. Capture bytes are unchanged
  across requests with different evaluated values.
- All 291 selected native cases passed: 272 query/native-registration/datatype/
  envelope/structural cases and 19 admission/inventory/generation cases. The
  strengthened source/capture/canonical-wire fixtures were rerun successfully.
- Full and reduced component WASM target compilation passed. This source adapter
  remains excluded from WASM; existing optimized binaries were exercised rather
  than rebuilt. Existing tree-sitter/unused-code warnings remain.
- All 33 shared lifecycle/audit/factory Node cases passed with explicit GC.
  Real-WASM owned-generation and memory-budget drivers passed, including joined
  retirement, retained native sources, hard-pressure cleanup, independent heaps,
  stale aliases and disposed-memory collection.
- The 59-component/24-capability source audit passed and was refreshed. The
  unchanged full runtime remains 24,139,260 bytes. This is not a new minimal
  profile measurement or evidence of physical query transport acceptance.
- All 22 selected shared browser cases passed: four physical retirement/ownership/
  resume cases (96 filtered) and 18 component rendering/generation/attribute
  cases. These exercise the existing runtime; they do not accept a new query
  endpoint or physical source transport.
- Complete live Studio Login recovery passed through Nx in both worker and
  fallback modes: two cases, six filtered. Entered-credential refusal and the
  existing recovery budget remain covered. Native compilation and metadata had
  finished before browser acceptance. Saved projects/drafts were not cleared.
- Focused Rust formatting, diff checks and all 275 local documentation links
  passed. Generated screenshots, Vitest attachments and runtime binaries remain
  excluded from this change.
