# CEM-ML Phase 2 Normalized RunConfig Contract

Status: accepted target contract for the Phase 2 run-configuration and
multi-document lifecycle work. Implementation alignment is tracked in
[`todo.md`](todo.md).

This document defines the normalized run-configuration contract shared by
`cem_ml`, `cem_ml_cli`, WASM, and future hosts. It builds on the current surface
audit in
[`cem-ml-phase2-run-config-audit.md`](cem-ml-phase2-run-config-audit.md) and
the CLI/runtime requirements in
[`cem-ml-cli-contract.md`](cem-ml-cli-contract.md).

## Goals

- Keep `cem_ml` responsible for config parsing, validation, normalization, and
  defaulting.
- Preserve the existing JSON `RunConfig` and CLI flags as compatibility
  surfaces while introducing an explicit normalized run plan.
- Make root-scope context deterministic for every input, output, and
  schema-package manifest.
- Carry source/provenance for defaults, aliases, resolver choices, and
  diagnostics without changing authored config data.
- Normalize policy and budget values once, then let parse, validate, check,
  convert, trace, bench, fixture, and observability paths consume typed values.
- Keep host-owned capabilities out of the serializable config. Resolver objects,
  scheduler implementations, filesystem handles, network policies, and WASM
  host callbacks live in runtime context.

## Two Layers

`RunConfig` has two separate layers.

The authored config surface is what users write or pass on the command line:

- JSON config files with schema identity
  `https://cem.dev/ns/cli/run-config/1`;
- repeatable CLI records such as `--input-spec` and `--output-spec`;
- command-level defaults such as `--content-type`, `--schema`,
  `--default-namespace`, `--namespace`, `--module-map`, `--base-uri`,
  `--scope-policy`, `--scope-budget`, `--to-content-type`, and `--to-schema`;
- compatibility aliases such as `--from-format` and `--to-format`.

The normalized run plan is the execution-facing result of parsing, merging,
defaulting, validating, and resolving pure path-like values:

- deterministic run, input, output, schema-package, resolver, and scope ids;
- effective input/output/schema-package root scopes;
- normalized format identities;
- config provenance and compatibility alias provenance;
- typed scheduler, scope policy, and budget values;
- resolver and module-map identity records;
- diagnostics mode, report destinations, and observability destinations;
- pre-execution diagnostics with config field paths and source ranges when
  available.

The normalized run plan may be represented by Rust IR such as `NormalizedRunPlan`
alongside the existing serializable `RunConfig` structs. It does not replace the
JSON schema surface.

## Normalization Pipeline

Config normalization runs before document parsing.

1. **Read authored config source.** A host supplies raw config bytes or raw
   option records plus config `FormatIdentity`. CLI may infer
   `application/json` from `.json`; it still delegates parsing to `cem_ml`.
2. **Parse authored surface.** JSON config files parse into the authored
   `RunConfig` shape. CLI records parse into `InputSpec` and `OutputSpec`.
   Future CEM-native, YAML, or CSV config files are content-type adapters that
   produce the same authored shape.
3. **Merge compatibility inputs.** Command-level options become defaults.
   Config-file specs keep explicit values. CLI `--input-spec` and
   `--output-spec` append additional specs and use the same defaulting rules.
4. **Build effective scopes.** Each input, output, and schema-package manifest
   gets its own root scope. Explicit per-spec fields override command defaults;
   missing fields inherit defaults.
5. **Normalize pure values.** The plan resolves config-relative local
   module-map and output-destination paths, infers content type from known file
   extensions when no content type is declared, validates namespace/version-pin
   shape, validates resolver spec shape, and assigns deterministic ids.
6. **Resolve execution context.** With a host resolver registry available, the
   plan can materialize resolver identities, module-map identities, scheduler
   policy values, and lifecycle adapter preflight outcomes. Unsupported
   capabilities are diagnostics, not silent fallbacks.
7. **Execute command profile.** Parse, validate, check, convert, trace, bench,
   fixture, transform, and observe flows consume the normalized plan. Command
   profiles select which inputs and outputs are required, which report format is
   primary, and which output destinations are legal.

## Data Contract

The normalized run plan is conceptual. Field names below are stable contract
names; Rust structs may use idiomatic names.

```text
NormalizedRunPlan {
  runId,
  commandProfile,
  configIdentity,
  authoredSources[],
  inputs[],
  outputs[],
  schemaPackages[],
  resolvers[],
  scheduler,
  diagnosticsMode,
  provenance,
  diagnostics[]
}
```

### Run Identity

`runId` is deterministic for the normalized plan. It includes normalized config
structure, effective scopes, declared input/output/spec URIs, resolver bindings,
module-map identities when materialized, scheduler settings, and command
profile. It excludes input bytes, output bytes, wall-clock time, cache hits,
absolute temporary file handles, and live host callback identities.

Input and output ids are order-stable:

```text
inputId:         "input:<index>"
outputId:        "output:<index>"
schemaPackageId: "schemaPackage:<index>"
resolverId:      "resolver:<purpose>:<scheme>:<index>"
scopeId:         "scope:<kind>:<index>"
```

These ids are diagnostic/report identities. They do not replace authored URIs
or resolved URIs.

### Config Identity

```text
ConfigIdentity {
  declaredUri?,
  resolvedUri?,
  contentType,
  schemaIdentity,
  namespaceIdentity?,
  sourceKind: file | file-uri | custom-uri | bytes | cli-records | host-object,
  sourceRange?
}
```

`schemaIdentity` for the current JSON config surface is
`https://cem.dev/ns/cli/run-config/1`. Unknown config schema or namespace
identity is a config diagnostic before document parsing.

### Inputs

```text
NormalizedInput {
  inputId,
  declaredUri,
  resolvedUri?,
  byteSourceKind,
  fromFormatHint?,
  identity,
  rootScope,
  lifecyclePreflight?,
  provenance,
  sourceRange?
}
```

`identity` is the normalized `FormatIdentity` for lifecycle dispatch:
content type, schema identity, default namespace, named namespace map, and base
URI projection. `fromFormatHint` exists only for compatibility aliases such as
`--from-format cem|html|xml`; explicit content type, schema, or namespace
identity is authoritative.

Library and WASM callers may supply bytes or streams instead of URI-backed
inputs. The normalized plan still records a declared input identity and
byte-source kind, but it must not serialize live streams or host handles.

### Outputs

```text
NormalizedOutput {
  outputId,
  inputId,
  declaredDestination?,
  resolvedDestination?,
  toFormatFallback?,
  identity,
  rootScope,
  primaryOutputPolicy,
  sidecars[],
  lifecyclePreflight?,
  provenance,
  sourceRange?
}
```

`inputId` is explicit when authored through `inputRef`; when one input exists,
an omitted `inputRef` binds to that sole input. Ambiguous or unknown input refs
are config diagnostics. `toFormatFallback` exists only for compatibility
aliases such as `--to-format`; target content type, schema, or namespace
identity is authoritative for export selection.

Global `--out` is valid only for command shapes with one effective output.
Multiple configured outputs must use per-output `destination` values for
primary target-native bytes. Debug artifacts such as `--artifact-json` are
sidecars, not primary outputs.

### Root Scope

Every input, output, and schema-package manifest has an effective root scope.

```text
NormalizedRootScope {
  scopeId,
  direction: input | output | schemaPackage,
  identity,
  defaultNamespace?,
  namespaces,
  versionPins,
  baseUri?,
  resolverContext,
  moduleMap?,
  policy,
  budgets,
  outputPipeline?,
  provenance
}
```

`identity` includes content type and schema identity. Namespace bindings seed
schema validation's document-root namespace context; they are not a substitute
for schema identity unless a lifecycle adapter explicitly declares namespace
dispatch.

`baseUri` is the effective base for diagnostics/report projection and for
relative resource specifiers whose resolver purpose uses the root scope. A
config-relative path normalization step may derive a resolved local path or URI,
but it must preserve the authored spelling as provenance.

`outputPipeline` carries output-only selectors such as output color type, CEMT
formatter, formatter profile, CEMT colorizer, and color profile. These fields do
not participate in input lifecycle dispatch.

### Resolver Context

```text
ResolverBinding {
  resolverId,
  scheme,
  purposes[],
  directions[],
  declaredUriPrefix,
  resolvedLocalRoot?,
  support: required | optional,
  provenance
}
```

The serializable run config may declare resolver mappings, but live resolver
objects are host runtime capabilities. CLI local mirror maps are one resolver
binding implementation. WASM and Rust hosts may register other resolvers by
scheme, purpose, and direction.

Resolver purposes remain explicit: `config`, `input`, `template`,
`moduleMap`, `output`, `report`, and `observeEvents`. A resolver registered for
one purpose or direction must not silently authorize another.

### Module Map Identity

```text
ModuleMapIdentity {
  declaredUri,
  resolvedUri?,
  contentType,
  entriesHash?,
  resolverId?,
  baseUri?,
  state: valid | missing | invalid | unreadable | unsupported,
  diagnostics[],
  provenance
}
```

Module-map identity is part of the root scope. It is not a global hidden input.
Relative module-map paths resolve against the config document base when they
come from config files, or against the command/host base when they come from
command defaults.

Loading a module map is a resolver-backed preflight step. Malformed,
unreadable, or unsupported module maps produce deterministic diagnostics and do
not silently collapse to an empty alias map.

### Policy And Budgets

```text
NormalizedScopePolicy {
  policyName?,
  cpuWorkers,
  queueSize,
  ioStreams,
  memoryBytes,
  pluginTimeBudgetMs?,
  overflow: block | reject | spillToParent,
  provenance
}

NormalizedBudgets {
  timeoutMs?,
  stackDepth?,
  parseMs?,
  validateMs?,
  checkMs?,
  convertMs?,
  traceMs?,
  inspectMs?,
  benchMs?,
  fixtureValidateMs?,
  fixtureRoundtripMs?,
  observeMs?,
  pluginMs?,
  memoryBytes?,
  unknown[]
}
```

The authored `ScopeConfig.budgets` map remains string-keyed for compatibility,
but the normalized run plan must parse recognized aliases once and expose typed
values. Unknown budget names are preserved in provenance and reported through a
stable diagnostic unless the host has declared a future extension.

Typed policy values feed scheduler worker pools, queue limits, IO limits, and
budget diagnostics. `timeoutMs` is the general active-time scope deadline;
operation-specific time fields constrain their corresponding child execution
scopes. `stackDepth` is the logical engine-frame limit. Their aliases,
inheritance, enforcement, and failure behavior are canonicalized in
[`cem-ml-operation-control-design.md`](cem-ml-operation-control-design.md).
Command paths must not each invent their own parsing rules.

### Diagnostics Mode

```text
DiagnosticsMode {
  failLevel: parse | validate | strict,
  primaryKind: report | content,
  reportProjection: text | json | xml | cem | html | markdown,
  reportDestinations[],
  observeEventsDestination?,
  quiet,
  verbose,
  noColor
}
```

Diagnostics mode is run-plan metadata, not document identity. Validation-style
commands are report-primary. Parse/convert/load/save-style commands are
content-primary unless a command explicitly asks for a report projection.

Config diagnostics are emitted before document parsing. They should carry:

- stable diagnostic code;
- severity;
- message;
- config URI or CLI-record source;
- field path such as `inputs[0].rootScope.moduleMap`;
- source range when the config parser exposes one;
- normalized id when available.

Compatibility output may continue to project only the current URI/message
fields while structured report projections gain field paths and source ranges.

## Defaulting And Precedence

Defaulting is deterministic:

1. Host defaults provide the baseline scheduler, resolver registry, and root
   scope defaults.
2. Command-level context options become input root-scope defaults.
3. Convert target options become output root-scope defaults.
4. Authored config-file specs override missing default fields for their own
   spec only.
5. Repeatable CLI input/output spec records append specs after config-file
   specs and follow the same override rules.
6. Per-spec `rootScope` fields override command defaults.
7. Inferred content type fills only a missing content type. It must be recorded
   as inferred provenance.
8. Schema defaults derived from content-type registry lookup fill only a
   missing schema identity. They must be recorded as registry-derived
   provenance.

Conflicts are explicit. Providing both a compatibility alias and a schema-owned
identity is legal only when the alias can be treated as a hint or fallback. If
two authored fields claim the same normalized field with different explicit
values, normalization emits a config diagnostic before document parsing.

## Lifecycle Preflight

Lifecycle adapter resolution is a read-only preflight outcome in the normalized
plan when the required registry is available:

```text
LifecyclePreflight {
  adapterId?,
  state: matched | ambiguous | unsupported | deferred,
  diagnostics[],
  provenance
}
```

`deferred` is valid for hosts that normalize config before installing a full
adapter registry. Execution must resolve deferred lifecycle choices before
parsing, loading, or exporting documents.

Explicit content type is authoritative for adapter selection. Schema and
namespace identities participate only when no content type is present or when a
versioned adapter declares that combined identity rule.

## Compatibility Aliases

Compatibility aliases remain supported, but they lower into explicit
provenance:

- `--from-format cem` is an input format hint for CEM syntax;
- `--from-format html` is an input format hint for HTML syntax;
- `--from-format xml` is an input format hint for XML syntax;
- `--to-format` values are output projection or target fallback hints;
- legacy report aliases such as `--report-json` and `--report-md` lower to
  `DiagnosticsMode.reportProjection` plus destination provenance.

Aliases do not silently override content type, schema identity, namespace
identity, or explicit output spec fields.

## Implementation Boundary

The next implementation slice should add or align library normalization APIs
without changing command behavior:

```text
parse authored config/records
  -> validate authored shape
  -> normalize defaults and pure values
  -> produce NormalizedRunPlan + config diagnostics
  -> existing command-specific requests consume the plan
```

Execution unification should come later. First make the normalized plan
observable in tests while preserving current CLI output compatibility.
