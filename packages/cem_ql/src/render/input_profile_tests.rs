//! Owned renderer input regressions and a test-only synthesis/copy baseline.
use super::*;
use crate::compile_profile::measure;
use std::cell::Cell;

thread_local! {
    static FORCE_INPUT_COPY: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn force_input_copy() -> bool {
    FORCE_INPUT_COPY.with(Cell::get)
}

fn with_input_copy<T>(enabled: bool, run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            FORCE_INPUT_COPY.with(|value| value.set(self.0));
        }
    }
    let _reset = Reset(FORCE_INPUT_COPY.with(|value| value.replace(enabled)));
    run()
}

pub(super) fn with_copied_input<T>(run: impl FnOnce() -> T) -> T {
    with_input_copy(true, run)
}

pub(super) fn copied_data_document(bindings: &BTreeMap<String, ItemStream>) -> ItemStream {
    let synthesized = build_data_document(bindings);
    let Some(explicit) = bindings.get(DATA_DOCUMENT_BINDING) else {
        return synthesized;
    };
    let explicit = {
        let _profile = crate::compile_profile::Span::new("copy/explicit-data-document");
        explicit.clone()
    };
    merge_data_documents(explicit, synthesized)
}

fn build_data_document(bindings: &BTreeMap<String, ItemStream>) -> ItemStream {
    let _profile = crate::compile_profile::Span::new("copy/data-document-synthesis");
    let attributes: BTreeMap<String, Vec<Item>> = bindings
        .iter()
        .filter(|(name, _)| name.as_str() != DATA_DOCUMENT_BINDING)
        .map(|(name, stream)| (name.clone(), stream.items.clone()))
        .collect();
    let mut datadom = BTreeMap::new();
    for (name, stream) in bindings
        .iter()
        .filter(|(name, _)| name.as_str() != DATA_DOCUMENT_BINDING)
    {
        datadom.insert(name.clone(), stream.items.clone());
    }
    datadom.insert("attributes".to_owned(), vec![Item::Record(attributes)]);
    ItemStream::once(Item::Record(datadom))
}

fn merge_data_documents(mut explicit: ItemStream, synthesized: ItemStream) -> ItemStream {
    let _profile = crate::compile_profile::Span::new("copy/data-document-merge");
    let Some(Item::Record(synthesized_fields)) = synthesized.items.first() else {
        return explicit;
    };
    for item in &mut explicit.items {
        let Item::Record(explicit_fields) = item else {
            continue;
        };
        for (name, values) in synthesized_fields {
            explicit_fields
                .entry(name.clone())
                .or_insert_with(|| values.clone());
        }
    }
    explicit
}

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn record(fields: impl IntoIterator<Item = (&'static str, Vec<Item>)>) -> Item {
    Item::Record(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

#[test]
fn default_input_moves_explicit_allocation_and_skips_transient_copies() {
    let mut bindings = BTreeMap::from([
        ("label".into(), ItemStream::once(text("host"))),
        (
            "datadom".into(),
            ItemStream::once(record([("label", vec![text("explicit")])])),
        ),
    ]);
    let expected = copied_data_document(&bindings);
    let allocation = bindings["datadom"].items.as_ptr();
    let (actual, stages) = measure(|| super::data_document_with_host_bindings(&mut bindings));
    assert_eq!(actual, expected);
    assert_eq!(actual.items.as_ptr(), allocation);
    assert!(!bindings.contains_key("datadom"));
    assert!(!stages.contains_key("copy/explicit-data-document"));
    assert!(!stages.contains_key("copy/data-document-synthesis"));
    assert!(!stages.contains_key("copy/data-document-merge"));
}

fn same_stream(actual: &ItemStream, expected: &ItemStream) {
    assert_eq!(actual, expected);
    assert_eq!(actual.diagnostics, expected.diagnostics);
    assert_eq!(actual.chain, expected.chain);
    // Iteration also checks the private cursor and the retained terminal error.
    let mut actual = actual.clone();
    let mut expected = expected.clone();
    loop {
        let next = actual.next_item();
        assert_eq!(next, expected.next_item());
        if next.is_none() {
            break;
        }
    }
}

#[test]
fn direct_input_preserves_precedence_stream_metadata_and_host_bindings() {
    let native = crate::eval::imported_cem_tree(
        cem_ml::import::import_data("<name>ivy</name>", "xml", "cem", "memory:input").unwrap(),
    );
    for explicit in [
        None,
        Some(vec![]),
        Some(vec![text("scalar")]),
        Some(vec![Item::Array(vec![native.clone()])]),
        Some(vec![native.clone()]),
        Some(vec![crate::eval::values::reference(vec![native.clone()])]),
        Some(vec![record([])]),
        Some(vec![record([("label", vec![]), ("attributes", vec![])])]),
        Some(vec![record([
            ("label", vec![text("override")]),
            (
                "attributes",
                vec![record([("private", vec![text("explicit")])])],
            ),
        ])]),
        Some(vec![text("prefix"), record([]), native.clone(), record([])]),
    ] {
        let mut bindings = BTreeMap::from([
            ("label".into(), ItemStream::once(text("host"))),
            (
                "attributes".into(),
                ItemStream::once(record([("label", vec![text("nested-host")])])),
            ),
            ("native".into(), ItemStream::once(native.clone())),
            ("empty".into(), ItemStream::empty()),
        ]);
        if let Some(items) = explicit {
            let mut stream = ItemStream::from_items(items);
            stream.chain = true;
            stream.diagnostics.push(Diagnostic {
                code: "fixture.input".into(),
                ..Default::default()
            });
            stream.error = Some(EvalError::TypeError("retained input failure"));
            stream.next_item();
            bindings.insert("datadom".into(), stream);
        }
        let original = bindings.clone();
        let expected = copied_data_document(&bindings);
        let actual = data_document_with_host_bindings(&mut bindings);
        same_stream(&actual, &expected);
        for (name, value) in &bindings {
            same_stream(value, &original[name]);
        }
        let mut complete = original.clone();
        complete.insert("datadom".into(), expected);
        bindings.insert("datadom".into(), actual);
        assert_eq!(bindings, complete);
    }
}

#[test]
fn direct_input_owns_results_without_invoking_native_accessors() {
    use crate::eval::{QueryItemView, QueryItemViewKind};
    use std::sync::Arc;
    #[derive(Debug)]
    struct Owner(Arc<()>);
    impl QueryItemView for Owner {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.input-owner"
        }
        fn identity(&self) -> String {
            "retained".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Record
        }
        fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
            panic!("must not materialize native fields")
        }
        fn field(&self, _: &str) -> Option<Vec<Item>> {
            panic!("must not invoke native accessors")
        }
    }
    for copied in [true, false] {
        let owner = Arc::new(());
        let weak = Arc::downgrade(&owner);
        let native = Item::native(Owner(owner));
        let identity = native.identity();
        let mut bindings = BTreeMap::from([
            ("native".into(), ItemStream::once(native)),
            (
                "datadom".into(),
                ItemStream::once(record([("label", vec![text("original")])])),
            ),
        ]);
        let mut output =
            with_input_copy(copied, || data_document_with_host_bindings(&mut bindings));
        let Item::Record(fields) = &mut output.items[0] else {
            panic!("record")
        };
        assert_eq!(fields["native"][0].identity(), identity);
        assert_eq!(
            Arc::strong_count(
                &fields["native"][0]
                    .view()
                    .unwrap()
                    .downcast_ref::<Owner>()
                    .unwrap()
                    .0
            ),
            1
        );
        fields.insert("label".into(), vec![text("changed")]);
        // The host caller still owns its values; output keeps native ownership
        // independently after all input bindings have gone away.
        drop(bindings);
        assert!(weak.upgrade().is_some());
        drop(output);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn direct_input_preserves_declarations_hooks_and_scope_restoration() {
    let data = TemplateData::default()
        .with_binding("label", ItemStream::once(text("host")))
        .with_binding(
            "datadom",
            ItemStream::once(record([
                (
                    "attributes",
                    vec![record([("label", vec![text("explicit")])])],
                ),
                ("slices", vec![record([("selected", vec![text("slice")])])]),
            ])),
        );
    let original = data.bindings.clone();
    for source in [
        r#"{attribute @name=label | default}{attribute @name=other | seeded}{attribute @name=selected @select=datadom.slices.selected}{p | {$label}|{$other}|{$selected}|{$datadom.attributes.label}}"#,
        r#"{template @on=expression @into=content | {$value}}{span | {$datadom.attributes.label}}"#,
        r#"{template @on=expression @into=attribute | {$value}}{span @title="{$label}" | fixed}"#,
        r#"{span | {cem:variable @name=datadom @select='{label: "inner"}'}{$datadom.label}}{$datadom.attributes.label}"#,
        r#"{try | {cem:variable @name=label @select='"inner"'}{$1 / 0}{catch @as=label @test=false | wrong}{catch | {$label}}}{$label}"#,
    ] {
        let artifact = compile_template(
            source,
            &CompileTemplateOptions {
                host_bindings: data.bindings.keys().cloned().collect(),
                ..Default::default()
            },
        );
        assert!(
            artifact.diagnostics.is_empty(),
            "{source}: {:?}",
            artifact.diagnostics
        );
        let expected = with_copied_input(|| render_compiled_template(&artifact, &data));
        let actual = render_compiled_template(&artifact, &data);
        copy_profile_tests::verify_plan(&actual, &expected);
        assert_eq!(data.bindings, original);
    }
}

#[test]
fn copied_input_construction_preserves_renderer_contracts() {
    with_copied_input(|| {
        copy_profile_tests::borrowing_preserves_focus_records_recovery_and_callbacks();
        copy_profile_tests::borrowing_preserves_reader_retention_across_renders();
        copy_profile_tests::borrowing_preserves_protected_failures_and_recovery();
        copy_profile_tests::borrowing_preserves_scoped_cancellation_and_budget_failure();
    });
}

pub(super) fn tree_fixture(count: usize) -> (TemplateArtifact, TemplateData) {
    let artifact = compile_template(
        include_str!("../../../cem-elements/demo/data-tree-view.cemt"),
        &CompileTemplateOptions {
            host_bindings: vec!["island".into()],
            ..Default::default()
        },
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let data = TemplateData::default()
        .with_binding(
            "island",
            ItemStream::once(copy_profile_tests::controls(count)),
        )
        .with_binding(
            "datadom",
            ItemStream::once(record([
                (
                    "payload",
                    vec![record([(
                        "nodes",
                        vec![record([(
                            "text",
                            vec![text(include_str!(
                                "../../../cem-elements/demo/tree-source.xml"
                            ))],
                        )])],
                    )])],
                ),
                ("slices", vec![record([])]),
            ])),
        );
    (artifact, data)
}

#[test]
fn direct_input_preserves_authored_tree() {
    let (artifact, data) = tree_fixture(2);
    let expected = with_copied_input(|| render_compiled_template(&artifact, &data));
    let actual = render_compiled_template(&artifact, &data);
    copy_profile_tests::verify_plan(&actual, &expected);
    let html = render_plan_to_html(&actual);
    assert!(html.contains("Selected branches"));
    assert!(html.contains('🍒') && html.contains('🍋'));
}

#[test]
fn default_render_builds_input_context_directly() {
    let data = TemplateData::default()
        .with_binding("label", ItemStream::once(text("host")))
        .with_binding(
            "datadom",
            ItemStream::once(record([("label", vec![text("explicit")])])),
        );
    let artifact = compile_template(
        "{p | {$datadom.label}|{$datadom.attributes.label}|{$label}}",
        &CompileTemplateOptions {
            host_bindings: vec!["label".into()],
            ..Default::default()
        },
    );
    assert!(artifact.diagnostics.is_empty());
    let (actual, stages) = measure(|| render_compiled_template(&artifact, &data));
    assert!(actual.diagnostics.is_empty());
    assert_eq!(render_plan_to_html(&actual), "<p>explicit|host|host</p>");
    assert_eq!(stages["copy/initial-bindings"].calls, 1);
    assert!(!stages.contains_key("copy/explicit-data-document"));
    assert!(!stages.contains_key("copy/data-document-synthesis"));
    assert!(!stages.contains_key("copy/data-document-merge"));
    assert_eq!(stages["render/direct-input-document"].calls, 1);
}
