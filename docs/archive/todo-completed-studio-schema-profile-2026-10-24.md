# Completed: opt-in Studio schema-asset deployment (2026-10-24)

- [x] Add an opt-in native Studio deployment graph for the optional WASM and
  complete schema source pack, preserving the default complete deployment.
- [x] Add native authored-map fixtures separating strict HTML import URL
  expectations from profile asset selection, preserving source provenance and
  rejecting incomplete expectation maps and missing/mismatched HTML URLs.
- [x] Add profile fixtures for native graph output, runtime/pack pairing,
  default-output preservation, scoped cache isolation, cold offline commands,
  declarative demand and generation-bound preparation without credential or
  publication authority bypass.

`@epa-wg/cem-studio:build:schema-assets` emits `dist/schema-assets/` through native
CEM graphs. Its prerequisite `@epa-wg/cem-ml:build:schema-assets` emits the pack and
optional WASM separately from the ordinary npm outputs. Build orchestration
creates named configuration DTOs and generated wasm-bindgen factory inputs;
HTML, module-edge rewriting and deployment byte emission remain native. The
canonical default Studio graph and its authored HTML remain unchanged.

The native `rewrite-importmap @authored-map` selects the exact original URLs to
validate; `source-map` retains deployment source selection and provenance.
Expectation keys must match the destination import keys. Existing paired map
checks and HTML source mismatch/missing diagnostics remain enforced. Omitting
this attribute preserves the existing behavior.

Both command and rendering factories select the pinned optional binary. The
factory closures retain distinct physical heap ownership and their native
retirement gate. The profile cache family includes the registration scope and a
deployment digest covering module inputs, the complete ordinary native output
receipt (including HTML, metadata and samples), the canonical graph and profile
builder. Default cleanup preserves profile caches, and profile cleanup preserves
default caches. Invalid profile identity/digest pairs refuse installation.

## Verification

The following passed with the repository Rust and Node toolchains:

- Native graph lowering cohort: 12 tests, including the new authored-map case;
  transform-config cohort: 24 tests. The extended authored-map case also executes
  the native graph and asserts exact HTML URL mismatch/missing refusals.
- Fresh native CLI and optional release WASM builds. The optional build retained
  the original embedded bodies and left the ordinary npm output intact.
- Studio `build`, `verify:output`, `build:schema-assets`,
  `verify:schema-assets` and `typecheck` Nx targets. These focused runs used
  `--excludeTaskDependencies` after rebuilding their native prerequisites.
- Shared schema-asset Vitest cohort: 35 tests across three files.
- `node tools/scripts/verify-cem-schema-assets-offline.mjs`: 205 assets,
  declarative demand, cold offline source import/reactivation, distinct heaps,
  cleanup, missing source refusal and invalid/incomplete inventory refusal.
- ESLint for both new profile scripts and the service worker; `git diff --check`.

The new emitted-output browser fixture uses isolated browser storage, never the
user's projects, drafts or caches. It checks all 205 native-emitted pack files
against their original bytes, both deployed binary hashes and factory bootstrap
pins, and default/optional cache coexistence in both installation orders. It
physically disconnects the HTTP fixture as well as applying browser offline mode,
then reloads Studio and runs a real `version` worker command successfully.
Declarative `http-request` demand uses the shared runtime/loader. Original source
bytes reimport on a fresh heap with independent native owners and identical
provenance. Stale source bindings and entered-credential host admission refuse;
live package owners prevent retirement until explicitly disposed. Invalid or
orphaned profile metadata refuses installation without creating caches.

Machine-readable evidence is generated at
`dist/reports/cem-studio/schema-assets-profile.json`, with native graph reports
in `schema-assets-build.json` and `schema-assets-samples.json`.

## Remaining boundary

Deployment and preparation grant no coordinated publication or credential
permission. A trusted embedding host must install the shared HTTP opener and
provide explicit native replacement grants; delivery metadata and demand URLs
cannot do so. The normal Studio bootstrap does not activate optional packages
implicitly. A package-owner adapter for coordinated command/render group
publication remains an explicit next task. Fresh preparation must remain
subordinate to that publication/ownership confirmation; no raw owner or grant
crosses a status or checkpoint DTO.

Entered passwords/one-time codes continue to block recovery until cleared.
Automatic whole-Studio recovery remains disabled. Embedded-body exclusion,
complete Studio demand integration and download/latency/headroom measurements
remain unfinished. This profile makes no binary-size, checkpoint-size or memory
savings claim.
