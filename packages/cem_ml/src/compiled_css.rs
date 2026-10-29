//! Pinned native CSS IR. Only the native importer produces this tree vocabulary;
//! binary admission restores decoded nodes, never parses CSS or grants resources.
use crate::{
    content_cache::ContentHash,
    parser::{
        document::CemDocument,
        tree::{CemTreeNodeKind, CemTreeRange, CemTreeSemantics, RetainedCemTree},
        CemAstNode, ExpandedName,
    },
    schema::registry::CSS_SCHEMA_URI,
    source_map::SourceMapStack,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

const MAGIC: &[u8] = b"CEMCSS1\0";
const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_NODES: usize = 100_000;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capsule {
    version: u8,
    cem_ml_version: String,
    source_uri: String,
    source_hash: String,
    source: String,
    source_maps: Vec<SourceMapStack>,
    strings: Vec<String>,
    nodes: Vec<Node>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Node {
    kind: u8,
    name: u32,
    value: u32,
    children: Vec<u32>,
    attributes: Vec<u32>,
    source: u32,
    range: (u32, u32, u64, u64),
}
pub struct AdmittedCss {
    pub source: String,
    pub tree: Arc<RetainedCemTree>,
}

pub fn encode(tree: &RetainedCemTree, source: &str) -> Result<Vec<u8>, String> {
    if source.len() > MAX_BYTES || tree.ast().nodes.len() > MAX_NODES {
        return Err("compiled CSS limit".into());
    }
    let fingerprint =
        crate::import::compiled_css_fingerprint(tree.source_uri(), source.as_bytes())?;
    if tree.source_key(0)
        != Some(format!(
            "cem-source:1:{}:0",
            blake3::Hash::from_bytes(fingerprint).to_hex()
        ))
    {
        return Err("compiled CSS source does not own the imported tree".into());
    }
    let mut strings = Vec::<String>::new();
    let mut string_ids = HashMap::<String, u32>::new();
    let mut intern = |value: &str| -> u32 {
        if let Some(id) = string_ids.get(value) {
            return *id;
        }
        let id = strings.len() as u32;
        strings.push(value.into());
        string_ids.insert(value.into(), id);
        id
    };
    let mut source_maps = Vec::<SourceMapStack>::new();
    let mut source_ids = HashMap::<Vec<u8>, u32>::new();
    let nodes = (0..tree.ast().nodes.len())
        .map(|id| {
            let node = tree.node(id as u32).ok_or("compiled CSS missing node")?;
            let kind = match node.kind {
                CemTreeNodeKind::Document => 0,
                CemTreeNodeKind::Element => 1,
                CemTreeNodeKind::Attribute => 2,
                _ => return Err("unsupported compiled CSS node"),
            };
            if node.name.as_ref().is_some_and(|name| {
                name.namespace_uri != if kind == 1 { CSS_SCHEMA_URI } else { "" }
            }) {
                return Err("unsupported compiled CSS namespace");
            }
            let key = rmp_serde::to_vec(&node.source).map_err(|_| "invalid CSS source map")?;
            let source = *source_ids.entry(key).or_insert_with(|| {
                let id = source_maps.len() as u32;
                source_maps.push(node.source.clone());
                id
            });
            Ok(Node {
                kind,
                name: intern(node.name.as_ref().map_or("", |n| n.local_name.as_str())),
                value: intern(&node.value),
                children: node.children.clone(),
                attributes: node.attributes.clone(),
                source,
                range: (
                    node.range.line,
                    node.range.column,
                    node.range.offset,
                    node.range.length,
                ),
            })
        })
        .collect::<Result<Vec<_>, &str>>()?;
    let capsule = Capsule {
        version: 1,
        cem_ml_version: crate::VERSION.into(),
        source_uri: tree.source_uri().into(),
        source_hash: ContentHash::from_blake3(source.as_bytes()).header_value(),
        source: source.into(),
        source_maps,
        strings,
        nodes,
    };
    let body = rmp_serde::to_vec(&capsule).map_err(|e| e.to_string())?;
    if body.len() + MAGIC.len() + 32 > MAX_BYTES {
        return Err("compiled CSS limit".into());
    }
    let mut bytes = MAGIC.to_vec();
    bytes.extend(blake3::hash(&body).as_bytes());
    bytes.extend(body);
    Ok(bytes)
}

/// The expected content hash is supplied by the admitted template/build manifest.
pub fn admit(bytes: &[u8], expected_hash: &str) -> Result<AdmittedCss, String> {
    if bytes.len() > MAX_BYTES
        || !bytes.starts_with(MAGIC)
        || bytes.len() < MAGIC.len() + 32
        || ContentHash::from_blake3(bytes).header_value() != expected_hash
    {
        return Err("compiled CSS content hash/format/size mismatch".into());
    }
    let (hash, body) = bytes[MAGIC.len()..].split_at(32);
    if blake3::hash(body).as_bytes() != hash {
        return Err("compiled CSS body integrity mismatch".into());
    }
    let mut cursor = std::io::Cursor::new(body);
    let mut decoder = rmp_serde::Deserializer::new(&mut cursor);
    decoder.set_max_depth(64);
    let capsule = Capsule::deserialize(&mut decoder).map_err(|e| e.to_string())?;
    if cursor.position() as usize != body.len()
        || capsule.version != 1
        || capsule.cem_ml_version != crate::VERSION
        || capsule.nodes.is_empty()
        || capsule.nodes.len() > MAX_NODES
        || capsule.strings.len() > 2 * MAX_NODES
        || capsule.source_maps.len() > MAX_NODES
        || capsule
            .source_maps
            .iter()
            .any(|source| source.frames.len() > 64)
        || capsule.source.len() > MAX_BYTES
        || capsule.source_hash != ContentHash::from_blake3(capsule.source.as_bytes()).header_value()
    {
        return Err("compiled CSS version/source/structure mismatch".into());
    }
    // Interning must not turn a small capsule into unbounded native allocations.
    let source_costs = capsule
        .source_maps
        .iter()
        .map(|source| {
            let bytes = rmp_serde::to_vec(source).map_err(|e| e.to_string())?.len();
            let frames =
                source.frames.len() * std::mem::size_of::<crate::source_map::SourceMapFrame>();
            let ranges = source
                .frames
                .iter()
                .map(|frame| match &frame.span {
                    crate::source_map::FrameSpan::Single(_) => 0,
                    crate::source_map::FrameSpan::Multi(ranges) => {
                        ranges.len() * std::mem::size_of::<crate::source::ByteRange>()
                    }
                })
                .sum::<usize>();
            Ok(bytes + frames + ranges)
        })
        .collect::<Result<Vec<usize>, String>>()?;
    let mut expanded = 0usize;
    let mut ast = CemDocument::default();
    let mut semantics = CemTreeSemantics {
        source_fingerprint: Some(crate::import::compiled_css_fingerprint(
            &capsule.source_uri,
            capsule.source.as_bytes(),
        )?),
        ..Default::default()
    };
    let mut edges = 0usize;
    for (index, node) in capsule.nodes.into_iter().enumerate() {
        let name = capsule
            .strings
            .get(node.name as usize)
            .ok_or("compiled CSS string index")?;
        let value = capsule
            .strings
            .get(node.value as usize)
            .ok_or("compiled CSS string index")?;
        let source = capsule
            .source_maps
            .get(node.source as usize)
            .ok_or("compiled CSS source index")?;
        let cost = name.len()
            + value.len()
            + source_costs[node.source as usize]
            + std::mem::size_of::<CemAstNode>()
            + std::mem::size_of::<crate::parser::tree::CemTreeNode>();
        expanded = expanded
            .checked_add(cost.saturating_mul(2))
            .ok_or("compiled CSS expansion limit")?;
        if expanded > MAX_BYTES {
            return Err("compiled CSS expansion limit".into());
        }
        let id = index as u32;
        edges = edges
            .checked_add(node.children.len() + node.attributes.len())
            .ok_or("compiled CSS edge limit")?;
        if edges > MAX_NODES
            || node
                .range
                .2
                .checked_add(node.range.3)
                .is_none_or(|end| end > capsule.source.len() as u64)
            || node.children.len() > MAX_NODES
            || node.attributes.len() > MAX_NODES
        {
            return Err("compiled CSS range/edge limit".into());
        }
        semantics.ranges.insert(
            id,
            CemTreeRange {
                line: node.range.0,
                column: node.range.1,
                offset: node.range.2,
                length: node.range.3,
            },
        );
        let name = ExpandedName {
            namespace_uri: if node.kind == 1 {
                CSS_SCHEMA_URI.into()
            } else {
                String::new()
            },
            local_name: name.clone(),
            schema_id: None,
        };
        ast.nodes.push(match node.kind {
            0 if id == 0
                && name.local_name.is_empty()
                && value.is_empty()
                && node.attributes.is_empty() =>
            {
                CemAstNode::Document {
                    node_id: id,
                    root_children: node.children,
                    source: source.clone(),
                }
            }
            1 if id != 0 && !name.local_name.is_empty() && value.is_empty() => {
                CemAstNode::Element {
                    node_id: id,
                    expanded_name: name,
                    attributes: node.attributes,
                    children: node.children,
                    has_explicit_boundary: true,
                    source: source.clone(),
                }
            }
            2 if id != 0
                && !name.local_name.is_empty()
                && node.children.is_empty()
                && node.attributes.is_empty() =>
            {
                CemAstNode::Attribute {
                    node_id: id,
                    expanded_name: name,
                    value: Some(value.clone()),
                    value_nodes: vec![],
                    source: source.clone(),
                }
            }
            _ => return Err("compiled CSS node kind mismatch".into()),
        });
    }
    let mut seen = vec![false; ast.nodes.len()];
    let mut pending = vec![(0u32, 0usize)];
    while let Some((id, depth)) = pending.pop() {
        if depth > 256 || id as usize >= seen.len() || seen[id as usize] {
            return Err("compiled CSS graph/depth mismatch".into());
        }
        seen[id as usize] = true;
        let edges = match &ast.nodes[id as usize] {
            CemAstNode::Document { root_children, .. } => root_children.iter().collect::<Vec<_>>(),
            CemAstNode::Element {
                children,
                attributes,
                ..
            } => children.iter().chain(attributes).collect(),
            _ => vec![],
        };
        pending.extend(edges.into_iter().map(|id| (*id, depth + 1)));
    }
    if seen.iter().any(|seen| !seen) {
        return Err("compiled CSS unreachable node".into());
    }
    // The shared native graph constructor validates reachability, edge kinds,
    // duplicate ownership, dangling IDs and cycles before publishing an owner.
    let tree = RetainedCemTree::new(ast, capsule.source_uri, &capsule.source, semantics, None)?;
    let roots = &tree
        .node(0)
        .ok_or("compiled CSS missing document")?
        .children;
    if roots.len() != 1
        || tree
            .node(roots[0])
            .and_then(|n| n.name.as_ref())
            .is_none_or(|n| n.local_name != "style-block")
    {
        return Err("compiled CSS requires a scoped style block".into());
    }
    Ok(AdmittedCss {
        source: capsule.source,
        tree,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        css_emission::{derive_css_stylesheet_identity, emit_css_import_closure, CssManagedScope},
        css_imports::{CssImportClosure, CssImportLimits},
        module_resolution::{
            CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
            CemResolutionContextHandle, CemScopedModuleUrlResolver,
        },
        scheduler::AbortSignal,
    };
    fn tree(css: &str) -> Arc<RetainedCemTree> {
        crate::import::import_data_bytes(
            css.as_bytes(),
            "text/css; mode=scoped-style-block",
            "cem",
            "urn:compiled-css-test",
        )
        .unwrap()
    }
    fn closure(tree: Arc<RetainedCemTree>) -> CssImportClosure {
        let handle = CemResolutionContextHandle::new("css-test");
        let context = CemModuleUrlContext {
            identity: "css-context".into(),
            resolver_identity: "resolver:1".into(),
            resource_policy_stamp: "policy:1".into(),
            frames: vec![CemModuleUrlFrame::new(
                "root",
                "https://app.test/ui/view.xhtml",
            )],
        };
        let capability = CemModuleUrlResolutionCapability::new(
            Arc::new(CemScopedModuleUrlResolver::new().with_context(handle.clone(), context)),
            handle,
        );
        CssImportClosure::new(
            tree,
            "https://app.test/ui/view.xhtml",
            capability,
            CssImportLimits::default(),
            AbortSignal::new(),
        )
        .unwrap()
    }
    #[test]
    fn compiled_tree_preserves_decoded_nodes_ranges_and_source_identity_without_css_parsing() {
        let css = ":scope { --label: 'café'; background: url('../img/a.svg'); }\n:where(.item):hover { color: red; }";
        let before = crate::validation::css::CSS_SOURCE_PARSE_CALLS.with(|n| n.get());
        let original = tree(css);
        let parsed = crate::validation::css::CSS_SOURCE_PARSE_CALLS.with(|n| n.get());
        assert!(parsed > before);
        let bytes = encode(&original, css).unwrap();
        assert_eq!(bytes, encode(&original, css).unwrap());
        let hash = ContentHash::from_blake3(&bytes).header_value();
        let restored = admit(&bytes, &hash).unwrap();
        assert_eq!(
            crate::validation::css::CSS_SOURCE_PARSE_CALLS.with(|n| n.get()),
            parsed
        );
        assert!(!Arc::ptr_eq(&original, &restored.tree));
        assert_eq!(restored.source, css);
        assert_eq!(original.source_uri(), restored.tree.source_uri());
        for id in 0..original.ast().nodes.len() {
            assert_eq!(
                format!("{:?}", original.node(id as u32)),
                format!("{:?}", restored.tree.node(id as u32))
            );
            assert_eq!(
                original.source_key(id as u32),
                restored.tree.source_key(id as u32)
            );
        }
        assert!(encode(&original, "other source").is_err());
    }
    #[test]
    fn scopes_resources_import_requests_and_diagnostics_match_the_source_path() {
        let css = "@import './base.css' screen; :scope { background: url('../img/a.svg'); animation: pulse 1s; } @keyframes pulse { to { opacity: .5; } } #forbidden { color:red; }";
        let original = tree(css);
        let bytes = encode(&original, css).unwrap();
        let restored = admit(&bytes, &ContentHash::from_blake3(&bytes).header_value()).unwrap();
        for scope in [
            CssManagedScope::Private {
                tag: "cem-test".into(),
                context: None,
            },
            CssManagedScope::Shared {
                name: "shared".into(),
                context: None,
            },
            CssManagedScope::Instance,
        ] {
            let mut source = closure(original.clone());
            let mut compiled = closure(restored.tree.clone());
            let first = source.next_import().unwrap().unwrap();
            let second = compiled.next_import().unwrap().unwrap();
            assert_eq!(
                first.resolution.resolved_url,
                second.resolution.resolved_url
            );
            assert_eq!(
                first.resolution.resolved_url,
                "https://app.test/ui/base.css"
            );
            // Existing authorized external text/css imports remain available.
            for (graph, request) in [(&mut source, first), (&mut compiled, second)] {
                graph
                    .complete_response(
                        request.id,
                        crate::resolver::ResolvedRead {
                            uri: request.resolution.resolved_url.clone(),
                            bytes: b".base { color: blue; }".to_vec(),
                            content_type: Some("text/css".into()),
                        },
                        &Default::default(),
                    )
                    .unwrap();
            }
            let a = derive_css_stylesheet_identity(&source, &scope, "decl:1", "style:0").unwrap();
            let b = derive_css_stylesheet_identity(&compiled, &scope, "decl:1", "style:0").unwrap();
            assert_eq!(a, b);
            let a = emit_css_import_closure(&source, &a.scope, &a.owner_key).unwrap();
            let b = emit_css_import_closure(&compiled, &b.scope, &b.owner_key).unwrap();
            assert_eq!(a.css(), b.css());
            assert!(!a.diagnostics.is_empty());
            assert_eq!(
                format!("{:?}", a.diagnostics),
                format!("{:?}", b.diagnostics)
            );
            assert!(a.css().contains("https://app.test/img/a.svg"));
        }
    }
    #[test]
    fn hashes_versions_ranges_and_cycles_fail_closed() {
        let css = ":scope { color:red; }";
        let bytes = encode(&tree(css), css).unwrap();
        assert!(admit(&bytes, "").is_err());
        let mut corrupt = bytes.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(admit(&corrupt, &ContentHash::from_blake3(&corrupt).header_value()).is_err());
        for which in 0..8 {
            let body = &bytes[MAGIC.len() + 32..];
            let mut capsule: Capsule = rmp_serde::from_slice(body).unwrap();
            match which {
                0 => capsule.version = 2,
                1 => capsule.cem_ml_version = "foreign".into(),
                2 => capsule.source_hash = "wrong".into(),
                3 => capsule.nodes[0].children = vec![0],
                4 => capsule.nodes[0].range.3 = u64::MAX,
                5 => capsule.nodes[0].source = u32::MAX,
                6 => capsule.nodes[0].name = u32::MAX,
                _ => {
                    let id = capsule.strings.len() as u32; capsule.strings.push("x".repeat(8 * 1024 * 1024));
                    for node in &mut capsule.nodes { if node.kind == 2 { node.value = id; } }
                },
            }
            let body = rmp_serde::to_vec(&capsule).unwrap();
            let mut altered = MAGIC.to_vec();
            altered.extend(blake3::hash(&body).as_bytes());
            altered.extend(body);
            assert!(admit(&altered, &ContentHash::from_blake3(&altered).header_value()).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(admit(
            &trailing,
            &ContentHash::from_blake3(&trailing).header_value()
        )
        .is_err());
    }
}
