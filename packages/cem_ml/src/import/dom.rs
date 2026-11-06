//! Primitive live-DOM observations enter here, never through a JSON DOM or HTML
//! serialization. Coordinates name a runtime capture, not authored source bytes.
use super::*;
use crate::value::artifact::{CemValueArtifactLimits, CemValueProvenance};
use std::collections::BTreeSet;

pub struct DomCapture {
    ast: CemDocument,
    semantics: CemTreeSemantics,
    uri: String,
    keys: BTreeSet<String>,
    depths: Vec<usize>,
    bytes: usize,
    limits: CemValueArtifactLimits,
    failed: bool,
}
impl DomCapture {
    pub fn new(
        uri: &str,
        document_uri: &str,
        base_uri: &str,
        limits: CemValueArtifactLimits,
    ) -> Result<Self, String> {
        if uri
            .len()
            .saturating_add(document_uri.len())
            .saturating_add(base_uri.len())
            > limits.max_bytes.min(MAX_DOCUMENT_BYTES)
        {
            return Err("DOM capture identity byte limit exceeded".into());
        }
        if !url::Url::parse(uri).is_ok_and(|u| u.scheme() == "cem-dom" && !u.cannot_be_a_base())
            || [document_uri, base_uri]
                .iter()
                .any(|uri| !uri.is_empty() && url::Url::parse(uri).is_err())
        {
            return Err("Invalid DOM capture source identity".into());
        }
        let mut capture = Self {
            ast: CemDocument::default(),
            semantics: CemTreeSemantics::default(),
            uri: uri.into(),
            keys: BTreeSet::new(),
            depths: Vec::new(),
            bytes: 0,
            limits: CemValueArtifactLimits {
                max_bytes: limits.max_bytes.min(MAX_DOCUMENT_BYTES),
                max_values: limits.max_values.min(MAX_DOCUMENT_VALUES),
                max_depth: limits.max_depth.min(MAX_DEPTH),
            },
            failed: false,
        };
        capture.semantics.document_metadata = Some(crate::parser::tree::CemDocumentMetadata {
            document_uri: (!document_uri.is_empty()).then(|| document_uri.into()),
            base_uri: (!base_uri.is_empty()).then(|| base_uri.into()),
        });
        Ok(capture)
    }
    /// Preorder observations of actual DOM nodes; attributes use their owning
    /// element as parent. The native importer owns all arena IDs and mapping.
    pub fn push(
        &mut self,
        parent: u32,
        key: &str,
        kind: &str,
        namespace: &str,
        name: &str,
        value: &str,
    ) -> Result<u32, String> {
        if self.failed {
            return Err("DOM capture already failed".into());
        }
        let result = self.push_inner(parent, key, kind, namespace, name, value);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn push_inner(
        &mut self,
        parent: u32,
        key: &str,
        kind: &str,
        namespace: &str,
        name: &str,
        value: &str,
    ) -> Result<u32, String> {
        let id = self.ast.nodes.len() as u32;
        let depth = if id == 0 {
            0
        } else {
            self.depths
                .get(parent as usize)
                .ok_or("Invalid DOM capture parent")?
                + 1
        };
        let bytes = [key, kind, namespace, name, value]
            .iter()
            .try_fold(self.bytes, |sum, s| sum.checked_add(s.len()))
            .ok_or("DOM capture byte limit exceeded")?;
        if key.is_empty()
            || self.keys.contains(key)
            || id as usize >= self.limits.max_values
            || depth > self.limits.max_depth
            || bytes > self.limits.max_bytes
        {
            return Err("DOM capture identity or bounds exceeded".into());
        }
        if id == 0 && parent != 0 {
            return Err("Invalid DOM capture root parent".into());
        }
        if id == 0 && kind != "document" || id != 0 && kind == "document" {
            return Err("DOM capture requires one document owner".into());
        }
        if id != 0
            && !matches!(
                self.ast.get(parent),
                Some(CemAstNode::Document { .. } | CemAstNode::Element { .. })
            )
        {
            return Err("Invalid DOM capture parent kind".into());
        }
        if kind == "attribute" && !matches!(self.ast.get(parent), Some(CemAstNode::Element { .. }))
        {
            return Err("DOM attribute requires an element owner".into());
        }
        if let Some(CemAstNode::Element { attributes, .. }) = self.ast.get(parent) {
            if kind == "attribute" && attributes.iter().any(|id| matches!(self.ast.get(*id), Some(CemAstNode::Attribute {expanded_name, ..}) if expanded_name.namespace_uri == namespace && expanded_name.local_name == name)) { return Err("Duplicate DOM attribute".into()); }
        }
        let source = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Multi(Vec::new()),
                transform: TransformKind::SourceBoundary {
                    uri: self.uri.clone(),
                },
            }],
        };
        let expanded_name = ExpandedName {
            namespace_uri: namespace.into(),
            local_name: name.into(),
            schema_id: None,
        };
        if matches!(kind, "element" | "attribute")
            && (name.is_empty() || name.chars().any(|c| c.is_whitespace() || c.is_control()))
        {
            return Err("Invalid DOM expanded name".into());
        }
        let node = match kind {
            "document" => CemAstNode::Document {
                node_id: id,
                root_children: Vec::new(),
                source,
            },
            "element" => CemAstNode::Element {
                node_id: id,
                expanded_name,
                attributes: Vec::new(),
                children: Vec::new(),
                has_explicit_boundary: true,
                source,
            },
            "attribute" => CemAstNode::Attribute {
                node_id: id,
                expanded_name,
                value: Some(value.into()),
                value_nodes: Vec::new(),
                source,
            },
            "text" => CemAstNode::Text {
                node_id: id,
                data: value.into(),
                source,
            },
            "comment" => CemAstNode::Comment {
                node_id: id,
                data: value.into(),
                source,
            },
            _ => return Err("Unsupported DOM capture node kind".into()),
        };
        if id != 0 {
            match &mut self.ast.nodes[parent as usize] {
                CemAstNode::Document { root_children, .. } => root_children.push(id),
                CemAstNode::Element {
                    attributes,
                    children,
                    ..
                } => {
                    if kind == "attribute" {
                        attributes.push(id)
                    } else {
                        children.push(id)
                    }
                }
                _ => unreachable!(),
            }
        }
        self.ast.nodes.push(node);
        self.semantics.provenance.insert(
            id,
            CemValueProvenance {
                source_uri: Some(self.uri.clone()),
                source_key: Some(key.into()),
                line_number: None,
            },
        );
        self.semantics.unknown_source_lines.insert(id);
        self.keys.insert(key.into());
        self.depths.push(depth);
        self.bytes = bytes;
        Ok(id)
    }
    pub fn finish(self) -> Result<Arc<RetainedCemTree>, String> {
        if self.failed || self.ast.nodes.is_empty() {
            return Err("Incomplete DOM capture".into());
        }
        RetainedCemTree::new(self.ast, self.uri, "", self.semantics, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dom_capture_retains_decoded_values_identity_and_unknown_coordinates() {
        let mut c = DomCapture::new(
            "cem-dom://runtime/one",
            "https://example.test/page",
            "https://example.test/base/",
            Default::default(),
        )
        .unwrap();
        let root = c.push(0, "dom-1", "document", "", "", "").unwrap();
        let b = c
            .push(
                root,
                "dom-2",
                "element",
                "http://www.w3.org/1999/xhtml",
                "b",
                "",
            )
            .unwrap();
        let attr = c
            .push(b, "dom-3", "attribute", "", "slot", "heading")
            .unwrap();
        let text = c.push(b, "dom-4", "text", "", "", "<&amp;\r\nβ").unwrap();
        let t = c.finish().unwrap();
        assert_eq!(t.node(text).unwrap().value, "<&amp;\r\nβ");
        assert_eq!(t.source_parent(text), Some(b));
        assert_eq!(t.source_parent(attr), Some(b));
        assert_eq!(t.source_key(text).as_deref(), Some("dom-4"));
        assert_eq!(t.source_line_number(text), None);
        assert!(
            matches!(t.node(text).unwrap().source.frames[0].span,FrameSpan::Multi(ref spans) if spans.is_empty())
        );
    }
    #[test]
    fn dom_capture_preserves_foreign_names_and_comments() {
        let mut c =
            DomCapture::new("cem-dom://runtime/foreign", "", "", Default::default()).unwrap();
        c.push(0, "root", "document", "", "", "").unwrap();
        let svg = c
            .push(0, "svg", "element", "http://www.w3.org/2000/svg", "svg", "")
            .unwrap();
        let attr = c
            .push(svg, "viewBox", "attribute", "", "viewBox", "0 0 20 20")
            .unwrap();
        let comment = c
            .push(svg, "comment", "comment", "", "", "&amp; literal")
            .unwrap();
        let tree = c.finish().unwrap();
        assert_eq!(
            tree.node(svg).unwrap().name.as_ref().unwrap().namespace_uri,
            "http://www.w3.org/2000/svg"
        );
        assert_eq!(
            tree.node(attr).unwrap().name.as_ref().unwrap().local_name,
            "viewBox"
        );
        assert_eq!(tree.node(comment).unwrap().value, "&amp; literal");
    }
    #[test]
    fn dom_capture_enforces_bytes_depth_and_unique_attributes() {
        for limits in [
            CemValueArtifactLimits {
                max_values: 10,
                max_bytes: 30,
                max_depth: 5,
            },
            CemValueArtifactLimits {
                max_values: 10,
                max_bytes: 1000,
                max_depth: 0,
            },
        ] {
            let mut c = DomCapture::new("cem-dom://runtime/one", "", "", limits).unwrap();
            c.push(0, "root", "document", "", "", "").unwrap();
            assert!(c
                .push(
                    0,
                    "child",
                    "text",
                    "",
                    "",
                    "a deliberately long decoded value"
                )
                .is_err());
        }
        let mut c = DomCapture::new("cem-dom://runtime/one", "", "", Default::default()).unwrap();
        c.push(0, "root", "document", "", "", "").unwrap();
        let b = c.push(0, "b", "element", "", "b", "").unwrap();
        c.push(b, "slot1", "attribute", "", "slot", "one").unwrap();
        assert!(c.push(b, "slot2", "attribute", "", "slot", "two").is_err());
    }
    #[test]
    fn dom_capture_bounds_and_invalid_observations_poison_the_owner() {
        for (parent, key, kind) in [
            (90, "child", "text"),
            (0, "root", "text"),
            (0, "child", "record"),
        ] {
            let mut c =
                DomCapture::new("cem-dom://runtime/one", "", "", Default::default()).unwrap();
            c.push(0, "root", "document", "", "", "").unwrap();
            assert!(c.push(parent, key, kind, "", "", "").is_err());
            assert!(c.finish().is_err());
        }
        let mut c = DomCapture::new(
            "cem-dom://runtime/one",
            "",
            "",
            CemValueArtifactLimits {
                max_values: 1,
                max_bytes: 100,
                max_depth: 2,
            },
        )
        .unwrap();
        c.push(0, "root", "document", "", "", "").unwrap();
        assert!(c.push(0, "child", "text", "", "", "x").is_err());
    }
}
