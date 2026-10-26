# Typed XPath Parse, Inspect and source export

- [x] Reproduce generic CEM fallback with native source-operation fixtures.
- [x] Route Parse and Inspect through the original XPath syntax and tokens.
- [x] Connect native borrowed owners to registered formatter/colorizer assets.
- [x] Verify exact source, diagnostics, identity and output preservation in the CLI.
- [x] Declare shared projection fields and verify monotonic events with retained native indexes.
- [x] Verify rebuilt WASM, ordinary Studio configurations, saved copies and offline runs.

## Native source contract

The lifecycle already owned a parsed `XPathExpressionAst`, but source Parse and
Inspect bypassed it. The original EQName catalog source produced CEM
`unknown_html_element` diagnostics and an unrelated CEM AST. Two new native
tests reproduced both the successful-source fallback and invalid-source
publication before the routing repair.

Source inspection now consumes the lifecycle-owned XPath AST. Its typed CEM
presentation has an expression root, the parser's existing syntax-node parent
links, and a separate ordered lexical-token view. Events retain the original
lexical lifecycle and syntax start/end events in separate domains, including
native token indexes and depth. Offsets come from the original token byte
ranges. Presentation maps retain the native attachment source ID and add the
explicit original URI boundary; no source string is reparsed or AST encoded
through JSON. JSON remains an explicitly requested structural export. A source
expression has no document DOM, so DOM projection requests refuse.
The shared AST/events schemas declare the emitted native fields. Events use one
monotonic sequence across both domains, retaining original domain indexes in
`native-sequence`. Tests compare every emitted element/attribute and allowed
value against the compiled shared schema declarations.

The XPath adapter also recognizes its registered source target identity.
Same-identity source export uses `XPathExpressionCemtSubjectRef`, contextual
borrowed token views and the existing registered `xpath.format-expression` and
`xpath.color-expression` assets. Native source strings, attachment and static
context remain on the original owner. Token maps use the existing native range
helper and attachment source ID. The writer receives text syntax rather than
an opaque target; XPath source and XPath result identities remain distinct.
Unrelated targets refuse instead of entering generic CEM conversion.

The `compact`, `pretty` and `tabular` formatter profiles preserve lexical
spelling, Unicode, comments, escapes and internal whitespace. They do not reflow
XPath grammar. Plain/terminal output uses the shared final-newline policy,
including CRLF. An already terminated source is unchanged. HTML coloring is an
escaped semantic span fragment with the displayed source retained verbatim,
without an added `pre` wrapper or EOF newline. Its primary and writer metadata
identify HTML MIME/schema, while plain/terminal output retains XPath identity.

Hard native syntax, UTF-8, static namespace and resource-policy diagnostics
prevent successful replacement output. Source operations perform no evaluation
or resource fetch and grant no additional capabilities. Caller namespace
bindings for standalone source loading remain the next separate TODO; the
sample uses EQNames and unbound prefixes still diagnose. The known cross-query
SourceBoundary consistency failure also stays open with its original assertion.

## Shared Studio delivery

The ordinary XPath catalog project adds `parse-filter`, `parse-filter-events`,
`inspect-filter`, `format-filter` and `color-filter` configurations over the
existing source resource. They use the shared command transport and declarative
Studio controls. No application UI JavaScript or new component behavior is
introduced. Existing query results, namespace joins, original native-node
provenance and intentional failure/repair remain part of the same project.

Active capability documentation now describes the new source operations and
retains the namespace-binding limitation. Two stale CSS-selector export claims
were also corrected to reflect the preceding completed native export work;
typed selector Parse/Inspect remain separate limits.

## Verification

- Native XPath-focused cohort: **117 passed**, including native parser,
  evaluator, lifecycle, XSLT integration, borrowed output and source projections.
- Real CLI source-operation cohort: **6 passed**, including default CEM-ML and
  explicit JSON presentations, original ranges, exact Unicode/CRLF output,
  semantic HTML/ANSI, hard source errors and unchanged existing output files.
- External data import boundary: **2 passed**.
- Shared WASM command-service browser cases: **2 passed**, covering the XPath
  project's native source operations and existing CEM-QL workflows.
- Credential refusal: passed with worker and fallback runtimes.
- Full Studio build and 21 dependencies, Studio type checking, 23 catalog
  contract tests, catalog audit and source-only README preview policy pass.
  The audit records 210 authored configurations and 215 documented workflows
  across 22 authored projects and 32 registered native packages.
- Native catalog: all **247 configuration receipts** match their declared
  contracts: 182 clean commands, 30 verified outputs, 33 intentional diagnostic
  cases and two documented Parse limits. Existing companion diagnostic limits
  remain qualified; this is not a uniform source-conformance claim.
- Deployed CEM-QL and XPath project acceptance: passed native contracts, source
  navigation, exact repair, original ranges, JSON tables, persisted data/query
  edits, parameter maps, invalid-query refusal, independent copies and offline
  execution. XPath also passes exact source formatting, escaped semantic HTML
  and native Parse/Inspect on the edited offline copy. Read-only originals remain
  unchanged.

The browser deployment uses catalog identity
`8cae0cccefe89b8214f624159986567ec6aa07a3c63e8d2b5283263ba1fc3c44`
and deployed SHA-256
`7e9aaebc6bbfe902323403a3afe03b70323e34801304d5e29aa78b2e3d4621e7`.
The catalog and deployed bytes remained unchanged throughout browser acceptance.

All browser checks use isolated profiles and temporary projects. Existing user
saved projects and drafts are preserved. Entered passwords and one-time codes
continue to block recovery; automatic whole-Studio recovery stays disabled.
Generated screenshots, Vitest attachments, native/WASM binaries and reports are
excluded from Git.
