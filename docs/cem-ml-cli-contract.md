# `cem-ml-cli` Feature Summary

This document summarizes planned `cem-ml-cli` features: command capabilities, option
behavior, report fields, diagnostics, fixture workflows, and exit-code policy. It is an
input to implementation planning, not a compatibility contract with removed CLI behavior.

The default parser-backed input surface is canonical CEM-ML (`.cem`) using the
curly-brace syntax in [`cem-ml-syntax.md`](cem-ml-syntax.md). XML and HTML are
secondary parity input surfaces and should remain selectable anywhere parser-backed
input format is exposed.

## Major Requirement: Structural Data Lifecycle

`cem-ml` and `cem-ml-cli` MUST provide one structural data lifecycle over every supported
format:

1. **Validate** input bytes against the declared format identity.
2. **Load** the validated input into the internal CEM document AST / event model.
3. **Export** that internal representation into a declared external format.

The format identity is the pair of:

- **content type** — the wire/container syntax, for example `application/cem+xml`,
  `text/html`, `application/xml`, `application/xslt+xml`, or
  `text/custom-element-xslt`;
- **schema / namespace identity** — the structural vocabulary and validation contract
  active inside that content type.

The generic CEM-ML parser/event/AST pipeline is the internal spine. Namespace-specific
and content-type-specific behavior MUST be supplied by registered plugins/adapters that
can participate in these lifecycle stages:

- `load`: bytes + format identity → normalized events / CEM AST;
- `validate`: normalized events / CEM AST + schema identity → diagnostics/report;
- `export`: CEM AST + target format identity → bytes/projection + source map.

The CLI is a thin orchestration layer over this lifecycle. It MUST NOT grow separate
format-specific validation engines per command. Format-specific behavior belongs in
`cem-ml` plugins/adapters, and CLI flags only select the input and output identities.

## Major Requirement: Root Scope And Run Configuration

The document root is the root scope for the whole AST tree. It MUST use the same
scope model as any internal AST scope: descendants inherit root parameters until an
inner scope explicitly overrides them, and diagnostics/source maps must be able to
report which effective scope parameters were active.

Root-scope input parameters MUST be available through the Rust library API, the WASM
API, and the CLI. The shared model must include at least:

- default content type;
- schema identity and version pins;
- default namespace binding;
- named namespace bindings;
- module map / resolver identity;
- base URI;
- scope policy and resource budget hooks needed by validation, module resolution, and
  plugin/adaptor dispatch.

The input → internal AST → output chain MUST accept output scope options as well.
Output identity is not just an output format enum: outputs can declare content type,
schema identity, namespace bindings, version pins, base URI, and module-map/resolver
identity where the target adapter needs them.

Configuration surfaces:

- Rust library and WASM APIs MUST accept structured arrays of input specs and output
  specs so one run can validate or transform multiple sources without collapsing their
  per-document root scopes.
- The target normalized run-config contract is split into authored config and
  normalized run-plan layers in
  [`cem-ml-phase2-run-config-contract.md`](cem-ml-phase2-run-config-contract.md).
  The authored JSON and CLI flag surfaces remain compatibility inputs; the
  normalized plan owns effective scopes, typed policy/budget values,
  resolver/module-map identities, provenance, diagnostics mode, and
  deterministic per-input/per-output execution ids.
- Config parsing, validation, normalization, and defaulting MUST be implemented in
  `cem_ml`. The CLI, WASM adapter, and Rust callers provide raw config bytes or raw
  spec-record strings plus declared config content type and schema identity; they must
  not carry separate config semantics.
- Config files are structural data too: config bytes + config content type + config
  schema/namespace identity are parsed through the CEM-ML-owned config lifecycle before
  document parsing starts. JSON is the current supported config content type for the
  implemented parser-backed run-config surface. Transform graph configuration is
  CEM-ML-first: do not add JSON `transforms[]` as the first transform graph surface.
  YAML or CSV config documents can be added later as content-type adapters that produce
  the same normalized graph/run model.
- CLI configuration has its own schema identity and validation contract. The existing
  JSON `RunConfig` surface has schema identity `https://cem.dev/ns/cli/run-config/1`
  and checked-in JSON Schema `packages/cem_ml/schema/cli/run-config.schema.json`
  (`https://cem.dev/schema/cli/run-config.schema.json`) for CI/tooling validation.
  The CEM-ML transform graph config has schema identity
  `https://cem.dev/ns/cli/transform-config/1`, separate from the CEM core document
  schema and separate from template schemas, for the `run` / `import` / `transform`
  / `export` element set and its attributes. Its checked-in schema artifact is
  `packages/cem_ml/schema/cli/transform-config.md`.
- CLI MUST support a config-file surface for reproducible CI/build use. The file is the
  preferred shape for multi-source runs, module maps, namespace maps, and multiple
  outputs.
- CLI MAY also support repeatable CSV option values for concise one-liners. CSV is a
  convenience projection of the same input/output spec arrays, not a separate behavior
  path. It must be parsed with deterministic escaping rules rather than ad-hoc comma
  splitting.

Multi-document build/CI runs MUST share one scheduler/thread-pool run context across
documents where safe, while keeping per-document root scopes, diagnostics, source maps,
and resource policy accounting isolated. This is required for validating or transforming
many data sources in one CLI invocation without paying one full runtime setup cost per
document.

## Transform Runtime Boundary

`cem-ml transform` is the data + template -> document command. It must not reuse
`convert` semantics:

- `convert` request shape: one document input, optional target identity, and document
  export/projection output.
- `transform` request shape: a CEM-ML-authored directed graph of import, transform, and
  export nodes. The CLI one-liner remains a shorthand for the single-import,
  single-template-transform, single-export graph.

Transform config MUST use CEM-ML nesting as the primary graph syntax:

```cem
{run |
  {import @id="book" @src="inputs/*.xml" @content-type="application/xml" |
    {transform @id="base" @src="templates/book.xslt" @template-content-type="application/xslt+xml" |
      {export @out="book/chapters/{stem}.html" @content-type="text/html"}

      {transform @id="html" @src="templates/book2html.xslt" |
        {export @out="book/chapters/{stem}.html" @content-type="text/html"}
      }

      {transform @id="chart1" @src="illustrations/chart1.xslt" |
        {export @out="book/chapters/{stem}/img/chart1.svg" @content-type="image/svg+xml"}
      }
    }
  }
}
```

Graph semantics:

- `import` reads resources and creates initial artifact nodes.
- `transform` consumes the current parent artifact and creates a child artifact.
- `export` consumes the current parent artifact and writes it; it does not mutate the
  artifact for sibling nodes.
- sibling transform/export nodes are branches from the same parent artifact.
- nesting encodes dependency and default data flow, not imperative mutation.
- the graph must be acyclic.
- one import produces one artifact per matched source in the first implementation.
- one transform consumes one primary artifact and produces one artifact in the first
  implementation.
- cardinality-changing stages such as split, group, reduce, and flat-map are future
  explicit transform kinds.
- output path templates should use named bindings such as `{stem}`, `{path}`, and
  `{index}` instead of positional `*` propagation.

Source-derived binding names:

- `{src}` is the resolved source path used by the CLI host.
- `{path}` is the matched path relative to the config document directory when possible.
- `{dir}` is the directory portion of `{path}`.
- `{file}` is the file name with extension.
- `{stem}` is the file name without its final extension.
- `{ext}` is the final extension without the dot.
- `{index}` is the stable zero-based index of the match within a sorted import match list.

Bindings are attached to imported artifacts, preserved by one-to-one transform stages,
and consumed by export `@out` templates. Missing bindings are hard config errors.
Duplicate resolved output destinations are hard runtime errors before writes begin.

Cross-input joins are allowed through explicit references, not inference. A transform
without `@input` consumes its parent artifact. A transform with `@input` consumes that
named primary artifact. Additional joins use named `@with:*` bindings:

```cem
{run |
  {import @id="orders" @src="data/orders.xml" @content-type="application/xml"}
  {import @id="customers" @src="data/customers.xml" @content-type="application/xml"}

  {transform
    @id="report"
    @src="templates/report.xslt"
    @input="orders"
    @with:customers="customers"
    |
    {export @out="dist/report.html" @content-type="text/html"}
  }
}
```

Reference semantics:

- IDs are unique across import and transform nodes in one config graph.
- `@input` and `@with:*` must reference existing import/transform IDs.
- references must not create cycles.
- secondary artifacts are passed to the transform engine as named inputs.
- document order must not imply joins; joins are explicit only.

Current implementation slice:

- `cem_ml::transform_config::parse_transform_graph_config` parses CEM-ML config
  bytes into import, transform, and export graph nodes plus dependency edges.
- The parser validates missing required operation attributes, duplicate IDs,
  unresolved explicit refs, cycles, and wildcard output patterns. Duplicate output
  destinations are validated after bindings are resolved.
- `cem_ml::engine::TransformGraphRequest` and `TransformGraphResponse` define the
  graph-shaped engine boundary for loaded imports, template-backed transform stages,
  export nodes, graph dependencies, scheduler scope IDs, diagnostics, artifacts, and
  scheduler trace.
- `RealCemMlEngine::transform_graph` executes the first in-memory graph runtime slice
  for CEM-native template stages when an executable adapter is registered. It imports
  graph data, executes one-to-one stages once their primary and secondary artifacts
  are available, and returns export artifacts in the response.
- `cem-ml transform --config FILE` lowers transform config into `TransformGraphRequest`
  in the CLI host, resolves relative import/template/export paths against the config
  document path, and writes graph export artifacts to configured destinations through
  the resolver layer. The first CLI expansion slice supports local filesystem import
  globs and resolver-backed import globs with exactly one `*` in the file name plus an optional
  single `**` directory segment for recursive descent, source-derived output bindings, and
  one-to-one binding propagation through transform stages. Resolver
  glob expansion requires an explicit list-capable resolver and is bounded by a
  deterministic max-entry guard.

The Rust/WASM engine API models transform as a first-class graph request/response
pair instead of smuggling template information through `ConvertRequest` or CLI-only
options. The graph-lowered request shape includes:

- imported data `EngineInput` nodes with their own root scopes and format identities;
- transform stage nodes with template resources and named primary/secondary input refs;
- export nodes with target `FormatIdentity`, output `ScopeConfig`, and destination;
- scheduler scope IDs for import, template load/compile, transform execution, and
  export;
- optional preservation of source-map frames from import and template inputs;
- the shared `EngineContext`, including resolver registry and scheduler settings.

The runtime API design is intentionally conservative for the first implementation
slice:

- CEM-native templates are not part of the base CEM-ML document lifecycle. They are
  selected through `TransformTemplateAdapterRegistry`, whose adapters match template
  content type, schema, and namespace identity. Built-in adapters cover the current
  CEM-native and XSLT identities; hosts can register newer CEM-native template
  iterations at runtime.
- `TransformTemplateAdapter` is also the execution plugin boundary. Adapters compile
  a `TemplateInput` plus entrypoint, params, and declared data binding names into an
  opaque compiled artifact, then render it against a primary data artifact, optional
  secondary artifacts, and a target identity/scope. The static built-in adapters
  currently return deterministic adapter-not-implemented errors for compile/render.
- Executable template adapters take precedence over selector-only adapters that
  match the same template identity. This lets hosts install a real runtime adapter
  for an identity already recognized by the built-in CLI/API contract while still
  rejecting multiple executable matches as ambiguous.
- `TransformExecutionPolicy` defaults to `runtimePhase=cem-ql-fragment`,
  `cardinality=one-to-one`, `duplicateDestinationPolicy=reject`,
  `failurePolicy=fail-fast`, and `outputPolicy=content-primary`. The current
  runtime accepts both `cem-ql-fragment` for fragment templates and
  `cem-native-modules` for declared CEM-native module templates.
- `TransformTemplateEntrypoint` is present on single-template requests and graph
  stages. `cem-ql-fragment` remains implicit-entrypoint only;
  `cem-native-modules` accepts validated public named entrypoints from the
  CEM-native template declaration schema.
- `params` is present on transform requests/stages. `cem-ql-fragment` rejects
  non-empty params; `cem-native-modules` validates caller params against declared
  module/template params before compilation and passes them to the executable
  adapter.
- `TransformDiagnosticOrigin` reserves stable report/source-map origin categories:
  `config`, `import`, `template-load`, `template-compile`, `template-execution`,
  and `export`.
- `validate_transform_request_runtime_contract` and
  `validate_transform_graph_runtime_contract` perform pre-execution runtime
  validation for the supported phases: `cem-ql-fragment` is CEM-native,
  implicit-entrypoint, no-params only; `cem-native-modules` is the CEM-native
  declaration/module phase; graph validation also checks known artifact refs,
  unique graph IDs, and duplicate output destination rejection.
- The default `CemMlEngine::transform` and `transform_graph` methods still return
  `NotImplemented`. `RealCemMlEngine::transform` implements the first one-to-one
  CEM-native template slice, and `RealCemMlEngine::transform_graph` implements the
  first in-memory CEM-native graph slice, when an executable adapter is available.

Supported template content types must be explicit adapter capabilities, not
hard-coded parser assumptions. The first runtime design supports both XSLT templates
and CEM-native templates through the transform-template adapter registry:

- XSLT template identities include `application/xslt+xml`, `text/xsl`, and the
  existing legacy custom-element XSLT content types such as
  `text/custom-element-xslt`.
- CEM-native template identities include `application/cem+xml`, `application/cem`,
  `text/cem`, `text/cem-ml`, and CEM core schema/namespace identity when no content
  type is present.
- Unsupported template identity combinations fail with deterministic diagnostics
  before execution, just as unsupported input/output identities do for parser-backed
  commands.

Current implementation slice: `cem_ml::engine::TransformTemplateKind` and
`classify_transform_template_identity_with_registry` encode that adapter selection
boundary for the CLI one-liner request helper and graph stages.
`EngineContext` carries a `template_adapter_registry` with built-in adapters by
default, and hosts may register runtime adapters for newer template content
types/schemas. The registry can also return the matched adapter object for future
compile/render calls. The CEM-ML graph config parser records `templateKind` on
transform nodes when identity is explicit or can be inferred from `@src`, and emits
deterministic diagnostics for unsupported or missing template identity. CLI dispatch
executes the one-to-one CEM-native path through the host-registered CEM-QL adapter.
Programmatic graph execution is available through `RealCemMlEngine::transform_graph`;
CLI config graph dispatch is available for concrete CEM-native graph paths, local
filename import globs, resolver-backed filename import globs, optional `**`
recursive import glob segments, explicit `join @mode="collect"` nodes, and
source-binding `join @mode="group-by" @by="..."` and same-binding
`join @mode="match-by" @by="..." @with:...` nodes, and positional
`join @mode="zip" @with:...` nodes. Bounded XSLT 1.0 parity executes through the
registered compatibility adapter; full XSLT 3.0/4.0 engine execution remains
deferred.

The first concrete executable CEM-native adapter lives in
`cem_ml_transform_cem_ql`, outside `cem_ml`, so it can depend on both `cem_ml` and
`cem_ql` without introducing a cycle. It compiles CEM-ML template fragments through
`cem_ql::render::compile_template`, carries the compiled payload in-process on the
adapter artifact, and renders through `cem_ql::render::render_compiled_template`.
`RealCemMlEngine::transform` uses that registered executable adapter path for the
minimal one-to-one programmatic engine runtime: the data document is loaded through
the lifecycle layer, parsed to the internal CEM DOM projection, passed as the
primary transform data artifact without JSON serialization in the native path,
compiled/rendered by the selected adapter, and returned as
`TransformResponse.primary`. The CLI host context registers the same executable
CEM-QL adapter for transform request construction and dispatch.
Data-driven CEM-native templates should read the primary artifact through
`$input`, named secondary artifacts through their graph labels, and params through
their declared names. The CEM-QL renderer also preserves the legacy compatibility
projection `$datadom.attributes.*`; direct `$label`-style primary-object
convenience bindings are adapter compatibility behavior, not the portable
template contract.

CEM-native template execution landed before bounded XSLT parity expansion, in this order:

1. Minimal CEM-native runtime: support pure CEM-QL evaluation and CEM-ML fragments
   with embedded CEM-QL, with one implicit template entrypoint per template resource.
2. Native template/module layer: add named templates, explicit entrypoints, params,
   imports/includes, visibility, caching, and recursion/cycle limits.
3. XSLT parity expansion: deepen XSLT support after the native named-template/module
   substrate exists, so migration has a first-class native landing zone.

The native template/module layer has the following contract for the implemented
CEM-native runtime and the bounded XSLT parity substrate:

- A template resource compiles as a module owned by the selected template adapter.
  Module syntax, declarations, and schema rules are part of the template
  content-type/schema plugin, not the stable base CEM-ML document AST.
- The implicit entrypoint is the module default render entrypoint. Explicit
  `TransformTemplateEntrypoint::named("name")` selects a public named template
  exported by the module. Missing or private entrypoints fail during template
  compile/validation, before rendering data.
- Declarations are private by default. Public visibility must be explicit. This
  keeps helper templates and local params from becoming an accidental cross-module
  API.
- Params are immutable per render call. Template declarations may provide defaults;
  caller-provided params override defaults by name. Unknown params are fatal unless
  the selected adapter explicitly declares an extension bucket for them. A caller
  param is considered provided when its key is present, including when the value is
  `null`; only omitted param keys use declaration defaults or trigger missing
  required-param diagnostics. For a selected named entrypoint, the local param name
  and its qualified `entrypoint.name` form are aliases; either form may be used
  alone, but providing both aliases is a fatal duplicate-param diagnostic.
- Param declarations may declare `@type`: `any`, `string`, `boolean`, `number`,
  `integer`, `array`, `object`, or `json`. Omitted `@type` is `any`. Typed caller
  params are validated as JSON shapes before adapter compilation. Params are
  non-nullable by default; `@nullable="true"` allows an explicit JSON `null`
  caller value or literal `@default="null"`. Explicit `null` is still considered
  provided for requiredness. String-valued caller params from CLI/config inputs
  are normalized at the module contract boundary: nullable literal `null` becomes
  JSON null, `boolean` accepts `true`/`false`, and
  `number`/`integer`/`array`/`object`/`json` parse JSON before validation;
  non-nullable `any`/`string` params keep text such as `null` as a string.
  `@default` is literal: `any`/`string` keep the
  attribute text as a string, `boolean` accepts only `true`/`false`, and
  `number`/`integer`/`array`/`object`/`json` parse JSON and must match the
  declared type. Default expressions remain reserved behind `@default-expr` /
  `@defaultExpr`; using either spelling is fatal until expression evaluation
  context, resolver policy, and reporting are defined.
- Data bindings are explicit adapter inputs. The stable binding set for the native
  module layer should include the primary artifact under `input`, named secondary
  artifacts under their `@with:*` labels, and declared params under their names.
  The CEM-QL data document also exposes these host bindings as top-level fields
  while retaining the legacy `datadom.attributes.*` projection. Any direct
  convenience bindings derived from the primary object are adapter compatibility
  behavior and must not be required by portable templates.
- Imports should be implemented before includes. `import` loads a separate module
  through the template resolver, keeps that module's private declarations isolated,
  and exposes only public exports under an explicit alias or namespace. `include`
  is reserved for a later lexical merge feature and should remain unsupported until
  the import/caching/cycle rules are proven.
- Module resolution uses the same resolver registry with purpose `template`.
  Relative imports resolve against the importing template's resolved URI. Resolver
  diagnostics use the `template-load` origin; compile failures use
  `template-compile`; render failures use `template-execution`.
- Template module caching is keyed by adapter ID, resolved template URI, template
  identity, content hash, selected entrypoint, execution policy, and the resolved
  dependency graph hash. Cache entries must not cross adapter versions or template
  schema versions.
- Cycles in the import graph are fatal compile-time diagnostics. Recursive template
  calls are allowed only through explicit named-template calls and must be bounded
  by scheduler/runtime limits. The default failure policy remains fail-fast.
- Output remains content-primary for this layer. Multi-output/report side effects
  should continue to be modeled by graph branches and export nodes rather than by
  arbitrary template writes.

The v1 CEM-native template declaration schema is
`https://cem.dev/ns/template/cem-native/1`, with checked-in schema artifact
`packages/cem_ml/schema-packages/cem-native-template/v1/`. This schema is owned by
the CEM-native template adapter family and is not the base CEM core document
schema. It defines the declaration vocabulary:

- `module` is the template resource root and may contain `import`, module-level
  `param`, named `template`, and one default `body`.
- `import @as @src` declares a resolver-backed module dependency. Optional
  `@content-type` / `@contentType` and `@schema` provide identity hints.
- `param @name` declares an immutable render parameter. Optional `@type`,
  `@nullable`, `@default`, `@default-expr` / `@defaultExpr`, `@required`, and
  `@visibility` describe JSON shape, nullability, literal defaults, the reserved
  future expression-default slot, requiredness, and API surface.
- `template @name` declares a named entrypoint. Optional `@visibility="public"`
  exports it across module boundaries; otherwise it is private.
- `body` holds the implicit entrypoint body or the body of a named template.
- `call @template` invokes a same-module template; `call @from @template`
  invokes a public template from an imported module. `@with:*` passes named
  bindings.
- `include` is intentionally absent and remains reserved.

Current API state: `TransformTemplateCompileRequest` carries
`TransformTemplateModuleOptions`, and the stable model includes import
declarations, entrypoint declarations, param declarations, module limits, module
cache keys, and reserved diagnostic codes for module compile/runtime failures.
The engine now parses and lowers the v1 CEM-native template declaration schema
before adapter compilation: `module` declarations provide resolver-backed
imports, module params, named entrypoints, and template-local params in
`TransformTemplateModuleOptions`; `call` sites from `body` content are lowered
as non-executing call records. Plain CEM fragment templates without the
native-template schema continue to compile as declaration-free fragments.
The real engine validates named entrypoint requests against public declarations,
rejects unknown caller params, validates typed caller/default param values, and
reports missing required params before adapter compilation. It also validates
same-module `call @template` targets and imported `call @from @template` targets
against public entrypoints from resolver-backed import modules.
The `cem_ml_transform_cem_ql` executable adapter now preserves the selected
entrypoint, caller params, and param declarations in the compiled payload. During
render, caller params override declaration defaults, module-level defaults apply
when omitted, explicit `null` values remain bound as caller values, and named
entrypoint-local params are exposed through their local names inside the selected
template or called imported template. Qualified caller params such as `card.title`
bind the same local value as `title` for selected entrypoint `card`; passing both
aliases is rejected before adapter compilation.
Compile requests now also carry `TransformTemplateModulePreflight`, populated by
the real engine before adapter compilation. The preflight recursively reads
declared imports through the template resolver, resolves relative imports against
the importing template URI, records resolved module bytes/identity/content hashes
with `parentUri` for non-root import edges, constructs the dependency graph cache
key input, rejects duplicate import aliases per importing module, rejects
reserved includes, enforces import-depth limits, and rejects import cycles. The
CEM-QL executable adapter compiles preflighted modules into its native payload
and exposes import metadata on the compiled artifact. It dispatches validated
same-module and imported module calls during render with the current data
context. When an imported module's public entrypoint renders, unqualified calls
resolve against that imported module's own named templates, and `call @from`
resolves against that module's own import aliases rather than the root module.
Call `@with:*` whole-expression attributes preserve their evaluated CEM-QL item
stream for the invoked template; literal and mixed attribute-value-template forms
remain string bindings.
Same-module recursive calls, including recursive calls that occur while rendering
an imported module, are bounded by the module recursion limit and report
`cem.transform_template.recursion_limit` when exceeded.

`cem_ml` remains the stable API contract and cannot directly call
`cem_ql::render` while `cem_ql` depends on `cem_ml`; executable renderers must be
registered by crates or hosts above both layers.

Template reads and transform outputs must use the shared resolver layer. Template reads
use the dedicated resolver purpose `template`, separate from ordinary data `input`
reads, so hosts can authorize executable/semi-executable template resources separately
from data. Local paths and local `file://` URIs use existing local resolver behavior;
remote/custom template URIs require registered template resolvers. Output writes must
follow the same local-only default and registered-resolver behavior as `convert --out`,
side reports, and configured output destinations.

Transform responses must preserve the content-primary command contract:

- primary rendered document bytes/projection go to `--out` when provided, otherwise
  stdout;
- JSON/Markdown reports are side outputs, not replacements for primary content;
- diagnostics include data/template/target identity and source-map frame context where
  available;
- scheduler traces distinguish data loading, template loading/compilation, execution,
  and output export;
- graph validation must reject duplicate IDs, unresolved references, cycles, ambiguous
  output paths, and unsupported cardinality-changing stages before execution.
- runtime preflight already rejects the unsupported first-slice cases that can appear
  in programmatic engine requests even if the CEM-ML config parser was bypassed.

The single-template CLI path writes the primary rendered document to `--out` when
provided, otherwise stdout. Diagnostics and warnings are written to stderr unless
`--report-json` or `--report-md` is provided; when a report destination is provided,
diagnostics are recorded in the report side output instead. Transform report
directory destinations use the default basename `cem-ml.transform.report`.

Current implementation status:

- `parse`, `validate`, `check`, `inspect`, `convert`, and fixture flows already route
  through the `cem_ml::engine::CemMlEngine` trait.
- `cem_ml::run_config::RunConfig` defines the shared structured shape for input specs,
  output specs, root scope configuration, and scheduler configuration. `cem_ml` owns
  `parse_run_config(bytes + FormatIdentity)`, plus repeatable CSV `InputSpec` /
  `OutputSpec` record parsing. CLI accepts `--config`, `--config-content-type`,
  `--config-schema`, repeatable `--input-spec`, and repeatable `--output-spec` by delegating parsing to
  `cem_ml`; WASM exposes helpers over the same library parser. This is the first
  execution slice: input specs override global input content-type/schema/base URI during
  lifecycle dispatch, and the first output spec can select conversion target content
  type, schema, namespace identity, and destination. Config diagnostics for malformed JSON,
  unsupported config content type/schema identity, duplicate input URIs, and unknown output input references fail before
  document parsing, and report-capable commands emit those diagnostics through requested JSON/Markdown reports.
  `--observe-events` uses the same normalized input list and
  lifecycle dispatch path as parser-backed commands, including `--input-spec` and
  `--config` inputs. Config-file convert execution fans out multiple `outputs[]`
  records, using `inputRef` or the sole configured input for each output. Normalized
  `RunConfig.scheduler` flows into the engine execution context, and the trace worker
  policy is derived from that scheduler config. Validate/check reports embed a shared
  run-level scheduler trace with per-document scope IDs. Validate/check execute
  lifecycle loading and parser-backed validation through scheduler-dispatched
  per-document tasks, so the trace reflects actual document work instead of report
  projection only. Convert can write explicit side reports from scheduler traces returned
  by engine convert execution while preserving content-primary stdout/`--out` behavior.
  Full input and output root-scope config reaches engine requests. Recognized
  root-scope scheduler policy and budget fields derive the per-scope worker policy for
  scheduled validate/check, trace, and convert execution; `parseMs` enforces a
  parser-backed pipeline wall-clock budget; `validateMs` and `checkMs` enforce
  scheduled per-input document work budgets; `convertMs` enforces input/output-scope
  convert work budgets; `traceMs`, `inspectMs`, `benchMs`, `fixtureValidateMs`,
  `fixtureRoundtripMs`, and `observeMs` enforce trace, inspect, benchmark, fixture,
  and observability workflow budgets; and effective `baseUri` values project relative
  report input and diagnostic URIs.
  Root-scope default and named namespace bindings seed schema validation's
  document-root namespace context. Recognized CEM-ML root-scope version pins resolve
  against the embedded document-format version. Input root-scope module maps provide
  the resolver base for relative schema `src` identities, load local JSON alias maps
  for schema-source specifier resolution, and normalize relative module-map paths
  against the config document path, including local `file://` config document bases.
  Config-file output destinations normalize relative paths against the config document
  path, including local `file://` config document bases.
  Run-config normalization validates root-scope module-map, namespace, and version-pin
  option shape before document parsing, while unreadable or malformed module maps,
  unknown future budget keys, and unsupported version-pin targets emit deterministic
  execution diagnostics instead of being silently ignored. Remote/custom module-map
  URI values now load through registered `EngineContext` resolvers when available and
  otherwise emit an explicit unsupported-resolver diagnostic instead of being treated
  as local paths. Config document reads plus configured and positional input reads use
  the same resolver-aware read path when a runtime context provides a resolver.
  Reserved transform request construction uses the dedicated `template` read purpose
  for template resources, not the ordinary `input` purpose. Primary output,
  side-report, and observability event destinations now write through registered
  resolvers when available. The default CLI context remains local-only, but
  `--resolver-read-map URI-PREFIX=DIR`, `--resolver-write-map URI-PREFIX=DIR`, and
  run-config `resolvers` entries can explicitly map remote/custom URI prefixes to local
  filesystem roots for reads or writes.
- `--schema` and `--content-type` are carried in `EngineContext` and emitted in reports.
  `cem_ml::lifecycle::LifecycleRegistry` now owns built-in input content-type dispatch
  for parser-backed commands (`parse`, `validate`, `check`, `inspect`, `convert`,
  `trace`, `bench`, and fixture workflows). CEM core schema identity
  (`https://cem.dev/ns/core/1`), CLI transform config schema identity
  (`https://cem.dev/ns/cli/transform-config/1`), and CEM-native template schema identity
  (`https://cem.dev/ns/template/cem-native/1`) select the CEM adapter when no content type
  is present. CEM core namespace identity selects the CEM adapter when no content type or
  schema is present, and HTML/SVG namespace identity selects the HTML adapter when no content
  type or schema is present. HTML/SVG schema identities (`http://www.w3.org/1999/xhtml`,
  `http://www.w3.org/2000/svg`) also select the HTML adapter when no content type is present.
  SVG content type (`image/svg+xml`) selects the XML adapter.
  XSLT namespace identity (`http://www.w3.org/1999/XSL/Transform`) selects the
  `custom-element-xslt-compat` adapter when no content type or schema is present,
  while explicit content type remains authoritative. Unsupported input identities
  emit deterministic lifecycle diagnostics with the declared content type, schema, and/or
  namespace while preserving the fallback input format. CEM/HTML target export selection is
  registry-owned for `convert --to-content-type application/cem+xml`,
  `convert --to-schema https://cem.dev/ns/core/1`, and
  `convert --to-content-type text/html` / `application/xhtml+xml`; `.xhtml` path
  inference feeds `application/xhtml+xml` into the same HTML adapter. HTML/SVG target
  schema identities also select the HTML adapter. XML target export is
  registry-owned for `convert --to-content-type application/xml` / `text/xml` / `image/svg+xml`, plus namespace-only
  CEM core and HTML/SVG targets. Semantic DOM/AST/events projection schemas are
  registry-owned for `https://cem.dev/ns/projection/dom/1`,
  `https://cem.dev/ns/projection/ast/1`, and
  `https://cem.dev/ns/projection/events/1`, with primary
  `application/vnd.cem.*+cem-bin` content types. JSON output remains an optional
  debug/interchange projection selected by `dom-json`,
  `application/vnd.cem.dom+json`, `application/vnd.cem.ast+json`,
  `application/vnd.cem.events+json`, or generic JSON output with explicit schema
  identity in output-spec/config root-scope identities. The legacy
  `https://cem.dev/ns/projection/dom-json/1` schema remains a compatibility
  identity for the DOM JSON view until callers migrate to the semantic DOM
  schema plus `+json` content type.
  Transform config and CEM-native template schema targets
  are also registry-owned as CEM output syntax. Unsupported target identities emit a
  deterministic lifecycle diagnostic with the declared content type, schema, and/or namespace while
  preserving the requested fallback output projection. Broader third-party or future
  non-CEM adapters remain deferred plugin/content-type work outside the current
  built-in lifecycle set.
- Root-scope configuration is complete for the current CLI/lib/WASM contract. Current execution uses run-config identity
  fields for lifecycle dispatch and conversion output selection, recognized scheduler
  policy/budget fields for scheduled worker policy, `parseMs` for parser-backed
  wall-clock enforcement, `validateMs` / `checkMs` for scheduled per-input document
  work budgets, `convertMs` for input/output-scope convert work budgets, and
  `traceMs` / `inspectMs` / `benchMs` / `fixtureValidateMs` / `fixtureRoundtripMs` /
  `observeMs` for trace, inspect, benchmark, fixture, and observability workflow
  budgets.
  Effective `baseUri` values project relative report input and diagnostic URIs.
  Default and named namespace maps seed schema validation's root namespace context.
  Recognized CEM-ML version pins resolve through the document-format
  version resolver. Input module maps resolve relative schema-source identities and
  local JSON aliases, including config-relative module-map paths, local `file://`
  module-map URIs, and local `file://` config document bases. Config-file output
  destinations normalize relative paths against the config document path, including
  local `file://` config document bases. Configured, positional, and fixture-materialized
  input reads resolve local `file://` URIs, and primary output, per-output conversion,
  side-report, and observability event writes resolve local `file://` destinations to
  filesystem paths. Config document reads, configured and positional input reads,
  fixture/benchmark materialized input reads, remote/custom module-map URI values, and
  output/report/observability destinations all have resolver-aware helper paths. The
  default CLI context registers no remote/custom resolvers, so these URI sources and
  destinations still reject explicitly unless a host, CLI resolver-map option, or
  run-config `resolvers` entry installs one.
- `validate` / `check` / `convert` route `custom-element-xslt` input through the first
  shared lifecycle adapter path, lowering custom-element XSLT compatibility input to canonical
  CEM-ML through `cem_ml::legacy_custom_element`; the `custom-element-xslt` to
  `application/cem+xml` route selects canonical CEM-ML export from the declared
  target identity through the lifecycle registry.
- `schema` and `plugin` CLI command groups are reserved until the registry and plugin
  lifecycle surfaces are promoted from library internals to command-line workflows.

## Resolver Semantics

Resolver behavior is a shared runtime contract, not a CLI-only path parser. The
`cem_ml` library owns URI classification, purpose-aware resolver request/response
types, and deterministic resolver diagnostics. Host surfaces provide concrete resolver
implementations:

- Rust hosts pass a resolver registry into execution context.
- WASM hosts expose callback-backed read/write resolvers for browser, worker, or Node
  embedding through `onResolveRead` and `onResolveWrite`; Rust-side WASM entrypoints
  install `JsResourceResolver` into the execution context for selected URI schemes.
- The CLI stays local-only by default and installs opt-in local mirror resolvers from
  `--resolver-read-map URI-PREFIX=DIR`, `--resolver-write-map URI-PREFIX=DIR`, or
  run-config `resolvers` entries. These maps do not fetch network resources; they map a
  remote/custom URI prefix to a local filesystem root. Read maps do not imply write
  permission, and write maps do not imply read permission. CLI read maps also install
  list support for resolver-backed transform import globs.

Every resolver operation carries a purpose so hosts can apply policy by capability:

- `config` reads the run configuration document before parsing it.
- `input` reads configured, positional, or fixture-materialized document inputs.
- `moduleMap` reads root-scope module-map JSON and establishes the base URI for relative
  schema-source identities.
- `output` writes primary output or per-output conversion artifacts.
- `report` writes JSON or Markdown side reports.
- `observeEvents` writes JSONL observability event streams.

Resolver requests include the declared URI, the effective base URI for relative values,
the operation purpose, the direction (`read`, `write`, or `list`), an optional content-type hint,
and the root-scope or output-scope identity that caused the request. Resolver responses
return the resolver-finalized `resolvedUri`, bytes for reads, write acknowledgement for writes, or sorted URI entries for lists,
optional content type, and optional cache metadata. Reports and diagnostics should keep
the declared URI visible while also allowing `resolvedUri` when it differs.

Built-in semantics:

- Plain filesystem paths and local `file://` URIs are handled by the local filesystem
  resolver.
- `file://localhost/...` is equivalent to a local `file://` URI.
- Non-local `file://host/...`, `http://...`, `https://...`, and custom schemes are
  delegated to registered resolvers by scheme and purpose.
- If no resolver is registered for a scheme/purpose pair, the operation fails with a
  deterministic resolver diagnostic instead of falling back to local path behavior.
- Relative URI/path values are resolved against the effective base URI before resolver
  dispatch. For config-derived values, the config document URI is the base unless an
  explicit root-scope base URI overrides report/diagnostic projection only.

Error mapping must stay stable across CLI, Rust, and WASM hosts. Resolver failures use
diagnostics such as unsupported resolver, permission denied, not found, invalid URI,
read failure, and write failure; CLI commands may wrap those diagnostics in command-level
I/O messages, but they must not replace the underlying resolver code or URI.

## Functional Surface

- Parse one input into structured output.
- Load supported inputs into the internal CEM event stream / AST through the adapter selected by content type + schema.
- Validate one or more inputs and emit human-readable or machine-readable diagnostics.
- Run CI-oriented checks with hard-violation behavior.
- Inspect parsed output as summary, tree, AST, events, diagnostics, or source-offset views.
- Convert/export supported documents from one declared document format into another declared document format, or into
  debug projections through the same internal AST/binary artifact spine.
- Reserve transform execution for applying a template/stylesheet to data to produce a document; the CLI shape is
  parseable, but runtime execution is not implemented.
- Trace parser and validator work with deterministic text or JSON output.
- Benchmark parse and validate work with deterministic text or JSON reports.
- Validate the default semantic fixture set or explicitly provided fixture paths.
- Round trip fixtures through parser-backed projections until transform/render snapshots exist.
- Print help and version information.
- Reserve schema and plugin workflows until their subsystems are designed.

## Planned Option Behavior

- Fail level: `parse`, `validate`, `strict`.
- Input identity selection by content type and schema, with `--from-format cem|html|xml`
  retained only as a convenience alias while the registry matures.
- Output identity selection by content type, schema, and namespace identity, with
  `--to-format cem|html|dom-json|ast|events|dom-bin|ast-bin|events-bin`
  retained for current projections and debug layers. `dom-json` is a JSON view
  over the CEM DOM projection, not the native transform transport; `*-bin`
  selects sealed CEM binary projection artifacts. CLI stdout and `--out` write
  raw artifact bytes for `*-bin` from the native byte response payload. The
  primary JSON response for binary artifacts is metadata-only and omits embedded
  chunk data; full JSON chunk envelopes remain compatibility/debug views.
  Library routing exposes the same binary artifacts as sealed chunk streams.
  Multiple sinks can receive the same immutable chunk bytes through deterministic
  or parallel route execution without changing the binary representation.
- Root-scope configuration for inputs and outputs: default content type, schema,
  version pins, default namespace, named namespaces, module map / resolver identity,
  base URI, scope policy, and resource budgets.
- Command-level input root-scope defaults include `--content-type`, `--schema`,
  `--default-namespace`, repeatable `--namespace PREFIX=URI`, `--module-map`,
  repeatable `--version-pin NAME=CONSTRAINT`, `--scope-policy`, repeatable
  `--scope-budget NAME=VALUE`, and `--base-uri`;
  config files or `--input-spec` records remain the preferred shape for richer
  per-input maps and resolver settings. These command-level defaults use the same
  run-config default-scope validation as their `rootScope` config/spec counterparts.
- For `convert`, command-level `--to-content-type`, `--to-schema`, `--default-namespace`,
  repeatable `--namespace PREFIX=URI`, and `--base-uri` also form the default output
  target identity unless an `--output-spec` / config output supplies a richer output root scope.
- `convert` is the implemented document-to-document conversion command, for example:

    ```bash
    cem-ml convert input.xml \
      --content-type application/xml \
      --to-content-type application/cem+xml \
      --out output.cem
    ```

- `transform` is the data + template -> document workflow. Its direct command shape is:

    ```bash
    cem-ml transform data.xml \
      --data-content-type application/xml \
      --template view.xsl \
      --template-content-type application/xslt+xml \
      --to-content-type text/html \
      --out view.html
    ```

    It also accepts `--data-schema`, `--template-schema`, `--template-entrypoint`, repeatable
    `--param NAME=VALUE`, `--to-schema`, shared context options, `--report`, `--report-format`, and
    compatibility aliases `--report-json` / `--report-md`. The current CLI runtime executes the one-to-one CEM-native path and CEM-ML
    `--config` graph dispatch for concrete paths plus local and resolver-backed filename import globs, optional `**`
    recursive import glob segments, source-derived output bindings, explicit `join @mode="collect"` aggregation, and
    source-binding `join @mode="group-by" @by="..."` aggregation, and same-binding
    `join @mode="match-by" @by="..." @with:...` aggregation, and positional
    `join @mode="zip" @with:...` aggregation. Bounded XSLT 1.0 parity executes through the registered compatibility
    adapter, while full XSLT 3.0/4.0 engine execution remains deferred.

- Multi-source configuration via config file, plus repeatable CSV option records for
  CLI one-liners. Config files are preferred for CI/build reproducibility.
- Config-file content type via `--config-content-type`, inferred from extension when
  omitted for known config formats, plus config schema identity via `--config-schema`.
- Output format selection for CEM-native, XML, JSON, text, HTML, Markdown, DOM JSON debug view, AST, events, CEM binary projections, and
  tree-shaped output where relevant.
- Output destination handling for stdout and `--out`.
- Report destinations for default CEM-ML reports plus explicit JSON and Markdown projections, including directory
  destinations with default filenames.
- Schema, content-type, namespace, and base-URI option plumbing even before full
  schema resolution exists.
- Quiet, verbose, and no-color terminal behavior.
- Zero-hard-violations check behavior.
- Source-offset preservation for conversion and parser projection workflows.
- Convert input/output format selection.
- Inspect view selection.
- Benchmark iterations, budget, profile, cold-cache, and JSON report options.
- Default canonical CEM-ML fixture paths and secondary semantic HTML parity fixture paths.

## Output Shapes

Diagnostics keep these fields where available:

- `uri`
- `line`
- `column`
- `byteOffset`
- `code`
- `severity`
- `message`
- optional `node`
- future `sourceMap`

Reports are rendered from the canonical AST-associated report tree. Report event nodes keep:

- source module state
- event sequence
- source-map stack at event time
- visible partial DOM/AST hierarchy at event time

Reports default to CEM-ML syntax. JSON is an explicit report projection selected with
`--report-format json` or the compatibility alias `--report-json`; Markdown is selected with
`--report-format md` or `--report-md`. The JSON projection keeps deterministic field names:

The checked-in JSON Schema for this explicit JSON projection is
`packages/cem_ml/schema/cli/report.schema.json`
(`https://cem.dev/schema/cli/report.schema.json`). The schema is copied to
`packages/cem_ml/dist/cli/report.schema.json` by `cem_ml:build:docs`, and
`cem_ml:test:cli-schema-artifacts` verifies the dist copy and schema identity.

- `generatedAt`
- `inputs`
- `summary.inputCount`
- `summary.infoCount`
- `summary.warningCount`
- `summary.errorCount`
- `summary.fatalCount`
- `summary.hardViolationCount`
- `options.failLevel`
- `options.schema`
- `options.contentType`
- `options.baseUri`
- `reportAst.schedulerTrace.eventCount`
- `reportAst.schedulerTrace.events[]`
- optional `reportAst.convert.outputCount`
- optional `reportAst.convert.outputs[]`
- optional `reportAst.transform`
- optional `reportAst.transformGraph.exportCount`
- optional `reportAst.transformGraph.exports[]`

Convert report output entries keep one item per successfully written primary
convert output. These entries describe the target-native destination and target
identity; debug artifacts written with `convert --artifact-json` are not primary
outputs and are intentionally not listed here.

- `input`
- optional `destination` (`<stdout>` in Markdown when omitted)
- optional `contentType`
- optional `schema`
- `outputKind`
- optional `conversion`

`conversion` is the selected conversion execution summary. It is report-only
metadata; it never changes the primary output bytes. It contains:

- optional `converterId`, such as `cem-dom-projection-to-html-cemt`,
  `direct-cem-output`, or a project-local converter id;
- optional `implementation`, currently one of:
    - `cemt`: an executable CEMT converter template rendered, then its output
      passed through the declared CEMT output pipeline when present;
    - `direct-cemt-output-pipeline`: the selected converter or built-in route
      supplied the formatter/colorizer/writer CEMT output pipeline directly;
    - `rust-fallback`: a selected CEMT converter could not execute and used its
      declared Rust fallback;
- optional `rustFallback` when the Rust fallback path was used;
- optional `outputPipeline.stages[]`.

`outputPipeline.stages[]` is ordered by execution phase. Formatter and colorizer
stages write CEM trees. The writer stage runs last and writes the requested
target-native content type. Stage entries can include `stage`, `function`,
`profile`, `contentType`, `schema`, `category`, and `produces`.

Single transform report entries keep:

- `input`
- optional `destination`
- `outputKind`
- `hasSourceMap`
- `outputSpanCount`
- optional `sourceMapRef`

Transform graph export report entries keep:

- `exportId`
- `input`
- optional `destination`
- optional `contentType`
- optional `schema`
- `outputKind`
- `hasSourceMap`
- `outputSpanCount`
- optional `sourceMapRef`
- optional `collectionItems[]` for collection exports, with `input`, `artifactId`, `hasSourceMap`, and
  `outputSpanCount`

When a single transform response or export artifact advertises a source map and has a concrete destination,
`sourceMapRef` is the sidecar reference for that output, currently `{destination}.map`. The CLI writes the source-map
JSON payload to that sidecar through the output resolver. Object-shaped sidecars include:

- `frames`: the origin-first source-map stack;
- `outputSpans`: output byte ranges paired with their origin stacks;
- `exportId`;
- `input`;
- `destination`;
- graph collection item sidecars also include `artifactId`, optional `uri`, and `collectionDestination`.

For graph exports, the CLI adds `exportId`, `input`, `destination`, and `outputSpans` metadata to object-shaped
source-map payloads. Collection export sidecars use a collection-shaped source-map payload with per-item `sourceMap` and
`outputSpans` entries instead of flattening multiple provenance stacks into one. Stdout outputs omit `sourceMapRef`
because they do not have a stable adjacent file path.
`cem-ml transform --source-map-summary` prints source-map refs and output-span counts to stdout after successful writes.
For single transforms, `--source-map-summary` requires `--out` so the summary cannot be appended to document stdout.
Collection primary JSON outputs are a stable public projection with `kind`, `mode`, `count`, `bindings`, and `items[]`
entries containing `input`, `artifactId`, `uri`, `identity`, `primary`, and `bindings`. Per-item `sourceMap` and
`outputSpans` are intentionally omitted from the primary output and remain in sidecar/report surfaces.
When an export artifact is object-shaped and has a string `content` field, the configured output destination receives
that document content; sidecar/report metadata remains outside the primary output body.

The deterministic default timestamp for feature tests is `1970-01-01T00:00:00.000Z`.

## Report Ownership

- Fixture validation default CEM-ML: `packages/cem_ml_cli/dist/cem-ml.report.cem`
- Parse default CEM-ML (`cem-ml parse`): `packages/cem_ml_cli/dist/cem-ml.report.cem`
- Convert default CEM-ML (`cem-ml convert`): `packages/cem_ml_cli/dist/cem-ml.convert.report.cem`
- Transform default CEM-ML (`cem-ml transform`): `packages/cem_ml_cli/dist/cem-ml.transform.report.cem`
- Fixture roundtrip default CEM-ML: `packages/cem_ml_cli/dist/cem-ml.roundtrip.report.cem`
- Benchmark default CEM-ML (`cem-ml bench`): `packages/cem_ml_cli/dist/cem-ml.bench.report.cem`
- Explicit JSON replaces the extension with `.json`, e.g. `--report-format json` or `--report-json`.
- Explicit Markdown replaces the extension with `.md`, e.g. `--report-format md` or `--report-md`.

CEM-ML reports are the primary structured projection. JSON is a machine-validation projection covered by
`report.schema.json`. Text, Markdown, and HTML reports are reference-implementation convenience projections.

## Exit Codes

- `0`: success
- `1`: parse, validation, strict-mode, or benchmark budget failure
- `2`: CLI usage error, including reserved commands
- `3`: schema resolution error, reserved
- `4`: transform failure
- `5`: plugin failure, reserved
- `6`: I/O failure
- `7`: unexpected internal failure

## Verification Scope

Rust-side tests should assert functional behavior, option parsing, JSON/report fields, diagnostics, and exit codes.
