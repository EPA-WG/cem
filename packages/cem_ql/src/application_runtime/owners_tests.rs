use super::*;
use crate::declaration_artifact::build_declaration_artifact;
use crate::template_artifact::TemplateArtifactSourceMapMode;
fn fixture(body: &str) -> crate::declaration_artifact::BuiltDeclarationArtifact {
    build_declaration_artifact(&format!(r#"<cem-element tag="cem-owned-proof"><template id="cem-owned-proof" type="text/cem-ml">{body}</template></cem-element>"#), "https://app.test/components/proof.xhtml", TemplateArtifactSourceMapMode::Prod, None).unwrap()
}
fn context() -> String {
    serde_json::json!({"identity":"owned","resolverIdentity":"resolver","resourcePolicyStamp":"policy","frames":[{"frameId":"root","baseUrl":"https://app.test/components/","scopes":[],"specifiers":{"imports":{},"resources":{}}}]}).to_string()
}
#[test]
fn owns_native_declaration_styles_and_invalidates_last_lease_on_release_and_disposal() {
    let built = fixture("{style | ``` :host { --proof: ready; } ```}{p | Static}");
    let mut owner = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    let style = owner.stylesheet_native("first", 0, &context()).unwrap();
    assert!(style.emission().unwrap().css().contains("cem-owned-proof"));
    let same = owner.stylesheet_native("second", 0, &context()).unwrap();
    assert!(Arc::ptr_eq(&style, &same));
    assert_eq!(owner.release_native("first").unwrap(), 1);
    assert!(style.emission().is_ok());
    assert_eq!(owner.release_native("second").unwrap(), 1);
    assert!(style.emission().is_err());
    let next = owner.stylesheet_native("third", 0, &context()).unwrap();
    owner.dispose_native();
    assert!(next.emission().is_err());
    assert!(owner.describe_native().is_err());
    assert!(owner
        .render_native(b"{}", "app://data/response.json", "datadom")
        .is_err());
}
#[test]
fn refusal_does_not_publish_an_owner_or_replace_a_valid_consumer() {
    let built = fixture("{style | ``` :host { color:red; } ```}{p | Static}");
    assert!(
        CompiledDeclarationOwner::admit(&built.template.bytes, &built.metadata_json, "").is_err()
    );
    let mut corrupt = built.template.bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(
        CompiledDeclarationOwner::admit(&corrupt, &built.metadata_json, &built.metadata_hash)
            .is_err()
    );
    let mut owner = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    let style = owner.stylesheet_native("current", 0, &context()).unwrap();
    assert!(owner.stylesheet_native("current", 0, "{}").is_err());
    assert!(owner.stylesheet_native("current", 99, &context()).is_err());
    assert!(style.emission().is_ok());
    let external =
        fixture("{style | ``` @import './source.css'; :host {color:red} ```}{p | Static}");
    let mut external = CompiledDeclarationOwner::admit(
        &external.template.bytes,
        &external.metadata_json,
        &external.metadata_hash,
    )
    .unwrap();
    assert!(external.stylesheet_native("denied", 0, &context()).is_err());
    let capable = build_declaration_artifact(r#"<cem-element tag="cem-form-proof" capability="form-control"><template id="cem-form-proof" type="text/cem-ml">{input}</template></cem-element>"#,"https://app.test/form.xhtml",TemplateArtifactSourceMapMode::Prod,None).unwrap();
    assert!(CompiledDeclarationOwner::admit(
        &capable.template.bytes,
        &capable.metadata_json,
        &capable.metadata_hash
    )
    .is_err());
}
#[test]
fn renders_native_json_repeatedly_and_never_falls_back_to_unselected_functions() {
    let built = fixture("{p | {$datadom.children.children.children.children.value}}");
    let owner = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    for value in [
        br#"{"value":"First"}"#.as_slice(),
        br#"{"value":"Second <img>"}"#,
    ] {
        let html = owner
            .render_native(value, "app://data/response.json", "datadom")
            .unwrap();
        assert!(html.contains(if value == br#"{"value":"First"}"# {
            "First"
        } else {
            "Second &lt;img&gt;"
        }));
    }
    assert!(owner
        .render_native(b"{", "app://data/response.json", "datadom")
        .is_err());
    assert!(owner
        .render_native(b"{}", "app://data/response.json", "unknown")
        .is_err());
    let denied = fixture(r#"{p | {$ | str:upper("private value")}}"#);
    let denied = CompiledDeclarationOwner::admit(
        &denied.template.bytes,
        &denied.metadata_json,
        &denied.metadata_hash,
    )
    .unwrap();
    assert_eq!(
        denied
            .render_native(b"{}", "app://data/response.json", "datadom")
            .unwrap_err(),
        "compiled application render failed"
    );
}

#[test]
fn v3_replays_closed_imports_and_refuses_changed_resolution_without_losing_old_output() {
    use crate::template_artifact::compile_template_artifact_imports;
    let mut built =
        fixture("{style | ``` @import './theme.css'; :host { --root: yes; } ```}{p | Static}");
    let value: CemModuleUrlContext = serde_json::from_str(&context()).unwrap();
    let handle = CemResolutionContextHandle::new("build");
    let capability = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(handle.clone(), value)),
        handle,
    );
    built.template = compile_template_artifact_imports(
        built.template,
        "https://app.test/components/proof.xhtml",
        capability,
        &mut |request| {
            Ok(cem_ml::resolver::ResolvedRead {
                uri: request.resolution.resolved_url.clone(),
                bytes: b":host { --imported: ready; }".to_vec(),
                content_type: Some("text/css; mode=stylesheet".into()),
            })
        },
    )
    .unwrap();
    let mut meta: DeclarationArtifactMetadata = serde_json::from_str(&built.metadata_json).unwrap();
    meta.template_format = built.template.identity.artifact_version.clone();
    meta.template.content_hash = built.template.content_hash.header_value();
    built.metadata_json = serde_json::to_string(&meta).unwrap();
    built.metadata_hash =
        cem_ml::content_cache::ContentHash::from_blake3(built.metadata_json.as_bytes())
            .header_value();
    let mut owner = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    let output = owner.stylesheet_native("live", 0, &context()).unwrap();
    assert!(output
        .emission()
        .unwrap()
        .css()
        .contains("--imported:ready"));
    let mut changed: serde_json::Value = serde_json::from_str(&context()).unwrap();
    changed["frames"][0]["specifiers"]["resources"]["./theme.css"] =
        serde_json::json!({"target":"./changed.css"});
    assert!(owner
        .stylesheet_native("live", 0, &changed.to_string())
        .is_err());
    assert!(output.emission().is_ok());
    owner.release_native("live").unwrap();
    assert!(output.emission().is_err());
}

#[test]
fn metadata_cannot_relabel_the_compiled_template_format() {
    let built = fixture("{p | Static}");
    let mut meta: DeclarationArtifactMetadata = serde_json::from_str(&built.metadata_json).unwrap();
    meta.template_format = "cem-template-artifact/3".into();
    let json = serde_json::to_string(&meta).unwrap();
    let pin = cem_ml::content_cache::ContentHash::from_blake3(json.as_bytes()).header_value();
    assert_eq!(
        CompiledDeclarationOwner::admit(&built.template.bytes, &json, &pin)
            .err()
            .unwrap(),
        "compiled declaration template format mismatch"
    );
}

#[test]
fn native_inputs_bind_attributes_and_slices_without_json_or_stale_state() {
    use crate::eval::{portable::encode_values, AtomValue, Item, ItemStream};
    use cem_ml::value::artifact::CemValueArtifactLimits;
    let built = fixture("{attribute @name=label}{p @title='{$label}' | {$datadom.attributes.label} / {$datadom.slices.amount}}");
    let owner = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    let values = |label: &str, amount: i64| {
        let input = crate::render::RenderPlanAttribute {
            name: "label".into(),
            value: label.into(),
            value_stream: ItemStream::once(Item::Atomic(AtomValue::String(label.into()))),
            namespace: None,
            qualified_name: None,
            contract: None,
            source_map: Default::default(),
        };
        encode_values(
            &ItemStream::from_items(vec![
                crate::eval::output::output_attribute(input),
                Item::Atomic(AtomValue::Integer(amount)),
            ]),
            &CemValueArtifactLimits::default(),
        )
        .unwrap()
    };
    let bindings = r#"[{"name":"label","index":0,"target":"attribute"},{"name":"amount","index":1,"target":"slice"}]"#;
    let first = owner
        .render_inputs_native(&values("Tea <img>", 2), bindings)
        .unwrap();
    assert!(first.contains("title=\"Tea &lt;img&gt;\""), "{first}");
    assert!(first.contains("Tea &lt;img&gt; / 2"), "{first}");
    assert!(owner
        .render_inputs_native(
            &values("Later", 3),
            r#"[{"name":"label","index":1,"target":"attribute"}]"#
        )
        .is_err());
    let next = owner
        .render_inputs_native(&values("Later", 3), bindings)
        .unwrap();
    assert!(next.contains("Later / 3"), "{next}");
    assert!(!next.contains("Tea"));
    let empty = encode_values(&ItemStream::empty(), &CemValueArtifactLimits::default()).unwrap();
    let blank = owner.render_inputs_native(&empty, "[]").unwrap();
    assert!(!blank.contains("Later"));
}

#[test]
fn json_inputs_cannot_replace_runtime_owned_roots() {
    let built = fixture("{p | Static}");
    let owner = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    for binding in ["instanceID", "island", "suggestions"] {
        assert!(
            owner
                .render_native(b"{}", "app://data/response.json", binding)
                .is_err(),
            "{binding}"
        );
    }
    assert!(owner
        .render_native(b"{}", "app://data/response.json", "datadom")
        .is_ok());
}

#[test]
fn instance_inputs_validate_compiled_receivers_and_never_retain_identity_or_failed_values() {
    use crate::eval::{
        output::output_attribute, portable::encode_values, AtomValue, Item, ItemStream,
    };
    let built = fixture(
        r#"{attribute @name=quantity @type=integer @required=true @minInclusive=1 @maxInclusive=5}{p @id="{$instanceID}" | {$quantity}/{$datadom.attributes.quantity}/{$instanceID}}"#,
    );
    let mut owner = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    let bytes = |value: &str| {
        encode_values(
            &ItemStream::once(output_attribute(crate::render::RenderPlanAttribute {
                name: "quantity".into(),
                namespace: None,
                qualified_name: None,
                value: "wrong compatibility projection".into(),
                value_stream: ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
                contract: None,
                source_map: Default::default(),
            })),
            &Default::default(),
        )
        .unwrap()
    };
    let routing = r#"[{"target":"attribute","name":"quantity","index":0}]"#;
    for id in ["first", "second", "cart:é"] {
        let html = owner
            .render_instance_inputs_native(&bytes("003"), routing, id)
            .unwrap();
        assert!(html.contains(&format!("id=\"{id}\"")), "{html}");
        assert!(html.contains(&format!("3/3/{id}")), "{html}");
        for value in ["0", "6", "private credential"] {
            assert_eq!(
                owner
                    .render_instance_inputs_native(&bytes(value), routing, id)
                    .unwrap_err(),
                "compiled application render failed"
            );
        }
    }
    let empty = encode_values(&ItemStream::empty(), &Default::default()).unwrap();
    assert_eq!(
        owner
            .render_instance_inputs_native(&empty, "[]", "missing")
            .unwrap_err(),
        "compiled application render failed"
    );
    for id in ["", " ", "a b", "line\nbreak", "\0", &"x".repeat(1025)] {
        assert_eq!(
            owner
                .render_instance_inputs_native(&bytes("2"), routing, id)
                .unwrap_err(),
            "compiled declaration instance identity refused"
        );
    }
    let valid = owner
        .render_instance_inputs_native(&bytes("2"), routing, "valid")
        .unwrap();
    assert!(valid.contains("2/2/valid"));
    assert_eq!(
        owner
            .render_inputs_native(&bytes("2"), routing)
            .unwrap_err(),
        "compiled application render failed"
    );
    let escaped = owner
        .render_instance_inputs_native(&bytes("2"), routing, "quoted\"<svg>")
        .unwrap();
    assert!(!escaped.contains("<svg>"));
    assert!(escaped.contains("&lt;svg&gt;"));
    owner.dispose_native();
    assert!(owner
        .render_instance_inputs_native(&bytes("2"), routing, "disposed")
        .is_err());
}

#[test]
fn compiled_receiver_defaults_and_unresolved_types_keep_native_contracts() {
    let empty = crate::eval::portable::encode_values(
        &crate::eval::ItemStream::empty(),
        &Default::default(),
    )
    .unwrap();
    for (body, expected) in [
        ("{attribute @name=quantity @type=integer @minInclusive=1 | 002}{p | {$quantity}/{$datadom.attributes.quantity}}", Some("2/2")),
        ("{attribute @name=quantity @type=integer @maxInclusive=1 | 002}{p | {$quantity}}", None),
        ("{attribute @name=quantity @type=warehouse-count | 002}{p | {$quantity}}", None),
    ] {
        let built = fixture(body);
        let owner = CompiledDeclarationOwner::admit(&built.template.bytes, &built.metadata_json, &built.metadata_hash).unwrap();
        let result = owner.render_instance_inputs_native(&empty, "[]", "default-owner");
        if let Some(expected) = expected { assert!(result.unwrap().contains(expected)); }
        else { assert_eq!(result.unwrap_err(), "compiled application render failed"); }
    }
}
