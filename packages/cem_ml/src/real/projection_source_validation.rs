//! Validation-only admission for explicit native projection source identities.
//!
//! Projection lifecycle adapters remain output-only. Source validation consumes
//! the existing native header/shape facts without importing an AST, decoding a
//! replay artifact, or interpreting binary input as CEM text.

use crate::diagnostics::Diagnostic;
use crate::engine::{EngineContext, EngineInput, FormatIdentity};
use crate::lifecycle::DOM_JSON_PROJECTION_SCHEMA;
use crate::schema::registry::{
    CEM_AST_PROJECTION_SCHEMA_URI, CEM_DOM_PROJECTION_SCHEMA_URI, CEM_EVENTS_PROJECTION_SCHEMA_URI,
};
use crate::validation::{cem_ast_projection, cem_dom_projection, cem_events_projection};

#[derive(Clone, Copy)]
enum Projection {
    Dom,
    Ast,
    Events,
}

impl Projection {
    fn from_schema(schema: &str) -> Option<Self> {
        match schema {
            CEM_DOM_PROJECTION_SCHEMA_URI | DOM_JSON_PROJECTION_SCHEMA => Some(Self::Dom),
            CEM_AST_PROJECTION_SCHEMA_URI => Some(Self::Ast),
            CEM_EVENTS_PROJECTION_SCHEMA_URI => Some(Self::Events),
            _ => None,
        }
    }

    fn schema(self) -> &'static str {
        match self {
            Self::Dom => CEM_DOM_PROJECTION_SCHEMA_URI,
            Self::Ast => CEM_AST_PROJECTION_SCHEMA_URI,
            Self::Events => CEM_EVENTS_PROJECTION_SCHEMA_URI,
        }
    }

    fn matches_schema(self, schema: &str) -> bool {
        schema == self.schema() || matches!(self, Self::Dom) && schema == DOM_JSON_PROJECTION_SCHEMA
    }
}

pub(super) fn validate(input: &EngineInput, context: &EngineContext) -> Option<Vec<Diagnostic>> {
    let identity = input
        .identity
        .clone()
        .or_else(|| input.root_scope.format_identity_option())
        .unwrap_or_else(|| FormatIdentity::from(context));
    let schema = match identity.content_type.as_deref() {
        Some(content_type) => context
            .schema_registry
            .resolve_content_type(content_type)
            .ok()?
            .schema_uri
            .as_str(),
        None => identity.schema.as_deref()?.trim(),
    };
    let projection = Projection::from_schema(schema)?;
    if identity
        .schema
        .as_deref()
        .is_some_and(|schema| !projection.matches_schema(schema.trim()))
    {
        return None;
    }

    #[cfg(feature = "external-schema-assets")]
    if let Some(message) = crate::schema::package_sources::missing_external_source(
        Some(context),
        Some(projection.schema()),
        identity.content_type.as_deref(),
    ) {
        return Some(vec![Diagnostic {
            uri: Some(input.uri.clone()),
            code: "cem.schema_asset.missing".into(),
            severity: crate::diagnostics::Severity::Error,
            message,
            ..Diagnostic::default()
        }]);
    }

    let content_type = identity.content_type.as_deref();
    Some(match projection {
        Projection::Dom => cem_dom_projection::validate_cem_dom_projection_source_bytes(
            cem_dom_projection::CemDomProjectionSourceValidationRequest {
                bytes: &input.bytes,
                source_uri: &input.uri,
                content_type,
            },
        ),
        Projection::Ast => cem_ast_projection::validate_cem_ast_projection_source_bytes(
            cem_ast_projection::CemAstProjectionSourceValidationRequest {
                bytes: &input.bytes,
                source_uri: &input.uri,
                content_type,
            },
        ),
        Projection::Events => cem_events_projection::validate_cem_events_projection_source_bytes(
            cem_events_projection::CemEventsProjectionSourceValidationRequest {
                bytes: &input.bytes,
                source_uri: &input.uri,
                content_type,
            },
        ),
    })
}
