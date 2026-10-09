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
        attribute: &SchemaDeclarationNode,
    ) -> Result<EnumerationRestriction, DatatypeCompilationIssue> {
        let fail = |code| pending(code, attribute);
        let malformed = |code| invalid(code, attribute);
        let (Some(EqualityBinding::Ready(equality)), Some(ConstantBinding::Ready(interpreter))) =
            (&descriptor.equality, &descriptor.interpreter)
        else {
            return Err(fail("datatype-enumeration-unavailable"));
        };
        let runtime = self
            .runtime
            .ok_or_else(|| fail("datatype-constant-context-unavailable"))?;
        let check = || {
            runtime
                .control
                .check_scope(runtime.scope)
                .map_err(|_| fail("datatype-constant-control"))
        };
        check()?;
        let ValueRepresentation::Scalar(representation) = descriptor.representation else {
            return Err(malformed("unsupported-datatype-facet"));
        };
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
        let tree = self
            .host
            .input_source_tree(attribute)
            .filter(|t| Arc::ptr_eq(t.ast_owner(), attribute.document()))
            .ok_or_else(|| fail("datatype-source-owner-unavailable"))?;
        let candidate = RetainedCemNode::new(tree, attribute.node_id())
            .ok_or_else(|| malformed("datatype-source-owner"))?
            .query_item();
        let attribution = DiagnosticAttribution::from_node(&candidate);
        let mut constants = vec![];
        // Offsets are byte ranges in the decoded value; source ownership stays on the attribute.
        let mut start = None;
        for (position, (index, ch)) in lexical
            .char_indices()
            .chain(std::iter::once((lexical.len(), ' ')))
            .enumerate()
        {
            if position % 64 == 0 {
                check()?;
            }
            if !ch.is_whitespace() {
                start.get_or_insert(index);
                continue;
            }
            let Some(start) = start.take() else { continue };
            if self.preparation.max_constants == 0 {
                return Err(fail("datatype-constant-count-limit"));
            }
            self.preparation.max_constants -= 1;
            self.spend(1, attribute)?;
            check()?;
            let token = ConstantToken {
                source: attribute.clone(),
                lexical: lexical.clone(),
                span: start..index,
            };
            let execution = interpreter
                .registration
                .implementation
                .interpret(ConstantCall {
                    token: &token,
                    datatype: &interpreter.datatype,
                    runtime,
                });
            check()?;
            let (value, mut diagnostics, issue) = match execution {
                ConstantExecution::Prepared { value, diagnostics } => {
                    (Some(value), diagnostics, None)
                }
                ConstantExecution::Rejected(d) => {
                    (None, d, Some(malformed("datatype-constant-rejected")))
                }
                ConstantExecution::Pending(d) => (None, d, Some(fail("datatype-constant-pending"))),
                ConstantExecution::Unavailable(d) => {
                    (None, d, Some(fail("datatype-constant-unavailable")))
                }
                ConstantExecution::Failed(d) => (None, d, Some(fail("datatype-constant-failed"))),
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
            if value.len() != 1 || !datatype_validation::scalar(&value[0], representation) {
                return Err(malformed("datatype-constant-representation"));
            }
            self.validate_constant(descriptor, &value, &candidate, &attribution, attribute)?;
            constants.push(PreparedConstant {
                token,
                value: value[0].clone(),
            });
        }
        if constants.is_empty() {
            return Err(malformed("datatype-empty-vocabulary"));
        }
        Ok(EnumerationRestriction {
            source: descriptor.source.clone(),
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
