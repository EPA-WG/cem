# Non-CEM CEMT Typed-Result Inventory

Status: inventory complete; serializer-free typed-result contract selected and
materialized-tree artifact introduced. The generic CEMT output-function
runtime now returns a closed native-or-CEM-tree result, lowers tree results at
producer completion, and passes typed raw/formatted/colored/materialized
artifacts directly between formatter, colorizer, and writer. The compatibility
stage fallback and its generic CEM-tree DTO are deleted.
Formatter/colorizer/writer paths for lossless and generic-data JSON, JSON
Schema, CSV, YAML, Markdown, all seven XML-family owners, both RELAX NG syntax
branches, and the DOM-projection native producer now use borrowed
evaluators/subjects and typed tree results end to
end. The CEM-QL direct-output bridge also exposes its package-owned token AST
through the extensible borrowed evaluator contract and enters the typed
materialized pipeline without a JSON DTO; JSON graph routing is closed. This
inventory is promoted as active migration evidence by `docs/todo.md`.

## Existing Typed Baseline

`CemtTreeArtifact` has one concrete lifecycle: it retains an owning
`Arc<CemTreeAstStream>` and represents raw, formatted, or colored stages by
adding typed formatter and colorizer overlays over that same owner. Its writer
can therefore traverse the original nodes while applying generated operations
without reconstructing a materialized tree.

That lifecycle is correct for native CEM formatting and coloring. It does not
describe every remaining producer.

## Remaining Producers

| Producer family                                             | Input owner before execution                                                                                                                    | Current result                                                                                | Current handoff                                                                                                                    | Ownership/provenance gap                                                                                                                                                   |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Lossless, generic-data JSON, and JSON Schema pipelines     | `JsonDocumentAst`, `GenericDataDocumentAst`, or `JsonSchemaDocumentAst`                                                                          | `CemtMaterializedTreeArtifact` owning ordered `WriterToken` nodes, plus an optional typed color overlay                               | Borrowed evaluator → exact `Arc<CemTreeAstStream>` → overlay → direct writer and typed stage output; real JSON graph collection/secondary routing retains the exact artifact and owner `Arc` | All production JSON and JSON Schema formatter, colorizer, writer, and stage handoffs are closed; production serializers are deleted and compatibility subjects remain test-only.          |
| Direct CSV pipeline                                        | `CsvDocumentAst` or a borrowed CSV-contract view over `GenericDataDocumentAst`                                                                    | `CemtMaterializedTreeArtifact` owning ordered `WriterToken` nodes, plus an optional typed color overlay                              | Borrowed evaluator → exact `Arc<CemTreeAstStream>` → overlay → typed stage output and direct writer                                                                       | Closed for both production owners; compatibility `Value` subjects and composers are test-only parity oracles.                                                             |
| Direct YAML pipeline                                      | `YamlDocumentAst` or a borrowed YAML-contract view over `GenericDataDocumentAst`                                                                  | `CemtMaterializedTreeArtifact` owning ordered `WriterToken` nodes, plus an optional typed color overlay                              | Borrowed evaluator → exact `Arc<CemTreeAstStream>` → overlay → typed stage output and direct writer                                                                       | Closed for both production owners; compatibility `Value` subjects and composers are test-only parity oracles.                                                             |
| Direct Markdown pipeline                                  | `MarkdownDocumentAst`                                                                                                                             | `CemtMaterializedTreeArtifact` owning ordered `WriterToken` nodes, plus an optional typed color overlay                              | Borrowed evaluator → exact `Arc<CemTreeAstStream>` → overlay → typed stage output and direct writer                                                                       | Closed for the sole production owner; compatibility `Value` subjects and composers are test-only parity oracles.                                                          |
| XML-family direct pipelines                                | `XmlDocumentAst`, `HtmlDocumentAst`, `CssDocumentAst`, `XhtmlDocumentAst`, `SvgDocumentAst`, `MathMlDocumentAst`, or `XsltStylesheetAst`                                                                          | `CemtMaterializedTreeArtifact` owning ordered `WriterToken` nodes, plus an optional typed color overlay                              | Closed native-owner sum → borrowed evaluator → exact `Arc<CemTreeAstStream>` → overlay → typed stage output and direct writer                                              | Closed for all seven production owners; every compatibility composer family, including XML, is test-only.                                                                  |
| Relax NG direct pipeline                                    | `RelaxNgDocumentAst`, with XML and compact syntax selecting different formatter/colorizer contracts                                             | `CemtMaterializedTreeArtifact` owning ordered `WriterToken` nodes, plus an optional typed color overlay                              | Borrowed syntax-preserving evaluator → exact `Arc<CemTreeAstStream>` → overlay → typed stage output and direct writer                                                       | Closed for both syntax branches; RELAX NG and nested XML compatibility composers are test-only parity oracles.                                                              |
| CEM-QL direct-output bridge                                  | Package-owned CEM-QL lexer token AST with exact ranges, cooked values, roles, source maps, and output spans                                      | `CemtMaterializedTreeArtifact` owning ordered `WriterToken` nodes, plus an optional typed color overlay                              | Extensible borrowed package record/sequence view → exact `Arc<CemTreeAstStream>` → optional overlay → typed stage output and direct writer                                  | Closed for production direct text and HTML output; the former token-tree `serde_json::Value` DTO and generic output-pipeline handoff are deleted.                            |
| Generic CEMT output-function runtime                        | Borrowed package evaluator values or a typed CEM-tree artifact                                                                                   | Closed native payload or declared raw/formatted/colored/materialized CEM-tree artifact         | Typed result is lowered at producer completion and the exact artifact is passed directly to the next stage                         | Closed. A public-JSON producer cannot claim a CEM-tree runtime result; explicit JSON must enter through a registered parser.                                               |
| Removed compatibility CEM-tree stage fallback               | Formerly accepted any non-native primary body                                                                                                    | No fallback result                                                                             | The generic DTO fallback and value-shape recovery path are deleted                                                                  | Closed. Undeclared, ambiguous, and public-JSON CEM-tree stage results are rejected at the producer/adapter boundary.                                                       |

The legacy `CemtOutputArtifact`, `CemtEvaluator(Value)`, `CemtRuntime(Value)`,
and `transform_template_output_cemt_subject` cross-tier contracts are deleted.
Test-only `Value` subjects remain compatibility oracles and must not become a
production owner variant. `PublicJson` remains valid only for explicit parser,
public response, debug, and registered JSON-export boundaries.

## Remaining Consumers

| Consumer                                      | Current behavior                                                                                                          | Typed requirement                                                                                                                              |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Conversion parity and direct converter output | Accepts the closed typed tree result and retains its owner/source-map identity through the output pipeline.               | Keep typed entrypoints as the only successful CEM-tree stage handoff.                                                                           |
| Generic format-to-color chaining              | Passes the formatted typed artifact directly and exposes its borrowed typed evaluator view.                              | Keep JSON/public projections outside the chaining path.                                                                                         |
| Typed writer                                  | Dispatches on formatted/colored typed artifacts and rejects raw or stage/profile mismatches before rendering.            | Remove the now-dead compatibility value writer helpers after their remaining test oracles migrate.                                              |
| Transform graph stage routing                 | The JSON pipeline returns the selected artifact as `TransformTemplateOutputArtifact { body: MaterializedCemtTree(..) }`; `transform_data_artifact_from_output` retains that exact `Arc`. | Apply the same typed stage-output contract to the remaining package producers.                                                                  |
| Graph joins                                   | A real ordered `TransformArtifactCollection` run retains the JSON data artifact, materialized artifact, owner, and declared child order.          | Keep the closed routing test as a regression gate while migrating the remaining typed tree producers.                                         |
| Secondary-input and encode-expression binding | JSON secondary-input adapter dispatch receives the exact same data artifact/materialized artifact/owner as the collection; encode-expression bindings still accept only explicit JSON. | Keep native typed bodies at adapter dispatch. Add borrowed evaluator bindings only for a package whose expression contract requires them; never project the body through JSON. |
| Public conversion boundary                    | Projects typed tree owners and overlays to JSON only after writer execution for public/debug responses.                  | Keep this one-way projection out of formatter, colorizer, writer, graph, and secondary-input execution.                                         |

## Contract Assessment

The remaining producers do not share the existing `CemtTreeArtifact`
invariants:

- native CEM formatting retains one raw CEM-tree owner and applies overlays;
- package formatters generate a new materialized tree from heterogeneous native
  document owners;
- DOM compatibility can generate a tree from explicit JSON without any native
  package owner; and
- graph/secondary-input routing preserves arbitrary native bodies while the
  evaluator binding layer currently requires explicit JSON.

Extending `CemtTreeArtifact` would therefore require a closed owner/lifecycle
sum type with separate overlay-backed and materialized branches. Treating those
differences as optional fields would make stage validity dependent on runtime
field combinations and is rejected by the migration acceptance criteria.

## Resolved Contract

No serializer, encoder, `serde_json::Value` projection, DTO, or re-parser may
sit between transformation layers. The native stream handed off by one layer
is the stream consumed by the next layer.

The materialized-tree contract is therefore fixed as follows:

1. A package formatter evaluates through a borrowed typed view over the owning
   `Arc` of its native package AST. It must not call `to_cemt_subject()`,
   `into_cemt_subject()`, `serde_json::to_value()`, or an equivalent DTO
   projection.
2. The formatter constructs and owns an `Arc<CemTreeAstStream>` result directly
   from typed evaluator nodes. The result retains typed input provenance; it
   does not recover provenance by serializing and parsing the package AST.
3. The colorizer receives that exact formatted AST-stream `Arc` through a typed
   artifact view and returns a typed colored AST stream or overlay. The writer
   consumes that typed result directly.
4. Materialized CEMT trees receive a first-class `TransformArtifactBody`
   variant. Graph stages, ordered joins, and secondary inputs dispatch
   exhaustively on the variant and preserve its `Arc` identity instead of
   relying on extension downcasts.
5. `CemtTreeArtifact` remains the owner-plus-overlay contract for native CEM.
   A separate closed materialized-tree artifact carries the package/result
   identity, typed stage, output-function identity, result
   `Arc<CemTreeAstStream>`, source-map provenance, and output spans for package
   formatters that generate a new tree.
6. Serialization is permitted only at a registered external encoding/export
   boundary. JSON and `+json` inputs parse once into their native lossless AST;
   JSON and `+json` outputs serialize once after the final typed artifact.

The explicit-JSON DOM compatibility branch must either enter through a parser
edge that creates an AST stream before transformation begins or be removed from
production and retained only as a test fixture. It cannot be an owner variant
or an intermediate transformation representation.

## Materialized Writer-Token Resolution

The JSON formatter does not return DOM-shaped CEM nodes. It returns ordered
writer-token records with token kind, text, role, style, value metadata, source
map, and output span. The former `CemTreeAstNode` algebra could not retain that
information: mapping tokens to `Text` or `RawText` would lose colorizer and
writer semantics, while retaining the records as `serde_json::Value` would
reintroduce the prohibited intermediate DTO.

The recommended resolution is now implemented. `CemTreeAstNode::WriterToken`
owns concrete token kind/text/role, style, formatter metadata, source-range,
source-map, and output-span fields. Its evaluator record borrows those fields
directly, including nested style, metadata, range, and output-span records;
there is no serializer or `Value` projection in that view.

Colored materialized results retain the formatted owner and attach a
`CemtMaterializedTreeColorOverlay` keyed by `CemtOwnerPath`.
`CemtMaterializedTreeArtifact::new_colored` validates the colorizer producer,
target identity, writer-token target kind, unique targets, color role/profile,
and output style while preserving the exact formatted
`Arc<CemTreeAstStream>`. The separate materialized-token-stream alternative is
rejected because it would change the selected owner and graph contract.

## Implementation After Decision

The lossless JSON formatter now lowers the package CEMT result directly into
writer-token nodes, the colorizer receives the exact formatted owner and emits
only the typed overlay, and the writer traverses those typed artifacts. The
selected formatted/colored result is also returned as a first-class production
stage output body. Real graph execution proves exact data-artifact,
materialized-artifact, and owner identity through ordered collection and named
secondary-input handoffs in both formatted-only and colored-overlay modes. The
`JsonDocumentAst` production path no longer calls its subject serializer.

Generic-data JSON ingress now uses a borrowed evaluator over
`GenericDataDocumentAst` directly. Ordered and duplicate mapping entries,
generated or missing member names, normalized JSON number lexemes, source
ranges/maps, and the original owner survive without a `JsonDocumentAst`,
`serde_json::Value`, DTO, or serialization/reparse boundary. CSV and YAML
production conversions select the same typed materialized JSON pipeline, and
the former generic-data compatibility projection has been deleted.

JSON Schema now applies the same owner/view/materialized-result pattern.
`JsonSchemaDocumentCemtSubjectRef` borrows the outer owner and the existing
lossless JSON view while retaining source parameters, dialect, parse facts,
dialect facts, ranges, and maps. Its formatter and colorizer produce typed
materialized artifacts; the writer consumes their exact AST stream, including
tabular close-scope compaction through a typed token plan. The production
`JsonSchemaDocumentAst` serializer has been removed, while a test-only `Value`
pipeline remains solely as the byte-parity oracle.

CSV now applies the same contract to both production owners.
`CsvDocumentCemtSubjectRef` borrows the lossless native table, including source,
encoding, dialect, rows, fields, exact lexemes, ranges, maps, and facts.
`GenericDataCsvDocumentCemtSubjectRef` exposes the existing CSV table projection
lazily over `GenericDataDocumentAst`, including header union/deduplication,
first-duplicate selection, ragged missing cells, scalar coercion, exact number
lexemes, generated ranges/maps, and document order. Both paths produce the
materialized writer-token stream, retain its exact owner through the typed color
overlay and stage body, and use the direct writer. Production CSV serializers
are gone; source audits keep the compatibility composer test-only.

YAML now closes both production owners. `YamlDocumentCemtSubjectRef` borrows
the lossless stream, including source and encoding reports, facts, directives,
comments, documents, tags, anchors, aliases, scalar styles and exact lexemes,
ranges, and maps. `GenericDataYamlDocumentCemtSubjectRef` exposes the YAML
contract lazily over ordered generic-data documents, duplicate mappings,
sequences, aliases, nulls, and exact number lexemes. Both paths use the typed
materialized formatter/colorizer/writer lifecycle and retain the exact owner in
the stage body; production YAML composers are test-only parity oracles. The
YAML formatter assets also now use valid binary `extend` calls for root scalar
and alias branches.

Markdown now closes its sole production owner.
`MarkdownDocumentCemtSubjectRef` borrows source and encoding metadata,
CommonMark/GFM variant and parse facts, ordered events, optional event fields,
ranges, maps, and line-ending policy. Its formatter and colorizer use the typed
materialized lifecycle, retain the exact owner through the selected stage body,
and invoke the direct writer; production Markdown DTO composers are test-only
parity oracles.

The shared XML-family boundary is now closed for `XmlDocumentAst`,
`HtmlDocumentAst`, `CssDocumentAst`, `XhtmlDocumentAst`, `SvgDocumentAst`,
`MathMlDocumentAst`, and `XsltStylesheetAst`. The closed
`XmlFamilyDocumentCemtSubjectRef` preserves each exact owner while exposing
borrowed source, encoding, fact, event, namespace, range/map, and package layout
semantics. Formatter and colorizer results lower directly into the typed
materialized tree, the direct writer consumes that tree plus its overlay, and
the typed stage body retains the exact selected artifact and owner `Arc`.
Compatibility-oracle parity covers all seven owners, including SVG/MathML
markup-token and structural-layout derivation. The production subject trait no
longer serializes any of these owners, and the XML composer is test-only.

RELAX NG now closes both syntax branches. `RelaxNgDocumentCemtSubjectRef`
borrows the exact `RelaxNgDocumentAst` and exposes syntax kind, source/media
parameters, facts, XML events or compact tokens, ranges/maps, and line endings.
The syntax-selected formatter materializes typed writer tokens, including
typed `syntaxKind` metadata; coloring retains the exact formatted owner through
an owner-path overlay, the selected artifact is the typed stage body, and the
direct writer consumes it. RELAX NG and nested XML compatibility composers are
test-only parity oracles.

`DomProjectionParityCemtAdapter` is now closed for native production input. It
borrows `CemtTreeSubjectRef`, retains the exact input
`Arc<CemTreeAstStream>` and source maps, adds typed retained-node/layout
operations, returns a formatted `CemtTreeArtifact`, and enters color/writer
execution through the typed formatted-stage entrypoint. Directive elision is
tracked by owner path rather than value-shape recovery. The explicit-JSON
branch remains compatibility-only and never becomes a native owner.

The CEM-QL direct-output bridge now owns `CemQlSourceTokenTreeAst` and exposes
it through core-owned borrowed package record/sequence traits. Formatter
execution reads the lexer tokens, cooked values, roles, exact byte ranges,
source maps, and output spans without constructing `serde_json::Value`; the
formatter result is lowered immediately into an owned
`Arc<CemTreeAstStream>`. Color selection either retains that exact owner with a
typed overlay or skips coloring for the `none` profile, and the materialized
writer consumes the selected typed artifact directly. Source audits reject a
return to the former token-tree JSON builder or generic runtime pipeline.

Next, delete the remaining legacy `PublicJson` CEM-tree compatibility APIs and
their value-based runtime/writer helpers. Migrate or remove test-only parity
oracles that still construct formatted/colored JSON artifacts, keep real JSON
ingress behind a registered parser, and preserve `PublicJson` only for actual
JSON/public/debug output. The direct typed formatter → colorizer → writer path
is now the contract to retain.
