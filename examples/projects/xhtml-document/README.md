# XHTML document

Inspect and query an XHTML article, then build a page from it. The example also shows how a missing namespace changes which source format is accepted.

## Essential files

- [data/article.xhtml](data/article.xhtml) — The original XHTML article.
- [queries/outline.xpath](queries/outline.xpath) — Reads the article heading and structure.
- [templates/view.cemt](templates/view.cemt) — Builds the article view.
- [graphs/article.cem](graphs/article.cem) — Connects the article to the view template.
- [data/missing-namespace.xhtml](data/missing-namespace.xhtml) — An XHTML-looking document with its namespace removed.

## Using the example

Use this ordinary Studio project for XML-serialized web documents, publishing
pipelines and namespace-aware content extraction. It keeps the original XHTML
source, query, template and build configuration as editable resources.

For XML business data and cross-source reconciliation, use the
[Supplier pipeline](../supplier-pipeline/README.md). That project imports the
existing supplier catalog and stock feed; this example does not copy them.

## Resources and identities

- `data/article.xhtml` is `application/xhtml+xml` with schema
  `https://cem.dev/ns/data/xhtml/1`. Its `html`, `head`, `body` and article content
  use `http://www.w3.org/1999/xhtml`.
- The embedded SVG has its own `http://www.w3.org/2000/svg` namespace and a
  `title` element. Its title is distinct from the XHTML document title.
- `queries/outline.xpath` is `application/vnd.cem.xpath` with schema
  `https://cem.dev/ns/query/xpath/1`. The command explicitly binds
  `xhtml=http://www.w3.org/1999/xhtml`; source prefix spelling is independent of
  the query prefix.
- `templates/view.cemt` declares `application/vnd.cem.template+cem` and
  `https://cem.dev/ns/template/cem-native/1`. The native CEMT template selects
  XHTML elements by expanded name from the shared retained tree and constructs
  an HTML article.
- `graphs/article.cem` declares `application/cem` and
  `https://cem.dev/ns/cli/transform-config/1`. The version-1 native transformation
  configuration imports XHTML, applies the template and exports `generated/article.html` as
  `text/html` with schema `https://cem.dev/ns/data/html/1`.
- `expected/outline.json` declares `application/json` with
  `https://cem.dev/ns/data/json/1`; it records the explicit query result contract.

The XHTML-to-HTML mapping is authored explicitly. XHTML source identity is
preserved during import; the native shared tree is the input to both query and
template processing.
The configured `xhtml` binding reaches query execution. Standalone XPath source
validation currently does not inherit caller namespace bindings; standalone
XPath Parse/Inspect/format are not configurations in this project.

## Configurations

| Configuration | Typical use | Expected result |
| --- | --- | --- |
| `inspect-xhtml` — Inspect retained XHTML structure | Review source nodes, namespaces and positions | A CEM-ML structural projection containing XHTML and SVG names |
| `validate-xhtml` — Validate XHTML source | Check XML syntax and supported XHTML document structure | Zero errors and warnings |
| `query-outline` — Select the namespaced document outline | Extract a publishing summary with XPath | Four strings matching `expected/outline.json` |
| `repair-namespace` — Repair the XHTML root namespace | Diagnose an XML document that is not XHTML | `cem.xhtml.namespace_missing`; zero errors after the repair below |
| `build-html` — Build the XHTML article as HTML | Publish selected document content | `generated/article.html`, matching `expected/article.html` |

The outline is:

```text
title=Shared document workflows
heading=An XHTML field guide
paragraphs=2
links=1
```

The foreign SVG title does not participate. The built article contains the XHTML
title as its accessible label, its heading and its two paragraphs. It deliberately
selects those fields rather than copying the whole source page.

## Failure and repair

Run **Repair the XHTML root namespace** against
`data/missing-namespace.xhtml`. It is well-formed XML with an `html` root, but the
root has no XHTML namespace. The native validator reports
`cem.xhtml.namespace_missing` at line 2, column 1, byte offset 39.

Clone the example to edit it, then replace:

```xml
<html lang="en">
```

with:

```xml
<html xmlns="http://www.w3.org/1999/xhtml" lang="en">
```

Save the resource and rerun the same configuration. It now reports zero errors
and warnings. This is a source repair in an ordinary project, without a dedicated
tutorial screen or application-specific parser.

## Editing, builds and provenance

Open the project to build it, or select `data/article.xhtml` in Structure to edit
the source. Clone first to keep the catalog example read-only. Change the heading,
save and rerun **Build the XHTML article as HTML**. Select
`generated/article.html` in Output files to use the shared HTML preview.

The output artifact carries its original input resource identities and a source
map. Generated ranges point to the authored instructions and expressions in
`templates/view.cemt`; computed text does not imply arbitrary character-by-character
lineage to the input. Inspect the artifact's provenance in Studio to navigate
back to those resources.
The source coordinates printed by `inspect-xhtml` do not themselves populate
owned artifact-source references in the current Parse/Inspect Provenance panel.
Use the build's actual output maps or native diagnostics for source navigation.

Saved copies keep their own source and configuration edits. Once Studio's runtime
and project resources are cached, reopen the saved copy offline and rebuild it.
The original example and another saved copy remain independent. This example
contains no credential controls; Studio's existing rule still blocks recovery
while a password or one-time code is entered elsewhere.
Automatic whole-Studio recovery remains disabled. The project adds no local
JavaScript behavior or credential storage.

The same build can run natively from the repository:

```sh
cem-ml transform --config examples/projects/xhtml-document/graphs/article.cem \
  --scope-budget queueSize=1024
```

## Supported boundaries

XHTML uses the XML parser and a distinct XHTML schema contract. Current source
validation covers XML well-formedness, namespace binding, the XHTML root and
ordered `head`/`body` structure. It does not establish complete HTML vocabulary
or content-model validity, arbitrary XSD/RELAX NG instance validity, or semantic
validity of embedded SVG. Those require their own supported contracts.

Native XHTML `compact`, `pretty` and `tabular` formatting currently preserves
source lexemes; it does not reflow mixed content. DTDs and non-built-in entities
are rejected without external resolution. The generated HTML is a separate,
explicit output identity, and the preview follows the shared script-free
presentation policy. All example data is fictional.
