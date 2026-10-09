//! Immutable datatype name discovery over explicit lifecycle inputs. Exports
//! name original sources; they neither copy declarations nor authorize crossings.
use cem_ml::{
    parser::CemAstNode,
    schema::{
        datatype_registry::DatatypeSource,
        declaration_references::{SchemaDeclarationHost, SchemaDeclarationNode},
        registry::CEM_SCHEMA_URI,
    },
    validation::xpath::xpath_is_qname,
};
use std::collections::BTreeMap;

/// Effective aliases at this original declaration, supplied by its lexical consumer.
/// None is a retained pending binding, distinct from an undeclared prefix.
#[derive(Debug, Clone)]
pub struct DatatypeNameDeclaration {
    pub source: DatatypeSource,
    pub aliases: BTreeMap<String, Option<String>>,
}
/// An explicitly admitted public name. Renaming does not rebind the target's dependencies.
#[derive(Debug, Clone)]
pub struct DatatypeExport {
    pub namespace: String,
    pub name: String,
    pub source: DatatypeSource,
}
#[derive(Debug, Clone)]
pub struct DatatypeNameScope {
    pub scope: SchemaDeclarationNode,
    pub namespace: Option<String>,
    pub declarations: Vec<DatatypeNameDeclaration>,
    /// Public names admitted into this scope by explicitly supplied export contracts.
    pub imports: Vec<DatatypeExport>,
}
#[derive(Debug, Clone, Copy)]
pub struct DatatypeNameLimits {
    pub max_work: usize,
    pub max_name_bytes: usize,
}
impl Default for DatatypeNameLimits {
    fn default() -> Self {
        Self {
            max_work: 100_000,
            max_name_bytes: 1_048_576,
        }
    }
}
#[derive(Debug, Clone)]
pub struct DatatypeNameError {
    pub code: &'static str,
    pub source: SchemaDeclarationNode,
    pub related: Option<SchemaDeclarationNode>,
    pub pending: bool,
}
fn error(code: &'static str, source: &SchemaDeclarationNode) -> DatatypeNameError {
    DatatypeNameError {
        code,
        source: source.clone(),
        related: None,
        pending: false,
    }
}
fn local(value: &str) -> bool {
    !value.contains(':') && xpath_is_qname(value)
}
fn spend(
    limits: &mut DatatypeNameLimits,
    bytes: usize,
    source: &SchemaDeclarationNode,
) -> Result<(), DatatypeNameError> {
    if limits.max_work == 0 || bytes > limits.max_name_bytes {
        return Err(DatatypeNameError {
            pending: true,
            ..error("datatype-name-collection-limit", source)
        });
    }
    limits.max_work -= 1;
    limits.max_name_bytes -= bytes;
    Ok(())
}
#[derive(Debug, Clone)]
struct Scope {
    source: SchemaDeclarationNode,
    namespace: Option<String>,
    locals: BTreeMap<String, DatatypeSource>,
    imports: BTreeMap<(String, String), DatatypeSource>,
}
#[derive(Debug, Clone, Default)]
pub struct DatatypeNameCatalog {
    scopes: BTreeMap<String, Scope>,
    declarations: BTreeMap<String, DatatypeNameDeclaration>,
}
#[derive(Debug)]
pub enum DatatypeNameLookup<'a> {
    Target(&'a DatatypeSource),
    Pending(&'static str),
    Unresolved(&'static str),
}
impl DatatypeNameCatalog {
    /// Collect all local names before checking explicit exports, permitting forward
    /// declarations without source-order overrides. Failure publishes no partial catalog.
    /// Full datatype facet/capability admission remains with descriptor compilation.
    pub fn collect<H: SchemaDeclarationHost>(
        inputs: &[DatatypeNameScope],
        host: &H,
        mut limits: DatatypeNameLimits,
    ) -> Result<Self, DatatypeNameError> {
        let mut catalog = Self::default();
        for input in inputs {
            spend(
                &mut limits,
                input.namespace.as_ref().map_or(0, String::len),
                &input.scope,
            )?;
            if catalog.scopes.contains_key(&input.scope.identity()) {
                return Err(error("duplicate-datatype-name-scope", &input.scope));
            }
            let mut scope = Scope {
                source: input.scope.clone(),
                namespace: input.namespace.clone(),
                locals: BTreeMap::new(),
                imports: BTreeMap::new(),
            };
            for declaration in &input.declarations {
                let source = &declaration.source;
                spend(&mut limits, 0, source.declaration())?;
                if source.scope().identity() != input.scope.identity() {
                    return Err(error("datatype-name-source-scope", source.declaration()));
                }
                let effective =
                    host.input_expanded_name(source.declaration())
                        .ok_or_else(|| DatatypeNameError {
                            pending: true,
                            ..error("datatype-name-pending", source.declaration())
                        })?;
                if effective.local_name != "type"
                    || (!effective.namespace_uri.is_empty()
                        && effective.namespace_uri != CEM_SCHEMA_URI)
                {
                    return Err(error("datatype-metamodel-name", source.declaration()));
                }
                let mut name = None;
                for field in source.attributes() {
                    spend(&mut limits, 0, field)?;
                    if host.input_consumed_namespace_attribute(field) {
                        continue;
                    }
                    let effective =
                        host.input_expanded_name(field)
                            .ok_or_else(|| DatatypeNameError {
                                pending: true,
                                ..error("datatype-name-pending", field)
                            })?;
                    if effective.local_name != "name" {
                        continue;
                    }
                    if !effective.namespace_uri.is_empty()
                        && effective.namespace_uri != CEM_SCHEMA_URI
                    {
                        return Err(error("datatype-metamodel-name", field));
                    }
                    if name.is_some() {
                        return Err(error("duplicate-datatype-name-field", field));
                    }
                    let CemAstNode::Attribute {
                        value: Some(value),
                        value_nodes,
                        ..
                    } = field.node()
                    else {
                        return Err(error("invalid-datatype-name", field));
                    };
                    let value = value.trim();
                    if !value_nodes.is_empty() || !local(value) {
                        return Err(error("invalid-datatype-name", field));
                    }
                    spend(&mut limits, value.len(), field)?;
                    name = Some(value.to_owned());
                }
                let name =
                    name.ok_or_else(|| error("missing-datatype-name", source.declaration()))?;
                if let Some(old) = scope.locals.get(&name) {
                    if old.declaration().identity() != source.declaration().identity() {
                        return Err(DatatypeNameError {
                            related: Some(old.declaration().clone()),
                            ..error("duplicate-datatype-name", source.declaration())
                        });
                    }
                }
                for (prefix, uri) in &declaration.aliases {
                    spend(
                        &mut limits,
                        prefix
                            .len()
                            .saturating_add(uri.as_ref().map_or(0, String::len)),
                        source.declaration(),
                    )?;
                    if !local(prefix) {
                        return Err(error("invalid-datatype-prefix", source.declaration()));
                    }
                }
                let key = source.declaration().identity();
                if let Some(old) = catalog.declarations.get(&key) {
                    if old.source.scope().identity() != source.scope().identity()
                        || old.aliases != declaration.aliases
                    {
                        return Err(error(
                            "conflicting-datatype-name-environment",
                            source.declaration(),
                        ));
                    }
                }
                scope.locals.insert(name, source.clone());
                catalog.declarations.insert(key, declaration.clone());
            }
            catalog.scopes.insert(input.scope.identity(), scope);
        }
        // An export must name a source collected in its own original environment.
        for input in inputs {
            for export in &input.imports {
                spend(
                    &mut limits,
                    export.namespace.len().saturating_add(export.name.len()),
                    export.source.declaration(),
                )?;
                if !local(&export.name) {
                    return Err(error(
                        "invalid-datatype-export-name",
                        export.source.declaration(),
                    ));
                }
                let registered = catalog
                    .declarations
                    .get(&export.source.declaration().identity())
                    .ok_or_else(|| {
                        error("uncollected-datatype-export", export.source.declaration())
                    })?;
                if registered.source.scope().identity() != export.source.scope().identity() {
                    return Err(error(
                        "datatype-export-source-scope",
                        export.source.declaration(),
                    ));
                }
                let scope = catalog.scopes.get_mut(&input.scope.identity()).unwrap();
                let key = (export.namespace.clone(), export.name.clone());
                let old = scope.imports.get(&key).or_else(|| {
                    if scope.namespace.as_deref() == Some(&export.namespace) {
                        scope.locals.get(&export.name)
                    } else {
                        None
                    }
                });
                if let Some(old) = old {
                    if old.declaration().identity() != export.source.declaration().identity() {
                        return Err(DatatypeNameError {
                            related: Some(old.declaration().clone()),
                            ..error("ambiguous-datatype-export", export.source.declaration())
                        });
                    }
                }
                scope.imports.insert(key, export.source.clone());
            }
        }
        Ok(catalog)
    }
    pub fn contains_scope(&self, scope: &SchemaDeclarationNode) -> bool {
        self.scopes.contains_key(&scope.identity())
    }
    pub fn sources(&self) -> impl Iterator<Item = &DatatypeSource> {
        self.declarations.values().map(|d| &d.source)
    }
    pub fn scopes(&self) -> impl Iterator<Item = &SchemaDeclarationNode> {
        self.scopes.values().map(|s| &s.source)
    }
    pub fn source(&self, target: &SchemaDeclarationNode) -> Option<&DatatypeSource> {
        self.declarations.get(&target.identity()).map(|d| &d.source)
    }
    pub fn lookup(&self, source: &DatatypeSource, qname: &str) -> DatatypeNameLookup<'_> {
        use DatatypeNameLookup::*;
        let Some(scope) = self.scopes.get(&source.scope().identity()) else {
            return Pending("datatype-name-scope-unavailable");
        };
        let Some(declaration) = self
            .declarations
            .get(&source.declaration().identity())
            .filter(|d| d.source.scope().identity() == source.scope().identity())
        else {
            return Pending("datatype-name-source-unavailable");
        };
        if !xpath_is_qname(qname) {
            return Unresolved("invalid-datatype-qname");
        }
        let (namespace, name) = if let Some((prefix, name)) = qname.split_once(':') {
            match declaration.aliases.get(prefix) {
                Some(Some(uri)) => (uri.as_str(), name),
                Some(None) => return Pending("datatype-prefix-pending"),
                None => return Unresolved("unknown-datatype-prefix"),
            }
        } else {
            let Some(namespace) = &scope.namespace else {
                return Pending("datatype-namespace-pending");
            };
            (namespace.as_str(), qname)
        };
        let selected = if scope.namespace.as_deref() == Some(namespace) {
            scope.locals.get(name)
        } else {
            None
        }
        .or_else(|| scope.imports.get(&(namespace.into(), name.into())));
        selected.map_or(Unresolved("unexported-datatype-name"), Target)
    }
}
