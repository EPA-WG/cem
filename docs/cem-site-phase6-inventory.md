# CEM Site Phase 6 Inventory

Date: 2026-08-19

Status: inventory, content ownership, and application boundary accepted;
the initial root-wired shell now implements this boundary.

## Purpose

This inventory identifies the existing documentation, generated outputs,
examples, browser entry points, and resolved Nx ownership that a Phase 6 CEM
Site can reuse. It does not scaffold the site.

The site is a presentation and navigation consumer. It must not become a second
source for token values, component behavior, parser/schema contracts, release
facts, examples, or generated reports.

## Workspace Findings

The resolved Nx graph contains 41 projects. None owns a `serve` target or a CEM
Site build. The current browser-oriented targets are:

- `cem-elements:storybook` and `cem-elements:build-storybook`, which own the
  substrate story and browser-test surface;
- `@epa-wg/cem-theme:build:docs`, which compiles package source Markdown to
  copied Markdown plus XHTML under `packages/cem-theme/dist/`;
- `cem_ml:build:docs`, which compiles three schema-owned Markdown documents to
  copied Markdown plus XHTML and emits related JSON schemas;
- the root `yarn start` script, which serves the filesystem root and opens one
  selected HTML/XHTML path for local debugging rather than producing a
  deployable site;
- `@epa-wg/custom-element:start`, which runs that package's legacy development
  server and is not a root documentation application.

Vite is already a workspace development dependency, but `@nx/vite` is not
installed and no resolved Vite application target exists. The two checked-in
Vite configurations belong to package tests. This is not a production-build
gap: the accepted site boundary makes CEM-ML CLI the static application build
authority. Vite may later serve or test the generated output. The Yarn workspace
declaration is currently `packages/*`, so a package-managed project under
`apps/` still requires an explicit workspace-boundary change.

## Reusable Authored Sources

The inventory found 211 non-generated Markdown files outside archived todo
history. That number describes the review surface, not an instruction to publish
every file.

| Content family                  | Current owner                                                                                                  | Site treatment                                                                                                                                        |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Product overview and navigation | `README.md`, `docs/index.md`, `roadmap.md`                                                                     | Render or summarize by reference; keep detailed execution history out of default navigation.                                                          |
| Active planning                 | `docs/todo.md`                                                                                                 | Optional project-status view; never treat it as product documentation.                                                                                |
| Token semantics                 | `packages/cem-theme/src/lib/tokens/*.md`                                                                       | Canonical. Render from the source documents or their owner-generated XHTML; never infer or write values from CSS, DTCG, native, or Figma projections. |
| Theme architecture              | `packages/cem-theme/README.md`, `packages/cem-theme/docs/*.md`                                                 | Render as package-owned guides.                                                                                                                       |
| Component semantics             | `docs/component-mvp.md`, `packages/cem-components/README.md`, `packages/cem-components/docs/*.md`              | Render as the component reference, behavior, state, keyboard, and accessibility source.                                                               |
| Substrate/runtime semantics     | `packages/cem-elements/README.md`, `packages/cem-elements/docs/*.md`, active `docs/cem-element-*.md` contracts | Render by reference; preserve the distinction between the bare substrate and styled components.                                                       |
| CEM-ML/CEM-QL contracts         | Active `docs/cem-ml-*.md`, `docs/cem-ql-*.md`, `packages/cem_ml/schema/**/*.md`, and package READMEs           | Render authored contracts and generated schemas without copying their normative text into site-owned files.                                           |
| Release facts                   | Root and package `CHANGELOG.md` files plus active release documents under `docs/`                              | Build release pages from these owners and generated release metadata.                                                                                 |
| Historical rationale            | `docs/archive/`, temporary decision notes, and retained history documents                                      | Exclude from primary navigation and search unless explicitly promoted as historical material.                                                         |

Snapshot counts useful for planning include 70 active root `docs/*.md` files, 20
theme source/architecture Markdown files, 39 component package Markdown files,
and 43 CEM-ML/CLI package Markdown files. Publication requires an explicit
allowlist or manifest; directory-wide copying would expose internal and temporary
material.

## Generated Projections

Generated outputs remain owned by the target that creates them and must be
regenerated before site assembly.

| Projection                | Owner target                                    | Current useful outputs                                                         | Site rule                                                                                                    |
| ------------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------ |
| Theme source docs         | `@epa-wg/cem-theme:build:docs`                  | Copied Markdown and XHTML under `packages/cem-theme/dist/lib/`                 | Consume generated output or source through a manifest; do not patch `dist/`.                                 |
| Token browser data        | `@epa-wg/cem-theme:build:tokens`                | Public `cem.tokens.json`, voice tokens, TypeScript metadata, and token reports | Prefer public package export subpaths and report contracts. Debug intermediate/resolved JSON stays excluded. |
| Platform snippets/reports | `@epa-wg/cem-theme:build:token-platforms`       | Per-mode JSON, Swift, Android XML/Kotlin, and platform reports                 | Present generated snippets and reports read-only.                                                            |
| CEM-ML schema docs        | `cem_ml:build:docs` and schema-artifact targets | Copied Markdown/XHTML plus run, report, and observability schemas              | Consume declared outputs; do not duplicate schema definitions in the site.                                   |
| Component verification    | `@epa-wg/cem-components` verification targets   | State-matrix and inventory JSON/Markdown reports                               | Use reports as build inputs or status evidence, not authored prose.                                          |
| Browser stories           | `cem-elements:build-storybook`                  | `packages/cem-elements/storybook-static/`                                      | Link or embed as a separately owned artifact; do not make Storybook the site router.                         |

No Typedoc, custom-elements manifest, or equivalent generated API-documentation
target exists. Phase 6 therefore needs either an explicit API projection target
or a deliberately scoped authored-reference policy; it cannot claim a generated
API browser from the current workspace.

The existing generic Markdown compiler remains a useful package-local producer,
but it is not the site build authority. It processes one project's source tree,
rewrites Markdown links to XHTML, injects CDN-hosted Prism assets, and assumes a
relative `index.css`. The CEM-ML CLI graph must instead own cross-package content
conversion, navigation/layout composition, dependency resolution, inlining or
emission, import rewriting, fingerprinting, and clean deployment assembly.

## Examples And Browser Entry Points

The inspected example areas contain 142 files across HTML/XHTML, CEM-ML, XML,
JSON, CSS, native language, image, and documentation formats.

- `examples/cem-ml/` owns five canonical CEM-ML product fixtures plus transform
  and schema examples; `examples/semantic/` is their secondary HTML parity
  surface.
- `packages/cem-components/examples/` owns eight authored workflow fragments.
  The executable counterparts under `packages/cem-components/tests/workflows/`
  remain test fixtures and should be referenced rather than republished as source
  documents.
- `packages/cem-elements/index.html`, its demo directory, and Storybook own
  substrate demonstrations. Their workspace-relative imports make them
  development inputs, not portable deployment pages.
- `packages/custom-element/index.html` and its demos are compatibility/reference
  material, not the Phase 6 runtime baseline.
- `examples/ios/` and `examples/android/` provide native token consumption
  examples.
- Figma examples remain deferred with Phases 10 and 11 and are not Phase 6 site
  update inputs.

The site should wrap or link these owners through explicit manifest entries. It
should not copy example markup into page components, because copied examples
would drift away from executable fixture gates.

## Content Ownership Contract

1. The site owns only navigation, route metadata, layout, presentation
   components, search indexes, and build/deployment assembly.
2. Markdown token specifications under
   `packages/cem-theme/src/lib/tokens/` remain canonical. Every CSS, DTCG,
   TypeScript, native, and Figma value shown by the site is a downstream
   projection.
3. Authored package and contract documentation stays in its current owning
   package or `docs/` path. The site references or renders it; edits flow back to
   that owner.
4. Generated files are immutable site inputs regenerated through their owning Nx
   targets. Site code never patches `dist/` or commits copied generated prose as
   a new authority.
5. Examples remain owned by their fixture/example directories and verification
   targets. Site wrappers identify the source path and, where applicable, the Nx
   gate that proves the example.
6. Publication and search use an explicit checked-in content manifest. Archive,
   temporary, internal audit, and debug artifacts are excluded by default.
7. JavaScript runtime dependencies enter the CEM-ML graph through paired,
   schema-owned module maps. Authors choose exact distributable `.js`/`.mjs`
   files, including public package subpath files under `node_modules`, and exact
   app-relative destination URLs. CEM-ML emits only those opaque typed assets
   and rewrites the browser import map; it does not discover package exports or
   transitive imports. Deployed pages never retain `node_modules` or repository
   filesystem paths.
8. Live Figma state is outside Phase 6. The site may describe the deferred
   workflow but must not trigger or require a Figma refresh.

## Drift And Missing Gates

- The root README still says the release flow refreshes Figma afterwards, which
  conflicts with the new Phase 10 deferral and needs a later documentation
  correction.
- `docs/index.md` is manually curated even though it describes itself as a map of
  every document, report, and example. There is no drift verifier against the
  publishable source set.
- CI has no site build, deployment, link, search-index, accessibility, or clean
  artifact gate, and no GitHub Pages/site deployment workflow exists.
- The root development server exposes the filesystem root and opens a browser;
  it is appropriate for local debugging but unsuitable as a deployment or CI
  boundary.
- Component workflow examples are fragments and need a site-owned wrapper whose
  CEM-ML build graph maps exact production JavaScript files without duplicating
  the fragment or preserving workspace-relative imports.

## Application Boundary Decision

Accepted: create a dedicated `apps/cem-site` Nx application with static
deployment output and CEM/custom-element components for its shell. The project
owns route metadata, the publication manifest, layouts, navigation, search,
accessibility gates, and deployment configuration. Package-local Markdown
compilation remains independent.

The build boundary is deliberately split by responsibility:

1. Nx invokes the CEM-ML CLI build graph and declares its source, module-map,
   package/lockfile, template, and generated-report inputs plus the clean site
   output. A planning-only CEM-ML aggregate module-asset SHA-256 is an Nx runtime
   input, so dynamically declared npm bytes participate without duplicating map
   resolution in JavaScript. Nx schedules and caches the task; it does not define
   web transformation semantics.
2. Dedicated CEM-ML source and destination module maps explicitly pair exact
   bare npm specifiers with `.js`/`.mjs` source files and app-relative deployed
   URLs. Source values may enter `node_modules`; destination values never do.
   The CEM-ML module map is the build-time authority and the HTML browser import
   map is only its destination projection.
3. Each declared JavaScript file is an opaque typed graph import/export. CEM-ML
   copies its bytes beside the HTML export and does not parse JavaScript, select
   package exports, discover transitive imports, or copy undeclared files. The
   authored map therefore owns completeness.
4. The produced directory contains the explicitly declared JavaScript runtime
   set and remains runnable without those source files or `node_modules`, provided
   the authored map declares every module the application loads. Asset handling
   is native graph transformation, not a post-build copy exception.
5. Vite may be added as a development server or browser-test harness over the
   generated directory, but it is not the production build authority and does
   not own dependency bundling.

The resulting `apps/cem-site` application keeps an explicit checked-in
route/source allowlist and publication graph, uses the native Markdown → HTML →
recovered DOM → CEMT pipeline for shared layout composition, and consumes the
generated CEM-ML transform-config Markdown from `cem_ml:build:docs`. Its first
package-authored surfaces render the `@epa-wg/cem-ml-cli` browser/Node usage
guide and `@epa-wg/cem-ml` WASM runtime reference directly from their owning
README files. The dedicated module-map contract and deterministic
resolved-read/digest manifest remain the required path for later interactive
JavaScript; the current static surfaces intentionally ship no runtime
JavaScript.

## Evidence Commands

The inventory used resolved Nx configuration rather than only checked-in
`project.json` files:

```bash
yarn nx show projects --json
yarn nx show projects --withTarget serve --json
yarn nx show projects --withTarget storybook --json
yarn nx show projects --withTarget build-storybook --json
yarn nx show projects --withTarget build:docs --json
yarn nx show project <project> --json
yarn nx graph --print
```

File discovery used `rg --files` with generated and archive exclusions for
authored-source counts, then explicit generated directories for projection
inspection.
