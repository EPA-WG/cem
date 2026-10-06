use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::CemAstNode,
    schema::{
        declaration_references::SchemaDeclarationNode,
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        scope_controls::{decode_schema_host_control, SchemaHostControlIssue, SchemaHostSource},
        vocab::CompiledSchema,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use std::sync::Arc;

fn capture(text: &str) -> LexicallyScopedDocument {
    CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
            SourceId(1),
            text.as_bytes().to_vec(),
        ))),
    )
    .build_with_lexical_scopes()
}
fn hosts(captured: &LexicallyScopedDocument) -> Vec<SchemaDeclarationNode> {
    captured
        .document()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "host" => {
                SchemaDeclarationNode::new(captured.document().clone(), *node_id)
            }
            _ => None,
        })
        .collect()
}

#[test]
fn host_controls_use_original_handles_and_captured_names() {
    let captured = capture("@ns c = https://cem.dev/ns/core/1\n{host @c:schema-select=library} {host @schema-src=./library.cem}\n@ns c = urn:foreign\n{host @c:schema-select=ignored}");
    let hosts = hosts(&captured);
    let decode = |host| {
        decode_schema_host_control(host, |source| {
            captured
                .expanded_name(source.document(), source.node_id())
                .cloned()
        })
    };
    let selected = decode(hosts[0].clone()).unwrap().unwrap();
    assert!(
        matches!(&selected.source, SchemaHostSource::LiteralSelector(value) if value == "library")
    );
    assert!(Arc::ptr_eq(selected.host.document(), captured.document()));
    assert!(Arc::ptr_eq(
        selected.attribute.document(),
        captured.document()
    ));
    let uri = decode(hosts[1].clone()).unwrap().unwrap();
    assert!(matches!(&uri.source, SchemaHostSource::Uri(value) if value == "./library.cem"));
    assert!(decode(hosts[2].clone()).unwrap().is_none());
    assert!(
        matches!(decode_schema_host_control(hosts[0].clone(), |_| None), Err(error) if error.issue == SchemaHostControlIssue::NamesNotReady)
    );
}

#[test]
fn host_controls_retain_native_slots_and_reject_conflicts_or_missing_values() {
    let captured = capture("{host @schema-select={#library}} {host @schema-select={library}} {host @schema-src=uri @schema-select=library} {host @schema-select}");
    let hosts = hosts(&captured);
    let decode = |host| {
        decode_schema_host_control(host, |source| {
            captured
                .expanded_name(source.document(), source.node_id())
                .cloned()
        })
    };
    for host in &hosts[..2] {
        let selected = decode(host.clone()).unwrap().unwrap();
        let SchemaHostSource::NativeSelector(payload) = selected.source else {
            panic!("native slot")
        };
        assert!(Arc::ptr_eq(payload.document(), captured.document()));
        assert!(matches!(
            payload.node(),
            CemAstNode::Reference { targets: None, .. } | CemAstNode::Element { .. }
        ));
    }
    for (host, issue) in [
        (hosts[2].clone(), SchemaHostControlIssue::ConflictingSources),
        (hosts[3].clone(), SchemaHostControlIssue::InvalidValue),
    ] {
        let error = decode(host).unwrap_err();
        assert_eq!(error.issue, issue);
        assert!(Arc::ptr_eq(error.source.document(), captured.document()));
    }
}

#[test]
fn imported_host_controls_use_the_shared_typed_names_and_keep_attributes_literal() {
    let text = r#"<root xmlns:c="https://cem.dev/ns/core/1"><host c:schema-select="library"/><host xmlns:c="urn:foreign" c:schema-select="library"/></root>"#;
    let tree = cem_ml::import::import_data_bytes(
        text.as_bytes(),
        "application/xml",
        "cem",
        "controls.xml",
    )
    .unwrap();
    let hosts: Vec<_> = tree
        .ast_owner()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "host" => {
                SchemaDeclarationNode::new(tree.ast_owner().clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    let decode = |host| {
        decode_schema_host_control(host, |source| match source.node() {
            CemAstNode::Attribute { expanded_name, .. } => Some(expanded_name.clone()),
            _ => None,
        })
    };
    let control = decode(hosts[0].clone()).unwrap().unwrap();
    assert!(
        matches!(&control.source, SchemaHostSource::LiteralSelector(value) if value == "library")
    );
    assert!(
        matches!(control.attribute.node(), CemAstNode::Attribute { value_nodes, .. } if value_nodes.is_empty())
    );
    assert!(Arc::ptr_eq(control.host.document(), tree.ast_owner()));
    assert!(decode(hosts[1].clone()).unwrap().is_none());
}
