//! Explicit JSON export of native CEM values. This is never an import path.
use super::artifact::{CemValueArtifactLimits, CemValueGraph, CemValueRecord};

/// Export one generic-data document/value or native scalar. References are
/// traversed under the same limits; no caller-owned node is changed.
pub fn write_json(
    graph: &CemValueGraph,
    limits: &CemValueArtifactLimits,
) -> Result<String, String> {
    graph.validate(limits)?;
    let mut writer = Writer {
        graph,
        limits,
        output: String::new(),
        visits: 0,
    };
    let roots = writer.content(&graph.roots, 0)?;
    let [root] = roots.as_slice() else {
        return Err("JSON export requires one value".into());
    };
    writer.value(*root, 0)?;
    Ok(writer.output)
}

struct Writer<'a> {
    graph: &'a CemValueGraph,
    limits: &'a CemValueArtifactLimits,
    output: String,
    visits: usize,
}
impl Writer<'_> {
    fn check(&mut self, depth: usize) -> Result<(), String> {
        // Graph values may be visited for structure, attributes and lexical text.
        // Bound reference expansion without counting these passes as extra values.
        self.visits += 1;
        if depth > self.limits.max_depth || self.visits > self.limits.max_values.saturating_mul(4) {
            return Err("Native JSON export depth/work limit exceeded".into());
        }
        Ok(())
    }
    fn record(&self, id: u32) -> &CemValueRecord {
        &self.graph.records[id as usize]
    }
    fn push(&mut self, text: &str) -> Result<(), String> {
        if self.output.len().saturating_add(text.len()) > self.limits.max_bytes {
            return Err("Native JSON export byte limit exceeded".into());
        }
        self.output.push_str(text);
        Ok(())
    }
    fn quoted(&mut self, text: &str) -> Result<(), String> {
        self.push("\"")?;
        for c in text.chars() {
            match c {
                '"' => self.push("\\\"")?,
                '\\' => self.push("\\\\")?,
                '\n' => self.push("\\n")?,
                '\r' => self.push("\\r")?,
                '\t' => self.push("\\t")?,
                '\u{0008}' => self.push("\\b")?,
                '\u{000c}' => self.push("\\f")?,
                c if c <= '\u{001f}' => self.push(&format!("\\u{:04x}", c as u32))?,
                c => self.push(c.encode_utf8(&mut [0; 4]))?,
            }
        }
        self.push("\"")
    }
    /// Structural whitespace/comments are not JSON values. String content uses
    /// `text`, which preserves whitespace instead.
    fn content(&mut self, ids: &[u32], depth: usize) -> Result<Vec<u32>, String> {
        let mut result = Vec::new();
        for &id in ids {
            self.check(depth)?;
            let record = self.record(id);
            match record.kind.as_str() {
                "reference" => result.extend(self.content(&record.targets.clone(), depth + 1)?),
                "document" => result.extend(self.content(&record.children.clone(), depth + 1)?),
                "comment" | "processing-instruction" => {}
                "text" | "whitespace" if record.lexical.trim().is_empty() => {}
                _ => result.push(id),
            }
        }
        Ok(result)
    }
    fn text(&mut self, ids: &[u32], depth: usize) -> Result<String, String> {
        let mut text = String::new();
        for &id in ids {
            self.check(depth)?;
            let record = self.record(id);
            let value = match record.kind.as_str() {
                "reference" => self.text(&record.targets.clone(), depth + 1)?,
                "attribute" if record.has_native_content() => {
                    self.text(&record.values.clone(), depth + 1)?
                }
                "attribute" | "text" | "whitespace" | "cdata" | "raw-text" | "atomic" => {
                    record.lexical.clone()
                }
                "comment" | "processing-instruction" => String::new(),
                _ => return Err("JSON scalar content cannot contain child elements".into()),
            };
            if text.len().saturating_add(value.len()) > self.limits.max_bytes {
                return Err("Native JSON scalar byte limit exceeded".into());
            }
            text.push_str(&value);
        }
        Ok(text)
    }
    fn attributes(
        &mut self,
        id: u32,
        property: bool,
        depth: usize,
    ) -> Result<Option<String>, String> {
        let mut key = None;
        for attr in self.record(id).attributes.clone() {
            self.check(depth)?;
            let record = self.record(attr);
            if record.namespace == "http://www.w3.org/2000/xmlns/"
                || (record.namespace.is_empty() && record.name == "xmlns")
            {
                continue;
            }
            if property && record.namespace.is_empty() && record.name == "name" && key.is_none() {
                key = Some(self.text(&[attr], depth + 1)?);
            } else {
                return Err("Unsupported attribute in JSON value tree".into());
            }
        }
        if property && key.is_none() {
            return Err("JSON property requires a name attribute".into());
        }
        Ok(key)
    }
    fn value(&mut self, id: u32, depth: usize) -> Result<(), String> {
        self.check(depth)?;
        let record = self.record(id);
        if record.kind == "atomic" {
            let datatype = record.datatype.clone();
            let lexical = record.lexical.clone();
            return match datatype.as_str() {
                "string" | "anyURI" => self.quoted(&lexical),
                "null" => self.push("null"),
                "boolean" if matches!(lexical.as_str(), "true" | "false") => self.push(&lexical),
                "integer" | "decimal" | "double"
                    if crate::transform_artifact::is_json_number_lexeme(&lexical) =>
                {
                    self.push(&lexical)
                }
                _ => Err("Native scalar is not representable in JSON".into()),
            };
        }
        if record.kind != "element" || record.namespace != "cem:generic-data" {
            return Err("JSON export requires generic-data CEM nodes".into());
        }
        let name = record.name.clone();
        let children = record.children.clone();
        self.attributes(id, false, depth)?;
        match name.as_str() {
            "object" => {
                self.push("{")?;
                for (index, property) in self.content(&children, depth + 1)?.into_iter().enumerate()
                {
                    let record = self.record(property);
                    if record.kind != "element"
                        || record.namespace != "cem:generic-data"
                        || record.name != "property"
                    {
                        return Err("JSON object requires property elements".into());
                    }
                    let children = record.children.clone();
                    let key = self
                        .attributes(property, true, depth + 1)?
                        .expect("required key");
                    let values = self.content(&children, depth + 1)?;
                    let [value] = values.as_slice() else {
                        return Err("JSON property requires one value".into());
                    };
                    if index > 0 {
                        self.push(",")?;
                    }
                    self.quoted(&key)?;
                    self.push(":")?;
                    self.value(*value, depth + 1)?;
                }
                self.push("}")
            }
            "array" => {
                self.push("[")?;
                for (index, value) in self.content(&children, depth + 1)?.into_iter().enumerate() {
                    if index > 0 {
                        self.push(",")?;
                    }
                    self.value(value, depth + 1)?;
                }
                self.push("]")
            }
            "string" => {
                let text = self.text(&children, depth + 1)?;
                self.quoted(&text)
            }
            "number" | "boolean" | "null" => {
                let text = self.text(&children, depth + 1)?;
                let text = text.trim();
                match name.as_str() {
                    "number" if crate::transform_artifact::is_json_number_lexeme(text) => {
                        self.push(text)
                    }
                    "boolean" if matches!(text, "true" | "false") => self.push(text),
                    "null" if text.is_empty() => self.push("null"),
                    _ => Err("Invalid JSON scalar lexical value".into()),
                }
            }
            _ => Err("Unsupported generic-data JSON value".into()),
        }
    }
}
