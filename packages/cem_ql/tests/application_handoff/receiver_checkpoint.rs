use super::fixture;
use cem_ml::{
    committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES, operation_control::OperationControl,
    runtime_admission::generation::NativeGeneration,
};
use cem_ql::{
    application_source_endpoint::{
        ApplicationBootstrapOwner, ApplicationSourceEndpoint, ApplicationSourceSession,
        VerifiedReceiverCheckpoint,
    },
    application_source_transport::{BuiltApplicationSources, SourceInput},
};
use std::sync::Arc;

const TAG: &str = "cem-receiver-frame";

struct Session {
    session: ApplicationSourceSession,
    _generation: Arc<NativeGeneration>,
    _code: Arc<ApplicationBootstrapOwner>,
    _endpoint: Arc<ApplicationSourceEndpoint>,
}

fn open(deployment: &BuiltApplicationSources, inputs: Arc<fixture::Inputs>) -> Session {
    let generation = NativeGeneration::claim(Arc::new(Default::default())).unwrap();
    let code = ApplicationBootstrapOwner::new(
        &generation,
        OperationControl::default(),
        deployment.pin.clone(),
        fixture::Bootstrap { inputs },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let session = endpoint
        .open(
            &deployment.bytes,
            &serde_json::to_string(&deployment.pin).unwrap(),
        )
        .unwrap();
    Session {
        session,
        _generation: generation,
        _code: code,
        _endpoint: endpoint,
    }
}

fn restore_inputs(checkpoint: &VerifiedReceiverCheckpoint) -> Arc<fixture::Inputs> {
    let copy = |input: &SourceInput| SourceInput {
        identity: input.identity.clone(),
        tree: input.tree.clone(),
        capture: input.capture.clone(),
    };
    let view = checkpoint.input_source();
    let cart = checkpoint
        .sources()
        .find(|source| source.identity.uri != view.identity.uri)
        .unwrap();
    Arc::new(fixture::Inputs {
        view: copy(view),
        cart: copy(cart),
    })
}

#[test]
fn changed_receiver_sources_resume_through_fresh_original_bindings() {
    let (deployment, _) = fixture::build_prepared();
    let (same_deployment, edited) = fixture::prepared_sources(
        "{view @items={#data} @count=4 @choice=ok}",
        fixture::RECEIVER,
        "{cart @title=Coffee | {item @quantity=4}}",
    );
    assert_eq!(same_deployment.bytes, deployment.bytes);
    let control = OperationControl::default();
    let inputs = fixture::Inputs::restore(&edited, &control).unwrap();
    let mut original = open(&deployment, inputs.clone());
    let owner = original
        .session
        .register_receiver_input(TAG, inputs.root())
        .unwrap();
    assert_eq!(
        original
            .session
            .render_receiver_native(&owner, "view")
            .unwrap(),
        "<p>Coffee/4/ok/view</p>"
    );
    assert!(original.session.capture().is_err());
    let (bytes, seal) = original
        .session
        .capture_receiver_checkpoint(
            &owner,
            &[&inputs.view, &inputs.cart],
            MAX_WORKSPACE_CHECKPOINT_BYTES,
            &control,
            |_| true,
        )
        .unwrap();
    let checkpoint = seal.verify(&bytes).unwrap();
    let fresh_inputs = restore_inputs(&checkpoint);
    assert!(!Arc::ptr_eq(&inputs.view.tree, &fresh_inputs.view.tree));
    assert!(!Arc::ptr_eq(&inputs.cart.tree, &fresh_inputs.cart.tree));
    assert!(checkpoint
        .input_context()
        .unwrap()
        .bindings
        .contains_key("data"));
    drop(owner);
    drop(original);
    let mut fresh = open(&deployment, fresh_inputs);
    let owner = fresh
        .session
        .restore_receiver_checkpoint(checkpoint, MAX_WORKSPACE_CHECKPOINT_BYTES)
        .unwrap();
    assert_eq!(
        fresh
            .session
            .render_receiver_native(&owner, "view")
            .unwrap(),
        "<p>Coffee/4/ok/view</p>"
    );
    assert!(fresh.session.capture().is_err());
}

#[test]
fn receiver_checkpoints_refuse_foreign_sources_stale_contexts_and_unbudgeted_bytes() {
    let (deployment, data) = fixture::build_prepared();
    let authority = OperationControl::default();
    let inputs = fixture::Inputs::restore(&data, &authority).unwrap();
    let foreign = fixture::Inputs::restore(&data, &authority).unwrap();
    let mut original = open(&deployment, inputs.clone());
    let owner = original
        .session
        .register_receiver_input(TAG, inputs.root())
        .unwrap();
    let capture = |available, policy| {
        original.session.capture_receiver_checkpoint(
            &owner,
            &[&inputs.view, &inputs.cart],
            available,
            &authority,
            |_| policy,
        )
    };
    assert!(capture(deployment.bytes.len() + 1, true).is_err());
    assert!(capture(MAX_WORKSPACE_CHECKPOINT_BYTES, false).is_err());
    assert!(original
        .session
        .capture_receiver_checkpoint(
            &owner,
            &[&inputs.view, &foreign.cart],
            MAX_WORKSPACE_CHECKPOINT_BYTES,
            &authority,
            |_| true,
        )
        .is_err());
    let (_, foreign_data) = fixture::prepared_sources(
        fixture::VIEW,
        fixture::RECEIVER,
        "{cart @title=Coffee | {item @quantity=4}}",
    );
    let edited = fixture::Inputs::restore(&foreign_data, &authority).unwrap();
    let mut unrelated = open(&deployment, edited.clone());
    let unrelated_owner = unrelated
        .session
        .register_receiver_input(TAG, edited.root())
        .unwrap();
    let (foreign_bytes, foreign_seal) = unrelated
        .session
        .capture_receiver_checkpoint(
            &unrelated_owner,
            &[&edited.view, &edited.cart],
            MAX_WORKSPACE_CHECKPOINT_BYTES,
            &authority,
            |_| true,
        )
        .unwrap();
    let (bytes, seal) = capture(MAX_WORKSPACE_CHECKPOINT_BYTES, true).unwrap();
    assert_ne!(bytes, foreign_bytes);
    assert!(seal.verify(&foreign_bytes).is_err());

    let checkpoint = foreign_seal.verify(&foreign_bytes).unwrap();
    let mut fresh = open(&deployment, restore_inputs(&checkpoint));
    assert!(fresh
        .session
        .restore_receiver_checkpoint(checkpoint, deployment.bytes.len() + 1)
        .is_err());

    let (bytes, seal) = capture(MAX_WORKSPACE_CHECKPOINT_BYTES, true).unwrap();
    let checkpoint = seal.verify(&bytes).unwrap();
    let mut wrong_host = open(&deployment, foreign);
    assert!(wrong_host
        .session
        .restore_receiver_checkpoint(checkpoint, MAX_WORKSPACE_CHECKPOINT_BYTES)
        .is_err());

    let (bytes, seal) = capture(MAX_WORKSPACE_CHECKPOINT_BYTES, true).unwrap();
    assert!(original
        .session
        .with_application(|_, host| {
            let scope = host.registered_source_scope(&inputs.view.tree).unwrap();
            assert!(host.set_context(scope, Some(Default::default())));
            Ok(())
        })
        .is_err());
    assert!(seal.verify(&bytes).is_err());
}

#[test]
fn receiver_checkpoint_seal_obeys_cancellation_and_refuses_credentials() {
    let (deployment, data) = fixture::build_prepared();
    let authority = OperationControl::default();
    let inputs = fixture::Inputs::restore(&data, &authority).unwrap();
    let mut original = open(&deployment, inputs.clone());
    let owner = original
        .session
        .register_receiver_input(TAG, inputs.root())
        .unwrap();
    let (bytes, seal) = original
        .session
        .capture_receiver_checkpoint(
            &owner,
            &[&inputs.view, &inputs.cart],
            MAX_WORKSPACE_CHECKPOINT_BYTES,
            &authority,
            |_| true,
        )
        .unwrap();
    authority.abort_signal().abort();
    assert!(seal.verify(&bytes).is_err());

    let (_, private) = fixture::prepared_sources(
        fixture::VIEW,
        fixture::RECEIVER,
        "{cart @title=Tea @password=private}",
    );
    let authority = OperationControl::default();
    let inputs = fixture::Inputs::restore(&private, &authority).unwrap();
    let mut private_session = open(&deployment, inputs.clone());
    let owner = private_session
        .session
        .register_receiver_input(TAG, inputs.root())
        .unwrap();
    assert!(private_session
        .session
        .capture_receiver_checkpoint(
            &owner,
            &[&inputs.view, &inputs.cart],
            MAX_WORKSPACE_CHECKPOINT_BYTES,
            &authority,
            |_| true,
        )
        .is_err());
}

#[test]
fn receiver_checkpoint_seal_expires_after_capture() {
    let (deployment, data) = fixture::build_prepared();
    let authority = OperationControl::default();
    let inputs = fixture::Inputs::restore(&data, &authority).unwrap();
    let mut original = open(&deployment, inputs.clone());
    let owner = original
        .session
        .register_receiver_input(TAG, inputs.root())
        .unwrap();
    let (bytes, seal) = original
        .session
        .capture_receiver_checkpoint(
            &owner,
            &[&inputs.view, &inputs.cart],
            MAX_WORKSPACE_CHECKPOINT_BYTES,
            &authority,
            |_| true,
        )
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(5));
    assert!(seal.verify(&bytes).is_err());
}
