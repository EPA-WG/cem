# CEM Site

This Nx application publishes a static site through the native CEM-ML CLI. Its
checked-in `site.cem` graph is the publication manifest: every source, transform,
route, and output is explicit.

Run `yarn nx run cem-site:build` to produce `dist/apps/cem-site`, or run
`yarn nx run cem-site:verify` to build and verify the route allowlist, links,
generated-document provenance, transform report, and source-map sidecars.

Authored content follows a Hugo-like folder hierarchy under `content/`. Shared
semantic HTML composition lives in `layouts/page.cemt`. Cross-package generated
documents are read only from declared upstream Nx outputs; the application does
not duplicate or patch their source text.
