# XHTML Resource Schema Package

Status: schema, typed XML lifecycle input/output adapter, examples, formatter,
and colorizer

This package defines registry identity and executable source handling for XHTML
resources.

XHTML source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `application/xhtml+xml` content type
are parsed as XML and validated as HTML vocabulary in the XHTML namespace.

## Owned Identities

- Schema URI: `https://cem.dev/ns/data/xhtml/1`
- Primary content type: `application/xhtml+xml`
- Document namespace: `http://www.w3.org/1999/xhtml`
- Preferred extensions: `.xhtml`, `.xht`

RFC 3236 registers `application/xhtml+xml` with optional `charset` and
`profile` parameters. IANA now records `profile` as deprecated because XHTML
profiles were obsoleted in HTML5.

## Resource Model

The schema describes XHTML resources as an XML-serialized HTML document model:

- `XhtmlDocumentAst` reuses the generic typed XML event stream while preserving
  XHTML content type/schema identity, URI, MIME parameters, byte length, line
  ending, encoding report, and source-map stacks;
- XML declarations, doctypes, qualified names, namespace declarations,
  attributes, text, CDATA, comments, processing instructions, entity
  references, foreign-content boundaries, and source ranges remain lexical;
- the root element must be `html` in the XHTML namespace;
- `head` and `body` structure is explicit and ordered;
- metadata, flow, phrasing, and interactive content are modeled as XHTML
  vocabulary facets over XML elements;
- foreign SVG, MathML, and other namespace content is preserved without HTML
  tokenizer recovery or void-element rewriting;
- `text/html` remains a separate HTML serialization identity and is not claimed
  by this package.

## Parser Facts And Diagnostics

The adapter emits neutral facts for XML parse and encoding errors, namespace
binding and attribute uniqueness, root and XHTML namespace identity, ordered
`head`/`body` structure, deprecated MIME profile parameters, DTD/entity safety,
source-map availability, observed doctypes, and foreign-content namespaces.
Constraints in `schema/xhtml.cem` bind reportable facts to package-owned
diagnostic codes through `xhtml-report-fact`; observed structure and foreign
content remain non-diagnostic facts.

The generic XML parser owns well-formedness, encoding, qualified-name,
namespace, duplicate-attribute, DTD, entity, and source-range mechanics. XHTML
owns the `html` root, XHTML namespace, `head`/`body` ordering, MIME profile, and
XML serialization constraints. The current validator does not perform complete
HTML vocabulary or content-model validation beyond those schema bindings.

## Output Artifacts

The package owns `compact`, `pretty`, and `tabular` formatter wrappers plus
`terminal`, `html`, and `md` colorizer wrappers. The formatter consumes an
`xhtml-document` and emits a package-owned CEM tree; the colorizer consumes that
tree before the shared text writer.

All formatter profiles currently preserve source lexemes. Their metadata
records a `lexical-lossless-*` layout decision, while whitespace reflow remains
deferred until XHTML mixed-content semantics can be preserved. Same-schema
output retains XML declaration, empty-element spelling, qualified names,
foreign content, detected line endings, and appends one final newline when
absent.

## Resolver And Entity Safety

The lifecycle registry selects the `xhtml` adapter only for
`application/xhtml+xml` or `https://cem.dev/ns/data/xhtml/1`; XHTML input does
not fall through to HTML, generic XML, or CEM parsing. Same-schema output uses
XML serialization and package-owned XHTML CEMT assets, never HTML tokenizer
recovery or HTML void-element rules.

The package inherits the generic XML reject-only DTD and non-built-in entity
policy. Doctype and entity lexemes remain in the typed event stream for
diagnostics and tooling, but no external subset, filesystem resource, or
network entity is resolved. Foreign namespace events are preserved; semantic
validation by SVG, MathML, or another package requires an explicit converter or
validation contract.

## Verification

`yarn nx run cem_ml_schema_package_xhtml_v1:verify` runs manifest validation,
complete example indexing, schema-derived fact tests, dedicated lifecycle
load/export coverage, exact same-schema engine and CLI conversion, executable
formatter/colorizer profile tests, schema-owned CLI example validation, and
README/SVG preview drift checks with no source fallback.

## Release Behavior

XHTML input is parsed once into `XhtmlDocumentAst` through the generic XML event
model and validated from XHTML schema fact bindings. Same-schema conversion
executes the package formatter, optional colorizer, and XML text writer; output
metadata identifies `xhtml-lifecycle-output` and
`xhtml-ast-stream-to-xhtml-output-pipeline`. Cross-schema conversion requires
an explicit registered converter path.

## Tracked Incomplete Work

- Add complete XHTML vocabulary and content-model validation independently of
  XML well-formedness.
- Define mixed-content-aware whitespace/reflow semantics before formatter
  profiles alter source lexemes.
- Add explicit bounded DTD/entity resolution policy before allowing any
  external resource access.
- Bind foreign SVG and MathML validation through explicit schema-package
  composition without changing lexical XHTML serialization.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Each SVG previews the rendered example
content or validation diagnostics for expected-fail examples. The target writes a
preformatted HTML preview to
`dist/cem_ml/schema-packages/<package>/v1/examples/<example-file>.html`,
then renders the `<pre>` spans through headless Chromium into
`examples/previews/<example-file>.svg`.

<details>
<summary>basic-document</summary>

- Source: [`examples/basic-document.xhtml`](./examples/basic-document.xhtml)
- Content type: `application/xhtml+xml`
- Schema: `https://cem.dev/ns/data/xhtml/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xhtml/v1/examples/basic-document.xhtml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xhtml/v1/examples/basic-document.xhtml,contentType=application/xhtml+xml,schema=https://cem.dev/ns/data/xhtml/1 \
  --to-content-type application/xhtml+xml --to-schema https://cem.dev/ns/data/xhtml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of XHTML Resource Schema Package basic-document example](examples/previews/basic-document.xhtml.svg)

<details>
<summary>form-page</summary>

- Source: [`examples/form-page.xhtml`](./examples/form-page.xhtml)
- Content type: `application/xhtml+xml`
- Schema: `https://cem.dev/ns/data/xhtml/1`
- Expected result: `pass`
- Preview renderer: `CLI convert, tabular formatter, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xhtml/v1/examples/form-page.xhtml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xhtml/v1/examples/form-page.xhtml,contentType=application/xhtml+xml,schema=https://cem.dev/ns/data/xhtml/1 \
  --to-content-type application/xhtml+xml --to-schema https://cem.dev/ns/data/xhtml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile html
```

</details>

![Preview of XHTML Resource Schema Package form-page example](examples/previews/form-page.xhtml.svg)

<details>
<summary>invalid-missing-namespace</summary>

- Source: [`examples/invalid-missing-namespace.xhtml`](./examples/invalid-missing-namespace.xhtml)
- Content type: `application/xhtml+xml`
- Schema: `https://cem.dev/ns/data/xhtml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xhtml.namespace_missing`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xhtml/v1/examples/invalid-missing-namespace.xhtml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xhtml+xml --schema https://cem.dev/ns/data/xhtml/1 \
  packages/cem_ml/schema-packages/xhtml/v1/examples/invalid-missing-namespace.xhtml
```

</details>

![Preview of XHTML Resource Schema Package invalid-missing-namespace example](examples/previews/invalid-missing-namespace.xhtml.svg)

<details>
<summary>invalid-body-before-head</summary>

- Source: [`examples/invalid-body-before-head.xhtml`](./examples/invalid-body-before-head.xhtml)
- Content type: `application/xhtml+xml`
- Schema: `https://cem.dev/ns/data/xhtml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xhtml.head_body_order`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xhtml/v1/examples/invalid-body-before-head.xhtml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xhtml+xml --schema https://cem.dev/ns/data/xhtml/1 \
  packages/cem_ml/schema-packages/xhtml/v1/examples/invalid-body-before-head.xhtml
```

</details>

![Preview of XHTML Resource Schema Package invalid-body-before-head example](examples/previews/invalid-body-before-head.xhtml.svg)

<details>
<summary>invalid-not-well-formed</summary>

- Source: [`examples/invalid-not-well-formed.xhtml`](./examples/invalid-not-well-formed.xhtml)
- Content type: `application/xhtml+xml`
- Schema: `https://cem.dev/ns/data/xhtml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xhtml.not_well_formed_xml`
- Preview renderer: `CLI validate, JSON report, preview HTML + html2svg`
- Preview HTML: `dist/cem_ml/schema-packages/xhtml/v1/examples/invalid-not-well-formed.xhtml.html`

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xhtml+xml --schema https://cem.dev/ns/data/xhtml/1 \
  packages/cem_ml/schema-packages/xhtml/v1/examples/invalid-not-well-formed.xhtml
```

</details>

![Preview of XHTML Resource Schema Package invalid-not-well-formed example](examples/previews/invalid-not-well-formed.xhtml.svg)
