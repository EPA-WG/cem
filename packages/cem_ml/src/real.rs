//! Real (parser-enabled) `CemMlEngine` implementation.
//!
//! Bridges the library pipeline (tokenize → normalize → schema-validate
//! → AST build → validation rules → render) into the `CemMlEngine` trait
//! that `cem-ml-cli` calls through. This is the production engine that
//! replaces `NotImplementedEngine` in `cem-ml-cli/src/main.rs`.

use crate::diagnostics::Diagnostic;
use crate::engine::*;
use crate::events::cem::CemEventNormalizer;
use crate::formatter;
use crate::interpreter::light_dom::LightDomInterpreter;
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::projection;
use crate::report::{Report, ReportOptionsSnapshot};
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

fn run_pipeline(bytes: &[u8]) -> PipelineRun {
    run_pipeline_as(bytes, InputFormat::Cem)
}

fn run_pipeline_as(bytes: &[u8], from_format: InputFormat) -> PipelineRun {
    match from_format {
        InputFormat::Cem => run_pipeline_with::<CemTokenizer>(bytes),
        InputFormat::Html => run_pipeline_with::<HtmlTokenizer>(bytes),
        InputFormat::Xml => run_pipeline_with::<XmlTokenizer>(bytes),
    }
}

fn run_pipeline_with<T>(bytes: &[u8]) -> PipelineRun
where
    T: SchemaTokenizer + FromBytes,
{
    // Schema-machine pass.
    let schema_outcome = {
        let src = BytesSource::new(SourceId(1), bytes.to_vec());
        let tok = T::from_bytes(src);
        let normalizer = CemEventNormalizer::new(tok);
        CemSchemaMachine::new(CompiledSchema::cem_core(), normalizer).run()
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

fn input_uris(inputs: &[EngineInput]) -> Vec<String> {
    inputs.iter().map(|i| i.uri.clone()).collect()
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
        let from_format = request.input.from_format.unwrap_or(InputFormat::Cem);
        let run = run_pipeline_as(&request.input.bytes, from_format);
        let primary = match request.projection {
            ParseProjection::DomJson | ParseProjection::Json => projection::dom_json(&run.document),
            ParseProjection::Ast => projection::ast_json(&run.document),
            ParseProjection::Events => {
                projection::events_json_as(&request.input.bytes, from_format)
            }
        };
        Ok(ParseResponse {
            primary,
            diagnostics: run.diagnostics,
        })
    }

    fn validate(&self, request: ValidateRequest) -> EngineResult<ValidateResponse> {
        let inputs = input_uris(&request.inputs);
        let mut all_diags: Vec<Diagnostic> = Vec::new();
        for input in &request.inputs {
            let run = run_pipeline_as(&input.bytes, input.from_format.unwrap_or(InputFormat::Cem));
            all_diags.extend(run.diagnostics);
        }
        let report = Report::deterministic(
            inputs,
            all_diags,
            snapshot(request.fail_level, &request.context),
        );
        Ok(ValidateResponse { report })
    }

    fn check(&self, request: CheckRequest) -> EngineResult<CheckResponse> {
        let inputs = input_uris(&request.inputs);
        let mut all_diags: Vec<Diagnostic> = Vec::new();
        for input in &request.inputs {
            let run = run_pipeline_as(&input.bytes, input.from_format.unwrap_or(InputFormat::Cem));
            all_diags.extend(run.diagnostics);
        }
        let report = Report::deterministic(
            inputs,
            all_diags,
            snapshot(request.fail_level, &request.context),
        );
        let hard_violation_count = report.summary.hard_violation_count;
        Ok(CheckResponse {
            report,
            hard_violation_count,
        })
    }

    fn inspect(&self, request: InspectRequest) -> EngineResult<InspectResponse> {
        let from_format = request.input.from_format.unwrap_or(InputFormat::Cem);
        let run = run_pipeline_as(&request.input.bytes, from_format);
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
                    "input": request.input.uri,
                    "elements": elements,
                    "attributes": attributes,
                    "diagnosticCount": run.diagnostics.len(),
                })
            }
            InspectView::Ast => projection::ast_json(&run.document),
            InspectView::Events => projection::events_json_as(&request.input.bytes, from_format),
            InspectView::Diagnostics => json!({
                "kind": "diagnostics",
                "input": request.input.uri,
                "diagnostics": run.diagnostics,
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
                    "input": request.input.uri,
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
        let from_format = request.input.from_format.unwrap_or(InputFormat::Cem);
        let run = run_pipeline_as(&request.input.bytes, from_format);
        let primary = match request.to_format {
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
            LayerFormat::Events => projection::events_json_as(&request.input.bytes, from_format),
        };
        Ok(ConvertResponse {
            primary,
            diagnostics: run.diagnostics,
        })
    }

    fn trace(&self, request: TraceRequest) -> EngineResult<TraceResponse> {
        let from_format = request.input.from_format.unwrap_or(InputFormat::Cem);
        let scheduler_trace = crate::scheduler::SchedulerTrace::new();
        let policy = crate::scheduler::ScopePolicy {
            cpu_workers: 1,
            queue_size: 8,
            io_streams: 4,
            memory_bytes: 8 * 1024 * 1024,
            plugin_time_budget_ms: None,
            overflow: crate::scheduler::OverflowPolicy::Reject,
        };
        let pool = crate::scheduler::WorkerPool::new(0, policy, scheduler_trace.clone());
        let abort = crate::scheduler::AbortSignal::new();
        for task in ["tokenize", "normalize", "schema", "ast", "validate"] {
            pool.submit(task, &abort).map_err(|err| {
                EngineError::Internal(format!("scheduler trace setup failed: {err}"))
            })?;
        }
        let run = run_pipeline_as(&request.input.bytes, from_format);
        pool.run_to_completion(&abort, |_| {});
        let report = Report::deterministic(
            vec![request.input.uri.clone()],
            run.diagnostics,
            snapshot(FailLevel::Validate, &request.context),
        )
        .with_scheduler_trace(&scheduler_trace);
        let body = json!({
            "kind": "trace",
            "input": request.input.uri,
            "projection": request.projection,
            "events": projection::events_json_as(&request.input.bytes, from_format),
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
                let _ = run_pipeline(&input.bytes);
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
        let inputs = input_uris(&request.inputs);
        let mut all_diags: Vec<Diagnostic> = Vec::new();
        for input in &request.inputs {
            let bytes = if input.bytes.is_empty() {
                // Default fixtures arrive with bytes left blank by the CLI
                // dispatcher (placeholder_input); read from disk now.
                match std::fs::read(&input.uri) {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(EngineError::Io {
                            path: input.uri.clone().into(),
                            source: e,
                        });
                    }
                }
            } else {
                input.bytes.clone()
            };
            let run = run_pipeline_as(&bytes, input.from_format.unwrap_or(InputFormat::Cem));
            all_diags.extend(run.diagnostics);
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
        let inputs = input_uris(&request.inputs);
        let mut artifacts: Vec<Value> = Vec::new();
        let mut all_diags: Vec<Diagnostic> = Vec::new();
        for input in &request.inputs {
            let bytes = if input.bytes.is_empty() {
                match std::fs::read(&input.uri) {
                    Ok(b) => b,
                    Err(e) => {
                        return Err(EngineError::Io {
                            path: input.uri.clone().into(),
                            source: e,
                        });
                    }
                }
            } else {
                input.bytes.clone()
            };
            let run = run_pipeline_as(&bytes, input.from_format.unwrap_or(InputFormat::Cem));
            let rendered = LightDomInterpreter::new().render(&run.document);
            artifacts.push(json!({
                "input": input.uri,
                "toFormat": request.to_format,
                "rendered": rendered.rendered,
            }));
            all_diags.extend(run.diagnostics);
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
    fn convert_dom_json_returns_document_tree() {
        let req = ConvertRequest {
            input: input(b"{p Hi}", "in"),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "document");
    }

    #[test]
    fn convert_html_to_canonical_cem_ml_returns_source_map() {
        let req = ConvertRequest {
            input: EngineInput {
                uri: "in.html".to_owned(),
                bytes: br#"<button cem:action="primary" type="submit">Save</button>"#.to_vec(),
                from_format: Some(InputFormat::Html),
            },
            to_format: LayerFormat::Cem,
            preserve_source_offsets: true,
            context: ctx(),
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
            },
            to_format: LayerFormat::Cem,
            preserve_source_offsets: true,
            context: ctx(),
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
