# XSLT Schema Package v1

This package owns standard standalone XSLT stylesheet identity, typed source
validation, lexical output profiles, and the bounded custom-element XSLT
compatibility boundary.

## Owned Identities

- Schema URI: `https://cem.dev/ns/transform/xslt/1`
- Primary media type: `application/xslt+xml`
- Standard alias: `text/xsl`
- Document namespace: `http://www.w3.org/1999/XSL/Transform`
- Compatibility aliases: `custom-element-xslt`,
  `text/custom-element-xslt`, `application/custom-element-xslt`, and
  `text/x-custom-element-xslt`

Standard content type, package schema, and namespace identities select the
dedicated `xslt` lifecycle adapter. Only the four compatibility aliases select
`custom-element-xslt-compat`; that adapter accepts legacy fragments and lowers
its bounded dialect to CEM. Standard XSLT is never silently reinterpreted as a
legacy fragment.

## Resource Model

`XsltStylesheetAst` wraps the generic XML event model while preserving the
actual media type and MIME parameters, XML declaration and doctype lexemes,
qualified element and attribute names, namespace bindings, stylesheet version,
top-level declarations and templates, literal result elements, XPath-bearing
attribute text, source ranges, source maps, and source line ending.

XPath and attribute value templates remain lexical source values. The source
lifecycle does not parse or execute them. Explicit transform-template execution
continues through the existing bounded XSLT parity capability.

## Parser Facts And Diagnostics

The adapter emits neutral XML and XSLT facts for encoding, namespace prefix and
attribute errors, root and namespace identity, version syntax/support,
top-level declarations and template entrypoints, external URI access,
extension instructions/functions, literal result elements, XPath-bearing
attributes, DTD/entities, and source maps. Reportable facts bind to diagnostics
declared by `schema/xslt.cem` through `xslt-report-fact` contracts.

## Output Artifacts

The package provides executable compact, pretty, and tabular formatter wrappers
plus terminal, HTML, and Markdown colorizer wrappers. Private helpers project
the typed stylesheet event stream into a CEM tree with source maps. Current
profiles preserve lexical XML, namespace/version attributes, XPath text,
literal result content, source line endings, and the default final newline;
stylesheet-aware reflow is not yet defined.

## Resolver And Entity Safety

- `xsl:include`, `xsl:import`, non-local `xsl:result-document`, and
  `document()` references require an explicit resolver policy.
- Extension instructions and functions require a registered capability.
- DTD declarations and non-built-in entities inherit the reject-only XML safety
  policy.
- Browser `XSLTProcessor` delegation is prohibited. Native lifecycle and
  explicit transform-template capabilities own execution.

## Validation

Validate a standard stylesheet through the CLI:

```bash
cem-ml validate --format json \
  --content-type application/xslt+xml \
  --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl
```

Standard validation requires an `xsl:stylesheet` or `xsl:transform` root in the
XSLT namespace, a supported version, and at least one top-level template.

## Verification

```bash
yarn nx run cem_ml_schema_package_xslt_v1:verify
yarn nx run cem_ml_schema_package_xslt_v1:samples2readme
yarn nx run cem_ml:test:schema-package-structure
yarn nx run cem_ml:test:cli-schema-artifacts
```

The package verify target covers typed parsing and schema facts, standard and
legacy lifecycle ownership, both standard media types, same-schema output,
formatter/colorizer execution, CLI behavior, parity compatibility, schema-owned
examples, and preview drift.

## Release Behavior

- Standard stylesheet conversion is lossless lexical XML output with package
  CEMT profiles and a final newline.
- Legacy aliases retain bounded lowering and generated-source-map behavior.
- Embedded or explicitly invoked transformation behavior is not inferred from a
  standalone stylesheet resource.
- Cross-schema conversion still requires a registered converter.

## Tracked Incomplete Work

This package is not a general XSLT 3.0 processor. Streaming execution, package
composition, schema-aware XPath typing, dynamic evaluation, and unrestricted
extension capabilities remain outside this package. Browser-native XSLT
execution is intentionally excluded rather than deferred.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Each SVG previews the rendered example
content or validation diagnostics for expected-fail examples. The target writes a
preformatted HTML preview to
`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,
then renders the `<pre>` spans through headless Chromium into
`examples/previews/<example-file>.svg`.

<details>
<summary>basic-stylesheet</summary>

- Source: [`examples/basic-stylesheet.xsl`](./examples/basic-stylesheet.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/xslt+xml --to-schema https://cem.dev/ns/transform/xslt/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of XSLT Schema Package v1 basic-stylesheet example](examples/previews/basic-stylesheet.xsl.svg)

<details>
<summary>named-template</summary>

- Source: [`examples/named-template.xslt`](./examples/named-template.xslt)
- Content type: `text/xsl`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/named-template.xslt.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/named-template.xslt,contentType=text/xsl,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type text/xsl --to-schema https://cem.dev/ns/transform/xslt/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of XSLT Schema Package v1 named-template example](examples/previews/named-template.xslt.svg)

<details>
<summary>legacy-custom-element-stylesheet</summary>

- Source: [`examples/legacy-custom-element-stylesheet.xsl`](./examples/legacy-custom-element-stylesheet.xsl)
- Content type: `custom-element-xslt`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-stylesheet.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-stylesheet.xsl,contentType=custom-element-xslt,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

![Preview of XSLT Schema Package v1 legacy-custom-element-stylesheet example](examples/previews/legacy-custom-element-stylesheet.xsl.svg)

<details>
<summary>legacy-custom-element-fragment</summary>

- Source: [`examples/legacy-custom-element-fragment.html`](./examples/legacy-custom-element-fragment.html)
- Content type: `custom-element-xslt`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-fragment.html.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-fragment.html,contentType=custom-element-xslt,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

![Preview of XSLT Schema Package v1 legacy-custom-element-fragment example](examples/previews/legacy-custom-element-fragment.html.svg)

<details>
<summary>unsupported-extension-warning</summary>

- Source: [`examples/unsupported-extension-warning.xsl`](./examples/unsupported-extension-warning.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Expected diagnostics: `legacy_xslt.unsupported_construct`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/unsupported-extension-warning.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/unsupported-extension-warning.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/xslt+xml --to-schema https://cem.dev/ns/transform/xslt/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of XSLT Schema Package v1 unsupported-extension-warning example](examples/previews/unsupported-extension-warning.xsl.svg)

<details>
<summary>invalid-missing-namespace</summary>

- Source: [`examples/invalid-missing-namespace.xsl`](./examples/invalid-missing-namespace.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.namespace_missing`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-namespace.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-namespace.xsl
```

</details>

![Preview of XSLT Schema Package v1 invalid-missing-namespace example](examples/previews/invalid-missing-namespace.xsl.svg)

<details>
<summary>invalid-missing-version</summary>

- Source: [`examples/invalid-missing-version.xsl`](./examples/invalid-missing-version.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.version_missing`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-version.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-version.xsl
```

</details>

![Preview of XSLT Schema Package v1 invalid-missing-version example](examples/previews/invalid-missing-version.xsl.svg)

<details>
<summary>invalid-external-include</summary>

- Source: [`examples/invalid-external-include.xsl`](./examples/invalid-external-include.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.external_uri_rejected`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-external-include.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-external-include.xsl
```

</details>

![Preview of XSLT Schema Package v1 invalid-external-include example](examples/previews/invalid-external-include.xsl.svg)

<details>
<summary>invalid-missing-entrypoint</summary>

- Source: [`examples/invalid-missing-entrypoint.xsl`](./examples/invalid-missing-entrypoint.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.entrypoint_missing`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-entrypoint.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-entrypoint.xsl
```

</details>

![Preview of XSLT Schema Package v1 invalid-missing-entrypoint example](examples/previews/invalid-missing-entrypoint.xsl.svg)

<details>
<summary>invalid-not-well-formed</summary>

- Source: [`examples/invalid-not-well-formed.xsl`](./examples/invalid-not-well-formed.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.not_well_formed_xml`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-not-well-formed.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-not-well-formed.xsl
```

</details>

![Preview of XSLT Schema Package v1 invalid-not-well-formed example](examples/previews/invalid-not-well-formed.xsl.svg)
