#[path = "../fixtures/application-handoff/runtime.rs"]
mod fixture;

#[cfg(feature = "application-source-runtime")]
#[path = "application_handoff/receiver_checkpoint.rs"]
mod receiver_checkpoint;

#[cfg(feature = "application-source-runtime")]
#[path = "application_handoff/typed_producer.rs"]
mod typed_producer;

#[cfg(feature = "application-source-runtime")]
#[path = "application_handoff/nested_receivers.rs"]
mod nested_receivers;

#[cfg(feature = "application-source-runtime")]
#[path = "application_handoff/stylesheets.rs"]
mod stylesheets;

#[cfg(feature = "application-source-runtime")]
#[test]
fn browser_provider_demand_needs_the_original_bootstrap() {
    use cem_ql::application_source_endpoint::{ApplicationBootstrapOwner, ApplicationSourceEndpoint};
    use std::sync::Arc;
    let (deployment, input) = fixture::build_provider();
    let control = Default::default();
    let generation = cem_ml::runtime_admission::generation::NativeGeneration::claim(Arc::new(Default::default())).unwrap();
    let code = ApplicationBootstrapOwner::new(&generation, control, deployment.pin.clone(), fixture::Bootstrap {
        inputs: fixture::Inputs::restore(&input, &Default::default()).unwrap(),
    }).unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    assert!(endpoint.open(&deployment.bytes, &serde_json::to_string(&deployment.pin).unwrap()).is_err());
    assert_eq!(generation.inventory().unwrap().live_roots, 2);
}

#[cfg(feature = "application-source-runtime")]
#[test]
fn browser_provider_recovers_the_original_input_instance_and_referenced_source() {
    use cem_ml::{committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES, operation_control::OperationControl, runtime_admission::generation::NativeGeneration};
    use cem_ql::{application_source_endpoint::{
        ApplicationBootstrap, ApplicationBootstrapOwner, ApplicationBrowserBehavior, ApplicationSourceEndpoint, PreparedApplication,
    }, application_source_transport::VerifiedApplicationSources};
    use std::sync::Arc;

    struct ProviderBootstrap(fixture::Bootstrap);
    impl ApplicationBootstrap for ProviderBootstrap {
        fn prepare(&self, sources: Arc<VerifiedApplicationSources>, generation: &Arc<NativeGeneration>, control: OperationControl) -> Result<PreparedApplication, String> {
            self.0.prepare(sources, generation, control)?
                .with_browser_behavior("cem-receiver-frame", ApplicationBrowserBehavior::FormControl)
        }
    }
    let (deployment, original) = fixture::build_provider();
    let control = OperationControl::default();
    let provision = |inputs| {
        let generation = NativeGeneration::claim(Arc::new(Default::default())).unwrap();
        let code = ApplicationBootstrapOwner::new(&generation, control.clone(), deployment.pin.clone(),
            ProviderBootstrap(fixture::Bootstrap { inputs })).unwrap();
        let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
        let session = endpoint.open(&deployment.bytes, &serde_json::to_string(&deployment.pin).unwrap()).unwrap();
        (generation, code, endpoint, session)
    };
    let inputs = fixture::Inputs::restore(&original, &control).unwrap();
    let (_generation, _code, _endpoint, mut session) = provision(inputs.clone());
    let tag = "cem-receiver-frame";
    let foreign = fixture::Inputs::restore(&original, &control).unwrap();
    assert!(session.register_browser_receiver_input(tag, foreign.root(), "source-route").is_err());
    assert!(session.render_browser_island_json(tag, &[], "source-route").is_err());
    let input = session.register_browser_receiver_input(tag, inputs.root(), "source-route").unwrap();
    assert!(session.render_receiver_native(&input, "source-route").is_err());
    assert!(session.render_browser_receiver_json(&input, "other").is_err());
    assert!(session.capture_receiver_checkpoint(&input, &[&inputs.view, &foreign.cart],
        MAX_WORKSPACE_CHECKPOINT_BYTES, &control, |_| true).is_err());
    let render = |session: &mut cem_ql::application_source_endpoint::ApplicationSourceSession, input: &_| {
        let json = session.render_browser_receiver_json(input, "source-route").unwrap();
        let receipt: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(receipt["value"], "Tea");
        assert_eq!(receipt["behaviorIdentity"], "cem-elements-form-control-v4");
        assert!(receipt["html"].as_str().unwrap().contains("value=\"Tea\""));
    };
    render(&mut session, &input);
    let (bytes, seal) = session.capture_receiver_checkpoint(&input, &[&inputs.view, &inputs.cart],
        MAX_WORKSPACE_CHECKPOINT_BYTES, &control, |_| true).unwrap();
    let checkpoint = seal.verify(&bytes).unwrap();
    let fresh_inputs = receiver_checkpoint::restore_inputs(&checkpoint);
    let (_fresh_generation, _fresh_code, _fresh_endpoint, mut fresh) = provision(fresh_inputs);
    let restored = fresh.restore_receiver_checkpoint(checkpoint, MAX_WORKSPACE_CHECKPOINT_BYTES).unwrap();
    assert!(fresh.render_browser_receiver_json(&input, "source-route").is_err());
    assert!(fresh.render_browser_receiver_json(&restored, "other").is_err());
    render(&mut fresh, &restored);
}

#[cfg(feature = "application-source-runtime")]
#[test]
fn browser_navigation_demand_stays_inert_without_its_original_bootstrap_route() {
    use cem_ql::application_source_endpoint::{
        ApplicationBootstrapOwner, ApplicationSourceEndpoint,
    };
    use std::sync::Arc;

    let (deployment, input) = fixture::build_navigation();
    let control = Default::default();
    let sources =
        cem_ql::application_source_transport::verify(&deployment.bytes, &deployment.pin, &control)
            .unwrap();
    assert!(sources
        .application()
        .requirements()
        .require_static_inventory()
        .is_err());
    let generation = cem_ml::runtime_admission::generation::NativeGeneration::claim(Arc::new(
        Default::default(),
    ))
    .unwrap();
    let code = ApplicationBootstrapOwner::new(
        &generation,
        control.clone(),
        deployment.pin.clone(),
        fixture::Bootstrap {
            inputs: fixture::Inputs::restore(&input, &control).unwrap(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    assert!(endpoint
        .open(
            &deployment.bytes,
            &serde_json::to_string(&deployment.pin).unwrap()
        )
        .is_err());
    assert_eq!(generation.inventory().unwrap().live_roots, 2);
}

#[cfg(feature = "application-source-runtime")]
#[test]
fn browser_navigation_uses_original_island_and_closed_output() {
    use cem_ml::{
        operation_control::OperationControl, runtime_admission::generation::NativeGeneration,
    };
    use cem_ql::{
        application_source_endpoint::{
            ApplicationBootstrap, ApplicationBootstrapOwner, ApplicationBrowserBehavior,
            ApplicationSourceEndpoint, PreparedApplication,
        },
        application_source_transport::VerifiedApplicationSources,
    };
    use std::sync::Arc;

    struct BrowserBootstrap(fixture::Bootstrap);
    impl ApplicationBootstrap for BrowserBootstrap {
        fn prepare(
            &self,
            sources: Arc<VerifiedApplicationSources>,
            generation: &Arc<NativeGeneration>,
            control: OperationControl,
        ) -> Result<PreparedApplication, String> {
            self.0
                .prepare(sources, generation, control)?
                .with_browser_behavior("cem-count", ApplicationBrowserBehavior::NavigationLandmark)
        }
    }
    let control = OperationControl::default();
    let xml = include_str!("../fixtures/component-application/island-domains.xhtml")
        .replace("quantity", "count");
    let island = cem_ql::api::native_values::import_document(
        xml.as_bytes(),
        "application/xml",
        "app://runtime/island.xhtml",
        &Default::default(),
    )
    .unwrap();
    for button_type in ["button", "submit"] {
        let (deployment, input) = fixture::navigation_sources(button_type);
        let generation = NativeGeneration::claim(Arc::new(Default::default())).unwrap();
        let code = ApplicationBootstrapOwner::new(
            &generation,
            control.clone(),
            deployment.pin.clone(),
            BrowserBootstrap(fixture::Bootstrap {
                inputs: fixture::Inputs::restore(&input, &control).unwrap(),
            }),
        )
        .unwrap();
        let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
        let mut session = endpoint
            .open(
                &deployment.bytes,
                &serde_json::to_string(&deployment.pin).unwrap(),
            )
            .unwrap();
        let render =
            |session: &mut cem_ql::application_source_endpoint::ApplicationSourceSession| {
                session.render_browser_island_json("cem-count", &island, "island-one")
            };
        if button_type == "submit" {
            assert!(render(&mut session).is_err());
            continue;
        }
        assert!(session
            .render_island_native("cem-count", &island, "island-one")
            .is_err());
        assert!(session
            .project_island_native("cem-count", &island, "island-one")
            .is_err());
        assert!(session
            .render_browser_island_json("cem-app", &island, "island-one")
            .is_err());
        assert!(session
            .render_browser_island_json("cem-count", &island, "foreign")
            .is_err());
        let frame: serde_json::Value =
            serde_json::from_str(&render(&mut session).unwrap()).unwrap();
        assert_eq!(
            frame["behaviorIdentity"],
            "cem-elements-navigation-landmark-v1"
        );
        let html = frame["html"].as_str().unwrap();
        assert!(
            html.contains("id=\"island-one\"") && html.contains(">3</button>"),
            "{html}"
        );
        code.retire();
        assert!(render(&mut session).is_err());
    }
}

#[cfg(feature = "application-source-runtime")]
#[test]
fn native_receiver_frame_requires_original_input_and_available_preparation() {
    use cem_ql::{
        application_source_endpoint::{ApplicationBootstrapOwner, ApplicationSourceEndpoint},
        eval::RetainedCemNode,
    };
    use std::sync::Arc;
    let (source, data) = fixture::build();
    let control = Default::default();
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let generation = cem_ml::runtime_admission::generation::NativeGeneration::claim(Arc::new(
        Default::default(),
    ))
    .unwrap();
    let code = ApplicationBootstrapOwner::new(
        &generation,
        control,
        source.pin.clone(),
        fixture::Bootstrap {
            inputs: inputs.clone(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut session = endpoint
        .open(&source.bytes, &serde_json::to_string(&source.pin).unwrap())
        .unwrap();
    let element = inputs
        .view
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            cem_ml::parser::CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "view" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let root = RetainedCemNode::new(inputs.view.tree.clone(), element).unwrap();
    let input = session
        .register_receiver_input("cem-receiver-frame", root)
        .unwrap();
    // A registered source alone cannot invent a preparer or a typed-only grant.
    assert!(session
        .render_receiver_native(&input, "native-one")
        .is_err());
    let foreign = fixture::Inputs::restore(&data, &Default::default()).unwrap();
    assert!(session
        .register_receiver_input(
            "cem-receiver-frame",
            RetainedCemNode::new(foreign.view.tree.clone(), element).unwrap()
        )
        .is_err());
    code.retire();
    assert!(session
        .render_receiver_native(&input, "native-one")
        .is_err());
    let (source, data) = fixture::build_prepared();
    let control = Default::default();
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let code = ApplicationBootstrapOwner::new(
        &generation,
        control,
        source.pin.clone(),
        fixture::Bootstrap {
            inputs: inputs.clone(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut session = endpoint
        .open(&source.bytes, &serde_json::to_string(&source.pin).unwrap())
        .unwrap();
    let input = session
        .register_receiver_input(
            "cem-receiver-frame",
            RetainedCemNode::new(inputs.view.tree.clone(), element).unwrap(),
        )
        .unwrap();
    assert!(session
        .capture()
        .unwrap_err()
        .contains("receiver_checkpoint"));
    let html = session
        .render_receiver_native(&input, "native-one")
        .unwrap();
    assert_eq!(html, "<p>Tea/3/ok/native-one</p>");
    assert!(!session
        .project_receiver_native(&input, "native-one")
        .unwrap()
        .is_empty());
    assert_eq!(
        session
            .render_receiver_native(&input, "native-one")
            .unwrap(),
        html
    );
    let mut extra = vec![];
    for _ in 0..63 {
        extra.push(
            session
                .register_receiver_input("cem-receiver-frame", inputs.root())
                .unwrap(),
        );
    }
    assert!(session
        .register_receiver_input("cem-receiver-frame", inputs.root())
        .is_err());
    drop(extra);
    let mut sibling = endpoint
        .open(&source.bytes, &serde_json::to_string(&source.pin).unwrap())
        .unwrap();
    assert!(sibling
        .render_receiver_native(&input, "native-one")
        .is_err());
    session
        .replace(&source.bytes, &serde_json::to_string(&source.pin).unwrap())
        .unwrap();
    assert!(session
        .render_receiver_native(&input, "native-one")
        .is_err());
    let renewed = session
        .register_receiver_input("cem-receiver-frame", inputs.root())
        .unwrap();
    assert_eq!(
        session
            .render_receiver_native(&renewed, "native-one")
            .unwrap(),
        html
    );
    assert!(session.capture().is_err());
    drop(input);
    drop(renewed);
    assert_eq!(session.capture().unwrap(), source.bytes);
}

#[cfg(feature = "application-source-runtime")]
#[test]
fn receiver_frames_refuse_qualified_missing_and_unvalidated_original_inputs() {
    use cem_ql::application_source_endpoint::{
        ApplicationBootstrapOwner, ApplicationSourceEndpoint,
    };
    use std::sync::Arc;
    let required = fixture::RECEIVER.replace(
        "@optional-attributes=\"items count choice\"",
        "@required-attributes=count @optional-attributes=\"items choice\"",
    );
    let cases: [(String, String, String, bool, bool); 11] = [
        (
            "{other @items={#data} @count=3 @choice=ok}".into(),
            fixture::RECEIVER.into(),
            fixture::CART.into(),
            false,
            false,
        ),
        (
            "{view @items={#data} @choice=ok}".into(),
            required.clone(),
            fixture::CART.into(),
            false,
            false,
        ),
        (
            "{view @items={#data} @count=3 @choice=ok @unknown=yes}".into(),
            fixture::RECEIVER.into(),
            fixture::CART.into(),
            false,
            false,
        ),
        (
            "{view @items={#data} @count=3 @choice=ok | {child}}".into(),
            fixture::RECEIVER.into(),
            fixture::CART.into(),
            false,
            false,
        ),
        (
            "@ns x = urn:foreign\n{x:view @items={#data} @count=3 @choice=ok}".into(),
            fixture::RECEIVER.into(),
            fixture::CART.into(),
            false,
            false,
        ),
        (
            "@ns x = urn:foreign\n{view @items={#data} @x:count=3 @choice=ok}".into(),
            fixture::RECEIVER.into(),
            fixture::CART.into(),
            false,
            false,
        ),
        (
            fixture::VIEW.into(),
            fixture::RECEIVER.into(),
            "{cart @title=Tea | {item @quantity=bad}}".into(),
            true,
            false,
        ),
        (
            "{view @items={#data} @count=2 @choice=ok}".into(),
            fixture::RECEIVER.into(),
            fixture::CART.into(),
            true,
            false,
        ),
        (
            "{view @items={#data} @count=3 @choice={#data}}".into(),
            fixture::RECEIVER.into(),
            fixture::CART.into(),
            true,
            false,
        ),
        (
            format!(
                "{{view @items={{#data}} @count=3 @choice={}}}",
                "x".repeat(1025)
            ),
            fixture::RECEIVER.into(),
            fixture::CART.into(),
            true,
            false,
        ),
        (
            fixture::VIEW.into(),
            required,
            fixture::CART.into(),
            true,
            true,
        ),
    ];
    for (view, schema, cart, registration, accepted) in cases {
        let (source, data) = fixture::prepared_sources(fixture::VIEW, &schema, &cart);
        let control = Default::default();
        let restored = fixture::Inputs::restore(&data, &control).unwrap();
        let inputs = Arc::new(fixture::Inputs {
            view: fixture::imported(&view, "file:///runtime/case-view.cem"),
            cart: cem_ql::application_source_transport::SourceInput {
                identity: restored.cart.identity.clone(),
                tree: restored.cart.tree.clone(),
                capture: restored.cart.capture.clone(),
            },
        });
        let generation = cem_ml::runtime_admission::generation::NativeGeneration::claim(Arc::new(
            Default::default(),
        ))
        .unwrap();
        let code = ApplicationBootstrapOwner::new(
            &generation,
            control,
            source.pin.clone(),
            fixture::Bootstrap {
                inputs: inputs.clone(),
            },
        )
        .unwrap();
        let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
        let mut session = endpoint
            .open(&source.bytes, &serde_json::to_string(&source.pin).unwrap())
            .unwrap();
        let input = session.register_receiver_input("cem-receiver-frame", inputs.root());
        if registration {
            let input = input.unwrap();
            if accepted {
                assert_eq!(
                    session
                        .render_receiver_native(&input, "native-one")
                        .unwrap(),
                    "<p>Tea/3/ok/native-one</p>"
                );
            } else {
                assert!(
                    session
                        .render_receiver_native(&input, "native-one")
                        .is_err(),
                    "original input case / {cart}"
                );
                assert!(session
                    .project_receiver_native(&input, "native-one")
                    .is_err());
            }
        } else {
            assert!(input.is_err(), "original input registration")
        }
    }
}

#[cfg(feature = "application-source-runtime")]
#[test]
fn native_receiver_fields_cannot_shadow_runtime_roots() {
    use cem_ql::application_source_endpoint::{
        ApplicationBootstrapOwner, ApplicationSourceEndpoint,
    };
    use std::sync::Arc;
    let (source, data) = fixture::build_reserved();
    let control = Default::default();
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let generation = cem_ml::runtime_admission::generation::NativeGeneration::claim(Arc::new(
        Default::default(),
    ))
    .unwrap();
    let code = ApplicationBootstrapOwner::new(
        &generation,
        control,
        source.pin.clone(),
        fixture::Bootstrap {
            inputs: inputs.clone(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut session = endpoint
        .open(&source.bytes, &serde_json::to_string(&source.pin).unwrap())
        .unwrap();
    assert!(session
        .register_receiver_input("cem-receiver-frame", inputs.root())
        .is_err());
}

fn direct_inputs() -> (Vec<u8>, &'static str) {
    use cem_ql::{api::native_values, eval::portable};
    let input = native_values::import_document(
        b"<view count='003' data-fruit-name='banana'/>",
        "application/xml",
        "app://runtime/input.xml",
        &Default::default(),
    )
    .unwrap();
    let mut graph =
        cem_ml::value::artifact::CemValueGraph::decode(&input, &Default::default()).unwrap();
    graph.roots = ["count", "data-fruit-name"]
        .into_iter()
        .map(|name| {
            graph
                .records
                .iter()
                .position(|r| r.kind == "attribute" && r.name == name)
                .unwrap() as u32
        })
        .collect();
    let mut values = portable::decode_values(
        &graph.encode(&Default::default()).unwrap(),
        &Default::default(),
    )
    .unwrap();
    let json = native_values::import_document(
        br#"{"title":"Tea"}"#,
        "application/json",
        "app://runtime/cart.json",
        &Default::default(),
    )
    .unwrap();
    values.items.extend(
        portable::decode_values(&json, &Default::default())
            .unwrap()
            .items,
    );
    (
        portable::encode_values(&values, &Default::default()).unwrap(),
        r#"[{"name":"count","index":0,"target":"attribute"},{"name":"fruitName","index":1,"target":"dataset","attribute":"data-fruit-name"},{"name":"cart","index":2,"target":"slice"}]"#,
    )
}
#[cfg(feature = "application-source-runtime")]
const PROJECTION_ROUTE: &str = r#"[{"target":"slice","name":"fragment","index":0}]"#;

#[cfg(feature = "application-source-runtime")]
fn projection_consumer() -> cem_ql::declaration_artifact::BuiltDeclarationArtifact {
    cem_ql::declaration_artifact::build_declaration_artifact(
        r#"<cem-element tag="cem-projection-consumer"><template id="cem-projection-consumer" type="text/cem-ml">{section | {$datadom.slices.fragment}}</template></cem-element>"#,
        "app://runtime/projection-consumer.xhtml",
        cem_ql::template_artifact::TemplateArtifactSourceMapMode::Prod,
        None,
    )
    .unwrap()
}

#[cfg(all(feature = "component-runtime", feature = "application-source-runtime"))]
#[test]
fn native_optional_receiver_projection_is_admitted_by_a_compiled_consumer() {
    use cem_ql::{
        application_runtime::owners::CompiledDeclarationOwner,
        application_source_endpoint::{ApplicationBootstrapOwner, ApplicationSourceEndpoint},
    };
    let (deployment, input) = fixture::build_prepared();
    let control = Default::default();
    let inputs = fixture::Inputs::restore(&input, &control).unwrap();
    let generation = cem_ml::runtime_admission::generation::NativeGeneration::claim(
        std::sync::Arc::new(Default::default()),
    )
    .unwrap();
    let code = ApplicationBootstrapOwner::new(
        &generation,
        control,
        deployment.pin.clone(),
        fixture::Bootstrap {
            inputs: inputs.clone(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut source = endpoint
        .open(
            &deployment.bytes,
            &serde_json::to_string(&deployment.pin).unwrap(),
        )
        .unwrap();
    let input = source
        .register_receiver_input("cem-receiver-frame", inputs.root())
        .unwrap();
    let projection = source
        .project_receiver_native(&input, "native-one")
        .unwrap();
    let built = projection_consumer();
    let consumer = CompiledDeclarationOwner::admit(
        &built.template.bytes,
        &built.metadata_json,
        &built.metadata_hash,
    )
    .unwrap();
    assert_eq!(
        consumer
            .render_inputs_native(&projection, PROJECTION_ROUTE)
            .unwrap(),
        "<section><p>Tea/3/ok/native-one</p></section>"
    );
    assert!(consumer
        .render_inputs_native(&projection[..5], PROJECTION_ROUTE)
        .is_err());
}

#[test]
fn native_application_worker_fixture_has_original_sources_and_real_consumers() {
    use cem_ql::{
        application_source_endpoint::{ApplicationBootstrapOwner, ApplicationSourceEndpoint},
        application_source_transport,
    };
    use std::sync::Arc;
    let (bytes, input) = fixture::build_prepared();
    let control = Default::default();
    let inputs = fixture::Inputs::restore(&input, &control).unwrap();
    let g = cem_ml::runtime_admission::generation::NativeGeneration::claim(Arc::new(
        Default::default(),
    ))
    .unwrap();
    let code = ApplicationBootstrapOwner::new(
        &g,
        control,
        bytes.pin.clone(),
        fixture::Bootstrap {
            inputs: inputs.clone(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut session = endpoint
        .open(&bytes.bytes, &serde_json::to_string(&bytes.pin).unwrap())
        .unwrap();
    assert_eq!(
        fixture::validate(&mut session, &inputs).unwrap(),
        "true:true:true"
    );
    assert_eq!(session.capture().unwrap(), bytes.bytes);
    assert!(
        application_source_transport::verify(&bytes.bytes, &bytes.pin, &Default::default()).is_ok()
    );
    if let Some(dir) = std::env::var_os("CEM_APPLICATION_HANDOFF_FIXTURE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("installed.cemah"), bytes.bytes).unwrap();
        std::fs::write(dir.join("installed.inputs"), input).unwrap();
        let (typed, typed_input) = fixture::build_typed();
        std::fs::write(dir.join("typed.cemah"), typed.bytes).unwrap();
        std::fs::write(dir.join("typed.pin.json"), serde_json::to_vec(&typed.pin).unwrap()).unwrap();
        std::fs::write(dir.join("typed.inputs"), typed_input).unwrap();
        let (nested, nested_input) = fixture::nested_sources(fixture::NESTED_VIEW);
        let (_, bad_nested) = fixture::nested_sources(&fixture::NESTED_VIEW.replace("@count=4", "@count=2"));
        std::fs::write(dir.join("nested.cemah"), nested.bytes).unwrap();
        std::fs::write(dir.join("nested.pin.json"), serde_json::to_vec(&nested.pin).unwrap()).unwrap();
        std::fs::write(dir.join("nested.inputs"), nested_input).unwrap();
        std::fs::write(dir.join("nested.bad.inputs"), bad_nested).unwrap();
        let (styled, styled_input) = fixture::build_styled();
        std::fs::write(dir.join("styled.cemah"), styled.bytes).unwrap();
        std::fs::write(dir.join("styled.pin.json"), serde_json::to_vec(&styled.pin).unwrap()).unwrap();
        std::fs::write(dir.join("styled.inputs"), styled_input).unwrap();
        #[cfg(feature = "application-source-runtime")]
        {
            let (navigation, navigation_input) = fixture::build_navigation();
            std::fs::write(dir.join("navigation.cemah"), navigation.bytes).unwrap();
            std::fs::write(dir.join("navigation.pin.json"), serde_json::to_vec(&navigation.pin).unwrap()).unwrap();
            std::fs::write(dir.join("navigation.inputs"), navigation_input).unwrap();
            let (provider, provider_input) = fixture::build_provider();
            std::fs::write(dir.join("provider.cemah"), provider.bytes).unwrap();
            std::fs::write(dir.join("provider.pin.json"), serde_json::to_vec(&provider.pin).unwrap()).unwrap();
            std::fs::write(dir.join("provider.inputs"), provider_input).unwrap();
        }
        for (name, view, cart) in [
            ("edited", "{view @items={#data} @count=4 @choice=ok}", "{cart @title=Coffee | {item @quantity=4}}"),
            ("later", fixture::VIEW, "{cart @title=Cocoa | {item @quantity=2}}"),
            ("private", fixture::VIEW, "{cart @title=Tea @password=private | {item @quantity=3}}"),
        ] {
            let (_, input) = fixture::prepared_sources(view, fixture::RECEIVER, cart);
            std::fs::write(dir.join(format!("installed.{name}.inputs")), input).unwrap();
        }
        std::fs::write(
            dir.join("installed.pin.json"),
            serde_json::to_vec(&bytes.pin).unwrap(),
        )
        .unwrap();
        let xml = include_str!("../fixtures/component-application/island-domains.xhtml")
            .replace("quantity", "count");
        for (name, text) in [
            ("frame.island", xml.clone()),
            (
                "frame.bad-count",
                xml.replace(
                    "name=\"count\" value=\"003\"",
                    "name=\"count\" value=\"002\"",
                ),
            ),
            (
                "frame.bad-namespace",
                xml.replace("https://cem.dev/ns/runtime/instance-slices", "urn:wrong"),
            ),
        ] {
            let native = cem_ql::api::native_values::import_document(
                text.as_bytes(),
                "application/xml",
                "app://runtime/island.xhtml",
                &Default::default(),
            )
            .unwrap();
            std::fs::write(dir.join(name), native).unwrap();
        }
        #[cfg(feature = "application-source-runtime")]
        {
            let consumer = projection_consumer();
            std::fs::write(dir.join("projection-consumer.bin"), consumer.template.bytes).unwrap();
            std::fs::write(dir.join("projection-consumer.json"), consumer.metadata_json).unwrap();
            std::fs::write(dir.join("projection-consumer.hash"), consumer.metadata_hash).unwrap();
            std::fs::write(dir.join("projection-consumer.route.json"), PROJECTION_ROUTE).unwrap();
        }
        let (inputs, route) = direct_inputs();
        std::fs::write(dir.join("frame.inputs"), inputs).unwrap();
        std::fs::write(dir.join("frame.route.json"), route).unwrap();
    }
}

#[cfg(feature = "application-source-runtime")]
#[test]
fn original_scalar_schema_admits_qualified_islands_and_rejects_source_contract_mismatches() {
    use cem_ql::{
        api::native_values,
        application_source_endpoint::{ApplicationBootstrapOwner, ApplicationSourceEndpoint},
    };
    use std::sync::Arc;
    let (source, input) = fixture::build();
    let control = Default::default();
    let inputs = fixture::Inputs::restore(&input, &control).unwrap();
    let g = cem_ml::runtime_admission::generation::NativeGeneration::claim(Arc::new(
        Default::default(),
    ))
    .unwrap();
    let code = ApplicationBootstrapOwner::new(
        &g,
        control,
        source.pin.clone(),
        fixture::Bootstrap { inputs },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut session = endpoint
        .open(&source.bytes, &serde_json::to_string(&source.pin).unwrap())
        .unwrap();
    let xml = include_str!("../fixtures/component-application/island-domains.xhtml")
        .replace("quantity", "count");
    let bytes = |s: &str| {
        native_values::import_document(
            s.as_bytes(),
            "application/xml",
            "app://runtime/island.xhtml",
            &Default::default(),
        )
        .unwrap()
    };
    let island = bytes(&xml);
    let output = session
        .render_island_native("cem-count", &island, "island-one")
        .unwrap();
    assert!(
        output.contains("3/banana/ Tea &amp; cake /island-one"),
        "{output}"
    );
    let projected = session
        .project_island_native("cem-count", &island, "island-one")
        .unwrap();
    let graph =
        cem_ml::value::artifact::CemValueGraph::decode(&projected, &Default::default()).unwrap();
    assert!(graph.records.iter().any(|n| n.name == "strong"
        && n.namespace == "http://www.w3.org/1999/xhtml"
        && n.provenance.as_ref().and_then(|p| p.source_uri.as_deref())
            == Some("app://runtime/island.xhtml")));
    let original =
        cem_ml::value::artifact::CemValueGraph::decode(&island, &Default::default()).unwrap();
    let strong = original
        .records
        .iter()
        .find(|n| n.name == "strong")
        .unwrap();
    let retained = graph.records.iter().find(|n| n.name == "strong").unwrap();
    assert_eq!(retained.source, strong.source);
    let retained = retained.provenance.as_ref().unwrap();
    let original = strong.provenance.as_ref().unwrap();
    assert_eq!(retained.source_key, original.source_key);
    assert_eq!(retained.source_uri, original.source_uri);
    assert_eq!(retained.line_number, original.line_number);
    for invalid in [
        xml.replace(
            "name=\"count\" value=\"003\"",
            "name=\"count\" value=\"002\"",
        ),
        xml.replace("https://cem.dev/ns/runtime/instance-slices", "urn:wrong"),
        xml.replace("value=\"banana\"", "value=\"mismatch\"")
            .replacen("mismatch", "banana", 1),
    ] {
        assert!(session
            .render_island_native("cem-count", &bytes(&invalid), "island-one")
            .is_err());
    }
    assert!(session
        .render_island_native("cem-count", &island, "foreign-instance")
        .is_err());
    assert!(session
        .render_island_native("cem-value", &island, "island-one")
        .is_err());
    assert_eq!(
        session
            .render_island_native("cem-count", &island, "island-one")
            .unwrap(),
        output
    );
    let (inputs, route) = direct_inputs();
    let html = session
        .render_inputs_native("cem-count", &inputs, route, "direct-one")
        .unwrap();
    assert!(html.contains("3/banana/Tea/direct-one"), "{html}");
    let mut graph =
        cem_ml::value::artifact::CemValueGraph::decode(&inputs, &Default::default()).unwrap();
    for missing in ["provenance", "source", "namespace", "base"] {
        let mut invalid = graph.clone();
        let root = &mut invalid.records[invalid.roots[0] as usize];
        match missing {
            "provenance" => root.provenance = None,
            "source" => root.source.frames.clear(),
            "namespace" => root.namespace = "urn:wrong".into(),
            "base" => root.provenance.as_mut().unwrap().source_uri = Some("data:invalid".into()),
            _ => unreachable!(),
        }
        assert!(
            session
                .render_inputs_native(
                    "cem-count",
                    &invalid.encode(&Default::default()).unwrap(),
                    route,
                    "direct-one"
                )
                .is_err(),
            "{missing}"
        );
    }
    // Even a slice routed from an attribute retains its qualified source domain.
    graph.records[graph.roots[1] as usize].namespace = "urn:wrong".into();
    let routed_slice = route.replace("\"target\":\"dataset\"", "\"target\":\"slice\"");
    assert!(session
        .render_inputs_native(
            "cem-count",
            &graph.encode(&Default::default()).unwrap(),
            &routed_slice,
            "direct-one"
        )
        .is_err());
    for invalid in [
        route.replace("fruitName", "wrong"),
        route.replace("\"index\":0", "\"index\":99"),
        route.replace("cart", "count"),
    ] {
        assert!(session
            .render_inputs_native("cem-count", &inputs, &invalid, "direct-one")
            .is_err());
    }
    code.retire();
    assert!(session
        .render_island_native("cem-count", &island, "island-one")
        .is_err());
    assert!(session
        .project_island_native("cem-count", &island, "island-one")
        .is_err());
    assert!(session
        .render_inputs_native("cem-count", &inputs, route, "direct-one")
        .is_err());

    // This is a separate original native-authored application, not a mutation of
    // verified IR or a metadata claim granting a nested component owner.
    let (nested, input) =
        fixture::build_count("{attribute @name=count @type=cart-count}{cem-app}{p | {$count}}");
    let control = Default::default();
    let inputs = fixture::Inputs::restore(&input, &control).unwrap();
    let code = ApplicationBootstrapOwner::new(
        &g,
        control,
        nested.pin.clone(),
        fixture::Bootstrap { inputs },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut nested = endpoint
        .open(&nested.bytes, &serde_json::to_string(&nested.pin).unwrap())
        .unwrap();
    assert!(nested
        .render_island_native("cem-count", &island, "island-one")
        .unwrap_err()
        .contains("additional ownership"));
}
