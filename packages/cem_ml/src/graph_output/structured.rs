//! Shared writers for explicit structured output/control publication, not AST handoff.
use crate::engine as eng;
use crate::schema::registry::{content_type_essence, XML_SCHEMA_URI, YAML_SCHEMA_URI};
use std::io;

pub fn structured_document_bytes(
    value: &serde_json::Value,
    identity: Option<&eng::FormatIdentity>,
) -> io::Result<Vec<u8>> {
    match structured_content_type(identity) {
        "application/yaml" => serialize_json_value_as_yaml(value),
        "application/xml" => serialize_json_value_as_xml(value),
        _ => serde_json::to_vec_pretty(value)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
    }
}

pub fn structured_content_type(identity: Option<&eng::FormatIdentity>) -> &'static str {
    let mime = identity
        .and_then(|identity| identity.content_type.as_deref())
        .map(content_type_essence);
    let schema = identity
        .and_then(|identity| identity.schema.as_deref())
        .map(str::trim);
    let yaml = match mime.as_deref() {
        Some("application/yaml" | "application/x-yaml" | "text/yaml" | "text/x-yaml") => true,
        Some("application/json" | "text/json") => false,
        _ => schema == Some(YAML_SCHEMA_URI),
    };
    if yaml {
        "application/yaml"
    } else if matches!(mime.as_deref(), Some("application/xml" | "text/xml"))
        || schema == Some(XML_SCHEMA_URI)
    {
        "application/xml"
    } else {
        "application/json"
    }
}

fn serialize_json_value_as_yaml(value: &serde_json::Value) -> io::Result<Vec<u8>> {
    let yaml = json_value_to_yaml(value);
    let mut content = String::new();
    let mut emitter = yaml_rust2::YamlEmitter::new(&mut content);
    emitter.multiline_strings(true);
    emitter
        .dump(&yaml)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok(content.into_bytes())
}

fn json_value_to_yaml(value: &serde_json::Value) -> yaml_rust2::Yaml {
    match value {
        serde_json::Value::Null => yaml_rust2::Yaml::Null,
        serde_json::Value::Bool(value) => yaml_rust2::Yaml::Boolean(*value),
        serde_json::Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                yaml_rust2::Yaml::Integer(value)
            } else {
                yaml_rust2::Yaml::Real(number.to_string())
            }
        }
        serde_json::Value::String(value) => yaml_rust2::Yaml::String(value.clone()),
        serde_json::Value::Array(values) => {
            yaml_rust2::Yaml::Array(values.iter().map(json_value_to_yaml).collect())
        }
        serde_json::Value::Object(fields) => {
            let mut out = yaml_rust2::yaml::Hash::new();
            for (key, value) in fields {
                out.insert(
                    yaml_rust2::Yaml::String(key.clone()),
                    json_value_to_yaml(value),
                );
            }
            yaml_rust2::Yaml::Hash(out)
        }
    }
}

fn serialize_json_value_as_xml(value: &serde_json::Value) -> io::Result<Vec<u8>> {
    let root = structured_xml_root_name(value);
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    write_structured_xml_value(&mut out, &root, value, 0);
    out.push('\n');
    Ok(out.into_bytes())
}

fn structured_xml_root_name(value: &serde_json::Value) -> String {
    value
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .map(xml_element_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "value".to_owned())
}

fn write_structured_xml_value(
    out: &mut String,
    name: &str,
    value: &serde_json::Value,
    depth: usize,
) {
    let name = xml_element_name(name);
    let indent = crate::transform_template::DEFAULT_FORMATTER_INDENT.repeat(depth);
    match value {
        serde_json::Value::Object(fields) => {
            out.push_str(&indent);
            out.push('<');
            out.push_str(&name);
            out.push_str(">\n");
            for (key, child) in fields {
                write_structured_xml_value(out, key, child, depth + 1);
            }
            out.push_str(&indent);
            out.push_str("</");
            out.push_str(&name);
            out.push_str(">\n");
        }
        serde_json::Value::Array(values) => {
            out.push_str(&indent);
            out.push('<');
            out.push_str(&name);
            out.push_str(">\n");
            for child in values {
                write_structured_xml_value(out, "item", child, depth + 1);
            }
            out.push_str(&indent);
            out.push_str("</");
            out.push_str(&name);
            out.push_str(">\n");
        }
        serde_json::Value::Null => {
            out.push_str(&indent);
            out.push('<');
            out.push_str(&name);
            out.push_str("/>\n");
        }
        serde_json::Value::Bool(value) => {
            write_structured_xml_text(out, &indent, &name, if *value { "true" } else { "false" });
        }
        serde_json::Value::Number(value) => {
            write_structured_xml_text(out, &indent, &name, &value.to_string());
        }
        serde_json::Value::String(value) => {
            write_structured_xml_text(out, &indent, &name, value);
        }
    }
}

fn write_structured_xml_text(out: &mut String, indent: &str, name: &str, value: &str) {
    out.push_str(indent);
    out.push('<');
    out.push_str(name);
    out.push('>');
    push_xml_escaped_text(out, value);
    out.push_str("</");
    out.push_str(name);
    out.push_str(">\n");
}

fn push_xml_escaped_text(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
}

fn xml_element_name(value: &str) -> String {
    let mut out = String::with_capacity(value.len().max(1));
    for (index, ch) in value.chars().enumerate() {
        let valid = ch == '_' || ch == '-' || ch.is_ascii_alphanumeric();
        if index == 0 && !(ch == '_' || ch.is_ascii_alphabetic()) {
            out.push('_');
        }
        out.push(if valid { ch } else { '-' });
    }
    if out.is_empty() {
        "_".to_owned()
    } else {
        out
    }
}
