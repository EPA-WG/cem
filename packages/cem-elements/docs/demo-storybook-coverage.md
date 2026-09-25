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
records these unannotated-template results with binding `text = "A"`. Escapes
below denote actual source characters, not CEM escape syntax.

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

**Decision accepted after investigation:** the user selected a shared,
explicitly scoped template layout policy and opting sample 1 into it on
2026-09-24. The accepted layout mode drops
only whitespace-only source trivia containing CR/LF and otherwise made of ASCII
spaces/tabs/line endings. It preserves inline spaces, Unicode spacing, nonempty
literal runs, expression values and rich-content bodies. An explicit preserve
mode retains body whitespace, including opening layout; policy selection
is declarative and inherited within the selected template scope. Compiler
handling must retain source evidence and stay independent of browser projection
and the XSLT compiler. The implementation uses `@cem:whitespace=layout|preserve`.

An opt-in policy protects current templates, especially inline mixed content and
text generators, while allowing sample 1 to retain its multiline formatting.
A new default would fix sample 1 without a policy annotation, but changes the
rendered output of existing templates, including `pre`/`textarea` and multiline
inline separators. The accepted opt-in leaves the default unchanged.

Investigation validation: 5 new characterization tests, 49 template-render tests
and 9 XSLT-output tests pass (63 total). Native lint passes with existing
warnings. These characterize current behavior; they do not claim the textarea
issue or the three-sample audit is fixed.

### Opt-in template whitespace and DOM-merge audit

The shared native compiler now consumes `@cem:whitespace=layout|preserve`
before lowering template instructions. The policy is lexical, inherited and
overridable on constructors and control nodes. The
[package contract](../../cem_ql/README.md#template-whitespace) defines the exact
ASCII layout runs, opening-trivia behavior, literal/rich/value preservation and
diagnostics. Unannotated templates and XSLT retain their previous behavior.
Suppressed layout remains zero-width text with its authored byte range in the
portable IR; dev artifact reload preserves that evidence, and prod reload
preserves the output semantics.

Sample 1 opts only its textarea into layout handling. Its closing brace stays
on the authored separate line, and the initial value is exactly `Hello world!`.
Its explanatory paragraph describes the distinction between template indentation
and typed whitespace. The source contract, teaching inventory and legacy mapping
record this correction without changing sample ownership or legends.

The three named Storybook steps check initial states, actual typing and Tab
commit, exact values, word/codepoint counts, empty/whitespace/Unicode input,
middle insertion and replacement, live control identity, focus, selection range
and backward selection direction. They settle the owning instance before reading
the current control and reject diagnostics. The independent gallery applies the
same three sample contracts in both standalone and source-loaded modes. Edits
must publish a new render revision before state assertions pass, even when word
counts are unchanged. The standalone page also checks two cards sharing a row
at 1280px and no horizontal overflow at 1280px/390px.

Validation passes all 734 native tests (9 existing ignored), including five new
policy regressions following the five investigation fixtures; 510 unit tests;
221 Storybook tests; and 31 standalone / 37 source-loaded documents. The
aggregate coverage gate, native/package lint and typecheck pass with existing
warnings. The existing Nx inputs cover the changed sources and fixtures.

This closes the three-sample DOM-merge audit. The broader per-sample audit stays
open; the next fixture is the 23-sample `external-template.html` audit, including
named/anonymous sources, fragment types, payloads, fallbacks, nested relative
resources and file previews.

## External-template behavior audit, 2026-09-24

The existing source-loaded owner now has a named step for every one of the 23
public legends and checks their exact order. The independent gallery uses the
same complete 23-sample contract in standalone and source-document modes.
Authored demos, supporting resources and story ownership remain unchanged.

The checks distinguish supplied versus default slot content, verify both
anonymous local instances, preserve SVG namespaces and internal `use`
references, and require exact missing-source/fragment fallback text. Storybook
also requires the corresponding loader diagnostics. Whole HTML keeps its SVG
and MathML namespaces while selected fragments exclude unrelated siblings.
Imported scripts are absent from rendered instances; the gallery's existing
console-error gate also detects execution of the source's `Stranger danger!`
script.

All three complete-island trees expose lifecycle namespaces and their distinct
payload evidence. Sample 4b retains the typed CEM-ML source and schema, as its
description promises. The named CEM-ML and named/anonymous XSLT payload trees
require four branches, their exact names/attributes and their own leaf text.
The embedded XSLT fragment requires only its basket and two fruit items.
Disclosure checks close and reopen the connected live element and verify its
identity. Nested templates require the expected output, exact library-relative
link/image URLs and a loaded image. Both file previews must display the exact
file contents with highlighted tokens and no demo output.

Validation passes all 510 unit tests, 221 Storybook tests and 31 standalone /
37 source-loaded documents. Aggregate coverage, lint and typecheck pass with
the two existing lint warnings. No shared transformation behavior or authored
fixture changed, and no new behavior decision was needed.

This completes the 23-sample audit. The broader per-sample audit remains open;
the next fixture covers all 11 `for-each.html` samples, including its two
external-file previews.

## For-each behavior audit, 2026-09-24

The source-loaded owner has 11 named steps in authored legend order. The
independent gallery applies one shared contract to both loading modes, including
exact external-file contents, highlighted tokens and empty preview demo output.
The checks cover all fruit/record fields, one-based positions, computed row
colors, nested table cells, payload order, grouped location parameters and both
HTTP result rows in each format. Checkbox cases exercise two complete
pointer/keyboard cycles, connected control identity and the dynamic table's
persistent header. Every product row must include its exact price: `$10`, `$25`
and `$15`. Storybook rejects diagnostics for every executable sample.

The user approved fixture corrections for the unset comparisons in samples 3,
6 and 9 and the missing final brace in sample 6. The checkbox guards now use
`?? false`, and the HTTP resource-state guards use `?? ""`. Small native render
fixtures demonstrate the absent-slice errors and clean guarded states, followed
by both boolean outcomes. Shared comparison semantics remain unchanged.

The separate approved browser investigation traced the blank prices to shared
HTML projection. Native CEM-ML renders the prices correctly even with their
authored newline, and adding the missing brace alone did not fix the browser.
The outer source-document projector consumed `${$product.price}` as its own
interpolation before the inner CEM-ML template could compile it.

Projection now preserves text, attributes and slots inside nested inert HTML
`template` contents for their eventual consumer. The template element's own
attributes still belong to the outer projection. Boundary regressions cover
both null and XHTML namespaces, nested templates, source-map retention, input
immutability, outer slot projection and ordinary behavior for foreign elements
named `template`. The regressions failed before the fix and pass afterward.

Validation passes all 51 native template-render tests, 513 package unit tests,
221 Storybook tests and 31 standalone / 37 source-loaded documents. Aggregate
coverage, native/package lint and typecheck pass with existing warnings. The
gallery also verifies a two-card row at 1280px and no overflow at 1280px/390px.
Existing Nx inputs cover the changed sources and fixtures.

This completes the 11-sample audit. The broader per-sample audit remains open.
The next fixture covers all five `form.html` samples: step advancement,
lifecycle validation, native/custom validity messages and form-associated DCE
values.

## Form audit checkpoint, 2026-09-24

The existing five-sample source-loaded Storybook interaction passes, but its
checks skip sample 2's initial confirmation prompt and diagnostic history.
The browser never renders `Select a confirmation method.`: its authored
`not(datadom.formData.lifecycle.confirmBy)` fails to compile with
`cem.ql.use_rust_boolean_ops`, which explicitly directs authors to prefix `!`.
The two password/SMS guards also compare an absent value with a string and
report `cem.ql.type_error` / `cem.ql.render.test_failed`.

A small native render fixture reproduces the missing prompt and all three
diagnostic categories. A candidate fixture correction uses
`(datadom.formData.lifecycle.confirmBy ?? "") == "password"` (and `"sms"`)
plus `!(datadom.formData.lifecycle.confirmBy ?? "")`. It produces the initial
prompt and the expected email/SMS/password branches without diagnostics.

All 52 native template-render tests, the existing form Storybook interaction
and native lint pass (existing warnings). Temporary browser diagnostic logging
was removed.

The authored demo remains unchanged. The five-sample audit stays open while
the user chooses fixture-only corrections versus shared syntax/comparison
work. Passing existing browser checks do not constitute sign-off on the
initial state or on complete form behavior.

## Form guard correction and follow-up findings, 2026-09-24

The user approved fixture-only correction of sample 2's confirmation guards.
Its password/SMS comparisons now use empty-string fallbacks, and its initial
prompt uses canonical prefix negation. All five samples have named Storybook steps;
sample 2 now checks its initial prompt and clean diagnostic history, the
ten-character username boundary, every confirmation method, pointer/keyboard
selection, password validation recovery, retained values across branch changes
and live form/username/radio identity. The independent gallery shares those
contracts across standalone and source-document loading. The other four
samples retain their existing contracts pending the remainder of the audit.

The expanded checks exposed two further decisions:

- Selecting Password after SMS with username `abcdefghij` inserts an empty
  required password control. Its native `validity.valid` is false, but the
  rendered form state still says `true` with no message. The strict browser
  assertion fails; subsequent password input refreshes the form correctly.
  Passing checkpoint tests cover the inserted control and later validation
  transitions, leaving the immediate form-state refresh explicitly open.
- Sample 4 initially displays length 4 for its empty control. A native
  characterization shows that an unset declared slice projects empty text but
  converts to the four-character string `null` inside `str:length`.
  `str:length(datadom.slices.email ?? "")` gives initial length 0 and preserves
  later empty, three/four-character and Unicode values. Its validity expression
  and displayed count need the same policy decision.

No shared runtime behavior or sample 4 markup has changed. The pending choices
are shared form-validation refresh versus recording the limitation, and a
sample 4 fallback versus shared null conversion. The five-sample audit remains
open. All 53 native template-render tests, 513 unit tests, 221 Storybook tests
and 31 standalone / 37 source-loaded documents pass. Aggregate coverage,
native/package lint and typecheck pass with existing warnings. The form page
also passes the desktop two-card layout and 1280px/390px overflow checks.
Existing Nx inputs cover the changed sources and fixtures.

## Form refresh and length checkpoint, 2026-09-24

The user approved shared validation refresh and fixture-only empty-string
fallbacks for sample 4. The runtime now recaptures its forms after committing
DOM and applying custom validity, then rerenders when form data or validation
has changed. This covers initial forms and conditional required controls
without another input event. Nested instances retain their form state and
event handlers. Readiness includes refresh renders; circular form rules stop
after eight refreshes with `cem-element.form_state_unstable` and can recover
after a later state change.

Four new browser regressions failed before the runtime fix. Five now pass,
covering DOM, processing-host and direct WASM rendering, conditional insertion
and removal, persisted form state, slice mirrors, live identity, rapid updates,
nested ownership and bounded circular rules. Sample 2 again requires immediate
invalid form state after Password replaces SMS, in Storybook and both gallery
modes. Storybook also expands sample 1's step/submit boundaries, sample 3's
native validity recovery and sample 5's repeated choices and native FormData.

Sample 4's displayed length now uses the approved fallback and starts at 0.
Applying it inside the custom-validity expression exposed a separate mismatch:
the browser returned `abc`, length 3, valid true. Its legacy evaluator replaces
a truthy `??` operand with boolean true, resolves missing paths as literal text
and uses UTF-16 string length. Native characterization instead preserves the
value and counts codepoints. Native `false ?? message` retains false, while the
existing browser validity dialect returns the message, so a shared fix also
needs an explicit compatibility choice. The validity-expression edit is
deferred; passing checks do not claim initial custom-message or Unicode
validity parity. All 54 native template-render tests pass.

The new decision is whether to fix shared custom-validity expression handling
or keep only the displayed-count correction. The five-sample audit remains
open for that decision, the remaining independent-gallery expansion and the
form-associated choice keyboard/submit contracts. In particular, Space on a
focused option currently follows the capability's dropdown open/commit
behavior; it does not directly activate that option as a native button would.

Checkpoint validation passes: 54 native template-render tests, 513 unit tests,
226 Storybook tests and 31 standalone / 37 source-loaded documents. Aggregate
coverage, native/package lint and typecheck pass with existing warnings. The
form page passes desktop two-card layout and 1280px/390px overflow checks.

## Shared validity expressions and form audit, 2026-09-24

The approved fix preserves selected fallback values, resolves missing data
paths as empty and counts Unicode codepoints in both `str:length` and legacy
`string-length`. Four new browser regressions failed before the fix and pass
afterward. They cover form and control messages across DOM, processing-host
and direct WASM rendering, absent/null/empty values, the three/four-character
boundary, Unicode, legacy path aliases, stable input identity and committed
validation state. The native fallback/codepoint characterization also passes.

The existing validity dialect keeps its boolean/message fallback contract:
`false ?? "message"` produces the message. Ordinary CEM-QL remains null/empty
coalescing. The package README now documents this compatibility boundary.
Sample 4 uses the approved empty-string fallback in both length expressions;
its initial custom message, empty recovery and Unicode boundaries have exact
Storybook and independent-gallery assertions.

The remaining form checks now cover sample 1's explicit Next boundary and
cancelled/allowed submission, sample 2's conditional-control validation,
sample 3's native missing/type-mismatch messages and recovery, and sample 5's
repeated values, native FormData and live host identity. Choice keyboard checks
use the shared dropdown contract: Space opens, an arrow changes the active
option, and Space commits. Mismatched choices cancel submission; matching
choices allow it. Test listeners observe cancellation before preventing the
demo's real GET navigation. The same gallery contracts run standalone and
source-loaded.

The empty-choice submission probe found a separate shared-focus decision.
Submission is correctly blocked, but Chromium reports both `firstFruit` and
`secondFruit` as invalid controls that are not focusable. The choice capability
passes `.cem-select__control` as its native validity anchor; this demo supplies
a nonfocusable fieldset. Passing checkpoint checks leave this initial-submit
case pending instead of accepting its console errors. The five-sample audit
remains open for the user's shared validation-focus decision.

Checkpoint validation passes: the native fallback/codepoint regression,
513 unit tests, 230 Storybook tests and 31 standalone / 37 source-loaded
documents. Aggregate coverage, lint and typecheck pass with the two existing
lint warnings. The form page passes desktop two-card layout and 1280px/390px
overflow checks. The next item is the shared invalid-submission focus decision.

## Shared validation focus and form audit, 2026-09-24

The user approved the shared focus fix. The choice capability now selects its
focusable control or the first usable HTML descendant of a wrapper. Disabled,
hidden, inert and non-HTML descendants are excluded. Authored focus surfaces,
including negative tabindex values, retain their behavior. The runtime updates
native validity and its anchor after DOM commits, covering the initial render
and replacement controls as well as the demo's nonfocusable fieldset.

Two browser regressions failed before the fix, including a focusable button
whose initial validity anchor was set before its DOM existed. They now exercise
native `checkValidity`, `reportValidity` and submission, disabled-anchor changes,
conditional control replacement, preserved selection/FormData, no focus movement
during rendering and no injected host/wrapper tab stops. Sample 5's Storybook
and both independent gallery modes require blocked empty submission and exact
focus on the first invalid fruit choice, then the second after the first is
filled, and the first again after clearing it. Mismatch cancellation and valid
submission remain covered. The authored demo is unchanged.

Validation passes: all 232 Storybook tests, including the production `cem-select`
consumer, 513 unit tests and 31 standalone / 37 source-loaded documents.
Aggregate coverage, lint and typecheck pass with the two existing lint warnings.
The form page passes desktop two-card layout and 1280px/390px overflow checks.
All five form samples are now audited. The next fixture covers the nine
`hex-grid.html` samples; the overall per-sample audit and separate browser-startup
investigations remain open.

## Hex-grid behavior audit, 2026-09-24

The source-loaded story now has nine named sample steps. Each checks exact link
names, titles, image alternatives, label text, resolved link/image URLs and
current-page state. Local images must finish in their expected load/error state
with the matching fallback visibility. Keyboard focus raises the label inside
its link, blur restores the authored presentation, live link identity survives
the interaction and produced instances have no diagnostics. The full external
React image URL remains an explicit URL-resolution case; local images provide
the deterministic successful-load checks.

The existing geometry and presentation checks remain in place. Responsive
coverage now exercises all four 5–4, 4–3, 3–2 and 2–1 honeycomb breakpoints,
including adjacent cells, staggered rows and helpers staying inside their cells.
Raised labels must also fit below the link's top edge. Percentage/fixed sizing,
alternating backgrounds, long labels, wrapper colors, image-button filters and
the current-page row retain their focused checks.

Both independent gallery modes now exercise real pointer hover, keyboard focus
and return to rest for every sample. They verify exact names and image states,
uniform/alternating backgrounds, isolated wrapper hover colors, stronger
image-button filters and wrapped text. Existing row checks retain native
Tab/Shift+Tab and Enter navigation. The standalone page additionally checks
desktop two-card layout and 1280px/390px overflow.

Validation passes all 232 Storybook tests, 513 unit tests and 31 standalone /
37 source-loaded documents. Aggregate coverage, lint and typecheck pass with
the two existing lint warnings. Desktop/mobile inspection confirms the expected
card layout and no page overflow. All nine samples are audited; the authored
demo is unchanged and no new behavior decision was needed. The next fixture
covers the seven `http-request.html` samples, including its four external-file
previews. The broader per-sample audit and separate startup investigations remain
open.

## HTTP-request audit checkpoint, 2026-09-24

The seven named Storybook steps cover the URL selector, Pokémon buttons,
request/response metadata and four exact external-file previews. The independent
standalone and source-document harnesses now share the same interaction contract.
Preset changes preserve the last requested URL and response until GET; malformed
JSON fails with no stale rows, a valid preset recovers, and an empty request
returns to idle. Direct input edits preserve live input/button identity and the
caret, restart the removed request through keyboard Enter, and load a second
typed URL through GET. All six sprite buttons retain their names, titles,
alternatives and exact URLs; their images load and their buttons accept focus.
The metadata case verifies all eight fields, exact source-relative resolution
and the compact link's complete title. Each preview displays the complete file
text with no demo output, including the intentionally invalid JSON prose.

One malformed request contributes exactly
`cem-element.http_request_parse_failed` to its instance's diagnostic history.
The other instances and source document have no diagnostics. Existing startup
tracing and frame-count waits remain unchanged: their separate timeout is still
unattributed, and this audit does not claim to resolve it.

Direct editing also exposed a shared live-input discrepancy. To reproduce, type
`./http-data-compact.json` into sample 0, then choose All records. The owning
render settles with these values in both Storybook and standalone Chromium:

| Surface | Value |
| --- | --- |
| Live input value | `./http-data-compact.json` |
| Input value attribute and defaultValue | `./http-data.json` |
| Selected URL output and GET button value | `./http-data.json` |
| Requested URL output / state before GET | empty / `idle` |
| Requested URL output / state after GET | `./http-data.json` / `loaded` |

The discrepancy persists for another second after render settlement, with the
same connected input and no diagnostics. GET loads the full alpha/beta response
while the field still displays compact. A screenshot and read-only runtime
observations confirmed the visible mismatch. The strict input-value assertion
fails; the demo and shared runtime are unchanged pending the user's decision.

Passing checkpoint tests keep the preset sequence before direct typing. They
explicitly leave preset-after-typing refresh open rather than asserting the
stale value as desired behavior. Checkpoint validation passes all 232 Storybook
tests, 513 unit tests and 31 standalone / 37 source-loaded documents. Aggregate
coverage, lint and typecheck pass with the two existing lint warnings. The HTTP
page passes desktop two-card layout and 1280px/390px overflow checks. The
seven-sample audit remains open at the shared input-value refresh decision in
`docs/todo.md`.

## Shared input-value refresh, 2026-09-24

The user approved fixing the shared renderer. Updating an input's `value`
attribute changes its default value, but after editing the browser retains a
separate dirty live value. Shared attribute application and removal now refresh
that live value when the rendered value changes. Unchanged values leave pending
edits intact; existing render focus/selection restoration retains the control
and clamps selection to the new length. Native sanitization and form reset
defaults remain in effect. File inputs and controls with reflected values keep
their native behavior.

Five browser regressions failed before the fix. Three cover DOM templates,
processing-host rendering and direct WASM rendering: a pending change-bound
edit survives an unrelated render, blur commits it, a preset returns to the
initial value, focused replacements preserve/clamp selection, and clearing
works. Input-bound typing also retains the caret during a middle insertion.
Two more regressions cover direct render plans and patch transactions, checking
text, number, range, color, date, checkbox, radio, hidden, button and file inputs
against native values, including removal, unchanged unbound edits and reset.
All five pass with no unexpected diagnostics.

The HTTP story and both independent gallery contracts again require typing the
compact URL and choosing All records before GET. The visible input, Selected
URL and GET value must all return to `./http-data.json`; the request remains
idle until GET, then loads the full response. The authored demo is unchanged.

The first full Storybook run passed 236/237 tests; the worker/fallback story
timed out waiting for its first worker instance's span within 120 frames,
before its update assertions. A full rerun with existing readiness tracing
enabled passed all 237 tests. Those trace hooks do not observe this story's
owning runtime, so the startup failure remains unattributed and has its own
open fixture in TODO. No wait was changed.

Validation passes all 237 Storybook tests on rerun, 513 unit tests and 31
standalone / 37 source-loaded documents. Aggregate coverage, lint and typecheck
pass with the two existing lint warnings. The HTTP page passes desktop two-card layout and
1280px/390px overflow checks. All seven HTTP samples are now audited. Next are
the twelve `local-storage.html` samples; the broader per-sample audit and
separate startup investigations remain open.

## Local-storage behavior audit, 2026-09-24

The source-loaded story now has twelve named steps. Every step checks that the
other samples' storage keys retain their prior values, then settles its owning
instances and checks diagnostic history. Only the JSON validation sample has
diagnostics: the deliberate invalid import and the previously accepted
whole-scope replacement on JSON root transitions. All other samples and the
source document retain empty histories.

Typed date/time forms now exercise recovery, empty input and return to their
initial value while retaining the same input. Date normalization preserves the
original timestamp in storage. Number checks distinguish raw strings from
coerced values. JSON checks cover object/array/scalar transitions, false, zero,
null, empty strings and empty containers, remove obsolete list rows, preserve
exact stored text and recover from invalid input. The initial-only reader keeps
its original output after writes while a fresh instance reads the new count.

The JSON basket checks every field, total and exact exported JSON, keyboard
activation, reset and an external zero-count update. The fruit watcher checks
all five counts after each button. Both text editors now require matching live
input values, slice outputs and storage after edits from either instance,
clearing, Unicode paste and an external write. Their input identities, focus
and selection remain stable. A diagnostic probe found that the test keyboard
helper splits an emoji into lone surrogates; the Unicode case uses paste to
insert the complete string, while ordinary edits still use keyboard typing.

Both independent gallery modes share the expanded exact-storage and output
contracts. The standalone lifecycle checks retain reload persistence for stored,
empty and missing values, and now also verify text edits in both directions
between two real tabs, including clearing. The twelve-card page passes desktop
two-card layout and 1280px/390px overflow checks in the focused gallery run.
The authored demo and shared runtime are unchanged.

The first full gate passed all 237 Storybook tests and 513 unit tests, but its
gallery stopped on hex-grid after Chromium reported `ERR_NETWORK_CHANGED` for
three local JavaScript modules. The source highlighter never mounted, so its
existing token-count check expired. This was separate from the focused
local-storage checks; no retry behavior, assertion or timeout was changed.

The unchanged full rerun passes all 31 standalone pages and 37 source-loaded
documents. Aggregate coverage, all 237 Storybook tests, 513 unit tests, lint and
typecheck pass with the two existing lint warnings. All twelve local-storage
samples are audited, with no new behavior decision needed. The next fixture
covers the three `location-element.html` samples. The broader per-sample audit
and separately tracked startup investigations remain open.

## Location-element behavior audit, 2026-09-24

Three named Storybook steps now check the complete ordered URL fields and query
rows. The live reader starts from the actual window URL, observes pushState and
replaceState, retains repeated parameters and preserves its article/input
identity. History length distinguishes adding an entry from replacing one.
The initial reader's five fields remain equal to its original capture after
history and hash changes. The external reader publishes the complete authored
URL and both parameter rows without changing the page location. All three
instances and the source document have empty diagnostic histories.

The story checks the native hash link's resolved URL and the GET form's method,
action, label, submit semantics and draft FormData. Typing keeps focus/caret and
does not navigate. Its cleanup restores the original URL and history state.
The original 180-frame startup predicates and read-only checkpoints remain;
this audit does not attribute or close the historical startup timeout.

The first expanded story attempted to click the hash link. A temporary probe
showed that Vitest installs `<base target="_parent">`, so the native link
navigated the parent test runner and closed its browser connection. The probe
is removed. Storybook checks resolution; both independent gallery modes execute
actual link navigation, Back/Forward and native GET without modifying the
authored link or the harness's base target.

Standalone and source-loaded checks now share all three sample contracts and
the navigation journey. Exact fields and parameter rows are checked after each
history movement. A query containing spaces and `&` is submitted by button,
then an empty query is submitted with Enter. Each GET must replace the document,
update both window readers, preserve the external reader and reset the form to
its authored default. The source harness remounts the authored document after
each real reload. Focused desktop two-card layout and 1280px/390px overflow
checks pass. The authored demo and shared runtime are unchanged.

Validation: the focused story and both focused gallery modes pass. The full
aggregate gate passes all 237 Storybook tests, 513 unit tests and 31 standalone /
37 source-loaded documents. Lint and typecheck pass with the two existing lint
warnings. All three location samples are audited without a new behavior
decision. The next fixture is the nine-cell scalar-referrer matrix in
`module-url-referrer.html`; the broader per-sample audit and separate startup
investigations remain open.

## Scalar-referrer matrix audit, 2026-09-24

The matrix's nine scalar src/referrer combinations resolve correctly in
Storybook, standalone Chromium and the independent source-document harness.
Storybook now names each referrer row, checks the exact ordered URL values and
row/column headings, verifies all nine displayed source controls and bindings,
and requires empty declaration, instance and source-host diagnostic histories.
Both gallery modes share the complete nine-cell contract, replacing five
partial-value checks. Request observation begins before document loading and
rejects target/referrer fetches; the page URL must remain unchanged.

The audit found that the mobile output was clipped despite a passing page-width
check. At 390px, a 372px card contained a roughly 893px standalone / 989px
source-loaded table. The user approved a fixture-only correction. The authored
matrix now constrains its table to the card with wrapping cells, explicit row/column
header scopes, multiline resource controls and source beginning at column 1.
All original inputs and full output URLs are preserved. Both focused gallery
modes verify table/cell containment at 1280px and 390px and access to the end of
the source panel's horizontal scroller. This intentionally wide, single-matrix
page is not subject to the ordinary gallery's two-card-row check.

A related-page link completes the existing parent-page cross-reference. Its
standalone target is correct, but the source-loaded check exposed an ordinary
HTML navigation limitation: both the existing `../index.html` link and new
`./module-url.html` link resolve relative to the host document, outside the
authored demo directory. Storybook reproduces the related-link mismatch. The
nine CEM module URLs remain source-relative and correct. The user approved
investigating shared source-link resolution. The loader retains the final
source URL as `resourceBaseUrl`, but `templateFromDocument`/`templateFromTarget`
import ordinary attributes unchanged. The existing loading contract governs
CEM resource controls and supplied no ordinary-link rebasing policy.

The focused story and both gallery modes pass after the approved presentation
correction. The full aggregate gate passes all 237 Storybook tests, 513 unit
tests and 31 standalone / 37 source-loaded documents. Lint and typecheck pass
with the two existing lint warnings. The navigation follow-up below closes the
remaining matrix audit; the next demo is `module-url.html`.

### Opt-in source navigation policy

The user approved an explicit per-declaration opt-in. Shared
`link-base="source"` resolves rendered HTML anchor and area hrefs against the
declaration's retained final source URL. Omitted policy and explicit
`link-base="document"` preserve the browser document's ordinary resolution.
Empty and fragment-only links retain current-page behavior. Inert template
contents, consumer payload and other URL attributes keep their existing
ownership and behavior; nested declarations opt in independently.

The native resolver runs after bindings evaluate and before patch comparison,
preserving node identity through dynamic href changes. DOM templates, direct
WASM, retained workers and main-thread fallback use the same resolver. Policy
and final source base participate in registration and processing identities;
renderer version 1.6.0 identifies the optional behavior. Invalid policy values
prevent registration, and URL-resolution errors fail the render without a
host-relative fallback. Neither import maps nor target fetches participate.

The matrix's Storybook and independent loading declarations enable source mode.
Both now require the existing index link and related module-URL link to resolve
inside the authored package/demo directories. No fixture href or harness URL
rewriting is used. Five browser stories cover default and explicit modes,
redirect bases, dynamic changes with stable anchors, consumer/inert ownership,
literal empty hrefs, native fragment clicks, processing fallback and invalid
policy. Four focused native regressions cover URL forms, escaping, preservation
and resolution failures. A unit regression checks retained artifact identities.
The focused location/matrix gallery passes in both modes, retaining the location
demo's native fragment behavior. Final aggregate validation passes all 242
Storybook tests, 514 unit tests and 31 standalone / 37 source-loaded documents.
Build, lint and typecheck pass with the two existing lint warnings. The matrix
audit is complete; the broader per-sample audit and separate startup
investigations remain open.

## Module-URL behavior audit, 2026-09-24

All 13 authored cases now have named Storybook steps. The audit compares the
shown import map with the authored resource entries, checks exact image/link
URLs and image decoding, and waits for the actual nested declaration content
in the relative, mapped and fragment examples. Component-local mappings,
wrapper overrides and all three descendant-referrer URLs are checked together.
Only the intentionally missing map entry reports
`cem-element.module_url_resolve_failed`; source-host, declaration and other
instance diagnostic histories are empty. The accepted `link-base="source"`
policy on the loading declarations keeps the index and four related-page
links in their authored directories.

Sample 6's bare comparison image previously requested `/lib-dir/Smiley.svg`
from the source harness. An existing server alias supplied the image at that
wrong destination. Removing the alias during characterization reproduced a
404 and zero natural image width. The user approved a fixture correction using
`cem-module-url`; the comparison image now resolves from the declaration source,
and the server alias is removed. Both browser modes require the exact resolved
URL and a decoded image. No shared image-source behavior changed.

At 390px, the import-map output measured about 634px and the node-referrer table
about 485px standalone / 422px source-loaded inside a 372px output. The approved
fixture presentation changes wrap the map preview, constrain the three-column
table, add column header semantics, wrap full URL disclosures and remove an
unnecessary figure margin. Both previously unlabeled images have alternatives.
The map preview's static CSS stays inside its authored sample, and the CEM-ML
styles stay with their owning declarations. All original resolution cases and
complete URL values remain available.

The independent verifier now shares all 13 sample contracts between modes.
Real Enter/Space input opens and closes URL disclosures, and the layout check
opens every disclosure at 1280px/390px before checking image, table and card
containment plus source-panel scroll reachability. Standalone retains its
12-card, two-column desktop gallery. Request observation rejects the old alias,
unresolved bare-specifier fetches and remote resolution-only target fetches;
the document URL remains unchanged. The focused story and both focused gallery
modes pass. The full aggregate gate passes 242 Storybook tests, 514 unit tests
and 31 standalone / 37 source-loaded documents. Build, lint and typecheck pass
with the two existing lint warnings. This fixture audit is complete. The next
behavior audit is the six-sample `npm-versions-demo.html`; broader coverage and
historical startup investigations remain open.


## NPM versions behavior audit, 2026-09-24

The five picker cases and shared JSON preview now have named Storybook steps.
Both browser modes check all four ordered versions and publication dates,
default and explicit selections, native label association, the projected label,
initially empty reflected values, repeated changes, propagated output and
sibling isolation. Selection checks compare the live select's identity and its
current value with the reflected host attribute. The source-loaded declarations
use the accepted `link-base="source"` policy to verify the index and all three
related-page destinations. The shared helper now distinguishes initialversion
defaults from explicit currentversion control; the deterministic data is unchanged.

The independent gallery shares the five picker contracts between standalone and
source-loaded modes; the sixth preview uses the real shared demo component.
Native Arrow/Home/End keys exercise the selects. All six cards and their controls
fit at 1280px and 390px in both modes, the standalone gallery retains a two-card
desktop row, and complete source lines remain reachable through their source-panel
scrollers. Source-host, loading declaration, sample declaration and produced-instance
diagnostic histories must stay empty. Storybook additionally checks Back/Forward,
clearing the hash and preservation of the latest user-selection output, restoring
both the original URL and history state afterward.

### Shared select refresh

The extended independent URL lifecycle passed once in both modes, including
native Enter/Space activation, Back/Forward and reload with a version deep link.
Further source-loaded runs exposed an intermittent stale select after the
existing sample checks and desktop/mobile presentation checks:

1. Activate “Set URL to 0.0.22,” then select 0.1.0.
2. Activate “Set URL to 0.0.25,” then select 0.1.0 again.
3. Press Enter on “Set URL to 0.0.25.”

One instrumented run still had this state after the unchanged 45-second timeout:

| Observable | Expected | Actual |
| --- | --- | --- |
| Page hash | `#version=0.0.25` | `#version=0.0.25` |
| Current-hash output | `#version=0.0.25` | `#version=0.0.25` |
| Native select value | `0.0.25` | `0.1.0` |
| Reflected value / last-selection output | `0.1.0` | `0.1.0` |

The user approved a shared fix. A native regression verifies the authored
picker emits the correct selected option for every external version and still
reflects the latest user selection separately. A second native contract verifies
currentversion emits a select value without changing option defaults; all six
native picker tests pass. Two focused browser regressions then reproduced the dirty-option failure
in direct render plans and worker patches, without HTTP or URL timing.

Shared DOM projection now reconciles each affected select after the complete
render transaction when selected presence changes or conditional branches
replace selected option nodes. It uses all explicit defaults for a multiple select and the last explicit default for a
single select; clearing all defaults follows native one-row/listbox behavior,
including disabled options and groups. Unchanged option identities and selected
attributes preserve user edits. Six browser regressions cover retained and
conditional option identities in direct plans, patches and fragment merges,
live selection, retained select identity, focus, reset defaults and absence
of generated input/change events. Existing input-refresh regressions
also pass. No fixture-specific synchronization or timeout change is used.

### Explicit control through superseded renders

The full demo still exposes a separate case when a new external hash arrives
before the preceding picker render commits. Just before the failing Enter
press, the hash, host initialversion and live value are all 0.1.0, but the option
defaults still select 0.0.25. The next hash requests 0.0.25 again, superseding
the unfinished render. Its final default matches the previously committed plan,
so unchanged-default preservation keeps the user's live 0.1.0 choice. The
rendered attributes are correct; this is not a lost host mutation. An extra
render-settlement round trip avoids the race, but is not a runtime fix.

The user approved explicit shared `select[value]` control. The URL picker now
passes `currentversion` to this binding, while other samples keep initialversion
and native user-choice preservation. The renderer applies explicit values after
options on every owning commit, including empty worker patches and unchanged
plans. Missing matches remain unselected; removing control restores option
defaults. Nested component ownership, focus and native reset defaults are preserved,
and no synthetic input/change events are emitted.

Three controlled projection regressions cover direct plans, worker patches and
fragment merges, including a dirty live choice with an unchanged final plan,
conditional option replacement, delayed options, empty and removed values,
child ownership and reset behavior. Three runtime stories cover DOM templates,
worker CEM-ML and direct WASM, including superseding slice updates. Together with
the six default-selection regressions, existing input checks and authored NPM
story, all 18 focused browser tests pass.

The independent lifecycle again returns to the previous external URL without
waiting for the child's intermediate selection render. It also verifies native
Enter/Space activation, Back/Forward, clearing, deep-link reload and last-user
selection output. Both focused gallery modes pass. The full gate passes 254
Storybook tests, 514 unit tests and 31 standalone / 37 source-loaded documents.
Build, typecheck and lint pass with the two existing warnings. The six-sample
NPM audit is complete. The next audit is the 12 samples in `scoped-css.html`;
broader coverage and historical startup investigations remain open.


## Scoped CSS behavior audit, 2026-09-25

All 12 authored cases now have named Storybook steps and share the complete
independent sample contracts between standalone and source-loaded modes. Checks
cover both private instances and the outside control, ordinary page cascade,
shared peers, separate private/shared artifacts, invalid and mismatched scopes,
instance payload styles, fragment and anonymous identities, real seeded
animations and external template CSS. The descendant checkbox is toggled off
and on with Space, checking color, text shadow, parent color, focus and identity.

The audit found three issues; the user approved shared module-default
investigation, fixture-only layout correction and the existing source-link
opt-in. Sample 7's root module compiled its slice binding but the renderer only
seeded defaults at the outer template level. It emitted two unknown-variable
errors and a failed condition in addition to its two intended dynamic-style
errors. Shared default initialization now opens the root module's declaration
prelude without descending into body content or named templates. The authored
sample keeps its module wrapper. Native regressions cover wrapped/unwrapped and
namespaced forms, empty and boolean values, host precedence, declaration
isolation, and exactly the two intended source-mapped style diagnostics.

Worker and direct-WASM browser regressions verify initial defaults, reflected
attribute defaults, later slice and empty-attribute updates, retained output
identity and sibling isolation. Storybook and the independent source harness
check exact diagnostic histories for every declaration and produced instance;
only the documented invalid-scope and dynamic-style cases may diagnose.

Sample 5's four flex items previously shrank to 57px each at a 390px viewport,
causing a 396px output inside a 372px card. Its sample-local page stylesheet now
uses a wrapping 12rem basis and border-box sizing. Navigation enables
`link-base="source"` on the page's loading declarations and sample 12's external
fragment consumer. All four destinations are checked in both modes.

All 77 focused native tests and eight focused browser stories pass. Both
independent gallery modes pass at 1280px and 390px, including source-panel
scroll reachability and the standalone two-card desktop row. The full gate
passes 256 Storybook tests, 514 unit tests and 31 standalone / 37 source-loaded
documents, plus build, lint and typecheck (two existing warnings). This fixture
audit is complete. The next audit is `set-url.html` (four samples); broader
coverage and historical startup investigations remain open.
