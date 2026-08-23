# Token Pipeline Smoke

Validation run: 2026-05-01.

Figma propagation leg added: 2026-06-17.

Repeatable non-Figma propagation gate added: 2026-06-17.

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

## Figma Propagation Leg

Run the deterministic Figma leg with:

```bash
yarn nx run @epa-wg/cem-theme:smoke:figma-propagation
```

`yarn nx run @epa-wg/cem-theme:test:figma` also runs this smoke after the generated Figma file validation.

The smoke changes one canonical source token in memory:

```text
--cem-color-cyan-xl -> #e8ffff
```

It then verifies the generated Figma mode-file contract that a refresh must preserve:

- `cem/color/cyan/xl` keeps the same Figma variable path, type, and CSS metadata while its value changes in every mode.
- `cem/palette/comfort` changes under `light` and `contrast-light`, matching the sample frame/card background binding.
- `cem/palette/comfort/text` changes under `dark` and `contrast-dark`, matching the sample card text binding.
- The same sample fixture variable names remain bound; no manual rebinding is required.
- `native` fixture-facing `cem/palette/comfort` and `cem/palette/comfort/text` values stay browser-system-color derived.

Gap: this smoke does not call the Figma REST API or inspect the private `CEM UI Kit` file live. Credentialed live
validation remains under the Figma REST API sync policy in `packages/cem-theme/docs/token-figma.md`; until that exists
in CI, the release gate validates the generated import artifacts and the checked-in fixture/evidence offline.

## Repeatable Non-Figma Gate

Run the deterministic source-to-platform smoke with:

```bash
yarn nx run @epa-wg/cem-theme:smoke:token-propagation
```

The gate temporarily changes the canonical markdown row:

```text
--cem-color-blue-xl: #e0e8ff -> #dce8ff
```

It runs `yarn nx run @epa-wg/cem-theme:build:tokens`, rewrites platform artifacts with
`build-token-platforms.mjs`, validates them with `validate-platforms.mjs`, restores the source file, and repeats the
same generation path so the worktree returns to the original token value. Platform emission is run directly after the
Nx token build so local Nx cache cannot leave restored-source platform artifacts at the temporary smoke value.

Checked propagation surfaces:

- source markdown and built token XHTML
- generated per-spec and combined CSS
- canonical, resolved, TypeScript, and flat per-mode JSON token outputs
- iOS Swift constants and asset-catalog hints
- Android `values`, `values-night`, and Compose constants
- iOS and Android reports showing zero fail-hard violations

This portable gate does not pretend to compile Swift or Kotlin when the current host lacks Xcode or the Android
toolchain. Phase 8 owns real supported-host gates through
`@epa-wg/cem-theme:compile:ios-platform` and `@epa-wg/cem-theme:compile:android-platform`; CI runs both against clean
copies of the generated packages.
