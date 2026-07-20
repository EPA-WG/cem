# CEM-ML Reference Normalization Vocabulary

Status: accepted target design for schema-owned reference constraints;
implementation pending.

This note defines the normalized value vocabulary used by
`schema:reference-resolution`. Comparison operators are defined separately in
[`cem-ml-reference-comparison-design.md`](cem-ml-reference-comparison-design.md).
Selector syntax, lookup syntax, execution boundaries, and diagnostic projection
are defined in
[`cem-ml-reference-vocabulary-design.md`](cem-ml-reference-vocabulary-design.md).
The current implemented schema-package surface remains documented in
[`../packages/cem_ml/schema-packages/schema-package/v1/README.md`](../packages/cem_ml/schema-packages/schema-package/v1/README.md).

## Goals

- Let schemas name the normalization applied before reference checks.
- Keep package-specific Rust branches from owning value interpretation.
- Preserve declared values and source ranges while adding normalized comparison
  values.
- Separate pure scalar normalization from engine-assisted normalization that
  needs a registry, resolver, parser, or compiled CEMT module.
- Apply compatible item-normalization and equivalence semantics to declared
  values and referenced values before comparison, even when one side is a named
  set normalizer.
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
  one value, set normalizers are named separately, and comparison sides expose
  compatible item-normalization semantics.
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

- `binding`: schema-local normalized-envelope identity selected by the
  reference rule. Reserve `name` for domain values such as artifact names,
  function names, profile names, element names, or namespace binding names.
- `normalizer`: one of the vocabulary names below.
- `itemNormalizer`: the scalar/item normalizer that defines comparison
  equivalence. For scalar operands this is the same as `normalizer`; for set
  normalizers it is declared by the collection normalizer.
- `cardinality`: `one`, `optional`, `set`, or conceptual `sequence`.
- `shape`: `scalar` or `record`.
- `declaredValue`: the parsed semantic scalar, record, or list item selected
  from source, with scalar type preserved.
- `sourceLexeme`: the exact authored token spelling when the value came from a
  source token and lexical spelling was captured. Registry-derived or
  synthesized values may omit it.
- `normalizedValue`: the normalized scalar, record, or set used by later
  comparison.
- `sourceRange`: range of the declared field that produced the value. This is
  location metadata and never participates in equality.
- `provenance`: optional normalizer-supplied metadata that explains how the
  normalized value was derived without becoming part of normal equality. For
  schema identity this includes the declared URI, parsed version constraint,
  AC-V-10 match rule, registry layer, and descriptor source range when known.
- `state`: `valid`, `missing`, `invalid`, `unresolved`, or `unsupported`.
- `reason`: stable identifier for every non-valid value.
- `support`: `required` or `optional` for the behavior using the normalizer.
  Unsupported required normalizers are schema/compiler errors when statically
  known per AC-S-7 and AC-S-8, or `state=unsupported` normalization outcomes
  at runtime when the need is discovered dynamically.
  Unsupported optional normalizers may be reported as annotations but must not
  make an otherwise independent comparison pass or fail.

`missing`, `invalid`, `unresolved`, and `unsupported` are normalization
outcomes, not failed comparisons. The comparison vocabulary decides which
outcomes are violations.

`pending` is not a terminal normalized state. Deferred or asynchronous lookup
execution may track `pending` internally, but it must either defer comparison or
finalize to one of the terminal states above before diagnostics are emitted. A
lookup that can still complete must not be reported as final `unresolved`.

Source compatibility may accept `@support="soft"` only as syntax sugar. It
lowers immediately to `support=optional` plus an additive reporting/provenance
flag that requests a warning when the capability is unsupported. Normalized IR
and comparison metadata use only `required` or `optional`.

## Cardinality, Shape, And Collection Provenance

Cardinality and shape are independent axes:

```text
cardinality: one | optional | set | sequence
shape:       scalar | record
```

`scalar`, `record`, `set`, and `record-set` are not peer concepts. A record set
is `cardinality=set, shape=record`. A scalar set is `cardinality=set,
shape=scalar`. Candidate cardinality remains separately scoped to candidate
selection and uses its own `zero-or-more|optional|one-or-more|exactly-one`
vocabulary.

`sequence` is reserved in the conceptual model for ordered reference lists that
preserve duplicates as semantic data, such as future ARIA IDREF-sequence
checks. It is deferred from the initial schema-package reference-resolution
surface. Initial package checks use `one`, `optional`, and `set` only.

The initial normalizer registry therefore has no host-language ID or IDREF-list
normalizer. HTML/ARIA IDs, fragment identifiers, and slot target names must not
be normalized with `schema:identifier-token`, and ordered IDREF sequences must
not be normalized as sorted sets. A future runtime reference tier must add
explicit host-language ID normalizers, sequence comparison semantics,
snapshot/revision identity, pending-dependency behavior, and recomputation when
the underlying reference registry changes.

Set normalizers are named normalizers with an explicit item normalizer. They
own item normalization, duplicate handling, deterministic comparison order, and
per-item provenance.

A valid set result has two views:

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

Comparisons consume `comparisonSet`. Diagnostics and structured provenance use
`items` so invalid entries, duplicate origins, and source order remain
addressable.

## Normalizer Compatibility

Normalizer symmetry means the same item-normalization and equivalence
semantics, not necessarily the same named collection normalizer on both
comparison operands.

A scalar operand normalized by `N` can compare against a set operand normalized
by a named `set-of(N)` normalizer only when the comparison operator declares
that scalar/set pairing compatible. `schema:member-of` is the canonical case:
the actual scalar `schema:content-type-identity` value may be tested against an
expected `schema:content-type-identity-set` because both expose
`itemNormalizer=schema:content-type-identity`. Pure RFC media syntax checks may
use the same scalar/set pattern with `schema:media-type-essence` and
`schema:media-type-essence-set`.

Set-to-set operators compare duplicate-free `comparisonSet` values only when
their `itemNormalizer` values, or explicitly declared item equivalence
semantics, are compatible. `schema:equals` does not treat a scalar value and a
set containing that scalar as equal unless a future operator-specific
projection rule explicitly says so.

Mixed normalizer names without operator-declared compatible item outputs are
malformed comparison declarations. They must be rejected before value
comparison rather than silently coerced.

## Exact Scalar Semantics

Exact scalar normalization is typed. `schema:scalar-exact` compares the parsed
semantic scalar as `(type, value)` and performs no implicit coercion. A string
`"1"`, integer `1`, decimal `1.0`, boolean `true`, and null are distinct even
when their source lexemes look similar or a host language would coerce them.

`schema:string-exact` is the text-only exact normalizer. It accepts only parsed
string/text values and compares codepoints exactly. It does not stringify
numbers, booleans, null, records, or lists.

Lexical spelling is a separate concern. `schema:source-lexeme-exact` compares
`sourceLexeme`, not `declaredValue`; use it only for contracts where authored
spelling, quote style, numeric spelling, escape spelling, or other token text is
the semantic value. Validators must not reconstruct `sourceLexeme` from
`declaredValue`.

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

## Staged Lookup Normalization

Lookup-based operands have two separate normalized envelopes:

- `lookupKey`: the normalized key used to find a target descriptor, document,
  function, or in-memory binding.
- `comparableResult`: the normalized value consumed by comparison.

The lookup key is provenance. It is not an `actual`, `expected`, or
`forbidden` comparison operand, and its binding must not replace the operand's
public `binding`.

The canonical staged order is:

```text
source extraction
-> source cardinality guard
-> lookup-key normalization
-> lookup
-> raw-result cardinality and shape guard
-> comparable-result extraction
-> comparable-result normalization
-> normalized-result cardinality and shape guard
-> state policy
-> comparison
-> diagnostic projection
```

Normalization stages annotate values and finalize operand state. They do not
emit assertion violations by themselves; state policy, comparison, and
diagnostic projection own that transition.

For operands without lookup, the comparable-result extraction stage is the
source value itself. For operands with lookup, the operand normalizer applies to
the comparable result selected from the lookup result; lookup key normalizers
live on the lookup key declarations.

The endpoint content-type/schema check illustrates the split. Endpoint
`@schema` normalizes to `schema:schema-identity` as a lookup key. Endpoint
`@content-type` then normalizes to `schema:content-type-identity` using that
schema context. The referenced schema descriptor's `contentTypes` field
normalizes to `schema:content-type-identity-set` as the expected comparison
value. The expected operand binding is therefore `content-type`; the schema key
binding remains lookup provenance.

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

Schema and namespace operands remain separate domains even when namespace
metadata resolves schema fields. A `schema:namespace-uri` value is never
comparable to `schema:schema-uri-declaration`, `schema:schema-uri`, or
`schema:schema-identity`. Schema-reference checks consume schema-source or
descriptor URI/identity fields. Namespace-claim checks consume namespace fields
only. The `schemaUri` and `schemaVersion` values produced by
`schema:namespace-metadata` are dispatch metadata and provenance; they do not
make the namespace text itself a schema identity.

## Schema V1 Compatibility Adapter

The current schema definition language v1 uses `schema @namespace` to identify
the target schema being described. That behavior is a versioned compatibility
adapter, not the long-term schema identity model.

For schema v1 sources, the loader may read `schema @namespace` as a legacy
target-schema reference only inside the schema-v1 adapter. The adapter must
project explicit schema-source identity data for downstream checks: a schema URI
declaration when the source provides one, and the complete
`schema:schema-identity` once descriptor resolution has succeeded. It also
retains the original namespace attribute spelling and source range as
provenance. Downstream comparison rules then use schema normalizers for
schema-reference checks and namespace normalizers for namespace-claim checks.
They must not compare a manifest schema URI directly with the legacy namespace
attribute.

A later schema source version should expose an explicit schema identity field
or descriptor identity record instead of preserving namespace-as-identity.

## Package Validation Bootstrap

Local schema-package validation has two distinct phases: declaration
consistency before registry admission, then registry-backed validation against a
provisional overlay. A package must not be inserted into the host catalog in
order to prove that it is valid.

The bootstrap sequence is:

```text
validate package manifest shape against trusted built-ins
-> resolve declared package-local schema source
-> run pure manifest/source declaration consistency checks
-> construct isolated provisional descriptor
-> run registry-backed package checks against trusted registries plus overlay
-> admit descriptor to host catalog only after required checks pass
```

Pure declaration consistency checks compare authored manifest metadata with the
referenced schema source without resolving the package through the global schema
registry:

- manifest schema URI and schema source URI use
  `schema:schema-uri-declaration`;
- manifest and schema-source content-type claims that are strict RFC media
  types use `schema:media-type-essence-set`;
- manifest namespace claims and schema source namespace claims use
  `schema:namespace-uri-set`.

These declaration checks do not cross-bind schema and namespace operands. A
missing schema-source URI is not satisfied by a namespace claim, and namespace
agreement does not prove schema URI agreement.

Only after those checks pass may the validator construct a provisional
descriptor. The provisional descriptor is isolated to the current package
validation run, records package-local provenance, and is not visible to the
host runtime catalog. If it collides with an already trusted descriptor identity
or another provisional descriptor, validation fails instead of shadowing.
Registered content identity checks, including compatibility aliases and legacy
bare aliases, run only after the provisional descriptor exists because alias
ownership and ambiguity are registry facts.

Registry-backed checks then run against a composed validation view:

```text
trusted built-ins + explicit trusted dependencies + current provisional overlay
```

Endpoint, example, artifact, namespace-dispatch, and registry-identity checks
may use `schema:schema-identity` in this phase. Catalog admission happens only
after every required check succeeds. Optional unsupported capabilities remain
normalization outcomes; required failures block admission.

## Media Syntax Versus Registered Content Identity

Media syntax and registered content identity are separate domains.

`schema:media-type`, `schema:media-type-essence`, and
`schema:media-type-essence-set` parse only RFC media-type syntax. They do not
apply schema-package alias tables, CEM-QL `accepts` aliases, file-extension
heuristics, or legacy spellings such as `custom-element-xslt`. A value without
valid media-type syntax produces `state=invalid` with
`reason=invalid-media-type`.

Registered content identity is engine-assisted and registry-backed.
`schema:content-type-identity` accepts RFC media identities and registered
aliases, including legacy aliases when a schema package claims them. It
requires enough registry context to select one owner. The context may come from
an explicit schema identity, namespace metadata, package validation overlay, or
host content registry. If no registered owner matches, the result is
`state=unresolved, reason=unknown-content-type`. If more than one owner matches
and no schema context disambiguates the value, the result is
`state=unresolved, reason=ambiguous-content-type`. If the value resolves but is
not claimed by the required schema context, the result is
`state=invalid, reason=content-type-schema-mismatch`.

The normalized content identity value is a registry identity record:

```text
{
  contentType: <canonical registered content type>,
  schemaIdentity: <owning schema identity, when known>,
  routingProfile: <alias-specific routing profile, when behaviorally distinct>
}
```

Provenance preserves the declared spelling, whether the declaration matched an
RFC media type or an alias, the alias owner, registry layer, and source range.
Alias spelling does not enter equality unless the registry declares a distinct
routing profile.

The `schema:media-type` name is retained because symbol lookup is kinded:
datatype names, normalizer names, behavior names, and capability names live in
separate registries. The datatype and normalizer named `schema:media-type`
share one media grammar primitive; unkinded references that cannot select a
symbol kind are schema-definition errors.

CEM-QL's AC-QA-1.1 alias table is scoped to `read(uri, accepts?)` negotiation
and compiled `accepts` policy stamps. It is not a global reference-normalization
alias table. Reference normalization uses schema/content registries through
`schema:content-type-identity`.

## Identifier And Profile Domains

`schema:identifier-token` is the reference-normalization view of the schema
scalar datatype `schema:identifier`, which is based on the CEM-ML
`cemml:identifier` / local-name grammar. For reference normalization, that
grammar is:

```text
identifier-token := identifier-start identifier-char*
identifier-start := [A-Za-z_]
identifier-char  := [A-Za-z0-9_-]
```

Identifier-token equality is exact codepoint equality over the parsed string.
No case folding, Unicode normalization, hyphen/underscore conversion,
namespace-prefix expansion, URI normalization, whitespace trimming, or list
splitting is applied. Values containing dots, colons, slashes, whitespace,
empty text, or source quoting required only to escape non-identifier characters
are not valid identifier tokens.

Use `schema:identifier-token` only for schema-owned local tokens such as
content categories, check kinds, simple enum values, and contract-local names.
It is not a catch-all for profile symbols, function names, artifact identities,
resource paths, namespace names, schema URIs, or host-language IDs.

`schema:profile-name` uses the existing CEM-ML
`schema:symbol-reference` / `cemml:symbol-reference` grammar: one or more
identifier-token segments separated by a single dot.

```text
profile-name := identifier-token ( "." identifier-token )*
```

Profile equality is exact and case-sensitive over the whole symbol. Package
qualified profiles such as `acme.showcase.format-tree` are valid. Empty
segments, leading or trailing dots, repeated dots, path separators, URI syntax,
and case-insensitive aliases are not valid profile names.

## Function Name And Identity Domains

`schema:function-name` is the authored exported function name. It uses the same
dotted `schema:symbol-reference` grammar as profiles, but it belongs to the
function-export domain rather than the profile domain.

```text
function-name := identifier-token ( "." identifier-token )*
```

Function-name equality is exact and case-sensitive over the parsed lexical
name. It does not prove that a function exists in a compiled module, identify
the artifact that declared it, or include target metadata such as content type,
schema, category, subject, or profile.

`schema:function-identity` is the engine-assisted identity for a compiled CEMT
function declaration. Its normalized value is a record:

```text
{
  moduleIdentity: <compiled module or artifact identity>,
  canonicalName: <compiler canonical exported function name>,
  functionProfile?: <schema:profile-name>,
  functionKind?: <schema:identifier-token>
}
```

`moduleIdentity` is supplied by artifact or document resolution, usually as
`schema:artifact-identity` for schema-package CEMT assets. `canonicalName` is
the compiler's canonical export spelling and must compare with manifest
`@function-name` through the `schema:function-name` normalizer. The optional
profile and kind fields are compared only when the schema contract declares
those field pairs.

## Artifact Name And Identity Domains

`schema:artifact-name` is an authored manifest artifact ID. It uses
`schema:identifier-token` grammar and is valid only when the source explicitly
declares an artifact ID field. It is not derived from a file path, URI
basename, function name, artifact kind, or registry lookup.

`schema:artifact-identity` is the engine-assisted identity produced after
`schema:document-uri` resolution for a package or registry artifact reference.
Its normalized value is a record:

```text
{
  declaredUri: <authored artifact URI or package-relative path>,
  resolvedUri: <resolver-final artifact URI>,
  packageContext?: <owning package or registry context>,
  artifactKind?: <schema:identifier-token>,
  sourceContentType?: <schema:content-type-identity or schema:media-type>,
  sourceSchemaIdentity?: <schema:schema-identity>
}
```

Artifact identity equality is based on the resolved artifact URI inside the
same package or registry context, plus artifact kind when the contract requires
kind-specific identity. `declaredUri`, source spelling, source range, resolver
steps, and aliases remain provenance. A path-derived artifact identity must not
be compared as `schema:artifact-name`, and an authored artifact ID must not be
invented from a path.

## Composite Function And Artifact Lookup Keys

Function/artifact lookups use composite normalized records rather than
concatenated strings. A package artifact function lookup key is built from the
declared key parts that the schema contract requires:

```text
{
  artifactIdentity: <schema:artifact-identity>,
  functionName: <schema:function-name>,
  sourceContentType?: <schema:content-type-identity or schema:media-type>,
  sourceSchemaIdentity?: <schema:schema-identity>,
  targetContentType?: <schema:content-type-identity>,
  targetSchemaIdentity?: <schema:schema-identity>,
  targetCategory?: <schema:content-category>,
  functionProfile?: <schema:profile-name>,
  subjectType?: <schema:identifier-token>
}
```

Each field is normalized independently with its domain normalizer before the
record participates in lookup or comparison. Composite equality is fieldwise
over the declared normalized fields. Undeclared fields do not participate in
identity; optional declared fields use explicit state or presence policy.
Authored spellings, source ranges, lookup key bindings, resolver provenance,
and registry provenance stay attached to each key part.

Validators must not build a composite key by joining raw strings such as
`path + "#" + function-name`, by comparing basenames, or by normalizing every
component through `schema:identifier-token`.

## Document URI Identity Contract

`schema:document-uri` finalizes a declared resource URI or path into document
identity. It is not a fetch, read, list, parse, schema load, or readability
check.

Document URI finalization uses the active resolver context:

```text
{
  declaredUri: <authored URI/path/specifier>,
  resolvedUri: <final URI after base/module-map/protocol resolution>,
  effectiveBaseUri: <QueryContextScope.effectiveBaseUri>,
  resolverPurpose: <config | input | moduleMap | output | report | observeEvents | schema-owned purpose>,
  packageContext?: <current package or dependency context>,
  moduleMapContext?: <root or scope module-map identity>,
  policy?: <active resolver/scope policy>
}
```

The comparison value contains only `declaredUri` and `resolvedUri`. Base URI,
resolver purpose, package/module-map context, policy decisions, resolver
mapping steps, cache hints, and source range are provenance. Those provenance
fields participate in cache or diagnostics only when the host policy or
declared lookup says they do.

Finalization rules:

- relative URI/path values resolve against `QueryContextScope.effectiveBaseUri`
  before resolver dispatch;
- `./...` values resolve against the active package/context root when the
  schema declares package context;
- protocol-qualified values resolve through the matching resolver for the
  declared purpose;
- bare values resolve through module-map context when one is active;
- policy denial finalizes to `state=unsupported, reason=policy-denied`;
- malformed URI/path syntax finalizes to `state=invalid,
reason=invalid-document-uri`;
- missing resolver or mapping context finalizes to `state=unresolved,
reason=unresolved-document`.

`schema:document-uri` must not assert that `resolvedUri` exists or is readable.
Resource existence, readability, parsing, schema loading, and CEMT compilation
belong to explicit lookup or resource behaviors such as
`schema:resource-readable`, `schema:resource-parse`,
`schema:schema-identity`, or `schema:function-identity`.

Language-specific reference semantics are separate capabilities. JSON Schema
`$ref` and `$dynamicRef` resolution depends on `$id`, anchors, dynamic anchors,
dialect, and dynamic scope, so it must not be modeled as generic
`schema:document-uri` URL joining. Namespace registry lookup likewise does not
change `schema:namespace-uri` exact textual equality; descriptor-equivalent
namespace identity would require a separate normalizer or capability.

## Identifier And Artifact Split Examples

The profile value `acme.showcase.format-tree` is valid
`schema:profile-name` because it is a dotted symbol. It is not a valid
`schema:identifier-token` because identifier tokens do not contain dots.
`acme.showcase.format-tree` and `Acme.showcase.format-tree` are distinct
profile values.

An authored manifest field such as `@id="formatter-main"` may normalize as
`schema:artifact-name`. A manifest field such as
`@path="formatters/cem-format-tree.cemt"` does not normalize as an artifact
name. It first normalizes as `schema:document-uri`, then as
`schema:artifact-identity`:

```text
{
  declaredUri: "formatters/cem-format-tree.cemt",
  resolvedUri: "cem+repo://.../cem-ml/v1/formatters/cem-format-tree.cemt",
  packageContext: "cem-ml@1",
  artifactKind: "formatter"
}
```

The manifest field `@function-name="cem.format-tree"` normalizes as
`schema:function-name`. After the resolved CEMT artifact is compiled, the
matching declaration normalizes as `schema:function-identity`:

```text
{
  moduleIdentity: <artifact identity above>,
  canonicalName: "cem.format-tree",
  functionKind: "format-function"
}
```

A formatter artifact lookup that checks executable output metadata uses a
composite key with independently normalized parts:

```text
{
  artifactIdentity: <schema:artifact-identity from @path>,
  functionName: "cem.format-tree",
  targetContentType: <schema:content-type-identity from @target-content-type>,
  targetSchemaIdentity: <schema:schema-identity from @target-schema>,
  targetCategory: "cem-tree",
  functionProfile: "compact",
  subjectType: "cem-ast-node"
}
```

Each key part retains its own declared spelling and source range. Diagnostics
can point to the path, function name, target content type, target schema,
category, profile, or subject source independently instead of pointing at a
synthetic combined key.

## Vocabulary

| Normalizer                         | Output                | Placement                                                            | Semantics                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ---------------------------------- | --------------------- | -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `schema:scalar-exact`              | typed scalar          | pure                                                                 | Preserve the parsed scalar type and semantic value exactly. Equality is `(type, value)` with no coercion: strings, integers, decimals, booleans, and null do not compare across types. Source spelling is not part of equality; use `schema:source-lexeme-exact` when spelling matters.                                                                                                                                                                                                                                                                                             |
| `schema:string-exact`              | string                | pure                                                                 | Accept only parsed string/text values and compare codepoints exactly. No Unicode normalization, case folding, URI normalization, whitespace trimming, or conversion from numeric, boolean, null, record, or list values is applied.                                                                                                                                                                                                                                                                                                                                                 |
| `schema:source-lexeme-exact`       | string                | pure                                                                 | Compare the captured `sourceLexeme` exactly as authored for the selected scalar or list item. This normalizer is for spelling-sensitive contracts and must not reconstruct a lexeme from `declaredValue`; values without captured source lexemes cannot satisfy it.                                                                                                                                                                                                                                                                                                                 |
| `schema:identifier-token`          | string                | pure                                                                 | Validate the schema scalar datatype `schema:identifier` / CEM-ML local-name grammar and preserve it exactly. This is an exact, case-sensitive local token, not a profile, function, artifact, path, namespace, schema URI, or host-language ID normalizer.                                                                                                                                                                                                                                                                                                                          |
| `schema:media-type`                | record                | engine-assisted initially; pure when content-type parsing is exposed | Parse strict RFC media-type syntax into `essence`, `type`, `subtype`, optional `suffix`, and `parameters`. Type, subtype, suffix, and parameter names normalize to lowercase. Parameter values are unquoted and otherwise preserved unless a registered parameter-specific rule declares case-insensitive comparison. Legacy aliases and bare content identifiers are not accepted by this normalizer. Invalid media syntax produces `state=invalid`.                                                                                                                               |
| `schema:media-type-essence`        | string                | engine-assisted initially; pure when content-type parsing is exposed | Apply `schema:media-type` and project its lowercase essence. `Text/HTML; Charset=UTF-8` normalizes to `text/html`. Parameter information remains available through `schema:media-type` when needed.                                                                                                                                                                                                                                                                                                                                                                                 |
| `schema:media-type-essence-set`    | set of scalar strings | engine-assisted initially; pure when content-type parsing is exposed | Set normalizer with `itemNormalizer=schema:media-type-essence`. It applies the strict RFC item normalizer to each declared media-type claim, exposes a sorted duplicate-free `comparisonSet`, and keeps source-ordered item outcomes with invalid items and duplicate origins addressable by source range.                                                                                                                                                                                                                                                                          |
| `schema:content-type-identity`     | record                | engine-assisted                                                      | Resolve an RFC media type or registered content alias through the active schema/content registry. The normalized value is `{ contentType, schemaIdentity, routingProfile? }`; provenance preserves declared spelling, alias owner, registry layer, and source range. Unknown content types produce `state=unresolved, reason=unknown-content-type`; ambiguous values without required schema context produce `state=unresolved, reason=ambiguous-content-type`; values outside an explicit schema context produce `state=invalid, reason=content-type-schema-mismatch`.             |
| `schema:content-type-identity-set` | set of records        | engine-assisted                                                      | Set normalizer with `itemNormalizer=schema:content-type-identity`. It resolves each registered content-type claim or alias against the active registry view, exposes a sorted duplicate-free `comparisonSet` of content identity records, and preserves source-ordered item outcomes, declared spellings, aliases, states, reasons, duplicates, and source ranges.                                                                                                                                                                                                                  |
| `schema:schema-uri-declaration`    | string                | pure                                                                 | Preserve the declared schema URI exactly after CEM-ML parsing and validate only schema-URI declaration syntax. If the last path segment is an AC-V-10 version tail, expose the parsed constraint as provenance. This normalizer does not resolve a descriptor, select a version, or apply generic URI text canonicalization.                                                                                                                                                                                                                                                        |
| `schema:schema-identity`           | record                | engine-assisted                                                      | Resolve a schema reference through the AC-F-2/AC-P-6-aware schema resolution context using AC-V-9 through AC-V-13. The normalized value is the complete identity record `{ uri, embeddedVersion }`, where `uri` is the matched descriptor's stable schema URI and `embeddedVersion` is the descriptor's complete SemVer 2.0 string. Provenance retains the source form, declared URI or alias when present, version constraint, resolver or metadata source, and match rule; unresolved or ambiguous resolution produces `state=unresolved`.                                        |
| `schema:schema-uri`                | URI projection string | engine-assisted                                                      | Apply `schema:schema-identity` and project only the resolved descriptor `uri`. This is a lossy compatibility normalizer for rules that explicitly want stable URI equality without version identity. It must not be used where complete schema identity, cache identity, or schema-version compatibility is intended.                                                                                                                                                                                                                                                               |
| `schema:document-uri`              | URI record            | engine-assisted                                                      | Finalize a resource URI/path/specifier against `QueryContextScope.effectiveBaseUri`, resolver purpose, package/module-map context, and policy. The comparison value contains `{ declaredUri, resolvedUri }`; base URI, resolver purpose, package/module-map context, policy, resolver steps, cache hints, and source range are provenance. This normalizer does not fetch, list, read, parse, compile, or assert existence/readability. Malformed syntax is `invalid-document-uri`; missing resolver or mapping context is `unresolved-document`; denied policy is `policy-denied`. |
| `schema:namespace-uri`             | string                | pure                                                                 | Preserve the namespace URI string exactly after CEM-ML parsing. Namespace equality remains textual. Dispatch and content-type/schema selection use `schema:namespace-metadata`, not this text normalizer.                                                                                                                                                                                                                                                                                                                                                                           |
| `schema:namespace-uri-set`         | set of scalar strings | pure                                                                 | Set normalizer with `itemNormalizer=schema:namespace-uri`. It exposes a sorted duplicate-free `comparisonSet` while preserving source-ordered namespace claim outcomes, duplicate origins, states, reasons, and source ranges.                                                                                                                                                                                                                                                                                                                                                      |
| `schema:namespace-metadata`        | record                | engine-assisted                                                      | Resolve namespace metadata per AC-P-6.1 through the local-first metadata chain. The normalized record contains `{ namespaceUri, contentType, schemaUri, schemaVersion }`, where `schemaUri` and `schemaVersion` are the resolved `schema:schema-identity` pair. Missing metadata without an explicit schema form is governed by AC-P-6.7 scope policy.                                                                                                                                                                                                                              |
| `schema:artifact-name`             | string                | pure                                                                 | Validate an authored manifest artifact ID using `schema:identifier-token` grammar and preserve it exactly. This normalizer is not used for package-relative paths, resolved URIs, basenames, function names, or path-derived artifact identity.                                                                                                                                                                                                                                                                                                                                     |
| `schema:artifact-identity`         | record                | engine-assisted                                                      | Resolve an artifact URI/path through `schema:document-uri` and package/registry context into `{ declaredUri, resolvedUri, packageContext?, artifactKind?, sourceContentType?, sourceSchemaIdentity? }`. Equality uses the resolved artifact URI in its package/registry context, plus artifact kind when the contract requires it. Declared spelling, source range, resolver steps, and aliases remain provenance.                                                                                                                                                                  |
| `schema:function-name`             | string                | pure                                                                 | Validate a dotted exported function symbol and preserve it exactly. This normalizer owns lexical function-name equality only; it does not identify the compiled module or artifact that declares the function.                                                                                                                                                                                                                                                                                                                                                                      |
| `schema:function-identity`         | record                | engine-assisted                                                      | Lift a compiled CEMT function declaration into `{ moduleIdentity, canonicalName, functionProfile?, functionKind? }`. The module or artifact identity is supplied by resolver context; `canonicalName` is compared with manifest `@function-name` using `schema:function-name`; optional profile/kind fields compare only when declared by the schema contract.                                                                                                                                                                                                                      |
| `schema:content-category`          | string                | pure                                                                 | Alias of `schema:identifier-token` for artifact/converter target categories. Kept as a named normalizer so diagnostics can report the domain-specific value kind.                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `schema:profile-name`              | string                | pure                                                                 | Validate a dotted `schema:symbol-reference` profile symbol and preserve it exactly. Profiles are case-sensitive schema-owned symbols; no case folding, segment normalization, alias expansion, or conversion to bare identifier tokens is applied.                                                                                                                                                                                                                                                                                                                                  |

## Registry-Derived Values

Registry-backed rules must normalize descriptor projections through the same
normalizers as declared manifest values:

- Schema descriptor `contentTypes` use `schema:content-type-identity-set` when
  a reference check needs registered content identity or alias support. Pure RFC
  media syntax checks use `schema:media-type-essence-set`.
- Schema descriptor declarations use `schema:schema-uri-declaration` for
  source/manifest consistency checks that run before registry admission.
- Schema descriptor identity uses `schema:schema-identity`; when the descriptor
  itself is the registry-derived value, the engine lifts the descriptor's
  resolved identity directly instead of resolving its URI again.
- Schema descriptor namespaces use `schema:namespace-uri-set`.
- Namespace-dispatch metadata uses `schema:namespace-metadata`; the resolved
  metadata source participates in cache and policy identity per AC-P-6.1.
- CEMT function output metadata uses `schema:function-identity`,
  `schema:function-name`,
  `schema:content-type-identity`, `schema:media-type`,
  `schema:media-type-essence`, `schema:schema-identity`,
  `schema:content-category`, and `schema:profile-name` as applicable.

This keeps a future comparison such as "endpoint content type is a member of
the endpoint schema's registered content identities" independent from the Rust
package rule that currently performs it.

## Invalid And Unresolved Reasons

The first stable reason identifiers are:

- `missing-value`: the selected field is absent.
- `invalid-scalar`: the selected field does not satisfy the normalizer's scalar
  grammar.
- `invalid-media-type`: media type parsing failed.
- `unknown-content-type`: no registered content identity matched the declared
  content type or alias.
- `ambiguous-content-type`: more than one registered content identity matched
  and no schema context disambiguated the value.
- `content-type-schema-mismatch`: the declared content type or alias resolved,
  but not under the required schema context.
- `invalid-schema-uri`: schema URI declaration parsing failed.
- `invalid-schema-version-constraint`: a schema URI tail looks like a version
  constraint but is not one of the AC-V-10 forms.
- `unresolved-schema`: schema registry lookup failed.
- `unresolved-namespace-metadata`: no namespace metadata resolved before
  AC-P-6.7 policy selected an outcome.
- `invalid-document-uri`: document URI/path/specifier parsing failed before
  resolver finalization.
- `unresolved-document`: no resolver, module-map entry, package context, or
  other required mapping could finalize the declared URI.
- `unresolved-function`: compiled artifact metadata did not expose the named
  function.
- `unsupported-normalizer`: the active engine cannot execute the requested
  normalizer.
- `unsupported-capability`: the active engine cannot execute the requested
  engine-assisted capability.
- `policy-denied`: the active scope policy denied the requested operation.

Comparison rules may project these reasons into `invalidValues`,
`expectedValues`, or a dedicated unresolved-reference detail, but that
projection is not part of this normalization vocabulary.

## Application To Current Rust-Backed Checks

- Converter endpoint content-type/schema compatibility normalizes endpoint
  `@content-type` with `schema:content-type-identity` using endpoint `@schema`
  as schema context; normalizes endpoint `@schema` with
  `schema:schema-identity` as the registry lookup key; and normalizes the
  referenced schema descriptor `contentTypes` with
  `schema:content-type-identity-set` as the expected comparison value.
- Example content-type/schema compatibility uses the same three normalizers.
- Package schema source metadata consistency normalizes manifest schema URI,
  strict RFC media-type claims, and manifest namespace claims with
  `schema:schema-uri-declaration`, `schema:media-type-essence-set`, and
  `schema:namespace-uri-set` before registry admission. Registered aliases use
  `schema:content-type-identity-set` against the validation overlay after
  provisional descriptor construction. Registry consumers use
  `schema:schema-identity` and `schema:content-type-identity` against the
  validation overlay.
- Namespace-driven content-type/schema dispatch normalizes the active namespace
  with `schema:namespace-metadata`. An explicit AC-F-2 schema form may refine
  the resolved schema within the namespace-selected content type, but a direct
  form that selects a different content type is invalid per AC-P-6.1.
- Artifact function lookup normalizes manifest `@function-name` with
  `schema:function-name` and compiled CEMT declarations with
  `schema:function-identity`. Lookup compares the manifest lexical function
  name with the identity record's `canonicalName` inside the resolved
  module/artifact identity.
- Artifact function metadata matching normalizes target content type, target
  schema, target category, function profile, and function name with the
  corresponding vocabulary terms above.

## Acceptance Examples

Schema URI version-tail constraints are provenance, not complete identity. If
the registry contains this descriptor:

```text
{
  uri: "https://example.test/ns/widget",
  embeddedVersion: "1.4.2+20260720"
}
```

then declarations such as `https://example.test/ns/widget`,
`https://example.test/ns/widget/1`, `https://example.test/ns/widget/1.4`, and
`https://example.test/ns/widget/1.4.2` can resolve to the same
`schema:schema-identity` record. The declared URI and parsed version constraint
remain provenance; cache and complete identity use the resolved
`embeddedVersion`.

Local custom package validation starts outside the host catalog. A package
manifest that declares `@uri="https://example.test/ns/note/1"` and
`@source="schema/note.cem"` first validates the manifest and schema-source
declarations with pure URI, media-type, and namespace normalizers. Only after
those checks pass does validation construct an isolated provisional descriptor
for registry-backed endpoint, example, artifact, and namespace checks. The
descriptor is admitted to the host catalog only after every required check
passes.

Media syntax and registered content identity produce different outcomes:

- `Text/HTML; Charset=UTF-8` is valid `schema:media-type`; its essence
  projection is `text/html`;
- `text/xsl` is a valid RFC media type and may resolve through
  `schema:content-type-identity` to the XSLT package's registered canonical
  identity when that package claims the alias;
- `custom-element-xslt` is invalid under `schema:media-type`, but can be a
  valid registered legacy alias under `schema:content-type-identity` when the
  compatibility package owns it;
- a registered alias claimed by multiple packages without required schema
  context is `unresolved, reason=ambiguous-content-type`;
- a malformed media string is `invalid, reason=invalid-media-type`.

Scalar-to-set comparison uses item-normalizer compatibility. An actual scalar
`schema:media-type-essence` value such as `text/html` may be checked against an
expected `schema:media-type-essence-set` because both sides expose media-type
essence equivalence. The expected set's `comparisonSet` is sorted and
duplicate-free, while `items` preserve source order, duplicate groups, invalid
member reasons, declared values, normalized values, and source ranges for
diagnostics.

Lifecycle outcomes remain distinct:

- `missing`: source extraction found no value, such as an absent optional
  package field;
- `invalid`: a value exists but violates syntax, shape, cardinality, or
  normalizer rules;
- `unresolved`: a lookup key normalized but no target, resolver mapping, or
  unambiguous registry owner was found;
- `unsupported`: the active engine cannot execute the declared normalizer,
  lookup, or capability;
- `pending`: an internal non-terminal lookup state; comparison is deferred
  until the value finalizes to a terminal state.

Ordered ARIA IDREF sequences and JSON Schema dynamic reference traversal are
explicitly deferred from the first package-check release. ARIA/HTML/slot
references need future sequence and runtime revision semantics; JSON Schema
`$ref` and `$dynamicRef` traversal remains owned by the JSON Schema loader.

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
- `schema:scalar-exact` preserves parsed scalar type. Use
  `schema:string-exact` for text-only exact contracts and
  `schema:source-lexeme-exact` for spelling-sensitive contracts.
- A comparison must identify the collection/scalar `normalizer` and effective
  `itemNormalizer` for each side. Operators compare item-normalized values
  with compatible equivalence semantics; identical normalizer names are not
  required for scalar `N` against an operator-compatible `set-of(N)`, and
  incompatible mixed normalizers are malformed.
- URI, Unicode, and case normalization are never implicit. They happen only
  through a named normalizer with documented equivalence semantics.
- Source ranges attach to the declared field or list item that fed the
  normalizer. Registry-derived values attach to the registry descriptor range
  when available; otherwise they carry descriptor identity only.
- Normalized set comparison views are sorted and duplicate-free for
  deterministic comparisons; source-ordered item outcomes retain invalid
  entries, duplicate origins, reasons, and source ranges for diagnostics.
