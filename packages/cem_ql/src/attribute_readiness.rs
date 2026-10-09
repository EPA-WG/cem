//! Declaration-time prerequisites for explicit attribute consumers. A ready
//! report does not activate a schema or replace package lifecycle checks.
use crate::{
    attribute_validation::{AttributeValidation, AttributeValidationLimits},
    datatype_facets::BoundAttributeFacets,
    datatype_preparation::PreparationInput,
    datatype_results::DiagnosticAttribution,
    datatype_validation::ValidationRuntime,
    eval::RetainedCemNode,
};
use cem_ml::{
    operation_control::ControlError,
    parser::CemAstNode,
    schema::{
        datatype_contracts::LexicalInput,
        datatype_validation::ValueRepresentation,
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        document_model::{
            attribute_facets::FacetContext, DiagnosticBehavior, EngineDiagnosticBehavior,
        },
    },
};
use std::{collections::BTreeMap, sync::Arc};

/// The host associates compiled diagnostic bindings with their original schema.
/// `complete` includes the diagnostic declarations and their behavior dependencies.
/// A map copied from a consuming or replacement schema is not a valid substitute.
#[derive(Debug, Clone)]
pub struct AttributeDiagnosticBindings {
    pub schema: SchemaDeclarationNode,
    pub complete: bool,
    pub behaviors: BTreeMap<String, DiagnosticBehavior>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeReadinessState {
    Ready,
    Pending,
    Invalid,
}
#[derive(Debug, Clone)]
pub struct AttributeReadinessIssue {
    pub code: &'static str,
    pub source: SchemaDeclarationNode,
}
#[derive(Debug)]
pub struct AttributeDeclarationReadiness {
    pub declaration: SchemaDeclarationNode,
    pub diagnostic_schema: Option<SchemaDeclarationNode>,
    pub state: AttributeReadinessState,
    pub issues: Vec<AttributeReadinessIssue>,
    pub default_source: Option<SchemaDeclarationNode>,
    pub default_validation: Option<AttributeValidation>,
    pub control_stop: Option<ControlError>,
}
impl AttributeDeclarationReadiness {
    pub fn is_ready(&self) -> bool {
        self.state == AttributeReadinessState::Ready
    }
    fn issue(
        &mut self,
        state: AttributeReadinessState,
        code: &'static str,
        source: SchemaDeclarationNode,
    ) {
        self.state = state;
        self.issues.push(AttributeReadinessIssue { code, source });
    }
}
impl BoundAttributeFacets {
    /// Check original declaring-scope diagnostic dependencies and validate a
    /// literal default through the same preparation/rule/facet consumer as data.
    /// The original @default is the candidate, matching @values constant ingress.
    /// No synthetic use-site attribute or copied source tree is created.
    pub fn check_declaration_readiness(
        &self,
        host: &impl SchemaDeclarationHost,
        diagnostics: &AttributeDiagnosticBindings,
        runtime: &ValidationRuntime<'_>,
        limits: AttributeValidationLimits,
    ) -> AttributeDeclarationReadiness {
        let mut report = self.check_declaration_readiness_inner(host, diagnostics, runtime, limits);
        if let Err(error) = runtime.control.check_scope(runtime.scope) {
            report.state = AttributeReadinessState::Pending;
            report.control_stop = Some(error);
        }
        report
    }

    fn check_declaration_readiness_inner(
        &self,
        host: &impl SchemaDeclarationHost,
        diagnostics: &AttributeDiagnosticBindings,
        runtime: &ValidationRuntime<'_>,
        limits: AttributeValidationLimits,
    ) -> AttributeDeclarationReadiness {
        let declaration = self.binding().declaration.clone();
        let mut report = AttributeDeclarationReadiness {
            declaration: declaration.clone(),
            diagnostic_schema: None,
            state: AttributeReadinessState::Pending,
            issues: vec![],
            default_source: None,
            default_validation: None,
            control_stop: None,
        };
        if let Err(error) = runtime.control.check_scope(runtime.scope) {
            report.control_stop = Some(error);
            return report;
        }
        let Some(schema) = host.declaration_schema(&declaration) else {
            report.issue(
                AttributeReadinessState::Pending,
                "attribute-declaration-scope-pending",
                declaration,
            );
            return report;
        };
        if schema.identity() != diagnostics.schema.identity() {
            report.issue(
                AttributeReadinessState::Invalid,
                "attribute-diagnostic-scope-mismatch",
                declaration,
            );
            return report;
        }
        report.diagnostic_schema = Some(schema);
        let model = self.binding().local_constraints();
        let dependencies = [
            (
                "values-diagnostic",
                model.values_diagnostic.as_deref(),
                EngineDiagnosticBehavior::ValueVocabulary,
            ),
            (
                "type-diagnostic",
                model.type_diagnostic.as_deref(),
                EngineDiagnosticBehavior::ScalarType,
            ),
            (
                "datatype-param-diagnostic",
                model.datatype_param_diagnostic.as_deref(),
                EngineDiagnosticBehavior::DatatypeParam,
            ),
        ];
        if dependencies.iter().any(|(_, code, _)| code.is_some()) && !diagnostics.complete {
            report.issue(
                AttributeReadinessState::Pending,
                "attribute-diagnostic-bindings-pending",
                declaration,
            );
            return report;
        }
        for (field, code, expected) in dependencies {
            let Some(code) = code else { continue };
            let source = self
                .constraint_field(field)
                .unwrap_or_else(|| declaration.clone());
            let issue = match diagnostics.behaviors.get(code) {
                None => Some("attribute-diagnostic-unresolved"),
                Some(behavior) if behavior.code != code => {
                    Some("attribute-diagnostic-code-mismatch")
                }
                Some(behavior) if behavior.engine_behavior != Some(expected) => {
                    Some("attribute-diagnostic-family")
                }
                Some(_) => None,
            };
            if let Some(issue) = issue {
                report.issue(AttributeReadinessState::Invalid, issue, source);
            }
        }
        if !report.issues.is_empty() {
            return report;
        }
        let Some(default) = model.default_value.as_deref() else {
            report.state = AttributeReadinessState::Ready;
            return report;
        };
        let Some(field) = self.constraint_field("default") else {
            report.issue(
                AttributeReadinessState::Pending,
                "attribute-default-source-pending",
                declaration,
            );
            return report;
        };
        report.default_source = Some(field.clone());
        if self.profile().family().representation() == ValueRepresentation::Nodes {
            report.issue(
                AttributeReadinessState::Invalid,
                "attribute-default-native-required",
                field,
            );
            return report;
        }
        let Some(tree) = host.input_source_tree(&field) else {
            report.issue(
                AttributeReadinessState::Pending,
                "attribute-default-source-pending",
                field,
            );
            return report;
        };
        if !Arc::ptr_eq(tree.ast_owner(), field.document()) {
            report.issue(
                AttributeReadinessState::Invalid,
                "attribute-default-owner-mismatch",
                field,
            );
            return report;
        }
        let Some(candidate) = RetainedCemNode::new(tree, field.node_id()) else {
            report.issue(
                AttributeReadinessState::Invalid,
                "attribute-default-source-invalid",
                field,
            );
            return report;
        };
        let CemAstNode::Attribute { source, .. } = field.node() else {
            report.issue(
                AttributeReadinessState::Invalid,
                "attribute-default-source-invalid",
                field,
            );
            return report;
        };
        let candidate = candidate.query_item();
        let input = PreparationInput {
            lexical: LexicalInput::new(Arc::from(default), source.clone()),
            fallback: DiagnosticAttribution::from_node(&candidate),
            candidate: vec![candidate],
        };
        let values = BTreeMap::from([(model.name.clone(), default.to_owned())]);
        let validation = self.validate_lexical(
            &input,
            FacetContext {
                element_name: "schema:attribute-default",
                source: field.node(),
                diagnostic_behaviors: &diagnostics.behaviors,
                attribute_values: &values,
            },
            runtime,
            limits,
        );
        match validation.accepted {
            Some(true) => report.state = AttributeReadinessState::Ready,
            Some(false) => report.issue(
                AttributeReadinessState::Invalid,
                "attribute-default-invalid",
                field.clone(),
            ),
            None => report.issue(
                AttributeReadinessState::Pending,
                "attribute-default-incomplete",
                field.clone(),
            ),
        }
        report.default_validation = Some(validation);
        report
    }

    fn constraint_field(&self, name: &str) -> Option<SchemaDeclarationNode> {
        self.binding().constraint_fields().iter().rev().find(|field| matches!(field.node(), CemAstNode::Attribute {expanded_name,..} if expanded_name.namespace_uri.is_empty() && expanded_name.local_name == name)).cloned()
    }
}
