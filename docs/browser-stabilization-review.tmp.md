# Browser stabilization review

Status: option 1 accepted and implemented, 2026-09-21.
The user selected identical remounts with live stylesheet ownership. The updated
[registration](cem-element-design.md) and
[CSS ownership](cem-ml-uid-and-scoped-css-design.md) contracts are normative;
the original reproduction and alternatives below remain review history.

## Completed fixture corrections

The CEMT data-table uses a `th` for its row-selection control. Its first `td`
contains the first data column. The gallery verifier and the table story now
follow that structure; inspector headings are selected from `thead`. The initial
stabilization kept XSLT's selection `td` as a separate fixture contract and
changed no viewer output. The user subsequently approved `th` for headings in
both versions on 2026-09-21; XSLT now follows CEMT and the verifier uses one
shared heading selector.

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

## Accepted decision: declaration ownership across remounts

Before this change, two rules interacted:

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

The runtime now records accepted owners separately from its retained processing
declaration. New same-scope registrations require proof that earlier owners
mounted and disconnected; a declaration awaiting its first mount still reserves
the binding. Weak owner references avoid retaining every removed gallery tree.
One document mutation observer handles custom-element and manually registered
owners, including queued connection evidence consumed during registration.
Scope disposal triggers immediate cleanup, and compatible aliases cannot revive
the original processing scope after its disposal. The base viewers are unchanged.
Registration identity also includes the resolved named CSS scope. The identity
fixture reproduced equal identities for differently scoped declarations before
this correction; remounts and aliases now reject that mismatch. Unnamed/private
declarations retain their previous identity encoding.


## Implementation verification

The pure registration cases first reproduced the rejected-remount failure, and
all three initial browser regressions failed before implementation. The completed
suite covers six lifecycle cases, including delayed source completion, batched
manual connection/removal, document adoption, CSS-scope incompatibility, original
owner reconnection and scope disposal. The source-loaded hex gallery mounts three
times with the same four stylesheet nodes, the same browser constructor, expected
computed styles and no declaration diagnostics.

Build, typecheck, all 410 runtime unit tests and lint pass (two existing lint
warnings). All 12 focused CSS/registration/lifecycle stories pass. The final full
parallel run passes 185 of 188 stories; the three CEM-QL/local-storage failures
listed above remain unchanged. The packaged hex gallery passes its standalone
and source-loaded interaction checks. No base viewer template changes were needed.

## Authored source previews

The real `cem-demo-element` capture reproduced tracking attributes in all three
source-loaded cell-overrides cards. `projection.ts` correctly stamps the nodes
in the source template; `cem-demo-element` then serialized those same attributes
into its displayed source. The highlighted text matched that contaminated source,
so the problem was capture, rather than highlighting or the CEM-ML importer.

DOM-derived previews now serialize an inert snapshot with the six reserved render
tracking attributes omitted, recursively including nested templates. Importing
into an inert document avoids constructing another set of custom elements just
to capture source. Input templates and live output retain their tracking metadata;
other attributes and literal text remain visible. String-valued source, the source
attribute and fetched source remain verbatim. This is DOM serialization, not a
new promise of byte-exact author-source recovery or reversal of upstream changes.
The precise presentation contract is in the
[demo element README](../packages/cem-demo-element/README.md).

The source-loaded preview fixture failed before the correction while the three
existing cell-override interaction stories passed. Coverage now compares every
real source-loaded cell card with its authored DOM serialization, checks retained
metadata in the source template, and checks the three live examples. A separate
demo-element fixture covers body capture, direct and explicit-slot templates,
node-valued source and verbatim strings/attributes, including nested templates.
The base viewers, runtime projection and external data import boundary are unchanged.

All 11 demo-element stories pass. The full runtime browser suite passes 186 of
189 stories, with the same local-storage and two CEM-QL failures listed above.
Packaged standalone/source-loaded checks confirm identical displayed source in
all three cards, retained template metadata, working live examples and no browser
errors. Both packages pass typecheck and lint; lint retains only the runtime's
two existing warnings. The demo-element build passes as part of its test target.
The intermittent stock startup finding and the three reproducible failures remain
open; clean source presentation does not resolve those separate issues.

## Accepted decision: native JSON storage write-back

Accepted 2026-09-21: preserve two-way binding with native CEM export. Implement
the recommended contract below; the alternative is review history.

The JSON validation failure is a render-contract mismatch, reproduced in both
standalone and source-loaded pages after declaration/render settlement:

- The storage key contains `{"a":1,"b":"B"}`. The slice contains the host record
  with fields `a` and `b`, so the read has completed.
- The sample inserts the entire record with `{$datadom.slices.json}` in its
  summary. Rendering reports `cem.ql.render.expression_type`:
  `Expression content requires native nodes or atomic values`.
- No replacement output is published. The initial `null` outputs remain and
  the `ul` never appears. Longer startup waits cannot repair this failure.
- The native structured-data fixture already requires rejection of direct
  record insertion. Selecting an explicit scalar field renders successfully.

At diagnosis, `localStorageStringToValue` used JavaScript `JSON.parse`, and its
inverse used `JSON.stringify`. Both JSON demos used JavaScript document records;
the basket's buttons parsed and mutated one. The
[import principle](cem-data-import-principle.md) recorded this older typed-value
protocol separately from document ingestion. It needed migration to satisfy the
user's instruction that JSON data stays in native CEM trees. Merely guarding the
record interpolation would repair the visible symptom while retaining that debt.

Native reads have a clear direction: retain the imported CEM document using
shared CEM-ML import and the processing-host lifecycle. Publish a document node
as the JSON slice, so CEM-QL/XPath use the same node APIs as loaded HTTP data.
Preserve source order and lexical values. Distinguish a valid JSON `null` node
from an absent key or invalid source; preserve invalid raw storage text and
publish diagnostics without a partial tree. Scalar storage types keep their
existing behavior. Remove JavaScript parsing/record mutation from samples and
their fixture adapters.

The remaining choice concerns the existing two-way slice contract. HTTP's
retained-document operation imports and releases documents but does not export
edited native slices back to storage. Portable native attribute values already
cross worker boundaries; native slice transport/write-back needs explicit wiring.
The existing CEM-ML generic-data JSON output pipeline is a useful export boundary,
but this audit does not establish a direct retained-CEM-node writer API.

### Recommended: preserve two-way binding with native CEM export

Keep `local-storage @type=json` readable and writable. Reads yield a retained
document; writes accept a native generic-data document/value node and serialize
it to JSON through a shared CEM-ML export capability. A host null/empty write
removes the key; a native JSON-null node writes `null`. Reject invalid or
nonrepresentable trees without changing storage. Construct new values through
CEMT/CEM-QL; never mutate imported input trees or decode them in JavaScript.

Retain unchanged input bytes through read-only refreshes. An explicit native
write uses the documented JSON exporter, preserving property order and valid
numeric lexical values rather than promising original whitespace. Revision
checks must prevent a stale asynchronous import/export from overwriting a newer
storage event. Release retained documents on replacement/disconnect/disposal and
cover worker fallback, reconnect and saved state with the native transport.

Benefit: preserves the advertised two-way behavior and supports native editing
without another representation. Cost: adds shared native slice transport and
JSON export wiring; requires lifecycle and write-failure coverage beyond a demo
fix. Consumer field selections and order assertions must migrate to CEM nodes.

### Alternative: native read view, writes through an explicit text slice

Make the JSON slice a read-only native document. Authors edit a companion
`local-storage @type=text` slice containing explicit JSON source; the JSON
reader imports subsequent storage events. No JavaScript object decoder is needed.

Benefit: smaller migration that reuses string write-back. Cost: removes the old
JSON slice's two-way write contract, forces native editors to choose an explicit
export path, and makes the basket less representative of typed editing. This is
a public contract choice, not an implementation shortcut to apply silently.

The user approved the two-way native binding. Its implementation retains the
CEM-ML import boundary and does not restore record insertion into CEMT content.

### Independent fixture corrections

The structured browser boundary now selects `datadom.slots.leading.text` for
metadata text and checks that direct slot-record insertion fails, matching the
native renderer fixture. The navigation parity rows call `dom:children` and
`dom:parent` on imported nodes and check actual names. Separate negative cases
require typed errors for the opaque identifiers produced by legacy `cemml:parse`.
No evaluator or renderer behavior changes are needed for these corrections.

The nearby `dom:descendants` and `dom:attribute` default evaluator branches still
return empty sequences. Their old rows do not establish native-node support;
the capability audit is now explicitly in TODO, separate from this fixture repair.

Verification: both updated native fixtures pass (`template_render`'s structured
data-document case and `retained_node_values`' navigation/cardinality case).
The full parallel browser suite now passes 188 of 189 stories; only the diagnosed
local-storage JSON sample remains failing. Typecheck and lint pass, with the same
two existing lint warnings. This change repairs fixtures and records the migration
decision; it does not change storage, rendering, evaluator semantics or base viewers.

### Native storage implementation

The accepted migration adds CEM-ML's `value::json::write_json` over the portable
native value graph. CEM-QL's host adapter delegates byte import and JSON export
to CEM-ML; it does not parse external formats. Stateless processing-host `value`
jobs use protocol v6 and replay on worker failure. Rendering validates native
slice bindings separately from attribute bindings and releases decode handles.

Native `slice-value` event attributes supply their retained value sequence.
The JSON basket constructs replacement generic-data nodes; no JavaScript JSON
object is created in either JSON sample. Imported slices contain documents;
authored replacement values can be document, element, reference or atomic
values accepted by the exporter. The basket selects the object from either
an imported document or an authored object. Existing base viewers are unchanged.

Storage jobs carry revision guards. Live reads invalidate pending exports;
exports also compare the current raw key before committing. Invalid reads keep
the raw key, clear the native slice and diagnose; invalid exports restore the
accepted slice. Key changes, disconnect and direct/ancestor scope disposal invalidate pending
work. Native artifacts persist in the explicit saved-state envelope, while
reconnect reimports the current storage source. Native JSON null remains
separate from an absent key; empty/atomic-null writes remove the key.

Authoritative JSON `@value` is validated before writing and restored after an
external write or slice edit. Unchanged reads do not reformat stored text.
Explicit exports use compact JSON, preserve order/duplicate keys and numeric
lexical values, escape strings, and reject nonrepresentable structures under
byte/value/depth ceilings. The same environment limits can only be lowered by
CEM scopes.

Verification on 2026-09-21: all seven native storage cases, 413 runtime unit
tests and 190 browser stories pass. Build, typecheck and lint pass with the two
existing lint warnings. Focused packaged gallery checks pass for standalone
and source-loaded storage pages; real demo cards show two desktop columns and
390px containment, with repeated basket edits persisted and no browser errors.
The complete packaged gallery also passes all 28 standalone pages and 34
source-loaded documents, including the previously failing JSON validation list.
The ancestor-scope fixture first reproduced a late import publication after
disposal; storage now observes every scope ancestor and removes all listeners.

The broader native run also exposed stale fixtures: the registry count predates
six native-value function signatures, table selectors predate row headings,
inspection rejects an oversized payload before allocating it, and node-pair
labels must compare text rather than serialized subtree markup. The old XSLT
output prerequisite now checks retained references and explicit `dom:text`.
Those fixtures now assert the existing contracts. Three strict `xslt_data_view`
comparisons initially failed because XSLT used selection `td` and CEMT used
`th`. The subsequently approved heading alignment changes XSLT to `th` and
removes its browser-fixture exception. All six focused native viewer tests now
pass, including sorting and imported aspects; DOM comparisons remain strict.

## Stock startup follow-up

On 2026-09-21, the remaining historical stock-warning failure was investigated
separately from the corrected storage and stylesheet-remount failures.

The original 2026-09-20 snapshot timed out on the second source-loaded card's
first warning-count check. Only Pokémon styles were listed. It did not record
whether the stock sample had mounted, its marker survived, the declaration
loaded or registered, or a diagnostic was attached to that declaration.
Consequently it cannot establish that the stock matching rule failed.

Twelve fresh-page probes using the gallery helper all passed. They released
the stock declaration immediately, after the first viewer became ready, or
after editing that viewer. Every card mounted with its authored template,
the stock fixture marker remained present, the declaration was requested,
and exactly one warning appeared without runtime diagnostics or browser errors.

The permanent `DelayedStockDeclaration` story uses the real demo cards and
separate registrations, withholding the stock declaration while the Pokémon
sample renders and changes. It verifies that stock registration/styles are
absent while loading, then checks declaration/render settlement, the single
warning, stylesheet presence, preserved source and a subsequent stock edit.
It passes with all 191 runtime browser stories. Typecheck and lint pass, with
the same two existing lint warnings.
The full packaged gallery also passes all 28 standalone pages and 34
source-loaded documents on this revision.

Gallery failure snapshots now report each card's legend, marker, mount/template
state and warning count, plus each declaration's source, browser registration,
stylesheets, instances and diagnostics. The source-loaded harness retains its
own runtime for read-only diagnostic access. An injected 503 for `stock-cell.cemt`
reproduces the missing-warning/missing-styles pattern; the new snapshot identifies
`cem-element.src_load_failed`, the unregistered tag and the still-mounted card.
This validates the evidence capture, not the cause of the original failure.

The historical timeout remains open pending reproduction with this evidence.
No stock predicate, base viewer, runtime behavior or startup timeout was changed.
The independent native DOM-helper audit can proceed to its explicit
[name-selection decision](dom-helper-native-review.tmp.md).
