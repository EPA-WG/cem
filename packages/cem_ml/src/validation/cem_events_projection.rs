use crate::diagnostics::{Diagnostic, Severity};
use crate::schema::registry::{
    content_type_essence, CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
    CEM_EVENTS_PROJECTION_CONTENT_TYPE, CEM_EVENTS_PROJECTION_SCHEMA_URI,
};

#[derive(Debug, Clone, Copy)]
pub struct CemEventsProjectionSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CemEventsProjectionSourceKind {
    Binary,
    Json,
}

pub fn validate_cem_events_projection_source_bytes(
    request: CemEventsProjectionSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    match cem_events_projection_source_kind(&request) {
        CemEventsProjectionSourceKind::Binary => validate_cem_events_projection_binary(&request),
        CemEventsProjectionSourceKind::Json => validate_cem_events_projection_json(&request),
    }
}

fn cem_events_projection_source_kind(
    request: &CemEventsProjectionSourceValidationRequest<'_>,
) -> CemEventsProjectionSourceKind {
    let content_type = request.content_type.map(content_type_essence);
    match content_type.as_deref() {
        Some(CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE) => CemEventsProjectionSourceKind::Json,
        Some(CEM_EVENTS_PROJECTION_CONTENT_TYPE) => CemEventsProjectionSourceKind::Binary,
        _ if request.bytes.starts_with(b"CEMPROJ\0") => CemEventsProjectionSourceKind::Binary,
        _ => CemEventsProjectionSourceKind::Json,
    }
}

fn validate_cem_events_projection_binary(
    request: &CemEventsProjectionSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    match validate_cem_events_projection_binary_bytes(request.bytes) {
        Ok(()) => Vec::new(),
        Err((code, message)) => vec![cem_events_projection_diagnostic(request, code, message)],
    }
}

fn validate_cem_events_projection_binary_bytes(bytes: &[u8]) -> Result<(), (&'static str, String)> {
    if !bytes.starts_with(b"CEMPROJ\0") {
        return Err((
            "cem.projection.events.binary_magic",
            "CEM events binary projection must start with CEMPROJ\\0 magic".to_owned(),
        ));
    }

    let mut reader = ProjectionBinaryReader::new(&bytes[b"CEMPROJ\0".len()..]);
    let version = reader.read_u16("version")?;
    if version != 1 {
        return Err((
            "cem.projection.events.binary_version",
            format!("unsupported CEM projection binary version `{version}`; expected `1`"),
        ));
    }

    let projection_kind = reader.read_u8("projection kind")?;
    if projection_kind != 3 {
        return Err((
            "cem.projection.events.projection_mismatch",
            format!("binary projection kind `{projection_kind}` is not CEM events kind `3`"),
        ));
    }

    let schema = reader.read_str("schema")?;
    if schema != CEM_EVENTS_PROJECTION_SCHEMA_URI {
        return Err((
            "cem.projection.events.projection_mismatch",
            format!(
                "binary projection schema `{schema}` is not `{CEM_EVENTS_PROJECTION_SCHEMA_URI}`"
            ),
        ));
    }

    let content_type = reader.read_str("content type")?;
    if content_type != CEM_EVENTS_PROJECTION_CONTENT_TYPE {
        return Err((
            "cem.projection.events.projection_mismatch",
            format!(
                "binary projection content type `{content_type}` is not `{CEM_EVENTS_PROJECTION_CONTENT_TYPE}`"
            ),
        ));
    }

    let _event_count = reader.read_u32("event count")?;
    Ok(())
}

struct ProjectionBinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ProjectionBinaryReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_u8(&mut self, field: &'static str) -> Result<u8, (&'static str, String)> {
        let bytes = self.read_exact(field, 1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self, field: &'static str) -> Result<u16, (&'static str, String)> {
        let bytes = self.read_exact(field, 2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self, field: &'static str) -> Result<u32, (&'static str, String)> {
        let bytes = self.read_exact(field, 4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_str(&mut self, field: &'static str) -> Result<String, (&'static str, String)> {
        let len = self.read_u32(field)? as usize;
        let bytes = self.read_exact(field, len)?;
        std::str::from_utf8(bytes)
            .map(str::to_owned)
            .map_err(|error| {
                (
                    "cem.projection.events.binary_truncated",
                    format!("CEM events binary projection {field} is not UTF-8: {error}"),
                )
            })
    }

    fn read_exact(
        &mut self,
        field: &'static str,
        len: usize,
    ) -> Result<&'a [u8], (&'static str, String)> {
        let end = self.offset.checked_add(len).ok_or_else(|| {
            (
                "cem.projection.events.binary_truncated",
                format!("CEM events binary projection {field} length overflows input"),
            )
        })?;
        if end > self.bytes.len() {
            return Err((
                "cem.projection.events.binary_truncated",
                format!("CEM events binary projection is truncated while reading {field}"),
            ));
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}

fn validate_cem_events_projection_json(
    request: &CemEventsProjectionSourceValidationRequest<'_>,
) -> Vec<Diagnostic> {
    let value = match serde_json::from_slice::<serde_json::Value>(request.bytes) {
        Ok(value) => value,
        Err(error) => {
            return vec![cem_events_projection_diagnostic(
                request,
                "cem.projection.events.json_parse_error",
                format!("CEM events JSON projection parse error: {error}"),
            )];
        }
    };

    match validate_cem_events_projection_json_value(&value) {
        Ok(()) => Vec::new(),
        Err(message) => vec![cem_events_projection_diagnostic(
            request,
            "cem.projection.events.json_shape",
            message,
        )],
    }
}

fn validate_cem_events_projection_json_value(value: &serde_json::Value) -> Result<(), String> {
    if let Some(object) = value.as_object() {
        if object.get("kind").and_then(serde_json::Value::as_str) == Some("cem-binary-projection") {
            return validate_cem_binary_projection_json(
                object,
                "events",
                CEM_EVENTS_PROJECTION_SCHEMA_URI,
                CEM_EVENTS_PROJECTION_CONTENT_TYPE,
            );
        }
    }

    let events = value
        .as_array()
        .ok_or_else(|| "CEM events JSON projection root must be an event array".to_owned())?;
    for (index, event) in events.iter().enumerate() {
        validate_cem_event_json(event, &format!("$[{index}]"))?;
    }
    Ok(())
}

fn validate_cem_event_json(value: &serde_json::Value, path: &str) -> Result<(), String> {
    let object = json_object(value, path)?;
    let Some(kind) = object.get("kind").and_then(serde_json::Value::as_str) else {
        return Err(format!("{path}.kind must be a string"));
    };

    match kind {
        "open" | "close" | "name" => {
            expect_json_string_field(object, "name", path, None)?;
            validate_required_byte_range(object, path)
        }
        "value" => {
            let value = object
                .get("value")
                .ok_or_else(|| format!("{path}.value is required"))?;
            if value.is_array() || value.is_object() {
                return Err(format!(
                    "{path}.value must be a string, number, boolean, or null"
                ));
            }
            validate_required_byte_range(object, path)
        }
        "trivia" => {
            let trivia = expect_json_string_field(object, "trivia", path, None)?;
            if !matches!(trivia, "whitespace" | "comment") {
                return Err(format!(
                    "{path}.trivia `{trivia}` is not a supported trivia kind"
                ));
            }
            expect_json_string_field(object, "data", path, None)?;
            validate_required_byte_range(object, path)
        }
        "processing-instruction" => {
            expect_json_string_field(object, "target", path, None)?;
            expect_json_string_field(object, "data", path, None)?;
            validate_required_byte_range(object, path)
        }
        "separator" => validate_required_byte_range(object, path),
        "mode-switch" => {
            expect_json_string_field(object, "contentType", path, None)?;
            Ok(())
        }
        "error" => {
            expect_json_string_field(object, "code", path, None)?;
            validate_required_byte_range(object, path)
        }
        _ => Err(format!(
            "{path}.kind `{kind}` is not a supported CEM events kind"
        )),
    }
}

fn validate_cem_binary_projection_json(
    object: &serde_json::Map<String, serde_json::Value>,
    projection: &str,
    schema_uri: &str,
    content_type: &str,
) -> Result<(), String> {
    expect_json_string_field(object, "projection", "$", Some(projection))?;
    expect_json_string_field(object, "schema", "$", Some(schema_uri))?;
    expect_json_string_field(object, "contentType", "$", Some(content_type))?;
    expect_json_string_field(object, "formatVersion", "$", Some("cem-projection-bin/1"))?;
    expect_json_string_field(object, "hashScheme", "$", None)?;
    expect_json_string_field(object, "hash", "$", None)?;
    expect_json_u64_field(object, "byteLength", "$")?;
    if let Some(native_bytes) = object.get("nativeBytes") {
        if !native_bytes.is_boolean() {
            return Err("$.nativeBytes must be a boolean".to_owned());
        }
    }
    if let Some(chunks) = object.get("chunks") {
        validate_cem_events_projection_json_chunks(chunks)?;
    }
    Ok(())
}

fn validate_cem_events_projection_json_chunks(chunks: &serde_json::Value) -> Result<(), String> {
    let chunks = chunks
        .as_array()
        .ok_or_else(|| "$.chunks must be an array".to_owned())?;
    let mut expected_offset = 0_u64;
    for (index, chunk) in chunks.iter().enumerate() {
        let path = format!("$.chunks[{index}]");
        let object = json_object(chunk, &path)?;
        expect_json_string_field(object, "id", &path, None)?;
        if object.get("sealed").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err(format!("{path}.sealed must be true"));
        }
        let offset = expect_json_u64_field(object, "byteOffset", &path)?;
        if offset != expected_offset {
            return Err(format!(
                "{path}.byteOffset must be contiguous at {expected_offset}"
            ));
        }
        let byte_length = expect_json_u64_field(object, "byteLength", &path)?;
        expect_json_string_field(object, "hash", &path, None)?;
        expect_json_string_field(object, "dataEncoding", &path, Some("hex"))?;
        let data = expect_json_string_field(object, "data", &path, None)?;
        if data.len() % 2 != 0 || !data.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!("{path}.data must be even-length hex"));
        }
        if (data.len() / 2) as u64 != byte_length {
            return Err(format!(
                "{path}.byteLength does not match decoded hex data length"
            ));
        }
        expected_offset += byte_length;
    }
    Ok(())
}

fn validate_required_byte_range(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), String> {
    if !object.contains_key("byteRange") {
        return Err(format!("{path}.byteRange is required"));
    }
    validate_optional_byte_range(object, path)
}

fn validate_optional_byte_range(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), String> {
    let Some(byte_range) = object.get("byteRange") else {
        return Ok(());
    };
    if byte_range.is_null() {
        return Ok(());
    }
    let range_path = format!("{path}.byteRange");
    let range = json_object(byte_range, &range_path)?;
    expect_json_u64_field(range, "start", &range_path)?;
    expect_json_u64_field(range, "len", &range_path)?;
    Ok(())
}

fn json_object<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))
}

fn expect_json_string_field<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
    path: &str,
    expected: Option<&str>,
) -> Result<&'a str, String> {
    let value = object
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{path}.{field} must be a string"))?;
    if let Some(expected) = expected {
        if value != expected {
            return Err(format!(
                "{path}.{field} must be `{expected}`, got `{value}`"
            ));
        }
    }
    Ok(value)
}

fn expect_json_u64_field(
    object: &serde_json::Map<String, serde_json::Value>,
    field: &str,
    path: &str,
) -> Result<u64, String> {
    object
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("{path}.{field} must be a non-negative integer"))
}

fn cem_events_projection_diagnostic(
    request: &CemEventsProjectionSourceValidationRequest<'_>,
    code: &'static str,
    message: String,
) -> Diagnostic {
    Diagnostic {
        uri: Some(request.source_uri.to_owned()),
        code: code.to_owned(),
        severity: Severity::Error,
        message,
        ..Diagnostic::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate(bytes: &[u8], content_type: &str) -> Vec<Diagnostic> {
        validate_cem_events_projection_source_bytes(CemEventsProjectionSourceValidationRequest {
            bytes,
            source_uri: "fixture.events",
            content_type: Some(content_type),
        })
    }

    fn has_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn cem_events_projection_validator_accepts_binary_fixture() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-events-projection/v1/examples/basic-events.cem-bin"
            ),
            CEM_EVENTS_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_events_projection_validator_reports_binary_magic() {
        let diagnostics = validate(b"not-a-cem-projection", CEM_EVENTS_PROJECTION_CONTENT_TYPE);

        assert!(has_code(&diagnostics, "cem.projection.events.binary_magic"));
    }

    #[test]
    fn cem_events_projection_validator_accepts_basic_json() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-events-projection/v1/examples/basic-events.events.json"
            ),
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_events_projection_validator_accepts_nested_json() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-events-projection/v1/examples/nested-events.events.json"
            ),
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_events_projection_validator_reports_invalid_kind_json_shape() {
        let diagnostics = validate(
            include_bytes!(
                "../../schema-packages/cem-events-projection/v1/examples/invalid-kind.events.json"
            ),
            CEM_EVENTS_JSON_PROJECTION_CONTENT_TYPE,
        );

        assert!(has_code(&diagnostics, "cem.projection.events.json_shape"));
    }
}
