use super::*;
use cem_ql::query_validator_capsule::profiles::transport;
use cem_ql::query_validator_capsule::profiles::{self, QueryResultProfileSource};
pub(super) fn custom_profile(shared: bool) -> Arc<profiles::VerifiedQueryResultProfile> {
    let a = "@ns s = https://cem.dev/ns/schema/1\n@ns d = urn:custom:diagnostic\n@default s\n{schema @namespace=https://cem.dev/ns/schema/1 | {value-contracts | {value-contract @name=datatype-validation-result | {value-field @name=accepted @kind=boolean @required=true} {value-field @name=diagnostics @type=d:diagnostic @required=true @cardinality=zero-or-more}}}}";
    let b = "{schema @namespace=urn:custom:diagnostic | {value-contracts | {value-contract @name=diagnostic | {value-field @name=code @kind=string @required=true} {value-field @name=severity @kind=string @required=true} {value-field @name=message @kind=string @required=true} {value-field @name=source @kind=node @cardinality=zero-or-one}}}}";
    let combined = format!("{a}\n{b}");
    let diagnostic = format!("@ns s = https://cem.dev/ns/schema/1\n@default s\n{b}");
    let mut sources = Vec::new();
    let mut original = None;
    for (i, namespace) in [CEM_SCHEMA_URI, "urn:custom:diagnostic"].iter().enumerate() {
        let text = if shared {
            combined.as_str()
        } else if i == 0 {
            a
        } else {
            diagnostic.as_str()
        };
        let capture = if shared && i == 1 {
            original.take().unwrap()
        } else {
            capture(text)
        };
        if shared && i == 0 {
            original = Some(capture.clone());
        }
        let schema = capture
            .document()
            .nodes
            .iter()
            .filter_map(|n| match n {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "schema" => Some(*node_id),
                _ => None,
            })
            .nth(if shared { i } else { 0 })
            .unwrap();
        let uri = if shared {
            "https://example.test/profile.cem"
        } else if i == 0 {
            "https://example.test/result.cem"
        } else {
            "https://example.test/diagnostic.cem"
        };
        let tree = RetainedCemTree::from_shared(
            capture.document().clone(),
            uri,
            text,
            Default::default(),
            None,
        )
        .unwrap();
        let contracts = ValueContractSource::new(
            SchemaDeclarationNode::new(capture.document().clone(), schema).unwrap(),
            *namespace,
            BTreeMap::from([("d".into(), "urn:custom:diagnostic".into())]),
        )
        .with_captured_names(capture.clone())
        .unwrap();
        sources.push(QueryResultProfileSource {
            tree,
            capture,
            contracts,
            identity: SourceIdentity::new(uri, SourceId(1), text.as_bytes()),
        });
    }
    profiles::build(
        sources,
        ContractName::new(CEM_SCHEMA_URI, "datatype-validation-result"),
        ContractName::new("urn:custom:diagnostic", "diagnostic"),
        &Default::default(),
    )
    .unwrap()
}
#[test]
fn custom_profiles_preserve_multiple_sources_and_shared_original_arenas() {
    for shared in [false, true] {
        let p = custom_profile(shared);
        let packet = transport::export(&p, &Default::default()).unwrap();
        let fresh = transport::verify(&packet.bytes, &packet.pin, &Default::default()).unwrap();
        assert_eq!(p.pin(), fresh.pin());
        assert_eq!(fresh.sources().len(), 2);
        assert_eq!(
            Arc::ptr_eq(
                fresh.sources()[0].capture.document(),
                fresh.sources()[1].capture.document()
            ),
            shared
        );
        assert_eq!(
            transport::export(&fresh, &Default::default())
                .unwrap()
                .bytes,
            packet.bytes
        );
    }
}
#[test]
fn complete_profile_sources_roundtrip_without_schema_catalog_or_old_arenas() {
    let p = super::execution::profile(super::execution::profile_text());
    let packet = transport::export(&p, &Default::default()).unwrap();
    let fresh = transport::verify(&packet.bytes, &packet.pin, &Default::default()).unwrap();
    assert_eq!(p.pin(), fresh.pin());
    assert!(!Arc::ptr_eq(
        p.sources()[0].tree.ast_owner(),
        fresh.sources()[0].tree.ast_owner()
    ));
    assert!(Arc::ptr_eq(
        fresh.sources()[0].tree.ast_owner(),
        fresh.sources()[0].capture.document()
    ));
    assert_eq!(
        transport::export(&fresh, &Default::default())
            .unwrap()
            .bytes,
        packet.bytes
    );
    let mut changed = packet.pin.clone();
    changed.profile.result.local.push_str("other");
    assert!(transport::verify(&packet.bytes, &changed, &Default::default()).is_err());
    for data in [
        &packet.bytes[..packet.bytes.len() - 1],
        &b"not a profile"[..],
    ] {
        assert!(transport::verify(data, &packet.pin, &Default::default()).is_err());
    }
    let signal = cem_ml::scheduler::AbortSignal::new();
    let control = OperationControl::new(signal.clone());
    signal.abort();
    assert!(transport::verify(&packet.bytes, &packet.pin, &control).is_err());
    assert!(transport::export(&p, &control).is_err());
    let oversized = vec![0; transport::MAX_BYTES + 1];
    assert!(transport::verify(&oversized, &packet.pin, &Default::default()).is_err());
    let mut appended = packet.bytes.clone();
    appended.push(0);
    let mut pin = packet.pin.clone();
    pin.hash = cem_ml::content_cache::ContentHash::from_blake3(&appended).header_value();
    assert!(transport::verify(&appended, &pin, &Default::default()).is_err());
}
