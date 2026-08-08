use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use serde_json::json;

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::FormatIdentity;
use crate::lifecycle::LoadedInputAstStream;
use crate::query::{
    QueryAstOwner, QueryEvaluatorAdapter, QueryExecutionLimits, QueryExecutionRequest,
    QueryExecutionResult, QueryInputModel, QueryInputOwner, QueryLanguage, QueryNativeArtifact,
    QueryNativeResult, CSS_SELECTOR_LANGUAGE_VERSION, CSS_SELECTOR_RESULT_REPRESENTATION_ID,
};
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::package_sources::builtin_schema_package_source;
use crate::schema::registry::{
    CSS_SELECTOR_CONTENT_TYPE, CSS_SELECTOR_SCHEMA_URI, HTML_NAMESPACE_URI,
};
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use crate::validation::css::{
    css_syntax_lossless_events, CssEventAst, CssFactKind, CssSourceRange,
};
use crate::validation::html::{HtmlDocumentAst, HtmlEventAst, HtmlEventKind};
use crate::validation::xml::{XmlAttributeAst, XmlDocumentAst, XmlEventAst, XmlEventKind};

const CSS_SELECTOR_PACKAGE_ID: &str = "css-selector";
const CSS_SELECTOR_FACT_BEHAVIOR: &str = "css-selector-report-fact";
const CSS_SELECTOR_AST_REPRESENTATION_ID: &str = "cem.css-selector-expression-ast";
const ELEMENT_TREE_REPRESENTATION_ID: &str = "cem.lifecycle.element-tree";
const ELEMENT_TREE_INPUT_MODELS: &[QueryInputModel] = &[QueryInputModel::ElementTree];

#[derive(Debug, Clone, Copy)]
pub struct CssSelectorSourceRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
    pub namespace_bindings: &'a BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSelectorSource {
    pub uri: String,
    pub content_type: String,
    pub byte_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssSelectorSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CssSelectorSourceRange {
    pub start: CssSelectorSourcePosition,
    pub byte_length: u64,
}

impl CssSelectorSourceRange {
    fn from_css(range: CssSourceRange) -> Self {
        Self {
            start: CssSelectorSourcePosition {
                line: range.start.line,
                column: range.start.column,
                byte_offset: range.start.byte_offset,
            },
            byte_length: range.byte_length,
        }
    }

    fn covering(first: Self, last: Self) -> Self {
        Self {
            start: first.start,
            byte_length: last
                .end_byte_offset()
                .saturating_sub(first.start.byte_offset),
        }
    }

    pub fn end_byte_offset(self) -> u64 {
        self.start.byte_offset.saturating_add(self.byte_length)
    }

    pub fn source_map(self) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(
                    self.start.byte_offset,
                    u32::try_from(self.byte_length).unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: CSS_SELECTOR_CONTENT_TYPE.to_owned(),
                },
            }],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSelectorTokenAst {
    pub index: usize,
    pub depth: usize,
    pub token_kind: String,
    pub value: Option<String>,
    pub lexeme: String,
    pub recovered: bool,
    pub source_range: CssSelectorSourceRange,
}

impl From<CssEventAst> for CssSelectorTokenAst {
    fn from(event: CssEventAst) -> Self {
        Self {
            index: event.index,
            depth: event.depth,
            token_kind: event.token_kind,
            value: event.value,
            lexeme: event.lexeme,
            recovered: event.recovered,
            source_range: CssSelectorSourceRange::from_css(event.source_range),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CssSelectorFactKind {
    LexicalInvalid,
    ParseInvalid,
    NamespaceUnbound,
    FeatureUnsupported,
    CapabilityMissing,
    BudgetExceeded,
    InputUnsupported,
}

impl CssSelectorFactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LexicalInvalid => "lexical-invalid",
            Self::ParseInvalid => "parse-invalid",
            Self::NamespaceUnbound => "namespace-unbound",
            Self::FeatureUnsupported => "feature-unsupported",
            Self::CapabilityMissing => "capability-missing",
            Self::BudgetExceeded => "budget-exceeded",
            Self::InputUnsupported => "input-unsupported",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSelectorFact {
    pub kind: CssSelectorFactKind,
    pub source_range: Option<CssSelectorSourceRange>,
    pub message: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CssSelectorDiagnosticBinding {
    contract: String,
    behavior: Option<String>,
    diagnostic_code: String,
    severity: Severity,
    policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSelectorSchemaContractCatalog {
    fact_bindings: BTreeMap<String, CssSelectorDiagnosticBinding>,
}

impl CssSelectorSchemaContractCatalog {
    pub fn from_builtin() -> &'static Self {
        static CATALOG: OnceLock<CssSelectorSchemaContractCatalog> = OnceLock::new();
        CATALOG.get_or_init(|| {
            let source = builtin_schema_package_source(CSS_SELECTOR_PACKAGE_ID)
                .expect("built-in CSS selector schema package source must be registered");
            Self::from_schema_source(source.schema_source)
        })
    }

    pub fn from_schema_source(schema_source: &str) -> Self {
        let model = compile_schema_document_model(CSS_SELECTOR_SCHEMA_URI, schema_source);
        let fact_bindings = model
            .constraints
            .values()
            .filter_map(|constraint| {
                if constraint.behavior.as_deref()?.trim() != CSS_SELECTOR_FACT_BEHAVIOR {
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
                    CssSelectorDiagnosticBinding {
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

    fn binding_for_fact(&self, kind: CssSelectorFactKind) -> Option<&CssSelectorDiagnosticBinding> {
        self.fact_bindings.get(kind.as_str())
    }
}

fn css_selector_fact_diagnostics(source_uri: &str, facts: &[CssSelectorFact]) -> Vec<Diagnostic> {
    let catalog = CssSelectorSchemaContractCatalog::from_builtin();
    facts
        .iter()
        .filter_map(|fact| {
            let binding = catalog.binding_for_fact(fact.kind)?;
            Some(Diagnostic {
                uri: Some(source_uri.to_owned()),
                line: fact.source_range.map(|range| range.start.line),
                column: fact.source_range.map(|range| range.start.column),
                byte_offset: fact.source_range.map(|range| range.start.byte_offset),
                code: binding.diagnostic_code.clone(),
                severity: binding.severity,
                message: fact.message.clone(),
                details: Some(json!({
                    "schema": CSS_SELECTOR_SCHEMA_URI,
                    "schemaPackage": CSS_SELECTOR_PACKAGE_ID,
                    "schemaConstraint": binding.contract,
                    "schemaBehavior": binding.behavior,
                    "schemaPolicy": binding.policy,
                    "factKind": fact.kind.as_str(),
                    "factValue": fact.value,
                    "contentType": CSS_SELECTOR_CONTENT_TYPE,
                    "sourceRange": fact.source_range.map(|range| json!({
                        "byteOffset": range.start.byte_offset,
                        "byteLength": range.byte_length,
                        "line": range.start.line,
                        "column": range.start.column,
                    })),
                })),
                source_map: fact.source_range.map(CssSelectorSourceRange::source_map),
                ..Diagnostic::default()
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssSelectorNamespace {
    Any,
    None,
    Default {
        namespace_uri: String,
    },
    Named {
        prefix: String,
        namespace_uri: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssSelectorAttributeOperator {
    Equals,
    Includes,
    DashMatch,
    PrefixMatch,
    SuffixMatch,
    SubstringMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssSelectorAttributeModifier {
    AsciiInsensitive,
    CaseSensitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssSelectorCombinator {
    Descendant,
    Child,
    NextSibling,
    SubsequentSibling,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssSelectorSimpleSelector {
    Type {
        namespace: CssSelectorNamespace,
        local_name: String,
        universal: bool,
        source_range: CssSelectorSourceRange,
    },
    Id {
        value: String,
        source_range: CssSelectorSourceRange,
    },
    Class {
        value: String,
        source_range: CssSelectorSourceRange,
    },
    Attribute {
        namespace: CssSelectorNamespace,
        local_name: String,
        operator: Option<CssSelectorAttributeOperator>,
        value: Option<String>,
        modifier: Option<CssSelectorAttributeModifier>,
        source_range: CssSelectorSourceRange,
    },
    PseudoClass {
        name: String,
        selectors: Option<Box<CssSelectorListAst>>,
        relative: bool,
        source_range: CssSelectorSourceRange,
    },
}

impl CssSelectorSimpleSelector {
    fn source_range(&self) -> CssSelectorSourceRange {
        match self {
            Self::Type { source_range, .. }
            | Self::Id { source_range, .. }
            | Self::Class { source_range, .. }
            | Self::Attribute { source_range, .. }
            | Self::PseudoClass { source_range, .. } => *source_range,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSelectorCompoundAst {
    pub simple_selectors: Vec<CssSelectorSimpleSelector>,
    pub source_range: CssSelectorSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSelectorAst {
    pub leading_combinator: Option<CssSelectorCombinator>,
    pub compounds: Vec<CssSelectorCompoundAst>,
    pub combinators: Vec<CssSelectorCombinator>,
    pub specificity: (u32, u32, u32),
    pub source_range: CssSelectorSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CssSelectorListAst {
    pub selectors: Vec<CssSelectorAst>,
    pub source_range: CssSelectorSourceRange,
}

#[derive(Debug, Clone)]
pub struct CssSelectorExpressionAst {
    pub source: CssSelectorSource,
    pub identity: FormatIdentity,
    pub language_version: String,
    pub tokens: Vec<CssSelectorTokenAst>,
    pub selector_list: Option<CssSelectorListAst>,
    pub facts: Vec<CssSelectorFact>,
    pub source_map: SourceMapStack,
}

impl QueryNativeArtifact for CssSelectorExpressionAst {
    fn representation_id(&self) -> &'static str {
        CSS_SELECTOR_AST_REPRESENTATION_ID
    }

    fn source_map(&self) -> Option<&SourceMapStack> {
        Some(&self.source_map)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl QueryAstOwner for CssSelectorExpressionAst {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::CssSelector
    }

    fn identity(&self) -> &FormatIdentity {
        &self.identity
    }

    fn source_uri(&self) -> &str {
        &self.source.uri
    }
}

pub fn validate_css_selector_source_bytes(
    request: CssSelectorSourceRequest<'_>,
) -> Vec<Diagnostic> {
    let (_, diagnostics) = css_selector_expression_ast_from_source_bytes(request);
    diagnostics
}

pub fn css_selector_expression_ast_from_source_bytes(
    request: CssSelectorSourceRequest<'_>,
) -> (Option<CssSelectorExpressionAst>, Vec<Diagnostic>) {
    let content_type = request
        .content_type
        .unwrap_or(CSS_SELECTOR_CONTENT_TYPE)
        .to_owned();
    let source = match std::str::from_utf8(request.bytes) {
        Ok(source) => source,
        Err(error) => {
            let fact = CssSelectorFact {
                kind: CssSelectorFactKind::LexicalInvalid,
                source_range: None,
                message: format!(
                    "CSS selector bytes are not valid UTF-8 at byte {}: {error}",
                    error.valid_up_to()
                ),
                value: None,
            };
            return (
                None,
                css_selector_fact_diagnostics(request.source_uri, &[fact]),
            );
        }
    };

    let (events, css_facts) = css_syntax_lossless_events(source);
    let tokens = events
        .into_iter()
        .map(CssSelectorTokenAst::from)
        .collect::<Vec<_>>();
    let mut facts = css_facts
        .into_iter()
        .filter(|fact| {
            matches!(
                fact.kind,
                CssFactKind::ParseError
                    | CssFactKind::InvalidToken
                    | CssFactKind::BadString
                    | CssFactKind::BadUrl
            )
        })
        .map(|fact| CssSelectorFact {
            kind: CssSelectorFactKind::LexicalInvalid,
            source_range: fact.source_range.map(CssSelectorSourceRange::from_css),
            message: fact.message,
            value: fact.value,
        })
        .collect::<Vec<_>>();

    if crate::schema::registry::content_type_essence(&content_type) != CSS_SELECTOR_CONTENT_TYPE {
        facts.push(CssSelectorFact {
            kind: CssSelectorFactKind::InputUnsupported,
            source_range: tokens.first().map(|token| token.source_range),
            message: format!(
                "CSS selector source requires `{CSS_SELECTOR_CONTENT_TYPE}`, not `{content_type}`"
            ),
            value: Some(content_type.clone()),
        });
    }

    let selector_list = SelectorParser::new(&tokens, request.namespace_bindings, &mut facts)
        .parse_selector_list(0, tokens.len(), false);
    let source_range = tokens
        .first()
        .zip(tokens.last())
        .map(|(first, last)| {
            CssSelectorSourceRange::covering(first.source_range, last.source_range)
        })
        .unwrap_or(CssSelectorSourceRange {
            start: CssSelectorSourcePosition {
                line: 1,
                column: 1,
                byte_offset: 0,
            },
            byte_length: 0,
        });
    let ast = CssSelectorExpressionAst {
        source: CssSelectorSource {
            uri: request.source_uri.to_owned(),
            content_type: content_type.clone(),
            byte_length: request.bytes.len(),
        },
        identity: FormatIdentity {
            content_type: Some(content_type),
            schema: Some(CSS_SELECTOR_SCHEMA_URI.to_owned()),
            namespaces: request.namespace_bindings.clone(),
            ..FormatIdentity::default()
        },
        language_version: CSS_SELECTOR_LANGUAGE_VERSION.to_owned(),
        tokens,
        selector_list,
        facts,
        source_map: source_range.source_map(),
    };
    let diagnostics = css_selector_fact_diagnostics(request.source_uri, &ast.facts);
    (Some(ast), diagnostics)
}

struct SelectorParser<'a, 'f> {
    tokens: &'a [CssSelectorTokenAst],
    namespaces: &'a BTreeMap<String, String>,
    facts: &'f mut Vec<CssSelectorFact>,
}

impl<'a, 'f> SelectorParser<'a, 'f> {
    fn new(
        tokens: &'a [CssSelectorTokenAst],
        namespaces: &'a BTreeMap<String, String>,
        facts: &'f mut Vec<CssSelectorFact>,
    ) -> Self {
        Self {
            tokens,
            namespaces,
            facts,
        }
    }

    fn parse_selector_list(
        &mut self,
        start: usize,
        end: usize,
        allow_relative: bool,
    ) -> Option<CssSelectorListAst> {
        let start = self.skip_trivia(start, end).0;
        let end = self.trim_trivia_end(start, end);
        if start >= end {
            self.parse_error(None, "CSS selector list is empty");
            return None;
        }
        let base_depth = self.tokens[start..end]
            .iter()
            .filter(|token| !is_trivia(token))
            .map(|token| token.depth)
            .min()
            .unwrap_or(0);
        let mut selectors = Vec::new();
        let mut segment_start = start;
        for index in start..end {
            if self.tokens[index].depth == base_depth && self.tokens[index].token_kind == "comma" {
                let segment_end = self.trim_trivia_end(segment_start, index);
                if let Some(selector) =
                    self.parse_selector(segment_start, segment_end, allow_relative)
                {
                    selectors.push(selector);
                }
                segment_start = index.saturating_add(1);
            }
        }
        let segment_end = self.trim_trivia_end(segment_start, end);
        if let Some(selector) = self.parse_selector(segment_start, segment_end, allow_relative) {
            selectors.push(selector);
        }
        if selectors.is_empty() {
            return None;
        }
        Some(CssSelectorListAst {
            source_range: CssSelectorSourceRange::covering(
                selectors.first().unwrap().source_range,
                selectors.last().unwrap().source_range,
            ),
            selectors,
        })
    }

    fn parse_selector(
        &mut self,
        start: usize,
        end: usize,
        allow_relative: bool,
    ) -> Option<CssSelectorAst> {
        let (mut cursor, _) = self.skip_trivia(start, end);
        if cursor >= end {
            self.parse_error(
                self.tokens.get(start).map(|token| token.source_range),
                "Empty selector in selector list",
            );
            return None;
        }
        let leading_combinator = if allow_relative {
            self.explicit_combinator(cursor).map(|(combinator, next)| {
                cursor = self.skip_trivia(next, end).0;
                combinator
            })
        } else {
            None
        };
        let (first, next) = self.parse_compound(cursor, end)?;
        cursor = next;
        let mut compounds = vec![first];
        let mut combinators = Vec::new();
        loop {
            let (after_trivia, saw_trivia) = self.skip_trivia(cursor, end);
            if after_trivia >= end {
                break;
            }
            if self.is_column_combinator(after_trivia) {
                self.unsupported(
                    Some(CssSelectorSourceRange::covering(
                        self.tokens[after_trivia].source_range,
                        self.tokens[after_trivia + 1].source_range,
                    )),
                    "The Selectors Level 4 column combinator is not enabled by the current conformance matrix",
                    Some("||".to_owned()),
                );
                return None;
            }
            let (combinator, next) =
                if let Some((explicit, next)) = self.explicit_combinator(after_trivia) {
                    (explicit, self.skip_trivia(next, end).0)
                } else if saw_trivia {
                    (CssSelectorCombinator::Descendant, after_trivia)
                } else {
                    self.parse_error(
                        self.tokens
                            .get(after_trivia)
                            .map(|token| token.source_range),
                        "Expected a CSS selector combinator",
                    );
                    return None;
                };
            let (compound, next_cursor) = self.parse_compound(next, end)?;
            combinators.push(combinator);
            compounds.push(compound);
            cursor = next_cursor;
        }
        let source_range = CssSelectorSourceRange::covering(
            compounds.first().unwrap().source_range,
            compounds.last().unwrap().source_range,
        );
        let specificity = selector_specificity(&compounds);
        Some(CssSelectorAst {
            leading_combinator,
            compounds,
            combinators,
            specificity,
            source_range,
        })
    }

    fn parse_compound(
        &mut self,
        start: usize,
        end: usize,
    ) -> Option<(CssSelectorCompoundAst, usize)> {
        let mut cursor = start;
        let mut simple_selectors = Vec::new();
        if let Some((simple, next)) = self.parse_type_selector(cursor, end) {
            simple_selectors.push(simple);
            cursor = next;
        }
        loop {
            if cursor >= end || is_trivia(&self.tokens[cursor]) {
                break;
            }
            if self.explicit_combinator(cursor).is_some()
                || self.tokens[cursor].token_kind == "comma"
            {
                break;
            }
            let token = &self.tokens[cursor];
            match token.token_kind.as_str() {
                "id-hash" | "hash" => {
                    simple_selectors.push(CssSelectorSimpleSelector::Id {
                        value: token.value.clone().unwrap_or_default(),
                        source_range: token.source_range,
                    });
                    cursor += 1;
                }
                "delimiter" if token.value.as_deref() == Some(".") => {
                    let Some(name) = self
                        .tokens
                        .get(cursor + 1)
                        .filter(|next| next.token_kind == "ident" && !is_trivia(next))
                    else {
                        self.parse_error(
                            Some(token.source_range),
                            "Class selector requires an identifier after `.`",
                        );
                        return None;
                    };
                    simple_selectors.push(CssSelectorSimpleSelector::Class {
                        value: name.value.clone().unwrap_or_default(),
                        source_range: CssSelectorSourceRange::covering(
                            token.source_range,
                            name.source_range,
                        ),
                    });
                    cursor += 2;
                }
                "square-open" => {
                    let (simple, next) = self.parse_attribute_selector(cursor, end)?;
                    simple_selectors.push(simple);
                    cursor = next;
                }
                "colon" => {
                    let (simple, next) = self.parse_pseudo_class(cursor, end)?;
                    simple_selectors.push(simple);
                    cursor = next;
                }
                _ => break,
            }
        }
        if simple_selectors.is_empty() {
            self.parse_error(
                self.tokens.get(start).map(|token| token.source_range),
                "Expected a CSS compound selector",
            );
            return None;
        }
        let source_range = CssSelectorSourceRange::covering(
            simple_selectors.first().unwrap().source_range(),
            simple_selectors.last().unwrap().source_range(),
        );
        Some((
            CssSelectorCompoundAst {
                simple_selectors,
                source_range,
            },
            cursor,
        ))
    }

    fn parse_type_selector(
        &mut self,
        start: usize,
        end: usize,
    ) -> Option<(CssSelectorSimpleSelector, usize)> {
        let first = self.tokens.get(start)?;
        if is_trivia(first) {
            return None;
        }
        let is_ident = first.token_kind == "ident";
        let is_universal = is_delimiter(first, "*");
        let explicit_none = is_delimiter(first, "|");
        if !is_ident && !is_universal && !explicit_none {
            return None;
        }

        if explicit_none {
            let name = self.tokens.get(start + 1)?;
            if name.token_kind != "ident" && !is_delimiter(name, "*") {
                self.parse_error(
                    Some(first.source_range),
                    "No-namespace type selector requires a name or `*`",
                );
                return None;
            }
            return Some((
                CssSelectorSimpleSelector::Type {
                    namespace: CssSelectorNamespace::None,
                    local_name: name.value.clone().unwrap_or_else(|| "*".to_owned()),
                    universal: is_delimiter(name, "*"),
                    source_range: CssSelectorSourceRange::covering(
                        first.source_range,
                        name.source_range,
                    ),
                },
                start + 2,
            ));
        }

        if start + 2 < end && is_delimiter(&self.tokens[start + 1], "|") {
            let name = &self.tokens[start + 2];
            if name.token_kind != "ident" && !is_delimiter(name, "*") {
                self.parse_error(
                    Some(name.source_range),
                    "Namespaced type selector requires a name or `*`",
                );
                return None;
            }
            let namespace = if is_universal {
                CssSelectorNamespace::Any
            } else {
                self.named_namespace(first.value.clone().unwrap_or_default(), first.source_range)
            };
            return Some((
                CssSelectorSimpleSelector::Type {
                    namespace,
                    local_name: name.value.clone().unwrap_or_else(|| "*".to_owned()),
                    universal: is_delimiter(name, "*"),
                    source_range: CssSelectorSourceRange::covering(
                        first.source_range,
                        name.source_range,
                    ),
                },
                start + 3,
            ));
        }

        let namespace = self
            .namespaces
            .get("")
            .cloned()
            .map(|namespace_uri| CssSelectorNamespace::Default { namespace_uri })
            .unwrap_or(CssSelectorNamespace::Any);
        Some((
            CssSelectorSimpleSelector::Type {
                namespace,
                local_name: first.value.clone().unwrap_or_else(|| "*".to_owned()),
                universal: is_universal,
                source_range: first.source_range,
            },
            start + 1,
        ))
    }

    fn parse_attribute_selector(
        &mut self,
        start: usize,
        end: usize,
    ) -> Option<(CssSelectorSimpleSelector, usize)> {
        let open = &self.tokens[start];
        let close = self.find_close(start, end, "square-close")?;
        let significant = (start + 1..close)
            .filter(|index| !is_trivia(&self.tokens[*index]))
            .collect::<Vec<_>>();
        if significant.is_empty() {
            self.parse_error(Some(open.source_range), "Attribute selector is empty");
            return None;
        }
        let mut position = 0usize;
        let first = &self.tokens[significant[position]];
        let (namespace, local_name) = if is_delimiter(first, "|") {
            position += 1;
            let name = significant
                .get(position)
                .and_then(|index| self.tokens.get(*index))?;
            position += 1;
            (
                CssSelectorNamespace::None,
                name.value.clone().unwrap_or_default(),
            )
        } else if position + 2 < significant.len()
            && is_delimiter(&self.tokens[significant[position + 1]], "|")
        {
            let namespace = if is_delimiter(first, "*") {
                CssSelectorNamespace::Any
            } else {
                self.named_namespace(first.value.clone().unwrap_or_default(), first.source_range)
            };
            let name = &self.tokens[significant[position + 2]];
            position += 3;
            (namespace, name.value.clone().unwrap_or_default())
        } else {
            position += 1;
            (
                CssSelectorNamespace::None,
                first.value.clone().unwrap_or_default(),
            )
        };
        if local_name.is_empty() {
            self.parse_error(
                Some(first.source_range),
                "Attribute selector requires a name",
            );
            return None;
        }

        let mut operator = None;
        let mut value = None;
        let mut modifier = None;
        if position < significant.len() {
            let operator_token = &self.tokens[significant[position]];
            operator = match operator_token.token_kind.as_str() {
                "include-match" => Some(CssSelectorAttributeOperator::Includes),
                "dash-match" => Some(CssSelectorAttributeOperator::DashMatch),
                "prefix-match" => Some(CssSelectorAttributeOperator::PrefixMatch),
                "suffix-match" => Some(CssSelectorAttributeOperator::SuffixMatch),
                "substring-match" => Some(CssSelectorAttributeOperator::SubstringMatch),
                "delimiter" if operator_token.value.as_deref() == Some("=") => {
                    Some(CssSelectorAttributeOperator::Equals)
                }
                _ => None,
            };
            if operator.is_none() {
                self.parse_error(
                    Some(operator_token.source_range),
                    "Invalid attribute selector operator",
                );
                return None;
            }
            position += 1;
            let Some(value_token) = significant
                .get(position)
                .and_then(|index| self.tokens.get(*index))
            else {
                self.parse_error(
                    Some(operator_token.source_range),
                    "Attribute selector operator requires a value",
                );
                return None;
            };
            if !matches!(value_token.token_kind.as_str(), "ident" | "string") {
                self.parse_error(
                    Some(value_token.source_range),
                    "Attribute selector value must be an identifier or string",
                );
                return None;
            }
            value = value_token.value.clone();
            position += 1;
            if let Some(modifier_token) = significant
                .get(position)
                .and_then(|index| self.tokens.get(*index))
            {
                modifier = match modifier_token.value.as_deref() {
                    Some(value) if value.eq_ignore_ascii_case("i") => {
                        Some(CssSelectorAttributeModifier::AsciiInsensitive)
                    }
                    Some(value) if value.eq_ignore_ascii_case("s") => {
                        Some(CssSelectorAttributeModifier::CaseSensitive)
                    }
                    _ => {
                        self.parse_error(
                            Some(modifier_token.source_range),
                            "Unsupported attribute selector modifier",
                        );
                        return None;
                    }
                };
                position += 1;
            }
        }
        if position != significant.len() {
            self.parse_error(
                significant
                    .get(position)
                    .and_then(|index| self.tokens.get(*index))
                    .map(|token| token.source_range),
                "Unexpected token in attribute selector",
            );
            return None;
        }
        Some((
            CssSelectorSimpleSelector::Attribute {
                namespace,
                local_name,
                operator,
                value,
                modifier,
                source_range: CssSelectorSourceRange::covering(
                    open.source_range,
                    self.tokens[close].source_range,
                ),
            },
            close + 1,
        ))
    }

    fn parse_pseudo_class(
        &mut self,
        start: usize,
        end: usize,
    ) -> Option<(CssSelectorSimpleSelector, usize)> {
        let colon = &self.tokens[start];
        let next = self.tokens.get(start + 1)?;
        if next.token_kind == "colon" {
            let feature = self.tokens.get(start + 2);
            let range = feature.map_or(colon.source_range, |token| {
                CssSelectorSourceRange::covering(colon.source_range, token.source_range)
            });
            self.unsupported(
                Some(range),
                "CSS selector query expressions do not select pseudo-elements",
                feature.and_then(|token| token.value.clone()),
            );
            let cursor = self.consume_simple_or_function(start + 2, end);
            return Some((
                CssSelectorSimpleSelector::PseudoClass {
                    name: "pseudo-element".to_owned(),
                    selectors: None,
                    relative: false,
                    source_range: range,
                },
                cursor,
            ));
        }
        if next.token_kind == "ident" {
            let name = next.value.clone().unwrap_or_default();
            let range = Some(CssSelectorSourceRange::covering(
                colon.source_range,
                next.source_range,
            ));
            if requires_host_capability(&name) {
                self.capability_missing(
                    range,
                    format!("Pseudo-class `:{name}` requires lifecycle host-state capabilities"),
                    Some(name.clone()),
                );
            } else {
                self.unsupported(
                    range,
                    format!(
                        "Pseudo-class `:{name}` is not enabled by the current conformance matrix"
                    ),
                    Some(name.clone()),
                );
            }
            return Some((
                CssSelectorSimpleSelector::PseudoClass {
                    name,
                    selectors: None,
                    relative: false,
                    source_range: CssSelectorSourceRange::covering(
                        colon.source_range,
                        next.source_range,
                    ),
                },
                start + 2,
            ));
        }
        if next.token_kind != "function-open" {
            self.parse_error(
                Some(next.source_range),
                "Pseudo-class requires an identifier or function",
            );
            return None;
        }
        let name = next.value.clone().unwrap_or_default().to_ascii_lowercase();
        let Some(close) = self.find_close(start + 1, end, "parenthesis-close") else {
            self.parse_error(
                Some(next.source_range),
                "Pseudo-class function is not closed",
            );
            return None;
        };
        let relative = name == "has";
        let selectors = if matches!(name.as_str(), "is" | "where" | "not" | "has") {
            self.parse_selector_list(start + 2, close, relative)
                .map(Box::new)
        } else {
            let range = Some(CssSelectorSourceRange::covering(
                colon.source_range,
                self.tokens[close].source_range,
            ));
            if requires_host_capability(&name) {
                self.capability_missing(
                    range,
                    format!(
                        "Pseudo-class function `:{name}()` requires lifecycle host-state capabilities"
                    ),
                    Some(name.clone()),
                );
            } else {
                self.unsupported(
                    range,
                    format!(
                        "Pseudo-class function `:{name}()` is not enabled by the current conformance matrix"
                    ),
                    Some(name.clone()),
                );
            }
            None
        };
        Some((
            CssSelectorSimpleSelector::PseudoClass {
                name,
                selectors,
                relative,
                source_range: CssSelectorSourceRange::covering(
                    colon.source_range,
                    self.tokens[close].source_range,
                ),
            },
            close + 1,
        ))
    }

    fn named_namespace(
        &mut self,
        prefix: String,
        source_range: CssSelectorSourceRange,
    ) -> CssSelectorNamespace {
        let namespace_uri = self.namespaces.get(&prefix).cloned().unwrap_or_default();
        if namespace_uri.is_empty() {
            self.facts.push(CssSelectorFact {
                kind: CssSelectorFactKind::NamespaceUnbound,
                source_range: Some(source_range),
                message: format!(
                    "CSS selector namespace prefix `{prefix}` is not explicitly bound"
                ),
                value: Some(prefix.clone()),
            });
        }
        CssSelectorNamespace::Named {
            prefix,
            namespace_uri,
        }
    }

    fn find_close(&mut self, start: usize, end: usize, kind: &str) -> Option<usize> {
        let depth = self.tokens.get(start)?.depth;
        let found = (start + 1..end).find(|index| {
            self.tokens[*index].depth == depth && self.tokens[*index].token_kind == kind
        });
        if found.is_none() {
            self.parse_error(
                self.tokens.get(start).map(|token| token.source_range),
                "CSS component-value block is not closed",
            );
        }
        found
    }

    fn consume_simple_or_function(&mut self, start: usize, end: usize) -> usize {
        let Some(token) = self.tokens.get(start) else {
            return end;
        };
        if token.token_kind == "function-open" {
            self.find_close(start, end, "parenthesis-close")
                .map_or(end, |close| close + 1)
        } else {
            (start + 1).min(end)
        }
    }

    fn explicit_combinator(&self, index: usize) -> Option<(CssSelectorCombinator, usize)> {
        let token = self.tokens.get(index)?;
        let combinator = match token.value.as_deref()? {
            ">" => CssSelectorCombinator::Child,
            "+" => CssSelectorCombinator::NextSibling,
            "~" => CssSelectorCombinator::SubsequentSibling,
            "|" if self
                .tokens
                .get(index + 1)
                .is_some_and(|next| is_delimiter(next, "|")) =>
            {
                return None;
            }
            _ => return None,
        };
        Some((combinator, index + 1))
    }

    fn is_column_combinator(&self, index: usize) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| is_delimiter(token, "|"))
            && self
                .tokens
                .get(index + 1)
                .is_some_and(|token| is_delimiter(token, "|"))
    }

    fn skip_trivia(&self, mut index: usize, end: usize) -> (usize, bool) {
        let mut saw = false;
        while index < end && is_trivia(&self.tokens[index]) {
            saw = true;
            index += 1;
        }
        (index, saw)
    }

    fn trim_trivia_end(&self, start: usize, mut end: usize) -> usize {
        while end > start && is_trivia(&self.tokens[end - 1]) {
            end -= 1;
        }
        end
    }

    fn parse_error(
        &mut self,
        source_range: Option<CssSelectorSourceRange>,
        message: impl Into<String>,
    ) {
        self.facts.push(CssSelectorFact {
            kind: CssSelectorFactKind::ParseInvalid,
            source_range,
            message: message.into(),
            value: None,
        });
    }

    fn unsupported(
        &mut self,
        source_range: Option<CssSelectorSourceRange>,
        message: impl Into<String>,
        value: Option<String>,
    ) {
        self.facts.push(CssSelectorFact {
            kind: CssSelectorFactKind::FeatureUnsupported,
            source_range,
            message: message.into(),
            value,
        });
    }

    fn capability_missing(
        &mut self,
        source_range: Option<CssSelectorSourceRange>,
        message: impl Into<String>,
        value: Option<String>,
    ) {
        self.facts.push(CssSelectorFact {
            kind: CssSelectorFactKind::CapabilityMissing,
            source_range,
            message: message.into(),
            value,
        });
    }
}

fn requires_host_capability(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "active"
            | "any-link"
            | "autofill"
            | "buffering"
            | "checked"
            | "current"
            | "default"
            | "defined"
            | "disabled"
            | "enabled"
            | "focus"
            | "focus-visible"
            | "focus-within"
            | "fullscreen"
            | "future"
            | "host"
            | "host-context"
            | "hover"
            | "in-range"
            | "indeterminate"
            | "invalid"
            | "link"
            | "local-link"
            | "modal"
            | "muted"
            | "open"
            | "optional"
            | "out-of-range"
            | "past"
            | "paused"
            | "picture-in-picture"
            | "placeholder-shown"
            | "playing"
            | "read-only"
            | "read-write"
            | "required"
            | "seeking"
            | "stalled"
            | "state"
            | "target"
            | "target-within"
            | "user-invalid"
            | "user-valid"
            | "valid"
            | "visited"
            | "volume-locked"
    )
}

fn is_trivia(token: &CssSelectorTokenAst) -> bool {
    matches!(
        token.token_kind.as_str(),
        "whitespace" | "comment" | "presentation-gap"
    )
}

fn is_delimiter(token: &CssSelectorTokenAst, value: &str) -> bool {
    token.token_kind == "delimiter" && token.value.as_deref() == Some(value)
}

fn selector_specificity(compounds: &[CssSelectorCompoundAst]) -> (u32, u32, u32) {
    let mut specificity = (0u32, 0u32, 0u32);
    for simple in compounds
        .iter()
        .flat_map(|compound| &compound.simple_selectors)
    {
        match simple {
            CssSelectorSimpleSelector::Id { .. } => specificity.0 += 1,
            CssSelectorSimpleSelector::Class { .. }
            | CssSelectorSimpleSelector::Attribute { .. } => specificity.1 += 1,
            CssSelectorSimpleSelector::PseudoClass {
                name, selectors, ..
            } => {
                if name != "where" {
                    let nested = selectors
                        .as_deref()
                        .and_then(|list| {
                            list.selectors
                                .iter()
                                .map(|selector| selector.specificity)
                                .max()
                        })
                        .unwrap_or((0, 1, 0));
                    specificity.0 += nested.0;
                    specificity.1 += nested.1;
                    specificity.2 += nested.2;
                }
            }
            CssSelectorSimpleSelector::Type {
                universal: false, ..
            } => specificity.2 += 1,
            CssSelectorSimpleSelector::Type {
                universal: true, ..
            } => {}
        }
    }
    specificity
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CssSelectorNodeHandle {
    Html { event_index: usize },
    Xml { event_index: usize },
}

#[derive(Debug, Clone)]
pub struct CssSelectorElementTreeOwner {
    owner: Arc<LoadedInputAstStream>,
    identity: FormatIdentity,
    source_map: SourceMapStack,
}

impl CssSelectorElementTreeOwner {
    pub fn from_lifecycle(
        owner: Arc<LoadedInputAstStream>,
        identity: FormatIdentity,
    ) -> Result<Self, CssSelectorFact> {
        let byte_length = match owner.as_ref() {
            LoadedInputAstStream::HtmlDocument(document) => document.source.byte_length,
            LoadedInputAstStream::XmlDocument(document) => document.source.byte_length,
            LoadedInputAstStream::XhtmlDocument(document) => {
                document.xml_document.source.byte_length
            }
            LoadedInputAstStream::SvgDocument(document) => document.xml_document.source.byte_length,
            LoadedInputAstStream::MathMlDocument(document) => {
                document.xml_document.source.byte_length
            }
            LoadedInputAstStream::XsltStylesheet(document) => {
                document.xml_document.source.byte_length
            }
            LoadedInputAstStream::RelaxNgDocument(document) => document
                .xml_document
                .as_ref()
                .map(|xml| xml.source.byte_length)
                .ok_or_else(|| CssSelectorFact {
                    kind: CssSelectorFactKind::InputUnsupported,
                    source_range: None,
                    message: "Relax NG compact syntax does not expose an element-tree view"
                        .to_owned(),
                    value: Some("relax-ng-compact".to_owned()),
                })?,
            _ => {
                return Err(CssSelectorFact {
                    kind: CssSelectorFactKind::InputUnsupported,
                    source_range: None,
                    message: "Lifecycle input does not expose an HTML/XML-family element-tree view"
                        .to_owned(),
                    value: Some(lifecycle_ast_stream_kind(owner.as_ref()).to_owned()),
                })
            }
        };
        let content_type = identity
            .content_type
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_owned());
        Ok(Self {
            owner,
            identity,
            source_map: SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(1),
                    span: FrameSpan::Single(ByteRange::new(
                        0,
                        u32::try_from(byte_length).unwrap_or(u32::MAX),
                    )),
                    transform: TransformKind::ContentTypeTransform { content_type },
                }],
            },
        })
    }

    pub fn lifecycle_owner(&self) -> &Arc<LoadedInputAstStream> {
        &self.owner
    }

    fn html_document(&self) -> Option<&HtmlDocumentAst> {
        match self.owner.as_ref() {
            LoadedInputAstStream::HtmlDocument(document) => Some(document),
            _ => None,
        }
    }

    fn xml_document(&self) -> Option<&XmlDocumentAst> {
        match self.owner.as_ref() {
            LoadedInputAstStream::XmlDocument(document) => Some(document),
            LoadedInputAstStream::XhtmlDocument(document) => Some(&document.xml_document),
            LoadedInputAstStream::SvgDocument(document) => Some(&document.xml_document),
            LoadedInputAstStream::MathMlDocument(document) => Some(&document.xml_document),
            LoadedInputAstStream::XsltStylesheet(document) => Some(&document.xml_document),
            LoadedInputAstStream::RelaxNgDocument(document) => document.xml_document.as_ref(),
            _ => None,
        }
    }

    fn all_elements(&self) -> Vec<CssSelectorNativeNode> {
        if let Some(document) = self.html_document() {
            return document
                .events
                .iter()
                .filter(|event| event.kind == HtmlEventKind::StartElement)
                .map(|event| CssSelectorNativeNode {
                    owner: self.clone(),
                    handle: CssSelectorNodeHandle::Html {
                        event_index: event.index,
                    },
                })
                .collect();
        }
        self.xml_document()
            .into_iter()
            .flat_map(|document| &document.events)
            .filter(|event| {
                matches!(
                    event.kind,
                    XmlEventKind::StartElement | XmlEventKind::EmptyElement
                )
            })
            .map(|event| CssSelectorNativeNode {
                owner: self.clone(),
                handle: CssSelectorNodeHandle::Xml {
                    event_index: event.index,
                },
            })
            .collect()
    }
}

fn lifecycle_ast_stream_kind(owner: &LoadedInputAstStream) -> &'static str {
    match owner {
        LoadedInputAstStream::HtmlDocument(_) => "html-document",
        LoadedInputAstStream::CssDocument(_) => "css-document",
        LoadedInputAstStream::CsvDocument(_) => "csv-document",
        LoadedInputAstStream::YamlDocument(_) => "yaml-document",
        LoadedInputAstStream::JsonDocument(_) => "json-document",
        LoadedInputAstStream::JsonSchemaDocument(_) => "json-schema-document",
        LoadedInputAstStream::MarkdownDocument(_) => "markdown-document",
        LoadedInputAstStream::XmlDocument(_) => "xml-document",
        LoadedInputAstStream::XhtmlDocument(_) => "xhtml-document",
        LoadedInputAstStream::SvgDocument(_) => "svg-document",
        LoadedInputAstStream::MathMlDocument(_) => "mathml-document",
        LoadedInputAstStream::XPathExpression(_) => "xpath-expression",
        LoadedInputAstStream::XsltStylesheet(_) => "xslt-stylesheet",
        LoadedInputAstStream::RelaxNgDocument(_) => "relax-ng-document",
    }
}

impl QueryNativeArtifact for CssSelectorElementTreeOwner {
    fn representation_id(&self) -> &'static str {
        ELEMENT_TREE_REPRESENTATION_ID
    }

    fn source_map(&self) -> Option<&SourceMapStack> {
        Some(&self.source_map)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl QueryInputOwner for CssSelectorElementTreeOwner {
    fn identity(&self) -> &FormatIdentity {
        &self.identity
    }

    fn input_models(&self) -> &[QueryInputModel] {
        ELEMENT_TREE_INPUT_MODELS
    }
}

#[derive(Debug, Clone)]
pub struct CssSelectorNativeNode {
    owner: CssSelectorElementTreeOwner,
    handle: CssSelectorNodeHandle,
}

impl PartialEq for CssSelectorNativeNode {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.owner.owner, &other.owner.owner) && self.handle == other.handle
    }
}

impl Eq for CssSelectorNativeNode {}

impl CssSelectorNativeNode {
    fn html_event(&self) -> Option<&HtmlEventAst> {
        let CssSelectorNodeHandle::Html { event_index } = self.handle else {
            return None;
        };
        self.owner.html_document()?.events.get(event_index)
    }

    fn xml_event(&self) -> Option<&XmlEventAst> {
        let CssSelectorNodeHandle::Xml { event_index } = self.handle else {
            return None;
        };
        self.owner.xml_document()?.events.get(event_index)
    }

    fn depth(&self) -> usize {
        self.html_event()
            .map(|event| event.depth)
            .or_else(|| self.xml_event().map(|event| event.depth))
            .unwrap_or(0)
    }

    fn local_name(&self) -> &str {
        self.html_event()
            .and_then(|event| event.local_name.as_deref())
            .or_else(|| {
                self.xml_event()
                    .and_then(|event| event.local_name.as_deref())
            })
            .unwrap_or_default()
    }

    fn namespace_uri(&self) -> &str {
        self.html_event()
            .map(|event| event.namespace_uri.as_str())
            .or_else(|| {
                self.xml_event()
                    .and_then(|event| event.namespace_uri.as_deref())
            })
            .unwrap_or_default()
    }

    fn is_html_element(&self) -> bool {
        self.namespace_uri() == HTML_NAMESPACE_URI
    }

    fn document_order(&self) -> u64 {
        match self.handle {
            CssSelectorNodeHandle::Html { event_index }
            | CssSelectorNodeHandle::Xml { event_index } => event_index as u64,
        }
    }

    fn node_id(&self) -> String {
        match self.handle {
            CssSelectorNodeHandle::Html { event_index } => format!("html:event:{event_index}"),
            CssSelectorNodeHandle::Xml { event_index } => format!("xml:event:{event_index}"),
        }
    }

    fn source_range(&self) -> (u64, u64) {
        self.html_event()
            .map(|event| {
                (
                    event.source_range.start.byte_offset,
                    event.source_range.byte_length,
                )
            })
            .or_else(|| {
                self.xml_event().map(|event| {
                    (
                        event.source_range.start.byte_offset,
                        event.source_range.byte_length,
                    )
                })
            })
            .unwrap_or((0, 0))
    }

    fn source_map(&self) -> SourceMapStack {
        let (byte_offset, byte_length) = self.source_range();
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(
                    byte_offset,
                    u32::try_from(byte_length).unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: self
                        .owner
                        .identity
                        .content_type
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_owned()),
                },
            }],
        }
    }

    fn parent(
        &self,
        state: &mut CssSelectorEvaluationState<'_>,
    ) -> Result<Option<Self>, CssSelectorFact> {
        let depth = self.depth();
        if depth == 0 {
            return Ok(None);
        }
        for candidate in self.owner.all_elements().into_iter().rev() {
            if candidate.document_order() >= self.document_order() {
                continue;
            }
            state.consume()?;
            if candidate.depth().saturating_add(1) == depth {
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }

    fn previous_siblings(
        &self,
        state: &mut CssSelectorEvaluationState<'_>,
    ) -> Result<Vec<Self>, CssSelectorFact> {
        let depth = self.depth();
        let mut siblings = Vec::new();
        for candidate in self.owner.all_elements().into_iter().rev() {
            if candidate.document_order() >= self.document_order() {
                continue;
            }
            state.consume()?;
            if candidate.depth() < depth {
                break;
            }
            if candidate.depth() == depth {
                siblings.push(candidate);
            }
        }
        siblings.reverse();
        Ok(siblings)
    }

    fn attribute_values<'a>(
        &'a self,
        namespace: &CssSelectorNamespace,
        local_name: &str,
    ) -> Vec<&'a str> {
        if let Some(event) = self.html_event() {
            return event
                .attributes
                .iter()
                .filter(|attribute| {
                    matches!(
                        namespace,
                        CssSelectorNamespace::Any | CssSelectorNamespace::None
                    ) && attribute.local_name.eq_ignore_ascii_case(local_name)
                })
                .map(|attribute| attribute.value.as_deref().unwrap_or_default())
                .collect();
        }
        self.xml_event()
            .into_iter()
            .flat_map(|event| &event.attributes)
            .filter(|attribute| {
                namespace_matches(
                    namespace,
                    attribute.namespace_uri.as_deref().unwrap_or_default(),
                ) && attribute.local_name == local_name
            })
            .map(xml_attribute_value)
            .collect()
    }
}

fn xml_attribute_value(attribute: &XmlAttributeAst) -> &str {
    attribute
        .entity_decoded_value
        .as_deref()
        .unwrap_or(attribute.value.as_str())
}

#[derive(Debug, Clone)]
pub struct CssSelectorMatchedNode {
    pub native_node: CssSelectorNativeNode,
    pub node_id: String,
    pub local_name: String,
    pub namespace_uri: String,
    pub document_order: u64,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone)]
pub struct CssSelectorResultArtifact {
    pub content_type: String,
    pub schema_uri: String,
    pub language_version: String,
    pub matches: Vec<CssSelectorMatchedNode>,
    pub observed_work_units: u64,
    pub source_map: SourceMapStack,
}

impl QueryNativeArtifact for CssSelectorResultArtifact {
    fn representation_id(&self) -> &'static str {
        CSS_SELECTOR_RESULT_REPRESENTATION_ID
    }

    fn source_map(&self) -> Option<&SourceMapStack> {
        Some(&self.source_map)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl QueryNativeResult for CssSelectorResultArtifact {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::CssSelector
    }
}

#[derive(Debug, Clone, Default)]
pub struct CemCssSelectorEvaluator;

impl QueryEvaluatorAdapter for CemCssSelectorEvaluator {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::CssSelector
    }

    fn evaluate(
        &self,
        request: QueryExecutionRequest<'_>,
    ) -> Result<QueryExecutionResult, Vec<Diagnostic>> {
        if let Err(error) = request.validate_contract() {
            let fact = CssSelectorFact {
                kind: CssSelectorFactKind::InputUnsupported,
                source_range: None,
                message: error.to_string(),
                value: None,
            };
            return Err(css_selector_fact_diagnostics(
                request.query_ast_owner.source_uri(),
                &[fact],
            ));
        }
        let Some(expression) = request
            .query_ast_owner
            .as_any()
            .downcast_ref::<CssSelectorExpressionAst>()
        else {
            let fact = CssSelectorFact {
                kind: CssSelectorFactKind::InputUnsupported,
                source_range: None,
                message: "CSS selector evaluator requires the package-owned selector AST"
                    .to_owned(),
                value: Some(request.query_ast_owner.representation_id().to_owned()),
            };
            return Err(css_selector_fact_diagnostics(
                request.query_ast_owner.source_uri(),
                &[fact],
            ));
        };
        if expression.identity.namespaces != *request.namespace_bindings {
            let fact = CssSelectorFact {
                kind: CssSelectorFactKind::InputUnsupported,
                source_range: expression
                    .selector_list
                    .as_ref()
                    .map(|selector_list| selector_list.source_range),
                message: "CSS selector execution namespace bindings differ from the parsed query static context"
                    .to_owned(),
                value: Some("namespace-context-mismatch".to_owned()),
            };
            return Err(css_selector_fact_diagnostics(
                &expression.source.uri,
                &[fact],
            ));
        }
        let parse_diagnostics =
            css_selector_fact_diagnostics(&expression.source.uri, &expression.facts);
        if parse_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            return Err(parse_diagnostics);
        }
        let Some(selector_list) = expression.selector_list.as_ref() else {
            let fact = CssSelectorFact {
                kind: CssSelectorFactKind::ParseInvalid,
                source_range: None,
                message: "CSS selector evaluator requires a typed selector list".to_owned(),
                value: None,
            };
            return Err(css_selector_fact_diagnostics(
                &expression.source.uri,
                &[fact],
            ));
        };
        let Some(owner) = request
            .input_ast_owner
            .as_any()
            .downcast_ref::<CssSelectorElementTreeOwner>()
        else {
            let fact = CssSelectorFact {
                kind: CssSelectorFactKind::InputUnsupported,
                source_range: None,
                message: "CSS selector evaluator requires a borrowed lifecycle element-tree owner"
                    .to_owned(),
                value: Some(request.input_ast_owner.representation_id().to_owned()),
            };
            return Err(css_selector_fact_diagnostics(
                &expression.source.uri,
                &[fact],
            ));
        };

        let mut state = CssSelectorEvaluationState {
            limits: request.limits,
            observed_work_units: 0,
            abort_signal: request.abort_signal,
            expression_range: selector_list.source_range,
        };
        let mut matched_handles = BTreeSet::new();
        let mut matches = Vec::new();
        for node in owner.all_elements() {
            let matched = selector_list
                .selectors
                .iter()
                .try_fold(false, |matched, selector| {
                    if matched {
                        Ok(true)
                    } else {
                        matches_selector(selector, &node, &mut state)
                    }
                });
            let matched = match matched {
                Ok(matched) => matched,
                Err(fact) => {
                    return Err(css_selector_fact_diagnostics(
                        &expression.source.uri,
                        &[fact],
                    ))
                }
            };
            if !matched || !matched_handles.insert(node.handle) {
                continue;
            }
            if request
                .limits
                .max_result_items
                .is_some_and(|limit| matches.len() as u64 >= limit)
            {
                let fact = CssSelectorFact {
                    kind: CssSelectorFactKind::BudgetExceeded,
                    source_range: Some(selector_list.source_range),
                    message: format!(
                        "CSS selector result exceeded the configured result item limit of {}",
                        request.limits.max_result_items.unwrap()
                    ),
                    value: request
                        .limits
                        .max_result_items
                        .map(|value| value.to_string()),
                };
                return Err(css_selector_fact_diagnostics(
                    &expression.source.uri,
                    &[fact],
                ));
            }
            matches.push(CssSelectorMatchedNode {
                node_id: node.node_id(),
                local_name: node.local_name().to_owned(),
                namespace_uri: node.namespace_uri().to_owned(),
                document_order: node.document_order(),
                source_map: node.source_map(),
                native_node: node,
            });
        }
        matches.sort_by_key(|matched| matched.document_order);
        let result_artifact: Arc<dyn QueryNativeResult> = Arc::new(CssSelectorResultArtifact {
            content_type: "application/vnd.cem.query-result+css-selector".to_owned(),
            schema_uri: CSS_SELECTOR_SCHEMA_URI.to_owned(),
            language_version: CSS_SELECTOR_LANGUAGE_VERSION.to_owned(),
            matches,
            observed_work_units: state.observed_work_units,
            source_map: expression.source_map.clone(),
        });
        QueryExecutionResult::new(
            QueryLanguage::CssSelector,
            expression.identity.clone(),
            Arc::new(owner.clone()),
            result_artifact,
            expression.source_map.clone(),
        )
        .map_err(|error| {
            css_selector_fact_diagnostics(
                &expression.source.uri,
                &[CssSelectorFact {
                    kind: CssSelectorFactKind::InputUnsupported,
                    source_range: Some(selector_list.source_range),
                    message: error.to_string(),
                    value: None,
                }],
            )
        })
    }
}

struct CssSelectorEvaluationState<'a> {
    limits: QueryExecutionLimits,
    observed_work_units: u64,
    abort_signal: &'a crate::scheduler::AbortSignal,
    expression_range: CssSelectorSourceRange,
}

impl CssSelectorEvaluationState<'_> {
    fn consume(&mut self) -> Result<(), CssSelectorFact> {
        if self.abort_signal.is_aborted() {
            return Err(CssSelectorFact {
                kind: CssSelectorFactKind::CapabilityMissing,
                source_range: Some(self.expression_range),
                message: "CSS selector execution was cancelled by the host".to_owned(),
                value: Some("cancellation".to_owned()),
            });
        }
        self.observed_work_units = self.observed_work_units.saturating_add(1);
        if self
            .limits
            .max_work_units
            .is_some_and(|limit| self.observed_work_units > limit)
        {
            return Err(CssSelectorFact {
                kind: CssSelectorFactKind::BudgetExceeded,
                source_range: Some(self.expression_range),
                message: format!(
                    "CSS selector evaluation consumed {} work units, exceeding the configured limit of {}",
                    self.observed_work_units,
                    self.limits.max_work_units.unwrap()
                ),
                value: self.limits.max_work_units.map(|value| value.to_string()),
            });
        }
        Ok(())
    }
}

fn matches_selector(
    selector: &CssSelectorAst,
    node: &CssSelectorNativeNode,
    state: &mut CssSelectorEvaluationState<'_>,
) -> Result<bool, CssSelectorFact> {
    if selector.compounds.is_empty() {
        return Ok(false);
    }
    matches_selector_at(selector, selector.compounds.len() - 1, node, None, state)
}

fn matches_selector_at(
    selector: &CssSelectorAst,
    compound_index: usize,
    node: &CssSelectorNativeNode,
    relative_anchor: Option<&CssSelectorNativeNode>,
    state: &mut CssSelectorEvaluationState<'_>,
) -> Result<bool, CssSelectorFact> {
    if !matches_compound(&selector.compounds[compound_index], node, state)? {
        return Ok(false);
    }
    if compound_index == 0 {
        return relative_anchor.map_or(Ok(true), |anchor| {
            matches_relative_anchor(
                selector
                    .leading_combinator
                    .unwrap_or(CssSelectorCombinator::Descendant),
                anchor,
                node,
                state,
            )
        });
    }
    let combinator = selector.combinators[compound_index - 1];
    match combinator {
        CssSelectorCombinator::Child => match node.parent(state)? {
            Some(parent) => matches_selector_at(
                selector,
                compound_index - 1,
                &parent,
                relative_anchor,
                state,
            ),
            None => Ok(false),
        },
        CssSelectorCombinator::Descendant => {
            let mut current = node.parent(state)?;
            while let Some(parent) = current {
                if matches_selector_at(
                    selector,
                    compound_index - 1,
                    &parent,
                    relative_anchor,
                    state,
                )? {
                    return Ok(true);
                }
                current = parent.parent(state)?;
            }
            Ok(false)
        }
        CssSelectorCombinator::NextSibling => node
            .previous_siblings(state)?
            .into_iter()
            .last()
            .map_or(Ok(false), |sibling| {
                matches_selector_at(
                    selector,
                    compound_index - 1,
                    &sibling,
                    relative_anchor,
                    state,
                )
            }),
        CssSelectorCombinator::SubsequentSibling => {
            for sibling in node.previous_siblings(state)?.into_iter().rev() {
                if matches_selector_at(
                    selector,
                    compound_index - 1,
                    &sibling,
                    relative_anchor,
                    state,
                )? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

fn matches_compound(
    compound: &CssSelectorCompoundAst,
    node: &CssSelectorNativeNode,
    state: &mut CssSelectorEvaluationState<'_>,
) -> Result<bool, CssSelectorFact> {
    state.consume()?;
    for simple in &compound.simple_selectors {
        if !matches_simple(simple, node, state)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matches_simple(
    simple: &CssSelectorSimpleSelector,
    node: &CssSelectorNativeNode,
    state: &mut CssSelectorEvaluationState<'_>,
) -> Result<bool, CssSelectorFact> {
    match simple {
        CssSelectorSimpleSelector::Type {
            namespace,
            local_name,
            universal,
            ..
        } => Ok(namespace_matches(namespace, node.namespace_uri())
            && (*universal
                || if node.is_html_element() {
                    node.local_name().eq_ignore_ascii_case(local_name)
                } else {
                    node.local_name() == local_name
                })),
        CssSelectorSimpleSelector::Id { value, .. } => Ok(node
            .attribute_values(&CssSelectorNamespace::None, "id")
            .iter()
            .any(|actual| *actual == value)),
        CssSelectorSimpleSelector::Class { value, .. } => Ok(node
            .attribute_values(&CssSelectorNamespace::None, "class")
            .iter()
            .any(|actual| actual.split_whitespace().any(|class| class == value))),
        CssSelectorSimpleSelector::Attribute {
            namespace,
            local_name,
            operator,
            value,
            modifier,
            ..
        } => {
            let values = node.attribute_values(namespace, local_name);
            if operator.is_none() {
                return Ok(!values.is_empty());
            }
            let expected = value.as_deref().unwrap_or_default();
            Ok(values.iter().any(|actual| {
                attribute_value_matches(
                    actual,
                    expected,
                    operator.unwrap(),
                    modifier.as_ref().copied(),
                )
            }))
        }
        CssSelectorSimpleSelector::PseudoClass {
            name,
            selectors: Some(selectors),
            ..
        } if matches!(name.as_str(), "is" | "where") => {
            for selector in &selectors.selectors {
                if matches_selector(selector, node, state)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        CssSelectorSimpleSelector::PseudoClass {
            name,
            selectors: Some(selectors),
            ..
        } if name == "not" => {
            for selector in &selectors.selectors {
                if matches_selector(selector, node, state)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        CssSelectorSimpleSelector::PseudoClass {
            name,
            selectors: Some(selectors),
            ..
        } if name == "has" => {
            for selector in &selectors.selectors {
                for candidate in node.owner.all_elements() {
                    if candidate == *node {
                        continue;
                    }
                    state.consume()?;
                    if matches_selector_at(
                        selector,
                        selector.compounds.len() - 1,
                        &candidate,
                        Some(node),
                        state,
                    )? {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }
        CssSelectorSimpleSelector::PseudoClass { .. } => Ok(false),
    }
}

fn matches_relative_anchor(
    combinator: CssSelectorCombinator,
    anchor: &CssSelectorNativeNode,
    leftmost_match: &CssSelectorNativeNode,
    state: &mut CssSelectorEvaluationState<'_>,
) -> Result<bool, CssSelectorFact> {
    state.consume()?;
    match combinator {
        CssSelectorCombinator::Child => Ok(leftmost_match.parent(state)?.as_ref() == Some(anchor)),
        CssSelectorCombinator::Descendant => {
            let mut current = leftmost_match.parent(state)?;
            while let Some(parent) = current {
                if parent == *anchor {
                    return Ok(true);
                }
                current = parent.parent(state)?;
            }
            Ok(false)
        }
        CssSelectorCombinator::NextSibling => {
            Ok(leftmost_match.previous_siblings(state)?.last() == Some(anchor))
        }
        CssSelectorCombinator::SubsequentSibling => Ok(leftmost_match
            .previous_siblings(state)?
            .iter()
            .any(|candidate| candidate == anchor)),
    }
}

fn namespace_matches(namespace: &CssSelectorNamespace, actual: &str) -> bool {
    match namespace {
        CssSelectorNamespace::Any => true,
        CssSelectorNamespace::None => actual.is_empty(),
        CssSelectorNamespace::Default { namespace_uri }
        | CssSelectorNamespace::Named { namespace_uri, .. } => namespace_uri == actual,
    }
}

fn attribute_value_matches(
    actual: &str,
    expected: &str,
    operator: CssSelectorAttributeOperator,
    modifier: Option<CssSelectorAttributeModifier>,
) -> bool {
    let normalized;
    let normalized_expected;
    let (actual, expected) = if modifier == Some(CssSelectorAttributeModifier::AsciiInsensitive) {
        normalized = actual.to_ascii_lowercase();
        normalized_expected = expected.to_ascii_lowercase();
        (normalized.as_str(), normalized_expected.as_str())
    } else {
        (actual, expected)
    };
    match operator {
        CssSelectorAttributeOperator::Equals => actual == expected,
        CssSelectorAttributeOperator::Includes => {
            actual.split_whitespace().any(|part| part == expected)
        }
        CssSelectorAttributeOperator::DashMatch => {
            actual == expected
                || actual
                    .strip_prefix(expected)
                    .is_some_and(|suffix| suffix.starts_with('-'))
        }
        CssSelectorAttributeOperator::PrefixMatch => actual.starts_with(expected),
        CssSelectorAttributeOperator::SuffixMatch => actual.ends_with(expected),
        CssSelectorAttributeOperator::SubstringMatch => actual.contains(expected),
    }
}
