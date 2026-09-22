//! Native-only field traversal experiment; production keeps owned input copies.
use super::*;
use crate::compile_profile::measure;
use crate::eval::{Diagnostic, EvalError, QueryItemView};
use std::borrow::Cow;
use std::cell::Cell;
use std::sync::{Arc, Mutex, Weak};

thread_local! {
    static BORROW_FIELD_INPUT: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn enabled() -> bool {
    BORROW_FIELD_INPUT.with(Cell::get)
}

pub(crate) fn with_borrowed_fields<T>(operation: impl FnOnce() -> T) -> T {
    with_strategy(true, operation)
}

pub(crate) fn with_copied_fields<T>(operation: impl FnOnce() -> T) -> T {
    with_strategy(false, operation)
}

fn with_strategy<T>(borrow: bool, operation: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            BORROW_FIELD_INPUT.with(|value| value.set(self.0));
        }
    }
    let _reset = Reset(BORROW_FIELD_INPUT.with(|value| value.replace(borrow)));
    operation()
}

pub(super) fn borrowed_field(input: ItemStream, field: &str) -> Option<ItemStream> {
    // Borrow the already-owned input only within this projection. Native
    // accessors still supply owned members, and the original input retains
    // their owners throughout access. Flatten precisely one array level.
    let items: Vec<Cow<'_, Item>> = {
        let _profile = crate::compile_profile::Span::new("candidate/field-input");
        input
            .items
            .iter()
            .flat_map(|item| match item {
                Item::Array(values) => values.iter().map(Cow::Borrowed).collect(),
                Item::Native(view) => view
                    .members()
                    .map(|values| values.into_iter().map(Cow::Owned).collect())
                    .unwrap_or_else(|| vec![Cow::Borrowed(item)]),
                _ => vec![Cow::Borrowed(item)],
            })
            .collect()
    };
    if !items.is_empty()
        && !items.iter().any(|item| {
            matches!(item.as_ref(), Item::Record(_))
                || item.view().is_some_and(|view| {
                    matches!(
                        view.kind(),
                        QueryItemViewKind::Record | QueryItemViewKind::Node
                    )
                })
        })
    {
        return None;
    }
    let mut out = ItemStream::empty();
    out.diagnostics = input.diagnostics;
    out.error = input.error;
    for item in items {
        match item.as_ref() {
            Item::Record(record) => {
                if let Some(values) = record.get(field) {
                    let _profile = crate::compile_profile::Span::new("copy/field-selected");
                    out.items.extend(values.clone());
                }
            }
            Item::Native(view) => {
                if let Some(values) = view.field(field) {
                    out.items.extend(values);
                }
            }
            _ => {}
        }
    }
    Some(out)
}

fn text(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.into()))
}

fn record(value: Item) -> Item {
    Item::Record(BTreeMap::from([("label".into(), vec![value])]))
}

fn same_stream(actual: &ItemStream, expected: &ItemStream) {
    assert_eq!(actual, expected);
    assert_eq!(actual.diagnostics, expected.diagnostics);
    assert_eq!(actual.cursor, expected.cursor);
    assert_eq!(actual.chain, expected.chain);
}

#[test]
fn field_candidate_borrows_input_but_returns_owned_selected_values() {
    let input = ItemStream::once(record(record(text("kept"))));
    let expected = record_field(input.clone(), "label").unwrap();
    let (mut actual, stages) =
        measure(|| with_borrowed_fields(|| record_field(input.clone(), "label").unwrap()));
    same_stream(&actual, &expected);
    let Item::Record(fields) = &mut actual.items[0] else {
        panic!("selected record")
    };
    fields.clear();
    assert_eq!(record_field(input, "label").unwrap(), expected);
    assert!(!stages.contains_key("copy/field-input"));
    assert_eq!(stages["candidate/field-input"].calls, 1);
    assert_eq!(stages["copy/field-selected"].calls, 1);
}

#[test]
fn field_candidate_preserves_one_level_projection_and_stream_status() {
    let row = record(text("kept"));
    for (items, expected) in [
        (vec![], Some(vec![])),
        (vec![Item::Array(vec![])], Some(vec![])),
        (vec![row.clone()], Some(vec![text("kept")])),
        (vec![Item::Record(BTreeMap::new())], Some(vec![])),
        (vec![text("scalar")], None),
        (vec![Item::Array(vec![text("scalar")])], None),
        (
            vec![Item::Array(vec![Item::Array(vec![row.clone()])])],
            None,
        ),
        (
            vec![text("ignored"), Item::Array(vec![row.clone(), row.clone()])],
            Some(vec![text("kept"), text("kept")]),
        ),
        (
            vec![row.clone(), Item::Array(vec![Item::Array(vec![row])])],
            Some(vec![text("kept")]),
        ),
    ] {
        let mut input = ItemStream::from_items(items);
        input.cursor = 3;
        input.chain = true;
        input.error = Some(EvalError::TypeError("retained error"));
        input.diagnostics.push(Diagnostic {
            code: "fixture.field".into(),
            severity: Severity::Warning,
            ..Default::default()
        });
        let baseline = record_field(input.clone(), "label");
        let candidate = with_borrowed_fields(|| record_field(input.clone(), "label"));
        assert_eq!(
            candidate.as_ref().map(|stream| &stream.items),
            expected.as_ref()
        );
        if let (Some(actual), Some(expected)) = (candidate, baseline) {
            same_stream(&actual, &expected);
            assert_eq!(actual.diagnostics, input.diagnostics);
            assert_eq!(actual.error, input.error);
            // Projection has always reset the cursor and chain marker.
            assert_eq!(actual.cursor, 0);
            assert!(!actual.chain);
        }
    }
}

#[test]
fn field_candidate_preserves_query_reads_shadowing_and_recovery() {
    use crate::api::{compile, evaluate, CompileContext, EvaluationContext};
    let input = ItemStream::once(Item::Record(BTreeMap::from([
        ("label".into(), vec![text("outer")]),
        ("extra".into(), vec![record(text("nested"))]),
        (
            "rows".into(),
            vec![Item::Array(vec![
                record(text("first")),
                record(text("second")),
            ])],
        ),
    ])));
    let context = EvaluationContext {
        policy_bindings: BTreeMap::from([("input".into(), input.clone())]),
        ..Default::default()
    };
    let options = CompileContext {
        policy_bindings: context.policy_bindings.clone(),
        ..Default::default()
    };
    for source in [
        "input",
        "input.label",
        "input.extra.label",
        "input.rows.label",
        "input.missing",
        "(input.label, input.label)",
        "(input.extra, input)",
        "record:entries(input).key",
        "{ let input = {label: \"inner\"}; input.label }",
        "seq:map((1, 2), fn(n) => input.label)",
        "declare function first() { second() } declare function second() { input.label } first()",
        "try { 1 / 0 } catch (code, message) { input.label }",
    ] {
        let query = compile(source, &options).unwrap();
        let expected = evaluate(&query, &context);
        let actual = with_borrowed_fields(|| evaluate(&query, &context));
        assert!(actual.error.is_none(), "{source}: {actual:?}");
        same_stream(&actual, &expected);
        assert_eq!(context.policy_bindings["input"], input);
    }
}

#[test]
fn field_candidate_preserves_native_accessor_order_and_parent_retention() {
    type Log = Arc<Mutex<Vec<&'static str>>>;
    #[derive(Debug)]
    struct Parent(Arc<()>, Log);
    #[derive(Debug)]
    struct Row(Weak<()>, Log);
    impl Drop for Parent {
        fn drop(&mut self) {
            self.1.lock().unwrap().push("parent:drop");
        }
    }
    impl QueryItemView for Parent {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.field-parent"
        }
        fn identity(&self) -> String {
            "parent".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            QueryItemViewKind::Array
        }
        fn members(&self) -> Option<Vec<Item>> {
            self.1.lock().unwrap().push("parent:members");
            Some(vec![Item::native(Row(
                Arc::downgrade(&self.0),
                self.1.clone(),
            ))])
        }
    }
    impl QueryItemView for Row {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn representation_id(&self) -> &'static str {
            "fixture.field-row"
        }
        fn identity(&self) -> String {
            "row".into()
        }
        fn kind(&self) -> QueryItemViewKind {
            self.1.lock().unwrap().push("row:kind");
            QueryItemViewKind::Record
        }
        fn field(&self, name: &str) -> Option<Vec<Item>> {
            assert!(
                self.0.upgrade().is_some(),
                "parent retained through field access"
            );
            assert_eq!(name, "label");
            self.1.lock().unwrap().push("row:field");
            Some(vec![text("native")])
        }
    }
    for candidate in [false, true] {
        let log = Arc::new(Mutex::new(vec![]));
        let input = ItemStream::once(Item::native(Parent(Arc::new(()), log.clone())));
        let run = || record_field(input, "label").unwrap();
        let result = if candidate {
            with_borrowed_fields(run)
        } else {
            run()
        };
        assert_eq!(result.items, vec![text("native")]);
        assert_eq!(
            *log.lock().unwrap(),
            ["parent:members", "row:kind", "row:field", "parent:drop"]
        );
    }
}
