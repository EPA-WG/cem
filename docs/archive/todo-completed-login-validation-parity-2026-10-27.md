# Login validation parity — 2026-10-27

- [x] Reproduce the CLI/worker difference with failing native composed-engine fixtures before changing validation composition.
- [x] Run element-template embedding checks in the shared admitted-source validation path and retire their duplicate CLI Validate/Check pass.
- [x] Preserve the existing declared/runtime binding policy, original diagnostic fields and exact generic-CEM compatibility evidence.
- [x] Verify malformed-source refusal, selector normalization, cancellation, validation time accounting and native reload ownership.
- [x] Verify CLI/common-command/WASM/Studio parity, independent saved/offline sources and complete credential-safe worker/fallback recovery; publish docs and acceptance evidence without changing limits or deadlines.

## Cause and correction

CLI Validate/Check ran an additional CEM-QL template-embedding compiler outside
the engine. Worker commands ran the shared engine's schema validation without
that extra pass. Login consequently produced the same 15 advisory attribute
warnings in both hosts, but an undeclared lexical name failed only in CLI.

The native source-validation stage now supports an admitted-source supplement.
The engine invokes it after ordinary lifecycle/schema validation completes
without hard diagnostics. It supplements the report rather than claiming the
grammar or replacing schema validation. The composed CEM-QL adapter uses the
existing element-template embedding compiler for this source contract. Other
source types and consumer-specific CLI embedding paths keep their behavior.

The compiler receives the original borrowed source, effective identity and
actual loaded syntax. It does not clone source buffers, evaluate bindings,
fetch Login data, render controls or send credentials. Declared locals and the
existing runtime host binding policy remain admitted; arbitrary runtime `$`
bindings are not reclassified as undeclared lexical names in this change.

For the authored Login source, replacing `dom:text(heading)` with
`dom:text(not_declared)` produces exactly one
`cem.ql.render.compile_failed` error at original byte **655**, with the same URI,
source maps and absent line/column values as the established CLI compiler.
All 15 schema warnings remain advisory, with their original facts for the
actual submitted source. Repair clears the error without changing source IDs,
paths, authored configurations or saved copies.

Malformed/unadmitted sources, pending validation and native reload ingress do
not enter this byte-source supplement. Reload tests supply deliberately invalid
placeholder bytes and assert that the supplement cannot reinterpret them.
Cancellation checks surround every callback, and the existing validation time
measurement includes its work. There is no budget or deadline increase.

## Verification

- Native element-template engine fixtures: **4/4 pass**, including Validate/Check, exact error/maps, repair, declared/runtime bindings, root-scope/MIME/schema selectors, syntax admission, cancellation and time budgets.
- Native reload validation: **2/2 pass**, preserving captured owners, missing-capture refusal and no byte-source supplement on reload ingress.
- Targeted CLI source-identity and embedding suites: **7/7 pass**, retaining the original generic-CEM error codes/ranges and canonical element-template behavior.
- Existing source-operation and structured-data regressions: **12/12 pass**.
- Full native CLI library: **549/549 pass**. The expanded Login CLI fixture checks both Validate and Check and requires exactly one original undeclared-name error before repair.
- Public WASM package: **22/22 pass**, including Validate/Check failure, original diagnostics/maps, malformed-source refusal and repair in both Node and browser bindings. A fetch callback refuses any attempted Login data or credential transport.
- Package verification: browser/Node ABI, **657 integrity records** and **32 schema packages** pass; all **12 CEM-QL WASM regression scripts** pass.
- Studio browser source/live-view checks: **3/3 pass**, including independent saved Login sources, original source-operation ownership and live rendering.
- Complete live Login recovery: **2/2 pass** in worker and fallback modes, preserving credential refusal and the original checkpoint, witness and deadline guards.
- Catalog acceptance/audit units: **48/48 pass**; documentation audit retains **22 authored projects, 217 configurations, 222 documented workflows and 32 native packages**.
- Complete native catalog acceptance: **254 configurations**, with **220 passes, 33 intentional diagnostic cases, one known source limitation and zero companion exceptions**; the Feature Tour seed and **32 native schema-package examples** pass.
- Complete worker catalog acceptance: **254 configurations online and 254 offline**; each phase retains **217 passes, 33 intentional diagnostic cases, one known source limitation, three binary execution policy refusals and zero companion exceptions**. Offline output evidence passes, with original projects and saved copies unchanged.
- Assembled demo verification passes: exact inventory/build hashes, all **333 Studio files**, gallery runtime isolation, independent saved/offline HTML/CSS preview, **10 unchanged CSS generators** and **120 canonical playgrounds** plus the live tree gallery.

The remaining known source limitation is portable Studio project CEM Inspect
(`portable-config/inspect-project-cem`), with its existing native unresolved
namespace/element diagnostics. It remains explicit in acceptance and belongs to
the typed command/configuration/control source-adapter follow-up. No assertion
or source-diagnostic exception was weakened for this correction.

The assembled demo retains **898 files** and all four deployed WASMs below
**25 MiB**, without removing capabilities. Binaryen 132.0.0 `-Oz` preserves the
original import/export inventories:

| Runtime | Optimized bytes | SHA-256 | Imports / exports |
| --- | ---: | --- | --- |
| CEM-QL | 23,713,601 | `d8df029be5b9aa40272992367245169ce4b63ed08fdd4ed94da49416785b959c` | 28 / 169 |
| CEM-ML browser and Node | 24,385,675 | `54a11bcba6ccc52bc4af380b9deb0f2415d36c5eb70671a3713b948882db7c06` | 28 / 170 |

## Shared components and credential privacy

The implementation is shared Rust in the engine and composed CEM-QL adapter.
No production UI JavaScript, component styling, binding syntax or Studio-only
validation workaround is introduced. Studio continues to use its existing
declarative shared controls and native command-service results.

Entered passwords and one-time codes continue to block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. Existing 4 MiB
checkpoint/group budgets, 32,768-node witness guard and original deadlines remain
unchanged. Generated binaries, reports, screenshots and Vitest attachments are
excluded from the source commit.
