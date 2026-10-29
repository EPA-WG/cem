# Schema examples and portable operations

Open a working file in any of the supported content types and see what CEM can check, inspect or transform. Compare the formats side by side, or clone the project to try your own changes.

## Essential files

Each link opens the file in this project. The served filenames below are relative
to Studio's `samples/feature-tour/` directory; saved copies keep their own resource
bytes and do not depend on those download addresses. Binary projections stay
available as exact downloads when a text view is not appropriate.

| Project file | Served file | Purpose |
| --- | --- | --- |
| [data/cem-ast-projection/basic-ast.cem-bin](data/cem-ast-projection/basic-ast.cem-bin) | `resources/cem-ast-projection-basic-ast.cem-bin` | Inspect a saved syntax tree, including its source structure. |
| [data/cem-data-island/complete-instance.cem](data/cem-data-island/complete-instance.cem) | `resources/cem-data-island-complete-instance.cem` | Inspect saved component data and instance state. |
| [data/cem-dom-projection/basic-dom.cem-bin](data/cem-dom-projection/basic-dom.cem-bin) | `resources/cem-dom-projection-basic-dom.cem-bin` | Inspect a saved document tree without reconstructing source text. |
| [data/cem-element-template/basic-card.cem](data/cem-element-template/basic-card.cem) | `resources/cem-element-template-basic-card.cem` | Read a declarative card component and its template. |
| [data/cem-events-projection/basic-events.cem-bin](data/cem-events-projection/basic-events.cem-bin) | `resources/cem-events-projection-basic-events.cem-bin` | Inspect a saved stream of document events. |
| [data/cem-ml/basic.cem](data/cem-ml/basic.cem) | `resources/cem-ml-basic.cem` | Start with a small CEM document. |
| [data/cem-native-template/basic-template.cem](data/cem-native-template/basic-template.cem) | `resources/cem-native-template-basic-template.cem` | Read a native template that produces document content. |
| [data/cem-ql/basic-query.cemql](data/cem-ql/basic-query.cemql) | `resources/cem-ql-basic-query.cemql` | Try a small query written in CEM-QL. |
| [data/cem-transform/basic-transform.cemt](data/cem-transform/basic-transform.cemt) | `resources/cem-transform-basic-transform.cemt` | Read a transformation template. |
| [data/cli-command/parse-ast.json](data/cli-command/parse-ast.json) | `resources/cli-command-parse-ast.json` | Inspect a saved command for parsing a source file. |
| [data/css/basic-stylesheet.css](data/css/basic-stylesheet.css) | `resources/css-basic-stylesheet.css` | Inspect a basic stylesheet and its rules. |
| [data/css-selector/basic-selector.css-selector](data/css-selector/basic-selector.css-selector) | `resources/css-selector-basic-selector.css-selector` | Read a selector that finds document elements. |
| [data/csv/basic-table.csv](data/csv/basic-table.csv) | `resources/csv-basic-table.csv` | Inspect a small table stored as CSV. |
| [data/html/basic-document.html](data/html/basic-document.html) | `resources/html-basic-document.html` | Inspect a complete HTML document. |
| [data/json/basic-object.json](data/json/basic-object.json) | `resources/json-basic-object.json` | Inspect an object with structured JSON values. |
| [data/json-schema/basic-schema.json](data/json-schema/basic-schema.json) | `resources/json-schema-basic-schema.json` | Read rules describing a JSON object. |
| [data/markdown/basic-document.md](data/markdown/basic-document.md) | `resources/markdown-basic-document.md` | Preview headings and paragraphs written in Markdown. |
| [data/mathml/basic-presentation.mml](data/mathml/basic-presentation.mml) | `resources/mathml-basic-presentation.mml` | Inspect a mathematical formula written in MathML. |
| [data/module-map/source-modules.json](data/module-map/source-modules.json) | `resources/module-map-source-modules.json` | Read version 1 module name and file mappings. |
| [data/module-map-v2/source-modules-and-resources.json](data/module-map-v2/source-modules-and-resources.json) | `resources/module-map-v2-source-modules-and-resources.json` | Read version 2 mappings for modules and other resources. |
| [data/module-map-v3/source-worker-safe-modules.json](data/module-map-v3/source-worker-safe-modules.json) | `resources/module-map-v3-source-worker-safe-modules.json` | Read version 3 mappings with worker-safe modules. |
| [data/relax-ng/basic-schema-xml.rng](data/relax-ng/basic-schema-xml.rng) | `resources/relax-ng-basic-schema-xml.rng` | Read an XML grammar written in RELAX NG. |
| [data/schema/basic-schema.cem](data/schema/basic-schema.cem) | `resources/schema-basic-schema.cem` | Read a schema that defines a CEM vocabulary. |
| [data/schema-package/basic-package.cem](data/schema-package/basic-package.cem) | `resources/schema-package-basic-package.cem` | Read the manifest that groups a schema with its examples. |
| [data/scss/basic-source.scss](data/scss/basic-source.scss) | `resources/scss-basic-source.scss` | Inspect stylesheet variables and other SCSS source features. |
| [data/studio-project/feature-tour-cem.cem](data/studio-project/feature-tour-cem.cem) | `resources/studio-project-feature-tour-cem.cem` | Read a portable Studio project description. |
| [data/svg/basic-icon.svg](data/svg/basic-icon.svg) | `resources/svg-basic-icon.svg` | Inspect an SVG icon and its drawing instructions. |
| [data/xhtml/basic-document.xhtml](data/xhtml/basic-document.xhtml) | `resources/xhtml-basic-document.xhtml` | Inspect an HTML document written as namespace-aware XML. |
| [data/xml/basic-document.xml](data/xml/basic-document.xml) | `resources/xml-basic-document.xml` | Inspect a small XML document and its elements. |
| [data/xpath/basic-path.xpath](data/xpath/basic-path.xpath) | `resources/xpath-basic-path.xpath` | Read a path expression that selects document nodes. |
| [data/xslt/basic-stylesheet.xsl](data/xslt/basic-stylesheet.xsl) | `resources/xslt-basic-stylesheet.xsl` | Read an XSLT stylesheet that transforms XML. |
| [data/yaml/basic-document.yaml](data/yaml/basic-document.yaml) | `resources/yaml-basic-document.yaml` | Inspect a small YAML document with named values. |
| [workbenches/input.cem](workbenches/input.cem) | `workbenches/input.cem` | A small CEM input for the portable operations. |
| [workbenches/data.xml](workbenches/data.xml) | `workbenches/data.xml` | XML data for the portable query and transformation. |
| [workbenches/template.xpath](workbenches/template.xpath) | `workbenches/template.xpath` | The XPath transformation used by the portable build. |
| [workbenches/graph.cem](workbenches/graph.cem) | `workbenches/graph.cem` | Connects the portable import, transformation and export. |

## Using the showcase

Use this ordinary project to compare native source validation across the
registered schema packages, then inspect five small portable command workflows.
The generated **Schema examples** catalog project and legacy **CEM-ML Feature
Tour** seed contain the same instructions. They use the normal Studio resource
editor, configurations and result viewers; there are no numbered checkpoint pages
or project-specific UI behaviors.

Original examples are read-only. Clone the project to edit text resources and
save configurations. Existing saved projects and drafts are independent of
catalog updates; cloning or updating this template does not overwrite them.

## Resources and selected source identities

`README.md` is a `text/markdown; charset=utf-8` resource with schema
`https://cem.dev/ns/data/markdown/1`. Each package contributes its first
manifest-declared passing source. The table records that selected identity, not
all aliases or every operation supported by the language.

| Configuration | Source resource | Content type | Schema |
| --- | --- | --- | --- |
| `validate-cem-ast-projection` | `example-cem-ast-projection` | `application/vnd.cem.ast+cem-bin` | `https://cem.dev/ns/projection/ast/1` |
| `validate-cem-data-island` | `example-cem-data-island` | `application/vnd.cem.data-island+cem` | `https://cem.dev/ns/runtime/data-island` |
| `validate-cem-dom-projection` | `example-cem-dom-projection` | `application/vnd.cem.dom+cem-bin` | `https://cem.dev/ns/projection/dom/1` |
| `validate-cem-element-template` | `example-cem-element-template` | `application/vnd.cem.element-template+cem` | `https://cem.dev/ns/template/cem-element/1` |
| `validate-cem-events-projection` | `example-cem-events-projection` | `application/vnd.cem.events+cem-bin` | `https://cem.dev/ns/projection/events/1` |
| `validate-cem-ml` | `example-cem-ml` | `application/cem` | `https://cem.dev/ns/cem-ml/1` |
| `validate-cem-native-template` | `example-cem-native-template` | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` |
| `validate-cem-ql` | `example-cem-ql` | `application/vnd.cem.query+cem-ql` | `https://cem.dev/ns/query/cem-ql/1` |
| `validate-cem-transform` | `example-cem-transform` | `application/vnd.cem.transform+cem` | `https://cem.dev/ns/transform/cem/1` |
| `validate-cli-command` | `example-cli-command` | `application/vnd.cem.cli-command+json` | `https://cem.dev/ns/cli/command/1` |
| `validate-css` | `example-css` | `text/css` | `https://cem.dev/ns/data/css/1` |
| `validate-css-selector` | `example-css-selector` | `application/vnd.cem.query-expression+css-selector` | `https://cem.dev/ns/query/css-selector/1` |
| `validate-csv` | `example-csv` | `text/csv` | `https://cem.dev/ns/data/csv/1` |
| `validate-html` | `example-html` | `text/html` | `https://cem.dev/ns/data/html/1` |
| `validate-json` | `example-json` | `application/json` | `https://cem.dev/ns/data/json/1` |
| `validate-json-schema` | `example-json-schema` | `application/schema+json` | `https://cem.dev/ns/data/json-schema/1` |
| `validate-markdown` | `example-markdown` | `text/markdown; charset=utf-8; variant=CommonMark` | `https://cem.dev/ns/data/markdown/1` |
| `validate-mathml` | `example-mathml` | `application/mathml+xml` | `https://cem.dev/ns/data/mathml/1` |
| `validate-module-map` | `example-module-map` | `application/vnd.cem.module-map+json` | `https://cem.dev/ns/data/module-map/1` |
| `validate-module-map-v2` | `example-module-map-v2` | `application/vnd.cem.module-map+json` | `https://cem.dev/ns/data/module-map/2` |
| `validate-module-map-v3` | `example-module-map-v3` | `application/vnd.cem.module-map+json` | `https://cem.dev/ns/data/module-map/3` |
| `validate-relax-ng` | `example-relax-ng` | `application/relax-ng+xml` | `https://cem.dev/ns/data/relax-ng/1` |
| `validate-schema` | `example-schema` | `application/vnd.cem.schema+cem` | `https://cem.dev/ns/schema/1` |
| `validate-schema-package` | `example-schema-package` | `application/vnd.cem.schema-package+cem` | `https://cem.dev/ns/schema-package/1` |
| `validate-scss` | `example-scss` | `text/vnd.cem.scss` | `https://cem.dev/ns/data/scss/1` |
| `validate-studio-project` | `example-studio-project` | `application/vnd.cem.studio-project+cem` | `https://cem.dev/ns/studio/project/1` |
| `validate-svg` | `example-svg` | `image/svg+xml` | `https://cem.dev/ns/data/svg/1` |
| `validate-xhtml` | `example-xhtml` | `application/xhtml+xml` | `https://cem.dev/ns/data/xhtml/1` |
| `validate-xml` | `example-xml` | `application/xml` | `https://cem.dev/ns/data/xml/1` |
| `validate-xpath` | `example-xpath` | `application/vnd.cem.xpath` | `https://cem.dev/ns/query/xpath/1` |
| `validate-xslt` | `example-xslt` | `application/xslt+xml` | `https://cem.dev/ns/transform/xslt/1` |
| `validate-yaml` | `example-yaml` | `application/yaml` | `https://cem.dev/ns/data/yaml/1` |

The validation resources retain their original bytes and dependencies. Their
`config/<package>.validate.json` resources use `application/json` and schema
`https://cem.dev/ns/cli/run-config/1`. The catalog project stores the five portable
workbench commands in versioned `application/vnd.cem.cli-command+json` resources
with schema `https://cem.dev/ns/cli/command/1`; the legacy seed keeps its existing
run-configuration representation and supplies the same authored command arguments.

There are 32 source-validation configurations and five portable workbench
configurations. The two grouping entries are folders, not extra operations.
Native validation of a manifest's passing example establishes that selected
source contract. It does not establish instance validation, complete language
conformance, all registered MIME aliases, fetching, execution or safe hydration.
Module-map source checks do not execute deployment semantics. JSON Schema and
RELAX NG source checks do not validate their example data against those schemas.
The data-island example is source-validation evidence, not permission to resume
an arbitrary runtime or replay its state.

The three native projection examples are binary inputs. Their header validators
are covered by native fixtures; Studio keeps binary resources read-only and
non-runnable with exact-byte downloads, including in saved copies. A declared
configuration is not a promise that binary-input execution is enabled in the UI.
Successful binary-header validation does not establish payload integrity or
projection replay.

## Portable command configurations and expected results

| Configuration | Input and dependencies | Explicit result | Expected summary |
| --- | --- | --- | --- |
| `workbench-conversion` | `operation-input`, CEM-ML `workbenches/input.cem` | Named DOM JSON projection, requested with `dom-json`; not an internal AST handoff. | `expected/conversion.json`: kind `convert`, one output. |
| `workbench-query` | `operation-data`, XML `workbenches/data.xml` | Explicit XPath JSON result for `//item`. | `expected/query.json`: kind `query`, two items. |
| `workbench-transformation` | `operation-data` and `operation-template`, XPath `workbenches/template.xpath` | Direct XPath transformation with an explicitly declared `application/vnd.cem.xpath-result+json` target. | `expected/transformation.json`: kind `transform`, direct mode, two items. |
| `workbench-trace` | `operation-input`, CEM-ML `workbenches/input.cem` | Explicit parser/validation trace JSON. | `expected/trace.json`: kind `trace`, with events. |
| `workbench-transformation-graph` | `operation-graph`, XML data and XPath template | Native graph exports `graph-output.json` as `application/vnd.cem.xpath-result+json`. | `expected/transformation-graph.json`: graph mode, one artifact and two items. |

The workbench CEM-ML source uses `application/cem` with schema
`https://cem.dev/ns/cem-ml/1`; XML uses `application/xml` with
`https://cem.dev/ns/data/xml/1`. The XPath template uses
`application/vnd.cem.xpath` with `https://cem.dev/ns/query/xpath/1`. The graph
resource declares `application/vnd.cem.transform+cem` with
`https://cem.dev/ns/transform/cem/1`. Expected JSON summaries use
`application/json` with `https://cem.dev/ns/data/json/1`; these are bounded result
summaries, not golden copies of every serialized artifact or an imported AST.

Run one configuration at a time and review native diagnostics and operation
output. The graph and direct transformation use the same small two-item XML
source. These workflows exercise their actual named commands; command availability
alone does not imply arbitrary source Parse, formatting, query or conversion
support. Native `check` and capability/version metadata are separate service
boundaries, not extra configurations in this project.

## Source locations and provenance

Native readers retain the source owner and original ranges. Human structural
presentation defaults to native CEM-ML; the portable workbench JSON boundaries
above are explicit. Stored projection fields containing byte ranges or numeric
source IDs are data and do not recreate their original source owner.

Studio follows only the source identities and native ranges actually published
by a command. Parse/Inspect can present coordinates without publishing owned
artifact-source references to the Provenance panel. Query command provenance
may describe the query source rather than a fine mapping to input data. Do not
infer an input-character mapping from a JSON result or an expected summary.

## Failure and repair

This project starts with passing source fixtures; it has no dedicated authored
failure/repair configurations. To explore a diagnostic, edit one cloned text
source, save it, run its validation configuration, and review the original native
URI and location when supplied. Restore the exact original resource bytes to
repair that experiment, then rerun. A fresh clone also restores the original
fixture without changing existing copies. Some source validators report no text
coordinates; no replacement character ranges are invented.

For predefined reproducible failures and exact repairs, open the ordinary CSV
inventory, XHTML document, schema-authoring or Native DOM, AST and events
projects. Their configured failure buffers are distinct from these selected
passing package examples.

## Persistence and privacy

Saved copies retain their own resources and configuration IDs. Offline operation
requires the normal Studio asset cache and already-loaded source/command resources;
this README introduces no extra loader, service or storage key. Example themes
and persisted page-theme settings remain independent.

Entered passwords and one-time codes block recovery until the user explicitly
clears them. Automatic whole-Studio recovery remains disabled. No source example,
expected result or catalog update grants credential capture, external resource
reads, code execution or replay of side effects.
