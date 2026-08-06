# XPath Schema Package

Status: package, lossless XPath 3.1 syntax, lifecycle loading, and typed evaluation contracts

This package owns standalone and embedded XPath expression syntax. Host
languages declare expression slots and static context, then associate the
resulting `XPathExpressionAst` with a document, subtree, element, or attribute.
The XPath AST remains independently addressable when its events are fused into
an XSLT, XML, CEMT, or CEM-QL transformation stream.

## Owned Identities

- Schema URI: `https://cem.dev/ns/query/xpath/1`
- Primary content type: `application/vnd.cem.xpath`
- Interoperability alias: `text/xpath`
- Result artifact content type: `application/vnd.cem.xpath-result+json`
- Preferred extension: `.xpath`
- Syntax baseline: [XPath 3.1](https://www.w3.org/TR/xpath-31/)

XPath is specified as a component used by host languages and has no standalone
media type in the [IANA media-type registry](https://www.iana.org/assignments/media-types/media-types.xhtml).
The package therefore uses a CEM vendor media type as primary identity and does
not present `text/xpath` as a registered standard.

## Syntax And AST Model

The current foundation uses a CEM-owned longest-match scanner that preserves
exact token lexemes, UTF-8 byte ranges, line/column positions, nested comments,
whitespace, delimiter depth, lexical errors, and source-map frames. Its
package-private token categories distinguish numeric forms, strings, EQNames,
keywords, word and symbol operators, punctuation, trivia, and errors. The
scanner follows the normative XPath 3.1 lexical grammar and does not call Xee.
The pinned `xee-xpath-lexer` crate is a development-only differential oracle
for package examples and ambiguous lexical boundaries.

The CEM-owned recursive-descent parser consumes the scanner token stream
directly, resolves names from the attachment static context, and constructs the
typed package AST without reparsing or an intermediate representation. The
`xee-xpath-ast` and `xee-xpath-lexer` crates are development-only differential
oracles for the completed grammar slices. The
[Xee source pinned at commit `200b1e3356ea9d6dd2901d67bd941b779df7e5b7`](https://github.com/Paligo/xee/tree/200b1e3356ea9d6dd2901d67bd941b779df7e5b7)
is an MIT-licensed, non-normative implementation reference, never an AST or
execution boundary. XPath 3.1, XDM 3.1, and Functions and Operators 3.1 remain
normative, and adapted implementation ideas require recorded source provenance
and license review.

Full XPath 3.1 is the accepted destination. Delivery is staged through explicit
conformance slices and the schema-owned
[`tests/xpath-3.1-conformance.cem`](./tests/xpath-3.1-conformance.cem) gap
matrix. Behavior outside a completed slice remains visible through stable typed
diagnostics rather than silently inheriting omissions from the reference
implementation.

The primary syntax contract is a strongly typed W3C expression model with
typed names, literals, operators, sequence types, paths, steps, node tests,
maps, arrays, and function items. The lossless token stream remains a separate
source-fidelity artifact. XSLT, CEMT, and CEM-QL fusion consumes a derived
start/end syntax event stream rather than weakening the primary AST into a
generic property bag. The current parser slice represents rooted and relative
paths, axes, node tests, predicates, variables, binary operators, function
calls, maps, arrays, typed names/literals, and host-adjusted ranges directly.
Its balanced syntax events are derived from that tree. The runtime parser and
public syntax module have no Xee, serde, Xot, or JSON representation dependency.

The lifecycle stream emits one zero-width `start-expression` event, one event
for each lossless token, and one zero-width `end-expression` event. Token events
retain their token index, delimiter depth, absolute host-adjusted range, and
source map, so a host can fuse the stream without rewriting XPath identities.

The adapter follows XPath 3.1 longest-match tokenization. It does not reuse the
legacy custom-element XPath rewriter or infer XPath by applying CEM-QL syntax.

## Schema-Owned Diagnostics

The native parser emits neutral facts for decode, lexical, parse, namespace,
delimiter, host-association, external-resource, source-map, and event-lifecycle
conditions. `schema/xpath.cem` owns the diagnostic code, severity, contract,
behavior, and policy bound to each reportable fact.

Standalone validation accepts the primary content type, the `text/xpath` alias,
or the package schema URI and maps those facts without lowering the expression
to CEM. Diagnostics retain exact byte, line, column, and source-map coordinates.
Manifest-owned pass and failure fixtures exercise the same validation path.

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
XSLT. Entity-free XPath-bearing XSLT attributes now carry the package AST, exact
attribute-value range, owning XML event identity, and inherited namespace
context directly. Attribute value template segmentation and entity-decoded XML
token-range projection remain the next XSLT fusion slice; the generic XML AST
already owns the decoded scalar-to-source map and strict boundary projection.

## Transformation Boundary

XPath is a transformation language peer to CEM-QL, CEMT, and XSLT. The package
owns parsing and static syntax; the CEM-ML `transform` path owns execution
planning. Hosts may supply context items and bindings, but must not implement a
private parser, evaluator, or external-resource resolver.

Execution will consume the package-owned AST and existing CEM XML AST/event
streams directly. It must not reparse source text, copy XML into Xot or another
evaluator-owned replacement tree, or project input/results through JSON. The
strict native-AST transform data-plane migration tracked in `docs/todo.md` is a
prerequisite for registering XPath execution.

The current implementation defines `XPathEvaluationRequest`,
`XPathEvaluatorCapabilities`, and `XPathResultArtifact`. Result sequences retain
XPath order across node, atomic, map, array, function, and mixed items. Node
items retain the exact lifecycle AST owner plus a typed node handle alongside
source/node identity, atomic values retain type plus lexical value, and function
items are evaluator-scoped handles rather than serialized closures. Every
artifact and item carries an origin-first source map, and the result keeps the
static context plus resolver and safety policy stamps.

The first native evaluator slice executes literals, variables, context items,
expression sequences, and child/self paths with name or kind tests directly over
the package-owned XPath AST and lifecycle-owned XML event AST. Unsupported
operators, axes, predicates, functions, and constructors fail with a stable
schema-owned diagnostic. The evaluator does not read expression source text,
project through CEMT or JSON, reparse XML, or construct a replacement tree.

The standalone executable adapter is registered for XPath template identities.
It compiles template source once at the lifecycle/compile boundary, evaluates a
primary lifecycle-owned XML document AST as the context item, and returns the
typed `XPathResultArtifact`. The `transform` command invokes the registered JSON
result exporter only after native evaluation completes. Parameters, named
entrypoints, secondary inputs, and non-XML input AST families are rejected until
their context and XDM binding contracts are defined.

The result media type is intentionally distinct from expression source and does
not enter the XPath source parser. XML, JSON, CEM, and text serialization remain
explicit downstream conversion edges. CEM-QL, CEMT, and XSLT invocation adapters
remain unregistered pending schema-owned call, context, and variable-binding
semantics.

## Formatter And Colorizer Profiles

The package registers `compact`, `pretty`, and `tabular` formatters plus
`terminal`, `html`, and `md` colorizers. In this foundation slice all formatter
profiles are intentionally lexical-lossless aliases. Reflow is deferred until
grammar-node boundaries and embedded expression source maps are exercised by
the lifecycle output pipeline.

## Safety

Parsing performs no I/O and does not evaluate expressions. Evaluator capability
validation requires the package-owned AST, deterministic native and WASM
results, item-origin source maps, all XPath 3.1 item kinds, and CEM resolver-only
resource access. Functions such as `doc()`, `collection()`, and
`unparsed-text()` cannot receive a direct filesystem or network boundary.
Current time, timezone, environment variables, randomness, recursion,
cancellation, and work budgets must also be explicit request capabilities; the
evaluator cannot read ambient process or host state.

## Verification

`yarn nx run cem_ml_schema_package_xpath_v1:verify` validates the schema-package
manifest and fixture expectations, runs lossless lexer/parser, schema-diagnostic
handoff, lifecycle loading, no-fallback validation, and host-attachment tests,
verifies the full-destination conformance matrix, native evaluator owner/path and
scalar semantics, standalone transform routing, mixed result artifacts and
evaluator capability rejection, verifies embedded catalog identity, and checks
that README examples use fenced XPath source with no SVG fallback.
`yarn nx run cem_ml:build:wasm` verifies that the CEM-owned scanner and parser
remain compatible with the browser WASM target.

## Tracked Incomplete Work

- Implement a CEM-owned XPath 3.1 compiler/evaluator and prove native/WASM AST
  consumption through CEM-only resolver and safety capabilities.
- Define the XPath parser's typed source-range remapping input, project token
  and diagnostic ranges through the generic entity-decoded XML attribute map,
  segment XSLT AVTs, and associate those remaining expression ASTs with exact
  source ranges.
- Define schema-owned context and variable bindings, then add CEM-QL, CEMT, and
  XSLT XPath invocation adapters.
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
<summary>explicit-axes-and-escaped-string</summary>

- Source: [`examples/explicit-axes-and-escaped-string.xpath`](./examples/explicit-axes-and-escaped-string.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `pass`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/descendant::book[@title = "The ""Quoted"" Book"]/ancestor-or-self::node()
```

<details>
<summary>unknown-prefix</summary>

- Source: [`examples/unknown-prefix.xpath`](./examples/unknown-prefix.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.unknown_namespace_prefix`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/ns:book
```

<details>
<summary>invalid-token</summary>

- Source: [`examples/invalid-token.xpath`](./examples/invalid-token.xpath)
- Content type: `text/xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.lexical_error`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/`book
```

<details>
<summary>mismatched-delimiter</summary>

- Source: [`examples/mismatched-delimiter.xpath`](./examples/mismatched-delimiter.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.parse_error`, `cem.xpath.mismatched_delimiter`, `cem.xpath.unclosed_delimiter`
- README rendering: fenced `xpath` source

</details>

```xpath
/catalog/book[1)
```

<details>
<summary>external-resource-denied</summary>

- Source: [`examples/external-resource-denied.xpath`](./examples/external-resource-denied.xpath)
- Content type: `application/vnd.cem.xpath`
- Schema: `https://cem.dev/ns/query/xpath/1`
- Expected result: `fail`
- Expected diagnostics: `cem.xpath.external_resource_denied`
- README rendering: fenced `xpath` source

</details>

```xpath
doc("catalog.xml")/catalog
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
