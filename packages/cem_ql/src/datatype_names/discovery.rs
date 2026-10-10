//! Discover schema members using captured names and bounded native collection selection.
use super::*;
use crate::schema_references::CemQlSchemaDeclarationHost;
use cem_ml::schema::{
    datatype_registry::{DatatypeDependencyValue, DatatypeRegistry, DatatypeRegistryError},
    machine::LexicallyScopedDocument,
    reference_traversal::ReferenceTraversalLimits,
};
use std::sync::Arc;
mod selection;
#[derive(Debug, Clone)]
pub struct DatatypeSchemaSource {
    pub schema: SchemaDeclarationNode,
    pub captured: Arc<LexicallyScopedDocument>,
    pub imports: Vec<DatatypeExport>,
}
fn pending(code: &'static str, source: &SchemaDeclarationNode) -> DatatypeNameError {
    DatatypeNameError {
        pending: true,
        ..error(code, source)
    }
}
fn named(
    host: &CemQlSchemaDeclarationHost,
    node: &SchemaDeclarationNode,
    expected: &str,
) -> Result<bool, DatatypeNameError> {
    let name = host
        .input_expanded_name(node)
        .ok_or_else(|| pending("datatype-source-name-pending", node))?;
    if name.local_name != expected {
        return Ok(false);
    }
    if !name.namespace_uri.is_empty() && name.namespace_uri != CEM_SCHEMA_URI {
        return Err(error("datatype-metamodel-name", node));
    }
    Ok(true)
}
fn children(
    source: &SchemaDeclarationNode,
    limits: &mut DatatypeNameLimits,
) -> Result<Vec<SchemaDeclarationNode>, DatatypeNameError> {
    let CemAstNode::Element { children, .. } = source.node() else {
        return Err(error("datatype-schema-element-required", source));
    };
    let mut output = vec![];
    for id in children {
        spend(limits, 0, source)?;
        let child = SchemaDeclarationNode::new(source.document().clone(), *id)
            .ok_or_else(|| error("datatype-source-child", source))?;
        match child.node() {
            CemAstNode::Whitespace { .. } | CemAstNode::Comment { .. } => {}
            CemAstNode::Text { data, .. } if data.trim().is_empty() => {}
            _ => output.push(child),
        }
    }
    Ok(output)
}
fn field(
    host: &CemQlSchemaDeclarationHost,
    node: &SchemaDeclarationNode,
    name: &str,
    limits: &mut DatatypeNameLimits,
) -> Result<Option<(String, SchemaDeclarationNode)>, DatatypeNameError> {
    let CemAstNode::Element { attributes, .. } = node.node() else {
        return Err(error("datatype-source-element-required", node));
    };
    let mut found = None;
    for id in attributes {
        spend(limits, 0, node)?;
        let attribute = SchemaDeclarationNode::new(node.document().clone(), *id).unwrap();
        if host.input_consumed_namespace_attribute(&attribute) {
            continue;
        }
        if !named(host, &attribute, name)? {
            continue;
        }
        if found.is_some() {
            return Err(error("duplicate-datatype-source-field", &attribute));
        }
        let CemAstNode::Attribute {
            value: Some(value),
            value_nodes,
            ..
        } = attribute.node()
        else {
            return Err(pending("datatype-source-field-unavailable", &attribute));
        };
        if !value_nodes.is_empty() {
            return Err(pending("datatype-source-field-unavailable", &attribute));
        }
        spend(limits, value.len(), &attribute)?;
        if value.trim().is_empty() {
            return Err(error("empty-datatype-source-field", &attribute));
        }
        found = Some((value.trim().to_owned(), attribute));
    }
    Ok(found)
}
impl DatatypeNameCatalog {
    /// This attaches only immutable source-name metadata, never runtime contexts.
    /// Publication is explicit through install_datatype_names after full discovery.
    pub fn discover(
        inputs: &[DatatypeSchemaSource],
        host: &mut CemQlSchemaDeclarationHost,
        limits: DatatypeNameLimits,
    ) -> Result<Self, DatatypeNameError> {
        if inputs.is_empty() {
            return Ok(Self::default());
        }
        Self::discover_with_limits(
            inputs,
            host,
            limits,
            ReferenceTraversalLimits::schema_defaults().map_err(|_| {
                pending("datatype-selection-defaults-unavailable", &inputs[0].schema)
            })?,
        )
    }

    /// Native collection slots run at this explicit consumer stage, under one
    /// request per selected schema and a shared remaining request work budget.
    /// Selected declarations retain their original supplied schema environment.
    pub fn discover_with_limits(
        inputs: &[DatatypeSchemaSource],
        host: &mut CemQlSchemaDeclarationHost,
        mut limits: DatatypeNameLimits,
        mut traversal: ReferenceTraversalLimits,
    ) -> Result<Self, DatatypeNameError> {
        let mut indices = BTreeMap::new();
        // Attach every supplied owner's immutable names before selecting across owners.
        for (index, input) in inputs.iter().enumerate() {
            spend(&mut limits, 0, &input.schema)?;
            if !Arc::ptr_eq(input.schema.document(), input.captured.document()) {
                return Err(error("datatype-capture-owner", &input.schema));
            }
            if indices.insert(input.schema.identity(), index).is_some() {
                return Err(error("duplicate-datatype-name-scope", &input.schema));
            }
            host.attach_captured_names(&input.captured)
                .map_err(|_| pending("datatype-source-owner-unavailable", &input.schema))?;
            host.attach_captured_namespaces(input.captured.clone())
                .map_err(|_| pending("datatype-source-owner-unavailable", &input.schema))?;
            if !named(host, &input.schema, "schema")? {
                return Err(error("datatype-schema-required", &input.schema));
            }
        }
        let mut members = vec![BTreeMap::new(); inputs.len()];
        let mut environments = vec![];
        for input in inputs {
            let namespace = field(host, &input.schema, "namespace", &mut limits)?
                .ok_or_else(|| error("datatype-schema-namespace-required", &input.schema))?
                .0;
            let (types, entries) =
                selection::collections(input, host, &mut limits, &mut traversal)?;
            let mut uses = BTreeMap::<String, (String, SchemaDeclarationNode)>::new();
            for entry in entries {
                if !matches!(entry.node(), CemAstNode::Element { .. }) {
                    return Err(error("datatype-use-content", &entry));
                }
                if !named(host, &entry, "use")? {
                    return Err(error("datatype-use-required", &entry));
                }
                original_input(&entry, inputs, &indices, host)?;
                let (alias, origin) = field(host, &entry, "as", &mut limits)?
                    .ok_or_else(|| error("datatype-use-alias-required", &entry))?;
                if !local(&alias) {
                    return Err(error("invalid-datatype-prefix", &origin));
                }
                let uri = field(host, &entry, "schema", &mut limits)?
                    .ok_or_else(|| error("datatype-use-schema-required", &entry))?
                    .0;
                if let Some((old, source)) = uses.get(&alias) {
                    if old != &uri {
                        return Err(DatatypeNameError {
                            related: Some(source.clone()),
                            ..error("conflicting-datatype-use", &origin)
                        });
                    }
                }
                uses.insert(alias, (uri, origin));
            }
            for declaration in types {
                let index = original_input(&declaration, inputs, &indices, host)?;
                members[index].insert(declaration.identity(), declaration);
            }
            environments.push((namespace, uses));
        }
        let schema_aliases = inputs
            .iter()
            .zip(&environments)
            .map(|(input, (_, uses))| {
                (
                    input.schema.identity(),
                    uses.iter()
                        .map(|(alias, (uri, _))| (alias.clone(), uri.clone()))
                        .collect(),
                )
            })
            .collect();
        let mut scopes = vec![];
        for ((input, types), (namespace, uses)) in inputs.iter().zip(members).zip(environments) {
            let mut declarations = vec![];
            let mut registry = DatatypeRegistry::default();
            for declaration in types.into_values() {
                if input
                    .captured
                    .namespace_binding(declaration.document(), declaration.node_id())
                    .is_some()
                    || input
                        .captured
                        .pending_namespace_declaration(
                            declaration.document(),
                            declaration.node_id(),
                        )
                        .is_some()
                {
                    continue;
                }
                if !matches!(declaration.node(), CemAstNode::Element { .. }) {
                    return Err(
                        if matches!(declaration.node(), CemAstNode::Reference { .. }) {
                            pending("datatype-collection-selection-unavailable", &declaration)
                        } else {
                            error("datatype-collection-content", &declaration)
                        },
                    );
                }
                if !named(host, &declaration, "type")? {
                    return Err(error("datatype-declaration-required", &declaration));
                }
                let name = field(host, &declaration, "name", &mut limits)?
                    .ok_or_else(|| error("missing-datatype-name", &declaration))?
                    .0;
                registry
                    .insert(input.schema.clone(), declaration.clone())
                    .map_err(|issue| match issue {
                        DatatypeRegistryError::Duplicate {
                            existing, incoming, ..
                        } => DatatypeNameError {
                            related: Some(existing),
                            ..error("duplicate-datatype-name", &incoming)
                        },
                        _ => error("invalid-datatype-name", &declaration),
                    })?;
                let source = registry
                    .source(&input.schema, &name)
                    .ok_or_else(|| error("invalid-datatype-name", &declaration))?;
                let mut aliases = BTreeMap::new();
                // Only consumed QName prefixes need admission; unused pending prefixes
                // do not make a literal local dependency unavailable.
                for dependency in source.plan().dependencies {
                    if let DatatypeDependencyValue::Literal(qname) = &dependency.value {
                        let Some((prefix, _)) = qname.split_once(':') else {
                            continue;
                        };
                        let lexical = host
                            .datatype_captured_prefix(
                                &input.captured,
                                &dependency.attribute,
                                prefix,
                            )
                            .map_err(|code| pending(code, &dependency.attribute))?;
                        let alias = match (lexical, uses.get(prefix)) {
                            (Some(Some(uri)), Some((declared, origin))) if uri != *declared => {
                                return Err(DatatypeNameError {
                                    related: Some(origin.clone()),
                                    ..error("conflicting-datatype-alias", &dependency.attribute)
                                })
                            }
                            (Some(value), _) => Some(value),
                            (None, Some((uri, _))) => Some(Some(uri.clone())),
                            (None, None) => None,
                        };
                        if let Some(value) = alias {
                            spend(
                                &mut limits,
                                prefix
                                    .len()
                                    .saturating_add(value.as_ref().map_or(0, String::len)),
                                &dependency.attribute,
                            )?;
                            aliases.insert(prefix.into(), value);
                        }
                    }
                }
                declarations.push(DatatypeNameDeclaration { source, aliases });
            }
            // Charge before copying caller-supplied import lists into the candidate.
            for export in &input.imports {
                spend(
                    &mut limits,
                    export.namespace.len().saturating_add(export.name.len()),
                    export.source.declaration(),
                )?;
            }
            // Exports are caller-authorized original handles, never inferred by URI.
            scopes.push(DatatypeNameScope {
                scope: input.schema.clone(),
                namespace: Some(namespace),
                declarations,
                imports: input.imports.clone(),
            });
        }
        let mut catalog = Self::collect(&scopes, host, limits)?;
        catalog.schema_aliases = schema_aliases;
        Ok(catalog)
    }
}

fn original_input(
    node: &SchemaDeclarationNode,
    inputs: &[DatatypeSchemaSource],
    indices: &BTreeMap<String, usize>,
    host: &CemQlSchemaDeclarationHost,
) -> Result<usize, DatatypeNameError> {
    let schema = host
        .declaration_schema(node)
        .ok_or_else(|| pending("datatype-selected-schema-unavailable", node))?;
    let index = *indices
        .get(&schema.identity())
        .ok_or_else(|| pending("datatype-selected-schema-unavailable", node))?;
    if !Arc::ptr_eq(node.document(), inputs[index].captured.document()) {
        return Err(error("datatype-capture-owner", node));
    }
    Ok(index)
}
