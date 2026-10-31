# Atomic CEMAPPD3 root and owner installation — 2026-10-31

- [x] Stage explicit root/owner installation and replacement atomically;
  failed staging preserves prior bindings without partial receiver permissions.
  - [x] Add native fixtures binding exact CEMAPPD3 graph owners and exported roots
    to actual-host compilations; exercise preparation, conversion and list export.
  - [x] Test missing/extra/foreign/stale compilations, weak owner loss, failed
    replacement, retirement and late callback results without retiring reusable
    owners. Cover installation-wide peer loss across multiple assemblies and
    reentry/busy publication refusal.
  - [x] Verify structural receiver compatibility and isolated worker/fallback
    Login privacy acceptance; preserve legacy admission, budgets and deadlines.

## Delivered contract

Native-only `structural_receiver::application::v3` adds explicit staging and a
separate synchronous application replacement slot for CEMAPPD3. The embedding
prepares actual source contexts, crossing grants, registrations, implementation
owners and ordinary CEMDM2 compilations first. Exact inert envelope owner accessors
retain original arenas; existing borrowed accessors and wire bytes are unchanged.

Staging checks every named original exported root against the matching original
graph's caller-held compilation on the actual host. Missing/extra bindings,
independently restored copies with identical pins, foreign/stale contexts and
retired owners refuse. Matching addresses alone supply no permission. No callback,
source evaluation, fetch, compilation or host-registry change occurs during stage
or publication. Mutable providers are not frozen; changed inputs still need the
ordinary renewed source selection/compilation path.

Every installed root shares one installation-wide liveness guard, with weak
references to all required graph/compilation owners and the base application.
Peer ownership loss across multiple graphs blocks retained sibling roots too.
The guard retains no host or callback. The embedding keeps compilations and
implementation owners alive; a temporary last owner cannot authorize publication.

Publication rechecks all guards immediately before replacing one native slot.
Failed staging or publication preserves its prior contents. Abandoning a staged
candidate, replacing/clearing a slot or explicitly retiring an installed root
revokes that installation's root guards, including retained clones. Caller-held
compilations remain reusable by other installations. Structural receiver models
retire through the base application's existing guards on slot replacement/clear;
they are not widened to include named datatype roots.

Installed roots delegate validation, preparation, conversion and list serialization
to the shared guarded compilation paths. Original restricted source/rule contexts,
source diagnostics/spans, cumulative/query budgets and missing-capability results
remain intact. Reentry and publication while a required compilation is executing
refuse. Clearing an installation during a callback withholds late reports without
retiring its reusable compilation.

The CEMAPPD3 bounds remain eight assemblies, 64 named roots and 64 aggregate graph
modules, with existing base-application and nested format limits. Added guards and
weak ownership entries are bounded by the alias inventory. This is not a measured
heap budget or a physical-generation transaction.

See the [design](../cem-native-schema-capsules.md#native-assembly-application-installation),
[implementation](../../packages/cem_ql/src/structural_receiver/application/v3.rs),
[shared ownership guard](../../packages/cem_ql/src/structural_receiver/application/v3/ownership.rs),
[fixtures](../../packages/cem_ql/tests/support/datatype_assembly_installation.rs)
and [preceding envelope contract](todo-completed-assembly-application-envelope-2026-10-31.md).

## Verification

- A failing-first native installation fixture preceded implementation. Ten native
  cases cover exact roots/owners, callback-free staging, ordinary operation
  forwarding, transactional replacement, source identity, weak ownership, host
  disposal, cancellation, peer loss, reentry, late reports and structural models.
- All 747 distinct native cases passed: 545 integration and 202 core/shared
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

Mixed node/scalar/executable named receiver contracts, generation-bound transport,
checkpoint enrollment and component admission remain separate gates. Envelope
selection/executable/native-link closure flags remain false; CEMAPPD1/CEMAPPD2
admission remains unchanged. No application/component JavaScript was added, and
shared UI/theme ownership is unchanged. Entered passwords/OTPs still block recovery
until explicitly cleared. Automatic whole-Studio recovery remains off, with 4 MiB
checkpoints and five-second deadlines. Remaining work is in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
