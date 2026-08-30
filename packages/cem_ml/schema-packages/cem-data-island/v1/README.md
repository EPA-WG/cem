# CEM Data-Island Schema Package

This package owns the namespace-aware CEM AST context used by serialized
`cem-element` instances. The root schema URI is
`https://cem.dev/ns/runtime/data-island`; the browser lifecycle contract carried
by the root is currently `0.1.2`.

The context root fixes domain-part order and admission. Qualified domain
namespaces keep attributes, the derived dataset, payload, slices, portable
resource lifecycle state, forms, validation, events, and hydration data
distinguishable to validators, transforms, IDEs, and debuggers. Payload and
runtime result children retain their producing schema namespaces instead of
crossing a JSON data model.

Live host capabilities such as controllers, listeners, streams, and DOM or
storage handles are not AST data. Hydration recreates them from portable
semantic state under the current resolver and security policies.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>complete-instance</summary>

- Source: [`examples/complete-instance.cem`](./examples/complete-instance.cem)
- Content type: `application/vnd.cem.data-island+cem`
- Schema: `https://cem.dev/ns/runtime/data-island`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-data-island/v1/examples/complete-instance.cem,contentType=application/vnd.cem.data-island+cem,schema=https://cem.dev/ns/runtime/data-island \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns cem-island = "https://cem.dev/ns/runtime/data-island"
@ns cem-hydration = "https://cem.dev/ns/runtime/hydration-data"
@ns cem-attributes = "https://cem.dev/ns/runtime/instance-attributes"
@ns cem-dataset = "https://cem.dev/ns/runtime/instance-dataset"
@ns cem-payload = "https://cem.dev/ns/runtime/instance-payload"
@ns cem-slices = "https://cem.dev/ns/runtime/instance-slices"
@ns cem-resources = "https://cem.dev/ns/runtime/resource-state"
@ns cem-form = "https://cem.dev/ns/runtime/form-state"
@ns cem-validation = "https://cem.dev/ns/runtime/validation-state"
@ns cem-events = "https://cem.dev/ns/runtime/event-state"
@ns html = "http://www.w3.org/1999/xhtml"
@default cem-island

{context-root @version="0.1.2" |
    {cem-hydration:data |
        {cem-hydration:field @name=version @kind=string | 0.1.2}
        {cem-hydration:field @name=instanceId @kind=string | fruit-card-1}
        {cem-hydration:field @name=outputTarget @kind=string | light-dom}
    }
    {cem-attributes:attributes |
        {cem-attributes:attribute @name=title @value="Fruit card"}
        {cem-attributes:attribute @name=data-fruit @value=banana}
    }
    {cem-dataset:dataset |
        {cem-dataset:entry @key=fruit @value=banana}
    }
    {cem-payload:payload @content-type=text/html @schema="https://cem.dev/ns/data/html/1" |
        {html:strong @slot=heading | Fruit inventory}
    }
    {cem-slices:slices |
        {cem-slices:slice @name=selection @kind=string | banana}
    }
    {cem-resources:resources |
        {cem-resources:resource @name=revisions @kind=record |
            {cem-resources:resource @name="http:fruit" @kind=number | 1}
        }
    }
    {cem-form:form-state |
        {cem-form:field @name=quantity @kind=string | 2}
    }
    {cem-validation:validation-state |
        {cem-validation:field @name=quantity @kind=string | valid}
    }
    {cem-events:event-state |
        {cem-events:event @name=change @kind=record |
            {cem-events:event @name=type @kind=string | change}
        }
    }
}
```
