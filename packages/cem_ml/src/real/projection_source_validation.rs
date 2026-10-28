//! Projection validation uses the same retained import and integrity contract as
//! lifecycle Parse/Inspect/conversion. It grants no runtime replay authority.

use crate::diagnostics::Diagnostic;
use crate::engine::{EngineContext, EngineInput, FormatIdentity};
use crate::import::projection::ProjectionKind;

pub(super) fn validate(input: &EngineInput, context: &EngineContext) -> Option<Vec<Diagnostic>> {
    let identity = input
        .identity
        .clone()
        .or_else(|| input.root_scope.format_identity_option())
        .unwrap_or_else(|| FormatIdentity::from(context));
    let projection =
        ProjectionKind::identity(identity.schema.as_deref(), identity.content_type.as_deref())?;
    if identity
        .schema
        .as_deref()
        .is_some_and(|s| ProjectionKind::identity(Some(s), None) != Some(projection))
    {
        return Some(vec![Diagnostic {
            uri: Some(input.uri.clone()),
            code: format!("cem.projection.{}.projection_mismatch", projection.name()),
            severity: crate::diagnostics::Severity::Error,
            message: "Projection schema/MIME mismatch".into(),
            ..Default::default()
        }]);
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

    Some(
        match crate::import::projection::load(
            &input.bytes,
            &input.uri,
            identity.content_type.as_deref(),
            projection,
        ) {
            Ok(_) => vec![],
            Err(error) => vec![error.diagnostic(projection, &input.uri)],
        },
    )
}
