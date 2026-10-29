use cem_ml::content_cache::ContentHash;
use cem_ql::{
    declaration_artifact::{admit_declaration_metadata, build_declaration_artifact},
    template_artifact::TemplateArtifactSourceMapMode,
};
use std::{fs, path::PathBuf};

#[test]
fn canonical_cart_declarations_compile_with_intrinsic_metadata_in_both_modes() {
    for mode in [
        TemplateArtifactSourceMapMode::Dev,
        TemplateArtifactSourceMapMode::Prod,
    ] {
        for (tag, capability) in [
            ("cem-action", Some("action-command")),
            ("cem-text-field", Some("form-control")),
            ("cem-card", None),
            ("cem-text", None),
            ("cem-alert", None),
        ] {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                "../cem-components/src/components/{tag}/{tag}.xhtml"
            ));
            let source = fs::read_to_string(path).unwrap();
            let built = build_declaration_artifact(
                &source,
                &format!("https://app.test/components/{tag}.xhtml"),
                mode,
                None,
            )
            .unwrap();
            let metadata =
                admit_declaration_metadata(&built.metadata_json, &built.metadata_hash).unwrap();
            assert_eq!(metadata.tag, tag);
            assert_eq!(metadata.capability.as_deref(), capability);
            assert_eq!(
                metadata.declaration_source_hash,
                ContentHash::from_blake3(source.as_bytes()).header_value()
            );
            let loaded = built.template.reload_pinned(&metadata.template).unwrap();
            assert!(!loaded.stylesheets.is_empty());
            assert!(loaded.diagnostics.is_empty());
        }
    }
}

#[test]
fn metadata_preserves_bases_alias_scope_and_runtime_capability_precedence() {
    let source = r#"<cem-element tag="cem-example" version="1" scope="shop" capability="form-control" link-base="source" xml:base="../shared/">
      <template id="cem-example" type="text/cem-ml" capability="action-command" xml:base="ui/">{style @scope=shop | ``` :scope { display:block; } ```}{p | Hello}</template>
    </cem-element>"#;
    let built = build_declaration_artifact(
        source,
        "file:///shop/components/view.xhtml",
        TemplateArtifactSourceMapMode::Prod,
        Some("cem-alias"),
    )
    .unwrap();
    let metadata = admit_declaration_metadata(&built.metadata_json, &built.metadata_hash).unwrap();
    assert_eq!(metadata.tag, "cem-alias");
    assert_eq!(metadata.declared_tag, "cem-example");
    assert_eq!(metadata.resource_base_url, "file:///shop/shared/ui/");
    assert_eq!(metadata.capability.as_deref(), Some("form-control"));
    assert_eq!(metadata.scope.as_deref(), Some("shop"));
    assert_eq!(metadata.version.as_deref(), Some("1"));
    assert_eq!(metadata.link_base, "source");
    assert!(
        admit_declaration_metadata(&(built.metadata_json.clone() + " "), &built.metadata_hash)
            .is_err()
    );
    assert!(admit_declaration_metadata(&built.metadata_json, "").is_err());
    let original: serde_json::Value = serde_json::from_str(&built.metadata_json).unwrap();
    let mut extra = original.clone();
    extra["unknown"] = true.into();
    let json = extra.to_string();
    let hash = ContentHash::from_blake3(json.as_bytes()).header_value();
    assert!(admit_declaration_metadata(&json, &hash).is_err());
    let mut missing = original.clone();
    missing.as_object_mut().unwrap().remove("tag");
    let json = missing.to_string();
    let hash = ContentHash::from_blake3(json.as_bytes()).header_value();
    assert!(admit_declaration_metadata(&json, &hash).is_err());
    let mut legacy = original.clone(); legacy.as_object_mut().unwrap().remove("templateFormat");
    let json = legacy.to_string(); let hash = ContentHash::from_blake3(json.as_bytes()).header_value();
    assert_eq!(admit_declaration_metadata(&json, &hash).unwrap().template_format, "cem-template-artifact/1");
    legacy["templateFormat"] = "cem-template-artifact/99".into();
    let json = legacy.to_string(); let hash = ContentHash::from_blake3(json.as_bytes()).header_value();
    assert!(admit_declaration_metadata(&json, &hash).is_err());
    let mut value = original;
    value["template"]["cemQlVersion"] = "foreign".into();
    let json = value.to_string();
    let hash = ContentHash::from_blake3(json.as_bytes()).header_value();
    assert!(admit_declaration_metadata(&json, &hash).is_err());
}

#[test]
fn build_refuses_unsupported_or_invalid_declarations_without_source_fallback() {
    for source in [
        "<cem-element tag=\"cem-x\"><template id=\"cem-x\" type=\"text/cem-ml\">{$missing}</template></cem-element>",
        "<cem-element tag=\"cem-x\"><template id=\"cem-x\" type=\"text/cem-ml\" xpath-functions=\"x.cemt\">Hello</template></cem-element>",
        "<cem-element tag=\"cem-x\"><template id=\"other\" type=\"text/cem-ml\">Hello</template></cem-element>",
        "<cem-element tag=\"cem-x\"><template id=\"cem-x\" type=\"text/cem-ml\">{location-element}</template></cem-element>",
        "<cem-element tag=\"cem-x\"><template id=\"cem-x\" type=\"text/cem-ml\">{module | {import @as=remote @src=./other.cemt}{body | Hello}}</template></cem-element>",
    ] { assert!(build_declaration_artifact(source, "https://app.test/x.xhtml", TemplateArtifactSourceMapMode::Prod, None).is_err(), "{source}"); }
}

#[test]
fn declaration_metadata_retains_static_module_maps_without_source_loading() {
    let source = r#"<cem-element tag="cem-map"><template id="cem-map" type="text/cem-ml">{module-map | {resource @specifier="image" @target="./image.svg" @content-type="image/svg+xml"}}{p | Image}</template></cem-element>"#;
    let built = build_declaration_artifact(
        source,
        "https://app.test/map.xhtml",
        TemplateArtifactSourceMapMode::Prod,
        None,
    )
    .unwrap();
    let metadata = admit_declaration_metadata(&built.metadata_json, &built.metadata_hash).unwrap();
    let loaded = built.template.reload_pinned(&metadata.template).unwrap();
    assert_eq!(metadata.module_map, loaded.module_map);
    assert_eq!(
        metadata.module_map.unwrap().specifiers.resources["image"]
            .content_type_hint
            .as_deref(),
        Some("image/svg+xml")
    );
}

#[test]
fn compiled_css_build_preserves_xml_bases_and_fails_missing_dependencies() {
    use cem_ql::declaration_artifact::build_declaration_imports;
    let source = r#"<cem-element tag="cem-imported" xml:base="../shared/"><template id="cem-imported" type="text/cem-ml" xml:base="styles/">{style | ``` @import './main.css'; :scope {display:block} ```}{p | Imported}</template></cem-element>"#;
    let build = || {
        build_declaration_artifact(
            source,
            "file:///app/components/view.xhtml",
            TemplateArtifactSourceMapMode::Prod,
            None,
        )
        .unwrap()
    };
    assert!(
        build_declaration_imports(build(), &mut |_| Err("Missing build dependency".into()))
            .err()
            .unwrap()
            .contains("Missing build dependency")
    );
    let mut requests = Vec::new();
    let built = build_declaration_imports(build(), &mut |request| {
        requests.push(request.resolution.resolved_url.clone());
        Ok(cem_ml::resolver::ResolvedRead {
            uri: request.resolution.resolved_url.clone(),
            bytes: b":scope {color:red}".to_vec(),
            content_type: Some("text/css".into()),
        })
    })
    .unwrap();
    assert_eq!(requests, vec!["file:///app/shared/styles/main.css"]);
    let metadata = admit_declaration_metadata(&built.metadata_json, &built.metadata_hash).unwrap();
    assert_eq!(metadata.template_format, "cem-template-artifact/3");
    assert_eq!(metadata.resource_base_url, "file:///app/shared/styles/");
    assert!(built.template.reload_pinned(&metadata.template).is_ok());
}
