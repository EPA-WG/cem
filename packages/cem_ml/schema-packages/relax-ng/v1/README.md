# RELAX NG Schema Package

This package defines registry identity for RELAX NG schema resources.

RELAX NG schema source is not CEM-ML syntax. The schema package and manifest are
authored in CEM-ML, but `.rng` resources are parsed as XML syntax and `.rnc`
resources are parsed as compact syntax text.

## Owned Identities

- Schema URI: `https://cem.dev/ns/data/relax-ng/1`
- Primary content type: `application/relax-ng+xml`
- Alias content types: `application/relax-ng-compact-syntax`
- Preferred extensions: `.rng`, `.rnc`
- XML syntax namespace: `http://relaxng.org/ns/structure/1.0`

## Resource Model

The schema describes RELAX NG resources as validation schema inputs:

- XML syntax resources preserve RELAX NG namespace identity, grammar root,
  start pattern, defines, patterns, attributes, and source offsets;
- compact syntax resources preserve namespace declarations, start definition,
  pattern definitions, operators, string literals, and source offsets;
- include and external reference declarations remain explicit, but are rejected
  unless an explicit resolver policy enables them.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Each SVG previews the example content, not
the validation report. The target writes a preformatted HTML preview to
`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,
then renders the `<pre>` spans through headless Chromium into
`examples/previews/<example-file>.svg`.
Source snapshots are used only where the current CLI cannot yet render
the package formatter/colorizer path for that content identity.

<details>
<summary>basic-schema-xml</summary>

- Source: [`examples/basic-schema.rng`](./examples/basic-schema.rng)
- Content type: `application/relax-ng+xml`
- Schema: `https://cem.dev/ns/data/relax-ng/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/relax-ng/v1/examples/basic-schema.rng.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/relax-ng/v1/examples/basic-schema.rng,contentType=application/relax-ng+xml,schema=https://cem.dev/ns/data/relax-ng/1 \
  --from-format xml --to-content-type application/relax-ng+xml --to-schema \
  https://cem.dev/ns/data/relax-ng/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

</details>

![Preview of RELAX NG Schema Package basic-schema-xml example](examples/previews/basic-schema.rng.svg)

<details>
<summary>datatype-schema</summary>

- Source: [`examples/datatype-schema.rng`](./examples/datatype-schema.rng)
- Content type: `application/relax-ng+xml`
- Schema: `https://cem.dev/ns/data/relax-ng/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/relax-ng/v1/examples/datatype-schema.rng.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/relax-ng/v1/examples/datatype-schema.rng,contentType=application/relax-ng+xml,schema=https://cem.dev/ns/data/relax-ng/1 \
  --from-format xml --to-content-type application/relax-ng+xml --to-schema \
  https://cem.dev/ns/data/relax-ng/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

</details>

![Preview of RELAX NG Schema Package datatype-schema example](examples/previews/datatype-schema.rng.svg)

<details>
<summary>basic-schema-compact</summary>

- Source: [`examples/basic-schema.rnc`](./examples/basic-schema.rnc)
- Content type: `application/relax-ng-compact-syntax`
- Schema: `https://cem.dev/ns/data/relax-ng/1`
- Expected result: `pass`
- Preview renderer: `source snapshot HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/relax-ng/v1/examples/basic-schema.rnc.html`

</details>

![Preview of RELAX NG Schema Package basic-schema-compact example](examples/previews/basic-schema.rnc.svg)

<details>
<summary>invalid-missing-start</summary>

- Source: [`examples/invalid-missing-start.rng`](./examples/invalid-missing-start.rng)
- Content type: `application/relax-ng+xml`
- Schema: `https://cem.dev/ns/data/relax-ng/1`
- Expected result: `fail`
- Expected diagnostics: `cem.relax_ng.missing_start`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/relax-ng/v1/examples/invalid-missing-start.rng.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/relax-ng/v1/examples/invalid-missing-start.rng,contentType=application/relax-ng+xml,schema=https://cem.dev/ns/data/relax-ng/1 \
  --from-format xml --to-content-type application/relax-ng+xml --to-schema \
  https://cem.dev/ns/data/relax-ng/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

</details>

![Preview of RELAX NG Schema Package invalid-missing-start example](examples/previews/invalid-missing-start.rng.svg)

<details>
<summary>invalid-unknown-element</summary>

- Source: [`examples/invalid-unknown-element.rng`](./examples/invalid-unknown-element.rng)
- Content type: `application/relax-ng+xml`
- Schema: `https://cem.dev/ns/data/relax-ng/1`
- Expected result: `fail`
- Expected diagnostics: `cem.relax_ng.unknown_element`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/relax-ng/v1/examples/invalid-unknown-element.rng.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/relax-ng/v1/examples/invalid-unknown-element.rng,contentType=application/relax-ng+xml,schema=https://cem.dev/ns/data/relax-ng/1 \
  --from-format xml --to-content-type application/relax-ng+xml --to-schema \
  https://cem.dev/ns/data/relax-ng/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

</details>

![Preview of RELAX NG Schema Package invalid-unknown-element example](examples/previews/invalid-unknown-element.rng.svg)

<details>
<summary>invalid-unclosed-compact</summary>

- Source: [`examples/invalid-unclosed-compact.rnc`](./examples/invalid-unclosed-compact.rnc)
- Content type: `application/relax-ng-compact-syntax`
- Schema: `https://cem.dev/ns/data/relax-ng/1`
- Expected result: `fail`
- Expected diagnostics: `cem.relax_ng.compact_parse_error`
- Preview renderer: `source snapshot HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/relax-ng/v1/examples/invalid-unclosed-compact.rnc.html`

</details>

![Preview of RELAX NG Schema Package invalid-unclosed-compact example](examples/previews/invalid-unclosed-compact.rnc.svg)
