use cem_ml::schema::{
    document_model::compile_schema_document_model, reference_traversal::ReferenceTraversalLimits,
};

fn scope(source: &str) -> cem_ml::schema::document_model::SchemaDocumentModel {
    compile_schema_document_model("https://example.test/scope", source)
}

#[test]
fn schema_defaults_and_child_overrides_are_independent_of_lexical_depth() {
    let defaults = ReferenceTraversalLimits::schema_defaults().unwrap();
    assert_eq!(defaults.max_depth, 128);
    assert_eq!(defaults.max_work, 100_000);
    let parent = defaults
        .for_scope(&scope(
            r#"{schema | {constraints |
            {constraint @kind="reference-traversal-depth" @value="16"}
        }}"#,
        ))
        .unwrap();
    assert_eq!(parent.max_depth, 16);
    assert_eq!(parent.max_work, defaults.max_work);
    let child = parent
        .for_scope(&scope(
            r#"{schema | {constraints |
            {constraint @kind="reference-traversal-work" @value="50"}
        }}"#,
        ))
        .unwrap();
    assert_eq!(child.max_depth, 16);
    assert_eq!(child.max_work, 50);
    assert_eq!(parent.max_work, defaults.max_work);
    assert_eq!(defaults.for_scope(&scope("{schema}")).unwrap(), defaults);
}

#[test]
fn invalid_scope_limits_are_rejected_without_changing_parent_policy() {
    let parent = ReferenceTraversalLimits::schema_defaults().unwrap();
    for kind in ["reference-traversal-depth", "reference-traversal-work"] {
        for value in [
            "0",
            "-1",
            "unknown",
            "",
            "999999999999999999999999999999999999",
        ] {
            let source = format!(
                r#"{{schema | {{constraints | {{constraint @kind="{kind}" @value="{value}"}} }} }}"#
            );
            let error = parent.for_scope(&scope(&source)).unwrap_err();
            assert_eq!(error.constraint, kind);
            if !value.is_empty() {
                assert_eq!(error.value.as_deref(), Some(value));
            }
        }
        let source = format!(r#"{{schema | {{constraints | {{constraint @kind="{kind}"}} }} }}"#);
        assert!(parent.for_scope(&scope(&source)).is_err());
    }
    assert_eq!(parent, ReferenceTraversalLimits::schema_defaults().unwrap());
    let invalid_parent = ReferenceTraversalLimits {
        max_depth: 0,
        ..parent
    };
    assert!(invalid_parent.for_scope(&scope("{schema}")).is_err());
}
