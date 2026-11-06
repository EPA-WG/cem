//! Retained native SSR envelope. This contract does not execute lifecycle work.
use std::sync::Arc;

use serde::Serialize;

use crate::content_cache::ContentHash;
use crate::dependency_identity::{DependencyLayer, DependencySnapshot, DependencyVersion};
use crate::diagnostics::Diagnostic;
use crate::source_map::SourceMapStack;
use crate::ssr_route::{SsrBasePolicy, SsrBaseSelection, SsrPageRoute};
use crate::transform_artifact::{
    TransformArtifactBody, TransformDataArtifact, TransformNativeArtifact,
};
use crate::validation::html::{HtmlDocumentAst, HtmlSourcePosition, HtmlSourceRange};

pub const SSR_PAGE_CONTENT_TYPE: &str = "application/vnd.cem.ssr-page+cem-bin";
pub const SSR_PAGE_SCHEMA: &str = "https://cem.dev/ns/projection/ssr-page/1";
pub const SSR_PAGE_REPRESENTATION: &str = "cem.ssr-page";
pub const SSR_METADATA_LIMIT: usize = 1024 * 1024;

/// Explicit caller-selected limits; no implicit wall/CPU or resource grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsrSettlePolicy {
    pub max_declaration_depth: u32,
    pub max_instances: u32,
    pub max_render_passes: u32,
    pub max_resources: u32,
    pub max_total_bytes: u64,
    pub max_wall_time_ms: u64,
    pub max_cpu_time_ms: u64,
    pub allow_static_fallback: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsrExecutionUsage {
    pub declaration_depth: u32,
    pub instances: u32,
    pub render_passes: u32,
    pub resources: u32,
    pub total_bytes: u64,
    pub wall_time_ms: u64,
    pub cpu_time_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SsrSourceMapMode {
    Retained,
    None,
}

/// Producer-owned deterministic dependency closure and compiler identities.
/// A complete hash remains a cache candidate, never admission/reuse authority.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsrExecutionIdentity {
    pub dependencies: DependencySnapshot,
    pub lifecycle_version: String,
    pub island_schema_version: String,
    pub render_plan_version: String,
    pub cem_ql_version: String,
    pub entrypoint: String,
    pub parameters_hash: ContentHash,
    pub source_map_mode: SsrSourceMapMode,
    pub formatter_profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsrArtifactRoute {
    pub source_uri: String,
    pub page_url: String,
    pub document_base_url: String,
    pub output_uri: Option<String>,
    pub page_source_map: Option<SourceMapStack>,
    pub base_source_map: Option<SourceMapStack>,
    pub output_source_map: Option<SourceMapStack>,
    pub base_selection: SsrBaseSelection,
    pub base_policy: SsrBasePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SsrPartialReason {
    ParticipantUnavailable,
    CapabilityUnavailable,
    Cancelled,
    BudgetExhausted,
    Incomplete,
}

/// No settled/ready constructor exists until the shared native coordinator owns
/// lifecycle completion and validates islands, plans, ranges and policy freshness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum SsrLifecycleState {
    Unrendered,
    Partial { reason: SsrPartialReason },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsrAdoptionIdentity<'a> {
    lifecycle_version: &'a str,
    island_schema_version: &'a str,
    render_plan_version: &'a str,
    hydration_ready: bool,
    lifecycle: SsrLifecycleState,
}

impl SsrAdoptionIdentity<'_> {
    pub fn hydration_ready(&self) -> bool {
        self.hydration_ready
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrArtifactError {
    SourceOwnerMismatch,
    DocumentRequired,
    InvalidPolicy,
    BudgetExceeded,
    InvalidProgress,
    InvalidIdentity,
    IdentityIncomplete,
    MetadataBounds,
}

#[derive(Debug, Clone)]
pub struct CemSsrPageArtifact {
    source: Arc<HtmlDocumentAst>,
    document: TransformArtifactBody,
    applied_tree: Option<Arc<crate::parser::tree::RetainedCemTree>>,
    route: SsrArtifactRoute,
    identity: SsrExecutionIdentity,
    policy: SsrSettlePolicy,
    usage: SsrExecutionUsage,
    lifecycle: SsrLifecycleState,
    source_map: Option<SourceMapStack>,
    diagnostics: Vec<Diagnostic>,
}

impl CemSsrPageArtifact {
    pub fn unrendered(
        source: Arc<HtmlDocumentAst>,
        route: &SsrPageRoute<'_>,
        document: TransformArtifactBody,
        identity: SsrExecutionIdentity,
        policy: SsrSettlePolicy,
    ) -> Result<Self, SsrArtifactError> {
        if !std::ptr::eq(source.as_ref(), route.source_document()) {
            return Err(SsrArtifactError::SourceOwnerMismatch);
        }
        match &document {
            TransformArtifactBody::HtmlDomProjection(owner) if !Arc::ptr_eq(owner, &source) => {
                return Err(SsrArtifactError::SourceOwnerMismatch);
            }
            TransformArtifactBody::HtmlDomProjection(_)
            | TransformArtifactBody::DomProjection(_) => {}
            _ => return Err(SsrArtifactError::DocumentRequired),
        }
        policy.validate()?;
        identity.validate_protocol()?;
        let usage = SsrExecutionUsage {
            total_bytes: source.source.byte_length as u64,
            ..SsrExecutionUsage::default()
        };
        if !usage.within(&policy) {
            return Err(SsrArtifactError::BudgetExceeded);
        }
        let source_map = source.source_map_for_range(HtmlSourceRange {
            start: HtmlSourcePosition {
                line: 1,
                column: 1,
                byte_offset: 0,
            },
            byte_length: source.source_text.len() as u64,
        });
        let artifact = Self {
            source,
            document,
            applied_tree: None,
            identity,
            policy,
            usage,
            source_map,
            route: SsrArtifactRoute {
                source_uri: route.source_uri().into(),
                page_url: route.page_url().into(),
                document_base_url: route.document_base_url().into(),
                output_uri: route.output_uri().map(str::to_owned),
                page_source_map: route.page_url_source_map().cloned(),
                base_source_map: route.base_source_map().cloned(),
                output_source_map: route.output_uri_source_map().cloned(),
                base_selection: route.base_selection(),
                base_policy: route.base_policy(),
            },
            lifecycle: SsrLifecycleState::Unrendered,
            diagnostics: Vec::new(),
        };
        artifact.check_metadata_bounds()?;
        Ok(artifact)
    }

    pub fn source_document(&self) -> &Arc<HtmlDocumentAst> {
        &self.source
    }
    pub fn document(&self) -> &TransformArtifactBody {
        &self.document
    }
    /// Applied native view retains its source/producer owners. It is not a ready proof.
    pub fn applied_tree(&self) -> Option<&Arc<crate::parser::tree::RetainedCemTree>> {
        self.applied_tree.as_ref()
    }
    pub fn into_applied_dom(
        mut self,
        application: &crate::parser::tree_apply::CemTreeApplication,
        usage: SsrExecutionUsage,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Self, SsrArtifactError> {
        if self.lifecycle
            != (SsrLifecycleState::Partial {
                reason: SsrPartialReason::Incomplete,
            })
            || self.usage.render_passes.checked_add(1) != Some(usage.render_passes)
        {
            return Err(SsrArtifactError::InvalidProgress);
        }
        let owner = application
            .source()
            .native_owner()
            .and_then(|owner| owner.downcast_ref::<HtmlDocumentAst>())
            .ok_or(SsrArtifactError::SourceOwnerMismatch)?;
        if !std::ptr::eq(owner, self.source.as_ref())
            || application.source().source_uri() != self.route.source_uri
            || application.tree().document_uri() != Some(self.route.page_url.as_str())
            || application.tree().base_uri() != Some(self.route.document_base_url.as_str())
        {
            return Err(SsrArtifactError::SourceOwnerMismatch);
        }
        self.document =
            TransformArtifactBody::DomProjection(application.tree().ast_owner().clone());
        self.applied_tree = Some(application.tree().clone());
        self.into_partial(SsrPartialReason::Incomplete, usage, diagnostics)
    }
    pub fn route(&self) -> &SsrArtifactRoute {
        &self.route
    }
    pub fn policy(&self) -> &SsrSettlePolicy {
        &self.policy
    }
    pub fn usage(&self) -> SsrExecutionUsage {
        self.usage
    }
    pub fn lifecycle(&self) -> SsrLifecycleState {
        self.lifecycle
    }
    pub fn source_map(&self) -> Option<&SourceMapStack> {
        self.source_map.as_ref()
    }
    pub fn adoption_identity(&self) -> SsrAdoptionIdentity<'_> {
        SsrAdoptionIdentity {
            lifecycle_version: &self.identity.lifecycle_version,
            island_schema_version: &self.identity.island_schema_version,
            render_plan_version: &self.identity.render_plan_version,
            hydration_ready: false,
            lifecycle: self.lifecycle,
        }
    }
    /// Conservative deterministic input identity only. Equal hashes authorize
    /// neither cache reuse nor skipping fresh policy/resource/lifecycle admission.
    pub fn render_input_identity(&self) -> Result<ContentHash, SsrArtifactError> {
        let dependencies = &self.identity.dependencies;
        if dependencies.layer != DependencyLayer::RenderProjection
            || dependencies.root_uri != self.route.source_uri
            || dependencies
                .sources
                .values()
                .any(|source| !matches!(source.version, DependencyVersion::Content(_)))
        {
            return Err(SsrArtifactError::IdentityIncomplete);
        }
        let root = dependencies
            .sources
            .get(&dependencies.root_uri)
            .ok_or(SsrArtifactError::IdentityIncomplete)?;
        if root.version
            != DependencyVersion::Content(ContentHash::from_blake3(
                self.source.source_text.as_bytes(),
            ))
            || crate::schema::registry::content_type_essence(&root.content_type)
                != self.source.source.media_type
            || root.schema.as_deref() != Some(crate::schema::registry::HTML_SCHEMA_URI)
        {
            return Err(SsrArtifactError::IdentityIncomplete);
        }
        let closure = dependencies
            .identity()
            .map_err(|_| SsrArtifactError::IdentityIncomplete)?;
        // Named control-metadata boundary: no source, AST, rendered output or
        // physical destination enters this serialization. Tuples cannot alias
        // when version/entrypoint strings themselves contain delimiters.
        let metadata = bounded_json(&(
            SSR_PAGE_SCHEMA,
            crate::VERSION,
            (
                &self.route.source_uri,
                &self.route.page_url,
                &self.route.document_base_url,
                self.route.base_policy,
            ),
            &closure.hash,
            &self.identity.lifecycle_version,
            &self.identity.island_schema_version,
            &self.identity.render_plan_version,
            &self.identity.cem_ql_version,
            &self.identity.entrypoint,
            &self.identity.parameters_hash,
            self.identity.source_map_mode,
            &self.identity.formatter_profile,
            &self.policy,
        ))?;
        let mut hasher = blake3::Hasher::new_derive_key("cem SSR render input identity/1");
        hasher.update(&metadata);
        Ok(ContentHash {
            scheme: crate::content_cache::HASH_SCHEME.into(),
            hex: hasher.finalize().to_hex().to_string(),
        })
    }
    pub fn into_partial(
        mut self,
        reason: SsrPartialReason,
        usage: SsrExecutionUsage,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<Self, SsrArtifactError> {
        if usage
            .counters()
            .iter()
            .zip(self.usage.counters())
            .any(|(next, previous)| *next < previous)
        {
            return Err(SsrArtifactError::InvalidProgress);
        }
        if !usage.within(&self.policy) && reason != SsrPartialReason::BudgetExhausted {
            return Err(SsrArtifactError::BudgetExceeded);
        }
        self.lifecycle = SsrLifecycleState::Partial { reason };
        self.usage = usage;
        self.diagnostics = diagnostics;
        self.check_metadata_bounds()?;
        Ok(self)
    }
    /// A static-fallback candidate still needs normal publication/policy checks.
    /// This is not a write grant or a hydration/adoption decision.
    pub fn static_fallback_candidate(&self) -> bool {
        self.policy.allow_static_fallback
            && matches!(self.lifecycle, SsrLifecycleState::Partial { .. })
    }
    /// Explicit diagnostic control export. Native DOM/source owners never enter
    /// the report; this sidecar is not browser hydration data or a native handoff.
    pub fn export_report_json(&self) -> Result<Vec<u8>, SsrArtifactError> {
        bounded_json(&self.report())
    }
    pub fn into_transform_artifact(self, id: &str) -> TransformDataArtifact {
        TransformDataArtifact::new(
            id,
            Some(self.route.page_url.clone()),
            Some(crate::engine::FormatIdentity {
                content_type: Some(SSR_PAGE_CONTENT_TYPE.into()),
                schema: Some(SSR_PAGE_SCHEMA.into()),
                ..crate::engine::FormatIdentity::default()
            }),
            TransformArtifactBody::Extension(Arc::new(self)),
        )
    }

    fn check_metadata_bounds(&self) -> Result<(), SsrArtifactError> {
        if self.diagnostics.len() > 256 {
            return Err(SsrArtifactError::MetadataBounds);
        }
        // Only typed control records are serialized. Bound aggregate retained
        // metadata, including escaped text and origin stacks, before acceptance.
        bounded_json(&(
            &self.identity,
            &self.route,
            &self.policy,
            &self.usage,
            &self.diagnostics,
            &self.source_map,
        ))?;
        self.export_report_json()?;
        Ok(())
    }

    fn report(&self) -> SsrArtifactReport<'_> {
        let (identity, refusal) = match self.render_input_identity() {
            Ok(identity) => (Some(identity), None),
            Err(error) => (None, Some(error.code())),
        };
        SsrArtifactReport {
            content_type: SSR_PAGE_CONTENT_TYPE,
            schema: SSR_PAGE_SCHEMA,
            representation: SSR_PAGE_REPRESENTATION,
            document_representation: self.document.representation_id(),
            route: &self.route,
            policy: &self.policy,
            usage: self.usage,
            budget_exceeded: !self.usage.within(&self.policy),
            adoption: self.adoption_identity(),
            render_input_identity: identity,
            identity_refusal: refusal,
            source_map: self.source_map.as_ref(),
            diagnostics: &self.diagnostics,
        }
    }
}

impl TransformNativeArtifact for CemSsrPageArtifact {
    fn representation_id(&self) -> &'static str {
        SSR_PAGE_REPRESENTATION
    }
    fn source_map(&self) -> Option<&SourceMapStack> {
        self.source_map.as_ref()
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl SsrSettlePolicy {
    /// Supported initial envelope ceilings, not defaults for any existing host.
    pub fn validate(&self) -> Result<(), SsrArtifactError> {
        let ceilings = [64, 4096, 64, 1024, 64 * 1024 * 1024, 60_000, 60_000];
        if self
            .counters()
            .iter()
            .zip(ceilings)
            .any(|(value, max)| *value == 0 || *value > max)
        {
            Err(SsrArtifactError::InvalidPolicy)
        } else {
            Ok(())
        }
    }
    fn counters(&self) -> [u64; 7] {
        [
            self.max_declaration_depth.into(),
            self.max_instances.into(),
            self.max_render_passes.into(),
            self.max_resources.into(),
            self.max_total_bytes,
            self.max_wall_time_ms,
            self.max_cpu_time_ms,
        ]
    }
}

impl SsrExecutionUsage {
    fn counters(&self) -> [u64; 7] {
        [
            self.declaration_depth.into(),
            self.instances.into(),
            self.render_passes.into(),
            self.resources.into(),
            self.total_bytes,
            self.wall_time_ms,
            self.cpu_time_ms,
        ]
    }
    fn within(&self, policy: &SsrSettlePolicy) -> bool {
        self.counters()
            .iter()
            .zip(policy.counters())
            .all(|(value, max)| *value <= max)
    }
}

impl SsrExecutionIdentity {
    fn validate_protocol(&self) -> Result<(), SsrArtifactError> {
        if [
            &self.lifecycle_version,
            &self.island_schema_version,
            &self.render_plan_version,
            &self.cem_ql_version,
            &self.entrypoint,
            &self.formatter_profile,
        ]
        .iter()
        .any(|text| text.is_empty() || text.trim() != text.as_str() || text.len() > 4096)
            || self.parameters_hash.scheme != crate::content_cache::HASH_SCHEME
            || self.parameters_hash.hex.len() != 64
            || !self
                .parameters_hash
                .hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(SsrArtifactError::InvalidIdentity);
        }
        if self.dependencies.sources.len() > crate::dependency_identity::MAX_DEPENDENCY_SOURCES
            || self.dependencies.imports.len() > crate::dependency_identity::MAX_DEPENDENCY_IMPORTS
            || self.dependencies.options.len() > 128
        {
            return Err(SsrArtifactError::MetadataBounds);
        }
        Ok(())
    }
}

impl SsrArtifactError {
    pub fn code(self) -> &'static str {
        match self {
            Self::SourceOwnerMismatch => "cem.ssr.artifact_owner_mismatch",
            Self::DocumentRequired => "cem.ssr.artifact_document_required",
            Self::InvalidPolicy => "cem.ssr.settle_policy_invalid",
            Self::BudgetExceeded => "cem.ssr.settle_budget",
            Self::InvalidProgress => "cem.ssr.progress_invalid",
            Self::InvalidIdentity => "cem.ssr.identity_invalid",
            Self::IdentityIncomplete => "cem.ssr.identity_incomplete",
            Self::MetadataBounds => "cem.ssr.artifact_metadata_bounds",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SsrArtifactReport<'a> {
    content_type: &'static str,
    schema: &'static str,
    representation: &'static str,
    document_representation: &'static str,
    route: &'a SsrArtifactRoute,
    policy: &'a SsrSettlePolicy,
    usage: SsrExecutionUsage,
    budget_exceeded: bool,
    adoption: SsrAdoptionIdentity<'a>,
    render_input_identity: Option<ContentHash>,
    identity_refusal: Option<&'static str>,
    source_map: Option<&'a SourceMapStack>,
    diagnostics: &'a [Diagnostic],
}

/// Bounded control serialization only. Writes fail before allocating beyond the
/// metadata ceiling, even if escaping an otherwise short string expands it.
fn bounded_json(value: &impl Serialize) -> Result<Vec<u8>, SsrArtifactError> {
    struct Writer(Vec<u8>);
    impl std::io::Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes.len() > SSR_METADATA_LIMIT.saturating_sub(self.0.len()) {
                return Err(std::io::Error::other("SSR metadata limit"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut writer = Writer(Vec::new());
    serde_json::to_writer(&mut writer, value).map_err(|_| SsrArtifactError::MetadataBounds)?;
    Ok(writer.0)
}

#[cfg(test)]
#[path = "ssr_artifact_tests.rs"]
mod tests;
