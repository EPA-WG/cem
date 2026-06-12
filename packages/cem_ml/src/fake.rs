//! Feature-gated fake engine used by `cem-ml-cli` feature tests.
//!
//! Returns deterministic, contract-shaped responses without doing any real parsing.
//! Only enable via `--features fake-engine`. The real engine arrives in Phase 11 of
//! `cem-ml-cli-plan.md`.

use crate::diagnostics::{Diagnostic, Severity};
use crate::engine::*;
use crate::report::{Report, ReportOptionsSnapshot};
use serde_json::json;

#[derive(Debug, Default)]
pub struct FakeEngine;

impl FakeEngine {
    pub fn new() -> Self {
        Self
    }
}

fn snapshot(fail_level: FailLevel, ctx: &EngineContext) -> ReportOptionsSnapshot {
    ReportOptionsSnapshot {
        fail_level,
        schema: ctx.schema.clone(),
        content_type: ctx.content_type.clone(),
        base_uri: ctx.base_uri.clone(),
    }
}

fn input_uris(inputs: &[EngineInput]) -> Vec<String> {
    inputs.iter().map(|i| i.uri.clone()).collect()
}

fn fake_diagnostic(uri: &str) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.to_owned()),
        line: Some(1),
        column: Some(1),
        byte_offset: Some(0),
        code: "fake.engine.placeholder".to_owned(),
        severity: Severity::Info,
        message: "fake engine: parser not yet implemented".to_owned(),
        node: None,
        source_map: None,
    }
}

impl CemMlEngine for FakeEngine {
    fn parse(&self, request: ParseRequest) -> EngineResult<ParseResponse> {
        let primary = json!({
            "kind": "fake-parse",
            "projection": request.projection,
            "input": request.input.uri,
            "preserveSourceOffsets": request.preserve_source_offsets,
        });
        Ok(ParseResponse {
            primary,
            diagnostics: vec![fake_diagnostic(&request.input.uri)],
        })
    }

    fn validate(&self, request: ValidateRequest) -> EngineResult<ValidateResponse> {
        let inputs = input_uris(&request.inputs);
        let diagnostics: Vec<Diagnostic> = inputs.iter().map(|u| fake_diagnostic(u)).collect();
        let report = Report::deterministic(
            inputs,
            diagnostics,
            snapshot(request.fail_level, &request.context),
        );
        Ok(ValidateResponse { report })
    }

    fn check(&self, request: CheckRequest) -> EngineResult<CheckResponse> {
        let inputs = input_uris(&request.inputs);
        let diagnostics: Vec<Diagnostic> = inputs.iter().map(|u| fake_diagnostic(u)).collect();
        let report = Report::deterministic(
            inputs,
            diagnostics,
            snapshot(request.fail_level, &request.context),
        );
        let hard_violation_count = report.summary.hard_violation_count;
        Ok(CheckResponse {
            report,
            hard_violation_count,
        })
    }

    fn inspect(&self, request: InspectRequest) -> EngineResult<InspectResponse> {
        let body = json!({
            "kind": "fake-inspect",
            "view": request.show,
            "input": request.input.uri,
        });
        Ok(InspectResponse {
            view: request.show,
            body,
        })
    }

    fn convert(&self, request: ConvertRequest) -> EngineResult<ConvertResponse> {
        let primary = json!({
            "kind": "fake-convert",
            "toFormat": request.to_format,
            "input": request.input.uri,
            "preserveSourceOffsets": request.preserve_source_offsets,
        });
        Ok(ConvertResponse {
            primary,
            diagnostics: vec![],
        })
    }

    fn trace(&self, request: TraceRequest) -> EngineResult<TraceResponse> {
        let body = json!({
            "kind": "fake-trace",
            "projection": request.projection,
            "input": request.input.uri,
        });
        Ok(TraceResponse { body })
    }

    fn bench(&self, request: BenchRequest) -> EngineResult<BenchResponse> {
        let body = json!({
            "kind": "fake-bench",
            "projection": request.projection,
            "inputs": input_uris(&request.inputs),
            "iterations": request.iterations,
            "budgetMs": request.budget_ms,
            "profile": request.profile,
            "coldCache": request.cold_cache,
        });
        Ok(BenchResponse {
            body,
            budget_exceeded: false,
        })
    }

    fn fixture_validate(
        &self,
        request: FixtureValidateRequest,
    ) -> EngineResult<FixtureValidateResponse> {
        let inputs = input_uris(&request.inputs);
        let diagnostics: Vec<Diagnostic> = inputs.iter().map(|u| fake_diagnostic(u)).collect();
        let report = Report::deterministic(
            inputs,
            diagnostics,
            snapshot(request.fail_level, &request.context),
        );
        Ok(FixtureValidateResponse { report })
    }

    fn fixture_roundtrip(
        &self,
        request: FixtureRoundtripRequest,
    ) -> EngineResult<FixtureRoundtripResponse> {
        let inputs = input_uris(&request.inputs);
        let artifacts: Vec<serde_json::Value> = inputs
            .iter()
            .map(|u| {
                json!({
                    "input": u,
                    "toFormat": request.to_format,
                })
            })
            .collect();
        let diagnostics: Vec<Diagnostic> = inputs.iter().map(|u| fake_diagnostic(u)).collect();
        let report = Report::deterministic(
            inputs,
            diagnostics,
            snapshot(FailLevel::Validate, &request.context),
        );
        Ok(FixtureRoundtripResponse { report, artifacts })
    }
}
