# Mixed node/scalar/executable receiver contracts — 2026-10-31

- [x] Define a separately versioned mixed receiver declaration contract before
  generation-bound transport, ownership/checkpoint enrollment and admission.
  - [x] Add native roundtrip fixtures for structural nodes, shared closed scalar
    facets and exact CEMAPPD3 assembly-root requirements; preserve original sources.
  - [x] Test exact dependency/alias/occurrence agreement, unsupported policies,
    malformed/repinned records, source conflicts, limits, cancellation and legacy
    refusal. Reuse shared literal cardinality policy and test malformed,
    contradictory and out-of-budget count fields.
  - [x] Verify shared native/WASM/component checks and isolated Studio Login
    credential-policy acceptance; keep verification inert and admission closed.

## Delivered contract

Native-only `application_envelope::mixed` implements CEMMRC1 with a separately
typed package/manifest/parent/registry pin. It packages one compiled declaration
and a named receiver map against an independently verified CEMAPPD3 package.
It does not append that declaration to the parent or replace a public tag.

Node requirements retain exact SC1 schema/source/registry pins from the parent.
Closed scalar requirements reuse the existing D1 facet policy, including original
source maps and restrictions. They remain trusted build-supplied projections;
source assertions are checked for hash consistency without claiming a reconstructed
original schema AST or source-owner permission. Executable requirements identify
exact parent-exported assembly roots; the parent already verifies CEMDM2/CEMOP2/
CEMOS1 pins and native source agreement. No callbacks or executable descriptors
are serialized, and unsupported kinds cannot fall back to scalar strings.

Verification recomputes the shared native inventory and retains exact literal
receiver fields from the compiled IR. Alias requirements must agree completely;
local attributes are unique top-level directives with explicit types and the
bounded name/type/count field vocabulary. Static component dependencies must
already exist in the parent. Dynamic/nested/qualified declarations, defaults,
unknown policies, extra fields and unresolved inventory gaps refuse. Primitive
occurrences also count toward the inventory bound. The source and registry demand
inventory stays available for later composition; verification extends no registry.

SC1 and mixed declarations now share the original literal count-envelope policy.
Malformed/overflowing counts, inconsistent intervals and exact/min/max conflicts
refuse, preserving SC1's 0–100,000 bound and absent-field semantics. Full native
schema/runtime count rules are unchanged. A failing-first count fixture caught
acceptance of malformed local count metadata before the shared policy was applied.

CEMMRC1 has a 1 MiB encoded limit, 64 KiB declaration metadata limit, at most 64
aliases and 64 total occurrences, existing D1 scalar projection limits, canonical
binary preflight and caller-controlled verification work. Parent CEMAPPD3 remains
independently bounded and referenced, without copying its native source owners.
These are input bounds, not a measured heap or recovery budget.

See the [design](../cem-native-schema-capsules.md#mixed-receiver-declaration-contracts),
[implementation](../../packages/cem_ql/src/application_envelope/mixed.rs),
[integration fixtures](../../packages/cem_ql/tests/application_envelope/mixed.rs),
[wire fixtures](../../packages/cem_ql/src/application_envelope/mixed/tests.rs)
and [preceding native root installation](todo-completed-assembly-application-installation-2026-10-31.md).

## Verification

- A failing-first native mixed roundtrip preceded implementation. Nine new
  integration cases and four wire cases cover all requirement variants, ordinary
  scalar rendering/facet diagnostics, exact pins and fields, source claims,
  static dependencies, cardinalities, bounds, cancellation and legacy refusal.
- All 760 distinct native cases passed: 554 integration and 206 core/shared
  traversal cases. WASM-target compilation passed.
- All 18 selected component browser cases passed. Isolated complete Studio Login
  recovery passed through Nx in worker/fallback modes: two selected cases, six
  filtered. Native compilation/metadata work was drained before browser acceptance;
  saved projects and drafts were not cleared.
- Source audit, focused Rust formatting, relative documentation links and diff
  checks passed. Generated screenshots and Vitest attachments remain excluded.
- Ten audit/factory cases and both independent-heap WASM drivers passed. Existing
  component (11,474,094 bytes) and full (24,202,382 bytes) assets were exercised,
  not rebuilt. No endpoint is added to those binaries by this native-only stage.

Live mixed binding against original receiving occurrences and actual-host guards,
complete application composition/publication, generation-bound transport and
checkpoint ownership remain gates. Both runtime-admission and executable-closure
flags remain false; CEMAPPD1/CEMAPPD2/CEMAPPD3 admission remains unchanged. No
application/component JavaScript was added, and shared UI/theme ownership is
unchanged. Entered passwords/OTPs still block recovery until explicitly cleared.
Automatic whole-Studio recovery remains off, with 4 MiB checkpoints and five-second
deadlines. Remaining work is in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
