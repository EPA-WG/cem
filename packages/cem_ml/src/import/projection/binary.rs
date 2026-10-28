use super::*;

pub(super) struct Reader<'a> {
    pub bytes: &'a [u8],
    pub offset: usize,
}
impl<'a> Reader<'a> {
    fn error(&self, text: &str) -> ProjectionError {
        ProjectionError::new(
            "binary_payload",
            format!("{text} at projection byte {}", self.offset),
        )
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(n)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| {
                ProjectionError::new(
                    "binary_truncated",
                    format!("Truncated projection at byte {}", self.offset),
                )
            })?;
        let out = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn boolean(&mut self) -> Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(self.error("Invalid boolean tag")),
        }
    }
    fn string(&mut self) -> Result<String> {
        let length = self.u32()? as usize;
        let value = self.take(length)?;
        std::str::from_utf8(value)
            .map(str::to_owned)
            .map_err(|_| self.error("Invalid UTF-8 string"))
    }
    fn optional(&mut self) -> Result<Option<String>> {
        if self.boolean()? {
            self.string().map(Some)
        } else {
            Ok(None)
        }
    }
    fn range(&mut self) -> Result<Option<ByteRange>> {
        if !self.boolean()? {
            return Ok(None);
        }
        let start = self.u64()?;
        let len = self.u32()?;
        start
            .checked_add(u64::from(len))
            .ok_or_else(|| self.error("Source range overflow"))?;
        Ok(Some(ByteRange::new(start, len)))
    }
    fn ids(&mut self, count: usize) -> Result<Vec<u32>> {
        let n = self.u32()? as usize;
        if n > LIMIT || n > self.bytes.len().saturating_sub(self.offset) / 4 {
            return Err(self.error("Invalid ID list length"));
        }
        let mut ids = Vec::with_capacity(n);
        for _ in 0..n {
            let id = self.u32()?;
            if id as usize >= count {
                return Err(self.error("Dangling node ID"));
            }
            ids.push(id)
        }
        Ok(ids)
    }
    fn name(&mut self) -> Result<ExpandedName> {
        let namespace = self.string()?;
        let local = self.string()?;
        if local.is_empty() {
            return Err(self.error("Empty node name"));
        }
        if self.boolean()? {
            self.u32()?;
        }
        Ok(name(namespace, local))
    }
    fn qname(&mut self) -> Result<String> {
        let lexical = self.string()?;
        self.optional()?;
        self.string()?;
        self.range()?;
        Ok(lexical)
    }
    fn tag(&mut self, max: u8) -> Result<u8> {
        let tag = self.u8()?;
        if tag == 0 || tag > max {
            return Err(self.error("Unknown enum tag"));
        }
        Ok(tag)
    }
}
pub(super) fn decode(bytes: &[u8], kind: ProjectionKind, uri: &str) -> Result<CemDocument> {
    decode_with_records(bytes, kind, uri).map(|(doc, _)| doc)
}
pub(super) fn decode_with_records(
    bytes: &[u8],
    kind: ProjectionKind,
    uri: &str,
) -> Result<(CemDocument, Vec<(usize, usize)>)> {
    if !bytes.starts_with(b"CEMPROJ\0") {
        return Err(ProjectionError::new(
            "binary_magic",
            "Projection requires CEMPROJ\\0 magic",
        ));
    }
    let mut r = Reader { bytes, offset: 8 };
    let version = r.u16()?;
    if !matches!(version, 1 | 2) {
        return Err(ProjectionError::new(
            "binary_version",
            "Unsupported CEM projection version",
        ));
    }
    if r.u8()? != kind.tag() || r.string()? != kind.schema() || r.string()? != kind.binary_mime() {
        return Err(ProjectionError::new(
            "projection_mismatch",
            "Binary projection kind/schema/MIME mismatch",
        ));
    }
    let count = r.u32()? as usize;
    if count > LIMIT || count > bytes.len().saturating_sub(r.offset) / 10 {
        return Err(ProjectionError::new(
            "source_limit",
            "Invalid bounded projection record count",
        ));
    }
    let mut doc = if kind == ProjectionKind::Events {
        ImportBuilder::new().ast
    } else {
        CemDocument::default()
    };
    let mut records = vec![(0, r.offset)];
    for index in 0..count {
        let start = r.offset;
        if kind == ProjectionKind::Events {
            event(&mut r, &mut doc, index, uri, kind, version)?;
            records.push((start, r.offset - start));
            continue;
        }
        let start = r.offset;
        let tag = r.u8()?;
        let id = r.u32()?;
        if id as usize != index {
            return Err(r.error("Node IDs must match their arena index"));
        }
        r.range()?;
        let empty = SourceMapStack::default();
        let node = match tag {
            1 => CemAstNode::Document {
                node_id: id,
                root_children: r.ids(count)?,
                source: empty,
            },
            2 => CemAstNode::Element {
                node_id: id,
                expanded_name: r.name()?,
                has_explicit_boundary: r.boolean()?,
                attributes: r.ids(count)?,
                children: r.ids(count)?,
                source: empty,
            },
            3 => CemAstNode::Attribute {
                node_id: id,
                expanded_name: r.name()?,
                value: r.optional()?,
                value_nodes: if version >= 2 { r.ids(count)? } else { vec![] },
                source: empty,
            },
            4..=6 | 8..=10 => {
                let data = r.string()?;
                match tag {
                    4 => CemAstNode::Text {
                        node_id: id,
                        data,
                        source: empty,
                    },
                    5 => CemAstNode::Whitespace {
                        node_id: id,
                        data,
                        source: empty,
                    },
                    6 => CemAstNode::Comment {
                        node_id: id,
                        data,
                        source: empty,
                    },
                    8 => CemAstNode::Cdata {
                        node_id: id,
                        data,
                        source: empty,
                    },
                    9 => CemAstNode::RawText {
                        node_id: id,
                        data,
                        source: empty,
                    },
                    _ => CemAstNode::Error {
                        node_id: id,
                        code: data,
                        source: empty,
                    },
                }
            }
            7 => CemAstNode::ProcessingInstruction {
                node_id: id,
                target: r.string()?,
                data: r.string()?,
                source: empty,
            },
            11 if version >= 2 => {
                let expression = r.string()?;
                let context = r.u32()?;
                if context as usize >= count {
                    return Err(r.error("Invalid reference context"));
                }
                if r.boolean()? {
                    r.ids(count)?;
                }
                CemAstNode::Reference {
                    node_id: id,
                    expression,
                    context: 0,
                    targets: None,
                    source: empty,
                }
            }
            _ => return Err(r.error("Unknown node tag for projection version")),
        };
        records.push((start, r.offset - start));
        doc.nodes.push(node);
        set_source(
            doc.nodes.last_mut().unwrap(),
            source(uri, start, r.offset - start),
        );
    }
    if r.offset != bytes.len() {
        return Err(r.error("Trailing projection bytes"));
    }
    if kind == ProjectionKind::Events {
        let end = r.offset;
        set_source(&mut doc.nodes[0], source(uri, 0, end));
    }
    check_tree(&doc)?;
    Ok((doc, records))
}
fn event(
    r: &mut Reader<'_>,
    doc: &mut CemDocument,
    index: usize,
    uri: &str,
    kind: ProjectionKind,
    version: u16,
) -> Result<()> {
    let start = r.offset;
    if r.u64()? != index as u64 {
        return Err(r.error("Event sequence must be contiguous"));
    }
    let tag = r.tag(9)?;
    let reported = r.range()?;
    let mut fields = vec![
        ("sequence", index.to_string()),
        (
            "kind",
            [
                "open",
                "close",
                "name",
                "value",
                "trivia",
                "processing-instruction",
                "separator",
                "mode-switch",
                "error",
            ][(tag - 1) as usize]
                .into(),
        ),
    ];
    if let Some(range) = reported {
        fields.extend([
            ("reported-offset", range.start.to_string()),
            ("reported-length", range.len.to_string()),
        ]);
    }
    match tag {
        1..=3 => {
            fields.push(("name", r.qname()?));
            if tag == 2 {
                fields.push(("synthesis", r.tag(6)?.to_string()));
            }
        }
        4 => {
            let value_tag = r.tag(7)?;
            if version < 2 && value_tag == 7 {
                return Err(r.error("Typed prelude requires projection version 2"));
            }
            fields.push((
                "value-kind",
                [
                    "text",
                    "integer",
                    "float",
                    "boolean",
                    "null",
                    "expression",
                    "typed-prelude",
                ][(value_tag - 1) as usize]
                    .into(),
            ));
            let value = match value_tag {
                1 | 6 => r.string()?,
                2 => i64::from_be_bytes(r.take(8)?.try_into().unwrap()).to_string(),
                3 => {
                    let v = f64::from_bits(r.u64()?);
                    if !v.is_finite() {
                        return Err(r.error("Non-finite event scalar"));
                    }
                    v.to_string()
                }
                4 => r.boolean()?.to_string(),
                5 => "null".into(),
                7 => {
                    fields.push((
                        "prelude-role",
                        ["schema-selector", "namespace", "default-namespace"]
                            [(r.tag(3)? - 1) as usize]
                            .into(),
                    ));
                    if let Some(prefix) = r.optional()? {
                        fields.push(("prefix", prefix));
                    }
                    for (offset, length, optional) in [
                        ("prelude-role-offset", "prelude-role-length", false),
                        ("prelude-prefix-offset", "prelude-prefix-length", true),
                        ("prelude-value-offset", "prelude-value-length", false),
                        ("prelude-payload-offset", "prelude-payload-length", false),
                    ] {
                        if let Some(range) = r.range()? {
                            fields.extend([
                                (offset, range.start.to_string()),
                                (length, range.len.to_string()),
                            ]);
                        } else if !optional {
                            return Err(r.error("Required prelude range"));
                        }
                    }
                    fields.push((
                        "prelude-kind",
                        ["reference", "expression"][(r.tag(2)? - 1) as usize].into(),
                    ));
                    let expression = r.string()?;
                    if let Some(error) = r.optional()? {
                        fields.push(("error", error));
                    }
                    expression
                }
                _ => unreachable!(),
            };
            fields.push(("value", value));
        }
        5 => {
            fields.push((
                "trivia",
                if r.tag(2)? == 1 {
                    "whitespace"
                } else {
                    "comment"
                }
                .into(),
            ));
            fields.push(("data", r.string()?));
        }
        6 => {
            fields.push(("target", r.string()?));
            fields.push(("data", r.string()?));
        }
        7 => fields.push(("separator", r.tag(5)?.to_string())),
        8 => fields.push(("content-type", r.string()?)),
        9 => {
            fields.push(("code", r.string()?));
            fields.push(("severity", r.tag(4)?.to_string()));
        }
        _ => unreachable!(),
    }
    push_event(doc, kind, uri, start, r.offset - start, fields)
}
pub(super) fn push_event(
    doc: &mut CemDocument,
    kind: ProjectionKind,
    uri: &str,
    start: usize,
    length: usize,
    fields: Vec<(&str, String)>,
) -> Result<()> {
    if doc.nodes.len().saturating_add(fields.len() + 1) > LIMIT {
        return Err(ProjectionError::new(
            "source_limit",
            "Event projection materialization exceeds the node limit",
        ));
    }
    let id = doc.nodes.len() as u32;
    let map = source(uri, start, length);
    doc.nodes.push(CemAstNode::Element {
        node_id: id,
        expanded_name: name(kind.schema().into(), "event".into()),
        attributes: vec![],
        children: vec![],
        has_explicit_boundary: true,
        source: map.clone(),
    });
    if let CemAstNode::Document { root_children, .. } = &mut doc.nodes[0] {
        root_children.push(id)
    }
    for (key, value) in fields {
        let child = doc.nodes.len() as u32;
        doc.nodes.push(CemAstNode::Attribute {
            node_id: child,
            expanded_name: name("".into(), key.into()),
            value: Some(value),
            value_nodes: vec![],
            source: map.clone(),
        });
        if let CemAstNode::Element { attributes, .. } = &mut doc.nodes[id as usize] {
            attributes.push(child)
        }
    }
    Ok(())
}
