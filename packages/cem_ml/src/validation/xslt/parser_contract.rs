//! XSLT parser facts and syntax metadata belong to the original retained
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
struct XsltParserContract(XsltSchemaContractCatalog);

impl NativeParserContract for XsltParserContract {
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
            .unwrap_or(XSLT_CONTENT_TYPE);
        let (document, diagnostics) =
            xslt_stylesheet_ast_from_source_bytes_with_modules_and_contracts(
                XsltSourceValidationRequest {
                    bytes: &input.bytes,
                    source_uri: &input.uri,
                    content_type: Some(content_type),
                },
                &[],
                &self.0,
            );
        LoadedInput {
            bytes: input.bytes.clone(),
            from_format: InputFormat::Xml,
            ast_stream: document.map(LoadedInputAstStream::XsltStylesheet),
            diagnostics,
            adapter_id: Some(adapter_id),
        }
    }
}

pub(crate) fn compile_parser_contract(
    model: &SchemaDocumentModel,
) -> (Option<Arc<dyn NativeParserContract>>, Vec<Diagnostic>) {
    use XsltFactKind::*;
    // Only actual native facts may select diagnostics. Safety failures stay
    // hard; authored advisory severity, diagnostic names and policy survive.
    let facts = [
        (NotWellFormedXml, "stylesheet-module", true),
        (UnsupportedEncoding, "stylesheet-module", true),
        (EncodingConflict, "stylesheet-module", true),
        (UnboundNamespacePrefix, "stylesheet-module", true),
        (DuplicateAttribute, "stylesheet-module", true),
        (DtdRejected, "stylesheet-module", true),
        (ExternalEntityRejected, "stylesheet-module", true),
        (EntityExpansionLimit, "stylesheet-module", true),
        (SourceMapUnavailable, "instruction", false),
        (RootNotStylesheet, "stylesheet", true),
        (NamespaceMissing, "stylesheet", true),
        (VersionMissing, "stylesheet", true),
        (VersionMalformed, "stylesheet", true),
        (UnsupportedVersion, "stylesheet", true),
        (EntryPointMissing, "template", true),
        (ExternalUriRejected, "declaration", true),
        (UnsupportedConstruct, "instruction", false),
        (AvtUnclosedExpression, "attribute-value-template", true),
        (AvtUnescapedRightBrace, "attribute-value-template", true),
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
                json!({"schemaUri": model.schema_uri, "checkKind": "xslt-parser-fact-contract",
                "constraint": constraint.map(|c| &c.kind), "behavior": XSLT_FACT_BEHAVIOR}),
            ),
            ..Default::default()
        });
    };
    if model.behaviors.contains_key(XSLT_FACT_BEHAVIOR) {
        invalid(
            None,
            "The native XSLT parser-fact intrinsic cannot be replaced by a schema behavior".into(),
        );
    }
    for constraint in model
        .constraints
        .values()
        .filter(|c| c.behavior.as_deref() == Some(XSLT_FACT_BEHAVIOR))
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
                    "XSLT constraint `{}` requires a supported native fact-kind",
                    constraint.kind
                ),
            );
            continue;
        };
        if !seen.insert(fact.0.as_str()) {
            invalid(
                Some(constraint),
                format!("XSLT fact `{}` is bound more than once", fact.0.as_str()),
            );
            continue;
        }
        if constraint.target.as_deref() != Some(fact.1) {
            invalid(
                Some(constraint),
                format!(
                    "XSLT fact `{}` requires target `{}`",
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
                    "XSLT fact `{}` requires a declared diagnostic",
                    fact.0.as_str()
                ),
            );
            continue;
        };
        if fact.2 && !diagnostic.severity.is_hard_violation() {
            invalid(
                Some(constraint),
                format!(
                    "XSLT safety fact `{}` requires a hard diagnostic",
                    fact.0.as_str()
                ),
            );
        }
        if diagnostic.behavior.is_some() {
            invalid(
                Some(constraint),
                format!(
                    "XSLT fact diagnostic `{}` cannot select another execution behavior",
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
                    "Native XSLT validation requires a binding for fact `{}`",
                    fact.as_str()
                ),
            );
        }
    }
    for kind in [
        XSLT_ATTRIBUTE_VALUE_DEFAULT_GRAMMAR,
        XSLT_XPATH_EXPRESSION_ATTRIBUTES,
        XSLT_PATTERN_ATTRIBUTES,
        XSLT_AVT_ATTRIBUTES,
    ] {
        let Some(constraint) = model.constraint(kind) else {
            invalid(
                None,
                format!("Native XSLT parsing requires grammar declaration `{kind}`"),
            );
            continue;
        };
        if constraint.target.as_deref() != Some("attribute-value") || constraint.value.is_none() {
            invalid(
                Some(constraint),
                format!(
                    "XSLT grammar `{kind}` requires attribute-value target and a declared value"
                ),
            );
            continue;
        }
        let value = constraint.value.as_deref().unwrap();
        if kind == XSLT_ATTRIBUTE_VALUE_DEFAULT_GRAMMAR {
            if XsltAttributeValueGrammar::from_schema_value(value).is_none() {
                invalid(
                    Some(constraint),
                    "XSLT default attribute grammar is not registered".into(),
                );
            }
        } else if value.split_ascii_whitespace().any(|selector| {
            XsltAttributeValueGrammarRule::from_schema_selector(
                selector,
                XsltAttributeValueGrammar::Literal,
            )
            .is_none()
        }) {
            invalid(
                Some(constraint),
                format!("XSLT grammar `{kind}` contains an invalid selector"),
            );
        }
    }
    let implementation = diagnostics.is_empty().then(|| {
        Arc::new(XsltParserContract(
            XsltSchemaContractCatalog::from_document_model(model),
        )) as Arc<dyn NativeParserContract>
    });
    (implementation, diagnostics)
}

#[cfg(test)]
mod tests;
