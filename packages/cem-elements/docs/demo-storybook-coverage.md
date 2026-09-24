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
for those assertions. Runtime behavior, existing waits and timing budgets are
unchanged. The separately approved root-gallery correction is described below.

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
  `MaterialIconLinkParity`. The aggregate remains failing until those separate
  cases are resolved; no test is skipped or weakened to make the inventory pass.
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
