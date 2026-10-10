//! Sequence contracts and lifecycle-owned executable datatype snapshots.
use super::{
    datatype_registry::DatatypeSource, datatype_validation::ValueRepresentation,
    declaration_references::SchemaDeclarationNode,
};
use crate::{
    diagnostics::Diagnostic,
    operation_control::{ControlError, ExecutionScopeId, OperationControl},
    parser::document::CemDocument,
    schema::{datatype_registry::DatatypeDependencySite, reference_policy::ReferenceOccurrence},
    source_map::SourceMapStack,
    value::reference_resolution::ReferenceResolutionIssueKind,
};
use std::{any::Any, fmt::Debug, ops::Range, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemBounds {
    pub min: usize,
    pub max: Option<usize>,
}
impl ItemBounds {
    pub fn new(min: usize, max: Option<usize>) -> Result<Self, &'static str> {
        if max.is_some_and(|max| max < min) {
            Err("empty-cardinality-intersection")
        } else {
            Ok(Self { min, max })
        }
    }
    pub fn admits(self, count: usize) -> bool {
        count >= self.min && self.max.is_none_or(|max| count <= max)
    }
    pub fn intersect(self, other: Self) -> Result<Self, &'static str> {
        Self::new(
            self.min.max(other.min),
            match (self.max, other.max) {
                (Some(a), Some(b)) => Some(a.min(b)),
                (a, b) => a.or(b),
            },
        )
    }
}
#[derive(Debug, Clone)]
pub struct LexicalInput {
    pub text: Arc<str>,
    pub source: SourceMapStack,
}
impl LexicalInput {
    pub fn new(text: Arc<str>, source: SourceMapStack) -> Self {
        Self { text, source }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct TokenizationLimits {
    pub max_bytes: usize,
    pub max_tokens: usize,
    /// Includes every inherited admission check and the selected tokenizer.
    pub max_tokenizers: usize,
}
impl Default for TokenizationLimits {
    fn default() -> Self {
        Self {
            max_bytes: 1_048_576,
            max_tokens: 100_000,
            max_tokenizers: 256,
        }
    }
}
#[derive(Debug, Clone)]
pub enum TokenizationError {
    AbsentInput,
    Rejected,
    IncompatibleReplacement,
    Limit,
    InvalidSpan,
    UnaccountedInput,
    Control(ControlError),
    Implementation(String),
}
pub struct TokenizationRequest<'a> {
    pub input: &'a LexicalInput,
    pub control: &'a OperationControl,
    pub scope: ExecutionScopeId,
    pub limits: TokenizationLimits,
}
pub trait Tokenizer: Debug + Send + Sync {
    /// Return ordered source byte spans; normalization and item conversion are separate.
    /// Implementations must poll control and bound their allocations/work.
    fn tokenize(
        &self,
        request: TokenizationRequest<'_>,
    ) -> Result<Vec<Range<usize>>, TokenizationError>;
    /// Explicit separator policy. The default permits Unicode whitespace only.
    fn accepts_separator(&self, text: &str) -> bool {
        text.chars().all(char::is_whitespace)
    }
}
#[derive(Debug, Clone)]
pub struct RegisteredTokenizer {
    id: String,
    implementation: Arc<dyn Tokenizer>,
    inherited: Vec<Arc<dyn Tokenizer>>,
}
#[derive(Debug, Clone)]
pub struct TokenizedInput {
    pub input: LexicalInput,
    pub tokens: Vec<Range<usize>>,
}
impl RegisteredTokenizer {
    pub fn new(
        id: impl Into<String>,
        implementation: impl Tokenizer + 'static,
    ) -> Result<Self, &'static str> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err("empty-tokenizer-id");
        }
        Ok(Self {
            id,
            implementation: Arc::new(implementation),
            inherited: vec![],
        })
    }
    /// A capability value only; a host must register it for an exact list source.
    pub fn whitespace() -> Self {
        Self::new("cem:unicode-whitespace/1", Whitespace).unwrap()
    }
    pub fn identity(&self) -> &str {
        &self.id
    }
    /// Check every inherited tokenizer against the same original input. This
    /// preserves exact token boundaries; it never feeds one result into another.
    pub fn checked_replacement(mut self, base: &Self) -> Self {
        let mut inherited = base.inherited.clone();
        inherited.push(base.implementation.clone());
        inherited.append(&mut self.inherited);
        self.inherited = inherited;
        self
    }
    pub fn invocations(&self) -> usize {
        self.inherited.len().saturating_add(1)
    }
    pub fn tokenize(
        &self,
        input: Option<LexicalInput>,
        control: &OperationControl,
        scope: ExecutionScopeId,
        limits: TokenizationLimits,
    ) -> Result<TokenizedInput, TokenizationError> {
        control
            .check_scope(scope)
            .map_err(TokenizationError::Control)?;
        let input = input.ok_or(TokenizationError::AbsentInput)?;
        if input.text.len() > limits.max_bytes || self.invocations() > limits.max_tokenizers {
            return Err(TokenizationError::Limit);
        }
        let mut expected = None;
        for implementation in self
            .inherited
            .iter()
            .chain(std::iter::once(&self.implementation))
        {
            control
                .check_scope(scope)
                .map_err(TokenizationError::Control)?;
            let outcome = implementation.tokenize(TokenizationRequest {
                input: &input,
                control,
                scope,
                limits,
            });
            control
                .check_scope(scope)
                .map_err(TokenizationError::Control)?;
            let tokens = outcome?;
            if tokens.len() > limits.max_tokens {
                return Err(TokenizationError::Limit);
            }
            let mut end = 0;
            for (i, span) in tokens.iter().enumerate() {
                if i % 64 == 0 {
                    control
                        .check_scope(scope)
                        .map_err(TokenizationError::Control)?;
                }
                if span.start < end
                    || span.start >= span.end
                    || span.end > input.text.len()
                    || !input.text.is_char_boundary(span.start)
                    || !input.text.is_char_boundary(span.end)
                {
                    return Err(TokenizationError::InvalidSpan);
                }
                if !implementation.accepts_separator(&input.text[end..span.start]) {
                    return Err(TokenizationError::UnaccountedInput);
                }
                end = span.end;
            }
            if !implementation.accepts_separator(&input.text[end..]) {
                return Err(TokenizationError::UnaccountedInput);
            }
            control
                .check_scope(scope)
                .map_err(TokenizationError::Control)?;
            if expected
                .as_ref()
                .is_some_and(|previous| previous != &tokens)
            {
                return Err(TokenizationError::IncompatibleReplacement);
            }
            expected = Some(tokens);
        }
        Ok(TokenizedInput {
            input,
            tokens: expected.expect("selected tokenizer"),
        })
    }
}

#[derive(Debug)]
struct Whitespace;
impl Tokenizer for Whitespace {
    fn tokenize(&self, r: TokenizationRequest<'_>) -> Result<Vec<Range<usize>>, TokenizationError> {
        let mut spans = vec![];
        let mut start = None;
        for (count, (offset, c)) in r.input.text.char_indices().enumerate() {
            if count % 64 == 0 {
                r.control
                    .check_scope(r.scope)
                    .map_err(TokenizationError::Control)?;
            }
            if c.is_whitespace() {
                if let Some(start) = start.take() {
                    if spans.len() >= r.limits.max_tokens {
                        return Err(TokenizationError::Limit);
                    }
                    spans.push(start..offset);
                }
            } else if start.is_none() {
                start = Some(offset);
            }
        }
        if let Some(start) = start {
            if spans.len() >= r.limits.max_tokens {
                return Err(TokenizationError::Limit);
            }
            spans.push(start..r.input.text.len());
        }
        Ok(spans)
    }
}
/// The executable owner is retained in the model without depending on a query backend.
/// Implementations are supplied by an explicit host compiler, never deserialized.
pub trait CompiledDatatypeContract: Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn source(&self) -> &DatatypeSource;
    fn representation(&self) -> ValueRepresentation;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatatypeIssueState {
    Pending,
    Invalid,
}
#[derive(Debug, Clone)]
pub struct DatatypeCompilationIssue {
    pub code: &'static str,
    pub state: DatatypeIssueState,
    pub source: SchemaDeclarationNode,
    pub related: Option<SchemaDeclarationNode>,
}
#[derive(Debug, Clone)]
pub struct DatatypeReferenceIssue {
    pub source: Option<SchemaDeclarationNode>,
    pub kind: ReferenceResolutionIssueKind,
    pub occurrence: ReferenceOccurrence,
    pub reason: String,
}
#[derive(Debug, Clone)]
pub struct DatatypeCompilation {
    owner: Arc<CemDocument>,
    pub sources: Vec<DatatypeSource>,
    pub contracts: Vec<Arc<dyn CompiledDatatypeContract>>,
    pub issues: Vec<DatatypeCompilationIssue>,
    pub dependency_sites: Vec<DatatypeDependencySite>,
    pub reference_issues: Vec<DatatypeReferenceIssue>,
    pub diagnostics: Vec<Diagnostic>,
}
impl DatatypeCompilation {
    pub fn new(owner: Arc<CemDocument>) -> Self {
        Self {
            owner,
            sources: vec![],
            contracts: vec![],
            issues: vec![],
            dependency_sites: vec![],
            reference_issues: vec![],
            diagnostics: vec![],
        }
    }
    pub fn owner(&self) -> &Arc<CemDocument> {
        &self.owner
    }
    pub fn matches_owner(&self, owner: &Arc<CemDocument>) -> bool {
        Arc::ptr_eq(&self.owner, owner)
    }
    pub fn is_ready(&self) -> bool {
        self.issues.is_empty()
            && self.sources.iter().all(|s| {
                self.contracts.iter().any(|c| {
                    c.source().declaration().identity() == s.declaration().identity()
                        && c.source().scope().identity() == s.scope().identity()
                })
            })
    }
}
