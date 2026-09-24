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
waits. The approved root-gallery and fixture corrections are described below.

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
