//! Schema-owned structural contracts for runtime records. No query evaluation,
//! datatype execution, URI loading or implicit access grants occur here.
use super::machine::LexicallyScopedDocument;
use super::{declaration_references::SchemaDeclarationNode, registry::CEM_SCHEMA_URI};
use crate::parser::CemAstNode;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContractName {
    pub namespace: String,
    pub local: String,
}
impl ContractName {
    pub fn new(namespace: impl Into<String>, local: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            local: local.into(),
        }
    }
}
/// The host supplies the original declaring bindings and only authorized sources.
#[derive(Debug, Clone)]
pub struct ValueContractSource {
    pub schema: SchemaDeclarationNode,
    pub namespace: String,
    pub bindings: BTreeMap<String, String>,
    captured: Option<Arc<LexicallyScopedDocument>>,
}
impl ValueContractSource {
    pub fn new(
        schema: SchemaDeclarationNode,
        namespace: impl Into<String>,
        bindings: BTreeMap<String, String>,
    ) -> Self {
        Self {
            schema,
            namespace: namespace.into(),
            bindings,
            captured: None,
        }
    }
    pub fn with_captured_names(
        mut self,
        captured: Arc<LexicallyScopedDocument>,
    ) -> Result<Self, ValueContractError> {
        if !Arc::ptr_eq(captured.document(), self.schema.document()) {
            return Err(ValueContractError::at("unrelated-name-owner", &self.schema));
        }
        self.captured = Some(captured);
        Ok(self)
    }
    pub(crate) fn name<'a>(
        &'a self,
        node: &'a SchemaDeclarationNode,
    ) -> Option<&'a crate::parser::ExpandedName> {
        if let Some(captured) = &self.captured {
            return captured.expanded_name(node.document(), node.node_id());
        }
        match node.node() {
            CemAstNode::Element { expanded_name, .. }
            | CemAstNode::Attribute { expanded_name, .. } => Some(expanded_name),
            _ => None,
        }
    }
    pub(crate) fn named(&self, node: &SchemaDeclarationNode, local: &str) -> bool {
        self.name(node)
            .is_some_and(|name| name.namespace_uri == CEM_SCHEMA_URI && name.local_name == local)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct ValueContractLimits {
    pub max_contracts: usize,
    pub max_fields: usize,
    pub max_depth: usize,
    pub max_values: usize,
}
impl Default for ValueContractLimits {
    fn default() -> Self {
        Self {
            max_contracts: 256,
            max_fields: 4096,
            max_depth: 64,
            max_values: 100_000,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    One,
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
}
impl Cardinality {
    pub fn admits(self, count: usize) -> bool {
        match self {
            Self::One => count == 1,
            Self::ZeroOrOne => count <= 1,
            Self::ZeroOrMore => true,
            Self::OneOrMore => count > 0,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueFieldType {
    String,
    Boolean,
    Node,
    Record(ContractName),
}
#[derive(Debug, Clone)]
pub struct ValueFieldContract {
    pub source: SchemaDeclarationNode,
    pub name: String,
    pub value_type: ValueFieldType,
    pub required: bool,
    pub cardinality: Cardinality,
    pub values: Option<BTreeSet<String>>,
}
#[derive(Debug, Clone)]
pub struct ValueContract {
    pub source: SchemaDeclarationNode,
    pub name: ContractName,
    pub allow_extra: bool,
    pub fields: BTreeMap<String, ValueFieldContract>,
}
#[derive(Clone)]
pub struct ValueContractError {
    pub code: &'static str,
    pub path: Vec<String>,
    pub source: Option<SchemaDeclarationNode>,
}
impl std::fmt::Debug for ValueContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueContractError")
            .field("code", &self.code)
            .field("path", &self.path)
            .field(
                "source",
                &self.source.as_ref().map(SchemaDeclarationNode::identity),
            )
            .finish()
    }
}
impl ValueContractError {
    pub fn new(code: &'static str) -> Self {
        Self {
            code,
            path: vec![],
            source: None,
        }
    }
    pub(crate) fn at(code: &'static str, source: &SchemaDeclarationNode) -> Self {
        Self {
            code,
            path: vec![],
            source: Some(source.clone()),
        }
    }
    fn field(mut self, name: impl Into<String>) -> Self {
        self.path.insert(0, name.into());
        self
    }
}
/// Native adapters supply typed views, never AST-to-JSON projections. A field
/// missing from this list differs from a present field with an empty sequence.
pub trait ContractValue: Clone {
    fn string(&self) -> Option<String>;
    fn boolean(&self) -> Option<bool>;
    fn is_native_node(&self) -> bool;
    fn record_fields(&self) -> Option<Vec<(String, Vec<Self>)>>;
}
#[derive(Debug, Clone)]
pub struct ValueContracts {
    contracts: BTreeMap<ContractName, ValueContract>,
    limits: ValueContractLimits,
}
impl ValueContracts {
    pub fn compile(
        sources: &[ValueContractSource],
        limits: ValueContractLimits,
    ) -> Result<Self, ValueContractError> {
        let mut result = Self {
            contracts: BTreeMap::new(),
            limits,
        };
        let mut fields = 0;
        for input in sources {
            if input.name(&input.schema).is_none() {
                return Err(ValueContractError::at("pending-source-name", &input.schema));
            }
            if !input.named(&input.schema, "schema") || input.namespace.is_empty() {
                return Err(ValueContractError::at(
                    "invalid-schema-source",
                    &input.schema,
                ));
            }
            if input
                .schema
                .document()
                .diagnostics
                .iter()
                .any(|d| d.severity.is_hard_violation())
            {
                return Err(ValueContractError::at(
                    "invalid-source-document",
                    &input.schema,
                ));
            }
            for collection in elements(&input.schema)? {
                if input.name(&collection).is_none() {
                    return Err(ValueContractError::at("pending-source-name", &collection));
                }
                if !input.named(&collection, "value-contracts") {
                    continue;
                }
                attrs(input, &collection, &[])?;
                for declaration in elements(&collection)? {
                    if !input.named(&declaration, "value-contract") {
                        return Err(ValueContractError::at(
                            "unexpected-declaration",
                            &declaration,
                        ));
                    }
                    let attributes = attrs(input, &declaration, &["name", "allow-extra"])?;
                    let local = required(&attributes, "name", &declaration)?;
                    if !identifier(&local) {
                        return Err(ValueContractError::at("invalid-name", &declaration));
                    }
                    let name = ContractName::new(&input.namespace, local);
                    if let Some(previous) = result.contracts.get(&name) {
                        if previous.source.identity() == declaration.identity() {
                            continue;
                        }
                        return Err(ValueContractError::at("duplicate-contract", &declaration));
                    }
                    if result.contracts.len() >= limits.max_contracts {
                        return Err(ValueContractError::at("contract-limit", &declaration));
                    }
                    let mut contract = ValueContract {
                        source: declaration.clone(),
                        name: name.clone(),
                        allow_extra: boolean(&attributes, "allow-extra", &declaration)?,
                        fields: BTreeMap::new(),
                    };
                    for field in elements(&declaration)? {
                        fields += 1;
                        if fields > limits.max_fields {
                            return Err(ValueContractError::at("field-limit", &field));
                        }
                        if !input.named(&field, "value-field") {
                            return Err(ValueContractError::at("unexpected-field", &field));
                        }
                        let a = attrs(
                            input,
                            &field,
                            &["name", "kind", "type", "required", "cardinality", "values"],
                        )?;
                        let field_name = required(&a, "name", &field)?;
                        if field_name.is_empty() {
                            return Err(ValueContractError::at("invalid-field-name", &field));
                        }
                        let ty = match (a.get("kind"), a.get("type")) {
                            (Some(kind), None) => match kind.as_str() {
                                "string" => ValueFieldType::String,
                                "boolean" => ValueFieldType::Boolean,
                                "node" => ValueFieldType::Node,
                                _ => {
                                    return Err(ValueContractError::at(
                                        "unsupported-field-kind",
                                        &field,
                                    ))
                                }
                            },
                            (None, Some(name)) => {
                                ValueFieldType::Record(resolve_name(name, input, &field)?)
                            }
                            _ => return Err(ValueContractError::at("field-type-choice", &field)),
                        };
                        let cardinality = match a
                            .get("cardinality")
                            .map(String::as_str)
                            .unwrap_or("one")
                        {
                            "one" => Cardinality::One,
                            "zero-or-one" => Cardinality::ZeroOrOne,
                            "zero-or-more" => Cardinality::ZeroOrMore,
                            "one-or-more" => Cardinality::OneOrMore,
                            _ => return Err(ValueContractError::at("invalid-cardinality", &field)),
                        };
                        let values = a
                            .get("values")
                            .map(|s| s.split_whitespace().map(str::to_owned).collect());
                        if values.is_some() && ty != ValueFieldType::String {
                            return Err(ValueContractError::at("unsupported-values", &field));
                        }
                        if !elements(&field)?.is_empty() {
                            return Err(ValueContractError::at("unexpected-field-child", &field));
                        }
                        let item = ValueFieldContract {
                            source: field.clone(),
                            name: field_name.clone(),
                            value_type: ty,
                            required: boolean(&a, "required", &field)?,
                            cardinality,
                            values,
                        };
                        if contract.fields.insert(field_name, item).is_some() {
                            return Err(ValueContractError::at("duplicate-field", &field));
                        }
                    }
                    result.contracts.insert(name, contract);
                }
            }
        }
        let mut done = BTreeMap::new();
        let mut active = BTreeSet::new();
        for name in result.contracts.keys() {
            result.check_dependencies(name, &mut done, &mut active, 0)?;
        }
        Ok(result)
    }
    pub fn get(&self, name: &ContractName) -> Option<&ValueContract> {
        self.contracts.get(name)
    }
    pub fn limits(&self) -> ValueContractLimits {
        self.limits
    }
    fn check_dependencies(
        &self,
        name: &ContractName,
        done: &mut BTreeMap<ContractName, usize>,
        active: &mut BTreeSet<ContractName>,
        depth: usize,
    ) -> Result<usize, ValueContractError> {
        let contract = self
            .get(name)
            .ok_or_else(|| ValueContractError::new("unknown-contract"))?;
        if depth > self.limits.max_depth {
            return Err(ValueContractError::at("depth-limit", &contract.source));
        }
        if let Some(height) = done.get(name) {
            if depth.saturating_add(*height) > self.limits.max_depth {
                return Err(ValueContractError::at("depth-limit", &contract.source));
            }
            return Ok(*height);
        }
        if !active.insert(name.clone()) {
            return Err(ValueContractError::at("cyclic-contract", &contract.source));
        }
        let mut height = 0;
        for field in contract.fields.values() {
            if let ValueFieldType::Record(target) = &field.value_type {
                let child_height = self
                    .check_dependencies(target, done, active, depth + 1)
                    .map_err(|mut e| {
                        if e.source.is_none() {
                            e.source = Some(field.source.clone());
                        }
                        e
                    })?;
                height = height.max(child_height + 1);
            }
        }
        active.remove(name);
        done.insert(name.clone(), height);
        Ok(height)
    }
    /// Exactly one result record. Nested streams share this request's work cap.
    pub fn validate<V: ContractValue>(
        &self,
        name: &ContractName,
        values: &[V],
    ) -> Result<(), ValueContractError> {
        let contract = self
            .get(name)
            .ok_or_else(|| ValueContractError::new("unknown-contract"))?;
        if values.len() != 1 {
            return Err(ValueContractError::at(
                "result-cardinality",
                &contract.source,
            ));
        }
        self.validate_record(name, &values[0], 0, &mut 0)
    }
    fn validate_record<V: ContractValue>(
        &self,
        name: &ContractName,
        value: &V,
        depth: usize,
        work: &mut usize,
    ) -> Result<(), ValueContractError> {
        let contract = self
            .get(name)
            .ok_or_else(|| ValueContractError::new("unknown-contract"))?;
        if depth > self.limits.max_depth {
            return Err(ValueContractError::at("depth-limit", &contract.source));
        }
        charge(work, 1, self.limits.max_values, &contract.source)?;
        let entries = value
            .record_fields()
            .ok_or_else(|| ValueContractError::at("record-required", &contract.source))?;
        charge(
            work,
            entries.len(),
            self.limits.max_values,
            &contract.source,
        )?;
        let mut fields = BTreeMap::new();
        for (key, values) in entries {
            charge(work, values.len(), self.limits.max_values, &contract.source)?;
            if !contract.allow_extra && !contract.fields.contains_key(&key) {
                return Err(ValueContractError::at("undeclared-field", &contract.source).field(key));
            }
            if fields.insert(key.clone(), values).is_some() {
                return Err(
                    ValueContractError::at("duplicate-value-field", &contract.source).field(key),
                );
            }
        }
        for (key, field) in &contract.fields {
            let Some(values) = fields.get(key) else {
                if field.required {
                    return Err(ValueContractError::at("missing-field", &field.source).field(key));
                }
                continue;
            };
            if !field.cardinality.admits(values.len()) {
                return Err(ValueContractError::at("field-cardinality", &field.source).field(key));
            }
            for (index, item) in values.iter().enumerate() {
                let valid = match &field.value_type {
                    ValueFieldType::String => item.string().is_some_and(|s| {
                        field
                            .values
                            .as_ref()
                            .is_none_or(|allowed| allowed.contains(&s))
                    }),
                    ValueFieldType::Boolean => item.boolean().is_some(),
                    ValueFieldType::Node => item.is_native_node(),
                    ValueFieldType::Record(target) => {
                        self.validate_record(target, item, depth + 1, work)
                            .map_err(|e| e.field(index.to_string()).field(key))?;
                        true
                    }
                };
                if !valid {
                    return Err(ValueContractError::at("field-value", &field.source)
                        .field(index.to_string())
                        .field(key));
                }
            }
        }
        Ok(())
    }
}
fn charge(
    work: &mut usize,
    count: usize,
    max: usize,
    source: &SchemaDeclarationNode,
) -> Result<(), ValueContractError> {
    *work = work
        .checked_add(count)
        .ok_or_else(|| ValueContractError::at("work-limit", source))?;
    if *work > max {
        return Err(ValueContractError::at("work-limit", source));
    }
    Ok(())
}
pub(crate) fn elements(
    parent: &SchemaDeclarationNode,
) -> Result<Vec<SchemaDeclarationNode>, ValueContractError> {
    let CemAstNode::Element { children, .. } = parent.node() else {
        return Err(ValueContractError::at("element-required", parent));
    };
    let mut result = vec![];
    for id in children {
        let child = SchemaDeclarationNode::new(parent.document().clone(), *id).unwrap();
        match child.node() {
            CemAstNode::Element { .. } => result.push(child),
            CemAstNode::Whitespace { .. } | CemAstNode::Comment { .. } => {}
            CemAstNode::Text { data, .. } if data.trim().is_empty() => {}
            _ => return Err(ValueContractError::at("unsupported-source-content", &child)),
        }
    }
    Ok(result)
}
pub(crate) fn attrs(
    input: &ValueContractSource,
    source: &SchemaDeclarationNode,
    allowed: &[&str],
) -> Result<BTreeMap<String, String>, ValueContractError> {
    let CemAstNode::Element { attributes, .. } = source.node() else {
        return Err(ValueContractError::at("element-required", source));
    };
    let mut result = BTreeMap::new();
    for id in attributes {
        let a = SchemaDeclarationNode::new(source.document().clone(), *id).unwrap();
        let CemAstNode::Attribute {
            value: Some(value),
            value_nodes,
            ..
        } = a.node()
        else {
            return Err(ValueContractError::at("literal-field-required", &a));
        };
        if !value_nodes.is_empty() {
            return Err(ValueContractError::at("literal-field-required", &a));
        }
        let expanded_name = input
            .name(&a)
            .ok_or_else(|| ValueContractError::at("pending-source-name", &a))?;
        if (!expanded_name.namespace_uri.is_empty()
            && expanded_name.namespace_uri != CEM_SCHEMA_URI)
            || !allowed.contains(&expanded_name.local_name.as_str())
        {
            return Err(ValueContractError::at("unknown-source-field", &a));
        }
        if result
            .insert(expanded_name.local_name.clone(), value.clone())
            .is_some()
        {
            return Err(ValueContractError::at("duplicate-source-field", &a));
        }
    }
    Ok(result)
}
pub(crate) fn required(
    a: &BTreeMap<String, String>,
    key: &str,
    source: &SchemaDeclarationNode,
) -> Result<String, ValueContractError> {
    a.get(key)
        .cloned()
        .ok_or_else(|| ValueContractError::at("missing-source-field", source).field(key))
}
pub(crate) fn boolean(
    a: &BTreeMap<String, String>,
    key: &str,
    source: &SchemaDeclarationNode,
) -> Result<bool, ValueContractError> {
    match a.get(key).map(String::as_str) {
        None | Some("false") => Ok(false),
        Some("true") => Ok(true),
        _ => Err(ValueContractError::at("invalid-boolean", source).field(key)),
    }
}
fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}
pub(crate) fn resolve_name(
    value: &str,
    input: &ValueContractSource,
    source: &SchemaDeclarationNode,
) -> Result<ContractName, ValueContractError> {
    let (namespace, local) = if let Some((prefix, local)) = value.split_once(':') {
        if !identifier(prefix) {
            return Err(ValueContractError::at("invalid-contract-name", source));
        }
        (
            input
                .bindings
                .get(prefix)
                .ok_or_else(|| ValueContractError::at("unknown-prefix", source))?,
            local,
        )
    } else {
        (&input.namespace, value)
    };
    if !identifier(local) {
        return Err(ValueContractError::at("invalid-contract-name", source));
    }
    Ok(ContractName::new(namespace, local))
}
