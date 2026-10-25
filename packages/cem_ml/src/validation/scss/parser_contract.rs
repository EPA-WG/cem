//! Retained SCSS source and expansion facts use the selected original schema.
use super::*;
use crate::{
    engine::{EngineContext, EngineInput, FormatIdentity},
    lifecycle::{load_scss_input_with_context, LoadedInput},
    schema::{
        document_model::{SchemaDocumentModel, INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE},
        parser_contracts::NativeParserContract,
    },
};
use std::{collections::BTreeSet, sync::Arc};

// Intrinsic codes identify facts already emitted by the shared SCSS parser and
// evaluator. Schema bindings choose their public diagnostic code and severity;
// the retained typed SCSS/CSS AST and source frames are never projected/reparsed.
const FACTS: &[(&str, &str, &str, bool)] = &[
    (
        "identity-mismatch",
        "source-resource",
        "cem.scss.identity_mismatch",
        true,
    ),
    (
        "unsupported-encoding",
        "source-resource",
        "cem.scss.unsupported_encoding",
        true,
    ),
    ("parse-error", "stylesheet", "cem.scss.parse_error", true),
    (
        "module-error",
        "module-graph",
        "cem.scss.module_error",
        true,
    ),
    (
        "import-deprecated",
        "import-rule",
        "cem.scss.import_deprecated",
        false,
    ),
    (
        "resolver-denied",
        "module-edge",
        "cem.scss.resolver_denied",
        true,
    ),
    (
        "module-cycle",
        "module-graph",
        "cem.scss.module_cycle",
        true,
    ),
    (
        "budget-exceeded",
        "evaluation-request",
        "cem.scss.budget_exceeded",
        true,
    ),
    (
        "cancelled",
        "evaluation-request",
        "cem.scss.cancelled",
        true,
    ),
    (
        "origin-unavailable",
        "generated-node",
        "cem.scss.origin_unavailable",
        true,
    ),
    (
        "handoff-invalid",
        "css-handoff",
        "cem.scss.handoff_invalid",
        true,
    ),
];

#[derive(Debug)]
struct ScssDiagnosticBinding {
    fact_kind: String,
    diagnostic_code: String,
    severity: Severity,
    constraint: String,
    policy: Option<String>,
}

#[derive(Debug)]
struct ScssParserContract(BTreeMap<String, ScssDiagnosticBinding>);

impl ScssParserContract {
    fn bind(&self, diagnostics: &mut Vec<Diagnostic>) {
        for diagnostic in diagnostics {
            if let Some(binding) = self.0.get(&diagnostic.code) {
                diagnostic.code.clone_from(&binding.diagnostic_code);
                diagnostic.severity = binding.severity;
                if let Some(details) = diagnostic
                    .details
                    .as_mut()
                    .and_then(serde_json::Value::as_object_mut)
                {
                    details.insert(
                        "nativeFact".into(),
                        serde_json::json!({
                            "factKind": binding.fact_kind, "constraint": binding.constraint,
                            "behavior": SCSS_FACT_BEHAVIOR, "policy": binding.policy,
                        }),
                    );
                }
            }
        }
    }
}

impl NativeParserContract for ScssParserContract {
    fn load(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        adapter_id: &'static str,
    ) -> LoadedInput {
        self.load_with_context(
            input,
            identity,
            adapter_id,
            &EngineContext::default(),
            false,
        )
    }

    fn load_with_context(
        &self,
        input: &EngineInput,
        identity: &FormatIdentity,
        _adapter_id: &'static str,
        context: &EngineContext,
        source_validation: bool,
    ) -> LoadedInput {
        load_scss_input_with_context(
            input,
            identity,
            context,
            source_validation,
            Some(&|diagnostics| self.bind(diagnostics)),
        )
    }
}

pub(crate) fn compile_parser_contract(
    model: &SchemaDocumentModel,
) -> (Option<Arc<dyn NativeParserContract>>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::new();
    let mut bindings = BTreeMap::new();
    let mut invalid = |constraint: Option<&crate::schema::document_model::ConstraintDefinition>,
                       message: String| {
        diagnostics.push(Diagnostic {
            code: INVALID_DIAGNOSTIC_BEHAVIOR_CONTRACT_CODE.into(),
            severity: Severity::Error,
            message,
            source_map: constraint.map(|c| c.source_map.clone()),
            details: Some(serde_json::json!({"schemaUri": model.schema_uri,
                "checkKind": "scss-native-fact-contract", "constraint": constraint.map(|c| &c.kind),
                "behavior": SCSS_FACT_BEHAVIOR})),
            ..Default::default()
        });
    };
    if model.behaviors.contains_key(SCSS_FACT_BEHAVIOR) {
        invalid(
            None,
            "The native SCSS fact intrinsic cannot be replaced by a schema behavior".into(),
        );
    }
    for constraint in model
        .constraints
        .values()
        .filter(|c| c.behavior.as_deref() == Some(SCSS_FACT_BEHAVIOR))
    {
        let Some(fact) = constraint
            .fact_kind
            .as_deref()
            .map(str::trim)
            .and_then(|name| FACTS.iter().find(|(kind, _, _, _)| *kind == name))
        else {
            invalid(
                Some(constraint),
                format!(
                    "SCSS constraint `{}` requires a supported native fact-kind",
                    constraint.kind
                ),
            );
            continue;
        };
        if !seen.insert(fact.0) {
            invalid(
                Some(constraint),
                format!("SCSS fact `{}` is bound more than once", fact.0),
            );
        }
        if constraint.target.as_deref() != Some(fact.1) {
            invalid(
                Some(constraint),
                format!("SCSS fact `{}` requires target `{}`", fact.0, fact.1),
            );
        }
        let Some(diagnostic) = constraint
            .diagnostic
            .as_deref()
            .and_then(|code| model.diagnostics.get(code))
        else {
            invalid(
                Some(constraint),
                format!("SCSS fact `{}` requires a declared diagnostic", fact.0),
            );
            continue;
        };
        if diagnostic.behavior.is_some() || fact.3 && !diagnostic.severity.is_hard_violation() {
            invalid(Some(constraint), format!("SCSS fact `{}` requires its declared native diagnostic without replacement behavior{}", fact.0,
                if fact.3 { " and with hard severity" } else { "" }));
        }
        bindings.insert(
            fact.2.into(),
            ScssDiagnosticBinding {
                fact_kind: fact.0.into(),
                diagnostic_code: diagnostic.code.clone(),
                severity: diagnostic.severity,
                constraint: constraint.kind.clone(),
                policy: constraint.policy.clone(),
            },
        );
    }
    for (fact, _, _, _) in FACTS {
        if !seen.contains(fact) {
            invalid(
                None,
                format!("Native SCSS validation requires a binding for fact `{fact}`"),
            );
        }
    }
    let implementation = diagnostics
        .is_empty()
        .then(|| Arc::new(ScssParserContract(bindings)) as Arc<dyn NativeParserContract>);
    (implementation, diagnostics)
}
