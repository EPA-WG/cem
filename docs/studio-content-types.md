# Studio content types and schema inventory

Audited 2026-10-24. This is the implementation map for the **Content types and
schemas** sample collection in [the active checklist](todo.md#studio-samples-supported-content-types-and-schemas).
The collection and its category folders are the next implementation step.
Examples remain ordinary cloneable projects with resources and configurations.

## Sources and scope

Package identities below come from the
[native-emitted registry](../packages/cem_ml/schema/schema-assets-registry-v1.json),
which is also consumed by the native optional-package catalog. The native
`cem-ml-schema-assets-emit --check-registry` command checks it against the complete
embedded manifests. The
[Feature Tour generator](../packages/cem-studio/scripts/generate-feature-tour.mjs)
selects one passing manifest example for each package; its generated catalog
retains source paths, manifest paths, resource identities and byte hashes.

The registry records ownership and negotiation selectors. It includes source
types, binary artifacts and result types; its aliases are not a promise that
every command accepts every selector. Shared MIME values require the appropriate
schema identity. Namespace dependencies such as XHTML, SVG, MathML and runtime
data-island namespaces do not create additional source parsers.

The browser [capability contract](../packages/cem_ml/src/capability.rs) supplies
the operation availability below. Availability means the command surface exists;
it does not establish per-format support. Formatting/coloring are `convert`
options, not separate browser operations. Development-only debugger controls do
not change these content-type guarantees.

<!-- browser-operations:start -->
| Operation | Availability |
| --- | --- |
| `parse` | `available` |
| `validate` | `available` |
| `check` | `available` |
| `inspect` | `available` |
| `convert` | `available` |
| `query` | `available` |
| `transform` | `available` |
| `trace` | `available` |
| `version-capabilities` | `available` |
| `bench` | `unavailable` |
| `fixture` | `unavailable` |
| `schema-mutation` | `unavailable` |
| `plugin-mutation` | `unavailable` |
<!-- browser-operations:end -->

## Registered identities

All versions are package versions; schema URI suffixes are independent. In
particular, module-map packages v1, v2 and v3 each have package version `1.0.0`.
“Other selectors” preserves the registry exactly, including binary/result and
legacy selectors that are not interchangeable authoring syntaxes.

<!-- native-identities:start -->
| Package | Package version | Schema identity | Primary content type | Other registered selectors |
| --- | --- | --- | --- | --- |
| `cem-ast-projection` | `1.0.0` | `https://cem.dev/ns/projection/ast/1` | `application/vnd.cem.ast+cem-bin` | `application/vnd.cem.ast+json` |
| `cem-data-island` | `1.0.0` | `https://cem.dev/ns/runtime/data-island` | `application/vnd.cem.data-island+cem` | — |
| `cem-dom-projection` | `1.0.0` | `https://cem.dev/ns/projection/dom/1` | `application/vnd.cem.dom+cem-bin` | `application/vnd.cem.dom+json` |
| `cem-element-template` | `1.0.0` | `https://cem.dev/ns/template/cem-element/1` | `application/vnd.cem.element-template+cem` | — |
| `cem-events-projection` | `1.0.0` | `https://cem.dev/ns/projection/events/1` | `application/vnd.cem.events+cem-bin` | `application/vnd.cem.events+json` |
| `cem-ml` | `1.0.0` | `https://cem.dev/ns/cem-ml/1` | `application/cem` | `text/cem-ml`, `text/cem`, `application/cem+xml` |
| `cem-native-template` | `1.0.0` | `https://cem.dev/ns/template/cem-native/1` | `application/vnd.cem.template+cem` | `application/cem`, `application/cem+xml`, `text/cem`, `text/cem-ml` |
| `cem-ql` | `1.0.0` | `https://cem.dev/ns/query/cem-ql/1` | `application/vnd.cem.query+cem-ql` | `text/cem-ql`, `application/vnd.cem.query-expression+cem-ql`, `application/vnd.cem.query-artifact+cem-bin`, `cem-ql/1`, `cem-ql/module` |
| `cem-transform` | `1.0.0` | `https://cem.dev/ns/transform/cem/1` | `application/vnd.cem.transform+cem` | — |
| `cli-command` | `1.0.0` | `https://cem.dev/ns/cli/command/1` | `application/vnd.cem.cli-command+json` | — |
| `css` | `1.0.0` | `https://cem.dev/ns/data/css/1` | `text/css` | — |
| `css-selector` | `1.0.0` | `https://cem.dev/ns/query/css-selector/1` | `application/vnd.cem.query-expression+css-selector` | — |
| `csv` | `1.0.0` | `https://cem.dev/ns/data/csv/1` | `text/csv` | — |
| `html` | `1.0.0` | `https://cem.dev/ns/data/html/1` | `text/html` | — |
| `json` | `1.0.0` | `https://cem.dev/ns/data/json/1` | `application/json` | `text/json` |
| `json-schema` | `1.0.0` | `https://cem.dev/ns/data/json-schema/1` | `application/schema+json` | — |
| `markdown` | `1.0.0` | `https://cem.dev/ns/data/markdown/1` | `text/markdown` | — |
| `mathml` | `1.0.0` | `https://cem.dev/ns/data/mathml/1` | `application/mathml+xml` | `application/mathml-presentation+xml`, `application/mathml-content+xml` |
| `module-map` | `1.0.0` | `https://cem.dev/ns/data/module-map/1` | `application/vnd.cem.module-map+json` | — |
| `module-map-v2` | `1.0.0` | `https://cem.dev/ns/data/module-map/2` | — | `application/vnd.cem.module-map+json` |
| `module-map-v3` | `1.0.0` | `https://cem.dev/ns/data/module-map/3` | — | `application/vnd.cem.module-map+json` |
| `relax-ng` | `1.0.0` | `https://cem.dev/ns/data/relax-ng/1` | `application/relax-ng+xml` | `application/relax-ng-compact-syntax` |
| `schema` | `1.0.0` | `https://cem.dev/ns/schema/1` | `application/vnd.cem.schema+cem` | — |
| `schema-package` | `1.0.0` | `https://cem.dev/ns/schema-package/1` | `application/vnd.cem.schema-package+cem` | — |
| `scss` | `1.0.0` | `https://cem.dev/ns/data/scss/1` | `text/vnd.cem.scss` | `text/x-scss` |
| `studio-project` | `1.0.0` | `https://cem.dev/ns/studio/project/1` | `application/vnd.cem.studio-project+cem` | `application/vnd.cem.studio-project+json` |
| `svg` | `1.0.0` | `https://cem.dev/ns/data/svg/1` | `image/svg+xml` | — |
| `xhtml` | `1.0.0` | `https://cem.dev/ns/data/xhtml/1` | `application/xhtml+xml` | — |
| `xml` | `1.0.0` | `https://cem.dev/ns/data/xml/1` | `application/xml` | `text/xml`, `application/xml-external-parsed-entity`, `text/xml-external-parsed-entity`, `application/xml-dtd` |
| `xpath` | `1.0.0` | `https://cem.dev/ns/query/xpath/1` | `application/vnd.cem.xpath` | `application/vnd.cem.xpath-artifact+cem-bin`, `text/xpath`, `application/vnd.cem.xpath-result+json` |
| `xslt` | `1.0.0` | `https://cem.dev/ns/transform/xslt/1` | `application/xslt+xml` | `application/vnd.cem.xslt-bundle+cem-bin`, `text/xsl`, `custom-element-xslt`, `text/custom-element-xslt`, `application/custom-element-xslt`, `text/x-custom-element-xslt` |
| `yaml` | `1.0.0` | `https://cem.dev/ns/data/yaml/1` | `application/yaml` | `application/x-yaml`, `text/yaml`, `text/x-yaml` |
<!-- native-identities:end -->

The CEM-QL standalone expression schema is
`https://cem.dev/ns/query/cem-ql/1#expression`, selected with
`application/vnd.cem.query-expression+cem-ql`; it is a mode of the CEM-QL package.
Run/transform configuration schemas under `https://cem.dev/ns/cli/` are consumed
by the command/graph APIs. Their use does not add another registered package.

## Operations and boundaries

“Source validation” checks the selected language's source contract. It differs
from validating an instance against an authored schema. “Inspect” refers to
native projections and source locations, not necessarily a lossless source AST.
Formatting/coloring require the relevant native pipeline and registered
artifacts; the mere presence of a manifest artifact is insufficient.

| Families | Import and source validation | Formatting and inspection | Query or transformation use; explicit limits |
| --- | --- | --- | --- |
| JSON, YAML, CSV | Native data imports and source validation. CSV carries header/quoting diagnostics. | Native formatting/conversion and retained data projections. | Data inputs for supported queries/templates; a report needs an authored query/template. CSV is tabular and conversion must respect its data shape. |
| XML, XHTML | XML data import; selected schema validation is separate from XML well-formedness. XHTML has its own source contract. | XML/XHTML formatting and source projections. | Namespace-aware queries and template data. Generic XML import rejects DTDs and unresolved entities, despite the registry's DTD/external-entity selectors. |
| HTML | Native HTML import and source validation. | Formatting/coloring and retained element inspection. | Element-tree CSS selectors, XPath/CEM-QL semantic queries, CEM rendering and preview. Browser HTML behavior alone does not prove native validation. |
| CEM-ML | Native CEM parser/source validation; not a generic data-loader MIME. | Native formatting/coloring, DOM/AST/events and source ranges. | CEM templates and retained CEM queries. Explicit schema selects generic source versus template semantics. |
| Markdown | Native source validation and Markdown-to-HTML conversion; not a generic data-loader MIME. | Native source formatting/coloring. Current Parse AST/events describe generated HTML. | Article/site HTML inputs to layouts; HTML projection is not a Markdown source AST. |
| CSS | Native retained CSS import and stylesheet validation. | Typed CSS inspection and native formatter/colorizer pipeline. | Scoped stylesheet preparation. It is not a query language or a general Sass compiler. |
| SCSS | Explicit MIME/schema selects passive source validation; generic import and filename inference are unavailable. | Parse AST/events inspect expanded typed CSS with SCSS origin frames. | Bounded resolver/context-owned expansion to plain CSS. Passive validation does not fetch modules or expand. Same-schema SCSS source export, source DomJson/Json and command HTML-colored CSS export are unavailable. |
| SVG, MathML | XML syntax import and package-specific source validation are distinct steps. | Native XML-family formatting/projection; use supported vocabulary/subsets. | Shared media/HTML previews and document queries where an element tree is available. Static asset copying proves neither parsing nor complete SVG/MathML semantics. |
| CSS selector | Source/expression validation; not data. | Registered expression formatting/coloring. | Explicit canonical MIME or schema selects element-tree query dispatch; supplied MIME/schema must agree. No general-purpose transformation language. |
| XPath | Expression validation and native query compilation; not generic data. | Native expression formatting/coloring and inspection. | XPath/XDM-tree query or direct transformation. Binary artifacts and JSON results are not XPath source text. Supported native subset applies. |
| CEM-QL | Query modules and standalone expressions have explicit native source handling. | Native Parse/Inspect, formatting/coloring, source ranges. | Native-items query/evaluation, parameters and template integration. Module, expression and binary artifact selectors have distinct contracts. |
| CEM native/element templates | CEM source plus the selected template contract; element declarations run through shared `cem-elements`. | CEM-native formatting/inspection. | Native templates render data; element templates also bind declarative browser state. Source validation alone does not execute browser bindings. |
| CEM transformation | CEM configuration source validation. | CEM formatting/source projections. | Authored direct/graph execution, dependency resolution, parameters, artifacts and provenance. Run/transform config schemas remain explicit. |
| XSLT | Native stylesheet-source validation/Parse, separate from generic XML import. | Native AST/events, source formatting/coloring. Source DomJson/Json are unavailable. | Complete deployment has bounded native XSLT execution. Optional-body deployment supports owned source operations but executable bundle compilation still needs parser-contract handoff; do not advertise optional executable graphs yet. |
| CEM schema/package | Native declaration/manifest validation and package loading. | CEM formatting/source inspection. | Explicitly supplied schemas/packages validate instances, as Login demonstrates. Admission and complete package resource validation are separate phases. |
| JSON Schema | JSON parsing and Draft 2020-12 dialect source checks. | JSON-family source inspection/formatting. | Instance assertions, remote references and dynamic scope are unavailable. Do not label schema-source validation as validation of a JSON payload against that schema. |
| Relax NG | Native XML/compact schema-source validation and Parse; compact grammar checks remain lexical. | Source formatting/coloring, AST/events; native Events retain source. Source DomJson/Json are unavailable. | No semantic grammar compiler or instance validator; include/external references are rejected. Do not label schema-source validation as validation of an XML instance. |
| Module maps v1/v2/v3 | Native version-specific JSON control contract validation; v2/v3 require their explicit schema. | JSON source projections/formatting. | Resolver input for the corresponding version; validation is not a grant to read external resources. No implicit dependency discovery, npm export selection or undeclared copying. |
| CLI command, Studio project | Explicit portable control contracts; Studio project supports CEM and JSON encodings. | Source inspection follows the chosen encoding. | Command execution and project import/build go through native grammar and repository services. Control documents are not executable just because generic JSON parsed. |
| DOM/AST/events projections | Explicit native projection codecs; binary types are not text source. | Human-readable JSON inspection and source-origin views. | Consume/produce through the supported projection APIs. No automatic general format conversion or evaluation guarantee for all projection selectors. |
| Runtime data island | Declared CEM runtime-state envelope validation. | CEM source inspection. | Shared runtime hydration/state contract; not a standalone data query language or a portable secret store. |

The generic retained-document
[import boundary](../packages/cem_ml/src/import.rs) accepts JSON (`application/json`,
`text/json`, `+json`), XML (`application/xml`, `text/xml`, `+xml`), YAML
(`application/yaml`, `application/x-yaml`, `text/yaml`), CSV, HTML and `text/css`.
Its short selectors are `json`, `xml`, `yaml`, `csv` and `html`. `text/x-yaml` is
registered but is not accepted at this boundary. MIME essence matching is
case-insensitive; CSV defaults to `header=present`. UTF-8 `text/plain` is an
additional loader input with the CEM text projection only; it has no schema
package. Structured `+json`/`+xml` suffixes select syntax parsing, not domain
validation, query compilation or template execution.

The native [query dispatch](../packages/cem_ml/src/query.rs) binds CSS selectors
to element trees, XPath to XDM trees and CEM-QL to native items. A sample must
show the data import/projection and language together. Query availability must
not be copied onto every registry row. Package READMEs under
[schema-packages](../packages/cem_ml/schema-packages/README.md) own each language's
supported grammar, formatter profiles and subset restrictions.

The complete deployment remains the default. The opt-in WASM profile omits CSV,
Markdown, SCSS, XSLT and Relax NG source bodies and requires trusted host demand,
original-byte admission and publication on every consuming runtime. See the
[optional-format evidence](archive/todo-completed-optional-format-demand-2026-10-24.md).
No sample, resource or URL may grant itself package activation authority.

## Existing samples and missing workflows

Every registered identity maps to project `schema-examples`, resource
`example-<package-id>`, configuration `validate-<package-id>`, and configuration
resource `run-<package-id>`. This covers the manifest-selected content type/schema
pair only. Other registered selectors need their own operation evidence; the
inventory does not imply that every alias has a passing Studio configuration.
Generated resources and native manifest references remain authoritative.

The following references identify current richer coverage. They use project IDs,
resource IDs and configuration IDs, independently of display labels or folders.

<!-- existing-samples:start -->
| Project | Resource | Configuration | Demonstrated use |
| --- | --- | --- | --- |
| `supplier-pipeline` | `supplier-stock` | `stock-source` | CSV source inspection. |
| `supplier-pipeline` | `supplier-catalog` | `catalog-source` | XML catalog source inspection. |
| `supplier-pipeline` | `supplier-reconcile` | `availability` | CSV + XML reconciliation, HTML/JSON/YAML outputs, provenance and deliberate repair. |
| `markdown-site` | `data-site-json` | `build-site` | Site metadata, Markdown folder tree, layouts, relative CSS/images, explicit entry output and provenance. |
| `login` | `login-html` | `semantics-html` | XPath query of the HTML semantic contract. |
| `login` | `login-cem` | `semantics-cem-cemql` | CEM-QL query of the CEM semantic contract. |
| `login` | `login-expression` | `expression-html` | Standalone expression evaluation. |
| `login` | `login-expression` | `expression-format-tabular` | Native standalone expression formatting. |
| `login` | `login-view-template` | `render-cem` | Native data-to-HTML template rendering. |
| `login` | `login-view-graph` | `render-graph` | Graph execution, generated output and source provenance. |
| `login` | `login-live-view` | `live-view` | Shared declarative browser rendering. |
| `login` | `login-invalid` | `validation-repair` | Authored CEM schema/package, intentional failure and repair. |
| `schema-examples` | `operation-input` | `workbench-conversion` | CEM-to-DOM projection. |
| `schema-examples` | `operation-data` | `workbench-query` | XML data with an XPath query. |
| `schema-examples` | `operation-template` | `workbench-transformation` | Direct XPath transformation. |
| `schema-examples` | `operation-graph` | `workbench-transformation-graph` | Portable transformation graph. |
| `schema-examples` | `operation-input` | `workbench-trace` | Parser/validation trace. |
<!-- existing-samples:end -->

These are planned functional projects or extensions, not additional routes. The
category names below are discovery metadata; they do not affect runtime authority.

| Category | Families / selectors to cover | Reuse or planned workflow |
| --- | --- | --- |
| Data | JSON and YAML aliases; CSV | New payload/configuration report project and inventory/table project. Include query/format configs, quoting/header failures and repair; reuse Supplier outputs. |
| Data | XML, XHTML and XML aliases | Extend Supplier with namespaces and selected instance validation; add XHTML operations. Keep DTD/entity refusals visible. |
| Documents and UI | Markdown | Reuse `markdown-site`; expose article conversion and complete site outputs. |
| Documents and UI | HTML, CEM-ML, native/element templates, runtime data island | Reuse Login and add a minimal declarative component project; explain template identity and live runtime bindings. |
| Documents and UI | SVG and MathML variants | New diagram/formula project with native validation, inspection and shared preview; document supported vocabulary. |
| Documents and UI | CSS and SCSS aliases | New stylesheet project with inspect/format, supported scoped CSS and bounded SCSS-to-CSS expansion. Include unavailable source-export/loader cases. |
| Queries and transformations | CSS selectors, XPath source aliases, CEM-QL modules/expressions | Reuse Login semantic comparisons and expression configs; add selectors, parameter maps, joins/filtering and source provenance. Treat compiled artifacts/results as separate inputs/outputs. |
| Queries and transformations | CEM transformation and XSLT source/custom-element selectors | Reuse existing CEM graphs; add a complete-profile bounded XSLT graph and explicit optional-profile refusal. Do not treat custom-element mode as a universal XSLT alias. |
| Schema authoring | CEM schema and schema package | Extend Login's failure/repair package with declarations, references and explicit instance inputs. |
| Schema authoring | JSON Schema and both Relax NG syntaxes | Add schema-source inspect/format/validation projects. Show instance validation only where independently supported; document unsupported drafts/patterns rather than promise arbitrary validators. |
| Schema authoring | Module maps v1/v2/v3, CLI command, Studio project encodings | New portable configuration/dependency-resolution project. Demonstrate supported versions, mapped/relative resources and validated run configs. |
| Schema authoring | DOM/AST/events binary/JSON projections | Extend existing conversion sample with native codec inputs, human-readable inspection and source locations. Separate serialization from document parsing and execution. |
| Data | UTF-8 `text/plain` loader input | Include a small text-resource use alongside configuration samples; explain the CEM-only projection and absence of a registered schema package. |

Each future project needs a README, explicit configurations, expected artifacts,
source provenance and failure/repair where applicable. UI stays in shared
declarative components; native parsing/evaluation stays in the engine. Example
theme/storage keys remain independent from Studio. Entered passwords/one-time
codes block recovery until explicitly cleared, and automatic whole-Studio
recovery remains disabled.

## Verification and updates

The existing `@epa-wg/cem-studio:verify:feature-tour` gate checks this identity table
against the native-emitted registry, the capability list against runtime metadata,
and each existing sample reference against the generated ordinary project bundle.
It also runs all manifest-selected source validations through the native CLI and
checks deployed bytes and hashes. The `test:feature-tour` browser cases exercise
the authored Login and Supplier workflows; site preview/build browser acceptance
lives in `test:workbench`. These checks establish the covered operations; they do
not turn untested aliases or planned workflows into supported commands.

When packages change, regenerate/check the native registry, update this audit,
then run the existing Feature Tour and output gates. Keep sample definitions in
[examples/projects](../examples/projects/README.md) and generated Schema examples
in their existing pipeline. This document is planning/verification evidence, not
a second runtime dispatcher or an activation allowlist.
