use super::*;
use crate::{
    datatype_enumeration::*,
    datatype_results::DiagnosticAttribution,
    datatype_validation::{self, ValidationInput, ValidationStopReason},
};

impl<H: DatatypeDependencyHost> Compiler<'_, '_, H> {
    pub(super) fn equality(
        &self,
        source: &DatatypeSource,
        base: Option<&ExecutableDatatype>,
        representation: ValueRepresentation,
    ) -> Result<Option<EqualityBinding>, DatatypeCompilationIssue> {
        match self
            .implementations
            .equalities
            .get(&source.declaration().identity())
        {
            Some((registered, binding)) => {
                if registered.scope().identity() != source.scope().identity() {
                    return Err(invalid("equality-source-scope", source.declaration()));
                }
                if let EqualityBinding::Ready(r) = binding {
                    if representation != ValueRepresentation::Scalar(r.representation()) {
                        return Err(invalid(
                            "equality-representation-incompatible",
                            source.declaration(),
                        ));
                    }
                }
                Ok(Some(binding.clone()))
            }
            None => Ok(base.and_then(|b| b.equality.clone())),
        }
    }
    pub(super) fn interpreter(
        &self,
        source: &DatatypeSource,
        base: Option<&ExecutableDatatype>,
        representation: ValueRepresentation,
    ) -> Result<Option<ConstantBinding>, DatatypeCompilationIssue> {
        match self
            .implementations
            .interpreters
            .get(&source.declaration().identity())
        {
            Some((registered, binding)) => {
                if registered.scope().identity() != source.scope().identity() {
                    return Err(invalid("constant-source-scope", source.declaration()));
                }
                if let ConstantBinding::Ready(r) = binding {
                    if representation != ValueRepresentation::Scalar(r.representation()) {
                        return Err(invalid(
                            "constant-representation-incompatible",
                            source.declaration(),
                        ));
                    }
                }
                Ok(Some(binding.clone()))
            }
            None => Ok(base.and_then(|b| b.interpreter.clone())),
        }
    }
    pub(super) fn prepare_enumeration(
        &mut self,
        descriptor: &ExecutableDatatype,
        plan: &DatatypeSourcePlan,
    ) -> Result<EnumerationRestriction, DatatypeCompilationIssue> {
        let vocabulary_source = descriptor
            .source
            .attribute("values")
            .unwrap_or_else(|| descriptor.source.declaration());
        let fail = |code| pending(code, vocabulary_source);
        let malformed = |code| invalid(code, vocabulary_source);
        let (Some(EqualityBinding::Ready(equality)), Some(ConstantBinding::Ready(interpreter))) =
            (&descriptor.equality, &descriptor.interpreter)
        else {
            return Err(fail("datatype-enumeration-unavailable"));
        };
        let runtime = self
            .runtime
            .ok_or_else(|| fail("datatype-constant-context-unavailable"))?;
        runtime
            .control
            .check_scope(runtime.scope)
            .map_err(|_| fail("datatype-constant-control"))?;
        let ValueRepresentation::Scalar(representation) = descriptor.representation else {
            return Err(malformed("unsupported-datatype-facet"));
        };
        let equality = equality
            .bind(
                self.host
                    .input_source_tree(equality.identity().source.declaration())
                    .ok_or_else(|| fail("equality-source-owner-unavailable"))?,
            )
            .ok_or_else(|| malformed("equality-source-owner"))?;
        let interpreter = interpreter
            .bind(
                self.host
                    .input_source_tree(interpreter.identity().source.declaration())
                    .ok_or_else(|| fail("constant-source-owner-unavailable"))?,
            )
            .ok_or_else(|| malformed("constant-source-owner"))?;
        let fields = if let Some(attribute) = descriptor.source.attribute("values") {
            vec![(attribute.clone(), None)]
        } else {
            self.retained_constant_fields(plan)?
                .into_iter()
                .map(|(field, declaration)| (field, Some(declaration)))
                .collect()
        };
        let mut constants = vec![];
        for (attribute, declaration) in fields {
            let fail = |code| pending(code, &attribute);
            let malformed = |code| invalid(code, &attribute);
            let lexical = match attribute.node() {
                CemAstNode::Attribute {
                    value: Some(value),
                    value_nodes,
                    ..
                } if value_nodes.is_empty() => value.as_str(),
                _ => return Err(malformed("datatype-values-require-literal-tokens")),
            };
            if lexical.len() > self.preparation.max_lexical_bytes {
                return Err(fail("datatype-constant-byte-limit"));
            }
            self.preparation.max_lexical_bytes -= lexical.len();
            let lexical: Arc<str> = Arc::from(lexical);
            let tree = self
                .host
                .input_source_tree(&attribute)
                .filter(|t| Arc::ptr_eq(t.ast_owner(), attribute.document()))
                .ok_or_else(|| fail("datatype-source-owner-unavailable"))?;
            let candidate = RetainedCemNode::new(tree, attribute.node_id())
                .ok_or_else(|| malformed("datatype-source-owner"))?
                .query_item();
            let attribution = DiagnosticAttribution::from_node(&candidate);
            let mut spans = vec![];
            let form = if declaration.is_some() {
                if self.preparation.max_constants == 0 {
                    return Err(fail("datatype-constant-count-limit"));
                }
                spans.push(0..lexical.len());
                ConstantForm::RetainedLiteral
            } else {
                let mut start = None;
                for (position, (index, ch)) in lexical
                    .char_indices()
                    .chain(std::iter::once((lexical.len(), ' ')))
                    .enumerate()
                {
                    if position % 64 == 0 {
                        runtime
                            .control
                            .check_scope(runtime.scope)
                            .map_err(|_| fail("datatype-constant-control"))?;
                    }
                    if !ch.is_whitespace() {
                        start.get_or_insert(index);
                        continue;
                    }
                    let Some(start) = start.take() else { continue };
                    if spans.len() >= self.preparation.max_constants {
                        return Err(fail("datatype-constant-count-limit"));
                    }
                    spans.push(start..index);
                }
                ConstantForm::WhitespaceToken
            };
            for span in spans {
                self.preparation.max_constants -= 1;
                self.spend(1, &attribute)?;
                let token = ConstantToken {
                    source: attribute.clone(),
                    lexical: lexical.clone(),
                    span,
                    form,
                };
                let execution = interpreter
                    .registration
                    .implementation
                    .interpret(ConstantCall {
                        token: &token,
                        candidate: &candidate,
                        fallback: &attribution,
                        limits: self.preparation.validation,
                        datatype: &interpreter.datatype,
                        runtime,
                    });
                runtime
                    .control
                    .check_scope(runtime.scope)
                    .map_err(|_| fail("datatype-constant-control"))?;
                if let Some(failure) = runtime.query_failure() {
                    self.diagnostics.extend(failure.diagnostics);
                    return Err(fail("datatype-constant-query-budget"));
                }
                let (value, mut diagnostics, issue) = match execution {
                    ConstantExecution::Prepared { value, diagnostics } => {
                        (Some(value), diagnostics, None)
                    }
                    ConstantExecution::Rejected(d) => {
                        (None, d, Some(malformed("datatype-constant-rejected")))
                    }
                    ConstantExecution::Pending(d) => {
                        (None, d, Some(fail("datatype-constant-pending")))
                    }
                    ConstantExecution::Unavailable(d) => {
                        (None, d, Some(fail("datatype-constant-unavailable")))
                    }
                    ConstantExecution::Failed(d) => {
                        (None, d, Some(fail("datatype-constant-failed")))
                    }
                };
                if diagnostics.len() > self.preparation.validation.max_diagnostics {
                    return Err(fail("datatype-constant-diagnostic-limit"));
                }
                self.preparation.validation.max_diagnostics -= diagnostics.len();
                attribute_diagnostics(&mut diagnostics, &attribution, runtime)
                    .map_err(|_| fail("datatype-constant-control"))?;
                self.diagnostics.extend(diagnostics);
                if let Some(issue) = issue {
                    return Err(issue);
                }
                let value = value.unwrap();
                if declaration.is_some() {
                    use crate::eval::{AtomValue, Item};
                    let [scalar] = value.as_slice() else {
                        return Err(malformed("datatype-constant-representation"));
                    };
                    let bytes = match scalar {
                        Item::Atomic(
                            AtomValue::String(s) | AtomValue::Decimal(s) | AtomValue::AnyUri(s),
                        ) => s.len(),
                        Item::Atomic(_) => 0,
                        _ => crate::typed_scalar::immutable_storage(scalar)
                            .ok_or_else(|| malformed("datatype-constant-immutable-value-required"))?
                            .0
                            .len(),
                    };
                    if bytes > self.preparation.max_retained_value_bytes {
                        return Err(fail("datatype-constant-value-byte-limit"));
                    }
                    self.preparation.max_retained_value_bytes -= bytes;
                }
                if value.len() != 1 || !datatype_validation::scalar(&value[0], representation) {
                    return Err(malformed("datatype-constant-representation"));
                }
                self.validate_constant(descriptor, &value, &candidate, &attribution, &attribute)?;
                constants.push(PreparedConstant {
                    token,
                    value: value[0].clone(),
                    declaration: declaration.clone(),
                });
            }
        }
        if constants.is_empty() {
            return Err(malformed("datatype-empty-vocabulary"));
        }
        Ok(EnumerationRestriction {
            source: descriptor.source.clone(),
            vocabulary_source: vocabulary_source.clone(),
            constants,
            equality,
            interpreter,
        })
    }
    fn validate_constant(
        &mut self,
        descriptor: &ExecutableDatatype,
        value: &[crate::eval::Item],
        candidate: &crate::eval::Item,
        attribution: &DiagnosticAttribution,
        attribute: &SchemaDeclarationNode,
    ) -> Result<(), DatatypeCompilationIssue> {
        let fail = |code| pending(code, attribute);
        let limits = self.preparation.validation;
        if descriptor.rules.len() > limits.max_rules {
            return Err(fail("datatype-constant-rule-limit"));
        }
        if limits.max_input_values < 2 {
            return Err(fail("datatype-constant-value-limit"));
        }
        self.preparation.validation.max_rules -= descriptor.rules.len();
        self.preparation.validation.max_input_values -= 2;
        // The descriptor contains all base/local rules and inherited vocabularies,
        // but does not yet contain the vocabulary being prepared.
        let output = descriptor.validate(
            &ValidationInput {
                value: value.to_vec(),
                candidate: vec![candidate.clone()],
                fallback: attribution.clone(),
            },
            self.runtime.unwrap(),
            limits,
        );
        self.preparation.validation.max_comparisons = self
            .preparation
            .validation
            .max_comparisons
            .saturating_sub(output.comparisons);
        let mut diagnostics = output.enumeration_diagnostics;
        for rule in output.completed {
            diagnostics.extend(rule.result.diagnostics);
            diagnostics.extend(rule.result.execution_diagnostics);
        }
        if let Some(stop) = &output.stopped {
            // Equality diagnostics are already retained in enumeration_diagnostics.
            if output.enumeration_stop.is_none() {
                match &stop.reason {
                    ValidationStopReason::Pending(d)
                    | ValidationStopReason::Unavailable(d)
                    | ValidationStopReason::Failed(d) => diagnostics.extend(d.clone()),
                    ValidationStopReason::Result(
                        crate::datatype_results::DatatypeResultError::Execution(stream),
                    ) => diagnostics.extend(stream.diagnostics.clone()),
                    _ => {}
                }
            }
        }
        if diagnostics.len() > self.preparation.validation.max_diagnostics {
            return Err(fail("datatype-constant-diagnostic-limit"));
        }
        self.preparation.validation.max_diagnostics -= diagnostics.len();
        attribute_diagnostics(&mut diagnostics, attribution, self.runtime.unwrap())
            .map_err(|_| fail("datatype-constant-control"))?;
        self.diagnostics.extend(diagnostics);
        match output.accepted {
            Some(true) => Ok(()),
            Some(false) => Err(invalid("datatype-constant-contract-rejected", attribute)),
            None => Err(fail("datatype-constant-validation-incomplete")),
        }
    }
}
