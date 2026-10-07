//! Element base references are a schema compilation consumer, not a parser action.
use super::*;

pub(crate) fn native_base_attribute(
    document: &CemDocument,
    declaration: AstNodeId,
) -> Option<AstNodeId> {
    let CemAstNode::Element { attributes, .. } = document.get(declaration)? else {
        return None;
    };
    let id = attributes.iter().rev().find(|id| matches!(document.get(**id), Some(CemAstNode::Attribute { expanded_name, .. }) if expanded_name.local_name == "base"))?;
    matches!(document.get(*id), Some(CemAstNode::Attribute { value_nodes, .. }) if !value_nodes.is_empty()).then_some(*id)
}
pub(crate) fn authored_bases(
    document: &CemDocument,
    schema_id: Option<AstNodeId>,
) -> Vec<(AstNodeId, AstNodeId)> {
    let mut declarations = vec![];
    let Some(CemAstNode::Element { children, .. }) = schema_id.and_then(|id| document.get(id))
    else {
        return declarations;
    };
    for collection in children {
        if !document
            .get(*collection)
            .is_some_and(|node| is_element(node, "elements"))
        {
            continue;
        }
        let CemAstNode::Element { children, .. } = document.get(*collection).unwrap() else {
            unreachable!()
        };
        for child in children {
            if document
                .get(*child)
                .is_some_and(|node| is_element(node, "element"))
            {
                if let Some(attribute) = native_base_attribute(document, *child) {
                    declarations.push((*child, attribute));
                }
            }
        }
    }
    declarations
}
fn source_reference(document: &CemDocument, attribute: AstNodeId) -> Option<AstNodeId> {
    let CemAstNode::Attribute { value_nodes, .. } = document.get(attribute)? else {
        return None;
    };
    let [id] = value_nodes.as_slice() else {
        return None;
    };
    matches!(document.get(*id), Some(CemAstNode::Reference { .. })).then_some(*id)
}
fn malformed_site(
    schema_uri: &str,
    document: &CemDocument,
    attribute: AstNodeId,
) -> DeclarationReferenceSite {
    let occurrence = ReferenceOccurrence {
        identity: format!("native-base:{schema_uri}:{attribute}"),
        node_id: Some(attribute),
        expression: None,
        source_map: document_model::source_stack_for_node(document.get(attribute).unwrap()).clone(),
    };
    let diagnostic = invalid_target(
        schema_uri,
        &occurrence,
        None,
        "Element @base requires one explicit native reference constructor",
    );
    DeclarationReferenceSite {
        kind: SchemaDeclarationKind::ElementBase,
        occurrence,
        resolution: Some(DeclarationReferenceResolution {
            nodes: vec![],
            state: ReferenceResolutionState::Invalid,
            failed: true,
            issues: vec![],
            diagnostics: vec![diagnostic],
            work_used: 0,
        }),
    }
}
pub(crate) fn retain_pending_bases(
    schema_uri: &str,
    document: &CemDocument,
    schema_id: Option<AstNodeId>,
    declarations: &BTreeMap<AstNodeId, Vec<CompiledSchemaDeclaration>>,
    compilation: &mut DeclarationReferenceCompilation,
) {
    for (declaration, attribute) in authored_bases(document, schema_id) {
        if declarations.contains_key(&declaration) {
            continue;
        }
        if let Some(id) = source_reference(document, attribute) {
            compilation.retain_pending(
                schema_uri,
                document.get(id).unwrap(),
                SchemaDeclarationKind::ElementBase,
            );
        } else {
            compilation
                .sites
                .push(malformed_site(schema_uri, document, attribute));
        }
    }
}

/// Resolve inheritance as one temporary forest of original declaration handles.
/// Nested bases share the enclosing request's active links and scope budgets.
pub(crate) fn compile<H: SchemaDeclarationHost>(
    schema_uri: &str,
    root: H::Node,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    origin: ReferenceOccurrence,
    owning: bool,
    seen: &mut BTreeSet<String>,
    name_work: &mut usize,
    name_issues: &mut Vec<DeclarationNameIssue>,
) -> Result<
    (
        Vec<CompiledSchemaDeclaration>,
        DeclarationReferenceResolution,
    ),
    ReferenceResolutionError,
> {
    use crate::value::reference_resolution::{
        resolve_consumer_structure, resolve_reference_structure,
    };
    let initial_issues = name_issues.len();
    let mut blocked = BTreeSet::new();
    let children = |host: &H, value: &H::Node| {
        let target = host.declaration_node(value)?;
        if let Err(issue) = names::check(&target, host, limits, name_work) {
            blocked.insert(target.identity());
            name_issues.push(issue);
            return None;
        }
        if !is_element(target.node(), "element") {
            return None;
        }
        let attribute = native_base_attribute(target.document(), target.node_id())?;
        let reference = source_reference(target.document(), attribute)?;
        Some(vec![host.source_reference(
            SchemaDeclarationNode::new(target.document().clone(), reference).unwrap(),
        )])
    };
    let walk = if owning {
        resolve_consumer_structure(root, host, limits, Some(origin.clone()), children)?
    } else {
        resolve_reference_structure(root, host, limits, children)?
    };
    let mut result = DeclarationReferenceResolution {
        nodes: walk
            .resolution
            .nodes
            .iter()
            .filter_map(|value| host.declaration_node(value))
            .collect(),
        state: walk.resolution.state,
        failed: walk.resolution.failed,
        issues: walk
            .resolution
            .issues
            .into_iter()
            .map(|issue| DeclarationReferenceIssue {
                occurrence: issue.occurrence,
                kind: issue.kind,
                reason: issue.reason,
            })
            .collect(),
        diagnostics: walk.resolution.diagnostics,
        work_used: walk.resolution.work_used,
    };
    if name_issues.len() != initial_issues {
        if result.state == ReferenceResolutionState::Resolved {
            result.state = ReferenceResolutionState::Pending;
        }
    }
    let mut models = vec![None; walk.resolution.nodes.len()];
    for index in (0..models.len()).rev() {
        let target = host.declaration_node(&walk.resolution.nodes[index]);
        let mut occurrence = walk.origins[index].clone();
        let mut error = None;
        if let Some(target) = target.as_ref() {
            if blocked.contains(&target.identity()) {
                continue;
            }
            let lexical = host.declaration_schema(target);
            let aliases = match lexical {
                Some(schema)
                    if Arc::ptr_eq(schema.document(), target.document())
                        && is_element(schema.node(), "schema") =>
                {
                    Some(document_model::collect_schema_uses(
                        schema.document(),
                        schema.node_id(),
                    ))
                }
                None => Some(BTreeMap::new()),
                _ => {
                    error = Some("Invalid declaring lexical schema owner or kind");
                    None
                }
            };
            let native = native_base_attribute(target.document(), target.node_id());
            if native
                .is_some_and(|attribute| source_reference(target.document(), attribute).is_none())
            {
                occurrence =
                    malformed_site(schema_uri, target.document(), native.unwrap()).occurrence;
                error = Some("Element @base requires one explicit native reference constructor");
            } else if native.is_some()
                && walk.children_complete[index]
                && walk.children[index].len() != 1
            {
                let reference = source_reference(target.document(), native.unwrap()).unwrap();
                let source = host.source_reference(
                    SchemaDeclarationNode::new(target.document().clone(), reference).unwrap(),
                );
                occurrence = host.reference_occurrence(&source).unwrap_or(occurrence);
                error = Some("Element @base must select exactly one named element declaration");
            }
            if let Some(aliases) = aliases {
                let base = walk.children[index]
                    .first()
                    .and_then(|child| models[*child].as_ref());
                if is_element(target.node(), "element") {
                    models[index] = document_model::compile_element_model_with_base(
                        target.document(),
                        target.node_id(),
                        &aliases,
                        seen,
                        base,
                    );
                }
                if models[index].is_none() {
                    error = Some("Expected a named element declaration");
                }
            }
        } else {
            error = Some("Expected a retained element declaration");
        }
        if let Some(error) = error {
            result.state = ReferenceResolutionState::Invalid;
            result.failed = true;
            result.diagnostics.push(invalid_target(
                schema_uri,
                &occurrence,
                target.as_ref(),
                error,
            ));
        }
    }
    let compiled = walk
        .roots
        .iter()
        .filter_map(|index| models[*index].take())
        .map(CompiledSchemaDeclaration::Element)
        .collect();
    Ok((compiled, result))
}

pub(crate) fn compile_authored<H: SchemaDeclarationHost>(
    schema_uri: &str,
    document: &Arc<CemDocument>,
    schema_id: Option<AstNodeId>,
    host: &mut H,
    limits: ReferenceTraversalLimits,
    seen: &mut BTreeSet<String>,
    declarations: &mut BTreeMap<AstNodeId, Vec<CompiledSchemaDeclaration>>,
    compilation: &mut DeclarationReferenceCompilation,
    name_work: &mut usize,
) -> Result<(), ReferenceResolutionError> {
    for (declaration, attribute) in authored_bases(document, schema_id) {
        let Some(reference) = source_reference(document, attribute) else {
            declarations.insert(declaration, vec![]);
            compilation
                .sites
                .push(malformed_site(schema_uri, document, attribute));
            continue;
        };
        let value =
            host.source_reference(SchemaDeclarationNode::new(document.clone(), reference).unwrap());
        let occurrence = host
            .reference_occurrence(&value)
            .ok_or(ReferenceResolutionError::NotReference)?;
        let root = host
            .source_reference(SchemaDeclarationNode::new(document.clone(), declaration).unwrap());
        let (models, resolution) = compile(
            schema_uri,
            root,
            host,
            limits,
            occurrence.clone(),
            true,
            seen,
            name_work,
            &mut compilation.name_issues,
        )?;
        declarations.insert(declaration, models);
        compilation.sites.push(DeclarationReferenceSite {
            kind: SchemaDeclarationKind::ElementBase,
            occurrence,
            resolution: Some(resolution),
        });
    }
    Ok(())
}
