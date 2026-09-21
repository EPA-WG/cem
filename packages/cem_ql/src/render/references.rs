//! References are expanded one level only, at output projection boundaries.
//! Children keep native owners; expansion never changes the source tree.
use super::*;
use crate::eval::values::reference_values;
use cem_ml::validation::xpath::{XPathNativeNode, XPathResultNodeKind};
use cem_ml::value::CemReference;

fn native(item: &Item) -> Option<XPathNativeNode> {
    if let Some(view) = item
        .view()
        .and_then(|v| v.downcast_ref::<crate::xpath::functions::XPathQueryItem>())
    {
        return view.xpath_item().native_node().cloned();
    }
    crate::eval::result_native_node(item)?.ok()
}

fn occurrence(item: Item, source_map: SourceMapStack) -> RenderPlanNode {
    RenderPlanNode::Reference {
        reference: CemReference::new(vec![item]),
        source_map,
    }
}

impl PlanRenderer<'_> {
    pub(super) fn insert_values(
        &mut self,
        stream: ItemStream,
        source: &SourceMapStack,
        out: &mut ResultBuffer,
    ) {
        let mut pending = vec![stream.items.as_slice().iter()];
        while let Some(items) = pending.last_mut() {
            let Some(item) = items.next() else {
                pending.pop();
                continue;
            };
            if !self.charge_result(0, pending.len(), source) {
                return;
            }
            if let Some(targets) = reference_values(item) {
                pending.push(targets.iter());
                continue;
            }
            if item
                .view()
                .and_then(|v| v.field("kind"))
                .is_some_and(|kind| {
                    kind.first().and_then(Item::atom) == Some(AtomValue::String("attribute".into()))
                })
            {
                self.fail_insertion("An attribute cannot be inserted as body content", source);
                return;
            }
            if item.view().is_some_and(|v| {
                v.downcast_ref::<crate::eval::output::OutputView>()
                    .is_some()
                    || v.downcast_ref::<crate::eval::portable::GraphView>()
                        .is_some()
            }) {
                out.push(occurrence(item.clone(), source.clone()));
            } else if let Some(node) = native(item) {
                if node.result_node_kind() == XPathResultNodeKind::Attribute {
                    self.fail_insertion("An attribute cannot be inserted as body content", source);
                    return;
                }
                out.push(occurrence(item.clone(), source.clone()));
            } else if item.atom().is_some() {
                let text = self.render_stream_text(&ItemStream::once(item.clone()), source);
                out.push(RenderPlanNode::Text {
                    text,
                    source_map: source.clone(),
                });
            } else {
                self.fail_insertion(
                    "Expression content requires native nodes or atomic values",
                    source,
                );
                return;
            }
        }
    }

    fn fail_insertion(&mut self, message: &str, source: &SourceMapStack) {
        self.recovery_depth += 1;
        self.template_failure(render_diagnostic(
            "cem.ql.render.expression_type",
            message.into(),
            source_map_start(source),
            source.clone(),
        ));
        self.recovery_depth -= 1;
    }
}

/// Project one level of a retained reference for a DOM/string export. This is
/// not an intermediate CEM handoff: callers keep the reference until exporting.
pub fn expand_reference(reference: &CemReference<Item>) -> Vec<RenderPlanNode> {
    let mut result = Vec::new();
    for item in reference.values() {
        if let Some(graph) = item
            .view()
            .and_then(|v| v.downcast_ref::<crate::eval::portable::GraphView>())
        {
            result.extend(expand_graph(graph));
            continue;
        }
        if let Some(values) = reference_values(item) {
            result.push(RenderPlanNode::Reference {
                reference: CemReference::new(values.to_vec()),
                source_map: item.source_map().unwrap_or_default(),
            });
            continue;
        }
        if let Some((tree, id)) = crate::eval::imported_source_node(item) {
            use cem_ml::parser::CemAstNode;
            let source = tree.ast().get(id).expect("retained source node");
            let value = || {
                tree.source_text_fragments(id)
                    .into_iter()
                    .flatten()
                    .collect::<String>()
            };
            let projected = match source {
                CemAstNode::Text { source, .. }
                | CemAstNode::Whitespace { source, .. }
                | CemAstNode::RawText { source, .. } => Some(RenderPlanNode::Text {
                    text: value(),
                    source_map: source.clone(),
                }),
                CemAstNode::Cdata { source, .. } => Some(RenderPlanNode::Cdata {
                    text: value(),
                    source_map: source.clone(),
                }),
                _ => None,
            };
            if let Some(projected) = projected {
                result.push(projected);
                continue;
            }
        }
        if let Some(view) = item
            .view()
            .and_then(|v| v.downcast_ref::<crate::eval::output::OutputView>())
        {
            result.push(view.node().clone());
            continue;
        }
        let Some(node) = native(item) else {
            result.push(RenderPlanNode::Text {
                text: item_to_string(item),
                source_map: SourceMapStack::default(),
            });
            continue;
        };
        let source_map = node.source_map();
        let children = || {
            if crate::eval::imported_source_node(item).is_some() {
                return item
                    .view()
                    .and_then(|v| v.field("children"))
                    .unwrap_or_default()
                    .into_iter()
                    .map(|child| {
                        let source = child.source_map().unwrap_or_default();
                        occurrence(child, source)
                    })
                    .collect::<Vec<_>>();
            }
            node.child_nodes()
                .into_iter()
                .map(|child| {
                    let source_map = child.source_map();
                    occurrence(
                        crate::xpath::functions::XPathQueryItem::from_node(child),
                        source_map,
                    )
                })
                .collect::<Vec<_>>()
        };
        match node.result_node_kind() {
            XPathResultNodeKind::Document => result.extend(children()),
            XPathResultNodeKind::Element => {
                let namespace =
                    (!node.namespace_uri().is_empty()).then(|| node.namespace_uri().to_owned());
                let mut attributes = Vec::new();
                if let Some(namespace) = &namespace {
                    attributes.push(RenderPlanAttribute {
                        contract: None,
                        name: "xmlns".into(),
                        namespace: Some("http://www.w3.org/2000/xmlns/".into()),
                        qualified_name: Some("xmlns".into()),
                        value: namespace.clone(),
                        value_stream: string_stream(namespace.clone()),
                        source_map: source_map.clone(),
                    });
                }
                for (index, attribute) in node.attribute_nodes().into_iter().enumerate() {
                    let namespace = (!attribute.namespace_uri().is_empty())
                        .then(|| attribute.namespace_uri().to_owned());
                    let name = attribute.local_name().to_owned();
                    let qualified_name = match namespace.as_deref() {
                        None => name.clone(),
                        Some("http://www.w3.org/XML/1998/namespace") => format!("xml:{name}"),
                        Some(uri) => {
                            let prefix = format!("ns{}", index + 1);
                            attributes.push(RenderPlanAttribute {
                                contract: None,
                                name: prefix.clone(),
                                namespace: Some("http://www.w3.org/2000/xmlns/".into()),
                                qualified_name: Some(format!("xmlns:{prefix}")),
                                value: uri.into(),
                                value_stream: string_stream(uri.into()),
                                source_map: attribute.source_map(),
                            });
                            format!("{prefix}:{name}")
                        }
                    };
                    attributes.push(RenderPlanAttribute {
                        contract: None,
                        name,
                        qualified_name: Some(qualified_name),
                        namespace,
                        value: attribute.semantic_value().into(),
                        value_stream: string_stream(attribute.semantic_value().into()),
                        source_map: attribute.source_map(),
                    });
                }
                result.push(RenderPlanNode::Element {
                    tag: node.local_name().into(),
                    qualified_name: Some(node.local_name().into()),
                    namespace,
                    attributes,
                    children: children(),
                    source_map,
                });
            }
            XPathResultNodeKind::Text => result.push(RenderPlanNode::Text {
                text: node.semantic_value().into(),
                source_map,
            }),
            XPathResultNodeKind::Comment => result.push(RenderPlanNode::Comment {
                text: node.semantic_value().into(),
                source_map,
            }),
            XPathResultNodeKind::ProcessingInstruction => {
                result.push(RenderPlanNode::ProcessingInstruction {
                    target: node.local_name().into(),
                    data: node.semantic_value().into(),
                    source_map,
                })
            }
            _ => {}
        }
    }
    result
}

pub(super) fn expand_reference_scoped(
    reference: &CemReference<Item>, budget: &mut crate::eval::value_control::ValueControl<'_>,
) -> Result<Vec<RenderPlanNode>, cem_ml::operation_control::ControlError> {
    let mut result = Vec::new();
    for item in reference.values() {
        budget.charge(0, 0)?;
        if let Some(view) = item.view().filter(|v| v.kind() == crate::eval::QueryItemViewKind::Node) {
            if reference_values(item).is_none() {
                view.parent(budget.query_scope).map_err(|e| budget.access_error(e))?;
                let kind = view.field("kind").and_then(|v| v.first().and_then(Item::atom));
                if matches!(kind, Some(AtomValue::String(ref k)) if k == "element" || k == "document") {
                    for child in view.children(budget.query_scope).map_err(|e| budget.access_error(e))? {
                        budget.charge(0, 0)?;
                        child.map_err(|e| budget.access_error(e))?;
                    }
                }
                if native(item).is_none()
                    && view.downcast_ref::<crate::eval::output::OutputView>().is_none()
                    && view.downcast_ref::<crate::eval::portable::GraphView>().is_none()
                {
                    return Err(budget.failure("cem.value.projection_unsupported"));
                }
            }
        } else if item.atom().is_none() {
            return Err(budget.failure("cem.value.projection_unsupported"));
        }
        result.extend(expand_reference(&CemReference::new(vec![item.clone()])));
    }
    Ok(result)
}

fn expand_graph(view: &crate::eval::portable::GraphView) -> Vec<RenderPlanNode> {
    let record = view.record();
    let source_map = record.source.clone();
    let values = |ids: &[u32]| {
        ids.iter()
            .map(|&id| occurrence(view.item(id), source_map.clone()))
            .collect::<Vec<_>>()
    };
    let node = match record.kind.as_str() {
        "reference" => return values(&record.targets),
        "document" => return values(&record.children),
        "element" => {
            let mut attributes = Vec::new();
            if !record.namespace.is_empty() {
                attributes.push(RenderPlanAttribute {
                    name: "xmlns".into(),
                    qualified_name: Some("xmlns".into()),
                    namespace: Some("http://www.w3.org/2000/xmlns/".into()),
                    value: record.namespace.clone(),
                    value_stream: string_stream(record.namespace.clone()),
                    contract: None,
                    source_map: source_map.clone(),
                });
            }
            for (index, &id) in record.attributes.iter().enumerate() {
                let attribute = &view.owner.records[id as usize];
                if attribute.namespace == "http://www.w3.org/2000/xmlns/" {
                    continue;
                }
                let qualified_name = if attribute.namespace.is_empty() {
                    attribute.name.clone()
                } else if attribute.namespace == "http://www.w3.org/XML/1998/namespace" {
                    format!("xml:{}", attribute.name)
                } else {
                    let prefix = format!("ns{}", index + 1);
                    attributes.push(RenderPlanAttribute {
                        name: prefix.clone(),
                        qualified_name: Some(format!("xmlns:{prefix}")),
                        namespace: Some("http://www.w3.org/2000/xmlns/".into()),
                        value: attribute.namespace.clone(),
                        value_stream: string_stream(attribute.namespace.clone()),
                        contract: None,
                        source_map: attribute.source.clone(),
                    });
                    format!("{prefix}:{}", attribute.name)
                };
                attributes.push(RenderPlanAttribute {
                    name: attribute.name.clone(),
                    namespace: (!attribute.namespace.is_empty())
                        .then(|| attribute.namespace.clone()),
                    qualified_name: Some(qualified_name),
                    value: attribute.lexical.clone(),
                    value_stream: if !attribute.has_native_content() {
                        string_stream(attribute.lexical.clone())
                    } else {
                        ItemStream::from_items(
                            attribute.values.iter().map(|&id| view.item(id)).collect(),
                        )
                    },
                    contract: attribute.contract.clone().map(std::sync::Arc::new),
                    source_map: attribute.source.clone(),
                });
            }
            RenderPlanNode::Element {
                tag: record.name.clone(),
                qualified_name: Some(record.name.clone()),
                namespace: (!record.namespace.is_empty()).then(|| record.namespace.clone()),
                attributes,
                children: values(&record.children),
                source_map,
            }
        }
        "comment" => RenderPlanNode::Comment {
            text: record.lexical.clone(),
            source_map,
        },
        "processing-instruction" => RenderPlanNode::ProcessingInstruction {
            target: record.name.clone(),
            data: record.lexical.clone(),
            source_map,
        },
        "cdata" => RenderPlanNode::Cdata {
            text: record.lexical.clone(),
            source_map,
        },
        _ => RenderPlanNode::Text {
            text: record.lexical.clone(),
            source_map,
        },
    };
    vec![node]
}
