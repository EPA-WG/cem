//! Inert native CEM-QL source inspection. Typed parser nodes go directly to the
//! shared CEM projection writer; no query evaluation or document-parser fallback.
use super::*;
use cem_ml::{
    engine::{EngineInput, SourceInspection, SourceInspectionStage},
    projection::{
        projection_attribute as attr, projection_node, projection_vocabulary_document,
        CemTreeAstNode, CemTreeAstStream,
    },
    query::CemQlQuerySourceKind,
    schema::registry::{
        CEM_AST_PROJECTION_SCHEMA_URI, CEM_EVENTS_PROJECTION_SCHEMA_URI,
        CEM_QL_EXPRESSION_CONTENT_TYPE, CEM_QL_EXPRESSION_SCHEMA_URI,
    },
    source_map::{FrameSpan, SourceMapFrame},
};
use cem_ql::parser::{
    Expression, FunctionParam, PathStep, PipelineStep, QName, RecordEntry, RecordKey, TypeExpr,
};

#[derive(Debug)]
pub(super) struct CemQlSourceInspection;

impl SourceInspectionStage for CemQlSourceInspection {
    fn inspect(&self, input: &EngineInput) -> Option<SourceInspection> {
        let identity = input
            .identity
            .clone()
            .unwrap_or_else(|| input.root_scope.format_identity());
        let claims_ql = identity
            .content_type
            .as_deref()
            .map(content_type_essence)
            .is_some_and(|media| {
                matches!(
                    media.as_str(),
                    CEM_QL_CONTENT_TYPE | "text/cem-ql" | CEM_QL_EXPRESSION_CONTENT_TYPE
                )
            })
            || matches!(
                identity.schema.as_deref(),
                Some(CEM_QL_SCHEMA_URI | CEM_QL_EXPRESSION_SCHEMA_URI)
            );
        if !claims_ql {
            return None;
        }
        let kind = CemQlQuerySourceKind::from_identity(&identity);
        let mut diagnostics = Vec::new();
        let parsed = if kind.is_none() {
            diagnostics.push(cem_ql_query_diagnostic(
                &input.uri,
                "cem.ql.query_identity_unsupported",
                "query source content type and schema must agree",
            ));
            None
        } else if let Ok(source) = std::str::from_utf8(&input.bytes) {
            let parsed = if kind == Some(CemQlQuerySourceKind::Expression) {
                cem_ql::api::parse_expression(source)
            } else {
                cem_ql::api::parse(source)
            };
            if kind == Some(CemQlQuerySourceKind::Module) {
                diagnostics.extend(cem_ql_parse_diagnostics(&input.uri, &parsed));
            } else {
                diagnostics.extend(parsed.diagnostics.clone());
            }
            Some(Arc::new(parsed))
        } else {
            diagnostics.push(cem_ql_invalid_utf8_diagnostic(&input.uri));
            None
        };
        finish_cem_ql_source_validation_diagnostics(&input.uri, &input.bytes, &mut diagnostics);
        let schema = identity.schema.clone().unwrap_or_else(|| {
            kind.map(|k| k.schema_uri())
                .unwrap_or(CEM_QL_SCHEMA_URI)
                .into()
        });
        let media = identity.content_type.clone().unwrap_or_else(|| {
            if kind == Some(CemQlQuerySourceKind::Expression) {
                CEM_QL_EXPRESSION_CONTENT_TYPE
            } else {
                CEM_QL_CONTENT_TYPE
            }
            .into()
        });
        let mut builder = Builder {
            uri: &input.uri,
            media: &media,
            source: parsed
                .as_ref()
                .map(|p| p.module.source.as_str())
                .unwrap_or(""),
            nodes: Vec::new(),
            offsets: Vec::new(),
        };
        if let Some(parsed) = &parsed {
            builder.walk(&parsed.module.nodes);
        }
        let ast = document("ast", &schema, &media, &input.uri, builder.nodes, false);
        let events = parsed
            .as_ref()
            .map(|parsed| {
                Lexer::new(&parsed.module.source)
                    .scan_all()
                    .into_iter()
                    .enumerate()
                    .map(|(i, token)| {
                        let source = source_map(&input.uri, &media, token.range);
                        let kind = match token.kind {
                            TokenKind::Whitespace
                            | TokenKind::LineComment
                            | TokenKind::BlockComment => "trivia",
                            TokenKind::Invalid => "error",
                            TokenKind::Ident | TokenKind::PrefixedName => "name",
                            TokenKind::EndOfInput => "close",
                            _ => "value",
                        };
                        CemTreeAstNode::Element {
                            name: "event".into(),
                            attributes: vec![
                                attr("sequence", i.to_string(), &source),
                                attr("kind", kind, &source),
                                attr("name", format!("{:?}", token.kind), &source),
                                attr(
                                    "value",
                                    parsed
                                        .module
                                        .source
                                        .get(token.range.start as usize..token.range.end() as usize)
                                        .unwrap_or(""),
                                    &source,
                                ),
                                attr("source-id", &input.uri, &source),
                                attr("byte-offset", token.range.start.to_string(), &source),
                                attr("byte-length", token.range.len.to_string(), &source),
                            ],
                            children: Vec::new(),
                            source,
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(SourceInspection {
            identity: FormatIdentity {
                content_type: Some(media.clone()),
                schema: Some(schema.clone()),
                ..Default::default()
            },
            ast: parsed.as_ref().map(|parsed| retain_source(parsed, ast)),
            events: parsed.as_ref().map(|parsed| {
                retain_source(
                    parsed,
                    document("event-stream", &schema, &media, &input.uri, events, true),
                )
            }),
            offsets: builder.offsets,
            diagnostics,
        })
    }
}

// Both structural and lexical projections retain the native source through writing.
fn retain_source(parsed: &Arc<ParseResult>, stream: CemTreeAstStream) -> CemTreeAstStream {
    CemTreeAstStream::from_native_values(
        Arc::new((parsed.clone(), stream.as_nodes().to_vec())),
        |owner| owner.1.clone(),
    )
}

fn source_map(uri: &str, media: &str, range: ByteRange) -> SourceMapStack {
    SourceMapStack {
        frames: vec![
            SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(range),
                transform: TransformKind::SourceBoundary { uri: uri.into() },
            },
            SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(range),
                transform: TransformKind::ContentTypeTransform {
                    content_type: media.into(),
                },
            },
        ],
    }
}
fn document(
    name: &str,
    schema: &str,
    media: &str,
    uri: &str,
    children: Vec<CemTreeAstNode>,
    events: bool,
) -> CemTreeAstStream {
    let source = source_map(uri, media, ByteRange::new(0, 0));
    let mut attributes = vec![
        attr("schema", schema, &source),
        attr("content-type", media, &source),
        attr("hash-scheme", "blake3", &source),
    ];
    if !events {
        attributes.push(attr("source-id", uri, &source));
    }
    let root = CemTreeAstNode::Element {
        name: name.into(),
        attributes,
        children,
        source: source.clone(),
    };
    projection_vocabulary_document(
        if events { "cemevents" } else { "cemast" },
        if events {
            CEM_EVENTS_PROJECTION_SCHEMA_URI
        } else {
            CEM_AST_PROJECTION_SCHEMA_URI
        },
        root,
        source,
    )
}

#[derive(Clone, Copy)]
enum Node<'a> {
    Surface(&'a SurfaceNode),
    Expr(&'a Expression),
    Param(&'a FunctionParam),
    Path(&'a PathStep),
    Step(&'a PipelineStep),
    Entry(&'a RecordEntry),
    Key(&'a RecordKey),
    Name(&'a QName),
    Type(&'a TypeExpr),
}
struct Builder<'a> {
    uri: &'a str,
    media: &'a str,
    source: &'a str,
    nodes: Vec<CemTreeAstNode>,
    offsets: Vec<ByteRange>,
}
impl Builder<'_> {
    fn walk(&mut self, nodes: &[SurfaceNode]) {
        let mut pending: Vec<_> = nodes
            .iter()
            .enumerate()
            .rev()
            .map(|(order, node)| (Node::Surface(node), None::<usize>, order, "root"))
            .collect();
        while let Some((node, parent, order, role)) = pending.pop() {
            let mut children = Vec::new();
            let mut value = None;
            let (kind, range) = match node {
                Node::Surface(SurfaceNode::Expression(e)) => {
                    pending.push((Node::Expr(e), parent, order, role));
                    continue;
                }
                Node::Surface(SurfaceNode::Module(d)) => {
                    value = Some(d.uri.clone());
                    ("module", d.range)
                }
                Node::Surface(SurfaceNode::Import(d)) => {
                    value = Some(d.uri.clone());
                    ("import", d.range)
                }
                Node::Surface(SurfaceNode::DeclareVariable(d)) => {
                    children.extend([
                        ("name", Node::Name(&d.name)),
                        ("value", Node::Expr(&d.value)),
                    ]);
                    ("declare-variable", d.range)
                }
                Node::Surface(SurfaceNode::DeclareFunction(d)) => {
                    children.push(("name", Node::Name(&d.name)));
                    children.extend(d.params.iter().map(|p| ("parameter", Node::Param(p))));
                    children.push(("body", Node::Expr(&d.body)));
                    ("declare-function", d.range)
                }
                Node::Name(n) => {
                    value = Some(
                        n.prefix
                            .as_ref()
                            .map(|p| format!("{p}:{}", n.local))
                            .unwrap_or_else(|| n.local.clone()),
                    );
                    ("name", n.range)
                }
                Node::Type(t) => {
                    children.push(("name", Node::Name(&t.name)));
                    ("type", t.range)
                }
                Node::Param(p) => {
                    children.push(("name", Node::Name(&p.name)));
                    let mut range = p.name.range;
                    if let Some(t) = &p.type_annotation {
                        children.push(("type", Node::Type(t)));
                        range.len = (t.range.end() - range.start) as u32;
                    }
                    ("parameter", range)
                }
                Node::Key(RecordKey::Static { value: v, range }) => {
                    value = Some(v.clone());
                    ("static-key", *range)
                }
                Node::Key(RecordKey::Computed { expr, range }) => {
                    children.push(("expression", Node::Expr(expr)));
                    ("computed-key", *range)
                }
                Node::Entry(e) => {
                    children.extend([("key", Node::Key(&e.key)), ("value", Node::Expr(&e.value))]);
                    ("entry", e.range)
                }
                Node::Path(PathStep::Parent(r)) => ("parent", *r),
                Node::Path(PathStep::Self_(r)) => ("self", *r),
                Node::Path(PathStep::Axis {
                    axis,
                    name_test,
                    predicates,
                    range,
                }) => {
                    value = Some(format!(
                        "{axis:?}:{}:{}",
                        name_test.prefix.as_deref().unwrap_or(""),
                        name_test.local.as_deref().unwrap_or("*")
                    ));
                    children.extend(predicates.iter().map(|p| ("predicate", Node::Expr(p))));
                    ("axis", *range)
                }
                Node::Step(PipelineStep::Named {
                    name,
                    args,
                    called,
                    range,
                }) => {
                    value = Some(called.to_string());
                    children.push(("name", Node::Name(name)));
                    children.extend(args.iter().map(|a| ("argument", Node::Expr(a))));
                    ("pipeline-step", *range)
                }
                Node::Step(PipelineStep::Lambda { lambda, range }) => {
                    children.push(("lambda", Node::Expr(lambda)));
                    ("pipeline-lambda", *range)
                }
                Node::Expr(e) => {
                    let kind = match e {
                        Expression::Literal(_, r) => {
                            value = self
                                .source
                                .get(r.start as usize..r.end() as usize)
                                .map(str::to_owned);
                            "literal"
                        }
                        Expression::Name(n, _) => {
                            children.push(("name", Node::Name(n)));
                            "reference"
                        }
                        Expression::LeadingDot(_) => "leading-dot",
                        Expression::Path { steps, .. } => {
                            children.extend(steps.iter().map(|s| ("step", Node::Path(s))));
                            "path"
                        }
                        Expression::Pipeline { source, steps, .. } => {
                            children.push(("source", Node::Expr(source)));
                            children.extend(steps.iter().map(|s| ("step", Node::Step(s))));
                            "pipeline"
                        }
                        Expression::BinaryOp { op, lhs, rhs, .. } => {
                            value = Some(op.symbol().into());
                            children.extend([("lhs", Node::Expr(lhs)), ("rhs", Node::Expr(rhs))]);
                            "binary"
                        }
                        Expression::SetOp { op, lhs, rhs, .. } => {
                            value = Some(format!("{op:?}"));
                            children.extend([("lhs", Node::Expr(lhs)), ("rhs", Node::Expr(rhs))]);
                            "set"
                        }
                        Expression::UnaryOp { op, operand, .. } => {
                            value = Some(format!("{op:?}"));
                            children.push(("operand", Node::Expr(operand)));
                            "unary"
                        }
                        Expression::TryCatch {
                            body,
                            code,
                            message,
                            handler,
                            ..
                        } => {
                            children.extend([
                                ("body", Node::Expr(body)),
                                ("code", Node::Name(code)),
                                ("message", Node::Name(message)),
                                ("handler", Node::Expr(handler)),
                            ]);
                            "try-catch"
                        }
                        Expression::If {
                            cond,
                            then_branch,
                            else_branch,
                            ..
                        } => {
                            children.extend([
                                ("condition", Node::Expr(cond)),
                                ("then", Node::Expr(then_branch)),
                                ("else", Node::Expr(else_branch)),
                            ]);
                            "if"
                        }
                        Expression::Let {
                            name, value, body, ..
                        } => {
                            children.extend([
                                ("binding", Node::Name(name)),
                                ("value", Node::Expr(value)),
                                ("body", Node::Expr(body)),
                            ]);
                            "let"
                        }
                        Expression::For {
                            var, source, body, ..
                        } => {
                            children.extend([
                                ("binding", Node::Name(var)),
                                ("source", Node::Expr(source)),
                                ("body", Node::Expr(body)),
                            ]);
                            "for"
                        }
                        Expression::Quantified {
                            kind,
                            var,
                            source,
                            predicate,
                            ..
                        } => {
                            value = Some(format!("{kind:?}"));
                            children.extend([
                                ("binding", Node::Name(var)),
                                ("source", Node::Expr(source)),
                                ("predicate", Node::Expr(predicate)),
                            ]);
                            "quantified"
                        }
                        Expression::Record { entries, .. } => {
                            children.extend(entries.iter().map(|e| ("entry", Node::Entry(e))));
                            "record"
                        }
                        Expression::Sequence { items, .. } => {
                            children.extend(items.iter().map(|e| ("item", Node::Expr(e))));
                            "sequence"
                        }
                        Expression::Call { callee, args, .. } => {
                            children.push(("callee", Node::Expr(callee)));
                            children.extend(args.iter().map(|e| ("argument", Node::Expr(e))));
                            "call"
                        }
                        Expression::Lambda { params, body, .. } => {
                            children.extend(params.iter().map(|p| ("parameter", Node::Param(p))));
                            children.push(("body", Node::Expr(body)));
                            "lambda"
                        }
                        Expression::InstanceOf { value, ty, .. }
                        | Expression::CastAs { value, ty, .. }
                        | Expression::TreatAs { value, ty, .. } => {
                            children
                                .extend([("value", Node::Expr(value)), ("type", Node::Type(ty))]);
                            match e {
                                Expression::InstanceOf { .. } => "instance-of",
                                Expression::CastAs { .. } => "cast-as",
                                _ => "treat-as",
                            }
                        }
                    };
                    (kind, e.range())
                }
            };
            let name = match node {
                Node::Surface(SurfaceNode::Import(import)) => {
                    import.alias.as_deref().unwrap_or(role)
                }
                _ => role,
            };
            let id = self.nodes.len();
            self.nodes.push(projection_node(
                &id.to_string(),
                kind,
                parent.map(|p| p.to_string()).as_deref(),
                order,
                Some(name),
                value.as_deref(),
                self.uri,
                Some((range.start, u64::from(range.len))),
                source_map(self.uri, self.media, range),
            ));
            self.offsets.push(range);
            pending.extend(
                children
                    .into_iter()
                    .enumerate()
                    .rev()
                    .map(|(order, (role, child))| (child, Some(id), order, role)),
            );
        }
    }
}
