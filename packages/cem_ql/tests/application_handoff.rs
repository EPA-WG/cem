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
