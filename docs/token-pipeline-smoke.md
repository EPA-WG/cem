# Token Pipeline Smoke

Validation run: 2026-05-01.

## Source Change

Changed one source token in `packages/cem-theme/src/lib/tokens/cem-colors.md`:

```text
--cem-color-blue-xl: #ecf0ff -> #e0e8ff
```

## Commands

Successful targeted builds:

```bash
yarn nx run @epa-wg/cem-theme:build
yarn nx run @epa-wg/cem-theme:build:token-platforms
```

Root build attempt:

```bash
yarn build
```

## Result

Both targeted commands completed successfully.

The root `yarn build` command did not reach target execution. It failed while loading Nx plugins:

```text
Failed to load 3 default Nx plugin(s)
Failed to load 3 Nx plugin(s): @nx/js/typescript, @nx/eslint/plugin, @nx/vitest
```

Retrying after `yarn nx reset` and retrying with `NX_DAEMON=false yarn build` produced the same plugin-worker startup
failure. The Nx daemon log reported `EPERM` while listening on `/tmp/.../d.sock`.

Build notes:

- Manifest validation passed for all generated CSS files.
- Token coverage stayed complete: 418/418, gap 0.
- Token export extracted 421 tokens from 10 specs.
- Canonical visual token count stayed 371; voice token count stayed 42.
- Figma token count stayed 230, with 141 excluded tokens listed in the Figma report.
- Platform export emitted 371 tokens across 5 JSON mode files.
- Platform validation passed: 371 tokens consistent across 5 JSON mode files.
- Android report showed 0 fail-hard violations.
- iOS report showed 0 fail-hard violations.

Expected existing warnings:

- Three deprecated dimension tokens are not resolved in CSS:
  - `--cem-layout-inline-tight`
  - `--cem-layout-inline`
  - `--cem-layout-inline-loose`
- Five optional visual tokens are skipped by canonical emission:
  - `--cem-layout-stack-tight`
  - `--cem-layout-stack-loose`
  - `--cem-bend-control-round-ends`
  - `--cem-layer-back-deep`
  - `--cem-layer-work-floating`

## Propagation Check

The new value `#e0e8ff` appears in:

- Source markdown: `packages/cem-theme/src/lib/tokens/cem-colors.md`
- Generated CSS: `packages/cem-theme/dist/lib/css/cem-colors.css`
- Combined CSS: `packages/cem-theme/dist/lib/css/cem-combined.css`
- Built token XHTML: `packages/cem-theme/dist/lib/tokens/cem-colors.xhtml`
- Canonical JSON: `packages/cem-theme/dist/lib/tokens/cem.tokens.json`
- Resolved JSON: `packages/cem-theme/dist/lib/tokens/cem.tokens.resolved.json`
- TypeScript metadata: `packages/cem-theme/dist/lib/tokens/cem.tokens.ts`
- Figma mode files:
  - `packages/cem-theme/dist/lib/tokens/figma/cem-light.tokens.json`
  - `packages/cem-theme/dist/lib/tokens/figma/cem-dark.tokens.json`
  - `packages/cem-theme/dist/lib/tokens/figma/cem-contrast-light.tokens.json`
  - `packages/cem-theme/dist/lib/tokens/figma/cem-contrast-dark.tokens.json`
  - `packages/cem-theme/dist/lib/tokens/figma/cem-native.tokens.json`
- Flat platform JSON:
  - `packages/cem-theme/dist/lib/token-platforms/json/cem-tokens-light.json`
  - `packages/cem-theme/dist/lib/token-platforms/json/cem-tokens-dark.json`
  - `packages/cem-theme/dist/lib/token-platforms/json/cem-tokens-contrast-light.json`
  - `packages/cem-theme/dist/lib/token-platforms/json/cem-tokens-contrast-dark.json`
  - `packages/cem-theme/dist/lib/token-platforms/json/cem-tokens-native.json`
- iOS outputs:
  - `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`
  - `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.xcassets-hints.json`
- Android outputs:
  - `packages/cem-theme/dist/lib/token-platforms/android/values/cem-tokens.xml`
  - `packages/cem-theme/dist/lib/token-platforms/android/values-night/cem-tokens.xml`
  - `packages/cem-theme/dist/lib/token-platforms/android/compose/CEMTokens.kt`

The old value `#ecf0ff` was not found in the checked source and generated token/CSS/platform output paths after the
build.

## Report Diff Check

Generated reports are under ignored `dist/` paths, so they are not available as tracked git diffs. The regenerated
report summaries were checked instead:

- JSON platform report: 371 tokens per mode, 5 mode files.
- iOS report: 371 Swift token constants per mode, 26 color asset hints, 0 fail-hard violations.
- Android report: 193 light resources, 193 night resources, 371 Compose string constants, 178 skipped XML resources,
  0 fail-hard violations.

No new report failure category appeared from the one-token color change.
