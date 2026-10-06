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

#[test]
fn wrapping_controls_use_captured_names_and_keep_original_native_slots() {
    use cem_ml::schema::scope_controls::validate_schema_body_controls;
    let captured = capture("@ns c = https://cem.dev/ns/core/1\n{c:schema @select=library | {child}} {schema @select={#library} |} {c:schema @src=./external | {child}} {c:schema @select=library}\n@ns c = urn:foreign\n{c:schema @select=ordinary | {child}}");
    let wrappers: Vec<_> = captured
        .document()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => {
                SchemaDeclarationNode::new(captured.document().clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    let validate = |node: SchemaDeclarationNode| {
        validate_schema_body_controls(
            node.clone(),
            |source| {
                captured
                    .expanded_name(source.document(), source.node_id())
                    .cloned()
            },
            captured.schema_element_form(node.document(), node.node_id()),
        )
    };
    let literal = validate(wrappers[0].clone());
    assert!(
        matches!(&literal.control().unwrap().source, SchemaHostSource::LiteralSelector(value) if value == "library")
    );
    let native = validate(wrappers[1].clone());
    let SchemaHostSource::NativeSelector(payload) = &native.control().unwrap().source else {
        panic!("native selector")
    };
    assert!(matches!(
        payload.node(),
        CemAstNode::Reference { targets: None, .. }
    ));
    assert!(Arc::ptr_eq(payload.document(), captured.document()));
    assert!(
        matches!(&validate(wrappers[2].clone()).control().unwrap().source, SchemaHostSource::Uri(value) if value == "./external")
    );
    // No-body forms need following-region scheduling, not a body override.
    assert!(!validate(wrappers[3].clone()).has_override());
    assert!(!validate(wrappers[4].clone()).has_override());
    for wrapper in &wrappers[..3] {
        let contract = validate(wrapper.clone());
        assert!(contract.issue().is_none());
        assert_eq!(contract.attributes().len(), 1);
        assert!(Arc::ptr_eq(contract.host().document(), captured.document()));
    }
}

#[test]
fn wrapping_control_conflicts_and_missing_metadata_never_create_exemptions() {
    use cem_ml::schema::scope_controls::validate_schema_body_controls;
    let captured = capture("{schema @src=uri @select=library | {child}} {schema @select | {child}} {schema @select=library @schema-select=other | {child}} {schema @name=declaration | {child}}");
    let wrappers: Vec<_> = captured
        .document()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => {
                SchemaDeclarationNode::new(captured.document().clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    for (id, issue, count) in [
        (0, SchemaHostControlIssue::ConflictingSources, 2),
        (1, SchemaHostControlIssue::InvalidValue, 1),
        (2, SchemaHostControlIssue::ConflictingSources, 2),
    ] {
        let contract = validate_schema_body_controls(
            wrappers[id].clone(),
            |source| {
                captured
                    .expanded_name(source.document(), source.node_id())
                    .cloned()
            },
            captured.schema_element_form(wrappers[id].document(), wrappers[id].node_id()),
        );
        assert_eq!(contract.issue().unwrap().issue, issue);
        assert_eq!(contract.attributes().len(), count);
        assert_eq!(contract.diagnostics().len(), 1);
    }
    let missing = validate_schema_body_controls(wrappers[0].clone(), |_| None, None);
    assert_eq!(
        missing.issue().unwrap().issue,
        SchemaHostControlIssue::NamesNotReady
    );
    assert!(missing.attributes().is_empty());
    assert!(missing.diagnostics().is_empty());
    assert!(!validate_schema_body_controls(
        wrappers[3].clone(),
        |source| captured
            .expanded_name(source.document(), source.node_id())
            .cloned(),
        captured.schema_element_form(wrappers[3].document(), wrappers[3].node_id())
    )
    .has_override());
}

#[test]
fn imported_empty_wrapper_and_sibling_forms_require_original_event_metadata() {
    use cem_ml::{
        import::import_xml_ast_with_lexical_scopes,
        schema::{machine::SchemaElementForm, scope_controls::validate_schema_body_controls},
        validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest},
    };
    let text = "<root xmlns:c='https://cem.dev/ns/core/1'><c:schema select='library'></c:schema><c:schema select='library'/><c:schema xmlns:c='urn:foreign' select='ordinary'></c:schema></root>";
    let (document, diagnostics) = xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
        bytes: text.as_bytes(),
        source_uri: "forms.xml",
        content_type: Some("application/xml"),
    });
    assert!(diagnostics.is_empty());
    let imported =
        import_xml_ast_with_lexical_scopes(&document.unwrap(), CompiledSchema::cem_core()).unwrap();
    let captured = imported.captured;
    let wrappers: Vec<_> = captured
        .document()
        .nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "schema" => {
                SchemaDeclarationNode::new(captured.document().clone(), *node_id)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        captured.schema_element_form(wrappers[0].document(), wrappers[0].node_id()),
        Some(SchemaElementForm::Wrapping)
    );
    assert_eq!(
        captured.schema_element_form(wrappers[1].document(), wrappers[1].node_id()),
        Some(SchemaElementForm::Following)
    );
    assert_eq!(
        captured.schema_element_form(wrappers[2].document(), wrappers[2].node_id()),
        None
    );
    for node in &wrappers {
        assert!(matches!(node.node(),CemAstNode::Element{children,..} if children.is_empty()));
    }
    let name = |node: &SchemaDeclarationNode| {
        captured
            .expanded_name(node.document(), node.node_id())
            .cloned()
    };
    let wrapped =
        validate_schema_body_controls(wrappers[0].clone(), name, Some(SchemaElementForm::Wrapping));
    assert!(wrapped.control().is_some());
    let following = validate_schema_body_controls(
        wrappers[1].clone(),
        name,
        Some(SchemaElementForm::Following),
    );
    assert!(!following.has_override());
    let missing = validate_schema_body_controls(wrappers[0].clone(), name, None);
    assert_eq!(
        missing.issue().unwrap().issue,
        SchemaHostControlIssue::BodyFormNotReady
    );
    assert!(missing.attributes().is_empty());
    assert!(missing.diagnostics().is_empty());
    let foreign = capture("{schema @select=library |}").document().clone();
    assert!(captured
        .schema_element_form(&foreign, wrappers[0].node_id())
        .is_none());
    let no_element_name = validate_schema_body_controls(
        wrappers[0].clone(),
        |node| {
            if node.node_id() == wrappers[0].node_id() {
                None
            } else {
                name(node)
            }
        },
        Some(SchemaElementForm::Wrapping),
    );
    assert_eq!(
        no_element_name.issue().unwrap().issue,
        SchemaHostControlIssue::NamesNotReady
    );
    assert!(no_element_name.attributes().is_empty());
}
