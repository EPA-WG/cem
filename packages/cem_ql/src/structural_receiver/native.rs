//! Host-owned implementations and weak, generation-bound proxy registrations.
use super::query::{BindingOwner, FreshQueryBinding};
use super::*;
use crate::{
    datatype_compilation::{DatatypeImplementation, TokenizerBinding},
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{
        DatatypeValidationRegistry, NativeDatatypeValidator, RuleExecution, ValidationCall,
    },
    eval::RetainedCemNode,
    native_registrations::{
        self as requirements, ImplementationIdentity, NativeLinkError, NativeSignature,
        Requirement, TokenizerMode, VerifiedNativeRequirement,
    },
};
use cem_ml::schema::{
    attribute_datatypes::AttributeDatatypeContextLease,
    datatype_contracts::{RegisteredTokenizer, TokenizationError, TokenizationRequest, Tokenizer},
    datatype_registry::DatatypeKind,
};
use std::{ops::Range, sync::Weak};
mod operations;
pub use operations::{FreshNativeOperation, NativeResultAdapter};

pub(super) struct Entry {
    identity: ImplementationIdentity,
    actual: Actual,
}
enum Actual {
    Validator(Arc<dyn NativeDatatypeValidator>),
    Tokenizer(Arc<dyn Tokenizer>),
    Preparation(Arc<dyn crate::datatype_preparation::NativeLexicalPreparer>),
    Conversion(Arc<dyn crate::datatype_conversion::NativeDatatypeConverter>),
    Equality(Arc<dyn crate::datatype_enumeration::NativeScalarEquality>),
    ListSerializer(Arc<dyn crate::datatype_serialization::NativeListSerializer>),
    Constant(Arc<dyn crate::datatype_enumeration::NativeConstantInterpreter>),
    FacetProfile(cem_ml::schema::document_model::attribute_facets::FacetFamily),
    ListItems(cem_ml::schema::datatype_validation::ScalarRepresentation),
    SourceConversion(Arc<dyn crate::datatype_conversion::SourceDatatypeConverter>),
    SourceEquality(Arc<dyn crate::datatype_enumeration::SourceScalarEquality>),
    SourceConstant(Arc<dyn crate::datatype_enumeration::SourceConstantInterpreter>),
}
#[derive(Clone)]
struct Guard {
    entry: Weak<Entry>,
    context: AttributeDatatypeContextLease,
    active: Weak<AtomicBool>,
    control: OperationControl,
}
impl std::fmt::Debug for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeRegistrationGuard(..)")
    }
}
impl Guard {
    fn entry(&self) -> Option<Arc<Entry>> {
        if !self.context.is_live()
            || !self
                .active
                .upgrade()
                .is_some_and(|active| active.load(Ordering::Acquire))
            || self.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
        {
            return None;
        }
        self.entry.upgrade()
    }
}
#[derive(Debug)]
struct ValidatorProxy(Guard);
impl NativeDatatypeValidator for ValidatorProxy {
    fn validate(&self, call: ValidationCall<'_>) -> RuleExecution {
        let Some(entry) = self.0.entry() else {
            return RuleExecution::Unavailable(vec![]);
        };
        let Actual::Validator(callback) = &entry.actual else {
            return RuleExecution::Unavailable(vec![]);
        };
        let result = callback.validate(call);
        if self.0.entry().is_none() {
            RuleExecution::Unavailable(vec![])
        } else {
            result
        }
    }
}
#[derive(Debug)]
struct TokenizerProxy(Guard);
impl Tokenizer for TokenizerProxy {
    fn check_available(&self) -> Result<(), TokenizationError> {
        let entry = self.0.entry().ok_or(TokenizationError::Unavailable)?;
        let Actual::Tokenizer(callback) = &entry.actual else {
            return Err(TokenizationError::Unavailable);
        };
        let result = callback.check_available();
        self.0.entry().ok_or(TokenizationError::Unavailable)?;
        result
    }
    fn tokenize(
        &self,
        request: TokenizationRequest<'_>,
    ) -> Result<Vec<Range<usize>>, TokenizationError> {
        let entry = self.0.entry().ok_or(TokenizationError::Unavailable)?;
        let Actual::Tokenizer(callback) = &entry.actual else {
            return Err(TokenizationError::Unavailable);
        };
        let result = callback.tokenize(request);
        self.check_available()?;
        result
    }
    fn accepts_separator(&self, text: &str) -> bool {
        self.0.entry().is_some_and(|entry| {
            matches!(&entry.actual, Actual::Tokenizer(callback) if callback.accepts_separator(text))
        })
    }
}
pub struct FreshNativeTokenizer {
    requirement: Arc<VerifiedNativeRequirement>,
    tokenizer: RegisteredTokenizer,
    active: Arc<AtomicBool>,
    guard: Guard,
}
impl FreshNativeTokenizer {
    pub(super) fn is_live(&self) -> bool {
        self.guard.entry().is_some()
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
    pub fn install(
        &self,
        implementation: &mut DatatypeImplementation,
    ) -> Result<(), NativeLinkError> {
        self.guard.entry().ok_or(NativeLinkError::Expired)?;
        let Requirement::Tokenizer(source, mode) = &self.requirement.requirement else {
            return Err(NativeLinkError::InvalidSource);
        };
        if implementation.kind != DatatypeKind::List
            || implementation.source.scope().identity() != source.scope().identity()
            || implementation.source.declaration().identity() != source.declaration().identity()
        {
            return Err(NativeLinkError::InvalidSource);
        }
        implementation.tokenizer = match mode {
            TokenizerMode::Ready => TokenizerBinding::Ready(self.tokenizer.clone()),
            TokenizerMode::CheckedReplacement => {
                TokenizerBinding::CheckedReplacement(self.tokenizer.clone())
            }
        };
        Ok(())
    }
}
impl Drop for FreshNativeTokenizer {
    fn drop(&mut self) {
        self.retire();
    }
}
impl FreshStructuralHost {
    pub fn replace_native_validator(
        &self,
        previous: &FreshQueryBinding,
        requirement: Arc<VerifiedNativeRequirement>,
        scope: DeclarationScope,
        datatype: AstNodeId,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, NativeLinkError> {
        if !previous.live(self)
            || previous.scope != scope
            || !matches!(previous.declaration, BindingOwner::Native(_))
        {
            return Err(NativeLinkError::Expired);
        }
        let staged = self.link_native_validator(requirement, scope, datatype, adapter)?;
        self.check().map_err(|_| NativeLinkError::Expired)?;
        if previous
            .active
            .compare_exchange(true, false, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(NativeLinkError::Expired);
        }
        Ok(staged)
    }
    fn register_implementation(
        &mut self,
        identity: ImplementationIdentity,
        actual: Actual,
    ) -> Result<(), NativeLinkError> {
        self.check().map_err(|_| NativeLinkError::Expired)?;
        identity
            .check()
            .map_err(|_| NativeLinkError::IdentityMismatch)?;
        if self.native_implementations.contains_key(&identity.id) {
            return Err(NativeLinkError::Duplicate);
        }
        self.native_implementations
            .insert(identity.id.clone(), Arc::new(Entry { identity, actual }));
        self.context.advance();
        Ok(())
    }
    pub fn register_native_validator(
        &mut self,
        identity: ImplementationIdentity,
        callback: impl NativeDatatypeValidator + 'static,
    ) -> Result<(), NativeLinkError> {
        if !matches!(identity.signature, NativeSignature::Validator(_)) {
            return Err(NativeLinkError::IdentityMismatch);
        }
        self.register_implementation(identity, Actual::Validator(Arc::new(callback)))
    }
    pub fn register_native_tokenizer(
        &mut self,
        identity: ImplementationIdentity,
        callback: impl Tokenizer + 'static,
    ) -> Result<(), NativeLinkError> {
        if identity.signature != NativeSignature::TokenizerSpans {
            return Err(NativeLinkError::IdentityMismatch);
        }
        self.register_implementation(identity, Actual::Tokenizer(Arc::new(callback)))
    }
    pub fn remove_native_implementation(&mut self, id: &str) -> Result<(), NativeLinkError> {
        self.check().map_err(|_| NativeLinkError::Expired)?;
        if self.native_implementations.remove(id).is_none() {
            return Err(NativeLinkError::Unavailable);
        }
        self.context.advance();
        Ok(())
    }
    fn native_entry(
        &self,
        requirement: &VerifiedNativeRequirement,
        scope: DeclarationScope,
    ) -> Result<Arc<Entry>, NativeLinkError> {
        self.check().map_err(|_| NativeLinkError::Expired)?;
        if requirement.pin().registry_hash != self.registry_hash {
            return Err(NativeLinkError::IdentityMismatch);
        }
        self.inner
            .registered_validator_context(scope, requirement.source())
            .ok_or(NativeLinkError::InvalidSource)?;
        let entry = self
            .native_implementations
            .get(&requirement.implementation().id)
            .ok_or(NativeLinkError::Unavailable)?;
        if entry.identity != *requirement.implementation() {
            return Err(NativeLinkError::IdentityMismatch);
        }
        Ok(entry.clone())
    }
    pub fn link_native_validator(
        &self,
        requirement: Arc<VerifiedNativeRequirement>,
        scope: DeclarationScope,
        datatype: AstNodeId,
        adapter: DatatypeResultAdapter,
    ) -> Result<FreshQueryBinding, NativeLinkError> {
        let entry = self.native_entry(&requirement, scope)?;
        let Requirement::Validator(contract) = &requirement.requirement else {
            return Err(NativeLinkError::InvalidSource);
        };
        let selected =
            requirements::datatype(requirement.capture(), contract.owner().node_id(), datatype)
                .map_err(|_| NativeLinkError::InvalidSource)?;
        if selected.plan().kind
            != cem_ml::schema::datatype_registry::DatatypeKindSource::Explicit(
                contract.signature().kind,
            )
        {
            return Err(NativeLinkError::InvalidSource);
        }
        let active = Arc::new(AtomicBool::new(true));
        let proxy = ValidatorProxy(Guard {
            entry: Arc::downgrade(&entry),
            context: self.context.lease(),
            active: Arc::downgrade(&active),
            control: self.control.clone(),
        });
        let mut registry = DatatypeValidationRegistry::default();
        registry
            .register_native(&entry.identity.id, contract.clone(), adapter, None, proxy)
            .map_err(|_| NativeLinkError::InvalidSource)?;
        let datatype = RetainedCemNode::new(requirement.source().clone(), datatype)
            .ok_or(NativeLinkError::InvalidSource)?
            .query_item();
        let rule = registry
            .bind(
                contract.owner(),
                contract.behavior(),
                datatype,
                contract.signature().kind,
            )
            .map_err(|_| NativeLinkError::InvalidSource)?;
        self.check().map_err(|_| NativeLinkError::Expired)?;
        Ok(FreshQueryBinding {
            declaration: BindingOwner::Native(requirement),
            scope,
            context: self.context.lease(),
            rule,
            active,
        })
    }
    pub fn link_native_tokenizer(
        &self,
        requirement: Arc<VerifiedNativeRequirement>,
        scope: DeclarationScope,
    ) -> Result<FreshNativeTokenizer, NativeLinkError> {
        let entry = self.native_entry(&requirement, scope)?;
        if !matches!(requirement.requirement, Requirement::Tokenizer(..)) {
            return Err(NativeLinkError::InvalidSource);
        }
        let active = Arc::new(AtomicBool::new(true));
        let guard = Guard {
            entry: Arc::downgrade(&entry),
            context: self.context.lease(),
            active: Arc::downgrade(&active),
            control: self.control.clone(),
        };
        let tokenizer = RegisteredTokenizer::new(&entry.identity.id, TokenizerProxy(guard.clone()))
            .map_err(|_| NativeLinkError::InvalidSource)?;
        self.check().map_err(|_| NativeLinkError::Expired)?;
        Ok(FreshNativeTokenizer {
            requirement,
            tokenizer,
            active,
            guard,
        })
    }
}
