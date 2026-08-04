# Non-CEM CEMT Typed-Result Inventory

Status: inventory complete; typed-result contract not yet selected for
production. This inventory is promoted as active migration evidence by
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
| Direct CSV, YAML, JSON, JSON Schema, and Markdown pipelines | `CsvDocumentAst`, `YamlDocumentAst`, `JsonDocumentAst`, `JsonSchemaDocumentAst`, `MarkdownDocumentAst`, or a generic-data compatibility subject | A newly materialized formatted tree, optionally followed by a newly materialized colored tree | `TransformTemplateOutputFunctionExecution::CemtEvaluator(Value)` and `TransformTemplateEncodedArtifactPayload::CemtRuntime(Value)` | Each `*DocumentOutputSubject::into_cemt_subject` consumes the native AST into a DTO value before evaluation, so neither the source owner nor its native identity survives. |
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
| Transform graph stage routing                 | `transform_data_artifact_from_output` already clones `TransformArtifactBody`, preserving native `Arc` identity.           | Route the chosen tree artifact body without converting it to an adapter DTO.                                                                   |
| Graph joins                                   | `TransformArtifactCollection` preserves each child body and order.                                                        | Keep each typed tree child intact; do not flatten the collection through JSON.                                                                 |
| Secondary-input and encode-expression binding | Secondary artifacts retain native bodies, but `transform_template_render_value_bindings` accepts only explicit JSON.      | Define whether CEMT expressions receive a borrowed typed artifact view or whether typed bodies are restricted to adapter-level stage dispatch. |
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

The recommended direction is a second closed materialized-tree artifact family,
leaving `CemtTreeArtifact` as the owner-plus-overlay contract for native CEM.
The materialized family should contain a typed stage, output-function identity,
package/result identity, an owned `Arc<CemTreeAstStream>` result, source-map and
output-span provenance, and a closed owner descriptor. Writers, graph routing,
and public exporters can then dispatch over the two explicit artifact families.

## Decisions Required Before Implementation

Two choices change the artifact's fields and routing contract, so production
implementation must stop until they are resolved:

1. Define retained ownership for a materialized package result. Either retain
   the original heterogeneous package AST in a closed owner enum, requiring the
   direct pipeline APIs to accept and preserve `Arc` owners, or declare the
   generated `Arc<CemTreeAstStream>` to be the result owner while source
   provenance separately identifies the input package AST.
2. Choose the graph representation. Either add a first-class
   `TransformArtifactBody` variant for materialized CEMT trees or keep a closed
   `TransformNativeArtifact` extension representation. The former makes graph
   and secondary-input dispatch exhaustive; the latter avoids widening the
   shared body enum but requires checked downcasts at every typed consumer.

The explicit-JSON DOM compatibility branch also needs a disposition after
those choices: isolate it as a compatibility parser edge that immediately
produces the chosen typed artifact, or remove it from production and retain an
equivalent test-only fixture path.

## Implementation After Decision

Add red tests for one package-native formatter, its colorizer and writer, a
graph stage plus ordered join, and a secondary-input edge. The tests must prove
owner `Arc` identity, typed stage validation, source-map/output-span retention,
and absence of `Value` classification. Then migrate every producer in one
atomic slice, remove `CemtOutputArtifact`,
`transform_template_output_cemt_subject`, `CemtEvaluator(Value)`, and
`CemtRuntime(Value)`, and add source audits before running the full verification
matrix.
