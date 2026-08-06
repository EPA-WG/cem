//! Source audit for the transform data plane's explicit JSON boundaries.
//!
//! These regions are the active load, routing, adapter, encoder, and export
//! handoffs. Any new `serde_json` encode/decode operation in them must be
//! registered here with a lifecycle/export classification or the audit fails.

use std::collections::BTreeMap;

const REAL_SOURCE: &str = include_str!("../src/real.rs");
const ARTIFACT_SOURCE: &str = include_str!("../src/transform_artifact.rs");
const PROJECTION_SOURCE: &str = include_str!("../src/projection.rs");
const TEMPLATE_SOURCE: &str = include_str!("../src/transform_template.rs");
const CEM_QL_ADAPTER_SOURCE: &str = include_str!("../../cem_ml_transform_cem_ql/src/lib.rs");

#[derive(Debug)]
struct JsonBoundaryRegion {
    id: &'static str,
    class: &'static str,
    source: &'static str,
    start: &'static str,
    end: &'static str,
    expected_operations: &'static [(&'static str, usize)],
}

const NO_JSON_OPERATIONS: &[(&str, usize)] = &[];
const FROM_SLICE_ONCE: &[(&str, usize)] = &[("from_slice", 1)];
const FROM_STR_ONCE: &[(&str, usize)] = &[("from_str", 1)];
const TO_STRING_ONCE: &[(&str, usize)] = &[("to_string", 1)];
const TO_STRING_TWICE: &[(&str, usize)] = &[("to_string", 2)];
const TO_STRING_PRETTY_TWICE_AND_FROM_STR_ONCE: &[(&str, usize)] =
    &[("from_str", 1), ("to_string_pretty", 2)];
const TO_VALUE_ONCE: &[(&str, usize)] = &[("to_value", 1)];
const TO_VALUE_TWICE: &[(&str, usize)] = &[("to_value", 2)];
const TO_VALUE_THREE_TIMES: &[(&str, usize)] = &[("to_value", 3)];
const TO_VEC_ONCE: &[(&str, usize)] = &[("to_vec", 1)];
const TO_VEC_THREE_TIMES: &[(&str, usize)] = &[("to_vec", 3)];

const JSON_BOUNDARY_ALLOWLIST: &[JsonBoundaryRegion] = &[
    JsonBoundaryRegion {
        id: "lifecycle-load-native-owner",
        class: "serializer-free-native-route",
        source: REAL_SOURCE,
        start: "fn load_transform_data_artifact",
        end: "fn collect_transform_graph_join",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "graph-join-native-collection",
        class: "serializer-free-native-route",
        source: REAL_SOURCE,
        start: "fn collect_transform_graph_join(",
        end: "fn collect_transform_graph_join_metadata",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "cemt-native-binding",
        class: "serializer-free-native-route",
        source: REAL_SOURCE,
        start: "fn transform_template_ast_binding",
        end: "fn transform_template_render_insertion_context",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "cemql-native-ingress",
        class: "serializer-free-native-route",
        source: CEM_QL_ADAPTER_SOURCE,
        start: "fn artifact_query_stream",
        end: "fn template_data_from_artifacts",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "cemql-compile-cache-key",
        class: "serializer-free-native-route",
        source: CEM_QL_ADAPTER_SOURCE,
        start: "impl TransformTemplateAdapter for CemQlExpressionTransformTemplateAdapter",
        end: "impl TransformTemplateAdapter for XsltParityTransformTemplateAdapter",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "collection-cemt-view",
        class: "serializer-free-native-route",
        source: ARTIFACT_SOURCE,
        start: "pub fn cemt_evaluator_view",
        end: "pub enum TransformArtifactBody",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "collection-record-sequence-views",
        class: "serializer-free-native-route",
        source: ARTIFACT_SOURCE,
        start: "impl CemtEvaluatorRecordView for TransformArtifactCollection",
        end: "pub enum TransformEncoding",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "markdown-writer-token-handoff",
        class: "serializer-free-native-route",
        source: REAL_SOURCE,
        start: "fn markdown_generated_html_output",
        end: "fn markdown_html_source_writer_tokens",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "explicit-transform-export-response",
        class: "public-export-boundary",
        source: REAL_SOURCE,
        start: "fn transform_artifact_export_primary",
        end: "fn transform_graph_target_is_css",
        expected_operations: FROM_SLICE_ONCE,
    },
    JsonBoundaryRegion {
        id: "template-output-public-projection",
        class: "public-export-boundary",
        source: REAL_SOURCE,
        start: "fn convert_primary_from_template_output",
        end: "fn convert_primary_to_cem_with_cemt_pipeline",
        expected_operations: TO_VALUE_TWICE,
    },
    JsonBoundaryRegion {
        id: "cem-output-public-projection",
        class: "public-export-boundary",
        source: REAL_SOURCE,
        start: "fn convert_primary_to_cem_with_cemt_pipeline",
        end: "fn convert_primary_to_markup_with_cemt_pipeline",
        expected_operations: TO_VALUE_TWICE,
    },
    JsonBoundaryRegion {
        id: "markup-output-public-projection",
        class: "public-export-boundary",
        source: REAL_SOURCE,
        start: "fn convert_primary_to_markup_with_cemt_pipeline",
        end: "fn cem_tree_nodes_for_markup_output",
        expected_operations: TO_VALUE_TWICE,
    },
    JsonBoundaryRegion {
        id: "html-importmap-json-document",
        class: "explicit-json-lifecycle",
        source: REAL_SOURCE,
        start: "fn apply_importmap_rewrite",
        end: "fn select_transform_template_adapter",
        expected_operations: TO_STRING_PRETTY_TWICE_AND_FROM_STR_ONCE,
    },
    JsonBoundaryRegion {
        id: "typed-template-json-parameter",
        class: "explicit-json-lifecycle",
        source: REAL_SOURCE,
        start: "fn coerce_transform_template_param_value",
        end: "fn validate_transform_template_module_contract",
        expected_operations: FROM_STR_ONCE,
    },
    JsonBoundaryRegion {
        id: "data-artifact-explicit-json-constructor",
        class: "explicit-json-lifecycle",
        source: ARTIFACT_SOURCE,
        start: "    pub fn explicit_json(\n        artifact_id:",
        end: "    pub fn encoded_text",
        expected_operations: TO_VEC_ONCE,
    },
    JsonBoundaryRegion {
        id: "lossless-json-public-projection",
        class: "public-export-boundary",
        source: ARTIFACT_SOURCE,
        start: "fn json_ast_value_to_public_json",
        end: "impl<'a> CemtEvaluatorRecord<'a>",
        expected_operations: FROM_STR_ONCE,
    },
    JsonBoundaryRegion {
        id: "evaluator-public-projection",
        class: "public-export-boundary",
        source: ARTIFACT_SOURCE,
        start: "    pub fn to_public_json(&self) -> Result<serde_json::Value, String> {\n        match self {",
        end: "    fn record_to_public_json",
        expected_operations: TO_VALUE_THREE_TIMES,
    },
    JsonBoundaryRegion {
        id: "json-output-encoder",
        class: "registered-exporter-boundary",
        source: TEMPLATE_SOURCE,
        start: "pub fn transform_template_encode_json_value",
        end: "pub fn transform_template_format_json_value",
        expected_operations: TO_STRING_ONCE,
    },
    JsonBoundaryRegion {
        id: "typed-json-output-encoder",
        class: "registered-exporter-boundary",
        source: TEMPLATE_SOURCE,
        start: "fn transform_template_format_typed_json_value",
        end: "fn transform_template_join_rendered_json_values",
        expected_operations: TO_STRING_TWICE,
    },
    JsonBoundaryRegion {
        id: "template-output-explicit-json-constructor",
        class: "explicit-json-lifecycle",
        source: TEMPLATE_SOURCE,
        start: "    pub fn explicit_json(\n        uri:",
        end: "    pub fn with_metadata",
        expected_operations: TO_VEC_ONCE,
    },
    JsonBoundaryRegion {
        id: "template-output-explicit-json-reader",
        class: "public-export-boundary",
        source: TEMPLATE_SOURCE,
        start: "    pub fn explicit_json_value",
        end: "    pub fn cemt_subject",
        expected_operations: FROM_SLICE_ONCE,
    },
    JsonBoundaryRegion {
        id: "typed-template-json-literal",
        class: "explicit-json-lifecycle",
        source: TEMPLATE_SOURCE,
        start: "fn parse_template_typed_literal_value",
        end: "fn template_element_name",
        expected_operations: FROM_STR_ONCE,
    },
    JsonBoundaryRegion {
        id: "cemql-result-json-exporter",
        class: "registered-exporter-boundary",
        source: CEM_QL_ADAPTER_SOURCE,
        start: "impl TransformArtifactExporter for CemQlJsonResultExporter",
        end: "struct SchemaBehaviorCandidate",
        expected_operations: TO_VEC_ONCE,
    },
    JsonBoundaryRegion {
        id: "native-projection-json-exporters",
        class: "registered-exporter-boundary",
        source: ARTIFACT_SOURCE,
        start: "struct DomProjectionJsonExporter;",
        end: "impl fmt::Debug for TransformArtifactExporterRegistry",
        expected_operations: TO_VEC_THREE_TIMES,
    },
    JsonBoundaryRegion {
        id: "dom-projection-borrowed-serializer",
        class: "serializer-free-native-route",
        source: PROJECTION_SOURCE,
        start: "pub struct DomJsonProjectionRef",
        end: "pub fn dom_json",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "event-stream-borrowed-serializer",
        class: "serializer-free-native-route",
        source: PROJECTION_SOURCE,
        start: "pub struct NormalizedEventStream",
        end: "pub fn events_json_as",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "module-cache-native-policy-stamp",
        class: "serializer-free-native-route",
        source: TEMPLATE_SOURCE,
        start: "impl TransformTemplateModuleCacheKey",
        end: "pub struct TransformTemplateCompiledArtifact",
        expected_operations: NO_JSON_OPERATIONS,
    },
    JsonBoundaryRegion {
        id: "cemt-provenance-public-field",
        class: "public-export-boundary",
        source: ARTIFACT_SOURCE,
        start: "fn cemt_public_insert_provenance",
        end: "impl TransformNativeArtifact for CemtTreeArtifact",
        expected_operations: TO_VALUE_ONCE,
    },
];

fn region_source(region: &JsonBoundaryRegion) -> &'static str {
    region
        .source
        .split_once(region.start)
        .unwrap_or_else(|| panic!("{} start marker `{}`", region.id, region.start))
        .1
        .split_once(region.end)
        .unwrap_or_else(|| panic!("{} end marker `{}`", region.id, region.end))
        .0
}

fn serde_json_operations(source: &str) -> BTreeMap<String, usize> {
    const AUDITED: &[&str] = &[
        "from_reader",
        "from_slice",
        "from_str",
        "from_value",
        "to_string",
        "to_string_pretty",
        "to_value",
        "to_vec",
        "to_vec_pretty",
        "to_writer",
        "to_writer_pretty",
    ];

    let mut operations = BTreeMap::new();
    let mut remaining = source;
    while let Some((_, tail)) = remaining.split_once("serde_json::") {
        let operation = tail
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();
        let operation_len = operation.len();
        if AUDITED.contains(&operation.as_str()) {
            *operations.entry(operation).or_insert(0) += 1;
        }
        remaining = &tail[operation_len..];
    }
    operations
}

#[test]
fn transform_json_boundary_allowlist_rejects_unregistered_serialization() {
    let mut ids = std::collections::BTreeSet::new();
    for region in JSON_BOUNDARY_ALLOWLIST {
        assert!(
            ids.insert(region.id),
            "duplicate boundary id `{}`",
            region.id
        );
        assert!(
            matches!(
                region.class,
                "serializer-free-native-route"
                    | "explicit-json-lifecycle"
                    | "registered-exporter-boundary"
                    | "public-export-boundary"
            ),
            "{} has unsupported class `{}`",
            region.id,
            region.class
        );
        let expected = region
            .expected_operations
            .iter()
            .map(|(operation, count)| ((*operation).to_owned(), *count))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            serde_json_operations(region_source(region)),
            expected,
            "JSON boundary `{}` ({}) changed; register an explicit lifecycle/export operation or remove the serializer handoff",
            region.id,
            region.class
        );
    }
}

#[test]
fn collection_semantic_gates_remain_registered() {
    let cli_source = include_str!("../../cem_ml_cli/src/dispatch.rs");
    for required in [
        "transform_config_collect_join_feeds_single_transform",
        "transform_config_group_by_join_feeds_grouped_transforms",
        "transform_config_match_by_join_feeds_keyed_transforms",
        "transform_config_zip_join_feeds_positional_transforms",
        "transform_config_zip_join_rejects_count_mismatch",
    ] {
        assert!(
            cli_source.contains(required),
            "collection behavior gate `{required}` must remain registered"
        );
    }
}
