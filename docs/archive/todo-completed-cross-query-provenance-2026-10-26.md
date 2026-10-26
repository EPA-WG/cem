# Cross-query original-source provenance — completed 2026-10-26

The original `query_fixture_runs_all_languages_with_the_same_native_nodes_and_report_shape`
assertion reproduced on `cfacfcd2`: CSS-selector returned a ContentTypeTransform
frame while CEM-QL and XPath also retained the original SourceBoundary URI. The
assertion remains unchanged. The complete selector-package verifier now passes.

## Shared implementation

- [x] Add retained-owner/import regressions before changing the implementation.
  HTML and XML cases cover original byte ranges/URIs, Unicode, CRLF, decoded
  attribute values, separate input/query maps and retention of the exact native
  lifecycle owner.
- [x] Move CSS-selector document access into the shared native import boundary.
  Its literal element-tree profile retains inert CEM payloads and namespace
  declarations without evaluating them. Navigation reads typed CEM names and
  values; source event handles and order remain compatibility metadata. The
  import-owned attribute-name policy preserves HTML case-insensitivity and XML
  case-sensitivity without using event handles to interpret attributes.
- [x] Preserve HTML/XML-family admission and original outer owners for XHTML,
  SVG, MathML, XSLT and XML Relax NG. Reject compact Relax NG, non-element input,
  malformed/partial documents, unsafe authored HTML and shared byte/node/depth
  limits. Retention grants no resource access or execution.
- [x] Put original-URI attachment on `RetainedCemTree` for all three native query
  consumers. Semantic nodes keep merged text-run frames; CEM-QL source nodes
  keep their individual source-arena ranges. Existing boundaries take precedence;
  empty maps and invalid IDs never acquire invented source facts.
- [x] Extend the import-boundary architectural guard to CSS-selector input and
  evaluation. No browser document records, parser-AST navigation or exporter
  frame synthesis is introduced.
- [x] Correct an obsolete one-frame XPath text-run expectation to require both
  the complete original multi-range frame and its URI boundary. All byte, line,
  column and covering-range assertions remain in place.

## Verification

- [x] Native CEM import/view suites: 65 tests passed across selectors, import
  guards, source provenance, document limits, string imports, XHTML, SVG/MathML
  and XPath CEM/XML views.
- [x] CEM-QL data import, data-tree and source-provenance suites: 13 tests passed.
- [x] CLI query cohort: all 20 tests passed, including the unchanged original
  equality assertion and new HTML/XML Unicode/CRLF equality checks across all
  three languages.
- [x] Complete `cem_ml_schema_package_css_selector_v1:verify` gate passed,
  including package validation, native source/export/conversion tests, CLI
  examples and the complete query cohort.
- [x] Full Studio build and its 21 dependencies passed, including both WASM
  runtimes and the 59-component declarative architecture check. Studio type
  checking, catalog audit and 23 catalog acceptance contract tests passed.
- [x] Four focused Studio browser-worker cases passed: ordinary CSS-selector,
  CEM-QL and XPath source/configuration/repair operations, plus exact Login
  input-map equality across CSS-selector, CEM-QL and XPath.
- [x] Credential refusal passed on worker and fallback runtimes. Entered
  passwords and one-time codes continue to block recovery; automatic
  whole-Studio recovery stays disabled.
- [x] Deployed CSS/SCSS/selector navigation, saved copies and offline acceptance
  passed. Exact input URI/range assertions survive document/query edits, invalid
  query refusal and repair, independent copies and offline reloads. Catalog
  originals remain read-only and unchanged.

The verified catalog identity is
`e4438dbded38d65016ddbd4ab0265d271fdffdf34b5a7079c0b35cbfa190fce2`;
the frozen deployment SHA-256 is
`27a3feb098bbdd71fec989e7d17b85b14a98a146b9ff4377c09d6bc822abcc46`.
Catalog definitions remain at 22 authored projects, 213 configurations and
218 documented workflows across 32 registered native packages.

Tests use temporary project copies and isolated browser profiles. User projects
and drafts are preserved. Screenshots, Vitest attachments, reports, binaries and
WASM output remain outside Git. The shared native import contract is documented
in [the data import principle](../cem-data-import-principle.md).
