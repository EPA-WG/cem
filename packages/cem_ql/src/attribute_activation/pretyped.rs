//! Private engine issuer for the core's opaque, process-local handoff.
use super::*;
use crate::{
    eval::Item,
    preparation_evidence::{
        AttributePreparationInvocation, SealedPreparationEvidence, MAX_EVIDENCE_METADATA_BYTES,
    },
};
use cem_ml::{
    schema::attribute_datatypes::{
        AttributeDatatypeContextLease, AttributeDatatypePreparation, NativeAttributePreparation,
    },
    source_map::SourceMapStack,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Weak,
};

#[derive(Debug)]
struct InputSnapshot {
    tree: Option<Arc<cem_ml::parser::tree::RetainedCemTree>>,
    node: cem_ml::parser::AstNodeId,
    name: cem_ml::parser::ExpandedName,
    source: SourceMapStack,
    has_value: bool,
    element: String,
    values: BTreeMap<String, String>,
    context: AttributeDatatypeContextLease,
    control: cem_ml::operation_control::OperationControl,
}
impl InputSnapshot {
    fn capture(input: &AttributeDatatypeInput<'_>, text: &str) -> Option<Self> {
        let context = input.context?;
        let lease = context.lease();
        if !lease.matches(context) || input.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err() {
            return None;
        }
        let CemAstNode::Attribute {
            node_id,
            expanded_name,
            value,
            value_nodes,
            source,
            ..
        } = input.source
        else {
            return None;
        };
        if !value_nodes.is_empty() || value.as_deref().unwrap_or("") != text {
            return None;
        }
        if let Some(tree) = &input.source_tree {
            if !tree
                .ast()
                .get(*node_id)
                .is_some_and(|node| std::ptr::eq(node, input.source))
            {
                return None;
            }
        }
        let mut bytes = input
            .element_name
            .len()
            .saturating_add(expanded_name.local_name.len())
            .saturating_add(expanded_name.namespace_uri.len());
        for (index, (key, value)) in input.attribute_values.iter().enumerate() {
            if index % 64 == 0 && input.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err() {
                return None;
            }
            bytes = bytes
                .saturating_add(std::mem::size_of::<(String, String)>())
                .saturating_add(key.len())
                .saturating_add(value.len());
            if bytes > MAX_EVIDENCE_METADATA_BYTES {
                return None;
            }
        }
        if bytes > MAX_EVIDENCE_METADATA_BYTES {
            return None;
        }
        // Source metadata is bounded by AttributePreparationInvocation before
        // this snapshot is captured; retain the original owner, never rebuild it.
        Some(Self {
            tree: input.source_tree.clone(),
            node: *node_id,
            name: expanded_name.clone(),
            source: source.clone(),
            has_value: value.is_some(),
            element: input.element_name.into(),
            values: input.attribute_values.clone(),
            context: lease,
            control: input.control.clone(),
        })
    }
    fn matches(
        &self,
        input: &AttributeDatatypeInput<'_>,
        evidence: &SealedPreparationEvidence,
    ) -> bool {
        let Some(context) = input.context else {
            return false;
        };
        if !self.context.matches(context)
            || !self.control.same_operation(input.control)
            || input.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
            || self.element != input.element_name
            || &self.values != input.attribute_values
        {
            return false;
        }
        match (&self.tree, &input.source_tree) {
            (Some(original), Some(current))
                if Arc::ptr_eq(original, current)
                    && current
                        .ast()
                        .get(self.node)
                        .is_some_and(|node| std::ptr::eq(node, input.source)) => {}
            (None, None) => {}
            _ => return false,
        }
        matches!(input.source, CemAstNode::Attribute {node_id,expanded_name,value,value_nodes,source,..}
            if *node_id==self.node && expanded_name.local_name==self.name.local_name && expanded_name.namespace_uri==self.name.namespace_uri && expanded_name.schema_id==self.name.schema_id && source==&self.source && value_nodes.is_empty()
            && value.is_some()==self.has_value && value.as_deref().unwrap_or("")==evidence.lexical().text.as_ref())
    }
}
#[derive(Debug)]
struct Prepared {
    invocation: AttributePreparationInvocation,
    evidence: SealedPreparationEvidence,
    input: InputSnapshot,
    publication: Weak<AtomicBool>,
}
impl Prepared {
    fn current(&self, input: &AttributeDatatypeInput<'_>) -> bool {
        self.publication
            .upgrade()
            .is_some_and(|state| state.load(Ordering::Acquire))
            && self.input.matches(input, &self.evidence)
            && self.evidence.verify(&self.invocation).is_ok()
    }
}
fn failed(input: &AttributeDatatypeInput<'_>, message: &str) -> AttributeDatatypePreparation {
    AttributeDatatypePreparation {
        diagnostics: incomplete_input(input, message).diagnostics,
        ..Default::default()
    }
}
pub(super) fn incomplete_input(
    input: &AttributeDatatypeInput<'_>,
    message: &str,
) -> AttributeDatatypeValidation {
    let source = match input.source {
        CemAstNode::Attribute { source, .. }
        | CemAstNode::Element { source, .. }
        | CemAstNode::Reference { source, .. } => Some(source),
        _ => None,
    };
    if source.is_some_and(|source| {
        crate::preparation_evidence::check_attribute_source_metadata(source, input.control).is_err()
    }) {
        return AttributeDatatypeValidation {
            accepted: None,
            diagnostics: vec![Diagnostic {
                code: "cem.schema_validation.attribute_datatype_incomplete".into(),
                severity: Severity::Error,
                message: message.into(),
                ..Default::default()
            }],
        };
    }
    incomplete(input.source, message)
}
pub(super) fn prepare(
    active: &ActiveAttributeDatatype,
    input: &AttributeDatatypeInput<'_>,
) -> AttributeDatatypePreparation {
    let AttributeDatatypeValue::Lexical(text) = &input.value else {
        return failed(input, "Original lexical input is required for preparation");
    };
    if !active.publication.load(Ordering::Acquire) {
        return failed(input, "Attribute preparation publication has expired");
    }
    let CemAstNode::Attribute {
        node_id,
        source,
        value,
        value_nodes,
        ..
    } = input.source
    else {
        return failed(input, "Original attribute input is required");
    };
    if !value_nodes.is_empty() || value.as_deref().unwrap_or("") != *text {
        return failed(input, "Attribute lexical input does not match its source");
    }
    if text.len() > active.limits.preparation.max_lexical_bytes
        || crate::preparation_evidence::check_attribute_source_metadata(source, input.control)
            .is_err()
    {
        return failed(input, "Attribute preparation source exceeds limits");
    }
    if input.source_tree.as_ref().is_some_and(|tree| {
        !tree
            .ast()
            .get(*node_id)
            .is_some_and(|node| std::ptr::eq(node, input.source))
    }) {
        return failed(input, "Attribute source owner does not match");
    }
    let candidate = input
        .source_tree
        .as_ref()
        .and_then(|tree| RetainedCemNode::new(tree.clone(), *node_id))
        .map(|node| Item::native(AttributeCandidate(node.query_item())))
        .into_iter()
        .collect();
    let original = PreparationInput {
        lexical: LexicalInput::new(Arc::from(*text), source.clone()),
        candidate,
        fallback: DiagnosticAttribution {
            source_map: Some(source.clone()),
            ..Default::default()
        },
    };
    let runtime = ValidationRuntime {
        control: input.control,
        scope: ROOT_EXECUTION_SCOPE_ID,
        query: Default::default(),
    };
    let Ok(invocation) = AttributePreparationInvocation::new(
        &active.facets,
        original,
        &runtime,
        active.limits.preparation,
    ) else {
        return failed(input, "Attribute preparation input is incomplete");
    };
    let Some(snapshot) = InputSnapshot::capture(input, text) else {
        return failed(
            input,
            "Attribute preparation context is unavailable or exceeds limits",
        );
    };
    let result = invocation.prepare();
    let Some(evidence) = result.evidence else {
        let mut diagnostics = result.report.diagnostics;
        diagnostics.push(diagnostic(
            input.source,
            if result.report.accepted == Some(false) {
                "cem.schema_validation.attribute_datatype_invalid"
            } else {
                "cem.schema_validation.attribute_datatype_incomplete"
            },
            "Attribute lexical preparation did not complete successfully",
        ));
        return AttributeDatatypePreparation {
            handle: None,
            accepted: result.report.accepted,
            diagnostics,
        };
    };
    let prepared = Prepared {
        invocation,
        evidence,
        input: snapshot,
        publication: Arc::downgrade(&active.publication),
    };
    if !prepared.current(input) {
        prepared.invocation.close();
        return failed(
            input,
            "Attribute preparation context expired during execution",
        );
    }
    AttributeDatatypePreparation {
        handle: Some(NativeAttributePreparation::new(prepared)),
        accepted: None,
        diagnostics: result.report.diagnostics,
    }
}
pub(super) fn validate(
    active: &ActiveAttributeDatatype,
    handle: &NativeAttributePreparation,
    input: &AttributeDatatypeInput<'_>,
) -> AttributeDatatypeValidation {
    let Some(prepared) = handle.downcast_ref::<Prepared>() else {
        return incomplete_input(input, "Native preparation issuer is unavailable");
    };
    if !prepared.evidence.matches_binding(&active.facets) {
        return incomplete_input(
            input,
            "Native preparation belongs to a different attribute binding",
        );
    }
    if !prepared.current(input) {
        prepared.invocation.close();
        return incomplete_input(
            input,
            "Native preparation input, context or publication has expired",
        );
    }
    let report = active.facets.validate_pretyped(
        &prepared.invocation,
        Some(&prepared.evidence),
        FacetContext {
            element_name: input.element_name,
            source: input.source,
            diagnostic_behaviors: &active.diagnostics.behaviors,
            attribute_values: input.attribute_values,
        },
        active.limits.max_facet_model_bytes,
    );
    if !prepared.current(input) {
        prepared.invocation.close();
        return incomplete_input(input, "Native preparation expired during validation");
    }
    export_validation(&report, input.source)
}
