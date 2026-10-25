//! RELAX NG parser facts and syntax metadata belong to the original retained
//! schema source. No optional embedded body is consulted during execution.
use super::*;
use crate::{
    engine::{EngineInput, FormatIdentity, InputFormat},
    lifecycle::{LoadedInput, LoadedInputAstStream},
    schema::{
        document_model::{SchemaDocumentModel, INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE},
        parser_contracts::NativeParserContract,
    },
};
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug)]
struct RelaxNgParserContract(RelaxNgSchemaContractCatalog);

impl NativeParserContract for RelaxNgParserContract {
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
            .unwrap_or(RELAX_NG_XML_CONTENT_TYPE);
        let (document, diagnostics) = relax_ng_document_ast_from_source_bytes_with_contracts(
            RelaxNgSourceValidationRequest {
                bytes: &input.bytes,
                source_uri: &input.uri,
                content_type: Some(content_type),
            },
            &self.0,
        );
        LoadedInput {
            bytes: input.bytes.clone(),
            from_format: document
                .as_ref()
                .map(|document| match document.syntax_kind {
                    RelaxNgSyntaxKind::Xml => InputFormat::Xml,
                    RelaxNgSyntaxKind::Compact => InputFormat::Cem,
                })
                .unwrap_or(InputFormat::Cem),
            ast_stream: document.map(LoadedInputAstStream::RelaxNgDocument),
            diagnostics,
            adapter_id: Some(adapter_id),
        }
    }
}

pub(crate) fn compile_parser_contract(
    model: &SchemaDocumentModel,
) -> (Option<Arc<dyn NativeParserContract>>, Vec<Diagnostic>) {
    use RelaxNgFactKind::*;
    // Only actual native facts may select diagnostics. Safety failures stay
    // hard; authored advisory severity, diagnostic names and policy survive.
    let facts = [
        (XmlParseError, "xml-syntax", true),
        (CompactParseError, "compact-syntax", true),
        (UnsupportedEncoding, "schema-resource", true),
        (EncodingConflict, "schema-resource", false),
        (InvalidNamespace, "xml-syntax", true),
        (InvalidRoot, "xml-syntax", true),
        (UnknownElement, "pattern", true),
        (MissingRequiredAttribute, "pattern", true),
        (MissingStart, "schema-resource", true),
        (IncludeRejected, "include", true),
        (ExternalReferenceRejected, "external-ref", true),
        (SourceMapUnavailable, "schema-resource", false),
    ];
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut invalid = |constraint: Option<&crate::schema::document_model::ConstraintDefinition>,
                       message: String| {
        diagnostics.push(Diagnostic {
            code: INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE.into(),
            severity: Severity::Error,
            message,
            source_map: constraint.map(|c| c.source_map.clone()),
            details: Some(
                json!({"schemaUri": model.schema_uri, "checkKind": "relax-ng-parser-fact-contract",
                "constraint": constraint.map(|c| &c.kind), "behavior": RELAX_NG_FACT_BEHAVIOR}),
            ),
            ..Default::default()
        });
    };
    if model.behaviors.contains_key(RELAX_NG_FACT_BEHAVIOR) {
        invalid(
            None,
            "The native RELAX NG parser-fact intrinsic cannot be replaced by a schema behavior"
                .into(),
        );
    }
    for constraint in model
        .constraints
        .values()
        .filter(|c| c.behavior.as_deref() == Some(RELAX_NG_FACT_BEHAVIOR))
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
                    "RELAX NG constraint `{}` requires a supported native fact-kind",
                    constraint.kind
                ),
            );
            continue;
        };
        if !seen.insert(fact.0.as_str()) {
            invalid(
                Some(constraint),
                format!(
                    "RELAX NG fact `{}` is bound more than once",
                    fact.0.as_str()
                ),
            );
            continue;
        }
        if constraint.target.as_deref() != Some(fact.1) {
            invalid(
                Some(constraint),
                format!(
                    "RELAX NG fact `{}` requires target `{}`",
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
                    "RELAX NG fact `{}` requires a declared diagnostic",
                    fact.0.as_str()
                ),
            );
            continue;
        };
        if fact.2 && !diagnostic.severity.is_hard_violation() {
            invalid(
                Some(constraint),
                format!(
                    "RELAX NG safety fact `{}` requires a hard diagnostic",
                    fact.0.as_str()
                ),
            );
        }
        if diagnostic.behavior.is_some() {
            invalid(
                Some(constraint),
                format!(
                    "RELAX NG fact diagnostic `{}` cannot select another execution behavior",
                    diagnostic.code
                ),
            );
        }
    }
    for (fact, _, _) in facts {
        if !seen.contains(fact.as_str()) {
            invalid(
                None,
                format!(
                    "Native RELAX NG validation requires a binding for fact `{}`",
                    fact.as_str()
                ),
            );
        }
    }
    let implementation = diagnostics.is_empty().then(|| {
        Arc::new(RelaxNgParserContract(
            RelaxNgSchemaContractCatalog::from_document_model(model),
        )) as Arc<dyn NativeParserContract>
    });
    (implementation, diagnostics)
}

#[cfg(test)]
mod tests;
