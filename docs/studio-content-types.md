# Studio content types and schema inventory

Audited 2026-10-25. This is the implementation map for the **Content types and
schemas** sample collection in [the active checklist](todo.md#studio-samples-supported-content-types-and-schemas).
The collection is available under **Examples → Content types and schemas**, with
Data, Documents and UI, Queries and transformations, and Schema authoring folders.
Examples remain ordinary cloneable projects with resources and configurations.
Folder listings reuse existing projects; the missing functional workflows below
remain the next implementation steps.

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
| CSS selector | Source/expression validation; not data. | Registered source profiles; command source conversion is currently unavailable, and Inspect AST is a generic text envelope. | Explicit canonical MIME or schema selects element-tree query dispatch; supplied MIME/schema must agree. No general-purpose transformation language. |
| XPath | Expression validation and native query compilation; not generic data. | Native typed query results and registered source-profile assets; command source Parse/Inspect/format are not admitted as typed XPath source operations yet. | XPath/XDM-tree query or direct transformation. Binary artifacts and JSON results are not XPath source text. Supported native subset applies. |
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
| `xpath-catalog` | `catalog-source` | `query-filter` | Use namespace-qualified paths, native numeric comparison and a simple map to report products with stock at least four. |
| `xpath-catalog` | `catalog-source` | `query-nodes` | Export genuine retained product nodes with stable native identities, original input URI, original opening-element ranges and source maps. |
| `xpath-catalog` | `catalog-source` | `query-join` | Evaluate dependent for bindings over the same original XML owner, joining supplier IDs with an explicit static namespace binding. |
| `xpath-catalog` | `catalog-source` | `query-parameters` | Build a native XPath map from the XML selection attributes, then filter and join through native map lookups. These are owned data parameters, not external variable bindings. |
| `xpath-catalog` | `catalog-source` | `inspect-parameters` | Export the typed native map, integer, strings and array with their real query-origin frames. Computed items do not claim arbitrary data-source character lineage. |
| `xpath-catalog` | `invalid-query` | `repair-query` | Close the missing predicate delimiter and revalidate the same saved source. Its independent failure copy does not affect normal query inputs. |
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
names JSON explicitly because the current CEM-ML attribute-string writer refuses
mixed single/double quotes in that embedded template source. This structural
export is a human-facing result, never an AST handoff. Other inspection commands
retain their normal CEM-ML presentation. The native HTML artifact is a separately
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
no JSON tree is fed back as an execution input. Inspector text envelopes are not
selector syntax trees, and registered selector formatters do not currently imply
working command source export. The schema's unsupported-feature, host-capability
and work-budget boundaries remain unchanged.

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
Query namespace bindings work; standalone source validation currently omits caller
prefix bindings, so its independently validated source uses explicit EQNames.
Standalone XPath Parse/Inspect/format still have native routing/export gaps and
are excluded from the project's advertised operations. The result inspector is
an actual native query returning typed map/array items, never a replacement tree.
Selected XPath nodes retain original XML URI/ranges; computed scalars/maps carry
query origins without arbitrary input character mappings.

Shared command publication attaches an original-buffer boundary to text-query
source maps while preserving their existing fine ranges and raw result exports.
Studio resolves those ranges against explicit native source identities, rather
than the active data file. Ambiguous or unknown query identities refuse navigation;
compiled query artifacts never receive fabricated text ranges on their binary
buffers. Existing ownership, revision, draft and UTF-8 guards remain effective.

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
| Queries and transformations | CSS selectors, XPath source aliases, CEM-QL modules/expressions | `css-selector`, `xpath-catalog` and `cemql-orders` cover native selections, namespace context, joins, input-derived/native parameter maps, typed results and original selected-node ranges. CEM-QL source/module operations retain their explicit identities; XPath source-command limits remain visible. |
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
the authored Login, Supplier, data-report, XHTML, Markdown, HTML/CEM card, SVG/MathML, CSS/SCSS/selector and CEM-QL/XPath data workflows;
site preview/build browser acceptance also lives in `test:workbench`, with deployed
article/site and offline checks in the navigation gate. These checks establish the
covered operations; they do not turn untested aliases or planned workflows into
supported commands.

When packages change, regenerate/check the native registry, update this audit,
then run the existing Feature Tour and output gates. Keep sample definitions in
[examples/projects](../examples/projects/README.md) and generated Schema examples
in their existing pipeline. This document is planning/verification evidence, not
a second runtime dispatcher or an activation allowlist.
