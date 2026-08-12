//! Common product-version and runtime-capability contract.

use serde::{Deserialize, Serialize};

/// Version of the capability-manifest projection, independent of the product
/// version carried by [`ProductVersion`].
pub const CAPABILITY_CONTRACT_VERSION: u16 = 1;

/// Host-provided runtime, target, and ABI identities are bounded before they
/// enter a serialized manifest.
pub const MAX_IDENTITY_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductVersion {
    pub common_version: String,
}

pub fn product_version() -> ProductVersion {
    ProductVersion {
        common_version: crate::VERSION.to_owned(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeKind {
    Native,
    WasmNode,
    WasmBrowserWorker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityOperation {
    Parse,
    Validate,
    Check,
    Inspect,
    Convert,
    Query,
    Transform,
    Trace,
    VersionCapabilities,
    Bench,
    Fixture,
    SchemaMutation,
    PluginMutation,
}

impl CapabilityOperation {
    pub const ALL: [Self; 13] = [
        Self::Parse,
        Self::Validate,
        Self::Check,
        Self::Inspect,
        Self::Convert,
        Self::Query,
        Self::Transform,
        Self::Trace,
        Self::VersionCapabilities,
        Self::Bench,
        Self::Fixture,
        Self::SchemaMutation,
        Self::PluginMutation,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityAvailability {
    Available,
    DevelopmentOnly,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationCapability {
    pub operation: CapabilityOperation,
    pub availability: CapabilityAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRequest {
    pub runtime: RuntimeKind,
    pub target_identity: String,
    pub abi_identity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityManifest {
    pub contract_version: u16,
    pub common_version: String,
    pub runtime: RuntimeKind,
    pub target_identity: String,
    pub abi_identity: String,
    pub operations: Vec<OperationCapability>,
}

impl CapabilityManifest {
    pub fn availability(&self, operation: CapabilityOperation) -> CapabilityAvailability {
        self.operations
            .iter()
            .find(|entry| entry.operation == operation)
            .map(|entry| entry.availability)
            .expect("every capability operation is present in the common manifest")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityError {
    pub code: &'static str,
    pub field: &'static str,
    pub message: String,
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CapabilityError {}

pub fn capability_manifest(
    request: CapabilityRequest,
) -> Result<CapabilityManifest, CapabilityError> {
    validate_identity("targetIdentity", &request.target_identity)?;
    validate_identity("abiIdentity", &request.abi_identity)?;

    let operations = CapabilityOperation::ALL
        .into_iter()
        .map(|operation| OperationCapability {
            operation,
            availability: operation_availability(request.runtime, operation),
        })
        .collect();
    Ok(CapabilityManifest {
        contract_version: CAPABILITY_CONTRACT_VERSION,
        common_version: product_version().common_version,
        runtime: request.runtime,
        target_identity: request.target_identity,
        abi_identity: request.abi_identity,
        operations,
    })
}

fn operation_availability(
    runtime: RuntimeKind,
    operation: CapabilityOperation,
) -> CapabilityAvailability {
    use CapabilityAvailability::{Available, DevelopmentOnly, Unavailable};
    use CapabilityOperation::{
        Bench, Check, Convert, Fixture, Inspect, Parse, PluginMutation, Query, SchemaMutation,
        Trace, Transform, Validate, VersionCapabilities,
    };

    match operation {
        Parse | Validate | Check | Inspect | Convert | Query | Transform | Trace
        | VersionCapabilities => Available,
        Bench if runtime == RuntimeKind::Native => Available,
        Bench => Unavailable,
        Fixture if runtime != RuntimeKind::WasmBrowserWorker => DevelopmentOnly,
        Fixture | SchemaMutation | PluginMutation => Unavailable,
    }
}

fn validate_identity(field: &'static str, value: &str) -> Result<(), CapabilityError> {
    if value.is_empty() {
        return Err(CapabilityError {
            code: "cem.capability.identity_empty",
            field,
            message: format!("{field} must not be empty"),
        });
    }
    if value.len() > MAX_IDENTITY_BYTES {
        return Err(CapabilityError {
            code: "cem.capability.identity_too_long",
            field,
            message: format!(
                "{field} is {} bytes; the maximum is {MAX_IDENTITY_BYTES}",
                value.len()
            ),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(CapabilityError {
            code: "cem.capability.identity_control_character",
            field,
            message: format!("{field} must not contain control characters"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(runtime: RuntimeKind) -> CapabilityRequest {
        CapabilityRequest {
            runtime,
            target_identity: "x86_64-unknown-linux-gnu".to_owned(),
            abi_identity: "rust-v1".to_owned(),
        }
    }

    #[test]
    fn version_response_uses_the_common_cargo_version() {
        assert_eq!(product_version().common_version, crate::VERSION);
    }

    #[test]
    fn first_release_matrix_keeps_required_and_explicit_gap_operations() {
        let native = capability_manifest(request(RuntimeKind::Native)).unwrap();
        let node = capability_manifest(request(RuntimeKind::WasmNode)).unwrap();
        let browser = capability_manifest(request(RuntimeKind::WasmBrowserWorker)).unwrap();

        for operation in [
            CapabilityOperation::Parse,
            CapabilityOperation::Validate,
            CapabilityOperation::Check,
            CapabilityOperation::Inspect,
            CapabilityOperation::Convert,
            CapabilityOperation::Query,
            CapabilityOperation::Transform,
            CapabilityOperation::Trace,
            CapabilityOperation::VersionCapabilities,
        ] {
            assert_eq!(native.availability(operation), CapabilityAvailability::Available);
            assert_eq!(node.availability(operation), CapabilityAvailability::Available);
            assert_eq!(browser.availability(operation), CapabilityAvailability::Available);
        }

        assert_eq!(
            native.availability(CapabilityOperation::Bench),
            CapabilityAvailability::Available
        );
        assert_eq!(
            node.availability(CapabilityOperation::Bench),
            CapabilityAvailability::Unavailable
        );
        assert_eq!(
            browser.availability(CapabilityOperation::Fixture),
            CapabilityAvailability::Unavailable
        );
        for operation in [
            CapabilityOperation::SchemaMutation,
            CapabilityOperation::PluginMutation,
        ] {
            assert_eq!(native.availability(operation), CapabilityAvailability::Unavailable);
            assert_eq!(node.availability(operation), CapabilityAvailability::Unavailable);
            assert_eq!(browser.availability(operation), CapabilityAvailability::Unavailable);
        }
    }

    #[test]
    fn manifest_identity_fields_are_bounded_before_wire_projection() {
        let mut invalid = request(RuntimeKind::WasmNode);
        invalid.target_identity = "x".repeat(MAX_IDENTITY_BYTES + 1);
        assert_eq!(
            capability_manifest(invalid).unwrap_err().code,
            "cem.capability.identity_too_long"
        );

        let mut invalid = request(RuntimeKind::WasmNode);
        invalid.abi_identity.clear();
        assert_eq!(
            capability_manifest(invalid).unwrap_err().code,
            "cem.capability.identity_empty"
        );
    }

    #[test]
    fn serialized_manifest_has_stable_versioned_field_names() {
        let manifest = capability_manifest(request(RuntimeKind::WasmNode)).unwrap();
        let value = serde_json::to_value(manifest).unwrap();
        assert_eq!(value["contractVersion"], CAPABILITY_CONTRACT_VERSION);
        assert_eq!(value["commonVersion"], crate::VERSION);
        assert_eq!(value["runtime"], "wasm-node");
        assert_eq!(value["targetIdentity"], "x86_64-unknown-linux-gnu");
        assert_eq!(value["abiIdentity"], "rust-v1");
        assert_eq!(value["operations"][0]["operation"], "parse");
        assert_eq!(value["operations"][0]["availability"], "available");
    }
}
