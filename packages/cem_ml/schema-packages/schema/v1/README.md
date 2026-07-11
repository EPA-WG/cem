# CEM Schema Definition Language Package

Status: initial source package

This package defines the CEM-ML schema declaration language used to describe
validation schemas for input content.

Owned schema URI:

```text
https://cem.dev/ns/schema/1
```

Primary content type:

```text
application/vnd.cem.schema+cem
```

Schema source files are ordinary CEM-ML documents using this namespace for the
schema-authoring vocabulary. The target schema being described is carried by the
`schema @namespace` attribute.

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
`when-values`, and by presence selectors such as `when-present-attributes`.
Use presence selectors for dependent-required rules such as "when this
attribute is present, require these other attributes".
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
Use `min-children` and `max-children` for broader child occurrence ranges
expressed as `child=count` name-value pairs.
Use `path-layout-attributes` with `path-layout-prefix` and
`path-layout-extension` for package-relative path layout contracts, such as
formatter artifacts under `formatters/` and colorizer artifacts under
`colorizers/`.

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

Individual `{field-contract}` declarations can also bind `@diagnostic` plus
`@behavior`. The diagnostic code remains the report identity, while the
contract-local behavior selects the operational algorithm for that contract.
This lets one broad diagnostic family such as
`cem.schema_package.converter_check` report a conditional dependency through
`schema:field-dependency` and a choice/case exclusion through
`schema:choice-case`.

Attribute-owned engine primitives are also schema-visible. Attribute
declarations bind `@values` failures through `@values-diagnostic` to
`schema:value-vocabulary`, scalar syntax failures for boolean, integer, number,
URI, media-type, and path attributes through `@type-diagnostic` to
`schema:scalar-type`,
and datatype parameter failures for integer
`@minInclusive`/`@maxInclusive`/`@minExclusive`/`@maxExclusive` bounds, string
`@minLength`/`@maxLength`/`@length` constraints, and regex `@pattern` through
`@datatype-param-diagnostic` to
`schema:datatype-param`. In all cases the diagnostic `@code` remains the
stable output identity while `@behavior` selects the reusable algorithm
contract.

Operational constraints bind their execution behavior at the `{constraint}`
declaration while keeping the diagnostic family code stable. Constraint
declarations can use `@diagnostic` plus `@behavior` to bind resource readability
checks to `schema:resource-readable`, parser/validation checks to
`schema:resource-parse`, and cross-reference checks to
`schema:reference-resolution`. This lets a schema-package diagnostic such as
`cem.schema_package.artifact_check` remain the report identity while individual
constraint `checkKind` values select different engine algorithms.

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
    @match='kind = "page" and label = null' |
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
        {body | {$ { message: "Page resource needs a label", details: { checkKind: "resource-label", expected: $expected, element: $candidate.name } } }}
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
Constraint-owned `schema:resource-readable`, `schema:resource-parse`, and
`schema:reference-resolution` bindings now compile and stamp operational
diagnostics with their declared behavior. Integer `minInclusive`,
`maxInclusive`, `minExclusive`, and `maxExclusive` bounds, string
`minLength`/`maxLength`/`length` constraints, and regex `pattern` datatype
parameter variations now execute through
`schema:datatype-param`; boolean, integer, number, basic absolute-URI, basic
media-type, and scope-context path scalar syntax now execute through
`schema:scalar-type`;
required-one/max-one attribute choice cardinality and nested choice/case groups
now execute through `schema:choice-case`; `min-children`/`max-children` child
occurrence ranges now execute through `schema:child-occurrence`; additional
datatype parameter variations remain follow-up work.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-schema.cem`](examples/basic-schema.cem) | Minimal schema definition with content type, element, and attribute declarations. | Pass |
| [`typed-resource-schema.cem`](examples/typed-resource-schema.cem) | Resource schema with imports, conditional `schema:required-fields`, nested exact-one `schema:choice-case`, ranged `schema:child-occurrence`, `schema:value-vocabulary`, `schema:scalar-type` number/URI/media-type/path syntax, and `schema:datatype-param` integer-bound/string length/pattern attribute diagnostics, and open-content policy. | Pass |
| [`custom-behavior-schema.cem`](examples/custom-behavior-schema.cem) | Custom schema that defines a diagnostic algorithm with CEM-QL candidate matching and a CEM-ML behavior function. | Pass |
| [`custom-behavior-schema-strict.cem`](examples/custom-behavior-schema-strict.cem) | Variant custom schema that changes the match condition and function-produced result declaratively. | Pass |
| [`invalid-unclosed-schema.cem`](examples/invalid-unclosed-schema.cem) | Missing closing schema scope syntax diagnostic. | Fail with `cem.ast.unclosed_scope` |
| [`invalid-missing-required-attribute.cem`](examples/invalid-missing-required-attribute.cem) | Schema declaration missing its required `namespace` attribute. | Fail with `cem.schema_model.missing_required_attribute` |
| [`invalid-diagnostic-behavior.cem`](examples/invalid-diagnostic-behavior.cem) | Diagnostic references a behavior absent from the imported engine catalog. | Fail with `cem.schema_definition.unknown_diagnostic_behavior` |
| [`invalid-custom-behavior-unresolved-function.cem`](examples/invalid-custom-behavior-unresolved-function.cem) | Custom behavior references a function that the schema does not declare. | Fail with `cem.schema_definition.unresolved_behavior_function` |
| [`invalid-custom-behavior-argument-type.cem`](examples/invalid-custom-behavior-argument-type.cem) | Custom behavior diagnostic argument does not match the declared parameter type. | Fail with `cem.schema_definition.invalid_diagnostic_behavior_contract` |
| [`invalid-custom-behavior-signature.cem`](examples/invalid-custom-behavior-signature.cem) | Custom behavior function requires a parameter with no input, argument, or default binding. | Fail with `cem.schema_definition.invalid_diagnostic_behavior_contract` |
| [`invalid-custom-behavior-unsafe-call.cem`](examples/invalid-custom-behavior-unsafe-call.cem) | Custom behavior body attempts a CEMT-style self-call instead of pure declarative result construction. | Fail with `cem.schema_behavior.function_failed` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.schema+cem \
  --schema https://cem.dev/ns/schema/1 \
  packages/cem_ml/schema-packages/schema/v1/examples/basic-schema.cem
```
