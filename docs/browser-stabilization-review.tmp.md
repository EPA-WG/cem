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

## Startup follow-up decision: continue investigation

The user selected continued investigation on 2026-09-21. The monitoring option
below was not accepted; startup remains active. The subsequent fault-injection
findings are recorded in the next section.

Follow-up audit on 2026-09-21, starting at `e16da2c1`:

- The full parallel Chromium Storybook run passes all 198 tests in 41 files,
  including the delayed-stock regression and the new implicit-module cases.
- A separate startup probe runs 24 fresh browser pages in six batches of four.
  Twelve use the gallery verifier's simplified card helper; twelve load the
  packaged `cem-demo-element`. Each imports the authored `cell-overrides.html`
  through `<cem-element src="…">` and uses the packaged CEM runtime.
- For each implementation, four runs release `stock-cell.cemt` immediately,
  four release it after the Pokémon table appears, and four release it after
  editing that table's source. These probes run separately from the full
  Storybook suite; they do not establish a combined stress result.
- All 24 runs request the stock declaration, retain all three cards and their
  templates/live regions, retain the stock fixture marker, and show exactly
  one stock warning. No browser errors or declaration/instance diagnostics
  occur. Helper connection traces always find the authored template present.

The helper differs from the real component, but this matrix does not reproduce
a mounting difference. Runtime materialization builds a new element's children
before connecting it. There is no evidence here for changing card mounting,
stock matching, request lifecycle, viewer output or timeout limits. The original
snapshot still lacks the information needed to attribute its failure to any of
these causes. Passing runs do not prove that the historical failure was fixed.

**Recommended:** finish the active startup stabilization item with its known
readiness fixes, and retain the historical stock timeout as a monitored finding.
Keep `DelayedStockDeclaration`, the full Storybook/gallery gates and the enriched
failure snapshots. Reopen active diagnosis if the warning check fails again;
use the captured mount, marker, registration, style, loading and runtime evidence
to select the failing layer. Proceed next to the page-and-legend Storybook
coverage inventory already listed in TODO.

**Alternative:** keep the historical timeout as the next active blocker and
continue fault-injection or load experiments. This may find another failure,
but no currently failing case identifies a product correction to implement.
It also leaves authored-sample coverage waiting behind an unconfirmed cause.

This follow-up does not claim a runtime fix or mark startup complete.

## Reproduced defect: failed declaration sources cannot recover

Investigation at `b299f7fd` identifies a deterministic recovery defect in the
shared browser source loader. It does not yet identify the cause of the original
single-run stock timeout.

### Minimal source-load reproduction

Use the packaged runtime with its default HTTP loader, an external HTML source
containing a CEM-ML template, and a server whose response can change from failing
to healthy without changing the source URL.

1. Register a declaration referencing `/fixture-source.html#card`. Fail the
   request with HTTP 503, or send HTTP 200 headers and interrupt the response
   body before it completes. Await `whenDeclarationSettled`.
2. Confirm `cem-element.src_load_failed` on the declaration and no produced
   browser tag. Restore a healthy response containing the selected template.
3. Remove and reattach the original declaration, explicitly call
   `registerDeclaration`, then register a new declaration element with a different
   produced tag but the same source reference. Await both attempts.
4. Finally register that source through a fresh runtime as a control.

Both transport failure cases give the same result:

| Attempt after the server recovers | New HTTP requests | Result |
| --- | ---: | --- |
| Original declaration registered again | 0 | No produced tag or output; original failure remains |
| New declaration using the same runtime and URL | 0 | Cached failure; no produced tag or output |
| Declaration using a fresh runtime | 1 | Template renders `Ready` without diagnostics |

The explicit registration call returns `true` for the original element but starts
no new work. This is not a request that is merely slow or awaiting retry.

### Reproduction with the authored stock demo

The source-loaded `cell-overrides.html` reproduces the same defect with both the
gallery's simplified helper and the real packaged `cem-demo-element`:

1. Return HTTP 503 for the stock source requests while allowing all other
   resources to load. All three cards mount, the Pokémon table renders, and
   the stock tag remains undefined with zero warnings and zero stylesheets.
2. Restore the stock source and remount the entire authored page under a new
   outer produced tag in the same runtime. All three cards still mount, but
   the stock tag remains undefined. There are **no additional stock requests**.
3. Reload the browser document with the healthy server. A new stock request
   succeeds, the tag registers, and exactly one warning appears.

The nested loader first tries the declaration-relative URL and then its existing
page-relative fallback. The failed diagnostic reports the last candidate. The
probe records both requests; this investigation does not change URL resolution.

### Cause and limit of attribution

In `packages/cem-elements/src/lib/cem-elements.ts`:

- `loadSrcDocumentParsed` stores the pending parse promise in `srcDocuments`.
  Its rejection is never removed. Every later declaration sharing the cache key
  receives the same rejected promise without invoking the loader.
- `registerDeclaration` puts an external declaration into
  `registeredDeclarationElements` before acquisition completes. A failed load
  leaves it there, and the early return prevents that element from trying again.

The original `/tmp/cem-cell-gallery-final.log` was recovered during this audit.
Its sample text describes an earlier cell-dispatch design and differs from the
first committed sample; the exact source/build identity was not recorded. It reports
no browser HTTP errors and lacks declaration diagnostics and request history.
The injected failures therefore demonstrate a current recovery
defect and a matching visible symptom, not proof that HTTP failure caused that
historical run. Existing passing load/remount cases do not exercise this path.

### Accepted contract: retry on explicit registration or reconnect

**Accepted 2026-09-21:** the user approved this behavior and requested renewed
startup investigation after implementation.

Permit a new attempt after failed source acquisition when a host
explicitly registers the declaration again, reconnects it, or mounts a new
declaration. Evict a rejected source-cache entry only if it still names that
failed attempt; release the failed element's registration-in-progress marker.
Concurrent consumers continue sharing a pending attempt, and successful source
documents remain cached. Keep the existing diagnostic history; successful recovery
is demonstrated by registration and rendered output, not by deleting past errors.

This adds no timer, background retry loop, new element attribute or data-format
handling. Every explicit retry must pass the existing scope and resolver checks.
Keep missing-fragment and successful-source invalidation policy separate: this
proposal addresses failures of acquisition/stream reading, not live source editing.

The alternative is to document that a failed declaration source requires a new
runtime or document. That preserves current retention behavior but prevents normal
gallery remounts from recovering after a transient source failure.

The tests-first plan adds focused browser regressions for HTTP and body-stream
failures, retries using original and replacement declarations, concurrent load
sharing, successful cache reuse, disposed scopes, and the real stock gallery.
Use a controlled response gate instead of sleep-based readiness. Run those cases,
the full Storybook suite and the independent gallery checks.

### Source-retry implementation

The browser host now removes a rejected source promise from its cache, guarded
by that promise's identity, and removes the failed declaration's registration
marker before reporting its diagnostic. This lets reconnects and explicit
registration retry through the existing scope and resolver checks. Pending
requests remain shared; successful documents and diagnostic history are retained.
The CEM import and transformation layers, URL candidate policy and base viewers
are unchanged.

Tests were added before the fix: all four declaration-source retry stories fail
on the old runtime, and the authored stock remount story times out after recovery.
With the fix, all twelve stories across the retry and cell-override files pass.
Coverage includes original and replacement owners, simultaneous consumers,
interrupted response streams, successful cache reuse, direct and ancestor scope
disposal, disposal during retry, and stock warning/style recovery followed by
source editing. This is browser-host lifecycle behavior, so the failing cases
exercise that boundary rather than adding unrelated Rust parser tests.

Final verification on 2026-09-21 passes all 203 browser stories in 42 files,
all 441 runtime unit tests in 52 files, and the complete gallery of 29 standalone
pages and 35 source-loaded documents. The package build, declaration type check
and lint pass; lint reports only the two pre-existing non-null-assertion warnings.

### Renewed startup investigation after the fix

The rebuilt packaged runtime was tested through its default HTTP loader, using
fresh Chromium documents for the gallery helper and the real `cem-demo-element`.
Both load the authored `cell-overrides.html` without changing viewer templates.
The server controls only the stock declaration response. Each case waits for
the Pokémon table and stock declaration owner before changing the load or mount
state; response gates replace timing guesses.

| Case, run with both card implementations | Observed result |
| --- | --- |
| Close the HTTP connection without a response; restore it and remount the page | Load failure is diagnosed; remount makes a new request and renders the warning |
| Send HTTP 200 and part of the body, then terminate before the advertised content length; restore it and remount | Body failure is diagnosed; one healthy request on remount restores output |
| Hold the stock response pending, remount the whole page, then release it | Both owners share one request; the connected replacement receives styles and output |
| Hold the response pending, reconnect and explicitly register the original owner, then release it | One request remains pending; registration and rendering finish normally |

All eight cases pass. Every final page retains three mounted cards and the stock
fixture marker, shows one warning and two stock declaration stylesheets, and has
no declaration/instance diagnostics on the recovered owner. Changing stock from
zero to seven removes the warning in all eight cases. No page exceptions occur.
The deliberately failed requests report `ERR_EMPTY_RESPONSE` or
`ERR_CONTENT_LENGTH_MISMATCH`. Chromium itself repeats some empty-response socket
attempts before the loader's fallback candidate; these server requests are not
a new runtime retry loop. Pending remount/reconnect cases make exactly one stock
request each.

A separate minimal default-loader probe also confirms HTTP 503 and interrupted
body recovery for the original and replacement declarations: both render `Ready`
after settlement, share one healthy request, and retain the original diagnostic
only on the failed owner. Awaiting declaration settlement alone does not imply
render settlement; the probe checks both before reading output.

The source audit also checked downstream static-import acquisition:
`ensureProcessingArtifact` already removes rejected compilation promises, and
`preflightCemMlTemplateModules` keeps its loaded-source map within an individual
attempt. There is no equivalent permanent rejected-source cache in that path.
This is a code audit, not proof that every import/worker startup interleaving is
correct.

Four further cases isolate the stock template's imported data-table module, with
both card implementations. A host resolver adds a query marker only when resolving
the stock template's import; the server can then fail or hold that request without
affecting the Pokémon viewer's import of the same file. Authored templates remain
unchanged. An HTTP 503 leaves the stock browser tag defined, installs no stock
styles, and reports `cem-element.processing_host_render_failed` on the instance.
Restoring the source and remounting produces a new request and recovers. Holding
the import through a page remount shares one pending request and renders after
release. All four cases recover one warning and two styles, pass the subsequent
stock edit, and show no scheduler fallback, overflow or cancellation.

Two final cases hold the stock compilation request at the worker transport
boundary, using the existing worker-factory injection point with the real worker.
The tag is defined, but no stock styles or warning exist while compilation is
held. Remounting the page and releasing the request recovers with both card
implementations. Worker request/response traces show replies to all three stock
compile requests observed across this remount sequence. The recovered instances
have no diagnostics, and the scheduler reports no fallback or overflow. This
checks delayed completion and recovery; it does not establish a compile-time
performance bound or a policy for a permanently unresponsive worker.

All fourteen startup cases pass. Source acquisition, imported-module acquisition
and pending worker compilation can each explain a temporarily absent warning;
registration and diagnostics distinguish their observed states. None establishes
which state caused the historical timeout, so startup remains active. The next
investigation should measure request, worker-queue, compile and render timing under
parallel load to identify any unexplained delay without increasing wait limits.
No new runtime behavior is selected from the old, incomplete snapshot.

## Startup timing under parallel load

The 2026-09-21 follow-up starts at `5ecbf60f`. The repeatable probe is
[`tools/scripts/diagnose-cem-stock-startup.mjs`](../tools/scripts/diagnose-cem-stock-startup.mjs).
It serves the authored gallery through the packaged runtime, alternating the
gallery helper and real demo cards in fresh browser contexts. It records:

- Git revision, changed file names, hashes of critical source/runtime files,
  browser/Node versions, CPU/kernel, concurrency, and run timestamps.
- Browser resource timing through response-body completion; the common viewer
  import entries include both Pokémon and stock consumers.
- Existing scheduler enqueue/dispatch events and worker construction/readiness.
- Stock compile/render request and response timestamps, outcomes, and diagnostics.
- First stock owner, stylesheet, table and warning observations, plus final card
  mount state, registration, styles and diagnostics.

The worker factory only observes the real worker's messages. Request-to-response
time includes transport, engine initialization and main-thread delivery; it is
**not** a pure native compile/render duration. Scheduler wait means enqueue to
dispatch; worker readiness and dispatch-to-send time are recorded separately.
Measurements overlap and must not be added as independent parts of startup.
The probe preserves the gallery's 45-second limit and does not alter template
sources, runtime policy or worker counts. Its JSON file is an explicit diagnostic
control report, not an external document or AST handoff.

Reproduce after building the packaged dependencies, sequentially:

```sh
yarn nx run cem-elements:build
yarn nx run @epa-wg/cem-demo-element:build
node tools/scripts/diagnose-cem-stock-startup.mjs --concurrency=1 --batches=2 --label=baseline --output=/tmp/cem-stock-baseline.json
node tools/scripts/diagnose-cem-stock-startup.mjs --concurrency=4 --batches=2 --label=four-pages --output=/tmp/cem-stock-four.json
node tools/scripts/diagnose-cem-stock-startup.mjs --concurrency=8 --batches=2 --label=eight-pages --output=/tmp/cem-stock-eight.json
```

For combined load, start `yarn nx run cem-elements:test`, wait until Vitest starts
running (its build prerequisites must have finished), then run the probe in a
second terminal with `--concurrency=8 --batches=4 --label=alongside-storybook` and
a separate output path. Retain both reports; compare their timestamps to confirm
overlap. Timing ranges below describe this machine/run, not a performance SLA.

The initial matrix used Chromium 148.0.7778.96, Node 24.16.0, WSL2 Linux
6.6.114.1 and an Intel Core Ultra 7 258V with eight available CPU threads:

| Load | Probes | Warning after mount | Max stock source | Max viewer import | Max stock queue | Max stock compile round trip | Max stock render round trip |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| One page | 2 | 1.36–1.45 s | 3 ms | 5 ms | 409 ms | 82 ms | 166 ms |
| Four pages | 8 | 2.12–2.48 s | 29 ms | 30 ms | 558 ms | 126 ms | 231 ms |
| Eight pages | 16 | 3.61–4.77 s | 59 ms | 204 ms | 1,347 ms | 368 ms | 493 ms |
| Eight pages alongside Storybook | 32 | 7.75–10.28 s | 186 ms | 291 ms | 2,347 ms | 585 ms | 1,073 ms |

All 58 stock probes pass. There are no failed source requests, runtime
diagnostics, scheduler overflows or fallback events. The combined probe runs
overlap the active Storybook tests. Worker readiness reaches 4.41 seconds in
that run. Initial stock compilation shares a worker with the native-value card's
initialization, accounting for observed queue waits. The second stock compile
request is the existing render preflight in `CemScopeProcessingHost.renderDiff`;
the engine's retained-artifact path returns it quickly. The trace does not show
repeated full compilation or an unexplained 45-second stall.

### Reproduced fixture readiness failures

The initial combined run passes 200 of 203 Storybook cases. Three assertions
expire before their expected output is present:

- `form-demo.stories.ts` / `EveryAuthoredSample` stops after 300 animation frames
  while waiting for its five anonymous form instances.
- `data-table-demo.stories.ts` / `MatchingPresentationAspects` uses the default
  one-second poll immediately after attaching its second instance.
- `data-table-demo.stories.ts` / `NativeXsltViewer` uses the same short poll after
  resetting invalid source, while the restored table is still pending.

The first fixture correction waits for the existing declaration/render lifecycle
before those assertions. All six cases in the two focused files pass, and the
form startup and second-instance checks pass in the next combined run. That
repeat exposes additional short polls at table format/IP-address updates and
an XPath-sort toggle, plus a 300-frame data-slices startup check. All 32 stock
probes still pass, with warning times reaching 11.54 seconds; the suite passes
199 of 203 cases in that intermediate run.

Replacing the data-slices frame cutoff with settlement reveals a deterministic
assertion error even without stress: **16 legends contain 18 rendered instances**.
The attribute-initialization sample and the emotion-attribute sample each author
two instances. The old equality-to-16 readiness condition could pass during a
partially rendered state or miss that transient count entirely. The corrected
test validates each legend's intended instance count after settlement, then
executes the existing interaction checks.

The final fixture changes share `whenCemSourceRendered` in the Storybook preview
to await the source host, its declarations and initial produced instances.
Forms and data-slices use that helper; table and XPath-sort transitions use the
existing `whenCemRendered` before output assertions. The eight cases across
those four files pass together. Story budgets, data/order/focus assertions,
runtime and viewer templates remain unchanged; the instance-count assertion now
matches the authored source. These are fixture corrections, not evidence of a
fixed historical stock timeout.

The synchronized repeat starts its probe process when Vitest announces `RUN`.
All 32 stock probes pass in 6.03–11.99 seconds. It exposes one more fixed-frame
cutoff in `InlineBrowserSubstrateContract`; that fixture now retains its runtime
and awaits declaration/render settlement before inspecting the button. The
source/runtime hashes recorded by all six timing reports match (the diagnostic
script itself gained per-run timestamps between the baseline and combined runs).

### Remaining stress boundary

Under the synchronized extra load, these complete interaction journeys exhaust
their existing 30-second story budgets:

- Multi-format Data Tables / `EveryAuthoredSample`;
- Retained Data Trees / `EditingSelectionAndDisclosure`;
- Table Inspector / `MultipleSelectionAndRecovery`.

Those failures are different from a one-second poll or an incorrect instance
count. The current results do not identify which part of each journey consumes
the budget, or establish that an individual operation never settles. No viewer
change, larger timeout, story split or concurrency cap is justified from this
snapshot alone. The [follow-up below](#long-story-phase-investigation) timestamps
setup and interaction/settlement phases in these three cases under the same
load before considering changes to test organization or execution policy.

The historical stock warning timeout remains open. Passing stock probes and
corrected fixture readiness do not prove that the original failure is fixed.

Final verification: the normal full Storybook suite passes all 203 tests in
42 files after the fixture corrections (62.42 seconds). Package lint passes with
its two existing warnings; the diagnostic script passes syntax and lint checks.
The earlier synchronized stress run remains recorded as 199/203 plus 32/32 stock
probes; it is not reported as a passing stress gate. Runtime/viewer source and
packaged runtime hashes were unchanged throughout this investigation.

## Long-story phase investigation

The follow-up at `3e83149d` adds opt-in observations to the existing table
`EveryAuthoredSample`, tree `EditingSelectionAndDisclosure` and inspector
`MultipleSelectionAndRecovery` stories. Enable them with:

```sh
STORYBOOK_CEM_STORY_TIMING=1 yarn nx run cem-elements:test
```

The shared `.storybook/story-timing.ts` helper emits labeled diagnostic control
records immediately. It uses warning output because this runner hides ordinary
console output for passing tests; these records are not runtime warnings. Each
record contains the story, phase, UTC timestamp and elapsed time since the
story's `play` function starts. Setup includes the existing source-page readiness
assertion; module import and work before `play` are outside that clock.

Action start/dispatch and successful DOM assertions are observed without changing
the actions or their waits. Where the story already awaits rendering (table sort
controls), the completion marker includes that wait and its assertion. Other
actions attach an **unawaited observer** to the existing `whenCemRendered`
lifecycle. That lifecycle follows the instance's latest render: a subsequent
action can extend an earlier observer's wait. Several observers completing
together therefore do not measure several independently slow render jobs. The
records identify visible progress and eventual settlement, not native evaluator
CPU time. Logging and observation also add some overhead.
The final helper also records the target tag and whether it is connected at
dispatch/settlement. A settlement after detachment is not evidence of a successful
live render. The runner can replay records when reporting a failed test; use
`(story, at, phase)` to deduplicate them.

The normal instrumented full suite passes all 203 cases in 42 files (60.73 s).
The same full suite with observation disabled passes 203/203 (51.68 s); these are
single runs with varying scheduling, not an isolated estimate of logging cost.

| Journey | Initial readiness | Complete play | Observed interaction cost |
| --- | ---: | ---: | --- |
| Table | 11.58 s | 24.66 s | Four formats sorted by 20.07 s; namespace edit verified at 21.08 s, invalid JSON at 22.99 s |
| Tree | 8.17 s | 26.31 s | First/second selections verified at 10.36/12.89 s; reset takes 5.21 s, reselect 2.53 s, replacement 2.31 s, final restore 3.22 s |
| Inspector | 1.77 s | 15.81 s | Ascending/descending orders verified at 6.30/7.96 s; source order at 11.10 s, recovery at 15.80 s |

The tree's close/open disclosure interactions each complete within 40 ms in that
run. Its two selections, reset and subsequent source transitions account for
most of the interaction time. The inspector starts after earlier stories in its
file have loaded the same source document; its setup cost is not a cold-start
comparison with the table and tree.

### Combined-load results and timeout cleanup

Both runs below start the existing eight-page/four-batch stock probe when the
full suite announces Vitest `RUN`, after its build prerequisites finish. The
first starts at `2026-09-22T05:05:32.538Z`; the repeat with connection-state
observations starts at `05:07:41.425Z`. They use the same machine described in
the startup timing report. The probe's critical source/runtime hashes still
match that earlier report; no viewer or runtime source was changed.

| Run | Storybook result | Stock probes | Stock warning time |
| --- | --- | --- | --- |
| Initial phase observations | 201/203, 82.36 s | 32/32 pass | 7.96–10.68 s |
| Connection-state repeat | 200/203, 83.41 s | 32/32 pass | 4.73–12.07 s |

The table and tree exhaust the full 30-second story budget in both runs. The
inspector passes the first and exhausts the same budget in the repeat. There
are no other failing stories in these runs.

- **Table:** initial readiness takes 24.01/26.11 seconds, leaving fewer than
  six/four seconds for the entire four-format journey. The first column change
  settles and passes its assertion at 29.48/29.68 seconds. That setup plus the
  first successful operation already accounts for almost the whole budget.
  The async play function continues after timeout; in the repeat it reaches the
  namespace edit at 54.05 seconds with `connected: false`. Its subsequent
  assertions and settlement must not be counted as successful live coverage.
- **Tree:** initial readiness takes 19.42/19.26 seconds. The first selection is
  verified at 22.85/24.58 seconds, and the second is dispatched at 22.93/24.65
  seconds. Its count-of-two assertion never reports success. The initial run
  reports late settlement at 34.76 seconds; the repeat reports it at 35.16
  seconds **after the viewer has detached**. This rules out treating that late
  signal as proof of recovery. The traces do not yet distinguish a long pending
  worker job from stale or missing live output for that selection.
- **Inspector:** the first run verifies every action and finishes at 27.58
  seconds. In the repeat, source-order restoration is verified at 25.55 seconds,
  the second instance at 25.89 seconds and its independent selection at 26.48
  seconds. Reset and invalid-source checks pass by 28.55 seconds. The final
  recovery is dispatched at 28.57 seconds; settlement is observed at 30.54
  seconds on a detached viewer. The late `complete` marker at 30.59 seconds does
  not turn this into a passing test. This journey makes steady progress up to
  its deadline, rather than stopping at initial readiness.

The result supports cumulative setup/interaction pressure for table and
inspector, but does not fully explain the tree's second selection. It also
exposes a diagnostic trap: test timeout does not automatically stop an async
play function, and settlement can follow teardown. The next bounded fixture
investigation should capture the tree's scheduling, worker request/response,
render revision, selection output and connection state **before the deadline**,
using the existing observation hooks. Track post-timeout continuation separately
so detached work cannot be mistaken for a recovered live interaction. Keep
story organization, budgets and concurrency unchanged until that evidence is
available; there is no proposed runtime correction from these traces alone.

The historical stock timeout remains unattributed. These 64 additional passing
probes bring the phase-timing probe total to 186; they do not close that issue.
Local evidence is retained in `/tmp/cem-story-phases-baseline.log`,
`/tmp/cem-story-phases-combined.log`, `/tmp/cem-story-phases-connected.log`,
`/tmp/cem-story-phases-stock.json` and
`/tmp/cem-story-phases-connected-stock.json`. The tables above retain the
findings when those temporary files are unavailable. Reproduce the observations
with the environment flag above and the existing combined-load probe procedure;
enable the flag only on the Storybook process.

Final lint passes with the two existing non-null-assertion warnings. This change
adds fixture observations and documentation only; existing assertions, waits,
story budgets, concurrency configuration and viewer/runtime behavior remain
unchanged.

## Live tree selection through the worker queue

The 2026-09-22 investigation starts at `7c92e8f1` and adds
`.storybook/tree-processing-timing.ts`. Enable its observations alongside the
existing phase marks:

```sh
STORYBOOK_CEM_TREE_TRACE=1 STORYBOOK_CEM_STORY_TIMING=1 yarn nx run cem-elements:test
```

The helper uses the existing scheduler observer and worker-construction seam.
It calls the default worker factory, forwards messages and transfer options,
and records enqueue/dispatch/cancel events, request/response job IDs, instance
revisions, branch-selection control tokens, patch-operation counts and
diagnostic codes. It does not serialize source documents, native artifacts,
result subtrees or complete runtime snapshots. The JSON records are explicitly
diagnostic control data.

The detailed trace activates only for `EditingSelectionAndDisclosure`, before
mounting its authored page. It reads the existing XML viewer's output count,
checked controls, committed revision and connection state, and observes click
and change events. It never calls `snapshotInstance`, which would create a new
revision. A document mutation observer records state changes and ends the trace
on removal of the story root, releasing its listeners. Existing phase marks
still identify later async play/settlement activity. Trace elapsed time starts
at render preparation; phase elapsed time starts at `play`, so correlate the
two streams by their UTC timestamps. Failing-test console replay may duplicate
records; deduplicate by timestamp, event, job ID and scheduling kind.

### Observed live selection

The focused four-story file passes. For its XML viewer, the second selection
produces instance `cem-instance-2`, revision `3`, with both branch keys present.
Its worker round trip takes 955.8 ms; the connected output changes to count 2
2.3 ms after the response. Each request and response includes the same revision.

The full suite is then run twice with the existing eight-page/four-batch stock
probe, launched when Vitest announces `RUN` after build prerequisites:

| Run | Storybook result | Stock probes | Stock warning time |
| --- | --- | --- | --- |
| Worker trace, started 13:27:25 UTC | 201/203, 68.70 s | 32/32 pass | 6.00–9.14 s |
| Repeat, started 13:29:15 UTC | 201/203, 69.63 s | 32/32 pass | 5.63–9.37 s |

Both runs fail only the table and tree's full 30-second journeys. The inspector
passes. In both runs the tree's second selection succeeds **while connected**:
revision 3/count 2 appears at 19.448 and 26.060 seconds from trace start.

The repeat identifies the competing work precisely. All times below are seconds
from that trace's start, on worker `cem-processing-pool-1-slot-1`, owner scope 1:

| Event | Time | Evidence |
| --- | ---: | --- |
| Second selection's change event | 16.841 | Viewer connected, revision 2/count 1, both checkboxes checked |
| Selection compile preflight reply | 16.895 | Job 22 succeeds; its worker round trip is 2.4 ms |
| Remote-document viewer dispatched | 16.895 | Render job 21, instance 5/revision 4, same worker and owner |
| Selection render enqueued | 16.896 | Job 23 waits behind job 21 |
| Remote-document viewer replies | 21.958 | Success after a 5.063 s worker round trip |
| Selection render dispatched/sent | 21.959 | Queue wait 5.064 s; instance 2/revision 3 has `branch.1.1` and `branch.1.2`, both `edit-0` |
| Selection render replies | 26.043 | Success, five patch operations, no diagnostics; worker round trip 4.083 s |
| Connected DOM commits selection | 26.060 | Revision 3/count 2, both controls checked; 17.4 ms after reply |

This is 9.219 seconds from change event to live output. The competing anonymous
component is authored by the fourth card's `data-tree-request.cemt` declaration;
it imports the same tree inspection template. In the first combined run the
selection is dispatched before that component's long render, so count 2 appears
earlier; the later reset then waits behind its 5.404-second round trip instead.
The complete journey reaches another selection near its deadline and is removed
at 30 seconds in both runs. These traces show expensive work sharing one queue,
with successful connected output when there is time to commit it. They do not
show a permanently pending selection, stale committed revision, scheduler
overflow or fallback. They support a timing explanation for the earlier
untraced second-selection failures, but cannot prove their exact interleaving.

Round-trip time includes messaging, engine execution and response delivery; it
is not a native CPU profile. The next optimization cannot be selected from these
timestamps alone. A fast compile-cache response does not identify which render
stage consumes the remaining time.

### Pending decision: rendering profile or story split

Accepted 2026-09-22: the user chose option 1, profiling the worker/native rendering
path. The profiling fixture is active in `docs/todo.md`; optimization remains
subject to the measured hotspot and bounded correction described below.

The requested live-selection investigation is complete. The user requested a
stop at decisions, and the active TODO keeps story organization, time budgets
and concurrency unchanged until this evidence is reviewed. The next work can
take either of these concrete directions:

| Direction | Benefit | Cost or limit |
| --- | --- | --- |
| **Profile the worker/native rendering path first (recommended)** | Investigates latency users also experience; preserves current coverage, templates and execution policy | Needs a native/worker profiling fixture before any optimization can be justified; does not immediately make the combined stress gate pass |
| Split the long journeys into focused stories | Separates format sorting, selection/disclosure and source recovery failures; gives each journey its own existing 30-second budget | Repeats authored-page startup and its competing renders; may still require performance work, and must preserve cross-action and whole-page coverage |

For the recommended direction, use the existing small XML tree and local
request document as fixtures. Measure warmed and initial import, tree
inspection/branch rendering, result construction/diff and worker transport
separately. Keep external syntax handling in CEM-ML import, use retained native
trees downstream, and leave viewer templates unchanged. Identify a measured
hotspot and a bounded correction with Rust coverage before implementing any
shared evaluator or rendering behavior change. No timeout increase, concurrency
cap, worker ownership change or template simplification is proposed here.

Verification: the final normal full suite with observations enabled passes all
203 tests in 42 files (48.98 s). The diagnostic helper passes a targeted strict
TypeScript check; package lint passes with its two existing warnings. An ad hoc
expanded TypeScript check also reports the existing `definePreview` parameter
typing issue in `preview.ts`; the package typecheck excludes these Storybook
files, so it is not claimed as coverage for them. The helper's worker
`postMessage` overload handling was corrected and verified independently.
The stock probe's critical source/runtime hashes match the preceding phase
investigation. All 64 new stock probes pass, bringing the timing-probe total to
250; the historical stock warning timeout remains unattributed.

Local evidence: `/tmp/cem-tree-processing-focused.log`,
`/tmp/cem-tree-processing-stress.log`, `/tmp/cem-tree-processing-repeat.log`,
`/tmp/cem-tree-processing-normal.log` and the two `*-stock.json` reports. The
tables above preserve the findings when those temporary files are unavailable.

### Worker/native tree rendering profile

Completed 2026-09-22. The accepted profiling work identifies repeated built-in
conversion-registry construction inside `cemml:inspect` as the dominant native
cost for these two tree fixtures. Production code, viewer templates, story
budgets and worker concurrency are unchanged by this investigation.

The retained probes are:

- `packages/cem_ql/tests/tree_render_profile.rs`: an opt-in release-mode Rust
  test using the authored tree/request templates, the small XML sample and the
  local request document. It measures a first call and five subsequent calls
  per stage. It checks successful diagnostics, selected-count output and exact
  branch-output equality with an in-memory diagnostic variant that omits only
  the inspection expression. It also compares registry construction candidates
  without changing the production constructor.
- `tools/scripts/profile-cem-tree-render.mjs`: an isolated browser probe using
  the packaged WASM, real workers and unchanged authored declarations. It
  instruments HTTP-served copies of three built modules in memory; installed
  files and worker messages are unchanged. Worker/job correlation separates
  host round trips from worker and nested stage durations. Two selection/reset
  cycles per component verify eighteen connected output/count assertions.
  Its explicitly named JSON report contains diagnostic/control metadata, not
  imported document objects.

Both paths load external XML through CEM-ML import and use retained native CEM
trees downstream. Native HTML serialization is measured as an explicit export
stage; the browser's result mapping is a separate render-plan protocol stage.
These probes do not introduce a document parsing or evaluation path.

Measurements use an Intel Core Ultra 7 258V, Rust 1.96.0, Node 24.16.0 and
Chromium 148.0.7778.96. Runtime sources are at `c9d2845e`; the report records the
profiling script, template and packaged-module SHA-256 hashes. The packaged
WASM hash is
`dd9e2cef978a65254b919e8573d577503b25cbe228784721cb76937716d3f338`.
Native and browser measurements ran sequentially without the earlier
eight-page stress load. Wall-clock timings are observations, not assertions.

Native warm medians, milliseconds (five calls per stage):

| Stage | Small XML | Local request XML |
| --- | ---: | ---: |
| CEM-ML import | 0.343 | 0.374 |
| Typed inspection projection | 0.014 | 0.021 |
| Built-in schema registry | 13.444 | 14.177 |
| Built-in conversion registry | 539.770 | 528.714 |
| Typed inspection output pipeline, registries already built | 14.835 | 20.019 |
| `cemml:inspect` expression | 563.803 | 553.179 |
| Full authored viewer render | 560.912 | 583.647 |
| Same render, inspection expression omitted in memory | 1.708 | 2.037 |
| Explicit HTML export | 0.192 | 0.253 |
| Request template render with retained document | 580.408 | 552.072 |

Initial full renders take 586.663/565.729 ms, and initial conversion-registry
construction takes 543.996/539.977 ms. Template compilation is measured
separately: 21.861 ms warm for the viewer and 28.402 ms for the request module
closure. The registry is rebuilt on every call, so warming does not remove
that cost. Independent timings overlap and vary; they are not an additive
breakdown of a single render. Omitting inspection preserves exact branch HTML
and reduces these renders to about 2 ms, corroborating the direct stage probe.

The profiling-only assembly candidate passes exact converter metadata and
artifact ordering comparisons. Both candidates preserve exact typed output:

| Registry construction candidate | Small XML run | Local request run |
| --- | ---: | ---: |
| One local schema registry per assembly | 46.397 ms | 51.440 ms |
| Clone an already-built conversion registry | 0.031 ms | 0.030 ms |

The first candidate reduces measured construction time by roughly 90–91%; it
still runs the existing metadata extraction and artifact validation. The
second measures owned cloning only, not cache initialization or cache lifetime
behavior. Neither candidate is wired into production rendering, so no optimized
end-to-end browser result is claimed. The release profiling test passes in
28.38 seconds after compilation.

Browser warm medians, milliseconds (eight selection renders per component):

| Stage | Small XML | Local request XML |
| --- | ---: | ---: |
| Host send-to-response round trip | 1188.30 | 1230.15 |
| Worker request handling | 1186.65 | 1227.70 |
| WASM render plus binding/string transfer | 1181.05 | 1222.20 |
| Result JSON parsing and node mapping | 0.25 | 0.30 |
| JavaScript patch diff | 0.10 | 0.15 |
| Render-plan content hash | 2.60 | 2.45 |
| Boundary validation, summed calls | 0.80 | 1.15 |

The WASM stage ranges are 895.0–1226.3 and 1120.1–1281.3 ms. Mapping remains
below 0.7 ms and diff below 1 ms in these samples. The request's single retained
document import takes 1.5 ms inside WASM, 1.8 ms for worker handling. Scoping,
slot projection and resource lowering are also recorded in the raw report;
none approaches the render cost. Stage spans nest and must not be added as
independent costs. Host and worker clocks have separate origins, so only
durations are compared. WASM timings include returned-string transfer; the
independent native measurements identify the constructor within that stage.

The source path explains the repeated work:

1. `packages/cem_ql/src/eval/inspection.rs` constructs both built-in registries
   on every `cemml:inspect` call.
2. `ConversionRegistry::with_builtin_converters` in
   `packages/cem_ml/src/conversion.rs` loads eighteen built-in packages for
   converter descriptors, then the same eighteen for package artifacts.
3. Every `load_builtin_schema_package` call in
   `packages/cem_ml/src/schema/package_loader.rs` constructs the complete
   `SchemaRegistry::with_builtin_schemas` again. One conversion-registry
   assembly therefore causes thirty-six schema-registry constructions.
4. Those loaders use embedded schema/package sources. The formatter pipeline
   subsequently uses the registered package artifacts, so dropping the
   conversion registry would change package-aware formatting behavior.

The browser probe passes all eighteen connected assertions with no worker or
runtime diagnostics. This is isolated stage attribution, not a repeat of the
203-story combined-load gate. It does not establish that a correction will
make that gate pass, or explain the historical stock warning timeout. The
earlier 250 stock timing probes remain a separate evidence set.

Reproduce sequentially from the repository root; the build commands are
prerequisites when packaged browser artifacts are missing or stale:

```bash
cargo test -p cem-ql --release --test tree_render_profile -- --ignored --nocapture
yarn nx run cem_ql:build:wasm
yarn nx run cem-elements:build
node tools/scripts/profile-cem-tree-render.mjs --output=/tmp/cem-tree-render-profile.json
```

Do not run the native profile, browser profile or asset-producing builds
concurrently. The browser probe fails if its expected module instrumentation
anchors change. JavaScript syntax/lint and Rust formatting checks pass. The
normal/combined Storybook suites were not rerun for this profiling-only change.
Local evidence is retained in `/tmp/cem-tree-native-profile-candidate.log`,
`/tmp/cem-tree-browser-profile-final.log` and
`/tmp/cem-tree-render-profile-final.json`; the findings here do not depend on
those temporary files remaining available.

### Pending decision: built-in conversion registry assembly

Accepted 2026-09-22: reuse one local schema registry per assembly. The user
authorized the recommended correction and its native/browser verification.

The requested profiling is complete. The accepted scope above requires a
bounded proposal before shared evaluator/rendering changes, and the user asked
to stop at decisions. Choose the next implementation scope:

| Direction | Benefit | Cost or limit |
| --- | --- | --- |
| **Reuse one schema registry per built-in conversion-registry assembly (recommended)** | Removes thirty-five repeated schema-registry constructions; preserves owned registries and current lifetimes; can retain package order, validation and formatter selection exactly | Still parses package metadata and validates artifacts on each assembly; must verify native/WASM behavior and the combined browser gate |
| Cache the immutable built-in conversion registry and return owned clones | Subsequent construction becomes a cheap clone | Retains metadata for the process/worker lifetime and keeps the original cold-start cost; introduces a cache lifetime decision |

For the recommended correction, keep the existing built-in package list and
registration order. Resolve package descriptors against one local schema
registry, load each embedded package once, and run the existing converter and
artifact extraction/validation functions. Use an internal descriptor-based
loader; retain current public loader behavior and error contracts. Do not skip
package formatters, cache document inspection output, or introduce shared
mutable registries. Apply the same construction pattern to the standalone
built-in descriptor/artifact helpers so they do not retain the repeated work.

Implementation acceptance, if selected: add focused Rust regression coverage
for complete converter/artifact metadata and ordering, default/tabular package
selection and exact typed inspection output, plus isolation of independently
constructed registries. Run the relevant package-loading/conversion/inspection
tests, rebuild WASM and browser assets, then rerun these probes and the normal
and combined-load Storybook gates. Keep time budgets, concurrency and viewers
unchanged. Any remaining hotspot or gate failure requires new evidence before
another shared behavior change.

### Local schema registry correction

The accepted implementation resolves the existing eighteen conversion packages
against one local `SchemaRegistry`, then retains those loaded package
descriptors for the existing descriptor and artifact registration passes. The
standalone built-in descriptor and artifact helpers also use one local schema
registry per call. The embedded-source loader exposes its descriptor-based
entry point only within CEM-ML; public lookup and error contracts are unchanged.
No global cache or shared mutable registry is introduced.

The new `conversion_registry_tests.rs` regression compares complete metadata
and artifact order against packages loaded independently through the public
loader. It checks default/tabular formatter selection and exact typed
inspection output for XML, JSON, YAML and CSV, including the retained source
owner. A second test verifies independent registry mutations and duplicate-ID
errors. Both tests passed against the original implementation before the
constructor was changed. After the correction, all 160 conversion tests, five
package-loader tests and ten CEM-QL inspection/viewer tests pass. The latter
retain coverage of lowered scope budgets, cancellation and source revision
handling. External data still enters only through CEM-ML import.

The release profiling fixture passes after the production change. Warm medians
from five calls per stage, in milliseconds, compared with the preceding profile:

| Stage | Small XML before | Small XML after | Local XML before | Local XML after |
| --- | ---: | ---: | ---: | ---: |
| Conversion-registry construction | 539.770 | 59.276 | 528.714 | 48.254 |
| `cemml:inspect` expression | 563.803 | 77.501 | 553.179 | 82.890 |
| Full authored viewer render | 560.912 | 79.970 | 583.647 | 84.666 |
| Request template with retained document | 580.408 | 79.540 | 552.072 | 90.061 |

The full native render takes about 86% less time for both fixtures. These are separate
wall-clock samples, not deterministic time limits. First full renders also
improve to 77.764/83.648 ms. Exact output/metadata assertions pass; omitting
inspection still leaves the same branches and about 2 ms of native work.

After successful `cem_ql:build:wasm` and `cem-elements:build`, the unchanged
worker probe passes all eighteen connected checks without diagnostics. A repeat
after the full test target rebuilds standalone CEM-ML WASM also passes. Warm
medians from the final eight selection renders, milliseconds:

| Stage | Small XML before | Small XML after | Local XML before | Local XML after |
| --- | ---: | ---: | ---: | ---: |
| Host send-to-response round trip | 1188.30 | 338.50 | 1230.15 | 736.65 |
| Worker request handling | 1186.65 | 337.20 | 1227.70 | 734.55 |
| WASM render plus binding/string transfer | 1181.05 | 333.80 | 1222.20 | 728.35 |

Browser gains differ from native gains; remaining WASM work is not attributed
by this correction. JavaScript result mapping remains 0.25/0.30 ms and diff
0.10/0.15 ms. The first post-change probe records round-trip medians of
328.85/633.80 ms, illustrating timing variance across runs. Both use packaged
CEM-QL WASM with SHA-256
`698f00144d5099e279ffa668680e1449f020f1ad253fe8801df5817ade551059`.
The original probes, templates, time budgets and worker configuration are
unchanged. Native and browser profiles ran separately from builds and suites.

Browser verification with the existing phase/worker observations enabled:

| Run | Storybook result | Tree journey | Inspector journey | Stock probes |
| --- | --- | ---: | ---: | --- |
| Normal | 203/203; 54.44 s suite | 10.886 s | 15.239 s | — |
| Combined, started 14:14:47 UTC | 202/203; 78.75 s suite | 23.499 s | 28.033 s | 32/32 pass; warning 7.08–11.15 s |
| Combined repeat, started 14:17:14 UTC | 202/203; 74.14 s suite | 21.557 s | 27.295 s | 32/32 pass; warning 5.39–10.96 s |

Both combined runs fail only `data-table-demo.stories.ts`'s
`EveryAuthoredSample` at its existing 30-second limit. The tree completes its
selection, disclosure, reset and source-replacement sequence while connected
in both runs; it previously timed out under this load. The table is still a
real failing gate: setup consumes 22.712/21.122 seconds, XML numeric comparison
is verified at 29.003/26.695 seconds, and CSV sorting reaches verification at
30.998/29.031 seconds. Later format/namespace activity continues after teardown
and does not count as passing coverage. The registry correction therefore
improves the measured native/browser paths but does not close overall browser
stabilization. All 64 new stock probes pass (314 total timing probes); the
historical stock warning timeout remains unattributed.

Next, continue the already-selected profiling direction with the remaining
table startup and sorting path. Reuse `data_view_templates.rs` and the authored
multi-format table page; separate native compilation, import, row/column
selection, sorting and output from browser setup/worker queue time. Measure
the competing authored instances as well as a single retained document before
proposing another shared correction. Keep the viewer sources, story coverage,
budgets and concurrency intact. Story splitting and additional runtime changes
are not part of this completed registry correction.

Evidence: `/tmp/cem-registry-baseline.log`,
`/tmp/cem-registry-conversion.log`, `/tmp/cem-registry-loader.log`,
`/tmp/cem-registry-inspection.log`, `/tmp/cem-registry-native-profile.log`,
`/tmp/cem-registry-browser-profile-final.json`,
`/tmp/cem-registry-storybook-normal.log`,
`/tmp/cem-registry-assembly-stress.log`,
`/tmp/cem-registry-assembly-repeat.log` and their `*-stock.json` reports.
Use the profile commands above and the existing synchronized eight-page,
four-batch stock-probe procedure to reproduce these checks. Rust formatting
and whitespace checks also pass.

### Table startup and sorting profile

The next profiling step is complete on 2026-09-22 against production revision
`73a31da3`. No shared runtime, viewer, story, time-budget or concurrency change
is included. The opt-in Rust fixture `table_render_profile.rs` separates CEMT
compilation, native import, row/column selection, grouping, record projection,
sorting, rendering and HTML export. It reuses the small XML, CSV, YAML and JSON
cases from `data_view_templates.rs` and reads expressions from the unchanged
authored table template. Imported documents stay native CEM trees throughout.

The browser profiler now accepts `--fixture=table`. It first mounts the same
four small fixtures individually, then opens a fresh page with the complete
authored `data-table.html`: four CEMT format examples, the CEMT aspect example
and two XSLT examples. Interaction begins at the story's four-textarea boundary,
so remaining sample setup still competes for the worker. All seven cards must
have a table before completion. Each scenario verifies twenty connected row
orders through column selection and repeated text/numeric comparisons. The
original tree mode remains available and passes its eighteen connected checks.

The probe instruments HTTP-served copies of built assets in memory. It records
compile/render spans, worker queue events, revisions and host-control field
sizes; installed assets and worker messages remain unchanged. Size inspection
decodes only the explicitly named host-control envelope. XML, JSON, YAML and
CSV document content still enters through CEM-ML import, including in the
diagnostic variant. Reports identify the source revision, asset hashes and
runtime environment. Nested spans overlap and must not be added together;
field-size collection happens after the WASM span and adds observation overhead
to the worker total. String sizes below are UTF-16 code units, not UTF-8 bytes.

Native warm medians from five calls per stage, milliseconds:

| Stage | XML | CSV | YAML | JSON |
| --- | ---: | ---: | ---: | ---: |
| Direct CEM import | 0.343 | 0.453 | 0.324 | 0.306 |
| Discover columns | 0.049 | 0.054 | 0.055 | 0.052 |
| Select rows | 0.011 | 0.013 | 0.012 | 0.012 |
| Project records | 0.075 | 0.127 | 0.075 | 0.074 |
| Numeric sort expression | 0.085 | 0.097 | 0.087 | 0.105 |
| Full render, source order | 4.749 | 6.804 | 6.677 | 6.844 |
| Full render, numeric order | 7.557 | 7.077 | 7.982 | 6.868 |
| HTML export, numeric order | 0.228 | 0.222 | 0.234 | 0.220 |

Full template compilation takes 65.410 ms. The separate `data:read` expression
imports on its first call and uses its context's native reader cache on warm
calls; its warm times must not be reported as fresh import costs. A diagnostic
template variant binds the already retained native document instead of running
the `cem-data` instruction. Exact HTML agrees in all twelve format/sort cases,
but the variant shows no consistent render improvement. Ordinary small-document
sorting and import do not explain the observed browser interaction cost. These
are wall-clock samples with visible variance, not performance assertions.

The final browser baseline shows a different input scale. After two control
changes, a small table's host-control argument has 183,545–186,191 code units;
the authored page's arguments have 206,626–212,398. The same island snapshot is
available as both `island` and `datadom.island`. Each copy grows from about
28,000 to 89,000 code units for the small fixtures. On the authored page each
copy reaches 97,217–99,585 code units; its actual `datadom.payload` remains
430–874. These are host state projections, not an alternate representation of
the imported document.

For attribution only, `--omit-island` drops those two fields from the served
WASM call's control argument. It is restricted to this table fixture and is
explicitly not a proposed runtime change: public island access must remain.
Both baseline and diagnostic runs pass all forty connected order assertions
with no errors. Warm WASM render medians across the final three comparisons,
milliseconds, are:

| Case | XML baseline / omission | CSV baseline / omission | YAML baseline / omission | JSON baseline / omission |
| --- | ---: | ---: | ---: | ---: |
| Small fixtures | 278.3 / 20.1 | 326.1 / 20.8 | 325.2 / 19.7 | 342.0 / 19.5 |
| Complete authored page | 433.5 / 43.7 | 358.5 / 34.0 | 480.2 / 48.0 | 329.6 / 34.9 |

This counterfactual localizes a large cost to unused control metadata; it does
not prove full public-contract parity or the gain of a safe optimization.
The native second fixture reproduces that cost independently: adding an unread
synthetic host `island` containing 256 nested control records changes the same
table render from 4.953 ms to 350.921 ms, with byte-for-byte identical HTML and
empty diagnostics. The synthetic record is control metadata, not parsed data.

Source inspection identifies repeated deep copies. CEMT's `evaluate_query`
clones the entire `EvaluationContext` for every expression. Lowering predeclares
every policy binding in `CompiledQuery`, and `bind_policy_bindings` then clones
each declared value into evaluator scope, including unread bindings. Record and
array items own their contents, so these clones copy nested values. The host
data-document merge also retains bindings in the synthesized `datadom`.

A deliberately narrow native experiment compiles
`seq:map((1, 2, 3), fn(n) => n + delta)` with both `delta` and the unused island.
Filtering that pure query's policy binding map to IR variable references keeps
the lambda's captured `delta` and exactly preserves `[2, 3, 4]` and diagnostics.
Median evaluation falls from 0.585 ms to 0.053 ms. Cloning its full evaluation
context alone takes 0.130 ms. This experiment is not a general dependency
algorithm: opaque/native callbacks can observe context indirectly.

Startup has a separate measured cost. The full-page baseline reaches four
tables after 4,880.9 ms, compared with 1,503.0 ms for sequential small fixtures.
Two `retainXsltComponent` calls spend 1,169.6 and 1,311.8 ms in compilation;
queued compilation jobs wait as long as 2,602.1 ms before dispatch. Small-fixture
queue waits are at most 0.1 ms. In the omission run, authored setup is 3,344.7 ms
and the two XSLT calls still take 852.3 and 1,041.3 ms. Those separate runs also
vary in compilation time, so their setup difference is not a controlled
measurement of binding-copy savings. Omitting control metadata does not remove
cold XSLT compilation or prove the combined-load table gate will pass.

Verification: both opt-in Rust tests pass, as do forty baseline table checks,
forty diagnostic table checks and eighteen original tree checks (98 total).
JavaScript syntax/lint, Rust formatting and whitespace checks pass. No complete
Storybook gate was rerun for this profiling-only change. The latest gate result
remains the previous 203/203 normal and 202/203 combined-load runs; overall
stabilization and historical stock-timeout attribution remain open.

Reproduce against freshly built assets, running benchmarks separately from
builds and other suites:

```sh
cargo test -p cem-ql --release --test table_render_profile -- --ignored --nocapture --test-threads=1
node tools/scripts/profile-cem-tree-render.mjs --fixture=table --output=/tmp/cem-table-profile-final.json
node tools/scripts/profile-cem-tree-render.mjs --fixture=table --omit-island --output=/tmp/cem-table-profile-omitted-final.json
node tools/scripts/profile-cem-tree-render.mjs --output=/tmp/cem-tree-profile-regression.json
```

Native evidence: `/tmp/cem-table-native-profile-final.log`. Browser reports
above have corresponding `.log` files; final table runs started at
14:43:12/14:43:31 UTC. Earlier independent browser samples in
`/tmp/cem-table-browser-sizes.json` and
`/tmp/cem-table-browser-omit-island.json` show the same attribution.

### Pending decision: expression binding copies

The requested investigation is complete. Per the user's instruction to stop
before another shared behavior decision, implementation pauses here.

Accepted 2026-09-22: reduce unused binding copies with conservative full-context
fallback. The implementation and verification are recorded below.

Recommend limiting expression-local binding copies to compiler-proven
dependencies, with a full-context fallback wherever access cannot be proven.
Apply the same dependency contract to CEMT's evaluation-context preparation and
CEM-QL's evaluator binding setup. Preserve the complete outer template scope,
both public island paths, default data-document merging, node/reference
identity, focus, diagnostics, recovery, scope budgets and reader/native
capabilities. Preserve captures and transitive function dependencies; opaque
native calls or indirect evaluation retain the complete context until an
explicit dependency contract supports narrowing it. A selected `datadom`
binding stays complete, including `datadom.island`; this proposal does not prune
record members or delete public fields.

The benefit is a bounded change at the measured repeated-copy sites, using
compiler information without changing public value representation. The cost
is dependency analysis and conservative fallback; retained full records,
other template scope copies and cold XSLT compilation may still dominate.
The pure-query experiment validates one mechanism, not an optimized full
renderer. Measure the actual end-to-end gain before claiming a timeout fix.

An alternative is to change records/arrays and binding environments to shared
immutable storage. That could make broader cloning cheap, including unknown
callback access, but expands the work into value representation, identity,
mutation and portable-transport contracts. It is not recommended as the first
correction. Dropping island fields or splitting the authored story would not
address the requested shared behavior with existing contracts and coverage.

If the recommended correction is selected, first add focused Rust cases for
direct and transitive reads, nested/forward function captures, shadowing,
defaults, recovery, template callbacks and native functions that inspect
otherwise unread context. Cover both island paths, record enumeration, focus,
scope/limits/cancellation and exact authored output. If dependency metadata is
serialized, cover artifact reload and its compatibility contract. Replace the
profiling fixture's assertion of the current unused-binding behavior as needed;
retain the output/capture assertions and before/after timing evidence. Then
rebuild WASM/browser assets, repeat both native and browser profiles, and run
normal and combined-load Storybook gates with unchanged viewers, budgets and
concurrency. Only after that verification resume the remaining readiness-helper
audit, or stop with evidence if a different shared hotspot needs a decision.

### Expression binding selection implementation

The accepted correction derives dependencies from the existing query IR at
evaluation time. It scans all function bodies and lambda captures, including
forward and nested functions, and selects values by resolved binding ID rather
than source spelling. CEMT copies only the selected bindings into its temporary
evaluation context, and CEM-QL uses the same analysis when populating evaluator
scope. The complete outer CEMT environment and caller data remain intact.

Native extension calls, query-driven CEMT dispatch, unresolved calls/modules
and unknown methods retain full context. The current native callback API takes
explicit arguments, focus, scope/control and resolver capabilities; it does not
expose an ambient binding map. The fallback still deliberately covers native
calls as agreed. Template dispatch can read its host's complete outer scope.
Other supported built-ins use arguments, callbacks in the same IR and retained
capabilities. New IR variants require an explicit dependency review; unknown
stdlib modules fall back rather than assuming closed dependencies.

Selected record/array values remain complete, including `datadom.island` when
`datadom` is selected. Both public island paths, default data-document merging,
node/reference identity, focus, diagnostics, reader caches, resolver/native
capabilities and scope policies are preserved. No document-format handling or
viewer source is changed. The declaration map, query bytes and template
artifact schema are unchanged; dependencies are derived again after reload,
so no artifact version or dependency metadata migration is needed.

Six focused unit cases cover selection, nested/forward captures, shadowing,
recovery, focus, whole records/native nodes, artifact reload and conservative
fallback. Two CEMT fixtures verify indirect template reads of both island paths,
default attribute expressions, record enumeration, caller immutability and
scope/recovery restoration, including portable template reload. Together with
the existing focused native-function, viewer, artifact, expression, recovery,
call-budget and native-view suites, all 77 native checks pass. The original
profiling fixture intentionally retains its declaration-count assertion:
unused declarations remain in IR, while their values need not be copied into
every evaluation. End-to-end measurements and complete gate results follow.

The complete native test command invoked by `yarn nx run cem_ql:test` passes
628 tests, with the three opt-in profiling tests skipped. `cem_ql:lint` completes
with 131 CEM-ML and 41 CEM-QL warnings in existing files and none in the new
modules. After Cargo had finished successfully, both Nx wrappers were stopped
during cache publication: each was copying the 77 GB Rust target directory.
Their terminal logs retain all completed test/lint results; no successful Nx
cache publication is claimed. Timing below was rerun after those writes stopped.

Both release profiling tests pass. Warm medians across five calls, milliseconds:

| Native stage | Before | After |
| --- | ---: | ---: |
| Query with unread control binding | 0.585 | 0.014 |
| Same query with diagnostic declaration filtering | 0.053 | 0.013 |
| Full table render without unread control | 4.953 | 2.762 |
| Full table render with unread control | 350.921 | 74.388 |

The full loaded render takes about 79% less time with exact HTML and diagnostic
parity. The declaration-filtering experiment now has essentially the same
evaluation cost as the unmodified query, because production evaluation selects
the read bindings. Full context cloning still costs 0.140 ms; the correction
avoids repeated copies rather than changing value representation. Complete
selected records and other template scope copies remain, so this is not the
diagnostic browser island omission. Small source-order full renders improve
from 4.749/6.804/6.677/6.844 ms to 2.272/3.187/3.435/3.260 ms for
XML/CSV/YAML/JSON. Template compilation is comparable at 66.842 ms. Evidence:
`/tmp/cem-binding-unit.log`, `/tmp/cem-binding-regressions.log`,
`/tmp/cem-binding-full-native.log`, `/tmp/cem-binding-lint.log` and
`/tmp/cem-binding-native-profile-final.log`.

The WASM and packaged browser builds pass. The ordinary table profiler keeps
the complete control input and passes all forty connected checks without
diagnostics. The argument sizes after comparison changes remain exactly the
same as before: 183,545–186,191 code units for the small fixtures and
206,626–212,398 for the authored page. Warm WASM render medians across the
final three comparisons, milliseconds:

| Case | XML before / after | CSV before / after | YAML before / after | JSON before / after |
| --- | ---: | ---: | ---: | ---: |
| Small fixtures | 278.3 / 67.5 | 326.1 / 64.4 | 325.2 / 54.5 | 342.0 / 53.0 |
| Complete authored page | 433.5 / 99.3 | 358.5 / 68.8 | 480.2 / 96.1 | 329.6 / 77.5 |

These gains use production binding selection with both island paths intact.
The authored page reaches four tables in 3,674.2 ms; cold XSLT compilation still
takes 980.4 and 1,116.1 ms. Startup comparisons include compilation/run variance
and are not attributed solely to this correction. The original tree probe
also passes all eighteen checks: final eight-render round-trip medians are
177.75 ms for small XML and 268.15 ms for retained request XML. All 58 checks
run against packaged CEM-QL WASM SHA-256
`aaff3cb84360e36f05d5eef09d720270ce2e8d5065f9741dd46872dae51af31b`.
Reports: `/tmp/cem-binding-browser-profile.json` and
`/tmp/cem-binding-tree-profile.json`, with matching `.log` files. Browser
profiling ran separately from builds and test suites.

Storybook verification retains the existing phase observations, viewer sources,
assertions, time limits and worker concurrency. Combined load adds the existing
eight-page/four-batch stock probe:

| Run (UTC start) | Storybook | Table setup / complete | Tree complete | Inspector complete | Stock probes |
| --- | --- | --- | ---: | ---: | --- |
| Normal, 15:42:46 | 203/203; 46.09 s suite | 11.028 / 14.953 s | 9.609 s | 5.837 s | — |
| Combined, 15:43:47 | 201/203; 72.49 s suite | 18.800 / 27.469 s | 16.531 s | 11.694 s | 32/32; warning 6.59–10.34 s |
| Late overlap, 15:45:27 | 202/203; 64.76 s suite | 11.651 / 15.309 s | 9.617 s | 5.887 s | 32/32; warning 4.75–8.86 s |
| Synchronized, 15:47:05 | 200/203; 69.36 s suite | 22.237 / **timeout at 30 s** | 17.757 s | 11.722 s | 32/32; warning 5.28–10.15 s |

The first combined run starts its stock load at 15:43:56.187 and the table
finishes within budget. It fails external-src declaration loading at the
existing 120-frame first-button wait and the NPM-version page at its 200-frame
default-selection wait. The late-overlap probe starts at 15:46:01.101, after
that run's table journey has completed, so it provides no second confirmation
of table completion under load. It fails the location page's 180-frame
initial-reader wait. Those failures are observations, not proof of either a
runtime regression or a fixture-only cause; lifecycle/worker evidence is needed.

The final run launches the stock probe automatically as soon as the Vitest
`RUN` marker appears (15:47:06.058). It repeats the external-src and location
readiness failures and the table exceeds its 30-second limit. The table reaches
XML column verification at 25.209 s, CSV numeric comparison at 27.361 s, YAML
at 28.799 s and JSON at 29.238 s. Namespace/reset/invalid-input work continues
after the deadline; the later 31.898 s completion marker is not passing coverage.
The binding-copy correction improves measured native/browser rendering, but
does not close the table stress gate or overall browser stabilization.

All 96 new stock probes pass (410 total timing probes). The historical
45-second stock warning timeout remains unattributed. Logs:
`/tmp/cem-binding-storybook-normal.log`,
`/tmp/cem-binding-storybook-stress.log`,
`/tmp/cem-binding-storybook-repeat.log`,
`/tmp/cem-binding-synchronized.log`, and reports
`/tmp/cem-binding-stock-stress.json`, `/tmp/cem-binding-stock-repeat.json`,
`/tmp/cem-binding-synchronized-stock.json`. Synchronize future load repeats to
the Vitest `RUN` marker rather than a manual later check, and retain timestamps
to distinguish actual setup overlap from overlap elsewhere in the suite.

Next, profile the remaining table setup/render cost under that synchronized
load: cold XSLT compilation/queue time remains separate from full selected
record copies and other template scope copies. Continue the already-planned
readiness audit with the three observed frame-count failures, collecting
declaration/render/worker state before attributing or changing them. Another
shared performance correction requires its own measured proposal and decision;
this accepted change does not authorize record-member pruning, shared mutable
value storage, story splitting or larger time budgets.

### Remaining startup and readiness attribution

The next investigation keeps production CEM-ML/CEM-QL/runtime code, viewer
sources, story assertions, time limits and worker concurrency unchanged. The
existing opt-in native table profiler now also isolates constant interpolation
with unread host controls, selection of a member from a complete control record,
and the public XSLT preflight/name-resolution/compile/reload boundaries. Both
base and imported-aspect XSLT components must render exactly the same HTML as
independently compiled, reloaded bundles, including numeric row order. All
external source data continues through native CEM-ML import; synthetic records
in these profiles are explicitly host controls.

`STORYBOOK_CEM_TREE_TRACE=1` now observes the table, external-src, NPM-version
and location stories as well as the tree. The added readiness observations
record registration, declaration/render settlement, diagnostic codes, control
counts and the existing frame-loop outcome. They never create runtime snapshots
or revisions, serialize document/native artifact content, await additional work
in the story, or extend a deadline. Observers disconnect when the story root
leaves the document. Existing assertion messages containing rendered markup are
redacted in the timing stream. Trace logging can affect timing; these are
attribution runs, not a promise of identical latency without instrumentation.

The table browser profiler adds `--stock-load`. It launches the existing
32-probe stock load (eight pages, four batches) at the authored table mount,
records the child launch/result and report path, and fails if the load probe
fails. Its isolated small-table case runs first without the competing load.
It preserves all seven authored cards and all forty connected table checks.
This is a standalone attribution workload, distinct from the complete
Storybook stress gate.

Reproduce the native and standalone browser profiles with:

```sh
cargo test -p cem-ql --release --test table_render_profile -- --ignored --nocapture --test-threads=1
node tools/scripts/profile-cem-tree-render.mjs --fixture=table --stock-load --output=/tmp/cem-startup-table-load.json
```

Run profiling after compilation and other checks have finished. For the full
Storybook workload, enable both trace flags and start the existing stock probe
as soon as the Vitest `RUN` marker appears, retaining UTC timestamps. This run
uses `/tmp/cem-startup-synchronized.py` to poll that marker every 100 ms and
launch `diagnose-cem-stock-startup.mjs --concurrency=8 --batches=4`. The table
profiler's mount synchronization does not substitute for that full-suite run.

#### Native evidence

All four release profiling fixtures pass, with five warm measurements per
stage. The final run is `/tmp/cem-startup-native-profile-final.log`:

| Stage | Warm median (ms) |
| --- | ---: |
| 100 literal spans, unread control record | 0.122 |
| 100 constant interpolations, empty input | 0.237 |
| Same interpolations, unread control record | 11.346 |
| Select one member of a small control record | 0.010 |
| Same member, record also containing unread controls | 0.102 |
| Authored native table without extra controls | 2.134 |
| Identical table with unread control binding | 73.496 |
| Base / aspects XSLT bundle compilation | 508.426 / 839.982 |
| Base / aspects bundle reload | 11.337 / 19.618 |
| Base / aspects complete component compilation | 529.391 / 877.129 |
| Base / aspects component render | 24.937 / 39.409 |

The constant-interpolation comparison reads no control binding and declares no
hooks, isolating renderer work from record member access. The member-selection
comparison separately confirms that selecting a binding still copies its
complete record; it does not authorize dropping unread members from public
values. Exact HTML and empty diagnostics are asserted throughout.

Both bundle imports remain validated. Reload is only a small portion of cold
component construction, so bypassing artifact validation is neither justified
nor proposed. The base bundle contains 104 XPath programs / 2,183,361 bytes;
the imported-aspect bundle contains 127 programs / 3,771,191 bytes. More precise
attribution inside compilation remains future work.

#### Browser evidence

Normal Storybook passes **203/203** in 44.74 s. The table reaches its setup
boundary in 10.019 s and completes in 14.043 s; tree and inspector journeys
complete in 8.113 and 5.260 s. External-src finds its first button after
45 frames / 1.499 s. NPM's default selection takes 113 frames / 2.523 s and
location's two readers take 50 frames / 2.225 s.

The synchronized full-suite run starts at 16:15:54 UTC; stock starts at
16:15:54.874. Storybook passes **202/203** in 66.19 s, with external-src's first
button as the only failure. Table setup takes 22.652 s and completion 29.213 s;
tree completes in 18.049 s and inspector in 10.685 s. This near-deadline table
pass does not close the prior 30-second timeout or establish stable headroom.
All 32 concurrent stock probes pass, with initial warnings in 5.62–9.56 s.
The table's two XSLT compile worker round trips are 6.678 and 4.365 s; its
cached CEMT compile requests wait 11.788–11.828 s before dispatch. These are
whole-job/scheduler observations, distinct from the native-function spans in
the standalone probe below.

External-src's failure has concrete lifecycle evidence:

- The button's produced tag is defined at 35.7 ms; its declaration settles at
  2,116.0 ms without diagnostics.
- The unchanged 120-frame button wait expires after **4,030.8 ms** (4,086.6 ms
  from trace start). All four declarations are registered, the whole-document
  and subtree output is present, and neither declarations nor instances report
  diagnostic errors. The button itself is absent.
- The button's render settles without diagnostics at **4,135.6 ms**, 49 ms
  after the timeout. The root detaches at 4,152.7 ms. Settlement alone does not
  prove that every subsequent assertion would pass; this remains failed
  coverage, not a passing test obtained by counting late work.

That observed wait expires during unfinished rendering. It is not a missing
registration or reported declaration-load error. Worker events for this
external-src instance are absent in this run, so the trace does not establish
which internal stage consumes the pending render time. In particular, existing
worker pools can predate the diagnostic worker factory; do not infer worker
execution details from lifecycle settlement alone. NPM and location pass in
this load run (default selection 101 frames / 2.100 s; readers 32 frames /
1.089 s). Their earlier failing runs still need failure-time evidence before
attribution; a passing repeat is insufficient.

The standalone loaded table probe passes all **40/40** checks and **32/32**
stock probes. The authored four-table boundary takes **8.872 s**; all seven
cards are verified by 14.838 s. Cold base/aspect `retainXsltComponent` calls
consume **3.762 s** and **2.353 s** on the same worker. Subsequent CEMT compile
requests wait up to **6.468 s** before dispatch, then complete in 0.5–16.1 ms
from cache. These expensive cold XSLT jobs delay the first CEMT table renders;
the queue is a measured consequence, not evidence that concurrency should
change. Initial four-table native render calls take 94–230 ms each.

Warm comparison render medians under this load are 457.8/230.4/119.2/117.7 ms
for XML/CSV/YAML/JSON. Those comparisons occur sequentially at different phases
of the competing load and must not be read as relative format-parser costs.
The isolated small-table medians are 58.0/52.3/53.2/52.3 ms. CEM import and all
public island bindings remain intact. The stock probes' warnings appear in
4.50–5.81 s. All 64 new stock probes pass (474 total timing probes); the
historical 45-second stock warning timeout remains unattributed.

Package lint passes with its two existing non-null-assertion warnings. Rust
fixture formatting, script syntax and diff checks pass. No production rebuild
was needed; the browser target reused the existing WASM/dependency builds.

Evidence: `/tmp/cem-startup-normal.log`,
`/tmp/cem-startup-synchronized.log`,
`/tmp/cem-startup-synchronized-stock.json`,
`/tmp/cem-startup-table-load.json`, and its `.stock.json` report. The profiler
records source/build hashes; packaged CEM-QL WASM remains
`aaff3cb84360e36f05d5eef09d720270ce2e8d5065f9741dd46872dae51af31b`.

### Pending decision: expression hooks with no eligible handler

Accepted 2026-09-22: skip copies when no hook can run. Implementation and
validation are recorded below.

The measured no-hook interpolation fixture and code inspection identify a
bounded next correction. `PlanRenderer::apply_expression_hook` clones every
hook scope, the complete binding environment, current focus and input sequence
before checking whether any hook can run. The ordinary table declares no
expression hooks, but each interpolation still incurs this saving/restoration.
Compiler-proven expression binding selection cannot remove copies made by this
separate renderer path. Copies of a selected record are another cost and remain
outside this proposal.

Recommend returning the unchanged input stream immediately when **no visible
hook matches the output destination and is outside the active-hook stack**.
Test that eligibility before cloning any scope/context/input. An eligible
hook with a false match predicate must still follow the existing evaluation
path: predicate evaluation, diagnostics, ordering, captured bindings, lexical
scope, active-hook recursion exclusion and recovery are observable behavior.
Content/attribute construction and typed conversion still run at their existing
later boundaries. No binding declarations or record members are removed, and
all public island access, native identities, references and input metadata stay
available.

At proposal time, this fast path was neither implemented nor benchmarked. The
fixture measured the existing overhead, without claiming a post-change gain
or a fix for cold XSLT compilation. The alternatives are retaining that repeated
work, or a broader shared-value representation change with a larger API and
ownership review. Prefer the small eligibility check first.

After approval, add focused native regressions for content and attribute
interpolation with no hooks, opposite-destination hooks and only active hooks;
preserve whole-sequence values/diagnostics and native identity. Exercise false
predicates, nested lexical/priority dispatch, captured caller scope, typed
attributes, recoverable errors and depth limits, including portable reload.
Then implement the guard, repeat these profiles, rebuild WASM and run normal
and synchronized Storybook gates. Keep the viewers, budgets, concurrency and
all assertions unchanged.

The readiness audit remains separate: use the external-src lifecycle evidence
to design a bounded wait that observes the actual declaration/render lifecycle
and still checks the authored output inside the existing story limit. Capture
NPM/location failures when they recur. Do not replace these waits with larger
frame counts or count late settlement as passing coverage. Cold XSLT lowering
also remains a separate profiling task; shared compiler changes require their
own measured proposal. The investigation stopped before shared renderer implementation, as requested;
the user subsequently approved the bounded eligibility check.

### Expression hook eligibility implementation

The approved correction checks the visible hook scopes before saving any
renderer state. If no hook has the requested destination outside the active
hook stack, `apply_expression_hook` returns its owned `ItemStream` directly.
The input items, references, cursor, chain marker, errors and diagnostics stay
intact, and the current binding environment/focus stays in place. If any hook
is eligible, the original predicate, ordering, capture, recursion and recovery
path runs unchanged. A predicate that returns false is still evaluated and can
emit diagnostics or raise a recoverable error.

The production change is confined to that guard. It adds no cache, mutable
shared values, record pruning, API/schema changes or external-format handling.
Native CEM-ML import, node reuse, content/attribute construction and typed
validation retain their existing boundaries. Viewer sources and all browser
assertions, waits and concurrency remain unchanged.

A deterministic unit regression reproduced the old copy before implementation:
`content/empty` returned a different item-vector allocation. With the guard,
all six content/attribute × empty/opposite/active cases preserve both input and
binding allocations, native identity, diagnostics, partially consumed stream
state and errors. The active-only case runs at the call-depth ceiling to verify
that bypassing an ineligible handler adds no recursion failure.

Two new public integration tests run directly compiled and reloaded portable
artifacts. They check native body/attribute identity, mixed atomic/node and
empty sequences, integer conversion and invalid destination constraints.
Native predicate probes verify exactly one call per eligible expression,
complete captured control records, focus, warning diagnostics and recovery from
predicate errors. Existing hook/attribute/portable/binding tests cover lexical
activation, priority, outer fallback during recursion, captured caller behavior,
limits and cancellation. The focused checks pass (one allocation regression and
69 integration tests), followed by the complete native suite: **641 passed**,
five opt-in profiles skipped. The full Nx native test command uses
`--skipNxCache`, avoiding publication of the large Rust build directory.


All four release profiling fixtures pass with exact HTML/diagnostic checks.
After builds finish, warm medians across five calls are:

| Native stage | Before (ms) | After (ms) |
| --- | ---: | ---: |
| 100 constant interpolations, empty input | 0.237 | 0.192 |
| Same interpolations, unread control record | 11.346 | 0.287 |
| 100 literal spans, unread control record | 0.122 | 0.130 |
| Full table without extra control binding | 2.134 | 2.689 |
| Identical table with unread control binding | 73.496 | 34.299 |
| Select a member from a loaded control record | 0.102 | 0.120 |

The isolated interpolation overhead falls about 97.5%; the full loaded native
table takes about 53% less time. The simple table/record figures retain timing
variance and remaining work: selected records and other template scopes still
copy complete bindings. This correction does not authorize pruning them.
Cold XSLT component construction remains 554.089/914.653 ms for base/aspects;
its artifact sizes and XPath program counts are unchanged. Skipping no-handler
saves does not solve cold compilation.

Native lint passes with the existing 131 CEM-ML and 41 CEM-QL warnings and none
in the changed production code. WASM and packaged browser builds pass. The
normal table profiler passes all forty checks; authored startup takes 3.310 s
and cold XSLT compilation takes 924.9/1,031.5 ms. Warm browser render medians
across the final three comparisons are:

| Case | XML before / after | CSV before / after | YAML before / after | JSON before / after |
| --- | ---: | ---: | ---: | ---: |
| Small fixtures | 67.5 / 27.6 | 64.4 / 24.6 | 54.5 / 27.2 | 53.0 / 26.4 |
| Complete authored page | 99.3 / 39.1 | 68.8 / 32.4 | 96.1 / 37.6 | 77.5 / 31.1 |

The before measurements use the same binding-selection production build
recorded above, before this guard. The final comparison control-input sizes (including both island paths) are
exactly unchanged for both table scenarios. The unchanged tree probe also passes all
18 checks; its final eight render round-trip medians are 132.25 ms (small XML)
and 165.30 ms (retained request XML). The updated packaged WASM SHA-256 is
`653a28d17e8f2992572edb5143b3079acc8de36d2de945c7fed45648a3f7482f`.

Evidence: `/tmp/cem-hook-fastpath-red.log`, `/tmp/cem-hook-fastpath-green.log`,
`/tmp/cem-hook-compatibility-red.log` (compatibility assertions pass before the
guard), `/tmp/cem-hook-regressions.log`, `/tmp/cem-hook-full-native.log`,
`/tmp/cem-hook-native-lint.log`, `/tmp/cem-hook-native-profile.log`,
`/tmp/cem-hook-browser-build.log`, `/tmp/cem-hook-table-profile.json`, and
`/tmp/cem-hook-tree-profile.json`.


The first normal Storybook run passes **201/203** in 39.55 s. Its table reaches
setup in 8.840 s and finishes in 11.700 s; tree/inspector complete in
6.914/3.118 s. The failures are `FailedTransportCanRetry` and
`InterruptedBodyCanRetry` in `declaration-source-retry.stories.ts`, at line 90's
unchanged paragraph assertion following declaration settlement. Their source
is the literal `{p | Ready}`. Both use the Testing Library default
`asyncUtilTimeout: 1000`; neither waits for render settlement. The failures
show missing output at that deadline, not a captured render/worker failure.
They therefore do not establish either a renderer regression or a fixture-only
cause.

The isolated four-story retry suite passes in 2.06 s (532 ms of tests), and a
full normal repeat passes **203/203** in 38.69 s. The repeat's table setup /
completion is 7.903 / 10.177 s, with tree/inspector at 6.365/2.835 s. The first
run's failures remain recorded; the repeat does not erase them. Add these two
waits to the readiness audit and capture lifecycle/worker state when they fail
again before changing assertions or source-retry behavior. Logs:
`/tmp/cem-hook-storybook-normal.log`, `/tmp/cem-hook-retry-focused.log`, and
`/tmp/cem-hook-storybook-repeat.log`.


The synchronized full-suite gate starts at 16:40:39 UTC; the stock probe starts
at 16:40:39.848. **Storybook passes 203/203** in 63.94 s and all **32/32** stock
probes pass (warning readiness 3.46–10.21 s). Table setup / completion is
20.489 / 28.264 s; tree and inspector complete in 15.119 / 8.038 s. External-src
finds its button in 94/120 frames (3.800 s), NPM's default selection in 137/200
frames (2.968 s), and location's readers in 37/180 frames (1.106 s). Those waits
are unchanged. This is a passing synchronized gate, but one near-deadline table
journey does not close overall stabilization or the historical readiness
failures. Logs: `/tmp/cem-hook-synchronized.log` and
`/tmp/cem-hook-synchronized-stock.json`.


The standalone table probe with synchronized stock load also passes **40/40**
checks and **32/32** stock probes. Its authored setup is 10.008 s and all seven
cards are verified by 14.665 s. Cold XSLT calls take 3.594/2.972 s and the maximum
worker queue wait is 7.616 s. Under that changing load, the final comparison
render medians are 195.2/80.6/86.3/120.0 ms for XML/CSV/YAML/JSON. Those cases run
sequentially at different load phases; neither their ratios nor total startup
variation should be attributed entirely to the guard. Stock warning readiness
is 4.43–6.03 s. Reports: `/tmp/cem-hook-table-load.json` and its `.stock.json`
companion. Across this implementation, all 98 browser profile checks and 64
stock probes pass (538 total stock timing probes).

The approved optimization is complete. Next, continue the readiness audit
(external-src, NPM/location and the newly observed source-retry waits), using
actual declaration/render/worker state and retaining output assertions inside
the existing story deadline. Separately profile cold XSLT construction and the
remaining selected-record/template-scope copies before proposing another shared
change. No overall stabilization completion is claimed: the initial normal
retry failures and historical timeouts remain evidence requiring follow-up.

### Declaration and retry readiness audit

The external-source story now awaits each of its four declarations, asserts
registration, and awaits each produced instance's render before checking the
existing button, whole-document, subtree and XSLT output. Its earlier captured
120-frame deadline expired during pending rendering. The fixture uses the
existing public lifecycle promises and the existing 30-second story deadline;
no frame budget is increased and no output assertion is removed.

The opt-in readiness observer now recognizes the runtime's declaration tag,
excludes declaration elements from the produced-instance inventory, and records
the state of its initial render observation. `initialRender` is deliberately
not a claim about every later HTTP or interaction revision. Explicit retry
checkpoints capture acquisition count, registration, retained diagnostic
history and render state. The shared worker tracer observes compile/render
requests and responses. This is control metadata, not a runtime snapshot or a
source-document export. Tracing remains disabled by default.

Before changing retry waits, the focused four-story subset and normal full
suite pass (4/4 and 203/203); the normal suite takes 37.01 s. A synchronized
full suite also passes 203/203 in 59.10 s, alongside 32/32 stock probes. The
table journey takes 26.355 s; tree/inspector take 15.570/6.841 s. External-source
button/XSLT renders settle at 4.001/4.124 s and all subsequent assertions pass.
NPM's default selection takes 65/200 frames (1.513 s), and location's readers
take 33/180 frames (1.123 s). Those two waits remain unchanged.

The retry stories run late in that full suite, so focused concurrent coverage
is necessary. A first stock-first attempt passes 4/4 and 32/32 but does not
overlap: Nx startup delays the stories until 2.1 s after stock finishes. This
is not evidence about retry behavior under contention. Launching stock at the
focused Vitest `RUN` marker then reproduces **one failure out of four** with
the original one-second wait:

- `retry/transport` successfully settles its first retried declaration at
  876.7 ms. All three produced tags register; the replacement declaration has
  no errors. The first two declarations retain their intentional earlier
  `src_load_failed` history, as required by the retry contract.
- The worker becomes ready at 1,052.9 ms and successfully completes compile
  jobs. The first instance's `render-diff` is sent at 1,834.4 ms.
- The assertion expires at **1,914.9 ms**, while that render is in flight.
  All three initial renders remain pending and none reports an instance
  diagnostic. The two acquisition attempts remain exactly two.
- Cleanup disposes the scope after failure. Render diagnostics observed after
  that disposal are not evidence that source retry failed before the timeout;
  this failed run does not verify the remaining output/cache assertions.

The shared recovery assertion now awaits declaration settlement, checks
registration, awaits render settlement, and directly asserts `Ready`. Both
transport and interrupted-body recovery, replacement and cache reuse take this
same path. Source loading, retries, request counts, diagnostic history and
scope-disposal behavior are unchanged. The stream failure from the earlier
normal suite was not reproduced here; only the transport failure above is
attributed by this run.

With that fixture correction, the same synchronized focused workload passes
**4/4**, including every later cache assertion, alongside **32/32** stock probes.
The transport declaration settles at 805.6 ms and its render at 2,132.5 ms:
the intervening **1,326.9 ms** exceeds the removed polling deadline, yet the
complete story passes within its existing limit. No production/runtime/viewer
change is involved.

The remaining inventory needs separate attention. Thirteen demo-story files
contain animation-frame waits: components, data slices, DOM merge, for-each,
forms, HTTP, local storage, location, module URL, module URL referrers, NPM,
set-URL and string functions. Some waits check interactions rather than startup;
they must not all be replaced by a generic initialization wait. NPM's five
cards contain five named pickers plus three anonymous wrappers and asynchronous
native HTTP data. Location's three cards contain separate anonymous readers.
A complete card count establishes neither child initialization nor resource
completion. `whenRenderSettled` observes current rendering and pending native
local-storage work; it is not a general future-HTTP completion signal. Preserve
resource/output-specific checks and capture NPM/location failure state before
choosing their correction. No shared lifecycle extension is proposed here.

Evidence: `/tmp/cem-readiness-retry-focused.log`,
`/tmp/cem-readiness-normal.log`, `/tmp/cem-readiness-synchronized.log` and its
`-stock.json` report; `/tmp/cem-readiness-retry-load.log` (no actual overlap),
`/tmp/cem-readiness-retry-synchronized.log` (reproduced failure), and
`/tmp/cem-readiness-retry-fixed.log`, with their respective `-stock.json` reports.

Final normal coverage, with worker/readiness tracing disabled, passes
**203/203** in 41.83 s; the table journey takes 13.152 s. Final synchronized
coverage passes **203/203** in 63.60 s, with table/tree/inspector completion at
**28.600/18.163/8.066 s**. Its 32 stock probes pass, with warnings at 4.76–9.70 s.
NPM's unchanged default-selection wait uses 114/200 frames (2.561 s), and
location's unchanged reader wait uses 31/180 frames (0.945 s). All 160 stock
probes in this audit pass (698 cumulative timing probes); this still does not
attribute the historical 45-second stock timeout.

Final evidence: `/tmp/cem-readiness-final-normal.log`,
`/tmp/cem-readiness-final-synchronized.log` and its `-stock.json` report.
Package lint passes with its two existing non-null-assertion warnings
(`/tmp/cem-readiness-final-lint.log`); the whitespace check passes. No native
production code changed, and the browser target reused the existing WASM build.
The fixture correction is complete, not overall browser stabilization. Keep
NPM/location tracing available for a reproduced failure and continue the
independent cold-XSLT stage profile before choosing another shared optimization.

### Cold XSLT compiler stage profile

Native profiling now follows the actual compiler through test-only,
thread-local spans. The spans and candidate code are absent from production
and WASM builds. No timer runs in ordinary tests unless a profiling session is
active. Two ignored release fixtures inspect the complete base/aspect viewer
stylesheets, including imported modules, Dev source maps, native input and
unchanged artifact validation. Every compilation builds a fresh bundle; only
process-level schema initialization warms up. Nested timings are inclusive and
must not be added to their parent stages.

The final release run reports these five-repeat medians (milliseconds):

| Stage | Base viewer | Imported aspects |
| --- | ---: | ---: |
| Complete bundle compilation | 493.444 | 821.583 |
| Stylesheet parsing/validation | 4.698 | 5.922 |
| Lowering, including XPath programs | 11.473 | 13.916 |
| XPath adaptation/compilation within lowering | 4.675 | 5.568 |
| Generated CEMT artifact, including encoding | 462.770 | 774.143 |
| CEMT compilation within that artifact | 455.092 | 759.460 |
| Artifact encoding | 7.693 | 15.341 |
| Bundle composition, including hostile-reload validation | 14.118 | 25.046 |
| CEM-QL type checking within CEMT compilation | 425.383 | 702.005 |
| Function-surface seeding within type checking | 327.898 | 536.570 |
| Registry assembly within seeding | 43.117 | 70.195 |
| Host-binding declarations within type checking | 7.214 | 14.773 |
| Inference/checker disposal within type checking | 90.221 | 149.504 |

The first base compile takes 646.926 ms, including 154.790 ms of stylesheet
parsing/schema initialization. Later parsing is about 5 ms. The aspects case
starts after that process-level initialization; its first compile is 821.997 ms.
An unprofiled compile takes 529.411/810.525 ms. These observations retain ordinary
timing variance; they are not post-optimization gains. Separate bundle reload is 11.177/18.538 ms.
Validation bypass is not justified by these results.

Generated CEMT contains **959/1,559** compiled query expressions, versus
**104/127** XPath programs. Each query seeds the complete standard type surface:
17 default aliases plus the bare helper surface each assemble a new registry,
for **17,262/28,062** registry assemblies per bundle. The seeded table has
399 signatures and 32,859 parameter slots. `native:call` alone declares
arities 1 through 255, accounting for 32,640 of those slots. Registry assembly
is measurable but is only part of repeated signature construction and disposal.
No arity or function is removed by this investigation.

The profile asserts exact equality of complete artifact bytes across all six
recorded compiles and a compile without recording, then reloads and renders
native XML with numeric sorting and empty diagnostics. A second reload renders
identical HTML. These fixture URIs yield bundles of 2,184,579/3,772,915 bytes;
generated CEMT is 73,208/117,864 bytes. Source-map owners account for URI-dependent
artifact sizes; comparisons use identical URIs. Runtime source data still enters
through shared CEM-ML import. Viewer templates and production code paths are
unchanged.

### Prepared type-checking baseline (accepted 2026-09-22)

The user approved one lazy prepared baseline per template compilation on
2026-09-22. The measurements below record the pre-implementation proposal;
implementation and verification are the active TODO.

Two test-only candidates retain the full function table. Six batches of 128
checker setups, reporting the median after the first batch, give the following.
All timed batches have recording disabled and include checker disposal; registry
counts are collected separately. The prepared case also includes preparation
and disposal of its one extra baseline.

| Setup strategy | Median (ms) | Range (ms) |
| --- | ---: | ---: |
| Current complete initialization for each checker | 53.819 | 53.136–58.322 |
| One local registry per checker initialization | 48.565 | 47.448–50.071 |
| One prepared built-in baseline, cloned per checker | 44.105 | 43.976–45.702 |

The local-registry candidate reduces 2,304 registry assemblies to 128 and
improves this setup measurement about 10%. The prepared-baseline candidate
includes preparing its baseline once per batch and improves the measurement
about 18%. It still allocates and disposes independent function tables. These
are setup measurements, not measured whole-compiler or browser gains; the
prepared candidate does not yet participate in the real template compiler.

Both candidates match every signature, imported prefix and pre-existing host
variable scope against current initialization. Eight cases under strict and
development configurations also match inferred types and complete diagnostics:
simple expressions, lambdas, a standard-library alias, an opaque host import,
URI helpers, native calls, unknown functions and invalid arithmetic. Existing
custom host function registration remains present. The candidate alias list
exists only in the test fixture, with complete-table equality guarding drift;
it must not become a second production registry.

**Recommend one lazily initialized, owned built-in type-checker baseline per
CEMT compilation.** Clone it into a fresh checker for each expression, then
apply that expression's type configuration, import aliases, host bindings and
local declarations in the current order. Initialize it only when an expression
actually reaches type checking. Literal-only templates and failures before
type checking should not pay for preparation. Keep standalone compilation's
public API and existing behavior. Do not share a mutable checker, cache caller
bindings, remove public function entries, change arity handling, weaken type
checks, or introduce a process-global cache.

This retains one additional fixed built-in surface during compilation and
requires an internal prepared-context path between CEMT and CEM-QL. Each query
still owns its working table, and the prepared surface is released when that
template compilation ends. The smaller alternative changes only local registry
assembly, with less code but the smaller measured benefit. Keeping initialization
unchanged avoids both changes and retains the measured cost.

After the decision, add native regressions for cross-expression/template
isolation, custom/local function overrides, imported aliases, strict/development
diagnostics, failed compilation, no-expression templates, source maps and
complete portable bytes. Implement the accepted path, repeat the actual stage
profile, rebuild WASM and check normal plus synchronized Storybook/standalone
startup with existing limits, concurrency and viewer sources. The remaining
selected-record/template-scope copying and NPM/location readiness attribution
remain separate open tasks.

Reproduce the profiles with:

```sh
cargo test -p cem-ql --release --lib profile_ -- --ignored --nocapture --test-threads=1
```

Evidence: `/tmp/cem-xslt-stage-profile.log` (initial stage split),
`/tmp/cem-xslt-type-stage-profile.log` (type-check attribution),
`/tmp/cem-xslt-candidate-profile.log` (first candidate comparison), and
`/tmp/cem-xslt-final-profile.log` (final profiles with setup timing separated
from registry counting). Both final release profiles pass. The full native
Nx test gate passes **648 tests**, with seven opt-in profiles skipped; native
lint passes with the existing 131 CEM-ML/41 CEM-QL warnings. New fixture
formatting and whitespace checks pass. Logs:
`/tmp/cem-xslt-stage-native-tests.log` and `/tmp/cem-xslt-stage-native-lint.log`.
At that profiling checkpoint, browser/WASM gates were deferred until an actual
production optimization. The test-only investigation made no browser speed or
stabilization-completion claim; implementation results follow below.

### Prepared baseline implementation and validation

The accepted implementation owns `PreparedTypeChecking` inside each CEMT
`TemplateCompiler`. Parsing and import resolution still run first. The first
expression that reaches type checking seeds the existing built-in surface once;
every expression receives an independent clone with its current type
configuration, module imports and host bindings, followed by local declarations.
The baseline contains no caller data and is dropped with the template compiler.
Public `compile`, `type_check` and `TypeChecker` entry points keep their existing
initialization and ordering. There is no process cache, signature pruning,
mutable checker reuse, format-specific branch or viewer change.

Native regressions compare strict/development diagnostics and complete query IR
across imports, local function overrides, variables, policy bindings, arity and
failed expressions. The baseline's complete signature table remains unchanged.
Literal templates and parse/import failures do not create it; two separate
three-expression template compilations each assemble the built-in registries
18 times, compared with 54 before this change. Portable template envelopes match
byte for byte against the standalone checker path in Dev and Prod modes,
including compilation diagnostics and source frames; reloaded valid templates
render successfully. A thread-local test-only comparison switch is restored on
unwind and is absent from production and WASM.

The release profile now compares both actual viewer bundles against fresh
checkers in the same binary, preserving every portable byte and rendering check.
Paired timings run with recording disabled for both paths. The complete native
suite passes **652 tests**, with seven opt-in profiles skipped. Native lint passes with the unchanged 131 CEM-ML/41 CEM-QL warnings.

The release profile passes both fixtures. Each measurement creates a fresh
bundle; the reported five-run medians exclude the first run and retain warm
process-level schema caches. Both paths run in the same binary without stage
recording during the paired comparison:

| Complete bundle compilation | Fresh checkers (ms) | Prepared baseline (ms) | Reduction |
| --- | ---: | ---: | ---: |
| Base viewer | 517.700 | 439.875 | 15.0% |
| Aspects viewer | 883.580 | 756.092 | 14.4% |

Prepared ranges are 436.991–465.036 / 735.023–790.800 ms; fresh-checker ranges
are 505.719–572.264 / 863.393–908.897 ms. This is measured complete native
compilation, not the earlier setup-only projection or a browser speed claim.
The separately recorded stages show 18 registry assemblies per bundle instead
of 17,262/28,062. All 959/1,559 expression checks still run with the complete
function table. Bundle sizes remain 2,184,579/3,772,915 bytes. Native source
import, numeric sorting, empty diagnostics, reload and repeated rendering pass.

Evidence: `/tmp/cem-prepared-red.log`, `/tmp/cem-prepared-focused.log`,
`/tmp/cem-prepared-native-tests.log`, `/tmp/cem-prepared-native-lint.log`, and
`/tmp/cem-prepared-release-profile.log`. Browser verification follows below.

The browser package and WASM build pass. The packaged CEM-QL WASM SHA-256 is
`935a52f614f56d9b163e308ea68b4b084ec9675dff4670fc16a16352aff1bc23`.
Standalone table and tree profiles pass **40/40** and **18/18** assertions with
no reported errors. This table run measures base/aspects XSLT retention at
799.0/753.1 ms, versus 924.9/1,031.5 ms in the preceding hook profile. Authored
four-table startup measures 3.015 s (previously 3.310 s); all seven cards are
ready at 4.219 s (previously 4.417 s). These single-run browser comparisons are
observations, not a controlled speed estimate. Viewer sources, data-import
boundaries, rendering limits and worker concurrency are unchanged.

Logs: `/tmp/cem-prepared-browser-build.log`,
`/tmp/cem-prepared-table-profile.{json,log}` and
`/tmp/cem-prepared-tree-profile.{json,log}`.

Both full Storybook runs pass **203/203** tests: normal 40.43 s and synchronized
62.57 s. Viewer completion times (normal / synchronized) are 11.195 / 24.082 s
for tables, 7.184 / 15.410 s for trees and 3.581 / 7.342 s for the inspector.
The synchronized table remains below its unchanged 30-second deadline; the
preceding run completed at 28.600 s. These are observations under the recorded
workloads, not a claim that all historical readiness failures are resolved.

The synchronized run launches stock probes only after Vitest's `RUN` marker.
All **32/32** probes pass at eight concurrent pages over four batches, with
initial warnings in 3.987–9.237 s and no recorded errors. Probes ran from
17:53:21.690 through 17:53:57.665 UTC, overlapping table, tree and inspector
work. NPM and location stories started later, after this additional stock load
ended: their longest waits were 153/200 frames (2.797 s) and 31/180 frames
(1.077 s). This run therefore does not settle their historical failures under
load. Their readiness attribution, the historical stock timeout and remaining
selected-record/template-scope copies remain open; no viewer, request-lifecycle,
concurrency or deadline adjustment is bundled into this compiler change.

Evidence: `/tmp/cem-prepared-normal.log`,
`/tmp/cem-prepared-synchronized.log`,
`/tmp/cem-prepared-synchronized-stock.{json,log}` and
`/tmp/cem-prepared-synchronized-status.log` (both subprocesses exit zero).
