use super::*;
use cem_ml::schema::document_model::{
    compile_attribute_model, compile_schema_document_model_from_retained,
};
fn registered(
    host: &FreshStructuralHost,
    tree: &Arc<RetainedCemTree>,
    node: &SchemaDeclarationNode,
) -> bool {
    host.inner
        .source_tree(node)
        .is_some_and(|t| Arc::ptr_eq(t, tree))
        && host
            .inner
            .scope(&host.inner.source_reference(node.clone()))
            .is_some_and(|s| host.inner.registered_scope_validator_context(s).is_some())
}
fn original(
    tree: &Arc<RetainedCemTree>,
    id: AstNodeId,
    schema: &str,
) -> Result<SchemaDeclarationNode, String> {
    let node = SchemaDeclarationNode::new(tree.ast_owner().clone(), id).ok_or(REFUSED)?;
    let parent = tree
        .source_parent(id)
        .and_then(|p| tree.ast().get(p))
        .ok_or(REFUSED)?;
    let parent_id = tree.source_parent(id).ok_or(REFUSED)?;
    if !matches!(parent,CemAstNode::Element {expanded_name,..} if expanded_name.local_name=="attributes")
    {
        return Err(REFUSED.into());
    }
    let schema_node = tree
        .source_parent(parent_id)
        .and_then(|p| tree.ast().get(p))
        .ok_or(REFUSED)?;
    if !matches!(schema_node,CemAstNode::Element {expanded_name,attributes,..} if expanded_name.local_name=="schema" && attributes.iter().any(|a|matches!(tree.ast().get(*a),Some(CemAstNode::Attribute{expanded_name,value:Some(v),value_nodes,..}) if expanded_name.local_name=="namespace" && v==schema && value_nodes.is_empty())))
    {
        return Err(REFUSED.into());
    }
    Ok(node)
}
pub(super) fn local_scalar(local: &AttributeModel) -> Result<AttributeValueContract, String> {
    let mut model = local.clone();
    model.item_count = None;
    model.min_items = None;
    model.max_items = None;
    let result = AttributeValueContract {
        model,
        ..Default::default()
    };
    if !crate::application_declaration::valid_scalar_value(&result) {
        return Err(REFUSED.into());
    }
    Ok(result)
}
pub(super) fn bind(
    host: &mut FreshStructuralHost,
    contract: &Arc<VerifiedMixedReceiverDeclaration>,
    source: &MixedReceiverSource,
    scalars: &BTreeMap<String, ScalarReceiverSource>,
    roots: &BTreeMap<String, Arc<v3::InstalledDatatypeRoot>>,
) -> Result<(SchemaDocumentModel, BTreeMap<String, Receiver>), String> {
    if source.attributes.len() > 64
        || source.attributes.keys().ne(contract.occurrences().keys())
        || !Arc::ptr_eq(source.source.ast_owner(), source.capture.document())
    {
        return Err(REFUSED.into());
    }
    cem_ml::schema::structural_capsule::validate_mixed_receiver_source(
        source.source.ast(),
        &source.schema_uri,
        &host.control,
    )?;
    let mut expected_scalars = BTreeSet::new();
    let mut expected_roots = BTreeSet::new();
    for occurrence in contract.occurrences().values() {
        match contract.receiver(&occurrence.fields["type"]) {
            Some(ReceiverContract::Scalar(s)) => {
                expected_scalars.insert(s.name.clone());
            }
            Some(ReceiverContract::Executable { root, .. }) => {
                expected_roots.insert(root.clone());
            }
            _ => {}
        }
    }
    if expected_scalars != scalars.keys().cloned().collect()
        || expected_roots != roots.keys().cloned().collect()
    {
        return Err(REFUSED.into());
    }
    for (alias, src) in scalars {
        let Some(ReceiverContract::Scalar(value)) = contract.receiver(alias) else {
            return Err(REFUSED.into());
        };
        if src.identity.uri != value.source_uri
            || src.identity.hash != value.source_hash
            || src.source.source_uri() != src.identity.uri
            || src.source.ast().nodes.len() > 16_384
            || src.identity.byte_length > 1024 * 1024
            || src.attributes.len() != value.value.models().count()
        {
            return Err(REFUSED.into());
        }
        cem_ml::schema::structural_capsule::validate_literal_receiver_source(
            src.source.ast(),
            &value.schema_uri,
            &host.control,
        )?;
        for (id, expected) in src.attributes.iter().zip(value.value.models()) {
            let node = original(&src.source, *id, &value.schema_uri)?;
            if !registered(host, &src.source, &node)
                || compile_attribute_model(src.source.ast(), *id).as_ref() != Some(expected)
            {
                return Err(REFUSED.into());
            }
        }
    }
    let all = compile_schema_document_model_from_retained(&source.schema_uri, &source.source);
    if !all.compile_diagnostics.is_empty() {
        return Err(REFUSED.into());
    }
    let mut result = BTreeMap::new();
    for (name, occurrence) in contract.occurrences() {
        host.check().map_err(|_| REFUSED)?;
        let id = source.attributes[name];
        let declaration = original(&source.source, id, &source.schema_uri)?;
        if !registered(host, &source.source, &declaration) {
            return Err(REFUSED.into());
        }
        let local = compile_attribute_model(source.source.ast(), id).ok_or(REFUSED)?;
        if local.name != *name {
            return Err(REFUSED.into());
        }
        for (key, actual) in [
            ("itemCount", &local.item_count),
            ("minItems", &local.min_items),
            ("maxItems", &local.max_items),
        ] {
            if actual.as_ref() != occurrence.fields.get(key) {
                return Err(REFUSED.into());
            }
        }
        if local.item_count.is_some() || local.min_items.is_some() || local.max_items.is_some() {
            cem_ml::schema::structural_capsule::validate_literal_cardinality(
                local.item_count.as_deref(),
                local.min_items.as_deref(),
                local.max_items.as_deref(),
            )?;
        }
        let binding = match contract.receiver(&occurrence.fields["type"]) {
            Some(ReceiverContract::Node(_)) => {
                // Compile a private single-field contract from the original
                // literal attribute. This is not the whole schema with its
                // pending flags cleared: other source fields remain pending
                // and the enclosing model is never published as ready.
                let mut model = SchemaDocumentModel {
                    schema_uri: all.schema_uri.clone(),
                    attributes: BTreeMap::from([(name.clone(), local.clone())]),
                    ..Default::default()
                };
                model
                    .declaration_references
                    .attribute_declarations
                    .insert(name.clone(), declaration.clone());
                let guard = StructuralNodeReceiver::link(
                    contract
                        .node(&occurrence.fields["type"])
                        .ok_or(REFUSED)?
                        .clone(),
                    &host.registry_hash,
                    declaration.clone(),
                    &model,
                    &host.context,
                    &host.control,
                )?;
                model
                    .attribute_datatypes
                    .insert(name.clone(), guard.clone());
                Binding::Node { model, guard }
            }
            Some(ReceiverContract::Scalar(s)) => {
                let mut value = s.value.clone();
                let local = local_scalar(&local)?;
                if local
                    .model
                    .value_type
                    .as_deref()
                    .map(|v| v.strip_prefix("schema:").unwrap_or(v))
                    != value
                        .model
                        .value_type
                        .as_deref()
                        .map(|v| v.strip_prefix("schema:").unwrap_or(v))
                {
                    return Err(REFUSED.into());
                }
                value.restrict_with(&local);
                Binding::Scalar { contract: value }
            }
            Some(ReceiverContract::Executable { root, target }) => {
                let owner = roots.get(root).ok_or(REFUSED)?;
                if !owner.matches_original_root(host, contract.application(), target) {
                    return Err(REFUSED.into());
                }
                let bound = owner.bind_receiver(host, declaration.clone())?;
                // Only the signed local count vocabulary is allowed for this
                // executable profile; no unsupported facet may disappear.
                // Comparison only: preserve the original native pending slot.
                // This does not publish a schema or manufacture its readiness.
                let original = bound.local_constraints();
                let allowed = AttributeModel {
                    name: original.name.clone(),
                    value_type: original.value_type.clone(),
                    native_type_pending: original.native_type_pending,
                    source_map: original.source_map.clone(),
                    item_count: original.item_count.clone(),
                    min_items: original.min_items.clone(),
                    max_items: original.max_items.clone(),
                    ..Default::default()
                };
                if original != &allowed {
                    return Err(REFUSED.into());
                }
                Binding::Executable {
                    root: Arc::downgrade(owner),
                }
            }
            None => {
                let value = local_scalar(&local)?;
                if value.model.value_type.as_deref() != Some(occurrence.fields["type"].as_str()) {
                    return Err(REFUSED.into());
                }
                Binding::Scalar { contract: value }
            }
        };
        result.insert(
            name.clone(),
            Receiver {
                declaration,
                local,
                binding,
            },
        );
    }
    Ok((all, result))
}
