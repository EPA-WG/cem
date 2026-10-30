//! Checked native record-result profiles reuse the full compiler's adapters.
use super::*;
use crate::{
    datatype_conversion::{DatatypeConversionResultAdapter, SourceDatatypeConverter},
    datatype_enumeration::{
        DatatypeConstantResultAdapter, DatatypeEqualityResultAdapter, SourceConstantInterpreter,
        SourceScalarEquality,
    },
    native_registrations::SourceContract,
};
#[derive(Debug, Clone)]
pub enum NativeResultAdapter {
    Conversion(DatatypeConversionResultAdapter),
    Equality(DatatypeEqualityResultAdapter),
    Constant(DatatypeConstantResultAdapter),
}
pub(super) enum SourceRegistration {
    Conversion(RegisteredDatatypeConverter),
    Equality(RegisteredScalarEquality),
    Constant(RegisteredConstantInterpreter),
}
impl SourceRegistration {
    pub(super) fn install(
        &self,
        source: DatatypeSource,
        registry: &mut DatatypeImplementations,
    ) -> Result<(), &'static str> {
        match self {
            Self::Conversion(r) => {
                registry.select_converter(source, ConverterBinding::Ready(r.clone()))
            }
            Self::Equality(r) => {
                registry.select_equality(source, EqualityBinding::Ready(r.clone()))
            }
            Self::Constant(r) => {
                registry.select_constant_interpreter(source, ConstantBinding::Ready(r.clone()))
            }
        }
    }
}
proxy!(
    SourceDatatypeConverter,
    convert,
    ConversionCall,
    RuleExecution,
    SourceConversion
);
proxy!(
    SourceScalarEquality,
    compare,
    EqualityCall,
    RuleExecution,
    SourceEquality
);
proxy!(
    SourceConstantInterpreter,
    interpret,
    ConstantCall,
    RuleExecution,
    SourceConstant
);
impl FreshStructuralHost {
    register!(
        register_source_converter,
        SourceDatatypeConverter,
        SourceConversion
    );
    register!(
        register_source_equality,
        SourceScalarEquality,
        SourceEquality
    );
    register!(
        register_source_constant_interpreter,
        SourceConstantInterpreter,
        SourceConstant
    );
    pub fn link_native_source_operation(
        &self,
        requirement: Arc<VerifiedNativeRequirement>,
        scope: DeclarationScope,
        adapter: NativeResultAdapter,
    ) -> Result<FreshNativeOperation, NativeLinkError> {
        let entry = self.native_entry(&requirement, scope)?;
        let Requirement::SourceOperation(source, contract) = &requirement.requirement else {
            return Err(NativeLinkError::InvalidSource);
        };
        let active = Arc::new(AtomicBool::new(true));
        let guard = Guard {
            entry: Arc::downgrade(&entry),
            context: self.context.lease(),
            active: Arc::downgrade(&active),
            control: self.control.clone(),
        };
        let proxy = Proxy(guard.clone());
        let id = &requirement.implementation().id;
        let lease = Arc::new(Lease(guard.clone()));
        let registration = match (contract, adapter) {
            (SourceContract::Conversion(c), NativeResultAdapter::Conversion(adapter)) => {
                SourceRegistration::Conversion(
                    RegisteredDatatypeConverter::from_source(
                        source.clone(),
                        id,
                        c.clone(),
                        adapter,
                        proxy,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease),
                )
            }
            (SourceContract::Equality(c), NativeResultAdapter::Equality(adapter)) => {
                SourceRegistration::Equality(
                    RegisteredScalarEquality::from_source(
                        source.clone(),
                        id,
                        c.clone(),
                        adapter,
                        proxy,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease),
                )
            }
            (SourceContract::Constant(c), NativeResultAdapter::Constant(adapter)) => {
                SourceRegistration::Constant(
                    RegisteredConstantInterpreter::from_source(
                        source.clone(),
                        id,
                        c.clone(),
                        adapter,
                        proxy,
                    )
                    .map_err(|_| NativeLinkError::InvalidSource)?
                    .with_availability(lease),
                )
            }
            _ => return Err(NativeLinkError::InvalidSource),
        };
        self.check().map_err(|_| NativeLinkError::Expired)?;
        Ok(FreshNativeOperation {
            requirement,
            active,
            guard,
            source_registration: Some(registration),
        })
    }
}
