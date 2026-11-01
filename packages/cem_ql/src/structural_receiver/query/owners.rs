//! Original source/registry/request lifetimes. Inventory categories group native
//! owners by role; no wire format or handle reconstruction is needed.
use super::*;
use cem_ml::runtime_admission::{
    generation::NativeGeneration, NativeResourceLease, NativeRootKind,
};
use std::sync::Weak;

pub(in crate::structural_receiver) struct RuntimeOwner {
    runtime: Option<Weak<NativeGeneration>>,
    _lease: Option<NativeResourceLease>,
}
impl RuntimeOwner {
    pub(in crate::structural_receiver) fn new(
        host: &FreshStructuralHost,
        kind: NativeRootKind,
    ) -> Result<Self, String> {
        Ok(Self {
            runtime: host.runtime.clone(),
            _lease: host.runtime_resource(kind)?,
        })
    }
    pub(in crate::structural_receiver) fn current(&self) -> bool {
        self.runtime
            .as_ref()
            .is_none_or(|g| g.upgrade().is_some_and(|g| g.is_open()))
    }
    pub(in crate::structural_receiver) fn matches(&self, host: &FreshStructuralHost) -> bool {
        self.current()
            && match (&self.runtime, &host.runtime) {
                (None, None) => true,
                (Some(a), Some(b)) => Weak::ptr_eq(a, b),
                _ => false,
            }
    }
}
/// Retains original shared components of a verified declaration, including declarations
/// held by value inside an original module/assembly. No new declaration is made.
pub(in crate::structural_receiver) struct QuerySourceLease {
    source: Arc<RetainedCemTree>,
    _capture: Arc<cem_ml::schema::machine::LexicallyScopedDocument>,
    _program: Arc<crate::query_validator_capsule::VerifiedValidatorQuery>,
    pub(super) scope: DeclarationScope,
    context: AttributeDatatypeContextLease,
    active: AtomicBool,
    _runtime: RuntimeOwner,
}
impl QuerySourceLease {
    pub(in crate::structural_receiver) fn current(&self) -> bool {
        self.active.load(Ordering::Acquire) && self.context.is_live() && self._runtime.current()
    }
    fn live(&self, host: &FreshStructuralHost) -> bool {
        self.current()
            && self._runtime.matches(host)
            && self.context.matches(&host.context)
            && host
                .inner
                .registered_validator_context(self.scope, &self.source)
                .is_some()
    }
}
/// Explicit original source owner. Bindings borrow its authority weakly, while
/// retaining their own inert source/program data until they actually drop.
pub struct FreshQuerySource {
    declaration: Arc<VerifiedQueryDeclaration>,
    pub(super) owner: QuerySourceLease,
}
impl FreshQuerySource {
    pub fn declaration(&self) -> &Arc<VerifiedQueryDeclaration> {
        &self.declaration
    }
    pub fn retire(&self) {
        self.owner.active.store(false, Ordering::Release);
    }
    pub(in crate::structural_receiver) fn live(&self, host: &FreshStructuralHost) -> bool {
        self.owner.live(host)
    }
}
impl FreshStructuralHost {
    pub(in crate::structural_receiver) fn query_source_lease(
        &self,
        declaration: &VerifiedQueryDeclaration,
        scope: DeclarationScope,
    ) -> Result<QuerySourceLease, String> {
        let _entry = self.runtime_entry()?;
        self.check().map_err(|_| INVALID)?;
        if declaration.pin().program.registry_hash != self.registry_hash {
            return Err(INVALID.into());
        }
        self.inner
            .registered_validator_context(scope, declaration.source())
            .ok_or(INVALID)?;
        let source = QuerySourceLease {
            source: declaration.source().clone(),
            _capture: declaration.capture().clone(),
            _program: declaration.program.clone(),
            scope,
            context: self.context.lease(),
            active: AtomicBool::new(true),
            _runtime: RuntimeOwner::new(self, NativeRootKind::Compilation)?,
        };
        self.check().map_err(|_| INVALID)?;
        if !source.live(self) {
            return Err(INVALID.into());
        }
        Ok(source)
    }
    pub fn query_source(
        &self,
        declaration: Arc<VerifiedQueryDeclaration>,
        scope: DeclarationScope,
    ) -> Result<Arc<FreshQuerySource>, String> {
        let _entry = self.runtime_entry()?;
        let owner = self.query_source_lease(&declaration, scope)?;
        let source = Arc::new(FreshQuerySource { declaration, owner });
        self.check().map_err(|_| INVALID)?;
        if !source.live(self) {
            return Err(INVALID.into());
        }
        Ok(source)
    }
}
