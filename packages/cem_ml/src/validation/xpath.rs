use crate::schema::registry::content_type_essence;
pub use crate::schema::registry::{XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI};
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, SourceId};
use crate::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use xee_xpath_ast::ast::XPath;
use xee_xpath_ast::{Namespaces, ParserError, VariableNames, XPathParserContext};
use xee_xpath_lexer::Token as XeeToken;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct XPathSourcePosition {
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathExpectedResult {
    pub sequence_type: String,
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
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
    if matches!(attachment, XPathAttachment::Host(_)) {
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
    }

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
