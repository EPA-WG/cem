use cem_ml::capability::{capability_manifest, CapabilityRequest, RuntimeKind};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let target_identity = arguments
        .next()
        .expect("usage: native-capability-emit TARGET-ID ABI-ID");
    let abi_identity = arguments
        .next()
        .expect("usage: native-capability-emit TARGET-ID ABI-ID");
    assert!(arguments.next().is_none(), "unexpected extra argument");

    let manifest = capability_manifest(CapabilityRequest {
        runtime: RuntimeKind::Native,
        target_identity,
        abi_identity,
        debug_control_active: cfg!(feature = "debug-control"),
    })
    .expect("native deployment identities satisfy common capability bounds");
    serde_json::to_writer_pretty(std::io::stdout(), &manifest)
        .expect("native capability manifest writes to stdout");
    println!();
}
