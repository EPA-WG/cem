#![cfg(feature = "application-contract-build")]
use cem_ml::validation::json::graphql::{GraphqlContracts, ResponseKind};
use cem_ql::application_build::compile_graphql_contracts;

const SCHEMA: &str = include_str!("../fixtures/component-application/shopping-cart/schema.graphql");
const OPERATIONS: &str =
    include_str!("../fixtures/component-application/shopping-cart/operations.graphql");

#[test]
fn shopping_cart_contracts_validate_native_variables_and_responses() {
    let built = compile_graphql_contracts(SCHEMA, OPERATIONS).unwrap();
    let (bytes, pin) = built.pinned_bytes().unwrap();
    let runtime = GraphqlContracts::admit(&bytes, &pin).unwrap();
    let validate = |json: &[u8]| {
        runtime.validate_variables(
            "SetCartLineQuantity",
            json,
            "app://cart/variables.json",
            &Default::default(),
        )
    };
    assert!(validate(br#"{"cartId":"cart-1","lineId":"tea","quantity":2}"#).is_ok());
    for bad in [
        br#"{"cartId":"cart-1","lineId":"tea"}"#.as_slice(),
        br#"{"cartId":"cart-1","lineId":"tea","quantity":"2"}"#,
        br#"{"cartId":"cart-1","lineId":"tea","quantity":2147483648}"#,
        br#"{"cartId":"cart-1","lineId":"tea","quantity":2,"quantity":3}"#,
    ] {
        assert!(validate(bad).is_err());
    }
    for (name, json, kind) in [
        (
            "Cart",
            include_bytes!("../fixtures/component-application/shopping-cart/cart.json").as_slice(),
            ResponseKind::Data,
        ),
        (
            "SetCartLineQuantity",
            include_bytes!("../fixtures/component-application/shopping-cart/updated.json"),
            ResponseKind::Data,
        ),
        (
            "RemoveCartLine",
            include_bytes!("../fixtures/component-application/shopping-cart/removed.json"),
            ResponseKind::Data,
        ),
        (
            "Cart",
            include_bytes!("../fixtures/component-application/shopping-cart/empty.json"),
            ResponseKind::Data,
        ),
        (
            "Cart",
            include_bytes!("../fixtures/component-application/shopping-cart/partial-error.json"),
            ResponseKind::Partial,
        ),
        (
            "Cart",
            include_bytes!("../fixtures/component-application/shopping-cart/failed.json"),
            ResponseKind::Errors,
        ),
    ] {
        assert_eq!(
            runtime
                .validate_response(name, json, "app://cart/response.json", &Default::default())
                .unwrap(),
            kind
        );
    }
    assert!(compile_graphql_contracts(
        SCHEMA,
        &OPERATIONS.replace("quantity: $quantity", "quantity: $lineId")
    )
    .is_err());
    let control = cem_ml::operation_control::OperationControl::default();
    assert!(runtime
        .validate_variables("Other", b"{}", "app://cart/variables.json", &control)
        .is_err());
    assert!(runtime
        .validate_variables(
            "Cart",
            &vec![b' '; 1024 * 1024 + 1],
            "app://cart/variables.json",
            &control
        )
        .is_err());
    control.abort_signal().abort();
    assert!(runtime
        .validate_variables(
            "Cart",
            br#"{"id":"cart-1"}"#,
            "app://cart/variables.json",
            &control
        )
        .is_err());
    assert!(GraphqlContracts::admit(&bytes, "foreign").is_err());
    let mut changed = bytes;
    changed.push(0);
    assert!(GraphqlContracts::admit(&changed, &pin).is_err());
}

#[test]
fn validates_fixed_graphql_input_shapes_and_merged_response_keys() {
    let schema = "
        type Query { probe(input: Probe): Result! }
        input Probe { ids: [ID!]!, count: Int! = 1, mode: Mode, enabled: Boolean }
        enum Mode { SIMPLE FULL }
        type Result { amount: Float!, active: Boolean!, mode: Mode! }
    ";
    let operations = "
        query Probe($value: Probe!, $spare: Probe = {ids: []}) {
            merged: probe(input: $value) { amount }
            ...Extra
            other: probe(input: $spare) { ... on Result { mode } }
        }
        fragment Extra on Query { merged: probe(input: $value) { active } }
    ";
    let contracts = compile_graphql_contracts(schema, operations).unwrap();
    let vars = |bytes: &[u8]| {
        contracts.validate_variables("Probe", bytes, "app://probe/vars", &Default::default())
    };
    for good in [
        br#"{"value":{"ids":[1,"line"],"mode":null,"enabled":true}}"#.as_slice(),
        br#"{"value":{"ids":"singleton","count":-2147483648,"mode":"FULL"}}"#,
        br#"{"value":{"ids":[],"count":2.147483647e9},"spare":null}"#,
    ] {
        assert!(vars(good).is_ok(), "{:?}", vars(good));
    }
    for bad in [
        br#"{"value":{"ids":null}}"#.as_slice(),
        br#"{"value":{"ids":[null]}}"#,
        br#"{"value":{"ids":1.1}}"#,
        br#"{"value":{"ids":[],"mode":"OTHER"}}"#,
        br#"{"value":{"ids":[],"count":2147483647.000000001}}"#,
        br#"{"value":{"ids":[],"count":null}}"#,
        br#"{"value":{"ids":[],"unknown":0}}"#,
        br#"{"value":{"ids":[],"ids":[]}}"#,
        br#"{"value":{"ids":[],}}"#,
        br#"{"value":{"ids":[]}}true"#,
    ] {
        assert!(vars(bad).is_err(), "{bad:?}");
    }
    let response = |bytes: &[u8]| {
        contracts.validate_response("Probe", bytes, "app://probe/response", &Default::default())
    };
    assert_eq!(
        response(br#"{"data":{"merged":{"amount":1.2,"active":true},"other":{"mode":"FULL"}}}"#),
        Ok(ResponseKind::Data)
    );
    assert_eq!(
        response(br#"{"errors":[{"message":"Bad variables"}]}"#),
        Ok(ResponseKind::Errors)
    );
    for bad in [
        br#"{"data":{"merged":{"amount":1.2},"other":{"mode":"FULL"}}}"#.as_slice(),
        br#"{"data":{"merged":{"amount":1.2,"active":true},"other":{"mode":"OTHER"}}}"#,
        br#"{"data":{"probe":{"amount":1.2,"active":true},"other":{"mode":"FULL"}}}"#,
        br#"{"data":null,"errors":[{"message":"Failed","path":["merged","unknown"]}]}"#,
        br#"{"data":null,"errors":[{"message":"Failed","path":["other",0]}]}"#,
        br#"{"data":null,"errors":[{"message":"Failed","path":["merged"],"locations":[{"line":0,"column":1}]}]}"#,
        br#"{"data":null,"errors":[{"message":"Failed"}]}"#,
        br#"{"errors":[]}"#,
        br#"{"errors":[{"message":3}]}"#,
        br#"{"errors":[{"message":"x","extensions":[]}]}"#,
        br#"{"errors":[{"message":"x","message":"y"}]}"#,
    ] {
        assert!(response(bad).is_err(), "{bad:?}");
    }
    assert!(compile_graphql_contracts(
        schema,
        &operations.replace("amount }", "amount @skip(if: true) }")
    )
    .is_err());
    assert!(compile_graphql_contracts(
        "scalar Custom type Query { custom: Custom }",
        "query Q { custom }"
    )
    .is_err());
    assert!(compile_graphql_contracts(
        "interface I { id: ID } type Query { thing: I }",
        "query Q { thing { id } }"
    )
    .is_err());
    assert!(compile_graphql_contracts("type Query { id: ID }", "{ id }").is_err());
}

#[test]
fn records_cart_entry_and_component_dependencies_without_closing_provider_gaps() {
    use cem_ml::content_cache::ContentHash;
    use cem_ql::{
        application_build::{build_dependency_manifest, compile_application_entry},
        declaration_closure::{build_declaration_closure, DeclarationBuildSource},
        template_artifact::TemplateArtifactSourceMapMode::Prod,
    };
    let components =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../cem-components/src/components");
    let roots = [
        "cem-action",
        "cem-alert",
        "cem-card",
        "cem-text",
        "cem-text-field",
    ]
    .map(String::from);
    let closure = build_declaration_closure(&roots, Prod, |tag| {
        let path = format!("{tag}/{tag}.xhtml");
        Ok(DeclarationBuildSource {
            source: std::fs::read_to_string(components.join(&path)).map_err(|e| e.to_string())?,
            uri: format!("https://app.example/components/{path}"),
        })
    })
    .unwrap();
    let view = include_str!("../fixtures/component-application/shopping-cart/cart-view.cem");
    let uri = "https://app.example/cart-view.cem";
    let entry = compile_application_entry(view, uri, vec!["response".into()], Prod).unwrap();
    let contracts = compile_graphql_contracts(SCHEMA, OPERATIONS).unwrap();
    let build = |entry| build_dependency_manifest(&closure.requirements, Some(entry), &contracts);
    let manifest = build(&entry).unwrap();
    let again = build(&entry).unwrap();
    assert_eq!(again.bytes, manifest.bytes);
    assert_eq!(again.content_hash, manifest.content_hash);
    assert!(!manifest.static_inventory_complete);
    let recipe = manifest.selected_runtime_source.as_ref().unwrap();
    assert_eq!(again.selected_runtime_source.as_ref(), Some(recipe));
    assert!(recipe.contains("SelectedBuiltin::SequenceWhere"));
    assert!(recipe.contains("SelectedBuiltin::SequenceCount"));
    assert!(recipe.contains(&manifest.content_hash));
    assert_eq!(
        manifest.content_hash,
        ContentHash::from_blake3(&manifest.bytes).header_value()
    );
    let report: serde_json::Value = serde_json::from_slice(&manifest.bytes).unwrap();
    assert_eq!(
        report["requirementsHash"],
        closure.requirements.pinned_json().unwrap().1
    );
    assert_eq!(report["nativeLinkClosureVerified"], false);
    assert_eq!(report["entry"]["sourceUri"], uri);
    assert_eq!(
        report["entry"]["template"]["sourceHash"],
        ContentHash::from_blake3(view.as_bytes()).header_value()
    );
    assert_eq!(
        report["entry"]["template"]["contentHash"],
        entry.template().content_hash.header_value()
    );
    assert_eq!(
        report["entry"]["template"]["hostBindings"],
        serde_json::json!(["response"])
    );
    assert_eq!(report["entry"]["componentRoots"], serde_json::json!(roots));
    assert_eq!(
        report["entry"]["inventory"]["unresolved"],
        serde_json::json!([])
    );
    let registry = &report["selectedRegistryEntries"]["sequence-where"];
    assert_eq!(registry["module"], "cem:stdlib/sequence");
    assert_eq!(registry["arity"], 2);
    assert!(registry["executionUnits"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("local-lambdas")));
    assert!(report["sourcePaths"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "packages/cem_ml/src/operation_control.rs"
        )));
    assert!(
        report["requirements"]["declarations"]["cem-text-field"]["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["kind"] == "shared-behavior-closure")
    );
    assert_eq!(
        report["graphql"]["operations"]["RemoveCartLine"],
        "mutation"
    );
    assert_eq!(
        report["graphql"]["guardHash"],
        contracts.pinned_bytes().unwrap().1
    );
    let foreign = compile_application_entry("{cem-outside}", uri, vec![], Prod).unwrap();
    assert!(build(&foreign).is_err());
    let open = compile_application_entry(
        "{p | {$ | num:abs(-1)}}",
        uri,
        vec!["response".into()],
        Prod,
    )
    .unwrap();
    assert!(build(&open).unwrap().selected_runtime_source.is_none());
}

#[test]
fn leaves_unreviewed_native_dispatch_and_source_demands_explicit() {
    use cem_ql::{
        application_build::build_dependency_manifest,
        declaration_closure::{build_declaration_closure, DeclarationBuildSource},
        template_artifact::TemplateArtifactSourceMapMode::Prod,
    };
    let source = r#"<cem-element tag="cem-probe"><template id="cem-probe" type="text/cem-ml">
        {p | {$ | num:abs(-1)} {$ | data:read("https://app.example/input.json", str:lower("JSON"))}}
    </template></cem-element>"#;
    let build = |source: &str| {
        build_declaration_closure(&["cem-probe".into()], Prod, |_| {
            Ok(DeclarationBuildSource {
                source: source.into(),
                uri: "https://app.example/cem-probe.xhtml".into(),
            })
        })
        .unwrap()
    };
    let graphql = compile_graphql_contracts(SCHEMA, OPERATIONS).unwrap();
    let closed = build(&source.replace(
        r#"{$ | num:abs(-1)} {$ | data:read("https://app.example/input.json", str:lower("JSON"))}"#,
        "{$ | seq:count((1,2))}",
    ));
    let closed = build_dependency_manifest(&closed.requirements, None, &graphql).unwrap();
    assert!(closed.static_inventory_complete);
    let open = build_dependency_manifest(&build(source).requirements, None, &graphql).unwrap();
    assert!(!open.static_inventory_complete);
    let report: serde_json::Value = serde_json::from_slice(&open.bytes).unwrap();
    assert!(
        report["requirements"]["declarations"]["cem-probe"]["unresolved"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["kind"] == "dynamic-data-format")
    );
    assert!(report["declarations"]["cem-probe"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|mapping| mapping["demand"]["name"] == "abs" && mapping["gap"].is_string()));
}
