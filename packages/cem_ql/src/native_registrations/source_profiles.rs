//! Original, inert native behavior profiles. Live adapters and callbacks must be
//! supplied by the fresh host; verification never compiles/evaluates a query.
use super::*;
use cem_ml::schema::{
    datatype_conversion::ConversionBehaviorContract,
    datatype_enumeration::{ConstantBehaviorContract, EqualityBehaviorContract},
    value_contracts::ContractName,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceProfile {
    Conversion,
    Equality,
    Constant,
}
pub(crate) enum SourceContract {
    Conversion(ConversionBehaviorContract),
    Equality(EqualityBehaviorContract),
    Constant(ConstantBehaviorContract),
}
pub fn export_source_operation(
    capture: &LexicallyScopedDocument,
    source: &DatatypeSource,
    behavior: &SchemaDeclarationNode,
    profile: SourceProfile,
    identity: SourceIdentity,
    implementation: ImplementationIdentity,
    registry_hash: &str,
    control: &OperationControl,
) -> Result<BuiltNativeRequirement, String> {
    if !Arc::ptr_eq(capture.document(), source.declaration().document())
        || !Arc::ptr_eq(capture.document(), behavior.document())
    {
        return Err(INVALID.into());
    }
    build(
        capture,
        identity,
        implementation,
        Selection::SourceOperation {
            scope: source.scope().node_id(),
            declaration: source.declaration().node_id(),
            behavior: behavior.node_id(),
            profile,
        },
        registry_hash,
        control,
    )
}
pub(super) fn restore(
    capture: &Arc<LexicallyScopedDocument>,
    datatype: &DatatypeSource,
    behavior: u32,
    profile: SourceProfile,
    implementation: &ImplementationIdentity,
    work: &Work<'_>,
) -> Result<SourceContract, String> {
    let scope = datatype.scope().node_id();
    if !member(capture, scope, behavior, "behaviors", "behavior") {
        return Err(INVALID.into());
    }
    let source = validator_source(capture, scope, behavior, work)?;
    let behavior =
        SchemaDeclarationNode::new(capture.document().clone(), behavior).ok_or(INVALID)?;
    let result = |name| ContractName::new(CEM_SCHEMA_URI, name);
    let (contract, actual) = match (&implementation.signature, profile) {
        (NativeSignature::SourceConversion(signature), SourceProfile::Conversion) => {
            let mut direct = implementation.clone();
            direct.signature = NativeSignature::Conversion(*signature);
            check_operation(datatype, &direct, OperationMode::Conversion)?;
            let c = ConversionBehaviorContract::compile(
                &source,
                &behavior,
                *signature,
                result("datatype-conversion-result"),
            )
            .map_err(|error| format!("{INVALID}: {}", error.code))?;
            let actual = c.implementation().clone();
            (SourceContract::Conversion(c), actual)
        }
        (NativeSignature::SourceEquality(rep), SourceProfile::Equality) => {
            let mut direct = implementation.clone();
            direct.signature = NativeSignature::Equality(*rep);
            check_operation(datatype, &direct, OperationMode::Equality)?;
            let c = EqualityBehaviorContract::compile(
                &source,
                &behavior,
                *rep,
                result("datatype-equality-result"),
            )
            .map_err(|error| format!("{INVALID}: {}", error.code))?;
            let actual = c.implementation().clone();
            (SourceContract::Equality(c), actual)
        }
        (NativeSignature::SourceConstant(rep), SourceProfile::Constant) => {
            let mut direct = implementation.clone();
            direct.signature = NativeSignature::Constant(*rep);
            check_operation(datatype, &direct, OperationMode::Constant)?;
            let c = ConstantBehaviorContract::compile(
                &source,
                &behavior,
                *rep,
                result("datatype-constant-result"),
            )
            .map_err(|error| format!("{INVALID}: {}", error.code))?;
            let actual = c.implementation().clone();
            (SourceContract::Constant(c), actual)
        }
        _ => return Err(INVALID.into()),
    };
    if !matches!(actual, ValidationImplementation::Native(id) if id == implementation.id) {
        return Err(INVALID.into());
    }
    work.check()?;
    Ok(contract)
}
