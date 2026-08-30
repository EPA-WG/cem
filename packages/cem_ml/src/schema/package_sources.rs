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
        package_id: "cli-command",
        path: "schema-packages/cli-command/v1/schema/cli-command.schema.json",
        source: include_str!(
            "../../schema-packages/cli-command/v1/schema/cli-command.schema.json"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "studio-project",
        path: "schema-packages/studio-project/v1/schema/studio-project.schema.json",
        source: include_str!(
            "../../schema-packages/studio-project/v1/schema/studio-project.schema.json"
        ),
    },
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
        path: "schema-packages/json/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/json/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json",
        path: "schema-packages/json/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/json/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json",
        path: "schema-packages/json/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/json/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json",
        path: "schema-packages/json/v1/formatters/json-format-document.cemt",
        source: include_str!("../../schema-packages/json/v1/formatters/json-format-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json",
        path: "schema-packages/json/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/json/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json",
        path: "schema-packages/json/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/json/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "json",
        path: "schema-packages/json/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/json/v1/colorizers/md.cemt"),
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
        path: "schema-packages/yaml/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/yaml/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "yaml",
        path: "schema-packages/yaml/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/yaml/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "yaml",
        path: "schema-packages/yaml/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/yaml/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "yaml",
        path: "schema-packages/yaml/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/yaml/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "yaml",
        path: "schema-packages/yaml/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/yaml/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "yaml",
        path: "schema-packages/yaml/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/yaml/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "csv",
        path: "schema-packages/csv/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/csv/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "csv",
        path: "schema-packages/csv/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/csv/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "csv",
        path: "schema-packages/csv/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/csv/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "csv",
        path: "schema-packages/csv/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/csv/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "csv",
        path: "schema-packages/csv/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/csv/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "csv",
        path: "schema-packages/csv/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/csv/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "markdown",
        path: "schema-packages/markdown/v1/formatters/markdown-format-document.cemt",
        source: include_str!(
            "../../schema-packages/markdown/v1/formatters/markdown-format-document.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "markdown",
        path: "schema-packages/markdown/v1/colorizers/markdown-color-document.cemt",
        source: include_str!(
            "../../schema-packages/markdown/v1/colorizers/markdown-color-document.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css",
        path: "schema-packages/css/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/css/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css",
        path: "schema-packages/css/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/css/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css",
        path: "schema-packages/css/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/css/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css",
        path: "schema-packages/css/v1/formatters/css-format-document.cemt",
        source: include_str!("../../schema-packages/css/v1/formatters/css-format-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css",
        path: "schema-packages/css/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/css/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css",
        path: "schema-packages/css/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/css/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css",
        path: "schema-packages/css/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/css/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css",
        path: "schema-packages/css/v1/colorizers/css-color-document.cemt",
        source: include_str!("../../schema-packages/css/v1/colorizers/css-color-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "scss",
        path: "schema-packages/scss/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/scss/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "scss",
        path: "schema-packages/scss/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/scss/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "scss",
        path: "schema-packages/scss/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/scss/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "scss",
        path: "schema-packages/scss/v1/formatters/scss-format-source.cemt",
        source: include_str!("../../schema-packages/scss/v1/formatters/scss-format-source.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "scss",
        path: "schema-packages/scss/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/scss/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "scss",
        path: "schema-packages/scss/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/scss/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "scss",
        path: "schema-packages/scss/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/scss/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "scss",
        path: "schema-packages/scss/v1/colorizers/scss-color-source.cemt",
        source: include_str!("../../schema-packages/scss/v1/colorizers/scss-color-source.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css-selector",
        path: "schema-packages/css-selector/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/css-selector/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css-selector",
        path: "schema-packages/css-selector/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/css-selector/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css-selector",
        path: "schema-packages/css-selector/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/css-selector/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css-selector",
        path: "schema-packages/css-selector/v1/formatters/css-selector-format-expression.cemt",
        source: include_str!(
            "../../schema-packages/css-selector/v1/formatters/css-selector-format-expression.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css-selector",
        path: "schema-packages/css-selector/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/css-selector/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css-selector",
        path: "schema-packages/css-selector/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/css-selector/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css-selector",
        path: "schema-packages/css-selector/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/css-selector/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "css-selector",
        path: "schema-packages/css-selector/v1/colorizers/css-selector-color-expression.cemt",
        source: include_str!(
            "../../schema-packages/css-selector/v1/colorizers/css-selector-color-expression.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "html",
        path: "schema-packages/html/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/html/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "html",
        path: "schema-packages/html/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/html/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "html",
        path: "schema-packages/html/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/html/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "html",
        path: "schema-packages/html/v1/formatters/html-format-document.cemt",
        source: include_str!("../../schema-packages/html/v1/formatters/html-format-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "html",
        path: "schema-packages/html/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/html/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "html",
        path: "schema-packages/html/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/html/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "html",
        path: "schema-packages/html/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/html/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "html",
        path: "schema-packages/html/v1/colorizers/html-color-document.cemt",
        source: include_str!("../../schema-packages/html/v1/colorizers/html-color-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xml",
        path: "schema-packages/xml/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/xml/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xml",
        path: "schema-packages/xml/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/xml/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xml",
        path: "schema-packages/xml/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/xml/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xml",
        path: "schema-packages/xml/v1/formatters/xml-format-document.cemt",
        source: include_str!("../../schema-packages/xml/v1/formatters/xml-format-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xml",
        path: "schema-packages/xml/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/xml/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xml",
        path: "schema-packages/xml/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/xml/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xml",
        path: "schema-packages/xml/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/xml/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xml",
        path: "schema-packages/xml/v1/colorizers/xml-color-document.cemt",
        source: include_str!("../../schema-packages/xml/v1/colorizers/xml-color-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/formatters/xml-compact.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/formatters/xml-compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/formatters/xml-pretty.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/formatters/xml-pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/formatters/xml-tabular.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/formatters/xml-tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/formatters/compact-compact.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/formatters/compact-compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/formatters/compact-pretty.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/formatters/compact-pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/formatters/compact-tabular.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/formatters/compact-tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/formatters/relax-ng-format-schema.cemt",
        source: include_str!(
            "../../schema-packages/relax-ng/v1/formatters/relax-ng-format-schema.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/colorizers/xml-terminal.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/colorizers/xml-terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/colorizers/xml-html.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/colorizers/xml-html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/colorizers/xml-md.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/colorizers/xml-md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/colorizers/compact-terminal.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/colorizers/compact-terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/colorizers/compact-html.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/colorizers/compact-html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/colorizers/compact-md.cemt",
        source: include_str!("../../schema-packages/relax-ng/v1/colorizers/compact-md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "relax-ng",
        path: "schema-packages/relax-ng/v1/colorizers/relax-ng-color-schema.cemt",
        source: include_str!(
            "../../schema-packages/relax-ng/v1/colorizers/relax-ng-color-schema.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xhtml",
        path: "schema-packages/xhtml/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/xhtml/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xhtml",
        path: "schema-packages/xhtml/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/xhtml/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xhtml",
        path: "schema-packages/xhtml/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/xhtml/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xhtml",
        path: "schema-packages/xhtml/v1/formatters/xhtml-format-document.cemt",
        source: include_str!(
            "../../schema-packages/xhtml/v1/formatters/xhtml-format-document.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xhtml",
        path: "schema-packages/xhtml/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/xhtml/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xhtml",
        path: "schema-packages/xhtml/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/xhtml/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xhtml",
        path: "schema-packages/xhtml/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/xhtml/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xhtml",
        path: "schema-packages/xhtml/v1/colorizers/xhtml-color-document.cemt",
        source: include_str!("../../schema-packages/xhtml/v1/colorizers/xhtml-color-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "svg",
        path: "schema-packages/svg/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/svg/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "svg",
        path: "schema-packages/svg/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/svg/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "svg",
        path: "schema-packages/svg/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/svg/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "svg",
        path: "schema-packages/svg/v1/formatters/svg-format-document.cemt",
        source: include_str!("../../schema-packages/svg/v1/formatters/svg-format-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "svg",
        path: "schema-packages/svg/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/svg/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "svg",
        path: "schema-packages/svg/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/svg/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "svg",
        path: "schema-packages/svg/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/svg/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "svg",
        path: "schema-packages/svg/v1/colorizers/svg-color-document.cemt",
        source: include_str!("../../schema-packages/svg/v1/colorizers/svg-color-document.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "mathml",
        path: "schema-packages/mathml/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/mathml/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "mathml",
        path: "schema-packages/mathml/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/mathml/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "mathml",
        path: "schema-packages/mathml/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/mathml/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "mathml",
        path: "schema-packages/mathml/v1/formatters/mathml-format-document.cemt",
        source: include_str!(
            "../../schema-packages/mathml/v1/formatters/mathml-format-document.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "mathml",
        path: "schema-packages/mathml/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/mathml/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "mathml",
        path: "schema-packages/mathml/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/mathml/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "mathml",
        path: "schema-packages/mathml/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/mathml/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "mathml",
        path: "schema-packages/mathml/v1/colorizers/mathml-color-document.cemt",
        source: include_str!(
            "../../schema-packages/mathml/v1/colorizers/mathml-color-document.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xpath",
        path: "schema-packages/xpath/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/xpath/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xpath",
        path: "schema-packages/xpath/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/xpath/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xpath",
        path: "schema-packages/xpath/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/xpath/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xpath",
        path: "schema-packages/xpath/v1/formatters/xpath-format-expression.cemt",
        source: include_str!(
            "../../schema-packages/xpath/v1/formatters/xpath-format-expression.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xpath",
        path: "schema-packages/xpath/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/xpath/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xpath",
        path: "schema-packages/xpath/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/xpath/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xpath",
        path: "schema-packages/xpath/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/xpath/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xpath",
        path: "schema-packages/xpath/v1/colorizers/xpath-color-expression.cemt",
        source: include_str!(
            "../../schema-packages/xpath/v1/colorizers/xpath-color-expression.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xslt",
        path: "schema-packages/xslt/v1/formatters/compact.cemt",
        source: include_str!("../../schema-packages/xslt/v1/formatters/compact.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xslt",
        path: "schema-packages/xslt/v1/formatters/pretty.cemt",
        source: include_str!("../../schema-packages/xslt/v1/formatters/pretty.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xslt",
        path: "schema-packages/xslt/v1/formatters/tabular.cemt",
        source: include_str!("../../schema-packages/xslt/v1/formatters/tabular.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xslt",
        path: "schema-packages/xslt/v1/formatters/xslt-format-stylesheet.cemt",
        source: include_str!(
            "../../schema-packages/xslt/v1/formatters/xslt-format-stylesheet.cemt"
        ),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xslt",
        path: "schema-packages/xslt/v1/colorizers/terminal.cemt",
        source: include_str!("../../schema-packages/xslt/v1/colorizers/terminal.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xslt",
        path: "schema-packages/xslt/v1/colorizers/html.cemt",
        source: include_str!("../../schema-packages/xslt/v1/colorizers/html.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xslt",
        path: "schema-packages/xslt/v1/colorizers/md.cemt",
        source: include_str!("../../schema-packages/xslt/v1/colorizers/md.cemt"),
    },
    BuiltinSchemaPackageArtifactSource {
        package_id: "xslt",
        path: "schema-packages/xslt/v1/colorizers/xslt-color-stylesheet.cemt",
        source: include_str!("../../schema-packages/xslt/v1/colorizers/xslt-color-stylesheet.cemt"),
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
        package_id: "cem-element-template",
        schema_path: "schema-packages/cem-element-template/v1/schema/cem-element-template.cem",
        manifest_source: include_str!("../../schema-packages/cem-element-template/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/cem-element-template/v1/schema/cem-element-template.cem"
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
        package_id: "module-map",
        schema_path: "schema-packages/module-map/v1/schema/module-map.cem",
        manifest_source: include_str!("../../schema-packages/module-map/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/module-map/v1/schema/module-map.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "module-map-v2",
        schema_path: "schema-packages/module-map-v2/v1/schema/module-map-v2.cem",
        manifest_source: include_str!("../../schema-packages/module-map-v2/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/module-map-v2/v1/schema/module-map-v2.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "module-map-v3",
        schema_path: "schema-packages/module-map-v3/v1/schema/module-map-v3.cem",
        manifest_source: include_str!("../../schema-packages/module-map-v3/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/module-map-v3/v1/schema/module-map-v3.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "cli-command",
        schema_path: "schema-packages/cli-command/v1/schema/cli-command.cem",
        manifest_source: include_str!("../../schema-packages/cli-command/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/cli-command/v1/schema/cli-command.cem"
        ),
    },
    BuiltinSchemaPackageSource {
        package_id: "studio-project",
        schema_path: "schema-packages/studio-project/v1/schema/studio-project.cem",
        manifest_source: include_str!("../../schema-packages/studio-project/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/studio-project/v1/schema/studio-project.cem"
        ),
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
        package_id: "xpath",
        schema_path: "schema-packages/xpath/v1/schema/xpath.cem",
        manifest_source: include_str!("../../schema-packages/xpath/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/xpath/v1/schema/xpath.cem"),
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
        package_id: "scss",
        schema_path: "schema-packages/scss/v1/schema/scss.cem",
        manifest_source: include_str!("../../schema-packages/scss/v1/package.cem"),
        schema_source: include_str!("../../schema-packages/scss/v1/schema/scss.cem"),
    },
    BuiltinSchemaPackageSource {
        package_id: "css-selector",
        schema_path: "schema-packages/css-selector/v1/schema/css-selector.cem",
        manifest_source: include_str!("../../schema-packages/css-selector/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/css-selector/v1/schema/css-selector.cem"
        ),
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
    BuiltinSchemaPackageSource {
        package_id: "cem-data-island",
        schema_path: "schema-packages/cem-data-island/v1/schema/cem-data-island.cem",
        manifest_source: include_str!("../../schema-packages/cem-data-island/v1/package.cem"),
        schema_source: include_str!(
            "../../schema-packages/cem-data-island/v1/schema/cem-data-island.cem"
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
        CEM_AST_JSON_PROJECTION_CONTENT_TYPE, CEM_AST_PROJECTION_CONTENT_TYPE,
        CEM_AST_PROJECTION_SCHEMA_URI, CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
        CEM_DOM_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_SCHEMA_URI,
        CEM_ELEMENT_TEMPLATE_CONTENT_TYPE, CEM_ELEMENT_TEMPLATE_SCHEMA_URI,
        CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_CONTENT_TYPE,
        CEM_EVENTS_PROJECTION_SCHEMA_URI, CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI,
        CEM_NATIVE_TEMPLATE_CONTENT_TYPE, CEM_NATIVE_TEMPLATE_SCHEMA_URI, CEM_QL_CONTENT_TYPE,
        CEM_QL_EXPRESSION_CONTENT_TYPE, CEM_QL_EXPRESSION_SCHEMA_URI, CEM_QL_SCHEMA_URI,
        CEM_SCHEMA_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_URI,
        CEM_SCHEMA_URI, CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI, CSS_CONTENT_TYPE,
        CSS_SCHEMA_URI, CSS_SELECTOR_CONTENT_TYPE, CSS_SELECTOR_SCHEMA_URI, CSV_CONTENT_TYPE,
        CSV_SCHEMA_URI, HTML_CONTENT_TYPE, HTML_SCHEMA_URI, JSON_CONTENT_TYPE,
        JSON_SCHEMA_CONTENT_TYPE, JSON_SCHEMA_SCHEMA_URI, JSON_VALUE_SCHEMA_URI,
        MARKDOWN_SCHEMA_URI, MATHML_CONTENT_TYPE, MATHML_SCHEMA_URI, MODULE_MAP_CONTENT_TYPE,
        MODULE_MAP_SCHEMA_URI, MODULE_MAP_V2_SCHEMA_URI, MODULE_MAP_V3_SCHEMA_URI,
        RELAX_NG_COMPACT_CONTENT_TYPE,
        RELAX_NG_SCHEMA_URI, RELAX_NG_XML_CONTENT_TYPE, SCSS_CONTENT_TYPE, SCSS_SCHEMA_URI,
        SVG_CONTENT_TYPE, SVG_SCHEMA_URI, XHTML_CONTENT_TYPE, XHTML_SCHEMA_URI, XML_CONTENT_TYPE,
        XML_SCHEMA_URI, XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI, XSLT_CONTENT_TYPE, XSLT_SCHEMA_URI,
        YAML_CONTENT_TYPE, YAML_SCHEMA_URI,
    };

    #[test]
    fn cem_data_island_package_is_registered() {
        let package = builtin_schema_package_source("cem-data-island")
            .expect("built-in CEM data-island schema package");
        assert_eq!(
            package.schema_path,
            "schema-packages/cem-data-island/v1/schema/cem-data-island.cem"
        );
        assert!(package.manifest_source.contains("https://cem.dev/ns/runtime/data-island"));
        assert!(package.schema_source.contains("@name=\"context-root\""));
        let examples = schema_package_examples_from_package_sources(package)
            .expect("CEM data-island package examples");
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].id, "complete-instance");
        assert_eq!(
            examples[0].expected_result,
            SchemaPackageExampleExpectedResult::Pass
        );
    }
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

    fn assert_manifest_artifact_attr(
        package_id: &str,
        artifact_kind: &str,
        artifact_path: &str,
        attrs: &BTreeMap<String, String>,
        attr_name: &str,
    ) {
        assert!(
            matches!(attrs.get(attr_name), Some(value) if !value.trim().is_empty()),
            "{} `{}` artifact `{}` must declare non-empty `{}` in package.cem",
            package_id,
            artifact_kind,
            artifact_path,
            attr_name
        );
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
    ) -> Vec<crate::schema::registry::SchemaPackageExampleDescriptor> {
        let package = builtin_schema_package_source(package_id).expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();
        let expected_paths = top_level_example_paths(package_id);

        assert_eq!(
            declared_paths, expected_paths,
            "{package_id} top-level examples must be discoverable from package.cem"
        );
        let expected_content_type = crate::schema::registry::content_type_essence(content_type);
        assert!(examples.iter().all(|example| {
            crate::schema::registry::content_type_essence(&example.content_type)
                == expected_content_type
        }));
        assert!(examples.iter().all(|example| example.schema == schema_uri));
        examples
    }

    #[test]
    fn cem_ml_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("cem-ml", CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI);
        let expected = [
            (
                "basic",
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "nested-handoff",
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "embedded-handoffs",
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.handoff.child_parser_deferred"),
            ),
            (
                "formatter-coloring-pipeline-package-artifacts",
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.schema.unresolved_namespace"),
            ),
            (
                "invalid-unclosed-scope",
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ast.unclosed_scope"),
            ),
            (
                "invalid-unsupported-handoffs",
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.handoff.unsupported_content_type"),
            ),
        ];

        let actual_ids = examples
            .iter()
            .map(|example| example.id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_ids = expected
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_ids, expected_ids,
            "cem-ml examples must match the explicit package-owned coverage set"
        );

        for (id, content_type, schema, expected_result, expected_code) in expected {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CEM-ML example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.schema, schema);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn schema_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("schema", CEM_SCHEMA_CONTENT_TYPE, CEM_SCHEMA_URI);
        for id in ["custom-behavior-schema", "custom-behavior-schema-strict"] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("custom behavior schema example `{id}`"));
            assert_eq!(
                example.expected_result,
                SchemaPackageExampleExpectedResult::Pass
            );
            assert!(example.expected_diagnostic_codes.is_empty());
        }

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

        let invalid_behavior = examples
            .iter()
            .find(|example| example.id == "invalid-diagnostic-behavior")
            .expect("invalid diagnostic behavior schema example");
        assert_eq!(
            invalid_behavior.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            invalid_behavior.expected_diagnostic_codes,
            vec!["cem.schema_definition.unknown_diagnostic_behavior".to_owned()]
        );

        for (id, expected_code) in [
            (
                "invalid-custom-behavior-unresolved-function",
                "cem.schema_definition.unresolved_behavior_function",
            ),
            (
                "invalid-custom-behavior-select-query",
                "cem.schema_behavior.query_invalid",
            ),
            (
                "invalid-custom-behavior-match-query",
                "cem.schema_behavior.query_invalid",
            ),
            (
                "invalid-custom-behavior-argument-type",
                "cem.schema_definition.invalid_diagnostic_behavior_contract",
            ),
            (
                "invalid-custom-behavior-signature",
                "cem.schema_definition.invalid_diagnostic_behavior_contract",
            ),
            (
                "invalid-custom-behavior-unsafe-call",
                "cem.schema_behavior.function_failed",
            ),
            (
                "invalid-custom-behavior-contracts",
                "cem.schema_definition.invalid_diagnostic_behavior_contract",
            ),
            (
                "invalid-datatype-param-length",
                "cem.schema_definition.invalid_datatype_param",
            ),
            (
                "invalid-datatype-param-bound",
                "cem.schema_definition.invalid_datatype_param",
            ),
            (
                "invalid-datatype-param-pattern",
                "cem.schema_definition.invalid_datatype_param",
            ),
            (
                "invalid-datatype-param-digits",
                "cem.schema_definition.invalid_datatype_param",
            ),
            (
                "invalid-datatype-param-uri-media",
                "cem.schema_definition.invalid_datatype_param",
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("invalid custom behavior schema example `{id}`"));
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
    fn schema_package_manifest_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "schema-package",
            CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
            CEM_SCHEMA_PACKAGE_URI,
        );
        for (id, expected_code) in [
            (
                "invalid-converter-contract",
                "cem.schema_package.converter_check",
            ),
            (
                "invalid-converter-template-unreadable",
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
                "invalid-primary-content-type",
                "cem.schema_package.content_type_conflict",
            ),
            (
                "invalid-primary-content-type-missing",
                "cem.schema_package.content_type_conflict",
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
        let invalid_schema_metadata = examples
            .iter()
            .find(|example| example.id == "invalid-schema-metadata")
            .expect("schema-package invalid schema metadata example");
        assert_eq!(
            invalid_schema_metadata.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            invalid_schema_metadata.expected_diagnostic_codes,
            vec![
                "cem.schema_package.schema_uri_mismatch".to_owned(),
                "cem.schema_package.schema_content_type_mismatch".to_owned(),
                "cem.schema_package.schema_namespace_mismatch".to_owned(),
            ]
        );
        for (id, expected_code) in [
            (
                "invalid-schema-source-unreadable",
                "cem.schema_package.schema_source_unreadable",
            ),
            (
                "invalid-schema-source-invalid",
                "cem.schema_package.schema_source_invalid",
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
        let missing_required = examples
            .iter()
            .find(|example| example.id == "invalid-missing-required-attribute")
            .expect("schema-package missing required example");
        assert_eq!(
            missing_required.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            missing_required.expected_diagnostic_codes,
            vec![
                "cem.schema_model.missing_required_attribute".to_owned(),
                "cem.schema_package.package_check".to_owned(),
            ]
        );
    }

    #[test]
    fn cem_native_template_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "cem-native-template",
            CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
            CEM_NATIVE_TEMPLATE_SCHEMA_URI,
        );

        let expected = [
            (
                "basic-template",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "module-template",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-missing-required-attribute",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.schema_model.missing_required_attribute"),
            ),
            (
                "invalid-duplicate-import-alias",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.template.import_alias_duplicate"),
            ),
            (
                "invalid-duplicate-template-entrypoint",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.template.entrypoint_duplicate"),
            ),
            (
                "invalid-duplicate-param",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.template.param_duplicate"),
            ),
            (
                "invalid-duplicate-let",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.template.let_duplicate"),
            ),
            (
                "invalid-unknown-call",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.template.call_unknown"),
            ),
            (
                "invalid-default-expr-reserved",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.template.param_default_expr_reserved"),
            ),
            (
                "invalid-expression-parse",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.use_rust_boolean_ops"),
            ),
            (
                "invalid-expression-type-error",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.type_error"),
            ),
            (
                "invalid-expression-data-binding",
                CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.data_binding_missing"),
            ),
        ];
        let actual_ids = examples
            .iter()
            .map(|example| example.id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_ids = expected
            .iter()
            .map(|(id, _, _, _)| *id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_ids, expected_ids,
            "checked CEM-native template example expectations must cover every manifest example"
        );

        for (id, content_type, expected_result, expected_code) in expected {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CEM-native template example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn cem_element_template_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "cem-element-template",
            CEM_ELEMENT_TEMPLATE_CONTENT_TYPE,
            CEM_ELEMENT_TEMPLATE_SCHEMA_URI,
        );

        assert_eq!(examples.len(), 2);
        let basic = examples
            .iter()
            .find(|example| example.id == "basic-card")
            .expect("basic CEM Element template example");
        assert_eq!(
            basic.expected_result,
            SchemaPackageExampleExpectedResult::Pass
        );
        assert!(basic.expected_diagnostic_codes.is_empty());

        let invalid = examples
            .iter()
            .find(|example| example.id == "invalid-unknown-instruction")
            .expect("invalid CEM Element template example");
        assert_eq!(
            invalid.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            invalid.expected_diagnostic_codes,
            vec!["cem.schema.unknown_html_element".to_owned()]
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

        let expected = [
            (
                "basic-transform",
                CEM_TRANSFORM_CONTENT_TYPE,
                CEM_TRANSFORM_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "module-transform",
                CEM_TRANSFORM_CONTENT_TYPE,
                CEM_TRANSFORM_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "function-declarations",
                CEM_TRANSFORM_CONTENT_TYPE,
                CEM_TRANSFORM_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "formatter-coloring-pipeline",
                CEM_TRANSFORM_CONTENT_TYPE,
                CEM_TRANSFORM_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "formatter-coloring-pipeline-fixture",
                CEM_ML_CONTENT_TYPE,
                CEM_ML_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.schema.unresolved_namespace"),
            ),
            (
                "invalid-missing-required-attribute",
                CEM_TRANSFORM_CONTENT_TYPE,
                CEM_TRANSFORM_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.schema_model.missing_required_attribute"),
            ),
            (
                "invalid-function-missing-category",
                CEM_TRANSFORM_CONTENT_TYPE,
                CEM_TRANSFORM_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.schema_model.missing_required_attribute"),
            ),
            (
                "invalid-function-missing-contract-metadata",
                CEM_TRANSFORM_CONTENT_TYPE,
                CEM_TRANSFORM_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.schema_model.missing_required_attribute"),
            ),
        ];
        let actual_ids = examples
            .iter()
            .map(|example| example.id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_ids = expected
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_ids, expected_ids,
            "checked CEM transform example expectations must cover every manifest example"
        );

        for (id, content_type, schema, expected_result, expected_code) in expected {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CEM transform example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.schema, schema);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn cem_ql_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("cem-ql").expect("CEM-QL package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("CEM-QL package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            declared_paths,
            top_level_example_paths("cem-ql"),
            "CEM-QL top-level examples must be discoverable from package.cem"
        );
        assert!(examples.iter().all(|example| {
            example.schema == CEM_QL_SCHEMA_URI || example.schema == CEM_QL_EXPRESSION_SCHEMA_URI
        }));

        let expected = [
            (
                "basic-query",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "basic-expression",
                CEM_QL_EXPRESSION_CONTENT_TYPE,
                CEM_QL_EXPRESSION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-expression-parse",
                CEM_QL_EXPRESSION_CONTENT_TYPE,
                CEM_QL_EXPRESSION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.parse_error"),
            ),
            (
                "invalid-expression-type-error",
                CEM_QL_EXPRESSION_CONTENT_TYPE,
                CEM_QL_EXPRESSION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.type_error"),
            ),
            (
                "invalid-expression-data-binding",
                CEM_QL_EXPRESSION_CONTENT_TYPE,
                CEM_QL_EXPRESSION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.data_binding_missing"),
            ),
            (
                "module-query",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "operators-and-control",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "collections-and-pipelines",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "stdlib-data-helpers",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "host-resource-helpers",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "alias-content-type",
                "text/cem-ql",
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "line-ending-lf",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "line-ending-crlf",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "comments-and-whitespace",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "source-token-ranges",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "compiled-artifact-identity",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-parse",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.parse_error"),
            ),
            (
                "invalid-missing-module",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.module_uri_missing"),
            ),
            (
                "invalid-old-syntax",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.use_rust_boolean_ops"),
            ),
            (
                "invalid-utf8",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.invalid_utf8"),
            ),
            (
                "invalid-duplicate-import-alias",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.import_alias_duplicate"),
            ),
            (
                "invalid-unresolved-import",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.import_unresolved"),
            ),
            (
                "invalid-type-error",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.type_error"),
            ),
            (
                "invalid-duplicate-declaration",
                CEM_QL_CONTENT_TYPE,
                CEM_QL_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.ql.declaration_duplicate"),
            ),
        ];
        let actual_ids = examples
            .iter()
            .map(|example| example.id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_ids = expected
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_ids, expected_ids,
            "checked CEM-QL example expectations must cover every manifest example"
        );

        for (id, content_type, schema, expected_result, expected_code) in expected {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CEM-QL example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.schema, schema);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn cem_ql_formatter_and_colorizer_track_rust_first_operator_roles() {
        let formatter = builtin_schema_package_artifact_source(
            "cem-ql",
            "schema-packages/cem-ql/v1/formatters/cem-ql-format-tree.cemt",
        )
        .expect("CEM-QL formatter artifact source")
        .source;
        for needle in [
            r#"@name="cem-ql.format-tree.operator-role""#,
            r#""&&": "cem-ql.operator.boolean""#,
            r#""/": "cem-ql.operator.arithmetic-or-child""#,
            r#""is": "cem-ql.operator.type-test""#,
            r#""same_node": "cem-ql.operator.node-identity""#,
            r#""div": "/""#,
            r#""lambda": "fn(...) => expression""#,
            r#""and": "cem.ql.use_rust_boolean_ops""#,
            r#"@name="cem-ql.format-tree.legacy-diagnostic""#,
        ] {
            assert!(
                formatter.contains(needle),
                "CEM-QL formatter missing Rust-first role contract `{needle}`"
            );
        }

        let colorizer = builtin_schema_package_artifact_source(
            "cem-ql",
            "schema-packages/cem-ql/v1/colorizers/cem-ql-color-tree.cemt",
        )
        .expect("CEM-QL colorizer artifact source")
        .source;
        for needle in [
            r#"@name="cem-ql.color-tree.token-role""#,
            r#""==": "cem-ql.operator.comparison""#,
            r#""%": "cem-ql.operator.arithmetic""#,
            r#""|": "cem-ql.operator.set""#,
            r#""cem-ql.operator.boolean": "syntax.punctuation""#,
            r#""cem-ql.legacy.syntax": "diagnostic.error""#,
            r#""True": "diagnostic.error""#,
            r#""lambda": "diagnostic.error""#,
        ] {
            assert!(
                colorizer.contains(needle),
                "CEM-QL colorizer missing Rust-first role contract `{needle}`"
            );
        }

        let schema = builtin_schema_package_source("cem-ql")
            .expect("CEM-QL package source")
            .schema_source;
        assert!(
            schema.contains(r#"@code="cem.ql.use_rust_boolean_ops""#),
            "CEM-QL schema must declare the legacy boolean syntax diagnostic"
        );
    }

    #[test]
    fn json_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("json", JSON_CONTENT_TYPE, JSON_VALUE_SCHEMA_URI);
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
    fn yaml_output_assets_follow_schema_package_readme_contract() {
        let package = builtin_schema_package_source("yaml").expect("YAML package source");
        let artifacts =
            package_manifest_artifact_attrs(package.package_id, package.manifest_source);
        let output_asset_paths = artifacts
            .iter()
            .filter(|attrs| {
                matches!(
                    attrs.get("kind").map(String::as_str),
                    Some("formatter" | "colorizer")
                )
            })
            .filter_map(|attrs| attrs.get("path").cloned())
            .collect::<BTreeSet<_>>();
        let expected_paths = [
            "schema-packages/yaml/v1/formatters/compact.cemt",
            "schema-packages/yaml/v1/formatters/pretty.cemt",
            "schema-packages/yaml/v1/formatters/tabular.cemt",
            "schema-packages/yaml/v1/colorizers/terminal.cemt",
            "schema-packages/yaml/v1/colorizers/html.cemt",
            "schema-packages/yaml/v1/colorizers/md.cemt",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

        assert_eq!(output_asset_paths, expected_paths);

        for (path, kind, profile_attr, profile) in [
            (
                "schema-packages/yaml/v1/formatters/compact.cemt",
                "formatter",
                "formatter-profile",
                "compact",
            ),
            (
                "schema-packages/yaml/v1/formatters/pretty.cemt",
                "formatter",
                "formatter-profile",
                "pretty",
            ),
            (
                "schema-packages/yaml/v1/formatters/tabular.cemt",
                "formatter",
                "formatter-profile",
                "tabular",
            ),
            (
                "schema-packages/yaml/v1/colorizers/terminal.cemt",
                "colorizer",
                "color-profile",
                "terminal",
            ),
            (
                "schema-packages/yaml/v1/colorizers/html.cemt",
                "colorizer",
                "color-profile",
                "html",
            ),
            (
                "schema-packages/yaml/v1/colorizers/md.cemt",
                "colorizer",
                "color-profile",
                "md",
            ),
        ] {
            let attrs = artifacts
                .iter()
                .find(|attrs| attrs.get("path").map(String::as_str) == Some(path))
                .unwrap_or_else(|| panic!("YAML output asset `{path}`"));
            assert_eq!(attrs.get("kind").map(String::as_str), Some(kind));
            assert_eq!(attrs.get(profile_attr).map(String::as_str), Some(profile));
            let function_profile = match profile {
                "pretty" => "yaml.pretty",
                _ => profile,
            };
            assert_eq!(
                attrs.get("function-profile").map(String::as_str),
                Some(function_profile)
            );
        }
    }

    #[test]
    fn csv_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples("csv", CSV_CONTENT_TYPE, CSV_SCHEMA_URI);

        let expected = [
            (
                "basic-table",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "quoted-fields",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "header-absent",
                "text/csv; header=absent",
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "line-ending-lf",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "line-ending-crlf",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "utf8-bom",
                "text/csv; charset=utf-8",
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "spaced-fields",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "tabs-and-empty-fields",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "formula-looking-values",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "wide-unicode",
                "text/csv; charset=utf-8",
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-unclosed-quote",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.csv.unclosed_quote"),
            ),
            (
                "invalid-quote-escape",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.csv.invalid_quote_escape"),
            ),
            (
                "ragged-row",
                CSV_CONTENT_TYPE,
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.csv.inconsistent_field_count"),
            ),
            (
                "unsupported-charset",
                "text/csv; charset=iso-8859-1",
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.csv.unsupported_encoding"),
            ),
            (
                "us-ascii-non-ascii-byte",
                "text/csv; charset=us-ascii",
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.csv.unsupported_encoding"),
            ),
            (
                "invalid-header-parameter",
                "text/csv; header=maybe",
                CSV_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.csv.invalid_header_parameter"),
            ),
        ];
        let actual_ids = examples
            .iter()
            .map(|example| example.id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_ids = expected
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_ids, expected_ids,
            "checked CSV example expectations must cover every manifest example"
        );

        for (id, content_type, schema, expected_result, expected_code) in expected {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CSV example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.schema, schema);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn csv_output_assets_follow_schema_package_readme_contract() {
        let package = builtin_schema_package_source("csv").expect("CSV package source");
        let artifacts =
            package_manifest_artifact_attrs(package.package_id, package.manifest_source);
        let output_asset_paths = artifacts
            .iter()
            .filter(|attrs| {
                matches!(
                    attrs.get("kind").map(String::as_str),
                    Some("formatter" | "colorizer")
                )
            })
            .filter_map(|attrs| attrs.get("path").cloned())
            .collect::<BTreeSet<_>>();
        let expected_paths = [
            "schema-packages/csv/v1/formatters/compact.cemt",
            "schema-packages/csv/v1/formatters/pretty.cemt",
            "schema-packages/csv/v1/formatters/tabular.cemt",
            "schema-packages/csv/v1/colorizers/terminal.cemt",
            "schema-packages/csv/v1/colorizers/html.cemt",
            "schema-packages/csv/v1/colorizers/md.cemt",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

        assert_eq!(output_asset_paths, expected_paths);

        for (path, kind, profile_attr, profile) in [
            (
                "schema-packages/csv/v1/formatters/compact.cemt",
                "formatter",
                "formatter-profile",
                "compact",
            ),
            (
                "schema-packages/csv/v1/formatters/pretty.cemt",
                "formatter",
                "formatter-profile",
                "pretty",
            ),
            (
                "schema-packages/csv/v1/formatters/tabular.cemt",
                "formatter",
                "formatter-profile",
                "tabular",
            ),
            (
                "schema-packages/csv/v1/colorizers/terminal.cemt",
                "colorizer",
                "color-profile",
                "terminal",
            ),
            (
                "schema-packages/csv/v1/colorizers/html.cemt",
                "colorizer",
                "color-profile",
                "html",
            ),
            (
                "schema-packages/csv/v1/colorizers/md.cemt",
                "colorizer",
                "color-profile",
                "md",
            ),
        ] {
            let attrs = artifacts
                .iter()
                .find(|attrs| attrs.get("path").map(String::as_str) == Some(path))
                .unwrap_or_else(|| panic!("CSV output asset `{path}`"));
            assert_eq!(attrs.get("kind").map(String::as_str), Some(kind));
            assert_eq!(attrs.get(profile_attr).map(String::as_str), Some(profile));
            assert_eq!(
                attrs.get("function-profile").map(String::as_str),
                Some(profile)
            );
            assert!(
                builtin_schema_package_artifact_source("csv", path).is_some(),
                "CSV output asset `{path}` must be embedded"
            );
        }
    }

    #[test]
    fn markdown_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("markdown").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("markdown"),
            "markdown top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), 5);
        assert!(examples
            .iter()
            .all(|example| example.schema == MARKDOWN_SCHEMA_URI));

        for (id, content_type, expected_result, expected_code) in [
            (
                "basic-document",
                "text/markdown; charset=utf-8; variant=CommonMark",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "gfm-worklog",
                "text/markdown; charset=utf-8; variant=GFM",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "markdown-html-svg",
                "text/markdown; charset=utf-8; variant=CommonMark",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-embedded-html",
                "text/markdown; charset=utf-8; variant=CommonMark",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.markdown.embedded_html_rejected"),
            ),
            (
                "unknown-variant",
                "text/markdown; charset=utf-8; variant=CustomWiki",
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.markdown.unknown_variant"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("Markdown example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn module_map_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "module-map",
            MODULE_MAP_CONTENT_TYPE,
            MODULE_MAP_SCHEMA_URI,
        );
        assert_eq!(examples.len(), 2);
        assert!(examples
            .iter()
            .all(|example| example.expected_result == SchemaPackageExampleExpectedResult::Pass));
    }

    #[test]
    fn module_map_v2_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "module-map-v2",
            MODULE_MAP_CONTENT_TYPE,
            MODULE_MAP_V2_SCHEMA_URI,
        );
        assert_eq!(examples.len(), 2);
        assert!(examples
            .iter()
            .all(|example| example.expected_result == SchemaPackageExampleExpectedResult::Pass));
    }

    #[test]
    fn module_map_v3_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "module-map-v3",
            MODULE_MAP_CONTENT_TYPE,
            MODULE_MAP_V3_SCHEMA_URI,
        );
        assert_eq!(examples.len(), 2);
        assert!(examples
            .iter()
            .all(|example| example.expected_result == SchemaPackageExampleExpectedResult::Pass));
    }

    #[test]
    fn css_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples("css", CSS_CONTENT_TYPE, CSS_SCHEMA_URI);

        for (id, expected_result, expected_code) in [
            (
                "basic-stylesheet",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "scoped-component",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "style-attribute",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-import",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.css.import_rejected"),
            ),
            (
                "invalid-url",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.css.url_rejected"),
            ),
            (
                "invalid-token",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.css.invalid_token"),
            ),
            (
                "invalid-declaration",
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.css.invalid_declaration"),
            ),
            (
                "encoding-conflict",
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.css.encoding_conflict"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CSS example `{id}`"));
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn scss_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("scss").expect("SCSS package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("SCSS package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(declared_paths, top_level_example_paths("scss"));
        assert_eq!(examples.len(), 7);
        assert!(examples
            .iter()
            .all(|example| example.schema == SCSS_SCHEMA_URI));

        for (id, content_type, expected_result, expected_code) in [
            (
                "basic-source",
                SCSS_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "tokens-partial",
                SCSS_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "module-entry",
                "text/vnd.cem.scss; charset=utf-8",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "forward-entry",
                SCSS_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "deprecated-import",
                SCSS_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.scss.import_deprecated"),
            ),
            (
                "compatibility-alias",
                "text/x-scss; charset=UTF-8",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-indented-syntax",
                SCSS_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.scss.parse_error"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("SCSS example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn css_selector_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples(
            "css-selector",
            CSS_SELECTOR_CONTENT_TYPE,
            CSS_SELECTOR_SCHEMA_URI,
        );

        assert_eq!(examples.len(), 9);
        for (id, expected_result, expected_code) in [
            (
                "basic-selector",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "relational-selector",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "unbound-namespace",
                SchemaPackageExampleExpectedResult::Fail,
                Some("css-selector.namespace.unbound"),
            ),
            (
                "source-map-selector",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "namespace-wildcard",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "budgeted-relational",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-syntax",
                SchemaPackageExampleExpectedResult::Fail,
                Some("css-selector.parse.invalid"),
            ),
            (
                "unsupported-pseudo-element",
                SchemaPackageExampleExpectedResult::Fail,
                Some("css-selector.feature.unsupported"),
            ),
            (
                "missing-host-capability",
                SchemaPackageExampleExpectedResult::Fail,
                Some("css-selector.capability.missing"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CSS selector example `{id}`"));
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn html_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("html", HTML_CONTENT_TYPE, HTML_SCHEMA_URI);

        for (id, expected_result, expected_code) in [
            (
                "basic-document",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            ("fragment", SchemaPackageExampleExpectedResult::Pass, None),
            (
                "svg-mathml-islands",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-script",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.html.script_rejected"),
            ),
            (
                "invalid-external-resource",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.html.external_resource_rejected"),
            ),
            (
                "invalid-custom-element",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.html.custom_element_name_invalid"),
            ),
            (
                "encoding-conflict",
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.html.encoding_conflict"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("HTML example `{id}`"));
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn xml_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("xml").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("xml"),
            "xml top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), 5);
        assert!(examples
            .iter()
            .all(|example| example.schema == XML_SCHEMA_URI));

        for (id, content_type, expected_result, expected_code) in [
            (
                "basic-document",
                XML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "namespaced-document",
                "text/xml; charset=utf-8",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-mismatched-tag",
                XML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xml.parse_error"),
            ),
            (
                "invalid-unbound-prefix",
                XML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xml.unbound_namespace_prefix"),
            ),
            (
                "invalid-doctype",
                XML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xml.dtd_rejected"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("XML example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn relax_ng_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("relax-ng").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("relax-ng"),
            "relax-ng top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), 6);
        assert!(examples
            .iter()
            .all(|example| example.schema == RELAX_NG_SCHEMA_URI));

        for (id, content_type, expected_result, expected_code) in [
            (
                "basic-schema-xml",
                RELAX_NG_XML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "datatype-schema",
                RELAX_NG_XML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "basic-schema-compact",
                RELAX_NG_COMPACT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-missing-start",
                RELAX_NG_XML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.relax_ng.missing_start"),
            ),
            (
                "invalid-unknown-element",
                RELAX_NG_XML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.relax_ng.unknown_element"),
            ),
            (
                "invalid-unclosed-compact",
                RELAX_NG_COMPACT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.relax_ng.compact_parse_error"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("RELAX NG example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn xhtml_package_examples_are_manifest_indexed() {
        let examples =
            manifest_indexed_package_examples("xhtml", XHTML_CONTENT_TYPE, XHTML_SCHEMA_URI);

        for (id, expected_result, expected_code) in [
            (
                "basic-document",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            ("form-page", SchemaPackageExampleExpectedResult::Pass, None),
            (
                "invalid-missing-namespace",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xhtml.namespace_missing"),
            ),
            (
                "invalid-body-before-head",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xhtml.head_body_order"),
            ),
            (
                "invalid-not-well-formed",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xhtml.not_well_formed_xml"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("XHTML example `{id}`"));
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn svg_package_examples_are_manifest_indexed() {
        let examples = manifest_indexed_package_examples("svg", SVG_CONTENT_TYPE, SVG_SCHEMA_URI);

        for (id, expected_result, expected_code) in [
            ("basic-icon", SchemaPackageExampleExpectedResult::Pass, None),
            ("bar-chart", SchemaPackageExampleExpectedResult::Pass, None),
            (
                "unnamed-icon",
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.svg.accessible_name_missing"),
            ),
            (
                "invalid-missing-namespace",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.svg.namespace_missing"),
            ),
            (
                "invalid-script",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.svg.script_rejected"),
            ),
            (
                "invalid-external-image",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.svg.external_resource_rejected"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("SVG example `{id}`"));
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn mathml_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("mathml").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("mathml"),
            "mathml top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), 7);
        assert!(examples
            .iter()
            .all(|example| example.schema == MATHML_SCHEMA_URI));

        for (id, content_type, expected_result, expected_code) in [
            (
                "basic-presentation",
                MATHML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "content-expression",
                "application/mathml-content+xml",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "semantics-external-annotation",
                MATHML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                Some("cem.mathml.external_annotation_rejected"),
            ),
            (
                "invalid-missing-namespace",
                MATHML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.mathml.namespace_missing"),
            ),
            (
                "invalid-root-not-math",
                MATHML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.mathml.root_not_math"),
            ),
            (
                "invalid-content-profile-presentation-only",
                "application/mathml-content+xml",
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.mathml.malformed_expression"),
            ),
            (
                "invalid-not-well-formed",
                MATHML_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.mathml.not_well_formed_xml"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("MathML example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn xpath_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("xpath").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("xpath"),
            "XPath top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), 10);
        assert!(examples
            .iter()
            .all(|example| example.schema == XPATH_SCHEMA_URI));
        assert!(examples.iter().any(|example| {
            example.id == "basic-path"
                && example.content_type == XPATH_CONTENT_TYPE
                && example.expected_result == SchemaPackageExampleExpectedResult::Pass
        }));
        assert!(examples.iter().any(|example| {
            example.id == "functions-and-variables"
                && example.content_type == "text/xpath"
                && example.expected_result == SchemaPackageExampleExpectedResult::Pass
        }));
        let invalid = examples
            .iter()
            .find(|example| example.id == "invalid-unclosed-predicate")
            .expect("invalid XPath example");
        assert_eq!(
            invalid.expected_result,
            SchemaPackageExampleExpectedResult::Fail
        );
        assert_eq!(
            invalid.expected_diagnostic_codes,
            [
                "cem.xpath.parse_error".to_owned(),
                "cem.xpath.unclosed_delimiter".to_owned(),
            ]
        );
        for (id, expected_codes) in [
            ("unknown-prefix", vec!["cem.xpath.unknown_namespace_prefix"]),
            ("invalid-token", vec!["cem.xpath.lexical_error"]),
            (
                "mismatched-delimiter",
                vec![
                    "cem.xpath.parse_error",
                    "cem.xpath.mismatched_delimiter",
                    "cem.xpath.unclosed_delimiter",
                ],
            ),
            (
                "external-resource-denied",
                vec!["cem.xpath.external_resource_denied"],
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("XPath example `{id}`"));
            assert_eq!(
                example.expected_result,
                SchemaPackageExampleExpectedResult::Fail
            );
            assert_eq!(
                example.expected_diagnostic_codes,
                expected_codes
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn xslt_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("xslt").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("xslt"),
            "xslt top-level examples must be discoverable from package.cem"
        );
        assert_eq!(examples.len(), 11);
        assert!(examples
            .iter()
            .all(|example| example.schema == XSLT_SCHEMA_URI));

        for (id, content_type, expected_result, expected_code) in [
            (
                "basic-stylesheet",
                XSLT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "named-template",
                "text/xsl",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "profile-semantics-characterization",
                XSLT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                Some("legacy_xslt.unsupported_construct"),
            ),
            (
                "legacy-custom-element-stylesheet",
                "custom-element-xslt",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "legacy-custom-element-fragment",
                "custom-element-xslt",
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "unsupported-extension-warning",
                XSLT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Pass,
                Some("legacy_xslt.unsupported_construct"),
            ),
            (
                "invalid-missing-namespace",
                XSLT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xslt.namespace_missing"),
            ),
            (
                "invalid-missing-version",
                XSLT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xslt.version_missing"),
            ),
            (
                "invalid-external-include",
                XSLT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xslt.external_uri_rejected"),
            ),
            (
                "invalid-missing-entrypoint",
                XSLT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xslt.entrypoint_missing"),
            ),
            (
                "invalid-not-well-formed",
                XSLT_CONTENT_TYPE,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.xslt.not_well_formed_xml"),
            ),
        ] {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("XSLT example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn cem_dom_projection_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("cem-dom-projection").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("cem-dom-projection"),
            "cem-dom-projection top-level examples must be discoverable from package.cem"
        );

        let expected = [
            (
                "basic-dom",
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "basic-dom-json",
                CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "nested-dom-json",
                CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-kind-json",
                CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.projection.dom.json_shape"),
            ),
            (
                "invalid-binary",
                CEM_DOM_PROJECTION_CONTENT_TYPE,
                CEM_DOM_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.projection.dom.binary_magic"),
            ),
        ];

        let actual_ids = examples
            .iter()
            .map(|example| example.id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_ids = expected
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_ids, expected_ids,
            "cem-dom-projection examples must match the explicit package-owned coverage set"
        );

        for (id, content_type, schema, expected_result, expected_code) in expected {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CEM DOM projection example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.schema, schema);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn cem_ast_projection_package_examples_are_manifest_indexed() {
        let package = builtin_schema_package_source("cem-ast-projection").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("cem-ast-projection"),
            "cem-ast-projection top-level examples must be discoverable from package.cem"
        );

        let expected = [
            (
                "basic-ast",
                CEM_AST_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "basic-ast-json",
                CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "nested-ast-json",
                CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-kind-json",
                CEM_AST_JSON_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.projection.ast.json_shape"),
            ),
            (
                "invalid-binary",
                CEM_AST_PROJECTION_CONTENT_TYPE,
                CEM_AST_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.projection.ast.binary_magic"),
            ),
        ];
        let actual_ids = examples
            .iter()
            .map(|example| example.id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_ids = expected
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_ids, expected_ids,
            "checked CEM AST projection example expectations must cover every manifest example"
        );

        for (id, content_type, schema, expected_result, expected_code) in expected {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CEM AST projection example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.schema, schema);
            assert_eq!(example.expected_result, expected_result);
            let expected_codes = expected_code
                .map(|code| vec![code.to_owned()])
                .unwrap_or_default();
            assert_eq!(example.expected_diagnostic_codes, expected_codes);
        }
    }

    #[test]
    fn cem_events_projection_package_examples_are_manifest_indexed() {
        let package =
            builtin_schema_package_source("cem-events-projection").expect("package source");
        let examples =
            schema_package_examples_from_package_sources(package).expect("package examples");
        let declared_paths = examples
            .iter()
            .map(|example| example.path.clone())
            .collect::<BTreeSet<_>>();

        assert_eq!(
            declared_paths,
            top_level_example_paths("cem-events-projection"),
            "cem-events-projection top-level examples must be discoverable from package.cem"
        );

        let expected = [
            (
                "basic-events",
                CEM_EVENTS_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "basic-events-json",
                CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "nested-events-json",
                CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Pass,
                None,
            ),
            (
                "invalid-kind-json",
                CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.projection.events.json_shape"),
            ),
            (
                "invalid-binary",
                CEM_EVENTS_PROJECTION_CONTENT_TYPE,
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
                SchemaPackageExampleExpectedResult::Fail,
                Some("cem.projection.events.binary_magic"),
            ),
        ];
        let actual_ids = examples
            .iter()
            .map(|example| example.id.as_str())
            .collect::<BTreeSet<_>>();
        let expected_ids = expected
            .iter()
            .map(|(id, _, _, _, _)| *id)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            actual_ids, expected_ids,
            "checked CEM events projection example expectations must cover every manifest example"
        );

        for (id, content_type, schema, expected_result, expected_code) in expected {
            let example = examples
                .iter()
                .find(|example| example.id == id)
                .unwrap_or_else(|| panic!("CEM events projection example `{id}`"));
            assert_eq!(example.content_type, content_type);
            assert_eq!(example.schema, schema);
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
    fn declared_builtin_artifacts_match_embedded_folder_sources() {
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
                    embedded_artifact_paths.contains(&(package.package_id, path.as_str())),
                    "{} declared artifact `{}` must be embedded in package_sources.rs",
                    package.package_id,
                    path
                );
            }
        }
    }

    #[test]
    fn builtin_output_artifacts_declare_manifest_metadata_required_for_loading() {
        const OUTPUT_STAGE_REQUIRED_ATTRS: &[&str] = &[
            "content-type",
            "schema",
            "target-content-type",
            "target-schema",
            "target-category",
            "function-name",
        ];

        for package in builtin_schema_package_sources() {
            let package_id = package.package_id;
            for attrs in package_manifest_artifact_attrs(package_id, package.manifest_source) {
                let Some(kind) = attrs.get("kind").map(String::as_str) else {
                    continue;
                };
                if output_artifact_directory(kind).is_none() {
                    continue;
                }
                let path = attrs.get("path").map(String::as_str).unwrap_or("<missing>");

                for attr_name in OUTPUT_STAGE_REQUIRED_ATTRS {
                    assert_manifest_artifact_attr(package_id, kind, path, &attrs, attr_name);
                }

                match kind {
                    "formatter" => assert_manifest_artifact_attr(
                        package_id,
                        kind,
                        path,
                        &attrs,
                        "formatter-profile",
                    ),
                    "colorizer" => assert_manifest_artifact_attr(
                        package_id,
                        kind,
                        path,
                        &attrs,
                        "color-profile",
                    ),
                    _ => {}
                }
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
            .contains(r#"node.kind == "raw-text""#));
        assert!(dom_xml_converter
            .source
            .contains(r#"{template @name="emit-node""#));
        assert!(dom_xml_converter.source.contains(r#"node.kind == "cdata""#));
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
            "schema-packages/json/v1/formatters/tabular.cemt",
        )
        .expect("JSON formatter source");
        let formatter_helper = builtin_schema_package_artifact_source(
            "json",
            "schema-packages/json/v1/formatters/json-format-document.cemt",
        )
        .expect("JSON formatter helper source");
        let colorizer = builtin_schema_package_artifact_source(
            "json",
            "schema-packages/json/v1/colorizers/html.cemt",
        )
        .expect("JSON colorizer source");
        let colorizer_helper = builtin_schema_package_artifact_source(
            "json",
            "schema-packages/json/v1/colorizers/json-color-document.cemt",
        )
        .expect("JSON colorizer helper source");

        assert!(formatter.source.contains(r#"@name="json.format-document""#));
        assert!(formatter.source.contains(r#"@category="json-document""#));
        assert!(formatter_helper
            .source
            .contains(r#"@name="json.format-document.tree""#));
        assert!(colorizer.source.contains(r#"@name="json.color-document""#));
        assert!(colorizer
            .source
            .contains(r#"@content-type="application/json""#));
        assert!(colorizer_helper
            .source
            .contains(r#"@name="json.color-document.tree""#));
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
        assert!(formatter
            .source
            .contains(r#"@subject="json-schema-document""#));
        assert!(formatter.source.contains(r#"@produces="cem-tree""#));
        assert!(colorizer
            .source
            .contains(r#"@name="json-schema.color-document""#));
        assert!(colorizer.source.contains(r#"@subject="cem-tree""#));
        assert!(colorizer.source.contains(r#"@produces="cem-tree""#));
        assert!(colorizer
            .source
            .contains(r#"@content-type="application/schema+json""#));
        assert!(!formatter.source.contains(r#"@subject="json""#));
        assert!(!formatter.source.contains(r#"@produces="tokens""#));
        assert!(!colorizer.source.contains(r#"@subject="tokens""#));
        assert!(!colorizer.source.contains(r#"@produces="tokens""#));
    }

    #[test]
    fn catalog_exposes_yaml_output_artifact_sources() {
        for profile in BASELINE_FORMATTER_PROFILES {
            let formatter = builtin_schema_package_artifact_source(
                "yaml",
                &format!("schema-packages/yaml/v1/formatters/{profile}.cemt"),
            )
            .unwrap_or_else(|| panic!("YAML `{profile}` formatter source"));
            let function_profile = match *profile {
                "pretty" => "yaml.pretty",
                _ => profile,
            };

            assert!(formatter.source.contains(r#"@name="yaml.format-document""#));
            assert!(formatter.source.contains(r#"@category="yaml-document""#));
            assert!(formatter
                .source
                .contains(&format!(r#"@profile="{function_profile}""#)));
        }

        for profile in BASELINE_COLORIZER_PROFILES {
            let colorizer = builtin_schema_package_artifact_source(
                "yaml",
                &format!("schema-packages/yaml/v1/colorizers/{profile}.cemt"),
            )
            .unwrap_or_else(|| panic!("YAML `{profile}` colorizer source"));

            assert!(colorizer.source.contains(r#"@name="yaml.color-document""#));
            assert!(colorizer
                .source
                .contains(r#"@content-type="application/yaml""#));
            assert!(colorizer
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
        }
    }

    #[test]
    fn catalog_exposes_csv_output_artifact_sources() {
        for profile in BASELINE_FORMATTER_PROFILES {
            let formatter = builtin_schema_package_artifact_source(
                "csv",
                &format!("schema-packages/csv/v1/formatters/{profile}.cemt"),
            )
            .unwrap_or_else(|| panic!("CSV `{profile}` formatter source"));

            assert!(formatter.source.contains(r#"@name="csv.format-document""#));
            assert!(formatter.source.contains(r#"@category="csv-document""#));
            assert!(formatter
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
        }

        for profile in BASELINE_COLORIZER_PROFILES {
            let colorizer = builtin_schema_package_artifact_source(
                "csv",
                &format!("schema-packages/csv/v1/colorizers/{profile}.cemt"),
            )
            .unwrap_or_else(|| panic!("CSV `{profile}` colorizer source"));

            assert!(colorizer.source.contains(r#"@name="csv.color-document""#));
            assert!(colorizer.source.contains(r#"@content-type="text/csv""#));
            assert!(colorizer
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
        }
    }

    #[test]
    fn catalog_exposes_markdown_output_artifact_sources() {
        let formatter = builtin_schema_package_artifact_source(
            "markdown",
            "schema-packages/markdown/v1/formatters/markdown-format-document.cemt",
        )
        .expect("Markdown formatter source");
        let colorizer = builtin_schema_package_artifact_source(
            "markdown",
            "schema-packages/markdown/v1/colorizers/markdown-color-document.cemt",
        )
        .expect("Markdown colorizer source");

        assert!(formatter
            .source
            .contains(r#"@name="markdown.format-document""#));
        assert!(formatter
            .source
            .contains(r#"@category="markdown-document""#));
        assert!(formatter.source.contains(r#"@subject="markdown-document""#));
        assert!(formatter.source.contains(r#"@produces="cem-tree""#));
        assert!(colorizer
            .source
            .contains(r#"@name="markdown.color-document""#));
        assert!(colorizer.source.contains(r#"@subject="cem-tree""#));
        assert!(colorizer.source.contains(r#"@produces="cem-tree""#));
        assert!(colorizer
            .source
            .contains(r#"@content-type="text/markdown""#));
        assert!(!formatter.source.contains(r#"@subject="json""#));
        assert!(!formatter.source.contains(r#"@produces="tokens""#));
        assert!(!formatter.source.contains(r#"@type="json""#));
        assert!(!colorizer.source.contains(r#"@subject="tokens""#));
        assert!(!colorizer.source.contains(r#"@produces="tokens""#));
    }

    #[test]
    fn catalog_exposes_css_output_artifact_sources() {
        for (path, function, profile) in [
            ("formatters/compact.cemt", "css.format-document", "compact"),
            ("formatters/pretty.cemt", "css.format-document", "pretty"),
            ("formatters/tabular.cemt", "css.format-document", "tabular"),
            ("colorizers/terminal.cemt", "css.color-document", "terminal"),
            ("colorizers/html.cemt", "css.color-document", "html"),
            ("colorizers/md.cemt", "css.color-document", "md"),
        ] {
            let full_path = format!("schema-packages/css/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("css", &full_path)
                .unwrap_or_else(|| panic!("CSS artifact source `{full_path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
            assert!(artifact.source.contains(r#"@produces="cem-tree""#));
        }
        for path in [
            "schema-packages/css/v1/formatters/css-format-document.cemt",
            "schema-packages/css/v1/colorizers/css-color-document.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("css", path)
                .unwrap_or_else(|| panic!("CSS helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }

    #[test]
    fn catalog_exposes_scss_source_output_artifact_sources() {
        for (path, function, profile) in [
            ("formatters/compact.cemt", "scss.format-source", "compact"),
            ("formatters/pretty.cemt", "scss.format-source", "pretty"),
            ("formatters/tabular.cemt", "scss.format-source", "tabular"),
            ("colorizers/terminal.cemt", "scss.color-source", "terminal"),
            ("colorizers/html.cemt", "scss.color-source", "html"),
            ("colorizers/md.cemt", "scss.color-source", "md"),
        ] {
            let full_path = format!("schema-packages/scss/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("scss", &full_path)
                .unwrap_or_else(|| panic!("SCSS artifact source `{full_path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
            assert!(artifact.source.contains(r#"@produces="cem-tree""#));
            assert!(artifact
                .source
                .contains(r#"@content-type="text/vnd.cem.scss""#));
            assert!(artifact
                .source
                .contains(r#"@schema="https://cem.dev/ns/data/scss/1""#));
            assert!(!artifact.source.contains(r#"@content-type="text/css""#));
        }
        for path in [
            "schema-packages/scss/v1/formatters/scss-format-source.cemt",
            "schema-packages/scss/v1/colorizers/scss-color-source.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("scss", path)
                .unwrap_or_else(|| panic!("SCSS helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
            assert!(helper.source.contains("text/vnd.cem.scss"));
            assert!(helper.source.contains("https://cem.dev/ns/data/scss/1"));
            assert!(!helper.source.contains(r#"@content-type="text/css""#));
        }
    }

    #[test]
    fn catalog_exposes_css_selector_output_artifact_sources() {
        for (path, function, profile) in [
            (
                "formatters/compact.cemt",
                "css-selector.format-expression",
                "compact",
            ),
            (
                "formatters/pretty.cemt",
                "css-selector.format-expression",
                "pretty",
            ),
            (
                "formatters/tabular.cemt",
                "css-selector.format-expression",
                "tabular",
            ),
            (
                "colorizers/terminal.cemt",
                "css-selector.color-expression",
                "terminal",
            ),
            (
                "colorizers/html.cemt",
                "css-selector.color-expression",
                "html",
            ),
            ("colorizers/md.cemt", "css-selector.color-expression", "md"),
        ] {
            let full_path = format!("schema-packages/css-selector/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("css-selector", &full_path)
                .unwrap_or_else(|| panic!("CSS selector artifact source `{full_path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
            assert!(artifact.source.contains(r#"@produces="cem-tree""#));
            assert!(artifact
                .source
                .contains(r#"@content-type="application/vnd.cem.query-expression+css-selector""#));
            assert!(artifact
                .source
                .contains(r#"@schema="https://cem.dev/ns/query/css-selector/1""#));
            assert!(!artifact.source.contains(r#"@content-type="text/css""#));
        }
        for path in [
            "schema-packages/css-selector/v1/formatters/css-selector-format-expression.cemt",
            "schema-packages/css-selector/v1/colorizers/css-selector-color-expression.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("css-selector", path)
                .unwrap_or_else(|| panic!("CSS selector helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
            assert!(helper
                .source
                .contains("application/vnd.cem.query-expression+css-selector"));
            assert!(helper
                .source
                .contains("https://cem.dev/ns/query/css-selector/1"));
            assert!(!helper.source.contains(r#"@content-type="text/css""#));
        }
    }

    #[test]
    fn catalog_exposes_html_output_artifact_sources() {
        for (path, function, profile) in [
            ("formatters/compact.cemt", "html.format-document", "compact"),
            ("formatters/pretty.cemt", "html.format-document", "pretty"),
            ("formatters/tabular.cemt", "html.format-document", "tabular"),
            (
                "colorizers/terminal.cemt",
                "html.color-document",
                "terminal",
            ),
            ("colorizers/html.cemt", "html.color-document", "html"),
            ("colorizers/md.cemt", "html.color-document", "md"),
        ] {
            let full_path = format!("schema-packages/html/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("html", &full_path)
                .unwrap_or_else(|| panic!("HTML artifact source `{full_path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
        }
        for path in [
            "schema-packages/html/v1/formatters/html-format-document.cemt",
            "schema-packages/html/v1/colorizers/html-color-document.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("html", path)
                .unwrap_or_else(|| panic!("HTML helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }

    #[test]
    fn catalog_exposes_xml_output_artifact_sources() {
        for (path, profile) in [
            ("formatters/compact.cemt", "compact"),
            ("formatters/pretty.cemt", "xml.pretty"),
            ("formatters/tabular.cemt", "tabular"),
        ] {
            let formatter = builtin_schema_package_artifact_source(
                "xml",
                &format!("schema-packages/xml/v1/{path}"),
            )
            .unwrap_or_else(|| panic!("XML formatter source `{path}`"));
            assert!(formatter.source.contains(r#"@name="xml.format-document""#));
            assert!(formatter.source.contains(r#"@category="xml-document""#));
            assert!(formatter
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(formatter.source.contains("{body |"));
        }
        for (path, profile) in [
            ("colorizers/terminal.cemt", "terminal"),
            ("colorizers/html.cemt", "html"),
            ("colorizers/md.cemt", "md"),
        ] {
            let colorizer = builtin_schema_package_artifact_source(
                "xml",
                &format!("schema-packages/xml/v1/{path}"),
            )
            .unwrap_or_else(|| panic!("XML colorizer source `{path}`"));
            assert!(colorizer.source.contains(r#"@name="xml.color-document""#));
            assert!(colorizer
                .source
                .contains(r#"@content-type="application/xml""#));
            assert!(colorizer
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(colorizer.source.contains("{body |"));
        }

        for path in [
            "schema-packages/xml/v1/formatters/xml-format-document.cemt",
            "schema-packages/xml/v1/colorizers/xml-color-document.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("xml", path)
                .unwrap_or_else(|| panic!("XML helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }

    #[test]
    fn catalog_exposes_relax_ng_output_artifact_sources() {
        for (path, function, profile) in [
            (
                "formatters/xml-compact.cemt",
                "relax-ng.format-xml-document",
                "compact",
            ),
            (
                "formatters/xml-pretty.cemt",
                "relax-ng.format-xml-document",
                "pretty",
            ),
            (
                "formatters/xml-tabular.cemt",
                "relax-ng.format-xml-document",
                "tabular",
            ),
            (
                "formatters/compact-compact.cemt",
                "relax-ng.format-compact-document",
                "compact",
            ),
            (
                "formatters/compact-pretty.cemt",
                "relax-ng.format-compact-document",
                "pretty",
            ),
            (
                "formatters/compact-tabular.cemt",
                "relax-ng.format-compact-document",
                "tabular",
            ),
            (
                "colorizers/xml-terminal.cemt",
                "relax-ng.color-xml-document",
                "terminal",
            ),
            (
                "colorizers/xml-html.cemt",
                "relax-ng.color-xml-document",
                "html",
            ),
            (
                "colorizers/xml-md.cemt",
                "relax-ng.color-xml-document",
                "md",
            ),
            (
                "colorizers/compact-terminal.cemt",
                "relax-ng.color-compact-document",
                "terminal",
            ),
            (
                "colorizers/compact-html.cemt",
                "relax-ng.color-compact-document",
                "html",
            ),
            (
                "colorizers/compact-md.cemt",
                "relax-ng.color-compact-document",
                "md",
            ),
        ] {
            let full_path = format!("schema-packages/relax-ng/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("relax-ng", &full_path)
                .unwrap_or_else(|| panic!("RELAX NG artifact source `{full_path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
        }
        for path in [
            "schema-packages/relax-ng/v1/formatters/relax-ng-format-schema.cemt",
            "schema-packages/relax-ng/v1/colorizers/relax-ng-color-schema.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("relax-ng", path)
                .unwrap_or_else(|| panic!("RELAX NG helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }

    #[test]
    fn catalog_exposes_xhtml_output_artifact_sources() {
        for (path, function, profile) in [
            (
                "formatters/compact.cemt",
                "xhtml.format-document",
                "compact",
            ),
            (
                "formatters/pretty.cemt",
                "xhtml.format-document",
                "xml.pretty",
            ),
            (
                "formatters/tabular.cemt",
                "xhtml.format-document",
                "tabular",
            ),
            (
                "colorizers/terminal.cemt",
                "xhtml.color-document",
                "terminal",
            ),
            ("colorizers/html.cemt", "xhtml.color-document", "html"),
            ("colorizers/md.cemt", "xhtml.color-document", "md"),
        ] {
            let full_path = format!("schema-packages/xhtml/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("xhtml", &full_path)
                .unwrap_or_else(|| panic!("XHTML artifact source `{full_path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
        }
        for path in [
            "schema-packages/xhtml/v1/formatters/xhtml-format-document.cemt",
            "schema-packages/xhtml/v1/colorizers/xhtml-color-document.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("xhtml", path)
                .unwrap_or_else(|| panic!("XHTML helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }

    #[test]
    fn catalog_exposes_svg_output_artifact_sources() {
        for (path, function, profile) in [
            ("formatters/compact.cemt", "svg.format-document", "compact"),
            (
                "formatters/pretty.cemt",
                "svg.format-document",
                "xml.pretty",
            ),
            ("formatters/tabular.cemt", "svg.format-document", "tabular"),
            ("colorizers/terminal.cemt", "svg.color-document", "terminal"),
            ("colorizers/html.cemt", "svg.color-document", "html"),
            ("colorizers/md.cemt", "svg.color-document", "md"),
        ] {
            let full_path = format!("schema-packages/svg/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("svg", &full_path)
                .unwrap_or_else(|| panic!("SVG artifact source `{full_path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
        }
        for path in [
            "schema-packages/svg/v1/formatters/svg-format-document.cemt",
            "schema-packages/svg/v1/colorizers/svg-color-document.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("svg", path)
                .unwrap_or_else(|| panic!("SVG helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }

    #[test]
    fn catalog_exposes_mathml_output_artifact_sources() {
        for (path, function, profile) in [
            (
                "formatters/compact.cemt",
                "mathml.format-document",
                "compact",
            ),
            (
                "formatters/pretty.cemt",
                "mathml.format-document",
                "xml.pretty",
            ),
            (
                "formatters/tabular.cemt",
                "mathml.format-document",
                "tabular",
            ),
            (
                "colorizers/terminal.cemt",
                "mathml.color-document",
                "terminal",
            ),
            ("colorizers/html.cemt", "mathml.color-document", "html"),
            ("colorizers/md.cemt", "mathml.color-document", "md"),
        ] {
            let full_path = format!("schema-packages/mathml/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("mathml", &full_path)
                .unwrap_or_else(|| panic!("MathML artifact source `{full_path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
        }
        for path in [
            "schema-packages/mathml/v1/formatters/mathml-format-document.cemt",
            "schema-packages/mathml/v1/colorizers/mathml-color-document.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("mathml", path)
                .unwrap_or_else(|| panic!("MathML helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }

    #[test]
    fn catalog_exposes_xpath_output_artifact_sources() {
        for (path, function, profile) in [
            (
                "formatters/compact.cemt",
                "xpath.format-expression",
                "compact",
            ),
            (
                "formatters/pretty.cemt",
                "xpath.format-expression",
                "pretty",
            ),
            (
                "formatters/tabular.cemt",
                "xpath.format-expression",
                "tabular",
            ),
            (
                "colorizers/terminal.cemt",
                "xpath.color-expression",
                "terminal",
            ),
            ("colorizers/html.cemt", "xpath.color-expression", "html"),
            ("colorizers/md.cemt", "xpath.color-expression", "md"),
        ] {
            let full_path = format!("schema-packages/xpath/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("xpath", &full_path)
                .unwrap_or_else(|| panic!("XPath artifact source `{path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
        }
        for path in [
            "schema-packages/xpath/v1/formatters/xpath-format-expression.cemt",
            "schema-packages/xpath/v1/colorizers/xpath-color-expression.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("xpath", path)
                .unwrap_or_else(|| panic!("XPath helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }

    #[test]
    fn catalog_exposes_xslt_output_artifact_sources() {
        for (path, function, profile) in [
            (
                "formatters/compact.cemt",
                "xslt.format-stylesheet",
                "compact",
            ),
            (
                "formatters/pretty.cemt",
                "xslt.format-stylesheet",
                "xml.pretty",
            ),
            (
                "formatters/tabular.cemt",
                "xslt.format-stylesheet",
                "tabular",
            ),
            (
                "colorizers/terminal.cemt",
                "xslt.color-stylesheet",
                "terminal",
            ),
            ("colorizers/html.cemt", "xslt.color-stylesheet", "html"),
            ("colorizers/md.cemt", "xslt.color-stylesheet", "md"),
        ] {
            let full_path = format!("schema-packages/xslt/v1/{path}");
            let artifact = builtin_schema_package_artifact_source("xslt", &full_path)
                .unwrap_or_else(|| panic!("XSLT artifact source `{path}`"));
            assert!(artifact.source.contains(&format!(r#"@name="{function}""#)));
            assert!(artifact
                .source
                .contains(&format!(r#"@profile="{profile}""#)));
            assert!(artifact.source.contains("{body |"));
        }
        for path in [
            "schema-packages/xslt/v1/formatters/xslt-format-stylesheet.cemt",
            "schema-packages/xslt/v1/colorizers/xslt-color-stylesheet.cemt",
        ] {
            let helper = builtin_schema_package_artifact_source("xslt", path)
                .unwrap_or_else(|| panic!("XSLT helper source `{path}`"));
            assert!(helper.source.contains("{body |"));
        }
    }
}
