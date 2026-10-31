# Complete native application publication — 2026-10-31

- [x] Compose mixed declarations into a separately versioned complete application
  publication with exact registry demand and live native code/owner closure.
  - [x] Add failing-first CEMAPPD4 composition fixtures preserving original
    CEMAPPD3/MRC1 owners, complete component roots and template/query demand.
  - [x] Reject repinned/unknown/trailing packages, duplicate or unreachable
    declarations, conflicting declaration/scalar/native-source assertions and
    incorrect inventories; preserve nested limits and legacy refusal.
  - [x] Publish parent/mixed groups under one guard with exact caller-held
    registry and compilation/code owners. Test original parent structural input,
    peer loss, renewed selectors, stale receipts, callback retirement, reentry,
    weak lifetimes and atomic replacement.
  - [x] Verify native/WASM/shared component checks and isolated complete Studio
    Login recovery with the existing credential privacy policy.

## Delivered behavior

Native-only `application_envelope::v4` adds CEMAPPD4. It packages one original
CEMAPPD3 parent, CEMMRC1 declarations and explicit component roots. Verification
retains one restored parent owner for all children, recomputes the complete
static component graph through the shared bounded dependency order, and rejects
missing, duplicate, cyclic or unreachable records. Source assertions agree
across original declarations, scalar projections and the parent's native sources.
Native source snapshots retain the parent's complete agreement checks; a
declaration/scalar hash does not fabricate a source snapshot or grant authority.

The combined registry demand reuses the shared template IR inventory/planner and
includes verified portable query-function demand from parent contracts and
assembly leaves. Requirements, template plan and combined demand are pinned
independently. The original parent recipe is unchanged. All nested formats and
legacy readers preserve their semantics; older readers refuse the new package.

`structural_receiver::application::publication` links an exact caller-held
registry selection to the original package instance and actual host epoch. It
reuses parent staging, exact compilation/operation/rule owners and original mixed
receiver binding. One guard enrolls all peer groups, parent guards and registry
ownership. Native fields and prepared receipts renew the whole publication's
receiving selectors; guarded native operations also cover root validation,
preparation, conversion/list serialization, parent structural input and a
synchronous borrowed-registry callback. Raw parent models/installations do not
escape as unchecked publication authority.

The atomic slot explicitly retires old publications even when retained `Arc`
handles remain. Failure preserves the previous slot. Registry/peer/code-owner
loss, epoch changes, closure, cancellation, reentry and callback-time retirement
withhold output. Registry callbacks release temporary leases before checking
caller ownership again. Retained applications/receipts keep no caller registry,
compilation or host callback owner alive. Reusable compilations and independently
staged parent installations remain available after publication retirement.

The package remains bounded to 4 MiB encoded, 32 mixed declarations, 64 total
components and 256 mixed occurrences, with original nested limits, canonical
preflight and caller-controlled work. CEMMRC1's parent-only static dependency
subset remains unchanged; sibling references require a future declaration
contract. Original receiving schemas, data scopes/grants and executable
registrations still come from the actual native embedding.

See the [design](../cem-native-schema-capsules.md#versioned-complete-application-publication),
[envelope](../../packages/cem_ql/src/application_envelope/v4.rs),
[native publication](../../packages/cem_ql/src/structural_receiver/application/publication.rs),
[composition fixtures](../../packages/cem_ql/tests/application_envelope/publication.rs),
[wire fixtures](../../packages/cem_ql/src/application_envelope/v4/tests.rs)
and [lifecycle fixtures](../../packages/cem_ql/tests/support/application_publication.rs).

## Verification

- Twenty new cases passed: six composition, nine native lifecycle/parent-input
  and five wire/bounds cases. Missing APIs failed before implementation. Separate
  failing-first regressions demonstrated late registry callback output after
  last-caller ownership loss and conflicting original declaration source claims;
  both now refuse.
- All 796 distinct native cases passed: 585 integration cases across 19 targets
  and 211 core/shared traversal cases across ten runs. The final assembly target
  passed all 67 cases, including the additional parent-input fixture.
  WASM-target compilation passed with the existing cem-ml unused-field warnings;
  native tree-sitter retains its existing unused-label warning.
- All 18 selected shared component browser cases passed. Isolated complete
  Studio Login recovery passed through Nx in worker/fallback modes: two cases,
  six filtered. Native compilation/metadata work was drained before browser
  acceptance. Saved projects and drafts were not cleared.
- Ten audit/factory cases and both independent-heap WASM drivers passed. Existing
  component (11,474,094 bytes) and full (24,202,382 bytes) assets were exercised,
  not rebuilt. Source audit, focused Rust formatting, local documentation links
  and diff checks passed. Screenshots and Vitest attachments remain excluded.

This proves the selected publication's live native owner closure, not linked
binary omission, arbitrary provider closure or physical runtime/deployment
authority. Portable link-closure and runtime-admission flags remain false. There
is no renderer/component admission, worker/fallback endpoint or new production
JavaScript. Shared component/theme ownership and credential privacy remain
unchanged: entered passwords/OTPs block recovery until explicitly cleared;
automatic whole-Studio recovery stays off, with 4 MiB checkpoints and five-second
deadlines. Generation enrollment, fresh-source/owner transport and the reduced
UI ABI remain in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).
