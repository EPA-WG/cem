# CEM Site

This Nx application publishes a static site through the native CEM-ML CLI. Its
checked-in `site.cem` graph is the publication manifest: every source, transform,
route, and output is explicit.

The checked-in `site.routes.json` is the audited publication allowlist. It
records each route or resource, its canonical source, owning Nx project, and
the upstream target required for generated inputs. Authored sources have no
upstream generation target and participate directly in the site build hash.

Run `yarn nx run cem-site:build` to produce `dist/apps/cem-site`, or run
`yarn nx run cem-site:verify` to build and verify the route allowlist, links,
generated-document provenance, transform report, source-map sidecars, token
catalog traceability, and component-catalog ownership.

Authored content follows a Hugo-like folder hierarchy under `content/`. Shared
semantic HTML composition lives in `layouts/page.cemt`. Cross-package generated
documents are read only from declared upstream Nx outputs; the application does
not duplicate or patch their source text. Archive, planning, temporary, Figma,
and debug-token paths remain excluded from the publication allowlist.

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
