//! Lifecycle installation of original attribute contracts after datatype compilation.
mod pretyped;
mod external_typed;
use crate::{
    attribute_datatypes::bind_attribute_datatype,
    attribute_readiness::{AttributeDiagnosticBindings, AttributeReadinessState},
    attribute_validation::{
        AttributeDatatypePhase, AttributeValidation, AttributeValidationLimits,
    },
    datatype_compilation::DatatypeValidation,
    datatype_facets::{AttributeFacetBindingError, BoundAttributeFacets},
    datatype_preparation::PreparationInput,
    datatype_results::DiagnosticAttribution,
    datatype_validation::{ValidationInput, ValidationRuntime},
    eval::RetainedCemNode,
    schema_references::CemQlSchemaDeclarationHost,
};
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    operation_control::ROOT_EXECUTION_SCOPE_ID,
    parser::CemAstNode,
    schema::{
        attribute_datatypes::{
            AttributeDatatypeInput, AttributeDatatypeValidation, AttributeDatatypeValue,
            CompiledAttributeDatatype,
        },
        datatype_contracts::{
            DatatypeCompilation, DatatypeCompilationIssue, DatatypeIssueState, LexicalInput,
        },
        datatype_validation::ValueRepresentation,
        declaration_references::{
            SchemaDeclarationHost, SchemaDeclarationKind, SchemaDeclarationNode,
        },
        document_model::{
            attribute_facets::{FacetCompilationError, FacetContext},
            SchemaDocumentModel,
        },
        reference_traversal::ReferenceTraversalLimits,
    },
    value::reference_resolution::{ReferenceResolutionError, ReferenceResolutionState},
};
use std::{collections::BTreeMap, sync::Arc};

/// Original attribute metadata with no implicit grant to its source ancestors
/// or unevaluated native payload. Authorized value navigation is a separate role.
#[derive(Debug)]
struct AttributeCandidate(crate::eval::Item);
pub(crate) fn original_attribute_candidate(value: &crate::eval::Item) -> Option<crate::eval::RetainedCemNode> {
    let candidate = value.view()?.downcast_ref::<AttributeCandidate>()?;
    crate::eval::retained_cem_node(&candidate.0)
}
impl crate::eval::QueryItemView for AttributeCandidate {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.ql.attribute-candidate"
    }
    fn identity(&self) -> String {
        self.0.identity().expect("retained attribute identity")
    }
    fn kind(&self) -> crate::eval::QueryItemViewKind {
        crate::eval::QueryItemViewKind::Node
    }
    fn field(&self, name: &str) -> Option<Vec<crate::eval::Item>> {
        match name {
            "id" | "kind" | "name" | "namespace" | "value" | "values" | "line" => {
                self.0.view()?.field(name)
            }
            _ => None,
        }
    }
    fn source_map(&self) -> Option<cem_ml::source_map::SourceMapStack> {
        self.0.source_map()
    }
    fn provenance(&self) -> Option<cem_ml::value::artifact::CemValueProvenance> {
        self.0.view()?.provenance()
    }
}

#[derive(Debug)]
struct ActiveAttributeDatatype {
    facets: BoundAttributeFacets,
    diagnostics: AttributeDiagnosticBindings,
    limits: AttributeValidationLimits,
    publication: Arc<std::sync::atomic::AtomicBool>,
}
impl CompiledAttributeDatatype for ActiveAttributeDatatype {
    fn typed_admission(&self) -> Option<cem_ml::schema::attribute_datatypes::NativeAttributeTypedAdmission> {
        external_typed::admission(self)
    }
    fn prepare(&self, input: AttributeDatatypeInput<'_>) -> cem_ml::schema::attribute_datatypes::AttributeDatatypePreparation {
        pretyped::prepare(self, &input)
    }
    fn retire_preparations(&self) {
        self.publication.store(false, std::sync::atomic::Ordering::Release);
    }
    fn declaration(&self) -> &SchemaDeclarationNode {
        &self.facets.binding().declaration
    }
    fn is_node_valued(&self) -> bool {
        self.facets.profile().family().representation() == ValueRepresentation::Nodes
    }
    fn validate(&self, input: AttributeDatatypeInput<'_>) -> AttributeDatatypeValidation {
        if let AttributeDatatypeValue::ExternalTyped(handle) = &input.value {
            return external_typed::validate(self, handle, &input);
        }
        if let AttributeDatatypeValue::Prepared(handle) = &input.value {
            return pretyped::validate(self, handle, &input);
        }
        let candidate = input.source_tree.as_ref().and_then(|tree| {
            let CemAstNode::Attribute { node_id, .. } = input.source else {
                return None;
            };
            // An arena-local ID alone cannot authorize another candidate owner.
            if !std::ptr::eq(tree.ast().get(*node_id)?, input.source) {
                return None;
            }
            RetainedCemNode::new(tree.clone(), *node_id).map(|node| node.query_item())
        });
        let fallback = candidate
            .as_ref()
            .map(DiagnosticAttribution::from_node)
            .unwrap_or_else(|| {
                let source_map = match input.source {
                    CemAstNode::Attribute { source, .. } => Some(source.clone()),
                    _ => None,
                };
                DiagnosticAttribution {
                    source_map,
                    ..Default::default()
                }
            });
        let candidate = candidate
            .into_iter()
            .map(|original| crate::eval::Item::native(AttributeCandidate(original)))
            .collect();
        // Rule parameters are closed typed roles. Runtime control is invocation
        // owned; compilation contexts and prior selections are never reused.
        let runtime = ValidationRuntime {
            control: input.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        };
        let context = FacetContext {
            element_name: input.element_name,
            source: input.source,
            diagnostic_behaviors: &self.diagnostics.behaviors,
            attribute_values: input.attribute_values,
        };
        let report = match input.value {
            AttributeDatatypeValue::Prepared(_) | AttributeDatatypeValue::ExternalTyped(_) => unreachable!(),
            AttributeDatatypeValue::Lexical(text) => self.facets.validate_lexical(
                &PreparationInput {
                    lexical: LexicalInput::new(
                        Arc::from(text),
                        fallback.source_map.clone().unwrap_or_default(),
                    ),
                    candidate,
                    fallback,
                },
                context,
                &runtime,
                self.limits,
            ),
            AttributeDatatypeValue::Nodes(access) => {
                let Ok(tree) = crate::attribute_values::NativeAttributeQueryTree::new(access)
                else {
                    return incomplete(input.source, "Attribute target access is unavailable");
                };
                self.facets.validate_nodes(
                    &ValidationInput {
                        value: tree.roots(),
                        candidate,
                        fallback,
                    },
                    context,
                    &runtime,
                    self.limits,
                )
            }
        };
        export_validation(&report, input.source)
    }
}
fn export_validation(report: &AttributeValidation, source: &CemAstNode) -> AttributeDatatypeValidation {
        let mut diagnostics = validation_diagnostics(report);
        if report.accepted.is_none()
            || (report.accepted == Some(false)
                && !diagnostics.iter().any(|d| d.severity.is_hard_violation()))
        {
            diagnostics.push(diagnostic(
                source,
                if report.accepted.is_none() {
                    "cem.schema_validation.attribute_datatype_incomplete"
                } else {
                    "cem.schema_validation.attribute_datatype_invalid"
                },
                if report.accepted.is_none() {
                    "Attribute datatype validation is incomplete"
                } else {
                    "Attribute value violates its compiled datatype contract"
                },
            ));
        }
        AttributeDatatypeValidation {
            accepted: report.accepted,
            diagnostics,
        }
}
fn diagnostic(source: &CemAstNode, code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        severity: Severity::Error,
        message: message.into(),
        source_map: match source {
            CemAstNode::Element { source, .. }
            | CemAstNode::Attribute { source, .. }
            | CemAstNode::Reference { source, .. } => Some(source.clone()),
            _ => None,
        },
        ..Default::default()
    }
}
fn incomplete(source: &CemAstNode, message: &str) -> AttributeDatatypeValidation {
    AttributeDatatypeValidation {
        accepted: None,
        diagnostics: vec![diagnostic(
            source,
            "cem.schema_validation.attribute_datatype_incomplete",
            message,
        )],
    }
}
fn datatype_diagnostics(report: &DatatypeValidation, out: &mut Vec<Diagnostic>) {
    out.extend(report.enumeration_diagnostics.clone());
    for rule in &report.completed {
        out.extend(rule.result.diagnostics.clone());
        out.extend(rule.result.execution_diagnostics.clone());
    }
}
fn validation_diagnostics(report: &AttributeValidation) -> Vec<Diagnostic> {
    let mut out = vec![];
    match &report.datatype {
        Some(AttributeDatatypePhase::Lexical(prepared)) => {
            out.extend(prepared.diagnostics.clone());
            if let Some(validation) = &prepared.validation {
                datatype_diagnostics(validation, &mut out);
            }
        }
        Some(AttributeDatatypePhase::Nodes { validation, .. })
        | Some(AttributeDatatypePhase::ExternalTyped { validation, .. })
        | Some(AttributeDatatypePhase::Pretyped { validation, .. }) => {
            datatype_diagnostics(validation, &mut out)
        }
        None => {}
    }
    if let Some(facets) = &report.facets {
        out.extend(facets.diagnostics.clone());
    }
    out
}
fn issue(
    compilation: &mut DatatypeCompilation,
    source: &SchemaDeclarationNode,
    code: &'static str,
    invalid: bool,
) {
    compilation.issues.push(DatatypeCompilationIssue {
        code,
        source: source.clone(),
        related: None,
        state: if invalid {
            DatatypeIssueState::Invalid
        } else {
            DatatypeIssueState::Pending
        },
    });
}
fn diagnostic_bindings(
    schema: SchemaDeclarationNode,
    model: &SchemaDocumentModel,
) -> AttributeDiagnosticBindings {
    AttributeDiagnosticBindings {
        schema,
        complete: model.declaration_references.name_issues.is_empty()
            && model
                .declaration_references
                .sites
                .iter()
                .filter(|s| {
                    matches!(
                        s.kind,
                        SchemaDeclarationKind::Diagnostic | SchemaDeclarationKind::Behavior
                    )
                })
                .all(|s| {
                    s.state() == ReferenceResolutionState::Resolved
                        && !s.resolution.as_ref().is_some_and(|r| r.failed)
                }),
        behaviors: model.diagnostic_behaviors.clone(),
    }
}
/// Called only for an explicitly installed datatype compiler. All typed attributes
/// are guarded before binding; partial consumers never authorize package publication.
/// The same immutable descriptor is used by literal and native slots.
pub fn activate_attribute_datatypes(
    model: &mut SchemaDocumentModel,
    compilation: &mut DatatypeCompilation,
    host: &mut CemQlSchemaDeclarationHost,
    mut limits: ReferenceTraversalLimits,
    runtime: &ValidationRuntime<'_>,
) -> Result<(), ReferenceResolutionError> {
    let declarations = model.declaration_references.attribute_declarations.clone();
    model.attribute_datatypes.clear();
    for (name, declaration) in &declarations {
        let has_type = matches!(declaration.node(), CemAstNode::Element {attributes, ..} if attributes.iter().any(|id| matches!(declaration.document().get(*id), Some(CemAstNode::Attribute {expanded_name, ..}) if expanded_name.local_name == "type")));
        if has_type {
            if let Some(attribute) = model.attributes.get_mut(name) {
                attribute.native_type_pending = true;
            }
        }
    }
    let mut diagnostic_models = BTreeMap::<String, AttributeDiagnosticBindings>::new();
    // Keep binding prerequisites immutable for this pass, so one failed attribute
    // does not erase independent facts at later source positions.
    let datatypes = compilation.clone();
    for (name, declaration) in declarations {
        if !model
            .attributes
            .get(&name)
            .is_some_and(|m| m.native_type_pending)
        {
            continue;
        }
        if runtime.control.check_scope(runtime.scope).is_err() || limits.max_work == 0 {
            issue(
                compilation,
                &declaration,
                "attribute-type-work-limit",
                false,
            );
            continue;
        }
        // Charge even a malformed source before any selector or local compilation.
        limits.max_work -= 1;
        if limits.max_work == 0 {
            issue(
                compilation,
                &declaration,
                "attribute-type-work-limit",
                false,
            );
            continue;
        }
        let binding = bind_attribute_datatype(declaration.clone(), &datatypes, host, limits)?;
        limits.max_work = limits.max_work.saturating_sub(binding.work_used);
        compilation.diagnostics.extend(binding.diagnostics);
        let Some(bound) = binding.bound else {
            issue(
                compilation,
                &declaration,
                binding
                    .issue
                    .unwrap_or("attribute-type-selection-incomplete"),
                binding.state == ReferenceResolutionState::Invalid,
            );
            continue;
        };
        let facets = match bound.compile_facets(&model.schema_uri, Default::default()) {
            Ok(facets) => facets,
            Err(error) => {
                let invalid = matches!(
                    error,
                    AttributeFacetBindingError::Contract(FacetCompilationError::Invalid(_))
                );
                if let AttributeFacetBindingError::Contract(FacetCompilationError::Invalid(
                    diagnostics,
                )) = error
                {
                    compilation.diagnostics.extend(diagnostics);
                }
                issue(
                    compilation,
                    &declaration,
                    "attribute-facet-profile-incomplete",
                    invalid,
                );
                continue;
            }
        };
        if facets.profile().family().representation() != ValueRepresentation::Nodes
            && !facets.binding().datatype.admits_external_typed()
            && !has_lexical_preparation(&facets.binding().datatype)
        {
            issue(
                compilation,
                &declaration,
                "attribute-preparation-unavailable",
                false,
            );
            continue;
        }
        let Some(schema) = host.declaration_schema(&declaration) else {
            issue(
                compilation,
                &declaration,
                "attribute-declaration-scope-pending",
                false,
            );
            continue;
        };
        let diagnostics = if let Some(diagnostics) = diagnostic_models.get(&schema.identity()) {
            diagnostics.clone()
        } else {
            if host.input_source_tree(&schema).is_none() {
                issue(
                    compilation,
                    &declaration,
                    "attribute-diagnostic-owner-pending",
                    false,
                );
                continue;
            };
            if limits.max_work == 0 {
                issue(
                    compilation,
                    &declaration,
                    "attribute-diagnostic-work-limit",
                    false,
                );
                continue;
            }
            let declaring_uri = match schema.node() {
                CemAstNode::Element { attributes, .. } => attributes
                    .iter()
                    .find_map(|id| match schema.document().get(*id) {
                        Some(CemAstNode::Attribute {
                            expanded_name,
                            value,
                            ..
                        }) if expanded_name.local_name == "namespace" => value.clone(),
                        _ => None,
                    })
                    .unwrap_or_else(|| model.schema_uri.clone()),
                _ => model.schema_uri.clone(),
            };
            let own_model = cem_ml::schema::declaration_references::compile_selected_schema_with_declaration_references(&declaring_uri, &schema, host, limits)?;
            let bindings = diagnostic_bindings(schema.clone(), &own_model);
            diagnostic_models.insert(schema.identity(), bindings.clone());
            bindings
        };
        let validation_limits = AttributeValidationLimits::default();
        let readiness =
            facets.check_declaration_readiness(host, &diagnostics, runtime, validation_limits);
        if let Some(default) = &readiness.default_validation {
            compilation
                .diagnostics
                .extend(validation_diagnostics(default));
        }
        if !readiness.is_ready() {
            if readiness.issues.is_empty() {
                issue(
                    compilation,
                    &declaration,
                    "attribute-readiness-incomplete",
                    false,
                );
            }
            for problem in readiness.issues {
                issue(
                    compilation,
                    &problem.source,
                    problem.code,
                    readiness.state == AttributeReadinessState::Invalid,
                );
            }
            continue;
        }
        model.attribute_datatypes.insert(
            name.clone(),
            Arc::new(ActiveAttributeDatatype {
                facets,
                diagnostics,
                limits: validation_limits,
                publication: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            }),
        );
        model.attributes.get_mut(&name).unwrap().native_type_pending = false;
    }
    Ok(())
}

fn has_lexical_preparation(datatype: &crate::datatype_compilation::ExecutableDatatype) -> bool {
    let mut current = datatype;
    loop {
        if current.preparation().is_none() {
            return false;
        }
        match current.item() {
            Some(item) => current = item,
            None => return true,
        }
    }
}
