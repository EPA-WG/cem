# Multi-Platform Token Export — Design Document

**Status:** Proposal (not yet implemented). Recommends Approach E (Hybrid).
**Audience:** CEM maintainers, design-system reviewers, downstream platform teams.
**Decision required:** see §12 (Open decisions).

---

## 1. Context & goals

CEM ships a generated token manifest from canonical markdown specs in `packages/cem-theme/src/lib/tokens/*.md`.
The exact count is build output, not design-document truth; `dist/lib/tokens/generated-token-coverage.md` and
`manifest-utils.mjs` are authoritative. The CSS pipeline is mature and validated. The next strategic step is
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

### Implementation state

The repo has design notes for token export, but no implemented Style Dictionary config, DTCG build target, Figma import
files, or native platform outputs. Treat this as the first implementation architecture, not a migration plan. Any
prototype artifacts must either become generated outputs under `dist/` or be removed before release.

---

## 2. Architectural principles

```
┌─────────────────┐
│  *.md (truth)   │
└────────┬────────┘
         │ build:docs (compile-markdown.mjs)
         ▼
┌─────────────────┐         ┌──────────────────────────┐
│  *.xhtml        │────────▶│  CSS pipeline (existing) │──▶ dist/lib/css/*.css
│  (h6 + table)   │         └──────────────────────────┘
└────────┬────────┘
         │ export-tokens.mjs (NEW — extract + classify)
         ▼
┌─────────────────┐
│  intermediate   │
│     JSON        │
└────────┬────────┘
         │ export-tokens.mjs (NEW — resolve through generated CSS when needed)
         ▼
┌─────────────────┐         ┌──────────────────────────┐
│  resolved JSON  │────────▶│  DTCG emission           │──▶ dist/lib/tokens/cem.tokens.json + figma/
│  (per-theme)    │         └──────────┬───────────────┘     + cem.tokens.report.{md,json}
└─────────────────┘                    │
                                       ▼
                          ┌─────────────────────────────┐
                          │  Style Dictionary (Phase D) │──▶ dist/lib/token-platforms/{ios,android,js,scss,json}/
                          └─────────────────────────────┘
                                       │
                                       ▼
                          ┌─────────────────────────────┐
                          │  Tokens Studio / Figma      │──▶ Figma Variables (read-only)
                          │  (Phase E)                  │
                          └─────────────────────────────┘
```

Principles:

- **md → XHTML → DTCG JSON → consumers** is the new spine. CSS continues from XHTML directly (existing path
  unchanged).
- **DTCG-compatible JSON** is the canonical machine-readable export format. It is based on the
  [Design Tokens Community Group](https://design-tokens.github.io/community-group/format/) format used by Figma token
  import, Style Dictionary, and Tokens Studio, but CEM metadata stays namespaced under `$extensions.cem`.
- **Tier-aware emission** matches the CSS pipeline; the same `tier` column in source tables drives all outputs.
- **Token classification.** Every token belongs to one of three buckets: cross-platform / web-only / voice-audio
  (see §5).

---

## 3. Approach comparison

| Approach                                   | Source format                                        | Cross-platform fan-out                                                      | Figma path                                               | Pros                                                                                   | Cons                                                                                                                              |
| ------------------------------------------ | ---------------------------------------------------- | --------------------------------------------------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| **A. Style Dictionary on DTCG JSON**       | DTCG JSON (derived from XHTML)                       | Style Dictionary DTCG-aware transforms                                      | Figma import or Tokens Studio reads the same JSON        | Mature ecosystem; standards-aligned; one source feeds many targets                     | Adds Style Dictionary dep; needs custom transforms for `color-mix()` recipes; multi-dim modes need design                         |
| **B. Per-platform generators**             | XHTML directly (mirror CSS pipeline)                 | Hand-rolled `cem-ios.html`, `cem-android.html`, `cem-figma.html` generators | Custom Figma plugin or static JSON                       | Stays in established pattern; full control over output shape                           | Reinvents Style Dictionary's transform library; massive boilerplate per format; per-platform color/font conversion is error-prone |
| **C. DTCG JSON only**                      | DTCG JSON                                            | Consumer-owned (apps transform themselves)                                  | DTCG-aware tools consume directly                        | Lightest weight; pure-standards; future-proof                                          | App teams want native code, not "transform yourself"; high adoption friction                                                      |
| **D. Tokens Studio + GitHub two-way sync** | DTCG JSON in repo                                    | Same as A                                                                   | Tokens Studio syncs both ways via Git provider           | Designers can author in Figma, push to repo                                            | **Two-way sync risks designers overwriting md source-of-truth**; needs strict governance and review gates                         |
| **E. Hybrid (recommended)**                | XHTML → DTCG JSON → {Style Dictionary, Figma import} | Style Dictionary fans out to iOS/Android/JS                                 | Figma imports or pulls DTCG JSON in a read-only workflow | Best of A + C: web stays as-is, native gets Style Dictionary, Figma consumes standards | Most components to integrate; needs build-time color resolution                                                                   |

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

| M3 tier   | CEM equivalent                                                                         | Examples                                                                                  |
| --------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| Reference | Branded colors (D0), thickness scale, size scale, dimension scale, breakpoint widths   | `--cem-color-blue-xl`, `--cem-typography-thickness-bold`, `--cem-dim-large`               |
| System    | Emotional palette (D0), action intent tokens, semantic shape/stroke/layering endpoints | `--cem-palette-trust`, `--cem-action-primary-hover-background`, `--cem-bend-control`      |
| Component | Controls geometry (D2c), action border-radius, role typography                         | `--cem-control-height`, `--cem-action-border-radius`, `--cem-typography-button-font-size` |

`packages/cem-theme/src/lib/tokens/cem-m3-parity.md` already documents the role-by-role mapping. The DTCG export
will (optionally, behind a flag) emit an M3-shaped alias group so consumers expecting M3 names get a drop-in
adapter.

**Important framing:** M3, Angular Material, MUI, and other external systems are **adapter layers**, not
replacements for CEM tokens. CEM markdown specs remain canonical; CEM CSS custom properties remain the stable
web-facing API. Adapter sets (M3-shaped, etc.) are derived outputs, gated behind opt-in flags, and never become
the source of truth for any CEM token.

---

## 5. Token classification

Every CEM token belongs to one of three buckets. This drives **what gets exported where**:

| Bucket                    | Examples                                                                                                                                | DTCG JSON         | iOS/Android       | SCSS/JS | Figma | Notes                                              |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ----------------- | ----------------- | ------- | ----- | -------------------------------------------------- |
| **Cross-platform visual** | Branded colors, resolved palette/action colors, spacing, typography sizes/weights, timing, shape radii, controls geometry               | ✓                 | ✓ (resolved)      | ✓       | ✓     | Recipes resolve per theme when inputs are portable |
| **Web-only**              | `forced-colors` fallbacks, `@container` rules, unresolved system colors, ring composite recipes, CSS selectors and media-query behavior | —                 | —                 | —       | —     | CSS only (existing pipeline)                       |
| **Voice/audio data**      | `--cem-voice-*-{speech-rate,speech-pitch,speech-volume,ssml-emphasis,ink-thickness,icon-stroke-multiplier}`                             | ✓ (separate file) | v2 / TTS adapters | ✓       | —     | Separate `cem.voice.tokens.json`; not visual       |

### Concrete translation gotchas

| Token                                        | Value                                                          | Translation strategy                                                                                                          |
| -------------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `--cem-color-visitedtext-30-black`           | `color-mix(in srgb, VisitedText 30%, black)`                   | System color `VisitedText` resolves only on web/native theme. Other platforms require an explicit mapping or skip report.     |
| `--cem-bend-round`                           | `calc(var(--cem-shape-height, var(--cem-control-height)) / 2)` | Resolve to numeric px/rem at build time; mobile cannot evaluate nested `calc()` at token load.                                |
| `--cem-palette-trust`                        | `light-dark(var(--cem-color-blue-l), var(--cem-color-blue-d))` | Map to two values per theme: light= and dark=. iOS asset catalogs and Android `values-night/` consume them directly.          |
| `--cem-typography-data-font-variant-numeric` | `tabular-nums lining-nums`                                     | OpenType feature settings; Android Roboto may not support `lining-nums`. Document as recommended, not required, on Android.   |
| `--cem-voice-loud-speech-rate`               | `0.94`                                                         | Not a visual property; routes only to TTS adapter (AVSpeechSynthesizer / Android TextToSpeech). Excluded from visual exports. |

### 5.1 Portability classification (per-value)

Bucket tagging (above) is per-token. **Portability** is a finer-grained per-value tag that drives how each token
value is emitted to each target. Every emitted token carries a `portability` field in `$extensions.cem`:

| Portability      | Meaning                                                                                                                    | Emission strategy                                                                                        |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| `literal`        | Directly portable scalar value                                                                                             | `#ecf0ff`, `4px`, `200ms`, `400` — emit as-is to every target                                            |
| `alias`          | References another token; can be expressed as a DTCG reference                                                             | DTCG-aware targets get `"$value": "{cem.color.blue.l}"`; non-aware targets get the resolved primitive    |
| `mode`           | Different concrete values per theme mode                                                                                   | Emit as DTCG modes (Figma) or per-resource-qualifier files (Android `values-night/`, iOS asset catalogs) |
| `css-expression` | Requires CSS runtime evaluation that cannot be pre-resolved per theme (e.g., `currentColor`, `@container` math)            | Skip on non-web targets; document in report; CSS keeps the expression                                    |
| `platform-note`  | Semantically useful but not directly representable on the target (e.g., system colors, SSML emphasis, complex font stacks) | Skip on incompatible targets; emit metadata-only entry to the report                                     |

**Rule:** the exporter classifies every token at extraction time and propagates the classification through every
output. Reports list every `css-expression` and `platform-note` skipped per target so consumers can see what the
target _cannot_ represent — no silent loss.

This taxonomy is orthogonal to the bucket: a "cross-platform visual" token may still be `css-expression` on
non-web (e.g., `--cem-bend-round` is `calc(...)` resolvable per theme; on iOS it becomes `literal` after
resolution; on a target that only consumes raw DTCG without resolution it stays `css-expression`).

---

## 6. Mode strategy

CEM has multiple orthogonal mode dimensions:

| Mode dimension  | Values                                                       | Cardinality |
| --------------- | ------------------------------------------------------------ | ----------- |
| Theme           | `light`, `dark`, `contrast-light`, `contrast-dark`, `native` | 5           |
| Spacing density | `dense`, `normal`, `sparse`                                  | 3           |
| Coupling        | `forgiving`, `balanced`, `compact`                           | 3           |
| Shape style     | `sharp`, `smooth`, `round`                                   | 3           |

Cartesian product = 135 tuples — impractical to express as a single DTCG mode axis.

**Strategy:**

1. **Theme is the only exported mode axis.** Cross-platform tokens may get mode values for `light` and `dark` first,
   then `contrast-light` / `contrast-dark` once concrete platform mappings are proven. `native` stays web-only unless
   a target has an explicit system-color mapping. This maps to:
    - Figma import mode files, where every imported mode must contain the same token names and types
    - iOS asset catalogs (`Any Appearance`, `Dark`, and later high-contrast variants)
    - Android resource qualifiers (`values/`, `values-night/`, and later high-contrast qualifiers if supported)
2. **Spacing/coupling/shape are parallel groups.** Each appears as its own token group with explicit prefix
   (e.g., `spacing.dense.layout-stack-gap`, `spacing.normal.layout-stack-gap`, `spacing.sparse.layout-stack-gap`).
   Consumers select at app level (toggle a CSS attribute on web; pick a Compose theme on Android; etc.).
3. **Forced-colors and reduced-motion are platform-native conventions.** Not exported as tokens. iOS adapters
   read `UITraitCollection.accessibilityContrast` / `UIAccessibility.isReduceMotionEnabled`; Android reads
   `Configuration.uiMode` / `Settings.Global.TRANSITION_ANIMATION_SCALE`. Adapter docs (Phase F) describe the
   mapping per platform.

Counts: 9 parallel groups (3 + 3 + 3) instead of 135 tuples; the theme axis is the only true import/export mode
dimension.

**Mode-completeness rule.** A token must have a concrete value for every mode it claims, or be excluded from all
mode files for that target. Partial mode coverage (token X has a light value but no dark value) is a fail-hard
validation error — Figma collections and iOS asset catalogs both reject mismatched mode sets. Tokens with
incomplete mode coverage are emitted only into the canonical DTCG JSON with a `platform-note` portability tag and
listed in the per-target report.

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

**Decision (per portability tier):**

- `literal` values pass through unchanged.
- `alias` values are **preserved as DTCG references** (`"$value": "{cem.color.blue.l}"`) for consumers that
  understand them (Figma Variables, Tokens Studio, Style Dictionary, DTCG-aware tooling). Non-aware consumers
  get the resolved primitive via Style Dictionary's reference resolver. **Aliases preserved over resolution
  whenever the target supports references** — this keeps the semantic graph intact in design tools.
- `mode` values resolve to per-theme primitives at build time and emit per Figma/iOS/Android mode conventions.
- `css-expression` recipes resolve at build time to per-theme RGBA _only if_ the recipe's inputs are themselves
  resolvable (e.g., `color-mix(in srgb, var(--cem-palette-trust) 70%, var(--cem-palette-trust-x))` — both inputs
  are alias-resolvable). Otherwise the recipe is reported as CSS-only and skipped on non-web targets.
- `platform-note` values never resolve; they're skipped with a report entry.

Web keeps recipes verbatim (existing behavior, no change). The CSS pipeline never sees the DTCG JSON layer.

**Implementation:** `packages/cem-theme/scripts/export-tokens.mjs` uses Playwright for computed-value capture when a
target needs resolved values. It should serve a minimal fixture over HTTP that loads the generated
`dist/lib/css/*.css` files, applies `.cem-theme-*` classes and `data-cem-*` attributes to `<html>`, then reads
`getComputedStyle(document.documentElement).getPropertyValue('--cem-...')` for every token. This follows the repo's
HTTP-served custom-element constraints and lets the browser resolve `light-dark()`, `color-mix()`, `calc()`, and
`var()` chains from the same CSS output shipped to web consumers.

For pure JSON post-processing (when Playwright is not viable), an explicit color-math dependency such as
[`culori`](https://culorijs.org/) can be added to resolve `color-mix(in srgb, ...)` per
[CSS Color Module Level 5 §2.1](https://www.w3.org/TR/css-color-5/#color-mix). That fallback must be treated as a
separate implementation path and validated against browser-computed values.

---

## 8. Implementation phases

### Phase A — Token-data extraction (foundational)

Add `derive*Tokens()` companions to `derive*Manifest()` in
`packages/cem-theme/scripts/manifest-utils.mjs`. Where the existing manifest functions return
`{ name, tier, categoryId }`, the new ones return `{ name, valueRaw, tier, description, mode, category, sourceTable }`.

Add `packages/cem-theme/scripts/export-tokens.mjs` as the orchestration script. Its first stage iterates compiled
`dist/lib/tokens/*.xhtml`, calls each derivation, and builds an in-memory intermediate model. The script may emit
`dist/lib/tokens/cem.tokens.intermediate.json` behind a debug flag, but the intermediate file is not a public package
contract.

**Output shape (excerpt):**

```json
{
  "version": "0.0.7",
  "generatedAt": "2026-04-29T10:00:00Z",
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

The same `export-tokens.mjs` script resolves values that non-web targets cannot evaluate. Because resolution needs the
generated CSS custom properties, `build:tokens` depends on `build:css`, not just `build:docs`.

For computed capture:

1. Launch headless Chromium with Playwright.
2. Serve a minimal local HTML fixture over HTTP. The fixture links or injects the generated `dist/lib/css/*.css` files.
3. For each supported theme class (`light`, `dark` first; contrast modes once mapped):
    - Set the relevant `.cem-theme-*` class on `<html>`.
    - Set `data-cem-spacing`, `data-cem-coupling`, and `data-cem-shape` only for the parallel-group tokens being
      resolved.
    - Read `getComputedStyle(document.documentElement).getPropertyValue(token.name)`.
    - Capture the resolved RGBA, numeric, dimension, or string value.
4. Keep both `valueRaw` and `valueByMode` in memory, and optionally write `dist/lib/tokens/cem.tokens.resolved.json`
   behind a debug flag.

**Output shape (excerpt):**

```json
{
    "--cem-palette-trust": {
        "valueRaw": "light-dark(var(--cem-color-blue-l), var(--cem-color-blue-d))",
        "valueByTheme": {
            "light": "rgb(33, 87, 178)",
            "dark": "rgb(122, 158, 220)",
            "contrast-light": "rgb(0, 0, 102)",
            "contrast-dark": "rgb(180, 200, 240)"
        }
    }
}
```

Spacing/coupling/shape mode resolution happens analogously by setting `data-cem-spacing`, `data-cem-coupling`,
`data-cem-shape` attributes on `<html>` and re-reading the affected tokens. Each becomes a parallel token group in
the DTCG output.

### Phase C — DTCG JSON emission

The final stage of `export-tokens.mjs` transforms the extracted and resolved model to the
[DTCG-compatible format](https://design-tokens.github.io/community-group/format/).

- Tier-aware filtering (default: required + recommended; `--with-optional`, `--with-adapter`,
  `--with-deprecated` flags add tiers).
- Theme mode values in `$extensions.cem.modes`, plus separate Figma mode files where needed.
- Spacing/coupling/shape as parallel groups.
- Web-only categories filtered out (forced-colors fallbacks, ring recipes, system color references that don't
  resolve outside web).
- Aliases preserved (`"$value": "{cem.color.blue.l}"`) when source token references another token; resolution
  happens only when the consumer can't follow references.
- Every emitted token carries `$extensions.cem` metadata pointing back to its markdown source — this is the
  traceability contract that lets reviewers verify a derived value matches the spec.

**`$extensions.cem` shape (every token):**

```json
{
    "cem": {
        "palette": {
            "comfort": {
                "$type": "color",
                "$value": "{cem.color.blue.l}",
                "$description": "Primary comfort surface color.",
                "$extensions": {
                    "cem": {
                        "cssName": "--cem-palette-comfort",
                        "spec": "cem-colors",
                        "sourceTable": "cem-palette-emotion-shift",
                        "tier": "required",
                        "category": "d0-palette",
                        "rawValue": "light-dark(var(--cem-color-blue-l), var(--cem-color-blue-d))",
                        "portability": "alias",
                        "modes": {
                            "light": "rgb(33, 87, 178)",
                            "dark": "rgb(122, 158, 220)"
                        }
                    }
                }
            }
        }
    }
}
```

`$extensions.cem` fields:

| Field         | Purpose                                                                                 |
| ------------- | --------------------------------------------------------------------------------------- |
| `cssName`     | Original CSS custom property name (`--cem-...`) — preserved across all derived outputs  |
| `spec`        | Source spec name (e.g., `cem-colors`) for back-link to markdown                         |
| `sourceTable` | h6 id of the source table in the spec                                                   |
| `tier`        | `required` / `recommended` / `optional` / `adapter` / `deprecated`                      |
| `category`    | Internal categorization (e.g., `d0-palette`, `d2c-controls`) for filtering and grouping |
| `rawValue`    | Original CSS value verbatim (recipe with `var()`/`color-mix()`/`calc()` intact)         |
| `portability` | `literal` / `alias` / `mode` / `css-expression` / `platform-note` (see §5.1)            |
| `modes`       | Resolved per-theme values when `portability` is `mode` or `alias` with theme variance   |

**Outputs:**

- `dist/lib/tokens/cem.tokens.json` — canonical DTCG JSON, visual tokens (cross-platform bucket).
- `dist/lib/tokens/cem.voice.tokens.json` — voice/audio bucket, separate file (for TTS adapters).
- `dist/lib/tokens/cem.tokens.report.md` — human-readable report listing every token, its portability, and what
  was skipped per target and why. **First-class output, not optional.**
- `dist/lib/tokens/cem.tokens.report.json` — machine-readable equivalent for CI assertions.

**Generated provenance.** Text/code outputs include generated-file headers. JSON outputs cannot use comments, so they
carry provenance in top-level `$extensions.cem.generated` metadata: package version, generation timestamp, source
spec list, and build command. No generated artifact may ship without provenance.

### Phase D — Style Dictionary fan-out

Add `packages/cem-theme/style-dictionary.config.mjs` and an Nx target `build:token-platforms` that depends on
`build:tokens`. Keep `build:css` independent so the existing web output can ship even if native exports remain
experimental.

Custom transforms required:

- `cem/size/rem-to-pt` (iOS): rem → pt at 16pt base.
- `cem/size/rem-to-dp` (Android): rem → dp at 16dp base.
- `cem/size/rem-to-sp` (Android, typography only): rem → sp.
- `cem/category/web-only-filter`: drop web-only categories before emission.
- `cem/mode/expand-themes`: expand `$extensions.cem.modes` to per-platform conventions (asset catalog hints for iOS;
  `values-night/` for Android).

**Outputs:**

```
dist/lib/token-platforms/
├── ios/
│   ├── CEMTokens.swift                    (struct CEMTokens { static let colorPaletteTrust = UIColor(...) })
│   ├── CEMTokens.xcassets-hints.json      (asset-catalog scaffolding)
│   └── ios-report.md                      (skipped tokens + reasons)
├── android/
│   ├── values/cem-tokens.xml              (light theme)
│   ├── values-night/cem-tokens.xml        (dark theme)
│   ├── values-night-hcc/cem-tokens.xml    (contrast-dark)
│   ├── compose/CEMTokens.kt               (val cemColorPaletteTrust = Color(...))
│   └── android-report.md                  (skipped tokens + reasons)
├── js/
│   ├── cem-tokens.ts                      (typed exports + metadata)
│   └── js-report.md
├── json/
│   ├── cem-tokens.json                    (resolved-per-theme, flat)
│   └── json-report.md
└── scss/
    ├── cem-tokens.scss                    ($cem-color-palette-trust: ...;)
    └── scss-report.md
```

**Per-platform report contract.** Every platform output ships a `*-report.md` documenting:

1. Token coverage: how many of the manifest's required/recommended tokens emitted to this target.
2. Skipped tokens with reason: `css-expression unresolvable on iOS`, `system color VisitedText has no native
equivalent on Android`, `voice token excluded from visual export`, etc.
3. Platform-specific transformation choices applied (e.g., `rem→pt at 16pt base`, `color resolved via Playwright
in light theme`).
4. Validation summary: pass/fail counts for each fail-hard rule (see §11).

**TypeScript metadata contract.** The JS/TS output is for tooling, docs, autocomplete, and tests. It is not a
replacement for CSS runtime styling in browsers. At minimum it should expose:

```ts
export type CemTokenName = '--cem-palette-comfort' | '--cem-action-primary-default-background';

export interface CemTokenMeta {
    name: CemTokenName;
    type: 'color' | 'dimension' | 'number' | 'duration' | 'fontFamily' | 'string';
    tier: 'required' | 'recommended' | 'optional' | 'adapter' | 'deprecated';
    spec: string;
    sourceTable: string;
    portability: 'literal' | 'alias' | 'mode' | 'css-expression' | 'platform-note';
}
```

### Phase E — Figma integration

Two paths, both **read-only** (write-back deferred until governance is designed):

1. **Tokens Studio** (optional bridge).
    - Configure a pull-only workflow pointing at generated token JSON. If the chosen storage provider cannot be
      permissioned read-only, treat Tokens Studio as a manual import path rather than a sync source.
    - Designers fetch in the plugin; tokens appear as Figma Variables organized by dimension.
    - Push/write-back stays disabled until a governance process exists for converting designer edits into markdown
      spec changes.
2. **Figma Variables direct DTCG import.**
    - Designers or design-ops drag-drop generated DTCG JSON files into the Variables panel.
    - Lower operational risk than plugin sync; no write-back path.
    - Use this as the MVP validation path before adding any automated Figma REST API sync.

### Figma import contract

Figma import has stricter requirements than canonical DTCG JSON. Generate separate Figma files and validate them before
manual import:

- Import one file per mode (`cem-light.tokens.json`, `cem-dark.tokens.json`, later contrast files). Figma creates or
  updates variables only when a token is present in every imported mode file and has the same `$type` in each file.
- Emit only supported value shapes: sRGB/HSL colors, `dimension` values in `px`, `duration` values in `s`, single-string
  `fontFamily`, numbers, booleans encoded through `com.figma.type`, and strings.
- Normalize names exactly as Figma does: nested DTCG groups become slash-separated names. Duplicate normalized names
  must fail before import because Figma ignores duplicates after the first one.
- Keep aliases inside the same collection where possible. Cross-collection aliases require Figma-specific
  `com.figma.aliasData` metadata and should be deferred until the one-collection flow is proven.
- Exclude unsupported or incomplete-mode tokens from all Figma mode files and list them in `cem-figma-report.md`.

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

Path layout consolidates exports under `dist/lib/tokens/` (canonical + Figma-shaped) and `dist/lib/token-platforms/`
(Style Dictionary outputs). This mirrors the existing `dist/lib/css/` convention and keeps related artifacts
co-located.

```
packages/cem-theme/
├── docs/
│   └── token-export.md                          (this file)
├── scripts/
│   ├── manifest-utils.mjs                       (extended: tokensFromTableWithValues)
│   ├── derive-tokens.mjs                        (new: full derive*Tokens() helpers)
│   ├── export-tokens.mjs                        (new: XHTML + CSS → DTCG JSON + Figma + reports)
│   ├── build-token-platforms.mjs                (new: Style Dictionary driver)
│   └── validate-platforms.mjs                   (new: post-emit coverage + name validation)
├── style-dictionary.config.mjs                  (new)
└── dist/lib/                                    (generated)
    ├── tokens/
    │   ├── cem-colors.xhtml                     (existing)
    │   ├── ...other spec xhtml...               (existing)
    │   ├── cem.tokens.json                      (new: canonical DTCG)
    │   ├── cem.voice.tokens.json                (new: voice/TTS metadata, v2 adapters)
    │   ├── cem.tokens.report.md                 (new: human-readable portability report)
    │   ├── cem.tokens.report.json               (new: CI-readable equivalent)
    │   └── figma/
    │       ├── cem-light.tokens.json            (Figma mode file)
    │       ├── cem-dark.tokens.json             (Figma mode file)
    │       └── cem-figma-report.md              (skipped tokens + Figma-specific notes)
    └── token-platforms/                         (Style Dictionary outputs)
        ├── ios/
        ├── android/
        ├── js/
        ├── json/
        └── scss/

examples/
├── ios/CEMTokensExample/
├── android/cem-tokens-example/
├── figma/README.md
└── web/import-tokens.ts
```

Rationale for path consolidation:

- `dist/lib/tokens/` already holds the compiled XHTML; adding canonical JSON, reports, and Figma files keeps all
  token artifacts in one tree.
- `dist/lib/token-platforms/` (new) clearly signals "Style Dictionary derived" outputs distinct from CSS or XHTML.
- Single `export-tokens.mjs` script handles extraction, classification, CSS-backed resolution, DTCG emission, Figma
  split, and reports; `build-token-platforms.mjs` is the Style Dictionary driver. Two scripts, two Nx targets.

---

## 10. Reused infrastructure

The pipeline extends — does not replace — what already exists:

| Existing utility                      | Path                                                    | New use                                                                        |
| ------------------------------------- | ------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `extractTable()`, `tokensFromTable()` | `packages/cem-theme/scripts/manifest-utils.mjs`         | Add `tokensFromTableWithValues()` capturing value cells, not just names + tier |
| Validator pattern                     | `packages/cem-theme/scripts/validate-manifest.mjs`      | Mirror as `validate-platforms.mjs` for non-CSS outputs                         |
| Playwright capture/debug scripts      | `tools/scripts/capture-xpath-text.mjs`, `debug-cem.mjs` | Model for CSS-backed computed-value capture in `export-tokens.mjs`             |
| Markdown → XHTML compilation          | `tools/scripts/compile-markdown.mjs`                    | Unchanged; export pipeline reads its output                                    |
| Nx target wiring                      | `packages/cem-theme/project.json`                       | Add `build:tokens` and `build:token-platforms` targets parallel to `build:css` |

### Nx target shape

```json
{
    "build:tokens": {
        "dependsOn": ["build:css"],
        "executor": "nx:run-commands",
        "options": {
            "command": "node scripts/export-tokens.mjs",
            "cwd": "packages/cem-theme"
        },
        "outputs": [
            "{projectRoot}/dist/lib/tokens/cem.tokens.json",
            "{projectRoot}/dist/lib/tokens/cem.voice.tokens.json",
            "{projectRoot}/dist/lib/tokens/cem.tokens.report.md",
            "{projectRoot}/dist/lib/tokens/cem.tokens.report.json",
            "{projectRoot}/dist/lib/tokens/figma"
        ]
    },
    "build:token-platforms": {
        "dependsOn": ["build:tokens"],
        "executor": "nx:run-commands",
        "options": {
            "command": "node scripts/build-token-platforms.mjs",
            "cwd": "packages/cem-theme"
        },
        "outputs": ["{projectRoot}/dist/lib/token-platforms"]
    }
}
```

`build:css` does **not** depend on `build:tokens` — the CSS pipeline ships independently. `build:tokens` depends on
`build:css` because resolved non-web values are captured from generated CSS. `build` can add `build:tokens` after the
canonical JSON/report outputs are stable, and add `build:token-platforms` only after native outputs leave the
experimental phase.

---

## 11. Verification plan

### 11.1 Validation contract — fail hard

The exporter **must exit non-zero and fail the build** on any of these:

- A token marked `required` or `recommended` in any spec's manifest is missing from `cem.tokens.json`.
- Two tokens collide on canonical DTCG path (e.g., both `--cem-foo-bar` and `--cem-foo--bar` normalize to
  `cem.foo.bar`).
- Two tokens collide on Figma slash-normalized name within one collection (e.g., `--cem-action-primary-default-background`
  and `--cem-action-primary-default--background` both normalize to `action/primary/default/background`).
- Same token name has different `$type` across mode files (e.g., `color` in light, `dimension` in dark).
- `cem.tokens.json` fails W3C DTCG schema validation.
- A platform output references a token that should be filtered out for that platform (e.g., voice token in iOS
  visual export).
- A generated output is missing the standard generated provenance header or JSON metadata.
- Mode-completeness violation: a token has a value for one mode but not all modes it claims (see §6).

### 11.2 Validation contract — warn and report

The exporter logs a warning and includes the token in the per-target report (but does not fail):

- `optional`-tier token has no portable value for a target — listed as "skipped optional".
- `css-expression` portability that cannot resolve outside CSS — listed with the original recipe.
- System color value (`Highlight`, `CanvasText`, etc.) that has no native equivalent — listed under
  `platform-note`.
- `deprecated` tokens excluded from a target — listed with deprecation note.
- `adapter`-tier tokens excluded from default outputs — listed under "opt-in via `--with-adapter`".

### 11.3 Per-phase verification

| Phase | Verification                                                                                                                                                                                |
| ----- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A     | `export-tokens.mjs` produces JSON with every manifest-derived token name eligible for export; manifest validator confirms parity with CSS output.                                           |
| B     | 5-token spot check per theme matches manual `getComputedStyle()` capture from `tools/scripts/debug-cem.mjs`.                                                                                |
| C     | DTCG JSON validates against W3C DTCG schema (`@design-tokens/parser` or `tokens-json-validator`). Figma Variables imports without error. Report file lists every skipped token with reason. |
| D     | Generated Swift compiles with Xcode 15+; Kotlin with Gradle 8+; TypeScript passes `tsc --noEmit`; SCSS compiles with `dart-sass`. Each per-platform report shows zero fail-hard violations. |
| E     | Figma Variables direct DTCG import succeeds; modes (light/dark) align. Optional Tokens Studio pull workflow loads the same generated JSON without write-back.                               |
| F     | Sample iOS and Android apps render a button + card using only CEM tokens; visual parity with web reference (manual screenshot diff).                                                        |
| G     | `yarn nx affected -t lint test build typecheck` green; `verify:phase13` green.                                                                                                              |

### 11.4 End-to-end smoke (after Phases A–G land)

1. Change a value in `cem-colors.md` (e.g., `--cem-color-blue-xl: #ecf0ff` → `#e0e8ff`).
2. Run `yarn build`.
3. Verify the new value flows through:
    - `dist/lib/css/cem-colors.css` (existing CSS pipeline)
    - `dist/lib/tokens/cem.tokens.json`
    - `dist/lib/token-platforms/ios/CEMTokens.swift`
    - `dist/lib/token-platforms/android/values/cem-tokens.xml`
    - `dist/lib/token-platforms/js/cem-tokens.ts`
4. Import or refresh the Figma test collection; verify the new value appears in the Variables panel.
5. Diff the per-target report files against the previous build — only the changed token should appear in the
   diff, with no spurious changes to other tokens' portability or mode classifications.

---

## 12. Open decisions (require sign-off before Phase A)

| #   | Decision                                 | Recommendation                                                                                                                           | Rationale                                                                                                                                                                                                  |
| --- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Style Dictionary version                 | **Current project-compatible major, pinned by lockfile**                                                                                 | Style Dictionary has DTCG support from v4 onward and newer v5 releases add DTCG 2025.10 support. Choose the version that matches the repo's Node runtime and required DTCG value shapes.                   |
| 2   | Tokens Studio plan                       | **Optional pull-only workflow; direct Figma import is the MVP path**                                                                     | Tokens Studio Git providers support two-way sync, which is useful later but risky before governance exists. Do not depend on a paid/free tier claim in this design doc.                                    |
| 3   | Figma sync direction                     | **Read-only** (md → DTCG → Figma)                                                                                                        | Two-way sync risks designer-authored drift; governance for designer changes not yet designed.                                                                                                              |
| 4   | Voice token export scope                 | **Defer to v2**                                                                                                                          | No consumer apps with TTS adapters yet; emit metadata placeholder only in v1. Saves complexity.                                                                                                            |
| 5   | Adapter and deprecated tier export       | **Flag-gated** (`--with-adapter`, `--with-deprecated`)                                                                                   | Matches the existing CSS pipeline contract.                                                                                                                                                                |
| 6   | Color-resolution math library            | **Browser capture first; add `culori` only for explicit non-Playwright fallback**                                                        | Browser capture validates against shipped CSS behavior. A JS color library is useful only if it is tested against browser-computed values.                                                                 |
| 7   | DTCG mode encoding                       | **Canonical `$extensions.cem.modes`; Figma gets one generated file per mode**                                                            | Multi-mode DTCG support is still tool-specific. Keeping CEM modes namespaced avoids overcommitting to one draft shape while Figma import remains file-per-mode.                                            |
| 8   | Output stability commitment              | **Stabilize after Phase D ships once**                                                                                                   | Token names in CSS are already stable; DTCG/platform names need one full release cycle to stabilize before promising backwards compat.                                                                     |
| 9   | Px → dp/pt mapping policy                | **Per token category** (spacing/shape → dp; typography sizes → sp; iOS uses points uniformly)                                            | Global px→dp creates accessibility issues — Android typography must scale with system font size (sp), other dimensions should not. iOS points are unitless on layout. Document per category, not globally. |
| 10  | `color-mix()` resolution policy          | **Resolve at export time per theme; emit alias-preserved DTCG for DTCG-aware consumers**                                                 | Both paths coexist: native platforms get pre-resolved RGBA; DTCG-aware tools see the alias graph. Custom Style Dictionary transform reads `$extensions.cem.modes` for native targets.                      |
| 11  | Tokens Studio support tier               | **Optional bridge, not source of truth**                                                                                                 | Same DTCG JSON serves Tokens Studio and direct Figma Variables import; designers choose the workflow. CEM commits to neither.                                                                              |
| 12  | Direct Figma REST API sync               | **Defer to post-v1**; manual file import is sufficient                                                                                   | API sync requires file-id config, write permissions, and CI plumbing — adds complexity without proportional value while `cem.tokens.json` import works.                                                    |
| 13  | Figma collections                        | **One CEM collection** with groups by dimension; split only if Figma's per-collection limits hit or designers report navigation friction | Single collection simplifies cross-token references; splitting later is a non-breaking move (consumers reimport).                                                                                          |
| 14  | iOS dynamic type interaction             | **Do not auto-scale** in v1                                                                                                              | CEM typography sizes are nominal; product apps decide whether to apply iOS dynamic type. Exposing a hook is a future enhancement.                                                                          |
| 15  | Android color modes                      | **Resource qualifiers (`values-night/`)** for v1; Compose color schemes optional                                                         | Resource qualifiers are the platform-native pattern and work without app-level wiring. Compose color schemes are a Compose-only convenience and can be added once value coverage stabilizes.               |
| 16  | Package export map for generated JSON/TS | **Expose `dist/lib/tokens/cem.tokens.json` and TS metadata via `package.json` exports field**                                            | Consumers reach the artifacts via `import meta from '@epa-wg/cem-theme/tokens'` rather than deep paths; one less migration surface if internal layout changes.                                             |

---

## 13. Risks and mitigations

| Risk                                                               | Impact                                                                            | Mitigation                                                                                                                                                       |
| ------------------------------------------------------------------ | --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Style Dictionary's `color-mix()` support is incomplete             | Cross-platform colors may emit as raw `color-mix(...)` strings that don't compile | Resolve recipes before Style Dictionary sees them; Style Dictionary receives concrete values plus CEM metadata                                                   |
| Figma Variables and Tokens Studio interpret DTCG modes differently | Designers see different mode behavior than expected                               | Validate direct Figma import first; treat Tokens Studio as optional bridge until mode behavior is proven                                                         |
| Multi-mode dimensionality explosion                                | DTCG output becomes huge and hard to navigate                                     | Theme as the only import/export mode axis; spacing/coupling/shape as parallel groups (9 vs 135)                                                                  |
| Voice tokens leaking into visual exports                           | Designers see speech-rate as a "color" or "size" in Figma                         | Hard-filter via category prefix; separate `cem.voice.tokens.json`; Style Dictionary's `cem/category/web-only-filter` excludes voice from visual platform outputs |
| Native platforms can't represent all theme modes                   | iOS and Android may not have a clean "contrast-light" or "native" equivalent      | Map theme tuples to platform conventions: light → `Any Appearance`, dark → `Dark`, contrast-\* → high-contrast variants; native theme is web-only and excluded   |
| DTCG draft format changes before W3C ratification                  | Output JSON shape may need migration                                              | Pin DTCG schema version in `cem.tokens.json` `$schema`; add migration script if the spec revision requires a new wire shape                                      |

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

## 15. Recommended MVP

The full Phase A–G arc is the destination. The MVP is the **smallest slice that delivers measurable value**:

1. **Canonical DTCG JSON** — `dist/lib/tokens/cem.tokens.json` with `$extensions.cem` traceability and per-theme
   resolved modes (Phases A + B + C, scoped to required + recommended tiers).
2. **Portability report** — `cem.tokens.report.md` listing every token, its portability classification, and what
   the canonical export resolved vs. preserved as alias vs. skipped.
3. **Figma file exports** — `cem-light.tokens.json` and `cem-dark.tokens.json` for direct Figma Variables import,
   covering only `literal`-portability and `alias`-portability tokens. `css-expression` and `platform-note`
   tokens listed in `cem-figma-report.md`.
4. **TypeScript metadata** — `cem-tokens.ts` typed token names + metadata for consumer tooling, autocomplete in
   IDEs, and docs generation. Not a runtime CSS replacement.
5. **Defer native** — Android XML/Compose and iOS Swift exports stay behind a separate `build:token-platforms`
   target until unit-mapping policy (decisions §9, §14, §15) and mode coverage are validated.

Why this slice:

- Designers and developers get usable artifacts on day one (DTCG JSON for tooling, Figma files for design).
- The hardest cross-platform questions (px→dp, dynamic type, asset catalogs) are deferred until evidence accrues.
- Native exports are the _most_ expensive to maintain incorrectly — premature shipping creates support load
  without commensurate adoption.
- Reports surface every gap so the next milestone can prioritize what's missing.

After the MVP ships and stabilizes, native platform exports follow under the experimental stability label.

---

## 16. Stability and versioning

Generated artifacts have different maturity. The doc commits to explicit stability labels so consumers know what
they can rely on:

| Tier             | Includes                                                                                                                      | Stability commitment                                                                                   |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| **Stable**       | Markdown specs (`packages/cem-theme/src/lib/tokens/*.md`), CSS custom properties (`dist/lib/css/*.css`), required token names | Backwards compatible; breaking changes only with major version bump and migration notes                |
| **Beta**         | Canonical DTCG JSON (`cem.tokens.json`), TypeScript metadata, portability reports                                             | Schema may evolve in minor releases; renames discouraged; consumers should pin minor versions          |
| **Experimental** | Figma exports, Android XML/Compose, iOS Swift, asset-catalog hints, direct Figma REST API sync                                | May change shape, naming, or structure freely between minor releases; consumers regenerate per release |

**Breaking changes** (require major version bump):

- Removing or renaming a token in the **Stable** tier.
- Changing a token's `$type` in canonical DTCG JSON.
- Changing Figma slash-normalized names after a release publishes.
- Changing px→dp / px→pt / px→sp default mapping policy per category.
- Moving a token from `required` to `optional` in a way that removes it from default outputs.
- Removing a portability classification value (`literal`/`alias`/`mode`/`css-expression`/`platform-note`).

**Non-breaking changes** (minor bump):

- Adding new tokens, specs, or modes.
- Adding new platform outputs (e.g., a new `dist/lib/token-platforms/dart/` for Flutter).
- Adding fields to `$extensions.cem` metadata.
- Tightening portability classification (e.g., a token previously `css-expression` becomes `mode` once
  resolution is implemented for it).

**Generated provenance** carries the version and source-spec timestamp so a downstream consumer can detect when their
cached copy diverges from the upstream package version.

---

## References

- [W3C Design Tokens Community Group format](https://design-tokens.github.io/community-group/format/)
- [Material Design 3 — How to use design tokens](https://m3.material.io/foundations/design-tokens/how-to-use-tokens)
- [Style Dictionary](https://styledictionary.com/)
- [Tokens Studio for Figma](https://tokens.studio/)
- [`culori` color library](https://culorijs.org/)
- [CSS Color Module Level 5 §2.1 (`color-mix()`)](https://www.w3.org/TR/css-color-5/#color-mix)
- [Figma Variables modes](https://help.figma.com/hc/en-us/articles/15343816063383-Modes-for-variables)
- [Figma Variables REST API](https://developers.figma.com/docs/rest-api/variables/)
- CEM internal: `packages/cem-theme/src/lib/tokens/cem-m3-parity.md` — M3 role mapping
- CEM internal: `packages/cem-theme/docs/docs-generation.md` — md → XHTML → CSS pipeline
- CEM internal: `packages/cem-theme/docs/token-export-codex.md` — superseded rationale merged into this canonical
  design document
- CEM internal: `CLAUDE.md` — token manifest contract
