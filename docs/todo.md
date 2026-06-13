# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).
Each item names the AC reference and design home so the closing change ships with a citation.

## Active — Evolutionary Architecture of the Authoring/Rendering Model

Design home: [`content-type-switch.md`](content-type-switch.md) (BRD). The open questions and implementation gates
for the current architecture have landed; only deferred capability work remains here.

- [ ] **Immediate goal: structural data lifecycle for lib + CLI.** Design homes:
      [`cem-ml-cli-contract.md`](cem-ml-cli-contract.md), [`cem-ml-cli-plan.md`](cem-ml-cli-plan.md), and
      [`../roadmap.md` §Phase 2](../roadmap.md#phase-2---schema-defined-parser-and-document-runtime). Promote format
      identity from report metadata to execution input: content type + schema/namespace identity select the adapter
      that validates bytes, loads normalized events / CEM AST, and exports to the requested target identity. Keep
      `--from-format` / `--to-format` as compatibility aliases while adding explicit input/output content-type and
      schema selection.
- [ ] **Immediate goal: XSLT 1.0 lifecycle adapter.** Move the existing legacy custom-element XSLT 1.0 lowering
      (`cem_ml::legacy_custom_element`) behind the lifecycle adapter registry instead of the current one-off
      `convert --content-type custom-element-xslt` branch. `cem-ml validate --content-type custom-element-xslt <input>`
      must validate raw legacy XSLT input directly; `cem-ml convert --content-type custom-element-xslt
      --to-content-type application/cem+xml <input>` must load through the same adapter and export canonical CEM-ML.
      First implementation slice is in place through the shared CLI/lib lifecycle load path for `validate`, `check`, and
      `convert`; keep this item open until the adapter registry abstraction owns that dispatch.
- [ ] **Wishlist (future — NOT in the immediate release timeline):** engine XSLT 3.0/4.0 execution
      behind G-NVDL-FULL (AC-P-6.9). The architecture keeps the capability-gated seam — XSLT is a
      peer language behind explicit dispatch, not the primary model or a browser-native dependency —
      so the engine can add XSLT 3/4 later without breaking content. Building the XSLT 3/4 engine
      remains out of scope for the current release.

## Phase 3.1 — Substrate / Legacy Compatibility Follow-Up

Design homes:
[`custom-element-template-migration-options.md`](custom-element-template-migration-options.md) and
[`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md).

- [x] Legacy DCE `hasBoolAttribute()` boolean-attribute helper — implemented as a compile-time rewrite
      in `cem_ml::legacy_custom_element::emit_call`; expands to the idiomatic HTML boolean attribute
      test `not (attr = "false") and (attr = "" or attr = "attr" or attr = "true")`. Allowlist entry
      removed from `legacy-compat-manifest.json`.
- [ ] Tier 3 XSLT remains an explicit handoff/deferred scope outside the bounded compatibility profile: unresolved
      dynamic construction names outside the scalar AVT subset, EXSLT `func:function`, and `msxsl:script` are
      non-transpilable in the legacy custom-element bridge.

## Phase 4 — CEM Component Set

Roadmap: [`../roadmap.md` §Phase 4](../roadmap.md). Components come before the Figma UI Kit so the design library maps
to proven web component names, states, attributes, and accessibility behavior instead of inventing a parallel model.

- [x] Complete the custom-element XSLT parity scope before expanding the component catalog. The engine now has the first
      stylesheet-compat slices for `xsl:stylesheet`, root/named `xsl:template`, `xsl:call-template`, params, bounded
      `xsl:apply-templates` over inline `exsl:node-set($var)/*` variables, sample-style source child/attribute/text
      traversal, absolute/descendant selectors, namespace wildcards, indexed child steps, parent-relative paths, simple
      predicates including scalar equality checks, current attribute/child `for-each` unions, preceding-sibling
      traversal, variable-rooted current-node paths, static EXSLT node-set variable aliases, filtered static node-set
      attribute extraction, static `if`/`when` folding for known current-node tests, default template fallbacks, basic
      template priority, scalar and node-set template params, multi-key `xsl:sort`, literal `count`/`sum` over
      supported node selections, bounded current-node copy/copy-of/attribute construction, scalar-AVT `xsl:element`
      construction, `hasBoolAttribute()` boolean-attribute rewriting, and recursion safety. The copied material
      component templates now convert without unexpected diagnostics in both the Rust engine manifest gate and the
      browser/WASM custom-element gate. Future XPath/function expansion is sample-driven follow-up, not a blocker for
      Phase 4 catalog expansion. Track the inventory with `yarn nx run @epa-wg/custom-element:xslt:inventory`; track the
      remaining bounded implementation questions in [`custom-element-xslt-parity-decision.md`](custom-element-xslt-parity-decision.md).
- [ ] Define the Phase 4 component MVP list and state matrix across actions, inputs, navigation, content, feedback,
      and the first app workflow surfaces. Use Angular Material only as a coverage and ergonomics benchmark, not as a
      required implementation dependency.
- [ ] Expand `@epa-wg/cem-components` from the current primitives into the practical Material-style surface:
      action/icon-button/menu-item, text field/textarea/select/checkbox/radio/switch, app bar/nav/tabs, card/list/table,
      chip/badge/avatar/media preview, dialog/sheet/toast/progress/skeleton/alert.
- [ ] Add component docs and examples for semantics, token usage, states, and accessibility notes. The exit gate is that
      the future CEM site and Figma site demo can be built from this component set without one-off UI controls.

## Phase 5 — Figma UI Kit Token Validation (`examples/figma`)

Roadmap: [`../roadmap.md` §Phase 5](../roadmap.md). Token export contract:
[`../packages/cem-theme/docs/token-export.md`](../packages/cem-theme/docs/token-export.md). Figma library workflow:
[`../packages/cem-theme/docs/token-figma.md`](../packages/cem-theme/docs/token-figma.md). These items moved from
Phase 1 because the validation is only meaningful against a populated Figma UI Kit. This phase starts after the
Phase 4 component set has stable names, variants, and state semantics.

- [ ] Validate native Figma library variables against the generated `figma/cem-*.tokens.json` files for every mode.
      Surface the validation in `nx run @epa-wg/cem-theme:test:figma` (new target) or extend the existing
      token-platform report. Block release when a mode disagrees with the canonical spine.
- [ ] Extend the token-change smoke test with the Figma propagation leg: change one canonical token, refresh the Figma
      mode files, and assert the UI Kit variables reflect the change without manual rework. Track gaps in
      `token-pipeline-smoke.md`. The non-Figma leg of the same smoke test lives under Phase 8.

## Phase 8 — Native Platform Packages (`@epa-wg/cem-theme` native outputs)

Roadmap: [`../roadmap.md` §Phase 8](../roadmap.md). Token export contract:
[`../packages/cem-theme/docs/token-export.md`](../packages/cem-theme/docs/token-export.md). These items moved from
Phase 1 because they validate Phase 8 native artifacts (iOS Swift, Android Kotlin/Compose) and are gated by the
available toolchains, not the Phase 1 token-spine work that already shipped.

- [ ] Compile generated Swift (`packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`) with a supported Xcode
      toolchain. Add the compile step as a release gate; fail loudly when symbols drift.
- [ ] Compile generated Kotlin/Compose (`packages/cem-theme/dist/lib/token-platforms/android/`) with the supported
      Gradle toolchain. Add the compile step as a release gate.
- [ ] Wire a token-change smoke test for the non-Figma propagation path: change one canonical token, regenerate CSS,
      JSON, Swift, and Android outputs, and assert every artifact moves coherently. Track gaps in
      `token-pipeline-smoke.md`. (The Figma propagation leg of the same smoke test lives in Phase 5.)
