# Schema-asset profile measurements

Keep the complete Studio deployment as the default. The current external-source
profile reduces initial native linear-memory capacity by 256 KiB, but its CSV
activation increases cold-command cost and retained linear capacity. Its complete
offline deployment also costs more bytes. These observations do not justify a
separate rare-command WASM ABI/heap yet. Optional demand now follows selected
project/resources; startup and first-use costs are measured separately before
expanding adoption.

## Reproducing the measurements

```sh
yarn nx run @epa-wg/cem-studio:test:schema-measurements
yarn nx run @epa-wg/cem-studio:measure:schema-assets
```

The uncached, nonparallel measurement target depends on default output and
schema-assets acceptance. It writes the ignored report
`dist/reports/cem-studio/schema-assets-measurements.json`, including every raw
sample, distributions, runtime hashes, native memory configurations, inventories,
response paths, cancellations and environment. A failed run records partial
samples with `status=failed`; it cannot leave an earlier successful report in
place. The script also accepts `--samples=3` through `--samples=9` when invoked
directly after preparing the deployments. The default is five.

Each of the three workloads runs five times per profile, using **30 fresh Chromium
processes and contexts** in total. Runs are serial and alternate profile order.
The server preloads deployment files, sends uncompressed loopback responses with
`Cache-Control: no-store`, and blocks service workers. Native factories use fresh
physical heaps. The source-paint fixture and real Studio view use the same
703-byte published Login HTML example; its exact source and native paint result are
checked. CSV validation uses identical fixed public fixture bytes and verifies
equivalent native command outcomes. Native cleanup must leave a complete,
nonblocking inventory and zero active operations/reservations.

Command timing includes factory import/initialization, optional loader import,
source acquisition/activation, native run-plan normalization, host callbacks,
validation and artifact cleanup. It measures the native command-service boundary
on an isolated factory, without the CLI worker transport. Native paint timing
uses the editor's `sourceHighlightPaintV1` operation on a distinct rendering
factory. Actual Studio timing starts at navigation to
`?sample=login&file=login-html` and ends at its first native-highlight readiness,
with the selected source verified and zero optional demand across at least four enrolled
runtimes. A subsequent fixed CSV parse measures the first dependent Studio
command including native planning and coordinated activation; its delivery is
recorded separately from the editor-readiness cutoff. This measures the readonly example editor, not typing latency or
credential entry. No source text or credentials are written to the report.

A small sample count and uncontrolled OS/filesystem caches, host scheduling and
thermal state limit inference. Browser processes are fresh; the host machine and
server are not reset. Timings are observations without pass/fail speed thresholds
or a claim of statistical significance. The profiles also differ in native ABI
and metadata/initialization paths, so this is a comparison of complete deployed
profiles, rather than causal attribution to source-body removal alone.

## Historical eager-demand run, 2026-10-24

Artifact baseline: `d12e3ef9`; Node 24.16.0, Chromium 148.0.7778.96, Linux/WSL2
6.6.114.1, Intel Core Ultra 7 258V, eight logical CPUs. Values below are medians;
parentheses give observed minimum–maximum milliseconds.
This historical Studio readiness cutoff included unconditional CSV activation.

| Workload | Complete | External sources |
| --- | ---: | ---: |
| First CSV validation result, including setup | 342 (313–480) | 998 (880–1156) |
| Explicit CSV activation stage | 0 | 771 (699–864) |
| CSV operation after setup | 231 (191–245) | 99 (93–108) |
| Native HTML paint call | 58 (51–64) | 61 (55–72) |
| First native paint result, including factory setup | 158 (145–167) | 165 (145–181) |
| Studio Login HTML editor readiness | 6208 (5839–6686) | 8930 (8173–9922) |

Optional activation prepares its native context before validation. Its 99 ms
operation is part of the 998 ms first-result total. Eager CSV demand also adds
work to Studio startup even when the selected HTML source needs no CSV package.
Native paint operation results match exactly across profiles and all samples.
Two superseded startup requests were cancelled with `ERR_ABORTED` and remain in
the report. Other transport errors refuse the measurement; cancellations do not
waive editor/source/demand readiness or native ownership checks.

## Delivery

Raw and gzip level-9 sizes are bytes. Offline totals sum every deployed file by
URL, compressing each file separately; HTTP headers, TLS and packaging are
excluded. The optional factories currently use the same binary bytes under two
different URLs. Those two responses remain part of deployment/transport costs.

| Artifact or deployment | Complete raw / gzip | External raw / gzip |
| --- | ---: | ---: |
| Command WASM | 27,120,456 / 7,251,862 | 27,090,472 / 7,303,244 |
| Rendering WASM | 26,341,799 / 7,021,407 | 27,090,472 / 7,303,244 |
| Complete offline deployment | 58,145,560 / 15,153,602 | 60,358,303 / 15,728,652 |
| Deployed files | 313 | 518 |

The complete optional inventory includes all **205 pinned pack files**. The
optional offline deployment adds 2,212,743 raw bytes and 575,050 gzip bytes in
this comparison. Its command binary alone is 29,984 raw bytes smaller and 51,382
gzip bytes larger. The report records both factory binary hashes and the optional
deployment digest.

Median Studio startup response bodies offered by the local server totalled
111,543,356 complete-profile bytes and 113,969,563 external-profile bytes. Several
physical runtimes request assets in an uncached startup. This is the sum of full
server response bodies through the readiness cutoff, including any recorded
cancelled request; it is not received TCP bytes, production compressed transfer,
or a unique-content cache estimate.

## Native allocator and linear-memory observations

All five samples give the same listed memory values per workload. These are
isolated command/rendering factory heaps, not aggregate Studio heaps, JavaScript
loader storage, compiled-code residency or browser RSS. Allocator values count
requested allocation sizes. Linear memory reports `WebAssembly.Memory` capacity,
which remains allocated after native owner cleanup.

| Observation | Complete | External sources |
| --- | ---: | ---: |
| Initial native linear capacity, either factory | 3,866,624 B | 3,604,480 B |
| Paint: allocator live / lifetime peak after operation | 4,681 / 950,194 B | 4,681 / 950,194 B |
| Paint: linear capacity after operation | 4,915,200 B | 4,653,056 B |
| CSV: allocator live after operation | 1,172 B | 1,638,155 B |
| CSV: lifetime allocator peak | 6,656,397 B | 7,933,829 B |
| CSV: linear capacity after operation and cleanup | 15,269,888 B | 18,612,224 B |
| CSV: allocator live after cleanup | 1,172 B | 51,942 B |
| CSV: normal-work policy headroom after operation | 991,363,072 B | 988,020,736 B |

Native admission retains its **768 MiB soft limit, 1 GiB hard limit, 64 MiB
recovery reserve and 64-operation bound**. Normal-work headroom is the nonnegative
remainder of `hardLimit - recoveryReserve - max(allocatorLive, linearCapacity) -
reservedBytes`. Lifetime peak is reported separately; it is not added to live
allocation or capacity. Every sampled endpoint is quiescent with its existing
reserve intact. The optional CSV owner releases native allocations on cleanup,
but its larger linear capacity persists. Neither checkpoint savings nor reduced
whole-Studio memory is established.

## Project/resource-driven demand run, 2026-10-24

Source baseline: `851a5a1e` plus the accompanying resource-demand change.
Environment and fixture match the recorded method above. All **30 fresh-browser
samples passed**. Every optional HTML editor sample reached readiness with four
enrolled runtimes, zero retained demands and **no schema-asset HTTP requests**.
The subsequent first CSV command completed with one demand across those four
physical runtimes. Native command/paint outcomes remain equivalent across profiles.

| Workload, median (minimum–maximum ms) | Complete | External sources |
| --- | ---: | ---: |
| Studio Login HTML editor readiness | 7625 (7503–7832) | 8130 (7726–8375) |
| First dependent Studio CSV parse, including preparation | 689 (656–720) | 3098 (3058–3368) |
| Isolated CSV validation, including factory setup | 427 (390–441) | 1190 (1145–1333) |
| Isolated CSV activation stage | 0 | 943 (914–990) |
| Native HTML paint call | 64 (55–87) | 72 (70–82) |

The first dependent Studio command adds **36 responses / 822,756 raw body bytes**
for the optional profile and zero additional responses for complete. These bytes
are recorded separately from editor readiness: startup response bodies total
111,554,926 complete / 113,386,055 optional bytes. Two superseded startup requests
were recorded as `ERR_ABORTED`; other transport failures remain fatal.

| Current artifact or deployment | Complete raw / gzip bytes | External raw / gzip bytes |
| --- | ---: | ---: |
| Command WASM | 27,120,456 / 7,251,862 | 27,146,065 / 7,314,091 |
| Rendering WASM | 26,341,799 / 7,021,407 | 27,146,065 / 7,314,091 |
| Complete offline deployment | 58,154,482 / 15,155,353 | 60,481,961 / 15,752,486 |

The optional binary SHA-256 is
`ec3269d5228e34dc3cecf05714ccd596d808e1c64d5beac10aa0b92d0f89b2f1`;
its deployment digest is
`bb82420ecfdc71f076967625637ea25938322b37e42ddc7e2cd5262da8095cde`.
The complete binary hashes remain unchanged from the historical run. Optional
offline delivery adds 2,327,479 raw / 597,133 gzip bytes. The isolated native
memory observations in the table above are unchanged, including the 256 KiB
initial/paint capacity reduction and the larger retained CSV capacity. They do
not measure aggregate Studio memory or checkpoint size.

Demand is deferred to actual resource/command use, including output-only CSV
conversion and native graph dependencies. It changes when activation costs are
paid; the two runs are not a causal performance comparison or a speed guarantee.
The optional profile remains experimental. The separate plaintext 200-line
editor settling timeouts reproduce on the frozen previous commit and are the
next Studio functionality item; this 703-byte HTML readiness measurement does
not characterize them.

## Next decision

Keep the external-source profile experimental and preserve the complete default.
Project/resource-driven demand retains trusted grants and complete physical-runtime
publication before a dependent job. Add trusted demand for the other excluded
formats next. A matched-ABI
comparison and whole-workspace runtime census remain useful before proposing a
separate rare-command heap; this run establishes no need to allocate one.

UI composition remains declarative; native requirements and acquisition are
service responsibilities. Entered passwords and
one-time codes still block recovery until explicitly cleared, automatic
whole-Studio recovery remains disabled, and checkpoint, loader and allocation/
retirement limits are unchanged. Saved projects/drafts in existing browser
sessions were never opened or cleared.
