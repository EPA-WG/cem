# Versioned assembly-aware application envelope — 2026-10-31

- [x] Define a separately versioned assembly-aware application envelope with exact
  CEMDM2/CEMOP2/CEMOS1 pins, bounded inventories and whole-source agreement;
  preserve existing CEMAPPD1/CEMAPPD2 admission semantics.
  - [x] Add native roundtrip fixtures for explicit assembly roots, exact
    module/profile/operation pins, shared dependency bytes and fresh inert owners.
  - [x] Add refusal fixtures for cross-assembly/base source and implementation
    conflicts, missing/unused records, malformed or repinned inventories, limits
    and cancellation.
  - [x] Verify legacy format/admission refusal and isolated worker/fallback Login
    credential-policy gates; record limits without enabling publication.

## Delivered contract

Native-only `application_envelope::v3` implements CEMAPPD3 with a separately typed
trusted package/manifest/registry pin. One exactly pinned CEMAPPD2 binary retains
its existing compiled declarations, structural named receivers, query/native
contracts and selected registry recipe. Explicit application root aliases name
original exported datatypes in the root module of a selected CEMDM2 assembly.
These aliases do not bind component receiver names.

Flat inventories retain exact CEMDM2 graphs, DT1/QD1/NR1 leaves, CEMOP2 profiles
and CEMOS1 sources. Shared artifact bytes are packed once by content hash. The
ordinary nested verifiers check exact associations, source/profile mapping,
imports, constant target order, closure and all existing nested limits. Missing,
unused, mismatched or conflicting records refuse. Application and graph
inventories expose deterministic dependency orders.

A shared source ledger checks complete original snapshots and identities across
all graphs, leaves/profiles and the base structural/query/native contracts.
Conflicting snapshots refuse even under an identical claimed source hash. A
shared native implementation ledger also refuses the same implementation ID
with different ABI/module/export/signature identity across CEMNR1/CEMOS1. There
is no JSON AST handoff, source-text reparse, fabricated resolver or runtime
selection. Fresh verification retains independent source owners; each profile
maps to its own graph's original source arena after whole-source agreement.

Encoded package and unique outer bytes each remain bounded by 4 MiB. Additional
bounds cover eight assemblies, 32 profiles, 64 leaves/operation sources, 64 root
aliases, 64 aggregate restored modules and 256 associations in each inventory
layer. Canonical MessagePack preflight, nested verifier budgets and caller
cancellation remain explicit. These are input bounds, not measured heap limits.
Refused builder insertions preserve prior staged inventories. Verification never
publishes a live registry or replaces a prior owner.

See the [design](../cem-native-schema-capsules.md#versioned-assembly-aware-application-envelope),
[implementation](../../packages/cem_ql/src/application_envelope/v3.rs),
[integration fixtures](../../packages/cem_ql/tests/application_envelope/versioned.rs),
[wire fixtures](../../packages/cem_ql/src/application_envelope/v3/tests.rs)
and [preceding root operations](todo-completed-assembly-root-operations-2026-10-31.md).

## Verification

- A failing-first native roundtrip fixture preceded the API. Eight new integration
  cases and four wire/boundary cases cover exact pins, original owners, shared
  profiles/sources, datatype leaves, structural/query contracts, whole-source and
  implementation conflicts, repinned/malformed inputs, cancellation, limits and
  transactional staging.
- All 737 distinct native cases passed: 535 integration and 202 core/shared
  traversal cases. The WASM target compilation check passed.
- All 18 selected component browser cases, ten audit/factory cases and both
  independent-heap WASM drivers passed.
- Isolated complete Studio Login recovery passed through Nx in worker/fallback
  modes: two selected cases, six filtered. Native compilation/metadata processes
  were drained before browser acceptance. Saved projects and drafts were not cleared.
- The source audit, focused Rust formatting, relative documentation links and
  diff checks passed. Existing component (11,474,094 bytes) and full
  (24,202,382 bytes) WASM assets were exercised, not rebuilt; this native-only
  package format does not add an endpoint to those binaries. Generated screenshots
  and Vitest attachments remain excluded.

Atomic actual-host root/owner installation and replacement, mixed
node/scalar/executable receivers, physical-generation transport and checkpoint
ownership remain separate gates. The verifier's selection/executable/native-link
closure flags remain false. Existing CEMAPPD1/CEMAPPD2 readers and component
admission refuse CEMAPPD3. No application/component JavaScript was added and
shared UI/theme ownership is unchanged. Entered passwords/OTPs still block
recovery until explicitly cleared, automatic whole-Studio recovery remains off,
and 4 MiB checkpoints and five-second deadlines are unchanged. Remaining work
is in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).
