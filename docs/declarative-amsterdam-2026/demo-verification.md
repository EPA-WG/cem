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
