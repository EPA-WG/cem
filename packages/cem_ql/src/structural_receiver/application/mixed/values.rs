use super::*;
use crate::{
    datatype_preparation::PreparationInput, datatype_results::DiagnosticAttribution, eval::Item,
};
use cem_ml::{
    diagnostics::Diagnostic,
    schema::{
        attribute_references::NativeAttributeReferenceValidation,
        datatype_contracts::LexicalInput,
        document_model::{convert_attribute_value_with_check, AttributeValueConversionError},
    },
};
pub struct ReceiverInput {
    pub source: Arc<RetainedCemTree>,
    pub attribute: AstNodeId,
}
pub struct ScalarValidation {
    pub accepted: bool,
    pub value: Option<TypedAttributeValue>,
    pub diagnostics: Vec<Diagnostic>,
}
pub struct MixedDatatypeValidation {
    pub accepted: Option<bool>,
    pub datatype: DatatypeValidation,
    pub diagnostics: Vec<Diagnostic>,
}
pub(super) struct Input<'a> {
    pub node: &'a CemAstNode,
    pub element: String,
    pub declaration: SchemaDeclarationNode,
    pub stack: &'a cem_ml::source_map::SourceMapStack,
}
pub(super) fn input<'a>(
    host: &FreshStructuralHost,
    name: &str,
    value: &'a ReceiverInput,
) -> Result<Input<'a>, String> {
    let node = value.source.ast().get(value.attribute).ok_or(REFUSED)?;
    if !matches!(node,CemAstNode::Attribute{expanded_name,..} if expanded_name.namespace_uri.is_empty() && expanded_name.local_name==name)
    {
        return Err(REFUSED.into());
    }
    let declaration = SchemaDeclarationNode::new(value.source.ast_owner().clone(), value.attribute)
        .ok_or(REFUSED)?;
    if host
        .inner
        .source_tree(&declaration)
        .is_none_or(|t| !Arc::ptr_eq(t, &value.source))
        || host
            .inner
            .scope(&host.inner.source_reference(declaration.clone()))
            .is_none_or(|s| host.inner.registered_scope_validator_context(s).is_none())
    {
        return Err(REFUSED.into());
    }
    let Some(CemAstNode::Element { expanded_name, .. }) = value
        .source
        .source_parent(value.attribute)
        .and_then(|p| value.source.ast().get(p))
    else {
        return Err(REFUSED.into());
    };
    crate::preparation_evidence::check_attribute_source_metadata(
        match node {
            CemAstNode::Attribute { source, .. } => source,
            _ => return Err(REFUSED.into()),
        },
        &host.control,
    )
    .map_err(|_| REFUSED)?;
    Ok(Input {
        node,
        element: expanded_name.local_name.clone(),
        declaration,
        stack: match node {
            CemAstNode::Attribute { source, .. } => source,
            _ => return Err(REFUSED.into()),
        },
    })
}
pub(super) fn lexical(
    host: &FreshStructuralHost,
    name: &str,
    value: &ReceiverInput,
) -> Result<PreparationInput, String> {
    let data = input(host, name, value)?;
    let CemAstNode::Attribute {
        value: text,
        value_nodes,
        source,
        ..
    } = data.node
    else {
        return Err(REFUSED.into());
    };
    if !value_nodes.is_empty() || text.as_ref().is_some_and(|s| s.len() > 1_048_576) {
        return Err(REFUSED.into());
    }
    Ok(PreparationInput {
        lexical: LexicalInput::new(Arc::from(text.as_deref().unwrap_or("")), source.clone()),
        candidate: crate::attribute_activation::receiver_candidate(
            value.source.clone(),
            value.attribute,
        )
        .into_iter()
        .collect(),
        fallback: DiagnosticAttribution::from_node(
            &crate::eval::RetainedCemNode::new(value.source.clone(), value.attribute)
                .ok_or(REFUSED)?
                .query_item(),
        ),
    })
}
pub(super) fn counts(
    group: &Group,
    receiver: &Receiver,
    value: &ReceiverInput,
    host: &FreshStructuralHost,
    count: usize,
) -> Result<Vec<Diagnostic>, String> {
    let data = input(host, &receiver.local.name, value)?;
    let mut diagnostics = vec![];
    cem_ml::schema::document_model::validate_prepared_attribute_count(
        &group.schema_uri,
        &data.element,
        &receiver.local,
        count,
        data.node,
        &mut diagnostics,
    );
    Ok(diagnostics)
}
impl LinkedMixedReceivers {
    /// Validate only the selected original field; this is not whole-input schema acceptance.
    pub fn validate_node(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        value: &ReceiverInput,
    ) -> Result<NativeAttributeReferenceValidation<CemQlSchemaReferenceNode>, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        let data = input(host, name, value)?;
        let receiver = self.group.receivers.get(name).ok_or(REFUSED)?;
        let Binding::Node { model, .. } = &receiver.binding else {
            return Err(REFUSED.into());
        };
        let report = cem_ml::schema::attribute_references::validate_native_attribute_reference(
            data.declaration,
            model,
            &data.element,
            host,
            ReferenceTraversalLimits {
                max_depth: 64,
                max_work: 100_000,
            },
        )
        .map_err(|_| REFUSED)?;
        self.group.refresh(host)?;
        Ok(report)
    }
    pub fn validate_scalar(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        value: &ReceiverInput,
    ) -> Result<ScalarValidation, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        let receiver = self.group.receivers.get(name).ok_or(REFUSED)?;
        let Binding::Scalar { contract } = &receiver.binding else {
            return Err(REFUSED.into());
        };
        let original = lexical(host, name, value)?;
        let result = convert_attribute_value_with_check(
            &original.lexical.text,
            contract,
            &original.lexical.source,
            &mut || host.control.check_scope(ROOT_EXECUTION_SCOPE_ID),
        );
        let mut report = match result {
            Ok(value) => ScalarValidation {
                accepted: true,
                value: Some(value),
                diagnostics: vec![],
            },
            Err(AttributeValueConversionError::Invalid(diagnostics)) => ScalarValidation {
                accepted: false,
                value: None,
                diagnostics,
            },
            Err(AttributeValueConversionError::Interrupted(_)) => return Err(REFUSED.into()),
        };
        report
            .diagnostics
            .extend(counts(&self.group, receiver, value, host, 1)?);
        report.accepted &= !report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation());
        if !report.accepted {
            report.value = None;
        }
        self.group.refresh(host)?;
        Ok(report)
    }
    /// Closed scalar typed inspection data is revalidated; it is never preparation authority.
    pub fn validate_scalar_typed(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        source: &ReceiverInput,
        value: &TypedAttributeValue,
    ) -> Result<ScalarValidation, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        let data = input(host, name, source)?;
        if value.lexical.len() > 1_048_576 {
            return Err(REFUSED.into());
        }
        let receiver = self.group.receivers.get(name).ok_or(REFUSED)?;
        let Binding::Scalar { contract } = &receiver.binding else {
            return Err(REFUSED.into());
        };
        let result =
            convert_attribute_value_with_check(&value.lexical, contract, data.stack, &mut || {
                host.control.check_scope(ROOT_EXECUTION_SCOPE_ID)
            });
        let mut report = match result {
            Ok(converted) => ScalarValidation {
                accepted: converted == *value,
                value: (converted == *value).then_some(converted),
                diagnostics: vec![],
            },
            Err(AttributeValueConversionError::Invalid(diagnostics)) => ScalarValidation {
                accepted: false,
                value: None,
                diagnostics,
            },
            Err(_) => return Err(REFUSED.into()),
        };
        report
            .diagnostics
            .extend(counts(&self.group, receiver, source, host, 1)?);
        report.accepted &= !report
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation());
        if !report.accepted {
            report.value = None;
        }
        self.group.refresh(host)?;
        Ok(report)
    }
    /// Typed executable ingress requires the compiled descriptor's explicit typed-only profile.
    pub fn validate_executable_typed(
        &self,
        host: &mut FreshStructuralHost,
        name: &str,
        source: &ReceiverInput,
        values: Vec<Item>,
        limits: ValidationLimits,
    ) -> Result<MixedDatatypeValidation, String> {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        let _ = input(host, name, source)?;
        let receiver = self.group.receivers.get(name).ok_or(REFUSED)?;
        let Binding::Executable { root } = &receiver.binding else {
            return Err(REFUSED.into());
        };
        let root = root.upgrade().ok_or(REFUSED)?;
        let count = values.len();
        let candidate = crate::attribute_activation::receiver_candidate(
            source.source.clone(),
            source.attribute,
        )
        .into_iter()
        .collect();
        let fallback = DiagnosticAttribution::from_node(
            &crate::eval::RetainedCemNode::new(source.source.clone(), source.attribute)
                .ok_or(REFUSED)?
                .query_item(),
        );
        let input = ValidationInput {
            value: values,
            candidate,
            fallback,
        };
        let datatype = root.with_runtime(host, |d, r, c| {
            if d.admits_external_typed() {
                Ok(d.validate_with_rule_contexts(&input, r, limits, c))
            } else {
                Err(REFUSED.to_string())
            }
        })??;
        let diagnostics = counts(&self.group, receiver, source, host, count)?;
        if crate::attribute_validation::diagnostic_cost(&datatype).saturating_add(diagnostics.len())
            > limits.max_diagnostics
        {
            return Err(REFUSED.into());
        }
        let accepted = datatype
            .accepted
            .map(|a| a && !diagnostics.iter().any(|d| d.severity.is_hard_violation()));
        self.group.refresh(host)?;
        Ok(MixedDatatypeValidation {
            accepted,
            datatype,
            diagnostics,
        })
    }
}
