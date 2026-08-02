# XPath Schema Package

Status: package and lossless XPath 3.1 syntax foundation

This package owns standalone and embedded XPath expression syntax. Host
languages declare expression slots and static context, then associate the
resulting `XPathExpressionAst` with a document, subtree, element, or attribute.
The XPath AST remains independently addressable when its events are fused into
an XSLT, XML, CEMT, or CEM-QL transformation stream.

## Owned Identities

- Schema URI: `https://cem.dev/ns/query/xpath/1`
- Primary content type: `application/vnd.cem.xpath`
- Interoperability alias: `text/xpath`
- Preferred extension: `.xpath`
- Syntax baseline: [XPath 3.1](https://www.w3.org/TR/xpath-31/)

XPath is specified as a component used by host languages and has no standalone
media type in the [IANA media-type registry](https://www.iana.org/assignments/media-types/media-types.xhtml).
The package therefore uses a CEM vendor media type as primary identity and does
not present `text/xpath` as a registered standard.

## Syntax And AST Model

The native adapter uses a pinned XPath 3.1-aware lexer and parser. It preserves
exact token lexemes, UTF-8 byte ranges, line/column positions, nested comments,
whitespace, delimiter depth, parser facts, and source-map frames. The parsed
grammar AST is carried beside the lossless token stream: semantic consumers use
the tree, while formatters and diagnostics retain the original source.

The lifecycle stream emits one zero-width `start-expression` event, one event
for each lossless token, and one zero-width `end-expression` event. Token events
retain their token index, delimiter depth, absolute host-adjusted range, and
source map, so a host can fuse the stream without rewriting XPath identities.

The adapter follows XPath 3.1 longest-match tokenization. It does not reuse the
legacy custom-element XPath rewriter or infer XPath by applying CEM-QL syntax.

## Host Association

Standalone expressions use their own source identity. Embedded expressions add
an attachment envelope containing:

- host source, content type, schema, node kind, node identity, and source range;
- the absolute expression range within the host source;
- namespace, variable, and function static-context bindings;
- expected sequence result and evaluation phase;
- resolver and safety policy stamps.

This allows an XPath tree to be attached to an XML document or AST subtree and
also fused into an owning XSLT stream without transferring grammar ownership to
XSLT. Attribute value template segmentation and XSLT attachment are tracked as
the next integration slice.

## Transformation Boundary

XPath is a transformation language peer to CEM-QL, CEMT, and XSLT. The package
owns parsing and static syntax; the CEM-ML `transform` path will own execution
planning. Hosts may supply context items and bindings, but must not implement a
private parser, evaluator, or external-resource resolver.

The current slice parses and models expressions. Standalone lifecycle loading,
evaluation, XSLT attribute fusion, CEM-QL/CEMT adapters, and external resource
capabilities remain explicitly tracked work.

## Formatter And Colorizer Profiles

The package registers `compact`, `pretty`, and `tabular` formatters plus
`terminal`, `html`, and `md` colorizers. In this foundation slice all formatter
profiles are intentionally lexical-lossless aliases. Reflow is deferred until
grammar-node boundaries and embedded expression source maps are exercised by
the lifecycle output pipeline.

## Safety

Parsing performs no I/O and does not evaluate expressions. Functions that can
read resources, including `doc()`, `collection()`, and `unparsed-text()`, require
an explicit resolver capability during future evaluation. Static context and
policy stamps are part of the attachment identity so a parsed tree cannot be
silently reused under a broader policy.

## Verification

`yarn nx run cem_ml_schema_package_xpath_v1:verify` validates the schema-package
manifest, runs lossless lexer/parser and host-attachment tests, verifies embedded
catalog identity, and checks that README examples use fenced XPath source with
no SVG fallback. `yarn nx run cem_ml:build:wasm` verifies that the pinned parser
dependency stack remains compatible with the browser WASM target.

## Tracked Incomplete Work

- Bind package facts to runtime diagnostics and add dedicated lifecycle loading.
- Segment XSLT XPath attributes and AVTs, then associate their ASTs with exact
  XML attribute-value ranges.
- Add standalone transformation execution and CEM-QL/CEMT/XSLT adapters.
- Define grammar-aware formatting before making profile output differ.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-path</summary>

- Source: [`examples/basic-path.xpath`](./examples/basic-path.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/book[@lang = "en"]/title
```

<details>
<summary>functions-and-variables</summary>

- Source: [`examples/functions-and-variables.xpath`](./examples/functions-and-variables.xpath)
- Content type: `text/xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
for $book in /catalog/book
return normalize-space($book/title)
```

<details>
<summary>maps-arrays-and-comments</summary>

- Source: [`examples/maps-arrays-and-comments.xpath`](./examples/maps-arrays-and-comments.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
(: Preserve nested comments (: including inner trivia :). :)
map {
    "titles": array { /catalog/book/title/string() },
    "count": count(/catalog/book)
}
```

<details>
<summary>unicode-qname</summary>

- Source: [`examples/unicode-qname.xpath`](./examples/unicode-qname.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/π/@γλώσσα
```

<details>
<summary>invalid-unclosed-predicate</summary>

- Source: [`examples/invalid-unclosed-predicate.xpath`](./examples/invalid-unclosed-predicate.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.parse_error`, `cem.xpath.unclosed_delimiter`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/book[1
```
