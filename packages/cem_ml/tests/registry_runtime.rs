//! Scoped template-registry integration smoke (AC-R-1, AC-R-2, AC-R-3).

use cem_ml::diagnostics::Severity;
use cem_ml::registry::{CollisionDiagnostic, RegistryScopeId, ScopedRegistryTree, TemplateRef};

fn dce(tag: &str) -> TemplateRef {
    TemplateRef::DceTagName {
        tag_name: tag.into(),
    }
}

#[test]
fn ac_r_1_dce_tag_names_are_first_class_template_refs() {
    let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
    assert!(tree
        .install(RegistryScopeId(0), "x-card", dce("x-card"))
        .is_none());
    let hit = tree.resolve(RegistryScopeId(0), "x-card").unwrap();
    assert!(matches!(
        hit.template_ref,
        TemplateRef::DceTagName { tag_name } if tag_name == "x-card"
    ));
}

#[test]
fn ac_r_2_descendants_inherit_ancestor_registry_entries() {
    let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
    tree.install(RegistryScopeId(0), "x-card", dce("x-card"));
    tree.add_scope(RegistryScopeId(1), RegistryScopeId(0));
    tree.add_scope(RegistryScopeId(2), RegistryScopeId(1));
    let hit = tree.resolve(RegistryScopeId(2), "x-card").unwrap();
    assert_eq!(hit.scope, RegistryScopeId(0));
}

#[test]
fn ac_r_3_shadowing_install_surfaces_warning_diagnostic() {
    let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
    tree.install(RegistryScopeId(0), "x-card", dce("x-card"));
    tree.add_scope(RegistryScopeId(1), RegistryScopeId(0));
    let diag = tree
        .install(RegistryScopeId(1), "x-card", dce("x-card-v2"))
        .expect("shadowing must emit a diagnostic");
    assert_eq!(diag.code(), CollisionDiagnostic::CODE);
    assert_eq!(diag.severity, Severity::Warning);
    assert_eq!(diag.child_scope, 1);
    assert_eq!(diag.ancestor_scope, 0);
}

#[test]
fn ac_r_3_policy_can_raise_collision_severity() {
    let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
    tree.install(RegistryScopeId(0), "x-card", dce("x-card"));
    tree.add_scope(RegistryScopeId(1), RegistryScopeId(0));
    let diag = tree
        .install_with_collision_severity(
            RegistryScopeId(1),
            "x-card",
            dce("x-card-v2"),
            Severity::Error,
        )
        .expect("shadowing must emit a diagnostic");
    assert_eq!(diag.code(), "cem.registry.collision");
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn ac_r_3_sibling_scopes_can_register_same_name_without_collision_against_each_other() {
    let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
    tree.add_scope(RegistryScopeId(1), RegistryScopeId(0));
    tree.add_scope(RegistryScopeId(2), RegistryScopeId(0));
    assert!(tree
        .install(RegistryScopeId(1), "x-card", dce("a"))
        .is_none());
    assert!(tree
        .install(RegistryScopeId(2), "x-card", dce("b"))
        .is_none());
    // Each sibling resolves locally to its own entry; neither shadows
    // the other because neither is an ancestor of the other.
    assert_eq!(
        tree.resolve(RegistryScopeId(1), "x-card").unwrap().scope,
        RegistryScopeId(1)
    );
    assert_eq!(
        tree.resolve(RegistryScopeId(2), "x-card").unwrap().scope,
        RegistryScopeId(2)
    );
}

#[test]
fn ac_r_2_shadowed_child_does_not_change_grandchild_fallback() {
    let mut tree = ScopedRegistryTree::new(RegistryScopeId(0));
    tree.install(RegistryScopeId(0), "x-card", dce("root-card"));
    tree.add_scope(RegistryScopeId(1), RegistryScopeId(0));
    tree.add_scope(RegistryScopeId(2), RegistryScopeId(1));
    tree.install(RegistryScopeId(1), "x-card", dce("child-card"));

    let child_hit = tree.resolve(RegistryScopeId(1), "x-card").unwrap();
    assert_eq!(child_hit.scope, RegistryScopeId(1));
    assert!(matches!(
        child_hit.template_ref,
        TemplateRef::DceTagName { tag_name } if tag_name == "child-card"
    ));

    let grandchild_hit = tree.resolve(RegistryScopeId(2), "x-card").unwrap();
    assert_eq!(grandchild_hit.scope, RegistryScopeId(1));
}
