# CEM Site

This Nx application publishes a static site through the native CEM-ML CLI. Its
checked-in `site.cem` graph is the publication manifest: every source, transform,
route, and output is explicit.

The site's primary UI authoring contract is the repository's
[declarative UI principle](../../docs/declarative-ui-principle.md). Visible
markup, component composition, interaction bindings, and UI state projection
must be XHTML/CEM-ML. Site JavaScript may provide non-UI host services only; it
must not render/mutate visible UI or attach app-local UI behavior. A missing or
overly verbose CEM-ML capability hard-stops route UI work until a reusable
declarative capability is implemented in `cem-elements`. The `cem-elements`
runtime and its JavaScript are the sufficient imperative substrate; Site must
not duplicate that machinery. A component needed by Site must first follow the
proven `cem-components/src/components/cem-select/` XHTML plus CSF Next
`play`-test pattern.
The accepted CSS target installs declaration CSS once through native `@scope`.
Site may style components only through documented tags, public `scope="name"`
groups, projected `slot="name"` roots, component-owned `part="name"` hooks, and
custom properties. It must not select data-island or render-identity markers.
Site follows the normative
[CEM light-DOM CSS scoping rules](../../docs/cem-ml-uid-and-scoped-css-design.md).
That behavior remains a migration target until its active todo gate passes.

The checked-in `site.routes.json` is the audited publication and search
allowlist. It
records each route or resource, its canonical source, owning Nx project, and
the upstream target required for generated inputs. Every entry also declares a
content role and relative-link policy. Authored sources have no upstream
generation target and participate directly in the site build hash.
Verification resolves the current Nx project graph, derives the unique deepest
project root for every source, and rejects a declared owner that does not match.
Generated routes additionally require an existing target on that same owner and
an exact scheduling edge from `cem-site:build`.

Run `yarn nx run cem-site:build` to produce `dist/apps/cem-site`, or run
`yarn nx run cem-site:verify` to build and verify the route allowlist, links,
generated-document provenance, transform report, source-map sidecars, token
catalog traceability, component-catalog ownership, and deterministic clean
publication. The focused `cem-site:verify:determinism` target executes the same
native build twice after removing the output directory and records a per-file
and aggregate SHA-256 report under `dist/reports/cem-site/determinism.json`.
The cached `cem-site:verify:production` target then serves that exact output and
crawls every page in Chromium. It rejects failed same-origin responses,
page/console/runtime errors, landmark and heading drift, inaccessible names,
broken ARIA or fragment references, missing keyboard focus indicators, and any
file that differs from the clean deterministic inventory.

Authored content follows a Hugo-like folder hierarchy under `content/`. Shared
semantic HTML composition lives in `layouts/page.cemt`. Cross-package generated
documents are read only from declared upstream Nx outputs; the application does
not duplicate or patch their source text. Archive, planning, temporary, Figma,
and debug-token paths remain excluded from the publication allowlist.

References use one of two explicit roles. `generated-reference` is reserved for
an artifact with a scheduled upstream Nx generator; `authored-reference` renders
the current owner's Markdown and must keep `upstreamTarget` null. The site does
not label authored documentation as generated API output. Package-owned
repository-relative links are rewritten by the shared CEMT layout to their
canonical `develop` source URLs, while site-owned root-relative navigation stays
on the static site.

The authored route set includes the canonical CEM-ML and component example
indexes, the package-owned component reference, the workspace changelog, and
the component, element, theme, and custom-element changelogs. Example links
remain source links except for the dedicated production-backed interactive
route described below.

The static `/packages/` route is generated from the manifest's audited public
surface inventory. Verification derives that inventory independently from every
non-private workspace `package.json` and every workspace Cargo crate that
explicitly declares `publish = true`, then requires one owner-matched reference
route for each deployment or crate identity. Package pages consume canonical
package Markdown directly; CEM Site does not copy their documentation.

The static `/tokens/` route consumes
`@epa-wg/cem-theme:build:tokens`' public `cem.tokens.catalog.json` output as
native JSON. The route manifest enumerates the exact canonical theme Markdown
specifications behind that catalog, and verification maps every generated token
record back to its source table heading. The generated values are read-only and
the page ships no JavaScript.

The static `/components/` route consumes the public
`@epa-wg/cem-components:build:catalog` JSON output. Its manifest distinguishes
canonical component sources from generated state-matrix evidence, while site
verification proves every component row, source link, example owner, and local
Storybook target. Figma projections and the Storybook build itself remain
outside the site output.

The static `/components/angular-material/` route consumes the exact pinned
Angular Material parity inventory owned by `@epa-wg/cem-components`. The site
build schedules that package's cached parity verifier, retains all catalog rows,
and renders benchmark provenance, coverage status, CEM owners, states, keyboard
and accessibility contracts, evidence, and deliberate scope boundaries. It
ships no JavaScript and does not add Angular as a runtime dependency.

The `/examples/interactive/` route fuses a site-owned JSON fixture with a native
CEMT layout. Its runtime XHTML view composes canonical `cem-field`, `cem-action`,
`cem-surface` and `cem-media-preview` declarations through `cem-elements`.
Token filtering and the action counter use declarative slices. Named theme
surfaces pair palette backgrounds with their intended text colors. Source and
rendered HTML use the shared preview's literal text mode. The rendered-HTML
preview is an explicit HTML export of the actual settled greeting paragraph;
instance hydration metadata is outside that example output.

Both interactive routes mount their XHTML views through the shared
`mountCemView` host-service boundary. Application JavaScript registers declaration
metadata and snapshots the native build's scalar control data before mounting;
it does not create visible markup, attach UI listeners or write rendered state.
Search matching and URL transport remain non-UI services. The complete static
search index stays useful without JavaScript. Runtime search preserves native
form submission, current input text, stable heading links and unrelated URL state.

Paired module-map v3 documents declare the exact source and destination graph,
including the canonical runtime, shared capabilities, XHTML declarations, CSS,
worker and WASM resources. The native CEM-ML transform publishes and rewrites
that graph into app-relative resources. JavaScript entries declare exact
`moduleImports` edges; the native publisher rewrites those bare specifiers to
relative deployed URLs. Other assets are byte-preserved. Verification checks
source/output digests, byte lengths and the complete dependency graph. No legacy
component installer, component behavior modules or runtime aliases are deployed.

The browser fixtures load standalone `cem-select.xhtml#cem-select` with its
`choice-select` capability and verify selection and declaration-owned scoped CSS.
They also verify action loading, retained editors, literal output, search phrase
ranking, empty and multiple matches, form submission, URL preservation and the
no-JavaScript search index. Production checks cover five themes, inherited public
tokens, forced colors, accessible names, keyboard focus and stable fragment links.
This automated evidence covers Chromium; physical-device and spoken
assistive-technology acceptance remain separate work.

Catalog and manifest layouts query retained generic-data CEM nodes directly.
They do not expect imported JSON to become records. The shared document importer
supports the token catalog within its 65,536-value budget; small expression
imports retain their existing limits. Runtime bootstrapping reads the already
published semantic HTML as scalar control data; it does not reparse source JSON
or replace the native import boundary with an application document model.
