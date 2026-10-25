# Studio content-type and schema inventory, 2026-10-24

- [x] Audit native manifests, the emitted registry, generic retained-document
  import, native query/source-inspection boundaries and browser capabilities.
  Record all 32 package identities, package/schema versions, 69 content-type
  claims (63 distinct selectors) and 13 browser operation availability records
  in [the inventory](../studio-content-types.md).
- [x] Separate source import, source validation, instance validation,
  formatting/inspection and query/transformation use. Document registered alias
  limits, UTF-8 plain text outside the package catalog, JSON Schema/RELAX NG
  source-only validation, Markdown's generated-HTML Parse view and the optional
  SCSS/XSLT operation restrictions.
- [x] Map all registered families to their existing generated Schema examples
  resource/configuration identities. Map 17 richer workflows to actual ordinary
  Login, Supplier, Markdown site and Schema examples project resources and
  configurations; identify missing functional projects in all four planned
  discovery categories. Keep metadata distinct from execution authority.
- [x] Verify inventory identities, versions, primary/other selectors and browser
  availability against native-generated JSON DTOs in the existing Feature Tour
  gate. Check documented workflow references against actual project bundles,
  plus all 32 stable validation resource/configuration mappings. Compare MIME
  essence for membership while preserving authored parameters in native inputs.
  Include the new source inputs in the Nx verification cache contract.
- [x] Cross-reference the audit from Studio and project authoring documentation.
  Remove the obsolete hardcoded 31-example claim. Preserve declarative UI/shared
  components, independent clones/drafts, trusted package demand, credential
  refusal and disabled automatic whole-Studio recovery.

## Verification

The native emitter's `--check-registry` check passes against the committed
registry. `@epa-wg/cem-studio:verify:feature-tour` passes generator drift, all 32
native source validations, project bundle checks, Supplier/site commands and
artifact/provenance assertions, plus the new inventory/reference checks.
The report records 32 packages, 69 claims, 63 distinct selectors, 13 browser
operations and 17 documented existing workflows.

All 15 `@epa-wg/cem-studio:test:feature-tour` browser tests pass. JavaScript syntax,
changed-script lint, authored Studio source/script lint and Git whitespace checks
pass. These checks reuse the current complete deployment and native outputs;
there is no native or runtime implementation change requiring a WASM rebuild.

The package-wide lint target also scans ignored generated schema-assets WASM
bindings and reports 238 errors and five warnings in `cem_ql.js` and
`cem_ql_factory.js`. Authored source/script lint passes separately; generated
bindings are not edited to satisfy authored-code rules. The first normal gallery
check found an extra optional-profile generated graph; regenerating the ignored
Feature Tour inputs restored its deterministic file set before the passing run.

Generated screenshots, Vitest attachments, native outputs and reports remain
excluded from Git. Tests use isolated browser/storage contexts; saved projects
and drafts are not cleared.

## Next work

Add the Content types and schemas catalog folder and category folders through the
application data model and shared trees/listings. Then implement the JSON/YAML
payload/configuration/report workflows and CSV inventory/quoting/failure-repair
project. Reuse existing authored projects for richer workflows instead of
creating dedicated sample pages.
