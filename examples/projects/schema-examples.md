# Schema examples and portable operations

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
