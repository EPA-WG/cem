use super::XPathSourceRange;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathSyntaxAst {
    pub root: XPathExpressionSequence,
    pub events: Vec<XPathSyntaxEvent>,
}

impl XPathSyntaxAst {
    pub(super) fn new(root: XPathExpressionSequence) -> Self {
        let mut events = Vec::new();
        root.emit_events(0, &mut events);
        for (index, event) in events.iter_mut().enumerate() {
            event.index = index;
        }
        Self { root, events }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathExpressionSequence {
    pub expressions: Vec<XPathExpressionNode>,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathExpressionNode {
    pub expression: XPathExpression,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathExpression {
    Path(XPathPathExpression),
    Unary {
        operator: XPathUnaryOperator,
        operand: Box<XPathExpressionNode>,
    },
    Binary {
        operator: XPathBinaryOperator,
        left: Box<XPathExpressionNode>,
        right: Box<XPathExpressionNode>,
    },
    SimpleMap {
        input: Box<XPathExpressionNode>,
        mappings: Vec<XPathExpressionNode>,
    },
    TreatAs {
        operand: Box<XPathExpressionNode>,
        sequence_type: XPathSequenceType,
    },
    InstanceOf {
        operand: Box<XPathExpressionNode>,
        sequence_type: XPathSequenceType,
    },
    For {
        binding: XPathName,
        binding_expression: Box<XPathExpressionNode>,
        return_expression: Box<XPathExpressionNode>,
    },
    Let {
        binding: XPathName,
        binding_expression: Box<XPathExpressionNode>,
        return_expression: Box<XPathExpressionNode>,
    },
    If {
        condition: Box<XPathExpressionSequence>,
        then_expression: Box<XPathExpressionNode>,
        else_expression: Box<XPathExpressionNode>,
    },
    Quantified {
        quantifier: XPathQuantifier,
        binding: XPathName,
        binding_expression: Box<XPathExpressionNode>,
        satisfies_expression: Box<XPathExpressionNode>,
    },
    Unsupported {
        production: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathQuantifier {
    Some,
    Every,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathUnaryOperator {
    Plus,
    Minus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathSequenceType {
    Empty {
        source_range: XPathSourceRange,
    },
    Item {
        item_type: XPathSequenceItemType,
        occurrence: XPathOccurrenceIndicator,
        source_range: XPathSourceRange,
    },
}

impl XPathSequenceType {
    pub fn source_range(&self) -> XPathSourceRange {
        match self {
            Self::Empty { source_range } | Self::Item { source_range, .. } => *source_range,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathSequenceItemType {
    AnyItem {
        source_range: XPathSourceRange,
    },
    Atomic(XPathName),
    Kind {
        kind: XPathKindTest,
        lexical: String,
        source_range: XPathSourceRange,
    },
    Parenthesized {
        item_type: Box<XPathSequenceItemType>,
        source_range: XPathSourceRange,
    },
    Unsupported {
        production: String,
        lexical: String,
        source_range: XPathSourceRange,
    },
}

impl XPathSequenceItemType {
    pub fn source_range(&self) -> XPathSourceRange {
        match self {
            Self::AnyItem { source_range }
            | Self::Kind { source_range, .. }
            | Self::Parenthesized { source_range, .. }
            | Self::Unsupported { source_range, .. } => *source_range,
            Self::Atomic(name) => name.source_range,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathOccurrenceIndicator {
    ExactlyOne,
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathPathExpression {
    pub root: XPathPathRoot,
    pub steps: Vec<XPathStepNode>,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathPathRoot {
    Relative,
    Rooted,
    RootedDescendant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathStepNode {
    pub step: XPathStep,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathStep {
    Axis {
        axis: XPathAxis,
        node_test: XPathNodeTest,
        predicates: Vec<XPathExpressionSequence>,
    },
    Primary(XPathPrimaryExpression),
    Postfix {
        primary: XPathPrimaryExpression,
        postfixes: Vec<XPathPostfixExpression>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathPrimaryExpression {
    Literal(XPathLiteral),
    VariableReference(XPathName),
    Parenthesized(Option<Box<XPathExpressionSequence>>),
    ContextItem,
    FunctionCall {
        name: XPathName,
        arguments: Vec<XPathExpressionNode>,
    },
    MapConstructor {
        entries: Vec<XPathMapConstructorEntry>,
    },
    ArrayConstructor(XPathArrayConstructor),
    Unsupported {
        production: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathMapConstructorEntry {
    pub key: XPathExpressionNode,
    pub value: XPathExpressionNode,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathArrayConstructor {
    Square(XPathExpressionSequence),
    Curly(Option<Box<XPathExpressionSequence>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathPostfixExpression {
    Predicate(XPathExpressionSequence),
    ArgumentList(Vec<XPathExpressionNode>),
    Lookup { lexical: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathName {
    pub lexical: String,
    pub prefix: Option<String>,
    pub local_name: String,
    pub namespace_uri: Option<String>,
    pub source_range: XPathSourceRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathLiteralKind {
    Integer,
    Decimal,
    Double,
    String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathLiteral {
    pub kind: XPathLiteralKind,
    pub lexical: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathAxis {
    Ancestor,
    AncestorOrSelf,
    Attribute,
    Child,
    Descendant,
    DescendantOrSelf,
    Following,
    FollowingSibling,
    Namespace,
    Parent,
    Preceding,
    PrecedingSibling,
    SelfAxis,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathNodeTest {
    Name(XPathNameTest),
    Kind {
        kind: XPathKindTest,
        lexical: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XPathNameTest {
    Name(XPathName),
    Any,
    AnyNamespace { local_name: String },
    Namespace { namespace_uri: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathKindTest {
    Document,
    Element,
    Attribute,
    SchemaElement,
    SchemaAttribute,
    ProcessingInstruction,
    Comment,
    Text,
    NamespaceNode,
    AnyNode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathBinaryOperator {
    Or,
    And,
    ValueEqual,
    ValueNotEqual,
    ValueLessThan,
    ValueLessThanOrEqual,
    ValueGreaterThan,
    ValueGreaterThanOrEqual,
    GeneralEqual,
    GeneralNotEqual,
    GeneralLessThan,
    GeneralLessThanOrEqual,
    GeneralGreaterThan,
    GeneralGreaterThanOrEqual,
    NodeIs,
    NodePrecedes,
    NodeFollows,
    Concatenate,
    Range,
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
    Modulo,
    Union,
    Intersect,
    Except,
    Sequence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathSyntaxEventKind {
    StartNode,
    EndNode,
}

impl XPathSyntaxEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartNode => "start-node",
            Self::EndNode => "end-node",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XPathSyntaxNodeKind {
    ExpressionSequence,
    PathExpression,
    UnaryExpression,
    BinaryExpression,
    SimpleMapExpression,
    TreatAsExpression,
    InstanceOfExpression,
    SequenceType,
    ForExpression,
    LetExpression,
    IfExpression,
    QuantifiedExpression,
    UnsupportedExpression,
    AxisStep,
    PrimaryStep,
    PostfixStep,
    Predicate,
    Literal,
    VariableReference,
    ParenthesizedExpression,
    ContextItem,
    FunctionCall,
    MapConstructor,
    MapEntry,
    ArrayConstructor,
    UnsupportedPrimary,
    ArgumentList,
    Lookup,
}

impl XPathSyntaxNodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExpressionSequence => "expression-sequence",
            Self::PathExpression => "path-expression",
            Self::UnaryExpression => "unary-expression",
            Self::BinaryExpression => "binary-expression",
            Self::SimpleMapExpression => "simple-map-expression",
            Self::TreatAsExpression => "treat-as-expression",
            Self::InstanceOfExpression => "instance-of-expression",
            Self::SequenceType => "sequence-type",
            Self::ForExpression => "for-expression",
            Self::LetExpression => "let-expression",
            Self::IfExpression => "if-expression",
            Self::QuantifiedExpression => "quantified-expression",
            Self::UnsupportedExpression => "unsupported-expression",
            Self::AxisStep => "axis-step",
            Self::PrimaryStep => "primary-step",
            Self::PostfixStep => "postfix-step",
            Self::Predicate => "predicate",
            Self::Literal => "literal",
            Self::VariableReference => "variable-reference",
            Self::ParenthesizedExpression => "parenthesized-expression",
            Self::ContextItem => "context-item",
            Self::FunctionCall => "function-call",
            Self::MapConstructor => "map-constructor",
            Self::MapEntry => "map-entry",
            Self::ArrayConstructor => "array-constructor",
            Self::UnsupportedPrimary => "unsupported-primary",
            Self::ArgumentList => "argument-list",
            Self::Lookup => "lookup",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XPathSyntaxEvent {
    pub index: usize,
    pub kind: XPathSyntaxEventKind,
    pub node_kind: XPathSyntaxNodeKind,
    pub depth: usize,
    pub source_range: XPathSourceRange,
}

fn emit_node(
    kind: XPathSyntaxNodeKind,
    source_range: XPathSourceRange,
    depth: usize,
    events: &mut Vec<XPathSyntaxEvent>,
    children: impl FnOnce(usize, &mut Vec<XPathSyntaxEvent>),
) {
    events.push(XPathSyntaxEvent {
        index: 0,
        kind: XPathSyntaxEventKind::StartNode,
        node_kind: kind,
        depth,
        source_range,
    });
    children(depth + 1, events);
    events.push(XPathSyntaxEvent {
        index: 0,
        kind: XPathSyntaxEventKind::EndNode,
        node_kind: kind,
        depth,
        source_range,
    });
}

impl XPathExpressionSequence {
    fn emit_events(&self, depth: usize, events: &mut Vec<XPathSyntaxEvent>) {
        emit_node(
            XPathSyntaxNodeKind::ExpressionSequence,
            self.source_range,
            depth,
            events,
            |depth, events| {
                for expression in &self.expressions {
                    expression.emit_events(depth, events);
                }
            },
        );
    }
}

impl XPathExpressionNode {
    fn emit_events(&self, depth: usize, events: &mut Vec<XPathSyntaxEvent>) {
        let kind = match &self.expression {
            XPathExpression::Path(_) => XPathSyntaxNodeKind::PathExpression,
            XPathExpression::Unary { .. } => XPathSyntaxNodeKind::UnaryExpression,
            XPathExpression::Binary { .. } => XPathSyntaxNodeKind::BinaryExpression,
            XPathExpression::SimpleMap { .. } => XPathSyntaxNodeKind::SimpleMapExpression,
            XPathExpression::TreatAs { .. } => XPathSyntaxNodeKind::TreatAsExpression,
            XPathExpression::InstanceOf { .. } => XPathSyntaxNodeKind::InstanceOfExpression,
            XPathExpression::For { .. } => XPathSyntaxNodeKind::ForExpression,
            XPathExpression::Let { .. } => XPathSyntaxNodeKind::LetExpression,
            XPathExpression::If { .. } => XPathSyntaxNodeKind::IfExpression,
            XPathExpression::Quantified { .. } => XPathSyntaxNodeKind::QuantifiedExpression,
            XPathExpression::Unsupported { .. } => XPathSyntaxNodeKind::UnsupportedExpression,
        };
        emit_node(
            kind,
            self.source_range,
            depth,
            events,
            |depth, events| match &self.expression {
                XPathExpression::Path(path) => path.emit_children(depth, events),
                XPathExpression::Unary { operand, .. } => operand.emit_events(depth, events),
                XPathExpression::Binary { left, right, .. } => {
                    left.emit_events(depth, events);
                    right.emit_events(depth, events);
                }
                XPathExpression::SimpleMap { input, mappings } => {
                    input.emit_events(depth, events);
                    for mapping in mappings {
                        mapping.emit_events(depth, events);
                    }
                }
                XPathExpression::TreatAs {
                    operand,
                    sequence_type,
                }
                | XPathExpression::InstanceOf {
                    operand,
                    sequence_type,
                } => {
                    operand.emit_events(depth, events);
                    emit_leaf(
                        XPathSyntaxNodeKind::SequenceType,
                        sequence_type.source_range(),
                        depth,
                        events,
                    );
                }
                XPathExpression::For {
                    binding,
                    binding_expression,
                    return_expression,
                }
                | XPathExpression::Let {
                    binding,
                    binding_expression,
                    return_expression,
                } => {
                    emit_leaf(
                        XPathSyntaxNodeKind::VariableReference,
                        binding.source_range,
                        depth,
                        events,
                    );
                    binding_expression.emit_events(depth, events);
                    return_expression.emit_events(depth, events);
                }
                XPathExpression::If {
                    condition,
                    then_expression,
                    else_expression,
                } => {
                    condition.emit_events(depth, events);
                    then_expression.emit_events(depth, events);
                    else_expression.emit_events(depth, events);
                }
                XPathExpression::Quantified {
                    binding,
                    binding_expression,
                    satisfies_expression,
                    ..
                } => {
                    emit_leaf(
                        XPathSyntaxNodeKind::VariableReference,
                        binding.source_range,
                        depth,
                        events,
                    );
                    binding_expression.emit_events(depth, events);
                    satisfies_expression.emit_events(depth, events);
                }
                XPathExpression::Unsupported { .. } => {}
            },
        );
    }
}

impl XPathPathExpression {
    fn emit_children(&self, depth: usize, events: &mut Vec<XPathSyntaxEvent>) {
        for step in &self.steps {
            step.emit_events(depth, events);
        }
    }
}

impl XPathStepNode {
    fn emit_events(&self, depth: usize, events: &mut Vec<XPathSyntaxEvent>) {
        let kind = match &self.step {
            XPathStep::Axis { .. } => XPathSyntaxNodeKind::AxisStep,
            XPathStep::Primary(_) => XPathSyntaxNodeKind::PrimaryStep,
            XPathStep::Postfix { .. } => XPathSyntaxNodeKind::PostfixStep,
        };
        emit_node(
            kind,
            self.source_range,
            depth,
            events,
            |depth, events| match &self.step {
                XPathStep::Axis { predicates, .. } => {
                    for predicate in predicates {
                        emit_node(
                            XPathSyntaxNodeKind::Predicate,
                            predicate.source_range,
                            depth,
                            events,
                            |depth, events| predicate.emit_events(depth, events),
                        );
                    }
                }
                XPathStep::Primary(primary) => {
                    primary.emit_events(self.source_range, depth, events)
                }
                XPathStep::Postfix { primary, postfixes } => {
                    primary.emit_events(self.source_range, depth, events);
                    for postfix in postfixes {
                        postfix.emit_events(self.source_range, depth, events);
                    }
                }
            },
        );
    }
}

impl XPathPrimaryExpression {
    fn emit_events(
        &self,
        source_range: XPathSourceRange,
        depth: usize,
        events: &mut Vec<XPathSyntaxEvent>,
    ) {
        let kind = match self {
            Self::Literal(_) => XPathSyntaxNodeKind::Literal,
            Self::VariableReference(_) => XPathSyntaxNodeKind::VariableReference,
            Self::Parenthesized(_) => XPathSyntaxNodeKind::ParenthesizedExpression,
            Self::ContextItem => XPathSyntaxNodeKind::ContextItem,
            Self::FunctionCall { .. } => XPathSyntaxNodeKind::FunctionCall,
            Self::MapConstructor { .. } => XPathSyntaxNodeKind::MapConstructor,
            Self::ArrayConstructor(_) => XPathSyntaxNodeKind::ArrayConstructor,
            Self::Unsupported { .. } => XPathSyntaxNodeKind::UnsupportedPrimary,
        };
        emit_node(
            kind,
            source_range,
            depth,
            events,
            |depth, events| match self {
                Self::Parenthesized(Some(expression)) => expression.emit_events(depth, events),
                Self::FunctionCall { arguments, .. } => {
                    for argument in arguments {
                        argument.emit_events(depth, events);
                    }
                }
                Self::MapConstructor { entries } => {
                    for entry in entries {
                        emit_node(
                            XPathSyntaxNodeKind::MapEntry,
                            entry.source_range,
                            depth,
                            events,
                            |depth, events| {
                                entry.key.emit_events(depth, events);
                                entry.value.emit_events(depth, events);
                            },
                        );
                    }
                }
                Self::ArrayConstructor(XPathArrayConstructor::Square(expression)) => {
                    expression.emit_events(depth, events)
                }
                Self::ArrayConstructor(XPathArrayConstructor::Curly(Some(expression))) => {
                    expression.emit_events(depth, events)
                }
                _ => {}
            },
        );
    }
}

impl XPathPostfixExpression {
    fn emit_events(
        &self,
        source_range: XPathSourceRange,
        depth: usize,
        events: &mut Vec<XPathSyntaxEvent>,
    ) {
        match self {
            Self::Predicate(expression) => emit_node(
                XPathSyntaxNodeKind::Predicate,
                expression.source_range,
                depth,
                events,
                |depth, events| expression.emit_events(depth, events),
            ),
            Self::ArgumentList(arguments) => emit_node(
                XPathSyntaxNodeKind::ArgumentList,
                source_range,
                depth,
                events,
                |depth, events| {
                    for argument in arguments {
                        argument.emit_events(depth, events);
                    }
                },
            ),
            Self::Lookup { .. } => {
                emit_leaf(XPathSyntaxNodeKind::Lookup, source_range, depth, events)
            }
        }
    }
}

fn emit_leaf(
    kind: XPathSyntaxNodeKind,
    source_range: XPathSourceRange,
    depth: usize,
    events: &mut Vec<XPathSyntaxEvent>,
) {
    emit_node(kind, source_range, depth, events, |_, _| {});
}
