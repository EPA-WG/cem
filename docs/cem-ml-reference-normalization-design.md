# CEM-ML Reference Normalization Vocabulary

Status: design decision for schema-owned reference constraints.

This note defines the normalized value vocabulary used by
`schema:reference-resolution`. Comparison operators are defined separately in
[`cem-ml-reference-comparison-design.md`](cem-ml-reference-comparison-design.md).
This note intentionally stops before selector syntax, lookup syntax, and
diagnostic projection syntax; those are separate todo items.

## Goals

- Let schemas name the normalization applied before reference checks.
- Keep package-specific Rust branches from owning value interpretation.
- Preserve declared values and source ranges while adding normalized comparison
  values.
- Separate pure scalar normalization from engine-assisted normalization that
  needs a registry, resolver, parser, or compiled CEMT module.
- Apply the same named normalizer to declared values and referenced values
  before comparison.
- Keep normalization annotation-like: it produces derived values and state, but
  it does not mutate source data or decide assertion success by itself.

## Prior Art Applied First

The following prior-art concepts are applied to this design before expanding
comparison or lookup syntax:

- XML Schema's `whiteSpace` facet is useful because it treats normalization as
  an explicit, named datatype constraint that runs before value validation.
  CEM adopts the explicit-normalizer shape, but keeps reference normalization
  separate from general datatype facets.
- JSON Schema's split between assertion keywords and annotations is useful.
  CEM normalizers produce normalized values and state as reference annotations;
  later comparison vocabulary decides whether those states violate a rule.
- Elasticsearch keyword normalizers are useful because they guarantee a scalar
  normalizer yields a single token and apply the same normalization at indexing
  and search time. CEM adopts the same invariant: scalar normalizers produce
  one value, set normalizers are named separately, and both comparison sides
  use the same normalizer.
- RFC 3986's URI comparison ladder is useful because it separates simple string
  comparison, syntax-based normalization, scheme-based normalization, and
  protocol/resource equivalence. CEM keeps `schema:schema-uri` as registry
  identity and `schema:document-uri` as resolver identity instead of silently
  applying generic URI canonicalization.
- RFC 9110 media-type rules are useful because type/subtype tokens are
  case-insensitive while parameters can be semantically significant. CEM adds a
  full media-type record normalizer so essence-only checks do not erase
  parameter information when future rules need it.
- Unicode normalization forms are useful only as explicit future string
  normalizers. CEM does not apply Unicode NFC/NFKC or compatibility folding
  implicitly to identifiers, namespaces, profiles, or exact scalar values.

Concepts deliberately not adopted first: implicit input mutation, automatic
case folding for all strings, automatic Unicode compatibility folding, and
heuristic URI equivalence.

Source references used for this comparison:

- [XML Schema 1.1 Datatypes `whiteSpace` facet](https://www.w3.org/TR/xmlschema11-2/#rf-whiteSpace)
- [JSON Schema Validation overview](https://json-schema.org/draft/2020-12/json-schema-validation#section-3)
- [Elasticsearch keyword normalizer](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/normalizer)
- [RFC 3986 URI normalization and comparison](https://www.rfc-editor.org/rfc/rfc3986#section-6)
- [RFC 9110 media type semantics](https://www.rfc-editor.org/rfc/rfc9110#section-8.3.1)
- [Unicode Normalization Forms](https://www.unicode.org/reports/tr15/)

## Normalized Value Model

A normalized reference value has these conceptual fields:

- `name`: schema-local binding name selected by the future reference rule.
- `normalizer`: one of the vocabulary names below.
- `cardinality`: `one`, `optional`, or `set`.
- `declaredValue`: the source scalar or list item as written by the schema
  author.
- `normalizedValue`: the normalized scalar, record, or set used by later
  comparison.
- `sourceRange`: range of the declared field that produced the value.
- `state`: `valid`, `missing`, `invalid`, or `unresolved`.
- `reason`: stable identifier for invalid or unresolved values.
- `support`: `required` or `optional` for the behavior using the normalizer.
  Unsupported required normalizers are schema/compiler errors when statically
  known, or `unsupported-normalizer` normalization outcomes at runtime.
  Unsupported optional normalizers may be reported as annotations but must not
  make an otherwise independent comparison pass or fail.

`missing`, `invalid`, and `unresolved` are normalization outcomes, not failed
comparisons. The comparison vocabulary decides which outcomes are violations.

## Placement

Normalizers declare an execution placement:

- `pure`: deterministic from the declared scalar or list only. These can be
  implemented as CEM-ML/CEM-QL functions once the standard library exposes the
  required parser helpers.
- `engine-assisted`: deterministic, but needs host context such as schema
  registry lookup, resolver URI finalization, content-type parsing until exposed
  as a pure helper, or compiled CEMT module metadata.

Engine-assisted normalization may read already-loaded registry or resource
metadata. It must not introduce ad hoc package-specific semantics.

## Vocabulary

| Normalizer | Output | Placement | Semantics |
| --- | --- | --- | --- |
| `schema:scalar-exact` | string | pure | Preserve the parsed scalar exactly. No case folding, Unicode normalization, URI normalization, or whitespace trimming beyond CEM-ML attribute parsing. |
| `schema:identifier-token` | string | pure | Validate an identifier-like token and preserve it exactly. Use for content category and profile values whose vocabulary is schema-owned. |
| `schema:media-type` | record | engine-assisted initially; pure when content-type parsing is exposed | Parse a media type or legacy content-type alias into `essence`, `type`, `subtype`, optional `suffix`, and `parameters`. Type, subtype, suffix, and parameter names normalize to lowercase. Parameter values are unquoted and otherwise preserved unless a registered parameter-specific rule declares case-insensitive comparison. Invalid media syntax produces `state=invalid`. |
| `schema:media-type-essence` | string | engine-assisted initially; pure when content-type parsing is exposed | Apply `schema:media-type` and project its lowercase essence. `Text/HTML; Charset=UTF-8` normalizes to `text/html`. Parameter information remains available through `schema:media-type` when needed. |
| `schema:media-type-essence-set` | sorted string set | engine-assisted for registry descriptors; pure for literal lists | Apply `schema:media-type-essence` to each declared content-type claim, drop duplicates after normalization, and keep invalid items addressable by source range. |
| `schema:schema-uri` | URI identity string | engine-assisted | Resolve a schema URI through the schema registry. The normalized value is the registry descriptor's canonical schema URI. If no descriptor resolves, produce `state=unresolved`. Do not treat URI text normalization as equivalent to registry identity. |
| `schema:document-uri` | URI record | engine-assisted | Resolve a resource URI or path against the active document/package base and resolver context. The normalized record contains `declaredUri` and `resolvedUri`; diagnostics keep both when they differ. Resolver policy failures produce `state=unresolved`. |
| `schema:namespace-uri` | string | pure | Preserve the namespace URI string exactly after CEM-ML parsing. Namespace equality remains textual unless a future namespace registry says otherwise. |
| `schema:artifact-name` | string | pure | Preserve an artifact identity token exactly. Intended for manifest-owned artifact ids or path-derived stable artifact names before lookup. |
| `schema:function-name` | string | pure for declared manifest values; engine-assisted for CEMT module declarations | Preserve a declared function name exactly. When normalizing a compiled CEMT module declaration, use the compiler's canonical function identity. |
| `schema:content-category` | string | pure | Alias of `schema:identifier-token` for artifact/converter target categories. Kept as a named normalizer so diagnostics can report the domain-specific value kind. |
| `schema:profile-name` | string | pure | Alias of `schema:identifier-token` for formatter/colorizer/function profiles. Profiles are case-sensitive schema-owned tokens. |

## Registry-Derived Values

Registry-backed rules must normalize descriptor projections through the same
normalizers as declared manifest values:

- Schema descriptor `contentTypes` use `schema:media-type-essence-set`.
- Schema descriptor URI uses `schema:schema-uri`.
- Schema descriptor namespaces use `schema:namespace-uri` as a set.
- CEMT function output metadata uses `schema:function-name`,
  `schema:media-type`, `schema:media-type-essence`, `schema:schema-uri`,
  `schema:content-category`,
  and `schema:profile-name` as applicable.

This keeps a future comparison such as "endpoint content type is a member of
the endpoint schema's registered content-type essences" independent from the
Rust package rule that currently performs it.

## Invalid And Unresolved Reasons

The first stable reason identifiers are:

- `missing-value`: the selected field is absent.
- `invalid-scalar`: the selected field does not satisfy the normalizer's scalar
  grammar.
- `invalid-media-type`: media type parsing failed.
- `unresolved-schema`: schema registry lookup failed.
- `unresolved-document`: resolver lookup or URI finalization failed.
- `unresolved-function`: compiled artifact metadata did not expose the named
  function.
- `unsupported-normalizer`: the active engine cannot execute the requested
  normalizer.

Comparison rules may project these reasons into `invalidValues`,
`expectedValues`, or a dedicated unresolved-reference detail, but that
projection is not part of this normalization vocabulary.

## Application To Current Rust-Backed Checks

- Converter endpoint content-type/schema compatibility normalizes endpoint
  `@content-type` with `schema:media-type-essence`, endpoint `@schema` with
  `schema:schema-uri`, and the referenced schema descriptor content types with
  `schema:media-type-essence-set`.
- Example content-type/schema compatibility uses the same three normalizers.
- Package schema source metadata consistency normalizes manifest schema URI,
  manifest content-type claims, and manifest namespace claims with
  `schema:schema-uri`, `schema:media-type-essence-set`, and
  `schema:namespace-uri`.
- Artifact function lookup normalizes manifest `@function-name` with
  `schema:function-name` and CEMT module declarations with the engine-assisted
  `schema:function-name` form.
- Artifact function metadata matching normalizes target content type, target
  schema, target category, function profile, and function name with the
  corresponding vocabulary terms above.

## Implementation Notes

- Existing Rust helpers such as media-type essence extraction become
  implementations of these normalizers, not package-specific rule logic.
- CEM-QL equality keeps its existing exact-value semantics. Normalization must
  happen explicitly through a named normalizer before any comparison.
- Scalar normalizers must return exactly one normalized value when valid. Any
  normalizer that can return multiple values must be named as a set normalizer
  and produce deterministic ordering.
- A comparison must identify the normalizer for each side. Most reference
  checks should use the same normalizer on both sides; mixed normalizers are
  allowed only when the comparison vocabulary explicitly says how their outputs
  are comparable.
- URI, Unicode, and case normalization are never implicit. They happen only
  through a named normalizer with documented equivalence semantics.
- Source ranges attach to the declared field or list item that fed the
  normalizer. Registry-derived values attach to the registry descriptor range
  when available; otherwise they carry descriptor identity only.
- Normalized sets are sorted and duplicate-free for deterministic diagnostics.
