# Demo teaching-point audit

Functional reference: `~/aWork/custom-element/demo`, reviewed locally. The
prototype supplies the lessons, not the current syntax or runtime contract.
CEM-ML is the preferred declaration format, rendering into light DOM from
instance-owned data islands. Explicit XSLT compatibility cases exercise the
existing bounded native lowering path.

An observer demo must let the reader change the observed state independently.
A writer demo must show a deliberate command and its real external result.
Rendering a resource's initial value alone does not demonstrate observation.
Plain HTML Storage, History, and attribute controls are external stimuli;
they do not implement the DCE's state projection or rendering.

Use emoji and Unicode for compact, recognizable actions and sample values:
`+🍒`, `↺🛒`, `→`, `−`, `∅ URL`, and fruit choices. Symbolic controls keep readable
accessible names and matching tooltips; fruit counters keep named terms.
Keep API names, URLs, typed values, validation messages, and other teaching
labels literal when replacing them would obscure the demonstrated behavior.

| Demo | Preserved or restored lesson |
| --- | --- |
| `local-storage.html` | External Storage API writes, removal, defaults, typed values, JSON baskets, live versus initial-only reads, reload, cross-tab updates, and two-way slice editors. |
| `attributes.html` | External `setAttribute`/`removeAttribute`, missing versus empty values, authoritative `select`, reflected title/value, and event-over-container precedence. Controls are visible in each sample's source; no hidden delegated writer. |
| `data-slices.html` | Defaults, change versus input, pointer event payloads, arithmetic, transformed values, multi-target events, attribute writes, checkboxes, and radio groups. Added the missing feature explanation and related links. |
| `dom-merge.html` | Textarea commits on change; text input updates live without losing focus/caret. Empty and whitespace-only inputs count as zero words. |
| `external-template.html` | Real anonymous declarations, reusable named declarations, independent SVG/HTML/MathML fragments, separate missing-source and missing-fragment fallbacks, complete CEMT islands, and compatibility XSLT trees. Expandable trees remain interactive. |
| `embed-1.html`, `embed-lib.html`, `lib-dir/embed-lib.html` | An external document really contains another DCE. Source-relative `file#fragment` lookup and relative file/image resolution remain visible in source and rendered links. |
| `for-each.html` | Sequences, position, record fields, nested tables, checkbox-controlled insertion/removal, payloads, location parameters, and JSON/XML resources. Explain why inert CEM-ML avoids HTML table-parser relocation. |
| `cell-overrides.html` | An inline cell template receives the original `name` node, finds its sibling `id` through retained-tree navigation, and renders a Pokémon image plus name using `{$node}` to reuse the name subtree and project alternative text. A separate `inspect` rule changes zero-stock content inside the base cell. Missing and repeated values keep the base grouping; sorting and source-loaded image resolution remain visible. A third lesson passes a number, date and native subtree into a child component, with a scoped hook showing text-only insertion. |
| `form.html` | A deliberate Next action advances the first form; native and custom validity, conditional confirmation controls, and form-associated DCE values remain intact. Valid native submissions navigate; use sample values only. |
| `http-request.html` | Draft URL versus GET, initial idle state, empty URL, malformed JSON failure, stale-result removal, recovery, response-driven Pokémon buttons, and request/response metadata. Fixtures remain local; existing version-pinned illustration assets are preserved. |
| `location-element.html` | Plain History API controls, native hash links and GET navigation feed a live reader; an initial-only reader stays unchanged until reload. Explicit `href` parses without navigating. |
| `set-url.html` | All six browser write methods, conditional activation, draft versus applied target, repeated commands, and multiple examples operating together. Triggered writes do not undo independent navigation. |
| `npm-versions-demo.html` | Default/preselected releases, dates, slots, propagated values, actual hash-to-picker parsing, immediate picker-to-hash writes, later external changes, and clearing the hash. |
| `module-url.html`, `module-url-referrer.html` | Relative, absolute and mapped URLs; missing mappings; nested declarations; local-map overrides and referrer contexts. Existing resolution and clickable-link examples are retained. |
| `scoped-css.html` | Private styles, outer cascade, shared scopes, inert per-instance style payloads, fragment/external styles, and native checkbox styling. Current native `@scope` semantics intentionally replace prototype CSS behavior. |
| `hex-grid.html` | Responsive linked image grid, hover/focus, instance sizing, wrappers, long labels and failed-image presentation. Case 9 restores the horizontal row with semantic current-page state, a visible ✓, public color hooks, and keyboard navigation. |
| `html-template.html`, `.xhtml` and supporting template documents | Loading/fragment fixtures, not gallery pages. Preserve document content, namespaces, and the inert imported-script check. |
| `functions/str.html` | Current-only string-function examples retained; no corresponding prototype page. |

Examples commented out in the prototype are not treated as implemented legacy
features. The current deterministic resource fixtures and native CSS scoping
remain authoritative; remote service responses and old XPath syntax are not
restored.

Nested library references retain explicit `file#fragment` URLs. In the current
runtime a bare `#fragment` searches the live document, not the imported library;
copying that prototype shorthand would make rendering depend on which other
examples have already loaded the library into the page.

## Complete local case inventory

[`legacy-demo-cases.json`](legacy-demo-cases.json) is the case-by-case audit of
all **32 files** in the local prototype's `demo/` directory. It pins each file's
SHA-256, records exact normalized legacy legends, separates variants inside a
card, and links each mapped lesson to current authored legends. The local
file list, fingerprints, 80 card identities, and four commented card identities
were checked against the checkout, not inferred from the current gallery.

| Inventory boundary | Result |
| --- | --- |
| 80 `html-demo-element` cards | Every card has a mapping and rationale; this includes the unnamed, non-executing form-data illustration. |
| Five live examples outside demo cards | Storage writer/watcher, two sized grids, the library's grid, and its horizontal row; all five are mapped. |
| Four commented external-template cards | Prototype ideas, excluded from runnable coverage. The tiny commented hex-grid payload is also not an active instance. |
| Supporting files | Libraries, assets, unused format variants, and unported XML viewers are classified separately. |

`covered` means the teaching point remains observable, not that legacy syntax
or incidental markup is copied. `migrated` records a meaningful change, such as
explicit compatibility annotations, native CSS scopes, local HTTP fixtures,
current data-island paths, or splitting one legacy card across focused examples.
Those split examples are not claims of identical composition. `missing` stays
linked to an open TODO rather than being counted as implemented.

The remaining work is explicit in `docs/todo.md`:

- `tree.xml` and `table.xml` are standalone `xml-stylesheet` viewer entry
  points, not DCE gallery cards. The completed
  [native migration review](xml-viewer-migration.md) separates implemented
  grouping/branch selection from **scaffold-only sorting**: the local stylesheet
  has a placeholder sort key and header annotations, but no interaction handler.
  First-row-only columns and omitted row text are additional legacy limits.
  XML-VIEW-1 through XML-VIEW-4 in TODO cover the typed inspection boundary,
  shared grouping/sorting capability, and subsequent tree/table viewers.
  The [multi-format table demo](../demo/data-table.html) now partially migrates
  the table lessons and extends them to CSV, YAML and JSON: editable sources,
  union columns, retained row text, stable native sorting and source-row
  selection. A shared CEMT viewer transforms the imported native CEM AST;
  generic query functions provide grouping and sorting, with no Rust table
  implementation. Imported match aspects switch notes to a tree and an
  IP-filter record to a preview form without changing the base viewer. The lossless
  tree/independent-branch selection work remains open; MSXML/EXSLT scripts and
  browser XSLT stay disabled.

The unreferenced `html-template.xml`, malformed `xhtml-template.xhtml`,
`template.xsl`, and `logo.png` are recorded as unused prototype variants/assets,
not silently treated as passing demos. The legacy reference to a root
`embed-lib.html` also points to a file absent from this checkout; the current
repository supplies its own deterministic library fixture.

`legacy-demo-case-map.spec.ts` validates current legend/fixture links, keeps
missing cases attached to open TODOs, and prevents unported/commented examples
from being reported as covered. CI does not read a developer's home directory;
updating the legacy baseline requires a fresh local comparison. Nx test inputs
include the manifest and the files this guard reads.

## Case-level loader follow-up

The first detailed comparison found two missing gallery variants, restored
before the complete inventory above. These are functional migrations, not
automatic dispatch of arbitrary legacy markup:

| Local legacy case | Current case in `external-template.html` | Contract decision |
| --- | --- | --- |
| Case 4, anonymous `src="tree.xsl"` beside the named instance | `7d. Anonymous external XSLT` | A separate anonymous declaration supplies its own payload to the same standalone XSLT resource path; expandable branches remain interactive. |
| Case 7, `html-template.xhtml#embedded-xsl` | `7e. Embedded XSLT fragment` using `html-template.xhtml#embedded-xslt` | Genuine stylesheet bytes live in a `template lang="custom-element-v0"` wrapper, the existing explicit compatibility boundary. ID selection alone does not enable XSLT execution. |

The old `embedded-xsl` ID remains a CEM-ML fixture for compatibility; it is not
evidence of XSLT execution. The new, separately named fragment declares its
XSLT and data-island namespaces locally and goes through the bounded Rust
lowering path. No new template-language heuristic or browser `XSLTProcessor`
path is introduced. CEM-ML remains the preferred authoring format.

Native fixtures in `packages/cem_ql/tests/external_xslt_demos.rs` cover both
authored stylesheet sources and unsupported-instruction diagnostics. Browser
coverage checks payload projection, disclosure toggling, missing fragments,
and the explicit language boundary. Both public cases are also part of the
standalone/source-loaded sample inventory.

The subsequent module-URL comparison restored legacy case 4 as
`4d. Mapped image and same-library fragment`. A single `lib-root/` prefix maps
the direct image and external fragment; the loaded library then resolves its
own image and nested component. Both Smiley images load and both links keep
their correct fragment targets. Native resolution is covered by
`packages/cem_ml/tests/module_url_demo.rs`, with source, Storybook, and
standalone/source-loaded demo checks for the authored composition.

The active `hex-grid-dce.html` horizontal strip is now case
`9. Horizontal row with a current page`. It reuses `cem-hex-grid layout="row"`
and `cem-hex-image-link`; no second grid implementation is needed. The legacy
`img[selected]` becomes `aria-current="page"` on an ordinary payload link.
A visible ✓ and public `--cem-hex-current-*` properties mark the current page
without conflating it with focus or creating a fake tab widget. Tab/Shift+Tab
preserve that state, Enter follows the focused link, and narrow rows scroll
inside their card. Native typed-record projection, scoped-CSS policy, browser
geometry, public-property isolation, and keyboard checks protect this lesson.

## Regression coverage

Source-contract units protect meaningful declaration shapes and case inventories.
Source-loaded Storybook interactions exercise the reader controls. The demo
fixture gate checks every authored case in standalone and imported documents,
including namespace-sensitive templates and local-storage lifecycle cases.
Desktop/mobile checks cover compact cards and page overflow.

Verified 2026-09-12 through Nx: 339 unit tests, 152 Storybook Chromium tests,
17 standalone pages, 23 source-loaded documents, lint, and typecheck. Layout
checks cover the gallery and landing page at 1440px and 390px viewport widths.
The final full Storybook rerun used `--maxWorkers=2`. Default parallel runs also
passed earlier, but later runs intermittently exhausted the unchanged scoped-CSS
frame wait and legacy icon-link two-second wait. Startup readiness remains an
open TODO; the final passing run does not establish default-run stability.
The hex-row follow-up also passes all 47 native `template_render` tests.
Its final full browser run uses two workers without a concurrent demo sweep;
earlier overlapping build-cache restores briefly removed shared WASM imports.
Repeated source-gallery mount/style lifecycle is a separate recorded fixture
TODO, not evidence provided by the single-mount gallery story.
The symbolic-label pass additionally checks accessible names, tooltips, fruit
selection, and Enter/Space activation. This headless host lacks emoji fonts;
Unicode text is asserted, but pictogram appearance requires an emoji-capable font.

The loader follow-up additionally passes the full native `cem_ql:test` suite,
including three authored-XSLT fixture tests, and native lint (with existing
source warnings). Both new cards fit side by side at 1440px; the page has no
horizontal overflow at 390px. Nx native test inputs include the external
stylesheet and XHTML library so fixture edits invalidate cached test results.

`location-element trigger` is an opt-in event-command gate; untriggered writers
retain their existing continuous behavior. Event payload revisions make repeated
equal-value clicks distinguishable. The trigger helper is unit-tested before
browser checks exercise multiple writers, pending drafts, external changes,
and repeat commands.
