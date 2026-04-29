# Todo

## Token export pipeline (`packages/cem-theme/docs/token-export.md`)

Design doc status: Proposal (Approach E — Hybrid). Requires sign-off on open decisions before Phase A starts.

---

### Pre-implementation: open decisions (§12)

- [ ] **D1** Pin Style Dictionary version compatible with repo Node runtime and DTCG value shapes (v4+ for DTCG; v5+ for DTCG 2025.10).
- [ ] **D2** Confirm Tokens Studio plan: optional pull-only workflow; direct Figma Variables import is the MVP path. No paid-tier dependency.
- [ ] **D3** Lock Figma sync direction as **read-only** (md → DTCG → Figma). Write-back deferred until governance is designed.
- [ ] **D4** Voice token export: emit metadata placeholder in v1 only; full TTS adapter support deferred to v2.
- [ ] **D5** Adapter and deprecated tier export: flag-gated (`--with-adapter`, `--with-deprecated`), matching CSS pipeline contract.
- [ ] **D6** Color resolution: browser/Playwright capture is primary; add `culori` only as an explicit fallback, validated against browser values.
- [ ] **D7** DTCG mode encoding: `$extensions.cem.modes` in canonical JSON; Figma gets one generated file per mode (`cem-light.tokens.json`, `cem-dark.tokens.json`).
- [ ] **D8** Output stability: commit to schema stability after Phase D ships one full release cycle.
- [ ] **D9** px→dp/pt mapping: per token category — spacing/shape → dp; typography sizes → sp; iOS uses points uniformly. Document per category.
- [ ] **D10** `color-mix()` policy: resolve per theme at export time; preserve alias-form DTCG for DTCG-aware consumers (Figma, Tokens Studio, Style Dictionary).
- [ ] **D11** Tokens Studio: optional bridge only, not source of truth. Same DTCG JSON serves both paths; CEM commits to neither tool.
- [ ] **D12** Figma REST API sync: defer to post-v1. Manual file import is sufficient MVP.
- [ ] **D13** Figma collections: one CEM collection with groups by dimension; split only if per-collection limits or navigation friction arise.
- [ ] **D14** iOS dynamic type: do not auto-scale in v1. Product apps decide whether to apply dynamic type.
- [ ] **D15** Android color modes: resource qualifiers (`values/`, `values-night/`) for v1; Compose color schemes optional add-on.
- [ ] **D16** Package export map: expose `dist/lib/tokens/cem.tokens.json` and TS metadata via `package.json` `exports` field (e.g., `@epa-wg/cem-theme/tokens`).

---

### MVP scope (Phases A + B + C, §15)

#### Phase A — Token-data extraction

- [ ] Extend `packages/cem-theme/scripts/manifest-utils.mjs`: add `tokensFromTableWithValues()` — same XPath as `tokensFromTable()` but also captures value cells, description, mode column, and source-table id.
- [ ] Add `packages/cem-theme/scripts/derive-tokens.mjs`: `derive*Tokens()` helpers returning `{ name, valueRaw, tier, description, mode, category, sourceTable }` per spec (mirrors `derive*Manifest()` shape).
- [ ] Create `packages/cem-theme/scripts/export-tokens.mjs` (orchestration). Stage 1: iterate `dist/lib/tokens/*.xhtml`, call derivation helpers, build in-memory intermediate model. Emit `dist/lib/tokens/cem.tokens.intermediate.json` behind `--debug` flag only (not a public contract).
- [ ] Validate Phase A: `export-tokens.mjs` JSON contains every manifest-eligible token name; parity check against existing CSS output via manifest validator.

#### Phase B — Theme-aware value resolution

- [ ] In `export-tokens.mjs` Stage 2: launch headless Chromium via Playwright; serve a minimal HTTP fixture that links `dist/lib/css/*.css`.
- [ ] For each theme class (`light`, `dark`; `contrast-light`, `contrast-dark` once mappings are proven): set `.cem-theme-*` on `<html>`, read `getComputedStyle(documentElement).getPropertyValue(token.name)`, capture resolved RGBA/numeric/dimension value.
- [ ] Repeat for spacing/coupling/shape parallel groups by setting `data-cem-spacing`, `data-cem-coupling`, `data-cem-shape` attributes.
- [ ] Store both `valueRaw` and `valueByTheme` in memory. Emit `dist/lib/tokens/cem.tokens.resolved.json` behind `--debug` flag only.
- [ ] Validate Phase B: 5-token spot check per theme against manual `debug-cem.mjs` `getComputedStyle()` capture.

#### Phase C — DTCG JSON emission

- [ ] In `export-tokens.mjs` Stage 3: transform extracted + resolved model to DTCG-compatible format.
  - Tier-aware filtering: required + recommended by default; `--with-optional`, `--with-adapter`, `--with-deprecated` flags.
  - Per-theme mode values in `$extensions.cem.modes`; separate Figma mode files.
  - Spacing/coupling/shape as parallel token groups (9 groups, not 135 tuples).
  - Filter web-only categories (forced-colors fallbacks, ring recipes, unresolved system colors).
  - Preserve aliases as DTCG references (`"$value": "{cem.color.blue.l}"`); resolve only for non-reference-aware consumers.
  - Add `$extensions.cem` metadata to every token: `cssName`, `spec`, `sourceTable`, `tier`, `category`, `rawValue`, `portability`, `modes`.
- [ ] Emit `dist/lib/tokens/cem.tokens.json` — canonical DTCG JSON, visual tokens (cross-platform bucket). Include generated provenance in top-level `$extensions.cem.generated`.
- [ ] Emit `dist/lib/tokens/cem.voice.tokens.json` — voice/audio bucket, metadata-only in v1.
- [ ] Emit `dist/lib/tokens/cem.tokens.report.md` — human-readable report: every token, portability classification, what was skipped per target and why. **First-class output.**
- [ ] Emit `dist/lib/tokens/cem.tokens.report.json` — machine-readable equivalent for CI assertions.
- [ ] Emit `dist/lib/tokens/figma/cem-light.tokens.json` and `cem-dark.tokens.json` — Figma mode files with Figma-specific validation (name normalization, `$type` consistency, no partial modes).
- [ ] Emit `dist/lib/tokens/figma/cem-figma-report.md` — skipped tokens + Figma-specific notes.
- [ ] Implement fail-hard validation (exit non-zero) for: missing required/recommended tokens, DTCG path collisions, Figma slash-name collisions, `$type` mismatch across modes, DTCG schema validation failure, voice token in visual export, missing provenance header, mode-completeness violations.
- [ ] Implement warn-and-report for: skipped optional tokens, unresolvable `css-expression`, system color `platform-note`, excluded deprecated tokens, excluded adapter-tier tokens.
- [ ] Add `build:tokens` Nx target in `packages/cem-theme/project.json`: depends on `build:css`; executor `nx:run-commands`; outputs `cem.tokens.json`, `cem.voice.tokens.json`, `cem.tokens.report.*`, `figma/`.
- [ ] Validate Phase C: DTCG JSON passes `@design-tokens/parser` or `tokens-json-validator`; Figma Variables direct import succeeds without error; report lists every skipped token with reason.

#### MVP: TypeScript metadata (§15 item 4)

- [ ] Emit `dist/lib/js/cem-tokens.ts` with `CemTokenName` union type and `CemTokenMeta` interface; must pass `tsc --noEmit`.
- [ ] Wire `package.json` `exports` field to expose `dist/lib/tokens/cem.tokens.json` and TS metadata under a stable import path (Decision D16).

---

### Post-MVP: Phase D — Style Dictionary fan-out

- [ ] Add `packages/cem-theme/style-dictionary.config.mjs`.
- [ ] Create `packages/cem-theme/scripts/build-token-platforms.mjs` (Style Dictionary driver).
- [ ] Implement custom Style Dictionary transforms: `cem/size/rem-to-pt`, `cem/size/rem-to-dp`, `cem/size/rem-to-sp`, `cem/category/web-only-filter`, `cem/mode/expand-themes`.
- [ ] Add `build:token-platforms` Nx target: depends on `build:tokens`; outputs `dist/lib/token-platforms/`.
- [ ] Emit iOS: `CEMTokens.swift`, `CEMTokens.xcassets-hints.json`, `ios-report.md`.
- [ ] Emit Android: `values/cem-tokens.xml`, `values-night/cem-tokens.xml`, `values-night-hcc/cem-tokens.xml`, `compose/CEMTokens.kt`, `android-report.md`.
- [ ] Emit JS/TS: `cem-tokens.ts` (typed + metadata), `js-report.md`.
- [ ] Emit JSON: `cem-tokens.json` (resolved-per-theme, flat), `json-report.md`.
- [ ] Emit SCSS: `cem-tokens.scss`, `scss-report.md`.
- [ ] Validate Phase D: Swift compiles (Xcode 15+); Kotlin compiles (Gradle 8+); `tsc --noEmit` passes; `dart-sass` compiles SCSS; per-platform reports show zero fail-hard violations.

### Post-MVP: Phase E — Figma integration

- [ ] Document Tokens Studio pull-only workflow in `examples/figma/README.md` (pull only; no write-back).
- [ ] Document Figma Variables direct DTCG import workflow with screenshots and step-by-step instructions.
- [ ] Validate: Figma Variables direct DTCG import succeeds; light/dark modes align; optional Tokens Studio pull loads same JSON without write-back.

### Post-MVP: Phase F — Adapter examples

- [ ] `examples/ios/CEMTokensExample/` — minimal SwiftUI app: button + card using only CEM tokens.
- [ ] `examples/android/cem-tokens-example/` — minimal Compose app, parallel to iOS.
- [ ] `examples/web/import-tokens.ts` — TypeScript consumption sample with type-safe imports.
- [ ] `examples/figma/CEMTokens.fig` (or screenshots) — sample Figma file showing token application.
- [ ] Validate Phase F: iOS and Android apps render button + card with CEM tokens; visual parity with web reference (manual screenshot diff).

### Post-MVP: Phase G — Documentation cross-links

- [ ] Update `packages/cem-theme/docs/docs-generation.md` to reference the export pipeline.
- [ ] Update `packages/cem-theme/src/lib/tokens/index.md`: add "Platform consumption" section.
- [ ] Add "Token export contract" section to `CLAUDE.md` parallel to "Token manifest contract".
- [ ] Cross-reference `cem-m3-parity.md` to the M3 alias adapter set.
- [ ] Validate Phase G: `yarn nx affected -t lint test build typecheck` green; `verify:phase13` green.

---

### End-to-end smoke (after all phases land)

- [ ] Change a value in `cem-colors.md`; run `yarn build`; verify new value propagates through: `dist/lib/css/cem-colors.css`, `cem.tokens.json`, `CEMTokens.swift`, `cem-tokens.xml`, `cem-tokens.ts`.
- [ ] Import/refresh Figma test collection; verify new value appears in Variables panel.
- [ ] Diff per-target report files: only the changed token should appear; no spurious portability or mode changes.

---

### New script / file checklist (§9)

| File                                                                                | Status             |
|-------------------------------------------------------------------------------------|--------------------|
| `packages/cem-theme/scripts/manifest-utils.mjs` (extended)                          | pending            |
| `packages/cem-theme/scripts/derive-tokens.mjs` (new)                                | pending            |
| `packages/cem-theme/scripts/export-tokens.mjs` (new)                                | pending            |
| `packages/cem-theme/scripts/build-token-platforms.mjs` (new, Phase D)               | pending            |
| `packages/cem-theme/scripts/validate-platforms.mjs` (new, Phase D)                  | pending            |
| `packages/cem-theme/style-dictionary.config.mjs` (new, Phase D)                     | pending            |
| `packages/cem-theme/project.json` (`build:tokens`, `build:token-platforms` targets) | pending            |
| `dist/lib/tokens/cem.tokens.json`                                                   | generated          |
| `dist/lib/tokens/cem.voice.tokens.json`                                             | generated          |
| `dist/lib/tokens/cem.tokens.report.md`                                              | generated          |
| `dist/lib/tokens/cem.tokens.report.json`                                            | generated          |
| `dist/lib/tokens/figma/cem-light.tokens.json`                                       | generated          |
| `dist/lib/tokens/figma/cem-dark.tokens.json`                                        | generated          |
| `dist/lib/tokens/figma/cem-figma-report.md`                                         | generated          |
| `dist/lib/token-platforms/ios/`, `android/`, `js/`, `json/`, `scss/`                | generated, Phase D |
| `examples/ios/`, `examples/android/`, `examples/web/`, `examples/figma/`            | pending, Phase F   |
