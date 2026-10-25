# CSS selector queries

Open **Examples → Content types and schemas → Queries and transformations →
CSS selector queries**, or find the same project under **Documents and UI**.
Clone it to edit an ordinary saved project. Typical uses are finding document
sections, checking an HTML publishing structure, selecting XML domain records
and examining the original source ranges of matched elements.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| Data and expected JSON artifacts | `application/json` | `https://cem.dev/ns/data/json/1` |
| CSS selector source | `application/vnd.cem.query-expression+css-selector` | `https://cem.dev/ns/query/css-selector/1` |
| XML data | `application/xml` | `https://cem.dev/ns/data/xml/1` |
| HTML source and expected artifacts | `text/html` | `https://cem.dev/ns/data/html/1` |

## Query and document identities

Selectors use `application/vnd.cem.query-expression+css-selector` and schema
`https://cem.dev/ns/query/css-selector/1`. This identity is separate from the
`text/css` stylesheet schema. Extensions do not select query semantics.

- `data/products.html` uses native HTML identity, `text/html` with schema
  `https://cem.dev/ns/data/html/1`.
- `queries/ready-cards.css-selector` selects ready product cards under the
  catalog using child/class/attribute selectors and `:has(> h2)`. Its overlapping
  `article#tea` branch deliberately selects Tea again.
- `data/catalog.xml` uses `application/xml` and schema
  `https://cem.dev/ns/data/xml/1`, with the namespace `urn:example:products`.
- `queries/namespaced-products.css-selector` uses the explicitly supplied
  `c=urn:example:products` static namespace binding. The source's prefix spelling
  is not query identity; an unbound prefix is an error.

Both queries consume genuine borrowed lifecycle-owned element trees. They do
not construct a browser DOM, parse serialized JSON results back into a tree or
substitute a stylesheet parser. The JSON result is a named human-facing export.

| Configuration | Typical use | Expected result |
| --- | --- | --- |
| `inspect-selector` — Inspect the original selector source | Review original query text and byte offsets | Existing CEM text-envelope inspection, not a typed selector AST |
| `validate-selector` — Validate the ready-card selector | Check standalone query syntax and capability facts | Zero errors and warnings |
| `query-cards` — Select ready HTML product cards | Select structural document nodes | Tea and Cocoa element identities, deduplicated and in document order |
| `query-namespaces` — Select namespaced XML products | Select native XML nodes with explicit namespace context | Tea and Cocoa product identities in source order |
| `repair-selector` — Repair the unclosed attribute selector | Locate and repair malformed query source | `css-selector.parse.invalid`, then zero errors after repair |

## Native result contracts

The exact selected identity fields are checked in `expected/ready-cards.json`
and `expected/namespaced-products.json`. They contain `localName`, `namespaceUri`,
`documentOrder` and `nodeId`. These are native node identities, not invented
attribute/text result fields. The current native JSON matcher result does not
include each matched element's text or attributes.

The HTML matches are `html:event:13` and `html:event:31`, both XHTML `article`
elements. Their original opening-tag spans start at UTF-8 byte offsets 128 and
369, with lengths 62 and 64. The duplicate Tea selection is removed by native
identity, rather than by comparing serialized node values.

The XML matches are `xml:event:3` and `xml:event:9`, both `product` elements in
`urn:example:products`. Their opening-tag spans start at offsets 77 and 186,
with lengths 32 and 34. Each result also carries the query's original source
range. Source ranges are supplied by the native query result and retain their
original content-type transforms; they are not browser-generated positions.

Matched-document coordinate fields are part of the explicitly exported native
result. They are distinct from the query-source frames published to Studio's
Provenance selector. They do not turn every exported numeric range into an owned
link to a document. Standalone Inspect likewise prints coordinate data without
publishing owned artifact-source references.

In a saved copy, change Coffee's `data-state="draft"` to `data-state="ready"`,
save and rerun **Select ready HTML product cards**. All three cards now match,
while the duplicate Tea branch still yields only one Tea identity. Unsaved drafts
leave the persisted execution input unchanged. Edit the selector source to
`.product[data-state="draft"]` and rerun to change the selected set without
changing the document. The XML query can be edited independently.

## Failure and repair

`failures/invalid.css-selector` contains `article[` with an unclosed attribute
selector. Run **Repair the unclosed attribute selector**: native validation
reports one `css-selector.parse.invalid` at line 1, column 8, original byte offset
7. Replace that incomplete line with the valid query from
`queries/ready-cards.css-selector`, save and rerun. Validation then reports zero
errors and warnings, and the repaired source matches the valid query exactly.

The query language's parser and matching semantics are native. The current
standalone `inspect ast` command still presents a generic CEM text envelope;
it is explicitly not a typed selector-AST export. Although the schema package
declares formatter/colorizer assets, the current general converter has no
same-selector lifecycle export adapter. A conversion attempt reports
`cem.lifecycle.target_adapter_unsupported` and can return a compatibility JSON
projection. This project therefore exposes no misleading formatting operation
or fallback formatter. Original query source remains available in the editor.

## Bounded semantics and independent offline projects

The supported contract is the package's staged Selectors Level 4 matrix, not
universal browser selector parity. Unknown extensions and pseudo-elements remain
unsupported. Browsing history, UI state, shadow trees and host-state
pseudo-classes require advertised capabilities; they cannot silently match
false. Relational traversal consumes native work/result budgets. Parsing,
validation and source inspection do not fetch resources or evaluate a host
cascade. See the
[CSS selector package](../../../packages/cem_ml/schema-packages/css-selector/v1/README.md).

This project produces query results rather than an HTML transformation graph;
it does not invent a site build or a dedicated example screen. The native HTML
and XML resources can be viewed and queried in the same Studio workspace.
There is no sample UI JavaScript or project theme/storage key.

Once Studio and project resources are cached, each saved copy can reopen and
rerun its native validation and queries offline. Sources, commands and drafts
remain independent of the catalog original and other copies. Entered passwords
or one-time codes continue to block recovery, and automatic whole-Studio
recovery remains disabled.
