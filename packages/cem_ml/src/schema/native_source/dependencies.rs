//! Separate passive source profile for unevaluated native dependency references.
//! No targets, executable callbacks, runtime scopes or crossing permissions.
use super::*;
use std::collections::BTreeMap;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeDependencySourceSnapshot {
    pub(super) version: u32,
    pub(super) nodes: Vec<SourceNode>,
    pub(super) contexts: BTreeMap<AstNodeId, AstNodeId>,
    pub(super) lexical: super::super::machine::LexicalReloadMetadata,
}
pub(super) fn capture_nodes(document: &CemDocument) -> Result<Vec<SourceNode>, String> {
    nodes_profile(document, true, true)
}
impl NativeDependencySourceSnapshot {
    pub fn capture(
        capture: &super::super::machine::LexicallyScopedDocument,
        identity: &SourceIdentity,
        poll: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<Self, String> {
        poll()?;
        let nodes = capture_nodes(capture.document())?;
        let contexts = capture
            .document()
            .nodes
            .iter()
            .filter_map(|n| match n {
                CemAstNode::Reference {
                    node_id, context, ..
                } => Some((*node_id, *context)),
                _ => None,
            })
            .collect::<BTreeMap<_, _>>();
        restore_profile(&nodes, identity, true, Some(&contexts), poll)?;
        let fingerprint = fingerprint(&nodes, &contexts)?;
        let result = Self {
            version: 1,
            nodes,
            contexts,
            lexical: super::super::machine::LexicalReloadMetadata::export(capture, fingerprint),
        };
        result.restore(identity, poll)?;
        Ok(result)
    }
    pub fn restore(
        &self,
        identity: &SourceIdentity,
        poll: &mut dyn FnMut() -> Result<(), String>,
    ) -> Result<std::sync::Arc<super::super::machine::LexicallyScopedDocument>, String> {
        poll()?;
        if self.version != 1 {
            return Err(INVALID.into());
        }
        let document = restore_profile(&self.nodes, identity, true, Some(&self.contexts), poll)?;
        restore_lexical(
            document,
            &self.lexical,
            identity,
            fingerprint(&self.nodes, &self.contexts)?,
            poll,
        )
    }
}
fn fingerprint(
    nodes: &[SourceNode],
    contexts: &BTreeMap<AstNodeId, AstNodeId>,
) -> Result<[u8; 32], String> {
    Ok(
        *blake3::hash(&rmp_serde::to_vec_named(&(1u32, nodes, contexts)).map_err(|_| INVALID)?)
            .as_bytes(),
    )
}
