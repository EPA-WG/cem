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
