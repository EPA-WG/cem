use super::*;
use crate::validation::json::{self as parser, JsonMemberAst, JsonValueAst as J};
use std::collections::BTreeSet;
fn bad(text: impl Into<String>) -> ProjectionError {
    ProjectionError::new("json_shape", text)
}
fn object(v: &J) -> Result<&[JsonMemberAst]> {
    let J::Object { members, .. } = v else {
        return Err(bad("Expected projection object"));
    };
    let mut names = BTreeSet::new();
    if members.iter().any(|m| !names.insert(&m.name)) {
        return Err(bad("Duplicate projection object field"));
    }
    Ok(members)
}
fn get<'a>(o: &'a [JsonMemberAst], key: &str) -> Option<&'a J> {
    o.iter().find(|m| m.name == key).map(|m| &m.value)
}
fn required<'a>(o: &'a [JsonMemberAst], key: &str) -> Result<&'a J> {
    get(o, key).ok_or_else(|| bad(format!("Missing {key}")))
}
fn string(v: &J) -> Result<&str> {
    if let J::String { value, .. } = v {
        Ok(value)
    } else {
        Err(bad("Expected string"))
    }
}
fn text<'a>(o: &'a [JsonMemberAst], key: &str) -> Result<&'a str> {
    string(required(o, key)?)
}
fn unsigned(v: &J) -> Result<u64> {
    if let J::Number { lexeme, .. } = v {
        lexeme.parse().map_err(|_| bad("Expected unsigned integer"))
    } else {
        Err(bad("Expected unsigned integer"))
    }
}
fn array(v: &J) -> Result<&[J]> {
    if let J::Array { items, .. } = v {
        Ok(items)
    } else {
        Err(bad("Expected array"))
    }
}
fn range(v: &J) -> Result<()> {
    if matches!(v, J::Null { .. }) {
        return Ok(());
    }
    let o = object(v)?;
    let start = unsigned(required(o, "start")?)?;
    let len = unsigned(required(o, "len")?)?;
    if len > u32::MAX as u64 || start.checked_add(len).is_none() {
        return Err(bad("Invalid reported byte range"));
    }
    Ok(())
}
fn coordinates(v: &J, uri: &str) -> SourceMapStack {
    let r = v.range();
    source(uri, r.start.byte_offset as usize, r.byte_length as usize)
}
fn check_limits(root: &J) -> Result<()> {
    let mut pending = vec![(root, 0)];
    let mut n = 0;
    while let Some((v, d)) = pending.pop() {
        n += 1;
        if n > LIMIT || d > DEPTH {
            return Err(ProjectionError::new(
                "source_limit",
                "Projection JSON exceeds bounded values/depth",
            ));
        }
        match v {
            J::Object { members, .. } => pending.extend(members.iter().map(|m| (&m.value, d + 1))),
            J::Array { items, .. } => pending.extend(items.iter().map(|v| (v, d + 1))),
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn decode(
    bytes: &[u8],
    kind: ProjectionKind,
    uri: &str,
) -> Result<(CemDocument, Option<Arc<parser::JsonDocumentAst>>)> {
    std::str::from_utf8(bytes)
        .map_err(|e| ProjectionError::new("json_parse_error", e.to_string()))?;
    let (document, diagnostics) = parser::json_document_ast_from_source_bytes_bounded(
        parser::JsonSourceValidationRequest {
            bytes,
            source_uri: uri,
            content_type: Some(JSON_CONTENT_TYPE),
        },
        DEPTH,
        LIMIT,
    );
    if let Some(error) = diagnostics.iter().find(|d| d.severity.is_hard_violation()) {
        return Err(ProjectionError::new(
            if error.code == "cem.json.resource_limit" {
                "source_limit"
            } else {
                "json_parse_error"
            },
            error.message.clone(),
        ));
    }
    let owner = Arc::new(document.ok_or_else(|| bad("Missing projection JSON owner"))?);
    let root = owner
        .root
        .as_ref()
        .ok_or_else(|| bad("Empty projection JSON"))?;
    check_limits(root)?;
    let mut ast = if matches!(root, J::Object { .. })
        && get(object(root)?, "kind")
            .map(string)
            .transpose()?
            .is_some_and(|s| s == "cem-binary-projection")
    {
        envelope(root, kind, uri)?
    } else if kind == ProjectionKind::Events {
        events(root, kind, uri)?
    } else {
        let mut ast = CemDocument::default();
        node(root, &mut ast, 0, false, 0, uri)?;
        ast
    };
    check_tree(&ast).map_err(|e| bad(e.message))?;
    // Reported numeric handles are data, never a restored runtime/schema context.
    for node in &mut ast.nodes {
        if let CemAstNode::Reference {
            context, targets, ..
        } = node
        {
            *context = 0;
            *targets = None;
        }
    }
    Ok((ast, Some(owner)))
}
fn push(ast: &mut CemDocument, node: CemAstNode) -> Result<u32> {
    if ast.nodes.len() >= LIMIT {
        return Err(ProjectionError::new(
            "source_limit",
            "Projection node limit exceeded",
        ));
    }
    let id = ast.nodes.len() as u32;
    ast.nodes.push(node);
    Ok(id)
}
fn node(
    v: &J,
    ast: &mut CemDocument,
    parent: u32,
    attribute: bool,
    depth: usize,
    uri: &str,
) -> Result<u32> {
    if depth > DEPTH {
        return Err(ProjectionError::new(
            "source_limit",
            "Projection nesting exceeds 64 levels",
        ));
    }
    let o = object(v)?;
    if let Some(r) = get(o, "byteRange") {
        range(r)?;
    }
    let id = ast.nodes.len() as u32;
    let map = coordinates(v, uri);
    if attribute {
        let ns = get(o, "namespace").map(string).transpose()?.unwrap_or("");
        let value = match get(o, "value") {
            None | Some(J::Null { .. }) => None,
            Some(v) => Some(string(v)?.to_owned()),
        };
        push(
            ast,
            CemAstNode::Attribute {
                node_id: id,
                expanded_name: name(ns.into(), text(o, "name")?.into()),
                value,
                value_nodes: vec![],
                source: map,
            },
        )?;
        let mut values = vec![];
        if let Some(v) = get(o, "valueNodes") {
            for child in array(v)? {
                values.push(node(child, ast, id, false, depth + 1, uri)?);
            }
        }
        if let CemAstNode::Attribute { value_nodes, .. } = &mut ast.nodes[id as usize] {
            *value_nodes = values
        }
        return Ok(id);
    }
    match text(o, "kind")? {
        "document" => {
            if id != 0 {
                return Err(bad("Only the root may be a document"));
            }
            push(
                ast,
                CemAstNode::Document {
                    node_id: id,
                    root_children: vec![],
                    source: map,
                },
            )?;
            let mut children = vec![];
            for child in array(required(o, "children")?)? {
                children.push(node(child, ast, id, false, depth + 1, uri)?);
            }
            if let CemAstNode::Document { root_children, .. } = &mut ast.nodes[id as usize] {
                *root_children = children
            }
        }
        "element" => {
            let ns = get(o, "namespace").map(string).transpose()?.unwrap_or("");
            push(
                ast,
                CemAstNode::Element {
                    node_id: id,
                    expanded_name: name(ns.into(), text(o, "name")?.into()),
                    attributes: vec![],
                    children: vec![],
                    has_explicit_boundary: true,
                    source: map,
                },
            )?;
            let mut attributes = vec![];
            for attr in array(required(o, "attributes")?)? {
                attributes.push(node(attr, ast, id, true, depth + 1, uri)?);
            }
            let mut children = vec![];
            for child in array(required(o, "children")?)? {
                children.push(node(child, ast, id, false, depth + 1, uri)?);
            }
            if let CemAstNode::Element {
                attributes: a,
                children: c,
                ..
            } = &mut ast.nodes[id as usize]
            {
                *a = attributes;
                *c = children
            }
        }
        "reference" => {
            if unsigned(required(o, "context")?)? > u32::MAX as u64 {
                return Err(bad("Invalid reported reference context"));
            }
            if let Some(v) = get(o, "targets").filter(|v| !matches!(v, J::Null { .. })) {
                for target in array(v)? {
                    if unsigned(target)? > u32::MAX as u64 {
                        return Err(bad("Invalid reported reference ID"));
                    }
                }
            }
            push(
                ast,
                CemAstNode::Reference {
                    node_id: id,
                    expression: text(o, "expression")?.into(),
                    context: parent,
                    targets: None,
                    source: map,
                },
            )?;
        }
        kind => {
            let n = match kind {
                "text" => CemAstNode::Text {
                    node_id: id,
                    data: text(o, "data")?.into(),
                    source: map,
                },
                "whitespace" => CemAstNode::Whitespace {
                    node_id: id,
                    data: text(o, "data")?.into(),
                    source: map,
                },
                "comment" => CemAstNode::Comment {
                    node_id: id,
                    data: text(o, "data")?.into(),
                    source: map,
                },
                "cdata" => CemAstNode::Cdata {
                    node_id: id,
                    data: text(o, "data")?.into(),
                    source: map,
                },
                "raw-text" => CemAstNode::RawText {
                    node_id: id,
                    data: text(o, "data")?.into(),
                    source: map,
                },
                "processing-instruction" => CemAstNode::ProcessingInstruction {
                    node_id: id,
                    target: text(o, "target")?.into(),
                    data: text(o, "data")?.into(),
                    source: map,
                },
                "error" => CemAstNode::Error {
                    node_id: id,
                    code: text(o, "code")?.into(),
                    source: map,
                },
                _ => return Err(bad("Unsupported projection node kind")),
            };
            push(ast, n)?;
        }
    }
    Ok(id)
}
fn events(root: &J, kind: ProjectionKind, uri: &str) -> Result<CemDocument> {
    let mut doc = ImportBuilder::new().ast;
    set_source(&mut doc.nodes[0], coordinates(root, uri));
    for (index, event) in array(root)?.iter().enumerate() {
        let o = object(event)?;
        let k = text(o, "kind")?;
        let mut fields = vec![("sequence", index.to_string()), ("kind", k.into())];
        if let Some(v) = get(o, "byteRange") {
            range(v)?;
            if !matches!(v, J::Null { .. }) {
                let r = object(v)?;
                fields.extend([
                    (
                        "reported-offset",
                        unsigned(required(r, "start")?)?.to_string(),
                    ),
                    (
                        "reported-length",
                        unsigned(required(r, "len")?)?.to_string(),
                    ),
                ]);
            }
        } else if k != "mode-switch" {
            return Err(bad("Event requires byteRange"));
        }
        match k {
            "open" | "close" | "name" => fields.push(("name", text(o, "name")?.into())),
            "value" => {
                let value = required(o, "value")?;
                let syntax = get(o, "valueSyntax").map(string).transpose()?;
                let (value_kind, scalar) = match value {
                    J::String { value, .. } => (
                        if syntax == Some("expression") {
                            "expression"
                        } else {
                            "text"
                        },
                        value.clone(),
                    ),
                    J::Number { lexeme, .. } => ("number", lexeme.clone()),
                    J::Boolean { value, .. } => ("boolean", value.to_string()),
                    J::Null { .. } => ("null", "null".into()),
                    J::Object { .. } => ("typed-prelude", prelude(value, &mut fields)?),
                    _ => return Err(bad("Invalid event value")),
                };
                if syntax.is_some_and(|s| s != "expression")
                    || syntax.is_some() && value_kind != "expression"
                {
                    return Err(bad("Invalid valueSyntax"));
                }
                fields.extend([("value-kind", value_kind.into()), ("value", scalar)]);
            }
            "trivia" => {
                let trivia = text(o, "trivia")?;
                if !matches!(trivia, "comment" | "whitespace") {
                    return Err(bad("Invalid trivia kind"));
                }
                fields.extend([("trivia", trivia.into()), ("data", text(o, "data")?.into())]);
            }
            "processing-instruction" => fields.extend([
                ("target", text(o, "target")?.into()),
                ("data", text(o, "data")?.into()),
            ]),
            "mode-switch" => fields.push(("content-type", text(o, "contentType")?.into())),
            "error" => fields.push(("code", text(o, "code")?.into())),
            "separator" => {}
            _ => return Err(bad("Invalid event kind")),
        }
        let r = event.range();
        binary::push_event(
            &mut doc,
            kind,
            uri,
            r.start.byte_offset as usize,
            r.byte_length as usize,
            fields,
        )?;
    }
    Ok(doc)
}

fn prelude(v: &J, fields: &mut Vec<(&str, String)>) -> Result<String> {
    let o = object(v)?;
    if o.iter().any(|m| {
        !matches!(
            m.name.as_str(),
            "role"
                | "prefix"
                | "role_range"
                | "prefix_range"
                | "value_range"
                | "payload_range"
                | "kind"
                | "expression"
                | "error"
        )
    }) {
        return Err(bad("Unknown typed-prelude field"));
    }
    let role = match text(o, "role")? {
        "SchemaSelector" => "schema-selector",
        "Namespace" => "namespace",
        "DefaultNamespace" => "default-namespace",
        _ => return Err(bad("Invalid prelude role")),
    };
    let kind = match text(o, "kind")? {
        "Reference" => "reference",
        "Expression" => "expression",
        _ => return Err(bad("Invalid prelude kind")),
    };
    fields.extend([("prelude-role", role.into()), ("prelude-kind", kind.into())]);
    for key in ["prefix", "error"] {
        if let Some(v) = get(o, key).filter(|v| !matches!(v, J::Null { .. })) {
            fields.push((key, string(v)?.into()));
        }
    }
    for (key, offset, length) in [
        ("role_range", "prelude-role-offset", "prelude-role-length"),
        (
            "prefix_range",
            "prelude-prefix-offset",
            "prelude-prefix-length",
        ),
        (
            "value_range",
            "prelude-value-offset",
            "prelude-value-length",
        ),
        (
            "payload_range",
            "prelude-payload-offset",
            "prelude-payload-length",
        ),
    ] {
        let v = required(o, key)?;
        range(v)?;
        if !matches!(v, J::Null { .. }) {
            let r = object(v)?;
            fields.extend([
                (offset, unsigned(required(r, "start")?)?.to_string()),
                (length, unsigned(required(r, "len")?)?.to_string()),
            ]);
        } else if key != "prefix_range" {
            return Err(bad("Required prelude range"));
        }
    }
    Ok(text(o, "expression")?.into())
}

fn envelope(root: &J, kind: ProjectionKind, uri: &str) -> Result<CemDocument> {
    let o = object(root)?;
    if text(o, "projection")? != kind.name()
        || text(o, "schema")? != kind.schema()
        || text(o, "contentType")? != kind.binary_mime()
    {
        return Err(bad("Envelope projection identity mismatch"));
    }
    if text(o, "hashScheme")? != crate::content_cache::HASH_SCHEME {
        return Err(bad("Unsupported projection hash scheme"));
    }
    let version = text(o, "formatVersion")?;
    if !matches!(version, "cem-projection-bin/1" | "cem-projection-bin/2") {
        return Err(bad("Unsupported envelope version"));
    }
    if let Some(v) = get(o, "nativeBytes") {
        if !matches!(v, J::Boolean { .. }) {
            return Err(bad("nativeBytes must be boolean"));
        }
    }
    let length = unsigned(required(o, "byteLength")?)?;
    if length > MAX_DOCUMENT_BYTES as u64 {
        return Err(ProjectionError::new(
            "source_limit",
            "Envelope payload exceeds 16 MiB",
        ));
    }
    let chunks = array(required(o, "chunks")?)?;
    if chunks.is_empty() || chunks.len() > LIMIT + 1 {
        return Err(bad("Envelope requires bounded nonempty chunks"));
    }
    let mut bytes = Vec::new();
    let mut spans = vec![];
    let mut ids = BTreeSet::new();
    let mut links = vec![];
    for chunk in chunks {
        let c = object(chunk)?;
        let id = text(c, "id")?;
        if !ids.insert(id) {
            return Err(bad("Duplicate chunk ID"));
        }
        if !matches!(required(c, "sealed")?, J::Boolean { value: true, .. }) {
            return Err(bad("Unsealed chunk"));
        }
        if unsigned(required(c, "byteOffset")?)? != bytes.len() as u64 {
            return Err(bad("Noncontiguous chunk offsets"));
        }
        let data_value = required(c, "data")?;
        let data = string(data_value)?;
        let count = unsigned(required(c, "byteLength")?)?;
        if count == 0
            || count != data.len() as u64 / 2
            || data.len() % 2 != 0
            || bytes.len() as u64 + count > length
        {
            return Err(bad("Invalid chunk byte length"));
        }
        let start = bytes.len();
        for pair in data.as_bytes().chunks_exact(2) {
            let hex = |c: u8| -> Result<u8> {
                match c {
                    b'0'..=b'9' => Ok(c - b'0'),
                    b'a'..=b'f' => Ok(c - b'a' + 10),
                    b'A'..=b'F' => Ok(c - b'A' + 10),
                    _ => Err(bad("Invalid hex data")),
                }
            };
            bytes.push((hex(pair[0])? << 4) | hex(pair[1])?);
        }
        if text(c, "dataEncoding")? != "hex" || text(c, "hash")? != hash(&bytes[start..]) {
            return Err(bad("Chunk encoding/hash mismatch"));
        }
        spans.push((start, bytes.len(), data_value));
        if let Some(p) = get(c, "parentId") {
            links.push(string(p)?);
        }
        if let Some(v) = get(c, "rootId") {
            string(v)?;
        }
        if let Some(v) = get(c, "childLinks") {
            for link in array(v)? {
                let link = object(link)?;
                links.push(text(link, "chunkId")?);
                text(link, "rootId")?;
            }
        }
        if let Some(v) = get(c, "sourceMapDeltas") {
            for delta in array(v)? {
                let d = object(delta)?;
                if unsigned(required(d, "sourceId")?)? > u32::MAX as u64 {
                    return Err(bad("Invalid reported source ID"));
                }
                let offset = unsigned(required(d, "byteOffset")?)?;
                let len = unsigned(required(d, "byteLength")?)?;
                if len > u32::MAX as u64 {
                    return Err(bad("Invalid reported source length"));
                }
                offset
                    .checked_add(len)
                    .ok_or_else(|| bad("Source-map delta overflow"))?;
                text(d, "transform")?;
            }
        }
    }
    if bytes.len() as u64 != length
        || text(o, "hash")? != hash(&bytes)
        || u16::from_be_bytes(
            bytes
                .get(8..10)
                .ok_or_else(|| bad("Missing binary header"))?
                .try_into()
                .unwrap(),
        )
        .to_string()
            != version.rsplit('/').next().unwrap()
    {
        return Err(bad("Envelope length/hash/version mismatch"));
    }
    if links.iter().any(|id| !ids.contains(id)) {
        return Err(bad("Dangling chunk link"));
    }
    let (mut doc, records) = binary::decode_with_records(&bytes, kind, uri)?;
    check_chunk_layout(chunks, &doc, &records, kind)?;
    for node in &mut doc.nodes {
        let mut old = SourceMapStack::default();
        std::mem::swap(
            &mut old,
            match node {
                CemAstNode::Document { source, .. }
                | CemAstNode::Element { source, .. }
                | CemAstNode::Attribute { source, .. }
                | CemAstNode::Reference { source, .. }
                | CemAstNode::Text { source, .. }
                | CemAstNode::Whitespace { source, .. }
                | CemAstNode::Comment { source, .. }
                | CemAstNode::Cdata { source, .. }
                | CemAstNode::RawText { source, .. }
                | CemAstNode::ProcessingInstruction { source, .. }
                | CemAstNode::Error { source, .. } => source,
            },
        );
        let mut mapped = SourceMapStack::default();
        for frame in old.frames {
            let FrameSpan::Single(range) = frame.span else {
                continue;
            };
            for (start, end, value) in &spans {
                let a = (range.start as usize).max(*start);
                let b = (range.end() as usize).min(*end);
                if a >= b {
                    continue;
                }
                let map = match value {
                    J::String {
                        value,
                        lexeme,
                        range,
                    } if lexeme.len() == value.len() + 2
                        && lexeme.as_bytes().get(1..lexeme.len() - 1) == Some(value.as_bytes()) =>
                    {
                        source(
                            uri,
                            range.start.byte_offset as usize + 1 + (a - start) * 2,
                            (b - a) * 2,
                        )
                    }
                    _ => coordinates(value, uri),
                };
                mapped.frames.extend(map.frames);
            }
        }
        set_source(node, mapped);
    }
    Ok(doc)
}
// Hashes bind bytes; verify the separate chunk topology against decoded owning
// edges as well. Serialized source-map deltas stay descriptive, not authority.
fn check_chunk_layout(
    chunks: &[J],
    doc: &CemDocument,
    records: &[(usize, usize)],
    kind: ProjectionKind,
) -> Result<()> {
    if chunks.len() != records.len() {
        return Err(bad("Chunk count does not match records"));
    }
    let events = kind == ProjectionKind::Events;
    let count = records.len() - 1;
    let id = |index: usize| {
        if index == 0 {
            "root".to_owned()
        } else {
            format!("{}-{}", if events { "event" } else { "node" }, index - 1)
        }
    };
    let root_id = |index: usize| {
        if index == 0 {
            format!("projection:{}", kind.name())
        } else {
            format!("{}:{}", if events { "event" } else { "node" }, index - 1)
        }
    };
    let mut children = vec![vec![]; records.len()];
    let mut parents = vec![0usize; records.len()];
    if events {
        children[0] = (1..records.len()).collect();
    } else {
        children[0].push(1);
        for (index, node) in doc.nodes.iter().enumerate() {
            let list = match node {
                CemAstNode::Document { root_children, .. } => root_children.clone(),
                CemAstNode::Element {
                    attributes,
                    children,
                    ..
                } => attributes.iter().chain(children).copied().collect(),
                CemAstNode::Attribute { value_nodes, .. } => value_nodes.clone(),
                _ => vec![],
            };
            for child in list {
                let target = child as usize + 1;
                if target > count {
                    return Err(bad("Invalid chunk target"));
                }
                children[index + 1].push(target);
                parents[target] = index + 1;
            }
        }
    }
    for (index, (chunk, (start, length))) in chunks.iter().zip(records).enumerate() {
        let c = object(chunk)?;
        if text(c, "id")? != id(index)
            || text(c, "rootId")? != root_id(index)
            || unsigned(required(c, "byteOffset")?)? != *start as u64
            || unsigned(required(c, "byteLength")?)? != *length as u64
        {
            return Err(bad("Chunk identity/boundary does not match decoded record"));
        }
        let parent = get(c, "parentId").map(string).transpose()?;
        if (index == 0 && parent.is_some())
            || index > 0 && parent != Some(id(parents[index]).as_str())
        {
            return Err(bad("Chunk parent does not match owning edge"));
        }
        let links = get(c, "childLinks").map(array).transpose()?.unwrap_or(&[]);
        if links.len() != children[index].len() {
            return Err(bad("Chunk children do not match owning edges"));
        }
        for (link, child) in links.iter().zip(&children[index]) {
            let l = object(link)?;
            if text(l, "chunkId")? != id(*child) || text(l, "rootId")? != root_id(*child) {
                return Err(bad("Chunk link does not match owning edge"));
            }
        }
    }
    Ok(())
}

fn hash(bytes: &[u8]) -> String {
    format!(
        "{}:{}",
        crate::content_cache::HASH_SCHEME,
        blake3::hash(bytes).to_hex()
    )
}
