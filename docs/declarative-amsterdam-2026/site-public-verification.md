# CEM Site public verification — 2026-11-03

The [presentation guide](https://cem-amsterdam-2026.trans-acf.workers.dev/guides/declarative-amsterdam-2026/)
and the [complete CEM Site](https://cem-amsterdam-2026.trans-acf.workers.dev/)
are published and verified. The owner deployed the prepared Cloudflare bundle
as version `431d2a65-f8b6-4f5f-86f1-cdf552c8fb7f`. The
[public readback](site-public-431d2a65-readback.json) records exact byte checks,
each browser suite, source-map headers and participant destinations.

All 303 original native files match the verified release after HTTP decoding.
The native aggregate SHA-256 is
`741d9f3c06f15ed8c3f131b3bfdff8e2a0344c4e35cb80f6ddbec5ffe76f19d2`.
The uploaded bundle is identified by the [preparation record](site-cloudflare-publication.md)
and source commit `c4cc5bdf3137403c8d85dd2e32c77fd4b097154c`.
The version ID is owner-reported; the served bytes and browser behavior are
independently checked on the actual public origin.

## Public acceptance

Default Chromium passes the complete production crawl: 27 routes, 86 fragment
links, the existing landmark/heading/accessibility-name/keyboard checks and zero
runtime errors. This includes the Amsterdam guide at its canonical public route.
The interactive and search suites also pass against the public host, including
their existing theme, forced-color, retained-control, declaration/scoped-style,
URL/search and no-JavaScript assertions. Each suite uses a fresh context and
runs sequentially. Their original assertions and deadlines are unchanged.

The oversized native token source map is preserved at its original public URL.
Its decoded bytes match the original 45,614,987-byte artifact. Public HEAD returns
200 with JSON/gzip headers; conditional revalidation returns 304. All three
pinned participant-source pages return 200. The existing demo/Studio host's
inventory still matches `4d21f9cd…`; the separate Site deployment did not replace
it. The earlier full Studio workflow acceptance remains separate evidence.

Preparation owns the two clean native builds and local acceptance. Public
verification consumes that verified prepared release, checks its recorded
identity and every byte, then runs the browser suites. The initial invocation
unnecessarily started an upstream WASM optimization before any public checks;
it was cancelled. The corrected uncached target verifies the existing release
without rebuilding it during post-deployment acceptance. No product bytes changed
and no further deployment was needed for that verifier correction.

Reproduce against the prepared release:

```sh
CEM_SITE_ACCEPTANCE_URL=https://cem-amsterdam-2026.trans-acf.workers.dev \
yarn nx run cem-site:verify:public --parallel=1
```

All 15 hosting guards, five canonical-publication regressions, Site/Worker lint
and 65 presentation/resource guards pass. This establishes Chromium public Site
acceptance. Physical-device/spoken assistive-technology coverage, public CLI
installation, measured full-session rehearsal and final handoff are separate.
No native slides, existing projects/drafts, credential policies or Studio recovery
budgets were changed.
