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
}
