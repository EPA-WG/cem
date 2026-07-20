# CEM-ML Phase 2 Run-Config And Lifecycle Audit

Status: completed audit for the first Phase 2 parser/runtime checklist item.
This document records current `cem_ml` and `cem_ml_cli` surfaces against the
roadmap's run-configuration and lifecycle contract. It does not change runtime
behavior.

## Phase 2 Target

Phase 2 requires one schema-defined document runtime contract shared by the
Rust library, CLI, WASM, and future hosts:

- every supported input follows validate, load into internal AST/events, then
  export;
- format identity is content type plus schema or namespace identity;
- every document root has a scope-zero context with content type, schema,
  version pins, namespace bindings, module-map/resolver identity, base URI,
  scope policy, and budgets;
- APIs accept input and output spec arrays;
- CLI config files and repeatable CLI records lower to the same normalized
  `RunConfig`;
- multi-document runs preserve per-document diagnostics, source maps, resource
  accounting, and scheduler boundaries.

## Current Surfaces

### `cem_ml::run_config`

Current status: partially implemented and usable as the shared authored config
shape.

- `RunConfig` is a serializable array-shaped model with `inputs`, `outputs`,
  `schemaPackages`, `resolvers`, and `scheduler`.
- `InputSpec` and `OutputSpec` both carry a `rootScope`; `OutputSpec` also
  carries `inputRef` and `destination`.
- `ScopeConfig` carries default content type, schema, output-color/CEMT
  selectors, version pins, default namespace, named namespaces, module map,
  base URI, policy, and string-keyed budgets.
- `parse_run_config` is owned by `cem_ml`, accepts bytes plus
  `FormatIdentity`, validates the run-config schema or namespace identity, and
  currently supports JSON only.
- `normalize_run_config` applies CLI/lib defaults, resolves relative module-map
  and output-destination paths against the config base, infers content types
  from file extensions, and fills schema-package manifest identity defaults.
- CSV-style `parse_input_spec_record` and `parse_output_spec_record` exist and
  lower CLI records into the same `InputSpec`/`OutputSpec` structs.
- The JSON Schema artifact
  `packages/cem_ml/schema/cli/run-config.schema.json` matches the current
  serializable surface.

Gaps:

- There is no separate normalized-run IR that distinguishes authored config
  fields from normalized execution fields, derived defaults, and provenance.
- Config diagnostics are deterministic but coarse: they attach the config URI,
  not per-field source ranges or structured field paths.
- Config parsing supports JSON only; CEM-native/YAML/CSV config files remain
  future content-type adapters.
- `ScopeConfig.policy` and `ScopeConfig.budgets` are still stringly typed at
  the config boundary. Runtime code parses selected aliases later instead of
  normalizing typed budget and policy values in one place.

### CLI Option Surface

Current status: broad command plumbing exists and lowers into `RunConfig`
defaults.

- `RunOptions` exposes `--config`, `--config-content-type`,
  `--config-schema`, repeatable `--input-spec`, and repeatable
  `--output-spec`.
- `ContextOptions` exposes input root-scope defaults: `--content-type`,
  `--schema`, `--default-namespace`, repeatable `--namespace`,
  `--module-map`, repeatable `--version-pin`, `--scope-policy`,
  repeatable `--scope-budget`, `--base-uri`, resolver read/write maps, and
  schema-package manifests.
- `ConvertArgs` adds target identity and output-stage selectors:
  `--to-content-type`, `--to-schema`, output color type, CEMT formatter and
  colorizer selectors, `--out`, and `--artifact-json`.
- CLI dispatch reads config files through resolver-aware config reads, appends
  `--input-spec` and `--output-spec`, then calls `cem_ml::run_config`
  normalization.
- CLI resolver maps and run-config `resolvers[]` register local mirror
  read/write resolvers by purpose.

Gaps:

- CLI command options are the effective compatibility surface, but the
  normalized contract does not yet describe which options are aliases, which
  are defaults, and which are forbidden together.
- CLI record parsing is useful for one-liners, but the stable CSV grammar and
  diagnostics are not yet documented as a config projection.
- Config and command-line defaults can still be reasoned about only by reading
  dispatch code; the normalized contract should define precedence explicitly.

### Engine Request Surface

Current status: command-specific request structs are implemented, but no single
normalized run-plan API exists.

- `EngineInput` carries URI, bytes, optional legacy `from_format`, optional
  `FormatIdentity`, and `ScopeConfig`.
- `ParseRequest`, `InspectRequest`, `TraceRequest`, and `ConvertRequest`
  operate on one `EngineInput`.
- `ValidateRequest`, `CheckRequest`, `BenchRequest`, and fixture validation
  accept input arrays.
- `ConvertRequest` carries target identity, target scope, and a scheduler scope
  id; CLI dispatch fans out multiple configured outputs into repeated
  per-output `ConvertRequest` executions.
- `EngineContext` carries global schema/content/base URI defaults, scheduler
  config, registries, schema-package manifests, resolver registry, and template
  adapters.

Gaps:

- The library does not yet expose one `RunConfig` or `RunPlan` execution API
  that preserves config-level input/output graph identity through parse,
  validate, convert, trace, and report.
- Multi-output convert exists in CLI dispatch, not as a first-class library
  run context.
- `EngineContext` duplicates some root-scope identity fields; the next design
  should specify which fields remain host/global defaults and which must live
  only on normalized document scopes.

### Lifecycle And Adapter Dispatch

Current status: the core adapter boundary is present.

- `LifecycleRegistry` owns input load and target export selection.
- Built-in adapters cover CEM-ML, HTML, XML, legacy custom-element XSLT, and
  DOM/AST/events projections.
- Adapter matching uses content type, schema, and namespace identity, with
  unsupported or ambiguous identities reported as lifecycle diagnostics.
- `RealCemMlEngine` routes parse, inspect, trace, validate, check, bench, and
  convert through lifecycle load and the internal pipeline.

Gaps:

- The lifecycle registry is selected inside the real engine rather than being
  modeled as an explicit normalized run-plan stage.
- Adapter diagnostics are runtime outcomes; the normalized config contract does
  not yet say which identity combinations are allowed, ambiguous, or
  unsupported before execution.
- XSLT compatibility is implemented as an adapter, but the broader Phase 2
  adapter registry contract is not yet represented in `RunConfig`.

### Resolver, Module Map, Policy, And Budgets

Current status: resolver and scheduler primitives exist, with partial
run-config integration.

- Resolver requests carry purpose, direction, base URI, and content-type hint.
- CLI and run-config resolver specs register local mirror resolvers for read,
  write, and list support by scheme and purpose.
- Root module maps are loaded in the real engine from local paths, file URIs,
  or registered resolvers and produce schema/module alias maps.
- `SchedulerConfig` carries thread-pool and max-parallel-document settings.
- `ScopePolicy` has typed runtime caps for CPU workers, queue size, IO streams,
  memory bytes, plugin time budget, and overflow policy.
- Runtime budget parsing recognizes specific budget aliases such as parse,
  validate, check, convert, trace, inspect, bench, fixture, observe, CPU,
  queue, IO, memory, plugin time, and overflow.

Gaps:

- Run-config budget and policy values are not normalized into typed
  `ScopePolicy` values before execution.
- Some budgets are enforced as wall-clock diagnostics in command-specific
  paths; other recognized or unknown budgets are preserved, warned, or parsed
  later.
- Resolver provenance, effective base URI, module-map identity, and policy
  decisions are not yet projected as one normalized scope identity record.

### Reports And Source Maps

Current status: report and source-map surfaces are present but not tied to a
single run-plan identity.

- Reports are deterministic and include scheduler trace where command paths
  attach it.
- Diagnostics preserve URIs and source maps from parser/validation execution.
- Convert and transform responses can carry source-map stacks, output spans,
  primary bytes, conversion metadata, and scheduler trace.

Gaps:

- Run-config diagnostics do not carry per-field source maps.
- Multi-document run reports do not yet expose one normalized run id with
  per-input/per-output scope identities and resource accounting.
- Output specs do not yet define a stable report identity for all generated
  artifacts across library, CLI, and WASM hosts.

## Recommended Resolution

Do not start by rewriting execution. The next work item should freeze the
normalized `RunConfig` contract and make it explicit that `RunConfig` has two
layers:

- authored config surface: JSON and CLI record fields as users provide them;
- normalized run plan: resolved input/output specs, effective root scopes,
  typed policy/budget values, resolver/module-map identities, alias/default
  provenance, and deterministic per-document/per-output execution ids.

The design should keep current CLI flags as compatibility aliases and keep
`cem_ml` as the owner of parsing and normalization. CLI, WASM, and Rust hosts
should provide raw config bytes or raw option records plus config identity, then
consume the normalized run plan.

The implementation order should be:

1. Define the normalized `RunConfig`/run-plan data contract in docs and, if
   needed, a separate Rust IR alongside the existing serializable structs.
2. Add source/provenance fields for normalized config diagnostics without
   changing existing CLI output defaults.
3. Normalize scope policy and budgets once, then have parse/validate/check/
   convert/trace/bench consume those typed values.
4. Move multi-output convert and multi-document accounting toward a library
   run-plan execution boundary after the contract is stable.
