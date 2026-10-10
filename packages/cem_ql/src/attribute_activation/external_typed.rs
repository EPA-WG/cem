//! Current core invocation checks for explicitly admitted external values.
use super::*;
use crate::external_typed::{ExternalTypedInput, ExternalTypedValue};
use cem_ml::schema::attribute_datatypes::{
    NativeAttributeTypedAdmission, NativeAttributeTypedValue,
};
use std::sync::atomic::Ordering;

pub(super) fn admission(active: &ActiveAttributeDatatype) -> Option<NativeAttributeTypedAdmission> {
    crate::external_typed::native_admission(&active.facets, &active.publication)
}
pub(super) fn validate(
    active: &ActiveAttributeDatatype,
    handle: &NativeAttributeTypedValue,
    input: &AttributeDatatypeInput<'_>,
) -> AttributeDatatypeValidation {
    let fail = || {
        pretyped::incomplete_input(
            input,
            "External typed value, input context or publication is unavailable",
        )
    };
    let Some(value) = handle.downcast_ref::<ExternalTypedValue>() else {
        return fail();
    };
    let Some(context) = input.context else {
        return fail();
    };
    let lease = context.lease();
    if !lease.matches(context)
        || !active.publication.load(Ordering::Acquire)
        || input.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
    {
        return fail();
    }
    let CemAstNode::Attribute {
        node_id,
        source,
        expanded_name,
        ..
    } = input.source
    else {
        return fail();
    };
    if crate::preparation_evidence::check_attribute_source_metadata(source, input.control).is_err()
    {
        return fail();
    }
    if input.source_tree.as_ref().is_some_and(|tree| {
        !tree
            .ast()
            .get(*node_id)
            .is_some_and(|n| std::ptr::eq(n, input.source))
    }) {
        return fail();
    }
    let mut bytes = input
        .element_name
        .len()
        .saturating_add(expanded_name.local_name.len())
        .saturating_add(expanded_name.namespace_uri.len());
    for (key, value) in input.attribute_values {
        bytes = bytes
            .saturating_add(std::mem::size_of::<(String, String)>())
            .saturating_add(key.len())
            .saturating_add(value.len());
        if bytes > crate::preparation_evidence::MAX_EVIDENCE_METADATA_BYTES
            || input.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
        {
            return fail();
        }
    }
    if bytes > crate::preparation_evidence::MAX_EVIDENCE_METADATA_BYTES {
        return fail();
    }
    let candidate = input
        .source_tree
        .as_ref()
        .and_then(|tree| RetainedCemNode::new(tree.clone(), *node_id))
        .map(|node| crate::eval::Item::native(AttributeCandidate(node.query_item())))
        .into_iter()
        .collect();
    let report = active.facets.validate_external(
        ExternalTypedInput {
            value: Some(value),
            candidate,
            fallback: DiagnosticAttribution {
                source_map: Some(source.clone()),
                ..Default::default()
            },
        },
        FacetContext {
            element_name: input.element_name,
            source: input.source,
            diagnostic_behaviors: &active.diagnostics.behaviors,
            attribute_values: input.attribute_values,
        },
        &ValidationRuntime {
            control: input.control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            query: Default::default(),
        },
        active.limits,
    );
    if !lease.matches(context)
        || !active.publication.load(Ordering::Acquire)
        || input.control.check_scope(ROOT_EXECUTION_SCOPE_ID).is_err()
    {
        return fail();
    }
    export_validation(&report, input.source)
}
