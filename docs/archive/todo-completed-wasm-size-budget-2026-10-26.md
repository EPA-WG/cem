# Complete-runtime WASM deployment budget — 2026-10-26

- [x] Measure pinned Binaryen `-Os`/`-Oz` against both complete post-bindgen runtime binaries, retaining import/export descriptors, capability/protocol manifests and real native execution.
- [x] Integrate the selected `-Oz` mode into both shared builders before factories, package integrity, runtime vendoring and native Studio graph publication; cache the optimizer identity and reports through Nx.
- [x] Add optimizer and strict deployment-budget fixtures, including refusal before mutation, retained-file preservation, exact 25 MiB refusal and replacement/retirement of oversized owned outputs.
- [x] Rebuild and publish all four WASM assets below 25 MiB while keeping fewer than 1,000 files; run Studio/demo/runtime checks and retain explicit follow-ups for observed test failures.

## Measured and reproduced output

| Runtime | Post-bindgen input | `-Os` | Selected `-Oz` |
| --- | ---: | ---: | ---: |
| CEM-QL | 26,630,206 bytes | 23,667,268 bytes | **23,416,451 bytes / 22.33 MiB** |
| CEM-ML common-query bridge | 27,408,583 bytes | 24,339,027 bytes | **24,066,033 bytes / 22.95 MiB** |

The regular release builders reproduced the measured outputs byte-for-byte:

- CEM-QL SHA-256: `6c0e8b5e3ef4bd8b0c80f15b6b6b3d62233aa36b313e46bf91c87d0a243b4240`.
- CEM-ML SHA-256: `bc064a6072ce02c985abc32cb4f73cbce4c9eba7dbef340460aeed34b18e0a25`.

All 169 CEM-QL exports, 170 CEM-ML exports and 28 imports per module remain.
Capability and worker-protocol manifests, query evaluation, template rendering,
HTML output and highlighting compare exactly with their unoptimized instances.
The CEM-QL WASM target-feature listing also matches its original input. Keep the
full embedded schemas, default `debug-control` capability and native startup
bridge; no feature-removal, feature-expansion, closed-world or relaxed-math
options were used. Browser/Node binaries share optimization only when their
original bytes match; their target-specific glue remains separate.

The deployment contains **898 files, 114,301,242 bytes (109.01 MiB)**, including
its inventory. Gallery and Studio each retain a separately paired CEM-QL and
CEM-ML WASM/loader deployment. The largest file is 24,066,033 bytes. Neither
chunks nor optional schema-asset profiles are needed.

Publication preflights the intended outputs, retained regular files and generated
inventory against the strict below-25-MiB bound and the existing 999-file total.
Unknown and modified obsolete files remain preserved and counted; budget overflow
refuses before copying or retiring files. Reports, screenshots, attachments,
native binaries and generated WASM remain excluded from Git/deployment inputs.

## Verification

- Tooling: **19/19** optimizer, deployment and local-hosting cases pass.
- Catalog contracts: **23/23** pass; Studio typechecking passes.
- Common runtime: **10/10** browser/Node tests pass; package verification passes
  for both ABIs, **657 integrity records and all 32 schema packages**.
- CEM-QL: **11/12** boundary scripts pass, including checked compilation,
  checkpoints, constraints/native values, generation ownership, render/CSS
  caches, memory/headroom accounting and the 30-case shared URL matrix.
  The remaining XML byte-roundtrip failure also reproduces with the original
  unoptimized binary, as described below.
- Studio source cohort: **273/274** cases pass across workbench, native samples,
  navigation and recovery. The sole failure is a JavaScript dynamic-module fetch
  in whole-workspace fallback recovery; the unchanged isolated case then passes
  in 45.6 seconds. All other recovery cases, including live Login privacy on
  worker and fallback, pass in the full invocation. Do not count the isolated
  rerun as a green full-cohort invocation. An initial default-reporter invocation
  was interrupted to enable per-test diagnostics; it produced no completed-file
  result and is not acceptance evidence.
- Deployed demo acceptance passes: **all 333 Studio files match their build bytes**, saved-copy
  selection and native HTML/CSS previews work, offline reload/rebuild matches
  online output, service-worker scope stays under `/studio/`, all ten theme
  generators emit unchanged CSS, all 120 canonical playground URLs respond,
  and the live tree gallery renders. Shared initialization and independent
  factory memories, native rendering/highlighting and every owned file hash pass.

All existing deadlines, the 4 MiB checkpoint budget, entered-password/OTP refusal
and disabled automatic whole-Studio recovery remain unchanged. No Studio or
component UI JavaScript was introduced; reusable optimization lives in shared
build tooling.

## Preserved follow-ups

`packages/cem_ql/tests/reference-transport-wasm.mjs:105` still rejects an XML
reload bundle whose export/import roundtrip changes namespace/attribute ordering.
Both the original unoptimized release
`06b202cdf4ec0083497092b416361bb197d395fe71907ec1bafc2d9083c723b4` and `-Oz`
fail the original strict byte assertion. Keep that assertion and native ownership
guards while adding native deterministic-order fixtures before repairing it.

The full Studio invocation's fallback failure occurred after fallback WASM
initialization and before workspace construction:
`Failed to fetch dynamically imported module: .../cem-elements/dist/index.js`.
That unchanged file was present and returned HTTP 200; the isolated original
case passes. Attribute this aggregate-only loading failure before changing the
runtime or test harness. Both items remain explicit in [todo.md](../todo.md).

See [the size analysis](../wasm-deployment-size.md) for timing, memory, workflow
and reproducible comparison commands.
