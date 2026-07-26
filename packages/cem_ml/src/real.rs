//! Real (parser-enabled) `CemMlEngine` implementation.
//!
//! Bridges the library pipeline (tokenize → normalize → schema-validate
//! → AST build → validation rules → render) into the `CemMlEngine` trait
//! that `cem-ml-cli` calls through. This is the production engine that
//! replaces `NotImplementedEngine` in `cem-ml-cli/src/main.rs`.

use crate::conversion::{
    conversion_descriptors_from_validated_schema_package_manifest,
    conversion_output_safety_contract,
    conversion_package_artifacts_from_validated_schema_package_manifest,
    direct_cem_output_pipeline, direct_html_output_pipeline, direct_xml_output_pipeline,
    execute_conversion_output_pipeline,
    execute_conversion_output_pipeline_from_formatted_cem_tree_with_environment,
    execute_conversion_output_pipeline_with_environment,
    execute_csv_document_output_pipeline_with_environment, ConversionExecution,
    ConversionOutputPipeline, ConversionOutputPipelineEnvironment,
    ConversionPackageArtifactDescriptor, ConversionPackageArtifactRead,
    ConversionRustFallbackDescriptor, GenericDataTextConversionOutcome, GenericDataTextDocument,
    CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE,
};
use crate::diagnostics::{
    project_diagnostics_for_source, source_map_coordinate_details, Diagnostic, Severity,
};
use crate::engine::*;
use crate::events::cem::CemEventNormalizer;
use crate::interpreter::light_dom::LightDomInterpreter;
use crate::interpreter::xml::XmlInterpreter;
use crate::interpreter::OutputSpan;
use crate::lifecycle::{ExportSelection, LifecycleRegistry, LoadedInput};
use crate::parser::builder::CemAstBuilder;
use crate::parser::document::CemDocument;
use crate::parser::format;
use crate::projection;
use crate::report::{Report, ReportOptionsSnapshot};
use crate::resolver::{
    has_uri_scheme, is_windows_drive_path, parse_local_file_uri, ResolveDirection,
    ResolvePolicyDecision, ResolvePolicyDiagnostic, ResolvePurpose, ResolveRequest, ResolvedRead,
    ResolverDiagnostic,
};
use crate::run_config::ScopeConfig;
use crate::schema::document_model::compile_schema_document_model;
use crate::schema::machine::CemSchemaMachine;
use crate::schema::package_consistency::validate_schema_package_source_consistency;
use crate::schema::registry::{
    content_type_essence, schema_descriptor_from_manifest_and_schema_sources,
    schema_source_path_from_manifest_source, SchemaDescriptor,
    CEM_DOM_JSON_PROJECTION_CONTENT_TYPE, CEM_DOM_PROJECTION_CONTENT_TYPE,
    CEM_DOM_PROJECTION_SCHEMA_URI, CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
    CEM_NATIVE_TEMPLATE_SCHEMA_URI, CEM_SCHEMA_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
    CEM_SCHEMA_PACKAGE_URI, CSS_CONTENT_TYPE, CSS_SCHEMA_URI, CSV_CONTENT_TYPE, CSV_SCHEMA_URI,
    HTML_CONTENT_TYPE, HTML_SCHEMA_URI, XHTML_CONTENT_TYPE, XHTML_SCHEMA_URI, XML_CONTENT_TYPE,
    XML_SCHEMA_URI,
};
use crate::schema::vocab::CompiledSchema;
use crate::source::line_index::LineIndex;
use crate::source::{ByteRange, BytesSource, SourceId};
use crate::source_map::SourceMapStack;
use crate::tokenizer::cem::CemTokenizer;
use crate::tokenizer::html::HtmlTokenizer;
use crate::tokenizer::xml::XmlTokenizer;
use crate::tokenizer::SchemaTokenizer;
use crate::transform_config::{
    parse_transform_graph_config, TransformGraphParseRequest, TRANSFORM_CONFIG_SCHEMA_URI,
};
use crate::transform_template::{
    evaluate_transform_template_encode_expressions, parse_cem_native_template_module_options,
    parse_transform_template_output_color_type, transform_template_call_argument_is_dynamic,
    transform_template_encode_options_line_ending_data, try_apply_transform_template_let_bindings,
    TransformTemplateAdapter, TransformTemplateAdapterLookup, TransformTemplateCompileRequest,
    TransformTemplateCompiledArtifact, TransformTemplateDataArtifact,
    TransformTemplateEncodeEvaluationContext, TransformTemplateEncodedArtifactInsertionContext,
    TransformTemplateModuleCacheKey, TransformTemplateModuleDependencyKind,
    TransformTemplateModuleImport, TransformTemplateModuleOptions,
    TransformTemplateModuleParamDeclaration, TransformTemplateModuleParamType,
    TransformTemplateModuleParseRequest, TransformTemplateModulePreflight,
    TransformTemplateModuleVisibility, TransformTemplateOutputArtifact,
    TransformTemplateOutputColorSelection, TransformTemplateOutputFunctionRegistry,
    TransformTemplateOutputProducedKind, TransformTemplateRenderRequest,
    TransformTemplateResolvedModule, CEM_TEMPLATE_IMPORT_DENIED_CODE,
    CEM_TEMPLATE_IMPORT_UNRESOLVED_CODE, TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE,
    TRANSFORM_TEMPLATE_ENTRYPOINT_NOT_PUBLIC_CODE, TRANSFORM_TEMPLATE_IMPORT_ALIAS_DUPLICATE_CODE,
    TRANSFORM_TEMPLATE_IMPORT_CYCLE_CODE, TRANSFORM_TEMPLATE_IMPORT_DEPTH_CODE,
    TRANSFORM_TEMPLATE_INCLUDE_RESERVED_CODE, TRANSFORM_TEMPLATE_LET_EXPR_INVALID_CODE,
    TRANSFORM_TEMPLATE_PARAM_DUPLICATE_ALIAS_CODE, TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE,
    TRANSFORM_TEMPLATE_PARAM_TYPE_CODE, TRANSFORM_TEMPLATE_PARAM_UNKNOWN_CODE,
};
use crate::validation::{
    rules::validate_cem_native_template_source_semantics, RuleContext, RuleRegistry,
    RuleResourceRead, RuleResourceReader,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Default, Clone)]
pub struct RealCemMlEngine;

#[derive(Debug, Default, Clone)]
struct TransformOutputMetadata {
    source_map: Option<SourceMapStack>,
    output_spans: Vec<OutputSpan>,
    raw_content: Option<String>,
}

impl RealCemMlEngine {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
struct ExportConversionExecution {
    converter_id: String,
    source: FormatIdentity,
    target: FormatIdentity,
    execution: ConversionExecution,
    rust_fallback: Option<ConversionRustFallbackDescriptor>,
    output_pipeline: Option<ConversionOutputPipeline>,
    output_pipeline_diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
enum ExportConversionTemplateResult {
    Rendered(Value),
    UseDirectPipeline,
    UseRustFallback,
    Failed,
}

fn trimmed_scope_value(value: Option<&String>) -> Option<&str> {
    value
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
}

fn canonical_cemt_formatter_profile_selector(selector: &str) -> String {
    match selector.trim() {
        "compact" | "pretty" | "tabular" => selector.trim().to_owned(),
        "format-tree" | "cem.format-tree" | "canonical" => "compact".to_owned(),
        other => other.to_owned(),
    }
}

fn apply_cemt_output_scope_overrides(
    pipeline: &mut ConversionOutputPipeline,
    target_scope: &ScopeConfig,
) {
    if let Some(formatter) = trimmed_scope_value(target_scope.cemt_formatter.as_ref()) {
        pipeline.cemt_options.formatter = Some(formatter.to_owned());
    }
    let formatter_profile = trimmed_scope_value(target_scope.cemt_formatter_profile.as_ref())
        .or_else(|| trimmed_scope_value(target_scope.cemt_formatter.as_ref()));
    if let Some(formatter_profile) = formatter_profile {
        let formatter_profile = canonical_cemt_formatter_profile_selector(formatter_profile);
        pipeline.cemt_options.formatter_profile = Some(formatter_profile.clone());
        pipeline.cemt_insertion_context.formatter_profile = Some(formatter_profile.clone());
        pipeline.writer_insertion_context.formatter_profile = Some(formatter_profile);
    }
    pipeline
        .cemt_options
        .formatter_options
        .extend(target_scope.cemt_formatter_options.clone());
    if let Some(colorizer) = trimmed_scope_value(target_scope.cemt_colorizer.as_ref()) {
        pipeline.cemt_options.colorizer = Some(colorizer.to_owned());
    }
    if let Some(color_profile) = trimmed_scope_value(target_scope.cemt_color_profile.as_ref()) {
        pipeline.cemt_options.color_profile = Some(color_profile.to_owned());
        pipeline.cemt_insertion_context.color_profile = Some(color_profile.to_owned());
        pipeline.writer_insertion_context.color_profile = Some(color_profile.to_owned());
    }
}

fn cemt_output_pipeline_for_scope(
    mut pipeline: ConversionOutputPipeline,
    target_scope: &ScopeConfig,
) -> ConversionOutputPipeline {
    apply_cemt_output_scope_overrides(&mut pipeline, target_scope);
    pipeline
}

fn convert_metadata_for_cemt_converter(
    conversion: &ExportConversionExecution,
    pipeline: Option<&ConversionOutputPipeline>,
) -> ConvertExecutionMetadata {
    ConvertExecutionMetadata {
        converter_id: Some(conversion.converter_id.clone()),
        implementation: Some("cemt".to_owned()),
        rust_fallback: None,
        output_pipeline: pipeline.map(convert_output_pipeline_metadata),
    }
}

fn convert_metadata_for_rust_fallback(
    conversion: &ExportConversionExecution,
) -> ConvertExecutionMetadata {
    ConvertExecutionMetadata {
        converter_id: Some(conversion.converter_id.clone()),
        implementation: Some("rust-fallback".to_owned()),
        rust_fallback: conversion
            .rust_fallback
            .as_ref()
            .map(|fallback| fallback.rust_symbol.clone()),
        output_pipeline: None,
    }
}

fn convert_metadata_for_direct_output_pipeline(
    converter_id: &str,
    pipeline: &ConversionOutputPipeline,
) -> ConvertExecutionMetadata {
    ConvertExecutionMetadata {
        converter_id: Some(converter_id.to_owned()),
        implementation: Some("direct-cemt-output-pipeline".to_owned()),
        rust_fallback: None,
        output_pipeline: Some(convert_output_pipeline_metadata(pipeline)),
    }
}

fn convert_output_pipeline_metadata(
    pipeline: &ConversionOutputPipeline,
) -> ConvertOutputPipelineMetadata {
    ConvertOutputPipelineMetadata {
        stages: vec![
            ConvertOutputPipelineStageMetadata {
                stage: "formatter".to_owned(),
                function: Some(
                    pipeline
                        .cemt_options
                        .formatter
                        .clone()
                        .unwrap_or_else(|| "cem.format-tree".to_owned()),
                ),
                profile: pipeline.cemt_options.formatter_profile.clone(),
                content_type: Some(pipeline.cemt_target.content_type.clone()),
                schema: Some(pipeline.cemt_target.schema.clone()),
                category: Some(pipeline.cemt_target.category.clone()),
                produces: Some(pipeline.cemt_produces.as_str().to_owned()),
            },
            ConvertOutputPipelineStageMetadata {
                stage: "colorizer".to_owned(),
                function: Some(
                    pipeline
                        .cemt_options
                        .colorizer
                        .clone()
                        .unwrap_or_else(|| "cem.color-tree".to_owned()),
                ),
                profile: pipeline.cemt_options.color_profile.clone(),
                content_type: Some(pipeline.cemt_target.content_type.clone()),
                schema: Some(pipeline.cemt_target.schema.clone()),
                category: Some(pipeline.cemt_target.category.clone()),
                produces: Some(pipeline.cemt_produces.as_str().to_owned()),
            },
            ConvertOutputPipelineStageMetadata {
                stage: "writer".to_owned(),
                function: None,
                profile: pipeline
                    .writer_insertion_context
                    .color_profile
                    .clone()
                    .or_else(|| pipeline.cemt_options.color_profile.clone()),
                content_type: Some(pipeline.writer_insertion_context.content_type.clone()),
                schema: Some(pipeline.writer_insertion_context.schema.clone()),
                category: pipeline.writer_insertion_context.category.clone(),
                produces: Some(pipeline.writer_produces.as_str().to_owned()),
            },
        ],
    }
}

fn resolve_export_conversion_execution(
    context: &EngineContext,
    to_format: LayerFormat,
    target: Option<&FormatIdentity>,
) -> Option<ExportConversionExecution> {
    let source = export_conversion_source_identity(to_format)?;
    let target = target
        .cloned()
        .or_else(|| export_conversion_target_identity(to_format))?;

    let execution = context
        .converter_registry
        .resolve_direct_execution(
            &context.schema_registry,
            &context.template_adapter_registry,
            &source,
            &target,
        )
        .ok()?;

    let (contract, output_pipeline_diagnostics) =
        conversion_output_safety_contract(execution.descriptor);

    Some(ExportConversionExecution {
        converter_id: execution.descriptor.id.clone(),
        source: FormatIdentity {
            content_type: Some(execution.source.content_type),
            schema: Some(execution.source.schema),
            ..FormatIdentity::default()
        },
        target: FormatIdentity {
            content_type: Some(execution.target.content_type),
            schema: Some(execution.target.schema),
            ..FormatIdentity::default()
        },
        execution: execution.execution,
        rust_fallback: execution.descriptor.rust_fallback.clone(),
        output_pipeline: contract.map(|contract| contract.pipeline),
        output_pipeline_diagnostics,
    })
}

fn export_conversion_source_identity(to_format: LayerFormat) -> Option<FormatIdentity> {
    match to_format {
        LayerFormat::Html | LayerFormat::Xml | LayerFormat::DomJson => Some(FormatIdentity {
            content_type: Some(CEM_DOM_PROJECTION_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_DOM_PROJECTION_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        }),
        _ => None,
    }
}

fn export_conversion_target_identity(to_format: LayerFormat) -> Option<FormatIdentity> {
    let (content_type, schema) = match to_format {
        LayerFormat::Html => (HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
        LayerFormat::Xml => (XML_CONTENT_TYPE, XML_SCHEMA_URI),
        LayerFormat::DomJson => (
            CEM_DOM_JSON_PROJECTION_CONTENT_TYPE,
            CEM_DOM_PROJECTION_SCHEMA_URI,
        ),
        _ => return None,
    };

    Some(FormatIdentity {
        content_type: Some(content_type.to_owned()),
        schema: Some(schema.to_owned()),
        ..FormatIdentity::default()
    })
}

fn render_export_conversion_template(
    context: &EngineContext,
    conversion: &ExportConversionExecution,
    to_format: LayerFormat,
    document: &CemDocument,
    target_scope: &ScopeConfig,
    diagnostics: &mut Vec<Diagnostic>,
) -> ExportConversionTemplateResult {
    let ConversionExecution::CemtTemplate {
        adapter_id,
        template,
    } = &conversion.execution
    else {
        return ExportConversionTemplateResult::UseDirectPipeline;
    };

    let mut local_diagnostics = Vec::new();
    local_diagnostics.extend(conversion.output_pipeline_diagnostics.clone());
    if local_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return publish_converter_cemt_diagnostics(
            conversion,
            "output safety contract validation failed",
            local_diagnostics,
            diagnostics,
        );
    }

    let Some(adapter) =
        select_converter_template_adapter(context, template, *adapter_id, &mut local_diagnostics)
    else {
        return fallback_or_publish_converter_diagnostics(
            conversion,
            "template adapter selection failed",
            local_diagnostics,
            diagnostics,
        );
    };

    let Some(template_input) = load_converter_template(context, template, &mut local_diagnostics)
    else {
        return fallback_or_publish_converter_diagnostics(
            conversion,
            "template asset loading failed",
            local_diagnostics,
            diagnostics,
        );
    };

    let params = BTreeMap::new();
    let data_bindings = vec!["input".to_owned()];
    let entrypoint = template
        .entrypoint
        .as_deref()
        .map(TransformTemplateEntrypoint::named)
        .unwrap_or_else(TransformTemplateEntrypoint::implicit);
    let execution_policy = TransformExecutionPolicy::default();
    let Some(compiled) = compile_transform_template(
        TransformTemplateCompileSpec {
            context,
            adapter: &adapter,
            template: &template_input,
            template_kind: adapter.kind(),
            entrypoint: &entrypoint,
            params: &params,
            data_bindings: &data_bindings,
            module_options: TransformTemplateModuleOptions::default(),
            execution_policy,
        },
        &mut local_diagnostics,
    ) else {
        return publish_converter_cemt_diagnostics(
            conversion,
            "template compilation failed",
            local_diagnostics,
            diagnostics,
        );
    };

    let primary_input = TransformTemplateDataArtifact {
        artifact_id: "input".to_owned(),
        uri: None,
        identity: Some(conversion.source.clone()),
        value: projection::dom_json(document),
    };
    let secondary_inputs = BTreeMap::new();
    let render_target = conversion
        .output_pipeline
        .as_ref()
        .map(|pipeline| pipeline.cemt_target.format_identity())
        .unwrap_or_else(|| conversion.target.clone());
    let Some(output) = render_transform_stage(
        TransformStageRenderSpec {
            context,
            adapter: &adapter,
            compiled: &compiled,
            primary_input: &primary_input,
            secondary_inputs: &secondary_inputs,
            target: Some(&render_target),
            target_scope,
            execution_policy,
            diagnostic_uri: &template_input.uri,
            diagnostic_node: None,
        },
        &mut local_diagnostics,
    ) else {
        return publish_converter_cemt_diagnostics(
            conversion,
            "template rendering failed",
            local_diagnostics,
            diagnostics,
        );
    };

    let output = if let Some(pipeline) = conversion.output_pipeline.as_ref() {
        let output_uri = output.uri.clone();
        let pipeline = cemt_output_pipeline_for_scope(pipeline.clone(), target_scope);
        let pipeline_execution =
            execute_conversion_output_pipeline_from_formatted_cem_tree_with_context(
                context,
                &pipeline,
                output.value,
                output.source_map,
                output.output_spans,
                &conversion.converter_id,
                Some("output"),
                Some(&template_input.uri),
            );
        local_diagnostics.extend(pipeline_execution.diagnostics);
        if local_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            return publish_converter_cemt_diagnostics(
                conversion,
                "CEMT output pipeline failed",
                local_diagnostics,
                diagnostics,
            );
        }
        let Some(value) = pipeline_execution.output else {
            return publish_converter_cemt_diagnostics(
                conversion,
                "CEMT output pipeline produced no writer artifact",
                local_diagnostics,
                diagnostics,
            );
        };
        TransformTemplateOutputArtifact {
            uri: output_uri,
            identity: Some(conversion.target.clone()),
            value,
            source_map: pipeline_execution.source_map,
            output_spans: pipeline_execution.output_spans,
        }
    } else {
        output
    };

    let Some(primary) = convert_primary_from_template_output(output, to_format) else {
        return publish_converter_cemt_diagnostics(
            conversion,
            "template output did not match the selected target content type",
            local_diagnostics,
            diagnostics,
        );
    };

    diagnostics.append(&mut local_diagnostics);
    ExportConversionTemplateResult::Rendered(primary)
}

fn select_converter_template_adapter(
    context: &EngineContext,
    template: &crate::conversion::ConversionTemplateDescriptor,
    expected_adapter_id: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Arc<dyn TransformTemplateAdapter>> {
    let identity = converter_template_identity(template);
    match context.template_adapter_registry.select_adapter(&identity) {
        TransformTemplateAdapterLookup::Matched(adapter) if adapter.id() == expected_adapter_id => {
            Some(adapter)
        }
        TransformTemplateAdapterLookup::Matched(adapter) => {
            diagnostics.push(converter_template_diagnostic(
                Some(&template.path),
                "cem.converter.adapter_changed",
                Severity::Fatal,
                format!(
                    "converter template adapter changed from `{expected_adapter_id}` to `{}`",
                    adapter.id()
                ),
            ));
            None
        }
        TransformTemplateAdapterLookup::Ambiguous(ids) => {
            diagnostics.push(converter_template_diagnostic(
                Some(&template.path),
                "cem.converter.adapter_ambiguous",
                Severity::Fatal,
                format!(
                    "converter template identity matched multiple adapters: {}",
                    ids.join(", ")
                ),
            ));
            None
        }
        TransformTemplateAdapterLookup::Unsupported => {
            diagnostics.push(converter_template_diagnostic(
                Some(&template.path),
                "cem.converter.adapter_unsupported",
                Severity::Fatal,
                "no converter template adapter matched template identity",
            ));
            None
        }
    }
}

fn converter_template_identity(
    template: &crate::conversion::ConversionTemplateDescriptor,
) -> FormatIdentity {
    FormatIdentity {
        content_type: Some(template.content_type.clone()),
        schema: template.schema.clone(),
        ..FormatIdentity::default()
    }
}

fn load_converter_template(
    context: &EngineContext,
    template: &crate::conversion::ConversionTemplateDescriptor,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TemplateInput> {
    match read_converter_template(context, template) {
        Ok(read) => Some(TemplateInput {
            uri: read.uri,
            bytes: read.bytes,
            identity: Some(converter_template_identity(template)),
            root_scope: ScopeConfig::default(),
        }),
        Err(error) => {
            diagnostics.push(converter_template_diagnostic(
                Some(&template.path),
                "cem.converter.template_unreadable",
                Severity::Fatal,
                error.to_string(),
            ));
            None
        }
    }
}

fn read_converter_template(
    context: &EngineContext,
    template: &crate::conversion::ConversionTemplateDescriptor,
) -> Result<ResolvedRead, ResolverDiagnostic> {
    let content_type_hint = Some(template.content_type.as_str());
    let uri = template.path.as_str();
    if has_uri_scheme(uri) && !is_windows_drive_path(uri) {
        if let Some(path) = parse_local_file_uri(uri).transpose().map_err(|error| {
            ResolverDiagnostic::InvalidFileUri {
                uri: uri.to_owned(),
                message: error.to_string(),
            }
        })? {
            return read_local_template_import(uri, path, content_type_hint);
        }
        if let Some(read) = read_registered_resource(
            Some(context),
            uri,
            ResolvePurpose::Template,
            content_type_hint,
        )? {
            return Ok(read);
        }
        return Err(ResolverDiagnostic::UnsupportedResolver {
            uri: uri.to_owned(),
            purpose: ResolvePurpose::Template,
            direction: ResolveDirection::Read,
        });
    }

    let mut last_error = None;
    for path in converter_template_candidate_paths(uri) {
        match std::fs::read(&path) {
            Ok(bytes) => {
                return Ok(ResolvedRead {
                    uri: path.to_string_lossy().into_owned(),
                    bytes,
                    content_type: content_type_hint.map(str::to_owned),
                });
            }
            Err(error) => {
                last_error = Some((path, error));
            }
        }
    }

    let (path, error) =
        last_error.unwrap_or_else(|| (PathBuf::from(uri), std::io::ErrorKind::NotFound.into()));
    Err(ResolverDiagnostic::Io {
        uri: path.to_string_lossy().into_owned(),
        message: error.to_string(),
    })
}

fn execute_conversion_output_pipeline_with_context(
    context: Option<&EngineContext>,
    pipeline: &ConversionOutputPipeline,
    rendered_value: Value,
    rendered_source_map: Option<SourceMapStack>,
    rendered_output_spans: Vec<OutputSpan>,
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
) -> crate::conversion::ConversionOutputPipelineExecution {
    let Some(context) = context else {
        return execute_conversion_output_pipeline(
            pipeline,
            rendered_value,
            rendered_source_map,
            rendered_output_spans,
            converter_id,
            diagnostic_node,
            diagnostic_uri,
        );
    };

    let package_artifact_reader =
        |artifact: &ConversionPackageArtifactDescriptor| -> Result<ConversionPackageArtifactRead, String> {
            read_conversion_package_artifact(context, artifact).map_err(|error| error.to_string())
        };
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &context.schema_registry,
        conversion_registry: &context.converter_registry,
        package_artifact_reader: Some(&package_artifact_reader),
        artifact_cache: None,
    };
    execute_conversion_output_pipeline_with_environment(
        &environment,
        pipeline,
        rendered_value,
        rendered_source_map,
        rendered_output_spans,
        converter_id,
        diagnostic_node,
        diagnostic_uri,
    )
}

fn execute_conversion_output_pipeline_from_formatted_cem_tree_with_context(
    context: &EngineContext,
    pipeline: &ConversionOutputPipeline,
    formatted_value: Value,
    formatted_source_map: Option<SourceMapStack>,
    formatted_output_spans: Vec<OutputSpan>,
    converter_id: &str,
    diagnostic_node: Option<&str>,
    diagnostic_uri: Option<&str>,
) -> crate::conversion::ConversionOutputPipelineExecution {
    let package_artifact_reader =
        |artifact: &ConversionPackageArtifactDescriptor| -> Result<ConversionPackageArtifactRead, String> {
            read_conversion_package_artifact(context, artifact).map_err(|error| error.to_string())
        };
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &context.schema_registry,
        conversion_registry: &context.converter_registry,
        package_artifact_reader: Some(&package_artifact_reader),
        artifact_cache: None,
    };
    execute_conversion_output_pipeline_from_formatted_cem_tree_with_environment(
        &environment,
        pipeline,
        formatted_value,
        formatted_source_map,
        formatted_output_spans,
        converter_id,
        diagnostic_node,
        diagnostic_uri,
    )
}

fn read_conversion_package_artifact(
    context: &EngineContext,
    artifact: &ConversionPackageArtifactDescriptor,
) -> Result<ConversionPackageArtifactRead, ResolverDiagnostic> {
    let content_type_hint = artifact.content_type.as_deref();
    let uri = artifact.path.as_str();
    if has_uri_scheme(uri) && !is_windows_drive_path(uri) {
        if let Some(path) = parse_local_file_uri(uri).transpose().map_err(|error| {
            ResolverDiagnostic::InvalidFileUri {
                uri: uri.to_owned(),
                message: error.to_string(),
            }
        })? {
            return read_local_template_import(uri, path, content_type_hint)
                .map(conversion_package_artifact_read_from_resolved);
        }
        if let Some(read) = read_registered_resource(
            Some(context),
            uri,
            ResolvePurpose::Template,
            content_type_hint,
        )? {
            return Ok(conversion_package_artifact_read_from_resolved(read));
        }
        return Err(ResolverDiagnostic::UnsupportedResolver {
            uri: uri.to_owned(),
            purpose: ResolvePurpose::Template,
            direction: ResolveDirection::Read,
        });
    }

    let mut last_error = None;
    for path in converter_template_candidate_paths(uri) {
        match std::fs::read(&path) {
            Ok(bytes) => {
                return Ok(ConversionPackageArtifactRead {
                    uri: path.to_string_lossy().into_owned(),
                    bytes,
                    content_type: content_type_hint.map(str::to_owned),
                });
            }
            Err(error) => {
                last_error = Some((path, error));
            }
        }
    }

    let (path, error) =
        last_error.unwrap_or_else(|| (PathBuf::from(uri), std::io::ErrorKind::NotFound.into()));
    Err(ResolverDiagnostic::Io {
        uri: path.to_string_lossy().into_owned(),
        message: error.to_string(),
    })
}

fn conversion_package_artifact_read_from_resolved(
    read: ResolvedRead,
) -> ConversionPackageArtifactRead {
    ConversionPackageArtifactRead {
        uri: read.uri,
        bytes: read.bytes,
        content_type: read.content_type,
    }
}

fn context_with_loaded_schema_package_manifests(
    context: &EngineContext,
) -> EngineResult<(EngineContext, Vec<Diagnostic>)> {
    if context.schema_package_manifests.is_empty() {
        return Ok((context.clone(), Vec::new()));
    }

    let mut enriched = context.clone();
    enriched.schema_package_manifests.clear();
    let mut diagnostics = Vec::new();
    for package_input in &context.schema_package_manifests {
        diagnostics.extend(load_schema_package_manifest_into_context(
            &mut enriched,
            package_input,
        )?);
    }
    Ok((enriched, diagnostics))
}

fn load_schema_package_manifest_into_context(
    context: &mut EngineContext,
    input: &EngineInput,
) -> EngineResult<Vec<Diagnostic>> {
    let mut package_input = input.clone();
    if package_input.from_format.is_none() {
        package_input.from_format = Some(InputFormat::Cem);
    }
    if package_input.identity.is_none() {
        package_input.identity = Some(schema_package_manifest_identity());
    }
    if package_input.root_scope.schema.is_none() {
        package_input.root_scope.schema = Some(CEM_SCHEMA_PACKAGE_URI.to_owned());
    }
    if package_input.root_scope.default_content_type.is_none() {
        package_input.root_scope.default_content_type =
            Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned());
    }

    let package_input = materialized_input(&package_input, context)?;
    let loaded = load_input_through_lifecycle(&package_input, context);
    let manifest_uri = input_uri(&package_input, context);
    let run = run_pipeline_as_scoped_with_context_and_source_uri(
        &loaded.bytes,
        loaded.from_format,
        &package_input.root_scope,
        context,
        &manifest_uri,
    );

    let mut diagnostics = loaded.diagnostics;
    diagnostics.extend(run.diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        project_diagnostics_for_source(&mut diagnostics, &loaded.bytes);
        project_diagnostic_uris(&mut diagnostics, &package_input, context);
        return Ok(diagnostics);
    }

    match std::str::from_utf8(&loaded.bytes) {
        Ok(manifest_source) => {
            let mut registration_diagnostics =
                register_validated_schema_package_manifest(context, manifest_source, &manifest_uri);
            diagnostics.append(&mut registration_diagnostics);
        }
        Err(error) => diagnostics.push(schema_package_load_diagnostic(
            Some(&manifest_uri),
            "cem.schema_package.manifest_source_invalid",
            format!("schema package manifest `{manifest_uri}` is not UTF-8 CEM-ML: {error}"),
        )),
    }

    project_diagnostics_for_source(&mut diagnostics, &loaded.bytes);
    project_diagnostic_uris(&mut diagnostics, &package_input, context);
    Ok(diagnostics)
}

fn register_validated_schema_package_manifest(
    context: &mut EngineContext,
    manifest_source: &str,
    manifest_uri: &str,
) -> Vec<Diagnostic> {
    let base_path = schema_package_manifest_base_path(manifest_uri);
    let package_id_hint = "external-schema-package";
    let mut diagnostics = Vec::new();

    register_schema_document_model_from_validated_schema_package_manifest(
        context,
        manifest_source,
        manifest_uri,
        package_id_hint,
        &mut diagnostics,
    );

    match conversion_descriptors_from_validated_schema_package_manifest(
        package_id_hint,
        manifest_source,
        &base_path,
    ) {
        Ok(descriptors) => {
            for descriptor in descriptors {
                if let Err(error) = context.converter_registry.register(descriptor) {
                    diagnostics.push(schema_package_load_diagnostic(
                        Some(manifest_uri),
                        "cem.schema_package.converter_registration_failed",
                        format!("schema package converter could not be registered: {error}"),
                    ));
                }
            }
        }
        Err(error) => diagnostics.push(schema_package_manifest_error_diagnostic(
            manifest_uri,
            "converter metadata",
            error,
        )),
    }

    match conversion_package_artifacts_from_validated_schema_package_manifest(
        package_id_hint,
        manifest_source,
        &base_path,
    ) {
        Ok(artifacts) => {
            for artifact in artifacts {
                context
                    .converter_registry
                    .register_package_artifact(artifact);
            }
        }
        Err(error) => diagnostics.push(schema_package_manifest_error_diagnostic(
            manifest_uri,
            "artifact metadata",
            error,
        )),
    }

    diagnostics
}

fn register_schema_document_model_from_validated_schema_package_manifest(
    context: &mut EngineContext,
    manifest_source: &str,
    manifest_uri: &str,
    package_id_hint: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let schema_source_path = match schema_source_path_from_manifest_source(manifest_source) {
        Ok(Some(schema_source_path)) => schema_source_path,
        Ok(None) => return,
        Err(error) => {
            diagnostics.push(schema_package_manifest_error_diagnostic(
                manifest_uri,
                "schema metadata",
                error,
            ));
            return;
        }
    };
    let read = match read_schema_package_schema_source(context, manifest_uri, &schema_source_path) {
        Ok(read) => read,
        Err(error) => {
            diagnostics.push(schema_package_load_diagnostic(
                Some(manifest_uri),
                "cem.schema_package.schema_source_unreadable",
                format!(
                    "schema package manifest `{manifest_uri}` references unreadable schema source `{schema_source_path}`: {error}"
                ),
            ));
            return;
        }
    };
    let schema_source = match String::from_utf8(read.bytes) {
        Ok(schema_source) => schema_source,
        Err(error) => {
            diagnostics.push(schema_package_load_diagnostic(
                Some(&read.uri),
                "cem.schema_package.schema_source_invalid",
                format!(
                    "schema package manifest `{manifest_uri}` references non-UTF-8 schema source `{}`: {error}",
                    read.uri
                ),
            ));
            return;
        }
    };
    let descriptor = match schema_descriptor_from_manifest_and_schema_sources(
        package_id_hint,
        manifest_source,
        &read.uri,
        &schema_source,
    ) {
        Ok(descriptor) => descriptor,
        Err(error) => {
            diagnostics.push(schema_package_manifest_error_diagnostic(
                manifest_uri,
                "schema metadata",
                error,
            ));
            return;
        }
    };
    let model = compile_schema_document_model(&descriptor.schema_uri, &schema_source);
    if !model.compile_diagnostics.is_empty() {
        diagnostics.extend(model.compile_diagnostics.into_iter().map(|mut diagnostic| {
            if diagnostic.uri.is_none() {
                diagnostic.uri = Some(read.uri.clone());
            }
            diagnostic
        }));
        return;
    }
    if context
        .schema_registry
        .schema(&descriptor.schema_uri)
        .is_some_and(|existing| {
            schema_descriptor_matches_registered_identity(existing, &descriptor)
        })
    {
        return;
    }
    if let Err(error) = context.schema_registry.register(descriptor) {
        diagnostics.push(schema_package_load_diagnostic(
            Some(manifest_uri),
            "cem.schema_package.schema_registration_failed",
            format!("schema package schema could not be registered: {error}"),
        ));
        return;
    }
    if !model.is_empty() {
        context.schema_document_models.register(model);
    }
}

fn schema_descriptor_matches_registered_identity(
    existing: &SchemaDescriptor,
    candidate: &SchemaDescriptor,
) -> bool {
    existing.package_id == candidate.package_id
        && existing.schema_uri == candidate.schema_uri
        && existing.version == candidate.version
        && existing.content_types == candidate.content_types
        && existing.namespaces == candidate.namespaces
        && existing.uses == candidate.uses
}

fn read_schema_package_schema_source(
    context: &EngineContext,
    manifest_uri: &str,
    schema_source_path: &str,
) -> Result<ResolvedRead, ResolverDiagnostic> {
    let manifest_template = TemplateInput {
        uri: manifest_uri.to_owned(),
        bytes: Vec::new(),
        identity: Some(schema_package_manifest_identity()),
        root_scope: ScopeConfig::default(),
    };
    read_template_import_source(
        context,
        &manifest_template,
        schema_source_path,
        Some(CEM_SCHEMA_CONTENT_TYPE),
    )
}

fn schema_package_manifest_identity() -> FormatIdentity {
    FormatIdentity {
        content_type: Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned()),
        schema: Some(CEM_SCHEMA_PACKAGE_URI.to_owned()),
        ..FormatIdentity::default()
    }
}

fn schema_package_manifest_base_path(manifest_uri: &str) -> String {
    let manifest_uri = manifest_uri.trim();
    manifest_uri
        .rsplit_once('/')
        .map(|(base, _)| {
            if base.is_empty() {
                ".".to_owned()
            } else {
                base.to_owned()
            }
        })
        .unwrap_or_else(|| ".".to_owned())
}

fn schema_package_manifest_error_diagnostic(
    uri: &str,
    subject: &str,
    error: impl std::fmt::Display,
) -> Diagnostic {
    schema_package_load_diagnostic(
        Some(uri),
        "cem.schema_package.manifest_conversion_metadata_invalid",
        format!("schema package manifest `{uri}` has invalid {subject}: {error}"),
    )
}

fn schema_package_load_diagnostic(
    uri: Option<&str>,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        uri: uri.map(str::to_owned),
        code: code.to_owned(),
        severity: Severity::Error,
        message: message.into(),
        details: None,
        ..Diagnostic::default()
    }
}

fn converter_template_candidate_paths(uri: &str) -> Vec<PathBuf> {
    let direct = PathBuf::from(uri);
    let mut candidates = Vec::new();
    push_converter_template_candidate(&mut candidates, direct.clone());
    if direct.is_absolute() {
        return candidates;
    }

    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(package_relative_uri) = uri.strip_prefix("packages/cem_ml/") {
        push_converter_template_candidate(&mut candidates, crate_root.join(package_relative_uri));
    } else {
        push_converter_template_candidate(
            &mut candidates,
            PathBuf::from("packages/cem_ml").join(uri),
        );
        push_converter_template_candidate(&mut candidates, crate_root.join(uri));
    }
    candidates
}

fn push_converter_template_candidate(candidates: &mut Vec<PathBuf>, path: PathBuf) {
    if !candidates.iter().any(|candidate| candidate == &path) {
        candidates.push(path);
    }
}

fn convert_primary_from_template_output(
    output: TransformTemplateOutputArtifact,
    to_format: LayerFormat,
) -> Option<Value> {
    let kind = match to_format {
        LayerFormat::Html => "html",
        LayerFormat::Xml => "xml",
        _ => return None,
    };
    let source_map = output
        .source_map
        .and_then(|source_map| serde_json::to_value(source_map).ok())
        .unwrap_or(Value::Null);
    let output_spans = serde_json::to_value(output.output_spans).unwrap_or_else(|_| json!([]));

    match output.value {
        Value::String(content) => Some(json!({
            "kind": kind,
            "content": content,
            "sourceMap": source_map,
            "outputSpans": output_spans,
        })),
        Value::Object(mut object) => {
            object.get("content").and_then(Value::as_str)?;
            if let Some(existing_kind) = object.get("kind").and_then(Value::as_str) {
                if existing_kind != kind {
                    return None;
                }
            } else {
                object.insert("kind".to_owned(), Value::String(kind.to_owned()));
            }
            object.entry("sourceMap".to_owned()).or_insert(source_map);
            object
                .entry("outputSpans".to_owned())
                .or_insert(output_spans);
            Some(Value::Object(object))
        }
        _ => None,
    }
}

fn convert_primary_to_cem_with_cemt_pipeline(
    context: Option<&EngineContext>,
    document: &CemDocument,
    from_format: InputFormat,
    target_scope: &ScopeConfig,
    diagnostic_uri: Option<&str>,
) -> Result<Value, Vec<Diagnostic>> {
    let pipeline = cemt_output_pipeline_for_scope(direct_cem_output_pipeline(), target_scope);
    let pipeline_execution = execute_conversion_output_pipeline_with_context(
        context,
        &pipeline,
        projection::cem_tree_nodes_json_with_source_content_type(
            document,
            Some(input_format_content_type(from_format)),
        ),
        None,
        Vec::new(),
        "direct-cem-output",
        Some("output"),
        diagnostic_uri,
    );
    let mut diagnostics = pipeline_execution.diagnostics;
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(diagnostics);
    }
    let Some(output) = pipeline_execution.output else {
        diagnostics.push(direct_cem_output_pipeline_diagnostic(
            diagnostic_uri,
            "CEMT output pipeline produced no writer artifact",
        ));
        return Err(diagnostics);
    };
    let Some(output) = output.as_str() else {
        diagnostics.push(direct_cem_output_pipeline_diagnostic(
            diagnostic_uri,
            "CEMT output pipeline writer artifact was not text",
        ));
        return Err(diagnostics);
    };
    let mut content = output.to_owned();
    if !content.is_empty() && !content.ends_with('\n') {
        content.push_str(&transform_template_encode_options_line_ending_data(
            &pipeline.cemt_options,
        ));
    }
    let source_map = pipeline_execution
        .source_map
        .and_then(|source_map| serde_json::to_value(source_map).ok())
        .unwrap_or(Value::Null);
    let output_spans =
        serde_json::to_value(pipeline_execution.output_spans).unwrap_or_else(|_| json!([]));
    Ok(json!({
        "kind": "cem",
        "content": content,
        "sourceMap": source_map,
        "outputSpans": output_spans,
    }))
}

fn convert_primary_to_markup_with_cemt_pipeline(
    context: Option<&EngineContext>,
    document: &CemDocument,
    from_format: InputFormat,
    pipeline: ConversionOutputPipeline,
    kind: &str,
    converter_id: &str,
    target_scope: &ScopeConfig,
    diagnostic_uri: Option<&str>,
) -> Result<Value, Vec<Diagnostic>> {
    let pipeline = cemt_output_pipeline_for_scope(pipeline, target_scope);
    let pipeline_execution = execute_conversion_output_pipeline_with_context(
        context,
        &pipeline,
        cem_tree_nodes_for_markup_output(document, from_format),
        None,
        Vec::new(),
        converter_id,
        Some("output"),
        diagnostic_uri,
    );
    let mut diagnostics = pipeline_execution.diagnostics;
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(diagnostics);
    }
    let Some(output) = pipeline_execution.output else {
        diagnostics.push(direct_markup_output_pipeline_diagnostic(
            diagnostic_uri,
            kind,
            "CEMT output pipeline produced no writer artifact",
        ));
        return Err(diagnostics);
    };
    let Some(output) = output.as_str() else {
        diagnostics.push(direct_markup_output_pipeline_diagnostic(
            diagnostic_uri,
            kind,
            "CEMT output pipeline writer artifact was not text",
        ));
        return Err(diagnostics);
    };
    let source_map = pipeline_execution
        .source_map
        .and_then(|source_map| serde_json::to_value(source_map).ok())
        .unwrap_or(Value::Null);
    let output_spans =
        serde_json::to_value(pipeline_execution.output_spans).unwrap_or_else(|_| json!([]));
    Ok(json!({
        "kind": kind,
        "content": output,
        "sourceMap": source_map,
        "outputSpans": output_spans,
    }))
}

fn cem_tree_nodes_for_markup_output(document: &CemDocument, from_format: InputFormat) -> Value {
    let mut nodes = projection::cem_tree_nodes_json_with_source_content_type(
        document,
        Some(input_format_content_type(from_format)),
    );
    remove_cem_directive_nodes(&mut nodes);
    nodes
}

fn remove_cem_directive_nodes(value: &mut Value) {
    match value {
        Value::Array(items) => {
            items.retain(|item| !is_cem_directive_node(item));
            for item in items {
                remove_cem_directive_nodes(item);
            }
        }
        Value::Object(fields) => {
            for key in ["children", "nodes", "slots"] {
                if let Some(children) = fields.get_mut(key) {
                    remove_cem_directive_nodes(children);
                }
            }
        }
        _ => {}
    }
}

fn is_cem_directive_node(value: &Value) -> bool {
    let Value::Object(fields) = value else {
        return false;
    };
    fields
        .get("kind")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind.trim() == "directive")
        || fields
            .get("name")
            .or_else(|| fields.get("localName"))
            .or_else(|| fields.get("tag"))
            .or_else(|| fields.get("tagName"))
            .and_then(Value::as_str)
            .map(str::trim)
            .is_some_and(|name| name.starts_with('@'))
}

fn direct_cem_output_pipeline_diagnostic(uri: Option<&str>, message: &str) -> Diagnostic {
    Diagnostic {
        uri: uri.map(str::to_owned),
        code: CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE.to_owned(),
        severity: Severity::Error,
        message: format!("direct CEM output could not execute CEMT output pipeline: {message}"),
        node: Some("output".to_owned()),
        details: None,
        ..Diagnostic::default()
    }
}

fn direct_markup_output_pipeline_diagnostic(
    uri: Option<&str>,
    kind: &str,
    message: &str,
) -> Diagnostic {
    Diagnostic {
        uri: uri.map(str::to_owned),
        code: CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE.to_owned(),
        severity: Severity::Error,
        message: format!("direct {kind} output could not execute CEMT output pipeline: {message}"),
        node: Some("output".to_owned()),
        details: None,
        ..Diagnostic::default()
    }
}

fn input_format_content_type(from_format: InputFormat) -> &'static str {
    match from_format {
        InputFormat::Cem => "application/cem",
        InputFormat::Html => "text/html",
        InputFormat::Xml => "application/xml",
    }
}

fn fallback_or_publish_converter_diagnostics(
    conversion: &ExportConversionExecution,
    reason: &str,
    mut local_diagnostics: Vec<Diagnostic>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ExportConversionTemplateResult {
    if let Some(fallback) = conversion.rust_fallback.as_ref() {
        diagnostics.push(converter_template_diagnostic(
            None,
            "cem.converter.cemt_fallback",
            Severity::Warning,
            format!(
                "converter `{}` used Rust fallback `{}` because {reason}",
                conversion.converter_id, fallback.rust_symbol
            ),
        ));
        return ExportConversionTemplateResult::UseRustFallback;
    }

    diagnostics.append(&mut local_diagnostics);
    ExportConversionTemplateResult::Failed
}

fn publish_converter_cemt_diagnostics(
    conversion: &ExportConversionExecution,
    reason: &str,
    mut local_diagnostics: Vec<Diagnostic>,
    diagnostics: &mut Vec<Diagnostic>,
) -> ExportConversionTemplateResult {
    if !local_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        local_diagnostics.push(converter_template_diagnostic(
            None,
            CONVERSION_OUTPUT_PIPELINE_EXECUTION_CODE,
            Severity::Error,
            format!(
                "converter `{}` could not complete CEMT conversion: {reason}",
                conversion.converter_id
            ),
        ));
    }
    diagnostics.append(&mut local_diagnostics);
    ExportConversionTemplateResult::Failed
}

fn converter_template_diagnostic(
    uri: Option<&str>,
    code: &str,
    severity: Severity,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        uri: uri.map(str::to_owned),
        code: code.to_owned(),
        severity,
        message: message.into(),
        details: None,
        ..Diagnostic::default()
    }
}

/// Aggregate every layer's diagnostics for an input through the
/// pipeline. Used by every parser-backed request, and by the public
/// observability entry points [`observe_pipeline`] and
/// [`observe_pipeline_scoped`].
pub struct PipelineRun {
    pub document: CemDocument,
    pub diagnostics: Vec<Diagnostic>,
}

fn run_pipeline_as(bytes: &[u8], from_format: InputFormat) -> PipelineRun {
    run_pipeline_as_with_context(bytes, from_format, None, None, None)
}

fn run_pipeline_as_scoped(
    bytes: &[u8],
    from_format: InputFormat,
    root_scope: &ScopeConfig,
) -> PipelineRun {
    run_pipeline_as_with_context(bytes, from_format, Some(root_scope), None, None)
}

fn run_pipeline_as_scoped_with_context_and_source_uri(
    bytes: &[u8],
    from_format: InputFormat,
    root_scope: &ScopeConfig,
    context: &EngineContext,
    source_uri: &str,
) -> PipelineRun {
    run_pipeline_as_with_context(
        bytes,
        from_format,
        Some(root_scope),
        Some(context),
        Some(source_uri),
    )
}

fn run_pipeline_as_with_context(
    bytes: &[u8],
    from_format: InputFormat,
    root_scope: Option<&ScopeConfig>,
    context: Option<&EngineContext>,
    source_uri: Option<&str>,
) -> PipelineRun {
    match from_format {
        InputFormat::Cem => {
            run_pipeline_with::<CemTokenizer>(bytes, root_scope, context, source_uri)
        }
        InputFormat::Html => {
            run_pipeline_with::<HtmlTokenizer>(bytes, root_scope, context, source_uri)
        }
        InputFormat::Xml => {
            run_pipeline_with::<XmlTokenizer>(bytes, root_scope, context, source_uri)
        }
    }
}

fn run_pipeline_with<T>(
    bytes: &[u8],
    root_scope: Option<&ScopeConfig>,
    context: Option<&EngineContext>,
    source_uri: Option<&str>,
) -> PipelineRun
where
    T: SchemaTokenizer + FromBytes,
{
    let started_at = Instant::now();
    let module_map = root_scope
        .map(|scope| load_root_module_map(scope, context))
        .unwrap_or_default();
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
            machine = machine.with_root_module_map_entries(
                module_map
                    .uri
                    .as_deref()
                    .or(root_scope.module_map.as_deref()),
                &module_map.entries,
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
        if let Some(root_scope) = root_scope {
            apply_root_scope_version_pins(&mut doc, root_scope);
        }
        doc
    };
    document.diagnostics.extend(schema_outcome.diagnostics);
    document.diagnostics.extend(module_map.diagnostics);

    // Validation rule registry.
    let registry = RuleRegistry::with_tier_a_rules();
    let schema_uri = root_scope
        .and_then(|scope| scope.schema.as_deref())
        .or_else(|| context.and_then(|context| context.schema.as_deref()));
    let content_type = root_scope
        .and_then(|scope| scope.default_content_type.as_deref())
        .or_else(|| context.and_then(|context| context.content_type.as_deref()));
    let rule_resource_reader = |uri: &str,
                                base_uri: Option<&str>,
                                content_type_hint: Option<&str>|
     -> Result<RuleResourceRead, String> {
        let context = context
            .ok_or_else(|| format!("no engine context is available to resolve resource `{uri}`"))?;
        let read = if let Some(base_uri) = base_uri {
            let template = TemplateInput {
                uri: base_uri.to_owned(),
                bytes: Vec::new(),
                identity: None,
                root_scope: ScopeConfig::default(),
            };
            read_template_import_source(context, &template, uri, content_type_hint)
                .map_err(|error| error.to_string())
        } else if has_uri_scheme(uri) && !is_windows_drive_path(uri) {
            read_registered_template_import(context, uri, None, content_type_hint)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| {
                    ResolverDiagnostic::UnsupportedResolver {
                        uri: uri.to_owned(),
                        purpose: ResolvePurpose::Template,
                        direction: ResolveDirection::Read,
                    }
                    .to_string()
                })
        } else {
            read_local_template_import(uri, PathBuf::from(uri), content_type_hint)
                .map_err(|error| error.to_string())
        }?;
        Ok(RuleResourceRead {
            uri: read.uri,
            bytes: read.bytes,
            content_type: read.content_type,
        })
    };
    let resource_reader = context.map(|_| &rule_resource_reader as &RuleResourceReader<'_>);
    let rule_diags = registry.run(&RuleContext {
        document: &document,
        schema_uri,
        content_type,
        source_uri,
        resource_reader,
        schema_registry: context.map(|context| &context.schema_registry),
        schema_document_models: context.map(|context| &context.schema_document_models),
        upstream_diagnostics: &document.diagnostics,
        schema_behavior_evaluator: context
            .and_then(|context| context.schema_behavior_evaluator.as_deref())
            .map(|evaluator| {
                evaluator as &dyn crate::schema::document_model::SchemaBehaviorEvaluator
            }),
    });

    let mut diagnostics = document.diagnostics.clone();
    diagnostics.extend(rule_diags);
    if let Some(root_scope) = root_scope {
        diagnostics.extend(parse_budget_diagnostics(
            root_scope,
            started_at.elapsed().as_nanos(),
        ));
    }
    project_diagnostics_for_source(&mut diagnostics, bytes);
    PipelineRun {
        document,
        diagnostics,
    }
}

fn apply_root_scope_version_pins(document: &mut CemDocument, scope: &ScopeConfig) {
    for (target, constraint) in &scope.version_pins {
        let target = target.trim();
        let constraint = constraint.trim();
        if !is_cem_ml_version_pin_target(target) {
            document.diagnostics.push(Diagnostic {
                code: "cem.scope.version_pin_target_unsupported".to_owned(),
                severity: Severity::Warning,
                message: format!(
                    "root-scope version pin target `{target}` is not supported by this engine; \
                     supported targets are `{}`, `{}`, and `application/cem+xml`",
                    format::SUPPORTED_FORMAT_ID,
                    format::SUPPORTED_CONTENT_TYPE
                ),
                details: None,
                ..Diagnostic::default()
            });
            continue;
        }

        match format::resolve_doc_directive(&format!(
            "{} {constraint}",
            format::SUPPORTED_FORMAT_ID
        )) {
            Ok(identity) => {
                let message = format!(
                    "resolved root-scope version pin {} {} -> embedded {}",
                    identity.format_id, identity.content_type, identity.format_version
                );
                document.format_identity = Some(identity);
                document.diagnostics.push(Diagnostic {
                    code: format::VERSION_RESOLVED_CODE.to_owned(),
                    severity: Severity::Info,
                    message,
                    ..Diagnostic::default()
                });
            }
            Err(err) => {
                document.diagnostics.push(Diagnostic {
                    code: err.code().to_owned(),
                    severity: Severity::Error,
                    message: format!(
                        "root-scope version pin `{target}:{constraint}` is invalid: {}",
                        err.message()
                    ),
                    details: None,
                    ..Diagnostic::default()
                });
            }
        }
    }
}

fn is_cem_ml_version_pin_target(target: &str) -> bool {
    target == format::SUPPORTED_FORMAT_ID
        || target == format::SUPPORTED_CONTENT_TYPE
        || target == "application/cem+xml"
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
        details: None,
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
    diagnostics.extend(root_scope_budget_diagnostics(uri, scope, direction));
    diagnostics
}

fn root_scope_metadata_diagnostics(
    uri: &str,
    scope: &ScopeConfig,
    direction: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if direction == "output" && scope.module_map.is_some() {
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
        details: None,
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

#[derive(Debug, Default)]
struct LoadedModuleMap {
    entries: BTreeMap<String, String>,
    diagnostics: Vec<Diagnostic>,
    uri: Option<String>,
}

fn load_root_module_map(scope: &ScopeConfig, context: Option<&EngineContext>) -> LoadedModuleMap {
    let Some(module_map) = scope.module_map.as_deref().map(str::trim) else {
        return LoadedModuleMap::default();
    };
    if module_map.is_empty() {
        return LoadedModuleMap::default();
    }

    let (bytes, resolved_uri) = match parse_local_file_uri(module_map) {
        Some(Ok(path)) => match std::fs::read(&path) {
            Ok(bytes) => (bytes, module_map.to_owned()),
            Err(error) => {
                return unreadable_module_map(module_map, error);
            }
        },
        Some(Err(error)) => {
            match read_registered_resource(
                context,
                module_map,
                ResolvePurpose::ModuleMap,
                Some("application/json"),
            ) {
                Ok(Some(read)) => (read.bytes, read.uri),
                Ok(None) => return unreadable_module_map(module_map, error),
                Err(error) => return resolver_module_map_error(module_map, error),
            }
        }
        None if has_uri_scheme(module_map) && !is_windows_drive_path(module_map) => {
            match read_registered_resource(
                context,
                module_map,
                ResolvePurpose::ModuleMap,
                Some("application/json"),
            ) {
                Ok(Some(read)) => (read.bytes, read.uri),
                Ok(None) => return unsupported_module_map_resolver(module_map),
                Err(error) => return resolver_module_map_error(module_map, error),
            }
        }
        None => match std::fs::read(module_map) {
            Ok(bytes) => (bytes, module_map.to_owned()),
            Err(error) => {
                return unreadable_module_map(module_map, error);
            }
        },
    };

    let value = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => value,
        Err(error) => {
            return LoadedModuleMap {
                entries: BTreeMap::new(),
                diagnostics: vec![Diagnostic {
                    code: "cem.scope.module_map_invalid".to_owned(),
                    severity: Severity::Warning,
                    message: format!(
                        "root-scope moduleMap `{module_map}` is not valid JSON: {error}"
                    ),
                    details: None,
                    ..Diagnostic::default()
                }],
                uri: Some(resolved_uri),
            };
        }
    };
    match module_map_aliases(&value) {
        Ok(entries) => LoadedModuleMap {
            entries,
            diagnostics: Vec::new(),
            uri: Some(resolved_uri),
        },
        Err(message) => LoadedModuleMap {
            entries: BTreeMap::new(),
            diagnostics: vec![Diagnostic {
                code: "cem.scope.module_map_invalid".to_owned(),
                severity: Severity::Warning,
                message: format!("root-scope moduleMap `{module_map}` is invalid: {message}"),
                details: None,
                ..Diagnostic::default()
            }],
            uri: Some(resolved_uri),
        },
    }
}

fn unreadable_module_map(module_map: &str, error: impl std::fmt::Display) -> LoadedModuleMap {
    LoadedModuleMap {
        entries: BTreeMap::new(),
        diagnostics: vec![Diagnostic {
            code: "cem.scope.module_map_unreadable".to_owned(),
            severity: Severity::Warning,
            message: format!("root-scope moduleMap `{module_map}` could not be read: {error}"),
            details: None,
            ..Diagnostic::default()
        }],
        uri: Some(module_map.to_owned()),
    }
}

fn unsupported_module_map_resolver(module_map: &str) -> LoadedModuleMap {
    LoadedModuleMap {
        entries: BTreeMap::new(),
        diagnostics: vec![Diagnostic {
            code: "cem.scope.module_map_resolver_unsupported".to_owned(),
            severity: Severity::Warning,
            message: format!(
                "root-scope moduleMap `{module_map}` uses a remote/custom URI resolver, \
                 but only local paths and local file:// URIs are supported"
            ),
            details: None,
            ..Diagnostic::default()
        }],
        uri: Some(module_map.to_owned()),
    }
}

fn resolver_module_map_error(module_map: &str, error: ResolverDiagnostic) -> LoadedModuleMap {
    match error {
        ResolverDiagnostic::UnsupportedResolver { .. } => {
            unsupported_module_map_resolver(module_map)
        }
        other => unreadable_module_map(module_map, other),
    }
}

fn module_map_aliases(value: &Value) -> Result<BTreeMap<String, String>, String> {
    let Some(object) = value.as_object() else {
        return Err("expected a JSON object".to_owned());
    };
    let mut aliases = BTreeMap::new();
    collect_module_map_aliases(object, &mut aliases)?;
    Ok(aliases)
}

fn collect_module_map_aliases(
    object: &serde_json::Map<String, Value>,
    aliases: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    for (key, value) in object {
        match key.as_str() {
            "imports" | "schemas" | "modules" => {
                let Some(nested) = value.as_object() else {
                    return Err(format!("`{key}` must be a JSON object"));
                };
                collect_module_map_aliases(nested, aliases)?;
            }
            _ => {
                if let Some(target) = module_map_entry_target(key, value)? {
                    aliases.insert(key.clone(), target);
                }
            }
        }
    }
    Ok(())
}

fn module_map_entry_target(key: &str, value: &Value) -> Result<Option<String>, String> {
    if let Some(target) = value.as_str() {
        return Ok(Some(target.to_owned()));
    }
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    for field in ["uri", "src", "path"] {
        if let Some(target) = object.get(field) {
            let Some(target) = target.as_str() else {
                return Err(format!("moduleMap entry `{key}.{field}` must be a string"));
            };
            return Ok(Some(target.to_owned()));
        }
    }
    Ok(None)
}

fn scope_time_budget<'a>(
    scope: &'a ScopeConfig,
    aliases: &[&str],
) -> Option<(&'a str, Result<u64, String>)> {
    scope
        .budgets
        .iter()
        .find(|(field, _)| {
            let normalized = normalize_scope_key(field);
            aliases.iter().any(|alias| *alias == normalized)
        })
        .map(|(field, value)| (field.as_str(), parse_u64_budget(field, value)))
}

fn parse_budget_diagnostics(scope: &ScopeConfig, elapsed_ns: u128) -> Vec<Diagnostic> {
    time_budget_diagnostics(scope, &["parsems", "parsetimebudgetms"], elapsed_ns)
}

fn time_budget_diagnostics(
    scope: &ScopeConfig,
    aliases: &[&str],
    elapsed_ns: u128,
) -> Vec<Diagnostic> {
    let Some((field, Ok(budget_ms))) = scope_time_budget(scope, aliases) else {
        return Vec::new();
    };
    let budget_ns = (budget_ms as u128) * 1_000_000;
    if elapsed_ns <= budget_ns {
        return Vec::new();
    }
    vec![Diagnostic {
        code: "cem.scope.budget_exceeded".to_owned(),
        severity: Severity::Error,
        message: format!(
            "root-scope budget `{field}` exceeded: elapsed {elapsed_ns}ns > budget {budget_ns}ns"
        ),
        details: None,
        ..Diagnostic::default()
    }]
}

fn root_scope_budget_diagnostics(
    uri: &str,
    scope: &ScopeConfig,
    direction: &str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (field, value) in &scope.budgets {
        match normalize_scope_key(field).as_str() {
            "cpu" | "cpuworkers" | "queue" | "queuesize" | "io" | "iostreams" => {
                if let Err(message) = parse_u32_budget(field, value) {
                    diagnostics.push(scope_policy_diagnostic(
                        uri,
                        "cem.scope.budget_invalid",
                        message,
                        direction,
                    ));
                }
            }
            "memory" | "memorybytes" | "pluginms" | "plugintimebudgetms" | "parsems"
            | "parsetimebudgetms" | "validatems" | "validatetimebudgetms" | "checkms"
            | "checktimebudgetms" | "convertms" | "converttimebudgetms" | "tracems"
            | "tracetimebudgetms" | "inspectms" | "inspecttimebudgetms" | "benchms"
            | "benchtimebudgetms" | "fixturevalidatems" | "fixturevalidatetimebudgetms"
            | "fixtureroundtripms" | "fixtureroundtriptimebudgetms" | "observems"
            | "observetimebudgetms" => {
                if let Err(message) = parse_u64_budget(field, value) {
                    diagnostics.push(scope_policy_diagnostic(
                        uri,
                        "cem.scope.budget_invalid",
                        message,
                        direction,
                    ));
                }
            }
            "overflow" => match normalize_scope_key(value).as_str() {
                "block" | "reject" | "spilltoparent" => {}
                _ => diagnostics.push(scope_policy_diagnostic(
                    uri,
                    "cem.scope.budget_invalid",
                    format!(
                        "budget `overflow` expects block, reject, or spill-to-parent, got `{value}`"
                    ),
                    direction,
                )),
            },
            _ => diagnostics.push(scope_policy_diagnostic(
                uri,
                "cem.scope.budget_unenforced",
                format!(
                    "budget `{field}` is parsed and preserved, but runtime enforcement is not implemented yet"
                ),
                direction,
            )),
        }
    }
    diagnostics
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
            "parsems" | "parsetimebudgetms" | "validatems" | "validatetimebudgetms"
            | "checkms" | "checktimebudgetms" | "convertms" | "converttimebudgetms"
            | "tracems" | "tracetimebudgetms" | "inspectms" | "inspecttimebudgetms"
            | "benchms" | "benchtimebudgetms" | "fixturevalidatems"
            | "fixturevalidatetimebudgetms" | "fixtureroundtripms"
            | "fixtureroundtriptimebudgetms" | "observems" | "observetimebudgetms" => {
                if let Err(message) = parse_u64_budget(field, value) {
                    diagnostics.push(scope_policy_diagnostic(
                        uri,
                        "cem.scope.budget_invalid",
                        message,
                        direction,
                    ));
                }
            }
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

fn scheduler_policy_for_transform_scope(
    context: &EngineContext,
    uri: &str,
    scope: &ScopeConfig,
    direction: &str,
) -> (crate::scheduler::ScopePolicy, Vec<Diagnostic>) {
    scheduler_policy_for_scope(context, uri, scope, direction)
}

fn has_hard_transform_diagnostic(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
}

fn load_transform_data_artifact(
    input: &EngineInput,
    context: &EngineContext,
    artifact_id: impl Into<String>,
) -> (TransformTemplateDataArtifact, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut scope_diagnostics =
        root_scope_metadata_diagnostics(&input.uri, &input.root_scope, "input");
    diagnostics.append(&mut scope_diagnostics);
    let mut loaded = load_input_through_lifecycle(input, context);
    diagnostics.append(&mut loaded.diagnostics);
    let run = run_pipeline_as_scoped_with_context_and_source_uri(
        &loaded.bytes,
        loaded.from_format,
        &input.root_scope,
        context,
        &input_uri(input, context),
    );
    let value = projection::dom_json(&run.document);
    diagnostics.extend(run.diagnostics);
    project_diagnostics_for_source(&mut diagnostics, &loaded.bytes);
    project_diagnostic_uris(&mut diagnostics, input, context);

    (
        TransformTemplateDataArtifact {
            artifact_id: artifact_id.into(),
            uri: Some(input_uri(input, context)),
            identity: input.identity.clone(),
            value,
        },
        diagnostics,
    )
}

fn collect_transform_graph_join(
    join: &TransformGraphJoin,
    artifacts: &BTreeMap<String, TransformTemplateDataArtifact>,
    artifact_metadata: &BTreeMap<String, TransformOutputMetadata>,
) -> TransformTemplateDataArtifact {
    let mode = match join.mode {
        TransformGraphJoinMode::Collect => "collect",
        TransformGraphJoinMode::GroupBy => "group-by",
        TransformGraphJoinMode::MatchBy => "match-by",
        TransformGraphJoinMode::Zip => "zip",
    };
    match join.mode {
        TransformGraphJoinMode::Collect
        | TransformGraphJoinMode::GroupBy
        | TransformGraphJoinMode::MatchBy
        | TransformGraphJoinMode::Zip => {
            let mut by_input = join
                .input_names
                .iter()
                .map(|name| (name.clone(), Vec::new()))
                .collect::<BTreeMap<_, _>>();
            let items = join
                .inputs
                .iter()
                .filter_map(|input| {
                    artifacts.get(&input.artifact_id).map(|artifact| {
                        let metadata = artifact_metadata.get(&input.artifact_id);
                        let source_map = metadata
                            .and_then(|metadata| metadata.source_map.as_ref())
                            .and_then(|source_map| serde_json::to_value(source_map).ok())
                            .unwrap_or(Value::Null);
                        let output_spans = metadata
                            .map(|metadata| {
                                serde_json::to_value(&metadata.output_spans)
                                    .unwrap_or_else(|_| json!([]))
                            })
                            .unwrap_or_else(|| json!([]));
                        let item = json!({
                            "input": input.input_name.clone(),
                            "artifactId": artifact.artifact_id.clone(),
                            "uri": artifact.uri.clone(),
                            "destination": input.destination.clone(),
                            "identity": input.target.clone().or_else(|| artifact.identity.clone()),
                            "primary": artifact.value.clone(),
                            "bindings": input.bindings.clone(),
                            "sourceMap": source_map,
                            "outputSpans": output_spans,
                        });
                        by_input
                            .entry(input.input_name.clone())
                            .or_default()
                            .push(item.clone());
                        item
                    })
                })
                .collect::<Vec<_>>();
            TransformTemplateDataArtifact {
                artifact_id: join.id.clone(),
                uri: None,
                identity: None,
                value: json!({
                    "kind": "collection",
                    "mode": mode,
                    "count": items.len(),
                    "bindings": join.bindings.clone(),
                    "inputs": by_input,
                    "items": items,
                }),
            }
        }
    }
}

fn collect_transform_graph_join_metadata(
    join: &TransformGraphJoin,
    artifact_metadata: &BTreeMap<String, TransformOutputMetadata>,
) -> TransformOutputMetadata {
    let mut source_maps = Vec::new();
    let mut output_spans = Vec::new();

    for input in &join.inputs {
        let Some(metadata) = artifact_metadata.get(&input.artifact_id) else {
            continue;
        };
        if let Some(source_map) = metadata.source_map.clone() {
            source_maps.push(source_map);
        }
        output_spans.extend(metadata.output_spans.clone());
    }

    TransformOutputMetadata {
        source_map: if source_maps.len() == 1 {
            source_maps.pop()
        } else {
            None
        },
        output_spans,
        raw_content: None,
    }
}

fn transform_graph_artifact_raw_content(
    artifact: &TransformTemplateDataArtifact,
    metadata: Option<&TransformOutputMetadata>,
) -> Option<String> {
    metadata
        .and_then(|metadata| metadata.raw_content.clone())
        .or_else(|| match &artifact.value {
            Value::String(value) => Some(value.clone()),
            Value::Object(fields) => fields
                .get("content")
                .and_then(Value::as_str)
                .map(str::to_owned),
            _ => None,
        })
}

fn transform_graph_export_primary(
    artifact: &TransformTemplateDataArtifact,
    metadata: &TransformOutputMetadata,
    target: Option<&FormatIdentity>,
    style_projection: TransformGraphHtmlStyleProjection,
) -> (Value, Option<SourceMapStack>, Vec<OutputSpan>) {
    if transform_graph_target_is_css(target) {
        if let Some(raw_content) = transform_graph_artifact_raw_content(artifact, Some(metadata)) {
            if let Some(css) =
                extract_inline_style_css_document(&raw_content, &metadata.output_spans)
            {
                let source_map = metadata
                    .source_map
                    .clone()
                    .filter(|_| !css.output_spans.is_empty());
                return (
                    json!({
                        "kind": "document",
                        "content": css.content,
                    }),
                    source_map,
                    css.output_spans,
                );
            }
        }
    }

    if transform_graph_target_is_html(target) {
        if let Some(raw_content) = transform_graph_artifact_raw_content(artifact, Some(metadata)) {
            let html = match style_projection {
                TransformGraphHtmlStyleProjection::Inline => None,
                TransformGraphHtmlStyleProjection::Link(stylesheet_href) => {
                    html_with_extracted_stylesheet_link(
                        &raw_content,
                        &stylesheet_href,
                        target,
                        &metadata.output_spans,
                    )
                }
                TransformGraphHtmlStyleProjection::Omit => {
                    html_without_inline_styles(&raw_content, &metadata.output_spans)
                }
            };
            if let Some(html) = html {
                let source_map = metadata
                    .source_map
                    .clone()
                    .filter(|_| !html.output_spans.is_empty());
                return (
                    json!({
                        "kind": "html",
                        "content": html.content,
                    }),
                    source_map,
                    html.output_spans,
                );
            }
        }
    }

    (
        artifact.value.clone(),
        metadata.source_map.clone(),
        metadata.output_spans.clone(),
    )
}

fn transform_graph_target_is_css(target: Option<&FormatIdentity>) -> bool {
    let Some(target) = target else {
        return false;
    };
    target
        .content_type
        .as_deref()
        .map(content_type_essence)
        .is_some_and(|essence| essence == CSS_CONTENT_TYPE)
        || target.schema.as_deref() == Some(CSS_SCHEMA_URI)
}

fn transform_graph_target_is_html(target: Option<&FormatIdentity>) -> bool {
    let Some(target) = target else {
        return false;
    };
    target
        .content_type
        .as_deref()
        .map(content_type_essence)
        .is_some_and(|essence| matches!(essence.as_str(), HTML_CONTENT_TYPE | XHTML_CONTENT_TYPE))
        || matches!(
            target.schema.as_deref(),
            Some(HTML_SCHEMA_URI) | Some(XHTML_SCHEMA_URI)
        )
}

#[derive(Debug, Clone, PartialEq)]
struct ExtractedInlineCssDocument {
    content: String,
    output_spans: Vec<OutputSpan>,
}

fn extract_inline_style_css_document(
    raw: &str,
    source_output_spans: &[OutputSpan],
) -> Option<ExtractedInlineCssDocument> {
    let blocks = inline_style_blocks(raw)
        .into_iter()
        .filter_map(|block| trimmed_inline_style_content_range(raw, block))
        .collect::<Vec<_>>();

    if blocks.is_empty() {
        None
    } else {
        let mut content = String::new();
        let mut output_spans = Vec::new();
        for (index, (source_start, source_end)) in blocks.into_iter().enumerate() {
            if index > 0 {
                content.push_str("\n\n");
            }
            let output_start = content.len();
            content.push_str(&raw[source_start..source_end]);
            rebase_output_spans(
                source_output_spans,
                source_start,
                source_end,
                output_start,
                &mut output_spans,
            );
        }
        content.push('\n');
        Some(ExtractedInlineCssDocument {
            content,
            output_spans,
        })
    }
}

fn trimmed_inline_style_content_range(
    raw: &str,
    block: InlineStyleBlock,
) -> Option<(usize, usize)> {
    let content = &raw[block.content_start..block.content_end];
    let css = content.trim();
    if css.is_empty() {
        return None;
    }
    let leading = content.find(css).unwrap_or_default();
    let start = block.content_start + leading;
    Some((start, start + css.len()))
}

fn rebase_output_spans(
    source_output_spans: &[OutputSpan],
    source_start: usize,
    source_end: usize,
    output_start: usize,
    rebased: &mut Vec<OutputSpan>,
) {
    for span in source_output_spans {
        let span_start = span.output_range.start as usize;
        let span_end = span.output_range.end() as usize;
        let overlap_start = span_start.max(source_start);
        let overlap_end = span_end.min(source_end);
        if overlap_start >= overlap_end {
            continue;
        }
        rebased.push(OutputSpan {
            output_range: ByteRange::new(
                (output_start + overlap_start - source_start) as u64,
                (overlap_end - overlap_start) as u32,
            ),
            origin: span.origin.clone(),
        });
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ProjectedHtmlDocument {
    content: String,
    output_spans: Vec<OutputSpan>,
}

fn html_with_extracted_stylesheet_link(
    raw: &str,
    href: &str,
    target: Option<&FormatIdentity>,
    source_output_spans: &[OutputSpan],
) -> Option<ProjectedHtmlDocument> {
    project_inline_style_blocks(
        raw,
        Some(stylesheet_link_tag(href, target)),
        source_output_spans,
    )
}

fn html_without_inline_styles(
    raw: &str,
    source_output_spans: &[OutputSpan],
) -> Option<ProjectedHtmlDocument> {
    project_inline_style_blocks(raw, None, source_output_spans)
}

fn project_inline_style_blocks(
    raw: &str,
    replacement: Option<String>,
    source_output_spans: &[OutputSpan],
) -> Option<ProjectedHtmlDocument> {
    let blocks = inline_style_blocks(raw);
    if blocks.is_empty() {
        return None;
    }

    let replacement_len = replacement.as_ref().map(String::len).unwrap_or_default();
    let mut html = String::with_capacity(raw.len() + replacement_len);
    let mut output_spans = Vec::new();
    let mut cursor = 0;
    let mut inserted = false;
    for block in blocks {
        let output_start = html.len();
        html.push_str(&raw[cursor..block.tag_start]);
        rebase_output_spans(
            source_output_spans,
            cursor,
            block.tag_start,
            output_start,
            &mut output_spans,
        );
        if !inserted {
            if let Some(replacement) = replacement.as_deref() {
                html.push_str(replacement);
                inserted = true;
            }
        }
        cursor = block.tag_end;
    }
    let output_start = html.len();
    html.push_str(&raw[cursor..]);
    rebase_output_spans(
        source_output_spans,
        cursor,
        raw.len(),
        output_start,
        &mut output_spans,
    );
    Some(ProjectedHtmlDocument {
        content: html,
        output_spans,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InlineStyleBlock {
    tag_start: usize,
    tag_end: usize,
    content_start: usize,
    content_end: usize,
}

fn inline_style_blocks(raw: &str) -> Vec<InlineStyleBlock> {
    let mut cursor = 0;
    let mut blocks = Vec::new();
    while let Some(start) = find_ascii_case_insensitive(raw, "<style", cursor) {
        let Some(open_end_offset) = raw[start..].find('>') else {
            break;
        };
        let content_start = start + open_end_offset + 1;
        let Some(close) = find_ascii_case_insensitive(raw, "</style>", content_start) else {
            break;
        };
        let tag_end = close + "</style>".len();
        blocks.push(InlineStyleBlock {
            tag_start: start,
            tag_end,
            content_start,
            content_end: close,
        });
        cursor = tag_end;
    }
    blocks
}

fn stylesheet_link_tag(href: &str, target: Option<&FormatIdentity>) -> String {
    let href = escape_html_attribute(href);
    if transform_graph_target_is_xhtml(target) {
        format!(r#"<link rel="stylesheet" href="{href}" />"#)
    } else {
        format!(r#"<link rel="stylesheet" href="{href}">"#)
    }
}

fn transform_graph_target_is_xhtml(target: Option<&FormatIdentity>) -> bool {
    let Some(target) = target else {
        return false;
    };
    target
        .content_type
        .as_deref()
        .map(content_type_essence)
        .is_some_and(|essence| essence == XHTML_CONTENT_TYPE)
        || target.schema.as_deref() == Some(XHTML_SCHEMA_URI)
}

fn escape_html_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn transform_graph_stylesheet_href(
    html_destination: Option<&str>,
    css_destination: &str,
) -> String {
    let Some(html_destination) = html_destination else {
        return css_destination.to_owned();
    };
    if has_uri_scheme(html_destination) || has_uri_scheme(css_destination) {
        return css_destination.to_owned();
    }

    let html_path = Path::new(html_destination);
    let css_path = Path::new(css_destination);
    let Some(html_parent) = html_path.parent() else {
        return css_destination.to_owned();
    };
    css_path
        .strip_prefix(html_parent)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty())
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| css_destination.to_owned())
}

fn transform_graph_stylesheet_href_for_export(
    export: &TransformGraphExport,
    exports: &[TransformGraphExport],
) -> Option<String> {
    exports
        .iter()
        .filter(|candidate| candidate.id != export.id && candidate.input == export.input)
        .find(|candidate| transform_graph_target_is_css(candidate.target.as_ref()))
        .and_then(|candidate| candidate.destination.as_deref())
        .map(|css_destination| {
            transform_graph_stylesheet_href(export.destination.as_deref(), css_destination)
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TransformGraphHtmlStyleProjection {
    Inline,
    Link(String),
    Omit,
}

fn transform_graph_html_style_projection_for_export(
    export: &TransformGraphExport,
    exports: &[TransformGraphExport],
) -> TransformGraphHtmlStyleProjection {
    match export.style_policy {
        TransformGraphStylePolicy::Inline => TransformGraphHtmlStyleProjection::Inline,
        TransformGraphStylePolicy::Omit => TransformGraphHtmlStyleProjection::Omit,
        TransformGraphStylePolicy::Link | TransformGraphStylePolicy::Auto => {
            if let Some(stylesheet_href) =
                transform_graph_stylesheet_href_for_export(export, exports)
            {
                TransformGraphHtmlStyleProjection::Link(stylesheet_href)
            } else {
                TransformGraphHtmlStyleProjection::Inline
            }
        }
    }
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    if needle.is_empty()
        || from > haystack.len()
        || needle.len() > haystack.len().saturating_sub(from)
    {
        return None;
    }

    (from..=haystack.len() - needle.len())
        .find(|index| haystack[*index..*index + needle.len()].eq_ignore_ascii_case(needle))
}

fn importmap_rewrite_diagnostic(
    uri: Option<String>,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        uri,
        code: code.to_owned(),
        severity: Severity::Fatal,
        message: message.into(),
        details: None,
        ..Diagnostic::default()
    }
}

fn script_tag_is_importmap(tag: &str) -> bool {
    if !tag.starts_with("<script") {
        return false;
    }
    let tag = tag.to_ascii_lowercase();
    tag.contains("type=\"importmap\"")
        || tag.contains("type='importmap'")
        || tag.contains("type=importmap")
}

fn find_html_importmap_script(html: &str) -> Result<Option<(usize, usize)>, &'static str> {
    let lower = html.to_ascii_lowercase();
    let mut cursor = 0usize;
    let mut found = None;
    while let Some(offset) = lower[cursor..].find("<script") {
        let start = cursor + offset;
        let Some(tag_end_offset) = lower[start..].find('>') else {
            break;
        };
        let tag_end = start + tag_end_offset;
        if script_tag_is_importmap(&lower[start..=tag_end]) {
            let content_start = tag_end + 1;
            let Some(end_offset) = lower[content_start..].find("</script>") else {
                return Err("unterminated");
            };
            let content_end = content_start + end_offset;
            if found.is_some() {
                return Err("duplicate");
            }
            found = Some((content_start, content_end));
            cursor = content_end + "</script>".len();
        } else {
            cursor = tag_end + 1;
        }
    }
    Ok(found)
}

fn importmap_imports_mut(value: &mut Value) -> Option<&mut serde_json::Map<String, Value>> {
    value
        .as_object_mut()?
        .entry("imports")
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
}

fn validate_importmap_source_imports(
    uri: Option<String>,
    current: &Value,
    expected: &BTreeMap<String, String>,
) -> Vec<Diagnostic> {
    if expected.is_empty() {
        return Vec::new();
    }
    let imports = current
        .get("imports")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut diagnostics = Vec::new();
    for (key, expected_value) in expected {
        match imports.get(key).and_then(Value::as_str) {
            Some(actual) if actual == expected_value => {}
            Some(actual) => diagnostics.push(importmap_rewrite_diagnostic(
                uri.clone(),
                "cem.importmap.source_mismatch",
                format!(
                    "importmap entry `{key}` points to `{actual}`; expected `{expected_value}`"
                ),
            )),
            None => diagnostics.push(importmap_rewrite_diagnostic(
                uri.clone(),
                "cem.importmap.source_missing",
                format!("importmap is missing source entry `{key}`"),
            )),
        }
    }
    diagnostics
}

fn apply_importmap_rewrite(
    uri: Option<String>,
    html: &str,
    rewrite: &TransformGraphImportMapRewrite,
) -> (Option<String>, Vec<Diagnostic>) {
    let script_range = match find_html_importmap_script(html) {
        Ok(Some(range)) => range,
        Ok(None) => {
            if rewrite.missing_policy == TransformGraphImportMapMissingPolicy::Ignore {
                return (Some(html.to_owned()), Vec::new());
            }
            if rewrite.missing_policy == TransformGraphImportMapMissingPolicy::Insert {
                let imports = rewrite
                    .target_imports
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                    .collect::<serde_json::Map<_, _>>();
                let mut root = serde_json::Map::new();
                root.insert("imports".to_owned(), Value::Object(imports));
                let serialized =
                    serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_default();
                let block = format!(
                    "    <script type=\"importmap\">\n{}\n    </script>\n",
                    serialized
                        .lines()
                        .map(|line| format!("      {line}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                let lower = html.to_ascii_lowercase();
                if let Some(head_end) = lower.find("</head>") {
                    let mut rewritten = String::new();
                    rewritten.push_str(&html[..head_end]);
                    rewritten.push_str(&block);
                    rewritten.push_str(&html[head_end..]);
                    return (Some(rewritten), Vec::new());
                }
                return (Some(format!("{block}{html}")), Vec::new());
            }
            return (
                None,
                vec![importmap_rewrite_diagnostic(
                    uri,
                    "cem.importmap.missing_script",
                    format!(
                        "rewrite-importmap node `{}` could not find `<script type=\"importmap\">`",
                        rewrite.id
                    ),
                )],
            );
        }
        Err("duplicate") => {
            return (
                None,
                vec![importmap_rewrite_diagnostic(
                    uri,
                    "cem.importmap.duplicate_script",
                    format!(
                        "rewrite-importmap node `{}` found more than one importmap script",
                        rewrite.id
                    ),
                )],
            )
        }
        Err(_) => {
            return (
                None,
                vec![importmap_rewrite_diagnostic(
                    uri,
                    "cem.importmap.unterminated_script",
                    format!(
                        "rewrite-importmap node `{}` found an unterminated importmap script",
                        rewrite.id
                    ),
                )],
            )
        }
    };

    let (content_start, content_end) = script_range;
    let script_content = html[content_start..content_end].trim();
    let mut importmap = match serde_json::from_str::<Value>(script_content) {
        Ok(value) if value.is_object() => value,
        Ok(_) => {
            return (
                None,
                vec![importmap_rewrite_diagnostic(
                    uri,
                    "cem.importmap.invalid_json",
                    format!(
                        "rewrite-importmap node `{}` requires an object importmap",
                        rewrite.id
                    ),
                )],
            )
        }
        Err(error) => {
            return (
                None,
                vec![importmap_rewrite_diagnostic(
                    uri,
                    "cem.importmap.invalid_json",
                    format!(
                        "rewrite-importmap node `{}` could not parse importmap JSON: {error}",
                        rewrite.id
                    ),
                )],
            )
        }
    };

    let mut diagnostics =
        validate_importmap_source_imports(uri.clone(), &importmap, &rewrite.source_imports);
    if !diagnostics.is_empty() {
        return (None, diagnostics);
    }

    match rewrite.mode {
        TransformGraphImportMapRewriteMode::ReplaceScript => {
            let imports = rewrite
                .target_imports
                .iter()
                .map(|(key, value)| (key.clone(), Value::String(value.clone())))
                .collect::<serde_json::Map<_, _>>();
            let mut root = serde_json::Map::new();
            root.insert("imports".to_owned(), Value::Object(imports));
            importmap = Value::Object(root);
        }
        TransformGraphImportMapRewriteMode::ReplaceImports
        | TransformGraphImportMapRewriteMode::Merge => {
            let Some(imports) = importmap_imports_mut(&mut importmap) else {
                diagnostics.push(importmap_rewrite_diagnostic(
                    uri.clone(),
                    "cem.importmap.imports_invalid",
                    format!(
                        "rewrite-importmap node `{}` requires an object `imports` map",
                        rewrite.id
                    ),
                ));
                return (None, diagnostics);
            };
            for (key, value) in &rewrite.target_imports {
                imports.insert(key.clone(), Value::String(value.clone()));
            }
        }
    }

    if let Some(imports) = importmap.get("imports").and_then(Value::as_object) {
        for (key, value) in imports {
            if value
                .as_str()
                .is_some_and(|target| target.contains("node_modules"))
            {
                diagnostics.push(importmap_rewrite_diagnostic(
                    uri.clone(),
                    "cem.importmap.node_modules_leak",
                    format!(
                        "rewrite-importmap node `{}` left `node_modules` in target entry `{key}`",
                        rewrite.id
                    ),
                ));
            }
        }
    }
    if !diagnostics.is_empty() {
        return (None, diagnostics);
    }

    let serialized = serde_json::to_string_pretty(&importmap).unwrap_or_else(|_| "{}".to_owned());
    let script_indent = html[..content_start]
        .rfind('\n')
        .map(|newline| {
            html[newline + 1..content_start]
                .chars()
                .take_while(|ch| ch.is_whitespace())
                .collect::<String>()
        })
        .unwrap_or_default();
    let json_indent = format!("{script_indent}  ");
    let indented = serialized
        .lines()
        .map(|line| format!("{json_indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let replacement = format!("\n{indented}\n{script_indent}");
    let mut rewritten = String::new();
    rewritten.push_str(&html[..content_start]);
    rewritten.push_str(&replacement);
    rewritten.push_str(&html[content_end..]);
    (Some(rewritten), Vec::new())
}

fn select_transform_template_adapter(
    context: &EngineContext,
    template: &TemplateInput,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Arc<dyn TransformTemplateAdapter>> {
    match template.identity.as_ref() {
        Some(identity) => match context.template_adapter_registry.select_adapter(identity) {
            TransformTemplateAdapterLookup::Matched(adapter) => Some(adapter),
            TransformTemplateAdapterLookup::Ambiguous(ids) => {
                diagnostics.push(Diagnostic {
                    uri: Some(template.uri.clone()),
                    code: "cem.transform_template.adapter_ambiguous".to_owned(),
                    severity: Severity::Fatal,
                    message: format!(
                        "multiple transform template adapters matched template identity: {}",
                        ids.join(", ")
                    ),
                    details: None,
                    ..Diagnostic::default()
                });
                None
            }
            TransformTemplateAdapterLookup::Unsupported => {
                diagnostics.push(Diagnostic {
                    uri: Some(template.uri.clone()),
                    code: TRANSFORM_TEMPLATE_UNSUPPORTED_CODE.to_owned(),
                    severity: Severity::Fatal,
                    message: "no transform template adapter matched template identity".to_owned(),
                    details: None,
                    ..Diagnostic::default()
                });
                None
            }
        },
        None => {
            diagnostics.push(Diagnostic {
                uri: Some(template.uri.clone()),
                code: TRANSFORM_TEMPLATE_UNSUPPORTED_CODE.to_owned(),
                severity: Severity::Fatal,
                message: "transform template identity is required for execution".to_owned(),
                details: None,
                ..Diagnostic::default()
            });
            None
        }
    }
}

struct TransformTemplateCompileSpec<'a> {
    context: &'a EngineContext,
    adapter: &'a Arc<dyn TransformTemplateAdapter>,
    template: &'a TemplateInput,
    template_kind: TransformTemplateKind,
    entrypoint: &'a TransformTemplateEntrypoint,
    params: &'a BTreeMap<String, Value>,
    data_bindings: &'a [String],
    module_options: TransformTemplateModuleOptions,
    execution_policy: TransformExecutionPolicy,
}

fn compile_transform_template(
    spec: TransformTemplateCompileSpec<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TransformTemplateCompiledArtifact> {
    let module_options = if spec.template_kind == TransformTemplateKind::CemNative {
        lower_transform_template_module_options(spec.template, spec.module_options, diagnostics)?
    } else {
        spec.module_options
    };
    let params = normalize_transform_template_module_params(
        spec.params,
        spec.entrypoint.name.as_deref(),
        &module_options,
    );
    validate_transform_template_module_contract(
        spec.template,
        spec.entrypoint,
        &params,
        &module_options,
        diagnostics,
    )?;
    let module_preflight = preflight_transform_template_modules(
        spec.context,
        spec.adapter.id(),
        spec.template,
        spec.entrypoint,
        &module_options,
        spec.execution_policy,
        diagnostics,
    )?;
    let imported_modules = parse_imported_template_modules(&module_preflight, diagnostics)?;
    validate_transform_template_call_sites_with_imported_modules(
        &spec.template.uri,
        &module_options,
        &module_preflight,
        &imported_modules,
        diagnostics,
    )?;
    let compiled_module_options = normalize_transform_template_module_call_arguments(
        &module_options,
        &module_preflight,
        &imported_modules,
    );
    match spec.adapter.compile(TransformTemplateCompileRequest {
        template: spec.template,
        entrypoint: spec.entrypoint,
        params: &params,
        data_bindings: spec.data_bindings,
        module_options: compiled_module_options.clone(),
        module_preflight,
        execution_policy: spec.execution_policy,
    }) {
        Ok(mut response) => {
            diagnostics.append(&mut response.diagnostics);
            Some(
                response
                    .artifact
                    .with_module_options(compiled_module_options),
            )
        }
        Err(err) => {
            diagnostics.push(err.diagnostic(Some(&spec.template.uri)));
            None
        }
    }
}

fn lower_transform_template_module_options(
    template: &TemplateInput,
    overlay_options: TransformTemplateModuleOptions,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TransformTemplateModuleOptions> {
    let mut response =
        parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
            template: template.clone(),
        });
    let has_fatal = response
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Fatal);
    diagnostics.append(&mut response.diagnostics);
    if has_fatal {
        return None;
    }

    let mut module_options = response.module_options;
    module_options.imports.extend(overlay_options.imports);
    module_options
        .entrypoints
        .extend(overlay_options.entrypoints);
    module_options.params.extend(overlay_options.params);
    module_options.calls.extend(overlay_options.calls);
    module_options
        .let_bindings
        .extend(overlay_options.let_bindings);
    module_options
        .encode_expressions
        .extend(overlay_options.encode_expressions);
    module_options.functions.extend(overlay_options.functions);
    module_options
        .output_functions
        .extend(overlay_options.output_functions);
    module_options.limits = overlay_options.limits;
    Some(module_options)
}

fn normalize_transform_template_module_params(
    params: &BTreeMap<String, Value>,
    selected_entrypoint: Option<&str>,
    module_options: &TransformTemplateModuleOptions,
) -> BTreeMap<String, Value> {
    let mut normalized = params.clone();
    for declaration in &module_options.params {
        for name in accepted_param_names(declaration, selected_entrypoint) {
            let Some(value) = normalized.get(name) else {
                continue;
            };
            let coerced = coerce_transform_template_param_value(declaration, value);
            normalized.insert(name.to_owned(), coerced);
        }
    }
    normalized
}

fn coerce_transform_template_param_value(
    declaration: &TransformTemplateModuleParamDeclaration,
    value: &Value,
) -> Value {
    let Value::String(raw) = value else {
        return value.clone();
    };

    if declaration.nullable && raw.trim() == "null" {
        return Value::Null;
    }

    match declaration.value_type {
        TransformTemplateModuleParamType::Any | TransformTemplateModuleParamType::String => {
            value.clone()
        }
        TransformTemplateModuleParamType::Boolean => match raw.trim() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => value.clone(),
        },
        TransformTemplateModuleParamType::Number
        | TransformTemplateModuleParamType::Integer
        | TransformTemplateModuleParamType::Array
        | TransformTemplateModuleParamType::Object
        | TransformTemplateModuleParamType::Json => {
            let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
                return value.clone();
            };
            if declaration
                .value_type
                .accepts(&parsed, declaration.nullable)
            {
                parsed
            } else {
                value.clone()
            }
        }
    }
}

fn validate_transform_template_module_contract(
    template: &TemplateInput,
    entrypoint: &TransformTemplateEntrypoint,
    params: &BTreeMap<String, Value>,
    module_options: &TransformTemplateModuleOptions,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    let has_module_contract = !module_options.imports.is_empty()
        || !module_options.entrypoints.is_empty()
        || !module_options.params.is_empty()
        || !module_options.calls.is_empty()
        || !module_options.let_bindings.is_empty()
        || !module_options.encode_expressions.is_empty()
        || !module_options.functions.is_empty()
        || !module_options.output_functions.is_empty();
    if !has_module_contract {
        return Some(());
    }

    let mut has_fatal = false;
    let selected_entrypoint = entrypoint.name.as_deref();

    if let Some(name) = selected_entrypoint {
        let public = module_options.entrypoints.iter().any(|decl| {
            decl.name == name && decl.visibility == TransformTemplateModuleVisibility::Public
        });
        if !public {
            diagnostics.push(template_module_diagnostic(
                Some(&template.uri),
                TRANSFORM_TEMPLATE_ENTRYPOINT_NOT_PUBLIC_CODE,
                format!("template entrypoint `{name}` is not declared public"),
            ));
            has_fatal = true;
        }
    }

    let mut allowed_params = BTreeSet::new();
    for declaration in &module_options.params {
        if declaration.visibility == TransformTemplateModuleVisibility::Public
            && !declaration.name.contains('.')
        {
            allowed_params.insert(declaration.name.clone());
        }

        if let Some(entrypoint_name) = selected_entrypoint {
            if let Some(local_name) = declaration
                .name
                .strip_prefix(entrypoint_name)
                .and_then(|remaining| remaining.strip_prefix('.'))
            {
                allowed_params.insert(local_name.to_owned());
                allowed_params.insert(declaration.name.clone());
            }
        }
    }

    for name in params.keys() {
        if !allowed_params.contains(name) {
            diagnostics.push(template_module_diagnostic(
                Some(&template.uri),
                TRANSFORM_TEMPLATE_PARAM_UNKNOWN_CODE,
                format!("template param `{name}` is not declared for this entrypoint"),
            ));
            has_fatal = true;
        }
    }

    let mut checked_alias_params = BTreeSet::new();
    for declaration in &module_options.params {
        let accepted_names = accepted_param_names(declaration, selected_entrypoint);
        if accepted_names.is_empty() {
            continue;
        }

        let display_name = accepted_names[0];
        if !checked_alias_params.insert(display_name.to_owned()) {
            continue;
        }
        let provided_names = accepted_names
            .iter()
            .filter(|name| params.contains_key(**name))
            .copied()
            .collect::<Vec<_>>();
        if provided_names.len() > 1 {
            diagnostics.push(template_module_diagnostic(
                Some(&template.uri),
                TRANSFORM_TEMPLATE_PARAM_DUPLICATE_ALIAS_CODE,
                format!(
                    "template param `{display_name}` is provided through duplicate aliases `{}`",
                    provided_names.join("`, `")
                ),
            ));
            has_fatal = true;
        }
    }

    let mut checked_typed_params = BTreeSet::new();
    for declaration in &module_options.params {
        let accepted_names = accepted_param_names(declaration, selected_entrypoint);
        if accepted_names.is_empty() {
            continue;
        }

        let display_name = accepted_names[0];
        if !checked_typed_params.insert(display_name.to_owned()) {
            continue;
        }

        if let Some(default_value) = &declaration.default_value {
            if !declaration
                .value_type
                .accepts(default_value, declaration.nullable)
            {
                diagnostics.push(template_module_diagnostic(
                    Some(&template.uri),
                    TRANSFORM_TEMPLATE_PARAM_TYPE_CODE,
                    format!(
                        "template param `{display_name}` default value does not match declared type `{}`",
                        declaration.value_type.as_contract_name()
                    ),
                ));
                has_fatal = true;
            }
        }

        for name in accepted_names {
            let Some(value) = params.get(name) else {
                continue;
            };
            if !declaration.value_type.accepts(value, declaration.nullable) {
                diagnostics.push(template_module_diagnostic(
                    Some(&template.uri),
                    TRANSFORM_TEMPLATE_PARAM_TYPE_CODE,
                    format!(
                        "template param `{name}` value does not match declared type `{}`",
                        declaration.value_type.as_contract_name()
                    ),
                ));
                has_fatal = true;
            }
        }
    }

    let mut checked_required_params = BTreeSet::new();
    for declaration in &module_options.params {
        if !declaration.required || declaration.default_value.is_some() {
            continue;
        }

        let accepted_names = accepted_param_names(declaration, selected_entrypoint);
        if accepted_names.is_empty() {
            continue;
        }
        let display_name = accepted_names[0];
        if !checked_required_params.insert(display_name.to_owned()) {
            continue;
        }
        if !accepted_names.iter().any(|name| params.contains_key(*name)) {
            diagnostics.push(template_module_diagnostic(
                Some(&template.uri),
                TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE,
                format!("template param `{display_name}` is required for this entrypoint"),
            ));
            has_fatal = true;
        }
    }

    if has_fatal {
        None
    } else {
        Some(())
    }
}

fn accepted_param_names<'a>(
    declaration: &'a TransformTemplateModuleParamDeclaration,
    selected_entrypoint: Option<&'a str>,
) -> Vec<&'a str> {
    let mut accepted_names = Vec::new();
    if declaration.visibility == TransformTemplateModuleVisibility::Public
        && !declaration.name.contains('.')
    {
        accepted_names.push(declaration.name.as_str());
    }
    if let Some(entrypoint_name) = selected_entrypoint {
        if let Some(local_name) = declaration
            .name
            .strip_prefix(entrypoint_name)
            .and_then(|remaining| remaining.strip_prefix('.'))
        {
            accepted_names.push(local_name);
            accepted_names.push(declaration.name.as_str());
        }
    }
    accepted_names
}

#[cfg(test)]
fn validate_transform_template_call_sites(
    template: &TemplateInput,
    module_options: &TransformTemplateModuleOptions,
    module_preflight: &TransformTemplateModulePreflight,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    let imported_modules = parse_imported_template_modules(module_preflight, diagnostics)?;
    validate_transform_template_call_sites_with_imported_modules(
        &template.uri,
        module_options,
        module_preflight,
        &imported_modules,
        diagnostics,
    )
}

fn validate_transform_template_call_sites_with_imported_modules(
    template_uri: &str,
    module_options: &TransformTemplateModuleOptions,
    module_preflight: &TransformTemplateModulePreflight,
    imported_modules: &BTreeMap<String, TransformTemplateModuleOptions>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    let mut has_fatal = !validate_module_call_sites(
        template_uri,
        module_options,
        None,
        module_preflight,
        imported_modules,
        diagnostics,
    );

    for module in &module_preflight.resolved_imports {
        let Some(imported_options) = imported_modules.get(&module.uri) else {
            continue;
        };
        if !validate_module_call_sites(
            &module.uri,
            imported_options,
            Some(module.uri.as_str()),
            module_preflight,
            imported_modules,
            diagnostics,
        ) {
            has_fatal = true;
        }
    }

    if has_fatal {
        None
    } else {
        Some(())
    }
}

fn normalize_transform_template_module_call_arguments(
    module_options: &TransformTemplateModuleOptions,
    module_preflight: &TransformTemplateModulePreflight,
    imported_modules: &BTreeMap<String, TransformTemplateModuleOptions>,
) -> TransformTemplateModuleOptions {
    let mut normalized = module_options.clone();
    for call in &mut normalized.calls {
        let target_options = match call.from.as_deref() {
            Some(alias) => find_preflight_import(module_preflight, None, alias)
                .and_then(|module| imported_modules.get(&module.uri)),
            None => Some(module_options),
        };
        if let Some(target_options) = target_options {
            call.arguments = normalize_transform_template_module_params(
                &call.arguments,
                Some(&call.template),
                target_options,
            );
        }
    }
    normalized
}

fn validate_module_call_sites(
    module_uri: &str,
    module_options: &TransformTemplateModuleOptions,
    parent_uri: Option<&str>,
    module_preflight: &TransformTemplateModulePreflight,
    imported_modules: &BTreeMap<String, TransformTemplateModuleOptions>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if module_options.calls.is_empty() {
        return true;
    }
    let mut has_fatal = false;
    let local_entrypoints: BTreeSet<&str> = module_options
        .entrypoints
        .iter()
        .map(|entrypoint| entrypoint.name.as_str())
        .collect();

    for call in &module_options.calls {
        match call.from.as_deref() {
            Some(alias) => {
                let Some(imported_module) =
                    find_preflight_import(module_preflight, parent_uri, alias)
                else {
                    diagnostics.push(template_module_diagnostic(
                        Some(module_uri),
                        TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE,
                        format!(
                            "template call to `{}` references unknown import alias `{alias}`",
                            call.template
                        ),
                    ));
                    has_fatal = true;
                    continue;
                };
                let Some(imported_options) = imported_modules.get(&imported_module.uri) else {
                    diagnostics.push(template_module_diagnostic(
                        Some(module_uri),
                        TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE,
                        format!(
                            "template call to `{}` references unresolved import alias `{alias}`",
                            call.template
                        ),
                    ));
                    has_fatal = true;
                    continue;
                };
                if !imported_options.entrypoints.iter().any(|entrypoint| {
                    entrypoint.name == call.template
                        && entrypoint.visibility == TransformTemplateModuleVisibility::Public
                }) {
                    diagnostics.push(template_module_diagnostic(
                        Some(module_uri),
                        TRANSFORM_TEMPLATE_ENTRYPOINT_NOT_PUBLIC_CODE,
                        format!(
                            "template call to `{alias}:{}` does not target a public imported entrypoint",
                            call.template
                        ),
                    ));
                    has_fatal = true;
                    continue;
                }
                if !validate_transform_template_call_arguments(
                    module_uri,
                    call,
                    imported_options,
                    diagnostics,
                ) {
                    has_fatal = true;
                }
            }
            None => {
                if !local_entrypoints.contains(call.template.as_str()) {
                    diagnostics.push(template_module_diagnostic(
                        Some(module_uri),
                        TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE,
                        format!(
                            "template call to `{}` does not target a declared same-module entrypoint",
                            call.template
                        ),
                    ));
                    has_fatal = true;
                    continue;
                }
                if !validate_transform_template_call_arguments(
                    module_uri,
                    call,
                    module_options,
                    diagnostics,
                ) {
                    has_fatal = true;
                }
            }
        }
    }

    !has_fatal
}

fn validate_transform_template_call_arguments(
    module_uri: &str,
    call: &crate::transform_template::TransformTemplateModuleCallSite,
    target_options: &TransformTemplateModuleOptions,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let params = call
        .arguments
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    let params =
        normalize_transform_template_module_params(&params, Some(&call.template), target_options);
    let selected_entrypoint = Some(call.template.as_str());
    let mut has_fatal = false;

    let mut allowed_params = BTreeSet::new();
    for declaration in &target_options.params {
        if declaration.visibility == TransformTemplateModuleVisibility::Public
            && !declaration.name.contains('.')
        {
            allowed_params.insert(declaration.name.clone());
        }

        if let Some(local_name) = declaration
            .name
            .strip_prefix(call.template.as_str())
            .and_then(|remaining| remaining.strip_prefix('.'))
        {
            allowed_params.insert(local_name.to_owned());
            allowed_params.insert(declaration.name.clone());
        }
    }

    for name in params.keys() {
        if !allowed_params.contains(name) {
            diagnostics.push(template_module_diagnostic(
                Some(module_uri),
                TRANSFORM_TEMPLATE_PARAM_UNKNOWN_CODE,
                format!(
                    "template call to `{}` passes undeclared param `{name}`",
                    call.template
                ),
            ));
            has_fatal = true;
        }
    }

    let mut checked_alias_params = BTreeSet::new();
    for declaration in &target_options.params {
        let accepted_names = accepted_param_names(declaration, selected_entrypoint);
        if accepted_names.is_empty() {
            continue;
        }

        let display_name = accepted_names[0];
        if !checked_alias_params.insert(display_name.to_owned()) {
            continue;
        }
        let provided_names = accepted_names
            .iter()
            .filter(|name| params.contains_key(**name))
            .copied()
            .collect::<Vec<_>>();
        if provided_names.len() > 1 {
            diagnostics.push(template_module_diagnostic(
                Some(module_uri),
                TRANSFORM_TEMPLATE_PARAM_DUPLICATE_ALIAS_CODE,
                format!(
                    "template call to `{}` provides param `{display_name}` through duplicate aliases `{}`",
                    call.template,
                    provided_names.join("`, `")
                ),
            ));
            has_fatal = true;
        }
    }

    let mut checked_typed_params = BTreeSet::new();
    for declaration in &target_options.params {
        let accepted_names = accepted_param_names(declaration, selected_entrypoint);
        if accepted_names.is_empty() {
            continue;
        }

        let display_name = accepted_names[0];
        if !checked_typed_params.insert(display_name.to_owned()) {
            continue;
        }

        for name in accepted_names {
            let Some(value) = params.get(name) else {
                continue;
            };
            if transform_template_call_argument_is_dynamic(value) {
                continue;
            }
            if !declaration.value_type.accepts(value, declaration.nullable) {
                diagnostics.push(template_module_diagnostic(
                    Some(module_uri),
                    TRANSFORM_TEMPLATE_PARAM_TYPE_CODE,
                    format!(
                        "template call to `{}` param `{name}` value does not match declared type `{}`",
                        call.template,
                        declaration.value_type.as_contract_name()
                    ),
                ));
                has_fatal = true;
            }
        }
    }

    let mut checked_required_params = BTreeSet::new();
    for declaration in &target_options.params {
        if !declaration.required || declaration.default_value.is_some() {
            continue;
        }

        let accepted_names = accepted_param_names(declaration, selected_entrypoint);
        if accepted_names.is_empty() {
            continue;
        }
        let display_name = accepted_names[0];
        if !checked_required_params.insert(display_name.to_owned()) {
            continue;
        }
        if !accepted_names.iter().any(|name| params.contains_key(*name)) {
            diagnostics.push(template_module_diagnostic(
                Some(module_uri),
                TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE,
                format!(
                    "template call to `{}` is missing required param `{display_name}`",
                    call.template
                ),
            ));
            has_fatal = true;
        }
    }

    !has_fatal
}

fn parse_imported_template_modules(
    module_preflight: &TransformTemplateModulePreflight,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<BTreeMap<String, TransformTemplateModuleOptions>> {
    let mut imported_modules = BTreeMap::new();
    let mut has_fatal = false;

    for module in &module_preflight.resolved_imports {
        let mut response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: TemplateInput {
                    uri: module.uri.clone(),
                    bytes: module.bytes.clone(),
                    identity: module.identity.clone(),
                    root_scope: ScopeConfig::default(),
                },
            });
        if response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Fatal)
        {
            has_fatal = true;
        }
        diagnostics.append(&mut response.diagnostics);
        imported_modules.insert(module.uri.clone(), response.module_options);
    }

    if has_fatal {
        None
    } else {
        Some(imported_modules)
    }
}

fn find_preflight_import<'a>(
    module_preflight: &'a TransformTemplateModulePreflight,
    parent_uri: Option<&str>,
    alias: &str,
) -> Option<&'a TransformTemplateResolvedModule> {
    module_preflight
        .resolved_imports
        .iter()
        .find(|module| module.parent_uri.as_deref() == parent_uri && module.alias == alias)
}

fn preflight_transform_template_modules(
    context: &EngineContext,
    adapter_id: &str,
    template: &TemplateInput,
    entrypoint: &TransformTemplateEntrypoint,
    module_options: &TransformTemplateModuleOptions,
    execution_policy: TransformExecutionPolicy,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TransformTemplateModulePreflight> {
    let mut ancestry = vec![template.uri.clone()];
    let mut seen_aliases = BTreeSet::new();
    let mut resolved_imports = Vec::new();
    let mut dependency_hash_input = Vec::new();
    let has_fatal = !preflight_template_module_imports(
        context,
        template,
        &module_options.imports,
        None,
        module_options.limits.max_import_depth,
        0,
        &mut ancestry,
        &mut seen_aliases,
        &mut resolved_imports,
        &mut dependency_hash_input,
        diagnostics,
    );

    if has_fatal {
        return None;
    }

    let template_hash = content_hash(&template.bytes);
    let dependency_graph_hash = content_hash(&dependency_hash_input);
    let cache_key = TransformTemplateModuleCacheKey::new(
        adapter_id,
        template.uri.clone(),
        template.identity.clone(),
        template_hash,
        entrypoint.clone(),
        execution_policy,
        dependency_graph_hash,
    );
    Some(TransformTemplateModulePreflight {
        resolved_imports,
        cache_key: Some(cache_key),
    })
}

#[allow(clippy::too_many_arguments)]
fn preflight_template_module_imports(
    context: &EngineContext,
    importing_template: &TemplateInput,
    imports: &[TransformTemplateModuleImport],
    parent_uri: Option<&str>,
    max_import_depth: u32,
    depth: u32,
    ancestry: &mut Vec<String>,
    seen_alias_scopes: &mut BTreeSet<(Option<String>, String)>,
    resolved_imports: &mut Vec<TransformTemplateResolvedModule>,
    dependency_hash_input: &mut Vec<u8>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut has_fatal = false;

    for import in imports {
        if import.kind == TransformTemplateModuleDependencyKind::IncludeReserved {
            diagnostics.push(template_module_diagnostic(
                Some(&import.uri),
                TRANSFORM_TEMPLATE_INCLUDE_RESERVED_CODE,
                format!(
                    "template module import `{}` uses reserved include semantics; use `import`",
                    import.alias
                ),
            ));
            has_fatal = true;
            continue;
        }

        let alias_scope = (parent_uri.map(str::to_owned), import.alias.clone());
        if !seen_alias_scopes.insert(alias_scope) {
            diagnostics.push(template_module_diagnostic(
                Some(&import.uri),
                TRANSFORM_TEMPLATE_IMPORT_ALIAS_DUPLICATE_CODE,
                format!(
                    "template module import alias `{}` is declared more than once",
                    import.alias
                ),
            ));
            has_fatal = true;
            continue;
        }

        if depth >= max_import_depth {
            diagnostics.push(template_module_diagnostic(
                Some(&import.uri),
                TRANSFORM_TEMPLATE_IMPORT_DEPTH_CODE,
                format!(
                    "template module import `{}` exceeds max import depth {max_import_depth}",
                    import.alias
                ),
            ));
            has_fatal = true;
            continue;
        }

        match read_template_module_import(context, importing_template, import, parent_uri) {
            Ok(module) => {
                if ancestry.iter().any(|uri| uri == &module.uri) {
                    diagnostics.push(template_module_diagnostic(
                        Some(&module.uri),
                        TRANSFORM_TEMPLATE_IMPORT_CYCLE_CODE,
                        format!(
                            "template module import `{}` creates an import cycle through `{}`",
                            import.alias, module.uri
                        ),
                    ));
                    has_fatal = true;
                    continue;
                }
                let module_template = TemplateInput {
                    uri: module.uri.clone(),
                    bytes: module.bytes.clone(),
                    identity: module.identity.clone(),
                    root_scope: ScopeConfig::default(),
                };
                let mut parse_response =
                    parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                        template: module_template.clone(),
                    });
                let module_has_fatal = parse_response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.severity == Severity::Fatal);
                diagnostics.append(&mut parse_response.diagnostics);
                if module_has_fatal {
                    has_fatal = true;
                    continue;
                }

                if let Some(parent) = parent_uri {
                    dependency_hash_input.extend_from_slice(parent.as_bytes());
                }
                dependency_hash_input.push(0);
                dependency_hash_input.extend_from_slice(import.alias.as_bytes());
                dependency_hash_input.push(0);
                dependency_hash_input.extend_from_slice(import.uri.as_bytes());
                dependency_hash_input.push(0);
                if let Some(normalized_uri) = &module.normalized_uri {
                    dependency_hash_input.extend_from_slice(normalized_uri.as_bytes());
                }
                dependency_hash_input.push(0);
                if let Some(substituted_uri) = &module.substituted_uri {
                    dependency_hash_input.extend_from_slice(substituted_uri.as_bytes());
                }
                dependency_hash_input.push(0);
                if let Some(resolver_policy_stamp) = &module.resolver_policy_stamp {
                    dependency_hash_input.extend_from_slice(resolver_policy_stamp.as_bytes());
                }
                dependency_hash_input.push(0);
                if let Some(content_type) = import
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.content_type.as_deref())
                {
                    dependency_hash_input.extend_from_slice(content_type.as_bytes());
                }
                dependency_hash_input.push(0);
                if let Some(schema) = import
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.schema.as_deref())
                {
                    dependency_hash_input.extend_from_slice(schema.as_bytes());
                }
                dependency_hash_input.push(0);
                dependency_hash_input.extend_from_slice(module.uri.as_bytes());
                dependency_hash_input.push(0);
                dependency_hash_input.extend_from_slice(module.content_hash.as_bytes());
                dependency_hash_input.push(0);

                let module_uri = module.uri.clone();
                resolved_imports.push(module);
                ancestry.push(module_uri.clone());
                if !preflight_template_module_imports(
                    context,
                    &module_template,
                    &parse_response.module_options.imports,
                    Some(module_uri.as_str()),
                    max_import_depth,
                    depth + 1,
                    ancestry,
                    seen_alias_scopes,
                    resolved_imports,
                    dependency_hash_input,
                    diagnostics,
                ) {
                    has_fatal = true;
                }
                ancestry.pop();
            }
            Err(error) => {
                diagnostics.push(template_import_resolution_diagnostic(
                    importing_template,
                    import,
                    parent_uri,
                    &error,
                ));
                has_fatal = true;
            }
        }
    }

    !has_fatal
}

#[derive(Debug, Clone)]
struct TemplateImportSourceRead {
    read: ResolvedRead,
    policy_decision: ResolvePolicyDecision,
}

#[derive(Debug, Clone)]
enum TemplateImportReadError {
    Policy(ResolvePolicyDiagnostic),
    Resolver {
        error: ResolverDiagnostic,
        policy_decision: ResolvePolicyDecision,
    },
}

impl TemplateImportReadError {
    fn source_uri(&self) -> &str {
        match self {
            Self::Policy(error) => error.request.uri.as_str(),
            Self::Resolver {
                policy_decision, ..
            } => policy_decision.requested_uri.as_str(),
        }
    }
}

impl std::fmt::Display for TemplateImportReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Policy(error) => write!(f, "{error}"),
            Self::Resolver { error, .. } => write!(f, "{error}"),
        }
    }
}

#[derive(Debug, Clone)]
struct TemplateImportResolutionDiagnosticKind {
    code: &'static str,
    fact_kind: &'static str,
    contract: &'static str,
    reason: String,
}

fn template_import_resolution_diagnostic(
    importing_template: &TemplateInput,
    import: &TransformTemplateModuleImport,
    parent_uri: Option<&str>,
    error: &TemplateImportReadError,
) -> Diagnostic {
    let kind = template_import_resolution_diagnostic_kind(error);
    let source_range = import.source_range.map(byte_range_details);
    let policy_decision = template_import_error_policy_decision(error);
    let requested_uri = policy_decision
        .map(|decision| decision.requested_uri.as_str())
        .unwrap_or_else(|| error.source_uri());
    let normalized_uri = policy_decision
        .map(|decision| Value::String(decision.normalized_uri.clone()))
        .unwrap_or_else(|| Value::String(error.source_uri().trim().to_owned()));
    let effective_uri = policy_decision
        .map(|decision| Value::String(decision.effective_uri.clone()))
        .unwrap_or(Value::Null);
    let substituted_uri = policy_decision
        .and_then(|decision| decision.substituted_uri.as_deref())
        .map(|uri| Value::String(uri.to_owned()))
        .unwrap_or(Value::Null);
    let policy_stamp = template_import_error_policy_stamp(error);
    let resolver_code = template_import_error_resolver_code(error)
        .map(Value::String)
        .unwrap_or(Value::Null);
    let resolver_uri = template_import_error_resolver_uri(error)
        .map(Value::String)
        .unwrap_or(Value::Null);
    Diagnostic {
        uri: Some(importing_template.uri.clone()),
        byte_offset: import.source_range.map(|range| range.start),
        code: kind.code.to_owned(),
        severity: Severity::Error,
        message: format!(
            "template module import `{}` could not resolve `{}`: {error}",
            import.alias, requested_uri
        ),
        details: Some(json!({
            "behavior": "cem-template-resolution-fact",
            "factKind": kind.fact_kind,
            "contract": kind.contract,
            "disposition": "artifact-blocking",
            "recoverable": true,
            "fatal": false,
            "requestedUri": requested_uri,
            "normalizedUri": normalized_uri,
            "effectiveUri": effective_uri,
            "resolvedUri": Value::Null,
            "substitutedUri": substituted_uri,
            "importAlias": import.alias.as_str(),
            "parentUri": parent_uri,
            "sourceUri": importing_template.uri.as_str(),
            "contentType": import
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            "schema": import
                .identity
                .as_ref()
                .and_then(|identity| identity.schema.as_deref()),
            "scheme": template_import_uri_scheme(requested_uri),
            "schemeTier": template_import_scheme_tier(requested_uri),
            "reason": kind.reason.as_str(),
            "resolverCode": resolver_code,
            "resolverUri": resolver_uri,
            "resolverPolicy": policy_stamp,
            "cacheStampBehavior": "artifact-blocking-no-cache-key",
            "sourceRange": source_range,
        })),
        ..Diagnostic::default()
    }
}

fn template_import_resolution_diagnostic_kind(
    error: &TemplateImportReadError,
) -> TemplateImportResolutionDiagnosticKind {
    match error {
        TemplateImportReadError::Policy(error) => TemplateImportResolutionDiagnosticKind {
            code: CEM_TEMPLATE_IMPORT_DENIED_CODE,
            fact_kind: "denied-import",
            contract: "external-template-imports-require-resolver-policy",
            reason: error.reason.clone(),
        },
        TemplateImportReadError::Resolver {
            error: ResolverDiagnostic::UnsupportedResolver { .. },
            policy_decision,
        } if policy_decision.substituted_uri.is_some() => TemplateImportResolutionDiagnosticKind {
            code: CEM_TEMPLATE_IMPORT_UNRESOLVED_CODE,
            fact_kind: "unresolved-import",
            contract: "import-target-must-resolve",
            reason: "substitution-target-missing".to_owned(),
        },
        TemplateImportReadError::Resolver {
            error: ResolverDiagnostic::Io { .. },
            policy_decision,
        } if policy_decision.substituted_uri.is_some() => TemplateImportResolutionDiagnosticKind {
            code: CEM_TEMPLATE_IMPORT_UNRESOLVED_CODE,
            fact_kind: "unresolved-import",
            contract: "import-target-must-resolve",
            reason: "substitution-target-missing".to_owned(),
        },
        TemplateImportReadError::Resolver {
            error: ResolverDiagnostic::UnsupportedResolver { .. },
            policy_decision,
        } if policy_decision.requested_uri.starts_with("cem:")
            || policy_decision.requested_uri.starts_with("urn:cem:") =>
        {
            TemplateImportResolutionDiagnosticKind {
                code: CEM_TEMPLATE_IMPORT_UNRESOLVED_CODE,
                fact_kind: "unresolved-import",
                contract: "import-target-must-resolve",
                reason: "registry-miss".to_owned(),
            }
        }
        TemplateImportReadError::Resolver {
            error:
                ResolverDiagnostic::UnsupportedResolver { .. }
                | ResolverDiagnostic::NonLocalFileUri { .. }
                | ResolverDiagnostic::InvalidFileUri { .. },
            policy_decision,
        } => TemplateImportResolutionDiagnosticKind {
            code: CEM_TEMPLATE_IMPORT_DENIED_CODE,
            fact_kind: "denied-import",
            contract: "external-template-imports-require-resolver-policy",
            reason: match error {
                TemplateImportReadError::Resolver {
                    error: ResolverDiagnostic::NonLocalFileUri { .. },
                    ..
                } => "non-local-file-uri",
                TemplateImportReadError::Resolver {
                    error: ResolverDiagnostic::InvalidFileUri { .. },
                    ..
                } => "invalid-file-uri",
                _ if template_import_uri_scheme(&policy_decision.requested_uri).is_some() => {
                    "missing-uri-scheme-grant"
                }
                _ => "resolver-unsupported",
            }
            .to_owned(),
        },
        TemplateImportReadError::Resolver {
            error: ResolverDiagnostic::Io { .. },
            ..
        } => TemplateImportResolutionDiagnosticKind {
            code: CEM_TEMPLATE_IMPORT_UNRESOLVED_CODE,
            fact_kind: "unresolved-import",
            contract: "import-target-must-resolve",
            reason: "read-failed".to_owned(),
        },
    }
}

fn template_import_error_policy_decision(
    error: &TemplateImportReadError,
) -> Option<&ResolvePolicyDecision> {
    match error {
        TemplateImportReadError::Policy(_) => None,
        TemplateImportReadError::Resolver {
            policy_decision, ..
        } => Some(policy_decision),
    }
}

fn template_import_error_policy_stamp(error: &TemplateImportReadError) -> &str {
    match error {
        TemplateImportReadError::Policy(error) => error.policy_stamp.as_str(),
        TemplateImportReadError::Resolver {
            policy_decision, ..
        } => policy_decision.policy_stamp.as_str(),
    }
}

fn template_import_error_resolver_code(error: &TemplateImportReadError) -> Option<String> {
    match error {
        TemplateImportReadError::Policy(_) => None,
        TemplateImportReadError::Resolver { error, .. } => Some(error.code().to_owned()),
    }
}

fn template_import_error_resolver_uri(error: &TemplateImportReadError) -> Option<String> {
    match error {
        TemplateImportReadError::Policy(_) => None,
        TemplateImportReadError::Resolver { error, .. } => {
            Some(resolver_diagnostic_uri(error).to_owned())
        }
    }
}

fn template_import_uri_scheme(uri: &str) -> Option<&str> {
    uri.split_once(':').map(|(scheme, _)| scheme)
}

fn template_import_scheme_tier(uri: &str) -> &'static str {
    if uri.starts_with("cem:") {
        "platform-stdlib"
    } else if uri.starts_with("urn:cem:") {
        "plugin-registry"
    } else if template_import_uri_scheme(uri).is_some() {
        "external-resource"
    } else {
        "relative-resource"
    }
}

fn resolver_diagnostic_uri(error: &ResolverDiagnostic) -> &str {
    match error {
        ResolverDiagnostic::UnsupportedResolver { uri, .. }
        | ResolverDiagnostic::NonLocalFileUri { uri }
        | ResolverDiagnostic::InvalidFileUri { uri, .. }
        | ResolverDiagnostic::Io { uri, .. } => uri.as_str(),
    }
}

fn byte_range_details(range: ByteRange) -> Value {
    json!({
        "span": {
            "start": range.start,
            "len": range.len,
            "end": range.end(),
        }
    })
}

fn read_template_module_import(
    context: &EngineContext,
    template: &TemplateInput,
    import: &TransformTemplateModuleImport,
    parent_uri: Option<&str>,
) -> Result<TransformTemplateResolvedModule, TemplateImportReadError> {
    let content_type_hint = import
        .identity
        .as_ref()
        .and_then(|identity| identity.content_type.as_deref().map(str::to_owned));
    let source_read = read_template_import_source_with_policy(
        context,
        template,
        &import.uri,
        content_type_hint.as_deref(),
    )?;
    let TemplateImportSourceRead {
        read,
        policy_decision,
    } = source_read;
    let mut identity = import.identity.clone();
    if let Some(content_type) = read.content_type.clone() {
        let mut resolved_identity = identity.unwrap_or_default();
        if resolved_identity.content_type.is_none() {
            resolved_identity.content_type = Some(content_type);
        }
        identity = Some(resolved_identity);
    }
    let content_hash = content_hash(&read.bytes);
    Ok(TransformTemplateResolvedModule {
        alias: import.alias.clone(),
        parent_uri: parent_uri.map(str::to_owned),
        requested_uri: Some(policy_decision.requested_uri),
        normalized_uri: Some(policy_decision.normalized_uri),
        substituted_uri: policy_decision.substituted_uri,
        resolver_policy_stamp: Some(policy_decision.policy_stamp),
        uri: read.uri,
        identity,
        content_hash,
        bytes: read.bytes,
    })
}

fn read_template_import_source_with_policy(
    context: &EngineContext,
    template: &TemplateInput,
    import_uri: &str,
    content_type_hint: Option<&str>,
) -> Result<TemplateImportSourceRead, TemplateImportReadError> {
    let mut request =
        ResolveRequest::new(import_uri, ResolvePurpose::Template, ResolveDirection::Read)
            .with_base_uri(template.uri.as_str());
    if let Some(content_type_hint) = content_type_hint {
        request = request.with_content_type_hint(content_type_hint);
    }
    let policy_decision = context
        .resolver_policy
        .decide(&request)
        .map_err(TemplateImportReadError::Policy)?;
    let read = read_template_import_source(
        context,
        template,
        &policy_decision.effective_uri,
        content_type_hint,
    )
    .map_err(|error| TemplateImportReadError::Resolver {
        error,
        policy_decision: policy_decision.clone(),
    })?;
    Ok(TemplateImportSourceRead {
        read,
        policy_decision,
    })
}

fn read_template_import_source(
    context: &EngineContext,
    template: &TemplateInput,
    import_uri: &str,
    content_type_hint: Option<&str>,
) -> Result<ResolvedRead, ResolverDiagnostic> {
    let base_uri = template.uri.as_str();
    if has_uri_scheme(import_uri) && !is_windows_drive_path(import_uri) {
        if let Some(path) = parse_local_file_uri(import_uri)
            .transpose()
            .map_err(|error| ResolverDiagnostic::InvalidFileUri {
                uri: import_uri.to_owned(),
                message: error.to_string(),
            })?
        {
            return read_local_template_import(import_uri, path, content_type_hint);
        }
        if let Some(read) =
            read_registered_template_import(context, import_uri, None, content_type_hint)?
        {
            return Ok(read);
        }
        return Err(ResolverDiagnostic::UnsupportedResolver {
            uri: import_uri.to_owned(),
            purpose: ResolvePurpose::Template,
            direction: ResolveDirection::Read,
        });
    }

    if has_uri_scheme(base_uri) && !is_windows_drive_path(base_uri) {
        if let Some(base_path) = parse_local_file_uri(base_uri)
            .transpose()
            .map_err(|error| ResolverDiagnostic::InvalidFileUri {
                uri: base_uri.to_owned(),
                message: error.to_string(),
            })?
        {
            let path = base_path
                .parent()
                .map(|parent| parent.join(import_uri))
                .unwrap_or_else(|| PathBuf::from(import_uri));
            let uri = path.to_string_lossy().into_owned();
            return read_local_template_import(&uri, path, content_type_hint);
        }
        if let Some(read) =
            read_registered_template_import(context, import_uri, Some(base_uri), content_type_hint)?
        {
            return Ok(read);
        }
        return Err(ResolverDiagnostic::UnsupportedResolver {
            uri: import_uri.to_owned(),
            purpose: ResolvePurpose::Template,
            direction: ResolveDirection::Read,
        });
    }

    let path = PathBuf::from(base_uri)
        .parent()
        .map(|parent| parent.join(import_uri))
        .unwrap_or_else(|| PathBuf::from(import_uri));
    let uri = path.to_string_lossy().into_owned();
    read_local_template_import(&uri, path, content_type_hint)
}

fn read_registered_template_import(
    context: &EngineContext,
    uri: &str,
    base_uri: Option<&str>,
    content_type_hint: Option<&str>,
) -> Result<Option<ResolvedRead>, ResolverDiagnostic> {
    let mut request = ResolveRequest::new(uri, ResolvePurpose::Template, ResolveDirection::Read);
    if let Some(base_uri) = base_uri {
        request = request.with_base_uri(base_uri);
    }
    if let Some(content_type_hint) = content_type_hint {
        request = request.with_content_type_hint(content_type_hint);
    }
    match context.resolver_registry.read(&request) {
        Ok(read) => Ok(Some(read)),
        Err(ResolverDiagnostic::UnsupportedResolver { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

fn read_local_template_import(
    uri: &str,
    path: PathBuf,
    content_type_hint: Option<&str>,
) -> Result<ResolvedRead, ResolverDiagnostic> {
    std::fs::read(&path)
        .map(|bytes| ResolvedRead {
            uri: uri.to_owned(),
            bytes,
            content_type: content_type_hint.map(str::to_owned),
        })
        .map_err(|error| ResolverDiagnostic::Io {
            uri: uri.to_owned(),
            message: error.to_string(),
        })
}

fn content_hash(bytes: &[u8]) -> String {
    format!("cem-bin/1+blake3:{}", blake3::hash(bytes).to_hex())
}

fn text_content_hash(bytes: &[u8]) -> String {
    format!("cem-text/1+blake3:{}", blake3::hash(bytes).to_hex())
}

fn primary_bytes_from_text_document(document: &GenericDataTextDocument) -> PrimaryBytes {
    let bytes = document.content.as_bytes().to_vec();
    PrimaryBytes {
        content_type: document.content_type.clone(),
        schema: Some(document.schema.clone()),
        format_version: "cem-text/1".to_owned(),
        hash_scheme: "cem-text/1+blake3".to_owned(),
        hash: text_content_hash(&bytes),
        bytes,
    }
}

fn primary_bytes_from_binary_artifact(
    artifact: &projection::BinaryProjectionArtifact,
) -> PrimaryBytes {
    let stream = artifact.to_chunk_stream();
    let routed = projection::route_projection_stream(
        &stream,
        &[projection::ProjectionStreamRoute::new("primary")],
        projection::ProjectionRouteMode::Deterministic,
    )
    .expect("deterministic projection stream routing must not fail");
    let bytes = routed
        .into_iter()
        .next()
        .map(|route| route.concatenated_bytes())
        .unwrap_or_default();
    PrimaryBytes {
        content_type: artifact.content_type.clone(),
        schema: Some(artifact.schema.clone()),
        format_version: artifact.format_version.clone(),
        hash_scheme: artifact.hash_scheme.clone(),
        hash: artifact.hash.clone(),
        bytes,
    }
}

fn maybe_convert_generic_data_text(
    request: &ConvertRequest,
    started_at: Instant,
) -> Option<ConvertResponse> {
    let source = request
        .input
        .identity
        .clone()
        .unwrap_or_else(|| request.input.root_scope.format_identity());
    let target = request
        .target
        .clone()
        .or_else(|| request.target_scope.format_identity_option())?;

    let outcome = crate::conversion::convert_generic_data_text(
        &request.context.schema_registry,
        &source,
        &target,
        Some(&request.input.uri),
        &request.input.bytes,
    );

    let mut diagnostics = Vec::new();
    diagnostics.extend(root_scope_metadata_diagnostics(
        &request.input.uri,
        &request.input.root_scope,
        "input",
    ));
    diagnostics.extend(root_scope_metadata_diagnostics(
        &request.input.uri,
        &request.target_scope,
        "output",
    ));

    match outcome {
        GenericDataTextConversionOutcome::Unsupported => None,
        GenericDataTextConversionOutcome::Converted {
            document,
            diagnostics: mut conversion_diagnostics,
        } => {
            diagnostics.append(&mut conversion_diagnostics);
            append_convert_time_budget_diagnostics(&mut diagnostics, request, started_at);
            let primary_bytes = primary_bytes_from_text_document(&document);
            let hash = primary_bytes.hash.clone();
            Some(ConvertResponse {
                primary: json!({
                    "kind": "document",
                    "contentType": document.content_type,
                    "schema": document.schema,
                    "hash": hash,
                }),
                primary_bytes: Some(primary_bytes),
                conversion: None,
                diagnostics,
                scheduler_trace: crate::report::SchedulerTraceReport::default(),
            })
        }
        GenericDataTextConversionOutcome::Failed {
            diagnostics: mut conversion_diagnostics,
        } => {
            diagnostics.append(&mut conversion_diagnostics);
            append_convert_time_budget_diagnostics(&mut diagnostics, request, started_at);
            Some(ConvertResponse {
                primary: Value::Null,
                primary_bytes: None,
                conversion: None,
                diagnostics,
                scheduler_trace: crate::report::SchedulerTraceReport::default(),
            })
        }
    }
}

fn maybe_convert_csv_text(
    request: &ConvertRequest,
    started_at: Instant,
) -> Option<ConvertResponse> {
    let source = request
        .input
        .identity
        .clone()
        .unwrap_or_else(|| request.input.root_scope.format_identity());
    let target = request
        .target
        .clone()
        .or_else(|| request.target_scope.format_identity_option())?;
    if !format_identity_matches_csv(&request.context, &source)
        || !format_identity_matches_csv(&request.context, &target)
    {
        return None;
    }

    let mut diagnostics = Vec::new();
    diagnostics.extend(root_scope_metadata_diagnostics(
        &request.input.uri,
        &request.input.root_scope,
        "input",
    ));
    diagnostics.extend(root_scope_metadata_diagnostics(
        &request.input.uri,
        &request.target_scope,
        "output",
    ));

    let content_type = source
        .content_type
        .as_deref()
        .or(request.input.root_scope.default_content_type.as_deref())
        .unwrap_or(CSV_CONTENT_TYPE);
    let (table_value, mut csv_diagnostics) =
        crate::validation::csv::csv_table_value_from_source_bytes(
            crate::validation::csv::CsvSourceValidationRequest {
                bytes: &request.input.bytes,
                source_uri: &request.input.uri,
                content_type: Some(content_type),
            },
        );
    diagnostics.append(&mut csv_diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        append_convert_time_budget_diagnostics(&mut diagnostics, request, started_at);
        return Some(ConvertResponse {
            primary: Value::Null,
            primary_bytes: None,
            conversion: Some(convert_metadata_for_csv_direct_output(
                &request.target_scope,
            )),
            diagnostics,
            scheduler_trace: crate::report::SchedulerTraceReport::default(),
        });
    }

    let table_value = table_value?;
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &request.context.schema_registry,
        conversion_registry: &request.context.converter_registry,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let execution = execute_csv_document_output_pipeline_with_environment(
        &environment,
        table_value,
        &request.target_scope,
        Some(&request.input.uri),
    );
    diagnostics.extend(execution.diagnostics.clone());
    append_convert_time_budget_diagnostics(&mut diagnostics, request, started_at);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Some(ConvertResponse {
            primary: Value::Null,
            primary_bytes: None,
            conversion: Some(convert_metadata_for_csv_direct_output(
                &request.target_scope,
            )),
            diagnostics,
            scheduler_trace: crate::report::SchedulerTraceReport::default(),
        });
    }

    let content = execution
        .output
        .as_ref()
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let (content_type, schema, format_version) =
        csv_direct_output_primary_identity(&target, &request.target_scope);
    let bytes = content.into_bytes();
    let primary_bytes = PrimaryBytes {
        content_type,
        schema: Some(schema.clone()),
        format_version,
        hash_scheme: "cem-text/1+blake3".to_owned(),
        hash: text_content_hash(&bytes),
        bytes,
    };
    let hash = primary_bytes.hash.clone();
    Some(ConvertResponse {
        primary: json!({
            "kind": "document",
            "contentType": primary_bytes.content_type,
            "schema": schema,
            "hash": hash,
        }),
        primary_bytes: Some(primary_bytes),
        conversion: Some(convert_metadata_for_csv_direct_output(
            &request.target_scope,
        )),
        diagnostics,
        scheduler_trace: crate::report::SchedulerTraceReport::default(),
    })
}

fn format_identity_matches_csv(context: &EngineContext, identity: &FormatIdentity) -> bool {
    let explicit_schema_matches = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == CSV_SCHEMA_URI);
    if let Some(content_type) = identity.content_type.as_deref() {
        let essence = content_type_essence(content_type);
        if essence == CSV_CONTENT_TYPE {
            return identity.schema.is_none() || explicit_schema_matches;
        }
        if let Ok(descriptor) = context.schema_registry.resolve_content_type(content_type) {
            return descriptor.schema_uri == CSV_SCHEMA_URI
                && (identity.schema.is_none() || explicit_schema_matches);
        }
    }
    explicit_schema_matches
}

fn csv_direct_output_primary_identity(
    target: &FormatIdentity,
    target_scope: &ScopeConfig,
) -> (String, String, String) {
    if csv_direct_output_color_selection(target_scope)
        .as_ref()
        .is_some_and(csv_direct_output_color_selection_is_html)
    {
        return (
            HTML_CONTENT_TYPE.to_owned(),
            HTML_SCHEMA_URI.to_owned(),
            "csv-html/1".to_owned(),
        );
    }
    (
        target
            .content_type
            .clone()
            .unwrap_or_else(|| CSV_CONTENT_TYPE.to_owned()),
        target
            .schema
            .clone()
            .unwrap_or_else(|| CSV_SCHEMA_URI.to_owned()),
        "csv/1".to_owned(),
    )
}

fn csv_direct_output_color_selection(
    target_scope: &ScopeConfig,
) -> Option<TransformTemplateOutputColorSelection> {
    if let Some(output_color_type) = target_scope
        .output_color_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return parse_transform_template_output_color_type(output_color_type).ok();
    }

    match target_scope
        .cemt_color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
    {
        Some("html") => parse_transform_template_output_color_type("html").ok(),
        _ => None,
    }
}

fn csv_direct_output_color_selection_requests_color(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.output_color_type != "none"
}

fn csv_direct_output_color_selection_is_html(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "html-color"
        && csv_direct_output_color_selection_requests_color(selection)
}

fn csv_direct_output_inferred_color_profile(target_scope: &ScopeConfig) -> Option<String> {
    target_scope.cemt_color_profile.clone().or_else(|| {
        csv_direct_output_color_selection(target_scope)
            .filter(csv_direct_output_color_selection_requests_color)
            .and_then(|selection| match selection.target.category.as_str() {
                "html-color" => Some("html".to_owned()),
                "terminal-color" => Some("terminal".to_owned()),
                _ => None,
            })
    })
}

fn csv_direct_output_writer_stage_identity(
    target_scope: &ScopeConfig,
) -> (&'static str, &'static str, &'static str) {
    if csv_direct_output_color_selection(target_scope)
        .as_ref()
        .is_some_and(csv_direct_output_color_selection_is_html)
    {
        (HTML_CONTENT_TYPE, HTML_SCHEMA_URI, "html-document")
    } else {
        (CSV_CONTENT_TYPE, CSV_SCHEMA_URI, "csv-document")
    }
}

fn convert_metadata_for_csv_direct_output(target_scope: &ScopeConfig) -> ConvertExecutionMetadata {
    let formatter_profile = target_scope
        .cemt_formatter_profile
        .clone()
        .unwrap_or_else(|| "compact".to_owned());
    let color_profile = csv_direct_output_inferred_color_profile(target_scope);
    let writer_profile = csv_direct_output_color_selection(target_scope)
        .map(|selection| selection.output_color_type)
        .or_else(|| color_profile.clone());
    let (writer_content_type, writer_schema, writer_category) =
        csv_direct_output_writer_stage_identity(target_scope);
    ConvertExecutionMetadata {
        converter_id: Some("csv-direct-output".to_owned()),
        implementation: Some("direct-cemt-output-pipeline".to_owned()),
        rust_fallback: None,
        output_pipeline: Some(ConvertOutputPipelineMetadata {
            stages: vec![
                ConvertOutputPipelineStageMetadata {
                    stage: "formatter".to_owned(),
                    function: Some(
                        target_scope
                            .cemt_formatter
                            .clone()
                            .unwrap_or_else(|| "csv.format-document".to_owned()),
                    ),
                    profile: Some(formatter_profile),
                    content_type: Some(CSV_CONTENT_TYPE.to_owned()),
                    schema: Some(CSV_SCHEMA_URI.to_owned()),
                    category: Some("csv-document".to_owned()),
                    produces: Some(
                        TransformTemplateOutputProducedKind::CemTree
                            .as_str()
                            .to_owned(),
                    ),
                },
                ConvertOutputPipelineStageMetadata {
                    stage: "colorizer".to_owned(),
                    function: target_scope.cemt_colorizer.clone().or_else(|| {
                        color_profile
                            .as_ref()
                            .map(|_| "csv.color-document".to_owned())
                    }),
                    profile: color_profile,
                    content_type: Some(CSV_CONTENT_TYPE.to_owned()),
                    schema: Some(CSV_SCHEMA_URI.to_owned()),
                    category: Some("csv-document".to_owned()),
                    produces: Some(
                        TransformTemplateOutputProducedKind::CemTree
                            .as_str()
                            .to_owned(),
                    ),
                },
                ConvertOutputPipelineStageMetadata {
                    stage: "writer".to_owned(),
                    function: None,
                    profile: writer_profile,
                    content_type: Some(writer_content_type.to_owned()),
                    schema: Some(writer_schema.to_owned()),
                    category: Some(writer_category.to_owned()),
                    produces: Some(
                        TransformTemplateOutputProducedKind::Text
                            .as_str()
                            .to_owned(),
                    ),
                },
            ],
        }),
    }
}

fn append_convert_time_budget_diagnostics(
    diagnostics: &mut Vec<Diagnostic>,
    request: &ConvertRequest,
    started_at: Instant,
) {
    let elapsed_ns = started_at.elapsed().as_nanos();
    diagnostics.extend(time_budget_diagnostics(
        &request.input.root_scope,
        &["convertms", "converttimebudgetms"],
        elapsed_ns,
    ));
    diagnostics.extend(time_budget_diagnostics(
        &request.target_scope,
        &["convertms", "converttimebudgetms"],
        elapsed_ns,
    ));
}

fn template_module_diagnostic(
    uri: Option<&str>,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        uri: uri.map(str::to_owned),
        code: code.to_owned(),
        severity: Severity::Fatal,
        message: message.into(),
        details: None,
        ..Diagnostic::default()
    }
}

struct TransformStageRenderSpec<'a> {
    context: &'a EngineContext,
    adapter: &'a Arc<dyn TransformTemplateAdapter>,
    compiled: &'a TransformTemplateCompiledArtifact,
    primary_input: &'a TransformTemplateDataArtifact,
    secondary_inputs: &'a BTreeMap<String, TransformTemplateDataArtifact>,
    target: Option<&'a FormatIdentity>,
    target_scope: &'a ScopeConfig,
    execution_policy: TransformExecutionPolicy,
    diagnostic_uri: &'a str,
    diagnostic_node: Option<&'a str>,
}

fn render_transform_stage(
    spec: TransformStageRenderSpec<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TransformTemplateOutputArtifact> {
    match spec.adapter.render(TransformTemplateRenderRequest {
        compiled: spec.compiled,
        primary_input: spec.primary_input,
        secondary_inputs: spec.secondary_inputs,
        target: spec.target,
        target_scope: spec.target_scope,
        execution_policy: spec.execution_policy,
    }) {
        Ok(mut response) => {
            annotate_transform_stage_diagnostics(&mut response.diagnostics, spec.diagnostic_node);
            diagnostics.append(&mut response.diagnostics);
            apply_render_encode_expressions(&spec, &mut response.output, diagnostics);
            Some(response.output)
        }
        Err(err) => {
            let mut diagnostic = err.diagnostic(Some(spec.diagnostic_uri));
            annotate_transform_stage_diagnostics(
                std::slice::from_mut(&mut diagnostic),
                spec.diagnostic_node,
            );
            diagnostics.push(diagnostic);
            None
        }
    }
}

fn apply_render_encode_expressions(
    spec: &TransformStageRenderSpec<'_>,
    output: &mut TransformTemplateOutputArtifact,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if spec.compiled.module_options.encode_expressions.is_empty() {
        return;
    }

    let registry =
        TransformTemplateOutputFunctionRegistry::from_module_options(&spec.compiled.module_options);
    let value_bindings = match transform_template_render_value_bindings(spec) {
        Ok(value_bindings) => value_bindings,
        Err(message) => {
            diagnostics.push(Diagnostic {
                uri: Some(spec.diagnostic_uri.to_owned()),
                code: TRANSFORM_TEMPLATE_LET_EXPR_INVALID_CODE.to_owned(),
                severity: Severity::Fatal,
                message,
                node: spec.diagnostic_node.map(str::to_owned),
                ..Diagnostic::default()
            });
            return;
        }
    };
    let mut evaluated = evaluate_transform_template_encode_expressions(
        &spec.compiled.module_options.encode_expressions,
        TransformTemplateEncodeEvaluationContext {
            registry: &registry,
            value_bindings: &value_bindings,
            host_capabilities: spec
                .context
                .transform_template_encode_registry
                .host_capabilities(),
            output_color_type: None,
            uri: Some(spec.diagnostic_uri),
        },
        |binding, subject| {
            spec.context
                .transform_template_encode_registry
                .encode(binding, subject)
        },
    );

    let mut encode_diagnostics = std::mem::take(&mut evaluated.diagnostics);
    annotate_transform_stage_diagnostics(&mut encode_diagnostics, spec.diagnostic_node);
    diagnostics.append(&mut encode_diagnostics);
    if evaluated.encoded.is_empty() {
        return;
    }

    let insertion_context = transform_template_render_insertion_context(spec, output);
    let mut composition =
        evaluated.compose_text_artifacts(&insertion_context, Some(spec.diagnostic_uri));
    annotate_transform_stage_diagnostics(&mut composition.diagnostics, spec.diagnostic_node);
    diagnostics.append(&mut composition.diagnostics);

    if let Some(artifact) = composition.artifact {
        *output = artifact.into_output_artifact(output.uri.clone());
    }
}

fn transform_template_render_value_bindings(
    spec: &TransformStageRenderSpec<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    let mut value_bindings = BTreeMap::new();
    value_bindings.insert(
        spec.primary_input.artifact_id.clone(),
        spec.primary_input.value.clone(),
    );
    value_bindings
        .entry("input".to_owned())
        .or_insert_with(|| spec.primary_input.value.clone());

    for (name, artifact) in spec.secondary_inputs {
        value_bindings.insert(name.clone(), artifact.value.clone());
        value_bindings.insert(artifact.artifact_id.clone(), artifact.value.clone());
    }

    try_apply_transform_template_let_bindings(
        &mut value_bindings,
        &spec.compiled.module_options.let_bindings,
        spec.compiled.entrypoint.name.as_deref(),
    )?;

    Ok(value_bindings)
}

fn transform_template_render_insertion_context(
    spec: &TransformStageRenderSpec<'_>,
    output: &TransformTemplateOutputArtifact,
) -> TransformTemplateEncodedArtifactInsertionContext {
    let mut context = spec
        .target
        .or(output.identity.as_ref())
        .map(TransformTemplateEncodedArtifactInsertionContext::from_format_identity)
        .unwrap_or_default();
    context.produces = Some(TransformTemplateOutputProducedKind::Text);
    context
}

fn annotate_transform_stage_diagnostics(
    diagnostics: &mut [Diagnostic],
    diagnostic_node: Option<&str>,
) {
    let Some(node) = diagnostic_node else {
        return;
    };
    for diagnostic in diagnostics {
        if diagnostic.node.is_none() {
            diagnostic.node = Some(node.to_owned());
        }
    }
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
    budget_aliases: &[&str],
) -> EngineResult<(Vec<Diagnostic>, crate::scheduler::SchedulerTrace)> {
    let trace = crate::scheduler::SchedulerTrace::new();
    let abort = crate::scheduler::AbortSignal::new();
    let mut all_diags: Vec<Diagnostic> = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let started_at = Instant::now();
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
        let mut source_bytes_for_projection: Option<Vec<u8>> = None;
        pool.run_to_completion(&abort, |task| {
            if task.ends_with(":lifecycle-load") {
                let mut scope_diagnostics =
                    root_scope_metadata_diagnostics(&input.uri, &input.root_scope, "input");
                input_diags.append(&mut scope_diagnostics);
                let mut loaded = load_input_through_lifecycle(input, context);
                input_diags.append(&mut loaded.diagnostics);
                source_bytes_for_projection = Some(loaded.bytes.clone());
                loaded_input = Some(loaded);
                return;
            }

            let mut loaded = loaded_input
                .take()
                .unwrap_or_else(|| load_input_through_lifecycle(input, context));
            input_diags.append(&mut loaded.diagnostics);
            source_bytes_for_projection = Some(loaded.bytes.clone());
            if is_transform_config_schema(input, context) {
                input_diags.extend(validate_transform_config_document(
                    input,
                    context,
                    &loaded.bytes,
                ));
            } else {
                let run = run_pipeline_as_scoped_with_context_and_source_uri(
                    &loaded.bytes,
                    loaded.from_format,
                    &input.root_scope,
                    context,
                    &input_uri(input, context),
                );
                if is_schema_package_manifest_schema(input, context) {
                    if let Some(manifest_path) = input_local_path(input, context) {
                        input_diags.extend(validate_schema_package_source_consistency(
                            &manifest_path,
                            &run.document,
                        ));
                    }
                }
                if is_cem_native_template_schema(input, context) {
                    let identity = effective_input_identity(input, context);
                    input_diags.extend(validate_cem_native_template_source_semantics(
                        &loaded.bytes,
                        &input_uri(input, context),
                        identity.content_type.as_deref(),
                        identity.schema.as_deref(),
                    ));
                }
                input_diags.extend(run.diagnostics);
            }
        });
        input_diags.extend(time_budget_diagnostics(
            &input.root_scope,
            budget_aliases,
            started_at.elapsed().as_nanos(),
        ));
        if let Some(source_bytes) = source_bytes_for_projection.as_ref() {
            project_diagnostics_for_source(&mut input_diags, source_bytes);
        }
        project_diagnostic_uris(&mut input_diags, input, context);
        all_diags.extend(input_diags);
    }
    Ok((all_diags, trace))
}

fn effective_input_identity(input: &EngineInput, context: &EngineContext) -> FormatIdentity {
    input
        .identity
        .clone()
        .or_else(|| input.root_scope.format_identity_option())
        .unwrap_or_else(|| FormatIdentity::from(context))
}

fn parser_stage_report_for_input(
    input: &EngineInput,
    context: &EngineContext,
    from_format: InputFormat,
    source_len: usize,
    diagnostics: &[Diagnostic],
) -> crate::report::ParserStageReport {
    let display_uri = input_uri(input, context);
    let identity = effective_input_identity(input, context);
    let stages = ["tokenize", "normalize", "schema", "ast", "validate"]
        .into_iter()
        .map(|stage| crate::report::ParserStageReportEntry {
            input: display_uri.clone(),
            scope_id: 0,
            stage: stage.to_owned(),
            identity: identity.clone(),
            source_map_boundary: crate::report::ParserStageSourceMapBoundary {
                source_id: 1,
                byte_start: 0,
                byte_len: source_len as u64,
                transform: parser_stage_boundary_transform(stage, from_format).to_owned(),
            },
            diagnostics: parser_stage_diagnostics(stage, diagnostics),
        })
        .collect::<Vec<_>>();
    crate::report::ParserStageReport {
        stage_count: stages.len() as u64,
        stages,
    }
}

fn parser_stage_boundary_transform(stage: &str, from_format: InputFormat) -> &'static str {
    match stage {
        "tokenize" => match from_format {
            InputFormat::Cem => "cem-tokenizer",
            InputFormat::Html => "html-tokenizer",
            InputFormat::Xml => "xml-tokenizer",
        },
        "normalize" => "event-normalizer",
        "schema" => "schema-validation",
        "ast" => "cem-ast-builder",
        "validate" => "validation-rules",
        _ => "unknown",
    }
}

fn parser_stage_diagnostics(stage: &str, diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|diagnostic| parser_stage_owns_diagnostic(stage, &diagnostic.code))
        .cloned()
        .collect()
}

fn parser_stage_owns_diagnostic(stage: &str, code: &str) -> bool {
    match stage {
        "tokenize" => {
            code.starts_with("cem.token")
                || code.starts_with("cem.source")
                || code.contains(".encoding")
        }
        "normalize" => code.starts_with("cem.event") || code.starts_with("cem.projection.events"),
        "schema" => code.starts_with("cem.schema") || code.starts_with("cem.doc."),
        "ast" => {
            code.starts_with("cem.parser")
                || code.starts_with("cem.ast")
                || code.starts_with("cem.projection.ast")
                || code.starts_with("cem.projection.dom")
        }
        "validate" => {
            !parser_stage_owns_diagnostic("tokenize", code)
                && !parser_stage_owns_diagnostic("normalize", code)
                && !parser_stage_owns_diagnostic("schema", code)
                && !parser_stage_owns_diagnostic("ast", code)
        }
        _ => false,
    }
}

fn is_transform_config_schema(input: &EngineInput, context: &EngineContext) -> bool {
    let identity = effective_input_identity(input, context);
    identity.schema.as_deref() == Some(TRANSFORM_CONFIG_SCHEMA_URI)
}

fn is_schema_package_manifest_schema(input: &EngineInput, context: &EngineContext) -> bool {
    let identity = effective_input_identity(input, context);
    identity.schema.as_deref() == Some(CEM_SCHEMA_PACKAGE_URI)
        || identity
            .content_type
            .as_deref()
            .is_some_and(|content_type| {
                content_type_essence(content_type) == CEM_SCHEMA_PACKAGE_CONTENT_TYPE
            })
}

fn is_cem_native_template_schema(input: &EngineInput, context: &EngineContext) -> bool {
    let identity = effective_input_identity(input, context);
    identity.schema.as_deref() == Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI)
        || identity
            .content_type
            .as_deref()
            .is_some_and(|content_type| {
                content_type_essence(content_type) == CEM_NATIVE_TEMPLATE_CONTENT_TYPE
            })
}

fn input_local_path(input: &EngineInput, context: &EngineContext) -> Option<PathBuf> {
    let uri = input_uri(input, context);
    match parse_local_file_uri(&uri) {
        Some(Ok(path)) => Some(path),
        Some(Err(_)) => None,
        None if !has_uri_scheme(&uri) || is_windows_drive_path(&uri) => Some(PathBuf::from(uri)),
        None => None,
    }
}

fn validate_transform_config_document(
    input: &EngineInput,
    context: &EngineContext,
    bytes: &[u8],
) -> Vec<Diagnostic> {
    let identity = effective_input_identity(input, context);
    match parse_transform_graph_config(TransformGraphParseRequest {
        bytes: bytes.to_vec(),
        identity,
        base_uri: input.root_scope.base_uri.clone(),
    }) {
        Ok(response) => response.diagnostics,
        Err(error) => vec![Diagnostic {
            uri: Some(input.uri.clone()),
            code: error.code.to_owned(),
            severity: Severity::Fatal,
            message: error.message,
            details: None,
            ..Diagnostic::default()
        }],
    }
}

fn read_registered_resource(
    context: Option<&EngineContext>,
    uri: &str,
    purpose: ResolvePurpose,
    content_type_hint: Option<&str>,
) -> Result<Option<ResolvedRead>, ResolverDiagnostic> {
    if !has_uri_scheme(uri) || is_windows_drive_path(uri) {
        return Ok(None);
    }
    let Some(context) = context else {
        return Ok(None);
    };

    let mut request = ResolveRequest::new(uri, purpose, ResolveDirection::Read);
    if let Some(content_type_hint) = content_type_hint {
        request = request.with_content_type_hint(content_type_hint);
    }
    match context.resolver_registry.read(&request) {
        Ok(read) => Ok(Some(read)),
        Err(ResolverDiagnostic::UnsupportedResolver { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

fn materialized_input(input: &EngineInput, context: &EngineContext) -> EngineResult<EngineInput> {
    if !input.bytes.is_empty() {
        return Ok(input.clone());
    }
    let input_path = match parse_local_file_uri(&input.uri) {
        Some(Ok(path)) => path,
        Some(Err(error)) => {
            if let Some(read) = read_registered_resource(
                Some(context),
                &input.uri,
                ResolvePurpose::Input,
                input
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.content_type.as_deref()),
            )
            .map_err(|error| resolver_input_error(input, error))?
            {
                return Ok(resolved_engine_input(input, read));
            }
            return Err(EngineError::Io {
                path: input.uri.clone().into(),
                source: std::io::Error::new(std::io::ErrorKind::InvalidInput, error),
            });
        }
        None if has_uri_scheme(&input.uri) && !is_windows_drive_path(&input.uri) => {
            if let Some(read) = read_registered_resource(
                Some(context),
                &input.uri,
                ResolvePurpose::Input,
                input
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.content_type.as_deref()),
            )
            .map_err(|error| resolver_input_error(input, error))?
            {
                return Ok(resolved_engine_input(input, read));
            }
            return Err(EngineError::Io {
                path: input.uri.clone().into(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "remote/custom input URI resolvers are not implemented",
                ),
            });
        }
        None => PathBuf::from(&input.uri),
    };
    let bytes = std::fs::read(&input_path).map_err(|source| EngineError::Io {
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

fn resolved_engine_input(input: &EngineInput, read: ResolvedRead) -> EngineInput {
    let mut identity = input.identity.clone();
    if let Some(content_type) = read.content_type {
        let mut resolved_identity = identity.unwrap_or_default();
        if resolved_identity.content_type.is_none() {
            resolved_identity.content_type = Some(content_type);
        }
        identity = Some(resolved_identity);
    }
    EngineInput {
        uri: read.uri,
        bytes: read.bytes,
        from_format: input.from_format,
        identity,
        root_scope: input.root_scope.clone(),
    }
}

fn resolver_input_error(input: &EngineInput, error: ResolverDiagnostic) -> EngineError {
    EngineError::Io {
        path: input.uri.clone().into(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, error),
    }
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
    observe_pipeline_with_scope(bytes, from_format, None, observer)
}

/// Run the observable Tier A pipeline while applying root-scope
/// configuration to parser-backed validation and observability workflow
/// budget diagnostics.
pub fn observe_pipeline_scoped(
    bytes: &[u8],
    from_format: InputFormat,
    root_scope: &ScopeConfig,
    observer: &dyn crate::observability::EngineObserver,
) -> PipelineRun {
    observe_pipeline_with_scope(bytes, from_format, Some(root_scope), observer)
}

fn observe_pipeline_with_scope(
    bytes: &[u8],
    from_format: InputFormat,
    root_scope: Option<&ScopeConfig>,
    observer: &dyn crate::observability::EngineObserver,
) -> PipelineRun {
    use crate::events::{EventNormalizer, NormalizedEvent, ScalarValue};
    use crate::observability::{EventEmitter, EventSequencer, ParseEventKind};
    use crate::source_map::TransformKind;

    let started_at = Instant::now();
    let line_index = LineIndex::from_bytes_lossy(bytes);
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
    let mut normalizer_diags: Vec<Diagnostic>;
    {
        match from_format {
            InputFormat::Cem => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let mut tok = CemTokenizer::from_source(src);
                let tok_diags = tok.take_diagnostics();
                let mut normalizer = CemEventNormalizer::new(tok);
                while let Some(event) = normalizer.next_event() {
                    emit_parse_event(&mut emit, &event, &line_index);
                }
                normalizer_diags = tok_diags;
            }
            InputFormat::Html => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let mut tok = HtmlTokenizer::from_source(src);
                let tok_diags = tok.take_diagnostics();
                let mut normalizer = CemEventNormalizer::new(tok);
                while let Some(event) = normalizer.next_event() {
                    emit_parse_event(&mut emit, &event, &line_index);
                }
                normalizer_diags = tok_diags;
            }
            InputFormat::Xml => {
                let src = BytesSource::new(SourceId(1), bytes.to_vec());
                let mut tok = XmlTokenizer::from_source(src);
                let tok_diags = tok.take_diagnostics();
                let mut normalizer = CemEventNormalizer::new(tok);
                while let Some(event) = normalizer.next_event() {
                    emit_parse_event(&mut emit, &event, &line_index);
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

    let mut run = match root_scope {
        Some(root_scope) => run_pipeline_as_scoped(bytes, from_format, root_scope),
        None => run_pipeline_as(bytes, from_format),
    };

    emit.transform(TransformKind::CemAstBuilder, "AST built", None, None);

    let mut budget_diags = root_scope
        .map(|root_scope| {
            time_budget_diagnostics(
                root_scope,
                &["observems", "observetimebudgetms"],
                started_at.elapsed().as_nanos(),
            )
        })
        .unwrap_or_default();
    run.diagnostics.append(&mut budget_diags);
    project_diagnostics_for_source(&mut run.diagnostics, bytes);
    project_diagnostics_for_source(&mut normalizer_diags, bytes);

    // Validate channel — every accumulated diagnostic, plus the
    // normalizer's own diagnostics we collected above (they are also
    // folded into `run.diagnostics` by run_pipeline_as).
    let mut emitted_codes_offsets = std::collections::HashSet::<(String, Option<u64>)>::new();
    for diag in run.diagnostics.iter().chain(normalizer_diags.iter()) {
        let key = (diag.code.clone(), diag.byte_offset);
        if emitted_codes_offsets.insert(key) {
            let coordinate = diag
                .byte_offset
                .map(|byte_offset| line_index.project_host(byte_offset));
            let source_map_coordinates = diag.source_map.as_ref().and_then(|source_map| {
                source_map_coordinate_details(source_map, &line_index, SourceId(1))
            });
            emit.validate_with_coordinates(diag, coordinate, source_map_coordinates);
        }
    }

    fn emit_parse_event(
        emit: &mut EventEmitter<'_>,
        event: &NormalizedEvent,
        line_index: &LineIndex,
    ) {
        match event {
            NormalizedEvent::OpenScope {
                name,
                byte_range,
                source_map,
            } => emit_parse_projected(
                emit,
                ParseEventKind::OpenScope,
                Some(name.lexical_name.clone()),
                None,
                Some(byte_range.start),
                Some(source_map.clone()),
                line_index,
            ),
            NormalizedEvent::CloseScope {
                name,
                byte_range,
                source_map,
                ..
            } => emit_parse_projected(
                emit,
                ParseEventKind::CloseScope,
                Some(name.lexical_name.clone()),
                None,
                Some(byte_range.start),
                Some(source_map.clone()),
                line_index,
            ),
            NormalizedEvent::Name { name, byte_range } => emit_parse_projected(
                emit,
                ParseEventKind::Name,
                Some(name.lexical_name.clone()),
                None,
                Some(byte_range.start),
                None,
                line_index,
            ),
            NormalizedEvent::Value { value, byte_range } => {
                let v = match value {
                    ScalarValue::Text(t) => t.clone(),
                    ScalarValue::Int(i) => i.to_string(),
                    ScalarValue::Float(f) => f.to_string(),
                    ScalarValue::Bool(b) => b.to_string(),
                    ScalarValue::Null => String::new(),
                };
                emit_parse_projected(
                    emit,
                    ParseEventKind::Value,
                    None,
                    Some(v),
                    Some(byte_range.start),
                    None,
                    line_index,
                );
            }
            NormalizedEvent::Trivia {
                kind,
                data,
                byte_range,
            } => emit_parse_projected(
                emit,
                ParseEventKind::Trivia,
                Some(format!("{kind:?}")),
                Some(data.clone()),
                Some(byte_range.start),
                None,
                line_index,
            ),
            NormalizedEvent::Separator { kind, byte_range } => emit_parse_projected(
                emit,
                ParseEventKind::Separator,
                Some(format!("{kind:?}")),
                None,
                Some(byte_range.start),
                None,
                line_index,
            ),
            NormalizedEvent::ModeSwitch {
                content_type,
                handoff,
                ..
            } => emit_parse_projected(
                emit,
                ParseEventKind::ModeSwitch,
                Some(content_type.clone()),
                None,
                Some(handoff.source_span.start),
                None,
                line_index,
            ),
            NormalizedEvent::ProcessingInstruction {
                target,
                data,
                byte_range,
            } => emit_parse_projected(
                emit,
                ParseEventKind::ProcessingInstruction,
                Some(target.clone()),
                Some(data.clone()),
                Some(byte_range.start),
                None,
                line_index,
            ),
            NormalizedEvent::Error {
                code, byte_range, ..
            } => emit_parse_projected(
                emit,
                ParseEventKind::Error,
                Some(code.clone()),
                None,
                Some(byte_range.start),
                None,
                line_index,
            ),
        }
    }

    fn emit_parse_projected(
        emit: &mut EventEmitter<'_>,
        kind: ParseEventKind,
        name: Option<String>,
        value: Option<String>,
        byte_offset: Option<u64>,
        source_map: Option<SourceMapStack>,
        line_index: &LineIndex,
    ) {
        let coordinate = byte_offset.map(|byte_offset| line_index.project_host(byte_offset));
        let source_map_coordinates = source_map.as_ref().and_then(|source_map| {
            source_map_coordinate_details(source_map, line_index, SourceId(1))
        });
        emit.parse_with_coordinates(
            kind,
            name,
            value,
            byte_offset,
            source_map,
            coordinate,
            source_map_coordinates,
        );
    }

    run
}

impl CemMlEngine for RealCemMlEngine {
    fn parse(&self, request: ParseRequest) -> EngineResult<ParseResponse> {
        let loaded = load_input_through_lifecycle(&request.input, &request.context);
        let from_format = loaded.from_format;
        let run = run_pipeline_as_scoped_with_context_and_source_uri(
            &loaded.bytes,
            from_format,
            &request.input.root_scope,
            &request.context,
            &input_uri(&request.input, &request.context),
        );
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
        project_diagnostics_for_source(&mut diagnostics, &loaded.bytes);
        project_diagnostic_uris(&mut diagnostics, &request.input, &request.context);
        Ok(ParseResponse {
            primary,
            diagnostics,
        })
    }

    fn validate(&self, request: ValidateRequest) -> EngineResult<ValidateResponse> {
        let (context, mut all_diags) =
            context_with_loaded_schema_package_manifests(&request.context)?;
        let inputs = input_uris(&request.inputs, &context);
        let (mut validation_diagnostics, scheduler_trace) = run_scheduled_validation_documents(
            &context,
            &request.inputs,
            &["validatems", "validatetimebudgetms"],
        )?;
        all_diags.append(&mut validation_diagnostics);
        let report =
            Report::deterministic(inputs, all_diags, snapshot(request.fail_level, &context))
                .with_scheduler_trace(&scheduler_trace);
        Ok(ValidateResponse { report })
    }

    fn check(&self, request: CheckRequest) -> EngineResult<CheckResponse> {
        let (context, mut all_diags) =
            context_with_loaded_schema_package_manifests(&request.context)?;
        let inputs = input_uris(&request.inputs, &context);
        let (mut validation_diagnostics, scheduler_trace) = run_scheduled_validation_documents(
            &context,
            &request.inputs,
            &["checkms", "checktimebudgetms"],
        )?;
        all_diags.append(&mut validation_diagnostics);
        let report =
            Report::deterministic(inputs, all_diags, snapshot(request.fail_level, &context))
                .with_scheduler_trace(&scheduler_trace);
        let hard_violation_count = report.summary.hard_violation_count;
        Ok(CheckResponse {
            report,
            hard_violation_count,
        })
    }

    fn inspect(&self, request: InspectRequest) -> EngineResult<InspectResponse> {
        let started_at = Instant::now();
        let loaded = load_input_through_lifecycle(&request.input, &request.context);
        let from_format = loaded.from_format;
        let run = run_pipeline_as_scoped_with_context_and_source_uri(
            &loaded.bytes,
            from_format,
            &request.input.root_scope,
            &request.context,
            &input_uri(&request.input, &request.context),
        );
        let mut diagnostics = root_scope_execution_diagnostics(
            &request.input.uri,
            &request.input.root_scope,
            "input",
        );
        diagnostics.extend(loaded.diagnostics);
        diagnostics.extend(run.diagnostics);
        diagnostics.extend(time_budget_diagnostics(
            &request.input.root_scope,
            &["inspectms", "inspecttimebudgetms"],
            started_at.elapsed().as_nanos(),
        ));
        project_diagnostics_for_source(&mut diagnostics, &loaded.bytes);
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

    fn convert(&self, mut request: ConvertRequest) -> EngineResult<ConvertResponse> {
        let started_at = Instant::now();
        if let Some(response) = maybe_convert_generic_data_text(&request, started_at) {
            return Ok(response);
        }
        let (context, mut package_diagnostics) =
            context_with_loaded_schema_package_manifests(&request.context)?;
        request.context = context.clone();
        if let Some(mut response) = request
            .context
            .convert_request_handlers
            .iter()
            .find_map(|handler| handler.maybe_convert(&request))
        {
            if !package_diagnostics.is_empty() {
                let mut diagnostics = package_diagnostics;
                diagnostics.append(&mut response.diagnostics);
                response.diagnostics = diagnostics;
            }
            return Ok(response);
        }
        if let Some(mut response) = maybe_convert_csv_text(&request, started_at) {
            if !package_diagnostics.is_empty() {
                let mut diagnostics = package_diagnostics;
                diagnostics.append(&mut response.diagnostics);
                response.diagnostics = diagnostics;
            }
            return Ok(response);
        }
        let trace = crate::scheduler::SchedulerTrace::new();
        let (policy, mut diagnostics) = scheduler_policy_for_convert(&request);
        diagnostics.append(&mut package_diagnostics);
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
        let mut primary_bytes: Option<PrimaryBytes> = None;
        let mut conversion: Option<ConvertExecutionMetadata> = None;
        let mut source_bytes_for_projection: Option<Vec<u8>> = None;
        pool.run_to_completion(&abort, |task| {
            if task.ends_with(":lifecycle-load") {
                let mut scope_diagnostics = root_scope_metadata_diagnostics(
                    &request.input.uri,
                    &request.input.root_scope,
                    "input",
                );
                diagnostics.append(&mut scope_diagnostics);
                let mut loaded = registry.load(&request.input, &context);
                diagnostics.append(&mut loaded.diagnostics);
                source_bytes_for_projection = Some(loaded.bytes.clone());
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
                .unwrap_or_else(|| registry.load(&request.input, &context));
            diagnostics.append(&mut loaded.diagnostics);
            source_bytes_for_projection = Some(loaded.bytes.clone());
            let mut export = export_selection.take().unwrap_or_else(|| {
                registry.select_export(request.target.as_ref(), request.to_format)
            });
            diagnostics.append(&mut export.diagnostics);
            let to_format = export.to_format;
            let export_conversion =
                resolve_export_conversion_execution(&context, to_format, request.target.as_ref());

            let from_format = loaded.from_format;
            let run = run_pipeline_as_scoped_with_context_and_source_uri(
                &loaded.bytes,
                from_format,
                &request.input.root_scope,
                &context,
                &input_uri(&request.input, &context),
            );
            primary = Some(match to_format {
                LayerFormat::Cem => {
                    let pipeline = cemt_output_pipeline_for_scope(
                        direct_cem_output_pipeline(),
                        &request.target_scope,
                    );
                    conversion = Some(convert_metadata_for_direct_output_pipeline(
                        "direct-cem-output",
                        &pipeline,
                    ));
                    convert_primary_to_cem_with_cemt_pipeline(
                        Some(&context),
                        &run.document,
                        from_format,
                        &request.target_scope,
                        Some(&request.input.uri),
                    )
                    .unwrap_or_else(|mut cemt_diagnostics| {
                        diagnostics.append(&mut cemt_diagnostics);
                        Value::Null
                    })
                }
                LayerFormat::Html => match export_conversion.as_ref() {
                    Some(export_conversion) => {
                        let render_result = render_export_conversion_template(
                            &context,
                            export_conversion,
                            to_format,
                            &run.document,
                            &request.target_scope,
                            &mut diagnostics,
                        );
                        match render_result {
                            ExportConversionTemplateResult::Rendered(primary) => {
                                let pipeline =
                                    export_conversion.output_pipeline.as_ref().map(|pipeline| {
                                        cemt_output_pipeline_for_scope(
                                            pipeline.clone(),
                                            &request.target_scope,
                                        )
                                    });
                                conversion = Some(convert_metadata_for_cemt_converter(
                                    export_conversion,
                                    pipeline.as_ref(),
                                ));
                                primary
                            }
                            ExportConversionTemplateResult::Failed => {
                                conversion = Some(convert_metadata_for_cemt_converter(
                                    export_conversion,
                                    None,
                                ));
                                Value::Null
                            }
                            ExportConversionTemplateResult::UseRustFallback => {
                                conversion =
                                    Some(convert_metadata_for_rust_fallback(export_conversion));
                                let rendered = LightDomInterpreter::new().render(&run.document);
                                let output_spans = rendered
                                    .output_spans
                                    .iter()
                                    .map(|span| {
                                        json!({
                                            "outputRange": span.output_range,
                                            "origin": span.origin,
                                        })
                                    })
                                    .collect::<Vec<_>>();
                                let source_map = rendered.source_map.clone();
                                diagnostics.extend(rendered.diagnostics);
                                json!({
                                    "kind": "html",
                                    "content": rendered.rendered,
                                    "sourceMap": source_map,
                                    "outputSpans": output_spans,
                                })
                            }
                            ExportConversionTemplateResult::UseDirectPipeline => {
                                let pipeline = export_conversion
                                    .output_pipeline
                                    .clone()
                                    .unwrap_or_else(direct_html_output_pipeline);
                                let pipeline =
                                    cemt_output_pipeline_for_scope(pipeline, &request.target_scope);
                                conversion = Some(convert_metadata_for_direct_output_pipeline(
                                    &export_conversion.converter_id,
                                    &pipeline,
                                ));
                                convert_primary_to_markup_with_cemt_pipeline(
                                    Some(&context),
                                    &run.document,
                                    from_format,
                                    export_conversion
                                        .output_pipeline
                                        .clone()
                                        .unwrap_or_else(direct_html_output_pipeline),
                                    "html",
                                    &export_conversion.converter_id,
                                    &request.target_scope,
                                    Some(&request.input.uri),
                                )
                                .unwrap_or_else(
                                    |mut cemt_diagnostics| {
                                        diagnostics.append(&mut cemt_diagnostics);
                                        Value::Null
                                    },
                                )
                            }
                        }
                    }
                    None => {
                        let pipeline = cemt_output_pipeline_for_scope(
                            direct_html_output_pipeline(),
                            &request.target_scope,
                        );
                        conversion = Some(convert_metadata_for_direct_output_pipeline(
                            "direct-html-output",
                            &pipeline,
                        ));
                        convert_primary_to_markup_with_cemt_pipeline(
                            Some(&context),
                            &run.document,
                            from_format,
                            direct_html_output_pipeline(),
                            "html",
                            "direct-html-output",
                            &request.target_scope,
                            Some(&request.input.uri),
                        )
                        .unwrap_or_else(|mut cemt_diagnostics| {
                            diagnostics.append(&mut cemt_diagnostics);
                            Value::Null
                        })
                    }
                },
                LayerFormat::Xml => match export_conversion.as_ref() {
                    Some(export_conversion) => {
                        let render_result = render_export_conversion_template(
                            &context,
                            export_conversion,
                            to_format,
                            &run.document,
                            &request.target_scope,
                            &mut diagnostics,
                        );
                        match render_result {
                            ExportConversionTemplateResult::Rendered(primary) => {
                                let pipeline =
                                    export_conversion.output_pipeline.as_ref().map(|pipeline| {
                                        cemt_output_pipeline_for_scope(
                                            pipeline.clone(),
                                            &request.target_scope,
                                        )
                                    });
                                conversion = Some(convert_metadata_for_cemt_converter(
                                    export_conversion,
                                    pipeline.as_ref(),
                                ));
                                primary
                            }
                            ExportConversionTemplateResult::Failed => {
                                conversion = Some(convert_metadata_for_cemt_converter(
                                    export_conversion,
                                    None,
                                ));
                                Value::Null
                            }
                            ExportConversionTemplateResult::UseRustFallback => {
                                conversion =
                                    Some(convert_metadata_for_rust_fallback(export_conversion));
                                let rendered = XmlInterpreter::new().render(&run.document);
                                let output_spans = rendered
                                    .output_spans
                                    .iter()
                                    .map(|span| {
                                        json!({
                                            "outputRange": span.output_range,
                                            "origin": span.origin,
                                        })
                                    })
                                    .collect::<Vec<_>>();
                                let source_map = rendered.source_map.clone();
                                diagnostics.extend(rendered.diagnostics);
                                json!({
                                    "kind": "xml",
                                    "content": rendered.rendered,
                                    "sourceMap": source_map,
                                    "outputSpans": output_spans,
                                })
                            }
                            ExportConversionTemplateResult::UseDirectPipeline => {
                                let pipeline = export_conversion
                                    .output_pipeline
                                    .clone()
                                    .unwrap_or_else(direct_xml_output_pipeline);
                                let pipeline =
                                    cemt_output_pipeline_for_scope(pipeline, &request.target_scope);
                                conversion = Some(convert_metadata_for_direct_output_pipeline(
                                    &export_conversion.converter_id,
                                    &pipeline,
                                ));
                                convert_primary_to_markup_with_cemt_pipeline(
                                    Some(&context),
                                    &run.document,
                                    from_format,
                                    export_conversion
                                        .output_pipeline
                                        .clone()
                                        .unwrap_or_else(direct_xml_output_pipeline),
                                    "xml",
                                    &export_conversion.converter_id,
                                    &request.target_scope,
                                    Some(&request.input.uri),
                                )
                                .unwrap_or_else(
                                    |mut cemt_diagnostics| {
                                        diagnostics.append(&mut cemt_diagnostics);
                                        Value::Null
                                    },
                                )
                            }
                        }
                    }
                    None => {
                        let pipeline = cemt_output_pipeline_for_scope(
                            direct_xml_output_pipeline(),
                            &request.target_scope,
                        );
                        conversion = Some(convert_metadata_for_direct_output_pipeline(
                            "direct-xml-output",
                            &pipeline,
                        ));
                        convert_primary_to_markup_with_cemt_pipeline(
                            Some(&context),
                            &run.document,
                            from_format,
                            direct_xml_output_pipeline(),
                            "xml",
                            "direct-xml-output",
                            &request.target_scope,
                            Some(&request.input.uri),
                        )
                        .unwrap_or_else(|mut cemt_diagnostics| {
                            diagnostics.append(&mut cemt_diagnostics);
                            Value::Null
                        })
                    }
                },
                LayerFormat::DomJson => projection::dom_json(&run.document),
                LayerFormat::Ast => projection::ast_json(&run.document),
                LayerFormat::Events => projection::events_json_as(&loaded.bytes, from_format),
                LayerFormat::DomBin => {
                    let artifact = projection::dom_binary_projection_artifact(&run.document);
                    primary_bytes = Some(primary_bytes_from_binary_artifact(&artifact));
                    artifact.to_metadata_json()
                }
                LayerFormat::AstBin => {
                    let artifact = projection::ast_binary_projection_artifact(&run.document);
                    primary_bytes = Some(primary_bytes_from_binary_artifact(&artifact));
                    artifact.to_metadata_json()
                }
                LayerFormat::EventsBin => {
                    let artifact = projection::events_binary_projection_artifact_as(
                        &loaded.bytes,
                        from_format,
                    );
                    primary_bytes = Some(primary_bytes_from_binary_artifact(&artifact));
                    artifact.to_metadata_json()
                }
            });
            diagnostics.extend(run.diagnostics);
        });
        let Some(primary) = primary else {
            return Err(EngineError::Internal(
                "scheduler did not dispatch convert task".to_owned(),
            ));
        };
        let elapsed_ns = started_at.elapsed().as_nanos();
        diagnostics.extend(time_budget_diagnostics(
            &request.input.root_scope,
            &["convertms", "converttimebudgetms"],
            elapsed_ns,
        ));
        diagnostics.extend(time_budget_diagnostics(
            &request.target_scope,
            &["convertms", "converttimebudgetms"],
            elapsed_ns,
        ));
        if let Some(source_bytes) = source_bytes_for_projection.as_ref() {
            project_diagnostics_for_source(&mut diagnostics, source_bytes);
        }
        project_diagnostic_uris(&mut diagnostics, &request.input, &context);
        Ok(ConvertResponse {
            primary,
            primary_bytes,
            conversion,
            diagnostics,
            scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
        })
    }

    fn transform(&self, request: TransformRequest) -> EngineResult<TransformResponse> {
        let started_at = Instant::now();
        let trace = crate::scheduler::SchedulerTrace::new();
        let abort = crate::scheduler::AbortSignal::new();
        let mut diagnostics = validate_transform_request_runtime_contract(&request);

        let (data_policy, mut data_scope_diags) = scheduler_policy_for_transform_scope(
            &request.context,
            &request.data.uri,
            &request.data.root_scope,
            "input",
        );
        diagnostics.append(&mut data_scope_diags);
        let (template_policy, mut template_scope_diags) = scheduler_policy_for_transform_scope(
            &request.context,
            &request.template.uri,
            &request.template.root_scope,
            "template",
        );
        diagnostics.append(&mut template_scope_diags);
        let (output_policy, mut output_scope_diags) = scheduler_policy_for_transform_scope(
            &request.context,
            &request.data.uri,
            &request.target_scope,
            "output",
        );
        diagnostics.append(&mut output_scope_diags);

        if has_hard_transform_diagnostic(&diagnostics) {
            return Ok(TransformResponse {
                primary: Value::Null,
                source_map: None,
                output_spans: Vec::new(),
                diagnostics,
                scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
            });
        }

        let data_pool = crate::scheduler::WorkerPool::new(
            request.scheduler_scope_ids.data_load,
            data_policy,
            trace.clone(),
        );
        let template_pool = crate::scheduler::WorkerPool::new(
            request.scheduler_scope_ids.template_load,
            template_policy,
            trace.clone(),
        );
        let execution_pool = crate::scheduler::WorkerPool::new(
            request.scheduler_scope_ids.execution,
            scheduler_policy_from_context(&request.context),
            trace.clone(),
        );
        let output_pool = crate::scheduler::WorkerPool::new(
            request.scheduler_scope_ids.output,
            output_policy,
            trace.clone(),
        );

        for (pool, task) in [
            (&data_pool, format!("{}:data-load", request.data.uri)),
            (
                &template_pool,
                format!("{}:template-compile", request.template.uri),
            ),
            (
                &execution_pool,
                format!("{}:template-execution", request.template.uri),
            ),
            (&output_pool, format!("{}:output", request.data.uri)),
        ] {
            pool.submit(task, &abort).map_err(|err| {
                EngineError::Internal(format!("scheduler dispatch failed: {err}"))
            })?;
        }

        let mut primary_input: Option<TransformTemplateDataArtifact> = None;
        data_pool.run_to_completion(&abort, |_| {
            let (artifact, mut data_diagnostics) =
                load_transform_data_artifact(&request.data, &request.context, "data");
            diagnostics.append(&mut data_diagnostics);
            primary_input = Some(artifact);
        });

        if has_hard_transform_diagnostic(&diagnostics) {
            return Ok(TransformResponse {
                primary: Value::Null,
                source_map: None,
                output_spans: Vec::new(),
                diagnostics,
                scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
            });
        }

        let adapter = select_transform_template_adapter(
            &request.context,
            &request.template,
            &mut diagnostics,
        );

        let mut compiled = None;
        let data_bindings = vec!["input".to_owned()];
        if let Some(adapter) = adapter.as_ref() {
            template_pool.run_to_completion(&abort, |_| {
                compiled = compile_transform_template(
                    TransformTemplateCompileSpec {
                        context: &request.context,
                        adapter,
                        template: &request.template,
                        template_kind: request.template_kind,
                        entrypoint: &request.template_entrypoint,
                        params: &request.params,
                        data_bindings: &data_bindings,
                        module_options: TransformTemplateModuleOptions::default(),
                        execution_policy: request.execution_policy,
                    },
                    &mut diagnostics,
                );
            });
        }

        if has_hard_transform_diagnostic(&diagnostics) {
            return Ok(TransformResponse {
                primary: Value::Null,
                source_map: None,
                output_spans: Vec::new(),
                diagnostics,
                scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
            });
        }

        let Some(adapter) = adapter else {
            return Ok(TransformResponse {
                primary: Value::Null,
                source_map: None,
                output_spans: Vec::new(),
                diagnostics,
                scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
            });
        };
        let Some(compiled) = compiled else {
            return Err(EngineError::Internal(
                "scheduler did not dispatch transform template compile task".to_owned(),
            ));
        };
        let Some(primary_input) = primary_input else {
            return Err(EngineError::Internal(
                "scheduler did not dispatch transform data load task".to_owned(),
            ));
        };

        let secondary_inputs = BTreeMap::new();
        let mut rendered = None;
        execution_pool.run_to_completion(&abort, |_| {
            rendered = render_transform_stage(
                TransformStageRenderSpec {
                    context: &request.context,
                    adapter: &adapter,
                    compiled: &compiled,
                    primary_input: &primary_input,
                    secondary_inputs: &secondary_inputs,
                    target: request.target.as_ref(),
                    target_scope: &request.target_scope,
                    execution_policy: request.execution_policy,
                    diagnostic_uri: &request.template.uri,
                    diagnostic_node: None,
                },
                &mut diagnostics,
            );
        });

        if has_hard_transform_diagnostic(&diagnostics) {
            return Ok(TransformResponse {
                primary: Value::Null,
                source_map: None,
                output_spans: Vec::new(),
                diagnostics,
                scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
            });
        }

        let rendered = rendered.ok_or_else(|| {
            EngineError::Internal(
                "scheduler did not dispatch transform template execution task".to_owned(),
            )
        })?;
        output_pool.run_to_completion(&abort, |_| {});

        let elapsed_ns = started_at.elapsed().as_nanos();
        diagnostics.extend(time_budget_diagnostics(
            &request.data.root_scope,
            &["transformms", "transformtimebudgetms"],
            elapsed_ns,
        ));
        diagnostics.extend(time_budget_diagnostics(
            &request.template.root_scope,
            &["transformms", "transformtimebudgetms"],
            elapsed_ns,
        ));
        diagnostics.extend(time_budget_diagnostics(
            &request.target_scope,
            &["transformms", "transformtimebudgetms"],
            elapsed_ns,
        ));

        Ok(TransformResponse {
            primary: rendered.value,
            source_map: rendered.source_map,
            output_spans: rendered.output_spans,
            diagnostics,
            scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
        })
    }

    fn transform_graph(
        &self,
        request: TransformGraphRequest,
    ) -> EngineResult<TransformGraphResponse> {
        let started_at = Instant::now();
        let trace = crate::scheduler::SchedulerTrace::new();
        let abort = crate::scheduler::AbortSignal::new();
        let mut diagnostics = validate_transform_graph_runtime_contract(&request);
        let mut artifacts: BTreeMap<String, TransformTemplateDataArtifact> = BTreeMap::new();
        let mut artifact_metadata: BTreeMap<String, TransformOutputMetadata> = BTreeMap::new();
        let mut exported = Vec::new();
        let raw_imports = request
            .importmap_rewrites
            .iter()
            .map(|rewrite| rewrite.primary_input.clone())
            .collect::<BTreeSet<_>>();

        for import in &request.imports {
            let (policy, mut scope_diagnostics) = scheduler_policy_for_transform_scope(
                &request.context,
                &import.input.uri,
                &import.input.root_scope,
                "input",
            );
            diagnostics.append(&mut scope_diagnostics);
            let pool =
                crate::scheduler::WorkerPool::new(import.scheduler_scope_id, policy, trace.clone());
            pool.submit(format!("{}:import", import.id), &abort)
                .map_err(|err| {
                    EngineError::Internal(format!("scheduler dispatch failed: {err}"))
                })?;
            pool.run_to_completion(&abort, |_| {
                let artifact = if raw_imports.contains(&import.id) {
                    diagnostics.extend(root_scope_metadata_diagnostics(
                        &import.input.uri,
                        &import.input.root_scope,
                        "input",
                    ));
                    TransformTemplateDataArtifact {
                        artifact_id: import.id.clone(),
                        uri: Some(input_uri(&import.input, &request.context)),
                        identity: import.input.identity.clone(),
                        value: Value::String(
                            String::from_utf8(import.input.bytes.clone()).unwrap_or_default(),
                        ),
                    }
                } else {
                    let (artifact, mut import_diagnostics) =
                        load_transform_data_artifact(&import.input, &request.context, &import.id);
                    diagnostics.append(&mut import_diagnostics);
                    artifact
                };
                artifacts.insert(import.id.clone(), artifact);
                artifact_metadata.insert(
                    import.id.clone(),
                    TransformOutputMetadata {
                        source_map: None,
                        output_spans: Vec::new(),
                        raw_content: String::from_utf8(import.input.bytes.clone()).ok(),
                    },
                );
            });
        }

        if has_hard_transform_diagnostic(&diagnostics) {
            return Ok(TransformGraphResponse {
                artifacts: exported,
                diagnostics,
                scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
            });
        }

        let mut completed_joins = BTreeSet::new();
        let mut completed_stages = BTreeSet::new();
        let mut completed_importmap_rewrites = BTreeSet::new();
        while completed_joins.len() + completed_stages.len() + completed_importmap_rewrites.len()
            < request.joins.len() + request.stages.len() + request.importmap_rewrites.len()
        {
            let mut progressed = false;

            for join in &request.joins {
                if completed_joins.contains(&join.id) {
                    continue;
                }
                if !join
                    .inputs
                    .iter()
                    .all(|input| artifacts.contains_key(&input.artifact_id))
                {
                    continue;
                }

                progressed = true;
                let pool = crate::scheduler::WorkerPool::new(
                    join.scheduler_scope_id,
                    scheduler_policy_from_context(&request.context),
                    trace.clone(),
                );
                pool.submit(format!("{}:join", join.id), &abort)
                    .map_err(|err| {
                        EngineError::Internal(format!("scheduler dispatch failed: {err}"))
                    })?;
                pool.run_to_completion(&abort, |_| {
                    let artifact =
                        collect_transform_graph_join(join, &artifacts, &artifact_metadata);
                    let metadata = collect_transform_graph_join_metadata(join, &artifact_metadata);
                    artifacts.insert(join.id.clone(), artifact);
                    artifact_metadata.insert(join.id.clone(), metadata);
                });
                completed_joins.insert(join.id.clone());
            }

            for rewrite in &request.importmap_rewrites {
                if completed_importmap_rewrites.contains(&rewrite.id) {
                    continue;
                }
                let Some(primary_input) = artifacts.get(&rewrite.primary_input).cloned() else {
                    continue;
                };

                progressed = true;
                let pool = crate::scheduler::WorkerPool::new(
                    rewrite.scheduler_scope_id,
                    scheduler_policy_from_context(&request.context),
                    trace.clone(),
                );
                pool.submit(format!("{}:rewrite-importmap", rewrite.id), &abort)
                    .map_err(|err| {
                        EngineError::Internal(format!("scheduler dispatch failed: {err}"))
                    })?;
                pool.run_to_completion(&abort, |_| {
                    let raw_content = transform_graph_artifact_raw_content(
                        &primary_input,
                        artifact_metadata.get(&rewrite.primary_input),
                    );
                    let Some(raw_content) = raw_content else {
                        diagnostics.push(importmap_rewrite_diagnostic(
                            primary_input.uri.clone(),
                            "cem.importmap.input_content_missing",
                            format!(
                                "rewrite-importmap node `{}` requires a text HTML input artifact",
                                rewrite.id
                            ),
                        ));
                        return;
                    };
                    let (rewritten, mut rewrite_diagnostics) =
                        apply_importmap_rewrite(primary_input.uri.clone(), &raw_content, rewrite);
                    diagnostics.append(&mut rewrite_diagnostics);
                    if let Some(rewritten) = rewritten {
                        artifacts.insert(
                            rewrite.id.clone(),
                            TransformTemplateDataArtifact {
                                artifact_id: rewrite.id.clone(),
                                uri: primary_input.uri.clone(),
                                identity: primary_input.identity.clone(),
                                value: Value::String(rewritten.clone()),
                            },
                        );
                        artifact_metadata.insert(
                            rewrite.id.clone(),
                            TransformOutputMetadata {
                                source_map: None,
                                output_spans: Vec::new(),
                                raw_content: Some(rewritten),
                            },
                        );
                    }
                });
                if has_hard_transform_diagnostic(&diagnostics) {
                    return Ok(TransformGraphResponse {
                        artifacts: exported,
                        diagnostics,
                        scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
                    });
                }
                completed_importmap_rewrites.insert(rewrite.id.clone());
            }

            for stage in &request.stages {
                if completed_stages.contains(&stage.id) {
                    continue;
                }
                let Some(primary_input) = artifacts.get(&stage.primary_input).cloned() else {
                    continue;
                };
                if !stage
                    .secondary_inputs
                    .values()
                    .all(|artifact_id| artifacts.contains_key(artifact_id))
                {
                    continue;
                }

                progressed = true;
                let (template_policy, mut template_scope_diagnostics) =
                    scheduler_policy_for_transform_scope(
                        &request.context,
                        &stage.template.uri,
                        &stage.template.root_scope,
                        "template",
                    );
                diagnostics.append(&mut template_scope_diagnostics);
                let adapter = select_transform_template_adapter(
                    &request.context,
                    &stage.template,
                    &mut diagnostics,
                );
                if has_hard_transform_diagnostic(&diagnostics) {
                    return Ok(TransformGraphResponse {
                        artifacts: exported,
                        diagnostics,
                        scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
                    });
                }
                let Some(adapter) = adapter else {
                    return Ok(TransformGraphResponse {
                        artifacts: exported,
                        diagnostics,
                        scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
                    });
                };

                let template_pool = crate::scheduler::WorkerPool::new(
                    stage.scheduler_scope_ids.template_load,
                    template_policy,
                    trace.clone(),
                );
                template_pool
                    .submit(format!("{}:template-compile", stage.id), &abort)
                    .map_err(|err| {
                        EngineError::Internal(format!("scheduler dispatch failed: {err}"))
                    })?;
                let mut compiled = None;
                let mut data_bindings = vec!["input".to_owned()];
                data_bindings.extend(stage.secondary_inputs.keys().cloned());
                template_pool.run_to_completion(&abort, |_| {
                    compiled = compile_transform_template(
                        TransformTemplateCompileSpec {
                            context: &request.context,
                            adapter: &adapter,
                            template: &stage.template,
                            template_kind: stage.template_kind,
                            entrypoint: &stage.template_entrypoint,
                            params: &stage.params,
                            data_bindings: &data_bindings,
                            module_options: TransformTemplateModuleOptions::default(),
                            execution_policy: stage.execution_policy,
                        },
                        &mut diagnostics,
                    );
                });

                if has_hard_transform_diagnostic(&diagnostics) {
                    return Ok(TransformGraphResponse {
                        artifacts: exported,
                        diagnostics,
                        scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
                    });
                }
                let Some(compiled) = compiled else {
                    return Err(EngineError::Internal(format!(
                        "scheduler did not dispatch transform graph template compile task `{}`",
                        stage.id
                    )));
                };

                let mut secondary_inputs = BTreeMap::new();
                for (name, artifact_id) in &stage.secondary_inputs {
                    if let Some(artifact) = artifacts.get(artifact_id) {
                        secondary_inputs.insert(name.clone(), artifact.clone());
                    }
                }

                let execution_pool = crate::scheduler::WorkerPool::new(
                    stage.scheduler_scope_ids.execution,
                    scheduler_policy_from_context(&request.context),
                    trace.clone(),
                );
                execution_pool
                    .submit(format!("{}:template-execution", stage.id), &abort)
                    .map_err(|err| {
                        EngineError::Internal(format!("scheduler dispatch failed: {err}"))
                    })?;
                let mut rendered = None;
                let diagnostic_node = format!("transform:{}", stage.id);
                execution_pool.run_to_completion(&abort, |_| {
                    rendered = render_transform_stage(
                        TransformStageRenderSpec {
                            context: &request.context,
                            adapter: &adapter,
                            compiled: &compiled,
                            primary_input: &primary_input,
                            secondary_inputs: &secondary_inputs,
                            target: stage.target.as_ref(),
                            target_scope: &ScopeConfig::default(),
                            execution_policy: stage.execution_policy,
                            diagnostic_uri: &stage.template.uri,
                            diagnostic_node: Some(diagnostic_node.as_str()),
                        },
                        &mut diagnostics,
                    );
                });

                if has_hard_transform_diagnostic(&diagnostics) {
                    return Ok(TransformGraphResponse {
                        artifacts: exported,
                        diagnostics,
                        scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
                    });
                }
                let Some(output) = rendered else {
                    return Err(EngineError::Internal(format!(
                        "scheduler did not dispatch transform graph execution task `{}`",
                        stage.id
                    )));
                };
                artifacts.insert(
                    stage.id.clone(),
                    TransformTemplateDataArtifact {
                        artifact_id: stage.id.clone(),
                        uri: output.uri.clone(),
                        identity: output.identity.clone(),
                        value: output.value.clone(),
                    },
                );
                artifact_metadata.insert(
                    stage.id.clone(),
                    TransformOutputMetadata {
                        source_map: output.source_map,
                        output_spans: output.output_spans,
                        raw_content: transform_graph_artifact_raw_content(
                            artifacts.get(&stage.id).expect("stage artifact inserted"),
                            None,
                        ),
                    },
                );
                completed_stages.insert(stage.id.clone());
            }

            if !progressed {
                diagnostics.push(Diagnostic {
                    code: "cem.transform_runtime.graph_order_invalid".to_owned(),
                    severity: Severity::Fatal,
                    message:
                        "transform graph stages could not be ordered from their declared inputs"
                            .to_owned(),
                    details: None,
                    ..Diagnostic::default()
                });
                return Ok(TransformGraphResponse {
                    artifacts: exported,
                    diagnostics,
                    scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
                });
            }
        }

        for export in &request.exports {
            let (policy, mut output_scope_diagnostics) = scheduler_policy_for_transform_scope(
                &request.context,
                export.destination.as_deref().unwrap_or(&export.id),
                &export.target_scope,
                "output",
            );
            diagnostics.append(&mut output_scope_diagnostics);
            let pool =
                crate::scheduler::WorkerPool::new(export.scheduler_scope_id, policy, trace.clone());
            pool.submit(format!("{}:export", export.id), &abort)
                .map_err(|err| {
                    EngineError::Internal(format!("scheduler dispatch failed: {err}"))
                })?;
            pool.run_to_completion(&abort, |_| {
                if let Some(artifact) = artifacts.get(&export.input) {
                    let metadata = artifact_metadata
                        .get(&export.input)
                        .cloned()
                        .unwrap_or_default();
                    let identity = export.target.clone().or_else(|| artifact.identity.clone());
                    let style_projection =
                        transform_graph_html_style_projection_for_export(export, &request.exports);
                    let (primary, source_map, output_spans) = transform_graph_export_primary(
                        artifact,
                        &metadata,
                        identity.as_ref(),
                        style_projection,
                    );
                    exported.push(TransformGraphArtifact {
                        export_id: export.id.clone(),
                        input: export.input.clone(),
                        destination: export.destination.clone(),
                        identity,
                        primary,
                        source_map,
                        output_spans,
                    });
                }
            });
        }

        let elapsed_ns = started_at.elapsed().as_nanos();
        for import in &request.imports {
            diagnostics.extend(time_budget_diagnostics(
                &import.input.root_scope,
                &["transformms", "transformtimebudgetms"],
                elapsed_ns,
            ));
        }
        for stage in &request.stages {
            diagnostics.extend(time_budget_diagnostics(
                &stage.template.root_scope,
                &["transformms", "transformtimebudgetms"],
                elapsed_ns,
            ));
        }
        for export in &request.exports {
            diagnostics.extend(time_budget_diagnostics(
                &export.target_scope,
                &["transformms", "transformtimebudgetms"],
                elapsed_ns,
            ));
        }

        Ok(TransformGraphResponse {
            artifacts: exported,
            diagnostics,
            scheduler_trace: crate::report::SchedulerTraceReport::from_trace(&trace),
        })
    }

    fn trace(&self, request: TraceRequest) -> EngineResult<TraceResponse> {
        let started_at = Instant::now();
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
        let run = run_pipeline_as_scoped_with_context_and_source_uri(
            &loaded.bytes,
            from_format,
            &request.input.root_scope,
            &request.context,
            &input_uri(&request.input, &request.context),
        );
        pool.run_to_completion(&abort, |_| {});
        let mut diagnostics = policy_diagnostics;
        diagnostics.extend(root_scope_metadata_diagnostics(
            &request.input.uri,
            &request.input.root_scope,
            "input",
        ));
        diagnostics.extend(loaded.diagnostics);
        diagnostics.extend(run.diagnostics);
        diagnostics.extend(time_budget_diagnostics(
            &request.input.root_scope,
            &["tracems", "tracetimebudgetms"],
            started_at.elapsed().as_nanos(),
        ));
        project_diagnostics_for_source(&mut diagnostics, &loaded.bytes);
        project_diagnostic_uris(&mut diagnostics, &request.input, &request.context);
        let parser_stages = parser_stage_report_for_input(
            &request.input,
            &request.context,
            from_format,
            loaded.bytes.len(),
            &diagnostics,
        );
        let mut report = Report::deterministic(
            vec![input_uri(&request.input, &request.context)],
            diagnostics,
            snapshot(FailLevel::Validate, &request.context),
        )
        .with_scheduler_trace(&scheduler_trace);
        report.report_ast.parser_stages = Some(parser_stages);
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
        let mut diagnostics: Vec<Diagnostic> = Vec::new();
        let mut budget_exceeded = false;
        for _ in 0..iterations {
            let t = Instant::now();
            for input in &request.inputs {
                let input = materialized_input(input, &request.context)?;
                let input_started_at = Instant::now();
                let loaded = load_input_through_lifecycle(&input, &request.context);
                let _ = run_pipeline_as_scoped_with_context_and_source_uri(
                    &loaded.bytes,
                    loaded.from_format,
                    &input.root_scope,
                    &request.context,
                    &input_uri(&input, &request.context),
                );
                let mut budget_diags = time_budget_diagnostics(
                    &input.root_scope,
                    &["benchms", "benchtimebudgetms"],
                    input_started_at.elapsed().as_nanos(),
                );
                if !budget_diags.is_empty() {
                    budget_exceeded = true;
                    project_diagnostic_uris(&mut budget_diags, &input, &request.context);
                    diagnostics.extend(budget_diags);
                }
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
            "diagnostics": diagnostics,
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
            let input = materialized_input(input, &request.context)?;
            let started_at = Instant::now();
            let mut input_diags =
                root_scope_execution_diagnostics(&input.uri, &input.root_scope, "input");
            let loaded = load_input_through_lifecycle(&input, &request.context);
            input_diags.extend(loaded.diagnostics);
            let run = run_pipeline_as_scoped_with_context_and_source_uri(
                &loaded.bytes,
                loaded.from_format,
                &input.root_scope,
                &request.context,
                &input_uri(&input, &request.context),
            );
            input_diags.extend(run.diagnostics);
            input_diags.extend(time_budget_diagnostics(
                &input.root_scope,
                &["fixturevalidatems", "fixturevalidatetimebudgetms"],
                started_at.elapsed().as_nanos(),
            ));
            project_diagnostics_for_source(&mut input_diags, &loaded.bytes);
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
            let input = materialized_input(input, &request.context)?;
            let started_at = Instant::now();
            let mut input_diags =
                root_scope_execution_diagnostics(&input.uri, &input.root_scope, "input");
            let loaded = load_input_through_lifecycle(&input, &request.context);
            input_diags.extend(loaded.diagnostics);
            let run = run_pipeline_as_scoped_with_context_and_source_uri(
                &loaded.bytes,
                loaded.from_format,
                &input.root_scope,
                &request.context,
                &input_uri(&input, &request.context),
            );
            let rendered = LightDomInterpreter::new().render(&run.document);
            artifacts.push(json!({
                "input": input_uri(&input, &request.context),
                "toFormat": request.to_format,
                "rendered": rendered.rendered,
            }));
            input_diags.extend(run.diagnostics);
            input_diags.extend(time_budget_diagnostics(
                &input.root_scope,
                &["fixtureroundtripms", "fixtureroundtriptimebudgetms"],
                started_at.elapsed().as_nanos(),
            ));
            project_diagnostics_for_source(&mut input_diags, &loaded.bytes);
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
    use crate::conversion::{
        ConversionDescriptor, ConversionEndpoint, ConversionImplementation,
        ConversionPackageArtifactDescriptor, ConversionReadiness, ConversionRegistry,
        ConversionRustFallbackDescriptor, ConversionTemplateDescriptor,
    };
    use crate::resolver::{
        ResolvePolicyRequestKey, ResolvePolicySubstitution, ResolverPolicy, ResolverRegistry,
        ResourceResolver,
    };
    use crate::schema::registry::{
        CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, CEM_SCHEMA_CONTENT_TYPE,
        CEM_TRANSFORM_CONTENT_TYPE, CEM_TRANSFORM_SCHEMA_URI,
    };
    use crate::transform_template::{
        TransformTemplateAdapterCapability, TransformTemplateAdapterError,
        TransformTemplateAdapterExecutionPhase, TransformTemplateAdapterRegistry,
        TransformTemplateAdapterResult, TransformTemplateCompileResponse,
        TransformTemplateRenderResponse,
    };

    #[derive(Debug)]
    struct StaticReadResolver {
        resolved_uri: &'static str,
        bytes: &'static [u8],
        content_type: Option<&'static str>,
    }

    impl ResourceResolver for StaticReadResolver {
        fn read(&self, _request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            Ok(ResolvedRead {
                uri: self.resolved_uri.to_owned(),
                bytes: self.bytes.to_vec(),
                content_type: self.content_type.map(str::to_owned),
            })
        }

        fn write(
            &self,
            request: &ResolveRequest,
            _bytes: &[u8],
        ) -> Result<crate::resolver::ResolvedWrite, ResolverDiagnostic> {
            Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Write,
            })
        }
    }

    #[derive(Debug)]
    struct MapReadResolver {
        entries: Vec<(&'static str, &'static [u8], Option<&'static str>)>,
    }

    impl ResourceResolver for MapReadResolver {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            let uri = resolve_test_uri(request);
            let Some((resolved_uri, bytes, content_type)) = self
                .entries
                .iter()
                .find(|(entry_uri, _, _)| *entry_uri == uri)
            else {
                return Err(ResolverDiagnostic::UnsupportedResolver {
                    uri: request.uri.clone(),
                    purpose: request.purpose,
                    direction: ResolveDirection::Read,
                });
            };

            Ok(ResolvedRead {
                uri: (*resolved_uri).to_owned(),
                bytes: bytes.to_vec(),
                content_type: content_type.map(str::to_owned),
            })
        }

        fn write(
            &self,
            request: &ResolveRequest,
            _bytes: &[u8],
        ) -> Result<crate::resolver::ResolvedWrite, ResolverDiagnostic> {
            Err(ResolverDiagnostic::UnsupportedResolver {
                uri: request.uri.clone(),
                purpose: request.purpose,
                direction: ResolveDirection::Write,
            })
        }
    }

    fn resolve_test_uri(request: &ResolveRequest) -> String {
        if has_uri_scheme(&request.uri) {
            return request.uri.clone();
        }
        request
            .base_uri
            .as_deref()
            .and_then(|base| {
                base.rsplit_once('/')
                    .map(|(dir, _)| format!("{dir}/{}", request.uri))
            })
            .unwrap_or_else(|| request.uri.clone())
    }

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

    fn test_source_map_stack(start: u64, len: u32) -> SourceMapStack {
        SourceMapStack {
            frames: vec![crate::source_map::SourceMapFrame {
                source_id: SourceId(1),
                span: crate::source_map::FrameSpan::Single(ByteRange::new(start, len)),
                transform: crate::source_map::TransformKind::InterpreterRender,
            }],
        }
    }

    fn test_output_span(output_start: usize, len: usize, origin_start: u64) -> OutputSpan {
        OutputSpan {
            output_range: ByteRange::new(output_start as u64, len as u32),
            origin: test_source_map_stack(origin_start, len as u32),
        }
    }

    fn context_with_resolver(
        scheme: &str,
        purpose: ResolvePurpose,
        resolver: impl ResourceResolver + 'static,
    ) -> EngineContext {
        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(scheme, purpose, ResolveDirection::Read, resolver);
        EngineContext {
            resolver_registry,
            ..ctx()
        }
    }

    #[test]
    fn transform_graph_export_projects_inline_styles_to_css_document() {
        let raw = "<HTML><head><STYLE>.card { color: red; }</STYLE><style media=\"screen\">
.grid { display: grid; }
</style></head><body>Hi</body></HTML>";
        let artifact = TransformTemplateDataArtifact {
            artifact_id: "page".to_owned(),
            uri: Some("page.html".to_owned()),
            identity: Some(FormatIdentity {
                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                schema: Some(HTML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            value: json!({
                "kind": "html",
                "content": raw,
            }),
        };
        let metadata = TransformOutputMetadata {
            source_map: None,
            output_spans: Vec::new(),
            raw_content: Some(raw.to_owned()),
        };
        let target = FormatIdentity {
            content_type: Some("text/css; charset=utf-8".to_owned()),
            schema: Some(CSS_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let (primary, source_map, output_spans) = transform_graph_export_primary(
            &artifact,
            &metadata,
            Some(&target),
            TransformGraphHtmlStyleProjection::Inline,
        );

        assert_eq!(primary["kind"], "document");
        assert_eq!(
            primary["content"],
            ".card { color: red; }\n\n.grid { display: grid; }\n"
        );
        assert!(source_map.is_none());
        assert!(output_spans.is_empty());
    }

    #[test]
    fn transform_graph_export_rebases_inline_style_spans_to_css_document() {
        let raw = "<html><head><style>
  .card { color: red; }
</style><style> .grid { display: grid; } </style></head><body>Hi</body></html>";
        let first_css = ".card { color: red; }";
        let second_css = ".grid { display: grid; }";
        let first_start = raw.find(first_css).unwrap();
        let second_start = raw.find(second_css).unwrap();
        let source_map = test_source_map_stack(0, raw.len() as u32);
        let artifact = TransformTemplateDataArtifact {
            artifact_id: "page".to_owned(),
            uri: Some("page.html".to_owned()),
            identity: Some(FormatIdentity {
                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                schema: Some(HTML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            value: json!({
                "kind": "html",
                "content": raw,
            }),
        };
        let first_span = test_output_span(first_start, first_css.len(), 100);
        let second_span = test_output_span(second_start, second_css.len(), 200);
        let metadata = TransformOutputMetadata {
            source_map: Some(source_map.clone()),
            output_spans: vec![first_span.clone(), second_span.clone()],
            raw_content: Some(raw.to_owned()),
        };
        let target = FormatIdentity {
            content_type: Some(CSS_CONTENT_TYPE.to_owned()),
            schema: Some(CSS_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let (primary, exported_source_map, output_spans) = transform_graph_export_primary(
            &artifact,
            &metadata,
            Some(&target),
            TransformGraphHtmlStyleProjection::Inline,
        );

        assert_eq!(primary["kind"], "document");
        assert_eq!(
            primary["content"],
            ".card { color: red; }\n\n.grid { display: grid; }\n"
        );
        assert_eq!(exported_source_map, Some(source_map));
        assert_eq!(output_spans.len(), 2);
        assert_eq!(
            output_spans[0].output_range,
            ByteRange::new(0, first_css.len() as u32)
        );
        assert_eq!(output_spans[0].origin, first_span.origin);
        assert_eq!(
            output_spans[1].output_range,
            ByteRange::new((first_css.len() + 2) as u64, second_css.len() as u32)
        );
        assert_eq!(output_spans[1].origin, second_span.origin);
    }

    #[test]
    fn transform_graph_export_replaces_inline_styles_with_stylesheet_link() {
        let raw = r#"<html><head><style>.card { color: red; }</style><style>.grid { display: grid; }</style></head><body>Hi</body></html>"#;
        let artifact = TransformTemplateDataArtifact {
            artifact_id: "page".to_owned(),
            uri: Some("page.html".to_owned()),
            identity: Some(FormatIdentity {
                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                schema: Some(HTML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            value: json!({
                "kind": "html",
                "content": raw,
            }),
        };
        let metadata = TransformOutputMetadata {
            source_map: None,
            output_spans: Vec::new(),
            raw_content: Some(raw.to_owned()),
        };
        let target = FormatIdentity {
            content_type: Some(HTML_CONTENT_TYPE.to_owned()),
            schema: Some(HTML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let (primary, source_map, output_spans) = transform_graph_export_primary(
            &artifact,
            &metadata,
            Some(&target),
            TransformGraphHtmlStyleProjection::Link(
                "assets/page.css?mode=screen&theme=\"dark\"".to_owned(),
            ),
        );

        assert_eq!(primary["kind"], "html");
        assert_eq!(
            primary["content"],
            r#"<html><head><link rel="stylesheet" href="assets/page.css?mode=screen&amp;theme=&quot;dark&quot;"></head><body>Hi</body></html>"#
        );
        assert!(source_map.is_none());
        assert!(output_spans.is_empty());
    }

    #[test]
    fn transform_graph_export_rebases_linked_html_style_spans() {
        let raw = "<html><head><style>.card { color: red; }</style></head><body>Hi</body></html>";
        let before = "<html><head>";
        let after = "</head><body>Hi</body></html>";
        let replacement = r#"<link rel="stylesheet" href="page.css">"#;
        let after_start = raw.find(after).unwrap();
        let source_map = test_source_map_stack(0, raw.len() as u32);
        let artifact = TransformTemplateDataArtifact {
            artifact_id: "page".to_owned(),
            uri: Some("page.html".to_owned()),
            identity: Some(FormatIdentity {
                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                schema: Some(HTML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            value: json!({
                "kind": "html",
                "content": raw,
            }),
        };
        let before_span = test_output_span(0, before.len(), 100);
        let style_span =
            test_output_span(before.len(), raw.len() - before.len() - after.len(), 200);
        let after_span = test_output_span(after_start, after.len(), 300);
        let metadata = TransformOutputMetadata {
            source_map: Some(source_map.clone()),
            output_spans: vec![before_span.clone(), style_span, after_span.clone()],
            raw_content: Some(raw.to_owned()),
        };
        let target = FormatIdentity {
            content_type: Some(HTML_CONTENT_TYPE.to_owned()),
            schema: Some(HTML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let (primary, exported_source_map, output_spans) = transform_graph_export_primary(
            &artifact,
            &metadata,
            Some(&target),
            TransformGraphHtmlStyleProjection::Link("page.css".to_owned()),
        );

        assert_eq!(primary["kind"], "html");
        assert_eq!(primary["content"], format!("{before}{replacement}{after}"));
        assert_eq!(exported_source_map, Some(source_map));
        assert_eq!(output_spans.len(), 2);
        assert_eq!(
            output_spans[0].output_range,
            ByteRange::new(0, before.len() as u32)
        );
        assert_eq!(output_spans[0].origin, before_span.origin);
        assert_eq!(
            output_spans[1].output_range,
            ByteRange::new(
                (before.len() + replacement.len()) as u64,
                after.len() as u32
            )
        );
        assert_eq!(output_spans[1].origin, after_span.origin);
    }

    #[test]
    fn transform_graph_export_rebases_omitted_html_style_spans() {
        let raw = "<html><head><style>.card { color: red; }</style></head><body>Hi</body></html>";
        let before = "<html><head>";
        let after = "</head><body>Hi</body></html>";
        let after_start = raw.find(after).unwrap();
        let source_map = test_source_map_stack(0, raw.len() as u32);
        let artifact = TransformTemplateDataArtifact {
            artifact_id: "page".to_owned(),
            uri: Some("page.html".to_owned()),
            identity: Some(FormatIdentity {
                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                schema: Some(HTML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            value: json!({
                "kind": "html",
                "content": raw,
            }),
        };
        let before_span = test_output_span(0, before.len(), 100);
        let style_span =
            test_output_span(before.len(), raw.len() - before.len() - after.len(), 200);
        let after_span = test_output_span(after_start, after.len(), 300);
        let metadata = TransformOutputMetadata {
            source_map: Some(source_map.clone()),
            output_spans: vec![before_span.clone(), style_span, after_span.clone()],
            raw_content: Some(raw.to_owned()),
        };
        let target = FormatIdentity {
            content_type: Some(HTML_CONTENT_TYPE.to_owned()),
            schema: Some(HTML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let (primary, exported_source_map, output_spans) = transform_graph_export_primary(
            &artifact,
            &metadata,
            Some(&target),
            TransformGraphHtmlStyleProjection::Omit,
        );

        assert_eq!(primary["kind"], "html");
        assert_eq!(primary["content"], format!("{before}{after}"));
        assert_eq!(exported_source_map, Some(source_map));
        assert_eq!(output_spans.len(), 2);
        assert_eq!(
            output_spans[0].output_range,
            ByteRange::new(0, before.len() as u32)
        );
        assert_eq!(output_spans[0].origin, before_span.origin);
        assert_eq!(
            output_spans[1].output_range,
            ByteRange::new(before.len() as u64, after.len() as u32)
        );
        assert_eq!(output_spans[1].origin, after_span.origin);
    }

    #[test]
    fn transform_graph_export_omits_inline_styles_without_link() {
        let raw =
            r#"<html><head><style>.card { color: red; }</style></head><body>Hi</body></html>"#;
        let artifact = TransformTemplateDataArtifact {
            artifact_id: "page".to_owned(),
            uri: Some("page.html".to_owned()),
            identity: Some(FormatIdentity {
                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                schema: Some(HTML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            value: json!({
                "kind": "html",
                "content": raw,
            }),
        };
        let metadata = TransformOutputMetadata {
            source_map: None,
            output_spans: Vec::new(),
            raw_content: Some(raw.to_owned()),
        };
        let target = FormatIdentity {
            content_type: Some(HTML_CONTENT_TYPE.to_owned()),
            schema: Some(HTML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let (primary, source_map, output_spans) = transform_graph_export_primary(
            &artifact,
            &metadata,
            Some(&target),
            TransformGraphHtmlStyleProjection::Omit,
        );

        assert_eq!(primary["kind"], "html");
        assert_eq!(
            primary["content"],
            r#"<html><head></head><body>Hi</body></html>"#
        );
        assert!(source_map.is_none());
        assert!(output_spans.is_empty());
    }

    fn ready_cemt_html_export_context(
        template_uri: &'static str,
        template_bytes: &'static [u8],
    ) -> EngineContext {
        let mut converter_registry = ConversionRegistry::with_builtin_converters();
        converter_registry
            .register(ConversionDescriptor {
                id: "test-dom-to-html-cemt-ready".to_owned(),
                package_id: "test-dom-projection".to_owned(),
                from: ConversionEndpoint::with_schema(
                    CEM_DOM_PROJECTION_CONTENT_TYPE,
                    CEM_DOM_PROJECTION_SCHEMA_URI,
                ),
                to: ConversionEndpoint::with_schema(HTML_CONTENT_TYPE, HTML_SCHEMA_URI),
                implementation: ConversionImplementation::Cemt,
                readiness: ConversionReadiness::Ready,
                template: Some(ConversionTemplateDescriptor {
                    path: template_uri.to_owned(),
                    content_type: CEM_TRANSFORM_CONTENT_TYPE.to_owned(),
                    schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                    entrypoint: Some("main".to_owned()),
                }),
                rust_symbol: None,
                rust_fallback: Some(ConversionRustFallbackDescriptor {
                    rust_symbol: "HtmlExportConverter".to_owned(),
                    reason: "test fallback".to_owned(),
                }),
                streamable: true,
                lossiness: Some("serialization".to_owned()),
                output_contract: crate::conversion::ConversionOutputContractDescriptor {
                    output_syntax: Some(crate::conversion::ConversionOutputSyntax::Html),
                    encoding_category: Some("html-document".to_owned()),
                    formatter_profile: Some("compact".to_owned()),
                    color_profile: Some("classes".to_owned()),
                    parity: None,
                },
                parity_fixtures: Vec::new(),
                implicit: true,
                explicit_only: false,
                cost: 1,
            })
            .unwrap();

        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(
            "cem+test",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            StaticReadResolver {
                resolved_uri: template_uri,
                bytes: template_bytes,
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE),
            },
        );

        let mut template_adapter_registry =
            TransformTemplateAdapterRegistry::with_builtin_adapters();
        template_adapter_registry.register(ReadyCemtHtmlExportAdapter);

        EngineContext {
            converter_registry,
            resolver_registry,
            template_adapter_registry,
            ..ctx()
        }
    }

    #[derive(Clone)]
    struct ReadyCemtHtmlExportAdapter;

    impl TransformTemplateAdapter for ReadyCemtHtmlExportAdapter {
        fn id(&self) -> &'static str {
            "ready-cemt-html-export-test"
        }

        fn kind(&self) -> TransformTemplateKind {
            TransformTemplateKind::CemNative
        }

        fn capability(&self) -> TransformTemplateAdapterCapability {
            TransformTemplateAdapterCapability::Executable
        }

        fn matches_template(&self, identity: &FormatIdentity) -> bool {
            identity
                .content_type
                .as_deref()
                .is_some_and(|content_type| {
                    content_type
                        .split(';')
                        .next()
                        .unwrap_or(content_type)
                        .trim()
                        == CEM_TRANSFORM_CONTENT_TYPE
                })
                || identity
                    .schema
                    .as_deref()
                    .is_some_and(|schema| schema == CEM_TRANSFORM_SCHEMA_URI)
        }

        fn compile(
            &self,
            request: TransformTemplateCompileRequest<'_>,
        ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
            let template_text = std::str::from_utf8(&request.template.bytes).unwrap_or_default();
            if template_text.contains("adapter-compile-fail") {
                return Err(TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Compile,
                    "adapter compile sentinel",
                ));
            }

            Ok(TransformTemplateCompileResponse {
                artifact: TransformTemplateCompiledArtifact::new(
                    self.id(),
                    self.kind(),
                    request.template.uri.clone(),
                    request.template.identity.clone(),
                    request.entrypoint.clone(),
                    json!({
                        "templateBytes": request.template.bytes.len(),
                        "templateText": template_text,
                    }),
                ),
                diagnostics: Vec::new(),
            })
        }

        fn render(
            &self,
            request: TransformTemplateRenderRequest<'_>,
        ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
            let template_text = request
                .compiled
                .opaque
                .get("templateText")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if template_text.contains("adapter-render-fail") {
                return Err(TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Render,
                    "adapter render sentinel",
                ));
            }

            if request.target.is_some_and(|target| {
                target.content_type.as_deref().is_some_and(|content_type| {
                    content_type_essence(content_type)
                        == crate::schema::registry::CEM_ML_CONTENT_TYPE
                })
            }) {
                if template_text.contains("adapter-output-pipeline-fail") {
                    return Ok(TransformTemplateRenderResponse {
                        output: TransformTemplateOutputArtifact {
                            uri: None,
                            identity: request.target.cloned(),
                            value: Value::String("not a CEM tree".to_owned()),
                            source_map: Some(test_source_map_stack(10, 8)),
                            output_spans: vec![test_output_span(0, 8, 10)],
                        },
                        diagnostics: Vec::new(),
                    });
                }

                return Ok(TransformTemplateRenderResponse {
                    output: TransformTemplateOutputArtifact {
                        uri: None,
                        identity: request.target.cloned(),
                        value: json!([{
                            "kind": "element",
                            "name": "cemt-ready",
                            "children": [{
                                "kind": "text",
                                "value": request.primary_input.value["kind"]
                                    .as_str()
                                    .unwrap_or("unknown")
                            }]
                        }]),
                        source_map: Some(test_source_map_stack(10, 8)),
                        output_spans: vec![test_output_span(0, 8, 10)],
                    },
                    diagnostics: Vec::new(),
                });
            }

            Ok(TransformTemplateRenderResponse {
                output: TransformTemplateOutputArtifact {
                    uri: None,
                    identity: request.target.cloned(),
                    value: json!({
                        "kind": "html",
                        "content": format!(
                            "<direct-ready>{}</direct-ready>",
                            request.primary_input.value["kind"].as_str().unwrap_or("unknown")
                        ),
                    }),
                    source_map: None,
                    output_spans: Vec::new(),
                },
                diagnostics: Vec::new(),
            })
        }
    }

    fn template(uri: &str, bytes: &[u8]) -> TemplateInput {
        TemplateInput {
            uri: uri.to_owned(),
            bytes: bytes.to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some("text/cem-ml".to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        }
    }

    #[test]
    fn template_module_preflight_reads_relative_imports_through_template_resolver() {
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            StaticReadResolver {
                resolved_uri: "cem+vfs://templates/ui.cem",
                bytes: b"{template @name=\"card\"}",
                content_type: Some("text/cem-ml"),
            },
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        )
        .expect("preflight should resolve import");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(preflight.resolved_imports.len(), 1);
        assert_eq!(preflight.resolved_imports[0].alias, "ui");
        assert_eq!(
            preflight.resolved_imports[0].uri,
            "cem+vfs://templates/ui.cem"
        );
        assert_eq!(
            preflight.resolved_imports[0]
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/cem-ml")
        );
        assert!(preflight.resolved_imports[0]
            .content_hash
            .starts_with("cem-bin/1+blake3:"));
        assert!(preflight
            .cache_key
            .expect("cache key")
            .dependency_graph_hash
            .starts_with("cem-bin/1+blake3:"));
    }

    #[test]
    fn template_module_preflight_reads_nested_imports_with_parent_scope() {
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            MapReadResolver {
                entries: vec![
                    (
                        "cem+vfs://templates/ui.cem",
                        br#"{@doc cem-ml 1}
{module |
  {import @as="icons" @src="icons.cem"}
  {template @name="card" @visibility="public" | {body | {call @from="icons" @template="check"}}}
}"#,
                        Some("text/cem-ml"),
                    ),
                    (
                        "cem+vfs://templates/icons.cem",
                        br#"{@doc cem-ml 1}
{module |
  {template @name="check" @visibility="public" | {body | {span | Check}}}
}"#,
                        Some("text/cem-ml"),
                    ),
                ],
            },
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: None,
                from: Some("ui".to_owned()),
                template: "card".to_owned(),
                arguments: BTreeMap::new(),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        )
        .expect("preflight should resolve nested imports");
        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &preflight,
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(preflight.resolved_imports.len(), 2);
        assert_eq!(preflight.resolved_imports[0].alias, "ui");
        assert_eq!(preflight.resolved_imports[0].parent_uri, None);
        assert_eq!(preflight.resolved_imports[1].alias, "icons");
        assert_eq!(
            preflight.resolved_imports[1].parent_uri.as_deref(),
            Some("cem+vfs://templates/ui.cem")
        );
        assert_eq!(
            preflight.resolved_imports[1].uri,
            "cem+vfs://templates/icons.cem"
        );
        assert!(preflight
            .cache_key
            .expect("cache key")
            .dependency_graph_hash
            .starts_with("cem-bin/1+blake3:"));
    }

    #[test]
    fn template_module_options_are_lowered_from_native_template_declarations() {
        let template = TemplateInput {
            uri: "templates/page.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {import @as="ui" @src="ui.cem" @content-type="text/cem-ml"}
  {template @name="card" @visibility="public" | {body | {let @name="layout" @type="string" @value="block"}}}
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                schema: Some(crate::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let mut diagnostics = Vec::new();

        let options = lower_transform_template_module_options(
            &template,
            TransformTemplateModuleOptions::default(),
            &mut diagnostics,
        )
        .expect("module declarations should lower");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(options.imports.len(), 1);
        assert_eq!(options.imports[0].alias, "ui");
        assert_eq!(options.imports[0].uri, "ui.cem");
        assert_eq!(options.entrypoints.len(), 1);
        assert_eq!(options.entrypoints[0].name, "card");
        assert_eq!(options.let_bindings.len(), 1);
        assert_eq!(
            options.let_bindings[0].owner_entrypoint.as_deref(),
            Some("card")
        );
        assert_eq!(options.let_bindings[0].name, "layout");
        assert_eq!(options.let_bindings[0].value, json!("block"));
    }

    #[test]
    fn template_module_options_preserve_overlay_call_sites() {
        let template = TemplateInput {
            uri: "templates/page.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" | {body | {span | Card}}}
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                schema: Some(crate::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let overlay = TransformTemplateModuleOptions {
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: None,
                template: "missing".to_owned(),
                arguments: BTreeMap::new(),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let options = lower_transform_template_module_options(&template, overlay, &mut diagnostics)
            .expect("module declarations should lower");
        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &TransformTemplateModulePreflight::default(),
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert_eq!(options.calls.len(), 1);
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE));
    }

    #[test]
    fn compile_transform_template_preserves_lowered_encode_metadata_for_render() {
        let template = TemplateInput {
            uri: "templates/page.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="html.text"
      @category="html-text"
      @subject="string"
      @produces="text"
      @content-type="text/html"
      @schema="https://cem.dev/ns/data/html/1"
      @canonical=true
      @streamable=true
      @deterministic=true |
      {param @name="subject" @type="string" @required=true}
  }
  {template @name="main" @visibility="public" |
    {body |
      {let @name="encoderName" @value="html.text"}
      {$ encode($input.title, { contentType: "text/html", schema: "https://cem.dev/ns/data/html/1", category: "html-text", context: "text" }, { mode: "fragment", encoder: $encoderName }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();

        let compiled = match compile_transform_template(
            TransformTemplateCompileSpec {
                context: &ctx(),
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        ) {
            Some(compiled) => compiled,
            None => panic!("template should compile: {diagnostics:?}"),
        };

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(compiled.module_options.encode_expressions.len(), 1);
        assert_eq!(compiled.module_options.output_functions.len(), 1);
        let encode = &compiled.module_options.encode_expressions[0];
        assert_eq!(encode.owner.as_deref(), Some("main"));
        assert_eq!(encode.subject, "$input.title");
        assert_eq!(encode.target.context.as_deref(), Some("text"));
        assert_eq!(compiled.module_options.let_bindings.len(), 1);
        assert_eq!(
            compiled.module_options.output_functions[0].name,
            "html.text"
        );
    }

    #[test]
    fn compile_transform_template_normalizes_call_arguments_for_render() {
        let template = TemplateInput {
            uri: "templates/calls.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="main" @visibility="public" |
    {body | {call @template="badge" @with:title=" Ready " @with:count="2" @with:enabled="true"}}
  }
  {template @name="badge" |
    {param @name="title" @type="string" @required="true"}
    {param @name="count" @type="integer" @required="true"}
    {param @name="enabled" @type="boolean" @default="false"}
    {body | {span | Badge}}
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let mut diagnostics = Vec::new();

        let compiled = match compile_transform_template(
            TransformTemplateCompileSpec {
                context: &ctx(),
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        ) {
            Some(compiled) => compiled,
            None => panic!("template should compile: {diagnostics:?}"),
        };

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let call = &compiled.module_options.calls[0];
        assert_eq!(call.template, "badge");
        assert_eq!(call.arguments.get("title"), Some(&json!(" Ready ")));
        assert_eq!(call.arguments.get("count"), Some(&json!(2)));
        assert_eq!(call.arguments.get("enabled"), Some(&json!(true)));
    }

    #[test]
    fn render_transform_stage_evaluates_and_composes_encoded_text_artifacts() {
        let template = TemplateInput {
            uri: "templates/page.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="html.text"
      @category="html-text"
      @subject="string"
      @produces="text"
      @content-type="text/html"
      @schema="https://cem.dev/ns/data/html/1"
      @canonical=true
      @streamable=true
      @deterministic=true |
      {param @name="subject" @type="string" @required=true}
  }
  {template @name="main" @visibility="public" |
    {body |
      {let @name="encoderName" @value="html.text"}
      {$ encode($input.title, { contentType: "text/html", schema: "https://cem.dev/ns/data/html/1", category: "html-text", context: "text" }, { mode: "fragment", encoder: $encoderName }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        assert_eq!(compiled.module_options.let_bindings.len(), 1);
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"title": "Hello <CEM> & friends"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some("text/html".to_owned()),
            schema: Some("https://cem.dev/ns/data/html/1".to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            output.value,
            Value::String("Hello &lt;CEM&gt; &amp; friends".to_owned())
        );
        assert_eq!(
            output
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/html")
        );
        assert_eq!(
            output
                .identity
                .as_ref()
                .and_then(|identity| identity.schema.as_deref()),
            Some("https://cem.dev/ns/data/html/1")
        );
    }

    #[test]
    fn render_transform_stage_reports_invalid_let_expression() {
        let template = TemplateInput {
            uri: "templates/page-invalid-let.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="html.text"
      @category="html-text"
      @subject="string"
      @produces="text"
      @content-type="text/html"
      @schema="https://cem.dev/ns/data/html/1"
      @canonical=true
      @streamable=true
      @deterministic=true |
      {param @name="subject" @type="string" @required=true}
  }
  {template @name="main" @visibility="public" |
    {body |
      {let @name="depth" @type="integer" @expr="$input.title"}
      {$ encode($input.title, { contentType: "text/html", schema: "https://cem.dev/ns/data/html/1", category: "html-text", context: "text" }, { mode: "fragment", encoder: "html.text" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"title": "Hello CEM"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some("text/html".to_owned()),
            schema: Some("https://cem.dev/ns/data/html/1".to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: Some("template:main"),
            },
            &mut diagnostics,
        )
        .expect("template render returns adapter output with diagnostics");

        assert!(output.value.is_object());
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_LET_EXPR_INVALID_CODE
                && diagnostic.severity == Severity::Fatal
                && diagnostic.node.as_deref() == Some("template:main")
                && diagnostic
                    .message
                    .contains("template let `depth` expected integer, got string")
        }));
    }

    #[test]
    fn render_transform_stage_adapts_token_artifacts_to_text_output() {
        let template = TemplateInput {
            uri: "templates/token-stream.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="html.tokens"
      @visibility="public"
      @category="html-token-stream"
      @subject="string"
      @produces="tokens"
      @content-type="text/html"
      @schema="https://cem.dev/ns/data/html/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.title, { contentType: "text/html", schema: "https://cem.dev/ns/data/html/1", category: "html-token-stream", context: "text" }, { mode: "fragment", encoder: "html.tokens" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let mut context = ctx();
        context.transform_template_encode_registry.register(
            "html.tokens",
            |_binding: &crate::transform_template::TransformTemplateEncodeBinding,
             subject: &Value| {
                Ok(json!({
                    "tokens": [{
                        "kind": "syntax.text",
                        "text": subject.as_str().unwrap_or_default()
                    }]
                }))
            },
        );
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"title": "Hello CEM"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some("text/html".to_owned()),
            schema: Some("https://cem.dev/ns/data/html/1".to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(output.value, Value::String("Hello CEM".to_owned()));
        assert_eq!(
            output
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("text/html")
        );
        assert_eq!(
            output
                .identity
                .as_ref()
                .and_then(|identity| identity.schema.as_deref()),
            Some("https://cem.dev/ns/data/html/1")
        );
    }

    #[test]
    fn render_transform_stage_rejects_mixed_html_encode_contexts() {
        let template = TemplateInput {
            uri: "templates/page.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="html.text"
      @category="html-text"
      @subject="string"
      @produces="text"
      @content-type="text/html"
      @schema="https://cem.dev/ns/data/html/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {encoding-function
      @name="html.attribute"
      @category="html-attribute"
      @subject="string"
      @produces="text"
      @content-type="text/html"
      @schema="https://cem.dev/ns/data/html/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.title, { contentType: "text/html", schema: "https://cem.dev/ns/data/html/1", category: "html-text", context: "text" }, { mode: "fragment", encoder: "html.text" }) }
      {$ encode($input.title, { contentType: "text/html", schema: "https://cem.dev/ns/data/html/1", category: "html-attribute", context: "double-quoted-attribute" }, { mode: "fragment", encoder: "html.attribute" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"title": "Hello & CEM"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some("text/html".to_owned()),
            schema: Some("https://cem.dev/ns/data/html/1".to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders with diagnostics");

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == crate::transform_template::TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_CONTEXT_MISMATCH_CODE
                && diagnostic.message.contains("category")
        }));
        assert!(output.value.is_object());
    }

    #[test]
    fn render_transform_stage_evaluates_builtin_json_document_encoder() {
        let template = TemplateInput {
            uri: "templates/data-json.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="json.document"
      @category="json-document"
      @subject="array"
      @produces="text"
      @content-type="application/json"
      @schema="https://cem.dev/ns/data/json/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.items, { contentType: "application/json", schema: "https://cem.dev/ns/data/json/1", category: "json-document" }, { encoder: "json.document" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"items": ["Hello", 2, true]}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(crate::schema::registry::JSON_CONTENT_TYPE.to_owned()),
            schema: Some(crate::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            output.value,
            Value::String(r#"["Hello",2,true]"#.to_owned())
        );
        assert_eq!(
            output
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some(crate::schema::registry::JSON_CONTENT_TYPE)
        );
        assert_eq!(
            output
                .identity
                .as_ref()
                .and_then(|identity| identity.schema.as_deref()),
            Some(crate::schema::registry::JSON_VALUE_SCHEMA_URI)
        );
    }

    #[test]
    fn render_transform_stage_rejects_mixed_json_encode_contexts() {
        let template = TemplateInput {
            uri: "templates/data-json.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="json.document"
      @category="json-document"
      @subject="array"
      @produces="text"
      @content-type="application/json"
      @schema="https://cem.dev/ns/data/json/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {encoding-function
      @name="json.string"
      @category="json-string"
      @subject="string"
      @produces="text"
      @content-type="application/json"
      @schema="https://cem.dev/ns/data/json/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.items, { contentType: "application/json", schema: "https://cem.dev/ns/data/json/1", category: "json-document" }, { encoder: "json.document" }) }
      {$ encode($input.name, { contentType: "application/json", schema: "https://cem.dev/ns/data/json/1", category: "json-string" }, { encoder: "json.string" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"items": ["Hello"], "name": "CEM"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(crate::schema::registry::JSON_CONTENT_TYPE.to_owned()),
            schema: Some(crate::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders with diagnostics");

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == crate::transform_template::TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_CONTEXT_MISMATCH_CODE
                && diagnostic.message.contains("category")
        }));
        assert!(output.value.is_object());
    }

    #[test]
    fn render_transform_stage_applies_pretty_json_formatter_options() {
        let template = TemplateInput {
            uri: "templates/data-json.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="json.document"
      @category="json-document"
      @subject="array"
      @produces="text"
      @content-type="application/json"
      @schema="https://cem.dev/ns/data/json/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.items, { contentType: "application/json", schema: "https://cem.dev/ns/data/json/1", category: "json-document" }, { encoder: "json.document", pretty: true, indent: "    ", lineEnding: "lf", formatterProfile: "json.pretty" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"items": ["Hello", 2, true]}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(crate::schema::registry::JSON_CONTENT_TYPE.to_owned()),
            schema: Some(crate::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            output.value,
            Value::String("[\n    \"Hello\",\n    2,\n    true\n]".to_owned())
        );
    }

    #[test]
    fn render_transform_stage_rejects_mixed_json_formatter_profiles() {
        let template = TemplateInput {
            uri: "templates/data-json.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="json.string"
      @category="json-string"
      @subject="string"
      @produces="text"
      @content-type="application/json"
      @schema="https://cem.dev/ns/data/json/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.first, { contentType: "application/json", schema: "https://cem.dev/ns/data/json/1", category: "json-string" }, { encoder: "json.string", formatterProfile: "json.pretty" }) }
      {$ encode($input.second, { contentType: "application/json", schema: "https://cem.dev/ns/data/json/1", category: "json-string" }, { encoder: "json.string", formatterProfile: "json.canonical" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"first": "a", "second": "b"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(crate::schema::registry::JSON_CONTENT_TYPE.to_owned()),
            schema: Some(crate::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders with diagnostics");

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == crate::transform_template::TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_CONTEXT_MISMATCH_CODE
                && diagnostic.message.contains("formatterProfile")
        }));
        assert!(output.value.is_object());
    }

    #[test]
    fn render_transform_stage_evaluates_builtin_xml_text_encoder() {
        let template = TemplateInput {
            uri: "templates/data-xml.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="xml.text"
      @category="xml-text"
      @subject="string"
      @produces="text"
      @content-type="application/xml"
      @schema="https://cem.dev/ns/data/xml/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.title, { contentType: "application/xml", schema: "https://cem.dev/ns/data/xml/1", category: "xml-text", context: "text" }, { encoder: "xml.text" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"title": "Hello <CEM> & \"friends\""}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(crate::schema::registry::XML_CONTENT_TYPE.to_owned()),
            schema: Some(crate::schema::registry::XML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            output.value,
            Value::String("Hello &lt;CEM&gt; &amp; \"friends\"".to_owned())
        );
        assert_eq!(
            output
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some(crate::schema::registry::XML_CONTENT_TYPE)
        );
        assert_eq!(
            output
                .identity
                .as_ref()
                .and_then(|identity| identity.schema.as_deref()),
            Some(crate::schema::registry::XML_SCHEMA_URI)
        );
    }

    #[test]
    fn render_transform_stage_applies_xml_formatter_controls() {
        let template = TemplateInput {
            uri: "templates/data-xml.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="xml.text"
      @category="xml-text"
      @subject="string"
      @produces="text"
      @content-type="application/xml"
      @schema="https://cem.dev/ns/data/xml/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.title, { contentType: "application/xml", schema: "https://cem.dev/ns/data/xml/1", category: "xml-text", context: "text" }, { encoder: "xml.text", pretty: true, formatterProfile: "xml.pretty", lineEnding: "crlf", namespacePolicy: "repair", indent: "  " }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"title": "Line 1\nLine 2 <CEM> & friends"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(crate::schema::registry::XML_CONTENT_TYPE.to_owned()),
            schema: Some(crate::schema::registry::XML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            output.value,
            Value::String("Line 1\r\nLine 2 &lt;CEM&gt; &amp; friends".to_owned())
        );
    }

    #[test]
    fn render_transform_stage_rejects_mixed_xml_formatter_profiles() {
        let template = TemplateInput {
            uri: "templates/data-xml.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="xml.text"
      @category="xml-text"
      @subject="string"
      @produces="text"
      @content-type="application/xml"
      @schema="https://cem.dev/ns/data/xml/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.first, { contentType: "application/xml", schema: "https://cem.dev/ns/data/xml/1", category: "xml-text", context: "text" }, { encoder: "xml.text", formatterProfile: "xml.pretty" }) }
      {$ encode($input.second, { contentType: "application/xml", schema: "https://cem.dev/ns/data/xml/1", category: "xml-text", context: "text" }, { encoder: "xml.text", formatterProfile: "xml.canonical" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"first": "a", "second": "b"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(crate::schema::registry::XML_CONTENT_TYPE.to_owned()),
            schema: Some(crate::schema::registry::XML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders with diagnostics");

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == crate::transform_template::TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_CONTEXT_MISMATCH_CODE
                && diagnostic.message.contains("formatterProfile")
        }));
        assert!(output.value.is_object());
    }

    #[test]
    fn render_transform_stage_rejects_mixed_xml_encode_contexts() {
        let template = TemplateInput {
            uri: "templates/data-xml.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {encoding-function
      @name="xml.text"
      @category="xml-text"
      @subject="string"
      @produces="text"
      @content-type="application/xml"
      @schema="https://cem.dev/ns/data/xml/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {encoding-function
      @name="xml.attribute"
      @category="xml-attribute-value"
      @subject="string"
      @produces="text"
      @content-type="application/xml"
      @schema="https://cem.dev/ns/data/xml/1"
      @canonical=true
      @streamable=true
      @deterministic=true}
  {template @name="main" @visibility="public" |
    {body |
      {$ encode($input.title, { contentType: "application/xml", schema: "https://cem.dev/ns/data/xml/1", category: "xml-text", context: "text" }, { encoder: "xml.text" }) }
      {$ encode($input.title, { contentType: "application/xml", schema: "https://cem.dev/ns/data/xml/1", category: "xml-attribute-value", context: "double-quoted-attribute" }, { encoder: "xml.attribute" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let context = ctx();
        let adapter: Arc<dyn TransformTemplateAdapter> = Arc::new(ReadyCemtHtmlExportAdapter);
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let mut diagnostics = Vec::new();
        let compiled = compile_transform_template(
            TransformTemplateCompileSpec {
                context: &context,
                adapter: &adapter,
                template: &template,
                template_kind: TransformTemplateKind::CemNative,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: TransformTemplateModuleOptions::default(),
                execution_policy: TransformExecutionPolicy::default(),
            },
            &mut diagnostics,
        )
        .expect("template compiles");
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "input".to_owned(),
            uri: None,
            identity: None,
            value: json!({"title": "Hello & CEM"}),
        };
        let secondary_inputs = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(crate::schema::registry::XML_CONTENT_TYPE.to_owned()),
            schema: Some(crate::schema::registry::XML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let output = render_transform_stage(
            TransformStageRenderSpec {
                context: &context,
                adapter: &adapter,
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
                diagnostic_uri: &template.uri,
                diagnostic_node: None,
            },
            &mut diagnostics,
        )
        .expect("template renders with diagnostics");

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == crate::transform_template::TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_CONTEXT_MISMATCH_CODE
                && diagnostic.message.contains("category")
        }));
        assert!(output.value.is_object());
    }

    #[test]
    fn template_module_contract_rejects_private_or_missing_named_entrypoints() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::new();
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &params,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_ENTRYPOINT_NOT_PUBLIC_CODE));
    }

    #[test]
    fn template_module_contract_accepts_public_entrypoint_params() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "locale".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::Any,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "card.title".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::Any,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([
            ("locale".to_owned(), json!("en-US")),
            ("title".to_owned(), json!("Intro")),
        ]);
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &params,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn template_module_contract_accepts_qualified_entrypoint_param_aliases() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "card.title".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::Any,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([("card.title".to_owned(), json!("Intro"))]);
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &params,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn template_module_contract_rejects_duplicate_param_aliases() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "card.title".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::Any,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([
            ("card.title".to_owned(), json!("Intro")),
            ("title".to_owned(), json!("Overview")),
        ]);
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &params,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_DUPLICATE_ALIAS_CODE));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE));
    }

    #[test]
    fn template_module_contract_treats_explicit_null_params_as_provided() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "locale".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::Any,
                    nullable: true,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "card.title".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::Any,
                    nullable: true,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([
            ("locale".to_owned(), Value::Null),
            ("title".to_owned(), Value::Null),
        ]);
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &params,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn template_module_contract_rejects_non_nullable_null_params_as_type_mismatch() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "card.title".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::String,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([("title".to_owned(), Value::Null)]);
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &params,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_TYPE_CODE));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE));
    }

    #[test]
    fn template_module_contract_rejects_unknown_and_missing_params() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "card.title".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::Any,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([("unexpected".to_owned(), json!("value"))]);
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &params,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_UNKNOWN_CODE));
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE));
    }

    #[test]
    fn template_module_contract_coerces_declared_string_params() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                TransformTemplateModuleParamDeclaration {
                    name: "enabled".to_owned(),
                    value_type: TransformTemplateModuleParamType::Boolean,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "options".to_owned(),
                    value_type: TransformTemplateModuleParamType::Object,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "raw".to_owned(),
                    value_type: TransformTemplateModuleParamType::Any,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "maybe".to_owned(),
                    value_type: TransformTemplateModuleParamType::Any,
                    nullable: true,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "card.count".to_owned(),
                    value_type: TransformTemplateModuleParamType::Integer,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "card.subtitle".to_owned(),
                    value_type: TransformTemplateModuleParamType::String,
                    nullable: true,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "card.tags".to_owned(),
                    value_type: TransformTemplateModuleParamType::Array,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([
            ("enabled".to_owned(), json!("true")),
            ("options".to_owned(), json!(r#"{"compact":true}"#)),
            ("raw".to_owned(), json!("null")),
            ("maybe".to_owned(), json!("null")),
            ("count".to_owned(), json!("42")),
            ("subtitle".to_owned(), json!("null")),
            ("tags".to_owned(), json!(r#"["a","b"]"#)),
        ]);

        let normalized =
            normalize_transform_template_module_params(&params, Some("card"), &options);

        assert_eq!(normalized.get("enabled"), Some(&json!(true)));
        assert_eq!(normalized.get("options"), Some(&json!({"compact": true})));
        assert_eq!(normalized.get("raw"), Some(&json!("null")));
        assert_eq!(normalized.get("maybe"), Some(&Value::Null));
        assert_eq!(normalized.get("count"), Some(&json!(42)));
        assert_eq!(normalized.get("subtitle"), Some(&Value::Null));
        assert_eq!(normalized.get("tags"), Some(&json!(["a", "b"])));

        let mut diagnostics = Vec::new();
        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &normalized,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn template_module_contract_rejects_uncoercible_string_params() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                TransformTemplateModuleParamDeclaration {
                    name: "enabled".to_owned(),
                    value_type: TransformTemplateModuleParamType::Boolean,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "options".to_owned(),
                    value_type: TransformTemplateModuleParamType::Object,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "card.count".to_owned(),
                    value_type: TransformTemplateModuleParamType::Integer,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([
            ("enabled".to_owned(), json!("yes")),
            ("options".to_owned(), json!("not json")),
            ("count".to_owned(), json!("1.5")),
        ]);
        let normalized =
            normalize_transform_template_module_params(&params, Some("card"), &options);
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &normalized,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert_eq!(normalized.get("enabled"), Some(&json!("yes")));
        assert_eq!(normalized.get("options"), Some(&json!("not json")));
        assert_eq!(normalized.get("count"), Some(&json!("1.5")));
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_TYPE_CODE)
                .count(),
            3
        );
    }

    #[test]
    fn template_module_contract_rejects_param_type_mismatches() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "card".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Public,
                },
            ],
            params: vec![
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "locale".to_owned(),
                    value_type: crate::transform_template::TransformTemplateModuleParamType::String,
                    nullable: false,
                    default_value: Some(json!(true)),
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Public,
                },
                crate::transform_template::TransformTemplateModuleParamDeclaration {
                    name: "card.count".to_owned(),
                    value_type:
                        crate::transform_template::TransformTemplateModuleParamType::Integer,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let params = BTreeMap::from([("count".to_owned(), json!(1.5))]);
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_module_contract(
            &template,
            &TransformTemplateEntrypoint::named("card"),
            &params,
            &options,
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_TYPE_CODE)
                .count(),
            2
        );
    }

    #[test]
    fn template_module_call_validation_accepts_same_module_private_entrypoints() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "helper".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: None,
                template: "helper".to_owned(),
                arguments: BTreeMap::new(),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &TransformTemplateModulePreflight::default(),
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn template_module_call_validation_rejects_unknown_same_module_entrypoints() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: None,
                template: "missing".to_owned(),
                arguments: BTreeMap::new(),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &TransformTemplateModulePreflight::default(),
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE));
    }

    #[test]
    fn template_module_call_validation_accepts_typed_call_arguments() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "badge".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            params: vec![
                TransformTemplateModuleParamDeclaration {
                    name: "badge.title".to_owned(),
                    value_type: TransformTemplateModuleParamType::String,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "badge.count".to_owned(),
                    value_type: TransformTemplateModuleParamType::Integer,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "badge.options".to_owned(),
                    value_type: TransformTemplateModuleParamType::Object,
                    nullable: false,
                    default_value: Some(json!({})),
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: None,
                template: "badge".to_owned(),
                arguments: BTreeMap::from([
                    ("count".to_owned(), json!("2")),
                    ("options".to_owned(), json!(r#"{"compact":true}"#)),
                    ("title".to_owned(), json!(" Ready ")),
                ]),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &TransformTemplateModulePreflight::default(),
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn template_module_call_validation_defers_dynamic_call_argument_types() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "row".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            params: vec![TransformTemplateModuleParamDeclaration {
                name: "row.item".to_owned(),
                value_type: TransformTemplateModuleParamType::Object,
                nullable: false,
                default_value: None,
                required: true,
                visibility: TransformTemplateModuleVisibility::Private,
            }],
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: None,
                template: "row".to_owned(),
                arguments: BTreeMap::from([("item".to_owned(), json!("{$item}"))]),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &TransformTemplateModulePreflight::default(),
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn template_module_call_validation_rejects_invalid_call_arguments() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "badge".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            params: vec![
                TransformTemplateModuleParamDeclaration {
                    name: "badge.title".to_owned(),
                    value_type: TransformTemplateModuleParamType::String,
                    nullable: false,
                    default_value: None,
                    required: true,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
                TransformTemplateModuleParamDeclaration {
                    name: "badge.count".to_owned(),
                    value_type: TransformTemplateModuleParamType::Integer,
                    nullable: false,
                    default_value: None,
                    required: false,
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: None,
                template: "badge".to_owned(),
                arguments: BTreeMap::from([
                    ("count".to_owned(), json!("not-int")),
                    ("unexpected".to_owned(), json!("value")),
                ]),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &TransformTemplateModulePreflight::default(),
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_UNKNOWN_CODE));
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE));
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_TYPE_CODE));
    }

    #[test]
    fn template_module_call_validation_rejects_duplicate_argument_aliases() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            entrypoints: vec![
                crate::transform_template::TransformTemplateModuleEntrypointDeclaration {
                    name: "badge".to_owned(),
                    visibility: TransformTemplateModuleVisibility::Private,
                },
            ],
            params: vec![TransformTemplateModuleParamDeclaration {
                name: "badge.title".to_owned(),
                value_type: TransformTemplateModuleParamType::String,
                nullable: false,
                default_value: None,
                required: true,
                visibility: TransformTemplateModuleVisibility::Private,
            }],
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: None,
                template: "badge".to_owned(),
                arguments: BTreeMap::from([
                    ("badge.title".to_owned(), json!("Qualified")),
                    ("title".to_owned(), json!("Local")),
                ]),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &TransformTemplateModulePreflight::default(),
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_DUPLICATE_ALIAS_CODE));
        assert!(!diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE));
    }

    #[test]
    fn template_module_call_validation_accepts_imported_public_entrypoints() {
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            StaticReadResolver {
                resolved_uri: "cem+vfs://templates/ui.cem",
                bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" | {body | {span | Icon}}}
}"#,
                content_type: Some("text/cem-ml"),
            },
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: Some("ui".to_owned()),
                template: "icon".to_owned(),
                arguments: BTreeMap::new(),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();
        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        )
        .expect("preflight should resolve import");

        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &preflight,
            &mut diagnostics,
        );

        assert!(validated.is_some(), "{diagnostics:?}");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn template_module_call_validation_rejects_imported_private_entrypoints() {
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            StaticReadResolver {
                resolved_uri: "cem+vfs://templates/ui.cem",
                bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" | {body | {span | Icon}}}
}"#,
                content_type: Some("text/cem-ml"),
            },
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            calls: vec![crate::transform_template::TransformTemplateModuleCallSite {
                owner_entrypoint: Some("card".to_owned()),
                from: Some("ui".to_owned()),
                template: "icon".to_owned(),
                arguments: BTreeMap::new(),
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();
        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        )
        .expect("preflight should resolve import");

        let validated = validate_transform_template_call_sites(
            &template,
            &options,
            &preflight,
            &mut diagnostics,
        );

        assert!(validated.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_ENTRYPOINT_NOT_PUBLIC_CODE));
    }

    #[test]
    fn template_module_preflight_rejects_reserved_includes() {
        let template = template("main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::IncludeReserved,
                source_range: None,
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &ctx(),
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        );

        assert!(preflight.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_INCLUDE_RESERVED_CODE));
    }

    #[test]
    fn template_module_preflight_rejects_duplicate_import_aliases() {
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            StaticReadResolver {
                resolved_uri: "cem+vfs://templates/ui.cem",
                bytes: b"{template @name=\"card\"}",
                content_type: Some("text/cem-ml"),
            },
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![
                TransformTemplateModuleImport {
                    alias: "ui".to_owned(),
                    uri: "ui.cem".to_owned(),
                    identity: None,
                    kind: TransformTemplateModuleDependencyKind::Import,
                    source_range: None,
                },
                TransformTemplateModuleImport {
                    alias: "ui".to_owned(),
                    uri: "ui-2.cem".to_owned(),
                    identity: None,
                    kind: TransformTemplateModuleDependencyKind::Import,
                    source_range: None,
                },
            ],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        );

        assert!(preflight.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_IMPORT_ALIAS_DUPLICATE_CODE));
    }

    #[test]
    fn template_module_preflight_rejects_direct_import_cycles() {
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            StaticReadResolver {
                resolved_uri: "cem+vfs://templates/main.cem",
                bytes: b"{main}",
                content_type: Some("text/cem-ml"),
            },
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "self".to_owned(),
                uri: "main.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        );

        assert!(preflight.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_IMPORT_CYCLE_CODE));
    }

    #[test]
    fn template_module_preflight_rejects_nested_import_cycles() {
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            MapReadResolver {
                entries: vec![
                    (
                        "cem+vfs://templates/ui.cem",
                        br#"{@doc cem-ml 1}
{module | {import @as="main" @src="main.cem"}}"#,
                        Some("text/cem-ml"),
                    ),
                    (
                        "cem+vfs://templates/main.cem",
                        br#"{@doc cem-ml 1}
{module | {body | {span | Main}}}"#,
                        Some("text/cem-ml"),
                    ),
                ],
            },
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        );

        assert!(preflight.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_IMPORT_CYCLE_CODE));
    }

    #[test]
    fn template_module_preflight_enforces_import_depth_limit() {
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            MapReadResolver {
                entries: vec![(
                    "cem+vfs://templates/ui.cem",
                    br#"{@doc cem-ml 1}
{module | {import @as="icons" @src="icons.cem"}}"#,
                    Some("text/cem-ml"),
                )],
            },
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            limits: crate::transform_template::TransformTemplateModuleLimits {
                max_import_depth: 1,
                max_recursion_depth: 64,
            },
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        );

        assert!(preflight.is_none());
        assert!(diagnostics
            .iter()
            .any(|diag| diag.code == TRANSFORM_TEMPLATE_IMPORT_DEPTH_CODE));
    }

    #[test]
    fn template_module_preflight_reports_denied_external_imports() {
        let template = template(
            "cem+vfs://templates/main.cem",
            br#"{@doc cem-ml 1}
{module |
  {import @as="ui" @src="https://example.test/templates/ui.cem"}
  {template @name="card" | {body | {span | Card}}}
}"#,
        );
        let parse_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: template.clone(),
            });
        assert!(
            parse_response.diagnostics.is_empty(),
            "{:?}",
            parse_response.diagnostics
        );
        assert!(parse_response.module_options.imports[0]
            .source_range
            .is_some());
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &ctx(),
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &parse_response.module_options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        );

        assert!(preflight.is_none());
        let diagnostic = diagnostics
            .iter()
            .find(|diag| diag.code == CEM_TEMPLATE_IMPORT_DENIED_CODE)
            .unwrap_or_else(|| panic!("expected denied import diagnostic: {diagnostics:?}"));
        let details = diagnostic.details.as_ref().expect("details");
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.uri.as_deref(), Some(template.uri.as_str()));
        assert_eq!(
            details["requestedUri"],
            "https://example.test/templates/ui.cem"
        );
        assert_eq!(details["importAlias"], "ui");
        assert_eq!(details["resolverCode"], "cem.resolver.unsupported");
        assert_eq!(details["reason"], "missing-uri-scheme-grant");
        assert_eq!(details["schemeTier"], "external-resource");
        assert!(details["sourceRange"]["span"]["start"].is_u64());
    }

    #[test]
    fn template_module_preflight_reports_unresolved_local_imports() {
        let template = template(
            "main.cem",
            br#"{@doc cem-ml 1}
{module |
  {import @as="missing" @src="definitely-missing-template-import.cem"}
  {template @name="card" | {body | {span | Card}}}
}"#,
        );
        let parse_response =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: template.clone(),
            });
        assert!(
            parse_response.diagnostics.is_empty(),
            "{:?}",
            parse_response.diagnostics
        );
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &ctx(),
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &parse_response.module_options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        );

        assert!(preflight.is_none());
        let diagnostic = diagnostics
            .iter()
            .find(|diag| diag.code == CEM_TEMPLATE_IMPORT_UNRESOLVED_CODE)
            .unwrap_or_else(|| panic!("expected unresolved import diagnostic: {diagnostics:?}"));
        let details = diagnostic.details.as_ref().expect("details");
        assert_eq!(
            details["requestedUri"],
            "definitely-missing-template-import.cem"
        );
        assert_eq!(details["importAlias"], "missing");
        assert_eq!(details["resolverCode"], "cem.resolver.io");
        assert_eq!(details["reason"], "read-failed");
        assert_eq!(details["schemeTier"], "relative-resource");
    }

    #[test]
    fn template_module_preflight_applies_explicit_policy_substitution() {
        let mut context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            MapReadResolver {
                entries: vec![(
                    "cem+vfs://templates/substitute.cem",
                    br#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" | {body | {span | Substitute}}}
}"#,
                    Some("text/cem-ml"),
                )],
            },
        );
        context.resolver_policy = ResolverPolicy::new().with_substitution(
            ResolvePolicyRequestKey::new(
                "ui.cem",
                ResolvePurpose::Template,
                ResolveDirection::Read,
            )
            .with_base_uri("cem+vfs://templates/main.cem"),
            ResolvePolicySubstitution::new("substitute.cem", "test-substitution"),
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        )
        .expect("preflight should resolve substituted import");

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let resolved = &preflight.resolved_imports[0];
        assert_eq!(resolved.requested_uri.as_deref(), Some("ui.cem"));
        assert_eq!(resolved.normalized_uri.as_deref(), Some("ui.cem"));
        assert_eq!(resolved.substituted_uri.as_deref(), Some("substitute.cem"));
        assert_eq!(resolved.uri, "cem+vfs://templates/substitute.cem");
        assert!(resolved
            .resolver_policy_stamp
            .as_deref()
            .is_some_and(|stamp| stamp.contains("substitute.cem")));
        assert!(preflight
            .cache_key
            .expect("cache key")
            .dependency_graph_hash
            .starts_with("cem-bin/1+blake3:"));
    }

    #[test]
    fn template_module_preflight_reports_unresolved_failed_policy_substitution() {
        let mut context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::Template,
            MapReadResolver {
                entries: Vec::new(),
            },
        );
        context.resolver_policy = ResolverPolicy::new().with_substitution(
            ResolvePolicyRequestKey::new(
                "ui.cem",
                ResolvePurpose::Template,
                ResolveDirection::Read,
            )
            .with_base_uri("cem+vfs://templates/main.cem"),
            ResolvePolicySubstitution::new("missing.cem", "test-substitution"),
        );
        let template = template("cem+vfs://templates/main.cem", b"{main}");
        let options = TransformTemplateModuleOptions {
            imports: vec![TransformTemplateModuleImport {
                alias: "ui".to_owned(),
                uri: "ui.cem".to_owned(),
                identity: None,
                kind: TransformTemplateModuleDependencyKind::Import,
                source_range: None,
            }],
            ..TransformTemplateModuleOptions::default()
        };
        let mut diagnostics = Vec::new();

        let preflight = preflight_transform_template_modules(
            &context,
            "adapter",
            &template,
            &TransformTemplateEntrypoint::implicit(),
            &options,
            TransformExecutionPolicy::default(),
            &mut diagnostics,
        );

        assert!(preflight.is_none());
        let diagnostic = diagnostics
            .iter()
            .find(|diag| diag.code == CEM_TEMPLATE_IMPORT_UNRESOLVED_CODE)
            .unwrap_or_else(|| panic!("expected unresolved import diagnostic: {diagnostics:?}"));
        let details = diagnostic.details.as_ref().expect("details");
        assert_eq!(details["requestedUri"], "ui.cem");
        assert_eq!(details["normalizedUri"], "ui.cem");
        assert_eq!(details["effectiveUri"], "missing.cem");
        assert_eq!(details["substitutedUri"], "missing.cem");
        assert_eq!(details["resolverCode"], "cem.resolver.unsupported");
        assert_eq!(details["reason"], "substitution-target-missing");
        assert!(details["resolverPolicy"]
            .as_str()
            .is_some_and(|stamp| stamp.contains("missing.cem")));
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
    fn parse_accepts_recognized_root_scope_scheduler_budgets_without_unenforced_warning() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("queueSize".to_owned(), "12".to_owned());
        source
            .root_scope
            .budgets
            .insert("pluginTimeBudgetMs".to_owned(), "7".to_owned());
        let req = ParseRequest {
            input: source,
            projection: ParseProjection::DomJson,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: false,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().parse(req).unwrap();
        assert!(!resp
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.budgets_unenforced"));
        assert!(!resp
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.budget_unenforced"));
    }

    #[test]
    fn parse_reports_invalid_recognized_root_scope_scheduler_budgets() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("queueSize".to_owned(), "nope".to_owned());
        let req = ParseRequest {
            input: source,
            projection: ParseProjection::DomJson,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: false,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().parse(req).unwrap();
        assert!(resp
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.budget_invalid"));
    }

    #[test]
    fn root_module_map_json_loads_flat_and_nested_aliases() {
        let value = json!({
            "ui/button": "./schemas/button.schema",
            "schemas": {
                "ui/card": {
                    "src": "./schemas/card.schema"
                }
            },
            "imports": {
                "ui/list": "./schemas/list.schema"
            }
        });

        let aliases = module_map_aliases(&value).unwrap();
        assert_eq!(
            aliases.get("ui/button").map(String::as_str),
            Some("./schemas/button.schema")
        );
        assert_eq!(
            aliases.get("ui/card").map(String::as_str),
            Some("./schemas/card.schema")
        );
        assert_eq!(
            aliases.get("ui/list").map(String::as_str),
            Some("./schemas/list.schema")
        );
    }

    #[test]
    fn root_module_map_loader_reports_invalid_json() {
        let path = std::env::temp_dir().join(format!(
            "cem-ml-invalid-module-map-{}.json",
            std::process::id()
        ));
        std::fs::write(&path, "{").unwrap();
        let scope = ScopeConfig {
            module_map: Some(path.to_string_lossy().into_owned()),
            ..ScopeConfig::default()
        };

        let loaded = load_root_module_map(&scope, None);
        let _ = std::fs::remove_file(path);

        assert!(loaded.entries.is_empty());
        assert!(loaded
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.module_map_invalid"));
    }

    #[test]
    fn root_module_map_loader_reads_local_file_uri() {
        let path = std::env::temp_dir().join(format!(
            "cem-ml-file-uri-module-map-{}.json",
            std::process::id()
        ));
        std::fs::write(
            &path,
            r#"{"schemas":{"ui/button":"./schemas/button.schema"}}"#,
        )
        .unwrap();
        let scope = ScopeConfig {
            module_map: Some(format!("file://{}", path.display())),
            ..ScopeConfig::default()
        };

        let loaded = load_root_module_map(&scope, None);
        let _ = std::fs::remove_file(path);

        assert!(loaded.diagnostics.is_empty());
        assert_eq!(
            loaded.entries.get("ui/button").map(String::as_str),
            Some("./schemas/button.schema")
        );
    }

    #[test]
    fn root_module_map_loader_reports_non_local_file_uri() {
        let scope = ScopeConfig {
            module_map: Some("file://example.test/cem.modules.json".to_owned()),
            ..ScopeConfig::default()
        };

        let loaded = load_root_module_map(&scope, None);

        assert!(loaded.entries.is_empty());
        assert!(loaded
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.module_map_unreadable"));
    }

    #[test]
    fn root_module_map_loader_reports_remote_uri_resolver_unsupported() {
        let scope = ScopeConfig {
            module_map: Some("https://example.test/cem.modules.json".to_owned()),
            ..ScopeConfig::default()
        };

        let loaded = load_root_module_map(&scope, None);

        assert!(loaded.entries.is_empty());
        assert!(loaded.diagnostics.iter().any(|diag| {
            diag.code == "cem.scope.module_map_resolver_unsupported"
                && diag.message.contains("remote/custom URI resolver")
        }));
    }

    #[test]
    fn root_module_map_loader_reads_custom_resolver_uri() {
        let scope = ScopeConfig {
            module_map: Some("cem+vfs://workspace/maps/cem.modules.json".to_owned()),
            ..ScopeConfig::default()
        };
        let context = context_with_resolver(
            "cem+vfs",
            ResolvePurpose::ModuleMap,
            StaticReadResolver {
                resolved_uri: "cem+vfs://workspace/maps/cem.modules.json",
                bytes: br#"{"schemas":{"ui/button":"./schemas/button.schema"}}"#,
                content_type: Some("application/json"),
            },
        );

        let loaded = load_root_module_map(&scope, Some(&context));

        assert!(loaded.diagnostics.is_empty());
        assert_eq!(
            loaded.entries.get("ui/button").map(String::as_str),
            Some("./schemas/button.schema")
        );
        assert_eq!(
            loaded.uri.as_deref(),
            Some("cem+vfs://workspace/maps/cem.modules.json")
        );
    }

    #[test]
    fn parse_enforces_root_scope_parse_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("parseMs".to_owned(), "0".to_owned());
        let req = ParseRequest {
            input: source,
            projection: ParseProjection::DomJson,
            fail_level: FailLevel::Parse,
            preserve_source_offsets: false,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().parse(req).unwrap();
        assert!(resp.diagnostics.iter().any(|diag| {
            diag.code == "cem.scope.budget_exceeded" && diag.severity == Severity::Error
        }));
    }

    #[test]
    fn validate_enforces_root_scope_validate_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("validateMs".to_owned(), "0".to_owned());
        let req = ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert!(resp.report.diagnostics.iter().any(|diag| {
            diag.code == "cem.scope.budget_exceeded" && diag.severity == Severity::Error
        }));
    }

    #[test]
    fn check_enforces_root_scope_check_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("checkMs".to_owned(), "0".to_owned());
        let req = CheckRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            zero_hard_violations: false,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().check(req).unwrap();
        assert!(resp.report.diagnostics.iter().any(|diag| {
            diag.code == "cem.scope.budget_exceeded" && diag.severity == Severity::Error
        }));
    }

    #[test]
    fn convert_enforces_root_scope_convert_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("convertMs".to_owned(), "0".to_owned());
        let req = ConvertRequest {
            input: source,
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert!(resp.diagnostics.iter().any(|diag| {
            diag.code == "cem.scope.budget_exceeded" && diag.severity == Severity::Error
        }));
    }

    #[test]
    fn convert_enforces_target_scope_convert_ms_budget() {
        let mut target_scope = ScopeConfig::default();
        target_scope
            .budgets
            .insert("convertMs".to_owned(), "0".to_owned());
        let req = ConvertRequest {
            input: input(b"{p Hi}", "budgeted.cem"),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
            target: None,
            target_scope,
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert!(resp.diagnostics.iter().any(|diag| {
            diag.code == "cem.scope.budget_exceeded" && diag.severity == Severity::Error
        }));
    }

    #[test]
    fn trace_enforces_root_scope_trace_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("traceMs".to_owned(), "0".to_owned());
        let req = TraceRequest {
            input: source,
            projection: TraceProjection::Json,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().trace(req).unwrap();
        let diagnostics = resp.body["report"]["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
    }

    #[test]
    fn inspect_enforces_root_scope_inspect_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("inspectMs".to_owned(), "0".to_owned());
        let req = InspectRequest {
            input: source,
            show: InspectView::Diagnostics,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().inspect(req).unwrap();
        let diagnostics = resp.body["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
    }

    #[test]
    fn bench_enforces_root_scope_bench_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("benchMs".to_owned(), "0".to_owned());
        let req = BenchRequest {
            inputs: vec![source],
            projection: BenchProjection::Json,
            iterations: 1,
            budget_ms: None,
            profile: None,
            cold_cache: false,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().bench(req).unwrap();
        assert!(resp.budget_exceeded);
        let diagnostics = resp.body["diagnostics"].as_array().unwrap();
        assert!(diagnostics
            .iter()
            .any(|diag| diag["code"] == "cem.scope.budget_exceeded"));
    }

    #[test]
    fn fixture_validate_enforces_root_scope_fixture_validate_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("fixtureValidateMs".to_owned(), "0".to_owned());
        let req = FixtureValidateRequest {
            inputs: vec![source],
            fail_level: FailLevel::Validate,
            zero_hard_violations: false,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().fixture_validate(req).unwrap();
        assert!(resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.budget_exceeded"));
    }

    #[test]
    fn fixture_roundtrip_enforces_root_scope_fixture_roundtrip_ms_budget() {
        let mut source = input(b"{p Hi}", "budgeted.cem");
        source
            .root_scope
            .budgets
            .insert("fixtureRoundtripMs".to_owned(), "0".to_owned());
        let req = FixtureRoundtripRequest {
            inputs: vec![source],
            to_format: LayerFormat::DomJson,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().fixture_roundtrip(req).unwrap();
        assert!(resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.budget_exceeded"));
    }

    #[test]
    fn observe_pipeline_enforces_root_scope_observe_ms_budget() {
        let mut root_scope = ScopeConfig::default();
        root_scope
            .budgets
            .insert("observeMs".to_owned(), "0".to_owned());
        let observer = crate::observability::BufferingObserver::new();

        let run = observe_pipeline_scoped(b"{p Hi}", InputFormat::Cem, &root_scope, &observer);
        assert!(run.diagnostics.iter().any(|diag| {
            diag.code == "cem.scope.budget_exceeded" && diag.severity == Severity::Error
        }));

        let events = observer.drain();
        assert!(events.iter().any(|event| {
            event.validate.as_ref().is_some_and(|validate| {
                validate.code == "cem.scope.budget_exceeded" && validate.severity == "error"
            })
        }));
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
    fn validate_schema_package_artifact_contract_reads_cemt_through_template_resolver() {
        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(
            "cem+vfs",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            MapReadResolver {
                entries: vec![(
                    "cem+vfs://packages/demo/v1/formatters/demo.cemt",
                    br#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="demo.format"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true
    }
}
"#,
                    Some(CEM_TRANSFORM_CONTENT_TYPE),
                )],
            },
        );
        let req = ValidateRequest {
            inputs: vec![input(
                br#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="demo" @version="1.0.0" |
    {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
    {content-type @value="application/vnd.example.demo+cem" @primary=true}
    {artifact
        @kind="formatter"
        @path="formatters/demo.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="wrong-tree"
        @function-name="demo.format"
        @formatter-profile="compact"
    }
}
"#,
                "cem+vfs://packages/demo/v1/package.cem",
            )],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext {
                schema: Some(CEM_SCHEMA_PACKAGE_URI.to_owned()),
                content_type: Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned()),
                resolver_registry,
                ..ctx()
            },
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();

        assert!(resp.report.diagnostics.iter().any(|diag| {
            diag.code == "cem.schema_package.artifact_check"
                && diag.details.as_ref().and_then(|details| {
                    details.get("checkKind").and_then(serde_json::Value::as_str)
                }) == Some("artifact-function-contract")
                && diag
                    .message
                    .contains("target category metadata expected `wrong-tree`")
                && diag.message.contains("CEMT declares `cem-tree`")
        }));
        let artifact_source_read_failed = resp.report.diagnostics.iter().any(|diag| {
            diag.code == "cem.schema_package.artifact_check"
                && diag.details.as_ref().and_then(|details| {
                    details.get("checkKind").and_then(serde_json::Value::as_str)
                }) == Some("artifact-source-readable")
        });
        assert!(!artifact_source_read_failed);
    }

    #[test]
    fn validate_schema_package_converter_contract_reads_cemt_through_template_resolver() {
        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(
            "cem+vfs",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            MapReadResolver {
                entries: vec![(
                    "cem+vfs://packages/demo/v1/converters/demo-to-html.cemt",
                    br#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {template @name="main" | {text "not a supported DOM converter"}}
}
"#,
                    Some(CEM_TRANSFORM_CONTENT_TYPE),
                )],
            },
        );
        let req = ValidateRequest {
            inputs: vec![input(
                br#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="demo" @version="1.0.0" |
    {schema @uri="https://example.test/ns/demo/1" @source="schema/demo.cem"}
    {content-type @value="application/vnd.example.demo+cem" @primary=true}
    {converter
        @id="demo-to-html"
        @implementation="cemt"
        @template="converters/demo-to-html.cemt"
        @template-content-type="application/vnd.cem.transform+cem"
        @template-schema="https://cem.dev/ns/transform/cem/1"
        @output-syntax="html"
        @encoding-category="html-document"
        @formatter-profile="compact"
        @color-profile="classes" |
        {from @content-type="application/vnd.example.demo+cem" @schema="https://example.test/ns/demo/1"}
        {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
    }
}
"#,
                "cem+vfs://packages/demo/v1/package.cem",
            )],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: EngineContext {
                schema: Some(CEM_SCHEMA_PACKAGE_URI.to_owned()),
                content_type: Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned()),
                resolver_registry,
                ..ctx()
            },
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();

        assert!(resp.report.diagnostics.iter().any(|diag| {
            diag.code == "cem.schema_package.converter_check"
                && diag.details.as_ref().and_then(|details| {
                    details.get("checkKind").and_then(serde_json::Value::as_str)
                }) == Some("converter-template-contract")
                && diag.message.contains("demo-to-html")
                && diag
                    .message
                    .contains("formatted CEM tree before the writer")
                && diag.message.contains("supported DOM projection converter")
        }));
        assert!(!resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.schema_package.converter_template_source_unreadable"));
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
        source.root_scope.policy = Some("strict".to_owned());
        source
            .root_scope
            .budgets
            .insert("layoutMs".to_owned(), "5".to_owned());
        let req = ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert!(resp.report.summary.warning_count >= 2);
        assert!(!resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.module_map_unenforced"));
        assert!(resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.policy_unenforced"));
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
    fn validate_resolves_root_scope_version_pins() {
        let mut source = input(b"{p Hi}", "versioned.cem");
        source
            .root_scope
            .version_pins
            .insert("cem-ml".to_owned(), "1".to_owned());
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
            .any(|diag| diag.code == "cem.doc.version_resolved"));
        assert!(!resp
            .report
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.scope.version_pins_unenforced"));
    }

    #[test]
    fn validate_reports_invalid_root_scope_version_pins() {
        let mut source = input(b"{p Hi}", "versioned.cem");
        source
            .root_scope
            .version_pins
            .insert("cem-ml".to_owned(), "2".to_owned());
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
            .any(|diag| diag.code == "cem.doc.version_unsupported"));
    }

    #[test]
    fn validate_reports_unsupported_root_scope_version_pin_targets() {
        let mut source = input(b"{p Hi}", "versioned.cem");
        source
            .root_scope
            .version_pins
            .insert("urn:other-format".to_owned(), "1".to_owned());
        let req = ValidateRequest {
            inputs: vec![source],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().validate(req).unwrap();
        assert!(resp.report.diagnostics.iter().any(|diag| {
            diag.code == "cem.scope.version_pin_target_unsupported"
                && diag.severity == Severity::Warning
        }));
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
    fn convert_dom_bin_returns_native_primary_bytes() {
        let req = ConvertRequest {
            input: input(b"{p Hi}", "in"),
            to_format: LayerFormat::DomBin,
            preserve_source_offsets: false,
            context: ctx(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        let bytes = resp.primary_bytes.as_ref().expect("primary bytes");

        assert_eq!(resp.primary["kind"], "cem-binary-projection");
        assert_eq!(resp.primary["nativeBytes"], true);
        assert!(resp.primary.get("chunks").is_none());
        assert_eq!(
            bytes.content_type,
            crate::schema::registry::CEM_DOM_PROJECTION_CONTENT_TYPE
        );
        assert_eq!(
            bytes.schema.as_deref(),
            Some(crate::lifecycle::DOM_PROJECTION_SCHEMA)
        );
        assert_eq!(bytes.hash, resp.primary["hash"].as_str().unwrap());
        assert!(bytes.bytes.starts_with(b"CEMPROJ\0"));
    }

    #[test]
    fn convert_dom_json_does_not_return_native_primary_bytes() {
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

        assert!(resp.primary_bytes.is_none());
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
            "{cem:if @test=\"!(disabled)\" |\n    {button | Go}\n}\n"
        );
        assert!(
            resp.diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_target_cem_content_type_selects_canonical_cem_export() {
        let req = ConvertRequest {
            input: input(b"{p | Hi}", "in.cem"),
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
        assert_eq!(resp.primary["content"], "{p | Hi}\n");
        assert_ne!(resp.primary["sourceMap"], Value::Null);
        assert!(
            !resp.primary["outputSpans"].as_array().unwrap().is_empty(),
            "CEM output must pass through the CEMT writer pipeline, not the byte-preserving short-circuit"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_target_cem_applies_generic_line_ending_formatter_option() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
            target: Some(FormatIdentity {
                content_type: Some("application/cem+xml".to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig {
                cemt_formatter_options: BTreeMap::from([(
                    "lineEnding".to_owned(),
                    "crlf".to_owned(),
                )]),
                ..ScopeConfig::default()
            },
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "cem");
        assert_eq!(resp.primary["content"], "@doc cem-ml 1\r\n{p | Hi}\r\n");
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_target_cem_content_type_keeps_doc_directive_on_own_line() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
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
        assert_eq!(resp.primary["content"], "@doc cem-ml 1\n{p | Hi}\n");
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_target_cem_pretty_aligns_block_closing_braces_with_opening_indent() {
        let convert_with_indent_options = |cemt_formatter_options| {
            let req = ConvertRequest {
                input: input(
                    br#"@doc cem-ml 1

{article @id="welcome" |
    {h1 | Welcome}
    {p | This is a minimal CEM-ML document.}
}"#,
                    "basic.cem",
                ),
                to_format: LayerFormat::DomJson,
                preserve_source_offsets: false,
                context: ctx(),
                target: Some(FormatIdentity {
                    content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig {
                    cemt_formatter_profile: Some("pretty".to_owned()),
                    cemt_formatter_options,
                    ..ScopeConfig::default()
                },
                scheduler_scope_id: 0,
            };
            RealCemMlEngine::new().convert(req).unwrap()
        };

        let resp = convert_with_indent_options(BTreeMap::new());
        assert_eq!(
            resp.primary["content"].as_str().unwrap(),
            "@doc cem-ml 1\n{article @id=welcome |\n    {h1 |\n        Welcome\n    }\n    {p |\n        This is a minimal CEM-ML document.\n    }\n}\n"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );

        let resp =
            convert_with_indent_options(BTreeMap::from([("indent".to_owned(), "  ".to_owned())]));
        assert_eq!(
            resp.primary["content"].as_str().unwrap(),
            "@doc cem-ml 1\n{article @id=welcome |\n  {h1 |\n    Welcome\n  }\n  {p |\n    This is a minimal CEM-ML document.\n  }\n}\n"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_target_cem_pretty_wraps_attributes_at_wrap_column() {
        let req = ConvertRequest {
            input: input(
                br#"@doc cem-ml 1
{article @data-attr1=abc @data-attr2="long attr value" @data-attr3=abc @data-attr4=abc @data-attr5=abc @data-attr6=abc @id=welcome |
    {h1 | Welcome}
}"#,
                "long-attributes.cem",
            ),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
            target: Some(FormatIdentity {
                content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig {
                cemt_formatter_profile: Some("pretty".to_owned()),
                ..ScopeConfig::default()
            },
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert_eq!(
            resp.primary["content"].as_str().unwrap(),
            "@doc cem-ml 1\n{article @data-attr1=abc @data-attr2=\"long attr value\" @data-attr3=abc @data-attr4=abc\n    @data-attr5=abc @data-attr6=abc @id=welcome |\n    {h1 |\n        Welcome\n    }\n}\n"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_target_cem_tabular_wraps_attributes_only_after_wrap_column() {
        let convert_with_options = |cemt_formatter_options| {
            let req = ConvertRequest {
                input: input(
                    br#"@doc cem-ml 1
{article @data-tone=info @data-size=lg @id=welcome}"#,
                    "tabular-attributes.cem",
                ),
                to_format: LayerFormat::DomJson,
                preserve_source_offsets: false,
                context: ctx(),
                target: Some(FormatIdentity {
                    content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig {
                    cemt_formatter_profile: Some("tabular".to_owned()),
                    cemt_formatter_options,
                    ..ScopeConfig::default()
                },
                scheduler_scope_id: 0,
            };
            RealCemMlEngine::new().convert(req).unwrap()
        };

        let resp = convert_with_options(BTreeMap::new());
        assert_eq!(
            resp.primary["content"].as_str().unwrap(),
            "@doc cem-ml 1\n{article @data-size=lg @data-tone=info @id=welcome}\n"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );

        let resp =
            convert_with_options(BTreeMap::from([("wrapColumn".to_owned(), "48".to_owned())]));
        assert_eq!(
            resp.primary["content"].as_str().unwrap(),
            "@doc cem-ml 1\n{article @data-size=lg @data-tone=info\n    @id=welcome}\n"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn direct_cem_output_publishes_cemt_writer_failure_without_legacy_formatter() {
        let document = CemDocument {
            nodes: vec![
                crate::parser::CemAstNode::Document {
                    node_id: 0,
                    root_children: vec![1],
                    source: SourceMapStack::default(),
                },
                crate::parser::CemAstNode::Element {
                    node_id: 1,
                    expanded_name: crate::parser::ExpandedName {
                        namespace_uri: String::new(),
                        local_name: "1invalid".to_owned(),
                        schema_id: None,
                    },
                    attributes: Vec::new(),
                    children: Vec::new(),
                    has_explicit_boundary: true,
                    source: SourceMapStack::default(),
                },
            ],
            ..CemDocument::default()
        };

        let diagnostics = convert_primary_to_cem_with_cemt_pipeline(
            None,
            &document,
            InputFormat::Cem,
            &ScopeConfig::default(),
            Some("invalid.cem"),
        )
        .expect_err("invalid generated CEM tree names must fail through CEMT diagnostics");

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity.is_hard_violation()
                    && diagnostic.code
                        == crate::transform_template::TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_WRITER_ADAPTER_FAILED_CODE
                    && diagnostic.message.contains("CEM name cannot start with `1`")),
            "expected hard CEMT writer diagnostic, got: {:?}",
            diagnostics
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "cem.converter.cemt_fallback"),
            "direct CEM output must not publish converter fallback diagnostics: {:?}",
            diagnostics
        );
    }

    #[test]
    fn convert_html_layer_executes_packaged_cemt_dom_converter_pipeline() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: ctx(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "html", "{:?}", resp.diagnostics);
        assert_eq!(
            resp.primary["content"],
            "<p class=\"cem-color cem-color-syntax-name\" data-role=\"syntax.name\"><span class=\"cem-color cem-color-syntax-string\" data-role=\"syntax.string\">Hi</span></p>"
        );
        assert!(!resp.primary["content"].as_str().unwrap().contains("@doc"));
        let conversion = resp.conversion.as_ref().expect("conversion metadata");
        assert_eq!(
            conversion.converter_id.as_deref(),
            Some("cem-dom-projection-to-html-cemt")
        );
        assert_eq!(conversion.implementation.as_deref(), Some("cemt"));
        let stages = &conversion
            .output_pipeline
            .as_ref()
            .expect("conversion output pipeline")
            .stages;
        assert!(stages.iter().any(|stage| {
            stage.stage == "formatter" && stage.function.as_deref() == Some("cem.format-tree")
        }));
        assert!(stages.iter().any(|stage| {
            stage.stage == "colorizer"
                && stage.function.as_deref() == Some("cem.color-tree")
                && stage.profile.as_deref() == Some("classes")
        }));
        assert!(stages.iter().any(|stage| {
            stage.stage == "writer" && stage.content_type.as_deref() == Some(HTML_CONTENT_TYPE)
        }));
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_html_layer_with_cemt_output_selectors_keeps_packaged_converter_pipeline() {
        let req = ConvertRequest {
            input: input(
                b"@doc cem-ml 1\n{article | Ready {strong | now}.}",
                "in.cem",
            ),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: ctx(),
            target: None,
            target_scope: ScopeConfig {
                cemt_formatter: Some("acme.showcase.format-tree".to_owned()),
                cemt_formatter_profile: Some("acme.showcase.format-tree".to_owned()),
                cemt_colorizer: Some("acme.showcase.color-tree".to_owned()),
                cemt_color_profile: Some("classes".to_owned()),
                ..ScopeConfig::default()
            },
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert_eq!(resp.primary["kind"], "html", "{:?}", resp.diagnostics);
        let content = resp.primary["content"].as_str().expect("HTML content");
        assert!(content.contains(r#"<article class="cem-color cem-color-syntax-name""#));
        assert!(content.contains(r#"<strong class="cem-color cem-color-syntax-keyword""#));
        let conversion = resp.conversion.as_ref().expect("conversion metadata");
        assert_eq!(
            conversion.converter_id.as_deref(),
            Some("cem-dom-projection-to-html-cemt")
        );
        assert_eq!(conversion.implementation.as_deref(), Some("cemt"));
        let stages = &conversion
            .output_pipeline
            .as_ref()
            .expect("conversion output pipeline")
            .stages;
        assert!(stages.iter().any(|stage| {
            stage.stage == "formatter"
                && stage.function.as_deref() == Some("acme.showcase.format-tree")
        }));
        assert!(stages.iter().any(|stage| {
            stage.stage == "colorizer"
                && stage.function.as_deref() == Some("acme.showcase.color-tree")
                && stage.profile.as_deref() == Some("classes")
        }));
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_xml_layer_executes_packaged_cemt_dom_converter_pipeline() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p}", "in.cem"),
            to_format: LayerFormat::Xml,
            preserve_source_offsets: false,
            context: ctx(),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "xml");
        assert_eq!(resp.primary["content"], "<p/>");
        let conversion = resp.conversion.as_ref().expect("conversion metadata");
        assert_eq!(
            conversion.converter_id.as_deref(),
            Some("cem-dom-projection-to-xml-cemt")
        );
        assert_eq!(conversion.implementation.as_deref(), Some("cemt"));
        let stages = &conversion
            .output_pipeline
            .as_ref()
            .expect("conversion output pipeline")
            .stages;
        assert!(stages.iter().any(|stage| {
            stage.stage == "writer" && stage.content_type.as_deref() == Some(XML_CONTENT_TYPE)
        }));
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_html_executes_ready_cemt_converter_template() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: ready_cemt_html_export_context(
                "cem+test://converters/dom-to-html.cemt",
                b"{module | {body | {p | Ready}}}",
            ),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert_eq!(resp.primary["kind"], "html", "{:?}", resp.diagnostics);
        assert_eq!(
            resp.primary["content"],
            "<cemt-ready class=\"cem-color cem-color-syntax-name\" data-role=\"syntax.name\"><span class=\"cem-color cem-color-syntax-string\" data-role=\"syntax.string\">document</span></cemt-ready>"
        );
        assert_ne!(resp.primary["sourceMap"], Value::Null);
        assert_eq!(
            resp.primary["outputSpans"]
                .as_array()
                .expect("output spans array")
                .len(),
            1
        );
        let conversion = resp.conversion.as_ref().expect("conversion metadata");
        assert_eq!(
            conversion.converter_id.as_deref(),
            Some("test-dom-to-html-cemt-ready")
        );
        assert_eq!(conversion.implementation.as_deref(), Some("cemt"));
        let stages = &conversion
            .output_pipeline
            .as_ref()
            .expect("conversion output pipeline")
            .stages;
        assert!(stages.iter().any(|stage| {
            stage.stage == "formatter" && stage.function.as_deref() == Some("cem.format-tree")
        }));
        assert!(stages.iter().any(|stage| {
            stage.stage == "colorizer"
                && stage.function.as_deref() == Some("cem.color-tree")
                && stage.profile.as_deref() == Some("classes")
        }));
        assert!(stages.iter().any(|stage| {
            stage.stage == "writer" && stage.content_type.as_deref() == Some(HTML_CONTENT_TYPE)
        }));
        assert!(!resp
            .diagnostics
            .iter()
            .any(|diag| diag.code == "cem.converter.cemt_fallback"));
    }

    #[test]
    fn convert_html_publishes_ready_cemt_compile_failure_without_rust_fallback() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: ready_cemt_html_export_context(
                "cem+test://converters/dom-to-html.cemt",
                b"{module @name=adapter-compile-fail}",
            ),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert_eq!(resp.primary, Value::Null);
        assert!(resp.diagnostics.iter().any(|diag| {
            diag.code == TransformTemplateAdapterError::FAILED_CODE
                && diag.severity == Severity::Fatal
                && diag.message.contains("adapter compile sentinel")
        }));
        assert!(
            resp.diagnostics
                .iter()
                .all(|diag| diag.code != "cem.converter.cemt_fallback"),
            "ready CEMT compile failure must not use Rust fallback: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_html_publishes_ready_cemt_render_failure_without_rust_fallback() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: ready_cemt_html_export_context(
                "cem+test://converters/dom-to-html.cemt",
                b"{module @name=adapter-render-fail}",
            ),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert_eq!(resp.primary, Value::Null);
        assert!(resp.diagnostics.iter().any(|diag| {
            diag.code == TransformTemplateAdapterError::FAILED_CODE
                && diag.severity == Severity::Fatal
                && diag.message.contains("adapter render sentinel")
        }));
        assert!(
            resp.diagnostics
                .iter()
                .all(|diag| diag.code != "cem.converter.cemt_fallback"),
            "ready CEMT render failure must not use Rust fallback: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_html_publishes_ready_cemt_output_pipeline_failure_without_rust_fallback() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: ready_cemt_html_export_context(
                "cem+test://converters/dom-to-html.cemt",
                b"{module @name=adapter-output-pipeline-fail}",
            ),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert_eq!(resp.primary, Value::Null);
        assert!(
            resp.diagnostics.iter().any(|diag| {
                diag.code
                    == crate::transform_template::TRANSFORM_TEMPLATE_ENCODED_ARTIFACT_VALUE_TYPE_CODE
                    && diag.severity.is_hard_violation()
                    && diag
                        .message
                        .contains("expected object with `kind: \"cem-tree\"`")
                    && diag.message.contains("got `root` string")
            }),
            "{:?}",
            resp.diagnostics
        );
        assert!(
            resp.diagnostics
                .iter()
                .all(|diag| diag.code != "cem.converter.cemt_fallback"),
            "ready CEMT output pipeline failure must not use Rust fallback: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_html_falls_back_to_rust_when_ready_cemt_asset_is_unreadable() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: ready_cemt_html_export_context(
                "cem+missing://converters/dom-to-html.cemt",
                b"{module | {body | {p | Ready}}}",
            ),
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert_eq!(resp.primary["kind"], "html");
        assert_eq!(resp.primary["content"], "<p>Hi</p>");
        assert!(resp.diagnostics.iter().any(|diag| {
            diag.code == "cem.converter.cemt_fallback" && diag.severity == Severity::Warning
        }));
    }

    #[test]
    fn convert_target_html_content_type_selects_html_export() {
        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{p | Hi}", "in.cem"),
            to_format: LayerFormat::DomJson,
            preserve_source_offsets: false,
            context: ctx(),
            target: Some(FormatIdentity {
                content_type: Some("text/html".to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };
        let resp = RealCemMlEngine::new().convert(req).unwrap();
        assert_eq!(resp.primary["kind"], "html");
        assert_eq!(
            resp.primary["content"],
            "<p class=\"cem-color cem-color-syntax-name\" data-role=\"syntax.name\"><span class=\"cem-color cem-color-syntax-string\" data-role=\"syntax.string\">Hi</span></p>"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_html_direct_pipeline_reads_cemt_package_artifacts_through_template_resolver() {
        let formatter_path = "cem+test://packages/cem-ml/v1/formatters/cem-format-tree.cemt";
        let formatter_helper_path =
            "cem+test://packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt";
        let colorizer_path = "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree.cemt";
        let colorizer_helper_path =
            "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt";
        let formatter_source =
            crate::schema::package_sources::builtin_schema_package_artifact_source(
                "cem-ml",
                "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt",
            )
            .expect("embedded formatter source");
        let formatter_helper_source =
            crate::schema::package_sources::builtin_schema_package_artifact_source(
                "cem-ml",
                "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt",
            )
            .expect("embedded formatter helper source");
        let colorizer_source =
            crate::schema::package_sources::builtin_schema_package_artifact_source(
                "cem-ml",
                "schema-packages/cem-ml/v1/colorizers/cem-color-tree.cemt",
            )
            .expect("embedded colorizer source");
        let colorizer_helper_source =
            crate::schema::package_sources::builtin_schema_package_artifact_source(
                "cem-ml",
                "schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt",
            )
            .expect("embedded colorizer helper source");

        let mut converter_registry = ConversionRegistry::new();
        converter_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "formatter".to_owned(),
            path: formatter_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(crate::schema::registry::CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(crate::schema::registry::CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.format-tree".to_owned()),
            function_profile: None,
            formatter_profile: Some("compact".to_owned()),
            color_profile: None,
            generated: false,
        });
        converter_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "formatter-helper".to_owned(),
            path: formatter_helper_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(crate::schema::registry::CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(crate::schema::registry::CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.format-tree.apply-stage".to_owned()),
            function_profile: Some("cem.format-tree".to_owned()),
            formatter_profile: Some("compact".to_owned()),
            color_profile: None,
            generated: false,
        });
        converter_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "colorizer".to_owned(),
            path: colorizer_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(crate::schema::registry::CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(crate::schema::registry::CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.color-tree".to_owned()),
            function_profile: Some("css-custom-properties".to_owned()),
            formatter_profile: None,
            color_profile: Some("classes".to_owned()),
            generated: false,
        });
        converter_registry.register_package_artifact(ConversionPackageArtifactDescriptor {
            package_id: "cem-ml".to_owned(),
            kind: "colorizer-helper".to_owned(),
            path: colorizer_helper_path.to_owned(),
            content_type: Some(CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            target_content_type: Some(crate::schema::registry::CEM_ML_CONTENT_TYPE.to_owned()),
            target_schema: Some(crate::schema::registry::CEM_ML_SCHEMA_URI.to_owned()),
            target_category: Some("cem-tree".to_owned()),
            function_name: Some("cem.color-tree.apply-stage".to_owned()),
            function_profile: Some("css-custom-properties".to_owned()),
            formatter_profile: None,
            color_profile: None,
            generated: false,
        });
        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(
            "cem+test",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            MapReadResolver {
                entries: vec![
                    (
                        formatter_path,
                        formatter_source.source.as_bytes(),
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                    (
                        formatter_helper_path,
                        formatter_helper_source.source.as_bytes(),
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                    (
                        colorizer_path,
                        colorizer_source.source.as_bytes(),
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                    (
                        colorizer_helper_path,
                        colorizer_helper_source.source.as_bytes(),
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                ],
            },
        );

        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{main | Ready}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: EngineContext {
                converter_registry,
                resolver_registry,
                ..ctx()
            },
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert_eq!(resp.primary["kind"], "html");
        assert_eq!(
            resp.primary["content"],
            "<main class=\"cem-color cem-color-syntax-name\" data-role=\"syntax.name\"><span class=\"cem-color cem-color-syntax-string\" data-role=\"syntax.string\">Ready</span></main>"
        );
        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
    }

    #[test]
    fn convert_loads_context_schema_package_manifest_artifacts_before_output_pipeline() {
        const PACKAGE_URI: &str = "cem+test://packages/cem-ml/v1/package.cem";
        const SCHEMA_URI: &str = "cem+test://packages/cem-ml/v1/schema/cem-ml-generic.cem";
        const FORMATTER_URI: &str = "cem+test://packages/cem-ml/v1/formatters/cem-format-tree.cemt";
        const FORMATTER_HELPER_URI: &str =
            "cem+test://packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt";
        const COLORIZER_URI: &str = "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree.cemt";
        const COLORIZER_HELPER_URI: &str =
            "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt";
        const PACKAGE_MANIFEST: &[u8] = br#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="cem-ml" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/cem-ml/1" @source="schema/cem-ml-generic.cem"}
    {content-type @value="application/cem" @primary=true}
    {content-type @value="text/cem-ml" @alias=true}
    {content-type @value="text/cem" @alias=true}
    {content-type @value="application/cem+xml" @alias=true}
    {namespace @prefix="cemml" @uri="https://cem.dev/ns/cem-ml/1"}
    {namespace @prefix="schema" @uri="https://cem.dev/ns/schema/1"}
    {artifact
        @kind="formatter"
        @path="formatters/cem-format-tree.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="cem.format-tree"
        @formatter-profile="compact"
    }
    {artifact
        @kind="formatter-helper"
        @path="formatters/cem-format-tree-helpers.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="cem.format-tree.apply-stage"
        @function-profile="cem.format-tree"
        @formatter-profile="compact"
    }
    {artifact
        @kind="colorizer"
        @path="colorizers/cem-color-tree.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="cem.color-tree"
        @function-profile="css-custom-properties"
        @color-profile="classes"
    }
    {artifact
        @kind="colorizer-helper"
        @path="colorizers/cem-color-tree-helpers.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="cem.color-tree.apply-stage"
        @function-profile="css-custom-properties"
    }
}
"#;
        let schema_source = crate::schema::package_sources::builtin_schema_package_source("cem-ml")
            .expect("embedded schema source");
        let formatter_source =
            crate::schema::package_sources::builtin_schema_package_artifact_source(
                "cem-ml",
                "schema-packages/cem-ml/v1/formatters/cem-format-tree.cemt",
            )
            .expect("embedded formatter source");
        let formatter_helper_source =
            crate::schema::package_sources::builtin_schema_package_artifact_source(
                "cem-ml",
                "schema-packages/cem-ml/v1/formatters/cem-format-tree-helpers.cemt",
            )
            .expect("embedded formatter helper source");
        let colorizer_source =
            crate::schema::package_sources::builtin_schema_package_artifact_source(
                "cem-ml",
                "schema-packages/cem-ml/v1/colorizers/cem-color-tree.cemt",
            )
            .expect("embedded colorizer source");
        let colorizer_helper_source =
            crate::schema::package_sources::builtin_schema_package_artifact_source(
                "cem-ml",
                "schema-packages/cem-ml/v1/colorizers/cem-color-tree-helpers.cemt",
            )
            .expect("embedded colorizer helper source");
        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(
            "cem+test",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            MapReadResolver {
                entries: vec![
                    (
                        SCHEMA_URI,
                        schema_source.schema_source.as_bytes(),
                        Some(CEM_SCHEMA_CONTENT_TYPE),
                    ),
                    (
                        FORMATTER_URI,
                        formatter_source.source.as_bytes(),
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                    (
                        FORMATTER_HELPER_URI,
                        formatter_helper_source.source.as_bytes(),
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                    (
                        COLORIZER_URI,
                        colorizer_source.source.as_bytes(),
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                    (
                        COLORIZER_HELPER_URI,
                        colorizer_helper_source.source.as_bytes(),
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                ],
            },
        );

        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{main | Ready}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: EngineContext {
                schema_package_manifests: vec![EngineInput {
                    uri: PACKAGE_URI.to_owned(),
                    bytes: PACKAGE_MANIFEST.to_vec(),
                    from_format: Some(InputFormat::Cem),
                    identity: Some(schema_package_manifest_identity()),
                    root_scope: Default::default(),
                }],
                converter_registry: ConversionRegistry::new(),
                resolver_registry,
                ..ctx()
            },
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
        assert_eq!(resp.primary["kind"], "html");
        assert_eq!(
            resp.primary["content"],
            "<main class=\"cem-color cem-color-syntax-name\" data-role=\"syntax.name\"><span class=\"cem-color cem-color-syntax-string\" data-role=\"syntax.string\">Ready</span></main>"
        );
    }

    #[test]
    fn convert_prefers_context_schema_package_output_artifacts_over_builtins() {
        const PACKAGE_URI: &str = "cem+test://packages/cem-ml/v1/package.cem";
        const SCHEMA_URI: &str = "cem+test://packages/cem-ml/v1/schema/cem-ml-generic.cem";
        const FORMATTER_URI: &str = "cem+test://packages/cem-ml/v1/formatters/cem-format-tree.cemt";
        const COLORIZER_URI: &str = "cem+test://packages/cem-ml/v1/colorizers/cem-color-tree.cemt";
        const PACKAGE_MANIFEST: &[u8] = br#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="cem-ml" @version="1.0.0" |
    {schema @uri="https://cem.dev/ns/cem-ml/1" @source="schema/cem-ml-generic.cem"}
    {content-type @value="application/cem" @primary=true}
    {content-type @value="text/cem-ml" @alias=true}
    {content-type @value="text/cem" @alias=true}
    {content-type @value="application/cem+xml" @alias=true}
    {namespace @prefix="cemml" @uri="https://cem.dev/ns/cem-ml/1"}
    {namespace @prefix="schema" @uri="https://cem.dev/ns/schema/1"}
    {artifact
        @kind="formatter"
        @path="formatters/cem-format-tree.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="cem.format-tree"
        @formatter-profile="compact"
    }
    {artifact
        @kind="colorizer"
        @path="colorizers/cem-color-tree.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="cem.color-tree"
        @function-profile="css-custom-properties"
        @color-profile="classes"
    }
}
"#;
        let schema_source = crate::schema::package_sources::builtin_schema_package_source("cem-ml")
            .expect("embedded schema source");
        const FORMATTER_SOURCE: &[u8] = br#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="cem.format-tree"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="json" @required=true}
        {body |
            {$ {
                kind: "cem-tree",
                contentType: "application/cem",
                schema: "https://cem.dev/ns/cem-ml/1",
                category: "cem-tree",
                mode: "fragment",
                canonical: true,
                formatterProfile: "compact",
                formatNodes: [
                    {
                        kind: "format-marker",
                        name: "cem.format-tree",
                        formatterRole: "formatter.boundary",
                        formatterProfile: "compact"
                    },
                    {
                        kind: "format-decision",
                        name: "external-formatter",
                        formatterRole: "formatter.external-override",
                        formatterProfile: "compact"
                    }
                ],
                nodes: [{
                    kind: "element",
                    name: "external-widget",
                    children: []
                }]
            } }
        }
    }
}
"#;
        const COLORIZER_SOURCE: &[u8] = br#"@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {color-function
        @name="cem.color-tree"
        @category="cem-tree"
        @subject="cem-tree"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @profile="css-custom-properties"
        @canonical=false
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {body |
            {$ {
                kind: $subject.kind,
                contentType: $subject.contentType,
                schema: $subject.schema,
                category: $subject.category,
                mode: $subject.mode,
                canonical: $subject.canonical,
                formatterProfile: $subject.formatterProfile,
                formatNodes: $subject.formatNodes,
                colored: true,
                colorProfile: "classes",
                colorNodes: [
                    {
                        kind: "color-marker",
                        name: "cem.color-tree",
                        colorizerRole: "colorizer.boundary",
                        colorProfile: "classes"
                    },
                    {
                        kind: "color-decision",
                        name: "external-colorizer",
                        colorizerRole: "colorizer.external-override",
                        colorProfile: "classes"
                    }
                ],
                nodes: map($subject.nodes, match($item.kind, {
                    element: {
                        kind: $item.kind,
                        name: "external-widget",
                        writerAttributeNodes: [
                            {
                                kind: "writer-attribute",
                                name: "class",
                                value: "external-package-color",
                                colorProfile: "classes",
                                colorizerOwned: true,
                                colorizerRole: "colorizer.writer-attribute"
                            },
                            {
                                kind: "writer-attribute",
                                name: "data-package-stage",
                                value: "external-cemt",
                                colorProfile: "classes",
                                colorizerOwned: true,
                                colorizerRole: "colorizer.writer-attribute"
                            }
                        ],
                        children: []
                    },
                    default: $item
                }))
            } }
        }
    }
}
"#;
        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(
            "cem+test",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            MapReadResolver {
                entries: vec![
                    (
                        SCHEMA_URI,
                        schema_source.schema_source.as_bytes(),
                        Some(CEM_SCHEMA_CONTENT_TYPE),
                    ),
                    (
                        FORMATTER_URI,
                        FORMATTER_SOURCE,
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                    (
                        COLORIZER_URI,
                        COLORIZER_SOURCE,
                        Some(CEM_TRANSFORM_CONTENT_TYPE),
                    ),
                ],
            },
        );

        let req = ConvertRequest {
            input: input(b"@doc cem-ml 1\n{main}", "in.cem"),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: EngineContext {
                schema_package_manifests: vec![EngineInput {
                    uri: PACKAGE_URI.to_owned(),
                    bytes: PACKAGE_MANIFEST.to_vec(),
                    from_format: Some(InputFormat::Cem),
                    identity: Some(schema_package_manifest_identity()),
                    root_scope: Default::default(),
                }],
                resolver_registry,
                ..ctx()
            },
            target: None,
            target_scope: Default::default(),
            scheduler_scope_id: 0,
        };

        let resp = RealCemMlEngine::new().convert(req).unwrap();

        assert!(
            resp.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            resp.diagnostics
        );
        assert_eq!(resp.primary["kind"], "html");
        assert_eq!(
            resp.primary["content"],
            r#"<external-widget class="external-package-color" data-package-stage="external-cemt"></external-widget>"#
        );
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
        let parser_stages = &resp.body["report"]["reportAst"]["parserStages"];
        assert_eq!(parser_stages["stageCount"], 5);
        assert_eq!(parser_stages["stages"][0]["input"], "in.cem");
        assert_eq!(parser_stages["stages"][0]["scopeId"], 0);
        assert_eq!(parser_stages["stages"][0]["stage"], "tokenize");
        assert_eq!(
            parser_stages["stages"][0]["sourceMapBoundary"]["transform"],
            "cem-tokenizer"
        );
        assert_eq!(
            parser_stages["stages"][4]["sourceMapBoundary"]["transform"],
            "validation-rules"
        );
        assert!(parser_stages["stages"][0]["diagnostics"].is_array());
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
    fn fixture_validate_reads_local_file_uri_paths_from_disk() {
        let path = std::env::temp_dir().join(format!(
            "cem-ml-fixture-file-uri-{}.cem",
            std::process::id()
        ));
        std::fs::write(&path, "{p Hi}").unwrap();
        let uri = format!("file://{}", path.display());
        let req = FixtureValidateRequest {
            inputs: vec![EngineInput {
                uri: uri.clone(),
                bytes: Vec::new(),
                from_format: None,
                identity: None,
                root_scope: Default::default(),
            }],
            fail_level: FailLevel::Validate,
            zero_hard_violations: true,
            context: ctx(),
        };

        let resp = RealCemMlEngine::new().fixture_validate(req).unwrap();
        let _ = std::fs::remove_file(path);

        assert_eq!(resp.report.summary.hard_violation_count, 0);
        assert_eq!(resp.report.summary.input_count, 1);
        assert_eq!(resp.report.inputs[0], uri);
    }

    #[test]
    fn fixture_validate_rejects_remote_input_uri_without_resolver() {
        let req = FixtureValidateRequest {
            inputs: vec![EngineInput {
                uri: "https://example.test/fixture.cem".to_owned(),
                bytes: Vec::new(),
                from_format: Some(InputFormat::Cem),
                identity: None,
                root_scope: Default::default(),
            }],
            fail_level: FailLevel::Validate,
            zero_hard_violations: true,
            context: ctx(),
        };

        let err = RealCemMlEngine::new().fixture_validate(req).unwrap_err();

        match err {
            EngineError::Io { source, .. } => {
                assert_eq!(source.kind(), std::io::ErrorKind::InvalidInput);
                assert!(source
                    .to_string()
                    .contains("remote/custom input URI resolvers are not implemented"));
            }
            other => panic!("expected EngineError::Io, got {other:?}"),
        }
    }

    #[test]
    fn fixture_validate_reads_custom_resolver_input_uri() {
        let req = FixtureValidateRequest {
            inputs: vec![EngineInput {
                uri: "cem+vfs://fixtures/login.cem".to_owned(),
                bytes: Vec::new(),
                from_format: Some(InputFormat::Cem),
                identity: None,
                root_scope: Default::default(),
            }],
            fail_level: FailLevel::Validate,
            zero_hard_violations: true,
            context: context_with_resolver(
                "cem+vfs",
                ResolvePurpose::Input,
                StaticReadResolver {
                    resolved_uri: "cem+vfs://fixtures/login.cem",
                    bytes: b"{main | Loaded}",
                    content_type: Some("application/cem+xml"),
                },
            ),
        };

        let resp = RealCemMlEngine::new().fixture_validate(req).unwrap();

        assert_eq!(resp.report.summary.input_count, 1);
        assert_eq!(resp.report.summary.hard_violation_count, 0);
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
