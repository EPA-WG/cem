# CEM-ML Phase 2 Lifecycle Adapter And Content-Identity Audit

Status: completed audit for the Phase 2 lifecycle adapter checklist item. This
document records current `cem_ml` and `cem_ml_cli` behavior against the
roadmap's content-identity-first adapter target. It does not change runtime
behavior.

## Scope

This audit covers the current validate/load/export lifecycle path after the
normalized run-config work:

- library identity surfaces in `EngineContext`, `EngineInput`, `FormatIdentity`,
  `InputFormat`, and `LayerFormat`;
- `cem_ml::lifecycle` input and target adapter dispatch;
- `RealCemMlEngine` parse, validate, check, inspect, trace, bench, and convert
  entry points;
- `cem_ml_cli` validate and convert request construction, including run-config
  fanout;
- CEM-ML, HTML/XML parity, custom-element XSLT compatibility, projections,
  diagnostics, source maps, and enum alias behavior.

Transform-template adapter dispatch, schema-package converter dispatch, and
resource streaming are referenced only where they intersect the lifecycle
adapter boundary.

## Phase 2 Target

The Phase 2 target is an adapter contract where content type plus schema or
namespace identity is primary:

- every supported format follows validate, load into CEM AST/events, then
  export;
- CEM-ML, HTML/XML parity, and XSLT 1.0 custom-element compatibility are
  adapters over the shared internal spine;
- CLI `--from-format` and `--to-format` remain compatibility aliases, not the
  primary selection model;
- unsupported or ambiguous identity outcomes are deterministic diagnostics, not
  silent fallback;
- diagnostics, source-map stacks, scheduler traces, and per-document identities
  survive adapter boundaries.

## Current Surfaces

### Identity And Alias Surface

Current status: identity fields exist, but enum fallbacks are still part of the
public engine request shape.

- `FormatIdentity` carries `contentType`, `schema`, `defaultNamespace`,
  `namespaces`, and `baseUri`.
- `EngineContext` carries global `schema`, `contentType`, and `baseUri`
  defaults plus registries and scheduler configuration.
- `EngineInput` carries bytes, URI, optional `FormatIdentity`, root scope, and
  optional `InputFormat` compatibility hint.
- `ConvertRequest` carries target identity and target scope, but it still
  requires a `LayerFormat`.
- `InputFormat` is limited to `cem`, `html`, and `xml`.
- `LayerFormat` is limited to `cem`, `html`, `xml`, `dom-json`, `ast`,
  `events`, `dom-bin`, `ast-bin`, and `events-bin`.

Gaps:

- `ConvertRequest.to_format` is still mandatory, so target identity cannot yet
  be the only export selector.
- `InputFormat` and `LayerFormat` do not record provenance that distinguishes
  explicit aliases from fallback defaults.
- There is no typed lifecycle adapter descriptor that says which identities,
  operations, projections, or source-map guarantees an adapter owns.

### Lifecycle Registry

Current status: `cem_ml::lifecycle` is the main library dispatch boundary and
already models a useful first-generation adapter registry.

- `LifecycleAdapter` exposes `id`, `matches_input`, `load`,
  `matches_target`, and `target_format`.
- `LifecycleRegistry::with_builtin_adapters()` registers:
  - `cem-ml`;
  - `html`;
  - `xml`;
  - `custom-element-xslt-compat`;
  - DOM, AST, and events projection adapters for JSON and binary outputs.
- Input matching uses content type first. Schema identity participates only when
  content type is absent. Namespace identity participates only when both content
  type and schema are absent.
- Target matching follows the same content-type-first shape and maps matched
  identities back to `LayerFormat`.
- Unsupported input identities emit `cem.lifecycle.adapter_unsupported`.
- Ambiguous input identities emit `cem.lifecycle.adapter_ambiguous`.
- Unsupported target identities emit
  `cem.lifecycle.target_adapter_unsupported`.
- Ambiguous target identities emit `cem.lifecycle.target_adapter_ambiguous`.

Gaps:

- Adapter selection currently returns `InputFormat` or `LayerFormat`, not an
  execution-facing adapter plan.
- Unsupported input and target identity currently fall back to the enum hint or
  fallback format with a warning. That preserves compatibility, but it conflicts
  with the content-identity-first goal when an author explicitly provides an
  unsupported content type or schema.
- The registry does not distinguish operation support such as validate, load,
  convert/export, inspect, trace, projection, and pass-through.
- Adapter diagnostics do not carry a normalized lifecycle preflight record with
  `matched`, `ambiguous`, `unsupported`, or `deferred` state.
- Adapter matching rules are embedded in code and tests, not captured as a
  stable contract for WASM or host integrations.

### Built-In Adapter Coverage

Current status: built-in adapters cover the immediate format families.

- CEM-ML identity selects `InputFormat::Cem` and `LayerFormat::Cem`.
- HTML and XHTML content types select `InputFormat::Html` and
  `LayerFormat::Html`.
- XML, SVG, and MathML content types select `InputFormat::Xml` and
  `LayerFormat::Xml`.
- Schema-only and namespace-only dispatch is covered for CEM core, transform
  config, native templates, CEM transform, HTML/XHTML/SVG/MathML, and XSLT.
- DOM, AST, and events projection target identities select JSON or binary
  projection formats.
- Tests assert that document content type takes precedence over conflicting
  projection or schema identities.

Gaps:

- SVG and MathML split between HTML namespace dispatch and XML package-schema
  dispatch. That may be correct, but the rule should be explicit in the shared
  contract because both are XML-family vocabularies.
- Projection adapters are target-only and map directly to internal projections;
  the contract should clarify that these are artifact/projection adapters, not
  document loaders.
- The registry does not yet expose adapter capability metadata, so callers
  cannot inspect supported identities without running selection.

### XSLT Custom-Element Compatibility

Current status: XSLT/custom-element compatibility is routed through lifecycle
input loading.

- `LegacyCustomElementXsltAdapter` matches legacy custom-element content types,
  standard XSLT content types, XSLT schema identity, and XSLT namespace identity
  when higher-priority identity fields are absent.
- The adapter lowers legacy/XSLT source to canonical CEM-ML bytes and then
  continues through the CEM tokenizer and AST pipeline.
- Unsupported legacy constructs surface warning diagnostics from the lowering
  layer.

Gaps:

- The adapter is input-only. Target/export behavior for custom-element XSLT is
  not modeled as a lifecycle export adapter.
- Legacy conversion diagnostics currently project code/message/URI, but not
  byte ranges, source-map stacks, or an explicit generated-from-XSLT boundary.
- The converted CEM-ML source becomes the parser input. Generated-node source
  maps back to the original XSLT/custom-element source are not yet a complete
  boundary contract.
- The adapter does not expose a declared compatibility profile or unsupported
  construct capability set through `LifecycleAdapter` metadata.

### Engine Execution Paths

Current status: the real engine runs document work through lifecycle loading.

- Parse, inspect, trace, validate, check, bench, and fixture flows call
  lifecycle load before running the parser/validation pipeline.
- Convert schedules `lifecycle-load`, `select-export`, and `convert` tasks,
  then runs the shared parser/AST pipeline and target export branch.
- Convert reports include scheduler trace, conversion metadata, target content
  type/schema, output kind, and output pipeline stages where available.
- HTML/XML export can use schema-package CEMT converters or direct CEMT output
  pipelines with Rust fallback behavior.
- Diagnostics are projected to input URIs after pipeline execution.

Gaps:

- Lifecycle load and target selection are engine-local runtime actions, not
  normalized run-plan preflight records.
- Convert target identity is consulted in `select_export`, but fallback
  `LayerFormat` still controls behavior when identity is absent or unsupported.
- `adapter_id` from input load and target selection is not carried into the
  public report AST.
- Report identity does not yet tie adapter selection, lifecycle preflight, and
  output artifact identity together across inputs and outputs.

### CLI Validate And Convert

Current status: CLI already feeds content identity into the engine for common
paths.

- Global `--content-type`, `--schema`, namespace options, module map, resolver
  maps, base URI, scope policy, and budgets lower into input root-scope
  defaults.
- Positional inputs infer content type from file extension when no explicit
  content type exists.
- Resolver reads can provide content type metadata, which fills missing input
  content type.
- `--to-content-type` and `--to-schema` lower into convert output root scope
  defaults.
- Per-output run-config identity overrides convert target defaults.
- Multi-output convert fans out output specs and passes per-output target
  identity and root scope into `ConvertRequest`.
- Tests cover content-type and schema identity selection for HTML, XHTML, XML,
  SVG, CEM, DOM/AST/events projections, unknown target identities, and legacy
  custom-element input conversion.

Gaps:

- `--from-format` and `--to-format` are still visible as format selectors
  rather than clearly documented compatibility hints.
- `--to-format` defaults to `dom-json`, so target identity must override a
  concrete enum fallback rather than replacing an absent selector.
- Unsupported target identity currently warns and preserves the `--to-format`
  fallback. The next contract should decide whether explicit unsupported target
  identity is a hard config/preflight error unless an explicit fallback mode is
  requested.
- Validate/check have no target identity, but their input identity behavior
  still allows unsupported identity fallback when `--from-format` is provided.
- CLI help text still describes `--content-type` and `--schema` as values to
  record on diagnostics/reports, even though they now participate in lifecycle
  dispatch.

### Diagnostics And Source Maps

Current status: diagnostics and source-map surfaces exist through the parser,
validation, transform, and output pipeline layers.

- Parser and validation diagnostics are merged after lifecycle load.
- Scheduler trace is reported for validation and convert flows.
- Convert responses can carry source maps, output spans, primary bytes, and
  conversion metadata.
- Output pipeline metadata reports formatter, colorizer, writer, content type,
  schema, category, and produced artifact kind.

Gaps:

- Lifecycle adapter diagnostics are not yet normalized with field paths or
  source ranges.
- Legacy XSLT lowering does not yet preserve original-source byte ranges in
  engine diagnostics.
- Adapter selection does not currently emit a report-visible lifecycle event or
  source-map boundary.
- There is no cross-host contract for how an adapter that generates CEM-ML,
  HTML, XML, or binary projection bytes must attach source-map frames.

## Recommended Resolution

Keep the existing `LifecycleRegistry` and its tests as the implementation
foundation, but define a stronger shared lifecycle adapter selection contract
before changing behavior.

The next contract should specify:

- an adapter descriptor with stable id, supported input identities, supported
  target identities, operations, projections, output artifact kinds, and
  source-map guarantees;
- lifecycle preflight records on normalized inputs and outputs:
  `matched`, `ambiguous`, `unsupported`, or `deferred`;
- exact precedence for content type, schema identity, namespace identity, and
  enum aliases;
- when unsupported explicit identity is a warning with fallback and when it is
  a hard pre-execution diagnostic;
- how `--from-format` and `--to-format` lower into provenance-only hints;
- report projection of input adapter id, target adapter id, selected
  operation, diagnostics, and fallback reason;
- XSLT/custom-element lowering boundaries, including source ranges and
  source-map frames from original legacy source into generated CEM-ML.

Recommended implementation order after the contract is accepted:

1. Add typed lifecycle adapter descriptors without changing current matching
   behavior.
2. Project lifecycle preflight into `NormalizedRunPlan` and `RunContext`.
3. Change CLI help and normalization metadata so content identity is visibly
   primary and enum flags are compatibility hints.
4. Tighten unsupported explicit identity behavior once fixtures document the
   desired fallback policy.
5. Extend XSLT/custom-element adapter diagnostics with source ranges and
   generated-boundary source-map frames.
