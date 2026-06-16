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
      HTML/SVG namespace identity selects the HTML adapter when no content type or schema is present. XSLT namespace
      identity (`http://www.w3.org/1999/XSL/Transform`) selects the legacy custom-element XSLT compatibility adapter
      when no content type or schema is present, while explicit content type remains authoritative. Unsupported input
      identities now emit deterministic lifecycle diagnostics with the declared content type, schema, and/or namespace
      while preserving the fallback input format.
      CEM/HTML target export is registry-owned for `--to-content-type application/cem+xml`,
      `--to-schema https://cem.dev/ns/core/1`, `--to-content-type text/html`,
      `--to-content-type application/xhtml+xml`, XML target export is registry-owned for
      `--to-content-type application/xml` / `text/xml`, plus namespace-only CEM core and HTML/SVG targets; unsupported target
      identities now emit a deterministic lifecycle
      diagnostic with the declared content type, schema, and/or namespace while preserving the requested fallback output projection.
      Keep this item open until broader target adapters beyond current CEM, HTML, and XML output surfaces are
      registry-owned too.
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
- [ ] **Wishlist (future — NOT in the immediate release timeline):** expanded runtime support for the
      `cem-ml transform` data + template -> document command. Design homes:
      [`cem-ml-cli-contract.md`](cem-ml-cli-contract.md#planned-option-behavior) and
      [`cem-ml-cli-plan.md`](cem-ml-cli-plan.md#phase-6---command-behavior). The CEM-ML graph config parser/lowering
      boundary now exists for nested `run` / `import` / `join` / `transform` / `export` nodes, explicit collect joins,
      source-binding group-by joins, same-binding match-by joins, positional zip joins, and cross-input references
      through `@input` and `@with:*`, and the engine graph request/response boundary now has a first in-memory
      CEM-native execution slice for loaded programmatic requests.
      Template identity classification now supports both
      XSLT and CEM-native templates through
      `TransformTemplateAdapterRegistry`; CEM-native templates are insulated as registered template content-type/schema
      adapters instead of being treated as the base CEM-ML language. `EngineContext` carries built-in adapters and can be
      extended by hosts with newer CEM-native template iterations at runtime. The runtime API now records the first
      execution contract: CEM-QL-fragment phase, one-to-one cardinality, reject duplicate destinations, fail-fast
      execution, content-primary output, implicit template entrypoint, future params, and stable diagnostic origins for
      config/import/template/export phases. `TransformTemplateAdapter` also has the future compile/render plugin
      boundary with opaque compiled artifacts and primary/secondary data artifacts; static built-in adapters still
      return deterministic adapter-not-implemented errors unless an executable adapter is registered. Executable
      template adapters take precedence over selector-only adapters for the same identity; multiple executable matches
      remain ambiguous. Compile requests carry declared data binding names so graph secondary inputs are known to
      template compilers. `cem_ml_transform_cem_ql` now provides the first executable CEM-native adapter crate, above both
      `cem_ml` and `cem_ql`, and compiles/renders CEM-ML fragments through `cem_ql::render` while carrying the compiled
      payload in-process on the adapter artifact. `RealCemMlEngine::transform` now runs the minimal one-to-one
      programmatic engine path when a host registers an executable adapter: data is loaded through lifecycle, parsed to
      DOM JSON, compiled/rendered through the selected adapter, and returned as content-primary output. The CLI host
      context now registers the CEM-QL executable adapter and dispatches the one-to-one CEM-native path. Transform CLI
      primary output writes to `--out` or stdout by default; warnings/diagnostics write to stderr unless a
      `--report-json`/`--report-md` destination is provided. Runtime preflight validation now rejects deferred
      first-slice features and bad programmatic graph refs/destinations before execution. `RealCemMlEngine::transform_graph`
      now imports in-memory graph inputs, executes one-to-one CEM-native stages after their primary and secondary
      artifacts are available, and returns response export artifacts. The CLI host now lowers CEM-ML transform config
      into graph requests, resolves relative import/template/export paths against the config document path, and writes
      graph export destinations through the resolver layer. The CLI host also expands local filesystem and
      resolver-backed import globs with exactly one `*` in the file name and an optional single `**` directory segment
      for recursive descent, derives `{src}`, `{path}`, `{dir}`, `{file}`, `{stem}`, `{ext}`, and `{index}` bindings,
      preserves those bindings through one-to-one transform branches, and applies them to export `@out` templates.
      Resolver-backed glob expansion requires an explicit list-capable resolver, sorts by resolved URI, and enforces a
      deterministic max-entry guard.
      Explicit `join @mode="collect"` graph nodes aggregate all artifacts from their primary input into one collection
      artifact and expose a downstream `{count}` binding.
      Source-binding `join @mode="group-by" @by="NAME"` graph nodes aggregate one collection artifact per distinct
      binding value and expose downstream `{key}`, `{count}`, and `{NAME}` bindings.
      Same-binding `join @mode="match-by" @by="NAME" @with:LABEL="NODE"` graph nodes aggregate one collection artifact
      per primary key, attach same-key named secondary artifacts, and expose downstream `{key}`, `{count}`, and `{NAME}`
      bindings. Missing secondary matches produce empty named secondary collections rather than fatal errors.
      Positional `join @mode="zip" @with:LABEL="NODE"` graph nodes aggregate one collection artifact per index across
      primary and named secondary artifact streams, expose downstream `{index}` and `{count}` bindings, and fail when any
      input stream has a different count.
      Transform graph reports now include `reportAst.transformGraph` export metadata for resolved export IDs,
      destinations, content identities, output kinds, source-map presence, output-span counts, and sidecar refs. Graph
      exports with artifact source maps write `{destination}.map` sidecars through the output resolver.
      Keep the checked-in CEM-native CLI transform-config schema
      (`packages/cem_ml/schema/cli/transform-config.md`,
      `https://cem.dev/ns/cli/transform-config/1`) separate from CEM core document schemas and template schemas.
      Runtime order: start with pure CEM-QL evaluation plus CEM-ML fragments
      with embedded CEM-QL and one implicit entrypoint; then add local filename-glob input enumeration plus named
      path-template expansion for graph configs; then add native named templates/modules, explicit entrypoints, params,
      imports/includes, visibility, caching, and recursion/cycle limits; then expand XSLT parity on that native
      substrate. Next runtime implementation work is CEM-native template semantics before XSLT parity.
      Native template module semantics should start with an adapter-owned module contract rather than changing the base
      CEM-ML AST: implicit entrypoint means module default render; explicit entrypoints select public exported templates;
      declarations are private by default; params are immutable with template defaults and fatal unknown names unless an
      adapter declares an extension bucket; portable data bindings are primary `input`, named secondary graph inputs, and
      params; imports come before includes and load isolated modules through the `template` resolver; includes remain
      reserved; module cache keys include adapter ID, resolved URI, identity, content hash, selected entrypoint, execution
      policy, and dependency graph hash; import cycles are fatal compile diagnostics; recursive calls require explicit
      runtime/scheduler limits; multiple outputs stay modeled by graph branches and exports.
      The native template module API shape now exists on `TransformTemplateCompileRequest` through
      `TransformTemplateModuleOptions`, import declarations, entrypoint declarations with explicit visibility, param
      declarations, module limits, cache keys, non-executing call-site records, and reserved diagnostics for
      private/missing entrypoints, unknown calls, unknown params, import cycles, recursion limits, and reserved includes.
      `TransformTemplateModulePreflight` now carries
      resolver-backed import reads, resolved module bytes/identity/content hashes, dependency graph cache-key input, and
      compile-time diagnostics for duplicate aliases, reserved includes, and direct self-import cycles. The CEM-QL
      executable adapter now compiles preflighted modules into its native payload and exposes import metadata on the
      compiled artifact. It dispatches validated same-module and direct imported module calls during render with the
      current data context; call `@with:*` whole-expression attributes preserve their evaluated CEM-QL item stream for
      the invoked template, while literal and mixed attribute-value-template forms remain string bindings.
      Same-module recursive calls are bounded by the module recursion limit and report
      `cem.transform_template.recursion_limit` when exceeded. The CEM-native template declaration schema now has its own
      identity
      (`https://cem.dev/ns/template/cem-native/1`) and checked-in artifact
      `packages/cem_ml/schema/template/cem-native-template.md`, covering `module`, `import`, `param`, `template`,
      `body`, and `call` while keeping `include` reserved. The real engine now parses/lowers that schema into
      `TransformTemplateModuleOptions` before adapter compilation while preserving declaration-free CEM fragment
      templates. It validates named entrypoint requests against public declarations, rejects unknown caller params, and
      reports missing required caller params before adapter compilation. It also validates same-module and imported
      public `call` targets. Next implementation boundary is transitive module execution and transitive recursion
      behavior before XSLT parity expansion. The separate adapter crate avoids the dependency cycle where `cem_ql`
      currently depends on `cem_ml`.
- [ ] **Wishlist (future — schema/tooling):** wire CLI config schemas into generated/published artifacts. The JSON
      `RunConfig` config-file surface uses schema identity `https://cem.dev/ns/cli/run-config/1` and has checked-in JSON
      Schema `packages/cem_ml/schema/cli/run-config.schema.json`
      (`https://cem.dev/schema/cli/run-config.schema.json`) for CI/editor validation. The CEM-ML transform graph config
      uses schema identity `https://cem.dev/ns/cli/transform-config/1` for the CLI config element set (`run`, `import`,
      `join`, `transform`, `export`) and has checked-in schema artifact
      `packages/cem_ml/schema/cli/transform-config.md`; it must not reuse CEM core document schema or CEM-native template
      schema as its validation identity.

## Phase 3.1 — Substrate / Legacy Compatibility Follow-Up

Design homes:
[`custom-element-template-migration-options.md`](custom-element-template-migration-options.md) and
[`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md).

- [x] Legacy DCE `hasBoolAttribute()` boolean-attribute helper — implemented as a compile-time rewrite
      in `cem_ml::legacy_custom_element::emit_call`; expands to the idiomatic HTML boolean attribute
      test `not (attr = "false") and (attr = "" or attr = "attr" or attr = "true")`. Allowlist entry
      removed from `legacy-compat-manifest.json`.
- [x] Tier 3 XSLT remains an explicit handoff/deferred scope outside the bounded compatibility profile: unresolved
      dynamic construction names outside the scalar AVT subset, EXSLT `func:function`, and `msxsl:script` are
      non-transpilable in the legacy custom-element bridge and emit deterministic conversion diagnostics instead of
      lowering silently.

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
