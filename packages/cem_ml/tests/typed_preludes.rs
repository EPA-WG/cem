use cem_ml::{
    ast::reload::{ReferenceReloadBundle, ReloadError, ReloadLimits},
    events::{
        cem::{collect_events, CemEventNormalizer},
        NormalizedEvent, ScalarValue,
    },
    parser::{document::CemDocument, CemAstNode},
    schema::{
        machine::{CemSchemaMachine, LexicallyScopedDocument},
        namespace_references::PendingNamespaceValue,
        scoping::SchemaSource,
        vocab::CompiledSchema,
    },
    source::{ByteRange, BytesSource, SourceId},
    tokenizer::{
        cem::{CemTokenizer, TypedPreludePreview},
        SchemaTokenKind, SchemaTokenizer,
    },
};

fn tokenizer(source: &str) -> CemTokenizer {
    CemTokenizer::from_source_with_typed_prelude_preview(
        BytesSource::new(SourceId(17), source.as_bytes().to_vec()),
        TypedPreludePreview::default(),
    )
}
fn parse(source: &str) -> LexicallyScopedDocument {
    CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(tokenizer(source)),
    )
    .build_with_lexical_scopes()
}
fn elements(doc: &CemDocument, local: &str) -> Vec<u32> {
    doc.nodes
        .iter()
        .filter_map(|node| match node {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => Some(*node_id),
            _ => None,
        })
        .collect()
}
fn slice(source: &str, range: ByteRange) -> &str {
    &source[range.start as usize..range.end() as usize]
}

#[test]
fn native_consumer_views_require_original_owner_and_keep_slot_kind_and_role() {
    use cem_ml::schema::{
        declaration_references::SchemaDeclarationNode,
        namespace_references::decode_native_namespace_property,
        prelude_values::{decode_native_prelude_value, NativePreludeValueError},
        scope_controls::{
            validate_typed_schema_prelude, SchemaHostSource, SchemaScopeControlExtent,
        },
    };
    use cem_ml::tokenizer::cem::{TypedPreludeKind, TypedPreludeRole};
    let text = "@schema select={#library}\n@ns ui = {$ #library}\n@default {#library}";
    let capture = parse(text);
    let foreign = parse(text);
    for (name, role, kind) in [
        (
            "@schema",
            TypedPreludeRole::SchemaSelector,
            TypedPreludeKind::Reference,
        ),
        (
            "@ns",
            TypedPreludeRole::Namespace,
            TypedPreludeKind::Expression,
        ),
        (
            "@default",
            TypedPreludeRole::DefaultNamespace,
            TypedPreludeKind::Reference,
        ),
    ] {
        let id = elements(capture.document(), name)[0];
        let declaration = SchemaDeclarationNode::new(capture.document().clone(), id).unwrap();
        let view = decode_native_prelude_value(declaration.clone(), &capture)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(view.role(), role);
        assert_eq!(view.kind(), kind);
        assert_eq!(view.declaration().node_id(), id);
        assert_eq!(
            view.value().node_id(),
            capture.typed_prelude(capture.document(), id).unwrap().value
        );
        assert_eq!(
            decode_native_prelude_value(declaration.clone(), &foreign).unwrap_err(),
            NativePreludeValueError::OwnerMismatch
        );
        if role == TypedPreludeRole::SchemaSelector {
            let contract = validate_typed_schema_prelude(declaration, &capture);
            assert_eq!(contract.extent(), SchemaScopeControlExtent::Following);
            assert!(contract.attributes().is_empty());
            assert_eq!(contract.control().unwrap().attribute.node_id(), id);
            assert!(matches!(
                contract.control().unwrap().source,
                SchemaHostSource::NativeSelector(_)
            ));
        } else {
            let property = decode_native_namespace_property(declaration, &capture).unwrap();
            assert_eq!(
                property.prefix,
                if role == TypedPreludeRole::Namespace {
                    "ui"
                } else {
                    ""
                }
            );
            assert_eq!(property.value.node_id(), view.value().node_id());
        }
    }
}

#[test]
fn native_consumer_views_reject_quoted_and_malformed_preludes_without_literal_fallback() {
    use cem_ml::schema::{
        declaration_references::SchemaDeclarationNode, prelude_values::decode_native_prelude_value,
        scope_controls::validate_typed_schema_prelude,
    };
    for text in [
        "@schema select=\"{#library}\"",
        "@schema select={#library} trailing",
        "@ns ui = {#library} trailing",
    ] {
        let capture = parse(text);
        let declaration = SchemaDeclarationNode::new(capture.document().clone(), 1).unwrap();
        assert!(decode_native_prelude_value(declaration.clone(), &capture).is_err());
        let contract = validate_typed_schema_prelude(declaration, &capture);
        assert!(contract.has_override());
        assert!(contract.control().is_none());
    }
}

#[test]
fn primary_tokens_events_and_original_edges_retain_roles_and_exact_spans() {
    let source =
        "@schema select={#schemaChoice}\n@ns ui = {$ #namespaceChoice}\n@default {#defaultChoice}";
    let mut tokens = tokenizer(source);
    let mut count = 0;
    while let Some(token) = tokens.next_token() {
        if let SchemaTokenKind::TypedDirective { value, .. } = token.kind {
            assert!(value.error.is_none());
            assert!(slice(source, value.value_range).starts_with('{'));
            count += 1;
        }
    }
    assert_eq!(count, 3);
    let events = collect_events(CemEventNormalizer::new(tokenizer(source)));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                NormalizedEvent::Value {
                    value: ScalarValue::TypedPrelude(_),
                    ..
                }
            ))
            .count(),
        3
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, NormalizedEvent::Name { .. })));
    let capture = parse(source);
    let owner = capture.document();
    let foreign = parse(source).document().clone();
    for (local, text, reference) in [
        ("@schema", "#schemaChoice", true),
        ("@ns", "#namespaceChoice", false),
        ("@default", "#defaultChoice", true),
    ] {
        let id = elements(owner, local)[0];
        let slot = capture.typed_prelude(owner, id).unwrap();
        assert_eq!(slot.directive, id);
        assert_eq!(slice(source, slot.syntax.payload_range), text);
        assert_eq!(slot.required_version.minor, 1);
        assert!(capture.typed_prelude(&foreign, id).is_none());
        let CemAstNode::Element {
            attributes,
            children,
            ..
        } = owner.get(id).unwrap()
        else {
            panic!()
        };
        assert!(attributes.is_empty());
        assert_eq!(children, &[slot.value]);
        if reference {
            assert!(matches!(owner.get(slot.value), Some(CemAstNode::Reference {
                expression, context, targets: None, .. }) if expression == text && *context == id));
        } else {
            assert!(
                matches!(owner.get(slot.value), Some(CemAstNode::Element {expanded_name, ..})
                if expanded_name.local_name == "$")
            );
        }
        assert!(capture.snapshot(owner, slot.value).is_some());
    }
    assert!(ReferenceReloadBundle::export(&capture, vec![
        cem_ml::ast::reload::ReloadSource::new(SourceId(0), "typed.cem", source.as_bytes(), true),
        cem_ml::ast::reload::ReloadSource::new(SourceId(17), "typed.cem", source.as_bytes(), true),
    ], ReloadLimits::default()).is_ok());
}

#[test]
fn explicit_preview_preserves_legacy_and_quoted_literal_paths() {
    for header in ["", "@doc cem-ml 1\n", "@doc cem-ml 1.0\n"] {
        let source = format!("{header}@ns ui = {{#choice}}\n@schema select={{#choice}}\n");
        let mut legacy =
            CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
        while let Some(token) = legacy.next_token() {
            assert!(!matches!(
                token.kind,
                SchemaTokenKind::TypedDirective { .. }
            ));
        }
        if !header.is_empty() {
            let capture = parse(&source);
            assert!(!capture.has_typed_preludes());
        }
    }
    for header in [
        "@doc cem-ml 1.1\n",
        "@doc cem-ml 1.1.0\n",
        "@doc cem-ml 1.1.0+test\n",
    ] {
        assert!(parse(&format!("{header}@default {{#choice}}\n")).has_typed_preludes());
    }
    for header in [
        "@doc cem-ml 1.2\n",
        "@doc other 1.1\n",
        "@doc cem-ml 1.1.0-rc.1\n",
    ] {
        let capture = parse(&format!("{header}@default {{#choice}}\n"));
        assert!(!capture.has_typed_preludes());
        assert!(capture
            .diagnostics()
            .iter()
            .any(|d| d.code.starts_with("cem.doc.")));
    }
    let capture =
        parse("@ns ui = \"{#choice}\"\n@default '{#choice}'\n@schema select=\"{#choice}\"\n");
    assert!(!capture.has_typed_preludes());
    assert!(!capture
        .document()
        .nodes
        .iter()
        .any(|n| matches!(n, CemAstNode::Reference { .. })));
    assert_eq!(cem_ml::parser::format::SUPPORTED_VERSION.minor, 1);
}

#[test]
fn pending_declarations_mask_previous_bindings_and_keep_original_aliases() {
    let capture = parse("@ns ui = urn:old\n@schema src=outer\n@ns ui = {#choice}\n@default ui\n@schema select={#schema}\n{ui:panel | {#after}}\n@ns ui = urn:new\n{item}\n");
    let owner = capture.document();
    let ns = elements(owner, "@ns")[1];
    let slot = capture.typed_prelude(owner, ns).unwrap();
    assert_eq!(
        slot.preceding
            .namespaces
            .binding("ui")
            .unwrap()
            .namespace_uri,
        "urn:old"
    );
    assert!(matches!(
        capture
            .pending_namespace_declaration(owner, ns)
            .unwrap()
            .value,
        PendingNamespaceValue::Native
    ));
    let alias = elements(owner, "@default")[0];
    assert!(
        matches!(capture.pending_namespace_declaration(owner, alias).unwrap().value,
        PendingNamespaceValue::Alias(id) if id == ns)
    );
    let panel = elements(owner, "panel")[0];
    assert_eq!(
        capture
            .pending_namespace_name(owner, panel)
            .unwrap()
            .declaration,
        ns
    );
    assert!(capture.expanded_name(owner, panel).is_none());
    let item = elements(owner, "item")[0];
    assert_eq!(
        capture
            .pending_namespace_name(owner, item)
            .unwrap()
            .declaration,
        alias
    );
    let schema = elements(owner, "@schema")[1];
    let slot = capture.typed_prelude(owner, schema).unwrap();
    assert_eq!(
        slot.preceding.schema.active,
        SchemaSource::Uri("outer".into())
    );
    let after = owner
        .nodes
        .iter()
        .find_map(|node| match node {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression == "#after" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    assert!(
        matches!(capture.snapshot(owner, after).unwrap().schema.active,
        SchemaSource::PendingPrelude {directive: Some(id), ..} if id == schema)
    );
    assert!(capture
        .snapshot(owner, after)
        .unwrap()
        .namespaces
        .binding("ui")
        .is_none());
}

#[test]
fn block_recognition_and_restoration_follow_physical_lines() {
    for newline in ["\n", "\r\n", "\r"] {
        let source = "@ns ui = urn:outer\n{section |\n /* comment */\n @ns ui = {#choice}\n @default {$ choice}\n @schema select={#schema}\n {ui:inside}\n}\n{ui:outside}\n".replace('\n', newline);
        let capture = parse(&source);
        let owner = capture.document();
        assert_eq!(elements(owner, "@schema").len(), 1);
        let inner = elements(owner, "inside")[0];
        let outer = elements(owner, "outside")[0];
        assert!(capture.pending_namespace_name(owner, inner).is_some());
        assert_eq!(
            capture.expanded_name(owner, outer).unwrap().namespace_uri,
            "urn:outer"
        );
        for content in [
            "{section | @ns ui = {#x}\n}",
            "{section | text\n@ns ui = {#x}\n}",
            "{section |\n\\@ns ui = {#x}\n}",
            "{section |\n@unknown x\n@ns ui = {#x}\n}",
        ] {
            assert!(
                !parse(&content.replace('\n', newline)).has_typed_preludes(),
                "{content}"
            );
        }
    }
}

#[test]
fn nested_delimiters_strings_and_comments_do_not_terminate_the_slot() {
    for expression in [
        "map { 'key': '}'}",
        "concat('it''s }', \"{\")",
        "(: } (: nested :) :) #x",
        "/* } */ #x",
    ] {
        let source = format!("@default {{$ {expression}}}\n{{next}}");
        let capture = parse(&source);
        let owner = capture.document();
        let slot = capture
            .typed_prelude(owner, elements(owner, "@default")[0])
            .unwrap();
        assert!(slot.syntax.error.is_none(), "{expression}");
        assert_eq!(slot.syntax.expression, expression);
        assert_eq!(elements(owner, "next").len(), 1);
    }
}

#[test]
fn malformed_native_values_remain_invalid_and_recover_at_their_line() {
    for body in [
        "@default {$ }",
        "@default {#}",
        "@default {$ x} junk",
        "@default literal {#x}",
        "@default {#a}{#b}",
        "@schema src={#x}",
        "@schema select={#x} select={#y}",
        "@schema src=uri select={#x}",
        "@schema select={#x} src=uri",
        "@default {x}",
        "@ns ui = {#x",
        "@default {$ 'unterminated",
        "@default {$ /* unterminated",
        "@default {$ // close }",
    ] {
        let source = format!("{{section |\n{body}\n}}\n{{next}}");
        let capture = parse(&source);
        assert!(
            capture
                .document()
                .nodes
                .iter()
                .any(|node| matches!(node, CemAstNode::Error { .. })),
            "{body}"
        );
        assert_eq!(elements(capture.document(), "next").len(), 1, "{body}");
        assert!(capture.has_typed_preludes(), "{body}");
        assert!(
            capture
                .diagnostics()
                .iter()
                .any(|d| d.code == "cem.tokenizer.invalid_typed_prelude" && d.source_map.is_some()),
            "{body}"
        );
    }
    let capture = parse("@schema src=old\n@schema select={#}\n{#after}");
    let after = capture
        .document()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression == "#after" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    assert!(matches!(
        capture
            .snapshot(capture.document(), after)
            .unwrap()
            .schema
            .active,
        SchemaSource::PendingPrelude { .. }
    ));
}

#[test]
fn multiline_errors_stop_before_the_next_directive_and_restore_parent_scope() {
    for newline in ["\n", "\r\n", "\r"] {
        for value in [
            "{#choice",
            "{$ 'text",
            "{$ 'text\\",
            "{$ (: comment",
            "{$ map {",
        ] {
            let source = format!("@schema src=outer\n{{section |\n@schema select={value}\n@ns ui = urn:inner\n{{ui:child}}\n}}\n{{#outside}}").replace('\n', newline);
            let capture = parse(&source);
            let owner = capture.document();
            let child = elements(owner, "child")[0];
            assert_eq!(
                capture.expanded_name(owner, child).unwrap().namespace_uri,
                "urn:inner"
            );
            let outside = owner
                .nodes
                .iter()
                .find_map(|n| match n {
                    CemAstNode::Reference {
                        node_id,
                        expression,
                        ..
                    } if expression == "#outside" => Some(*node_id),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                capture.snapshot(owner, outside).unwrap().schema.active,
                SchemaSource::Uri("outer".into())
            );
        }
    }
    let capture = parse("{section |\n@default {#x} }\n{next}");
    let owner = capture.document();
    let next = elements(owner, "next")[0];
    assert!(
        matches!(owner.root().unwrap(), CemAstNode::Document {root_children, ..} if root_children.contains(&next))
    );
}

#[test]
fn unicode_spans_and_embedding_frames_belong_to_the_original_source() {
    use cem_ml::source_map::{FrameSpan, TransformKind};
    use cem_ml::tokenizer::cem::TypedPreludeRole;
    let source = "@ns éclair = urn:before\n@ns éclair = {$ | #choix}\n@default {#défaut}";
    let capture = parse(source);
    let owner = capture.document();
    let ns = elements(owner, "@ns")[1];
    let slot = capture.typed_prelude(owner, ns).unwrap();
    assert_eq!(slot.syntax.role, TypedPreludeRole::Namespace);
    assert_eq!(slice(source, slot.syntax.prefix_range.unwrap()), "éclair");
    assert_eq!(slot.syntax.expression, "#choix");
    assert_eq!(slice(source, slot.syntax.payload_range), "#choix");
    let CemAstNode::Element { source: frames, .. } = owner.get(slot.value).unwrap() else {
        panic!()
    };
    assert!(frames
        .frames
        .iter()
        .all(|frame| frame.source_id == SourceId(17)));
    assert!(frames.frames.iter().any(|frame| matches!(
        frame.transform,
        TransformKind::ExpressionEmbedding { .. }
    ) && matches!(frame.span, FrameSpan::Single(span) if span == slot.syntax.payload_range)));
    let default = capture
        .typed_prelude(owner, elements(owner, "@default")[0])
        .unwrap();
    assert_eq!(slice(source, default.syntax.payload_range), "#défaut");
    assert_eq!(default.syntax.role, TypedPreludeRole::DefaultNamespace);
}

#[test]
fn invalid_namespace_slots_shadow_old_values_and_unknown_sites_are_rejected() {
    for declaration in ["@ns ui = {#}", "@ns ui = {$ x} trailing"] {
        let capture = parse(&format!("@ns ui = urn:old\n{declaration}\n{{ui:child}}"));
        let owner = capture.document();
        assert!(capture
            .pending_namespace_name(owner, elements(owner, "child")[0])
            .is_some());
        assert!(capture
            .namespace_binding(owner, elements(owner, "@ns")[1])
            .is_none());
    }
    let malformed = parse("@default{#x}\n{next}");
    assert!(malformed
        .document()
        .nodes
        .iter()
        .any(|node| matches!(node, CemAstNode::Error { .. })));
    assert!(!parse("{section |\n@default{#x}\n}").has_typed_preludes());
    let mut tokens = tokenizer("@other field={#choice}\n{next}");
    assert!(std::iter::from_fn(|| tokens.next_token()).any(|token|
        matches!(token.kind, SchemaTokenKind::Error {code} if code == "cem.prelude.unsupported_site")));
}

#[test]
fn cancellation_does_not_admit_partial_typed_slots() {
    use cem_ml::{operation_control::OperationControl, scheduler::AbortSignal};
    let control = OperationControl::new(AbortSignal::new());
    control.cancel_root(Some("fixture".into()), None).unwrap();
    let mut tokens = CemTokenizer::from_source_with_control_and_typed_prelude_preview(
        BytesSource::new(SourceId(17), b"@default {#choice}".to_vec()),
        control.clone(),
        control.root_scope(),
        TypedPreludePreview::default(),
    );
    assert!(tokens.next_token().is_none());
}

#[test]
fn public_document_admission_requires_complete_reload_metadata() {
    use cem_ml::{ast::reload::ReloadSource, parser::builder::CemAstBuilder};
    let document = CemAstBuilder::new(CemEventNormalizer::new(tokenizer(
        "@doc cem-ml 1.1\n@default {#choice}",
    )))
    .top_level(true)
    .build();
    assert_eq!(document.format_identity.unwrap().format_version, cem_ml::schema::ir::SemVer::new(1, 1, 0));
    assert!(!document.diagnostics.iter().any(|d| d.code == "cem.doc.version_unsupported"));

    let source = "{#x}";
    let capture = parse(source);
    let mut bundle = ReferenceReloadBundle::export(
        &capture,
        vec![ReloadSource::new(
            SourceId(17),
            "preview.cem",
            source.as_bytes(),
            true,
        )],
        ReloadLimits::default(),
    )
    .unwrap();
    let snapshot = bundle
        .lexical
        .as_mut()
        .unwrap()
        .occurrences
        .values_mut()
        .next()
        .unwrap();
    snapshot.schema.active = SchemaSource::PendingPrelude {
        directive: None,
        value_range: ByteRange::new(0, 4),
    };
    assert!(matches!(
        bundle.reload(ReloadLimits::default()),
        Err(ReloadError::InvalidMetadata)
    ));
}

#[test]
fn streaming_validation_keeps_pending_default_aliases_unavailable() {
    let source = "@ns ui = urn:old\n@default ui\n@ns ui = {#choice}\n@default ui\n{item}";
    let mut last_default = None;
    CemSchemaMachine::new(
        CompiledSchema::cem_core(),
        CemEventNormalizer::new(tokenizer(source)),
    )
    .run_with_observer(|machine| {
        last_default = machine
            .current_ns_context()
            .binding("")
            .map(|binding| binding.namespace_uri.clone());
        assert_ne!(last_default.as_deref(), Some("ui"));
    });
    assert!(last_default.is_none());
}

#[test]
fn preview_scanning_has_finite_value_and_nesting_limits() {
    for source in ["@schema select={#x}", "@ns ui = {#x}", "@default {#x}"] {
        let tok = CemTokenizer::from_source_with_typed_prelude_preview(
            BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
            TypedPreludePreview {
                max_value_bytes: 4,
                max_nesting: 1,
            },
        );
        let capture =
            CemSchemaMachine::new(CompiledSchema::cem_core(), CemEventNormalizer::new(tok))
                .build_with_lexical_scopes();
        assert!(
            !capture
                .document()
                .nodes
                .iter()
                .any(|n| matches!(n, CemAstNode::Error { .. })),
            "{source}"
        );
    }

    for (source, limits) in [
        (
            "@default {$ abcdef}\n{next}",
            TypedPreludePreview {
                max_value_bytes: 4,
                max_nesting: 8,
            },
        ),
        (
            "@default {$ map { 'a': map {}}}\n{next}",
            TypedPreludePreview {
                max_value_bytes: 1024,
                max_nesting: 2,
            },
        ),
    ] {
        let tok = CemTokenizer::from_source_with_typed_prelude_preview(
            BytesSource::new(SourceId(1), source.as_bytes().to_vec()),
            limits,
        );
        let capture =
            CemSchemaMachine::new(CompiledSchema::cem_core(), CemEventNormalizer::new(tok))
                .build_with_lexical_scopes();
        assert!(capture
            .document()
            .nodes
            .iter()
            .any(|n| matches!(n, CemAstNode::Error { .. })));
        assert_eq!(elements(capture.document(), "next").len(), 1);
    }
}
