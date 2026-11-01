#[path = "../fixtures/application-handoff/runtime.rs"]
mod fixture;
#[test]
fn native_application_worker_fixture_has_original_sources_and_real_consumers() {
    use cem_ql::{
        application_source_endpoint::{ApplicationBootstrapOwner, ApplicationSourceEndpoint},
        application_source_transport,
    };
    use std::sync::Arc;
    let (bytes, input) = fixture::build();
    let control = Default::default();
    let inputs = fixture::Inputs::restore(&input, &control).unwrap();
    let g = cem_ml::runtime_admission::generation::NativeGeneration::claim(Arc::new(
        Default::default(),
    ))
    .unwrap();
    let code = ApplicationBootstrapOwner::new(
        &g,
        control,
        bytes.pin.clone(),
        fixture::Bootstrap {
            inputs: inputs.clone(),
        },
    )
    .unwrap();
    let endpoint = ApplicationSourceEndpoint::new(&code).unwrap();
    let mut session = endpoint
        .open(&bytes.bytes, &serde_json::to_string(&bytes.pin).unwrap())
        .unwrap();
    assert_eq!(
        fixture::validate(&mut session, &inputs).unwrap(),
        "true:true:true"
    );
    assert_eq!(session.capture().unwrap(), bytes.bytes);
    assert!(
        application_source_transport::verify(&bytes.bytes, &bytes.pin, &Default::default()).is_ok()
    );
    if let Some(dir) = std::env::var_os("CEM_APPLICATION_HANDOFF_FIXTURE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("installed.cemah"), bytes.bytes).unwrap();
        std::fs::write(dir.join("installed.inputs"), input).unwrap();
        std::fs::write(
            dir.join("installed.pin.json"),
            serde_json::to_vec(&bytes.pin).unwrap(),
        )
        .unwrap();
    }
}
