# Native-provisioned application installation — 2026-11-01

- [x] Add original Rust bootstrap/code owners and live installation over CEMAH1
  sources, actual fresh hosts/control, registry/compilation and typed retained
  native rule/operation/tokenizer/query/profile owners.
- [x] Exercise complete native mixed consumers, source recapture, changed pins,
  equal-byte foreign source objects, foreign runtime/control, missing owners,
  atomic replacement/failure, callback-time code retirement/drop, escaped
  preparation handles, cancellation, bounded sessions and drain cleanup.
- [x] Add non-panicking shared native-registry mutable access and a census
  invalidation fixture for reentrant refusal.
- [x] Compile the default-off WASM installation ABI; exercise refusal without
  provisioning and unknown tokens in independent heaps and a real worker.

The [native endpoint contract](../cem-native-schema-capsules.md#native-provisioned-application-installation)
defines actual ownership, weak publication enrollment, native admission and
original context checks. Packet pins cannot supply callbacks or grants. Bootstrap
invocation retains executable code while releasing temporary owner authority
before the callback; expiry is checked before publication and after native
operations. Escaped receivers/preparations use the same shared guard.

`PreparedApplication` retains only explicit typed owners. `replace` stages and
publishes a complete candidate before retiring the prior session; failures release
candidate roots and preserve the prior publication under its existing owner
checks. Eight sessions share the
existing 256-root generation ceiling, with larger applications able to exhaust
that shared ceiling sooner. Packets remain bounded to 4 MiB and pins to 16 KiB.
Actual resources, mutations and active calls remain in the original census.

The opt-in `applicationSourceOpenV1/CaptureV1/ReleaseV1` ABI requires Rust-only
installation and original per-heap object tokens. Cleanup behind closed admission
requires the original token. No JS owner constructor, source parser substitute,
application/component JS, component admission or default export change was added.

Installed fresh worker/fallback mixed-consumer acceptance and full shared
resource/checkpoint/journal handoff remain the next gate. The unprovisioned probe
is not that acceptance. Credential refusal, manual Studio recovery, existing
aggregate budget and deadlines remain unchanged.

## Verification

- 323 native tests passed: 303 application/module/rule/receiver tests and 20
  generation/admission/inventory tests, including seven new endpoint cases and
  the new reentrant-registry census fixture.
- 44 shared Node ownership/resource/factory/audit tests passed.
- `cem_ql:test:application-source-wasm` passed 36 reader/ABI-refusal checks in
  two independent heaps and a real worker, using the 257,955-byte native fixture.
- Opt-in application WASM compilation passed under both component and
  full/default profiles.
- 22 isolated component browser tests passed for compiled declarations, native
  attribute values, generations, fallback retirement and fresh owner resume.
- Studio’s two complete Login recovery/credential-privacy browser cases passed
  separately after native/WASM work finished (24 browser checks total).
- The refreshed catalog audit preserves 59 components, 24 shared capabilities
  and the unchanged 24,139,260-byte production WASM. Private probe artifacts are
  not release-size measurements.
- Generated source fixtures, WASM/glue, screenshots and Vitest attachments are
  excluded from the commit.
