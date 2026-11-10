use super::*;
use crate::render::CompileTemplateOptions;
use crate::template_artifact::{
    compile_template_artifact, compile_template_artifact_with_compiled_styles,
    TemplateArtifactSourceMapMode,
};

fn compiled(source: &str) -> (Vec<u8>, String) {
    let artifact = compile_template_artifact_with_compiled_styles(
        source,
        &CompileTemplateOptions {
            host_bindings: vec!["response".into()],
            ..Default::default()
        },
        TemplateArtifactSourceMapMode::Prod,
    )
    .unwrap();
    let loaded = artifact
        .reload(&crate::template_artifact::TemplateArtifactLoadContext {
            expected_source_hash: Some(artifact.identity.source_hash.clone()),
            host_bindings: vec!["response".into()],
            source_map_mode: TemplateArtifactSourceMapMode::Prod,
        })
        .unwrap();
    assert!(loaded.diagnostics.is_empty(), "{:?}", loaded.diagnostics);
    let pins = PinnedTemplateArtifactContext {
        content_hash: artifact.content_hash.header_value(),
        source_hash: artifact.identity.source_hash.header_value(),
        cem_ml_version: cem_ml::VERSION.into(),
        cem_ql_version: crate::VERSION.into(),
        host_bindings: vec!["response".into()],
        source_map_mode: TemplateArtifactSourceMapMode::Prod,
    };
    (artifact.bytes, serde_json::to_string(&pins).unwrap())
}

#[test]
fn pinned_compiled_json_render_and_input_refusals() {
    let (bytes, pins) = compiled("{p | {$response.children.children.children.children.value}}");
    let output = render(
        &bytes,
        &pins,
        br#"{"name":"Tea <img>"}"#,
        "app://cart/data.json",
        "response",
    )
    .unwrap();
    assert!(output.contains("Tea &lt;img&gt;"), "{output}");
    assert!(render(&bytes, &pins, b"{", "app://cart/data.json", "response").is_err());
    assert!(render(&bytes, &pins, b"{}", "", "response").is_err());
    assert!(render(&bytes, &pins, b"{}", "app://cart/data.json", "missing").is_err());
    let mut wrong: PinnedTemplateArtifactContext = serde_json::from_str(&pins).unwrap();
    wrong.content_hash.clear();
    assert!(render(
        &bytes,
        &serde_json::to_string(&wrong).unwrap(),
        b"{}",
        "app://cart/data.json",
        "response"
    )
    .is_err());
    assert!(render(
        &bytes,
        &pins,
        &vec![b' '; MAX_INPUT_BYTES + 1],
        "app://cart/data.json",
        "response"
    )
    .is_err());
    let legacy = compile_template_artifact(
        "{p | legacy}",
        &Default::default(),
        TemplateArtifactSourceMapMode::Prod,
    );
    assert!(render(
        &legacy.bytes,
        &pins,
        b"{}",
        "app://cart/data.json",
        "response"
    )
    .unwrap_err()
    .contains("compiled CSS"));
}

#[test]
fn missing_build_selection_denies_calls_without_general_fallback() {
    let (bytes, pins) = compiled(r#"{p | {$ | str:upper("secret")}}"#);
    assert!(render(&bytes, &pins, b"{}", "app://cart/data.json", "response").is_err());
}

#[cfg(feature = "application-contract-build")]
#[test]
fn selected_application_guards_original_json_before_rendering() {
    use super::selected::{SelectedApplication, SelectedApplicationAssets};
    use crate::{
        application_build::{compile_application_entry, compile_graphql_contracts},
        selected_registry::{SelectedBuiltin, SelectedFunctionRegistry},
    };
    let source = r#"
        {cem:variable @name=envelope @select="seq:first(response.children)"}
        {cem:variable @name=data @select='seq:first(seq:where(envelope.children, fn(field) => field.attributes.value == "data").children)'}
        {cem:variable @name=item @select='seq:first(seq:where(data.children, fn(field) => field.attributes.value == "item").children)'}
        {p | {$ | seq:first(seq:where(item.children, fn(field) => field.attributes.value == "label").children).children.value}}
    "#;
    let entry = compile_application_entry(
        source,
        "app://selected/view.cem",
        vec!["response".into()],
        TemplateArtifactSourceMapMode::Prod,
    )
    .unwrap();
    let document = "query Read($id: ID!) { item(id: $id) { label } }";
    let (guard, guard_hash) = compile_graphql_contracts(
        "type Query { item(id: ID!): Item! } type Item { label: String! }",
        document,
    )
    .unwrap()
    .pinned_bytes()
    .unwrap();
    let pins = PinnedTemplateArtifactContext {
        content_hash: entry.template().content_hash.header_value(),
        source_hash: entry.template().identity.source_hash.header_value(),
        cem_ml_version: cem_ml::VERSION.into(),
        cem_ql_version: crate::VERSION.into(),
        host_bindings: vec!["response".into()],
        source_map_mode: TemplateArtifactSourceMapMode::Prod,
    };
    let pins_json = serde_json::to_string(&pins).unwrap();
    let assets = SelectedApplicationAssets {
        entry: &entry.template().bytes,
        entry_pins: &pins_json,
        guard: &guard,
        guard_hash: &guard_hash,
        operations: document,
    };
    let open = |assets| {
        SelectedApplication::admit(
            assets,
            SelectedFunctionRegistry::new([
                SelectedBuiltin::SequenceFirst,
                SelectedBuiltin::SequenceWhere,
            ])
            .unwrap(),
        )
    };
    let app = open(assets).unwrap();
    assert_eq!(app.operations(), document);
    app.validate_variables("Read", br#"{"id":"tea"}"#, "app://selected/request")
        .unwrap();
    assert!(app
        .validate_variables("Read", b"{}", "app://selected/request")
        .is_err());
    let uri = "app://selected/response";
    let good = br#"{"data":{"item":{"label":"Tea <img>"}}}"#;
    let tree = cem_ml::validation::json::graphql::GraphqlContracts::admit(&guard, &guard_hash)
        .unwrap()
        .admit_response("Read", good, uri, &Default::default())
        .unwrap()
        .into_retained_tree()
        .unwrap();
    let original = cem_ml::import::import_json_bytes(good, uri).unwrap();
    assert_eq!(tree.source_uri(), uri);
    assert!(matches!(
        tree.native_owner()
            .unwrap()
            .downcast_ref::<cem_ml::lifecycle::LoadedInputAstStream>(),
        Some(cem_ml::lifecycle::LoadedInputAstStream::JsonDocument(_))
    ));
    assert_eq!(format!("{:?}", tree.ast()), format!("{:?}", original.ast()));
    for id in 1..tree.ast().nodes.len() as u32 {
        assert_eq!(tree.source_key(id), original.source_key(id));
        let coordinates = |range: cem_ml::parser::tree::CemTreeRange| {
            (range.offset, range.length, range.line, range.column)
        };
        assert_eq!(
            tree.source_node_range(id).map(coordinates),
            original.source_node_range(id).map(coordinates)
        );
        assert_eq!(tree.source_node_map(id), original.source_node_map(id));
    }
    assert!(app
        .render_response("Read", good, uri)
        .unwrap()
        .contains("Tea &lt;img&gt;"));
    assert!(app
        .render_response("Read", br#"{"data":{"item":{"label":0}}}"#, uri)
        .is_err());
    assert!(app.render_response("Other", good, uri).is_err());
    assert!(app.render_response("Read", good, "relative").is_err());
    assert!(app.render_response("Read", good, uri).is_ok());
    assert!(open(SelectedApplicationAssets {
        operations: "query Foreign { item }",
        ..assets
    })
    .is_err());
    assert!(open(SelectedApplicationAssets {
        guard_hash: "foreign",
        ..assets
    })
    .is_err());
    assert!(open(SelectedApplicationAssets {
        entry: b"CEMTPLA2\nforeign",
        ..assets
    })
    .is_err());
    let denied =
        SelectedApplication::admit(assets, SelectedFunctionRegistry::new([]).unwrap()).unwrap();
    assert!(denied.render_response("Read", good, uri).is_err());
}
