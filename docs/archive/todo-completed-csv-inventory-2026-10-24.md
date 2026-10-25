# CSV inventory and portable validation configurations, 2026-10-24

- [x] Add the ordinary [CSV inventory project](../../examples/projects/csv-inventory/README.md)
  to Data and Queries and transformations. Its 18 authored resources and 12
  configurations produce a portable archive with 30 resources. Reuse Studio's
  shared source editor, compact CSV/JSON tables and sandboxed HTML artifact view.
  No dedicated example screen or new JavaScript UI capability was introduced.
- [x] Demonstrate explicit present/absent headers, UTF-8 names, quoted commas and
  doubled quotes. Include native inspection, validation, compact interchange
  formatting, aligned review formatting, standalone queries and report builds.
  Keep CSV cells as strings at import. Authored native rules convert canonical
  nonnegative counts, select low-stock rows and retain invalid counts visibly as
  original strings with `invalid-count` status.
- [x] Add intentional header-mode and row-width warnings, unclosed/invalid quote
  errors and exact repairs. Preserve native severity and coordinates: header
  metadata warnings have no invented location; row-width warnings have a line
  and byte offset without an invented column. Syntax failures stop publication;
  Studio retains the last successful report until repair and rebuilding.
- [x] Publish deterministic HTML and typed JSON reports with explicit HTML entry,
  captured input identities and template source-map origins. Keep source field
  offsets distinct from template origins and avoid promising field-level lineage
  for every generated character. Verify independent saved copies and unchanged
  read-only originals.
- [x] Preserve explicit `validate --config` commands through ordinary project
  generation and Studio's existing non-UI command service. Make header policy an
  editable run-configuration resource, independently of its own source validation.
  Preserve native input/package identities over project metadata defaults.
- [x] Fix native invocation binding for relative run-configuration input and
  schema-package addresses. Resolve runtime addresses against the invocation
  working directory before resource requirements/admission, retaining declared
  addresses, effective identities, authored configuration bytes and scope policy.
  Explicit external addresses remain explicit read requirements; Studio's exact
  admitted resource map continues to reject undeclared inputs.
- [x] Repair shared CSV string imports exposed by the optional table preview.
  `data:read` and `data:parse` now use the same owned parser selection as byte
  imports. Missing optional sources produce refusals without trapping; admitted
  original contracts preserve header/quote behavior, source locations and
  diagnostics. Pending contracts cannot fall back to an unrelated built-in.
  Keep the separate 32 KiB/4096-value string limits and larger byte-import budget.
- [x] Add native and browser fixtures for configurations, artifacts, limits,
  malformed inputs, exact repairs, saved edits, provenance and offline execution.
  Add a dedicated `verify:csv-schema-assets` target with optional deployment
  inputs and its separate evidence output. Document native import/authoring
  contracts and project operation limits.

## Verification

The native invocation cohort passes all eight cases, including two new tests
covering virtual/filesystem roots, canonical input/package addresses, preserved
authored policy and explicit external requirements. The regression first failed
with bare project input addresses.

CSV string-import checks pass four default-profile and five external-profile
executions. The original external regression reproduced the missing-built-in
panic before the fix. Older string import, document-limit and import-boundary
cohorts pass nine cases; CEM-QL data import passes six and string-import parity
passes two further cases. One unchanged XML document-URI assertion is recorded
below, rather than weakened.

Fresh default CLI, rendering WASM, command WASM, optional source pack/WASM and
both Studio deployments build successfully. The optional build verifies that the
default deployment is unchanged. Native Feature Tour verification passes all
32 registered source examples, project contracts and all 12 CSV configurations,
including repeated exact artifacts and failure/repair. The checked documentation
map has 35 workflow mappings. Deployment verification passes 218 declared assets
and 315 output files; six catalog projects plus the starter/schema tour have
89 sample cache URLs. Studio typecheck, authored source/script lint, syntax and
Git whitespace checks pass. The native HTML fixture preserves emitted trailing
spaces through a narrowly scoped `.gitattributes` setting.

All 19 final Feature Tour browser cases pass with unchanged deadlines (57.25
seconds overall). They cover saved header configuration execution/repair,
canonical diagnostic URIs, null metadata positions, undeclared input denial,
quoted fields, explicit count rules, independent copies, native provenance,
failed imports without publications and immediate repair on the same validator.

The final deployed default navigation gate passes catalog/tree selection,
Create/Clone/Delete, guarded drafts, Supplier, Markdown and JSON/YAML workflows,
themes, forced colors and 1440/900/390/320-pixel layouts. All 14 CSV evidence flags
are true: shared previews, saved edits, header/row warnings, quote failure/repair,
retained successful artifacts, source coordinates, provenance and offline use.
The offline branch rereads exact repaired saved header-policy bytes, executes
that configuration offline, rebuilds the saved CSV and confirms unchanged originals.

The final optional CSV navigator passes the same CSV scenario and offline header
execution/rebuild with package state idle, four runtimes and one package demand.
Its separate report is `dist/reports/cem-studio/csv-inventory-schema-assets.json`.
A direct probe against the fresh optional Node/browser WASM also confirms both
string boundaries refuse missing sources without poisoning the heap, work after
owned activation, retain quote diagnostics and work after repair. The WASM pair
has identical SHA-256 `ab2dd73262f3be763b9744673fc1eab083d16d6d53eae78c2be0c64305acd104`.

All browser tests use isolated storage. Actual saved projects and drafts are not
cleared. Screenshots, Vitest attachments, binaries and generated reports remain
excluded from Git; the normal portable project source archive is checked in.

## Preserved policy and limits

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled; recovery budgets and
capture/admission policy are unchanged. No component styles or theme defaults
were added, and production parsing/evaluation remains native.

CSV syntax validation does not enforce the inventory's particular columns or
business rules. Header-mode/row-width warnings remain advisory under the standard
CSV package. Compact output preserves interchange values; aligned review output
is a presentation. Selected-record CSV export from the constructed graph model
remains unsupported; this project publishes HTML/JSON reports and separately
formats the complete original inventory as CSV.

The existing XML parity assertion
`uri_accessors_share_native_metadata_and_enforce_optional_node_arguments` expects
`data:document_uri(data:parse("<r/>", "xml"))` to be empty. The preserved pre-fix
optional WASM returns `data:parse/source`, matching the failing current result.
This XML accessor path is unchanged by the CSV fix; keep its original assertion
and repair it separately. The broad lint target's generated WASM-glue scope issue
also remains in the native/release follow-ups; authored Studio lint passes.

Continue with XML/XHTML workflows extending Supplier, the existing Markdown
article/site project, and HTML/CEM-ML semantic/component examples using shared
declarative controls and native processing.
