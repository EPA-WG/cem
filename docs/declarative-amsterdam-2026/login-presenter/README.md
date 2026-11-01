# Frozen Login presenter project

This is the ordinary Login example project at version **2.8.1**, captured from
source revision **`c5e5b89274175997387a4d0e160a7e6f6262147f`** on 2026-11-01.
Use it to recover the agreed inputs and expected results during preparation or
as a native presenter fallback. It adds no Studio example page or special UI.

- [login.cem-studio.json](login.cem-studio.json) is the portable project archive:
  15 authored resources and 20 command configurations, with their stable IDs,
  source identities, resolver paths and original bytes.
- [example.json](example.json) is the frozen authored definition, including the
  exact arguments, dependencies and intentional method-attribute repair.
- [manifest.json](manifest.json) pins SHA-256 hashes for the archive, definition,
  every resource, and every saved output/diagnostic file. Each checkpoint records
  its configuration ID, source ID, complete native argv and expected exit code.
- The [walkthrough](../login-walkthrough.md) gives the ordered Studio actions,
  edit instructions, expected visible results and comparison/provenance limits.

The archive remains an ordinary portable project, with `login-seed` as its source
project identity. Make an independent editable copy through Studio's existing
project services. The frozen files are reference inputs; editing the live sample
or a saved copy never changes this archive. No existing saved projects, drafts,
browser storage or entered field values are read by the freeze/replay tools.

## Saved checkpoints

All 20 configurations are captured, plus a separate saved result after the
intentional validation failure is repaired. Outputs are the original native
bytes, with adjacent `*.diagnostics.json` files containing the original command
diagnostics. No query result or JSON representation is used as an AST handoff.

| Demonstration | Configuration / saved output | Expected result |
| --- | --- | --- |
| Native source structure | [read-edit](checkpoints/read-edit.cem), [inspect](checkpoints/inspect.cem) | Typed CEM-ML structural presentation with original source coordinates; three unresolved-reference warnings. |
| Baseline validation | [validate](checkpoints/validate.json) | Exit 0, zero errors and three existing warnings. |
| Intentional failure | [validation-repair](checkpoints/validation-repair.json) | Exit 1, `demo.login.method_required` at the original form and three warnings. |
| Saved repair | [validation-repair-repaired](checkpoints/validation-repair-repaired.json) | Restore `@method=post`; exit 0 and the same three warnings. |
| Formatting | [format-pretty](checkpoints/format-pretty.cem), [format-tabular](checkpoints/format-tabular.cem) | Plain CEM-ML with the two native layouts. |
| Source colors | [source-colors](checkpoints/source-colors.html) | Escaped source HTML; styles remain blocked in Studio's sandboxed document preview. |
| HTML conversion | [html-conversion](checkpoints/html-conversion.cem) | Native HTML-to-CEM-ML source export. |
| Semantic comparison | [semantics-html](checkpoints/semantics-html.json), [semantics-cem](checkpoints/semantics-cem.json), [semantics-html-cemql](checkpoints/semantics-html-cemql.json), [semantics-cem-cemql](checkpoints/semantics-cem-cemql.json) | The same 21 named string values; explicit query-result JSON, preserving each query's own report. |
| Rendered HTML | [render-html](checkpoints/render-html.html), [render-cem](checkpoints/render-cem.html), [render-graph](checkpoints/render-graph.html) | Exact expected HTML including its final three spaces; graph destination `generated/login.html`. |
| Live-view source | [live-view](checkpoints/live-view.cem) | Native Parse of the registered element-template; 15 advisory attribute warnings. This saved Parse result is distinct from the interactive preview. |
| Standalone expression | [expression-html](checkpoints/expression-html.json), [expression-cem](checkpoints/expression-cem.json) | `title=Sign in` and `required=2`. |
| Expression source layout | [expression-format-pretty](checkpoints/expression-format-pretty.cem), [expression-format-tabular](checkpoints/expression-format-tabular.cem) | Authored expression text, including its Unicode/comment, without evaluation. The current profiles preserve the same layout. |

The generated HTML files are available as saved presentation artifacts even
without Studio. The live interaction and graph provenance retain their separate
deployed browser acceptance in the walkthrough; a saved Parse result does not
claim to capture focus, user interaction, credentials or runtime owner state.

## Verify and replay

From the repository root, with the project's supported Node/Rust toolchain:

```sh
yarn nx run @epa-wg/cem-studio:verify:login-presenter
```

Nx builds the native CLI and generates the current sample archive before the
checks. The verifier requires the current authored sources and generated Login
archive to match the frozen inputs. It writes those original project resources
to a temporary directory and executes every command with the shared native CLI.
It then compares all 21 output files, diagnostic files and exit codes exactly,
including the original failure and saved repair, and removes the temporary
project. No browser, network connection, authentication service or live
credential is required for this replay. Initial toolchain/dependency installation
is separate; do not advertise an unverified offline installation route.

The test target `test:login-presenter` also checks drift refusal: changed archive
bytes, missing checkpoints, altered arguments, escaping output paths, weakened
credential policy and altered outputs/diagnostics even with renewed hashes.

If input or output drift is intentional, review the source changes and the
walkthrough first. Replace the freeze explicitly, then review the complete diff
and rerun verification:

```sh
node packages/cem-studio/scripts/login-presenter.mjs --write
yarn nx run @epa-wg/cem-studio:verify:login-presenter
```

Normal verification never updates the reference files. The capture command first
executes all configurations against the authored expected contracts and writes
the manifest after producing the checkpoint files. Rehearse any revised baseline
before presenting it. A checksum establishes byte identity, not release readiness
or authority to change the agreed demonstration.

## Credential privacy

Use disposable demo credentials only during the separately tested live preview.
Entered passwords or one-time codes block recovery until explicitly cleared.
Automatic whole-Studio recovery stays disabled, the aggregate budget stays at
4 MiB and the handoff deadline at five seconds. The manifest pins these policies;
the reference archive and outputs contain checked-in project data only. Replaying
the freeze does not export or persist any entered credential.

## Verification record

The initial capture and a fresh native replay matched all 21 outputs and original
diagnostic files byte for byte. The corresponding deployed Login navigation,
independent-copy, schema failure/repair, live-preview and offline draft acceptance
passed at the source revision above. Participant access/install instructions and
timed rehearsal remain separate work.

On 2026-11-01, `test:login-presenter` passed all 12 tests,
`verify:login-presenter` matched all 21 native checkpoint results, and Studio
lint passed. Documentation links and the complete 46-file freeze inventory were
checked. Generated browser screenshots and Vitest attachments are excluded.
