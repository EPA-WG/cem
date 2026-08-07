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

Whole XPath expression attributes on XSLT instruction nodes are parsed once into
independently addressable `XPathExpressionAst` values owned by the
[`xpath/v1`](../../xpath/v1/README.md) package. Each expression retains its exact
attribute-value range, owning XML event and attribute identity, inherited
namespace bindings, and `xpath-default-namespace`. Entity-decoded expressions
use the generic XML AST's typed decoded-scalar-to-source map during that single
parse, so tokens, syntax, facts, and diagnostics keep original XML coordinates.
XPath diagnostics continue to be schema-owned by the XPath package. No JSON
projection, replacement XML tree, source reparse, or post-parse range rewrite
mediates this association.

Schema-owned attribute-value constraints classify values as literals, whole
XPath expressions, XSLT patterns, or attribute value templates. The current
selector catalog separates whole expressions and patterns from the complete
XSLT 3.0 instruction AVT matrix and classifies non-XSLT attributes on literal
result elements as AVTs. The instruction matrix contains the 59 attributes on
15 elements whose values are braced in the XSLT 3.0 element syntax summaries:

- `xsl:evaluate`: `base-uri`, `schema-aware`
- `xsl:element`: `name`, `namespace`
- `xsl:attribute`: `name`, `namespace`, `separator`
- `xsl:value-of`: `separator`
- `xsl:processing-instruction`: `name`
- `xsl:namespace`: `name`
- `xsl:number`: `format`, `lang`, `letter-value`, `ordinal`, `start-at`,
  `grouping-separator`, `grouping-size`
- `xsl:sort`: `lang`, `order`, `collation`, `stable`, `case-order`, `data-type`
- `xsl:for-each-group`: `collation`
- `xsl:merge-key`: `lang`, `order`, `collation`, `case-order`, `data-type`
- `xsl:analyze-string`: `regex`, `flags`
- `xsl:source-document`: `href`
- `xsl:message`: `terminate`, `error-code`
- `xsl:assert`: `error-code`
- `xsl:result-document`: `format`, `href`, `method`, `allow-duplicate-names`,
  `build-tree`, `byte-order-mark`, `cdata-section-elements`, `doctype-public`,
  `doctype-system`, `encoding`, `escape-uri-attributes`, `html-version`,
  `include-content-type`, `indent`, `item-separator`,
  `json-node-output-method`, `media-type`, `normalization-form`,
  `omit-xml-declaration`, `parameter-document`, `standalone`,
  `suppress-indentation`, `undeclare-prefixes`, `output-version`

Fixed lexical neighbors such as `xsl:output@method`,
`xsl:result-document@use-character-maps`, and XSLT control attributes on literal
result elements remain outside AVT parsing. Each selected AVT is segmented into
typed literal, expression, empty-expression, or error nodes.
Literal nodes retain their exact XML lexeme and their effective value after
doubled-brace expansion. Expression nodes directly own the single
`XPathExpressionAst` parse plus enclosure and expression-content ranges; a
native offset projector views the existing decoded XML attribute map so nested
XPath tokens and diagnostics remain in original XML coordinates. Empty or
comment-only expressions remain explicit zero-value nodes, and malformed braces
remain lossless error nodes bound to schema-owned diagnostics. Patterns stay
lexical typed facts until a package-owned pattern parser is registered. The
source lifecycle does not execute embedded expressions. Explicit
transform-template execution continues through the existing bounded XSLT parity
capability.

## Parser Facts And Diagnostics

The adapter emits neutral XML and XSLT facts for encoding, namespace prefix and
attribute errors, root and namespace identity, version syntax/support,
top-level declarations and template entrypoints, external URI access,
extension instructions/functions, literal result elements, XPath-bearing
attributes, AVT delimiter errors, DTD/entities, and source maps. Reportable facts
bind to diagnostics declared by `schema/xslt.cem` through `xslt-report-fact`
contracts.

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
examples, and README source-fence generation drift.

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
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-stylesheet</summary>

- Source: [`examples/basic-stylesheet.xsl`](./examples/basic-stylesheet.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/xslt+xml --to-schema https://cem.dev/ns/transform/xslt/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/">
    <main>
      <h1>Sign in</h1>
    </main>
  </xsl:template>
</xsl:stylesheet>
```

<details>
<summary>named-template</summary>

- Source: [`examples/named-template.xslt`](./examples/named-template.xslt)
- Content type: `text/xsl`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/named-template.xslt,contentType=text/xsl,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type text/xsl --to-schema https://cem.dev/ns/transform/xslt/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/">
    <section>default</section>
  </xsl:template>
  <xsl:template name="profile">
    <section class="profile">
      <p><xsl:value-of select="$label"/></p>
    </section>
  </xsl:template>
</xsl:stylesheet>
```

<details>
<summary>profile-semantics-characterization</summary>

- Source: [`examples/profile-semantics-characterization.xsl`](./examples/profile-semantics-characterization.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Expected diagnostics: `legacy_xslt.unsupported_construct`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/profile-semantics-characterization.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/xslt+xml --to-schema https://cem.dev/ns/transform/xslt/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!-- formatter characterization -->
<xsl:stylesheet
  xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:ui="urn:example:ui"
  xmlns:ext="urn:example:ext"
  extension-element-prefixes="ext"
  exclude-result-prefixes="ext"
  version="3.0">
  <xsl:mode name="profile"/>
  <xsl:param name="mode" select="'full'"/>
  <xsl:param name="label" select="'Profile'"/>
  <xsl:template match="/catalog/item[@active = true()]" mode="profile">
    <ui:card class="item-{@id}" data-label="{$label}">
      <xsl:if test="@visible and $mode = 'full'">
        <xsl:text>  fixed text  </xsl:text>
        <![CDATA[foreign <text> & exact]]>
        <xsl:value-of select="normalize-space(title)"/>
        <ext:widget ext:mode="{$mode}">literal extension text</ext:widget>
      </xsl:if>
    </ui:card>
  </xsl:template>
</xsl:stylesheet>
```

<details>
<summary>legacy-custom-element-stylesheet</summary>

- Source: [`examples/legacy-custom-element-stylesheet.xsl`](./examples/legacy-custom-element-stylesheet.xsl)
- Content type: `custom-element-xslt`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-stylesheet.xsl,contentType=custom-element-xslt,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/">
    <article>
      <xsl:if test="$ready">
        <button>Continue</button>
      </xsl:if>
    </article>
  </xsl:template>
</xsl:stylesheet>
```

<details>
<summary>legacy-custom-element-fragment</summary>

- Source: [`examples/legacy-custom-element-fragment.html`](./examples/legacy-custom-element-fragment.html)
- Content type: `custom-element-xslt`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- README rendering: fenced `html` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-fragment.html,contentType=custom-element-xslt,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```html
<article>
  <button>Continue</button>
</article>
```

<details>
<summary>unsupported-extension-warning</summary>

- Source: [`examples/unsupported-extension-warning.xsl`](./examples/unsupported-extension-warning.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `pass`
- Expected diagnostics: `legacy_xslt.unsupported_construct`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xslt/v1/examples/unsupported-extension-warning.xsl,contentType=application/xslt+xml,schema=https://cem.dev/ns/transform/xslt/1 \
  --to-content-type application/xslt+xml --to-schema https://cem.dev/ns/transform/xslt/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet
  xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:msxsl="urn:schemas-microsoft-com:xslt"
  version="1.0">
  <xsl:template match="/">
    <msxsl:script language="JScript">function run(){return 1;}</msxsl:script>
  </xsl:template>
</xsl:stylesheet>
```

<details>
<summary>invalid-missing-namespace</summary>

- Source: [`examples/invalid-missing-namespace.xsl`](./examples/invalid-missing-namespace.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.namespace_missing`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-namespace.xsl
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<stylesheet version="1.0">
  <template match="/">
    <main/>
  </template>
</stylesheet>
```

<details>
<summary>invalid-missing-version</summary>

- Source: [`examples/invalid-missing-version.xsl`](./examples/invalid-missing-version.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.version_missing`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-version.xsl
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform">
  <xsl:template match="/">
    <main/>
  </xsl:template>
</xsl:stylesheet>
```

<details>
<summary>invalid-external-include</summary>

- Source: [`examples/invalid-external-include.xsl`](./examples/invalid-external-include.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.external_uri_rejected`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-external-include.xsl
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:include href="shared/base.xsl"/>
  <xsl:template match="/">
    <main/>
  </xsl:template>
</xsl:stylesheet>
```

<details>
<summary>invalid-missing-entrypoint</summary>

- Source: [`examples/invalid-missing-entrypoint.xsl`](./examples/invalid-missing-entrypoint.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.entrypoint_missing`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-entrypoint.xsl
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:output method="html"/>
</xsl:stylesheet>
```

<details>
<summary>invalid-not-well-formed</summary>

- Source: [`examples/invalid-not-well-formed.xsl`](./examples/invalid-not-well-formed.xsl)
- Content type: `application/xslt+xml`
- Schema: `https://cem.dev/ns/transform/xslt/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xslt.not_well_formed_xml`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xslt+xml --schema https://cem.dev/ns/transform/xslt/1 \
  packages/cem_ml/schema-packages/xslt/v1/examples/invalid-not-well-formed.xsl
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:template match="/">
    <main>
  </xsl:template>
</xsl:stylesheet>
```
