use crate::diagnostics::{Diagnostic, Severity};
use crate::events::cem::CemEventNormalizer;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::resolver::{has_uri_scheme, is_windows_drive_path, parse_local_file_uri};
use crate::schema::registry::content_type_essence;
use crate::source::{BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub(crate) const SCHEMA_PACKAGE_SOURCE_CONSISTENCY_CONSTRAINT_DIAGNOSTICS: &[(&str, &str)] = &[
    (
        "cem.schema_package.schema_source_unreadable",
        "schema-source-metadata-consistency",
    ),
    (
        "cem.schema_package.schema_source_invalid",
        "schema-source-metadata-consistency",
    ),
    (
        "cem.schema_package.schema_uri_mismatch",
        "schema-source-metadata-consistency",
    ),
    (
        "cem.schema_package.schema_content_type_mismatch",
        "schema-source-metadata-consistency",
    ),
    (
        "cem.schema_package.schema_namespace_mismatch",
        "schema-source-metadata-consistency",
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

    let schema_path = match schema_source_path(manifest_path, schema_source) {
        Ok(path) => path,
        Err(diagnostic) => return vec![diagnostic],
    };
    let schema_source_bytes = match std::fs::read(&schema_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            return vec![schema_package_consistency_diagnostic(
                "cem.schema_package.schema_source_unreadable",
                format!(
                    "schema package source `{}` could not be read: {error}",
                    schema_path.display()
                ),
            )];
        }
    };
    let schema_source_text = match String::from_utf8(schema_source_bytes) {
        Ok(text) => text,
        Err(error) => {
            return vec![schema_package_consistency_diagnostic(
                "cem.schema_package.schema_source_unreadable",
                format!(
                    "schema package source `{}` is not UTF-8 CEM-ML: {error}",
                    schema_path.display()
                ),
            )];
        }
    };
    let schema_document = parse_cem_document(&schema_source_text);
    let Some(schema_id) = first_cem_element_id_by_local_name(&schema_document, "schema") else {
        return vec![schema_package_consistency_diagnostic(
            "cem.schema_package.schema_source_invalid",
            format!(
                "schema package source `{}` does not contain a `schema` root",
                schema_path.display()
            ),
        )];
    };
    let schema_attrs = cem_collect_attrs(&schema_document, schema_id);
    if let Some(schema_namespace) = cem_optional_attr(&schema_attrs, "namespace") {
        if schema_namespace != manifest_schema_uri {
            diagnostics.push(schema_package_consistency_diagnostic(
                "cem.schema_package.schema_uri_mismatch",
                format!(
                    "package schema URI `{manifest_schema_uri}` does not match schema namespace `{schema_namespace}` in `{}`",
                    schema_path.display()
                ),
            ));
        }
    }

    let package_content_types = package_content_type_claims(manifest, package_id);
    let schema_content_types = schema_content_type_claims(&schema_document, schema_id);
    if !schema_content_types.is_empty() && package_content_types != schema_content_types {
        diagnostics.push(schema_package_consistency_diagnostic(
            "cem.schema_package.schema_content_type_mismatch",
            format!(
                "package content types `{}` do not match schema source content types `{}`",
                format_claim_set(&package_content_types),
                format_claim_set(&schema_content_types)
            ),
        ));
    }

    let package_namespace_uris = package_namespace_uri_claims(manifest, package_id);
    let schema_namespace_uris = schema_namespace_uri_claims(&schema_document, schema_id);
    if !schema_namespace_uris.is_empty() {
        for namespace_uri in package_namespace_uris {
            if !schema_namespace_uris.contains(&namespace_uri) {
                diagnostics.push(schema_package_consistency_diagnostic(
                    "cem.schema_package.schema_namespace_mismatch",
                    format!(
                        "package namespace URI `{namespace_uri}` is not declared by schema source `{}`",
                        schema_path.display()
                    ),
                ));
            }
        }
    }

    diagnostics
}

fn schema_source_path(manifest_path: &Path, schema_source: &str) -> Result<PathBuf, Diagnostic> {
    let schema_source = schema_source.trim();
    if let Some(parsed) = parse_local_file_uri(schema_source) {
        return parsed.map_err(|error| {
            schema_package_consistency_diagnostic(
                "cem.schema_package.schema_source_unreadable",
                format!("schema package source URI `{schema_source}` is invalid: {error}"),
            )
        });
    }
    if has_uri_scheme(schema_source) && !is_windows_drive_path(schema_source) {
        return Err(schema_package_consistency_diagnostic(
            "cem.schema_package.schema_source_unreadable",
            format!(
                "schema package source `{schema_source}` is not a local path or local file URI"
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

fn package_namespace_uri_claims(document: &CemDocument, package_id: AstNodeId) -> BTreeSet<String> {
    cem_child_element_ids_by_local_name(document, package_id, "namespace")
        .into_iter()
        .filter_map(|node_id| {
            let attrs = cem_collect_attrs(document, node_id);
            cem_optional_attr(&attrs, "uri").map(str::to_owned)
        })
        .collect()
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

fn schema_package_consistency_diagnostic(
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity: Severity::Error,
        message: message.into(),
        details: None,
        ..Diagnostic::default()
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
