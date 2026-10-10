//! Source-only native function sites stay inactive until an executable consumer
//! supplies a checked binding. Passive function selection alone never clears them.
use super::*;

pub(crate) fn retain(
    schema_uri: &str,
    document: &crate::parser::document::CemDocument,
    declaration: AstNodeId,
    compilation: &mut DeclarationReferenceCompilation,
) {
    let Some(CemAstNode::Element {
        expanded_name,
        attributes,
        ..
    }) = document.get(declaration)
    else {
        return;
    };
    if expanded_name.local_name != "behavior" {
        return;
    }
    let slots: Vec<_> = attributes
        .iter()
        .filter_map(|id| match document.get(*id) {
            Some(node @ CemAstNode::Attribute { expanded_name, .. })
                if expanded_name.local_name == "function" =>
            {
                Some(node)
            }
            _ => None,
        })
        .collect();
    if !slots.iter().any(
        |slot| matches!(slot, CemAstNode::Attribute { value_nodes, .. } if !value_nodes.is_empty()),
    ) {
        return;
    }
    let slot = slots[0];
    if slots.len() == 1 {
        if let CemAstNode::Attribute {
            value_nodes, value, ..
        } = slot
        {
            if value.as_deref().is_none_or(str::is_empty) {
                if let [reference] = value_nodes.as_slice() {
                    if let Some(node @ CemAstNode::Reference { .. }) = document.get(*reference) {
                        compilation.retain_pending(
                            schema_uri,
                            node,
                            SchemaDeclarationKind::BehaviorFunction,
                        );
                        let occurrence = &compilation.sites.last().unwrap().occurrence;
                        compilation.function_callers.insert(
                            occurrence.identity.clone(),
                            format!("schema-node:{document:p}:{declaration}"),
                        );
                        return;
                    }
                }
            }
        }
    }
    let occurrence = ReferenceOccurrence {
        identity: format!("native-function:{schema_uri}:{declaration}"),
        node_id: Some(declaration),
        expression: None,
        source_map: document_model::source_stack_for_node(slot).clone(),
    };
    let diagnostic = invalid_target(
        schema_uri,
        &occurrence,
        None,
        "Behavior @function requires one explicit native reference constructor in one binding slot",
    );
    compilation.sites.push(DeclarationReferenceSite {
        kind: SchemaDeclarationKind::BehaviorFunction,
        occurrence,
        resolution: Some(DeclarationReferenceResolution {
            nodes: vec![],
            state: ReferenceResolutionState::Invalid,
            failed: true,
            issues: vec![],
            diagnostics: vec![diagnostic],
            work_used: 0,
        }),
    });
}
pub(crate) fn retain_authored(
    schema_uri: &str,
    document: &crate::parser::document::CemDocument,
    schema_id: Option<AstNodeId>,
    compilation: &mut DeclarationReferenceCompilation,
) {
    let Some(CemAstNode::Element { children, .. }) = schema_id.and_then(|id| document.get(id))
    else {
        return;
    };
    for id in children {
        if let Some(CemAstNode::Element {
            expanded_name,
            children,
            ..
        }) = document.get(*id)
        {
            if expanded_name.local_name == "behaviors" {
                for declaration in children {
                    retain(schema_uri, document, *declaration, compilation);
                }
            }
        }
    }
}
