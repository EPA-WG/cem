# Original application source handoff — 2026-11-01

- [x] Carry complete original base/mixed receiver and scalar schema sources with
  unchanged CEMAPPD4 bytes, exact role/selectors and independent packet pins.
- [x] Capture under actual publication/native admission guards; rebuild original
  receiver trees on fresh native hosts with explicit contexts, code, registry,
  compilation owners and crossing grants. Cover shared sources, wrong original
  captures, conflicting/changed metadata, malformed/oversized data, cancellation,
  disposal, foreign hosts and generation drains.
- [x] Port existing native application/module verification, compilation,
  preparation, receiver and publication contracts through the separate default-off
  `application-source-runtime` feature. Preserve other targets and default ABI.
- [x] Emit a native fixture and verify passive source restoration in independent
  WASM heaps and a real worker, with changed pins, malformed bytes, cancellation
  and subsequent successful reads after refusal.

## Contract and limits

The [canonical handoff contract](../cem-native-schema-capsules.md#original-application-source-handoff)
describes CEMAH1 framing, source agreement and guards. The entire encoded packet
is bounded to 4 MiB, with 2,112 external source roles, 16,384 aggregate external
nodes before restoration, 64 selectors per role, depth 64 and one million
preflight visits. Capture accounts cumulative source copying before retaining
another record. Existing nested package bounds are preserved. The shared-source
fixture is 257,955 bytes. These are encoded bounds, not peak-heap measurements.

No runtime context, callback, grant, input value, credential or old native handle
is serialized. The native embedding supplies actual owners before staging and
publishing. Whole-Studio recovery remains manual; entered passwords/OTP block
recovery until cleared, with the existing aggregate 4 MiB budget and deadlines.
No application/component JavaScript or UI change was added.

Live native bootstrap/installation endpoints and full worker/fallback
session/checkpoint handoff remain the next execution gates. The passive WASM
reader grants no installation authority. Component admission, generated app
bootstraps and linked-code omission remain separate.

## Verification

- 315 native regression tests passed: 296 CEM-QL application/module/rule/receiver
  tests and 19 CEM-ML generation/admission/inventory tests. The five new source
  handoff cases also passed after final capture bounds were added.
- 44 shared Node ownership/resource/factory/audit tests passed.
- `cem_ql:test:application-source-wasm` passed 24 reader checks across two
  independent heaps and a real worker, using the native-emitted original fixture.
- Opt-in application WASM compilation passed both with the component profile and
  the full/default profile. The separate query-only component profile also passed
  its WASM compilation check.
- 22 isolated browser checks passed for compiled declarations, attribute values,
  component generations, original fallback ownership, retirement and owner resume.
- Studio’s two complete live Login recovery/credential-privacy browser cases
  passed separately after native/WASM work finished (24 browser checks total).
- The refreshed catalog audit preserves 59 components, 24 shared capabilities
  and the unchanged 24,139,260-byte production WASM. The private reader probe is
  not a release artifact or a minimum-size measurement.
- Generated native fixtures, WASM/glue, screenshots and Vitest attachments remain
  outside the commit.
