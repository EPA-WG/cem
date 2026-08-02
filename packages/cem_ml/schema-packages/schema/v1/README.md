# CEM Schema Definition Language Package

Status: current implemented surface for the schema definition language package.
Reference-normalization target design lives in
[`../../../../../docs/cem-ml-reference-normalization-design.md`](../../../../../docs/cem-ml-reference-normalization-design.md),
with lookup and comparison vocabulary in
[`../../../../../docs/cem-ml-reference-vocabulary-design.md`](../../../../../docs/cem-ml-reference-vocabulary-design.md)
and
[`../../../../../docs/cem-ml-reference-comparison-design.md`](../../../../../docs/cem-ml-reference-comparison-design.md).

This package defines the CEM-ML schema declaration language used to describe
validation schemas for input content.

Owned schema URI:

```text
https://cem.dev/ns/schema/1
```

Schema source:

```text
schema/cem-schema.cem
```

This filename is the documented v1 bootstrap exception to the default
`schema/schema.cem` shape. It preserves the schema-definition identity embedded
by the runtime catalog while the package still follows the rest of the
versioned folder contract.

Primary content type:

```text
application/vnd.cem.schema+cem
```

Schema source files are ordinary CEM-ML documents using this namespace for the
schema-authoring vocabulary. The target schema being described is carried by the
`schema @namespace` attribute in schema v1.

`schema @namespace` is a schema-v1 compatibility adapter input, not the
long-term schema identity field. Loaders may use it to project explicit
schema-source identity metadata for downstream package checks, while preserving
the authored namespace value as provenance. Manifest schema URI consistency
must compare against an explicit schema URI or resolved schema identity
projection, and namespace consistency must compare only namespace claims. A
future schema-source version should expose an explicit schema identity field
instead of relying on namespace-as-identity.

When schema v1 source is projected into a target descriptor, descriptor
provenance records the schema source artifact, source content type, package or
built-in origin, authored namespace compatibility value, declared content-type
claims, namespace claims, and source ranges when available. Current validators
may still emit compatibility diagnostics keyed to the shipped schema/package
fields, but target reference checks consume explicit schema URI declarations,
complete schema identity records, and namespace claims as separate domains.

## Folder Contract

`package.cem` is the manifest-owned index for this folder. It declares the
schema URI and source file, primary content type, namespace claim, and every
validation example under `examples/`.

`project.json` owns the package-local Nx library
`cem_ml_schema_package_schema_v1`. Its `verify` target validates `package.cem`
through the CLI at the parse failure boundary and tracks `README.md`,
`schema/**/*.cem`, `formatters/**/*.cemt`, `colorizers/**/*.cemt`,
`converters/**/*.cemt`, and `examples/**/*` as package inputs. Full semantic
schema-package validation remains in the final registry/package gate.

Example metadata is intentionally manifest-owned. This package does not require
checked-in `.example.cem` sidecars because `package.cem` already records the
example path, content type, schema URI, expected pass/fail result, and expected
diagnostic codes.

## CEMT Output Status

The schema definition package currently declares no converter edges and no
package-owned formatter or colorizer CEMT artifacts. The schema-package
structure audit therefore reports the baseline formatter/colorizer profiles as
alignment gaps, not hard errors. Until schema-specific CEMT formatters and
colorizers are authored, schema source examples rely on the generic CEM-ML
output path rather than a schema-definition-specific output pipeline.

## Field Contract Requirement

The schema definition language owns field contracts for every schema-declared
construct. A schema must be able to declare required, optional, and forbidden
fields or attributes; accepted children; value types and vocabularies; defaults;
dependent fields; mutually exclusive groups; conditional rules; open-content
policy; and the diagnostic contract for failed checks.

This follows the same separation of concerns used by established schema systems:
RELAX NG patterns own structure, XSD owns complex types and attribute use,
JSON Schema owns `properties`, `required`, `dependentRequired`, and
`if`/`then`, and SHACL owns shape constraints. In CEM, the `.cem` schema source
is the authority. Rust validators compile and evaluate the schema declarations
and perform operational checks, but they must not be the source of
package-specific required-field lists or conditional field rules.

Field-check diagnostics should identify the contract family and carry structured
details such as target, check kind, expected fields, missing fields, invalid
fields, forbidden fields, and actual values. They should not require one
diagnostic code per individual metadata or schema field.

Field contracts can be gated by value selectors such as `when-attribute` plus
`when-values`, by attribute selectors such as `when-present-attributes` and
`when-absent-attributes`, by child selectors such as `when-present-children`
and `when-absent-children`, or by combining selector families on one dependency
contract. Use attribute selectors for dependent-required rules such as "when
this attribute is present and that attribute is absent, require these other
attributes".
Use child selectors with `schema:field-dependency` when direct-child structure
implies required attributes, forbidden attributes, or forbidden attribute
values, such as requiring a label when a reference child is present and no
fallback child is present.
Use `forbidden-attributes` with the same selectors when a present or valued
gate makes another attribute invalid.
Use `forbidden-attribute-values` for value-specific exclusions, such as a
schema-owned mutual exclusion where one attribute value makes another attribute
value invalid while leaving other values legal.
Use `required-one-attributes` and `max-one-attributes` for choice cardinality
over alternative attributes. Declaring both on one field contract creates an
exactly-one attribute choice while still preserving the broad diagnostic family
code.
Use nested `choice` groups with `case` entries for broader grouped choices:
`choice @mode` accepts `exactly-one`, `at-least-one`, or `at-most-one`, and
each `case` declares the attributes and/or children that make that case
present. This is the preferred syntax for new `schema:choice-case` contracts;
the flat `required-one-attributes` and `max-one-attributes` attributes remain
compatibility shorthand for simple attribute choices.
Use `required-children` plus `max-one-children` for exact-one child occurrence
contracts, such as schema package converter `from`/`to` endpoints.
Use `required-one-child` and `max-one-child` for cardinality over a set of
alternative child names. Declaring both on one field contract creates an
exactly-one child choice while still preserving the broad diagnostic family
code.
Use `min-children` and `max-children` for broader child occurrence ranges
expressed as `child=count` name-value pairs.
Use `min-total-children`, `max-total-children`, and
`exact-total-children` when the contract is over the total number of element
children in the current scope instead of over one child name.
Use `min-distinct-children`, `max-distinct-children`, and
`exact-distinct-children` when the contract is over the number of different
child element names present in the current scope.
Use `selected-children` with `min-selected-children`,
`max-selected-children`, or `exact-selected-children` when the contract is
over total occurrences of only the declared child-name set.
Use `selected-children` with `min-selected-distinct-children`,
`max-selected-distinct-children`, or `exact-selected-distinct-children` when
the contract is over the number of different declared child names present,
ignoring unselected child names.
Schema-definition validation rejects selected child occurrence bounds that do
not declare `selected-children`, and rejects exact child occurrence bounds
outside their declared min/max envelope. It also rejects impossible direct
child boundary/sequence contradictions, such as a required boundary or sequence
matching its forbidden counterpart, or an `exact-child-sequence` that cannot
satisfy the same contract's boundary, prefix, suffix, required-sequence, or
forbidden-sequence declarations. Required fields or children cannot also be
forbidden by the same field contract, and required-one choices must leave at
least one non-forbidden, accepted alternative. Conditional selectors must be
satisfiable: `when-values` requires `when-attribute`, all-present selectors
cannot overlap all-absent selectors, and any-present/any-absent selectors must
leave at least one possible candidate.
Use `ordered-children` when the declared child-name set must appear in that
relative order among direct element children. Unlisted children are ignored;
requiredness and multiplicity remain separate child occurrence contracts.
Use `forbidden-ordered-children` when the declared child-name set must not
appear in that relative order; unlike `forbidden-child-sequence`, intervening
children do not make the order valid.
Use `first-child` and `last-child` when a direct element child must occupy the
first or last element-child position in the current scope.
Use `forbidden-first-child` and `forbidden-last-child` when a direct element
child is allowed in the scope but must not occupy a boundary position.
Use `required-child-sequence` when a contiguous direct-child run must appear
somewhere in the current scope. Extra children may appear before or after the
run, but intervening children break the sequence.
Use `forbidden-child-sequence` when a contiguous direct-child run must not
appear in the current scope.
Use `exact-child-sequence` when the complete direct element-child stream must
match the declared sequence, with no missing, extra, or reordered element
children.
Use `prefix-child-sequence` and `suffix-child-sequence` when the direct
element-child stream must start or end with a declared run while still allowing
other children outside that edge.
Use `forbidden-prefix-child-sequence` and
`forbidden-suffix-child-sequence` when the direct element-child stream may
contain a run elsewhere, but must not start or end with that run.
Use `path-layout-attributes` with `path-layout-prefix`,
`path-layout-directory-names`, `path-layout-forbidden-directory-names`, and
`path-layout-extension`, plus `path-layout-basenames` and
`path-layout-forbidden-basenames`, for package-relative path layout contracts,
such as formatter artifacts under `formatters/` and colorizer artifacts under
`colorizers/`.

The generic path-layout field-contract vocabulary is closed at prefix,
directory-name allow/forbid, extension, and basename allow/forbid facets for
the current design. A `schema:path` value resolves in the active scope context
before these facets run: `./...` is relative to the active context root,
protocol-prefixed values use their protocol resolver, and bare values use the
active module map or aliases. Defer generic path depth, segment count, suffix,
glob or segment-class, and alias or module-map matching facets until a concrete
schema-owned check can define stable resolver provenance, source-range
projection, and cross-protocol comparison behavior.

## Declarative Behavior Registry

The schema definition language declares reusable validation behavior under
`{behaviors}`. A `{diagnostic}` binds its stable `@code` to a qualified behavior
reference, and a field contract refers to that code. The code is the stable
behavior-contract identity emitted in CLI and report output; the behavior
reference selects the algorithm contract used to produce that diagnostic.

```cem
{behaviors |
    {behavior
        @name="field-contract"
        @implementation="engine"
        @execution="ast-validation"
        @primitive="schema:field-contract" |
        {inputs |
            {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            {input-binding @name="diagnostic" @type="schema:diagnostic" @source="diagnostic" @required=true}
        }
        {parameters |
            {parameter @name="contract" @type="schema:field-contract" @required=true}
        }
        {result
            @type="schema:diagnostic-result"
            @severity="error"
            @message="diagnostic"
            @source-range="candidate" |
            {detail @name="checkKind" @type="schema:identifier" @required=true}
            {detail @name="sourceRange" @type="schema:object" @source="candidate"}
        }
    }
}

{diagnostics |
    {diagnostic
        @code="example.resource.missing_label"
        @severity="warning"
        @behavior="schema:required-fields"
        @message="Page resources should declare a label"
    }
}
```

Behavior references resolve through schema `{uses}` aliases. Engine-provided
behaviors bind to a primitive algorithm with `@primitive`. The initial
field-contract primitive is `schema:field-contract`, and the bootstrap schema
declares named behavior contracts backed by that primitive:
`schema:required-fields`, `schema:forbidden-fields`,
`schema:dependent-required-fields`, `schema:mutual-exclusion`,
`schema:field-dependency`, `schema:choice-case`,
`schema:child-occurrence`, and `schema:path-layout`.
The required, forbidden, dependency, mutual-exclusion, and choice-case
contracts declare the structured detail fields that the engine emits for those
diagnostic results.

Individual `{field-contract}` declarations can also bind `@diagnostic` plus
`@behavior`. The diagnostic code remains the report identity, while the
contract-local behavior selects the operational algorithm for that contract.
This lets one broad diagnostic family such as
`cem.schema_package.converter_check` report a conditional dependency through
`schema:field-dependency` and a choice/case exclusion through
`schema:choice-case`.

Attribute-owned engine primitives are also schema-visible. Attribute
declarations bind `@values` failures through `@values-diagnostic` to
`schema:value-vocabulary`, scalar syntax failures for identifier, name-list,
type-reference, symbol-reference, wildcard-name/list/type-reference, boolean,
integer, number, qualified-name, semantic version, URI, media-type, and path
attributes through `@type-diagnostic` to `schema:scalar-type`,
and datatype parameter failures for integer and number
`@minInclusive`/`@maxInclusive`/`@minExclusive`/`@maxExclusive` bounds, string
`@minLength`/`@maxLength`/`@length`/`@stringPrefixes`/`@stringSuffixes`/
`@stringForbiddenPrefixes`/`@stringForbiddenSuffixes`/
`@stringIncludes`/`@stringExcludes`
constraints, list `@itemCount`/`@minItems`/`@maxItems` item-count
constraints, numeric `@totalDigits`/`@fractionDigits` digit-count
constraints, and regex `@pattern`, path `@pathPrefixes`/`@pathForbiddenPrefixes`/
`@pathDirectoryNames`/`@pathForbiddenDirectoryNames`/
`@pathExtensions`/`@pathForbiddenExtensions`/`@pathBasenames`/
`@pathForbiddenBasenames`, URI
`@uriSchemes`/`@uriForbiddenSchemes`/`@uriHosts`/`@uriForbiddenHosts`/
`@uriPorts`/`@uriForbiddenPorts`/`@uriRequiresAuthority`/`@uriPathPrefixes`/
`@uriForbiddenPathPrefixes`/`@uriPathExtensions`/`@uriForbiddenPathExtensions`/
`@uriPathBasenames`/`@uriForbiddenPathBasenames`/`@uriQueries`/`@uriForbiddenQueries`/
`@uriQueryParameters`/`@uriQueryParameterValues`/`@uriQueryForbiddenParameters`/
`@uriQueryRequiredParameters`/`@uriFragments`/`@uriForbiddenFragments`, and media-type
`@mediaTypes`/`@mediaTypeForbiddenEssences`/`@mediaTypeTypes`/`@mediaTypeSubtypes`/`@mediaTypeSuffixes`/
`@mediaTypeForbiddenTypes`/`@mediaTypeForbiddenSubtypes`/
`@mediaTypeForbiddenSuffixes`/`@mediaTypeParameters`/
`@mediaTypeParameterValues`/`@mediaTypeForbiddenParameters`/
`@mediaTypeRequiredParameters` constraints
through `@datatype-param-diagnostic` to
`schema:datatype-param`. In all cases the diagnostic `@code` remains the
stable output identity while `@behavior` selects the reusable algorithm
contract. The `schema:value-vocabulary` and `schema:scalar-type` result
contracts declare the emitted attribute value/type detail keys, and the
`schema:datatype-param` result contract declares the emitted datatype-specific
detail keys.

Operational constraints bind their execution behavior at the `{constraint}`
declaration while keeping the diagnostic family code stable. Constraint
declarations can use `@diagnostic` plus `@behavior` to bind resource readability
checks to `schema:resource-readable`, parser/validation checks to
`schema:resource-parse`, and cross-reference checks to
`schema:reference-resolution`. This lets a schema-package diagnostic such as
`cem.schema_package.artifact_check` remain the report identity while individual
constraint `checkKind` values select different engine algorithms.
Their result contracts declare the emitted resource path, read error,
parse/source diagnostic, expected-value, invalid-value, and source-range
details.

Function behaviors bind to a schema-declared function with `@function`, declare
typed `{inputs}`, optional typed `{parameters}`, and a `{result}` shape with
structured `{detail}` entries and source-range propagation policy:

```cem
{behavior
    @name="resource-label"
    @implementation="function"
    @execution="ast-validation"
    @function="resource-label-result"
    @select="resource"
    @match='kind == "page" && label == null' |
    {inputs |
        {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true}
    }
    {parameters |
        {parameter @name="expected" @type="schema:string" @required=true @default="label"}
    }
    {result @type="schema:diagnostic-result" @source-range="candidate" |
        {detail @name="checkKind" @type="schema:identifier" @required=true}
        {detail @name="expected" @type="schema:string" @required=true}
    }
    {function @name="resource-label-result" @returns="object" @deterministic=true |
        {param @name="candidate" @type="object" @required=true}
        {param @name="expected" @type="string" @required=true}
        {body | {$ { message: "Page resource needs a label", details: { checkKind: "resource-label", expected: expected, element: candidate.name } } }}
    }
}

{diagnostics |
    {diagnostic @code="example.resource.missing_label" @severity="warning" @behavior="resource-label" |
        {arguments |
            {argument @name="expected" @value="label"}
        }
    }
}
```

The compiler now validates diagnostic behavior references, unsupported engine
primitives, missing function bindings, inline function lookup, function return
type, diagnostic result shape, required CEM-QL `@select`/`@match` expressions,
executable CEM-ML behavior body presence, and required function-parameter
binding through declared inputs or defaulted behavior parameters. The CEM-QL
schema-behavior bridge now evaluates direct candidate selection and match
expressions, binds defaulted typed behavior parameters, and executes inline
CEM-ML behavior function bodies outside CEMT to produce diagnostic messages and
structured details. Qualified function references now resolve through schema
`{uses}` aliases to visible reusable CEM-ML behavior functions. Diagnostic
`{arguments}` now bind non-default parameter overrides for function behaviors.
The first field-contract-backed and attribute-owned engine behavior aliases now
compile and execute through diagnostic `@behavior`; field-contract-local
`schema:field-dependency` and `schema:choice-case` bindings now compile and
stamp operational diagnostics while preserving broad diagnostic family codes.
The `schema:field-dependency` alias covers required, forbidden, and
forbidden-value field dependencies gated by values, attribute presence, or
child structure, and its result contract declares the emitted
required/missing/forbidden/invalid field, value, and condition details.
Constraint-owned `schema:resource-readable`, `schema:resource-parse`, and
`schema:reference-resolution` bindings now compile and stamp operational
diagnostics with their declared behavior; their result contracts declare the
emitted path/error/source-diagnostic and reference expected/invalid value
details. Integer and number `minInclusive`,
`maxInclusive`, `minExclusive`, and `maxExclusive` bounds, numeric
`totalDigits`/`fractionDigits` constraints, string `minLength`/`maxLength`/
`length`/`stringPrefixes`/`stringSuffixes`/`stringForbiddenPrefixes`/
`stringForbiddenSuffixes`/`stringIncludes`/`stringExcludes` constraints, list
`itemCount`/`minItems`/`maxItems` constraints, regex `pattern`, path
prefix/forbidden-prefix/directory-name/forbidden-directory-name/extension/forbidden-extension/basename/forbidden-basename `pathPrefixes`/`pathForbiddenPrefixes`/`pathDirectoryNames`/`pathForbiddenDirectoryNames`/`pathExtensions`/`pathForbiddenExtensions`/`pathBasenames`/`pathForbiddenBasenames`, URI scheme/forbidden-scheme/host/forbidden-host/port/forbidden-port/authority/path-prefix/forbidden-path-prefix/path-extension/forbidden-path-extension/path-basename/forbidden-path-basename/query/forbidden-query/query-parameter-name/value/forbidden-parameter/required-parameter/fragment/forbidden-fragment
`uriSchemes`/`uriForbiddenSchemes`/`uriHosts`/`uriForbiddenHosts`/`uriPorts`/
`uriForbiddenPorts`/`uriRequiresAuthority`/`uriPathPrefixes`/
`uriForbiddenPathPrefixes`/`uriPathExtensions`/`uriForbiddenPathExtensions`/
`uriPathBasenames`/`uriForbiddenPathBasenames`/`uriQueries`/`uriForbiddenQueries`/
`uriQueryParameters`/`uriQueryParameterValues`/`uriQueryForbiddenParameters`/
`uriQueryRequiredParameters`/`uriFragments`/`uriForbiddenFragments`, and media-type
essence/forbidden-essence/type/subtype/structured-suffix/forbidden-type/forbidden-subtype/
forbidden-structured-suffix/parameter-name/value/forbidden-parameter/
required-parameter `mediaTypes`/`mediaTypeForbiddenEssences`/`mediaTypeTypes`/`mediaTypeSubtypes`/`mediaTypeSuffixes`/
`mediaTypeForbiddenTypes`/`mediaTypeForbiddenSubtypes`/`mediaTypeForbiddenSuffixes`/`mediaTypeParameters`/
`mediaTypeParameterValues`/`mediaTypeForbiddenParameters`/
`mediaTypeRequiredParameters` datatype parameter variations now execute through
`schema:datatype-param`, whose result contract declares the emitted
family-specific string/list/digit/path/URI/media-type details; identifier, name-list, type-reference,
symbol-reference, wildcard-name/list/type-reference, boolean, integer, number,
qualified-name, semantic version, basic absolute-URI, basic media-type, and
scope-context path scalar syntax now execute through `schema:scalar-type`,
whose result contract declares emitted scalar value/type details;
schema-definition validation also rejects numeric bound/digit-count params on
non-numeric attributes, string length/prefix/suffix/forbidden-prefix/forbidden-suffix/include/exclude params on non-string attributes,
list item-count params on non-list attributes, inconsistent numeric min/max
bound envelopes, inconsistent string/list min/max/exact count envelopes, and
inconsistent URI/media-type required/forbidden parameter declarations;
required-one/max-one attribute choice cardinality and nested choice/case groups
now execute through `schema:choice-case`, whose result contract declares the
flat and grouped choice details emitted by the engine; child-set cardinality,
`min-children`/`max-children` named child occurrence ranges, and
total/distinct/selected occurrence and selected-distinct child-count bounds,
child presence/absence condition selectors, required/forbidden relative child
ordering, required/forbidden boundary child placement, and
exact/required/forbidden/prefix/suffix/forbidden-prefix/forbidden-suffix child sequences, now
execute through `schema:child-occurrence`; the
currently declared attribute datatype-parameter vocabulary is covered by
`schema:datatype-param`. Attribute declarations can now carry literal
`@default` metadata, and default values are validated against declared scalar
types, value vocabularies, and datatype parameters; applying omitted defaults
to candidate input remains a runtime contract step.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-schema</summary>

- Source: [`examples/basic-schema.cem`](./examples/basic-schema.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/basic-schema.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="note" @namespace="https://example.test/ns/note/1" @version="1.0.0" |
    {summary |
        {text | Minimal note schema used as a schema-definition validation example.}
    }

    {content-types |
        {content-type @value="application/vnd.example.note+cem" @primary=true}
    }

    {elements |
        {element @name="note" @optional-attributes="id" @children="text"}
        {element @name="text"}
    }

    {attributes |
        {attribute @name="id" @type="schema:identifier"}
    }
}
```

<details>
<summary>typed-resource-schema</summary>

- Source: [`examples/typed-resource-schema.cem`](./examples/typed-resource-schema.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/typed-resource-schema.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="typed-resource" @namespace="https://example.test/ns/resource/1" @version="1.0.0" |
    {summary |
        {text | Resource schema with imports, namespace claims, attributes, diagnostics, and open-content policy.}
    }

    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }

    {content-types |
        {content-type @value="application/vnd.example.resource+cem" @primary=true}
    }

    {namespaces |
        {namespace @prefix="resource" @uri="https://example.test/ns/resource/1" @role="schema"}
    }

    {elements |
        {element @name="resource" @required-attributes="id kind" @optional-attributes="label priority rank weight serial ratio slug tags aliases code href format payload metadata asset inline qualified version" @children="field*"}
        {element @name="single-resource" @required-attributes="id kind" @children="field"}
        {element @name="linked-resource" @required-attributes="id kind" @optional-attributes="label" @children="field reference"}
        {element @name="conditional-resource" @required-attributes="id kind" @optional-attributes="label inline format" @children="field reference fallback"}
        {element @name="field" @required-attributes="name type"}
        {element @name="reference" @required-attributes="href"}
        {element @name="fallback"}
    }

    {attributes |
        {attribute @name="id" @type="cemml:identifier"}
        {attribute
            @name="priority"
            @type="schema:integer"
            @minInclusive=1
            @maxInclusive=10
            @datatype-param-diagnostic="example.resource.invalid_priority"
        }
        {attribute
            @name="rank"
            @type="schema:integer"
            @minExclusive=0
            @maxExclusive=100
            @datatype-param-diagnostic="example.resource.invalid_rank"
        }
        {attribute
            @name="weight"
            @type="schema:number"
            @minInclusive=0.0
            @maxInclusive=1.0
            @type-diagnostic="example.resource.invalid_weight"
            @datatype-param-diagnostic="example.resource.invalid_weight_range"
        }
        {attribute
            @name="serial"
            @type="schema:integer"
            @totalDigits=6
            @datatype-param-diagnostic="example.resource.invalid_serial_digits"
        }
        {attribute
            @name="ratio"
            @type="schema:number"
            @totalDigits=4
            @fractionDigits=2
            @datatype-param-diagnostic="example.resource.invalid_ratio_digits"
        }
        {attribute
            @name="slug"
            @type="schema:string"
            @stringPrefixes="page- component-"
            @stringSuffixes="-slug -id"
            @stringForbiddenPrefixes="draft- private-"
            @stringForbiddenSuffixes="-tmp -bak"
            @pattern="[a-z][a-z0-9-]*"
            @datatype-param-diagnostic="example.resource.invalid_slug"
        }
        {attribute
            @name="tags"
            @type="schema:name-list"
            @minItems=1
            @maxItems=4
            @datatype-param-diagnostic="example.resource.invalid_tags"
        }
        {attribute
            @name="aliases"
            @type="schema:wildcard-name-list"
            @itemCount=2
            @datatype-param-diagnostic="example.resource.invalid_aliases"
        }
        {attribute
            @name="code"
            @type="schema:string"
            @length=4
            @datatype-param-diagnostic="example.resource.invalid_code"
        }
        {attribute
            @name="kind"
            @type="schema:identifier"
            @values="page component token"
            @values-diagnostic="example.resource.invalid_kind"
        }
        {attribute
            @name="href"
            @type="schema:uri"
            @type-diagnostic="example.resource.invalid_href"
            @uriSchemes="https"
            @uriForbiddenSchemes="ftp file"
            @uriHosts="api.example.test assets.example.test"
            @uriForbiddenHosts="legacy.example.test"
            @uriPorts="443 8443"
            @uriForbiddenPorts="80 8080"
            @uriRequiresAuthority=true
            @uriPathPrefixes="/resources/ /assets/"
            @uriForbiddenPathPrefixes="/assets/private/"
            @uriPathExtensions="cem json"
            @uriForbiddenPathExtensions="bak tmp"
            @uriPathBasenames="resource.cem asset.json"
            @uriForbiddenPathBasenames="private.json secret.cem"
            @uriQueries="view=resource view=asset"
            @uriForbiddenQueries="debug=true trace=true"
            @uriQueryParameters="view"
            @uriQueryParameterValues="view=resource view=asset"
            @uriQueryForbiddenParameters="debug"
            @uriQueryRequiredParameters="view"
            @uriFragments="resource asset"
            @uriForbiddenFragments="debug trace"
            @datatype-param-diagnostic="example.resource.invalid_href_scheme"
        }
        {attribute
            @name="format"
            @type="schema:media-type"
            @type-diagnostic="example.resource.invalid_format"
            @mediaTypes="application/json text/html"
            @mediaTypeForbiddenEssences="application/xml image/png"
            @mediaTypeTypes="application text"
            @mediaTypeSubtypes="json html"
            @mediaTypeForbiddenTypes="image"
            @mediaTypeForbiddenSubtypes="xml"
            @mediaTypeParameters="charset profile"
            @mediaTypeParameterValues="charset=utf-8 profile=default"
            @mediaTypeRequiredParameters="charset"
            @datatype-param-diagnostic="example.resource.invalid_format_type"
        }
        {attribute
            @name="payload"
            @type="schema:media-type"
            @mediaTypeSuffixes="json xml"
            @mediaTypeForbiddenSuffixes="zip"
            @datatype-param-diagnostic="example.resource.invalid_payload_suffix"
        }
        {attribute
            @name="metadata"
            @type="schema:media-type"
            @mediaTypeForbiddenParameters="profile"
            @datatype-param-diagnostic="example.resource.invalid_metadata_parameter"
        }
        {attribute
            @name="asset"
            @type="schema:path"
            @type-diagnostic="example.resource.invalid_asset"
            @pathPrefixes="./assets/ @assets/"
            @pathForbiddenPrefixes="./assets/private/ @assets/private/"
            @pathDirectoryNames="@assets assets"
            @pathForbiddenDirectoryNames="private tmp"
            @pathExtensions="cem cemt"
            @pathForbiddenExtensions="bak tmp"
            @pathBasenames="resource.cem theme.cemt"
            @pathForbiddenBasenames="secret.cem private.cemt"
            @datatype-param-diagnostic="example.resource.invalid_asset_path"
        }
        {attribute
            @name="inline"
            @type="schema:string"
            @minLength=3
            @maxLength=80
            @stringIncludes="ref: label:"
            @stringExcludes="TODO FIXME"
            @datatype-param-diagnostic="example.resource.invalid_inline"
        }
        {attribute
            @name="qualified"
            @type="schema:qualified-name"
            @type-diagnostic="example.resource.invalid_qualified_name"
        }
        {attribute
            @name="version"
            @type="schema:semver"
            @type-diagnostic="example.resource.invalid_version"
        }
        {attribute @name="label" @type="schema:string" @default="untitled"}
        {attribute @name="name" @type="cemml:identifier"}
        {attribute @name="type" @type="schema:type-reference"}
    }

    {field-contracts |
        {field-contract
            @name="page-resource-label"
            @target="resource"
            @when-attribute="kind"
            @when-values="page"
            @required-attributes="label"
            @diagnostic="example.resource.missing_label"
            @check-kind="required-fields"
        }
        {field-contract
            @name="resource-link-choice"
            @target="resource"
            @diagnostic="example.resource.link_choice"
            @behavior="schema:choice-case"
            @check-kind="choice-case" |
            {choice @name="resource-link-source" @mode="exactly-one" |
                {case @name="href-link" @attributes="href"}
                {case @name="inline-link" @attributes="inline"}
            }
        }
        {field-contract
            @name="resource-field-range"
            @target="resource"
            @min-children="field=1"
            @max-children="field=8"
            @min-total-children=1
            @max-total-children=8
            @exact-distinct-children=1
            @selected-children="field"
            @min-selected-children=1
            @max-selected-children=8
            @exact-selected-distinct-children=1
            @first-child="field"
            @last-child="field"
            @required-child-sequence="field"
            @prefix-child-sequence="field"
            @suffix-child-sequence="field"
            @diagnostic="example.resource.field_range"
            @behavior="schema:child-occurrence"
            @check-kind="child-occurrence-range"
        }
        {field-contract
            @name="single-resource-exact-sequence"
            @target="single-resource"
            @exact-child-sequence="field"
            @diagnostic="example.resource.field_range"
            @behavior="schema:child-occurrence"
            @check-kind="exact-child-sequence"
        }
        {field-contract
            @name="linked-resource-child-choice"
            @target="linked-resource"
            @required-one-child="field reference"
            @max-one-child="field reference"
            @ordered-children="field reference"
            @forbidden-ordered-children="reference field"
            @forbidden-child-sequence="reference field"
            @forbidden-prefix-child-sequence="reference field"
            @forbidden-suffix-child-sequence="field reference"
            @diagnostic="example.resource.child_choice"
            @behavior="schema:child-occurrence"
            @check-kind="exactly-one-child"
        }
        {field-contract
            @name="conditional-resource-reference-field"
            @target="conditional-resource"
            @when-present-children="reference"
            @when-absent-children="fallback"
            @required-children="field"
            @diagnostic="example.resource.child_choice"
            @behavior="schema:child-occurrence"
            @check-kind="conditional-required-children"
        }
        {field-contract
            @name="conditional-resource-reference-label"
            @target="conditional-resource"
            @when-present-children="reference"
            @when-absent-children="fallback"
            @required-attributes="label"
            @diagnostic="example.resource.reference_dependency"
            @behavior="schema:field-dependency"
            @check-kind="child-gated-dependent-required-fields"
        }
        {field-contract
            @name="conditional-resource-reference-format-forbidden"
            @target="conditional-resource"
            @when-present-children="reference"
            @when-absent-children="fallback"
            @forbidden-attributes="format"
            @diagnostic="example.resource.reference_dependency"
            @behavior="schema:field-dependency"
            @check-kind="child-gated-dependent-forbidden-fields"
        }
        {field-contract
            @name="conditional-resource-reference-inline-legacy"
            @target="conditional-resource"
            @when-present-children="reference"
            @when-absent-children="fallback"
            @forbidden-attribute-values="inline=legacy"
            @diagnostic="example.resource.reference_dependency"
            @behavior="schema:field-dependency"
            @check-kind="child-gated-dependent-forbidden-values"
        }
        {field-contract
            @name="conditional-resource-forbidden-boundaries"
            @target="conditional-resource"
            @forbidden-first-child="fallback"
            @forbidden-last-child="reference"
            @diagnostic="example.resource.child_choice"
            @behavior="schema:child-occurrence"
            @check-kind="forbidden-boundary-children"
        }
    }

    {diagnostics |
        {diagnostic @code="example.resource.missing_field" @severity="error"}
        {diagnostic
            @code="example.resource.missing_label"
            @severity="warning"
            @behavior="schema:required-fields"
            @message="Page resources should declare a label"
        }
        {diagnostic
            @code="example.resource.reference_dependency"
            @severity="warning"
            @behavior="schema:field-dependency"
            @message="Referenced resources must use compatible field metadata"
        }
        {diagnostic
            @code="example.resource.invalid_kind"
            @severity="error"
            @behavior="schema:value-vocabulary"
            @message="Resource kind must use the declared vocabulary"
        }
        {diagnostic
            @code="example.resource.invalid_href"
            @severity="error"
            @behavior="schema:scalar-type"
            @message="Resource href must be an absolute URI"
        }
        {diagnostic
            @code="example.resource.invalid_format"
            @severity="error"
            @behavior="schema:scalar-type"
            @message="Resource format must be a media type"
        }
        {diagnostic
            @code="example.resource.invalid_href_scheme"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource href must use the declared URI constraints"
        }
        {diagnostic
            @code="example.resource.invalid_format_type"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource format must use the declared media-type constraints"
        }
        {diagnostic
            @code="example.resource.invalid_payload_suffix"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource payload must use the declared structured media-type suffix"
        }
        {diagnostic
            @code="example.resource.invalid_metadata_parameter"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource metadata must not use forbidden media-type parameters"
        }
        {diagnostic
            @code="example.resource.invalid_asset"
            @severity="error"
            @behavior="schema:scalar-type"
            @message="Resource asset must be a scoped path"
        }
        {diagnostic
            @code="example.resource.invalid_asset_path"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource asset must use the declared path constraints"
        }
        {diagnostic
            @code="example.resource.invalid_priority"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource priority must stay within the declared bounds"
        }
        {diagnostic
            @code="example.resource.invalid_rank"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource rank must stay within the declared exclusive bounds"
        }
        {diagnostic
            @code="example.resource.invalid_weight"
            @severity="error"
            @behavior="schema:scalar-type"
            @message="Resource weight must be numeric"
        }
        {diagnostic
            @code="example.resource.invalid_weight_range"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource weight must stay within the declared decimal bounds"
        }
        {diagnostic
            @code="example.resource.invalid_serial_digits"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource serial must satisfy the declared total digit limit"
        }
        {diagnostic
            @code="example.resource.invalid_ratio_digits"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource ratio must satisfy the declared digit limits"
        }
        {diagnostic
            @code="example.resource.invalid_slug"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource slug must match the declared prefix, suffix, and pattern constraints"
        }
        {diagnostic
            @code="example.resource.invalid_tags"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource tags must use the declared list item bounds"
        }
        {diagnostic
            @code="example.resource.invalid_aliases"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource aliases must use the declared list item count"
        }
        {diagnostic
            @code="example.resource.invalid_code"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Resource code must have the declared length"
        }
        {diagnostic
            @code="example.resource.invalid_inline"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Inline resource text must stay within the declared length bounds"
        }
        {diagnostic
            @code="example.resource.invalid_qualified_name"
            @severity="error"
            @behavior="schema:scalar-type"
            @message="Resource qualified name must use CEM qualified-name syntax"
        }
        {diagnostic
            @code="example.resource.invalid_version"
            @severity="error"
            @behavior="schema:scalar-type"
            @message="Resource version must use semantic version syntax"
        }
        {diagnostic
            @code="example.resource.link_choice"
            @severity="error"
            @behavior="schema:field-contract"
            @message="Resource must choose one link source"
        }
        {diagnostic
            @code="example.resource.field_range"
            @severity="error"
            @behavior="schema:field-contract"
            @message="Resource field children must stay within the declared range"
        }
        {diagnostic
            @code="example.resource.child_choice"
            @severity="error"
            @behavior="schema:field-contract"
            @message="Linked resources must choose one child source"
        }
    }

    {open-content |
        {accept @kind="extension-element" @policy="reject-unless-declared"}
    }
}
```

<details>
<summary>custom-behavior-schema</summary>

- Source: [`examples/custom-behavior-schema.cem`](./examples/custom-behavior-schema.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="custom-behavior" @namespace="https://example.test/ns/custom-behavior/1" @version="1.0.0" |
    {summary |
        {text | Custom schema example that defines a diagnostic algorithm with CEM-QL candidate matching and a CEM-ML behavior function.}
    }

    {content-types |
        {content-type @value="application/vnd.example.custom-behavior+cem" @primary=true}
    }

    {elements |
        {element @name="resource" @optional-attributes="kind label"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
        {attribute @name="label" @type="schema:string"}
    }

    {behaviors |
        {behavior
            @name="page-label"
            @implementation="function"
            @execution="ast-validation"
            @function="page-label-result"
            @select="resource"
            @match='kind == "page" && label == null' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true @default="label"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate" |
                {detail @name="checkKind" @type="schema:identifier" @required=true}
                {detail @name="element" @type="schema:identifier" @required=true}
                {detail @name="kind" @type="schema:identifier" @required=true}
                {detail @name="expected" @type="schema:string" @required=true}
                {detail @name="expectedFields" @type="schema:array" @required=true}
                {detail @name="sample" @type="schema:object" @required=true}
            }
            {function @name="page-label-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Page resource needs a label", details: { checkKind: "page-label", element: $candidate.name, kind: $candidate.attributes.kind, expected: $expected, expectedFields: [$expected], sample: { enabled: true, count: 1, nothing: null } } } }}
            }
        }
    }

    {diagnostics |
        {diagnostic @code="example.page_label" @severity="warning" @behavior="page-label"}
    }
}
```

<details>
<summary>custom-behavior-schema-strict</summary>

- Source: [`examples/custom-behavior-schema-strict.cem`](./examples/custom-behavior-schema-strict.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema-strict.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="custom-behavior-strict" @namespace="https://example.test/ns/custom-behavior-strict/1" @version="1.0.0" |
    {summary |
        {text | Variant custom schema example that changes the declarative match and behavior result without engine code.}
    }

    {content-types |
        {content-type @value="application/vnd.example.custom-behavior-strict+cem" @primary=true}
    }

    {elements |
        {element @name="resource" @optional-attributes="kind status title"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
        {attribute @name="status" @type="schema:identifier"}
        {attribute @name="title" @type="schema:string"}
    }

    {behaviors |
        {behavior
            @name="published-page-title"
            @implementation="function"
            @execution="ast-validation"
            @function="published-page-title-result"
            @select="resource"
            @match='kind == "page" && status == "published" && title == null' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true @default="title"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate" |
                {detail @name="checkKind" @type="schema:identifier" @required=true}
                {detail @name="element" @type="schema:identifier" @required=true}
                {detail @name="status" @type="schema:identifier" @required=true}
                {detail @name="expected" @type="schema:string" @required=true}
            }
            {function @name="published-page-title-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Published page resource needs a title", details: { checkKind: "published-page-title", element: $candidate.name, status: $candidate.attributes.status, expected: $expected } } }}
            }
        }
    }

    {diagnostics |
        {diagnostic @code="example.published_page_title" @severity="error" @behavior="published-page-title"}
    }
}
```

<details>
<summary>invalid-missing-required-attribute</summary>

- Source: [`examples/invalid-missing-required-attribute.cem`](./examples/invalid-missing-required-attribute.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_model.missing_required_attribute`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-missing-required-attribute.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="broken" @version="1.0.0" |
    {summary |
        {text | Missing the required schema namespace attribute.}
    }
}
```

<details>
<summary>invalid-diagnostic-behavior</summary>

- Source: [`examples/invalid-diagnostic-behavior.cem`](./examples/invalid-diagnostic-behavior.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.unknown_diagnostic_behavior`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-diagnostic-behavior.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-diagnostic-behavior" @namespace="https://example.test/ns/invalid-diagnostic-behavior/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }

    {elements |
        {element @name="item"}
    }

    {diagnostics |
        {diagnostic
            @code="example.invalid_behavior"
            @severity="error"
            @behavior="schema:not-an-engine-behavior"
        }
    }
}
```

<details>
<summary>invalid-custom-behavior-unresolved-function</summary>

- Source: [`examples/invalid-custom-behavior-unresolved-function.cem`](./examples/invalid-custom-behavior-unresolved-function.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.unresolved_behavior_function`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-unresolved-function.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-custom-behavior-unresolved-function" @namespace="https://example.test/ns/invalid-custom-behavior-unresolved-function/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
    }

    {behaviors |
        {behavior
            @name="page-kind"
            @implementation="function"
            @execution="ast-validation"
            @function="missing-page-kind-result"
            @select="resource"
            @match='kind = "page"' |
            {result @type="schema:diagnostic-result" @source-range="candidate"}
        }
    }

    {diagnostics |
        {diagnostic @code="example.page_kind" @severity="error" @behavior="page-kind"}
    }
}
```

<details>
<summary>invalid-custom-behavior-select-query</summary>

- Source: [`examples/invalid-custom-behavior-select-query.cem`](./examples/invalid-custom-behavior-select-query.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_behavior.query_invalid`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-select-query.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-custom-behavior-select-query" @namespace="https://example.test/ns/invalid-custom-behavior-select-query/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
    }

    {behaviors |
        {behavior
            @name="page-kind"
            @implementation="function"
            @execution="ast-validation"
            @function="page-kind-result"
            @select="missing_resource"
            @match='kind = "page"' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="page-kind-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {body | {$ { message: "Page kind failed", details: { element: $candidate.name } } }}
            }
        }
    }

    {diagnostics |
        {diagnostic @code="example.page_kind" @severity="error" @behavior="page-kind"}
    }
}
```

<details>
<summary>invalid-custom-behavior-match-query</summary>

- Source: [`examples/invalid-custom-behavior-match-query.cem`](./examples/invalid-custom-behavior-match-query.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_behavior.query_invalid`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-match-query.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-custom-behavior-match-query" @namespace="https://example.test/ns/invalid-custom-behavior-match-query/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
    }

    {behaviors |
        {behavior
            @name="page-kind"
            @implementation="function"
            @execution="ast-validation"
            @function="page-kind-result"
            @select="resource"
            @match='missing_kind = "page"' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="page-kind-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {body | {$ { message: "Page kind failed", details: { element: $candidate.name } } }}
            }
        }
    }

    {diagnostics |
        {diagnostic @code="example.page_kind" @severity="error" @behavior="page-kind"}
    }
}
```

<details>
<summary>invalid-custom-behavior-argument-type</summary>

- Source: [`examples/invalid-custom-behavior-argument-type.cem`](./examples/invalid-custom-behavior-argument-type.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_diagnostic_behavior_contract`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-argument-type.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-custom-behavior-argument-type" @namespace="https://example.test/ns/invalid-custom-behavior-argument-type/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
    }

    {behaviors |
        {behavior
            @name="page-limit"
            @implementation="function"
            @execution="ast-validation"
            @function="page-limit-result"
            @select="resource"
            @match='kind = "page"' |
            {parameters |
                {parameter @name="minimum" @type="schema:integer" @required=true}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="page-limit-result" @returns="object" @deterministic=true |
                {param @name="minimum" @type="integer" @required=true}
                {body | {$ { message: "Page limit failed", details: { minimum: $minimum } } }}
            }
        }
    }

    {diagnostics |
        {diagnostic @code="example.page_limit" @severity="error" @behavior="page-limit" |
            {arguments |
                {argument @name="minimum" @value="many"}
            }
        }
    }
}
```

<details>
<summary>invalid-custom-behavior-signature</summary>

- Source: [`examples/invalid-custom-behavior-signature.cem`](./examples/invalid-custom-behavior-signature.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_diagnostic_behavior_contract`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-signature.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-custom-behavior-signature" @namespace="https://example.test/ns/invalid-custom-behavior-signature/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
    }

    {behaviors |
        {behavior
            @name="page-label"
            @implementation="function"
            @execution="ast-validation"
            @function="page-label-result"
            @select="resource"
            @match='kind = "page"' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="page-label-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Page label failed", details: { expected: $expected } } }}
            }
        }
    }

    {diagnostics |
        {diagnostic @code="example.page_label" @severity="error" @behavior="page-label"}
    }
}
```

<details>
<summary>invalid-custom-behavior-unsafe-call</summary>

- Source: [`examples/invalid-custom-behavior-unsafe-call.cem`](./examples/invalid-custom-behavior-unsafe-call.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_behavior.function_failed`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-unsafe-call.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-custom-behavior-unsafe-call" @namespace="https://example.test/ns/invalid-custom-behavior-unsafe-call/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
    }

    {behaviors |
        {behavior
            @name="page-kind"
            @implementation="function"
            @execution="ast-validation"
            @function="page-kind-result"
            @select="resource"
            @match='kind = "page"' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="page-kind-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {body | {$ call("page-kind-result", { candidate: $candidate }) }}
            }
        }
    }

    {diagnostics |
        {diagnostic @code="example.page_kind" @severity="error" @behavior="page-kind"}
    }
}
```

<details>
<summary>invalid-custom-behavior-contracts</summary>

- Source: [`examples/invalid-custom-behavior-contracts.cem`](./examples/invalid-custom-behavior-contracts.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_diagnostic_behavior_contract`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-contracts.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-custom-behavior-contracts" @namespace="https://example.test/ns/invalid-custom-behavior-contracts/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind"}
    }

    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
    }

    {behaviors |
        {behavior
            @name="macro-behavior"
            @implementation="macro"
            @execution="ast-validation"
        }

        {behavior
            @name="render-behavior"
            @implementation="function"
            @execution="render"
            @function="render-result"
            @select="resource"
            @match='kind = "page"' |
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="render-result" @returns="object" @deterministic=true |
                {body | {$ { message: "Render behavior failed" } }}
            }
        }

        {behavior
            @name="missing-select"
            @implementation="function"
            @execution="ast-validation"
            @function="missing-select-result"
            @match='kind = "page"' |
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="missing-select-result" @returns="object" @deterministic=true |
                {body | {$ { message: "Missing select failed" } }}
            }
        }

        {behavior
            @name="missing-match"
            @implementation="function"
            @execution="ast-validation"
            @function="missing-match-result"
            @select="resource" |
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="missing-match-result" @returns="object" @deterministic=true |
                {body | {$ { message: "Missing match failed" } }}
            }
        }

        {behavior
            @name="missing-function-binding"
            @implementation="function"
            @execution="ast-validation"
            @select="resource"
            @match='kind = "page"' |
            {result @type="schema:diagnostic-result" @source-range="candidate"}
        }

        {behavior
            @name="missing-result"
            @implementation="function"
            @execution="ast-validation"
            @function="missing-result-function"
            @select="resource"
            @match='kind = "page"' |
            {function @name="missing-result-function" @returns="object" @deterministic=true |
                {body | {$ { message: "Missing result failed" } }}
            }
        }

        {behavior
            @name="wrong-result-type"
            @implementation="function"
            @execution="ast-validation"
            @function="wrong-result-type-result"
            @select="resource"
            @match='kind = "page"' |
            {result @type="schema:string" @source-range="candidate"}
            {function @name="wrong-result-type-result" @returns="object" @deterministic=true |
                {body | {$ { message: "Wrong result type failed" } }}
            }
        }

        {behavior
            @name="wrong-return-type"
            @implementation="function"
            @execution="ast-validation"
            @function="wrong-return-type-result"
            @select="resource"
            @match='kind = "page"' |
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="wrong-return-type-result" @returns="string" @deterministic=true |
                {body | {$ "not a diagnostic result" }}
            }
        }

        {behavior
            @name="missing-body"
            @implementation="function"
            @execution="ast-validation"
            @function="missing-body-result"
            @select="resource"
            @match='kind = "page"' |
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="missing-body-result" @returns="object" @deterministic=true}
        }

        {behavior
            @name="engine-with-argument"
            @implementation="engine"
            @execution="ast-validation"
            @primitive="schema:field-contract"
        }
    }

    {diagnostics |
        {diagnostic @code="example.unsupported_implementation" @severity="error" @behavior="macro-behavior"}
        {diagnostic @code="example.unsupported_execution" @severity="error" @behavior="render-behavior"}
        {diagnostic @code="example.missing_select" @severity="error" @behavior="missing-select"}
        {diagnostic @code="example.missing_match" @severity="error" @behavior="missing-match"}
        {diagnostic @code="example.missing_function_binding" @severity="error" @behavior="missing-function-binding"}
        {diagnostic @code="example.missing_result" @severity="error" @behavior="missing-result"}
        {diagnostic @code="example.wrong_result_type" @severity="error" @behavior="wrong-result-type"}
        {diagnostic @code="example.wrong_return_type" @severity="error" @behavior="wrong-return-type"}
        {diagnostic @code="example.missing_body" @severity="error" @behavior="missing-body"}
        {diagnostic @code="example.engine_argument" @severity="error" @behavior="engine-with-argument" |
            {arguments |
                {argument @name="field" @value="kind"}
            }
        }
    }
}
```

<details>
<summary>invalid-datatype-param-length</summary>

- Source: [`examples/invalid-datatype-param-length.cem`](./examples/invalid-datatype-param-length.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_datatype_param`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-length.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-datatype-param-length" @namespace="https://example.test/ns/invalid-datatype-param-length/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="label code count score rank blockedScore blockedRank status body tags aliases title titleRange shortCode longCode tagRange shortTags longAliases"}
    }

    {attributes |
        {attribute @name="label" @type="schema:string" @minLength=-1}
        {attribute @name="code" @type="schema:string" @length=-1}
        {attribute @name="count" @type="schema:integer" @maxLength=3}
        {attribute @name="score" @type="schema:number" @stringPrefixes="score-"}
        {attribute @name="rank" @type="schema:integer" @stringSuffixes="-rank"}
        {attribute @name="blockedScore" @type="schema:number" @stringForbiddenPrefixes="draft-"}
        {attribute @name="blockedRank" @type="schema:integer" @stringForbiddenSuffixes="-tmp"}
        {attribute @name="status" @type="schema:integer" @stringIncludes="open"}
        {attribute @name="body" @type="schema:boolean" @stringExcludes="TODO"}
        {attribute @name="tags" @type="schema:name-list" @minItems=-1}
        {attribute @name="aliases" @type="schema:wildcard-name-list" @itemCount=-1}
        {attribute @name="title" @type="schema:string" @maxItems=2}
        {attribute @name="titleRange" @type="schema:string" @minLength=5 @maxLength=3}
        {attribute @name="shortCode" @type="schema:string" @minLength=3 @length=2}
        {attribute @name="longCode" @type="schema:string" @length=5 @maxLength=4}
        {attribute @name="tagRange" @type="schema:name-list" @minItems=4 @maxItems=2}
        {attribute @name="shortTags" @type="schema:name-list" @minItems=3 @itemCount=2}
        {attribute @name="longAliases" @type="schema:wildcard-name-list" @itemCount=5 @maxItems=4}
    }
}
```

<details>
<summary>invalid-datatype-param-bound</summary>

- Source: [`examples/invalid-datatype-param-bound.cem`](./examples/invalid-datatype-param-bound.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_datatype_param`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-bound.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-datatype-param-bound" @namespace="https://example.test/ns/invalid-datatype-param-bound/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="priority title untyped closed openUpper openLower openBoth"}
    }

    {attributes |
        {attribute @name="priority" @type="schema:integer" @minInclusive=0.5}
        {attribute @name="title" @type="schema:string" @minInclusive=1}
        {attribute @name="untyped" @maxInclusive=10}
        {attribute @name="closed" @type="schema:integer" @minInclusive=5 @maxInclusive=4}
        {attribute @name="openUpper" @type="schema:integer" @minInclusive=5 @maxExclusive=5}
        {attribute @name="openLower" @type="schema:number" @minExclusive=3.5 @maxInclusive=3.5}
        {attribute @name="openBoth" @type="schema:number" @minExclusive=1.0 @maxExclusive=1.0}
    }
}
```

<details>
<summary>invalid-datatype-param-pattern</summary>

- Source: [`examples/invalid-datatype-param-pattern.cem`](./examples/invalid-datatype-param-pattern.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_datatype_param`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-pattern.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-datatype-param-pattern" @namespace="https://example.test/ns/invalid-datatype-param-pattern/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="code count"}
    }

    {attributes |
        {attribute @name="code" @type="schema:string" @pattern="["}
        {attribute @name="count" @type="schema:integer" @pattern="[0-9]+"}
    }
}
```

<details>
<summary>invalid-datatype-param-digits</summary>

- Source: [`examples/invalid-datatype-param-digits.cem`](./examples/invalid-datatype-param-digits.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_datatype_param`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-digits.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-datatype-param-digits" @namespace="https://example.test/ns/invalid-datatype-param-digits/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="serial ratio code"}
    }

    {attributes |
        {attribute @name="serial" @type="schema:integer" @totalDigits=0}
        {attribute @name="ratio" @type="schema:number" @fractionDigits=-1}
        {attribute @name="code" @type="schema:string" @fractionDigits=2}
    }
}
```

<details>
<summary>invalid-datatype-param-uri-media</summary>

- Source: [`examples/invalid-datatype-param-uri-media.cem`](./examples/invalid-datatype-param-uri-media.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_datatype_param`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-uri-media.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-datatype-param-uri-media" @namespace="https://example.test/ns/invalid-datatype-param-uri-media/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }

    {attributes |
        {attribute @name="template" @type="schema:path" @pathPrefixes="/absolute ./../bad"}
        {attribute @name="blockedTemplate" @type="schema:path" @pathForbiddenPrefixes="/private ./../secret"}
        {attribute @name="directory" @type="schema:path" @pathDirectoryNames="templates bad/name"}
        {attribute @name="blockedDirectory" @type="schema:path" @pathForbiddenDirectoryNames="private bad/name"}
        {attribute @name="script" @type="schema:path" @pathExtensions="cem .cem"}
        {attribute @name="blockedScript" @type="schema:path" @pathForbiddenExtensions="tmp .bak"}
        {attribute @name="image" @type="schema:path" @pathBasenames="card.cem bad/name"}
        {attribute @name="blockedImage" @type="schema:path" @pathForbiddenBasenames="secret.cem bad/name"}
        {attribute @name="href" @type="schema:uri" @uriSchemes="https ftp" @uriForbiddenSchemes="ftp 1bad"}
        {attribute @name="cdn" @type="schema:uri" @uriHosts="api.example.test blocked.example.test bad/host" @uriForbiddenHosts="blocked.example.test bad/host"}
        {attribute @name="portal" @type="schema:uri" @uriPorts="443 8080 0443 65536" @uriForbiddenPorts="8080 0443 65536"}
        {attribute @name="remote" @type="schema:uri" @uriRequiresAuthority=maybe}
        {attribute @name="asset" @type="schema:uri" @uriPathPrefixes="assets"}
        {attribute @name="blockedAsset" @type="schema:uri" @uriForbiddenPathPrefixes="private"}
        {attribute @name="download" @type="schema:uri" @uriPathExtensions="cem .json"}
        {attribute @name="blockedDownload" @type="schema:uri" @uriForbiddenPathExtensions="tmp .bak"}
        {attribute @name="uriFile" @type="schema:uri" @uriPathBasenames="schema.cem bad/name"}
        {attribute @name="blockedUriFile" @type="schema:uri" @uriForbiddenPathBasenames="secret.cem bad/name"}
        {attribute @name="query" @type="schema:uri" @uriQueries="view=resource ?bad"}
        {attribute @name="queryBlocked" @type="schema:uri" @uriQueries="view=resource debug=true" @uriForbiddenQueries="debug=true ?bad"}
        {attribute @name="queryParams" @type="schema:uri" @uriQueryParameters="view bad=name"}
        {attribute @name="queryValue" @type="schema:uri" @uriQueryParameterValues="view=resource bad bad&name=value"}
        {attribute @name="queryForbidden" @type="schema:uri" @uriQueryForbiddenParameters="debug bad=name"}
        {attribute @name="queryRequired" @type="schema:uri" @uriQueryRequiredParameters="view bad=name"}
        {attribute @name="queryPresenceConflict" @type="schema:uri" @uriQueryForbiddenParameters="debug" @uriQueryRequiredParameters="debug"}
        {attribute @name="queryValueConflict" @type="schema:uri" @uriQueryForbiddenParameters="debug" @uriQueryParameterValues="debug=true"}
        {attribute @name="anchor" @type="schema:uri" @uriFragments="overview #bad"}
        {attribute @name="anchorBlocked" @type="schema:uri" @uriFragments="overview debug" @uriForbiddenFragments="debug #bad"}
        {attribute @name="format" @type="schema:media-type" @mediaTypes="text/html application/json text" @mediaTypeForbiddenEssences="application/json text"}
        {attribute @name="typed" @type="schema:media-type" @mediaTypeTypes="application bad/type"}
        {attribute @name="subtyped" @type="schema:media-type" @mediaTypeSubtypes="json bad/subtype"}
        {attribute @name="structured" @type="schema:media-type" @mediaTypeSuffixes="json bad=suffix"}
        {attribute @name="blockedTyped" @type="schema:media-type" @mediaTypeForbiddenTypes="image bad/type"}
        {attribute @name="blockedSubtyped" @type="schema:media-type" @mediaTypeForbiddenSubtypes="html bad/subtype"}
        {attribute @name="blockedStructured" @type="schema:media-type" @mediaTypeForbiddenSuffixes="json bad=suffix"}
        {attribute @name="payload" @type="schema:media-type" @mediaTypeParameters="charset bad=name"}
        {attribute @name="encoding" @type="schema:media-type" @mediaTypeParameterValues="charset=utf-8 bad bad/name=value"}
        {attribute @name="legacy" @type="schema:media-type" @mediaTypeForbiddenParameters="profile bad=name"}
        {attribute @name="profiled" @type="schema:media-type" @mediaTypeRequiredParameters="charset bad=name"}
        {attribute @name="mediaPresenceConflict" @type="schema:media-type" @mediaTypeForbiddenParameters="profile" @mediaTypeRequiredParameters="profile"}
        {attribute @name="mediaValueConflict" @type="schema:media-type" @mediaTypeForbiddenParameters="profile" @mediaTypeParameterValues="profile=default"}
        {attribute @name="label" @type="schema:string" @mediaTypeSuffixes="json"}
        {attribute @name="title" @type="schema:string" @mediaTypeForbiddenParameters="profile"}
        {attribute @name="description" @type="schema:string" @mediaTypeParameterValues="charset=utf-8"}
        {attribute @name="formatLabel" @type="schema:string" @mediaTypeForbiddenEssences="text/html"}
        {attribute @name="schemeBlockedLabel" @type="schema:string" @uriForbiddenSchemes="ftp"}
        {attribute @name="typeLabel" @type="schema:string" @mediaTypeTypes="application"}
        {attribute @name="subtitle" @type="schema:string" @mediaTypeSubtypes="json"}
        {attribute @name="typeBlockedLabel" @type="schema:string" @mediaTypeForbiddenTypes="image"}
        {attribute @name="subtitleBlocked" @type="schema:string" @mediaTypeForbiddenSubtypes="html"}
        {attribute @name="structuredBlockedLabel" @type="schema:string" @mediaTypeForbiddenSuffixes="json"}
        {attribute @name="linkLabel" @type="schema:string" @uriHosts="api.example.test"}
        {attribute @name="linkBlockedLabel" @type="schema:string" @uriForbiddenHosts="api.example.test"}
        {attribute @name="portLabel" @type="schema:string" @uriPorts="443"}
        {attribute @name="portBlockedLabel" @type="schema:string" @uriForbiddenPorts="443"}
        {attribute @name="assetBlockedLabel" @type="schema:string" @uriForbiddenPathPrefixes="/private/"}
        {attribute @name="downloadLabel" @type="schema:string" @uriPathExtensions="cem"}
        {attribute @name="downloadBlockedLabel" @type="schema:string" @uriForbiddenPathExtensions="tmp"}
        {attribute @name="uriFileLabel" @type="schema:string" @uriPathBasenames="schema.cem"}
        {attribute @name="uriFileBlockedLabel" @type="schema:string" @uriForbiddenPathBasenames="secret.cem"}
        {attribute @name="queryLabel" @type="schema:string" @uriQueries="view=resource"}
        {attribute @name="queryBlockedLabel" @type="schema:string" @uriForbiddenQueries="debug=true"}
        {attribute @name="queryParamsLabel" @type="schema:string" @uriQueryParameters="view"}
        {attribute @name="queryValueLabel" @type="schema:string" @uriQueryParameterValues="view=resource"}
        {attribute @name="queryForbiddenLabel" @type="schema:string" @uriQueryForbiddenParameters="debug"}
        {attribute @name="queryRequiredLabel" @type="schema:string" @uriQueryRequiredParameters="view"}
        {attribute @name="anchorLabel" @type="schema:string" @uriFragments="overview"}
        {attribute @name="anchorBlockedLabel" @type="schema:string" @uriForbiddenFragments="debug"}
        {attribute @name="caption" @type="schema:string" @pathPrefixes="./templates/"}
        {attribute @name="blockedCaption" @type="schema:string" @pathForbiddenPrefixes="./private/"}
        {attribute @name="directoryLabel" @type="schema:string" @pathDirectoryNames="templates"}
        {attribute @name="directoryBlockedLabel" @type="schema:string" @pathForbiddenDirectoryNames="private"}
        {attribute @name="summary" @type="schema:string" @pathExtensions="cem"}
        {attribute @name="blockedSummary" @type="schema:string" @pathForbiddenExtensions="tmp"}
        {attribute @name="basenameLabel" @type="schema:string" @pathBasenames="card.cem"}
        {attribute @name="blockedBasenameLabel" @type="schema:string" @pathForbiddenBasenames="secret.cem"}
    }
}
```

<details>
<summary>invalid-field-contract-presence</summary>

- Source: [`examples/invalid-field-contract-presence.cem`](./examples/invalid-field-contract-presence.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_field_contract`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-presence.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-field-contract-presence" @namespace="https://example.test/ns/invalid-field-contract-presence/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }

    {elements |
        {element @name="group" @optional-attributes="id title label" @children="header main footer aside"}
        {element @name="header"}
        {element @name="main"}
        {element @name="footer"}
        {element @name="aside"}
    }

    {field-contracts |
        {field-contract
            @name="bad-required-forbidden-attribute"
            @target="group"
            @required-attributes="id"
            @forbidden-attributes="id"
            @diagnostic="example.group_presence"
            @behavior="schema:field-contract"
        }
        {field-contract
            @name="bad-required-one-forbidden-attributes"
            @target="group"
            @required-one-attributes="title label"
            @forbidden-attributes="title label"
            @diagnostic="example.group_presence"
            @behavior="schema:choice-case"
        }
        {field-contract
            @name="bad-required-forbidden-child"
            @target="group"
            @required-children="header"
            @forbidden-children="header"
            @diagnostic="example.group_presence"
            @behavior="schema:child-occurrence"
        }
        {field-contract
            @name="bad-required-unaccepted-child"
            @target="group"
            @accepted-children="header"
            @required-children="main"
            @diagnostic="example.group_presence"
            @behavior="schema:accepted-children"
        }
        {field-contract
            @name="bad-required-one-forbidden-child"
            @target="group"
            @required-one-child="header main"
            @forbidden-children="header main"
            @diagnostic="example.group_presence"
            @behavior="schema:child-occurrence"
        }
        {field-contract
            @name="bad-required-one-unaccepted-child"
            @target="group"
            @accepted-children="header"
            @required-one-child="main footer"
            @diagnostic="example.group_presence"
            @behavior="schema:accepted-children"
        }
    }

    {diagnostics |
        {diagnostic
            @code="example.group_presence"
            @severity="error"
            @behavior="schema:field-contract"
            @message="Group fields and children must satisfy consistent presence contracts"
        }
    }
}
```

<details>
<summary>invalid-field-contract-condition</summary>

- Source: [`examples/invalid-field-contract-condition.cem`](./examples/invalid-field-contract-condition.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_field_contract`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-condition.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-field-contract-condition" @namespace="https://example.test/ns/invalid-field-contract-condition/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }

    {elements |
        {element @name="asset" @optional-attributes="kind source token format" @children="reference fallback thumbnail"}
        {element @name="reference"}
        {element @name="fallback"}
        {element @name="thumbnail"}
    }

    {field-contracts |
        {field-contract
            @name="bad-values-without-attribute"
            @target="asset"
            @when-values="remote"
            @required-attributes="format"
            @diagnostic="example.asset_condition"
            @behavior="schema:field-dependency"
        }
        {field-contract
            @name="bad-attribute-present-absent"
            @target="asset"
            @when-attribute="kind"
            @when-values="remote"
            @when-absent-attributes="kind"
            @required-attributes="format"
            @diagnostic="example.asset_condition"
            @behavior="schema:field-dependency"
        }
        {field-contract
            @name="bad-all-present-absent-attributes"
            @target="asset"
            @when-present-attributes="source token"
            @when-absent-attributes="token"
            @required-attributes="format"
            @diagnostic="example.asset_condition"
            @behavior="schema:field-dependency"
        }
        {field-contract
            @name="bad-any-present-all-absent-attributes"
            @target="asset"
            @when-any-present-attributes="source token"
            @when-absent-attributes="source token"
            @required-attributes="format"
            @diagnostic="example.asset_condition"
            @behavior="schema:field-dependency"
        }
        {field-contract
            @name="bad-any-absent-all-present-attributes"
            @target="asset"
            @when-present-attributes="source token"
            @when-any-absent-attributes="source token"
            @required-attributes="format"
            @diagnostic="example.asset_condition"
            @behavior="schema:field-dependency"
        }
        {field-contract
            @name="bad-all-present-absent-children"
            @target="asset"
            @when-present-children="reference fallback"
            @when-absent-children="fallback"
            @required-attributes="format"
            @diagnostic="example.asset_condition"
            @behavior="schema:field-dependency"
        }
        {field-contract
            @name="bad-any-present-all-absent-children"
            @target="asset"
            @when-any-present-children="reference fallback"
            @when-absent-children="reference fallback"
            @required-attributes="format"
            @diagnostic="example.asset_condition"
            @behavior="schema:field-dependency"
        }
        {field-contract
            @name="bad-any-absent-all-present-children"
            @target="asset"
            @when-present-children="reference fallback"
            @when-any-absent-children="reference fallback"
            @required-attributes="format"
            @diagnostic="example.asset_condition"
            @behavior="schema:field-dependency"
        }
    }

    {diagnostics |
        {diagnostic
            @code="example.asset_condition"
            @severity="error"
            @behavior="schema:field-dependency"
            @message="Asset condition selectors must be satisfiable"
        }
    }
}
```

<details>
<summary>invalid-field-contract-child-sequence</summary>

- Source: [`examples/invalid-field-contract-child-sequence.cem`](./examples/invalid-field-contract-child-sequence.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_field_contract`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-child-sequence.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-field-contract-child-sequence" @namespace="https://example.test/ns/invalid-field-contract-child-sequence/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }

    {elements |
        {element @name="group" @children="header main footer aside"}
        {element @name="header"}
        {element @name="main"}
        {element @name="footer"}
        {element @name="aside"}
    }

    {field-contracts |
        {field-contract
            @name="bad-first-boundary"
            @target="group"
            @first-child="header"
            @forbidden-first-child="header"
            @diagnostic="example.group_child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="boundary-children"
        }
        {field-contract
            @name="bad-last-boundary"
            @target="group"
            @last-child="footer"
            @forbidden-last-child="footer"
            @diagnostic="example.group_child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="boundary-children"
        }
        {field-contract
            @name="bad-required-forbidden-sequence"
            @target="group"
            @required-child-sequence="header main"
            @forbidden-child-sequence="header main"
            @diagnostic="example.group_child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="required-child-sequence"
        }
        {field-contract
            @name="bad-prefix-forbidden-prefix"
            @target="group"
            @prefix-child-sequence="header main"
            @forbidden-prefix-child-sequence="header main"
            @diagnostic="example.group_child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="prefix-child-sequence"
        }
        {field-contract
            @name="bad-suffix-forbidden-suffix"
            @target="group"
            @suffix-child-sequence="main footer"
            @forbidden-suffix-child-sequence="main footer"
            @diagnostic="example.group_child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="suffix-child-sequence"
        }
        {field-contract
            @name="bad-exact-prefix"
            @target="group"
            @exact-child-sequence="header footer"
            @prefix-child-sequence="header main"
            @diagnostic="example.group_child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="exact-child-sequence"
        }
        {field-contract
            @name="bad-exact-required"
            @target="group"
            @exact-child-sequence="header footer"
            @required-child-sequence="main footer"
            @diagnostic="example.group_child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="exact-child-sequence"
        }
        {field-contract
            @name="bad-exact-forbidden"
            @target="group"
            @exact-child-sequence="header main footer"
            @forbidden-child-sequence="main footer"
            @diagnostic="example.group_child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="exact-child-sequence"
        }
    }

    {diagnostics |
        {diagnostic
            @code="example.group_child_sequence"
            @severity="error"
            @behavior="schema:field-contract"
            @message="Group children must satisfy consistent sequence contracts"
        }
    }
}
```

<details>
<summary>invalid-attribute-default</summary>

- Source: [`examples/invalid-attribute-default.cem`](./examples/invalid-attribute-default.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_definition.invalid_default_value`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-attribute-default.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="invalid-attribute-default" @namespace="https://example.test/ns/invalid-attribute-default/1" @version="1.0.0" |
    {elements |
        {element @name="item" @optional-attributes="count mode code"}
    }

    {attributes |
        {attribute @name="count" @type="schema:integer" @default="many"}
        {attribute @name="mode" @type="schema:identifier" @values="compact pretty" @default="tabular"}
        {attribute @name="code" @type="schema:string" @minLength=2 @default="x"}
    }
}
```

<details>
<summary>invalid-unclosed-schema</summary>

- Source: [`examples/invalid-unclosed-schema.cem`](./examples/invalid-unclosed-schema.cem)
- Content type: `application/vnd.cem.schema+cem`
- Schema: `https://cem.dev/ns/schema/1`
- Expected result: `fail`
- Expected diagnostics: `cem.ast.unclosed_scope`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/schema/v1/examples/invalid-unclosed-schema.cem,contentType=application/vnd.cem.schema+cem,schema=https://cem.dev/ns/schema/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="broken" @namespace="https://example.test/ns/broken/1" @version="1.0.0" |
    {summary |
        {text | Missing schema close}
```
