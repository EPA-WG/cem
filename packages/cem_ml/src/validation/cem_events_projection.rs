//! Projection validation shares the bounded native source decoder.
use crate::diagnostics::Diagnostic;
use crate::import::projection::{load, ProjectionKind};
#[cfg(test)]
use crate::schema::registry::{
    CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_CONTENT_TYPE,
};
#[derive(Debug, Clone, Copy)]
pub struct CemEventsProjectionSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}
pub fn validate_cem_events_projection_source_bytes(
    request: CemEventsProjectionSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    match load(
        request.bytes,
        request.source_uri,
        request.content_type,
        ProjectionKind::Events,
    ) {
        Ok(_) => vec![],
        Err(error) => vec![error.diagnostic(ProjectionKind::Events, request.source_uri)],
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn validate(bytes: &[u8], content_type: &str) -> Vec<Diagnostic> {
        validate_cem_events_projection_source_bytes(CemEventsProjectionSourceValidationRequest {
            bytes,
            source_uri: "fixture.events",
            content_type: Some(content_type),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn cem_events_projection_validator_accepts_binary_fixture() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-events-projection/v1/examples/basic-events.cem-bin"
            ),
            CEM_EVENTS_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_events_projection_validator_reports_binary_magic() {
        let diagnostics = validate(b"not-a-cem-projection", CEM_EVENTS_PROJECTION_CONTENT_TYPE);

        assert!(has_code(&diagnostics, "cem.projection.events.binary_magic"));
    }

    #[test]
    fn cem_events_projection_validator_accepts_basic_json() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-events-projection/v1/examples/basic-events.events.json"
            ),
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_events_projection_validator_accepts_nested_json() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-events-projection/v1/examples/nested-events.events.json"
            ),
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_events_projection_validator_reports_invalid_kind_json_shape() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-events-projection/v1/examples/invalid-kind.events.json"
            ),
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(has_code(&diagnostics, "cem.projection.events.json_shape"));
    }
}
