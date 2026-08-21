# CEM Site

This Nx application publishes a static site through the native CEM-ML CLI. Its
checked-in `site.cem` graph is the publication manifest: every source, transform,
route, and output is explicit.

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
CEMT layout. It demonstrates searchable canonical theme tokens, live
`cem-action` and `cem-field` primitives, an executable inline CEM custom-element
fixture, and the resulting native light DOM. Its production custom-element,
component, CSS, worker, and WASM resources are not special-case copies: paired
module-map v2 documents declare the exact source and destination graph, and the
CEM-ML transform publishes and rewrites that graph into the app-relative browser
import map. `cem-site:verify:interactive` exercises the route in Chromium, while
the static verifier rejects undeclared dependencies and byte drift for every
published runtime resource.

The `/search/` route consumes the same route manifest through native CEMT and a
paired module-map v2 contract. CEMT renders the manifest's `searchDocuments`
projection as the route's semantic HTML index; site-owned code only filters that
native projection. The search field and action are `@epa-wg/cem-components`
rendered on the `cem-elements`-backed custom-element light DOM, without a
bundler, JSON sidecar, or post-build copy. Every searchable fragment is
manifest-owned and verified against a unique rendered heading ID.
`cem-site:verify:search` covers URL queries, live filtering, CEM component
composition, import-map scope, and navigation to a stable heading fragment in
Chromium.
