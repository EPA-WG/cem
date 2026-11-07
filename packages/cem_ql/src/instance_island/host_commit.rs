//! Atomic native host-attribute/canonical-island staging. No source mutation.
use super::{
    construct::{attribute, element, PREFIXES},
    *,
};
use crate::{
    eval::{
        output::{output_nodes_with_source, OutputView},
        RetainedCemNode,
    },
    render::{HostAttributeUpdate, RenderPlanAttribute, RenderPlanNode},
};
use std::sync::Arc;
#[derive(Debug)]
pub struct CommittedHostIsland {
    template: Item,
    root: Item,
    bindings: TemplateData,
    writes: Arc<Vec<RenderPlanAttribute>>,
}
impl CommittedHostIsland {
    pub fn template(&self) -> &Item {
        &self.template
    }
    pub fn context_root(&self) -> &Item {
        &self.root
    }
    pub fn bindings(&self) -> &TemplateData {
        &self.bindings
    }
    pub fn writes(&self) -> &Arc<Vec<RenderPlanAttribute>> {
        &self.writes
    }
}
const REFUSED: &str = "native host/island commit refused";
fn children_mut(node: &mut RenderPlanNode) -> Result<&mut Vec<RenderPlanNode>, String> {
    match node {
        RenderPlanNode::Element { children, .. } => Ok(children),
        _ => Err(REFUSED.into()),
    }
}
fn literal(node: &RenderPlanNode, name: &str) -> Result<String, String> {
    let RenderPlanNode::Element { attributes, .. } = node else {
        return Err(REFUSED.into());
    };
    attributes
        .iter()
        .find(|a| a.name == name && a.namespace.is_none())
        .map(|a| a.value.clone())
        .ok_or(REFUSED.into())
}
/// The first profile consumes the shared constructor's canonical native template.
/// Writes are coalesced in order; no host or input-island owner is mutated.
pub fn prepare_host_commit(
    template: &Item,
    host: RetainedCemNode,
    instance_id: &str,
    updates: &[HostAttributeUpdate],
    control: &OperationControl,
) -> Result<CommittedHostIsland, String> {
    prepare_host_commit_with_source(template, host, instance_id, updates, control, None)
}

/// Consume an explicitly admitted source owner without starting a source compiler.
pub fn prepare_host_commit_with_source(
    template: &Item,
    host: RetainedCemNode,
    instance_id: &str,
    updates: &[HostAttributeUpdate],
    control: &OperationControl,
    source: Option<&Arc<cem_ml::import::element_tree::RetainedCemPayloadSource>>,
) -> Result<CommittedHostIsland, String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.code().to_string())?;
    if updates.len() > LIMITS.max_values {
        return Err(BOUNDS.into());
    }
    let root = children(template)?.into_iter().next().ok_or(REFUSED)?;
    let previous = bind_retained_root(&root, instance_id, control, source)?;
    let native = template
        .view()
        .and_then(|v| v.downcast_ref::<OutputView>())
        .ok_or(REFUSED)?;
    let anchor = native.derivation_source().ok_or(REFUSED)?;
    if !Arc::ptr_eq(anchor.owner(), host.owner()) || anchor.node_id() != host.node_id() {
        return Err(REFUSED.into());
    }
    let mut node = native.node().clone();
    let roots = children_mut(&mut node)?;
    if roots.len() != 1 {
        return Err(REFUSED.into());
    }
    let parts = children_mut(&mut roots[0])?;
    if parts.len() != PARTS.len() {
        return Err(REFUSED.into());
    }
    let tree = host.owner();
    let source = tree.node(host.node_id()).ok_or(REFUSED)?;
    if source
        .name
        .as_ref()
        .is_none_or(|name| name.namespace_uri != cem_ml::schema::registry::HTML_NAMESPACE_URI)
    {
        return Err(REFUSED.into());
    }
    let mut attributes = vec![];
    for id in &source.attributes {
        let a = tree.node(*id).ok_or(REFUSED)?;
        let name = a.name.as_ref().ok_or(REFUSED)?;
        if !name.namespace_uri.is_empty() {
            return Err(REFUSED.into());
        }
        attributes.push(attribute(
            &name.local_name,
            &a.value,
            &tree.node_source_map(*id).unwrap_or_default(),
        ));
    }
    let actual = attributes
        .iter()
        .filter(|a| a.name != "data-cem-render-scope")
        .map(|a| (a.name.clone(), a.value.clone()))
        .collect::<Vec<_>>();
    let asserted = children_mut(&mut parts[1])?
        .iter()
        .map(|a| Ok((literal(a, "name")?, literal(a, "value")?)))
        .collect::<Result<Vec<_>, String>>()?;
    if actual != asserted {
        return Err(REFUSED.into());
    }
    let policy = control
        .scope_tree()
        .scope(ROOT_EXECUTION_SCOPE_ID)
        .ok_or(REFUSED)?
        .effective_policy;
    let ceiling = LIMITS
        .max_bytes
        .min(policy.memory_bytes.try_into().unwrap_or(usize::MAX));
    let mut bytes = 0usize;
    let mut names = vec![];
    for index in coalesced_host_writes(updates, control)? {
        let update = &updates[index];
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_string())?;
        bytes = bytes
            .saturating_add(update.name.len())
            .saturating_add(update.value.len());
        if bytes > ceiling {
            return Err(BOUNDS.into());
        }
        if update.source_map().frames.is_empty()
            || update.name.starts_with("data-cem-")
            || update.name == "xmlns"
        {
            return Err(REFUSED.into());
        }
        cem_ml::validation::html::validate_passive_html_attributes(
            &source.name.as_ref().ok_or(REFUSED)?.local_name,
            &[(&update.name, &update.value)],
        )
        .map_err(|_| REFUSED)?;
        let a = attribute(&update.name, &update.value, update.source_map());
        if let Some(index) = attributes.iter().position(|a| a.name == update.name) {
            if attributes[index].value != update.value {
                attributes[index] = a;
            }
        } else {
            attributes.push(a);
        }
        if !names.contains(&update.name) {
            names.push(update.name.clone());
        }
        if attributes.len() > 64 {
            return Err(BOUNDS.into());
        }
    }
    let writes = names
        .into_iter()
        .filter_map(|name| {
            let next = attributes.iter().find(|a| a.name == name).unwrap();
            let unchanged = actual.iter().any(|(n, v)| n == &name && v == &next.value);
            (!unchanged).then(|| next.clone())
        })
        .collect::<Vec<_>>();
    if writes.is_empty() {
        return Ok(CommittedHostIsland {
            template: template.clone(),
            root,
            bindings: previous,
            writes: Arc::new(writes),
        });
    }
    let origin = tree.node_source_map(host.node_id()).unwrap_or_default();
    let mut canonical = vec![];
    let mut dataset = vec![];
    for a in &attributes {
        if a.name == "data-cem-render-scope" {
            continue;
        }
        canonical.push(element(
            "attribute",
            PARTS[1].1,
            Some(PREFIXES[2]),
            vec![
                attribute("name", &a.name, &a.source_map),
                attribute("value", &a.value, &a.source_map),
            ],
            vec![],
            &a.source_map,
        ));
        if let Some(suffix) = a.name.strip_prefix("data-") {
            dataset.push(element(
                "entry",
                PARTS[2].1,
                Some(PREFIXES[3]),
                vec![
                    attribute("key", &dataset_key(suffix), &a.source_map),
                    attribute("value", &a.value, &a.source_map),
                ],
                vec![],
                &a.source_map,
            ));
        }
    }
    parts[1] = element(
        "attributes",
        PARTS[1].1,
        Some(PREFIXES[2]),
        vec![],
        canonical,
        &origin,
    );
    parts[2] = element(
        "dataset",
        PARTS[2].1,
        Some(PREFIXES[3]),
        vec![],
        dataset,
        &origin,
    );
    let template = output_nodes_with_source(vec![node], host).items.remove(0);
    let root = children(&template)?.remove(0);
    let bindings = bind_retained_root(
        &root,
        instance_id,
        control,
        previous.native_payload_source.as_ref(),
    )?;
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.code().to_string())?;
    Ok(CommittedHostIsland {
        template,
        root,
        bindings,
        writes: Arc::new(writes),
    })
}

/// Named host-publication control metadata; never an AST or source-owner grant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostAttributeCommit {
    pub version: u8,
    pub writes: Vec<HostAttributeWrite>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostAttributeWrite {
    pub name: String,
    pub value: String,
    pub dataset_key: Option<String>,
}
pub fn prepare_host_write_control(
    updates: &[HostAttributeUpdate],
    control: &OperationControl,
) -> Result<HostAttributeCommit, String> {
    let indices = coalesced_host_writes(updates, control)?;
    Ok(HostAttributeCommit {
        version: 1,
        writes: indices
            .into_iter()
            .map(|index| {
                let update = &updates[index];
                let name = update.name.to_ascii_lowercase();
                let dataset_key = name.strip_prefix("data-").map(super::dataset_key);
                HostAttributeWrite {
                    name,
                    value: update.value.clone(),
                    dataset_key,
                }
            })
            .collect(),
    })
}
fn coalesced_host_writes(
    updates: &[HostAttributeUpdate],
    control: &OperationControl,
) -> Result<Vec<usize>, String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|e| e.code().to_string())?;
    if updates.len() > LIMITS.max_values {
        return Err(BOUNDS.into());
    }
    let policy = control
        .scope_tree()
        .scope(ROOT_EXECUTION_SCOPE_ID)
        .ok_or(REFUSED)?
        .effective_policy;
    let ceiling = LIMITS
        .max_bytes
        .min(policy.memory_bytes.try_into().unwrap_or(usize::MAX));
    let mut bytes = 0usize;
    let mut indices = Vec::<usize>::new();
    for (index, update) in updates.iter().enumerate() {
        control
            .check_scope(ROOT_EXECUTION_SCOPE_ID)
            .map_err(|e| e.code().to_string())?;
        bytes = bytes
            .saturating_add(update.name.len())
            .saturating_add(update.value.len());
        if bytes > ceiling {
            return Err(BOUNDS.into());
        }
        let name = update.name.to_ascii_lowercase();
        if name.is_empty() || name.starts_with("data-cem-") || name == "xmlns" {
            return Err(REFUSED.into());
        }
        if let Some(position) = indices
            .iter()
            .position(|previous| updates[*previous].name.eq_ignore_ascii_case(&name))
        {
            indices[position] = index;
        } else {
            indices.push(index);
        }
        if indices.len() > 64 {
            return Err(BOUNDS.into());
        }
    }
    Ok(indices)
}

#[cfg(test)]
mod write_control_tests {
    use super::*;
    #[test]
    fn html_case_folded_data_writes_feed_the_same_pass_dataset() {
        let result = crate::render::render_template(
            "{attribute @name=DATA-mode | native}{p | {$ datadom.dataset.mode}}",
            &Default::default(),
        );
        assert_eq!(result.rendered, "<p>native</p>");
    }

    #[test]
    fn coalesces_html_names_in_first_encounter_order_with_native_dataset_keys() {
        let c = OperationControl::default();
        let updates = vec![
            HostAttributeUpdate::new("DATA-user--id", "first"),
            HostAttributeUpdate::new("label", ""),
            HostAttributeUpdate::new("data-user--id", "last"),
        ];
        let p = prepare_host_write_control(&updates, &c).unwrap();
        assert_eq!(p.version, 1);
        assert_eq!(p.writes.len(), 2);
        assert_eq!(p.writes[0].name, "data-user--id");
        assert_eq!(p.writes[0].value, "last");
        assert_eq!(p.writes[0].dataset_key.as_deref(), Some("user-Id"));
        assert_eq!(p.writes[1].dataset_key, None);
        assert_eq!(p.writes[1].value, "");
    }
    #[test]
    fn bounds_and_original_control_are_preserved() {
        let c = OperationControl::default();
        assert!(prepare_host_write_control(
            &[HostAttributeUpdate::new(
                "label",
                "x".repeat(LIMITS.max_bytes + 1)
            )],
            &c
        )
        .is_err());
        let c = OperationControl::default();
        c.abort_signal().abort();
        assert_eq!(
            prepare_host_write_control(&[], &c).unwrap_err(),
            "host-cancellation"
        );
    }
    #[test]
    fn empty_commit_is_explicit_and_roundtrips_as_named_control() {
        let c = OperationControl::default();
        let p = prepare_host_write_control(&[], &c).unwrap();
        let s = serde_json::to_string(&p).unwrap();
        assert_eq!(s, "{\"version\":1,\"writes\":[]}");
    }
}
