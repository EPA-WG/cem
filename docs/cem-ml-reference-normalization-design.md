# CEM-ML Reference Normalization Vocabulary

Status: design decision for schema-owned reference constraints.

This note defines the normalized value vocabulary used by
`schema:reference-resolution`. Comparison operators are defined separately in
[`cem-ml-reference-comparison-design.md`](cem-ml-reference-comparison-design.md).
Selector syntax, lookup syntax, execution boundaries, and diagnostic projection
are defined in
[`cem-ml-reference-vocabulary-design.md`](cem-ml-reference-vocabulary-design.md).

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
  protocol/resource equivalence. CEM keeps declared schema URI text, resolved
  schema descriptor identity, URI-only schema projections, and document resolver
  identity separate instead of silently applying generic URI canonicalization.
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
- `provenance`: optional normalizer-supplied metadata that explains how the
  normalized value was derived without becoming part of normal equality. For
  schema identity this includes the declared URI, parsed version constraint,
  AC-V-10 match rule, registry layer, and descriptor source range when known.
- `state`: `valid`, `missing`, `invalid`, `unresolved`, or `unsupported`.
- `reason`: stable identifier for every non-valid value.
- `support`: `required` or `optional` for the behavior using the normalizer.
  Unsupported required normalizers are schema/compiler errors when statically
  known per AC-S-7 and AC-S-8, or `unsupported-normalizer` normalization
  outcomes at runtime when the need is discovered dynamically.
  Unsupported optional normalizers may be reported as annotations but must not
  make an otherwise independent comparison pass or fail.

`missing`, `invalid`, `unresolved`, and `unsupported` are normalization
outcomes, not failed comparisons. The comparison vocabulary decides which
outcomes are violations.

## Placement

Normalizers declare an execution placement:

- `pure`: deterministic from the declared scalar or list only. These can be
  implemented as CEM-ML/CEM-QL functions once the standard library exposes the
  required parser helpers.
- `engine-assisted`: deterministic, but needs host context such as schema
  registry lookup, AC-F-2 schema-source resolution, AC-P-6 namespace metadata,
  resolver URI finalization, content-type parsing until exposed as a pure
  helper, or compiled CEMT module metadata.

Engine-assisted normalization may read already-loaded registry or resource
metadata and may use policy-gated resolver results supplied by the host. It
must not introduce ad hoc package-specific semantics.

## Schema Identity Contract

Schema identity normalization follows the normative schema version identity
rules in [`cem-ml-ac.md`](cem-ml-ac.md#31-schema-version-identity).

The declared schema URI is not complete schema identity. Its optional version
tail is an author constraint: no tail means an unconstrained stable version,
`/MAJOR` constrains the major series, `/MAJOR.MINOR` constrains the minor
series, and a full SemVer tail constrains resolution by the AC-V-10 forgiving
rule. A prerelease tail resolves only by exact prerelease match. Build metadata
in the URI tail is matched case-sensitively when specified; a URI tail without
build metadata matches embedded versions with any build metadata. Unversioned,
major, major-minor, and non-prerelease full-version URI forms do not resolve to
prerelease descriptors. The loaded descriptor's complete `descriptor.version`
SemVer is the authoritative version.

The complete normalized schema identity is the AC-V-9 pair:

```text
{
  uri: <descriptor stable schema URI>,
  embeddedVersion: <descriptor.version full SemVer 2.0>
}
```

`embeddedVersion` includes prerelease and build metadata verbatim. It is the
schema fingerprint and policy-stamp version input per AC-V-12. The declared URI
tail, whether absent, partial, or full, does not enter that fingerprint when two
declarations resolve to the same embedded version.

The AC-CC-3 policy stamp still retains the declared schema reference required
by the active scope, and AC-P-6 namespace metadata stamps retain the resolved
metadata source. Fingerprint equality and policy satisfaction are therefore
different questions: author shorthand is preserved for policy/provenance, while
cache identity uses the resolved embedded version required by AC-V-12.

Resolution provenance remains attached to the normalized envelope instead of
the comparison value:

```text
{
  declaredUri: <source value>,
  versionConstraint: unconstrained | major | major-minor | full,
  matchRule: unconstrained | major | major-minor | full | prerelease-exact
}
```

The engine records the AC-V-13 `cem.v.semver_resolved` event with the declared
URI, embedded full version, and match rule. Rules that intentionally compare
only the stable URI must use the explicit URI projection; URI-only equality is
never complete schema identity equality.

AC-F-2 schema source forms feed this identity contract only after their own
source-kind resolution. `schema:schema-uri-declaration` applies to URI literal
fields. Inline schema `cem:name` bindings are scope-chain aliases and must not
be treated as URI declarations or cache identity. When an inline, file, or
selector-based schema source has produced a loaded descriptor, comparisons use
the descriptor's resolved `schema:schema-identity`; provenance records the
source form, such as `declaredUri`, `declaredName`, resolver URI, selected
expression, or inline `inline:<sha256-of-body>` cache identity as applicable.

AC-P-4 scope identity projects `schemaUri` for human-facing scope identity, but
that projection is not the complete schema identity. Scope, cache, policy, and
namespace-dispatch metadata that depend on schema version must also carry the
resolved embedded version.

## Namespace Metadata Contract

`schema:namespace-uri` is exact namespace text. It does not select a schema or
content type by itself.

Namespace-driven dispatch follows AC-P-6.1 and AC-P-6.5. A namespace used for
dispatch resolves through the local-first metadata chain:

```text
inline descriptor -> workspace registry -> package manifests -> external registry
```

External registry resolution is explicit opt-in and host-policy gated. The
normalized dispatch metadata is:

```text
{
  namespaceUri: <declared namespace URI>,
  contentType: <resolved content type>,
  schemaUri: <resolved schema identity uri>,
  schemaVersion: <resolved schema identity embeddedVersion>
}
```

The `schemaUri` and `schemaVersion` fields are the `schema:schema-identity`
pair projected into AC-P-6's metadata names. A version segment in the namespace
URI is treated as the namespace schema's AC-V-10 constraint; for example,
`https://cem.dev/ns/core/1` constrains the dispatched schema to major version
`1`. The metadata source participates in AC-CC-1 cache identity and AC-CC-3
policy stamps. If a namespace has no metadata and no explicit AC-F-2 schema
form, AC-P-6.7 scope policy selects `reject`, `allow`, or `ignore`; the
normalizer records that as provenance rather than inventing a schema identity.

## Vocabulary

| Normalizer | Output | Placement | Semantics |
| --- | --- | --- | --- |
| `schema:scalar-exact` | string | pure | Preserve the parsed scalar exactly. No case folding, Unicode normalization, URI normalization, or whitespace trimming beyond CEM-ML attribute parsing. |
| `schema:identifier-token` | string | pure | Validate an identifier-like token and preserve it exactly. Use for content category and profile values whose vocabulary is schema-owned. |
| `schema:media-type` | record | engine-assisted initially; pure when content-type parsing is exposed | Parse a media type or legacy content-type alias into `essence`, `type`, `subtype`, optional `suffix`, and `parameters`. Type, subtype, suffix, and parameter names normalize to lowercase. Parameter values are unquoted and otherwise preserved unless a registered parameter-specific rule declares case-insensitive comparison. Invalid media syntax produces `state=invalid`. |
| `schema:media-type-essence` | string | engine-assisted initially; pure when content-type parsing is exposed | Apply `schema:media-type` and project its lowercase essence. `Text/HTML; Charset=UTF-8` normalizes to `text/html`. Parameter information remains available through `schema:media-type` when needed. |
| `schema:media-type-essence-set` | sorted string set | engine-assisted for registry descriptors; pure for literal lists | Apply `schema:media-type-essence` to each declared content-type claim, drop duplicates after normalization, and keep invalid items addressable by source range. |
| `schema:schema-uri-declaration` | string | pure | Preserve the declared schema URI exactly after CEM-ML parsing and validate only schema-URI declaration syntax. If the last path segment is an AC-V-10 version tail, expose the parsed constraint as provenance. This normalizer does not resolve a descriptor, select a version, or apply generic URI text canonicalization. |
| `schema:schema-identity` | record | engine-assisted | Resolve a schema reference through the AC-F-2/AC-P-6-aware schema resolution context using AC-V-9 through AC-V-13. The normalized value is the complete identity record `{ uri, embeddedVersion }`, where `uri` is the matched descriptor's stable schema URI and `embeddedVersion` is the descriptor's complete SemVer 2.0 string. Provenance retains the source form, declared URI or alias when present, version constraint, resolver or metadata source, and match rule; unresolved or ambiguous resolution produces `state=unresolved`. |
| `schema:schema-uri` | URI projection string | engine-assisted | Apply `schema:schema-identity` and project only the resolved descriptor `uri`. This is a lossy compatibility normalizer for rules that explicitly want stable URI equality without version identity. It must not be used where complete schema identity, cache identity, or schema-version compatibility is intended. |
| `schema:document-uri` | URI record | engine-assisted | Resolve a resource URI or path against the active document/package base and resolver context. The normalized record contains `declaredUri` and `resolvedUri`; diagnostics keep both when they differ. Non-local or otherwise policy-gated resolution follows the active scope policy; resolver policy failures produce `state=unresolved`. |
| `schema:namespace-uri` | string | pure | Preserve the namespace URI string exactly after CEM-ML parsing. Namespace equality remains textual. Dispatch and content-type/schema selection use `schema:namespace-metadata`, not this text normalizer. |
| `schema:namespace-metadata` | record | engine-assisted | Resolve namespace metadata per AC-P-6.1 through the local-first metadata chain. The normalized record contains `{ namespaceUri, contentType, schemaUri, schemaVersion }`, where `schemaUri` and `schemaVersion` are the resolved `schema:schema-identity` pair. Missing metadata without an explicit schema form is governed by AC-P-6.7 scope policy. |
| `schema:artifact-name` | string | pure | Preserve an artifact identity token exactly. Intended for manifest-owned artifact ids or path-derived stable artifact names before lookup. |
| `schema:function-name` | string | pure for declared manifest values; engine-assisted for CEMT module declarations | Preserve a declared function name exactly. When normalizing a compiled CEMT module declaration, use the compiler's canonical function identity. |
| `schema:content-category` | string | pure | Alias of `schema:identifier-token` for artifact/converter target categories. Kept as a named normalizer so diagnostics can report the domain-specific value kind. |
| `schema:profile-name` | string | pure | Alias of `schema:identifier-token` for formatter/colorizer/function profiles. Profiles are case-sensitive schema-owned tokens. |

## Registry-Derived Values

Registry-backed rules must normalize descriptor projections through the same
normalizers as declared manifest values:

- Schema descriptor `contentTypes` use `schema:media-type-essence-set`.
- Schema descriptor declarations use `schema:schema-uri-declaration` for
  source/manifest consistency checks that run before registry admission.
- Schema descriptor identity uses `schema:schema-identity`; when the descriptor
  itself is the registry-derived value, the engine lifts `{ uri,
  embeddedVersion }` directly from the descriptor instead of resolving its URI
  again.
- Schema descriptor namespaces use `schema:namespace-uri` as a set.
- Namespace-dispatch metadata uses `schema:namespace-metadata`; the resolved
  metadata source participates in cache and policy identity per AC-P-6.1.
- CEMT function output metadata uses `schema:function-name`,
  `schema:media-type`, `schema:media-type-essence`, `schema:schema-identity`,
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
- `invalid-schema-uri`: schema URI declaration parsing failed.
- `invalid-schema-version-constraint`: a schema URI tail looks like a version
  constraint but is not one of the AC-V-10 forms.
- `unresolved-schema`: schema registry lookup failed.
- `unresolved-namespace-metadata`: no namespace metadata resolved before
  AC-P-6.7 policy selected an outcome.
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
  `schema:schema-identity`, and the referenced schema descriptor content types
  with `schema:media-type-essence-set`.
- Example content-type/schema compatibility uses the same three normalizers.
- Package schema source metadata consistency normalizes manifest schema URI,
  manifest content-type claims, and manifest namespace claims with
  `schema:schema-uri-declaration`, `schema:media-type-essence-set`, and
  `schema:namespace-uri`; after provisional descriptor construction, registry
  consumers use `schema:schema-identity`.
- Namespace-driven content-type/schema dispatch normalizes the active namespace
  with `schema:namespace-metadata`. An explicit AC-F-2 schema form may refine
  the resolved schema within the namespace-selected content type, but a direct
  form that selects a different content type is invalid per AC-P-6.1.
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
- Complete schema identity equality compares the `schema:schema-identity`
  normalized record. `schema:schema-uri` equality compares only the explicit
  URI projection and therefore cannot stand in for schema identity equality.
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
