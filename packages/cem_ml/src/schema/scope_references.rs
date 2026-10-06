//! Original-source target admission for the schema scope reference consumer.

use super::{declaration_references::SchemaDeclarationNode, registry::CEM_SCHEMA_URI};
use crate::parser::{CemAstNode, ExpandedName};

const CORE_NAMESPACE: &str = "https://cem.dev/ns/core/1";

/// Admission retains both the selected wrapper (if any) and the exact
/// declaration in its original arena. It is not a compiled/ready schema and
/// does not authorize a relationship crossing or establish an effective scope.
#[derive(Debug, Clone)]
pub struct SchemaScopeTarget {
    pub selected: SchemaDeclarationNode,
    pub declaration: SchemaDeclarationNode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaScopeTargetError {
    InvalidKindOrName,
    NameNotReady,
    DeclarationCount(usize),
}

/// Apply the adopted schema target-shape contract to an original source handle.
/// Runtime selection/cardinality, bounded chains, scope grants and dependency
/// readiness belong to the consuming lifecycle stage, before scope activation.
/// `resolved_name` supplies owner-checked, source-position expanded names. CEM
/// builder prefixes are lexical; absent metadata is pending, never a guessed URI.
pub fn admit_schema_scope_target<F>(
    selected: SchemaDeclarationNode,
    mut resolved_name: F,
) -> Result<SchemaScopeTarget, SchemaScopeTargetError>
where
    F: FnMut(&SchemaDeclarationNode) -> Option<ExpandedName>,
{
    if !matches!(selected.node(), CemAstNode::Element { .. }) {
        return Err(SchemaScopeTargetError::InvalidKindOrName);
    }
    let name = resolved_name(&selected).ok_or(SchemaScopeTargetError::NameNotReady)?;
    if is_schema_declaration(&name) {
        return Ok(SchemaScopeTarget {
            declaration: selected.clone(),
            selected,
        });
    }
    if name.namespace_uri != CORE_NAMESPACE || name.local_name != "schema" {
        return Err(SchemaScopeTargetError::InvalidKindOrName);
    }
    let CemAstNode::Element {
        attributes,
        children,
        ..
    } = selected.node()
    else {
        unreachable!()
    };
    let mut named = false;
    for id in attributes {
        let attribute = SchemaDeclarationNode::new(selected.document().clone(), *id)
            .ok_or(SchemaScopeTargetError::InvalidKindOrName)?;
        let name = resolved_name(&attribute).ok_or(SchemaScopeTargetError::NameNotReady)?;
        if name.namespace_uri == CORE_NAMESPACE && name.local_name == "name" {
            named = matches!(attribute.node(), CemAstNode::Attribute { value: Some(value), .. } if !value.trim().is_empty());
        }
    }
    if !named {
        return Err(SchemaScopeTargetError::InvalidKindOrName);
    }
    let mut declarations = Vec::new();
    for id in children {
        let child = SchemaDeclarationNode::new(selected.document().clone(), *id)
            .ok_or(SchemaScopeTargetError::InvalidKindOrName)?;
        if matches!(child.node(), CemAstNode::Element { .. }) {
            let name = resolved_name(&child).ok_or(SchemaScopeTargetError::NameNotReady)?;
            if is_schema_declaration(&name) {
                declarations.push(child);
            }
        }
    }
    if declarations.len() != 1 {
        return Err(SchemaScopeTargetError::DeclarationCount(declarations.len()));
    }
    Ok(SchemaScopeTarget {
        selected,
        declaration: declarations.remove(0),
    })
}

/// Compile the admitted declaration in its original arena. The host supplies
/// captured lexical bindings and bounded dependency evaluation. Callers must
/// check model readiness and hard compile diagnostics before activating a scope;
/// this function does not select a scope or fall back to an inherited schema.
pub fn compile_schema_scope_target<H: super::declaration_references::SchemaDeclarationHost>(
    schema_uri: &str,
    target: &SchemaScopeTarget,
    host: &mut H,
    limits: super::reference_traversal::ReferenceTraversalLimits,
) -> Result<
    super::document_model::SchemaDocumentModel,
    crate::value::reference_resolution::ReferenceResolutionError,
> {
    super::declaration_references::compile_selected_schema_with_declaration_references(
        schema_uri,
        &target.declaration,
        host,
        limits,
    )
}

fn is_schema_declaration(name: &ExpandedName) -> bool {
    name.namespace_uri == CEM_SCHEMA_URI && name.local_name == "schema"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn target(source: &str) -> SchemaDeclarationNode {
        let source = format!("<root xmlns:s='https://cem.dev/ns/schema/1' xmlns:c='https://cem.dev/ns/core/1' xmlns:r='https://cem.dev/ns/cem-ml/1'>{source}</root>");
        let tree = crate::import::import_data_bytes(
            source.as_bytes(),
            "application/xml",
            "cem",
            "source.xml",
        )
        .unwrap();
        let id = match tree.ast_owner().get(0).unwrap() {
            CemAstNode::Document { root_children, .. } => {
                match tree.ast_owner().get(root_children[0]).unwrap() {
                    CemAstNode::Element { children, .. } => children[0],
                    _ => panic!(),
                }
            }
            _ => panic!(),
        };
        SchemaDeclarationNode::new(tree.ast_owner().clone(), id).unwrap()
    }
    fn name(node: &SchemaDeclarationNode) -> Option<ExpandedName> {
        match node.node() {
            CemAstNode::Element { expanded_name, .. }
            | CemAstNode::Attribute { expanded_name, .. } => Some(expanded_name.clone()),
            _ => None,
        }
    }

    #[test]
    fn schema_scope_admission_retains_selected_owner_and_exact_declaration() {
        let direct = target("<s:schema name=\"direct\"/>");
        let admitted = admit_schema_scope_target(direct.clone(), name).unwrap();
        assert_eq!(admitted.selected.node_id(), direct.node_id());
        assert_eq!(admitted.declaration.node_id(), direct.node_id());
        assert!(Arc::ptr_eq(
            admitted.declaration.document(),
            direct.document()
        ));

        let wrapper = target(
            "<c:schema c:name=\"inline\"><s:schema name=\"chosen\"><s:elements><r:expr>#later</r:expr></s:elements></s:schema></c:schema>",
        );
        let admitted = admit_schema_scope_target(wrapper.clone(), name).unwrap();
        assert_eq!(admitted.selected.node_id(), wrapper.node_id());
        assert_ne!(admitted.declaration.node_id(), wrapper.node_id());
        assert!(Arc::ptr_eq(
            admitted.selected.document(),
            wrapper.document()
        ));
        assert!(Arc::ptr_eq(
            admitted.declaration.document(),
            wrapper.document()
        ));
        assert!(wrapper
            .document()
            .nodes
            .iter()
            .any(|node| matches!(node, CemAstNode::Reference { targets: None, .. })));
    }

    #[test]
    fn schema_scope_admission_rejects_wrong_kinds_and_ambiguous_wrappers() {
        for source in [
            "<schema/>",
            "<s:element/>",
            "<c:schema name='inline'><s:schema/></c:schema>",
            "<c:schema c:name=''><s:schema/></c:schema>",
        ] {
            assert_eq!(
                admit_schema_scope_target(target(source), name).unwrap_err(),
                SchemaScopeTargetError::InvalidKindOrName
            );
        }
        for (source, count) in [
            ("<c:schema c:name='inline'/>", 0),
            (
                "<c:schema c:name='inline'><s:schema/><s:schema/></c:schema>",
                2,
            ),
            (
                "<c:schema c:name='inline'><container><s:schema/></container></c:schema>",
                0,
            ),
            ("<c:schema c:name='inline'><schema/></c:schema>", 0),
        ] {
            assert_eq!(
                admit_schema_scope_target(target(source), name).unwrap_err(),
                SchemaScopeTargetError::DeclarationCount(count)
            );
        }
    }

    #[test]
    fn schema_scope_admission_requires_available_original_name_metadata() {
        let direct = target("<s:schema/>");
        assert_eq!(
            admit_schema_scope_target(direct, |_| None).unwrap_err(),
            SchemaScopeTargetError::NameNotReady
        );
        let wrapper = target("<c:schema c:name='inline'><s:schema/></c:schema>");
        let selected_id = wrapper.node_id();
        assert_eq!(
            admit_schema_scope_target(wrapper.clone(), |node| {
                (node.node_id() == selected_id)
                    .then(|| name(node))
                    .flatten()
            })
            .unwrap_err(),
            SchemaScopeTargetError::NameNotReady
        );
        assert_eq!(
            admit_schema_scope_target(wrapper, |node| {
                match node.node() {
                    CemAstNode::Element { .. } if node.node_id() != selected_id => None,
                    _ => name(node),
                }
            })
            .unwrap_err(),
            SchemaScopeTargetError::NameNotReady
        );
    }

    #[test]
    fn selected_schema_pending_slots_exclude_other_roots() {
        use crate::{
            events::cem::CemEventNormalizer,
            parser::builder::CemAstBuilder,
            source::{BytesSource, SourceId},
            tokenizer::cem::CemTokenizer,
        };
        let source = r#"{schema | {elements | {element @name="earlier" @base={#earlier-base}}} {attributes | {attribute @name="earlier" @type={#earlier-type}}}} {schema | {elements | {element @name="selected" @base={#selected-base}}} {attributes | {attribute @name="selected" @type={#selected-type}}}}"#;
        let tokenizer =
            CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
        let document = CemAstBuilder::new(CemEventNormalizer::new(tokenizer)).build();
        assert!(document.diagnostics.is_empty());
        let schema_ids: Vec<_> = document
            .nodes
            .iter()
            .filter_map(|node| match node {
                CemAstNode::Element {
                    node_id,
                    expanded_name,
                    ..
                } if expanded_name.local_name == "schema" => Some(*node_id),
                _ => None,
            })
            .collect();
        let model = super::super::document_model::compile_document_model_with_declarations(
            "selected",
            &document,
            Some(schema_ids[1]),
            &Default::default(),
            Default::default(),
        );
        assert!(!model.is_ready_for_validation());
        assert!(model.element("earlier").is_none());
        assert!(!model.attributes.contains_key("earlier"));
        assert!(model.attributes["selected"].native_type_pending);
        let expressions: Vec<_> = model
            .declaration_references
            .sites
            .iter()
            .map(|site| site.occurrence.expression.as_deref().unwrap())
            .collect();
        assert_eq!(expressions, vec!["#selected-base", "#selected-type"]);
        for site in &model.declaration_references.sites {
            let original = document.get(site.occurrence.node_id.unwrap()).unwrap();
            assert!(matches!(
                original,
                CemAstNode::Reference { targets: None, .. }
            ));
        }
    }
}
