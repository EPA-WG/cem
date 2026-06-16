//! CEM-ML transform graph configuration.
//!
//! This module owns the future `cem-ml transform` config syntax boundary. It
//! parses CEM-ML-authored `run` / `import` / `transform` / `export` trees into
//! a graph model and validates graph shape. It does not execute templates.

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::{
    classify_transform_template_identity, FormatIdentity, TransformTemplateKind,
    TRANSFORM_TEMPLATE_UNSUPPORTED_CODE,
};
use crate::events::cem::CemEventNormalizer;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::run_config;
use crate::source::{BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TRANSFORM_CONFIG_SCHEMA_URI: &str = "https://cem.dev/ns/cli/transform-config/1";
pub const TRANSFORM_CONFIG_NAMESPACE_URI: &str = TRANSFORM_CONFIG_SCHEMA_URI;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformConfigElementSchema {
    pub local_name: &'static str,
    pub required_attributes: &'static [&'static str],
    pub optional_attributes: &'static [&'static str],
    pub child_elements: &'static [&'static str],
}

pub const TRANSFORM_CONFIG_SCHEMA_ELEMENTS: &[TransformConfigElementSchema] = &[
    TransformConfigElementSchema {
        local_name: "run",
        required_attributes: &[],
        optional_attributes: &[],
        child_elements: &["import"],
    },
    TransformConfigElementSchema {
        local_name: "import",
        required_attributes: &["src"],
        optional_attributes: &["id", "content-type", "contentType", "schema"],
        child_elements: &["transform", "export"],
    },
    TransformConfigElementSchema {
        local_name: "transform",
        required_attributes: &["src"],
        optional_attributes: &[
            "id",
            "input",
            "with:*",
            "template-content-type",
            "templateContentType",
            "template-schema",
            "templateSchema",
        ],
        child_elements: &["transform", "export"],
    },
    TransformConfigElementSchema {
        local_name: "export",
        required_attributes: &["out"],
        optional_attributes: &["id", "content-type", "contentType", "schema"],
        child_elements: &[],
    },
];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphConfig {
    #[serde(default)]
    pub nodes: Vec<TransformGraphNode>,
    #[serde(default)]
    pub edges: Vec<TransformGraphEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphNode {
    pub id: String,
    pub kind: TransformGraphNodeKind,
    #[serde(default)]
    pub src: Option<String>,
    #[serde(default)]
    pub out: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    #[serde(default)]
    pub template_content_type: Option<String>,
    #[serde(default)]
    pub template_schema: Option<String>,
    #[serde(default)]
    pub template_kind: Option<TransformTemplateKind>,
    #[serde(default)]
    pub input_ref: Option<String>,
    #[serde(default)]
    pub with: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformGraphNodeKind {
    Import,
    Transform,
    Export,
}

impl TransformGraphNodeKind {
    fn as_str(self) -> &'static str {
        match self {
            TransformGraphNodeKind::Import => "import",
            TransformGraphNodeKind::Transform => "transform",
            TransformGraphNodeKind::Export => "export",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformGraphEdge {
    pub from: String,
    pub to: String,
    pub role: TransformGraphEdgeRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransformGraphEdgeRole {
    Parent,
    Input,
    With,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformGraphParseRequest {
    pub bytes: Vec<u8>,
    pub identity: FormatIdentity,
    pub base_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformGraphParseResponse {
    pub graph: TransformGraphConfig,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformGraphConfigError {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for TransformGraphConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TransformGraphConfigError {}

pub fn parse_transform_graph_config(
    request: TransformGraphParseRequest,
) -> Result<TransformGraphParseResponse, TransformGraphConfigError> {
    let content_type = request
        .identity
        .content_type
        .as_deref()
        .map(content_type_essence)
        .unwrap_or_else(|| "text/cem-ml".to_owned());

    if !matches!(
        content_type.as_str(),
        "text/cem-ml" | "application/cem+xml" | "application/cem" | "text/cem"
    ) {
        return Err(transform_config_error(
            "cem.transform_config.unsupported_content_type",
            format!(
                "transform graph config content type `{content_type}` is not supported; use CEM-ML"
            ),
        ));
    }
    validate_config_identity(&request.identity)?;

    let mut tokenizer = CemTokenizer::from_source(BytesSource::new(SourceId(1), request.bytes));
    let mut diagnostics = tokenizer.take_diagnostics();
    let normalizer = CemEventNormalizer::new(tokenizer);
    let document = CemAstBuilder::new(normalizer).build();
    diagnostics.extend(document.diagnostics.clone());

    let mut lowerer = GraphLowerer {
        document: &document,
        graph: TransformGraphConfig::default(),
        diagnostics,
        base_uri: request.base_uri.as_deref(),
        runs_seen: 0,
    };
    lowerer.lower_document();
    lowerer.validate_graph();

    Ok(TransformGraphParseResponse {
        graph: lowerer.graph,
        diagnostics: lowerer.diagnostics,
    })
}

fn validate_config_identity(identity: &FormatIdentity) -> Result<(), TransformGraphConfigError> {
    if let Some(schema) = identity.schema.as_deref().map(str::trim) {
        if !schema.is_empty() && schema != TRANSFORM_CONFIG_SCHEMA_URI {
            return Err(transform_config_error(
                "cem.transform_config.unsupported_schema_identity",
                format!(
                    "transform graph config schema `{schema}` is not supported; expected `{TRANSFORM_CONFIG_SCHEMA_URI}`"
                ),
            ));
        }
    }

    if let Some(default_namespace) = identity.default_namespace.as_deref().map(str::trim) {
        if !default_namespace.is_empty() && default_namespace != TRANSFORM_CONFIG_NAMESPACE_URI {
            return Err(transform_config_error(
                "cem.transform_config.unsupported_schema_identity",
                format!(
                    "transform graph config namespace `{default_namespace}` is not supported; expected `{TRANSFORM_CONFIG_NAMESPACE_URI}`"
                ),
            ));
        }
    }

    Ok(())
}

struct GraphLowerer<'a> {
    document: &'a CemDocument,
    graph: TransformGraphConfig,
    diagnostics: Vec<Diagnostic>,
    base_uri: Option<&'a str>,
    runs_seen: usize,
}

impl GraphLowerer<'_> {
    fn lower_document(&mut self) {
        let Some(CemAstNode::Document { root_children, .. }) = self.document.root() else {
            self.push_diag(
                "cem.transform_config.document_missing",
                "transform graph config has no document root",
            );
            return;
        };

        for child in root_children {
            let Some(element) = element_name(self.document, *child) else {
                continue;
            };
            match element {
                "@doc" => {}
                "run" => {
                    self.runs_seen += 1;
                    self.lower_children(*child, None);
                }
                other => self.push_diag(
                    "cem.transform_config.top_level_unsupported",
                    format!("top-level `{other}` is not valid in transform graph config"),
                ),
            }
        }

        if self.runs_seen == 0 {
            self.push_diag(
                "cem.transform_config.run_missing",
                "transform graph config requires a top-level `run` node",
            );
        } else if self.runs_seen > 1 {
            self.push_diag(
                "cem.transform_config.run_duplicate",
                "transform graph config must contain exactly one top-level `run` node",
            );
        }
    }

    fn lower_children(&mut self, parent_ast_id: AstNodeId, parent_graph_id: Option<String>) {
        let Some(CemAstNode::Element { children, .. }) = self.document.get(parent_ast_id) else {
            return;
        };
        for child in children {
            let Some(name) = element_name(self.document, *child) else {
                continue;
            };
            match name {
                "import" | "transform" | "export" => {
                    self.lower_operation(*child, parent_graph_id.clone());
                }
                other => self.push_diag(
                    "cem.transform_config.child_unsupported",
                    format!("`{other}` is not valid inside transform graph operation nodes"),
                ),
            }
        }
    }

    fn lower_operation(&mut self, ast_id: AstNodeId, parent_graph_id: Option<String>) {
        let Some(name) = element_name(self.document, ast_id) else {
            return;
        };
        let kind = match name {
            "import" => TransformGraphNodeKind::Import,
            "transform" => TransformGraphNodeKind::Transform,
            "export" => TransformGraphNodeKind::Export,
            _ => return,
        };
        let attrs = collect_attrs(self.document, ast_id);
        let id = attr_value(&attrs, "", "id").unwrap_or_else(|| format!("{}:{ast_id}", name));
        let input_ref = attr_value(&attrs, "", "input");
        let mut node = TransformGraphNode {
            id: id.clone(),
            kind,
            src: attr_value(&attrs, "", "src"),
            out: attr_value(&attrs, "", "out"),
            content_type: attr_value(&attrs, "", "content-type")
                .or_else(|| attr_value(&attrs, "", "contentType")),
            schema: attr_value(&attrs, "", "schema"),
            template_content_type: attr_value(&attrs, "", "template-content-type")
                .or_else(|| attr_value(&attrs, "", "templateContentType")),
            template_schema: attr_value(&attrs, "", "template-schema")
                .or_else(|| attr_value(&attrs, "", "templateSchema")),
            template_kind: None,
            input_ref,
            with: with_refs(&attrs),
        };
        if kind == TransformGraphNodeKind::Transform {
            self.classify_template_kind(&mut node);
        }

        if node.input_ref.is_none() {
            if let Some(parent_id) = parent_graph_id {
                self.graph.edges.push(TransformGraphEdge {
                    from: parent_id,
                    to: id.clone(),
                    role: TransformGraphEdgeRole::Parent,
                });
            }
        }

        self.graph.nodes.push(node);

        if kind == TransformGraphNodeKind::Export {
            self.lower_export_children(ast_id);
        } else {
            self.lower_children(ast_id, Some(id));
        }
    }

    fn lower_export_children(&mut self, ast_id: AstNodeId) {
        let Some(CemAstNode::Element { children, .. }) = self.document.get(ast_id) else {
            return;
        };
        for child in children {
            if let Some(name) = element_name(self.document, *child) {
                self.push_diag(
                    "cem.transform_config.export_child_unsupported",
                    format!("export nodes cannot contain `{name}` child operations"),
                );
            }
        }
    }

    fn validate_graph(&mut self) {
        let mut ids = BTreeSet::new();
        let mut known = BTreeSet::new();
        let nodes = self.graph.nodes.clone();
        for node in &nodes {
            if node.id.trim().is_empty() {
                self.push_diag(
                    "cem.transform_config.id_missing",
                    format!("{} node has an empty id", node.kind.as_str()),
                );
            } else if !ids.insert(node.id.clone()) {
                self.push_diag(
                    "cem.transform_config.id_duplicate",
                    format!(
                        "transform graph node id `{}` is declared more than once",
                        node.id
                    ),
                );
            }
            known.insert(node.id.clone());
            self.validate_required_attrs(node);
        }

        for node in &nodes {
            if let Some(input_ref) = node.input_ref.as_deref() {
                self.validate_ref(node, "input", input_ref, &known);
                if known.contains(input_ref) {
                    self.graph.edges.push(TransformGraphEdge {
                        from: input_ref.to_owned(),
                        to: node.id.clone(),
                        role: TransformGraphEdgeRole::Input,
                    });
                }
            }
            for (label, target) in &node.with {
                self.validate_ref(node, &format!("with:{label}"), target, &known);
                if known.contains(target) {
                    self.graph.edges.push(TransformGraphEdge {
                        from: target.clone(),
                        to: node.id.clone(),
                        role: TransformGraphEdgeRole::With,
                    });
                }
            }
        }

        self.validate_outputs(&nodes);
        self.validate_cycles();
    }

    fn validate_required_attrs(&mut self, node: &TransformGraphNode) {
        match node.kind {
            TransformGraphNodeKind::Import => {
                if node.src.as_deref().unwrap_or("").trim().is_empty() {
                    self.push_diag(
                        "cem.transform_config.import_src_missing",
                        format!("import node `{}` requires `@src`", node.id),
                    );
                }
            }
            TransformGraphNodeKind::Transform => {
                if node.src.as_deref().unwrap_or("").trim().is_empty() {
                    self.push_diag(
                        "cem.transform_config.transform_src_missing",
                        format!("transform node `{}` requires template `@src`", node.id),
                    );
                } else if node.template_kind.is_none() {
                    self.push_diag(
                        "cem.transform_config.template_identity_missing",
                        format!(
                            "transform node `{}` requires a supported template identity via `@template-content-type`, `@template-schema`, or `@src` extension",
                            node.id
                        ),
                    );
                }
            }
            TransformGraphNodeKind::Export => {
                if node.out.as_deref().unwrap_or("").trim().is_empty() {
                    self.push_diag(
                        "cem.transform_config.export_out_missing",
                        format!("export node `{}` requires `@out`", node.id),
                    );
                }
            }
        }
    }

    fn validate_ref(
        &mut self,
        node: &TransformGraphNode,
        field: &str,
        target: &str,
        known: &BTreeSet<String>,
    ) {
        if target.trim().is_empty() {
            self.push_diag(
                "cem.transform_config.ref_empty",
                format!("node `{}` has empty `@{field}` reference", node.id),
            );
        } else if !known.contains(target) {
            self.push_diag(
                "cem.transform_config.ref_unknown",
                format!(
                    "node `{}` references unknown graph node `{target}` via `@{field}`",
                    node.id
                ),
            );
        }
    }

    fn validate_outputs(&mut self, nodes: &[TransformGraphNode]) {
        let mut destinations = BTreeSet::new();
        for node in nodes {
            if node.kind != TransformGraphNodeKind::Export {
                continue;
            }
            let Some(out) = node.out.as_deref() else {
                continue;
            };
            if out.contains('*') {
                self.push_diag(
                    "cem.transform_config.output_pattern_wildcard",
                    format!(
                        "export node `{}` uses `*` in `@out`; use named path templates such as `{{stem}}`",
                        node.id
                    ),
                );
            }
            if !destinations.insert(out.to_owned()) {
                self.push_diag(
                    "cem.transform_config.output_duplicate",
                    format!("export destination pattern `{out}` is declared more than once"),
                );
            }
        }
    }

    fn validate_cycles(&mut self) {
        let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for edge in &self.graph.edges {
            outgoing
                .entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
        }

        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let node_ids: Vec<String> = self
            .graph
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect();
        for node_id in &node_ids {
            if has_cycle(node_id, &outgoing, &mut visiting, &mut visited) {
                self.push_diag(
                    "cem.transform_config.cycle",
                    format!("transform graph contains a cycle involving `{node_id}`"),
                );
                return;
            }
        }
    }

    fn push_diag(&mut self, code: &str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            uri: self.base_uri.map(str::to_owned),
            code: code.to_owned(),
            severity: Severity::Fatal,
            message: message.into(),
            ..Diagnostic::default()
        });
    }

    fn classify_template_kind(&mut self, node: &mut TransformGraphNode) {
        let identity = FormatIdentity {
            content_type: node.template_content_type.clone().or_else(|| {
                node.src
                    .as_deref()
                    .and_then(run_config::infer_content_type_from_path)
            }),
            schema: node.template_schema.clone(),
            ..FormatIdentity::default()
        };

        let has_identity = identity.content_type.is_some() || identity.schema.is_some();
        if !has_identity {
            return;
        }

        match classify_transform_template_identity(&identity) {
            Ok(kind) => node.template_kind = Some(kind),
            Err(error) => self.push_diag(
                TRANSFORM_TEMPLATE_UNSUPPORTED_CODE,
                format!("transform node `{}`: {}", node.id, error.message),
            ),
        }
    }
}

fn has_cycle(
    node_id: &str,
    outgoing: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    if visited.contains(node_id) {
        return false;
    }
    if !visiting.insert(node_id.to_owned()) {
        return true;
    }
    for next in outgoing.get(node_id).into_iter().flatten() {
        if has_cycle(next, outgoing, visiting, visited) {
            return true;
        }
    }
    visiting.remove(node_id);
    visited.insert(node_id.to_owned());
    false
}

fn element_name(doc: &CemDocument, node_id: AstNodeId) -> Option<&str> {
    match doc.get(node_id) {
        Some(CemAstNode::Element { expanded_name, .. }) => Some(expanded_name.local_name.as_str()),
        _ => None,
    }
}

fn collect_attrs(
    doc: &CemDocument,
    node_id: AstNodeId,
) -> BTreeMap<(String, String), Option<String>> {
    let mut attrs = BTreeMap::new();
    let Some(CemAstNode::Element { attributes, .. }) = doc.get(node_id) else {
        return attrs;
    };
    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = doc.get(*attr_id)
        else {
            continue;
        };
        attrs.insert(
            (
                expanded_name.namespace_uri.clone(),
                expanded_name.local_name.clone(),
            ),
            value.clone(),
        );
    }
    attrs
}

fn attr_value(
    attrs: &BTreeMap<(String, String), Option<String>>,
    prefix: &str,
    local: &str,
) -> Option<String> {
    attrs
        .get(&(prefix.to_owned(), local.to_owned()))
        .cloned()
        .flatten()
}

fn with_refs(attrs: &BTreeMap<(String, String), Option<String>>) -> BTreeMap<String, String> {
    attrs
        .iter()
        .filter(|((prefix, _), _)| prefix == "with")
        .map(|((_, local), value)| (local.clone(), value.clone().unwrap_or_default()))
        .collect()
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn transform_config_error(
    code: &'static str,
    message: impl Into<String>,
) -> TransformGraphConfigError {
    TransformGraphConfigError {
        code,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> TransformGraphParseResponse {
        parse_transform_graph_config(TransformGraphParseRequest {
            bytes: input.as_bytes().to_vec(),
            identity: FormatIdentity {
                content_type: Some("text/cem-ml".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: Some("file:///workspace/transform.cem".to_owned()),
        })
        .expect("config parsed")
    }

    fn has_diag(response: &TransformGraphParseResponse, code: &str) -> bool {
        response.diagnostics.iter().any(|diag| diag.code == code)
    }

    #[test]
    fn schema_shape_declares_transform_config_surface() {
        let run = TRANSFORM_CONFIG_SCHEMA_ELEMENTS
            .iter()
            .find(|element| element.local_name == "run")
            .expect("run schema");
        let transform = TRANSFORM_CONFIG_SCHEMA_ELEMENTS
            .iter()
            .find(|element| element.local_name == "transform")
            .expect("transform schema");

        assert_eq!(TRANSFORM_CONFIG_SCHEMA_URI, TRANSFORM_CONFIG_NAMESPACE_URI);
        assert_eq!(run.child_elements, &["import"]);
        assert_eq!(transform.required_attributes, &["src"]);
        assert!(transform.optional_attributes.contains(&"with:*"));
        assert!(transform.child_elements.contains(&"export"));
    }

    #[test]
    fn accepts_transform_config_schema_identity() {
        let response = parse_transform_graph_config(TransformGraphParseRequest {
            bytes: br#"{@doc cem-ml 1}{run | {import @id=book @src="book.xml"}}"#.to_vec(),
            identity: FormatIdentity {
                content_type: Some("text/cem-ml".to_owned()),
                schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.to_owned()),
                default_namespace: Some(TRANSFORM_CONFIG_NAMESPACE_URI.to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect("transform config schema identity accepted");

        assert_eq!(response.graph.nodes.len(), 1);
    }

    #[test]
    fn rejects_cem_core_schema_identity_for_transform_config() {
        let error = parse_transform_graph_config(TransformGraphParseRequest {
            bytes: br#"{@doc cem-ml 1}{run | {import @id=book @src="book.xml"}}"#.to_vec(),
            identity: FormatIdentity {
                content_type: Some("text/cem-ml".to_owned()),
                schema: Some(crate::schema::ir::CEM_CORE_NAMESPACE.to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect_err("CEM core schema is not transform config schema");

        assert_eq!(
            error.code,
            "cem.transform_config.unsupported_schema_identity"
        );
        assert!(error
            .message
            .contains(crate::schema::ir::CEM_CORE_NAMESPACE));
        assert!(error.message.contains(TRANSFORM_CONFIG_SCHEMA_URI));
    }

    #[test]
    fn parses_nested_branching_graph() {
        let response = parse(
            r#"{@doc cem-ml 1}
{run |
  {import @id=book @src="inputs/*.xml" @content-type="application/xml" |
    {transform @id=base @src="templates/book.xslt" @template-content-type="application/xslt+xml" |
      {export @id=main @out="book/chapters/{stem}.html" @content-type="text/html"}
      {transform @id=chart @src="illustrations/chart1.xslt" |
        {export @id=chart-out @out="book/chapters/{stem}/img/chart1.svg" @content-type="image/svg+xml"}
      }
    }
  }
}"#,
        );

        assert!(!has_diag(&response, "cem.transform_config.ref_unknown"));
        assert_eq!(response.graph.nodes.len(), 5);
        assert_eq!(
            response
                .graph
                .nodes
                .iter()
                .find(|node| node.id == "base")
                .and_then(|node| node.template_content_type.as_deref()),
            Some("application/xslt+xml")
        );
        assert_eq!(
            response
                .graph
                .nodes
                .iter()
                .find(|node| node.id == "base")
                .and_then(|node| node.template_kind),
            Some(TransformTemplateKind::Xslt)
        );
        assert_eq!(
            response
                .graph
                .nodes
                .iter()
                .find(|node| node.id == "chart")
                .and_then(|node| node.template_kind),
            Some(TransformTemplateKind::Xslt)
        );
        assert!(response.graph.edges.iter().any(|edge| {
            edge.from == "book" && edge.to == "base" && edge.role == TransformGraphEdgeRole::Parent
        }));
        assert!(response.graph.edges.iter().any(|edge| {
            edge.from == "base" && edge.to == "main" && edge.role == TransformGraphEdgeRole::Parent
        }));
        assert!(response.graph.edges.iter().any(|edge| {
            edge.from == "base" && edge.to == "chart" && edge.role == TransformGraphEdgeRole::Parent
        }));
    }

    #[test]
    fn classifies_cem_native_template_from_src_extension() {
        let response = parse(
            r#"{@doc cem-ml 1}
{run |
  {import @id=book @src="inputs/book.xml" |
    {transform @id=render @src="templates/book.cem" |
      {export @id=html @out="out/{stem}.html"}
    }
  }
}"#,
        );

        assert_eq!(
            response
                .graph
                .nodes
                .iter()
                .find(|node| node.id == "render")
                .and_then(|node| node.template_kind),
            Some(TransformTemplateKind::CemNative)
        );
        assert!(!has_diag(
            &response,
            "cem.transform_config.template_identity_missing"
        ));
    }

    #[test]
    fn lowers_explicit_cross_input_join_refs() {
        let response = parse(
            r#"{@doc cem-ml 1}
{run |
  {import @id=book @src="inputs/book.xml"}
  {import @id=stats @src="inputs/stats.xml"}
  {import @id=book-primary @src="inputs/primary.xml" |
    {transform @id=report @src="templates/report.xslt" @with:stats=stats |
      {export @id=html @out="out/{stem}.html"}
    }
  }
}"#,
        );

        let report = response
            .graph
            .nodes
            .iter()
            .find(|node| node.id == "report")
            .expect("report node");
        assert_eq!(report.with.get("stats").map(String::as_str), Some("stats"));
        assert!(response.graph.edges.iter().any(|edge| {
            edge.from == "stats" && edge.to == "report" && edge.role == TransformGraphEdgeRole::With
        }));
    }

    #[test]
    fn validates_missing_required_attrs_duplicate_ids_refs_and_outputs() {
        let response = parse(
            r#"{@doc cem-ml 1}
{run |
  {import @id=a}
  {import @id=a @src="other.xml"}
  {import @id=source @src="source.xml" |
    {transform @id=t @with:missing=unknown |
      {export @id=out1 @out="out/*.html"}
      {export @id=out2 @out="out/*.html"}
    }
  }
}"#,
        );

        for code in [
            "cem.transform_config.import_src_missing",
            "cem.transform_config.id_duplicate",
            "cem.transform_config.transform_src_missing",
            "cem.transform_config.ref_unknown",
            "cem.transform_config.output_pattern_wildcard",
            "cem.transform_config.output_duplicate",
        ] {
            assert!(has_diag(&response, code), "missing diagnostic {code}");
        }
    }

    #[test]
    fn validates_unknown_template_identity() {
        let response = parse(
            r#"{@doc cem-ml 1}
{run |
  {import @id=source @src="source.xml" |
    {transform @id=t @src="templates/view.bin" @template-content-type="application/octet-stream" |
      {export @id=out @out="out/{stem}.html"}
    }
  }
}"#,
        );

        assert!(has_diag(&response, TRANSFORM_TEMPLATE_UNSUPPORTED_CODE));
    }

    #[test]
    fn validates_missing_template_identity_when_src_extension_is_unknown() {
        let response = parse(
            r#"{@doc cem-ml 1}
{run |
  {import @id=source @src="source.xml" |
    {transform @id=t @src="templates/view.unknown" |
      {export @id=out @out="out/{stem}.html"}
    }
  }
}"#,
        );

        assert!(has_diag(
            &response,
            "cem.transform_config.template_identity_missing"
        ));
    }

    #[test]
    fn validates_explicit_ref_cycles() {
        let response = parse(
            r#"{@doc cem-ml 1}
{run |
  {import @id=source @src="source.xml" |
    {transform @id=a @input=b @src="a.xslt"}
    {transform @id=b @input=a @src="b.xslt"}
  }
}"#,
        );

        assert!(has_diag(&response, "cem.transform_config.cycle"));
    }

    #[test]
    fn rejects_non_cem_content_type() {
        let error = parse_transform_graph_config(TransformGraphParseRequest {
            bytes: b"{}".to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/json".to_owned()),
                ..FormatIdentity::default()
            },
            base_uri: None,
        })
        .expect_err("json rejected");

        assert_eq!(error.code, "cem.transform_config.unsupported_content_type");
    }
}
