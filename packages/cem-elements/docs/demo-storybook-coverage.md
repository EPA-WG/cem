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
documents with the real helper. Both final aggregate attempts and a dedicated
full Storybook rerun pass 219/220, failing only the previously recorded
referrer-frame timeout. An earlier full run before the harness replacement
passed 220/220. The timeout's budget and semantics are unchanged; the aggregate
gate and six-sample audit sign-off remain open pending that investigation.
Lint retains its two existing warnings.
