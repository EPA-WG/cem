use super::*;
use crate as cem_ql;
#[path = "../fixtures/component-application/named_receiver_types.rs"]
mod fixture;
#[test]
fn pinned_scalar_types_reload_with_original_schema_source_and_receiver_intersection() {
    let types = fixture::types();
    let expected = types[0].clone();
    let built = build(fixture::declaration(), types).unwrap();
    let admitted = admit(&built.bytes, &built.metadata_json, &built.metadata_hash).unwrap();
    assert_eq!(admitted.metadata.tag, "cem-owned-types");
    assert_eq!(admitted.types["cart-count"], expected.value);
    assert_eq!(
        admitted.types["cart-count"].model.source_map,
        expected.value.model.source_map
    );
    let parsed: Metadata = serde_json::from_str(&built.metadata_json).unwrap();
    assert_eq!(parsed.types["cart-count"].source_uri, expected.source_uri);
    assert_eq!(parsed.types["cart-count"].schema_uri, expected.schema_uri);
    assert_eq!(parsed.types["cart-count"].source_hash, expected.source_hash);
    // Full native renderer proves the same contract conversion/intersection.
    use crate::{
        eval::{AtomValue, Item, ItemStream},
        render::{render_compiled_template, render_plan_to_html, TemplateData},
    };
    let mut data = TemplateData::default()
        .with_binding(
            "quantity",
            ItemStream::once(Item::Atomic(AtomValue::String("003".into()))),
        )
        .with_binding(
            "instanceID",
            ItemStream::once(Item::Atomic(AtomValue::String("named".into()))),
        );
    data.bindings.insert(
        "datadom".into(),
        ItemStream::once(Item::Record(BTreeMap::new())),
    );
    data.value_types = admitted.types;
    let plan = render_compiled_template(&admitted.artifact, &data);
    assert!(
        !plan
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation()),
        "{:?}",
        plan.diagnostics
    );
    assert!(render_plan_to_html(&plan).contains("3/3/ABC"));
}
#[test]
fn build_refuses_missing_duplicate_unreferenced_or_unportable_types() {
    for mutation in 0..6 {
        let mut types = fixture::types();
        match mutation {
            0 => {
                types.pop();
            }
            1 => types.push(types[0].clone()),
            2 => types[0].name = "unused".into(),
            3 => types[0].value.model.native_type_pending = true,
            4 => types[0].value.model.value_type = Some("node".into()),
            _ => types[0].source_hash = "changed".into(),
        }
        assert!(build(fixture::declaration(), types).is_err());
    }
}
#[test]
fn changed_missing_and_relabelled_contracts_refuse_even_with_new_payload_pin() {
    let built = build(fixture::declaration(), fixture::types()).unwrap();
    for mutation in 0..6 {
        let mut payload: Payload = rmp_serde::from_slice(&built.bytes[MAGIC.len()..]).unwrap();
        let mut metadata: Metadata = serde_json::from_str(&built.metadata_json).unwrap();
        match mutation {
            0 => {
                payload.types.pop();
            }
            1 => payload.types[0].value.model.min_inclusive = Some("1".into()),
            2 => {
                payload.types[0].source_hash =
                    cem_ml::content_cache::ContentHash::from_blake3(b"other schema").header_value()
            }
            3 => payload.types.push(payload.types[0].clone()),
            4 => payload.types[0].name = "string".into(),
            _ => {
                metadata.types.clear();
                payload.types.clear();
            }
        }
        let body = rmp_serde::to_vec_named(&payload).unwrap();
        let bytes = [MAGIC, &body].concat();
        metadata.payload_hash =
            cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value();
        let json = serde_json::to_string(&metadata).unwrap();
        let pin = cem_ml::content_cache::ContentHash::from_blake3(json.as_bytes()).header_value();
        assert!(admit(&bytes, &json, &pin).is_err(), "mutation {mutation}");
    }
    assert!(admit(&built.bytes, &built.metadata_json, "stale").is_err());
    assert!(crate::declaration_artifact::admit_declaration_metadata(
        &built.metadata_json,
        &built.metadata_hash
    )
    .is_err());
}

#[cfg(feature = "component-runtime")]
#[test]
fn selected_owner_binds_pinned_types_for_inputs_islands_and_native_projection() {
    use crate::{
        application_runtime::owners::CompiledDeclarationOwner,
        eval::{output::output_attribute, portable::encode_values, AtomValue, Item, ItemStream},
        render::RenderPlanAttribute,
    };
    let built = build(fixture::declaration(), fixture::types()).unwrap();
    let owner =
        CompiledDeclarationOwner::admit(&built.bytes, &built.metadata_json, &built.metadata_hash)
            .unwrap();
    let input = |text: &str| {
        encode_values(
            &ItemStream::once(output_attribute(RenderPlanAttribute {
                name: "quantity".into(),
                value: text.into(),
                value_stream: ItemStream::once(Item::Atomic(AtomValue::String(text.into()))),
                namespace: None,
                qualified_name: None,
                contract: None,
                source_map: Default::default(),
            })),
            &Default::default(),
        )
        .unwrap()
    };
    let route = r#"[{"target":"attribute","name":"quantity","index":0}]"#;
    assert!(owner
        .render_instance_inputs_native(&input("003"), route, "named")
        .unwrap()
        .contains("3/3/ABC"));
    for bad in ["1", "5", "not a number"] {
        assert_eq!(
            owner
                .render_instance_inputs_native(&input(bad), route, "named")
                .unwrap_err(),
            "compiled application render failed"
        );
    }
    assert!(owner
        .render_instance_inputs_native(&input("004"), route, "next")
        .unwrap()
        .contains("4/4/ABC"));
    let source = include_str!("../fixtures/component-application/island-domains.xhtml");
    let tree = cem_ml::import::import_data_bytes(
        source.as_bytes(),
        "application/xml",
        "cem",
        "app://types/state.xhtml",
    )
    .unwrap();
    let bytes = encode_values(
        &ItemStream::once(crate::eval::imported_cem_tree(tree)),
        &Default::default(),
    )
    .unwrap();
    assert!(owner
        .render_island_native(&bytes, "island-one")
        .unwrap()
        .contains("3/3/ABC"));
    let exported = owner.project_island_native(&bytes, "island-one").unwrap();
    assert!(exported.starts_with(b"CEMV\x03"));
    assert!(owner.render_island_native(&exported, "island-one").is_err());
    assert!(owner
        .render_native(b"{}", "app://types/response.json", "datadom")
        .is_err());
}

#[cfg(feature = "component-runtime")]
#[test]
fn named_defaults_are_bound_for_native_json_without_schema_loading() {
    use crate::{
        application_runtime::owners::CompiledDeclarationOwner,
        declaration_artifact::build_declaration_artifact,
        template_artifact::TemplateArtifactSourceMapMode,
    };
    let source = r#"<cem-element tag="cem-owned-defaults"><template id="cem-owned-defaults" type="text/cem-ml">{attribute @name=quantity @type=cart-count | 003}{attribute @name=label @type=cart-label | ABC}{p | {$quantity}/{$label}}</template></cem-element>"#;
    let declaration = build_declaration_artifact(
        source,
        "https://app.test/components/defaults.xhtml",
        TemplateArtifactSourceMapMode::Prod,
        None,
    )
    .unwrap();
    let built = build(declaration, fixture::types()).unwrap();
    let mut owner =
        CompiledDeclarationOwner::admit(&built.bytes, &built.metadata_json, &built.metadata_hash)
            .unwrap();
    assert_eq!(
        owner
            .render_native(b"{}", "app://data/response.json", "datadom")
            .unwrap(),
        "<p>3/ABC</p>"
    );
    owner.dispose_native();
    assert!(owner
        .render_native(b"{}", "app://data/response.json", "datadom")
        .is_err());
}

#[test]
fn native_wasm_cases_reject_newly_pinned_but_inconsistent_contracts() {
    let built = build(fixture::declaration(), fixture::types()).unwrap();
    for (name, bytes, json, pin) in fixture::invalid_cases(&built) {
        assert!(admit(&bytes, &json, &pin).is_err(), "{name}");
    }
    assert!(admit(&built.bytes, &built.metadata_json, &built.metadata_hash).is_ok());
}

#[test]
fn consistent_integrity_pins_do_not_authorize_executable_or_node_contracts() {
    let built = build(fixture::declaration(), fixture::types()).unwrap();
    let cases = fixture::invalid_cases(&built);
    for name in [
        "node",
        "list",
        "content-type",
        "restriction-node",
        "restriction-pending",
        "policy",
    ] {
        let (_, bytes, json, pin) = cases.iter().find(|case| case.0 == name).unwrap();
        let metadata: Metadata = serde_json::from_str(json).unwrap();
        let payload: Payload = rmp_serde::from_slice(&bytes[MAGIC.len()..]).unwrap();
        assert_eq!(hash(json.as_bytes()), *pin, "{name}");
        assert_eq!(hash(bytes), metadata.payload_hash, "{name}");
        assert_eq!(
            rmp_serde::to_vec_named(&payload).unwrap(),
            bytes[MAGIC.len()..]
        );
        for ty in &payload.types {
            assert_eq!(
                hash(&rmp_serde::to_vec_named(ty).unwrap()),
                metadata.types[&ty.name].contract_hash,
                "{name}"
            );
        }
        let (_, artifact) = load_template(
            &payload.template,
            &metadata.declaration_json,
            &metadata.declaration_hash,
        )
        .unwrap();
        assert_eq!(
            required(&artifact).unwrap(),
            metadata.types.keys().cloned().collect()
        );
        assert_eq!(
            admit(bytes, json, pin).err().as_deref(),
            Some(INVALID),
            "{name}"
        );
        assert_eq!(
            build(fixture::declaration(), payload.types)
                .err()
                .as_deref(),
            Some(INVALID),
            "{name}"
        );
    }
}

#[test]
fn executable_extensions_and_future_versions_cannot_reinterpret_scalar_envelopes() {
    let built = build(fixture::declaration(), fixture::types()).unwrap();
    for (name, bytes, json, pin) in fixture::invalid_cases(&built)
        .into_iter()
        .filter(|case| matches!(case.0, "callback-extension" | "future-envelope"))
    {
        let metadata: Metadata = serde_json::from_str(&json).unwrap();
        assert_eq!(hash(json.as_bytes()), pin, "{name}");
        assert_eq!(hash(&bytes), metadata.payload_hash, "{name}");
        assert_eq!(admit(&bytes, &json, &pin).err().as_deref(), Some(INVALID));
        #[cfg(feature = "component-runtime")]
        assert!(
            crate::application_runtime::owners::CompiledDeclarationOwner::admit(
                &bytes, &json, &pin
            )
            .is_err(),
            "{name}"
        );
    }
    // Refusing an unsupported artifact cannot poison normal scalar admission.
    assert!(admit(&built.bytes, &built.metadata_json, &built.metadata_hash).is_ok());
}

#[test]
fn named_capsules_cannot_replace_shipped_type_names_or_schema_namespace() {
    use crate::{
        declaration_artifact::build_declaration_artifact,
        template_artifact::TemplateArtifactSourceMapMode,
    };
    for name in ["uri", "path", "date", "schema:custom"] {
        let source=format!("<cem-element tag=\"cem-type-guard\"><template id=\"cem-type-guard\" type=\"text/cem-ml\">{{attribute @name=value @type={name}}}{{p | {{$value}}}}</template></cem-element>");
        let declaration = build_declaration_artifact(
            &source,
            "https://app.test/guard.xhtml",
            TemplateArtifactSourceMapMode::Prod,
            None,
        )
        .unwrap();
        let mut contract = fixture::types().remove(0);
        contract.name = name.into();
        assert!(build(declaration, vec![contract]).is_err(), "{name}");
    }
}
