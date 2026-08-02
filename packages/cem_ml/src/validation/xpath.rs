use crate::diagnostics::{Diagnostic, Severity};
use crate::resolver::{ResolverPolicy, ResolverRegistry};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::content_type_essence;
pub use crate::schema::registry::{
    XPATH_CONTENT_TYPE, XPATH_RESULT_CONTENT_TYPE, XPATH_SCHEMA_URI,
};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
use xee_xpath_ast::ast::XPath;
use xee_xpath_ast::{Namespaces, ParserError, VariableNames, XPathParserContext};
use xee_xpath_lexer::Token as XeeToken;

const XPATH_PACKAGE_ID: &str = "xpath";
const XPATH_FACT_BEHAVIOR: &str = "xpath-report-fact";
pub const XPATH_GRAMMAR_VERSION: &str = "xpath-3.1/xee-0.1.4";

#[derive(Debug, Clone, Copy)]
pub struct XPathSourceRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathExpressionSource {
    pub uri: String,
    pub content_type: String,
    pub media_type: String,
    pub parameters: BTreeMap<String, String>,
    pub byte_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathSourceRange {
    pub start: XPathSourcePosition,
    pub byte_length: u64,
}

impl XPathSourceRange {
    pub fn new(line: u32, column: u32, byte_offset: u64, byte_length: u64) -> Self {
        Self {
            start: XPathSourcePosition {
                line,
                column,
                byte_offset,
            },
            byte_length,
        }
    }

    fn from_offsets(
        line_index: &LineIndex,
        origin: XPathSourcePosition,
        start: usize,
        end: usize,
    ) -> Self {
        let coordinate = line_index.project(start as u64);
        let line = origin
            .line
            .saturating_add(coordinate.line.saturating_sub(1));
        let column = if coordinate.line == 1 {
            origin
                .column
                .saturating_add(coordinate.column.saturating_sub(1))
        } else {
            coordinate.column
        };
        Self::new(
            line,
            column,
            origin.byte_offset.saturating_add(start as u64),
            end.saturating_sub(start) as u64,
        )
    }

    fn to_cemt_subject(self) -> Value {
        json!({
            "byteOffset": self.start.byte_offset,
            "byteLength": self.byte_length,
            "line": self.start.line,
            "column": self.start.column,
        })
    }

    fn source_map(self, source_id: u32, content_type: &str) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(source_id),
                span: FrameSpan::Single(ByteRange::new(
                    self.start.byte_offset,
                    u32::try_from(self.byte_length).unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: content_type.to_owned(),
                },
            }],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathTokenKind {
    Keyword,
    Name,
    Number,
    String,
    Operator,
    Punctuation,
    VariableSigil,
    Comment,
    Whitespace,
    Error,
}

impl XPathTokenKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Name => "name",
            Self::Number => "number",
            Self::String => "string",
            Self::Operator => "operator",
            Self::Punctuation => "punctuation",
            Self::VariableSigil => "variable-sigil",
            Self::Comment => "comment",
            Self::Whitespace => "whitespace",
            Self::Error => "error",
        }
    }

    pub fn role(self) -> &'static str {
        match self {
            Self::Keyword => "syntax.keyword",
            Self::Name => "syntax.name",
            Self::Number => "syntax.number",
            Self::String => "syntax.string",
            Self::Operator => "syntax.operator",
            Self::Punctuation | Self::VariableSigil => "syntax.punctuation",
            Self::Comment => "syntax.comment",
            Self::Whitespace => "syntax.whitespace",
            Self::Error => "syntax.error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathTokenAst {
    pub index: usize,
    pub kind: XPathTokenKind,
    pub lexeme: String,
    pub depth: usize,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathAstEventKind {
    StartExpression,
    Token,
    EndExpression,
}

impl XPathAstEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartExpression => "start-expression",
            Self::Token => "token",
            Self::EndExpression => "end-expression",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathAstEvent {
    pub index: usize,
    pub kind: XPathAstEventKind,
    pub token_index: Option<usize>,
    pub depth: usize,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathHostNodeKind {
    XmlDocument,
    XmlSubtree,
    XmlElement,
    XmlAttribute,
    XsltAttribute,
    CemtExpressionSlot,
    CemQlExpressionSlot,
}

impl XPathHostNodeKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::XmlDocument => "xml-document",
            Self::XmlSubtree => "xml-subtree",
            Self::XmlElement => "xml-element",
            Self::XmlAttribute => "xml-attribute",
            Self::XsltAttribute => "xslt-attribute",
            Self::CemtExpressionSlot => "cemt-expression-slot",
            Self::CemQlExpressionSlot => "cem-ql-expression-slot",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathHostOwner {
    pub source_id: u32,
    pub source_uri: String,
    pub content_type: Option<String>,
    pub schema_uri: Option<String>,
    pub node_kind: XPathHostNodeKind,
    pub node_id: Option<String>,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathStaticContext {
    pub namespaces: BTreeMap<String, String>,
    pub default_element_namespace: Option<String>,
    pub default_function_namespace: Option<String>,
    pub variable_bindings: BTreeMap<String, String>,
    pub function_bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathEvaluationPhase {
    Validate,
    Compile,
    Transform,
    Render,
    Runtime,
}

impl XPathEvaluationPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Validate => "validate",
            Self::Compile => "compile",
            Self::Transform => "transform",
            Self::Render => "render",
            Self::Runtime => "runtime",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathExpectedResult {
    pub sequence_type: String,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathResultItemKind {
    Node,
    Atomic,
    Map,
    Array,
    Function,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathEvaluatorAstInput {
    PackageAst,
    SourceText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathEvaluatorResourceAccess {
    CemResolver,
    Direct,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathEvaluatorSourceMapMode {
    ItemOrigins,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathEvaluatorCapabilities {
    pub evaluator_id: String,
    pub evaluator_version: String,
    pub xpath_version: String,
    pub grammar_version: String,
    pub ast_input: XPathEvaluatorAstInput,
    pub resource_access: XPathEvaluatorResourceAccess,
    pub source_map_mode: XPathEvaluatorSourceMapMode,
    pub deterministic: bool,
    pub targets: BTreeSet<String>,
    pub result_item_kinds: BTreeSet<XPathResultItemKind>,
}

impl XPathEvaluatorCapabilities {
    pub fn required(evaluator_id: impl Into<String>, evaluator_version: impl Into<String>) -> Self {
        Self {
            evaluator_id: evaluator_id.into(),
            evaluator_version: evaluator_version.into(),
            xpath_version: "3.1".to_owned(),
            grammar_version: XPATH_GRAMMAR_VERSION.to_owned(),
            ast_input: XPathEvaluatorAstInput::PackageAst,
            resource_access: XPathEvaluatorResourceAccess::CemResolver,
            source_map_mode: XPathEvaluatorSourceMapMode::ItemOrigins,
            deterministic: true,
            targets: BTreeSet::from(["native".to_owned(), "wasm32-unknown-unknown".to_owned()]),
            result_item_kinds: BTreeSet::from([
                XPathResultItemKind::Node,
                XPathResultItemKind::Atomic,
                XPathResultItemKind::Map,
                XPathResultItemKind::Array,
                XPathResultItemKind::Function,
            ]),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathEvaluatorIdentity {
    pub evaluator_id: String,
    pub evaluator_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathAtomicValue {
    pub type_name: String,
    pub lexical_value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum XPathResultNodeKind {
    Document,
    Element,
    Attribute,
    Text,
    Comment,
    ProcessingInstruction,
    Namespace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathMapEntry {
    pub key: XPathAtomicValue,
    pub value: XPathResultSequence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathResultSequence {
    pub sequence_type: String,
    pub items: Vec<XPathResultItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum XPathResultItem {
    Node {
        node_kind: XPathResultNodeKind,
        source_id: u32,
        source_uri: String,
        node_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        expanded_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source_range: Option<XPathSourceRange>,
        source_map: SourceMapStack,
    },
    Atomic {
        value: XPathAtomicValue,
        source_map: SourceMapStack,
    },
    Map {
        entries: Vec<XPathMapEntry>,
        source_map: SourceMapStack,
    },
    Array {
        members: Vec<XPathResultSequence>,
        source_map: SourceMapStack,
    },
    Function {
        evaluator_id: String,
        function_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        arity: usize,
        signature: String,
        source_map: SourceMapStack,
    },
}

impl XPathResultItem {
    pub fn kind(&self) -> XPathResultItemKind {
        match self {
            Self::Node { .. } => XPathResultItemKind::Node,
            Self::Atomic { .. } => XPathResultItemKind::Atomic,
            Self::Map { .. } => XPathResultItemKind::Map,
            Self::Array { .. } => XPathResultItemKind::Array,
            Self::Function { .. } => XPathResultItemKind::Function,
        }
    }

    fn source_map(&self) -> &SourceMapStack {
        match self {
            Self::Node { source_map, .. }
            | Self::Atomic { source_map, .. }
            | Self::Map { source_map, .. }
            | Self::Array { source_map, .. }
            | Self::Function { source_map, .. } => source_map,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XPathResultArtifact {
    pub content_type: String,
    pub schema_uri: String,
    pub xpath_version: String,
    pub grammar_version: String,
    pub evaluator: XPathEvaluatorIdentity,
    pub expression_uri: String,
    pub static_context: XPathStaticContext,
    pub resolver_policy_stamp: String,
    pub safety_policy_stamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_result: Option<XPathExpectedResult>,
    pub sequence: XPathResultSequence,
    pub source_map: SourceMapStack,
}

pub struct XPathEvaluationRequest<'a> {
    pub expression: &'a XPathExpressionAst,
    pub context_item: Option<XPathResultItem>,
    pub variable_bindings: BTreeMap<String, XPathResultSequence>,
    pub static_context: XPathStaticContext,
    pub expected_result: Option<XPathExpectedResult>,
    pub resolver_registry: &'a ResolverRegistry,
    pub resolver_policy: &'a ResolverPolicy,
    pub safety_policy_stamp: &'a str,
}

pub trait XPathEvaluatorAdapter: Send + Sync {
    fn capabilities(&self) -> &XPathEvaluatorCapabilities;

    fn evaluate(
        &self,
        request: XPathEvaluationRequest<'_>,
    ) -> Result<XPathResultArtifact, Vec<Diagnostic>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathEvaluationContractViolation {
    pub code: &'static str,
    pub message: String,
}

pub fn validate_xpath_evaluator_capabilities(
    capabilities: &XPathEvaluatorCapabilities,
) -> Vec<XPathEvaluationContractViolation> {
    let mut violations = Vec::new();
    if capabilities.xpath_version != "3.1" {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-version-unsupported",
            "XPath evaluator must implement XPath 3.1",
        );
    }
    if capabilities.grammar_version != XPATH_GRAMMAR_VERSION {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-grammar-mismatch",
            "XPath evaluator grammar must match the package-owned syntax AST",
        );
    }
    if capabilities.ast_input != XPathEvaluatorAstInput::PackageAst {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-ast-reparse-forbidden",
            "XPath evaluator must consume XPathExpressionAst without reparsing source text",
        );
    }
    if capabilities.resource_access != XPathEvaluatorResourceAccess::CemResolver {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-resolver-bypass",
            "XPath evaluator resource reads must use the CEM resolver boundary",
        );
    }
    if capabilities.source_map_mode != XPathEvaluatorSourceMapMode::ItemOrigins {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-source-map-missing",
            "XPath evaluator must retain item-level source-map origins",
        );
    }
    if !capabilities.deterministic {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-evaluator-nondeterministic",
            "XPath evaluator must materialize deterministic result artifacts",
        );
    }
    for target in ["native", "wasm32-unknown-unknown"] {
        if !capabilities.targets.contains(target) {
            push_xpath_contract_violation(
                &mut violations,
                "xpath-evaluator-target-missing",
                format!("XPath evaluator does not support required target `{target}`"),
            );
        }
    }
    for kind in [
        XPathResultItemKind::Node,
        XPathResultItemKind::Atomic,
        XPathResultItemKind::Map,
        XPathResultItemKind::Array,
        XPathResultItemKind::Function,
    ] {
        if !capabilities.result_item_kinds.contains(&kind) {
            push_xpath_contract_violation(
                &mut violations,
                "xpath-evaluator-result-kind-missing",
                format!("XPath evaluator does not support `{kind:?}` result items"),
            );
        }
    }
    violations
}

pub fn validate_xpath_result_artifact(
    artifact: &XPathResultArtifact,
    capabilities: &XPathEvaluatorCapabilities,
) -> Vec<XPathEvaluationContractViolation> {
    let mut violations = validate_xpath_evaluator_capabilities(capabilities);
    if artifact.content_type != XPATH_RESULT_CONTENT_TYPE
        || artifact.schema_uri != XPATH_SCHEMA_URI
        || artifact.xpath_version != "3.1"
        || artifact.grammar_version != XPATH_GRAMMAR_VERSION
    {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-result-identity-invalid",
            "XPath result artifact identity does not match the package contract",
        );
    }
    if artifact.evaluator.evaluator_id != capabilities.evaluator_id
        || artifact.evaluator.evaluator_version != capabilities.evaluator_version
    {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-result-evaluator-mismatch",
            "XPath result artifact evaluator identity does not match the selected adapter",
        );
    }
    if artifact.resolver_policy_stamp.trim().is_empty()
        || artifact.safety_policy_stamp.trim().is_empty()
    {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-result-policy-stamp-missing",
            "XPath result artifact must retain resolver and safety policy stamps",
        );
    }
    if artifact.source_map.frames.is_empty() {
        push_xpath_contract_violation(
            &mut violations,
            "xpath-result-source-map-required",
            "XPath result artifact must retain its expression source-map origin",
        );
    }
    if let Some(expected) = &artifact.expected_result {
        let item_count = artifact.sequence.items.len();
        if expected
            .min_items
            .is_some_and(|minimum| item_count < minimum)
            || expected
                .max_items
                .is_some_and(|maximum| item_count > maximum)
        {
            push_xpath_contract_violation(
                &mut violations,
                "xpath-result-cardinality-mismatch",
                "XPath result sequence does not satisfy the expected result contract",
            );
        }
    }
    validate_xpath_result_sequence(
        &artifact.sequence,
        &artifact.evaluator.evaluator_id,
        capabilities,
        &mut violations,
    );
    violations
}

fn validate_xpath_result_sequence(
    sequence: &XPathResultSequence,
    evaluator_id: &str,
    capabilities: &XPathEvaluatorCapabilities,
    violations: &mut Vec<XPathEvaluationContractViolation>,
) {
    if sequence.sequence_type.trim().is_empty() {
        push_xpath_contract_violation(
            violations,
            "xpath-result-sequence-type-missing",
            "XPath result sequence must retain its sequence type",
        );
    }
    for item in &sequence.items {
        if !capabilities.result_item_kinds.contains(&item.kind()) {
            push_xpath_contract_violation(
                violations,
                "xpath-result-item-kind-unsupported",
                format!(
                    "XPath result item kind `{:?}` is not supported",
                    item.kind()
                ),
            );
        }
        if item.source_map().frames.is_empty() {
            push_xpath_contract_violation(
                violations,
                "xpath-result-source-map-required",
                format!(
                    "XPath `{:?}` result item has no source-map origin",
                    item.kind()
                ),
            );
        }
        match item {
            XPathResultItem::Node {
                source_uri,
                node_id,
                ..
            } => {
                if source_uri.trim().is_empty() || node_id.trim().is_empty() {
                    push_xpath_contract_violation(
                        violations,
                        "xpath-result-node-identity-missing",
                        "XPath node result must retain source and node identity",
                    );
                }
            }
            XPathResultItem::Atomic { value, .. } => {
                validate_xpath_atomic_value(value, violations);
            }
            XPathResultItem::Map { entries, .. } => {
                for entry in entries {
                    validate_xpath_atomic_value(&entry.key, violations);
                    validate_xpath_result_sequence(
                        &entry.value,
                        evaluator_id,
                        capabilities,
                        violations,
                    );
                }
            }
            XPathResultItem::Array { members, .. } => {
                for member in members {
                    validate_xpath_result_sequence(member, evaluator_id, capabilities, violations);
                }
            }
            XPathResultItem::Function {
                evaluator_id: function_evaluator_id,
                function_id,
                signature,
                ..
            } => {
                if function_evaluator_id != evaluator_id
                    || function_id.trim().is_empty()
                    || signature.trim().is_empty()
                {
                    push_xpath_contract_violation(
                        violations,
                        "xpath-result-function-scope-invalid",
                        "XPath function result must be an evaluator-scoped typed handle",
                    );
                }
            }
        }
    }
}

fn validate_xpath_atomic_value(
    value: &XPathAtomicValue,
    violations: &mut Vec<XPathEvaluationContractViolation>,
) {
    if value.type_name.trim().is_empty() {
        push_xpath_contract_violation(
            violations,
            "xpath-result-atomic-type-missing",
            "XPath atomic result must retain its type name",
        );
    }
    if value.type_name == "xs:QName"
        && (value.namespace_uri.is_none() || value.local_name.as_deref().is_none_or(str::is_empty))
    {
        push_xpath_contract_violation(
            violations,
            "xpath-result-qname-identity-missing",
            "XPath QName result must retain expanded-name identity",
        );
    }
}

fn push_xpath_contract_violation(
    violations: &mut Vec<XPathEvaluationContractViolation>,
    code: &'static str,
    message: impl Into<String>,
) {
    violations.push(XPathEvaluationContractViolation {
        code,
        message: message.into(),
    });
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathHostAttachment {
    pub owner: XPathHostOwner,
    pub expression_range: XPathSourceRange,
    pub static_context: XPathStaticContext,
    pub expected_result: Option<XPathExpectedResult>,
    pub evaluation_phase: XPathEvaluationPhase,
    pub resolver_policy_stamp: Option<String>,
    pub safety_policy_stamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathAttachment {
    Standalone { source_id: u32 },
    Host(XPathHostAttachment),
}

impl XPathAttachment {
    fn source_id(&self) -> u32 {
        match self {
            Self::Standalone { source_id } => *source_id,
            Self::Host(attachment) => attachment.owner.source_id,
        }
    }

    fn expression_origin(&self) -> XPathSourcePosition {
        match self {
            Self::Standalone { .. } => XPathSourcePosition {
                line: 1,
                column: 1,
                byte_offset: 0,
            },
            Self::Host(attachment) => attachment.expression_range.start,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum XPathFactKind {
    InvalidUtf8,
    LexicalError,
    ParseError,
    UnknownNamespacePrefix,
    UnclosedDelimiter,
    MismatchedDelimiter,
    HostAssociationInvalid,
    ExternalResourceDenied,
    SourceMapUnavailable,
    EventLifecycleInvalid,
    Parsed,
    HostAssociationObserved,
}

impl XPathFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidUtf8 => "invalid-utf8",
            Self::LexicalError => "lexical-error",
            Self::ParseError => "parse-error",
            Self::UnknownNamespacePrefix => "unknown-namespace-prefix",
            Self::UnclosedDelimiter => "unclosed-delimiter",
            Self::MismatchedDelimiter => "mismatched-delimiter",
            Self::HostAssociationInvalid => "host-association-invalid",
            Self::ExternalResourceDenied => "external-resource-denied",
            Self::SourceMapUnavailable => "source-map-unavailable",
            Self::EventLifecycleInvalid => "event-lifecycle-invalid",
            Self::Parsed => "parsed",
            Self::HostAssociationObserved => "host-association-observed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathFact {
    pub kind: XPathFactKind,
    pub source_range: Option<XPathSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathDiagnosticBinding {
    pub fact_kind: String,
    pub contract: String,
    pub behavior: Option<String>,
    pub diagnostic_code: String,
    pub severity: Severity,
    pub policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathSchemaContractCatalog {
    pub fact_bindings: BTreeMap<String, XPathDiagnosticBinding>,
}

impl XPathSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<XPathSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(XPATH_PACKAGE_ID)
                .expect("built-in XPath schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(XPATH_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != XPATH_FACT_BEHAVIOR {
                    return None;
                }
                let fact_kind = constraint.fact_kind.as_deref()?.trim();
                let diagnostic_code = constraint.diagnostic.as_deref()?.trim();
                if fact_kind.is_empty() || diagnostic_code.is_empty() {
                    return None;
                }
                let diagnostic = model.diagnostics.get(diagnostic_code)?;
                Some((
                    fact_kind.to_owned(),
                    XPathDiagnosticBinding {
                        fact_kind: fact_kind.to_owned(),
                        contract: constraint.kind.clone(),
                        behavior: constraint.behavior.clone(),
                        diagnostic_code: diagnostic.code.clone(),
                        severity: diagnostic.severity,
                        policy: constraint.policy.clone(),
                    },
                ))
            })
            .collect();
        Self { fact_bindings }
    }

    fn binding_for_fact(&self, kind: XPathFactKind) -> Option<&XPathDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathSyntaxAst {
    root: XPath,
}

impl XPathSyntaxAst {
    pub fn to_json(&self) -> Value {
        serde_json::to_value(&self.root).expect("XPath AST must serialize")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathExpressionAst {
    pub source: XPathExpressionSource,
    pub source_text: Option<String>,
    pub tokens: Vec<XPathTokenAst>,
    pub events: Vec<XPathAstEvent>,
    pub syntax_ast: Option<XPathSyntaxAst>,
    pub facts: Vec<XPathFact>,
    pub attachment: XPathAttachment,
    pub line_ending: Option<String>,
}

impl XPathExpressionAst {
    pub fn to_cemt_subject(&self) -> Value {
        let source_id = self.attachment.source_id();
        json!({
            "kind": "xpath-expression",
            "contentType": self.source.media_type,
            "schema": XPATH_SCHEMA_URI,
            "category": "xpath-expression",
            "grammarVersion": "xpath-3.1/xee-0.1.4",
            "source": {
                "uri": self.source.uri,
                "contentType": self.source.content_type,
                "mediaType": self.source.media_type,
                "parameters": self.source.parameters,
                "byteLength": self.source.byte_length,
            },
            "sourceText": self.source_text,
            "tokens": self.tokens.iter().map(|token| json!({
                "index": token.index,
                "kind": token.kind.as_str(),
                "lexeme": token.lexeme,
                "depth": token.depth,
                "role": token.kind.role(),
                "sourceRange": token.source_range.to_cemt_subject(),
                "sourceMap": token.source_range.source_map(source_id, &self.source.media_type),
            })).collect::<Vec<_>>(),
            "events": self.events.iter().map(|event| json!({
                "index": event.index,
                "kind": event.kind.as_str(),
                "tokenIndex": event.token_index,
                "depth": event.depth,
                "sourceRange": event.source_range.to_cemt_subject(),
                "sourceMap": event.source_range.source_map(source_id, &self.source.media_type),
            })).collect::<Vec<_>>(),
            "syntaxAst": self.syntax_ast.as_ref().map(XPathSyntaxAst::to_json),
            "parseFacts": self.facts.iter().map(|fact| json!({
                "kind": fact.kind.as_str(),
                "sourceRange": fact.source_range.map(XPathSourceRange::to_cemt_subject),
                "message": fact.message,
                "value": fact.value,
            })).collect::<Vec<_>>(),
            "attachment": xpath_attachment_to_cemt_subject(&self.attachment),
            "lineEnding": self.line_ending,
        })
    }
}

pub fn validate_xpath_source_bytes(request: XPathSourceRequest<'_>) -> Vec<Diagnostic> {
    let ast = xpath_expression_ast_from_source_bytes(
        request,
        XPathAttachment::Standalone { source_id: 1 },
    );
    validate_xpath_expression_ast(&ast, XPathSchemaContractCatalog::from_builtin())
}

pub fn validate_xpath_expression_ast(
    ast: &XPathExpressionAst,
    contracts: &XPathSchemaContractCatalog,
) -> Vec<Diagnostic> {
    let source_id = ast.attachment.source_id();
    ast.facts
        .iter()
        .filter_map(|fact| {
            let binding = contracts.binding_for_fact(fact.kind)?;
            let (line, column, byte_offset) = fact
                .source_range
                .map(|range| {
                    (
                        Some(range.start.line),
                        Some(range.start.column),
                        Some(range.start.byte_offset),
                    )
                })
                .unwrap_or((None, None, None));
            Some(Diagnostic {
                uri: Some(ast.source.uri.clone()),
                line,
                column,
                byte_offset,
                code: binding.diagnostic_code.clone(),
                severity: binding.severity,
                message: fact.message.clone(),
                details: Some(json!({
                    "xpath": {
                        "phase": "parse",
                        "factKind": fact.kind.as_str(),
                        "contract": binding.contract,
                        "behavior": binding.behavior,
                        "policy": binding.policy,
                        "schema": XPATH_SCHEMA_URI,
                        "schemaPackage": XPATH_PACKAGE_ID,
                        "contentType": ast.source.content_type,
                        "mediaType": ast.source.media_type,
                        "byteLength": fact.source_range.map(|range| range.byte_length),
                        "value": fact.value,
                        "attachmentKind": match ast.attachment {
                            XPathAttachment::Standalone { .. } => "standalone",
                            XPathAttachment::Host(_) => "host",
                        },
                    }
                })),
                source_map: fact
                    .source_range
                    .map(|range| range.source_map(source_id, ast.source.media_type.as_str())),
                ..Diagnostic::default()
            })
        })
        .collect()
}

pub fn xpath_expression_ast_from_source_bytes(
    request: XPathSourceRequest<'_>,
    attachment: XPathAttachment,
) -> XPathExpressionAst {
    let content_type = request.content_type.unwrap_or(XPATH_CONTENT_TYPE);
    let source = XPathExpressionSource {
        uri: request.source_uri.to_owned(),
        content_type: content_type.to_owned(),
        media_type: content_type_essence(content_type),
        parameters: content_type_parameters(request.content_type),
        byte_length: request.bytes.len(),
    };
    let origin = attachment.expression_origin();
    let source_text = match std::str::from_utf8(request.bytes) {
        Ok(source_text) => source_text,
        Err(error) => {
            let line_index = LineIndex::from_bytes_lossy(request.bytes);
            let start = error.valid_up_to();
            let end = start.saturating_add(error.error_len().unwrap_or(1));
            return XPathExpressionAst {
                source,
                source_text: None,
                tokens: Vec::new(),
                events: Vec::new(),
                syntax_ast: None,
                facts: vec![XPathFact {
                    kind: XPathFactKind::InvalidUtf8,
                    source_range: Some(XPathSourceRange::from_offsets(
                        &line_index,
                        origin,
                        start,
                        end.min(request.bytes.len()),
                    )),
                    message: format!("XPath source must be valid UTF-8: {error}"),
                    value: Some(start.to_string()),
                }],
                attachment,
                line_ending: detect_line_ending_style_bytes(request.bytes).map(str::to_owned),
            };
        }
    };

    let line_index = LineIndex::from_utf8(source_text);
    let mut tokens = xpath_lossless_tokens(source_text, &line_index, origin);
    let mut facts = xpath_delimiter_facts(&mut tokens);
    if let XPathAttachment::Host(host) = &attachment {
        facts.push(XPathFact {
            kind: XPathFactKind::HostAssociationObserved,
            source_range: Some(XPathSourceRange::from_offsets(
                &line_index,
                origin,
                0,
                source_text.len(),
            )),
            message: "XPath expression is associated with a host AST node".to_owned(),
            value: Some("host".to_owned()),
        });
        facts.extend(xpath_host_attachment_facts(request, host));
    }
    facts.extend(xpath_external_resource_facts(&tokens, &attachment));

    let has_lexical_error = tokens
        .iter()
        .any(|token| token.kind == XPathTokenKind::Error);
    for token in tokens
        .iter()
        .filter(|token| token.kind == XPathTokenKind::Error)
    {
        facts.push(XPathFact {
            kind: XPathFactKind::LexicalError,
            source_range: Some(token.source_range),
            message: format!("XPath lexical error at `{}`", token.lexeme),
            value: Some(token.lexeme.clone()),
        });
    }

    let parser = xpath_parser_context(&attachment);
    let syntax_ast = if has_lexical_error {
        None
    } else {
        match parser.parse_xpath(source_text) {
            Ok(root) => {
                facts.push(XPathFact {
                    kind: XPathFactKind::Parsed,
                    source_range: Some(XPathSourceRange::from_offsets(
                        &line_index,
                        origin,
                        0,
                        source_text.len(),
                    )),
                    message: "XPath 3.1 expression parsed successfully".to_owned(),
                    value: Some("xpath-3.1".to_owned()),
                });
                Some(XPathSyntaxAst { root })
            }
            Err(error) => {
                let span = error.span();
                let start = span.start.min(source_text.len());
                let end = span.end.min(source_text.len()).max(start);
                let kind = if matches!(error, ParserError::UnknownPrefix { .. }) {
                    XPathFactKind::UnknownNamespacePrefix
                } else {
                    XPathFactKind::ParseError
                };
                facts.push(XPathFact {
                    kind,
                    source_range: Some(XPathSourceRange::from_offsets(
                        &line_index,
                        origin,
                        start,
                        end,
                    )),
                    message: xpath_parser_error_message(&error),
                    value: Some(format!("{error:?}")),
                });
                None
            }
        }
    };

    let events = xpath_ast_events(&tokens, &line_index, origin, source_text.len());
    facts.extend(xpath_stream_invariant_facts(&tokens, &events));
    XPathExpressionAst {
        source,
        source_text: Some(source_text.to_owned()),
        tokens,
        events,
        syntax_ast,
        facts,
        attachment,
        line_ending: detect_line_ending_style_bytes(request.bytes).map(str::to_owned),
    }
}

fn xpath_host_attachment_facts(
    request: XPathSourceRequest<'_>,
    host: &XPathHostAttachment,
) -> Vec<XPathFact> {
    let expression_start = host.expression_range.start.byte_offset;
    let expression_end = expression_start.saturating_add(host.expression_range.byte_length);
    let owner_start = host.owner.source_range.start.byte_offset;
    let owner_end = owner_start.saturating_add(host.owner.source_range.byte_length);
    let mut failures = Vec::new();
    if host.expression_range.byte_length != request.bytes.len() as u64 {
        failures.push(format!(
            "expression range length {} does not match {} source bytes",
            host.expression_range.byte_length,
            request.bytes.len()
        ));
    }
    if expression_start < owner_start || expression_end > owner_end {
        failures.push("expression range is outside the owning host node range".to_owned());
    }
    if host.owner.source_uri != request.source_uri {
        failures.push(format!(
            "owner source URI `{}` does not match expression source URI `{}`",
            host.owner.source_uri, request.source_uri
        ));
    }
    if failures.is_empty() {
        Vec::new()
    } else {
        vec![XPathFact {
            kind: XPathFactKind::HostAssociationInvalid,
            source_range: Some(host.expression_range),
            message: format!("Invalid XPath host association: {}", failures.join("; ")),
            value: host.owner.node_id.clone(),
        }]
    }
}

fn xpath_external_resource_facts(
    tokens: &[XPathTokenAst],
    attachment: &XPathAttachment,
) -> Vec<XPathFact> {
    let resolver_policy_present = matches!(
        attachment,
        XPathAttachment::Host(XPathHostAttachment {
            resolver_policy_stamp: Some(stamp),
            ..
        }) if !stamp.trim().is_empty()
    );
    if resolver_policy_present {
        return Vec::new();
    }

    let significant = tokens
        .iter()
        .filter(|token| {
            !matches!(
                token.kind,
                XPathTokenKind::Whitespace | XPathTokenKind::Comment
            )
        })
        .collect::<Vec<_>>();
    significant
        .windows(2)
        .filter_map(|pair| {
            let name = pair[0].lexeme.rsplit(':').next().unwrap_or_default();
            let is_external_function = matches!(
                name,
                "collection"
                    | "doc"
                    | "json-doc"
                    | "unparsed-text"
                    | "unparsed-text-available"
                    | "unparsed-text-lines"
                    | "uri-collection"
            );
            (is_external_function && pair[1].lexeme == "(").then(|| XPathFact {
                kind: XPathFactKind::ExternalResourceDenied,
                source_range: Some(pair[0].source_range),
                message: format!(
                    "XPath function `{}` requires an explicit resolver policy",
                    pair[0].lexeme
                ),
                value: Some(pair[0].lexeme.clone()),
            })
        })
        .collect()
}

fn xpath_stream_invariant_facts(
    tokens: &[XPathTokenAst],
    events: &[XPathAstEvent],
) -> Vec<XPathFact> {
    let mut facts = Vec::new();
    if tokens
        .iter()
        .any(|token| token.source_range.byte_length == 0)
    {
        facts.push(XPathFact {
            kind: XPathFactKind::SourceMapUnavailable,
            source_range: None,
            message: "XPath token stream contains a token without an exact source range".to_owned(),
            value: None,
        });
    }
    let lifecycle_valid = events.len() == tokens.len() + 2
        && events
            .first()
            .is_some_and(|event| event.kind == XPathAstEventKind::StartExpression)
        && events
            .last()
            .is_some_and(|event| event.kind == XPathAstEventKind::EndExpression)
        && events[1..events.len().saturating_sub(1)]
            .iter()
            .zip(tokens)
            .all(|(event, token)| {
                event.kind == XPathAstEventKind::Token
                    && event.token_index == Some(token.index)
                    && event.source_range == token.source_range
            });
    if !lifecycle_valid {
        facts.push(XPathFact {
            kind: XPathFactKind::EventLifecycleInvalid,
            source_range: None,
            message: "XPath AST event lifecycle does not match the lossless token stream"
                .to_owned(),
            value: None,
        });
    }
    facts
}

fn xpath_ast_events(
    tokens: &[XPathTokenAst],
    line_index: &LineIndex,
    origin: XPathSourcePosition,
    source_len: usize,
) -> Vec<XPathAstEvent> {
    let mut events = Vec::with_capacity(tokens.len() + 2);
    events.push(XPathAstEvent {
        index: 0,
        kind: XPathAstEventKind::StartExpression,
        token_index: None,
        depth: 0,
        source_range: XPathSourceRange::from_offsets(line_index, origin, 0, 0),
    });
    events.extend(tokens.iter().map(|token| XPathAstEvent {
        index: token.index + 1,
        kind: XPathAstEventKind::Token,
        token_index: Some(token.index),
        depth: token.depth,
        source_range: token.source_range,
    }));
    events.push(XPathAstEvent {
        index: tokens.len() + 1,
        kind: XPathAstEventKind::EndExpression,
        token_index: None,
        depth: 0,
        source_range: XPathSourceRange::from_offsets(line_index, origin, source_len, source_len),
    });
    events
}

fn xpath_lossless_tokens(
    source: &str,
    line_index: &LineIndex,
    origin: XPathSourcePosition,
) -> Vec<XPathTokenAst> {
    let mut tokens = Vec::new();
    let mut cursor = 0usize;
    for (token, span) in xee_xpath_lexer::lexer(source) {
        if cursor < span.start {
            xpath_push_trivia_tokens(source, line_index, origin, cursor, span.start, &mut tokens);
        }
        xpath_push_token(
            source,
            line_index,
            origin,
            span.start,
            span.end,
            xpath_token_kind(&token),
            &mut tokens,
        );
        cursor = span.end;
    }
    if cursor < source.len() {
        xpath_push_trivia_tokens(
            source,
            line_index,
            origin,
            cursor,
            source.len(),
            &mut tokens,
        );
    }
    tokens
}

fn xpath_push_trivia_tokens(
    source: &str,
    line_index: &LineIndex,
    origin: XPathSourcePosition,
    start: usize,
    end: usize,
    tokens: &mut Vec<XPathTokenAst>,
) {
    let mut cursor = start;
    while cursor < end {
        let rest = &source[cursor..end];
        if rest.starts_with("(:") {
            let comment_end = xpath_nested_comment_end(source, cursor, end).unwrap_or(end);
            xpath_push_token(
                source,
                line_index,
                origin,
                cursor,
                comment_end,
                XPathTokenKind::Comment,
                tokens,
            );
            cursor = comment_end;
            continue;
        }
        if rest.as_bytes()[0].is_ascii_whitespace() {
            let whitespace_end = cursor
                + rest
                    .as_bytes()
                    .iter()
                    .take_while(|byte| byte.is_ascii_whitespace())
                    .count();
            xpath_push_token(
                source,
                line_index,
                origin,
                cursor,
                whitespace_end,
                XPathTokenKind::Whitespace,
                tokens,
            );
            cursor = whitespace_end;
            continue;
        }
        let raw_end = cursor + rest.chars().next().map(char::len_utf8).unwrap_or(1);
        xpath_push_token(
            source,
            line_index,
            origin,
            cursor,
            raw_end,
            XPathTokenKind::Error,
            tokens,
        );
        cursor = raw_end;
    }
}

fn xpath_nested_comment_end(source: &str, start: usize, limit: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut cursor = start + 2;
    let mut depth = 1usize;
    while cursor + 1 < limit {
        match &bytes[cursor..cursor + 2] {
            b"(:" => {
                depth += 1;
                cursor += 2;
            }
            b":)" => {
                depth -= 1;
                cursor += 2;
                if depth == 0 {
                    return Some(cursor);
                }
            }
            _ => cursor += 1,
        }
    }
    None
}

fn xpath_push_token(
    source: &str,
    line_index: &LineIndex,
    origin: XPathSourcePosition,
    start: usize,
    end: usize,
    kind: XPathTokenKind,
    tokens: &mut Vec<XPathTokenAst>,
) {
    if start >= end || end > source.len() {
        return;
    }
    tokens.push(XPathTokenAst {
        index: tokens.len(),
        kind,
        lexeme: source[start..end].to_owned(),
        depth: 0,
        source_range: XPathSourceRange::from_offsets(line_index, origin, start, end),
    });
}

fn xpath_token_kind(token: &XeeToken<'_>) -> XPathTokenKind {
    use XeeToken::*;
    match token {
        Error => XPathTokenKind::Error,
        IntegerLiteral(_) | DecimalLiteral(_) | DoubleLiteral(_) => XPathTokenKind::Number,
        StringLiteral(_) => XPathTokenKind::String,
        PrefixedQName(_)
        | URIQualifiedName(_)
        | LocalNameWildcard(_)
        | PrefixWildcard(_)
        | BracedURILiteralWildcard(_)
        | NCName(_)
        | BracedURILiteral(_) => XPathTokenKind::Name,
        Dollar => XPathTokenKind::VariableSigil,
        Whitespace => XPathTokenKind::Whitespace,
        CommentStart => XPathTokenKind::Comment,
        ExclamationMark | NotEqual | Asterisk | Plus | Minus | Slash | DoubleSlash | LessThan
        | Precedes | LessThanEqual | Equal | Arrow | GreaterThan | GreaterThanEqual | Follows
        | Pipe | DoublePipe | ColonEqual | And | Or | Div | Idiv | Mod | Eq | Ne | Lt | Le | Gt
        | Ge | Is | To | Union | Intersect | Except => XPathTokenKind::Operator,
        Ancestor
        | AncestorOrSelf
        | Array
        | As
        | Attribute
        | Cast
        | Castable
        | Child
        | Comment
        | Descendant
        | DescendantOrSelf
        | DocumentNode
        | Element
        | Else
        | EmptySequence
        | Every
        | Following
        | FollowingSibling
        | For
        | Function
        | If
        | In
        | Instance
        | Item
        | Let
        | Map
        | Namespace
        | NamespaceNode
        | Node
        | Of
        | Parent
        | Preceding
        | PrecedingSibling
        | ProcessingInstruction
        | Return
        | Satisfies
        | SchemaAttribute
        | SchemaElement
        | Self_
        | Some
        | Text
        | Then
        | Treat
        | Switch
        | Typeswitch => XPathTokenKind::Keyword,
        _ => XPathTokenKind::Punctuation,
    }
}

fn xpath_delimiter_facts(tokens: &mut [XPathTokenAst]) -> Vec<XPathFact> {
    let mut stack = Vec::<(char, XPathSourceRange)>::new();
    let mut facts = Vec::new();
    for token in tokens {
        let delimiter = match token.lexeme.as_str() {
            "(" | "[" | "{" | ")" | "]" | "}" => token.lexeme.chars().next(),
            _ => None,
        };
        match delimiter {
            Some(close @ (')' | ']' | '}')) => {
                let expected = matching_open_delimiter(close);
                if stack.last().is_some_and(|(open, _)| *open == expected) {
                    stack.pop();
                } else {
                    facts.push(XPathFact {
                        kind: XPathFactKind::MismatchedDelimiter,
                        source_range: Some(token.source_range),
                        message: format!(
                            "XPath closing delimiter `{close}` has no matching `{expected}`"
                        ),
                        value: Some(close.to_string()),
                    });
                }
                token.depth = stack.len();
            }
            Some(open @ ('(' | '[' | '{')) => {
                token.depth = stack.len();
                stack.push((open, token.source_range));
            }
            _ => token.depth = stack.len(),
        }
    }
    for (open, source_range) in stack.into_iter().rev() {
        facts.push(XPathFact {
            kind: XPathFactKind::UnclosedDelimiter,
            source_range: Some(source_range),
            message: format!("XPath opening delimiter `{open}` is not closed"),
            value: Some(open.to_string()),
        });
    }
    facts
}

fn matching_open_delimiter(close: char) -> char {
    match close {
        ')' => '(',
        ']' => '[',
        '}' => '{',
        _ => close,
    }
}

fn xpath_parser_context(attachment: &XPathAttachment) -> XPathParserContext {
    let mut namespaces = Namespaces::default();
    if let XPathAttachment::Host(host) = attachment {
        if let Some(namespace) = &host.static_context.default_element_namespace {
            namespaces.default_element_namespace = namespace.clone();
        }
        if let Some(namespace) = &host.static_context.default_function_namespace {
            namespaces.default_function_namespace = namespace.clone();
        }
        let pairs = host
            .static_context
            .namespaces
            .iter()
            .map(|(prefix, namespace)| (prefix.as_str(), namespace.as_str()))
            .collect::<Vec<_>>();
        namespaces.add(&pairs);
    }
    XPathParserContext::new(namespaces, VariableNames::default())
}

fn xpath_parser_error_message(error: &ParserError) -> String {
    match error {
        ParserError::UnknownPrefix { prefix, .. } => {
            format!("XPath namespace prefix `{prefix}` is not declared in the static context")
        }
        ParserError::Reserved { name, .. } => {
            format!("XPath name `{name}` is reserved in this grammar position")
        }
        ParserError::ArityOverflow { .. } => {
            "XPath function arity exceeds the supported representation".to_owned()
        }
        ParserError::UnknownType { name, .. } => {
            format!("XPath sequence type `{name:?}` is unknown")
        }
        ParserError::IllegalFunctionInPattern { name, .. } => {
            format!("XPath function `{name:?}` is not legal in this pattern")
        }
        ParserError::ExpectedFound { .. } => {
            "XPath 3.1 expression did not match the grammar".to_owned()
        }
    }
}

fn xpath_attachment_to_cemt_subject(attachment: &XPathAttachment) -> Value {
    match attachment {
        XPathAttachment::Standalone { source_id } => json!({
            "kind": "standalone",
            "sourceId": source_id,
        }),
        XPathAttachment::Host(host) => json!({
            "kind": "host",
            "owner": {
                "sourceId": host.owner.source_id,
                "sourceUri": host.owner.source_uri,
                "contentType": host.owner.content_type,
                "schema": host.owner.schema_uri,
                "nodeKind": host.owner.node_kind.as_str(),
                "nodeId": host.owner.node_id,
                "sourceRange": host.owner.source_range.to_cemt_subject(),
            },
            "expressionRange": host.expression_range.to_cemt_subject(),
            "staticContext": {
                "namespaces": host.static_context.namespaces,
                "defaultElementNamespace": host.static_context.default_element_namespace,
                "defaultFunctionNamespace": host.static_context.default_function_namespace,
                "variableBindings": host.static_context.variable_bindings,
                "functionBindings": host.static_context.function_bindings,
            },
            "expectedResult": host.expected_result.as_ref().map(|result| json!({
                "sequenceType": result.sequence_type,
                "minItems": result.min_items,
                "maxItems": result.max_items,
            })),
            "evaluationPhase": host.evaluation_phase.as_str(),
            "resolverPolicyStamp": host.resolver_policy_stamp,
            "safetyPolicyStamp": host.safety_policy_stamp,
        }),
    }
}

fn content_type_parameters(content_type: Option<&str>) -> BTreeMap<String, String> {
    let Some(content_type) = content_type else {
        return BTreeMap::new();
    };
    content_type
        .split(';')
        .skip(1)
        .filter_map(|part| {
            let (name, value) = part.split_once('=')?;
            Some((
                name.trim().to_ascii_lowercase(),
                value.trim().trim_matches('"').to_owned(),
            ))
        })
        .collect()
}

fn detect_line_ending_style_bytes(source: &[u8]) -> Option<&'static str> {
    let has_crlf = source.windows(2).any(|pair| pair == b"\r\n");
    let has_lone_cr = source
        .iter()
        .enumerate()
        .any(|(index, byte)| *byte == b'\r' && source.get(index + 1).copied() != Some(b'\n'));
    let has_lf = source.iter().enumerate().any(|(index, byte)| {
        *byte == b'\n'
            && index
                .checked_sub(1)
                .and_then(|previous| source.get(previous))
                != Some(&b'\r')
    });
    match (has_crlf, has_lf, has_lone_cr) {
        (false, false, false) => None,
        (true, false, false) => Some("crlf"),
        (false, true, false) => Some("lf"),
        _ => Some("mixed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_cem_contract(source: &str) -> crate::parser::document::CemDocument {
        let source = crate::source::BytesSource::new(SourceId(91), source.as_bytes().to_vec());
        let tokenizer = crate::tokenizer::cem::CemTokenizer::from_source(source);
        let events = crate::events::cem::CemEventNormalizer::new(tokenizer);
        crate::parser::builder::CemAstBuilder::new(events).build()
    }

    fn contract_element_ids(
        document: &crate::parser::document::CemDocument,
        local_name: &str,
    ) -> Vec<crate::parser::AstNodeId> {
        document
            .iter()
            .filter_map(|node| match node {
                crate::parser::CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == local_name => Some(*node_id),
                _ => None,
            })
            .collect()
    }

    fn contract_attributes(
        document: &crate::parser::document::CemDocument,
        node_id: crate::parser::AstNodeId,
    ) -> BTreeMap<String, String> {
        let Some(crate::parser::CemAstNode::Element { attributes, .. }) = document.get(node_id)
        else {
            return BTreeMap::new();
        };
        attributes
            .iter()
            .filter_map(|attribute_id| match document.get(*attribute_id) {
                Some(crate::parser::CemAstNode::Attribute {
                    expanded_name,
                    value,
                    ..
                }) => value
                    .as_ref()
                    .map(|value| (expanded_name.local_name.clone(), value.clone())),
                _ => None,
            })
            .collect()
    }

    fn result_source_map(source_id: u32, byte_offset: u64) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(source_id),
                span: FrameSpan::Single(ByteRange::new(byte_offset, 1)),
                transform: TransformKind::Query,
            }],
        }
    }

    fn parse(source: &str) -> XPathExpressionAst {
        xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: source.as_bytes(),
                source_uri: "memory://expression.xpath",
                content_type: Some(XPATH_CONTENT_TYPE),
            },
            XPathAttachment::Standalone { source_id: 7 },
        )
    }

    #[test]
    fn xpath_result_artifact_preserves_ordered_mixed_sequences_and_item_origins() {
        let capabilities = XPathEvaluatorCapabilities::required("test.xpath", "1.0.0");
        let sequence = XPathResultSequence {
            sequence_type: "item()*".to_owned(),
            items: vec![
                XPathResultItem::Node {
                    node_kind: XPathResultNodeKind::Element,
                    source_id: 9,
                    source_uri: "memory://catalog.xml".to_owned(),
                    node_id: "node:12".to_owned(),
                    expanded_name: Some("{urn:catalog}book".to_owned()),
                    source_range: Some(XPathSourceRange::new(2, 3, 18, 24)),
                    source_map: result_source_map(9, 18),
                },
                XPathResultItem::Atomic {
                    value: XPathAtomicValue {
                        type_name: "xs:decimal".to_owned(),
                        lexical_value: "42.50".to_owned(),
                        namespace_uri: None,
                        local_name: None,
                    },
                    source_map: result_source_map(7, 4),
                },
                XPathResultItem::Map {
                    entries: vec![XPathMapEntry {
                        key: XPathAtomicValue {
                            type_name: "xs:string".to_owned(),
                            lexical_value: "title".to_owned(),
                            namespace_uri: None,
                            local_name: None,
                        },
                        value: XPathResultSequence {
                            sequence_type: "xs:string".to_owned(),
                            items: vec![XPathResultItem::Atomic {
                                value: XPathAtomicValue {
                                    type_name: "xs:string".to_owned(),
                                    lexical_value: "CEM".to_owned(),
                                    namespace_uri: None,
                                    local_name: None,
                                },
                                source_map: result_source_map(7, 8),
                            }],
                        },
                    }],
                    source_map: result_source_map(7, 6),
                },
                XPathResultItem::Array {
                    members: vec![XPathResultSequence {
                        sequence_type: "xs:boolean".to_owned(),
                        items: vec![XPathResultItem::Atomic {
                            value: XPathAtomicValue {
                                type_name: "xs:boolean".to_owned(),
                                lexical_value: "true".to_owned(),
                                namespace_uri: None,
                                local_name: None,
                            },
                            source_map: result_source_map(7, 10),
                        }],
                    }],
                    source_map: result_source_map(7, 9),
                },
                XPathResultItem::Function {
                    evaluator_id: "test.xpath".to_owned(),
                    function_id: "function:5".to_owned(),
                    name: Some("fn:string".to_owned()),
                    arity: 1,
                    signature: "function(item()?) as xs:string".to_owned(),
                    source_map: result_source_map(7, 12),
                },
            ],
        };
        let artifact = XPathResultArtifact {
            content_type: XPATH_RESULT_CONTENT_TYPE.to_owned(),
            schema_uri: XPATH_SCHEMA_URI.to_owned(),
            xpath_version: "3.1".to_owned(),
            grammar_version: XPATH_GRAMMAR_VERSION.to_owned(),
            evaluator: XPathEvaluatorIdentity {
                evaluator_id: "test.xpath".to_owned(),
                evaluator_version: "1.0.0".to_owned(),
            },
            expression_uri: "memory://query.xpath".to_owned(),
            static_context: XPathStaticContext::default(),
            resolver_policy_stamp: "resolver-policy/1;test".to_owned(),
            safety_policy_stamp: "xpath-safety/1;pure".to_owned(),
            expected_result: Some(XPathExpectedResult {
                sequence_type: "item()*".to_owned(),
                min_items: Some(0),
                max_items: None,
            }),
            sequence,
            source_map: result_source_map(7, 0),
        };

        assert!(
            validate_xpath_result_artifact(&artifact, &capabilities).is_empty(),
            "valid mixed sequence must satisfy the package contract"
        );
        let value = serde_json::to_value(&artifact).expect("result artifact serializes");
        assert_eq!(value["contentType"], XPATH_RESULT_CONTENT_TYPE);
        assert_eq!(value["evaluator"]["evaluatorId"], "test.xpath");
        assert_eq!(value["evaluator"]["evaluatorVersion"], "1.0.0");
        assert_eq!(value["sequence"]["items"][0]["kind"], "node");
        assert_eq!(value["sequence"]["items"][1]["kind"], "atomic");
        assert_eq!(value["sequence"]["items"][2]["kind"], "map");
        assert_eq!(value["sequence"]["items"][3]["kind"], "array");
        assert_eq!(value["sequence"]["items"][4]["kind"], "function");
        assert_eq!(
            value["sequence"]["items"][0]["sourceMap"]["frames"][0]["source_id"],
            9
        );
        assert_eq!(value["sequence"]["items"][4]["functionId"], "function:5");
        assert!(value["sequence"]["items"][4].get("closure").is_none());

        let mut invalid = artifact.clone();
        if let XPathResultItem::Atomic { source_map, .. } = &mut invalid.sequence.items[1] {
            source_map.frames.clear();
        }
        if let XPathResultItem::Function { evaluator_id, .. } = &mut invalid.sequence.items[4] {
            *evaluator_id = "other.xpath".to_owned();
        }
        let violations = validate_xpath_result_artifact(&invalid, &capabilities);
        assert!(violations
            .iter()
            .any(|violation| violation.code == "xpath-result-source-map-required"));
        assert!(violations
            .iter()
            .any(|violation| violation.code == "xpath-result-function-scope-invalid"));
    }

    #[test]
    fn xpath_evaluator_capabilities_forbid_reparse_resolver_bypass_and_missing_wasm() {
        let required = XPathEvaluatorCapabilities::required("test.xpath", "1.0.0");
        assert!(validate_xpath_evaluator_capabilities(&required).is_empty());

        let mut incompatible = required;
        incompatible.ast_input = XPathEvaluatorAstInput::SourceText;
        incompatible.resource_access = XPathEvaluatorResourceAccess::Direct;
        incompatible.targets.remove("wasm32-unknown-unknown");
        let violations = validate_xpath_evaluator_capabilities(&incompatible);

        for code in [
            "xpath-evaluator-ast-reparse-forbidden",
            "xpath-evaluator-resolver-bypass",
            "xpath-evaluator-target-missing",
        ] {
            assert!(
                violations.iter().any(|violation| violation.code == code),
                "missing capability violation {code}: {violations:?}"
            );
        }
    }

    #[test]
    fn xpath_evaluation_request_carries_package_ast_and_cem_resolver_boundary() {
        let expression = parse("/catalog/book");
        let resolver_registry = ResolverRegistry::new();
        let resolver_policy = ResolverPolicy::new();
        let request = XPathEvaluationRequest {
            expression: &expression,
            context_item: None,
            variable_bindings: BTreeMap::new(),
            static_context: XPathStaticContext::default(),
            expected_result: None,
            resolver_registry: &resolver_registry,
            resolver_policy: &resolver_policy,
            safety_policy_stamp: "xpath-safety/1;pure",
        };

        assert!(request.expression.syntax_ast.is_some());
        assert_eq!(
            request.resolver_policy.cache_stamp(),
            resolver_policy.cache_stamp()
        );
        assert!(std::ptr::eq(request.resolver_registry, &resolver_registry));
    }

    #[test]
    fn xpath_schema_declares_evaluation_result_and_capability_contracts() {
        let source = builtin_schema_package_source(XPATH_PACKAGE_ID)
            .expect("XPath package source")
            .schema_source;
        let model = compile_schema_document_model(XPATH_SCHEMA_URI, source);

        for element in [
            "evaluation-request",
            "evaluator-capabilities",
            "result-artifact",
            "sequence",
            "node-item",
            "atomic-item",
            "map-item",
            "array-item",
            "function-item",
            "source-map-frame",
            "source-range",
        ] {
            assert!(
                model.elements.contains_key(element),
                "XPath schema must own `{element}`"
            );
        }
        let result = model.elements.get("result-artifact").unwrap();
        assert!(result.required_attributes.contains("content-type"));
        assert!(result.required_attributes.contains("evaluator-id"));
        assert!(result.required_attributes.contains("resolver-policy-stamp"));
        assert!(result.required_attributes.contains("safety-policy-stamp"));
        assert!(result.child_elements.contains("sequence"));
        for contract in [
            "xpath-evaluator-package-ast",
            "xpath-evaluator-resource-access",
            "xpath-evaluator-runtime-targets",
            "xpath-result-item-order",
            "xpath-result-node-identity",
            "xpath-result-function-scope",
            "xpath-result-policy-stamps",
        ] {
            assert!(
                model.constraints.contains_key(contract),
                "XPath schema must own `{contract}`"
            );
        }
    }

    #[test]
    fn xpath_full_conformance_matrix_is_schema_owned_and_actionable() {
        let source = include_str!("../../schema-packages/xpath/v1/tests/xpath-3.1-conformance.cem");
        let document = parse_cem_contract(source);
        assert!(
            document.diagnostics.is_empty(),
            "conformance matrix must parse as CEM: {:?}",
            document.diagnostics
        );

        let model = compile_schema_document_model(
            XPATH_SCHEMA_URI,
            builtin_schema_package_source(XPATH_PACKAGE_ID)
                .expect("XPath package source")
                .schema_source,
        );
        let diagnostics = crate::schema::document_model::validate_document_model(&document, &model);
        assert!(
            diagnostics.is_empty(),
            "conformance matrix must satisfy the XPath schema: {diagnostics:?}"
        );

        let profiles = contract_element_ids(&document, "conformance-profile");
        assert_eq!(
            profiles.len(),
            1,
            "one XPath conformance profile is required"
        );
        let profile = contract_attributes(&document, profiles[0]);
        assert_eq!(
            profile.get("xpath-version").map(String::as_str),
            Some("3.1")
        );
        assert_eq!(profile.get("destination").map(String::as_str), Some("full"));
        assert_eq!(profile.get("delivery").map(String::as_str), Some("staged"));
        assert_eq!(profile.get("qt3-version").map(String::as_str), Some("3.1"));

        let references = contract_element_ids(&document, "normative-reference")
            .into_iter()
            .map(|node_id| contract_attributes(&document, node_id))
            .collect::<Vec<_>>();
        let reference_ids = references
            .iter()
            .filter_map(|reference| reference.get("id").cloned())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            reference_ids,
            BTreeSet::from([
                "fo31".to_owned(),
                "qt3-31".to_owned(),
                "xdm31".to_owned(),
                "xpath31".to_owned(),
            ])
        );
        assert!(references.iter().all(|reference| {
            reference
                .get("uri")
                .is_some_and(|uri| uri.starts_with("https://www.w3.org/"))
        }));

        let implementation_reference_ids =
            contract_element_ids(&document, "implementation-reference");
        assert_eq!(implementation_reference_ids.len(), 1);
        let implementation_reference =
            contract_attributes(&document, implementation_reference_ids[0]);
        assert_eq!(
            implementation_reference.get("usage").map(String::as_str),
            Some("reference-only")
        );
        assert_eq!(
            implementation_reference.get("commit").map(String::as_str),
            Some("200b1e3356ea9d6dd2901d67bd941b779df7e5b7")
        );

        let slices = contract_element_ids(&document, "conformance-slice")
            .into_iter()
            .map(|node_id| contract_attributes(&document, node_id))
            .collect::<Vec<_>>();
        assert!(
            slices.len() >= 10,
            "full XPath requires a complete slice inventory"
        );
        let slice_ids = slices
            .iter()
            .map(|slice| slice.get("id").expect("slice id").clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(slice_ids.len(), slices.len(), "slice ids must be unique");
        for required in [
            "syntax-and-static-context",
            "expressions-and-control-flow",
            "paths-and-node-tests",
            "operators",
            "type-system",
            "function-items",
            "maps-and-arrays",
            "functions-and-operators",
            "dynamic-context-and-resources",
            "schema-aware-evaluation",
            "xdm-results-and-serialization",
        ] {
            assert!(
                slice_ids.contains(required),
                "missing conformance slice `{required}`"
            );
        }
        for slice in &slices {
            let status = slice.get("status").expect("slice status");
            assert!(
                matches!(
                    status.as_str(),
                    "complete" | "transitional" | "partial" | "contract-only" | "planned"
                ),
                "unsupported slice status `{status}`"
            );
            if status != "complete" {
                assert!(
                    slice.get("gap").is_some_and(|gap| !gap.trim().is_empty()),
                    "non-complete slice requires a gap: {slice:?}"
                );
                assert!(
                    slice
                        .get("todo")
                        .is_some_and(|todo| !todo.trim().is_empty()),
                    "non-complete slice requires a todo reference: {slice:?}"
                );
            }
            assert!(
                slice.get("qt3-status").is_some(),
                "slice must declare QT3 mapping state: {slice:?}"
            );
        }
    }

    #[test]
    fn xpath_ast_is_lossless_for_unicode_names_nested_comments_and_xpath_31_structures() {
        let source = "for $\u{03c0} in /catalog/book[@lang = \"en\"] (: outer (: nested :) :) return map { \"title\": $\u{03c0}/title }";
        let ast = parse(source);

        assert_eq!(
            ast.tokens
                .iter()
                .map(|token| token.lexeme.as_str())
                .collect::<String>(),
            source
        );
        assert!(ast.syntax_ast.is_some());
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::Parsed));
        assert!(ast.tokens.iter().any(|token| {
            token.kind == XPathTokenKind::Comment && token.lexeme.contains("(: nested :)")
        }));
        assert!(ast.tokens.iter().any(|token| {
            token.kind == XPathTokenKind::Name
                && token.lexeme == "\u{03c0}"
                && token.source_range.byte_length == 2
        }));
        assert!(ast
            .tokens
            .windows(2)
            .all(
                |pair| pair[0].source_range.start.byte_offset + pair[0].source_range.byte_length
                    == pair[1].source_range.start.byte_offset
            ));
        assert_eq!(ast.events.len(), ast.tokens.len() + 2);
        assert_eq!(
            ast.events.first().map(|event| event.kind),
            Some(XPathAstEventKind::StartExpression)
        );
        assert_eq!(
            ast.events.last().map(|event| event.kind),
            Some(XPathAstEventKind::EndExpression)
        );
        assert!(ast.events[1..ast.events.len() - 1]
            .iter()
            .zip(&ast.tokens)
            .all(|(event, token)| {
                event.kind == XPathAstEventKind::Token
                    && event.token_index == Some(token.index)
                    && event.depth == token.depth
                    && event.source_range == token.source_range
            }));
    }

    #[test]
    fn xpath_ast_preserves_malformed_source_and_reports_parser_and_delimiter_facts() {
        let source = "/catalog/book[1";
        let ast = parse(source);

        assert_eq!(
            ast.tokens
                .iter()
                .map(|token| token.lexeme.as_str())
                .collect::<String>(),
            source
        );
        assert!(ast.syntax_ast.is_none());
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::ParseError));
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::UnclosedDelimiter));
    }

    #[test]
    fn xpath_ast_offsets_embedded_tokens_and_retains_owner_static_context() {
        let expression = "item/@id";
        let expression_range = XPathSourceRange::new(3, 15, 42, expression.len() as u64);
        let attachment = XPathAttachment::Host(XPathHostAttachment {
            owner: XPathHostOwner {
                source_id: 11,
                source_uri: "memory://stylesheet.xsl".to_owned(),
                content_type: Some("application/xslt+xml".to_owned()),
                schema_uri: Some("https://cem.dev/ns/transform/xslt/1".to_owned()),
                node_kind: XPathHostNodeKind::XsltAttribute,
                node_id: Some("event:4@select".to_owned()),
                source_range: XPathSourceRange::new(3, 7, 34, 19),
            },
            expression_range,
            static_context: XPathStaticContext {
                namespaces: BTreeMap::from([(
                    "app".to_owned(),
                    "https://example.test/app".to_owned(),
                )]),
                default_element_namespace: Some("https://example.test/app".to_owned()),
                variable_bindings: BTreeMap::from([("item".to_owned(), "element()".to_owned())]),
                ..XPathStaticContext::default()
            },
            expected_result: Some(XPathExpectedResult {
                sequence_type: "attribute()*".to_owned(),
                min_items: Some(0),
                max_items: None,
            }),
            evaluation_phase: XPathEvaluationPhase::Transform,
            resolver_policy_stamp: Some("resolver:none".to_owned()),
            safety_policy_stamp: Some("xpath:pure".to_owned()),
        });
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: expression.as_bytes(),
                source_uri: "memory://stylesheet.xsl",
                content_type: Some(XPATH_CONTENT_TYPE),
            },
            attachment,
        );

        assert_eq!(ast.tokens[0].source_range.start, expression_range.start);
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::HostAssociationObserved));
        let XPathAttachment::Host(host) = &ast.attachment else {
            panic!("expected host XPath attachment");
        };
        assert_eq!(host.owner.node_kind, XPathHostNodeKind::XsltAttribute);
        assert_eq!(
            host.static_context
                .namespaces
                .get("app")
                .map(String::as_str),
            Some("https://example.test/app")
        );
        assert_eq!(host.evaluation_phase, XPathEvaluationPhase::Transform);
        let subject = ast.to_cemt_subject();
        assert_eq!(subject["events"][0]["kind"], "start-expression");
        assert_eq!(subject["attachment"]["owner"]["nodeId"], "event:4@select");
        assert_eq!(subject["attachment"]["evaluationPhase"], "transform");
    }

    #[test]
    fn xpath_package_examples_match_declared_parse_expectations() {
        let passing = [
            (
                "basic-path",
                include_str!("../../schema-packages/xpath/v1/examples/basic-path.xpath"),
            ),
            (
                "functions-and-variables",
                include_str!(
                    "../../schema-packages/xpath/v1/examples/functions-and-variables.xpath"
                ),
            ),
            (
                "maps-arrays-and-comments",
                include_str!(
                    "../../schema-packages/xpath/v1/examples/maps-arrays-and-comments.xpath"
                ),
            ),
            (
                "unicode-qname",
                include_str!("../../schema-packages/xpath/v1/examples/unicode-qname.xpath"),
            ),
            (
                "explicit-axes-and-escaped-string",
                include_str!(
                    "../../schema-packages/xpath/v1/examples/explicit-axes-and-escaped-string.xpath"
                ),
            ),
        ];

        for (name, source) in passing {
            let ast = parse(source);
            assert_eq!(
                ast.tokens
                    .iter()
                    .map(|token| token.lexeme.as_str())
                    .collect::<String>(),
                source,
                "{name} must retain every source byte"
            );
            assert!(
                ast.syntax_ast.is_some(),
                "{name} must parse: {:?}",
                ast.facts
            );
            assert!(
                ast.facts
                    .iter()
                    .any(|fact| fact.kind == XPathFactKind::Parsed),
                "{name} must report its parsed fact"
            );
        }

        let invalid = parse(include_str!(
            "../../schema-packages/xpath/v1/examples/invalid-unclosed-predicate.xpath"
        ));
        assert!(invalid.syntax_ast.is_none());
        assert!(invalid
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::ParseError));
        assert!(invalid
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::UnclosedDelimiter));

        for (name, source, expected_facts) in [
            (
                "unknown-prefix",
                include_str!("../../schema-packages/xpath/v1/examples/unknown-prefix.xpath"),
                vec![XPathFactKind::UnknownNamespacePrefix],
            ),
            (
                "invalid-token",
                include_str!("../../schema-packages/xpath/v1/examples/invalid-token.xpath"),
                vec![XPathFactKind::LexicalError],
            ),
            (
                "mismatched-delimiter",
                include_str!("../../schema-packages/xpath/v1/examples/mismatched-delimiter.xpath"),
                vec![
                    XPathFactKind::ParseError,
                    XPathFactKind::MismatchedDelimiter,
                    XPathFactKind::UnclosedDelimiter,
                ],
            ),
        ] {
            let ast = parse(source);
            assert!(ast.syntax_ast.is_none(), "{name} must not parse");
            for expected in expected_facts {
                assert!(
                    ast.facts.iter().any(|fact| fact.kind == expected),
                    "{name} must report {expected:?}: {:?}",
                    ast.facts
                );
            }
        }
    }

    #[test]
    fn xpath_schema_contract_binds_reportable_facts_to_diagnostics() {
        let catalog = XPathSchemaContractCatalog::from_builtin();
        for (kind, code, severity) in [
            (
                XPathFactKind::InvalidUtf8,
                "cem.xpath.invalid_utf8",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::LexicalError,
                "cem.xpath.lexical_error",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::ParseError,
                "cem.xpath.parse_error",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::UnknownNamespacePrefix,
                "cem.xpath.unknown_namespace_prefix",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::UnclosedDelimiter,
                "cem.xpath.unclosed_delimiter",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::MismatchedDelimiter,
                "cem.xpath.mismatched_delimiter",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::HostAssociationInvalid,
                "cem.xpath.host_association_invalid",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::ExternalResourceDenied,
                "cem.xpath.external_resource_denied",
                crate::diagnostics::Severity::Error,
            ),
            (
                XPathFactKind::SourceMapUnavailable,
                "cem.xpath.source_map_unavailable",
                crate::diagnostics::Severity::Info,
            ),
            (
                XPathFactKind::EventLifecycleInvalid,
                "cem.xpath.event_lifecycle_invalid",
                crate::diagnostics::Severity::Error,
            ),
        ] {
            let binding = catalog
                .binding_for_fact(kind)
                .unwrap_or_else(|| panic!("schema binding for {}", kind.as_str()));
            assert_eq!(binding.diagnostic_code, code);
            assert_eq!(binding.severity, severity);
            assert_eq!(binding.behavior.as_deref(), Some("xpath-report-fact"));
        }
    }

    #[test]
    fn xpath_unknown_prefix_diagnostic_is_schema_declared() {
        let source = builtin_schema_package_source(XPATH_PACKAGE_ID)
            .expect("XPath package source")
            .schema_source
            .replace(
                r#"{constraint @kind="xpath-static-namespace" @target="static-context" @diagnostic="cem.xpath.unknown_namespace_prefix" @behavior="xpath-report-fact" @fact-kind="unknown-namespace-prefix" @policy="prefixed names resolve through the declared static context"}"#,
                r#"{constraint @kind="xpath-static-namespace" @target="static-context" @diagnostic="example.xpath.unknown_prefix" @behavior="xpath-report-fact" @fact-kind="unknown-namespace-prefix" @policy="prefixed names resolve through the declared static context"}"#,
            )
            .replace(
                r#"{diagnostic @code="cem.xpath.unknown_namespace_prefix" @severity="error"}"#,
                r#"{diagnostic @code="example.xpath.unknown_prefix" @severity="warning"}"#,
            );
        let contracts = XPathSchemaContractCatalog::from_schema_source(&source);
        let ast = parse("/catalog/ns:book");
        let diagnostics = validate_xpath_expression_ast(&ast, &contracts);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "example.xpath.unknown_prefix");
        assert_eq!(diagnostics[0].severity, Severity::Warning);
        assert_eq!(
            diagnostics[0].details.as_ref().unwrap()["xpath"]["contract"],
            "xpath-static-namespace"
        );
    }

    #[test]
    fn xpath_source_diagnostics_preserve_exact_ranges_maps_and_schema_details() {
        let source = "/catalog/ns:book";
        let diagnostics = validate_xpath_source_bytes(XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri: "memory://unknown-prefix.xpath",
            content_type: Some("text/xpath; charset=utf-8"),
        });
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "cem.xpath.unknown_namespace_prefix")
            .expect("unknown namespace diagnostic");

        assert_eq!(
            diagnostic.uri.as_deref(),
            Some("memory://unknown-prefix.xpath")
        );
        assert_eq!(diagnostic.line, Some(1));
        assert_eq!(diagnostic.column, Some(10));
        assert_eq!(diagnostic.byte_offset, Some(9));
        assert!(diagnostic.source_map.as_ref().is_some_and(|source_map| {
            source_map.frames.iter().any(|frame| {
                frame.source_id == SourceId(1)
                    && matches!(frame.span, FrameSpan::Single(range) if range.start == 9)
            })
        }));
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["xpath"]["factKind"],
            "unknown-namespace-prefix"
        );
        assert_eq!(
            diagnostic.details.as_ref().unwrap()["xpath"]["contract"],
            "xpath-static-namespace"
        );
    }

    #[test]
    fn xpath_validation_denies_external_resource_functions_without_resolver_policy() {
        let diagnostics = validate_xpath_source_bytes(XPathSourceRequest {
            bytes: b"doc(\"catalog.xml\")/catalog",
            source_uri: "memory://external.xpath",
            content_type: Some(XPATH_CONTENT_TYPE),
        });

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.xpath.external_resource_denied"
                && diagnostic.byte_offset == Some(0)
        }));
    }

    #[test]
    fn xpath_validation_reports_invalid_host_association_from_schema_binding() {
        let expression = "item/@id";
        let attachment = XPathAttachment::Host(XPathHostAttachment {
            owner: XPathHostOwner {
                source_id: 11,
                source_uri: "memory://stylesheet.xsl".to_owned(),
                content_type: Some("application/xslt+xml".to_owned()),
                schema_uri: Some("https://cem.dev/ns/transform/xslt/1".to_owned()),
                node_kind: XPathHostNodeKind::XsltAttribute,
                node_id: Some("event:4@select".to_owned()),
                source_range: XPathSourceRange::new(3, 7, 34, 19),
            },
            expression_range: XPathSourceRange::new(3, 15, 42, 2),
            static_context: XPathStaticContext::default(),
            expected_result: None,
            evaluation_phase: XPathEvaluationPhase::Transform,
            resolver_policy_stamp: None,
            safety_policy_stamp: None,
        });
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: expression.as_bytes(),
                source_uri: "memory://stylesheet.xsl",
                content_type: Some(XPATH_CONTENT_TYPE),
            },
            attachment,
        );
        let diagnostics =
            validate_xpath_expression_ast(&ast, &XPathSchemaContractCatalog::from_builtin());

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.xpath.host_association_invalid"
                && diagnostic.byte_offset == Some(42)
        }));
    }

    #[test]
    fn xpath_ast_reports_invalid_utf8_without_synthesizing_tokens() {
        let ast = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: &[b'/', 0xff, b'x'],
                source_uri: "memory://invalid.xpath",
                content_type: Some(XPATH_CONTENT_TYPE),
            },
            XPathAttachment::Standalone { source_id: 3 },
        );

        assert!(ast.source_text.is_none());
        assert!(ast.tokens.is_empty());
        assert!(ast.events.is_empty());
        assert!(ast.syntax_ast.is_none());
        assert!(ast
            .facts
            .iter()
            .any(|fact| fact.kind == XPathFactKind::InvalidUtf8));
    }
}
