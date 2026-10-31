//! Separate CEMOS1 entry points share the exact native guards, never an NR1 cast.
use super::*;
use crate::native_registrations::dependency_sources::VerifiedOperationSource;

pub(super) enum OperationOwner {
    Closed(Arc<VerifiedNativeRequirement>),
    Dependency(Arc<VerifiedOperationSource>),
}
impl OperationOwner {
    pub(super) fn requirement(&self) -> &Requirement {
        match self {
            Self::Closed(r) => &r.requirement,
            Self::Dependency(r) => r.requirement(),
        }
    }
    pub(super) fn implementation(&self) -> &ImplementationIdentity {
        match self {
            Self::Closed(r) => r.implementation(),
            Self::Dependency(r) => r.implementation(),
        }
    }
    pub(super) fn entry(
        &self,
        host: &FreshStructuralHost,
        scope: DeclarationScope,
    ) -> Result<Arc<Entry>, NativeLinkError> {
        match self {
            Self::Closed(r) => host.native_entry(r, scope),
            Self::Dependency(r) => host.native_owner_entry(
                &r.pin().registry_hash,
                r.source(),
                r.implementation(),
                scope,
            ),
        }
    }
}
impl FreshStructuralHost {
    /// Bind code to the exact ready source scope. This resolves no dependencies
    /// and grants no crossing, compilation or publication authority.
    pub fn link_dependency_operation(
        &self,
        requirement: Arc<VerifiedOperationSource>,
        scope: DeclarationScope,
    ) -> Result<FreshNativeOperation, NativeLinkError> {
        self.link_operation_owner(OperationOwner::Dependency(requirement), scope)
    }
    pub fn link_dependency_source_operation(
        &self,
        requirement: Arc<VerifiedOperationSource>,
        scope: DeclarationScope,
        adapter: NativeResultAdapter,
    ) -> Result<FreshNativeOperation, NativeLinkError> {
        self.link_source_operation_owner(OperationOwner::Dependency(requirement), scope, adapter)
    }
    pub fn link_dependency_tokenizer(
        &self,
        requirement: Arc<VerifiedOperationSource>,
        scope: DeclarationScope,
    ) -> Result<FreshNativeTokenizer, NativeLinkError> {
        self.link_tokenizer_owner(OperationOwner::Dependency(requirement), scope)
    }
}
