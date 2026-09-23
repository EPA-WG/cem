//! Native-only investigation of owned renderer input-context construction.
use super::*;
use crate::compile_profile::measure;
use std::cell::Cell;

thread_local! {
    static DIRECT_INPUT: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn enabled() -> bool {
    DIRECT_INPUT.with(Cell::get)
}

pub(super) fn with_candidate<T>(enabled: bool, run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            DIRECT_INPUT.with(|value| value.set(self.0));
        }
    }
    let _reset = Reset(DIRECT_INPUT.with(|value| value.replace(enabled)));
    run()
}

pub(super) fn take_data_document(bindings: &mut BTreeMap<String, ItemStream>) -> ItemStream {
    let _profile = crate::compile_profile::Span::new("candidate/direct-input-document");
    let mut document = bindings
        .remove(DATA_DOCUMENT_BINDING)
        .unwrap_or_else(|| ItemStream::once(Item::Record(BTreeMap::new())));
    for item in &mut document.items {
        let Item::Record(fields) = item else {
            continue;
        };
        for (name, stream) in bindings.iter() {
            // The synthesized attributes field always contains the complete host
            // binding map, including a host binding itself named attributes.
            if name != "attributes" {
                fields.entry(name.clone()).or_insert_with(|| {
                    let _copy = crate::compile_profile::Span::new("copy/input-missing-field");
                    stream.items.clone()
                });
            }
        }
        fields.entry("attributes".into()).or_insert_with(|| {
            let _copy = crate::compile_profile::Span::new("copy/input-missing-attributes");
            vec![Item::Record(
                bindings
                    .iter()
                    .map(|(name, stream)| (name.clone(), stream.items.clone()))
                    .collect(),
            )]
        });
    }
    document
}

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn record(fields: impl IntoIterator<Item = (&'static str, Vec<Item>)>) -> Item {
    Item::Record(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
}

#[test]
fn input_candidate_moves_explicit_allocation_and_skips_transient_copies() {
    let mut bindings = BTreeMap::from([
        ("label".into(), ItemStream::once(text("host"))),
        (
            "datadom".into(),
            ItemStream::once(record([("label", vec![text("explicit")])])),
        ),
    ]);
    let expected = data_document_with_host_bindings(&bindings);
    let allocation = bindings["datadom"].items.as_ptr();
    let (actual, stages) = measure(|| take_data_document(&mut bindings));
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
fn input_candidate_preserves_precedence_stream_metadata_and_host_bindings() {
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
        let expected = data_document_with_host_bindings(&bindings);
        let actual = take_data_document(&mut bindings);
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
fn input_candidate_owns_results_without_invoking_native_accessors() {
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
    for candidate in [false, true] {
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
        let mut output = if candidate {
            take_data_document(&mut bindings)
        } else {
            data_document_with_host_bindings(&bindings)
        };
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
fn input_candidate_preserves_declarations_hooks_and_scope_restoration() {
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
        let expected = render_compiled_template(&artifact, &data);
        let actual = with_candidate(true, || render_compiled_template(&artifact, &data));
        copy_profile_tests::verify_plan(&actual, &expected);
        assert_eq!(data.bindings, original);
    }
}

#[test]
fn input_candidate_preserves_renderer_contracts() {
    with_candidate(true, || {
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
fn input_candidate_preserves_authored_tree() {
    let (artifact, data) = tree_fixture(2);
    let expected = render_compiled_template(&artifact, &data);
    let actual = with_candidate(true, || render_compiled_template(&artifact, &data));
    copy_profile_tests::verify_plan(&actual, &expected);
    let html = render_plan_to_html(&actual);
    assert!(html.contains("Selected branches"));
    assert!(html.contains('🍒') && html.contains('🍋'));
}
