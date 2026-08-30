use cem_ml::diagnostics::Severity;
use cem_ql::api::{compile, parse, CompileContext};
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
        r#"declare let label = "submit"
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
fn strict_profile_reports_integer_if_conditions_as_type_errors() {
    let mut checker = TypeChecker::new();
    let report = check(
        r#"declare let count = 1
           if count { "bad" } else { "ok" }"#,
        &mut checker,
    );

    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diag| diag.code == "cem.ql.type_error")
        .unwrap_or_else(|| panic!("expected type error, got {:?}", report.diagnostics));
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(
        diagnostic
            .details
            .as_ref()
            .and_then(|details| details.get("behavior"))
            .and_then(serde_json::Value::as_str),
        Some("cem-ql-type-report-fact")
    );
    assert_eq!(
        diagnostic
            .details
            .as_ref()
            .and_then(|details| details.get("expressionKind"))
            .and_then(serde_json::Value::as_str),
        Some("if-condition")
    );
}

#[test]
fn compile_fails_on_static_type_errors_before_artifact_lowering() {
    let error = compile(
        r#"module "https://example.test/queries/invalid-type-error"

declare let count = 1

if count { "bad" } else { "ok" }
"#,
        &CompileContext::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "cem.ql.compile_failed");
    assert!(error.message.contains("cem.ql.type_error"), "{}", error);
}

#[test]
fn runtime_import_surface_avoids_unknown_stdlib_and_import_cascades() {
    let parsed = parse(
        r#"module "https://example.test/queries/import-surface"

import "cem:stdlib/sequence" as seq
import "https://example.test/queries/shared" as shared

{
    let rows = ({ name: "Ada" });
    let selected = rows.where(fn(row) => row.name == "Ada");
    (seq:count(selected), any(selected, fn(row) => row.name == "Ada"), shared:value())
}
"#,
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);

    let mut checker = TypeChecker::new();
    checker.seed_runtime_import_surface(&parsed.module);
    let report = checker.check_surface_module(&parsed.module);

    assert!(
        report.diagnostics.is_empty(),
        "seeded import surface should avoid false unknowns: {:?}",
        report.diagnostics
    );
}

#[test]
fn dev_profile_relaxes_static_failures_and_silences_cross_type_compare() {
    let mut checker = TypeChecker::with_config(TyConfig::dev_profile());
    checker.register_function(string_length_signature());
    let report = check("str:length(42) 1 == 1.0", &mut checker);

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
    let report = check("1 == 1.0", &mut checker);

    assert!(report.diagnostics.iter().any(|diag| {
        diag.code == "cem.ql.cross_type_compare" && diag.severity == Severity::Warning
    }));
    assert_eq!(report.root_type, Some(Type::atom(AtomType::Boolean)));
}

#[test]
fn minus_infers_numeric_or_stream_difference_by_operand_shape() {
    let mut numeric_checker = TypeChecker::new();
    let numeric = check("5 - 2", &mut numeric_checker);

    assert!(numeric.diagnostics.is_empty(), "{:?}", numeric.diagnostics);
    assert_eq!(numeric.root_type, Some(Type::atom(AtomType::Integer)));

    let mut stream_checker = TypeChecker::new();
    let stream = check("(1, 2, 3) - (2, 4)", &mut stream_checker);

    assert!(stream.diagnostics.is_empty(), "{:?}", stream.diagnostics);
    assert_eq!(
        stream.root_type,
        Some(Type::stream(Type::atom(AtomType::Integer)))
    );
}

#[test]
fn minus_rejects_mixed_numeric_and_stream_operands() {
    let mut checker = TypeChecker::new();
    let report = check("1 - (2, 3)", &mut checker);

    assert!(report.diagnostics.iter().any(|diag| {
        diag.code == "cem.ql.type_error" && diag.message.contains("numeric and stream")
    }));
}

#[test]
fn set_operators_promote_singleton_operands_to_streams() {
    let mut checker = TypeChecker::new();
    let report = check(r#"(1) | ("1")"#, &mut checker);

    assert!(
        !report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.ql.type_error"),
        "{:?}",
        report.diagnostics
    );
    assert!(report.diagnostics.iter().any(|diag| {
        diag.code == "cem.ql.cross_type_compare" && diag.severity == Severity::Warning
    }));
    assert_eq!(report.root_type, Some(Type::stream(Type::Any)));
}

#[test]
fn numeric_operators_require_matching_numeric_types() {
    for source in ["1 + 1.0", "1.0 * 1.0e0", "1 / 1.0e0", "1.0 % 2"] {
        let mut checker = TypeChecker::new();
        let report = check(source, &mut checker);

        assert!(
            report.diagnostics.iter().any(|diag| {
                diag.code == "cem.ql.type_error"
                    && diag.message.contains("matching numeric operand types")
            }),
            "{source}: {:?}",
            report.diagnostics
        );
    }
}

#[test]
fn plus_concatenates_only_exact_string_operands() {
    let mut checker = TypeChecker::new();
    let strings = check(r#""cem" + "-" + "ql""#, &mut checker);

    assert!(strings.diagnostics.is_empty(), "{:?}", strings.diagnostics);
    assert_eq!(strings.root_type, Some(Type::atom(AtomType::String)));

    for source in [r#""cem" + 1"#, r#"1 + "cem""#] {
        let mut checker = TypeChecker::new();
        let mixed = check(source, &mut checker);

        assert!(
            mixed.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "cem.ql.type_error"
                    && diagnostic
                        .message
                        .contains("implicit string conversion is not supported")
            }),
            "{source}: {:?}",
            mixed.diagnostics
        );
    }
}

#[test]
fn same_type_numeric_operators_preserve_operand_type() {
    for (source, expected) in [
        ("5 / 2", Type::atom(AtomType::Integer)),
        ("5.0 / 2.0", Type::atom(AtomType::Decimal)),
        ("5.0e0 / 2.0e0", Type::atom(AtomType::Double)),
    ] {
        let mut checker = TypeChecker::new();
        let report = check(source, &mut checker);

        assert!(
            report.diagnostics.is_empty(),
            "{source}: {:?}",
            report.diagnostics
        );
        assert_eq!(report.root_type, Some(expected), "{source}");
    }
}

#[test]
fn computed_record_keys_must_type_check_as_strings() {
    let mut checker = TypeChecker::new();
    let report = check(r#"{ [42]: "Ada" }"#, &mut checker);

    assert!(report
        .diagnostics
        .iter()
        .any(|diag| diag.code == "cem.ql.type_error"));
}

#[test]
fn deferred_any_satisfies_static_expectations_without_hard_type_errors() {
    let mut checker = TypeChecker::new();
    checker.declare_variable(QNameKey::new(None, "runtimeKey"), Type::Any);
    let report = check(r#"{ [runtimeKey]: "Ada" }"#, &mut checker);

    assert!(
        report.diagnostics.is_empty(),
        "deferred runtime Any should not fail static subtype checks: {:?}",
        report.diagnostics
    );
    assert_eq!(report.root_type, Some(Type::Any));
}

#[test]
fn unknown_type_uses_the_configured_static_resolution_severity() {
    let mut checker = TypeChecker::with_config(TyConfig::dev_profile());
    let report = check("treat_as(value, MissingType)", &mut checker);

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

    let report = check("treat_as(button, Control)", &mut checker);

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
