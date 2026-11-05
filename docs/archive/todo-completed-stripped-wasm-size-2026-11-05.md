# Stripped WASM size investigation — 2026-11-05

- [x] Establish the current source/toolchain cohort and inspect compiled debugger
  omission independently of binary size.
- [x] Verify freshly prepared artifacts with the original strict size and
  omission assertions and record the final byte/hash baseline.

## Historical failure and initial checkout

The original TODO recorded **27,264,508 bytes stripped** versus **27,033,499
bytes default**, with debug capability activation already refused. Those older
numbers remain historical evidence. Their complete source/build-stage receipts
were not available in this investigation; no specific compiler or retention
cause is assigned to that earlier inversion.

At the beginning of this investigation, installed default WASM was
**24,860,843 bytes**, SHA-256
`2b3cd1318545f57c6802a77886fe25c54ffb7f4c5b37cf327741c37d304d8e2f`.
Stripped still had the older **27,264,508-byte** artifact, SHA-256
`107a441aca18c9af2629668495385a5ab95b371970478761c873abb59ff8efb2`.
Only default had the current optimization report. The unchanged leaf verifier
rejected the pair at its Rust-owned command declaration comparison, before
reaching binary-size ordering. These installed artifacts were not a matched
current cohort.

## Matched current producers

The measured production source revision is `2da78b23ba133605eb4318477a5923a79f948741`.
This work changes documentation/evidence only. Rust sources, build scripts,
Cargo features, optimizer settings, public declarations and all existing
verifier assertions are unchanged.

The toolchain is Rust **1.96.0**, wasm-bindgen **0.2.122** and Binaryen
**132.0.0**. Cargo release uses optimization level **3**, no debuginfo and the
workspace's existing LTO setting. Both linked producers subsequently report
`fresh: true` with identical release settings; default has
`debug-control,default`, stripped has no enabled root features. Cargo's normal
dependency feature inventory confirms `cem-ml` and `cem-ql` also omit
`debug-control` in stripped.

Default's current linked producer reproduces the recorded pre-optimization
bindgen input exactly for **both** browser and Node: **28,239,585 bytes**, SHA-256
`b0a2dbb638164a4f486994042601e9a8df73904d038a1eadab75c00ede6b6a31`.
This used temporary bindgen output, preserving the installed optimized package.
The freshly rebuilt stripped bindgen input is **27,955,980 bytes**, SHA-256
`c3764da48d1b353056754b11fa870564f73ed9cc402ad91ad008d4bb8207ad63`.

The [machine-readable receipt](../stripped-wasm-profile-measurements.json)
records the tested revision, toolchain, a digest of **2,093 tracked local producer
corpus files**, Cargo freshness/settings, linked/function evidence, final
integrity/manifest identities and native CLI hashes. Its explicit source roots
and digest method are recorded; Cargo freshness independently covers Cargo's
dependency fingerprint.

## Independent omission evidence

The pre-bindgen linked WASM `name` section was decoded and its function names
demangled with GNU `c++filt -s rust`. Current default has **24,430 named
functions**, including **17** whose names contain `cem_ml::debug_control::`.
Current stripped has **24,329 named functions** and **zero** matching debugger
module prefixes (`cem_ml::debug_control::`, `cem_ml::dap::`,
`cem_ql::debug_control::`). Default's linked WASM also has zero matches for the
last two prefixes, so those are not claimed as additional removed functions.

The old stripped linked producer also had zero matching debugger module names;
its larger installed file was not evidence that these named debugger functions
had leaked. Named-function counts are structural evidence, not per-function byte
attribution or a proof based solely on searching arbitrary data strings.

The public export sets contain **172 default / 168 stripped** exports. Stripped
adds none and omits exactly:

- `pauseResumableOperation`
- `acknowledgeResumableOperationStop`
- `continueResumableOperation`
- `stepResumableOperation`

The production optimizer itself requires the public import/export descriptors
to remain unchanged. Final capability, native-symbol, compile-surface and CLI
checks remain the authority for the other admission/omission boundaries.

## Verified final baseline

| Stage / artifact | Default bytes | Stripped bytes | Reduction |
| ---------------- | ------------: | -------------: | --------: |
| Linked WASM, before bindgen/name removal | 32,858,089 | 32,555,500 | 302,589 |
| Bindgen WASM, before Binaryen | 28,239,585 | 27,955,980 | 283,605 |
| Final browser WASM, Binaryen `-Oz` | 24,860,843 | 24,544,191 | 316,652 |
| Final Node WASM, Binaryen `-Oz` | 24,860,843 | 24,544,191 | 316,652 |
| Native debug-profile CLI | 531,644,632 | 513,354,248 | 18,290,384 |

Browser and Node bytes are identical within each profile. Final stripped WASM
SHA-256 is `c9d8878f781a0a3b9379ff266c7d92525098cd1a0b5ddc3e58201de0065aa22f`.
Both final profiles remain below the existing **25 MiB** file ceiling. These are
aggregate profile deltas, not a per-function byte allocation or a compression,
HTTP-transfer or runtime-memory measurement.

The original uncached artifact verifier **passes** after producer refresh. It
retains every assertion: strict smaller browser/Node WASM and native CLI sizes,
all-file integrity, exact shared command declarations, isolated ABI identities,
debug capability activation refusal, unavailable debugger controls, retained
core controls, absent debugger WASM bindings/native symbols, missing native CLI
debug command, no stripped Cargo debug feature and refused Rust debugger imports.
The package build also preserves import/export descriptors through optimization
and regenerates factory pins/integrity afterward. The stripped package contains
**658 integrity records / 481 schema-package assets**.

Nx labels the failed-then-successful leaf task as flaky. Those invocations used
different prepared artifacts: first the old unmatched installed pair, then the
fresh matched producers. `--excludeTaskDependencies --skip-nx-cache` bypassed
producer execution/cache for the comparison itself. The label is retained as
tool output; it is not evidence of an identical-artifact runtime failure or an
unchanged-input timing repair.

The frozen presentation archive retains its exact size and SHA-256. Existing
stripped-build unused-field/function warnings remain. No threshold was relaxed,
no runtime/optimizer setting changed and no fixture was skipped to obtain this
artifact result. The practical correction here is preparing a current matched
cohort and recording its provenance; the current strict artifact gate passes.

## Reproduction and scope

```sh
yarn nx run @epa-wg/cem-ml:build:stripped --skip-nx-cache
yarn nx run cem_ml_cli:build --skip-nx-cache
yarn nx run cem_ml_cli:build:stripped --skip-nx-cache
yarn nx run @epa-wg/cem-ml:verify:stripped --excludeTaskDependencies --skip-nx-cache
```

All commands use Node **24.16.0**, `NX_DAEMON=false`, the pinned Rust toolchain,
and `CARGO_BUILD_JOBS=2` for builds. Default Cargo freshness and exact bindgen
input reproduction avoid repeating its already validated optimization pass.
The [package README](../../packages/cem-ml-npm/README.md#debug-control-and-stripped-profiles)
also documents rebuilding all four producers together when no matched default
artifact is available.

Artifact-only verification uses prepared dependencies. The full configured
native test graph still has the separately recorded version-3 binary fixture
failure; this investigation does not claim a green complete native release.
The original size ordering, file ceiling, capability refusals and recovery
contracts are preserved. No new public deployment is made.
