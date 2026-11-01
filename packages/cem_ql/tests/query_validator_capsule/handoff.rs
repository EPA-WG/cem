use super::*;
use cem_ml::runtime_admission::{generation::NativeGeneration, NativeAdmission};
use cem_ql::{
    query_validator_capsule::handoff as transport,
    schema_references::DeclarationScope,
    structural_receiver::{FreshQueryBinding, FreshQueryInvocation, FreshQuerySource},
};

struct Active {
    generation: Arc<NativeGeneration>,
    host: FreshStructuralHost,
    source: Arc<FreshQuerySource>,
    scope: DeclarationScope,
    binding: FreshQueryBinding,
    request: FreshQueryInvocation,
}
fn generation() -> Arc<NativeGeneration> {
    NativeGeneration::claim(Arc::new(NativeAdmission::default())).unwrap()
}
fn enrolled(g: &Arc<NativeGeneration>) -> FreshStructuralHost {
    FreshStructuralHost::new_in_generation(&registry_hash(), Default::default(), g).unwrap()
}
fn active() -> Active {
    active_with_control(Default::default())
}

#[test]
fn native_emitted_wasm_handoff_fixture_has_checked_source_and_independent_pins() {
    let old = active();
    let packet = old.host.capture_query_handoff(&old.binding).unwrap();
    let checked = transport::verify(&packet.bytes, &packet.pin, &Default::default()).unwrap();
    assert_eq!(
        checked.declaration().source().source_uri(),
        "https://example.test/schema.cem"
    );
    if let Some(path) = std::env::var_os("CEM_QUERY_HANDOFF_FIXTURE_DIR") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("source.cemq"), &packet.bytes).unwrap();
        // Explicit control/deployment metadata; the source AST stays binary.
        std::fs::write(
            path.join("pins.json"),
            serde_json::to_vec(&packet.pin).unwrap(),
        )
        .unwrap();
    }
}
fn active_with_control(control: OperationControl) -> Active {
    let generation = generation();
    let mut host =
        FreshStructuralHost::new_in_generation(&registry_hash(), control, &generation).unwrap();
    let d = verified(&declaration(
        r#"{accepted: str:length(value) > 2, diagnostics: {code: "query.note", severity: "warning", message: "original", source: datatype}}"#,
    ));
    let scope = register(&mut host, &d);
    let source = host.query_source(d.clone(), scope).unwrap();
    let binding = host
        .link_query_source(&source, datatype(&d), &functions(), adapter())
        .unwrap();
    let request = host.query_invocation(scope).unwrap();
    Active {
        generation,
        host,
        source,
        scope,
        binding,
        request,
    }
}

fn register_handoff(
    host: &mut FreshStructuralHost,
    handoff: &transport::VerifiedQueryHandoff,
) -> DeclarationScope {
    let d = handoff.declaration();
    host.register_captured_scope(
        d.source().clone(),
        d.capture().clone(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )
    .unwrap()
}

#[test]
fn handoff_reimports_original_source_and_requires_fresh_host_registry_and_request() {
    let old = active();
    let built = old.host.capture_query_handoff(&old.binding).unwrap();
    let restored = transport::verify(&built.bytes, &built.pin, &Default::default()).unwrap();
    let original = old.source.declaration();
    assert!(!Arc::ptr_eq(
        original.source().ast_owner(),
        restored.declaration().source().ast_owner()
    ));
    assert_eq!(
        original.source().source_uri(),
        restored.declaration().source().source_uri()
    );
    assert_eq!(old.generation.inventory().unwrap().live_roots, 4);
    let next = generation();
    let mut fresh = enrolled(&next);
    let scope = register_handoff(&mut fresh, &restored);
    assert!(fresh
        .link_query_handoff(
            &restored,
            scope,
            &SelectedFunctionRegistry::new([]).unwrap(),
            adapter()
        )
        .is_err());
    assert_eq!(next.inventory().unwrap().live_roots, 1);
    let linked = fresh
        .link_query_handoff(&restored, scope, &functions(), adapter())
        .unwrap();
    let request = fresh.query_invocation(scope).unwrap();
    assert_eq!(next.inventory().unwrap().live_roots, 4);
    assert!(fresh
        .validate_query(&old.request, &linked, &input("same"), Default::default())
        .is_err());
    assert!(fresh
        .validate_query(&request, &old.binding, &input("same"), Default::default())
        .is_err());
    let before = old
        .host
        .validate_query(
            &old.request,
            &old.binding,
            &input("same"),
            Default::default(),
        )
        .unwrap();
    let after = fresh
        .validate_query(&request, &linked, &input("same"), Default::default())
        .unwrap();
    assert_eq!(before.accepted, after.accepted);
    let a = &before.completed[0].result.diagnostics[0];
    let b = &after.completed[0].result.diagnostics[0];
    assert_eq!(
        (
            &a.code,
            &a.uri,
            &a.source_map,
            a.byte_offset,
            a.line,
            a.column
        ),
        (
            &b.code,
            &b.uri,
            &b.source_map,
            b.byte_offset,
            b.line,
            b.column
        )
    );
    assert!(Arc::ptr_eq(
        after.completed[0].datatype.document(),
        restored.declaration().source().ast_owner()
    ));
    let original_generation = old.generation.clone();
    let drain = original_generation
        .begin_drain(std::time::Duration::from_secs(5), Default::default())
        .unwrap();
    assert!(old.host.capture_query_handoff(&old.binding).is_err());
    assert!(drain.seal().is_err());
    drop(old);
    assert!(drain.seal().unwrap().is_current(&original_generation));
    assert_eq!(
        fresh
            .validate_query(&request, &linked, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}

#[test]
fn handoff_contains_source_and_selectors_without_evaluation_payload_or_context() {
    let old = active();
    let before = old.host.capture_query_handoff(&old.binding).unwrap();
    let private_value = "entered-password-not-source-80ff366c";
    old.host
        .validate_query(
            &old.request,
            &old.binding,
            &input(private_value),
            Default::default(),
        )
        .unwrap();
    let after = old.host.capture_query_handoff(&old.binding).unwrap();
    assert_eq!(before.bytes, after.bytes);
    assert!(!after
        .bytes
        .windows(private_value.len())
        .any(|s| s == private_value.as_bytes()));
    let second = old.host.query_invocation(old.scope).unwrap();
    second.retire();
    assert_eq!(
        after.bytes,
        old.host.capture_query_handoff(&old.binding).unwrap().bytes
    );
    // This source-only record grants no request/recovery authority. Credential
    // refusal and draining all requests remain the enclosing checkpoint policy.
}

#[test]
fn capture_refuses_foreign_retired_sources_bindings_and_changed_contexts() {
    let old = active();
    let other = active();
    assert!(other.host.capture_query_handoff(&old.binding).is_err());
    old.source.retire();
    assert!(old.host.capture_query_handoff(&old.binding).is_err());
    let mut old = active();
    assert!(old.host.set_context(old.scope, Some(Default::default())));
    assert!(old.host.capture_query_handoff(&old.binding).is_err());
    let old = active();
    old.binding.retire();
    assert!(old.host.capture_query_handoff(&old.binding).is_err());
    let old = active();
    old.host.close();
    assert!(old.host.capture_query_handoff(&old.binding).is_err());
    let old = active();
    drop(old.source);
    assert!(old.host.capture_query_handoff(&old.binding).is_err());
}

#[test]
fn matching_bytes_and_scopes_cannot_replace_the_actual_restored_source_owner() {
    let old = active();
    let built = old.host.capture_query_handoff(&old.binding).unwrap();
    let a = transport::verify(&built.bytes, &built.pin, &Default::default()).unwrap();
    let b = transport::verify(&built.bytes, &built.pin, &Default::default()).unwrap();
    let g = generation();
    let mut h = enrolled(&g);
    let scope = register_handoff(&mut h, &a);
    assert!(h
        .link_query_handoff(&b, scope, &functions(), adapter())
        .is_err());
    let binding = h
        .link_query_handoff(&a, scope, &functions(), adapter())
        .unwrap();
    let request = h.query_invocation(scope).unwrap();
    assert!(h.set_context(scope, None));
    assert!(h
        .link_query_handoff(&a, scope, &functions(), adapter())
        .is_err());
    assert!(h
        .validate_query(&request, &binding, &input("same"), Default::default())
        .is_err());
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Metadata {
    abi: String,
    ml_version: String,
    ql_version: String,
    datatype: u32,
    result: ContractName,
    diagnostic: ContractName,
}
fn split(bytes: &[u8]) -> (Metadata, Vec<u8>) {
    let at = transport::MAGIC.len();
    let length = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
    (
        rmp_serde::from_slice(&bytes[at + 4..at + 4 + length]).unwrap(),
        bytes[at + 4 + length..].to_vec(),
    )
}
fn assemble(meta: &Metadata, source: &[u8]) -> Vec<u8> {
    let meta = rmp_serde::to_vec_named(meta).unwrap();
    [
        transport::MAGIC,
        &(meta.len() as u32).to_le_bytes(),
        &meta,
        source,
    ]
    .concat()
}
fn repin(bytes: &[u8], original: &transport::QueryHandoffPin) -> transport::QueryHandoffPin {
    let mut pin = original.clone();
    pin.hash = cem_ml::content_cache::ContentHash::from_blake3(bytes).header_value();
    pin
}

#[test]
fn handoff_refuses_bad_versions_selectors_pins_and_incompatible_result_adapters() {
    let old = active();
    let built = old.host.capture_query_handoff(&old.binding).unwrap();
    for mutation in 0..6 {
        let (mut meta, mut source) = split(&built.bytes);
        match mutation {
            0 => meta.abi.push_str("/other"),
            1 => meta.ml_version.push_str("/other"),
            2 => meta.ql_version.push_str("/other"),
            3 => meta.datatype = 0,
            4 => meta.result.local.push_str("-other"),
            _ => source.push(0),
        }
        let bytes = assemble(&meta, &source);
        assert!(
            transport::verify(&bytes, &repin(&bytes, &built.pin), &Default::default()).is_err(),
            "mutation {mutation}"
        );
    }
    let mut pin = built.pin.clone();
    pin.declaration.program.source.uri.push_str("/other");
    assert!(transport::verify(&built.bytes, &pin, &Default::default()).is_err());
    let (mut meta, source) = split(&built.bytes);
    meta.diagnostic.local.push_str("-other");
    let bytes = assemble(&meta, &source);
    let restored =
        transport::verify(&bytes, &repin(&bytes, &built.pin), &Default::default()).unwrap();
    let g = generation();
    let mut h = enrolled(&g);
    let scope = register(&mut h, restored.declaration());
    assert!(h
        .link_query_handoff(&restored, scope, &functions(), adapter())
        .is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 1);
}

#[test]
fn bounded_framing_refuses_truncation_trailing_data_and_oversized_metadata_before_decode() {
    let old = active();
    let built = old.host.capture_query_handoff(&old.binding).unwrap();
    let at = transport::MAGIC.len();
    let mut packets = vec![
        vec![],
        built.bytes[..at].to_vec(),
        built.bytes[..at + 3].to_vec(),
        built.bytes[..built.bytes.len() - 1].to_vec(),
        vec![0; transport::MAX_BYTES + 1],
    ];
    let mut trailing = built.bytes.clone();
    trailing.push(0);
    packets.push(trailing);
    for length in [0, 4097, u32::MAX] {
        let mut bytes = built.bytes.clone();
        bytes[at..at + 4].copy_from_slice(&length.to_le_bytes());
        packets.push(bytes);
    }
    for body in [vec![0xdd, 255, 255, 255, 255], vec![0x91; 70], vec![0xc1]] {
        packets.push(
            [
                transport::MAGIC,
                &(body.len() as u32).to_le_bytes(),
                &body,
                b"CEMQD1\n",
            ]
            .concat(),
        );
    }
    let (meta, source) = split(&built.bytes);
    // Serde accepts positional struct arrays too; the wire requires a canonical
    // named map, and refuses even well-formed metadata with trailing values.
    let named = rmp_serde::to_vec_named(&meta).unwrap();
    for body in [
        rmp_serde::to_vec(&meta).unwrap(),
        [named.as_slice(), &[0]].concat(),
    ] {
        packets.push(
            [
                transport::MAGIC,
                &(body.len() as u32).to_le_bytes(),
                &body,
                &source,
            ]
            .concat(),
        );
    }
    for bytes in packets {
        assert!(
            transport::verify(&bytes, &repin(&bytes, &built.pin), &Default::default()).is_err()
        );
    }
    assert_eq!(old.generation.inventory().unwrap().live_roots, 4);
}

#[test]
fn cancelled_verification_and_failed_owner_allocation_cannot_install_or_retire_bindings() {
    let signal = cem_ml::scheduler::AbortSignal::new();
    let control = OperationControl::new(signal.clone());
    let old = active_with_control(control.clone());
    let built = old.host.capture_query_handoff(&old.binding).unwrap();
    signal.abort();
    assert!(old.host.capture_query_handoff(&old.binding).is_err());
    assert!(transport::verify(&built.bytes, &built.pin, &control).is_err());
    let restored = transport::verify(&built.bytes, &built.pin, &Default::default()).unwrap();
    let g = generation();
    let mut h = enrolled(&g);
    let scope = register_handoff(&mut h, &restored);
    let held = (0..254)
        .map(|_| {
            g.resource(cem_ml::runtime_admission::NativeRootKind::Evidence)
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(h
        .link_query_handoff(&restored, scope, &functions(), adapter())
        .is_err());
    assert_eq!(g.inventory().unwrap().live_roots, 255);
    assert_eq!(g.inventory().unwrap().active_entries, 0);
    drop(held);
    let linked = h
        .link_query_handoff(&restored, scope, &functions(), adapter())
        .unwrap();
    let request = h.query_invocation(scope).unwrap();
    assert_eq!(
        h.validate_query(&request, &linked, &input("same"), Default::default())
            .unwrap()
            .accepted,
        Some(true)
    );
}
