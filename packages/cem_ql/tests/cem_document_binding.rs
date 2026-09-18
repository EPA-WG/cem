//! CEM-LOADER-NATIVE: explicit native document injection into resource metadata.
use cem_ql::{
    eval::{AtomValue, Item, ItemStream},
    render::TemplateData,
};
use std::collections::BTreeMap;

fn record(name: &str, value: Item) -> Item {
    Item::Record(BTreeMap::from([(name.into(), vec![value])]))
}

#[test]
fn resource_binding_retains_a_document_instead_of_serializing_its_members() {
    let tree = cem_ml::import::import_data_bytes(
        b"{\"qty\":3}",
        "application/json",
        "cem",
        "memory:response",
    )
    .unwrap();
    let mut data = TemplateData::default().with_binding(
        "datadom",
        ItemStream::once(record(
            "slices",
            record("response", record("data", Item::Atomic(AtomValue::Null))),
        )),
    );
    assert!(data.bind_cem_document("missing", tree.clone()).is_err());
    data.bind_cem_document("response", tree).unwrap();
    let mut current = data.bindings["datadom"].items[0].clone();
    for name in ["slices", "response", "data"] {
        let Item::Record(mut fields) = current else {
            panic!("expected control metadata");
        };
        current = fields.remove(name).unwrap().remove(0);
    }
    assert_eq!(
        current.view().unwrap().representation_id(),
        "cem.ql.imported-cem-ast"
    );
    let children = current.view().unwrap().field("children").unwrap();
    drop(data);
    assert_eq!(
        children[0].view().unwrap().field("name").unwrap()[0].atom(),
        Some(AtomValue::String("object".into()))
    );
}

#[test]
fn http_demo_companion_consumes_json_and_xml_as_native_nodes() {
    use cem_ql::{
        api::{compile, evaluate, CompileContext, EvaluationContext},
        xpath::functions::CemtXPathFunctions,
    };
    use std::sync::Arc;
    let functions = CemtXPathFunctions::compile(
        include_str!("../../cem-elements/demo/http-data.cemt"),
        "memory:http-data.cemt",
    )
    .unwrap();
    for (bytes, mime) in [
        (
            br#"{"results":[{"name":"alpha","status":"ready"}]}"#.as_slice(),
            "application/json",
        ),
        (
            br#"<catalog><item name="alpha" status="ready"/></catalog>"#.as_slice(),
            "application/xml",
        ),
    ] {
        let mut context = EvaluationContext::default();
        functions
            .install(
                &mut context.native_functions,
                Arc::new(cem_ml::resolver::ResolverRegistry::new()),
                Arc::new(cem_ml::resolver::ResolverPolicy::new()),
            )
            .unwrap();
        context.policy_bindings.insert(
            "document".into(),
            ItemStream::once(cem_ql::eval::imported_cem_tree(
                cem_ml::import::import_data_bytes(bytes, mime, "cem", "memory:response").unwrap(),
            )),
        );
        let query = compile(
            r#"native:call("http.field", native:call("http.rows", document), "name")"#,
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        let result = evaluate(&query, &context);
        assert!(result.error.is_none(), "{result:?}");
        assert_eq!(
            result.items[0].atom(),
            Some(AtomValue::String("alpha".into()))
        );
    }
}
