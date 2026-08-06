# XML Resource Schema Package

Status: schema, typed lifecycle input/output adapter, examples, formatter,
colorizer, and DOM projection converter

This package defines registry identity for generic XML resources.

XML source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with XML content types are parsed by an XML
parser or adapter.

## Owned Identities

- Schema URI: `https://cem.dev/ns/data/xml/1`
- Primary content type: `application/xml`
- Alias content types: `text/xml`, `application/xml-external-parsed-entity`,
  `text/xml-external-parsed-entity`, `application/xml-dtd`
- Preferred extension: `.xml`

RFC 7303 standardizes the generic XML media types and the `+xml` structured
syntax suffix. This package maps only the generic media types above to the XML
schema URI. It does not claim arbitrary `+xml` types.

## Resource Model

The schema describes XML resources as a namespace-aware document model:

- documents preserve XML declaration, MIME charset, XML version, standalone
  state, root element, optional doctype, and source identity;
- elements and attributes preserve qualified names, expanded namespace identity,
  lexical order, and source offsets when available; each mapped attribute also
  retains its exact raw value range, built-in/numeric entity-decoded value, and
  one monotonic original-source span per decoded UTF-8 scalar plus exact
  decoded/source boundary positions;
- text, CDATA, comments, processing instructions, and entity references remain
  explicit nodes;
- DTD and external entity material is preserved as declarations but external
  resolution is rejected unless an explicit policy enables it;
- external parsed entities and XML DTD resources use the same schema identity
  with specialized top-level resource elements.

This package intentionally does not claim all media types ending in `+xml`.
Domain formats such as XHTML, SVG, MathML, XSLT, Atom, and RSS need their own
schema packages that can depend on the generic XML schema.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

All three formatter profiles currently preserve source event lexemes and
therefore intentionally produce the same XML bytes. Their CEM-tree metadata
distinguishes `lexical-lossless-compact`, `lexical-lossless-pretty`, and
`lexical-lossless-tabular` layout decisions. XML whitespace can be application
data, so readable reflow is not enabled without an XML-aware whitespace policy.
`compact` is the deterministic package default; it is not a claim of W3C XML
Canonicalization. The text writer preserves the selected source line ending
and appends one final newline at the document boundary when it is absent.

The colorizers consume the formatted CEM tree. `terminal` emits terminal role
styles, `html` emits class-based span metadata, and `md` emits Markdown-safe
span metadata. Color output does not bypass the formatter boundary.

## Source Identity And Parser Facts

The XML lifecycle adapter records the source URI, full content type, media-type
essence, content-type parameters, source byte length, detected line ending, MIME
charset, XML declaration encoding, and decoder status. Its typed event stream
preserves declaration, start/empty/end element, text, CDATA, comment,
processing-instruction, doctype, and entity-reference lexemes. Element events
carry qualified/local names, prefixes, resolved namespace URIs, attributes,
depth, byte ranges, line/column coordinates, and source-map stacks. Attribute
values remain lexically lossless while a parallel typed value and scalar map
decode only built-in and numeric references. Unresolved references fail closed:
the raw value and range remain available, but no decoded value is synthesized.
The typed map projects only scalar-aligned decoded ranges and zero-length
boundaries; interior UTF-8 bytes and out-of-bounds positions fail closed.

The parser emits neutral facts. Constraints in `schema/xml.cem` bind those facts
to package-owned diagnostic codes and severities for parse errors, unsupported
or conflicting encodings, unbound prefixes, duplicate expanded attribute
names, rejected DTD/external entities, entity expansion limits, and unavailable
source maps. The lifecycle adapter projects those schema bindings; callers do
not reinterpret XML parser errors as CEM-ML syntax diagnostics.

## Resolver And Entity Safety

Generic XML identities select the typed XML adapter only for this package's
schema URI or generic XML media types. SVG and MathML continue through their
specialized package identities. Same-schema output resolves the package's
profile wrapper and private helper assets before the common CEM-tree writer.

DTD declarations may be captured as events for diagnostics, but DTD-bearing
documents are rejected by the current policy. External subsets are never
fetched, undeclared entity references are rejected, and no filesystem or
network entity resolver is available. Namespace prefixes must be in scope and
attributes must be unique by expanded namespace URI plus local name.

## Verification

`yarn nx run cem_ml_schema_package_xml_v1:verify` runs:

- schema-package manifest validation and complete manifest example indexing;
- schema-owned CLI validation for every declared example and diagnostic code;
- typed lifecycle input/export, engine validation, and same-schema conversion
  regressions that prove XML does not fall through to the CEM or HTML parser;
- executable formatter/colorizer catalog and profile coverage across compact,
  pretty, tabular, terminal, HTML, and Markdown profiles;
- package-local README source-fence generation checks with no SVG fallback.

## Release Behavior

Generic XML input is decoded as supported UTF-8, parsed into the typed XML event
AST, and validated from schema-owned parser-fact bindings. Same-schema XML
conversion consumes that AST and executes the package CEMT formatter,
colorizer, and writer pipeline. XML declarations, namespace spellings,
attributes, comments, CDATA, processing instructions, and source ranges are
preserved through the formatter boundary. The DOM projection converter remains
the explicit route from generic XML to the CEM DOM projection schema.

## Tracked Incomplete Work

- Add opt-in decoders for supported non-UTF-8 XML encodings; the current parser
  reports unsupported input instead of transcoding it.
- Define an XML-aware whitespace/reflow policy before making `pretty` or
  `tabular` alter lexical content.
- Add a separately named/versioned XML canonicalization profile if W3C C14N is
  required; `compact` must not silently acquire C14N semantics.
- Add an explicit resolver policy before permitting DTD or external entity
  resolution. The current release remains reject-only.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-document</summary>

- Source: [`examples/basic-document.xml`](./examples/basic-document.xml)
- Content type: `application/xml`
- Schema: `https://cem.dev/ns/data/xml/1`
- Expected result: `pass`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/xml/v1/examples/basic-document.xml,contentType=application/xml,schema=https://cem.dev/ns/data/xml/1 \
  --from-format xml --to-content-type application/xml --to-schema \
  https://cem.dev/ns/data/xml/1 --cemt-formatter-profile tabular --cemt-color-profile \
  html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<catalog>
  <item id="a1">Alpha</item>
  <item id="b2">Beta</item>
</catalog>
```

<details>
<summary>namespaced-document</summary>

- Source: [`examples/namespaced-document.xml`](./examples/namespaced-document.xml)
- Content type: `text/xml; charset=utf-8`
- Schema: `https://cem.dev/ns/data/xml/1`
- Expected result: `pass`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  'uri=packages/cem_ml/schema-packages/xml/v1/examples/namespaced-document.xml,contentType=text/xml; charset=utf-8,schema=https://cem.dev/ns/data/xml/1' \
  --from-format xml --to-content-type 'text/xml; charset=utf-8' --to-schema \
  https://cem.dev/ns/data/xml/1 --cemt-formatter-profile tabular --cemt-color-profile \
  html
```

</details>

```xml
<?xml version="1.0" encoding="UTF-8"?>
<doc xmlns="https://example.test/doc"
     xmlns:meta="https://example.test/meta"
     meta:version="1">
  <section meta:id="intro">Hello</section>
</doc>
```

<details>
<summary>invalid-mismatched-tag</summary>

- Source: [`examples/invalid-mismatched-tag.xml`](./examples/invalid-mismatched-tag.xml)
- Content type: `application/xml`
- Schema: `https://cem.dev/ns/data/xml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xml.parse_error`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xml --schema https://cem.dev/ns/data/xml/1 \
  packages/cem_ml/schema-packages/xml/v1/examples/invalid-mismatched-tag.xml
```

</details>

```xml
<root>
  <item>Broken</root>
```

<details>
<summary>invalid-unbound-prefix</summary>

- Source: [`examples/invalid-unbound-prefix.xml`](./examples/invalid-unbound-prefix.xml)
- Content type: `application/xml`
- Schema: `https://cem.dev/ns/data/xml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xml.unbound_namespace_prefix`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xml --schema https://cem.dev/ns/data/xml/1 \
  packages/cem_ml/schema-packages/xml/v1/examples/invalid-unbound-prefix.xml
```

</details>

```xml
<root>
  <meta:item/>
</root>
```

<details>
<summary>invalid-doctype</summary>

- Source: [`examples/invalid-doctype.xml`](./examples/invalid-doctype.xml)
- Content type: `application/xml`
- Schema: `https://cem.dev/ns/data/xml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xml.dtd_rejected`
- README rendering: fenced `xml` source

```bash
dist/target/cem_ml_cli/debug/cem-ml validate --format json --fail-level parse \
  --content-type application/xml --schema https://cem.dev/ns/data/xml/1 \
  packages/cem_ml/schema-packages/xml/v1/examples/invalid-doctype.xml
```

</details>

```xml
<!DOCTYPE root SYSTEM "file:///etc/passwd">
<root/>
```
