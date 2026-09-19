//! A bounded XSLT pattern grammar using native XPath syntax, never token rewrites.
use super::*;
use cem_ml::validation::{xml::XmlSourceRange, xpath::*};

pub(super) fn range(range: XmlSourceRange) -> XPathSourceRange {
    XPathSourceRange::new(
        range.start.line,
        range.start.column,
        range.start.byte_offset,
        range.byte_length,
    )
}

// Exact decimal ordering: priorities are not rounded through binary floating point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Priority {
    negative: bool,
    whole: String,
    fraction: String,
}
impl Priority {
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let negative = value.starts_with('-');
        let value = value.strip_prefix(['-', '+']).unwrap_or(value);
        let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
        if whole.is_empty() && fraction.is_empty()
            || !whole
                .bytes()
                .chain(fraction.bytes())
                .all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let whole = whole.trim_start_matches('0');
        let fraction = fraction.trim_end_matches('0');
        Some(Self {
            negative: negative && !(whole.is_empty() && fraction.is_empty()),
            whole: whole.into(),
            fraction: fraction.into(),
        })
    }
}
impl Ord for Priority {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.negative != other.negative {
            return other.negative.cmp(&self.negative);
        }
        let order = (self.whole.len(), &self.whole, &self.fraction).cmp(&(
            other.whole.len(),
            &other.whole,
            &other.fraction,
        ));
        if self.negative {
            order.reverse()
        } else {
            order
        }
    }
}
impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Compiler<'_> {
    pub(super) fn generated_xpath(
        &self,
        event: &XmlEventAst,
        code: &str,
    ) -> CompileResult<XPathExpressionAst> {
        let mut expression = self.authored_context(event);
        let context = match &expression.attachment {
            XPathAttachment::StandaloneStaticContext { static_context, .. } => {
                static_context.clone()
            }
            _ => unreachable!(),
        };
        let origin = range(event.source_range);
        expression = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: code.as_bytes(),
                source_uri: &self.stylesheet.xml_document.source.uri,
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            XPathAttachment::Host(XPathHostAttachment {
                owner: XPathHostOwner {
                    source_id: 1,
                    source_uri: self.stylesheet.xml_document.source.uri.clone(),
                    content_type: Some("application/xslt+xml".into()),
                    schema_uri: Some(XSLT_SCHEMA_URI.into()),
                    node_kind: XPathHostNodeKind::XsltAttribute,
                    node_id: Some(format!("generated:{}", event.index)),
                    source_range: origin,
                },
                expression_range: origin,
                static_context: context,
                expected_result: None,
                evaluation_phase: XPathEvaluationPhase::Transform,
                resolver_policy_stamp: None,
                safety_policy_stamp: None,
            }),
        );
        let syntax = expression
            .syntax_ast
            .as_mut()
            .ok_or_else(|| self.error(event, "cem.xslt.compile_xpath", "invalid compiler XPath"))?;
        xpath::anchor_sequence(&mut syntax.root, origin)
            .map_err(|message| self.error(event, "cem.xslt.compile_xpath", message))?;
        syntax.events.clear();
        expression.tokens.clear();
        expression.events.clear();
        expression.source_text = None;
        expression.facts.clear();
        Ok(expression)
    }
    pub(super) fn patterns(
        &self,
        event: &XmlEventAst,
    ) -> CompileResult<Vec<(XPathExpressionAst, Priority)>> {
        let attribute = self.attribute(event, "match").expect("match attribute");
        let source = self.required(event, "match")?;
        let mut attached = self.generated_xpath(event, "0")?;
        let XPathAttachment::Host(host) = &mut attached.attachment else {
            unreachable!()
        };
        host.expression_range = range(attribute.value_source_range.unwrap_or(event.source_range));
        let mut expression = xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: source.as_bytes(),
                source_uri: &self.stylesheet.xml_document.source.uri,
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: attribute
                    .entity_decoded_source_map
                    .as_ref()
                    .map(|p| p as &dyn cem_ml::source::SourceRangeProjector),
            },
            attached.attachment,
        );
        let syntax = expression.syntax_ast.take().ok_or_else(|| {
            self.error(
                event,
                "cem.xslt.pattern_unsupported",
                "invalid or unsupported match pattern",
            )
        })?;
        if let Some((code, range)) = grouping::group_function(&syntax.root) {
            let mut diagnostics = self.error(
                event,
                "cem.xslt.group_pattern",
                format!("err:{code}: group context functions cannot be used in match patterns"),
            );
            let diagnostic = &mut diagnostics[0];
            diagnostic.line = Some(range.start.line);
            diagnostic.column = Some(range.start.column);
            diagnostic.byte_offset = Some(range.start.byte_offset);
            if let Some(frame) = diagnostic
                .source_map
                .as_mut()
                .and_then(|map| map.frames.last_mut())
            {
                frame.span = cem_ml::source_map::FrameSpan::Single(cem_ml::source::ByteRange::new(
                    range.start.byte_offset,
                    range.byte_length as u32,
                ));
            }
            return Err(diagnostics);
        }
        let [root] = syntax.root.expressions.as_slice() else {
            return Err(self.error(
                event,
                "cem.xslt.pattern_unsupported",
                "pattern must be a path or union of paths",
            ));
        };
        let mut branches = Vec::new();
        fn branches_of(node: &XPathExpressionNode, output: &mut Vec<XPathExpressionNode>) {
            if let XPathExpression::Binary {
                operator: XPathBinaryOperator::Union,
                left,
                right,
            } = &node.expression
            {
                branches_of(left, output);
                branches_of(right, output);
            } else {
                output.push(node.clone());
            }
        }
        branches_of(root, &mut branches);
        let explicit =
            self.attribute(event, "priority")
                .map(|_| {
                    Priority::parse(self.required(event, "priority").unwrap_or_default())
                        .ok_or_else(|| {
                            self.error(
                                event,
                                "cem.xslt.priority_invalid",
                                "priority must be an exact decimal",
                            )
                        })
                })
                .transpose()?;
        let mut output = Vec::new();
        for mut branch in branches {
            let XPathExpression::Path(path) = &mut branch.expression else {
                return Err(self.error(
                    event,
                    "cem.xslt.pattern_unsupported",
                    "only node path and union patterns are supported",
                ));
            };
            let mut default = "0.5";
            if path.root == XPathPathRoot::Relative && path.steps.len() == 1 {
                if let XPathStep::Axis {
                    node_test,
                    predicates,
                    ..
                } = &path.steps[0].step
                {
                    if predicates.is_empty() {
                        default = match node_test {
                            XPathNodeTest::Name(XPathNameTest::Name(_)) => "0",
                            XPathNodeTest::Name(
                                XPathNameTest::AnyNamespace { .. }
                                | XPathNameTest::Namespace { .. },
                            ) => "-0.25",
                            XPathNodeTest::Kind {
                                processing_instruction_target: Some(_),
                                ..
                            } => "0",
                            _ => "-0.5",
                        };
                    }
                }
            }
            for step in &path.steps {
                if let XPathStep::Axis {
                    node_test: XPathNodeTest::Kind { kind, lexical, .. },
                    ..
                } = &step.step
                {
                    // The shared typed node test currently retains element/type
                    // arguments only as lexical metadata. Do not silently match
                    // every element for a parameterized element(name) pattern.
                    let bare = match kind {
                        XPathKindTest::Element => Some("element()"),
                        XPathKindTest::Attribute => Some("attribute()"),
                        XPathKindTest::Document => Some("document-node()"),
                        XPathKindTest::SchemaElement | XPathKindTest::SchemaAttribute => Some(""),
                        _ => None,
                    };
                    if bare
                        .is_some_and(|bare| lexical.split_whitespace().collect::<String>() != bare)
                    {
                        return Err(self.error(
                            event,
                            "cem.xslt.pattern_unsupported",
                            "parameterized element/document/type patterns are outside this profile",
                        ));
                    }
                }
                if !matches!(
                    step.step,
                    XPathStep::Axis {
                        axis: XPathAxis::Child | XPathAxis::Attribute | XPathAxis::DescendantOrSelf,
                        ..
                    }
                ) {
                    return Err(self.error(
                        event,
                        "cem.xslt.pattern_unsupported",
                        "only child/attribute path patterns are supported",
                    ));
                }
            }
            if path.root == XPathPathRoot::Relative {
                path.root = XPathPathRoot::RootedDescendant;
            }
            let origin = branch.source_range;
            let current = xpath::primary(XPathPrimaryExpression::ContextItem, origin);
            let condition = XPathExpressionNode {
                source_range: origin,
                expression: XPathExpression::InstanceOf {
                    operand: Box::new(current.clone()),
                    sequence_type: XPathSequenceType::Item {
                        item_type: XPathSequenceItemType::Kind {
                            kind: XPathKindTest::AnyNode,
                            lexical: "node()".into(),
                            source_range: origin,
                        },
                        occurrence: XPathOccurrenceIndicator::ExactlyOne,
                        source_range: origin,
                    },
                },
            };
            let selection = XPathExpressionNode {
                source_range: origin,
                expression: XPathExpression::Binary {
                    operator: XPathBinaryOperator::Intersect,
                    left: Box::new(current),
                    right: Box::new(branch),
                },
            };
            let body = XPathExpressionNode {
                source_range: origin,
                expression: XPathExpression::If {
                    condition: Box::new(XPathExpressionSequence {
                        expressions: vec![condition],
                        source_range: origin,
                    }),
                    then_expression: Box::new(selection),
                    else_expression: Box::new(xpath::primary(
                        XPathPrimaryExpression::Parenthesized(None),
                        origin,
                    )),
                },
            };
            let mut program = expression.clone();
            program.syntax_ast = Some(XPathSyntaxAst {
                root: XPathExpressionSequence {
                    expressions: vec![body],
                    source_range: origin,
                },
                events: vec![],
            });
            program.source_text = None;
            program.tokens.clear();
            program.events.clear();
            program.facts.clear();
            output.push((
                program,
                explicit
                    .clone()
                    .unwrap_or_else(|| Priority::parse(default).unwrap()),
            ));
        }
        Ok(output)
    }
}
