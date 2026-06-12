# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).
Each item names the AC reference and design home so the closing change ships with a citation.

## Active — Evolutionary Architecture of the Authoring/Rendering Model

Design home: [`content-type-switch.md`](content-type-switch.md) (BRD). The open questions and implementation gates
for the current architecture have landed; only deferred capability work remains here.

- [ ] **Wishlist (future — NOT in the immediate release timeline):** engine XSLT 3.0/4.0 execution
      behind G-NVDL-FULL (AC-P-6.9). The architecture keeps the capability-gated seam — XSLT is a
      peer language behind explicit dispatch, not the primary model or a browser-native dependency —
      so the engine can add XSLT 3/4 later without breaking content. Building the XSLT 3/4 engine
      remains out of scope for the current release.

## Phase 3.1 — Substrate / Legacy Compatibility Follow-Up

Design homes:
[`custom-element-template-migration-options.md`](custom-element-template-migration-options.md) and
[`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md).

- [ ] Legacy DCE `hasBoolAttribute()` boolean-attribute helper is not reproduced yet. It is currently an
      allowlisted compatibility gap for legacy material `input`/`action` templates.
- [ ] Tier 3 standalone XSLT stylesheets remain an explicit handoff/deferred scope:
      push-model `apply-templates`/`call-template`/`sort`, EXSLT `func:function`, and `msxsl:script`
      are non-transpilable in the bounded legacy custom-element bridge.

## Phase 3.6 — `@epa-wg/custom-element` Monorepo Adoption

Roadmap: [`../roadmap.md` §Phase 3.6](../roadmap.md).

- [x] Publish-readiness pass for the next major: changelog, migration guide from external POC package to workspace
      package, bridge-window support matrix, breaking-change list, npm package contents check, and rollback plan for
      consumers that still depend on the old XSLT-only surface. Analysis landed in
      [`release-readiness-0.1.0.md`](release-readiness-0.1.0.md): target `0.1.0`, bridge policy
      deprecate-now / remove-next-major (FF-5-gated), changelog summary, breaking-change list, support matrix,
      rollback plan, and the npm-contents check. Local publish-readiness is complete for the nx `cem` group:
      manifests are at `0.1.0`, `CHANGELOG.md` is curated, README deprecation notices are present, and all four
      publish roots pack cleanly. Publish/tag/GitHub-release actions are intentionally skipped for now and will be
      run later by a maintainer, including the separate `@epa-wg/custom-element` `0.1.0` release pipeline (its local
      source/dist manifests still report `0.0.39`).

## Phase 4 — CEM Component Set

Roadmap: [`../roadmap.md` §Phase 4](../roadmap.md). Components come before the Figma UI Kit so the design library maps
to proven web component names, states, attributes, and accessibility behavior instead of inventing a parallel model.

- [ ] Complete the custom-element XSLT parity scope before expanding the component catalog. The engine now has the first
      stylesheet-compat slices for `xsl:stylesheet`, root/named `xsl:template`, `xsl:call-template`, params, bounded
      `xsl:apply-templates` over inline `exsl:node-set($var)/*` variables, sample-style source child/attribute/text
      traversal, default template fallbacks, basic template priority, multi-key `xsl:sort`, and recursion safety. Phase
      4 still needs the remaining copied component/sample parity: broader XPath-backed apply-template traversal and
      predicate/function behavior where sample-used. Track the inventory with
      `yarn nx run @epa-wg/custom-element:xslt:inventory`; track the remaining bounded implementation questions in
      [`custom-element-xslt-parity-decision.md`](custom-element-xslt-parity-decision.md).
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
