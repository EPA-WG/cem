//! Embedded CEM-QL expression extraction from checked-in CEM/CEMT assets.
//!
//! This layer intentionally extracts only. Parsing, type checking, runtime bindings,
//! and waivers are added by later audit phases so stale host syntax can be reported
//! with the original file/range instead of being hidden by a lossy pre-pass.

use std::path::{Component, Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use std::{fs, io, process::Command};

use cem_ml::source::{ByteRange, BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaTokenKind, SchemaTokenizer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedExpression {
    pub source_path: PathBuf,
    pub schema_package: Option<SchemaPackageIdentity>,
    pub artifact_role: EmbeddedArtifactRole,
    pub host_kind: EmbeddedHostKind,
    pub host_node: Option<String>,
    pub attribute_name: Option<String>,
    pub source: String,
    pub normalized_source: String,
    pub host_range: ByteRange,
    pub expression_range: ByteRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaPackageIdentity {
    pub package_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedArtifactRole {
    Formatter,
    Colorizer,
    Converter,
    Validation,
    Schema,
    PackageManifest,
    Example,
    DocumentationFixture,
    Demo,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedHostKind {
    AttributeValueTemplate,
    SelectAttribute,
    MatchAttribute,
    TestAttribute,
    BehaviorSelectAttribute,
    BehaviorMatchAttribute,
    ExpressionNode,
}

#[derive(Debug, Clone)]
struct NodeContext {
    name: String,
    start: u64,
    inside_behavior: bool,
}

/// Extract CEM-QL expression spans from one CEM/CEMT source file.
pub fn extract_embedded_expressions_from_source(
    source_path: impl Into<PathBuf>,
    source: &str,
) -> Vec<EmbeddedExpression> {
    let source_path = source_path.into();
    let base_role = classify_artifact_role(&source_path);
    let schema_package = schema_package_identity(&source_path);
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut expressions = Vec::new();
    let mut stack: Vec<NodeContext> = Vec::new();

    while let Some(token) = tokenizer.next_token() {
        match token.kind {
            SchemaTokenKind::NodeStart { name } => {
                let inside_behavior = local_name(&name) == "behavior"
                    || stack.last().is_some_and(|node| node.inside_behavior);
                stack.push(NodeContext {
                    name,
                    start: token.byte_range.start,
                    inside_behavior,
                });
            }
            SchemaTokenKind::NodeEnd { .. } => {
                stack.pop();
            }
            SchemaTokenKind::Attribute {
                name, value_range, ..
            } => {
                let Some(value_range) = value_range else {
                    continue;
                };
                let host_node = stack.last().map(|node| node.name.clone());
                let inside_behavior = stack.last().is_some_and(|node| node.inside_behavior);
                if let Some(host_kind) =
                    whole_attribute_expression_kind(&name, host_node.as_deref())
                {
                    let expression_range = trim_range(source, strip_quotes(source, value_range));
                    push_expression(EmbeddedExpressionInput {
                        expressions: &mut expressions,
                        source_path: &source_path,
                        schema_package: schema_package.clone(),
                        artifact_role: expression_role(base_role, inside_behavior),
                        host_kind,
                        host_node,
                        attribute_name: Some(name),
                        source,
                        host_range: value_range,
                        expression_range,
                    });
                } else {
                    extract_avt_expressions(AvtExtractionInput {
                        expressions: &mut expressions,
                        source_path: &source_path,
                        schema_package: schema_package.clone(),
                        artifact_role: expression_role(base_role, inside_behavior),
                        host_node,
                        attribute_name: Some(name),
                        source,
                        host_range: value_range,
                        body_range: strip_quotes(source, value_range),
                    });
                }
            }
            SchemaTokenKind::ExpressionNode(_) => {
                let host_node = stack.last().map(|node| node.name.clone());
                let inside_behavior = stack.last().is_some_and(|node| node.inside_behavior);
                let host_start = stack
                    .last()
                    .map(|node| node.start)
                    .unwrap_or(token.byte_range.start);
                let host_range = ByteRange::new(
                    host_start,
                    token.byte_range.end().saturating_sub(host_start) as u32,
                );
                let expression_range = trim_range(source, token.byte_range);
                push_expression(EmbeddedExpressionInput {
                    expressions: &mut expressions,
                    source_path: &source_path,
                    schema_package: schema_package.clone(),
                    artifact_role: expression_role(base_role, inside_behavior),
                    host_kind: EmbeddedHostKind::ExpressionNode,
                    host_node,
                    attribute_name: None,
                    source,
                    host_range,
                    expression_range,
                });
            }
            _ => {}
        }
    }

    expressions
}

#[cfg(not(target_arch = "wasm32"))]
/// Extract CEM-QL expressions from every checked-in `*.cem` and `*.cemt` file.
pub fn extract_repository_embedded_expressions(
    workspace_root: impl AsRef<Path>,
) -> io::Result<Vec<EmbeddedExpression>> {
    let workspace_root = workspace_root.as_ref();
    let mut expressions = Vec::new();
    for rel_path in checked_in_cem_sources(workspace_root)? {
        let abs_path = workspace_root.join(&rel_path);
        let source = fs::read_to_string(&abs_path)?;
        expressions.extend(extract_embedded_expressions_from_source(rel_path, &source));
    }
    Ok(expressions)
}

#[cfg(not(target_arch = "wasm32"))]
/// Return checked-in CEM/CEMT files. Falls back to a conservative walk outside git.
pub fn checked_in_cem_sources(workspace_root: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
    let workspace_root = workspace_root.as_ref();
    if let Ok(output) = Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .arg("ls-files")
        .arg("-z")
        .arg("--")
        .arg("*.cem")
        .arg("*.cemt")
        .output()
    {
        if output.status.success() {
            let mut paths = output
                .stdout
                .split(|byte| *byte == 0)
                .filter(|entry| !entry.is_empty())
                .map(|entry| PathBuf::from(String::from_utf8_lossy(entry).as_ref()))
                .collect::<Vec<_>>();
            paths.sort();
            return Ok(paths);
        }
    }

    let mut paths = Vec::new();
    walk_cem_sources(workspace_root, workspace_root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

struct EmbeddedExpressionInput<'a> {
    expressions: &'a mut Vec<EmbeddedExpression>,
    source_path: &'a Path,
    schema_package: Option<SchemaPackageIdentity>,
    artifact_role: EmbeddedArtifactRole,
    host_kind: EmbeddedHostKind,
    host_node: Option<String>,
    attribute_name: Option<String>,
    source: &'a str,
    host_range: ByteRange,
    expression_range: ByteRange,
}

fn push_expression(input: EmbeddedExpressionInput<'_>) {
    if input.expression_range.len == 0 {
        return;
    }
    let source = slice_range(input.source, input.expression_range)
        .unwrap_or_default()
        .to_owned();
    let normalized_source = normalize_host_expression(&source).to_owned();
    input.expressions.push(EmbeddedExpression {
        source_path: input.source_path.to_path_buf(),
        schema_package: input.schema_package,
        artifact_role: input.artifact_role,
        host_kind: input.host_kind,
        host_node: input.host_node,
        attribute_name: input.attribute_name,
        source,
        normalized_source,
        host_range: input.host_range,
        expression_range: input.expression_range,
    });
}

struct AvtExtractionInput<'a> {
    expressions: &'a mut Vec<EmbeddedExpression>,
    source_path: &'a Path,
    schema_package: Option<SchemaPackageIdentity>,
    artifact_role: EmbeddedArtifactRole,
    host_node: Option<String>,
    attribute_name: Option<String>,
    source: &'a str,
    host_range: ByteRange,
    body_range: ByteRange,
}

fn extract_avt_expressions(input: AvtExtractionInput<'_>) {
    let Some(body) = slice_range(input.source, input.body_range) else {
        return;
    };
    let mut chars = body.char_indices().peekable();
    while let Some((offset, c)) = chars.next() {
        if c != '{' {
            continue;
        }
        if matches!(chars.peek(), Some((_, '{'))) {
            chars.next();
            continue;
        }

        let mut depth = 1u32;
        let body_start = offset + 1;
        let mut body_end = None;
        while let Some((inner_offset, inner)) = chars.next() {
            match inner {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        body_end = Some(inner_offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = body_end else {
            break;
        };
        let expression_range = trim_range(
            input.source,
            ByteRange::new(
                input.body_range.start + body_start as u64,
                (end - body_start) as u32,
            ),
        );
        push_expression(EmbeddedExpressionInput {
            expressions: input.expressions,
            source_path: input.source_path,
            schema_package: input.schema_package.clone(),
            artifact_role: input.artifact_role,
            host_kind: EmbeddedHostKind::AttributeValueTemplate,
            host_node: input.host_node.clone(),
            attribute_name: input.attribute_name.clone(),
            source: input.source,
            host_range: input.host_range,
            expression_range,
        });
    }
}

fn whole_attribute_expression_kind(
    attribute_name: &str,
    host_node: Option<&str>,
) -> Option<EmbeddedHostKind> {
    match local_name(attribute_name) {
        "select" if host_node.map(local_name) == Some("behavior") => {
            Some(EmbeddedHostKind::BehaviorSelectAttribute)
        }
        "match" if host_node.map(local_name) == Some("behavior") => {
            Some(EmbeddedHostKind::BehaviorMatchAttribute)
        }
        "select" => Some(EmbeddedHostKind::SelectAttribute),
        "match" => Some(EmbeddedHostKind::MatchAttribute),
        "test" => Some(EmbeddedHostKind::TestAttribute),
        _ => None,
    }
}

fn expression_role(base_role: EmbeddedArtifactRole, inside_behavior: bool) -> EmbeddedArtifactRole {
    if inside_behavior {
        EmbeddedArtifactRole::Validation
    } else {
        base_role
    }
}

pub fn classify_artifact_role(path: impl AsRef<Path>) -> EmbeddedArtifactRole {
    let path = path.as_ref();
    let components = normalized_components(path);
    if components
        .iter()
        .any(|component| matches!(component.as_str(), "formatters"))
    {
        return EmbeddedArtifactRole::Formatter;
    }
    if components
        .iter()
        .any(|component| matches!(component.as_str(), "colorizers"))
    {
        return EmbeddedArtifactRole::Colorizer;
    }
    if components
        .iter()
        .any(|component| matches!(component.as_str(), "converters"))
    {
        return EmbeddedArtifactRole::Converter;
    }
    if components.iter().any(|component| {
        matches!(
            component.as_str(),
            "validators" | "validations" | "validation"
        )
    }) {
        return EmbeddedArtifactRole::Validation;
    }
    if path.file_name().and_then(|name| name.to_str()) == Some("package.cem") {
        return EmbeddedArtifactRole::PackageManifest;
    }
    if components.iter().any(|component| component == "schema") {
        return EmbeddedArtifactRole::Schema;
    }
    if components
        .windows(2)
        .any(|window| window == ["docs", "examples"])
    {
        return EmbeddedArtifactRole::DocumentationFixture;
    }
    if components.iter().any(|component| component == "demo") {
        return EmbeddedArtifactRole::Demo;
    }
    if components.iter().any(|component| component == "examples") {
        return EmbeddedArtifactRole::Example;
    }
    EmbeddedArtifactRole::Unknown
}

pub fn schema_package_identity(path: impl AsRef<Path>) -> Option<SchemaPackageIdentity> {
    let components = normalized_components(path.as_ref());
    let package_index = components
        .iter()
        .position(|component| component == "schema-packages")?;
    let package_id = components.get(package_index + 1)?;
    let version = components.get(package_index + 2)?;
    if !version.starts_with('v') {
        return None;
    }
    Some(SchemaPackageIdentity {
        package_id: package_id.clone(),
        version: version.clone(),
    })
}

fn normalized_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => value.to_str().map(str::to_owned),
            _ => None,
        })
        .collect()
}

fn local_name(name: &str) -> &str {
    name.rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(name)
}

fn strip_quotes(source: &str, range: ByteRange) -> ByteRange {
    let Some(text) = slice_range(source, range) else {
        return range;
    };
    let bytes = text.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        return ByteRange::new(range.start + 1, range.len.saturating_sub(2));
    }
    range
}

fn trim_range(source: &str, range: ByteRange) -> ByteRange {
    let Some(text) = slice_range(source, range) else {
        return range;
    };
    let Some((start, _)) = text.char_indices().find(|(_, c)| !c.is_whitespace()) else {
        return ByteRange::new(range.start, 0);
    };
    let end = text
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(offset, c)| offset + c.len_utf8())
        .unwrap_or(start);
    ByteRange::new(range.start + start as u64, (end - start) as u32)
}

fn slice_range(source: &str, range: ByteRange) -> Option<&str> {
    let start = usize::try_from(range.start).ok()?;
    let end = usize::try_from(range.end()).ok()?;
    source.get(start..end)
}

fn normalize_host_expression(source: &str) -> &str {
    let trimmed = source.trim();
    if let Some(rest) = trimmed.strip_prefix('$') {
        let is_simple_binding = !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
        if is_simple_binding {
            return rest;
        }
    }
    trimmed
}

#[cfg(not(target_arch = "wasm32"))]
fn walk_cem_sources(root: &Path, dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            if is_ignored_walk_dir(&path) {
                continue;
            }
            walk_cem_sources(root, &path, paths)?;
            continue;
        }
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "cem" | "cemt"))
        {
            let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            paths.push(rel);
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn is_ignored_walk_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | ".nx" | "node_modules" | "dist" | "target" | "storybook-static"
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_template_attributes_and_expression_nodes() {
        let source = r#"{root @class="card {datadom.attributes.kind}" |
  {cem:if @test='node.kind == "element"' |
    {cem:for-each @select="$node.children" @as="child" |
      {$ $child.name }
    }
  }
}"#;
        let expressions = extract_embedded_expressions_from_source(
            "packages/cem-elements/demo/sample.cemt",
            source,
        );
        let rows = expressions
            .iter()
            .map(|expression| {
                (
                    expression.host_kind,
                    expression.attribute_name.as_deref(),
                    expression.source.as_str(),
                    expression.normalized_source.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            rows,
            vec![
                (
                    EmbeddedHostKind::AttributeValueTemplate,
                    Some("class"),
                    "datadom.attributes.kind",
                    "datadom.attributes.kind"
                ),
                (
                    EmbeddedHostKind::TestAttribute,
                    Some("test"),
                    r#"node.kind == "element""#,
                    r#"node.kind == "element""#
                ),
                (
                    EmbeddedHostKind::SelectAttribute,
                    Some("select"),
                    "$node.children",
                    "node.children"
                ),
                (
                    EmbeddedHostKind::ExpressionNode,
                    None,
                    "$child.name",
                    "child.name"
                ),
            ]
        );
        assert_eq!(expressions[0].artifact_role, EmbeddedArtifactRole::Demo);
    }

    #[test]
    fn classifies_behavior_queries_as_validation_expressions() {
        let source = r#"{schema |
  {behavior @select="resource" @match='kind == "page"' |
    {function @name="result" | {body | {$ { message: $candidate.name } }}}
  }
}"#;
        let expressions = extract_embedded_expressions_from_source(
            "packages/cem_ml/schema-packages/schema/v1/examples/behavior.cem",
            source,
        );
        assert_eq!(
            expressions
                .iter()
                .map(|expression| (expression.host_kind, expression.artifact_role))
                .collect::<Vec<_>>(),
            vec![
                (
                    EmbeddedHostKind::BehaviorSelectAttribute,
                    EmbeddedArtifactRole::Validation
                ),
                (
                    EmbeddedHostKind::BehaviorMatchAttribute,
                    EmbeddedArtifactRole::Validation
                ),
                (
                    EmbeddedHostKind::ExpressionNode,
                    EmbeddedArtifactRole::Validation
                )
            ]
        );
        assert_eq!(
            expressions[0].schema_package,
            Some(SchemaPackageIdentity {
                package_id: "schema".to_owned(),
                version: "v1".to_owned()
            })
        );
    }
}
