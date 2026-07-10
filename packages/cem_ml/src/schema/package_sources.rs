//! Embedded built-in schema package sources.
//!
//! The runtime registry and package loader both consume this catalog. Keeping
//! embedded sources here avoids maintaining one package list for registry
//! identity and another for loading schema/package documents.

#[derive(Debug, Clone, Copy)]
pub struct BuiltinSchemaPackageSource {
    pub package_id: &'static str,
    pub schema_path: &'static str,
    pub manifest_source: &'static str,
    pub schema_source: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct BuiltinSchemaPackageArtifactSource {
    pub package_id: &'static str,
    pub path: &'static str,
    pub source: &'static str,
}

pub fn builtin_schema_package_sources() -> &'static [BuiltinSchemaPackageSource] {
    BUILTIN_SCHEMA_PACKAGE_SOURCES
}

pub fn builtin_schema_package_artifact_sources() -> &'static [BuiltinSchemaPackageArtifactSource] {
    BUILTIN_SCHEMA_PACKAGE_ARTIFACT_SOURCES
}

pub fn builtin_schema_package_source(
    package_id: &str,
) -> Option<&'static BuiltinSchemaPackageSource> {
    BUILTIN_SCHEMA_PACKAGE_SOURCES
        .iter()
        .find(|source| source.package_id == package_id)
}

pub fn builtin_schema_package_artifact_source(
    package_id: &str,
    path: &str,
) -> Option<&'static BuiltinSchemaPackageArtifactSource> {
    BUILTIN_SCHEMA_PACKAGE_ARTIFACT_SOURCES
        .iter()
        .find(|source| source.package_id == package_id && source.path == path)
}

static BUILTIN_SCHEMA_PACKAGE_ARTIFACT_SOURCES: &[BuiltinSchemaPackageArtifactSource] = &[
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-dom-projection",
        path: "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
        source: include_str!(
            "../../schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-dom-projection",
        path: "schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt",
        source: include_str!(
            "../../schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ml",
        path: "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt",
        source: include_str!("../../schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ml",
        path: "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt",
        source: include_str!(
            "../../schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ml",
        path: "schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt",
        source: include_str!(
            "../../schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ml",
        path: "schema-packages/cem-ml/v1/formatters/cem-tree-helpers.cemt",
        source: include_str!("../../schema-packages/cem-ml/v1/formatters/cem-tree-helpers.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ml",
        path: "schema-packages/cem-ml/v1/colorizers/cem-color-tree.cemt",
        source: include_str!("../../schema-packages/cem-ml/v1/colorizers/cem-color-tree.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ml",
        path: "schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt",
        source: include_str!(
            "../../schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ml",
        path: "schema-packages/cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt",
        source: include_str!(
            "../../schema-packages/cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ml",
        path: "schema-packages/cem-ml/v1/colorizers/cem-tree-helpers.cemt",
        source: include_str!("../../schema-packages/cem-ml/v1/colorizers/cem-tree-helpers.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-native-template",
        path: "schema-packages/cem-native-template/v1/formatters/template-format-tree.cemt",
        source: include_str!(
            "../../schema-packages/cem-native-template/v1/formatters/template-format-tree.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-native-template",
        path: "schema-packages/cem-native-template/v1/colorizers/template-color-tree.cemt",
        source: include_str!(
            "../../schema-packages/cem-native-template/v1/colorizers/template-color-tree.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-transform",
        path: "schema-packages/cem-transform/v1/formatters/transform-format-tree.cemt",
        source: include_str!(
            "../../schema-packages/cem-transform/v1/formatters/transform-format-tree.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-transform",
        path: "schema-packages/cem-transform/v1/colorizers/transform-color-tree.cemt",
        source: include_str!(
            "../../schema-packages/cem-transform/v1/colorizers/transform-color-tree.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ql",
        path: "schema-packages/cem-ql/v1/formatters/cem-ql-format-tree.cemt",
        source: include_str!("../../schema-packages/cem-ql/v1/formatters/cem-ql-format-tree.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "cem-ql",
        path: "schema-packages/cem-ql/v1/colorizers/cem-ql-color-tree.cemt",
        source: include_str!("../../schema-packages/cem-ql/v1/colorizers/cem-ql-color-tree.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json",
        path: "schema-packages/json/v1/formatters/json-format-document.cemt",
        source: include_str!("../../schema-packages/json/v1/formatters/json-format-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json",
        path: "schema-packages/json/v1/colorizers/json-color-document.cemt",
        source: include_str!("../../schema-packages/json/v1/colorizers/json-color-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json-schema",
        path: "schema-packages/json-schema/v1/formatters/json-schema-format-document.cemt",
        source: include_str!(
            "../../schema-packages/json-schema/v1/formatters/json-schema-format-document.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json-schema",
        path: "schema-packages/json-schema/v1/colorizers/json-schema-color-document.cemt",
        source: include_str!(
            "../../schema-packages/json-schema/v1/colorizers/json-schema-color-document.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "yaml",
        path: "schema-packages/yaml/v1/formatters/yaml-format-document.cemt",
        source: include_str!("../../schema-packages/yaml/v1/formatters/yaml-format-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "yaml",
        path: "schema-packages/yaml/v1/colorizers/yaml-color-document.cemt",
        source: include_str!("../../schema-packages/yaml/v1/colorizers/yaml-color-document.cemt"),
    },
];

static BUILTIN_SCHEMA_PACKAGE_SOURCES: &[BuiltinSchemaPackageSource] = &[
    BuiltinSchemaPackageSource {
        package_id: "cem-ml",
        schema_path: "schema-packages/cem-ml/v1/schema/cem-ml-generic.cem",
        manifest_source: include_str!("../../schema-packages/cem-ml/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/cem-ml/v1/schema/cem-ml-generic.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "schema",
        schema_path: "schema-packages/schema/v1/schema/cem-schema.cem",
        manifest_source: include_str!("../../schema-packages/schema/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/schema/v1/schema/cem-schema.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "schema-package",
        schema_path: "schema-packages/schema-package/v1/schema/schema-package.cem",
        manifest_source: include_str!("../../schema-packages/schema-package/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/schema-package/v1/schema/schema-package.cem"
        ),
    },
    BuiltinSchemaPackageSource {
        package_id: "cem-native-template",
        schema_path: "schema-packages/cem-native-template/v1/schema/cem-native-template.cem",
        manifest_source: include_str!("../../schema-packages/cem-native-template/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/cem-native-template/v1/schema/cem-native-template.cem"
        ),
    },
    BuiltinSchemaPackageSource {
        package_id: "cem-transform",
        schema_path: "schema-packages/cem-transform/v1/schema/cem-transform.cem",
        manifest_source: include_str!("../../schema-packages/cem-transform/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/cem-transform/v1/schema/cem-transform.cem"
        ),
    },
    BuiltinSchemaPackageSource {
        package_id: "cem-ql",
        schema_path: "schema-packages/cem-ql/v1/schema/cem-ql.cem",
        manifest_source: include_str!("../../schema-packages/cem-ql/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/cem-ql/v1/schema/cem-ql.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "json",
        schema_path: "schema-packages/json/v1/schema/json.cem",
        manifest_source: include_str!("../../schema-packages/json/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/json/v1/schema/json.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "yaml",
        schema_path: "schema-packages/yaml/v1/schema/yaml.cem",
        manifest_source: include_str!("../../schema-packages/yaml/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/yaml/v1/schema/yaml.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "csv",
        schema_path: "schema-packages/csv/v1/schema/csv.cem",
        manifest_source: include_str!("../../schema-packages/csv/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/csv/v1/schema/csv.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "markdown",
        schema_path: "schema-packages/markdown/v1/schema/markdown.cem",
        manifest_source: include_str!("../../schema-packages/markdown/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/markdown/v1/schema/markdown.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "xml",
        schema_path: "schema-packages/xml/v1/schema/xml.cem",
        manifest_source: include_str!("../../schema-packages/xml/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/xml/v1/schema/xml.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "relax-ng",
        schema_path: "schema-packages/relax-ng/v1/schema/relax-ng.cem",
        manifest_source: include_str!("../../schema-packages/relax-ng/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/relax-ng/v1/schema/relax-ng.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "xhtml",
        schema_path: "schema-packages/xhtml/v1/schema/xhtml.cem",
        manifest_source: include_str!("../../schema-packages/xhtml/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/xhtml/v1/schema/xhtml.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "svg",
        schema_path: "schema-packages/svg/v1/schema/svg.cem",
        manifest_source: include_str!("../../schema-packages/svg/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/svg/v1/schema/svg.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "mathml",
        schema_path: "schema-packages/mathml/v1/schema/mathml.cem",
        manifest_source: include_str!("../../schema-packages/mathml/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/mathml/v1/schema/mathml.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "xslt",
        schema_path: "schema-packages/xslt/v1/schema/xslt.cem",
        manifest_source: include_str!("../../schema-packages/xslt/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/xslt/v1/schema/xslt.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "html",
        schema_path: "schema-packages/html/v1/schema/html.cem",
        manifest_source: include_str!("../../schema-packages/html/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/html/v1/schema/html.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "css",
        schema_path: "schema-packages/css/v1/schema/css.cem",
        manifest_source: include_str!("../../schema-packages/css/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/css/v1/schema/css.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "json-schema",
        schema_path: "schema-packages/json-schema/v1/schema/json-schema.cem",
        manifest_source: include_str!("../../schema-packages/json-schema/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/json-schema/v1/schema/json-schema.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "cem-dom-projection",
        schema_path: "schema-packages/cem-dom-projection/v1/schema/cem-dom-projection.cem",
        manifest_source: include_str!("../../schema-packages/cem-dom-projection/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/cem-dom-projection/v1/schema/cem-dom-projection.cem"
        ),
    },
    BuiltinSchemaPackageSource {
        package_id: "cem-ast-projection",
        schema_path: "schema-packages/cem-ast-projection/v1/schema/cem-ast-projection.cem",
        manifest_source: include_str!("../../schema-packages/cem-ast-projection/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/cem-ast-projection/v1/schema/cem-ast-projection.cem"
        ),
    },
    BuiltinSchemaPackageSource {
        package_id: "cem-events-projection",
        schema_path: "schema-packages/cem-events-projection/v1/schema/cem-events-projection.cem",
        manifest_source: include_str!("../../schema-packages/cem-events-projection/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/cem-events-projection/v1/schema/cem-events-projection.cem"
        ),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cem::CemEventNormalizer;
    use crate::parser::builder::CemAstBuilder;
    use crate::parser::document::CemDocument;
    use crate::parser::{AstNodeId, CemAstNode};
    use crate::schema::registry::{
        schema_package_examples_from_package_sources, SchemaPackageExampleExpectedResult,
        CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
        CEM_NATIVE_TEMPLATE_SCHEMA_URI, CEM_QL_CONTENT_TYPE, CEM_QL_SCHEMA_URI,
        CEM_SCHEMA_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_URI,
        CEM_SCHEMA_URI, CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, JSON_CONTENT_TYPE,
        JSON_SCHEMA_CONTENT_TYPE, JSON_SCHEMA_SCHEMA_URI, JSON_VALUE_SCHEMA_URI, YAML_CONTENT_TYPE,
        YAML_SCHEMA_URI,
    };
    use crate::source::{BytesSource, SourceId};
    use crate::tokenizer::cem::CemTokenizer;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    const BASELINE_FORMATTER_PROFILES: &[&str] = &["compact", "pretty", "tabular"];
    const BASELINE_COLORIZER_PROFILES: &[&str] = &["terminal", "html", "md"];

    fn package_root(package_id: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("schema-packages")
            .join(package_id)
            .join("v1")
    }

    fn package_manifest_path(package_id: &str) -> PathBuf {
        package_root(package_id).join("package.cem")
    }

    fn package_relative_path(package_id: &str, path: &str) -> String {
        let path = path.trim();
        if path.starts_with("schema-packages/") {
            path.to_owned()
        } else {
            format!(
                "schema-packages/{package_id}/v1/{}",
                path.trim_start_matches("./")
            )
        }
    }

    fn package_root_relative_path(package_id: &str, path: &str) -> PathBuf {
        let path = path.trim();
        if let Some(relative) = path.strip_prefix(&format!("schema-packages/{package_id}/v1/")) {
            package_root(package_id).join(relative)
        } else {
            package_root(package_id).join(path.trim_start_matches("./"))
        }
    }

    fn parse_cem_document(input: &str) -> CemDocument {
        let src = BytesSource::new(SourceId(1), input.as_bytes().to_vec());
        let tok = CemTokenizer::from_source(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemAstBuilder::new(normalizer).build()
    }

    fn collect_attrs(document: &CemDocument, node_id: AstNodeId) -> BTreeMap<String, String> {
        let mut attrs = BTreeMap::new();
        let Some(CemAstNode::Element { attributes, .. }) = document.get(node_id) else {
            return attrs;
        };
        for attr_id in attributes {
            let Some(CemAstNode::Attribute {
                expanded_name,
                value,
                ..
            }) = document.get(*attr_id)
            else {
                continue;
            };
            attrs.insert(
                expanded_name.local_name.clone(),
                value.clone().unwrap_or_default(),
            );
        }
        attrs
    }

    fn element_ids_by_local_name(document: &CemDocument, local_name: &str) -> Vec<AstNodeId> {
        document
            .iter()
            .filter_map(|node| {
                let CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } = node
                else {
                    return None;
                };
                (expanded_name.local_name == local_name).then_some(*node_id)
            })
            .collect()
    }

    fn first_element_attrs(
        document: &CemDocument,
        local_name: &str,
    ) -> Option<BTreeMap<String, String>> {
        element_ids_by_local_name(document, local_name)
            .into_iter()
            .next()
            .map(|node_id| collect_attrs(document, node_id))
    }

    fn package_manifest_artifact_paths(
        package_id: &str,
        manifest_source: &str,
    ) -> BTreeSet<String> {
        let document = parse_cem_document(manifest_source);
        element_ids_by_local_name(&document, "artifact")
            .into_iter()
            .filter_map(|node_id| {
                let attrs = collect_attrs(&document, node_id);
                attrs
                    .get("path")
                    .map(|path| package_relative_path(package_id, path))
            })
            .collect()
    }

    fn package_manifest_artifact_attrs(
        package_id: &str,
        manifest_source: &str,
    ) -> Vec<BTreeMap<String, String>> {
        let document = parse_cem_document(manifest_source);
        element_ids_by_local_name(&document, "artifact")
            .into_iter()
            .map(|node_id| {
                let mut attrs = collect_attrs(&document, node_id);
                if let Some(path) = attrs.get("path").cloned() {
                    attrs.insert("path".to_owned(), package_relative_path(package_id, &path));
                }
                attrs
            })
            .collect()
    }

    fn directory_cemt_paths(package_id: &str, directory: &str) -> BTreeSet<String> {
        let root = package_root(package_id).join(directory);
        let Ok(entries) = std::fs::read_dir(root) else {
            return BTreeSet::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("cemt"))
            .filter_map(|path| {
                let file_name = path.file_name()?.to_str()?;
                Some(format!(
                    "schema-packages/{package_id}/v1/{directory}/{file_name}"
                ))
            })
            .collect()
    }

    fn top_level_example_paths(package_id: &str) -> BTreeSet<String> {
        let root = package_root(package_id).join("examples");
        let Ok(entries) = std::fs::read_dir(root) else {
            return BTreeSet::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter_map(|path| {
                let file_name = path.file_name()?.to_str()?;
                Some(format!(
                    "schema-packages/{package_id}/v1/examples/{file_name}"
                ))
            })
            .collect()
    }

    fn artifact_profiles(
        artifacts: &[BTreeMap<String, String>],
        kind: &str,
        profile_attr: &str,
    ) -> BTreeSet<String> {
        artifacts
            .iter()
            .filter(|attrs| attrs.get("kind").map(String::as_str) == Some(kind))
            .filter_map(|attrs| attrs.get(profile_attr).cloned())
            .collect()
    }

    fn assert_baseline_profiles(
        package_id: &str,
        kind: &str,
        profile_attr: &str,
        actual_profiles: &BTreeSet<String>,
        expected_profiles: &[&str],
    ) {
        if actual_profiles.is_empty() {
            return;
        }
        for profile in expected_profiles {
            assert!(
                actual_profiles.contains(*profile),
                "{} must declare baseline `{}` {} on `{}` artifacts; actual profiles: {:?}",
                package_id,
                profile,
                profile_attr,
                kind,
                actual_profiles
            );
        }
    }

    fn output_artifact_directory(kind: &str) -> Option<&'static str> {
        match kind {
            "formatter" | "formatter-helper" => Some("formatters"),
            "colorizer" | "colorizer-helper" => Some("colorizers"),
            _ => None,
        }
    }

    fn manifest_indexed_package_examples(
        package_id: &str,
        content_type: &str,
        schema_uri: &str,
        expected_count: usize,
    ) -> Vec<crate::schema::registry::SchemaPackageExampleDescriptor> {
        let package = builtin_schema_package_source(package_id).expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths(package_id),
            "{package_id} top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), expected_count);
        assert!(examples
            .iter()
            .all(|example| example.content_type == content_type));
        assert!(examples.iter().all(|example| example.schema == schema_uri));
        examples
    }

    #[test]
    fn cem_ml_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("cem-ml", CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, 4);
        let invalid = examples
            .iter()
            .find(|example| example.id == "invalid-unclosed-scope")
            .expect("invalid CEM-ML example");
        assert_eq!(
            invalid.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            invalid.expected_diagnostic_codes,
            vec!["cem.ast.unclosed_scope".to_owned()]
        );
    }

    #[test]
    fn schema_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("schema", CEM_SCHEMA_CONTENT_TYPE, CEM_SCHEMA_URI, 4);
        let missing_required = examples
            .iter()
            .find(|example| example.id == "invalid-missing-required-attribute")
            .expect("invalid missing required schema example");
        assert_eq!(
            missing_required.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            missing_required.expected_diagnostic_codes,
            vec!["cem.schema_model.missing_required_attribute".to_owned()]
        );

        let unclosed = examples
            .iter()
            .find(|example| example.id == "invalid-unclosed-schema")
            .expect("invalid unclosed schema example");
        assert_eq!(
            unclosed.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            unclosed.expected_diagnostic_codes,
            vec!["cem.ast.unclosed_scope".to_owned()]
        );
    }

    #[test]
    fn schema_package_manifest_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "schema-package",
            CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            CEM_SCHEMA_PACKAGE_URI,
            15,
        );
        for (id, expected_code) in [
            (
                "invalid-converter-contract",
                "cem.schema_package.converter_check",
            ),
            (
                "invalid-artifact-contract",
                "cem.schema_package.artifact_check",
            ),
            (
                "invalid-example-contract",
                "cem.schema_package.example_check",
            ),
            (
                "invalid-schema-metadata",
                "cem.schema_package.schema_uri_mismatch",
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("schema-package example `{id}`"));
            assert_eq!(
                example.expected_result,
                SchemaPackageExampleExpectedResult::Fail
            );
            assert_eq!(
                example.expected_diagnostic_codes,
                vec![expected_code.to_owned()]
            );
        }
    }

    #[test]
    fn cem_native_template_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "cem-native-template",
            CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
            CEM_NATIVE_TEMPLATE_SCHEMA_URI,
            3,
        );
        let missing_required = examples
            .iter()
            .find(|example| example.id == "invalid-missing-required-attribute")
            .expect("invalid missing required native template example");
        assert_eq!(
            missing_required.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            missing_required.expected_diagnostic_codes,
            vec!["cem.schema_model.missing_required_attribute".to_owned()]
        );
    }

    #[test]
    fn cem_transform_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("cem-transform").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("cem-transform"),
            "cem-transform top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), 8);

        let fixture = examples
            .iter()
            .find(|example| example.id == "formatter-coloring-pipeline-fixture")
            .expect("formatter/coloring pipeline fixture example");
        assert_eq!(fixture.content_type, CEM_ML_CONTENT_TYPE);
        assert_eq!(fixture.schema, CEM_ML_SCHEMA_URI);
        assert_eq!(
            fixture.expected_result,
            SchemaPackageExampleExpectedResult::Pass
        );

        for example in examples
            .iter()
            .filter(|example| example.id != "formatter-coloring-pipeline-fixture")
        {
            assert_eq!(example.content_type, CEM_TRANSFORM_CONTENT_TYPE);
            assert_eq!(example.schema, CEM_TRANSFORM_SCHEMA_URI);
        }

        for id in [
            "invalid-missing-required-attribute",
            "invalid-function-missing-category",
            "invalid-function-missing-contract-metadata",
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("invalid transform example `{id}`"));
            assert_eq!(
                example.expected_result,
                SchemaPackageExampleExpectedResult::Fail
            );
            assert_eq!(
                example.expected_diagnostic_codes,
                vec!["cem.schema_model.missing_required_attribute".to_owned()]
            );
        }
    }

    #[test]
    fn cem_ql_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("cem-ql", CEM_QL_CONTENT_TYPE, CEM_QL_SCHEMA_URI, 4);

        for (id, expected_code) in [
            ("invalid-parse", "cem.ql.parse_error"),
            ("invalid-missing-module", "cem.ql.module_uri_missing"),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("invalid CEM-QL example `{id}`"));
            assert_eq!(
                example.expected_result,
                SchemaPackageExampleExpectedResult::Fail
            );
            assert_eq!(
                example.expected_diagnostic_codes,
                vec![expected_code.to_owned()]
            );
        }
    }

    #[test]
    fn json_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("json", JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI, 3);
        let invalid = examples
            .iter()
            .find(|example| example.id == "invalid-trailing-comma")
            .expect("invalid JSON example");
        assert_eq!(
            invalid.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            invalid.expected_diagnostic_codes,
            vec!["cem.json.parse_error".to_owned()]
        );
    }

    #[test]
    fn json_schema_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "json-schema",
            JSON_SCHEMA_CONTENT_TYPE,
            JSON_SCHEMA_SCHEMA_URI,
            4,
        );

        for (id, expected_code) in [
            (
                "invalid-unsupported-dialect",
                "cem.json_schema.unsupported_dialect",
            ),
            ("invalid-parse", "cem.json_schema.parse_error"),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("invalid JSON Schema example `{id}`"));
            assert_eq!(
                example.expected_result,
                SchemaPackageExampleExpectedResult::Fail
            );
            assert_eq!(
                example.expected_diagnostic_codes,
                vec![expected_code.to_owned()]
            );
        }
    }

    #[test]
    fn yaml_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("yaml").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("yaml"),
            "yaml top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), 4);
        assert!(examples
            .iter()
            .all(|example| example.schema == YAML_SCHEMA_URI));

        for (id, content_type, expected_result, expected_code) in [
            (
                "basic-document",
                YAML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "nested-stream",
                "text/yaml",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-parse",
                YAML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.yaml.parse_error"),
            ),
            (
                "invalid-unsafe-tag",
                "application/x-yaml",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.yaml.unsafe_tag"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("YAML example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn builtin_schema_package_catalog_matches_core_folder_frame() {
        for source in builtin_schema_package_sources() {
            let root = package_root(source.package_id);
            let manifest_path = package_manifest_path(source.package_id);
            assert!(
                root.is_dir(),
                "{} package root must exist at {}",
                source.package_id,
                root.display()
            );
            assert!(
                manifest_path.is_file(),
                "{} package.cem must exist at {}",
                source.package_id,
                manifest_path.display()
            );
            assert!(
                root.join("schema").is_dir(),
                "{} package must include schema/",
                source.package_id
            );
            assert!(
                root.join("examples").is_dir(),
                "{} package must include examples/",
                source.package_id
            );
            assert!(
                std::fs::read_to_string(&manifest_path).expect("read package.cem")
                    == source.manifest_source,
                "{} embedded package.cem must match the folder source",
                source.package_id
            );

            let document = parse_cem_document(source.manifest_source);
            let package_attrs = first_element_attrs(&document, "package").expect("package element");
            assert_eq!(
                package_attrs.get("id").map(String::as_str),
                Some(source.package_id),
                "{} package.cem id must match the embedded catalog id",
                source.package_id
            );

            let schema_attrs = first_element_attrs(&document, "schema").expect("schema element");
            let schema_source = schema_attrs
                .get("source")
                .expect("schema source attribute in package.cem");
            let schema_path = package_relative_path(source.package_id, schema_source);
            assert_eq!(
                source.schema_path, schema_path,
                "{} embedded schema path must match package.cem schema source",
                source.package_id
            );
            assert!(
                source.schema_path.ends_with(".cem"),
                "{} schema source must be authored in .cem format",
                source.package_id
            );
            let schema_file = package_root_relative_path(source.package_id, source.schema_path);
            assert!(
                schema_file.is_file(),
                "{} schema source must exist at {}",
                source.package_id,
                schema_file.display()
            );
            assert!(
                std::fs::read_to_string(&schema_file).expect("read schema source")
                    == source.schema_source,
                "{} embedded schema source must match the folder source",
                source.package_id
            );

            let examples = std::fs::read_dir(root.join("examples"))
                .expect("read examples directory")
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_file())
                .count();
            assert!(
                examples > 0,
                "{} package examples/ must contain at least one fixture",
                source.package_id
            );
        }
    }

    #[test]
    fn declared_builtin_cemt_artifacts_match_embedded_folder_sources() {
        let embedded_artifact_paths = builtin_schema_package_artifact_sources()
            .iter()
            .map(|source| (source.package_id, source.path))
            .collect::<BTreeSet<_>>();

        for package in builtin_schema_package_sources() {
            let declared =
                package_manifest_artifact_paths(package.package_id, package.manifest_source);
            for path in &declared {
                let source_path = package_root_relative_path(package.package_id, path);
                assert!(
                    source_path.is_file(),
                    "{} declared artifact `{}` must exist at {}",
                    package.package_id,
                    path,
                    source_path.display()
                );
                assert!(
                    path.ends_with(".cemt"),
                    "{} declared CEMT artifact `{}` must use .cemt",
                    package.package_id,
                    path
                );
                assert!(
                    embedded_artifact_paths.contains(&(package.package_id, path.as_str())),
                    "{} declared artifact `{}` must be embedded in package_sources.rs",
                    package.package_id,
                    path
                );
            }
        }
    }

    #[test]
    fn builtin_output_asset_folders_are_manifest_indexed() {
        let embedded_artifact_paths = builtin_schema_package_artifact_sources()
            .iter()
            .map(|source| (source.package_id, source.path))
            .collect::<BTreeSet<_>>();

        for package in builtin_schema_package_sources() {
            let package_id = package.package_id;
            let declared_attrs =
                package_manifest_artifact_attrs(package_id, package.manifest_source);
            let declared_paths = declared_attrs
                .iter()
                .filter_map(|attrs| attrs.get("path").cloned())
                .collect::<BTreeSet<_>>();

            for directory in ["formatters", "colorizers"] {
                let directory_path = package_root(package_id).join(directory);
                let files = directory_cemt_paths(package_id, directory);
                if directory_path.exists() {
                    assert!(
                        directory_path.is_dir(),
                        "{} output asset path `{}` must be a directory",
                        package_id,
                        directory_path.display()
                    );
                    assert!(
                        !files.is_empty(),
                        "{} output asset folder `{directory}/` must contain at least one .cemt file",
                        package_id
                    );
                }

                for path in &files {
                    assert!(
                        declared_paths.contains(path),
                        "{} CEMT asset `{path}` must be discoverable from package.cem",
                        package_id
                    );
                    assert!(
                        embedded_artifact_paths.contains(&(package_id, path.as_str())),
                        "{} CEMT asset `{path}` must be embedded in the artifact source catalog",
                        package_id
                    );
                }
            }

            for attrs in &declared_attrs {
                let Some(kind) = attrs
                    .get("kind")
                    .and_then(|kind| output_artifact_directory(kind))
                else {
                    continue;
                };
                let path = attrs.get("path").expect("artifact path normalized");
                let expected_prefix = format!("schema-packages/{package_id}/v1/{kind}/");
                assert!(
                    path.starts_with(&expected_prefix),
                    "{} `{}` artifact `{}` must live under `{kind}/`",
                    package_id,
                    attrs.get("kind").expect("artifact kind"),
                    path
                );
            }
        }
    }

    #[test]
    fn builtin_output_stage_profile_sets_are_complete_when_declared() {
        for package in builtin_schema_package_sources() {
            let artifacts =
                package_manifest_artifact_attrs(package.package_id, package.manifest_source);
            let formatter_profiles =
                artifact_profiles(&artifacts, "formatter", "formatter-profile");
            let formatter_helper_profiles =
                artifact_profiles(&artifacts, "formatter-helper", "formatter-profile");
            let color_profiles = artifact_profiles(&artifacts, "colorizer", "color-profile");

            assert_baseline_profiles(
                package.package_id,
                "formatter",
                "formatter-profile",
                &formatter_profiles,
                BASELINE_FORMATTER_PROFILES,
            );
            assert_baseline_profiles(
                package.package_id,
                "formatter-helper",
                "formatter-profile",
                &formatter_helper_profiles,
                BASELINE_FORMATTER_PROFILES,
            );
            assert_baseline_profiles(
                package.package_id,
                "colorizer",
                "color-profile",
                &color_profiles,
                BASELINE_COLORIZER_PROFILES,
            );
        }
    }

    #[test]
    fn catalog_exposes_cem_ml_output_artifact_sources() {
        let formatter = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt",
        )
        .expect("CEM-ML formatter source");
        let colorizer = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/colorizers/cem-color-tree.cemt",
        )
        .expect("CEM-ML colorizer source");
        let canonical_formatter_helpers = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt",
        )
        .expect("CEM-ML canonical formatter helper source");
        let canonical_colorizer_helpers = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt",
        )
        .expect("CEM-ML canonical colorizer helper source");
        let showcase_formatter = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt",
        )
        .expect("CEM-ML showcase formatter source");
        let showcase_colorizer = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt",
        )
        .expect("CEM-ML showcase colorizer source");
        let formatter_helpers = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/formatters/cem-tree-helpers.cemt",
        )
        .expect("CEM-ML formatter helper source");
        let colorizer_helpers = builtin_schema_package_artifact_source(
            "cem-ml",
            "schema-packages/cem-ml/v1/colorizers/cem-tree-helpers.cemt",
        )
        .expect("CEM-ML colorizer helper source");
        let dom_html_converter = builtin_schema_package_artifact_source(
            "cem-dom-projection",
            "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
        )
        .expect("DOM projection HTML CEMT converter source");
        let dom_xml_converter = builtin_schema_package_artifact_source(
            "cem-dom-projection",
            "schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt",
        )
        .expect("DOM projection XML CEMT converter source");

        assert!(formatter.source.contains(r#"@name="cem.format-tree""#));
        assert!(colorizer.source.contains(r#"@name="cem.color-tree""#));
        assert!(dom_html_converter
            .source
            .contains(r#"{template @name="emit-node""#));
        assert!(dom_html_converter
            .source
            .contains(r#"node.kind = "raw-text""#));
        assert!(dom_xml_converter
            .source
            .contains(r#"{template @name="emit-node""#));
        assert!(dom_xml_converter.source.contains(r#"node.kind = "cdata""#));
        assert!(canonical_formatter_helpers
            .source
            .contains(r#"@name="cem.format-tree.apply-stage""#));
        assert!(canonical_formatter_helpers
            .source
            .contains(r#"@name="cem.format-tree.build-nodes""#));
        assert!(canonical_formatter_helpers
            .source
            .contains(r#"@returns="array""#));
        assert!(canonical_formatter_helpers
            .source
            .contains(r#"@name="cem.format-tree.build-node-list""#));
        assert!(canonical_formatter_helpers
            .source
            .contains(r#"@name="cem.format-tree.build-envelope""#));
        assert!(canonical_formatter_helpers
            .source
            .contains(r#"@returns="object""#));
        assert!(canonical_formatter_helpers
            .source
            .contains(r#"@name="cem.format-tree.format-node""#));
        assert!(canonical_formatter_helpers
            .source
            .contains(r#"@name="cem.format-tree.node-child-layout""#));
        assert!(canonical_colorizer_helpers
            .source
            .contains(r#"@name="cem.color-tree.apply-stage""#));
        assert!(canonical_colorizer_helpers
            .source
            .contains(r#"@name="cem.color-tree.color-node""#));
        assert!(canonical_colorizer_helpers
            .source
            .contains(r#"@name="cem.color-tree.writer-attribute-nodes""#));
        assert!(showcase_formatter
            .source
            .contains(r#"@name="acme.showcase.format-tree""#));
        assert!(formatter_helpers
            .source
            .contains(r#"@name="cemml.cem-tree.format-tree-base""#));
        assert!(showcase_colorizer
            .source
            .contains(r#"@name="acme.showcase.color-tree""#));
        assert!(colorizer_helpers
            .source
            .contains(r#"@name="cemml.cem-tree.color-tree-base""#));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == formatter.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == colorizer.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == canonical_formatter_helpers.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == canonical_colorizer_helpers.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == showcase_formatter.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == showcase_colorizer.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == formatter_helpers.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == colorizer_helpers.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == dom_html_converter.path));
        assert!(builtin_schema_package_artifact_sources()
            .iter()
            .any(|source| source.path == dom_xml_converter.path));
    }

    #[test]
    fn catalog_exposes_cem_native_template_output_artifact_sources() {
        let formatter = builtin_schema_package_artifact_source(
            "cem-native-template",
            "schema-packages/cem-native-template/v1/formatters/template-format-tree.cemt",
        )
        .expect("CEM-native template formatter source");
        let colorizer = builtin_schema_package_artifact_source(
            "cem-native-template",
            "schema-packages/cem-native-template/v1/colorizers/template-color-tree.cemt",
        )
        .expect("CEM-native template colorizer source");

        assert!(formatter.source.contains(r#"@name="template.format-tree""#));
        assert!(formatter
            .source
            .contains(r#"@content-type="application/vnd.cem.template+cem""#));
        assert!(colorizer.source.contains(r#"@name="template.color-tree""#));
        assert!(colorizer
            .source
            .contains(r#"@schema="https://cem.dev/ns/template/cem-native/1""#));
    }

    #[test]
    fn catalog_exposes_cem_transform_output_artifact_sources() {
        let formatter = builtin_schema_package_artifact_source(
            "cem-transform",
            "schema-packages/cem-transform/v1/formatters/transform-format-tree.cemt",
        )
        .expect("CEM transform formatter source");
        let colorizer = builtin_schema_package_artifact_source(
            "cem-transform",
            "schema-packages/cem-transform/v1/colorizers/transform-color-tree.cemt",
        )
        .expect("CEM transform colorizer source");

        assert!(formatter
            .source
            .contains(r#"@name="transform.format-tree""#));
        assert!(formatter
            .source
            .contains(r#"@content-type="application/vnd.cem.transform+cem""#));
        assert!(colorizer.source.contains(r#"@name="transform.color-tree""#));
        assert!(colorizer
            .source
            .contains(r#"@schema="https://cem.dev/ns/transform/cem/1""#));
    }

    #[test]
    fn catalog_exposes_cem_ql_output_artifact_sources() {
        let formatter = builtin_schema_package_artifact_source(
            "cem-ql",
            "schema-packages/cem-ql/v1/formatters/cem-ql-format-tree.cemt",
        )
        .expect("CEM-QL formatter source");
        let colorizer = builtin_schema_package_artifact_source(
            "cem-ql",
            "schema-packages/cem-ql/v1/colorizers/cem-ql-color-tree.cemt",
        )
        .expect("CEM-QL colorizer source");

        assert!(formatter.source.contains(r#"@name="cem-ql.format-tree""#));
        assert!(formatter
            .source
            .contains(r#"@content-type="application/vnd.cem.query+cem-ql""#));
        assert!(colorizer.source.contains(r#"@name="cem-ql.color-tree""#));
        assert!(colorizer
            .source
            .contains(r#"@schema="https://cem.dev/ns/query/cem-ql/1""#));
    }

    #[test]
    fn catalog_exposes_json_output_artifact_sources() {
        let formatter = builtin_schema_package_artifact_source(
            "json",
            "schema-packages/json/v1/formatters/json-format-document.cemt",
        )
        .expect("JSON formatter source");
        let colorizer = builtin_schema_package_artifact_source(
            "json",
            "schema-packages/json/v1/colorizers/json-color-document.cemt",
        )
        .expect("JSON colorizer source");

        assert!(formatter.source.contains(r#"@name="json.format-document""#));
        assert!(formatter.source.contains(r#"@category="json-document""#));
        assert!(colorizer.source.contains(r#"@name="json.color-document""#));
        assert!(colorizer
            .source
            .contains(r#"@content-type="application/json""#));
    }

    #[test]
    fn catalog_exposes_json_schema_output_artifact_sources() {
        let formatter = builtin_schema_package_artifact_source(
            "json-schema",
            "schema-packages/json-schema/v1/formatters/json-schema-format-document.cemt",
        )
        .expect("JSON Schema formatter source");
        let colorizer = builtin_schema_package_artifact_source(
            "json-schema",
            "schema-packages/json-schema/v1/colorizers/json-schema-color-document.cemt",
        )
        .expect("JSON Schema colorizer source");

        assert!(formatter
            .source
            .contains(r#"@name="json-schema.format-document""#));
        assert!(formatter
            .source
            .contains(r#"@category="json-schema-document""#));
        assert!(colorizer
            .source
            .contains(r#"@name="json-schema.color-document""#));
        assert!(colorizer
            .source
            .contains(r#"@content-type="application/schema+json""#));
    }

    #[test]
    fn catalog_exposes_yaml_output_artifact_sources() {
        let formatter = builtin_schema_package_artifact_source(
            "yaml",
            "schema-packages/yaml/v1/formatters/yaml-format-document.cemt",
        )
        .expect("YAML formatter source");
        let colorizer = builtin_schema_package_artifact_source(
            "yaml",
            "schema-packages/yaml/v1/colorizers/yaml-color-document.cemt",
        )
        .expect("YAML colorizer source");

        assert!(formatter.source.contains(r#"@name="yaml.format-document""#));
        assert!(formatter.source.contains(r#"@category="yaml-document""#));
        assert!(colorizer.source.contains(r#"@name="yaml.color-document""#));
        assert!(colorizer
            .source
            .contains(r#"@content-type="application/yaml""#));
    }
}
