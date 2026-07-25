//! CEM-native transform-template adapter backed by `cem_ql::render`.
//!
//! This crate intentionally sits above both `cem_ml` and `cem_ql`: `cem_ml`
//! owns the stable transform adapter contract, while `cem_ql` owns the current
//! CEM-native fragment renderer. Keeping the bridge here avoids a dependency
//! cycle from `cem_ml` back into `cem_ql`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use cem_ml::conversion::{
    execute_conversion_output_pipeline_with_environment, ConversionOutputPipeline,
    ConversionOutputPipelineEnvironment, ConversionOutputPipelineStage,
};
use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::engine::{
    ConvertExecutionMetadata, ConvertOutputPipelineMetadata, ConvertOutputPipelineStageMetadata,
    ConvertRequest, ConvertRequestHandler, ConvertResponse, EngineContext, FormatIdentity,
    LayerFormat, PrimaryBytes, TemplateInput, TransformTemplateEntrypoint, TransformTemplateKind,
    TRANSFORM_TEMPLATE_UNSUPPORTED_CODE,
};
use cem_ml::interpreter::OutputSpan;
use cem_ml::legacy_custom_element::{
    convert_template_source, LegacyConversionDiagnostic, TEMPLATE_CONTENT_TYPES,
    UNSUPPORTED_CONSTRUCT_CODE, UNSUPPORTED_FUNCTION_CODE,
};
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::{AstNodeId, CemAstNode};
use cem_ml::run_config::ScopeConfig;
use cem_ml::scheduler::ScopePolicy;
use cem_ml::schema::document_model::{
    BehaviorArgument, BehaviorDefinition, BehaviorFunctionParam, BehaviorParameter,
    DiagnosticBehavior, SchemaBehaviorEvaluator, SchemaDocumentModel,
    SCHEMA_BEHAVIOR_FUNCTION_FAILED_CODE, SCHEMA_BEHAVIOR_QUERY_FAILED_CODE,
    SCHEMA_BEHAVIOR_QUERY_INVALID_CODE, SCHEMA_BEHAVIOR_RESULT_INVALID_CODE,
};
use cem_ml::schema::registry::{
    CEM_ML_CONTENT_TYPE, CEM_ML_SCHEMA_URI, CEM_QL_CONTENT_TYPE, CEM_QL_SCHEMA_URI,
    HTML_CONTENT_TYPE, HTML_SCHEMA_URI,
};
use cem_ml::source::{ByteRange, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use cem_ml::transform_template::{
    parse_cem_native_template_module_options, parse_transform_template_output_color_type,
    TransformTemplateAdapter, TransformTemplateAdapterCapability, TransformTemplateAdapterError,
    TransformTemplateAdapterExecutionPhase, TransformTemplateAdapterRegistry,
    TransformTemplateAdapterResult, TransformTemplateCompileRequest,
    TransformTemplateCompileResponse, TransformTemplateCompiledArtifact,
    TransformTemplateDataArtifact, TransformTemplateEncodeOptions,
    TransformTemplateEncodedArtifactInsertionContext, TransformTemplateEncodedArtifactMode,
    TransformTemplateEncodingTarget, TransformTemplateModuleOptions,
    TransformTemplateModuleParamDeclaration, TransformTemplateModuleParseRequest,
    TransformTemplateModulePreflight, TransformTemplateOutputArtifact,
    TransformTemplateOutputColorSelection, TransformTemplateOutputProducedKind,
    TransformTemplateRenderRequest, TransformTemplateRenderResponse,
    TransformTemplateSourceMapPolicy, CEM_NATIVE_TEMPLATE_SCHEMA_URI,
    TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE, TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE,
    TRANSFORM_TEMPLATE_PARAM_TYPE_CODE, TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE,
};
use cem_ql::api::{
    compile, compile_expression, evaluate, CompileContext, CompiledExpression, EvaluationContext,
    ParseResult, StandaloneExpressionBinding, StandaloneExpressionContext,
};
use cem_ql::eval::{AtomValue, BudgetAxis, EvalError, Item, ItemStream, QueryContextScope};
use cem_ql::lexer::{CookedTokenPayload, Lexer, Token, TokenKind};
use cem_ql::parser::SurfaceNode;
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html_with_source_map,
    render_plan_to_xml_with_source_map, CompileTemplateOptions, RenderPlan, RenderPlanAttribute,
    RenderPlanNode, TemplateArtifact, TemplateAttributeValue, TemplateData, TemplateNode,
};
use cem_ql::types::Type;
use serde_json::{json, Map, Number, Value};

pub const CEM_QL_TEMPLATE_ADAPTER_ID: &str = "cem-ql-cem-native-template";
pub const CEM_QL_EXPRESSION_TEMPLATE_ADAPTER_ID: &str = "cem-ql-expression-template";
pub const XSLT_PARITY_TEMPLATE_ADAPTER_ID: &str = "cem-ql-xslt-parity-template";
const TRANSFORM_CALL_NODE: &str = "__cem_transform_call";

#[derive(Debug, Clone, Default)]
pub struct CemQlTransformTemplateAdapter;

#[derive(Debug, Clone, Default)]
pub struct CemQlExpressionTransformTemplateAdapter;

#[derive(Debug, Clone, Default)]
pub struct XsltParityTransformTemplateAdapter;

#[derive(Debug, Clone, Default)]
pub struct CemQlSchemaBehaviorEvaluator;

#[derive(Debug, Clone)]
struct SchemaBehaviorCandidate {
    node_id: AstNodeId,
    element: String,
    attributes: BTreeMap<String, String>,
    source_map: SourceMapStack,
}

impl SchemaBehaviorEvaluator for CemQlSchemaBehaviorEvaluator {
    fn compile_model(&self, model: &SchemaDocumentModel) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let select_bindings = select_binding_names(model);
        let match_bindings = match_binding_names(model, &[]);
        for diagnostic in model
            .diagnostic_behaviors
            .values()
            .filter(|diagnostic| diagnostic.function.is_some())
        {
            let Some(definition) = diagnostic.definition.as_ref() else {
                continue;
            };
            if let Some(select) = definition.select.as_deref() {
                if let Err(message) = compile_cem_ql_behavior_query(select, &select_bindings) {
                    diagnostics.push(schema_behavior_diagnostic(
                        SCHEMA_BEHAVIOR_QUERY_INVALID_CODE,
                        Severity::Error,
                        format!(
                            "diagnostic `{}` behavior `{}` has invalid CEM-QL select expression: {message}",
                            diagnostic.code, diagnostic.behavior
                        ),
                        &definition.source_map,
                        json!({
                            "schemaUri": model.schema_uri,
                            "diagnostic": diagnostic.code,
                            "behavior": diagnostic.behavior,
                            "queryKind": "select",
                            "query": select,
                        }),
                    ));
                }
            }
            if let Some(match_query) = definition.match_query.as_deref() {
                if let Err(message) = compile_cem_ql_behavior_query(match_query, &match_bindings) {
                    diagnostics.push(schema_behavior_diagnostic(
                        SCHEMA_BEHAVIOR_QUERY_INVALID_CODE,
                        Severity::Error,
                        format!(
                            "diagnostic `{}` behavior `{}` has invalid CEM-QL match expression: {message}",
                            diagnostic.code, diagnostic.behavior
                        ),
                        &definition.source_map,
                        json!({
                            "schemaUri": model.schema_uri,
                            "diagnostic": diagnostic.code,
                            "behavior": diagnostic.behavior,
                            "queryKind": "match",
                            "query": match_query,
                        }),
                    ));
                }
            }
            for parameter in &definition.parameters {
                if let Err(message) = behavior_parameter_default_value(parameter).map(|_| ()) {
                    diagnostics.push(schema_behavior_diagnostic(
                        SCHEMA_BEHAVIOR_RESULT_INVALID_CODE,
                        Severity::Error,
                        format!(
                            "diagnostic `{}` behavior `{}` has invalid default for behavior parameter `{}`: {message}",
                            diagnostic.code, diagnostic.behavior, parameter.name
                        ),
                        &definition.source_map,
                        json!({
                            "schemaUri": model.schema_uri,
                            "diagnostic": diagnostic.code,
                            "behavior": diagnostic.behavior,
                            "parameter": parameter.name,
                            "parameterType": parameter.value_type,
                        }),
                    ));
                }
            }
            if let Some(function_name) = diagnostic.function.as_deref() {
                if let Some(function) = diagnostic_behavior_function(diagnostic, definition) {
                    if let Some(body) = function.body_expression.as_deref() {
                        if let Err(message) = parse_schema_behavior_expression(body) {
                            diagnostics.push(schema_behavior_diagnostic(
                                SCHEMA_BEHAVIOR_FUNCTION_FAILED_CODE,
                                Severity::Error,
                                format!(
                                    "diagnostic `{}` behavior `{}` function `{function_name}` has invalid CEM-ML behavior body: {message}",
                                    diagnostic.code, diagnostic.behavior
                                ),
                                &definition.source_map,
                                json!({
                                    "schemaUri": model.schema_uri,
                                    "diagnostic": diagnostic.code,
                                    "behavior": diagnostic.behavior,
                                    "function": function_name,
                                }),
                            ));
                        }
                    }
                }
            }
        }
        diagnostics
    }

    fn validate_document(
        &self,
        document: &CemDocument,
        model: &SchemaDocumentModel,
    ) -> Vec<Diagnostic> {
        let candidates = collect_schema_behavior_candidates(document);
        let mut diagnostics = Vec::new();
        for diagnostic in model
            .diagnostic_behaviors
            .values()
            .filter(|diagnostic| diagnostic.function.is_some())
        {
            let Some(definition) = diagnostic.definition.as_ref() else {
                continue;
            };
            let Some(select) = definition.select.as_deref() else {
                continue;
            };
            let Some(match_query) = definition.match_query.as_deref() else {
                continue;
            };
            let select_binding_names = select_binding_names(model);
            let match_bindings = match_binding_names(model, &candidates);
            if compile_cem_ql_behavior_query(select, &select_binding_names).is_err()
                || compile_cem_ql_behavior_query(match_query, &match_bindings).is_err()
            {
                continue;
            }
            let selected = match evaluate_cem_ql_behavior_query(
                select,
                &select_binding_names,
                select_bindings(&candidates, model),
            ) {
                Ok(stream) => selected_candidate_ids(&stream),
                Err(message) => {
                    diagnostics.push(schema_behavior_diagnostic(
                        SCHEMA_BEHAVIOR_QUERY_FAILED_CODE,
                        Severity::Error,
                        format!(
                            "diagnostic `{}` behavior `{}` failed while evaluating CEM-QL select expression: {message}",
                            diagnostic.code, diagnostic.behavior
                        ),
                        &definition.source_map,
                        json!({
                            "schemaUri": model.schema_uri,
                            "diagnostic": diagnostic.code,
                            "behavior": diagnostic.behavior,
                            "queryKind": "select",
                            "query": select,
                        }),
                    ));
                    continue;
                }
            };
            for candidate in candidates
                .iter()
                .filter(|candidate| selected.contains(&candidate.node_id))
            {
                let matched = match evaluate_cem_ql_behavior_query(
                    match_query,
                    &match_bindings,
                    candidate_match_bindings(candidate, &match_bindings),
                ) {
                    Ok(stream) => stream_truthy(&stream),
                    Err(message) => {
                        diagnostics.push(schema_behavior_diagnostic(
                            SCHEMA_BEHAVIOR_QUERY_FAILED_CODE,
                            Severity::Error,
                            format!(
                                "diagnostic `{}` behavior `{}` failed while evaluating CEM-QL match expression: {message}",
                                diagnostic.code, diagnostic.behavior
                            ),
                            &candidate.source_map,
                            json!({
                                "schemaUri": model.schema_uri,
                                "diagnostic": diagnostic.code,
                                "behavior": diagnostic.behavior,
                                "queryKind": "match",
                                "query": match_query,
                                "candidate": candidate_json(candidate),
                            }),
                        ));
                        false
                    }
                };
                if !matched {
                    continue;
                }
                if let Some(diagnostic_result) =
                    execute_schema_behavior_function(model, diagnostic, definition, candidate)
                {
                    diagnostics.push(diagnostic_result);
                }
            }
        }
        diagnostics
    }
}

#[derive(Debug, Clone)]
struct CemQlCompiledTemplatePayload {
    template_uri: String,
    artifact: TemplateArtifact,
    selected_entrypoint: Option<String>,
    params: BTreeMap<String, Value>,
    param_declarations: Vec<TransformTemplateModuleParamDeclaration>,
    entrypoints: CemQlTemplateEntrypoints,
    modules: Vec<CemQlCompiledTemplateModulePayload>,
    max_recursion_depth: u32,
}

#[derive(Debug, Clone)]
struct CemQlCompiledExpressionPayload {
    template_uri: String,
    compiled: CompiledExpression,
    params: BTreeMap<String, Value>,
}

#[derive(Debug, Clone)]
struct CemQlCompiledTemplateModulePayload {
    alias: String,
    parent_uri: Option<String>,
    uri: String,
    content_hash: String,
    artifact: TemplateArtifact,
    entrypoints: CemQlTemplateEntrypoints,
    imports: BTreeMap<String, String>,
    param_declarations: Vec<TransformTemplateModuleParamDeclaration>,
}

#[derive(Debug, Clone, Default)]
struct CemQlTemplateEntrypoints {
    implicit: Option<TemplateArtifact>,
    named: BTreeMap<String, TemplateArtifact>,
}

fn compile_cem_ql_behavior_query(
    source: &str,
    binding_names: &BTreeSet<String>,
) -> Result<(), String> {
    let policy_bindings = binding_names
        .iter()
        .cloned()
        .map(|name| (name, ItemStream::empty()))
        .collect();
    compile(
        source,
        &CompileContext {
            policy_bindings,
            ..CompileContext::default()
        },
    )
    .map(|_| ())
    .map_err(|err| err.to_string())
}

fn evaluate_cem_ql_behavior_query(
    source: &str,
    binding_names: &BTreeSet<String>,
    bindings: BTreeMap<String, ItemStream>,
) -> Result<ItemStream, String> {
    let compile_bindings = binding_names
        .iter()
        .cloned()
        .map(|name| (name, ItemStream::empty()))
        .collect();
    let query = compile(
        source,
        &CompileContext {
            policy_bindings: compile_bindings,
            ..CompileContext::default()
        },
    )
    .map_err(|err| err.to_string())?;
    let stream = evaluate(
        &query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root(),
            diagnostics: Vec::new(),
            policy_bindings: bindings,
            current_item: None,
        },
    );
    if let Some(error) = stream.error.as_ref() {
        return Err(format!("{error:?}"));
    }
    if let Some(diagnostic) = stream
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
    }
    Ok(stream)
}

fn select_binding_names(model: &SchemaDocumentModel) -> BTreeSet<String> {
    let mut bindings = BTreeSet::from(["nodes".to_owned()]);
    bindings.extend(model.elements.keys().cloned());
    bindings
}

fn match_binding_names(
    model: &SchemaDocumentModel,
    candidates: &[SchemaBehaviorCandidate],
) -> BTreeSet<String> {
    let mut bindings = BTreeSet::from(["candidate".to_owned(), "element".to_owned()]);
    bindings.extend(model.attributes.keys().cloned());
    for element in model.elements.values() {
        bindings.extend(element.required_attributes.iter().cloned());
        bindings.extend(element.optional_attributes.iter().cloned());
    }
    for candidate in candidates {
        bindings.extend(candidate.attributes.keys().cloned());
    }
    bindings
}

fn collect_schema_behavior_candidates(document: &CemDocument) -> Vec<SchemaBehaviorCandidate> {
    document
        .iter()
        .filter_map(|node| {
            let CemAstNode::Element {
                node_id,
                expanded_name,
                attributes,
                source,
                ..
            } = node
            else {
                return None;
            };
            if expanded_name.local_name.is_empty()
                || expanded_name.local_name == "$"
                || expanded_name.local_name.starts_with('@')
            {
                return None;
            }
            let mut candidate_attributes = BTreeMap::new();
            for attr_id in attributes {
                let Some(CemAstNode::Attribute {
                    expanded_name,
                    value,
                    ..
                }) = document.get(*attr_id)
                else {
                    continue;
                };
                candidate_attributes.insert(
                    expanded_name.local_name.clone(),
                    value.clone().unwrap_or_default(),
                );
            }
            Some(SchemaBehaviorCandidate {
                node_id: *node_id,
                element: expanded_name.local_name.clone(),
                attributes: candidate_attributes,
                source_map: source.clone(),
            })
        })
        .collect()
}

fn select_bindings(
    candidates: &[SchemaBehaviorCandidate],
    model: &SchemaDocumentModel,
) -> BTreeMap<String, ItemStream> {
    let mut bindings = BTreeMap::new();
    let all_items = candidates
        .iter()
        .map(candidate_record_item)
        .collect::<Vec<_>>();
    bindings.insert("nodes".to_owned(), ItemStream::from_items(all_items));
    for element_name in model.elements.keys() {
        let items = candidates
            .iter()
            .filter(|candidate| candidate.element == *element_name)
            .map(candidate_record_item)
            .collect::<Vec<_>>();
        bindings.insert(element_name.clone(), ItemStream::from_items(items));
    }
    bindings
}

fn candidate_match_bindings(
    candidate: &SchemaBehaviorCandidate,
    binding_names: &BTreeSet<String>,
) -> BTreeMap<String, ItemStream> {
    let mut bindings = BTreeMap::new();
    bindings.insert(
        "candidate".to_owned(),
        ItemStream::once(candidate_record_item(candidate)),
    );
    bindings.insert(
        "element".to_owned(),
        ItemStream::once(Item::Atomic(AtomValue::String(candidate.element.clone()))),
    );
    for name in binding_names {
        if matches!(name.as_str(), "candidate" | "element") {
            continue;
        }
        let item = candidate
            .attributes
            .get(name)
            .map(|value| Item::Atomic(AtomValue::String(value.clone())))
            .unwrap_or(Item::Atomic(AtomValue::Null));
        bindings.insert(name.clone(), ItemStream::once(item));
    }
    bindings
}

fn candidate_record_item(candidate: &SchemaBehaviorCandidate) -> Item {
    let mut attributes = BTreeMap::new();
    for (name, value) in &candidate.attributes {
        attributes.insert(
            name.clone(),
            vec![Item::Atomic(AtomValue::String(value.clone()))],
        );
    }
    let mut record = BTreeMap::new();
    record.insert(
        "nodeId".to_owned(),
        vec![Item::Atomic(AtomValue::Integer(candidate.node_id as i64))],
    );
    record.insert(
        "name".to_owned(),
        vec![Item::Atomic(AtomValue::String(candidate.element.clone()))],
    );
    record.insert(
        "element".to_owned(),
        vec![Item::Atomic(AtomValue::String(candidate.element.clone()))],
    );
    record.insert("attributes".to_owned(), vec![Item::Record(attributes)]);
    Item::Record(record)
}

fn selected_candidate_ids(stream: &ItemStream) -> BTreeSet<AstNodeId> {
    let mut ids = BTreeSet::new();
    for item in &stream.items {
        collect_candidate_ids_from_item(item, &mut ids);
    }
    ids
}

fn collect_candidate_ids_from_item(item: &Item, ids: &mut BTreeSet<AstNodeId>) {
    match item {
        Item::Record(record) => {
            if let Some(id) = record
                .get("nodeId")
                .and_then(|items| items.first())
                .and_then(item_to_ast_node_id)
            {
                ids.insert(id);
            }
        }
        Item::Array(items) => {
            for item in items {
                collect_candidate_ids_from_item(item, ids);
            }
        }
        _ => {
            if let Some(id) = item_to_ast_node_id(item) {
                ids.insert(id);
            }
        }
    }
}

fn item_to_ast_node_id(item: &Item) -> Option<AstNodeId> {
    match item {
        Item::Atomic(AtomValue::Integer(value)) => (*value).try_into().ok(),
        Item::Atomic(AtomValue::String(value)) => value.parse::<AstNodeId>().ok(),
        _ => None,
    }
}

fn stream_truthy(stream: &ItemStream) -> bool {
    let Some(item) = stream.items.first() else {
        return false;
    };
    match item {
        Item::Atomic(AtomValue::Boolean(value)) => *value,
        Item::Atomic(AtomValue::Integer(value)) => *value != 0,
        Item::Atomic(AtomValue::Decimal(value)) => value != "0" && value != "0.0",
        Item::Atomic(AtomValue::Double(value)) => *value != 0.0 && !value.is_nan(),
        Item::Atomic(AtomValue::String(value)) | Item::Atomic(AtomValue::AnyUri(value)) => {
            !value.is_empty()
        }
        Item::Atomic(AtomValue::Null) => false,
        _ => true,
    }
}

fn diagnostic_behavior_function<'a>(
    diagnostic: &'a DiagnosticBehavior,
    definition: &'a BehaviorDefinition,
) -> Option<&'a cem_ml::schema::document_model::BehaviorFunctionDeclaration> {
    diagnostic.function_definition.as_ref().or_else(|| {
        diagnostic
            .function
            .as_deref()
            .and_then(|function_name| definition.inline_functions.get(function_name))
    })
}

fn execute_schema_behavior_function(
    model: &SchemaDocumentModel,
    diagnostic: &DiagnosticBehavior,
    definition: &BehaviorDefinition,
    candidate: &SchemaBehaviorCandidate,
) -> Option<Diagnostic> {
    let function_name = diagnostic.function.as_deref()?;
    let Some(function) = diagnostic_behavior_function(diagnostic, definition) else {
        return Some(schema_behavior_function_failed_diagnostic(
            model,
            diagnostic,
            function_name,
            candidate,
            format!("CEM-ML behavior function `{function_name}` is not declared"),
        ));
    };
    let Some(body_expression) = function.body_expression.as_deref() else {
        return Some(schema_behavior_function_failed_diagnostic(
            model,
            diagnostic,
            function_name,
            candidate,
            format!("CEM-ML behavior function `{function_name}` has no body expression"),
        ));
    };
    let mut arguments = BTreeMap::new();
    for param in &function.params {
        match schema_behavior_function_argument_value(param, diagnostic, definition, candidate) {
            Ok(Some(value)) => {
                arguments.insert(param.name.clone(), value);
            }
            Ok(None) if param.required => {
                return Some(schema_behavior_function_failed_diagnostic(
                    model,
                    diagnostic,
                    function_name,
                    candidate,
                    format!(
                        "required CEM-ML behavior function parameter `{}` was not bound",
                        param.name
                    ),
                ));
            }
            Ok(None) => {}
            Err(message) => {
                return Some(schema_behavior_function_failed_diagnostic(
                    model,
                    diagnostic,
                    function_name,
                    candidate,
                    message,
                ));
            }
        }
    }
    let expression = match parse_schema_behavior_expression(body_expression) {
        Ok(expression) => expression,
        Err(message) => {
            return Some(schema_behavior_function_failed_diagnostic(
                model,
                diagnostic,
                function_name,
                candidate,
                format!("invalid CEM-ML behavior body: {message}"),
            ))
        }
    };
    let result = match evaluate_schema_behavior_expression(&expression, &arguments) {
        Ok(value) => value,
        Err(message) => {
            return Some(schema_behavior_function_failed_diagnostic(
                model,
                diagnostic,
                function_name,
                candidate,
                message,
            ))
        }
    };
    let return_type = schema_behavior_value_type(&function.returns);
    if !return_type.accepts(&result) {
        return Some(schema_behavior_function_failed_diagnostic(
            model,
            diagnostic,
            function_name,
            candidate,
            format!(
                "CEM-ML behavior function `{function_name}` returned {}, expected {}",
                json_value_kind(&result),
                return_type.as_contract_name()
            ),
        ));
    }
    let Some(result) = result.as_object() else {
        return Some(schema_behavior_diagnostic(
            SCHEMA_BEHAVIOR_RESULT_INVALID_CODE,
            Severity::Error,
            format!(
                "diagnostic `{}` behavior `{}` function `{function_name}` returned a non-object result",
                diagnostic.code, diagnostic.behavior
            ),
            &candidate.source_map,
            json!({
                "schemaUri": model.schema_uri,
                "diagnostic": diagnostic.code,
                "behavior": diagnostic.behavior,
                "function": function_name,
                "candidate": candidate_json(candidate),
                "result": result,
            }),
        ));
    };
    let message = result
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| diagnostic.message.clone())
        .unwrap_or_else(|| {
            format!(
                "schema behavior `{}` matched `{}`",
                diagnostic.behavior, candidate.element
            )
        });
    let mut details = Map::new();
    details.insert(
        "schemaUri".to_owned(),
        Value::String(model.schema_uri.clone()),
    );
    details.insert(
        "diagnostic".to_owned(),
        Value::String(diagnostic.code.clone()),
    );
    details.insert(
        "behavior".to_owned(),
        Value::String(diagnostic.behavior.clone()),
    );
    details.insert(
        "function".to_owned(),
        Value::String(function_name.to_owned()),
    );
    details.insert(
        "element".to_owned(),
        Value::String(candidate.element.clone()),
    );
    details.insert("candidate".to_owned(), candidate_json(candidate));
    if let Some(source_range) = source_map_range_details(&candidate.source_map) {
        details.insert("sourceRange".to_owned(), source_range);
    }
    if let Some(result_details) = result.get("details").and_then(Value::as_object) {
        for (name, value) in result_details {
            details.insert(name.clone(), value.clone());
        }
    }
    Some(schema_behavior_diagnostic(
        &diagnostic.code,
        diagnostic.severity,
        message,
        &candidate.source_map,
        Value::Object(details),
    ))
}

fn schema_behavior_function_argument_value(
    param: &BehaviorFunctionParam,
    diagnostic: &DiagnosticBehavior,
    definition: &BehaviorDefinition,
    candidate: &SchemaBehaviorCandidate,
) -> Result<Option<Value>, String> {
    let value = match param.name.as_str() {
        "candidate" => Some(candidate_json(candidate)),
        "diagnostic" => Some(json!({
            "code": diagnostic.code,
            "severity": severity_name(diagnostic.severity),
            "behavior": diagnostic.behavior,
            "message": diagnostic.message,
        })),
        _ => definition
            .parameters
            .iter()
            .find(|parameter| parameter.name == param.name)
            .map(|parameter| behavior_parameter_bound_value(parameter, diagnostic))
            .transpose()?
            .flatten(),
    };
    if let Some(value) = value.as_ref() {
        let value_type = schema_behavior_value_type(&param.value_type);
        if !value_type.accepts(value) {
            return Err(format!(
                "CEM-ML behavior function parameter `{}` expected {}, got {}",
                param.name,
                value_type.as_contract_name(),
                json_value_kind(value)
            ));
        }
    }
    Ok(value)
}

fn behavior_parameter_bound_value(
    parameter: &BehaviorParameter,
    diagnostic: &DiagnosticBehavior,
) -> Result<Option<Value>, String> {
    if let Some(argument) = diagnostic
        .arguments
        .iter()
        .find(|argument| argument.name == parameter.name)
    {
        return behavior_argument_value(argument, parameter).map(Some);
    }
    behavior_parameter_default_value(parameter)
}

fn schema_behavior_function_failed_diagnostic(
    model: &SchemaDocumentModel,
    diagnostic: &DiagnosticBehavior,
    function_name: &str,
    candidate: &SchemaBehaviorCandidate,
    message: impl Into<String>,
) -> Diagnostic {
    let message = message.into();
    schema_behavior_diagnostic(
        SCHEMA_BEHAVIOR_FUNCTION_FAILED_CODE,
        Severity::Error,
        format!(
            "diagnostic `{}` behavior `{}` function `{function_name}` failed: {message}",
            diagnostic.code, diagnostic.behavior
        ),
        &candidate.source_map,
        json!({
            "schemaUri": model.schema_uri,
            "diagnostic": diagnostic.code,
            "behavior": diagnostic.behavior,
            "function": function_name,
            "candidate": candidate_json(candidate),
        }),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaBehaviorValueType {
    Any,
    String,
    Boolean,
    Number,
    Integer,
    Array,
    Object,
    Json,
}

impl SchemaBehaviorValueType {
    fn accepts(self, value: &Value) -> bool {
        match self {
            Self::Any | Self::Json => true,
            Self::String => value.is_string(),
            Self::Boolean => value.is_boolean(),
            Self::Number => value.is_number(),
            Self::Integer => value.as_i64().is_some() || value.as_u64().is_some(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }

    fn as_contract_name(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::String => "string",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Array => "array",
            Self::Object => "object",
            Self::Json => "json",
        }
    }
}

fn schema_behavior_value_type(value: &str) -> SchemaBehaviorValueType {
    match value
        .trim()
        .rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or_else(|| value.trim())
    {
        "string" | "identifier" | "diagnostic-code" | "behavior-reference" => {
            SchemaBehaviorValueType::String
        }
        "boolean" => SchemaBehaviorValueType::Boolean,
        "number" => SchemaBehaviorValueType::Number,
        "integer" => SchemaBehaviorValueType::Integer,
        "array" => SchemaBehaviorValueType::Array,
        "object" | "node" | "diagnostic" | "diagnostic-result" => SchemaBehaviorValueType::Object,
        "json" => SchemaBehaviorValueType::Json,
        _ => SchemaBehaviorValueType::Any,
    }
}

fn behavior_parameter_default_value(
    parameter: &BehaviorParameter,
) -> Result<Option<Value>, String> {
    let Some(default) = parameter.default.as_deref() else {
        return Ok(None);
    };
    behavior_parameter_raw_value(parameter, default).map(Some)
}

fn behavior_argument_value(
    argument: &BehaviorArgument,
    parameter: &BehaviorParameter,
) -> Result<Value, String> {
    behavior_parameter_raw_value(parameter, &argument.value)
}

fn behavior_parameter_raw_value(
    parameter: &BehaviorParameter,
    raw_value: &str,
) -> Result<Value, String> {
    if !parameter.values.is_empty() && !parameter.values.contains(raw_value) {
        return Err(format!(
            "value `{raw_value}` is outside declared values `{}`",
            parameter
                .values
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    let value_type = schema_behavior_value_type(&parameter.value_type);
    let value = match value_type {
        SchemaBehaviorValueType::Boolean => match raw_value.trim() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            other => return Err(format!("expected boolean value, got `{other}`")),
        },
        SchemaBehaviorValueType::Integer => {
            let value = raw_value
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("expected integer value, got `{raw_value}`"))?;
            Value::Number(Number::from(value))
        }
        SchemaBehaviorValueType::Number => {
            let value = raw_value
                .trim()
                .parse::<f64>()
                .map_err(|_| format!("expected number value, got `{raw_value}`"))?;
            Value::Number(
                Number::from_f64(value)
                    .ok_or_else(|| format!("expected finite number value, got `{raw_value}`"))?,
            )
        }
        SchemaBehaviorValueType::Array
        | SchemaBehaviorValueType::Object
        | SchemaBehaviorValueType::Json => {
            let value = serde_json::from_str::<Value>(raw_value)
                .map_err(|err| format!("value is not valid JSON: {err}"))?;
            if !value_type.accepts(&value) {
                return Err(format!(
                    "expected {} value, got {}",
                    value_type.as_contract_name(),
                    json_value_kind(&value)
                ));
            }
            value
        }
        SchemaBehaviorValueType::Any | SchemaBehaviorValueType::String => {
            Value::String(raw_value.to_owned())
        }
    };
    Ok(value)
}

#[derive(Debug, Clone, PartialEq)]
enum SchemaBehaviorExpression {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Path(Vec<String>),
    Array(Vec<SchemaBehaviorExpression>),
    Object(Vec<(String, SchemaBehaviorExpression)>),
}

fn parse_schema_behavior_expression(raw: &str) -> Result<SchemaBehaviorExpression, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("empty expression".to_owned());
    }
    if raw.starts_with('{') {
        return parse_schema_behavior_object(raw);
    }
    if raw.starts_with('[') {
        return parse_schema_behavior_array(raw);
    }
    if raw.starts_with('"') || raw.starts_with('\'') {
        return parse_schema_behavior_string_literal(raw).map(SchemaBehaviorExpression::String);
    }
    if raw == "null" {
        return Ok(SchemaBehaviorExpression::Null);
    }
    if raw == "true" {
        return Ok(SchemaBehaviorExpression::Bool(true));
    }
    if raw == "false" {
        return Ok(SchemaBehaviorExpression::Bool(false));
    }
    if raw.starts_with('$') {
        return parse_schema_behavior_path(raw);
    }
    if let Ok(Value::Number(number)) = serde_json::from_str::<Value>(raw) {
        return Ok(SchemaBehaviorExpression::Number(number));
    }
    Err(format!("unsupported expression `{raw}`"))
}

fn parse_schema_behavior_object(raw: &str) -> Result<SchemaBehaviorExpression, String> {
    let end = matching_schema_behavior_delimiter(raw, '{', '}')?;
    if end != raw.len() - 1 {
        return Err("unexpected content after object expression".to_owned());
    }
    let inner = raw[1..end].trim();
    if inner.is_empty() {
        return Ok(SchemaBehaviorExpression::Object(Vec::new()));
    }
    let mut fields = Vec::new();
    for field in split_schema_behavior_top_level(inner, ',')? {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let colon = find_schema_behavior_top_level(field, ':')?
            .ok_or_else(|| format!("object field `{field}` is missing `:`"))?;
        let key = parse_schema_behavior_object_key(&field[..colon])?;
        let value = parse_schema_behavior_expression(&field[colon + 1..])?;
        fields.push((key, value));
    }
    Ok(SchemaBehaviorExpression::Object(fields))
}

fn parse_schema_behavior_array(raw: &str) -> Result<SchemaBehaviorExpression, String> {
    let end = matching_schema_behavior_delimiter(raw, '[', ']')?;
    if end != raw.len() - 1 {
        return Err("unexpected content after array expression".to_owned());
    }
    let inner = raw[1..end].trim();
    if inner.is_empty() {
        return Ok(SchemaBehaviorExpression::Array(Vec::new()));
    }
    let mut items = Vec::new();
    for item in split_schema_behavior_top_level(inner, ',')? {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        items.push(parse_schema_behavior_expression(item)?);
    }
    Ok(SchemaBehaviorExpression::Array(items))
}

fn parse_schema_behavior_object_key(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.starts_with('"') || raw.starts_with('\'') {
        return parse_schema_behavior_string_literal(raw);
    }
    if raw.is_empty() {
        return Err("object field key is empty".to_owned());
    }
    if raw
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.'))
    {
        return Ok(raw.to_owned());
    }
    Err(format!("invalid object field key `{raw}`"))
}

fn parse_schema_behavior_string_literal(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    let quote = raw
        .chars()
        .next()
        .ok_or_else(|| "empty string literal".to_owned())?;
    if quote != '"' && quote != '\'' {
        return Err(format!("expected string literal, got `{raw}`"));
    }
    let end = matching_schema_behavior_quote(raw, quote)?;
    if end != raw.len() - 1 {
        return Err("unexpected content after string literal".to_owned());
    }
    if quote == '"' {
        return serde_json::from_str::<String>(raw)
            .map_err(|err| format!("invalid string literal: {err}"));
    }
    parse_schema_behavior_single_quoted_string(&raw[1..end])
}

fn parse_schema_behavior_single_quoted_string(inner: &str) -> Result<String, String> {
    let mut value = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            value.push(ch);
            continue;
        }
        let escaped = chars
            .next()
            .ok_or_else(|| "unterminated string escape".to_owned())?;
        match escaped {
            '\\' => value.push('\\'),
            '\'' => value.push('\''),
            '"' => value.push('"'),
            'n' => value.push('\n'),
            'r' => value.push('\r'),
            't' => value.push('\t'),
            other => return Err(format!("unsupported string escape `\\{other}`")),
        }
    }
    Ok(value)
}

fn parse_schema_behavior_path(raw: &str) -> Result<SchemaBehaviorExpression, String> {
    let path = raw
        .trim()
        .strip_prefix('$')
        .expect("path parser is only called for `$` expressions");
    if path.is_empty() {
        return Err("path expression is missing a binding name".to_owned());
    }
    let mut segments = Vec::new();
    for segment in path.split('.') {
        if segment.is_empty() {
            return Err(format!("path expression `{raw}` contains an empty segment"));
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        {
            return Err(format!(
                "path expression `{raw}` contains invalid segment `{segment}`"
            ));
        }
        segments.push(segment.to_owned());
    }
    Ok(SchemaBehaviorExpression::Path(segments))
}

fn matching_schema_behavior_delimiter(raw: &str, open: char, close: char) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in raw.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        if ch == open {
            depth += 1;
            continue;
        }
        if ch == close {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| format!("unexpected `{close}`"))?;
            if depth == 0 {
                return Ok(index);
            }
        }
    }
    Err(format!("unterminated `{open}` expression"))
}

fn matching_schema_behavior_quote(raw: &str, quote: char) -> Result<usize, String> {
    let mut escaped = false;
    for (index, ch) in raw.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Ok(index);
        }
    }
    Err("unterminated string literal".to_owned())
}

fn split_schema_behavior_top_level(raw: &str, delimiter: char) -> Result<Vec<&str>, String> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in raw.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        match ch {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| format!("unexpected `{ch}`"))?;
            }
            _ if ch == delimiter && depth == 0 => {
                parts.push(&raw[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    if quote.is_some() {
        return Err("unterminated string literal".to_owned());
    }
    if depth != 0 {
        return Err("unterminated nested expression".to_owned());
    }
    parts.push(&raw[start..]);
    Ok(parts)
}

fn find_schema_behavior_top_level(raw: &str, needle: char) -> Result<Option<usize>, String> {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in raw.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == active_quote {
                quote = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            continue;
        }
        match ch {
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| format!("unexpected `{ch}`"))?;
            }
            _ if ch == needle && depth == 0 => return Ok(Some(index)),
            _ => {}
        }
    }
    if quote.is_some() {
        return Err("unterminated string literal".to_owned());
    }
    if depth != 0 {
        return Err("unterminated nested expression".to_owned());
    }
    Ok(None)
}

fn evaluate_schema_behavior_expression(
    expression: &SchemaBehaviorExpression,
    bindings: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    match expression {
        SchemaBehaviorExpression::Null => Ok(Value::Null),
        SchemaBehaviorExpression::Bool(value) => Ok(Value::Bool(*value)),
        SchemaBehaviorExpression::Number(value) => Ok(Value::Number(value.clone())),
        SchemaBehaviorExpression::String(value) => Ok(Value::String(value.clone())),
        SchemaBehaviorExpression::Path(path) => resolve_schema_behavior_path(path, bindings),
        SchemaBehaviorExpression::Array(items) => items
            .iter()
            .map(|item| evaluate_schema_behavior_expression(item, bindings))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        SchemaBehaviorExpression::Object(fields) => {
            let mut object = Map::new();
            for (name, value) in fields {
                object.insert(
                    name.clone(),
                    evaluate_schema_behavior_expression(value, bindings)?,
                );
            }
            Ok(Value::Object(object))
        }
    }
}

fn resolve_schema_behavior_path(
    path: &[String],
    bindings: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    let Some((binding, segments)) = path.split_first() else {
        return Err("path expression is empty".to_owned());
    };
    let mut value = bindings
        .get(binding)
        .ok_or_else(|| format!("unknown CEM-ML behavior binding `${binding}`"))?;
    for segment in segments {
        match value {
            Value::Object(object) => {
                value = object.get(segment).ok_or_else(|| {
                    format!("CEM-ML behavior path `${}` is unresolved", path.join("."))
                })?;
            }
            Value::Array(items) => {
                let index = segment.parse::<usize>().map_err(|_| {
                    format!(
                        "CEM-ML behavior path `${}` expected array index, got `{segment}`",
                        path.join(".")
                    )
                })?;
                value = items.get(index).ok_or_else(|| {
                    format!(
                        "CEM-ML behavior path `${}` index `{index}` is out of bounds",
                        path.join(".")
                    )
                })?;
            }
            _ => {
                return Err(format!(
                    "CEM-ML behavior path `${}` crosses non-container field `{segment}`",
                    path.join(".")
                ));
            }
        }
    }
    Ok(value.clone())
}

fn json_value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Info => "info",
        Severity::Warning => "warning",
        Severity::Error => "error",
        Severity::Fatal => "fatal",
    }
}

fn candidate_json(candidate: &SchemaBehaviorCandidate) -> Value {
    json!({
        "nodeId": candidate.node_id,
        "name": candidate.element,
        "element": candidate.element,
        "attributes": candidate.attributes,
        "sourceRange": source_map_range_details(&candidate.source_map),
    })
}

fn source_map_range_details(source_map: &SourceMapStack) -> Option<Value> {
    source_map.current().map(|frame| {
        json!({
            "sourceId": frame.source_id.0,
            "span": frame_span_details(&frame.span),
        })
    })
}

fn frame_span_details(span: &FrameSpan) -> Value {
    match span {
        FrameSpan::Single(range) => json!({
            "kind": "single",
            "start": range.start,
            "len": range.len,
            "end": range.end(),
        }),
        FrameSpan::Multi(ranges) => json!({
            "kind": "multi",
            "ranges": ranges.iter().map(|range| {
                json!({
                    "start": range.start,
                    "len": range.len,
                    "end": range.end(),
                })
            }).collect::<Vec<_>>(),
        }),
    }
}

fn schema_behavior_diagnostic(
    code: &str,
    severity: Severity,
    message: String,
    source_map: &SourceMapStack,
    details: Value,
) -> Diagnostic {
    let byte_offset = source_map
        .frames
        .first()
        .and_then(|frame| match &frame.span {
            FrameSpan::Single(range) => Some(range.start),
            FrameSpan::Multi(ranges) => ranges.first().map(|range| range.start),
        });
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset,
        code: code.to_owned(),
        severity,
        message,
        node: None,
        details: Some(details),
        source_map: Some(source_map.clone()),
    }
}

impl TransformTemplateAdapter for CemQlTransformTemplateAdapter {
    fn id(&self) -> &'static str {
        CEM_QL_TEMPLATE_ADAPTER_ID
    }

    fn kind(&self) -> TransformTemplateKind {
        TransformTemplateKind::CemNative
    }

    fn capability(&self) -> TransformTemplateAdapterCapability {
        TransformTemplateAdapterCapability::Executable
    }

    fn matches_template(&self, identity: &FormatIdentity) -> bool {
        matches_cem_native_identity(identity)
    }

    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        let source = std::str::from_utf8(&request.template.bytes).map_err(|err| {
            TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                format!(
                    "template `{}` is not valid UTF-8: {err}",
                    request.template.uri
                ),
            )
        })?;
        let host_bindings = host_binding_names(
            request.params,
            request.data_bindings,
            &request.module_options,
        );
        let artifact = compile_template(
            source,
            &CompileTemplateOptions {
                host_bindings: host_bindings.clone(),
                ..CompileTemplateOptions::default()
            },
        );
        let mut diagnostics =
            diagnostics_with_uri(&artifact.diagnostics, request.template.uri.as_str());
        let entrypoints = extract_template_entrypoints(&artifact);
        let render_artifact =
            select_entrypoint_artifact(&artifact, &entrypoints, request.entrypoint.name.as_deref());
        let mut modules = compile_preflighted_modules(self.id(), &request, &host_bindings)?;
        for module in &modules {
            diagnostics.extend(diagnostics_with_uri(
                &module.artifact.diagnostics,
                module.uri.as_str(),
            ));
        }
        diagnostics.retain(|diagnostic| !is_cemt_encode_compile_diagnostic(diagnostic));
        let module_diagnostics = modules
            .iter()
            .map(|module| module.artifact.diagnostics.len())
            .sum::<usize>();
        let module_metadata = modules
            .iter()
            .map(|module| {
                json!({
                    "alias": module.alias,
                    "uri": module.uri,
                    "contentHash": module.content_hash,
                    "diagnostics": module.artifact.diagnostics.len(),
                })
            })
            .collect::<Vec<_>>();
        let mut render_artifact = protect_transform_call_artifact(render_artifact);
        let mut entrypoints = entrypoints;
        protect_transform_call_entrypoints(&mut entrypoints);
        clear_template_artifact_diagnostics(&mut render_artifact);
        clear_template_entrypoint_diagnostics(&mut entrypoints);
        for module in &mut modules {
            protect_transform_call_nodes(&mut module.artifact.nodes);
            protect_transform_call_entrypoints(&mut module.entrypoints);
            clear_template_artifact_diagnostics(&mut module.artifact);
            clear_template_entrypoint_diagnostics(&mut module.entrypoints);
        }
        let opaque = json!({
            "engine": "cem-ql",
            "templateBytes": request.template.bytes.len(),
            "diagnostics": artifact.diagnostics.len(),
            "moduleImports": modules.len(),
            "moduleDiagnostics": module_diagnostics,
            "modules": module_metadata,
            "moduleCacheKey": request.module_preflight.cache_key.clone(),
        });

        Ok(TransformTemplateCompileResponse {
            artifact: TransformTemplateCompiledArtifact::new(
                self.id(),
                self.kind(),
                request.template.uri.clone(),
                request.template.identity.clone(),
                request.entrypoint.clone(),
                opaque,
            )
            .with_native_payload(CemQlCompiledTemplatePayload {
                template_uri: request.template.uri.clone(),
                artifact: render_artifact,
                selected_entrypoint: request.entrypoint.name.clone(),
                params: request.params.clone(),
                param_declarations: request.module_options.params.clone(),
                entrypoints,
                modules,
                max_recursion_depth: request.module_options.limits.max_recursion_depth,
            }),
            diagnostics,
        })
    }

    fn render(
        &self,
        request: TransformTemplateRenderRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        render_cem_ql_payload(self.id(), request)
    }
}

impl TransformTemplateAdapter for CemQlExpressionTransformTemplateAdapter {
    fn id(&self) -> &'static str {
        CEM_QL_EXPRESSION_TEMPLATE_ADAPTER_ID
    }

    fn kind(&self) -> TransformTemplateKind {
        TransformTemplateKind::CemQlExpression
    }

    fn capability(&self) -> TransformTemplateAdapterCapability {
        TransformTemplateAdapterCapability::Executable
    }

    fn matches_template(&self, identity: &FormatIdentity) -> bool {
        matches_cem_ql_expression_identity(identity)
    }

    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        let source = std::str::from_utf8(&request.template.bytes).map_err(|err| {
            TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                format!(
                    "expression template `{}` is not valid UTF-8: {err}",
                    request.template.uri
                ),
            )
        })?;
        let context = expression_compile_context(
            &request.template.uri,
            request.params,
            request.data_bindings,
            request
                .module_preflight
                .cache_key
                .as_ref()
                .and_then(|cache_key| serde_json::to_string(cache_key).ok()),
        );
        let compiled = match compile_expression(source, &context) {
            Ok(compiled) => compiled,
            Err(error) => {
                let diagnostics = diagnostics_with_uri(&error.diagnostics, &request.template.uri);
                return Ok(TransformTemplateCompileResponse {
                    artifact: TransformTemplateCompiledArtifact::new(
                        self.id(),
                        self.kind(),
                        request.template.uri.clone(),
                        request.template.identity.clone(),
                        request.entrypoint.clone(),
                        json!({
                            "engine": "cem-ql",
                            "templateKind": "expression",
                            "diagnostics": diagnostics.len(),
                        }),
                    ),
                    diagnostics,
                });
            }
        };
        let diagnostics = diagnostics_with_uri(&compiled.diagnostics, &request.template.uri);

        Ok(TransformTemplateCompileResponse {
            artifact: TransformTemplateCompiledArtifact::new(
                self.id(),
                self.kind(),
                request.template.uri.clone(),
                request.template.identity.clone(),
                request.entrypoint.clone(),
                json!({
                    "engine": "cem-ql",
                    "templateKind": "expression",
                    "templateBytes": request.template.bytes.len(),
                    "diagnostics": diagnostics.len(),
                }),
            )
            .with_native_payload(CemQlCompiledExpressionPayload {
                template_uri: request.template.uri.clone(),
                compiled,
                params: request.params.clone(),
            }),
            diagnostics,
        })
    }

    fn render(
        &self,
        request: TransformTemplateRenderRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        render_cem_ql_expression_payload(self.id(), request)
    }
}

impl TransformTemplateAdapter for XsltParityTransformTemplateAdapter {
    fn id(&self) -> &'static str {
        XSLT_PARITY_TEMPLATE_ADAPTER_ID
    }

    fn kind(&self) -> TransformTemplateKind {
        TransformTemplateKind::Xslt
    }

    fn capability(&self) -> TransformTemplateAdapterCapability {
        TransformTemplateAdapterCapability::Executable
    }

    fn matches_template(&self, identity: &FormatIdentity) -> bool {
        matches_xslt_identity(identity)
    }

    fn compile(
        &self,
        request: TransformTemplateCompileRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateCompileResponse> {
        let source = std::str::from_utf8(&request.template.bytes).map_err(|err| {
            TransformTemplateAdapterError::failed(
                self.id(),
                TransformTemplateAdapterExecutionPhase::Compile,
                format!(
                    "template `{}` is not valid UTF-8: {err}",
                    request.template.uri
                ),
            )
        })?;
        let source = xslt_source_for_entrypoint(source, request.entrypoint, request.params);
        let lowered = convert_template_source(&source);
        let mut diagnostics = lowered
            .diagnostics
            .iter()
            .map(|diagnostic| {
                xslt_lowering_diagnostic_to_engine(
                    diagnostic,
                    request.template.uri.as_str(),
                    request.entrypoint.name.as_deref(),
                )
            })
            .collect::<Vec<_>>();
        let host_bindings = host_binding_names(
            request.params,
            request.data_bindings,
            &request.module_options,
        );
        let artifact = compile_template(
            &lowered.source,
            &CompileTemplateOptions {
                host_bindings: host_bindings.clone(),
                ..CompileTemplateOptions::default()
            },
        );
        diagnostics.extend(diagnostics_with_uri(
            &artifact.diagnostics,
            request.template.uri.as_str(),
        ));
        let mut render_artifact = artifact;
        clear_template_artifact_diagnostics(&mut render_artifact);
        let opaque = json!({
            "engine": "cem-ql",
            "source": "xslt-parity",
            "templateBytes": request.template.bytes.len(),
            "entrypoint": request.entrypoint.name.clone(),
            "loweredBytes": lowered.source.len(),
            "loweringDiagnostics": diagnostics.len(),
        });

        Ok(TransformTemplateCompileResponse {
            artifact: TransformTemplateCompiledArtifact::new(
                self.id(),
                self.kind(),
                request.template.uri.clone(),
                request.template.identity.clone(),
                request.entrypoint.clone(),
                opaque,
            )
            .with_native_payload(CemQlCompiledTemplatePayload {
                template_uri: request.template.uri.clone(),
                artifact: render_artifact,
                selected_entrypoint: request.entrypoint.name.clone(),
                params: request.params.clone(),
                param_declarations: Vec::new(),
                entrypoints: CemQlTemplateEntrypoints::default(),
                modules: Vec::new(),
                max_recursion_depth: request.module_options.limits.max_recursion_depth,
            }),
            diagnostics,
        })
    }

    fn render(
        &self,
        request: TransformTemplateRenderRequest<'_>,
    ) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
        render_cem_ql_payload(self.id(), request)
    }
}

fn xslt_source_for_entrypoint(
    source: &str,
    entrypoint: &TransformTemplateEntrypoint,
    params: &BTreeMap<String, Value>,
) -> String {
    let Some(name) = entrypoint.name.as_deref() else {
        return source.to_owned();
    };

    let wrapper = xslt_entrypoint_wrapper(name, params);
    for closing in ["</xsl:stylesheet>", "</stylesheet>"] {
        if let Some(index) = source.rfind(closing) {
            let mut out = String::with_capacity(source.len() + wrapper.len());
            out.push_str(&source[..index]);
            out.push_str(&wrapper);
            out.push_str(&source[index..]);
            return out;
        }
    }

    format!(r#"<xsl:stylesheet version="1.0">{source}{wrapper}</xsl:stylesheet>"#)
}

fn xslt_entrypoint_wrapper(name: &str, params: &BTreeMap<String, Value>) -> String {
    let mut out = format!(
        r#"<xsl:template match="/"><xsl:call-template name="{}">"#,
        xml_attr_escape(name)
    );
    for (name, value) in params {
        out.push_str(&format!(
            r#"<xsl:with-param name="{}">{}"#,
            xml_attr_escape(name),
            xml_text_escape(&xslt_param_text(value))
        ));
        out.push_str("</xsl:with-param>");
    }
    out.push_str("</xsl:call-template></xsl:template>");
    out
}

fn xslt_param_text(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn xslt_lowering_diagnostic_to_engine(
    diagnostic: &LegacyConversionDiagnostic,
    uri: &str,
    selected_entrypoint: Option<&str>,
) -> Diagnostic {
    if diagnostic.code == "legacy_xslt.call_template_missing_target" {
        let mut message = diagnostic.message.clone();
        if let Some(entrypoint) = selected_entrypoint {
            message = format!("XSLT template entrypoint `{entrypoint}` was not found");
        }
        return Diagnostic {
            uri: Some(uri.to_owned()),
            code: TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE.to_owned(),
            severity: Severity::Fatal,
            message,
            ..Diagnostic::default()
        };
    }
    if matches!(
        diagnostic.code.as_str(),
        UNSUPPORTED_CONSTRUCT_CODE | UNSUPPORTED_FUNCTION_CODE
    ) {
        return Diagnostic {
            uri: Some(uri.to_owned()),
            code: diagnostic.code.clone(),
            severity: Severity::Fatal,
            message: diagnostic.message.clone(),
            ..Diagnostic::default()
        };
    }

    let mut diagnostic = diagnostic.to_engine_diagnostic(Some(uri.to_owned()));
    diagnostic.severity = Severity::Warning;
    diagnostic
}

fn xml_attr_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_text_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub fn register_cem_ql_template_adapter(registry: &mut TransformTemplateAdapterRegistry) {
    registry.register(CemQlTransformTemplateAdapter);
    registry.register(CemQlExpressionTransformTemplateAdapter);
    registry.register(XsltParityTransformTemplateAdapter);
}

pub fn register_cem_ql_schema_behavior_evaluator(context: &mut EngineContext) {
    context.schema_behavior_evaluator = Some(Arc::new(CemQlSchemaBehaviorEvaluator));
}

pub fn register_cem_ql_source_output_converter(context: &mut EngineContext) {
    context
        .convert_request_handlers
        .push(Arc::new(CemQlSourceOutputConvertHandler));
}

pub fn register_cem_ql_runtime_adapters(context: &mut EngineContext) {
    register_cem_ql_template_adapter(&mut context.template_adapter_registry);
    register_cem_ql_schema_behavior_evaluator(context);
    register_cem_ql_source_output_converter(context);
}

pub fn engine_context_with_cem_ql_template_adapter() -> EngineContext {
    let mut context = EngineContext::default();
    context.template_adapter_registry = TransformTemplateAdapterRegistry::with_builtin_adapters();
    register_cem_ql_runtime_adapters(&mut context);
    context
}

const CEM_QL_DIRECT_OUTPUT_CONVERTER_ID: &str = "cem-ql-direct-output";
const CEM_QL_DIRECT_FORMATTER: &str = "cem-ql.format-tree";
const CEM_QL_DIRECT_COLORIZER: &str = "cem-ql.color-tree";
const CEM_QL_HTML_PREVIEW_PREFIX: &str =
    r#"<pre class="cem-output cem-output-cem-ql" style="white-space: pre; tab-size: 8">"#;
const CEM_QL_HTML_PREVIEW_SUFFIX: &str = "</pre>";

#[derive(Debug)]
struct CemQlSourceOutputConvertHandler;

impl ConvertRequestHandler for CemQlSourceOutputConvertHandler {
    fn maybe_convert(&self, request: &ConvertRequest) -> Option<ConvertResponse> {
        maybe_convert_cem_ql_source_output(request)
    }
}

/// Converts CEM-QL source through the schema-package formatter/colorizer stack.
///
/// This lives in the bridge crate because `cem_ml` owns the generic output
/// writer and `cem_ql` owns the parser/lexer. Keeping the direct source bridge
/// here avoids a dependency cycle from `cem_ml` back into `cem_ql`.
pub fn maybe_convert_cem_ql_source_output(request: &ConvertRequest) -> Option<ConvertResponse> {
    let source = request
        .input
        .identity
        .clone()
        .unwrap_or_else(|| request.input.root_scope.format_identity());
    if !format_identity_matches_cem_ql(&request.context, &source) {
        return None;
    }

    let target = request
        .target
        .clone()
        .or_else(|| request.target_scope.format_identity_option())
        .or_else(|| cem_ql_direct_target_for_layer_format(request.to_format));
    if !cem_ql_direct_target_supported(
        &request.context,
        target.as_ref(),
        &request.target_scope,
        request.to_format,
    ) {
        return None;
    }

    Some(convert_cem_ql_source_output(request, target.as_ref()))
}

fn convert_cem_ql_source_output(
    request: &ConvertRequest,
    target: Option<&FormatIdentity>,
) -> ConvertResponse {
    let mut diagnostics = Vec::new();
    let source = match std::str::from_utf8(&request.input.bytes) {
        Ok(source) => source,
        Err(_) => {
            diagnostics.push(cem_ql_invalid_utf8_diagnostic(&request.input.uri));
            return cem_ql_failed_convert_response(&request.target_scope, &diagnostics);
        }
    };

    let parsed = cem_ql::api::parse(source);
    diagnostics.extend(cem_ql_parse_diagnostics(&request.input.uri, &parsed));
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return cem_ql_failed_convert_response(&request.target_scope, &diagnostics);
    }

    let token_tree = cem_ql_source_token_tree(&request.input.uri, source);
    let output_color_selection = match cem_ql_output_color_selection(
        &request.context,
        &request.target_scope,
        target,
        request.to_format,
    ) {
        Ok(selection) => selection,
        Err(message) => {
            diagnostics.push(cem_ql_output_pipeline_diagnostic(
                Some(&request.input.uri),
                message,
            ));
            return cem_ql_failed_convert_response(&request.target_scope, &diagnostics);
        }
    };
    let color_profile = match cem_ql_cemt_color_profile_for_output(
        &request.target_scope,
        output_color_selection.as_ref(),
    ) {
        Ok(profile) => profile,
        Err(message) => {
            diagnostics.push(cem_ql_output_pipeline_diagnostic(
                Some(&request.input.uri),
                message,
            ));
            return cem_ql_failed_convert_response(&request.target_scope, &diagnostics);
        }
    };
    let formatter_profile = cem_ql_formatter_profile(&request.target_scope);
    let pipeline = cem_ql_output_pipeline(
        &request.target_scope,
        &formatter_profile,
        &color_profile,
        output_color_selection.as_ref(),
    );
    let environment = ConversionOutputPipelineEnvironment {
        schema_registry: &request.context.schema_registry,
        conversion_registry: &request.context.converter_registry,
        package_artifact_reader: None,
        artifact_cache: None,
    };
    let execution = execute_conversion_output_pipeline_with_environment(
        &environment,
        &pipeline,
        token_tree,
        Some(cem_ql_source_map(ByteRange::new(
            0,
            checked_byte_len(request.input.bytes.len()),
        ))),
        Vec::new(),
        CEM_QL_DIRECT_OUTPUT_CONVERTER_ID,
        Some("cem-ql"),
        Some(&request.input.uri),
    );
    diagnostics.extend(execution.diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation())
    {
        return cem_ql_failed_convert_response(&request.target_scope, &diagnostics);
    }

    let mut content = execution
        .output
        .as_ref()
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let html_output = cem_ql_direct_output_is_html(
        &request.context,
        target,
        &request.target_scope,
        request.to_format,
        output_color_selection.as_ref(),
    );
    if html_output {
        content = format!("{CEM_QL_HTML_PREVIEW_PREFIX}{content}{CEM_QL_HTML_PREVIEW_SUFFIX}");
    }

    let (content_type, schema, format_version) =
        cem_ql_direct_output_primary_identity(target, html_output);
    let bytes = content.into_bytes();
    let primary_bytes = PrimaryBytes {
        content_type,
        schema: Some(schema.clone()),
        format_version,
        hash_scheme: "cem-text/1+blake3".to_owned(),
        hash: cem_ql_text_content_hash(&bytes),
        bytes,
    };
    let hash = primary_bytes.hash.clone();
    ConvertResponse {
        primary: json!({
            "kind": "document",
            "contentType": primary_bytes.content_type,
            "schema": schema,
            "hash": hash,
        }),
        primary_bytes: Some(primary_bytes),
        conversion: Some(cem_ql_convert_metadata(
            &request.target_scope,
            &color_profile,
            output_color_selection.as_ref(),
            html_output,
        )),
        diagnostics,
        scheduler_trace: cem_ml::report::SchedulerTraceReport::default(),
    }
}

fn cem_ql_output_pipeline(
    target_scope: &ScopeConfig,
    formatter_profile: &str,
    color_profile: &str,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
) -> ConversionOutputPipeline {
    let cemt_target =
        TransformTemplateEncodingTarget::new(CEM_QL_CONTENT_TYPE, CEM_QL_SCHEMA_URI, "cem-tree");
    let line_ending = target_scope
        .cemt_formatter_options
        .get("lineEnding")
        .cloned();
    let options = TransformTemplateEncodeOptions {
        formatter: Some(
            target_scope
                .cemt_formatter
                .clone()
                .unwrap_or_else(|| CEM_QL_DIRECT_FORMATTER.to_owned()),
        ),
        colorizer: Some(
            target_scope
                .cemt_colorizer
                .clone()
                .unwrap_or_else(|| CEM_QL_DIRECT_COLORIZER.to_owned()),
        ),
        formatter_profile: Some(formatter_profile.to_owned()),
        color_profile: Some(color_profile.to_owned()),
        formatter_options: target_scope.cemt_formatter_options.clone(),
        line_ending,
        mode: TransformTemplateEncodedArtifactMode::Document,
        canonical: false,
        source_map_policy: TransformTemplateSourceMapPolicy::Generated,
        ..TransformTemplateEncodeOptions::default()
    };

    let mut cemt_context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        &cemt_target,
        Some(TransformTemplateOutputProducedKind::CemTree),
    );
    cemt_context.formatter_profile = Some(formatter_profile.to_owned());
    cemt_context.color_profile = Some(color_profile.to_owned());
    cemt_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    cemt_context.canonical = Some(false);
    cemt_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);

    let mut writer_context = TransformTemplateEncodedArtifactInsertionContext::from_encoding_target(
        &cemt_target,
        Some(TransformTemplateOutputProducedKind::Text),
    );
    writer_context.formatter_profile = Some(formatter_profile.to_owned());
    writer_context.color_profile = Some(color_profile.to_owned());
    writer_context.mode = Some(TransformTemplateEncodedArtifactMode::Document);
    writer_context.source_map_policy = Some(TransformTemplateSourceMapPolicy::Generated);
    if let Some(selection) = output_color_selection {
        writer_context.output_color_type = Some(selection.output_color_type.clone());
        if cem_ql_output_color_selection_is_terminal(selection) {
            writer_context.color_capability = Some(selection.output_color_type.clone());
        }
    }

    ConversionOutputPipeline {
        stages: vec![
            ConversionOutputPipelineStage::Transform,
            ConversionOutputPipelineStage::Format,
            ConversionOutputPipelineStage::Color,
            ConversionOutputPipelineStage::Writer,
        ],
        cemt_target,
        cemt_options: options,
        cemt_insertion_context: cemt_context,
        cemt_produces: TransformTemplateOutputProducedKind::CemTree,
        writer_insertion_context: writer_context,
        writer_produces: TransformTemplateOutputProducedKind::Text,
    }
}

fn cem_ql_source_token_tree(input_uri: &str, source: &str) -> Value {
    let tokens = Lexer::new(source)
        .scan_all()
        .into_iter()
        .filter(|token| token.kind != TokenKind::EndOfInput)
        .enumerate()
        .map(|(index, token)| cem_ql_source_token_node(input_uri, source, index, &token))
        .collect::<Vec<_>>();
    json!({
        "kind": "cem-ql-source",
        "contentType": CEM_QL_CONTENT_TYPE,
        "schema": CEM_QL_SCHEMA_URI,
        "sourceUri": input_uri,
        "tokens": tokens,
    })
}

fn cem_ql_source_token_node(input_uri: &str, source: &str, index: usize, token: &Token) -> Value {
    let lexeme = cem_ql_token_lexeme(source, token);
    let role = cem_ql_token_role(token.kind);
    let source_map = cem_ql_source_map(token.range);
    let output_span = OutputSpan {
        output_range: ByteRange::new(0, token.range.len),
        origin: source_map.clone(),
    };
    let mut value = Map::new();
    value.insert(
        "tokenKind".to_owned(),
        Value::String(cem_ql_token_kind_name(token.kind).to_owned()),
    );
    value.insert("lexeme".to_owned(), Value::String(lexeme.to_owned()));
    value.insert(
        "byteOffset".to_owned(),
        Value::Number(Number::from(token.range.start)),
    );
    value.insert(
        "byteLength".to_owned(),
        Value::Number(Number::from(token.range.len)),
    );
    value.insert("index".to_owned(), Value::Number(Number::from(index)));
    value.insert("role".to_owned(), Value::String(role.to_owned()));
    if let Some(operator) = cem_ql_token_operator(token.kind, lexeme) {
        value.insert("operator".to_owned(), Value::String(operator.to_owned()));
        value.insert(
            "cemQlRole".to_owned(),
            Value::String(cem_ql_operator_role(operator).to_owned()),
        );
    }
    if token.kind == TokenKind::XPathCompatWord {
        value.insert("legacy".to_owned(), Value::String(lexeme.to_owned()));
        value.insert(
            "diagnostic".to_owned(),
            Value::String(cem_ql_legacy_diagnostic_code(lexeme).to_owned()),
        );
        value.insert(
            "replacement".to_owned(),
            Value::String(cem_ql_legacy_replacement(lexeme).to_owned()),
        );
    }
    if let Some(cooked) = token.cooked.as_ref() {
        value.insert("cooked".to_owned(), cem_ql_cooked_token_value(cooked));
    }

    json!({
        "kind": cem_ql_token_node_kind(token.kind),
        "tokenKind": cem_ql_token_kind_name(token.kind),
        "text": lexeme,
        "lexeme": lexeme,
        "role": role,
        "sourceUri": input_uri,
        "sourceMap": source_map,
        "outputSpan": output_span,
        "value": value,
    })
}

fn cem_ql_token_lexeme<'a>(source: &'a str, token: &Token) -> &'a str {
    let start = token.range.start as usize;
    let end = token.range.end() as usize;
    source.get(start..end).unwrap_or_default()
}

fn cem_ql_source_map(range: ByteRange) -> SourceMapStack {
    SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(0),
            span: FrameSpan::Single(range),
            transform: TransformKind::ContentTypeTransform {
                content_type: CEM_QL_CONTENT_TYPE.to_owned(),
            },
        }],
    }
}

fn cem_ql_cooked_token_value(cooked: &CookedTokenPayload) -> Value {
    match cooked {
        CookedTokenPayload::Name(name) => json!({ "kind": "name", "value": name }),
        CookedTokenPayload::PrefixedName { prefix, local } => {
            json!({ "kind": "prefixed-name", "prefix": prefix, "local": local })
        }
        CookedTokenPayload::StringValue(value) => json!({ "kind": "string", "value": value }),
        CookedTokenPayload::IntValue(value) => json!({ "kind": "integer", "value": value }),
        CookedTokenPayload::DecimalValue(value) => json!({ "kind": "decimal", "value": value }),
        CookedTokenPayload::DoubleValue(value) => json!({ "kind": "double", "value": value }),
        CookedTokenPayload::BoolValue(value) => json!({ "kind": "boolean", "value": value }),
    }
}

fn cem_ql_token_kind_name(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Dot => "Dot",
        TokenKind::Comma => "Comma",
        TokenKind::LParen => "LParen",
        TokenKind::RParen => "RParen",
        TokenKind::LBracket => "LBracket",
        TokenKind::RBracket => "RBracket",
        TokenKind::LBrace => "LBrace",
        TokenKind::RBrace => "RBrace",
        TokenKind::Semicolon => "Semicolon",
        TokenKind::Pipe => "Pipe",
        TokenKind::Amp => "Amp",
        TokenKind::Minus => "Minus",
        TokenKind::Caret => "Caret",
        TokenKind::Assign => "Assign",
        TokenKind::FatArrow => "FatArrow",
        TokenKind::Colon => "Colon",
        TokenKind::ColonColon => "ColonColon",
        TokenKind::EqEq => "EqEq",
        TokenKind::BangEq => "BangEq",
        TokenKind::Lt => "Lt",
        TokenKind::Le => "Le",
        TokenKind::Gt => "Gt",
        TokenKind::Ge => "Ge",
        TokenKind::Plus => "Plus",
        TokenKind::Star => "Star",
        TokenKind::Slash => "Slash",
        TokenKind::Percent => "Percent",
        TokenKind::AmpAmp => "AmpAmp",
        TokenKind::PipePipe => "PipePipe",
        TokenKind::Bang => "Bang",
        TokenKind::Dollar => "Dollar",
        TokenKind::Coalesce => "Coalesce",
        TokenKind::DotDot => "DotDot",
        TokenKind::Let => "Let",
        TokenKind::In => "In",
        TokenKind::If => "If",
        TokenKind::Else => "Else",
        TokenKind::For => "For",
        TokenKind::Import => "Import",
        TokenKind::As => "As",
        TokenKind::Declare => "Declare",
        TokenKind::Function => "Function",
        TokenKind::Module => "Module",
        TokenKind::IsKw => "IsKw",
        TokenKind::FnKw => "FnKw",
        TokenKind::XPathCompatWord => "XPathCompatWord",
        TokenKind::Ident => "Ident",
        TokenKind::PrefixedName => "PrefixedName",
        TokenKind::StringLit => "StringLit",
        TokenKind::IntLit => "IntLit",
        TokenKind::DecimalLit => "DecimalLit",
        TokenKind::DoubleLit => "DoubleLit",
        TokenKind::BoolLit => "BoolLit",
        TokenKind::NullLit => "NullLit",
        TokenKind::Whitespace => "Whitespace",
        TokenKind::LineComment => "LineComment",
        TokenKind::BlockComment => "BlockComment",
        TokenKind::Invalid => "Invalid",
        TokenKind::EndOfInput => "EndOfInput",
    }
}

fn cem_ql_token_node_kind(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Whitespace => "cem-ql.whitespace",
        TokenKind::LineComment | TokenKind::BlockComment => "cem-ql.comment",
        TokenKind::Let
        | TokenKind::In
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::For
        | TokenKind::Import
        | TokenKind::As
        | TokenKind::Declare
        | TokenKind::Function
        | TokenKind::Module
        | TokenKind::IsKw
        | TokenKind::FnKw
        | TokenKind::BoolLit
        | TokenKind::NullLit => "cem-ql.keyword",
        TokenKind::Ident | TokenKind::PrefixedName => "cem-ql.name",
        TokenKind::StringLit => "cem-ql.string",
        TokenKind::IntLit | TokenKind::DecimalLit | TokenKind::DoubleLit => "cem-ql.number",
        TokenKind::XPathCompatWord => "cem-ql.legacy-token",
        TokenKind::Invalid => "cem-ql.diagnostic-token",
        _ if cem_ql_token_operator(kind, "").is_some() => "cem-ql.operator",
        _ => "cem-ql.punctuation",
    }
}

fn cem_ql_token_role(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Whitespace => "source.whitespace",
        TokenKind::LineComment | TokenKind::BlockComment => "syntax.comment",
        TokenKind::Let
        | TokenKind::In
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::For
        | TokenKind::Import
        | TokenKind::As
        | TokenKind::Declare
        | TokenKind::Function
        | TokenKind::Module
        | TokenKind::IsKw
        | TokenKind::FnKw
        | TokenKind::BoolLit
        | TokenKind::NullLit => "syntax.keyword",
        TokenKind::Ident | TokenKind::PrefixedName => "syntax.name",
        TokenKind::StringLit => "syntax.string",
        TokenKind::IntLit | TokenKind::DecimalLit | TokenKind::DoubleLit => "syntax.number",
        TokenKind::XPathCompatWord | TokenKind::Invalid => "diagnostic.error",
        _ => "syntax.punctuation",
    }
}

fn cem_ql_token_operator(kind: TokenKind, lexeme: &str) -> Option<&str> {
    match kind {
        TokenKind::Pipe => Some("|"),
        TokenKind::Amp => Some("&"),
        TokenKind::Minus => Some("-"),
        TokenKind::Caret => Some("^"),
        TokenKind::EqEq => Some("=="),
        TokenKind::BangEq => Some("!="),
        TokenKind::Lt => Some("<"),
        TokenKind::Le => Some("<="),
        TokenKind::Gt => Some(">"),
        TokenKind::Ge => Some(">="),
        TokenKind::Plus => Some("+"),
        TokenKind::Star => Some("*"),
        TokenKind::Slash => Some("/"),
        TokenKind::Percent => Some("%"),
        TokenKind::AmpAmp => Some("&&"),
        TokenKind::PipePipe => Some("||"),
        TokenKind::Bang => Some("!"),
        TokenKind::Coalesce => Some("??"),
        TokenKind::Dot => Some("."),
        TokenKind::IsKw => Some("is"),
        TokenKind::As => Some("as"),
        TokenKind::XPathCompatWord => match lexeme {
            "eq" => Some("=="),
            "ne" => Some("!="),
            "lt" => Some("<"),
            "le" => Some("<="),
            "gt" => Some(">"),
            "ge" => Some(">="),
            "div" => Some("/"),
            "mod" => Some("%"),
            "and" => Some("&&"),
            "or" => Some("||"),
            "not" => Some("!"),
            _ => None,
        },
        _ => None,
    }
}

fn cem_ql_operator_role(operator: &str) -> &'static str {
    match operator {
        "==" | "!=" | "<" | "<=" | ">" | ">=" => "cem-ql.operator.comparison",
        "+" | "*" | "%" => "cem-ql.operator.arithmetic",
        "-" => "cem-ql.operator.arithmetic-or-set",
        "/" => "cem-ql.operator.arithmetic-or-child",
        "&&" | "||" | "!" => "cem-ql.operator.boolean",
        "??" => "cem-ql.operator.coalesce",
        "|" | "&" | "^" => "cem-ql.operator.set",
        "." => "cem-ql.operator.pipeline",
        "is" => "cem-ql.operator.type-test",
        "as" => "cem-ql.operator.cast",
        _ => "syntax.punctuation",
    }
}

fn cem_ql_legacy_diagnostic_code(token: &str) -> &'static str {
    match token {
        "and" | "or" | "not" => "cem.ql.use_rust_boolean_ops",
        _ => "cem.ql.parse_error",
    }
}

fn cem_ql_legacy_replacement(token: &str) -> &'static str {
    match token {
        "eq" => "==",
        "ne" => "!=",
        "lt" => "<",
        "le" => "<=",
        "gt" => ">",
        "ge" => ">=",
        "div" => "/",
        "mod" => "%",
        "and" => "&&",
        "or" => "||",
        "not" => "!",
        "then" => "if condition { then_expr } else { else_expr }",
        "return" => "for name in stream { expr }",
        "some" => "any(stream, fn)",
        "every" => "all(stream, fn)",
        "satisfies" => "any(stream, fn) or all(stream, fn)",
        "instance" => "expr is Type",
        "cast" => "expr as Type",
        "treat" => "treat_as(expr, Type)",
        "True" => "true",
        "False" => "false",
        "None" => "null",
        "lambda" => "fn(...) => expression",
        _ => "",
    }
}

fn cem_ql_parse_diagnostics(input_uri: &str, parsed: &ParseResult) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !parsed.module.nodes.iter().any(|node| {
        matches!(
            node,
            SurfaceNode::Module(module) if !module.uri.trim().is_empty()
        )
    }) {
        diagnostics.push(cem_ql_module_uri_missing_diagnostic(input_uri));
    }
    diagnostics.extend(parsed.diagnostics.iter().cloned().map(|mut diagnostic| {
        diagnostic.uri = Some(input_uri.to_owned());
        diagnostic
    }));
    diagnostics
}

fn cem_ql_invalid_utf8_diagnostic(input_uri: &str) -> Diagnostic {
    Diagnostic {
        uri: Some(input_uri.to_owned()),
        code: "cem.ql.invalid_utf8".to_owned(),
        severity: Severity::Error,
        message: "CEM-QL source must be valid UTF-8".to_owned(),
        ..Diagnostic::default()
    }
}

fn cem_ql_module_uri_missing_diagnostic(input_uri: &str) -> Diagnostic {
    Diagnostic {
        uri: Some(input_uri.to_owned()),
        code: "cem.ql.module_uri_missing".to_owned(),
        severity: Severity::Error,
        message: "CEM-QL module source requires a `module \"...\"` URI declaration".to_owned(),
        ..Diagnostic::default()
    }
}

fn cem_ql_output_pipeline_diagnostic(uri: Option<&str>, message: String) -> Diagnostic {
    Diagnostic {
        uri: uri.map(str::to_owned),
        code: "cem.converter.output_pipeline_execution".to_owned(),
        severity: Severity::Error,
        message,
        node: Some("cem-ql".to_owned()),
        ..Diagnostic::default()
    }
}

fn cem_ql_failed_convert_response(
    target_scope: &ScopeConfig,
    diagnostics: &[Diagnostic],
) -> ConvertResponse {
    ConvertResponse {
        primary: Value::Null,
        primary_bytes: None,
        conversion: Some(cem_ql_convert_metadata(target_scope, "none", None, false)),
        diagnostics: diagnostics.to_vec(),
        scheduler_trace: cem_ml::report::SchedulerTraceReport::default(),
    }
}

fn cem_ql_formatter_profile(target_scope: &ScopeConfig) -> String {
    target_scope
        .cemt_formatter_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty())
        .unwrap_or("compact")
        .to_owned()
}

fn cem_ql_output_color_selection(
    context: &EngineContext,
    target_scope: &ScopeConfig,
    target: Option<&FormatIdentity>,
    to_format: LayerFormat,
) -> Result<Option<TransformTemplateOutputColorSelection>, String> {
    if let Some(output_color_type) = target_scope
        .output_color_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let selection =
            parse_transform_template_output_color_type(output_color_type).map_err(|message| {
                format!("invalid CEM-QL output color type `{output_color_type}`: {message}")
            })?;
        if cem_ql_target_identity_is_html(context, target) || to_format == LayerFormat::Html {
            if selection.target.category != "html-color" {
                return Err(format!(
                    "CEM-QL HTML output requires an HTML output color type; got `{output_color_type}`"
                ));
            }
        }
        return Ok(Some(selection));
    }

    if cem_ql_target_identity_is_html(context, target) || to_format == LayerFormat::Html {
        return parse_transform_template_output_color_type("html-css-vars")
            .map(Some)
            .map_err(|message| {
                format!("failed to select default CEM-QL HTML output color type: {message}")
            });
    }

    Ok(None)
}

fn cem_ql_cemt_color_profile_for_output(
    target_scope: &ScopeConfig,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
) -> Result<String, String> {
    let explicit = target_scope
        .cemt_color_profile
        .as_deref()
        .map(str::trim)
        .filter(|profile| !profile.is_empty());
    let inferred =
        output_color_selection.and_then(|selection| match selection.target.category.as_str() {
            "html-color" => Some("html"),
            "terminal-color" if selection.output_color_type == "none" => Some("none"),
            "terminal-color" => Some("terminal"),
            _ => None,
        });
    if let (Some(explicit), Some(inferred)) = (explicit, inferred) {
        if explicit != inferred {
            return Err(format!(
                "CEM-QL color profile `{explicit}` conflicts with output color type `{}`; use `{inferred}` or omit `--cemt-color-profile`",
                output_color_selection
                    .map(|selection| selection.output_color_type.as_str())
                    .unwrap_or_default()
            ));
        }
    }
    Ok(explicit
        .map(str::to_owned)
        .or_else(|| inferred.map(str::to_owned))
        .unwrap_or_else(|| "none".to_owned()))
}

fn cem_ql_output_color_selection_is_terminal(
    selection: &TransformTemplateOutputColorSelection,
) -> bool {
    selection.target.category == "terminal-color" && selection.output_color_type != "none"
}

fn cem_ql_direct_target_for_layer_format(to_format: LayerFormat) -> Option<FormatIdentity> {
    match to_format {
        LayerFormat::Html => Some(FormatIdentity {
            content_type: Some(HTML_CONTENT_TYPE.to_owned()),
            schema: Some(HTML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        }),
        _ => None,
    }
}

fn cem_ql_direct_target_supported(
    context: &EngineContext,
    target: Option<&FormatIdentity>,
    target_scope: &ScopeConfig,
    to_format: LayerFormat,
) -> bool {
    if target_scope
        .output_color_type
        .as_deref()
        .is_some_and(|value| parse_transform_template_output_color_type(value).is_ok())
        && target.is_none_or(|identity| {
            format_identity_matches_cem_ql(context, identity)
                || format_identity_matches_html(context, identity)
        })
    {
        return true;
    }
    target
        .map(|identity| {
            format_identity_matches_cem_ql(context, identity)
                || format_identity_matches_html(context, identity)
        })
        .unwrap_or(matches!(to_format, LayerFormat::Cem | LayerFormat::Html))
}

fn cem_ql_direct_output_is_html(
    context: &EngineContext,
    target: Option<&FormatIdentity>,
    target_scope: &ScopeConfig,
    to_format: LayerFormat,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
) -> bool {
    output_color_selection.is_some_and(|selection| selection.target.category == "html-color")
        || target_scope
            .cemt_color_profile
            .as_deref()
            .is_some_and(|profile| profile.trim() == "html")
        || cem_ql_target_identity_is_html(context, target)
        || to_format == LayerFormat::Html
}

fn cem_ql_target_identity_is_html(
    context: &EngineContext,
    target: Option<&FormatIdentity>,
) -> bool {
    target.is_some_and(|identity| format_identity_matches_html(context, identity))
}

fn format_identity_matches_cem_ql(context: &EngineContext, identity: &FormatIdentity) -> bool {
    let explicit_schema_matches = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == CEM_QL_SCHEMA_URI);
    if let Some(content_type) = identity.content_type.as_deref() {
        let essence = content_type_essence(content_type);
        if essence == CEM_QL_CONTENT_TYPE || essence == "text/cem-ql" {
            return identity.schema.is_none() || explicit_schema_matches;
        }
        if let Ok(descriptor) = context.schema_registry.resolve_content_type(content_type) {
            return descriptor.schema_uri == CEM_QL_SCHEMA_URI
                && (identity.schema.is_none() || explicit_schema_matches);
        }
    }
    explicit_schema_matches
}

fn format_identity_matches_html(context: &EngineContext, identity: &FormatIdentity) -> bool {
    let explicit_schema_matches = identity
        .schema
        .as_deref()
        .map(str::trim)
        .is_some_and(|schema| schema == HTML_SCHEMA_URI);
    if let Some(content_type) = identity.content_type.as_deref() {
        let essence = content_type_essence(content_type);
        if essence == HTML_CONTENT_TYPE {
            return identity.schema.is_none() || explicit_schema_matches;
        }
        if let Ok(descriptor) = context.schema_registry.resolve_content_type(content_type) {
            return descriptor.schema_uri == HTML_SCHEMA_URI
                && (identity.schema.is_none() || explicit_schema_matches);
        }
    }
    explicit_schema_matches
}

fn cem_ql_direct_output_primary_identity(
    target: Option<&FormatIdentity>,
    html_output: bool,
) -> (String, String, String) {
    if html_output {
        return (
            HTML_CONTENT_TYPE.to_owned(),
            HTML_SCHEMA_URI.to_owned(),
            "cem-ql-html/1".to_owned(),
        );
    }
    (
        target
            .and_then(|identity| identity.content_type.clone())
            .unwrap_or_else(|| CEM_QL_CONTENT_TYPE.to_owned()),
        target
            .and_then(|identity| identity.schema.clone())
            .unwrap_or_else(|| CEM_QL_SCHEMA_URI.to_owned()),
        "cem-ql/1".to_owned(),
    )
}

fn cem_ql_convert_metadata(
    target_scope: &ScopeConfig,
    color_profile: &str,
    output_color_selection: Option<&TransformTemplateOutputColorSelection>,
    html_output: bool,
) -> ConvertExecutionMetadata {
    let formatter_profile = cem_ql_formatter_profile(target_scope);
    let writer_profile = output_color_selection
        .map(|selection| selection.output_color_type.clone())
        .or_else(|| Some(color_profile.to_owned()));
    let (writer_content_type, writer_schema, writer_category) = if html_output {
        (HTML_CONTENT_TYPE, HTML_SCHEMA_URI, "html-document")
    } else {
        (CEM_QL_CONTENT_TYPE, CEM_QL_SCHEMA_URI, "cem-tree")
    };
    ConvertExecutionMetadata {
        converter_id: Some(CEM_QL_DIRECT_OUTPUT_CONVERTER_ID.to_owned()),
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
                            .unwrap_or_else(|| CEM_QL_DIRECT_FORMATTER.to_owned()),
                    ),
                    profile: Some(formatter_profile),
                    content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                    category: Some("cem-tree".to_owned()),
                    produces: Some(
                        TransformTemplateOutputProducedKind::CemTree
                            .as_str()
                            .to_owned(),
                    ),
                },
                ConvertOutputPipelineStageMetadata {
                    stage: "colorizer".to_owned(),
                    function: Some(
                        target_scope
                            .cemt_colorizer
                            .clone()
                            .unwrap_or_else(|| CEM_QL_DIRECT_COLORIZER.to_owned()),
                    ),
                    profile: Some(color_profile.to_owned()),
                    content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                    category: Some("cem-tree".to_owned()),
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

fn cem_ql_text_content_hash(bytes: &[u8]) -> String {
    format!("cem-text/1+blake3:{}", blake3::hash(bytes).to_hex())
}

fn checked_byte_len(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

fn matches_cem_native_identity(identity: &FormatIdentity) -> bool {
    if let Some(content_type) = identity.content_type.as_deref() {
        return matches!(
            content_type_essence(content_type).as_str(),
            "application/cem+xml"
                | "application/cem"
                | cem_ml::schema::registry::CEM_NATIVE_TEMPLATE_CONTENT_TYPE
                | cem_ml::schema::registry::CEM_TRANSFORM_CONTENT_TYPE
                | "text/cem"
                | "text/cem-ml"
        );
    }

    let schema = identity
        .schema
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if !schema.is_empty() {
        return schema == CEM_NATIVE_TEMPLATE_SCHEMA_URI
            || schema == cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI
            || schema == cem_ml::schema::ir::CEM_CORE_NAMESPACE;
    }

    identity.default_namespace.as_deref() == Some(cem_ml::schema::ir::CEM_CORE_NAMESPACE)
        || identity
            .namespaces
            .values()
            .any(|uri| uri == cem_ml::schema::ir::CEM_CORE_NAMESPACE)
}

fn matches_cem_ql_expression_identity(identity: &FormatIdentity) -> bool {
    if let Some(content_type) = identity.content_type.as_deref() {
        return content_type_essence(content_type)
            == cem_ml::schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE;
    }

    identity.schema.as_deref().map(str::trim)
        == Some(cem_ml::schema::registry::CEM_QL_EXPRESSION_SCHEMA_URI)
}

fn matches_xslt_identity(identity: &FormatIdentity) -> bool {
    if let Some(content_type) = identity.content_type.as_deref() {
        let essence = content_type_essence(content_type);
        return TEMPLATE_CONTENT_TYPES
            .iter()
            .any(|allowed| *allowed == essence);
    }

    identity.default_namespace.as_deref() == Some(cem_ml::schema::xslt::XSL_NAMESPACE)
        || identity
            .namespaces
            .values()
            .any(|uri| uri == cem_ml::schema::xslt::XSL_NAMESPACE)
}

fn render_cem_ql_payload(
    adapter_id: &'static str,
    request: TransformTemplateRenderRequest<'_>,
) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
    let payload = request
        .compiled
        .native_payload::<CemQlCompiledTemplatePayload>()
        .ok_or_else(|| {
            TransformTemplateAdapterError::failed(
                adapter_id,
                TransformTemplateAdapterExecutionPhase::Render,
                "compiled template artifact was not produced by the CEM-QL adapter",
            )
        })?;
    let data = template_data_from_artifacts(request.primary_input, request.secondary_inputs);
    let plan = render_payload_template(payload, &data);
    if target_is_cem_tree(request.target) {
        return Ok(TransformTemplateRenderResponse {
            output: TransformTemplateOutputArtifact {
                uri: None,
                identity: request.target.cloned(),
                value: render_plan_to_cem_tree_nodes(&plan),
                source_map: None,
                output_spans: Vec::new(),
            },
            diagnostics: plan.diagnostics,
        });
    }
    let rendered = if target_content_type_is(request.target, "application/xml") {
        render_plan_to_xml_with_source_map(&plan)
    } else {
        render_plan_to_html_with_source_map(&plan)
    };
    let identity = request.target.cloned().or_else(|| {
        Some(FormatIdentity {
            content_type: Some("text/html".to_owned()),
            ..FormatIdentity::default()
        })
    });

    Ok(TransformTemplateRenderResponse {
        output: TransformTemplateOutputArtifact {
            uri: None,
            identity,
            value: Value::String(rendered.rendered),
            source_map: Some(rendered.source_map),
            output_spans: rendered.output_spans,
        },
        diagnostics: rendered.diagnostics,
    })
}

fn render_cem_ql_expression_payload(
    adapter_id: &'static str,
    request: TransformTemplateRenderRequest<'_>,
) -> TransformTemplateAdapterResult<TransformTemplateRenderResponse> {
    let payload = request
        .compiled
        .native_payload::<CemQlCompiledExpressionPayload>()
        .ok_or_else(|| {
            TransformTemplateAdapterError::failed(
                adapter_id,
                TransformTemplateAdapterExecutionPhase::Render,
                "compiled expression artifact was not produced by the CEM-QL expression adapter",
            )
        })?;
    let result = evaluate(
        &payload.compiled.query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root(),
            diagnostics: Vec::new(),
            policy_bindings: expression_policy_bindings(request.primary_input, &payload.params),
            current_item: None,
        },
    );
    let diagnostics = diagnostics_with_uri(&result.diagnostics, &payload.template_uri);

    Ok(TransformTemplateRenderResponse {
        output: TransformTemplateOutputArtifact {
            uri: None,
            identity: Some(FormatIdentity {
                content_type: Some(cem_ml::schema::registry::JSON_CONTENT_TYPE.to_owned()),
                schema: Some(cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            value: item_stream_json(&result),
            source_map: None,
            output_spans: Vec::new(),
        },
        diagnostics,
    })
}

fn target_is_cem_tree(target: Option<&FormatIdentity>) -> bool {
    target.is_some_and(|identity| {
        identity
            .content_type
            .as_deref()
            .is_some_and(|content_type| content_type_essence(content_type) == CEM_ML_CONTENT_TYPE)
            && identity
                .schema
                .as_deref()
                .is_some_and(|schema| schema.trim() == CEM_ML_SCHEMA_URI)
    })
}

fn target_content_type_is(target: Option<&FormatIdentity>, expected: &str) -> bool {
    target
        .and_then(|identity| identity.content_type.as_deref())
        .is_some_and(|content_type| content_type_essence(content_type) == expected)
}

fn content_type_essence(content_type: &str) -> String {
    content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase()
}

fn render_plan_to_cem_tree_nodes(plan: &RenderPlan) -> Value {
    Value::Array(
        plan.nodes
            .iter()
            .filter_map(render_plan_node_to_cem_tree)
            .collect(),
    )
}

fn render_plan_node_to_cem_tree(node: &RenderPlanNode) -> Option<Value> {
    match node {
        RenderPlanNode::Element { tag, .. } if tag.trim().is_empty() => None,
        RenderPlanNode::Element {
            tag,
            namespace,
            attributes,
            children,
            source_map,
        } => Some(json!({
            "kind": "element",
            "name": render_plan_cem_tree_name(tag, namespace.as_deref()),
            "attributes": attributes
                .iter()
                .map(render_plan_attribute_to_cem_tree)
                .collect::<Vec<_>>(),
            "children": children
                .iter()
                .filter_map(render_plan_node_to_cem_tree)
                .collect::<Vec<_>>(),
            "sourceMap": render_plan_source_map_value(source_map),
        })),
        RenderPlanNode::Text { text, source_map } if text.trim().is_empty() => Some(json!({
            "kind": "whitespace",
            "data": text,
            "sourceMap": render_plan_source_map_value(source_map),
        })),
        RenderPlanNode::Text { text, source_map } => Some(json!({
            "kind": "text",
            "value": text,
            "sourceMap": render_plan_source_map_value(source_map),
        })),
        RenderPlanNode::Comment { text, source_map } => Some(json!({
            "kind": "comment",
            "data": text,
            "sourceMap": render_plan_source_map_value(source_map),
        })),
        RenderPlanNode::Cdata { text, source_map } => Some(json!({
            "kind": "cdata",
            "data": text,
            "sourceMap": render_plan_source_map_value(source_map),
        })),
        RenderPlanNode::ProcessingInstruction {
            target,
            data,
            source_map,
        } => Some(json!({
            "kind": "processing-instruction",
            "name": target,
            "target": target,
            "data": data,
            "sourceMap": render_plan_source_map_value(source_map),
        })),
    }
}

fn render_plan_attribute_to_cem_tree(attribute: &RenderPlanAttribute) -> Value {
    json!({
        "kind": "attribute",
        "name": render_plan_cem_tree_name(&attribute.name, attribute.namespace.as_deref()),
        "value": attribute.value,
        "sourceMap": render_plan_source_map_value(&attribute.source_map),
    })
}

fn render_plan_cem_tree_name(local_name: &str, namespace: Option<&str>) -> String {
    namespace
        .map(str::trim)
        .filter(|namespace| !namespace.is_empty())
        .map(|namespace| format!("{namespace}:{local_name}"))
        .unwrap_or_else(|| local_name.to_owned())
}

fn render_plan_source_map_value(source_map: &cem_ml::source_map::SourceMapStack) -> Value {
    serde_json::to_value(source_map).unwrap_or(Value::Null)
}

fn host_binding_names(
    params: &BTreeMap<String, Value>,
    data_bindings: &[String],
    module_options: &TransformTemplateModuleOptions,
) -> Vec<String> {
    let mut bindings = data_bindings.to_vec();
    for name in params.keys() {
        push_binding_name(&mut bindings, name);
    }
    extend_module_param_binding_names(&mut bindings, module_options);
    bindings
}

fn extend_module_param_binding_names(
    bindings: &mut Vec<String>,
    module_options: &TransformTemplateModuleOptions,
) {
    for param in &module_options.params {
        push_binding_name(bindings, &param.name);
        if let Some((_, local)) = param.name.split_once('.') {
            push_binding_name(bindings, local);
        }
    }
}

fn push_binding_name(bindings: &mut Vec<String>, name: &str) {
    if !bindings.iter().any(|binding| binding == name) {
        bindings.push(name.to_owned());
    }
}

fn compile_preflighted_modules(
    adapter_id: &'static str,
    request: &TransformTemplateCompileRequest<'_>,
    host_bindings: &[String],
) -> TransformTemplateAdapterResult<Vec<CemQlCompiledTemplateModulePayload>> {
    request
        .module_preflight
        .resolved_imports
        .iter()
        .map(|module| {
            let source = std::str::from_utf8(&module.bytes).map_err(|err| {
                TransformTemplateAdapterError::failed(
                    adapter_id,
                    TransformTemplateAdapterExecutionPhase::Compile,
                    format!("template module `{}` is not valid UTF-8: {err}", module.uri),
                )
            })?;
            let module_options = parse_imported_module_options(
                adapter_id,
                module.uri.clone(),
                module.bytes.clone(),
                module.identity.clone(),
            )?;
            let mut module_host_bindings = host_bindings.to_vec();
            extend_module_param_binding_names(&mut module_host_bindings, &module_options);
            let artifact = compile_template(
                source,
                &CompileTemplateOptions {
                    host_bindings: module_host_bindings,
                    ..CompileTemplateOptions::default()
                },
            );
            let entrypoints = extract_template_entrypoints(&artifact);
            Ok(CemQlCompiledTemplateModulePayload {
                alias: module.alias.clone(),
                parent_uri: module.parent_uri.clone(),
                uri: module.uri.clone(),
                content_hash: module.content_hash.clone(),
                artifact,
                entrypoints,
                imports: imported_module_aliases(&request.module_preflight, module.uri.as_str()),
                param_declarations: module_options.params,
            })
        })
        .collect()
}

fn imported_module_aliases(
    module_preflight: &TransformTemplateModulePreflight,
    parent_uri: &str,
) -> BTreeMap<String, String> {
    module_preflight
        .resolved_imports
        .iter()
        .filter(|module| module.parent_uri.as_deref() == Some(parent_uri))
        .map(|module| (module.alias.clone(), module.uri.clone()))
        .collect()
}

fn parse_imported_module_options(
    adapter_id: &'static str,
    uri: String,
    bytes: Vec<u8>,
    identity: Option<FormatIdentity>,
) -> TransformTemplateAdapterResult<TransformTemplateModuleOptions> {
    let response = parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
        template: TemplateInput {
            uri: uri.clone(),
            bytes,
            identity,
            root_scope: ScopeConfig::default(),
        },
    });
    if let Some(diagnostic) = response
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == Severity::Fatal)
    {
        return Err(TransformTemplateAdapterError::failed(
            adapter_id,
            TransformTemplateAdapterExecutionPhase::Compile,
            format!(
                "template module `{uri}` declarations failed to lower: {}",
                diagnostic.message
            ),
        ));
    }
    Ok(response.module_options)
}

fn extract_template_entrypoints(artifact: &TemplateArtifact) -> CemQlTemplateEntrypoints {
    let mut entrypoints = CemQlTemplateEntrypoints::default();
    if let Some(module_body) = artifact.nodes.iter().find_map(module_body_node_children) {
        entrypoints.implicit = Some(artifact_from_nodes(artifact, module_body.clone()));
    } else if artifact
        .nodes
        .iter()
        .all(|node| module_node_children(node).is_none())
    {
        entrypoints.implicit = Some(artifact.clone());
    }

    collect_template_entrypoints(&artifact.nodes, artifact, &mut entrypoints);
    entrypoints
}

fn collect_template_entrypoints(
    nodes: &[TemplateNode],
    source: &TemplateArtifact,
    entrypoints: &mut CemQlTemplateEntrypoints,
) {
    for node in nodes {
        let TemplateNode::Element {
            tag,
            attributes,
            children,
            ..
        } = node
        else {
            continue;
        };

        if local_name(tag) == "template" {
            if let Some(name) = literal_attribute(attributes, "name") {
                entrypoints.named.insert(
                    name,
                    artifact_from_nodes(source, template_body_nodes(children)),
                );
            }
        }

        collect_template_entrypoints(children, source, entrypoints);
    }
}

fn module_node_children(node: &TemplateNode) -> Option<&Vec<TemplateNode>> {
    let TemplateNode::Element { tag, children, .. } = node else {
        return None;
    };
    (local_name(tag) == "module").then_some(children)
}

fn module_body_node_children(node: &TemplateNode) -> Option<&Vec<TemplateNode>> {
    module_node_children(node)?
        .iter()
        .find_map(body_node_children)
}

fn body_node_children(node: &TemplateNode) -> Option<&Vec<TemplateNode>> {
    let TemplateNode::Element { tag, children, .. } = node else {
        return None;
    };
    (local_name(tag) == "body").then_some(children)
}

fn template_body_nodes(children: &[TemplateNode]) -> Vec<TemplateNode> {
    children
        .iter()
        .find_map(body_node_children)
        .cloned()
        .unwrap_or_else(|| children.to_vec())
}

fn artifact_from_nodes(source: &TemplateArtifact, nodes: Vec<TemplateNode>) -> TemplateArtifact {
    TemplateArtifact {
        nodes,
        diagnostics: source.diagnostics.clone(),
    }
}

fn diagnostics_with_uri(diagnostics: &[Diagnostic], uri: &str) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .cloned()
        .map(|mut diagnostic| {
            if diagnostic.uri.is_none() {
                diagnostic.uri = Some(uri.to_owned());
            }
            diagnostic
        })
        .collect()
}

fn fill_diagnostic_uri(diagnostics: &mut [Diagnostic], uri: &str) {
    for diagnostic in diagnostics {
        if diagnostic.uri.is_none() {
            diagnostic.uri = Some(uri.to_owned());
        }
    }
}

fn is_cemt_encode_compile_diagnostic(diagnostic: &Diagnostic) -> bool {
    diagnostic.code == "cem.ql.render.compile_failed"
        && diagnostic.message.contains("template expression `encode(")
}

fn clear_template_artifact_diagnostics(artifact: &mut TemplateArtifact) {
    artifact.diagnostics.clear();
}

fn protect_transform_call_artifact(mut artifact: TemplateArtifact) -> TemplateArtifact {
    protect_transform_call_nodes(&mut artifact.nodes);
    artifact
}

fn protect_transform_call_entrypoints(entrypoints: &mut CemQlTemplateEntrypoints) {
    if let Some(artifact) = &mut entrypoints.implicit {
        protect_transform_call_nodes(&mut artifact.nodes);
    }
    for artifact in entrypoints.named.values_mut() {
        protect_transform_call_nodes(&mut artifact.nodes);
    }
}

fn protect_transform_call_nodes(nodes: &mut [TemplateNode]) {
    for node in nodes {
        match node {
            TemplateNode::Element { tag, children, .. } => {
                if local_name(tag) == "call" {
                    *tag = TRANSFORM_CALL_NODE.to_owned();
                }
                protect_transform_call_nodes(children);
            }
            TemplateNode::If { children, .. } | TemplateNode::ForEach { children, .. } => {
                protect_transform_call_nodes(children);
            }
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    protect_transform_call_nodes(&mut branch.children);
                }
            }
            TemplateNode::Text { .. }
            | TemplateNode::Comment { .. }
            | TemplateNode::Expression(_) => {}
        }
    }
}

fn clear_template_entrypoint_diagnostics(entrypoints: &mut CemQlTemplateEntrypoints) {
    if let Some(artifact) = &mut entrypoints.implicit {
        clear_template_artifact_diagnostics(artifact);
    }
    for artifact in entrypoints.named.values_mut() {
        clear_template_artifact_diagnostics(artifact);
    }
}

fn select_entrypoint_artifact(
    fallback: &TemplateArtifact,
    entrypoints: &CemQlTemplateEntrypoints,
    selected: Option<&str>,
) -> TemplateArtifact {
    match selected {
        Some(name) => entrypoints
            .named
            .get(name)
            .cloned()
            .unwrap_or_else(|| fallback.clone()),
        None => entrypoints
            .implicit
            .clone()
            .unwrap_or_else(|| fallback.clone()),
    }
}

fn render_payload_template(
    payload: &CemQlCompiledTemplatePayload,
    data: &TemplateData,
) -> RenderPlan {
    let data = root_template_data_with_params(payload, data);
    let mut plan = render_compiled_template(&payload.artifact, &data);
    fill_diagnostic_uri(&mut plan.diagnostics, payload.template_uri.as_str());
    let nodes = expand_call_nodes(&plan.nodes, payload, None, &data, 0, &mut plan.diagnostics);
    RenderPlan {
        nodes,
        diagnostics: plan.diagnostics,
    }
}

fn root_template_data_with_params(
    payload: &CemQlCompiledTemplatePayload,
    data: &TemplateData,
) -> TemplateData {
    let mut data = data.clone();
    for (name, value) in &payload.params {
        bind_param_value(
            &mut data,
            payload.selected_entrypoint.as_deref(),
            name,
            value,
        );
    }
    apply_param_declarations(
        &mut data,
        &payload.param_declarations,
        payload.selected_entrypoint.as_deref(),
    );
    data
}

fn expand_call_nodes(
    nodes: &[RenderPlanNode],
    payload: &CemQlCompiledTemplatePayload,
    current_module: Option<&CemQlCompiledTemplateModulePayload>,
    data: &TemplateData,
    depth: u32,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<RenderPlanNode> {
    nodes
        .iter()
        .flat_map(|node| expand_call_node(node, payload, current_module, data, depth, diagnostics))
        .collect()
}

fn expand_call_node(
    node: &RenderPlanNode,
    payload: &CemQlCompiledTemplatePayload,
    current_module: Option<&CemQlCompiledTemplateModulePayload>,
    data: &TemplateData,
    depth: u32,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<RenderPlanNode> {
    let RenderPlanNode::Element {
        tag,
        namespace,
        attributes,
        children,
        source_map,
    } = node
    else {
        return vec![node.clone()];
    };

    if local_name(tag) == "call" || tag == TRANSFORM_CALL_NODE {
        return render_call_node(
            attributes,
            payload,
            current_module,
            data,
            depth,
            diagnostics,
            source_map,
        );
    }

    vec![RenderPlanNode::Element {
        tag: tag.clone(),
        namespace: namespace.clone(),
        attributes: attributes.clone(),
        children: expand_call_nodes(children, payload, current_module, data, depth, diagnostics),
        source_map: source_map.clone(),
    }]
}

fn render_call_node(
    attributes: &[RenderPlanAttribute],
    payload: &CemQlCompiledTemplatePayload,
    current_module: Option<&CemQlCompiledTemplateModulePayload>,
    data: &TemplateData,
    depth: u32,
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &cem_ml::source_map::SourceMapStack,
) -> Vec<RenderPlanNode> {
    let call_site_uri = current_template_uri(payload, current_module);
    if depth >= payload.max_recursion_depth {
        diagnostics.push(module_render_diagnostic(
            TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE,
            format!(
                "native template call recursion limit exceeded at depth {depth}; max depth is {}",
                payload.max_recursion_depth
            ),
            call_site_uri,
            source_map.clone(),
        ));
        return Vec::new();
    }

    let Some(template) = render_attr(attributes, "template") else {
        diagnostics.push(module_render_diagnostic(
            TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE,
            "native template call is missing a `template` target",
            call_site_uri,
            source_map.clone(),
        ));
        return Vec::new();
    };
    let from = render_attr(attributes, "from");

    let (target, target_module) = match from.as_deref() {
        Some(alias) => match current_module {
            Some(current) => {
                let module = current
                    .imports
                    .get(alias)
                    .and_then(|uri| payload.modules.iter().find(|module| module.uri == *uri));
                (
                    module.and_then(|module| module.entrypoints.named.get(&template)),
                    module,
                )
            }
            None => {
                let module = payload
                    .modules
                    .iter()
                    .find(|module| module.parent_uri.is_none() && module.alias == alias);
                (
                    module.and_then(|module| module.entrypoints.named.get(&template)),
                    module,
                )
            }
        },
        None => match current_module {
            Some(module) => (module.entrypoints.named.get(&template), Some(module)),
            None => (payload.entrypoints.named.get(&template), None),
        },
    };

    let Some(target) = target else {
        diagnostics.push(module_render_diagnostic(
            TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE,
            format!("native template call target `{template}` was not compiled"),
            call_site_uri,
            source_map.clone(),
        ));
        return Vec::new();
    };

    let mut call_data = call_data_with_bindings(data, attributes);
    let param_declarations = param_declarations_for_module(payload, target_module);
    apply_param_declarations(&mut call_data, param_declarations, Some(template.as_str()));
    if !validate_call_params(
        &call_data,
        param_declarations,
        Some(template.as_str()),
        diagnostics,
        call_site_uri,
        source_map,
    ) {
        return Vec::new();
    }
    let mut plan = render_compiled_template(target, &call_data);
    fill_diagnostic_uri(
        &mut plan.diagnostics,
        current_template_uri(payload, target_module),
    );
    diagnostics.append(&mut plan.diagnostics);
    expand_call_nodes(
        &plan.nodes,
        payload,
        target_module,
        &call_data,
        depth + 1,
        diagnostics,
    )
}

fn current_template_uri<'a>(
    payload: &'a CemQlCompiledTemplatePayload,
    module: Option<&'a CemQlCompiledTemplateModulePayload>,
) -> &'a str {
    module
        .map(|module| module.uri.as_str())
        .unwrap_or(payload.template_uri.as_str())
}

fn param_declarations_for_module<'a>(
    payload: &'a CemQlCompiledTemplatePayload,
    module: Option<&'a CemQlCompiledTemplateModulePayload>,
) -> &'a [TransformTemplateModuleParamDeclaration] {
    module
        .map(|module| module.param_declarations.as_slice())
        .unwrap_or(payload.param_declarations.as_slice())
}

fn bind_param_value(
    data: &mut TemplateData,
    selected_entrypoint: Option<&str>,
    name: &str,
    value: &Value,
) {
    let stream = value_to_stream(value);
    data.bindings.insert(name.to_owned(), stream.clone());
    if let Some((qualified, local)) = entrypoint_param_aliases(name, selected_entrypoint) {
        data.bindings
            .entry(qualified)
            .or_insert_with(|| stream.clone());
        data.bindings.entry(local).or_insert(stream);
    }
}

fn apply_param_declarations(
    data: &mut TemplateData,
    declarations: &[TransformTemplateModuleParamDeclaration],
    selected_entrypoint: Option<&str>,
) {
    for declaration in declarations {
        if let Some((qualified, local)) =
            entrypoint_param_aliases(&declaration.name, selected_entrypoint)
        {
            normalize_param_aliases(data, qualified.as_str(), local.as_str());
            if !data.bindings.contains_key(&qualified) && !data.bindings.contains_key(&local) {
                if let Some(default_value) = &declaration.default_value {
                    let stream = value_to_stream(default_value);
                    data.bindings.insert(qualified, stream.clone());
                    data.bindings.insert(local, stream);
                }
            }
            continue;
        }

        if declaration.name.contains('.') {
            continue;
        }
        if !data.bindings.contains_key(&declaration.name) {
            if let Some(default_value) = &declaration.default_value {
                data.bindings
                    .insert(declaration.name.clone(), value_to_stream(default_value));
            }
        }
    }
}

fn validate_call_params(
    data: &TemplateData,
    declarations: &[TransformTemplateModuleParamDeclaration],
    selected_entrypoint: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
    call_site_uri: &str,
    source_map: &cem_ml::source_map::SourceMapStack,
) -> bool {
    let mut valid = true;
    for declaration in declarations {
        if declaration.name.contains('.')
            && entrypoint_param_aliases(&declaration.name, selected_entrypoint).is_none()
        {
            continue;
        }

        let display_name = call_param_display_name(declaration, selected_entrypoint);
        let stream = call_param_stream(data, declaration, selected_entrypoint);
        if stream.is_none_or(|stream| stream.items.is_empty()) {
            if declaration.required && declaration.default_value.is_none() {
                diagnostics.push(module_render_diagnostic(
                    TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE,
                    format!("native template call requires param `{display_name}`"),
                    call_site_uri,
                    source_map.clone(),
                ));
                valid = false;
            }
            continue;
        }

        let stream = stream.expect("stream is known to be present");
        if !stream_accepts_param_type(stream, declaration) {
            diagnostics.push(module_render_diagnostic(
                TRANSFORM_TEMPLATE_PARAM_TYPE_CODE,
                format!(
                    "native template call param `{display_name}` value does not match declared type `{}`",
                    declaration.value_type.as_contract_name()
                ),
                call_site_uri,
                source_map.clone(),
            ));
            valid = false;
        }
    }
    valid
}

fn call_param_stream<'a>(
    data: &'a TemplateData,
    declaration: &TransformTemplateModuleParamDeclaration,
    selected_entrypoint: Option<&str>,
) -> Option<&'a ItemStream> {
    if let Some((qualified, local)) =
        entrypoint_param_aliases(&declaration.name, selected_entrypoint)
    {
        return data
            .bindings
            .get(local.as_str())
            .or_else(|| data.bindings.get(qualified.as_str()));
    }
    data.bindings.get(declaration.name.as_str())
}

fn call_param_display_name(
    declaration: &TransformTemplateModuleParamDeclaration,
    selected_entrypoint: Option<&str>,
) -> String {
    entrypoint_param_aliases(&declaration.name, selected_entrypoint)
        .map(|(_, local)| local)
        .unwrap_or_else(|| declaration.name.clone())
}

fn stream_accepts_param_type(
    stream: &ItemStream,
    declaration: &TransformTemplateModuleParamDeclaration,
) -> bool {
    if stream.items.len() > 1 {
        return matches!(
            declaration.value_type,
            cem_ml::transform_template::TransformTemplateModuleParamType::Any
                | cem_ml::transform_template::TransformTemplateModuleParamType::Array
                | cem_ml::transform_template::TransformTemplateModuleParamType::Json
        );
    }

    let Some(item) = stream.items.first() else {
        return false;
    };
    item_accepts_param_type(item, declaration)
}

fn item_accepts_param_type(
    item: &Item,
    declaration: &TransformTemplateModuleParamDeclaration,
) -> bool {
    use cem_ml::transform_template::TransformTemplateModuleParamType as ParamType;

    if matches!(item, Item::Atomic(AtomValue::Null)) {
        return declaration.nullable;
    }

    match declaration.value_type {
        ParamType::Any => true,
        ParamType::String => matches!(
            item,
            Item::Atomic(AtomValue::String(_)) | Item::Atomic(AtomValue::AnyUri(_))
        ),
        ParamType::Boolean => matches!(item, Item::Atomic(AtomValue::Boolean(_))),
        ParamType::Number => matches!(
            item,
            Item::Atomic(AtomValue::Integer(_))
                | Item::Atomic(AtomValue::Decimal(_))
                | Item::Atomic(AtomValue::Double(_))
        ),
        ParamType::Integer => matches!(item, Item::Atomic(AtomValue::Integer(_))),
        ParamType::Array => matches!(item, Item::Array(_)),
        ParamType::Object => matches!(item, Item::Record(_)),
        ParamType::Json => matches!(item, Item::Atomic(_) | Item::Array(_) | Item::Record(_)),
    }
}

fn normalize_param_aliases(data: &mut TemplateData, qualified: &str, local: &str) {
    match (
        data.bindings.get(qualified).cloned(),
        data.bindings.get(local).cloned(),
    ) {
        (Some(stream), None) => {
            data.bindings.insert(local.to_owned(), stream);
        }
        (None, Some(stream)) => {
            data.bindings.insert(qualified.to_owned(), stream);
        }
        _ => {}
    }
}

fn entrypoint_param_aliases(
    name: &str,
    selected_entrypoint: Option<&str>,
) -> Option<(String, String)> {
    let entrypoint = selected_entrypoint?;
    if let Some(local) = name
        .strip_prefix(entrypoint)
        .and_then(|remaining| remaining.strip_prefix('.'))
        .filter(|local| !local.trim().is_empty())
    {
        return Some((name.to_owned(), local.to_owned()));
    }

    if !name.contains('.') && !name.trim().is_empty() {
        return Some((format!("{entrypoint}.{name}"), name.to_owned()));
    }
    None
}

fn render_attr(attributes: &[RenderPlanAttribute], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .map(|attribute| attribute.value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn call_data_with_bindings(
    data: &TemplateData,
    attributes: &[RenderPlanAttribute],
) -> TemplateData {
    let mut data = data.clone();
    for attribute in attributes {
        let Some(name) = attribute.name.strip_prefix("with:") else {
            continue;
        };
        if name.trim().is_empty() {
            continue;
        }
        data.bindings
            .insert(name.to_owned(), attribute.value_stream.clone());
    }
    data
}

fn literal_attribute(
    attributes: &[cem_ql::render::TemplateAttribute],
    name: &str,
) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .and_then(|attribute| match &attribute.value {
            Some(TemplateAttributeValue::Literal(value)) => Some(value.clone()),
            _ => None,
        })
        .filter(|value| !value.trim().is_empty())
}

fn local_name(name: &str) -> &str {
    name.rsplit_once(':')
        .map(|(_, local)| local)
        .unwrap_or(name)
}

fn module_render_diagnostic(
    code: &str,
    message: impl Into<String>,
    uri: &str,
    source_map: cem_ml::source_map::SourceMapStack,
) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.to_owned()),
        code: code.to_owned(),
        severity: Severity::Fatal,
        message: message.into(),
        source_map: Some(source_map),
        ..Diagnostic::default()
    }
}

fn template_data_from_artifacts(
    primary: &TransformTemplateDataArtifact,
    secondary: &BTreeMap<String, TransformTemplateDataArtifact>,
) -> TemplateData {
    let mut data = TemplateData::default().with_binding("input", value_to_stream(&primary.value));
    match &primary.value {
        Value::Object(fields) => {
            for (name, value) in fields {
                data = data.with_binding(name.clone(), value_to_stream(value));
            }
        }
        _ => {
            data = data.with_binding("value", value_to_stream(&primary.value));
        }
    }
    for (name, artifact) in secondary {
        data = data.with_binding(name.clone(), value_to_stream(&artifact.value));
    }
    data
}

fn expression_compile_context(
    template_uri: &str,
    params: &BTreeMap<String, Value>,
    data_bindings: &[String],
    resolver_policy_stamp: Option<String>,
) -> StandaloneExpressionContext {
    let mut context = StandaloneExpressionContext {
        source_uri: Some(template_uri.to_owned()),
        resolver_policy_stamp,
        host_capability_profile: Some("cem-ml-transform".to_owned()),
        ..StandaloneExpressionContext::default()
    };
    for binding in data_bindings {
        context = context.with_binding(
            binding.clone(),
            StandaloneExpressionBinding::new(ItemStream::empty(), Type::Any),
        );
    }
    for name in params.keys() {
        context = context.with_binding(
            name.clone(),
            StandaloneExpressionBinding::new(ItemStream::empty(), Type::Any),
        );
    }
    context
}

fn expression_policy_bindings(
    primary: &TransformTemplateDataArtifact,
    params: &BTreeMap<String, Value>,
) -> BTreeMap<String, ItemStream> {
    let mut bindings = BTreeMap::new();
    bindings.insert("input".to_owned(), value_to_stream(&primary.value));
    for (name, value) in params {
        bindings.insert(name.clone(), value_to_stream(value));
    }
    bindings
}

fn value_to_stream(value: &Value) -> ItemStream {
    match value {
        Value::Array(items) => ItemStream::from_items(items.iter().map(value_to_item).collect()),
        _ => ItemStream::once(value_to_item(value)),
    }
}

fn value_to_item(value: &Value) -> Item {
    match value {
        Value::Null => Item::Atomic(AtomValue::Null),
        Value::Bool(value) => Item::Atomic(AtomValue::Boolean(*value)),
        Value::Number(value) => number_to_item(value),
        Value::String(value) => Item::Atomic(AtomValue::String(value.clone())),
        Value::Array(items) => Item::Array(items.iter().map(value_to_item).collect()),
        Value::Object(fields) => Item::Record(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), vec![value_to_item(value)]))
                .collect(),
        ),
    }
}

fn number_to_item(value: &Number) -> Item {
    if let Some(value) = value.as_i64() {
        Item::Atomic(AtomValue::Integer(value))
    } else if let Some(value) = value.as_u64().and_then(|value| i64::try_from(value).ok()) {
        Item::Atomic(AtomValue::Integer(value))
    } else if let Some(value) = value.as_f64() {
        Item::Atomic(AtomValue::Double(value))
    } else {
        Item::Atomic(AtomValue::Decimal(value.to_string()))
    }
}

fn item_stream_json(stream: &ItemStream) -> Value {
    json!({
        "items": stream.items.iter().map(item_json).collect::<Vec<_>>(),
        "diagnostics": stream.diagnostics,
        "error": stream.error.as_ref().map(eval_error_json),
    })
}

fn item_json(item: &Item) -> Value {
    match item {
        Item::Node(id) => json!({
            "kind": "node",
            "id": id,
        }),
        Item::Atomic(atom) => atom_json(atom),
        Item::Record(fields) => {
            let fields = fields
                .iter()
                .map(|(key, values)| {
                    (
                        key.clone(),
                        Value::Array(values.iter().map(item_json).collect::<Vec<_>>()),
                    )
                })
                .collect::<Map<_, _>>();
            json!({
                "kind": "record",
                "fields": fields,
            })
        }
        Item::Array(items) => json!({
            "kind": "array",
            "items": items.iter().map(item_json).collect::<Vec<_>>(),
        }),
        Item::Lambda(id) => json!({
            "kind": "lambda",
            "id": id.0,
        }),
        Item::Resource(resource) => json!({
            "kind": "resource",
            "id": resource.id,
            "contentType": resource.content_type,
            "schema": resource.schema,
            "roles": resource.roles,
            "failAccessor": resource.fail_accessor,
        }),
    }
}

fn atom_json(atom: &AtomValue) -> Value {
    match atom {
        AtomValue::String(value) => typed_atom_json("string", json!(value)),
        AtomValue::Integer(value) => typed_atom_json("integer", json!(value)),
        AtomValue::Decimal(value) => typed_atom_json("decimal", json!(value)),
        AtomValue::Double(value) => typed_atom_json("double", double_json(*value)),
        AtomValue::Boolean(value) => typed_atom_json("boolean", json!(value)),
        AtomValue::AnyUri(value) => typed_atom_json("any-uri", json!(value)),
        AtomValue::Null => typed_atom_json("null", Value::Null),
    }
}

fn typed_atom_json(atom_type: &str, value: Value) -> Value {
    json!({
        "kind": "atomic",
        "type": atom_type,
        "value": value,
    })
}

fn double_json(value: f64) -> Value {
    if value.is_nan() {
        Value::String("NaN".to_owned())
    } else if value == f64::INFINITY {
        Value::String("Infinity".to_owned())
    } else if value == f64::NEG_INFINITY {
        Value::String("-Infinity".to_owned())
    } else {
        json!(value)
    }
}

fn eval_error_json(error: &EvalError) -> Value {
    match error {
        EvalError::BudgetExceeded(axis) => json!({
            "kind": "eval",
            "type": "budget-exceeded",
            "axis": budget_axis_json(*axis),
            "message": format!("budget exceeded for `{}`", axis.as_str()),
        }),
        EvalError::Unsupported(message) => json!({
            "kind": "eval",
            "type": "unsupported",
            "message": message,
        }),
        EvalError::TypeError(message) => json!({
            "kind": "eval",
            "type": "type-error",
            "message": message,
        }),
    }
}

fn budget_axis_json(axis: BudgetAxis) -> Value {
    json!(axis.as_str())
}

pub fn json_object(fields: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect::<Map<_, _>>(),
    )
}

pub fn unsupported_identity_error_message(identity: &FormatIdentity) -> String {
    format!(
        "{TRANSFORM_TEMPLATE_UNSUPPORTED_CODE}: CEM-QL template adapter does not support template identity {identity:?}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::engine::{CemMlEngine, EngineContext};
    use cem_ml::engine::{
        ConvertRequest, EngineInput, InputFormat, LayerFormat, TemplateInput,
        TransformExecutionPolicy, TransformGraphExport, TransformGraphImport, TransformGraphJoin,
        TransformGraphJoinInput, TransformGraphJoinMode, TransformGraphRequest,
        TransformGraphStage, TransformRequest, TransformRuntimePhase, TransformSchedulerScopeIds,
        TransformStageSchedulerScopeIds, TransformTemplateEntrypoint,
    };
    use cem_ml::events::cem::CemEventNormalizer;
    use cem_ml::interpreter::{light_dom::LightDomInterpreter, xml::XmlInterpreter};
    use cem_ml::parser::builder::CemAstBuilder;
    use cem_ml::parser::document::CemDocument;
    use cem_ml::projection;
    use cem_ml::real::RealCemMlEngine;
    use cem_ml::resolver::{
        has_uri_scheme, ResolveDirection, ResolvePurpose, ResolveRequest, ResolvedRead,
        ResolvedWrite, ResolverDiagnostic, ResolverRegistry, ResourceResolver,
    };
    use cem_ml::run_config::ScopeConfig;
    use cem_ml::schema::package_sources::builtin_schema_package_artifact_sources;
    use cem_ml::schema::registry::{
        CEM_QL_CONTENT_TYPE, CEM_QL_SCHEMA_URI, CEM_SCHEMA_PACKAGE_CONTENT_TYPE,
        CEM_SCHEMA_PACKAGE_URI, CEM_TRANSFORM_CONTENT_TYPE, HTML_CONTENT_TYPE, HTML_SCHEMA_URI,
    };
    use cem_ml::source::{BytesSource, SourceId};
    use cem_ml::tokenizer::{cem::CemTokenizer, xml::XmlTokenizer};
    use cem_ml::transform_template::{
        TransformTemplateAdapterLookup, TransformTemplateModuleParamType,
        TransformTemplateModulePreflight, TransformTemplateResolvedModule,
    };

    const CUSTOM_BEHAVIOR_SCHEMA_URI: &str = "https://example.test/ns/custom-behavior/1";
    const CUSTOM_BEHAVIOR_SCHEMA: &str =
        include_str!("../../cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem");
    const CUSTOM_BEHAVIOR_STRICT_SCHEMA_URI: &str =
        "https://example.test/ns/custom-behavior-strict/1";
    const CUSTOM_BEHAVIOR_STRICT_SCHEMA: &str = include_str!(
        "../../cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema-strict.cem"
    );
    const SCHEMA_PACKAGE_SCHEMA: &str =
        include_str!("../../cem_ml/schema-packages/schema-package/v1/schema/schema-package.cem");

    fn packaged_dom_projection_artifact(value: Value) -> TransformTemplateDataArtifact {
        TransformTemplateDataArtifact {
            artifact_id: "dom".to_owned(),
            uri: Some("dom.json".to_owned()),
            identity: Some(FormatIdentity {
                content_type: Some(
                    cem_ml::schema::registry::CEM_DOM_JSON_PROJECTION_CONTENT_TYPE.to_owned(),
                ),
                schema: Some(cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            value,
        }
    }

    fn packaged_dom_projection_input(children: Vec<Value>) -> TransformTemplateDataArtifact {
        packaged_dom_projection_artifact(json_object([
            ("kind", Value::String("document".to_owned())),
            ("children", Value::Array(children)),
        ]))
    }

    fn document_from_cem(source: &str) -> CemDocument {
        let source = BytesSource::new(SourceId(1), source.as_bytes().to_vec());
        let tokenizer = CemTokenizer::from_source(source);
        let events = CemEventNormalizer::new(tokenizer);
        CemAstBuilder::new(events).build()
    }

    fn document_from_xml(source: &str) -> CemDocument {
        let source = BytesSource::new(SourceId(1), source.as_bytes().to_vec());
        let tokenizer = XmlTokenizer::from_source(source);
        let events = CemEventNormalizer::new(tokenizer);
        CemAstBuilder::new(events).build()
    }

    #[test]
    fn checked_in_custom_behavior_schema_examples_change_validation_declaratively() {
        let original = cem_ml::schema::document_model::compile_schema_document_model(
            CUSTOM_BEHAVIOR_SCHEMA_URI,
            CUSTOM_BEHAVIOR_SCHEMA,
        );
        assert!(
            original.compile_diagnostics.is_empty(),
            "{:#?}",
            original.compile_diagnostics
        );
        let strict = cem_ml::schema::document_model::compile_schema_document_model(
            CUSTOM_BEHAVIOR_STRICT_SCHEMA_URI,
            CUSTOM_BEHAVIOR_STRICT_SCHEMA,
        );
        assert!(
            strict.compile_diagnostics.is_empty(),
            "{:#?}",
            strict.compile_diagnostics
        );

        let evaluator = CemQlSchemaBehaviorEvaluator;
        let unlabeled_page = document_from_cem(r#"{resource @kind=page}"#);
        let original_diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &unlabeled_page,
                &original,
                Some(&evaluator),
            );
        let diagnostic = original_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.page_label")
            .unwrap_or_else(|| {
                panic!("checked-in custom behavior diagnostic: {original_diagnostics:#?}")
            });
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(diagnostic.message, "Page resource needs a label");
        let details = diagnostic.details.as_ref().expect("diagnostic details");
        assert_eq!(details["behavior"], json!("page-label"));
        assert_eq!(details["function"], json!("page-label-result"));
        assert_eq!(details["expected"], json!("label"));

        let strict_diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &unlabeled_page,
                &strict,
                Some(&evaluator),
            );
        assert!(
            strict_diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "example.published_page_title"),
            "strict custom behavior should not match unpublished pages: {strict_diagnostics:#?}"
        );

        let published_without_title =
            document_from_cem(r#"{resource @kind=page @status=published}"#);
        let strict_diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &published_without_title,
                &strict,
                Some(&evaluator),
            );
        let diagnostic = strict_diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.published_page_title")
            .unwrap_or_else(|| {
                panic!("checked-in strict custom behavior diagnostic: {strict_diagnostics:#?}")
            });
        assert_eq!(diagnostic.severity, Severity::Error);
        assert_eq!(diagnostic.message, "Published page resource needs a title");
        let details = diagnostic.details.as_ref().expect("diagnostic details");
        assert_eq!(details["behavior"], json!("published-page-title"));
        assert_eq!(details["function"], json!("published-page-title-result"));
        assert_eq!(details["expected"], json!("title"));
        assert_eq!(details["status"], json!("published"));
    }

    #[test]
    fn schema_declared_diagnostic_behavior_executes_cem_ql_match_and_cem_ml_function() {
        let model = cem_ml::schema::document_model::compile_schema_document_model(
            "https://example.test/ns/declarative-diagnostic/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="declarative-diagnostic" @namespace="https://example.test/ns/declarative-diagnostic/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind label"}
    }
    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
        {attribute @name="label" @type="schema:string"}
    }
    {behaviors |
        {behavior
            @name="page-label"
            @implementation="function"
            @execution="ast-validation"
            @function="page-label-result"
            @select="resource"
            @match='kind == "page" && label == null' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true @default="label"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate" |
                {detail @name="checkKind" @type="schema:identifier" @required=true}
                {detail @name="element" @type="schema:identifier" @required=true}
                {detail @name="kind" @type="schema:identifier" @required=true}
                {detail @name="expected" @type="schema:string" @required=true}
                {detail @name="expectedFields" @type="schema:array" @required=true}
                {detail @name="sample" @type="schema:object" @required=true}
            }
            {function @name="page-label-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Page resource needs a label", details: { checkKind: "page-label", element: $candidate.name, kind: $candidate.attributes.kind, expected: $expected, expectedFields: [$expected], sample: { enabled: true, count: 1, nothing: null } } } }}
            }
        }
    }
    {diagnostics |
        {diagnostic @code="example.page_label" @severity="warning" @behavior="page-label"}
    }
}"#,
        );
        assert!(
            model.compile_diagnostics.is_empty(),
            "{:#?}",
            model.compile_diagnostics
        );
        let evaluator = CemQlSchemaBehaviorEvaluator;

        let document = document_from_cem(r#"{resource @kind=page}"#);
        let diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &document,
                &model,
                Some(&evaluator),
            );
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.page_label")
            .unwrap_or_else(|| panic!("declarative diagnostic behavior result: {diagnostics:#?}"));
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(diagnostic.message, "Page resource needs a label");
        let details = diagnostic.details.as_ref().expect("diagnostic details");
        assert_eq!(
            details["schemaUri"],
            json!("https://example.test/ns/declarative-diagnostic/1")
        );
        assert_eq!(details["diagnostic"], json!("example.page_label"));
        assert_eq!(details["behavior"], json!("page-label"));
        assert_eq!(details["function"], json!("page-label-result"));
        assert_eq!(details["checkKind"], json!("page-label"));
        assert_eq!(details["element"], json!("resource"));
        assert_eq!(details["kind"], json!("page"));
        assert_eq!(details["expected"], json!("label"));
        assert_eq!(details["expectedFields"], json!(["label"]));
        assert_eq!(
            details["sample"],
            json!({
                "enabled": true,
                "count": 1,
                "nothing": null,
            })
        );
        assert!(details["sourceRange"]["span"]["start"].is_u64());

        let labeled = document_from_cem(r#"{resource @kind=page @label=Home}"#);
        let diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &labeled,
                &model,
                Some(&evaluator),
            );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "example.page_label"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn checked_in_schema_package_primary_content_type_behavior_counts_primary_children() {
        let model = cem_ml::schema::document_model::compile_schema_document_model(
            CEM_SCHEMA_PACKAGE_URI,
            SCHEMA_PACKAGE_SCHEMA,
        );
        assert!(
            model.compile_diagnostics.is_empty(),
            "{:#?}",
            model.compile_diagnostics
        );
        let evaluator = CemQlSchemaBehaviorEvaluator;

        let valid = document_from_cem(
            r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="valid" @version="1.0.0" |
    {schema @uri="https://example.test/ns/valid/1" @source="schema/valid.cem"}
    {content-type @value="application/vnd.example.valid+cem" @primary=true}
    {content-type @value="application/vnd.example.valid-alias+cem" @alias=true}
    {content-type @value="application/vnd.example.valid-secondary+cem" @primary=false}
}"#,
        );
        let diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &valid,
                &model,
                Some(&evaluator),
            );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.code != "cem.schema_package.content_type_conflict"),
            "valid package should not fail primary content-type count: {diagnostics:#?}"
        );

        let invalid_missing = document_from_cem(
            r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="invalid-missing" @version="1.0.0" |
    {schema @uri="https://example.test/ns/invalid-missing/1" @source="schema/invalid-missing.cem"}
    {content-type @value="application/vnd.example.invalid-missing+cem" @alias=true}
    {content-type @value="application/vnd.example.invalid-missing-secondary+cem" @primary=false}
}"#,
        );
        let diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &invalid_missing,
                &model,
                Some(&evaluator),
            );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "cem.schema_package.content_type_conflict"),
            "package without a primary content type should fail exact-one primary count: {diagnostics:#?}"
        );

        let invalid_duplicate = document_from_cem(
            r#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="invalid" @version="1.0.0" |
    {schema @uri="https://example.test/ns/invalid/1" @source="schema/invalid.cem"}
    {content-type @value="application/vnd.example.invalid+cem" @primary=true}
    {content-type @value="application/vnd.example.invalid-alt+cem" @primary=true}
}"#,
        );
        let diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &invalid_duplicate,
                &model,
                Some(&evaluator),
            );
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "cem.schema_package.content_type_conflict")
            .unwrap_or_else(|| {
                panic!("primary content-type behavior diagnostic: {diagnostics:#?}")
            });
        assert_eq!(
            diagnostic.message,
            "Schema package must declare exactly one primary content type"
        );
        let details = diagnostic.details.as_ref().expect("diagnostic details");
        assert_eq!(details["behavior"], json!("single-primary-content-type"));
        assert_eq!(
            details["function"],
            json!("single-primary-content-type-result")
        );
        assert_eq!(details["checkKind"], json!("single-primary-content-type"));
        assert_eq!(details["contract"], json!("single-primary-content-type"));
        assert_eq!(details["expectedPrimaryContentTypes"], json!(1));
    }

    #[test]
    fn schema_declared_diagnostic_behavior_executes_qualified_reusable_cem_ml_function() {
        let model = cem_ml::schema::document_model::compile_schema_document_model(
            "https://example.test/ns/reusable-diagnostic/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="reusable-diagnostic" @namespace="https://example.test/ns/reusable-diagnostic/1" @version="1.0.0" |
    {uses |
        {use @schema="https://example.test/ns/reusable-diagnostic/1" @as="self"}
    }
    {elements |
        {element @name="resource" @optional-attributes="kind label"}
    }
    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
        {attribute @name="label" @type="schema:string"}
    }
    {behaviors |
        {behavior @name="diagnostic-results" @implementation="function" @execution="ast-validation" |
            {function @name="shared-label-result" @returns="object" @visibility="package" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Reusable label check failed", details: { checkKind: "shared-label", element: $candidate.name, expected: $expected } } }}
            }
        }
        {behavior
            @name="page-label"
            @implementation="function"
            @execution="ast-validation"
            @function="self:shared-label-result"
            @select="resource"
            @match='kind == "page" && label == null' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true @default="label"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate" |
                {detail @name="checkKind" @type="schema:identifier" @required=true}
                {detail @name="element" @type="schema:identifier" @required=true}
                {detail @name="expected" @type="schema:string" @required=true}
            }
        }
    }
    {diagnostics |
        {diagnostic @code="example.page_label" @severity="warning" @behavior="page-label"}
    }
}"#,
        );
        assert!(
            model.compile_diagnostics.is_empty(),
            "{:#?}",
            model.compile_diagnostics
        );
        let evaluator = CemQlSchemaBehaviorEvaluator;
        let document = document_from_cem(r#"{resource @kind=page}"#);
        let diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &document,
                &model,
                Some(&evaluator),
            );
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.page_label")
            .unwrap_or_else(|| panic!("qualified reusable diagnostic behavior: {diagnostics:#?}"));
        assert_eq!(diagnostic.message, "Reusable label check failed");
        let details = diagnostic.details.as_ref().expect("diagnostic details");
        assert_eq!(details["function"], json!("self:shared-label-result"));
        assert_eq!(details["checkKind"], json!("shared-label"));
        assert_eq!(details["element"], json!("resource"));
        assert_eq!(details["expected"], json!("label"));
    }

    #[test]
    fn schema_declared_diagnostic_behavior_binds_diagnostic_argument_override() {
        let model = cem_ml::schema::document_model::compile_schema_document_model(
            "https://example.test/ns/argument-diagnostic/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="argument-diagnostic" @namespace="https://example.test/ns/argument-diagnostic/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind title"}
    }
    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
        {attribute @name="title" @type="schema:string"}
    }
    {behaviors |
        {behavior
            @name="page-title"
            @implementation="function"
            @execution="ast-validation"
            @function="page-title-result"
            @select="resource"
            @match='kind == "page" && title == null' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {parameters |
                {parameter @name="expected" @type="schema:string" @required=true @default="label"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate" |
                {detail @name="expected" @type="schema:string" @required=true}
            }
            {function @name="page-title-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {param @name="expected" @type="string" @required=true}
                {body | {$ { message: "Page resource needs a field", details: { expected: $expected } } }}
            }
        }
    }
    {diagnostics |
        {diagnostic @code="example.page_title" @severity="warning" @behavior="page-title" |
            {arguments |
                {argument @name="expected" @value="title"}
            }
        }
    }
}"#,
        );
        assert!(
            model.compile_diagnostics.is_empty(),
            "{:#?}",
            model.compile_diagnostics
        );
        let evaluator = CemQlSchemaBehaviorEvaluator;
        let document = document_from_cem(r#"{resource @kind=page}"#);
        let diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &document,
                &model,
                Some(&evaluator),
            );
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "example.page_title")
            .unwrap_or_else(|| panic!("diagnostic argument override: {diagnostics:#?}"));
        let details = diagnostic.details.as_ref().expect("diagnostic details");
        assert_eq!(details["expected"], json!("title"));
    }

    #[test]
    fn schema_declared_diagnostic_behavior_rejects_cemt_call_body() {
        let model = cem_ml::schema::document_model::compile_schema_document_model(
            "https://example.test/ns/declarative-diagnostic/1",
            r#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema @name="declarative-diagnostic" @namespace="https://example.test/ns/declarative-diagnostic/1" @version="1.0.0" |
    {elements |
        {element @name="resource" @optional-attributes="kind"}
    }
    {attributes |
        {attribute @name="kind" @type="schema:identifier"}
    }
    {behaviors |
        {behavior
            @name="page-kind"
            @implementation="function"
            @execution="ast-validation"
            @function="page-kind-result"
            @select="resource"
            @match='kind == "page"' |
            {inputs |
                {input-binding @name="candidate" @type="schema:node" @source="candidate" @required=true @source-range="candidate"}
            }
            {result @type="schema:diagnostic-result" @source-range="candidate"}
            {function @name="page-kind-result" @returns="object" @deterministic=true |
                {param @name="candidate" @type="object" @required=true}
                {body | {$ call("page-kind-result", { candidate: $candidate }) }}
            }
        }
    }
    {diagnostics |
        {diagnostic @code="example.page_kind" @severity="warning" @behavior="page-kind"}
    }
}"#,
        );
        assert!(
            model.compile_diagnostics.is_empty(),
            "{:#?}",
            model.compile_diagnostics
        );
        let evaluator = CemQlSchemaBehaviorEvaluator;
        let document = document_from_cem(r#"{resource @kind=page}"#);
        let diagnostics =
            cem_ml::schema::document_model::validate_document_model_with_behavior_evaluator(
                &document,
                &model,
                Some(&evaluator),
            );
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == SCHEMA_BEHAVIOR_FUNCTION_FAILED_CODE)
            .unwrap_or_else(|| panic!("CEMT call body must be rejected: {diagnostics:#?}"));
        assert!(
            diagnostic
                .message
                .contains("invalid CEM-ML behavior body: unsupported expression `call("),
            "{}",
            diagnostic.message
        );
    }

    #[derive(Debug)]
    struct MapReadResolver {
        entries: Vec<(String, &'static [u8], Option<&'static str>)>,
    }

    impl ResourceResolver for MapReadResolver {
        fn read(&self, request: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
            let uri = resolve_test_uri(request);
            let Some((resolved_uri, bytes, content_type)) = self
                .entries
                .iter()
                .find(|(entry_uri, _, _)| entry_uri == &uri)
            else {
                return Err(ResolverDiagnostic::UnsupportedResolver {
                    uri: request.uri.clone(),
                    purpose: request.purpose,
                    direction: ResolveDirection::Read,
                });
            };

            Ok(ResolvedRead {
                uri: resolved_uri.clone(),
                bytes: bytes.to_vec(),
                content_type: content_type.map(str::to_owned),
            })
        }

        fn write(
            &self,
            request: &ResolveRequest,
            _bytes: &[u8],
        ) -> Result<ResolvedWrite, ResolverDiagnostic> {
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
        let Some(base_uri) = request.base_uri.as_deref() else {
            return request.uri.clone();
        };
        let Some((base_dir, _)) = base_uri.rsplit_once('/') else {
            return request.uri.clone();
        };
        format!("{base_dir}/{}", request.uri)
    }

    fn schema_package_manifest_identity() -> FormatIdentity {
        FormatIdentity {
            content_type: Some(CEM_SCHEMA_PACKAGE_CONTENT_TYPE.to_owned()),
            schema: Some(CEM_SCHEMA_PACKAGE_URI.to_owned()),
            ..FormatIdentity::default()
        }
    }

    fn context_with_cem_ml_output_pipeline_artifacts() -> EngineContext {
        const PACKAGE_URI: &str = "cem+test://packages/cem-ml/v1/package.cem";
        const PACKAGE_MANIFEST: &[u8] = br#"@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="cem-ml" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/cem-ml-output-pipeline/1"
        @source="schema/cem-ml-output-pipeline.cem"
    }
    {content-type @value="application/vnd.example.cem-ml-output-pipeline+cem" @primary=true}
    {artifact
        @kind="formatter"
        @path="formatters/cem-format-tree.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
        @target-content-type="application/cem"
        @target-schema="https://cem.dev/ns/cem-ml/1"
        @target-category="cem-tree"
        @function-name="cem.format-tree"
        @formatter-profile="cem.format-tree"
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
        @formatter-profile="cem.format-tree"
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
        const PACKAGE_SCHEMA_URI: &str =
            "cem+test://packages/cem-ml/v1/schema/cem-ml-output-pipeline.cem";
        const PACKAGE_SCHEMA_SOURCE: &[u8] = br#"@doc cem-ml 1
@ns schema = "https://cem.dev/ns/schema/1"
@default schema

{schema
    @name="cem-ml-output-pipeline"
    @namespace="https://example.test/ns/cem-ml-output-pipeline/1"
    @version="1.0.0" |
    {elements |
        {element @name="package"}
    }
}
"#;

        let mut artifact_entries = builtin_schema_package_artifact_sources()
            .iter()
            .filter(|source| source.package_id == "cem-ml")
            .map(|source| {
                let relative = source
                    .path
                    .strip_prefix("schema-packages/cem-ml/v1/")
                    .expect("CEM-ML artifact path is package-relative");
                (
                    format!("cem+test://packages/cem-ml/v1/{relative}"),
                    source.source.as_bytes(),
                    Some(CEM_TRANSFORM_CONTENT_TYPE),
                )
            })
            .collect::<Vec<_>>();
        artifact_entries.push((
            PACKAGE_SCHEMA_URI.to_owned(),
            PACKAGE_SCHEMA_SOURCE,
            Some(cem_ml::schema::registry::CEM_SCHEMA_CONTENT_TYPE),
        ));

        let mut resolver_registry = ResolverRegistry::new();
        resolver_registry.register(
            "cem+test",
            ResolvePurpose::Template,
            ResolveDirection::Read,
            MapReadResolver {
                entries: artifact_entries,
            },
        );

        EngineContext {
            schema_package_manifests: vec![EngineInput {
                uri: PACKAGE_URI.to_owned(),
                bytes: PACKAGE_MANIFEST.to_vec(),
                from_format: Some(InputFormat::Cem),
                identity: Some(schema_package_manifest_identity()),
                root_scope: ScopeConfig::default(),
            }],
            resolver_registry,
            ..engine_context_with_cem_ql_template_adapter()
        }
    }

    fn render_packaged_dom_projection_converter(
        template_uri: &str,
        template_source: &str,
        document: &CemDocument,
        target_content_type: &str,
    ) -> String {
        let adapter = CemQlTransformTemplateAdapter;
        let template = TemplateInput {
            uri: template_uri.to_owned(),
            bytes: template_source.as_bytes().to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(cem_ml::schema::registry::CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
                schema: Some(cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let module_parse =
            parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                template: template.clone(),
            });
        assert!(
            module_parse.diagnostics.is_empty(),
            "{template_uri}: {:?}",
            module_parse.diagnostics
        );
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: module_parse.module_options,
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("packaged DOM projection converter asset should compile");
        assert!(
            compiled.diagnostics.is_empty(),
            "{template_uri}: {:?}",
            compiled.diagnostics
        );

        let primary_input = packaged_dom_projection_artifact(projection::dom_json(document));
        let secondary_inputs = BTreeMap::new();
        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled.artifact,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: Some(&FormatIdentity {
                    content_type: Some(target_content_type.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("packaged DOM projection converter asset should render");
        assert!(
            rendered.diagnostics.is_empty(),
            "{template_uri}: {:?}",
            rendered.diagnostics
        );
        rendered
            .output
            .value
            .as_str()
            .expect("converter output should be string content")
            .to_owned()
    }

    #[test]
    fn adapter_compiles_and_renders_cem_native_template() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{span @class="{$datadom.attributes.tone}" | {$datadom.attributes.label}}"#
                .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: Some("data.json".to_owned()),
            identity: None,
            value: json_object([
                ("label", Value::String("Save".to_owned())),
                ("tone", Value::String("primary".to_owned())),
            ]),
        };
        let secondary_inputs = BTreeMap::new();
        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should render");

        assert_eq!(
            rendered.output.value,
            Value::String(r#"<span class="primary">Save</span>"#.to_owned())
        );
        assert_eq!(
            rendered
                .output
                .identity
                .and_then(|identity| identity.content_type),
            Some("text/html".to_owned())
        );
    }

    #[test]
    fn xslt_parity_adapter_compiles_and_renders_lowered_template() {
        let adapter = XsltParityTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("application/xslt+xml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "view.xsl".to_owned(),
            bytes: br#"<xsl:stylesheet version="1.0"><xsl:template match="/"><main><h1>Sign in</h1></main></xsl:template></xsl:stylesheet>"#.to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::XsltParity,
                    ..TransformExecutionPolicy::default()
                },
            })
            .expect("XSLT parity template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: Some("data.cem".to_owned()),
            identity: None,
            value: json_object([("kind", Value::String("document".to_owned()))]),
        };
        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &BTreeMap::new(),
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::XsltParity,
                    ..TransformExecutionPolicy::default()
                },
            })
            .expect("XSLT parity template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<main><h1>Sign in</h1></main>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn xslt_parity_adapter_selects_named_entrypoint_with_params() {
        let adapter = XsltParityTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("application/xslt+xml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "view.xsl".to_owned(),
            bytes: br#"<xsl:stylesheet version="1.0"><xsl:template match="/"><main>default</main></xsl:template><xsl:template name="card"><article><xsl:value-of select="$title"/></article></xsl:template></xsl:stylesheet>"#.to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::from([("title".to_owned(), Value::String("Intro".to_owned()))]);
        let data_bindings = vec!["input".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::XsltParity,
                    ..TransformExecutionPolicy::default()
                },
            })
            .expect("XSLT parity template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: Some("data.cem".to_owned()),
            identity: None,
            value: json_object([("kind", Value::String("document".to_owned()))]),
        };
        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &BTreeMap::new(),
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::XsltParity,
                    ..TransformExecutionPolicy::default()
                },
            })
            .expect("XSLT parity template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<article>Intro</article>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn xslt_parity_adapter_reports_missing_named_entrypoint_as_fatal() {
        let adapter = XsltParityTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("application/xslt+xml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "view.xsl".to_owned(),
            bytes: br#"<xsl:stylesheet version="1.0"><xsl:template match="/"><main>default</main></xsl:template></xsl:stylesheet>"#.to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("missing"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::XsltParity,
                    ..TransformExecutionPolicy::default()
                },
            })
            .expect("XSLT parity template compile should return diagnostics");

        assert!(compiled.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE
                && diagnostic.severity == Severity::Fatal
                && diagnostic
                    .message
                    .contains("XSLT template entrypoint `missing` was not found")
        }));
    }

    #[test]
    fn xslt_parity_adapter_reports_unsupported_constructs_as_fatal() {
        let adapter = XsltParityTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("application/xslt+xml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "view.xsl".to_owned(),
            bytes: br#"<xsl:stylesheet version="1.0"><xsl:template match="/"><msxsl:script language="JScript">function run(){return 1;}</msxsl:script></xsl:template></xsl:stylesheet>"#.to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::XsltParity,
                    ..TransformExecutionPolicy::default()
                },
            })
            .expect("XSLT parity template compile should return diagnostics");

        assert!(compiled.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == UNSUPPORTED_CONSTRUCT_CODE && diagnostic.severity == Severity::Fatal
        }));
    }

    #[test]
    fn adapter_surfaces_compile_diagnostics_once() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{span | {$missing}}"#.to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile with diagnostics");
        assert_eq!(compiled.artifact.opaque["diagnostics"], 1);
        assert!(compiled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"));
        assert!(compiled.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.ql.render.compile_failed"
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled.artifact,
                primary_input: &TransformTemplateDataArtifact {
                    artifact_id: "data".to_owned(),
                    uri: None,
                    identity: None,
                    value: Value::Null,
                },
                secondary_inputs: &BTreeMap::new(),
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should render without repeating compile diagnostics");

        assert!(!rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"));
    }

    #[test]
    fn adapter_suppresses_cemt_encode_metadata_compile_diagnostics() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "color.cemt".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="main" @visibility="public" |
    {body |
      {$ encode([{"role": "diagnostic.error", "text": "Broken"}], { contentType: "text/plain", schema: "https://cem.dev/ns/data/text/terminal/1", category: "terminal-color" }, { colorizer: "terminal.ansi256", colorProfile: "ansi-256" }) }
    }
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned()];

        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("main"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile");

        assert!(
            !compiled
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "cem.ql.render.compile_failed"),
            "{:?}",
            compiled.diagnostics
        );
    }

    #[test]
    fn adapter_attributes_imported_module_compile_diagnostics_to_module_uri() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="card"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" | {body | {span | {$missing}}}}
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile with imported module diagnostics");

        assert_eq!(compiled.artifact.opaque["moduleDiagnostics"], 1);
        assert!(compiled.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.ql.render.compile_failed"
                && diagnostic.uri.as_deref() == Some("templates/ui.cem")
        }));
        assert!(!compiled.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cem.ql.render.compile_failed"
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_binds_primary_input_and_secondary_labels() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{span | {$input.label}:{$meta.count}}"#.to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["input".to_owned(), "meta".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: Some("data.json".to_owned()),
            identity: None,
            value: json_object([("label", Value::String("orders".to_owned()))]),
        };
        let secondary_inputs = BTreeMap::from([(
            "meta".to_owned(),
            TransformTemplateDataArtifact {
                artifact_id: "meta".to_owned(),
                uri: Some("meta.json".to_owned()),
                identity: None,
                value: json_object([("count", Value::Number(3.into()))]),
            },
        )]);

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<span>orders:3</span>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_compiles_preflighted_modules_into_payload() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{article | Main}"#.to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:test".to_owned(),
                        bytes: br#"{span | Imported}"#.to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("template and import should compile")
            .artifact;

        assert_eq!(compiled.opaque["moduleImports"], 1);
        assert_eq!(compiled.opaque["modules"][0]["alias"], "ui");
        assert_eq!(
            compiled.opaque["modules"][0]["contentHash"],
            "cem-bin/1+blake3:test"
        );
        let payload = compiled
            .native_payload::<CemQlCompiledTemplatePayload>()
            .expect("CEM-QL payload");
        assert_eq!(payload.modules.len(), 1);
        assert_eq!(payload.modules[0].alias, "ui");
        assert!(!payload.modules[0].artifact.nodes.is_empty());
    }

    #[test]
    fn adapter_dispatches_same_module_calls_during_render() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" | {body | {span | Help}}}
  {body | {div | {call @template="helper"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Help</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_passes_with_bindings_to_same_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="title"}
    {body | {span | {$title}}}
  }
  {body | {div | {call @template="helper" @with:title="Hello"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.title".to_owned(),
                        value_type: TransformTemplateModuleParamType::Any,
                        nullable: false,
                        default_value: None,
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Hello</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_reports_missing_required_params_for_same_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="title" @required="true"}
    {body | {span | {$title}}}
  }
  {body | {div | {call @template="helper"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.title".to_owned(),
                        value_type: TransformTemplateModuleParamType::Any,
                        nullable: false,
                        default_value: None,
                        required: true,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with required-param diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div></div>".to_owned())
        );
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_preserves_typed_with_bindings_for_same_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="enabled"}
    {body | {cem:if @test="enabled" | {span | Enabled}}{span | Done}}
  }
  {body | {div | {call @template="helper" @with:enabled="{enabled}"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["enabled".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.enabled".to_owned(),
                        value_type: TransformTemplateModuleParamType::Any,
                        nullable: false,
                        default_value: None,
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: json_object([("enabled", Value::Bool(false))]),
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Done</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_reports_type_mismatches_for_same_module_call_params() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="enabled" @type="boolean"}
    {body | {cem:if @test="enabled" | {span | Enabled}}}
  }
  {body | {div | {call @template="helper" @with:enabled="not-bool"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.enabled".to_owned(),
                        value_type: TransformTemplateModuleParamType::Boolean,
                        nullable: false,
                        default_value: None,
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with type diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div></div>".to_owned())
        );
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_TYPE_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_treats_nullable_null_with_bindings_as_provided() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="title" @type="string" @nullable="true" @required="true"}
    {body | {span | A{$title}B}}
  }
  {body | {div | {call @template="helper" @with:title="{title}"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["title".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.title".to_owned(),
                        value_type: TransformTemplateModuleParamType::String,
                        nullable: true,
                        default_value: None,
                        required: true,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: json_object([("title", Value::Null)]),
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>AB</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_treats_empty_with_binding_streams_as_missing_required_params() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="title" @required="true"}
    {body | {span | {$title}}}
  }
  {body | {div | {call @template="helper" @with:title="{missing}"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["missing".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.title".to_owned(),
                        value_type: TransformTemplateModuleParamType::Any,
                        nullable: false,
                        default_value: None,
                        required: true,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with required-param diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div></div>".to_owned())
        );
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_preserves_structured_with_bindings_for_same_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" |
    {param @name="settings"}
    {body | {cem:if @test="settings.enabled" | {span | Enabled}}}
  }
  {body | {div | {call @template="helper" @with:settings="{sourceSettings}"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["sourceSettings".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "helper.settings".to_owned(),
                        value_type: TransformTemplateModuleParamType::Any,
                        nullable: false,
                        default_value: None,
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: json_object([(
                "sourceSettings",
                json_object([("enabled", Value::Bool(true))]),
            )]),
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Enabled</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_applies_named_entrypoint_param_defaults() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "locale".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("en-US".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.title".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("Untitled".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>en-US:Untitled</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_applies_typed_param_defaults() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="enabled" @type="boolean" @default="true"}
    {param @name="count" @type="integer" @default="3"}
    {body | {p | {cem:if @test="enabled" | Enabled:}{$count}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.enabled".to_owned(),
                            value_type: TransformTemplateModuleParamType::Boolean,
                            nullable: false,
                            default_value: Some(Value::Bool(true)),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.count".to_owned(),
                            value_type: TransformTemplateModuleParamType::Integer,
                            nullable: false,
                            default_value: Some(Value::Number(Number::from(3))),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>Enabled:3</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_applies_nullable_typed_param_defaults() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="card" @visibility="public" |
    {param @name="subtitle" @type="string" @nullable="true" @default="null"}
    {body | {p | A{$subtitle}B}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                        name: "card.subtitle".to_owned(),
                        value_type: TransformTemplateModuleParamType::String,
                        nullable: true,
                        default_value: Some(Value::Null),
                        required: false,
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                    }],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(rendered.output.value, Value::String("<p>AB</p>".to_owned()));
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_binds_caller_params_for_named_entrypoints() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::from([
            ("locale".to_owned(), Value::String("fr-FR".to_owned())),
            ("title".to_owned(), Value::String("Intro".to_owned())),
        ]);
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "locale".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("en-US".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.title".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("Untitled".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>fr-FR:Intro</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_binds_qualified_caller_params_for_named_entrypoints() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::from([
            ("locale".to_owned(), Value::String("fr-FR".to_owned())),
            ("card.title".to_owned(), Value::String("Intro".to_owned())),
        ]);
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "locale".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("en-US".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.title".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("Untitled".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>fr-FR:Intro</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_treats_explicit_null_params_as_default_overrides() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::from([("title".to_owned(), Value::Null)]);
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::named("card"),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    entrypoints: vec![cem_ml::transform_template::TransformTemplateModuleEntrypointDeclaration {
                        name: "card".to_owned(),
                        visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Public,
                    }],
                    params: vec![
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "locale".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: false,
                            default_value: Some(Value::String("en-US".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                        cem_ml::transform_template::TransformTemplateModuleParamDeclaration {
                            name: "card.title".to_owned(),
                            value_type: TransformTemplateModuleParamType::Any,
                            nullable: true,
                            default_value: Some(Value::String("Untitled".to_owned())),
                            required: false,
                            visibility: cem_ml::transform_template::TransformTemplateModuleVisibility::Private,
                        },
                    ],
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<p>en-US:</p>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_stops_recursive_same_module_calls_at_limit() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="loop" | {body | {span | Loop {call @template="loop"}}}}
  {body | {div | {call @template="loop"}}}
}"#
            .to_vec(),
            identity: Some(identity),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    limits: cem_ml::transform_template::TransformTemplateModuleLimits {
                        max_import_depth: 32,
                        max_recursion_depth: 2,
                    },
                    ..Default::default()
                },
                module_preflight: Default::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with recursion diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Loop <span>Loop </span></span></div>".to_owned())
        );
        assert!(rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE));
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_dispatches_imported_module_calls_during_render() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" | {body | {span | Icon}}}
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Icon</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_dispatches_same_module_calls_inside_imported_modules() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="helper" | {body | {i | Root}}}
  {body | {div | {call @from="ui" @template="icon"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {body | {span | Icon {call @template="helper"}}}
  }
  {template @name="helper" | {body | {i | Imported}}}
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Icon <i>Imported</i></span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_dispatches_nested_import_calls_inside_imported_modules() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="card"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![
                        TransformTemplateResolvedModule {
                            alias: "ui".to_owned(),
                            parent_uri: None,
                            requested_uri: None,
                            normalized_uri: None,
                            substituted_uri: None,
                            resolver_policy_stamp: None,
                            uri: "templates/ui.cem".to_owned(),
                            identity: Some(identity.clone()),
                            content_hash: "cem-bin/1+blake3:ui".to_owned(),
                            bytes: br#"{@doc cem-ml 1}
{module |
  {import @as="icons" @src="icons.cem"}
  {template @name="card" @visibility="public" |
    {body | {section | Card {call @from="icons" @template="check"}}}
  }
}"#
                            .to_vec(),
                        },
                        TransformTemplateResolvedModule {
                            alias: "icons".to_owned(),
                            parent_uri: Some("templates/ui.cem".to_owned()),
                            requested_uri: None,
                            normalized_uri: None,
                            substituted_uri: None,
                            resolver_policy_stamp: None,
                            uri: "templates/icons.cem".to_owned(),
                            identity: Some(identity),
                            content_hash: "cem-bin/1+blake3:icons".to_owned(),
                            bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="check" @visibility="public" | {body | {span | Check}}}
}"#
                            .to_vec(),
                        },
                    ],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><section>Card <span>Check</span></section></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_bounds_recursive_calls_inside_imported_modules() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="loop"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: cem_ml::transform_template::TransformTemplateModuleOptions {
                    limits: cem_ml::transform_template::TransformTemplateModuleLimits {
                        max_import_depth: 32,
                        max_recursion_depth: 2,
                    },
                    ..Default::default()
                },
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="loop" @visibility="public" |
    {body | {span | Loop {call @template="loop"}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with recursion diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Loop <span>Loop </span></span></div>".to_owned())
        );
        assert!(rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE));
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE
                && diagnostic.uri.as_deref() == Some("templates/ui.cem")
        }));
        assert!(!rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
    }

    #[test]
    fn adapter_passes_with_bindings_to_imported_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon" @with:title="Imported"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {param @name="title"}
    {body | {span | {$title}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Imported</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_applies_imported_module_call_param_defaults() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="tone" @default="primary"}
  {template @name="icon" @visibility="public" |
    {param @name="title" @default="Imported"}
    {body | {span @class="{$tone}" | {$title}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span class=\"primary\">Imported</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_treats_imported_nullable_null_with_bindings_as_provided() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon" @with:title="{title}"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["title".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {param @name="title" @type="string" @nullable="true" @required="true"}
    {body | {span | A{$title}B}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: json_object([("title", Value::Null)]),
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>AB</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_treats_imported_empty_with_binding_streams_as_missing_required_params() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon" @with:title="{missing}"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["missing".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {span | {$title}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with required-param diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div></div>".to_owned())
        );
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
        assert!(!rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE
                && diagnostic.uri.as_deref() == Some("templates/ui.cem")
        }));
    }

    #[test]
    fn adapter_preserves_structured_with_bindings_for_imported_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon" @with:settings="{sourceSettings}"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["sourceSettings".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {param @name="settings"}
    {body | {cem:if @test="settings.enabled" | {span | Enabled}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: json_object([(
                "sourceSettings",
                json_object([("enabled", Value::Bool(true))]),
            )]),
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>Enabled</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_preserves_typed_with_bindings_for_imported_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon" @with:count="{sourceCount}"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = vec!["sourceCount".to_owned()];
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {param @name="count" @type="integer"}
    {body | {span | {$count}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: json_object([("sourceCount", Value::Number(7.into()))]),
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render");

        assert_eq!(
            rendered.output.value,
            Value::String("<div><span>7</span></div>".to_owned())
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn adapter_reports_missing_required_params_for_imported_module_calls() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {param @name="title" @required="true"}
    {body | {span | {$title}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with required-param diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div></div>".to_owned())
        );
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
        assert!(!rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE
                && diagnostic.uri.as_deref() == Some("templates/ui.cem")
        }));
    }

    #[test]
    fn adapter_reports_type_mismatches_for_imported_module_call_params() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{@doc cem-ml 1}
{module |
  {body | {div | {call @from="ui" @template="icon" @with:count="many"}}}
}"#
            .to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();
        let compiled = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:ui".to_owned(),
                        bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="icon" @visibility="public" |
    {param @name="count" @type="integer"}
    {body | {span | {$count}}}
  }
}"#
                        .to_vec(),
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should compile")
            .artifact;
        let primary_input = TransformTemplateDataArtifact {
            artifact_id: "data".to_owned(),
            uri: None,
            identity: None,
            value: Value::Null,
        };
        let secondary_inputs = BTreeMap::new();

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary_input,
                secondary_inputs: &secondary_inputs,
                target: None,
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect("module template should render with type diagnostic");

        assert_eq!(
            rendered.output.value,
            Value::String("<div></div>".to_owned())
        );
        assert!(rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_TYPE_CODE
                && diagnostic.uri.as_deref() == Some("template.cem")
        }));
        assert!(!rendered.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_PARAM_TYPE_CODE
                && diagnostic.uri.as_deref() == Some("templates/ui.cem")
        }));
    }

    #[test]
    fn adapter_rejects_non_utf8_preflighted_modules() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template = TemplateInput {
            uri: "template.cem".to_owned(),
            bytes: br#"{article | Main}"#.to_vec(),
            identity: Some(identity.clone()),
            root_scope: ScopeConfig::default(),
        };
        let params = BTreeMap::new();
        let data_bindings = Vec::new();

        let error = adapter
            .compile(TransformTemplateCompileRequest {
                template: &template,
                entrypoint: &TransformTemplateEntrypoint::implicit(),
                params: &params,
                data_bindings: &data_bindings,
                module_options: Default::default(),
                module_preflight: TransformTemplateModulePreflight {
                    resolved_imports: vec![TransformTemplateResolvedModule {
                        alias: "ui".to_owned(),
                        parent_uri: None,
                        requested_uri: None,
                        normalized_uri: None,
                        substituted_uri: None,
                        resolver_policy_stamp: None,
                        uri: "templates/ui.cem".to_owned(),
                        identity: Some(identity),
                        content_hash: "cem-bin/1+blake3:test".to_owned(),
                        bytes: vec![0xff],
                    }],
                    cache_key: None,
                },
                execution_policy: TransformExecutionPolicy::default(),
            })
            .expect_err("invalid module bytes should fail compile");

        assert_eq!(
            error.diagnostic_origin(),
            cem_ml::engine::TransformDiagnosticOrigin::TemplateCompile
        );
        assert!(error.to_string().contains("templates/ui.cem"));
    }

    #[test]
    fn registration_makes_adapter_available_through_engine_context() {
        let context = engine_context_with_cem_ql_template_adapter();
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };

        match context.template_adapter_registry.select_adapter(&identity) {
            TransformTemplateAdapterLookup::Matched(adapter) => {
                assert_eq!(adapter.id(), CEM_QL_TEMPLATE_ADAPTER_ID)
            }
            other => panic!("expected built-in plus runtime adapter ambiguity, got {other:?}"),
        }
    }

    #[test]
    fn adapter_accepts_cem_native_template_schema_identity() {
        let context = engine_context_with_cem_ql_template_adapter();
        let identity = FormatIdentity {
            schema: Some(cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned()),
            default_namespace: Some(
                cem_ml::transform_template::CEM_NATIVE_TEMPLATE_NAMESPACE_URI.to_owned(),
            ),
            ..FormatIdentity::default()
        };

        match context.template_adapter_registry.select_adapter(&identity) {
            TransformTemplateAdapterLookup::Matched(adapter) => {
                assert_eq!(adapter.id(), CEM_QL_TEMPLATE_ADAPTER_ID)
            }
            other => panic!("expected schema identity adapter match, got {other:?}"),
        }
    }

    #[test]
    fn adapter_accepts_cem_vendor_template_content_types() {
        let context = engine_context_with_cem_ql_template_adapter();
        for content_type in [
            cem_ml::schema::registry::CEM_NATIVE_TEMPLATE_CONTENT_TYPE,
            cem_ml::schema::registry::CEM_TRANSFORM_CONTENT_TYPE,
        ] {
            let identity = FormatIdentity {
                content_type: Some(format!("{content_type}; charset=utf-8")),
                ..FormatIdentity::default()
            };

            match context.template_adapter_registry.select_adapter(&identity) {
                TransformTemplateAdapterLookup::Matched(adapter) => {
                    assert_eq!(adapter.id(), CEM_QL_TEMPLATE_ADAPTER_ID)
                }
                other => {
                    panic!("expected content type `{content_type}` adapter match, got {other:?}")
                }
            }
        }
    }

    #[test]
    fn packaged_dom_projection_converter_assets_compile_with_cem_ql_adapter() {
        let adapter = CemQlTransformTemplateAdapter;
        let identity = FormatIdentity {
            content_type: Some(cem_ml::schema::registry::CEM_TRANSFORM_CONTENT_TYPE.to_owned()),
            schema: Some(cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        for (uri, source) in [
            (
                "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
                include_str!(
                    "../../cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
                ),
            ),
            (
                "schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt",
                include_str!(
                    "../../cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt"
                ),
            ),
        ] {
            let template = TemplateInput {
                uri: uri.to_owned(),
                bytes: source.as_bytes().to_vec(),
                identity: Some(identity.clone()),
                root_scope: ScopeConfig::default(),
            };
            let module_parse =
                parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
                    template: template.clone(),
                });
            assert!(
                module_parse.diagnostics.is_empty(),
                "{uri}: {:?}",
                module_parse.diagnostics
            );
            let params = BTreeMap::new();
            let data_bindings = vec!["input".to_owned()];

            let compiled = adapter
                .compile(TransformTemplateCompileRequest {
                    template: &template,
                    entrypoint: &TransformTemplateEntrypoint::named("main"),
                    params: &params,
                    data_bindings: &data_bindings,
                    module_options: module_parse.module_options,
                    module_preflight: Default::default(),
                    execution_policy: TransformExecutionPolicy::default(),
                })
                .expect("packaged DOM projection converter asset should compile");

            assert!(
                compiled.diagnostics.is_empty(),
                "{uri}: {:?}",
                compiled.diagnostics
            );
            let primary_input =
                packaged_dom_projection_input(if uri.ends_with("dom-to-xml.cemt") {
                    vec![
                        json_object([
                            ("kind", Value::String("processing-instruction".to_owned())),
                            ("name", Value::String("xml-stylesheet".to_owned())),
                            ("target", Value::String("xml-stylesheet".to_owned())),
                            ("data", Value::String("href=\"main.css\"".to_owned())),
                        ]),
                        json_object([
                            ("kind", Value::String("element".to_owned())),
                            ("name", Value::String("p".to_owned())),
                            ("namespace", Value::String(String::new())),
                            (
                                "attributes",
                                Value::Array(vec![json_object([
                                    ("name", Value::String("class".to_owned())),
                                    ("namespace", Value::String(String::new())),
                                    ("value", Value::String("lead".to_owned())),
                                ])]),
                            ),
                            (
                                "children",
                                Value::Array(vec![json_object([
                                    ("kind", Value::String("cdata".to_owned())),
                                    ("data", Value::String("Hi <all>".to_owned())),
                                ])]),
                            ),
                        ]),
                    ]
                } else {
                    vec![json_object([
                        ("kind", Value::String("element".to_owned())),
                        ("name", Value::String("p".to_owned())),
                        ("namespace", Value::String(String::new())),
                        (
                            "attributes",
                            Value::Array(vec![json_object([
                                ("name", Value::String("class".to_owned())),
                                ("namespace", Value::String(String::new())),
                                ("value", Value::String("lead".to_owned())),
                            ])]),
                        ),
                        (
                            "children",
                            Value::Array(vec![json_object([
                                ("kind", Value::String("text".to_owned())),
                                ("data", Value::String("Hi".to_owned())),
                            ])]),
                        ),
                    ])]
                });
            let secondary_inputs = BTreeMap::new();
            let target_content_type = if uri.ends_with("dom-to-xml.cemt") {
                "application/xml"
            } else {
                "text/html"
            };
            let rendered = adapter
                .render(TransformTemplateRenderRequest {
                    compiled: &compiled.artifact,
                    primary_input: &primary_input,
                    secondary_inputs: &secondary_inputs,
                    target: Some(&FormatIdentity {
                        content_type: Some(target_content_type.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    target_scope: &ScopeConfig::default(),
                    execution_policy: TransformExecutionPolicy::default(),
                })
                .expect("packaged DOM projection converter asset should render");
            assert!(
                rendered.diagnostics.is_empty(),
                "{uri}: {:?}",
                rendered.diagnostics
            );

            if uri.ends_with("dom-to-xml.cemt") {
                assert_eq!(
                    rendered.output.value,
                    Value::String(
                        r#"<?xml-stylesheet href="main.css"?><p class="lead"><![CDATA[Hi <all>]]></p>"#
                            .to_owned()
                    )
                );
            } else {
                assert_eq!(
                    rendered.output.value,
                    Value::String(r#"<p class="lead">Hi</p>"#.to_owned())
                );
            }
        }
    }

    #[test]
    fn packaged_dom_projection_converters_match_rust_serializer_parity() {
        let html_document = document_from_cem(include_str!("../../../examples/cem-ml/login.cem"));
        let html_rust = LightDomInterpreter::new().render(&html_document);
        assert!(
            html_rust.diagnostics.is_empty(),
            "{:?}",
            html_rust.diagnostics
        );
        let html_cemt = render_packaged_dom_projection_converter(
            "schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            include_str!(
                "../../cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
            ),
            &html_document,
            "text/html",
        );
        assert_eq!(html_cemt, html_rust.rendered);

        let xml_document = document_from_xml(include_str!(
            "../../../examples/cem-ml/cross-surface/conversion-rules.xml"
        ));
        let xml_rust = XmlInterpreter::new().render(&xml_document);
        assert!(
            xml_rust.diagnostics.is_empty(),
            "{:?}",
            xml_rust.diagnostics
        );
        let xml_cemt = render_packaged_dom_projection_converter(
            "schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt",
            include_str!(
                "../../cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-xml.cemt"
            ),
            &xml_document,
            "application/xml",
        );
        assert_eq!(xml_cemt, xml_rust.rendered);
    }

    #[test]
    fn real_engine_convert_uses_ready_packaged_dom_projection_cemt_converters() {
        for (uri, source, input_format, target_format, expected_kind, expected_content) in [
            {
                let source = include_str!("../../../examples/cem-ml/login.cem");
                (
                    "login.cem",
                    source,
                    InputFormat::Cem,
                    LayerFormat::Html,
                    "html",
                    r#"<main aria-labelledby="login-title" cem:screen="login" class="cem-color cem-color-syntax-name" data-role="syntax.name"><h1 id="login-title" class="cem-color cem-color-syntax-name" data-role="syntax.name"><span class="cem-color cem-color-syntax-string" data-role="syntax.string">Sign in</span></h1><form action="/session" method="post" cem:form="sign-in" class="cem-color cem-color-syntax-name" data-role="syntax.name"><label for="email" class="cem-color cem-color-syntax-name" data-role="syntax.name"><span class="cem-color cem-color-syntax-string" data-role="syntax.string">Email</span></label><input autocomplete="email" id="email" name="email" required type="email" class="cem-color cem-color-syntax-name" data-role="syntax.name"><label for="password" class="cem-color cem-color-syntax-name" data-role="syntax.name"><span class="cem-color cem-color-syntax-string" data-role="syntax.string">Password</span></label><input autocomplete="current-password" id="password" name="password" required type="password" class="cem-color cem-color-syntax-name" data-role="syntax.name"><button type="submit" cem:action="primary" class="cem-color cem-color-syntax-name" data-role="syntax.name"><span class="cem-color cem-color-syntax-string" data-role="syntax.string">Sign in</span></button></form></main>"#.to_owned(),
                )
            },
            {
                let source =
                    include_str!("../../../examples/cem-ml/cross-surface/conversion-rules.xml");
                (
                    "conversion-rules.xml",
                    source,
                    InputFormat::Xml,
                    LayerFormat::Xml,
                    "xml",
                    "<?xml version=\"1.0\"?><!DOCTYPE main><main aria-labelledby=\"title\" xmlns=\"http://www.w3.org/1999/xhtml\" cem:screen=\"conversion\" xmlns:cem=\"https://cem.dev/ns/core/1\"><!-- conversion fixture --><h1 id=\"title\">Conversion fixture</h1><svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M2 8h12\"/></svg><$>.items[0].name</$><button aria-label=\"Hello {.name}\" disabled=\"{.busy}\">Save</button></main>".to_owned(),
                )
            },
        ] {
            let response = RealCemMlEngine::new()
                .convert(ConvertRequest {
                    input: EngineInput {
                        uri: uri.to_owned(),
                        bytes: source.as_bytes().to_vec(),
                        from_format: Some(input_format),
                        identity: None,
                        root_scope: ScopeConfig::default(),
                    },
                    to_format: target_format,
                    preserve_source_offsets: false,
                    context: context_with_cem_ml_output_pipeline_artifacts(),
                    target: None,
                    target_scope: ScopeConfig::default(),
                    scheduler_scope_id: 0,
                })
                .expect("convert request should execute");

            assert_eq!(
                response.primary["kind"], expected_kind,
                "{uri}: {:?}",
                response.diagnostics
            );
            assert_eq!(response.primary["content"], expected_content, "{uri}");
            assert!(
                response
                    .diagnostics
                    .iter()
                    .all(|diagnostic| !diagnostic.severity.is_hard_violation()),
                "{uri}: {:?}",
                response.diagnostics
            );
            assert!(
                !response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "cem.converter.output_pipeline_execution"),
                "{uri}: {:?}",
                response.diagnostics
            );
            assert!(
                !response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "cem.converter.cemt_fallback"),
                "{uri}: {:?}",
                response.diagnostics
            );
        }
    }

    #[test]
    fn real_engine_convert_uses_registered_cem_ql_source_output_handler_for_native_text() {
        let source = r#"module "https://example.test/queries/direct"

declare let greeting = "Hello"

greeting
"#;
        let response = RealCemMlEngine::new()
            .convert(ConvertRequest {
                input: EngineInput {
                    uri: "direct.cemql".to_owned(),
                    bytes: source.as_bytes().to_vec(),
                    from_format: None,
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
                to_format: LayerFormat::Cem,
                preserve_source_offsets: false,
                context: engine_context_with_cem_ql_template_adapter(),
                target: Some(FormatIdentity {
                    content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig {
                    cemt_formatter: Some("cem-ql.format-tree".to_owned()),
                    cemt_formatter_profile: Some("tabular".to_owned()),
                    cemt_colorizer: Some("cem-ql.color-tree".to_owned()),
                    cemt_color_profile: Some("none".to_owned()),
                    ..ScopeConfig::default()
                },
                scheduler_scope_id: 0,
            })
            .expect("direct CEM-QL convert should use registered handler");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert!(
            !response
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "cem.lifecycle.adapter_unsupported"),
            "{:?}",
            response.diagnostics
        );
        let primary = response.primary_bytes.expect("primary bytes");
        assert_eq!(primary.content_type, CEM_QL_CONTENT_TYPE);
        assert_eq!(primary.schema.as_deref(), Some(CEM_QL_SCHEMA_URI));
        assert_eq!(
            String::from_utf8(primary.bytes).expect("CEM-QL output is UTF-8"),
            source
        );
        let stages = response
            .conversion
            .and_then(|metadata| metadata.output_pipeline)
            .expect("output pipeline metadata")
            .stages;
        assert_eq!(stages[0].function.as_deref(), Some("cem-ql.format-tree"));
        assert_eq!(stages[0].profile.as_deref(), Some("tabular"));
        assert_eq!(stages[1].function.as_deref(), Some("cem-ql.color-tree"));
        assert_eq!(stages[1].profile.as_deref(), Some("none"));
    }

    #[test]
    fn real_engine_convert_uses_registered_cem_ql_source_output_handler_for_alias_line_endings_and_comments(
    ) {
        let source = "module \"https://example.test/queries/direct-alias\"\n\n// retained comment\ndeclare let greeting = \"Hello\"\n\n/* retained block */\ngreeting\n";
        let response = RealCemMlEngine::new()
            .convert(ConvertRequest {
                input: EngineInput {
                    uri: "direct-alias.cemql".to_owned(),
                    bytes: source.as_bytes().to_vec(),
                    from_format: None,
                    identity: Some(FormatIdentity {
                        content_type: Some("text/cem-ql".to_owned()),
                        schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
                to_format: LayerFormat::Cem,
                preserve_source_offsets: false,
                context: engine_context_with_cem_ql_template_adapter(),
                target: Some(FormatIdentity {
                    content_type: Some("text/cem-ql".to_owned()),
                    schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig {
                    cemt_formatter: Some("cem-ql.format-tree".to_owned()),
                    cemt_formatter_profile: Some("compact".to_owned()),
                    cemt_colorizer: Some("cem-ql.color-tree".to_owned()),
                    cemt_color_profile: Some("none".to_owned()),
                    cemt_formatter_options: BTreeMap::from([(
                        "lineEnding".to_owned(),
                        "crlf".to_owned(),
                    )]),
                    ..ScopeConfig::default()
                },
                scheduler_scope_id: 0,
            })
            .expect("alias CEM-QL convert should use registered handler");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let primary = response.primary_bytes.expect("primary bytes");
        assert_eq!(primary.content_type, "text/cem-ql");
        assert_eq!(primary.schema.as_deref(), Some(CEM_QL_SCHEMA_URI));
        let output = String::from_utf8(primary.bytes).expect("CEM-QL output is UTF-8");
        assert_eq!(output, source.replace('\n', "\r\n"));
        assert!(output.contains("// retained comment"));
        assert!(output.contains("/* retained block */"));
    }

    #[test]
    fn real_engine_convert_uses_registered_cem_ql_source_output_handler_for_lf_line_ending_output()
    {
        let source = "module \"https://example.test/queries/direct-crlf\"\r\n\r\ndeclare let label = \"crlf\"\r\n\r\nlabel\r\n";
        let response = RealCemMlEngine::new()
            .convert(ConvertRequest {
                input: EngineInput {
                    uri: "direct-crlf.cemql".to_owned(),
                    bytes: source.as_bytes().to_vec(),
                    from_format: None,
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
                to_format: LayerFormat::Cem,
                preserve_source_offsets: false,
                context: engine_context_with_cem_ql_template_adapter(),
                target: Some(FormatIdentity {
                    content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig {
                    cemt_formatter: Some("cem-ql.format-tree".to_owned()),
                    cemt_formatter_profile: Some("compact".to_owned()),
                    cemt_colorizer: Some("cem-ql.color-tree".to_owned()),
                    cemt_color_profile: Some("none".to_owned()),
                    cemt_formatter_options: BTreeMap::from([(
                        "lineEnding".to_owned(),
                        "lf".to_owned(),
                    )]),
                    ..ScopeConfig::default()
                },
                scheduler_scope_id: 0,
            })
            .expect("CRLF CEM-QL convert should use registered handler");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let primary = response.primary_bytes.expect("primary bytes");
        assert_eq!(primary.content_type, CEM_QL_CONTENT_TYPE);
        let output = String::from_utf8(primary.bytes).expect("CEM-QL output is UTF-8");
        assert_eq!(output, source.replace("\r\n", "\n"));
        assert!(!output.contains("\r\n"));
    }

    #[test]
    fn real_engine_convert_uses_registered_cem_ql_source_output_handler_for_invalid_utf8() {
        let mut bytes = b"module \"https://example.test/queries/invalid-utf8\"\n\n".to_vec();
        bytes.extend_from_slice(b"declare let label = \"bad: ");
        bytes.push(0xff);
        bytes.extend_from_slice(b"\"\n\nlabel\n");

        let response = RealCemMlEngine::new()
            .convert(ConvertRequest {
                input: EngineInput {
                    uri: "invalid-utf8.cemql".to_owned(),
                    bytes,
                    from_format: None,
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
                to_format: LayerFormat::Cem,
                preserve_source_offsets: false,
                context: engine_context_with_cem_ql_template_adapter(),
                target: Some(FormatIdentity {
                    content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig {
                    cemt_formatter: Some("cem-ql.format-tree".to_owned()),
                    cemt_colorizer: Some("cem-ql.color-tree".to_owned()),
                    ..ScopeConfig::default()
                },
                scheduler_scope_id: 0,
            })
            .expect("invalid UTF-8 CEM-QL convert should return diagnostics");

        assert!(response.primary_bytes.is_none());
        assert!(response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.invalid_utf8"));
        assert!(!response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn real_engine_convert_uses_registered_cem_ql_source_output_handler_for_duplicate_shape_diagnostics(
    ) {
        let source = r#"module "https://example.test/queries/duplicate-shape"

import "https://example.test/modules/a" as ui
import "https://example.test/modules/b" as ui

declare let value = "first"
declare function value() { "second" }

value
"#;
        let response = RealCemMlEngine::new()
            .convert(ConvertRequest {
                input: EngineInput {
                    uri: "duplicate-shape.cemql".to_owned(),
                    bytes: source.as_bytes().to_vec(),
                    from_format: None,
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
                to_format: LayerFormat::Cem,
                preserve_source_offsets: false,
                context: engine_context_with_cem_ql_template_adapter(),
                target: Some(FormatIdentity {
                    content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig {
                    cemt_formatter: Some("cem-ql.format-tree".to_owned()),
                    cemt_colorizer: Some("cem-ql.color-tree".to_owned()),
                    ..ScopeConfig::default()
                },
                scheduler_scope_id: 0,
            })
            .expect("duplicate CEM-QL shape convert should return diagnostics");

        assert!(response.primary_bytes.is_none());
        assert!(response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.import_alias_duplicate"));
        assert!(response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.declaration_duplicate"));
        assert!(!response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.lifecycle.adapter_unsupported"));
    }

    #[test]
    fn real_engine_convert_uses_registered_cem_ql_source_output_handler_preserves_token_ranges() {
        let source = "module \"https://example.test/queries/source-token-ranges\"\n\n// comment\ndeclare let label = \"héllo\"\n\nlabel\n";
        let tree = cem_ql_source_token_tree("source-token-ranges.cemql", source);
        let tokens = tree["tokens"].as_array().expect("tokens array");
        assert!(tokens.iter().any(|token| {
            token["tokenKind"] == "LineComment" && token["role"] == "syntax.comment"
        }));

        let string_token = tokens
            .iter()
            .find(|token| token["lexeme"] == "\"héllo\"")
            .expect("string token with non-ASCII payload");
        let expected_offset = source.find("\"héllo\"").expect("string lexeme offset") as u64;
        let expected_len = "\"héllo\"".len() as u64;
        assert_eq!(
            string_token["value"]["byteOffset"].as_u64(),
            Some(expected_offset)
        );
        assert_eq!(
            string_token["value"]["byteLength"].as_u64(),
            Some(expected_len)
        );
        assert_eq!(
            string_token["sourceMap"]["frames"][0]["span"]["ranges"]["start"].as_u64(),
            Some(expected_offset)
        );
        assert_eq!(
            string_token["sourceMap"]["frames"][0]["span"]["ranges"]["len"].as_u64(),
            Some(expected_len)
        );
        assert_eq!(
            string_token["outputSpan"]["origin"]["frames"][0]["span"]["ranges"]["start"].as_u64(),
            Some(expected_offset)
        );
        assert_eq!(
            string_token["outputSpan"]["origin"]["frames"][0]["span"]["ranges"]["len"].as_u64(),
            Some(expected_len)
        );
    }

    #[test]
    fn real_engine_convert_uses_registered_cem_ql_source_output_handler_for_html() {
        let source = r#"module "https://example.test/queries/direct-html"

declare let greeting = "Hello"

if greeting == "Hello" {
  greeting
} else {
  "fallback"
}
"#;
        let response = RealCemMlEngine::new()
            .convert(ConvertRequest {
                input: EngineInput {
                    uri: "direct-html.cemql".to_owned(),
                    bytes: source.as_bytes().to_vec(),
                    from_format: None,
                    identity: Some(FormatIdentity {
                        content_type: Some(CEM_QL_CONTENT_TYPE.to_owned()),
                        schema: Some(CEM_QL_SCHEMA_URI.to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
                to_format: LayerFormat::Html,
                preserve_source_offsets: false,
                context: engine_context_with_cem_ql_template_adapter(),
                target: Some(FormatIdentity {
                    content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                    schema: Some(HTML_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig {
                    cemt_formatter: Some("cem-ql.format-tree".to_owned()),
                    cemt_formatter_profile: Some("tabular".to_owned()),
                    cemt_colorizer: Some("cem-ql.color-tree".to_owned()),
                    output_color_type: Some("html-css-vars".to_owned()),
                    ..ScopeConfig::default()
                },
                scheduler_scope_id: 0,
            })
            .expect("direct CEM-QL HTML convert should use registered handler");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        let primary = response.primary_bytes.expect("primary bytes");
        assert_eq!(primary.content_type, HTML_CONTENT_TYPE);
        assert_eq!(primary.schema.as_deref(), Some(HTML_SCHEMA_URI));
        let html = String::from_utf8(primary.bytes).expect("HTML output is UTF-8");
        assert!(html.starts_with(CEM_QL_HTML_PREVIEW_PREFIX), "{html}");
        assert!(html.ends_with(CEM_QL_HTML_PREVIEW_SUFFIX), "{html}");
        assert!(html.contains(r#"data-role="syntax.keyword""#), "{html}");
        assert!(html.contains(r#"data-role="syntax.string""#), "{html}");
        assert!(html.contains(r#"data-role="syntax.punctuation""#), "{html}");
        let stages = response
            .conversion
            .and_then(|metadata| metadata.output_pipeline)
            .expect("output pipeline metadata")
            .stages;
        assert_eq!(stages[0].function.as_deref(), Some("cem-ql.format-tree"));
        assert_eq!(stages[0].profile.as_deref(), Some("tabular"));
        assert_eq!(stages[1].function.as_deref(), Some("cem-ql.color-tree"));
        assert_eq!(stages[1].profile.as_deref(), Some("html"));
        assert_eq!(stages[2].content_type.as_deref(), Some(HTML_CONTENT_TYPE));
    }

    #[test]
    fn real_engine_transform_uses_registered_cem_ql_adapter() {
        let context = engine_context_with_cem_ql_template_adapter();
        let template_identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let request = TransformRequest {
            data: EngineInput {
                uri: "data.cem".to_owned(),
                bytes: b"{p @id=\"guide\"}".to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("text/cem-ml".to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template: TemplateInput {
                uri: "template.cem".to_owned(),
                bytes: br#"{p | {$datadom.attributes.kind}}"#.to_vec(),
                identity: Some(template_identity),
                root_scope: ScopeConfig::default(),
            },
            template_kind: TransformTemplateKind::CemNative,
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            params: BTreeMap::new(),
            preserve_source_offsets: false,
            context,
            target: Some(FormatIdentity {
                content_type: Some("text/html".to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds {
                data_load: 10,
                template_load: 11,
                execution: 12,
                output: 13,
            },
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform(request)
            .expect("transform should run through registered adapter");

        assert_eq!(
            response.primary,
            Value::String("<p>document</p>".to_owned())
        );
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert!(response.source_map.is_some());
        assert!(!response.output_spans.is_empty());
        assert_eq!(response.scheduler_trace.event_count, 12);
        let scopes = response
            .scheduler_trace
            .events
            .iter()
            .map(|event| event.scope_id)
            .collect::<Vec<_>>();
        assert!(scopes.contains(&10));
        assert!(scopes.contains(&11));
        assert!(scopes.contains(&12));
        assert!(scopes.contains(&13));
    }

    #[test]
    fn real_engine_transform_uses_registered_cem_ql_expression_adapter() {
        let context = engine_context_with_cem_ql_template_adapter();
        let request = TransformRequest {
            data: EngineInput {
                uri: "data.cem".to_owned(),
                bytes: b"{p @id=\"guide\"}".to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("text/cem-ml".to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template: TemplateInput {
                uri: "template.cem-ql".to_owned(),
                bytes: b"input.kind".to_vec(),
                identity: Some(FormatIdentity {
                    content_type: Some(
                        cem_ml::schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE.to_owned(),
                    ),
                    schema: Some(cem_ml::schema::registry::CEM_QL_EXPRESSION_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template_kind: TransformTemplateKind::CemQlExpression,
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            params: BTreeMap::new(),
            preserve_source_offsets: false,
            context,
            target: Some(FormatIdentity {
                content_type: Some(cem_ml::schema::registry::JSON_CONTENT_TYPE.to_owned()),
                schema: Some(cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds {
                data_load: 14,
                template_load: 15,
                execution: 16,
                output: 17,
            },
            execution_policy: TransformExecutionPolicy {
                runtime_phase: TransformRuntimePhase::CemQlExpression,
                ..TransformExecutionPolicy::default()
            },
        };

        let response = RealCemMlEngine::new()
            .transform(request)
            .expect("CEM-QL expression transform should execute");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(
            response.primary["items"],
            json!([{
                "kind": "atomic",
                "type": "string",
                "value": "document"
            }])
        );
        assert_eq!(response.primary["diagnostics"], json!([]));
        assert_eq!(response.primary["error"], Value::Null);
        assert!(response.source_map.is_none());
        assert!(response.output_spans.is_empty());
    }

    #[test]
    fn real_engine_transform_uses_registered_xslt_parity_adapter() {
        let context = engine_context_with_cem_ql_template_adapter();
        let request = TransformRequest {
            data: EngineInput {
                uri: "data.cem".to_owned(),
                bytes: b"{p @id=\"guide\"}".to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("text/cem-ml".to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template: TemplateInput {
                uri: "view.xsl".to_owned(),
                bytes: br#"<xsl:stylesheet version="1.0"><xsl:template match="/"><main><h1>Sign in</h1></main></xsl:template></xsl:stylesheet>"#.to_vec(),
                identity: Some(FormatIdentity {
                    content_type: Some("application/xslt+xml".to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template_kind: TransformTemplateKind::Xslt,
            template_entrypoint: TransformTemplateEntrypoint::implicit(),
            params: BTreeMap::new(),
            preserve_source_offsets: false,
            context,
            target: Some(FormatIdentity {
                content_type: Some("text/html".to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds {
                data_load: 20,
                template_load: 21,
                execution: 22,
                output: 23,
            },
            execution_policy: TransformExecutionPolicy {
                runtime_phase: TransformRuntimePhase::XsltParity,
                ..TransformExecutionPolicy::default()
            },
        };

        let response = RealCemMlEngine::new()
            .transform(request)
            .expect("XSLT parity transform should execute");

        assert_eq!(
            response.primary,
            Value::String("<main><h1>Sign in</h1></main>".to_owned())
        );
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
    }

    #[test]
    fn real_engine_transform_binds_cem_native_template_params() {
        let context = engine_context_with_cem_ql_template_adapter();
        let request = TransformRequest {
            data: EngineInput {
                uri: "data.cem".to_owned(),
                bytes: b"{p}".to_vec(),
                from_format: None,
                identity: Some(FormatIdentity {
                    content_type: Some("text/cem-ml".to_owned()),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template: TemplateInput {
                uri: "template.cem".to_owned(),
                bytes: br#"{@doc cem-ml 1}
{module |
  {param @name="locale" @default="en-US" @visibility="public"}
  {template @name="card" @visibility="public" |
    {param @name="title" @default="Untitled"}
    {body | {p | {$locale}:{$title}}}
  }
}"#
                .to_vec(),
                identity: Some(FormatIdentity {
                    schema: Some(
                        cem_ml::transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI.to_owned(),
                    ),
                    ..FormatIdentity::default()
                }),
                root_scope: ScopeConfig::default(),
            },
            template_kind: TransformTemplateKind::CemNative,
            template_entrypoint: TransformTemplateEntrypoint::named("card"),
            params: BTreeMap::from([
                ("locale".to_owned(), Value::String("fr-FR".to_owned())),
                ("title".to_owned(), Value::String("Intro".to_owned())),
            ]),
            preserve_source_offsets: false,
            context,
            target: Some(FormatIdentity {
                content_type: Some("text/html".to_owned()),
                ..FormatIdentity::default()
            }),
            target_scope: ScopeConfig::default(),
            scheduler_scope_ids: TransformSchedulerScopeIds {
                data_load: 10,
                template_load: 11,
                execution: 12,
                output: 13,
            },
            execution_policy: TransformExecutionPolicy {
                runtime_phase: cem_ml::engine::TransformRuntimePhase::CemNativeModules,
                ..TransformExecutionPolicy::default()
            },
        };

        let response = RealCemMlEngine::new()
            .transform(request)
            .expect("transform should bind CEM-native params");

        assert_eq!(
            response.primary,
            Value::String("<p>fr-FR:Intro</p>".to_owned()),
            "{:?}",
            response.diagnostics
        );
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
    }

    #[test]
    fn real_engine_transform_graph_executes_branched_cem_native_outputs() {
        let context = engine_context_with_cem_ql_template_adapter();
        let data_identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template_identity = data_identity.clone();
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "book".to_owned(),
                input: EngineInput {
                    uri: "book.cem".to_owned(),
                    bytes: b"{p @id=\"guide\"}".to_vec(),
                    from_format: None,
                    identity: Some(data_identity),
                    root_scope: ScopeConfig::default(),
                },
                scheduler_scope_id: 20,
            }],
            joins: Vec::new(),
            stages: vec![
                TransformGraphStage {
                    id: "html".to_owned(),
                    template: TemplateInput {
                        uri: "html.cem".to_owned(),
                        bytes: br#"{article | {$datadom.attributes.kind}}"#.to_vec(),
                        identity: Some(template_identity.clone()),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 21,
                        execution: 22,
                    },
                },
                TransformGraphStage {
                    id: "chart".to_owned(),
                    template: TemplateInput {
                        uri: "chart.cem".to_owned(),
                        bytes: br#"{svg | {$datadom.attributes.kind}}"#.to_vec(),
                        identity: Some(template_identity),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 23,
                        execution: 24,
                    },
                },
            ],
            importmap_rewrites: Vec::new(),
            exports: vec![
                TransformGraphExport {
                    id: "main".to_owned(),
                    input: "html".to_owned(),
                    destination: Some("dist/book.html".to_owned()),
                    target: Some(FormatIdentity {
                        content_type: Some("text/html".to_owned()),
                        ..FormatIdentity::default()
                    }),
                    target_scope: ScopeConfig::default(),
                    style_policy: Default::default(),
                    scheduler_scope_id: 25,
                },
                TransformGraphExport {
                    id: "chart-svg".to_owned(),
                    input: "chart".to_owned(),
                    destination: Some("dist/book/chart.svg".to_owned()),
                    target: Some(FormatIdentity {
                        content_type: Some("image/svg+xml".to_owned()),
                        ..FormatIdentity::default()
                    }),
                    target_scope: ScopeConfig::default(),
                    style_policy: Default::default(),
                    scheduler_scope_id: 26,
                },
            ],
            edges: Vec::new(),
            preserve_source_offsets: false,
            context,
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform_graph(request)
            .expect("transform graph should execute through registered adapter");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.artifacts.len(), 2);
        assert_eq!(response.artifacts[0].input, "html");
        assert_eq!(
            response.artifacts[0].primary,
            Value::String("<article>document</article>".to_owned())
        );
        assert_eq!(response.artifacts[1].input, "chart");
        assert_eq!(
            response.artifacts[1].primary,
            Value::String("<svg>document</svg>".to_owned())
        );
        assert_eq!(
            response.artifacts[1]
                .identity
                .as_ref()
                .and_then(|identity| identity.content_type.as_deref()),
            Some("image/svg+xml")
        );
        assert!(response
            .scheduler_trace
            .events
            .iter()
            .any(|event| event.scope_id == 26 && event.task == "chart-svg:export"));
    }

    #[test]
    fn real_engine_transform_graph_join_export_preserves_render_metadata() {
        let context = engine_context_with_cem_ql_template_adapter();
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "book".to_owned(),
                input: EngineInput {
                    uri: "book.cem".to_owned(),
                    bytes: b"{p @id=\"guide\"}".to_vec(),
                    from_format: None,
                    identity: Some(identity.clone()),
                    root_scope: ScopeConfig::default(),
                },
                scheduler_scope_id: 30,
            }],
            joins: vec![TransformGraphJoin {
                id: "collection".to_owned(),
                mode: TransformGraphJoinMode::Collect,
                input_names: vec!["primary".to_owned()],
                inputs: vec![TransformGraphJoinInput {
                    input_name: "primary".to_owned(),
                    artifact_id: "html".to_owned(),
                    bindings: BTreeMap::new(),
                    destination: None,
                    target: None,
                }],
                bindings: BTreeMap::new(),
                scheduler_scope_id: 33,
            }],
            stages: vec![TransformGraphStage {
                id: "html".to_owned(),
                template: TemplateInput {
                    uri: "html.cem".to_owned(),
                    bytes: br#"{article | {$datadom.attributes.kind}}"#.to_vec(),
                    identity: Some(identity),
                    root_scope: ScopeConfig::default(),
                },
                template_kind: TransformTemplateKind::CemNative,
                template_entrypoint: TransformTemplateEntrypoint::implicit(),
                params: BTreeMap::new(),
                execution_policy: TransformExecutionPolicy::default(),
                target: None,
                primary_input: "book".to_owned(),
                secondary_inputs: BTreeMap::new(),
                scheduler_scope_ids: TransformStageSchedulerScopeIds {
                    template_load: 31,
                    execution: 32,
                },
            }],
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "joined".to_owned(),
                input: "collection".to_owned(),
                destination: Some("dist/collection.json".to_owned()),
                target: Some(FormatIdentity {
                    content_type: Some("application/json".to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig::default(),
                style_policy: Default::default(),
                scheduler_scope_id: 34,
            }],
            edges: Vec::new(),
            preserve_source_offsets: false,
            context,
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform_graph(request)
            .expect("transform graph should execute join export");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.artifacts.len(), 1);
        let artifact = &response.artifacts[0];
        assert_eq!(artifact.input, "collection");
        assert!(artifact.source_map.is_some());
        assert!(!artifact.output_spans.is_empty());
        assert_eq!(artifact.primary["kind"], "collection");
        assert_eq!(artifact.primary["count"], 1);
    }

    #[test]
    fn real_engine_transform_graph_multi_input_join_preserves_per_item_metadata() {
        let context = engine_context_with_cem_ql_template_adapter();
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "book".to_owned(),
                input: EngineInput {
                    uri: "book.cem".to_owned(),
                    bytes: b"{p @id=\"guide\"}".to_vec(),
                    from_format: None,
                    identity: Some(identity.clone()),
                    root_scope: ScopeConfig::default(),
                },
                scheduler_scope_id: 40,
            }],
            joins: vec![TransformGraphJoin {
                id: "collection".to_owned(),
                mode: TransformGraphJoinMode::Collect,
                input_names: vec!["primary".to_owned()],
                inputs: vec![
                    TransformGraphJoinInput {
                        input_name: "primary".to_owned(),
                        artifact_id: "html".to_owned(),
                        bindings: BTreeMap::new(),
                        destination: None,
                        target: None,
                    },
                    TransformGraphJoinInput {
                        input_name: "primary".to_owned(),
                        artifact_id: "summary".to_owned(),
                        bindings: BTreeMap::new(),
                        destination: None,
                        target: None,
                    },
                ],
                bindings: BTreeMap::new(),
                scheduler_scope_id: 45,
            }],
            stages: vec![
                TransformGraphStage {
                    id: "html".to_owned(),
                    template: TemplateInput {
                        uri: "html.cem".to_owned(),
                        bytes: br#"{article | {$datadom.attributes.kind}}"#.to_vec(),
                        identity: Some(identity.clone()),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 41,
                        execution: 42,
                    },
                },
                TransformGraphStage {
                    id: "summary".to_owned(),
                    template: TemplateInput {
                        uri: "summary.cem".to_owned(),
                        bytes: br#"{section | {$datadom.attributes.kind}}"#.to_vec(),
                        identity: Some(identity),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 43,
                        execution: 44,
                    },
                },
            ],
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "joined".to_owned(),
                input: "collection".to_owned(),
                destination: Some("dist/collection.json".to_owned()),
                target: Some(FormatIdentity {
                    content_type: Some("application/json".to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig::default(),
                style_policy: Default::default(),
                scheduler_scope_id: 46,
            }],
            edges: Vec::new(),
            preserve_source_offsets: false,
            context,
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform_graph(request)
            .expect("transform graph should execute multi-input join export");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.artifacts.len(), 1);
        let artifact = &response.artifacts[0];
        assert_eq!(artifact.input, "collection");
        assert!(artifact.source_map.is_none());
        assert!(!artifact.output_spans.is_empty());
        let items = artifact.primary["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| !item["sourceMap"].is_null()
            && item["outputSpans"]
                .as_array()
                .is_some_and(|spans| !spans.is_empty())));
    }

    #[test]
    fn real_engine_transform_graph_attributes_stage_render_diagnostics() {
        let context = engine_context_with_cem_ql_template_adapter();
        let identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "book".to_owned(),
                input: EngineInput {
                    uri: "book.cem".to_owned(),
                    bytes: b"{p @id=\"guide\"}".to_vec(),
                    from_format: None,
                    identity: Some(identity.clone()),
                    root_scope: ScopeConfig::default(),
                },
                scheduler_scope_id: 40,
            }],
            joins: Vec::new(),
            stages: vec![TransformGraphStage {
                id: "chart".to_owned(),
                template: TemplateInput {
                    uri: "chart.cem".to_owned(),
                    bytes: br#"{@doc cem-ml 1}
{module |
  {template @name="loop" | {body | {span | Loop {call @template="loop"}}}}
  {body | {svg | {call @template="loop"}}}
}"#
                    .to_vec(),
                    identity: Some(identity),
                    root_scope: ScopeConfig::default(),
                },
                template_kind: TransformTemplateKind::CemNative,
                template_entrypoint: TransformTemplateEntrypoint::implicit(),
                params: BTreeMap::new(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::CemNativeModules,
                    ..TransformExecutionPolicy::default()
                },
                primary_input: "book".to_owned(),
                secondary_inputs: BTreeMap::new(),
                target: None,
                scheduler_scope_ids: TransformStageSchedulerScopeIds {
                    template_load: 41,
                    execution: 42,
                },
            }],
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "chart-svg".to_owned(),
                input: "chart".to_owned(),
                destination: Some("dist/book/chart.svg".to_owned()),
                target: Some(FormatIdentity {
                    content_type: Some("image/svg+xml".to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig::default(),
                style_policy: Default::default(),
                scheduler_scope_id: 43,
            }],
            edges: Vec::new(),
            preserve_source_offsets: false,
            context,
            execution_policy: TransformExecutionPolicy {
                runtime_phase: cem_ml::engine::TransformRuntimePhase::CemNativeModules,
                ..TransformExecutionPolicy::default()
            },
        };

        let response = RealCemMlEngine::new()
            .transform_graph(request)
            .expect("transform graph should return render diagnostics");

        assert!(response.artifacts.is_empty());
        assert!(response.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE
                && diagnostic.uri.as_deref() == Some("chart.cem")
                && diagnostic.node.as_deref() == Some("transform:chart")
        }));
    }

    #[test]
    fn real_engine_transform_graph_passes_secondary_inputs_to_stage() {
        let context = engine_context_with_cem_ql_template_adapter();
        let data_identity = FormatIdentity {
            content_type: Some("text/cem-ml".to_owned()),
            ..FormatIdentity::default()
        };
        let template_identity = data_identity.clone();
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "book".to_owned(),
                input: EngineInput {
                    uri: "book.cem".to_owned(),
                    bytes: b"{p @id=\"guide\"}".to_vec(),
                    from_format: None,
                    identity: Some(data_identity),
                    root_scope: ScopeConfig::default(),
                },
                scheduler_scope_id: 30,
            }],
            joins: Vec::new(),
            stages: vec![
                TransformGraphStage {
                    id: "stats".to_owned(),
                    template: TemplateInput {
                        uri: "stats.cem".to_owned(),
                        bytes: br#"{span | {$datadom.attributes.kind}}"#.to_vec(),
                        identity: Some(template_identity.clone()),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 31,
                        execution: 32,
                    },
                },
                TransformGraphStage {
                    id: "report".to_owned(),
                    template: TemplateInput {
                        uri: "report.cem".to_owned(),
                        bytes: br#"{section | {$input.kind}:{$stats}}"#.to_vec(),
                        identity: Some(template_identity),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "book".to_owned(),
                    secondary_inputs: BTreeMap::from([("stats".to_owned(), "stats".to_owned())]),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 33,
                        execution: 34,
                    },
                },
            ],
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "main".to_owned(),
                input: "report".to_owned(),
                destination: None,
                target: Some(FormatIdentity {
                    content_type: Some("text/html".to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig::default(),
                style_policy: Default::default(),
                scheduler_scope_id: 35,
            }],
            edges: Vec::new(),
            preserve_source_offsets: false,
            context,
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform_graph(request)
            .expect("transform graph should execute secondary input join");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.artifacts.len(), 1);
        assert_eq!(response.artifacts[0].input, "report");
        assert_eq!(
            response.artifacts[0].primary,
            Value::String(
                "<section>document:&lt;span&gt;document&lt;/span&gt;</section>".to_owned()
            )
        );
    }
}
