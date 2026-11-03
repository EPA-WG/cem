# Studio cold-start overlap — 2026-11-03

The public worker trace identifies a serialized startup path in owner-reported
version `ab74c3a7`: Studio waits for its validator's main-thread and worker
runtimes before installing the component runtime and its worker. The
[worker waterfall](startup-worker-waterfall.json) retains actual resource timings
from both worker realms, rather than inferring completion from main-page worker
script requests. The public main module matches the previously verified local
module, SHA-256 `51403722973d524d03515e3db3dd7c7e15cb9d8eb992e79727d3b5dde2adec08`.

In that capture the four WASM acquisitions run in sequence: approximately
1.83–12.91 s and 13.62–19.68 s for CEM-ML, then 19.82–25.82 s and
26.31–32.12 s for CEM-QL. Studio is not ready at the original deadline snapshot
and is ready at 38.27 s. This attributes a serial dependency in the observed
startup path. It does not attribute the earlier QUIC/aborted-response failures.

The default application now starts its existing validator and component
installation together. It waits for both original services before constructing
the repository/workspace or publishing the application. A failed peer causes a
successfully initialized validator to close; original errors remain intact and
concurrent/cleanup errors are retained together. Optional schema-assets startup
keeps its existing serial admission order. This is application service
orchestration; UI composition and behavior remain in their shared declarative
components. No parser, native owner, runtime grant or recovery policy is replaced.

The private startup module is registered in the source/destination module maps
and import map. The CSP import-map hash is updated to the exact emitted bytes;
its policy is otherwise unchanged. The graph-output verifier checks all declared
assets and inline script hashes. The new default deployment has 220 declared
assets and 335 Studio files; the assembled demo has 905 files. All four WASM
hashes match the preceding public inventory. Only six paths differ: main/startup
code, module map, index and its security-header files. CSS, component
declarations and sample hashes are unchanged. The
[readback](startup-overlap-readback.json) pins the local copy and measurements.

## Regression and controlled deployment evidence

Six startup tests exercise concurrent readiness, delayed publication, optional
serial admission, late-owner cleanup, concurrent failure and cleanup failure.
Four fail against the old serial coordinator; all six plus the existing 13
shell cases pass after the change. Typechecking, Studio lint, graph output and
the normal assembled-demo browser verifier pass.

The controlled browser fixture serves the actual graph-emitted deployment and
adds eight seconds before each WASM response. It uses a fresh browser context
for each variant and blocks service workers to isolate the acquisition order
from prewarming. The baseline replaces only the main module with the saved,
public-byte-matched original; other files and delays are identical. Both variants
initialize all four original runtimes without page errors. The old sequence
misses the unchanged 30-second readiness deadline and settles at 36.54 seconds;
the overlapping sequence is ready at 20.36 seconds. These are controlled-fixture
measurements, not a promised public-network startup time. The final prepared
module is separately rechecked under the same delay and is ready at 20.35 seconds;
its measured main-module hash exactly matches the assembled copy.

Run the candidate regression through Nx:

```sh
yarn nx run @epa-wg/cem-studio:verify:startup-latency
```

The archived comparison used `CEM_STUDIO_STARTUP_BASELINE` with the original
graph-emitted `main.js`. That fixture mode is superseded by the subsequent
worker-code reuse regression; the retained readback above pins the original
module hashes and request times. See [worker startup verification](startup-code-reuse-verification.md)
for the current comparison command. Historical measurements remain separate.

Normal offline/navigation acceptance also passes with service workers enabled
against the exact assembled copy at
`http://127.0.0.1:8081/studio/__cem_preview__/794250804fe15b59/`: all 23 selected
Login/Supplier configurations, draft preservation, failure/repair, live
interactions, graph provenance and independent copies. Its success report is
retained in the readback. This is local acceptance, not public acceptance.

## Delivery boundary

The overlap copy had 905 files and inventory SHA-256
`5deaa8422c508c2484a1cd177bf8641825ced00fa615a943ab123f783a74d3a2`.
Sasha's signed-in deployment route remains `npx wrangler@4 deploy` from the
checkout whose `../cem-bin` is the verified folder. The owner subsequently
uploaded this copy as `2ea68bcd`; [public readback](public-demo-2ea68bcd-readback.json)
confirms matching inventory/bytes but a command-worker startup timeout.
The [worker-code reuse correction](startup-code-reuse-verification.md) supersedes
this prepared copy. Repeat public freshness, the general browser verifier and
the full Login/Supplier suite after that upload, retaining the original deadlines.
Public acceptance stays open until those checks pass; the local participant kit
and frozen references remain the fallback. Entered-password/OTP refusal,
automatic recovery being disabled and the 4 MiB budget remain unchanged.
