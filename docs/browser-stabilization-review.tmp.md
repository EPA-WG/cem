# Browser stabilization review

Status: pending lifecycle decision, 2026-09-21. This is a review proposal,
not a replacement for the accepted registration or CSS contracts.

## Completed fixture corrections

The CEMT data-table uses a `th` for its row-selection control. Its first `td`
contains the first data column. The gallery verifier and the table story now
follow that structure; inspector headings are selected from `thead`. The XSLT
sample still has a selection `td`, and its verifier keeps that distinction.
No viewer template or output changed.

Scoped-CSS, hex-grid and legacy parity stories now use `storybook/test`'s
`waitFor` to observe their expected rendered elements, text, styles and image
readiness, bounded by the existing 30-second story budget. Frame counts and the
legacy two-second cutoff no longer impose shorter startup budgets. These waits
do not treat a missing stylesheet as successful readiness.

The five affected story files passed all 29 cases together. The source-loaded
stock-warning timeout remains a separate investigation; this run does not prove
its cause.

Build, typecheck and lint pass (two existing lint warnings). The full gallery
verifier advanced past the standalone table checks, then timed out in
`local-storage.html`: sample `3e. JSON validation` had no `ul` matching the
expected `b : B` output. Its cause remains unclassified and has a separate TODO.
The focused gallery verifier passes data-table and cell-overrides in both modes
(two standalone pages and two source-loaded documents), including stock edits
and repair. The earlier stock-warning timeout did not reproduce in this run.

The broad parallel Storybook run also exposed two CEM-QL fixture failures:
`CemQlDataDocumentBoundary` reports a diagnostic while inserting a slot bucket
containing JavaScript records; `RustFirstEvaluationTable` expects no diagnostics
for `dom:children(cemml:parse(...))`, but receives two `cem.ql.type_error`
diagnostics. Both need contract review against native node inputs. They have a
separate TODO; do not change evaluator semantics just to preserve stale fixture
expectations. The local-storage story also fails its JSON hydration assertion.
The final full parallel run passes 179 of 182 stories, including every modified
story; only those three separate cases fail. The table's invalid-input alert now
uses the same ten-second interaction budget as its sorting and reset checks.

## Pending decision: declaration ownership across remounts

Two existing rules interact:

- [Registration](cem-element-design.md): a second declaration for a tag in the
  same logical scope is an error even when its identity matches. Inherited and
  browser registrations with matching identities reuse the original compiled
  declaration and constructor.
- [Stylesheet ownership](cem-ml-uid-and-scoped-css-design.md): styles install once
  beside the effective declaration; removing that declaration removes its styles.

`registerResolvedDeclaration` retains the original compiled declaration on reuse.
`installDeclarationStylesheets` always appends its managed styles to that
declaration's element, including when it is detached. Waiting longer cannot make
those styles participate in the document cascade.

### Reproduction and observed results

A minimal DOM-template probe, after awaiting declaration and render settlement:

1. Register `style-remount-probe` with
   `<style>span { color: rgb(1, 2, 3); }</style><span>Ready</span>`.
2. Mount its declaration and one produced instance. The span is `rgb(1, 2, 3)`.
3. Remove the declaration while keeping the instance. The span becomes black.
4. Mount a new, identical declaration in the same logical scope. Registration
   reports `cem-element.registry_same_scope_duplicate`; the original declaration
   retains one managed style, the replacement has none, and the span stays black.
5. Reattach the original declaration. The original color returns immediately.
6. Remove it again and register an identical declaration through a runtime with
   a fresh explicit scope. Registration succeeds without diagnostics, but the
   replacement has no managed styles and the span stays black: the browser
   constructor still refers to the original effective declaration.

The real `hex-grid.html` source-loaded gallery reproduces the same behavior in
one browser document. Mount its declaration and produced page, remove both,
then load it under a different outer page tag:

| Observation | First mount | Second mount |
| --- | --- | --- |
| Managed declaration stylesheets | 4 | 0 |
| `.hex-grid` display | `flex` | `block` |
| `.hex-link` clip path | hexagonal `polygon(...)` | `none` |
| Nested registration diagnostics | none | four same-scope duplicates |

The duplicate tags are `cem-hex-image-link`, `cem-hex-grid`,
`cem-themed-framework-grid`, and `cem-image-button-grid`. Their output appears
because the document-global constructors survive removal, even though the new
declarations fail registration. The outer page tags differ deliberately so the
reproduction isolates nested declaration ownership.

### Recommended: support identical remounts with live declaration ownership

Allow an identical declaration to reconnect a retained registration after its
previous declaration element has disconnected. Keep one managed stylesheet set
per effective registration, attached to a connected compatible declaration.
Compatible aliases in other scopes participate in stylesheet ownership without
recompiling or redefining the browser tag. Remove the styles when the last live
owner disconnects; reattach them when a compatible owner returns.

Keep concurrent same-scope duplicates, incompatible identities, scope limits,
scope disposal and browser collisions fail-closed. A disconnected declaration
returning after replacement must join the already retained ownership rather than
create another stylesheet set. It must not reclaim a disposed processing scope.

Benefits: source-loaded views can unmount and remount normally; styling follows
live declarations; existing constructors and compiled artifacts remain reusable.
Cost: this adds a specific exception to the accepted same-scope duplicate rule
and requires explicit tracking of declaration ownership and disconnection. It
also needs a defined handoff for aliases and reconnected original elements.

### Alternative: retain the current lifetime contract

Require hosts to keep the original effective declarations connected for as long
as their browser registrations may be used. Update the shared gallery/Storybook
loader to retain and reuse those owners across story mounts.

Benefits: registration and stylesheet rules stay unchanged. Cost: mounts need
document-level declaration retention and deduplication; external hosts that remove
the original declaration still get unstyled reused instances, including through
a fresh compatible logical scope. Hiding failures with longer waits or unique
outer page tags does not solve nested ownership.

### Verification after the decision

- Add a permanent browser regression for repeated source-loaded gallery mounts.
- Cover the chosen behavior for a removed owner, a reconnected original,
  compatible aliases, simultaneous duplicates, incompatible replacements, and
  scope disposal. Assert stylesheet identity/count as well as computed styles.
- Run the affected stories under the full parallel Storybook suite.
- Investigate the separate stock-warning timeout and authored-source preview
  contamination in real source-loaded `cem-demo-element` cards.

The implementation is paused here under the user's stop-at-decisions instruction.
The runtime and base viewers are unchanged by this stabilization increment.
