# XPath catalog queries

Open **Examples → Content types and schemas → Queries and transformations →
XPath catalog queries**, or find the same ordinary project under **Data**.
Clone it to edit its resources and configurations without changing the catalog
original or another saved copy. It demonstrates document selection, a small
supplier join, data-driven parameter maps and native result inspection.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| Data and expected JSON artifacts | `application/json` | `https://cem.dev/ns/data/json/1` |
| XPath source | `application/vnd.cem.xpath` | `https://cem.dev/ns/query/xpath/1` |
| XML data | `application/xml` | `https://cem.dev/ns/data/xml/1` |

## Sources and identities

`data/catalog.xml` uses `application/xml` with schema
`https://cem.dev/ns/data/xml/1`. The catalog, product, supplier and selection
elements use `urn:example:catalog`; attributes such as `stock`, `sku` and
`minimum-stock` have no namespace. Its suppliers include **Café Harbor**, so
original byte positions must account for UTF-8 rather than JavaScript character
counts. XML source validation establishes source well-formedness and policy,
not a separate application-domain catalog schema.

All five query files declare `application/vnd.cem.xpath` with
`https://cem.dev/ns/query/xpath/1`. XPath is a distinct native language and is
not CEM-QL syntax. Expanded `Q{urn:example:catalog}name` tests make the primary
queries independent of a host prefix mapping. The supplier join instead uses
`c:` names and its configuration explicitly binds `c=urn:example:catalog`.
Query identity comes from metadata, not filename inference or text guessing.

## Configurations

| Configuration | Typical use | Result |
| --- | --- | --- |
| `validate-filter` — Validate the expanded-name XPath source | Check standalone grammar, namespace names and source policy | Zero errors and warnings |
| `query-filter` — Filter available products | Select stock of at least four and project readable product labels | `tea:Green tea`, `cocoa:Cocoa` |
| `query-nodes` — Inspect selected native product nodes | Inspect real retained nodes and source provenance | Two product node identities with their original opening-tag ranges |
| `query-join` — Join products to their suppliers | Evaluate dependent `for` bindings and matching supplier IDs | Three order-preserving supplier summaries |
| `query-parameters` — Apply the saved selection parameter map | Build a map from XML options and filter/join using its lookups | Two option fields and two selected product/supplier rows |
| `inspect-parameters` — Inspect the native map and array result | Inspect native map/array/atomic item kinds | One map containing an integer, a string and an array of SKUs |
| `repair-query` — Repair the unclosed XPath predicate | Diagnose and close the independent malformed source | Two native diagnostics, then no diagnostics after repair |

The query configurations use the original lifecycle-owned XML tree. Native
comparisons, XPath focus, dependent bindings, simple mapping and map lookup run
inside the native evaluator. No browser DOM, JavaScript selector, JSON AST or
replacement data tree participates in evaluation. JSON is the explicitly chosen
human-facing result export.

## Saved data, queries and parameters

Change Coffee's `stock="2"` to `stock="8"` in the saved catalog, then save and
rerun **Filter available products**. All three products now appear. Unsaved
drafts leave the persisted command inputs unchanged.

The catalog's `<selection minimum-stock="4" region="west"/>` is ordinary owned
data. **Apply the saved selection parameter map** constructs a native map with
`minimum-stock` and `region` keys from those attributes. It uses
`$options?minimum-stock` and `$options?region` while filtering products and joining
their suppliers. Change `minimum-stock="4"` to `minimum-stock="6"`: the baseline
map report keeps Tea and excludes Cocoa. This is data-driven parameter-map
handling; it does not claim that the CLI `query` command accepts external
variables or that direct XPath `transform` accepts `--param`. Those external
binding routes are currently unavailable.

Edit `queries/available.xpath` to change its literal stock threshold from four
to six, save and rerun its configuration. Data, query source and configuration
remain separate editable resources. The explicit namespace mapping in **Join
products to their suppliers** belongs to that configuration, and producer prefix
spelling is not an element's expanded-name identity.

## Inspecting real native results

**Inspect selected native product nodes** returns node items, not their
reconstructed text. At baseline they are `cem:node:27` and `cem:node:45`, both
`{urn:example:catalog}product` elements. The first opening element is line 9,
column 5, byte offset 320, length 47; the second is line 11, column 5, byte offset
532, length 49. `sourceUri`, `sourceRange` and `SourceBoundary` frames identify
the original catalog. These are original opening-element spans, not whole
product-subtree or arbitrary descendant-character mappings. After source edits,
follow the new result's coordinates rather than assuming baseline offsets.

Those original-data coordinates are explicit result fields, not newly granted
Studio source links. The common query publication used by the Provenance selector
identifies the admitted query buffer and its expression ranges separately.
Source-node exports must not be reparsed into runtime nodes or treated as owned
navigation endpoints merely because they contain a URI and numbers.

The scalar reports and constructed map/array retain their native expression
origin frames and the result's `expressionUri`. Computed text does not acquire
invented character-by-character lineage to the XML values that influenced it.
`expected/*.json` records exact semantic values, native node fields or the typed
map/array result, without embedding machine-specific input paths. Native node
handles and executable programs are not serialized for reuse as query inputs.

## Native command

From this project directory:

```sh
cem-ml query data/catalog.xml --content-type application/xml \
  --schema https://cem.dev/ns/data/xml/1 \
  --query-file queries/supplier-join.xpath \
  --query-content-type application/vnd.cem.xpath \
  --query-schema https://cem.dev/ns/query/xpath/1 \
  --namespace c=urn:example:catalog --output json
```

The expanded-name query files need no `--namespace` flag. The configured
prefixed join requires its declared host static context. Standalone source
validation currently does not receive that query-context binding from the CLI's
namespace option; the context-independent validation configuration therefore
uses the expanded-name source. An independently opened prefixed source can
report an unresolved namespace until evaluated in its configured query context.

## Deliberate failure and repair

`failures/unclosed-predicate.xpath` is separate from the working query files.
Run **Repair the unclosed XPath predicate**. The opening `[` is not closed,
producing `cem.xpath.unclosed_delimiter` at line 1, column 92, UTF-8 byte offset
91, plus `cem.xpath.parse_error` at line 2, column 1, byte offset 104.
Append `]` after `@stock >= 4`, save and revalidate. Both errors clear; the
repaired bytes match `queries/available-nodes.xpath` exactly. An invalid saved
query cannot publish a successful replacement result.

## Current limits and offline copies

This project exercises the documented native XPath subset, not complete XPath
3.1 conformance. It performs no `doc()`, collection or network/file resource
reads. Runtime host capabilities, work/result budgets and resolver policy remain
in force. Parsing or validating a source is not permission to execute arbitrary
resource-sensitive functions.

Standalone XPath command source export is currently unavailable despite its
registered formatter assets. Native `convert` refuses the missing target/AST
adapter; the project does not manufacture a formatted XPath fixture. Current
command Parse/Inspect paths can present a generic CEM projection, and `Q{...}`
can be misinterpreted as CEM structure. They are not advertised as XPath syntax
inspection. The supported inspection configurations inspect native query result
nodes, maps and arrays instead. These source-operation gaps are explicit native
follow-ups, separate from the evaluator used here.

The existing shared editor and structured result controls display this ordinary
project. Once the runtime and project resources are cached, each saved copy can
reopen and execute offline. Catalog originals, repaired resources, saved data,
queries, configurations and drafts remain independent. The project owns no UI
script, theme or storage key. Credential recovery refusal and disabled automatic
whole-Studio recovery remain unchanged.
