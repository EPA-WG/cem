# CEM Element Template Schema Package

This package owns the source-validation identity for inert templates consumed
by `@epa-wg/cem-elements`.

Schema URI: `https://cem.dev/ns/template/cem-element/1`

Primary content type: `application/vnd.cem.element-template+cem`

The profile declares `attribute`, `slice`, canonical `cem-module-url`, its static
`module-map` prelude entries, legacy `module-url`, `data`, and `option` authoring
instructions. `cem-module-url` accepts a scalar `referrer` or the mutually exclusive
browser `referrer-selector` bridge to one rendered descendant CEM context. CEM control
flow remains in the core `cem:` namespace,
output HTML/SVG/custom elements remain governed by their registered schemas,
and embedded expressions remain governed by CEM-QL. The compiler makes
`datadom`, declared attributes, and declared slices available as runtime
bindings without evaluating a browser data island during validation.

The package intentionally has no package-owned formatter or colorizer CEMT
assets. Generic CEM-tree presentation remains the output contract.

Nx project: `cem_ml_schema_package_cem_element_template_v1`.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-card</summary>

- Source: [`examples/basic-card.cem`](./examples/basic-card.cem)
- Content type: `application/vnd.cem.element-template+cem`
- Schema: `https://cem.dev/ns/template/cem-element/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-element-template/v1/examples/basic-card.cem,contentType=application/vnd.cem.element-template+cem,schema=https://cem.dev/ns/template/cem-element/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns element = "https://cem.dev/ns/template/cem-element/1"
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{attribute @name=label | Default}
{slice @name=open | false}
{button @type=button @aria-expanded="{$open}" |
    {cem:if @test="datadom.attributes.label" | {$label}}
}
```

<details>
<summary>invalid-unknown-instruction</summary>

- Source: [`examples/invalid-unknown-instruction.cem`](./examples/invalid-unknown-instruction.cem)
- Content type: `application/vnd.cem.element-template+cem`
- Schema: `https://cem.dev/ns/template/cem-element/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema.unknown_html_element`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-element-template/v1/examples/invalid-unknown-instruction.cem,contentType=application/vnd.cem.element-template+cem,schema=https://cem.dev/ns/template/cem-element/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{attribte @name=label | Default}
```
