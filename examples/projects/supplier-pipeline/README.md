# Supplier pipeline

Combine a supplier catalog with stock figures to make an availability report. Change a product or its stock, then rebuild the HTML, JSON and YAML reports.

## Essential files

- [data/catalog.xml](data/catalog.xml) — The supplier product catalog.
- [data/stock.csv](data/stock.csv) — Current stock quantities.
- [templates/reconcile.cemt](templates/reconcile.cemt) — Combines catalog products with stock data.
- [templates/view.cemt](templates/view.cemt) — Displays the availability report.
- [graphs/availability.cem](graphs/availability.cem) — Connects the inputs, transformations and outputs.
- [data/catalog-invalid.xml](data/catalog-invalid.xml) — A damaged catalog to repair.
- [expected/availability.html](expected/availability.html) — The reference availability page.

## Using the example

Open **Examples → Content types and schemas → Data → Supplier pipeline**, or
find the same project under **Amsterdam 2026**. Clone it, then open the resulting
ordinary editable project in Studio. The catalog original remains unchanged;
each copy owns its resources, configurations and saved revisions independently.

The JSON artifact and expected fixture use the shared native graph byte policy:
UTF-8, two-space indentation, LF and no appended newline, with native lexical
object-key order. Native CLI and Studio builds compare these exact bytes and
hashes; the supplier records and reconciliation rules are unchanged.

## Resources and configurations

The resource identities in [example.json](example.json) are:

| Resources | Content type | Schema identity |
| --- | --- | --- |
| `data/catalog.xml`, `data/catalog-invalid.xml` | `application/xml` | `https://cem.dev/ns/data/xml/1` |
| `data/stock.csv` | `text/csv;header=present` | `https://cem.dev/ns/data/csv/1` |
| `queries/products.xpath` | `application/vnd.cem.xpath` | `https://cem.dev/ns/query/xpath/1` |
| `templates/reconcile.cemt`, `templates/view.cemt` | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` |
| `graphs/availability.cem` | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| `expected/products.json`, `expected/availability.json` | `application/json` | `https://cem.dev/ns/data/json/1` |
| `expected/availability.yaml` | `application/yaml` | `https://cem.dev/ns/data/yaml/1` |
| `expected/availability.html` | `text/html` | `https://cem.dev/ns/data/html/1` |

- `data/catalog.xml` declares four products in the `urn:demo:supplier` vocabulary.
- `queries/products.xpath` selects IDs and names using an explicit namespace
  binding. `expected/products.json` records the four selected strings.
- `data/catalog-invalid.xml` contains an intentional mismatched closing tag for
  the native XML failure/repair workflow.
- `data/stock.csv` supplies product IDs and quantities. Its explicit
  `text/csv;header=present` identity keeps cells as strings at import.
- `templates/reconcile.cemt` matches product IDs across the two retained native
  trees and constructs a typed `cem:generic-data` result.
- `templates/view.cemt` composes that same result into a readable HTML view.
- `graphs/availability.cem` declares the named `stock` input and explicit CEM
  result identity. It branches to the view, a JSON service payload and a YAML
  partner feed. Graph joins do not perform business-record matching.
- `expected/availability.json`, `.yaml` and `.html` state the baseline results.
- **Inspect supplier catalog** (`catalog-source`) and **Inspect supplier stock**
  (`stock-source`) parse saved source through the native adapters.
  **Validate supplier catalog** (`validate-catalog`) validates XML source syntax
  and namespace bindings. **Select namespaced supplier products**
  (`query-products`) runs the saved XPath query. **Repair malformed supplier XML**
  (`repair-catalog`) validates the deliberate failure. **Supplier availability
  pipeline** (`availability`) executes the saved graph and all saved dependencies.

## XML namespaces and source validation

Run **Validate supplier catalog**, then **Select namespaced supplier products**.
The query returns `SKU-A=Café cups`, `SKU-B=Tea pots`, `SKU-C=Glass jars` and
`SKU-D=Serving trays`. Its configuration binds
`--namespace supplier=urn:demo:supplier`; `supplier` is the query's prefix and
the document uses a default namespace. Selection compares the namespace URI and
local name, independently of the source prefix spelling. The query uses explicit
XPath strings, and product IDs remain unqualified XML attributes.
The prefix binding belongs to query execution. Standalone XPath source validation
currently does not inherit caller namespace bindings, and standalone XPath
Parse/Inspect/format are not configurations in this project.

For example, changing the document to use a `p` prefix for the catalog, product
and name elements while binding `xmlns:p="urn:demo:supplier"` preserves the
selection. An element called `product` in another namespace does not qualify.
The reconciliation template follows the same expanded-name rule when importing
the catalog through the shared native CEM tree.

Source validation checks XML well-formedness, encoding, namespaces and the
registered parser contracts. It does not establish supplier business rules or
perform arbitrary XSD/RELAX NG instance validation. DTD-bearing documents and
external entity resolution are rejected; no filesystem/network entity resolver
is enabled. XML formatter profiles currently preserve lexical source bytes
rather than reindent mixed content. See the [XML package](../../../packages/cem_ml/schema-packages/xml/v1/README.md)
for supported contracts and the [XHTML document project](../xhtml-document/README.md)
for XML-serialized HTML, XHTML-specific validation and generated HTML output.

Run **Repair malformed supplier XML**. The intentional
`</product-broken>` on line 6 produces two `cem.xml.parse_error` diagnostics:
the closing tag does not match, and the open product consequently lacks its
matching close. The first diagnostic points at the original closing tag, after
the earlier UTF-8 `Café` text; no CEM-ML wrapper offsets replace the XML location.
Replace `</product-broken>` with `</product>`, save, then rerun the same
configuration. Both diagnostics clear. This changes only the invalid resource,
leaving the valid catalog and the availability graph's inputs independent.

## Build the availability report

Run **Supplier availability pipeline**. The operation output previews the HTML
view. In **Results → Operation output**, choose **Output artifact** to inspect
the HTML, JSON or YAML publication. **Copy exact output** and **Download exact
output** use that selected artifact's verified bytes, while source selection
stays on the graph. The execution details report the saved run and total output
count/size; the **Transformation graph** result also includes all three outputs.
The service/partner outputs contain typed numeric quantities and booleans.
No template text or JavaScript implements a JSON/YAML serializer: publication
uses the shared engine and the schema packages' formatters.

Native CLI equivalent (repository root, writing under this example's `generated/`):

```sh
cem-ml transform --config examples/projects/supplier-pipeline/graphs/availability.cem --scope-budget queueSize=1024
```

## Domain rules

The catalog determines product order. Matching uses an exact product ID, independent
of CSV row order; unlisted stock IDs do not add products. A unique stock row must
contain a canonical signed decimal integer whose native conversion is lossless.
The rule checks the text against its explicit `num:integer` round trip; permissive
numeric coercion alone does not validate the feed.

| Case | Quantity | Status | Available |
| --- | --- | --- | --- |
| Unique positive stock | Stock quantity | `available` | true |
| Unique zero stock | 0 | `out-of-stock` | false |
| Missing stock | 0 | `unknown` | false |
| Duplicate stock rows or catalog IDs | 0 | `ambiguous` | false |
| Unique negative stock | Negative quantity | `invalid` | false |
| Invalid integer text | No publication | Native export diagnostic | No publication |

`SKU-A` has 7 units, `SKU-B` has 0, `SKU-C` has no stock row, and `SKU-D` has two.
Duplicate rows are never silently summed or resolved by choosing the first one.
The sample only reports availability; it performs no reservation or account action.

## Saved edits

Select **Inspect supplier stock**, run once, then change `SKU-A,7` to `SKU-A,0`
and run that configuration again without saving. Its parsed output still uses 7;
the draft remains unsaved. Studio's normal draft guard applies when switching
configurations: cancel to keep editing, save to commit, or discard to leave the
persisted stock unchanged. Discarding and running the pipeline still reports 7.

Edit the stock to 0 again and choose **Save and validate** before switching to
**Supplier availability pipeline**. Run: all three outputs now report out of
stock. Saved changes survive offline reopening. Another project copy continues
to report 7 until its own stock is edited and saved.

## Failure and repair

Change `SKU-A,7` (or `SKU-A,0`) to `SKU-A,many`, save, and rerun the pipeline.
CSV syntax validation succeeds because the cell is a string. The domain rule
marks its constructed number as invalid, and native data export rejects it with
an `invalid data number` diagnostic. No output from that failed run is published.
Existing files from a prior successful native run remain prior outputs, not a
new successful result. Repair the cell to `7`, save, and rerun to regenerate all
three outputs. Decimal fractions, leading zeros and noncanonical integer text
are rejected by the same rule.

## Graph and source provenance

Inspect the imports, the named `@with:stock` dependency, reconciliation and view
stages, and three exports in the graph controls/results. The **Provenance** result
links generated HTML ranges to authored instructions in `templates/view.cemt`;
selecting a row opens that project resource and source range. Edit the explanatory
sentence, save and rerun to see its changed output and provenance.

Computed availability is explained by the reconciliation rule and stock resource.
This example does not claim that every generated character traces directly to an
original XML node or CSV cell. JSON/YAML formatter output and HTML template
provenance are separate boundaries. Export/import of the portable project and
independent copies use the same Studio facilities as user-created projects.

The two inspection configurations display native source coordinates in CEM-ML;
Parse/Inspect currently publishes no owned artifact-source references to the
Provenance panel. Follow the graph's actual output maps or native diagnostics;
printed source IDs and ranges do not confer navigation authority.

The example adds no JavaScript behavior, credentials or storage keys. Entered
passwords and one-time codes elsewhere block recovery until explicitly cleared;
automatic whole-Studio recovery remains disabled.
