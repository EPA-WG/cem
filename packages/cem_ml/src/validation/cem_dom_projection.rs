//! Projection validation shares the bounded native source decoder.
use crate::diagnostics::Diagnostic;
use crate::import::projection::{load, ProjectionKind};
#[cfg(test)]
use crate::schema::registry::{
    CEM_DOM_JSON_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_CONTENT_TYPE,
};
#[derive(Debug, Clone, Copy)]
pub struct CemDomProjectionSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}
pub fn validate_cem_dom_projection_source_bytes(
    request: CemDomProjectionSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    match load(
        request.bytes,
        request.source_uri,
        request.content_type,
        ProjectionKind::Dom,
    ) {
        Ok(_) => vec![],
        Err(error) => vec![error.diagnostic(ProjectionKind::Dom, request.source_uri)],
    }
}
#[cfg(test)]
fn validate_cem_dom_projection_binary_bytes(bytes: &[u8]) -> Result<(), String> {
    load(
        bytes,
        "fixture.dom",
        Some(CEM_DOM_PROJECTION_CONTENT_TYPE),
        ProjectionKind::Dom,
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}
#[cfg(test)]
fn validate_cem_tree_json_node(value: &serde_json::Value, _: &str, _: &str) -> Result<(), String> {
    load(
        &serde_json::to_vec(value).unwrap(),
        "fixture.dom",
        Some(CEM_DOM_JSON_PROJECTION_CONTENT_TYPE),
        ProjectionKind::Dom,
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    #[test]
    fn native_attribute_projection_validates_retained_values() {
        use crate::{
            events::cem::CemEventNormalizer,
            parser::builder::CemAstBuilder,
            source::{BytesSource, SourceId},
            tokenizer::cem::CemTokenizer,
        };
        let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
            BytesSource::new(SourceId(1), b"{item @native={#nodes}}".to_vec()),
        )))
        .build();
        let binary = crate::projection::dom_binary_projection_artifact(&document);
        assert!(validate_cem_dom_projection_binary_bytes(&binary.bytes).is_ok());
        let mut projected = crate::projection::dom_json(&document);
        assert!(validate_cem_tree_json_node(&projected, "$", "projection").is_ok());
        projected["children"][0]["attributes"][0]["valueNodes"][0]["targets"] =
            serde_json::json!(["invalid"]);
        assert!(validate_cem_tree_json_node(&projected, "$", "projection").is_err());
    }

    use super::*;

    fn validate(bytes: &[u8], content_type: &str) -> Vec<Diagnostic> {
        validate_cem_dom_projection_source_bytes(CemDomProjectionSourceValidationRequest {
            bytes,
            source_uri: "fixture.dom",
            content_type: Some(content_type),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn cem_dom_projection_validator_accepts_binary_fixture() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-dom-projection/v1/examples/basic-dom.cem-bin"
            ),
            CEM_DOM_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_dom_projection_validator_reports_binary_magic() {
        let diagnostics = validate(b"not-a-cem-projection", CEM_DOM_PROJECTION_CONTENT_TYPE);

        assert!(has_code(&diagnostics, "cem.projection.dom.binary_magic"));
    }

    #[test]
    fn cem_dom_projection_validator_accepts_basic_json() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-dom-projection/v1/examples/basic-dom.dom.json"
            ),
            CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_dom_projection_validator_accepts_nested_json() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-dom-projection/v1/examples/nested-dom.dom.json"
            ),
            CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_dom_projection_validator_reports_invalid_kind_json_shape() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-dom-projection/v1/examples/invalid-kind.dom.json"
            ),
            CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(has_code(&diagnostics, "cem.projection.dom.json_shape"));
    }
}
