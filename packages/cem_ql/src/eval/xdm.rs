//! Owned document contexts for constructed native stage output. No writer,
//! external parser or serialized value artifact participates in this boundary.
use super::{
    output::shared_output_nodes, portable, value_control::ValueControl, xpath_values, Item,
    QueryContextScope,
};
use crate::{render::RenderPlanNode, xpath::functions::XPathQueryItem};
use cem_ml::{
    operation_control::{ExecutionScopeId, MemoryPermit, OperationControl},
    validation::xpath::XPathResultItem,
    value::{
        artifact::{CemValueArtifactLimits, CemValueRecord},
        xpath::CemValueXPathProjection,
    },
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct OriginalValues {
    // Keep referenced source allocations alive after the stage artifact drops.
    // Retain raw values, never the cache-bearing document, to avoid a cycle.
    _values: Arc<Vec<RenderPlanNode>>,
    _memory: MemoryPermit,
}
#[derive(Debug, Clone)]
struct Projection {
    scope: QueryContextScope,
    root: u32,
    native: Arc<CemValueXPathProjection>,
}

/// One constructed XDM document per stage owner and node capability scope.
/// Ordinary CEM consumers can still use the unchanged flat native value sequence.
#[derive(Debug)]
pub struct OwnedXdmDocument {
    values: Arc<Vec<RenderPlanNode>>,
    projection: Mutex<Option<Projection>>,
}
impl OwnedXdmDocument {
    pub fn new(values: Arc<Vec<RenderPlanNode>>) -> Self {
        Self {
            values,
            projection: Mutex::new(None),
        }
    }
    pub fn values(&self) -> Arc<Vec<RenderPlanNode>> {
        self.values.clone()
    }

    /// Recheck the consuming scope even when a previous stage built the index.
    /// The document has no invented document URI or original source coordinates.
    pub fn document_with_control(
        &self,
        query_scope: QueryContextScope,
        limits: &CemValueArtifactLimits,
        control: &OperationControl,
        scope: ExecutionScopeId,
    ) -> Result<Item, String> {
        let result = (|| {
            let budget = ValueControl::new(control, scope, query_scope, *limits)
                .map_err(|e| e.to_string())?;
            let mut check = || control.check_scope(scope).map_err(|e| e.to_string());
            check()?;
            let cached = self
                .projection
                .lock()
                .map_err(|_| "Native XDM cache lock failed")?
                .as_ref()
                .filter(|p| p.scope == query_scope)
                .cloned();
            let projection = if let Some(projection) = cached {
                projection
            } else {
                // Bound the cache to the most recently used capability scope.
                *self
                    .projection
                    .lock()
                    .map_err(|_| "Native XDM cache lock failed")? = None;
                let (mut graph, _) = portable::encode_graph(
                    &shared_output_nodes(self.values.clone()),
                    &budget.limits,
                    true,
                    query_scope,
                    &mut check,
                )?;
                // A literal's original source URI is in its native source frames;
                // it is provenance, never the constructed document's URI.
                for record in &mut graph.records {
                    check()?;
                    if record.provenance.is_none() {
                        if let Some(uri) = record.source.frames.iter().find_map(|frame| {
                            if let cem_ml::source_map::TransformKind::SourceBoundary { uri } =
                                &frame.transform
                            {
                                Some(uri.clone())
                            } else {
                                None
                            }
                        }) {
                            record.provenance = Some(cem_ml::value::artifact::CemValueProvenance {
                                source_uri: Some(uri),
                                ..Default::default()
                            });
                        }
                    }
                }
                let root = u32::try_from(graph.records.len())
                    .map_err(|_| "Native XDM identity overflow")?;
                let children = std::mem::take(&mut graph.roots);
                for &child in &children {
                    check()?;
                    if graph.records[child as usize].parent.is_some() {
                        return Err("Stage output roots must be detached native occurrences".into());
                    }
                    graph.records[child as usize].parent = Some(root);
                }
                graph.records.push(CemValueRecord {
                    kind: "document".into(),
                    children,
                    ..Default::default()
                });
                graph.roots.push(root);
                let values = self.values.clone();
                let native = CemValueXPathProjection::build_with_owner(
                    Arc::new(graph),
                    &budget.limits,
                    &mut check,
                    |bytes, _| {
                        let memory = control
                            .charge_memory(scope, bytes as u64, None)
                            .map_err(|e| e.to_string())?;
                        Ok(Some(Arc::new(OriginalValues {
                            _values: values,
                            _memory: memory,
                        })
                            as Arc<dyn std::any::Any + Send + Sync>))
                    },
                )?;
                let projection = Projection {
                    scope: query_scope,
                    root,
                    native: Arc::new(native),
                };
                let mut cached = self
                    .projection
                    .lock()
                    .map_err(|_| "Native XDM cache lock failed")?;
                if let Some(existing) = cached.as_ref().filter(|p| p.scope == query_scope) {
                    existing.clone()
                } else {
                    *cached = Some(projection.clone());
                    projection
                }
            };
            let _admission = control
                .charge_memory(scope, projection.native.accounted_bytes as u64, None)
                .map_err(|e| e.to_string())?;
            projection
                .native
                .check_limits_with_check(&budget.limits, &mut check)?;
            let mut items = projection
                .native
                .items_with_check(projection.root, &mut check)?;
            if items
                .first()
                .and_then(XPathResultItem::native_node)
                .is_none()
            {
                return Err("Native stage projection lost its document owner".into());
            }
            if items.len() != 1 {
                return Err("Native stage projection must contain one document".into());
            }
            check()?;
            Ok(XPathQueryItem::wrap(items.remove(0)))
        })();
        xpath_values::checked_projection(result, control, scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::{
        operation_control::ROOT_EXECUTION_SCOPE_ID,
        scheduler::{AbortSignal, ScopePolicy},
        source::{ByteRange, SourceId},
        source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
        validation::xpath::{XPathNativeNode, XPathResultNodeKind},
    };
    fn values() -> Arc<Vec<RenderPlanNode>> {
        let source = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange::new(9, 13)),
                transform: TransformKind::CemTokenizer,
            }],
        }
        .with_source_uri("studio://xdm/templates/make.cemt");
        Arc::new(vec![RenderPlanNode::Element {
            tag: "item".into(),
            namespace: Some("urn:orders".into()),
            qualified_name: Some("o:item".into()),
            attributes: vec![],
            children: vec![RenderPlanNode::Text {
                text: "Café 🍒".into(),
                source_map: source.clone(),
            }],
            source_map: source,
        }])
    }
    fn node(item: &Item) -> XPathNativeNode {
        item.view()
            .unwrap()
            .downcast_ref::<XPathQueryItem>()
            .unwrap()
            .xpath_item()
            .native_node()
            .unwrap()
            .clone()
    }
    fn document(owner: &OwnedXdmDocument) -> Item {
        owner
            .document_with_control(
                QueryContextScope(0),
                &Default::default(),
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
            )
            .unwrap()
    }
    #[test]
    fn stage_document_has_stable_identity_original_provenance_and_no_document_uri() {
        let values = values();
        let weak = Arc::downgrade(&values);
        let owner = OwnedXdmDocument::new(values);
        let first = document(&owner);
        let second = document(&owner);
        let root = node(&first);
        assert_eq!(first.identity(), second.identity());
        assert_eq!(root.result_node_kind(), XPathResultNodeKind::Document);
        assert!(root.parent_node().is_none());
        assert_eq!(root.document_uri(), None);
        let child = root.child_nodes().remove(0);
        assert_eq!(child.parent_node().unwrap().identity(), root.identity());
        assert_eq!(child.source_uri(), Some("studio://xdm/templates/make.cemt"));
        assert_eq!(
            child.source_line_number(),
            None,
            "source text was not supplied; do not invent lines"
        );
        assert_eq!(child.string_value(), "Café 🍒");
        assert!(child.source_map().frames.iter().any(|frame| matches!(&frame.transform, TransformKind::SourceBoundary { uri } if uri == "studio://xdm/templates/make.cemt")));
        assert_eq!(
            child.source_map().origin().unwrap().span,
            FrameSpan::Single(ByteRange::new(9, 13))
        );
        let other = OwnedXdmDocument::new(owner.values());
        assert_ne!(first.identity(), document(&other).identity());
        drop(other);
        drop(owner);
        drop(second);
        assert!(
            weak.upgrade().is_some(),
            "selected XPath nodes retain raw stage values"
        );
        drop(child);
        drop(root);
        drop(first);
        assert!(
            weak.upgrade().is_none(),
            "no document/cache/native-owner cycle"
        );
    }
    #[test]
    fn memory_permits_follow_the_last_node_owner_and_cache_hits_do_not_accumulate() {
        let control = OperationControl::default();
        let owner = OwnedXdmDocument::new(values());
        let first = owner
            .document_with_control(
                QueryContextScope(0),
                &Default::default(),
                &control,
                ROOT_EXECUTION_SCOPE_ID,
            )
            .unwrap();
        let charged = control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap();
        assert!(charged > 0);
        for _ in 0..4 {
            assert_eq!(
                first.identity(),
                owner
                    .document_with_control(
                        QueryContextScope(0),
                        &Default::default(),
                        &control,
                        ROOT_EXECUTION_SCOPE_ID
                    )
                    .unwrap()
                    .identity()
            );
            assert_eq!(
                control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(),
                charged
            );
        }
        drop(owner);
        assert_eq!(
            control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(),
            charged
        );
        drop(first);
        assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }

    #[test]
    fn copied_references_retain_original_source_owners_keys_and_ranges() {
        let source = cem_ml::import::import_data(
            "<r xmlns='urn:source'>Café &amp; 🍒</r>",
            "xml",
            "cem",
            "studio://xdm/data/source.xml",
        )
        .unwrap();
        let weak = Arc::downgrade(&source);
        let child_id = source.node(0).unwrap().children[0];
        let key = source.source_key(child_id);
        let original_map = source.node_source_map(child_id).unwrap();
        let input = super::super::imported_cem_tree(source.clone());
        let owner = OwnedXdmDocument::new(Arc::new(vec![RenderPlanNode::Reference {
            reference: cem_ml::value::CemReference::new(vec![input]),
            source_map: Default::default(),
        }]));
        drop(source);
        let item = document(&owner);
        let root = node(&item);
        let child = root.child_nodes().remove(0);
        assert_eq!(child.source_uri(), Some("studio://xdm/data/source.xml"));
        assert_eq!(child.source_key(), key);
        assert_eq!(child.string_value(), "Café & 🍒");
        assert_eq!(child.source_map().frames, original_map.frames);
        assert_eq!(child.parent_node().unwrap().identity(), root.identity());
        drop(owner);
        drop(root);
        drop(item);
        assert!(
            weak.upgrade().is_some(),
            "selected native nodes retain original reference targets"
        );
        drop(child);
        assert!(
            weak.upgrade().is_none(),
            "source allocations release with the last actual node owner"
        );
    }

    #[test]
    fn cached_stage_document_rechecks_node_depth_bytes_memory_and_cancellation() {
        let owner = OwnedXdmDocument::new(values());
        let original = document(&owner);
        for limits in [
            CemValueArtifactLimits {
                max_values: 1,
                ..Default::default()
            },
            CemValueArtifactLimits {
                max_depth: 1,
                ..Default::default()
            },
            CemValueArtifactLimits {
                max_bytes: 1,
                ..Default::default()
            },
        ] {
            assert!(owner
                .document_with_control(
                    QueryContextScope(0),
                    &limits,
                    &OperationControl::default(),
                    ROOT_EXECUTION_SCOPE_ID
                )
                .is_err());
        }
        let small = OperationControl::with_root_policy(
            AbortSignal::new(),
            ScopePolicy::host_root().with_memory_bytes(1),
        )
        .unwrap();
        assert!(owner
            .document_with_control(
                QueryContextScope(0),
                &Default::default(),
                &small,
                ROOT_EXECUTION_SCOPE_ID
            )
            .is_err());
        let cancelled = OperationControl::default();
        cancelled.cancel_root(None, None).unwrap();
        assert!(owner
            .document_with_control(
                QueryContextScope(0),
                &Default::default(),
                &cancelled,
                ROOT_EXECUTION_SCOPE_ID
            )
            .is_err());
        assert_eq!(
            original.identity(),
            document(&owner).identity(),
            "a failed foreign consumer cannot poison the original document"
        );
    }
    #[test]
    fn empty_and_multi_root_outputs_are_documents_and_capability_cache_is_bounded() {
        let empty = OwnedXdmDocument::new(Arc::new(vec![]));
        assert!(node(&document(&empty)).child_nodes().is_empty());
        let mut roots = values().as_ref().clone();
        roots.extend(values().as_ref().clone());
        let owner = OwnedXdmDocument::new(Arc::new(roots));
        let first = document(&owner);
        let first_node = node(&first);
        assert_eq!(first_node.child_nodes().len(), 2);
        let scoped = owner
            .document_with_control(
                QueryContextScope(1),
                &Default::default(),
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
            )
            .unwrap();
        assert_ne!(first.identity(), scoped.identity());
        assert_eq!(
            first_node.child_nodes().len(),
            2,
            "evicted indexes live only through their actual node owners"
        );
    }
}
