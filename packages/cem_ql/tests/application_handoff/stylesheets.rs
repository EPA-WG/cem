use super::{fixture, receiver_checkpoint::restore_inputs};
use cem_ml::{
    committed_checkpoint::MAX_WORKSPACE_CHECKPOINT_BYTES,
    module_resolution::{
        CemModuleUrlResolution, CemModuleUrlResolutionCapability, CemModuleUrlResolutionError,
        CemModuleUrlResolutionRequest, CemModuleUrlResolver, CemResolutionContextHandle,
    },
    operation_control::OperationControl,
    runtime_admission::generation::NativeGeneration,
};
use cem_ql::{
    application_source_endpoint::{
        ApplicationBootstrap, ApplicationBootstrapOwner, ApplicationSourceEndpoint,
        PreparedApplication,
    },
    application_source_transport::VerifiedApplicationSources,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

const TAG: &str = "cem-receiver-frame";

struct StyledBootstrap {
    inputs: Arc<fixture::Inputs>,
    policy: Option<CemModuleUrlResolutionCapability>,
}

impl ApplicationBootstrap for StyledBootstrap {
    fn prepare(
        &self,
        sources: Arc<VerifiedApplicationSources>,
        generation: &Arc<NativeGeneration>,
        control: OperationControl,
    ) -> Result<PreparedApplication, String> {
        let staged = fixture::Bootstrap {
            inputs: self.inputs.clone(),
        }
        .prepare(sources, generation, control)?;
        match &self.policy {
            Some(policy) => staged.with_stylesheet_policy(policy.clone(), Default::default()),
            None => Ok(staged),
        }
    }
}

struct RevocableResolver {
    native: CemModuleUrlResolutionCapability,
    revoked: AtomicBool,
}

impl CemModuleUrlResolver for RevocableResolver {
    fn context_cache_identity(&self, _: &CemResolutionContextHandle) -> Option<String> {
        if self.revoked.load(Ordering::Acquire) {
            return None;
        }
        self.native.cache_identity()
    }

    fn resolve_module_url(
        &self,
        request: &CemModuleUrlResolutionRequest,
    ) -> Result<CemModuleUrlResolution, CemModuleUrlResolutionError> {
        self.native.resolve_from_context(
            request.purpose,
            request.authored_specifier.clone(),
            self.native.context(),
            request.referrer.clone(),
            request.source_map.clone(),
        )
    }
}

#[test]
fn styled_declarations_require_original_policy_and_restage_on_checkpoint_restore() {
    let (deployment, data) = fixture::build_styled();
    let pins = serde_json::to_string(&deployment.pin).unwrap();
    let control = OperationControl::default();
    let inputs = fixture::Inputs::restore(&data, &control).unwrap();
    let provision = |inputs, policy| {
        let generation = NativeGeneration::claim(Arc::new(Default::default())).unwrap();
        let code = ApplicationBootstrapOwner::new(
            &generation,
            control.clone(),
            deployment.pin.clone(),
            StyledBootstrap { inputs, policy },
        )
        .unwrap();
        let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
        (generation, code, endpoint)
    };
    for policy in [None, Some(fixture::style_capability(true))] {
        let (generation, _code, endpoint) = provision(inputs.clone(), policy);
        assert!(endpoint.open(&deployment.bytes, &pins).is_err());
        assert_eq!(generation.inventory().unwrap().live_roots, 2);
    }
    let resolver = Arc::new(RevocableResolver {
        native: fixture::style_capability(false),
        revoked: AtomicBool::new(false),
    });
    let capability = CemModuleUrlResolutionCapability::new(
        resolver.clone(),
        CemResolutionContextHandle::new("fixture"),
    );
    let (generation, _code, endpoint) = provision(inputs.clone(), Some(capability));
    let mut session = endpoint.open(&deployment.bytes, &pins).unwrap();
    let styles = session.stylesheets_json(TAG).unwrap();
    assert!(styles.contains("--imported:ready") && styles.contains("--original:ready"));
    assert!(styles.contains("cem-receiver-frame"));
    assert_eq!(session.stylesheets_json("cem-count").unwrap(), "[]");
    assert!(session.stylesheets_json("cem-missing").is_err());
    let input = session.register_receiver_input(TAG, inputs.root()).unwrap();
    assert_eq!(
        session.render_receiver_native(&input, "styled").unwrap(),
        "<p>Tea/3/ok/styled</p>"
    );
    let (bytes, seal) = session
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
    resolver.revoked.store(true, Ordering::Release);
    assert!(session.stylesheets_json(TAG).is_err());
    assert!(session.render_receiver_native(&input, "styled").is_err());
    assert!(session.capture().is_err());
    resolver.revoked.store(false, Ordering::Release);
    assert!(session.stylesheets_json(TAG).is_err());
    drop(input);
    drop(session);
    assert_eq!(generation.inventory().unwrap().live_roots, 2);
    let (_fresh_generation, _fresh_code, fresh) =
        provision(fresh_inputs, Some(fixture::style_capability(false)));
    let mut session = fresh.open(&deployment.bytes, &pins).unwrap();
    let input = session
        .restore_receiver_checkpoint(checkpoint, MAX_WORKSPACE_CHECKPOINT_BYTES)
        .unwrap();
    assert_eq!(session.stylesheets_json(TAG).unwrap(), styles);
    assert_eq!(
        session.render_receiver_native(&input, "styled").unwrap(),
        "<p>Tea/3/ok/styled</p>"
    );
}
