# Studio content types and schema inventory

Audited 2026-10-26. This is the implementation map for the **Content types and
schemas** sample collection in [the active checklist](todo.md#studio-samples-supported-content-types-and-schemas).
The collection is available under **Examples → Content types and schemas**, with
Data, Documents and UI, Queries and transformations, and Schema authoring folders.
Examples remain ordinary cloneable projects with resources and configurations.
Folder listings reuse existing projects. The
[catalog README/capability audit](studio-catalog-coverage.md) records 22 authored
projects and 213 configurations, plus the generated Schema examples project and
the starter. All 218 inventory rows (213 authored and five portable workbenches)
are checked against their actual resource/configuration references. The 32
generated package validations remain a separate registry matrix. These counts
describe declared coverage, not uniform native conformance or UI runnability.

Open `?collection=content-types-and-schemas` for the category listing. A project
opened from a category carries `collection=<category-id>&sample=<project-id>` so
the correct occurrence stays selected. Existing sample/file bookmarks still work,
as do Amsterdam and `?collection=examples`. Categories share projects and do not
copy resources or change archive identities. Saved clones and drafts stay
independent. The authored hierarchy is in
[collections.json](../examples/projects/collections.json).

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
| Markdown | Native source validation and Markdown-to-HTML conversion; opt-in YAML front matter has typed metadata checks. Not a generic data-loader MIME. | Native source formatting/coloring. Current Parse AST/events and the article Inspect workflow describe generated HTML. | Namespace-aware queries consume the generated body; site layouts receive metadata separately. An HTML projection is not a Markdown source AST. |
| CSS | Native retained CSS import and stylesheet validation. | Typed CSS inspection and native formatter/colorizer pipeline. | Scoped stylesheet preparation. It is not a query language or a general Sass compiler. |
| SCSS | Explicit MIME/schema selects passive source validation; generic import and filename inference are unavailable. | Parse AST/events inspect expanded typed CSS with SCSS origin frames. | Bounded resolver/context-owned expansion to plain CSS. Passive validation does not fetch modules or expand. Same-schema SCSS source export, source DomJson/Json and command HTML-colored CSS export are unavailable. |
| SVG, MathML | Dedicated family import retains the original XML-backed owner, namespace, source positions and profile policy. SVG and all three MathML MIME identities resolve before generic `+xml`. | Native same-schema formatting and default CEM-ML inspection. Explicit JSON inspection retains the existing compatibility projection. | Shared native queries/templates and bounded inline artifact previews. Complete vocabulary/geometry/algebra validation is not implied; browser admission is independent of source validation. |
| CSS selector | Source/expression validation; not data. | Native command source formatting/coloring use registered lexical-lossless profiles. Parse/Inspect do not expose a typed selector source AST; Inspect AST remains a generic text envelope. | Explicit canonical MIME or schema selects element-tree query dispatch; supplied MIME/schema must agree. Source formatting does not evaluate selectors or grant namespace/host capabilities. No general-purpose transformation language. |
| XPath | Expression validation and native query compilation; not generic data. Standalone source validation still omits caller prefix bindings. | Typed source Parse/Inspect retain native expression/token owners and original ranges; source formatting/coloring use the registered lexical-lossless pipeline. | XPath/XDM-tree query or direct transformation. Binary artifacts and JSON results are not XPath source text. Supported native subset applies. |
| CEM-QL | Query modules and standalone expressions have explicit native source handling. | Native Parse/Inspect, formatting/coloring, source ranges. | Native-items query/evaluation, parameters and template integration. Module, expression and binary artifact selectors have distinct contracts. |
| CEM native/element templates | CEM source plus the selected template contract; element declarations run through shared `cem-elements`. | CEM-native formatting/inspection. | Native templates render data; element templates also bind declarative browser state. Source validation alone does not execute browser bindings. |
| CEM transformation | CEM configuration source validation. | CEM formatting/source projections. | Authored direct/graph execution, dependency resolution, parameters, artifacts and provenance. Run/transform config schemas remain explicit. |
| XSLT | Native stylesheet-source validation/Parse, separate from generic XML import. | Parse exposes typed source AST/events; Inspect exposes the generic imported CEM structure. Native source formatting/coloring; source DomJson/Json are unavailable. | Complete deployment has bounded native XSLT execution. Optional-body deployment supports owned source operations but executable bundle compilation still needs parser-contract handoff; do not advertise optional executable graphs yet. |
| CEM schema/package | Native declaration/manifest validation and package loading. | CEM formatting/source inspection. | Explicitly supplied schemas/packages validate instances, as Login demonstrates. Admission and complete package resource validation are separate phases. |
| JSON Schema | JSON parsing and Draft 2020-12 dialect source checks. | Native source formatting. JSON-family Parse/Inspect can emit CEM tokenizer errors despite exit zero, including structural inspection of `$defs`/`$ref`; those operations are omitted from this project's configurations. | Instance assertions, remote references and dynamic scope are unavailable. Do not label schema-source validation as validation of a JSON payload against that schema. |
| Relax NG | Native XML/compact schema-source validation and Parse; compact grammar checks remain lexical. | Typed source Parse AST/events and native source formatting/coloring; Inspect shows generic imported CEM structure. Native Events retain source. Source DomJson/Json are unavailable. | No semantic grammar compiler or instance validator; include/external references are rejected. Do not label schema-source validation as validation of an XML instance. |
| Module maps v1/v2/v3 | Standalone validation checks JSON syntax under the selected identity; v2/v3 require their explicit schema. Paired-map version/deployment semantics are enforced by native graph planning. | Native same-identity source formatting and generic imported-CEM structural Inspect where supported. Generic JSON Parse has the documented source-identity/diagnostic gap. | Native deployment rewriting uses declared source/destination pairs and version-specific resource kinds/edges. Source validation is not a read grant. No implicit dependency discovery, npm export selection or undeclared copying. |
| CLI command | Native validation checks the versioned literal-argv resource envelope; the executing host's native codec checks actual argv grammar. | Studio's command/plan inspector uses native decoded controls. Generic source Parse/Inspect/format have routing/export gaps and are not advertised as supported command-source operations. | Authored commands use native grammar and bounded host resources. Source validation alone does not execute a command. |
| CLI run configuration | Native CLI loading/normalization through `--config`; this is not a registered standalone source adapter. | Inspect the actual native normalized command/plan. Generic source Parse/Inspect/format are unavailable. | Native run configuration and host grants remain explicit; parsing a JSON document does not grant resource reads. |
| Studio project | Explicit portable control contract with CEM and JSON encodings. | Native validation/formatting and cross-encoding conversion; CEM structural Inspect where supported. Generic source Parse/JSON Inspect retain the documented source-identity gaps. | Repository import/export and builds preserve captured resource identities. A valid manifest is not a grant to execute arbitrary resources. |
| DOM/AST/events projections | Registered JSON shape and binary-header validation; projection input adapters remain output-only. | Original CEM Parse AST/events and retained imported-tree Inspect default to native CEM-ML; JSON/binary exports require explicit targets. | DOM/events JSON exports validate; parser AST JSON exports an array while its registered validator requires an object. Binary validation does not establish payload integrity or safe replay. Numeric fields in imported JSON do not grant source ownership. |
| Runtime data island | Declared CEM runtime-state envelope validation. | CEM source inspection. | Shared runtime hydration/state contract; not a standalone data query language or a portable secret store. |

The generic retained-document
[import boundary](../packages/cem_ml/src/import.rs) accepts JSON (`application/json`,
`text/json`, `+json`), XML (`application/xml`, `text/xml`, `+xml`), XHTML
(`application/xhtml+xml`), SVG (`image/svg+xml`), MathML (`application/mathml+xml`,
`application/mathml-presentation+xml`, `application/mathml-content+xml`), YAML (`application/yaml`, `application/x-yaml`,
`text/yaml`), CSV, HTML and `text/css`. Its short selectors are `json`, `xml`,
`xhtml`, `yaml`, `csv` and `html`. `text/x-yaml` is registered but is not accepted
at this boundary. MIME essence matching is case-insensitive; CSV defaults to
`header=present`. UTF-8 `text/plain` is an additional loader input with the CEM
text projection only; it has no schema package. The explicit XHTML selector
retains and validates its XHTML owner before XML semantic import. The explicit
SVG/MathML media types likewise retain their family owners, profiles and policy;
they do not fall through to generic XML. Other structured
`+json`/`+xml` suffixes select syntax parsing, not domain validation, query
compilation or template execution.

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
| `csv-inventory` | `inventory-source` | `inspect-csv` | Inspect imported cell values and original source locations through the generic CEM data projection. |
| `csv-inventory` | `inventory-source` | `format-csv` | Compact CSV interchange formatting without changing cell values. |
| `csv-inventory` | `inventory-source` | `review-csv` | Tabular review presentation, separate from interchange bytes. |
| `csv-inventory` | `inventory-source` | `query-low-stock` | Filter inventory through the imported generic-data tree and explicit numeric rules. |
| `csv-inventory` | `headerless-source` | `query-headerless` | Explicit `header=absent` keeps the first row and uses positional cells. |
| `csv-inventory` | `invalid-header-policy` | `repair-header-policy` | Native header-mode warning; repair the authored run configuration and rerun it. |
| `csv-inventory` | `unclosed-quote` | `repair-unclosed-quote` | Hard native quote-closure failure with exact repair. |
| `csv-inventory` | `invalid-escape` | `repair-invalid-escape` | Hard native quote-escaping failure with exact repair. |
| `csv-inventory` | `ragged-rows` | `repair-ragged-rows` | Native row-width warning and source repair, retaining advisory severity. |
| `csv-inventory` | `inventory-graph` | `build-inventory` | Typed low-stock HTML/JSON report with template origins and captured inputs. |
| `json-yaml-report` | `api-payload` | `inspect-json` | Inspect the imported generic CEM data projection of an API payload. |
| `json-yaml-report` | `api-payload` | `query-json` | Filter records through a retained native CEM-QL expression. |
| `json-yaml-report` | `report-settings` | `validate-yaml` | Validate configuration source syntax, separately from application rules. |
| `json-yaml-report` | `report-settings` | `format-yaml` | Native same-schema YAML formatting. |
| `json-yaml-report` | `report-settings` | `query-yaml` | Inspect configured report title and status through native CEM-QL. |
| `json-yaml-report` | `report-graph` | `build-report` | JSON + YAML inputs, configured HTML report and selected-record JSON/YAML exports. |
| `json-yaml-report` | `invalid-payload` | `invalid-json` | Intentional JSON source failure with an explicit repair. |
| `json-yaml-report` | `invalid-settings` | `invalid-yaml` | Intentional YAML source failure with an explicit repair. |
| `supplier-pipeline` | `supplier-stock` | `stock-source` | CSV source inspection. |
| `supplier-pipeline` | `supplier-catalog` | `catalog-source` | XML catalog source inspection. |
| `supplier-pipeline` | `supplier-catalog` | `validate-catalog` | Native XML well-formedness and namespace validation, separate from supplier business rules. |
| `supplier-pipeline` | `supplier-catalog` | `query-products` | Expanded-name XPath selection with an explicit namespace binding and foreign-name exclusion. |
| `supplier-pipeline` | `supplier-invalid-catalog` | `repair-catalog` | Mismatched XML end-tag diagnostics at original UTF-8 coordinates with an exact repair. |
| `supplier-pipeline` | `supplier-reconcile` | `availability` | CSV + XML reconciliation, HTML/JSON/YAML outputs, provenance and deliberate repair. |
| `xhtml-document` | `article-source` | `inspect-xhtml` | Original XHTML lifecycle identity, expanded names, foreign content and source locations. |
| `xhtml-document` | `article-source` | `validate-xhtml` | XML-backed XHTML root/namespace/head-body checks, without claiming full HTML vocabulary validation. |
| `xhtml-document` | `article-source` | `query-outline` | Namespace-aware XPath document outline, excluding foreign SVG title elements. |
| `xhtml-document` | `article-invalid` | `repair-namespace` | Missing XHTML namespace diagnostic, saved source repair and revalidation. |
| `xhtml-document` | `article-graph` | `build-html` | Retained XHTML input to a CEMT HTML artifact with explicit entry and template provenance. |
| `markdown-site` | `graphs-build-cem` | `build-site` | Four Markdown pages, typed metadata, shared layouts/partials, relative CSS/images and byte-preserved assets, with an explicit entry artifact and original content/template provenance. |
| `markdown-site` | `content-articles-first-build-md` | `validate-article` | Validate the saved article's explicit Markdown identity and opt-in YAML metadata types. |
| `markdown-site` | `content-articles-first-build-md` | `preview-article` | Exact standalone body HTML, excluding front matter; site assets and shared layouts belong to the full build. |
| `markdown-site` | `content-articles-first-build-md` | `inspect-article` | Inspect the generated HTML structure and retained source information, rather than the Markdown parser's source AST. |
| `markdown-site` | `content-articles-first-build-md` | `query-article-outline` | XPath heading, image/link counts and image reference with an explicit XHTML namespace binding; metadata title does not become body text. |
| `markdown-site` | `invalid-article-metadata` | `repair-article-metadata` | Native typed front-matter failure at original source coordinates; repair a string weight to an integer in a resource outside the site content glob. |
| `html-cem-card` | `card-html` | `inspect-html` | Retained HTML elements, attributes and original source positions. |
| `html-cem-card` | `card-cem` | `inspect-cem` | Native CEM structure for the equivalent card source with an explicit XHTML namespace. |
| `html-cem-card` | `card-html` | `semantics-html` | Nine named card fields queried with a caller-bound XHTML prefix. |
| `html-cem-card` | `card-cem` | `semantics-cem` | The same nine fields with explicit HTML/CEM attribute namespace mappings. |
| `html-cem-card` | `card-graph` | `build-html` | Saved HTML data and a native CEMT template produce an exact HTML entry artifact with template provenance. |
| `html-cem-card` | `card-live-view` | `live-view` | Saved data bound to two shared declarative cards, named/default slots and a transient local action. |
| `html-cem-card` | `card-component` | `inspect-component` | Explicit JSON inspection of the canonical shared declaration's XML source, without registering a project component. |
| `html-cem-card` | `card-invalid-policy` | `repair-html-policy` | Native inline-handler refusal at original source coordinates; remove the handler and revalidate the same saved resource. |
| `svg-diagram` | `diagram-source` | `inspect-svg` | Family-preserving SVG inspection with real namespaces and original source offsets. |
| `svg-diagram` | `diagram-source` | `validate-svg` | SVG root/namespace, XML, viewport/accessibility and source-policy checks. |
| `svg-diagram` | `diagram-source` | `query-svg` | Expanded-name XPath diagram labels, dimensions and primitive counts. |
| `svg-diagram` | `diagram-source` | `format-svg` | Exact pretty SVG output, preserving XML identity and mixed text. |
| `svg-diagram` | `invalid-svg` | `repair-svg-namespace` | Repair the missing SVG namespace at original source coordinates. |
| `svg-diagram` | `diagram-graph` | `build-html` | Native CEMT HTML entry copying retained SVG geometry, text and source provenance. |
| `mathml-formula` | `formula-source` | `inspect-mathml` | Namespace- and profile-preserving Presentation MathML inspection. |
| `mathml-formula` | `formula-source` | `validate-presentation` | Validate the Presentation MathML alias and XML/root/namespace rules. |
| `mathml-formula` | `formula-content` | `validate-content` | Validate a separate Content MathML source; no algebra evaluation or presentation conversion is implied. |
| `mathml-formula` | `formula-source` | `query-mathml` | Expanded-name XPath fraction, exponent, identifier and token summary. |
| `mathml-formula` | `formula-source` | `format-mathml` | Exact pretty output retaining the Presentation MathML media identity. |
| `mathml-formula` | `invalid-mathml` | `repair-mathml-namespace` | Repair the missing MathML namespace with original byte/line coordinates. |
| `mathml-formula` | `formula-graph` | `build-html` | Native CEMT HTML entry with an inline fraction and superscript rendered by the shared artifact viewer. |
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
| `css-stylesheet` | `style-source` | `inspect-css` | Inspect the lossless typed CSS source/token vocabulary and original source positions. |
| `css-stylesheet` | `style-source` | `inspect-tree` | Inspect native semantic CSS rules, declarations and original ranges, independently of the lexical source AST. |
| `css-stylesheet` | `style-source` | `validate-css` | Check native syntax and source-policy facts, without evaluating the browser cascade or claiming full CSS conformance. |
| `css-stylesheet` | `style-source` | `format-css` | Use the native conservative lexical formatter; pretty preserves comments, strings, values and source layout instead of claiming a CSS beautifier. |
| `css-stylesheet` | `invalid-css` | `repair-css` | Restore the missing closing delimiter and revalidate the same saved CSS resource at the original source position. |
| `css-stylesheet` | `style-graph` | `build-html` | Publish an explicitly authored HTML wrapper and native CSS output as a two-artifact tree; the ordinary browser @scope rule is separate from DCE host scoping. |
| `css-selector` | `selector-source` | `inspect-selector` | View the existing CEM text envelope and original query bytes/offsets. This is not a typed selector-AST export; actual matching and validation use the native selector parser. |
| `css-selector` | `selector-source` | `validate-selector` | Validate standalone selector grammar and capability facts using its explicit query identity, independently of any host document. |
| `css-selector` | `selector-source` | `format-selector` | Run the registered native lexical formatter, preserving selector spelling, comments, escapes and source ranges with the shared text EOF policy. |
| `css-selector` | `selector-source` | `color-selector` | Produce escaped semantic HTML source presentation through the registered selector colorizer; retain original source text and native identity without evaluating the selector. |
| `css-selector` | `document-source` | `query-cards` | Select actual native HTML element identities in document order; the overlapping selector list deduplicates Tea. JSON is an explicit human-facing result export. |
| `css-selector` | `catalog-source` | `query-namespaces` | Use an explicit static namespace binding over the original native XML element tree; source prefix spelling is not query identity. |
| `css-selector` | `invalid-selector` | `repair-selector` | Replace the incomplete attribute selector with the valid project query and revalidate the same saved resource. |
| `scss-stylesheet` | `scss-source` | `validate-scss` | Validate the SCSS source grammar without loading its local module or expanding variables. |
| `scss-stylesheet` | `unresolved-module` | `validate-unresolved-module` | A syntactically valid source may mention an unavailable module: passive validation does not grant resolver access. |
| `scss-stylesheet` | `scss-source` | `parse-scss-ast` | Expand the owned SCSS module set and inspect typed CSS structure with original SCSS origin frames, not a raw SCSS source AST. |
| `scss-stylesheet` | `scss-source` | `parse-scss-events` | Inspect the expanded native CSS event projection and retained SCSS source identities. |
| `scss-stylesheet` | `scss-source` | `expand-css` | Use native variables, a namespaced local mixin and nested selectors; serialize the resulting typed CSS using its own output pipeline. |
| `scss-stylesheet` | `invalid-scss` | `repair-scss-syntax` | Restore the opening block brace and revalidate the same saved source before attempting expansion. |
| `scss-stylesheet` | `scss-graph` | `build-html` | Build a validated HTML source through an explicit CEMT wrapper plus native SCSS-to-CSS output; publish both artifacts with an explicit HTML entry. |
| `cemql-orders` | `order-data` | `inspect-data` | Inspect native generic-data nodes and original JSON source positions without using a serialized AST as query input. |
| `cemql-orders` | `filter-expression` | `inspect-expression` | Inspect typed native expression structure with original query URI and UTF-8 token/node ranges. |
| `cemql-orders` | `filter-expression` | `parse-expression-events` | Review native lossless expression token events, including the Unicode comment and original byte positions. |
| `cemql-orders` | `filter-expression` | `validate-expression` | Validate input-bound standalone syntax separately from execution or application data rules. |
| `cemql-orders` | `filter-expression` | `format-expression` | Apply the native source-preserving tabular query formatter with an exact expected byte fixture. |
| `cemql-orders` | `order-data` | `query-filter` | Select three ready orders and return typed order, units and total fields; the unjoined record remains in this filter result. |
| `cemql-orders` | `order-module` | `validate-module` | Check the explicit module URI, local declarations and root expression; no external library loading is enabled. |
| `cemql-orders` | `order-module` | `format-module` | Apply the native source-preserving tabular formatter while retaining module identity and local functions. |
| `cemql-orders` | `order-data` | `query-join` | Inner-join ready orders totaling at least thirty to their owned customer records; exclude the unmatched customer ID and retain integer result types. |
| `cemql-orders` | `order-data` | `query-source-nodes` | Return retained native ready-order nodes with original JSON source boundaries; runtime-local opaque identities are never compared or persisted as portable addresses. |
| `cemql-orders` | `invalid-expression` | `repair-expression` | Restore the final block brace after reviewing the two native parse diagnostics at original source positions. |
| `cemql-orders` | `parameter-model` | `validate-parameter-template` | Validate native template parameter declarations and their CEM-QL map/expression slots without inventing unbound host-expression globals. |
| `cemql-orders` | `ready-graph` | `report-ready` | Bind declared status/minimum parameters, construct a native CEM-QL options record and publish an application JSON table through the native generic-data writer. |
| `cemql-orders` | `draft-graph` | `report-draft` | Bind declared status/minimum parameters, construct a native CEM-QL options record and publish an application JSON table through the native generic-data writer. |
| `xpath-catalog` | `filter-query` | `validate-filter` | Validate standalone XPath grammar, explicit namespace names and supported source policy without running a document query. |
| `xpath-catalog` | `filter-query` | `parse-filter` | Present the native XPath expression and original token ranges as CEM-ML without interpreting EQNames as CEM structure. |
| `xpath-catalog` | `filter-query` | `parse-filter-events` | Present lossless native XPath token events, including the original comment and source byte positions, without evaluating the query. |
| `xpath-catalog` | `filter-query` | `inspect-filter` | Inspect the typed XPath source owner, EQNames and original UTF-8 ranges through the shared CEM-ML presentation. |
| `xpath-catalog` | `filter-query` | `format-filter` | Run the registered lexical-lossless native XPath formatter, preserving comments, EQNames and whitespace with the shared text EOF policy. |
| `xpath-catalog` | `filter-query` | `color-filter` | Produce escaped semantic HTML source presentation through the native XPath colorizer without evaluating the expression or granting resource access. |
| `xpath-catalog` | `join-query` | `validate-join` | Validate the original prefixed source with a saved caller namespace binding; removing that binding preserves native unknown-prefix errors. |
| `xpath-catalog` | `join-query` | `parse-join` | Inspect native source syntax and ranges under the caller's saved static namespace context. |
| `xpath-catalog` | `join-query` | `format-join` | Preserve original prefix spelling and whitespace while formatting with the saved caller namespace binding. |
| `xpath-catalog` | `catalog-source` | `query-filter` | Use namespace-qualified paths, native numeric comparison and a simple map to report products with stock at least four. |
| `xpath-catalog` | `catalog-source` | `query-nodes` | Export genuine retained product nodes with stable native identities, original input URI, original opening-element ranges and source maps. |
| `xpath-catalog` | `catalog-source` | `query-join` | Evaluate dependent for bindings over the same original XML owner, joining supplier IDs with an explicit static namespace binding. |
| `xpath-catalog` | `catalog-source` | `query-parameters` | Build a native XPath map from the XML selection attributes, then filter and join through native map lookups. These are owned data parameters, not external variable bindings. |
| `xpath-catalog` | `catalog-source` | `inspect-parameters` | Export the typed native map, integer, strings and array with their real query-origin frames. Computed items do not claim arbitrary data-source character lineage. |
| `xpath-catalog` | `invalid-query` | `repair-query` | Close the missing predicate delimiter and revalidate the same saved source. Its independent failure copy does not affect normal query inputs. |
| `cem-transform` | `report-template` | `validate-template` | Validate the declared template identity independently of graph execution or the output data contract. |
| `cem-transform` | `report-template` | `inspect-template` | Inspect the native source tree and original template ranges; this does not execute the template. |
| `cem-transform` | `report-template` | `format-template` | Apply the native tabular source formatter and compare its exact source bytes. |
| `cem-transform` | `report-graph` | `validate-graph` | Validate graph declarations, source/template identities, parameters and export definitions without claiming a successful build. |
| `cem-transform` | `report-graph` | `inspect-graph` | Inspect the native graph source tree: orders import, filtered report model, HTML view, parameters and explicit exports. |
| `cem-transform` | `invalid-template` | `repair-template` | Review the original native source diagnostic, restore the required declaration and revalidate the saved copy. |
| `cem-transform` | `report-graph` | `build-report` | Filter original orders into a native report model, then render the model through the shared CEM-native template runtime. The explicit HTML entry drives project preview. |
| `xslt-transform` | `report-template` | `validate-template` | Validate the declared template identity independently of graph execution or the output data contract. |
| `xslt-transform` | `report-template` | `inspect-template` | Inspect the native source tree and original template ranges; this does not execute the template. |
| `xslt-transform` | `report-template` | `format-template` | Apply the native tabular source formatter and compare its exact source bytes. |
| `xslt-transform` | `report-graph` | `validate-graph` | Validate graph declarations, source/template identities, parameters and export definitions without claiming a successful build. |
| `xslt-transform` | `report-graph` | `inspect-graph` | Inspect the native graph source tree: orders import, filtered report model, HTML view, parameters and explicit exports. |
| `xslt-transform` | `invalid-template` | `repair-template` | Review the original native source diagnostic, restore the required declaration and revalidate the saved copy. |
| `xslt-transform` | `orders-source` | `transform-html` | Run the named native XSLT page template on the original retained XML with declared minimum/heading parameters. This is the bounded native adapter, never browser XSLTProcessor. |
| `xslt-transform` | `report-graph` | `build-report` | Filter original orders into a native report model, then render the model through the shared CEM-native template runtime. The explicit HTML entry drives project preview. |
| `cem-schema` | `note-schema` | `validate-schema` | Validate native schema declarations, attribute types, field contracts and diagnostic bindings without validating an instance. |
| `cem-schema` | `note-schema` | `inspect-schema` | Inspect the imported native CEM schema source and its original ranges; this is a source projection rather than application JSON. |
| `cem-schema` | `note-schema` | `format-schema` | Apply the native tabular formatter with exact expected bytes. |
| `cem-schema` | `note-package` | `validate-package` | Validate the local schema source declaration, namespace claim and content-type metadata. Metadata alone grants no executable adapter. |
| `cem-schema` | `note-instance` | `validate-instance` | Load the project-owned package and apply its required-id contract to a separately selected instance. |
| `cem-schema` | `invalid-schema` | `repair-schema` | Restore the required schema namespace declaration after reviewing the original native diagnostic. |
| `cem-schema` | `invalid-instance` | `repair-instance` | Apply the loaded domain contract, then add the missing id to the separate instance source. |
| `json-schema` | `note-schema` | `validate-schema` | Validate JSON parsing and the supported dialect declaration; keyword assertions and reference traversal are not instance validation. |
| `json-schema` | `note-schema` | `format-schema` | Format the typed native JSON Schema source through its package-owned tabular writer. |
| `json-schema` | `note-data` | `validate-data-syntax` | Validate the example as ordinary JSON syntax. This configuration deliberately does not load or apply the authored JSON Schema. |
| `json-schema` | `invalid-dialect` | `repair-dialect` | Replace the unsupported Draft 7 dialect with the currently admitted Draft 2020-12 identity. |
| `relax-ng` | `xml-schema` | `validate-xml` | Validate the typed XML vocabulary, start pattern and local declaration syntax without compiling an instance validator. |
| `relax-ng` | `xml-schema` | `parse-xml` | Review typed RELAX NG XML elements, original namespace, source URI and UTF-8 ranges. |
| `relax-ng` | `xml-schema` | `format-xml` | Apply the lexical-lossless native XML formatter; tabular currently preserves all source lexemes. |
| `relax-ng` | `compact-schema` | `validate-compact` | Validate native lexical tokens, delimiters, namespace/start declarations and reference policy. Complete grammar compilation is not claimed. |
| `relax-ng` | `compact-schema` | `parse-compact` | Review lossless native compact tokens and original byte ranges. |
| `relax-ng` | `compact-schema` | `format-compact` | Apply the lexical-lossless native compact formatter with an exact source-preserving fixture. |
| `relax-ng` | `note-data` | `validate-data-syntax` | Validate this file as ordinary XML, separately from the authored RELAX NG schema. Instance validation is unavailable. |
| `relax-ng` | `invalid-xml` | `repair-xml-start` | Restore the local start/ref declaration after the original missing-start diagnostic. |
| `relax-ng` | `invalid-compact` | `repair-compact-close` | Close the note pattern after reviewing its original opening-delimiter range. |
| `relax-ng` | `external-reference` | `repair-reference-policy` | Replace an external reference with an inline pattern. The resolver policy stays reject-only; no remote or filesystem schema is loaded. |
| `module-maps` | `source-v1` | `validate-v1` | Validate the JSON source buffer under the explicit version identity; deployment semantic rules are enforced by graph planning, not this syntax adapter. |
| `module-maps` | `source-v1` | `inspect-v1` | Inspect the generic imported CEM source structure and original ranges; this is not a typed module-map AST or an execution plan. |
| `module-maps` | `source-v1` | `format-v1` | Apply the native tabular JSON source formatter with exact expected bytes; formatting grants no deployment or read authority. |
| `module-maps` | `invalid-syntax-v1` | `repair-v1` | Restore the missing outer object brace after reviewing the original native EOF diagnostic; this does not validate deployment semantics. |
| `module-maps` | `graph-v1` | `build-v1` | Lower the real v1 paired-map contract, copy only declared resources and rewrite the authored HTML import map; the sandboxed preview never executes those modules. |
| `module-maps` | `source-v2` | `validate-v2` | Validate the JSON source buffer under the explicit version identity; deployment semantic rules are enforced by graph planning, not this syntax adapter. |
| `module-maps` | `source-v2` | `inspect-v2` | Inspect the generic imported CEM source structure and original ranges; this is not a typed module-map AST or an execution plan. |
| `module-maps` | `source-v2` | `format-v2` | Apply the native tabular JSON source formatter with exact expected bytes; formatting grants no deployment or read authority. |
| `module-maps` | `invalid-syntax-v2` | `repair-v2` | Restore the missing outer object brace after reviewing the original native EOF diagnostic; this does not validate deployment semantics. |
| `module-maps` | `graph-v2` | `build-v2` | Lower the real v2 paired-map contract, copy only declared resources and rewrite the authored HTML import map; the sandboxed preview never executes those modules. |
| `module-maps` | `source-v3` | `validate-v3` | Validate the JSON source buffer under the explicit version identity; deployment semantic rules are enforced by graph planning, not this syntax adapter. |
| `module-maps` | `source-v3` | `inspect-v3` | Inspect the generic imported CEM source structure and original ranges; this is not a typed module-map AST or an execution plan. |
| `module-maps` | `source-v3` | `format-v3` | Apply the native tabular JSON source formatter with exact expected bytes; formatting grants no deployment or read authority. |
| `module-maps` | `invalid-syntax-v3` | `repair-v3` | Restore the missing outer object brace after reviewing the original native EOF diagnostic; this does not validate deployment semantics. |
| `module-maps` | `graph-v3` | `build-v3` | Lower the real v3 paired-map contract, copy only declared resources and rewrite the authored HTML import map; the sandboxed preview never executes those modules. |
| `module-maps` | `graph-v3` | `validate-graph` | Validate the graph source declaration independently from resolving maps, reading resources or executing its deployment. |
| `module-maps` | `graph-v3` | `inspect-graph` | Inspect the native graph source tree and explicit map/export references without running module deployment. |
| `portable-config` | `card-command` | `validate-command` | Validate literal argv storage, binary identity and supported command/common schema versions. Actual argv grammar is inspected by the shared native command inspector when the project executes it. |
| `portable-config` | `card-check` | `run-check` | Use the native run-config parser and normalized plan through validate --config, then validate only its explicitly configured project-relative XML input. This is execution, not a standalone run-config source adapter. |
| `portable-config` | `project-json` | `validate-project-json` | Validate native project IDs, resource references, relative paths, hashes and entry-artifact declarations without loading the resource bytes or granting execution. |
| `portable-config` | `project-cem` | `validate-project-cem` | Validate the canonical CEM encoding of the same portable project using the native Studio project owner. |
| `portable-config` | `project-cem` | `inspect-project-cem` | Inspect the generic imported CEM structure of the portable project source, including relative resource paths and explicit configuration references; this is not a typed project Parse export. |
| `portable-config` | `project-json` | `json-to-cem` | Use the native Studio project codec to produce the canonical CEM control projection with stable IDs and relative reference paths. |
| `portable-config` | `project-cem` | `cem-to-json` | Convert through the same native project model; normalized resource reference paths are preserved without serializing a parser AST. |
| `portable-config` | `project-cem` | `format-project-cem` | Apply the native tabular project writer with exact expected source bytes. |
| `portable-config` | `invalid-command` | `repair-command` | Restore command schemaVersion 1 after reviewing the native envelope diagnostic. This does not grant unknown argv syntax execution. |
| `portable-config` | `invalid-project` | `repair-project` | Restore portable project schemaVersion 1. Native reference, path, hash and forbidden-state checks remain active. |
| `portable-config` | `card-graph` | `build-card` | Execute the project-relative graph with an explicit heading parameter and HTML entry. Native output spans identify the actual CEM template; source validation alone does not run this build. |
| `native-projections` | `cem-source` | `validate-source` | Validate the original source before creating projections; this is not validation of an exported AST or DOM record. |
| `native-projections` | `xml-source` | `validate-xml` | Check XML well-formedness independently from importing its retained structural view. |
| `native-projections` | `cem-source` | `parse-ast` | Present the original native source nodes and source-frame ranges as CEM-ML text. No JSON AST handoff is used. |
| `native-projections` | `cem-source` | `parse-events` | Present the original native source ordered lexical events, trivia and byte ranges as CEM-ML text. No JSON AST handoff is used. |
| `native-projections` | `xml-source` | `inspect-xml` | Inspect the shared imported CEM tree, retaining XML namespace, comments, CDATA and original byte/line positions. This differs from parsing a serialized projection artifact. |
| `native-projections` | `plain-source` | `inspect-text` | Inspect native text/plain import as one retained text node with its original buffer. This family has no registered schema package or generic source-validation claim. |
| `native-projections` | `cem-source` | `export-dom-json` | Request a named JSON debug export from the original native source. The exported byteRange/sourceMap fields are data; revalidating them does not restore the original source owner. |
| `native-projections` | `cem-source` | `export-dom-binary` | Request exact native binary projection bytes from the original source for download. Studio does not execute configurations whose input is a binary projection. |
| `native-projections` | `cem-source` | `export-ast-json` | Request a named JSON debug export from the original native source. The native AST array export currently differs from the registered document-object validator; no AST JSON round trip is claimed. |
| `native-projections` | `cem-source` | `export-ast-binary` | Request exact native binary projection bytes from the original source for download. Studio does not execute configurations whose input is a binary projection. |
| `native-projections` | `cem-source` | `export-events-json` | Request a named JSON debug export from the original native source. The exported byteRange/sourceMap fields are data; revalidating them does not restore the original source owner. |
| `native-projections` | `cem-source` | `export-events-binary` | Request exact native binary projection bytes from the original source for download. Studio does not execute configurations whose input is a binary projection. |
| `native-projections` | `dom-json` | `validate-dom-json` | Validate the native JSON debug export using the registered projection source validator. This checks its supported shape and grants no decoded runtime import or source ownership. |
| `native-projections` | `invalid-dom` | `repair-dom-json` | Restore the supported document root kind after the original native projection-shape diagnostic. Native JSON projection diagnostics currently have null text coordinates. |
| `native-projections` | `ast-json` | `validate-ast-json` | Validate the separate authored document-object AST fixture using the registered projection source validator. This checks its supported shape and grants no decoded runtime import or source ownership. |
| `native-projections` | `invalid-ast` | `repair-ast-json` | Restore the supported document root kind after the original native projection-shape diagnostic. Native JSON projection diagnostics currently have null text coordinates. |
| `native-projections` | `events-json` | `validate-events-json` | Validate the native JSON debug export using the registered projection source validator. This checks its supported shape and grants no decoded runtime import or source ownership. |
| `native-projections` | `invalid-events` | `repair-events-json` | Restore the supported open event kind after the original native projection-shape diagnostic. Native JSON projection diagnostics currently have null text coordinates. |
| `native-projections` | `invalid-source` | `repair-source` | Review all three original native unclosed-scope diagnostics, including the tokenizer EOF byte/UTF-16 boundary, then restore the missing outer closing brace. |
| `csv-inventory` | `inventory-source` | `validate-csv` | Validate CSV source syntax and declared dialect, separately from inventory count rules. |
| `csv-inventory` | `headerless-source` | `inspect-headerless` | Declared header=absent keeps every record as an array of string cells instead of deriving object keys from its first row. |
| `json-yaml-report` | `api-payload` | `validate-json` | Validate native source syntax; this is not an authored application-schema instance check. |
| `json-yaml-report` | `api-payload` | `format-json` | Format saved source with the native package tabular profile without overwriting it. |
| `json-yaml-report` | `report-settings` | `inspect-yaml` | Inspect the retained generic-data CEM tree and original source offsets, rather than a format-specific source AST. |
| `login` | `login-cem` | `read-edit` | Parse the saved Login source. Its existing generic CEM source identity retains the documented runtime-binding/diagnostic limits; command availability alone is not source conformance. |
| `login` | `login-cem` | `validate` | Validate the saved Login source. Its existing generic CEM source identity retains the documented runtime-binding/diagnostic limits; command availability alone is not source conformance. |
| `login` | `login-cem` | `inspect` | Inspect the saved Login source. Its existing generic CEM source identity retains the documented runtime-binding/diagnostic limits; command availability alone is not source conformance. |
| `login` | `login-cem` | `format-pretty` | Format saved data/login.cem with the native pretty profile. Plain output is available for copy/download; formatting does not overwrite the source. Existing generic CEM source identity retains the documented runtime-binding/diagnostic limits; command availability alone is not source conformance. |
| `login` | `login-cem` | `format-tabular` | Format saved data/login.cem with the native tabular profile. Plain output is available for copy/download; formatting does not overwrite the source. Existing generic CEM source identity retains the documented runtime-binding/diagnostic limits; command availability alone is not source conformance. |
| `login` | `login-cem` | `source-colors` | Export the saved Login source as escaped, colored HTML. Copy/download preserves the native source colors; the sandboxed Studio preview blocks styles. Existing generic CEM source identity retains the documented runtime-binding/diagnostic limits; command availability alone is not source conformance. |
| `login` | `login-html` | `html-conversion` | Convert HTML to CEM-ML. The existing generic CEM source identity retains the documented runtime-binding/diagnostic limits; command availability alone is not source conformance. |
| `login` | `login-cem` | `semantics-cem` | Run the shared 21-field Login contract and compare with expected/semantics.json. Source wrappers and HTML mirrors are mapped explicitly by the query. |
| `login` | `login-html` | `semantics-html-cemql` | Run the equivalent CEM-QL module against the same 21-field contract. Consume ready namespace names explicitly with a bounded navigation budget; source declarations remain inert. |
| `login` | `login-html` | `render-html` | Generate an HTML Login form from the saved source. Edit the template instruction or a source field, save, and rerun. |
| `login` | `login-cem` | `expression-cem` | Evaluate saved queries/summary.cem-ql against this saved Login source. Compare title and required-field count with expected/expression.json; edit the expression through Resources. |
| `login` | `login-expression` | `expression-format-pretty` | Format saved queries/summary.cem-ql without evaluating it or adding a module. Plain output is available for copy/download and never overwrites the source. |
<!-- existing-samples:end -->

The Markdown article operations and complete site build use the same ordinary
project. Standalone inspection/query sees generated HTML; its offsets must not
be presented as a lossless Markdown source AST. The full graph carries original
Markdown metadata/body ranges and separate layout/partial origins into artifact
provenance where the native operation establishes them. Shared command-result
provenance controls open those source resources and ranges. Relative image/link
references remain references in standalone HTML; the full site's artifact tree
supplies the admitted pages and assets without granting ambient network access.

The HTML/CEM card project reuses `cem-card` from the production shared component
library. Its declaration resource is an XML source snapshot; the live view loads
the trusted packaged declaration. Edits to the snapshot do not replace runtime
code or grant new registration authority. The component inspection configuration
uses the default native CEM-ML presentation. Line-fenced literal attributes
preserve both quote styles and embedded rich-content fences without evaluating
the source. Explicit JSON remains a named human-facing export, never an AST
handoff. The native HTML artifact is a separately
authored CEMT projection, rather than whole-page DCE compilation. Login retains
the larger form/semantic comparison and credential recovery policy examples.

The vector/formula projects use genuine SVG/MathML lifecycle inputs through the
common retained tree. Their explicitly authored CEMT HTML wrappers copy the
namespaced source root and omit the source's XML declaration; they do not claim
a universal direct SVG/MathML-to-HTML converter. Native source validation covers
the documented family facts, rather than complete SVG geometry or MathML
vocabulary/arity/algebra semantics. MathML external-annotation diagnostics retain
their warning severity and grant no fetch or execution capability.

The shared artifact viewer admits a [bounded inline SVG/MathML subset](../packages/cem-elements/README.md#inline-svg-and-mathml-artifacts)
independently of native validation. Static geometry, text and presentation math
retain their namespaces. Scripts, handlers, foreign styles, resource URLs,
animation, annotations, unknown elements and namespace integration escapes are
excluded. Exact outputs and native source maps remain available for export even
when a document contains content outside that rendering subset.

The stylesheet projects keep source handling native. CSS Inspect retains lexical
structure, while tree inspection uses the retained CSS namespace. Current native
CSS formatter profiles preserve conservative lexical boundaries rather than
claiming general beautification. The graph exports typed CSS through the same
CSS-owned writer, keeping original source spans; SCSS hands its expanded typed
CSS owner directly to that pipeline without serializing/reparsing CSS.

The SCSS source identity is explicit. Source validation is passive and does not
resolve a module, while Parse/expansion requires an admitted resolver context.
Native CLI use grants a bounded project-root file resolver explicitly; Studio
supplies only owned virtual resources. Expanded Parse offsets describe generated
CSS, not original SCSS bytes; native expansion/module origin frames retain the
original provenance. The sample uses an explicit mixin argument and makes no full
Sass claim. Same-schema SCSS source export, source DomJson/Json and command
HTML-colored CSS output remain unavailable. Optional-profile use requires the
existing trusted source-package preparation; these new project acceptance checks
cover the complete default deployment.

The selector project never treats a stylesheet as a query. It validates and
executes an explicitly identified selector over native HTML/XML owners, preserves
namespace bindings and returns real matched node identities in document order
with duplicates removed. The selected JSON result is a human-facing export;
no JSON tree is fed back as an execution input. Parse/Inspect do not expose a
typed selector source AST, and inspector text envelopes are not selector syntax
trees. Native command source export runs the registered formatter/colorizer
pipeline over borrowed native token owners. Its lexical profiles preserve
spelling, comments, escapes and whitespace without reflow or selector evaluation;
they grant no namespace or host capabilities. The schema's unsupported-feature,
host-capability and work-budget boundaries remain unchanged.

The CEM-QL order project uses one owned JSON source for expression filtering,
local-module inner joins and original imported-node selection. Query record
fixtures retain complete native typed items rather than projecting them into
untyped application records. Native imported-node identities contain runtime-local
addresses and must not become portable identifiers or expected-output fixtures;
only their native representation, original source URI and byte ranges are checked.
Computed records do not acquire invented input field lineage.

Its parameter reports use declared CEMT parameters and a native CEM-QL record/map
inside the template. Explicit graph parameters select ready/draft orders and a
minimum total, and the graph exports ordinary JSON table entries. No query CLI
external-parameter option is implied. These JSON writer outputs currently have
no output spans or sidecar maps; the absence is checked explicitly. Selected
native nodes and query source maps demonstrate the supported provenance paths.
The CLI serializes these abstract JSON primaries with indentation; the shared
worker currently uses compact JSON. Cross-host report acceptance compares the
complete JSON value, while explicit CEM-QL source formatting retains exact byte
fixtures and repeated builds within each host remain byte-stable.

The XPath catalog project filters products, joins suppliers and constructs/looks
up native maps and arrays from an owned XML selection/options subtree. This is
data-driven parameter authoring, not an external XPath variable-binding grant.
Query and standalone source operations consume their caller's named namespaces
and default element namespace. Namespace-configured validation/Parse/format
commands are saved as native authored argv; bare file validation does not inherit
another configuration's bindings. Missing prefixes still diagnose at original
source ranges. EQNames remain useful for context-independent source.
Standalone XPath Parse/Inspect consume the native source owner and expose
original expression/token ranges as CEM-ML. Formatting/coloring run the registered
native lexical-lossless pipeline without evaluating the query or granting
resource access. The distinct result inspector is an actual native query
returning typed map/array items, never a replacement tree.
Selected XPath nodes retain original XML URI/ranges; computed scalars/maps carry
query origins without arbitrary input character mappings.

Shared command publication attaches an original-buffer boundary to text-query
source maps while preserving their existing fine ranges and raw result exports.
Studio resolves those ranges against explicit native source identities, rather
than the active data file. Ambiguous or unknown query identities refuse navigation;
compiled query artifacts never receive fabricated text ranges on their binary
buffers. Existing ownership, revision, draft and UTF-8 guards remain effective.

The ordinary `cem-transform` and `xslt-transform` projects use the same XML order
data and expose explicit HTML entry artifacts. The CEM graph also exports an
application JSON report.
The first graph uses CEM templates for both stages; the second uses XSLT for the
report and CEM for the HTML page. Intermediate results remain native CEM-tree
values. Declaring them as XML would encode text rather than create a new retained
XML owner, and the current XPath/XSLT input adapter does not accept those
constructed sequences. No serialized AST handoff or application-side reimport is
used. JSON report exports retain the previously documented unmapped and
cross-host whitespace limits; HTML artifacts carry native output spans.

The schema-authoring projects separate three contracts. `cem-schema` defines a
note vocabulary, required-id field contract and diagnostic, and supplies its
schema through a local package with an explicit namespace claim. Its schema
sources and its valid/invalid instances have separate configurations.
`json-schema` authors Draft 2020-12 source with local `$defs`/`$ref` declarations;
validation checks source/dialect support without traversing references or
applying instance assertions. `relax-ng` authors XML and compact source, showing
typed Parse, lexical-lossless formatting, local declarations and rejected
external references. Its compact checks remain lexical. Their example JSON/XML
data configurations validate syntax only and do not load the authored schemas.

The shared source editor admits owned XSLT, JSON Schema and both RELAX NG text
MIME types after media-essence normalization. UTF-8, size and source-kind bounds
remain enforced; compiled `+cem-bin` selectors stay read-only with exact downloads.
Text editing does not grant parser execution, schema loading or resolver access.

`module-maps` demonstrates actual v1/v2/v3 paired deployments, including relative
map/resource paths, explicit CSS/JSON/XHTML assets and declared bare module edges.
Standalone validation is JSON syntax validation. Version, key and resource-kind
contracts are checked when the native graph is prepared, before publication.
Opaque deployments have no output spans or sidecar maps. Their native
`moduleAssetManifest` captures source/deployed bytes, lengths, SHA-256, target and
owned URIs. Its hash contract versions 1/2 are independent of map schema versions
1/2/3. The aggregate hash remains stable across unchanged project roots.

The input-root-scope `--module-map` option is a separate unversioned schema-alias
map, accepting string or `uri`/`src`/`path` targets under its alias objects. It does
not execute the paired deployment contracts. Neither mechanism grants reads;
Studio supplies only declared project resources and CLI hosts retain their own
reader/grant policy. The static artifact preview strips scripts and keeps its
sandbox; deployed JavaScript assets are payloads, not Studio UI behavior.

`portable-config` separates a versioned CLI command envelope, its actual native
argv parser, a real `validate --config` run plan and a portable Studio project
model. The existing command inspector displays decoded argv and normalized
inputs. Duplicate run inputs refuse preparation before input reads; that refusal
does not masquerade as a completed engine command. Source version repairs use
the registered command/project source validators. Unsupported generic control
Parse/Inspect/format paths are omitted.

Native Studio JSON/CEM conversion preserves logical resource paths, actual hashes
and authored command references. The shared host provider imports the native
manifest, creates an independent namespace, and exports/reimports its real source
archive with revision guards. There is no invented CLI archive command or JSON
AST execution handoff. Saved files and configurations travel in source exports;
credentials, runtime bindings, selections and drafts are not embedded as host state.

`native-projections` compares parser-owned CEM AST/events with retained XML
inspection and a plain-text loader input. Human-facing views default to typed
CEM-ML presentation. Six explicit JSON/binary exports have exact native CLI
fixtures. Binary bytes match across hosts; shared worker JSON publication uses
compact serialization of the same full value rather than the CLI's indentation.
Same-host repeat/offline results remain byte-stable. Sealed native chunk replay
remains a typed engine operation. No JSON AST
parse/serialize/reparse step supplies evaluation or rendering. Original UTF-8
ranges remain distinct from numeric fields inside imported projection JSON.
Parse/Inspect retains those coordinates in its CEM-ML presentation but currently
publishes no owned artifact-source references to the Provenance panel. This
project follows actual native diagnostic ranges through the shared Diagnostics
selector; it never manufactures links from inspection data fields.

DOM/events JSON exports pass their registered shape validators. The native AST
debug writer emits an array, but the AST JSON validator accepts a document
object; its authored valid-object fixture is separate from the unchanged native
export. Projection input adapters remain output-only, and binary validators
currently check headers rather than payload/chunk integrity. Unsupported
projection-source Inspect/conversion/replay configurations are omitted.

Catalog and saved binary sources use the shared preview admission policy, remain
non-editable/non-runnable and download exact owned bytes. Catalog file transport
does not decode or prepare them as text. Binary exports from admitted text source
remain explicit downloadable operation results. Four original source/shape
repairs, saved Unicode edits, independent copies and offline checks use existing
shared editors, command controls, source navigation and result viewers.

The following rows describe existing reuse and planned extensions, rather than
additional routes. Category names are discovery metadata and do not affect
runtime authority.

| Category | Families / selectors to cover | Reuse or planned workflow |
| --- | --- | --- |
| Data | JSON and YAML aliases; CSV | Reuse `json-yaml-report` and `csv-inventory` for imported data, explicit queries/formatting and reports. CSV covers header-mode/row-width warnings and quote failures with repair, compact interchange versus tabular review, and explicit count rules. Reuse Supplier for cross-source reconciliation. |
| Data / Documents and UI | XML, XHTML and XML aliases | Reuse Supplier XML validation/namespace-aware selection and the `xhtml-document` query, repair and HTML graph. XML source validation does not establish domain instance rules; XHTML retains its own owner and source contract. Keep DTD/entity refusals visible. |
| Documents and UI | Markdown | Reuse `markdown-site` for saved-article validation, generated-HTML inspection/query, exact standalone preview and metadata failure/repair; its folder graph publishes complete site outputs and content/layout provenance. |
| Documents and UI | HTML, CEM-ML, native/element templates, runtime data island | Reuse Login and `html-cem-card` for semantic comparisons, shared card declarations/slots, local slice bindings and data-to-HTML generation. Keep project declaration authority and automatic DCE compilation limits explicit. |
| Documents and UI | SVG and MathML variants | `svg-diagram` and `mathml-formula` cover native retained inspection/query/formatting, validation/repair, HTML artifacts, original provenance and bounded actual vector/presentation-math rendering. Content MathML is validated separately without an algebra or presentation claim. |
| Documents and UI | CSS and SCSS aliases | `css-stylesheet` and `scss-stylesheet` cover native inspection, validation/repair, exact CSS output, scoped styles, owned-module expansion, native HTML/CSS graphs and shared previews. Keep unavailable SCSS source-export/loader cases explicit. |
| Queries and transformations | CSS selectors, XPath source aliases, CEM-QL modules/expressions | `css-selector`, `xpath-catalog` and `cemql-orders` cover native selections, namespace context, joins, input-derived/native parameter maps, typed results and original selected-node ranges. XPath and CEM-QL source operations consume their typed owners; CSS-selector and XPath formatting/coloring preserve lexical source. XPath source commands retain their saved caller namespace bindings. |
| Queries and transformations | CEM transformation and XSLT source/custom-element selectors | `cem-transform` and `xslt-transform` compare ordinary multi-step native builds, explicit HTML entries, source controls, parameter maps and provenance. XSLT execution requires the complete profile; custom-element mode is not a universal XSLT alias. |
| Schema authoring | CEM schema and schema package | `cem-schema` separates schema/package authoring from valid/invalid note instances, with explicit namespace claims and required-field diagnostics. |
| Schema authoring | JSON Schema and both Relax NG syntaxes | `json-schema` and `relax-ng` cover source validation, supported formatting/typed Parse, dialect/start/delimiter repairs and rejected external references. Their data examples validate syntax only. |
| Schema authoring | Module maps v1/v2/v3, CLI command, Studio project encodings | `module-maps` and `portable-config` cover real paired deployments, native asset identities, bounded reads, control codecs/plans, portable manifests, independent imports and guarded source archives. Source syntax/envelope checks stay separate from execution and read authority. |
| Schema authoring | DOM/AST/events binary/JSON projections | `native-projections` compares original CEM Parse, retained XML inspection, explicit JSON/binary exports, source ranges and registered shape validation. Binary source downloads stay read-only; codec/AST-shape limits remain explicit. |
| Data | UTF-8 `text/plain` loader input | `native-projections` inspects one owned plain-text resource through CEM-ML. It has no registered schema package, standalone validation or general formatter claim. |

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
the authored Login, Supplier, data-report, XHTML, Markdown, HTML/CEM card,
SVG/MathML, CSS/SCSS/selector, CEM-QL/XPath, transformation/schema-authoring,
module-map, portable-control and native-projection workflows;
site preview/build browser acceptance also lives in `test:workbench`, with deployed
article/site and offline checks in the navigation gate. The scoped
`verify:module-configs` gate checks actual native module asset identities,
independent map edits and offline deployments, plus native project codecs,
guarded provider import/export and saved run configurations. The scoped
`verify:native-projections` gate checks original-source views/ranges, exact binary
downloads, projection-source guards, repairs and independent offline copies. These checks
establish the covered operations; they do not turn untested aliases or planned
workflows into supported commands.

When packages change, regenerate/check the native registry, update this audit,
then run the existing Feature Tour and output gates. Keep sample definitions in
[examples/projects](../examples/projects/README.md) and generated Schema examples
in their existing pipeline. This document is planning/verification evidence, not
a second runtime dispatcher or an activation allowlist.
