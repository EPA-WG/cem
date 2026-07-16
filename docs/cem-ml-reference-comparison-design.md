# CEM-ML Reference Comparison Vocabulary

Status: design decision for schema-owned reference constraints.

This note defines comparison vocabulary for normalized reference values produced
by [`cem-ml-reference-normalization-design.md`](cem-ml-reference-normalization-design.md).
It intentionally stops before selector syntax, lookup syntax, and concrete CEM
surface syntax; those belong to the remaining cross-node/reference vocabulary
todo.

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

A comparison consumes normalized operands produced by named bindings. Each
operand has:

- `role`: `actual`, `expected`, or `forbidden`.
- `binding`: schema-local name from the future reference rule.
- `normalizer`: the normalizer applied to the operand.
- `projection`: optional field path for record normalizers, such as
  `essence`, `schemaUri`, `category`, or `profile`.
- `cardinality`: `one`, `optional`, or `set`.
- `statePolicy`: how `missing`, `invalid`, and `unresolved` operand states are
  interpreted before value comparison.

Comparisons should use the same normalizer on both sides. Mixed normalizers are
allowed only when the operator explicitly defines comparable projected outputs.

## State Policies

State policy runs before value comparison:

| Policy | Semantics |
| --- | --- |
| `schema:required-valid` | Operand must exist and normalize to `state=valid`. `missing`, `invalid`, `unresolved`, and `unsupported-normalizer` fail. |
| `schema:optional-absent-ok` | Missing operand passes without running the value comparison. Invalid, unresolved, and unsupported still fail. |
| `schema:compare-when-present` | Missing on either side passes; when both sides are present they must satisfy the comparison. Use for advisory optional metadata. |
| `schema:both-or-none` | Missing on both sides passes; exactly one missing side fails; when both are present they must satisfy the comparison. Use for optional profile metadata that must match if either side declares it. |
| `schema:unresolved-fails` | Unresolved references fail with an unresolved-reference reason and the source range of the reference field. This is the default for schema, document, and function lookups. |

Default state policy is `schema:required-valid` for required operands and
`schema:unresolved-fails` for engine-assisted lookup operands.

## Operators

| Operator | Operand Shape | Pass Condition | Primary Use |
| --- | --- | --- | --- |
| `schema:equals` | one actual, one expected | Normalized projected values are equal. | Schema URI consistency, namespace URI consistency, exact scalar metadata. |
| `schema:member-of` | one actual, expected set | Actual value is a member of the expected set. | Endpoint/example content type is registered by the referenced schema. |
| `schema:all-in` | actual set, expected set | Every actual value is a member of the expected set. Empty actual set is handled by state policy. | Package content-type claims must all be declared by the schema source. |
| `schema:contains-all` | actual set, expected set | Every expected value is present in the actual set. | Required diagnostic/code/profile sets once expressed declaratively. |
| `schema:intersects` | actual set, expected set | Actual and expected sets share at least one value. | Compatibility checks where any shared capability is enough. |
| `schema:disjoint` | actual set, forbidden set | Actual and forbidden sets have no shared values. | Forbidden content types, categories, profiles, or capabilities. |
| `schema:exists` | one actual | Operand exists and is valid after state policy. | Artifact CEMT function is declared. |
| `schema:record-fields-equal` | actual record, expected record | Each declared field pair is equal after projection and per-field state policy. | CEMT output function metadata contract matching. |
| `schema:record-fields-member-of` | actual record, expected record/set record | Each declared field pair satisfies `schema:member-of` after projection. | Descriptor records whose fields expose allowed sets. |

Operators are deterministic and side-effect-free. They do not read registries or
resources; lookup and normalization happen before comparison.

## Projection And Detail Ownership

Comparison failures project details by operand role:

- `expectedValues`: normalized expected values, grouped by binding or projected
  field name. For `schema:member-of`, this is the expected set. For
  `schema:equals`, this is the expected scalar. For record comparisons, this is
  a field-to-expected-value object.
- `invalidValues`: actual values that caused the comparison to fail, grouped by
  binding or projected field name. For `schema:disjoint`, this is the forbidden
  overlap observed on the actual side.
- `missingValues`: bindings or projected fields that failed state policy with
  `missing-value`.
- `unresolvedValues`: bindings or projected fields that failed state policy
  with `unresolved-*` reasons.
- `invalidFields`: field names whose actual values failed comparison or state
  policy.
- `comparison`: optional structured metadata with `operator`, `normalizer`,
  `actualBinding`, `expectedBinding`, and `projection`.

Source ranges attach to the operand that caused the violation:

- Failed actual values use the actual operand's source range.
- Missing actual values use the containing candidate or declared target range.
- Unresolved references use the source range of the reference field that failed
  lookup.
- Invalid expected/forbidden values use the descriptor or schema declaration
  range when available; otherwise they carry descriptor identity only.
- Record comparisons report the narrow field source range when available and
  fall back to the containing record range.

The existing `sourceRange` detail remains the top-level diagnostic anchor. More
specific ranges may be included under `actualValues`, `expectedValues`,
`invalidValues`, `missingValues`, or `unresolvedValues` once the diagnostic
payload supports per-value ranges.

## Application To Current Rust-Backed Checks

| Current Check | Normalized Operands | Operator |
| --- | --- | --- |
| `endpoint-content-type-schema` | endpoint `@content-type` as `actual` `schema:media-type-essence`; endpoint schema descriptor `contentTypes` as `expected` `schema:media-type-essence-set` | `schema:member-of` with `schema:required-valid` and `schema:unresolved-fails`. |
| `example-content-type-schema` | example `@content-type` as `actual`; example schema descriptor `contentTypes` as `expected` | `schema:member-of` with the same policies as endpoint compatibility. |
| `schema-uri-consistency` | package manifest schema URI as `actual`; loaded schema source namespace as `expected` | `schema:equals`. |
| `schema-content-type-consistency` | package manifest content-type claims as `actual` set; loaded schema source content types as `expected` set | `schema:all-in`. |
| `schema-namespace-consistency` | package manifest namespace claims as `actual` set; loaded schema source namespaces as `expected` set | `schema:all-in`, or `schema:equals` when the schema source exposes one canonical namespace. |
| `artifact-function-declared` | manifest `@function-name` and compiled CEMT declarations | `schema:exists` after engine-assisted `schema:function-name` lookup. |
| `artifact-function-contract` | manifest artifact target metadata and compiled CEMT output function metadata | `schema:record-fields-equal`; use `schema:both-or-none` for optional profile fields and `schema:required-valid` for kind/content-type/schema/category fields. |
| `example-expected-diagnostics` | declared expected diagnostic codes and observed validation report diagnostic codes | `schema:contains-all` for expected diagnostics, with future room for `schema:disjoint` on explicitly forbidden diagnostics. |

## Syntax Shape For Future Rules

The eventual CEM surface should keep the pieces separate. A future rule should
be able to say, conceptually:

```cem
{reference-check
    @operator="schema:member-of"
    @actual="endpoint.content-type"
    @actual-normalizer="schema:media-type-essence"
    @expected="endpoint.schema.contentTypes"
    @expected-normalizer="schema:media-type-essence-set"
    @state-policy="schema:required-valid schema:unresolved-fails"}
```

The exact element/attribute names are deferred. The important design decision is
that comparison declarations name bindings, normalizers, operators, and state
policies explicitly.

## Implementation Notes

- Comparison results should be implemented as reusable `schema:reference-resolution`
  evaluator primitives, not package-specific Rust branches.
- Comparisons operate on normalized value records. They should not call parser,
  resolver, registry, or CEMT compiler APIs directly.
- Diagnostic details should preserve today's broad keys (`expectedValues`,
  `invalidValues`, `actualValues`, `invalidFields`, `checkKind`) and add
  comparison metadata only as structured detail extensions.
- Operator behavior must be deterministic over sorted normalized sets.
- Empty sets are not automatically failures. State policy and cardinality decide
  whether an empty operand is valid before an operator runs.
