# Authored demo Storybook coverage

The [inventory](demo-storybook-coverage.json) maps all 35 authored HTML documents
and 224 normalized sample legends to existing asynchronous Storybook contracts.
One designated story owns each page's structural inventory. Other stories may
exercise additional interactions on the same page; this mapping is not a claim
that the designated story already asserts every sample's behavior.

## Enforced boundaries

- Unit tests discover `index.html` and every HTML document recursively under
  `demo/`. They reject missing, stale, duplicate or blank entries, changed story
  titles, removed exports, missing render/async play functions, and excluded
  story owners. The TypeScript AST check does not execute browser story modules.
- The browser inventory story parses each authored document as HTML, preserving
  attribute decoding and excluding comments, script text and inert nested
  templates from the page's sample inventory. It checks exact normalized legend
  order, including external-file source previews.
- After each owning story's `play`, the shared preview requires its actual
  `<cem-element src>` declaration, produced host and exact rendered legend list.
  It adds no readiness wait and does not suppress failures from the play function.
- Six supporting documents without demo cards have explicit source-document or
  fragment stories. The root gallery also has its own source-loaded story.
- `yarn nx run cem-elements:verify-demo-coverage --parallel=1` requires unit
  contracts, the full Storybook suite and the independent standalone/source-page
  verifier together. Phase 3A includes this aggregate. The independent verifier's
  sample assertions and source loading remain separately owned.

The broader TODO item remains open until each sample's observable output and
interaction contract has been audited. Structural ownership does not substitute
for those assertions. Inventory enforcement adds no runtime changes or readiness
waits. The approved root-gallery, fixture and shared empty-string corrections are
described below.

## Approved root-gallery correction

The initial root story observed three Bulbasaur buttons (`ivysaur`, `venusaur`, and
an empty label) and two Ninetales buttons (`vulpix` and an empty label). The last
button in each case uses the host's own Pokémon ID. The shared
`dataDocumentElementsByAttribute` implementation explicitly appends the host
after payload entries; the authored loop selected that broader lookup.

The user approved changing only the root demo's loop to
`datadom.payload.elementsByAttribute.pokemon-id`. The authored-template native
regression failed before that one-line correction with three buttons instead
of two. It now passes for two, one and zero payload entries, while a control
retains the broad lookup's additional host entry. The native Nx test inputs
include the root HTML source. Storybook and both independent gallery modes now
require exact payload button counts and nonempty image alternatives. Shared
lookup behavior is unchanged.

Desktop (1280px) and mobile (390px) root-page checks preserve all seven cards
without horizontal overflow or browser errors. Desktop cards share rows; mobile
cards fit the viewport. Both sizes produce exactly `ivysaur`/`venusaur` and
`vulpix`, with matching nonempty image alternatives. Sprite requests use offline
SVG fixtures, so this check does not assert remote-service availability.

## Verification evidence, 2026-09-23

- All 508 unit tests pass, including 55 inventory and deliberate-rejection cases.
- All nine new browser cases pass: two inventory checks, the root gallery, and
  six supporting documents. The checks include rejecting copied sample markup
  without a matching source declaration, a missing host and duplicate samples.
- The native authored-template regression passes for two, one and zero payload
  entries after reproducing the original extra-button failure. All 31 standalone
  pages and 37 source-loaded documents pass the independent gallery verifier
  with the corrected root demo and exact button-count assertions.
- Lint passes with the same two existing warnings. The package's Nx typecheck
  passes. An additional standalone compilation of the story imports reports
  the existing `definePreview` controls-parameter typing error; compiling the
  unchanged HEAD preview with the same temporary configuration reproduces it.
  Temporary configurations and diagnostic console output were removed.
- The final serial aggregate, after the root correction, passes 508/508 units,
  all 31 standalone / 37 source-loaded gallery documents, and 215/217 Storybook
  tests. Its only failing prerequisite is Storybook: the same null `href` in
  `NestedExternalSrcUsesLoadedDocumentBase` and null `src` in
  `MaterialIconLinkParity`. At that point the aggregate failed; no test was
  skipped or weakened to make the inventory pass.
  The 2026-09-24 follow-up below attributes the two failures separately.
- The earlier traced full browser run passes 214/217. `NestedExternalSrcUsesLoadedDocumentBase`
  reads a null inline resource `href`; `MaterialIconLinkParity` reads a null logo
  `src`; the referrer matrix exceeds its existing frame budget. The selected
  three-file control passes 83/85, with the first two failures repeated. Repeating
  that control with the unchanged HEAD preview (without the new coverage hook)
  produces the same 83/85 result. This rules out a requirement for the hook to
  reproduce those failures, but does not establish their lifecycle cause.
- The first independent gallery attempt encountered a transient missing
  `cem-ml-npm/dist/wasm/browser/cem_ml.js` while a separate diagnostic Nx command
  restored that shared output. This is not a product-failure attribution.
  Subsequent aggregate verification ran serially, without concurrent Nx
  commands that restore shared build artifacts.

The original HTTP/location/stock timeout attribution remains open. Existing
whole-scope replacement behavior remains accepted. Readiness failures require
captured lifecycle state before changing fixture waits or runtime semantics.

## URL failure attribution, 2026-09-24

Read-only worker transport observations captured the affected render revisions,
the `asset`, `cemurl` and `logourl` scalar slices, emitted resource controls,
diagnostics, and corresponding DOM attributes. No source documents or runtime
snapshots were serialized. The diagnostic run retained each original assertion;
on failure it observed the owning instance's render settlement and then rethrew
the original error. This later observation is evidence, not passing coverage.
Temporary instrumentation was removed before validation.

| Story | Original assertion | After the owning render settled | Attribution |
| --- | --- | --- | --- |
| `MaterialIconLinkParity` | At 1920.7ms, revision 2 has the expected href and a null logo src. Its render input contains only `cemurl`. | At 1949.1ms, revision 3 contains both slices and publishes the expected logo src, without diagnostics. | The href predicate completes before the independent logo URL publication. |
| `NestedExternalSrcUsesLoadedDocumentBase` | At 1929.9ms, the anchor href is null and compilation reports unknown variable `asset`. | At 1948.1ms, the next render input contains the correct absolute `asset` slice, but compilation still rejects `asset` and href remains null. | The authored bare variable is not a declared binding; extra waiting does not repair compilation. |

Material's existing predicate now requires both exact URLs before the remaining
URL, nested-icon, composition and accessibility assertions. Its 120-frame budget
and the shared runtime are unchanged. The focused icon-link selection passes
both matching stories after reproducing the original failure.

The user approved the fixture-only correction on 2026-09-24. The nested-source
template now uses `{$datadom.slices.asset}`. After its first output appears, the
story settles that specific instance and rejects diagnostics before asserting
the exact URL. The shared runtime/compiler and alias behavior are unchanged.
The native regression reads the actual story template, reproduced the unknown
variable error before the correction, and now verifies one compiled artifact
across absent, resolved and cleared resource-slice states. Nx native test inputs
include that source file.

The diagnostic run captured both failures and their settled states, then stalled
while the runner finished; it was terminated without claiming a full-suite
result. Final validation uses the restored, uninstrumented stories. Historical
HTTP/location/stock timeouts and the separate traced referrer-frame timeout
remain open.

Final validation after both corrections passes the native authored-template
regression, all 217 Storybook tests, all 508 unit tests, and all 31 standalone /
37 source-loaded gallery documents. The combined `verify-demo-coverage` gate,
lint and package typecheck pass through serial Nx execution; lint retains the
same two existing warnings. The full browser run with only the Material fix
passed 216/217, isolating the remaining nested-source failure before its approved
correction. The per-sample behavior audit remains open, starting with the eight
attributes-demo samples and their missing default-precedence assertion.

## Attributes audit finding, 2026-09-24

The audit draft adds a named step for each of the eight authored legends and
checks visible paragraphs and input values together with reflected attributes.
The first sample and the nonempty external changes pass. Case 1a's empty-value
assertion fails before the remaining cases can be validated.

After the authored **set p3** button reads an empty input, an immediate read
finds `p3=""` while the output still says `p3: def_P3`. After the produced
instance's render settles, the attribute is `p3="true"`, the output says
`p3: true`, and diagnostic history is empty. The original story checked only
the immediate attribute, allowing a pass before the contradictory output.
Temporary observations were removed after reproducing this transition.

The shared `hostAttributes()` reader in `cem-elements.ts` converts every empty
DOM attribute string to boolean `true`. The fixture's `??` expression then
keeps that boolean and reflects its text. This conflicts with the authored
empty-versus-missing lesson and the exact-string attribute authority required by
`docs/cem-element-lifecycle-principle.md`.

The user approved the shared fix on 2026-09-24. Host snapshots, template-value
bindings and serialized hydration now preserve empty DOM strings. Actual typed
booleans remain typed values. The native authored-template contract already
passed before the adapter fix, distinguishing missing values, empty strings,
the string `false`, and both boolean values. Browser regressions cover CEM-ML,
DOM templates and serialized hydration, including resumed mutation, empty
dataset values and explicit boolean slices. All three initially reproduced the
empty-string conversion. The hydration fixture captures serialized output and
island at the same revision before calling the snapshot API again.

The authored attributes HTML remains unchanged. The source-loaded story now
checks each normalized legend in its own named step:

| Sample | Observable assertions |
| --- | --- |
| 1. attributes definition | All three defaults, reflected and displayed. |
| 1a. External attribute changes | External p1/p3 edits, constant p2, removal restoring p3's default, and empty p3 remaining empty. |
| 1b. Container attribute values | Container p1/p3 values and isolation from the edited instance. |
| 2. attribute from slice | Initial title, nonempty input and empty input, reflected and displayed with the input value. |
| 3. V attribute matches input value | Default, input update, empty input and is-changed state. |
| 3a. Container value before input | Container value, input replacement, empty input and is-changed state. |
| 4. attribute defaults, from container, and from slice | Default before input, entered and cleared values, effective value and has-input state. |
| 4a. External changes versus user input | Container and external values before input; entered and cleared input retain precedence after further external changes. |

Both independent gallery modes now require the exact normalized `p3:` output
and empty reflected attribute, then removal restoring `def_P3`. This prevents a
substring or transient attribute-only assertion from hiding the conversion.

The first full browser run passed 216/220 tests. Three failures identified
consumers that relied on the coercion: the canonical CEM-ML render-loop fixture
and `cem-select`'s loading and invalid-state projections. The component
conventions already require presence-only boolean flags; preserving raw strings
therefore requires explicit presence checks, not truthiness. Those consumers
now use `seq:count(datadom.attributes.<name>) > 0`. The select declaration covers
busy, invalid and disabled flags in dropdown and listbox modes. Its native
contract reads the actual XHTML and exercises absent, empty, `false` and other
nonempty flag values. The existing listbox `aria-disabled` compound expression
also used an invalid XPath `$` prefix; the replacement is a bare CEM-QL
expression. Browser expectations retain the existing public behavior.

The canonical fixture's revised expression also exposed its frame-only startup
wait. A diagnostic run retained the failing assertion and then awaited that
instance's render: it produced the expected `aria-busy="true"` button with no
diagnostics. The fixture now awaits its own runtime's render settlement and
rejects diagnostics before asserting output. Its native regression reads the
actual story template and verifies absent and present flag values. Temporary
observations were removed; the shared render lifecycle and frame budget are
unchanged.

The fourth failure was the previously recorded referrer-frame timeout; its wait
budget and semantics remain unchanged. It repeated in the compatibility
selection and passed during the subsequent diagnostic selection, so that
historical attribution remains open.

Final serial validation passes all three native authored-template tests, all
220 Storybook tests, all 508 unit tests, and all 31 standalone / 37 source-loaded
gallery documents. The aggregate coverage gate, component declarative
architecture, lint and package typecheck pass; lint retains its two existing
warnings. The eight-sample attributes audit is complete. The next behavior-audit
fixture covers the six `cell-overrides.html` samples. The wider per-sample audit
and earlier startup-timeout investigations remain open.

## Cell-override behavior audit, 2026-09-24

All six samples have named steps in `AuthoredSourcePreviews`, with the existing
specialized stories retaining their interaction coverage. The authored HTML,
data files, templates and runtime are unchanged.

| Sample | Story and observable assertions |
| --- | --- |
| 1. Name cells become Pokémon pictures | `AuthoredSourcePreviews` checks every initial row. `PokemonCellPictures` checks each name, image alternative, sprite URL and unchanged data URL before sorting and after ascending/descending sorting; one selected source row remains selected. |
| pokemon-cells.json | `AuthoredSourcePreviews` checks the exact file text, highlighted source tokens and empty demo output. `PokemonCellPictures` also checks that sorting leaves the source preview unchanged. |
| 2. Zero-stock cells get a warning | `AuthoredSourcePreviews` checks all five name/stock pairs. `ConditionalStockFallback` verifies both sort directions, selection retention, the single zero-stock warning and ordinary reference stock. |
| stock-cells.xml | `AuthoredSourcePreviews` checks exact highlighted file text and empty demo output. `ConditionalStockFallback` checks that the preview remains unchanged after interactions. |
| stock-cell.cemt | `AuthoredSourcePreviews` checks exact highlighted template text and empty demo output. |
| 3. Native values pass into another component | `AuthoredSourcePreviews` and `NativeAttributeValues` check the exact paragraph values, numeric result, date, retained mixed-content name subtree and text-only projection. |

Stock assertions account for the base viewer's direct-text column before name
and stock. The story finds those columns by heading; the independent verifier
checks their headings and positions.

Both independent gallery modes additionally check per-row Pokémon name/URL
pairing after sorting, all five stock values before and after sorting, and
selection retention. Both modes require exact external-file preview text and
empty demo output, independently of Storybook.

The independent source harness now imports the real shared `cem-demo-element`
and maps its browser WASM dependency. The user approved this change after exact
preview-text assertions exposed that the former inline stub activated templates
but ignored external `src` previews. All three file-preview checks are shared
between standalone pages and source-loaded documents. The offline route for
legacy remote demo-helper imports is unchanged.
The scoped-CSS anonymous-checkbox assertion now targets `[slot=demo] b` so
source-highlighting tokens cannot substitute for the live element under test.

The focused four-story selection, 508 unit tests, lint and typecheck pass. The
independent gallery passes all 31 standalone pages and 37 source-loaded
documents with the real helper. Both initial aggregate attempts and a dedicated
full Storybook rerun passed 219/220, failing only the previously recorded
referrer-frame timeout. An earlier full run before the harness replacement
passed 220/220. The referrer readiness follow-up below resolves that failure;
the aggregate gate and six-sample audit now pass. Lint retains its two existing
warnings.

## Referrer matrix readiness, 2026-09-24

The scalar-referrer timeout is an assertion preceding render settlement. A
diagnostic full run retained the original 120-frame predicates and observed the
failure before awaiting the owning instance solely to collect its later state.
The original error was rethrown, so that run still failed at 219/220.

| Observation | Render revision | Nine URL cells | Connection and diagnostics |
| --- | --- | --- | --- |
| Original assertion failure | 1 | All empty | Same connected sample; no declaration or instance diagnostics |
| Owning render settled, 324.7ms later | 10 | All exact expected URLs | Same connected sample; no declaration or instance diagnostics |

The diagnosis also accounted for test order: Vitest prioritizes previously
failed files, whereas this short story ran later after passing. A temporary
sequencer placed the matrix first in the full suite to reproduce its startup
conditions. No worker or network delay was injected. Earlier passing probes
reached revision 10 in both focused and concurrent runs; those passes alone
were not treated as attribution.

The story now uses the existing `whenCemSourceRendered` helper before checking
the sample inventory and exact URL matrix, and rejects declaration/instance
diagnostics. Both 120-frame predicates, the nine expected URLs, transient-control
removal assertion, and 30-second story timeout are retained. The corrected
story passes its focused run and all 220 stories with the matrix first.
Temporary observations and test ordering have been removed. Final validation
under normal ordering passes all 220 Storybook tests, 508 unit tests, and
31 standalone / 37 source-loaded gallery documents. The aggregate
`verify-demo-coverage` gate, lint and typecheck pass; lint retains two existing
warnings. This closes the referrer fixture and cell-override audit sign-off.
The separate historical HTTP/location/stock investigations remain open.

## Data-slices audit checkpoint, 2026-09-24

`EveryAuthoredSample` now exposes named steps for all 16 authored samples.
The first six steps (A1, A2, B and 1–3) strengthen the existing checks:

| Samples | Added observable assertions |
| --- | --- |
| A1 / A2 | Initial input and displayed count agree; increments, decrements and direct editing (A1) or the declared tap event (A2) keep them paired. |
| B | Initial outputs are empty; mousemove and click each display the exact dispatched page X, event type and offset Y, with both offsets present in a valid computed shadow. |
| 1 / 2 | Initial input and output agree; input events leave the displayed slice unchanged until change, and clearing keeps both empty without restoring the default. |
| 3 | Initial input and output agree; input events update both, including an empty value. |

The next step exposed a defect in sample 4, whose legend promises an initial
slice value from an attribute. After the source render settled, both inputs
were empty while their attribute outputs correctly showed `😁` and `🤗`.
Both connected instances reported `cem.ql.render.compile_failed` at revision 1;
the declaration had no diagnostics. Native compilation of the actual authored
template identifies the attribute expression `$s ?? $a` as rejected XPath-style
variable syntax.

A temporary native probe replaced only that expression in memory with
`s ?? a`. It compiled without diagnostics and rendered the default `😁`, supplied
`🤗`, edited, and explicitly empty values correctly. Empty slices already use
the intended absent-versus-empty distinction; this diagnosis does not require
a change to their shared semantics. The probe and browser observations were
removed after recording the evidence.

The user chose to investigate shared syntax support instead of the proposed
demo-only correction. The [investigation](../../../docs/cem-ql-template-dollar-investigation.md)
confirms that both the renderer and embedded audit normalize only simple
references. Three native probes cover the current host surfaces, byte-preserving
token normalization, and its unwanted acceptance of prefixed declarations and
record keys. They support a parser-context implementation instead of global
replacement. The remaining decision is template-only reference aliases versus
aliases throughout CEM-QL, with declaration names kept bare in either case.

The authored HTML, compiler, runtime and independent gallery contracts are
unchanged. Existing assertions for samples 4–13 remain in place; their
strengthened audit and the gallery audit remain open in `docs/todo.md`.

Checkpoint validation passes the focused story and serial aggregate gate:
220 Storybook tests, 508 unit tests, all 31 standalone pages and 37 source-loaded
documents, lint and typecheck. Lint retains two existing warnings. Passing
these existing gates does not close sample 4's defect or the remaining audit.

## Shared reference syntax and data-slices audit, 2026-09-24

The user selected optional `$name` references in all CEM-QL queries, with
declaration names kept bare. AC-QS-7 records that contract. The shared expression
parser now accepts an adjacent prefix, retaining the original source and byte
ranges while using existing name resolution, type checking and lowering.
The renderer and embedded auditor no longer strip simple prefixes. Native
regressions cover module and standalone APIs, template spans and content,
audit agreement, literals/comments, diagnostic offsets, malformed prefixes,
and forbidden prefixes on declarations, parameters, members, static keys and
types. All 722 native tests pass, with 9 existing ignored tests.

Sample 4 retains its authored `{$s ?? $a}` expression. Its native regression
reads the actual HTML template and verifies default, supplied, edited and empty
values. The source contract retains that expression, and Storybook checks both
instances and rejects declaration/instance diagnostics. Nx's native test inputs
include `data-slices.html` so changes to the authored fixture invalidate the
test cache.

All 16 Storybook steps now check initial and resulting state. Both independent
gallery modes use the same sample contracts, with an atomic form-state check
for input values, checked properties and displayed output arrays.

| Sample | Observable behavior checked |
| --- | --- |
| A1 | Initial input and displayed count, increment, decrement and direct editing. |
| A2 | Initial input/output pair, click and tap increments, and decrement. |
| B | Initially empty metadata; mousemove and click handling. Storybook checks exact observed coordinates and both shadow offsets; gallery checks hover offset Y and both authored style offsets. |
| 1 | Blank initial state, input waiting for change, committed value and clearing. |
| 2 | `B` initial state, input waiting for change, committed value and clearing without restoring the default. |
| 3 | `B` initial state, input-event updates and clearing. |
| 4 | Default/supplied attribute fallbacks, independent edits to both instances, and empty slice distinct from an absent slice. |
| 5 | Input `B` paired with slice `xB`, edited `C`/`xC`, and empty input/`x`. |
| 6 | Initial `anonymous` and button-driven `broccoli`, paired with the input. |
| 7 | Nested initial zero and click/tap increments. |
| 8 | Independent clicked/focused values across focus, click, tap and blur. |
| 9 | Supplied/default emotions, independent edits and clearing, plus reflected host attributes. |
| 10 | Initially absent slices, one input updating both outputs, and clearing both. |
| 11 | Initial attribute/absent slice, updates and clearing paired with the input and reflected attribute. |
| 12 | Initial checkbox properties and all three outputs; independent uncheck/recheck transitions and explicit slice values. |
| 13 | Initial radio choice, switching to the other value and back, with exclusive checked state. |

Authored HTML and raw legacy event-expression evaluation are unchanged. The
broader per-sample audit and historical HTTP/location/stock investigations
remain separate open work.

Final aggregate validation passes all 220 Storybook tests, 508 unit tests,
31 standalone pages and 37 source-loaded documents, component lint and
typecheck. The inventory check requires literal `sampleContract` legends;
the two initially generated entries were made explicit before the successful
rerun. Rust lint also passes with caching disabled, with warnings in unchanged
code; component lint retains its two existing warnings. This completes the
16-sample data-slices audit. The next fixture audits the 10 data-table samples.

## Data-table audit, 2026-09-24

`EveryAuthoredSample` now runs a named step for each of the 10 authored samples,
awaiting the source document's render before inspecting its viewers. Both
independent gallery modes use the same sample contracts. Table-state checks
compare every scalar cell and the selected row/button state together; nested
cells have separate table assertions.

| Samples | Observable behavior checked |
| --- | --- |
| 1. XML | Exact attribute/text headings and cells, missing and empty mood values, plus both nested evolution rows. |
| 2. CSV | Exact columns and cells, including a quoted comma, escaped quotes and the empty final field. |
| 3. YAML | Exact quantities, fruit and later boolean values, missing cells, and both nested tag rows. |
| 4. JSON | Empty string, absent key and null remain distinct; malformed input removes stale tables and reset restores source, cells and selection. |
| 1–4 and 6. CEMT/XSLT tables | Text order and numeric order in both directions, selected row and pressed button retained across sorting, edited source clearing selection, reset restoring it, and explicit return to source order. The gallery also retains pointer/Enter/Space disclosure checks. |
| 5 and 7. CEMT/XSLT aspects | Notes render as a tree while visits retain exact table cells; local address/action edits survive disabling and restoring aspects. Clearing the address keeps the preview empty, and the source textarea remains unchanged. |
| Three external previews | Displayed code exactly matches each repository file, the shared helper reaches ready state, and its demo output remains empty. |

The XML namespace and inert processing-instruction regression now has its own
story. Existing XSLT format-switching, focus/caret preservation, instance
isolation and render-patch tests remain. The select helper checks the control's
selected value; complete row-state assertions verify the resulting sort.
Authored HTML, CEMT/XSLT templates and shared runtime behavior are unchanged.

Final aggregate validation passes all 221 Storybook tests, 508 unit tests,
31 standalone pages and 37 source-loaded documents, lint and typecheck. Lint
retains two existing warnings. The initial gallery run exposed whitespace
inserted by its visible-text helper between adjacent text nodes; form-output
checks now use exact DOM `textContent`, matching the Storybook assertions.
This completes the 10-sample data-table audit. The six data-tree samples are
next; the broader per-sample audit and historical startup investigations remain
open.

## Data-tree audit, 2026-09-24

`EditingSelectionAndDisclosure` now runs named steps for all six authored
samples after the source document settles. The existing repair, request and
preview stories remain focused entry points. Both independent gallery modes
use the same six sample contracts, including explicit external-preview entries
instead of appending attribute-only checks from the generic inventory loop.

| Sample | Observable behavior checked |
| --- | --- |
| 1. XML branches | Exact branch labels, namespace and empty attribute, ordered text/CDATA/processing-instruction content, and namespace declarations excluded from branch attributes. Independent selection agrees with checked properties, visible Selected labels and the total. Disclosure preserves selection; reload and replacement clear it. Script-like source remains inert. |
| 2. JSON branches | Exact object/property/string/array branch structure, present empty note, fruit and nested tag values. Parent/child selections remain independent; deselecting a collapsed parent preserves the live disclosure, summary and child checkbox identities, closed state and child selection. An unchanged-source change event and reload both clear selection. Storybook also verifies instance isolation. |
| 3. Repair | Malformed source has an error without stale inspection, branches or count. Repair creates the expected branches; selection is cleared after returning through the malformed original and repairing again. Storybook retains the textarea-focus check. |
| 4. Local requests | Loaded XML and JSON content, selector values and resolved download links; switching files clears selection. No source removes inspection, controls, selection count and link and reports idle. Reloading XML starts unselected. |
| XML/JSON source previews | Exact repository-file contents, ready state and empty demo output. Storybook additionally checks source token coloring. |

Gallery selection checks compare branch labels, checked properties, selected
classes, visible Selected markers and the displayed count in one observation.
The existing pointer and keyboard disclosure checks remain. Authored HTML and
CEMT templates are unchanged.

The stronger JSON interaction exposed a disclosure-retention defect:

1. In sample 2, select `1.3: property` (tags) and `1.3.1.2: string` (sweet).
2. Collapse the tags property's disclosure, retaining both selections.
3. Deselect the tags property with its checkbox, leaving sweet selected.

The independent standalone browser resolved the live disclosure again after
step 3 and found `open=true`. A focused Storybook probe confirmed that the old
`details` is disconnected and a different live element is open, with no owning
instance diagnostic codes. The first Storybook assertion had incorrectly
inspected the detached element, whose `open` property was still false.
The user approved a shared fix. A native regression establishes that source
provenance is stable across conditional label removal, including repeated
branches. Adapter regressions then reproduced changing disclosure IDs in both
a small repeated template and the actual JSON tree template. The adapter's
global preorder counter renumbered later elements when a Selected label
disappeared, causing the patch layer to replace them.

The shared WASM adapter now derives element, text and comment IDs from native
source provenance, parent identity and occurrence at the emitting site. This
keeps unrelated conditional output from shifting later IDs while distinguishing
repeated source sites. Repeated output at the same site remains positional;
this change does not introduce item-keyed list reconciliation. The existing
projection patch implementation needs no change for this defect.

Storybook and both gallery modes now deselect the parent while it is closed,
query the current elements and compare their identities with the connected
originals. They also check the closed property and remaining child selection
before reopening the disclosure.

Four metadata stories now assert scoped and stable IDs instead of ordinal
literals. The storage JSON transition regression also checks the accepted
whole-scope recovery policy: its object `ul` and array `ol` are separate authored
root branches and now have distinct IDs. Direct whole-plan application therefore
replaces that scope with the existing recovery diagnostic, including unchanged
paragraphs. No recovery or projection policy is changed.

Validation passes all 723 native tests (9 existing ignored), 510 unit tests,
221 Storybook tests, 31 standalone pages and 37 source-loaded documents. The
aggregate coverage gate, native lint, package lint and typecheck pass; existing
lint warnings remain. This closes the six-sample data-tree audit. The three
`dom-merge.html` samples are next; the broader per-sample audit remains open.

## DOM-merge audit checkpoint, 2026-09-24

The three samples cover a change-bound textarea counter, an input-bound CEM-QL
counter and an input-bound XPath comparison. Existing Storybook checks exercise
counts, repeated words, Unicode codepoints and some focus/caret retention. The
independent gallery has counting checks but lacks control identity, focus and
selection assertions. Those gaps remain part of the open audit.

A draft source-loaded interaction exposed a value-preservation issue in sample 1:

1. Its initial textarea value is `Hello world!` followed by a newline and 12 spaces.
2. Clearing and typing `one two three` leaves that exact value and the old count, 2.
3. Pressing Tab commits the slice. After the owning render settles, the count is
   3, but the live value is `one two three` followed by the same 13-character suffix.

The textarea's authored CEM-ML body places its closing brace on a separate,
indented line after `{$datadom.slices.text}`. A native regression confirms that
this literal suffix is preserved for nonempty, empty and Unicode bindings.
Putting the closing brace beside the expression yields exactly the binding in
all three cases. The shared projection's `setRenderedText` then applies the
rendered textarea body to its live value, explaining the browser observation.

The initial recommendation was limited to sample 1's closing-brace placement.
The user instead selected investigation of shared whitespace handling on
2026-09-24. The draft exact-value assertions reproduced the failure and were
removed from the checkpoint; they are not passing regression coverage. The
three-sample audit remains open in `docs/todo.md`.

Checkpoint validation: all 49 native template-render tests pass, including the
new whitespace case. Native lint passes with existing warnings. Browser
evidence is a deliberately failing exact-value probe, not a passing audit.

### Shared whitespace investigation, 2026-09-24

The suffix is source whitespace, not whitespace introduced by the browser or
the slice expression. The native tokenizer emits it as `Trivia` with an exact
byte range. `TemplateCompiler::parse_attributes` skips trivia through the first
body item, including whitespace after `|`. Once child compilation starts,
`compile_node` converts both `Text` and `Trivia` into rendered text. This makes
leading and trailing whitespace asymmetric. It applies equally to `textarea`,
`pre`, `p` and `code`; the CEM template compiler has no element-specific
whitespace mode.

The new `packages/cem_ql/tests/template_whitespace.rs` characterization fixture
records these results with binding `text = "A"`. Escapes below denote actual
source characters, not CEM escape syntax.

| CEM body after `\|` | Current rendered body | Compatibility implication |
| --- | --- | --- |
| `\n  {$text}\n  ` | `A\n  ` | Opening layout is skipped; closing layout is emitted. |
| `  {$text}  ` | `A  ` | Inline trailing spaces also survive. |
| `\n  literal\n  ` | `literal\n  ` | Trailing whitespace belongs to a nonempty literal text run. |
| `{span \| A} {span \| B}` | `<span>A</span> <span>B</span>` | Discarding all trivia would remove an intentional separator. |
| `{span \| A}\n  {span \| B}\n  ` | `<span>A</span>\n  <span>B</span>\n  ` | Discarding newline trivia changes existing mixed-content output too. |

Further cases establish that CRLF/tab layout behaves the same way, Unicode
spaces survive after a binding, and a whitespace-only body currently renders
empty. Bound values and triple-backtick rich-content bodies retain exact spaces,
newlines, tabs, NBSP/EM spaces and Unicode text. The render plan keeps the bound
value and suffix in separate text nodes, with a distinct source range for the
suffix even when the binding is empty. This gives a compiler policy enough
information to distinguish source layout from user data before projection.

`@xml:space=default` and `@xml:space=preserve` currently pass through as output
attributes in CEM templates; neither changes this behavior. The bounded XSLT
compiler has its own implemented `xml:space` rules, verified by the existing
`xslt_output` suite. Those rules do not currently configure CEM template bodies.

The input-AST requirements in [AC-P-9 and AC-O-6](../../../docs/cem-ml-ac.md)
require whitespace/source evidence to survive parsing and reporting. The
[rendered-output contract](../../../docs/cem-ml-stack-design.md#rendered-output-projections)
allows a selected transform/renderer to control emitted whitespace. These
requirements permit a shared compiler policy, but do not choose automatic
layout stripping. The generic AST builder separately suppresses tokenizer
trivia payloads while retaining their source ranges; the template compiler
consumes tokens directly, so changing that builder would not fix this demo.

Changing `setRenderedText` to trim the final body would remove legitimate
user-entered whitespace too. Ignoring changed textarea bodies would break the
accepted DATA-TABLE-2 edit/reset behavior: unrelated renders preserve dirty
edits, while a changed authored body updates the live control. The existing
`TextPatchIdentityAndDirtyTextarea` story covers that distinction. No projection
change is warranted by this investigation.

**Proposed decision, not an accepted contract:** add a shared, explicitly scoped
template layout policy and opt sample 1 into it (recommended), or make that
layout policy the default for all CEM templates. The proposed layout mode drops
only whitespace-only source trivia containing CR/LF and otherwise made of ASCII
spaces/tabs/line endings. It preserves inline spaces, Unicode spacing, nonempty
literal runs, expression values and rich-content bodies. An explicit preserve
mode would retain body whitespace, including opening layout; policy selection
would be declarative and inherited within the selected template scope. Compiler
handling must retain source evidence and stay independent of browser projection
and the XSLT compiler. Exact control syntax is part of implementation after the
scope/default decision.

An opt-in policy protects current templates, especially inline mixed content and
text generators, while allowing sample 1 to retain its multiline formatting.
A new default would fix sample 1 without a policy annotation, but changes the
rendered output of existing templates, including `pre`/`textarea` and multiline
inline separators. That compatibility choice needs user direction before
implementation, per the instruction to stop at decisions.

Investigation validation: 5 new characterization tests, 49 template-render tests
and 9 XSLT-output tests pass (63 total). Native lint passes with existing
warnings. These characterize current behavior; they do not claim the textarea
issue or the three-sample audit is fixed.
