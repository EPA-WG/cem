//! Explicit host admissions compiled into one immutable package snapshot. Source
//! execution labels, function names and query text never create admissions.
use crate::{
    datatype_compilation::DatatypeImplementations,
    datatype_conversion::{
        ConverterBinding, DatatypeConversionResultAdapter, RegisteredDatatypeConverter,
    },
    datatype_enumeration::{
        ConstantBinding, DatatypeConstantResultAdapter, DatatypeEqualityResultAdapter,
        EqualityBinding, RegisteredConstantInterpreter, RegisteredScalarEquality,
    },
    datatype_results::DatatypeResultAdapter,
    datatype_validation::{DatatypeValidationRegistry, LegacyAcceptance},
    schema_references::CemQlSchemaDeclarationHost,
};
use cem_ml::{
    diagnostics::Diagnostic,
    schema::{
        datatype_contracts::{DatatypeCompilationIssue, DatatypeIssueState},
        datatype_conversion::{ConversionBehaviorContract, ConversionSignature},
        datatype_enumeration::{ConstantBehaviorContract, EqualityBehaviorContract},
        datatype_registry::DatatypeSource,
        datatype_validation::{
            DatatypeBehaviorContract, ScalarRepresentation, ValidationSignature,
        },
        declaration_references::{SchemaDeclarationKind, SchemaDeclarationNode},
        document_model::SchemaDocumentModel,
        function_references::{
            CompiledFunctionBindings, FunctionCatalog, FunctionSelection, ScalarCompilationBudget,
        },
        value_contracts::{ValueContractError, ValueContractSource},
    },
    value::reference_resolution::ReferenceResolutionState,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub enum FunctionProfile {
    Validation {
        signature: ValidationSignature,
        adapter: DatatypeResultAdapter,
        legacy: Option<LegacyAcceptance>,
    },
    Conversion {
        datatype: DatatypeSource,
        signature: ConversionSignature,
        adapter: DatatypeConversionResultAdapter,
    },
    Equality {
        datatype: DatatypeSource,
        representation: ScalarRepresentation,
        adapter: DatatypeEqualityResultAdapter,
    },
    Constant {
        datatype: DatatypeSource,
        representation: ScalarRepresentation,
        adapter: DatatypeConstantResultAdapter,
    },
}
#[derive(Debug, Clone)]
pub struct FunctionAdmission {
    pub behavior: SchemaDeclarationNode,
    pub profile: FunctionProfile,
}
#[derive(Debug, Clone, Default)]
pub struct FunctionBindings {
    selections: Vec<(SchemaDeclarationNode, FunctionSelection)>,
    // Keep registrations alive even when a behavior is not used by a datatype.
    validators: DatatypeValidationRegistry,
    implementations: DatatypeImplementations,
    pub issues: Vec<DatatypeCompilationIssue>,
    pub diagnostics: Vec<Diagnostic>,
}
impl CompiledFunctionBindings for FunctionBindings {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
impl FunctionBindings {
    pub fn validators(&self) -> &DatatypeValidationRegistry {
        &self.validators
    }
    pub fn implementations(&self) -> &DatatypeImplementations {
        &self.implementations
    }
    pub fn selections(&self) -> impl Iterator<Item = (&SchemaDeclarationNode, &FunctionSelection)> {
        self.selections
            .iter()
            .map(|(behavior, selection)| (behavior, selection))
    }
    /// Only successful query registration can satisfy a native readiness guard.
    /// Owner-qualified identities prevent equal arena node IDs from crossing sites.
    pub fn activate(&self, model: &mut SchemaDocumentModel) {
        for site in &mut model.declaration_references.sites {
            if site.kind != SchemaDeclarationKind::BehaviorFunction || site.resolution.is_some() {
                continue;
            }
            let Some(caller) = model
                .declaration_references
                .function_callers
                .get(&site.occurrence.identity)
            else {
                continue;
            };
            if let Some((_, selection)) = self
                .selections
                .iter()
                .find(|(behavior, _)| behavior.identity() == *caller)
            {
                site.resolution = Some(selection.resolution.clone());
            }
        }
    }
    fn issue(&mut self, error: ValueContractError, fallback: &SchemaDeclarationNode) {
        let pending = error.code.contains("limit")
            || error.code.contains("bounds")
            || error.code.contains("pending")
            || error.code.contains("unavailable")
            || error.code.contains("incomplete");
        self.issues.push(DatatypeCompilationIssue {
            code: error.code,
            source: error.source.unwrap_or_else(|| fallback.clone()),
            related: None,
            state: if pending {
                DatatypeIssueState::Pending
            } else {
                DatatypeIssueState::Invalid
            },
        });
    }
}
/// Assembly, selection, signature/body inspection and registration share the
/// allowance subsequently used by datatype compilation. No user query is invoked.
pub fn compile_function_bindings(
    sources: &[ValueContractSource],
    exports: &[SchemaDeclarationNode],
    admissions: &[FunctionAdmission],
    host: &mut CemQlSchemaDeclarationHost,
    budget: &mut ScalarCompilationBudget,
    validators: DatatypeValidationRegistry,
    implementations: DatatypeImplementations,
) -> FunctionBindings {
    let mut output = FunctionBindings {
        validators,
        implementations,
        ..Default::default()
    };
    let Some(fallback) = sources
        .first()
        .map(|s| &s.schema)
        .or_else(|| admissions.first().map(|a| &a.behavior))
    else {
        return output;
    };
    let mut catalog = match FunctionCatalog::assemble(sources, host, budget) {
        Ok(catalog) => catalog,
        Err(error) => {
            output.issue(error, fallback);
            return output;
        }
    };
    for resolution in catalog.assembly_resolutions() {
        output.diagnostics.extend(resolution.diagnostics.clone());
    }
    if !catalog.assembly_is_complete() {
        output.issues.push(DatatypeCompilationIssue {
            code: "function-collection-incomplete",
            source: fallback.clone(),
            related: None,
            state: if catalog
                .assembly_resolutions()
                .iter()
                .any(|r| r.state == ReferenceResolutionState::Invalid)
            {
                DatatypeIssueState::Invalid
            } else {
                DatatypeIssueState::Pending
            },
        });
        return output;
    }
    for export in exports {
        if let Err(error) = catalog.export(export, budget) {
            output.issue(error, export);
        }
    }
    let mut seen = BTreeSet::new();
    for admission in admissions {
        let caller = &admission.behavior;
        if !seen.insert(caller.identity()) {
            output.issue(
                ValueContractError::new("duplicate-function-admission"),
                caller,
            );
            continue;
        }
        let selection = match catalog.select(caller, host, budget) {
            Ok(selection) => selection,
            Err(error) => {
                output.issue(error, caller);
                continue;
            }
        };
        output
            .diagnostics
            .extend(selection.resolution.diagnostics.clone());
        if selection.target().is_none() {
            output.issues.push(DatatypeCompilationIssue {
                code: "function-selection-incomplete",
                source: selection.attribute.clone(),
                related: None,
                state: if selection.resolution.state == ReferenceResolutionState::Invalid {
                    DatatypeIssueState::Invalid
                } else {
                    DatatypeIssueState::Pending
                },
            });
            continue;
        }
        let result = register(
            &selection,
            &admission.profile,
            budget,
            &mut output.validators,
            &mut output.implementations,
        );
        match result {
            Ok(()) => output.selections.push((caller.clone(), selection)),
            Err(error) => output.issue(error, caller),
        }
    }
    output
}
fn register(
    selection: &FunctionSelection,
    profile: &FunctionProfile,
    budget: &mut ScalarCompilationBudget,
    validators: &mut DatatypeValidationRegistry,
    implementations: &mut DatatypeImplementations,
) -> Result<(), ValueContractError> {
    match profile {
        FunctionProfile::Validation {
            signature,
            adapter,
            legacy,
        } => validators.register_query(
            DatatypeBehaviorContract::compile_selected(selection, signature.clone(), budget)?,
            adapter.clone(),
            legacy.clone(),
        ),
        FunctionProfile::Conversion {
            datatype,
            signature,
            adapter,
        } => {
            let contract = ConversionBehaviorContract::compile_selected(
                selection,
                *signature,
                adapter.result_contract().clone(),
                budget,
            )?;
            let registered = RegisteredDatatypeConverter::from_query(
                datatype.clone(),
                contract,
                adapter.clone(),
            )?;
            implementations
                .select_converter(datatype.clone(), ConverterBinding::Ready(registered))
                .map_err(ValueContractError::new)
        }
        FunctionProfile::Equality {
            datatype,
            representation,
            adapter,
        } => {
            let contract = EqualityBehaviorContract::compile_selected(
                selection,
                *representation,
                adapter.result_contract().clone(),
                budget,
            )?;
            let registered =
                RegisteredScalarEquality::from_query(datatype.clone(), contract, adapter.clone())?;
            implementations
                .select_equality(datatype.clone(), EqualityBinding::Ready(registered))
                .map_err(ValueContractError::new)
        }
        FunctionProfile::Constant {
            datatype,
            representation,
            adapter,
        } => {
            let contract = ConstantBehaviorContract::compile_selected(
                selection,
                *representation,
                adapter.result_contract().clone(),
                budget,
            )?;
            let registered = RegisteredConstantInterpreter::from_query(
                datatype.clone(),
                contract,
                adapter.clone(),
            )?;
            implementations
                .select_constant_interpreter(datatype.clone(), ConstantBinding::Ready(registered))
                .map_err(ValueContractError::new)
        }
    }
}
