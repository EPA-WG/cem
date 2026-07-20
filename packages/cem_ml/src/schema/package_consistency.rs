use crate::diagnostics::{Diagnostic, Severity};
use crate::events::cem::CemEventNormalizer;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::resolver::{has_uri_scheme, is_windows_drive_path, parse_local_file_uri};
use crate::schema::document_model::{
    compare_reference_declarative_operands, evaluate_reference_declarative_operands,
    load_builtin_document_model_for_identity, project_reference_declarative_diagnostic,
    select_reference_candidates, ReferenceCandidateSelection, ReferenceCapabilitySet,
    ReferenceDeclarativeComparisonEvaluation, ReferenceResolutionConstraint,
    ReferenceSourceBindings, ReferenceSourceValue, ReferenceSourceValueKind,
};
use crate::schema::reference_resolution::{
    compare_references, normalize_namespace_uri, normalize_namespace_uri_set, normalize_schema_uri,
    ReferenceComparisonInput, ReferenceComparisonOperator, ReferenceOperand, ReferenceOperandRole,
    ReferenceStatePolicy,
};
use crate::schema::registry::{content_type_essence, CEM_SCHEMA_PACKAGE_URI};
use crate::source::{BytesSource, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack};
use crate::tokenizer::cem::CemTokenizer;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[allow(dead_code)]
pub(crate) const SCHEMA_PACKAGE_SOURCE_CONSISTENCY_CONSTRAINT_DIAGNOSTICS: &[(&str, &str)] = &[
    (
        "cem.schema_package.schema_source_unreadable",
        "schema-source-readable",
    ),
    (
        "cem.schema_package.schema_source_invalid",
        "schema-source-valid",
    ),
    (
        "cem.schema_package.schema_uri_mismatch",
        "schema-uri-consistency",
    ),
    (
        "cem.schema_package.schema_content_type_mismatch",
        "schema-content-type-consistency",
    ),
    (
        "cem.schema_package.schema_namespace_mismatch",
        "schema-namespace-consistency",
    ),
];

pub fn validate_schema_package_source_consistency(
    manifest_path: &Path,
    manifest: &CemDocument,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let Some(package_id) = first_cem_element_id_by_local_name(manifest, "package") else {
        return diagnostics;
    };
    let schema_ids = cem_child_element_ids_by_local_name(manifest, package_id, "schema");
    let Some(schema_manifest_id) = schema_ids.first().copied() else {
        return diagnostics;
    };
    let schema_manifest_attrs = cem_collect_attrs(manifest, schema_manifest_id);
    let Some(manifest_schema_uri) =
        cem_optional_attr(&schema_manifest_attrs, "uri").map(str::to_owned)
    else {
        return diagnostics;
    };
    let Some(schema_source) = cem_optional_attr(&schema_manifest_attrs, "source") else {
        return diagnostics;
    };

    let schema_path =
        match schema_source_path(manifest_path, manifest, schema_manifest_id, schema_source) {
            Ok(path) => path,
            Err(diagnostic) => return vec![diagnostic],
        };
    let schema_source_bytes = match std::fs::read(&schema_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return vec![schema_package_consistency_diagnostic_at(
                manifest,
                schema_manifest_id,
                "cem.schema_package.schema_source_unreadable",
                format!(
                    "schema package source `{}` could not be read: {error}",
                    schema_path.display()
                ),
                schema_source_readable_details(
                    manifest,
                    schema_manifest_id,
                    &schema_path.display().to_string(),
                    error.to_string(),
                ),
            )];
        }
    };
    let schema_source_text = match String::from_utf8(schema_source_bytes) {
        Ok(text) => text,
        Err(error) => {
            return vec![schema_package_consistency_diagnostic_at(
                manifest,
                schema_manifest_id,
                "cem.schema_package.schema_source_unreadable",
                format!(
                    "schema package source `{}` is not UTF-8 CEM-ML: {error}",
                    schema_path.display()
                ),
                schema_source_readable_details(
                    manifest,
                    schema_manifest_id,
                    &schema_path.display().to_string(),
                    error.to_string(),
                ),
            )];
        }
    };
    let schema_document = parse_cem_document(&schema_source_text);
    let Some(schema_id) = first_cem_element_id_by_local_name(&schema_document, "schema") else {
        return vec![schema_package_consistency_diagnostic_at(
            manifest,
            schema_manifest_id,
            "cem.schema_package.schema_source_invalid",
            format!(
                "schema package source `{}` does not contain a `schema` root",
                schema_path.display()
            ),
            schema_source_valid_details(
                manifest,
                schema_manifest_id,
                &schema_path.display().to_string(),
            ),
        )];
    };
    let schema_attrs = cem_collect_attrs(&schema_document, schema_id);
    if let Some(schema_namespace) = cem_optional_attr(&schema_attrs, "namespace") {
        let comparison =
            compare_schema_uri_consistency(manifest_schema_uri.as_str(), schema_namespace);
        if !comparison.passed {
            diagnostics.push(schema_package_consistency_diagnostic_at(
                manifest,
                schema_manifest_id,
                "cem.schema_package.schema_uri_mismatch",
                format!(
                    "package schema URI `{manifest_schema_uri}` does not match schema namespace `{schema_namespace}` in `{}`",
                    schema_path.display()
                ),
                schema_reference_resolution_details(
                    manifest,
                    schema_manifest_id,
                    "schema-uri-consistency",
                    "schema",
                    "schema",
                    ["uri"],
                    serde_json::json!({ "uri": manifest_schema_uri }),
                    serde_json::json!({ "uri": schema_namespace }),
                    None,
                ),
            ));
        }
    }

    let package_content_types = package_content_type_claims(manifest, package_id);
    let schema_content_types = schema_content_type_claims(&schema_document, schema_id);
    let content_type_evaluation =
        evaluate_schema_content_type_consistency(manifest, package_id, &schema_document, schema_id);
    let content_type_consistency_passed = content_type_evaluation
        .as_ref()
        .map(schema_content_type_consistency_passed)
        .unwrap_or_else(|| package_content_types == schema_content_types);
    if !schema_content_types.is_empty() && !content_type_consistency_passed {
        let content_type_node_id =
            cem_child_element_ids_by_local_name(manifest, package_id, "content-type")
                .first()
                .copied()
                .unwrap_or(package_id);
        diagnostics.push(schema_package_consistency_diagnostic_at(
            manifest,
            content_type_node_id,
            "cem.schema_package.schema_content_type_mismatch",
            format!(
                "package content types `{}` do not match schema source content types `{}`",
                format_claim_set(&package_content_types),
                format_claim_set(&schema_content_types)
            ),
            schema_reference_resolution_details(
                manifest,
                content_type_node_id,
                "schema-content-type-consistency",
                "content-type",
                "content-type",
                ["value"],
                serde_json::json!({ "content-type": package_content_types }),
                serde_json::json!({ "content-type": schema_content_types }),
                content_type_evaluation.as_ref(),
            ),
        ));
    }

    let schema_namespace_uris = schema_namespace_uri_claims(&schema_document, schema_id);
    if !schema_namespace_uris.is_empty() {
        for namespace_id in cem_child_element_ids_by_local_name(manifest, package_id, "namespace") {
            let namespace_attrs = cem_collect_attrs(manifest, namespace_id);
            let Some(namespace_uri) = cem_optional_attr(&namespace_attrs, "uri") else {
                continue;
            };
            let namespace_comparison =
                compare_schema_namespace_consistency(namespace_uri, &schema_namespace_uris);
            if !namespace_comparison.passed {
                diagnostics.push(schema_package_consistency_diagnostic_at(
                    manifest,
                    namespace_id,
                    "cem.schema_package.schema_namespace_mismatch",
                    format!(
                        "package namespace URI `{namespace_uri}` is not declared by schema source `{}`",
                        schema_path.display()
                    ),
                    schema_reference_resolution_details(
                        manifest,
                        namespace_id,
                        "schema-namespace-consistency",
                        "namespace",
                        "namespace",
                        ["uri"],
                        serde_json::json!({ "uri": namespace_uri }),
                        serde_json::json!({ "uri": schema_namespace_uris }),
                        None,
                    ),
                ));
            }
        }
    }

    diagnostics
}

fn compare_schema_uri_consistency(
    manifest_schema_uri: &str,
    schema_namespace: &str,
) -> crate::schema::reference_resolution::ReferenceComparisonResult {
    compare_references(ReferenceComparisonInput {
        operator: ReferenceComparisonOperator::Equals,
        actual: ReferenceOperand::new(
            ReferenceOperandRole::Actual,
            "uri",
            normalize_schema_uri("uri", manifest_schema_uri, Some(manifest_schema_uri)),
        ),
        expected: Some(ReferenceOperand::new(
            ReferenceOperandRole::Expected,
            "uri",
            normalize_schema_uri("uri", schema_namespace, Some(schema_namespace)),
        )),
        forbidden: None,
        state_policy: ReferenceStatePolicy::RequiredValid,
    })
}

fn compare_schema_namespace_consistency(
    namespace_uri: &str,
    schema_namespace_uris: &BTreeSet<String>,
) -> crate::schema::reference_resolution::ReferenceComparisonResult {
    compare_references(ReferenceComparisonInput {
        operator: ReferenceComparisonOperator::MemberOf,
        actual: ReferenceOperand::new(
            ReferenceOperandRole::Actual,
            "uri",
            normalize_namespace_uri("uri", namespace_uri),
        ),
        expected: Some(ReferenceOperand::new(
            ReferenceOperandRole::Expected,
            "uri",
            normalize_namespace_uri_set("uri", schema_namespace_uris),
        )),
        forbidden: None,
        state_policy: ReferenceStatePolicy::RequiredValid,
    })
}

struct SchemaContentTypeConsistencyEvaluation {
    constraint: ReferenceResolutionConstraint,
    candidates: Option<ReferenceCandidateSelection>,
    comparison: ReferenceDeclarativeComparisonEvaluation,
}

fn schema_content_type_consistency_passed(
    evaluation: &SchemaContentTypeConsistencyEvaluation,
) -> bool {
    evaluation
        .comparison
        .result
        .as_ref()
        .is_some_and(|result| result.passed)
}

fn evaluate_schema_content_type_consistency(
    manifest: &CemDocument,
    package_id: AstNodeId,
    schema_document: &CemDocument,
    schema_id: AstNodeId,
) -> Option<SchemaContentTypeConsistencyEvaluation> {
    let constraint =
        schema_package_reference_resolution_constraint("schema-content-type-consistency")?;
    let mut bindings = ReferenceSourceBindings::default();
    bindings.bind_nodes("package", vec![package_id]);
    bindings.bind_field_values(
        "package",
        "content-types",
        package_content_type_claim_source_values(manifest, package_id),
    );
    bindings.bind_field_values(
        "schema-source",
        "content-types",
        schema_content_type_claim_source_values(schema_document, schema_id),
    );

    let candidates = constraint
        .candidates
        .first()
        .map(|declaration| select_reference_candidates(manifest, &[package_id], declaration));
    if let Some(candidates) = candidates.as_ref() {
        bindings.bind_candidate_selection(candidates);
    }

    let operands = evaluate_reference_declarative_operands(
        manifest,
        &[package_id],
        &bindings,
        &constraint.execution,
        &constraint.operands,
        &ReferenceCapabilitySet::default(),
    );
    let comparison = compare_reference_declarative_operands(operands, constraint.compare.as_ref());

    Some(SchemaContentTypeConsistencyEvaluation {
        constraint,
        candidates,
        comparison,
    })
}

fn schema_package_reference_resolution_constraint(
    kind: &str,
) -> Option<ReferenceResolutionConstraint> {
    static REFERENCE_CONSTRAINTS: OnceLock<BTreeMap<String, ReferenceResolutionConstraint>> =
        OnceLock::new();
    REFERENCE_CONSTRAINTS
        .get_or_init(|| {
            load_builtin_document_model_for_identity(Some(CEM_SCHEMA_PACKAGE_URI), None)
                .map(|model| {
                    model
                        .constraints
                        .into_iter()
                        .filter_map(|(kind, constraint)| {
                            constraint
                                .reference_resolution
                                .map(|reference| (kind, reference))
                        })
                        .collect()
                })
                .unwrap_or_default()
        })
        .get(kind)
        .cloned()
}

fn schema_source_path(
    manifest_path: &Path,
    manifest: &CemDocument,
    schema_manifest_id: AstNodeId,
    schema_source: &str,
) -> Result<PathBuf, Diagnostic> {
    let schema_source = schema_source.trim();
    if let Some(parsed) = parse_local_file_uri(schema_source) {
        return parsed.map_err(|error| {
            schema_package_consistency_diagnostic_at(
                manifest,
                schema_manifest_id,
                "cem.schema_package.schema_source_unreadable",
                format!("schema package source URI `{schema_source}` is invalid: {error}"),
                schema_source_readable_details(
                    manifest,
                    schema_manifest_id,
                    schema_source,
                    error.to_string(),
                ),
            )
        });
    }
    if has_uri_scheme(schema_source) && !is_windows_drive_path(schema_source) {
        return Err(schema_package_consistency_diagnostic_at(
            manifest,
            schema_manifest_id,
            "cem.schema_package.schema_source_unreadable",
            format!(
                "schema package source `{schema_source}` is not a local path or local file URI"
            ),
            schema_source_readable_details(
                manifest,
                schema_manifest_id,
                schema_source,
                "non-local source URI".to_owned(),
            ),
        ));
    }

    let source_path = PathBuf::from(schema_source);
    if source_path.is_absolute() {
        return Ok(source_path);
    }
    Ok(manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(source_path))
}

fn package_content_type_claims(document: &CemDocument, package_id: AstNodeId) -> BTreeSet<String> {
    cem_child_element_ids_by_local_name(document, package_id, "content-type")
        .into_iter()
        .filter_map(|node_id| content_type_claim(document, node_id))
        .collect()
}

fn package_content_type_claim_source_values(
    document: &CemDocument,
    package_id: AstNodeId,
) -> Vec<ReferenceSourceValue> {
    cem_child_element_ids_by_local_name(document, package_id, "content-type")
        .into_iter()
        .filter_map(|node_id| content_type_claim_source_value(document, node_id))
        .collect()
}

fn schema_content_type_claims(document: &CemDocument, schema_id: AstNodeId) -> BTreeSet<String> {
    let mut claims = BTreeSet::new();
    for content_types_id in
        cem_child_element_ids_by_local_name(document, schema_id, "content-types")
    {
        for content_type_id in
            cem_child_element_ids_by_local_name(document, content_types_id, "content-type")
        {
            if let Some(claim) = content_type_claim(document, content_type_id) {
                claims.insert(claim);
            }
        }
    }
    claims
}

fn schema_content_type_claim_source_values(
    document: &CemDocument,
    schema_id: AstNodeId,
) -> Vec<ReferenceSourceValue> {
    let mut values = Vec::new();
    for content_types_id in
        cem_child_element_ids_by_local_name(document, schema_id, "content-types")
    {
        for content_type_id in
            cem_child_element_ids_by_local_name(document, content_types_id, "content-type")
        {
            if let Some(value) = content_type_claim_source_value(document, content_type_id) {
                values.push(value);
            }
        }
    }
    values
}

fn content_type_claim(document: &CemDocument, node_id: AstNodeId) -> Option<String> {
    let attrs = cem_collect_attrs(document, node_id);
    let value = cem_optional_attr(&attrs, "value")?;
    let role = if cem_bool_attr(&attrs, "primary") {
        "primary"
    } else {
        "alias"
    };
    Some(format!("{role}:{}", content_type_essence(value)))
}

fn content_type_claim_source_value(
    document: &CemDocument,
    node_id: AstNodeId,
) -> Option<ReferenceSourceValue> {
    Some(ReferenceSourceValue {
        kind: ReferenceSourceValueKind::Field,
        binding: "content-type".to_owned(),
        name: Some("content-type".to_owned()),
        value: Some(content_type_claim(document, node_id)?),
        node_id: Some(node_id),
        source_map: cem_node_source_map(document, node_id).unwrap_or_default(),
    })
}

fn schema_namespace_uri_claims(document: &CemDocument, schema_id: AstNodeId) -> BTreeSet<String> {
    let mut claims = BTreeSet::new();
    for namespaces_id in cem_child_element_ids_by_local_name(document, schema_id, "namespaces") {
        for namespace_id in
            cem_child_element_ids_by_local_name(document, namespaces_id, "namespace")
        {
            let attrs = cem_collect_attrs(document, namespace_id);
            if let Some(uri) = cem_optional_attr(&attrs, "uri") {
                claims.insert(uri.to_owned());
            }
        }
    }
    claims
}

fn schema_package_consistency_diagnostic_at(
    document: &CemDocument,
    node_id: AstNodeId,
    code: &'static str,
    message: impl Into<String>,
    details: serde_json::Value,
) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity: Severity::Error,
        message: message.into(),
        details: Some(details),
        source_map: cem_node_source_map(document, node_id),
        ..Diagnostic::default()
    }
}

fn schema_source_readable_details(
    document: &CemDocument,
    node_id: AstNodeId,
    path: &str,
    error: String,
) -> serde_json::Value {
    serde_json::json!({
        "schemaUri": CEM_SCHEMA_PACKAGE_URI,
        "element": "schema",
        "contract": "schema-source-readable",
        "target": "schema",
        "diagnostic": "cem.schema_package.schema_source_unreadable",
        "behavior": "schema:resource-readable",
        "checkKind": "schema-source-readable",
        "path": path,
        "invalidFields": ["source"],
        "invalidValues": {
            "source": path,
        },
        "error": error,
        "actualValues": cem_collect_attrs(document, node_id),
        "sourceRange": cem_node_source_range_details(document, node_id),
    })
}

fn schema_source_valid_details(
    document: &CemDocument,
    node_id: AstNodeId,
    path: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schemaUri": CEM_SCHEMA_PACKAGE_URI,
        "element": "schema",
        "contract": "schema-source-valid",
        "target": "schema",
        "diagnostic": "cem.schema_package.schema_source_invalid",
        "behavior": "schema:resource-parse",
        "checkKind": "schema-source-valid",
        "path": path,
        "invalidFields": ["source"],
        "invalidValues": {
            "source": path,
        },
        "actualValues": cem_collect_attrs(document, node_id),
        "sourceRange": cem_node_source_range_details(document, node_id),
    })
}

fn schema_reference_resolution_details<const N: usize>(
    document: &CemDocument,
    node_id: AstNodeId,
    constraint: &str,
    element: &str,
    target: &str,
    invalid_fields: [&str; N],
    invalid_values: serde_json::Value,
    expected_values: serde_json::Value,
    reference_evaluation: Option<&SchemaContentTypeConsistencyEvaluation>,
) -> serde_json::Value {
    let details = serde_json::json!({
        "schemaUri": CEM_SCHEMA_PACKAGE_URI,
        "element": element,
        "contract": constraint,
        "target": target,
        "diagnostic": schema_package_source_consistency_diagnostic_code(constraint),
        "behavior": "schema:reference-resolution",
        "checkKind": constraint,
        "invalidFields": invalid_fields.into_iter().collect::<Vec<_>>(),
        "invalidValues": invalid_values,
        "expectedValues": expected_values,
        "actualValues": cem_collect_attrs(document, node_id),
        "sourceRange": cem_node_source_range_details(document, node_id),
    });
    let Some(reference_evaluation) = reference_evaluation else {
        return details;
    };
    let projection = project_reference_declarative_diagnostic(
        &reference_evaluation.comparison,
        reference_evaluation.constraint.projection.as_ref(),
        reference_evaluation.candidates.as_ref(),
    );
    merge_reference_projection_details(details, projection)
}

fn merge_reference_projection_details(
    mut details: serde_json::Value,
    projection: serde_json::Value,
) -> serde_json::Value {
    let (serde_json::Value::Object(details), serde_json::Value::Object(projection)) =
        (&mut details, projection)
    else {
        return details;
    };
    for (key, value) in projection {
        if matches!(key.as_str(), "actualValues" | "invalidFields") && details.contains_key(&key) {
            continue;
        }
        details.insert(key, value);
    }
    serde_json::Value::Object(details.clone())
}

fn schema_package_source_consistency_diagnostic_code(constraint: &str) -> &'static str {
    match constraint {
        "schema-uri-consistency" => "cem.schema_package.schema_uri_mismatch",
        "schema-content-type-consistency" => "cem.schema_package.schema_content_type_mismatch",
        "schema-namespace-consistency" => "cem.schema_package.schema_namespace_mismatch",
        "schema-source-readable" => "cem.schema_package.schema_source_unreadable",
        "schema-source-valid" => "cem.schema_package.schema_source_invalid",
        _ => "cem.schema_package.schema_uri_mismatch",
    }
}

fn cem_node_source_range_details(
    document: &CemDocument,
    node_id: AstNodeId,
) -> Option<serde_json::Value> {
    document
        .get(node_id)
        .and_then(|node| source_stack_for_node(node).current())
        .map(source_frame_range_details)
}

fn cem_node_source_map(document: &CemDocument, node_id: AstNodeId) -> Option<SourceMapStack> {
    document
        .get(node_id)
        .map(|node| source_stack_for_node(node).clone())
}

fn source_stack_for_node(node: &CemAstNode) -> &SourceMapStack {
    match node {
        CemAstNode::Document { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Whitespace { source, .. }
        | CemAstNode::Comment { source, .. }
        | CemAstNode::ProcessingInstruction { source, .. }
        | CemAstNode::Cdata { source, .. }
        | CemAstNode::RawText { source, .. }
        | CemAstNode::Error { source, .. } => source,
    }
}

fn source_frame_range_details(frame: &SourceMapFrame) -> serde_json::Value {
    serde_json::json!({
        "sourceId": frame.source_id.0,
        "span": frame_span_details(&frame.span),
    })
}

fn frame_span_details(span: &FrameSpan) -> serde_json::Value {
    match span {
        FrameSpan::Single(range) => serde_json::json!({
            "kind": "single",
            "start": range.start,
            "len": range.len,
            "end": range.end(),
        }),
        FrameSpan::Multi(ranges) => serde_json::json!({
            "kind": "multi",
            "ranges": ranges
                .iter()
                .map(|range| {
                    serde_json::json!({
                        "start": range.start,
                        "len": range.len,
                        "end": range.end(),
                    })
                })
                .collect::<Vec<_>>(),
        }),
    }
}

fn format_claim_set(claims: &BTreeSet<String>) -> String {
    claims.iter().cloned().collect::<Vec<_>>().join(", ")
}

fn parse_cem_document(input: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
    let tok = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tok);
    CemAstBuilder::new(normalizer).build()
}

fn cem_collect_attrs(document: &CemDocument, node_id: AstNodeId) -> BTreeMap<String, String> {
    let mut attrs = BTreeMap::new();
    let Some(CemAstNode::Element { attributes, .. }) = document.get(node_id) else {
        return attrs;
    };
    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = document.get(*attr_id)
        else {
            continue;
        };
        attrs.insert(
            expanded_name.local_name.clone(),
            value.clone().unwrap_or_default(),
        );
    }
    attrs
}

fn cem_optional_attr<'a>(attrs: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    attrs
        .get(name)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn cem_bool_attr(attrs: &BTreeMap<String, String>, name: &str) -> bool {
    matches!(
        attrs.get(name).map(String::as_str).map(str::trim),
        Some("") | Some("true") | Some("1")
    )
}

fn first_cem_element_id_by_local_name(
    document: &CemDocument,
    local_name: &str,
) -> Option<AstNodeId> {
    document.iter().find_map(|node| {
        let CemAstNode::Element {
            node_id,
            expanded_name,
            ..
        } = node
        else {
            return None;
        };
        (expanded_name.local_name == local_name).then_some(*node_id)
    })
}

fn cem_child_element_ids_by_local_name(
    document: &CemDocument,
    node_id: AstNodeId,
    local_name: &str,
) -> Vec<AstNodeId> {
    let Some(CemAstNode::Element { children, .. }) = document.get(node_id) else {
        return Vec::new();
    };
    children
        .iter()
        .copied()
        .filter(|child_id| {
            matches!(
                document.get(*child_id),
                Some(CemAstNode::Element { expanded_name, .. })
                    if expanded_name.local_name == local_name
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn schema_source_consistency_diagnostics_include_structured_details() {
        let temp_dir = test_dir("metadata");
        fs::create_dir_all(temp_dir.join("schema")).expect("test schema directory");
        fs::write(
            temp_dir.join("schema/source.cem"),
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="demo" @namespace="https://example.test/ns/actual/1" @version="1.0.0" |
    {content-types |
        {content-type @value="application/vnd.example.actual+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="actual" @uri="https://example.test/ns/actual/1"}
    }
}"#,
        )
        .expect("test schema source");
        let manifest = parse_cem_document(
            r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id=demo @version="1.0.0" |
    {schema @uri="https://example.test/ns/manifest/1" @source="schema/source.cem"}
    {content-type @value="application/vnd.example.manifest+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/manifest/1"}
}"#,
        );

        let diagnostics =
            validate_schema_package_source_consistency(&temp_dir.join("package.cem"), &manifest);

        assert_source_consistency_detail(
            &diagnostics,
            "cem.schema_package.schema_uri_mismatch",
            "schema-uri-consistency",
            "schema",
            "schema",
            "schema:reference-resolution",
        );
        assert_source_consistency_detail(
            &diagnostics,
            "cem.schema_package.schema_content_type_mismatch",
            "schema-content-type-consistency",
            "content-type",
            "content-type",
            "schema:reference-resolution",
        );
        let content_type_details = diagnostic_details(
            &diagnostics,
            "cem.schema_package.schema_content_type_mismatch",
        );
        assert_eq!(
            content_type_details["actualValues"],
            serde_json::json!({
                "primary": "true",
                "value": "application/vnd.example.manifest+cem",
            })
        );
        assert_eq!(
            content_type_details["invalidFields"],
            serde_json::json!(["value"])
        );
        assert_eq!(
            content_type_details["invalidValues"],
            serde_json::json!({
                "content-type": ["primary:application/vnd.example.manifest+cem"],
            })
        );
        assert_eq!(
            content_type_details["expectedValues"],
            serde_json::json!({
                "content-type": ["primary:application/vnd.example.actual+cem"],
            })
        );
        assert!(content_type_details.get("comparison").is_none());
        assert!(content_type_details.get("sourceRanges").is_none());
        assert_source_consistency_detail(
            &diagnostics,
            "cem.schema_package.schema_namespace_mismatch",
            "schema-namespace-consistency",
            "namespace",
            "namespace",
            "schema:reference-resolution",
        );
        fs::remove_dir_all(temp_dir).ok();
    }

    #[test]
    fn schema_source_read_failures_include_resource_behavior_details() {
        let temp_dir = test_dir("unreadable");
        fs::create_dir_all(&temp_dir).expect("test schema package directory");
        let manifest = parse_cem_document(
            r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id=demo @version="1.0.0" |
    {schema @uri="https://example.test/ns/demo/1" @source="schema/missing.cem"}
    {content-type @value="application/vnd.example.demo+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/demo/1"}
}"#,
        );

        let diagnostics =
            validate_schema_package_source_consistency(&temp_dir.join("package.cem"), &manifest);

        assert_source_consistency_detail(
            &diagnostics,
            "cem.schema_package.schema_source_unreadable",
            "schema-source-readable",
            "schema",
            "schema",
            "schema:resource-readable",
        );
        fs::remove_dir_all(temp_dir).ok();
    }

    fn diagnostic_details<'a>(
        diagnostics: &'a [Diagnostic],
        code: &str,
    ) -> &'a serde_json::Map<String, serde_json::Value> {
        diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .and_then(|diagnostic| diagnostic.details.as_ref())
            .and_then(serde_json::Value::as_object)
            .unwrap_or_else(|| panic!("expected details for {code}: {diagnostics:?}"))
    }

    fn assert_source_consistency_detail(
        diagnostics: &[Diagnostic],
        code: &str,
        check_kind: &str,
        element: &str,
        target: &str,
        behavior: &str,
    ) {
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("expected diagnostic {code}: {diagnostics:?}"));
        let details = diagnostic
            .details
            .as_ref()
            .unwrap_or_else(|| panic!("{code} must include details"));
        assert_eq!(
            details["schemaUri"],
            serde_json::json!(CEM_SCHEMA_PACKAGE_URI)
        );
        assert_eq!(details["element"], serde_json::json!(element));
        assert_eq!(details["target"], serde_json::json!(target));
        assert_eq!(details["diagnostic"], serde_json::json!(code));
        assert_eq!(details["behavior"], serde_json::json!(behavior));
        assert_eq!(details["checkKind"], serde_json::json!(check_kind));
        assert_eq!(details["contract"], serde_json::json!(check_kind));
        assert!(details["sourceRange"]["span"]["start"].is_u64());
        assert!(details["actualValues"].is_object());
    }

    fn test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "cem-package-consistency-{name}-{}-{unique}",
            std::process::id()
        ))
    }
}
