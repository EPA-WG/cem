# `todo-criticque.md` — Bullet-by-Bullet Followup

This document records how every bullet from `todo-criticque.md` was addressed in the revised `docs/todo.md`. Each row
links the critique point to a concrete change: a new principle (P*), a phase task, or an R&D entry (R-*-*).

Legend:
- **A** = Adopted as-is in `docs/todo.md`.
- **A+R** = Adopted; additionally surfaced as an R&D / Open Decision entry because canonical design needs to make a choice.
- **R** = Pure R&D entry (canonical design gap; generator can't proceed until decided).
- **D** = Deferred / partially adopted with a note explaining scope.

---

## Overall Findings

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| O1 | Plan treats "generate every token" as the main success measure; misses tier distinction (required / recommended / optional / adapter / deprecated). | A | Principle **P3** (tier-aware emission); manifest schema in Phase 4 task 1; manifest column `tier`. |
| O2 | Reorder D2 coupling before D3 shape and D5 stroke. | A | Phase order revised: …→ Phase 8 D2 → Phase 9 D3 → Phase 10 D5. Note in Phase 8 explains the dependency on `--cem-control-height` and guard math. |
| O3 | Add a metadata-schema task before more generators; pick one schema with stable IDs / columns / required flags / expected names. | A | Principle **P2** (h6 + table convention); Phase 4 task 1 (schema definition) + task 2 (`index.md` documentation) + task 3 (`cem-colors.md` worked example). R&D **R-Schema-1** captures any open column-set decisions. |
| O4 | Add manifest-based verification — exact tokens, no placeholders, no template remnants, valid CSS. | A | Principle **P1** (manifest contract) and **P4** (verification dimensions). Phase 4 task 4 builds the validator script; task 5 wires it into `build:css`. Phase 13 cross-phase verification re-asserts. |
| O5 | Breakpoints can't be used inside `@media` / `@container` conditions; tokens are JS/build/runtime reference only. | A | Principle **P5**; Phase 7 task 3 splits output into Block A (custom properties) / Block B (`@media` literals) / Block C (`@container` literals). |
| O6 | `@custom-media` should be optional / build-time only (limited availability). | A | Principle **P5** point 3; Phase 7 task 4 explicitly forbids `@custom-media` in production output, allowing only an optional `*.custom-media.css` artifact behind a build flag. |
| O7 | Verification should include browser-level checks via the existing Playwright capture path. | A | Principle **P4.4**; Phase 13 task 3 calls out the `CLAUDE.md` Playwright workflow and verifies populated `<code data-generated-css>` plus `:root` resolution under all theme modes. |

## Phase 4 — Foundation Primitives

### D1 Dimension

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| D1.1 | Generate spacing modes too (`data-cem-spacing="dense\|sparse"`). | A | Phase 5 task 2 (metadata blocks include spacing modes) + task 3 (emit `:root[data-cem-spacing="dense"]` / `…="sparse"` overrides). Verified live in spec at `cem-dimension.md` lines 432, 457. |
| D1.2 | Do NOT generate D2 coupling tokens from D1. | A | Phase 5 task 4 explicitly forbids emitting `--cem-coupling-*` from D1. |
| D1.3 | Fix exact token names (`--cem-layout-stack-gap`, `-cluster-gap`, `-gutter`/`-wide`/`-max`); `--cem-layout-inline-*` are deprecated. | A | Phase 5 preface and task 1 list the corrected names; manifest tier marks `--cem-layout-inline-*` as deprecated (Principle P3). Verified against `cem-dimension.md` lines 354/367/383/386/389/370–372. |
| D1.4 | Acceptance criterion: `gap = max(D1 gap, D2 guard)` between interactive affordances. | A | Phase 5 task 5 — documented as component-author responsibility in the manifest's `notes` column (generator does not enforce). |
| D1.5 | Validate reading rhythm with typography, not in isolation. | A | Phase 5 task 6 defers to D6 cross-check; Phase 12 task 6 lists D1↔D6 reading-rhythm vs line-height as a cross-spec check; Phase 13 task 7 enforces it at verification time. |

### D7 Timing

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| D7.1 | Add reduced-motion acceptance — shorten durations, preserve ordering. | A | Phase 6 task 4; Phase 13 task 6 (reduced-motion suite). |
| D7.2 | `highlighted` aliases `smooth` and does not satisfy "visibly more pronounced". | A+R | Phase 6 task 6 blocks closure on **R-D7-1**; R&D table records the choice (give `highlighted` a distinct curve OR mark it adapter-only). |
| D7.3 | Treat springs as optional reserved names — do not generate without real value encoding. | A+R | Phase 6 task 5 (only emit when metadata supplies real values); R&D **R-D7-2** demands canonical encoding (stiffness/damping/mass) or removal. |

### D1x Breakpoints

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| D1x.1 | Split output into custom properties / literal `@media` / optional `@custom-media`. | A | Principle **P5**; Phase 7 task 3 (Blocks A/B/C) + task 4 (no `@custom-media` in production). |
| D1x.2 | Material window-class thresholds 600 / 840 / 1200 / 1600; height 480 / 900. | A+R | Phase 7 task 2 confirms; R&D **R-D1x-1** captures the canonical-bounds decision in case spec deviates. |
| D1x.3 | Preserve "not device type" rule — no `isTablet` semantics. | A | Phase 7 task 6 — manifest notes column. |
| D1x.4 | Keep epsilon adapter-specific (CSS `0.01px`, MUI `0.05px`). | A | Phase 7 task 5 emits both `--cem-bp-epsilon-css` and `--cem-bp-epsilon-mui`. |
| D1x.5 | Container queries need `container-type` / `container` ancestor; tokens alone are insufficient. | A+R | Phase 7 task 3 Block C is gated on consumer providing containment (documented, not enforced); R&D **R-D1x-2** decides whether CEM ships a wrapper component. |

## Phase 5 — Geometry and Structure

### D3 Shape

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| D3.1 | Move D3 after D2, or provide robust `--cem-bend-round` fallback. | A | Phase order: D2 (Phase 8) → D3 (Phase 9). Phase 9 task 3 keeps the calc fallback `var(--cem-shape-height, var(--cem-control-height))` and adds a manifest-supplied sane constant if D2 is absent. |
| D3.2 | Generate `--cem-bend` and likely `--cem-action-border-radius`. | R | R&D **R-D3-1** (general `--cem-bend`) and **R-D3-2** (action border-radius ownership) — both block Phase 9 because spec is silent. |
| D3.3 | Treat `data-cem-shape="…"` as optional brand policy, not required selector. | A | Phase 9 task 3 marks the brand-mode override as **optional brand policy** in the manifest; not emitted as required. |
| D3.4 | Keep adapter-only aliases (`--cem-bend-xs`) out of product-facing requirements. | A | Principle **P3** (adapter-only tier); Phase 9 task 4 emits behind opt-in flag only. |
| D3.5 | Validation: focus-ring clip, forced colors, high zoom, round-end across density modes, RTL logical corners. | A | Phase 9 task 5 enumerates all five validations; deferred to Phase 13 browser-level suite. |

### D5 Stroke

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| D5.1 | Clarify D5↔D0 ownership of `--cem-zebra-strip-size`. | A+R | Phase 10 task 1 makes R&D **R-D5-1** a hard precondition; manifest reflects outcome. |
| D5.2 | Emit `--cem-ring-zebra-{3,4}` with forced-colors outline fallback. | A | Phase 10 task 4 — explicit "each accompanied by a `forced-colors: active` outline fallback". Reinforced by Principle P4. |
| D5.3 | WCAG focus checks — external `box-shadow` not always counted as component visual presentation. | A | Phase 10 task 7 records the caveat in spec prose; Phase 13 task 5 covers WCAG regression. |
| D5.4 | Preserve no-layout-shift rule (no border-box mutation). | A | Phase 10 task 6 — spec prose mandates `outline` / `box-shadow` / pseudo-elements only. |
| D5.5 | D5 depends on D2 guard math: `max(4 * --cem-zebra-strip-size, --cem-stroke-indicator-offset + --cem-stroke-focus)`. | A | Phase 10 task 5 — formula documented in D5 spec, mirrored as a manifest note in D2; Phase 13 task 7 cross-spec assert "D2 guard ≥ D5 worst-case indicator outset". |

### D4 Layering

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| D4.1 | D4 underspecified as CSS output. | R | R&D **R-D4-1** blocks Phase 11. |
| D4.2 | Avoid `--cem-elevation-*` as `z-index`. | A | Phase 11 task 5 — explicitly forbidden; manifest enforces with a generator unit test. |
| D4.3 | Decide: semantic aliases vs adapter hooks per channel. | R | R&D **R-D4-1** captures exactly this binary. Phase 11 cannot proceed without it. |
| D4.4 | Acceptance criterion: each rung perceivable on ≥1 channel (≥2 in dense UIs). | A+R | Phase 11 task 6 records the rule; R&D **R-D4-2** captures the formalization-and-verification mechanism. |
| D4.5 | Forced-colors validation where contour/spacing carry tier meaning. | A | Phase 11 task 7; Phase 13 task 4 forced-colors suite. |

## Phase 6 — Density and Coupling

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| C.1 | Move D2 before D3 and D5. | A | Phase order revised: D2 = Phase 8, D3 = Phase 9, D5 = Phase 10. |
| C.2 | Preserve D2 invariants — `zone-min` / `guard-min` mode-invariant; modes adjust geometry and halo only. | A | Phase 8 task 1 (manifest marks invariants); task 3 limits `data-cem-coupling="…"` overrides to **visual geometry and halo only**. |
| C.3 | Accessibility baseline — WCAG 2.2 AA target size 24×24 CSS px; Android 48dp+8dp recommendation; CEM defaults align with stricter. | A | Phase 8 task 4 records the baseline in the manifest. |
| C.4 | Token generation alone does not enforce coupling; components still need `min-block-size`, halo wrappers, gap formulas. | A | Phase 8 task 5 — manifest notes column documents the gap. |
| C.5 | Add proof surfaces (form trio, nav-list trailing actions, data-table row+selection). | A | Phase 8 task 6 — added as spec-prose proof surfaces, not generator output. |

## Phase 7 — Typography and Voice

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| T.1 | D6 not standalone — depends on D1 / D2 / D5. | A | Phase 12 preface acknowledges; task 6 enumerates cross-checks; Phase 13 task 7 verifies. |
| T.2 | Missing token groups: feature policies, reading-measure-max, paragraph-gap, text-transform roles, dark/contrast ink projections. | A+R | Phase 12 task 3 emits each missing group; R&D **R-D6-1** (canonical names) and **R-D6-2** (dark/contrast ink formula) block emission until canonical design records them. |
| T.3 | Semantic role output incomplete — data needs `font-variant-numeric`, script ligatures, initialism/iconized `text-transform`, reading measure+gap. | A | Phase 12 task 3 final bullet enforces role-specific properties. |
| T.4 | Voice tokens are CSS-exported data, not behavior; screen readers ignore them. | A | Phase 12 task 4 records the contract in the manifest. |
| T.5 | Metadata extraction must preserve quoted font-family stacks and comma-separated values. | A | Phase 12 task 3 first bullet; explicit fixture-test requirement. |
| T.6 | i18n acceptance — broad Unicode fallback, language coverage. | A | Phase 12 task 5 — fixture spec. |

## Build and Generator Notes

| # | Critique bullet | Action | Where in `docs/todo.md` |
|---|---|---|---|
| B.1 | Each new generator must populate exactly one `<code data-generated-css>`. | A | Principle **P6**; Phase 4 task 6 (investigate / fix duplicate output); Phase 13 verification reasserts. |
| B.2 | `cem-breakpoints.html` is a stub — replace before counting coverage. | A | Phase 7 task 3 — replaces the stub. Token Summary keeps D1x at "Generated 0" until Phase 7 lands. |
| B.3 | Use existing h6+table XPath convention or build a shared metadata parser. | A | Principle **P2** mandates the h6+table convention for every new spec. No shared parser is built — the existing pattern is canonical. Phase 4 task 2 documents the convention in `index.md`. |
| B.4 | Add CSS validity checks after capture. | A | Principle **P4.3**; Phase 4 task 4 builds the validator. |
| B.5 | Duplicate dist files (`cem-colors.css` + `cem-colors-1.css`) — define expectation or cleanup. | A | Phase 4 task 6 — explicit investigate-and-fix. Confirmed locally that both files are emitted from a single `data-generated-css` block, so the bug is in `capture-xpath-text.mjs` (or the generator template). |

## External Details Checked

The critique cites authoritative sources (MDN, W3C, Material, MUI, Android). All have been folded into the principles
and phase tasks above. No change to that section is needed in `docs/todo.md` itself, but the citations are honored:

- **MDN custom properties + `@custom-media`** → Principle P5.
- **MDN `@container` containment requirement** → Phase 7 task 3 Block C; R&D R-D1x-2.
- **Android Window Size Classes / MUI breakpoints / step=5** → Phase 7 task 5 (epsilon variants); R-D1x-1.
- **WCAG 2.4.11 (focus not hidden) / Focus Appearance / 2.5.8 (target size)** → Phase 10 task 7; Phase 8 task 4; Phase 13 task 5.
- **Android touch-target 48dp + 8dp** → Phase 8 task 4.

## Recommended Revised Phase Order — adopted

The critique's recommended ordering is implemented verbatim:

| Critique step | New phase in `docs/todo.md` |
|---|---|
| 1. Metadata schema and token manifest format. | **Phase 4** |
| 2. D1 dimension base + spacing modes. | **Phase 5** |
| 3. D7 timing. | **Phase 6** |
| 4. D1x breakpoints with split CSS variable / query output. | **Phase 7** |
| 5. D2 coupling. | **Phase 8** |
| 6. D3 shape. | **Phase 9** |
| 7. D5 stroke. | **Phase 10** |
| 8. D4 layering. | **Phase 11** |
| 9. D6 typography and voice. | **Phase 12** |
| 10. Cross-phase manifest, CSS validity, browser, forced-colors, accessibility verification. | **Phase 13** |

---

## Summary

- **Adopted**: every actionable bullet from the critique, plus 7 new transformation principles (P1–P7) that codify the
  contract going forward.
- **R&D / blockers surfaced**: 12 entries (R-Schema-1, R-D7-1/2, R-D1x-1/2, R-D3-1/2, R-D5-1, R-D4-1/2, R-D6-1/2). Each
  is gated on canonical design (`packages/cem-theme/src/lib/tokens/*.md`) recording the decision before its phase can
  start. Generators may not invent tokens.
- **Pipeline cleanup**: duplicate-output bug (`cem-colors.css` + `cem-colors-1.css`) and stub `cem-breakpoints.html`
  both have explicit fix tasks (Phase 4 task 6, Phase 7 task 3).
- **Verification overhaul**: Principle P4 plus Phase 13 replace "grep the spec" with manifest coverage + CSS validity +
  Playwright browser checks + forced-colors + accessibility regressions.
