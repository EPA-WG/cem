use cem_ml::schema::package_sources::builtin_schema_package_sources;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EXIT_OK: i32 = 0;
const EXIT_HARD_FAILURE: i32 = 1;

const CEM_ML_SCHEMA_URI: &str = "https://cem.dev/ns/cem-ml/1";
const CEM_ML_CONTENT_TYPE: &str = "application/cem";
const CEM_SCHEMA_URI: &str = "https://cem.dev/ns/schema/1";
const CEM_SCHEMA_CONTENT_TYPE: &str = "application/vnd.cem.schema+cem";
const CEM_SCHEMA_PACKAGE_URI: &str = "https://cem.dev/ns/schema-package/1";
const CEM_SCHEMA_PACKAGE_CONTENT_TYPE: &str = "application/vnd.cem.schema-package+cem";
const CEM_NATIVE_TEMPLATE_SCHEMA_URI: &str = "https://cem.dev/ns/template/cem-native/1";
const CEM_NATIVE_TEMPLATE_CONTENT_TYPE: &str = "application/vnd.cem.template+cem";
const CEM_TRANSFORM_SCHEMA_URI: &str = "https://cem.dev/ns/transform/cem/1";
const CEM_TRANSFORM_CONTENT_TYPE: &str = "application/vnd.cem.transform+cem";
const CEM_QL_SCHEMA_URI: &str = "https://cem.dev/ns/query/cem-ql/1";
const CEM_QL_CONTENT_TYPE: &str = "application/vnd.cem.query+cem-ql";
const JSON_SCHEMA_URI: &str = "https://cem.dev/ns/data/json/1";
const JSON_CONTENT_TYPE: &str = "application/json";
const YAML_SCHEMA_URI: &str = "https://cem.dev/ns/data/yaml/1";
const YAML_CONTENT_TYPE: &str = "application/yaml";
const YAML_TEXT_CONTENT_TYPE: &str = "text/yaml";
const YAML_LEGACY_CONTENT_TYPE: &str = "application/x-yaml";
const CSV_SCHEMA_URI: &str = "https://cem.dev/ns/data/csv/1";
const CSV_CONTENT_TYPE: &str = "text/csv";
const MARKDOWN_SCHEMA_URI: &str = "https://cem.dev/ns/data/markdown/1";
const MARKDOWN_COMMONMARK_CONTENT_TYPE: &str = "text/markdown; charset=utf-8; variant=CommonMark";
const MARKDOWN_GFM_CONTENT_TYPE: &str = "text/markdown; charset=utf-8; variant=GFM";
const MARKDOWN_UNKNOWN_VARIANT_CONTENT_TYPE: &str =
    "text/markdown; charset=utf-8; variant=CustomWiki";
const XML_SCHEMA_URI: &str = "https://cem.dev/ns/data/xml/1";
const XML_CONTENT_TYPE: &str = "application/xml";
const XML_TEXT_CONTENT_TYPE: &str = "text/xml; charset=utf-8";
const RELAX_NG_SCHEMA_URI: &str = "https://cem.dev/ns/data/relax-ng/1";
const RELAX_NG_XML_CONTENT_TYPE: &str = "application/relax-ng+xml";
const RELAX_NG_COMPACT_CONTENT_TYPE: &str = "application/relax-ng-compact-syntax";
const XHTML_SCHEMA_URI: &str = "https://cem.dev/ns/data/xhtml/1";
const XHTML_CONTENT_TYPE: &str = "application/xhtml+xml";
const SVG_SCHEMA_URI: &str = "https://cem.dev/ns/data/svg/1";
const SVG_CONTENT_TYPE: &str = "image/svg+xml";
const MATHML_SCHEMA_URI: &str = "https://cem.dev/ns/data/mathml/1";
const MATHML_CONTENT_TYPE: &str = "application/mathml+xml";
const MATHML_PRESENTATION_CONTENT_TYPE: &str = "application/mathml-presentation+xml";
const MATHML_CONTENT_CONTENT_TYPE: &str = "application/mathml-content+xml";
const XSLT_SCHEMA_URI: &str = "https://cem.dev/ns/transform/xslt/1";
const XSLT_CONTENT_TYPE: &str = "application/xslt+xml";
const XSLT_TEXT_CONTENT_TYPE: &str = "text/xsl";
const XSLT_CUSTOM_ELEMENT_CONTENT_TYPE: &str = "custom-element-xslt";
const HTML_SCHEMA_URI: &str = "https://cem.dev/ns/data/html/1";
const HTML_CONTENT_TYPE: &str = "text/html";
const HTML_WINDOWS_1252_CONTENT_TYPE: &str = "text/html; charset=windows-1252";
const CSS_SCHEMA_URI: &str = "https://cem.dev/ns/data/css/1";
const CSS_CONTENT_TYPE: &str = "text/css";
const CSS_ISO_8859_1_CONTENT_TYPE: &str = "text/css; charset=iso-8859-1";
const JSON_SCHEMA_SCHEMA_URI: &str = "https://cem.dev/ns/data/json-schema/1";
const JSON_SCHEMA_CONTENT_TYPE: &str = "application/schema+json";
const CEM_DOM_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/dom/1";
const CEM_DOM_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.dom+cem-bin";
const CEM_DOM_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.dom+json";
const CEM_AST_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/ast/1";
const CEM_AST_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.ast+cem-bin";
const CEM_AST_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.ast+json";
const CEM_EVENTS_PROJECTION_SCHEMA_URI: &str = "https://cem.dev/ns/projection/events/1";
const CEM_EVENTS_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.events+cem-bin";
const CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE: &str = "application/vnd.cem.events+json";

#[derive(Debug)]
struct ValidationExample {
    name: &'static str,
    path: &'static str,
    content_type: &'static str,
    schema_uri: &'static str,
    expected_exit: i32,
    expected_diagnostics: &'static [&'static str],
}

#[derive(Debug)]
struct DetailedValidationExample {
    name: &'static str,
    path: &'static str,
    content_type: &'static str,
    schema_uri: &'static str,
    expected: &'static [DiagnosticDetailExpectation],
}

#[derive(Debug)]
struct DiagnosticDetailExpectation {
    code: &'static str,
    severity: &'static str,
    behavior: &'static str,
    check_kind: &'static str,
    contract: &'static str,
}

#[derive(Debug)]
struct SchemaDefinitionDetailExample {
    name: &'static str,
    path: &'static str,
    expected: &'static [SchemaDefinitionDetailExpectation],
}

#[derive(Debug)]
struct SchemaDefinitionDetailExpectation {
    code: &'static str,
    severity: &'static str,
    check_kind: &'static str,
    attribute: &'static str,
    datatype_param: &'static str,
    param_value: &'static str,
}

const SCHEMA_PACKAGE_RUNTIME_CONSTRAINT_EXAMPLE_DIAGNOSTICS: &[(&str, &str)] = &[
    (
        "converter-from-to-required",
        "cem.schema_package.converter_check",
    ),
    (
        "cemt-template-identity-required",
        "cem.schema_package.converter_check",
    ),
    ("rust-symbol-required", "cem.schema_package.converter_check"),
    (
        "converter-planner-state-contract",
        "cem.schema_package.converter_check",
    ),
    (
        "converter-template-output-stage-contract",
        "cem.schema_package.converter_check",
    ),
    (
        "converter-endpoint-schema-content-type-match",
        "cem.schema_package.converter_check",
    ),
    (
        "cemt-native-fallback-reason",
        "cem.schema_package.converter_check",
    ),
    (
        "schema-source-readable",
        "cem.schema_package.schema_source_unreadable",
    ),
    (
        "schema-source-valid",
        "cem.schema_package.schema_source_invalid",
    ),
    (
        "schema-uri-consistency",
        "cem.schema_package.schema_uri_mismatch",
    ),
    (
        "schema-content-type-consistency",
        "cem.schema_package.schema_content_type_mismatch",
    ),
    (
        "schema-namespace-consistency",
        "cem.schema_package.schema_namespace_mismatch",
    ),
    (
        "artifact-output-stage-contract",
        "cem.schema_package.artifact_check",
    ),
    ("example-contract", "cem.schema_package.example_check"),
];

fn cem_ml(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args(args)
        .output()
        .expect("run cem-ml binary")
}

fn cem_ml_owned(args: &[String]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .args(args)
        .output()
        .expect("run cem-ml binary")
}

fn workspace_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is utf-8")
}

fn validate_example(example: &ValidationExample, path: &Path) -> Output {
    cem_ml(&[
        "validate",
        "--format",
        "json",
        "--content-type",
        example.content_type,
        "--schema",
        example.schema_uri,
        path.to_str().expect("example path is utf-8"),
    ])
}

fn test_temp_dir(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temporary test directory");
    root
}

fn write_test_file(path: &Path, source: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create temporary test file parent");
    }
    fs::write(path, source).expect("write temporary test file");
}

fn diagnostics(report: &serde_json::Value) -> &[serde_json::Value] {
    report["diagnostics"]
        .as_array()
        .expect("report diagnostics array")
}

fn has_diagnostic(report: &serde_json::Value, code: &str) -> bool {
    diagnostics(report)
        .iter()
        .any(|diagnostic| diagnostic["code"] == code)
}

fn find_diagnostic_detail<'a>(
    report: &'a serde_json::Value,
    expected: &DiagnosticDetailExpectation,
) -> Option<&'a serde_json::Value> {
    diagnostics(report).iter().find(|diagnostic| {
        diagnostic["code"] == expected.code
            && diagnostic["severity"] == expected.severity
            && diagnostic["details"]["behavior"] == expected.behavior
            && diagnostic["details"]["checkKind"] == expected.check_kind
            && diagnostic["details"]["contract"] == expected.contract
            && diagnostic["details"]["sourceRange"]["span"]["start"]
                .as_u64()
                .is_some()
            && diagnostic["sourceMap"]["frames"]
                .as_array()
                .is_some_and(|frames| !frames.is_empty())
    })
}

fn has_diagnostic_detail(
    report: &serde_json::Value,
    expected: &DiagnosticDetailExpectation,
) -> bool {
    find_diagnostic_detail(report, expected).is_some()
}

fn has_schema_definition_detail(
    report: &serde_json::Value,
    expected: &SchemaDefinitionDetailExpectation,
) -> bool {
    diagnostics(report).iter().any(|diagnostic| {
        diagnostic["code"] == expected.code
            && diagnostic["severity"] == expected.severity
            && diagnostic["details"]["checkKind"] == expected.check_kind
            && diagnostic["details"]["attribute"] == expected.attribute
            && diagnostic["details"]["datatypeParam"] == expected.datatype_param
            && diagnostic["details"]["paramName"] == expected.datatype_param
            && diagnostic["details"]["paramValue"] == expected.param_value
            && diagnostic["sourceMap"]["frames"]
                .as_array()
                .is_some_and(|frames| !frames.is_empty())
    })
}

fn assert_schema_package_runtime_constraint_example_coverage(examples: &[ValidationExample]) {
    let covered_diagnostics = examples
        .iter()
        .filter(|example| example.schema_uri == CEM_SCHEMA_PACKAGE_URI)
        .flat_map(|example| example.expected_diagnostics.iter().copied())
        .collect::<BTreeSet<_>>();

    for (constraint_kind, diagnostic_code) in SCHEMA_PACKAGE_RUNTIME_CONSTRAINT_EXAMPLE_DIAGNOSTICS
    {
        assert!(
            covered_diagnostics.contains(diagnostic_code),
            "schema-package runtime constraint `{constraint_kind}` needs a checked-in CLI validation example expecting `{diagnostic_code}`"
        );
    }
}

fn schema_package_manifest_paths() -> Vec<PathBuf> {
    let root = workspace_path("packages/cem_ml/schema-packages");
    let mut manifests = Vec::new();
    for entry in fs::read_dir(&root).expect("schema-packages directory exists") {
        let package_dir = entry.expect("schema package directory entry").path();
        let manifest = package_dir.join("v1/package.cem");
        if manifest.exists() {
            manifests.push(manifest);
        }
    }
    manifests.sort();
    manifests
}

fn schema_package_id_from_manifest_path(path: &Path) -> String {
    path.parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .expect("schema package manifest path has package id")
        .to_owned()
}

#[test]
fn schema_owned_examples_validate_through_cli() {
    let examples = [
        ValidationExample {
            name: "cem-ml basic",
            path: "packages/cem_ml/schema-packages/cem-ml/v1/examples/basic.cem",
            content_type: CEM_ML_CONTENT_TYPE,
            schema_uri: CEM_ML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-ml nested handoff",
            path: "packages/cem_ml/schema-packages/cem-ml/v1/examples/nested-handoff.cem",
            content_type: CEM_ML_CONTENT_TYPE,
            schema_uri: CEM_ML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["cem.handoff.child_parser_deferred"],
        },
        ValidationExample {
            name: "cem-ml invalid unclosed scope",
            path: "packages/cem_ml/schema-packages/cem-ml/v1/examples/invalid-unclosed-scope.cem",
            content_type: CEM_ML_CONTENT_TYPE,
            schema_uri: CEM_ML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema.unclosed_scope"],
        },
        ValidationExample {
            name: "schema basic",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/basic-schema.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "schema typed resource",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/typed-resource-schema.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "schema custom behavior",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "schema custom behavior strict",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema-strict.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "schema invalid unclosed scope",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-unclosed-schema.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema.unclosed_scope"],
        },
        ValidationExample {
            name: "schema invalid missing required attribute",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-missing-required-attribute.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_model.missing_required_attribute"],
        },
        ValidationExample {
            name: "schema invalid diagnostic behavior",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-diagnostic-behavior.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.unknown_diagnostic_behavior"],
        },
        ValidationExample {
            name: "schema invalid custom behavior unresolved function",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-unresolved-function.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.unresolved_behavior_function"],
        },
        ValidationExample {
            name: "schema invalid custom behavior select query",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-select-query.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_behavior.query_invalid"],
        },
        ValidationExample {
            name: "schema invalid custom behavior match query",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-match-query.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_behavior.query_invalid"],
        },
        ValidationExample {
            name: "schema invalid custom behavior argument type",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-argument-type.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_diagnostic_behavior_contract"],
        },
        ValidationExample {
            name: "schema invalid custom behavior signature",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-signature.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_diagnostic_behavior_contract"],
        },
        ValidationExample {
            name: "schema invalid custom behavior unsafe call",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-unsafe-call.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_behavior.function_failed"],
        },
        ValidationExample {
            name: "schema invalid custom behavior contracts",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-custom-behavior-contracts.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_diagnostic_behavior_contract"],
        },
        ValidationExample {
            name: "schema invalid datatype param length",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-length.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_datatype_param"],
        },
        ValidationExample {
            name: "schema invalid datatype param bound",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-bound.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_datatype_param"],
        },
        ValidationExample {
            name: "schema invalid datatype param pattern",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-pattern.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_datatype_param"],
        },
        ValidationExample {
            name: "schema invalid datatype param digits",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-digits.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_datatype_param"],
        },
        ValidationExample {
            name: "schema invalid datatype param uri/media",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-uri-media.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_datatype_param"],
        },
        ValidationExample {
            name: "schema invalid field contract child sequence",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-child-sequence.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_field_contract"],
        },
        ValidationExample {
            name: "schema invalid field contract presence",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-presence.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_field_contract"],
        },
        ValidationExample {
            name: "schema invalid field contract condition",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-condition.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_field_contract"],
        },
        ValidationExample {
            name: "schema invalid attribute default",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-attribute-default.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_definition.invalid_default_value"],
        },
        ValidationExample {
            name: "schema-package basic",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/basic-package.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "schema-package converter",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/converter-package.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "schema-package invalid unclosed scope",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-unclosed-package.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema.unclosed_scope"],
        },
        ValidationExample {
            name: "schema-package invalid missing required attribute",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-missing-required-attribute.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &[
                "cem.schema_model.missing_required_attribute",
                "cem.schema_package.package_check",
            ],
        },
        ValidationExample {
            name: "schema-package invalid primary content type",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-primary-content-type.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_package.content_type_conflict"],
        },
        ValidationExample {
            name: "schema-package invalid missing primary content type",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-primary-content-type-missing.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_package.content_type_conflict"],
        },
        ValidationExample {
            name: "schema-package invalid converter contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &[
                "cem.schema_package.converter_check",
                "cem.schema_model.invalid_attribute_value",
                "cem.schema_model.invalid_attribute_datatype_param",
            ],
        },
        ValidationExample {
            name: "schema-package invalid converter runtime constraints",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-runtime-constraints.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &[
                "cem.schema_model.invalid_attribute_type",
                "cem.schema_model.invalid_attribute_value",
                "cem.schema_package.converter_check",
            ],
        },
        ValidationExample {
            name: "schema-package invalid converter template contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-template-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_package.converter_check"],
        },
        ValidationExample {
            name: "schema-package invalid converter template unreadable",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-template-unreadable.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_package.converter_check"],
        },
        ValidationExample {
            name: "schema-package invalid artifact contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_package.artifact_check"],
        },
        ValidationExample {
            name: "schema-package invalid artifact layout",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-layout.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_package.artifact_check"],
        },
        ValidationExample {
            name: "schema-package invalid schema metadata",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-metadata.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &[
                "cem.schema_package.schema_uri_mismatch",
                "cem.schema_package.schema_content_type_mismatch",
                "cem.schema_package.schema_namespace_mismatch",
            ],
        },
        ValidationExample {
            name: "schema-package invalid schema source unreadable",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-source-unreadable.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_package.schema_source_unreadable"],
        },
        ValidationExample {
            name: "schema-package invalid schema source invalid",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-source-invalid.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_package.schema_source_invalid"],
        },
        ValidationExample {
            name: "schema-package invalid example contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-example-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &[
                "cem.schema_model.invalid_attribute_value",
                "cem.schema_package.example_check",
            ],
        },
        ValidationExample {
            name: "native-template basic",
            path: "packages/cem_ml/schema-packages/cem-native-template/v1/examples/basic-template.cem",
            content_type: CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
            schema_uri: CEM_NATIVE_TEMPLATE_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "native-template module",
            path: "packages/cem_ml/schema-packages/cem-native-template/v1/examples/module-template.cem",
            content_type: CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
            schema_uri: CEM_NATIVE_TEMPLATE_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "native-template invalid missing required attribute",
            path: "packages/cem_ml/schema-packages/cem-native-template/v1/examples/invalid-missing-required-attribute.cem",
            content_type: CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
            schema_uri: CEM_NATIVE_TEMPLATE_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_model.missing_required_attribute"],
        },
        ValidationExample {
            name: "transform-template basic",
            path: "packages/cem_ml/schema-packages/cem-transform/v1/examples/basic-transform.cemt",
            content_type: CEM_TRANSFORM_CONTENT_TYPE,
            schema_uri: CEM_TRANSFORM_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "transform-template module",
            path: "packages/cem_ml/schema-packages/cem-transform/v1/examples/module-transform.cemt",
            content_type: CEM_TRANSFORM_CONTENT_TYPE,
            schema_uri: CEM_TRANSFORM_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "transform-template function declarations",
            path: "packages/cem_ml/schema-packages/cem-transform/v1/examples/function-declarations.cemt",
            content_type: CEM_TRANSFORM_CONTENT_TYPE,
            schema_uri: CEM_TRANSFORM_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "transform-template invalid missing required attribute",
            path: "packages/cem_ml/schema-packages/cem-transform/v1/examples/invalid-missing-required-attribute.cemt",
            content_type: CEM_TRANSFORM_CONTENT_TYPE,
            schema_uri: CEM_TRANSFORM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_model.missing_required_attribute"],
        },
        ValidationExample {
            name: "transform-template invalid function missing category",
            path: "packages/cem_ml/schema-packages/cem-transform/v1/examples/invalid-function-missing-category.cemt",
            content_type: CEM_TRANSFORM_CONTENT_TYPE,
            schema_uri: CEM_TRANSFORM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_model.missing_required_attribute"],
        },
        ValidationExample {
            name: "transform-template invalid function missing contract metadata",
            path: "packages/cem_ml/schema-packages/cem-transform/v1/examples/invalid-function-missing-contract-metadata.cemt",
            content_type: CEM_TRANSFORM_CONTENT_TYPE,
            schema_uri: CEM_TRANSFORM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_model.missing_required_attribute"],
        },
        ValidationExample {
            name: "cem-ql basic",
            path: "packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql",
            content_type: CEM_QL_CONTENT_TYPE,
            schema_uri: CEM_QL_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-ql module",
            path: "packages/cem_ml/schema-packages/cem-ql/v1/examples/module-query.cemql",
            content_type: CEM_QL_CONTENT_TYPE,
            schema_uri: CEM_QL_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-ql invalid parse",
            path: "packages/cem_ml/schema-packages/cem-ql/v1/examples/invalid-parse.cemql",
            content_type: CEM_QL_CONTENT_TYPE,
            schema_uri: CEM_QL_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.ql.parse_error"],
        },
        ValidationExample {
            name: "cem-ql invalid missing module",
            path: "packages/cem_ml/schema-packages/cem-ql/v1/examples/invalid-missing-module.cemql",
            content_type: CEM_QL_CONTENT_TYPE,
            schema_uri: CEM_QL_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.ql.module_uri_missing"],
        },
        ValidationExample {
            name: "json basic object",
            path: "packages/cem_ml/schema-packages/json/v1/examples/basic-object.json",
            content_type: JSON_CONTENT_TYPE,
            schema_uri: JSON_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "json nested data",
            path: "packages/cem_ml/schema-packages/json/v1/examples/nested-data.json",
            content_type: JSON_CONTENT_TYPE,
            schema_uri: JSON_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "json invalid trailing comma",
            path: "packages/cem_ml/schema-packages/json/v1/examples/invalid-trailing-comma.json",
            content_type: JSON_CONTENT_TYPE,
            schema_uri: JSON_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.json.parse_error"],
        },
        ValidationExample {
            name: "yaml basic document",
            path: "packages/cem_ml/schema-packages/yaml/v1/examples/basic-document.yaml",
            content_type: YAML_CONTENT_TYPE,
            schema_uri: YAML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "yaml nested stream",
            path: "packages/cem_ml/schema-packages/yaml/v1/examples/nested-stream.yml",
            content_type: YAML_TEXT_CONTENT_TYPE,
            schema_uri: YAML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "yaml invalid parse",
            path: "packages/cem_ml/schema-packages/yaml/v1/examples/invalid-parse.yaml",
            content_type: YAML_CONTENT_TYPE,
            schema_uri: YAML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.yaml.parse_error"],
        },
        ValidationExample {
            name: "yaml invalid unsafe tag",
            path: "packages/cem_ml/schema-packages/yaml/v1/examples/invalid-unsafe-tag.yaml",
            content_type: YAML_LEGACY_CONTENT_TYPE,
            schema_uri: YAML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.yaml.unsafe_tag"],
        },
        ValidationExample {
            name: "csv basic table",
            path: "packages/cem_ml/schema-packages/csv/v1/examples/basic-table.csv",
            content_type: CSV_CONTENT_TYPE,
            schema_uri: CSV_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "csv quoted fields",
            path: "packages/cem_ml/schema-packages/csv/v1/examples/quoted-fields.csv",
            content_type: CSV_CONTENT_TYPE,
            schema_uri: CSV_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "csv invalid unclosed quote",
            path: "packages/cem_ml/schema-packages/csv/v1/examples/invalid-unclosed-quote.csv",
            content_type: CSV_CONTENT_TYPE,
            schema_uri: CSV_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.csv.unclosed_quote"],
        },
        ValidationExample {
            name: "csv ragged row",
            path: "packages/cem_ml/schema-packages/csv/v1/examples/ragged-row.csv",
            content_type: CSV_CONTENT_TYPE,
            schema_uri: CSV_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["cem.csv.inconsistent_field_count"],
        },
        ValidationExample {
            name: "markdown basic document",
            path: "packages/cem_ml/schema-packages/markdown/v1/examples/basic-document.md",
            content_type: MARKDOWN_COMMONMARK_CONTENT_TYPE,
            schema_uri: MARKDOWN_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "markdown gfm worklog",
            path: "packages/cem_ml/schema-packages/markdown/v1/examples/gfm-worklog.md",
            content_type: MARKDOWN_GFM_CONTENT_TYPE,
            schema_uri: MARKDOWN_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "markdown invalid embedded html",
            path: "packages/cem_ml/schema-packages/markdown/v1/examples/invalid-embedded-html.md",
            content_type: MARKDOWN_COMMONMARK_CONTENT_TYPE,
            schema_uri: MARKDOWN_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.markdown.embedded_html_rejected"],
        },
        ValidationExample {
            name: "markdown unknown variant",
            path: "packages/cem_ml/schema-packages/markdown/v1/examples/unknown-variant.md",
            content_type: MARKDOWN_UNKNOWN_VARIANT_CONTENT_TYPE,
            schema_uri: MARKDOWN_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["cem.markdown.unknown_variant"],
        },
        ValidationExample {
            name: "xml basic document",
            path: "packages/cem_ml/schema-packages/xml/v1/examples/basic-document.xml",
            content_type: XML_CONTENT_TYPE,
            schema_uri: XML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "xml namespaced document",
            path: "packages/cem_ml/schema-packages/xml/v1/examples/namespaced-document.xml",
            content_type: XML_TEXT_CONTENT_TYPE,
            schema_uri: XML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "xml invalid mismatched tag",
            path: "packages/cem_ml/schema-packages/xml/v1/examples/invalid-mismatched-tag.xml",
            content_type: XML_CONTENT_TYPE,
            schema_uri: XML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xml.parse_error"],
        },
        ValidationExample {
            name: "xml invalid unbound prefix",
            path: "packages/cem_ml/schema-packages/xml/v1/examples/invalid-unbound-prefix.xml",
            content_type: XML_CONTENT_TYPE,
            schema_uri: XML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xml.unbound_namespace_prefix"],
        },
        ValidationExample {
            name: "xml invalid doctype",
            path: "packages/cem_ml/schema-packages/xml/v1/examples/invalid-doctype.xml",
            content_type: XML_CONTENT_TYPE,
            schema_uri: XML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xml.dtd_rejected"],
        },
        ValidationExample {
            name: "relax-ng basic rng",
            path: "packages/cem_ml/schema-packages/relax-ng/v1/examples/basic-schema.rng",
            content_type: RELAX_NG_XML_CONTENT_TYPE,
            schema_uri: RELAX_NG_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "relax-ng datatype rng",
            path: "packages/cem_ml/schema-packages/relax-ng/v1/examples/datatype-schema.rng",
            content_type: RELAX_NG_XML_CONTENT_TYPE,
            schema_uri: RELAX_NG_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "relax-ng basic rnc",
            path: "packages/cem_ml/schema-packages/relax-ng/v1/examples/basic-schema.rnc",
            content_type: RELAX_NG_COMPACT_CONTENT_TYPE,
            schema_uri: RELAX_NG_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "relax-ng invalid missing start",
            path: "packages/cem_ml/schema-packages/relax-ng/v1/examples/invalid-missing-start.rng",
            content_type: RELAX_NG_XML_CONTENT_TYPE,
            schema_uri: RELAX_NG_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.relax_ng.missing_start"],
        },
        ValidationExample {
            name: "relax-ng invalid unknown element",
            path: "packages/cem_ml/schema-packages/relax-ng/v1/examples/invalid-unknown-element.rng",
            content_type: RELAX_NG_XML_CONTENT_TYPE,
            schema_uri: RELAX_NG_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.relax_ng.unknown_element"],
        },
        ValidationExample {
            name: "relax-ng invalid unclosed compact",
            path: "packages/cem_ml/schema-packages/relax-ng/v1/examples/invalid-unclosed-compact.rnc",
            content_type: RELAX_NG_COMPACT_CONTENT_TYPE,
            schema_uri: RELAX_NG_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.relax_ng.compact_parse_error"],
        },
        ValidationExample {
            name: "xhtml basic document",
            path: "packages/cem_ml/schema-packages/xhtml/v1/examples/basic-document.xhtml",
            content_type: XHTML_CONTENT_TYPE,
            schema_uri: XHTML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "xhtml form page",
            path: "packages/cem_ml/schema-packages/xhtml/v1/examples/form-page.xhtml",
            content_type: XHTML_CONTENT_TYPE,
            schema_uri: XHTML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "xhtml invalid missing namespace",
            path: "packages/cem_ml/schema-packages/xhtml/v1/examples/invalid-missing-namespace.xhtml",
            content_type: XHTML_CONTENT_TYPE,
            schema_uri: XHTML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xhtml.namespace_missing"],
        },
        ValidationExample {
            name: "xhtml invalid body before head",
            path: "packages/cem_ml/schema-packages/xhtml/v1/examples/invalid-body-before-head.xhtml",
            content_type: XHTML_CONTENT_TYPE,
            schema_uri: XHTML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xhtml.head_body_order"],
        },
        ValidationExample {
            name: "xhtml invalid not well formed",
            path: "packages/cem_ml/schema-packages/xhtml/v1/examples/invalid-not-well-formed.xhtml",
            content_type: XHTML_CONTENT_TYPE,
            schema_uri: XHTML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xhtml.not_well_formed_xml"],
        },
        ValidationExample {
            name: "svg basic icon",
            path: "packages/cem_ml/schema-packages/svg/v1/examples/basic-icon.svg",
            content_type: SVG_CONTENT_TYPE,
            schema_uri: SVG_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "svg bar chart",
            path: "packages/cem_ml/schema-packages/svg/v1/examples/bar-chart.svg",
            content_type: SVG_CONTENT_TYPE,
            schema_uri: SVG_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "svg unnamed icon",
            path: "packages/cem_ml/schema-packages/svg/v1/examples/unnamed-icon.svg",
            content_type: SVG_CONTENT_TYPE,
            schema_uri: SVG_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["cem.svg.accessible_name_missing"],
        },
        ValidationExample {
            name: "svg invalid missing namespace",
            path: "packages/cem_ml/schema-packages/svg/v1/examples/invalid-missing-namespace.svg",
            content_type: SVG_CONTENT_TYPE,
            schema_uri: SVG_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.svg.namespace_missing"],
        },
        ValidationExample {
            name: "svg invalid script",
            path: "packages/cem_ml/schema-packages/svg/v1/examples/invalid-script.svg",
            content_type: SVG_CONTENT_TYPE,
            schema_uri: SVG_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.svg.script_rejected"],
        },
        ValidationExample {
            name: "svg invalid external image",
            path: "packages/cem_ml/schema-packages/svg/v1/examples/invalid-external-image.svg",
            content_type: SVG_CONTENT_TYPE,
            schema_uri: SVG_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.svg.external_resource_rejected"],
        },
        ValidationExample {
            name: "mathml basic presentation",
            path: "packages/cem_ml/schema-packages/mathml/v1/examples/basic-presentation.mml",
            content_type: MATHML_PRESENTATION_CONTENT_TYPE,
            schema_uri: MATHML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "mathml content expression",
            path: "packages/cem_ml/schema-packages/mathml/v1/examples/content-expression.mathml",
            content_type: MATHML_CONTENT_CONTENT_TYPE,
            schema_uri: MATHML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "mathml external annotation",
            path: "packages/cem_ml/schema-packages/mathml/v1/examples/semantics-external-annotation.mml",
            content_type: MATHML_CONTENT_TYPE,
            schema_uri: MATHML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["cem.mathml.external_annotation_rejected"],
        },
        ValidationExample {
            name: "mathml invalid missing namespace",
            path: "packages/cem_ml/schema-packages/mathml/v1/examples/invalid-missing-namespace.mml",
            content_type: MATHML_CONTENT_TYPE,
            schema_uri: MATHML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.mathml.namespace_missing"],
        },
        ValidationExample {
            name: "mathml invalid root not math",
            path: "packages/cem_ml/schema-packages/mathml/v1/examples/invalid-root-not-math.mml",
            content_type: MATHML_CONTENT_TYPE,
            schema_uri: MATHML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.mathml.root_not_math"],
        },
        ValidationExample {
            name: "mathml invalid content profile",
            path: "packages/cem_ml/schema-packages/mathml/v1/examples/invalid-content-profile-presentation-only.mml",
            content_type: MATHML_CONTENT_CONTENT_TYPE,
            schema_uri: MATHML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.mathml.malformed_expression"],
        },
        ValidationExample {
            name: "mathml invalid not well formed",
            path: "packages/cem_ml/schema-packages/mathml/v1/examples/invalid-not-well-formed.mml",
            content_type: MATHML_CONTENT_TYPE,
            schema_uri: MATHML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.mathml.not_well_formed_xml"],
        },
        ValidationExample {
            name: "xslt basic stylesheet",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/basic-stylesheet.xsl",
            content_type: XSLT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "xslt named template",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/named-template.xslt",
            content_type: XSLT_TEXT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "xslt legacy custom element stylesheet",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-stylesheet.xsl",
            content_type: XSLT_CUSTOM_ELEMENT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "xslt legacy custom element fragment",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/legacy-custom-element-fragment.html",
            content_type: XSLT_CUSTOM_ELEMENT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "xslt unsupported extension warning",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/unsupported-extension-warning.xsl",
            content_type: XSLT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["legacy_xslt.unsupported_construct"],
        },
        ValidationExample {
            name: "xslt invalid missing namespace",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-namespace.xsl",
            content_type: XSLT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xslt.namespace_missing"],
        },
        ValidationExample {
            name: "xslt invalid missing version",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-version.xsl",
            content_type: XSLT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xslt.version_missing"],
        },
        ValidationExample {
            name: "xslt invalid external include",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/invalid-external-include.xsl",
            content_type: XSLT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xslt.external_uri_rejected"],
        },
        ValidationExample {
            name: "xslt invalid missing entrypoint",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/invalid-missing-entrypoint.xsl",
            content_type: XSLT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xslt.entrypoint_missing"],
        },
        ValidationExample {
            name: "xslt invalid not well formed",
            path: "packages/cem_ml/schema-packages/xslt/v1/examples/invalid-not-well-formed.xsl",
            content_type: XSLT_CONTENT_TYPE,
            schema_uri: XSLT_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.xslt.not_well_formed_xml"],
        },
        ValidationExample {
            name: "html basic document",
            path: "packages/cem_ml/schema-packages/html/v1/examples/basic-document.html",
            content_type: HTML_CONTENT_TYPE,
            schema_uri: HTML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "html fragment",
            path: "packages/cem_ml/schema-packages/html/v1/examples/fragment.html",
            content_type: HTML_CONTENT_TYPE,
            schema_uri: HTML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "html svg mathml islands",
            path: "packages/cem_ml/schema-packages/html/v1/examples/svg-mathml-islands.html",
            content_type: HTML_CONTENT_TYPE,
            schema_uri: HTML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "html invalid script",
            path: "packages/cem_ml/schema-packages/html/v1/examples/invalid-script.html",
            content_type: HTML_CONTENT_TYPE,
            schema_uri: HTML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.html.script_rejected"],
        },
        ValidationExample {
            name: "html invalid external resource",
            path: "packages/cem_ml/schema-packages/html/v1/examples/invalid-external-resource.html",
            content_type: HTML_CONTENT_TYPE,
            schema_uri: HTML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.html.external_resource_rejected"],
        },
        ValidationExample {
            name: "html invalid custom element",
            path: "packages/cem_ml/schema-packages/html/v1/examples/invalid-custom-element.html",
            content_type: HTML_CONTENT_TYPE,
            schema_uri: HTML_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.html.custom_element_name_invalid"],
        },
        ValidationExample {
            name: "html encoding conflict",
            path: "packages/cem_ml/schema-packages/html/v1/examples/encoding-conflict.html",
            content_type: HTML_WINDOWS_1252_CONTENT_TYPE,
            schema_uri: HTML_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["cem.html.encoding_conflict"],
        },
        ValidationExample {
            name: "css basic stylesheet",
            path: "packages/cem_ml/schema-packages/css/v1/examples/basic-stylesheet.css",
            content_type: CSS_CONTENT_TYPE,
            schema_uri: CSS_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "css scoped component",
            path: "packages/cem_ml/schema-packages/css/v1/examples/scoped-component.css",
            content_type: CSS_CONTENT_TYPE,
            schema_uri: CSS_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "css style attribute",
            path: "packages/cem_ml/schema-packages/css/v1/examples/style-attribute.css",
            content_type: CSS_CONTENT_TYPE,
            schema_uri: CSS_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "css invalid import",
            path: "packages/cem_ml/schema-packages/css/v1/examples/invalid-import.css",
            content_type: CSS_CONTENT_TYPE,
            schema_uri: CSS_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.css.import_rejected"],
        },
        ValidationExample {
            name: "css invalid url",
            path: "packages/cem_ml/schema-packages/css/v1/examples/invalid-url.css",
            content_type: CSS_CONTENT_TYPE,
            schema_uri: CSS_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.css.url_rejected"],
        },
        ValidationExample {
            name: "css invalid token",
            path: "packages/cem_ml/schema-packages/css/v1/examples/invalid-token.css",
            content_type: CSS_CONTENT_TYPE,
            schema_uri: CSS_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.css.invalid_token"],
        },
        ValidationExample {
            name: "css invalid declaration",
            path: "packages/cem_ml/schema-packages/css/v1/examples/invalid-declaration.css",
            content_type: CSS_CONTENT_TYPE,
            schema_uri: CSS_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["cem.css.invalid_declaration"],
        },
        ValidationExample {
            name: "css encoding conflict",
            path: "packages/cem_ml/schema-packages/css/v1/examples/encoding-conflict.css",
            content_type: CSS_ISO_8859_1_CONTENT_TYPE,
            schema_uri: CSS_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &["cem.css.encoding_conflict"],
        },
        ValidationExample {
            name: "json-schema basic",
            path: "packages/cem_ml/schema-packages/json-schema/v1/examples/basic-schema.schema.json",
            content_type: JSON_SCHEMA_CONTENT_TYPE,
            schema_uri: JSON_SCHEMA_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "json-schema catalog",
            path: "packages/cem_ml/schema-packages/json-schema/v1/examples/catalog-schema.schema.json",
            content_type: JSON_SCHEMA_CONTENT_TYPE,
            schema_uri: JSON_SCHEMA_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "json-schema invalid unsupported dialect",
            path: "packages/cem_ml/schema-packages/json-schema/v1/examples/invalid-unsupported-dialect.schema.json",
            content_type: JSON_SCHEMA_CONTENT_TYPE,
            schema_uri: JSON_SCHEMA_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.json_schema.unsupported_dialect"],
        },
        ValidationExample {
            name: "json-schema invalid parse",
            path: "packages/cem_ml/schema-packages/json-schema/v1/examples/invalid-parse.schema.json",
            content_type: JSON_SCHEMA_CONTENT_TYPE,
            schema_uri: JSON_SCHEMA_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.json_schema.parse_error"],
        },
        ValidationExample {
            name: "cem-dom binary basic",
            path: "packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/basic-dom.cem-bin",
            content_type: CEM_DOM_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_DOM_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-dom json basic",
            path: "packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/basic-dom.dom.json",
            content_type: CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_DOM_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-dom json nested",
            path: "packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/nested-dom.dom.json",
            content_type: CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_DOM_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-dom json invalid kind",
            path: "packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/invalid-kind.dom.json",
            content_type: CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_DOM_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.projection.dom.json_shape"],
        },
        ValidationExample {
            name: "cem-dom binary invalid magic",
            path: "packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/invalid-binary.cem-bin",
            content_type: CEM_DOM_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_DOM_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.projection.dom.binary_magic"],
        },
        ValidationExample {
            name: "cem-ast binary basic",
            path: "packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/basic-ast.cem-bin",
            content_type: CEM_AST_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_AST_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-ast json basic",
            path: "packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/basic-ast.ast.json",
            content_type: CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_AST_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-ast json nested",
            path: "packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/nested-ast.ast.json",
            content_type: CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_AST_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-ast json invalid kind",
            path: "packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/invalid-kind.ast.json",
            content_type: CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_AST_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.projection.ast.json_shape"],
        },
        ValidationExample {
            name: "cem-ast binary invalid magic",
            path: "packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/invalid-binary.cem-bin",
            content_type: CEM_AST_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_AST_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.projection.ast.binary_magic"],
        },
        ValidationExample {
            name: "cem-events binary basic",
            path: "packages/cem_ml/schema-packages/cem-events-projection/v1/examples/basic-events.cem-bin",
            content_type: CEM_EVENTS_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_EVENTS_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-events json basic",
            path: "packages/cem_ml/schema-packages/cem-events-projection/v1/examples/basic-events.events.json",
            content_type: CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_EVENTS_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-events json nested",
            path: "packages/cem_ml/schema-packages/cem-events-projection/v1/examples/nested-events.events.json",
            content_type: CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_EVENTS_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_OK,
            expected_diagnostics: &[],
        },
        ValidationExample {
            name: "cem-events json invalid kind",
            path: "packages/cem_ml/schema-packages/cem-events-projection/v1/examples/invalid-kind.events.json",
            content_type: CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_EVENTS_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.projection.events.json_shape"],
        },
        ValidationExample {
            name: "cem-events binary invalid magic",
            path: "packages/cem_ml/schema-packages/cem-events-projection/v1/examples/invalid-binary.cem-bin",
            content_type: CEM_EVENTS_PROJECTION_CONTENT_TYPE,
            schema_uri: CEM_EVENTS_PROJECTION_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.projection.events.binary_magic"],
        },
    ];

    assert_schema_package_runtime_constraint_example_coverage(&examples);

    for example in examples {
        let path = workspace_path(example.path);
        assert!(
            path.exists(),
            "schema validation example `{}` is missing at {}",
            example.name,
            path.display()
        );

        let output = validate_example(&example, &path);
        assert_eq!(
            output.status.code(),
            Some(example.expected_exit),
            "{} stderr:\n{}",
            example.name,
            stderr(&output)
        );
        assert!(
            stderr(&output).trim().is_empty(),
            "{} stderr must stay empty:\n{}",
            example.name,
            stderr(&output)
        );

        let report: serde_json::Value = serde_json::from_str(stdout(&output).trim())
            .unwrap_or_else(|err| panic!("{} stdout is validation JSON: {err}", example.name));
        let hard_violations = report["summary"]["hardViolationCount"]
            .as_u64()
            .expect("hardViolationCount is numeric");
        if example.expected_exit == EXIT_OK {
            assert_eq!(hard_violations, 0, "{} hard violation count", example.name);
        } else {
            assert!(
                hard_violations > 0,
                "{} expected at least one hard violation",
                example.name
            );
        }

        for expected in example.expected_diagnostics {
            assert!(
                has_diagnostic(&report, expected),
                "{} expected diagnostic `{}` in {}",
                example.name,
                expected,
                stdout(&output)
            );
        }
    }
}

#[test]
fn schema_package_schema_source_changes_cli_validation_model() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/custom-validation/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.custom-validation+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.div.missing_marker";

    let root = test_temp_dir("cem-ml-cli-schema-package-validation-model");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/custom-validation.cem");
    let input_path = root.join("examples/missing-name.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="custom-validation" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/custom-validation/1"
        @source="schema/custom-validation.cem"
    }
    {content-type @value="application/vnd.example.custom-validation+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/custom-validation/1"}
}
"#,
    );
    let required_schema = r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="custom-validation" @namespace="https://example.test/ns/custom-validation/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.custom-validation+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/custom-validation/1" @role="schema"}
    }
    {elements |
        {element @name="div" @optional-attributes="marker"}
    }
    {attributes |
        {attribute @name="marker" @type="schema:string"}
    }
    {field-contracts |
        {field-contract
            @name="div-marker"
            @target="div"
            @required-attributes="marker"
            @diagnostic="example.div.missing_marker"
            @behavior="schema:required-fields"
            @check-kind="required-fields"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.div.missing_marker"
            @severity="error"
            @behavior="schema:required-fields"
            @message="Div marker must be declared"
        }
    }
}
"#;
    write_test_file(&schema_path, required_schema);
    write_test_file(
        &input_path,
        r#"@doc cem-ml 1

{div}
"#,
    );

    let validate_args = || {
        vec![
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ]
    };

    let required_output = cem_ml_owned(&validate_args());
    assert_eq!(
        required_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "required schema stderr:\n{}",
        stderr(&required_output)
    );
    assert!(
        stderr(&required_output).trim().is_empty(),
        "required schema stderr must stay empty:\n{}",
        stderr(&required_output)
    );
    let required_report: serde_json::Value =
        serde_json::from_str(stdout(&required_output).trim()).expect("required report is JSON");
    assert!(
        has_diagnostic(&required_report, CUSTOM_DIAGNOSTIC),
        "expected `{CUSTOM_DIAGNOSTIC}` after required schema:\n{}",
        stdout(&required_output)
    );

    write_test_file(
        &schema_path,
        &required_schema.replace(
            r#"@required-attributes="marker""#,
            r#"@optional-attributes="marker""#,
        ),
    );

    let optional_output = cem_ml_owned(&validate_args());
    assert_eq!(
        optional_output.status.code(),
        Some(EXIT_OK),
        "optional schema stderr:\n{}",
        stderr(&optional_output)
    );
    assert!(
        stderr(&optional_output).trim().is_empty(),
        "optional schema stderr must stay empty:\n{}",
        stderr(&optional_output)
    );
    let optional_report: serde_json::Value =
        serde_json::from_str(stdout(&optional_output).trim()).expect("optional report is JSON");
    assert_eq!(
        optional_report["summary"]["hardViolationCount"].as_u64(),
        Some(0),
        "optional schema hard violations:\n{}",
        stdout(&optional_output)
    );
    assert!(
        !has_diagnostic(&optional_report, CUSTOM_DIAGNOSTIC),
        "`{CUSTOM_DIAGNOSTIC}` should disappear after schema source mutation:\n{}",
        stdout(&optional_output)
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_pattern_datatype_param_emits_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/pattern-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.pattern-runtime+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.item.invalid_code";

    let root = test_temp_dir("cem-ml-cli-pattern-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/pattern-runtime.cem");
    let input_path = root.join("examples/invalid-code.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="pattern-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/pattern-runtime/1"
        @source="schema/pattern-runtime.cem"
    }
    {content-type @value="application/vnd.example.pattern-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/pattern-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="pattern-runtime" @namespace="https://example.test/ns/pattern-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.pattern-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/pattern-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="item" @optional-attributes="code"}
    }
    {attributes |
        {attribute
            @name="code"
            @type="schema:string"
            @pattern="[A-Z][A-Z0-9-]*"
            @datatype-param-diagnostic="example.item.invalid_code"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item.invalid_code"
            @severity="error"
            @behavior="schema:datatype-param"
            @message="Item code must match the declared pattern"
        }
    }
}
"#,
    );
    write_test_file(
        &input_path,
        r#"@doc cem-ml 1

{item @code="bad_code"}
"#,
    );

    let output = cem_ml_owned(&[
        "validate".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
        "--schema-package".to_owned(),
        manifest_path.to_string_lossy().into_owned(),
        "--content-type".to_owned(),
        CUSTOM_CONTENT_TYPE.to_owned(),
        "--schema".to_owned(),
        CUSTOM_SCHEMA_URI.to_owned(),
        input_path.to_string_lossy().into_owned(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "pattern runtime stderr:\n{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).trim().is_empty(),
        "pattern runtime stderr must stay empty:\n{}",
        stderr(&output)
    );
    let report: serde_json::Value =
        serde_json::from_str(stdout(&output).trim()).expect("pattern runtime report is JSON");
    let diagnostic = diagnostics(&report)
        .iter()
        .find(|diagnostic| diagnostic["code"] == CUSTOM_DIAGNOSTIC)
        .unwrap_or_else(|| {
            panic!(
                "expected `{CUSTOM_DIAGNOSTIC}` in pattern runtime report:\n{}",
                stdout(&output)
            )
        });
    let details = &diagnostic["details"];
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(details["behavior"], "schema:datatype-param");
    assert_eq!(details["checkKind"], "datatype-param:pattern");
    assert_eq!(details["contract"], "attribute-datatype-param:code:pattern");
    assert_eq!(details["datatypeParam"], "pattern");
    assert_eq!(details["pattern"], "[A-Z][A-Z0-9-]*");
    assert_eq!(details["actualValue"], "bad_code");
    assert_eq!(details["invalidFields"], serde_json::json!(["code"]));
    assert!(details["sourceRange"]["span"]["start"].is_u64());
    assert!(diagnostic["sourceMap"]["frames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_field_dependency_forbidden_variants_emit_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/dependency-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.dependency-runtime+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.asset.dependency";

    let root = test_temp_dir("cem-ml-cli-field-dependency-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/dependency-runtime.cem");
    let valid_input_path = root.join("examples/valid-dependencies.cem");
    let invalid_input_path = root.join("examples/invalid-dependencies.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="dependency-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/dependency-runtime/1"
        @source="schema/dependency-runtime.cem"
    }
    {content-type @value="application/vnd.example.dependency-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/dependency-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="dependency-runtime" @namespace="https://example.test/ns/dependency-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.dependency-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/dependency-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="div" @optional-attributes="class id title role" @children="*"}
        {element @name="section" @optional-attributes="class id title role" @children="*"}
        {element @name="span" @optional-attributes="class id title role" @children="*"}
        {element @name="article" @optional-attributes="class id title role" @children="*"}
    }
    {attributes |
        {attribute @name="class" @type="schema:string"}
        {attribute @name="id" @type="schema:string"}
        {attribute @name="title" @type="schema:string"}
        {attribute @name="role" @type="schema:string"}
    }
    {field-contracts |
        {field-contract
            @name="class-title-id"
            @target="div"
            @when-present-attributes="class title"
            @required-attributes="id"
            @diagnostic="example.asset.dependency"
            @behavior="schema:field-dependency"
            @check-kind="dependent-required-fields"
        }
        {field-contract
            @name="remote-section-role"
            @target="section"
            @when-attribute="class"
            @when-values="remote"
            @when-present-attributes="id title"
            @required-attributes="role"
            @diagnostic="example.asset.dependency"
            @behavior="schema:field-dependency"
            @check-kind="dependent-required-fields"
        }
        {field-contract
            @name="remote-title-forbidden"
            @target="span"
            @when-attribute="class"
            @when-values="remote"
            @forbidden-attributes="title"
            @diagnostic="example.asset.dependency"
            @behavior="schema:field-dependency"
            @check-kind="dependent-forbidden-fields"
        }
        {field-contract
            @name="class-blocked-role"
            @target="span"
            @when-present-attributes="class"
            @forbidden-attribute-values="role=blocked"
            @diagnostic="example.asset.dependency"
            @behavior="schema:field-dependency"
            @check-kind="dependent-forbidden-values"
        }
        {field-contract
            @name="title-without-role-id"
            @target="article"
            @when-present-attributes="title"
            @when-absent-attributes="role"
            @required-attributes="id"
            @diagnostic="example.asset.dependency"
            @behavior="schema:field-dependency"
            @check-kind="dependent-required-fields"
        }
        {field-contract
            @name="article-section-title"
            @target="article"
            @when-present-children="section"
            @when-absent-children="span"
            @required-attributes="title"
            @diagnostic="example.asset.dependency"
            @behavior="schema:field-dependency"
            @check-kind="child-gated-dependent-required-fields"
        }
        {field-contract
            @name="article-section-role-forbidden"
            @target="article"
            @when-present-children="section"
            @when-absent-children="span"
            @forbidden-attributes="role"
            @diagnostic="example.asset.dependency"
            @behavior="schema:field-dependency"
            @check-kind="child-gated-dependent-forbidden-fields"
        }
        {field-contract
            @name="article-section-blocked-class"
            @target="article"
            @when-present-children="section"
            @when-absent-children="span"
            @forbidden-attribute-values="class=blocked"
            @diagnostic="example.asset.dependency"
            @behavior="schema:field-dependency"
            @check-kind="child-gated-dependent-forbidden-values"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.asset.dependency"
            @severity="error"
            @behavior="schema:field-dependency"
            @message="Asset dependency constraints must hold"
        }
    }
}
"#,
    );
    write_test_file(
        &valid_input_path,
        r#"@doc cem-ml 1

{div @class="local"}
{div @class="local" @title="ready" @id="item-1"}
{section @class="remote" @title="card"}
{section @class="remote" @title="card" @id="section-1" @role="region"}
{span @class="remote" @role="open"}
{article @title="card" @id="article-1"}
{article @title="card" @role="note"}
{article @title="card" @id="article-2" | {section}}
{article | {section} {span}}
{article @role="note" | {section} {span}}
{article @class="blocked" | {section} {span}}
"#,
    );
    write_test_file(
        &invalid_input_path,
        r#"@doc cem-ml 1

{div @class="local" @title="needs-id"}
{section @class="remote" @title="card" @id="section-1"}
{span @class="remote" @title="debug" @role="blocked"}
{article @title="card"}
{article | {section}}
{article @title="card" @id="article-3" @role="note" | {section}}
{article @title="card" @id="article-4" @class="blocked" | {section}}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let valid_output = validate(&valid_input_path);
    assert_eq!(
        valid_output.status.code(),
        Some(EXIT_OK),
        "field dependency valid stderr:\n{}",
        stderr(&valid_output)
    );
    assert!(
        stderr(&valid_output).trim().is_empty(),
        "field dependency valid stderr must stay empty:\n{}",
        stderr(&valid_output)
    );
    let valid_report: serde_json::Value = serde_json::from_str(stdout(&valid_output).trim())
        .expect("field dependency valid report is JSON");
    assert!(
        !has_diagnostic(&valid_report, CUSTOM_DIAGNOSTIC),
        "`{CUSTOM_DIAGNOSTIC}` should not be emitted for valid field dependencies:\n{}",
        stdout(&valid_output)
    );

    let output = validate(&invalid_input_path);
    assert_eq!(
        output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "field dependency runtime stderr:\n{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).trim().is_empty(),
        "field dependency runtime stderr must stay empty:\n{}",
        stderr(&output)
    );
    let report: serde_json::Value = serde_json::from_str(stdout(&output).trim())
        .expect("field dependency runtime report is JSON");
    for expected in [
        DiagnosticDetailExpectation {
            code: CUSTOM_DIAGNOSTIC,
            severity: "error",
            behavior: "schema:field-dependency",
            check_kind: "dependent-required-fields",
            contract: "class-title-id",
        },
        DiagnosticDetailExpectation {
            code: CUSTOM_DIAGNOSTIC,
            severity: "error",
            behavior: "schema:field-dependency",
            check_kind: "dependent-required-fields",
            contract: "remote-section-role",
        },
        DiagnosticDetailExpectation {
            code: CUSTOM_DIAGNOSTIC,
            severity: "error",
            behavior: "schema:field-dependency",
            check_kind: "dependent-forbidden-fields",
            contract: "remote-title-forbidden",
        },
        DiagnosticDetailExpectation {
            code: CUSTOM_DIAGNOSTIC,
            severity: "error",
            behavior: "schema:field-dependency",
            check_kind: "dependent-forbidden-values",
            contract: "class-blocked-role",
        },
        DiagnosticDetailExpectation {
            code: CUSTOM_DIAGNOSTIC,
            severity: "error",
            behavior: "schema:field-dependency",
            check_kind: "dependent-required-fields",
            contract: "title-without-role-id",
        },
        DiagnosticDetailExpectation {
            code: CUSTOM_DIAGNOSTIC,
            severity: "error",
            behavior: "schema:field-dependency",
            check_kind: "child-gated-dependent-required-fields",
            contract: "article-section-title",
        },
        DiagnosticDetailExpectation {
            code: CUSTOM_DIAGNOSTIC,
            severity: "error",
            behavior: "schema:field-dependency",
            check_kind: "child-gated-dependent-forbidden-fields",
            contract: "article-section-role-forbidden",
        },
        DiagnosticDetailExpectation {
            code: CUSTOM_DIAGNOSTIC,
            severity: "error",
            behavior: "schema:field-dependency",
            check_kind: "child-gated-dependent-forbidden-values",
            contract: "article-section-blocked-class",
        },
    ] {
        assert!(
            has_diagnostic_detail(&report, &expected),
            "expected structured field-dependency diagnostic {:?} in {}",
            expected,
            stdout(&output)
        );
    }

    let source_token_expected = DiagnosticDetailExpectation {
        code: CUSTOM_DIAGNOSTIC,
        severity: "error",
        behavior: "schema:field-dependency",
        check_kind: "dependent-required-fields",
        contract: "class-title-id",
    };
    let source_token = find_diagnostic_detail(&report, &source_token_expected)
        .expect("source/token dependency diagnostic");
    let source_token_details = &source_token["details"];
    assert_eq!(
        source_token_details["condition"],
        serde_json::json!({
            "attribute": null,
            "values": [],
            "presentAttributes": ["class", "title"],
            "absentAttributes": [],
            "presentChildren": [],
            "absentChildren": [],
        })
    );
    assert_eq!(
        source_token_details["requiredFields"],
        serde_json::json!(["id"])
    );
    assert_eq!(
        source_token_details["missingFields"],
        serde_json::json!(["id"])
    );

    let remote_source_token_expected = DiagnosticDetailExpectation {
        code: CUSTOM_DIAGNOSTIC,
        severity: "error",
        behavior: "schema:field-dependency",
        check_kind: "dependent-required-fields",
        contract: "remote-section-role",
    };
    let remote_source_token = find_diagnostic_detail(&report, &remote_source_token_expected)
        .expect("remote source/token dependency diagnostic");
    let remote_source_token_details = &remote_source_token["details"];
    assert_eq!(
        remote_source_token_details["condition"],
        serde_json::json!({
            "attribute": "class",
            "values": ["remote"],
            "presentAttributes": ["id", "title"],
            "absentAttributes": [],
            "presentChildren": [],
            "absentChildren": [],
        })
    );
    assert_eq!(
        remote_source_token_details["actualValues"]["class"],
        "remote"
    );
    assert_eq!(
        remote_source_token_details["missingFields"],
        serde_json::json!(["role"])
    );

    let absent_role_expected = DiagnosticDetailExpectation {
        code: CUSTOM_DIAGNOSTIC,
        severity: "error",
        behavior: "schema:field-dependency",
        check_kind: "dependent-required-fields",
        contract: "title-without-role-id",
    };
    let absent_role = find_diagnostic_detail(&report, &absent_role_expected)
        .expect("absent-role dependency diagnostic");
    let absent_role_details = &absent_role["details"];
    assert_eq!(
        absent_role_details["condition"],
        serde_json::json!({
            "attribute": null,
            "values": [],
            "presentAttributes": ["title"],
            "absentAttributes": ["role"],
            "presentChildren": [],
            "absentChildren": [],
        })
    );
    assert_eq!(
        absent_role_details["missingFields"],
        serde_json::json!(["id"])
    );

    let child_gated_expected = DiagnosticDetailExpectation {
        code: CUSTOM_DIAGNOSTIC,
        severity: "error",
        behavior: "schema:field-dependency",
        check_kind: "child-gated-dependent-required-fields",
        contract: "article-section-title",
    };
    let child_gated = find_diagnostic_detail(&report, &child_gated_expected)
        .expect("child-gated dependency diagnostic");
    let child_gated_details = &child_gated["details"];
    assert_eq!(
        child_gated_details["condition"],
        serde_json::json!({
            "attribute": null,
            "values": [],
            "presentAttributes": [],
            "absentAttributes": [],
            "presentChildren": ["section"],
            "absentChildren": ["span"],
        })
    );
    assert_eq!(
        child_gated_details["requiredFields"],
        serde_json::json!(["title"])
    );
    assert_eq!(
        child_gated_details["missingFields"],
        serde_json::json!(["title"])
    );
    assert_eq!(
        child_gated_details["childCounts"],
        serde_json::json!({
            "section": 1,
        })
    );

    let child_gated_forbidden_expected = DiagnosticDetailExpectation {
        code: CUSTOM_DIAGNOSTIC,
        severity: "error",
        behavior: "schema:field-dependency",
        check_kind: "child-gated-dependent-forbidden-fields",
        contract: "article-section-role-forbidden",
    };
    let child_gated_forbidden = find_diagnostic_detail(&report, &child_gated_forbidden_expected)
        .expect("child-gated forbidden field dependency diagnostic");
    let child_gated_forbidden_details = &child_gated_forbidden["details"];
    assert_eq!(
        child_gated_forbidden_details["condition"],
        serde_json::json!({
            "attribute": null,
            "values": [],
            "presentAttributes": [],
            "absentAttributes": [],
            "presentChildren": ["section"],
            "absentChildren": ["span"],
        })
    );
    assert_eq!(
        child_gated_forbidden_details["forbiddenFields"],
        serde_json::json!(["role"])
    );
    assert_eq!(
        child_gated_forbidden_details["invalidFields"],
        serde_json::json!(["role"])
    );

    let child_gated_forbidden_value_expected = DiagnosticDetailExpectation {
        code: CUSTOM_DIAGNOSTIC,
        severity: "error",
        behavior: "schema:field-dependency",
        check_kind: "child-gated-dependent-forbidden-values",
        contract: "article-section-blocked-class",
    };
    let child_gated_forbidden_value =
        find_diagnostic_detail(&report, &child_gated_forbidden_value_expected)
            .expect("child-gated forbidden value dependency diagnostic");
    let child_gated_forbidden_value_details = &child_gated_forbidden_value["details"];
    assert_eq!(
        child_gated_forbidden_value_details["condition"],
        serde_json::json!({
            "attribute": null,
            "values": [],
            "presentAttributes": [],
            "absentAttributes": [],
            "presentChildren": ["section"],
            "absentChildren": ["span"],
        })
    );
    assert_eq!(
        child_gated_forbidden_value_details["forbiddenAttributeValues"],
        serde_json::json!({
            "class": ["blocked"],
        })
    );
    assert_eq!(
        child_gated_forbidden_value_details["invalidFields"],
        serde_json::json!(["class"])
    );
    assert_eq!(
        child_gated_forbidden_value_details["invalidValues"],
        serde_json::json!({
            "class": "blocked",
        })
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_field_contract_primitives_emit_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/field-contract-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.field-contract-runtime+cem";
    const REQUIRED_DIAGNOSTIC: &str = "example.div.required_id";
    const FORBIDDEN_DIAGNOSTIC: &str = "example.div.forbidden_title";
    const MUTUAL_DIAGNOSTIC: &str = "example.span.mutual_class";
    const GENERIC_DIAGNOSTIC: &str = "example.section.generic_role";

    let root = test_temp_dir("cem-ml-cli-field-contract-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/field-contract-runtime.cem");
    let valid_input_path = root.join("examples/valid-fields.cem");
    let invalid_input_path = root.join("examples/invalid-fields.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="field-contract-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/field-contract-runtime/1"
        @source="schema/field-contract-runtime.cem"
    }
    {content-type @value="application/vnd.example.field-contract-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/field-contract-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="field-contract-runtime" @namespace="https://example.test/ns/field-contract-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.field-contract-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/field-contract-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="div" @optional-attributes="id class title" @children="*"}
        {element @name="span" @optional-attributes="class" @children="*"}
        {element @name="section" @optional-attributes="role" @children="*"}
    }
    {attributes |
        {attribute @name="id" @type="schema:string"}
        {attribute @name="class" @type="schema:string"}
        {attribute @name="title" @type="schema:string"}
        {attribute @name="role" @type="schema:string"}
    }
    {field-contracts |
        {field-contract
            @name="card-id-required"
            @target="div"
            @when-attribute="class"
            @when-values="card"
            @required-attributes="id"
            @diagnostic="example.div.required_id"
            @behavior="schema:required-fields"
            @check-kind="required-fields"
        }
        {field-contract
            @name="card-title-forbidden"
            @target="div"
            @when-attribute="class"
            @when-values="card"
            @forbidden-attributes="title"
            @diagnostic="example.div.forbidden_title"
            @behavior="schema:forbidden-fields"
            @check-kind="forbidden-fields"
        }
        {field-contract
            @name="span-blocked-class"
            @target="span"
            @forbidden-attribute-values="class=blocked"
            @diagnostic="example.span.mutual_class"
            @behavior="schema:mutual-exclusion"
            @check-kind="mutual-exclusion"
        }
        {field-contract
            @name="section-role-required"
            @target="section"
            @required-attributes="role"
            @diagnostic="example.section.generic_role"
            @check-kind="required-fields"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.div.required_id"
            @severity="error"
            @behavior="schema:required-fields"
            @message="Card divs must declare an id"
        }
        {diagnostic
            @code="example.div.forbidden_title"
            @severity="error"
            @behavior="schema:forbidden-fields"
            @message="Card divs must not declare title"
        }
        {diagnostic
            @code="example.span.mutual_class"
            @severity="error"
            @behavior="schema:mutual-exclusion"
            @message="Span class must avoid blocked value"
        }
        {diagnostic
            @code="example.section.generic_role"
            @severity="error"
            @behavior="schema:field-contract"
            @message="Section role is required through the generic field contract behavior"
        }
    }
}
"#,
    );
    write_test_file(
        &valid_input_path,
        r#"@doc cem-ml 1

{div @class="card" @id="card-1"}
{span @class="open"}
{section @role="region"}
"#,
    );
    write_test_file(
        &invalid_input_path,
        r#"@doc cem-ml 1

{div @class="card" @title="debug"}
{span @class="blocked"}
{section}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let valid_output = validate(&valid_input_path);
    assert_eq!(
        valid_output.status.code(),
        Some(EXIT_OK),
        "field contract valid stderr:\n{}",
        stderr(&valid_output)
    );
    assert!(
        stderr(&valid_output).trim().is_empty(),
        "field contract valid stderr must stay empty:\n{}",
        stderr(&valid_output)
    );
    let valid_report: serde_json::Value = serde_json::from_str(stdout(&valid_output).trim())
        .expect("field contract valid report is JSON");
    for code in [
        REQUIRED_DIAGNOSTIC,
        FORBIDDEN_DIAGNOSTIC,
        MUTUAL_DIAGNOSTIC,
        GENERIC_DIAGNOSTIC,
    ] {
        assert!(
            !has_diagnostic(&valid_report, code),
            "`{code}` should not be emitted for valid field contracts:\n{}",
            stdout(&valid_output)
        );
    }

    let invalid_output = validate(&invalid_input_path);
    assert_eq!(
        invalid_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "field contract invalid stderr:\n{}",
        stderr(&invalid_output)
    );
    assert!(
        stderr(&invalid_output).trim().is_empty(),
        "field contract invalid stderr must stay empty:\n{}",
        stderr(&invalid_output)
    );
    let invalid_report: serde_json::Value = serde_json::from_str(stdout(&invalid_output).trim())
        .expect("field contract invalid report is JSON");
    let diagnostic_for_code = |code: &str| {
        diagnostics(&invalid_report)
            .iter()
            .find(|diagnostic| diagnostic["code"] == code)
            .unwrap_or_else(|| {
                panic!(
                    "expected `{code}` in field contract invalid report:\n{}",
                    stdout(&invalid_output)
                )
            })
    };

    let required_diagnostic = diagnostic_for_code(REQUIRED_DIAGNOSTIC);
    let required_details = &required_diagnostic["details"];
    assert_eq!(required_diagnostic["severity"], "error");
    assert_eq!(required_details["behavior"], "schema:required-fields");
    assert_eq!(required_details["checkKind"], "required-fields");
    assert_eq!(required_details["contract"], "card-id-required");
    assert_eq!(
        required_details["requiredFields"],
        serde_json::json!(["id"])
    );
    assert_eq!(required_details["missingFields"], serde_json::json!(["id"]));
    assert_eq!(required_details["condition"]["attribute"], "class");
    assert_eq!(
        required_details["condition"]["values"],
        serde_json::json!(["card"])
    );
    assert_eq!(required_details["actualValues"]["class"], "card");

    let forbidden_diagnostic = diagnostic_for_code(FORBIDDEN_DIAGNOSTIC);
    let forbidden_details = &forbidden_diagnostic["details"];
    assert_eq!(forbidden_diagnostic["severity"], "error");
    assert_eq!(forbidden_details["behavior"], "schema:forbidden-fields");
    assert_eq!(forbidden_details["checkKind"], "forbidden-fields");
    assert_eq!(forbidden_details["contract"], "card-title-forbidden");
    assert_eq!(
        forbidden_details["forbiddenFields"],
        serde_json::json!(["title"])
    );
    assert_eq!(
        forbidden_details["invalidFields"],
        serde_json::json!(["title"])
    );
    assert_eq!(forbidden_details["condition"]["attribute"], "class");
    assert_eq!(
        forbidden_details["condition"]["values"],
        serde_json::json!(["card"])
    );
    assert_eq!(forbidden_details["actualValues"]["title"], "debug");

    let mutual_diagnostic = diagnostic_for_code(MUTUAL_DIAGNOSTIC);
    let mutual_details = &mutual_diagnostic["details"];
    assert_eq!(mutual_diagnostic["severity"], "error");
    assert_eq!(mutual_details["behavior"], "schema:mutual-exclusion");
    assert_eq!(mutual_details["checkKind"], "mutual-exclusion");
    assert_eq!(mutual_details["contract"], "span-blocked-class");
    assert_eq!(
        mutual_details["forbiddenAttributeValues"],
        serde_json::json!({
            "class": ["blocked"],
        })
    );
    assert_eq!(
        mutual_details["invalidValues"],
        serde_json::json!({
            "class": "blocked",
        })
    );
    assert_eq!(
        mutual_details["invalidFields"],
        serde_json::json!(["class"])
    );
    assert_eq!(mutual_details["actualValues"]["class"], "blocked");

    let generic_diagnostic = diagnostic_for_code(GENERIC_DIAGNOSTIC);
    let generic_details = &generic_diagnostic["details"];
    assert_eq!(generic_diagnostic["severity"], "error");
    assert_eq!(generic_details["behavior"], "schema:field-contract");
    assert_eq!(generic_details["checkKind"], "required-fields");
    assert_eq!(generic_details["contract"], "section-role-required");
    assert_eq!(generic_details["target"], "section");
    assert_eq!(
        generic_details["requiredFields"],
        serde_json::json!(["role"])
    );
    assert_eq!(
        generic_details["missingFields"],
        serde_json::json!(["role"])
    );

    for diagnostic in [
        required_diagnostic,
        forbidden_diagnostic,
        mutual_diagnostic,
        generic_diagnostic,
    ] {
        assert!(diagnostic["details"]["sourceRange"]["span"]["start"].is_u64());
        assert!(diagnostic["sourceMap"]["frames"]
            .as_array()
            .is_some_and(|frames| !frames.is_empty()));
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_attribute_value_and_scalar_type_emit_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/attribute-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.attribute-runtime+cem";
    const VALUE_DIAGNOSTIC: &str = "example.div.invalid_class";
    const TYPE_DIAGNOSTIC: &str = "example.div.invalid_id";

    let root = test_temp_dir("cem-ml-cli-attribute-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/attribute-runtime.cem");
    let valid_input_path = root.join("examples/valid-attributes.cem");
    let invalid_input_path = root.join("examples/invalid-attributes.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="attribute-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/attribute-runtime/1"
        @source="schema/attribute-runtime.cem"
    }
    {content-type @value="application/vnd.example.attribute-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/attribute-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="attribute-runtime" @namespace="https://example.test/ns/attribute-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.attribute-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/attribute-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="div" @optional-attributes="class id" @children="*"}
    }
    {attributes |
        {attribute
            @name="class"
            @type="schema:string"
            @values="card panel"
            @values-diagnostic="example.div.invalid_class"
        }
        {attribute
            @name="id"
            @type="schema:integer"
            @type-diagnostic="example.div.invalid_id"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.div.invalid_class"
            @severity="error"
            @behavior="schema:value-vocabulary"
            @message="Div class must use the declared vocabulary"
        }
        {diagnostic
            @code="example.div.invalid_id"
            @severity="error"
            @behavior="schema:scalar-type"
            @message="Div id must be an integer for this schema"
        }
    }
}
"#,
    );
    write_test_file(
        &valid_input_path,
        r#"@doc cem-ml 1

{div @class="card" @id=7}
"#,
    );
    write_test_file(
        &invalid_input_path,
        r#"@doc cem-ml 1

{div @class="unknown" @id="abc"}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let valid_output = validate(&valid_input_path);
    assert_eq!(
        valid_output.status.code(),
        Some(EXIT_OK),
        "attribute runtime valid stderr:\n{}",
        stderr(&valid_output)
    );
    assert!(
        stderr(&valid_output).trim().is_empty(),
        "attribute runtime valid stderr must stay empty:\n{}",
        stderr(&valid_output)
    );
    let valid_report: serde_json::Value = serde_json::from_str(stdout(&valid_output).trim())
        .expect("attribute runtime valid report is JSON");
    for code in [VALUE_DIAGNOSTIC, TYPE_DIAGNOSTIC] {
        assert!(
            !has_diagnostic(&valid_report, code),
            "`{code}` should not be emitted for valid attribute contracts:\n{}",
            stdout(&valid_output)
        );
    }

    let invalid_output = validate(&invalid_input_path);
    assert_eq!(
        invalid_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "attribute runtime invalid stderr:\n{}",
        stderr(&invalid_output)
    );
    assert!(
        stderr(&invalid_output).trim().is_empty(),
        "attribute runtime invalid stderr must stay empty:\n{}",
        stderr(&invalid_output)
    );
    let invalid_report: serde_json::Value = serde_json::from_str(stdout(&invalid_output).trim())
        .expect("attribute runtime invalid report is JSON");
    let diagnostic_for_code = |code: &str| {
        diagnostics(&invalid_report)
            .iter()
            .find(|diagnostic| diagnostic["code"] == code)
            .unwrap_or_else(|| {
                panic!(
                    "expected `{code}` in attribute runtime invalid report:\n{}",
                    stdout(&invalid_output)
                )
            })
    };

    let value_diagnostic = diagnostic_for_code(VALUE_DIAGNOSTIC);
    let value_details = &value_diagnostic["details"];
    assert_eq!(value_diagnostic["severity"], "error");
    assert_eq!(value_details["behavior"], "schema:value-vocabulary");
    assert_eq!(value_details["checkKind"], "value-vocabulary");
    assert_eq!(value_details["contract"], "attribute-values:class");
    assert_eq!(value_details["attribute"], "class");
    assert_eq!(value_details["valueType"], "schema:string");
    assert_eq!(
        value_details["expectedValues"],
        serde_json::json!(["card", "panel"])
    );
    assert_eq!(value_details["actualValue"], "unknown");
    assert_eq!(value_details["invalidFields"], serde_json::json!(["class"]));
    assert_eq!(value_details["actualValues"]["class"], "unknown");

    let type_diagnostic = diagnostic_for_code(TYPE_DIAGNOSTIC);
    let type_details = &type_diagnostic["details"];
    assert_eq!(type_diagnostic["severity"], "error");
    assert_eq!(type_details["behavior"], "schema:scalar-type");
    assert_eq!(type_details["type"], "integer");
    assert_eq!(type_details["checkKind"], "type:integer");
    assert_eq!(type_details["contract"], "attribute-type:id");
    assert_eq!(type_details["attribute"], "id");
    assert_eq!(type_details["valueType"], "schema:integer");
    assert_eq!(type_details["expectedType"], "schema:integer");
    assert_eq!(type_details["actualValue"], "abc");
    assert_eq!(type_details["invalidFields"], serde_json::json!(["id"]));
    assert_eq!(type_details["actualValues"]["id"], "abc");

    for diagnostic in [value_diagnostic, type_diagnostic] {
        assert!(diagnostic["details"]["sourceRange"]["span"]["start"].is_u64());
        assert!(diagnostic["sourceMap"]["frames"]
            .as_array()
            .is_some_and(|frames| !frames.is_empty()));
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_choice_case_groups_emit_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/choice-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.choice-runtime+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.item.choice";

    let root = test_temp_dir("cem-ml-cli-choice-case-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/choice-runtime.cem");
    let missing_input_path = root.join("examples/missing-choice.cem");
    let conflicting_input_path = root.join("examples/conflicting-choice.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="choice-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/choice-runtime/1"
        @source="schema/choice-runtime.cem"
    }
    {content-type @value="application/vnd.example.choice-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/choice-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="choice-runtime" @namespace="https://example.test/ns/choice-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.choice-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/choice-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="item" @optional-attributes="href inline" @children="body"}
        {element @name="body"}
    }
    {attributes |
        {attribute @name="href" @type="schema:string"}
        {attribute @name="inline" @type="schema:string"}
    }
    {field-contracts |
        {field-contract
            @name="item-link-source-choice"
            @target="item"
            @diagnostic="example.item.choice"
            @behavior="schema:choice-case"
            @check-kind="choice-case" |
            {choice @name="link-source" @mode="exactly-one" |
                {case @name="href-link" @attributes="href"}
                {case @name="inline-link" @attributes="inline"}
                {case @name="body-link" @children="body"}
            }
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.item.choice"
            @severity="error"
            @behavior="schema:choice-case"
            @message="Item must choose exactly one link source"
        }
    }
}
"#,
    );
    write_test_file(
        &missing_input_path,
        r#"@doc cem-ml 1

{item}
"#,
    );
    write_test_file(
        &conflicting_input_path,
        r#"@doc cem-ml 1

{item @href="/demo" @inline="text" | {body}}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let missing_output = validate(&missing_input_path);
    assert_eq!(
        missing_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "missing choice runtime stderr:\n{}",
        stderr(&missing_output)
    );
    assert!(
        stderr(&missing_output).trim().is_empty(),
        "missing choice runtime stderr must stay empty:\n{}",
        stderr(&missing_output)
    );
    let missing_report: serde_json::Value = serde_json::from_str(stdout(&missing_output).trim())
        .expect("missing choice runtime report is JSON");
    let missing_diagnostic = diagnostics(&missing_report)
        .iter()
        .find(|diagnostic| diagnostic["code"] == CUSTOM_DIAGNOSTIC)
        .unwrap_or_else(|| {
            panic!(
                "expected `{CUSTOM_DIAGNOSTIC}` in missing choice runtime report:\n{}",
                stdout(&missing_output)
            )
        });
    let missing_details = &missing_diagnostic["details"];
    assert_eq!(missing_diagnostic["severity"], "error");
    assert_eq!(missing_details["behavior"], "schema:choice-case");
    assert_eq!(missing_details["checkKind"], "choice-case");
    assert_eq!(missing_details["contract"], "item-link-source-choice");
    assert_eq!(
        missing_details["missingChoiceCases"],
        serde_json::json!(["link-source"])
    );
    assert_eq!(
        missing_details["missingChoiceFields"],
        serde_json::json!(["body", "href", "inline"])
    );
    assert_eq!(
        missing_details["conflictingChoiceCases"],
        serde_json::json!({})
    );
    assert_eq!(
        missing_details["conflictingChoiceFields"],
        serde_json::json!([])
    );
    assert_eq!(
        missing_details["choiceCases"],
        serde_json::json!([
            {
                "name": "link-source",
                "mode": "exactly-one",
                "cases": [
                    {"name": "href-link", "attributes": ["href"], "children": []},
                    {"name": "inline-link", "attributes": ["inline"], "children": []},
                    {"name": "body-link", "attributes": [], "children": ["body"]},
                ],
            }
        ])
    );
    assert!(missing_details["sourceRange"]["span"]["start"].is_u64());
    assert!(missing_diagnostic["sourceMap"]["frames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));

    let conflicting_output = validate(&conflicting_input_path);
    assert_eq!(
        conflicting_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "conflicting choice runtime stderr:\n{}",
        stderr(&conflicting_output)
    );
    assert!(
        stderr(&conflicting_output).trim().is_empty(),
        "conflicting choice runtime stderr must stay empty:\n{}",
        stderr(&conflicting_output)
    );
    let conflicting_report: serde_json::Value =
        serde_json::from_str(stdout(&conflicting_output).trim())
            .expect("conflicting choice runtime report is JSON");
    let conflicting_diagnostic = diagnostics(&conflicting_report)
        .iter()
        .find(|diagnostic| diagnostic["code"] == CUSTOM_DIAGNOSTIC)
        .unwrap_or_else(|| {
            panic!(
                "expected `{CUSTOM_DIAGNOSTIC}` in conflicting choice runtime report:\n{}",
                stdout(&conflicting_output)
            )
        });
    let conflicting_details = &conflicting_diagnostic["details"];
    assert_eq!(conflicting_diagnostic["severity"], "error");
    assert_eq!(conflicting_details["behavior"], "schema:choice-case");
    assert_eq!(conflicting_details["checkKind"], "choice-case");
    assert_eq!(conflicting_details["contract"], "item-link-source-choice");
    assert_eq!(
        conflicting_details["missingChoiceCases"],
        serde_json::json!([])
    );
    assert_eq!(
        conflicting_details["presentChoiceCases"]["link-source"],
        serde_json::json!(["href-link", "inline-link", "body-link"])
    );
    assert_eq!(
        conflicting_details["conflictingChoiceCases"]["link-source"],
        serde_json::json!(["href-link", "inline-link", "body-link"])
    );
    assert_eq!(
        conflicting_details["conflictingChoiceFields"],
        serde_json::json!(["body", "href", "inline"])
    );
    assert_eq!(
        conflicting_details["invalidFields"],
        serde_json::json!(["body", "href", "inline"])
    );
    assert!(conflicting_details["sourceRange"]["span"]["start"].is_u64());
    assert!(conflicting_diagnostic["sourceMap"]["frames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_accepted_children_emits_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/accepted-children-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.accepted-children-runtime+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.div.accepted_children";

    let root = test_temp_dir("cem-ml-cli-accepted-children-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/accepted-children-runtime.cem");
    let valid_input_path = root.join("examples/valid-children.cem");
    let invalid_input_path = root.join("examples/invalid-children.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="accepted-children-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/accepted-children-runtime/1"
        @source="schema/accepted-children-runtime.cem"
    }
    {content-type @value="application/vnd.example.accepted-children-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/accepted-children-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="accepted-children-runtime" @namespace="https://example.test/ns/accepted-children-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.accepted-children-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/accepted-children-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="div" @children="span* p* a*"}
        {element @name="span"}
        {element @name="p"}
        {element @name="a"}
    }
    {field-contracts |
        {field-contract
            @name="div-accepted-children"
            @target="div"
            @accepted-children="span p"
            @diagnostic="example.div.accepted_children"
            @behavior="schema:accepted-children"
            @check-kind="accepted-children"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.div.accepted_children"
            @severity="error"
            @behavior="schema:accepted-children"
            @message="Div children must use the accepted child set"
        }
    }
}
"#,
    );
    write_test_file(
        &valid_input_path,
        r#"@doc cem-ml 1

{div | {span} {p}}
"#,
    );
    write_test_file(
        &invalid_input_path,
        r#"@doc cem-ml 1

{div | {span} {a}}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let valid_output = validate(&valid_input_path);
    assert_eq!(
        valid_output.status.code(),
        Some(EXIT_OK),
        "accepted children valid stderr:\n{}",
        stderr(&valid_output)
    );
    assert!(
        stderr(&valid_output).trim().is_empty(),
        "accepted children valid stderr must stay empty:\n{}",
        stderr(&valid_output)
    );
    let valid_report: serde_json::Value = serde_json::from_str(stdout(&valid_output).trim())
        .expect("accepted children valid report is JSON");
    assert!(
        !has_diagnostic(&valid_report, CUSTOM_DIAGNOSTIC),
        "`{CUSTOM_DIAGNOSTIC}` should not be emitted for accepted children:\n{}",
        stdout(&valid_output)
    );

    let invalid_output = validate(&invalid_input_path);
    assert_eq!(
        invalid_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "accepted children invalid stderr:\n{}",
        stderr(&invalid_output)
    );
    assert!(
        stderr(&invalid_output).trim().is_empty(),
        "accepted children invalid stderr must stay empty:\n{}",
        stderr(&invalid_output)
    );
    let invalid_report: serde_json::Value = serde_json::from_str(stdout(&invalid_output).trim())
        .expect("accepted children invalid report is JSON");
    let diagnostic = diagnostics(&invalid_report)
        .iter()
        .find(|diagnostic| diagnostic["code"] == CUSTOM_DIAGNOSTIC)
        .unwrap_or_else(|| {
            panic!(
                "expected `{CUSTOM_DIAGNOSTIC}` in accepted children invalid report:\n{}",
                stdout(&invalid_output)
            )
        });
    let details = &diagnostic["details"];
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(details["behavior"], "schema:accepted-children");
    assert_eq!(details["checkKind"], "accepted-children");
    assert_eq!(details["contract"], "div-accepted-children");
    assert_eq!(
        details["acceptedChildren"],
        serde_json::json!(["p", "span"])
    );
    assert_eq!(details["invalidChildren"], serde_json::json!(["a"]));
    assert_eq!(
        details["childCounts"],
        serde_json::json!({
            "a": 1,
            "span": 1,
        })
    );
    assert!(details["sourceRange"]["span"]["start"].is_u64());
    assert!(diagnostic["sourceMap"]["frames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_path_layout_emits_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/path-layout-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.path-layout-runtime+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.link.path_layout";

    let root = test_temp_dir("cem-ml-cli-path-layout-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/path-layout-runtime.cem");
    let valid_input_path = root.join("examples/valid-path.cem");
    let invalid_input_path = root.join("examples/invalid-path.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="path-layout-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/path-layout-runtime/1"
        @source="schema/path-layout-runtime.cem"
    }
    {content-type @value="application/vnd.example.path-layout-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/path-layout-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="path-layout-runtime" @namespace="https://example.test/ns/path-layout-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.path-layout-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/path-layout-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="a" @optional-attributes="href" @children="*"}
    }
    {attributes |
        {attribute @name="href" @type="schema:path"}
    }
    {field-contracts |
        {field-contract
            @name="link-asset-path-layout"
            @target="a"
            @path-layout-attributes="href"
            @path-layout-prefix="assets"
            @path-layout-extension="cemt"
            @diagnostic="example.link.path_layout"
            @behavior="schema:path-layout"
            @check-kind="path-layout"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.link.path_layout"
            @severity="error"
            @behavior="schema:path-layout"
            @message="Link href must use the declared asset layout"
        }
    }
}
"#,
    );
    write_test_file(
        &valid_input_path,
        r#"@doc cem-ml 1

{a @href="assets/demo.cemt" | Asset}
"#,
    );
    write_test_file(
        &invalid_input_path,
        r#"@doc cem-ml 1

{a @href="transforms/demo.cem" | Asset}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let valid_output = validate(&valid_input_path);
    assert_eq!(
        valid_output.status.code(),
        Some(EXIT_OK),
        "path layout valid stderr:\n{}",
        stderr(&valid_output)
    );
    assert!(
        stderr(&valid_output).trim().is_empty(),
        "path layout valid stderr must stay empty:\n{}",
        stderr(&valid_output)
    );
    let valid_report: serde_json::Value = serde_json::from_str(stdout(&valid_output).trim())
        .expect("path layout valid report is JSON");
    assert!(
        !has_diagnostic(&valid_report, CUSTOM_DIAGNOSTIC),
        "`{CUSTOM_DIAGNOSTIC}` should not be emitted for matching path layout:\n{}",
        stdout(&valid_output)
    );

    let invalid_output = validate(&invalid_input_path);
    assert_eq!(
        invalid_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "path layout invalid stderr:\n{}",
        stderr(&invalid_output)
    );
    assert!(
        stderr(&invalid_output).trim().is_empty(),
        "path layout invalid stderr must stay empty:\n{}",
        stderr(&invalid_output)
    );
    let invalid_report: serde_json::Value = serde_json::from_str(stdout(&invalid_output).trim())
        .expect("path layout invalid report is JSON");
    let diagnostic = diagnostics(&invalid_report)
        .iter()
        .find(|diagnostic| diagnostic["code"] == CUSTOM_DIAGNOSTIC)
        .unwrap_or_else(|| {
            panic!(
                "expected `{CUSTOM_DIAGNOSTIC}` in path layout invalid report:\n{}",
                stdout(&invalid_output)
            )
        });
    let details = &diagnostic["details"];
    assert_eq!(diagnostic["severity"], "error");
    assert_eq!(details["behavior"], "schema:path-layout");
    assert_eq!(details["checkKind"], "path-layout");
    assert_eq!(details["contract"], "link-asset-path-layout");
    assert_eq!(details["invalidFields"], serde_json::json!(["href"]));
    assert_eq!(
        details["invalidValues"],
        serde_json::json!({
            "href": "transforms/demo.cem",
        })
    );
    assert_eq!(
        details["pathLayout"],
        serde_json::json!({
            "attributes": ["href"],
            "prefix": "assets",
            "extension": "cemt",
            "relative": true,
            "cleanSegments": true,
        })
    );
    assert!(details["sourceRange"]["span"]["start"].is_u64());
    assert!(diagnostic["sourceMap"]["frames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_child_occurrence_choice_emits_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/child-choice-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.child-choice-runtime+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.section.child_choice";

    let root = test_temp_dir("cem-ml-cli-child-choice-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/child-choice-runtime.cem");
    let valid_input_path = root.join("examples/valid-child-choice.cem");
    let missing_input_path = root.join("examples/missing-child-choice.cem");
    let conflicting_input_path = root.join("examples/conflicting-child-choice.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="child-choice-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/child-choice-runtime/1"
        @source="schema/child-choice-runtime.cem"
    }
    {content-type @value="application/vnd.example.child-choice-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/child-choice-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="child-choice-runtime" @namespace="https://example.test/ns/child-choice-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.child-choice-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/child-choice-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="section" @children="header* footer* aside*"}
        {element @name="header"}
        {element @name="footer"}
        {element @name="aside"}
    }
    {field-contracts |
        {field-contract
            @name="section-heading-or-trailing-choice"
            @target="section"
            @required-one-child="header footer"
            @max-one-child="header footer"
            @diagnostic="example.section.child_choice"
            @behavior="schema:child-occurrence"
            @check-kind="exactly-one-child"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.section.child_choice"
            @severity="error"
            @behavior="schema:child-occurrence"
            @message="Section must choose exactly one structural child"
        }
    }
}
"#,
    );
    write_test_file(
        &valid_input_path,
        r#"@doc cem-ml 1

{section | {header}}
"#,
    );
    write_test_file(
        &missing_input_path,
        r#"@doc cem-ml 1

{section | {aside}}
"#,
    );
    write_test_file(
        &conflicting_input_path,
        r#"@doc cem-ml 1

{section | {header} {footer}}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let valid_output = validate(&valid_input_path);
    assert_eq!(
        valid_output.status.code(),
        Some(EXIT_OK),
        "child choice valid stderr:\n{}",
        stderr(&valid_output)
    );
    assert!(
        stderr(&valid_output).trim().is_empty(),
        "child choice valid stderr must stay empty:\n{}",
        stderr(&valid_output)
    );
    let valid_report: serde_json::Value = serde_json::from_str(stdout(&valid_output).trim())
        .expect("child choice valid report is JSON");
    assert!(
        !has_diagnostic(&valid_report, CUSTOM_DIAGNOSTIC),
        "`{CUSTOM_DIAGNOSTIC}` should not be emitted for a single choice child:\n{}",
        stdout(&valid_output)
    );

    let missing_output = validate(&missing_input_path);
    assert_eq!(
        missing_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "missing child choice runtime stderr:\n{}",
        stderr(&missing_output)
    );
    assert!(
        stderr(&missing_output).trim().is_empty(),
        "missing child choice runtime stderr must stay empty:\n{}",
        stderr(&missing_output)
    );
    let missing_report: serde_json::Value = serde_json::from_str(stdout(&missing_output).trim())
        .expect("missing child choice runtime report is JSON");
    let missing_diagnostic = diagnostics(&missing_report)
        .iter()
        .find(|diagnostic| diagnostic["code"] == CUSTOM_DIAGNOSTIC)
        .unwrap_or_else(|| {
            panic!(
                "expected `{CUSTOM_DIAGNOSTIC}` in missing child choice report:\n{}",
                stdout(&missing_output)
            )
        });
    let missing_details = &missing_diagnostic["details"];
    assert_eq!(missing_diagnostic["severity"], "error");
    assert_eq!(missing_details["behavior"], "schema:child-occurrence");
    assert_eq!(missing_details["checkKind"], "exactly-one-child");
    assert_eq!(
        missing_details["contract"],
        "section-heading-or-trailing-choice"
    );
    assert_eq!(
        missing_details["requiredOneChild"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        missing_details["maxOneChild"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        missing_details["presentRequiredOneChild"],
        serde_json::json!([])
    );
    assert_eq!(
        missing_details["missingChoiceChildren"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        missing_details["conflictingChoiceChildren"],
        serde_json::json!([])
    );
    assert!(missing_details["sourceRange"]["span"]["start"].is_u64());
    assert!(missing_diagnostic["sourceMap"]["frames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));

    let conflicting_output = validate(&conflicting_input_path);
    assert_eq!(
        conflicting_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "conflicting child choice runtime stderr:\n{}",
        stderr(&conflicting_output)
    );
    assert!(
        stderr(&conflicting_output).trim().is_empty(),
        "conflicting child choice runtime stderr must stay empty:\n{}",
        stderr(&conflicting_output)
    );
    let conflicting_report: serde_json::Value =
        serde_json::from_str(stdout(&conflicting_output).trim())
            .expect("conflicting child choice runtime report is JSON");
    let conflicting_diagnostic = diagnostics(&conflicting_report)
        .iter()
        .find(|diagnostic| diagnostic["code"] == CUSTOM_DIAGNOSTIC)
        .unwrap_or_else(|| {
            panic!(
                "expected `{CUSTOM_DIAGNOSTIC}` in conflicting child choice report:\n{}",
                stdout(&conflicting_output)
            )
        });
    let conflicting_details = &conflicting_diagnostic["details"];
    assert_eq!(conflicting_diagnostic["severity"], "error");
    assert_eq!(conflicting_details["behavior"], "schema:child-occurrence");
    assert_eq!(conflicting_details["checkKind"], "exactly-one-child");
    assert_eq!(
        conflicting_details["contract"],
        "section-heading-or-trailing-choice"
    );
    assert_eq!(
        conflicting_details["presentRequiredOneChild"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        conflicting_details["presentMaxOneChild"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        conflicting_details["missingChoiceChildren"],
        serde_json::json!([])
    );
    assert_eq!(
        conflicting_details["conflictingChoiceChildren"],
        serde_json::json!(["footer", "header"])
    );
    assert!(conflicting_details["sourceRange"]["span"]["start"].is_u64());
    assert!(conflicting_diagnostic["sourceMap"]["frames"]
        .as_array()
        .is_some_and(|frames| !frames.is_empty()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_child_occurrence_sequence_emits_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/child-sequence-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.child-sequence-runtime+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.layout.child_sequence";

    let root = test_temp_dir("cem-ml-cli-child-sequence-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/child-sequence-runtime.cem");
    let valid_input_path = root.join("examples/valid-child-sequences.cem");
    let invalid_input_path = root.join("examples/invalid-child-sequences.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="child-sequence-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/child-sequence-runtime/1"
        @source="schema/child-sequence-runtime.cem"
    }
    {content-type @value="application/vnd.example.child-sequence-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/child-sequence-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="child-sequence-runtime" @namespace="https://example.test/ns/child-sequence-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.child-sequence-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/child-sequence-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="article" @children="*"}
        {element @name="section" @children="*"}
        {element @name="nav" @children="*"}
        {element @name="ul" @children="*"}
        {element @name="div" @children="*"}
        {element @name="dialog" @children="*"}
        {element @name="form" @children="*"}
        {element @name="fieldset" @children="*"}
        {element @name="label" @children="*"}
        {element @name="header"}
        {element @name="main" @children="*"}
        {element @name="footer"}
        {element @name="aside" @children="*"}
        {element @name="li"}
        {element @name="span"}
        {element @name="p"}
        {element @name="strong"}
        {element @name="small"}
        {element @name="legend"}
        {element @name="input"}
    }
    {field-contracts |
        {field-contract
            @name="article-ordered-children"
            @target="article"
            @ordered-children="header main footer"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="ordered-children"
        }
        {field-contract
            @name="article-forbidden-ordered-children"
            @target="article"
            @forbidden-ordered-children="footer header"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="forbidden-ordered-children"
        }
        {field-contract
            @name="section-boundary-children"
            @target="section"
            @first-child="header"
            @last-child="footer"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="boundary-children"
        }
        {field-contract
            @name="nav-required-child-sequence"
            @target="nav"
            @required-child-sequence="header main footer"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="required-child-sequence"
        }
        {field-contract
            @name="ul-forbidden-child-sequence"
            @target="ul"
            @forbidden-child-sequence="li span"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="forbidden-child-sequence"
        }
        {field-contract
            @name="div-exact-child-sequence"
            @target="div"
            @exact-child-sequence="span p strong"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="exact-child-sequence"
        }
        {field-contract
            @name="dialog-prefix-child-sequence"
            @target="dialog"
            @prefix-child-sequence="header main"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="prefix-child-sequence"
        }
        {field-contract
            @name="form-suffix-child-sequence"
            @target="form"
            @suffix-child-sequence="main footer"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="suffix-child-sequence"
        }
        {field-contract
            @name="aside-forbidden-prefix-child-sequence"
            @target="aside"
            @forbidden-prefix-child-sequence="footer header"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="forbidden-prefix-child-sequence"
        }
        {field-contract
            @name="main-forbidden-suffix-child-sequence"
            @target="main"
            @forbidden-suffix-child-sequence="footer header"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="forbidden-suffix-child-sequence"
        }
        {field-contract
            @name="fieldset-forbidden-first-child"
            @target="fieldset"
            @forbidden-first-child="legend"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="forbidden-first-child"
        }
        {field-contract
            @name="label-forbidden-last-child"
            @target="label"
            @forbidden-last-child="input"
            @diagnostic="example.layout.child_sequence"
            @behavior="schema:child-occurrence"
            @check-kind="forbidden-last-child"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.layout.child_sequence"
            @severity="error"
            @behavior="schema:child-occurrence"
            @message="Layout children must satisfy the declared sequence contracts"
        }
    }
}
"#,
    );
    write_test_file(
        &valid_input_path,
        r#"@doc cem-ml 1

{article | {header} {aside} {main} {footer}}
{section | {header} {main} {footer}}
{nav | {aside} {header} {main} {footer} {aside}}
{ul | {li} {aside} {span}}
{div | {span} {p} {strong}}
{dialog | {header} {main} {aside}}
{form | {aside} {main} {footer}}
{aside | {header} {footer}}
{main | {header} {footer}}
{fieldset | {input} {legend}}
{label | {input} {span}}
"#,
    );
    write_test_file(
        &invalid_input_path,
        r#"@doc cem-ml 1

{article | {main} {header} {footer}}
{article | {footer} {aside} {header} {main}}
{section | {main} {header} {footer}}
{nav | {header} {aside} {main} {footer}}
{ul | {aside} {li} {span} {footer}}
{div | {span} {small} {p} {strong}}
{dialog | {aside} {header} {main}}
{form | {main} {footer} {aside}}
{aside | {footer} {header} {main}}
{main | {aside} {footer} {header}}
{fieldset | {legend} {input}}
{label | {span} {input}}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let valid_output = validate(&valid_input_path);
    assert_eq!(
        valid_output.status.code(),
        Some(EXIT_OK),
        "child sequence valid stderr:\n{}",
        stderr(&valid_output)
    );
    assert!(
        stderr(&valid_output).trim().is_empty(),
        "child sequence valid stderr must stay empty:\n{}",
        stderr(&valid_output)
    );
    let valid_report: serde_json::Value = serde_json::from_str(stdout(&valid_output).trim())
        .expect("child sequence valid report is JSON");
    assert!(
        !has_diagnostic(&valid_report, CUSTOM_DIAGNOSTIC),
        "`{CUSTOM_DIAGNOSTIC}` should not be emitted for valid child sequences:\n{}",
        stdout(&valid_output)
    );

    let invalid_output = validate(&invalid_input_path);
    assert_eq!(
        invalid_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "child sequence invalid stderr:\n{}",
        stderr(&invalid_output)
    );
    assert!(
        stderr(&invalid_output).trim().is_empty(),
        "child sequence invalid stderr must stay empty:\n{}",
        stderr(&invalid_output)
    );
    let invalid_report: serde_json::Value = serde_json::from_str(stdout(&invalid_output).trim())
        .expect("child sequence invalid report is JSON");
    let diagnostic_for_contract = |contract: &str| {
        diagnostics(&invalid_report)
            .iter()
            .find(|diagnostic| {
                diagnostic["code"] == CUSTOM_DIAGNOSTIC
                    && diagnostic["details"]["contract"] == contract
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected `{CUSTOM_DIAGNOSTIC}` contract `{contract}` in child sequence report:\n{}",
                    stdout(&invalid_output)
                )
            })
    };

    let ordered_diagnostic = diagnostic_for_contract("article-ordered-children");
    let ordered_details = &ordered_diagnostic["details"];
    assert_eq!(ordered_diagnostic["severity"], "error");
    assert_eq!(ordered_details["behavior"], "schema:child-occurrence");
    assert_eq!(ordered_details["checkKind"], "ordered-children");
    assert_eq!(
        ordered_details["orderedChildren"],
        serde_json::json!(["header", "main", "footer"])
    );
    assert_eq!(
        ordered_details["orderedChildSequence"],
        serde_json::json!(["main", "header", "footer"])
    );
    assert_eq!(
        ordered_details["unorderedChildren"],
        serde_json::json!(["header"])
    );
    assert_eq!(ordered_details["invalidChildOrder"], true);

    let forbidden_ordered_diagnostic =
        diagnostic_for_contract("article-forbidden-ordered-children");
    let forbidden_ordered_details = &forbidden_ordered_diagnostic["details"];
    assert_eq!(forbidden_ordered_diagnostic["severity"], "error");
    assert_eq!(
        forbidden_ordered_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        forbidden_ordered_details["checkKind"],
        "forbidden-ordered-children"
    );
    assert_eq!(
        forbidden_ordered_details["forbiddenOrderedChildren"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        forbidden_ordered_details["actualChildSequence"],
        serde_json::json!(["footer", "aside", "header", "main"])
    );
    assert_eq!(
        forbidden_ordered_details["matchedForbiddenOrderedChildren"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        forbidden_ordered_details["invalidForbiddenChildOrder"],
        true
    );

    let boundary_diagnostic = diagnostic_for_contract("section-boundary-children");
    let boundary_details = &boundary_diagnostic["details"];
    assert_eq!(boundary_diagnostic["severity"], "error");
    assert_eq!(boundary_details["behavior"], "schema:child-occurrence");
    assert_eq!(boundary_details["checkKind"], "boundary-children");
    assert_eq!(boundary_details["firstChild"], "header");
    assert_eq!(boundary_details["lastChild"], "footer");
    assert_eq!(boundary_details["actualFirstChild"], "main");
    assert_eq!(boundary_details["actualLastChild"], "footer");
    assert_eq!(boundary_details["invalidFirstChild"], true);
    assert_eq!(boundary_details["invalidLastChild"], false);

    let required_sequence_diagnostic = diagnostic_for_contract("nav-required-child-sequence");
    let required_sequence_details = &required_sequence_diagnostic["details"];
    assert_eq!(required_sequence_diagnostic["severity"], "error");
    assert_eq!(
        required_sequence_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        required_sequence_details["checkKind"],
        "required-child-sequence"
    );
    assert_eq!(
        required_sequence_details["requiredChildSequence"],
        serde_json::json!(["header", "main", "footer"])
    );
    assert_eq!(
        required_sequence_details["actualChildSequence"],
        serde_json::json!(["header", "aside", "main", "footer"])
    );
    assert_eq!(
        required_sequence_details["matchedChildSequence"],
        serde_json::json!([])
    );
    assert_eq!(required_sequence_details["invalidChildSequence"], true);

    let forbidden_sequence_diagnostic = diagnostic_for_contract("ul-forbidden-child-sequence");
    let forbidden_sequence_details = &forbidden_sequence_diagnostic["details"];
    assert_eq!(forbidden_sequence_diagnostic["severity"], "error");
    assert_eq!(
        forbidden_sequence_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        forbidden_sequence_details["checkKind"],
        "forbidden-child-sequence"
    );
    assert_eq!(
        forbidden_sequence_details["forbiddenChildSequence"],
        serde_json::json!(["li", "span"])
    );
    assert_eq!(
        forbidden_sequence_details["actualChildSequence"],
        serde_json::json!(["aside", "li", "span", "footer"])
    );
    assert_eq!(
        forbidden_sequence_details["matchedForbiddenChildSequence"],
        serde_json::json!(["li", "span"])
    );
    assert_eq!(
        forbidden_sequence_details["invalidForbiddenChildSequence"],
        true
    );

    let exact_sequence_diagnostic = diagnostic_for_contract("div-exact-child-sequence");
    let exact_sequence_details = &exact_sequence_diagnostic["details"];
    assert_eq!(exact_sequence_diagnostic["severity"], "error");
    assert_eq!(
        exact_sequence_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(exact_sequence_details["checkKind"], "exact-child-sequence");
    assert_eq!(
        exact_sequence_details["exactChildSequence"],
        serde_json::json!(["span", "p", "strong"])
    );
    assert_eq!(
        exact_sequence_details["actualChildSequence"],
        serde_json::json!(["span", "small", "p", "strong"])
    );
    assert_eq!(exact_sequence_details["invalidExactChildSequence"], true);

    let prefix_sequence_diagnostic = diagnostic_for_contract("dialog-prefix-child-sequence");
    let prefix_sequence_details = &prefix_sequence_diagnostic["details"];
    assert_eq!(prefix_sequence_diagnostic["severity"], "error");
    assert_eq!(
        prefix_sequence_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        prefix_sequence_details["checkKind"],
        "prefix-child-sequence"
    );
    assert_eq!(
        prefix_sequence_details["prefixChildSequence"],
        serde_json::json!(["header", "main"])
    );
    assert_eq!(
        prefix_sequence_details["actualChildSequence"],
        serde_json::json!(["aside", "header", "main"])
    );
    assert_eq!(prefix_sequence_details["invalidPrefixChildSequence"], true);
    assert_eq!(prefix_sequence_details["invalidSuffixChildSequence"], false);

    let suffix_sequence_diagnostic = diagnostic_for_contract("form-suffix-child-sequence");
    let suffix_sequence_details = &suffix_sequence_diagnostic["details"];
    assert_eq!(suffix_sequence_diagnostic["severity"], "error");
    assert_eq!(
        suffix_sequence_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        suffix_sequence_details["checkKind"],
        "suffix-child-sequence"
    );
    assert_eq!(
        suffix_sequence_details["suffixChildSequence"],
        serde_json::json!(["main", "footer"])
    );
    assert_eq!(
        suffix_sequence_details["actualChildSequence"],
        serde_json::json!(["main", "footer", "aside"])
    );
    assert_eq!(suffix_sequence_details["invalidPrefixChildSequence"], false);
    assert_eq!(suffix_sequence_details["invalidSuffixChildSequence"], true);

    let forbidden_prefix_sequence_diagnostic =
        diagnostic_for_contract("aside-forbidden-prefix-child-sequence");
    let forbidden_prefix_sequence_details = &forbidden_prefix_sequence_diagnostic["details"];
    assert_eq!(forbidden_prefix_sequence_diagnostic["severity"], "error");
    assert_eq!(
        forbidden_prefix_sequence_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        forbidden_prefix_sequence_details["checkKind"],
        "forbidden-prefix-child-sequence"
    );
    assert_eq!(
        forbidden_prefix_sequence_details["forbiddenPrefixChildSequence"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        forbidden_prefix_sequence_details["actualChildSequence"],
        serde_json::json!(["footer", "header", "main"])
    );
    assert_eq!(
        forbidden_prefix_sequence_details["invalidForbiddenPrefixChildSequence"],
        true
    );
    assert_eq!(
        forbidden_prefix_sequence_details["invalidForbiddenSuffixChildSequence"],
        false
    );

    let forbidden_suffix_sequence_diagnostic =
        diagnostic_for_contract("main-forbidden-suffix-child-sequence");
    let forbidden_suffix_sequence_details = &forbidden_suffix_sequence_diagnostic["details"];
    assert_eq!(forbidden_suffix_sequence_diagnostic["severity"], "error");
    assert_eq!(
        forbidden_suffix_sequence_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        forbidden_suffix_sequence_details["checkKind"],
        "forbidden-suffix-child-sequence"
    );
    assert_eq!(
        forbidden_suffix_sequence_details["forbiddenSuffixChildSequence"],
        serde_json::json!(["footer", "header"])
    );
    assert_eq!(
        forbidden_suffix_sequence_details["actualChildSequence"],
        serde_json::json!(["aside", "footer", "header"])
    );
    assert_eq!(
        forbidden_suffix_sequence_details["invalidForbiddenPrefixChildSequence"],
        false
    );
    assert_eq!(
        forbidden_suffix_sequence_details["invalidForbiddenSuffixChildSequence"],
        true
    );

    let forbidden_first_diagnostic = diagnostic_for_contract("fieldset-forbidden-first-child");
    let forbidden_first_details = &forbidden_first_diagnostic["details"];
    assert_eq!(forbidden_first_diagnostic["severity"], "error");
    assert_eq!(
        forbidden_first_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        forbidden_first_details["checkKind"],
        "forbidden-first-child"
    );
    assert_eq!(forbidden_first_details["forbiddenFirstChild"], "legend");
    assert_eq!(forbidden_first_details["actualFirstChild"], "legend");
    assert_eq!(forbidden_first_details["actualLastChild"], "input");
    assert_eq!(forbidden_first_details["invalidForbiddenFirstChild"], true);
    assert_eq!(forbidden_first_details["invalidForbiddenLastChild"], false);

    let forbidden_last_diagnostic = diagnostic_for_contract("label-forbidden-last-child");
    let forbidden_last_details = &forbidden_last_diagnostic["details"];
    assert_eq!(forbidden_last_diagnostic["severity"], "error");
    assert_eq!(
        forbidden_last_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(forbidden_last_details["checkKind"], "forbidden-last-child");
    assert_eq!(forbidden_last_details["forbiddenLastChild"], "input");
    assert_eq!(forbidden_last_details["actualFirstChild"], "span");
    assert_eq!(forbidden_last_details["actualLastChild"], "input");
    assert_eq!(forbidden_last_details["invalidForbiddenFirstChild"], false);
    assert_eq!(forbidden_last_details["invalidForbiddenLastChild"], true);

    for diagnostic in [
        ordered_diagnostic,
        forbidden_ordered_diagnostic,
        boundary_diagnostic,
        required_sequence_diagnostic,
        forbidden_sequence_diagnostic,
        exact_sequence_diagnostic,
        prefix_sequence_diagnostic,
        suffix_sequence_diagnostic,
        forbidden_prefix_sequence_diagnostic,
        forbidden_suffix_sequence_diagnostic,
        forbidden_first_diagnostic,
        forbidden_last_diagnostic,
    ] {
        assert!(diagnostic["details"]["sourceRange"]["span"]["start"].is_u64());
        assert!(diagnostic["sourceMap"]["frames"]
            .as_array()
            .is_some_and(|frames| !frames.is_empty()));
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_runtime_child_occurrence_counts_emit_structured_details() {
    const CUSTOM_SCHEMA_URI: &str = "https://example.test/ns/child-count-runtime/1";
    const CUSTOM_CONTENT_TYPE: &str = "application/vnd.example.child-count-runtime+cem";
    const CUSTOM_DIAGNOSTIC: &str = "example.layout.child_count";

    let root = test_temp_dir("cem-ml-cli-child-count-runtime");
    let manifest_path = root.join("package.cem");
    let schema_path = root.join("schema/child-count-runtime.cem");
    let valid_input_path = root.join("examples/valid-child-counts.cem");
    let invalid_input_path = root.join("examples/invalid-child-counts.cem");

    write_test_file(
        &manifest_path,
        r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="child-count-runtime" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/child-count-runtime/1"
        @source="schema/child-count-runtime.cem"
    }
    {content-type @value="application/vnd.example.child-count-runtime+cem" @primary=true}
    {namespace @prefix="demo" @uri="https://example.test/ns/child-count-runtime/1"}
}
"#,
    );
    write_test_file(
        &schema_path,
        r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@ns cemml = "https://cem.dev/ns/cem-ml/1"
@default schema

{schema @name="child-count-runtime" @namespace="https://example.test/ns/child-count-runtime/1" @version="1.0.0" |
    {uses |
        {use @schema="https://cem.dev/ns/cem-ml/1" @as="cemml"}
        {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
    }
    {content-types |
        {content-type @value="application/vnd.example.child-count-runtime+cem" @primary=true}
    }
    {namespaces |
        {namespace @prefix="demo" @uri="https://example.test/ns/child-count-runtime/1" @role="schema"}
    }
    {elements |
        {element @name="section" @children="*"}
        {element @name="article" @children="*"}
        {element @name="nav" @children="*"}
        {element @name="ul" @children="*"}
        {element @name="ol" @children="*"}
        {element @name="div" @children="*"}
        {element @name="main" @children="*"}
        {element @name="footer" @children="*"}
        {element @name="aside" @children="*"}
        {element @name="header" @children="*"}
        {element @name="p"}
        {element @name="span"}
        {element @name="small"}
        {element @name="strong" @children="*"}
        {element @name="li"}
        {element @name="mark"}
    }
    {field-contracts |
        {field-contract
            @name="section-p-count-range"
            @target="section"
            @min-children="p=2"
            @max-children="p=3"
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="child-occurrence-range"
        }
        {field-contract
            @name="article-p-count-range"
            @target="article"
            @min-children="p=1"
            @max-children="p=2"
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="child-occurrence-range"
        }
        {field-contract
            @name="nav-exact-li-count"
            @target="nav"
            @exact-children="li=2"
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="exact-children"
        }
        {field-contract
            @name="ul-total-count-range"
            @target="ul"
            @min-total-children=2
            @max-total-children=3
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="total-child-occurrence-range"
        }
        {field-contract
            @name="ol-exact-total-count"
            @target="ol"
            @exact-total-children=2
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="exact-total-children"
        }
        {field-contract
            @name="div-distinct-count-range"
            @target="div"
            @min-distinct-children=2
            @max-distinct-children=2
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="distinct-child-range"
        }
        {field-contract
            @name="div-max-one-span"
            @target="div"
            @max-one-children="span"
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="max-one-children"
        }
        {field-contract
            @name="main-exact-distinct-count"
            @target="main"
            @exact-distinct-children=2
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="exact-distinct-children"
        }
        {field-contract
            @name="footer-selected-count-range"
            @target="footer"
            @selected-children="span small"
            @min-selected-children=2
            @max-selected-children=3
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="selected-child-range"
        }
        {field-contract
            @name="aside-exact-selected-count"
            @target="aside"
            @selected-children="span small"
            @exact-selected-children=2
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="exact-selected-children"
        }
        {field-contract
            @name="header-selected-distinct-count-range"
            @target="header"
            @selected-children="span small mark"
            @min-selected-distinct-children=2
            @max-selected-distinct-children=2
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="selected-distinct-child-range"
        }
        {field-contract
            @name="strong-span-without-small-mark"
            @target="strong"
            @when-present-children="span"
            @when-absent-children="small"
            @required-children="mark"
            @diagnostic="example.layout.child_count"
            @behavior="schema:child-occurrence"
            @check-kind="conditional-required-children"
        }
    }
    {diagnostics |
        {diagnostic
            @code="example.layout.child_count"
            @severity="error"
            @behavior="schema:child-occurrence"
            @message="Layout children must satisfy the declared count contracts"
        }
    }
}
"#,
    );
    write_test_file(
        &valid_input_path,
        r#"@doc cem-ml 1

{section | {p} {p}}
{article | {p} {p}}
{nav | {li} {li}}
{ul | {li} {li}}
{ol | {li} {li}}
{div | {span} {small}}
{main | {span} {small}}
{footer | {span} {small} {mark}}
{aside | {span} {small} {mark}}
{header | {span} {small}}
{strong | {span} {mark}}
{strong | {span} {small}}
"#,
    );
    write_test_file(
        &invalid_input_path,
        r#"@doc cem-ml 1

{section | {p}}
{article | {p} {p} {p}}
{nav | {li}}
{ul | {li}}
{ol | {li}}
{div | {span} {span}}
{main | {span}}
{footer | {span} {mark}}
{aside | {span} {mark}}
{header | {span} {small} {mark}}
{strong | {span}}
"#,
    );

    let validate = |input_path: &Path| {
        cem_ml_owned(&[
            "validate".to_owned(),
            "--format".to_owned(),
            "json".to_owned(),
            "--schema-package".to_owned(),
            manifest_path.to_string_lossy().into_owned(),
            "--content-type".to_owned(),
            CUSTOM_CONTENT_TYPE.to_owned(),
            "--schema".to_owned(),
            CUSTOM_SCHEMA_URI.to_owned(),
            input_path.to_string_lossy().into_owned(),
        ])
    };

    let valid_output = validate(&valid_input_path);
    assert_eq!(
        valid_output.status.code(),
        Some(EXIT_OK),
        "child count valid stderr:\n{}",
        stderr(&valid_output)
    );
    assert!(
        stderr(&valid_output).trim().is_empty(),
        "child count valid stderr must stay empty:\n{}",
        stderr(&valid_output)
    );
    let valid_report: serde_json::Value = serde_json::from_str(stdout(&valid_output).trim())
        .expect("child count valid report is JSON");
    assert!(
        !has_diagnostic(&valid_report, CUSTOM_DIAGNOSTIC),
        "`{CUSTOM_DIAGNOSTIC}` should not be emitted for valid child counts:\n{}",
        stdout(&valid_output)
    );

    let invalid_output = validate(&invalid_input_path);
    assert_eq!(
        invalid_output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "child count invalid stderr:\n{}",
        stderr(&invalid_output)
    );
    assert!(
        stderr(&invalid_output).trim().is_empty(),
        "child count invalid stderr must stay empty:\n{}",
        stderr(&invalid_output)
    );
    let invalid_report: serde_json::Value = serde_json::from_str(stdout(&invalid_output).trim())
        .expect("child count invalid report is JSON");
    let diagnostic_for_contract = |contract: &str| {
        diagnostics(&invalid_report)
            .iter()
            .find(|diagnostic| {
                diagnostic["code"] == CUSTOM_DIAGNOSTIC
                    && diagnostic["details"]["contract"] == contract
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected `{CUSTOM_DIAGNOSTIC}` contract `{contract}` in child count report:\n{}",
                    stdout(&invalid_output)
                )
            })
    };

    let under_range_diagnostic = diagnostic_for_contract("section-p-count-range");
    let under_range_details = &under_range_diagnostic["details"];
    assert_eq!(under_range_diagnostic["severity"], "error");
    assert_eq!(under_range_details["behavior"], "schema:child-occurrence");
    assert_eq!(under_range_details["checkKind"], "child-occurrence-range");
    assert_eq!(
        under_range_details["minChildren"],
        serde_json::json!({"p": "2"})
    );
    assert_eq!(
        under_range_details["maxChildren"],
        serde_json::json!({"p": "3"})
    );
    assert_eq!(
        under_range_details["underMinChildren"],
        serde_json::json!(["p"])
    );
    assert_eq!(
        under_range_details["overMaxChildren"],
        serde_json::json!([])
    );
    assert_eq!(
        under_range_details["childCounts"],
        serde_json::json!({"p": 1})
    );

    let over_range_diagnostic = diagnostic_for_contract("article-p-count-range");
    let over_range_details = &over_range_diagnostic["details"];
    assert_eq!(over_range_diagnostic["severity"], "error");
    assert_eq!(over_range_details["behavior"], "schema:child-occurrence");
    assert_eq!(over_range_details["checkKind"], "child-occurrence-range");
    assert_eq!(
        over_range_details["underMinChildren"],
        serde_json::json!([])
    );
    assert_eq!(
        over_range_details["overMaxChildren"],
        serde_json::json!(["p"])
    );
    assert_eq!(
        over_range_details["childCounts"],
        serde_json::json!({"p": 3})
    );

    let exact_child_diagnostic = diagnostic_for_contract("nav-exact-li-count");
    let exact_child_details = &exact_child_diagnostic["details"];
    assert_eq!(exact_child_diagnostic["severity"], "error");
    assert_eq!(exact_child_details["behavior"], "schema:child-occurrence");
    assert_eq!(exact_child_details["checkKind"], "exact-children");
    assert_eq!(
        exact_child_details["exactChildren"],
        serde_json::json!({"li": "2"})
    );
    assert_eq!(
        exact_child_details["invalidExactChildren"],
        serde_json::json!(["li"])
    );
    assert_eq!(
        exact_child_details["childCounts"],
        serde_json::json!({"li": 1})
    );

    let total_range_diagnostic = diagnostic_for_contract("ul-total-count-range");
    let total_range_details = &total_range_diagnostic["details"];
    assert_eq!(total_range_diagnostic["severity"], "error");
    assert_eq!(total_range_details["behavior"], "schema:child-occurrence");
    assert_eq!(
        total_range_details["checkKind"],
        "total-child-occurrence-range"
    );
    assert_eq!(total_range_details["minTotalChildren"], 2);
    assert_eq!(total_range_details["maxTotalChildren"], 3);
    assert_eq!(total_range_details["totalChildCount"], 1);
    assert_eq!(total_range_details["underMinTotalChildren"], true);
    assert_eq!(total_range_details["overMaxTotalChildren"], false);
    assert_eq!(total_range_details["invalidExactTotalChildren"], false);

    let exact_total_diagnostic = diagnostic_for_contract("ol-exact-total-count");
    let exact_total_details = &exact_total_diagnostic["details"];
    assert_eq!(exact_total_diagnostic["severity"], "error");
    assert_eq!(exact_total_details["behavior"], "schema:child-occurrence");
    assert_eq!(exact_total_details["checkKind"], "exact-total-children");
    assert_eq!(exact_total_details["exactTotalChildren"], 2);
    assert_eq!(exact_total_details["totalChildCount"], 1);
    assert_eq!(exact_total_details["invalidExactTotalChildren"], true);

    let distinct_range_diagnostic = diagnostic_for_contract("div-distinct-count-range");
    let distinct_range_details = &distinct_range_diagnostic["details"];
    assert_eq!(distinct_range_diagnostic["severity"], "error");
    assert_eq!(
        distinct_range_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(distinct_range_details["checkKind"], "distinct-child-range");
    assert_eq!(distinct_range_details["minDistinctChildren"], 2);
    assert_eq!(distinct_range_details["maxDistinctChildren"], 2);
    assert_eq!(distinct_range_details["distinctChildCount"], 1);
    assert_eq!(distinct_range_details["underMinDistinctChildren"], true);
    assert_eq!(distinct_range_details["overMaxDistinctChildren"], false);
    assert_eq!(
        distinct_range_details["invalidExactDistinctChildren"],
        false
    );

    let max_one_child_diagnostic = diagnostic_for_contract("div-max-one-span");
    let max_one_child_details = &max_one_child_diagnostic["details"];
    assert_eq!(max_one_child_diagnostic["severity"], "error");
    assert_eq!(max_one_child_details["behavior"], "schema:child-occurrence");
    assert_eq!(max_one_child_details["checkKind"], "max-one-children");
    assert_eq!(
        max_one_child_details["maxOneChildren"],
        serde_json::json!(["span"])
    );
    assert_eq!(
        max_one_child_details["duplicateChildren"],
        serde_json::json!(["span"])
    );
    assert_eq!(
        max_one_child_details["childCounts"],
        serde_json::json!({"span": 2})
    );

    let exact_distinct_diagnostic = diagnostic_for_contract("main-exact-distinct-count");
    let exact_distinct_details = &exact_distinct_diagnostic["details"];
    assert_eq!(exact_distinct_diagnostic["severity"], "error");
    assert_eq!(
        exact_distinct_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        exact_distinct_details["checkKind"],
        "exact-distinct-children"
    );
    assert_eq!(exact_distinct_details["exactDistinctChildren"], 2);
    assert_eq!(exact_distinct_details["distinctChildCount"], 1);
    assert_eq!(exact_distinct_details["invalidExactDistinctChildren"], true);

    let selected_range_diagnostic = diagnostic_for_contract("footer-selected-count-range");
    let selected_range_details = &selected_range_diagnostic["details"];
    assert_eq!(selected_range_diagnostic["severity"], "error");
    assert_eq!(
        selected_range_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(selected_range_details["checkKind"], "selected-child-range");
    assert_eq!(
        selected_range_details["selectedChildren"],
        serde_json::json!(["small", "span"])
    );
    assert_eq!(selected_range_details["minSelectedChildren"], 2);
    assert_eq!(selected_range_details["maxSelectedChildren"], 3);
    assert_eq!(selected_range_details["selectedChildCount"], 1);
    assert_eq!(selected_range_details["underMinSelectedChildren"], true);
    assert_eq!(selected_range_details["overMaxSelectedChildren"], false);
    assert_eq!(
        selected_range_details["invalidExactSelectedChildren"],
        false
    );

    let exact_selected_diagnostic = diagnostic_for_contract("aside-exact-selected-count");
    let exact_selected_details = &exact_selected_diagnostic["details"];
    assert_eq!(exact_selected_diagnostic["severity"], "error");
    assert_eq!(
        exact_selected_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        exact_selected_details["checkKind"],
        "exact-selected-children"
    );
    assert_eq!(exact_selected_details["exactSelectedChildren"], 2);
    assert_eq!(exact_selected_details["selectedChildCount"], 1);
    assert_eq!(exact_selected_details["invalidExactSelectedChildren"], true);

    let selected_distinct_diagnostic =
        diagnostic_for_contract("header-selected-distinct-count-range");
    let selected_distinct_details = &selected_distinct_diagnostic["details"];
    assert_eq!(selected_distinct_diagnostic["severity"], "error");
    assert_eq!(
        selected_distinct_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        selected_distinct_details["checkKind"],
        "selected-distinct-child-range"
    );
    assert_eq!(
        selected_distinct_details["selectedChildren"],
        serde_json::json!(["mark", "small", "span"])
    );
    assert_eq!(selected_distinct_details["minSelectedDistinctChildren"], 2);
    assert_eq!(selected_distinct_details["maxSelectedDistinctChildren"], 2);
    assert_eq!(selected_distinct_details["selectedDistinctChildCount"], 3);
    assert_eq!(
        selected_distinct_details["underMinSelectedDistinctChildren"],
        false
    );
    assert_eq!(
        selected_distinct_details["overMaxSelectedDistinctChildren"],
        true
    );
    assert_eq!(
        selected_distinct_details["invalidExactSelectedDistinctChildren"],
        false
    );

    let conditional_child_diagnostic = diagnostic_for_contract("strong-span-without-small-mark");
    let conditional_child_details = &conditional_child_diagnostic["details"];
    assert_eq!(conditional_child_diagnostic["severity"], "error");
    assert_eq!(
        conditional_child_details["behavior"],
        "schema:child-occurrence"
    );
    assert_eq!(
        conditional_child_details["checkKind"],
        "conditional-required-children"
    );
    assert_eq!(
        conditional_child_details["requiredChildren"],
        serde_json::json!(["mark"])
    );
    assert_eq!(
        conditional_child_details["missingChildren"],
        serde_json::json!(["mark"])
    );
    assert_eq!(
        conditional_child_details["condition"],
        serde_json::json!({
            "attribute": null,
            "values": [],
            "presentAttributes": [],
            "absentAttributes": [],
            "presentChildren": ["span"],
            "absentChildren": ["small"],
        })
    );
    assert_eq!(
        conditional_child_details["childCounts"],
        serde_json::json!({"span": 1})
    );

    for diagnostic in [
        under_range_diagnostic,
        over_range_diagnostic,
        exact_child_diagnostic,
        total_range_diagnostic,
        exact_total_diagnostic,
        distinct_range_diagnostic,
        max_one_child_diagnostic,
        exact_distinct_diagnostic,
        selected_range_diagnostic,
        exact_selected_diagnostic,
        selected_distinct_diagnostic,
        conditional_child_diagnostic,
    ] {
        assert!(diagnostic["details"]["sourceRange"]["span"]["start"].is_u64());
        assert!(diagnostic["sourceMap"]["frames"]
            .as_array()
            .is_some_and(|frames| !frames.is_empty()));
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn schema_datatype_param_examples_emit_structured_definition_details() {
    let examples = [
        SchemaDefinitionDetailExample {
            name: "schema invalid datatype param length",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-length.cem",
            expected: &[
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minLength",
                    attribute: "label",
                    datatype_param: "minLength",
                    param_value: "-1",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:length",
                    attribute: "code",
                    datatype_param: "length",
                    param_value: "-1",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:maxLength",
                    attribute: "count",
                    datatype_param: "maxLength",
                    param_value: "3",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minItems",
                    attribute: "tags",
                    datatype_param: "minItems",
                    param_value: "-1",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:itemCount",
                    attribute: "aliases",
                    datatype_param: "itemCount",
                    param_value: "-1",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:maxItems",
                    attribute: "title",
                    datatype_param: "maxItems",
                    param_value: "2",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:stringForbiddenPrefixes",
                    attribute: "blockedScore",
                    datatype_param: "stringForbiddenPrefixes",
                    param_value: "draft-",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:stringForbiddenSuffixes",
                    attribute: "blockedRank",
                    datatype_param: "stringForbiddenSuffixes",
                    param_value: "-tmp",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:stringIncludes",
                    attribute: "status",
                    datatype_param: "stringIncludes",
                    param_value: "open",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:stringExcludes",
                    attribute: "body",
                    datatype_param: "stringExcludes",
                    param_value: "TODO",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minLength",
                    attribute: "titleRange",
                    datatype_param: "minLength",
                    param_value: "5",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:length",
                    attribute: "shortCode",
                    datatype_param: "length",
                    param_value: "2",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:length",
                    attribute: "longCode",
                    datatype_param: "length",
                    param_value: "5",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minItems",
                    attribute: "tagRange",
                    datatype_param: "minItems",
                    param_value: "4",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:itemCount",
                    attribute: "shortTags",
                    datatype_param: "itemCount",
                    param_value: "2",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:itemCount",
                    attribute: "longAliases",
                    datatype_param: "itemCount",
                    param_value: "5",
                },
            ],
        },
        SchemaDefinitionDetailExample {
            name: "schema invalid datatype param bound",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-bound.cem",
            expected: &[
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minInclusive",
                    attribute: "priority",
                    datatype_param: "minInclusive",
                    param_value: "0.5",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minInclusive",
                    attribute: "title",
                    datatype_param: "minInclusive",
                    param_value: "1",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:maxInclusive",
                    attribute: "untyped",
                    datatype_param: "maxInclusive",
                    param_value: "10",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minInclusive",
                    attribute: "closed",
                    datatype_param: "minInclusive",
                    param_value: "5",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minInclusive",
                    attribute: "openUpper",
                    datatype_param: "minInclusive",
                    param_value: "5",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minExclusive",
                    attribute: "openLower",
                    datatype_param: "minExclusive",
                    param_value: "3.5",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:minExclusive",
                    attribute: "openBoth",
                    datatype_param: "minExclusive",
                    param_value: "1.0",
                },
            ],
        },
        SchemaDefinitionDetailExample {
            name: "schema invalid datatype param pattern",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-pattern.cem",
            expected: &[
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pattern",
                    attribute: "code",
                    datatype_param: "pattern",
                    param_value: "[",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pattern",
                    attribute: "count",
                    datatype_param: "pattern",
                    param_value: "[0-9]+",
                },
            ],
        },
        SchemaDefinitionDetailExample {
            name: "schema invalid datatype param digits",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-digits.cem",
            expected: &[
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:totalDigits",
                    attribute: "serial",
                    datatype_param: "totalDigits",
                    param_value: "0",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:fractionDigits",
                    attribute: "ratio",
                    datatype_param: "fractionDigits",
                    param_value: "-1",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:fractionDigits",
                    attribute: "code",
                    datatype_param: "fractionDigits",
                    param_value: "2",
                },
            ],
        },
        SchemaDefinitionDetailExample {
            name: "schema invalid datatype param uri/media",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-datatype-param-uri-media.cem",
            expected: &[
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathPrefixes",
                    attribute: "template",
                    datatype_param: "pathPrefixes",
                    param_value: "/absolute",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathPrefixes",
                    attribute: "template",
                    datatype_param: "pathPrefixes",
                    param_value: "./../bad",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenPrefixes",
                    attribute: "blockedTemplate",
                    datatype_param: "pathForbiddenPrefixes",
                    param_value: "/private",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenPrefixes",
                    attribute: "blockedTemplate",
                    datatype_param: "pathForbiddenPrefixes",
                    param_value: "./../secret",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathDirectoryNames",
                    attribute: "directory",
                    datatype_param: "pathDirectoryNames",
                    param_value: "bad/name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenDirectoryNames",
                    attribute: "blockedDirectory",
                    datatype_param: "pathForbiddenDirectoryNames",
                    param_value: "bad/name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathExtensions",
                    attribute: "script",
                    datatype_param: "pathExtensions",
                    param_value: ".cem",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenExtensions",
                    attribute: "blockedScript",
                    datatype_param: "pathForbiddenExtensions",
                    param_value: ".bak",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathBasenames",
                    attribute: "image",
                    datatype_param: "pathBasenames",
                    param_value: "bad/name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenBasenames",
                    attribute: "blockedImage",
                    datatype_param: "pathForbiddenBasenames",
                    param_value: "bad/name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathPrefixes",
                    attribute: "caption",
                    datatype_param: "pathPrefixes",
                    param_value: "./templates/",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenPrefixes",
                    attribute: "blockedCaption",
                    datatype_param: "pathForbiddenPrefixes",
                    param_value: "./private/",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathDirectoryNames",
                    attribute: "directoryLabel",
                    datatype_param: "pathDirectoryNames",
                    param_value: "templates",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenDirectoryNames",
                    attribute: "directoryBlockedLabel",
                    datatype_param: "pathForbiddenDirectoryNames",
                    param_value: "private",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathExtensions",
                    attribute: "summary",
                    datatype_param: "pathExtensions",
                    param_value: "cem",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenExtensions",
                    attribute: "blockedSummary",
                    datatype_param: "pathForbiddenExtensions",
                    param_value: "tmp",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathBasenames",
                    attribute: "basenameLabel",
                    datatype_param: "pathBasenames",
                    param_value: "card.cem",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:pathForbiddenBasenames",
                    attribute: "blockedBasenameLabel",
                    datatype_param: "pathForbiddenBasenames",
                    param_value: "secret.cem",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriPathBasenames",
                    attribute: "uriFile",
                    datatype_param: "uriPathBasenames",
                    param_value: "bad/name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenPathBasenames",
                    attribute: "blockedUriFile",
                    datatype_param: "uriForbiddenPathBasenames",
                    param_value: "bad/name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriPathBasenames",
                    attribute: "uriFileLabel",
                    datatype_param: "uriPathBasenames",
                    param_value: "schema.cem",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenPathBasenames",
                    attribute: "uriFileBlockedLabel",
                    datatype_param: "uriForbiddenPathBasenames",
                    param_value: "secret.cem",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriSchemes",
                    attribute: "href",
                    datatype_param: "uriSchemes",
                    param_value: "1bad",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriHosts",
                    attribute: "cdn",
                    datatype_param: "uriHosts",
                    param_value: "bad/host",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriHosts",
                    attribute: "linkLabel",
                    datatype_param: "uriHosts",
                    param_value: "api.example.test",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriPorts",
                    attribute: "portal",
                    datatype_param: "uriPorts",
                    param_value: "0443",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriPorts",
                    attribute: "portal",
                    datatype_param: "uriPorts",
                    param_value: "65536",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriPorts",
                    attribute: "portLabel",
                    datatype_param: "uriPorts",
                    param_value: "443",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriRequiresAuthority",
                    attribute: "remote",
                    datatype_param: "uriRequiresAuthority",
                    param_value: "maybe",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriPathPrefixes",
                    attribute: "asset",
                    datatype_param: "uriPathPrefixes",
                    param_value: "assets",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenPathPrefixes",
                    attribute: "blockedAsset",
                    datatype_param: "uriForbiddenPathPrefixes",
                    param_value: "private",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenPathPrefixes",
                    attribute: "assetBlockedLabel",
                    datatype_param: "uriForbiddenPathPrefixes",
                    param_value: "/private/",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriPathExtensions",
                    attribute: "download",
                    datatype_param: "uriPathExtensions",
                    param_value: ".json",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenPathExtensions",
                    attribute: "blockedDownload",
                    datatype_param: "uriForbiddenPathExtensions",
                    param_value: ".bak",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriPathExtensions",
                    attribute: "downloadLabel",
                    datatype_param: "uriPathExtensions",
                    param_value: "cem",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenPathExtensions",
                    attribute: "downloadBlockedLabel",
                    datatype_param: "uriForbiddenPathExtensions",
                    param_value: "tmp",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueries",
                    attribute: "query",
                    datatype_param: "uriQueries",
                    param_value: "?bad",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueries",
                    attribute: "queryLabel",
                    datatype_param: "uriQueries",
                    param_value: "view=resource",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenQueries",
                    attribute: "queryBlocked",
                    datatype_param: "uriForbiddenQueries",
                    param_value: "?bad",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenQueries",
                    attribute: "queryBlocked",
                    datatype_param: "uriForbiddenQueries",
                    param_value: "debug=true",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenQueries",
                    attribute: "queryBlockedLabel",
                    datatype_param: "uriForbiddenQueries",
                    param_value: "debug=true",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryParameters",
                    attribute: "queryParams",
                    datatype_param: "uriQueryParameters",
                    param_value: "bad=name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryParameters",
                    attribute: "queryParamsLabel",
                    datatype_param: "uriQueryParameters",
                    param_value: "view",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryParameterValues",
                    attribute: "queryValue",
                    datatype_param: "uriQueryParameterValues",
                    param_value: "bad",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryParameterValues",
                    attribute: "queryValue",
                    datatype_param: "uriQueryParameterValues",
                    param_value: "bad&name=value",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryParameterValues",
                    attribute: "queryValueLabel",
                    datatype_param: "uriQueryParameterValues",
                    param_value: "view=resource",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryForbiddenParameters",
                    attribute: "queryForbidden",
                    datatype_param: "uriQueryForbiddenParameters",
                    param_value: "bad=name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryForbiddenParameters",
                    attribute: "queryForbiddenLabel",
                    datatype_param: "uriQueryForbiddenParameters",
                    param_value: "debug",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryRequiredParameters",
                    attribute: "queryRequired",
                    datatype_param: "uriQueryRequiredParameters",
                    param_value: "bad=name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryRequiredParameters",
                    attribute: "queryRequiredLabel",
                    datatype_param: "uriQueryRequiredParameters",
                    param_value: "view",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryRequiredParameters",
                    attribute: "queryPresenceConflict",
                    datatype_param: "uriQueryRequiredParameters",
                    param_value: "debug",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriQueryParameterValues",
                    attribute: "queryValueConflict",
                    datatype_param: "uriQueryParameterValues",
                    param_value: "debug",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriFragments",
                    attribute: "anchor",
                    datatype_param: "uriFragments",
                    param_value: "#bad",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriFragments",
                    attribute: "anchorLabel",
                    datatype_param: "uriFragments",
                    param_value: "overview",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenFragments",
                    attribute: "anchorBlocked",
                    datatype_param: "uriForbiddenFragments",
                    param_value: "#bad",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenFragments",
                    attribute: "anchorBlocked",
                    datatype_param: "uriForbiddenFragments",
                    param_value: "debug",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:uriForbiddenFragments",
                    attribute: "anchorBlockedLabel",
                    datatype_param: "uriForbiddenFragments",
                    param_value: "debug",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypes",
                    attribute: "format",
                    datatype_param: "mediaTypes",
                    param_value: "text",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeSuffixes",
                    attribute: "label",
                    datatype_param: "mediaTypeSuffixes",
                    param_value: "json",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeTypes",
                    attribute: "typed",
                    datatype_param: "mediaTypeTypes",
                    param_value: "bad/type",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeTypes",
                    attribute: "typeLabel",
                    datatype_param: "mediaTypeTypes",
                    param_value: "application",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeSubtypes",
                    attribute: "subtyped",
                    datatype_param: "mediaTypeSubtypes",
                    param_value: "bad/subtype",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeSubtypes",
                    attribute: "subtitle",
                    datatype_param: "mediaTypeSubtypes",
                    param_value: "json",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeSuffixes",
                    attribute: "structured",
                    datatype_param: "mediaTypeSuffixes",
                    param_value: "bad=suffix",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeForbiddenTypes",
                    attribute: "blockedTyped",
                    datatype_param: "mediaTypeForbiddenTypes",
                    param_value: "bad/type",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeForbiddenTypes",
                    attribute: "typeBlockedLabel",
                    datatype_param: "mediaTypeForbiddenTypes",
                    param_value: "image",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeForbiddenSubtypes",
                    attribute: "blockedSubtyped",
                    datatype_param: "mediaTypeForbiddenSubtypes",
                    param_value: "bad/subtype",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeForbiddenSubtypes",
                    attribute: "subtitleBlocked",
                    datatype_param: "mediaTypeForbiddenSubtypes",
                    param_value: "html",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeForbiddenSuffixes",
                    attribute: "blockedStructured",
                    datatype_param: "mediaTypeForbiddenSuffixes",
                    param_value: "bad=suffix",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeForbiddenSuffixes",
                    attribute: "structuredBlockedLabel",
                    datatype_param: "mediaTypeForbiddenSuffixes",
                    param_value: "json",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeParameters",
                    attribute: "payload",
                    datatype_param: "mediaTypeParameters",
                    param_value: "bad=name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeParameterValues",
                    attribute: "encoding",
                    datatype_param: "mediaTypeParameterValues",
                    param_value: "bad",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeParameterValues",
                    attribute: "encoding",
                    datatype_param: "mediaTypeParameterValues",
                    param_value: "bad/name=value",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeParameterValues",
                    attribute: "description",
                    datatype_param: "mediaTypeParameterValues",
                    param_value: "charset=utf-8",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeForbiddenParameters",
                    attribute: "legacy",
                    datatype_param: "mediaTypeForbiddenParameters",
                    param_value: "bad=name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeForbiddenParameters",
                    attribute: "title",
                    datatype_param: "mediaTypeForbiddenParameters",
                    param_value: "profile",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeRequiredParameters",
                    attribute: "profiled",
                    datatype_param: "mediaTypeRequiredParameters",
                    param_value: "bad=name",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeRequiredParameters",
                    attribute: "mediaPresenceConflict",
                    datatype_param: "mediaTypeRequiredParameters",
                    param_value: "profile",
                },
                SchemaDefinitionDetailExpectation {
                    code: "cem.schema_definition.invalid_datatype_param",
                    severity: "error",
                    check_kind: "datatype-param:mediaTypeParameterValues",
                    attribute: "mediaValueConflict",
                    datatype_param: "mediaTypeParameterValues",
                    param_value: "profile",
                },
            ],
        },
    ];

    for example in examples {
        let path = workspace_path(example.path);
        assert!(
            path.exists(),
            "schema datatype-param validation example `{}` is missing at {}",
            example.name,
            path.display()
        );

        let output = validate_example(
            &ValidationExample {
                name: example.name,
                path: example.path,
                content_type: CEM_SCHEMA_CONTENT_TYPE,
                schema_uri: CEM_SCHEMA_URI,
                expected_exit: EXIT_HARD_FAILURE,
                expected_diagnostics: &[],
            },
            &path,
        );
        assert_eq!(
            output.status.code(),
            Some(EXIT_HARD_FAILURE),
            "{} stderr:\n{}",
            example.name,
            stderr(&output)
        );
        assert!(
            stderr(&output).trim().is_empty(),
            "{} stderr must stay empty:\n{}",
            example.name,
            stderr(&output)
        );

        let report: serde_json::Value = serde_json::from_str(stdout(&output).trim())
            .unwrap_or_else(|err| panic!("{} stdout is validation JSON: {err}", example.name));
        for expected in example.expected {
            assert!(
                has_schema_definition_detail(&report, expected),
                "{} expected structured schema definition diagnostic {:?} in {}",
                example.name,
                expected,
                stdout(&output)
            );
        }
    }
}

#[test]
fn schema_field_contract_examples_emit_structured_definition_details() {
    let path = workspace_path(
        "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-child-sequence.cem",
    );
    assert!(
        path.exists(),
        "schema field-contract validation example is missing at {}",
        path.display()
    );

    let output = validate_example(
        &ValidationExample {
            name: "schema invalid field contract child sequence",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-child-sequence.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &[],
        },
        &path,
    );
    assert_eq!(
        output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "schema invalid field contract child sequence stderr:\n{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).trim().is_empty(),
        "schema invalid field contract child sequence stderr must stay empty:\n{}",
        stderr(&output)
    );

    let report: serde_json::Value = serde_json::from_str(stdout(&output).trim())
        .expect("schema invalid field contract child sequence stdout is validation JSON");
    let detail_for = |contract: &str, conflict: &str| {
        diagnostics(&report)
            .iter()
            .find(|diagnostic| {
                diagnostic["code"] == "cem.schema_definition.invalid_field_contract"
                    && diagnostic["severity"] == "error"
                    && diagnostic["details"]["checkKind"] == "field-contract-child-sequence"
                    && diagnostic["details"]["contract"] == contract
                    && diagnostic["details"]["conflict"] == conflict
                    && diagnostic["sourceMap"]["frames"]
                        .as_array()
                        .is_some_and(|frames| !frames.is_empty())
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected invalid field-contract child sequence detail for `{contract}` / `{conflict}` in {}",
                    stdout(&output)
                )
            })
    };

    let diagnostic = detail_for(
        "bad-required-forbidden-sequence",
        "required-child-sequence/forbidden-child-sequence",
    );
    assert_eq!(
        diagnostic["details"]["requiredChildSequence"],
        serde_json::json!(["header", "main"])
    );
    assert_eq!(
        diagnostic["details"]["forbiddenChildSequence"],
        serde_json::json!(["header", "main"])
    );

    let diagnostic = detail_for(
        "bad-exact-prefix",
        "exact-child-sequence/prefix-child-sequence",
    );
    assert_eq!(
        diagnostic["details"]["exactChildSequence"],
        serde_json::json!(["header", "footer"])
    );
    assert_eq!(
        diagnostic["details"]["prefixChildSequence"],
        serde_json::json!(["header", "main"])
    );

    for (contract, conflict) in [
        ("bad-first-boundary", "first-child/forbidden-first-child"),
        ("bad-last-boundary", "last-child/forbidden-last-child"),
        (
            "bad-prefix-forbidden-prefix",
            "prefix-child-sequence/forbidden-prefix-child-sequence",
        ),
        (
            "bad-suffix-forbidden-suffix",
            "suffix-child-sequence/forbidden-suffix-child-sequence",
        ),
        (
            "bad-exact-required",
            "exact-child-sequence/required-child-sequence",
        ),
        (
            "bad-exact-forbidden",
            "exact-child-sequence/forbidden-child-sequence",
        ),
    ] {
        detail_for(contract, conflict);
    }

    let path = workspace_path(
        "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-presence.cem",
    );
    assert!(
        path.exists(),
        "schema field-contract presence validation example is missing at {}",
        path.display()
    );
    let output = validate_example(
        &ValidationExample {
            name: "schema invalid field contract presence",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-presence.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &[],
        },
        &path,
    );
    assert_eq!(
        output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "schema invalid field contract presence stderr:\n{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).trim().is_empty(),
        "schema invalid field contract presence stderr must stay empty:\n{}",
        stderr(&output)
    );
    let report: serde_json::Value = serde_json::from_str(stdout(&output).trim())
        .expect("schema invalid field contract presence stdout is validation JSON");
    let presence_detail_for = |contract: &str, conflict: &str| {
        diagnostics(&report)
            .iter()
            .find(|diagnostic| {
                diagnostic["code"] == "cem.schema_definition.invalid_field_contract"
                    && diagnostic["severity"] == "error"
                    && diagnostic["details"]["checkKind"] == "field-contract-presence"
                    && diagnostic["details"]["contract"] == contract
                    && diagnostic["details"]["conflict"] == conflict
                    && diagnostic["sourceMap"]["frames"]
                        .as_array()
                        .is_some_and(|frames| !frames.is_empty())
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected invalid field-contract presence detail for `{contract}` / `{conflict}` in {}",
                    stdout(&output)
                )
            })
    };

    let diagnostic = presence_detail_for(
        "bad-required-forbidden-attribute",
        "required-attributes/forbidden-attributes",
    );
    assert_eq!(
        diagnostic["details"]["conflictingFields"],
        serde_json::json!(["id"])
    );

    let diagnostic = presence_detail_for(
        "bad-required-one-forbidden-child",
        "required-one-child/forbidden-children",
    );
    assert_eq!(
        diagnostic["details"]["requiredOneChild"],
        serde_json::json!(["header", "main"])
    );
    assert_eq!(
        diagnostic["details"]["conflictingChildren"],
        serde_json::json!(["header", "main"])
    );

    let diagnostic = presence_detail_for(
        "bad-required-unaccepted-child",
        "required-children/accepted-children",
    );
    assert_eq!(
        diagnostic["details"]["acceptedChildren"],
        serde_json::json!(["header"])
    );
    assert_eq!(
        diagnostic["details"]["conflictingChildren"],
        serde_json::json!(["main"])
    );

    for (contract, conflict) in [
        (
            "bad-required-one-forbidden-attributes",
            "required-one-attributes/forbidden-attributes",
        ),
        (
            "bad-required-forbidden-child",
            "required-children/forbidden-children",
        ),
        (
            "bad-required-one-unaccepted-child",
            "required-one-child/accepted-children",
        ),
    ] {
        presence_detail_for(contract, conflict);
    }

    let path = workspace_path(
        "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-condition.cem",
    );
    assert!(
        path.exists(),
        "schema field-contract condition validation example is missing at {}",
        path.display()
    );
    let output = validate_example(
        &ValidationExample {
            name: "schema invalid field contract condition",
            path: "packages/cem_ml/schema-packages/schema/v1/examples/invalid-field-contract-condition.cem",
            content_type: CEM_SCHEMA_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &[],
        },
        &path,
    );
    assert_eq!(
        output.status.code(),
        Some(EXIT_HARD_FAILURE),
        "schema invalid field contract condition stderr:\n{}",
        stderr(&output)
    );
    assert!(
        stderr(&output).trim().is_empty(),
        "schema invalid field contract condition stderr must stay empty:\n{}",
        stderr(&output)
    );
    let report: serde_json::Value = serde_json::from_str(stdout(&output).trim())
        .expect("schema invalid field contract condition stdout is validation JSON");
    let condition_detail_for = |contract: &str, conflict: &str| {
        diagnostics(&report)
            .iter()
            .find(|diagnostic| {
                diagnostic["code"] == "cem.schema_definition.invalid_field_contract"
                    && diagnostic["severity"] == "error"
                    && diagnostic["details"]["checkKind"] == "field-contract-condition"
                    && diagnostic["details"]["contract"] == contract
                    && diagnostic["details"]["conflict"] == conflict
                    && diagnostic["sourceMap"]["frames"]
                        .as_array()
                        .is_some_and(|frames| !frames.is_empty())
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected invalid field-contract condition detail for `{contract}` / `{conflict}` in {}",
                    stdout(&output)
                )
            })
    };

    let diagnostic =
        condition_detail_for("bad-values-without-attribute", "when-values/when-attribute");
    assert_eq!(
        diagnostic["details"]["whenValues"],
        serde_json::json!(["remote"])
    );

    let diagnostic = condition_detail_for(
        "bad-attribute-present-absent",
        "when-present-attributes/when-absent-attributes",
    );
    assert_eq!(diagnostic["details"]["whenAttribute"], "kind");
    assert_eq!(
        diagnostic["details"]["conflictingFields"],
        serde_json::json!(["kind"])
    );

    let diagnostic = condition_detail_for(
        "bad-any-present-all-absent-attributes",
        "when-any-present-attributes/when-absent-attributes",
    );
    assert_eq!(
        diagnostic["details"]["conflictingFields"],
        serde_json::json!(["source", "token"])
    );

    let diagnostic = condition_detail_for(
        "bad-any-present-all-absent-children",
        "when-any-present-children/when-absent-children",
    );
    assert_eq!(
        diagnostic["details"]["conflictingChildren"],
        serde_json::json!(["fallback", "reference"])
    );

    for (contract, conflict) in [
        (
            "bad-all-present-absent-attributes",
            "when-present-attributes/when-absent-attributes",
        ),
        (
            "bad-any-absent-all-present-attributes",
            "when-any-absent-attributes/when-present-attributes",
        ),
        (
            "bad-all-present-absent-children",
            "when-present-children/when-absent-children",
        ),
        (
            "bad-any-absent-all-present-children",
            "when-any-absent-children/when-present-children",
        ),
    ] {
        condition_detail_for(contract, conflict);
    }
}

#[test]
fn schema_package_engine_behavior_examples_emit_structured_details() {
    let examples = [
        DetailedValidationExample {
            name: "schema-package invalid missing required attribute",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-missing-required-attribute.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.package_check",
                severity: "error",
                behavior: "schema:child-occurrence",
                check_kind: "child-occurrence",
                contract: "package-required-children",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid primary content type",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-primary-content-type.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.content_type_conflict",
                severity: "error",
                behavior: "single-primary-content-type",
                check_kind: "single-primary-content-type",
                contract: "single-primary-content-type",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid converter contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[
                DiagnosticDetailExpectation {
                    code: "cem.schema_model.invalid_attribute_value",
                    severity: "error",
                    behavior: "schema:value-vocabulary",
                    check_kind: "value-vocabulary",
                    contract: "attribute-values:template-content-type",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_model.invalid_attribute_datatype_param",
                    severity: "error",
                    behavior: "schema:datatype-param",
                    check_kind: "datatype-param:minInclusive",
                    contract: "attribute-datatype-param:cost:minInclusive",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.converter_check",
                    severity: "error",
                    behavior: "schema:required-fields",
                    check_kind: "required-fields",
                    contract: "converter-cemt-template-identity",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.converter_check",
                    severity: "error",
                    behavior: "schema:child-occurrence",
                    check_kind: "child-occurrence",
                    contract: "converter-from-to-endpoints",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.converter_check",
                    severity: "error",
                    behavior: "schema:reference-resolution",
                    check_kind: "endpoint-content-type-schema",
                    contract: "converter-endpoint-schema-content-type-match",
                },
            ],
        },
        DetailedValidationExample {
            name: "schema-package invalid converter runtime constraints",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-runtime-constraints.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[
                DiagnosticDetailExpectation {
                    code: "cem.schema_model.invalid_attribute_type",
                    severity: "error",
                    behavior: "schema:scalar-type",
                    check_kind: "type:boolean",
                    contract: "attribute-type:streamable",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.converter_check",
                    severity: "error",
                    behavior: "schema:forbidden-fields",
                    check_kind: "forbidden-fields",
                    contract: "converter-rust-template-forbidden",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.converter_check",
                    severity: "error",
                    behavior: "schema:dependent-required-fields",
                    check_kind: "dependent-required-fields",
                    contract: "converter-cemt-fallback-reason",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.converter_check",
                    severity: "error",
                    behavior: "schema:mutual-exclusion",
                    check_kind: "mutual-exclusion",
                    contract: "converter-planner-state",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.converter_check",
                    severity: "error",
                    behavior: "schema:field-dependency",
                    check_kind: "converter-output-contract",
                    contract: "converter-output-formatter-profile",
                },
            ],
        },
        DetailedValidationExample {
            name: "schema-package invalid converter template unreadable",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-template-unreadable.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.converter_check",
                severity: "error",
                behavior: "schema:resource-readable",
                check_kind: "converter-template-source-readable",
                contract: "converter-template-output-stage-contract",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid converter template contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-converter-template-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.converter_check",
                severity: "error",
                behavior: "schema:resource-parse",
                check_kind: "converter-template-contract",
                contract: "converter-template-output-stage-contract",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid schema source unreadable",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-source-unreadable.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.schema_source_unreadable",
                severity: "error",
                behavior: "schema:resource-readable",
                check_kind: "schema-source-readable",
                contract: "schema-source-readable",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid schema source invalid",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-source-invalid.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.schema_source_invalid",
                severity: "error",
                behavior: "schema:resource-parse",
                check_kind: "schema-source-valid",
                contract: "schema-source-valid",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid schema metadata",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-schema-metadata.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.schema_uri_mismatch",
                    severity: "error",
                    behavior: "schema:reference-resolution",
                    check_kind: "schema-uri-consistency",
                    contract: "schema-uri-consistency",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.schema_content_type_mismatch",
                    severity: "error",
                    behavior: "schema:reference-resolution",
                    check_kind: "schema-content-type-consistency",
                    contract: "schema-content-type-consistency",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.schema_namespace_mismatch",
                    severity: "error",
                    behavior: "schema:reference-resolution",
                    check_kind: "schema-namespace-consistency",
                    contract: "schema-namespace-consistency",
                },
            ],
        },
        DetailedValidationExample {
            name: "schema-package invalid artifact layout",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-layout.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.artifact_check",
                severity: "error",
                behavior: "schema:path-layout",
                check_kind: "path-layout",
                contract: "artifact-formatter-layout",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid artifact contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.artifact_check",
                severity: "error",
                behavior: "schema:reference-resolution",
                check_kind: "artifact-function-contract",
                contract: "artifact-output-stage-contract",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid artifact function missing",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-function-missing.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.artifact_check",
                severity: "error",
                behavior: "schema:reference-resolution",
                check_kind: "artifact-function-declared",
                contract: "artifact-output-stage-contract",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid artifact source unreadable",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-source-unreadable.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.artifact_check",
                severity: "error",
                behavior: "schema:resource-readable",
                check_kind: "artifact-source-readable",
                contract: "artifact-output-stage-contract",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid artifact source parse",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-source-parse.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[DiagnosticDetailExpectation {
                code: "cem.schema_package.artifact_check",
                severity: "error",
                behavior: "schema:resource-parse",
                check_kind: "artifact-cemt-valid",
                contract: "artifact-output-stage-contract",
            }],
        },
        DetailedValidationExample {
            name: "schema-package invalid example contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-example-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.example_check",
                    severity: "error",
                    behavior: "schema:field-dependency",
                    check_kind: "dependent-required-fields",
                    contract: "example-failing-diagnostics",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.example_check",
                    severity: "error",
                    behavior: "schema:reference-resolution",
                    check_kind: "example-content-type-schema",
                    contract: "example-contract",
                },
            ],
        },
        DetailedValidationExample {
            name: "schema-package invalid example source contract",
            path: "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-example-source-contract.cem",
            content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            schema_uri: CEM_SCHEMA_PACKAGE_URI,
            expected: &[
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.example_check",
                    severity: "error",
                    behavior: "schema:resource-readable",
                    check_kind: "example-source-readable",
                    contract: "example-contract",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.example_check",
                    severity: "error",
                    behavior: "schema:resource-parse",
                    check_kind: "example-source-validation",
                    contract: "example-contract",
                },
                DiagnosticDetailExpectation {
                    code: "cem.schema_package.example_check",
                    severity: "error",
                    behavior: "schema:reference-resolution",
                    check_kind: "example-expected-diagnostics",
                    contract: "example-contract",
                },
            ],
        },
    ];

    for example in examples {
        let path = workspace_path(example.path);
        assert!(
            path.exists(),
            "detailed schema validation example `{}` is missing at {}",
            example.name,
            path.display()
        );

        let output = validate_example(
            &ValidationExample {
                name: example.name,
                path: example.path,
                content_type: example.content_type,
                schema_uri: example.schema_uri,
                expected_exit: EXIT_HARD_FAILURE,
                expected_diagnostics: &[],
            },
            &path,
        );
        assert_eq!(
            output.status.code(),
            Some(EXIT_HARD_FAILURE),
            "{} stderr:\n{}",
            example.name,
            stderr(&output)
        );
        assert!(
            stderr(&output).trim().is_empty(),
            "{} stderr must stay empty:\n{}",
            example.name,
            stderr(&output)
        );

        let report: serde_json::Value = serde_json::from_str(stdout(&output).trim())
            .unwrap_or_else(|err| panic!("{} stdout is validation JSON: {err}", example.name));
        for expected in example.expected {
            assert!(
                has_diagnostic_detail(&report, expected),
                "{} expected structured diagnostic {:?} in {}",
                example.name,
                expected,
                stdout(&output)
            );
        }
    }
}

#[test]
fn schema_package_resource_behavior_examples_emit_structured_payloads() {
    let validate_schema_package_example = |name: &'static str, relative_path: &'static str| {
        let path = workspace_path(relative_path);
        assert!(
            path.exists(),
            "schema-package resource behavior example `{name}` is missing at {}",
            path.display()
        );

        let output = validate_example(
            &ValidationExample {
                name,
                path: relative_path,
                content_type: CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
                schema_uri: CEM_SCHEMA_PACKAGE_URI,
                expected_exit: EXIT_HARD_FAILURE,
                expected_diagnostics: &[],
            },
            &path,
        );
        assert_eq!(
            output.status.code(),
            Some(EXIT_HARD_FAILURE),
            "{name} stderr:\n{}",
            stderr(&output)
        );
        assert!(
            stderr(&output).trim().is_empty(),
            "{name} stderr must stay empty:\n{}",
            stderr(&output)
        );
        serde_json::from_str::<serde_json::Value>(stdout(&output).trim())
            .unwrap_or_else(|err| panic!("{name} stdout is validation JSON: {err}"))
    };

    let unreadable_report = validate_schema_package_example(
        "schema-package invalid artifact source unreadable",
        "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-source-unreadable.cem",
    );
    let unreadable_expected = DiagnosticDetailExpectation {
        code: "cem.schema_package.artifact_check",
        severity: "error",
        behavior: "schema:resource-readable",
        check_kind: "artifact-source-readable",
        contract: "artifact-output-stage-contract",
    };
    let unreadable = find_diagnostic_detail(&unreadable_report, &unreadable_expected)
        .unwrap_or_else(|| {
            panic!(
                "expected structured resource-readable diagnostic {:?} in {}",
                unreadable_expected, unreadable_report
            )
        });
    let details = &unreadable["details"];
    assert_eq!(details["element"], "artifact");
    assert_eq!(details["target"], "artifact");
    assert_eq!(details["path"], "formatters/missing.cemt");
    assert_eq!(details["invalidFields"], serde_json::json!(["path"]));
    assert_eq!(details["invalidValues"]["path"], "formatters/missing.cemt");
    assert_eq!(details["actualValues"]["function-name"], "bad.missing");
    assert!(details["error"]
        .as_str()
        .is_some_and(|error| error.contains("No such file")));

    let parse_report = validate_schema_package_example(
        "schema-package invalid artifact source parse",
        "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-artifact-source-parse.cem",
    );
    let parse_expected = DiagnosticDetailExpectation {
        code: "cem.schema_package.artifact_check",
        severity: "error",
        behavior: "schema:resource-parse",
        check_kind: "artifact-cemt-valid",
        contract: "artifact-output-stage-contract",
    };
    let parse = find_diagnostic_detail(&parse_report, &parse_expected).unwrap_or_else(|| {
        panic!(
            "expected structured resource-parse diagnostic {:?} in {}",
            parse_expected, parse_report
        )
    });
    let details = &parse["details"];
    assert_eq!(details["element"], "artifact");
    assert_eq!(details["target"], "artifact");
    assert_eq!(
        details["path"],
        "formatters/invalid-artifact-source-parse.cemt"
    );
    assert_eq!(details["invalidFields"], serde_json::json!(["path"]));
    assert_eq!(
        details["invalidValues"]["path"],
        "formatters/invalid-artifact-source-parse.cemt"
    );
    assert_eq!(details["actualValues"]["function-name"], "bad.invalid");
    assert_eq!(
        details["sourceDiagnostic"]["code"],
        "cem.transform_template.declaration_required"
    );
    assert_eq!(details["sourceDiagnostic"]["severity"], "Fatal");
    assert!(details["sourceDiagnostic"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("@category")));

    let reference_report = validate_schema_package_example(
        "schema-package invalid example contract",
        "packages/cem_ml/schema-packages/schema-package/v1/examples/invalid-example-contract.cem",
    );
    let reference_expected = DiagnosticDetailExpectation {
        code: "cem.schema_package.example_check",
        severity: "error",
        behavior: "schema:reference-resolution",
        check_kind: "example-content-type-schema",
        contract: "example-contract",
    };
    let reference =
        find_diagnostic_detail(&reference_report, &reference_expected).unwrap_or_else(|| {
            panic!(
                "expected structured reference-resolution diagnostic {:?} in {}",
                reference_expected, reference_report
            )
        });
    let details = &reference["details"];
    assert_eq!(details["element"], "example");
    assert_eq!(details["target"], "example");
    assert_eq!(details["exampleId"], "wrong-content-type");
    assert_eq!(details["schema"], XML_SCHEMA_URI);
    assert_eq!(
        details["invalidFields"],
        serde_json::json!(["content-type"])
    );
    assert_eq!(details["invalidValues"]["content-type"], HTML_CONTENT_TYPE);
    assert_eq!(details["actualValues"]["id"], "wrong-content-type");
    assert_eq!(details["actualValues"]["content-type"], HTML_CONTENT_TYPE);
    let expected_content_types = details["expectedValues"]["content-type"]
        .as_array()
        .expect("reference-resolution expected content-type values");
    assert!(
        expected_content_types
            .iter()
            .any(|value| value == XML_CONTENT_TYPE),
        "expected XML content type in reference-resolution payload: {details}"
    );
    assert!(
        expected_content_types
            .iter()
            .any(|value| value == "text/xml"),
        "expected text XML content type in reference-resolution payload: {details}"
    );
}

#[test]
fn builtin_schema_package_manifests_validate_through_cli() {
    let manifest_paths = schema_package_manifest_paths();
    let actual_package_ids = manifest_paths
        .iter()
        .map(|path| schema_package_id_from_manifest_path(path))
        .collect::<BTreeSet<_>>();
    let expected_package_ids = builtin_schema_package_sources()
        .iter()
        .map(|source| source.package_id.to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual_package_ids, expected_package_ids,
        "schema package folders must match embedded built-in package catalog"
    );

    let mut args = vec![
        "validate".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
        "--content-type".to_owned(),
        CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned(),
        "--schema".to_owned(),
        CEM_SCHEMA_PACKAGE_URI.to_owned(),
    ];
    args.extend(
        manifest_paths
            .iter()
            .map(|path| path.to_str().expect("manifest path is utf-8").to_owned()),
    );

    let output = cem_ml_owned(&args);
    assert_eq!(
        output.status.code(),
        Some(EXIT_OK),
        "package manifest sweep failed\nstdout:\n{}\nstderr:\n{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(
        stderr(&output).trim().is_empty(),
        "package manifest sweep stderr must stay empty:\n{}",
        stderr(&output)
    );

    let report: serde_json::Value = serde_json::from_str(stdout(&output).trim())
        .expect("package manifest sweep stdout is validation JSON");
    assert_eq!(
        report["summary"]["inputCount"].as_u64(),
        Some(manifest_paths.len() as u64),
        "package manifest sweep input count"
    );
    assert_eq!(
        report["summary"]["hardViolationCount"].as_u64(),
        Some(0),
        "package manifest sweep hard violations"
    );
    assert!(
        diagnostics(&report).is_empty(),
        "package manifest sweep diagnostics must stay empty:\n{}",
        stdout(&output)
    );
}
