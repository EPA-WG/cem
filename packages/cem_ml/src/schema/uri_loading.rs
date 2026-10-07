//! Loader-side URI acquisition and retained target selection. Query evaluation,
//! runtime contexts, relationship grants and schema activation remain consumers.
use crate::{
    diagnostics::{Diagnostic, Severity},
    import::{import_bytes_with_lexical_scopes, ScopedCemImport},
    operation_control::{ExecutionScopeId, OperationControl},
    parser::CemAstNode,
    resolver::{
        ResolveDirection, ResolvePolicyDecision, ResolvePurpose, ResolveRequest, ResolvedRead,
        ResolverDiagnostic, ResolverPolicy, ResolverRegistry,
    },
    schema::{
        declaration_references::SchemaDeclarationNode,
        scope_references::{admit_schema_scope_target, SchemaScopeTargetError},
        vocab::CompiledSchema,
    },
    source_map::SourceMapStack,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SchemaUriResourceRequest {
    request: ResolveRequest,
    public_part: Option<String>,
    pub policy_decision: ResolvePolicyDecision,
    source: SourceMapStack,
}
#[derive(Debug, Clone)]
pub struct SchemaUriResource {
    pub imported: ScopedCemImport,
    pub targets: Vec<SchemaDeclarationNode>,
}
impl SchemaUriResourceRequest {
    /// Resolve the loader URL and apply transport policy to the fragment-free
    /// resource. Public parts remain an explicit loader export contract. The
    /// chosen policy and resolved request are stable for this acquisition attempt.
    pub fn new(
        uri: &str,
        base_url: &str,
        content_type_hint: Option<&str>,
        policy: &ResolverPolicy,
        source: SourceMapStack,
    ) -> Result<Self, Diagnostic> {
        let error = |code, message| failure(code, message, uri, &source);
        let base = url::Url::parse(base_url)
            .map_err(|e| error("cem.schema.uri_invalid", e.to_string()))?;
        let mut target = base
            .join(uri)
            .map_err(|e| error("cem.schema.uri_invalid", e.to_string()))?;
        let public_part = target.fragment().map(str::to_owned);
        if public_part.as_deref() == Some("") {
            return Err(error(
                "cem.schema.public_part_invalid",
                "Empty public fragment.".into(),
            ));
        }
        target.set_fragment(None);
        let mut request = ResolveRequest::new(
            target.to_string(),
            ResolvePurpose::Template,
            ResolveDirection::Read,
        )
        .with_base_uri(base.to_string());
        request.content_type_hint = content_type_hint.map(str::to_owned);
        let decision = policy
            .decide(&request)
            .map_err(|e| error("cem.resolver.policy_denied", e.to_string()))?;
        let effective = base
            .join(&decision.effective_uri)
            .map_err(|e| error("cem.schema.uri_invalid", e.to_string()))?;
        if effective.fragment().is_some() {
            return Err(error(
                "cem.schema.uri_invalid",
                "Transport policy substitution must name a fragment-free resource.".into(),
            ));
        }
        request.uri = effective.to_string();
        Ok(Self {
            request,
            public_part,
            policy_decision: decision,
            source,
        })
    }
    pub fn source_map(&self) -> &SourceMapStack {
        &self.source
    }
    pub fn request(&self) -> &ResolveRequest {
        &self.request
    }
    /// Encoded URL fragment; the explicit export callback owns its interpretation.
    pub fn public_part(&self) -> Option<&str> {
        self.public_part.as_deref()
    }
    pub fn read(
        &self,
        registry: &ResolverRegistry,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<ResolvedRead, Diagnostic> {
        registry
            .read_with_control(&self.request, control, scope)
            .map_err(|e| {
                let mut diagnostic =
                    failure(e.code(), e.to_string(), &self.request.uri, &self.source);
                let original = match &e {
                    ResolverDiagnostic::Cancelled { source_map, .. } => source_map.as_ref(),
                    ResolverDiagnostic::Control { failure, .. } => failure.source_map.as_ref(),
                    _ => None,
                };
                if let Some(source) = original {
                    diagnostic.source_map = Some(source.clone());
                }
                diagnostic
            })
    }
    /// Import exactly once using final response provenance. For a fragment-free
    /// URI, select exactly one admissible direct document child; ancillary roots
    /// are not candidates. Public callbacks supply declared original exports, never
    /// implicit ID scans. Selection does not compile or evaluate declarations.
    pub fn import_response<F>(
        &self,
        response: ResolvedRead,
        public_exports: F,
    ) -> Result<SchemaUriResource, Diagnostic>
    where
        F: FnOnce(&ScopedCemImport, &str) -> Result<Vec<SchemaDeclarationNode>, String>,
    {
        let error = |code, message| failure(code, message, &response.uri, &self.source);
        // Validate final URL provenance; redirects cannot redefine the requested part.
        let final_url = url::Url::parse(&response.uri)
            .map_err(|e| error("cem.schema.uri_invalid", e.to_string()))?;
        if final_url.fragment().is_some() {
            return Err(error(
                "cem.schema.uri_invalid",
                "Final response URL must be fragment-free.".into(),
            ));
        }
        let mime = response
            .content_type
            .as_deref()
            .or(self.request.content_type_hint.as_deref())
            .ok_or_else(|| {
                error(
                    "cem.schema.import_failed",
                    "Schema response content type is unavailable.".into(),
                )
            })?;
        let imported = import_bytes_with_lexical_scopes(
            &response.bytes,
            mime,
            &response.uri,
            CompiledSchema::cem_core(),
        )
        .map_err(|e| error("cem.schema.import_failed", e))?;
        if let Some(diagnostic) = imported
            .captured
            .document()
            .diagnostics
            .iter()
            .chain(imported.captured.diagnostics())
            .find(|d| d.severity.is_hard_violation())
        {
            let mut diagnostic = diagnostic.clone();
            if diagnostic.uri.is_none() {
                diagnostic.uri = Some(response.uri.clone());
            }
            return Err(diagnostic);
        }
        let targets = if let Some(part) = &self.public_part {
            public_exports(&imported, part)
                .map_err(|e| error("cem.schema.public_part_unavailable", e))?
        } else {
            let Some(CemAstNode::Document { root_children, .. }) = imported.tree.ast().get(0)
            else {
                unreachable!("import owns a CEM document");
            };
            root_children
                .iter()
                .filter_map(|id| {
                    let source =
                        SchemaDeclarationNode::new(imported.tree.ast_owner().clone(), *id)?;
                    match admit_schema_scope_target(source.clone(), |node| {
                        imported
                            .captured
                            .expanded_name(node.document(), node.node_id())
                            .cloned()
                    }) {
                        Ok(_) | Err(SchemaScopeTargetError::DeclarationCount(_)) => Some(source),
                        _ => None,
                    }
                })
                .collect()
        };
        if targets.len() != 1 {
            return Err(error(
                "cem.schema.uri_target_count",
                format!("Expected one schema URI target; found {}.", targets.len()),
            ));
        }
        let target = &targets[0];
        if !Arc::ptr_eq(target.document(), imported.tree.ast_owner()) {
            return Err(error(
                "cem.schema.public_part_owner",
                "Public schema export must retain the loaded owner.".into(),
            ));
        }
        admit_schema_scope_target(target.clone(), |node| {
            imported
                .captured
                .expanded_name(node.document(), node.node_id())
                .cloned()
        })
        .map_err(|e| {
            error(
                "cem.schema.uri_target_invalid",
                format!("Invalid schema URI target: {e:?}"),
            )
        })?;
        Ok(SchemaUriResource { imported, targets })
    }
}
fn failure(code: &str, message: String, uri: &str, source: &SourceMapStack) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        message,
        uri: Some(uri.into()),
        source_map: Some(source.clone()),
        ..Default::default()
    }
}
