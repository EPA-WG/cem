//! Pipeline iterator-chain adapters.

use std::collections::BTreeMap;
use std::path::Path;

use cem_ml::diagnostics::Severity;
use cem_ml::events::cem::CemEventNormalizer;
use cem_ml::parser::builder::CemAstBuilder;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::CemAstNode;
use cem_ml::source::{BytesSource, SourceId};
use cem_ml::tokenizer::cem::CemTokenizer;

use crate::diagnostics::{
    POLICY_ACCESSOR_FAILED, READ_DENIED, READ_UNSATISFIABLE, UNRESOLVED_REFERENCE,
};
use crate::eval::{effective_boolean, first_integer, AtomValue, EvalCtx, Item, ItemStream};
use crate::ir::{IrId, IrStep};
use crate::resolve::ModuleUri;
use crate::stdlib;
use crate::{parser::QName, parser::SetOp};

pub(crate) fn apply_pipeline(
    source: ItemStream,
    steps: &[IrStep],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    let mut stream = source;
    for step in steps {
        stream = apply_step(stream, step, ctx);
        if stream.error.is_some() {
            break;
        }
    }
    stream
}

fn apply_step(input: ItemStream, step: &IrStep, ctx: &mut EvalCtx<'_>) -> ItemStream {
    match step {
        IrStep::Lambda(lambda) => apply_lambda_step(input, *lambda, ctx),
        IrStep::Named {
            binding,
            name,
            args,
        } => {
            let Some(binding) = binding else {
                return apply_builtin_step(input, name, args, ctx);
            };
            let mut out = ItemStream::empty();
            for item in input.items {
                let mut call_args = vec![ItemStream::once(item)];
                call_args.extend(ctx.eval_arg_streams(args));
                let stream = ctx.invoke_function(
                    *binding,
                    call_args,
                    args.first().copied().unwrap_or(IrId(0)),
                );
                out.append_stream(stream);
                if out.error.is_some() {
                    break;
                }
            }
            out
        }
        IrStep::NamedStdlib { module, name, args } => {
            apply_stdlib_step(input, module, name, args, ctx)
        }
    }
}

fn apply_lambda_step(input: ItemStream, lambda: IrId, ctx: &mut EvalCtx<'_>) -> ItemStream {
    let mut out = ItemStream::empty();
    out.diagnostics.extend(input.diagnostics);
    out.error = input.error;
    for item in input.items {
        let stream = ctx.with_current_item(item, |ctx| ctx.invoke_lambda(lambda, Vec::new()));
        out.append_stream(stream);
        if out.error.is_some() {
            break;
        }
    }
    out
}

fn apply_builtin_step(
    input: ItemStream,
    name: &QName,
    args: &[IrId],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    match name.local.as_str() {
        "first" => first(input),
        "last" => last(input),
        "take" => take(input, ctx.eval_arg_streams(args).first()),
        "drop" => drop(input, ctx.eval_arg_streams(args).first()),
        "nth" => nth(input, ctx.eval_arg_streams(args).first()),
        "target" => resolve_ref(input, ctx, args.first().copied().unwrap_or(IrId(0))),
        "where" => where_step(input, args.first().copied(), ctx),
        _ => record_field(input, &name.local).unwrap_or_else(|| {
            ctx.unknown_function(
                args.first().copied().unwrap_or(IrId(0)),
                "unknown pipeline step",
            )
        }),
    }
}

fn apply_stdlib_step(
    input: ItemStream,
    module: &ModuleUri,
    name: &QName,
    args: &[IrId],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    if module.0 == "cem:stdlib/sequence" {
        return apply_builtin_step(input, name, args, ctx);
    }
    ctx.unknown_function(
        args.first().copied().unwrap_or(IrId(0)),
        "unknown stdlib pipeline step",
    )
}

pub(crate) fn apply_stdlib_call(
    module: &ModuleUri,
    name: &QName,
    args: &[IrId],
    ctx: &mut EvalCtx<'_>,
) -> ItemStream {
    let arg_streams = ctx.eval_arg_streams(args);
    let source = args.first().copied().unwrap_or(IrId(0));
    if stdlib::ModuleRegistry::with_all_known()
        .resolve(&module.0, &name.local, args.len())
        .is_none()
    {
        return ctx.unknown_function(source, "unknown stdlib call");
    }
    match (module.0.as_str(), name.local.as_str()) {
        ("cem:stdlib/sequence", "first") => {
            first(arg_streams.into_iter().next().unwrap_or_default())
        }
        ("cem:stdlib/sequence", "last") => last(arg_streams.into_iter().next().unwrap_or_default()),
        ("cem:stdlib/sequence", "take") => {
            let mut args = arg_streams.into_iter();
            let source = args.next().unwrap_or_default();
            let n = args.next();
            take(source, n.as_ref())
        }
        ("cem:stdlib/sequence", "drop") => {
            let mut args = arg_streams.into_iter();
            let source = args.next().unwrap_or_default();
            let n = args.next();
            drop(source, n.as_ref())
        }
        ("cem:stdlib/sequence", "nth") => {
            let mut args = arg_streams.into_iter();
            let source = args.next().unwrap_or_default();
            let n = args.next();
            nth(source, n.as_ref())
        }
        ("cem:stdlib/sequence", "union") => binary_set_call(SetOp::Union, arg_streams, ctx, args),
        ("cem:stdlib/sequence", "intersect") => {
            binary_set_call(SetOp::Intersect, arg_streams, ctx, args)
        }
        ("cem:stdlib/sequence", "difference") => {
            binary_set_call(SetOp::Difference, arg_streams, ctx, args)
        }
        ("cem:stdlib/sequence", "symmetric_difference") => {
            binary_set_call(SetOp::SymmetricDifference, arg_streams, ctx, args)
        }
        ("cem:stdlib/sequence", "map") => callable_sequence(arg_streams, ctx, args, false, false),
        ("cem:stdlib/sequence", "flat_map") => {
            callable_sequence(arg_streams, ctx, args, true, false)
        }
        ("cem:stdlib/sequence", "where") => callable_sequence(arg_streams, ctx, args, false, true),
        ("cem:stdlib/sequence", "peek") => arg_streams.into_iter().next().unwrap_or_default(),
        ("cem:stdlib/strings", "length") => {
            let value = first_string(&arg_streams);
            ItemStream::once(Item::Atomic(AtomValue::Integer(
                value.chars().count() as i64
            )))
        }
        ("cem:stdlib/strings", "codepoints") => {
            let value = first_string(&arg_streams);
            ItemStream::from_items(
                value
                    .chars()
                    .map(|c| Item::Atomic(AtomValue::Integer(c as i64)))
                    .collect(),
            )
        }
        ("cem:stdlib/strings", "lower") => {
            let value = first_string(&arg_streams);
            ItemStream::once(Item::Atomic(AtomValue::String(value.to_lowercase())))
        }
        ("cem:stdlib/strings", "upper") => {
            let value = first_string(&arg_streams);
            ItemStream::once(Item::Atomic(AtomValue::String(value.to_uppercase())))
        }
        ("cem:stdlib/strings", "slice") => string_slice(arg_streams),
        ("cem:stdlib/strings", "concat") => string_concat(arg_streams),
        ("cem:stdlib/strings", "contains") => {
            string_predicate(arg_streams, |left, right| left.contains(right))
        }
        ("cem:stdlib/strings", "starts_with") => {
            string_predicate(arg_streams, |left, right| left.starts_with(right))
        }
        ("cem:stdlib/strings", "ends_with") => {
            string_predicate(arg_streams, |left, right| left.ends_with(right))
        }
        ("cem:stdlib/numbers", "double") => number_double(arg_streams),
        ("cem:stdlib/numbers", "decimal") => number_decimal(arg_streams),
        ("cem:stdlib/numbers", "integer") => number_integer(arg_streams),
        ("cem:stdlib/numbers", "string") => {
            ItemStream::once(Item::Atomic(AtomValue::String(first_string(&arg_streams))))
        }
        ("cem:stdlib/numbers", "abs") => number_unary(arg_streams, f64::abs),
        ("cem:stdlib/numbers", "floor") => number_unary(arg_streams, f64::floor),
        ("cem:stdlib/numbers", "ceil") => number_unary(arg_streams, f64::ceil),
        ("cem:stdlib/numbers", "round") => number_unary(arg_streams, f64::round),
        ("cem:stdlib/numbers", "format") => number_format(arg_streams),
        ("cem:stdlib/datetime", "to_utc") => ItemStream::once(Item::Atomic(AtomValue::String(
            normalize_utc(&first_string(&arg_streams)),
        ))),
        ("cem:stdlib/datetime", "components") => datetime_components(first_string(&arg_streams)),
        ("cem:stdlib/datetime", "format") => {
            ItemStream::once(Item::Atomic(AtomValue::String(first_string(&arg_streams))))
        }
        ("cem:stdlib/dom", "tainted") => ItemStream::once(Item::Atomic(AtomValue::Boolean(false))),
        ("cem:stdlib/dom", "children")
        | ("cem:stdlib/dom", "descendants")
        | ("cem:stdlib/dom", "parent")
        | ("cem:stdlib/dom", "attribute")
        | ("cem:stdlib/state", "read")
        | ("cem:stdlib/state", "keys")
        | ("cem:stdlib/template", "lookup")
        | ("cem:stdlib/template", "names") => ItemStream::empty(),
        ("cem:stdlib/dom", "resolve_ref") => resolve_ref(
            arg_streams.into_iter().next().unwrap_or_default(),
            ctx,
            source,
        ),
        ("cem:stdlib/report", "emit") => report_emit(arg_streams, ctx, source),
        ("cem:stdlib/report", "severity_floor") => ItemStream::empty(),
        ("cem:stdlib/cemml", "parse") => cemml_parse(arg_streams),
        ("cem:stdlib/cemml", "format") => {
            ItemStream::once(Item::Atomic(AtomValue::String(first_string(&arg_streams))))
        }
        ("cem:stdlib/content-types", "read") => read_resource(arg_streams, ctx, source),
        ("cem:stdlib/content-types", "html") => content_type("text/html"),
        ("cem:stdlib/content-types", "xml") => content_type("application/xml"),
        ("cem:stdlib/content-types", "svg") => content_type("image/svg+xml"),
        ("cem:stdlib/content-types", "mathml") => content_type("application/mathml+xml"),
        ("cem:stdlib/content-types", "css") => content_type("text/css"),
        ("cem:stdlib/content-types", "scss") => content_type("text/x-scss"),
        ("cem:stdlib/content-types", "json") => content_type("application/json"),
        ("cem:stdlib/content-types", "yaml") => content_type("application/yaml"),
        ("cem:stdlib/content-types", "csv") => content_type("text/csv"),
        ("cem:stdlib/content-types", "js") => content_type("application/javascript"),
        ("cem:stdlib/content-types", "ts") => content_type("application/typescript"),
        ("cem:stdlib/content-types", "cemml") => content_type("application/cem+xml"),
        ("cem:stdlib/content-types", "floor") | ("cem:stdlib/content-types", "default_accepts") => {
            ItemStream::from_items(
                CONTENT_TYPE_FLOOR
                    .iter()
                    .map(|media_type| Item::Atomic(AtomValue::String((*media_type).to_owned())))
                    .collect(),
            )
        }
        ("cem:stdlib/user", "has_role") => user_has_role(arg_streams, ctx, source),
        _ => ctx.unknown_function(source, "unknown stdlib call"),
    }
}

const CONTENT_TYPE_FLOOR: &[&str] = &[
    "text/html",
    "application/xml",
    "image/svg+xml",
    "application/mathml+xml",
    "text/css",
    "text/x-scss",
    "application/json",
    "application/yaml",
    "text/csv",
    "application/javascript",
    "application/typescript",
    "application/cem+xml",
];

fn content_type(value: &str) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::String(value.to_owned())))
}

fn first(mut input: ItemStream) -> ItemStream {
    input.items.truncate(1);
    input
}

fn last(mut input: ItemStream) -> ItemStream {
    if let Some(item) = input.items.pop() {
        input.items = vec![item];
    }
    input
}

fn take(mut input: ItemStream, n: Option<&ItemStream>) -> ItemStream {
    let n = n.and_then(first_integer).unwrap_or(0).max(0) as usize;
    input.items.truncate(n);
    input
}

fn drop(mut input: ItemStream, n: Option<&ItemStream>) -> ItemStream {
    let n = n.and_then(first_integer).unwrap_or(0).max(0) as usize;
    input.items = input.items.into_iter().skip(n).collect();
    input
}

fn nth(mut input: ItemStream, n: Option<&ItemStream>) -> ItemStream {
    let n = n.and_then(first_integer).unwrap_or(1).max(1) as usize;
    input.items = input
        .items
        .into_iter()
        .nth(n.saturating_sub(1))
        .into_iter()
        .collect();
    input
}

fn record_field(input: ItemStream, field: &str) -> Option<ItemStream> {
    if !input
        .items
        .iter()
        .any(|item| matches!(item, Item::Record(_)))
    {
        return None;
    }
    let mut out = ItemStream::empty();
    out.diagnostics.extend(input.diagnostics);
    out.error = input.error;
    for item in input.items {
        if let Item::Record(record) = item {
            if let Some(values) = record.get(field) {
                out.items.extend(values.clone());
            }
        }
    }
    Some(out)
}

fn user_has_role(arg_streams: Vec<ItemStream>, ctx: &mut EvalCtx<'_>, source: IrId) -> ItemStream {
    let Some(Item::Resource(resource)) =
        arg_streams.first().and_then(|stream| stream.items.first())
    else {
        return ItemStream::once(Item::Atomic(AtomValue::Boolean(false)));
    };
    if resource.fail_accessor {
        return ctx.fail_diagnostic(
            source,
            POLICY_ACCESSOR_FAILED,
            format!("policy accessor failed for resource `{}`", resource.id),
            "policy accessor failed",
        );
    }
    let role = arg_streams
        .get(1)
        .and_then(|stream| stream.items.first())
        .and_then(item_string)
        .unwrap_or_default();
    ItemStream::once(Item::Atomic(AtomValue::Boolean(
        resource.roles.iter().any(|candidate| candidate == &role),
    )))
}

fn read_resource(
    mut arg_streams: Vec<ItemStream>,
    ctx: &mut EvalCtx<'_>,
    source: IrId,
) -> ItemStream {
    let uri = first_string(&arg_streams);
    if !uri.starts_with("file://") {
        return ctx.fail_diagnostic(
            source,
            READ_DENIED,
            format!("read `{uri}` denied by scope policy"),
            "read denied",
        );
    }

    let path = &uri["file://".len()..];
    let wire_type = content_type_from_path(path);
    let accepts = arg_streams
        .get_mut(1)
        .map(resolve_accepts)
        .unwrap_or_else(|| {
            CONTENT_TYPE_FLOOR
                .iter()
                .map(|value| (*value).to_owned())
                .collect()
        });
    let Some(selected) = accepts
        .iter()
        .find(|accept| transform_reachable(&wire_type, accept))
        .cloned()
    else {
        return ctx.fail_diagnostic(
            source,
            READ_UNSATISFIABLE,
            format!(
                "read `{uri}` wire type `{wire_type}` cannot satisfy accepts [{}]",
                accepts.join(", ")
            ),
            "read unsatisfiable",
        );
    };

    let path_display = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path);
    ItemStream::once(Item::Node(format!("{selected}:{path_display}")))
}

fn resolve_accepts(stream: &mut ItemStream) -> Vec<String> {
    if stream.items.is_empty()
        || matches!(stream.items.first(), Some(Item::Atomic(AtomValue::Null)))
    {
        return CONTENT_TYPE_FLOOR
            .iter()
            .map(|value| (*value).to_owned())
            .collect();
    }

    if stream.items.len() == 1 {
        if let Some(header) = stream.items.first().and_then(item_string) {
            if header.contains(',') || header.contains(";q=") || header.contains('*') {
                return parse_accept_header(&header);
            }
        }
    }

    let accepts = stream
        .items
        .iter()
        .filter_map(item_string)
        .flat_map(|value| expand_accept_range(&normalize_content_type(&value)))
        .collect::<Vec<_>>();
    if accepts.is_empty() {
        CONTENT_TYPE_FLOOR
            .iter()
            .map(|value| (*value).to_owned())
            .collect()
    } else {
        accepts
    }
}

fn parse_accept_header(header: &str) -> Vec<String> {
    let mut entries = header
        .split(',')
        .enumerate()
        .filter_map(|(index, part)| {
            let mut segments = part.split(';').map(str::trim);
            let media_range = normalize_content_type(segments.next()?);
            let mut q = 1.0;
            for segment in segments {
                if let Some(raw) = segment.strip_prefix("q=") {
                    q = raw.parse::<f64>().unwrap_or(0.0);
                }
            }
            Some((index, q, media_range))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|(left_index, left_q, _), (right_index, right_q, _)| {
        right_q
            .partial_cmp(left_q)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left_index.cmp(right_index))
    });
    entries
        .into_iter()
        .flat_map(|(_, _, media_range)| expand_accept_range(&media_range))
        .collect()
}

fn expand_accept_range(media_range: &str) -> Vec<String> {
    match media_range {
        "*/*" => CONTENT_TYPE_FLOOR
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        range if range.ends_with("/*") => {
            let prefix = range.trim_end_matches('*');
            CONTENT_TYPE_FLOOR
                .iter()
                .filter(|value| value.starts_with(prefix))
                .map(|value| (*value).to_owned())
                .collect()
        }
        value => vec![value.to_owned()],
    }
}

fn content_type_from_path(path: &str) -> String {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "html" | "htm" => "text/html",
        "xml" => "application/xml",
        "svg" => "image/svg+xml",
        "mml" | "mathml" => "application/mathml+xml",
        "css" => "text/css",
        "scss" => "text/x-scss",
        "json" => "application/json",
        "yaml" | "yml" => "application/yaml",
        "csv" => "text/csv",
        "js" | "mjs" => "application/javascript",
        "ts" => "application/typescript",
        "cem" | "cemml" => "application/cem+xml",
        _ => "application/octet-stream",
    }
    .to_owned()
}

fn normalize_content_type(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "text/xml" => "application/xml",
        "text/json" => "application/json",
        "text/yaml" | "application/x-yaml" => "application/yaml",
        "text/scss" => "text/x-scss",
        "text/javascript" => "application/javascript",
        "text/typescript" | "application/x-typescript" => "application/typescript",
        other => other,
    }
    .to_owned()
}

fn transform_reachable(wire_type: &str, accept: &str) -> bool {
    wire_type == accept || (wire_type == "application/yaml" && accept == "application/json")
}

fn resolve_ref(input: ItemStream, ctx: &mut EvalCtx<'_>, source: IrId) -> ItemStream {
    let mut out = ItemStream::empty();
    out.diagnostics.extend(input.diagnostics);
    out.error = input.error;
    for item in input.items {
        match resolve_first_reference(&item) {
            ReferenceResolution::Resolved(target) => out.items.push(Item::Node(target)),
            ReferenceResolution::Unresolved(target_name) => {
                let diagnostic = ctx.emit_diagnostic(
                    source,
                    UNRESOLVED_REFERENCE,
                    format!("reference `{target_name}` did not match any element id"),
                    Severity::Warning,
                );
                out.extend_diagnostics(diagnostic);
            }
            ReferenceResolution::NoReference => {}
        }
    }
    out
}

enum ReferenceResolution {
    Resolved(String),
    Unresolved(String),
    NoReference,
}

fn resolve_first_reference(item: &Item) -> ReferenceResolution {
    let Some(source) = item_node_text(item) else {
        return ReferenceResolution::NoReference;
    };
    let doc = parse_cem_fragment(source);
    for node in doc.iter() {
        let CemAstNode::Element { attributes, .. } = node else {
            continue;
        };
        for attr_id in attributes {
            let Some(CemAstNode::Attribute {
                expanded_name,
                value: Some(target_name),
                ..
            }) = doc.get(*attr_id)
            else {
                continue;
            };
            if !matches!(
                expanded_name.local_name.as_str(),
                "for" | "aria-labelledby" | "aria-describedby" | "aria-controls"
            ) {
                continue;
            }
            return cem_ml::query::find_by_id(&doc, target_name)
                .map(|target| describe_node(&doc, target))
                .map(ReferenceResolution::Resolved)
                .unwrap_or_else(|| ReferenceResolution::Unresolved(target_name.clone()));
        }
    }
    ReferenceResolution::NoReference
}

fn item_node_text(item: &Item) -> Option<&str> {
    match item {
        Item::Node(value) => Some(value.as_str()),
        Item::Atomic(AtomValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

fn parse_cem_fragment(source: &str) -> CemDocument {
    let src = BytesSource::new(SourceId(1), source.as_bytes().to_vec());
    let tokenizer = CemTokenizer::from_source(src);
    let normalizer = CemEventNormalizer::new(tokenizer);
    CemAstBuilder::new(normalizer).build()
}

fn describe_node(doc: &CemDocument, node: &CemAstNode) -> String {
    let CemAstNode::Element {
        expanded_name,
        attributes,
        ..
    } = node
    else {
        return format!("{node:?}");
    };
    let id = attributes
        .iter()
        .find_map(|attr_id| match doc.get(*attr_id) {
            Some(CemAstNode::Attribute {
                expanded_name,
                value: Some(value),
                ..
            }) if expanded_name.local_name == "id" => Some(value.clone()),
            _ => None,
        });
    match id {
        Some(id) => format!("{}#{id}", expanded_name.local_name),
        None => expanded_name.local_name.clone(),
    }
}

fn where_step(input: ItemStream, predicate: Option<IrId>, ctx: &mut EvalCtx<'_>) -> ItemStream {
    let Some(predicate) = predicate else {
        return input;
    };
    let mut out = ItemStream::empty();
    out.diagnostics.extend(input.diagnostics);
    out.error = input.error;
    for item in input.items {
        let predicate_stream = ctx.with_current_item(item.clone(), |ctx| ctx.eval_id(predicate));
        if effective_boolean(&predicate_stream.items) {
            out.items.push(item);
        }
        out.extend_diagnostics(predicate_stream);
        if out.error.is_some() {
            break;
        }
    }
    out
}

fn binary_set_call(
    op: SetOp,
    arg_streams: Vec<ItemStream>,
    ctx: &mut EvalCtx<'_>,
    arg_ids: &[IrId],
) -> ItemStream {
    let mut args = arg_streams.into_iter();
    let lhs = args.next().unwrap_or_default();
    let rhs = args.next().unwrap_or_default();
    crate::eval::set_ops::apply_set_op(
        op,
        lhs,
        rhs,
        ctx,
        arg_ids.first().copied().unwrap_or(IrId(0)),
    )
}

fn item_string(item: &Item) -> Option<String> {
    match item {
        Item::Atomic(AtomValue::String(value)) | Item::Atomic(AtomValue::AnyUri(value)) => {
            Some(value.clone())
        }
        Item::Atomic(AtomValue::Integer(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Decimal(value)) => Some(value.clone()),
        Item::Atomic(AtomValue::Double(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Boolean(value)) => Some(value.to_string()),
        Item::Atomic(AtomValue::Null) => Some("null".to_owned()),
        Item::Node(value) => Some(value.clone()),
        _ => None,
    }
}

fn callable_sequence(
    mut arg_streams: Vec<ItemStream>,
    ctx: &mut EvalCtx<'_>,
    arg_ids: &[IrId],
    flatten: bool,
    filter: bool,
) -> ItemStream {
    let source = arg_streams.first().cloned().unwrap_or_default();
    let callable = arg_streams
        .get_mut(1)
        .and_then(|stream| stream.items.first().cloned());
    let Some(Item::Lambda(lambda)) = callable else {
        return ctx.unsupported(
            arg_ids.first().copied().unwrap_or(IrId(0)),
            "sequence function requires a lambda argument",
        );
    };
    let mut out = ItemStream::empty();
    for item in source.items {
        let stream = ctx.invoke_lambda(lambda, vec![ItemStream::once(item.clone())]);
        if filter {
            if effective_boolean(&stream.items) {
                out.items.push(item);
            }
            out.extend_diagnostics(stream);
        } else if flatten {
            out.append_stream(stream);
        } else if let Some(item) = stream.items.first().cloned() {
            out.items.push(item);
            out.extend_diagnostics(stream);
        } else {
            out.extend_diagnostics(stream);
        }
        if out.error.is_some() {
            break;
        }
    }
    out
}

fn first_string(streams: &[ItemStream]) -> String {
    streams
        .first()
        .and_then(|stream| stream.items.first())
        .and_then(item_string)
        .unwrap_or_default()
}

fn first_number(streams: &[ItemStream]) -> f64 {
    streams
        .first()
        .and_then(|stream| stream.items.first())
        .and_then(item_number)
        .unwrap_or(0.0)
}

fn item_number(item: &Item) -> Option<f64> {
    match item {
        Item::Atomic(AtomValue::Integer(value)) => Some(*value as f64),
        Item::Atomic(AtomValue::Decimal(value)) => value.parse().ok(),
        Item::Atomic(AtomValue::Double(value)) => Some(*value),
        Item::Atomic(AtomValue::String(value)) => value.parse().ok(),
        _ => None,
    }
}

fn string_slice(streams: Vec<ItemStream>) -> ItemStream {
    let value = first_string(&streams);
    let start = streams.get(1).and_then(first_integer).unwrap_or(0).max(0) as usize;
    let len = streams
        .get(2)
        .and_then(first_integer)
        .map(|value| value.max(0) as usize);
    let chars = value.chars().skip(start);
    let out = match len {
        Some(len) => chars.take(len).collect(),
        None => chars.collect(),
    };
    ItemStream::once(Item::Atomic(AtomValue::String(out)))
}

fn string_concat(streams: Vec<ItemStream>) -> ItemStream {
    let separator = streams
        .get(1)
        .and_then(|stream| stream.items.first())
        .and_then(item_string)
        .unwrap_or_default();
    let parts = streams
        .first()
        .map(|stream| {
            stream
                .items
                .iter()
                .filter_map(item_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    ItemStream::once(Item::Atomic(AtomValue::String(parts.join(&separator))))
}

fn string_predicate(streams: Vec<ItemStream>, f: fn(&str, &str) -> bool) -> ItemStream {
    let left = first_string(&streams);
    let right = streams
        .get(1)
        .and_then(|stream| stream.items.first())
        .and_then(item_string)
        .unwrap_or_default();
    ItemStream::once(Item::Atomic(AtomValue::Boolean(f(&left, &right))))
}

fn number_double(streams: Vec<ItemStream>) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::Double(first_number(&streams))))
}

fn number_decimal(streams: Vec<ItemStream>) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::Decimal(
        first_number(&streams).to_string(),
    )))
}

fn number_integer(streams: Vec<ItemStream>) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::Integer(
        first_number(&streams) as i64
    )))
}

fn number_unary(streams: Vec<ItemStream>, f: fn(f64) -> f64) -> ItemStream {
    let value = f(first_number(&streams));
    if value.fract() == 0.0 {
        ItemStream::once(Item::Atomic(AtomValue::Integer(value as i64)))
    } else {
        ItemStream::once(Item::Atomic(AtomValue::Double(value)))
    }
}

fn number_format(streams: Vec<ItemStream>) -> ItemStream {
    let value = first_number(&streams).to_string();
    let pattern = streams
        .get(1)
        .and_then(|stream| stream.items.first())
        .and_then(item_string)
        .unwrap_or_else(|| "{}".to_owned());
    let formatted = if pattern.contains("{}") {
        pattern.replace("{}", &value)
    } else {
        value
    };
    ItemStream::once(Item::Atomic(AtomValue::String(formatted)))
}

fn normalize_utc(value: &str) -> String {
    if value.ends_with('Z') {
        value.to_owned()
    } else {
        format!("{value}Z")
    }
}

fn datetime_components(value: String) -> ItemStream {
    let mut record = BTreeMap::new();
    let (date, time) = value.split_once('T').unwrap_or((value.as_str(), ""));
    let mut date_parts = date.split('-');
    let mut time_parts = time.trim_end_matches('Z').split(':');
    record.insert(
        "year".to_owned(),
        vec![Item::Atomic(AtomValue::Integer(parse_i64(
            date_parts.next(),
        )))],
    );
    record.insert(
        "month".to_owned(),
        vec![Item::Atomic(AtomValue::Integer(parse_i64(
            date_parts.next(),
        )))],
    );
    record.insert(
        "day".to_owned(),
        vec![Item::Atomic(AtomValue::Integer(parse_i64(
            date_parts.next(),
        )))],
    );
    record.insert(
        "hour".to_owned(),
        vec![Item::Atomic(AtomValue::Integer(parse_i64(
            time_parts.next(),
        )))],
    );
    record.insert(
        "minute".to_owned(),
        vec![Item::Atomic(AtomValue::Integer(parse_i64(
            time_parts.next(),
        )))],
    );
    record.insert(
        "second".to_owned(),
        vec![Item::Atomic(AtomValue::Integer(parse_i64(
            time_parts.next(),
        )))],
    );
    record.insert(
        "tz".to_owned(),
        vec![Item::Atomic(AtomValue::String("Z".to_owned()))],
    );
    ItemStream::once(Item::Record(record))
}

fn parse_i64(value: Option<&str>) -> i64 {
    value.and_then(|value| value.parse().ok()).unwrap_or(0)
}

fn report_emit(streams: Vec<ItemStream>, ctx: &mut EvalCtx<'_>, source: IrId) -> ItemStream {
    let code = first_string(&streams);
    let message = streams
        .get(1)
        .and_then(|stream| stream.items.first())
        .and_then(item_string)
        .unwrap_or_default();
    let severity = streams
        .get(2)
        .and_then(|stream| stream.items.first())
        .and_then(item_string)
        .as_deref()
        .map(severity_from_str)
        .unwrap_or(Severity::Warning);
    ctx.emit_diagnostic(source, code, message, severity)
}

fn severity_from_str(value: &str) -> Severity {
    match value {
        "info" => Severity::Info,
        "error" => Severity::Error,
        "fatal" => Severity::Fatal,
        _ => Severity::Warning,
    }
}

fn cemml_parse(streams: Vec<ItemStream>) -> ItemStream {
    let source = first_string(&streams);
    let formatted = cem_ml::formatter::format_source(&source);
    ItemStream::once(Item::Node(formatted))
}
