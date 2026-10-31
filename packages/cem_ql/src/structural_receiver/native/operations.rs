//! Exact native operation bindings feed the existing datatype compiler. These
//! selections retain source metadata, but only weak references to host code.
use super::*;
use crate::{
    datatype_compilation::DatatypeImplementations,
    datatype_conversion::{
        ConversionCall, ConversionExecution, ConverterBinding, NativeDatatypeConverter,
        RegisteredDatatypeConverter,
    },
    datatype_enumeration::{
        ConstantBinding, ConstantCall, ConstantExecution, EqualityBinding, EqualityCall,
        EqualityExecution, NativeConstantInterpreter, NativeScalarEquality,
        RegisteredConstantInterpreter, RegisteredScalarEquality,
    },
    datatype_facets::{FacetProfileBinding, RegisteredFacetProfile},
    datatype_preparation::{
        NativeLexicalPreparer, PreparationBinding, PreparationCall, PreparationExecution,
        RegisteredLexicalPreparation,
    },
    datatype_serialization::{
        ListSerializationCall, ListSerializerBinding, NativeListSerializer,
        RegisteredListSerializer, SerializationExecution,
    },
    native_registrations::{FacetMode, OperationMode, PreparationMode},
};
use cem_ml::schema::datatype_registry::DatatypeSource;
use cem_ml::schema::{
    datatype_validation::ScalarRepresentation, document_model::attribute_facets::FacetFamily,
};
pub use source_profiles::NativeResultAdapter;

#[derive(Debug)]
struct Lease(Guard);
impl crate::datatype_availability::DatatypeAvailability for Lease {
    fn is_available(&self) -> bool {
        self.0.entry().is_some()
    }
}

#[derive(Debug)]
struct Proxy(Guard);
// Always check after the call too: the implementation may have cancelled or
// retired its host while producing an otherwise successful result.
macro_rules! proxy {
    ($trait:ident, $method:ident, $call:ident, $execution:ident, $variant:ident) => {
        impl $trait for Proxy {
            fn $method(&self, call: $call<'_>) -> $execution {
                let Some(entry) = self.0.entry() else {
                    return $execution::Unavailable(vec![]);
                };
                let Actual::$variant(callback) = &entry.actual else {
                    return $execution::Unavailable(vec![]);
                };
                let result = callback.$method(call);
                if self.0.entry().is_none() {
                    $execution::Unavailable(vec![])
                } else {
                    result
                }
            }
        }
    };
}
proxy!(
    NativeLexicalPreparer,
    prepare,
    PreparationCall,
    PreparationExecution,
    Preparation
);
proxy!(
    NativeDatatypeConverter,
    convert,
    ConversionCall,
    ConversionExecution,
    Conversion
);
proxy!(
    NativeScalarEquality,
    compare,
    EqualityCall,
    EqualityExecution,
    Equality
);
proxy!(
    NativeListSerializer,
    serialize,
    ListSerializationCall,
    SerializationExecution,
    ListSerializer
);

proxy!(
    NativeConstantInterpreter,
    interpret,
    ConstantCall,
    ConstantExecution,
    Constant
);
/// Keep this owner alive for the lifetime of its installed compiler selections.
/// Neither this binding nor the resulting registrations serialize live authority.
pub struct FreshNativeOperation {
    requirement: OperationOwner,
    active: Arc<AtomicBool>,
    guard: Guard,
    source_registration: Option<source_profiles::SourceRegistration>,
}
impl FreshNativeOperation {
    pub fn is_live_on(&self, host: &FreshStructuralHost) -> bool {
        host.check().is_ok() && self.guard.context.matches(&host.context) && self.is_live()
    }
    pub(in crate::structural_receiver) fn is_live(&self) -> bool {
        self.guard.entry().is_some()
    }
    pub fn retire(&self) {
        self.active.store(false, Ordering::Release);
    }
    /// Install only on the exact original declaration. Selection failures leave
    /// the existing shared registry entry unchanged.
    pub fn install(
        &self,
        source: &DatatypeSource,
        implementations: &mut DatatypeImplementations,
    ) -> Result<(), NativeLinkError> {
        let entry = self.guard.entry().ok_or(NativeLinkError::Expired)?;
        let original = match self.requirement.requirement() {
            Requirement::Operation(original, _) | Requirement::SourceOperation(original, _) => {
                original
            }
            _ => return Err(NativeLinkError::InvalidSource),
        };
        if source.scope().identity() != original.scope().identity()
            || source.declaration().identity() != original.declaration().identity()
        {
            return Err(NativeLinkError::InvalidSource);
        }
        let identity = self.requirement.implementation();
        let proxy = Proxy(self.guard.clone());
        let source = source.clone();
        let lease = Arc::new(Lease(self.guard.clone()));
        let result = if let Some(registration) = &self.source_registration {
            registration.install(source, implementations)
        } else {
            let Requirement::Operation(_, mode) = self.requirement.requirement() else {
                return Err(NativeLinkError::InvalidSource);
            };
            match (&identity.signature, mode) {
                (NativeSignature::Preparation(signature), OperationMode::Preparation(mode)) => {
                    let registration = RegisteredLexicalPreparation::new(
                        source.clone(),
                        &identity.id,
                        *signature,
                        proxy,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease);
                    let binding = match mode {
                        PreparationMode::Ready => PreparationBinding::Ready(registration),
                        PreparationMode::CheckedReplacement => {
                            PreparationBinding::CheckedReplacement(registration)
                        }
                    };
                    implementations.select_preparation(source, binding)
                }
                (NativeSignature::Conversion(signature), OperationMode::Conversion) => {
                    let registration = RegisteredDatatypeConverter::new(
                        source.clone(),
                        &identity.id,
                        *signature,
                        proxy,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease);
                    implementations.select_converter(source, ConverterBinding::Ready(registration))
                }
                (NativeSignature::Equality(representation), OperationMode::Equality) => {
                    let registration = RegisteredScalarEquality::new(
                        source.clone(),
                        &identity.id,
                        *representation,
                        proxy,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease);
                    implementations.select_equality(source, EqualityBinding::Ready(registration))
                }
                (NativeSignature::ListSerializer(signature), OperationMode::ListSerializer) => {
                    let registration = RegisteredListSerializer::new(
                        source.clone(),
                        &identity.id,
                        *signature,
                        proxy,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease);
                    implementations
                        .select_list_serializer(source, ListSerializerBinding::Ready(registration))
                }
                (NativeSignature::Constant(rep), OperationMode::Constant) => {
                    let registration = RegisteredConstantInterpreter::new(
                        source.clone(),
                        &identity.id,
                        *rep,
                        proxy,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease);
                    implementations
                        .select_constant_interpreter(source, ConstantBinding::Ready(registration))
                }
                (NativeSignature::FacetProfile(family), OperationMode::FacetProfile(mode)) => {
                    let Actual::FacetProfile(actual) = &entry.actual else {
                        return Err(NativeLinkError::IdentityMismatch);
                    };
                    if actual != family {
                        return Err(NativeLinkError::IdentityMismatch);
                    }
                    let registration =
                        RegisteredFacetProfile::new(source.clone(), &identity.id, *actual)
                            .map_err(|_| NativeLinkError::InvalidSource)?
                            .with_availability(lease);
                    implementations.select_facets(
                        source,
                        match mode {
                            FacetMode::Ready => FacetProfileBinding::Ready(registration),
                            FacetMode::CheckedReplacement => {
                                FacetProfileBinding::CheckedReplacement(registration)
                            }
                        },
                    )
                }
                (NativeSignature::ListItems(item), OperationMode::ListItems(mode)) => {
                    let Actual::ListItems(actual) = &entry.actual else {
                        return Err(NativeLinkError::IdentityMismatch);
                    };
                    if actual != item {
                        return Err(NativeLinkError::IdentityMismatch);
                    }
                    let registration = RegisteredLexicalPreparation::list_items(
                        source.clone(),
                        &identity.id,
                        *actual,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease);
                    implementations.select_preparation(
                        source,
                        match mode {
                            PreparationMode::Ready => PreparationBinding::Ready(registration),
                            PreparationMode::CheckedReplacement => {
                                PreparationBinding::CheckedReplacement(registration)
                            }
                        },
                    )
                }
                _ => return Err(NativeLinkError::InvalidSource),
            }
        };
        result.map_err(|code| match code {
            "duplicate-preparation-selection"
            | "duplicate-converter-selection"
            | "duplicate-scalar-capability-selection"
            | "duplicate-list-serializer-selection" => NativeLinkError::Duplicate,
            "duplicate-facet-profile-selection" => NativeLinkError::Duplicate,
            _ => NativeLinkError::InvalidSource,
        })
    }
}
impl Drop for FreshNativeOperation {
    fn drop(&mut self) {
        self.retire();
    }
}

macro_rules! register {
    ($method:ident, $trait:ident, $variant:ident) => {
        pub fn $method(
            &mut self,
            identity: ImplementationIdentity,
            callback: impl $trait + 'static,
        ) -> Result<(), NativeLinkError> {
            if !matches!(identity.signature, NativeSignature::$variant(_)) {
                return Err(NativeLinkError::IdentityMismatch);
            }
            self.register_implementation(identity, Actual::$variant(Arc::new(callback)))
        }
    };
}
impl FreshStructuralHost {
    register!(
        register_native_constant_interpreter,
        NativeConstantInterpreter,
        Constant
    );
    register!(register_native_preparer, NativeLexicalPreparer, Preparation);
    register!(
        register_native_converter,
        NativeDatatypeConverter,
        Conversion
    );
    register!(register_native_equality, NativeScalarEquality, Equality);
    register!(
        register_native_list_serializer,
        NativeListSerializer,
        ListSerializer
    );

    pub fn register_native_facet_profile(
        &mut self,
        identity: ImplementationIdentity,
        actual: FacetFamily,
    ) -> Result<(), NativeLinkError> {
        if identity.signature != NativeSignature::FacetProfile(actual) {
            return Err(NativeLinkError::IdentityMismatch);
        }
        self.register_implementation(identity, Actual::FacetProfile(actual))
    }
    pub fn register_list_item_preparation(
        &mut self,
        identity: ImplementationIdentity,
        actual: ScalarRepresentation,
    ) -> Result<(), NativeLinkError> {
        if identity.signature != NativeSignature::ListItems(actual) {
            return Err(NativeLinkError::IdentityMismatch);
        }
        self.register_implementation(identity, Actual::ListItems(actual))
    }
    pub fn link_native_operation(
        &self,
        requirement: Arc<VerifiedNativeRequirement>,
        scope: DeclarationScope,
    ) -> Result<FreshNativeOperation, NativeLinkError> {
        self.link_operation_owner(OperationOwner::Closed(requirement), scope)
    }
    pub(super) fn link_operation_owner(
        &self,
        requirement: OperationOwner,
        scope: DeclarationScope,
    ) -> Result<FreshNativeOperation, NativeLinkError> {
        let entry = requirement.entry(self, scope)?;
        if !matches!(requirement.requirement(), Requirement::Operation(..)) {
            return Err(NativeLinkError::InvalidSource);
        }
        let active = Arc::new(AtomicBool::new(true));
        let guard = Guard {
            entry: Arc::downgrade(&entry),
            context: self.context.lease(),
            active: Arc::downgrade(&active),
            control: self.control.clone(),
        };
        self.check().map_err(|_| NativeLinkError::Expired)?;
        Ok(FreshNativeOperation {
            requirement,
            active,
            guard,
            source_registration: None,
        })
    }
}

mod source_profiles;
