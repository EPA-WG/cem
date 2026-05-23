use cem_ml::diagnostics::Severity;
use cem_ml::source::ByteRange;
use cem_ql::api::parse;
use cem_ql::parser::{ImportDecl, QName};
use cem_ql::resolve::{
    Arity, BindingKind, BindingSet, ImportKind, ImportPolicy, NameResolver, QNameKey, Resolution,
};

fn qname(local: &str) -> QName {
    QName {
        prefix: None,
        local: local.to_owned(),
        range: ByteRange::new(0, local.len() as u32),
    }
}

fn prefixed(prefix: &str, local: &str) -> QName {
    QName {
        prefix: Some(prefix.to_owned()),
        local: local.to_owned(),
        range: ByteRange::new(0, (prefix.len() + local.len() + 1) as u32),
    }
}

fn import(uri: &str) -> ImportDecl {
    ImportDecl {
        uri: uri.to_owned(),
        alias: None,
        range: ByteRange::new(0, uri.len() as u32),
    }
}

#[test]
fn binding_set_lookup_uses_the_documented_precedence_order() {
    let mut resolver = NameResolver::new();
    let mut set = BindingSet::new(7);
    let name = QNameKey::new(None, "item");
    let variable =
        resolver.declare_binding(&mut set, BindingKind::Variable, name.clone(), None, None);
    let _function = resolver.declare_binding(
        &mut set,
        BindingKind::Function,
        name.clone(),
        Some(Arity(0)),
        None,
    );
    let _ty = resolver.declare_binding(&mut set, BindingKind::SchemaType, name.clone(), None, None);
    assert_eq!(set.lookup(&qname("item")), Some(variable));
}

#[test]
fn resolver_uses_inner_to_outer_scope_order_and_records_trace() {
    let mut resolver = NameResolver::new();
    let mut outer = BindingSet::new(1);
    let mut inner = BindingSet::new(2);
    let name = QNameKey::new(None, "value");
    let _outer_id =
        resolver.declare_binding(&mut outer, BindingKind::Variable, name.clone(), None, None);
    let inner_id = resolver.declare_binding(&mut inner, BindingKind::Variable, name, None, None);
    resolver.push_site(inner);
    resolver.push_site(outer);
    assert_eq!(
        resolver.resolve(&qname("value")),
        Resolution::Resolved(inner_id)
    );
    assert_eq!(resolver.trace.len(), 1);
    assert_eq!(resolver.trace[0].scope_id, 2);
    assert_eq!(resolver.trace[0].binding_id, inner_id);
}

#[test]
fn ac_qv_v_1_covers_scope_inheritance_overlay_and_trace_cases() {
    let mut resolver = NameResolver::new();
    let mut outer = BindingSet::new(10);
    let mut inner = BindingSet::new(11);

    let inherited = resolver.declare_binding(
        &mut outer,
        BindingKind::Variable,
        QNameKey::new(None, "inherited"),
        None,
        None,
    );
    let _outer_shadowed = resolver.declare_binding(
        &mut outer,
        BindingKind::Variable,
        QNameKey::new(None, "shadowed"),
        None,
        None,
    );
    let inner_shadow = resolver.declare_binding(
        &mut inner,
        BindingKind::Variable,
        QNameKey::new(None, "shadowed"),
        None,
        None,
    );
    inner.overlay.insert(
        "cem:stdlib/strings",
        QNameKey::new(Some("str".to_owned()), "length"),
        cem_ql::resolve::BindingId(777),
    );

    resolver.push_site(inner);
    resolver.push_site(outer);

    assert_eq!(
        resolver.resolve(&qname("shadowed")),
        Resolution::Resolved(inner_shadow)
    );
    assert_eq!(
        resolver.resolve(&qname("inherited")),
        Resolution::Resolved(inherited)
    );
    assert_eq!(
        resolver.resolve_function(&prefixed("str", "length"), Arity(1)),
        Resolution::Resolved(cem_ql::resolve::BindingId(777))
    );
    assert_eq!(resolver.trace.len(), 3);
    assert_eq!(resolver.trace[0].scope_id, 11);
    assert_eq!(resolver.trace[0].binding_id, inner_shadow);
    assert_eq!(resolver.trace[1].scope_id, 10);
    assert_eq!(resolver.trace[1].binding_id, inherited);
    assert_eq!(resolver.trace[2].scope_id, 11);
    assert_eq!(resolver.trace[2].binding_kind, BindingKind::OverlayBinding);
}

#[test]
fn resolver_walks_module_bindings_function_params_and_calls() {
    let parsed = parse(
        r#"import "cem:stdlib/strings" as str
           declare variable source := 42
           declare function local:echo(item) { item }
           local:echo(source)"#,
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let mut resolver = NameResolver::new();
    let report = resolver.resolve_surface_module(&parsed.module, &ImportPolicy::new());
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert!(
        report
            .trace
            .iter()
            .any(|event| event.name == QNameKey::new(None, "item") && event.scope_id == 1),
        "{:?}",
        report.trace
    );
    assert!(
        report
            .trace
            .iter()
            .any(|event| event.name == QNameKey::new(Some("local".to_owned()), "echo")),
        "{:?}",
        report.trace
    );
    assert!(
        report
            .trace
            .iter()
            .any(|event| event.name == QNameKey::new(None, "source")),
        "{:?}",
        report.trace
    );
}

#[test]
fn unknown_names_emit_cem_ql_diagnostics() {
    let parsed = parse("missing");
    let mut resolver = NameResolver::new();
    let report = resolver.resolve_surface_module(&parsed.module, &ImportPolicy::new());
    assert!(report
        .diagnostics
        .iter()
        .any(|diag| diag.code == "cem.ql.unknown_variable"));
}

#[test]
fn import_policy_resolves_platform_stdlib_and_external_grants() {
    let policy = ImportPolicy::new().allow_scheme("https").unwrap();
    assert_eq!(
        policy
            .resolve_import(&import("cem:stdlib/strings"))
            .unwrap()
            .kind,
        ImportKind::PlatformStdlib
    );
    assert_eq!(
        policy
            .resolve_import(&import("https://example.test/query.cemql"))
            .unwrap()
            .kind,
        ImportKind::External
    );
    let denied = ImportPolicy::new()
        .resolve_import(&import("https://example.test/query.cemql"))
        .unwrap_err();
    assert_eq!(denied.code, "cem.ql.import_denied");
    assert_eq!(denied.severity, Severity::Warning);
}

#[test]
fn import_policy_handles_reserved_and_plugin_schemes() {
    let reserved = ImportPolicy::new().allow_scheme("cem").unwrap_err();
    assert_eq!(reserved.code, "cem.ql.reserved_scheme");
    let reserved_uri = ImportPolicy::new().allow_scheme("cem:").unwrap_err();
    assert_eq!(reserved_uri.code, "cem.ql.reserved_scheme");
    let reserved_plugin = ImportPolicy::new().allow_scheme("urn:cem:").unwrap_err();
    assert_eq!(reserved_plugin.code, "cem.ql.reserved_scheme");

    let unresolved = ImportPolicy::new()
        .resolve_import(&import("urn:cem:plugin:query"))
        .unwrap_err();
    assert_eq!(unresolved.code, "cem.ql.import_unresolved");

    let plugin = ImportPolicy::new().register_urn_cem("urn:cem:plugin:query");
    assert_eq!(
        plugin
            .resolve_import(&import("urn:cem:plugin:query"))
            .unwrap()
            .kind,
        ImportKind::PluginRegistry
    );
}

#[test]
fn stdlib_overlay_resolves_when_no_inner_binding_matches() {
    let mut site = BindingSet::new(4);
    let key = QNameKey::new(Some("str".to_owned()), "length");
    site.overlay
        .insert("cem:stdlib/strings", key, cem_ql::resolve::BindingId(99));

    let mut resolver = NameResolver::with_sites(vec![site]);
    assert_eq!(
        resolver.resolve_function(&prefixed("str", "length"), Arity(1)),
        Resolution::Resolved(cem_ql::resolve::BindingId(99))
    );
    assert_eq!(resolver.trace[0].binding_kind, BindingKind::OverlayBinding);
}
