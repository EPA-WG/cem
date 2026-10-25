# CEM transformation

This ordinary Studio project transforms an owned XML order document into an
application-data report and HTML. Open it from **Content types and schemas →
Queries and transformations**, then clone it to edit. The related **XSLT
transformation** project uses the same original data and produces the same HTML
through a bounded native XSLT report stage and a CEM-native view.

## Resources and configurations

- `orders-source` — `data/orders.xml`, `application/xml`,
  `https://cem.dev/ns/data/xml/1`. Three orders include ready and draft statuses,
  customer labels and numeric totals. UTF-8 `Café Noord` is intentional.
- `report-template` — `templates/report.cemt`,
  `application/vnd.cem.template+cem`, `https://cem.dev/ns/template/cem-native/1`.
  The declared `minimum` parameter filters ready orders into a native
  `cem:generic-data` array of records.
- `page-template` — `templates/page.cemt`, the same native template identity.
  The declared `heading` parameter labels an HTML view of that report model.
- `report-graph` — `graphs/report.cem`, `application/cem`,
  `https://cem.dev/ns/cli/transform-config/1`. It binds both parameters and names
  the intermediate JSON artifact and final HTML entry.
- `invalid-template` and `invalid-orders` are independent failure resources.
  Expected artifacts and formatter bytes are ordinary project resources too.

| Configuration | Typical use |
| --- | --- |
| `validate-template` | Check the template source contract without running it. |
| `inspect-template` | Inspect the native source tree and original ranges. |
| `format-template` | Compare native tabular formatting with exact expected source bytes. |
| `validate-graph` | Check the graph source, declared identities, parameters and exports. |
| `inspect-graph` | Inspect the two transformation stages and explicit export declarations. |
| `repair-template` | Diagnose and repair the invalid boolean parameter declaration. |
| `build-report` | Execute the native graph and preview its explicit HTML entry. |

The graph imports the original retained XML owner, filters ready orders whose
numeric total is at least thirty, constructs a native application-data model,
then renders the model using CEM-ML/CEM-QL in the native template runtime. The
intermediate stage uses `application/cem` with the CEM-ML schema so its constructed
CEM-tree sequence remains available to the next stage. The application JSON export
is a presentation artifact; it is never serialized and reimported to pass an AST
between stages.

## Build artifacts and provenance

`build-report` explicitly selects `generated/index.html` as its preview entry. It
contains two list items, `O-101` and `O-103`, under **Ready orders**.
`generated/report.json` contains the same two selected records and excludes the
draft order. Both artifacts have independently checked native CLI fixtures.

The HTML sidecar and output spans retain original `templates/page.cemt` source
boundaries. They describe template output provenance; this project does not claim
character-level lineage from computed output fields back into XML attributes.
The native generic-data JSON writer currently emits no output spans or `.map`
sidecar. CLI JSON is indented while the worker's abstract JSON publication is
compact; cross-host acceptance compares complete JSON values. Unchanged repeated
builds within either host retain the same bytes. Source formatter and HTML
fixtures remain byte-exact. Trailing HTML layout whitespace is native output.

The native `inspect` JSON is a read-only source projection. Execution continues
through original native source and retained owners, without JSON AST handoffs.

## Failure and repair

`repair-template` validates `failures/report.cemt`, whose input declaration has
`@required=maybe`. It produces one error,
`cem.schema_model.invalid_attribute_type`. Restore `@required=true` and validate
again. The native diagnostic currently reports top-level line 1, column 1, byte 0;
its detailed source-map/range entries identify the original parameter attribute.
The original catalog failure resource remains unchanged after a saved-copy repair.

Replacing saved `data/orders.xml` with the malformed `failures/orders.xml` makes
`build-report` fail without publishing new artifacts. Studio retains the previous
successful preview. Restore the valid XML and rebuild.

## Saved changes and independent copies

Change graph `{param @name=minimum @value=30}` to `@value=45`: only `O-101`
remains. Change `Café Noord` to `Café Haven` in the XML: both HTML and JSON reflect
the saved label. Change `@name=ul` to `@name=ol` in `templates/page.cemt`: only the
HTML presentation changes. Change graph **Ready orders** to **Dispatch queue** to
exercise the independent heading parameter.

Clone before editing. Unsaved source edits do not replace persisted command
inputs. A second copy can change parameters and templates independently while
its original and the read-only catalog remain unchanged. Reloading a saved copy
offline reuses persisted resources and the shared native command-service runtime.
No page-specific JavaScript, query parser or preview control is introduced.

Credential policy is unchanged: entered passwords and one-time codes block
recovery until explicitly cleared. Automatic whole-Studio recovery stays disabled.
