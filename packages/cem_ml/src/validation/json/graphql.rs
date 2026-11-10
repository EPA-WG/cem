//! Fixed operation guards at the native JSON boundary. GraphQL SDL and
//! executable-document parsing are build dependencies of the application.
use super::{
    extract_json_report_with_limits, JsonMemberAst, JsonParseFactKind, JsonSourceValidationRequest,
    JsonValueAst as Json,
};
use crate::{
    content_cache::ContentHash,
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAGIC: &[u8] = b"CEMGQL1\n";
const MAX_CONTRACT_BYTES: usize = 256 * 1024;
const MAX_JSON_BYTES: usize = 1024 * 1024;
const MAX_VALUES: usize = 16_384;
const MAX_DEPTH: usize = 48;
const INVALID: &str = "cem.graphql.contract.refused";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scalar {
    Boolean,
    Float,
    Id,
    Int,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shape {
    Scalar(Scalar),
    Enum(BTreeSet<String>),
    Object(BTreeMap<String, Field>),
    List(Box<Shape>),
    Nullable(Box<Shape>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub optional: bool,
    pub shape: Shape,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Query,
    Mutation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationContract {
    pub kind: OperationKind,
    pub variables: BTreeMap<String, Field>,
    pub data: BTreeMap<String, Field>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphqlContracts {
    schema_hash: String,
    document_hash: String,
    operations: BTreeMap<String, OperationContract>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseKind {
    Data,
    Partial,
    Errors,
}

impl GraphqlContracts {
    pub fn new(
        schema: &[u8],
        document: &[u8],
        operations: BTreeMap<String, OperationContract>,
    ) -> Self {
        Self {
            schema_hash: hash(schema),
            document_hash: hash(document),
            operations,
        }
    }

    pub fn schema_hash(&self) -> &str {
        &self.schema_hash
    }
    pub fn document_hash(&self) -> &str {
        &self.document_hash
    }
    pub fn operations(&self) -> &BTreeMap<String, OperationContract> {
        &self.operations
    }

    pub fn pinned_bytes(&self) -> Result<(Vec<u8>, String), String> {
        let body = rmp_serde::to_vec(self).map_err(|_| INVALID)?;
        if self.operations.is_empty()
            || self.operations.len() > 64
            || body.len() + MAGIC.len() > MAX_CONTRACT_BYTES
        {
            return Err(INVALID.into());
        }
        crate::native_binary::preflight_messagepack(&body, 32_768, 64, || Ok(()))?;
        let bytes = [MAGIC, &body].concat();
        let pin = hash(&bytes);
        Ok((bytes, pin))
    }

    /// The expected hash comes from the trusted application build.
    pub fn admit(bytes: &[u8], expected_hash: &str) -> Result<Self, String> {
        if bytes.len() > MAX_CONTRACT_BYTES || hash(bytes) != expected_hash {
            return Err(INVALID.into());
        }
        let body = bytes.strip_prefix(MAGIC).ok_or(INVALID)?;
        crate::native_binary::preflight_messagepack(body, 32_768, 64, || Ok(()))?;
        let mut reader = rmp_serde::Deserializer::from_read_ref(body);
        reader.set_max_depth(64);
        let contract = Self::deserialize(&mut reader).map_err(|_| INVALID)?;
        if contract.pinned_bytes()?.0 != bytes {
            return Err(INVALID.into());
        }
        Ok(contract)
    }

    pub fn validate_variables(
        &self,
        operation: &str,
        bytes: &[u8],
        uri: &str,
        control: &OperationControl,
    ) -> Result<(), String> {
        let operation = self.operations.get(operation).ok_or(INVALID)?;
        let document = decode(bytes, uri, control)?;
        fields(&operation.variables, object(&document)?, true)?;
        check(control)
    }

    pub fn validate_response(
        &self,
        operation: &str,
        bytes: &[u8],
        uri: &str,
        control: &OperationControl,
    ) -> Result<ResponseKind, String> {
        let operation = self.operations.get(operation).ok_or(INVALID)?;
        let document = decode(bytes, uri, control)?;
        let envelope = object(&document)?;
        if envelope
            .iter()
            .any(|member| !matches!(member.name.as_str(), "data" | "errors" | "extensions"))
        {
            return Err("cem.graphql.response.envelope".into());
        }
        if let Some(value) = get(envelope, "extensions") {
            object(value)?;
        }
        let data = get(envelope, "data");
        let errors = get(envelope, "errors");
        if let Some(errors) = errors {
            response_errors(errors, &operation.data, data.is_some())?;
        }
        let kind = match data {
            Some(Json::Object { members, .. }) => {
                fields(&operation.data, members, false)?;
                if errors.is_some() {
                    ResponseKind::Partial
                } else {
                    ResponseKind::Data
                }
            }
            Some(Json::Null { .. }) | None if errors.is_some() => ResponseKind::Errors,
            _ => return Err("cem.graphql.response.data".into()),
        };
        check(control)?;
        Ok(kind)
    }
}

fn hash(bytes: &[u8]) -> String {
    ContentHash::from_blake3(bytes).header_value()
}

fn check(control: &OperationControl) -> Result<(), String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| "cem.graphql.cancelled".into())
}

fn decode(bytes: &[u8], uri: &str, control: &OperationControl) -> Result<Json, String> {
    check(control)?;
    if bytes.len() > MAX_JSON_BYTES || uri.is_empty() || uri.len() > 4096 {
        return Err("cem.graphql.json.limit".into());
    }
    let report = extract_json_report_with_limits(
        JsonSourceValidationRequest {
            bytes,
            source_uri: uri,
            content_type: Some("application/json"),
        },
        None,
        Some((MAX_DEPTH, MAX_VALUES)),
    );
    if report
        .facts
        .iter()
        .any(|fact| fact.kind == JsonParseFactKind::DuplicateMemberName)
    {
        return Err("cem.graphql.json.duplicate".into());
    }
    check(control)?;
    report.root.ok_or_else(|| "cem.graphql.json.invalid".into())
}

fn object(value: &Json) -> Result<&[JsonMemberAst], String> {
    match value {
        Json::Object { members, .. } => Ok(members),
        _ => Err("cem.graphql.json.object".into()),
    }
}

fn get<'a>(members: &'a [JsonMemberAst], name: &str) -> Option<&'a Json> {
    members
        .iter()
        .find(|member| member.name == name)
        .map(|member| &member.value)
}

fn fields(
    expected: &BTreeMap<String, Field>,
    actual: &[JsonMemberAst],
    input: bool,
) -> Result<(), String> {
    let mut present_required = 0;
    for member in actual {
        let field = expected
            .get(&member.name)
            .ok_or("cem.graphql.field.unexpected")?;
        value(&field.shape, &member.value, input)
            .map_err(|error| format!("{}: {error}", member.name))?;
        present_required += usize::from(!field.optional);
    }
    // The native decoder already refused duplicate names.
    if present_required != expected.values().filter(|field| !field.optional).count() {
        return Err("cem.graphql.field.missing".into());
    }
    Ok(())
}

fn value(shape: &Shape, json: &Json, input: bool) -> Result<(), String> {
    let matches = match (shape, json) {
        (Shape::Nullable(_), Json::Null { .. }) => true,
        (Shape::Nullable(shape), json) => return value(shape, json, input),
        (Shape::Object(expected), Json::Object { members, .. }) => {
            return fields(expected, members, input)
        }
        (Shape::List(item), Json::Array { items, .. }) => {
            return items.iter().try_for_each(|json| value(item, json, input));
        }
        (Shape::List(item), json) if input && !matches!(json, Json::Null { .. }) => {
            return value(item, json, input)
        }
        (Shape::Enum(values), Json::String { value, .. }) => values.contains(value),
        (Shape::Scalar(Scalar::Id | Scalar::String), Json::String { .. })
        | (Shape::Scalar(Scalar::Boolean), Json::Boolean { .. }) => true,
        (Shape::Scalar(Scalar::Id), Json::Number { lexeme, .. }) if input => {
            integer_parts(lexeme).is_some()
        }
        (Shape::Scalar(Scalar::Int), Json::Number { lexeme, .. }) => int32(lexeme),
        (Shape::Scalar(Scalar::Float), Json::Number { lexeme, .. }) => {
            lexeme.parse::<f64>().is_ok_and(f64::is_finite)
        }
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err("cem.graphql.value.type".into())
    }
}

// JSON has a single number syntax. Check integrality on the source digits
// before rounding to avoid accepting fractions near the GraphQL Int boundary.
fn integer_parts(text: &str) -> Option<(bool, String, usize)> {
    let negative = text.starts_with('-');
    let text = text.trim_start_matches('-');
    let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((text, "0"));
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut digits = format!("{whole}{fraction}");
    if digits.bytes().all(|byte| byte == b'0') {
        return Some((false, "0".into(), 0));
    }
    let shift = exponent
        .parse::<i64>()
        .ok()?
        .checked_sub(fraction.len() as i64)?;
    if shift < 0 {
        let trim = usize::try_from(shift.unsigned_abs()).ok()?;
        let end = digits.len().checked_sub(trim)?;
        if !digits[end..].bytes().all(|byte| byte == b'0') {
            return None;
        }
        digits.truncate(end);
    }
    Some((
        negative,
        digits.trim_start_matches('0').into(),
        usize::try_from(shift.max(0)).ok()?,
    ))
}

fn int32(text: &str) -> bool {
    let Some((negative, digits, zeros)) = integer_parts(text) else {
        return false;
    };
    if digits.len().saturating_add(zeros) > 10 {
        return false;
    }
    digits.parse::<i64>().is_ok_and(|value| {
        let signed = if negative { -value } else { value };
        i32::try_from(signed * 10_i64.pow(zeros as u32)).is_ok()
    })
}

fn response_errors(
    errors: &Json,
    data: &BTreeMap<String, Field>,
    executed: bool,
) -> Result<(), String> {
    let Json::Array { items, .. } = errors else {
        return Err("cem.graphql.response.errors".into());
    };
    if items.is_empty() {
        return Err("cem.graphql.response.errors".into());
    }
    for item in items {
        let error = object(item)?;
        if !matches!(get(error, "message"), Some(Json::String { .. })) {
            return Err("cem.graphql.response.error_message".into());
        }
        if let Some(extensions) = get(error, "extensions") {
            object(extensions)?;
        }
        if let Some(locations) = get(error, "locations") {
            let Json::Array { items, .. } = locations else {
                return Err("cem.graphql.response.locations".into());
            };
            for item in items {
                let location = object(item)?;
                if location.len() != 2 || !["line", "column"].into_iter().all(|name|
                    matches!(get(location, name), Some(Json::Number { lexeme, .. }) if lexeme.parse::<u32>().is_ok_and(|n| n > 0)))
                {
                    return Err("cem.graphql.response.locations".into());
                }
            }
        }
        match get(error, "path") {
            Some(Json::Array { items, .. }) if valid_path(data, items) => (),
            None if !executed => (),
            _ => return Err("cem.graphql.response.error_path".into()),
        }
    }
    Ok(())
}

fn valid_path(data: &BTreeMap<String, Field>, path: &[Json]) -> bool {
    let Some((Json::String { value: name, .. }, rest)) = path.split_first() else {
        return false;
    };
    let Some(field) = data.get(name) else {
        return false;
    };
    let mut shape = &field.shape;
    for part in rest {
        if let Shape::Nullable(inner) = shape {
            shape = inner;
        }
        shape = match (shape, part) {
            (Shape::Object(fields), Json::String { value, .. }) => match fields.get(value) {
                Some(field) => &field.shape,
                None => return false,
            },
            (Shape::List(inner), Json::Number { lexeme, .. }) if lexeme.parse::<u32>().is_ok() => {
                inner
            }
            _ => return false,
        };
    }
    true
}
