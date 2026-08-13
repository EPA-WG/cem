//! Gate 9 fixtures that run only when the default debugger feature is absent.

use std::time::Duration;

use serde_json::{json, Value};

use crate::capability::{
    capability_manifest, CapabilityAvailability, CapabilityRequest, ControlCapabilityKind,
    ControlCoverage, OperationHostLimits, RuntimeKind,
};
use crate::engine::{FormatIdentity, InputFormat};
use crate::operation_control::{
    ControlCause, ControlError, OperationControl, OperationId, ROOT_EXECUTION_SCOPE_ID,
};
use crate::operation_handle::{
    ArtifactDisposition, EventSubscriptionOptions, OperationEventKind, OperationHandle,
    OperationOutcome, OperationTerminalStatus,
};
use crate::resumable_operation::{
    execute_operation_work, OperationSource, ResumableOperationHost, ResumableRunRequest,
};
use crate::run_config::ScopeConfig;
use crate::scheduler::{AbortSignal, ScopePolicy};
use crate::schema::registry::{
    XML_CONTENT_TYPE, XML_SCHEMA_URI, XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI,
};

fn request(runtime: RuntimeKind, active: bool) -> CapabilityRequest {
    CapabilityRequest {
        runtime,
        target_identity: "gate-9-stripped-target".to_owned(),
        abi_identity: "gate-9-stripped-abi".to_owned(),
        debug_control_active: active,
    }
}

#[test]
fn stripped_capability_rejects_debug_activation_and_retains_core_controls() {
    let manifest = capability_manifest(request(RuntimeKind::Native, false)).unwrap();
    assert!(!manifest.debug_control.compiled);
    assert!(!manifest.debug_control.active);
    assert_eq!(manifest.debug_control.dap_adapter_version, None);
    assert_eq!(manifest.debug_control.cem_debug_request_version, None);

    for control in [
        ControlCapabilityKind::Pause,
        ControlCapabilityKind::SourceBreakpoints,
        ControlCapabilityKind::Stepping,
        ControlCapabilityKind::SuspendedInspection,
        ControlCapabilityKind::Dap,
        ControlCapabilityKind::CemDebugRequests,
    ] {
        let capability = manifest.control(control);
        assert_eq!(capability.availability, CapabilityAvailability::Unavailable);
        assert_eq!(capability.coverage, ControlCoverage::None);
    }
    for control in [
        ControlCapabilityKind::RootCancellation,
        ControlCapabilityKind::ScopedCancellation,
        ControlCapabilityKind::OperationHandles,
        ControlCapabilityKind::BoundedSubscriptions,
    ] {
        assert_ne!(
            manifest.control(control).availability,
            CapabilityAvailability::Unavailable
        );
    }

    let error = capability_manifest(request(RuntimeKind::Native, true)).unwrap_err();
    assert_eq!(error.code, "cem.capability.debug_control_unavailable");
}

#[test]
fn stripped_control_core_enforces_cancellation_stack_memory_and_timeout() {
    let cancellation = OperationControl::new(AbortSignal::new());
    cancellation
        .cancel_root(Some("stripped fixture".to_owned()), None)
        .unwrap();
    assert!(matches!(
        cancellation.check_scope(ROOT_EXECUTION_SCOPE_ID),
        Err(ControlError::Triggered(failure))
            if matches!(failure.cause, ControlCause::HostCancellation { .. })
    ));

    let stack = OperationControl::with_policy(
        OperationId::from_raw(9_001),
        AbortSignal::new(),
        ScopePolicy::host_root().with_stack_depth(1),
    )
    .unwrap();
    let task = stack.register_task(ROOT_EXECUTION_SCOPE_ID).unwrap();
    let _first = stack
        .enter_frame(task, ROOT_EXECUTION_SCOPE_ID, None)
        .unwrap();
    assert!(matches!(
        stack.enter_frame(task, ROOT_EXECUTION_SCOPE_ID, None),
        Err(ControlError::Triggered(failure))
            if matches!(failure.cause, ControlCause::StackDepthExceeded { observed: 2, limit: 1 })
    ));

    let memory = OperationControl::with_policy(
        OperationId::from_raw(9_002),
        AbortSignal::new(),
        ScopePolicy::host_root().with_memory_bytes(8),
    )
    .unwrap();
    let _permit = memory
        .charge_memory(ROOT_EXECUTION_SCOPE_ID, 8, None)
        .unwrap();
    assert!(matches!(
        memory.charge_memory(ROOT_EXECUTION_SCOPE_ID, 1, None),
        Err(ControlError::Triggered(failure))
            if matches!(failure.cause, ControlCause::MemoryExceeded { .. })
    ));

    let timeout = OperationControl::with_policy(
        OperationId::from_raw(9_003),
        AbortSignal::new(),
        ScopePolicy::host_root().with_timeout_ms(Some(2)),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(5));
    assert!(matches!(
        timeout.check_scope(ROOT_EXECUTION_SCOPE_ID),
        Err(ControlError::Triggered(failure))
            if matches!(failure.cause, ControlCause::TimeoutExceeded { .. })
    ));
}

#[test]
fn stripped_operation_events_and_exactly_one_terminal_remain_available() {
    let (handle, publisher) = OperationHandle::<Value>::new(
        OperationControl::new(AbortSignal::new()),
        OperationHostLimits::default(),
    )
    .unwrap();
    let mut events = handle
        .subscribe(EventSubscriptionOptions::default())
        .unwrap();
    handle
        .publish_event(
            OperationEventKind::Progress,
            &json!({ "completed": 1, "total": 2 }),
        )
        .unwrap();
    handle
        .publish_event(
            OperationEventKind::Diagnostic,
            &json!({ "code": "cem.gate9.fixture", "severity": "info" }),
        )
        .unwrap();

    let winner = publisher
        .settle(OperationOutcome::succeeded(
            json!({ "stable": true }),
            Vec::new(),
            ArtifactDisposition::default(),
        ))
        .unwrap();
    assert!(winner.published());
    let loser = publisher
        .settle(OperationOutcome::cancelled(
            Some("late cancellation".to_owned()),
            Vec::new(),
            ArtifactDisposition::default(),
        ))
        .unwrap();
    assert!(!loser.published());
    assert_eq!(loser.outcome().status(), OperationTerminalStatus::Succeeded);
    assert_eq!(
        handle.terminal_summary().unwrap().status,
        OperationTerminalStatus::Succeeded
    );

    let mut kinds = Vec::new();
    while let Some(event) = events
        .blocking_next_timeout(Duration::from_millis(10))
        .unwrap()
    {
        kinds.push(event.kind);
    }
    assert!(kinds.contains(&OperationEventKind::Progress));
    assert!(kinds.contains(&OperationEventKind::Diagnostic));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == OperationEventKind::Terminal)
            .count(),
        1
    );
}

#[test]
fn stripped_resumable_query_commits_deterministically_across_packet_budgets() {
    let one_packet = drive_query(1, false);
    let reversed_batches = drive_query(4, true);
    assert_eq!(one_packet.status, OperationTerminalStatus::Succeeded);
    assert_eq!(one_packet.result, reversed_batches.result);
}

fn drive_query(
    max_packets: u32,
    reverse_batches: bool,
) -> crate::resumable_operation::ResumableOperationTerminal {
    let mut host = ResumableOperationHost::new(2).unwrap();
    let operation = host.start(query_request()).unwrap().operation_id;
    for _ in 0..32 {
        let poll = host.poll(operation, max_packets).unwrap();
        if let Some(terminal) = poll.terminal {
            return terminal;
        }
        let mut packets = poll.packets;
        if reverse_batches {
            packets.reverse();
        }
        for packet in packets {
            let accepted = host
                .accept_result(execute_operation_work(packet).unwrap())
                .unwrap();
            if let Some(terminal) = accepted.terminal {
                return terminal;
            }
        }
    }
    panic!("stripped resumable query did not reach a terminal result")
}

fn query_request() -> ResumableRunRequest {
    ResumableRunRequest::Query {
        data: source(
            "memory:gate-9.xml",
            b"<catalog><book id=\"a\"/><book id=\"b\"/></catalog>",
            XML_CONTENT_TYPE,
            XML_SCHEMA_URI,
            Some(InputFormat::Xml),
        ),
        query: source(
            "memory:gate-9.xpath",
            b"/catalog/book/@id/string()",
            XPATH_CONTENT_TYPE,
            XPATH_SCHEMA_URI,
            None,
        ),
    }
}

fn source(
    uri: &str,
    bytes: &[u8],
    content_type: &str,
    schema: &str,
    from_format: Option<InputFormat>,
) -> OperationSource {
    OperationSource {
        uri: uri.to_owned(),
        bytes: bytes.to_vec(),
        from_format,
        identity: FormatIdentity {
            content_type: Some(content_type.to_owned()),
            schema: Some(schema.to_owned()),
            ..FormatIdentity::default()
        },
        root_scope: ScopeConfig::default(),
    }
}
