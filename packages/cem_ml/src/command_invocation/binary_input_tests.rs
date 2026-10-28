use super::*;
use crate::engine::FormatIdentity;
fn fixture(
    family: &str,
) -> (
    ParsedCommandInvocationV1,
    CommandInvocationEnvironmentV1,
    CommandUriMapV1<VirtualResourceV1>,
) {
    let uri = format!("studio://binary/source.{family}.cem-bin");
    let bytes = match family {
        "dom" => include_bytes!(
            "../../../../examples/projects/native-projections/projections/dispatch.dom.cem-bin"
        )
        .as_slice(),
        "ast" => include_bytes!(
            "../../../../examples/projects/native-projections/projections/dispatch.ast.cem-bin"
        )
        .as_slice(),
        _ => include_bytes!(
            "../../../../examples/projects/native-projections/projections/dispatch.events.cem-bin"
        )
        .as_slice(),
    };
    let mime = format!("application/vnd.cem.{family}+cem-bin");
    let schema = format!("https://cem.dev/ns/projection/{family}/1");
    let parsed = ParsedCommandInvocationV1 {
        schema_version: 1,
        common_version: crate::VERSION.into(),
        command_path: vec!["parse".into()],
        global_options: BTreeMap::new(),
        options: BTreeMap::from([
            ("format".into(), ParsedCommandValueV1::String("ast".into())),
            (
                "content_type".into(),
                ParsedCommandValueV1::String(mime.clone()),
            ),
            (
                "schema".into(),
                ParsedCommandValueV1::String(schema.clone()),
            ),
        ]),
        positionals: BTreeMap::from([("input".into(), ParsedCommandValueV1::String(uri.clone()))]),
        meta_action: None,
    };
    let env = CommandInvocationEnvironmentV1 {
        request_id: "binary-source".into(),
        project_id: "binary".into(),
        project_revision: 1,
        resource_revision: 1,
        cwd: "studio://binary/".into(),
        resolver_policy_stamp: "binary".into(),
        safety_policy_stamp: "binary".into(),
        budget_policy_stamp: "default".into(),
        stdout_is_terminal: false,
    };
    let resources = BTreeMap::from([(
        uri,
        VirtualResourceV1 {
            bytes: bytes.to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(mime),
                schema: Some(schema),
                ..Default::default()
            }),
        },
    )])
    .into();
    (parsed, env, resources)
}
#[test]
fn binary_input_invocations_bind_native_admission_to_actual_owned_bytes() {
    for family in ["dom", "ast", "events"] {
        let (p, e, r) = fixture(family);
        let expected = r.values().next().unwrap().bytes.clone();
        let CommandInvocationBuildResponseV1::Ready { invocation } =
            build_command_invocation_v1(p, e, r)
        else {
            panic!("binary source must be admitted")
        };
        let json = serde_json::to_value(invocation).unwrap();
        let facts = json["binaryInputs"]
            .as_array()
            .expect("native binary admission facts");
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0]["sha256"], sha256_hex(&expected));
        assert_eq!(facts[0]["byteLength"], expected.len());
    }
}
#[test]
fn binary_input_invocations_refuse_corruption_and_metadata_relabeling() {
    for family in ["dom", "ast", "events"] {
        for variant in 0..4 {
            let (p, e, mut r) = fixture(family);
            let resource = r.values_mut().next().unwrap();
            match variant {
                0 => {
                    resource.bytes.pop();
                }
                1 => resource.bytes.push(0),
                2 => {
                    resource.identity.as_mut().unwrap().content_type =
                        Some("application/vnd.cem.query-artifact+cem-bin".into())
                }
                _ => {
                    resource.identity.as_mut().unwrap().content_type =
                        Some("application/cem".into())
                }
            };
            assert!(
                matches!(
                    build_command_invocation_v1(p, e, r),
                    CommandInvocationBuildResponseV1::Error { .. }
                ),
                "{family} variant {variant}"
            );
        }
    }
}

#[test]
fn binary_input_execution_rechecks_payload_and_hash_after_invocation_preview() {
    use crate::capability::{capability_manifest, CapabilityRequest, RuntimeKind};
    use crate::command_operation::prepare_command_operation_v1;
    use crate::command_service::CommandServiceLimitsV1;
    use crate::engine::EngineContext;
    let capability = capability_manifest(CapabilityRequest {
        runtime: RuntimeKind::Native,
        target_identity: "test".into(),
        abi_identity: "rust:1".into(),
        debug_control_active: false,
    })
    .unwrap();
    for family in ["dom", "ast", "events"] {
        let (p, e, r) = fixture(family);
        let CommandInvocationBuildResponseV1::Ready { invocation } =
            build_command_invocation_v1(p, e, r)
        else {
            panic!("admitted")
        };
        let mut request = invocation.request;
        assert!(prepare_command_operation_v1(
            &request,
            CommandServiceLimitsV1::default(),
            &EngineContext::default(),
            &capability
        )
        .is_ok());
        let input = &request.run_plan.plan().unwrap().inputs[0];
        let uri = input
            .resolved_uri
            .as_deref()
            .unwrap_or(&input.declared_uri)
            .to_owned();
        request.resources.get_mut(&uri).unwrap().bytes.pop();
        assert!(prepare_command_operation_v1(
            &request,
            CommandServiceLimitsV1::default(),
            &EngineContext::default(),
            &capability
        )
        .is_err());
        let sha = sha256_hex(&request.resources[&uri].bytes);
        request.resource_versions.get_mut(&uri).unwrap().sha256 = sha;
        let error = prepare_command_operation_v1(
            &request,
            CommandServiceLimitsV1::default(),
            &EngineContext::default(),
            &capability,
        )
        .unwrap_err();
        assert_eq!(error.code(), "cem.command_service.binary_input");
    }
}
#[test]
fn binary_input_unknown_codecs_and_absent_snapshots_do_not_gain_admission() {
    let (p, e, mut r) = fixture("dom");
    r.clear();
    assert!(matches!(
        build_command_invocation_v1(p, e, r),
        CommandInvocationBuildResponseV1::NeedsResources { .. }
    ));
    let (mut p, e, mut r) = fixture("dom");
    p.options.insert(
        "content_type".into(),
        ParsedCommandValueV1::String("application/vnd.cem.query-artifact+cem-bin".into()),
    );
    p.options.remove("schema");
    r.values_mut().next().unwrap().identity = Some(FormatIdentity {
        content_type: Some("application/vnd.cem.query-artifact+cem-bin".into()),
        ..Default::default()
    });
    assert!(matches!(
        build_command_invocation_v1(p, e, r),
        CommandInvocationBuildResponseV1::Error { .. }
    ));
}
