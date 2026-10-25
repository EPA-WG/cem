# CEM-QL and XPath data projects, 2026-10-25

- [x] Add ordinary [CEM-QL order queries](../../examples/projects/cemql-orders/README.md)
  and [XPath catalog queries](../../examples/projects/xpath-catalog/README.md)
  under Queries and transformations, reusing Data discovery where appropriate.
  Their 30 authored resources and 21 configurations use native execution.
- [x] Demonstrate standalone CEM-QL expression/module source operations, typed
  filtering and joins, original imported-node ranges, declared parameter maps
  and JSON table artifacts. Keep query and template parameter APIs distinct.
- [x] Demonstrate XPath namespace-aware filtering, supplier joins, data-derived
  parameter maps, typed map/array inspection and original XML node ranges.
  Document unsupported source commands instead of presenting generic CEM as XPath.
- [x] Add exact native source-format and result fixtures, original two-error
  failures/repairs, malformed-input refusal and independent-copy acceptance.
- [x] Add failing native common-command provenance cases before binding published
  text-query maps to the original query buffer. Preserve fine ranges and raw
  query exports; never attach text ranges to compiled artifact bytes.
- [x] Add failing Studio query-identity cases before inheriting unambiguous native
  stack identities. Preserve frame kind, ranges, indices, bounded rows, source
  ownership, revision, draft and UTF-8 guards; retain legacy source-operation use.
- [x] Regenerate ordinary portable archives and update the checked content-type
  workflow map and native-operation limits. Introduce no sample-specific UI JS,
  browser query implementation or serialized AST execution boundary.
- [x] Verify both deployed projects through shared controls, including catalog
  discovery, saved data/query/configuration edits, native repair, original source
  navigation, independent copies and offline runs without clearing user storage.

## Data, parameters and source contracts

The CEM-QL project has 17 resources and 14 configurations. One owned JSON source
contains customers and orders. Its input-only expression selects three ready
orders and constructs typed records; its local query module joins matching
customers and returns two records. Expected items retain native record/atomic
types rather than flattening them into application DTOs. The unmatched ready
order demonstrates a query rule, not a source-validation failure.

The native-node query returns the original three JSON order objects and their
source ranges. Runtime-local pointer identities are explicitly excluded from
goldens and portable identity claims. URI and original byte ranges identify the
source; computed records do not gain arbitrary input character lineage.

Two graph configurations bind declared CEMT `status`/`minimum` parameters. The
template constructs a native CEM-QL options record and performs bounded status,
minimum and customer filtering. Existing graph/CEMT APIs publish ordinary JSON
arrays as explicit entry artifacts; no query CLI external-parameter option or
new codec is implied. Ready/minimum 30 produces two rows; draft/minimum 0 one.
Changing the ready minimum to 45 selects one row independently of the query.

CLI JSON fixtures are exactly 191 and 97 bytes. These exports currently have an
abstract JSON primary rather than native encoded writer bytes: CLI serialization
is indented and worker serialization compact. Cross-host checks compare the full
JSON value; unchanged builds within one host remain byte-stable. The JSON outputs
have no spans or `.map` sidecars, and that absence is asserted explicitly. The
source formatter's byte fixtures remain exact across execution surfaces.

The XPath project has 13 resources and seven configurations. It queries original
namespaced XML products and suppliers, filters stock, joins supplier names and
constructs/looks up native maps and arrays using an owned selection/options node.
Those options are data-derived parameters; external XPath CLI variable binding
is not claimed. Query namespace bindings work. Independently validated source
uses explicit EQNames because the standalone source loader currently omits caller
prefix bindings. Original selected XML nodes retain URI and byte/line/column
ranges. Computed XPath results retain query origins without invented data lineage.

Standalone XPath Parse/Inspect can currently interpret EQNames as generic CEM,
and same-schema source conversion refuses its target adapter. Those operations
are omitted from the advertised configurations and remain explicit native
follow-ups. The result inspector executes a real query returning typed map/array
items. CEM-QL expression/module source formatting and inspection cover the
supported source-authoring path without introducing a private parser.

Each project's failure resource produces two native parse diagnostics. Exact
repairs restore the independently valid query bytes; saved copies leave catalog
failure resources unchanged. Invalid queries cannot publish a successful query
result; invalid JSON cannot replace a successful parameter-report build.

## Query identity and navigation

The common publisher previously emitted URI-less query ranges, while Studio's
local-range fallback used the selected data file. Shared native publication now
adds an original query-buffer `SourceBoundary` to a clone of the source-map
reference, preserving native fine ranges and raw result/item exports.

The guard admits registered text query MIME identities only. Binary/compiled
artifacts, internal cache aliases and schema-only identities receive no fabricated
text boundary. Inline queries name their owned inline buffer; this does not grant
that buffer project-resource authority.

Studio resolves URI-less query frames only from explicit, unambiguous native
boundaries with the same source ID in the same stack. It never borrows identities
across diagnostics/references or guesses filenames. Collisions, invalid witnesses
and unresolved query identities refuse navigation before the active-input
fallback. Direct native identities retain their precedence. Legacy Parse/Inspect
and existing SourceBoundary/SCSS behavior remain intact.

This is service-data projection through the existing declarative controls. The
100-row limit, captured revisions/hashes, source ownership, draft transitions,
stale-result and UTF-8 selection bounds remain unchanged.

## Verification

The native cohort passes **16 tests**: 12 common execution cases and four
authored-project CLI cases. Two original missing-query-URI cases fail before the
publisher repair; the compiled-artifact guard is also covered. Named and inline
buffers, Unicode byte lengths, ready-name execution, old fine ranges and raw
result stability pass alongside existing publication/cancellation cases.

The authored CLI cohort runs every configuration and checks exact typed items,
source formatter bytes, XML/JSON selected-node ranges and URIs, original
diagnostics/repairs, parameter/data changes, independent copies, JSON mapping
limits and refusal before publication.

The Studio query-identity cohort passes **21 browser cases**, with 16 initial
failures and two additional invalid-witness failures observed before their
repairs. Nine SCSS compatibility cases and the provenance-filtered cohort also
pass, preserving the existing source-navigation paths and guards.

All **37 Feature Tour worker browser tests pass**, including all 21 new authored
configurations and both parameter-report graphs through the shared worker. Exact
typed values, source formatter bytes, selected-node URI/ranges, published query
boundaries, original repairs, independent reports and invalid-input refusal pass.
The cross-host JSON contract compares complete values while each host's repeated
output remains stable. Studio and shared-runtime typechecks and authored Studio
lint pass with the exact native-generated CLI schema input. That temporary input
is removed after verification; the existing standalone preparation and generated
WASM lint scope follow-ups remain separate.

Native Feature Tour verification passes all **32 schema examples** and the
authored configurations. The checked workflow inventory now contains **108
entries**. Output verification passes 218 declared assets and 324 files;
deterministic builds match SHA-256
`838c516cecf86ac4e847d73f977373259abb50665bce9064dcd3816728a9c5aa`.

The deployed `verify:data-queries` gate passes all **31 evidence flags**: 16 for
CEM-QL and 15 for XPath. Actual source-map selection opens the owned CEM-QL query
and selects its original UTF-8 range. Saved data/query/parameter edits, JSON table
previews, last-good build retention, invalid-query refusal and independent copies
pass. Offline reloads execute native queries and report builds from the persisted
sources, preserve repairs and leave the read-only catalog originals unchanged.
The gate reports no unexpected page errors.

## Preserved policies and next work

Acceptance uses isolated storage and temporary copies. Saved user projects and
drafts are not cleared; read-only catalog originals remain unchanged. Generated
screenshots, Vitest attachments, binaries, reports and temporary schema inputs
remain outside Git.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery stays disabled. Theme tokens, component
registration authority and native resolver grants are unchanged.

Continue with CEM/XSLT transformation projects, schema-authoring projects, then
module-map and portable configuration projects. Keep the documented native
XPath source-operation and abstract JSON byte-format follow-ups separate.
