# JSON Route Serde Boundary Audit

This audit classifies production `serde_json::Value`, `serde_json::from_*`, and
`serde_json::to_*` use that can participate in JSON input or output routes. It
does not classify test-only parity/assertion code or `Value` used solely as a
diagnostic-details/public-response model.

## Allowed boundaries

| Route | Classification | Constraint |
| --- | --- | --- |
| `run_config::parse_run_config` | Explicit JSON configuration ingress into the typed `RunConfig` contract | Parsing occurs once at the configuration API edge; `Value` is not handed to transform stages. |
| `validation::{cem_ast_projection,cem_dom_projection,cem_events_projection}` JSON validators | Explicit registered JSON projection ingress | These validators consume the encoded projection at its validation edge and do not feed a generic JSON transform data plane. |
| `validation::json` string and number lexeme helpers | Lossless JSON parser implementation | `from_str` is limited to decoding one string token or validating one exact number lexeme while the owning `JsonValueAst` retains the lexeme and range. |
| `real::transform_artifact_export_primary` and `conversion_output_boundary_value` | Explicit JSON/public response export | An encoded JSON result is decoded only where the public `Value` response contract requires it. No later runtime stage consumes that projection. |
| WASM/API response, report, trace, cache, and public/debug projection helpers | Public, observability, or storage boundary | Serialization is the declared boundary representation, not an inter-layer AST handoff. |
| Import-map parse/rewrite/pretty-print in `real::apply_importmap_rewrite` | Explicit embedded JSON ingress and export | This is boundary-owned today, but remains scheduled for lossless AST editing so duplicate/order diagnostics survive the HTML JSON island. |

## Removed

`real::load_root_module_map` previously used
`serde_json::from_slice::<Value>` and traversed `serde_json::Map`. It now parses
once with `json_document_ast_from_source_bytes` and traverses ordered
`JsonValueAst` members directly. Duplicate-member diagnostics, declaration
order, source ranges, and last-declaration alias semantics survive without a
`Value`, serializer, DTO, or re-parser handoff.

The transform-template render binding plane now borrows the lifecycle-owned
`JsonValueAst` root through `CemtEvaluatorValue::Json`. Primary, named
secondary, artifact-id, and let bindings retain ordered members, duplicate
names, exact string/number lexemes, ranges, source maps, and AST identity.
Typed CEM-tree inputs use their native evaluator view on the same route. The
data-input `explicit_json_value`/`explicit_json_bytes` accessors and the legacy
production encode evaluator are gone; expression-created scalars and
collections remain typed evaluator values.

`conversion::DomProjectionParityCemtAdapter::render` now accepts only a typed
`CemTreeAstStream`. Its JSON compatibility ingress and JSON-to-tree fixture
helpers were deleted, and its formatter path retains the native stream owner.

The transform-template encoder boundary is now typed end to end.
`TransformTemplateEncodeBindingRequest.subject` contains only
`TransformTemplateEncodeSubjectMetadata`: the typed evaluator kind, native
representation id, and inferred semantic type candidates. The registered host
encoder trait accepts the borrowed `CemtEvaluatorValue` directly, built-in
encoders traverse typed scalar/sequence/record or lossless `JsonValueAst`
views, and `TransformTemplateEvaluatedEncodeExpression` retains only subject
metadata. The `execute_typed` compatibility projection and decoded evaluated
subject snapshot were deleted. Coverage proves duplicate members, exact
number/string lexemes, ranges, source maps, and native CEM-tree owner identity
reach the selected encoder unchanged.

## Remaining prohibited internal handoffs

1. Compile-request parameters still arrive as `BTreeMap<String, Value>` and the
   normalized parameter values are not retained by
   `TransformTemplateCompiledArtifact`, so render cannot add them to the typed
   binding scope. Their owner/lifetime contract must be selected before adding
   them; introducing another JSON-shaped DTO is not permitted.
2. Collection joins retain typed child artifacts, but `collect`, `groupBy`,
   `matchBy`, and `zip` do not yet define the evaluator shape and identity rules
   needed to expose those children as transform-template bindings. Do not infer
   those semantics from a JSON projection.

The next work is an explicit design point: choose parameter ownership first,
then define collection binding shapes. No implementation should begin by
projecting either contract through JSON.
