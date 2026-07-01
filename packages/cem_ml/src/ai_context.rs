//! AI-facing context projections over canonical CEM AST data.
//!
//! This module deliberately projects from [`CemDocument`] instead of replacing
//! canonical AST/DOM/event projections. Records carry canonical `cem-ast://`
//! refs and source maps so hosts can expand back to authoritative data.

use crate::diagnostics::{Diagnostic, Severity};
use crate::parser::document::CemDocument;
use crate::parser::{AstNodeId, CemAstNode};
use crate::source::ByteRange;
use crate::source_map::{FrameSpan, SourceMapStack};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const AI_CONTEXT_UNSUPPORTED_PROFILE: &str = "cem.ai_context.unsupported_profile";
pub const AI_CONTEXT_MISSING_EXPANSION_TARGET: &str = "cem.ai_context.missing_expansion_target";
pub const AI_CONTEXT_BUDGET_OMISSION: &str = "cem.ai_context.budget_omission";
pub const AI_CONTEXT_UNSAFE_INSTRUCTION_MIX: &str = "cem.ai_context.unsafe_instruction_mix";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextProjectionKind {
    #[default]
    ContextPack,
    EntityGraph,
    SemanticTokens,
    ContextFragment,
    EmbeddingRecord,
}

impl AiContextProjectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ContextPack => "ai-context-pack",
            Self::EntityGraph => "ai-entity-graph",
            Self::SemanticTokens => "ai-semantic-tokens",
            Self::ContextFragment => "ai-context-fragment",
            Self::EmbeddingRecord => "ai-embedding-record",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextProfile {
    Summary,
    Navigation,
    Refactor,
    TokenAuthoring,
    Diagnostic,
    Embedding,
}

impl AiContextProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Navigation => "navigation",
            Self::Refactor => "refactor",
            Self::TokenAuthoring => "token-authoring",
            Self::Diagnostic => "diagnostic",
            Self::Embedding => "embedding",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "summary" => Some(Self::Summary),
            "navigation" => Some(Self::Navigation),
            "refactor" => Some(Self::Refactor),
            "token-authoring" => Some(Self::TokenAuthoring),
            "diagnostic" => Some(Self::Diagnostic),
            "embedding" => Some(Self::Embedding),
            _ => None,
        }
    }

    pub fn default_budgets(self) -> AiContextBudgets {
        match self {
            Self::Summary => AiContextBudgets {
                max_nodes: Some(128),
                max_tokens: Some(4_096),
                max_characters: Some(16_384),
                max_depth: Some(12),
                max_diagnostics: Some(32),
                max_source_excerpt_chars: Some(256),
            },
            Self::Navigation => AiContextBudgets {
                max_nodes: Some(512),
                max_tokens: Some(8_192),
                max_characters: Some(32_768),
                max_depth: Some(32),
                max_diagnostics: Some(32),
                max_source_excerpt_chars: Some(80),
            },
            Self::Refactor => AiContextBudgets {
                max_nodes: Some(1_024),
                max_tokens: Some(16_384),
                max_characters: Some(65_536),
                max_depth: Some(64),
                max_diagnostics: Some(64),
                max_source_excerpt_chars: Some(512),
            },
            Self::TokenAuthoring => AiContextBudgets {
                max_nodes: Some(512),
                max_tokens: Some(12_288),
                max_characters: Some(49_152),
                max_depth: Some(48),
                max_diagnostics: Some(64),
                max_source_excerpt_chars: Some(256),
            },
            Self::Diagnostic => AiContextBudgets {
                max_nodes: Some(768),
                max_tokens: Some(16_384),
                max_characters: Some(65_536),
                max_depth: Some(64),
                max_diagnostics: Some(128),
                max_source_excerpt_chars: Some(512),
            },
            Self::Embedding => AiContextBudgets {
                max_nodes: Some(256),
                max_tokens: Some(6_144),
                max_characters: Some(24_576),
                max_depth: Some(32),
                max_diagnostics: Some(32),
                max_source_excerpt_chars: Some(512),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextBudgets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_nodes: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_characters: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_diagnostics: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_source_excerpt_chars: Option<usize>,
}

impl AiContextBudgets {
    pub fn with_profile_defaults(self, profile: Option<AiContextProfile>) -> Self {
        let defaults = profile
            .map(AiContextProfile::default_budgets)
            .unwrap_or_default();
        Self {
            max_nodes: self.max_nodes.or(defaults.max_nodes),
            max_tokens: self.max_tokens.or(defaults.max_tokens),
            max_characters: self.max_characters.or(defaults.max_characters),
            max_depth: self.max_depth.or(defaults.max_depth),
            max_diagnostics: self.max_diagnostics.or(defaults.max_diagnostics),
            max_source_excerpt_chars: self
                .max_source_excerpt_chars
                .or(defaults.max_source_excerpt_chars),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextHostMetadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextToolMetadata {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextProjectionRequest {
    #[serde(default)]
    pub kind: AiContextProjectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<AstNodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default)]
    pub budgets: AiContextBudgets,
    #[serde(default)]
    pub include_source_excerpts: bool,
    #[serde(default)]
    pub allow_instruction_mixing: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<AiContextHostMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<AiContextToolMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextProjection {
    pub kind: AiContextProjectionKind,
    pub metadata: AiContextProjectionMetadata,
    pub records: Vec<AiContextRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextProjectionMetadata {
    pub canonical_projection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<AiContextProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_ref: Option<String>,
    pub record_count: usize,
    pub budgets: AiContextBudgets,
    pub usage: AiContextUsage,
    pub lossiness: AiContextLossiness,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expansion_refs: Vec<AiContextExpansionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<AiContextHostMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<AiContextToolMetadata>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextUsage {
    pub nodes: usize,
    pub tokens: usize,
    pub characters: usize,
    pub depth: usize,
    pub diagnostics: usize,
    pub source_excerpt_characters: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextLossiness {
    pub lossy: bool,
    pub omitted_records: usize,
    pub omitted_characters: usize,
    pub omitted_source_excerpts: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<AiContextLossinessReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextLossinessReason {
    Budget,
    SourceExcerpt,
    ProfileFallback,
    MissingExpansionTarget,
    DiagnosticBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextExpansionRef {
    pub canonical_projection: String,
    pub canonical_ref: String,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextRecordKind {
    Document,
    Element,
    Attribute,
    Text,
    Whitespace,
    Comment,
    ProcessingInstruction,
    Cdata,
    RawText,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextBoundary {
    Data,
    Instruction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextRecord {
    pub id: String,
    pub canonical_ref: String,
    pub expansion_ref: AiContextExpansionRef,
    pub node_id: AstNodeId,
    pub kind: AiContextRecordKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facets: Vec<String>,
    pub boundary: AiContextBoundary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_ranges: Vec<ByteRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_map: Option<SourceMapStack>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AiContextTaskEvalKind {
    #[default]
    Retrieval,
    EditPrecision,
    TokenBudgetValue,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextTaskEvalRequest {
    #[serde(default)]
    pub kind: AiContextTaskEvalKind,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relevant_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edit_target_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiContextTaskEvalResult {
    pub kind: AiContextTaskEvalKind,
    pub name: String,
    pub passed: bool,
    pub record_count: usize,
    pub token_count: usize,
    pub relevant_total: usize,
    pub relevant_found: usize,
    pub edit_targets_total: usize,
    pub edit_targets_found: usize,
    pub source_mapped_edit_targets: usize,
    pub retrieval_precision: f64,
    pub retrieval_recall: f64,
    pub edit_precision: f64,
    pub token_budget_value: f64,
    pub within_token_budget: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub found_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_edit_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_source_mapped_edit_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_hit_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AiContextNodeCandidate {
    node_id: AstNodeId,
    depth: usize,
}

#[derive(Debug, Default)]
struct AiContextBudgetStats {
    omitted_records: usize,
    omitted_characters: usize,
    omitted_source_excerpts: usize,
    omitted_source_excerpt_characters: usize,
    source_excerpt_characters: usize,
    depth_omissions: usize,
}

#[derive(Debug, Default)]
struct AiContextRecordBuildStats {
    omitted_source_excerpts: usize,
    omitted_source_excerpt_characters: usize,
    source_excerpt_characters: usize,
}

pub fn project_cem_document_for_ai(
    document: &CemDocument,
    request: AiContextProjectionRequest,
) -> AiContextProjection {
    let mut diagnostics = Vec::new();
    let mut lossiness_reasons = Vec::new();
    let profile = resolve_ai_context_profile(&request, &mut diagnostics, &mut lossiness_reasons);
    let budgets = request.budgets.with_profile_defaults(profile);
    let root = request.root.unwrap_or(0);
    let Some(root_node) = document.get(root) else {
        diagnostics.push(ai_context_diagnostic(
            AI_CONTEXT_MISSING_EXPANSION_TARGET,
            Severity::Error,
            format!("AI context root node `{root}` does not exist in the canonical AST"),
            Some(ai_context_canonical_ref(root)),
            None,
        ));
        lossiness_reasons.push(AiContextLossinessReason::MissingExpansionTarget);
        limit_ai_context_diagnostics(&mut diagnostics, budgets, &mut lossiness_reasons);
        return AiContextProjection {
            kind: request.kind,
            metadata: AiContextProjectionMetadata {
                canonical_projection: "cem-ast".to_owned(),
                profile,
                root_ref: None,
                record_count: 0,
                budgets,
                usage: AiContextUsage {
                    diagnostics: diagnostics.len(),
                    ..AiContextUsage::default()
                },
                lossiness: ai_context_lossiness(
                    &lossiness_reasons,
                    &AiContextBudgetStats::default(),
                ),
                expansion_refs: Vec::new(),
                host: request.host,
                tool: request.tool,
            },
            records: Vec::new(),
            diagnostics,
        };
    };

    let parents = ai_context_parent_map(document);
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    let mut budget_stats = AiContextBudgetStats::default();
    collect_ai_context_node_ids(
        document,
        root,
        0,
        budgets.max_depth,
        &mut seen,
        &mut candidates,
        &mut budget_stats,
    );
    let eligible = candidates
        .iter()
        .copied()
        .filter(|candidate| {
            document
                .get(candidate.node_id)
                .is_some_and(|node| ai_context_includes_node(request.kind, node))
        })
        .collect::<Vec<_>>();
    let (selected, mut usage) =
        select_ai_context_candidates(document, &eligible, budgets, &mut budget_stats);
    let included_ids = selected
        .iter()
        .map(|candidate| candidate.node_id)
        .collect::<BTreeSet<_>>();

    let mut records = Vec::new();
    for candidate in selected {
        let Some(node) = document.get(candidate.node_id) else {
            continue;
        };
        if !included_ids.contains(&candidate.node_id) {
            continue;
        }
        let (record, record_stats) = ai_context_record_for_node(
            document,
            request.kind,
            profile,
            node,
            parents.get(&candidate.node_id).copied(),
            &included_ids,
            request.include_source_excerpts,
            budgets.max_source_excerpt_chars,
        );
        budget_stats.omitted_source_excerpts += record_stats.omitted_source_excerpts;
        budget_stats.omitted_source_excerpt_characters +=
            record_stats.omitted_source_excerpt_characters;
        budget_stats.source_excerpt_characters += record_stats.source_excerpt_characters;
        records.push(record);
    }
    usage.source_excerpt_characters = budget_stats.source_excerpt_characters;

    emit_ai_context_budget_diagnostics(
        &budget_stats,
        budgets,
        root_node,
        &mut diagnostics,
        &mut lossiness_reasons,
    );
    emit_ai_context_instruction_mix_diagnostic(
        &records,
        request.allow_instruction_mixing,
        root_node,
        &mut diagnostics,
    );
    limit_ai_context_diagnostics(&mut diagnostics, budgets, &mut lossiness_reasons);
    usage.diagnostics = diagnostics.len();

    AiContextProjection {
        kind: request.kind,
        metadata: AiContextProjectionMetadata {
            canonical_projection: "cem-ast".to_owned(),
            profile,
            root_ref: document.get(root).map(|_| ai_context_canonical_ref(root)),
            record_count: records.len(),
            budgets,
            usage,
            lossiness: ai_context_lossiness(&lossiness_reasons, &budget_stats),
            expansion_refs: vec![ai_context_expansion_ref(root, "root")],
            host: request.host,
            tool: request.tool,
        },
        records,
        diagnostics,
    }
}

pub fn evaluate_ai_context_task_projection(
    projection: &AiContextProjection,
    request: AiContextTaskEvalRequest,
) -> AiContextTaskEvalResult {
    let record_refs = projection
        .records
        .iter()
        .map(|record| record.canonical_ref.as_str())
        .collect::<BTreeSet<_>>();
    let source_mapped_refs = projection
        .records
        .iter()
        .filter(|record| !record.source_ranges.is_empty())
        .map(|record| record.canonical_ref.as_str())
        .collect::<BTreeSet<_>>();
    let found_refs = request
        .relevant_refs
        .iter()
        .filter(|reference| record_refs.contains(reference.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let missing_refs = request
        .relevant_refs
        .iter()
        .filter(|reference| !record_refs.contains(reference.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let edit_target_refs = if request.edit_target_refs.is_empty() {
        request.relevant_refs.as_slice()
    } else {
        request.edit_target_refs.as_slice()
    };
    let edit_targets_found = edit_target_refs
        .iter()
        .filter(|reference| record_refs.contains(reference.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let missing_edit_refs = edit_target_refs
        .iter()
        .filter(|reference| !record_refs.contains(reference.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let source_mapped_edit_refs = edit_target_refs
        .iter()
        .filter(|reference| source_mapped_refs.contains(reference.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let missing_source_mapped_edit_refs = edit_target_refs
        .iter()
        .filter(|reference| !source_mapped_refs.contains(reference.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let protected_hit_refs = request
        .protected_refs
        .iter()
        .filter(|reference| record_refs.contains(reference.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let retrieval_precision = ai_context_ratio(found_refs.len(), projection.records.len());
    let retrieval_recall = ai_context_ratio(found_refs.len(), request.relevant_refs.len());
    let edit_precision = ai_context_ratio(
        source_mapped_edit_refs.len(),
        source_mapped_edit_refs.len() + protected_hit_refs.len(),
    );
    let token_count = projection.metadata.usage.tokens;
    let token_budget_value = if token_count == 0 {
        found_refs.len() as f64
    } else {
        found_refs.len() as f64 * 1_000.0 / token_count as f64
    };
    let within_token_budget = request
        .max_tokens
        .is_none_or(|max_tokens| token_count <= max_tokens);
    let passed = match request.kind {
        AiContextTaskEvalKind::Retrieval => {
            !request.relevant_refs.is_empty() && missing_refs.is_empty()
        }
        AiContextTaskEvalKind::EditPrecision => {
            !edit_target_refs.is_empty()
                && missing_edit_refs.is_empty()
                && missing_source_mapped_edit_refs.is_empty()
                && protected_hit_refs.is_empty()
        }
        AiContextTaskEvalKind::TokenBudgetValue => {
            within_token_budget
                && !request.relevant_refs.is_empty()
                && missing_refs.is_empty()
                && protected_hit_refs.is_empty()
        }
    };

    AiContextTaskEvalResult {
        kind: request.kind,
        name: request.name,
        passed,
        record_count: projection.records.len(),
        token_count,
        relevant_total: request.relevant_refs.len(),
        relevant_found: found_refs.len(),
        edit_targets_total: edit_target_refs.len(),
        edit_targets_found: edit_targets_found.len(),
        source_mapped_edit_targets: source_mapped_edit_refs.len(),
        retrieval_precision,
        retrieval_recall,
        edit_precision,
        token_budget_value,
        within_token_budget,
        found_refs,
        missing_refs,
        missing_edit_refs,
        missing_source_mapped_edit_refs,
        protected_hit_refs,
    }
}

fn ai_context_record_for_node(
    document: &CemDocument,
    projection: AiContextProjectionKind,
    profile: Option<AiContextProfile>,
    node: &CemAstNode,
    parent: Option<AstNodeId>,
    included_ids: &BTreeSet<AstNodeId>,
    include_source_excerpts: bool,
    max_source_excerpt_chars: Option<usize>,
) -> (AiContextRecord, AiContextRecordBuildStats) {
    let node_id = ai_context_node_id(node);
    let (source_excerpt, stats) =
        ai_context_source_excerpt(node, include_source_excerpts, max_source_excerpt_chars);
    (
        AiContextRecord {
            id: format!("{}:{}", projection.as_str(), node_id),
            canonical_ref: ai_context_canonical_ref(node_id),
            expansion_ref: ai_context_expansion_ref(node_id, "record"),
            node_id,
            kind: ai_context_record_kind(node),
            label: ai_context_node_label(node),
            text: ai_context_node_text(node),
            source_excerpt,
            parent_ref: parent
                .filter(|parent_id| included_ids.contains(parent_id))
                .map(ai_context_canonical_ref),
            child_refs: ai_context_child_ids(node)
                .into_iter()
                .filter(|child_id| included_ids.contains(child_id))
                .map(ai_context_canonical_ref)
                .collect(),
            attributes: ai_context_element_attributes(document, node),
            facets: ai_context_facets(projection, profile, node),
            boundary: ai_context_boundary(node),
            source_ranges: ai_context_source_ranges(node),
            source_map: ai_context_source_map(node).cloned(),
        },
        stats,
    )
}

fn collect_ai_context_node_ids(
    document: &CemDocument,
    node_id: AstNodeId,
    depth: usize,
    max_depth: Option<usize>,
    seen: &mut BTreeSet<AstNodeId>,
    output: &mut Vec<AiContextNodeCandidate>,
    budget_stats: &mut AiContextBudgetStats,
) {
    let Some(node) = document.get(node_id) else {
        return;
    };
    if max_depth.is_some_and(|limit| depth > limit) {
        budget_stats.omitted_records += 1;
        budget_stats.depth_omissions += 1;
        budget_stats.omitted_characters += ai_context_node_character_count(node);
        return;
    }
    if !seen.insert(node_id) {
        return;
    }
    output.push(AiContextNodeCandidate { node_id, depth });
    for child_id in ai_context_child_ids(node) {
        collect_ai_context_node_ids(
            document,
            child_id,
            depth + 1,
            max_depth,
            seen,
            output,
            budget_stats,
        );
    }
}

fn ai_context_parent_map(document: &CemDocument) -> BTreeMap<AstNodeId, AstNodeId> {
    let mut parents = BTreeMap::new();
    for node in document.iter() {
        let parent = ai_context_node_id(node);
        for child in ai_context_child_ids(node) {
            parents.insert(child, parent);
        }
    }
    parents
}

fn select_ai_context_candidates(
    document: &CemDocument,
    candidates: &[AiContextNodeCandidate],
    budgets: AiContextBudgets,
    budget_stats: &mut AiContextBudgetStats,
) -> (Vec<AiContextNodeCandidate>, AiContextUsage) {
    let mut selected = Vec::new();
    let mut usage = AiContextUsage::default();
    for candidate in candidates {
        let Some(node) = document.get(candidate.node_id) else {
            continue;
        };
        let character_count = ai_context_node_character_count(node);
        let token_count = ai_context_estimated_token_count(character_count);
        if budgets
            .max_nodes
            .is_some_and(|limit| selected.len() >= limit)
        {
            ai_context_count_omitted_node(node, budget_stats);
            continue;
        }
        if character_count > 0
            && budgets
                .max_characters
                .is_some_and(|limit| usage.characters + character_count > limit)
        {
            ai_context_count_omitted_node(node, budget_stats);
            continue;
        }
        if token_count > 0
            && budgets
                .max_tokens
                .is_some_and(|limit| usage.tokens + token_count > limit)
        {
            ai_context_count_omitted_node(node, budget_stats);
            continue;
        }

        selected.push(*candidate);
        usage.nodes += 1;
        usage.characters += character_count;
        usage.tokens += token_count;
        usage.depth = usage.depth.max(candidate.depth);
    }
    (selected, usage)
}

fn ai_context_count_omitted_node(node: &CemAstNode, budget_stats: &mut AiContextBudgetStats) {
    budget_stats.omitted_records += 1;
    budget_stats.omitted_characters += ai_context_node_character_count(node);
}

fn ai_context_ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        if numerator == 0 {
            1.0
        } else {
            0.0
        }
    } else {
        numerator as f64 / denominator as f64
    }
}

fn resolve_ai_context_profile(
    request: &AiContextProjectionRequest,
    diagnostics: &mut Vec<Diagnostic>,
    lossiness_reasons: &mut Vec<AiContextLossinessReason>,
) -> Option<AiContextProfile> {
    let Some(profile_name) = request.profile.as_deref() else {
        return None;
    };
    if let Some(profile) = AiContextProfile::parse(profile_name) {
        return Some(profile);
    }
    diagnostics.push(ai_context_diagnostic(
        AI_CONTEXT_UNSUPPORTED_PROFILE,
        Severity::Warning,
        format!("AI context profile `{profile_name}` is not supported; using projection defaults"),
        None,
        None,
    ));
    lossiness_reasons.push(AiContextLossinessReason::ProfileFallback);
    None
}

fn emit_ai_context_budget_diagnostics(
    budget_stats: &AiContextBudgetStats,
    budgets: AiContextBudgets,
    root_node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
    lossiness_reasons: &mut Vec<AiContextLossinessReason>,
) {
    if budget_stats.omitted_records > 0 {
        let depth_suffix = if budget_stats.depth_omissions > 0 {
            format!(
                " including {} depth-budget omission(s)",
                budget_stats.depth_omissions
            )
        } else {
            String::new()
        };
        diagnostics.push(ai_context_diagnostic(
            AI_CONTEXT_BUDGET_OMISSION,
            Severity::Warning,
            format!(
                "AI context omitted {} record(s) and {} text character(s) to satisfy budgets{}",
                budget_stats.omitted_records, budget_stats.omitted_characters, depth_suffix
            ),
            Some(ai_context_canonical_ref(ai_context_node_id(root_node))),
            ai_context_source_map(root_node).cloned(),
        ));
        lossiness_reasons.push(AiContextLossinessReason::Budget);
    }
    if budget_stats.omitted_source_excerpts > 0 {
        diagnostics.push(ai_context_diagnostic(
            AI_CONTEXT_BUDGET_OMISSION,
            Severity::Warning,
            format!(
                "AI context truncated {} source excerpt(s) by {} character(s) to satisfy source excerpt budget {:?}",
                budget_stats.omitted_source_excerpts,
                budget_stats.omitted_source_excerpt_characters,
                budgets.max_source_excerpt_chars
            ),
            Some(ai_context_canonical_ref(ai_context_node_id(root_node))),
            ai_context_source_map(root_node).cloned(),
        ));
        lossiness_reasons.push(AiContextLossinessReason::SourceExcerpt);
    }
}

fn emit_ai_context_instruction_mix_diagnostic(
    records: &[AiContextRecord],
    allow_instruction_mixing: bool,
    root_node: &CemAstNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if allow_instruction_mixing {
        return;
    }
    let has_data = records
        .iter()
        .any(|record| record.boundary == AiContextBoundary::Data);
    let has_instruction = records
        .iter()
        .any(|record| record.boundary == AiContextBoundary::Instruction);
    if has_data && has_instruction {
        diagnostics.push(ai_context_diagnostic(
            AI_CONTEXT_UNSAFE_INSTRUCTION_MIX,
            Severity::Warning,
            "AI context contains both data and instruction records; consumers must keep boundaries separate".to_owned(),
            Some(ai_context_canonical_ref(ai_context_node_id(root_node))),
            ai_context_source_map(root_node).cloned(),
        ));
    }
}

fn limit_ai_context_diagnostics(
    diagnostics: &mut Vec<Diagnostic>,
    budgets: AiContextBudgets,
    lossiness_reasons: &mut Vec<AiContextLossinessReason>,
) {
    let Some(max_diagnostics) = budgets.max_diagnostics else {
        return;
    };
    if diagnostics.len() <= max_diagnostics {
        return;
    }
    diagnostics.truncate(max_diagnostics);
    lossiness_reasons.push(AiContextLossinessReason::DiagnosticBudget);
}

fn ai_context_lossiness(
    lossiness_reasons: &[AiContextLossinessReason],
    budget_stats: &AiContextBudgetStats,
) -> AiContextLossiness {
    let mut reasons = Vec::new();
    for reason in lossiness_reasons {
        if !reasons.contains(reason) {
            reasons.push(*reason);
        }
    }
    AiContextLossiness {
        lossy: !reasons.is_empty()
            || budget_stats.omitted_records > 0
            || budget_stats.omitted_source_excerpts > 0,
        omitted_records: budget_stats.omitted_records,
        omitted_characters: budget_stats.omitted_characters,
        omitted_source_excerpts: budget_stats.omitted_source_excerpts,
        reasons,
    }
}

fn ai_context_includes_node(projection: AiContextProjectionKind, node: &CemAstNode) -> bool {
    match projection {
        AiContextProjectionKind::ContextPack | AiContextProjectionKind::ContextFragment => true,
        AiContextProjectionKind::EntityGraph => matches!(
            node,
            CemAstNode::Document { .. } | CemAstNode::Element { .. } | CemAstNode::Attribute { .. }
        ),
        AiContextProjectionKind::SemanticTokens => !matches!(
            node,
            CemAstNode::Document { .. } | CemAstNode::Whitespace { .. }
        ),
        AiContextProjectionKind::EmbeddingRecord => ai_context_node_text(node)
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty()),
    }
}

fn ai_context_facets(
    projection: AiContextProjectionKind,
    profile: Option<AiContextProfile>,
    node: &CemAstNode,
) -> Vec<String> {
    let mut facets = vec![projection.as_str().to_owned()];
    if let Some(profile) = profile {
        facets.push(format!("profile:{}", profile.as_str()));
    }
    match node {
        CemAstNode::Element { .. } => facets.push("node".to_owned()),
        CemAstNode::Attribute { .. } => facets.push("attribute".to_owned()),
        CemAstNode::Text { .. } | CemAstNode::Cdata { .. } | CemAstNode::RawText { .. } => {
            facets.push("text".to_owned())
        }
        CemAstNode::Error { .. } => facets.push("diagnostic".to_owned()),
        _ => {}
    }
    facets
}

fn ai_context_boundary(node: &CemAstNode) -> AiContextBoundary {
    match node {
        CemAstNode::ProcessingInstruction { .. } => AiContextBoundary::Instruction,
        _ => AiContextBoundary::Data,
    }
}

fn ai_context_canonical_ref(node_id: AstNodeId) -> String {
    format!("cem-ast://node/{node_id}")
}

fn ai_context_expansion_ref(node_id: AstNodeId, label: &str) -> AiContextExpansionRef {
    AiContextExpansionRef {
        canonical_projection: "cem-ast".to_owned(),
        canonical_ref: ai_context_canonical_ref(node_id),
        label: label.to_owned(),
    }
}

fn ai_context_node_id(node: &CemAstNode) -> AstNodeId {
    match node {
        CemAstNode::Document { node_id, .. }
        | CemAstNode::Element { node_id, .. }
        | CemAstNode::Attribute { node_id, .. }
        | CemAstNode::Text { node_id, .. }
        | CemAstNode::Whitespace { node_id, .. }
        | CemAstNode::Comment { node_id, .. }
        | CemAstNode::ProcessingInstruction { node_id, .. }
        | CemAstNode::Cdata { node_id, .. }
        | CemAstNode::RawText { node_id, .. }
        | CemAstNode::Error { node_id, .. } => *node_id,
    }
}

fn ai_context_record_kind(node: &CemAstNode) -> AiContextRecordKind {
    match node {
        CemAstNode::Document { .. } => AiContextRecordKind::Document,
        CemAstNode::Element { .. } => AiContextRecordKind::Element,
        CemAstNode::Attribute { .. } => AiContextRecordKind::Attribute,
        CemAstNode::Text { .. } => AiContextRecordKind::Text,
        CemAstNode::Whitespace { .. } => AiContextRecordKind::Whitespace,
        CemAstNode::Comment { .. } => AiContextRecordKind::Comment,
        CemAstNode::ProcessingInstruction { .. } => AiContextRecordKind::ProcessingInstruction,
        CemAstNode::Cdata { .. } => AiContextRecordKind::Cdata,
        CemAstNode::RawText { .. } => AiContextRecordKind::RawText,
        CemAstNode::Error { .. } => AiContextRecordKind::Error,
    }
}

fn ai_context_node_label(node: &CemAstNode) -> String {
    match node {
        CemAstNode::Document { .. } => "document".to_owned(),
        CemAstNode::Element { expanded_name, .. } => expanded_name.local_name.clone(),
        CemAstNode::Attribute { expanded_name, .. } => format!("@{}", expanded_name.local_name),
        CemAstNode::Text { .. } => "#text".to_owned(),
        CemAstNode::Whitespace { .. } => "#whitespace".to_owned(),
        CemAstNode::Comment { .. } => "#comment".to_owned(),
        CemAstNode::ProcessingInstruction { target, .. } => format!("?{target}"),
        CemAstNode::Cdata { .. } => "#cdata".to_owned(),
        CemAstNode::RawText { .. } => "#raw-text".to_owned(),
        CemAstNode::Error { code, .. } => format!("!{code}"),
    }
}

fn ai_context_node_text(node: &CemAstNode) -> Option<String> {
    match node {
        CemAstNode::Attribute { value, .. } => value.clone(),
        CemAstNode::Text { data, .. }
        | CemAstNode::Whitespace { data, .. }
        | CemAstNode::Comment { data, .. }
        | CemAstNode::Cdata { data, .. }
        | CemAstNode::RawText { data, .. } => Some(data.clone()),
        CemAstNode::ProcessingInstruction { data, .. } => Some(data.clone()),
        CemAstNode::Error { code, .. } => Some(code.clone()),
        CemAstNode::Document { .. } | CemAstNode::Element { .. } => None,
    }
}

fn ai_context_node_character_count(node: &CemAstNode) -> usize {
    ai_context_node_text(node)
        .as_deref()
        .map(str::chars)
        .map(Iterator::count)
        .unwrap_or_default()
}

fn ai_context_estimated_token_count(character_count: usize) -> usize {
    if character_count == 0 {
        0
    } else {
        character_count.saturating_add(3) / 4
    }
}

fn ai_context_source_excerpt(
    node: &CemAstNode,
    include_source_excerpts: bool,
    max_source_excerpt_chars: Option<usize>,
) -> (Option<String>, AiContextRecordBuildStats) {
    let mut stats = AiContextRecordBuildStats::default();
    if !include_source_excerpts {
        return (None, stats);
    }
    let Some(text) = ai_context_node_text(node) else {
        return (None, stats);
    };
    let total_chars = text.chars().count();
    let Some(limit) = max_source_excerpt_chars else {
        stats.source_excerpt_characters = total_chars;
        return (Some(text), stats);
    };
    if total_chars <= limit {
        stats.source_excerpt_characters = total_chars;
        return (Some(text), stats);
    }
    let excerpt = text.chars().take(limit).collect::<String>();
    stats.source_excerpt_characters = limit;
    stats.omitted_source_excerpts = 1;
    stats.omitted_source_excerpt_characters = total_chars - limit;
    (Some(excerpt), stats)
}

fn ai_context_child_ids(node: &CemAstNode) -> Vec<AstNodeId> {
    match node {
        CemAstNode::Document { root_children, .. } => root_children.clone(),
        CemAstNode::Element {
            attributes,
            children,
            ..
        } => attributes.iter().chain(children.iter()).copied().collect(),
        _ => Vec::new(),
    }
}

fn ai_context_element_attributes(
    document: &CemDocument,
    node: &CemAstNode,
) -> BTreeMap<String, String> {
    let CemAstNode::Element { attributes, .. } = node else {
        return BTreeMap::new();
    };
    let mut output = BTreeMap::new();
    for attr_id in attributes {
        let Some(CemAstNode::Attribute {
            expanded_name,
            value,
            ..
        }) = document.get(*attr_id)
        else {
            continue;
        };
        output.insert(
            expanded_name.local_name.clone(),
            value.clone().unwrap_or_default(),
        );
    }
    output
}

fn ai_context_source_map(node: &CemAstNode) -> Option<&SourceMapStack> {
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
        | CemAstNode::Error { source, .. } => Some(source),
    }
}

fn ai_context_source_ranges(node: &CemAstNode) -> Vec<ByteRange> {
    let Some(source_map) = ai_context_source_map(node) else {
        return Vec::new();
    };
    let Some(frame) = source_map.origin().or_else(|| source_map.current()) else {
        return Vec::new();
    };
    match &frame.span {
        FrameSpan::Single(range) => vec![*range],
        FrameSpan::Multi(ranges) => ranges.clone(),
    }
}

fn ai_context_diagnostic(
    code: &str,
    severity: Severity,
    message: String,
    node: Option<String>,
    source_map: Option<SourceMapStack>,
) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        severity,
        message,
        node,
        source_map,
        ..Diagnostic::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;

    fn parse(input: &str) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    }

    fn record_ref_by_text(projection: &AiContextProjection, text: &str) -> String {
        projection
            .records
            .iter()
            .find(|record| record.text.as_deref() == Some(text))
            .map(|record| record.canonical_ref.clone())
            .unwrap_or_else(|| panic!("missing record text `{text}`"))
    }

    fn element_record_by_id<'a>(
        projection: &'a AiContextProjection,
        id: &str,
    ) -> &'a AiContextRecord {
        projection
            .records
            .iter()
            .find(|record| {
                record.kind == AiContextRecordKind::Element
                    && record.attributes.get("id").map(String::as_str) == Some(id)
            })
            .unwrap_or_else(|| panic!("missing element id `{id}`"))
    }

    #[test]
    fn context_pack_projects_ast_records_with_canonical_refs() {
        let document = parse("{button @id=save @cem:action=primary | Save}");

        let projection = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextPack,
                root: None,
                ..AiContextProjectionRequest::default()
            },
        );

        assert_eq!(projection.kind, AiContextProjectionKind::ContextPack);
        assert_eq!(projection.metadata.canonical_projection, "cem-ast");
        assert_eq!(
            projection.metadata.root_ref.as_deref(),
            Some("cem-ast://node/0")
        );
        assert_eq!(projection.metadata.record_count, projection.records.len());
        let button = projection
            .records
            .iter()
            .find(|record| record.kind == AiContextRecordKind::Element)
            .expect("element record");
        assert_eq!(button.label, "button");
        assert_eq!(button.parent_ref.as_deref(), Some("cem-ast://node/0"));
        assert_eq!(
            button.attributes.get("id").map(String::as_str),
            Some("save")
        );
        assert!(button.source_map.is_some());
        assert!(button.facets.contains(&"ai-context-pack".to_owned()));

        let id_attr = projection
            .records
            .iter()
            .find(|record| record.label == "@id")
            .expect("id attribute record");
        assert_eq!(id_attr.text.as_deref(), Some("save"));
        assert_eq!(id_attr.boundary, AiContextBoundary::Data);

        assert!(projection.records.iter().any(|record| {
            record.kind == AiContextRecordKind::Text && record.text.as_deref() == Some("Save")
        }));
    }

    #[test]
    fn projection_kinds_filter_records_for_ai_tasks() {
        let document = parse("{button @id=save | Save}");
        let element_id = document
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .expect("element node id");

        let fragment = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextFragment,
                root: Some(element_id),
                ..AiContextProjectionRequest::default()
            },
        );
        assert_eq!(
            fragment.metadata.root_ref.as_deref(),
            Some(format!("cem-ast://node/{element_id}").as_str())
        );
        assert!(!fragment
            .records
            .iter()
            .any(|record| record.kind == AiContextRecordKind::Document));
        assert!(fragment
            .records
            .iter()
            .find(|record| record.node_id == element_id)
            .is_some_and(|record| record.parent_ref.is_none()));

        let semantic = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::SemanticTokens,
                root: None,
                ..AiContextProjectionRequest::default()
            },
        );
        assert!(semantic.records.iter().all(|record| {
            record.kind != AiContextRecordKind::Document
                && record.kind != AiContextRecordKind::Whitespace
        }));
        assert!(semantic
            .records
            .iter()
            .all(|record| record.facets.contains(&"ai-semantic-tokens".to_owned())));

        let entity_graph = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::EntityGraph,
                root: None,
                ..AiContextProjectionRequest::default()
            },
        );
        assert!(entity_graph.records.iter().all(|record| matches!(
            record.kind,
            AiContextRecordKind::Document
                | AiContextRecordKind::Element
                | AiContextRecordKind::Attribute
        )));
        let entity_refs = entity_graph
            .records
            .iter()
            .map(|record| record.canonical_ref.as_str())
            .collect::<BTreeSet<_>>();
        assert!(entity_graph.records.iter().all(|record| record
            .parent_ref
            .as_deref()
            .is_none_or(|parent_ref| entity_refs.contains(parent_ref))));
        assert!(entity_graph.records.iter().all(|record| record
            .child_refs
            .iter()
            .all(|child_ref| entity_refs.contains(child_ref.as_str()))));

        let embedding = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::EmbeddingRecord,
                root: None,
                ..AiContextProjectionRequest::default()
            },
        );
        assert!(embedding
            .records
            .iter()
            .any(|record| record.text.as_deref() == Some("Save")));
        assert!(embedding.records.iter().all(|record| {
            record
                .text
                .as_deref()
                .is_some_and(|text| !text.trim().is_empty())
        }));
    }

    #[test]
    fn profile_controls_apply_budget_lossiness_and_metadata() {
        let document = parse("{button | Save}{p | More}");

        let projection = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextPack,
                profile: Some("summary".to_owned()),
                budgets: AiContextBudgets {
                    max_nodes: Some(4),
                    max_source_excerpt_chars: Some(2),
                    ..AiContextBudgets::default()
                },
                include_source_excerpts: true,
                host: Some(AiContextHostMetadata {
                    name: "fixture-host".to_owned(),
                    version: Some("1".to_owned()),
                    session_id: None,
                }),
                tool: Some(AiContextToolMetadata {
                    name: "fixture-tool".to_owned(),
                    version: None,
                    invocation_id: Some("run-1".to_owned()),
                }),
                ..AiContextProjectionRequest::default()
            },
        );

        assert_eq!(projection.metadata.profile, Some(AiContextProfile::Summary));
        assert_eq!(projection.metadata.budgets.max_nodes, Some(4));
        assert_eq!(projection.metadata.record_count, projection.records.len());
        assert_eq!(projection.metadata.usage.nodes, projection.records.len());
        assert_eq!(
            projection
                .metadata
                .host
                .as_ref()
                .map(|host| host.name.as_str()),
            Some("fixture-host")
        );
        assert_eq!(
            projection
                .metadata
                .tool
                .as_ref()
                .map(|tool| tool.name.as_str()),
            Some("fixture-tool")
        );
        assert!(projection.metadata.lossiness.lossy);
        assert!(projection.metadata.lossiness.omitted_records > 0);
        assert!(projection.metadata.lossiness.omitted_source_excerpts > 0);
        assert!(projection
            .metadata
            .lossiness
            .reasons
            .contains(&AiContextLossinessReason::Budget));
        assert!(projection
            .metadata
            .lossiness
            .reasons
            .contains(&AiContextLossinessReason::SourceExcerpt));
        assert!(projection
            .metadata
            .expansion_refs
            .iter()
            .any(|reference| reference.canonical_ref == "cem-ast://node/0"));
        assert!(projection.records.iter().all(|record| {
            record.expansion_ref.canonical_projection == "cem-ast"
                && record.canonical_ref == record.expansion_ref.canonical_ref
        }));
        assert!(projection
            .records
            .iter()
            .any(|record| !record.source_ranges.is_empty()));
        assert!(projection.records.iter().any(|record| {
            record.kind == AiContextRecordKind::Text
                && record.source_excerpt.as_deref() == Some("Sa")
                && record.facets.contains(&"profile:summary".to_owned())
        }));
        assert!(projection
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == AI_CONTEXT_BUDGET_OMISSION));
    }

    #[test]
    fn profile_controls_diagnose_unsupported_profile_missing_root_and_instruction_mix() {
        let document = parse("<?xml version=\"1.0\"?>{p | ok}");

        let missing = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                profile: Some("unknown-task".to_owned()),
                root: Some(9_999),
                ..AiContextProjectionRequest::default()
            },
        );
        let missing_codes = missing
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<BTreeSet<_>>();
        assert!(missing_codes.contains(AI_CONTEXT_UNSUPPORTED_PROFILE));
        assert!(missing_codes.contains(AI_CONTEXT_MISSING_EXPANSION_TARGET));
        assert!(missing.records.is_empty());
        assert!(missing
            .metadata
            .lossiness
            .reasons
            .contains(&AiContextLossinessReason::ProfileFallback));
        assert!(missing
            .metadata
            .lossiness
            .reasons
            .contains(&AiContextLossinessReason::MissingExpansionTarget));

        let mixed = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextPack,
                ..AiContextProjectionRequest::default()
            },
        );
        assert!(mixed.records.iter().any(|record| {
            record.boundary == AiContextBoundary::Instruction
                && record.kind == AiContextRecordKind::ProcessingInstruction
        }));
        assert!(mixed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == AI_CONTEXT_UNSAFE_INSTRUCTION_MIX));

        let allowed = project_cem_document_for_ai(
            &document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextPack,
                allow_instruction_mixing: true,
                ..AiContextProjectionRequest::default()
            },
        );
        assert!(!allowed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == AI_CONTEXT_UNSAFE_INSTRUCTION_MIX));
    }

    #[test]
    fn task_eval_fixtures_measure_retrieval_edit_precision_and_token_budget_value() {
        let embedding_document =
            parse("{form | {label | Email}{button | Submit}{aside | Marketing copy}}");

        let full_embedding = project_cem_document_for_ai(
            &embedding_document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::EmbeddingRecord,
                profile: Some("embedding".to_owned()),
                ..AiContextProjectionRequest::default()
            },
        );
        let email_ref = record_ref_by_text(&full_embedding, "Email");
        let submit_text_ref = record_ref_by_text(&full_embedding, "Submit");
        let marketing_ref = record_ref_by_text(&full_embedding, "Marketing copy");
        let retrieval_eval = evaluate_ai_context_task_projection(
            &full_embedding,
            AiContextTaskEvalRequest {
                kind: AiContextTaskEvalKind::Retrieval,
                name: "login-action-retrieval".to_owned(),
                relevant_refs: vec![email_ref.clone(), submit_text_ref.clone()],
                protected_refs: vec![marketing_ref.clone()],
                ..AiContextTaskEvalRequest::default()
            },
        );
        assert!(retrieval_eval.passed);
        assert_eq!(retrieval_eval.relevant_found, 2);
        assert_eq!(
            retrieval_eval.protected_hit_refs,
            vec![marketing_ref.clone()]
        );
        assert_eq!(retrieval_eval.retrieval_recall, 1.0);
        assert!((retrieval_eval.retrieval_precision - (2.0 / 3.0)).abs() < 0.001);

        let edit_document = parse(
            "{form @id=login | {label @id=email-label | Email}{button @id=submit | Submit}{aside @id=promo | Marketing copy}}",
        );
        let full_context = project_cem_document_for_ai(
            &edit_document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextPack,
                profile: Some("refactor".to_owned()),
                ..AiContextProjectionRequest::default()
            },
        );
        let button = element_record_by_id(&full_context, "submit");
        let promo = element_record_by_id(&full_context, "promo");
        let edit_projection = project_cem_document_for_ai(
            &edit_document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::ContextFragment,
                root: Some(button.node_id),
                profile: Some("refactor".to_owned()),
                ..AiContextProjectionRequest::default()
            },
        );
        let edit_eval = evaluate_ai_context_task_projection(
            &edit_projection,
            AiContextTaskEvalRequest {
                kind: AiContextTaskEvalKind::EditPrecision,
                name: "submit-button-edit".to_owned(),
                relevant_refs: vec![button.canonical_ref.clone()],
                edit_target_refs: vec![button.canonical_ref.clone()],
                protected_refs: vec![promo.canonical_ref.clone()],
                ..AiContextTaskEvalRequest::default()
            },
        );
        assert!(edit_eval.passed);
        assert_eq!(edit_eval.edit_targets_found, 1);
        assert_eq!(edit_eval.source_mapped_edit_targets, 1);
        assert!(edit_eval.protected_hit_refs.is_empty());
        assert_eq!(edit_eval.edit_precision, 1.0);

        let budgeted_embedding = project_cem_document_for_ai(
            &embedding_document,
            AiContextProjectionRequest {
                kind: AiContextProjectionKind::EmbeddingRecord,
                profile: Some("embedding".to_owned()),
                budgets: AiContextBudgets {
                    max_tokens: Some(4),
                    ..AiContextBudgets::default()
                },
                ..AiContextProjectionRequest::default()
            },
        );
        let budget_eval = evaluate_ai_context_task_projection(
            &budgeted_embedding,
            AiContextTaskEvalRequest {
                kind: AiContextTaskEvalKind::TokenBudgetValue,
                name: "login-action-token-budget".to_owned(),
                relevant_refs: vec![email_ref, submit_text_ref],
                protected_refs: vec![marketing_ref],
                max_tokens: Some(4),
                ..AiContextTaskEvalRequest::default()
            },
        );
        assert!(budget_eval.passed);
        assert_eq!(budget_eval.record_count, 2);
        assert_eq!(budget_eval.relevant_found, 2);
        assert!(budget_eval.within_token_budget);
        assert!(budget_eval.protected_hit_refs.is_empty());
        assert!(budget_eval.token_budget_value >= 500.0);
    }
}
