use super::{
    fixture,
    receiver_checkpoint::{open, restore_inputs},
};
use cem_ml::{
    committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES, operation_control::OperationControl,
    parser::CemAstNode, value::artifact::CemValueGraph,
};
use cem_ql::eval::{AtomValue, Item, RetainedCemNode};
use std::sync::Arc;

const TAG: &str = "cem-receiver-frame";

#[test]
fn mixed_receiver_source_refuses_a_choice_without_its_mode() {
    let schema = fixture::NESTED_RECEIVER.replace(
        "@required-children=leaf @max-total-children=1}",
        "@required-children=leaf @max-total-children=1 | {choice @name=branch | {case @children=leaf}}}",
    );
    let source = fixture::imported(&schema, "file:///runtime/receiver.cem");
    assert!(
        cem_ml::schema::structural_capsule::validate_mixed_receiver_source(
            source.tree.ast(),
            fixture::RECEIVER_URI,
            &OperationControl::default(),
        )
        .is_err()
    );
}

#[test]
fn original_nested_values_render_and_rebind_with_their_source_maps() {
    let (deployment, data) = fixture::nested_sources(fixture::NESTED_VIEW);
    let control = OperationControl::default();
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let mut original = open(&deployment, inputs.clone());
    let input = original
        .session
        .register_receiver_input(TAG, inputs.root())
        .unwrap();
    let expected = "<p>Tea/3/ok/4/leaf/Tea/detail/nested</p>";
    assert_eq!(
        original
            .session
            .render_receiver_native(&input, "nested")
            .unwrap(),
        expected
    );

    let projection = original
        .session
        .project_receiver_native(&input, "nested")
        .unwrap();
    let graph = CemValueGraph::decode(&projection, &Default::default()).unwrap();
    let attribute = fixture::attributes(&inputs.view.tree)["choice"];
    let origin = RetainedCemNode::new(inputs.view.tree.clone(), attribute)
        .unwrap()
        .query_item()
        .source_map()
        .unwrap();
    assert!(!origin.frames.is_empty());
    assert!(
        graph
            .records
            .iter()
            .any(|record| record.lexical == "leaf"
                && record.source.frames.starts_with(&origin.frames))
    );

    let child = inputs
        .view
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "group" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    assert!(original
        .session
        .register_receiver_input(
            TAG,
            RetainedCemNode::new(inputs.view.tree.clone(), child).unwrap(),
        )
        .is_err());

    let (bytes, seal) = original
        .session
        .capture_receiver_checkpoint(
            &input,
            &[&inputs.view, &inputs.cart],
            MAX_WORKSPACE_CHECKPOINT_BYTES,
            &control,
            |_| true,
        )
        .unwrap();
    let checkpoint = seal.verify(&bytes).unwrap();
    let fresh_inputs = restore_inputs(&checkpoint);
    drop(input);
    drop(original);
    let mut fresh = open(&deployment, fresh_inputs);
    let input = fresh
        .session
        .restore_receiver_checkpoint(checkpoint, MAX_WORKSPACE_CHECKPOINT_BYTES)
        .unwrap();
    assert_eq!(
        fresh
            .session
            .render_receiver_native(&input, "nested")
            .unwrap(),
        expected
    );
}

#[test]
fn original_nested_shape_and_values_refuse_before_output() {
    let (deployment, data) = fixture::nested_sources(fixture::NESTED_VIEW);
    let valid = fixture::NESTED_VIEW;
    let leaf = "{leaf @choice=leaf @items={#data} @label=detail}";
    let deep = format!(
        "{}{{leaf @choice=leaf}}{}",
        "{leaf @choice=leaf | ".repeat(65),
        "}".repeat(65)
    );
    let cases = [
        (
            valid.replace(
                "{group @count=4 | {leaf @choice=leaf @items={#data} @label=detail}} {tail}",
                "{tail} {group @count=4 | {leaf @choice=leaf @items={#data} @label=detail}}",
            ),
            false,
        ),
        (
            valid.replace(leaf, "{leaf @choice=leaf} {leaf @choice=other}"),
            false,
        ),
        (valid.replace(leaf, "{tail}"), false),
        (valid.replace(leaf, "{leaf}"), false),
        (
            valid.replace("{group @count=4 |", "{group @count=4 @items={#data} |"),
            false,
        ),
        (
            valid.replace(leaf, "{leaf @choice=leaf @unknown=bad}"),
            false,
        ),
        (
            format!(
                "@ns x = urn:foreign\n{}",
                valid.replace("{leaf ", "{x:leaf ")
            ),
            false,
        ),
        (valid.replace(leaf, &deep), false),
        (valid.replace("@count=4", "@count=2"), true),
        (valid.replace("@count=4", "@count=3"), false),
        (valid.replace("@count=4", "@count={#data}"), false),
        (valid.replace("@label=detail", "@label=x"), true),
        (
            valid.replace(leaf, "{leaf @choice=leaf @items={#missing}}"),
            true,
        ),
        (valid.replace("@choice=leaf", "@choice={#data}"), true),
    ];
    for (view, shape_allowed) in cases {
        let base = fixture::Inputs::restore(&data, &OperationControl::default()).unwrap();
        let mut inputs = Arc::try_unwrap(base).ok().unwrap();
        inputs.view = fixture::imported(&view, "file:///runtime/view.cem");
        let inputs = Arc::new(inputs);
        let mut app = open(&deployment, inputs.clone());
        let input = app.session.register_receiver_input(TAG, inputs.root());
        assert_eq!(input.is_ok(), shape_allowed, "{view}");
        if let Ok(input) = input {
            assert!(
                app.session
                    .render_receiver_native(&input, "nested")
                    .is_err(),
                "{view}"
            );
            assert!(
                app.session
                    .project_receiver_native(&input, "nested")
                    .is_err(),
                "{view}"
            );
        }
    }
}

#[test]
fn nested_typed_values_keep_exact_attribute_owners() {
    let view = fixture::NESTED_VIEW
        .replace("@choice=ok", "@choice")
        .replace("@choice=leaf", "@choice");
    let (deployment, data) = fixture::nested_typed_sources(&view);
    let control = OperationControl::default();
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let mut original = open(&deployment, inputs.clone());
    let root = inputs.root();
    let leaf = inputs
        .view
        .tree
        .ast()
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "leaf" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let leaf = RetainedCemNode::new(inputs.view.tree.clone(), leaf).unwrap();
    let root_producer = original
        .session
        .register_receiver_typed_producer(
            TAG,
            &root,
            "choice",
            vec![Item::Atomic(AtomValue::String("top".into()))],
            OperationControl::default(),
        )
        .unwrap();
    let leaf_producer = original
        .session
        .register_receiver_typed_producer(
            TAG,
            &leaf,
            "choice",
            vec![Item::Atomic(AtomValue::String("bottom".into()))],
            control.clone(),
        )
        .unwrap();
    let input = original
        .session
        .register_receiver_input_with_typed_producers(TAG, root, &[&root_producer, &leaf_producer])
        .unwrap();
    assert_eq!(
        original
            .session
            .render_receiver_native(&input, "nested")
            .unwrap(),
        "<p>Tea/3/top/4/bottom/Tea/detail/nested</p>"
    );
    control.abort_signal().abort();
    assert!(original
        .session
        .render_receiver_native(&input, "nested")
        .is_err());
}
