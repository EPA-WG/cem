//! Opt-in Markdown metadata is decoded once at the native import boundary.
use super::*;
use crate::validation::{
    generic_data::*,
    yaml::{yaml_document_ast_from_source_bytes, YamlSourceValidationRequest},
};
use std::{collections::BTreeSet, sync::Arc};

pub(super) struct FrontMatter {
    pub body_offset: usize,
    pub metadata: Option<Arc<GenericDataDocumentAst>>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(super) fn read(source: &str, uri: &str, option: Option<&String>) -> FrontMatter {
    let mut result = FrontMatter {
        body_offset: 0,
        metadata: None,
        diagnostics: vec![],
    };
    let Some(option) = option else {
        return result;
    };
    let error = |code: &str, message: String, offset: usize| {
        let range =
            MarkdownSourceRange::from_offsets(&LineIndex::from_utf8(source), offset, offset);
        Diagnostic {
            uri: Some(uri.into()),
            code: format!("cem.markdown.front_matter_{code}"),
            severity: Severity::Error,
            message,
            line: Some(range.start.line),
            column: Some(range.start.column),
            byte_offset: Some(offset as u64),
            source_map: Some(range.source_map().with_source_uri(uri)),
            ..Default::default()
        }
    };
    if option != "yaml" {
        result.diagnostics.push(error(
            "unsupported",
            "Supported front-matter parameter: yaml.".into(),
            0,
        ));
        return result;
    }
    let mut lines = source.split_inclusive('\n');
    let Some(first) = lines
        .next()
        .filter(|line| line.trim_end_matches(['\r', '\n']) == "---")
    else {
        return result;
    };
    let start = first.len();
    let mut end = start;
    let mut closed = false;
    for line in lines {
        if end.saturating_sub(start) > 64 * 1024 {
            break;
        }
        if line.trim_end_matches(['\r', '\n']) == "---" {
            result.body_offset = end + line.len();
            closed = true;
            break;
        }
        end += line.len();
    }
    if end.saturating_sub(start) > 64 * 1024 {
        result.diagnostics.push(error(
            "limit",
            "YAML front matter exceeds 64 KiB.".into(),
            start,
        ));
        return result;
    }
    if !closed {
        result.diagnostics.push(error(
            "unclosed",
            "YAML front matter requires a closing --- line.".into(),
            start,
        ));
        return result;
    }
    // Bound the registered YAML scanner before parser/tree construction.
    if let Err((code, message)) = scan_bounds(&source[start..end]) {
        result.diagnostics.push(error(code, message, start));
        return result;
    }
    let (parsed, diagnostics) = yaml_document_ast_from_source_bytes(YamlSourceValidationRequest {
        bytes: &source.as_bytes()[start..end],
        source_uri: uri,
        content_type: Some("application/yaml"),
    });
    if let Some(failure) = diagnostics
        .iter()
        .find(|item| item.severity.is_hard_violation())
    {
        result.diagnostics.push(error(
            "invalid",
            failure.message.clone(),
            start + failure.byte_offset.unwrap_or(0) as usize,
        ));
        return result;
    }
    let Some(parsed) = parsed else {
        return result;
    };
    let mut data = parsed.to_generic_data_ast();
    if let Err(message) = validate(&data) {
        result.diagnostics.push(error("invalid", message, start));
        return result;
    }
    let index = LineIndex::from_utf8(source);
    for document in &mut data.documents {
        rebase_range(&mut document.source_range, start, &index, uri);
        if let Some(root) = &mut document.root {
            rebase_value(root, start, &index, uri);
        }
    }
    // Coordinates and identity belong to the containing Markdown resource.
    data.source.byte_length = source.len();
    data.source.uri = uri.into();
    result.metadata = Some(Arc::new(data));
    result
}

fn scan_bounds(source: &str) -> Result<(), (&'static str, String)> {
    use yaml_rust2::scanner::{Scanner, TokenType};
    let mut depth = 0usize;
    for (count, token) in Scanner::new(source.chars()).enumerate() {
        if count >= 16384 {
            return Err(("limit", "Front matter exceeds 16,384 YAML tokens.".into()));
        }
        match token.1 {
            TokenType::BlockSequenceStart | TokenType::BlockMappingStart | TokenType::FlowSequenceStart | TokenType::FlowMappingStart => {
                depth += 1;
                if depth > 64 { return Err(("limit", "Front matter exceeds 64 levels.".into())); }
            }
            TokenType::BlockEnd | TokenType::FlowSequenceEnd | TokenType::FlowMappingEnd => depth = depth.saturating_sub(1),
            TokenType::Alias(_) | TokenType::Anchor(_) | TokenType::Tag(..) | TokenType::TagDirective(..) => return Err(("invalid", "Front matter supports literal YAML values without anchors, aliases or explicit tags.".into())),
            _ => {}
        }
    }
    Ok(())
}

fn validate(data: &GenericDataDocumentAst) -> Result<(), String> {
    if data.documents.len() != 1 {
        return Err("Front matter requires one YAML mapping document.".into());
    }
    let Some(GenericDataValueAst::Mapping { entries, .. }) = data.documents[0].root.as_ref() else {
        return Err("Front matter requires a YAML mapping.".into());
    };
    let mut pending = vec![(data.documents[0].root.as_ref().unwrap(), 0usize)];
    let mut count = 0usize;
    while let Some((value, depth)) = pending.pop() {
        count += 1;
        if depth > 64 || count > 4096 {
            return Err("Front matter exceeds 64 levels or 4,096 values.".into());
        }
        match value {
            GenericDataValueAst::Mapping { entries, .. } => {
                let mut keys = BTreeSet::new();
                for entry in entries {
                    let GenericDataValueAst::String { value: key, .. } = &entry.key else {
                        return Err("Metadata keys must be strings.".into());
                    };
                    if !keys.insert(key) {
                        return Err(format!("Duplicate metadata key `{key}`."));
                    }
                    pending.push((&entry.key, depth + 1));
                    pending.push((&entry.value, depth + 1));
                }
            }
            GenericDataValueAst::Sequence { items, .. } => {
                pending.extend(items.iter().map(|item| (item, depth + 1)))
            }
            GenericDataValueAst::Alias { .. } => {
                return Err("Metadata aliases are unsupported.".into())
            }
            _ => {}
        }
    }
    for entry in entries {
        let GenericDataValueAst::String { value: key, .. } = &entry.key else {
            unreachable!()
        };
        let valid = match key.as_str() {
            "title" | "description" | "layout" | "slug" | "date" => {
                matches!(entry.value, GenericDataValueAst::String { .. })
            }
            "draft" => matches!(entry.value, GenericDataValueAst::Boolean { .. }),
            "weight" => matches!(
                entry.value,
                GenericDataValueAst::Number {
                    number_kind: GenericDataNumberKind::Integer,
                    ..
                }
            ),
            "tags" => {
                matches!(&entry.value, GenericDataValueAst::Sequence { items, .. } if items.iter().all(|item| matches!(item, GenericDataValueAst::String { .. })))
            }
            _ => true,
        };
        if !valid {
            return Err(format!("Invalid type for metadata `{key}`."));
        }
    }
    Ok(())
}

fn rebase_range(
    range: &mut GenericDataSourceRangeAst,
    offset: usize,
    index: &LineIndex,
    uri: &str,
) {
    range.byte_offset += offset as u64;
    let coordinate = index.project(range.byte_offset);
    range.line = coordinate.line;
    range.column = coordinate.column;
    if let Some(source) = &mut range.source_map {
        for frame in &mut source.frames {
            match &mut frame.span {
                FrameSpan::Single(range) => range.start += offset as u64,
                FrameSpan::Multi(ranges) => {
                    for range in ranges {
                        range.start += offset as u64;
                    }
                }
            }
        }
        *source = source.clone().with_source_uri(uri);
    }
}

fn rebase_value(value: &mut GenericDataValueAst, offset: usize, index: &LineIndex, uri: &str) {
    let range = match value {
        GenericDataValueAst::Mapping {
            source_range,
            entries,
        } => {
            for entry in entries {
                rebase_range(&mut entry.source_range, offset, index, uri);
                rebase_value(&mut entry.key, offset, index, uri);
                rebase_value(&mut entry.value, offset, index, uri);
            }
            source_range
        }
        GenericDataValueAst::Sequence {
            source_range,
            items,
        } => {
            for item in items {
                rebase_value(item, offset, index, uri);
            }
            source_range
        }
        GenericDataValueAst::String { source_range, .. }
        | GenericDataValueAst::Number { source_range, .. }
        | GenericDataValueAst::Boolean { source_range, .. }
        | GenericDataValueAst::Null { source_range }
        | GenericDataValueAst::Alias { source_range, .. } => source_range,
    };
    rebase_range(range, offset, index, uri);
}
