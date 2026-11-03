# Public hosting identities and cached-client upgrades

The owner confirmed another upload. The public inventory is now exactly
`253278d16f34b620b2bbde2d9085f03950b69bee9d8938d250036334c623a55d`
(905 files); all 11 critical asset byte checks pass. No new Wrangler version ID
was supplied, so the [public readback](public-demo-253278d1-readback.json) records
the verified inventory and source commit rather than inventing a version ID.

Default Chromium still misses the unchanged 30-second readiness deadline. A
separate trace records the two main WASM transfers taking about 43.6 seconds,
without JavaScript errors. The HTTP/1.1 control uses explicit
`--disable-quic --disable-http2` flags and reaches readiness at 25.287 seconds.
That control neither replaces default-browser acceptance nor identifies which
transport mechanism caused the difference. The original failed asset request,
subsequent successful byte checks and both browser outcomes are retained.
Both traces also record `net::ERR_CACHE_WRITE_FAILURE` for the encoded CEM-ML
WASM URL and bulk aborted precache requests (99 in the default trace, 110 in
the control). The readback retains WASM request details and failure summaries;
these cache/network failures are not attributed to an individual protocol.

The controlled full public suite then reaches shell inspection but fails before
any selected configuration: a second shell module attempts to declare an
already installed view. Actual public HEAD requests establish that literal
`/studio/assets/@epa-wg/cem-studio/shell.js` returns 307 to the `%40epa-wg` URL,
which returns 200. JavaScript imports resolved before and after this redirect
can acquire distinct module identities.

## Asset adapter

The checked-in Wrangler configuration now binds `ASSETS` and runs
`tools/cloudflare/demo-worker.mjs` before asset resolution. The adapter forwards
literal `@` path characters to the asset service as `%40`, then returns its
response directly. The browser keeps the original module URL. Already encoded
paths, other paths, query strings, methods, conditional headers, response bodies
and CSP are preserved. This is hosting transport; no UI behavior moved into JS.

Two unit checks cover the observed redirect and request/response semantics. The
browser regression serves actual graph-emitted Studio through a backend that
reproduces Cloudflare's redirect. The baseline produces five redirects and the
same duplicate-declaration error; the adapter produces zero redirects, no
declaration errors and the same installation result on repeated calls.

The local fixture uses an ephemeral self-signed HTTPS certificate and trusts
only that fixture context, retaining the native CSP including
`upgrade-insecure-requests`. Public checks use normal certificate validation.
OpenSSL is required for this developer fixture. Wrangler 4.157.0 dry-run bundles
the adapter and confirms the `ASSETS` binding without an account login or deploy.

## Versioned static caches

A cache probe reproduced the other defect: `registration.update()` followed by
reload keeps the older command bytes in `cem-studio:0.1.0:runtime`. The isolated
saved project and unsaved draft survive, so storage clearing is unnecessary.

The default build now emits a provisional deployment through the existing native
graphs, fingerprints every emitted file in stable path order, and supplies that
digest as one generated build-input module. A second native emission writes the
final revision module. No emitted HTML or JavaScript is patched by the Node
build script. The provisional revision value avoids self-referential hashing.
Shell, stylesheet, module and sample changes all affect the fingerprint.

The service worker imports that revision module, allowing its dependency graph
to detect a new build even when the primary worker script is unchanged. Cache
names include scope and deployment identity. Fetches search only that worker's
planned caches; a restarted old worker reads its own cached inventory/module map
instead of metadata from a newer network deployment. Activation retires older
revisions of the same scope while retaining other scopes and legacy caches.
Mutable projects remain in IndexedDB, outside static caches.

Optional schema-assets builds emit their own revision module from their existing
explicit deployment digest; mismatched profile/worker identities are refused.
Six Node checks cover fingerprint determinism, changed non-module bytes, invalid
inputs, offline worker restart, scope/revision isolation and optional-profile
identity. Native optional-profile runtime acceptance is outside this default
deployment run.

The browser upgrade fixture starts with a graph-built shell snapshot and the
archived prior command module. It deploys the current graph at the same scope
and common version, waits for the replacement worker to activate after closing
its controlled tab, then reopens the project. The command bytes update and the
saved project, revision, resource, persisted text and dirty draft all match.
A subsequent synthetic release changes only a runtime module and the revision
dependency: it installs a new worker with the primary worker bytes unchanged.
The old active worker stays pinned while the update waits; the project and draft
survive this update and offline reload. No storage is cleared and no
`skipWaiting` or dirty-work safeguard is bypassed.

Useful commands, after retaining the prior graph-built Studio directory:

```sh
yarn nx run @epa-wg/cem-studio:test:deployment-revision
CEM_STUDIO_UPGRADE_BASELINE_ROOT=/absolute/path/to/prior/studio \
yarn nx run @epa-wg/cem-studio:verify:deployment-upgrade
yarn nx run @epa-wg/cem-studio:verify:hosting-module-identity
```

## Prepared deployment and remaining gates

The prepared `/home/suns/cem-bin` contains 906 files, inventory SHA-256
`ee7cc90c4507e467b9ab621b10682029cffb520a52dca7fc6b5d254163d78c71`.
Only four asset paths differ from the uploaded copy: `studio/cache-inventory.json`,
`studio/module-map.json`, `studio/service-worker.js`, and the new
`studio/deployment-revision.js`. All four WASM files remain byte-identical and
below 25 MiB. Main startup, native instances/memory, checkpoint budgets, credential
refusal, automatic-recovery policy and optional admission ordering are unchanged.

General local demo acceptance passes, including 336-file Studio freshness,
playground editing/Render, offline rebuild, worker scope, gallery rendering,
all ten theme generators and 120 playground HTTP checks. Lint, typecheck, the
cache regressions and the hosting adapter regression pass. The complete selected
23-configuration offline Login/Supplier walkthrough passes against the exact
`http://127.0.0.1:8081/studio/__cem_preview__/6ddd7228329531b8/` copy, including
validation failure/repair, graph provenance, independent copies and draft reload.
The [companion readback](hosting-upgrade-readback.json) pins this receipt, the
four unchanged WASM hashes, both browser regressions and the new inventory.

Deploy from the updated checkout using the owner's existing signed-in
`npx wrangler@4 deploy` route. The updated checkout is required for the new Worker
entry and binding as well as the assets. Then verify raw and encoded package
URLs both return 200 without redirects, run default public browser/full selected
acceptance, and verify cached-client updates separately. Keep the discussion
draft unposted and public functionality open until those gates pass.
