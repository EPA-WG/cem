use super::{fixture, receiver_checkpoint::open};
use cem_ml::{
    committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES, operation_control::OperationControl,
};
use cem_ql::eval::{AtomValue, Item};

const TAG: &str = "cem-receiver-frame";

#[test]
fn original_typed_producer_is_required_and_remains_live_through_render() {
    let (deployment, data) = fixture::build_typed();
    let control = OperationControl::default();
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let foreign = fixture::Inputs::restore(&data, &control).unwrap();
    let mut installed = open(&deployment, inputs.clone());
    let root = inputs.root();
    let ordinary = installed
        .session
        .register_receiver_input(TAG, root.clone())
        .unwrap();
    assert!(installed
        .session
        .render_receiver_native(&ordinary, "typed")
        .is_err());
    drop(ordinary);

    let issue = |session: &mut cem_ql::application_source_endpoint::ApplicationSourceSession,
                 root: &cem_ql::eval::RetainedCemNode,
                 field: &str,
                 value,
                 control| {
        session.register_receiver_typed_producer(
            TAG,
            root,
            field,
            vec![Item::Atomic(value)],
            control,
        )
    };
    assert!(issue(
        &mut installed.session,
        &foreign.root(),
        "choice",
        AtomValue::String("typed".into()),
        control.clone()
    )
    .is_err());
    assert!(issue(
        &mut installed.session,
        &root,
        "choice",
        AtomValue::Boolean(true),
        control.clone()
    )
    .is_err());
    assert!(issue(
        &mut installed.session,
        &root,
        "count",
        AtomValue::Integer(3),
        control.clone()
    )
    .is_err());
    assert!(issue(
        &mut installed.session,
        &root,
        "choice",
        AtomValue::String("x".repeat(MAX_WORKSPACE_CHECKPOINT_BYTES)),
        control.clone(),
    )
    .is_err());

    let producer = issue(
        &mut installed.session,
        &root,
        "choice",
        AtomValue::String("typed".into()),
        control.clone(),
    )
    .unwrap();
    let mut sibling = open(&deployment, inputs.clone());
    assert!(sibling
        .session
        .register_receiver_input_with_typed_producers(TAG, root.clone(), &[&producer])
        .is_err());
    assert!(installed
        .session
        .register_receiver_input_with_typed_producers(TAG, root.clone(), &[&producer, &producer])
        .is_err());
    let input = installed
        .session
        .register_receiver_input_with_typed_producers(TAG, root, &[&producer])
        .unwrap();
    assert_eq!(
        installed
            .session
            .render_receiver_native(&input, "typed")
            .unwrap(),
        "<p>Tea/3/typed/typed</p>"
    );
    let projected = installed
        .session
        .project_receiver_native(&input, "typed")
        .unwrap();
    let graph =
        cem_ml::value::artifact::CemValueGraph::decode(&projected, &Default::default()).unwrap();
    let attribute = fixture::attributes(&inputs.view.tree)["choice"];
    let origin = cem_ql::eval::RetainedCemNode::new(inputs.view.tree.clone(), attribute)
        .unwrap()
        .query_item()
        .source_map()
        .unwrap();
    assert!(!origin.frames.is_empty());
    assert!(graph.records.iter().any(
        |record| record.lexical == "typed" && record.source.frames.starts_with(&origin.frames)
    ));
    assert!(installed.session.capture().is_err());
    control.abort_signal().abort();
    assert!(installed
        .session
        .render_receiver_native(&input, "typed")
        .is_err());
    drop(producer);
    assert!(installed
        .session
        .project_receiver_native(&input, "typed")
        .is_err());
    assert!(installed
        .session
        .capture_receiver_checkpoint(
            &input,
            &[&inputs.view, &inputs.cart],
            MAX_WORKSPACE_CHECKPOINT_BYTES,
            &OperationControl::default(),
            |_| true,
        )
        .is_err());
    drop(input);
    assert_eq!(installed.session.capture().unwrap(), deployment.bytes);
}

#[test]
fn typed_producer_refuses_lexical_profiles_authored_values_and_stale_scopes() {
    let control = OperationControl::default();
    let (deployment, data) =
        fixture::prepared_sources(fixture::TYPED_VIEW, fixture::RECEIVER, fixture::CART);
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let mut lexical = open(&deployment, inputs.clone());
    assert!(lexical
        .session
        .register_receiver_typed_producer(
            TAG,
            &inputs.root(),
            "choice",
            vec![Item::Atomic(AtomValue::String("typed".into()))],
            control.clone(),
        )
        .is_err());

    let (deployment, data) = fixture::build_typed();
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let authored = std::sync::Arc::new(fixture::Inputs {
        view: fixture::imported(fixture::VIEW, "file:///runtime/authored-view.cem"),
        cart: cem_ql::application_source_transport::SourceInput {
            identity: inputs.cart.identity.clone(),
            tree: inputs.cart.tree.clone(),
            capture: inputs.cart.capture.clone(),
        },
    });
    let mut authored_session = open(&deployment, authored.clone());
    assert!(authored_session
        .session
        .register_receiver_typed_producer(
            TAG,
            &authored.root(),
            "choice",
            vec![Item::Atomic(AtomValue::String("typed".into()))],
            control.clone(),
        )
        .is_err());

    let mut installed = open(&deployment, inputs.clone());
    let root = inputs.root();
    let producer = installed
        .session
        .register_receiver_typed_producer(
            TAG,
            &root,
            "choice",
            vec![Item::Atomic(AtomValue::String("typed".into()))],
            control,
        )
        .unwrap();
    let input = installed
        .session
        .register_receiver_input_with_typed_producers(TAG, root, &[&producer])
        .unwrap();
    assert!(installed
        .session
        .with_application(|_, host| {
            let scope = host.registered_source_scope(&inputs.view.tree).unwrap();
            assert!(host.set_context(scope, Some(Default::default())));
            Ok(())
        })
        .is_err());
    assert!(installed
        .session
        .render_receiver_native(&input, "typed")
        .is_err());
}
