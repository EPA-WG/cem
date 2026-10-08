//! Invocation-owned source lifecycle admission for native element-ID consumers.
use super::{
    reference_transport::RetainedReferenceSource, StandaloneExpressionBinding,
    StandaloneExpressionContext,
};
use crate::{
    render::{
        ElementPlacementAdmission, ElementPlacementGrant, ElementPlacementSnapshot,
        ElementPlacementTransaction, ElementReferenceProjection, ElementReferenceProjectionError,
        RenderPlan, TemplateData,
    },
    schema_references::{CemQlSchemaDeclarationHost, DeclarationScope},
};
use cem_ml::{
    operation_control::{ExecutionScopeId, OperationControl},
    schema::reference_policy::ReferenceScopePolicy,
    value::artifact::CemValueArtifactLimits,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone)]
pub struct ElementReferenceSource {
    pub source: RetainedReferenceSource,
    /// Explicitly supply this invocation's template inputs; false means pending.
    pub context: bool,
    pub policy: ReferenceScopePolicy,
}
#[derive(Clone)]
pub struct ElementReferenceBinding {
    pub source: usize,
    pub name: String,
    /// Passive native query selection, never reference evaluation or ID lookup.
    pub select: String,
}
pub struct ElementReferenceExecution {
    host: CemQlSchemaDeclarationHost,
    requesting: DeclarationScope,
    placements: ElementPlacementSnapshot,
}
#[derive(Clone)]
pub struct ElementPlacementSelection {
    pub source: usize,
    pub select: String,
    pub token: String,
    pub producer: String,
    pub path: Vec<usize>,
    pub revision: String,
    pub id: String,
}
#[derive(Clone, Default)]
pub struct ElementPlacementInputs {
    pub admissions: Vec<ElementPlacementSelection>,
    pub grants: Vec<ElementPlacementGrant>,
    pub committed_revisions: BTreeMap<String, String>,
    pub prepared_transaction: Option<ElementPlacementTransaction>,
}
fn context(data: &TemplateData) -> StandaloneExpressionContext {
    StandaloneExpressionContext {
        bindings: data
            .bindings
            .iter()
            .map(|(name, values)| {
                (
                    name.clone(),
                    StandaloneExpressionBinding::any(values.clone()),
                )
            })
            .collect(),
        native_functions: data.native_functions.clone(),
        ..Default::default()
    }
}
impl ElementReferenceExecution {
    pub(crate) fn complete_selected_names(
        &mut self,
        sources: &[ElementReferenceSource],
        data: &TemplateData,
        values: crate::eval::ItemStream,
    ) -> Result<crate::eval::ItemStream, String> {
        use cem_ml::value::reference_resolution::ReferenceResolutionHost;
        use std::{
            collections::{BTreeMap, BTreeSet},
            sync::Arc,
        };
        let root = crate::eval::values::reference(Vec::new());
        let host = self
            .host
            .element_reference_host(self.requesting)
            .ok_or("Unknown requester")?;
        let request = host.scope_limits(&host.scope(&root));
        let frame = context(data);
        let owner_key =
            |tree: &cem_ml::parser::tree::RetainedCemTree| Arc::as_ptr(tree.ast_owner()) as usize;
        let mut grouped = BTreeMap::<usize, BTreeSet<cem_ml::parser::AstNodeId>>::new();
        let mut remaining = request.max_work;
        for node in values
            .items
            .iter()
            .filter_map(crate::eval::retained_cem_node)
        {
            remaining = remaining
                .checked_sub(1)
                .ok_or("Native name preparation work limit exceeded")?;
            grouped
                .entry(owner_key(node.owner()))
                .or_default()
                .insert(node.node_id());
        }
        let mut views = BTreeMap::new();
        for source in sources {
            let tree = source.source.ingress().source();
            let Some(roots) = grouped.remove(&owner_key(tree)) else {
                continue;
            };
            // Complete the owning forest once, while retaining selection order/duplicates.
            let mut forest_roots = Vec::new();
            for root in &roots {
                let mut parent = tree.source_parent(*root);
                let mut contained = false;
                while let Some(id) = parent {
                    remaining = remaining
                        .checked_sub(1)
                        .ok_or("Native name preparation work limit exceeded")?;
                    if roots.contains(&id) {
                        contained = true;
                        break;
                    }
                    parent = tree.source_parent(id);
                }
                if !contained {
                    forest_roots.push(*root);
                }
            }
            let limits = cem_ml::schema::reference_traversal::ReferenceTraversalLimits {
                max_depth: request.max_depth.min(source.policy.limits.max_depth),
                max_work: remaining.min(source.policy.limits.max_work),
            };
            let (names, ()) = self
                .host
                .with_namespace_lifecycle(
                    source
                        .source
                        .require_lexical()
                        .map_err(|e| format!("{e:?}"))?
                        .clone(),
                    &forest_roots,
                    limits,
                    |_, _, _, _| (source.context.then(|| frame.clone()), Default::default()),
                    |_, _| (),
                )
                .map_err(|e| format!("Native source name preparation: {e:?}"))?;
            if !names.is_complete() {
                return Err(
                    "cem.capability.source_incomplete: source namespaces are pending".into(),
                );
            }
            remaining = remaining
                .checked_sub(names.work_used)
                .ok_or("Native name preparation work limit exceeded")?;
            views.insert(
                owner_key(tree),
                crate::namespace_names::NamespaceQueryTree::new(tree.clone(), names.completion)?,
            );
        }
        let mut result = Vec::new();
        for item in values.items {
            if let Some(node) = crate::eval::retained_cem_node(&item) {
                let view = views
                    .get(&owner_key(node.owner()))
                    .ok_or("Selected source owner is not retained")?;
                result.push(
                    view.node(node.node_id())
                        .ok_or("Selected source names are not ready")?,
                );
            } else {
                result.push(item);
            }
        }
        Ok(crate::eval::ItemStream::from_items(result))
    }
    pub(crate) fn admit(&self, values: &crate::eval::ItemStream) -> Result<(), String> {
        for item in &values.items {
            if item
                .view()
                .is_some_and(|v| v.kind() == crate::eval::QueryItemViewKind::Node)
                && !self.host.admits_reference_target(self.requesting, item)
            {
                return Err("cem.capability.source_denied: source crossing has no grant".into());
            }
        }
        Ok(())
    }
    /// Shared consumer traversal, independent of DOM placement/ID projection.
    /// Owning containment groups all selected slots without adding reference depth.
    pub(crate) fn consume(
        &mut self,
        values: crate::eval::ItemStream,
    ) -> Result<crate::eval::ItemStream, String> {
        use cem_ml::value::reference_resolution::resolve_owned_reference_structure;
        let root =
            crate::eval::output::output_nodes(vec![crate::render::RenderPlanNode::Element {
                tag: "source-selection".into(),
                namespace: None,
                qualified_name: None,
                attributes: vec![],
                children: vec![],
                source_map: Default::default(),
            }])
            .items
            .remove(0);
        let identity = root.view().expect("native container").identity();
        let mut host = self
            .host
            .element_reference_host(self.requesting)
            .ok_or("Unknown requester")?;
        use cem_ml::value::reference_resolution::ReferenceResolutionHost;
        let limits = host.scope_limits(&host.scope(&root));
        let walk = resolve_owned_reference_structure(
            root,
            &mut host,
            limits,
            cem_ml::schema::reference_policy::ReferenceOccurrence {
                identity: identity.clone(),
                node_id: None,
                expression: None,
                source_map: Default::default(),
            },
            |_, item| {
                item.view()
                    .is_some_and(|v| v.identity() == identity)
                    .then(|| values.items.clone())
            },
        )
        .map_err(|e| e.to_string())?;
        let result = &walk.resolution;
        if !result.is_complete() {
            return Err(format!(
                "cem.capability.source_incomplete: {:?}: {}",
                result.state,
                result
                    .issues
                    .iter()
                    .map(|i| i.reason.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        Ok(crate::eval::ItemStream::from_items(
            walk.roots
                .iter()
                .flat_map(|root| &walk.children[*root])
                .map(|index| result.nodes[*index].clone())
                .collect(),
        ))
    }
    /// Build a fresh host over original retained owners and captures. Query
    /// bindings are staged atomically and contexts never enter source metadata.
    pub fn prepare(
        sources: Vec<ElementReferenceSource>,
        requesting: usize,
        bindings: &[ElementReferenceBinding],
        grants: &[(usize, usize)],
        data: &mut TemplateData,
    ) -> Result<Self, String> {
        Self::prepare_with_placements(
            sources,
            requesting,
            bindings,
            grants,
            ElementPlacementInputs::default(),
            data,
        )
    }
    pub fn prepare_with_placements(
        sources: Vec<ElementReferenceSource>,
        requesting: usize,
        bindings: &[ElementReferenceBinding],
        grants: &[(usize, usize)],
        placements: ElementPlacementInputs,
        data: &mut TemplateData,
    ) -> Result<Self, String> {
        let source = sources
            .get(requesting)
            .ok_or("Unknown requesting reference source")?;
        let ceiling = source.policy.limits.max_work;
        if sources
            .len()
            .saturating_add(bindings.len())
            .saturating_add(grants.len())
            .saturating_add(placements.admissions.len())
            .saturating_add(placements.grants.len())
            .saturating_add(placements.committed_revisions.len())
            > ceiling
        {
            return Err("Element reference metadata work limit exceeded".into());
        }
        let mut owners = BTreeSet::new();
        for source in &sources {
            source
                .source
                .require_lexical()
                .map_err(|e| format!("Missing source capture: {e:?}"))?;
            if source.policy.limits.max_work == 0 || source.policy.limits.max_depth == 0 {
                return Err("Invalid reference source bounds".into());
            }
            let identity =
                std::sync::Arc::as_ptr(source.source.ingress().source().ast_owner()) as usize;
            if !owners.insert(identity) {
                return Err("Duplicate reference source owner".into());
            }
        }
        let mut names = BTreeSet::new();
        for binding in bindings {
            if sources.get(binding.source).is_none()
                || binding.name.is_empty()
                || matches!(
                    binding.name.as_str(),
                    "datadom" | "input" | "context" | "attributes" | "slices" | "instanceID"
                )
                || !names.insert(&binding.name)
            {
                return Err("Invalid or duplicate reference source binding".into());
            }
        }
        for &(from, to) in grants {
            if sources.get(from).is_none() || sources.get(to).is_none() {
                return Err("Unknown reference crossing source".into());
            }
        }
        let selection_context = context(data);
        let mut staged = data.clone();
        for binding in bindings {
            let values = sources[binding.source]
                .source
                .evaluate(&binding.select, &selection_context)
                .map_err(|e| format!("{}: {}", e.code, e.message))?
                .result;
            if let Some(error) = &values.error {
                return Err(format!(
                    "Reference source selection `{}` failed: {error:?}",
                    binding.name
                ));
            }
            if values.items.len() > ceiling {
                return Err("Reference source selection exceeded bounds".into());
            }
            staged.bind_native_slice(&binding.name, values)?;
        }
        let frame = context(&staged);
        let mut snapshot = ElementPlacementSnapshot {
            admissions: Vec::new(),
            grants: placements.grants,
            committed_revisions: placements.committed_revisions,
            prepared_transaction: placements.prepared_transaction,
        };
        for admission in placements.admissions {
            let source = sources
                .get(admission.source)
                .ok_or("Unknown placement source")?;
            let selected = source
                .source
                .evaluate(&admission.select, &selection_context)
                .map_err(|e| format!("{}: {}", e.code, e.message))?
                .result;
            if selected.error.is_some()
                || selected.items.len() != 1
                || crate::eval::retained_cem_node(&selected.items[0]).is_none()
            {
                return Err("Placement admission must select exactly one original node".into());
            }
            snapshot.admissions.push(ElementPlacementAdmission {
                target: selected.items[0].clone(),
                token: admission.token,
                producer: admission.producer,
                path: admission.path,
                revision: admission.revision,
                id: admission.id,
            });
        }
        let mut host = CemQlSchemaDeclarationHost::new();
        let mut scopes = Vec::new();
        for source in &sources {
            let context = source.context.then(|| frame.clone());
            let scope = host.register_scope(
                source.source.ingress().source().clone(),
                context.clone(),
                source.policy.clone(),
            );
            scopes.push(scope);
            let capture = source.source.require_lexical().expect("validated capture");
            host.attach_captured_namespaces(capture.clone())
                .map_err(|e| format!("{e:?}"))?;
            host.attach_captured_lexical_scopes_with_policy_overrides(capture, |_, _, _| {
                (context.clone(), Default::default())
            })
            .map_err(|e| format!("{e:?}"))?;
        }
        for &(from, to) in grants {
            host.allow_scope_crossing(scopes[from], scopes[to]);
        }
        *data = staged;
        Ok(Self {
            host,
            requesting: scopes[requesting],
            placements: snapshot,
        })
    }
    pub fn project(
        self,
        plan: &RenderPlan,
        instance: &str,
        limits: &CemValueArtifactLimits,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<RenderPlan, ElementReferenceProjectionError> {
        self.project_with_placements(plan, instance, limits, control, scope)
            .map(|result| result.plan)
    }
    pub fn project_with_placements(
        self,
        plan: &RenderPlan,
        instance: &str,
        limits: &CemValueArtifactLimits,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<ElementReferenceProjection, ElementReferenceProjectionError> {
        self.project_with_options(plan, instance, limits, control, scope, Default::default())
    }
    pub fn project_with_options(
        mut self,
        plan: &RenderPlan,
        instance: &str,
        limits: &CemValueArtifactLimits,
        control: &OperationControl,
        scope: ExecutionScopeId,
        options: crate::render::ElementReferenceExportOptions,
    ) -> Result<ElementReferenceProjection, ElementReferenceProjectionError> {
        crate::render::project_element_reference_ids_with_host_and_placements_with_options(
            plan,
            instance,
            limits,
            control,
            scope,
            &mut self
                .host
                .element_reference_host(self.requesting)
                .expect("admitted requester"),
            &self.placements,
            options,
        )
    }
}
