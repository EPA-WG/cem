use cem_ml::schema::{
    attribute_datatypes::{
        AttributeDatatypeContext, AttributeDatatypeInput, AttributeDatatypeValue,
    },
    attribute_references::ConsumedAttributeValue,
    document_model::SchemaDocumentModel,
    input_references::InputReferenceHost,
};
use cem_ml::{
    content_cache::ContentHash,
    import::import_bytes_with_lexical_scopes,
    operation_control::OperationControl,
    parser::{tree::RetainedCemTree, CemAstNode},
    schema::{
        reference_policy::ReferenceScopePolicy,
        structural_capsule::{export, verify, SourceIdentity, VerifiedStructuralCapsule},
        vocab::CompiledSchema,
    },
    source::SourceId,
};
use cem_ql::{
    api::{StandaloneExpressionBinding, StandaloneExpressionContext},
    eval::{ItemStream, RetainedCemNode},
    structural_receiver::FreshStructuralHost,
};
use std::sync::Arc;
const SCHEMA: &str = include_str!("../../cem_ml/fixtures/structural-cart.cem");
const RECEIVER:&str="{schema @name=receiver @namespace=https://app.test/schema/receiver | {elements | {element @name=view @required-attributes=cart}} {attributes | {attribute @name=cart @type=node @minItems=1 @maxItems=2}}}";
fn hash(bytes: &[u8]) -> String {
    ContentHash::from_blake3(bytes).header_value()
}
fn imported(text: &str, uri: &str) -> Arc<RetainedCemTree> {
    import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        uri,
        CompiledSchema::cem_core(),
    )
    .unwrap()
    .tree
}
fn capsule() -> Arc<VerifiedStructuralCapsule> {
    let uri = "https://app.test/schema/cart.cem";
    let source = import_bytes_with_lexical_scopes(
        SCHEMA.as_bytes(),
        "text/cem-ml",
        uri,
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let built = export(
        &source.captured,
        SourceIdentity::new(uri, SourceId(1), SCHEMA.as_bytes()),
        "https://app.test/schema/cart/1",
        &hash(b"registry"),
        &OperationControl::default(),
    )
    .unwrap();
    Arc::new(verify(&built.bytes, &built.pin, &OperationControl::default()).unwrap())
}
fn declaration(tree: &Arc<RetainedCemTree>) -> u32 {
    tree.ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "attribute" => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
#[test]
fn linked_receiver_validates_structure_and_local_cardinality_on_the_actual_host() {
    let receiver = imported(RECEIVER, "https://app.test/receiver.cem");
    let source = imported("{view @cart={#data}}", "https://app.test/view.cem");
    let data = imported(
        "\n{cart @title=Tea | {item @quantity=3}}",
        "https://app.test/cart.cem",
    );
    let id = data
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == "cart" => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let node = RetainedCemNode::new(data.clone(), id).unwrap().query_item();
    let context = StandaloneExpressionContext::default().with_binding(
        "data",
        StandaloneExpressionBinding::any(ItemStream::once(node)),
    );
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    host.register_scope(receiver.clone(), None, policy.clone())
        .unwrap();
    let from = host
        .register_scope(source.clone(), Some(context), policy.clone())
        .unwrap();
    let to = host.register_scope(data, None, policy).unwrap();
    assert!(host.allow_scope_crossing(from, to));
    let linked = host
        .link_receiver(
            capsule(),
            "https://app.test/schema/receiver",
            receiver.clone(),
            declaration(&receiver),
        )
        .unwrap();
    let report = host.validate_input(source, &linked).unwrap();
    assert!(
        report.complete && !report.failed,
        "{:?}",
        report.diagnostics
    );
    assert_eq!(report.nodes[0].attribute_values[0].access.roots().len(), 1);
}

#[test]
fn closed_hosts_cannot_reopen_by_registering_a_source() {
    let receiver = imported(RECEIVER, "https://app.test/receiver.cem");
    let mut host =
        FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
    let _ = host.register_scope(
        receiver.clone(),
        None,
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    host.close();
    let _ = host.register_scope(
        receiver.clone(),
        None,
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    assert!(host
        .link_receiver(
            capsule(),
            "https://app.test/schema/receiver",
            receiver.clone(),
            declaration(&receiver)
        )
        .is_err());
}

fn values(tree: &Arc<RetainedCemTree>, count: usize) -> StandaloneExpressionContext {
    let id = tree
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let node = RetainedCemNode::new(tree.clone(), id).unwrap().query_item();
    StandaloneExpressionContext::default().with_binding(
        "data",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![node; count])),
    )
}
struct Fixture {
    host: FreshStructuralHost,
    receiver: Arc<RetainedCemTree>,
    source: Arc<RetainedCemTree>,
    data: Arc<RetainedCemTree>,
    from: cem_ql::schema_references::DeclarationScope,
    to: cem_ql::schema_references::DeclarationScope,
    model: SchemaDocumentModel,
}
impl Fixture {
    fn new(
        receiver_text: &str,
        source_text: &str,
        data_text: &str,
        count: usize,
        grant: bool,
    ) -> Self {
        let receiver = imported(receiver_text, "https://app.test/receiver.cem");
        let source = imported(source_text, "https://app.test/view.cem");
        let data = imported(data_text, "https://app.test/cart.cem");
        let mut host =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        host.register_scope(receiver.clone(), None, policy.clone())
            .unwrap();
        let from = host
            .register_scope(source.clone(), Some(values(&data, count)), policy.clone())
            .unwrap();
        let to = host.register_scope(data.clone(), None, policy).unwrap();
        if grant {
            assert!(host.allow_scope_crossing(from, to));
        }
        let model = host
            .link_receiver(
                capsule(),
                "https://app.test/schema/receiver",
                receiver.clone(),
                declaration(&receiver),
            )
            .unwrap();
        Self {
            host,
            receiver,
            source,
            data,
            from,
            to,
            model,
        }
    }
    fn relink(&mut self) {
        self.model = self
            .host
            .link_receiver(
                capsule(),
                "https://app.test/schema/receiver",
                self.receiver.clone(),
                declaration(&self.receiver),
            )
            .unwrap();
    }
    fn report(
        &mut self,
    ) -> cem_ml::schema::input_references::StructuralInputValidation<
        cem_ql::schema_references::CemQlSchemaReferenceNode,
    > {
        self.host
            .validate_input(self.source.clone(), &self.model)
            .unwrap()
    }
}
const VIEW: &str = "{view @cart={#data}}";
const DATA: &str = "\n{cart @title=Tea | {item @quantity=3}}";
fn accepted(
    model: &SchemaDocumentModel,
    context: &AttributeDatatypeContext,
    value: &ConsumedAttributeValue,
    tree: &Arc<RetainedCemTree>,
) -> Option<bool> {
    model.attribute_datatypes["cart"]
        .validate(AttributeDatatypeInput {
            value: AttributeDatatypeValue::Nodes(value.access.clone()),
            source: value.attribute.node(),
            source_tree: Some(tree.clone()),
            element_name: "view",
            attribute_values: &Default::default(),
            control: &OperationControl::default(),
            context: Some(context),
        })
        .accepted
}

#[test]
fn scalar_structure_and_child_failures_keep_original_target_diagnostics() {
    for data in [
        "\n{cart @title=T | {item @quantity=3}}",
        "\n{cart @title=Tea | {item @quantity=9}}",
        "\n{cart @title=Tea}",
        "\n{cart @title=Tea | {other}}",
        "\n{cart @title=Tea @extra=yes | {item @quantity=3}}",
    ] {
        let mut f = Fixture::new(RECEIVER, VIEW, data, 1, true);
        let result = f.report();
        assert!(
            result.complete && result.failed,
            "{data}: {:?}",
            result.diagnostics
        );
        assert!(result.diagnostics.iter().any(|d|d.uri.as_deref()==Some("https://app.test/cart.cem")&&d.line==Some(2)),"{:?}",result.diagnostics);
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.uri.as_deref() == Some("https://app.test/receiver.cem")));
        // Independent original-source compilation goes through the ordinary
        // full native host, not the linked adapter or a reloaded model.
        let schema = imported(SCHEMA, "https://app.test/schema/cart.cem");
        let native = cem_ml::schema::document_model::compile_schema_document_model_from_retained(
            "https://app.test/schema/cart/1",
            &schema,
        );
        let mut host = cem_ql::schema_references::CemQlSchemaDeclarationHost::new();
        host.register_scope(
            f.data.clone(),
            None,
            ReferenceScopePolicy::schema_defaults().unwrap(),
        );
        let expected = host
            .validate_input(
                f.data.clone(),
                &native,
                ReferenceScopePolicy::schema_defaults().unwrap().limits,
            )
            .unwrap();
        assert_eq!(result.complete, expected.complete);
        assert_eq!(result.failed, expected.failed);
        let diagnostics = |items: &Vec<cem_ml::diagnostics::Diagnostic>| {
            items
                .iter()
                .map(|d| {
                    format!(
                        "{:?}",
                        (
                            &d.code,
                            &d.uri,
                            &d.source_map,
                            d.byte_offset,
                            d.line,
                            d.column
                        )
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            diagnostics(&result.diagnostics),
            diagnostics(&expected.diagnostics)
        );
    }
}

#[test]
fn local_cardinality_counts_occurrences_and_preserves_duplicate_targets() {
    for count in [0, 1, 2, 3] {
        let mut f = Fixture::new(RECEIVER, VIEW, DATA, count, true);
        let result = f.report();
        assert!(result.complete);
        assert_eq!(
            result.failed,
            count == 0 || count == 3,
            "{:?}",
            result.diagnostics
        );
        let access = &result.nodes[0].attribute_values[0].access;
        assert_eq!(access.roots().len(), count);
        if count == 2 {
            assert_eq!(
                access.node(access.roots()[0]).unwrap().identity(),
                access.node(access.roots()[1]).unwrap().identity()
            );
        }
    }
    let receiver = RECEIVER.replace("@minItems=1", "@minItems=0");
    let mut f = Fixture::new(&receiver, VIEW, DATA, 0, true);
    let result = f.report();
    assert!(
        result.complete && !result.failed,
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn grant_and_context_changes_require_fresh_linking_and_capture() {
    let mut f = Fixture::new(RECEIVER, VIEW, DATA, 1, false);
    assert!(!f.report().complete);
    assert!(f.host.allow_scope_crossing(f.from, f.to));
    assert!(
        !f.report().complete,
        "old linked model must not survive a grant epoch change"
    );
    f.relink();
    let original = f.report();
    assert!(original.complete && !original.failed);
    let old = original.nodes[0].attribute_values[0].clone();
    assert!(f.host.set_context(f.from, Some(values(&f.data, 2))));
    assert!(!f.report().complete);
    f.relink();
    assert_eq!(
        accepted(
            &f.model,
            f.host.attribute_receiver_context().unwrap(),
            &old,
            &f.source
        ),
        None,
        "new contract must not accept an old access proof"
    );
    let current = f.report();
    assert!(current.complete && !current.failed);
    let current = &current.nodes[0].attribute_values[0];
    assert_eq!(
        accepted(
            &f.model,
            f.host.attribute_receiver_context().unwrap(),
            current,
            &f.source
        ),
        Some(true)
    );
    assert!(f.host.allow_scope_crossing(f.from, f.to)); // Idempotent grant preserves the epoch.
    assert_eq!(
        accepted(
            &f.model,
            f.host.attribute_receiver_context().unwrap(),
            current,
            &f.source
        ),
        Some(true)
    );
    assert_eq!(
        accepted(
            &f.model,
            &AttributeDatatypeContext::default(),
            current,
            &f.source
        ),
        None
    );
}

#[test]
fn captured_access_cannot_be_rebound_to_another_attribute_or_host() {
    let mut f = Fixture::new(
        RECEIVER,
        "{view @cart={#data}}{view @cart={#data}}",
        DATA,
        1,
        true,
    );
    let result = f.report();
    assert!(result.complete && !result.failed);
    let first = result.nodes[0].attribute_values[0].clone();
    let second = &result.nodes[1].attribute_values[0];
    let mut relabelled = first.clone();
    relabelled.attribute = second.attribute.clone();
    assert_eq!(
        accepted(
            &f.model,
            f.host.attribute_receiver_context().unwrap(),
            &relabelled,
            &f.source
        ),
        None
    );
    let other_tree = imported("{view @cart={#data}}", "https://app.test/view.cem");
    assert_eq!(
        accepted(
            &f.model,
            f.host.attribute_receiver_context().unwrap(),
            &first,
            &other_tree
        ),
        None
    );
    let mut next = Fixture::new(RECEIVER, VIEW, DATA, 1, true);
    let report = next
        .host
        .validate_input(f.source.clone(), &f.model)
        .unwrap();
    assert!(!report.complete);
    let context = f.host.attribute_receiver_context().unwrap().clone();
    drop(f.host);
    assert_eq!(
        accepted(&f.model, &context, &first, &f.source),
        None,
        "retaining a context clone must not keep a dropped host alive"
    );
}

#[test]
fn pending_nested_expressions_and_lexical_values_cannot_become_valid_nodes() {
    let mut f = Fixture::new(RECEIVER, VIEW, "{cart @title=Tea | {#nested}}", 1, true);
    assert!(
        !f.report().complete,
        "inner expressions must not be evaluated by a literal adapter"
    );
    let mut f = Fixture::new(
        RECEIVER,
        "{view @cart=\"{cart @title=Tea}\"}",
        DATA,
        1,
        true,
    );
    let report = f.report();
    assert!(report.failed, "strings must not be parsed as node inputs");
    for fields in [
        "@default=anything",
        "@pattern=anything",
        "@minItems=3 @maxItems=2",
        "@itemCount=65",
        "@type=string",
        "@type=custom",
    ] {
        let source_text = if fields.starts_with("@type=") {
            RECEIVER.replace("@type=node", fields)
        } else {
            RECEIVER.replace("@type=node", &format!("@type=node {fields}"))
        };
        let receiver = imported(&source_text, "https://app.test/receiver.cem");
        let mut host =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        host.register_scope(
            receiver.clone(),
            None,
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
        assert!(
            host.link_receiver(
                capsule(),
                "https://app.test/schema/receiver",
                receiver.clone(),
                declaration(&receiver)
            )
            .is_err(),
            "{fields}"
        );
    }
}

#[test]
fn registry_source_and_cancellation_failures_do_not_mutate_the_previous_binding() {
    let mut f = Fixture::new(RECEIVER, VIEW, DATA, 1, true);
    let foreign = imported(RECEIVER, "https://app.test/receiver.cem");
    assert!(f
        .host
        .link_receiver(
            capsule(),
            "https://app.test/schema/receiver",
            foreign.clone(),
            declaration(&foreign)
        )
        .is_err());
    assert!(f
        .host
        .link_receiver(
            capsule(),
            "https://app.test/wrong",
            f.receiver.clone(),
            declaration(&f.receiver)
        )
        .is_err());
    let good = f.report();
    assert!(good.complete && !good.failed);
    let mut host =
        FreshStructuralHost::new(&hash(b"other registry"), OperationControl::default()).unwrap();
    host.register_scope(
        f.receiver.clone(),
        None,
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )
    .unwrap();
    assert!(host
        .link_receiver(
            capsule(),
            "https://app.test/schema/receiver",
            f.receiver.clone(),
            declaration(&f.receiver)
        )
        .is_err());
    let control = OperationControl::default();
    let mut host = FreshStructuralHost::new(&hash(b"registry"), control.clone()).unwrap();
    host.register_scope(
        f.receiver.clone(),
        None,
        ReferenceScopePolicy::schema_defaults().unwrap(),
    )
    .unwrap();
    control.cancel_root(None, None).unwrap();
    assert!(host
        .link_receiver(
            capsule(),
            "https://app.test/schema/receiver",
            f.receiver.clone(),
            declaration(&f.receiver)
        )
        .is_err());
    assert!(host.validate_input(f.source.clone(), &f.model).is_err());
    assert!(f.report().complete);
}

#[test]
fn receiver_metadata_changes_cannot_reuse_a_previous_linked_contract() {
    let mut f = Fixture::new(RECEIVER, VIEW, DATA, 1, true);
    f.model.attributes.get_mut("cart").unwrap().max_items = Some("0".into());
    assert!(
        !f.report().complete,
        "a linked contract must not silently ignore new receiver restrictions"
    );
    let value = f.report().nodes[0].attribute_values[0].clone();
    assert_eq!(
        accepted(
            &f.model,
            f.host.attribute_receiver_context().unwrap(),
            &value,
            &f.source
        ),
        None
    );
}

#[test]
fn untyped_node_capture_cannot_supply_a_linked_receiver_invocation() {
    let mut f = Fixture::new(RECEIVER, VIEW, DATA, 1, true);
    let mut bare = f.model.clone();
    bare.attribute_datatypes.clear();
    let result = f.host.validate_input(f.source.clone(), &bare).unwrap();
    assert!(result.complete && !result.failed);
    let value = &result.nodes[0].attribute_values[0];
    assert_eq!(
        accepted(
            &f.model,
            f.host.attribute_receiver_context().unwrap(),
            value,
            &f.source
        ),
        None
    );
}

#[test]
fn retirement_and_data_bounds_never_reuse_a_completed_verdict() {
    let mut f = Fixture::new(RECEIVER, VIEW, DATA, 1, true);
    assert!(f.report().complete);
    f.model.attribute_datatypes["cart"].retire_preparations();
    assert!(!f.report().complete);
    f.relink();
    assert!(f.report().complete);
    let large = format!(
        "{{cart @title=Tea | {} {{item @quantity=3}}}}",
        "x".repeat(1024 * 1024)
    );
    let mut f = Fixture::new(RECEIVER, VIEW, &large, 1, true);
    assert!(!f.report().complete);
}

#[test]
fn mixed_original_owners_and_bounded_root_sequences_keep_their_identity() {
    let mut f = Fixture::new(RECEIVER, VIEW, DATA, 1, true);
    let other = imported(
        "{cart @title=Coffee | {item @quantity=4}}",
        "https://app.test/other-cart.cem",
    );
    let id = other
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let first = f
        .data
        .ast()
        .nodes
        .iter()
        .find_map(|n| match n {
            CemAstNode::Element { node_id, .. } => Some(*node_id),
            _ => None,
        })
        .unwrap();
    let to = f
        .host
        .register_scope(
            other.clone(),
            None,
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
    assert!(f.host.allow_scope_crossing(f.from, to));
    let context = StandaloneExpressionContext::default().with_binding(
        "data",
        StandaloneExpressionBinding::any(ItemStream::from_items(vec![
            RetainedCemNode::new(f.data.clone(), first)
                .unwrap()
                .query_item(),
            RetainedCemNode::new(other.clone(), id)
                .unwrap()
                .query_item(),
        ])),
    );
    assert!(f.host.set_context(f.from, Some(context)));
    f.relink();
    let result = f.report();
    assert!(
        result.complete && !result.failed,
        "{:?}",
        result.diagnostics
    );
    let access = &result.nodes[0].attribute_values[0].access;
    assert!(Arc::ptr_eq(
        access.node(access.roots()[0]).unwrap().document(),
        f.data.ast_owner()
    ));
    assert!(Arc::ptr_eq(
        access.node(access.roots()[1]).unwrap().document(),
        other.ast_owner()
    ));
    let receiver = RECEIVER.replace("@maxItems=2", "@maxItems=64");
    for count in [64, 65] {
        let mut f = Fixture::new(&receiver, VIEW, DATA, count, true);
        let report = f.report();
        assert_eq!(report.complete, count == 64, "{:?}", report.diagnostics);
    }
}

#[test]
fn receiver_source_policies_cannot_be_ignored_during_literal_linking() {
    for source in [
        RECEIVER.replace(
            "@namespace=https://app.test/schema/receiver",
            "@namespace=https://app.test/schema/receiver @namespace=https://foreign.test/schema",
        ),
        RECEIVER.replace("@name=receiver", "@name=receiver @unknown-policy=required"),
        RECEIVER.replace("{elements |", "{unknown-policy | {required}} {elements |"),
    ] {
        let receiver = imported(&source, "https://app.test/receiver.cem");
        let mut host =
            FreshStructuralHost::new(&hash(b"registry"), OperationControl::default()).unwrap();
        host.register_scope(
            receiver.clone(),
            None,
            ReferenceScopePolicy::schema_defaults().unwrap(),
        )
        .unwrap();
        assert!(
            host.link_receiver(
                capsule(),
                "https://app.test/schema/receiver",
                receiver.clone(),
                declaration(&receiver)
            )
            .is_err(),
            "{source}"
        );
    }
}
