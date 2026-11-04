# Studio startup and offline prefetch contention

The owner uploaded the updated checkout. Public inventory `ee7cc90c…` matches
all 906 prepared files and all 11 critical asset byte checks. The service worker
and revision module also match. Both literal `@epa-wg` and `%40epa-wg` module
URLs now return 200 without a redirect, confirming the asset adapter is live.
No new Wrangler version ID was supplied; the [public readback](public-demo-ee7cc90c-readback.json)
uses the observed inventory and source commit instead.

Public acceptance is still open. The general verifier encounters an aborted
WASM response body in gallery initialization. Two fresh default-browser selected
suite attempts miss the unchanged 30-second readiness limit before executing
any configuration. A separate fresh default HTTP/3 trace reaches readiness at
24.512 seconds with full main WASM transfers and no JavaScript errors; that
single diagnostic does not replace the failed full gates. The trace also records
a cache-write failure and aborted precache requests. All outcomes are retained,
without attributing the variation to an individual network protocol.

## Controlled reproduction

The deployed main module registers the service worker before starting native
runtime acquisition. Initial installation immediately precaches those same
large WASM files. An actual-deployment browser regression now keeps service
workers enabled and fairly shares 3.2 MiB/s of aggregate WASM delivery among
concurrent responses. It streams the original WASM bytes, without changing
native code, owners or memory. Other resources use the normal preview handler.

The saved deployed main module misses readiness at 30.418 seconds. Its service
worker starts at 0.424 seconds; four WASM transfers compete from the beginning,
and the rendering worker's own acquisition starts near the deadline.

The built candidate is ready at 26.171 seconds, with the same 30-second limit.
Core initialization finishes before service-worker creation at 23.322 seconds;
the later offline-cache transfers no longer compete with core acquisition.
Both original dedicated runtimes remain, alongside their independent main-thread
instances. Background precache is still running when this readiness fixture
closes; offline installation is checked separately by normal demo acceptance.
These are controlled local measurements, not a public-network latency promise.

Run the candidate, or compare it with a saved deployed graph-emitted main:

```sh
yarn nx run @epa-wg/cem-studio:verify:startup-prefetch
CEM_STUDIO_PREFETCH_BASELINE=/absolute/path/to/prior/main.js \
yarn nx run @epa-wg/cem-studio:verify:startup-prefetch
```

## Change and retained behavior

Default-profile service-worker registration now runs after core initialization
settles, before mounting the shell's update coordinator. The `finally` path still
checks for an update after a core failure, retaining the original core error.
Optional schema-assets profiles keep registration before package initialization
and retain serial native admission. Existing update activation, dirty-work and
credential-refusal policies are unchanged. No UI declaration or styling changed.
The native worker startup limit remains 10 seconds; Studio readiness remains
30 seconds and checkpoint limits remain 5 seconds/4 MiB.

The next `/home/suns/cem-bin` copy has 906 files, inventory SHA-256
`4d21f9cd63ddb3c7c989b9bc9c123040e7472612274f3be1cb56c97d532a0770`.
Only `studio/assets/@epa-wg/cem-studio/main.js` and
`studio/deployment-revision.js` differ from the uploaded copy. All four WASM
files remain byte-identical. General local demo acceptance passes, including
336-file Studio freshness, offline SCSS rebuild, worker scope, gallery rendering,
playground editing/Render, ten theme generators and 120 playground HTTP checks.
Cached-client upgrades, dependency-only updates, saved projects, dirty drafts and
offline reload pass with the new scheduling. Lint and typecheck pass.

The full selected 23-configuration offline walkthrough passes against the exact
`ceb366c46aade55c` preview alias: 20 Login and three Supplier configurations,
validation repair, template output, saved graph provenance, live view, source
inspection/formatting/coloring and offline draft restoration. The
[verification receipt](startup-prefetch-readback.json) retains the measured
comparison, prepared identities and local workflow/upgrade results.
The public workflow and all advertised public destinations remain unaccepted
until these new bytes are deployed and the unchanged public gates pass.

A new read-only public-link verifier checks the 14 frozen slide/demo URLs in
isolated default-browser contexts, including tree/resource/configuration selection
and zero saved projects. It records failures, including inventory mismatch,
before any browser starts. The current mismatch guard correctly refuses the
uploaded `ee7cc90c…` bytes while the prepared copy is `4d21f9cd…`; it does not
claim a navigation pass. After deployment run:

```sh
yarn nx run @epa-wg/cem-studio:verify:public-demo-links
```

Deploy through the owner's existing signed-in `npx wrangler@4 deploy` route
from the updated checkout using this verified `../cem-bin`. Repeat default
public general and full selected acceptance, then the read-only link check.
Keep the discussion comment unposted and preserve the local participant fallback.

## Public follow-up

The owner subsequently deployed the `4d21f9cd…` copy. Default public general
acceptance, all 23 selected offline configurations and all 14 fresh-context demo
links now pass. Readiness across those links is 23.952–28.555 seconds. See the
[public verification and receipt](public-demo-4d21f9cd-verification.md).
The pending-upload statements above describe the original prepared-copy stage.
