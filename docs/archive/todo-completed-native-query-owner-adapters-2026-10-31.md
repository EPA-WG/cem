# Native query owner adapters — 2026-10-31

- [x] Add original native query source, binding and invocation lifecycle adapters
  using the shared generation ledger. Keep source/program verification passive
  and existing formats unchanged.
  - [x] Test actual retained roots after retirement/host loss, original context/
    source identity, cancellation, generation closure/loss, owner limits, failed
    replacement and fresh native source reimport/relinking.
- [x] Cover imported query-rule owners and native validator bindings with the
  same accounting/guards. Test callback-time closure/retirement, request reentry
  and late-output refusal.

## Delivered behavior

`FreshStructuralHost::query_source` creates an explicit `FreshQuerySource` only
for the original verified declaration registered in that actual host's ready
scope and epoch, with its original registry identity. The owner retains original
source, lexical capture and compiled-program components. Imported declarations
stored by value in verified dependency/module/assembly owners share those same
original components through a private lease; no new declaration, source tree,
program or host is synthesized to gain authority.

`link_query_source` and `replace_query_source` bind the original declaration and
real selected function registry while keeping only weak authority over the
explicit source owner. Source retirement or loss of its last caller-held owner
fences every dependent binding. Retained binding data remains accounted for
until the binding drops. Existing `link_query`/`replace_query` convenience APIs
retain an implicit source owner for their prior binding lifetime. Failed
function/source/limit checks leave the previous binding usable.

Query requests retain their original context and shared execution budget, can
be explicitly retired, and refuse reentry while executing. A failed nested
attempt leaves the outer request usable. Real native activity spans callbacks;
the post-call checks withhold results after cancellation, request/binding/
source retirement, context changes or generation closure/loss. Budget exhaustion
and pending failures are not cleared or restarted.

Native validator bindings and imported datatype rule registrations use the same
guards. Imported compilation availability checks include these original query
owners while preparing and validating values. Retired or hostless handles remain
roots until actual destruction, including retained request contexts after source
and binding owners have gone.

The existing v2 inventory categories remain unchanged: query source/program
owners use Compilation, bound query/native/imported-rule snapshots use Registry,
and requests/budgets use Preparation. These are ownership role categories, not
one concrete Rust type per counter. The 256-root/256-entry limits still apply.
Use `new_in_generation(..., runtime_admission::linked_generation()?)` for the
original linked runtime; unrelated gates and serialized identities do not prove
physical Worker/Memory membership. Legacy unmanaged hosts remain supported.

This is the native adapter layer. Physical query-owner endpoints, opaque source/
binding transport, fresh-WASM reimport/relinking and query-specific worker/
owned-fallback checkpoint handoff remain in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).
No new WASM export, wire format, renderer admission, component/app JavaScript,
checkpoint content or action replay is introduced. Entered passwords/OTPs still
block recovery until explicitly cleared; automatic Studio recovery remains off,
with unchanged 4 MiB checkpoints and five-second deadlines.

See the [design](../cem-native-schema-capsules.md#portable-query-lifecycle-owners),
[source owners](../../packages/cem_ql/src/structural_receiver/query/owners.rs),
[query binding/request adapter](../../packages/cem_ql/src/structural_receiver/query.rs),
[query fixtures](../../packages/cem_ql/tests/query_validator_capsule/generation.rs),
[callback fixtures](../../packages/cem_ql/tests/native_registrations/query_owners.rs)
and [imported-rule fixture](../../packages/cem_ql/tests/support/datatype_assembly_compilation.rs).

## Verification

- All 283 selected native cases passed: 264 cases across seven query, native
  registration, datatype assembly/module/dependency, envelope and structural
  targets, plus 19 native admission/inventory/generation cases. Nine new cases
  cover source/request owners, callbacks and imported rules. Missing adapter APIs
  failed before implementation. Existing provenance, budget, pending-state and
  weak callback-lifetime regressions remain intact.
- Full and reduced component WASM target compilation passed with the existing
  unused-code/tree-sitter warnings. These adapters remain excluded from WASM;
  existing optimized assets were exercised rather than rebuilt.
- All 33 shared lifecycle/audit/factory Node cases passed with explicit GC.
  Real-WASM owned-generation and memory-budget drivers passed: original native
  source retention, independent heaps, old aliases, disposal/GC, hard-pressure
  refusal, cleanup and joined native retirement.
- Four physical retirement/ownership/resume browser stories passed (96 filtered),
  plus all 18 selected shared component cases, as regressions of the existing
  physical lifecycle infrastructure, not acceptance of a new query transport.
- Complete Studio Login recovery passed through Nx in worker/fallback modes:
  two cases, six filtered. Native compilation/metadata was drained before
  isolated browser acceptance. Saved projects and drafts were not cleared.
- Source audit, focused Rust formatting, local documentation links and diff
  checks passed. Generated screenshots, Vitest attachments and runtime binaries
  remain excluded.
