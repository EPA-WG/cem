//! Retained source signatures for explicitly registered datatype validators.
//! Compilation performs no lookup, execution, reference resolution or scope grant.
use super::{
    datatype_registry::DatatypeKind,
    declaration_references::SchemaDeclarationNode,
    registry::CEM_SCHEMA_URI,
    value_contracts::{
        self as fields, Cardinality, ContractName, ValueContractError, ValueContractSource,
    },
};
use crate::parser::CemAstNode;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarRepresentation {
    String,
    Boolean,
    Integer,
    Decimal,
    Double,
    AnyUri,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueRepresentation {
    Scalar(ScalarRepresentation),
    List(ScalarRepresentation),
    Nodes,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateRequirement {
    Optional,
    Required,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultRepresentation {
    Accepted(ContractName),
    Diagnostics(ContractName),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSignature {
    pub kind: DatatypeKind,
    pub value: ValueRepresentation,
    pub candidate: CandidateRequirement,
    pub result: ResultRepresentation,
}
#[derive(Debug, Clone)]
pub enum ValidationImplementation {
    Native(String),
    Query {
        function: SchemaDeclarationNode,
        body: String,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BehaviorProfile {
    Validation,
    Conversion,
}
#[derive(Debug, Clone)]
pub struct DatatypeBehaviorContract {
    owner: SchemaDeclarationNode,
    behavior: SchemaDeclarationNode,
    signature: ValidationSignature,
    implementation: ValidationImplementation,
}
impl DatatypeBehaviorContract {
    pub fn owner(&self) -> &SchemaDeclarationNode {
        &self.owner
    }
    pub fn behavior(&self) -> &SchemaDeclarationNode {
        &self.behavior
    }
    pub fn signature(&self) -> &ValidationSignature {
        &self.signature
    }
    pub fn implementation(&self) -> &ValidationImplementation {
        &self.implementation
    }
    /// The registration's signature is checked against original, captured source
    /// names and every declared input. Missing roles are never inferred by position.
    pub fn compile(
        source: &ValueContractSource,
        behavior: &SchemaDeclarationNode,
        signature: ValidationSignature,
    ) -> Result<Self, ValueContractError> {
        Self::compile_profile(source, behavior, signature, BehaviorProfile::Validation)
    }
    pub(super) fn compile_profile(
        source: &ValueContractSource,
        behavior: &SchemaDeclarationNode,
        signature: ValidationSignature,
        profile: BehaviorProfile,
    ) -> Result<Self, ValueContractError> {
        let fail = |code| ValueContractError::at(code, behavior);
        if !source.named(&source.schema, "schema")
            || !Arc::ptr_eq(source.schema.document(), behavior.document())
        {
            return Err(fail("unrelated-behavior-owner"));
        }
        if source
            .schema
            .document()
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(fail("invalid-source-document"));
        }
        let mut member = false;
        for collection in fields::elements(&source.schema)? {
            if source.named(&collection, "behaviors") {
                for child in fields::elements(&collection)? {
                    member |= child.identity() == behavior.identity();
                }
            }
        }
        if !member || !source.named(behavior, "behavior") {
            return Err(fail("unowned-behavior"));
        }
        if profile == BehaviorProfile::Validation {
            match (signature.kind, signature.value) {
                (DatatypeKind::List, ValueRepresentation::List(_))
                | (DatatypeKind::Node, ValueRepresentation::Nodes) => {}
                (
                    DatatypeKind::Scalar
                    | DatatypeKind::Lexical
                    | DatatypeKind::Grammar
                    | DatatypeKind::Reference,
                    ValueRepresentation::Scalar(_),
                ) => {}
                _ => return Err(fail("incompatible-kind-representation")),
            }
        }
        let behavior_attrs = fields::attrs(
            source,
            behavior,
            &[
                "name",
                "implementation",
                "execution",
                "primitive",
                "function",
            ],
        )?;
        if fields::required(&behavior_attrs, "name", behavior)?
            .trim()
            .is_empty()
            || fields::required(&behavior_attrs, "execution", behavior)?
                != match profile {
                    BehaviorProfile::Validation => "datatype-validation",
                    BehaviorProfile::Conversion => "datatype-conversion",
                }
        {
            return Err(fail(match profile {
                BehaviorProfile::Validation => "invalid-validation-behavior",
                BehaviorProfile::Conversion => "invalid-conversion-behavior",
            }));
        }
        let children = fields::elements(behavior)?;
        let mut inputs = None;
        let mut result = None;
        let mut functions = Vec::new();
        for child in children {
            if source.named(&child, "inputs") && inputs.is_none() {
                inputs = Some(child);
            } else if source.named(&child, "result") && result.is_none() {
                result = Some(child);
            } else if source.named(&child, "function") {
                functions.push(child);
            } else {
                return Err(ValueContractError::at(
                    match profile {
                        BehaviorProfile::Validation => "unsupported-validation-child",
                        BehaviorProfile::Conversion => "unsupported-conversion-child",
                    },
                    &child,
                ));
            }
        }
        let inputs = inputs.ok_or_else(|| fail("missing-inputs"))?;
        fields::attrs(source, &inputs, &[])?;
        let mut roles = BTreeMap::new();
        for input in fields::elements(&inputs)? {
            if !source.named(&input, "input-binding") {
                return Err(ValueContractError::at("invalid-input", &input));
            }
            let a = fields::attrs(
                source,
                &input,
                &["name", "type", "source", "required", "cardinality"],
            )?;
            let name = fields::required(&a, "name", &input)?;
            if fields::required(&a, "source", &input)? != name {
                return Err(ValueContractError::at("input-role-mismatch", &input));
            }
            check_role(source, &input, &a, &signature)?;
            if roles.insert(name, input.clone()).is_some() {
                return Err(ValueContractError::at("duplicate-input", &input));
            }
            if !fields::elements(&input)?.is_empty() {
                return Err(ValueContractError::at("unexpected-input-child", &input));
            }
        }
        if roles.len() != 3 {
            return Err(fail("missing-input-role"));
        }
        let result = result.ok_or_else(|| fail("missing-result"))?;
        let a = fields::attrs(source, &result, &["type", "cardinality"])?;
        let name = fields::resolve_name(&fields::required(&a, "type", &result)?, source, &result)?;
        let (expected, card) = match &signature.result {
            ResultRepresentation::Accepted(name) => (name, Cardinality::One),
            ResultRepresentation::Diagnostics(name) => (name, Cardinality::ZeroOrMore),
        };
        if &name != expected
            || cardinality(&a, &result)? != card
            || !fields::elements(&result)?.is_empty()
        {
            return Err(ValueContractError::at("result-signature-mismatch", &result));
        }
        let implementation = match fields::required(&behavior_attrs, "implementation", behavior)?
            .as_str()
        {
            "engine" if behavior_attrs.get("function").is_none() && functions.is_empty() => {
                let id = fields::required(&behavior_attrs, "primitive", behavior)?;
                if id.trim().is_empty() {
                    return Err(fail("missing-implementation-id"));
                }
                ValidationImplementation::Native(id)
            }
            "function" if behavior_attrs.get("primitive").is_none() && functions.len() == 1 => {
                let function = functions.remove(0);
                let fa = fields::attrs(source, &function, &["name", "returns", "deterministic"])?;
                if fields::required(&fa, "name", &function)?
                    != fields::required(&behavior_attrs, "function", behavior)?
                {
                    return Err(fail("function-binding-mismatch"));
                }
                let expected = if profile == BehaviorProfile::Conversion {
                    "datatype-conversion-result"
                } else {
                    match signature.result {
                        ResultRepresentation::Accepted(_) => "datatype-validation-result",
                        ResultRepresentation::Diagnostics(_) => "diagnostic-sequence",
                    }
                };
                if fields::required(&fa, "returns", &function)? != expected {
                    return Err(ValueContractError::at(
                        "function-result-mismatch",
                        &function,
                    ));
                }
                if fa.contains_key("deterministic") {
                    fields::boolean(&fa, "deterministic", &function)?;
                }
                let mut params = BTreeMap::new();
                let mut body = None;
                for child in fields::elements(&function)? {
                    if source.named(&child, "param") {
                        let pa = fields::attrs(
                            source,
                            &child,
                            &["name", "type", "required", "cardinality"],
                        )?;
                        check_role(source, &child, &pa, &signature)?;
                        let name = fields::required(&pa, "name", &child)?;
                        if params.insert(name, child.clone()).is_some()
                            || !fields::elements(&child)?.is_empty()
                        {
                            return Err(ValueContractError::at("invalid-function-param", &child));
                        }
                    } else if source.named(&child, "body") && body.is_none() {
                        body = Some(expression(source, &child)?);
                    } else {
                        return Err(ValueContractError::at("unsupported-function-child", &child));
                    }
                }
                if params.len() != 3 {
                    return Err(ValueContractError::at("missing-function-role", &function));
                }
                ValidationImplementation::Query {
                    body: body.ok_or_else(|| fail("missing-function-body"))?,
                    function,
                }
            }
            _ => return Err(fail("invalid-implementation-binding")),
        };
        Ok(Self {
            owner: source.schema.clone(),
            behavior: behavior.clone(),
            signature,
            implementation,
        })
    }
}
fn cardinality(
    a: &BTreeMap<String, String>,
    source: &SchemaDeclarationNode,
) -> Result<Cardinality, ValueContractError> {
    match a.get("cardinality").map(String::as_str).unwrap_or("one") {
        "one" => Ok(Cardinality::One),
        "zero-or-one" => Ok(Cardinality::ZeroOrOne),
        "zero-or-more" => Ok(Cardinality::ZeroOrMore),
        "one-or-more" => Ok(Cardinality::OneOrMore),
        _ => Err(ValueContractError::at("invalid-cardinality", source)),
    }
}
fn check_role(
    source: &ValueContractSource,
    node: &SchemaDeclarationNode,
    a: &BTreeMap<String, String>,
    signature: &ValidationSignature,
) -> Result<(), ValueContractError> {
    let name = fields::required(a, "name", node)?;
    let (ty, required, card) = match name.as_str() {
        "value" => match signature.value {
            ValueRepresentation::Scalar(p) => (primitive(p), true, Cardinality::One),
            ValueRepresentation::List(p) => (primitive(p), true, Cardinality::ZeroOrMore),
            ValueRepresentation::Nodes => ("node", true, Cardinality::ZeroOrMore),
        },
        "datatype" => ("node", true, Cardinality::One),
        "candidate" => match signature.candidate {
            CandidateRequirement::Optional => ("node", false, Cardinality::ZeroOrOne),
            CandidateRequirement::Required => ("node", true, Cardinality::One),
        },
        _ => return Err(ValueContractError::at("unknown-input-role", node)),
    };
    let authored = fields::required(a, "type", node)?;
    let matches = if authored.contains(':') {
        fields::resolve_name(&authored, source, node)? == ContractName::new(CEM_SCHEMA_URI, ty)
    } else {
        authored == ty
    };
    if !matches
        || fields::boolean(a, "required", node)? != required
        || cardinality(a, node)? != card
    {
        return Err(ValueContractError::at("input-signature-mismatch", node));
    }
    Ok(())
}
fn primitive(p: ScalarRepresentation) -> &'static str {
    match p {
        ScalarRepresentation::String => "string",
        ScalarRepresentation::Boolean => "boolean",
        ScalarRepresentation::Integer => "integer",
        ScalarRepresentation::Decimal => "decimal",
        ScalarRepresentation::Double => "double",
        ScalarRepresentation::AnyUri => "uri",
    }
}
fn expression(
    source: &ValueContractSource,
    body: &SchemaDeclarationNode,
) -> Result<String, ValueContractError> {
    fields::attrs(source, body, &[])?;
    let children = fields::elements(body)?;
    if children.len() != 1 {
        return Err(ValueContractError::at("single-expression-required", body));
    }
    let CemAstNode::Element {
        expanded_name,
        attributes,
        children,
        ..
    } = children[0].node()
    else {
        unreachable!()
    };
    if expanded_name.local_name != "$" || !attributes.is_empty() {
        return Err(ValueContractError::at("expression-required", body));
    }
    let mut expression = String::new();
    for id in children {
        match body.document().get(*id) {
            Some(CemAstNode::Text { data, .. } | CemAstNode::RawText { data, .. }) => {
                expression.push_str(data)
            }
            _ => {
                return Err(ValueContractError::at(
                    "unsupported-expression-content",
                    body,
                ))
            }
        }
    }
    if expression.trim().is_empty() {
        return Err(ValueContractError::at("empty-expression", body));
    }
    Ok(expression)
}
