# Opt-in query source WASM core — 2026-10-31

- [x] Make original query source/host/binding/request adapters compile behind an
  opt-in WASM feature without enabling production exports. Test native-emitted
  CEMQH1 bytes in independent WASM heaps and a real worker: provenance, missing
  functions, malformed packets, cancellation, budget exhaustion, original source/
  context guards, retained roots and closure/drop. Keep credential refusal and
  complete physical checkpoint handoff as separate admission gates.

## Delivered behavior

`query-source-runtime` is a disabled-by-default Cargo feature in CEM-QL and
CEM-ML. It shares the existing original query host, passive source/IR verifier,
source/binding/request leases, selected evaluator and cumulative native validator
work budget with Rust WASM embeddings. It works with the full runtime and the
separate component prototype. No production JavaScript source endpoint or
application admission is added.

Original structural receiver installation, native implementation registration,
datatype/imported-rule assembly and CEMAPPD4 publication stay native-only. There
is no parallel host, evaluator, fabricated scope or JSON AST boundary. The
passive source identity moves to the shared native source module and is
re-exported at the existing native structural capsule path. Its fields and
CEMQV1/CEMQD1/CEMQH1 wire representations are unchanged.

The test-only `query_source_probe` example constructs trusted fixture hosts on
the actual linked generation. It supplies reviewed function implementations and
fresh checked result adapters from the shipped schema, registers the exact
restored lexical capture, links original native bindings and explicitly creates
new requests. Probe exports are confined to the example and generated `dist`
fixture, outside both production WASM entry points.

The acceptance target first authors the binary source through a native Rust
fixture and writes independent deployment pins. Its Node driver consumes those
bytes in two distinct WASM memories and a real worker. It checks original
diagnostic provenance, unavailable functions, malformed bytes, cross-host request
refusal, source-only capture, cancellation and changed host contexts. Repeated
evaluations exhaust one enclosing budget; subsequent calls do not reset it.

Retired/cancelled owners remain in the original native role ledger until actual
drop. Closing the linked gate refuses ordinary work and old aliases. A real
five-second scoped drain cannot seal while query owners remain; cleanup drops
the actual original owners before sealing. The scoped ledger reports present
roles; it is not the complete fixed native census, a physical retirement receipt
or permission to allocate a replacement generation. Module-cache collection or
complete production physical disposal is not claimed by the probe.

The trusted fixture adapter parses the shipped schema; transported query source
is never reparsed/recompiled. The probe therefore is not a minimal-runtime size
measurement or proof that schema tooling has been removed. Its generated binary
is 14,565,785 bytes without wasm-opt and remains a test artifact.

Production shared worker/owned-fallback resource adapters, source/code/registry
owners and complete checkpoint/journal integration remain in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
Packets and scoped proofs grant no recovery authority. Entered passwords/OTPs
still block recovery until explicitly cleared; automatic Studio recovery stays
off, with the existing 4 MiB checkpoint and five-second recovery budgets.
No component/application JavaScript behavior is added.

See the [design](../cem-native-schema-capsules.md#opt-in-query-source-wasm-adapters),
[test-only native harness](../../packages/cem_ql/examples/query_source_probe.rs),
[WASM/worker driver](../../packages/cem_ql/tests/query-source-wasm.mjs) and
[verification target](../../tools/scripts/verify-query-source-wasm.mjs).

## Verification

- The feature check initially failed because no WASM query-source feature
  existed. The new `cem_ql:test:query-source-wasm` target now builds the real
  native-emitted source and test WASM example. The driver passed in two
  independent memories and a real worker, including budget, original source/
  context, cancellation, owner lifetime and drain checks.
- All 307 selected native cases passed: 273 query/native-registration/datatype/
  envelope/structural cases, 19 admission/inventory/generation cases, five
  original-source unit cases and ten structural-capsule unit cases. Existing
  native public paths, source metadata and provenance remain covered.
- Full/default, full with `query-source-runtime`, baseline component and opt-in
  component/query WASM target checks passed. The release probe compiled through
  pinned wasm-bindgen 0.2.122. Existing tree-sitter and unused-code warnings
  remain; the opt-in query core includes a native-only consumer's unused demand
  helper on WASM.
- All 33 shared lifecycle/audit/factory Node cases and the existing real-WASM
  owned-generation and memory-budget drivers passed. The source audit was
  refreshed; the unchanged full production binary remains 24,139,260 bytes.
  Existing production assets were exercised rather than rebuilt, since the
  new feature is disabled there.
- All 22 selected component/runtime browser cases passed: four physical
  retirement/ownership/resume cases (96 filtered) and 18 compiled declaration,
  generation and attribute cases. These are existing production regressions,
  not acceptance of a new production query endpoint.
- Complete live Studio Login recovery passed through Nx in worker and fallback
  modes: two cases, six filtered. Native compilation and metadata had finished
  before isolated browser acceptance. Credential refusal and existing recovery
  budgets remain covered; saved projects and drafts were not cleared.
- Focused Rust formatting, Node syntax, diff checks and all 334 local
  documentation links passed. Completed work was archived from the active TODO.
  Generated WASM fixtures, screenshots and Vitest attachments remain excluded.
