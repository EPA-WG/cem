# Amsterdam demo verification

The demo site is published at
[green-morning-fd6e.trans-acf.workers.dev](https://green-morning-fd6e.trans-acf.workers.dev/),
as confirmed by Sasha. Verify locally before checking the public deployment.
The demo deployment and the separately built CEM documentation Site have
different build inventories; acceptance of one does not establish the other.

## Local deployment — 2026-11-02

The initial `/home/suns/cem-bin` copy contained 898 files and matched its own
deployment inventory, but 23 Studio files were stale or absent compared with
`packages/cem-studio/dist/static`. These included runtime modules, the navigation
template and the Login sample. Inventory integrity alone does not establish
freshness against the current build.

The demo verifier now compares file bytes without formatting entire binary
buffers on failure. A stale copy fails promptly with the affected path and the
`yarn copy:demo` refresh command. The real stale deployment exercised this
negative case. All 17 deployment/hosting regression tests and verifier lint pass.

The release build and inventory-based refresh passed: 904 files total, with all
four WASM files below 25 MiB. The CEM-QL pair is 24,140,410 bytes per file and
the CEM-ML pair is 24,860,607 bytes per file. The deployment inventory SHA-256 is
`80fab6035f8e264325c27775b50733adfbfae7bf907c022b1712b215d78bbfb8`.
The local Studio scope is `/studio/__cem_preview__/6c04487c4a7f8804/`.

`yarn verify:demo` passed against `http://localhost:8081/`. All 334 Studio files
and paired gallery runtimes match their build outputs. Browser checks passed
for native rendering/highlighting, shared initialization, isolated runtime
memories, Studio links and URL selection, independent cloning, native HTML/CSS
preview, offline rebuild and service-worker scope. All 10 theme generators
rendered; all 120 canonical playground URLs returned successfully, and the live
tree gallery rendered. URL reachability does not establish interaction coverage
for every playground. Detailed ignored logs and inventory evidence are under
`dist/reports/demo-site/`.

The shared `@epa-wg/cem-studio:verify:login-navigation` target also passed against
that exact local Studio scope via `CEM_STUDIO_ACCEPTANCE_URL`, with build
dependencies and cache reuse excluded for this verification run. It exercised
20 Login and three Supplier configurations offline, including schema
failure/repair, template output, graph provenance across save/reload, live view,
source inspection/formatting/coloring and standalone expressions. Both README
routes, URL/tree agreement, independent source ownership and unsaved draft
restoration after offline reload passed. The temporary test context was closed;
existing browser storage was not used. The report is
`dist/reports/cem-studio/presentation-samples.json`.

Reproduce the complete local check from the repository root:

```bash
yarn copy:demo
yarn serve:demo
# In another terminal:
yarn verify:demo
```

Use isolated browser contexts for clone, editing and offline checks. Preserve
existing saved projects and drafts. Entered passwords or one-time codes must
continue blocking recovery until explicitly cleared; never export credentials.

Public-route functionality, deployment freshness and the slide-to-demo resource
comment remain separate delivery checks in [the todo list](../todo.md).

## Playground initialization repair — 2026-11-02

The real playground page initially remained at “Loading CEM…” because its import
map omitted `@epa-wg/cem-ml/wasm/factory`, a dependency of the shared command
service. The earlier runtime probes and playground HTTP checks did not exercise
this page's initialization. The authored page now maps the factory to the paired
browser runtime; no UI behavior was added.

After `yarn copy:demo`, the expanded `yarn verify:demo` passes. It opens the actual
deployed playground page, waits for initialization, edits the shared source
editor, invokes Render and checks the resulting HTML. The existing Studio,
gallery and theme checks still pass, with 904 files and unchanged WASM sizes.
Verifier lint and syntax checks also pass. The new run's log is
`dist/reports/demo-site/playground-browser.log`. This repair still needs to be
deployed to the public host.

## Public deployment check — 2026-11-02

Direct HTTPS requests and an isolated Chromium session confirm that the public
host still serves the older playground: it lacks the WASM factory mapping,
reports the unresolved `@epa-wg/cem-ml/wasm/factory` import, stays at “Loading
CEM…” and leaves Render disabled. The HTML request redirects to the extensionless
`/packages/cem-demo-element/demo/playground` route. This is a public deployment
failure; the repaired local page continues to pass.

The public deployment inventory has 898 files; the local inventory has 904.
Comparing the inventory records finds 99 changed or absent entries, including
six paths absent from the public inventory. These include shared runtime helpers
and newer Login/Supplier sample bytes. This comparison checks inventory records,
not a byte-for-byte download of every public asset. The public inventory SHA-256
is `7d88a2d8b34eaff2a1be945f3552e5c527058eedcd1d235d948155bc3ee51b2f`.
The playground HTML was also compared directly and differs from the local copy.

The general demo probe reached native rendering/highlighting successfully on
the public host, then stopped at its local-server-specific Studio revision-path
assertion: public Studio uses `/studio/`. That assertion does not establish a
public Studio runtime defect or full functional acceptance. The separate real
playground browser probe establishes the import failure above.

Refresh the published deployment from the verified `/home/suns/cem-bin` folder,
then recheck its inventory and browser functionality. Sasha confirms that no
Wrangler project is configured. No credentials were accessed or changed.
Existing projects/drafts and personal
browser storage were not accessed. Ignored evidence files are
`dist/reports/demo-site/public-{freshness,inventory-diff,playground}.json` and
`public-browser.log` in that directory.

The [discussion resource comment draft](discussion-resource-comment-draft.md)
maps physical slide numbers and stable slide IDs to ordinary project/resource/
configuration links. It is not posted, and its public operations remain pending
verification after deployment.


## Production verifier and refreshed local copy — 2026-11-03

The [current readback](public-demo-readback.json) records a new local refresh and
public check. The local directory still matched its own inventory, but Studio's
packaged `components.xhtml` differed from the current build output. `yarn copy:demo`
rebuilt/copied the selected dependencies through Nx and refreshed all 904 files.
The new inventory SHA-256 is
`19913ea5e36db3f247494a6a2fb1994cf36606fa54c392974141b6c5a28cb82e`.
All 334 Studio files and paired gallery runtime bytes now match their build outputs.
The four WASM files remain below 25 MiB.

`yarn verify:demo /home/suns/cem-bin http://127.0.0.1:8081/` passes the actual
playground's initialization/editor/Render, gallery native rendering/highlighting,
Studio navigation and independent SCSS project clone, HTML/CSS preview, offline
rebuild and scoped worker. All ten theme generators render; the live tree gallery
works and all 120 canonical playground URLs respond. Those HTTP responses do not
establish every playground's interaction coverage. This run is the general demo
verifier, not another complete Login/Supplier walkthrough.

The verifier now has an explicit production mode. It requires exactly `/studio/`
and its original worker scope, while local mode still requires the revision-based
preview subtree. Production mode compares the public inventory to the verified
local inventory, then checks original bytes for 11 critical assets: playground,
module/cache maps, both project samples, both generation factories and all four
WASM binaries. Browser interactions follow; byte checks alone never establish
exercise acceptance. A local HTTP control passes all 12 inventory/asset checks.

Six new fixtures cover separate hosting paths, stale inventory, same-size changed
asset bytes, HTTP-200 fallback HTML, missing assets and invalid deployment roots.
All 23 combined deployment/hosting tests and verifier lint pass. No application
UI behavior, service-worker scope, storage policy or browser deadline was changed.

### Public check and deployment handoff

The public inventory still has 898 files with SHA-256
`7d88a2d8b34eaff2a1be945f3552e5c527058eedcd1d235d948155bc3ee51b2f`.
It differs from the refreshed local inventory in 114 changed/missing records,
including six absent paths. This is an inventory-record comparison, not a
byte-for-byte download of every remote asset. Direct playground HTML still lacks
the factory import. An isolated Chromium 148.0.7778.96 context reproduces the
unresolved `@epa-wg/cem-ml/wasm/factory` error, “Loading CEM…” and disabled Render.
Production-mode verification refuses this stale inventory before claiming any
browser acceptance. Full public Login/Supplier acceptance remains unrun.

The earlier no-project note is superseded: Sasha checked in `wrangler.jsonc` at
`3006d632` for worker `green-morning-fd6e` and asset directory `../cem-bin`.
Wrangler 4.157.0 reports no authentication in this Linux environment. Sasha has
chosen to deploy from a signed-in machine; no login flow or temporary-account
deployment was started here. From the checkout with the verified `../cem-bin`,
the owner's deployment command is `npx wrangler@4 deploy`.

After that upload, run:

```sh
CEM_DEMO_HOSTING=production yarn verify:demo /home/suns/cem-bin https://green-morning-fd6e.trans-acf.workers.dev/
```

Then run the separate complete selected Login/Supplier acceptance against
`https://green-morning-fd6e.trans-acf.workers.dev/studio/` using
`CEM_STUDIO_ACCEPTANCE_URL`. Preserve the exact source/configuration checks,
independent copies, offline drafts and credential refusal. Do not post the
resource comment or advertise completed public acceptance until both gates pass.
The browser contexts used for these checks are disposable; no existing projects,
drafts or user browser storage were read or cleared, and no credentials were entered.


## Owner-deployed version ab74c3a7 — 2026-11-03

Sasha supplied Wrangler's version ID
`ab74c3a7-489a-4f77-be59-a3ed65f8cac4`. This version ID is owner-reported;
the Cloudflare account API was not queried. The
[post-upload readback](public-demo-ab74c3a7-readback.json) supersedes the earlier
stale-deployment finding: the public inventory now exactly matches the verified
904-file local inventory, SHA-256
`19913ea5e36db3f247494a6a2fb1994cf36606fa54c392974141b6c5a28cb82e`.
All 11 critical asset byte checks pass, including all four WASM files. This is
not a byte-for-byte download of every public asset.

The actual public playground also passes its isolated initialization, source
edit and Render check: the final state is `HTML → HTML`, with the expected
heading and paragraph and no browser errors or failed requests. That run uses
Chromium 148.0.7778.96 with HTTP/3 and HTTP/2 disabled. The earlier missing
factory import is repaired in the deployed page.

Public Studio acceptance remains incomplete. The general browser run reaches
native gallery rendering, then misses the original 30-second Studio startup
deadline. A separate default-browser Studio probe captures a WASM transfer
aborted with `ERR_QUIC_PROTOCOL_ERROR`. A retry using Chrome for Testing with
QUIC disabled fails during gallery WASM initialization with an aborted response
body; its cause is not attributed. The HTTP/1.1-only general run renders the
galleries but still misses Studio's startup deadline. Separate direct WASM
fetches succeed over both HTTP/2 and HTTP/1.1; those controls do not establish
that disabling either protocol repairs the failed runs.

A fresh HTTP/1.1-only lifecycle observation retains the failed cutoff: Studio
is still “Opening Studio…” at the 32.215-second snapshot after the existing
30-second readiness wait. At the later 52.238-second snapshot it is ready with
no page errors. This bounds observed late readiness; it is not an exact startup
time or a passing acceptance result. The two largest completed main-page WASM
transfers take about 10.24 and 6.34 seconds. Main-page CDP still lists worker
scripts as pending even at the ready snapshot; worker-target completion was not
captured, so those entries do not establish stalled worker downloads or explain
the remaining startup time. Preserve these distinctions before a runtime repair.

The full selected Login/Supplier verifier now supports an explicit
`amsterdam-2026` public profile, allowing only the exact HTTPS host and Studio
subtree and requiring the presentation suite. Loopback-only acceptance remains
the default. Credentials, query/fragment selections, foreign origins/ports,
profile typos and other suites are refused. Trailing slashes normalize before
constructing entry URLs. All 22 participant/target guards and Studio lint pass.

Public results use a separate `public-presentation-samples.json` receipt with
started/passed/failed status and target/browser provenance. The actual public
attempt writes `failed` at startup before any of the 23 configurations run;
an older local report cannot stand in for that result. Run public acceptance
directly rather than reusing a cached Nx result for a changing remote host:

```sh
CEM_STUDIO_PUBLIC_ACCEPTANCE=amsterdam-2026 \
CEM_STUDIO_ACCEPTANCE_URL=https://green-morning-fd6e.trans-acf.workers.dev/studio/ \
node packages/cem-studio/scripts/verify-static-worker.mjs --presentation-only
```

The diagnostic protocol controls used a temporary `CEM_STUDIO_CHROME` wrapper
that appends `--disable-quic --disable-http2` to the bundled Chrome for Testing
executable. They did not change browser defaults in the repository, application
code, deadlines, service-worker scope, credential refusal or saved storage.
No user browser profile was accessed and no real credentials were entered.

Next capture the worker-side loading/initialization timeline and reproduce the
cold-start bottleneck before choosing a bounded correction. Rerun both public
browser and complete Login/Supplier acceptance with unchanged deadlines. Keep
the verified local kit as the participant fallback and the discussion comment
unposted until public functionality is accepted.


## Prepared cold-start correction — 2026-11-03

The [worker trace and overlap verification](startup-overlap-verification.md)
now attribute the observed serial acquisition path to main-thread/worker
initialization of the validator followed by main-thread/worker initialization
of the component runtime. Overlapping those independent default-profile services
passes a controlled cold-download regression without extending the 30-second
deadline. Optional package activation retains its serial order. This does not
attribute the separate QUIC or aborted-response failures.

The new 905-file copy at `/home/suns/cem-bin` has inventory SHA-256
`5deaa8422c508c2484a1cd177bf8641825ced00fa615a943ab123f783a74d3a2`.
The normal general demo verifier and all 23 selected offline Login/Supplier
configurations pass against that exact copy. Source/destination maps include
the private startup helper and the CSP hash matches the emitted import map.
All four WASM files, component declarations, CSS and sample hashes match the
preceding public inventory. The [readback](startup-overlap-readback.json) retains
these boundaries and the complete local walkthrough receipt.

The owner must upload this new copy before post-fix public acceptance can run.
The earlier `ab74c3a7` public findings remain the baseline; no public pass or
new deployment version is inferred from local results. Use the existing
Wrangler deployment from the signed-in machine, then repeat production freshness,
the general public browser verifier and the full selected public suite above.
