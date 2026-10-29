# SVG diagram

Inspect an SVG diagram and include it in a generated page. Query its shapes and labels, edit the drawing and repair a missing namespace.

## Essential files

- [data/diagram.svg](data/diagram.svg) — The SVG drawing to inspect and edit.
- [queries/diagram.xpath](queries/diagram.xpath) — Reads the shapes and labels.
- [templates/diagram.cemt](templates/diagram.cemt) — Places the diagram on a page.
- [graphs/diagram.cem](graphs/diagram.cem) — Builds the diagram page.
- [failures/missing-namespace.svg](failures/missing-namespace.svg) — A drawing with a missing namespace to repair.

## Using the example

Open **Examples → Content types and schemas → Documents and UI → SVG diagram**.
The same ordinary project also appears under **Queries and transformations**.
Clone it to edit saved resources without changing the catalog original or another
copy. Its purpose is a small accessible vector diagram for documentation and
publishing, using native source processing and the shared output preview.

## Sources and configurations

`data/diagram.svg` declares `image/svg+xml` with schema
`https://cem.dev/ns/data/svg/1` and the SVG namespace
`http://www.w3.org/2000/svg`. It contains an accessible title/description, two
rectangles, a circle, two connecting lines and three text labels. Literal
geometry and `currentColor` paint keep it independent of Studio theme keys.

The remaining declared resource roles in [example.json](example.json) are:

| Resources | Content type | Schema identity |
| --- | --- | --- |
| `queries/diagram.xpath` | `application/vnd.cem.xpath` | `https://cem.dev/ns/query/xpath/1` |
| `templates/diagram.cemt` | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` |
| `graphs/diagram.cem` | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| `expected/summary.json` | `application/json` | `https://cem.dev/ns/data/json/1` |
| `expected/diagram.html` | `text/html` | `https://cem.dev/ns/data/html/1` |

The invalid SVG and expected formatted SVG retain the SVG source identity above.
Query/template/configuration declarations and reference outputs have separate
roles; their presence does not promise every standalone source operation.

| Configuration | Typical use | Expected result |
| --- | --- | --- |
| `inspect-svg` — Inspect retained SVG diagram | Review native elements, attributes and source ranges | SVG expanded names and original byte positions |
| `validate-svg` — Validate SVG source | Check XML and supported SVG source contracts | Zero errors and warnings |
| `query-svg` — Query the SVG diagram structure | Read a diagram summary with namespace-aware XPath | Seven strings matching `expected/summary.json` |
| `format-svg` — Format SVG (pretty) | Export a readable same-schema SVG | Exact bytes in `expected/diagram.svg` |
| `repair-svg-namespace` — Repair the SVG namespace | Diagnose well-formed XML with the wrong document identity | `cem.svg.namespace_missing`, then zero errors after repair |
| `build-html` — Build and preview the diagram | Publish an HTML page containing the retained SVG subtree | `generated/diagram.html`, matching `expected/diagram.html` |

The query binds `s=http://www.w3.org/2000/svg`. Namespace URI and local name
determine selection; producer prefix spelling does not. The binding belongs to
execution: standalone XPath source validation currently does not inherit caller
namespace bindings, and standalone XPath Parse/Inspect/format are not
configurations here. The query summary is:

```text
title=A small transformation pipeline
view-box=0 0 320 160
rectangles=2
circles=1
lines=2
first-label=Source
first-width=80
```

## Edit and build

In an editable copy, change the first text label **Source** to **Input**, save,
and rerun the query and build. The summary and visible diagram change. Change
the first rectangle's `width="80"` to `width="72"` to observe a geometry change;
the SVG grammar preserves the authored value, while browser rendering determines
the displayed shape. Unsaved drafts do not replace persisted command inputs.

`graphs/diagram.cem` imports the SVG using its own family identity, calls the
public `main` template in `templates/diagram.cemt`, and exports the explicit HTML
entry artifact. The CEMT template constructs the heading/caption and inserts the
retained `svg` element with its namespace and children. The XML declaration is
source material, not HTML body content, and is omitted by that selection.
This is an authored wrapper, not an implicit universal SVG-to-HTML converter.

Select the project and choose **Build project**. The shared artifact preview
admits this bounded inert SVG vocabulary inside the generated HTML. It does not
grant script, resource loading, arbitrary CSS, animation or foreign-object
execution. Standalone SVG conversion remains available as exact downloadable
bytes through **Format SVG (pretty)**.

For source navigation, run **Build and preview the diagram**, then open
**Commands and results → Results → Provenance**. Choose a **Source-map
provenance** row to open the original SVG or template range where the native
operation supplies one. Copied native nodes retain their source boundaries;
the caption and constructed HTML retain template origins. Generated indentation
and computed strings do not acquire invented character mappings.
`inspect-svg` prints original coordinates but currently publishes no owned
artifact-source references to the Provenance panel. Use the build output's maps
or native diagnostics; numeric inspection fields are not navigation authority.

Native build, from this project directory:

```sh
cem-ml transform --config graphs/diagram.cem --config-content-type application/cem \
  --config-schema https://cem.dev/ns/cli/transform-config/1 --scope-budget queueSize=1024
```

## Namespace failure and repair

`failures/missing-namespace.svg` is deliberately outside the graph inputs. Run
**Repair the SVG namespace**: its `svg` root has no namespace, producing one
`cem.svg.namespace_missing` at line 2, column 1, original byte offset 39.
Replace the opening `<svg ` with
`<svg xmlns="http://www.w3.org/2000/svg" `, save and rerun. Validation reports
zero errors and warnings; the repair restores the original valid source bytes.

## Supported boundaries and independent copies

The native SVG package validates XML and its documented root/namespace, viewport,
accessibility and policy facts. It does not establish full SVG vocabulary,
geometry, path-data, paint, CSS, animation or filter conformance. There is one
registered SVG media/schema identity; this project does not claim separate SVG
1.1 or SVG 2 conformance profiles. See the
[SVG package](../../../packages/cem_ml/schema-packages/svg/v1/README.md).

The same-schema formatter performs structural layout while preserving sensitive
text, lexical tokens and source-map boundaries. Its `pretty` profile is not XML
canonicalization. Scripts, handlers, DTDs, non-built-in entities, unadmitted URLs
and foreign content retain their existing refusal policies. Native validation
and the narrower preview vocabulary are separate contracts.

Once Studio's runtime and project resources are cached, saved copies reopen and
rebuild offline. Data, commands, templates and drafts remain project-local; the
catalog original stays unchanged. No JavaScript, external resources or storage
keys belong to this example. Entered passwords and one-time codes elsewhere
block recovery until explicitly cleared; automatic whole-Studio recovery remains
disabled.
