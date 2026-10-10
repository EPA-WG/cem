//! Explicit scalar equality and constant interpretation. Neither capability converts
//! runtime input, resolves references, or grants access to another scope.
use crate::{
    datatype_results::DiagnosticAttribution,
    datatype_validation::{ValidationLimits, ValidationRuntime, ValidationStopReason},
    eval::{Item, RetainedCemNode},
};
use cem_ml::{
    diagnostics::Diagnostic,
    parser::tree::RetainedCemTree,
    schema::{
        datatype_registry::DatatypeSource, datatype_validation::ScalarRepresentation,
        declaration_references::SchemaDeclarationNode,
    },
};
use std::{fmt::Debug, ops::Range, sync::Arc};

#[derive(Debug, Clone)]
pub struct ScalarCapabilityIdentity {
    pub source: DatatypeSource,
    pub implementation: String,
}
/// Byte span in the retained, decoded attribute value, not a fabricated source range.
#[derive(Debug, Clone)]
pub struct ConstantToken {
    pub source: SchemaDeclarationNode,
    pub lexical: Arc<str>,
    pub span: Range<usize>,
    pub form: ConstantForm,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantForm {
    WhitespaceToken,
    /// The complete decoded @value, including an explicitly empty string.
    RetainedLiteral,
}
impl ConstantToken {
    pub fn text(&self) -> &str {
        &self.lexical[self.span.clone()]
    }
}
#[derive(Debug, Clone)]
pub enum ConstantExecution {
    Prepared {
        value: Vec<Item>,
        diagnostics: Vec<Diagnostic>,
    },
    Rejected(Vec<Diagnostic>),
    Pending(Vec<Diagnostic>),
    Unavailable(Vec<Diagnostic>),
    Failed(Vec<Diagnostic>),
}
#[derive(Debug, Clone)]
pub enum EqualityExecution {
    Complete {
        equal: bool,
        diagnostics: Vec<Diagnostic>,
    },
    Pending(Vec<Diagnostic>),
    Unavailable(Vec<Diagnostic>),
    Failed(Vec<Diagnostic>),
}
#[path = "datatype_enumeration/source.rs"]
mod source;
pub use crate::datatype_results::enumeration::{
    DatatypeConstantResult, DatatypeConstantResultAdapter, DatatypeEqualityResult,
    DatatypeEqualityResultAdapter,
};
pub use source::{SourceConstantInterpreter, SourceScalarEquality};

pub struct ConstantCall<'a> {
    pub token: &'a ConstantToken,
    /// Original @values or constant @value attribute, in its retained view.
    pub candidate: &'a Item,
    pub datatype: &'a Item,
    pub runtime: &'a ValidationRuntime<'a>,
    pub fallback: &'a DiagnosticAttribution,
    pub limits: ValidationLimits,
}
pub struct EqualityCall<'a> {
    pub left: &'a Item,
    pub right: &'a Item,
    pub datatype: &'a Item,
    pub runtime: &'a ValidationRuntime<'a>,
    pub fallback: &'a DiagnosticAttribution,
    pub limits: ValidationLimits,
}
/// Callbacks cooperate with the supplied control and bound their own internal work.
pub trait NativeConstantInterpreter: Debug + Send + Sync {
    fn interpret(&self, call: ConstantCall<'_>) -> ConstantExecution;
}
pub trait NativeScalarEquality: Debug + Send + Sync {
    fn compare(&self, call: EqualityCall<'_>) -> EqualityExecution;
}
macro_rules! capability {
    ($registration:ident, $binding:ident, $bound:ident, $native:ident) => {
        #[derive(Debug, Clone)]
        pub struct $registration {
            identity: ScalarCapabilityIdentity,
            representation: ScalarRepresentation,
            pub(crate) implementation: Arc<dyn $native>,
        }
        impl $registration {
            pub fn new(
                source: DatatypeSource,
                id: impl Into<String>,
                representation: ScalarRepresentation,
                implementation: impl $native + 'static,
            ) -> Result<Self, &'static str> {
                let id = id.into();
                if id.trim().is_empty() {
                    return Err("empty-scalar-capability-id");
                }
                Ok(Self {
                    identity: ScalarCapabilityIdentity {
                        source,
                        implementation: id,
                    },
                    representation,
                    implementation: Arc::new(implementation),
                })
            }
            pub fn identity(&self) -> &ScalarCapabilityIdentity {
                &self.identity
            }
            pub fn representation(&self) -> ScalarRepresentation {
                self.representation
            }
            pub(crate) fn bind(&self, tree: Arc<RetainedCemTree>) -> Option<$bound> {
                let source = self.identity.source.declaration();
                if !Arc::ptr_eq(tree.ast_owner(), source.document()) {
                    return None;
                }
                Some($bound {
                    registration: self.clone(),
                    datatype: RetainedCemNode::new(tree, source.node_id())?.query_item(),
                })
            }
        }
        #[derive(Debug, Clone)]
        pub enum $binding {
            Unavailable,
            Ready($registration),
        }
        #[derive(Debug, Clone)]
        pub(crate) struct $bound {
            pub registration: $registration,
            pub datatype: Item,
        }
    };
}
capability!(
    RegisteredScalarEquality,
    EqualityBinding,
    BoundEquality,
    NativeScalarEquality
);
capability!(
    RegisteredConstantInterpreter,
    ConstantBinding,
    BoundInterpreter,
    NativeConstantInterpreter
);
#[derive(Debug, Clone)]
pub struct PreparedConstant {
    pub token: ConstantToken,
    pub value: Item,
    /// Original retained declaration, absent for legacy @values tokens.
    pub declaration: Option<SchemaDeclarationNode>,
}
#[derive(Debug, Clone)]
pub struct EnumerationRestriction {
    pub(crate) source: DatatypeSource,
    pub(crate) vocabulary_source: SchemaDeclarationNode,
    pub(crate) constants: Vec<PreparedConstant>,
    pub(crate) equality: BoundEquality,
    pub(crate) interpreter: BoundInterpreter,
}
impl EnumerationRestriction {
    pub fn source(&self) -> &DatatypeSource {
        &self.source
    }
    pub fn constants(&self) -> &[PreparedConstant] {
        &self.constants
    }
    pub fn vocabulary_source(&self) -> &SchemaDeclarationNode {
        &self.vocabulary_source
    }
    pub fn equality(&self) -> &ScalarCapabilityIdentity {
        self.equality.registration.identity()
    }
    pub fn interpreter(&self) -> &ScalarCapabilityIdentity {
        self.interpreter.registration.identity()
    }
}
#[derive(Debug, Clone)]
pub struct EnumerationValidation {
    pub source: SchemaDeclarationNode,
    pub accepted: bool,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Debug, Clone, Copy)]
pub struct ConstantPreparationLimits {
    pub max_constants: usize,
    pub max_lexical_bytes: usize,
    /// Aggregate immutable output text for the explicit retained-literal form.
    pub max_retained_value_bytes: usize,
    /// Cumulative across all constants and roots in a compilation.
    pub validation: ValidationLimits,
}
impl Default for ConstantPreparationLimits {
    fn default() -> Self {
        Self {
            max_constants: 4096,
            max_lexical_bytes: 1_048_576,
            max_retained_value_bytes: 1_048_576,
            validation: ValidationLimits::default(),
        }
    }
}
pub(crate) fn attribute_diagnostics(
    diagnostics: &mut [Diagnostic],
    attribution: &DiagnosticAttribution,
    runtime: &ValidationRuntime<'_>,
) -> Result<(), ValidationStopReason> {
    for (i, diagnostic) in diagnostics.iter_mut().enumerate() {
        if i % 64 == 0 {
            runtime
                .control
                .check_scope(runtime.scope)
                .map_err(ValidationStopReason::Control)?;
        }
        if diagnostic.uri.is_none() && diagnostic.node.is_none() && diagnostic.source_map.is_none()
        {
            attribution.apply(diagnostic);
        }
    }
    Ok(())
}
impl EnumerationRestriction {
    pub(crate) fn validate(
        &self,
        value: &Item,
        runtime: &ValidationRuntime<'_>,
        attribution: &DiagnosticAttribution,
        comparisons: &mut usize,
        diagnostics: &mut usize,
        output_diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<EnumerationValidation, ValidationStopReason> {
        let start = output_diagnostics.len();
        for constant in &self.constants {
            runtime
                .control
                .check_scope(runtime.scope)
                .map_err(ValidationStopReason::Control)?;
            if *comparisons == 0 {
                return Err(ValidationStopReason::Limit("comparisons"));
            }
            *comparisons -= 1;
            let result = self
                .equality
                .registration
                .implementation
                .compare(EqualityCall {
                    left: value,
                    right: &constant.value,
                    datatype: &self.equality.datatype,
                    runtime,
                    fallback: attribution,
                    limits: ValidationLimits {
                        max_diagnostics: *diagnostics,
                        max_comparisons: *comparisons,
                        max_input_values: 2,
                        max_rules: 0,
                    },
                });
            runtime
                .control
                .check_scope(runtime.scope)
                .map_err(ValidationStopReason::Control)?;
            if let Some(failure) = runtime.query_failure() {
                return Err(ValidationStopReason::Result(crate::datatype_results::DatatypeResultError::Execution(failure)));
            }
            let (equal, mut details, state) = match result {
                EqualityExecution::Complete { equal, diagnostics } => (Some(equal), diagnostics, 0),
                EqualityExecution::Pending(d) => (None, d, 1),
                EqualityExecution::Unavailable(d) => (None, d, 2),
                EqualityExecution::Failed(d) => (None, d, 3),
            };
            if details.len() > *diagnostics {
                return Err(ValidationStopReason::Limit("diagnostics"));
            }
            *diagnostics -= details.len();
            attribute_diagnostics(&mut details, attribution, runtime)?;
            output_diagnostics.extend(details.iter().cloned());
            match state {
                1 => return Err(ValidationStopReason::Pending(details)),
                2 => return Err(ValidationStopReason::Unavailable(details)),
                3 => return Err(ValidationStopReason::Failed(details)),
                _ => {}
            }
            if equal == Some(true) {
                return Ok(EnumerationValidation {
                    source: self.vocabulary_source.clone(),
                    accepted: true,
                    diagnostics: output_diagnostics[start..].to_vec(),
                });
            }
        }
        Ok(EnumerationValidation {
            source: self.vocabulary_source.clone(),
            accepted: false,
            diagnostics: output_diagnostics[start..].to_vec(),
        })
    }
}
