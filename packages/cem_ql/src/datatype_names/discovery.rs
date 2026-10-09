//! Discover literal schema members using captured names and source-position QName
//! contexts. Native collection slots remain explicit pending consumer work.
use super::*;
use crate::schema_references::CemQlSchemaDeclarationHost;
use cem_ml::schema::{
    datatype_registry::{DatatypeDependencyValue, DatatypeRegistry, DatatypeRegistryError},
    machine::LexicallyScopedDocument,
};
use std::sync::Arc;
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
        mut limits: DatatypeNameLimits,
    ) -> Result<Self, DatatypeNameError> {
        let mut scopes = vec![];
        for input in inputs {
            spend(&mut limits, 0, &input.schema)?;
            if !Arc::ptr_eq(input.schema.document(), input.captured.document()) {
                return Err(error("datatype-capture-owner", &input.schema));
            }
            host.attach_captured_names(&input.captured)
                .map_err(|_| pending("datatype-source-owner-unavailable", &input.schema))?;
            if !named(host, &input.schema, "schema")? {
                return Err(error("datatype-schema-required", &input.schema));
            }
            let namespace = field(host, &input.schema, "namespace", &mut limits)?
                .ok_or_else(|| error("datatype-schema-namespace-required", &input.schema))?
                .0;
            let mut declarations = vec![];
            let mut uses = BTreeMap::<String, (String, SchemaDeclarationNode)>::new();
            let mut types = vec![];
            for collection in children(&input.schema, &mut limits)? {
                if !matches!(collection.node(), CemAstNode::Element { .. }) {
                    return Err(
                        if matches!(collection.node(), CemAstNode::Reference { .. }) {
                            pending("datatype-schema-selection-unavailable", &collection)
                        } else {
                            error("datatype-schema-content", &collection)
                        },
                    );
                }
                if named(host, &collection, "types")? {
                    types.extend(children(&collection, &mut limits)?);
                } else if named(host, &collection, "uses")? {
                    for entry in children(&collection, &mut limits)? {
                        if !matches!(entry.node(), CemAstNode::Element { .. }) {
                            return Err(if matches!(entry.node(), CemAstNode::Reference { .. }) {
                                pending("datatype-use-selection-unavailable", &entry)
                            } else {
                                error("datatype-use-content", &entry)
                            });
                        }
                        if !named(host, &entry, "use")? {
                            return Err(error("datatype-use-required", &entry));
                        }
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
                }
            }
            let mut registry = DatatypeRegistry::default();
            for declaration in types {
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
        Self::collect(&scopes, host, limits)
    }
}
