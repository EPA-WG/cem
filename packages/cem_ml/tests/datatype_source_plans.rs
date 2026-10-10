//! Source classification prepares retained edges; it does not execute them.
use cem_ml::{
    events::cem::CemEventNormalizer,
    parser::{builder::CemAstBuilder, document::CemDocument, CemAstNode},
    schema::{
        datatype_registry::{
            DatatypeDependencyRole, DatatypeDependencyValue, DatatypeKind, DatatypeKindSource,
            DatatypePlanIssueKind, DatatypeRegistry, DatatypeSource,
        },
        declaration_references::SchemaDeclarationNode,
    },
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use std::sync::Arc;
fn source(text: &str) -> DatatypeSource {
    let owner = Arc::new(
        CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
            BytesSource::new(SourceId(1), text.as_bytes().to_vec()),
        )))
        .build(),
    );
    assert!(owner.diagnostics.is_empty(), "{:?}", owner.diagnostics);
    let node = |name: &str| {
        let id = owner
            .nodes
            .iter()
            .find_map(|node| match node {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == name => Some(*node_id),
                _ => None,
            })
            .unwrap();
        SchemaDeclarationNode::new(owner.clone(), id).unwrap()
    };
    let scope = node("schema");
    let mut registry = DatatypeRegistry::default();
    registry.insert(scope.clone(), node("type")).unwrap();
    registry.source(&scope, "custom").unwrap()
}

#[test]
fn list_item_and_inherited_base_plans_retain_different_edge_roles() {
    for (kind, role) in [
        ("list", DatatypeDependencyRole::ListItem),
        ("scalar", DatatypeDependencyRole::InheritedBase),
        ("node", DatatypeDependencyRole::InheritedBase),
    ] {
        let source = source(&format!(
            "{{schema | {{type @name=custom @kind={kind} @base={{#datatype}}}}}}"
        ));
        let plan = source.plan();
        assert!(plan.issues.is_empty());
        assert_eq!(plan.dependencies.len(), 1);
        let edge = &plan.dependencies[0];
        assert_eq!(edge.role, role);
        assert_eq!(
            edge.attribute.identity(),
            source.attribute("base").unwrap().identity()
        );
        let DatatypeDependencyValue::Native(reference) = &edge.value else {
            panic!()
        };
        assert!(Arc::ptr_eq(
            reference.document(),
            source.declaration().document()
        ));
        assert!(matches!(reference.node(), CemAstNode::Reference { .. }));
    }
}

#[test]
fn whole_list_plans_require_an_explicit_distinct_dependency() {
    for fields in ["@list-base=names", "@kind=list @list-base={#names}"] {
        let source = source(&format!("{{schema | {{type @name=custom {fields}}}}}"));
        let plan = source.plan();
        assert!(plan.issues.is_empty(), "{:?}", plan.issues);
        assert_eq!(plan.dependencies.len(), 1);
        assert_eq!(
            plan.dependencies[0].role,
            DatatypeDependencyRole::InheritedList
        );
        assert_eq!(
            plan.dependencies[0].attribute.identity(),
            source.attribute("list-base").unwrap().identity()
        );
    }
    for fields in [
        "@list-base=''",
        "@list-base={type}",
        "@list-base={$a}",
        "@kind=scalar @list-base=names",
        "@kind=node @list-base=names",
        "@kind=list @base=item @list-base=names",
        "@base=item @list-base=names",
        "@kind='' @list-base=names",
    ] {
        let plan = source(&format!("{{schema | {{type @name=custom {fields}}}}}")).plan();
        assert!(!plan.issues.is_empty(), "{fields}");
    }
}

#[test]
fn omitted_kind_waits_for_base_and_missing_kind_is_not_a_string_fallback() {
    let source = source("{schema | {type @name=custom @base=vendor:integer}}");
    let plan = source.plan();
    assert_eq!(plan.kind, DatatypeKindSource::Inherited);
    assert!(plan.issues.is_empty());
    assert_eq!(
        plan.dependencies[0].role,
        DatatypeDependencyRole::InheritedBase
    );
    assert!(
        matches!(&plan.dependencies[0].value, DatatypeDependencyValue::Literal(value) if value == "vendor:integer")
    );
    let missing = self::source("{schema | {type @name=custom}}").plan();
    assert_eq!(missing.kind, DatatypeKindSource::Missing);
    assert!(missing
        .issues
        .iter()
        .any(|issue| issue.kind == DatatypePlanIssueKind::MissingKind));
}

#[test]
fn malformed_kind_never_infers_from_an_available_base() {
    for field in ["@kind=''", "@kind=vendor-kind", "@kind={#kind}"] {
        let source = source(&format!(
            "{{schema | {{type @name=custom {field} @base=integer}}}}"
        ));
        let plan = source.plan();
        assert_eq!(plan.kind, DatatypeKindSource::Invalid);
        let issue = plan
            .issues
            .iter()
            .find(|issue| issue.kind == DatatypePlanIssueKind::InvalidKind)
            .unwrap();
        assert_eq!(
            issue.source.identity(),
            source.attribute("kind").unwrap().identity()
        );
    }
}

#[test]
fn retained_constant_plans_preserve_children_without_evaluation() {
    let source = source("{schema | {type @name=custom @kind=scalar | {constant @value=\"In progress\"} {#constants}}}");
    let plan = source.plan();
    assert!(plan.issues.is_empty());
    assert_eq!(plan.constant_slots.len(), 2);
    assert!(matches!(plan.constant_slots[0].node(), CemAstNode::Element { expanded_name, .. } if expanded_name.local_name == "constant"));
    assert!(matches!(plan.constant_slots[1].node(), CemAstNode::Reference { targets: None, .. }));
    assert!(plan.constant_slots.iter().all(|n| Arc::ptr_eq(n.document(), source.declaration().document())));
}

#[test]
fn retained_constant_metamodel_admits_explicit_values_and_rejects_unknown_children() {
    use cem_ml::schema::document_model::{compile_schema_document_model, validate_document_model};
    let model = compile_schema_document_model("schema", include_str!("../schema-packages/schema/v1/schema/cem-schema.cem"));
    for (children, valid) in [
        ("{constant @value=\"In progress\"} {constant @value=\"\"}", true),
        ("{constant}", false),
        ("{constant @value=x @unknown=y}", false),
        ("{value | In progress}", false),
    ] {
        let input = source(&format!("{{schema @name=test @namespace=urn:test @version=1.0.0 | {{types | {{type @name=custom @kind=scalar | {children}}}}}}}"));
        let diagnostics = validate_document_model(input.declaration().document(), &model);
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.severity.is_hard_violation()).collect();
        assert_eq!(errors.is_empty(), valid, "{children}: {errors:?}");
    }
}

#[test]
fn literal_rules_remain_descriptive_and_native_rule_shape_is_checked() {
    let prose =
        source("{schema | {type @name=custom @kind=scalar @rule='signed decimal integer'}}").plan();
    assert_eq!(
        prose.kind,
        DatatypeKindSource::Explicit(DatatypeKind::Scalar)
    );
    assert!(prose.dependencies.is_empty());
    assert!(prose.descriptive_rule.is_some());
    let native = source("{schema | {type @name=custom @kind=scalar @rule={#behavior}}}").plan();
    assert!(native.descriptive_rule.is_none());
    assert_eq!(
        native.dependencies[0].role,
        DatatypeDependencyRole::ValidationRule
    );
    for field in ["@base={datatype}", "@rule={behavior}"] {
        let source = source(&format!(
            "{{schema | {{type @name=custom @kind=scalar {field}}}}}"
        ));
        let plan = source.plan();
        assert!(plan
            .issues
            .iter()
            .any(|issue| issue.kind == DatatypePlanIssueKind::InvalidDependency));
        assert!(plan.dependencies.is_empty());
    }
}

#[test]
fn list_and_node_values_are_rejected_at_original_fields_without_scalar_rejection() {
    for kind in ["list", "node"] {
        let source = source(&format!(
            "{{schema | {{type @name=custom @kind={kind} @values='a b'}}}}"
        ));
        let plan = source.plan();
        let issue = plan
            .issues
            .iter()
            .find(|issue| issue.kind == DatatypePlanIssueKind::UnsupportedFacet)
            .unwrap();
        assert_eq!(
            issue.source.identity(),
            source.attribute("values").unwrap().identity()
        );
    }
    let source =
        source("{schema | {type @name=custom @kind=scalar @values='a b' @vendor-extension=kept}}");
    let plan = source.plan();
    assert!(plan.issues.is_empty());
    assert!(plan.source.attribute("values").is_some());
    assert!(plan.source.attribute("vendor-extension").is_some());
}
