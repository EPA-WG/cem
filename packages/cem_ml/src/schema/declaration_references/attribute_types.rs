//! Readiness protection for the adopted native attribute datatype slot.
//! Executable datatype consumption awaits its separate declaration contract.
use super::*;

pub(crate) fn is_pending_type(document: &CemDocument, declaration: AstNodeId) -> bool {
    match type_attribute(document, declaration) {
        Some(CemAstNode::Attribute { value_nodes, .. }) => !value_nodes.is_empty(),
        _ => false,
    }
}

fn type_attribute(document: &CemDocument, declaration: AstNodeId) -> Option<&CemAstNode> {
    let CemAstNode::Element {
        expanded_name,
        attributes,
        ..
    } = document.get(declaration)?
    else {
        return None;
    };
    if expanded_name.local_name != "attribute" {
        return None;
    }
    attributes
        .iter()
        .rev()
        .find_map(|id| match document.get(*id) {
            Some(node @ CemAstNode::Attribute { expanded_name, .. })
                if expanded_name.local_name == "type" =>
            {
                Some(node)
            }
            _ => None,
        })
}

pub(crate) fn retain_attribute_type(
    schema_uri: &str,
    document: &CemDocument,
    declaration: AstNodeId,
    compilation: &mut DeclarationReferenceCompilation,
) {
    // Match scalar attribute compilation's last-authored-slot precedence.
    let Some(attribute) = type_attribute(document, declaration) else {
        return;
    };
    let CemAstNode::Attribute {
        node_id,
        value_nodes,
        ..
    } = attribute
    else {
        unreachable!()
    };
    if value_nodes.is_empty() {
        return;
    }
    if let [reference] = value_nodes.as_slice() {
        if let Some(node @ CemAstNode::Reference { .. }) = document.get(*reference) {
            compilation.retain_pending(schema_uri, node, SchemaDeclarationKind::AttributeType);
            return;
        }
    }
    let occurrence = ReferenceOccurrence {
        identity: format!("native-attribute-type:{schema_uri}:{node_id}"),
        node_id: Some(*node_id),
        expression: None,
        source_map: document_model::source_stack_for_node(attribute).clone(),
    };
    let diagnostic = invalid_target(
        schema_uri,
        &occurrence,
        None,
        "Attribute @type requires one explicit native reference constructor",
    );
    compilation.sites.push(DeclarationReferenceSite {
        kind: SchemaDeclarationKind::AttributeType,
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

pub(crate) fn retain_authored_types(
    schema_uri: &str,
    document: &CemDocument,
    compilation: &mut DeclarationReferenceCompilation,
) {
    let Some(CemAstNode::Element { children, .. }) = document
        .nodes
        .iter()
        .find(|node| is_element(node, "schema"))
    else {
        return;
    };
    for collection in children {
        let Some(node @ CemAstNode::Element { children, .. }) = document.get(*collection) else {
            continue;
        };
        if !is_element(node, "attributes") {
            continue;
        }
        for declaration in children {
            retain_attribute_type(schema_uri, document, *declaration, compilation);
        }
    }
}
