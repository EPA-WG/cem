//! Explicit resolver/import lifecycle bridge. Completion supplies retained owners
//! and loader selections; consumption and scope authorization remain separate.
use super::{uri_loads::key, CemQlSchemaDeclarationHost, DeclarationScope};
use crate::api::StandaloneExpressionContext;
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    import::ScopedCemImport,
    parser::CemAstNode,
    resolver::{ResolvedRead, ResolverPolicy},
    schema::{
        declaration_references::SchemaDeclarationNode,
        reference_policy::ReferenceScopePolicy,
        scope_controls::{SchemaHostControl, SchemaHostSource},
        uri_loading::{SchemaUriResource, SchemaUriResourceRequest},
    },
    value::reference_resolution::ReferenceLinkEvaluation,
};

#[derive(Debug, Clone)]
pub struct SchemaUriLoadTicket {
    host: u64,
    generation: u64,
    control: SchemaHostControl,
    uri: String,
    request: SchemaUriResourceRequest,
}
impl SchemaUriLoadTicket {
    /// Submit this stable loader request through the host's resolver/resource
    /// queue, or use its controlled native `read` entry. Creation performs no I/O.
    pub fn request(&self) -> &SchemaUriResourceRequest {
        &self.request
    }
}
#[derive(Debug, Clone)]
pub struct SchemaUriLoadedScope {
    pub resource: SchemaUriResource,
    /// New destination relationship boundary; completion grants no crossings.
    pub scope: DeclarationScope,
}
impl CemQlSchemaDeclarationHost {
    /// Start a new original control/URI acquisition generation and immediately
    /// replace any previous loader snapshot with Pending. Policy failure publishes
    /// its hard diagnostic. An older ticket cannot complete this newer attempt.
    pub fn begin_schema_uri_resource(
        &mut self,
        control: &SchemaHostControl,
        base_url: &str,
        content_type_hint: Option<&str>,
        policy: &ResolverPolicy,
    ) -> Result<SchemaUriLoadTicket, Diagnostic> {
        let error = |message| control_error(control, "cem.schema.uri_handoff_invalid", message);
        let SchemaHostSource::Uri(uri) = &control.source else {
            return Err(error("A URI control is required.".into()));
        };
        if !std::sync::Arc::ptr_eq(control.host.document(), control.attribute.document()) {
            return Err(error(
                "Control value must retain the original host owner.".into(),
            ));
        }
        let source = match control.attribute.node() {
            CemAstNode::Attribute { source, .. }
            | CemAstNode::Text { source, .. }
            | CemAstNode::Element { source, .. } => source.clone(),
            _ => {
                return Err(error(
                    "URI control value must retain its source occurrence.".into(),
                ))
            }
        };
        let generation = self
            .next_schema_uri_generation
            .checked_add(1)
            .ok_or_else(|| error("Schema URI generation space exhausted.".into()))?;
        self.set_schema_uri_load(
            &control.host,
            uri,
            ReferenceLinkEvaluation::Pending("schema-resource-loading".into()),
        )
        .map_err(|e| error(e.to_string()))?;
        self.next_schema_uri_generation = generation;
        self.schema_uri_generations
            .insert(key(&control.host, uri), generation);
        let request =
            match SchemaUriResourceRequest::new(uri, base_url, content_type_hint, policy, source) {
                Ok(request) => request,
                Err(diagnostic) => {
                    self.set_schema_uri_load(
                        &control.host,
                        uri,
                        ReferenceLinkEvaluation::Invalid(vec![diagnostic.clone()]),
                    )
                    .expect("registered original control");
                    self.schema_uri_generations.remove(&key(&control.host, uri));
                    return Err(diagnostic);
                }
            };
        Ok(SchemaUriLoadTicket {
            host: self.identity,
            generation,
            control: control.clone(),
            uri: uri.clone(),
            request,
        })
    }

    /// Complete only the current acquisition. Errors/abort publish their original
    /// diagnostics; valid bytes import once before caller inputs are prepared.
    /// The context callback supplies the destination root inputs, including None
    /// for pending readiness. Returned capture supports the existing explicit
    /// local-occurrence handoff before references are consumed. No declaration
    /// evaluation, crossing grants or runtime activation happen on completion.
    pub fn complete_schema_uri_resource<C, P>(
        &mut self,
        ticket: &SchemaUriLoadTicket,
        response: Result<ResolvedRead, Diagnostic>,
        policy: ReferenceScopePolicy,
        context: C,
        public_exports: P,
    ) -> Result<SchemaUriLoadedScope, Diagnostic>
    where
        C: FnOnce(&SchemaUriResource) -> Option<StandaloneExpressionContext>,
        P: FnOnce(&ScopedCemImport, &str) -> Result<Vec<SchemaDeclarationNode>, String>,
    {
        let slot = key(&ticket.control.host, &ticket.uri);
        if ticket.host != self.identity
            || self.schema_uri_generations.get(&slot) != Some(&ticket.generation)
        {
            return Err(control_error(
                &ticket.control,
                "cem.schema.uri_generation_invalid",
                "Schema URI completion is stale, already settled or belongs to another host."
                    .into(),
            ));
        }
        let resource =
            response.and_then(|response| ticket.request.import_response(response, public_exports));
        let resource = match resource {
            Ok(resource) => resource,
            Err(diagnostic) => {
                self.set_schema_uri_load(
                    &ticket.control.host,
                    &ticket.uri,
                    ReferenceLinkEvaluation::Invalid(vec![diagnostic.clone()]),
                )
                .expect("registered original control");
                self.schema_uri_generations.remove(&slot);
                return Err(diagnostic);
            }
        };
        let inputs = context(&resource);
        let scope = self.register_scope(resource.imported.tree.clone(), inputs, policy);
        self.attach_captured_names(&resource.imported.captured)
            .expect("newly registered original owner");
        self.set_schema_uri_load(
            &ticket.control.host,
            &ticket.uri,
            ReferenceLinkEvaluation::Resolved(resource.targets.clone()),
        )
        .expect("registered original control");
        self.schema_uri_generations.remove(&slot);
        Ok(SchemaUriLoadedScope { resource, scope })
    }
}
fn control_error(control: &SchemaHostControl, code: &str, message: String) -> Diagnostic {
    let source = match control.attribute.node() {
        CemAstNode::Attribute { source, .. }
        | CemAstNode::Text { source, .. }
        | CemAstNode::Element { source, .. } => Some(source.clone()),
        _ => None,
    };
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        message,
        node: Some(control.attribute.node_id().to_string()),
        source_map: source,
        ..Default::default()
    }
}
