# Passive components and inert payloads

All 55 public components now have canonical XHTML/CEM-ML declarations,
colocated CSF Next plays, property pages and five-theme galleries. The final
nine owners complete the declarative migration. The closed migration inventory
requires zero legacy tags and zero authored JavaScript/TypeScript outside the
colocated stories. There is no component package JavaScript entry or installer;
load `components/<tag>` or `components.xhtml#<tag>` through `cem-elements`.
Runtime APIs and event-detail types come from `@epa-wg/cem-elements`.

| Owner | Contract |
| --- | --- |
| `cem-option` | Rich static choice payload: required `value`, optional `label`, `selected`, `disabled`. Select/autocomplete/timepicker own interaction and native form semantics. |
| `cem-option-group` | Required group `label`, optional `disabled`, direct option children. The consuming choice controller owns group roles and inherited disabled state. |
| `cem-tree-item` | Required unique `value` and `label`, optional `expandable`, `disabled`, `loading`; recursive item payload. Tree owns focus, hierarchy and expansion. |
| `cem-tab` | Required unique `value` and `label`, optional `disabled`; default slot is retained panel content. Tabs owns buttons and reciprocal panels. |
| `cem-step` | Required unique `value` and `label`; `completed`, `editable`, `optional`, `invalid`, `disabled` are application facts. Stepper owns navigation and regions. |
| `cem-badge` | Visible default-slot label, falling back to `label` (`Badge`). `tone` accepts info/success/warning/danger; other values use info styling. No live region or action. |
| `cem-avatar` | Named image identity (`label`, default `Avatar`) with `initials` fallback or projected media. No image fetching, crop or activation behavior. Supply a meaningful identity label. |
| `cem-media-preview` | Native named figure, default media slot and named caption slot. Both fall back to `label` (`Media preview`). Authored media owns its alternative text and native behavior. Explicit text/document/message modes consume `value` as described below. |
| `cem-app-bar` | Named banner (`label`, default `Application`), named title slot and default global-action slot. Native/projected controls keep their own focus and action semantics. |

“Inert payload” means the declaration adds no interaction, selection or focus
owner. This describes the default media mode; explicit output modes below expose
native text and document focus. It does not impose the HTML `inert` attribute on tab/step panel content.
Standalone payload declarations render a passive wrapper; supported controls
consume the authored payload before rendering their own native owners. Host
`hidden` and `dir` retain browser presence and inheritance semantics. Component
internals expose `part="root"`; media also exposes `media`/`caption`, and app bar
exposes `title`/`actions`. Existing compatibility classes remain available.

All-disabled listboxes have no active option. The reusable choice controller
now skips active-row lookup when the active ID is empty, preventing an invalid
`#` selector. The original failing consumer fixture and worker/fallback plays
cover this boundary.

## CEM principle audit

- Semantic intent: payload wrappers consume UI typography only; their consuming
  controls own selection/navigation tokens. Badge tone uses paired trust, calm,
  enthusiasm or danger palettes and remains meaningful through visible text.
  Identity, media and application context use the comfort/text pair.
- Bounded variation: four badge tones with an info fallback; payload status
  facts retain their existing consumer contracts. No component-local public
  token defaults, aliases or new token families are introduced.
- Accessibility: default payload modes add no tab stops or live regions. Avatar names, native figure
  captions/media alternatives, banner naming and consumer-owned roles remain
  explicit. Hidden preserves presence semantics. All-disabled choices remain
  valid listboxes with no active descendant.
- Cross-dimension harmony: D3 avatar/media endpoints own their geometry; badge
  uses the documented control bend. D8 initialism typography owns avatar
  initials; other compact UI text uses UI typography. D1 container inset frames
  the badge's text surface and app banner; related media/caption and grouped
  global actions use their corresponding gaps. D5 boundary strokes separate
  media/application surfaces. No action-state styling is assigned to passive
  wrappers.
- Theme survivability and ownership: all five theme modes inherit public token
  overrides; avatar/media/badge bend overrides are exercised. Forced colors use
  automatic adjustment and Canvas/CanvasText surfaces; no component introduces
  motion. Theme definitions and unused release tokens remain intact.

The governing sources are [CEM principles](../../cem-theme/src/lib/tokens/index.md#core-cem-principles),
[D0](../../cem-theme/src/lib/tokens/cem-colors.md),
[D1](../../cem-theme/src/lib/tokens/cem-dimension.md),
[D3](../../cem-theme/src/lib/tokens/cem-shape.md),
[D5](../../cem-theme/src/lib/tokens/cem-stroke.md) and
[D8](../../cem-theme/src/lib/tokens/cem-voice-fonts-typography.md).

## Evidence ownership and limits

The seven former workflow cases retain every markup and DOM assertion in
colocated card, avatar, app-bar and sheet stories. Generic native form, event,
accessible-name, visual snapshot and screenshot assertions live in the shared
runtime's `native-substrate.stories.ts`; the component harness and registry
installer tests are retired with their APIs. Workspace-owned build/publication
and forced-color tools live in `tools/cem-components/`.

Source, bundle and isolated-package delivery gates exercise property updates,
retained owners, public attributes, canonical source, five-theme galleries and
forced colors. Automated browser evidence does not establish physical-device
or spoken assistive-technology coverage. Studio/Site visible UI migration and
external Figma review remain separate tasks. The original migration updated
Studio's component resource-loading dependency; its subsequent preview adoption
uses the explicit modes below.

## Explicit output preview modes

`cem-media-preview` retains its named native figure and the existing `media`
mode default/caption slots and label fallbacks. Explicit modes read `value`:

- `text` renders exact literal text in a focusable, wrapping, scrollable `pre`.
  Presence of `truncated` adds “(truncated)” to its accessible name; it never
  edits the source. Like other presence attributes, `truncated="false"` is present.
- `document` renders a titled native iframe with fixed empty `sandbox` and `allow`,
  `referrerpolicy="no-referrer"` and lazy loading. Its document starts with a
  deny-all CSP, including scripts, styles, network, frames, forms and media.
  Host attributes cannot widen that policy. Document content has an opaque origin;
  the component's CEM theme styles the outer surface, not the isolated document.
- `live` interprets `value` as a CEM-ML template through the shared `live-preview`
  capability. `source-uri` names the template owner. Optional `resource-set` is
  an explicit JSON transport envelope: `{ "resources": [{ "uri": "...",
  "contentType": "...", "text": "..." }], "styles": ["..."] }`. Its text bodies
  enter the normal native import/declaration boundaries. There is no ambient fetch.
  Optional `declarations: [{ tag, uri }]` registers components and preflights their
  CSS before the view mounts; dynamic `cem-element` output is rejected. Output
  is limited to HTML; foreign namespaces such as SVG/MathML are not admitted.
  A parent-owned runtime renders into the frame's **light DOM**. The frame has
  `sandbox="allow-same-origin"` so the parent can manage it; scripts remain
  forbidden by both sandbox and CSP. This is distinct from document mode's opaque
  sandbox. Navigation elements/attributes are rejected before initial or incremental
  publication; forms cannot navigate. Browser storage, location controls, repository
  services, nested live previews and stylesheet fetches are disabled. Supplied CSS
  stays within the frame, which inherits no page theme. Form values and slices are
  transient and are destroyed on source/resource replacement or disconnect.
  The frame fits its content up to 4096 CSS pixels, then scrolls. A polite status
  reports readiness and errors. Limits: 256 KiB template, 128 resources, 16 MiB total.
- `message` renders literal explanatory text, for example why bytes require a
  download. Slots remain owned by media mode; unknown modes fall back to media.

- `artifact-tree` previews a complete immutable build output set. `artifact-path`
  selects an HTML file, optionally followed by a query and fragment. `resource-set`
  carries the realm-local opaque handle returned by `registerCemArtifactTree(artifacts)` from
  `@epa-wg/cem-elements`; each artifact has a relative `path`, declared
  `contentType` and exact `Uint8Array bytes`. Registration clones and admits the
  entire set before publishing its handle (128 files / 16 MiB aggregate).
  Dispose the registration when its run is replaced or released; this revokes all
  associated mounts and object URLs. Independent registrations never share assets.
  Small declarative examples can instead supply `{ "artifacts": [{ "path":
  "index.html", "contentType": "text/html", "text": "<h1>Example</h1>" }] }`.
  Each literal entry supplies exactly one of `text` or `base64` (for image bytes).
  `value` and live CEM's `source-uri` are unused in this mode.

  The shared capability manages a script-free frame with `allow-same-origin` for
  parent access, fixed CSP, no referrer and no CEM runtime or component registry.
  HTML is parsed in an inert template before admission. Scripts, refresh/base,
  nested browsing contexts, unsupported foreign markup and active URL/event
  attributes are removed; forms cannot submit. Inline static SVG and Presentation
  MathML use the shared [bounded foreign-content policy](../../cem-elements/README.md#inline-svg-and-mathml-artifacts),
  preserving admitted namespaces while excluding scripts, integration escapes,
  foreign styles and resource URLs. Relative stylesheet links, inline/style-block
  CSS, CSS imports and images resolve only against this run. The native retained
  CSS importer owns URL decoding and remapping, including escaped/static
  `image-set` references and import conditions; unsupported CSS grammar is
  reported and omitted. CSS retains the shared 64-level / 4096-event native import
  bounds. Allowed image MIME types are PNG, JPEG, GIF, WebP, AVIF,
  BMP, icon and SVG-as-image. The loader replaces references with run-owned object
  URLs; CSP permits blob styles/images and inline CSS while denying all network
  sources. [CSP source syntax](https://www.w3.org/TR/CSP/#framework-directive-source-list)
  admits the blob scheme, rather than individual blob URL expressions. Fonts,
  media, `srcset`, authored data/blob URLs and
  ambient network requests are outside this preview policy.

  Clicking a link never navigates the browser. It emits bubbling
  `cem-preview-navigate` with an own scalar `detail.value`: the artifact's path
  plus query/fragment. A trailing-slash link resolves to that directory's
  `index.html`. The parent chooses `artifact-path`, allowing URL/history ownership;
  query/fragment changes retain the current document and a same-document fragment
  also scrolls locally. Missing links still emit their
  requested path so the parent can display a missing selection. Missing assets,
  blocked external references and cyclic imports report polite status and never
  fall back to source resources or another run. Selected HTML retains the 256 KiB
  inline limit. Exact downloads remain the caller's original verified bytes.

Transport services own decoding, content-type selection and byte budgets; the
component never guesses content types or fetches external source. Studio retains
its 256 KiB inline limit and exact-byte download service. The native declarative
renderer owns content and frame identity: unrelated updates retain focus and do
not write an unchanged `srcdoc`. The declaration contains no JavaScript; live lifecycle, resource admission and realm handling belong to `cem-elements`.

The figure uses D0 comfort colors, D1 related gaps, D3 media bend and D5 boundary
strokes. Text uses the complete D6 script typography role and D1 container inset;
focus uses D5 stroke/offset/zebra tokens. All tokens stay theme-owned and inherit.
The text viewport's `60vb` maximum is a structural scrolling bound; it does not
substitute a typography measure or arbitrary dimension token. Frames retain their
native default block size and fit the container inline axis. Forced colors use
Canvas/CanvasText/Highlight and reduced motion requires no animated behavior.

`cem-media-preview[fit="fill"]` fills a bounded parent for Studio/workspace previews.
Text remains scrollable and document/live/artifact-tree frames retain their existing sandbox and
CSP. Omitted or unsupported `fit` values retain natural sizing.
