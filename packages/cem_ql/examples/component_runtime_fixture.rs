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
    owned_inputs(&output);
    owned_receiver(&output);
    owned_island(&output);
    owned_domains(&output);
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

fn owned_inputs(output: &std::path::Path) {
    use cem_ql::{
        declaration_artifact::build_declaration_artifact,
        eval::{output::output_attribute, portable::encode_values, AtomValue, Item},
        render::RenderPlanAttribute,
    };
    let source = r#"<cem-element tag="cem-owned-inputs"><template id="cem-owned-inputs" type="text/cem-ml">{attribute @name=label}{p @title='{$label}' | {$datadom.attributes.label} / {$datadom.slices.amount}}</template></cem-element>"#;
    let built = build_declaration_artifact(
        source,
        "https://app.test/components/inputs.xhtml",
        TemplateArtifactSourceMapMode::Prod,
        None,
    )
    .unwrap();
    let metadata: cem_ql::declaration_artifact::DeclarationArtifactMetadata =
        serde_json::from_str(&built.metadata_json).unwrap();
    let loaded = built.template.reload_pinned(&metadata.template).unwrap();
    fs::write(output.join("owned-inputs.bin"), built.template.bytes).unwrap();
    fs::write(output.join("owned-inputs.json"), built.metadata_json).unwrap();
    fs::write(output.join("owned-inputs.hash"), built.metadata_hash).unwrap();
    for (name, label, amount) in [("first", "Tea <img>", 2), ("second", "Coffee & cake", 3)] {
        let attribute = output_attribute(RenderPlanAttribute {
            name: "label".into(),
            namespace: None,
            qualified_name: None,
            value: "compatibility projection must not be used".into(),
            value_stream: ItemStream::once(Item::Atomic(AtomValue::String(label.into()))),
            contract: None,
            source_map: Default::default(),
        });
        let amount = Item::Atomic(AtomValue::Integer(amount));
        let values = ItemStream::from_items(vec![attribute.clone(), amount.clone()]);
        fs::write(
            output.join(format!("owned-input-{name}.cemv")),
            encode_values(&values, &Default::default()).unwrap(),
        )
        .unwrap();
        let mut data = TemplateData::default();
        data.bind_native_attribute(attribute).unwrap();
        data.bind_native_slice("amount", ItemStream::once(amount))
            .unwrap();
        let plan = render_compiled_template(&loaded, &data);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        fs::write(
            output.join(format!("owned-input-{name}.html")),
            render_plan_to_html(&plan),
        )
        .unwrap();
    }
}

fn owned_receiver(output: &std::path::Path) {
    use cem_ql::{
        declaration_artifact::build_declaration_artifact,
        eval::{output::output_attribute, portable::encode_values, AtomValue, Item},
        render::RenderPlanAttribute,
    };
    let source = r#"<cem-element tag="cem-owned-receiver"><template id="cem-owned-receiver" type="text/cem-ml">{attribute @name=quantity @type=integer @required=true @minInclusive=1 @maxInclusive=5}{p @id="{$instanceID}" | {$quantity}/{$datadom.attributes.quantity}/{$instanceID}}</template></cem-element>"#;
    let built = build_declaration_artifact(
        source,
        "https://app.test/components/receiver.xhtml",
        TemplateArtifactSourceMapMode::Prod,
        None,
    )
    .unwrap();
    let metadata: cem_ql::declaration_artifact::DeclarationArtifactMetadata =
        serde_json::from_str(&built.metadata_json).unwrap();
    let loaded = built.template.reload_pinned(&metadata.template).unwrap();
    fs::write(output.join("owned-receiver.bin"), built.template.bytes).unwrap();
    fs::write(output.join("owned-receiver.json"), built.metadata_json).unwrap();
    fs::write(output.join("owned-receiver.hash"), built.metadata_hash).unwrap();
    for (name, value) in [
        ("first", "003"),
        ("second", "004"),
        ("low", "0"),
        ("high", "6"),
        ("invalid", "private credential"),
    ] {
        let quantity = output_attribute(RenderPlanAttribute {
            name: "quantity".into(),
            namespace: None,
            qualified_name: None,
            value: "wrong compatibility projection".into(),
            value_stream: ItemStream::once(Item::Atomic(AtomValue::String(value.into()))),
            contract: None,
            source_map: Default::default(),
        });
        fs::write(
            output.join(format!("owned-receiver-{name}.cemv")),
            encode_values(&ItemStream::once(quantity.clone()), &Default::default()).unwrap(),
        )
        .unwrap();
        let mut data = TemplateData::default().with_binding(
            "instanceID",
            ItemStream::once(Item::Atomic(AtomValue::String(format!("receiver-{name}")))),
        );
        data.bind_native_attribute(quantity).unwrap();
        let plan = render_compiled_template(&loaded, &data);
        if matches!(name, "first" | "second") {
            assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
            fs::write(
                output.join(format!("owned-receiver-{name}.html")),
                render_plan_to_html(&plan),
            )
            .unwrap();
        } else {
            assert!(plan
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation()));
        }
    }
    fs::write(
        output.join("owned-receiver-missing.cemv"),
        encode_values(&ItemStream::empty(), &Default::default()).unwrap(),
    )
    .unwrap();
}

fn owned_island(output: &std::path::Path) {
    use cem_ql::{
        declaration_artifact::build_declaration_artifact,
        eval::{
            output::output_nodes,
            portable::{decode_values, encode_values},
            AtomValue, Item,
        },
    };
    use std::collections::BTreeMap;
    let body="{cem:if @test='$datadom.slots.heading' | {article @id='{$instanceID}' | {$datadom.slots.heading}}}";
    let source = format!(
        r#"<cem-element tag="cem-owned-island"><template id="cem-owned-island" type="text/cem-ml">{body}</template></cem-element>"#
    );
    let built = build_declaration_artifact(
        &source,
        "https://app.test/components/island.xhtml",
        TemplateArtifactSourceMapMode::Prod,
        None,
    )
    .unwrap();
    let metadata: cem_ql::declaration_artifact::DeclarationArtifactMetadata =
        serde_json::from_str(&built.metadata_json).unwrap();
    let loaded = built.template.reload_pinned(&metadata.template).unwrap();
    fs::write(output.join("owned-island.bin"), built.template.bytes).unwrap();
    fs::write(output.join("owned-island.json"), built.metadata_json).unwrap();
    fs::write(output.join("owned-island.hash"), built.metadata_hash).unwrap();
    let original = include_str!("../fixtures/component-application/passive-island.xhtml");
    for (name,source) in [
        ("first",original.to_owned()),
        ("second",original.replace("island-one","island-two").replace("Heading","Updated")),
        ("empty",original.replace("<html:strong slot=\"heading\">Heading</html:strong><html:span slot=\"heading\">Second</html:span>Tail","")),
        ("bad-version",original.replace("version=\"0.2.0\"","version=\"0.1.2\"")),
        ("bad-namespace",original.replace("runtime/instance-slices","runtime/foreign")),
        ("bad-state",original.replace("<r:resources/>","<r:resources><r:resource name=\"unimplemented\"/></r:resources>")),
        ("credential",original.replace("Tail</p:payload>","<html:input autocomplete=\"one-time-code\"/></p:payload>")),
    ] {
        let tree=cem_ml::import::import_data_bytes(source.as_bytes(),"application/xml","cem",&format!("app://island/{name}.xhtml")).unwrap();
        let input=encode_values(&ItemStream::once(imported_cem_tree(tree)),&Default::default()).unwrap();
        fs::write(output.join(format!("owned-island-{name}.cemv")),&input).unwrap();
        if !matches!(name,"first"|"second"|"empty"){continue;}
        // Independent full-renderer reference, using the native values directly.
        let values=decode_values(&input,&Default::default()).unwrap();
        let children=|item:&Item|item.view().unwrap().field("children").unwrap_or_default();
        let field=|item:&Item,key:&str|item.view().and_then(|view|view.field(key)).and_then(|items|items.first().and_then(Item::atom));
        let root=children(&values.items[0]).into_iter().find(|item|field(item,"kind")==Some(AtomValue::String("element".into()))).unwrap();
        let payload=children(&root).into_iter().find(|item|field(item,"name")==Some(AtomValue::String("payload".into()))).unwrap();
        let payload_children=children(&payload);
        let heading=payload_children.iter().filter(|item|field(item,"kind")==Some(AtomValue::String("element".into()))).cloned().collect();
        let id=if name=="second"{"island-two"}else{"island-one"};
        let data=TemplateData::default().with_binding("island",ItemStream::once(root)).with_binding("instanceID",ItemStream::once(Item::Atomic(AtomValue::String(id.into())))).with_binding("datadom",ItemStream::once(Item::Record(BTreeMap::from([
            ("payload".into(),vec![payload]),
            ("slots".into(),vec![Item::Record(BTreeMap::from([("heading".into(),heading)]))]),
        ]))));
        let plan=render_compiled_template(&loaded,&data);
        assert!(plan.diagnostics.is_empty(),"{name}: {:?}",plan.diagnostics);
        fs::write(output.join(format!("owned-island-{name}.html")),render_plan_to_html(&plan)).unwrap();
        fs::write(output.join(format!("owned-island-{name}.projection.cemv")),encode_values(&output_nodes(plan.nodes),&Default::default()).unwrap()).unwrap();
    }
}

// Reference HTML comes from the full native renderer with independently supplied
// typed control data. Domain provenance/export contracts are asserted natively;
// browser/worker callers transport opaque CEMV and never reconstruct this frame.
fn owned_domains(output: &std::path::Path) {
    use cem_ql::{
        declaration_artifact::build_declaration_artifact,
        eval::{portable::encode_values, AtomValue, Item},
    };
    use std::collections::BTreeMap;
    let body="{attribute @name=quantity @type=integer @required=true @minInclusive=1 @maxInclusive=5}{attribute @name=data-count @type=integer}{p @id='{$instanceID}' @title='{$datadom.slices.cart.title}' | {$quantity}/{$datadom.attributes.quantity}/{$datadom.attributes.data-count}/{$datadom.dataset.count}/{$datadom.dataset.fruitName}/{$datadom.slices.cart.title}/{$datadom.slices.cart.cost}/{$datadom.slices.cart.ready}}";
    let source = format!(
        r#"<cem-element tag="cem-owned-domains"><template id="cem-owned-domains" type="text/cem-ml">{body}</template></cem-element>"#
    );
    let built = build_declaration_artifact(
        &source,
        "https://app.test/components/domains.xhtml",
        TemplateArtifactSourceMapMode::Prod,
        None,
    )
    .unwrap();
    let metadata: cem_ql::declaration_artifact::DeclarationArtifactMetadata =
        serde_json::from_str(&built.metadata_json).unwrap();
    let loaded = built.template.reload_pinned(&metadata.template).unwrap();
    fs::write(output.join("owned-domains.bin"), built.template.bytes).unwrap();
    fs::write(output.join("owned-domains.json"), built.metadata_json).unwrap();
    fs::write(output.join("owned-domains.hash"), built.metadata_hash).unwrap();
    let original = include_str!("../fixtures/component-application/island-domains.xhtml");
    for (name, source) in [
        ("first", original.to_owned()),
        (
            "second",
            original
                .replace("island-one", "island-two")
                .replace("003", "004")
                .replace("banana", "pear")
                .replace("Tea &amp; cake", "Coffee &amp; cake"),
        ),
        (
            "dataset",
            original.replace(
                "key=\"fruitName\" value=\"banana\"",
                "key=\"fruitName\" value=\"wrong\"",
            ),
        ),
        (
            "boolean",
            original.replace("kind=\"boolean\">true", "kind=\"boolean\">yes"),
        ),
        (
            "alias",
            original.replace("name=\"cart\"", "name=\"quantity\""),
        ),
        (
            "credential",
            original.replace(
                "name=\"quantity\" value=\"003\"",
                "name=\"autocomplete\" value=\"one-time-code\"",
            ),
        ),
        ("bounds", original.replace("003", "009")),
    ] {
        let tree = cem_ml::import::import_data_bytes(
            source.as_bytes(),
            "application/xml",
            "cem",
            &format!("app://island/domains-{name}.xhtml"),
        )
        .unwrap();
        fs::write(
            output.join(format!("owned-domains-{name}.cemv")),
            encode_values(
                &ItemStream::once(imported_cem_tree(tree)),
                &Default::default(),
            )
            .unwrap(),
        )
        .unwrap();
        if !matches!(name, "first" | "second") {
            continue;
        }
        let second = name == "second";
        let atom = |s: &str| Item::Atomic(AtomValue::String(s.into()));
        let lexical = if second { "004" } else { "003" };
        let id = if second { "island-two" } else { "island-one" };
        let fields = BTreeMap::from([
            (
                "attributes".into(),
                vec![Item::Record(BTreeMap::from([
                    ("quantity".into(), vec![atom(lexical)]),
                    ("data-count".into(), vec![atom(lexical)]),
                ]))],
            ),
            (
                "dataset".into(),
                vec![Item::Record(BTreeMap::from([
                    ("count".into(), vec![atom(lexical)]),
                    (
                        "fruitName".into(),
                        vec![atom(if second { "pear" } else { "banana" })],
                    ),
                ]))],
            ),
            (
                "slices".into(),
                vec![Item::Record(BTreeMap::from([(
                    "cart".into(),
                    vec![Item::Record(BTreeMap::from([
                        (
                            "title".into(),
                            vec![atom(if second {
                                " Coffee & cake "
                            } else {
                                " Tea & cake "
                            })],
                        ),
                        (
                            "cost".into(),
                            vec![Item::Atomic(AtomValue::Decimal(
                                "9007199254740993.25".into(),
                            ))],
                        ),
                        ("ready".into(), vec![Item::Atomic(AtomValue::Boolean(true))]),
                    ]))],
                )]))],
            ),
        ]);
        let data = TemplateData::default()
            .with_binding("datadom", ItemStream::once(Item::Record(fields)))
            .with_binding("quantity", ItemStream::once(atom(lexical)))
            .with_binding("data-count", ItemStream::once(atom(lexical)))
            .with_binding("instanceID", ItemStream::once(atom(id)));
        let plan = render_compiled_template(&loaded, &data);
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
        fs::write(
            output.join(format!("owned-domains-{name}.html")),
            render_plan_to_html(&plan),
        )
        .unwrap();
    }
}
