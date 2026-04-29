# Multi-Platform Token Export — Design Document

**Status:** Proposal (not yet implemented). Recommends Approach E (Hybrid).
**Audience:** CEM maintainers, design-system reviewers, downstream platform teams.
**Decision required:** see §12 (Open decisions).

---

## 1. Context & goals

CEM ships **407 design tokens** across 13 dimensions today, generated from canonical markdown specs in
`packages/cem-theme/src/lib/tokens/*.md`. The CSS pipeline is mature and validated. The next strategic step is
exposing the same tokens to non-CSS consumers:

- **Figma designers** — canvas designs must use canonical CEM token values, not hand-typed hex codes or guesses.
  Every color swatch, type scale, spacing step, and corner radius in Figma should be a CEM token.
- **Application developers** on iOS, Android, Compose, SwiftUI, and other non-web runtimes — native code should
  consume the same vocabulary as the web. Visual parity across platforms requires a single source of truth.

### Constraints (non-negotiable)

1. **Markdown specs remain the single source of truth.** Every other format is **derived**, mirroring how
   `dist/lib/css/*.css` is derived today. No format may author tokens that don't exist in markdown.
2. **The existing CSS pipeline must keep working.** The CSS generators capture CEM-specific features
   (`color-mix()` recipes, `.cem-theme-*` overrides, `@media (forced-colors: active)` fallbacks, `@container`
   queries, `data-cem-{coupling,shape,spacing}` attribute selectors) that no off-the-shelf transform reproduces.
3. **Tier discipline propagates.** Required and recommended tokens emit by default; optional, adapter-only, and
   deprecated tokens emit behind explicit flags — same contract as the CSS pipeline.

### Greenfield state

A repo-wide search for `style-dictionary`, `DTCG`, `tokens.json`, `figma`, `tokens-studio`, iOS/Android/Swift/Compose
patterns returned **zero artifacts**. We are choosing the architecture for the first time, with no legacy to migrate.

---

## 2. Architectural principles

```
┌─────────────────┐
│  *.md (truth)   │
└────────┬────────┘
         │ build:docs (compile-markdown.mjs)
         ▼
┌─────────────────┐         ┌──────────────────────────┐
│  *.xhtml        │────────▶│  CSS pipeline (existing)  │──▶ dist/lib/css/*.css
│  (h6 + table)   │         └──────────────────────────┘
└────────┬────────┘
         │ extract-tokens.mjs (NEW — Phase A)
         ▼
┌─────────────────┐
│  intermediate   │
│     JSON        │
└────────┬────────┘
         │ resolve-tokens.mjs (NEW — Phase B, Playwright)
         ▼
┌─────────────────┐         ┌──────────────────────────┐
│  resolved JSON  │────────▶│  emit-dtcg.mjs (Phase C)  │──▶ dist/lib/tokens-json/*.dtcg.json
│  (per-theme)    │         └──────────┬───────────────┘
└─────────────────┘                    │
                                       ▼
                          ┌─────────────────────────────┐
                          │  Style Dictionary (Phase D)  │──▶ dist/lib/platforms/{ios,android,js,scss,json}/
                          └─────────────────────────────┘
                                       │
                                       ▼
                          ┌─────────────────────────────┐
                          │  Tokens Studio / Figma       │──▶ Figma Variables (read-only)
                          │  (Phase E)                   │
                          └─────────────────────────────┘
```

Principles:

- **md → XHTML → DTCG JSON → consumers** is the new spine. CSS continues from XHTML directly (existing path
  unchanged).
- **W3C DTCG JSON** is the canonical machine-readable intermediate. It is a [Design Tokens Community Group](https://design-tokens.github.io/community-group/format/)
  draft format adopted by Figma Variables import, Style Dictionary v4, and Tokens Studio.
- **Tier-aware emission** matches the CSS pipeline; the same `tier` column in source tables drives all outputs.
- **Token classification.** Every token belongs to one of three buckets: cross-platform / web-only / voice-audio
  (see §5).

---

## 3. Approach comparison

| Approach | Source format | Cross-platform fan-out | Figma path | Pros | Cons |
|---|---|---|---|---|---|
| **A. Style Dictionary on DTCG JSON** | DTCG JSON (derived from XHTML) | Style Dictionary v4 native transforms | Tokens Studio reads the same JSON | Mature ecosystem; standards-aligned; one source feeds many targets | Adds Style Dictionary dep; needs custom transforms for `color-mix()` recipes; multi-dim modes need design |
| **B. Per-platform generators** | XHTML directly (mirror CSS pipeline) | Hand-rolled `cem-ios.html`, `cem-android.html`, `cem-figma.html` generators | Custom Figma plugin or static JSON | Stays in established pattern; full control over output shape | Reinvents Style Dictionary's transform library; massive boilerplate per format; per-platform color/font conversion is error-prone |
| **C. DTCG JSON only** | DTCG JSON | Consumer-owned (apps transform themselves) | DTCG-aware tools consume directly | Lightest weight; pure-standards; future-proof | App teams want native code, not "transform yourself"; high adoption friction |
| **D. Tokens Studio + GitHub two-way sync** | DTCG JSON in repo | Same as A | Tokens Studio syncs both ways via Git provider | Designers can author in Figma, push to repo | **Two-way sync risks designers overwriting md source-of-truth**; needs strict governance and review gates |
| **E. Hybrid (recommended)** | XHTML → DTCG JSON → {Style Dictionary, Tokens Studio} | Style Dictionary fans out to iOS/Android/JS | Tokens Studio reads DTCG JSON read-only; existing CSS unchanged for web | Best of A + C: web stays as-is, native gets Style Dictionary, Figma reads-only via standards | Most components to integrate; needs build-time color resolution |

### Why Approach E

- The existing CSS path **already works** and captures features Style Dictionary's CSS transform doesn't reproduce
  (`color-mix()`, theme-mode classes, forced-colors fallbacks, container queries). Replacing it would lose
  capability.
- **Approach B duplicates Style Dictionary.** Writing per-platform generators by hand means re-implementing color
  conversion, unit scaling (rem→pt/dp), and platform-idiomatic emission for every target — work the SD ecosystem
  has already done and battle-tested.
- **Approach C punts adoption.** App teams asking for tokens want consumable artifacts (Swift constants, Android
  XML, TS modules), not raw JSON. Friction kills adoption.
- **Approach D's two-way sync is a governance landmine.** If a designer in Figma renames a token or changes a
  value, Tokens Studio writes it back to Git; the canonical md spec is bypassed. Until CEM has a governance
  process for designer-authored changes, sync stays one-way (read-only from md).
- **Approach E preserves all three sources of value:** the CSS pipeline keeps producing CSS the way it does today,
  the DTCG JSON layer becomes the canonical machine-readable export, and Style Dictionary handles platform
  fan-out without us reinventing it.

---

## 4. Reference: M3 design tokens

[Material Design 3 design tokens guide](https://m3.material.io/foundations/design-tokens/how-to-use-tokens) defines a
three-tier model:

- **Reference tokens** — primitives (e.g., `md.ref.palette.primary40`)
- **System tokens** — semantic roles (e.g., `md.sys.color.primary`)
- **Component tokens** — component-specific (e.g., `md.comp.button.container.color`)

CEM's existing layering already maps cleanly:

| M3 tier | CEM equivalent | Examples |
|---|---|---|
| Reference | Branded colors (D0), thickness scale, size scale, dimension scale, breakpoint widths | `--cem-color-blue-xl`, `--cem-typography-thickness-bold`, `--cem-dim-large` |
| System | Emotional palette (D0), action intent tokens, semantic shape/stroke/layering endpoints | `--cem-palette-trust`, `--cem-action-primary-hover-background`, `--cem-bend-control` |
| Component | Controls geometry (D2c), action border-radius, role typography | `--cem-control-height`, `--cem-action-border-radius`, `--cem-typography-button-font-size` |

`packages/cem-theme/src/lib/tokens/cem-m3-parity.md` already documents the role-by-role mapping. The DTCG export
will (optionally, behind a flag) emit an M3-shaped alias group so consumers expecting M3 names get a drop-in
adapter.

---

## 5. Token classification

Every CEM token belongs to one of three buckets. This drives **what gets exported where**:

| Bucket | Examples | DTCG JSON | iOS/Android | SCSS/JS | Figma | Notes |
|---|---|---|---|---|---|---|
| **Cross-platform visual** | Branded colors (resolved), palette (resolved), spacing, typography sizes/weights, timing, shape radii, controls geometry | ✓ | ✓ (resolved) | ✓ | ✓ | Recipes resolved per theme at build time |
| **Web-only** | `forced-colors` fallbacks, `@container` rules, system colors (`Highlight`, `CanvasText`, `SelectedItem`), ring composite recipes, `light-dark()` wrappers | — | — | — | — | CSS only (existing pipeline) |
| **Voice/audio data** | `--cem-voice-*-{speech-rate,speech-pitch,speech-volume,ssml-emphasis,ink-thickness,icon-stroke-multiplier}` | ✓ (separate file) | ✓ (TTS adapters) | ✓ | — | Separate `cem-voice.dtcg.json`; not visual |

### Concrete translation gotchas

| Token | Value | Translation strategy |
|---|---|---|
| `--cem-color-visitedtext-30-black` | `color-mix(in srgb, VisitedText 30%, black)` | System color `VisitedText` resolves only on web/native theme. For other platforms: fall back to a hex equivalent or skip. |
| `--cem-bend-round` | `calc(var(--cem-shape-height, var(--cem-control-height)) / 2)` | Resolve to numeric px/rem at build time; mobile cannot evaluate nested `calc()` at token load. |
| `--cem-palette-trust` | `light-dark(var(--cem-color-blue-l), var(--cem-color-blue-d))` | Map to two values per theme: light= and dark=. iOS asset catalogs and Android `values-night/` consume them directly. |
| `--cem-typography-data-font-variant-numeric` | `tabular-nums lining-nums` | OpenType feature settings; Android Roboto may not support `lining-nums`. Document as recommended, not required, on Android. |
| `--cem-voice-loud-speech-rate` | `0.94` | Not a visual property; routes only to TTS adapter (AVSpeechSynthesizer / Android TextToSpeech). Excluded from visual exports. |

---

## 6. Mode strategy

CEM has multiple orthogonal mode dimensions:

| Mode dimension | Values | Cardinality |
|---|---|---|
| Theme | `light`, `dark`, `contrast-light`, `contrast-dark`, `native` | 5 |
| Spacing density | `dense`, `normal`, `sparse` | 3 |
| Coupling | `forgiving`, `balanced`, `compact` | 3 |
| Shape style | `sharp`, `smooth`, `round` | 3 |

Cartesian product = 135 tuples — impractical to express as a single DTCG mode axis.

**Strategy:**

1. **Theme is DTCG-native.** Every cross-platform token gets a `$value` per theme via DTCG modes (light, dark,
   contrast-light, contrast-dark, native). This maps directly to:
    - Figma Variables modes (light/dark already first-class)
    - iOS asset catalogs (`Any Appearance`, `Dark`, `Light`, `High Contrast` variants)
    - Android resource qualifiers (`values/`, `values-night/`, `values-night-hcc/`)
2. **Spacing/coupling/shape are parallel groups.** Each appears as its own token group with explicit prefix
   (e.g., `spacing.dense.layout-stack-gap`, `spacing.normal.layout-stack-gap`, `spacing.sparse.layout-stack-gap`).
   Consumers select at app level (toggle a CSS attribute on web; pick a Compose theme on Android; etc.).
3. **Forced-colors and reduced-motion are platform-native conventions.** Not exported as tokens. iOS adapters
   read `UITraitCollection.accessibilityContrast` / `UIAccessibility.isReduceMotionEnabled`; Android reads
   `Configuration.uiMode` / `Settings.Global.TRANSITION_ANIMATION_SCALE`. Adapter docs (Phase F) describe the
   mapping per platform.

Counts: 9 parallel groups (3 + 3 + 3) instead of 135 tuples; the theme axis is the only true DTCG mode dimension.

---

## 7. Recipe vs resolved-value split

CEM's action state tokens use recipes such as
`color-mix(in srgb, var(--cem-palette-trust) 70%, var(--cem-palette-trust-x))` for hover state. Two superpowers:

1. **Theme switching is automatic.** If `--cem-palette-trust` changes per theme, hover follows.
2. **Adapter customization.** A product adapter can override the base palette and the hover state recipe follows.

Non-web platforms can't evaluate `color-mix()` at runtime. Two paths:

- **Resolve at build time → lose recipe semantics.** Each theme produces its own concrete RGBA. Theme switching
  becomes per-theme asset bundling (iOS asset catalogs, Android `values-night/`). Adapter customization at the
  recipe level is gone, but adapters can still override at the primitive (palette) level and rebuild.
- **Export the recipe in a portable form.** DTCG composite tokens or string-encoded math, requiring custom
  consumer logic. Native platforms get nothing useful.

**Decision:** Resolve at build time for non-web. Web keeps recipes (existing behavior, no change). For DTCG JSON
and downstream platforms, every recipe collapses to per-theme RGBA.

**Implementation:** `tools/scripts/resolve-tokens.mjs` (Phase B) uses Playwright to load each
`dist/lib/tokens/*.xhtml` page in headless Chromium, applies each `.cem-theme-*` class to `<html>`, then reads
`getComputedStyle(:root).getPropertyValue('--cem-...')` for every token. The browser performs the recipe
resolution natively — no need to re-implement `color-mix()` ourselves.

For pure JSON post-processing (when Playwright isn't viable), [`culori`](https://culorijs.org/) implements
`color-mix(in srgb, ...)` per [CSS Color Module Level 5 §2.1](https://www.w3.org/TR/css-color-5/#color-mix).

---

## 8. Implementation phases

### Phase A — Token-data extraction (foundational)

Add `derive*Tokens()` companions to `derive*Manifest()` in
`packages/cem-theme/scripts/manifest-utils.mjs`. Where the existing manifest functions return
`{ name, tier, categoryId }`, the new ones return `{ name, valueRaw, tier, description, mode, category, sourceTable }`.

Add `tools/scripts/extract-tokens.mjs` that iterates compiled `dist/lib/tokens/*.xhtml`, calls each derivation,
and emits `dist/lib/tokens-json/cem-tokens.intermediate.json`.

**Output shape (excerpt):**

```json
{
  "version": "0.0.7",
  "generatedAt": "2026-04-28T10:00:00Z",
  "specs": {
    "cem-colors": {
      "branded": [
        { "name": "--cem-color-blue-xl", "valueRaw": "#ecf0ff", "tier": "required",
          "category": "branded", "sourceTable": "cem-color-hue-variant" }
      ],
      "palette": [...],
      "action": [...],
      "zebra": [...]
    },
    "cem-shape": {...},
    ...
  }
}
```

### Phase B — Theme-aware value resolution

Add `tools/scripts/resolve-tokens.mjs` using Playwright (already in tree per `tools/scripts/capture-xpath-text.mjs`).

For each spec's compiled XHTML:

1. Launch headless Chromium, navigate to `file://...cem-<name>.xhtml`.
2. For each theme class in `[light, dark, contrast-light, contrast-dark, native]`:
    - Set `document.documentElement.className = 'cem-theme-' + theme`.
    - For each token in the intermediate JSON, read
      `getComputedStyle(document.documentElement).getPropertyValue(token.name)`.
    - Capture the resolved RGBA / numeric.
3. Write `dist/lib/tokens-json/cem-tokens.resolved.json` containing both `valueRaw` and `valueByTheme`.

**Output shape (excerpt):**

```json
{
  "--cem-palette-trust": {
    "valueRaw": "light-dark(var(--cem-color-blue-l), var(--cem-color-blue-d))",
    "valueByTheme": {
      "light": "rgb(33, 87, 178)",
      "dark":  "rgb(122, 158, 220)",
      "contrast-light": "rgb(0, 0, 102)",
      "contrast-dark":  "rgb(180, 200, 240)",
      "native": "Highlight"
    }
  }
}
```

Spacing/coupling/shape mode resolution happens analogously by setting `data-cem-spacing`, `data-cem-coupling`,
`data-cem-shape` attributes on `<html>` and re-reading the affected tokens. Each becomes a parallel token group in
the DTCG output.

### Phase C — DTCG JSON emission

Add `tools/scripts/emit-dtcg.mjs` transforming intermediate + resolved JSON to the
[W3C DTCG format](https://design-tokens.github.io/community-group/format/).

- Tier-aware filtering (default: required + recommended; `--with-optional`, `--with-adapter`,
  `--with-deprecated` flags add tiers).
- Theme as DTCG-native modes via `$extensions.modes` or DTCG mode collections.
- Spacing/coupling/shape as parallel groups.
- Web-only categories filtered out (forced-colors fallbacks, ring recipes, system color references that don't
  resolve outside web).

**Outputs:**

- `dist/lib/tokens-json/cem-tokens.dtcg.json` — visual tokens (cross-platform bucket).
- `dist/lib/tokens-json/cem-voice.dtcg.json` — voice/audio bucket, separate file.

### Phase D — Style Dictionary fan-out

Add `packages/cem-theme/style-dictionary.config.mjs` and an Nx target `build:platforms` parallel to `build:css`.

Custom transforms required:

- `cem/size/rem-to-pt` (iOS): rem → pt at 16pt base.
- `cem/size/rem-to-dp` (Android): rem → dp at 16dp base.
- `cem/size/rem-to-sp` (Android, typography only): rem → sp.
- `cem/category/web-only-filter`: drop web-only categories before emission.
- `cem/mode/expand-themes`: expand DTCG modes to per-platform conventions (asset catalog hints for iOS;
  `values-night/` for Android).

**Outputs:**

```
dist/lib/platforms/
├── ios/
│   ├── CEMTokens.swift                    (struct CEMTokens { static let colorPaletteTrust = UIColor(...) })
│   └── CEMTokens.xcassets-hints.json      (asset-catalog scaffolding)
├── android/
│   ├── values/cem-tokens.xml              (light theme)
│   ├── values-night/cem-tokens.xml        (dark theme)
│   ├── values-night-hcc/cem-tokens.xml    (contrast-dark)
│   └── compose/CEMTokens.kt               (val cemColorPaletteTrust = Color(...))
├── js/
│   └── cem-tokens.ts                      (typed exports)
├── json/
│   └── cem-tokens.json                    (resolved-per-theme, flat)
└── scss/
    └── cem-tokens.scss                    ($cem-color-palette-trust: ...;)
```

### Phase E — Figma integration

Two paths, both **read-only** (write-back deferred until governance is designed):

1. **Tokens Studio** (Figma plugin, free tier).
    - Configure Tokens Studio with the GitHub provider pointing at `dist/lib/tokens-json/cem-tokens.dtcg.json`.
    - Designers fetch in the plugin; tokens appear as Figma Variables organized by dimension.
    - Updates flow when designers click "Pull from GitHub".
2. **Figma Variables direct DTCG import.**
    - Figma added DTCG JSON import in 2024.
    - Designers (or design-ops) drag-drop the DTCG file into the Variables panel.
    - Lower fidelity than Tokens Studio (no auto-refresh) but no plugin dependency.

`examples/figma/README.md` will document both paths with screenshots and step-by-step instructions.

### Phase F — Adapter examples

Reference applications proving end-to-end consumption:

- `examples/ios/CEMTokensExample/` — minimal SwiftUI app with a button + card using only CEM tokens.
- `examples/android/cem-tokens-example/` — minimal Compose app, parallel.
- `examples/web/import-tokens.ts` — TypeScript consumption sample with type-safe imports.
- `examples/figma/CEMTokens.fig` — sample Figma file (or screenshots) showing token application.

### Phase G — Documentation cross-links

- Update `packages/cem-theme/docs/docs-generation.md` to reference the export pipeline.
- Update `packages/cem-theme/src/lib/tokens/index.md` with a "Platform consumption" section.
- Add a "Token export contract" section to `CLAUDE.md` parallel to the existing "Token manifest contract" section.
- Cross-reference from `cem-m3-parity.md` to the M3 alias adapter set.

---

## 9. Files to add

```
packages/cem-theme/
├── docs/
│   └── token-export.md                          (this file)
├── scripts/
│   ├── manifest-utils.mjs                       (extended: tokensFromTableWithValues)
│   ├── derive-tokens.mjs                        (new: full derive*Tokens() helpers)
│   └── validate-platforms.mjs                   (new: validate platform output coverage)
├── style-dictionary.config.mjs                  (new)
└── dist/lib/                                    (generated)
    ├── tokens-json/
    │   ├── cem-tokens.intermediate.json
    │   ├── cem-tokens.resolved.json
    │   ├── cem-tokens.dtcg.json
    │   └── cem-voice.dtcg.json
    └── platforms/
        ├── ios/
        ├── android/
        ├── js/
        ├── json/
        └── scss/

tools/scripts/
├── extract-tokens.mjs                           (new: XHTML → intermediate JSON)
├── resolve-tokens.mjs                           (new: Playwright theme resolution)
└── emit-dtcg.mjs                                (new: CEM JSON → DTCG JSON)

examples/
├── ios/CEMTokensExample/
├── android/cem-tokens-example/
├── figma/README.md
└── web/import-tokens.ts
```

---

## 10. Reused infrastructure

The pipeline extends — does not replace — what already exists:

| Existing utility | Path | New use |
|---|---|---|
| `extractTable()`, `tokensFromTable()` | `packages/cem-theme/scripts/manifest-utils.mjs` | Add `tokensFromTableWithValues()` capturing value cells, not just names + tier |
| Validator pattern | `packages/cem-theme/scripts/validate-manifest.mjs` | Mirror as `validate-platforms.mjs` for non-CSS outputs |
| Playwright XPath capture | `tools/scripts/capture-xpath-text.mjs` | Model for `resolve-tokens.mjs` (computed-value capture per theme class) |
| Markdown → XHTML compilation | `tools/scripts/compile-markdown.mjs` | Unchanged; export pipeline reads its output |
| Nx target wiring | `packages/cem-theme/project.json` | Add `build:tokens-json` and `build:platforms` targets parallel to `build:css` |

---

## 11. Verification plan

### Per-phase

| Phase | Verification |
|---|---|
| A | `extract-tokens.mjs` produces JSON with all 407 token names; manifest validator confirms parity with CSS output. |
| B | 5-token spot check per theme matches manual `getComputedStyle()` capture from `tools/scripts/debug-cem.mjs`. |
| C | DTCG JSON validates against W3C DTCG schema (`@design-tokens/parser` or `tokens-json-validator`). Figma Variables imports without error. |
| D | Generated Swift compiles with Xcode 15+; Kotlin with Gradle 8+; TypeScript passes `tsc --noEmit`; SCSS compiles with `dart-sass`. |
| E | Tokens Studio loads GitHub-hosted DTCG JSON; designer sees CEM token sets categorized by dimension. Figma Variables direct DTCG import succeeds. |
| F | Sample iOS and Android apps render a button + card using only CEM tokens; visual parity with web reference (manual screenshot diff). |
| G | `yarn nx affected -t lint test build typecheck` green; `verify:phase13` green. |

### End-to-end smoke (after Phases A–G land)

1. Change a value in `cem-colors.md` (e.g., `--cem-color-blue-xl: #ecf0ff` → `#e0e8ff`).
2. Run `yarn build`.
3. Verify the new value flows through:
    - `dist/lib/css/cem-colors.css` (existing CSS pipeline)
    - `dist/lib/tokens-json/cem-tokens.dtcg.json`
    - `dist/lib/platforms/ios/CEMTokens.swift`
    - `dist/lib/platforms/android/values/cem-tokens.xml`
    - `dist/lib/platforms/js/cem-tokens.ts`
4. Refresh Tokens Studio in Figma; verify the new value appears in the Variables panel.

---

## 12. Open decisions (require sign-off before Phase A)

| # | Decision | Recommendation | Rationale |
|---|---|---|---|
| 1 | Style Dictionary version | **v4** (DTCG-native) | v3 uses legacy CTI taxonomy; v4 reads DTCG JSON directly and is the future. |
| 2 | Tokens Studio plan | **Free** (read-only GitHub sync) | Read-only flow doesn't need pro features; v1 doesn't need advanced governance. |
| 3 | Figma sync direction | **Read-only** (md → DTCG → Figma) | Two-way sync risks designer-authored drift; governance for designer changes not yet designed. |
| 4 | Voice token export scope | **Defer to v2** | No consumer apps with TTS adapters yet; emit metadata placeholder only in v1. Saves complexity. |
| 5 | Adapter and deprecated tier export | **Flag-gated** (`--with-adapter`, `--with-deprecated`) | Matches the existing CSS pipeline contract. |
| 6 | Color-resolution math library | **`culori`** (for non-Playwright paths only) | Battle-tested; correct `color-mix(in srgb)` semantics. Playwright handles the primary resolution path. |
| 7 | DTCG mode encoding | **DTCG draft `$extensions.modes`** | The W3C draft spec doesn't yet finalize multi-mode encoding; we use the `$extensions` namespace for Figma/Tokens Studio compatibility today. |
| 8 | Output stability commitment | **Stabilize after Phase D ships once** | Token names in CSS are already stable; DTCG/platform names need one full release cycle to stabilize before promising backwards compat. |

---

## 13. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Style Dictionary's `color-mix()` support is incomplete | Cross-platform colors may emit as raw `color-mix(...)` strings that don't compile | Resolve all recipes at build time (Phase B); Style Dictionary only ever sees concrete RGBA |
| Figma Variables and Tokens Studio interpret DTCG modes differently | Designers see different mode behavior than expected | Ship Tokens Studio path first (more permissive); validate Figma Variables as Phase E.2 |
| Multi-mode dimensionality explosion | DTCG output becomes huge and hard to navigate | Theme as DTCG-native modes; spacing/coupling/shape as parallel groups (9 vs 135) |
| Voice tokens leaking into visual exports | Designers see speech-rate as a "color" or "size" in Figma | Hard-filter via category prefix; separate `cem-voice.dtcg.json`; Style Dictionary's `cem/category/web-only-filter` excludes voice from visual platform outputs |
| Native platforms can't represent all theme modes | iOS and Android may not have a clean "contrast-light" or "native" equivalent | Map theme tuples to platform conventions: light → `Any Appearance`, dark → `Dark`, contrast-* → high-contrast variants; native theme is web-only and excluded |
| DTCG draft format changes before W3C ratification | Output JSON shape may need migration | Pin DTCG schema version in `cem-tokens.dtcg.json` `$schema`; add migration script in Phase A2 if/when the spec rev'd |

---

## 14. Out of scope (explicit non-goals for v1)

- **Two-way Figma → md sync.** Governance for designer-authored changes is not designed yet. Tokens Studio's
  GitHub write-back capability stays disabled.
- **CI auto-regeneration on token change.** Build-on-merge hooks are a future enhancement; v1 ships with manual
  `yarn build`.
- **Migration of existing component libraries** (e.g., `@epa-wg/cem-components`) to consume new exports. Each
  adapter team owns the migration on their own timeline.
- **Locale-aware token variants.** CEM voice tokens are language-agnostic; per-locale TTS configuration is a
  product-level concern, not a token-level one.
- **Custom Figma plugin for CEM.** Tokens Studio plus Figma's native DTCG import cover the Figma path. A
  CEM-specific plugin is an option only if those two prove insufficient — not in v1.
- **Dynamic theme synthesis.** Themes are statically resolved at build time; runtime theme generation
  (e.g., from a brand color picker) is a separate feature.

---

## References

- [W3C Design Tokens Community Group format](https://design-tokens.github.io/community-group/format/)
- [Material Design 3 — How to use design tokens](https://m3.material.io/foundations/design-tokens/how-to-use-tokens)
- [Style Dictionary v4](https://styledictionary.com/)
- [Tokens Studio for Figma](https://tokens.studio/)
- [`culori` color library](https://culorijs.org/)
- [CSS Color Module Level 5 §2.1 (`color-mix()`)](https://www.w3.org/TR/css-color-5/#color-mix)
- CEM internal: `packages/cem-theme/src/lib/tokens/cem-m3-parity.md` — M3 role mapping
- CEM internal: `packages/cem-theme/docs/docs-generation.md` — md → XHTML → CSS pipeline
- CEM internal: `CLAUDE.md` — token manifest contract
