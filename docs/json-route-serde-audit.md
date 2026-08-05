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

## Removed in this slice

`real::load_root_module_map` previously used
`serde_json::from_slice::<Value>` and traversed `serde_json::Map`. It now parses
once with `json_document_ast_from_source_bytes` and traverses ordered
`JsonValueAst` members directly. Duplicate-member diagnostics, declaration
order, source ranges, and last-declaration alias semantics survive without a
`Value`, serializer, DTO, or re-parser handoff.

## Remaining prohibited internal handoffs

1. `real::transform_template_render_value_bindings` calls
   `TransformTemplateDataArtifact::explicit_json_value` for primary and
   secondary graph inputs, collapsing the lifecycle JSON AST before let/encode
   evaluation.
2. `conversion::DomProjectionParityCemtAdapter::render` retains an explicit
   JSON compatibility ingress after its typed `CemTreeAstStream` path. The
   compatibility branch should be deleted once its remaining callers are
   confirmed typed.
3. `TransformDataArtifact::explicit_json_value` and
   `TransformTemplateOutputArtifact::explicit_json_value` expose generic
   decoded values. The output accessor is allowed only at explicit public
   export; the data-input accessor must disappear with the typed evaluator
   migration.
4. JSON-typed transform-template parameters and let/encode bindings still enter
   the legacy `BTreeMap<String, Value>` evaluator. They need a borrowed value
   contract over `JsonDocumentAst`/`JsonValueAst`, with owned typed scalars only
   for expression-created values.

The next slice should address items 1 and 4 together: routing primary,
secondary, and let-bound JSON through borrowed AST evaluator views without
creating a parallel JSON-shaped DTO.
