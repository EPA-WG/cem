# XML and XHTML Studio workflows, 2026-10-24

- [x] Extend the ordinary [Supplier pipeline](../../examples/projects/supplier-pipeline/README.md)
  to version 1.1.0 with XML source validation, expanded-name XPath selection and
  intentional malformed-source repair. Reuse its existing namespaced catalog,
  reconciliation templates and HTML/JSON/YAML graph. Preserve resource and
  configuration IDs; add an explicit HTML entry artifact.
- [x] Add the ordinary [XHTML document project](../../examples/projects/xhtml-document/README.md)
  to Documents and UI and Queries and transformations. Its five configurations
  inspect retained structure, validate source, query the document outline,
  repair its missing namespace and generate an HTML article through CEMT.
  Keep its source/query/template/configuration resources editable in the shared
  workspace and cross-reference Supplier instead of duplicating its pipeline.
- [x] Repair native XHTML import before enabling the example's query/build.
  Resolve explicit XHTML before generic `+xml` handling, retain its original
  lifecycle wrapper and reuse XML semantic import into the common typed tree.
  Preserve expanded names, decoded text, CDATA, foreign namespaces, source
  coordinates and source ownership. Refuse hard XML/XHTML facts, DTDs and
  unresolved entities under the existing byte/event/depth limits.
- [x] Preserve explicit query namespace context through native command lowering,
  inline/query-file resources and deferred source-read requirements. Retain both
  named and default namespaces while keeping the query language, schema and base
  URI independent of its data identity. Studio's non-UI command binding preserves
  this native query identity alongside existing input/package identities.
  Cover stale project metadata without substituting URI-qualified query text.
- [x] Add native fixtures for all authored configurations, original diagnostic
  locations, exact repairs, namespace-prefix independence, foreign-name exclusion,
  graph outputs, provenance and independent edited copies. Add worker/browser and
  deployed catalog/clone/edit/build/preview/offline coverage. Actual saved
  projects and drafts are not cleared; browser storage is isolated.
- [x] Update the checked content-type inventory, project guides, shared import
  contract and native/XHTML package documentation. Regenerate normal portable
  sample archives. Keep generated screenshots, diagnostic attachments, binaries
  and reports excluded from Git.

## Shared implementation and source contracts

`cem-ml` owns external parsing and semantic import. Neither project introduces
an application parser, serialized AST handoff or sample-specific UI JavaScript.
Studio uses its existing declarative editor, tree, validation, project and media
preview controls. The small application service change binds native command
resource identities; it does not implement UI behavior or add a browser
capability. No theme tokens or component styles change.

The original XHTML gap was reproducible through retained lifecycle import and
through bytes: lifecycle import had no registered mapping, while bytes fell
through generic XML and lost XHTML's distinct source contract. Four regressions
failed before the fix. The common import now retains the actual XHTML owner;
disposing the last tree releases it. Source inspection receives expanded names
and original positions rather than the former generic fallback projection.

The worker namespace failure exposed two independent context losses. Native
lowering created a query identity without the supplied namespace bindings;
Studio then replaced dependency identities with project metadata. The new native
test first failed with an empty namespace map. Lowering now carries the normalized
CLI context and source-read expectations; Studio preserves the explicit query
identity. Direct portable query consumers keep their own static context instead
of implicitly inheriting a data document's lexical namespace declarations.

Supplier's malformed end tag reports two `cem.xml.parse_error` diagnostics;
the first is line 6, column 49, UTF-8 byte offset 315. Repair
`</product-broken>` to `</product>`. XHTML's missing root namespace reports
`cem.xhtml.namespace_missing` at line 2, column 1, byte offset 39; adding the
XHTML default namespace repairs it. Diagnostics retain the original project
resource URI in native and browser runs.

Generated HTML carries captured input identities and template source-map
origins. This does not establish arbitrary character-by-character lineage to
input fields. Supplier's existing semantic/trimmed fixture comparisons remain;
repeated emitted bytes are checked against the first build. Other projects
retain byte-exact fixture checks. A narrow `.gitattributes` rule preserves the
native writer's trailing spaces in the new XHTML HTML fixture.

The deployed helper checks generated HTML while the project is selected and
opens source files explicitly for editing/reloading. Its initial assumption
that selecting Supplier's catalog would infer a multi-input project output was
incorrect. File-preview inference and product navigation behavior are unchanged;
the corrected helper preserves its content, provenance and retained-output
assertions.

## Verification

The selected native suites pass **31 tests**:

```sh
cargo test -p cem-ml --test xhtml_import --test document_import_limits \
  --test import_strings --test cem_import_boundary
cargo test -p cem-ml-cli --test supplier_pipeline --test xhtml_document
cargo test -p cem-ml --lib command_invocation::tests::
```

The first cohort has 13 tests, Supplier/XHTML CLI workflows have nine, and
command invocation has nine. Additional compact, pretty and tabular XHTML
round trips preserve the original source bytes exactly.

Fresh native CLI, shared engine, rendering and command-service WASM packages
and Studio build successfully. Native Feature Tour verification passes all 32
manifest-selected schema examples and the authored project configurations. The
checked inventory has 43 workflow mappings. Seven catalog projects plus the
starter are delivered with 90 sample cache URLs. Deployment verification passes
218 declared assets and 316 files. Deterministic builds match SHA-256
`cd296d0a762a2eb235e2e4b992fc1a2c67519eab7132f0f312c16d79b83e8812`.

All **21 Feature Tour browser cases pass**, including the original cases and
new XML/XHTML source, worker namespace, repair, HTML output, provenance,
independence and malformed-import checks. The final run takes 51.15 seconds;
test assertions and deadlines are preserved. Authored Studio source/script lint,
typecheck, script syntax and whitespace checks pass.

The dedicated `@epa-wg/cem-studio:verify:xml-xhtml` gate passes both deployed
workflows. Its separate report is
`dist/reports/cem-studio/xml-xhtml-projects.json`. Each project records catalog
discovery, read-only originals, an independent clone, namespace queries, native
diagnostic coordinates/repair, saved edits, HTML preview, retained artifacts
after failure, input identities, template provenance and offline execution.
Reopening originals confirms their source and intentional failures are unchanged.

The complete `@epa-wg/cem-studio:verify:navigation` rerun also passes, including
both XML/XHTML workflows in the same session as Markdown, JSON/YAML and CSV.
It verifies tree/catalog and URL selection, Create/Clone/Delete, guarded drafts,
theme retention, forced colors and 1440/900/390/320-pixel layouts. Both new
projects record all eleven evidence flags as true in
`dist/reports/cem-studio/navigation.json`.

## Preserved policies and limits

XML syntax validation is separate from supplier domain rules. XHTML source
validation covers the documented XML facts, root/namespace and ordered
head/body checks; complete HTML vocabulary/content-model validation, arbitrary
XSD/RELAX NG instance validation and foreign SVG semantic validation are not
provided by this example. XHTML formatter profiles retain lexical layout.
The XHTML-to-HTML transformation is explicitly authored, not an automatic
cross-schema converter.

Entered passwords and one-time codes still block recovery until the user clears
them. Automatic whole-Studio recovery remains disabled. The default complete
deployment remains the tested delivery profile; optional package admission,
recovery budgets and checkpoint policies are unchanged.

Continue with the existing Markdown article/site workflow and its provenance,
HTML/CEM-ML semantic and component examples, then SVG/MathML inspection and
preview projects, as ordered in the active TODO.
