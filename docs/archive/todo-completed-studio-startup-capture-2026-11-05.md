# Public Studio startup capture — 2026-11-05

The historical first combined public run exceeded Studio's 30-second readiness
deadline without a request-level trace. Its empty `workers` array described
service-worker registrations, not dedicated workers. The separate successful
23,370 ms startup cannot establish the failed attempt's cause. Keep that item
conditional on recurrence; do not change runtime scheduling from these repeats.

The application verifier now retains monotonic stage marks, same-origin request
paths, transfer timing, HTTP status, failed requests and worker lifetimes during
initial Studio startup. It records service-worker request ownership explicitly;
Playwright groups page and dedicated-worker requests together. It records no
bodies, headers or query strings. Limits are 1,024 requests, 16 workers and 32
marks, with dropped counts. Capture detaches and copies at readiness or failure,
before browser cleanup; late completion cannot rewrite it. Failure traces are
saved in the public report, alongside the existing error and worker registrations.

- [x] Add the explicit capture/controlled-fixture task before fixture work.
- [x] Verify repeated transfers, ownership, failed/pending transfers, bounded
      storage, query redaction and teardown immutability.
- [x] Hold a real dedicated-worker HTTP response until the existing 30,000 ms
      readiness wait fails, assert the pending transfer and worker are captured,
      then release it and assert the failed capture stays unchanged.
- [x] Run `yarn nx run cem-site:test:publication-host`: **26/26 pass**.
- [x] Run `yarn nx run cem-site:lint`: **pass**.
- [x] Run the complete existing public `verifyLocalApps` function in separate
      fresh browsers with default and explicit HTTP/2 transport.
- [x] Match five deployed startup files to the accepted `928c385e…` receipt,
      and recheck the frozen presentation archive's size and SHA-256.

| Public application workflow | Default | Explicit HTTP/2 |
| --- | ---: | ---: |
| Initial view settled, from Studio navigation | 9,017 ms | 6,883 ms |
| Same-origin captured requests | 533 | 533 |
| Failed startup requests | 0 | 0 |
| Dropped capture records | 0 | 0 |
| Security headers and service-worker scope | Pass | Pass |
| Saved clone, online native build, styled artifact | Pass | Pass |
| Offline reload and identical native rebuild | Pass | Pass |
| Edited Playground render and legacy-host refusal | Pass | Pass |

Both runs preserve normal public TLS, the existing readiness deadline, default
transport behavior and all original workflow assertions. They are focused
application checks, not a new complete 1,206-asset Site acceptance. No runtime,
hosting configuration, frozen delivery material or public deployment changed.

An earlier separate direct default startup reached readiness in 11,786 ms but
captured `net::ERR_QUIC_PROTOCOL_ERROR` on a later QL WASM transfer at 10,306 ms,
after the core WASM transfers had completed. Subsequent cancellations also
preceded the capture boundary. That run proves a separate transport failure;
it does not establish the cause of the historical readiness timeout or constitute
offline acceptance. The later complete default workflow passed. Preserve both
observations; one successful default run does not establish transport reliability.

The [compact receipt](studio-startup-capture-2026-11-05.json) retains the five
identity pins, worker/stage records, WASM transfers and direct-probe failure
counts. Full fresh-workflow traces are local ignored files under
`dist/reports/cem-site/startup-audit-{default,http2}-2026-11-05.json`.
The [historical failed receipt](../declarative-amsterdam-2026/site-public-c0833c4c-readback.json)
and [separate successful network capture](../declarative-amsterdam-2026/site-public-c0833c4c-startup-network.json)
remain unchanged. On an actual recurrence, use the captured pending/failed
transfer and worker boundary to design a cause-specific regression before
changing the application or loader.
