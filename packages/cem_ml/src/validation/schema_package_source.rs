use super::{csv, xpath};
use crate::diagnostics::Diagnostic;
use crate::schema::package_loader::load_builtin_schema_package;
use crate::schema::registry::{SchemaDescriptor, SchemaRegistry};
use crate::source::SourceId;

#[derive(Debug, Clone, Copy)]
pub struct SchemaPackageSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
    pub schema_uri: Option<&'a str>,
    pub schema_registry: &'a SchemaRegistry,
}

#[derive(Debug, Clone)]
pub struct SchemaPackageSourceValidationReport {
    pub package_id: String,
    pub schema_uri: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn validate_schema_package_source(
    request: SchemaPackageSourceValidationRequest<'_>,
) -> Option<SchemaPackageSourceValidationReport> {
    let descriptor = resolve_schema_package_source_descriptor(
        request.schema_registry,
        request.content_type,
        request.schema_uri,
    )?;

    match descriptor.package_id.as_str() {
        "csv" => validate_csv_schema_package_source(request, descriptor),
        "xpath" => validate_xpath_schema_package_source(request, descriptor),
        _ => None,
    }
}

fn validate_xpath_schema_package_source(
    request: SchemaPackageSourceValidationRequest<'_>,
    descriptor: &SchemaDescriptor,
) -> Option<SchemaPackageSourceValidationReport> {
    if request.content_type.is_some_and(|content_type| {
        !matches!(
            crate::schema::registry::content_type_essence(content_type).as_str(),
            xpath::XPATH_CONTENT_TYPE | "text/xpath"
        )
    }) {
        return None;
    }
    let package = load_builtin_schema_package(&descriptor.schema_uri).ok()?;
    let contracts = xpath::XPathSchemaContractCatalog::from_schema_source(package.schema_source);
    let ast = xpath::xpath_expression_ast_from_source_bytes(
        xpath::XPathSourceRequest {
            bytes: request.bytes,
            source_uri: request.source_uri,
            content_type: request.content_type,
            source_range_projector: None,
        },
        xpath::XPathAttachment::Standalone {
            source_id: SourceId(1).0,
        },
    );

    Some(SchemaPackageSourceValidationReport {
        package_id: descriptor.package_id.clone(),
        schema_uri: descriptor.schema_uri.clone(),
        diagnostics: xpath::validate_xpath_expression_ast(&ast, &contracts),
    })
}

fn validate_csv_schema_package_source(
    request: SchemaPackageSourceValidationRequest<'_>,
    descriptor: &SchemaDescriptor,
) -> Option<SchemaPackageSourceValidationReport> {
    let package = load_builtin_schema_package(&descriptor.schema_uri).ok()?;
    let contracts = csv::CsvSchemaContractCatalog::from_schema_source(package.schema_source);
    let parse_report = csv::extract_csv_parse_report(csv::CsvSourceValidationRequest {
        bytes: request.bytes,
        source_uri: request.source_uri,
        content_type: request.content_type,
    });

    Some(SchemaPackageSourceValidationReport {
        package_id: descriptor.package_id.clone(),
        schema_uri: descriptor.schema_uri.clone(),
        diagnostics: csv::validate_csv_parse_report(&parse_report, &contracts),
    })
}

fn resolve_schema_package_source_descriptor<'a>(
    registry: &'a SchemaRegistry,
    content_type: Option<&str>,
    schema_uri: Option<&str>,
) -> Option<&'a SchemaDescriptor> {
    let schema_uri = schema_uri
        .map(str::trim)
        .filter(|schema| !schema.is_empty());
    match (schema_uri, content_type) {
        (Some(schema_uri), Some(content_type)) => {
            let descriptor = registry.resolve_content_type(content_type).ok()?;
            (descriptor.schema_uri == schema_uri).then_some(descriptor)
        }
        (Some(schema_uri), None) => registry.schema(schema_uri),
        (None, Some(content_type)) => registry.resolve_content_type(content_type).ok(),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::registry::{
        SchemaRegistry, CSV_SCHEMA_URI, JSON_VALUE_SCHEMA_URI, XPATH_RESULT_CONTENT_TYPE,
        XPATH_SCHEMA_URI,
    };

    #[test]
    fn csv_source_validation_enters_through_schema_package_content_type() {
        let registry = SchemaRegistry::with_builtin_schemas();
        let report = validate_schema_package_source(SchemaPackageSourceValidationRequest {
            bytes: b"id,name\n1,Ada\n",
            source_uri: "memory://table.csv",
            content_type: Some("text/csv; header=maybe"),
            schema_uri: None,
            schema_registry: &registry,
        })
        .expect("CSV content type has a schema-package source validator");

        assert_eq!(report.package_id, "csv");
        assert_eq!(report.schema_uri, CSV_SCHEMA_URI);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(
            report.diagnostics[0].code,
            "cem.csv.invalid_header_parameter"
        );
        assert_eq!(
            report.diagnostics[0].details.as_ref().unwrap()["contract"],
            "header-parameter-values"
        );
        assert_eq!(
            report.diagnostics[0].details.as_ref().unwrap()["factKind"],
            "invalid-header-parameter"
        );
    }

    #[test]
    fn csv_source_validation_enters_through_schema_uri_without_cem_ast() {
        let registry = SchemaRegistry::with_builtin_schemas();
        let report = validate_schema_package_source(SchemaPackageSourceValidationRequest {
            bytes: b"id,name\n1,\"Ada\n",
            source_uri: "memory://table.csv",
            content_type: None,
            schema_uri: Some(CSV_SCHEMA_URI),
            schema_registry: &registry,
        })
        .expect("CSV schema URI has a schema-package source validator");

        assert_eq!(report.package_id, "csv");
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.csv.unclosed_quote"));
    }

    #[test]
    fn explicit_schema_mismatch_does_not_enter_source_validator() {
        let registry = SchemaRegistry::with_builtin_schemas();
        let report = validate_schema_package_source(SchemaPackageSourceValidationRequest {
            bytes: b"id,name\n1,Ada\n",
            source_uri: "memory://table.csv",
            content_type: Some("text/csv"),
            schema_uri: Some(JSON_VALUE_SCHEMA_URI),
            schema_registry: &registry,
        });

        assert!(report.is_none());
    }

    #[test]
    fn xpath_source_validation_enters_through_primary_and_alias_content_types() {
        let registry = SchemaRegistry::with_builtin_schemas();
        for content_type in ["application/vnd.cem.xpath", "text/xpath; charset=utf-8"] {
            let report = validate_schema_package_source(SchemaPackageSourceValidationRequest {
                bytes: b"/catalog/ns:book",
                source_uri: "memory://unknown-prefix.xpath",
                content_type: Some(content_type),
                schema_uri: None,
                schema_registry: &registry,
            })
            .expect("XPath content type has a schema-package source validator");

            assert_eq!(report.package_id, "xpath");
            assert_eq!(report.schema_uri, XPATH_SCHEMA_URI);
            let diagnostic = report
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == "cem.xpath.unknown_namespace_prefix")
                .expect("unknown namespace diagnostic");
            assert_eq!(diagnostic.byte_offset, Some(9));
            assert!(diagnostic.source_map.is_some());
            assert_eq!(
                diagnostic.details.as_ref().unwrap()["xpath"]["behavior"],
                "xpath-report-fact"
            );
        }
    }

    #[test]
    fn xpath_source_validation_skips_result_content_type() {
        let registry = SchemaRegistry::with_builtin_schemas();
        let descriptor = registry
            .resolve_content_type(XPATH_RESULT_CONTENT_TYPE)
            .expect("XPath result content type is package-owned");
        assert_eq!(descriptor.package_id, "xpath");
        assert_eq!(descriptor.schema_uri, XPATH_SCHEMA_URI);
        let report = validate_schema_package_source(SchemaPackageSourceValidationRequest {
            bytes: br#"{"contentType":"application/vnd.cem.xpath-result+json"}"#,
            source_uri: "memory://result.xpath.json",
            content_type: Some(XPATH_RESULT_CONTENT_TYPE),
            schema_uri: Some(XPATH_SCHEMA_URI),
            schema_registry: &registry,
        });

        assert!(report.is_none());
    }

    #[test]
    fn xpath_source_validation_enters_through_schema_uri() {
        let registry = SchemaRegistry::with_builtin_schemas();
        let report = validate_schema_package_source(SchemaPackageSourceValidationRequest {
            bytes: b"/catalog/`book",
            source_uri: "memory://invalid-token.xpath",
            content_type: None,
            schema_uri: Some(XPATH_SCHEMA_URI),
            schema_registry: &registry,
        })
        .expect("XPath schema URI has a schema-package source validator");

        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.xpath.lexical_error"));
    }
}
