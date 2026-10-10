//! Caller-supplied original schema sources; no URI loading or execution registry.
use super::*;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
    source: ValueContractSource,
    behavior: SchemaDeclarationNode,
    function: SchemaDeclarationNode,
}
impl FunctionDeclaration {
    pub fn source(&self) -> &ValueContractSource {
        &self.source
    }
    pub fn behavior(&self) -> &SchemaDeclarationNode {
        &self.behavior
    }
    pub fn function(&self) -> &SchemaDeclarationNode {
        &self.function
    }
    fn metadata(
        &self,
        budget: &mut FunctionSelectionBudget,
    ) -> Result<(String, String), ValueContractError> {
        let attrs = fields(
            &self.source,
            &self.function,
            &["name", "visibility"],
            budget,
        )?;
        let name = required_name(&attrs, &self.function)?;
        let visibility = attrs
            .get("visibility")
            .map(literal)
            .transpose()?
            .unwrap_or("private");
        if !matches!(visibility, "private" | "package" | "public") {
            return Err(ValueContractError::at(
                "invalid-function-visibility",
                &self.function,
            ));
        }
        Ok((name, visibility.into()))
    }
    pub(super) fn check(
        &self,
        caller: &SchemaDeclarationNode,
        source: &ValueContractSource,
        budget: &mut FunctionSelectionBudget,
    ) -> Result<(), ValueContractError> {
        let (_, visibility) = self.metadata(budget)?;
        if self.behavior.identity() == caller.identity()
            || visibility == "public"
            || (visibility == "package"
                && source.schema.identity() == self.source.schema.identity())
        {
            Ok(())
        } else {
            Err(ValueContractError::at(
                "function-not-visible",
                &self.function,
            ))
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionCatalog {
    pub(super) sources: BTreeMap<String, ValueContractSource>,
    pub(super) behaviors: BTreeMap<String, ValueContractSource>,
    pub(super) functions: BTreeMap<String, FunctionDeclaration>,
    pub(super) incomplete: BTreeSet<String>,
    pub(super) members: BTreeMap<String, BTreeSet<String>>,
    pub(super) assembly_resolutions: Vec<DeclarationReferenceResolution>,
    exports: BTreeMap<(String, String), BTreeSet<String>>,
}
impl FunctionCatalog {
    /// Collect direct original declarations. References in behavior collections
    /// remain pending here; assembly/activation is a separate consumer stage.
    /// Sources carry the original lexical bindings, as with value contracts.
    pub fn collect(
        sources: &[ValueContractSource],
        budget: &mut FunctionSelectionBudget,
    ) -> Result<Self, ValueContractError> {
        let mut catalog = Self::default();
        for source in sources {
            budget.spend(1, &source.schema)?;
            budget.spend(source.schema.document().diagnostics.len(), &source.schema)?;
            if source.name(&source.schema).is_none() {
                return Err(ValueContractError::at(
                    "function-name-pending",
                    &source.schema,
                ));
            }
            if !source.named(&source.schema, "schema")
                || source.namespace.is_empty()
                || source
                    .schema
                    .document()
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation())
            {
                return Err(ValueContractError::at(
                    "invalid-function-source",
                    &source.schema,
                ));
            }
            let identity = source.schema.identity();
            if let Some(old) = catalog.sources.get(&identity) {
                if old.namespace != source.namespace || old.bindings != source.bindings {
                    return Err(ValueContractError::at(
                        "conflicting-function-source",
                        &source.schema,
                    ));
                }
                continue;
            }
            catalog.sources.insert(identity.clone(), source.clone());
            for collection in children(&source.schema, budget)? {
                if !matches!(collection.node(), CemAstNode::Element { .. }) {
                    continue;
                }
                if source.name(&collection).is_none() {
                    catalog.incomplete.insert(identity.clone());
                    continue;
                }
                if !source.named(&collection, "behaviors") {
                    continue;
                }
                for behavior in children(&collection, budget)? {
                    if matches!(behavior.node(), CemAstNode::Reference { .. }) {
                        catalog.incomplete.insert(identity.clone());
                        continue;
                    }
                    if !matches!(behavior.node(), CemAstNode::Element { .. }) {
                        continue;
                    }
                    if source.name(&behavior).is_none() {
                        catalog.incomplete.insert(identity.clone());
                        continue;
                    }
                    if !source.named(&behavior, "behavior") {
                        continue;
                    }
                    catalog
                        .behaviors
                        .insert(behavior.identity(), source.clone());
                    for function in children(&behavior, budget)? {
                        if source.named(&function, "function") {
                            catalog
                                .members
                                .entry(identity.clone())
                                .or_default()
                                .insert(function.identity());
                            catalog.functions.insert(
                                function.identity(),
                                FunctionDeclaration {
                                    source: source.clone(),
                                    behavior: behavior.clone(),
                                    function,
                                },
                            );
                        } else if matches!(function.node(), CemAstNode::Element { .. })
                            && source.name(&function).is_none()
                        {
                            catalog.incomplete.insert(identity.clone());
                        }
                    }
                }
            }
        }
        Ok(catalog)
    }
    /// Name export admission is explicit and distinct from a scope-crossing grant.
    pub fn export(
        &mut self,
        function: &SchemaDeclarationNode,
        budget: &mut FunctionSelectionBudget,
    ) -> Result<(), ValueContractError> {
        budget.spend(1, function)?;
        let declaration = self
            .functions
            .get(&function.identity())
            .ok_or_else(|| ValueContractError::at("function-source-unavailable", function))?;
        let (name, visibility) = declaration.metadata(budget)?;
        if visibility != "public" {
            return Err(ValueContractError::at(
                "function-export-not-public",
                function,
            ));
        }
        self.exports
            .entry((declaration.source.namespace.clone(), name))
            .or_default()
            .insert(function.identity());
        Ok(())
    }
    pub(super) fn source_for(
        &self,
        target: &SchemaDeclarationNode,
    ) -> Option<&ValueContractSource> {
        // Prefer the exact declaring schema when an arena contains several schemas.
        self.functions
            .get(&target.identity())
            .map(|f| &f.source)
            .or_else(|| {
                self.sources
                    .values()
                    .find(|s| Arc::ptr_eq(s.schema.document(), target.document()))
            })
    }
    pub(super) fn lookup(
        &self,
        source: &ValueContractSource,
        caller: &SchemaDeclarationNode,
        name: &str,
        slot: &SchemaDeclarationNode,
        budget: &mut FunctionSelectionBudget,
    ) -> Result<ReferenceLinkEvaluation<SchemaDeclarationNode>, ValueContractError> {
        let (namespace, local, qualified) = if let Some((prefix, local)) = name.split_once(':') {
            let Some(namespace) = source.bindings.get(prefix) else {
                return Ok(ReferenceLinkEvaluation::Unresolved(
                    "unknown-function-prefix".into(),
                ));
            };
            (namespace.as_str(), local, true)
        } else {
            (source.namespace.as_str(), name, false)
        };
        let same_schema = namespace == source.namespace;
        let mut own = vec![];
        let mut reusable = vec![];
        if same_schema && !qualified {
            for declaration in self.functions.values() {
                budget.spend(1, slot)?;
                if declaration.behavior.identity() != caller.identity() {
                    continue;
                }
                let (declared_name, _) = declaration.metadata(budget)?;
                if declared_name == local {
                    own.push(declaration.function.clone());
                }
            }
            if !own.is_empty() {
                return Ok(if own.len() == 1 {
                    ReferenceLinkEvaluation::Resolved(own)
                } else {
                    ReferenceLinkEvaluation::Invalid(vec![diagnostic(
                        slot,
                        "function-ambiguous",
                        None,
                    )])
                });
            }
        }
        for declaration in self.functions.values() {
            budget.spend(1, slot)?;
            if same_schema {
                if !self
                    .members
                    .get(&source.schema.identity())
                    .is_some_and(|members| members.contains(&declaration.function.identity()))
                {
                    continue;
                }
                let (declared_name, visibility) = declaration.metadata(budget)?;
                if declared_name != local {
                    continue;
                }
                if visibility != "private" {
                    reusable.push(declaration.function.clone());
                }
            } else if self
                .exports
                .get(&(namespace.into(), local.into()))
                .is_some_and(|exports| exports.contains(&declaration.function.identity()))
            {
                if self
                    .incomplete
                    .contains(&declaration.source.schema.identity())
                {
                    return Ok(ReferenceLinkEvaluation::Pending(
                        "function-collection-incomplete".into(),
                    ));
                }
                reusable.push(declaration.function.clone());
            }
        }
        if same_schema && self.incomplete.contains(&source.schema.identity()) {
            return Ok(ReferenceLinkEvaluation::Pending(
                "function-collection-incomplete".into(),
            ));
        }
        let candidates = reusable;
        if candidates.len() > 1 {
            return Ok(ReferenceLinkEvaluation::Invalid(vec![diagnostic(
                slot,
                "function-ambiguous",
                None,
            )]));
        }
        if !same_schema && candidates.is_empty() {
            return Ok(ReferenceLinkEvaluation::Unresolved(
                "function-export-unavailable".into(),
            ));
        }
        Ok(ReferenceLinkEvaluation::Resolved(candidates))
    }
}
