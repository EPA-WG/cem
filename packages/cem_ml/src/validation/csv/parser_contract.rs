//! CSV owns parser-fact semantics. The structural schema compiler supplies
//! native declarations; this stage never evaluates or reparses external data.
use super::*;
use crate::{
    engine::{EngineInput, FormatIdentity, InputFormat},
    lifecycle::{csv_lifecycle_adapter_diagnostics, LoadedInput, LoadedInputAstStream},
    schema::{
        document_model::{SchemaDocumentModel, INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE},
        parser_contracts::NativeParserContract,
    },
};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug)]
struct CsvParserContract(CsvSchemaContractCatalog);

impl NativeParserContract for CsvParserContract {
    fn load(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        adapter_id: &'static str,
    ) -> LoadedInput {
        let content_type = identity
            .content_type
            .as_deref()
            .or(input.root_scope.default_content_type.as_deref())
            .unwrap_or(CSV_CONTENT_TYPE);
        let (table, diagnostics) = csv_document_ast_from_source_bytes_with_contracts(
            CsvSourceValidationRequest {
                bytes: &input.bytes,
                source_uri: &input.uri,
                content_type: Some(content_type),
            },
            &self.0,
        );
        LoadedInput {
            bytes: input.bytes.clone(),
            from_format: input.from_format.unwrap_or(InputFormat::Cem),
            ast_stream: table.map(LoadedInputAstStream::CsvDocument),
            diagnostics: csv_lifecycle_adapter_diagnostics(adapter_id, diagnostics),
            adapter_id: Some(adapter_id),
        }
    }
}

pub(crate) fn compile_parser_contract(
    model: &SchemaDocumentModel,
) -> (Option<Arc<dyn NativeParserContract>>, Vec<Diagnostic>) {
    use CsvParseFactKind::*;
    // These are actual facts emitted by the registered CSV decoder/parser.
    // Syntax/encoding failures stay hard; schema-owned advisory facts can be
    // upgraded, while diagnostic names, contract names and policy stay authored.
    let facts = [
        (ParseError, "table", true),
        (UnsupportedCharset, "table", true),
        (UnsupportedEncoding, "table", true),
        (DeclaredUsAsciiNonAsciiByte, "table", true),
        (InvalidHeaderParameter, "table", false),
        (InvalidQuoteEscape, "field", true),
        (UnclosedQuote, "field", true),
        (RaggedRow, "table", false),
    ];
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut contracts = CsvSchemaContractCatalog {
        fact_bindings: BTreeMap::new(),
    };
    let mut invalid = |constraint: Option<&crate::schema::document_model::ConstraintDefinition>,
                       message: String| {
        diagnostics.push(Diagnostic {
            code: INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE.into(),
            severity: Severity::Error,
            message,
            source_map: constraint.map(|c| c.source_map.clone()),
            details: Some(
                json!({"schemaUri": model.schema_uri, "checkKind": "csv-parser-fact-contract",
                "constraint": constraint.map(|c| &c.kind), "behavior": CSV_PARSE_REPORT_BEHAVIOR}),
            ),
            ..Default::default()
        });
    };
    if model.behaviors.contains_key(CSV_PARSE_REPORT_BEHAVIOR) {
        invalid(
            None,
            "The native CSV parser-fact intrinsic cannot be replaced by a schema behavior".into(),
        );
    }
    for constraint in model
        .constraints
        .values()
        .filter(|c| c.behavior.as_deref() == Some(CSV_PARSE_REPORT_BEHAVIOR))
    {
        let Some(fact) = constraint
            .fact_kind
            .as_deref()
            .map(str::trim)
            .and_then(|name| facts.iter().find(|(kind, _, _)| kind.as_str() == name))
        else {
            invalid(
                Some(constraint),
                format!(
                    "CSV constraint `{}` requires a supported native fact-kind",
                    constraint.kind
                ),
            );
            continue;
        };
        if !seen.insert(fact.0.as_str()) {
            invalid(
                Some(constraint),
                format!("CSV fact `{}` is bound more than once", fact.0.as_str()),
            );
            continue;
        }
        if constraint.target.as_deref() != Some(fact.1) {
            invalid(
                Some(constraint),
                format!(
                    "CSV fact `{}` requires target `{}`",
                    fact.0.as_str(),
                    fact.1
                ),
            );
        }
        let Some(diagnostic) = constraint
            .diagnostic
            .as_deref()
            .and_then(|code| model.diagnostics.get(code))
        else {
            invalid(
                Some(constraint),
                format!(
                    "CSV fact `{}` requires a declared diagnostic",
                    fact.0.as_str()
                ),
            );
            continue;
        };
        if fact.2 && !diagnostic.severity.is_hard_violation() {
            invalid(
                Some(constraint),
                format!(
                    "CSV parser/encoding fact `{}` requires a hard diagnostic",
                    fact.0.as_str()
                ),
            );
        }
        if diagnostic.behavior.is_some() {
            invalid(
                Some(constraint),
                format!(
                    "CSV fact diagnostic `{}` cannot select another execution behavior",
                    diagnostic.code
                ),
            );
        }
        contracts.fact_bindings.insert(
            fact.0.as_str().into(),
            CsvDiagnosticBinding {
                fact_kind: fact.0.as_str().into(),
                contract: constraint.kind.clone(),
                behavior: constraint.behavior.clone(),
                diagnostic_code: diagnostic.code.clone(),
                severity: diagnostic.severity,
                policy: constraint.policy.clone(),
            },
        );
    }
    for (fact, _, _) in facts {
        if !seen.contains(fact.as_str()) {
            invalid(
                None,
                format!(
                    "Native CSV validation requires a binding for fact `{}`",
                    fact.as_str()
                ),
            );
        }
    }
    let implementation = diagnostics
        .is_empty()
        .then(|| Arc::new(CsvParserContract(contracts)) as Arc<dyn NativeParserContract>);
    (implementation, diagnostics)
}
