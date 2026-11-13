# Shared immutable WASM code delivery

The full source-component runtime now acquires and explicitly compiles its WASM
once in the original page, then sends the immutable `WebAssembly.Module` to each
processing worker. Page and worker still instantiate separate native heaps.
The selected compiled-cart runtime remains a separate profile and artifact.
Initialization remains shared infrastructure; declaration, theme, native lease
and credential policies continue to govern execution.

## Admission and ownership

The build emits `cem_ql_code.js` beside the matched bindings and WASM. Its
`cem-ql-code/1` record pins the binary SHA-256 and size, binding-source SHA-256,
import/export interface SHA-256, ABI tag and build-owned feature profile. The
loader reads only the installed fixed URL, bounds the streamed body to the
expected size, hashes the complete bytes before compilation, then checks the
compiled interface. Concurrent callers share one pending module promise. Failed
acquisition clears that promise for a subsequent explicit demand.

The processing protocol is v31. A worker first requests code using
`cem-render-code-v1` and its own build record. The host compares the record before
acquiring code and sends a structured clone of the compiled module. The worker
checks its own record and the module interface before announcing readiness.
Repeated/late deliveries, malformed records and work before admission refuse.
The built-in worker always uses this bootstrap; trusted custom worker factories
retain their existing v31 ready-envelope interface.

This is an original-host bootstrap, not an authored processing operation or a
capability grant. JavaScript cannot recover original bytes from a compiled
module: the receiving worker relies on the trusted sender's byte verification
and independently checks its own build pins and module interface. The binding
hash identifies the paired build; it does not re-fetch or authenticate JavaScript
in an already executing realm. Import/export descriptors do not prove function
signatures or instruction identity. Authored messages cannot select code URLs,
feature profiles, instances, memories or native handles.

The module cache retains no `WebAssembly.Instance`, memory, native binding
closure or source/style lease. Original page diagnostics and retained worker
rendering keep their separate native owners. Each owned fallback/fresh worker
instantiates its own heap; recovery and retirement continue to use their
original admission and complete-group checks. A cancelled participant stops
waiting without cancelling code acquisition for surviving participants. Pending
host delivery captures a weak reference so it cannot keep a retired transport
or its embedding closures alive. Late completion never posts to a retired
worker. An embedding's already initialized legacy instance is preserved.

Byte-cache eviction therefore does not prevent fresh-heap recovery while the
original page still retains verified code. A fresh realm without code must
acquire and verify the bytes; missing offline bytes refuse. Code reuse does not
authorize replay of a GraphQL mutation or recovery of an unknown write.

## Build and deployment

`cem_ql:build:wasm` emits the code assets after optimization. For an existing
matched native build, `cem_ql:build:code-bindings` refreshes only JavaScript after
checking the binary against its optimization receipt. Package verification
includes the code module and declarations. Shared-demo adapters route package
and root imports to the same code cache; Studio's explicit module maps include
both the runtime adapter and generated module.

The optional Studio schema-assets deployment generates separate pins from its
own binary and renderer bindings, with profile `cem-ml-optional-schema-assets`.
Its native command factory is regenerated from the same profile's bindings,
avoiding a stale wrapper beside an unchanged native artifact. Full and optional
records cannot be interchanged.

The optional profile's October 29 artifact lacked current payload-capture
entry points. Rebuilding it also exposed a stale committed catalog bootstrap:
the existing native `compiled_bootstrap_pin_matches_complete_native_catalog`
test failed against the complete current catalog. The pin is regenerated from
the native emitter's `bootstrap.json`, with no handwritten digest or relaxed
admission check; all twelve native asset-pack tests then pass. This precedes
rebuilding the optional profile. These optional native
bytes are separate from the unchanged full-runtime measurement cohort.

Verification requires WebCrypto. The supported measured environments are HTTPS
and trustworthy localhost; initialization refuses when `crypto.subtle` is
unavailable. Hashing requires a bounded full byte buffer before compilation,
replacing streaming compilation on this path. Network deduplication alone does
not establish a latency or resident-memory improvement.

## Verification and measurements

Both measured native binaries are unchanged from the earlier measurements:

| Profile | Bytes | SHA-256 |
| --- | ---: | --- |
| Full source runtime | 24,476,684 | `3fc8939ca8eef749b01e9a582dc642cf098ee7d7b0ef61275036f9d6d11e2f76` |
| Selected compiled cart | 6,511,691 | `2fbe29b66fdd64c55b9b05eb0f2f8d29ef56aa6cf99444cbdb6dfab8a3d51600` |

The optional Studio profile also passes online/offline demand for Markdown,
SCSS, XSLT and Relax NG, cold offline Studio/worker command startup, 205 cached
pack assets, independent native source reimport, package handoff, cache isolation
and credential/stale/retirement refusal. Its rebuilt binary is 29,172,552 bytes,
SHA-256 `b435cecc91a600b9d86569c06d793ccbb92d39081dfeed3a77090d8e254747f3`.

The earlier [40-run startup archive](cem-components-wasm-startup.md) and
[12-trace acquisition archive](cem-components-wasm-acquisition.md) remain
unchanged. The new [startup distributions](cem-components-wasm-startup-code-sharing.md)
and [diagnostic acquisition evidence](cem-components-wasm-acquisition-code-sharing.json)
use separate output names:

```bash
yarn nx run cem_ql:measure:browser-startup -- --trace-wasm --runs=2 --record=code-sharing --write-doc
yarn nx run cem_ql:measure:browser-startup -- --runs=5 --record=code-sharing --write-doc
```

All 40 fresh-browser observations acquire WASM once; all 120 process-family
PSS snapshots are complete. All twelve diagnostic traces also acquire once,
from the page, while original native inspection still finds independent
initialized page/worker heaps. Worker mode removes one 24,476,684-byte decoded
body (23.3 MiB) under the local no-store policy. This is separate from resident
memory and compressed/CDN transfer costs.

The following medians compare the preserved earlier archive with this run.
Ready time is milliseconds; after-use browser-family PSS is MiB. Each cell is
before → after, with five observations per workload/mode; see the linked
reports for all ranges, native counters and first-use results.

| Full workload | Mode | Ready ms | After-use PSS MiB |
| --- | --- | ---: | ---: |
| Controls | Worker | 1096 → 1070 | 388.0 → 342.3 |
| Data | Worker | 1103 → 1127 | 434.4 → 385.1 |
| Editor | Worker | 1076 → 1068 | 443.5 → 397.1 |
| Controls | Fallback | 819 → 1275 | 321.3 → 321.6 |
| Data | Fallback | 760 → 1205 | 373.8 → 376.6 |
| Editor | Fallback | 740 → 1159 | 381.6 → 381.4 |

Worker after-use PSS medians decline by 45.7–49.3 MiB with no native binary
change. Worker readiness medians remain close to the earlier values. Fallback
readiness medians increase by 419–456 ms, with broad overlapping per-run ranges,
while its memory remains roughly unchanged. This is a measured tradeoff, not a
general startup-speed improvement. Attribute the fallback phases before choosing
another loading change or claiming a cause; hashing/buffering and the changed
compilation path are candidates, not proven explanations. Selected-entry PSS
remains about 265/261 MiB after use; its readiness medians are 118/100 ms
(worker/fallback), compared with 109/99 ms before. These small descriptive
samples establish neither peaks nor fixed budgets or an all-capabilities minimum.

The subsequent [fallback phase diagnosis](cem-components-wasm-fallback-startup.md)
attributes most of the reported half-second increase to Playwright assertion
polling. Matched visibility-only medians shift by 19–65 ms while assertion lag
shifts to roughly 500 ms. Original distributions above remain archived unchanged;
the phase evidence neither supplies replacement PSS totals nor establishes a
cause for the smaller remaining startup difference.

Focused code-asset fixtures cover concurrent demand, independent memories,
failed-load retry, corrupt/truncated/oversized bytes, mismatched/accessor records,
interface mismatch and concurrent/repeated installation. Transport fixtures
cover held readiness, concurrent workers, premature ready, failed acquisition,
retirement and surviving demand. The runtime suite includes 840 passing tests.
Three pre-existing stylesheet expectations now include the optional undefined
portable-owner argument while preserving every original lease/load assertion.
Clean-package verification checks 403 packaged files, imports the package in an
isolated consumer and checks the paired 24,476,684-byte native binary.
The package build passes TypeScript compilation; lint reports zero errors and
830 warnings.

Selected-application action/recovery and deployed offline fixtures preserve
original frame/control state, fresh physical owners, credential refusal and
unknown-write fences. Recovery issues no GraphQL mutation, reconnect adds no
replay, and duplicate transport deliveries apply a trusted mutation once.
The complete Studio driver passes three recovery cycles and twelve retained
edits per mode, with physical groups of seven worker participants or four
fallback participants, four rendering domains, the original 4 MiB group bound,
and unchanged iframe/document/input identities. One earlier worker invocation
exceeded the original 110-second complete-run deadline during native compilation;
the subsequent standalone driver passes both modes without changing that limit.
One earlier deployed invocation timed out on the initial fallback quantity
mutation, before switching offline or invoking recovery. The isolated case and
two subsequent complete nine-case offline invocations pass. The fixture now
retains case, service/request and original browser-error context on that same
deadline; the intermittent initial-mutation stall remains unattributed.
All-capabilities/offline byte totals, clean npm-consumer startup distributions
and native link-closure proof remain separate work.
