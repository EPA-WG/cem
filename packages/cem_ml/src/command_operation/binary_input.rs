//! Passive binary input admission shared by invocation preview and execution.
use super::*;
use crate::import::projection::ProjectionKind;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript-projections", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct BinaryInputAdmissionV1 {
    pub input_id: String,
    pub uri: String,
    pub content_type: String,
    pub schema: String,
    pub byte_length: usize,
    pub sha256: String,
}
fn mime(identity: &FormatIdentity) -> Option<String> {
    identity.content_type.as_deref().map(|s| {
        s.split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    })
}
pub(super) fn binary(identity: &FormatIdentity) -> bool {
    mime(identity).is_some_and(|m| m.ends_with("+cem-bin"))
}
fn refused(uri: &str, message: impl Into<String>) -> CommandOperationPreparationError {
    CommandOperationPreparationError::BinaryInput {
        uri: uri.into(),
        message: message.into(),
    }
}
pub(super) fn merged_identity(
    uri: &str,
    plan: &FormatIdentity,
    resource: Option<&FormatIdentity>,
) -> Result<FormatIdentity, CommandOperationPreparationError> {
    let resource = resource.cloned().unwrap_or_default();
    let mut identity = plan.clone();
    if identity.content_type.is_none() {
        identity.content_type = resource.content_type.clone();
    }
    if identity.schema.is_none() {
        identity.schema = resource.schema.clone();
    }
    for (selected, reported) in [
        (&mut identity.base_uri, &resource.base_uri),
        (&mut identity.default_namespace, &resource.default_namespace),
    ] {
        if selected
            .as_ref()
            .zip(reported.as_ref())
            .is_some_and(|(a, b)| a != b)
        {
            return Err(refused(
                uri,
                "Owned binary scope conflicts with the command input",
            ));
        }
        if selected.is_none() {
            *selected = reported.clone();
        }
    }
    if !identity.namespaces.is_empty()
        && !resource.namespaces.is_empty()
        && identity.namespaces != resource.namespaces
    {
        return Err(refused(
            uri,
            "Owned binary namespace scope conflicts with the command input",
        ));
    }
    if identity.namespaces.is_empty() {
        identity.namespaces = resource.namespaces.clone();
    }
    let kind = ProjectionKind::identity(None, identity.content_type.as_deref())
        .filter(|kind| mime(&identity).as_deref() == Some(kind.binary_mime()))
        .ok_or_else(|| refused(uri, "No native binary source adapter admits this identity"))?;
    for declared in [plan, &resource] {
        if let Some(m) = mime(declared) {
            if m != kind.binary_mime() {
                return Err(refused(
                    uri,
                    "Owned binary content type conflicts with the command input",
                ));
            }
        }
        if let Some(schema) = declared.schema.as_deref() {
            if ProjectionKind::identity(Some(schema), None) != Some(kind) {
                return Err(refused(
                    uri,
                    "Owned binary schema conflicts with the command input",
                ));
            }
        }
    }
    Ok(identity)
}
pub(crate) fn admit(
    request: &CommandServiceRequestV1,
    plan: &NormalizedRunPlan,
) -> Result<Vec<BinaryInputAdmissionV1>, CommandOperationPreparationError> {
    let mut admitted = vec![];
    for input in &plan.inputs {
        let uri = input.resolved_uri.as_deref().unwrap_or(&input.declared_uri);
        let resource = inline_resource(request, uri)?;
        if !binary(&input.identity) && !resource.identity.as_ref().is_some_and(binary) {
            continue;
        }
        let identity = merged_identity(uri, &input.identity, resource.identity.as_ref())?;
        let kind = ProjectionKind::identity(None, identity.content_type.as_deref()).unwrap();
        crate::import::projection::load(
            &resource.bytes,
            uri,
            identity.content_type.as_deref(),
            kind,
        )
        .map_err(|e| refused(uri, e.to_string()))?;
        admitted.push(BinaryInputAdmissionV1 {
            input_id: input.input_id.clone(),
            uri: uri.into(),
            content_type: identity.content_type.unwrap(),
            schema: kind.schema().into(),
            byte_length: resource.bytes.len(),
            sha256: crate::command_service::sha256_hex(&resource.bytes),
        });
    }
    Ok(admitted)
}
