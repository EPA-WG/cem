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
use std::sync::{atomic::AtomicBool, OnceLock, Weak};
mod preparation;
mod sources;
mod values;
#[cfg(any(feature = "component-runtime", feature = "application-source-runtime"))]
mod frames;
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
pub(super) struct Group {
    contract: Arc<VerifiedMixedReceiverDeclaration>,
    source: Arc<RetainedCemTree>,
    schema_uri: String,
    context: AttributeDatatypeContextLease,
    receivers: BTreeMap<String, Receiver>,
    active: AtomicBool,
    busy: AtomicBool,
    publication: OnceLock<Weak<super::publication::Guard>>,
}
impl Group {
    fn publication(&self) -> Result<Option<Arc<super::publication::Guard>>, String> {
        self.publication
            .get()
            .map(|guard| guard.upgrade().ok_or_else(|| REFUSED.into()))
            .transpose()
    }
    pub(super) fn enroll(&self, guard: &Arc<super::publication::Guard>) -> Result<(), String> {
        self.publication
            .set(Arc::downgrade(guard))
            .map_err(|_| REFUSED.into())
    }
    fn refresh(&self, host: &mut FreshStructuralHost) -> Result<(), String> {
        if let Some(guard) = self.publication()? {
            guard.refresh(host)
        } else {
            self.refresh_local(host)
        }
    }
    pub(super) fn refresh_local(&self, host: &mut FreshStructuralHost) -> Result<(), String> {
        self.check_local(host)?;
        for receiver in self.receivers.values() {
            if let Binding::Executable { root } = &receiver.binding {
                root.upgrade()
                    .ok_or(REFUSED)?
                    .bind_receiver(host, receiver.declaration.clone())?;
                self.check_local(host)?;
            }
        }
        Ok(())
    }
    fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        self.check_local(host)?;
        if let Some(guard) = self.publication()? {
            guard.check(host)?;
        }
        Ok(())
    }
    pub(super) fn check_local(&self, host: &FreshStructuralHost) -> Result<(), String> {
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
        let publication = self
            .publication()?
            .map(|guard| guard.enter(host))
            .transpose()?;
        let runtime = host.runtime_entry()?;
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| REFUSED)?;
        Ok(Entry {
            busy: &self.busy,
            _publication: publication,
            _runtime: runtime,
        })
    }
    pub(super) fn retire(&self) {
        self.active.store(false, Ordering::Release);
        for r in self.receivers.values() {
            if let Binding::Node { guard, .. } = &r.binding {
                guard.retire_preparations();
            }
        }
    }
}

impl LinkedMixedReceivers {
    /// Closed scalar ingress only. Executable/node values need their explicit
    /// native receiver input adapters; a type label cannot grant that authority.
    pub(crate) fn runtime_scalar_contracts(
        &self,
        host: &mut FreshStructuralHost,
    ) -> Result<
        (
            BTreeMap<String, AttributeValueContract>,
            BTreeMap<String, AttributeValueContract>,
        ),
        String,
    > {
        let _entry = self.group.enter(host)?;
        self.group.refresh(host)?;
        let model = cem_ml::schema::document_model::compile_schema_document_model_from_retained(
            &self.group.schema_uri,
            &self.group.source,
        );
        if model.elements.len() > 1
            || model.elements.values().any(|e| {
                !e.required_attributes.is_empty()
                    || !e.child_elements.is_empty()
                    || e.allow_any_child
                    || !e.field_contracts.is_empty()
            })
        {
            return Err(
                "compiled application instance topology needs an original input adapter".into(),
            );
        }
        let mut fields = BTreeMap::new();
        let mut types = BTreeMap::new();
        for (name, receiver) in &self.group.receivers {
            let Binding::Scalar { contract } = &receiver.binding else {
                return Err("compiled application receiver ingress unavailable".into());
            };
            fields.insert(name.clone(), contract.clone());
            let alias = &self.group.contract.occurrences()[name].fields["type"];
            if let Some(ReceiverContract::Scalar(source)) = self.group.contract.receiver(alias) {
                types.insert(alias.clone(), source.value.clone());
            }
        }
        self.group.refresh(host)?;
        Ok((fields, types))
    }
}
struct Entry<'a> {
    busy: &'a AtomicBool,
    _publication: Option<super::publication::Entry>,
    _runtime: Option<cem_ml::runtime_admission::OwnedNativeEntry>,
}
impl Drop for Entry<'_> {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}
pub struct LinkedMixedReceivers {
    pub(super) group: Arc<Group>,
}
impl LinkedMixedReceivers {
    fn check(&self, host: &FreshStructuralHost) -> Result<(), String> {
        self.group.check(host)?;
        if self.group.busy.load(Ordering::Acquire)
            || self
                .group
                .publication()?
                .is_some_and(|guard| guard.is_busy())
        {
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
    let _runtime = host.runtime_entry()?;
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
            publication: OnceLock::new(),
        }),
    };
    result.check(host)?;
    Ok(result)
}
