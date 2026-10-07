//! Original-source reporting is independent of consuming scope and local IDs.
use cem_ml::{
    diagnostics::Diagnostic,
    import::import_bytes_with_lexical_scopes,
    parser::CemAstNode,
    schema::{
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        input_references::validate_structural_input_references,
        reference_policy::ReferenceScopePolicy,
        vocab::CompiledSchema,
    },
};
use cem_ql::{api::StandaloneExpressionContext, schema_references::CemQlSchemaDeclarationHost};

#[test]
fn structural_diagnostics_keep_the_authored_owner_under_a_foreign_effective_scope() {
    for (text, format) in [
        ("\n{item @extra=yes}", "text/cem-ml"),
        ("\n<item extra='yes'/>", "application/xml"),
    ] {
        let source = import_bytes_with_lexical_scopes(
            text.as_bytes(),
            format,
            "vendor.cem",
            CompiledSchema::cem_core(),
        )
        .unwrap();
        let schema = import_bytes_with_lexical_scopes(
            b"{schema | {elements | {element @name=item @required-attributes=needed}}}",
            "text/cem-ml",
            "consumer.cem",
            CompiledSchema::cem_core(),
        )
        .unwrap();
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        host.register_scope(
            source.tree.clone(),
            Some(StandaloneExpressionContext::default()),
            policy.clone(),
        );
        let effective = host.register_scope(
            schema.tree.clone(),
            Some(StandaloneExpressionContext::default()),
            policy.clone(),
        );
        let id = source
            .tree
            .ast()
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element { node_id, .. } => Some(*node_id),
                _ => None,
            })
            .unwrap();
        let node = SchemaDeclarationNode::new(source.tree.ast_owner().clone(), id).unwrap();
        assert!(host.assign_subtree_scope(&source.tree, id, effective));
        let model = host
            .compile("schema:consumer", schema.tree.clone(), policy.limits)
            .unwrap();
        let report = validate_structural_input_references(
            source.tree.ast_owner().clone(),
            &model,
            &mut host,
            policy.limits,
        )
        .unwrap();
        assert!(report.complete && report.failed);
        for code in [
            "cem.schema_model.missing_required_attribute",
            "cem.schema_model.unknown_attribute",
        ] {
            let diagnostic = report.diagnostics.iter().find(|d| d.code == code).unwrap();
            assert_eq!(diagnostic.uri.as_deref(), Some("vendor.cem"));
            assert_eq!(diagnostic.line, Some(2));
            assert!(diagnostic.byte_offset.unwrap() > 0);
            assert!(diagnostic.column.unwrap() > 0);
        }
        // A host-supplied diagnostic already belonging to another original owner
        // must not be rebound even when its local frame IDs match this source.
        let diagnostic = Diagnostic {
            uri: Some("another-vendor.cem".into()),
            line: Some(7),
            column: Some(9),
            byte_offset: Some(42),
            code: "vendor.rule".into(),
            ..Default::default()
        };
        assert_eq!(
            host.structural_diagnostic(&node, diagnostic.clone()),
            diagnostic
        );
    }
}
