//! Inert authored control sources retain their typed model and native syntax.
//! Loading never prepares argv, follows a resource URI or normalizes authored data.
use crate::{
    cli_command::{self, CliCommandResource, CLI_COMMAND_CONTENT_TYPE, CLI_COMMAND_SCHEMA_URI},
    diagnostics::{Diagnostic, Severity},
    engine::{EngineInput, FormatIdentity, InputFormat},
    lifecycle::{LoadedInput, LoadedInputAstStream},
    parser::document::CemDocument,
    run_config::{self, RunConfig, RUN_CONFIG_SCHEMA_URI},
    studio_project::{
        self, StudioProject, STUDIO_PROJECT_CEM_CONTENT_TYPE, STUDIO_PROJECT_JSON_CONTENT_TYPE,
        STUDIO_PROJECT_SCHEMA_URI,
    },
    validation::json::{
        json_document_ast_from_source_bytes, JsonDocumentAst, JsonSourceValidationRequest,
    },
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ControlSourceValue {
    Command(CliCommandResource),
    RunConfiguration(RunConfig),
    Project(StudioProject),
}
#[derive(Debug, Clone)]
pub enum ControlSourceSyntax {
    Json(JsonDocumentAst),
    Cem {
        document: Arc<CemDocument>,
        text: String,
    },
}
#[derive(Debug, Clone)]
pub struct ControlDocumentAst {
    pub value: ControlSourceValue,
    pub syntax: ControlSourceSyntax,
    pub uri: String,
    pub content_type: String,
    pub schema: &'static str,
    pub byte_length: usize,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum ControlSourceKind {
    Command,
    RunConfiguration,
    Project,
}
impl ControlSourceKind {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Command => "cli-command",
            Self::RunConfiguration => "run-config",
            Self::Project => "studio-project",
        }
    }
    fn schema(self) -> &'static str {
        match self {
            Self::Command => CLI_COMMAND_SCHEMA_URI,
            Self::RunConfiguration => RUN_CONFIG_SCHEMA_URI,
            Self::Project => STUDIO_PROJECT_SCHEMA_URI,
        }
    }
    fn default_content_type(self) -> &'static str {
        match self {
            Self::Command => CLI_COMMAND_CONTENT_TYPE,
            Self::RunConfiguration => "application/json",
            Self::Project => STUDIO_PROJECT_CEM_CONTENT_TYPE,
        }
    }
}
pub(crate) fn is_control_identity(identity: &FormatIdentity) -> bool {
    matches!(
        identity.schema.as_deref().map(str::trim),
        Some(CLI_COMMAND_SCHEMA_URI | RUN_CONFIG_SCHEMA_URI | STUDIO_PROJECT_SCHEMA_URI)
    ) || matches!(
        identity
            .content_type
            .as_deref()
            .map(crate::schema::registry::content_type_essence)
            .as_deref(),
        Some(
            CLI_COMMAND_CONTENT_TYPE
                | STUDIO_PROJECT_CEM_CONTENT_TYPE
                | STUDIO_PROJECT_JSON_CONTENT_TYPE
        )
    )
}

pub(crate) fn load(
    input: &EngineInput,
    identity: &FormatIdentity,
    kind: ControlSourceKind,
) -> LoadedInput {
    let content_type = identity
        .content_type
        .as_deref()
        .or(input.root_scope.default_content_type.as_deref())
        .unwrap_or(kind.default_content_type());
    let typed = match kind {
        ControlSourceKind::Command => {
            cli_command::parse_cli_command(&input.bytes, content_type, kind.schema())
                .map(|value| (ControlSourceValue::Command(value), None, Vec::new()))
                .map_err(|e| (e.code, e.message, e.field_path))
        }
        ControlSourceKind::RunConfiguration => {
            run_config::parse_run_config_source_bytes(&input.bytes, identity)
                .map(|parsed| {
                    let diagnostics =
                        run_config::validate_run_config(&parsed.config, Some(&input.uri));
                    (
                        ControlSourceValue::RunConfiguration(parsed.config),
                        None,
                        diagnostics,
                    )
                })
                .map_err(|e| (e.code, e.message, None))
        }
        ControlSourceKind::Project => studio_project::parse_studio_project_with_syntax(
            &input.bytes,
            content_type,
            kind.schema(),
        )
        .map(|(value, syntax)| (ControlSourceValue::Project(value), syntax, Vec::new()))
        .map_err(|e| (e.code, e.message, e.field_path)),
    };
    let (ast_stream, diagnostics) = match typed {
        Err((code, message, field_path)) => (
            None,
            vec![Diagnostic {
                uri: Some(input.uri.clone()),
                code: code.into(),
                severity: Severity::Error,
                message,
                details: Some(serde_json::json!({"fieldPath": field_path, "lifecycle": {
                    "adapterId": kind.id(), "operation": "load", "profile": format!("{}-v1",kind.id())
                }})),
                ..Default::default()
            }],
        ),
        Ok((value, syntax, mut diagnostics)) => {
            let syntax = if let Some(document) = syntax {
                Some(ControlSourceSyntax::Cem {
                    document: Arc::new(document),
                    text: String::from_utf8(input.bytes.clone()).expect("admitted CEM UTF-8"),
                })
            } else {
                let (syntax, native_diagnostics) =
                    json_document_ast_from_source_bytes(JsonSourceValidationRequest {
                        bytes: &input.bytes,
                        source_uri: &input.uri,
                        content_type: Some(content_type),
                    });
                diagnostics.extend(native_diagnostics);
                syntax.map(ControlSourceSyntax::Json)
            };
            let document = syntax
                .filter(|_| !diagnostics.iter().any(|d| d.severity.is_hard_violation()))
                .map(|syntax| {
                    LoadedInputAstStream::ControlDocument(ControlDocumentAst {
                        value,
                        syntax,
                        uri: input.uri.clone(),
                        content_type: content_type.into(),
                        schema: kind.schema(),
                        byte_length: input.bytes.len(),
                    })
                });
            (document, diagnostics)
        }
    };
    LoadedInput {
        bytes: input.bytes.clone(),
        from_format: input.from_format.unwrap_or(InputFormat::Cem),
        ast_stream,
        diagnostics,
        adapter_id: Some(kind.id()),
    }
}
