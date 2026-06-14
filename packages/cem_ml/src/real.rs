//! Real (parser-enabled) `CemMlEngine` implementation.
//!
//! Bridges the library pipeline (tokenize → normalize → schema-validate
//! → AST build → validation rules → render) into the `CemMlEngine` trait
//! that `cem-ml-cli` calls through. This is the production engine that
//! replaces `NotImplementedEngine` in `cem-ml-cli/src/main.rs`.

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::*;
use crate::events::cem::CemEventNormalizer;
use crate::formatter;
use crate::interpreter::light_dom::LightDomInterpreter;
use crate::lifecycle::{ExportSelection, LifecycleRegistry, LoadedInput};
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::projection;
use crate::report::{Report, ReportOptionsSnapshot};
use crate::run_config::ScopeConfig;
use crate::schema::machine::CemSchemaMachine;
use crate::schema::vocab::CompiledSchema;
use crate::source::{BytesSource, SourceId};
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::html::HtmlTokenizer;
use crate::tokenizer::xml::XmlTokenizer;
use crate::tokenizer::SchemaTokenizer;
use crate::validation::{RuleContext, RuleRegistry};
use serde_json::{json, Value};
use std::time::Instant;

#[derive(Debug, Default, Clone)]
pub struct RealCemMlEngine;

impl RealCemMlEngine {
    pub fn new() -> Self {
        Self
    }
}

/// Aggregate every layer's diagnostics for an input through the
/// pipeline. Used by every parser-backed request, and by the public
/// observability entry point [`observe_pipeline`].
pub struct PipelineRun {
    pub document: CemDocument,
    pub diagnostics: Vec<Diagnostic>,
}

fn run_pipeline_as(bytes: &[u8], from_format: InputFormat) -> PipelineRun {
    match from_format {
        InputFormat::Cem => run_pipeline_with::<CemTokenizer>(bytes, None),
        InputFormat::Html => run_pipeline_with::<HtmlTokenizer>(bytes, None),
        InputFormat::Xml => run_pipeline_with::<XmlTokenizer>(bytes, None),
    }
}

fn run_pipeline_as_scoped(
    bytes: &[u8],
    from_format: InputFormat,
    root_scope: &ScopeConfig,
) -> PipelineRun {
    match from_format {
        InputFormat::Cem => run_pipeline_with::<CemTokenizer>(bytes, Some(root_scope)),
        InputFormat::Html => run_pipeline_with::<HtmlTokenizer>(bytes, Some(root_scope)),
        InputFormat::Xml => run_pipeline_with::<XmlTokenizer>(bytes, Some(root_scope)),
    }
}

fn run_pipeline_with<T>(bytes: &[u8], root_scope: Option<&ScopeConfig>) -> PipelineRun
where
    T: SchemaTokenizer + FromBytes,
{
    // Schema-machine pass.
    let schema_outcome = {
        let src = BytesSource::new(SourceId(1), bytes.to_vec());
        let tok = T::from_bytes(src);
        let normalizer = CemEventNormalizer::new(tok);
        let mut machine = CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer);
        if let Some(root_scope) = root_scope {
            machine = machine.with_root_namespace_bindings(
                root_scope.default_namespace.as_deref(),
                &root_scope.namespaces,
            );
        }
        machine.run()
    };

    // AST + tokenizer-diag fold (separate parse so token-diags surface).
    let mut document = {
        let src = BytesSource::new(SourceId(1), bytes.to_vec());
        let mut tok = T::from_bytes(src);
        let tok_diags = tok.take_diagnostics();
        let normalizer = CemEventNormalizer::new(tok);
        let mut doc = CemAstBuilder::new(normalizer).build();
        doc.diagnostics.extend(tok_diags);
        doc
    };
    document.diagnostics.extend(schema_outcome.diagnostics);

    // Validation rule registry.
    let registry = RuleRegistry::with_tier_a_rules();
    let rule_diags = registry.run(&RuleContext {
        document: &document,
        upstream_diagnostics: &document.diagnostics,
    });

    let mut diagnostics = document.diagnostics.clone();
    diagnostics.extend(rule_diags);
    PipelineRun {
        document,
        diagnostics,
    }
}

trait FromBytes: Sized {
    fn from_bytes(src: BytesSource) -> Self;
    fn take_diagnostics(&mut self) -> Vec<Diagnostic>;
}

impl FromBytes for CemTokenizer {
    fn from_bytes(src: BytesSource) -> Self {
        CemTokenizer::from_source(src)
    }
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        CemTokenizer::take_diagnostics(self)
    }
}

impl FromBytes for HtmlTokenizer {
    fn from_bytes(src: BytesSource) -> Self {
        HtmlTokenizer::from_source(src)
    }
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        HtmlTokenizer::take_diagnostics(self)
    }
}

impl FromBytes for XmlTokenizer {
    fn from_bytes(src: BytesSource) -> Self {
        XmlTokenizer::from_source(src)
    }
    fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        XmlTokenizer::take_diagnostics(self)
    }
}

fn fail_level_to_report(level: FailLevel) -> FailLevel {
    level
}

fn snapshot(level: FailLevel, ctx: &EngineContext) -> ReportOptionsSnapshot {
    ReportOptionsSnapshot {
        fail_level: fail_level_to_report(level),
        schema: ctx.schema.clone(),
        content_type: ctx.content_type.clone(),
        base_uri: ctx.base_uri.clone(),
    }
}

fn effective_base_uri<'a>(context: &'a EngineContext, scope: &'a ScopeConfig) -> Option<&'a str> {
    scope
        .base_uri
        .as_deref()
        .or(context.base_uri.as_deref())
        .filter(|base| !base.trim().is_empty())
}

fn has_uri_scheme(value: &str) -> bool {
    let Some((scheme, _)) = value.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

fn resolve_uri(base_uri: Option<&str>, uri: &str) -> String {
    if uri.is_empty()
        || has_uri_scheme(uri)
        || std::path::Path::new(uri).is_absolute()
        || base_uri.is_none()
    {
        return uri.to_owned();
    }
    let base = base_uri.unwrap().trim();
    if base.is_empty() {
        return uri.to_owned();
    }
    let uri = uri.trim_start_matches("./");
    if base.ends_with('/') {
        format!("{base}{uri}")
    } else {
        format!("{base}/{uri}")
    }
}

fn input_uri(input: &EngineInput, context: &EngineContext) -> String {
    resolve_uri(effective_base_uri(context, &input.root_scope), &input.uri)
}

fn input_uris(inputs: &[EngineInput], context: &EngineContext) -> Vec<String> {
    inputs
        .iter()
        .map(|input| input_uri(input, context))
        .collect()
}

fn project_diagnostic_uris(
    diagnostics: &mut [Diagnostic],
    input: &EngineInput,
    context: &EngineContext,
) {
    let display_uri = input_uri(input, context);
    for diagnostic in diagnostics {
        diagnostic.uri = Some(match diagnostic.uri.as_deref() {
            Some(uri) => resolve_uri(effective_base_uri(context, &input.root_scope), uri),
            None => display_uri.clone(),
        });
    }
}

fn unsupported_scope_diagnostic(uri: &str, code: &str, field: &str, direction: &str) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.to_owned()),
        code: code.to_owned(),
        severity: Severity::Warning,
        message: format!(
            "{direction} root-scope field `{field}` is parsed and preserved, but runtime enforcement is not implemented yet"
        ),
        ..Diagnostic::default()
    }
}

fn root_scope_execution_diagnostics(
    uri: &str,
    scope: &ScopeConfig,
    direction: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = root_scope_metadata_diagnostics(uri, scope, direction);
    if scope.policy.is_some() {
        diagnostics.push(unsupported_scope_diagnostic(
            uri,
            "cem.scope.policy_unenforced",
            "policy",
            direction,
        ));
    }
    if !scope.budgets.is_empty() {
        diagnostics.push(unsupported_scope_diagnostic(
            uri,
            "cem.scope.budgets_unenforced",
            "budgets",
            direction,
        ));
    }
    diagnostics
}

fn root_scope_metadata_diagnostics(
    uri: &str,
    scope: &ScopeConfig,
    direction: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !scope.version_pins.is_empty() {
        diagnostics.push(unsupported_scope_diagnostic(
            uri,
            "cem.scope.version_pins_unenforced",
            "versionPins",
            direction,
        ));
    }
    if scope.module_map.is_some() {
        diagnostics.push(unsupported_scope_diagnostic(
            uri,
            "cem.scope.module_map_unenforced",
            "moduleMap",
            direction,
        ));
    }
    diagnostics
}

fn scope_policy_diagnostic(uri: &str, code: &str, message: String, direction: &str) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.to_owned()),
        code: code.to_owned(),
        severity: Severity::Warning,
        message: format!("{direction} root-scope {message}"),
        ..Diagnostic::default()
    }
}

fn normalize_scope_key(key: &str) -> String {
    key.chars()
        .filter(|ch| *ch != '-' && *ch != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn parse_u32_budget(field: &str, value: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map(|value| value.max(1))
        .map_err(|_| format!("budget `{field}` expects an unsigned integer, got `{value}`"))
}

fn parse_u64_budget(field: &str, value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("budget `{field}` expects an unsigned integer, got `{value}`"))
}

fn apply_scope_scheduler_fields(
    policy: &mut crate::scheduler::ScopePolicy,
    uri: &str,
    scope: &ScopeConfig,
    direction: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if let Some(named_policy) = scope.policy.as_deref() {
        match normalize_scope_key(named_policy).as_str() {
            "host" => *policy = crate::scheduler::ScopePolicy::host_root(),
            "deterministic" | "default" => {
                *policy = crate::scheduler::ScopePolicy {
                    cpu_workers: 1,
                    queue_size: 8,
                    io_streams: 4,
                    memory_bytes: 8 * 1024 * 1024,
                    plugin_time_budget_ms: None,
                    overflow: crate::scheduler::OverflowPolicy::Reject,
                };
            }
            _ => diagnostics.push(unsupported_scope_diagnostic(
                uri,
                "cem.scope.policy_unenforced",
                "policy",
                direction,
            )),
        }
    }

    for (field, value) in &scope.budgets {
        match normalize_scope_key(field).as_str() {
            "cpu" | "cpuworkers" => match parse_u32_budget(field, value) {
                Ok(value) => policy.cpu_workers = value,
                Err(message) => diagnostics.push(scope_policy_diagnostic(
                    uri,
                    "cem.scope.budget_invalid",
                    message,
                    direction,
                )),
            },
            "queue" | "queuesize" => match parse_u32_budget(field, value) {
                Ok(value) => policy.queue_size = value,
                Err(message) => diagnostics.push(scope_policy_diagnostic(
                    uri,
                    "cem.scope.budget_invalid",
                    message,
                    direction,
                )),
            },
            "io" | "iostreams" => match parse_u32_budget(field, value) {
                Ok(value) => policy.io_streams = value,
                Err(message) => diagnostics.push(scope_policy_diagnostic(
                    uri,
                    "cem.scope.budget_invalid",
                    message,
                    direction,
                )),
            },
            "memory" | "memorybytes" => match parse_u64_budget(field, value) {
                Ok(value) => policy.memory_bytes = value,
                Err(message) => diagnostics.push(scope_policy_diagnostic(
                    uri,
                    "cem.scope.budget_invalid",
                    message,
                    direction,
                )),
            },
            "pluginms" | "plugintimebudgetms" => match parse_u64_budget(field, value) {
                Ok(value) => policy.plugin_time_budget_ms = Some(value),
                Err(message) => diagnostics.push(scope_policy_diagnostic(
                    uri,
                    "cem.scope.budget_invalid",
                    message,
                    direction,
                )),
            },
            "overflow" => match normalize_scope_key(value).as_str() {
                "block" => policy.overflow = crate::scheduler::OverflowPolicy::Block,
                "reject" => policy.overflow = crate::scheduler::OverflowPolicy::Reject,
                "spilltoparent" => {
                    policy.overflow = crate::scheduler::OverflowPolicy::SpillToParent
                }
                _ => diagnostics.push(scope_policy_diagnostic(
                    uri,
                    "cem.scope.budget_invalid",
                    format!("budget `overflow` expects block, reject, or spill-to-parent, got `{value}`"),
                    direction,
                )),
            },
            _ => diagnostics.push(scope_policy_diagnostic(
                uri,
                "cem.scope.budget_unenforced",
                format!("budget `{field}` is parsed and preserved, but runtime enforcement is not implemented yet"),
                direction,
            )),
        }
    }

    diagnostics
}

fn load_input_through_lifecycle(input: &EngineInput, context: &EngineContext) -> LoadedInput {
    LifecycleRegistry::with_builtin_adapters().load(input, context)
}

fn scheduler_policy_from_context(context: &EngineContext) -> crate::scheduler::ScopePolicy {
    let mut policy = if context.scheduler.thread_pool.as_deref() == Some("host") {
        crate::scheduler::ScopePolicy::host_root()
    } else {
        crate::scheduler::ScopePolicy {
            cpu_workers: 1,
            queue_size: 8,
            io_streams: 4,
            memory_bytes: 8 * 1024 * 1024,
            plugin_time_budget_ms: None,
            overflow: crate::scheduler::OverflowPolicy::Reject,
        }
    };

    if let Some(max_parallel_documents) = context.scheduler.max_parallel_documents {
        policy.cpu_workers = max_parallel_documents.max(1);
    }

    policy
}

fn scheduler_policy_for_scope(
    context: &EngineContext,
    uri: &str,
    scope: &ScopeConfig,
    direction: &str,
) -> (crate::scheduler::ScopePolicy, Vec<Diagnostic>) {
    let mut policy = scheduler_policy_from_context(context);
    let diagnostics = apply_scope_scheduler_fields(&mut policy, uri, scope, direction);
    (policy, diagnostics)
}

fn scheduler_policy_for_convert(
    request: &ConvertRequest,
) -> (crate::scheduler::ScopePolicy, Vec<Diagnostic>) {
    let mut policy = scheduler_policy_from_context(&request.context);
    let mut diagnostics = apply_scope_scheduler_fields(
        &mut policy,
        &request.input.uri,
        &request.input.root_scope,
        "input",
    );
    diagnostics.extend(apply_scope_scheduler_fields(
        &mut policy,
        &request.input.uri,
        &request.target_scope,
        "output",
    ));
    (policy, diagnostics)
}

fn scheduler_policy_json(policy: crate::scheduler::ScopePolicy) -> Value {
    json!({
        "cpuWorkers": policy.cpu_workers,
        "queueSize": policy.queue_size,
        "ioStreams": policy.io_streams,
        "memoryBytes": policy.memory_bytes,
        "pluginTimeBudgetMs": policy.plugin_time_budget_ms,
        "overflow": policy.overflow,
    })
}

fn run_scheduled_validation_documents(
    context: &EngineContext,
    inputs: &[EngineInput],
) -> EngineResult<(Vec<Diagnostic>, crate::scheduler::SchedulerTrace)> {
    let trace = crate::scheduler::SchedulerTrace::new();
    let abort = crate::scheduler::AbortSignal::new();
    let mut all_diags: Vec<Diagnostic> = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let mut input_diags: Vec<Diagnostic> = Vec::new();
        let (policy, mut policy_diagnostics) =
            scheduler_policy_for_scope(context, &input.uri, &input.root_scope, "input");
        input_diags.append(&mut policy_diagnostics);
        let pool = crate::scheduler::WorkerPool::new(index as u32, policy, trace.clone());
        for task in ["lifecycle-load", "parse-validate"] {
            pool.submit(format!("{}:{task}", input.uri), &abort)
                .map_err(|err| {
                    EngineError::Internal(format!("scheduler dispatch failed: {err}"))
                })?;
        }
        let mut loaded_input: Option<LoadedInput> = None;
        pool.run_to_completion(&abort, |task| {
            if task.ends_with(":lifecycle-load") {
                let mut scope_diagnostics =
                    root_scope_metadata_diagnostics(&input.uri, &input.root_scope, "input");
                input_diags.append(&mut scope_diagnostics);
                let mut loaded = load_input_through_lifecycle(input, context);
                input_diags.append(&mut loaded.diagnostics);
                loaded_input = Some(loaded);
                return;
            }

            let mut loaded = loaded_input
                .take()
                .unwrap_or_else(|| load_input_through_lifecycle(input, context));
            input_diags.append(&mut loaded.diagnostics);
            let run = run_pipeline_as_scoped(&loaded.bytes, loaded.from_format, &input.root_scope);
            input_diags.extend(run.diagnostics);
        });
        project_diagnostic_uris(&mut input_diags, input, context);
        all_diags.extend(input_diags);
    }
    Ok((all_diags, trace))
}

fn materialized_input(input: &EngineInput) -> EngineResult<EngineInput> {
    if !input.bytes.is_empty() {
        return Ok(input.clone());
    }
    let bytes = std::fs::read(&input.uri).map_err(|source| EngineError::Io {
        path: input.uri.clone().into(),
        source,
    })?;
    Ok(EngineInput {
        uri: input.uri.clone(),
        bytes,
        from_format: input.from_format,
        identity: input.identity.clone(),
        root_scope: input.root_scope.clone(),
    })
}

/// Run the full Tier A pipeline (`tokenize → normalize → schema → AST →
/// validation rules`) while routing every observable event through the
/// supplied [`EngineObserver`].
///
/// AC-O-1 / AC-O-3: emits one `parse` event per [`crate::events::NormalizedEvent`],
/// one `transform` event per layer boundary the pipeline crosses,
/// and one `validate` event per emitted [`Diagnostic`]. Every event
/// carries a monotonic sequence number, the originating byte offset
/// (when known), and the source-map stack as it exists at emission.
pub fn observe_pipeline(
    bytes: &[u8],
    from_format: InputFormat,
    observer: &dyn crate::observability::EngineObserver,
) -> PipelineRun {
    use crate::events::{EventNormalizer, NormalizedEvent, ScalarValue};
    use crate::observability::{EventEmitter, EventSequencer, ParseEventKind};
    use crate::source_map::TransformKind;

    let mut sequencer = EventSequencer::new();
    let mut emit = EventEmitter::new(observer, &mut sequencer);

    // Layer boundary: tokenizer started. Profile decides which
    // TransformKind frames are pushed onto downstream source maps.
    let tokenizer_kind = match from_format {
        InputFormat::Cem => TransformKind::CemTokenizer,
        InputFormat::Html => TransformKind::HtmlTokenizer,
        InputFormat::Xml => TransformKind::XmlTokenizer,
    };
    emit.transform(
        tokenizer_kind.clone(),
        format!("tokenizer entered ({from_format:?})"),
        None,
        None,
    );

    // Event-normalizer pass — produces the `parse` channel feed.
    let normalizer_diags: Vec<Diagnostic>;
    {
        match from_format {
            InputFormat::Cem => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let mut tok = CemTokenizer::from_source(src);
                let tok_diags = tok.take_diagnostics();
                let mut normalizer = CemEventNormalizer::new(tok);
                while let Some(event) = normalizer.next_event() {
                    emit_parse_event(&mut emit, &event);
                }
                normalizer_diags = tok_diags;
            }
            InputFormat::Html => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let mut tok = HtmlTokenizer::from_source(src);
                let tok_diags = tok.take_diagnostics();
                let mut normalizer = CemEventNormalizer::new(tok);
                while let Some(event) = normalizer.next_event() {
                    emit_parse_event(&mut emit, &event);
                }
                normalizer_diags = tok_diags;
            }
            InputFormat::Xml => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let mut tok = XmlTokenizer::from_source(src);
                let tok_diags = tok.take_diagnostics();
                let mut normalizer = CemEventNormalizer::new(tok);
                while let Some(event) = normalizer.next_event() {
                    emit_parse_event(&mut emit, &event);
                }
                normalizer_diags = tok_diags;
            }
        }
    }

    emit.transform(
        TransformKind::EventNormalizer,
        "event normalizer drained",
        None,
        None,
    );

    let run = run_pipeline_as(bytes, from_format);

    emit.transform(TransformKind::CemAstBuilder, "AST built", None, None);

    // Validate channel — every accumulated diagnostic, plus the
    // normalizer's own diagnostics we collected above (they are also
    // folded into `run.diagnostics` by run_pipeline_as).
    let mut emitted_codes_offsets = std::collections::HashSet::<(String, Option<u64>)>::new();
    for diag in run.diagnostics.iter().chain(normalizer_diags.iter()) {
        let key = (diag.code.clone(), diag.byte_offset);
        if emitted_codes_offsets.insert(key) {
            emit.validate(diag);
        }
    }

    fn emit_parse_event(emit: &mut EventEmitter<'_>, event: &NormalizedEvent) {
        match event {
            NormalizedEvent::OpenScope {
                name,
                byte_range,
                source_map,
            } => emit.parse(
                ParseEventKind::OpenScope,
                Some(name.lexical_name.clone()),
                None,
                Some(byte_range.start),
                Some(source_map.clone()),
            ),
            NormalizedEvent::CloseScope {
                name,
                byte_range,
                source_map,
                ..
            } => emit.parse(
                ParseEventKind::CloseScope,
                Some(name.lexical_name.clone()),
                None,
                Some(byte_range.start),
                Some(source_map.clone()),
            ),
            NormalizedEvent::Name { name, byte_range } => emit.parse(
                ParseEventKind::Name,
                Some(name.lexical_name.clone()),
                None,
                Some(byte_range.start),
                None,
            ),
            NormalizedEvent::Value { value, byte_range } => {
                let v = match value {
                    ScalarValue::Text(t) => t.clone(),
                    ScalarValue::Int(i) => i.to_string(),
                    ScalarValue::Float(f) => f.to_string(),
                    ScalarValue::Bool(b) => b.to_string(),
                    ScalarValue::Null => String::new(),
                };
                emit.parse(
                    ParseEventKind::Value,
                    None,
                    Some(v),
                    Some(byte_range.start),
                    None,
                );
            }
            NormalizedEvent::Trivia {
                kind,
                data,
                byte_range,
            } => emit.parse(
                ParseEventKind::Trivia,
                Some(format!("{kind:?}")),
                Some(data.clone()),
                Some(byte_range.start),
                None,
            ),
            NormalizedEvent::Separator { kind, byte_range } => emit.parse(
                ParseEventKind::Separator,
                Some(format!("{kind:?}")),
                None,
                Some(byte_range.start),
                None,
            ),
            NormalizedEvent::ModeSwitch {
                content_type,
                handoff,
            } => emit.parse(
                ParseEventKind::ModeSwitch,
                Some(content_type.clone()),
                None,
                Some(handoff.source_span.start),
                None,
            ),
            NormalizedEvent::ProcessingInstruction {
                target,
                data,
                byte_range,
            } => emit.parse(
                ParseEventKind::ProcessingInstruction,
                Some(target.clone()),
                Some(data.clone()),
                Some(byte_range.start),
                None,
            ),
            NormalizedEvent::Error {
                code, byte_range, ..
            } => emit.parse(
                ParseEventKind::Error,
                Some(code.clone()),
                None,
                Some(byte_range.start),
                None,
            ),
        }
    }

    run
}

impl CemMlEngine for RealCemMlEngine {
    fn parse(&self, request: ParseRequest) -> EngineResult<ParseResponse> {
        let loaded = load_input_through_lifecycle(&request.input, &request.context);
        let from_format = loaded.from_format;
        let run = run_pipeline_as_scoped(&loaded.bytes, from_format, &request.input.root_scope);
        let primary = match request.projection {
            ParseProjection::DomJson | ParseProjection::Json => projection::dom_json(&run.document),
            ParseProjection::Ast => projection::ast_json(&run.document),
            ParseProjection::Events => projection::events_json_as(&loaded.bytes, from_format),
        };
        let mut diagnostics = root_scope_execution_diagnostics(
            &request.input.uri,
            &request.input.root_scope,
            "input",
        );
        diagnostics.extend(loaded.diagnostics);
        diagnostics.extend(run.diagnostics);
        project_diagnostic_uris(&mut diagnostics, &request.input, &request.context);
        Ok(ParseResponse {
            primary,
            diagnostics,
        })
    }

    fn validate(&self, request: ValidateRequest) -> EngineResult<ValidateResponse> {
        let inputs = input_uris(&request.inputs, &request.context);
        let (all_diags, scheduler_trace) =
            run_scheduled_validation_documents(&request.context, &request.inputs)?;
        let report = Report::deterministic(
            inputs,
            all_diags,
            snapshot(request.fail_level, &request.context),
        )
        .with_scheduler_trace(&scheduler_trace);
        Ok(ValidateResponse { report })
    }

    fn check(&self, request: CheckRequest) -> EngineResult<CheckResponse> {
        let inputs = input_uris(&request.inputs, &request.context);
        let (all_diags, scheduler_trace) =
            run_scheduled_validation_documents(&request.context, &request.inputs)?;
        let report = Report::deterministic(
            inputs,
            all_diags,
            snapshot(request.fail_level, &request.context),
        )
        .with_scheduler_trace(&scheduler_trace);
        let hard_violation_count = report.summary.hard_violation_count;
        Ok(CheckResponse {
            report,
            hard_violation_count,
        })
    }

    fn inspect(&self, request: InspectRequest) -> EngineResult<InspectResponse> {
        let loaded = load_input_through_lifecycle(&request.input, &request.context);
        let from_format = loaded.from_format;
        let run = run_pipeline_as_scoped(&loaded.bytes, from_format, &request.input.root_scope);
        let mut diagnostics = root_scope_execution_diagnostics(
            &request.input.uri,
            &request.input.root_scope,
            "input",
        );
        diagnostics.extend(loaded.diagnostics);
        diagnostics.extend(run.diagnostics);
        project_diagnostic_uris(&mut diagnostics, &request.input, &request.context);
        let display_uri = input_uri(&request.input, &request.context);
        let body = match request.show {
            InspectView::Summary => {
                let elements = run
                    .document
                    .iter()
                    .filter(|n| matches!(n, crate::parser::CemAstNode::Element { .. }))
                    .count();
                let attributes = run
                    .document
                    .iter()
                    .filter(|n| matches!(n, crate::parser::CemAstNode::Attribute { .. }))
                    .count();
                json!({
                    "kind": "summary",
                    "input": display_uri,
                    "elements": elements,
                    "attributes": attributes,
                    "diagnosticCount": diagnostics.len(),
                })
            }
            InspectView::Ast => projection::ast_json(&run.document),
            InspectView::Events => projection::events_json_as(&loaded.bytes, from_format),
            InspectView::Diagnostics => json!({
                "kind": "diagnostics",
                "input": display_uri,
                "diagnostics": diagnostics,
            }),
            InspectView::SourceOffsets => {
                let mut offsets: Vec<Value> = Vec::new();
                for node in run.document.iter() {
                    if let Some(range) = crate::query::origin_byte_range(node) {
                        offsets.push(json!({
                            "byteStart": range.start,
                            "byteLen": range.len,
                        }));
                    }
                }
                json!({
                    "kind": "source-offsets",
                    "input": display_uri,
                    "offsets": offsets,
                })
            }
            InspectView::Tree => projection::dom_json(&run.document),
        };
        Ok(InspectResponse {
            view: request.show,
            body,
        })
    }

    fn convert(&self, request: ConvertRequest) -> EngineResult<ConvertResponse> {
        let trace = crate::scheduler::SchedulerTrace::new();
        let (policy, mut diagnostics) = scheduler_policy_for_convert(&request);
        let pool =
            crate::scheduler::WorkerPool::new(request.scheduler_scope_id, policy, trace.clone());
        let abort = crate::scheduler::AbortSignal::new();
        for task in ["lifecycle-load", "select-export", "convert"] {
            pool.submit(format!("{}:{task}", request.input.uri), &abort)
                .map_err(|err| {
                    EngineError::Internal(format!("scheduler dispatch failed: {err}"))
                })?;
        }

        let registry = LifecycleRegistry::with_builtin_adapters();
        let mut loaded_input: Option<LoadedInput> = None;
        let mut export_selection: Option<ExportSelection> = None;
        let mut primary: Option<Value> = None;
        pool.run_to_completion(&abort, |task| {
            if task.ends_with(":lifecycle-load") {
                let mut scope_diagnostics = root_scope_metadata_diagnostics(
                    &request.input.uri,
                    &request.input.root_scope,
                    "input",
                );
                diagnostics.append(&mut scope_diagnostics);
                let mut loaded = registry.load(&request.input, &request.context);
                diagnostics.append(&mut loaded.diagnostics);
                loaded_input = Some(loaded);
                return;
            }

            if task.ends_with(":select-export") {
                let mut scope_diagnostics = root_scope_metadata_diagnostics(
                    &request.input.uri,
                    &request.target_scope,
                    "output",
                );
                diagnostics.append(&mut scope_diagnostics);
                let mut export = registry.select_export(request.target.as_ref(), request.to_format);
                diagnostics.append(&mut export.diagnostics);
                export_selection = Some(export);
                return;
            }

            let mut loaded = loaded_input
                .take()
                .unwrap_or_else(|| registry.load(&request.input, &request.context));
            diagnostics.append(&mut loaded.diagnostics);
            let mut export = export_selection.take().unwrap_or_else(|| {
                registry.select_export(request.target.as_ref(), request.to_format)
            });
            diagnostics.append(&mut export.diagnostics);
            let to_format = export.to_format;

            if to_format == LayerFormat::Cem && loaded.from_format == InputFormat::Cem {
                let mut content = String::from_utf8_lossy(&loaded.bytes).into_owned();
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                primary = Some(json!({
                    "kind": "cem",
                    "content": content,
                    "sourceMap": null,
                    "outputSpans": [],
                }));
                return;
            }

            let from_format = loaded.from_format;
            let run = run_pipeline_as_scoped(&loaded.bytes, from_format, &request.input.root_scope);
            primary = Some(match to_format {
                LayerFormat::Cem => {
                    let formatted = formatter::format_transform(
                        &run.document,
                        match from_format {
                            InputFormat::Cem => "application/cem",
                            InputFormat::Html => "text/html",
                            InputFormat::Xml => "application/xml",
                        },
                    );
                    json!({
                        "kind": "cem",
                        "content": formatted.rendered,
                        "sourceMap": formatted.source_map,
                        "outputSpans": formatted.output_spans.iter().map(|span| json!({
                            "outputRange": span.output_range,
                            "origin": span.origin,
                        })).collect::<Vec<_>>(),
                    })
                }
                LayerFormat::DomJson => projection::dom_json(&run.document),
                LayerFormat::Ast => projection::ast_json(&run.document),
                LayerFormat::Events => projection::events_json_as(&loaded.bytes, from_format),
            });
            diagnostics.extend(run.diagnostics);
        });
        let Some(primary) = primary else {
            return Err(EngineError::Internal(
                "scheduler did not dispatch convert task".to_owned(),
            ));
        };
        project_diagnostic_uris(&mut diagnostics, &request.input, &request.context);
        Ok(ConvertResponse {
            primary,
            diagnostics,
            scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
        })
    }

    fn trace(&self, request: TraceRequest) -> EngineResult<TraceResponse> {
        let loaded = load_input_through_lifecycle(&request.input, &request.context);
        let from_format = loaded.from_format;
        let scheduler_trace = crate::scheduler::SchedulerTrace::new();
        let (policy, policy_diagnostics) = scheduler_policy_for_scope(
            &request.context,
            &request.input.uri,
            &request.input.root_scope,
            "input",
        );
        let pool = crate::scheduler::WorkerPool::new(0, policy, scheduler_trace.clone());
        let abort = crate::scheduler::AbortSignal::new();
        for task in ["tokenize", "normalize", "schema", "ast", "validate"] {
            pool.submit(task, &abort).map_err(|err| {
                EngineError::Internal(format!("scheduler trace setup failed: {err}"))
            })?;
        }
        let run = run_pipeline_as_scoped(&loaded.bytes, from_format, &request.input.root_scope);
        pool.run_to_completion(&abort, |_| {});
        let mut diagnostics = policy_diagnostics;
        diagnostics.extend(root_scope_metadata_diagnostics(
            &request.input.uri,
            &request.input.root_scope,
            "input",
        ));
        diagnostics.extend(loaded.diagnostics);
        diagnostics.extend(run.diagnostics);
        project_diagnostic_uris(&mut diagnostics, &request.input, &request.context);
        let report = Report::deterministic(
            vec![input_uri(&request.input, &request.context)],
            diagnostics,
            snapshot(FailLevel::Validate, &request.context),
        )
        .with_scheduler_trace(&scheduler_trace);
        let body = json!({
            "kind": "trace",
            "input": input_uri(&request.input, &request.context),
            "projection": request.projection,
            "scheduler": {
                "threadPool": request.context.scheduler.thread_pool,
                "maxParallelDocuments": request.context.scheduler.max_parallel_documents,
                "policy": scheduler_policy_json(policy),
            },
            "events": projection::events_json_as(&loaded.bytes, from_format),
            "report": report,
        });
        Ok(TraceResponse { body })
    }

    fn bench(&self, request: BenchRequest) -> EngineResult<BenchResponse> {
        let iterations = request.iterations.max(1);
        let mut total_ns: u128 = 0;
        let mut per_iter_ns: Vec<u128> = Vec::with_capacity(iterations as usize);
        let mut budget_exceeded = false;
        for _ in 0..iterations {
            let t = Instant::now();
            for input in &request.inputs {
                let input = materialized_input(input)?;
                let loaded = load_input_through_lifecycle(&input, &request.context);
                let _ =
                    run_pipeline_as_scoped(&loaded.bytes, loaded.from_format, &input.root_scope);
            }
            let elapsed = t.elapsed().as_nanos();
            per_iter_ns.push(elapsed);
            total_ns += elapsed;
            if let Some(budget_ms) = request.budget_ms {
                if elapsed > (budget_ms as u128) * 1_000_000 {
                    budget_exceeded = true;
                }
            }
        }
        let mean_ns = if !per_iter_ns.is_empty() {
            total_ns / per_iter_ns.len() as u128
        } else {
            0
        };
        let body = json!({
            "kind": "bench",
            "iterations": iterations,
            "totalNs": total_ns,
            "meanNs": mean_ns,
            "perIterationNs": per_iter_ns,
            "budgetMs": request.budget_ms,
            "budgetExceeded": budget_exceeded,
        });
        Ok(BenchResponse {
            body,
            budget_exceeded,
        })
    }

    fn fixture_validate(
        &self,
        request: FixtureValidateRequest,
    ) -> EngineResult<FixtureValidateResponse> {
        let inputs = input_uris(&request.inputs, &request.context);
        let mut all_diags: Vec<Diagnostic> = Vec::new();
        for input in &request.inputs {
            let input = materialized_input(input)?;
            let mut input_diags =
                root_scope_execution_diagnostics(&input.uri, &input.root_scope, "input");
            let loaded = load_input_through_lifecycle(&input, &request.context);
            input_diags.extend(loaded.diagnostics);
            let run = run_pipeline_as_scoped(&loaded.bytes, loaded.from_format, &input.root_scope);
            input_diags.extend(run.diagnostics);
            project_diagnostic_uris(&mut input_diags, &input, &request.context);
            all_diags.extend(input_diags);
        }
        let report = Report::deterministic(
            inputs,
            all_diags,
            snapshot(request.fail_level, &request.context),
        );
        Ok(FixtureValidateResponse { report })
    }

    fn fixture_roundtrip(
        &self,
        request: FixtureRoundtripRequest,
    ) -> EngineResult<FixtureRoundtripResponse> {
        let inputs = input_uris(&request.inputs, &request.context);
        let mut artifacts: Vec<Value> = Vec::new();
        let mut all_diags: Vec<Diagnostic> = Vec::new();
        for input in &request.inputs {
            let input = materialized_input(input)?;
            let mut input_diags =
                root_scope_execution_diagnostics(&input.uri, &input.root_scope, "input");
            let loaded = load_input_through_lifecycle(&input, &request.context);
            input_diags.extend(loaded.diagnostics);
            let run = run_pipeline_as_scoped(&loaded.bytes, loaded.from_format, &input.root_scope);
            let rendered = LightDomInterpreter::new().render(&run.document);
            artifacts.push(json!({
                "input": input_uri(&input, &request.context),
                "toFormat": request.to_format,
                "rendered": rendered.rendered,
            }));
            input_diags.extend(run.diagnostics);
            project_diagnostic_uris(&mut input_diags, &input, &request.context);
            all_diags.extend(input_diags);
        }
        let report = Report::deterministic(
            inputs,
            all_diags,
            snapshot(FailLevel::Validate, &request.context),
        );
        Ok(FixtureRoundtripResponse { report, artifacts })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(bytes: &[u8], uri: &str) -> EngineInput {
        EngineInput {
            uri: uri.to_owned(),
            bytes: bytes.to_vec(),
            from_format: None,
            identity: None,
            root_scope: Default::default(),
        }
    }

    fn ctx() -> EngineContext {
        EngineContext::default()
    }

    #[test]
    fn parse_dom_json_returns_document_root() {
        let req = ParseRequest {
            input: input(b"{p Hi}", "in"),
            projection: ParseProjection::DomJson,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: false,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().parse(req).unwrap();
        assert_eq!(resp.primary["kind"], "document");
    }

    #[test]
    fn parse_events_returns_event_array() {
        let req = ParseRequest {
            input: input(b"{p Hi}", "in"),
            projection: ParseProjection::Events,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: false,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().parse(req).unwrap();
        assert!(resp.primary.is_array());
    }

    #[test]
    fn parse_legacy_custom_element_content_type_uses_lifecycle_adapter() {
        let req = ParseRequest {
            input: input(
                br#"<if test="$ready"><button>Go</button></if>"#,
                "legacy.html",
            ),
            projection: ParseProjection::DomJson,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: false,
            context: EngineContext {
                content_type: Some(crate::legacy_custom_element::TEMPLATE_LANG.to_owned()),
                ..ctx()
            },
        };
        let resp = RealCemMlEngine::new().parse(req).unwrap();
        assert_eq!(resp.primary["kind"], "document");
        assert_eq!(resp.primary["children"][0]["name"], "if");
        assert_eq!(resp.primary["children"][0]["namespace"], "cem");
    }

    #[test]
    fn validate_canonical_login_fixture_clean() {
        let bytes = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/cem-ml/login.cem"),
        )
        .unwrap();
        let req = ValidateRequest {
            inputs: vec![input(&bytes, "login.cem")],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert_eq!(resp.report.summary.hard_violation_count, 0);
        assert_eq!(resp.report.summary.input_count, 1);
    }

    #[test]
    fn validate_applies_context_base_uri_to_report_inputs_and_diagnostics() {
        let req = ValidateRequest {
            inputs: vec![input(b"{unknown}", "src/in.cem")],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext {
                base_uri: Some("file:///workspace/".to_owned()),
                ..ctx()
            },
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert_eq!(resp.report.inputs[0], "file:///workspace/src/in.cem");
        assert!(resp
            .report
            .diagnostics
            .iter()
            .all(|diag| diag.uri.as_deref() == Some("file:///workspace/src/in.cem")));
    }

    #[test]
    fn input_root_scope_base_uri_overrides_context_base_uri() {
        let mut source = input(b"{unknown}", "src/in.cem");
        source.root_scope.base_uri = Some("file:///scope/".to_owned());
        let req = ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext {
                base_uri: Some("file:///workspace/".to_owned()),
                ..ctx()
            },
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert_eq!(resp.report.inputs[0], "file:///scope/src/in.cem");
        assert!(resp
            .report
            .diagnostics
            .iter()
            .all(|diag| diag.uri.as_deref() == Some("file:///scope/src/in.cem")));
    }

    #[test]
    fn validate_legacy_custom_element_content_type_runs_xslt_lifecycle_adapter() {
        let req = ValidateRequest {
            inputs: vec![input(br#"<button>Go</button>"#, "legacy.html")],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext {
                content_type: Some(crate::legacy_custom_element::TEMPLATE_LANG.to_owned()),
                ..ctx()
            },
        };
        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert_eq!(resp.report.summary.hard_violation_count, 0);
        assert_eq!(resp.report.summary.error_count, 0);
        assert_eq!(resp.report.summary.warning_count, 0);
    }

    #[test]
    fn validate_report_embeds_run_level_scheduler_trace_for_each_input_scope() {
        let req = ValidateRequest {
            inputs: vec![input(b"{p One}", "one.cem"), input(b"{p Two}", "two.cem")],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext {
                scheduler: crate::run_config::SchedulerConfig {
                    thread_pool: Some("deterministic".to_owned()),
                    max_parallel_documents: Some(3),
                },
                ..ctx()
            },
        };
        let resp = RealCemMlEngine::new().validate(req).unwrap();
        let events = &resp.report.report_ast.scheduler_trace.events;
        assert_eq!(resp.report.summary.input_count, 2);
        assert_eq!(resp.report.report_ast.scheduler_trace.event_count, 12);
        assert!(events.iter().any(|event| event.scope_id == 0));
        assert!(events.iter().any(|event| event.scope_id == 1));
        assert!(events
            .iter()
            .any(|event| event.task == "two.cem:parse-validate"));
    }

    #[test]
    fn validate_reports_unenforced_root_scope_fields() {
        let mut source = input(b"{p Hi}", "scoped.cem");
        source.root_scope.module_map = Some("cem.modules.json".to_owned());
        source
            .root_scope
            .budgets
            .insert("parseMs".to_owned(), "5".to_owned());
        let req = ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert!(resp.report.summary.warning_count >= 2);
        assert!(resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.module_map_unenforced"));
        assert!(resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn validate_applies_root_scope_namespaces_to_schema_validation() {
        let mut source = input(b"{widget:panel Hi}", "scoped.cem");
        source
            .root_scope
            .namespaces
            .insert("widget".to_owned(), "urn:widgets".to_owned());
        let req = ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert!(resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.schema.unresolved_namespace"));
        assert!(!resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.namespaces_unenforced"));
    }

    #[test]
    fn input_identity_overrides_global_context_content_type() {
        let mut source = input(br#"<button>Go</button>"#, "legacy.html");
        source.identity = Some(FormatIdentity {
            content_type: Some(crate::legacy_custom_element::TEMPLATE_LANG.to_owned()),
            ..FormatIdentity::default()
        });
        let req = ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext {
                content_type: Some("application/cem+xml".to_owned()),
                ..ctx()
            },
        };
        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert_eq!(resp.report.summary.hard_violation_count, 0);
        assert_eq!(resp.report.summary.error_count, 0);
    }

    #[test]
    fn validate_legacy_custom_element_content_type_reports_unsupported_xslt() {
        let req = ValidateRequest {
            inputs: vec![input(br#"<xsl:copy-of select="node()"/>"#, "legacy.html")],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext {
                content_type: Some(crate::legacy_custom_element::TEMPLATE_LANG.to_owned()),
                ..ctx()
            },
        };
        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert_eq!(resp.report.summary.warning_count, 1);
        assert!(resp
            .report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code
                == crate::legacy_custom_element::UNSUPPORTED_CONSTRUCT_CODE));
    }

    #[test]
    fn check_zero_hard_violations_succeeds_on_clean_fixture() {
        let bytes = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/cem-ml/login.cem"),
        )
        .unwrap();
        let req = CheckRequest {
            inputs: vec![input(&bytes, "login.cem")],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            zero_hard_violations: true,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().check(req).unwrap();
        assert_eq!(resp.hard_violation_count, 0);
    }

    #[test]
    fn inspect_summary_view_counts_elements_and_attributes() {
        let req = InspectRequest {
            input: input(b"{button @type=submit | Save}", "in"),
            show: InspectView::Summary,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().inspect(req).unwrap();
        assert_eq!(resp.body["kind"], "summary");
        assert!(resp.body["elements"].as_u64().unwrap() >= 1);
        assert!(resp.body["attributes"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn inspect_legacy_custom_element_content_type_uses_lifecycle_adapter() {
        let req = InspectRequest {
            input: input(br#"<button type="button">Go</button>"#, "legacy.html"),
            show: InspectView::Summary,
            context: EngineContext {
                content_type: Some(crate::legacy_custom_element::TEMPLATE_LANG.to_owned()),
                ..ctx()
            },
        };
        let resp = RealCemMlEngine::new().inspect(req).unwrap();
        assert_eq!(resp.body["kind"], "summary");
        assert_eq!(resp.body["elements"], 1);
        assert_eq!(resp.body["attributes"], 1);
        assert_eq!(resp.body["diagnosticCount"], 0);
    }

    #[test]
    fn convert_dom_json_returns_document_tree() {
        let req = ConvertRequest {
            input: input(b"{p Hi}", "in"),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "document");
        assert_eq!(resp.scheduler_trace.event_count, 9);
    }

    #[test]
    fn convert_html_to_canonical_cem_ml_returns_source_map() {
        let req = ConvertRequest {
            input: EngineInput {
                uri: "in.html".to_owned(),
                bytes: br#"<button cem:action="primary" type="submit">Save</button>"#.to_vec(),
                from_format: Some(InputFormat::Html),
                identity: None,
                root_scope: Default::default(),
            },
            to_format: LayerFormat::Cem,
            preserve_source_offsets: true,
            context: ctx(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "cem");
        assert_eq!(
            resp.primary["content"].as_str().unwrap(),
            "{button @type=submit @cem:action=primary | Save}\n"
        );
        assert!(resp.primary["outputSpans"]
            .as_array()
            .unwrap()
            .iter()
            .any(|span| {
                span["origin"]["frames"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|frame| frame["transform"]["kind"] == "HtmlTokenizer")
                    && span["origin"]["frames"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|frame| {
                            frame["transform"]["kind"] == "ContentTypeTransform"
                                && frame["transform"]["content_type"] == "text/html"
                        })
            }));
    }

    #[test]
    fn convert_xml_to_canonical_cem_ml_returns_source_map() {
        let req = ConvertRequest {
            input: EngineInput {
                uri: "in.xml".to_owned(),
                bytes: br#"<button cem:action="primary" type="submit">Save</button>"#.to_vec(),
                from_format: Some(InputFormat::Xml),
                identity: None,
                root_scope: Default::default(),
            },
            to_format: LayerFormat::Cem,
            preserve_source_offsets: true,
            context: ctx(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "cem");
        assert_eq!(
            resp.primary["content"].as_str().unwrap(),
            "{button @type=submit @cem:action=primary | Save}\n"
        );
        assert!(resp.primary["outputSpans"]
            .as_array()
            .unwrap()
            .iter()
            .any(|span| {
                span["origin"]["frames"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|frame| frame["transform"]["kind"] == "XmlTokenizer")
                    && span["origin"]["frames"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|frame| {
                            frame["transform"]["kind"] == "ContentTypeTransform"
                                && frame["transform"]["content_type"] == "application/xml"
                        })
            }));
    }

    #[test]
    fn convert_legacy_custom_element_content_type_to_canonical_cem_ml() {
        let req = ConvertRequest {
            input: EngineInput {
                uri: "legacy.html".to_owned(),
                bytes: br#"<if test="not($disabled)"><button>Go</button></if>"#.to_vec(),
                from_format: None,
                identity: None,
                root_scope: Default::default(),
            },
            to_format: LayerFormat::Cem,
            preserve_source_offsets: false,
            context: EngineContext {
                content_type: Some(crate::legacy_custom_element::TEMPLATE_LANG.to_owned()),
                ..ctx()
            },
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "cem");
        assert_eq!(
            resp.primary["content"].as_str().unwrap(),
            "{cem:if @test=\"not (disabled)\" | {button | Go}}\n"
        );
        assert!(resp.diagnostics.is_empty());
    }

    #[test]
    fn convert_target_cem_content_type_selects_canonical_cem_export() {
        let req = ConvertRequest {
            input: input(b"{p Hi}", "in.cem"),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
            target: Some(FormatIdentity {
                content_type: Some("application/cem+xml".to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "cem");
        assert_eq!(resp.primary["content"], "{p Hi}\n");
        assert!(resp.diagnostics.is_empty());
    }

    #[test]
    fn convert_reports_unenforced_output_scope_fields() {
        let req = ConvertRequest {
            input: input(b"{p Hi}", "in.cem"),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
            target: None,
            target_scope: crate::run_config::ScopeConfig {
                module_map: Some("cem.modules.json".to_owned()),
                policy: Some("strict".to_owned()),
                ..Default::default()
            },
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert!(resp
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.module_map_unenforced"));
        assert!(resp
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.policy_unenforced"));
    }

    #[test]
    fn trace_response_embeds_scheduler_projection_in_report_ast() {
        let req = TraceRequest {
            input: input(b"{p Hi}", "in.cem"),
            projection: TraceProjection::Json,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().trace(req).unwrap();
        let scheduler_trace = &resp.body["report"]["reportAst"]["schedulerTrace"];
        assert_eq!(scheduler_trace["eventCount"], 15);
        assert_eq!(
            scheduler_trace["events"][0]["kind"],
            serde_json::Value::String("enqueue".to_owned())
        );
        assert_eq!(scheduler_trace["events"][0]["scopeId"], 0);
        assert_eq!(scheduler_trace["events"][0]["task"], "tokenize");
    }

    #[test]
    fn trace_applies_input_scope_scheduler_policy_and_budgets() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source.root_scope.policy = Some("deterministic".to_owned());
        source
            .root_scope
            .budgets
            .insert("queueSize".to_owned(), "12".to_owned());
        source
            .root_scope
            .budgets
            .insert("pluginTimeBudgetMs".to_owned(), "7".to_owned());
        let req = TraceRequest {
            input: source,
            projection: TraceProjection::Json,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().trace(req).unwrap();
        assert_eq!(resp.body["scheduler"]["policy"]["queueSize"], 12);
        assert_eq!(resp.body["scheduler"]["policy"]["pluginTimeBudgetMs"], 7);
        assert!(!resp.body["report"]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budgets_unenforced"));
    }

    #[test]
    fn trace_legacy_custom_element_content_type_uses_lifecycle_adapter() {
        let req = TraceRequest {
            input: input(br#"<button>Go</button>"#, "legacy.html"),
            projection: TraceProjection::Json,
            context: EngineContext {
                content_type: Some(crate::legacy_custom_element::TEMPLATE_LANG.to_owned()),
                ..ctx()
            },
        };
        let resp = RealCemMlEngine::new().trace(req).unwrap();
        assert_eq!(resp.body["kind"], "trace");
        assert!(resp.body["events"].to_string().contains("button"));
        assert_eq!(resp.body["report"]["summary"]["hardViolationCount"], 0);
    }

    #[test]
    fn bench_records_iteration_timings() {
        let req = BenchRequest {
            inputs: vec![input(b"{p Hi}", "in")],
            projection: BenchProjection::Json,
            iterations: 3,
            budget_ms: None,
            profile: None,
            cold_cache: false,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().bench(req).unwrap();
        assert_eq!(resp.body["iterations"], 3);
        assert_eq!(resp.body["perIterationNs"].as_array().unwrap().len(), 3);
        assert!(!resp.budget_exceeded);
    }

    #[test]
    fn fixture_validate_reads_default_fixture_paths_from_disk() {
        let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let inputs: Vec<EngineInput> =
            vec!["examples/cem-ml/login.cem", "examples/cem-ml/profile.cem"]
                .into_iter()
                .map(|p| EngineInput {
                    uri: workspace.join(p).to_string_lossy().into_owned(),
                    bytes: Vec::new(),
                    from_format: None,
                    identity: None,
                    root_scope: Default::default(),
                })
                .collect();
        let req = FixtureValidateRequest {
            inputs,
            fail_level: FailLevel::Validate,
            zero_hard_violations: true,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().fixture_validate(req).unwrap();
        assert_eq!(resp.report.summary.hard_violation_count, 0);
        assert_eq!(resp.report.summary.input_count, 2);
    }

    #[test]
    fn fixture_roundtrip_renders_html_for_each_input() {
        let bytes = std::fs::read(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/cem-ml/login.cem"),
        )
        .unwrap();
        let req = FixtureRoundtripRequest {
            inputs: vec![input(&bytes, "login.cem")],
            to_format: LayerFormat::DomJson,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().fixture_roundtrip(req).unwrap();
        assert_eq!(resp.artifacts.len(), 1);
        let rendered = resp.artifacts[0]["rendered"].as_str().unwrap();
        assert!(rendered.contains("<main"));
        assert!(rendered.contains("cem:screen"));
    }
}
