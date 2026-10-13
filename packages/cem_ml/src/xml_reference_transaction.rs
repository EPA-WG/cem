//! Native prepared evidence for one XML source edit and its supplied occurrences.
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (XmlSourceSnapshot, Vec<XmlOutputSnapshot>, String, u32) {
        let source = XmlSourceSnapshot {
            uri: "https://example.test/a.svg".into(),
            revision: "source-1".into(),
            bytes: br#"<svg xmlns="http://www.w3.org/2000/svg"><image href="old.svg"/></svg>"#
                .to_vec(),
        };
        let inventory = crate::document_reference_provenance::inspect_xml_reference_provenance(
            &source.bytes,
            &source.uri,
            100,
        )
        .unwrap();
        let key = inventory.references[0].source_key.clone().unwrap();
        let attribute = inventory.references[0].attribute;
        let outputs = vec![XmlOutputSnapshot {
            id: "preview".into(),
            revision: "output-1".into(),
            tree: inventory.document.tree,
        }];
        (source, outputs, key, attribute)
    }
    fn request<'a>(
        source: &'a XmlSourceSnapshot,
        outputs: &'a [XmlOutputSnapshot],
        key: &'a str,
        scope: XmlEditScope,
    ) -> XmlEditTransactionRequest<'a> {
        XmlEditTransactionRequest {
            source,
            outputs,
            source_key: key,
            to_uri: "https://example.test/new.svg",
            policy_stamp: "policy-1",
            scope,
            budget: 100,
        }
    }
    #[test]
    fn xml_edit_transaction_requires_explicit_scope_for_shared_sources() {
        let (source, mut outputs, key, attr) = fixture();
        outputs.push(XmlOutputSnapshot {
            id: "second-preview".into(),
            revision: "output-2".into(),
            tree: outputs[0].tree.clone(),
        });
        assert!(prepare_xml_edit_transaction(request(
            &source,
            &outputs,
            &key,
            XmlEditScope::Occurrence {
                output_id: "preview".into(),
                attribute: attr
            }
        ))
        .is_err());
        let plan = prepare_xml_edit_transaction(request(
            &source,
            &outputs,
            &key,
            XmlEditScope::SourceWide,
        ))
        .unwrap();
        assert_eq!(plan.affected_occurrences().len(), 2);
        outputs.reverse();
        plan.validate_current(&source, &outputs, "policy-1")
            .unwrap();
        assert_ne!(plan.candidate_bytes(), source.bytes);
        assert_eq!(plan.original_bytes(), source.bytes);
        assert!(plan.body_analysis_required());
    }
    #[test]
    fn xml_edit_transaction_rejects_unknown_and_changed_output_provenance() {
        let (source, mut outputs, key, _) = fixture();
        outputs[0].tree = RetainedCemTree::from_shared(
            outputs[0].tree.ast_owner().clone(),
            &source.uri,
            "",
            Default::default(),
            None,
        )
        .unwrap();
        assert!(prepare_xml_edit_transaction(request(
            &source,
            &outputs,
            &key,
            XmlEditScope::SourceWide
        ))
        .is_err());
        let changed = String::from_utf8(source.bytes.clone())
            .unwrap()
            .replace("old.svg", "changed.svg");
        outputs[0].tree = crate::import::import_data_bytes(
            changed.as_bytes(),
            "application/xml",
            "cem",
            &source.uri,
        )
        .unwrap();
        assert!(prepare_xml_edit_transaction(request(
            &source,
            &outputs,
            &key,
            XmlEditScope::SourceWide
        ))
        .is_err());
    }
    #[test]
    fn xml_edit_transaction_rejects_stale_read_sets() {
        let (mut source, mut outputs, key, _) = fixture();
        let plan = prepare_xml_edit_transaction(request(
            &source,
            &outputs,
            &key,
            XmlEditScope::SourceWide,
        ))
        .unwrap();
        assert!(plan
            .validate_current(&source, &outputs, "policy-2")
            .is_err());
        assert!(plan.validate_current(&source, &[], "policy-1").is_err());
        source.revision = "source-2".into();
        assert!(plan
            .validate_current(&source, &outputs, "policy-1")
            .is_err());
        source.revision = "source-1".into();
        source.bytes.push(b' ');
        assert!(plan
            .validate_current(&source, &outputs, "policy-1")
            .is_err());
        source.bytes.pop();
        outputs[0].revision = "output-2".into();
        assert!(plan
            .validate_current(&source, &outputs, "policy-1")
            .is_err());
        outputs[0].revision = "output-1".into();
        outputs[0].tree =
            crate::import::import_data_bytes(&source.bytes, "application/xml", "cem", &source.uri)
                .unwrap();
        assert!(plan
            .validate_current(&source, &outputs, "policy-1")
            .is_err());
    }
    #[test]
    fn xml_edit_transaction_rejects_duplicate_missing_and_unbounded_occurrences() {
        let (source, mut outputs, key, attr) = fixture();
        let plan = prepare_xml_edit_transaction(request(
            &source,
            &outputs,
            &key,
            XmlEditScope::Occurrence {
                output_id: "preview".into(),
                attribute: attr,
            },
        ))
        .unwrap();
        assert_eq!(plan.affected_occurrences().len(), 1);
        let mut bounded = request(&source, &outputs, &key, XmlEditScope::SourceWide);
        bounded.budget = 0;
        assert!(prepare_xml_edit_transaction(bounded).is_err());
        assert!(prepare_xml_edit_transaction(request(
            &source,
            &[],
            &key,
            XmlEditScope::SourceWide
        ))
        .is_err());
        outputs.push(outputs[0].clone());
        assert!(prepare_xml_edit_transaction(request(
            &source,
            &outputs,
            &key,
            XmlEditScope::SourceWide
        ))
        .is_err());
    }
}

use crate::parser::{
    tree::{CemTreeNodeKind, RetainedCemTree},
    AstNodeId,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone)]
pub struct XmlSourceSnapshot {
    pub uri: String,
    pub revision: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone)]
pub struct XmlOutputSnapshot {
    pub id: String,
    pub revision: String,
    pub tree: Arc<RetainedCemTree>,
}
pub enum XmlEditScope {
    Occurrence {
        output_id: String,
        attribute: AstNodeId,
    },
    SourceWide,
}
pub struct XmlEditTransactionRequest<'a> {
    pub source: &'a XmlSourceSnapshot,
    pub outputs: &'a [XmlOutputSnapshot],
    pub source_key: &'a str,
    pub to_uri: &'a str,
    pub policy_stamp: &'a str,
    pub scope: XmlEditScope,
    pub budget: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAffectedOccurrence {
    pub output_id: String,
    pub attribute: AstNodeId,
}
/// Immutable native evidence, not a project commit or an authorization token.
/// The host owns completeness of the supplied output membership and must validate
/// this read set inside its atomic commit boundary, after dependency admission.
pub struct PreparedXmlEditTransaction {
    source: XmlSourceSnapshot,
    outputs: Vec<XmlOutputSnapshot>,
    policy_stamp: String,
    affected: Vec<XmlAffectedOccurrence>,
    candidate: crate::import::xml_reference_edit::XmlReferenceEditPlan,
}
impl PreparedXmlEditTransaction {
    pub fn candidate_bytes(&self) -> &[u8] {
        &self.candidate.rewritten_bytes
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.source.bytes
    }
    pub fn affected_occurrences(&self) -> &[XmlAffectedOccurrence] {
        &self.affected
    }
    pub fn body_analysis_required(&self) -> bool {
        true
    }
    pub fn blockers(&self) -> &[&str] {
        &[
            "runtime-dependency-closure-required",
            "host-output-membership-completeness-required",
            "atomic-host-transaction-required",
        ]
    }
    /// Revalidate exact source bytes/revision, output set/revisions/native owners,
    /// and host policy identity. Success is a read-set match, never edit permission.
    pub fn validate_current(
        &self,
        source: &XmlSourceSnapshot,
        outputs: &[XmlOutputSnapshot],
        policy_stamp: &str,
    ) -> Result<(), String> {
        if source.uri != self.source.uri
            || source.revision != self.source.revision
            || source.bytes != self.source.bytes
        {
            return Err("source snapshot changed".into());
        }
        if policy_stamp != self.policy_stamp {
            return Err("execution policy changed".into());
        }
        validate_outputs(outputs)?;
        if outputs.len() != self.outputs.len() {
            return Err("output membership changed".into());
        }
        for expected in &self.outputs {
            let current = outputs
                .iter()
                .find(|o| o.id == expected.id)
                .ok_or("output membership changed")?;
            if current.revision != expected.revision || !Arc::ptr_eq(&current.tree, &expected.tree)
            {
                return Err("output revision or native owner changed".into());
            }
        }
        Ok(())
    }
}
fn validate_outputs(outputs: &[XmlOutputSnapshot]) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for output in outputs {
        if output.id.trim().is_empty()
            || output.revision.trim().is_empty()
            || !ids.insert(output.id.as_str())
        {
            return Err("unique output identities and revisions required".into());
        }
    }
    Ok(())
}
/// Only source-preserving native output attributes are admitted in this first
/// bounded profile. Unknown attribute provenance and opaque reference nodes block
/// preparation; they cannot silently hide another occurrence of the selected slot.
pub fn prepare_xml_edit_transaction(
    request: XmlEditTransactionRequest<'_>,
) -> Result<PreparedXmlEditTransaction, String> {
    let XmlEditTransactionRequest {
        source,
        outputs,
        source_key,
        to_uri,
        policy_stamp,
        scope,
        budget,
    } = request;
    if source.revision.trim().is_empty() || policy_stamp.trim().is_empty() {
        return Err("source revision and policy identity required".into());
    }
    validate_outputs(outputs)?;
    let inventory = crate::document_reference_provenance::inspect_xml_reference_provenance(
        &source.bytes,
        &source.uri,
        budget,
    )?;
    crate::document_reference_provenance::complete_xml_base_inventory(&inventory.document)?;
    let mut remaining = budget;
    let source_attrs = attributes(&inventory.document.tree, &mut remaining)?;
    let mut originals = BTreeMap::new();
    for id in source_attrs {
        let key = inventory
            .document
            .tree
            .source_key(id)
            .ok_or("source attribute provenance required")?;
        if originals.insert(key, id).is_some() {
            return Err("ambiguous original source key".into());
        }
    }
    let mut affected = Vec::new();
    for output in outputs {
        if output
            .tree
            .ast_owner()
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err("output import diagnostics".into());
        }
        for id in attributes(&output.tree, &mut remaining)? {
            let key = output
                .tree
                .source_key(id)
                .ok_or("complete output attribute provenance required")?;
            let uri = output
                .tree
                .node_source_uri(id)
                .ok_or("output source URI required")?;
            if uri != source.uri {
                continue;
            }
            let original = originals
                .get(&key)
                .ok_or("output refers to a different source snapshot")?;
            let a = inventory
                .document
                .tree
                .node(*original)
                .ok_or("source attribute missing")?;
            let b = output.tree.node(id).ok_or("output attribute missing")?;
            let name = |n: &crate::parser::tree::CemTreeNode| {
                n.name
                    .as_ref()
                    .map(|n| (n.namespace_uri.clone(), n.local_name.clone(), n.schema_id))
            };
            if name(a) != name(b) || a.value != b.value {
                return Err("output attribute changed from source".into());
            }
            if key == source_key {
                crate::document_reference_provenance::bind_reference_source(
                    &output.tree,
                    id,
                    &inventory,
                )?;
                affected.push(XmlAffectedOccurrence {
                    output_id: output.id.clone(),
                    attribute: id,
                });
            }
        }
    }
    if affected.is_empty() {
        return Err("no supplied occurrence matches the source reference".into());
    }
    if let XmlEditScope::Occurrence {
        output_id,
        attribute,
    } = scope
    {
        if affected.len() != 1
            || affected[0].output_id != output_id
            || affected[0].attribute != attribute
        {
            return Err("occurrence-only edit would affect another source occurrence".into());
        }
    }
    let candidate = crate::import::xml_reference_edit::plan_xml_reference_edit(
        &source.bytes,
        &source.uri,
        source_key,
        to_uri,
        budget,
    )?;
    Ok(PreparedXmlEditTransaction {
        source: source.clone(),
        outputs: outputs.to_vec(),
        policy_stamp: policy_stamp.into(),
        affected,
        candidate,
    })
}
fn attributes(tree: &RetainedCemTree, remaining: &mut usize) -> Result<Vec<AstNodeId>, String> {
    let mut pending = vec![0];
    let mut seen = BTreeSet::new();
    let mut attributes = Vec::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            return Err("ambiguous retained node occurrence".into());
        }
        *remaining = remaining
            .checked_sub(1)
            .ok_or("occurrence inspection budget exceeded")?;
        let node = tree.node(id).ok_or("invalid retained output node")?;
        if node.kind == CemTreeNodeKind::Reference {
            return Err("opaque reference occurrence analysis required".into());
        }
        *remaining = remaining
            .checked_sub(node.attributes.len())
            .ok_or("occurrence inspection budget exceeded")?;
        if node.children.len() > *remaining {
            return Err("occurrence inspection budget exceeded".into());
        }
        for attribute in &node.attributes {
            if !seen.insert(*attribute)
                || !tree
                    .node(*attribute)
                    .is_some_and(|n| n.kind == CemTreeNodeKind::Attribute)
            {
                return Err("ambiguous or invalid attribute occurrence".into());
            }
            attributes.push(*attribute);
        }
        pending.extend(node.children.iter().rev().copied());
    }
    Ok(attributes)
}
