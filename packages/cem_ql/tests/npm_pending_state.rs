//! Authored HTTP pending-state guards over metadata and retained CEM documents.
use cem_ql::{
    eval::{AtomValue, Item, ItemStream},
    render::{render_template, TemplateData},
    xpath::functions::CemtXPathFunctions,
};
use std::sync::Arc;

const PAGE: &str = include_str!("../../cem-elements/demo/npm-versions-demo.html");
const LIBRARY: &str = include_str!("../../cem-elements/demo/http-data.cemt");
const RESPONSE: &[u8] = include_bytes!("../../cem-elements/demo/npm-versions.json");

fn picker() -> &'static str {
    // Select the declared CEMT fixture source, not a response-data projection.
    PAGE.split_once("<template id=\"npm-version\"")
        .unwrap()
        .1
        .split_once('>')
        .unwrap()
        .1
        .split_once("</template>")
        .unwrap()
        .0
}

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn record(fields: impl IntoIterator<Item = (&'static str, Item)>) -> Item {
    Item::Record(
        fields
            .into_iter()
            .map(|(name, value)| (name.into(), vec![value]))
            .collect(),
    )
}

fn data(state: Option<&str>) -> TemplateData {
    let slices = match state {
        None => record([]),
        Some(state) => record([(
            "registry",
            record([
                ("kind", string("http-request")),
                ("state", string(state)),
                ("data", Item::Atomic(AtomValue::Null)),
            ]),
        )]),
    };
    TemplateData::default()
        .with_binding("package", ItemStream::once(string("@epa-wg/cem-elements")))
        .with_binding("initialversion", ItemStream::once(string("")))
        .with_binding("showdate", ItemStream::once(string("")))
        .with_binding(
            "datadom",
            ItemStream::once(record([("slices", slices), ("eventPayloads", record([]))])),
        )
}

fn install_functions(data: &mut TemplateData) {
    CemtXPathFunctions::compile(LIBRARY, "memory:http-data.cemt")
        .unwrap()
        .install(
            &mut data.native_functions,
            Arc::new(cem_ml::resolver::ResolverRegistry::new()),
            Arc::new(cem_ml::resolver::ResolverPolicy::new()),
        )
        .unwrap();
}

#[test]
fn strict_missing_state_comparison_remains_a_diagnostic() {
    let result = render_template(
        r#"{if @test='datadom.slices.registry.state == "loaded"' | ready}"#,
        &data(None),
    );
    assert!(result.rendered.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.type_error"),
        "{:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.render.test_failed"),
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn authored_picker_is_quiet_before_a_document_is_loaded() {
    for state in [None, Some("scheduled"), Some("in-progress"), Some("failed")] {
        // No function library: a pending branch must not try to query data.
        let result = render_template(picker(), &data(state));
        assert!(
            result.diagnostics.is_empty(),
            "{state:?}: {:?}",
            result.diagnostics
        );
        assert!(
            !result.rendered.contains("<select"),
            "{state:?}: {}",
            result.rendered
        );
        assert!(result.rendered.contains("<http-request"));
        assert!(result.rendered.contains("@epa-wg/cem-elements</code>"));
    }
}

#[test]
fn authored_picker_queries_retained_cem_data_and_preserves_selection() {
    let tree = cem_ml::import::import_data_bytes(
        RESPONSE,
        "application/json",
        "cem",
        "memory:npm-versions.json",
    )
    .unwrap();
    let weak = Arc::downgrade(&tree);
    let mut data = data(Some("loaded"));
    install_functions(&mut data);
    data.bind_cem_document("registry", tree.clone()).unwrap();
    drop(tree);
    for (selected, dates) in [("", ""), ("0.0.22", "true")] {
        data.bindings
            .insert("initialversion".into(), ItemStream::once(string(selected)));
        data.bindings
            .insert("showdate".into(), ItemStream::once(string(dates)));
        let result = render_template(picker(), &data);
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let html = &result.rendered;
        assert_eq!(html.matches("<option ").count(), 4, "{html}");
        let versions = ["0.1.0", "0.0.25", "0.0.22", "0.0.21"];
        let positions: Vec<_> = versions
            .iter()
            .map(|v| html.find(&format!("<option value=\"{v}\"")).unwrap())
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(html.contains("2024-04-20"), dates == "true");
        assert_eq!(
            html.contains("selected=\"true\""),
            !selected.is_empty(),
            "{html}"
        );
        if !selected.is_empty() {
            assert!(
                html.contains("<option value=\"0.0.22\" selected=\"true\""),
                "{html}"
            );
        }
        assert!(weak.upgrade().is_some());
    }
    drop(data);
    assert!(weak.upgrade().is_none());
}

#[test]
fn loaded_state_does_not_hide_an_absent_native_document() {
    let mut data = data(Some("loaded"));
    install_functions(&mut data);
    let result = render_template(picker(), &data);
    assert!(!result.diagnostics.is_empty());
    assert!(!result.rendered.contains("<option "));
}
