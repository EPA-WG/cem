# Markdown article and site workflows, 2026-10-25

- [x] Extend the ordinary [Markdown site project](../../examples/projects/markdown-site/README.md)
  to version 1.1.0, with 21 resources and six configurations. Preserve its
  existing content, layouts, graph, resource IDs and seven expected site outputs.
  Reuse Documents and UI catalog discovery and the same Studio workspace.
- [x] Add generated-HTML inspection and a namespace-aware XPath article outline,
  an exact standalone body preview, explicit Markdown/YAML metadata validation,
  and a typed metadata failure/repair outside the site's content glob.
- [x] Repair the shared native import of passive generated HTML. Reproduce the
  image-reference failure with native load/import and CLI query regressions
  before changing the engine. Preserve strict authored HTML policy, retained
  facts, event-handler refusal and structural bounds.
- [x] Add native fixtures for all authored configurations, independent copies,
  exact output/assets, original failure coordinates, separate metadata/body
  ranges and shared partial origins. Add browser-worker and deployed acceptance
  for catalog discovery, saved commands/repair, native provenance navigation,
  independent copies and offline execution, retaining existing site/pin checks.
- [x] Update the checked content-type map, sample guide and shared import contract.
  Keep UI declarative and reuse the existing source editor, Results, provenance,
  tree and preview controls. No application or shared UI behavior is added.

## Source and projection contracts

The article outline reports its body heading, one image, three links and the
relative image reference. Its explicit XHTML query namespace is independent of
the Markdown input identity. Standalone HTML excludes YAML front matter; changing
the metadata title does not replace the body heading. The full site graph uses
typed metadata separately and supplies the relative CSS, images and linked pages.
An isolated HTML body does not acquire a site asset resolver.

`weight: heavy` deliberately fails with `cem.markdown.front_matter_invalid` at
line 2, column 1, UTF-8 byte offset 4. Replacing it with `weight: 1` repairs the
same saved resource. Native and worker diagnostics retain its project source URI.
The failure resource lives under `failures/`, outside `content/**/*.md`.

Ordinary Markdown inspection/query first lowers into generated HTML. Its retained
tree positions describe that HTML, not a Markdown source AST. The explicit build
graph carries original Markdown metadata/body spans and layout/partial frames.
Tests distinguish Unicode metadata titles from body headings and check original
byte ranges; unrelated Markdown resources are not claimed as article origins.
The deployed workflow uses the existing Results provenance control to open an
actual output's Markdown resource and select its native source range.

The import failure arose because conversion already deferred script/resource
policy diagnostics while retaining the same facts; later common-tree import
rejected those facts again. A private native `Source`/`PassiveProjection` profile
now survives the retained HTML owner. Only Markdown lowering and the existing
typed HTML recovery converter mint it. The common importer defers exactly those
two policy facts for inert inspection/query and preserves the facts themselves.
This grants no fetching or script execution. Authored source parsing, reimported
export bytes, handlers, and node/depth bounds keep their strict policy. Source
URIs and provenance annotations cannot grant projection admission.

## Verification

The selected native suites pass **38 tests**: three passive-projection/import
regressions, 16 existing HTML validation cases, 12 shared import/XPath cases,
four authored Markdown CLI workflows and three complete-site/provenance cases.

```sh
cargo test -p cem-ml --lib passive_projection
cargo test -p cem-ml --lib validation::html
cargo test -p cem-ml --test xpath_cem_tree
cargo test -p cem-ml-cli --test markdown_article_project
cargo test -p cem-ml-transform-cem-ql --test markdown_site_project
```

Fresh native CLI, shared engine, rendering and command-service WASM packages and
Studio build successfully. `@epa-wg/cem-studio:verify:feature-tour` passes all
32 manifest-selected schema examples and the authored configurations; the checked
inventory now contains 48 workflow mappings. Normal portable archives and catalog
metadata are regenerated.

Deployed output verification passes 218 declared assets and 316 files.
Deterministic builds match SHA-256
`41fc09280785b300462cc0a8214a2bb4d327c97df36f5edbcf5e5c85ea6411e7`.

All **23 Feature Tour browser cases pass**, including article commands, metadata
repair coordinates, exact site assets, independent edits and malformed-source
refusal. The final run takes 52.32 seconds. A first run exposed the browser
fixture helper's one-folder-deep path assumption; dependency declarations were
already correct. The helper now supplies project-root resource URIs, retaining
the nested article and explicit query resource instead of moving either file.

The dedicated `@epa-wg/cem-studio:verify:markdown-site` gate passes, including
its 25 prerequisites. Its report is
`dist/reports/cem-studio/markdown-site.json`. All 17 evidence flags are true:
catalog discovery/read-only originals, saved article commands and exact preview,
outline query, saved metadata repair, native output-to-source selection, relative
navigation, static assets, site edit/repair, shared layouts and provenance,
portable independent copies, entry artifacts, pinned builds, comparisons and
offline reopening/rebuilds. It retains all seven site outputs and the existing
pin/asset workflows instead of substituting a smaller article-only gate.

Authored Studio source/script lint, typecheck and whitespace checks pass. Reports,
binaries, generated screenshots and Vitest attachments are excluded from Git;
normal portable sample archives remain versioned.

## Preserved policies and next work

Saved originals and intentional failures remain unchanged after editing copies.
Acceptance uses an isolated browser profile and does not clear the user's saved
projects or drafts. No theme/storage keys or component styling change.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. Optional package
admission and recovery budgets are unchanged; these workflow checks use the
default complete deployment.

Continue with HTML/CEM-ML semantic and minimal component examples, SVG/MathML
inspection and preview, then CSS/SCSS/selector workflows in the active TODO.
