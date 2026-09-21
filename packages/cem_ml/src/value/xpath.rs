//! Lazy semantic XPath projection of native values. No external syntax or parser
//! participates. Reference targets keep source identity; inserted occurrences
//! acquire their output location in this cached semantic index.
use super::artifact::{CemValueArtifactLimits, CemValueGraph};
use crate::{
    parser::{
        document::CemDocument,
        tree::{CemTreeSemantics, RetainedCemTree},
        CemAstNode, ExpandedName,
    },
    validation::xpath::{XPathAtomicValue, XPathNativeNode, XPathResultItem},
};
use std::sync::Arc;

#[derive(Debug)]
pub struct CemValueXPathProjection {
    graph: Arc<CemValueGraph>,
    tree: Arc<RetainedCemTree>,
    selections: Vec<Vec<u32>>,
    pub accounted_bytes: usize,
}

impl CemValueXPathProjection {
    pub fn build(
        graph: Arc<CemValueGraph>,
        limits: &CemValueArtifactLimits,
        check: &mut impl FnMut() -> Result<(), String>,
    ) -> Result<Self, String> {
        Self::build_with_owner(graph, limits, check, |_, _| Ok(None))
    }

    pub fn build_with_owner(
        graph: Arc<CemValueGraph>, limits: &CemValueArtifactLimits,
        check: &mut impl FnMut() -> Result<(), String>,
        retain: impl FnOnce(usize, usize) -> Result<Option<Arc<dyn std::any::Any + Send + Sync>>, String>,
    ) -> Result<Self, String> {
        check()?;
        graph.validate_with_check(limits, check)?;
        let mut builder = Builder {
            graph: &graph,
            limits,
            check,
            ast: CemDocument::default(),
            semantics: CemTreeSemantics {
                document_metadata: Some(Default::default()),
                ..Default::default()
            },
            selections: vec![Vec::new(); graph.records.len()],
            bytes: 0,
        };
        // Arena zero is an inaccessible storage sentinel, never a fabricated
        // parent for a detached value.
        builder.ast.nodes.push(CemAstNode::Document {
            node_id: 0,
            root_children: vec![],
            source: Default::default(),
        });
        let mut roots = vec![0];
        for (id, record) in graph.records.iter().enumerate() {
            (builder.check)()?;
            if record.parent.is_none()
                && (record.occurrence || !matches!(record.kind.as_str(), "atomic" | "reference"))
            {
                roots.extend(builder.node(id as u32, false, false, 0)?);
            }
        }
        let Builder {
            ast,
            semantics,
            selections,
            ..
        } = builder;
        // Both the native graph and the semantic index can own source frames,
        // names and provenance. Include those retained copies in the permit.
        let accounted_bytes = graph.accounted_bytes().saturating_mul(2).saturating_add(builder_cost(&ast));
        let retained = retain(accounted_bytes, ast.nodes.len())?;
        (check)()?;
        let owner: Arc<dyn std::any::Any + Send + Sync> = Arc::new((graph.clone(), retained));
        let tree = RetainedCemTree::native_forest(ast, roots, semantics, owner)?;
        Ok(Self {
            graph,
            tree,
            selections,
            accounted_bytes,
        })
    }

    pub fn items(&self, id: u32) -> Result<Vec<XPathResultItem>, String> {
        self.items_with_check(id, &mut || Ok(()))
    }

    pub fn items_with_check(&self, id: u32, check: &mut impl FnMut() -> Result<(), String>) -> Result<Vec<XPathResultItem>, String> {
        check()?;
        let record = self
            .graph
            .records
            .get(id as usize)
            .ok_or("Unknown native value")?;
        if record.kind == "atomic" {
            if record.datatype == "null" {
                return Ok(Vec::new());
            }
            let datatype = match record.datatype.as_str() {
                "number" => "decimal",
                "datetime" => "dateTime",
                other => other,
            };
            return Ok(vec![XPathResultItem::Atomic {
                value: XPathAtomicValue {
                    type_name: format!("xs:{datatype}"),
                    lexical_value: record.lexical.clone(),
                    namespace_uri: None,
                    local_name: None,
                },
                source_map: record.source.clone(),
            }]);
        }
        if record.kind == "reference" && record.parent.is_none() && !record.occurrence {
            let mut result = Vec::new();
            for &target in &record.targets {
                result.extend(self.items_with_check(target, check)?);
            }
            return Ok(result);
        }
        Ok(self.selections[id as usize]
            .iter()
            .filter_map(|&id| {
                // XPath coalesces adjacent text and omits empty text.
                self.tree
                    .canonical_id(id)
                    .and_then(|id| XPathNativeNode::cem_node(self.tree.clone(), id).ok())
                    .map(XPathResultItem::from_native_node)
            })
            .collect())
    }
}

fn builder_cost(ast: &CemDocument) -> usize {
    // Node arena plus semantic/index slots and source/name/text allocations.
    // The input graph already accounts for lexical payloads; allow a second
    // copy for the semantic representation and index maps.
    ast.nodes.len().saturating_mul(std::mem::size_of::<CemAstNode>() + 256)
}

struct Builder<'a, F> {
    graph: &'a CemValueGraph,
    limits: &'a CemValueArtifactLimits,
    check: &'a mut F,
    ast: CemDocument,
    semantics: CemTreeSemantics,
    selections: Vec<Vec<u32>>,
    bytes: usize,
}
impl<F: FnMut() -> Result<(), String>> Builder<'_, F> {
    fn node(
        &mut self,
        record_id: u32,
        occurrence: bool,
        content: bool,
        depth: usize,
    ) -> Result<Vec<u32>, String> {
        (self.check)()?;
        if depth > self.limits.max_depth {
            return Err("Native XPath projection depth exceeded".into());
        }
        let record = &self.graph.records[record_id as usize];
        if record.kind == "reference" || content && record.kind == "document" {
            let children = if record.kind == "reference" {
                &record.targets
            } else {
                &record.children
            };
            let mut nodes = Vec::new();
            for &child in children {
                nodes.extend(self.node(child, true, true, depth + 1)?);
            }
            if !occurrence {
                self.selections[record_id as usize] = nodes.clone();
            }
            return Ok(nodes);
        }
        if content && record.kind == "attribute" {
            return Err("An attribute cannot be inserted as XPath child content".into());
        }
        if self.ast.nodes.len() >= self.limits.max_values {
            return Err("Native XPath projection value limit exceeded".into());
        }
        let id =
            u32::try_from(self.ast.nodes.len()).map_err(|_| "Native XPath identity overflow")?;
        let source = record.source.clone();
        self.ast.nodes.push(CemAstNode::Text {
            node_id: id,
            data: String::new(),
            source: source.clone(),
        });
        let name = ExpandedName {
            namespace_uri: record.namespace.clone(),
            local_name: record.name.rsplit(':').next().unwrap_or("").into(),
            schema_id: None,
        };
        let value = if record.kind == "attribute" && record.has_native_content() {
            self.graph.string_value(record_id, self.limits.max_bytes)?
        } else {
            record.lexical.clone()
        };
        self.bytes = self
            .bytes
            .saturating_add(value.len() + record.name.len() + record.namespace.len());
        if self.bytes > self.limits.max_bytes {
            return Err("Native XPath projection byte limit exceeded".into());
        }
        let node = match record.kind.as_str() {
            "document" | "element" => {
                let mut attributes = Vec::new();
                for &attribute in &record.attributes {
                    let attr = &self.graph.records[attribute as usize];
                    if attr.namespace == "http://www.w3.org/2000/xmlns/"
                        || attr.name == "xmlns"
                        || attr.name.starts_with("xmlns:")
                    {
                        continue;
                    }
                    attributes.extend(self.node(attribute, occurrence, false, depth + 1)?);
                }
                let mut children = Vec::new();
                for &child in &record.children {
                    children.extend(self.node(child, occurrence, true, depth + 1)?);
                }
                if record.kind == "document" {
                    CemAstNode::Document {
                        node_id: id,
                        root_children: children,
                        source,
                    }
                } else {
                    CemAstNode::Element {
                        node_id: id,
                        expanded_name: name,
                        attributes,
                        children,
                        has_explicit_boundary: true,
                        source,
                    }
                }
            }
            "attribute" => CemAstNode::Attribute {
                node_id: id,
                expanded_name: name,
                value: Some(value),
                source,
            },
            "comment" => CemAstNode::Comment {
                node_id: id,
                data: value,
                source,
            },
            "processing-instruction" => CemAstNode::ProcessingInstruction {
                node_id: id,
                target: record.name.clone(),
                data: value,
                source,
            },
            _ => CemAstNode::Text {
                node_id: id,
                data: value,
                source,
            },
        };
        self.ast.nodes[id as usize] = node;
        if let Some(origin) = &record.provenance {
            self.semantics.provenance.insert(id, origin.clone());
            if let Some(uri) = &origin.source_uri {
                self.semantics.base_uris.insert(id, uri.clone());
            }
        }
        if !occurrence {
            self.selections[record_id as usize] = vec![id];
        }
        Ok(vec![id])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::artifact::CemValueRecord;

    #[test]
    fn native_value_xpath_projection_enforces_limits_and_cancellation() {
        let graph = Arc::new(CemValueGraph {
            roots: vec![0],
            records: vec![
                CemValueRecord {
                    kind: "element".into(),
                    name: "r".into(),
                    children: vec![1],
                    ..Default::default()
                },
                CemValueRecord {
                    kind: "text".into(),
                    lexical: "text".into(),
                    parent: Some(0),
                    ..Default::default()
                },
            ],
        });
        let limits = CemValueArtifactLimits::default();
        let projection =
            CemValueXPathProjection::build(graph.clone(), &limits, &mut || Ok(())).unwrap();
        let node = projection
            .items(0)
            .unwrap()
            .remove(0)
            .native_node()
            .unwrap()
            .clone();
        assert_eq!(node.string_value(), "text");
        assert!(node.parent_node().is_none());
        assert!(CemValueXPathProjection::build(
            graph.clone(),
            &CemValueArtifactLimits {
                max_values: 2,
                ..limits
            },
            &mut || Ok(())
        )
        .is_err());
        assert!(CemValueXPathProjection::build(
            graph.clone(),
            &CemValueArtifactLimits {
                max_bytes: 3,
                ..limits
            },
            &mut || Ok(())
        )
        .is_err());
        let mut steps = 0;
        let cancelled = CemValueXPathProjection::build(graph, &limits, &mut || {
            steps += 1;
            if steps > 1 {
                Err("cancelled".into())
            } else {
                Ok(())
            }
        });
        assert_eq!(cancelled.unwrap_err(), "cancelled");
    }
}
