//! Portable native CEM value graph. This binary import/export boundary preserves
//! references, ordered content and scalar contracts without a JSON document DOM.
use crate::{schema::document_model::AttributeValueContract, source_map::SourceMapStack};
use serde::{Deserialize, Serialize};

const MAGIC: &[u8] = b"CEMV\x01";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CemValueArtifactLimits {
    pub max_bytes: usize,
    pub max_values: usize,
    pub max_depth: usize,
}
impl Default for CemValueArtifactLimits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_values: 100_000,
            max_depth: 128,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CemValueProvenance {
    pub source_uri: Option<String>,
    pub source_key: Option<String>,
    pub line_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CemValueRecord {
    pub kind: String,
    pub name: String,
    pub namespace: String,
    pub lexical: String,
    pub datatype: String,
    pub parent: Option<u32>,
    pub children: Vec<u32>,
    pub attributes: Vec<u32>,
    pub values: Vec<u32>,
    pub targets: Vec<u32>,
    pub contract: Option<AttributeValueContract>,
    pub source: SourceMapStack,
    pub provenance: Option<CemValueProvenance>,
}
impl Default for CemValueRecord {
    fn default() -> Self {
        Self {
            kind: "atomic".into(),
            name: String::new(),
            namespace: String::new(),
            lexical: String::new(),
            datatype: "string".into(),
            parent: None,
            children: Vec::new(),
            attributes: Vec::new(),
            values: Vec::new(),
            targets: Vec::new(),
            contract: None,
            provenance: None,
            source: SourceMapStack::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CemValueGraph {
    pub roots: Vec<u32>,
    pub records: Vec<CemValueRecord>,
}

impl CemValueGraph {
    pub fn validate(&self, limits: &CemValueArtifactLimits) -> Result<(), String> {
        if self.records.len() > limits.max_values || self.roots.len() > limits.max_values {
            return Err("Native CEM value count limit exceeded".into());
        }
        if limits.max_bytes == 0 || limits.max_values == 0 || limits.max_depth == 0 {
            return Err("Native CEM artifact limits must be positive".into());
        }
        let size = self.records.len();
        let valid = |id: u32| (id as usize) < size;
        if self.roots.iter().any(|id| !valid(*id)) {
            return Err("Invalid native CEM root reference".into());
        }
        let mut bytes = 0usize;
        let mut edges = 0usize;
        for (index, record) in self.records.iter().enumerate() {
            if !matches!(
                record.kind.as_str(),
                "atomic"
                    | "document"
                    | "element"
                    | "attribute"
                    | "text"
                    | "whitespace"
                    | "cdata"
                    | "raw-text"
                    | "comment"
                    | "processing-instruction"
                    | "reference"
            ) {
                return Err("Unsupported native CEM value kind".into());
            }
            bytes = bytes.saturating_add(
                record.name.len()
                    + record.namespace.len()
                    + record.lexical.len()
                    + record.datatype.len(),
            );
            if bytes > limits.max_bytes {
                return Err("Native CEM value byte limit exceeded".into());
            }
            edges = edges
                .saturating_add(record.children.len())
                .saturating_add(record.attributes.len())
                .saturating_add(record.values.len())
                .saturating_add(record.targets.len());
            if edges > limits.max_values {
                return Err("Native CEM reference count limit exceeded".into());
            }
            if record.kind == "atomic" {
                use crate::schema::document_model::{convert_attribute_value, AttributeModel};
                match record.datatype.as_str() {
                    "string" | "anyURI" | "null" => {}
                    "double" if record.lexical.parse::<f64>().is_ok() => {}
                    _ => {
                        let contract = AttributeValueContract {
                            model: AttributeModel {
                                value_type: Some(record.datatype.clone()),
                                ..Default::default()
                            },
                            ..Default::default()
                        };
                        convert_attribute_value(&record.lexical, &contract, &record.source)
                            .map_err(|_| "Invalid native CEM atomic lexical value")?;
                    }
                }
            }
            if record
                .children
                .iter()
                .chain(&record.attributes)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != record.children.len() + record.attributes.len()
            {
                return Err("Duplicate owned CEM node occurrence".into());
            }
            if record.parent.is_some_and(|p| !valid(p)) {
                return Err("Invalid native CEM parent reference".into());
            }
            for id in record
                .children
                .iter()
                .chain(&record.attributes)
                .chain(&record.values)
                .chain(&record.targets)
            {
                if !valid(*id) {
                    return Err("Invalid native CEM value reference".into());
                }
            }
            if !matches!(record.kind.as_str(), "element" | "document")
                && (!record.children.is_empty() || !record.attributes.is_empty())
            {
                return Err("Only CEM elements/documents own structural children".into());
            }
            if record.kind != "element" && !record.attributes.is_empty() {
                return Err("Only elements own attributes".into());
            }
            if record.kind != "attribute"
                && (!record.values.is_empty() || record.contract.is_some())
            {
                return Err("Only attributes carry value contracts".into());
            }
            if record.kind != "reference" && !record.targets.is_empty() {
                return Err("Only references carry targets".into());
            }
            for &child in &record.children {
                let child = &self.records[child as usize];
                if child.parent != Some(index as u32)
                    || matches!(child.kind.as_str(), "attribute" | "atomic")
                {
                    return Err("Invalid CEM child ownership".into());
                }
            }
            for &attribute in &record.attributes {
                let attribute = &self.records[attribute as usize];
                if attribute.parent != Some(index as u32) || attribute.kind != "attribute" {
                    return Err("Invalid CEM attribute ownership".into());
                }
            }
            if let Some(parent) = record.parent {
                let parent = &self.records[parent as usize];
                if !parent.children.contains(&(index as u32))
                    && !parent.attributes.contains(&(index as u32))
                {
                    return Err("Inconsistent CEM parent backlink".into());
                }
            }
        }
        // Parent backlinks are deliberately excluded: owned content and value
        // references form a DAG, while a parent and its child naturally refer back.
        let mut state = vec![0u8; size];
        let mut heights = vec![0usize; size];
        let mut expanded = vec![0usize; size];
        for start in 0..size {
            if state[start] == 2 {
                continue;
            }
            let mut pending = vec![(start, false, 0usize)];
            while let Some((id, leave, depth)) = pending.pop() {
                if leave {
                    let record = &self.records[id];
                    let mut height = 0;
                    let mut work = 1usize;
                    for &child in record
                        .children
                        .iter()
                        .chain(&record.attributes)
                        .chain(&record.values)
                        .chain(&record.targets)
                    {
                        height = height.max(heights[child as usize].saturating_add(1));
                        work = work.saturating_add(expanded[child as usize]);
                    }
                    if height > limits.max_depth {
                        return Err("Native CEM value depth limit exceeded".into());
                    }
                    if work > limits.max_values {
                        return Err("Native CEM expanded value limit exceeded".into());
                    }
                    heights[id] = height;
                    expanded[id] = work;
                    state[id] = 2;
                    continue;
                }
                if state[id] == 1 {
                    return Err("Cyclic native CEM value graph".into());
                }
                if state[id] == 2 {
                    continue;
                }
                if depth > limits.max_depth {
                    return Err("Native CEM value depth limit exceeded".into());
                }
                state[id] = 1;
                pending.push((id, true, depth));
                let record = &self.records[id];
                for &child in record
                    .children
                    .iter()
                    .chain(&record.attributes)
                    .chain(&record.values)
                    .chain(&record.targets)
                    .rev()
                {
                    pending.push((child as usize, false, depth + 1));
                }
            }
        }
        for (id, record) in self.records.iter().enumerate() {
            let Some(contract) = &record.contract else {
                continue;
            };
            if matches!(
                contract.model.value_type.as_deref(),
                Some("node") | Some("any")
            ) {
                if contract.model.value_type.as_deref() == Some("node")
                    && record
                        .values
                        .iter()
                        .any(|&id| self.records[id as usize].kind == "atomic")
                {
                    return Err("Native node attribute contains an atomic value".into());
                }
                continue;
            }
            let lexical = self.string_value(id as u32, limits.max_bytes)?;
            crate::schema::document_model::convert_attribute_value(
                &lexical,
                contract,
                &record.source,
            )
            .map_err(|_| "Native CEM attribute violates its value contract")?;
        }
        if self
            .roots
            .iter()
            .fold(0usize, |sum, &id| sum.saturating_add(expanded[id as usize]))
            > limits.max_values
        {
            return Err("Native CEM expanded root value limit exceeded".into());
        }
        Ok(())
    }

    fn string_value(&self, root: u32, max_bytes: usize) -> Result<String, String> {
        let mut result = String::new();
        let mut pending = vec![(root, true)];
        while let Some((id, root)) = pending.pop() {
            let record = &self.records[id as usize];
            let value = match record.kind.as_str() {
                "element" | "document" => {
                    pending.extend(record.children.iter().rev().map(|&id| (id, false)));
                    ""
                }
                "reference" => {
                    pending.extend(record.targets.iter().rev().map(|&id| (id, root)));
                    ""
                }
                "attribute" if !record.values.is_empty() => {
                    pending.extend(record.values.iter().rev().map(|&id| (id, true)));
                    ""
                }
                "atomic" | "text" | "whitespace" | "cdata" | "raw-text" => record.lexical.as_str(),
                "attribute" | "comment" | "processing-instruction" if root => {
                    record.lexical.as_str()
                }
                _ => "",
            };
            if result.len().saturating_add(value.len()) > max_bytes {
                return Err("Native CEM attribute text limit exceeded".into());
            }
            result.push_str(value);
        }
        Ok(result)
    }

    pub fn encode(&self, limits: &CemValueArtifactLimits) -> Result<Vec<u8>, String> {
        self.validate(limits)?;
        let body = rmp_serde::to_vec_named(self).map_err(|e| e.to_string())?;
        if MAGIC.len() + body.len() + 32 > limits.max_bytes {
            return Err("Native CEM artifact byte limit exceeded".into());
        }
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(blake3::hash(&body).as_bytes());
        bytes.extend(body);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8], limits: &CemValueArtifactLimits) -> Result<Self, String> {
        if bytes.len() > limits.max_bytes {
            return Err("Native CEM artifact byte limit exceeded".into());
        }
        if bytes.len() < MAGIC.len() + 32 || !bytes.starts_with(MAGIC) {
            return Err("Unsupported native CEM value artifact version".into());
        }
        let (hash, body) = bytes[MAGIC.len()..].split_at(32);
        if blake3::hash(body).as_bytes() != hash {
            return Err("Native CEM value artifact integrity mismatch".into());
        }
        let mut cursor = std::io::Cursor::new(body);
        let mut decoder = rmp_serde::Deserializer::new(&mut cursor);
        // Graph edges are flat IDs. This cap bounds only the envelope/schema
        // metadata nesting, independently of the validated graph depth.
        decoder.set_max_depth(64);
        let graph = Self::deserialize(&mut decoder).map_err(|e| e.to_string())?;
        if cursor.position() as usize != body.len() {
            return Err("Trailing native CEM artifact data".into());
        }
        graph.validate(limits)?;
        Ok(graph)
    }
}
