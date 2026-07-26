# XSLT schema package v1

This package defines CEM schema identity for XSLT stylesheet resources and for the legacy custom-element XSLT compatibility markers.

- Schema URI: `https://cem.dev/ns/transform/xslt/1`
- Primary content type: `application/xslt+xml`
- Alias content types: `text/xsl`, `custom-element-xslt`, `text/custom-element-xslt`, `application/custom-element-xslt`, `text/x-custom-element-xslt`
- Document namespace: `http://www.w3.org/1999/XSL/Transform`
- Source schema: `schema/xslt.cem`

XSLT is XML-backed. Direct validation routes the XSLT media type and
compatibility aliases through this schema package and reuses the XML event
reader as the parser. The package registers the stylesheet identity and the
compatibility aliases consumed by the existing CEM legacy custom-element
adapter.

The current executable support is intentionally bounded: copied custom-element and XSLT parity templates can lower through the CEM-owned compatibility path, while full XSLT 3.0/4.0 execution remains capability-gated roadmap work. Browser-native `XSLTProcessor` execution is not part of this package contract.

## Validation

Validate XSLT stylesheet resources through the CLI with the schema URI and
content type:

```bash
cem-ml validate --format json \
  --content-type application/xslt+xml \
  --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl
```

The direct validator parses standard XSLT as XML, requires an `xsl:stylesheet`
or `xsl:transform` root in the XSLT namespace, validates the root `version`
attribute, requires at least one top-level `xsl:template`, rejects external URI
constructs such as `xsl:include`, and reports warning diagnostics for extension
constructs outside the bounded legacy compatibility profile. The
`custom-element-xslt` aliases also accept legacy custom-element fragment
resources when the source is not a stylesheet root.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Each SVG previews the example content, not
the validation report. The target writes a preformatted HTML preview to
`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,
then renders the `<pre>` spans through headless Chromium into
`examples/previews/<example-file>.svg`.
Source snapshots are used only where the current CLI cannot yet render
the package formatter/colorizer path for that content identity.

### basic-stylesheet

- Source: [`examples/basic-stylesheet.xsl`](examples/basic-stylesheet.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --from-format xml --to-content-type application/xslt+xml --to-schema \
  https://cem.dev/ns/transform/xslt/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

![Preview of XSLT schema package v1 basic-stylesheet example](examples/previews/basic-stylesheet.xsl.svg)

### named-template

- Source: [`examples/named-template.xslt`](examples/named-template.xslt)
- Content type: `text/xsl`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/named-template.xslt.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/named-template.xslt,contentType=text/xsl,schema=https://cem.dev/ns/transform/xslt/1 \
  --from-format xml --to-content-type text/xsl --to-schema \
  https://cem.dev/ns/transform/xslt/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

![Preview of XSLT schema package v1 named-template example](examples/previews/named-template.xslt.svg)

### legacy-custom-element-stylesheet

- Source: [`examples/legacy-custom-element-stylesheet.xsl`](examples/legacy-custom-element-stylesheet.xsl)
- Content type: `custom-element-xslt`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Preview renderer: `source snapshot HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-stylesheet.xsl.html`
![Preview of XSLT schema package v1 legacy-custom-element-stylesheet example](examples/previews/legacy-custom-element-stylesheet.xsl.svg)

### legacy-custom-element-fragment

- Source: [`examples/legacy-custom-element-fragment.html`](examples/legacy-custom-element-fragment.html)
- Content type: `custom-element-xslt`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Preview renderer: `source snapshot HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-fragment.html.html`
![Preview of XSLT schema package v1 legacy-custom-element-fragment example](examples/previews/legacy-custom-element-fragment.html.svg)

### unsupported-extension-warning

- Source: [`examples/unsupported-extension-warning.xsl`](examples/unsupported-extension-warning.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Expected diagnostics: `legacy_xslt.unsupported_construct`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/unsupported-extension-warning.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/unsupported-extension-warning.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --from-format xml --to-content-type application/xslt+xml --to-schema \
  https://cem.dev/ns/transform/xslt/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

![Preview of XSLT schema package v1 unsupported-extension-warning example](examples/previews/unsupported-extension-warning.xsl.svg)

### invalid-missing-namespace

- Source: [`examples/invalid-missing-namespace.xsl`](examples/invalid-missing-namespace.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.namespace_missing`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-namespace.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-namespace.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --from-format xml --to-content-type application/xslt+xml --to-schema \
  https://cem.dev/ns/transform/xslt/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

![Preview of XSLT schema package v1 invalid-missing-namespace example](examples/previews/invalid-missing-namespace.xsl.svg)

### invalid-missing-version

- Source: [`examples/invalid-missing-version.xsl`](examples/invalid-missing-version.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.version_missing`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-version.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-version.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --from-format xml --to-content-type application/xslt+xml --to-schema \
  https://cem.dev/ns/transform/xslt/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

![Preview of XSLT schema package v1 invalid-missing-version example](examples/previews/invalid-missing-version.xsl.svg)

### invalid-external-include

- Source: [`examples/invalid-external-include.xsl`](examples/invalid-external-include.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.external_uri_rejected`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-external-include.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/invalid-external-include.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --from-format xml --to-content-type application/xslt+xml --to-schema \
  https://cem.dev/ns/transform/xslt/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

![Preview of XSLT schema package v1 invalid-external-include example](examples/previews/invalid-external-include.xsl.svg)

### invalid-missing-entrypoint

- Source: [`examples/invalid-missing-entrypoint.xsl`](examples/invalid-missing-entrypoint.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.entrypoint_missing`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-entrypoint.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-entrypoint.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --from-format xml --to-content-type application/xslt+xml --to-schema \
  https://cem.dev/ns/transform/xslt/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

![Preview of XSLT schema package v1 invalid-missing-entrypoint example](examples/previews/invalid-missing-entrypoint.xsl.svg)

### invalid-not-well-formed

- Source: [`examples/invalid-not-well-formed.xsl`](examples/invalid-not-well-formed.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.not_well_formed_xml`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xslt/v1/examples/invalid-not-well-formed.xsl.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/invalid-not-well-formed.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --from-format xml --to-content-type application/xslt+xml --to-schema \
  https://cem.dev/ns/transform/xslt/1 --cemt-formatter-profile tabular \
  --cemt-color-profile terminal --output-color-type ansi-256
```

![Preview of XSLT schema package v1 invalid-not-well-formed example](examples/previews/invalid-not-well-formed.xsl.svg)
