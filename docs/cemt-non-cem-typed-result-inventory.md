# Non-CEM CEMT Typed-Result Inventory

Status: inventory complete; serializer-free typed-result contract selected and
materialized-tree artifact introduced. The lossless JSON formatter/colorizer/
writer path now uses the borrowed evaluator, materialized writer-token stream,
and typed color overlay end to end. JSON graph routing and generic-data JSON
ingress remain open. This inventory is promoted as active migration evidence by
`docs/todo.md`.

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
| DOM-projection parity adapter                               | Either `Arc<CemTreeAstStream>` or an explicit JSON DOM projection                                                                               | A newly generated raw CEM tree                                                                | `CemtOutputArtifact { value: Value }`                                                                                              | The generated tree has no typed result owner; the JSON-input branch has no package-native owner and currently emits no source map.                                         |
| Lossless JSON direct pipeline                              | `JsonDocumentAst`                                                                                                                               | `CemtMaterializedTreeArtifact` owning ordered `WriterToken` nodes, plus an optional typed color overlay                               | Borrowed evaluator → exact `Arc<CemTreeAstStream>` → overlay → direct writer and typed stage output; real graph collection/secondary routing retains the exact artifact and owner `Arc` | Production formatter, colorizer, writer, stage, graph, join, and secondary-input handoffs are closed; generic-data JSON ingress still uses compatibility. |
| Direct CSV, YAML, JSON Schema, and Markdown pipelines      | `CsvDocumentAst`, `YamlDocumentAst`, `JsonSchemaDocumentAst`, `MarkdownDocumentAst`, or a generic-data compatibility subject                    | A newly materialized formatted tree, optionally followed by a newly materialized colored tree                                      | `TransformTemplateOutputFunctionExecution::CemtEvaluator(Value)` and `TransformTemplateEncodedArtifactPayload::CemtRuntime(Value)`                                      | Each remaining `*DocumentOutputSubject::into_cemt_subject` consumes the native AST into a DTO value before evaluation, so neither the source owner nor its native identity survives.       |
| XML-family direct pipelines                                 | `XmlDocumentAst`, `HtmlDocumentAst`, `CssDocumentAst`, `XhtmlDocumentAst`, `SvgDocumentAst`, `MathMlDocumentAst`, or `XsltStylesheetAst`        | A newly materialized package-specific formatted/colored tree                                  | The same evaluator/runtime value envelopes                                                                                         | The common `XmlDocumentOutputSubject` erases seven distinct AST owners before formatting. The output is not an overlay over a raw `CemTreeAstStream`.                      |
| Relax NG direct pipeline                                    | `RelaxNgDocumentAst`, with XML and compact syntax selecting different formatter/colorizer contracts                                             | A newly materialized formatted/colored tree                                                   | The same evaluator/runtime value envelopes                                                                                         | The original syntax owner and syntax kind are lost when the formatter subject is built; stage metadata is inferred later from binding/value shape.                         |
| Generic CEMT output-function runtime                        | Explicit JSON subject and value bindings                                                                                                        | Any declared CEM-tree formatter/colorizer result                                              | `TransformTemplateOutputFunctionExecution::CemtEvaluator(Value)`                                                                   | The stage is carried by the selected binding while the payload remains untyped. Format-to-color chaining clones the value instead of retaining a typed result artifact.    |
| Compatibility CEM-tree stage fallback                       | Any primary body other than the native `CemtTreeArtifact` representation                                                                        | Formatter/colorizer evaluator output                                                          | `lower_conversion_cem_tree_output_stage_body` falls back to `CemtOutputArtifact`                                                   | A single value envelope hides whether the result was generated raw, materialized formatted, or materialized colored.                                                       |

Production occurrences of the legacy result envelopes are confined to
`packages/cem_ml/src/conversion.rs` and
`packages/cem_ml/src/transform_template.rs`. Test-only `Value` subject
implementations exercise the same compatibility boundary and must not become a
production owner variant.

## Remaining Consumers

| Consumer                                      | Current behavior                                                                                                          | Typed requirement                                                                                                                              |
| --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Conversion parity and direct converter output | `transform_template_output_cemt_subject` downcasts `CemtOutputArtifact` and clones its value.                             | Accept a closed typed tree result and preserve its owner/source-map identity through the output pipeline.                                      |
| Generic format-to-color chaining              | `evaluate_transform_template_encode_expressions` extracts and clones a formatter value to create the color request.       | Pass the formatted typed artifact directly and expose a typed evaluator view.                                                                  |
| Compatibility writer                          | `transform_template_writer_cem_tree_artifact_to_text` reads `CemtRuntime(Value)` and validates its shape at the consumer. | Dispatch on a declared typed stage and reject an invalid stage at producer completion.                                                         |
| Transform graph stage routing                 | The JSON pipeline returns the selected artifact as `TransformTemplateOutputArtifact { body: MaterializedCemtTree(..) }`; `transform_data_artifact_from_output` retains that exact `Arc`. | Apply the same typed stage-output contract to the remaining package producers.                                                                  |
| Graph joins                                   | A real ordered `TransformArtifactCollection` run retains the JSON data artifact, materialized artifact, owner, and declared child order.          | Keep the closed routing test as a regression gate while migrating the remaining typed tree producers.                                         |
| Secondary-input and encode-expression binding | JSON secondary-input adapter dispatch receives the exact same data artifact/materialized artifact/owner as the collection; encode-expression bindings still accept only explicit JSON. | Keep native typed bodies at adapter dispatch. Add borrowed evaluator bindings only for a package whose expression contract requires them; never project the body through JSON. |
| Public conversion boundary                    | `conversion_output_boundary_value` returns JSON values for CEM-tree and extension bodies.                                 | Lower only at an explicit registered exporter/public response boundary, not between formatter, colorizer, writer, or graph stages.             |

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

Next, replace the generic-data-to-JSON compatibility projection with a typed
evaluator view, closing every production JSON ingress before moving to JSON
Schema.

After this first end-to-end producer passes source audits and the full
verification matrix, migrate every remaining producer using the same direct
owner/view/builder pattern. Only then remove `CemtOutputArtifact`,
`transform_template_output_cemt_subject`, `CemtEvaluator(Value)`,
`CemtRuntime(Value)`, and adapter DTO conversions globally.
