use cem_ml::{
    import::import_bytes_with_lexical_scopes,
    parser::{
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode,
    },
    schema::vocab::CompiledSchema,
    value::artifact::CemValueArtifactLimits,
};
use cem_ql::eval::{
    imported_cem_tree,
    portable::{export_values, NativeExportFailureKind},
    values::reference,
    ItemStream,
};

#[test]
fn cemv_rejects_source_references_in_every_target_state_with_provenance() {
    for targets in [None, Some(vec![]), Some(vec![1])] {
        let source = import_bytes_with_lexical_scopes(
            b"{#input}",
            "text/cem-ml",
            "source.cem",
            CompiledSchema::cem_core(),
        )
        .unwrap();
        let mut ast = cem_ml::ast::DebugBinaryDecoder::new()
            .decode(
                &cem_ml::ast::DebugBinaryEncoder::new()
                    .encode(source.tree.ast())
                    .bytes,
            )
            .unwrap();
        if let CemAstNode::Reference { targets: value, .. } = &mut ast.nodes[1] {
            *value = targets;
        }
        let tree = RetainedCemTree::new(
            ast,
            "source.cem",
            "{#input}",
            CemTreeSemantics::default(),
            None,
        )
        .unwrap();
        let item = imported_cem_tree(tree)
            .view()
            .unwrap()
            .field("children")
            .unwrap()
            .remove(0);
        let original_map = item.source_map();
        let error =
            export_values(&ItemStream::once(item), &CemValueArtifactLimits::default()).unwrap_err();
        assert_eq!(
            error.kind,
            NativeExportFailureKind::UnsupportedSourceReference
        );
        assert_eq!(error.source, original_map);
    }
}

#[test]
fn materialized_reference_export_preserves_empty_nested_and_repeated_edges() {
    let leaf = reference(vec![]);
    let outer = reference(vec![leaf.clone(), leaf]);
    let limits = CemValueArtifactLimits::default();
    let bytes = export_values(&ItemStream::once(outer), &limits).unwrap();
    let restored = cem_ql::eval::portable::decode_values(&bytes, &limits).unwrap();
    let targets = restored.items[0].view().unwrap().field("targets").unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].identity(), targets[1].identity());
    assert!(targets[0]
        .view()
        .unwrap()
        .field("targets")
        .unwrap()
        .is_empty());
}

#[derive(Debug, Clone)]
struct CyclicView {
    cross_edge: bool,
    id: u32,
}
impl cem_ql::eval::QueryItemView for CyclicView {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "fixture.cycle"
    }
    fn identity(&self) -> String {
        format!("{}:{}", self.cross_edge, self.id)
    }
    fn kind(&self) -> cem_ql::eval::QueryItemViewKind {
        cem_ql::eval::QueryItemViewKind::Node
    }
    fn source_map(&self) -> Option<cem_ml::source_map::SourceMapStack> {
        Some(Default::default())
    }
    fn field(&self, name: &str) -> Option<Vec<cem_ql::eval::Item>> {
        use cem_ql::eval::{AtomValue, Item};
        match name {
            "kind" => Some(vec![Item::Atomic(AtomValue::String(
                if self.cross_edge && self.id == 0 {
                    "element"
                } else {
                    "reference"
                }
                .into(),
            ))]),
            "targets" if !self.cross_edge || self.id == 1 => Some(vec![Item::native(Self {
                cross_edge: self.cross_edge,
                id: 0,
            })]),
            _ => None,
        }
    }
    fn parent(
        &self,
        _: cem_ql::eval::QueryContextScope,
    ) -> Result<Option<cem_ql::eval::Item>, cem_ql::eval::QueryNodeAccessError> {
        Ok((self.id == 1).then(|| {
            cem_ql::eval::Item::native(Self {
                cross_edge: self.cross_edge,
                id: 0,
            })
        }))
    }
    fn children(
        &self,
        _: cem_ql::eval::QueryContextScope,
    ) -> Result<cem_ql::eval::QueryNodeIterator<'_>, cem_ql::eval::QueryNodeAccessError> {
        Ok(Box::new(
            (self.cross_edge && self.id == 0)
                .then(|| {
                    Ok(cem_ql::eval::Item::native(Self {
                        cross_edge: true,
                        id: 1,
                    }))
                })
                .into_iter(),
        ))
    }
}

#[test]
fn cycle_export_is_attributed_and_distinct_from_limits_and_cancellation() {
    use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    use cem_ql::eval::{portable::export_values_with_control, Item, QueryContextScope};
    let limits = CemValueArtifactLimits::default();
    for cross_edge in [false, true] {
        let error = export_values(
            &ItemStream::once(Item::native(CyclicView { cross_edge, id: 0 })),
            &limits,
        )
        .unwrap_err();
        assert_eq!(error.kind, NativeExportFailureKind::UnsupportedGraph);
        assert_eq!(error.message, "Cyclic native CEM value graph");
        assert!(error.source.is_some());
    }
    let values = ItemStream::once(reference(vec![]));
    let cancelled = OperationControl::default();
    cancelled.abort_signal().abort();
    assert_eq!(
        export_values_with_control(
            &values,
            &limits,
            QueryContextScope(0),
            &cancelled,
            ROOT_EXECUTION_SCOPE_ID
        )
        .unwrap_err()
        .kind,
        NativeExportFailureKind::Cancelled
    );
    assert_eq!(
        export_values(
            &values,
            &CemValueArtifactLimits {
                max_values: 0,
                ..limits
            }
        )
        .unwrap_err()
        .kind,
        NativeExportFailureKind::Limit
    );
    assert_eq!(
        export_values(&ItemStream::once(Item::Record(Default::default())), &limits)
            .unwrap_err()
            .kind,
        NativeExportFailureKind::InvalidValue
    );
    let mut partial = values;
    partial.error = Some(cem_ql::eval::EvalError::TypeError(
        "later stage failed".into(),
    ));
    assert_eq!(
        export_values(&partial, &limits).unwrap_err().kind,
        NativeExportFailureKind::InvalidValue
    );
}
