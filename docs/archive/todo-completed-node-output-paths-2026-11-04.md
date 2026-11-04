# Node output URI binding — 2026-11-04

- [x] Reproduce relative `--out` refusal before Parse and add native lowering,
  Node worker, executable and strict admission/publication regressions.
- [x] Bind output runtime destinations in shared Rust lowering while preserving
  authored paths, normalized config/scope identity and host authority checks.

`createNodeCommandService({ cwd })` previously lowered an ordinary relative output
as `runPlan.outputs.resolvedDestination = "inspection.cem"`. Common service
admission correctly refused it with `cem.command_service.uri_invalid` before
Parse could inspect the input. The Node executable therefore returned internal
exit 7, even when malformed CSV should produce the established Parse exit 1.
An explicit `file:` output reached the expected CSV refusal and preserved the
previous file. Four new Node regressions and two native regressions reproduce
the missing binding against the old implementation.

The fix is in the [shared Rust lowerer](../../packages/cem_ml/src/command_invocation.rs),
using the existing `resolve_host_uri` helper for normalized output destinations,
as already done for inputs/schema resources. It does not rewrite parsed argv,
`declaredDestination`, effective config, output scope, provenance or explicit
URI addresses. There is no parallel Node output parser and no change to Parse,
command-service URI validation, publication validation or host write grants.

| Destination | Runtime binding |
| --- | --- |
| Relative path without config | Resolve against invocation cwd, normalize dot segments, encode as absolute file URI |
| Absolute local path | Normalize/encode as a file URI |
| Explicit `file:`, application URI or stream | Preserve the existing address |
| Config-relative output | Preserve the normalizer's config-base policy, then bind its runtime address |
| Omitted CLI `--out` | Keep the existing stdout stream destination |
| Destination absent in a normalized output | Keep the existing in-memory policy |

The regression uses directory/file names containing spaces, Unicode, `#` and
`%` so file paths cannot accidentally become fragments or double-escaped URIs.
The original explicit-file-URI CSV case remains intact. Malformed CSV must retain
its original diagnostic URI/byte span, exit 1, zero output artifacts and prior
file contents. Repair must publish the correct file with exit 0. Both service
and actual executable paths are exercised.

The hostile-request case still rejects a forged relative prepared destination
before any host callback. A valid canonical output whose host commit returns a
relative URI must fail publication, roll back the already committed write and
restore the previous file, with zero published artifacts. This confirms the
lowering repair does not authorize invalid callback results.

## Verification

- Before the fix: two native output-binding tests and four Node worker/executable
  regressions fail at the missing output URI binding.
- After the fix: 16 shared invocation tests, eight service admission tests and
  six transactional publication tests pass (30 native checks).
- Rebuilt optimized default WASM and CLI packages pass, retaining generated
  command schema v1 and the existing native interface. Optimized WASM is
  24,860,843 bytes, within the existing 25 MiB gate.
- Full CLI Node suite: 61/61 pass, including all-command/executable matrices,
  all four new regressions, original explicit-file CSV refusal/repair,
  worker/cancellation, admission and retirement checks.
- Browser worker-pool and dedicated-worker command-service acceptance pass,
  including all portable operations, both transform sources, callback/cleanup,
  separate heaps, compiled-code reuse and original startup-deadline refusal.
- CLI lint passes with the existing unused `copyFileSync` import warning in
  `scripts/build.mjs`; no new lint error is introduced.
- The final Amsterdam delivery archive SHA-256 remains unchanged. No public Site
  deployment, frozen presentation material, UI code, tokens or recovery policy
  changed.

Focused verification used prepared workspace dependencies and Linux x64 / Node
24.16.0. Commands (repository root):

```sh
yarn nx run cem_ml:test --lib --args=command_invocation --excludeTaskDependencies --skip-nx-cache
yarn nx run cem_ml:test --lib --args=command_service::tests --excludeTaskDependencies --skip-nx-cache
yarn nx run cem_ml:test --lib --args=command_publication::tests --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-ml:build --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-ml-cli:build --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-ml-cli:test --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-ml-cli:test:browser --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-ml-cli:lint --excludeTaskDependencies --skip-nx-cache
```

The next linked-package source-map diagnostic remains separate. Its focused
pretyped handoff case still reproduces at `attribute_datatypes.rs:473`: the
candidate retains the additional `SourceBoundary { uri: "library.cem" }` frame.
No source-map assertion, datatype-preparation runtime or provenance was changed
as part of this output-path repair.
