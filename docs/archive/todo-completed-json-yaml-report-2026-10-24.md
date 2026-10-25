# Studio JSON and YAML report, 2026-10-24

- [x] Add JSON/YAML data workflows as one ordinary
  [JSON and YAML report project](../../examples/projects/json-yaml-report/README.md),
  shared by the Data and Queries and transformations folders. Its 17 authored
  source resources and 11 configurations cover saved API payload inspection,
  YAML settings, native source validation/formatting, standalone CEM-QL queries
  and a configured report. Generated portable command resources bring the
  archive to 28 resources. Originals remain read-only; saved clones are independent.
- [x] Build a retained native pipeline with explicit JSON/YAML import identities,
  a named settings input, typed model/view templates and HTML/JSON/YAML exports.
  Declare `generated/report.html` as the entry artifact. Reuse Studio's shared
  HTML artifact view, compact JSON table and source editor; YAML remains a text
  preview. No application-specific UI, parser, serializer or JavaScript capability
  was added.
- [x] Add native and Studio fixtures for all configurations, deterministic report
  bytes, source inspection offsets, formatting, queries, intentional syntax
  failures and exact repairs. Test saved input edits, independent copies,
  captured input identities, template provenance and offline rebuilding. Include
  generated project archives in browser-test cache inputs and verify all catalog
  archives/cache entries dynamically.
- [x] Diagnose and repair the malformed structured-data import panic shared by
  native transforms and graph builds. Failed imports now return native diagnostics
  without an artifact, rather than constructing a text fallback with JSON identity.
  Unsupported fallback representations also produce a diagnostic. Existing hard
  diagnostic/cancellation barriers prevent template execution and publication;
  valid imports retain lifecycle owners, identities and original bytes.
- [x] Add native malformed JSON/YAML/XML regressions for direct transforms and
  graph imports. Verify lossless JSON, retained XML, explicit projection boundaries,
  opaque JSON-labeled bytes and collection ownership in both native profiles.
  Browser commands assert exact parse codes, original input URIs/coordinates,
  empty publications and immediate repair on the same validator.
- [x] Document source syntax validation separately from application-schema instance
  checks. Describe inspection as the imported generic CEM projection with original
  source start offsets. Separate HTML template origins from captured graph data
  dependencies; do not promise field-level lineage for every rendered value.

## Verification

The final default CLI, rendering WASM, command-service WASM, project generation
and static Studio builds pass. Studio typecheck, authored `src`/`scripts` lint,
script syntax checks and whitespace checks pass.
The two byte-exact native JSON formatter/HTML exporter fixtures retain emitted
trailing spaces through narrowly scoped `.gitattributes` whitespace settings;
expected bytes and equality assertions are unchanged.

Seven focused native tests pass in each of the default and
`external-schema-assets` profiles. The two new guards cover malformed JSON,
YAML and XML in both transform entry points; five positive ownership/import
regressions remain green.

The native Feature Tour verifier passes all 32 registered source examples,
ordinary project contracts, all 11 new configurations, repeated exact report
bytes and native failure/repair builds. Its checked documentation map now has
25 workflow mappings. All 17 Feature Tour browser cases pass, including the
existing 15 cases and the two new source/report cases (46.78 seconds overall).

The exact-source deployed navigation verifier passes existing catalog selection,
Create/Clone/Delete, guarded drafts, Supplier, Markdown site, responsive layouts,
theme and forced-color checks. The new report checks read-only originals,
independent saved editing, compact JSON tables, sandboxed HTML, YAML title/filter
edits (two records to one and back), failed native builds retaining the last
successful artifacts, repair, template origins and captured inputs. Offline reload
and rebuild preserve artifact hashes, input identities and origins. Tests use an
isolated browser profile; actual saved projects/drafts are not cleared.

Deployment verification passes with 218 declared assets and 314 output files.
Five catalog projects plus the starter and schema tour have 88 sample cache URLs.
Screenshots, Vitest attachments, binaries and generated reports remain excluded
from Git; the normal checked-in portable project archive is included.

## Limits and next work

Entered passwords or one-time codes still block recovery until explicitly
cleared; automatic whole-Studio recovery remains disabled. This increment changes
neither credential capture nor recovery admission. No new theme overrides or
component-local presentation policy were introduced.

The broad Studio lint target also scans generated schema-asset WASM glue and
reports 238 errors and five warnings confined to `cem_ql.js` and
`cem_ql_factory.js`; authored source/script lint passes. A scope correction is
tracked after the functional samples. JSON source Parse's existing CEM tokenizer
diagnostics/projection identity also remain a native follow-up; this project uses
the clean, explicitly described imported-tree inspection path.

Continue with CSV inventory, header/quoting diagnostics and repair; extend Supplier
for XML/XHTML workflows; then reuse the Markdown site for article/site discovery
and source-to-artifact provenance. Keep these ordinary projects and use shared
controls and native processing throughout.
