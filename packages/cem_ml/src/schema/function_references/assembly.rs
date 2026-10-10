//! Resolve admitted behavior collections before symbolic function lookup. The
//! original declaration membership and lexical source never change on reuse.
use super::*;

impl FunctionCatalog {
    pub fn assemble<H: SchemaDeclarationHost>(
        sources: &[ValueContractSource],
        host: &mut H,
        budget: &mut ScalarCompilationBudget,
    ) -> Result<Self, ValueContractError> {
        let mut catalog = Self::collect(sources, budget)?;
        let mut seen = std::collections::BTreeSet::new();
        for source in sources {
            if !seen.insert(source.schema.identity()) {
                continue;
            }
            let mut complete = true;
            for collection in children(&source.schema, budget)? {
                if matches!(collection.node(), CemAstNode::Element { .. })
                    && source.name(&collection).is_none()
                {
                    complete = false;
                }
                if !source.named(&collection, "behaviors") {
                    continue;
                }
                for member in children(&collection, budget)? {
                    if !matches!(member.node(), CemAstNode::Reference { .. }) {
                        if matches!(member.node(), CemAstNode::Element { .. })
                            && source.name(&member).is_none()
                        {
                            complete = false;
                        }
                        if source.named(&member, "behavior") {
                            for function in children(&member, budget)? {
                                if matches!(function.node(), CemAstNode::Element { .. })
                                    && source.name(&function).is_none()
                                {
                                    complete = false;
                                }
                            }
                        }
                        continue;
                    }
                    budget.spend(1, &member)?;
                    let resolved = resolve_reference(
                        host.source_reference(member.clone()),
                        host,
                        budget.remaining_limits(),
                    )
                    .map_err(|_| ValueContractError::at("function-collection-bounds", &member))?;
                    budget.spend(resolved.work_used, &member)?;
                    if resolved.state != ReferenceResolutionState::Resolved || resolved.failed {
                        complete = false;
                    }
                    let mut resolution = DeclarationReferenceResolution {
                        nodes: vec![],
                        state: resolved.state,
                        failed: resolved.failed,
                        issues: resolved
                            .issues
                            .into_iter()
                            .map(|issue| DeclarationReferenceIssue {
                                occurrence: issue.occurrence,
                                kind: issue.kind,
                                reason: issue.reason,
                            })
                            .collect(),
                        diagnostics: resolved.diagnostics,
                        work_used: resolved.work_used,
                    };
                    let evaluator_diagnostics = resolution.diagnostics.len();
                    for value in resolved.nodes {
                        let Some(target) = host.declaration_node(&value) else {
                            invalidate(
                                &mut resolution,
                                &member,
                                "function-collection-target",
                                None,
                            );
                            continue;
                        };
                        budget.spend(1, &target)?;
                        resolution.nodes.push(target.clone());
                        if !matches!(target.node(), CemAstNode::Element { .. })
                            || host.input_expanded_name(&target).is_some_and(|name| {
                                name.local_name != "behavior"
                                    || (!name.namespace_uri.is_empty()
                                        && name.namespace_uri
                                            != super::super::registry::CEM_SCHEMA_URI)
                            })
                        {
                            invalidate(
                                &mut resolution,
                                &member,
                                "function-collection-target",
                                Some(&target),
                            );
                            continue;
                        }
                        let Some(declaring) = catalog.behaviors.get(&target.identity()) else {
                            defer(
                                &mut resolution,
                                &occurrence(&member),
                                "function-behavior-source-unavailable",
                            );
                            continue;
                        };
                        if host
                            .declaration_schema(&target)
                            .as_ref()
                            .map(SchemaDeclarationNode::identity)
                            != Some(declaring.schema.identity())
                        {
                            invalidate(
                                &mut resolution,
                                &member,
                                "function-behavior-owner-mismatch",
                                Some(&target),
                            );
                            continue;
                        }
                        if let Err(error) = fields(declaring, &target, &["name"], budget)
                            .and_then(|fields| required_name(&fields, &target))
                        {
                            if error.code == "function-work-limit" {
                                return Err(error);
                            }
                            if error.code == "function-name-pending" {
                                defer(&mut resolution, &occurrence(&member), error.code);
                            } else {
                                invalidate(
                                    &mut resolution,
                                    &member,
                                    error.code,
                                    error.source.as_ref(),
                                );
                            }
                            continue;
                        }
                        for function in catalog.functions.values() {
                            budget.spend(1, &target)?;
                            if function.behavior().identity() == target.identity() {
                                catalog
                                    .members
                                    .entry(source.schema.identity())
                                    .or_default()
                                    .insert(function.function().identity());
                            }
                        }
                    }
                    for diagnostic in &mut resolution.diagnostics[evaluator_diagnostics..] {
                        *diagnostic = host.structural_diagnostic(&member, diagnostic.clone());
                    }
                    if resolution.state != ReferenceResolutionState::Resolved || resolution.failed {
                        complete = false;
                    }
                    catalog.assembly_resolutions.push(resolution);
                }
            }
            if complete {
                catalog.incomplete.remove(&source.schema.identity());
            }
        }
        Ok(catalog)
    }
    pub fn assembly_is_complete(&self) -> bool {
        self.incomplete.is_empty()
    }
    pub fn assembly_resolutions(&self) -> &[DeclarationReferenceResolution] {
        &self.assembly_resolutions
    }
}
