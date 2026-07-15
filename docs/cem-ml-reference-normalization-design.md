# CEM-ML Reference Normalization Vocabulary

Status: design decision for schema-owned reference constraints.

This note defines the normalized value vocabulary used by
`schema:reference-resolution`. It intentionally stops before comparison
operators, selector syntax, lookup syntax, and diagnostic projection syntax;
those are separate todo items.

## Goals

- Let schemas name the normalization applied before reference checks.
- Keep package-specific Rust branches from owning value interpretation.
- Preserve declared values and source ranges while adding normalized comparison
  values.
- Separate pure scalar normalization from engine-assisted normalization that
  needs a registry, resolver, parser, or compiled CEMT module.

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
| `schema:media-type-essence` | string | engine-assisted initially; pure when content-type parsing is exposed | Parse a media type or legacy content-type alias, discard parameters, and lowercase the essence. `Text/HTML; Charset=UTF-8` normalizes to `text/html`. Invalid media syntax produces `state=invalid`. |
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
  `schema:media-type-essence`, `schema:schema-uri`, `schema:content-category`,
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
- Source ranges attach to the declared field or list item that fed the
  normalizer. Registry-derived values attach to the registry descriptor range
  when available; otherwise they carry descriptor identity only.
- Normalized sets are sorted and duplicate-free for deterministic diagnostics.
