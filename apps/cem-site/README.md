# CEM Site

This Nx application publishes a static site through the native CEM-ML CLI. Its
checked-in `site.cem` graph is the publication manifest: every source, transform,
route, and output is explicit.

The checked-in `site.routes.json` is the audited publication allowlist. It
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
remain source links in this static phase; executable copies and their runtime
assets belong to the later interactive-example work.

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
Storybook target. Executable declarations/examples, Figma projections, and the
Storybook build itself remain outside the site output.
