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
