# Login element-template source identity — 2026-10-27

- [x] Reproduce the authored generic-source failure with native CLI fixtures before changing selectors.
- [x] Adopt the existing element-template identity for the live resource and its command, retaining authored bytes and stable IDs.
- [x] Retain original generic-source diagnostic severity, URI and ranges as compatibility evidence; require the corrected source to pass Parse and Validate without hard binding errors.
- [x] Remove only Login's obsolete catalog acceptance exception, preserve advisory warnings and record the independent worker semantic-validation gap.
- [x] Verify editing, live rendering and complete worker/fallback recovery without changing credential policies, limits or deadlines.
- [x] Verify deployed independent saved/offline copies.
- [x] Complete regenerated catalog, deployed demo and source verification before delivery.

## Cause and correction

`login-live-view` (`views/login.cem`) and its `live-view` Parse configuration
used generic CEM-ML selectors. Standalone Parse returned zero while publishing
four hard `cem.ql.compile_failed` diagnostics for runtime `datadom` and the
locally declared `heading` binding. Validate returned one for the same source.

Both declarations now use the existing registered pair:

- `application/vnd.cem.element-template+cem`
- `https://cem.dev/ns/template/cem-element/1`

This reuses the same shared native contract as the HTML/CEM card example. Native
Parse and Validate succeed with **15 advisory**
`cem.schema.unknown_html_attribute` warnings. Their warning severity, original
URI and zero offsets remain intact; the change does not suppress them.

Login **2.8.1** retains every resource/configuration ID and path and the exact
original template bytes. `data/login.cem` remains generic data and
`templates/view.cemt` remains a native build template. Newly cloned projects
carry the corrected selectors. Existing saved projects, commands and drafts
retain their own identities and are not automatically rewritten.

The catalog no longer accepts the old four hard diagnostics as a known limit
for the corrected Login configuration. The original generic-source path remains
covered in Rust: Parse still returns zero with four errors, Validate returns one,
and the original offsets **196, 657, 1147, 1187**, embedding ranges and URI remain
unchanged. An intentionally undeclared binding still fails native CLI Validate
at original byte **655** and clears after repair.

The worker validation host differs for that intentionally undeclared binding:
it currently returns zero errors and the 15 warnings. The browser fixture pins
this observed difference separately; the active TODO records the shared native
validation/preparation follow-up. Worker success is not claimed as strict
undeclared-binding conformance. Malformed source must still refuse publication.

## Declarative layout and recovery

Displaying the new advisory diagnostics pushed the complete shell DOM to
**33,591 nodes**, beyond its existing **32,768-node** witness guard; depth and
text size remained within bounds. Merely removing a few single-child wrappers
allowed the initial capture but not later recovery cycles.

Redundant result-panel and command wrappers are removed. Studio-specific form,
command, editor-detail and build-control groups use native structural containers
inside their existing declarative components, consuming the same shared theme
gap tokens. Controls, tabs, previews, diagnostics and multi-item field grouping
continue to use shared components. All diagnostics remain present. No new
application behavior JavaScript or new runtime capability is introduced.

A fixture-only failure inventory records node counts, depth and text lengths,
without source text, attributes, input values or credentials. Runtime witness
limits, 4 MiB checkpoint/group budgets, existing deadlines, source ownership and
recovery admission rules are unchanged. Entered passwords and one-time codes
still block recovery until explicitly cleared; automatic whole-Studio recovery
remains disabled. Browser verification uses isolated fixture-owned contexts and
preserves the user's projects and drafts.

## Verification

- Native focused CLI: **8/8 pass**: three identity/compatibility/repair cases,
  two Login graph cases and three native template cases.
- Catalog acceptance/audit units: **47/47 pass**.
- Native Feature Tour: **250 configurations executed**, with 186 passing,
  30 verified Inspect outputs, 33 intentional diagnostics, one unchanged
  Supplier source limit and ten separate Inspect-companion limits. All 20 Login
  configurations and the 32 native schema-package examples are covered.
- Source/preview browser acceptance: **7 selected cases pass**, covering five
  source-editor cases (including readonly Login template coloring, URL selection,
  independent clone, durable draft recovery and saving), the authored worker
  command fixture and shared Login light-DOM preview with data/template edits.
- Worker command fixture passes with actual authored selectors, direct Parse and
  Validate, original/edited independent copies, all advisory warnings, malformed
  source refusal and explicit semantic-validation gap evidence.

- Complete live Login recovery: **2/2 pass**, one worker and one fallback case.
  Each retains two credential refusals, three fresh-generation recoveries,
  twelve edits, unknown-operation refusal and all existing byte/deadline checks.
- Complete package verification passes for both browser/Node ABIs,
  **657 integrity records and all 32 schema packages**. Runtime source is
  unchanged; the previously optimized releases retain their exact hashes.

- Deployed presentation acceptance passes **23 selected configurations**, including
  all **20 Login configurations** and three Supplier configurations. It checks
  independent saved copies, source/schema/template repairs, graph provenance,
  editor and persisted metadata, actual live control interaction, forced colors,
  saved data/template edits, reload and offline execution. The corrected MIME,
  schema and command selectors remain intact after saved/offline reopen.

- Assembled demo acceptance passes: all owned hashes and all 333 Studio files
  match the build, paired runtimes preserve independent factory memories, native
  rendering/highlighting and Studio selection/saved-copy/HTML-CSS preview/offline
  rebuild/worker scope pass. All ten theme generators retain their frozen CSS;
  all 120 canonical playgrounds and the live tree gallery pass.

Studio is rebuilt against the verified unchanged dependencies. The existing
inventory publisher replaces Studio's 333 files while checking the unchanged
managed gallery bytes. Its original 999-file, strict below-25-MiB, ownership/hash
cleanup and unrelated-file guards remain active. The assembled deployment has
**898 files, 114,692,313 bytes**; its four WASM files retain **22.42/23.04 MiB**
and all capabilities. Generated binaries, screenshots and Vitest attachments
are excluded from the source commit.
