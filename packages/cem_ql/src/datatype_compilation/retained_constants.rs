//! One bounded selection of the original declaration's constant slots. Selection
//! grants no scalar semantics; the original restriction interprets each literal.
use super::*;
use cem_ml::{
    schema::reference_policy::ReferenceOccurrence,
    value::reference_resolution::{resolve_owned_reference_structure, ReferenceResolutionState},
};

impl<H: DatatypeDependencyHost> Compiler<'_, '_, H> {
    pub(super) fn retained_constant_fields(
        &mut self,
        plan: &DatatypeSourcePlan,
    ) -> Result<Vec<(SchemaDeclarationNode, SchemaDeclarationNode)>, DatatypeCompilationIssue> {
        let declaration = plan.source.declaration();
        self.spend(0, declaration)?;
        if self.remaining == 0 {
            return Err(pending("datatype-work-limit", declaration));
        }
        let CemAstNode::Element {
            node_id, source, ..
        } = declaration.node()
        else {
            unreachable!()
        };
        let origin = ReferenceOccurrence {
            identity: format!("datatype-constants:{}", declaration.identity()),
            node_id: Some(*node_id),
            expression: None,
            source_map: source.clone(),
        };
        // Only the first entered node is the owning container. A selected type,
        // including this same type, is a terminal wrong target, never recursion.
        let mut container = true;
        let walk = resolve_owned_reference_structure(
            self.host.source_reference(declaration.clone()),
            self.host,
            ReferenceTraversalLimits {
                max_depth: self.max_depth,
                max_work: self.remaining,
            },
            origin,
            |host, _| {
                if !std::mem::take(&mut container) {
                    return None;
                }
                Some(
                    plan.constant_slots
                        .iter()
                        .map(|slot| host.source_reference(slot.clone()))
                        .collect(),
                )
            },
        )
        .map_err(|_| pending("datatype-constant-selection-unavailable", declaration))?;
        self.remaining = self.remaining.saturating_sub(walk.resolution.work_used);
        if walk.resolution.diagnostics.len() > self.preparation.validation.max_diagnostics {
            return Err(pending("datatype-constant-diagnostic-limit", declaration));
        }
        self.preparation.validation.max_diagnostics -= walk.resolution.diagnostics.len();
        self.diagnostics.extend(walk.resolution.diagnostics);
        for error in walk.resolution.issues {
            self.reference_issues.push(DatatypeReferenceIssue {
                source: self.host.declaration_node(&error.reference),
                kind: error.kind,
                occurrence: error.occurrence,
                reason: error.reason,
            });
        }
        self.spend(0, declaration)?;
        if walk.resolution.state != ReferenceResolutionState::Resolved || walk.resolution.failed {
            return Err(issue(
                "datatype-constant-selection-incomplete",
                if walk.resolution.failed {
                    DatatypeIssueState::Invalid
                } else {
                    DatatypeIssueState::Pending
                },
                declaration,
            ));
        }
        let Some(root) = walk
            .roots
            .first()
            .copied()
            .filter(|_| walk.roots.len() == 1)
        else {
            return Err(pending(
                "datatype-constant-selection-incomplete",
                declaration,
            ));
        };
        if walk.children[root].is_empty() {
            return Err(invalid("datatype-empty-vocabulary", declaration));
        }
        if walk.children[root].len() > self.preparation.max_constants {
            return Err(pending("datatype-constant-count-limit", declaration));
        }
        let mut fields = Vec::new();
        for index in &walk.children[root] {
            let target = self
                .host
                .declaration_node(&walk.resolution.nodes[*index])
                .ok_or_else(|| {
                    invalid("datatype-constant-retained-target-required", declaration)
                })?;
            let value = self.constant_field(&target)?;
            fields.push((value, target));
        }
        Ok(fields)
    }

    fn constant_field(
        &mut self,
        declaration: &SchemaDeclarationNode,
    ) -> Result<SchemaDeclarationNode, DatatypeCompilationIssue> {
        self.spend(1, declaration)?;
        if declaration
            .document()
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(invalid("invalid-datatype-constant-source", declaration));
        }
        let CemAstNode::Element {
            attributes,
            children,
            ..
        } = declaration.node()
        else {
            return Err(invalid(
                "datatype-constant-retained-target-required",
                declaration,
            ));
        };
        let name = self
            .host
            .input_expanded_name(declaration)
            .ok_or_else(|| pending("datatype-name-pending", declaration))?;
        if (!name.namespace_uri.is_empty() && name.namespace_uri != CEM_SCHEMA_URI)
            || name.local_name != "constant"
        {
            return Err(invalid("unsupported-datatype-child", declaration));
        }
        self.spend(attributes.len().saturating_add(children.len()), declaration)?;
        for id in children {
            let child = SchemaDeclarationNode::new(declaration.document().clone(), *id).unwrap();
            match child.node() {
                CemAstNode::Comment { .. } | CemAstNode::Whitespace { .. } => {}
                CemAstNode::Text { data, .. } if data.trim().is_empty() => {}
                _ => return Err(invalid("unsupported-datatype-constant-child", &child)),
            }
        }
        let mut value = None;
        for id in attributes {
            let field = SchemaDeclarationNode::new(declaration.document().clone(), *id).unwrap();
            if self.host.input_consumed_namespace_attribute(&field) {
                continue;
            }
            let name = self
                .host
                .input_expanded_name(&field)
                .ok_or_else(|| pending("datatype-name-pending", &field))?;
            if (!name.namespace_uri.is_empty() && name.namespace_uri != CEM_SCHEMA_URI)
                || name.local_name != "value"
            {
                return Err(invalid("unsupported-datatype-constant-field", &field));
            }
            if value.is_some() {
                return Err(invalid("duplicate-datatype-constant-field", &field));
            }
            if !matches!(field.node(), CemAstNode::Attribute { value: Some(_), value_nodes, .. } if value_nodes.is_empty())
            {
                return Err(invalid("datatype-constant-require-literal", &field));
            }
            value = Some(field);
        }
        value.ok_or_else(|| invalid("datatype-constant-value-required", declaration))
    }
}
