//! Native-only integration fixtures for the isolated application WASM prototype.
use cem_ql::{
    eval::{imported_cem_tree, ItemStream},
    render::{render_compiled_template, render_plan_to_html, CompileTemplateOptions, TemplateData},
    template_artifact::{
        compile_template_artifact, compile_template_artifact_with_compiled_styles,
        PinnedTemplateArtifactContext, TemplateArtifactSourceMapMode,
    },
};
use std::{fs, path::PathBuf};
fn main() {
    let output = PathBuf::from(std::env::args().nth(1).expect("fixture output directory"));
    fs::create_dir_all(&output).unwrap();
    let options = CompileTemplateOptions {
        host_bindings: vec!["response".into()],
        ..Default::default()
    };
    let cart = include_str!("../fixtures/component-application/shopping-cart/cart-view.cem");
    for (name, source) in [
        ("cart", cart),
        ("denied", r#"{p | {$ | str:upper("private input")}}"#),
        ("plain", "{p | {$response.kind}}"),
    ] {
        let compiled = compile_template_artifact_with_compiled_styles(
            source,
            &options,
            TemplateArtifactSourceMapMode::Prod,
        )
        .unwrap();
        let pins = PinnedTemplateArtifactContext {
            content_hash: compiled.content_hash.header_value(),
            source_hash: compiled.identity.source_hash.header_value(),
            cem_ml_version: cem_ml::VERSION.into(),
            cem_ql_version: cem_ql::VERSION.into(),
            host_bindings: options.host_bindings.clone(),
            source_map_mode: TemplateArtifactSourceMapMode::Prod,
        };
        let admitted = compiled.reload_pinned(&pins).unwrap();
        assert!(
            admitted.diagnostics.is_empty(),
            "{name}: {:?}",
            admitted.diagnostics
        );
        fs::write(output.join(format!("{name}.bin")), &compiled.bytes).unwrap();
        fs::write(
            output.join(format!("{name}.pins.json")),
            serde_json::to_vec(&pins).unwrap(),
        )
        .unwrap();
        if name == "cart" {
            let loaded = compiled.reload_pinned(&pins).unwrap();
            for response in [
                "cart",
                "updated",
                "removed",
                "empty",
                "partial-error",
                "failed",
            ] {
                let json = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
                    "fixtures/component-application/shopping-cart/{response}.json"
                )))
                .unwrap();
                let tree = cem_ml::import::import_data_bytes(
                    &json,
                    "application/json",
                    "cem",
                    &format!("app://shopping-cart/{response}.json"),
                )
                .unwrap();
                let data = TemplateData::default()
                    .with_binding("response", ItemStream::once(imported_cem_tree(tree)));
                let plan = render_compiled_template(&loaded, &data);
                assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
                fs::write(
                    output.join(format!("{response}.html")),
                    render_plan_to_html(&plan),
                )
                .unwrap();
            }
        }
    }
    fs::write(
        output.join("legacy.bin"),
        compile_template_artifact(
            "{p | legacy}",
            &options,
            TemplateArtifactSourceMapMode::Prod,
        )
        .bytes,
    )
    .unwrap();
    {
        use cem_ml::module_resolution::*;
        use std::sync::Arc;
        let handle = CemResolutionContextHandle::new("runtime-fixture");
        let context = CemModuleUrlContext {
            identity: "app".into(),
            resolver_identity: "fixture".into(),
            resource_policy_stamp: "fixture".into(),
            frames: vec![CemModuleUrlFrame::new(
                "root",
                "https://app.test/view.xhtml",
            )],
        };
        let capability = CemModuleUrlResolutionCapability::new(
            Arc::new(CemScopedModuleUrlResolver::new().with_context(handle.clone(), context)),
            handle,
        );
        let v2 = compile_template_artifact_with_compiled_styles(
            "{p | {$response.kind}}",
            &options,
            TemplateArtifactSourceMapMode::Prod,
        )
        .unwrap();
        let v3 = cem_ql::template_artifact::compile_template_artifact_imports(
            v2,
            "https://app.test/view.xhtml",
            capability,
            &mut |_| Err("no requests permitted".into()),
        )
        .unwrap();
        let pins = PinnedTemplateArtifactContext {
            content_hash: v3.content_hash.header_value(),
            source_hash: v3.identity.source_hash.header_value(),
            cem_ml_version: cem_ml::VERSION.into(),
            cem_ql_version: cem_ql::VERSION.into(),
            host_bindings: options.host_bindings.clone(),
            source_map_mode: TemplateArtifactSourceMapMode::Prod,
        };
        fs::write(output.join("v3.bin"), v3.bytes).unwrap();
        fs::write(
            output.join("v3.pins.json"),
            serde_json::to_vec(&pins).unwrap(),
        )
        .unwrap();
    }
    owned_declarations(&output);
    // Trusted native fixture selection, not a claim that all cart component
    // behavior/provider dependencies have closed application build recipes.
    fs::write(output.join("selected_registry.rs"), "use cem_ql::selected_registry::{SelectedBuiltin, SelectedFunctionRegistry};\npub fn selected_registry() -> SelectedFunctionRegistry { SelectedFunctionRegistry::new([SelectedBuiltin::SequenceFirst, SelectedBuiltin::SequenceWhere, SelectedBuiltin::SequenceCount]).unwrap() }\n").unwrap();
}

fn owned_declarations(output: &std::path::Path) {
    use cem_ml::module_resolution::*;
    use cem_ql::{
        declaration_artifact::{build_declaration_artifact, DeclarationArtifactMetadata},
        template_artifact::compile_template_artifact_imports,
    };
    use std::sync::Arc;
    let uri = "https://app.test/components/proof.xhtml";
    for (name, imports) in [("owned-inline", false), ("owned-imports", true)] {
        let import = if imports {
            "@import './theme.css';"
        } else {
            ""
        };
        let source = format!(
            r#"<cem-element tag="cem-owned-proof" scope="owned-proof"><template id="cem-owned-proof" type="text/cem-ml">{{style | ``` {import} :host {{ --private-proof: ready; }} ```}}{{style @scope=owned-proof | ``` .label {{ --shared-proof: ready; }} ```}}{{p | {{$datadom.children.children.children.children.value}}}}</template></cem-element>"#
        );
        let mut built =
            build_declaration_artifact(&source, uri, TemplateArtifactSourceMapMode::Prod, None)
                .unwrap();
        if imports {
            let handle = CemResolutionContextHandle::new("build");
            let context = CemModuleUrlContext {
                identity: "owned".into(),
                resolver_identity: "resolver".into(),
                resource_policy_stamp: "policy".into(),
                frames: vec![CemModuleUrlFrame::new(
                    "root",
                    "https://app.test/components/",
                )],
            };
            let cap = CemModuleUrlResolutionCapability::new(
                Arc::new(CemScopedModuleUrlResolver::new().with_context(handle.clone(), context)),
                handle,
            );
            built.template =
                compile_template_artifact_imports(built.template, uri, cap, &mut |request| {
                    Ok(cem_ml::resolver::ResolvedRead {
                        uri: request.resolution.resolved_url.clone(),
                        bytes: b":host { --imported-proof: ready; }".to_vec(),
                        content_type: Some("text/css; mode=stylesheet".into()),
                    })
                })
                .unwrap();
            let mut meta: DeclarationArtifactMetadata =
                serde_json::from_str(&built.metadata_json).unwrap();
            meta.template_format = built.template.identity.artifact_version.clone();
            meta.template.content_hash = built.template.content_hash.header_value();
            built.metadata_json = serde_json::to_string(&meta).unwrap();
            built.metadata_hash =
                cem_ml::content_cache::ContentHash::from_blake3(built.metadata_json.as_bytes())
                    .header_value();
        }
        fs::write(output.join(format!("{name}.bin")), built.template.bytes).unwrap();
        fs::write(output.join(format!("{name}.json")), built.metadata_json).unwrap();
        fs::write(output.join(format!("{name}.hash")), built.metadata_hash).unwrap();
    }
}
