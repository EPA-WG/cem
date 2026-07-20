# CEM-ML Reference Comparison Vocabulary

Status: design decision for schema-owned reference constraints.

This note defines comparison vocabulary for normalized reference values produced
by [`cem-ml-reference-normalization-design.md`](cem-ml-reference-normalization-design.md).
Candidate selection, lookup syntax, execution boundaries, concrete CEM surface
shape, and diagnostic projection are defined in
[`cem-ml-reference-vocabulary-design.md`](cem-ml-reference-vocabulary-design.md).

## Goals

- Compare already-normalized values without hiding normalization inside the
  comparison operator.
- Cover the current Rust-backed `schema:reference-resolution` checks:
  endpoint/example content-type compatibility, package schema metadata
  consistency, artifact function lookup, and artifact function metadata
  matching.
- Keep missing, invalid, and unresolved reference behavior declarative and
  visible in diagnostics.
- Preserve the current `expectedValues` and `invalidValues` diagnostic shape
  while making source-range ownership explicit.

## Comparison Input Model

A comparison consumes normalized comparable operands produced by named
bindings. Lookup keys, registry handles, resolver identities, and descriptor
origins may explain how a comparable operand was produced, but they are
provenance, not comparison operands. Each operand has:

- `role`: `actual`, `expected`, or `forbidden`.
- `binding`: schema-local name from the future reference rule.
- `normalizer`: the normalizer applied to the operand.
- `projection`: optional field path for record normalizers, such as
  `essence`, `schemaUri`, `category`, or `profile`.
- `cardinality`: `one`, `optional`, `set`, or conceptual `sequence`.
- `shape`: `scalar` or `record`.
- `statePolicy`: how `missing`, `invalid`, `unresolved`, and `unsupported`
  operand states are interpreted before value comparison.
- `presencePolicy`: optional relational rule for comparing missing or present
  operands together, such as optional profile metadata.

Comparisons should use the same normalizer on both sides. Mixed normalizers are
allowed only when the operator explicitly defines comparable projected outputs.
`sequence` is reserved for future ordered duplicate-preserving comparisons and
is not active in the initial package-check operator surface.

## Operand State Policies

Operand state policy runs before value comparison and is evaluated per operand:

| Policy                     | Semantics                                                                                                      |
| -------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `schema:required-valid`    | Operand must exist and normalize to `state=valid`. `missing`, `invalid`, `unresolved`, and `unsupported` fail. |
| `schema:optional-valid`    | `valid` or `missing` passes. `invalid`, `unresolved`, and `unsupported` fail.                                  |
| `schema:allow-unresolved`  | `valid` or `unresolved` passes. `missing`, `invalid`, and `unsupported` fail.                                  |
| `schema:allow-unsupported` | `valid` or `unsupported` passes. `missing`, `invalid`, and `unresolved` fail.                                  |

Default state policy is `schema:required-valid`. Engine-assisted lookup
operands also default to `schema:required-valid`; an unresolved lookup fails
because `unresolved` is not accepted by that state policy.

`pending` is not a comparison-visible operand state. Deferred lookups must
finalize before comparison or the comparison must be deferred.

## Comparison Presence Policies

Presence policies are relational rules. They are not operand states and do not
change normalization outcomes:

| Policy                        | Semantics                                                                                                                                      |
| ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `schema:compare-when-present` | Missing on either side passes; when both sides are present they must satisfy the comparison. Use for advisory optional metadata.               |
| `schema:both-or-none`         | Missing on both sides passes; exactly one missing side fails; when both are present they must satisfy the comparison. Use for paired metadata. |

## Operators

| Operator                         | Operand Shape                             | Pass Condition                                                                                   | Primary Use                                                               |
| -------------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------- |
| `schema:equals`                  | one actual, one expected                  | Normalized projected values are equal.                                                           | Schema URI consistency, namespace URI consistency, exact scalar metadata. |
| `schema:member-of`               | one actual, expected set                  | Actual value is a member of the expected set.                                                    | Endpoint/example content type is registered by the referenced schema.     |
| `schema:all-in`                  | actual set, expected set                  | Every actual value is a member of the expected set. Empty actual set is handled by state policy. | Package content-type claims must all be declared by the schema source.    |
| `schema:contains-all`            | actual set, expected set                  | Every expected value is present in the actual set.                                               | Required diagnostic/code/profile sets once expressed declaratively.       |
| `schema:intersects`              | actual set, expected set                  | Actual and expected sets share at least one value.                                               | Compatibility checks where any shared capability is enough.               |
| `schema:disjoint`                | actual set, forbidden set                 | Actual and forbidden sets have no shared values.                                                 | Forbidden content types, categories, profiles, or capabilities.           |
| `schema:exists`                  | one actual                                | Operand exists and is valid after state policy.                                                  | Artifact CEMT function is declared.                                       |
| `schema:record-fields-equal`     | actual record, expected record            | Each declared field pair is equal after projection and per-field state policy.                   | CEMT output function metadata contract matching.                          |
| `schema:record-fields-member-of` | actual record, expected record/set record | Each declared field pair satisfies `schema:member-of` after projection.                          | Descriptor records whose fields expose allowed sets.                      |

Operators are deterministic and side-effect-free. They do not read registries or
resources; lookup-key normalization, lookup, comparable-result normalization,
and state policy happen before comparison.
Set operators consume the operand's sorted duplicate-free `comparisonSet`.
Duplicate origins, invalid items, and source-ordered item outcomes remain
diagnostic/provenance data rather than operator inputs.

## Projection And Detail Ownership

Comparison failures project details through the canonical projection vocabulary
defined in
[`cem-ml-reference-vocabulary-design.md`](cem-ml-reference-vocabulary-design.md).
This comparison note owns only the comparison-side semantics:

- Expected operands provide final comparable reference values for
  `expectedValues`.
- Actual operands provide failing observed values for `invalidValues`.
- Missing, invalid, unresolved, and unsupported operand states are decided
  before comparison runs.
- `comparison` metadata describes operator, pass/fail result, primary binding,
  stable reason code, and role-keyed operand summaries.
  Lookup key bindings are provenance and must not replace role operand
  bindings in comparison metadata.

The compatibility projection profile may continue exposing older broad keys and
camelCase comparison fields where current CLI/report consumers expect them.
Structured projection uses the role-keyed `comparison.operands` shape and
ordered value arrays.

## Application To Current Rust-Backed Checks

| Current Check                     | Normalized Operands                                                                                                                                                                                                                                                             | Operator                                                                                                                                                      |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `endpoint-content-type-schema`    | endpoint `@content-type` as `actual` `content-type` using `schema:media-type-essence`; endpoint `@schema` as lookup key `schema` using `schema:schema-identity`; referenced schema descriptor `contentTypes` as `expected` `content-type` using `schema:media-type-essence-set` | `schema:member-of` with `schema:required-valid`.                                                                                                              |
| `example-content-type-schema`     | example `@content-type` as `actual` `content-type`; example `@schema` as lookup key `schema`; referenced schema descriptor `contentTypes` as `expected` `content-type`                                                                                                          | `schema:member-of` with the same policies as endpoint compatibility.                                                                                          |
| `schema-uri-consistency`          | package manifest schema URI declaration as `actual`; loaded schema source URI declaration as `expected`; both use `schema:schema-uri-declaration` before registry admission                                                                                                     | `schema:equals`.                                                                                                                                              |
| `schema-content-type-consistency` | package manifest content-type claims as `actual` set; loaded schema source content types as `expected` set                                                                                                                                                                      | `schema:all-in`.                                                                                                                                              |
| `schema-namespace-consistency`    | package manifest namespace claims as `actual` set; loaded schema source namespaces as `expected` set                                                                                                                                                                            | `schema:all-in`, or `schema:equals` when the schema source exposes one canonical namespace.                                                                   |
| `artifact-function-declared`      | manifest `@function-name` and compiled CEMT declarations                                                                                                                                                                                                                        | `schema:exists` after engine-assisted `schema:function-name` lookup.                                                                                          |
| `artifact-function-contract`      | manifest artifact target metadata and compiled CEMT output function metadata                                                                                                                                                                                                    | `schema:record-fields-equal`; use `schema:both-or-none` for optional profile fields and `schema:required-valid` for kind/content-type/schema/category fields. |
| `example-expected-diagnostics`    | declared expected diagnostic codes and observed validation report diagnostic codes                                                                                                                                                                                              | `schema:contains-all` for expected diagnostics, with future room for `schema:disjoint` on explicitly forbidden diagnostics.                                   |

## Syntax Shape

The concrete CEM surface keeps selector, operand, lookup, comparison, and
projection pieces separate. The endpoint content-type/schema check lowers to
this conceptual comparison input:

```text
actual operand:
  binding: content-type
  normalizer: schema:media-type-essence
  value source: endpoint.@content-type

expected lookup key provenance:
  binding: schema
  normalizer: schema:schema-identity
  value source: endpoint.@schema

expected operand:
  binding: content-type
  normalizer: schema:media-type-essence-set
  value source: schema descriptor contentTypes selected by lookup

comparison:
  operator: schema:member-of
```

The normative element/attribute vocabulary is defined in
[`cem-ml-reference-vocabulary-design.md`](cem-ml-reference-vocabulary-design.md).
The comparison design decision is that comparison declarations name bindings,
normalizers, operators, and state policies explicitly, while lookup-key
bindings remain provenance.

## Implementation Notes

- Comparison results should be implemented as reusable `schema:reference-resolution`
  evaluator primitives, not package-specific Rust branches.
- Comparisons operate on normalized value records. They should not call parser,
  resolver, registry, or CEMT compiler APIs directly.
- Diagnostic details should preserve today's broad keys (`expectedValues`,
  `invalidValues`, `actualValues`, `invalidFields`, `checkKind`) and add
  comparison metadata only as structured detail extensions.
- Operator behavior must be deterministic over sorted normalized
  `comparisonSet` values.
- Empty sets are not automatically failures. State policy and cardinality decide
  whether an empty operand is valid before an operator runs.
