//! Executable attribute contracts installed by an explicit schema compiler.
//! The core owns invocation/provenance; registered engines own typed execution.
use super::{
    attribute_references::NativeAttributeTargetAccess,
    declaration_references::SchemaDeclarationNode,
};
use crate::{
    diagnostics::Diagnostic,
    operation_control::OperationControl,
    parser::{tree::RetainedCemTree, CemAstNode},
};
use std::{
    any::Any,
    collections::BTreeMap,
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};

/// One immutable host context/grant epoch. Advancing or closing it invalidates
/// every outstanding preparation, including clones held by another consumer.
#[derive(Debug, Clone)]
pub struct AttributeDatatypeContext(Arc<AtomicBool>);
impl Default for AttributeDatatypeContext {
    fn default() -> Self {
        Self(Arc::new(AtomicBool::new(true)))
    }
}
impl AttributeDatatypeContext {
    pub fn close(&self) {
        self.0.store(false, Ordering::Release);
    }
    pub fn advance(&mut self) {
        self.close();
        *self = Self::default();
    }
    pub fn lease(&self) -> AttributeDatatypeContextLease {
        AttributeDatatypeContextLease(Arc::downgrade(&self.0))
    }
}
#[derive(Debug, Clone)]
pub struct AttributeDatatypeContextLease(Weak<AtomicBool>);
impl AttributeDatatypeContextLease {
    pub fn matches(&self, current: &AttributeDatatypeContext) -> bool {
        self.0
            .upgrade()
            .is_some_and(|state| Arc::ptr_eq(&state, &current.0) && state.load(Ordering::Acquire))
    }
}

/// In-process retained engine payload. Contents confer no authority until the
/// registered engine checks its private concrete issuer and live invocation.
/// There is deliberately no serialization/reload implementation.
#[derive(Clone)]
pub struct NativeAttributePreparation(Arc<dyn Any + Send + Sync>);
impl Debug for NativeAttributePreparation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAttributePreparation(..)")
    }
}
impl NativeAttributePreparation {
    pub fn new<T: Any + Send + Sync>(payload: T) -> Self {
        Self(Arc::new(payload))
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}

pub enum AttributeDatatypeValue<'a> {
    Lexical(&'a str),
    Nodes(Arc<NativeAttributeTargetAccess>),
    Prepared(&'a NativeAttributePreparation),
    ExternalTyped(&'a NativeAttributeTypedValue),
}
/// An engine-issued typed-only admission capability. Generic payloads do not
/// establish authority; engines verify their private concrete issuer.
#[derive(Clone)]
pub struct NativeAttributeTypedAdmission(Arc<dyn Any + Send + Sync>);
impl Debug for NativeAttributeTypedAdmission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAttributeTypedAdmission(..)")
    }
}
impl NativeAttributeTypedAdmission {
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self(Arc::new(value))
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}
/// Immutable external data, carrying no lexical preparation or cached verdict.
#[derive(Clone)]
pub struct NativeAttributeTypedValue(Arc<dyn Any + Send + Sync>);
impl Debug for NativeAttributeTypedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NativeAttributeTypedValue(..)")
    }
}
impl NativeAttributeTypedValue {
    pub fn new<T: Any + Send + Sync>(value: T) -> Self {
        Self(Arc::new(value))
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}
pub struct AttributeDatatypeInput<'a> {
    pub value: AttributeDatatypeValue<'a>,
    pub source: &'a CemAstNode,
    pub source_tree: Option<Arc<RetainedCemTree>>,
    pub element_name: &'a str,
    pub attribute_values: &'a BTreeMap<String, String>,
    pub control: &'a OperationControl,
    /// Required for explicit native preparation/consumption. The host advances
    /// this epoch when external context roots or effective grants change.
    pub context: Option<&'a AttributeDatatypeContext>,
}
#[derive(Debug, Default)]
pub struct AttributeDatatypePreparation {
    pub handle: Option<NativeAttributePreparation>,
    /// Preparation can reject lexical input, but cannot establish acceptance.
    pub accepted: Option<bool>,
    pub diagnostics: Vec<Diagnostic>,
}
#[derive(Debug)]
pub struct AttributeDatatypeValidation {
    /// None preserves incomplete execution, distinct from completed rejection.
    pub accepted: Option<bool>,
    pub diagnostics: Vec<Diagnostic>,
}
pub trait CompiledAttributeDatatype: Debug + Send + Sync {
    fn typed_admission(&self) -> Option<NativeAttributeTypedAdmission> {
        None
    }
    fn declaration(&self) -> &SchemaDeclarationNode;
    fn is_node_valued(&self) -> bool;
    fn validate(&self, input: AttributeDatatypeInput<'_>) -> AttributeDatatypeValidation;
    fn prepare(&self, _input: AttributeDatatypeInput<'_>) -> AttributeDatatypePreparation {
        AttributeDatatypePreparation::default()
    }
    /// Called only when a successful publication retires this exact contract.
    fn retire_preparations(&self) {}
}
