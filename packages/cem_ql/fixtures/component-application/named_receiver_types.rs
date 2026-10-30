//! Build-only fixture: compile native schema attribute facets, not schema AST JSON.
use super::cem_ql;
use cem_ql::{
    application_declaration::NamedReceiverType,
    declaration_artifact::{build_declaration_artifact, BuiltDeclarationArtifact},
    template_artifact::TemplateArtifactSourceMapMode,
};
pub fn declaration() -> BuiltDeclarationArtifact {
    build_declaration_artifact(r#"<cem-element tag="cem-owned-types"><template id="cem-owned-types" type="text/cem-ml">{attribute @name=quantity @type=cart-count @required=true @maxInclusive=4}{attribute @name=label @type=cart-label | ABC}{p @id='{$instanceID}' @title='{$label}' | {$quantity}/{$datadom.attributes.quantity}/{$label}}</template></cem-element>"#, "https://app.test/components/types.xhtml",TemplateArtifactSourceMapMode::Prod,None).unwrap()
}
pub fn types() -> Vec<NamedReceiverType> {
    let source = r#"<schema xmlns="https://cem.dev/ns/schema/1" name="cart" namespace="https://app.test/schema/cart/1"><attributes><attribute name="cart-count" type="integer" minInclusive="2" maxInclusive="5"/><attribute name="cart-label" type="string" pattern="[A-Z]+" minLength="3" whiteSpace="collapse"/></attributes></schema>"#;
    let source_uri = "https://app.test/schema/cart.xml";
    let schema_uri = "https://app.test/schema/cart/1";
    let tree =
        cem_ml::import::import_data_bytes(source.as_bytes(), "application/xml", "cem", source_uri)
            .unwrap();
    let model = cem_ml::schema::document_model::compile_schema_document_model_from_retained(
        schema_uri, &tree,
    );
    assert!(
        !model
            .compile_diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        model.compile_diagnostics
    );
    ["cart-count", "cart-label"]
        .into_iter()
        .map(|name| NamedReceiverType {
            name: name.into(),
            schema_uri: schema_uri.into(),
            source_uri: source_uri.into(),
            source_hash: cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes())
                .header_value(),
            value: cem_ml::schema::document_model::AttributeValueContract {
                model: model.attributes[name].clone(),
                ..Default::default()
            },
        })
        .collect()
}

/// Malformed native capsule/control metadata, never a JSON AST projection.
pub fn invalid_cases(
    built: &cem_ql::application_declaration::BuiltApplicationDeclaration,
) -> Vec<(&'static str, Vec<u8>, String, String)> {
    use serde::{Deserialize, Serialize};
    #[derive(Serialize, Deserialize)]
    struct Payload {
        template: Vec<u8>,
        types: Vec<NamedReceiverType>,
    }
    let mut cases = Vec::new();
    for name in [
        "changed",
        "missing",
        "duplicate",
        "source",
        "schema",
        "unsupported",
        "unbound",
        "trailing",
        "stale",
    ] {
        let mut payload: Payload =
            rmp_serde::from_slice(&built.bytes[cem_ql::application_declaration::MAGIC.len()..])
                .unwrap();
        // Explicit API control metadata consists of identities/pins and an opaque
        // original declaration metadata string; it contains no source/data AST.
        let mut metadata: serde_json::Value = serde_json::from_str(&built.metadata_json).unwrap();
        match name {
            "changed" => payload.types[0].value.model.min_inclusive = Some("1".into()),
            "missing" => {
                payload.types.pop();
            }
            "duplicate" => payload.types.push(payload.types[0].clone()),
            "source" => {
                payload.types[0].source_hash =
                    cem_ml::content_cache::ContentHash::from_blake3(b"other source").header_value()
            }
            "schema" => payload.types[0].schema_uri = "https://other.test/schema/1".into(),
            "unsupported" => payload.types[0].value.model.native_type_pending = true,
            "unbound" => {
                let removed = payload.types.remove(0);
                metadata["types"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&removed.name);
            }
            _ => {}
        }
        let mut bytes = [
            cem_ql::application_declaration::MAGIC,
            &rmp_serde::to_vec_named(&payload).unwrap(),
        ]
        .concat();
        if name == "trailing" {
            bytes.push(0);
        }
        metadata["payloadHash"] = cem_ml::content_cache::ContentHash::from_blake3(&bytes)
            .header_value()
            .into();
        let json = serde_json::to_string(&metadata).unwrap();
        let pin = if name == "stale" {
            format!("cem-bin/1+blake3:{}", "0".repeat(64))
        } else {
            cem_ml::content_cache::ContentHash::from_blake3(json.as_bytes()).header_value()
        };
        cases.push((name, bytes, json, pin));
    }
    cases
}
