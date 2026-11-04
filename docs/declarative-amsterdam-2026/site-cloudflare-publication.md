# CEM Site: Cloudflare publication

Public follow-up: the owner deployed this bundle as
`431d2a65-f8b6-4f5f-86f1-cdf552c8fb7f`. The [public presentation guide](https://cem-amsterdam-2026.trans-acf.workers.dev/guides/declarative-amsterdam-2026/)
now passes all 303 byte checks, the 27-route/86-fragment production crawl,
interactive examples and search. See the [final verification](site-public-verification.md).
The preparation-stage pending statements below are historical.

Prepared 2026-11-03. The owner selected Cloudflare and will deploy from their
existing signed-in machine. This publishes the native CEM Site as a separate
Worker, `cem-amsterdam-2026`. The existing demo/Studio Worker remains
`green-morning-fd6e`; its configuration and deployment are separate.

The [preparation receipt](site-cloudflare-preparation-readback.json) records a
303-file native publication with aggregate SHA-256
`741d9f3c06f15ed8c3f131b3bfdff8e2a0344c4e35cb80f6ddbec5ffe76f19d2`.
Two clean builds match. Local browser acceptance passes all 27 routes and 86
fragment links, interactive examples, search and the focused Amsterdam guide.
The guide has six sections, the native platform-role table, both source excerpts,
three pinned participant links and the existing privacy boundaries.

## Deploy the verified bundle

The deploy-only archive is `dist/releases/cem-site-cloudflare.tgz` (19,253,366
bytes), SHA-256
`9dfb3a2aeaed63260f81a232dd60addf48d80227e8065906570eb1d6685e93a0`.
Extract it into a new folder. It contains the prepared native assets, two small
hosting adapters, the separate Wrangler config, original inventory and deployment
instructions. No Rust build or application dependency installation is required
on the signed-in deployment machine. From the extracted folder run:

```sh
npx wrangler@4 deploy --config wrangler.site.jsonc
```

Share the actual URL and Current Version ID printed by Wrangler. The Worker name
is configured; no public origin or successful deployment is claimed before that
readback. Alternatively regenerate the same publication from this repository:

```sh
yarn nx run cem-site:prepare:cloudflare --parallel=1
npx wrangler@4 deploy --config wrangler.site.jsonc
```

The native output stays at `dist/apps/cem-site`. Wrangler uploads the separate
`dist/hosting/cem-site` packaging output.

## Lossless source-map hosting

The original `tokens/index.html.map` is 45,614,987 bytes, above Cloudflare's
[25 MiB individual asset limit](https://developers.cloudflare.com/workers/platform/limits/).
A direct-upload dry run refuses it. Packaging retains its original native bytes
and URL, while storing a 780,110-byte gzip representation in the asset service.
The Site Worker serves that stream with the documented
[manual response encoding](https://developers.cloudflare.com/workers/runtime-apis/response/#the-encodebody-option).
HTTP decoding returns the exact original source map. Identity-only clients use
streaming decompression; HEAD/cache/missing responses retain their semantics.
Every other published asset remains byte-preserved. HTML, application scripts,
styles, templates and native processing semantics are unchanged.

All 15 hosting guards and five canonical-publication regressions pass, as does
Site/Worker lint. Wrangler 4.157.0's packaged dry run passes. A real local
Wrangler/workerd server returns all 303 native files with matching decoded
bytes, including the 43.5 MiB map. Gzip and identity responses match its original
hash; HEAD returns 200 and conditional revalidation returns 304. These are local
hosting checks, not public acceptance.

## Verify the actual public host

After assigning the origin printed by Wrangler to `CEM_SITE_DEPLOYED_URL`, run:

```sh
CEM_SITE_ACCEPTANCE_URL="$CEM_SITE_DEPLOYED_URL" yarn nx run cem-site:verify:public --parallel=1
```

The target is uncached and uses the previously verified prepared release without
rebuilding or replacing it. Its identity must match the recorded clean-build
inventory and preparation receipt. It verifies every original native file before launching
the existing production, interactive and search browser suites sequentially.
HTML is fetched at the canonical routes declared by the native manifest.
Credentials, non-HTTPS URLs, non-root paths, queries, fragments and custom ports
are refused. Browser assertions and deadlines are unchanged. Public receipts
use `public-*.json` names and do not overwrite local acceptance evidence; a
failure records its stage and does not become a public pass.

The actual public host has now passed and the publication TODO is complete.
Timed rehearsal and final handoff remain separate work.

## Header navigation update — 2026-11-04

All seven native layouts now include Studio and Playground. The component link
is labeled **CEM Components** and opens the live package gallery at
`https://green-morning-fd6e.trans-acf.workers.dev/packages/cem-components/`.
Every header has 12 links separated by 11 decorative `•` spans with
`aria-hidden="true"`. The static component reference remains available through
ordinary Site content links.

Two clean native builds agree on 303 files. Site source verification and the
production browser checks pass: 27 routes, 86 fragment destinations and 1,163
keyboard focus stops. An additional inspection checks all 27 generated headers
for labels, destinations and separators. The three external destinations return
HTTP 200. Wrangler's deployment dry run passes.

Prepared native aggregate: `bce339c3ea8da49b16129ece65683f4504bb4912dc45abbddd662f1ad13497d6`.
Deploy-only archive SHA-256: `240e7934ecfc9d3dcae8dd20c5b2dee494ef192c218a17c452f0c0937a8923e4`.
The local alias is `dist/releases/cem-site-header-update.tgz`.
Extract and deploy with `npx wrangler@4 deploy --config wrangler.site.jsonc`.
Public acceptance passed on 2026-11-04: all 303 native files, 27 browser routes,
86 fragments, interactive examples and search. All 27 public headers have the
new links and separators. See the [header readback](site-public-header-readback.json).
