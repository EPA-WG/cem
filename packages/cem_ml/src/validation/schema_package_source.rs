use super::csv;
use crate::diagnostics::Diagnostic;
use crate::schema::package_loader::load_builtin_schema_package;
use crate::schema::registry::{SchemaDescriptor, SchemaRegistry};

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
        _ => None,
    }
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
    use crate::schema::registry::{SchemaRegistry, CSV_SCHEMA_URI, JSON_VALUE_SCHEMA_URI};

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
}
