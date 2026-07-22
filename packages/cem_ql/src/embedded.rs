//! Embedded CEM-QL expression extraction and compile auditing for checked-in
//! CEM/CEMT assets.
//!
//! Runtime fixture validation and waivers live in later audit phases. This layer
//! preserves source provenance while compiling extracted expressions through the
//! Rust-first parser, resolver, and type checker so stale host syntax fails with
//! canonical CEM-QL diagnostics.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use std::{fs, io, process::Command};

use cem_ml::diagnostics::Diagnostic;
use cem_ml::source::{ByteRange, BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaTokenKind, SchemaTokenizer};

use crate::api;
use crate::parser::{
    Expression, FunctionParam, PathStep, PipelineStep, RecordKey, SurfaceModule, SurfaceNode,
    TypeExpr,
};
use crate::resolve::{
    Arity, BindingKind, BindingSet, ImportPolicy, NameResolver, QNameKey, SchemaTypeId,
};
use crate::types::{FunctionSignature, SchemaTypeInfo, TyConfig, Type, TypeChecker};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedExpression {
    pub source: String,
    pub normalized_source: String,
    pub provenance: EmbeddedExpressionProvenance,
}

impl EmbeddedExpression {
    pub fn source_path(&self) -> &Path {
        &self.provenance.source_path
    }

    pub fn schema_package(&self) -> Option<&SchemaPackageIdentity> {
        self.provenance.schema_package.as_ref()
    }

    pub fn artifact_role(&self) -> EmbeddedArtifactRole {
        self.provenance.artifact_role
    }

    pub fn host_kind(&self) -> EmbeddedHostKind {
        self.provenance.host.kind
    }

    pub fn host_range(&self) -> ByteRange {
        self.provenance.host.range
    }

    pub fn expression_range(&self) -> ByteRange {
        self.provenance.cem_ql_range
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedExpressionProvenance {
    pub source_path: PathBuf,
    pub schema_package: Option<SchemaPackageIdentity>,
    pub artifact_role: EmbeddedArtifactRole,
    pub host: EmbeddedHostProvenance,
    pub cem_ql_range: ByteRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedHostProvenance {
    pub kind: EmbeddedHostKind,
    pub node_name: Option<String>,
    pub attribute_name: Option<String>,
    pub range: ByteRange,
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
    Validator,
    TransformConfig,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedCompileStage {
    Parse,
    Resolve,
    TypeCheck,
}

#[derive(Debug, Clone)]
pub struct EmbeddedCompileDiagnostic {
    pub stage: EmbeddedCompileStage,
    pub diagnostic: Diagnostic,
    pub local_byte_offset: Option<u64>,
    pub source_byte_offset: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct EmbeddedExpressionCompileReport {
    pub expression: EmbeddedExpression,
    pub parse_succeeded: bool,
    pub resolve_ran: bool,
    pub resolve_succeeded: bool,
    pub type_check_ran: bool,
    pub type_check_succeeded: bool,
    pub root_type: Option<Type>,
    pub diagnostics: Vec<EmbeddedCompileDiagnostic>,
}

impl EmbeddedExpressionCompileReport {
    pub fn has_hard_diagnostics(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.diagnostic.severity.is_hard_violation())
    }

    pub fn hard_diagnostics(&self) -> impl Iterator<Item = &EmbeddedCompileDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.diagnostic.severity.is_hard_violation())
    }

    pub fn diagnostics_for_stage(
        &self,
        stage: EmbeddedCompileStage,
    ) -> impl Iterator<Item = &EmbeddedCompileDiagnostic> {
        self.diagnostics
            .iter()
            .filter(move |diagnostic| diagnostic.stage == stage)
    }
}

#[derive(Debug, Clone, Default)]
struct EmbeddedHostCompileSupport {
    variables: BTreeSet<QNameKey>,
    functions: BTreeSet<(QNameKey, Arity)>,
    types: BTreeSet<QNameKey>,
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

/// Compile one extracted embedded expression through parser, resolver, and type checker.
pub fn compile_embedded_expression(
    expression: &EmbeddedExpression,
) -> EmbeddedExpressionCompileReport {
    let mut diagnostics = Vec::new();
    let parsed = api::parse(&expression.normalized_source);
    diagnostics.extend(map_compile_diagnostics(
        EmbeddedCompileStage::Parse,
        expression,
        &parsed.diagnostics,
    ));
    let parse_succeeded = !parsed
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation());

    let mut report = EmbeddedExpressionCompileReport {
        expression: expression.clone(),
        parse_succeeded,
        resolve_ran: false,
        resolve_succeeded: false,
        type_check_ran: false,
        type_check_succeeded: false,
        root_type: None,
        diagnostics,
    };

    if !parse_succeeded {
        return report;
    }

    let support = EmbeddedHostCompileSupport::from_module(&parsed.module);

    let mut resolver = NameResolver::new();
    let host_site = support.resolver_binding_set(&mut resolver);
    resolver.push_site(host_site);
    let resolution_report = resolver.resolve_surface_module(&parsed.module, &ImportPolicy::new());
    report.resolve_ran = true;
    report.resolve_succeeded = !resolution_report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation());
    report.diagnostics.extend(map_compile_diagnostics(
        EmbeddedCompileStage::Resolve,
        expression,
        &resolution_report.diagnostics,
    ));

    let mut type_checker = TypeChecker::with_config(TyConfig::dev_profile());
    support.seed_type_checker(&mut type_checker);
    let type_report = type_checker.check_surface_module(&parsed.module);
    report.type_check_ran = true;
    report.type_check_succeeded = !type_report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation());
    report.root_type = type_report.root_type;
    report.diagnostics.extend(map_compile_diagnostics(
        EmbeddedCompileStage::TypeCheck,
        expression,
        &type_report.diagnostics,
    ));

    report
}

/// Compile a batch of extracted embedded expressions.
pub fn compile_embedded_expressions(
    expressions: &[EmbeddedExpression],
) -> Vec<EmbeddedExpressionCompileReport> {
    expressions
        .iter()
        .map(compile_embedded_expression)
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
/// Extract and compile every checked-in embedded expression.
pub fn compile_repository_embedded_expressions(
    workspace_root: impl AsRef<Path>,
) -> io::Result<Vec<EmbeddedExpressionCompileReport>> {
    let expressions = extract_repository_embedded_expressions(workspace_root)?;
    Ok(compile_embedded_expressions(&expressions))
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

impl EmbeddedHostCompileSupport {
    fn from_module(module: &SurfaceModule) -> Self {
        let mut support = Self::default();
        for node in &module.nodes {
            support.collect_surface_node(node);
        }
        support
    }

    fn resolver_binding_set(&self, resolver: &mut NameResolver) -> BindingSet {
        let mut site = BindingSet::new(10_000);
        for name in &self.variables {
            resolver.declare_binding(&mut site, BindingKind::Variable, name.clone(), None, None);
        }
        for (name, arity) in &self.functions {
            resolver.declare_binding(
                &mut site,
                BindingKind::Function,
                name.clone(),
                Some(*arity),
                None,
            );
        }
        for name in &self.types {
            resolver.declare_binding(&mut site, BindingKind::SchemaType, name.clone(), None, None);
        }
        site
    }

    fn seed_type_checker(&self, type_checker: &mut TypeChecker) {
        for name in &self.variables {
            type_checker.declare_variable(name.clone(), Type::Any);
        }
        for (name, arity) in &self.functions {
            let Ok(param_count) = usize::try_from(arity.0) else {
                continue;
            };
            type_checker.register_function(FunctionSignature {
                name: name.clone(),
                params: vec![Type::Any; param_count],
                ret: Type::Any,
            });
        }
        for (index, name) in self.types.iter().enumerate() {
            type_checker.register_schema_type(SchemaTypeInfo {
                id: SchemaTypeId(index.try_into().unwrap_or(u32::MAX)),
                name: name.clone(),
                element_name: name.clone(),
                structural_supertypes: Vec::new(),
            });
        }
    }

    fn collect_surface_node(&mut self, node: &SurfaceNode) {
        match node {
            SurfaceNode::DeclareVariable(variable) => {
                self.collect_expression(&variable.value);
            }
            SurfaceNode::DeclareFunction(function) => {
                for param in &function.params {
                    self.collect_function_param(param);
                }
                self.collect_expression(&function.body);
            }
            SurfaceNode::Expression(expression) => {
                self.collect_expression(expression);
            }
            SurfaceNode::Module(_) | SurfaceNode::Import(_) => {}
        }
    }

    fn collect_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::Literal(_, _) | Expression::LeadingDot(_) => {}
            Expression::Name(name, _) => {
                self.variables.insert(QNameKey::from_qname(name));
            }
            Expression::Path { steps, .. } => {
                for step in steps {
                    if let PathStep::Axis { predicates, .. } = step {
                        for predicate in predicates {
                            self.collect_expression(predicate);
                        }
                    }
                }
            }
            Expression::Pipeline { source, steps, .. } => {
                self.collect_expression(source);
                for step in steps {
                    match step {
                        PipelineStep::Named { name, args, .. } => {
                            let plain_arity = Arity(args.len().try_into().unwrap_or(u32::MAX));
                            let piped_arity =
                                Arity((args.len() + 1).try_into().unwrap_or(u32::MAX));
                            self.functions
                                .insert((QNameKey::from_qname(name), plain_arity));
                            self.functions
                                .insert((QNameKey::from_qname(name), piped_arity));
                            for arg in args {
                                self.collect_expression(arg);
                            }
                        }
                        PipelineStep::Lambda { lambda, .. } => {
                            self.collect_expression(lambda);
                        }
                    }
                }
            }
            Expression::BinaryOp { lhs, rhs, .. } | Expression::SetOp { lhs, rhs, .. } => {
                self.collect_expression(lhs);
                self.collect_expression(rhs);
            }
            Expression::UnaryOp { operand, .. } => {
                self.collect_expression(operand);
            }
            Expression::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.collect_expression(cond);
                self.collect_expression(then_branch);
                self.collect_expression(else_branch);
            }
            Expression::Let { value, body, .. } => {
                self.collect_expression(value);
                self.collect_expression(body);
            }
            Expression::For { source, body, .. } => {
                self.collect_expression(source);
                self.collect_expression(body);
            }
            Expression::Quantified {
                source, predicate, ..
            } => {
                self.collect_expression(source);
                self.collect_expression(predicate);
            }
            Expression::Record { entries, .. } => {
                for entry in entries {
                    if let RecordKey::Computed { expr, .. } = &entry.key {
                        self.collect_expression(expr);
                    }
                    self.collect_expression(&entry.value);
                }
            }
            Expression::Sequence { items, .. } => {
                for item in items {
                    self.collect_expression(item);
                }
            }
            Expression::Call { callee, args, .. } => {
                if let Expression::Name(name, _) = callee.as_ref() {
                    self.functions.insert((
                        QNameKey::from_qname(name),
                        Arity(args.len().try_into().unwrap_or(u32::MAX)),
                    ));
                } else {
                    self.collect_expression(callee);
                }
                for arg in args {
                    self.collect_expression(arg);
                }
            }
            Expression::Lambda { params, body, .. } => {
                for param in params {
                    self.collect_function_param(param);
                }
                self.collect_expression(body);
            }
            Expression::InstanceOf { value, ty, .. }
            | Expression::CastAs { value, ty, .. }
            | Expression::TreatAs { value, ty, .. } => {
                self.collect_expression(value);
                self.collect_type(ty);
            }
        }
    }

    fn collect_function_param(&mut self, param: &FunctionParam) {
        if let Some(ty) = &param.type_annotation {
            self.collect_type(ty);
        }
    }

    fn collect_type(&mut self, ty: &TypeExpr) {
        self.types.insert(QNameKey::from_qname(&ty.name));
    }
}

fn map_compile_diagnostics(
    stage: EmbeddedCompileStage,
    expression: &EmbeddedExpression,
    diagnostics: &[Diagnostic],
) -> Vec<EmbeddedCompileDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let local_byte_offset = diagnostic.byte_offset;
            let source_byte_offset =
                local_byte_offset.map(|offset| expression.expression_range().start + offset);
            let mut mapped = diagnostic.clone();
            mapped
                .uri
                .get_or_insert_with(|| expression.source_path().to_string_lossy().into_owned());
            if let Some(source_byte_offset) = source_byte_offset {
                mapped.byte_offset = Some(source_byte_offset);
            }
            EmbeddedCompileDiagnostic {
                stage,
                diagnostic: mapped,
                local_byte_offset,
                source_byte_offset,
            }
        })
        .collect()
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
        source,
        normalized_source,
        provenance: EmbeddedExpressionProvenance {
            source_path: input.source_path.to_path_buf(),
            schema_package: input.schema_package,
            artifact_role: input.artifact_role,
            host: EmbeddedHostProvenance {
                kind: input.host_kind,
                node_name: input.host_node,
                attribute_name: input.attribute_name,
                range: input.host_range,
            },
            cem_ql_range: input.expression_range,
        },
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
        EmbeddedArtifactRole::Validator
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
        return EmbeddedArtifactRole::Validator;
    }
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".transform.cem"))
    {
        return EmbeddedArtifactRole::TransformConfig;
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
                    expression.provenance.host.kind,
                    expression.provenance.host.attribute_name.as_deref(),
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
        assert_eq!(
            expressions[0].provenance.artifact_role,
            EmbeddedArtifactRole::Demo
        );
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
                .map(|expression| {
                    (
                        expression.provenance.host.kind,
                        expression.provenance.artifact_role,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    EmbeddedHostKind::BehaviorSelectAttribute,
                    EmbeddedArtifactRole::Validator
                ),
                (
                    EmbeddedHostKind::BehaviorMatchAttribute,
                    EmbeddedArtifactRole::Validator
                ),
                (
                    EmbeddedHostKind::ExpressionNode,
                    EmbeddedArtifactRole::Validator
                )
            ]
        );
        assert_eq!(
            expressions[0].provenance.schema_package,
            Some(SchemaPackageIdentity {
                package_id: "schema".to_owned(),
                version: "v1".to_owned()
            })
        );
    }
}
