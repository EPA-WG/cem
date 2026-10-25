# CSV inventory

Use this ordinary project to inspect a spreadsheet export, compare declared
header policies, repair native CSV diagnostics and report inventory that needs
attention. Product names and quantities are fictional; the report performs no
ordering or stock reservation.

Open **Examples → Content types and schemas → Data → CSV inventory**, or find the
same project under **Queries and transformations**. Create an editable copy to
change resources. The original stays read-only, and each copy owns its saved
resources, configurations, revisions and drafts.

## Resources and identities

| Resource | Content type | Schema | Purpose |
| --- | --- | --- | --- |
| `data/inventory.csv` | `text/csv;charset=utf-8;header=present` | `https://cem.dev/ns/data/csv/1` | Four inventory records with named columns. |
| `data/inventory-headerless.csv` | `text/csv;charset=utf-8;header=absent` | `https://cem.dev/ns/data/csv/1` | The same four records without a header row. |
| `queries/low-stock.cem-ql`, `queries/headerless.cem-ql` | `application/vnd.cem.query-expression+cem-ql` | `https://cem.dev/ns/query/cem-ql/1#expression` | Saved standalone expressions over imported native trees. |
| `templates/model.cemt`, `templates/view.cemt` | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` | Typed report model and HTML table. |
| `graphs/inventory.cem` | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` | Explicit import, template stages and report exports. |
| `config/header-policy.json` | `application/json` | `https://cem.dev/ns/cli/run-config/1` | Authored validation configuration with an intentionally unknown header parameter. |

The three malformed CSV resources use the same declared CSV identity as the
inventory. `expected/` contains deterministic reports, query results, compact
interchange output and aligned review output. Other portable configuration
resources are generated from `example.json` by Studio's normal project pipeline.

The inventory columns are `sku`, `name`, `quantity` and `reorder_level`:

| SKU | Name | Quantity | Reorder level |
| --- | --- | --- | --- |
| SKU-A | Café cups, set | 3 | 5 |
| SKU-B | Tea "travel" pot | 8 | 4 |
| SKU-C | Glass jars | 0 | 2 |
| SKU-D | Serving trays | 12 | 6 |

The comma in `"Café cups, set"` belongs to one quoted cell. The source spelling
`"Tea ""travel"" pot"` doubles the quotes inside that quoted cell. Native import
retains the decoded cell text, original field ranges and row order. Numeric-looking
CSV cells remain strings; parsing does not infer numeric column types.

## Configurations and header policy

| Configuration | Result |
| --- | --- |
| **Inspect imported inventory tree** | Retained generic-data CEM projection, with header-derived mapping keys and original source start offsets. |
| **Validate inventory CSV source** | Native CSV syntax/dialect diagnostics, independently of inventory count rules. |
| **Format CSV for interchange** | Native `compact` CSV output with required quoting and no presentation padding. |
| **Review aligned CSV** | Native `tabular` review presentation. Padding is visual and must not be treated as a safe reimport contract. |
| **Select low-stock SKUs** | Explicit JSON query result containing `SKU-A` and `SKU-C`. |
| **Inspect declared headerless rows** | Arrays of string cells, with every data row preserved. |
| **Read headerless SKU cells** | Explicit JSON query result containing all four SKUs. |
| **Repair declared header policy** | Execute the authored run configuration and observe a native metadata warning. |
| **Repair an unclosed quoted field** / **Repair an unescaped field quote** | Intentional hard CSV syntax failures. |
| **Repair an incomplete inventory row** | Intentional row-width drift, reported as a warning. |
| **Build low-stock inventory** | An HTML table and typed JSON records, with HTML as the explicit entry artifact. |

With `header=present`, the first source row supplies mapping keys and is excluded
from data rows. With `header=absent`, every row remains an array of string cells;
there is no first-row header guess in these explicitly declared examples. The
report graph requires its named inventory columns; the headerless query instead
reads its positional first cell.

CSV source validation does not require these particular column names, reject
all duplicate/empty header names, enforce SKU uniqueness, or provide a complete
inventory application schema. The unknown MIME parameter `header=maybe` produces
a warning, rather than a CSV-cell syntax error. Row-width drift is also a warning
under the standard CSV package. The sample preserves those native severities.

Inspection presents the imported CEM generic-data tree, rather than a lossless
CSV-language AST. Its envelope identifies the CEM projection. Source start
offsets still refer to original CSV bytes: `SKU-A` starts at byte 32 in the
header-bearing source; the quoted café name starts at byte 38. The corresponding
headerless starts are 0 and 6. All input parsing, field decoding, diagnostics,
queries and serialization stay in the native shared engine.

## Numeric rules and report artifacts

The native low-stock expression explicitly converts quantity and reorder level
to integers and checks each conversion against the exact original text. Only
canonical non-negative decimal integers participate in its numeric comparison.
This avoids ordering `12` and `3` lexically or silently treating `many` as zero.
Leading zeros, surrounding spaces, negative values and decimal fractions are not
canonical counts under this authored inventory rule, even when valid CSV text.

The report model includes valid rows whose quantity is less than their reorder
level. It also includes rows with invalid counts so they remain visible for
repair: `status` is `invalid-count`, and both original count texts remain strings.
Valid low-stock rows have `status` `low-stock` and numeric JSON count fields.
The standalone selection query lists only valid low-stock SKUs; it does not return
the separate invalid-count records. Source validation and this reporting rule
are different operations.

Run **Build low-stock inventory**:

- `generated/low-stock.html` is the explicit entry artifact. Its table initially
  contains SKU-A and SKU-C and uses the heading **Low-stock inventory**.
- `generated/low-stock.json` is an array of those records, suitable for Studio's
  shared table preview. It preserves typed numbers for valid counts.
- `expected/low-stock.html` and `.json` define the baseline publications.

Native CLI equivalent, run from the repository root:

```sh
cem-ml transform --config examples/projects/csv-inventory/graphs/inventory.cem \
  --config-content-type application/cem \
  --config-schema https://cem.dev/ns/cli/transform-config/1 \
  --scope-budget queueSize=1024
```

Direct selected-record CSV export from this constructed CEM report model is not
currently available through the graph exporter. The project therefore publishes
HTML and JSON reports, and exposes native compact CSV formatting separately for
the complete original inventory. It does not implement a CSV serializer in a
template or JavaScript. Compact output uses LF records by default. For a receiver
that requires CRLF interchange, the native conversion accepts
`--cemt-formatter-option lineEnding=crlf`. `tabular` is for human review, not
machine interchange or lossless import after padding/trimming.

## Saved edits

In a copy, change SKU-A's quantity from `3` to `8`, save and validate, and rerun the
build. Only SKU-C remains in both report artifacts. A second copy still reports
both original low-stock rows. Reset to `3` to restore the baseline.

Change the quantity to `many`, save and build. CSV syntax validation still passes,
but SKU-A is now visibly flagged as `invalid-count`; its count fields are strings
in JSON. Repair to `3` and rebuild. The native rule also guards values such as
`03`, `-1` and `3.5` instead of depending on permissive numeric coercion.

Unsaved drafts are not saved build inputs. The normal save/discard/cancel guard
applies, and saved edits survive offline reopening. All source files, templates,
queries, configuration documents and expected results belong to the project
archive. With the default complete deployment, rebuilding needs no remote project
data. In the optional WASM deployment, the host first admits the shipped native
CSV package assets under host authority; cached package assets then allow offline
rebuilding. The project cannot grant itself package activation authority.

## Diagnostics and exact repairs

The malformed source resources are separate teaching inputs, leaving the original
report inputs valid. All repairs use the ordinary source or configuration editor.

| Configuration / resource | Native result | Exact repair |
| --- | --- | --- |
| **Repair declared header policy** / `config/header-policy.json` | `cem.csv.invalid_header_parameter`, warning, successful exit. The warning concerns input metadata and has no CSV row/cell position. | Replace `header=maybe` with `header=present`, save, then run this configuration again. |
| **Repair an unclosed quoted field** / `data/unclosed-quote.csv` | `cem.csv.unclosed_quote`, error at line 2, column 21, byte 52. | Replace `3,"5` with `3,"5"`, save and rerun. |
| **Repair an unescaped field quote** / `data/invalid-escape.csv` | `cem.csv.invalid_quote_escape`, error at line 2, column 11, byte 42. | Replace `Tea "travel pot` with `"Tea ""travel"" pot"`, save and rerun. |
| **Repair an incomplete inventory row** / `data/ragged-rows.csv` | `cem.csv.inconsistent_field_count`, warning for line 2 at byte 32. No column is reported. | Replace `SKU-A,"Café cups, set",3` with `SKU-A,"Café cups, set",3,5`, save and rerun. |

For the header case, **Save and validate** checks the configuration document's own
source contract. Run **Repair declared header policy** to execute its declared
CSV input validation. `inputs[0].uri` is `data/inventory.csv`, relative to the
project root; the native configuration declares its input identity independently
of the inventory resource's default metadata.

A ragged row's missing reorder value is not silently invented by the report:
if used as the main inventory input, it appears as `invalid-count`. A quote error
in the main saved inventory stops native import and publishes no new successful
report. Repair and rebuild to publish again. Earlier successful native files
remain earlier outputs; they are not results of a failed run.

## Graph, provenance and related projects

The shared graph shows the CSV import, model/view stages and both exports.
The captured build input identities include the inventory, graph and templates,
regardless of the current editor selection. HTML source-map ranges link to the
authored instructions in `templates/view.cemt`; follow and edit its explanatory
sentence to see a new publication and source range. These template ranges are
separate from the CSV source field offsets and do not claim that every generated
HTML character traces directly to a CSV cell.

For a related cross-format workflow, open **Supplier pipeline**. It reconciles
CSV stock and an XML catalog using explicit native domain rules and publishes
HTML, JSON and YAML. This project concentrates on CSV header/quoting diagnostics,
string-cell import and inventory filtering instead of duplicating that join.

Existing Studio credential privacy and recovery budgets are unchanged. Entered
passwords or one-time codes block recovery until the user clears them; automatic
whole-Studio recovery remains disabled.
