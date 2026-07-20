# CEM-ML Declarative Reference Vocabulary

Status: accepted target design for schema-owned cross-node reference
constraints; implementation pending.

This note defines the CEM surface vocabulary for schema-owned reference checks
that are currently declared in CEM-ML but still partly executed by Rust. It
builds on the normalized value model in
[`cem-ml-reference-normalization-design.md`](cem-ml-reference-normalization-design.md)
and the comparison operators in
[`cem-ml-reference-comparison-design.md`](cem-ml-reference-comparison-design.md).
The current implemented schema-package surface remains documented in
[`../packages/cem_ml/schema-packages/schema-package/v1/README.md`](../packages/cem_ml/schema-packages/schema-package/v1/README.md).

The vocabulary is centered on `schema:reference-resolution` constraints. It
lets schemas declare candidate selection, operand extraction, lookup, execution
requirements, comparison, and diagnostic projection without introducing
package-specific syntax for schema-package converter endpoints or artifact
metadata checks.

`schema:reference-resolution` is an orchestration and compatibility behavior.
Its target stages are normalization annotation, optional lookup, comparison,
then diagnostic projection. The behavior does not directly convert
normalization outcomes into violations; operand state policy, comparison
presence policy, and comparison operators decide assertion results before
projection formats diagnostics.

## Goals

- Keep candidate selection declarative and based on a narrow pure CEM-QL
  selector profile.
- Declare actual, expected, and forbidden operands with stable diagnostic
  identities.
- Keep normalization, lookup, state policy, and comparison as separate steps.
- Make engine-assisted behavior explicit through named capabilities.
- Preserve current CLI/report compatibility while adding structured projection
  metadata.
- Keep diagnostics actionable: public value buckets point at the value the user
  can inspect or fix, and structured metadata explains the relationship.

## Constraint Shape

A reference constraint is composed from these pieces:

- `candidates`: selects and names document nodes to validate.
- `actual`, `expected`, `forbidden`: declare normalized operands.
- `lookup`: resolves document or engine-assisted references.
- `compare`: declares the comparison operator.
- `projection`: declares diagnostic detail and source-range behavior.
- Constraint execution fields: declare pure versus engine-assisted execution,
  required capabilities, support policy, and package context.

Example:

```cem
{constraint
    @kind="endpoint-content-type-schema"
    @behavior="schema:reference-resolution"
    @execution="engine-assisted"
    @requires="schema:engine.registry"
    @package="current" |

    {candidates
        @select="$target.child(from, to)"
        @as="endpoint"
        @cardinality="zero-or-more"
        @on-empty="pass"}

    {actual
        @binding="content-type"
        @from="endpoint.@content-type"
        @normalizer="schema:content-type-identity"
        @cardinality="one"
        @shape="scalar"
        @state="required-valid"}

    {expected
        @binding="content-type"
        @from="endpoint.@schema"
        @normalizer="schema:content-type-identity-set"
        @cardinality="one"
        @shape="scalar"
        @result-cardinality="set"
        @result-shape="scalar"
        @state="required-valid" |

        {lookup
            @name="schema:registry.descriptor"
            @execution="engine-assisted"
            @requires="schema:engine.registry"
            @result="contentTypes"
            @result-cardinality="one"
            @result-shape="record"
            @source-range="key" |

            {key
                @binding="schema"
                @normalizer="schema:schema-identity"}
        }}

    {compare @operator="schema:member-of"}

    {projection
        @project="expected invalid missing unresolved comparison source-ranges"
        @value-view="raw"
        @source="operand"}
}
```

## Candidate Selection

Candidate selection uses a constrained CEM-QL selector with explicit
host-provided bindings. Do not add a separate `@scope` field. Scope and
authority belong to `QueryContextScope`; tree navigation belongs to CEM-QL
bindings and axes.

Candidate fields:

- `@select`: CEM-QL-compatible selector expression.
- `@as`: required stable candidate binding name.
- `@cardinality`: `zero-or-more`, `optional`, `one-or-more`, or `exactly-one`.
- `@on-empty`: `pass`, `missing`, or `unmatched`.

Selector bindings:

- `$target`: node selected by the constraint's `@target`.
- `$document`: current validated document root.
- Additional host bindings may be supplied as named data objects, not selector
  scopes.

Allowed traversal:

- Child and descendant traversal may start from `$target` or `$document`.
- Sibling and ancestor traversal may start from `$target` or another selected
  node binding.
- Document-global selection starts explicitly from `$document`.
- Candidate traversal is read-only, deterministic, and limited to in-memory
  document/context bindings.

Candidate ordering is diagnostic-stable. Validators preserve selector order and
must not apply an extra sort after candidate selection. Ordering becomes
semantic only when the selector uses explicit order-sensitive operations such as
`.first()`, `.last()`, or `.nth()`.

Candidate cardinality:

```text
zero-or-more -> 0..n
optional     -> 0..1
one-or-more  -> 1..n
exactly-one  -> 1
```

Empty candidate selection:

- `pass`: empty selection is valid.
- `missing`: a required candidate node is absent.
- `unmatched`: the selector failed to match the required document shape.

Defaults:

```text
zero-or-more -> @on-empty="pass"
optional     -> @on-empty="pass"
one-or-more  -> @on-empty="missing"
exactly-one  -> @on-empty="missing"
```

More candidates than allowed by `optional` or `exactly-one` produce an
`unmatched` candidate-cardinality diagnostic, not operand `invalidValues`.
Reserve `mismatch` for comparison failures after a candidate exists.

Candidate `@as` is the local candidate identity. It is required, unique within
the constraint, and intentionally excluded from attribute/child parity. Operand
field names still come from operand `@binding`, not candidate `@as`.

## Operands

Operands are declared with role elements:

- `actual`: observed value from the validated document or selected candidate.
- `expected`: required/reference value.
- `forbidden`: disallowed reference value.

Operand fields:

- `@binding`: required canonical diagnostic/value identity.
- `@from`: constrained source path relative to a candidate or lookup binding.
- `@normalizer`: named normalizer for the final comparable value. When the
  operand declares a lookup, lookup keys use their own key normalizers.
- `@cardinality`: `one`, `optional`, or `set`; conceptual `sequence` is
  reserved but not active in the initial package-check surface.
- `@shape`: `scalar` or `record`.
- `@result-cardinality`: final comparable result cardinality when it differs
  from source cardinality.
- `@result-shape`: final comparable result shape when it differs from source
  shape.
- `@state`: state policy.
- `@lookup`: simple inline lookup shorthand.
- `@diagnostic-field`: optional display/report alias. Canonical diagnostics
  still use `@binding`.

`@binding` is required on every operand role element and is the stable public
identity of the operand. It must be unique among operands of the same role
unless a future grouped operand design explicitly allows repeated bindings.
For lookup-based operands, `@binding` names the comparable result, not the
lookup key. Lookup key bindings are provenance identities only.

Attribute/child parity applies to `from`, `normalizer`, `cardinality`, `shape`,
`state`, `lookup`, `projection`, and potentially `compare`. It does not apply
to operand `@binding` or candidate `@as`; those identify the declaration
itself.

Use child form when the declaration needs multiple values, nested options,
source-range override, capability requirements, `@support` policy, projection
override, or future extension.

## Operand Source Paths

`@from` uses a constrained source path rooted at a candidate binding or lookup
binding. It is not full CEM-QL. Selection and filtering belong in
`candidates @select`; registry/resource/package resolution belongs in `lookup`;
comparison logic belongs in `compare`.

Allowed forms:

```text
binding
binding.@attribute
binding.name
binding.text
binding.child(element)
binding.child(element).@attribute
lookup-binding
lookup-binding.field(field-name)
```

Disallowed forms include arbitrary CEM-QL predicates, `.where(...)`,
`.map(...)`, `.first()`, `read(...)`, imports, function calls, computed joins,
and cross-document traversal.

Path lexical grammar:

- Bare names use `[A-Za-z_][A-Za-z0-9_-]*`.
- Qualified names use `prefix:local`, where both parts use the bare-name
  grammar.
- Attribute names are prefixed with `@`.
- QName prefixes resolve through the active constraint/schema namespace
  context; unknown prefixes are schema-definition errors.
- Quoted single-string names match literal lexical names.
- Quoted pair names match expanded `{namespace-uri, local-name}` identity.

Supported escaped forms:

```text
["literal-name"]
@["literal-attribute-name"]
child(["literal-element-name"])
child(["namespace-uri", "local-name"])
@["namespace-uri", "local-name"]
```

`binding.child(x)` returns all direct matching child elements in document order.
It never silently chooses the first match. Multiple direct children for
`@cardinality="one"` are `invalid`; zero results are `missing` unless `@state`
allows absence. Deeper traversal belongs in `candidates @select`.

## Normalization, Cardinality, And State

Operand evaluation order:

```text
@from extraction
-> source cardinality guard
-> lookup-key normalization, if lookup is present
-> lookup, if present
-> raw-result cardinality and shape guard, if lookup is present
-> comparable-result extraction
-> comparable-result normalization
-> normalized-result cardinality and shape guard
-> state policy
-> comparison
```

`@normalizer` attaches to the operand that declares it and produces the final
comparison value. It does not normalize lookup keys. Key normalization is
declared on `lookup/key` children.

For operands without lookup, comparable-result extraction is the source value
itself. For operands with lookup, comparable-result extraction selects the
declared lookup result projection before operand normalization.

Cardinality is checked in three phases:

- `@cardinality`: source values extracted by `@from`.
- lookup `@result-cardinality`: raw result returned by the lookup operation.
- `@result-cardinality`: normalized comparable result cardinality.

Shape is checked independently from cardinality:

- `@shape`: source item shape.
- lookup `@result-shape`: raw lookup result item shape.
- `@result-shape`: normalized comparable result item shape.

Multiple extracted source values for `@cardinality="one"` are `invalid`.
Validators must not normalize or dedupe multiple source values into an
accidental scalar.

`sequence` is reserved for ordered duplicate-preserving reference lists and is
not accepted by the initial package-check vocabulary. Use `set` only when
ordering and duplicates are not semantic after item provenance has been
recorded.

Named set normalizers declare an item normalizer and produce a structured set
result:

```text
comparisonSet:
  sorted duplicate-free normalized values

items:
  source-ordered item outcomes:
    declaredValue
    sourceLexeme, when captured
    normalizedValue, when valid
    state
    reason, when non-valid
    sourceRange
    duplicateGroup, when the normalized value duplicates another item
```

Comparison operators consume `comparisonSet`. Diagnostic projection and
structured provenance consume `items`, so invalid entries and duplicate origins
remain addressable even when comparison values dedupe.

`declaredValue` is the parsed semantic value. `sourceLexeme` is the exact
authored token spelling when the parser captured one, and `sourceRange` is
location metadata. Validators must not use `sourceRange` as part of value
equality or reconstruct `sourceLexeme` from `declaredValue`.

The operand's `@normalizer` remains the collection normalizer for set results.
Structured comparison metadata also exposes the effective `itemNormalizer`: for
scalar operands it is the scalar normalizer itself, and for set operands it is
the item normalizer declared by the named set normalizer.

Some engine-assisted normalizers need contextual provenance. For
`schema:content-type-identity`, an explicit schema lookup key, namespace
metadata lookup, package validation overlay, or host content registry supplies
the schema/content registry context. A content-type alias that needs context
and does not receive enough context finalizes to `unresolved` with
`reason=ambiguous-content-type`; validators must not fall back to a global
alias table.

Schema identity and namespace identity remain separate operand domains in this
vocabulary. Schema-reference operands bind to `schema:schema-uri-declaration`,
`schema:schema-uri`, or `schema:schema-identity` results. Namespace-claim
operands bind to `schema:namespace-uri` or `schema:namespace-uri-set` results.
Namespace metadata lookups may carry resolved `schemaUri` and `schemaVersion`
as lookup provenance, but those fields do not turn the namespace operand into a
schema operand unless a versioned adapter has projected an explicit schema
URI/identity result with its own binding.

Identifier-token and profile-name bindings are also separate domains.
`schema:identifier-token` is for schema-owned local tokens that use the
`schema:identifier` datatype grammar. Dotted profile selectors bind to
`schema:profile-name` and must not be normalized by first splitting, folding, or
coercing them into identifier tokens.

Function-name and function-identity bindings are separate domains. Manifest
`@function-name` operands bind to `schema:function-name`, which preserves the
authored exported symbol. Compiled CEMT declarations bind to
`schema:function-identity`, whose record carries the module/artifact identity,
canonical exported function name, and optional profile/kind metadata. Lookup
provenance may show both values, but comparisons must project the intended
field pair explicitly.

Authored artifact IDs and path-derived artifact identities are separate
domains. Manifest artifact ID operands bind to `schema:artifact-name` only when
an explicit ID field exists. Artifact path or URI operands first resolve through
`schema:document-uri`, then bind to `schema:artifact-identity` with package or
registry context. Validators must not derive an artifact name from a path
basename or compare a resolved artifact URI as an authored ID.

Document URI bindings are identity finalization, not resource access.
`schema:document-uri` applies effective base URI, resolver purpose,
package/module-map context, and policy to produce declared/resolved URI values
and resolver provenance. A later explicit lookup or resource behavior must own
fetching, reading, listing, parsing, schema loading, or compilation.

Composite function/artifact lookup keys use canonical `lookup` child form with
one `key` child per component. Each key child declares its own binding,
source, normalizer, cardinality, and shape. Required components include the
resolved artifact identity and lexical function name; contracts add content
type, schema identity, category, profile, and subject type only when those
fields participate in lookup identity. Validators must not pack composite keys
into one string or apply one broad normalizer to every component.

Operand states:

- `valid`: value exists, has the declared shape, and normalized successfully.
- `missing`: source extraction found no value.
- `invalid`: value exists but is malformed, rejected by a normalizer, or has
  the wrong cardinality/shape.
- `unresolved`: lookup key exists and normalizes, but the referenced target is
  absent.
- `unsupported`: validator cannot or must not execute the declared operation.

Every non-`valid` operand state carries a stable `reason`.

State policies:

- `required-valid`: only `valid` passes.
- `optional-valid`: `valid` or `missing` passes.
- `allow-unresolved`: `valid` or `unresolved` passes.
- `allow-unsupported`: `valid` or `unsupported` passes. Use only for portable
  optional capabilities.

Default reference-resolution checks use `@state="required-valid"` so missing,
invalid, unresolved, and unsupported operands all produce diagnostics.
`unresolved-fails` is not an operand state policy. Use `required-valid` when
unresolved references should fail, or `allow-unresolved` when unresolved
references are acceptable for a specific operand.

`pending` is not a final operand state. A deferred lookup may be pending while
the validator waits for an allowed resource or capability, but comparison is
deferred until the operand finalizes to `valid`, `missing`, `invalid`,
`unresolved`, or `unsupported`.

## Lookup

A lookup is declared by a canonical `lookup` child element. Operand `@lookup` is
compatibility shorthand for the simplest case where the operand source is
already the lookup key and the lookup result itself is the comparable value.
Any lookup that needs separate key normalization, result projection, result
normalization, capability requirements, package context, or source-range policy
must use child form.

Lookup produces a raw result envelope. The parent operand then extracts the
declared comparable result from that raw result and applies the parent
operand's `@normalizer`. Lookup key envelopes remain provenance.

Lookup is the first stage that may assert target availability. Normalizers such
as `schema:document-uri` may finalize a key to a resolved URI, but they must
not assert that the target exists, is readable, parses, or compiles. Resource
availability is represented by explicit lookup names or behaviors, for example
`schema:resource-readable`, `schema:resource-parse`,
`schema:registry.descriptor`, or `schema:function-identity`.

Use child form when lookup needs `@as`, `@key`, `key` children, `@result`,
`@requires`, `@package`, `@support`, source-range options, explicit state
policy, or multiple outputs.

Lookup fields:

- `@name`: lookup operation or capability name.
- `@execution`: `pure` or `engine-assisted`.
- `@select`: pure inline lookup selector over current document/context
  bindings.
- `@key`: simple one-key lookup reference.
- `key` children: canonical multi-key lookup form.
- `@as`: optional lookup result binding for later references/projection.
- `@result`: field or constrained source path selected from the raw lookup
  result before operand normalization. Omitted means the raw lookup result
  itself is the comparable result.
- `@result-cardinality`: raw lookup result cardinality.
- `@result-shape`: raw lookup result item shape.
- `@result-key`: simple record set identity field for `cardinality=set,
shape=record` results.
- `result-key` children: composite record set identity for `cardinality=set,
shape=record` results.
- `@provenance`: lookup provenance kind.
- `@source-range`: lookup key/result source-range policy.
- `@requires`: engine capability identifier.
- `@support`: capability support policy.

Lookup key child fields:

- `@binding`: required provenance identity for diagnostics and lookup traces.
- `@from`: optional constrained source path. Omitted means the parent operand
  source value.
- `@normalizer`: normalizer for this key envelope.
- `@cardinality`: key source cardinality; omitted inherits the parent
  operand's source cardinality.
- `@shape`: key item shape; omitted inherits the parent operand's source shape.

Key bindings are not comparison operand bindings. They may appear under
structured provenance when projected, but they must not populate
`comparison.operands.<role>.binding`, `actualValues`, `expectedValues`, or
`invalidValues` as standalone operands.

Lookup execution:

- Pure inline lookups use `@select` and run over current in-memory bindings.
- Pure lookups may read `$document`, `$target`, candidate bindings, operand
  bindings, or prior pure lookup results.
- Pure lookups use the same narrow selector profile as `candidates @select`.
- Engine-assisted lookups use named capabilities and declare `@requires`.
- `lookup` with `@select` defaults to `@execution="pure"`.
- `lookup` with `@requires` defaults to `@execution="engine-assisted"`.
- A lookup containing both `@select` and engine capability fields is invalid
  unless a future composed lookup design explicitly allows it.

Lookup result cardinality:

- `one`: exactly one raw result value. Zero results become `missing` or
  `unresolved` depending on lookup kind; multiple results are `invalid`.
- `optional`: zero or one raw result value. Multiple results are `invalid`.
- `set`: zero or more raw result values.

Lookup result shape:

- `scalar`: scalar raw item.
- `record`: structured raw item with named fields.

`sequence` and `stream` are not schema-declared lookup result cardinalities in
the initial package-check vocabulary.

A `cardinality=set, shape=record` lookup requires explicit `@result-key` or one
or more `result-key` children. A record set lookup without a result key is a
schema-definition error. Sort and dedupe use the normalized key tuple.
Duplicate normalized keys with identical records collapse to one record;
duplicate keys with different non-key fields are `invalid`. Validators must not
sort/dedupe record sets by implicit object serialization.

Lookup failures:

```text
key absent                         -> missing
key malformed                      -> invalid
required capability unavailable    -> unsupported
lookup capability unknown          -> unsupported
lookup execution denied by policy  -> unsupported(reason=policy-denied)
lookup runs, target absent         -> unresolved
lookup result wrong shape          -> invalid
required result field absent       -> missing
comparable result malformed        -> invalid
```

URI identity finalization is not a lookup target miss. A malformed URI is
`invalid(reason=invalid-document-uri)`. A URI that cannot be finalized because
the required resolver, module-map entry, package context, or policy mapping is
missing is `unresolved(reason=unresolved-document)`. A finalized URI whose
resource does not exist is reported by the explicit lookup/resource behavior
that tried to read or inspect it.

Do not use a generic lookup `@on-missing` policy in the initial vocabulary.
Candidate absence belongs to `@on-empty`; operand/lookup state acceptability
belongs to operand `@state`.

## Comparison Declaration

`compare` declares the value relationship after operands and lookups have
produced normalized comparable values and every operand has satisfied its
`@state` policy.

Compare fields:

- `@operator`: required comparison operator from
  [`cem-ml-reference-comparison-design.md`](cem-ml-reference-comparison-design.md).
- `@presence`: optional relational presence policy for missing operands.

`@presence` values:

- `when-present`: missing on either side passes; when both sides are present
  they must satisfy `@operator`. Use for advisory optional metadata.
- `both-or-none`: missing on both sides passes; exactly one missing side fails;
  when both are present they must satisfy `@operator`. Use for paired optional
  metadata.

Presence policy is comparison-owned. Do not put relational presence rules on
`actual`, `expected`, or `forbidden` operands. Presence policy only handles
`missing`; `invalid`, `unresolved`, and `unsupported` remain operand-state
policy decisions.

Surface shorthands lower to qualified IR before evaluation:

```text
operand @state="required-valid"      -> statePolicy=schema:required-valid
operand @state="optional-valid"      -> statePolicy=schema:optional-valid
operand @state="allow-unresolved"    -> statePolicy=schema:allow-unresolved
operand @state="allow-unsupported"   -> statePolicy=schema:allow-unsupported
compare @presence="when-present"     -> presencePolicy=schema:compare-when-present
compare @presence="both-or-none"     -> presencePolicy=schema:both-or-none
omitted compare @presence            -> no comparison presence policy
```

Each state or presence field accepts one value. Validators must not interpret
whitespace-separated policy lists, token order, or redundant policies such as
`unresolved-fails`.

Because operand state policy runs before comparison presence policy, optional
presence checks normally declare participating operands with
`@state="optional-valid"`:

```cem
{actual
    @binding="profile"
    @from="artifact.@profile"
    @normalizer="schema:profile-name"
    @cardinality="optional"
    @shape="scalar"
    @state="optional-valid"}

{expected
    @binding="profile"
    @from="function.profile"
    @normalizer="schema:profile-name"
    @cardinality="optional"
    @shape="scalar"
    @state="optional-valid"}

{compare @operator="schema:equals" @presence="both-or-none"}
```

## Capability Negotiation

Engine-assisted constraints and lookups declare capability dependencies with
versioned `@requires` and support policy `@support`.

`@requires` grammar:

```text
requires := capability-name [ "@" version-constraint ]

capability-name :=
    capability-id
  | capability-uri

capability-id      := namespace ":" capability-segment *( "." capability-segment )
namespace          := [A-Za-z_][A-Za-z0-9_-]*
capability-segment := [A-Za-z_][A-Za-z0-9_-]*

capability-uri := uri-scheme ":" uri-body
uri-scheme     := [A-Za-z][A-Za-z0-9+.-]*
uri-body       := 1*( non-whitespace-non-@ )

version-constraint :=
    major
  | major "." minor
  | major "." minor "." patch [ prerelease ] [ build ]
  | "^" major "." minor "." patch
  | "=" major "." minor "." patch [ prerelease ] [ build ]

prerelease := "-" semver-identifier *( "." semver-identifier )
build      := "+" semver-identifier *( "." semver-identifier )
```

Capability names are matched as opaque strings after lexical validation. `@` is
reserved as the version delimiter and is not allowed inside the capability
name; URI-like names must percent-encode literal `@`.

Version matching:

- no version: capability presence is enough.
- `@1`: same major, stable versions only.
- `@1.2`: same major and same minor, stable versions only.
- `@1.2.3`: same major and version greater than or equal to `1.2.3`, stable
  versions only.
- `@^1.2.3`: same major and version greater than or equal to `1.2.3`, stable
  versions only.
- `@=1.2.3`: exact `major.minor.patch`; prerelease/build rules apply.
- prerelease constraints match exactly.
- caret prerelease constraints are rejected initially.
- build metadata is exact only when the constraint includes build metadata.

Full npm/Cargo-style range syntax is out of scope initially.

Capability failure mapping:

```text
malformed constraint syntax      -> schema-definition invalid
unknown capability name          -> unsupported(reason=unsupported-capability)
no compatible capability version -> unsupported(reason=unsupported-capability)
capability blocked by policy     -> unsupported(reason=policy-denied)
capability exists but disabled   -> unsupported(reason=unsupported-capability)
```

Policy-denied capability execution always produces
`unsupported(reason=policy-denied)` at the operand/capability state level. A
dedicated policy diagnostic is additive reporting/provenance metadata; it does
not replace the `unsupported` state and does not change `@support` or operand
`@state` evaluation.

`@support` values:

- `required`: capability must exist; unavailable capability emits the normal
  constraint diagnostic and produces `unsupported`.
- `optional`: capability may be absent; absence produces `unsupported`, then
  operand `@state` decides pass/fail.

`@support="soft"` is not part of the normalized vocabulary. Source
compatibility may accept it only as shorthand for `@support="optional"` plus an
additive warning/provenance reporting flag for unsupported capability absence.
After lowering, validators and comparison metadata must expose `optional`, not
`soft`.

## Engine-Assisted Execution

Constraints are pure by default. Engine assistance is never inferred silently.
If `@execution` is omitted and an engine-assisted feature appears, the
constraint declaration is a schema-definition error.

Constraint execution fields:

- `@execution`: `pure` or `engine-assisted`.
- `@requires`: named capability for engine-assisted checks.
- `@support`: capability availability policy.
- `@package`: `none`, `current`, or `declared`.

`@determinism` is not part of the active vocabulary. Deterministic bounds are
validator/capability requirements, not a schema field.

Pure constraints may use:

- `candidates @select` with the narrow pure selector profile.
- operand `@from` paths.
- pure operand normalizers.
- operand cardinality and state checks.
- `compare`.
- pure inline `lookup @select`.
- diagnostic projection.

Pure constraints must not use schema registry lookup, package-relative
resolution, file reads, URI resolver access, artifact parsing, CEMT function
registry inspection, plugin/host callbacks, external mutable state, or
engine-assisted `lookup @requires`.

Engine-assisted helpers may read resources, resolve URIs, parse artifacts, and
query registries only through named read-only capabilities permitted by
`QueryContextScope`. They must not mutate documents or package files, write
artifacts, perform unbounded network access, read outside resolver/policy
grants, depend on wall-clock time, depend on nondeterministic global state, or
change validation behavior based on cache hits/misses.

`@package` belongs on `constraint` or package-aware `lookup`, not on
`candidates`:

- `none`: document-only; no package-relative resolution.
- `current`: use the current package context.
- `declared`: package context comes from an explicit operand or lookup key.

During local schema-package validation, `@package="current"` is evaluated in
two phases. Pure manifest/source consistency constraints run before registry
admission and must use declaration normalizers rather than registry identity
lookup. After those checks pass, the validator may build an isolated
provisional descriptor and expose it only through the current validation
overlay. Engine-assisted lookups for endpoint, example, artifact, namespace, or
registry-identity checks then query trusted registries plus that overlay. The
overlay is discarded unless every required check succeeds and the host admits
the descriptor to its catalog.

The initial vocabulary does not include generic `@fallback`. Fallback-like
behavior is expressed through explicit `@support` and operand `@state` policies.
Alternate lookup sources must be explicit lookup declarations; validators must
not perform implicit fallback lookups.

## Diagnostic Projection

Diagnostic projection is operand-owned by default, with optional schema-level
aliases. In this document, `projection` refers to diagnostic output only. Record
field extraction for comparison uses `value-path` in the comparison vocabulary,
not projection.

Projection fields:

- `@profile`: `compatibility` or `structured`; omitted means `compatibility`.
- `@project`: whitespace-separated semantic projection tokens.
- `@source`: diagnostic-level source-range owner: `operand`, `candidate`,
  `lookup`, or `constraint`.
- `@value-view`: `raw`, `normalized`, or `both`; omitted means `raw`.

`@comparison` is not active vocabulary. Use `@project="comparison"` to request
comparison metadata.

`@diagnostic-field` is declared on operands, not on `projection`. Projection
consumes aliases only when alias metadata is projected.

Projection token vocabulary:

```text
actual
expected
invalid
missing
unresolved
unsupported
comparison
source-range
source-ranges
provenance
aliases
candidate
all
```

Token semantics:

- `actual`: project `actualValues`.
- `expected`: project `expectedValues`.
- `invalid`: project `invalidValues` and `invalidFields`.
- `missing`: project `missingValues`.
- `unresolved`: project `unresolvedValues`.
- `unsupported`: target/additive token that projects `unsupportedValues` after
  the implementation and schema result contract support that bucket.
- `comparison`: project stable `comparison` metadata.
- `source-range`: project top-level `sourceRange`.
- `source-ranges`: project structured per-bucket `sourceRanges`; implies
  `source-range`.
- `provenance`: project lookup/registry provenance.
- `aliases`: project alias metadata from `@diagnostic-field`.
- `candidate`: project selected candidate context.
- `all`: project every stable implemented token except
  implementation/debug-only metadata.

Unknown `@project` tokens are schema-definition errors. Duplicate tokens are
deduped without warning. Tokens are semantic names, not JSON bucket names: use
`expected`, not `expectedValues`.

Projection defaults:

```text
compatibility -> actual expected invalid missing unresolved source-range
structured    -> actual expected invalid missing unresolved source-range source-ranges candidate
```

If `@project` is omitted, validators use the selected profile's default token
set. If `@profile="compatibility"` and `@project` includes structured tokens,
those fields are emitted additively without changing compatibility field
shapes. Full structured output shape requires explicit `@profile="structured"`.

Canonical diagnostics are emitted per candidate and per primary failing
operand/state. Bucket keys default to operand `@binding`, and every structured
bucket value is an ordered array. Aggregated summaries are report/UI views
derived from canonical diagnostics.

Projection ordering:

```text
candidate order -> operand declaration order -> source document order ->
lookup result order
```

Public buckets use raw values by default. Normalized values are comparison
metadata by default. Per-bucket overrides use `{bucket}` children with semantic
token names:

```cem
{projection @value-view="raw" |
    {bucket @name="expected" @value-view="normalized"}
    {bucket @name="comparison" @value-view="both"}
}
```

Per-operand value-view overrides are out of scope initially.

## Source Ranges And Provenance

Top-level `sourceRange` is the primary diagnostic anchor. Structured
`sourceRanges` is additive and may include related operand/lookup ranges.

Source owner precedence:

```text
1. Determine the primary actionable operand/state.
2. Apply projection @source: operand | candidate | lookup | constraint.
3. If the selected owner is lookup, apply lookup @source-range:
   none | key | result | result-preferred | key-preferred.
4. If the selected owner has no usable range, apply the state-specific fallback.
5. Emit the selected primary range as top-level sourceRange.
6. Emit related ranges under structured sourceRanges when requested.
```

`projection @source` is the diagnostic-level source owner selector.
`lookup @source-range` is the lookup-level key/result range policy. Lookup
`@source-range` affects top-level `sourceRange` only when
`projection @source="lookup"`.

Unresolved diagnostics point to the local reference or lookup key source range
by default. Lookup result ranges are not used for unresolved absence because no
result exists. Fallback order:

```text
key/reference range -> composite key part ranges -> candidate range ->
constraint declaration range
```

Registry-derived expected values stay value-focused in `expectedValues`.
Registry origin details are additive `provenance` metadata keyed by the same
operand binding and, when needed, value/index. Validators must not point at the
candidate or constraint declaration merely to manufacture a range for an
external expected value.

Lookup key provenance is keyed by the parent operand binding and records the
key binding, declared key value, key normalizer, normalized key value when
valid, key state/reason, lookup operation, capability, and source range. The
key binding does not become a comparison operand binding.

## Comparison Metadata

`comparison` is a stable vocabulary-owned summary of the declared comparison
and its result. It exposes declaration/result facts only; it must not expose
evaluator internals, implementation traces, parser AST details, lookup cache
keys, resolver retry details, or engine-private error objects.
For lookup-based operands, comparison metadata describes the final comparable
result after lookup and result normalization. Lookup keys are provenance and
are not listed as role operands.
`normalizer` reports the operand's scalar or collection normalizer.
`itemNormalizer` reports the item equivalence normalizer used by comparison.

Structured `comparison` fields:

```text
operator
passed
primary
reason
operands
operands.<role>.binding
operands.<role>.state
operands.<role>.reason
operands.<role>.normalizer
operands.<role>.itemNormalizer
operands.<role>.values
operands.<role>.items[].state
operands.<role>.items[].reason
```

`operands.<role>.reason` is omitted when the role operand is `valid`. For set
operands, structured metadata may include `items` with source-ordered item
states and reasons so invalid, unresolved, unsupported, and duplicate members
do not lose their root cause when the comparable `values` array contains only
valid comparison values.

Stable reason codes:

```text
not-equal
not-member
missing-required
invalid-value
unresolved-reference
unsupported-capability
policy-denied
forbidden-overlap
missing-required-members
no-intersection
record-field-mismatch
record-field-not-member
malformed-comparison
```

Example:

```json
{
    "comparison": {
        "operator": "schema:member-of",
        "passed": false,
        "primary": "content-type",
        "reason": "not-member",
        "operands": {
            "actual": {
                "binding": "content-type",
                "state": "valid",
                "normalizer": "schema:content-type-identity",
                "itemNormalizer": "schema:content-type-identity",
                "values": [
                    {
                        "contentType": "text/html",
                        "schemaIdentity": {
                            "uri": "https://cem.dev/ns/data/html/1",
                            "embeddedVersion": "1.0.0"
                        }
                    }
                ]
            },
            "expected": {
                "binding": "content-type",
                "state": "valid",
                "normalizer": "schema:content-type-identity-set",
                "itemNormalizer": "schema:content-type-identity",
                "values": [
                    {
                        "contentType": "application/json",
                        "schemaIdentity": {
                            "uri": "https://cem.dev/ns/data/json/1",
                            "embeddedVersion": "1.0.0"
                        }
                    }
                ]
            }
        }
    }
}
```

Compatibility projection may continue exposing existing camelCase fields such
as `actualBinding`, `expectedBinding`, and `actualNormalizer`, with
item-normalizer aliases added only as compatibility extensions when needed.
Structured projection uses the role-keyed `operands` object.

## Aliases

Standard diagnostic buckets are fixed and cannot be renamed by schemas. Fixed
buckets include `actualValues`, `expectedValues`, `invalidValues`,
`missingValues`, `unresolvedValues`, `invalidFields`, `comparison`,
`sourceRange`, `sourceRanges`, and `provenance`. `unsupportedValues` is a
target additive bucket until the implementation and schema result contract
ship it.

`@diagnostic-field` is an additive display/report alias for an operand; it does
not replace `@binding` in canonical diagnostics.

Alias grammar:

```text
diagnostic-field := bare-name
bare-name        := [A-Za-z_][A-Za-z0-9_-]*
```

Aliases must be unique within a constraint projection and must not collide with
fixed bucket names, candidate `@as` names, another operand's `@binding`, or
another alias. Alias collisions are schema-definition errors. Quoted/free-form
aliases are out of scope; if human-facing labels are needed later, add a
presentation-only field such as `@diagnostic-label`.

## Compatibility Contract

The compatibility projection profile preserves current broad CLI/report detail
keys and value shapes. The structured profile uses canonical operand-owned
projection with ordered arrays and additive metadata. New metadata must not
remove, rename, or reinterpret compatibility keys.

Frozen compatibility keys:

```text
schemaUri
diagnostic
behavior
checkKind
contract
element
target
actualValues
expectedValues
invalidValues
missingValues
unresolvedValues
invalidFields
sourceRange
```

Additive structured keys:

```text
comparison
sourceRanges
provenance
aliases
candidate
unsupportedValues
```

`sourceRange` remains the top-level primary diagnostic anchor. `sourceRanges`
is a precise ownership map; it does not replace `sourceRange`. `comparison`
explains operand relationships; it does not replace value buckets. `provenance`
explains lookup/registry origin; it does not replace values. `aliases` supports
display/report naming; it does not replace canonical operand `@binding`.
