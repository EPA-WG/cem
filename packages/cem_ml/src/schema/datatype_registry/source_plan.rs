//! Source classification only. The shared lifecycle compiler consumes these
//! retained dependency fields later; planning evaluates nothing and owns no budget.
use super::DatatypeSource;
use crate::{parser::CemAstNode, schema::declaration_references::SchemaDeclarationNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatatypeKind {
    Scalar,
    Lexical,
    List,
    Grammar,
    Reference,
    Node,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatatypeKindSource {
    Explicit(DatatypeKind),
    /// Resolve the inherited base before classifying this declaration.
    Inherited,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatatypeDependencyRole {
    InheritedBase,
    /// Explicit whole-list inheritance; never a list item selection.
    InheritedList,
    ListItem,
    ValidationRule,
}

#[derive(Debug, Clone)]
pub enum DatatypeDependencyValue {
    /// Compiler metadata, interpreted under the original declaration's aliases.
    Literal(String),
    /// Original reference node, consumed only by the shared lifecycle traversal.
    Native(SchemaDeclarationNode),
}

#[derive(Debug, Clone)]
pub struct DatatypeDependency {
    pub role: DatatypeDependencyRole,
    pub attribute: SchemaDeclarationNode,
    pub value: DatatypeDependencyValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatatypePlanIssueKind {
    MissingKind,
    InvalidKind,
    InvalidDependency,
    UnsupportedFacet,
}

#[derive(Debug, Clone)]
pub struct DatatypePlanIssue {
    pub kind: DatatypePlanIssueKind,
    pub source: SchemaDeclarationNode,
}

/// An available plan is not a ready datatype. Base/rule targets, registration,
/// signatures, unknown source fields and effective facets still need compilation.
#[derive(Debug, Clone)]
pub struct DatatypeSourcePlan {
    pub source: DatatypeSource,
    pub kind: DatatypeKindSource,
    pub dependencies: Vec<DatatypeDependency>,
    /// Original nontrivia child slots. Only the explicit enumeration consumer
    /// may select references and admit retained constant declarations.
    pub constant_slots: Vec<SchemaDeclarationNode>,
    pub descriptive_rule: Option<SchemaDeclarationNode>,
    pub issues: Vec<DatatypePlanIssue>,
}

impl DatatypeSource {
    pub fn plan(&self) -> DatatypeSourcePlan {
        let mut plan = DatatypeSourcePlan {
            source: self.clone(),
            kind: DatatypeKindSource::Missing,
            dependencies: Vec::new(),
            constant_slots: Vec::new(),
            descriptive_rule: None,
            issues: Vec::new(),
        };
        if let CemAstNode::Element { children, .. } = self.declaration().node() {
            plan.constant_slots = children
                .iter()
                .filter_map(|id| {
                    let child = SchemaDeclarationNode::new(
                        self.declaration().document().clone(),
                        *id,
                    )?;
                    match child.node() {
                        CemAstNode::Whitespace { .. } | CemAstNode::Comment { .. } => None,
                        CemAstNode::Text { data, .. } if data.trim().is_empty() => None,
                        _ => Some(child),
                    }
                })
                .collect();
        }
        plan.kind = match self.attribute("kind") {
            Some(attribute) => match scalar_field(attribute).and_then(parse_kind) {
                Some(kind) => DatatypeKindSource::Explicit(kind),
                None => {
                    plan.issue(DatatypePlanIssueKind::InvalidKind, attribute);
                    DatatypeKindSource::Invalid
                }
            },
            None if self.attribute("base").is_some() || self.attribute("list-base").is_some() => {
                DatatypeKindSource::Inherited
            }
            None => {
                plan.issue(DatatypePlanIssueKind::MissingKind, self.declaration());
                DatatypeKindSource::Missing
            }
        };
        if let Some(attribute) = self.attribute("base") {
            let role = if plan.kind == DatatypeKindSource::Explicit(DatatypeKind::List) {
                DatatypeDependencyRole::ListItem
            } else {
                DatatypeDependencyRole::InheritedBase
            };
            plan.dependency(attribute, role);
        }
        if let Some(attribute) = self.attribute("list-base") {
            if self.attribute("base").is_some()
                || matches!(plan.kind, DatatypeKindSource::Explicit(kind) if kind != DatatypeKind::List)
            {
                plan.issue(DatatypePlanIssueKind::InvalidDependency, attribute);
            }
            plan.dependency(attribute, DatatypeDependencyRole::InheritedList);
        }
        if let Some(attribute) = self.attribute("rule") {
            if scalar_field(attribute).is_some() {
                plan.descriptive_rule = Some(attribute.clone());
            } else {
                plan.dependency(attribute, DatatypeDependencyRole::ValidationRule);
            }
        }
        if matches!(
            plan.kind,
            DatatypeKindSource::Explicit(DatatypeKind::List | DatatypeKind::Node)
        ) {
            if let Some(attribute) = self.attribute("values") {
                plan.issue(DatatypePlanIssueKind::UnsupportedFacet, attribute);
            }
        }
        if plan.kind == DatatypeKindSource::Explicit(DatatypeKind::Node) {
            if let Some(attribute) = self.attribute("pattern") {
                plan.issue(DatatypePlanIssueKind::UnsupportedFacet, attribute);
            }
        }
        plan
    }
}

impl DatatypeSourcePlan {
    fn issue(&mut self, kind: DatatypePlanIssueKind, source: &SchemaDeclarationNode) {
        self.issues.push(DatatypePlanIssue {
            kind,
            source: source.clone(),
        });
    }

    fn dependency(&mut self, attribute: &SchemaDeclarationNode, role: DatatypeDependencyRole) {
        let value = if let Some(literal) =
            scalar_field(attribute).filter(|value| !value.trim().is_empty())
        {
            // A literal rule is descriptive, not a behavior dependency.
            if role == DatatypeDependencyRole::ValidationRule {
                self.issue(DatatypePlanIssueKind::InvalidDependency, attribute);
                return;
            }
            DatatypeDependencyValue::Literal(literal.trim().to_owned())
        } else {
            let CemAstNode::Attribute { value_nodes, .. } = attribute.node() else {
                self.issue(DatatypePlanIssueKind::InvalidDependency, attribute);
                return;
            };
            let reference = match value_nodes.as_slice() {
                [id] if matches!(
                    attribute.document().get(*id),
                    Some(CemAstNode::Reference { .. })
                ) =>
                {
                    SchemaDeclarationNode::new(attribute.document().clone(), *id).unwrap()
                }
                _ => {
                    self.issue(DatatypePlanIssueKind::InvalidDependency, attribute);
                    return;
                }
            };
            DatatypeDependencyValue::Native(reference)
        };
        self.dependencies.push(DatatypeDependency {
            role,
            attribute: attribute.clone(),
            value,
        });
    }
}

fn scalar_field(attribute: &SchemaDeclarationNode) -> Option<&str> {
    match attribute.node() {
        CemAstNode::Attribute {
            value, value_nodes, ..
        } if value_nodes.is_empty() => value.as_deref(),
        _ => None,
    }
}

fn parse_kind(value: &str) -> Option<DatatypeKind> {
    match value.trim() {
        "scalar" => Some(DatatypeKind::Scalar),
        "lexical" => Some(DatatypeKind::Lexical),
        "list" => Some(DatatypeKind::List),
        "grammar" => Some(DatatypeKind::Grammar),
        "reference" => Some(DatatypeKind::Reference),
        "node" => Some(DatatypeKind::Node),
        _ => None,
    }
}
