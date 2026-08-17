use cem_ml::capability::{capability_manifest, CapabilityManifest, CapabilityRequest, RuntimeKind};
use cem_ml::command_runtime::{
    CommandServiceControlAckV1, CommandServiceOperationRegistryV1, CommandServiceProgressStageV1,
    CommandServiceProgressV1,
};
use cem_ml::command_service::CommandServiceStatusV1;
use cem_ml::operation_control::OperationControl;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativePlatformParityEvidence {
    schema_version: u16,
    host: &'static str,
    capability: CapabilityManifest,
    success_progress: Vec<CommandServiceProgressV1>,
    cancellation: CancellationEvidence,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CancellationEvidence {
    acknowledgement: CommandServiceControlAckV1,
    status: CommandServiceStatusV1,
    exit_code: u8,
    progress: Vec<CommandServiceProgressV1>,
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let target_identity = arguments
        .next()
        .expect("usage: native-platform-parity-emit TARGET-ID ABI-ID");
    let abi_identity = arguments
        .next()
        .expect("usage: native-platform-parity-emit TARGET-ID ABI-ID");
    assert!(arguments.next().is_none(), "unexpected extra argument");

    let capability = capability_manifest(CapabilityRequest {
        runtime: RuntimeKind::Native,
        target_identity,
        abi_identity,
        debug_control_active: false,
    })
    .expect("native platform parity identities satisfy common capability bounds");

    let registry = CommandServiceOperationRegistryV1::default();
    let control = OperationControl::default();
    let operation_id = control.operation_id();
    let registration = registry
        .register("native-platform-parity-cancel", control.clone())
        .expect("native parity cancellation request registers");
    let acknowledgement = registry
        .cancel(
            "native-platform-parity-cancel",
            Some("native parity fixture cancellation".to_owned()),
        )
        .expect("native parity cancellation routes through the common registry");
    assert!(control.is_cancelled(), "native parity control is cancelled");

    let evidence = NativePlatformParityEvidence {
        schema_version: 1,
        host: "native",
        capability,
        success_progress: success_progress("native-platform-parity-success", operation_id),
        cancellation: CancellationEvidence {
            acknowledgement,
            status: CommandServiceStatusV1::Cancelled,
            exit_code: 130,
            progress: vec![
                CommandServiceProgressV1::new(
                    "native-platform-parity-cancel",
                    operation_id,
                    1,
                    CommandServiceProgressStageV1::Accepted,
                    None,
                ),
                CommandServiceProgressV1::new(
                    "native-platform-parity-cancel",
                    operation_id,
                    2,
                    CommandServiceProgressStageV1::Terminal,
                    Some(CommandServiceStatusV1::Cancelled),
                ),
            ],
        },
    };
    drop(registration);

    serde_json::to_writer_pretty(std::io::stdout(), &evidence)
        .expect("native platform parity evidence writes to stdout");
    println!();
}

fn success_progress(
    request_id: &str,
    operation_id: cem_ml::operation_control::OperationId,
) -> Vec<CommandServiceProgressV1> {
    [
        CommandServiceProgressStageV1::Accepted,
        CommandServiceProgressStageV1::Prepared,
        CommandServiceProgressStageV1::Executing,
        CommandServiceProgressStageV1::Terminal,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, stage)| {
        CommandServiceProgressV1::new(
            request_id,
            operation_id,
            index as u64 + 1,
            stage,
            (stage == CommandServiceProgressStageV1::Terminal)
                .then_some(CommandServiceStatusV1::Succeeded),
        )
    })
    .collect()
}
