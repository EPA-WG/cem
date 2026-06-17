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
- [ ] **Wishlist (future — NOT in the immediate release timeline):** engine XSLT 3.0/4.0 execution
      behind G-NVDL-FULL (AC-P-6.9). The architecture keeps the capability-gated seam — XSLT is a
      peer language behind explicit dispatch, not the primary model or a browser-native dependency —
      so the engine can add XSLT 3/4 later without breaking content. Building the XSLT 3/4 engine
      remains out of scope for the current release.
- [ ] **Immediate goal: expanded runtime support for the `cem-ml transform` data + template -> document command.**
      Design homes:
      [`cem-ml-cli-contract.md`](cem-ml-cli-contract.md#planned-option-behavior) and
      [`cem-ml-cli-plan.md`](cem-ml-cli-plan.md#phase-6---command-behavior). The CEM-ML graph config parser/lowering
      is now the active implementation track, not future wishlist. First active slice: preserve runtime provenance
      through graph collection joins so join-produced export artifacts carry the source-map/output-span metadata already
      emitted by upstream transform stages; keep tests separate from `examples/` while covering example-shaped graph
      cases. Multi-input join collections now retain per-item source maps/output spans while keeping aggregate export
      metadata honest when no single source-map stack can represent the whole collection.
      The parser/lowering boundary already exists for nested `run` / `import` / `join` / `transform` / `export` nodes,
      explicit collect joins,
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
      Single-transform reports now include `reportAst.transform` metadata for input, destination, output kind,
      source-map presence, output-span count, and `{destination}.map` sidecar refs when a concrete destination exists.
      Transform graph reports now include `reportAst.transformGraph` export metadata for resolved export IDs, input
      artifact IDs, destinations, content identities, output kinds, source-map presence, output-span counts, and
      sidecar refs. Graph exports with artifact source maps write `{destination}.map` sidecars with
      export/input/destination metadata through the output resolver.
      Keep the checked-in CEM-native CLI transform-config schema
      (`packages/cem_ml/schema/cli/transform-config.md`,
      `https://cem.dev/ns/cli/transform-config/1`) separate from CEM core document schemas and template schemas.
      Runtime order: start with pure CEM-QL evaluation plus CEM-ML fragments
      with embedded CEM-QL and one implicit entrypoint; then add local filename-glob input enumeration plus named
      path-template expansion for graph configs; then add native named templates/modules, explicit entrypoints, params,
      imports/includes, visibility, caching, and recursion/cycle limits; then expand XSLT parity on that native
      substrate. Next runtime implementation work is CEM-native template semantics before XSLT parity.
      Remaining CEM-native parity closure before XSLT: first surface named entrypoints and params in the direct CLI as
      `--template-entrypoint NAME` plus repeatable `--param NAME=VALUE`; then surface the same controls in CEM-ML graph
      config as `transform @entrypoint` plus child `param @name @value` records; then lower those fields into existing
      `TransformRequest` / `TransformGraphStage` entrypoint and params without changing the engine API; then prove
      resolver-backed imported CEM-native modules through direct CLI and graph config, including relative import
      resolution, imported call diagnostics, stdout/default output behavior, report destinations, and source-map sidecars
      for configured exports; then freeze conformance fixtures for implicit/public/private/missing entrypoints,
      defaults/nulls/type coercion, same/imported calls, `@with:*` secondary inputs, nested imports, import cycles/depth,
      and recursion limits. The separate CLI integration suite
      `packages/cem_ml_cli/tests/cem_native_module_conformance.rs` now covers those example-shaped cases without making
      `examples/` files executable test fixtures. `include`, `@default-expr` / `@defaultExpr`, unknown-param extension
      buckets, arbitrary template writes, and XSLT execution stay outside this closure. XSLT parity starts only after
      supported CEM-native
      module semantics are available through the programmatic API, direct CLI, and CEM-ML graph config with stable
      diagnostics/reports and schema docs.
      Native template module semantics should start with an adapter-owned module contract rather than changing the base
      CEM-ML AST: implicit entrypoint means module default render; explicit entrypoints select public exported templates;
      declarations are private by default; params are immutable with template defaults and fatal unknown names unless an
      adapter declares an extension bucket; explicit `null` caller params count as provided and do not fall back to
      defaults; selected-entrypoint local and qualified param names are aliases, and providing both aliases is fatal;
      v1 param `@type` supports `any`, `string`, `boolean`, `number`, `integer`, `array`, `object`, and `json`, with
      omitted `@type` as `any`; typed caller params are validated as JSON shapes before adapter compilation; params are
      non-nullable by default; `@nullable="true"` allows explicit JSON `null` caller values and literal
      `@default="null"`; explicit `null` remains provided for requiredness; `@default` is literal only, with raw strings
      for `any`/`string`, `true`/`false` for `boolean`, and parsed JSON for `number`/`integer`/`array`/`object`/`json`;
      string-valued caller params from CLI/config inputs are normalized at the module contract boundary before adapter
      compile, with nullable literal `null` becoming JSON null, `boolean` accepting `true`/`false`,
      `number`/`integer`/`array`/`object`/`json` parsing JSON, and non-nullable `any`/`string` keeping text such as
      `null` as a string;
      `@default-expr` / `@defaultExpr` is reserved and fatal until expression context, resolver policy, and reporting are
      defined;
      portable data bindings are primary `input`, named secondary graph inputs, and params; the CEM-QL data document also exposes those host bindings as top-level fields while retaining
      `datadom.attributes.*` compatibility; imports come before includes and load isolated modules through the `template` resolver; includes remain
      reserved; module cache keys include adapter ID, resolved URI, identity, content hash, selected entrypoint, execution
      policy, and dependency graph hash; import cycles are fatal compile diagnostics; recursive calls require explicit
      runtime/scheduler limits; multiple outputs stay modeled by graph branches and exports.
      The native template module API shape now exists on `TransformTemplateCompileRequest` through
      `TransformTemplateModuleOptions`, import declarations, entrypoint declarations with explicit visibility, param
      declarations with types/defaults/required flags, module limits, cache keys, non-executing call-site records, and
      reserved diagnostics for private/missing entrypoints, unknown calls, unknown/missing/type-mismatched params, import
      cycles, recursion limits, and reserved includes.
      `TransformTemplateModulePreflight` now carries
      recursive resolver-backed import reads, resolved module bytes/identity/content hashes with `parentUri` for
      non-root import edges, dependency graph cache-key input, and compile-time diagnostics for duplicate aliases per
      importing module, reserved includes, import-depth overflow, and import cycles. The CEM-QL executable adapter now
      compiles preflighted modules into its native payload and exposes import metadata on the compiled artifact. It
      dispatches validated same-module and imported module calls during render with the current data context; when an
      imported module's public entrypoint renders, unqualified calls resolve against that imported module's own named
      templates, and `call @from` resolves against that module's own import aliases rather than the root module. Call
      `@with:*` whole-expression attributes preserve their evaluated CEM-QL item stream for the invoked template, while
      literal and mixed attribute-value-template forms remain string bindings. It also preserves the selected entrypoint,
      caller params, and param declarations in the compiled payload; caller params override declaration defaults, omitted
      defaults are applied during same-module and imported renders, explicit `null` remains bound as a caller value, and
      named entrypoint-local params bind through their local names inside the invoked template. Qualified caller params
      bind equivalently for the selected entrypoint, while duplicate local+qualified aliases are rejected before adapter
      compilation. The
      CEM-QL renderer declares stable primary `input` at compile time and makes primary/secondary
      host bindings available as top-level data-document fields while preserving `datadom.attributes.*` compatibility.
      Same-module recursive calls, including recursive calls inside an imported module, are bounded by
      the module recursion limit and report `cem.transform_template.recursion_limit` when exceeded. The CEM-native
      template declaration schema now has its own
      identity
      (`https://cem.dev/ns/template/cem-native/1`) and checked-in artifact
      `packages/cem_ml/schema/template/cem-native-template.md`, covering `module`, `import`, `param`, `template`,
      `body`, and `call`, including the first `param @type` JSON-shape surface and `param @nullable` nullability flag,
      while keeping `param @default-expr` / `@defaultExpr` and `include` reserved. The real engine now parses/lowers that schema into
      `TransformTemplateModuleOptions` before adapter compilation while preserving declaration-free CEM fragment
      templates. It validates named entrypoint requests against public declarations, rejects unknown caller params,
      validates typed caller/default param values, and reports missing required caller params before adapter compilation.
      It also validates same-module and imported public `call` targets. The first broader XSLT parity follow-up is now
      covered by `packages/cem_ml/tests/xslt_adapter_output_parity.rs`, which proves XSLT 1.0 compatibility lowering
      renders the same light-DOM output as equivalent CEM sources for login/profile/asset-shaped cases while keeping the
      tests separate from `examples/`. Executable XSLT parity now runs through direct `cem-ml transform` and CEM-ML graph
      config by registering the parity adapter from `cem_ml_transform_cem_ql`; CLI integration coverage lives in
      `packages/cem_ml_cli/tests/xslt_parity_transform.rs` and keeps example-shaped cases out of `examples/`.
      XSLT parity currently supports the implicit entrypoint only; CLI `--template-entrypoint`, `--param`, and graph
      `transform @entrypoint` / child `param` remain CEM-native-only surfaces. The separate adapter crate avoids the
      dependency cycle where `cem_ql` currently depends on `cem_ml`. Transform graph runtime phase now lives on each
      `TransformGraphStage`, while duplicate-destination and other graph-wide execution controls stay on
      `TransformGraphRequest`; mixed CEM-native and XSLT stages are covered by CLI integration tests. Quoted XPath
      string literals in lowered XSLT value/param expressions now render as text while scalar variables that represent
      rewritten CEM-QL expressions still splice as expressions.
- [ ] **Wishlist (future — schema/tooling):** wire CLI config schemas into generated/published artifacts. The JSON
      `RunConfig` config-file surface uses schema identity `https://cem.dev/ns/cli/run-config/1` and has checked-in JSON
      Schema `packages/cem_ml/schema/cli/run-config.schema.json`
      (`https://cem.dev/schema/cli/run-config.schema.json`) for CI/editor validation. The CEM-ML transform graph config
      uses schema identity `https://cem.dev/ns/cli/transform-config/1` for the CLI config element set (`run`, `import`,
      `join`, `transform`, `export`) and has checked-in schema artifact
      `packages/cem_ml/schema/cli/transform-config.md`; it must not reuse CEM core document schema or CEM-native template
      schema as its validation identity.

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
