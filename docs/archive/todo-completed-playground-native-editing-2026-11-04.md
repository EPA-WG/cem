# Playground native editing regression — 2026-11-04

- [x] Reproduce the existing Native Editing And Source Preservation failure
  with the unchanged runtime and original assertion.
- [x] Correct the integration fixture to check native DOM preservation, exact
  source and selection while respecting the shared terminal caret contract.

The original assertion counted all element children and required zero after
Unicode typing into a source with two trailing newlines. The single element was
the shared editor's deliberately retained terminal `<br>`, not syntax-color
markup or parsed user HTML. Removing it would contradict the adopted
[source-editor contract](../../packages/cem-components/docs/source-editor-contract.md)
and its whole-source clipboard regression, which preserves the final LF/CRLF.
The canonical `LosslessColoring` story also explicitly requires the terminal BR.

The Playground story now admits exactly one unadorned terminal BR, then checks
serialized native markup and every child-node identity before/after actual
syntax paint and rendering. Exact text, UTF-16 selection, retained control,
trailing blank lines, native undo/redo and language-change behavior remain
asserted. No plaintext runtime, app behavior, declarations, styles, timeouts or
public assets changed. Other no-trailing-newline stories retain zero-element
assertions.

The previously frozen presentation PDF, kit, public release and final transfer
are unchanged by this test-only repair. Existing theme/inheritance, forced-color,
credential/refusal and checkpoint contracts remain unchanged.

## Verification

Prepared workspace, Node 24.16.0, Playwright Chromium 148.0.7778.96:

- Original focused story: one failure at the zero-element assertion (19 skipped).
- Corrected focused story: one pass (19 skipped), retaining all native-editing
  checks and adding exact DOM/node-identity assertions.
- Complete `@epa-wg/cem-demo-element:test`: 20/20 pass across both story files.
- Package lint and typecheck pass.
- Canonical `cem-source-editor` cohort: 10/10 pass, including LF/CRLF whole-source
  replacement, trailing-newline input/paste, lossless worker/fallback coloring,
  composition/selection, theme inheritance and retained native editing.

Reproduce with existing prepared dependencies (Nx task dependencies excluded
without rebuilding the frozen deployment):

```sh
NX_DAEMON=false yarn nx run @epa-wg/cem-demo-element:test \
  --excludeTaskDependencies --skip-nx-cache
NX_DAEMON=false yarn nx run cem-elements:test \
  packages/cem-components/src/components/cem-source-editor/cem-source-editor.stories.ts \
  --excludeTaskDependencies --skip-nx-cache
NX_DAEMON=false yarn nx run-many -t lint,typecheck \
  -p @epa-wg/cem-demo-element --excludeTaskDependencies --skip-nx-cache --parallel=1
```
