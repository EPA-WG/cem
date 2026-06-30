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

#[derive(Debug)]
struct ValidationExample {
    name: &'static str,
    path: &'static str,
    content_type: &'static str,
    schema_uri: &'static str,
    expected_exit: i32,
    expected_diagnostics: &'static [&'static str],
}

fn cem_ml(args: &[&str]) -> Output {
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
            expected_diagnostics: &["cem.schema_model.missing_required_attribute"],
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
            name: "transform-template invalid missing required attribute",
            path: "packages/cem_ml/schema-packages/cem-transform/v1/examples/invalid-missing-required-attribute.cemt",
            content_type: CEM_TRANSFORM_CONTENT_TYPE,
            schema_uri: CEM_TRANSFORM_SCHEMA_URI,
            expected_exit: EXIT_HARD_FAILURE,
            expected_diagnostics: &["cem.schema_model.missing_required_attribute"],
        },
    ];

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
