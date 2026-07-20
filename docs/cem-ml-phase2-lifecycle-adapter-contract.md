# CEM-ML Phase 2 Lifecycle Adapter Selection Contract

Status: accepted target contract for Phase 2 lifecycle adapter selection.
Implementation alignment is tracked in [`todo.md`](todo.md). This document
builds on the audit in
[`cem-ml-phase2-lifecycle-adapter-audit.md`](cem-ml-phase2-lifecycle-adapter-audit.md)
and the normalized run-config contract in
[`cem-ml-phase2-run-config-contract.md`](cem-ml-phase2-run-config-contract.md).

## Goals

- Make content identity the primary selection surface for validate, load, and
  export.
- Keep `--from-format` and `--to-format` as compatibility hints, not schema
  identity.
- Preserve the existing `LifecycleRegistry` shape while adding typed adapter
  descriptors and preflight records.
- Make unsupported and ambiguous identity decisions deterministic before
  document parsing or output writing.
- Carry adapter selection, fallback reason, diagnostics, and source-map
  guarantees into normalized run plans and reports.
- Keep host capabilities explicit. Host resolvers, plugin registries, WASM
  callbacks, filesystem access, and network access are runtime context, not
  serializable adapter descriptors.

## Definitions

**Content identity** is the normalized `FormatIdentity` used for lifecycle
selection:

```text
FormatIdentity {
  contentType?,
  schema?,
  defaultNamespace?,
  namespaces,
  baseUri?
}
```

**Lifecycle adapter** is the document or artifact boundary that can load an
input into the CEM AST/events spine, export a CEM artifact to a target, or
produce an internal projection.

**Compatibility alias** is a legacy enum such as `--from-format cem|html|xml`
or `--to-format cem|html|xml|dom-json|ast|events|dom-bin|ast-bin|events-bin`.
Aliases can supply hints and fallback defaults only when content identity is
absent or when an explicit fallback policy allows them.

**Lifecycle preflight** is the normalized, read-only selection result attached
to an input or output before execution.

## Adapter Descriptor

Every adapter exposed by `cem_ml` has a descriptor. Rust implementations may
derive these descriptors from trait methods, but the contract fields are stable
for normalized run plans, WASM host introspection, and report projections.

```text
LifecycleAdapterDescriptor {
  adapterId,
  family,
  version,
  operations[],
  inputIdentities[],
  targetIdentities[],
  projections[],
  outputArtifacts[],
  compatibilityAliases[],
  sourceMapContract,
  diagnostics,
  support: required | optional,
  provenance
}
```

`adapterId` is stable and lower-kebab-case. Built-in ids are:

```text
cem-ml
html
xml
custom-element-xslt-compat
dom-json-projection
dom-binary-projection
ast-projection
ast-binary-projection
events-projection
events-binary-projection
```

`family` groups related adapters such as `document`, `compatibility`,
`projection`, or `package-converter`. `version` is the adapter contract version,
not the document schema version.

`operations` contains one or more operation capabilities:

```text
validate
load
inspect
trace
bench
export
project
pass-through
lower-compatibility
```

An adapter that can validate/load input is not automatically an export adapter.
An adapter that can project DOM/AST/events is not a document loader unless it
declares a load operation.

## Identity Matchers

Adapter descriptors declare matchers rather than ad hoc string comparisons.

```text
LifecycleIdentityMatcher {
  contentTypes[],
  schemas[],
  namespaces[],
  contentTypeSchemaRules[],
  namespaceRules[],
  priority,
  selectorKind: document | artifact | projection | compatibility,
  requiresContentType?,
  requiresSchema?,
  requiresNamespace?,
  aliases[],
  provenance
}
```

Content type matching uses the MIME essence: lowercase type/subtype without
parameters. Authored spelling and parameters are preserved as provenance and
report data.

Schema matching uses canonical schema identity or accepted URI-only
compatibility projections, following the schema identity rules in
[`cem-ml-reference-normalization-design.md`](cem-ml-reference-normalization-design.md).

Namespace matching uses namespace values only. Namespace values are not schema
identity unless the adapter explicitly declares a namespace dispatch rule.

## Selection Precedence

Lifecycle selection is deterministic.

1. **Explicit content type is primary.** If `contentType` is present, adapters
   are selected from content-type matchers first.
2. **Content-type-specific schema rules refine selection only when declared.**
   If a content type is known and an adapter declares `contentTypeSchemaRules`,
   the schema can narrow or reject that content-type match.
3. **Schema-only selection applies only when content type is absent.** If no
   content type is present and `schema` is present, adapters are selected by
   schema identity.
4. **Namespace-only selection applies only when content type and schema are
   absent.** Namespace selection is a compatibility path for document families
   that expose root namespace identity before schema identity is known.
5. **Compatibility aliases are hints.** Aliases run only after identity
   selection has no explicit matched adapter or when identity is absent.
6. **Fallback requires policy.** An unsupported explicit identity cannot silently
   fall back to an alias unless the normalized request carries an explicit
   fallback policy.

When a content type and schema appear inconsistent, the adapter may still match
by content type if it does not declare a combined rule. The preflight must
record that schema was ignored for selection. A future adapter that declares a
combined rule may reject the same pair with a hard diagnostic.

## Preflight Record

Inputs and outputs in `NormalizedRunPlan` and `RunContext` carry lifecycle
preflight records.

```text
LifecyclePreflight {
  operation,
  direction: input | output,
  state: matched | ambiguous | unsupported | deferred,
  adapterId?,
  adapterFamily?,
  selectedFormat?,
  selectedArtifact?,
  matchedBy: contentType | contentTypeSchema | schema | namespace | alias | fallback | deferred,
  identity,
  ignoredIdentityFields[],
  aliasHint?,
  fallbackPolicy,
  fallbackReason?,
  diagnostics[],
  sourceMapContract?,
  provenance
}
```

`selectedFormat` is an internal compatibility projection such as `cem`, `html`,
`xml`, `dom-json`, `ast`, `events`, `dom-bin`, `ast-bin`, or `events-bin`.
It is not the primary public identity.

`deferred` is valid only when a host normalizes config before installing a full
adapter registry. Execution must resolve deferred selections before parsing,
loading, exporting, or writing output.

## Fallback Policy

Fallback policy is explicit.

```text
LifecycleFallbackPolicy {
  mode: reject | warn-and-use-alias | allow-missing-identity-alias,
  alias?,
  reason?,
  provenance
}
```

Default policy:

- missing identity may use the alias or command default;
- unsupported explicit input identity is a hard pre-execution diagnostic;
- unsupported explicit target identity is a hard pre-execution diagnostic;
- ambiguous identity is always a hard pre-execution diagnostic;
- warning fallback is allowed only for compatibility windows that record
  `warn-and-use-alias` provenance.

Current CLI behavior preserves some warning fallbacks for compatibility. The
target contract makes those fallbacks observable and temporary: each warning
fallback must carry `fallbackPolicy.mode = warn-and-use-alias` and a reason.

## Diagnostic Contract

Lifecycle diagnostics use stable codes.

```text
cem.lifecycle.adapter_matched
cem.lifecycle.adapter_ambiguous
cem.lifecycle.adapter_unsupported
cem.lifecycle.adapter_identity_conflict
cem.lifecycle.adapter_operation_unsupported
cem.lifecycle.adapter_deferred_unresolved
cem.lifecycle.alias_fallback_used
cem.lifecycle.target_adapter_ambiguous
cem.lifecycle.target_adapter_unsupported
cem.lifecycle.source_map_boundary_missing
```

Severity rules:

- `matched` diagnostics are normally omitted from human output but may appear
  in structured trace/report projections.
- `ambiguous`, `identity_conflict`, `operation_unsupported`, and unresolved
  `deferred` are hard diagnostics.
- `unsupported` is hard when identity was explicitly authored.
- `unsupported` may be warning only when identity was inferred and an explicit
  alias fallback policy exists.
- `alias_fallback_used` is warning unless the command requests strict
  lifecycle conformance.
- missing source-map boundary is warning during the Phase 2 compatibility
  window and hard once an adapter declares a required source-map guarantee.

Every lifecycle diagnostic should carry:

- input or output id;
- adapter ids considered, when available;
- normalized identity;
- field path such as `inputs[0].rootScope.defaultContentType` or
  `outputs[0].rootScope.schema`;
- source range when the config/parser exposes one;
- fallback policy and reason when fallback happens.

## Built-In Adapter Contract

### CEM-ML Document Adapter

`cem-ml` supports input `validate`, `load`, `inspect`, `trace`, and `bench`,
plus target `export`.

Input and target identities include:

- `application/cem+xml`;
- `application/cem`;
- `text/cem`;
- `text/cem-ml`;
- CEM schema-package, schema, native-template, transform, transform-config, and
  CEM core schema identities when content type is absent;
- CEM core namespace when content type and schema are absent.

It selects internal format `cem`.

### HTML Document Adapter

`html` supports input document load and target export for HTML-family
documents.

Input and target identities include:

- `text/html`;
- `application/xhtml+xml`;
- HTML and XHTML schema identities when content type is absent;
- HTML namespace when content type and schema are absent;
- SVG and MathML namespace compatibility when content type and schema are
  absent and the adapter is operating in HTML parsing mode.

It selects internal format `html`.

### XML Document Adapter

`xml` supports input document load and target export for XML-family documents.

Input and target identities include:

- `application/xml`;
- `text/xml`;
- `image/svg+xml`;
- `application/mathml+xml`;
- `application/mathml-presentation+xml`;
- `application/mathml-content+xml`;
- XML, SVG, and MathML package schema identities when content type is absent.

It selects internal format `xml`.

### Custom-Element XSLT Compatibility Adapter

`custom-element-xslt-compat` supports input `validate`,
`lower-compatibility`, and `load` for the inventory-backed XSLT 1.0 plus
limited EXSLT compatibility profile. It is separate from any future XSLT
3.0/4.0 peer-language engine.

Input identities include:

- `custom-element-xslt`;
- `text/custom-element-xslt`;
- `application/custom-element-xslt`;
- `text/x-custom-element-xslt`;
- `application/xslt+xml`;
- `text/xsl`;
- XSLT schema identity when content type is absent;
- XSLT namespace when content type and schema are absent.

It validates the original XSLT/custom-element source first, then selects
internal format `cem` after lowering to generated CEM-ML. Lifecycle diagnostics
carry the adapter id, profile, operation, generated CEM-ML identity, and
`generated-boundary` source-map contract in structured details.

Target/export behavior is not supported in Phase 2 unless a future adapter
declares it explicitly. A request to export to custom-element XSLT must produce
`cem.lifecycle.adapter_operation_unsupported`.

### Projection Adapters

Projection adapters are target-only unless explicitly extended later:

- `dom-json-projection` selects `dom-json`;
- `dom-binary-projection` selects `dom-bin`;
- `ast-projection` selects `ast`;
- `ast-binary-projection` selects `ast-bin`;
- `events-projection` selects `events`;
- `events-binary-projection` selects `events-bin`.

Projection adapters match CEM projection content types and projection schema
identities. They do not load documents or validate source syntax.

## CLI Alias Rules

CLI aliases lower into normalized provenance:

```text
--from-format cem  -> input aliasHint = cem
--from-format html -> input aliasHint = html
--from-format xml  -> input aliasHint = xml
--to-format cem    -> output aliasHint = cem
...
```

`--content-type`, `--schema`, `--default-namespace`, `--namespace`,
`--to-content-type`, and `--to-schema` are lifecycle identity fields, not
display-only report metadata.

`--from-format` is legal with content identity only as a compatibility hint. If
the hint conflicts with a matched adapter, the matched adapter wins and the
preflight records an ignored alias. In strict lifecycle mode this conflict may
be promoted to a hard diagnostic.

`--to-format` is legal with target identity only as a fallback hint. If the
target identity matches an adapter, the adapter wins. If target identity is
explicit and unsupported, fallback is rejected by default.

Multiple output specs use each output's root-scope identity independently.
Global `--to-content-type`, `--to-schema`, and `--to-format` are defaults only.

## Reports

Structured reports should project lifecycle selection for every input and
output.

```text
LifecycleReport {
  inputs[]: LifecycleInputReport,
  outputs[]: LifecycleOutputReport
}

LifecycleInputReport {
  inputId,
  uri,
  identity,
  state,
  adapterId?,
  matchedBy,
  selectedFormat?,
  aliasHint?,
  fallbackReason?,
  diagnostics[]
}

LifecycleOutputReport {
  outputId,
  inputId,
  destination?,
  identity,
  state,
  adapterId?,
  matchedBy,
  selectedFormat?,
  selectedArtifact?,
  aliasHint?,
  fallbackReason?,
  diagnostics[]
}
```

Human reports may summarize only non-matched outcomes. JSON/CEM reports should
preserve complete preflight records once the normalized run plan exposes them.

Report identity must include selected adapter ids, matched operation, fallback
policy, and lifecycle diagnostics count. It must not include input bytes,
output bytes, wall-clock time, or host callback identity.

## Source-Map Contract

Adapters declare one source-map contract:

```text
none
preserve-input
generated-boundary
full-stack
```

- `none`: adapter does not claim source-map preservation.
- `preserve-input`: adapter passes parser source ranges through unchanged.
- `generated-boundary`: adapter creates generated output and must attach a
  frame from generated bytes to the source range or adapter operation that
  generated them.
- `full-stack`: adapter preserves source ranges through every generated node or
  emitted output span.

Phase 2 target for built-ins:

- CEM-ML, HTML, and XML document adapters: `preserve-input`;
- DOM/AST/events projections: `preserve-input`;
- binary projections: `preserve-input`;
- custom-element XSLT compatibility: `generated-boundary` now, `full-stack` later for
  lowered nodes that can be traced to original XSLT/custom-element source.

When an adapter lowers one language to another, diagnostics from the generated
language must preserve an adapter boundary frame that identifies:

- original URI;
- original source range when known;
- adapter id;
- operation;
- generated content identity;
- generated source range when known.

## Normalized Run-Plan Integration

`NormalizedInput.lifecyclePreflight` and
`NormalizedOutput.lifecyclePreflight` carry the preflight records defined here.

`RunContext` should expose those records per document and output so build/CI,
WASM, and CLI hosts can inspect lifecycle decisions before running document
work.

Execution paths may continue to call the existing `LifecycleRegistry`, but the
observable result must be equivalent to the normalized preflight record:

```text
normalize config and aliases
  -> build effective input/output identities
  -> lifecycle preflight with registry when available
  -> execute matched/deferred adapter
  -> project lifecycle reports and diagnostics
```

If a host normalizes without a registry, it records `deferred`. The host must
resolve deferred records before execution or emit
`cem.lifecycle.adapter_deferred_unresolved`.

## Implementation Boundary

The first implementation slice should be additive:

1. Add typed lifecycle descriptor structs and descriptor methods for built-in
   adapters.
2. Add preflight result structs matching this contract.
3. Populate preflight records in `NormalizedRunPlan` and `RunContext`.
4. Keep current runtime matching and CLI behavior until fixtures are updated.
5. Record current warning fallbacks as explicit fallback policy.

Behavior tightening should happen only after CLI fixtures cover identity-first
selection, enum alias fallback, unsupported identity rejection, and XSLT
source-map boundary behavior.
