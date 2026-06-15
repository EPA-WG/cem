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
      Built-in input content-type dispatch is now registry-backed across parser-backed commands. CEM core schema or
      namespace identity (`https://cem.dev/ns/core/1`) selects the CEM adapter when no content type is present, and
      HTML/SVG namespace identity selects the HTML adapter when no content type or schema is present, while
      explicit content type remains authoritative. Unsupported input identities now emit deterministic lifecycle
      diagnostics with the declared content type, schema, and/or namespace while preserving the fallback input format.
      CEM/HTML target export is registry-owned for `--to-content-type application/cem+xml`,
      `--to-schema https://cem.dev/ns/core/1`, `--to-content-type text/html`,
      `--to-content-type application/xhtml+xml`, XML target export is registry-owned for
      `--to-content-type application/xml` / `text/xml`, plus namespace-only CEM core and HTML/SVG targets; unsupported target
      identities now emit a deterministic lifecycle
      diagnostic with the declared content type, schema, and/or namespace while preserving the requested fallback output projection.
      Keep this item open until remaining non-CEM schema/namespace-specific selection and remaining non-CEM target
      export adapters are registry-owned too.
- [x] **Immediate goal: root-scope run configuration for lib + WASM + CLI.** Design homes:
      [`cem-ml-cli-contract.md`](cem-ml-cli-contract.md#major-requirement-root-scope-and-run-configuration) and
      [`cem-ml-cli-plan.md`](cem-ml-cli-plan.md#run-configuration-shape). Add a serializable `RunConfig` with input and
      output spec arrays, root `ScopeConfig` per input/output, module-map/resolver identity, default and named namespace
      bindings, schema/version pins, base URI, scope policy, and resource budgets. Expose the same shape through Rust
      lib APIs, WASM APIs, CLI config files, and repeatable CSV CLI records. Keep scheduler/thread-pool configuration
      at run level so build/CI can validate or transform multiple documents in one shared runtime while preserving
      per-document root-scope diagnostics and budget accounting. The shared `cem_ml::run_config` model now exists,
      `cem_ml` owns JSON config parsing by declared config content type plus CSV input/output spec parsing, WASM exposes
      helpers over the same library parser, and CLI `--config`, `--config-content-type`, repeatable `--input-spec`, and
      repeatable `--output-spec` map into engine input identities and the first conversion output target/destination.
      Positional inputs now receive the same normalized content-type identity path, including extension inference when
      no explicit `--content-type` is supplied; command-level `--default-namespace` and repeatable
      `--namespace PREFIX=URI`, `--module-map`, repeatable `--version-pin NAME=CONSTRAINT`, `--scope-policy`, and
      repeatable `--scope-budget NAME=VALUE` flags now flow into positional input root scopes alongside `--schema`,
      `--content-type`, and `--base-uri`, using the same run-config default-scope validation as config/spec
      root-scope fields. Convert target identity now preserves command-level default/named namespace bindings and
      output-spec namespace-only identities when selecting export adapters. Config diagnostics now fail before
      document parsing for malformed JSON,
      unsupported config content types, duplicate input URIs, and unknown output input references, and report-capable
      CLI commands represent those diagnostics in generated JSON/Markdown reports. The `--observe-events` path now uses
      the same configured input list and lifecycle dispatch as parser-backed commands, including `--input-spec` and
      `--config` inputs. Config-file convert execution now fans out multiple `outputs[]` records, using `inputRef` or
      the sole configured input for each output. Normalized `RunConfig.scheduler` now flows into engine execution
      context, the trace worker policy is derived from that scheduler config, and validate/check reports now embed a
      shared run-level scheduler trace with per-document scope IDs. Validate/check now execute lifecycle loading and
      parser-backed validation through scheduler-dispatched per-document tasks, so the trace reflects actual document
      work instead of report projection only. Convert can now write explicit side reports from scheduler traces returned
      by engine convert execution while preserving content-primary stdout/`--out` behavior. Full input and output
      root-scope config now reaches engine requests. Recognized root-scope scheduler policy and budget fields now derive
      the per-scope worker policy for scheduled validate/check, trace, and convert execution; `parseMs` enforces a
      parser-backed pipeline wall-clock budget; `validateMs` and `checkMs` enforce scheduled per-input document work
      budgets; `convertMs` enforces input/output-scope convert work budgets; `traceMs`, `inspectMs`, `benchMs`,
      `fixtureValidateMs`, `fixtureRoundtripMs`, and `observeMs` enforce trace, inspect, benchmark, fixture, and
      observability workflow budgets; and
      effective `baseUri` values now project relative report input and diagnostic URIs.
      Root-scope default and named namespace bindings now seed schema validation's document-root
      namespace context, and recognized CEM-ML root-scope version pins now resolve against the embedded document-format
      version. Input root-scope module maps now provide the resolver base for relative schema `src` identities, load
      local JSON alias maps from paths and local `file://` URIs for schema-source specifier resolution, and normalize
      relative module-map paths against the config document path, including local `file://` config document bases.
      Config-file output destinations now normalize relative paths against the config document path, including local
      `file://` config document bases. Configured, positional, and
      fixture-materialized input reads resolve local `file://` URIs, and primary output, per-output conversion,
      side-report, and observability event writes resolve local `file://` destinations to filesystem paths. Run-config
      normalization now validates
      root-scope module-map, namespace, and version-pin option shape before
      document parsing, while unreadable or malformed module maps, unknown future budget keys, and unsupported
      version-pin targets emit deterministic execution diagnostics instead of being silently ignored. Remote/custom
      module-map URI values now emit an explicit unsupported-resolver diagnostic; config document reads and configured,
      positional, and fixture-materialized input reads now reject remote/custom URI values when no resolver is
      registered; and CLI file-write paths now reject remote/custom URI destinations instead of treating them as local
      paths. The resolver implementation is documented in
      [`cem-ml-cli-contract.md` §Resolver Semantics](cem-ml-cli-contract.md#resolver-semantics) and
      [`cem-ml-cli-plan.md` §Run Configuration Shape](cem-ml-cli-plan.md#run-configuration-shape). The shared
      `cem_ml::resolver` request/response types, local path/local `file://` handling, `ResourceResolver`, and
      `ResolverRegistry` now exist; `EngineContext` carries the registry; CLI/run-config/real-engine local URI parsing
      uses that shared code; registered resolvers can now read custom config documents, configured/positional inputs,
      module-map URIs, fixture/benchmark materialized input URIs, and fixture placeholder materialization for
      pre-engine observability/template diagnostics; registered write resolvers can now handle primary output,
      side-report, and observability destinations; WASM hosts can register callback-backed read/write resolvers through
      `onResolveRead` / `onResolveWrite`; and the CLI can opt into local mirror resolver maps with
      `--resolver-read-map`, `--resolver-write-map`, or run-config `resolvers` entries while remaining local-only by
      default.
- [x] **Immediate goal: XSLT 1.0 lifecycle adapter.** Move the existing legacy custom-element XSLT 1.0 lowering
      (`cem_ml::legacy_custom_element`) behind the lifecycle adapter registry instead of the current one-off
      `convert --content-type custom-element-xslt` branch. `cem-ml validate --content-type custom-element-xslt <input>`
      must validate raw legacy XSLT input directly; `cem-ml convert --content-type custom-element-xslt
      --to-content-type application/cem+xml <input>` must load through the same adapter and export canonical CEM-ML.
      The dispatch now lives in `cem_ml::lifecycle::LifecycleRegistry`; broader schema-aware lifecycle selection remains
      tracked by the structural data lifecycle item above.
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
