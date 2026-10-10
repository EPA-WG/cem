//! Literal and captured typed directives over original payloads, without CEM
//! reparsing or synthetic attribute/reference nodes. Their admission stays distinct.
use super::*;
use crate::schema::{
    machine::{
        parse_directive_attrs, parse_schema_source_body, SchemaDirectiveError, SchemaElementForm,
    },
    scoping::SchemaSource,
};

pub(super) fn validate_typed(
    host: SchemaDeclarationNode,
    captured: &crate::schema::machine::LexicallyScopedDocument,
) -> SchemaHostControlContract {
    use crate::{
        schema::prelude_values::decode_native_prelude_value, tokenizer::cem::TypedPreludeRole,
    };
    let decoded = decode_native_prelude_value(host.clone(), captured);
    let (control, issue) = match decoded {
        Ok(slot) if slot.role() == TypedPreludeRole::SchemaSelector => (
            Some(SchemaHostControl {
                host: host.clone(),
                attribute: host.clone(),
                source: SchemaHostSource::NativeSelector(slot.value().clone()),
            }),
            None,
        ),
        _ => (
            None,
            Some(SchemaHostControlError {
                source: host.clone(),
                issue: SchemaHostControlIssue::InvalidValue,
            }),
        ),
    };
    SchemaHostControlContract {
        host,
        attributes: vec![],
        pending_attributes: vec![],
        control,
        issue,
        extent: SchemaScopeControlExtent::Following,
    }
}

pub(super) fn validate(
    host: SchemaDeclarationNode,
    form: Option<SchemaElementForm>,
) -> SchemaHostControlContract {
    let mut contract = SchemaHostControlContract {
        host: host.clone(),
        attributes: vec![],
        pending_attributes: vec![],
        control: None,
        issue: None,
        extent: SchemaScopeControlExtent::Following,
    };
    let decoded = decode(&host, form);
    match decoded {
        Ok(control) => contract.control = Some(control),
        Err(issue) => contract.issue = Some(issue),
    }
    contract
}

fn decode(
    host: &SchemaDeclarationNode,
    form: Option<SchemaElementForm>,
) -> Result<SchemaHostControl, SchemaHostControlError> {
    let error = |source, issue| SchemaHostControlError { source, issue };
    if form != Some(SchemaElementForm::Prelude) {
        return Err(error(
            host.clone(),
            SchemaHostControlIssue::BodyFormNotReady,
        ));
    }
    let CemAstNode::Element {
        attributes,
        children,
        ..
    } = host.node()
    else {
        return Err(error(host.clone(), SchemaHostControlIssue::InvalidHost));
    };
    if !attributes.is_empty() {
        return Err(error(host.clone(), SchemaHostControlIssue::InvalidValue));
    }
    let mut body = String::new();
    let mut payload = host.clone();
    for id in children {
        let node = SchemaDeclarationNode::new(host.document().clone(), *id)
            .ok_or_else(|| error(host.clone(), SchemaHostControlIssue::InvalidValue))?;
        match node.node() {
            CemAstNode::Text { data, .. } => {
                if body.is_empty() {
                    payload = node.clone();
                } else {
                    // Match the existing machine's fragmented directive values.
                    body.push(' ');
                }
                body.push_str(data);
            }
            CemAstNode::Whitespace { .. } => body.push(' '),
            _ => return Err(error(node, SchemaHostControlIssue::InvalidValue)),
        }
    }
    // The legacy source decoder accepts the bare URI shorthand and quoted
    // selectors. The shared consumer also rejects repeated source declarations,
    // consistently with element controls, without changing lexical capture.
    if parse_directive_attrs(&body)
        .iter()
        .filter(|(name, _)| matches!(name.as_str(), "src" | "select"))
        .count()
        > 1
    {
        return Err(error(payload, SchemaHostControlIssue::ConflictingSources));
    }
    let source = match parse_schema_source_body(&body) {
        Ok(SchemaSource::Uri(value)) if !value.trim().is_empty() => SchemaHostSource::Uri(value),
        Ok(SchemaSource::Select(value)) if !value.trim().is_empty() => {
            SchemaHostSource::LiteralSelector(value)
        }
        Err(SchemaDirectiveError::ExclusiveSrcSelect) => {
            return Err(error(payload, SchemaHostControlIssue::ConflictingSources))
        }
        _ => return Err(error(payload, SchemaHostControlIssue::InvalidValue)),
    };
    Ok(SchemaHostControl {
        host: host.clone(),
        attribute: payload,
        source,
    })
}
