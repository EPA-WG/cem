//! Explicit native binding and scoped consumption of original mixed receivers.
//! No component registry, renderer activation or generation/transport publication.
use super::*;
use crate::{
    application_envelope::mixed::{ReceiverContract, VerifiedMixedReceiverDeclaration},
    datatype_compilation::DatatypeValidation,
    datatype_preparation::PreparationLimits,
    datatype_validation::{ValidationInput, ValidationLimits},
};
use cem_ml::schema::{
    attribute_datatypes::AttributeDatatypeContextLease,
    document_model::{AttributeModel, AttributeValueContract, TypedAttributeValue},
    machine::LexicallyScopedDocument,
    structural_capsule::SourceIdentity,
};
use std::sync::{atomic::AtomicBool, Weak};
mod preparation;
mod sources;
mod values;
pub use preparation::{
    MixedPreparationEvidence, MixedPreparationInvocation, MixedPreparationResult,
};
pub use values::{MixedDatatypeValidation, ReceiverInput, ScalarValidation};
const REFUSED: &str = "native mixed receiver binding refused";
pub struct MixedReceiverSource {
    pub schema_uri: String,
    pub source: Arc<RetainedCemTree>,
    pub capture: Arc<LexicallyScopedDocument>,
    pub attributes: BTreeMap<String, AstNodeId>,
}
pub struct ScalarReceiverSource {
    pub identity: SourceIdentity,
    pub source: Arc<RetainedCemTree>,
    /// Primary model, followed by each original restriction in authored order.
    pub attributes: Vec<AstNodeId>,
}
enum Binding {
    Node {
        model: SchemaDocumentModel,
        guard: Arc<StructuralNodeReceiver>,
    },
    Scalar {
        contract: AttributeValueContract,
    },
    Executable {
        root: Weak<v3::InstalledDatatypeRoot>,
    },
}
struct Receiver {
    declaration: SchemaDeclarationNode,
    local: AttributeModel,
    binding: Binding,
}
struct Group {
    contract: Arc<VerifiedMixedReceiverDeclaration>,
    source: Arc<RetainedCemTree>,
    schema_uri: String,
    context: AttributeDatatypeContextLease,
    receivers: BTreeMap<String, Receiver>,
    active: AtomicBool,
    busy: AtomicBool,
}
impl Group {
    fn refresh(&self, host: &mut FreshStructuralHost) -> Result<(), String> {
        self.check(host)?;
        for receiver in self.receivers.values() {
            if let Binding::Executable { root } = &receiver.binding {
                root.upgrade()
                    .ok_or(REFUSED)?
                    .bind_receiver(host, receiver.declaration.clone())?;
                self.check(host)?;
            }
        }
        Ok(())
    }
    fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        host.check().map_err(|_| REFUSED)?;
        if !self.active.load(Ordering::Acquire) || !self.context.matches(&host.context) {
            return Err(REFUSED.into());
        }
        for receiver in self.receivers.values() {
            if host
                .inner
                .source_tree(&receiver.declaration)
                .is_none_or(|t| !Arc::ptr_eq(t, &self.source))
            {
                return Err(REFUSED.into());
            }
            match &receiver.binding {
                Binding::Node { guard, .. } if !guard.is_available(&host.context) => {
                    return Err(REFUSED.into())
                }
                Binding::Executable { root }
                    if root.upgrade().is_none_or(|r| !r.is_available(host)) =>
                {
                    return Err(REFUSED.into())
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn enter(&self, host: &FreshStructuralHost) -> Result<Entry<'_>, String> {
        self.check(host)?;
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| REFUSED)?;
        Ok(Entry(&self.busy))
    }
    fn retire(&self) {
        self.active.store(false, Ordering::Release);
        for r in self.receivers.values() {
            if let Binding::Node { guard, .. } = &r.binding {
                guard.retire_preparations();
            }
        }
    }
}
struct Entry<'a>(&'a AtomicBool);
impl Drop for Entry<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
pub struct LinkedMixedReceivers {
    group: Arc<Group>,
}
impl LinkedMixedReceivers {
    fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        self.group.check(host)?;
        if self.group.busy.load(Ordering::Acquire) {
            return Err(REFUSED.into());
        }
        Ok(())
    }
    /// Read-only owner/epoch liveness, not a selection or invocation receipt.
    /// Every consuming operation renews original type selections on the host.
    pub fn is_available(&self, host: &FreshStructuralHost) -> bool {
        self.check(host).is_ok()
    }
    pub fn contract(&self) -> &Arc<VerifiedMixedReceiverDeclaration> {
        &self.group.contract
    }
    pub fn receiver_count(&self) -> usize {
        self.group.receivers.len()
    }
}
impl Drop for LinkedMixedReceivers {
    fn drop(&mut self) {
        self.group.retire();
    }
}
#[derive(Default)]
pub struct MixedReceiverSlot {
    current: Option<LinkedMixedReceivers>,
}
impl MixedReceiverSlot {
    pub fn publish(
        &mut self,
        host: &mut FreshStructuralHost,
        staged: LinkedMixedReceivers,
    ) -> Result<(), String> {
        {
            let _entry = staged.group.enter(host)?;
            staged.group.refresh(host)?;
        }
        staged.check(host)?;
        let previous = self.current.replace(staged);
        drop(previous);
        Ok(())
    }
    pub fn current(&self, host: &FreshStructuralHost) -> Result<&LinkedMixedReceivers, String> {
        let current = self.current.as_ref().ok_or(REFUSED)?;
        current.check(host)?;
        Ok(current)
    }
    pub fn clear(&mut self) {
        self.current = None;
    }
}
pub fn stage(
    host: &mut FreshStructuralHost,
    contract: Arc<VerifiedMixedReceiverDeclaration>,
    source: MixedReceiverSource,
    scalar_sources: BTreeMap<String, ScalarReceiverSource>,
    roots: BTreeMap<String, Arc<v3::InstalledDatatypeRoot>>,
) -> Result<LinkedMixedReceivers, String> {
    host.check().map_err(|_| REFUSED)?;
    if contract.pin().registry_hash != host.registry_hash {
        return Err(REFUSED.into());
    }
    let receivers = sources::bind(host, &contract, &source, &scalar_sources, &roots)?;
    let result = LinkedMixedReceivers {
        group: Arc::new(Group {
            contract,
            source: source.source,
            schema_uri: source.schema_uri,
            context: host.context.lease(),
            receivers,
            active: AtomicBool::new(true),
            busy: AtomicBool::new(false),
        }),
    };
    result.check(host)?;
    Ok(result)
}
