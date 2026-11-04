# CEM Site: Cloudflare publication

## Prepared update: Markdown tables and Tokens navigation — 2026-11-04

Implemented and locally verified; **owner deployment and public verification
are pending**. All 21 Site Markdown imports now select GFM. Seven additional
table-bearing pages render semantic tables through the existing native
Markdown-to-HTML/DOM/page pipeline. The shared parser retains strict CommonMark
when explicitly requested. Literal pipes in 16 component-reference inline-code
spans are escaped so GFM does not truncate their cells or links.

All seven header layouts now send Tokens to the same-origin URL
`/packages/cem-theme/dist/lib/css-generators/`. The native `/tokens/` document
remains available through ordinary content links.

The native audit and browser checks cover all 21 Markdown documents, 17 tables
and 619 cells. The transform reference has its four named columns and eight body
rows. Browser acceptance checks all 27 headers, follows Tokens to the generator
index and exercises live Colors generation. Production checks pass 27 routes,
86 fragments and 1,161 focus stops. All 80 source-only snippets retain trimmed
source, with colored tokens in 68. Native tests cover the old-policy failure,
table cells/inline markup, escaped pipes, literal fenced examples and lost
inline-code/link refusal. The regular Site verification now includes this audit;
public verification consumes its fingerprinted inventory from the prepared
release without rebuilding.

Extract `dist/releases/cem-site-tables-and-tokens.tgz` (54,684,165 bytes) into a
fresh folder and deploy there:

```sh
npx wrangler@4 deploy --config wrangler.site.jsonc
```

Expected Worker: `cem-amsterdam-2026`. Archive SHA-256:
`c89dbe164766dca1b3c2265302a4abdbc6d6eb59c8705c6b9c6266f1f47bb2b6`.
Two clean builds agree on 303 native files; the combined package has 1,206 public
files with aggregate SHA-256
`c9e577dc4b2f1b2d5c2026a51ec42f7fd7ae9d779532d8a7ac9ed695dd007148`.
Wrangler dry run passes. All decoded file bytes match through a Node harness
using the actual Site adapter. The local Wrangler server stalled during asset
and snippet checks and exceeded their original deadlines; that attempt is not
recorded as a workerd pass. The successful production/table tests and Node
harness are separate evidence. Public acceptance follows owner deployment.
See the [tables/Tokens readback](site-tables-tokens-readback.json).

The accepted `c0833c4c…` release below predates this update.

## Current update: every demo link stays on the Site — 2026-11-04

The owner deployed version `c0833c4c-6820-4014-a44a-65aba2901282`; public file
checks and the complete public target pass using an explicit HTTP/2 browser
profile. The earlier default-transport startup and search failures are retained
as separate observations. See the
[public verification](site-public-verification.md) and
[public readback](site-public-c0833c4c-readback.json).
All seven layouts now use relative URLs for CEM Components
(`/packages/cem-components/`), Studio (`/studio/`) and Playground
(`/packages/cem-demo-element/demo/playground`). The presentation Site includes
all of their content and dependencies. The 383 published HTML/XHTML files contain
no links to the former green-morning host; every native header retains its
11 decorative fat-dot separators.

This update supersedes the previous galleries/snippets-only bundle. Extract
`dist/releases/cem-site-local-content.tgz` (54,649,537 bytes) into a fresh folder
and deploy from that folder:

```sh
npx wrangler@4 deploy --config wrangler.site.jsonc
```

Expected Worker: `cem-amsterdam-2026`. Archive SHA-256:
`77ee95afe257a530a96f37af00516022cc31a64867a53dd85812f0ba36730b1c`.
The combined inventory contains 1,206 public files: 303 deterministic native
files, 902 package/Studio files (including all 336 Studio build files), and the
publication manifest. Combined SHA-256:
`7c98cb068079627ed3b066d869fce9ba06ccf0cc80933a33960ec0e914c47d20`.
All decoded file hashes match through local workerd. Studio security headers
are emitted as a separate fingerprinted `_headers` control file scoped to
`/studio/*`; the Site root keeps its own publication.

Browser checks on local HTTPS pass for actual header navigation, Studio startup,
exact security headers, saved project cloning, native HTML/CSS preview, offline
rebuild, worker scope and Playground edited-source rendering. No request goes
to the former demo host. The local test accepts a development certificate only
on loopback; public verification uses normal HTTPS certificate validation.
Plain HTTP preview is unsuitable for the Studio precache test because its
production CSP upgrades redirected HTTP requests to HTTPS. The production
security policy is preserved unchanged.

The Site production crawl (27 routes/86 fragments), native determinism, source
verification, 23 publication guards, five canonical regressions, lint and Wrangler
dry run pass. The combined Site budget is now 1,999 public files to accommodate
Studio alongside the native Site; this is below Cloudflare's documented
[20,000-file free-plan limit](https://developers.cloudflare.com/workers/platform/limits/#static-assets).
The standalone demo's 999-file budget is unchanged.
See the [local-content readback](site-local-content-readback.json).

The public `cem-site:verify:public` target also verifies Studio/Playground
navigation, security headers, worker scope, online/offline project build and
Playground rendering. The accepted HTTP/2 profile command and its default-browser
limitations are recorded in the public verification linked above.

## Previous prepared update

### Galleries and source snippets — 2026-11-04

Prepared and locally verified; **owner deployment and public acceptance are
pending**. This update includes all 59 component galleries and their property
playgrounds on the presentation Site. CEM Components uses the relative URL
`/packages/cem-components/` in all seven header layouts. Studio and Playground
retain their existing demo-host destinations. The native document transform
trims surrounding snippet whitespace and publishes code through source-only
`cem-demo-element`, preserving indentation and escaped source. Supported
languages receive syntax coloring without executing the examples.

Use the new deploy-only archive
`dist/releases/cem-site-galleries-and-snippets.tgz` (38,581,167 bytes), SHA-256
`11c7c19e3cb9884161c7c68e412ea28049b935016f686c9d9473c319c0be7659`.
Extract it into a fresh folder and run there:

```sh
npx wrangler@4 deploy --config wrangler.site.jsonc
```

The output must name `cem-amsterdam-2026`. Share its public URL and Current
Version ID for verification. This single Site deployment includes the galleries;
the old separate demo-gallery bundle is superseded for this task.

The package contains 303 deterministic native files, 566 package/dependency
files and one gallery manifest: 870 published files, combined SHA-256
`75f760ffb79abbc51a197cea1115822cadda4a078f91b54882931fe6a85e6c88`.
All 870 decoded files match through actual local workerd. Browser checks pass
for all 59 gallery and 59 property URLs, three gallery theme/restoration flows,
and 80 exact, trimmed, source-only snippets (68 contain colored tokens). The
27-route/86-fragment Site crawl, interactive workflows and search pass.
See the [current preparation readback](site-galleries-snippets-readback.json).

Public verification after deployment uses the prepared combined inventory:

```sh
CEM_SITE_ACCEPTANCE_URL=https://cem-amsterdam-2026.trans-acf.workers.dev yarn nx run cem-site:verify:public --parallel=1
```

The shared viewer suite passes 19 of 20 stories. Its remaining native-editing
failure also reproduces with the unchanged viewer: an editable plaintext
control contains one element where the existing assertion requires zero.
The source-only story, six other Playground stories and all Site/gallery checks
pass. The editor issue is recorded separately without weakening that assertion.

## Historical publication receipts

The records below describe earlier 303-file releases and their public checks;
their archive hashes and external component link are superseded by this update.

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
