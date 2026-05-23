use cem_ml::diagnostics::Severity;
use cem_ql::api::parse;
use cem_ql::resolve::{Arity, QNameKey, SchemaTypeId};
use cem_ql::types::{
    AtomType, FunctionSignature, FunctionSignatureKey, NodeKind, RecordField, SchemaTypeInfo,
    SubtypeChecker, TyConfig, Type, TypeChecker,
};

fn string_length_signature() -> FunctionSignature {
    FunctionSignature {
        name: QNameKey::new(Some("str".to_owned()), "length"),
        params: vec![Type::atom(AtomType::String)],
        ret: Type::atom(AtomType::Integer),
    }
}

fn check(source: &str, checker: &mut TypeChecker) -> cem_ql::types::TypeReport {
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    checker.check_surface_module(&parsed.module)
}

#[test]
fn checker_infers_literals_declarations_params_and_registered_calls() {
    let mut checker = TypeChecker::new();
    checker.register_function(string_length_signature());
    let report = check(
        r#"declare variable label := "submit"
           declare function local:echo(item as string) { item }
           str:length(local:echo(label))"#,
        &mut checker,
    );

    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert_eq!(report.root_type, Some(Type::atom(AtomType::Integer)));
    assert!(checker.functions.contains_key(&FunctionSignatureKey {
        name: QNameKey::new(Some("local".to_owned()), "echo"),
        arity: Arity(1),
    }));
}

#[test]
fn strict_profile_reports_static_type_errors_as_errors() {
    let mut checker = TypeChecker::new();
    checker.register_function(string_length_signature());
    let report = check("str:length(42)", &mut checker);

    assert!(report
        .diagnostics
        .iter()
        .any(|diag| { diag.code == "cem.ql.type_error" && diag.severity == Severity::Error }));
}

#[test]
fn dev_profile_relaxes_static_failures_and_silences_cross_type_compare() {
    let mut checker = TypeChecker::with_config(TyConfig::dev_profile());
    checker.register_function(string_length_signature());
    let report = check("str:length(42) 1 = 1.0", &mut checker);

    assert!(report
        .diagnostics
        .iter()
        .any(|diag| { diag.code == "cem.ql.type_error" && diag.severity == Severity::Warning }));
    assert!(!report
        .diagnostics
        .iter()
        .any(|diag| diag.code == "cem.ql.cross_type_compare"));
}

#[test]
fn cross_atom_type_comparison_emits_warning_under_strict_profile() {
    let mut checker = TypeChecker::new();
    let report = check("1 = 1.0", &mut checker);

    assert!(report.diagnostics.iter().any(|diag| {
        diag.code == "cem.ql.cross_type_compare" && diag.severity == Severity::Warning
    }));
    assert_eq!(report.root_type, Some(Type::atom(AtomType::Boolean)));
}

#[test]
fn unknown_type_uses_the_configured_static_resolution_severity() {
    let mut checker = TypeChecker::with_config(TyConfig::dev_profile());
    let report = check("value treat as MissingType", &mut checker);

    assert!(report
        .diagnostics
        .iter()
        .any(|diag| { diag.code == "cem.ql.unknown_type" && diag.severity == Severity::Warning }));
    assert!(report.diagnostics.iter().any(|diag| {
        diag.code == "cem.ql.unknown_variable" && diag.severity == Severity::Warning
    }));
}

#[test]
fn schema_element_types_are_scope_relative_and_walk_structural_supertypes() {
    let control = SchemaTypeId(1);
    let button = SchemaTypeId(2);
    let mut checker = TypeChecker::new();
    checker.register_schema_type(SchemaTypeInfo {
        id: control,
        name: QNameKey::new(None, "Control"),
        element_name: QNameKey::new(None, "control"),
        structural_supertypes: Vec::new(),
    });
    checker.register_schema_type(SchemaTypeInfo {
        id: button,
        name: QNameKey::new(None, "Button"),
        element_name: QNameKey::new(None, "button"),
        structural_supertypes: vec![control],
    });
    checker.declare_variable(QNameKey::new(None, "button"), Type::SchemaElement(button));

    let report = check("button treat as Control", &mut checker);

    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    assert_eq!(report.root_type, Some(Type::SchemaElement(control)));
    assert!(Type::SchemaElement(button).is_subtype_of(
        &Type::Node(NodeKind::Element(QNameKey::new(None, "button"))),
        &checker.schemas
    ));
}

#[test]
fn structural_subtype_checker_rejects_record_shape_drift() {
    let schemas = Default::default();
    let checker = SubtypeChecker::new(&schemas);
    let left = Type::Record(vec![RecordField {
        name: "name".to_owned(),
        ty: Type::atom(AtomType::String),
    }]);
    let right = Type::Record(vec![RecordField {
        name: "name".to_owned(),
        ty: Type::atom(AtomType::Integer),
    }]);

    assert!(!checker.is_subtype(&left, &right));
}
