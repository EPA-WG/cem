//! Native value containers shared by query evaluation and transformation output.
//! A reference owns its ordered targets, not copies of their retained subtrees.
use std::sync::Arc;
pub mod artifact;

#[derive(Debug)]
pub struct CemReference<T>(Arc<[T]>);

impl<T> Clone for CemReference<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> CemReference<T> {
    pub fn new(values: Vec<T>) -> Self {
        Self(values.into())
    }
    pub fn values(&self) -> &[T] {
        &self.0
    }
    /// Runtime identity of the reference, distinct from any target identity.
    pub fn identity(&self) -> String {
        format!("cem:reference:{:p}", Arc::as_ptr(&self.0))
    }
}

impl<T> PartialEq for CemReference<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<T> Eq for CemReference<T> {}

use crate::source_map::SourceMapStack;

/// Common constructed CEM content with host-native values and sequences.
/// References preserve source ownership; output structure is immutable once published.
#[derive(Debug, Clone, PartialEq)]
pub enum CemValueNode<T, S> {
    /// One output occurrence retaining immutable values without copying targets.
    Reference {
        reference: CemReference<T>,
        source_map: SourceMapStack,
    },
    Element {
        tag: String,
        namespace: Option<String>,
        /// Native expanded-name construction supplies its lexical QName explicitly.
        /// `None` retains the existing CEMT name/namespace contract.
        qualified_name: Option<String>,
        attributes: Vec<CemValueAttribute<S>>,
        children: Vec<CemValueNode<T, S>>,
        source_map: SourceMapStack,
    },
    Text {
        text: String,
        source_map: SourceMapStack,
    },
    Comment {
        text: String,
        source_map: SourceMapStack,
    },
    Cdata {
        text: String,
        source_map: SourceMapStack,
    },
    ProcessingInstruction {
        target: String,
        data: String,
        source_map: SourceMapStack,
    },
}

#[derive(Debug, Clone)]
pub struct CemValueAttribute<S> {
    pub name: String,
    pub namespace: Option<String>,
    pub qualified_name: Option<String>,
    pub value: String,
    /// Authoritative native value sequence. `value` is a compatibility text
    /// projection; native handoffs retain this sequence and its contract.
    pub value_stream: S,
    pub contract: Option<Arc<crate::schema::document_model::AttributeValueContract>>,
    pub source_map: SourceMapStack,
}

impl<S: PartialEq> PartialEq for CemValueAttribute<S> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.namespace == other.namespace
            && self.qualified_name == other.qualified_name
            && self.value == other.value
            && self.value_stream == other.value_stream
            && self.contract == other.contract
            && self.source_map == other.source_map
    }
}

impl<S: PartialEq> Eq for CemValueAttribute<S> {}

impl<T: PartialEq, S: PartialEq> Eq for CemValueNode<T, S> {}
