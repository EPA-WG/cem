# Supplier pipeline

Open **Supplier pipeline** from **Example projects**, choose **Create project from
example**, then open the resulting project. It is an ordinary editable project
in the existing Studio workspace. The catalog original remains unchanged; each
copy owns its resources, configurations and saved revisions independently.

## Resources and configurations

- `data/catalog.xml` declares four products in the `urn:demo:supplier` vocabulary.
- `data/stock.csv` supplies product IDs and quantities. Its explicit
  `text/csv;header=present` identity keeps cells as strings at import.
- `templates/reconcile.cemt` matches product IDs across the two retained native
  trees and constructs a typed `cem:generic-data` result.
- `templates/view.cemt` composes that same result into a readable HTML view.
- `graphs/availability.cem` declares the named `stock` input and explicit CEM
  result identity. It branches to the view, a JSON service payload and a YAML
  partner feed. Graph joins do not perform business-record matching.
- `expected/availability.json`, `.yaml` and `.html` state the baseline results.
- **Inspect supplier catalog** and **Inspect supplier stock** parse saved source
  through the native adapters. **Supplier availability pipeline** executes the
  saved graph and all saved dependencies.

Run **Supplier availability pipeline**. The operation output previews the HTML
view; the **Transformation graph** result includes all three named outputs.
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
