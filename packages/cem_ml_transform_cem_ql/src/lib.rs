//! CEM-native transform-template adapter backed by `cem_ql::render`.
//!
//! This crate intentionally sits above both `cem_ml` and `cem_ql`: `cem_ml`
//! owns the stable transform adapter contract, while `cem_ql` owns the current
//! CEM-native fragment renderer. Keeping the bridge here avoids a dependency
//! cycle from `cem_ml` back into `cem_ql`.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use cem_ml::conversion::{
    execute_conversion_output_pipeline_from_typed_cemt_subject_with_environment,
    ConversionOutputPipeline, ConversionOutputPipelineEnvironment, ConversionOutputPipelineStage,
};
use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::engine::{
    ConvertExecutionMetadata, ConvertOutputPipelineMetadata, ConvertOutputPipelineStageMetadata,
    ConvertRequest, ConvertRequestHandler, ConvertResponse, EngineContext, FormatIdentity,
    InputFormat, LayerFormat, PrimaryBytes, TemplateInput, TransformTemplateEntrypoint,
    TransformTemplateKind, TRANSFORM_TEMPLATE_UNSUPPORTED_CODE,
};
use cem_ml::interpreter::OutputSpan;
use cem_ml::legacy_custom_element::{
    convert_template_source, LegacyConversionDiagnostic, TEMPLATE_CONTENT_TYPES,
    UNSUPPORTED_CONSTRUCT_CODE, UNSUPPORTED_FUNCTION_CODE, XSLT_TEMPLATE_CONTENT_TYPES,
};
use cem_ml::lifecycle::LoadedInputAstStream;
use cem_ml::parser::document::CemDocument;
use cem_ml::parser::{AstNodeId, CemAstNode};
use cem_ml::projection::{CemTreeAstAttribute, CemTreeAstNode, CemTreeAstStream};
use cem_ml::query::{
    QueryAstOwner, QueryEncodedOutput, QueryEvaluatorAdapter, QueryExecutionRequest,
    QueryExecutionResult, QueryExportFormat, QueryExportRequest, QueryInputModel, QueryInputOwner,
    QueryLanguage, QueryNativeArtifact, QueryNativeResult, QueryPreparationRequest,
    QueryPreparedOwners, QueryResultExporter, QueryResultExporterRegistry, QueryRuntimeAdapter,
};
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
    HTML_CONTENT_TYPE, HTML_SCHEMA_URI, XSLT_SCHEMA_URI,
};
use cem_ml::source::{ByteRange, BytesSource, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenizer};
use cem_ml::transform_artifact::{
    CemtEvaluatorNumber, CemtEvaluatorRecordRef, CemtEvaluatorRecordView, CemtEvaluatorSequenceRef,
    CemtEvaluatorSequenceView, CemtEvaluatorValue, CemtEvaluatorValueKind, CemtEvaluatorValueRef,
    TransformArtifactBody, TransformArtifactCollection, TransformArtifactCollectionMode,
    TransformArtifactExportRequest, TransformArtifactExporter, TransformEncodedArtifact,
    TransformEncoding, TransformNativeArtifact,
};
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
    TransformTemplateParameterArena, TransformTemplateRenderRequest,
    TransformTemplateRenderResponse, TransformTemplateSourceMapPolicy,
    CEM_NATIVE_TEMPLATE_SCHEMA_URI, DEFAULT_FORMATTER_TAB_SIZE,
    TRANSFORM_TEMPLATE_CALL_UNKNOWN_CODE, TRANSFORM_TEMPLATE_PARAM_REQUIRED_CODE,
    TRANSFORM_TEMPLATE_PARAM_TYPE_CODE, TRANSFORM_TEMPLATE_RECURSION_LIMIT_CODE,
};
use cem_ml::validation::json::{
    json_document_ast_from_source_bytes, JsonNumberKind, JsonSourceValidationRequest, JsonValueAst,
};
use cem_ml::validation::xml::{XmlAttributeAst, XmlDocumentAst, XmlEventAst};
use cem_ql::api::{
    compile, compile_expression, evaluate, evaluate_with_abort, CompileContext, CompiledExpression,
    EvaluationContext, ParseResult, StandaloneExpressionBinding, StandaloneExpressionContext,
};
use cem_ql::eval::{
    AtomValue, BudgetAxis, EvalError, Item, ItemStream, QueryContextScope, QueryItemView,
    QueryItemViewKind,
};
use cem_ql::lexer::{CookedTokenPayload, Lexer, Token, TokenKind};
use cem_ql::parser::SurfaceNode;
use cem_ql::render::{
    compile_template, render_compiled_template, render_plan_to_html_with_source_map,
    render_plan_to_xml_with_source_map, CompileTemplateOptions, RenderPlan, RenderPlanAttribute,
    RenderPlanNode, TemplateArtifact, TemplateAttributeValue, TemplateData, TemplateNode,
};
use cem_ql::template::{
    compile_embedding, extract_embeddings, DefaultAttributeClassifier, EmbeddedExpression,
};
use cem_ql::types::Type;
use serde_json::{json, Map, Number, Value};

pub const CEM_QL_TEMPLATE_ADAPTER_ID: &str = "cem-ql-cem-native-template";
pub const CEM_QL_EXPRESSION_TEMPLATE_ADAPTER_ID: &str = "cem-ql-expression-template";
pub const XSLT_PARITY_TEMPLATE_ADAPTER_ID: &str = "cem-ql-xslt-parity-template";
const TRANSFORM_CALL_NODE: &str = "__cem_transform_call";
const CEM_QL_RESULT_REPRESENTATION_ID: &str = "cem-ql.result-sequence";
const CEM_QL_SOURCE_TOKEN_AST_REPRESENTATION_ID: &str = "cem-ql.source-token-ast";
const CEM_QL_QUERY_AST_REPRESENTATION_ID: &str = "cem-ql.expression-query-ast";
const CEM_QL_QUERY_INPUT_REPRESENTATION_ID: &str = "cem-ql.native-item-input";
const CEM_QL_QUERY_RESULT_REPRESENTATION_ID: &str = "cem-ql.query-result-sequence";
const CEM_QL_QUERY_INPUT_MODELS: &[QueryInputModel] = &[QueryInputModel::NativeItems];

#[derive(Debug, Clone, Copy)]
pub struct CemQlSourceValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
    pub schema: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CemQlTemplateEmbeddingIdentity<'a> {
    pub content_type: Option<&'a str>,
    pub schema: Option<&'a str>,
}

impl<'a> From<Option<&'a FormatIdentity>> for CemQlTemplateEmbeddingIdentity<'a> {
    fn from(identity: Option<&'a FormatIdentity>) -> Self {
        Self {
            content_type: identity.and_then(|identity| identity.content_type.as_deref()),
            schema: identity.and_then(|identity| identity.schema.as_deref()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CemQlTemplateEmbeddingValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub from_format: InputFormat,
    pub source_uri: Option<&'a str>,
    pub identity: CemQlTemplateEmbeddingIdentity<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct CemNativeTemplateExpressionValidationRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub content_type: Option<&'a str>,
    pub schema: Option<&'a str>,
}

#[derive(Debug, Clone, Default)]
pub struct CemQlTransformTemplateAdapter;

#[derive(Debug, Clone, Default)]
pub struct CemQlExpressionTransformTemplateAdapter;

#[derive(Debug, Clone, Default)]
pub struct XsltParityTransformTemplateAdapter;

#[derive(Debug, Clone, Default)]
pub struct CemQlSchemaBehaviorEvaluator;

#[derive(Debug, Clone)]
struct CemQlResultArtifact {
    stream: ItemStream,
}

impl TransformNativeArtifact for CemQlResultArtifact {
    fn representation_id(&self) -> &'static str {
        CEM_QL_RESULT_REPRESENTATION_ID
    }

    fn source_map(&self) -> Option<&SourceMapStack> {
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone, Default)]
struct CemQlJsonResultExporter;

impl TransformArtifactExporter for CemQlJsonResultExporter {
    fn id(&self) -> &'static str {
        "cem-ql.result-json"
    }

    fn representation_id(&self) -> &'static str {
        CEM_QL_RESULT_REPRESENTATION_ID
    }

    fn export(
        &self,
        request: TransformArtifactExportRequest<'_>,
    ) -> Result<Arc<TransformEncodedArtifact>, String> {
        let TransformArtifactBody::Extension(native) = request.body else {
            return Err(format!(
                "expected native `{CEM_QL_RESULT_REPRESENTATION_ID}` body, got `{}`",
                request.body.representation_id()
            ));
        };
        let result = native
            .as_any()
            .downcast_ref::<CemQlResultArtifact>()
            .ok_or_else(|| {
                "CEM-QL result body type does not match its representation".to_owned()
            })?;
        let bytes = serde_json::to_vec(&item_stream_json(&result.stream))
            .map_err(|error| format!("CEM-QL result JSON encoding failed: {error}"))?;
        TransformEncodedArtifact::new(request.target.clone(), TransformEncoding::Json, bytes)
            .map(Arc::new)
            .map_err(|error| error.to_string())
    }
}

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
    param_declarations: Vec<TransformTemplateModuleParamDeclaration>,
    entrypoints: CemQlTemplateEntrypoints,
    modules: Vec<CemQlCompiledTemplateModulePayload>,
    max_recursion_depth: u32,
}

#[derive(Debug, Clone)]
struct CemQlCompiledExpressionPayload {
    template_uri: String,
    compiled: CompiledExpression,
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
            .with_parameters(request.params.clone())
            .with_native_payload(CemQlCompiledTemplatePayload {
                template_uri: request.template.uri.clone(),
                artifact: render_artifact,
                selected_entrypoint: request.entrypoint.name.clone(),
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
                .map(|cache_key| cache_key.resolver_policy_stamp()),
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
                    )
                    .with_parameters(request.params.clone()),
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
            .with_parameters(request.params.clone())
            .with_native_payload(CemQlCompiledExpressionPayload {
                template_uri: request.template.uri.clone(),
                compiled,
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
        let source = xslt_source_for_entrypoint(source, request.entrypoint, request.params)
            .map_err(|message| {
                TransformTemplateAdapterError::failed(
                    self.id(),
                    TransformTemplateAdapterExecutionPhase::Compile,
                    message,
                )
            })?;
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
            .with_parameters(request.params.clone())
            .with_native_payload(CemQlCompiledTemplatePayload {
                template_uri: request.template.uri.clone(),
                artifact: render_artifact,
                selected_entrypoint: request.entrypoint.name.clone(),
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
    params: &TransformTemplateParameterArena,
) -> Result<String, String> {
    let Some(name) = entrypoint.name.as_deref() else {
        return Ok(source.to_owned());
    };

    let wrapper = xslt_entrypoint_wrapper(name, params)?;
    for closing in ["</xsl:stylesheet>", "</stylesheet>"] {
        if let Some(index) = source.rfind(closing) {
            let mut out = String::with_capacity(source.len() + wrapper.len());
            out.push_str(&source[..index]);
            out.push_str(&wrapper);
            out.push_str(&source[index..]);
            return Ok(out);
        }
    }

    Ok(format!(
        r#"<xsl:stylesheet version="1.0">{source}{wrapper}</xsl:stylesheet>"#
    ))
}

fn xslt_entrypoint_wrapper(
    name: &str,
    params: &TransformTemplateParameterArena,
) -> Result<String, String> {
    let mut out = format!(
        r#"<xsl:template match="/"><xsl:call-template name="{}">"#,
        xml_attr_escape(name)
    );
    for (name, value) in params.iter() {
        out.push_str(&format!(
            r#"<xsl:with-param name="{}">{}"#,
            xml_attr_escape(name),
            xml_text_escape(&xslt_param_text(name, value)?)
        ));
        out.push_str("</xsl:with-param>");
    }
    out.push_str("</xsl:call-template></xsl:template>");
    Ok(out)
}

fn xslt_param_text(name: &str, value: &CemtEvaluatorValue<'_>) -> Result<String, String> {
    match value.kind() {
        CemtEvaluatorValueKind::Null => Ok(String::new()),
        CemtEvaluatorValueKind::Boolean => value
            .as_bool()
            .map(|value| value.to_string())
            .ok_or_else(|| format!("XSLT entrypoint param `{name}` has no boolean value")),
        CemtEvaluatorValueKind::Number => value
            .as_number()
            .map(CemtEvaluatorNumber::key_string)
            .ok_or_else(|| format!("XSLT entrypoint param `{name}` has no numeric value")),
        CemtEvaluatorValueKind::String => value
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("XSLT entrypoint param `{name}` has no string value")),
        kind => Err(format!(
            "XSLT entrypoint param `{name}` must be scalar, got `{}`",
            kind.as_str()
        )),
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

pub fn register_cem_ql_artifact_exporters(context: &mut EngineContext) {
    context
        .transform_artifact_exporter_registry
        .register(CemQlJsonResultExporter);
}

pub fn register_cem_ql_runtime_adapters(context: &mut EngineContext) {
    register_cem_ql_template_adapter(&mut context.template_adapter_registry);
    register_cem_ql_schema_behavior_evaluator(context);
    register_cem_ql_source_output_converter(context);
    register_cem_ql_artifact_exporters(context);
    context
        .query_runtime_registry
        .register(CemQlQueryRuntimeAdapter);
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
const CEM_QL_HTML_PREVIEW_SUFFIX: &str = "</pre>";

fn cem_ql_html_preview_prefix(tab_size: usize) -> String {
    format!(
        r#"<pre class="cem-output cem-output-cem-ql" style="white-space: pre; tab-size: {tab_size}">"#
    )
}

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
    let execution = execute_conversion_output_pipeline_from_typed_cemt_subject_with_environment(
        &environment,
        &pipeline,
        CemtEvaluatorValue::borrowed(CemtEvaluatorValueRef::Record(
            CemtEvaluatorRecordRef::Package {
                record: &token_tree,
            },
        )),
        CEM_QL_SOURCE_TOKEN_AST_REPRESENTATION_ID,
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
        let prefix = cem_ql_html_preview_prefix(cem_ql_formatter_tab_size(&request.target_scope));
        content = format!("{prefix}{content}{CEM_QL_HTML_PREVIEW_SUFFIX}");
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

#[derive(Debug)]
struct CemQlSourceTokenTreeAst {
    input_uri: String,
    tokens: Vec<CemQlSourceTokenAst>,
}

#[derive(Debug)]
struct CemQlSourceTokenAst {
    input_uri: String,
    value: CemQlSourceTokenValueAst,
    source_map: SourceMapStack,
    output_span: OutputSpan,
}

#[derive(Debug)]
struct CemQlSourceTokenValueAst {
    index: usize,
    token: Token,
    lexeme: String,
    cooked: Option<CemQlCookedTokenAst>,
}

#[derive(Debug)]
struct CemQlCookedTokenAst(CookedTokenPayload);

fn cem_ql_source_token_tree(input_uri: &str, source: &str) -> CemQlSourceTokenTreeAst {
    let tokens = Lexer::new(source)
        .scan_all()
        .into_iter()
        .filter(|token| token.kind != TokenKind::EndOfInput)
        .enumerate()
        .map(|(index, token)| cem_ql_source_token_node(input_uri, source, index, &token))
        .collect::<Vec<_>>();
    CemQlSourceTokenTreeAst {
        input_uri: input_uri.to_owned(),
        tokens,
    }
}

fn cem_ql_source_token_node(
    input_uri: &str,
    source: &str,
    index: usize,
    token: &Token,
) -> CemQlSourceTokenAst {
    let lexeme = cem_ql_token_lexeme(source, token);
    let source_map = cem_ql_source_map(token.range);
    let output_span = OutputSpan {
        output_range: ByteRange::new(0, token.range.len),
        origin: source_map.clone(),
    };
    CemQlSourceTokenAst {
        input_uri: input_uri.to_owned(),
        value: CemQlSourceTokenValueAst {
            index,
            token: token.clone(),
            lexeme: lexeme.to_owned(),
            cooked: token.cooked.clone().map(CemQlCookedTokenAst),
        },
        source_map,
        output_span,
    }
}

impl CemtEvaluatorRecordView for CemQlSourceTokenTreeAst {
    fn field_names(&self) -> &'static [&'static str] {
        &["kind", "contentType", "schema", "sourceUri", "tokens"]
    }

    fn field<'a>(&'a self, name: &str) -> Option<CemtEvaluatorValueRef<'a>> {
        match name {
            "kind" => Some(CemtEvaluatorValueRef::String("cem-ql-source")),
            "contentType" => Some(CemtEvaluatorValueRef::String(CEM_QL_CONTENT_TYPE)),
            "schema" => Some(CemtEvaluatorValueRef::String(CEM_QL_SCHEMA_URI)),
            "sourceUri" => Some(CemtEvaluatorValueRef::String(&self.input_uri)),
            "tokens" => Some(CemtEvaluatorValueRef::Sequence(
                CemtEvaluatorSequenceRef::Package { sequence: self },
            )),
            _ => None,
        }
    }
}

impl CemtEvaluatorSequenceView for CemQlSourceTokenTreeAst {
    fn len(&self) -> usize {
        self.tokens.len()
    }

    fn item<'a>(&'a self, index: usize) -> Option<CemtEvaluatorValueRef<'a>> {
        self.tokens.get(index).map(|token| {
            CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::Package { record: token })
        })
    }
}

impl CemtEvaluatorRecordView for CemQlSourceTokenAst {
    fn field_names(&self) -> &'static [&'static str] {
        &[
            "kind",
            "tokenKind",
            "text",
            "lexeme",
            "role",
            "sourceUri",
            "sourceMap",
            "outputSpan",
            "value",
        ]
    }

    fn field<'a>(&'a self, name: &str) -> Option<CemtEvaluatorValueRef<'a>> {
        match name {
            "kind" => Some(CemtEvaluatorValueRef::String(cem_ql_token_node_kind(
                self.value.token.kind,
            ))),
            "tokenKind" => Some(CemtEvaluatorValueRef::String(cem_ql_token_kind_name(
                self.value.token.kind,
            ))),
            "text" | "lexeme" => Some(CemtEvaluatorValueRef::String(&self.value.lexeme)),
            "role" => Some(CemtEvaluatorValueRef::String(cem_ql_token_role(
                self.value.token.kind,
            ))),
            "sourceUri" => Some(CemtEvaluatorValueRef::String(&self.input_uri)),
            "sourceMap" => Some(CemtEvaluatorValueRef::SourceMap(&self.source_map)),
            "outputSpan" => Some(CemtEvaluatorValueRef::Record(
                CemtEvaluatorRecordRef::OutputSpan {
                    output_span: &self.output_span,
                },
            )),
            "value" => Some(CemtEvaluatorValueRef::Record(
                CemtEvaluatorRecordRef::Package {
                    record: &self.value,
                },
            )),
            _ => None,
        }
    }
}

impl CemtEvaluatorRecordView for CemQlSourceTokenValueAst {
    fn field_names(&self) -> &'static [&'static str] {
        &[
            "tokenKind",
            "lexeme",
            "byteOffset",
            "byteLength",
            "index",
            "role",
            "operator",
            "cemQlRole",
            "legacy",
            "diagnostic",
            "replacement",
            "cooked",
        ]
    }

    fn field<'a>(&'a self, name: &str) -> Option<CemtEvaluatorValueRef<'a>> {
        let operator = || cem_ql_token_operator(self.token.kind, &self.lexeme);
        match name {
            "tokenKind" => Some(CemtEvaluatorValueRef::String(cem_ql_token_kind_name(
                self.token.kind,
            ))),
            "lexeme" => Some(CemtEvaluatorValueRef::String(&self.lexeme)),
            "byteOffset" => Some(cem_ql_evaluator_u64(self.token.range.start)),
            "byteLength" => Some(cem_ql_evaluator_u64(u64::from(self.token.range.len))),
            "index" => Some(cem_ql_evaluator_u64(self.index as u64)),
            "role" => Some(CemtEvaluatorValueRef::String(cem_ql_token_role(
                self.token.kind,
            ))),
            "operator" => operator().map(CemtEvaluatorValueRef::String),
            "cemQlRole" => operator()
                .map(cem_ql_operator_role)
                .map(CemtEvaluatorValueRef::String),
            "legacy" if self.token.kind == TokenKind::XPathCompatWord => {
                Some(CemtEvaluatorValueRef::String(&self.lexeme))
            }
            "diagnostic" if self.token.kind == TokenKind::XPathCompatWord => Some(
                CemtEvaluatorValueRef::String(cem_ql_legacy_diagnostic_code(&self.lexeme)),
            ),
            "replacement" if self.token.kind == TokenKind::XPathCompatWord => Some(
                CemtEvaluatorValueRef::String(cem_ql_legacy_replacement(&self.lexeme)),
            ),
            "cooked" => self.cooked.as_ref().map(|cooked| {
                CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::Package { record: cooked })
            }),
            _ => None,
        }
    }
}

impl CemtEvaluatorRecordView for CemQlCookedTokenAst {
    fn field_names(&self) -> &'static [&'static str] {
        &["kind", "value", "prefix", "local"]
    }

    fn field<'a>(&'a self, name: &str) -> Option<CemtEvaluatorValueRef<'a>> {
        match (&self.0, name) {
            (CookedTokenPayload::Name(_), "kind") => Some(CemtEvaluatorValueRef::String("name")),
            (CookedTokenPayload::PrefixedName { .. }, "kind") => {
                Some(CemtEvaluatorValueRef::String("prefixed-name"))
            }
            (CookedTokenPayload::StringValue(_), "kind") => {
                Some(CemtEvaluatorValueRef::String("string"))
            }
            (CookedTokenPayload::IntValue(_), "kind") => {
                Some(CemtEvaluatorValueRef::String("integer"))
            }
            (CookedTokenPayload::DecimalValue(_), "kind") => {
                Some(CemtEvaluatorValueRef::String("decimal"))
            }
            (CookedTokenPayload::DoubleValue(_), "kind") => {
                Some(CemtEvaluatorValueRef::String("double"))
            }
            (CookedTokenPayload::BoolValue(_), "kind") => {
                Some(CemtEvaluatorValueRef::String("boolean"))
            }
            (CookedTokenPayload::Name(value), "value")
            | (CookedTokenPayload::StringValue(value), "value")
            | (CookedTokenPayload::DecimalValue(value), "value") => {
                Some(CemtEvaluatorValueRef::String(value))
            }
            (CookedTokenPayload::PrefixedName { prefix, .. }, "prefix") => {
                Some(CemtEvaluatorValueRef::String(prefix))
            }
            (CookedTokenPayload::PrefixedName { local, .. }, "local") => {
                Some(CemtEvaluatorValueRef::String(local))
            }
            (CookedTokenPayload::IntValue(value), "value") => Some(CemtEvaluatorValueRef::Number(
                CemtEvaluatorNumber::integer(*value),
            )),
            (CookedTokenPayload::DoubleValue(value), "value") => {
                CemtEvaluatorNumber::decimal(*value).map(CemtEvaluatorValueRef::Number)
            }
            (CookedTokenPayload::BoolValue(value), "value") => {
                Some(CemtEvaluatorValueRef::Boolean(*value))
            }
            _ => None,
        }
    }
}

fn cem_ql_evaluator_u64(value: u64) -> CemtEvaluatorValueRef<'static> {
    CemtEvaluatorValueRef::Number(CemtEvaluatorNumber::unsigned_integer(value))
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

/// Validate CEM-native host template embeddings against the CEM-QL compiler.
///
/// This mirrors the engine-facing CEM-template boundary: non-CEM inputs do not
/// host CEM-QL embeddings, schema-definition inputs leave behavior expressions
/// to the schema behavior evaluator, and native-template / transform identities
/// use the context-aware renderer compiler.
pub fn validate_cem_ql_template_embedding_source_bytes(
    request: CemQlTemplateEmbeddingValidationRequest<'_>,
) -> Vec<Diagnostic> {
    if !matches!(request.from_format, InputFormat::Cem) {
        return Vec::new();
    }
    if cem_ql_template_identity_is_transform(request.identity) {
        return validate_context_aware_template_embedding_source_bytes(request, true);
    }
    if cem_ql_template_identity_is_native_template(request.identity) {
        return validate_context_aware_template_embedding_source_bytes(request, false);
    }
    if cem_ql_template_identity_is_schema_definition(request.identity) {
        return Vec::new();
    }
    if cem_ql_template_identity_is_xslt(request.identity) {
        return Vec::new();
    }
    validate_raw_template_embedding_source_bytes(request)
}

pub fn validate_cem_native_template_embedded_expression_source_bytes(
    request: CemNativeTemplateExpressionValidationRequest<'_>,
) -> Vec<Diagnostic> {
    if !cem_native_template_expression_validation_request_matches_identity(&request) {
        return Vec::new();
    }

    let Ok(source) = std::str::from_utf8(request.bytes) else {
        return Vec::new();
    };

    let expressions =
        cem_ql::embedded::extract_embedded_expressions_from_source(request.source_uri, source);
    let mut diagnostics = cem_ql::embedded::compile_embedded_expressions(&expressions)
        .into_iter()
        .flat_map(|report| {
            report
                .diagnostics
                .into_iter()
                .map(|diagnostic| diagnostic.diagnostic)
        })
        .collect::<Vec<_>>();
    for diagnostic in &mut diagnostics {
        if diagnostic.uri.as_deref() == Some(request.source_uri) {
            diagnostic.uri = None;
        }
    }
    finish_cem_ql_source_validation_diagnostics(
        request.source_uri,
        request.bytes,
        &mut diagnostics,
    );
    diagnostics
}

fn validate_raw_template_embedding_source_bytes(
    request: CemQlTemplateEmbeddingValidationRequest<'_>,
) -> Vec<Diagnostic> {
    let tokens = tokenize_cem_source(request.bytes);
    let classifier = DefaultAttributeClassifier;
    let mut diagnostics = Vec::new();
    let ctx = CompileContext::default();
    for embedding in extract_embeddings(&tokens, &classifier) {
        let (_, diags) = compile_embedding(&embedding, &ctx);
        for diagnostic in diags {
            diagnostics.push(annotate_template_embedding_uri(
                diagnostic,
                request.source_uri,
                &embedding,
            ));
        }
    }
    diagnostics
}

fn validate_context_aware_template_embedding_source_bytes(
    request: CemQlTemplateEmbeddingValidationRequest<'_>,
    skip_cemt_function_bodies: bool,
) -> Vec<Diagnostic> {
    let source = std::str::from_utf8(request.bytes).unwrap_or("");
    let artifact = compile_template(
        source,
        &CompileTemplateOptions {
            host_bindings: Vec::new(),
            skip_cemt_function_bodies,
        },
    );
    artifact
        .diagnostics
        .into_iter()
        .map(|diagnostic| annotate_template_diagnostic_uri(diagnostic, request.source_uri))
        .collect()
}

fn cem_ql_template_identity_is_native_template(
    identity: CemQlTemplateEmbeddingIdentity<'_>,
) -> bool {
    identity
        .schema
        .is_some_and(|schema| matches!(schema.trim(), CEM_NATIVE_TEMPLATE_SCHEMA_URI))
        || identity.content_type.is_some_and(|content_type| {
            matches!(
                content_type_essence(content_type).as_str(),
                cem_ml::schema::registry::CEM_NATIVE_TEMPLATE_CONTENT_TYPE
            )
        })
}

fn cem_ql_template_identity_is_transform(identity: CemQlTemplateEmbeddingIdentity<'_>) -> bool {
    identity.schema.is_some_and(|schema| {
        matches!(
            schema.trim(),
            cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI
        )
    }) || identity.content_type.is_some_and(|content_type| {
        matches!(
            content_type_essence(content_type).as_str(),
            cem_ml::schema::registry::CEM_TRANSFORM_CONTENT_TYPE
        )
    })
}

fn cem_ql_template_identity_is_schema_definition(
    identity: CemQlTemplateEmbeddingIdentity<'_>,
) -> bool {
    identity
        .schema
        .is_some_and(|schema| matches!(schema.trim(), cem_ml::schema::registry::CEM_SCHEMA_URI))
        || identity.content_type.is_some_and(|content_type| {
            matches!(
                content_type_essence(content_type).as_str(),
                cem_ml::schema::registry::CEM_SCHEMA_CONTENT_TYPE
            )
        })
}

fn cem_ql_template_identity_is_xslt(identity: CemQlTemplateEmbeddingIdentity<'_>) -> bool {
    identity
        .schema
        .is_some_and(|schema| schema.trim() == XSLT_SCHEMA_URI)
        || identity.content_type.is_some_and(|content_type| {
            let content_type = content_type_essence(content_type);
            XSLT_TEMPLATE_CONTENT_TYPES.contains(&content_type.as_str())
        })
}

fn cem_native_template_expression_validation_request_matches_identity(
    request: &CemNativeTemplateExpressionValidationRequest<'_>,
) -> bool {
    request.schema.map(str::trim) == Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI)
        || request.content_type.is_some_and(|content_type| {
            content_type_essence(content_type)
                == cem_ml::schema::registry::CEM_NATIVE_TEMPLATE_CONTENT_TYPE
        })
}

fn tokenize_cem_source(bytes: &[u8]) -> Vec<SchemaToken> {
    let src = BytesSource::new(SourceId(1), bytes.to_vec());
    let mut tokenizer = CemTokenizer::from_source(src);
    let _ = tokenizer.take_diagnostics();
    let mut out = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        out.push(token);
    }
    out
}

fn annotate_template_embedding_uri(
    mut diagnostic: Diagnostic,
    uri: Option<&str>,
    _embedding: &EmbeddedExpression,
) -> Diagnostic {
    if diagnostic.uri.is_none() {
        diagnostic.uri = uri.map(str::to_owned);
    }
    diagnostic
}

fn annotate_template_diagnostic_uri(mut diagnostic: Diagnostic, uri: Option<&str>) -> Diagnostic {
    if diagnostic.uri.is_none() {
        diagnostic.uri = uri.map(str::to_owned);
    }
    diagnostic
}

pub fn validate_cem_ql_source_bytes(request: CemQlSourceValidationRequest<'_>) -> Vec<Diagnostic> {
    let Ok(source) = std::str::from_utf8(request.bytes) else {
        return vec![cem_ql_invalid_utf8_diagnostic(request.source_uri)];
    };

    let mut diagnostics = if cem_ql_source_validation_request_is_expression(&request) {
        let context =
            StandaloneExpressionContext::default().with_input(ItemStream::empty(), Type::Any);
        match compile_expression(source, &context) {
            Ok(compiled) => compiled.diagnostics,
            Err(error) => error.diagnostics,
        }
    } else {
        let mut diagnostics = Vec::new();
        let parsed = cem_ql::api::parse(source);
        if !parsed.module.nodes.iter().any(|node| {
            matches!(
                node,
                SurfaceNode::Module(module) if !module.uri.trim().is_empty()
            )
        }) {
            diagnostics.push(cem_ql_module_uri_missing_diagnostic(request.source_uri));
        }
        diagnostics.extend(
            cem_ql::api::resolve_imports(&parsed.module, &cem_ql::resolve::ImportPolicy::new())
                .into_iter(),
        );
        diagnostics.extend(parsed.diagnostics);
        if !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            diagnostics.extend(
                cem_ql::api::type_check(&parsed.module, &CompileContext::default()).into_iter(),
            );
        }
        diagnostics
    };

    finish_cem_ql_source_validation_diagnostics(
        request.source_uri,
        request.bytes,
        &mut diagnostics,
    );
    diagnostics
}

fn cem_ql_source_validation_request_is_expression(
    request: &CemQlSourceValidationRequest<'_>,
) -> bool {
    request
        .content_type
        .map(content_type_essence)
        .is_some_and(|content_type| {
            content_type == cem_ml::schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE
        })
        || request.schema.map(str::trim)
            == Some(cem_ml::schema::registry::CEM_QL_EXPRESSION_SCHEMA_URI)
}

fn finish_cem_ql_source_validation_diagnostics(
    source_uri: &str,
    bytes: &[u8],
    diagnostics: &mut [Diagnostic],
) {
    cem_ml::diagnostics::project_diagnostics_for_source(diagnostics, bytes);
    for diagnostic in diagnostics {
        diagnostic.uri.get_or_insert_with(|| source_uri.to_owned());
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

fn cem_ql_formatter_tab_size(target_scope: &ScopeConfig) -> usize {
    target_scope
        .cemt_formatter_options
        .get("tabSize")
        .map(String::as_str)
        .map(str::trim)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_FORMATTER_TAB_SIZE as usize)
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
    let data = template_data_from_artifacts(request.primary_input, request.secondary_inputs)
        .map_err(|message| {
            TransformTemplateAdapterError::failed(
                adapter_id,
                TransformTemplateAdapterExecutionPhase::Render,
                message,
            )
        })?;
    let plan = render_payload_template(payload, request.compiled.parameters(), &data).map_err(
        |message| {
            TransformTemplateAdapterError::failed(
                adapter_id,
                TransformTemplateAdapterExecutionPhase::Render,
                message,
            )
        },
    )?;
    if target_is_cem_tree(request.target) {
        return Ok(TransformTemplateRenderResponse {
            output: TransformTemplateOutputArtifact {
                uri: None,
                identity: request.target.cloned(),
                body: TransformArtifactBody::CemTree(Arc::new(render_plan_to_cem_tree_nodes(
                    &plan,
                ))),
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
    let identity = request.target.cloned().unwrap_or_else(|| FormatIdentity {
        content_type: Some("text/html".to_owned()),
        ..FormatIdentity::default()
    });

    let output = TransformTemplateOutputArtifact::encoded_text(None, identity, rendered.rendered)
        .map_err(|error| {
            TransformTemplateAdapterError::failed(
                adapter_id,
                TransformTemplateAdapterExecutionPhase::Render,
                error.to_string(),
            )
        })?
        .with_metadata(Some(rendered.source_map), rendered.output_spans);
    Ok(TransformTemplateRenderResponse {
        output,
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
    let policy_bindings =
        expression_policy_bindings(request.primary_input, request.compiled.parameters()).map_err(
            |message| {
                TransformTemplateAdapterError::failed(
                    adapter_id,
                    TransformTemplateAdapterExecutionPhase::Render,
                    message,
                )
            },
        )?;
    let result = evaluate(
        &payload.compiled.query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root(),
            diagnostics: Vec::new(),
            policy_bindings,
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
            body: TransformArtifactBody::Extension(Arc::new(CemQlResultArtifact {
                stream: result,
            })),
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

fn render_plan_to_cem_tree_nodes(plan: &RenderPlan) -> CemTreeAstStream {
    CemTreeAstStream::new(
        plan.nodes
            .iter()
            .filter_map(render_plan_node_to_cem_tree)
            .collect(),
    )
}

fn render_plan_node_to_cem_tree(node: &RenderPlanNode) -> Option<CemTreeAstNode> {
    match node {
        RenderPlanNode::Element { tag, .. } if tag.trim().is_empty() => None,
        RenderPlanNode::Element {
            tag,
            namespace,
            attributes,
            children,
            source_map,
        } => Some(CemTreeAstNode::Element {
            name: render_plan_cem_tree_name(tag, namespace.as_deref()),
            attributes: attributes
                .iter()
                .map(render_plan_attribute_to_cem_tree)
                .collect::<Vec<_>>(),
            children: children
                .iter()
                .filter_map(render_plan_node_to_cem_tree)
                .collect::<Vec<_>>(),
            source: source_map.clone(),
        }),
        RenderPlanNode::Text { text, source_map } if text.trim().is_empty() => {
            Some(CemTreeAstNode::Whitespace {
                data: text.clone(),
                source: source_map.clone(),
            })
        }
        RenderPlanNode::Text { text, source_map } => Some(CemTreeAstNode::Text {
            value: text.clone(),
            source: source_map.clone(),
        }),
        RenderPlanNode::Comment { text, source_map } => Some(CemTreeAstNode::Comment {
            data: text.clone(),
            source: source_map.clone(),
        }),
        RenderPlanNode::Cdata { text, source_map } => Some(CemTreeAstNode::Cdata {
            data: text.clone(),
            source: source_map.clone(),
        }),
        RenderPlanNode::ProcessingInstruction {
            target,
            data,
            source_map,
        } => Some(CemTreeAstNode::ProcessingInstruction {
            name: target.clone(),
            target: target.clone(),
            data: data.clone(),
            source: source_map.clone(),
        }),
    }
}

fn render_plan_attribute_to_cem_tree(attribute: &RenderPlanAttribute) -> CemTreeAstAttribute {
    CemTreeAstAttribute {
        name: render_plan_cem_tree_name(&attribute.name, attribute.namespace.as_deref()),
        value: Some(attribute.value.clone()),
        source: attribute.source_map.clone(),
    }
}

fn render_plan_cem_tree_name(local_name: &str, namespace: Option<&str>) -> String {
    namespace
        .map(str::trim)
        .filter(|namespace| !namespace.is_empty())
        .map(|namespace| format!("{namespace}:{local_name}"))
        .unwrap_or_else(|| local_name.to_owned())
}

fn host_binding_names(
    params: &TransformTemplateParameterArena,
    data_bindings: &[String],
    module_options: &TransformTemplateModuleOptions,
) -> Vec<String> {
    let mut bindings = data_bindings.to_vec();
    for (name, _) in params.iter() {
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
            | TemplateNode::ProjectPayload { .. }
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
    params: &TransformTemplateParameterArena,
    data: &TemplateData,
) -> Result<RenderPlan, String> {
    let data = root_template_data_with_params(payload, params, data)?;
    let mut plan = render_compiled_template(&payload.artifact, &data);
    fill_diagnostic_uri(&mut plan.diagnostics, payload.template_uri.as_str());
    let nodes = expand_call_nodes(&plan.nodes, payload, None, &data, 0, &mut plan.diagnostics);
    Ok(RenderPlan {
        nodes,
        diagnostics: plan.diagnostics,
    })
}

fn root_template_data_with_params(
    payload: &CemQlCompiledTemplatePayload,
    params: &TransformTemplateParameterArena,
    data: &TemplateData,
) -> Result<TemplateData, String> {
    let mut data = data.clone();
    for (name, value) in params.iter() {
        bind_param_value(
            &mut data,
            payload.selected_entrypoint.as_deref(),
            name,
            value,
        )?;
    }
    apply_param_declarations(
        &mut data,
        &payload.param_declarations,
        payload.selected_entrypoint.as_deref(),
    );
    Ok(data)
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
    value: &CemtEvaluatorValue<'_>,
) -> Result<(), String> {
    let stream = evaluator_param_value_to_stream(value)?;
    data.bindings.insert(name.to_owned(), stream.clone());
    if let Some((qualified, local)) = entrypoint_param_aliases(name, selected_entrypoint) {
        data.bindings
            .entry(qualified)
            .or_insert_with(|| stream.clone());
        data.bindings.entry(local).or_insert(stream);
    }
    Ok(())
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
                    let stream = explicit_param_value_to_stream(default_value);
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
                data.bindings.insert(
                    declaration.name.clone(),
                    explicit_param_value_to_stream(default_value),
                );
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

    if matches!(item.atom(), Some(AtomValue::Null)) {
        return declaration.nullable;
    }

    match declaration.value_type {
        ParamType::Any => true,
        ParamType::String => matches!(
            item.atom(),
            Some(AtomValue::String(_) | AtomValue::AnyUri(_))
        ),
        ParamType::Boolean => matches!(item.atom(), Some(AtomValue::Boolean(_))),
        ParamType::Number => matches!(
            item.atom(),
            Some(AtomValue::Integer(_) | AtomValue::Decimal(_) | AtomValue::Double(_))
        ),
        ParamType::Integer => matches!(item.atom(), Some(AtomValue::Integer(_))),
        ParamType::Array => {
            matches!(item, Item::Array(_))
                || item
                    .view()
                    .is_some_and(|view| view.kind() == QueryItemViewKind::Array)
        }
        ParamType::Object => {
            matches!(item, Item::Record(_))
                || item
                    .view()
                    .is_some_and(|view| view.kind() == QueryItemViewKind::Record)
        }
        ParamType::Json => {
            item.atom().is_some()
                || matches!(item, Item::Array(_) | Item::Record(_))
                || item.view().is_some_and(|view| {
                    matches!(
                        view.kind(),
                        QueryItemViewKind::Record | QueryItemViewKind::Array
                    )
                })
        }
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

#[derive(Debug, Clone)]
struct CemDocumentQueryView {
    document: Arc<CemDocument>,
    node_id: AstNodeId,
}

impl CemDocumentQueryView {
    fn item(document: Arc<CemDocument>, node_id: AstNodeId) -> Item {
        Item::native(Self { document, node_id })
    }

    fn node(&self) -> Option<&CemAstNode> {
        self.document.get(self.node_id)
    }

    fn field_names(&self) -> &'static [&'static str] {
        match self.node() {
            Some(CemAstNode::Document { .. }) => &["kind", "children"],
            Some(CemAstNode::Element { .. }) => {
                &["kind", "name", "namespace", "attributes", "children"]
            }
            Some(CemAstNode::Attribute { .. }) => &["kind", "name", "namespace", "value"],
            Some(CemAstNode::Text { .. })
            | Some(CemAstNode::Whitespace { .. })
            | Some(CemAstNode::Comment { .. })
            | Some(CemAstNode::Cdata { .. })
            | Some(CemAstNode::RawText { .. }) => &["kind", "data", "value"],
            Some(CemAstNode::ProcessingInstruction { .. }) => {
                &["kind", "name", "target", "data", "value"]
            }
            Some(CemAstNode::Error { .. }) => &["kind", "code"],
            None => &[],
        }
    }

    fn child_items(&self, ids: &[AstNodeId]) -> Vec<Item> {
        ids.iter()
            .filter(|id| self.document.get(**id).is_some())
            .map(|id| Self::item(Arc::clone(&self.document), *id))
            .collect()
    }
}

impl QueryItemView for CemDocumentQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.cem-document-view"
    }

    fn identity(&self) -> String {
        format!("{:p}:{}", Arc::as_ptr(&self.document), self.node_id)
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            self.field_names()
                .iter()
                .map(|name| ((*name).to_owned(), self.field(name).unwrap_or_default()))
                .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let node = self.node()?;
        let values = match (node, name) {
            (CemAstNode::Document { .. }, "kind") => atom_items("document"),
            (CemAstNode::Document { root_children, .. }, "children") => {
                self.child_items(root_children)
            }
            (CemAstNode::Element { .. }, "kind") => atom_items("element"),
            (CemAstNode::Element { expanded_name, .. }, "name") => {
                atom_items(expanded_name.local_name.clone())
            }
            (CemAstNode::Element { expanded_name, .. }, "namespace") => {
                atom_items(expanded_name.namespace_uri.clone())
            }
            (CemAstNode::Element { attributes, .. }, "attributes") => {
                vec![Item::Array(self.child_items(attributes))]
            }
            (CemAstNode::Element { children, .. }, "children") => {
                vec![Item::Array(self.child_items(children))]
            }
            (CemAstNode::Attribute { .. }, "kind") => atom_items("attribute"),
            (CemAstNode::Attribute { expanded_name, .. }, "name") => {
                atom_items(expanded_name.local_name.clone())
            }
            (CemAstNode::Attribute { expanded_name, .. }, "namespace") => {
                atom_items(expanded_name.namespace_uri.clone())
            }
            (CemAstNode::Attribute { value, .. }, "value") => vec![value
                .as_ref()
                .map(|value| Item::Atomic(AtomValue::String(value.clone())))
                .unwrap_or(Item::Atomic(AtomValue::Null))],
            (CemAstNode::Text { .. }, "kind") => atom_items("text"),
            (CemAstNode::Whitespace { .. }, "kind") => atom_items("whitespace"),
            (CemAstNode::Comment { .. }, "kind") => atom_items("comment"),
            (CemAstNode::Cdata { .. }, "kind") => atom_items("cdata"),
            (CemAstNode::RawText { .. }, "kind") => atom_items("raw-text"),
            (CemAstNode::Text { data, .. }, "data" | "value")
            | (CemAstNode::Whitespace { data, .. }, "data" | "value")
            | (CemAstNode::Comment { data, .. }, "data" | "value")
            | (CemAstNode::Cdata { data, .. }, "data" | "value")
            | (CemAstNode::RawText { data, .. }, "data" | "value") => atom_items(data.clone()),
            (CemAstNode::ProcessingInstruction { .. }, "kind") => {
                atom_items("processing-instruction")
            }
            (CemAstNode::ProcessingInstruction { target, .. }, "name" | "target") => {
                atom_items(target.clone())
            }
            (CemAstNode::ProcessingInstruction { data, .. }, "data" | "value") => {
                atom_items(data.clone())
            }
            (CemAstNode::Error { .. }, "kind") => atom_items("error"),
            (CemAstNode::Error { code, .. }, "code") => atom_items(code.clone()),
            _ => Vec::new(),
        };
        Some(values)
    }

    fn source_map(&self) -> Option<SourceMapStack> {
        match self.node()? {
            CemAstNode::Document { source, .. }
            | CemAstNode::Element { source, .. }
            | CemAstNode::Attribute { source, .. }
            | CemAstNode::Text { source, .. }
            | CemAstNode::Whitespace { source, .. }
            | CemAstNode::Comment { source, .. }
            | CemAstNode::ProcessingInstruction { source, .. }
            | CemAstNode::Cdata { source, .. }
            | CemAstNode::RawText { source, .. }
            | CemAstNode::Error { source, .. } => Some(source.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonQueryPathSegment {
    Member(usize),
    Index(usize),
}

#[derive(Debug, Clone)]
struct JsonValueQueryView {
    owner: Arc<LoadedInputAstStream>,
    path: Vec<JsonQueryPathSegment>,
}

impl JsonValueQueryView {
    fn root(owner: Arc<LoadedInputAstStream>) -> Option<Item> {
        let LoadedInputAstStream::JsonDocument(document) = owner.as_ref() else {
            return None;
        };
        document.root.as_ref()?;
        Some(Item::native(Self {
            owner,
            path: Vec::new(),
        }))
    }

    fn value(&self) -> Option<&JsonValueAst> {
        let LoadedInputAstStream::JsonDocument(document) = self.owner.as_ref() else {
            return None;
        };
        let mut value = document.root.as_ref()?;
        for segment in &self.path {
            value = match (value, segment) {
                (JsonValueAst::Object { members, .. }, JsonQueryPathSegment::Member(index)) => {
                    &members.get(*index)?.value
                }
                (JsonValueAst::Array { items, .. }, JsonQueryPathSegment::Index(index)) => {
                    items.get(*index)?
                }
                _ => return None,
            };
        }
        Some(value)
    }

    fn child(&self, segment: JsonQueryPathSegment) -> Item {
        let mut path = self.path.clone();
        path.push(segment);
        Item::native(Self {
            owner: Arc::clone(&self.owner),
            path,
        })
    }
}

impl QueryItemView for JsonValueQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.json-ast-view"
    }

    fn identity(&self) -> String {
        let range = self.value().map(JsonValueAst::range);
        format!(
            "{:p}:{}:{}",
            Arc::as_ptr(&self.owner),
            range.map(|range| range.start.byte_offset).unwrap_or(0),
            range.map(|range| range.byte_length).unwrap_or(0)
        )
    }

    fn kind(&self) -> QueryItemViewKind {
        match self.value() {
            Some(JsonValueAst::Object { .. }) => QueryItemViewKind::Record,
            Some(JsonValueAst::Array { .. }) => QueryItemViewKind::Array,
            _ => QueryItemViewKind::Atomic,
        }
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        let JsonValueAst::Object { members, .. } = self.value()? else {
            return None;
        };
        let mut names = Vec::new();
        let mut seen = BTreeSet::new();
        for member in members {
            if seen.insert(member.name.clone()) {
                names.push(member.name.clone());
            }
        }
        Some(
            names
                .into_iter()
                .map(|name| {
                    let items = self.field(&name).unwrap_or_default();
                    (name, items)
                })
                .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let JsonValueAst::Object { members, .. } = self.value()? else {
            return None;
        };
        Some(
            members
                .iter()
                .enumerate()
                .filter(|(_, member)| member.name == name)
                .map(|(index, _)| self.child(JsonQueryPathSegment::Member(index)))
                .collect(),
        )
    }

    fn members(&self) -> Option<Vec<Item>> {
        let JsonValueAst::Array { items, .. } = self.value()? else {
            return None;
        };
        Some(
            items
                .iter()
                .enumerate()
                .map(|(index, _)| self.child(JsonQueryPathSegment::Index(index)))
                .collect(),
        )
    }

    fn atom(&self) -> Option<AtomValue> {
        match self.value()? {
            JsonValueAst::String { value, .. } => Some(AtomValue::String(value.clone())),
            JsonValueAst::Number {
                lexeme,
                number_kind,
                ..
            } => match number_kind {
                JsonNumberKind::Integer => lexeme
                    .parse::<i64>()
                    .map(AtomValue::Integer)
                    .ok()
                    .or_else(|| Some(AtomValue::Decimal(lexeme.clone()))),
                JsonNumberKind::Decimal => Some(AtomValue::Decimal(lexeme.clone())),
                JsonNumberKind::Exponent => lexeme
                    .parse::<f64>()
                    .map(AtomValue::Double)
                    .ok()
                    .or_else(|| Some(AtomValue::Decimal(lexeme.clone()))),
            },
            JsonValueAst::Boolean { value, .. } => Some(AtomValue::Boolean(*value)),
            JsonValueAst::Null { .. } => Some(AtomValue::Null),
            JsonValueAst::Object { .. } | JsonValueAst::Array { .. } => None,
        }
    }

    fn source_map(&self) -> Option<SourceMapStack> {
        self.value().map(|value| value.range().source_map())
    }
}

#[derive(Debug, Clone)]
struct XmlDocumentQueryView {
    owner: Arc<LoadedInputAstStream>,
}

impl XmlDocumentQueryView {
    fn document(&self) -> Option<&XmlDocumentAst> {
        match self.owner.as_ref() {
            LoadedInputAstStream::XmlDocument(document) => Some(document),
            _ => None,
        }
    }
}

impl QueryItemView for XmlDocumentQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.xml-document-view"
    }

    fn identity(&self) -> String {
        "xml:document".to_owned()
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            ["kind", "resourceKind", "events"]
                .into_iter()
                .map(|name| (name.to_owned(), self.field(name).unwrap_or_default()))
                .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let document = self.document()?;
        Some(match name {
            "kind" => atom_items("xml-document"),
            "resourceKind" => atom_items(document.resource_kind.clone()),
            "events" => document
                .events
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    Item::native(XmlEventQueryView {
                        owner: Arc::clone(&self.owner),
                        index,
                    })
                })
                .collect(),
            _ => Vec::new(),
        })
    }
}

#[derive(Debug, Clone)]
struct XmlEventQueryView {
    owner: Arc<LoadedInputAstStream>,
    index: usize,
}

impl XmlEventQueryView {
    fn event(&self) -> Option<&XmlEventAst> {
        let LoadedInputAstStream::XmlDocument(document) = self.owner.as_ref() else {
            return None;
        };
        document.events.get(self.index)
    }
}

impl QueryItemView for XmlEventQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.xml-event-view"
    }

    fn identity(&self) -> String {
        format!("xml:event:{}", self.index)
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            [
                "kind",
                "depth",
                "qualifiedName",
                "localName",
                "prefix",
                "namespaceUri",
                "attributes",
                "value",
                "lexeme",
                "whitespaceOnly",
            ]
            .into_iter()
            .map(|name| (name.to_owned(), self.field(name).unwrap_or_default()))
            .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let event = self.event()?;
        Some(match name {
            "kind" => atom_items(event.kind.as_str()),
            "depth" => vec![Item::Atomic(AtomValue::Integer(
                i64::try_from(event.depth).unwrap_or(i64::MAX),
            ))],
            "qualifiedName" => optional_atom_items(event.qualified_name.as_deref()),
            "localName" => optional_atom_items(event.local_name.as_deref()),
            "prefix" => optional_atom_items(event.prefix.as_deref()),
            "namespaceUri" => optional_atom_items(event.namespace_uri.as_deref()),
            "attributes" => vec![Item::Array(
                event
                    .attributes
                    .iter()
                    .enumerate()
                    .map(|(attribute_index, _)| {
                        Item::native(XmlAttributeQueryView {
                            owner: Arc::clone(&self.owner),
                            event_index: self.index,
                            attribute_index,
                        })
                    })
                    .collect(),
            )],
            "value" => optional_atom_items(event.value.as_deref()),
            "lexeme" => atom_items(event.lexeme.clone()),
            "whitespaceOnly" => vec![Item::Atomic(AtomValue::Boolean(event.whitespace_only))],
            _ => Vec::new(),
        })
    }

    fn source_map(&self) -> Option<SourceMapStack> {
        self.event().map(|event| event.source_range.source_map())
    }
}

#[derive(Debug, Clone)]
struct XmlAttributeQueryView {
    owner: Arc<LoadedInputAstStream>,
    event_index: usize,
    attribute_index: usize,
}

impl XmlAttributeQueryView {
    fn attribute(&self) -> Option<&XmlAttributeAst> {
        let LoadedInputAstStream::XmlDocument(document) = self.owner.as_ref() else {
            return None;
        };
        document
            .events
            .get(self.event_index)?
            .attributes
            .get(self.attribute_index)
    }
}

impl QueryItemView for XmlAttributeQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.xml-attribute-view"
    }

    fn identity(&self) -> String {
        format!(
            "xml:event:{}:attribute:{}",
            self.event_index, self.attribute_index
        )
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            [
                "qualifiedName",
                "localName",
                "prefix",
                "namespaceUri",
                "value",
            ]
            .into_iter()
            .map(|name| (name.to_owned(), self.field(name).unwrap_or_default()))
            .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let attribute = self.attribute()?;
        Some(match name {
            "qualifiedName" => atom_items(attribute.qualified_name.clone()),
            "localName" => atom_items(attribute.local_name.clone()),
            "prefix" => optional_atom_items(attribute.prefix.as_deref()),
            "namespaceUri" => optional_atom_items(attribute.namespace_uri.as_deref()),
            "value" => atom_items(attribute.value.clone()),
            _ => Vec::new(),
        })
    }

    fn source_map(&self) -> Option<SourceMapStack> {
        let LoadedInputAstStream::XmlDocument(document) = self.owner.as_ref() else {
            return None;
        };
        document
            .events
            .get(self.event_index)
            .map(|event| event.source_range.source_map())
    }
}

#[derive(Debug, Clone)]
struct TransformCollectionQueryView {
    collection: Arc<TransformArtifactCollection>,
}

impl QueryItemView for TransformCollectionQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.transform-collection-view"
    }

    fn identity(&self) -> String {
        format!("{:p}:collection", Arc::as_ptr(&self.collection))
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            ["kind", "mode", "count", "bindings", "items"]
                .into_iter()
                .map(|name| (name.to_owned(), self.field(name).unwrap_or_default()))
                .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        Some(match name {
            "kind" => atom_items("collection"),
            "mode" => atom_items(collection_mode_name(self.collection.mode)),
            "count" => vec![Item::Atomic(AtomValue::Integer(
                i64::try_from(self.collection.items.len()).unwrap_or(i64::MAX),
            ))],
            "bindings" => string_map_record_items(&self.collection.bindings),
            "items" => vec![Item::Array(
                self.collection
                    .items
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        Item::native(TransformCollectionItemQueryView {
                            collection: Arc::clone(&self.collection),
                            index,
                        })
                    })
                    .collect(),
            )],
            _ => Vec::new(),
        })
    }
}

#[derive(Debug, Clone)]
struct TransformCollectionItemQueryView {
    collection: Arc<TransformArtifactCollection>,
    index: usize,
}

impl QueryItemView for TransformCollectionItemQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.transform-collection-item-view"
    }

    fn identity(&self) -> String {
        format!("{:p}:item:{}", Arc::as_ptr(&self.collection), self.index)
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            [
                "inputName",
                "input",
                "artifactId",
                "uri",
                "destination",
                "target",
                "bindings",
                "artifact",
                "primary",
                "sourceMap",
                "outputSpans",
            ]
            .into_iter()
            .map(|name| (name.to_owned(), self.field(name).unwrap_or_default()))
            .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let item = self.collection.items.get(self.index)?;
        Some(match name {
            "inputName" | "input" => atom_items(item.input_name.clone()),
            "artifactId" => atom_items(item.artifact.artifact_id.clone()),
            "uri" => optional_atom_items(item.artifact.uri.as_deref()),
            "destination" => optional_atom_items(item.destination.as_deref()),
            "target" => item
                .target
                .as_ref()
                .map(|_| {
                    vec![Item::native(TransformCollectionTargetQueryView {
                        collection: Arc::clone(&self.collection),
                        item_index: self.index,
                    })]
                })
                .unwrap_or_else(null_items),
            "bindings" => string_map_record_items(&item.bindings),
            "artifact" | "primary" => artifact_query_stream(&item.artifact)
                .map(|stream| stream.items)
                .unwrap_or_default(),
            "sourceMap" => item
                .source_map
                .as_ref()
                .map(|_| {
                    vec![Item::native(TransformCollectionSourceMapQueryView {
                        collection: Arc::clone(&self.collection),
                        item_index: self.index,
                        owner: TransformCollectionSourceMapOwner::Item,
                    })]
                })
                .unwrap_or_else(null_items),
            "outputSpans" => vec![Item::Array(
                item.output_spans
                    .iter()
                    .enumerate()
                    .map(|(span_index, _)| {
                        Item::native(TransformCollectionOutputSpanQueryView {
                            collection: Arc::clone(&self.collection),
                            item_index: self.index,
                            span_index,
                        })
                    })
                    .collect(),
            )],
            _ => Vec::new(),
        })
    }

    fn source_map(&self) -> Option<SourceMapStack> {
        self.collection
            .items
            .get(self.index)
            .and_then(|item| item.source_map.clone())
    }
}

#[derive(Debug, Clone)]
struct TransformCollectionTargetQueryView {
    collection: Arc<TransformArtifactCollection>,
    item_index: usize,
}

impl TransformCollectionTargetQueryView {
    fn target(&self) -> Option<&FormatIdentity> {
        self.collection.items.get(self.item_index)?.target.as_ref()
    }
}

impl QueryItemView for TransformCollectionTargetQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.transform-format-identity-view"
    }

    fn identity(&self) -> String {
        format!(
            "{:p}:item:{}:target",
            Arc::as_ptr(&self.collection),
            self.item_index
        )
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            [
                "contentType",
                "schema",
                "defaultNamespace",
                "namespaces",
                "baseUri",
            ]
            .into_iter()
            .map(|name| (name.to_owned(), self.field(name).unwrap_or_default()))
            .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let target = self.target()?;
        Some(match name {
            "contentType" => optional_atom_items(target.content_type.as_deref()),
            "schema" => optional_atom_items(target.schema.as_deref()),
            "defaultNamespace" => optional_atom_items(target.default_namespace.as_deref()),
            "namespaces" => string_map_record_items(&target.namespaces),
            "baseUri" => optional_atom_items(target.base_uri.as_deref()),
            _ => Vec::new(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum TransformCollectionSourceMapOwner {
    Item,
    OutputSpan(usize),
}

#[derive(Debug, Clone)]
struct TransformCollectionSourceMapQueryView {
    collection: Arc<TransformArtifactCollection>,
    item_index: usize,
    owner: TransformCollectionSourceMapOwner,
}

impl TransformCollectionSourceMapQueryView {
    fn source_map_ref(&self) -> Option<&SourceMapStack> {
        let item = self.collection.items.get(self.item_index)?;
        match self.owner {
            TransformCollectionSourceMapOwner::Item => item.source_map.as_ref(),
            TransformCollectionSourceMapOwner::OutputSpan(span_index) => {
                item.output_spans.get(span_index).map(|span| &span.origin)
            }
        }
    }
}

impl QueryItemView for TransformCollectionSourceMapQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.transform-source-map-view"
    }

    fn identity(&self) -> String {
        let owner = match self.owner {
            TransformCollectionSourceMapOwner::Item => "item".to_owned(),
            TransformCollectionSourceMapOwner::OutputSpan(index) => {
                format!("output-span:{index}")
            }
        };
        format!(
            "{:p}:item:{}:source-map:{owner}",
            Arc::as_ptr(&self.collection),
            self.item_index
        )
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            ["kind", "frameCount"]
                .into_iter()
                .map(|name| (name.to_owned(), self.field(name).unwrap_or_default()))
                .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let source_map = self.source_map_ref()?;
        Some(match name {
            "kind" => atom_items("source-map"),
            "frameCount" => integer_items(source_map.frames.len() as u64),
            _ => Vec::new(),
        })
    }

    fn source_map(&self) -> Option<SourceMapStack> {
        self.source_map_ref().cloned()
    }
}

#[derive(Debug, Clone)]
struct TransformCollectionOutputSpanQueryView {
    collection: Arc<TransformArtifactCollection>,
    item_index: usize,
    span_index: usize,
}

impl TransformCollectionOutputSpanQueryView {
    fn output_span(&self) -> Option<&OutputSpan> {
        self.collection
            .items
            .get(self.item_index)?
            .output_spans
            .get(self.span_index)
    }
}

impl QueryItemView for TransformCollectionOutputSpanQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.transform-output-span-view"
    }

    fn identity(&self) -> String {
        format!(
            "{:p}:item:{}:output-span:{}",
            Arc::as_ptr(&self.collection),
            self.item_index,
            self.span_index
        )
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        Some(
            ["outputRange", "origin"]
                .into_iter()
                .map(|name| (name.to_owned(), self.field(name).unwrap_or_default()))
                .collect(),
        )
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let span = self.output_span()?;
        Some(match name {
            "outputRange" => vec![Item::Record(BTreeMap::from([
                ("start".to_owned(), integer_items(span.output_range.start)),
                (
                    "length".to_owned(),
                    integer_items(u64::from(span.output_range.len)),
                ),
                ("end".to_owned(), integer_items(span.output_range.end())),
            ]))],
            "origin" => vec![Item::native(TransformCollectionSourceMapQueryView {
                collection: Arc::clone(&self.collection),
                item_index: self.item_index,
                owner: TransformCollectionSourceMapOwner::OutputSpan(self.span_index),
            })],
            _ => Vec::new(),
        })
    }

    fn source_map(&self) -> Option<SourceMapStack> {
        self.output_span().map(|span| span.origin.clone())
    }
}

fn collection_mode_name(mode: TransformArtifactCollectionMode) -> &'static str {
    mode.as_str()
}

fn atom_items(value: impl Into<String>) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::String(value.into()))]
}

fn optional_atom_items(value: Option<&str>) -> Vec<Item> {
    vec![value
        .map(|value| Item::Atomic(AtomValue::String(value.to_owned())))
        .unwrap_or(Item::Atomic(AtomValue::Null))]
}

fn null_items() -> Vec<Item> {
    vec![Item::Atomic(AtomValue::Null)]
}

fn integer_items(value: u64) -> Vec<Item> {
    vec![Item::Atomic(AtomValue::Integer(
        i64::try_from(value).unwrap_or(i64::MAX),
    ))]
}

fn string_map_record_items(values: &BTreeMap<String, String>) -> Vec<Item> {
    vec![Item::Record(
        values
            .iter()
            .map(|(name, value)| (name.clone(), atom_items(value.clone())))
            .collect(),
    )]
}

#[derive(Debug, Clone)]
struct EncodedTextQueryView {
    encoded: Arc<TransformEncodedArtifact>,
}

impl QueryItemView for EncodedTextQueryView {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "cem.ql.encoded-text-view"
    }

    fn identity(&self) -> String {
        format!("{:p}:text", Arc::as_ptr(&self.encoded))
    }

    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Atomic
    }

    fn atom(&self) -> Option<AtomValue> {
        std::str::from_utf8(self.encoded.bytes.as_ref())
            .ok()
            .map(|value| AtomValue::String(value.to_owned()))
    }
}

fn artifact_query_stream(artifact: &TransformTemplateDataArtifact) -> Result<ItemStream, String> {
    match &artifact.body {
        TransformArtifactBody::CemDocument(document) => document
            .root()
            .map(|_| ItemStream::once(CemDocumentQueryView::item(Arc::clone(document), 0)))
            .ok_or_else(|| "native CEM transform artifact has no document root".to_owned()),
        TransformArtifactBody::Lifecycle(owner) => lifecycle_query_stream(Arc::clone(owner)),
        TransformArtifactBody::Collection(collection) => Ok(ItemStream::once(Item::native(
            TransformCollectionQueryView {
                collection: Arc::clone(collection),
            },
        ))),
        TransformArtifactBody::Encoded(encoded) if encoded.encoding == TransformEncoding::Json => {
            let content_type =
                encoded.identity.content_type.as_deref().ok_or_else(|| {
                    "explicit JSON transform artifact has no content type".to_owned()
                })?;
            let source_uri = artifact
                .uri
                .as_deref()
                .unwrap_or("memory:transform-input.json");
            let (document, diagnostics) =
                json_document_ast_from_source_bytes(JsonSourceValidationRequest {
                    bytes: encoded.bytes.as_ref(),
                    source_uri,
                    content_type: Some(content_type),
                });
            let document = document.ok_or_else(|| {
                let messages = diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("explicit JSON transform artifact could not enter its lifecycle AST: {messages}")
            })?;
            lifecycle_query_stream(Arc::new(LoadedInputAstStream::JsonDocument(document)))
        }
        TransformArtifactBody::Encoded(encoded) if encoded.encoding == TransformEncoding::Text => {
            std::str::from_utf8(encoded.bytes.as_ref()).map_err(|error| {
                format!("encoded text transform artifact is not valid UTF-8: {error}")
            })?;
            Ok(ItemStream::once(Item::native(EncodedTextQueryView {
                encoded: Arc::clone(encoded),
            })))
        }
        TransformArtifactBody::Extension(native)
            if native.representation_id() == CEM_QL_RESULT_REPRESENTATION_ID =>
        {
            native
                .as_any()
                .downcast_ref::<CemQlResultArtifact>()
                .map(|result| result.stream.clone())
                .ok_or_else(|| {
                    "CEM-QL result body type does not match its representation".to_owned()
                })
        }
        body => Err(format!(
            "transform artifact representation `{}` has no CEM-QL native query view",
            body.representation_id()
        )),
    }
}

fn lifecycle_query_stream(owner: Arc<LoadedInputAstStream>) -> Result<ItemStream, String> {
    match owner.as_ref() {
        LoadedInputAstStream::JsonDocument(_) => {
            let item = JsonValueQueryView::root(owner)
                .ok_or_else(|| "JSON lifecycle AST has no root value".to_owned())?;
            Ok(item
                .members()
                .map(ItemStream::from_items)
                .unwrap_or_else(|| ItemStream::once(item)))
        }
        LoadedInputAstStream::XmlDocument(_) => {
            Ok(ItemStream::once(Item::native(XmlDocumentQueryView {
                owner,
            })))
        }
        stream => Err(format!(
            "lifecycle representation `{}` has no CEM-QL native query view",
            lifecycle_representation_name(stream)
        )),
    }
}

fn lifecycle_representation_name(stream: &LoadedInputAstStream) -> &'static str {
    match stream {
        LoadedInputAstStream::HtmlDocument(_) => "html-document",
        LoadedInputAstStream::CssDocument(_) => "css-document",
        LoadedInputAstStream::CssSelectorExpression(_) => "css-selector-expression",
        LoadedInputAstStream::CsvDocument(_) => "csv-document",
        LoadedInputAstStream::YamlDocument(_) => "yaml-document",
        LoadedInputAstStream::JsonDocument(_) => "json-document",
        LoadedInputAstStream::JsonSchemaDocument(_) => "json-schema-document",
        LoadedInputAstStream::MarkdownDocument(_) => "markdown-document",
        LoadedInputAstStream::XmlDocument(_) => "xml-document",
        LoadedInputAstStream::XhtmlDocument(_) => "xhtml-document",
        LoadedInputAstStream::SvgDocument(_) => "svg-document",
        LoadedInputAstStream::MathMlDocument(_) => "mathml-document",
        LoadedInputAstStream::XPathExpression(_) => "xpath-expression",
        LoadedInputAstStream::XsltStylesheet(_) => "xslt-stylesheet",
        LoadedInputAstStream::RelaxNgDocument(_) => "relax-ng-document",
    }
}

#[derive(Debug, Clone)]
pub struct CemQlQueryAstOwner {
    compiled: CompiledExpression,
    identity: FormatIdentity,
    source_uri: String,
    source_map: SourceMapStack,
}

impl CemQlQueryAstOwner {
    pub fn from_source_bytes(
        bytes: &[u8],
        source_uri: &str,
        identity: FormatIdentity,
        resolver_policy_stamp: &str,
    ) -> Result<Self, Vec<Diagnostic>> {
        let source = std::str::from_utf8(bytes).map_err(|error| {
            vec![cem_ql_query_diagnostic(
                source_uri,
                "cem.ql.query_invalid_utf8",
                format!("CEM-QL query source is not valid UTF-8: {error}"),
            )]
        })?;
        let context = StandaloneExpressionContext {
            source_uri: Some(source_uri.to_owned()),
            resolver_policy_stamp: Some(resolver_policy_stamp.to_owned()),
            host_capability_profile: Some("cem-ml-query".to_owned()),
            ..StandaloneExpressionContext::default()
        }
        .with_input(ItemStream::empty(), Type::Any);
        let compiled = compile_expression(source, &context).map_err(|error| {
            let diagnostics = diagnostics_with_uri(&error.diagnostics, source_uri);
            if diagnostics.is_empty() {
                vec![cem_ql_query_diagnostic(
                    source_uri,
                    error.code,
                    error.message,
                )]
            } else {
                diagnostics
            }
        })?;
        let source_map = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(
                    0,
                    u32::try_from(bytes.len()).unwrap_or(u32::MAX),
                )),
                transform: TransformKind::ContentTypeTransform {
                    content_type: identity
                        .content_type
                        .clone()
                        .unwrap_or_else(|| "application/octet-stream".to_owned()),
                },
            }],
        };
        Ok(Self {
            compiled,
            identity,
            source_uri: source_uri.to_owned(),
            source_map,
        })
    }
}

impl QueryNativeArtifact for CemQlQueryAstOwner {
    fn representation_id(&self) -> &'static str {
        CEM_QL_QUERY_AST_REPRESENTATION_ID
    }

    fn source_map(&self) -> Option<&SourceMapStack> {
        Some(&self.source_map)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl QueryAstOwner for CemQlQueryAstOwner {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::CemQl
    }

    fn identity(&self) -> &FormatIdentity {
        &self.identity
    }

    fn source_uri(&self) -> &str {
        &self.source_uri
    }
}

#[derive(Debug, Clone)]
pub struct CemQlNativeItemsOwner {
    owner: Arc<LoadedInputAstStream>,
    identity: FormatIdentity,
    stream: ItemStream,
    source_map: SourceMapStack,
}

impl CemQlNativeItemsOwner {
    pub fn from_lifecycle(
        owner: Arc<LoadedInputAstStream>,
        identity: FormatIdentity,
    ) -> Result<Self, String> {
        let stream = lifecycle_query_stream(Arc::clone(&owner))?;
        let source_map = stream
            .items
            .first()
            .and_then(Item::source_map)
            .unwrap_or_else(|| SourceMapStack {
                frames: vec![SourceMapFrame {
                    source_id: SourceId(1),
                    span: FrameSpan::Single(ByteRange::new(0, 0)),
                    transform: TransformKind::ContentTypeTransform {
                        content_type: identity
                            .content_type
                            .clone()
                            .unwrap_or_else(|| "application/octet-stream".to_owned()),
                    },
                }],
            });
        Ok(Self {
            owner,
            identity,
            stream,
            source_map,
        })
    }

    pub fn stream(&self) -> &ItemStream {
        &self.stream
    }

    pub fn lifecycle_owner(&self) -> &Arc<LoadedInputAstStream> {
        &self.owner
    }
}

impl QueryNativeArtifact for CemQlNativeItemsOwner {
    fn representation_id(&self) -> &'static str {
        CEM_QL_QUERY_INPUT_REPRESENTATION_ID
    }

    fn source_map(&self) -> Option<&SourceMapStack> {
        Some(&self.source_map)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl QueryInputOwner for CemQlNativeItemsOwner {
    fn identity(&self) -> &FormatIdentity {
        &self.identity
    }

    fn input_models(&self) -> &[QueryInputModel] {
        CEM_QL_QUERY_INPUT_MODELS
    }
}

#[derive(Debug, Clone)]
pub struct CemQlQueryResultArtifact {
    stream: ItemStream,
    source_map: SourceMapStack,
}

impl CemQlQueryResultArtifact {
    pub fn stream(&self) -> &ItemStream {
        &self.stream
    }
}

impl QueryNativeArtifact for CemQlQueryResultArtifact {
    fn representation_id(&self) -> &'static str {
        CEM_QL_QUERY_RESULT_REPRESENTATION_ID
    }

    fn source_map(&self) -> Option<&SourceMapStack> {
        Some(&self.source_map)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl QueryNativeResult for CemQlQueryResultArtifact {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::CemQl
    }
}

#[derive(Debug, Clone, Default)]
pub struct CemQlQueryEvaluator;

impl QueryEvaluatorAdapter for CemQlQueryEvaluator {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::CemQl
    }

    fn evaluate(
        &self,
        request: QueryExecutionRequest<'_>,
    ) -> Result<QueryExecutionResult, Vec<Diagnostic>> {
        request.validate_contract().map_err(|error| {
            vec![cem_ql_query_diagnostic(
                request.query_ast_owner.source_uri(),
                "cem.ql.query_contract_invalid",
                error.to_string(),
            )]
        })?;
        let query = request
            .query_ast_owner
            .as_any()
            .downcast_ref::<CemQlQueryAstOwner>()
            .ok_or_else(|| {
                vec![cem_ql_query_diagnostic(
                    request.query_ast_owner.source_uri(),
                    "cem.ql.query_ast_unsupported",
                    "CEM-QL query evaluator requires the package-owned compiled expression",
                )]
            })?;
        let input = request
            .input_ast_owner
            .as_any()
            .downcast_ref::<CemQlNativeItemsOwner>()
            .ok_or_else(|| {
                vec![cem_ql_query_diagnostic(
                    query.source_uri(),
                    "cem.ql.query_input_unsupported",
                    "CEM-QL query evaluator requires a lifecycle-owned native item stream",
                )]
            })?;
        if !request.bindings.is_empty() {
            return Err(vec![cem_ql_query_diagnostic(
                query.source_uri(),
                "cem.ql.query_binding_unsupported",
                "CEM-QL query host bindings require package-owned typed item artifacts",
            )]);
        }
        let mut stream = evaluate_with_abort(
            &query.compiled.query,
            &EvaluationContext {
                scope: QueryContextScope(0),
                scope_policy: *request.scope_policy,
                diagnostics: Vec::new(),
                policy_bindings: BTreeMap::from([("input".to_owned(), input.stream().clone())]),
                current_item: None,
            },
            request.abort_signal,
        );
        if request.abort_signal.is_aborted() {
            return Err(vec![cem_ml::diagnostics::Diagnostic {
                source_map: request.abort_signal.source_map(),
                ..cem_ql_query_diagnostic(
                    query.source_uri(),
                    "cem.ql.query_cancelled",
                    "CEM-QL query execution was cancelled by the host",
                )
            }]);
        }
        let mut diagnostics = diagnostics_with_uri(&stream.diagnostics, query.source_uri());
        if let Some(error) = stream.error.as_ref() {
            diagnostics.push(cem_ql_query_diagnostic(
                query.source_uri(),
                "cem.ql.query_evaluation_failed",
                format!("{error:?}"),
            ));
        }
        if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation())
        {
            return Err(diagnostics);
        }
        let item_limit = match (
            request.limits.max_result_items,
            request.limits.max_work_units,
        ) {
            (Some(results), Some(work)) => Some(results.min(work)),
            (Some(results), None) => Some(results),
            (None, Some(work)) => Some(work),
            (None, None) => None,
        };
        if item_limit.is_some_and(|limit| stream.items.len() as u64 > limit) {
            return Err(vec![cem_ql_query_diagnostic(
                query.source_uri(),
                "cem.ql.query_result_limit_exceeded",
                format!(
                    "CEM-QL query returned {} items, exceeding the configured limit of {}",
                    stream.items.len(),
                    item_limit.expect("item limit guard")
                ),
            )]);
        }
        stream.diagnostics.clear();
        let source_map = query.source_map.clone();
        let native_result: Arc<dyn QueryNativeResult> = Arc::new(CemQlQueryResultArtifact {
            stream,
            source_map: source_map.clone(),
        });
        QueryExecutionResult::new(
            QueryLanguage::CemQl,
            query.identity.clone(),
            Arc::new(input.clone()),
            native_result,
            source_map,
        )
        .map_err(|error| {
            vec![cem_ql_query_diagnostic(
                query.source_uri(),
                "cem.ql.query_contract_invalid",
                error.to_string(),
            )]
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CemQlQueryRuntimeAdapter;

impl QueryRuntimeAdapter for CemQlQueryRuntimeAdapter {
    fn language(&self) -> QueryLanguage {
        QueryLanguage::CemQl
    }

    fn prepare(
        &self,
        request: QueryPreparationRequest<'_>,
    ) -> Result<QueryPreparedOwners, Vec<Diagnostic>> {
        let input =
            CemQlNativeItemsOwner::from_lifecycle(request.lifecycle_owner, request.input_identity)
                .map_err(|message| {
                    vec![cem_ql_query_diagnostic(
                        request.input_uri,
                        "cem.ql.query_input_unsupported",
                        message,
                    )]
                })?;
        let query = CemQlQueryAstOwner::from_source_bytes(
            &request.query.bytes,
            &request.query.uri,
            request.query.identity.clone(),
            request.resolver_policy_stamp,
        )?;
        Ok(QueryPreparedOwners {
            query_ast_owner: Arc::new(query),
            input_ast_owner: Arc::new(input),
            diagnostics: Vec::new(),
        })
    }

    fn evaluate(
        &self,
        request: QueryExecutionRequest<'_>,
    ) -> Result<QueryExecutionResult, Vec<Diagnostic>> {
        CemQlQueryEvaluator.evaluate(request)
    }
}

fn cem_ql_query_diagnostic(
    uri: &str,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic {
        uri: Some(uri.to_owned()),
        code: code.into(),
        severity: Severity::Fatal,
        message: message.into(),
        ..Diagnostic::default()
    }
}

#[derive(Debug, Clone, Copy)]
struct CemQlQueryResultExporter {
    format: QueryExportFormat,
}

impl QueryResultExporter for CemQlQueryResultExporter {
    fn id(&self) -> &'static str {
        match self.format {
            QueryExportFormat::Terminal => "cem-ql.query-result-terminal",
            QueryExportFormat::Cem => "cem-ql.query-result-cem",
            QueryExportFormat::Json => "cem-ql.query-result-json",
        }
    }

    fn language(&self) -> QueryLanguage {
        QueryLanguage::CemQl
    }

    fn format(&self) -> QueryExportFormat {
        self.format
    }

    fn export(&self, request: QueryExportRequest<'_>) -> Result<QueryEncodedOutput, String> {
        let result = request
            .result
            .native_result
            .as_any()
            .downcast_ref::<CemQlQueryResultArtifact>()
            .ok_or_else(|| "CEM-QL exporter requires the native item sequence".to_owned())?;
        let (content_type, bytes) = match self.format {
            QueryExportFormat::Terminal => {
                let mut text = format!(
                    "CEM-QL: {} item{}\n",
                    result.stream.items.len(),
                    if result.stream.items.len() == 1 {
                        ""
                    } else {
                        "s"
                    }
                );
                for item in &result.stream.items {
                    text.push_str(
                        &serde_json::to_string(&item_json(item))
                            .map_err(|error| error.to_string())?,
                    );
                    text.push('\n');
                }
                ("text/plain; charset=utf-8".to_owned(), text.into_bytes())
            }
            QueryExportFormat::Cem => {
                let mut text = format!(
                    "{{query-result @language=\"cem-ql\" @count={} |\n",
                    result.stream.items.len()
                );
                for item in &result.stream.items {
                    text.push_str("  ");
                    text.push_str(&cem_ql_cem_item(item));
                    text.push('\n');
                }
                text.push_str("}\n");
                ("text/cem-ml; charset=utf-8".to_owned(), text.into_bytes())
            }
            QueryExportFormat::Json => {
                let body = json!({
                    "language": "cem-ql",
                    "result": item_stream_json(&result.stream),
                });
                (
                    "application/vnd.cem.query-result+cem-ql".to_owned(),
                    serde_json::to_vec_pretty(&body).map_err(|error| error.to_string())?,
                )
            }
        };
        Ok(QueryEncodedOutput {
            content_type,
            bytes,
        })
    }
}

pub fn register_cem_ql_query_exporters(registry: &mut QueryResultExporterRegistry) {
    for format in [
        QueryExportFormat::Terminal,
        QueryExportFormat::Cem,
        QueryExportFormat::Json,
    ] {
        registry.register(CemQlQueryResultExporter { format });
    }
}

fn cem_ql_cem_item(item: &Item) -> String {
    match item {
        Item::Node(id) => format!("{{item @kind=\"node\" @id={id}}}"),
        Item::Atomic(atom) => format!(
            "{{item @kind=\"atomic\" @value=\"{}\"}}",
            cem_ql_query_attribute_escape(&atom_json(atom).to_string())
        ),
        Item::Record(fields) => {
            format!("{{item @kind=\"record\" @fields={}}}", fields.len())
        }
        Item::Array(items) => format!("{{item @kind=\"array\" @items={}}}", items.len()),
        Item::Native(view) => format!(
            "{{item @kind=\"native\" @representation=\"{}\" @identity=\"{}\"}}",
            cem_ql_query_attribute_escape(view.representation_id()),
            cem_ql_query_attribute_escape(&view.identity())
        ),
        Item::Lambda(id) => format!("{{item @kind=\"lambda\" @id=\"{id:?}\"}}"),
        Item::Resource(resource) => format!(
            "{{item @kind=\"resource\" @id=\"{}\"}}",
            cem_ql_query_attribute_escape(&resource.id)
        ),
    }
}

fn cem_ql_query_attribute_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn template_data_from_artifacts(
    primary: &TransformTemplateDataArtifact,
    secondary: &BTreeMap<String, Arc<TransformTemplateDataArtifact>>,
) -> Result<TemplateData, String> {
    let primary_stream = artifact_query_stream(primary)?;
    let mut data = TemplateData::default().with_binding("input", primary_stream.clone());
    if let [item] = primary_stream.items.as_slice() {
        if let Some(fields) = item.view().and_then(|view| view.fields()) {
            for (name, items) in fields {
                data = data.with_binding(name, ItemStream::from_items(items));
            }
        } else if let Item::Record(fields) = item {
            for (name, items) in fields {
                data = data.with_binding(name.clone(), ItemStream::from_items(items.clone()));
            }
        } else {
            data = data.with_binding("value", primary_stream.clone());
        }
    } else {
        data = data.with_binding("value", primary_stream.clone());
    }
    for (name, artifact) in secondary {
        data = data.with_binding(name.clone(), artifact_query_stream(artifact)?);
    }
    Ok(data)
}

fn expression_compile_context(
    template_uri: &str,
    params: &TransformTemplateParameterArena,
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
    for (name, _) in params.iter() {
        context = context.with_binding(
            name.to_owned(),
            StandaloneExpressionBinding::new(ItemStream::empty(), Type::Any),
        );
    }
    context
}

fn expression_policy_bindings(
    primary: &TransformTemplateDataArtifact,
    params: &TransformTemplateParameterArena,
) -> Result<BTreeMap<String, ItemStream>, String> {
    let mut bindings = BTreeMap::new();
    bindings.insert("input".to_owned(), artifact_query_stream(primary)?);
    for (name, value) in params.iter() {
        bindings.insert(name.to_owned(), evaluator_param_value_to_stream(value)?);
    }
    Ok(bindings)
}

fn evaluator_param_value_to_stream(value: &CemtEvaluatorValue<'_>) -> Result<ItemStream, String> {
    if value.kind() == CemtEvaluatorValueKind::Sequence {
        return (0..value.length().map_err(|error| error.to_string())?)
            .map(|index| {
                value
                    .item(index)
                    .ok_or_else(|| format!("typed parameter sequence item {index} is unavailable"))
                    .and_then(|value| evaluator_param_value_to_item(&value))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(ItemStream::from_items);
    }

    evaluator_param_value_to_item(value).map(ItemStream::once)
}

fn evaluator_param_value_to_item(value: &CemtEvaluatorValue<'_>) -> Result<Item, String> {
    match value.kind() {
        CemtEvaluatorValueKind::Null => Ok(Item::Atomic(AtomValue::Null)),
        CemtEvaluatorValueKind::Boolean => value
            .as_bool()
            .map(|value| Item::Atomic(AtomValue::Boolean(value)))
            .ok_or_else(|| "typed boolean parameter value is unavailable".to_owned()),
        CemtEvaluatorValueKind::Number => value
            .as_number()
            .map(evaluator_param_number_to_item)
            .ok_or_else(|| "typed numeric parameter value is unavailable".to_owned()),
        CemtEvaluatorValueKind::String => value
            .as_str()
            .map(|value| Item::Atomic(AtomValue::String(value.to_owned())))
            .ok_or_else(|| "typed string parameter value is unavailable".to_owned()),
        CemtEvaluatorValueKind::Sequence => (0..value
            .length()
            .map_err(|error| error.to_string())?)
            .map(|index| {
                value
                    .item(index)
                    .ok_or_else(|| format!("typed parameter sequence item {index} is unavailable"))
                    .and_then(|value| evaluator_param_value_to_item(&value))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Item::Array),
        CemtEvaluatorValueKind::Record => {
            let record = value
                .owned_record()
                .ok_or_else(|| "typed parameter record is not owned by its arena".to_owned())?;
            record
                .field_names()
                .into_iter()
                .map(|name| {
                    value
                        .field(&name)
                        .ok_or_else(|| {
                            format!("typed parameter record field `{name}` is unavailable")
                        })
                        .and_then(|value| evaluator_param_value_to_item(&value))
                        .map(|value| (name, vec![value]))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map(Item::Record)
        }
        kind => Err(format!(
            "typed parameter arena cannot expose `{}` as a CEM-QL item",
            kind.as_str()
        )),
    }
}

fn evaluator_param_number_to_item(value: CemtEvaluatorNumber) -> Item {
    if let Some(value) = value.as_i64() {
        Item::Atomic(AtomValue::Integer(value))
    } else if let Some(value) = value.as_u64().and_then(|value| i64::try_from(value).ok()) {
        Item::Atomic(AtomValue::Integer(value))
    } else {
        Item::Atomic(AtomValue::Double(value.as_f64()))
    }
}

fn explicit_param_value_to_stream(value: &Value) -> ItemStream {
    match value {
        Value::Array(items) => {
            ItemStream::from_items(items.iter().map(explicit_param_value_to_item).collect())
        }
        _ => ItemStream::once(explicit_param_value_to_item(value)),
    }
}

fn explicit_param_value_to_item(value: &Value) -> Item {
    match value {
        Value::Null => Item::Atomic(AtomValue::Null),
        Value::Bool(value) => Item::Atomic(AtomValue::Boolean(*value)),
        Value::Number(value) => explicit_param_number_to_item(value),
        Value::String(value) => Item::Atomic(AtomValue::String(value.clone())),
        Value::Array(items) => {
            Item::Array(items.iter().map(explicit_param_value_to_item).collect())
        }
        Value::Object(fields) => Item::Record(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), vec![explicit_param_value_to_item(value)]))
                .collect(),
        ),
    }
}

fn explicit_param_number_to_item(value: &Number) -> Item {
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
        Item::Native(view) => {
            let mut value = if let Some(atom) = view.atom() {
                atom_json(&atom)
            } else if let Some(items) = view.members() {
                json!({
                    "kind": "array",
                    "items": items.iter().map(item_json).collect::<Vec<_>>(),
                })
            } else if let Some(fields) = view.fields() {
                let fields = fields
                    .into_iter()
                    .map(|(name, items)| {
                        (
                            name,
                            Value::Array(items.iter().map(item_json).collect::<Vec<_>>()),
                        )
                    })
                    .collect::<Map<_, _>>();
                json!({
                    "kind": "record",
                    "fields": fields,
                })
            } else {
                json!({
                    "kind": "native",
                })
            };
            let object = value
                .as_object_mut()
                .expect("native item JSON projections are objects");
            object.insert("identity".to_owned(), Value::String(view.identity()));
            object.insert(
                "representation".to_owned(),
                Value::String(view.representation_id().to_owned()),
            );
            object.insert("sourceMap".to_owned(), json!(view.source_map()));
            value
        }
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
        EvalError::Cancelled => json!({
            "kind": "eval",
            "type": "cancelled",
            "message": "evaluation cancelled by the host",
        }),
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
        CEM_QL_CONTENT_TYPE, CEM_QL_EXPRESSION_CONTENT_TYPE, CEM_QL_EXPRESSION_SCHEMA_URI,
        CEM_QL_SCHEMA_URI, CEM_SCHEMA_PACKAGE_CONTENT_TYPE, CEM_SCHEMA_PACKAGE_URI,
        CEM_TRANSFORM_CONTENT_TYPE, HTML_CONTENT_TYPE, HTML_SCHEMA_URI, XML_CONTENT_TYPE,
        XML_SCHEMA_URI,
    };
    use cem_ml::source::{BytesSource, SourceId};
    use cem_ml::tokenizer::{cem::CemTokenizer, xml::XmlTokenizer};
    use cem_ml::transform_artifact::TransformArtifactCollectionItem;
    use cem_ml::transform_template::{
        TransformTemplateAdapterLookup, TransformTemplateModuleParamType,
        TransformTemplateModulePreflight, TransformTemplateResolvedModule,
    };
    use cem_ml::validation::xml::{xml_document_ast_from_source_bytes, XmlSourceValidationRequest};

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

    fn has_diagnostic_code(diagnostics: &[Diagnostic], code: &str) -> bool {
        diagnostics.iter().any(|diagnostic| diagnostic.code == code)
    }

    fn test_output_value(output: &TransformTemplateOutputArtifact) -> Value {
        match &output.body {
            TransformArtifactBody::Encoded(encoded)
                if encoded.encoding == TransformEncoding::Text =>
            {
                Value::String(
                    output
                        .encoded_text_value()
                        .expect("encoded text output")
                        .to_owned(),
                )
            }
            TransformArtifactBody::Encoded(encoded)
                if encoded.encoding == TransformEncoding::Json =>
            {
                output.explicit_json_value().expect("explicit JSON output")
            }
            TransformArtifactBody::CemTree(_) => {
                output.cemt_subject().expect("typed CEM-tree output")
            }
            TransformArtifactBody::Extension(native) => native
                .as_any()
                .downcast_ref::<CemQlResultArtifact>()
                .map(|result| item_stream_json(&result.stream))
                .expect("CEM-QL native result output"),
            body => panic!(
                "test output helper does not support `{}`",
                body.representation_id()
            ),
        }
    }

    #[test]
    fn common_query_runner_executes_registered_cem_ql_runtime_with_native_owners() {
        let input_identity = FormatIdentity {
            content_type: Some(XML_CONTENT_TYPE.to_owned()),
            schema: Some(XML_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let response = cem_ml::query::run_query(cem_ml::query::QueryRunRequest {
            data: EngineInput {
                uri: "memory:catalog.xml".to_owned(),
                bytes: b"<catalog><book id=\"a\"/><book id=\"b\"/></catalog>".to_vec(),
                from_format: Some(InputFormat::Xml),
                identity: Some(input_identity.clone()),
                root_scope: ScopeConfig {
                    default_content_type: input_identity.content_type.clone(),
                    schema: input_identity.schema.clone(),
                    ..ScopeConfig::default()
                },
            },
            query: cem_ml::query::QuerySource {
                uri: "memory:query.cem-ql".to_owned(),
                bytes: b"input".to_vec(),
                identity: FormatIdentity {
                    content_type: Some(CEM_QL_EXPRESSION_CONTENT_TYPE.to_owned()),
                    schema: Some(CEM_QL_EXPRESSION_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                },
            },
            context: engine_context_with_cem_ql_template_adapter(),
            context_item: None,
            bindings: Default::default(),
            limits: None,
        })
        .expect("registered CEM-QL query runtime succeeds");

        assert_eq!(response.language, QueryLanguage::CemQl);
        let result = response
            .result
            .native_result
            .as_any()
            .downcast_ref::<CemQlQueryResultArtifact>()
            .expect("CEM-QL native result remains adapter-owned");
        assert_eq!(result.stream().items.len(), 1);
        assert!(!response.result.source_map.frames.is_empty());
    }

    #[test]
    fn cem_ql_source_validator_accepts_module_source() {
        let diagnostics = validate_cem_ql_source_bytes(CemQlSourceValidationRequest {
            bytes: br#"module "https://example.test/queries/main"

declare let count = 2

count + 1"#,
            source_uri: "fixture.cemql",
            content_type: Some(CEM_QL_CONTENT_TYPE),
            schema: Some(CEM_QL_SCHEMA_URI),
        });

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cem_ql_source_validator_reports_expression_parse_error_without_module_uri() {
        let diagnostics = validate_cem_ql_source_bytes(CemQlSourceValidationRequest {
            bytes: b"input +",
            source_uri: "fixture.cemql",
            content_type: Some(cem_ml::schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE),
            schema: Some(cem_ml::schema::registry::CEM_QL_EXPRESSION_SCHEMA_URI),
        });

        assert!(has_diagnostic_code(&diagnostics, "cem.ql.parse_error"));
        assert!(!has_diagnostic_code(
            &diagnostics,
            "cem.ql.module_uri_missing"
        ));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.uri.as_deref() == Some("fixture.cemql")));
    }

    #[test]
    fn cem_ql_source_validator_reports_invalid_utf8() {
        let diagnostics = validate_cem_ql_source_bytes(CemQlSourceValidationRequest {
            bytes: b"module \"https://example.test/queries/invalid\"\n\n\xff",
            source_uri: "fixture.cemql",
            content_type: Some(CEM_QL_CONTENT_TYPE),
            schema: Some(CEM_QL_SCHEMA_URI),
        });

        assert!(has_diagnostic_code(&diagnostics, "cem.ql.invalid_utf8"));
    }

    #[test]
    fn template_embedding_validator_reports_generic_broken_embeddings() {
        let diagnostics = validate_cem_ql_template_embedding_source_bytes(
            CemQlTemplateEmbeddingValidationRequest {
                bytes: b"{p | {$ 1 + }}",
                from_format: InputFormat::Cem,
                source_uri: Some("broken.cem"),
                identity: CemQlTemplateEmbeddingIdentity::default(),
            },
        );

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.starts_with("cem.ql.")));
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.uri.as_deref() == Some("broken.cem")));
    }

    #[test]
    fn template_embedding_validator_skips_html_inputs() {
        let diagnostics = validate_cem_ql_template_embedding_source_bytes(
            CemQlTemplateEmbeddingValidationRequest {
                bytes: b"<p>Hello {{ not-cem-ql }}</p>",
                from_format: InputFormat::Html,
                source_uri: Some("page.html"),
                identity: CemQlTemplateEmbeddingIdentity::default(),
            },
        );

        assert!(
            diagnostics.is_empty(),
            "HTML inputs produce no cem-ql template diagnostics; got {diagnostics:?}"
        );
    }

    #[test]
    fn template_embedding_validator_leaves_xslt_avts_to_the_xslt_ast() {
        let source = br#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/"><card class="item-{$label}"/></xsl:template></xsl:stylesheet>"#;

        for identity in [
            CemQlTemplateEmbeddingIdentity {
                content_type: Some("application/xslt+xml; charset=UTF-8"),
                schema: None,
            },
            CemQlTemplateEmbeddingIdentity {
                content_type: Some(CEM_ML_CONTENT_TYPE),
                schema: Some(XSLT_SCHEMA_URI),
            },
            CemQlTemplateEmbeddingIdentity {
                content_type: Some("custom-element-xslt"),
                schema: None,
            },
        ] {
            let diagnostics = validate_cem_ql_template_embedding_source_bytes(
                CemQlTemplateEmbeddingValidationRequest {
                    bytes: source,
                    from_format: InputFormat::Cem,
                    source_uri: Some("stylesheet.xsl"),
                    identity,
                },
            );
            assert!(
                diagnostics.is_empty(),
                "XSLT AVTs belong to the native XSLT stream: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn template_embedding_validator_leaves_schema_behaviors_to_schema_evaluator() {
        let diagnostics = validate_cem_ql_template_embedding_source_bytes(
            CemQlTemplateEmbeddingValidationRequest {
                bytes: br#"{schema |
                {behaviors |
                    {behavior @select="resource" @match='kind = "page"' |
                        {function @name="result" @returns="object" |
                            {body | {$ { message: "Page", details: { kind: $kind } } }}
                        }
                    }
                }
            }"#,
                from_format: InputFormat::Cem,
                source_uri: Some("schema.cem"),
                identity: CemQlTemplateEmbeddingIdentity {
                    content_type: Some(cem_ml::schema::registry::CEM_SCHEMA_CONTENT_TYPE),
                    schema: Some(cem_ml::schema::registry::CEM_SCHEMA_URI),
                },
            },
        );

        assert!(
            diagnostics.is_empty(),
            "schema behavior CEM-QL surfaces should be validated by the schema evaluator: {diagnostics:?}"
        );
    }

    #[test]
    fn template_embedding_validator_accepts_transform_context_bindings() {
        let diagnostics = validate_cem_ql_template_embedding_source_bytes(
            CemQlTemplateEmbeddingValidationRequest {
                bytes: include_bytes!(
                    "../../cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt"
                ),
                from_format: InputFormat::Cem,
                source_uri: Some("dom-to-html.cemt"),
                identity: CemQlTemplateEmbeddingIdentity {
                    content_type: Some(CEM_TRANSFORM_CONTENT_TYPE),
                    schema: Some(cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI),
                },
            },
        );

        assert!(
            diagnostics.is_empty(),
            "context-aware CEMT validation should accept loop/call bindings: {diagnostics:?}"
        );
    }

    #[test]
    fn template_embedding_validator_accepts_cemt_runtime_function_bodies() {
        let diagnostics = validate_cem_ql_template_embedding_source_bytes(
            CemQlTemplateEmbeddingValidationRequest {
                bytes: include_bytes!(
                    "../../cem_ml/schema-packages/cem-transform/v1/examples/function-declarations.cemt"
                ),
                from_format: InputFormat::Cem,
                source_uri: Some("function-declarations.cemt"),
                identity: CemQlTemplateEmbeddingIdentity {
                    content_type: Some(CEM_TRANSFORM_CONTENT_TYPE),
                    schema: Some(cem_ml::schema::registry::CEM_TRANSFORM_SCHEMA_URI),
                },
            },
        );

        assert!(
            diagnostics.is_empty(),
            "CEMT runtime function bodies should not be compiled as CEM-QL render expressions: {diagnostics:?}"
        );
    }

    #[test]
    fn native_template_expression_validator_preserves_slot_source_details() {
        const SOURCE_URI: &str =
            "packages/cem_ml/schema-packages/cem-native-template/v1/examples/invalid-expression-parse.cem";
        let diagnostics = validate_cem_native_template_embedded_expression_source_bytes(
            CemNativeTemplateExpressionValidationRequest {
                bytes: include_bytes!(
                    "../../cem_ml/schema-packages/cem-native-template/v1/examples/invalid-expression-parse.cem"
                ),
                source_uri: SOURCE_URI,
                content_type: Some(cem_ml::schema::registry::CEM_NATIVE_TEMPLATE_CONTENT_TYPE),
                schema: Some(CEM_NATIVE_TEMPLATE_SCHEMA_URI),
            },
        );

        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "cem.ql.use_rust_boolean_ops")
            .unwrap_or_else(|| {
                panic!("missing native-template CEM-QL diagnostic: {diagnostics:?}")
            });
        assert_eq!(diagnostic.uri.as_deref(), Some(SOURCE_URI));
        assert!(diagnostic.line.is_some(), "{diagnostic:?}");
        assert!(diagnostic.column.is_some(), "{diagnostic:?}");
        let details = diagnostic
            .details
            .as_ref()
            .expect("native-template diagnostic carries expression-slot details");
        assert_eq!(details["expressionSlot"]["contract"], "expression-slot");
        assert_eq!(
            details["expressionSlot"]["hostPackage"],
            "cem-native-template/v1"
        );
        assert_eq!(details["expressionSlot"]["slotKind"], "test-attribute");
        assert!(details["expressionSlot"]["expressionRange"]["byteOffset"].is_u64());
    }

    fn packaged_dom_projection_artifact(value: Value) -> TransformTemplateDataArtifact {
        TransformTemplateDataArtifact::explicit_json(
            "dom",
            Some("dom.json".to_owned()),
            FormatIdentity {
                content_type: Some(
                    cem_ml::schema::registry::CEM_DOM_JSON_PROJECTION_CONTENT_TYPE.to_owned(),
                ),
                schema: Some(cem_ml::schema::registry::CEM_DOM_PROJECTION_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            },
            &value,
        )
        .expect("packaged DOM projection has explicit JSON identity")
    }

    fn explicit_json_test_artifact(
        artifact_id: &str,
        uri: Option<&str>,
        value: Value,
    ) -> TransformTemplateDataArtifact {
        TransformTemplateDataArtifact::explicit_json(
            artifact_id,
            uri.map(str::to_owned),
            FormatIdentity {
                content_type: Some(cem_ml::schema::registry::JSON_CONTENT_TYPE.to_owned()),
                schema: Some(cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            },
            &value,
        )
        .expect("test data has explicit JSON identity")
    }

    #[test]
    fn cql_expression_output_stays_native_until_registered_json_export() {
        let adapter = CemQlExpressionTransformTemplateAdapter;
        let template = TemplateInput {
            uri: "result.cem-ql".to_owned(),
            bytes: b"input.title".to_vec(),
            identity: Some(FormatIdentity {
                content_type: Some(
                    cem_ml::schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE.to_owned(),
                ),
                schema: Some(cem_ml::schema::registry::CEM_QL_EXPRESSION_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            root_scope: ScopeConfig::default(),
        };
        let params = TransformTemplateParameterArena::default();
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
                    runtime_phase: TransformRuntimePhase::CemQlExpression,
                    ..TransformExecutionPolicy::default()
                },
            })
            .expect("expression compiles")
            .artifact;
        let primary = explicit_json_test_artifact(
            "input",
            Some("input.json"),
            json!({"title": "Native result"}),
        );
        let secondary = BTreeMap::new();
        let target = FormatIdentity {
            content_type: Some(cem_ml::schema::registry::JSON_CONTENT_TYPE.to_owned()),
            schema: Some(cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };

        let rendered = adapter
            .render(TransformTemplateRenderRequest {
                compiled: &compiled,
                primary_input: &primary,
                secondary_inputs: &secondary,
                target: Some(&target),
                target_scope: &ScopeConfig::default(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::CemQlExpression,
                    ..TransformExecutionPolicy::default()
                },
            })
            .expect("expression renders");
        let TransformArtifactBody::Extension(native) = &rendered.output.body else {
            panic!("CEM-QL expression output must remain native before export");
        };
        let result = native
            .as_any()
            .downcast_ref::<CemQlResultArtifact>()
            .expect("native CEM-QL result body");
        assert_eq!(result.stream.items.len(), 1);

        let context = engine_context_with_cem_ql_template_adapter();
        let encoded = context
            .transform_artifact_exporter_registry
            .export(&rendered.output.body, &target)
            .expect("registered explicit JSON exporter");
        assert_eq!(encoded.encoding, TransformEncoding::Json);
        assert_eq!(
            serde_json::from_slice::<Value>(encoded.bytes.as_ref()).expect("exported JSON"),
            item_stream_json(&result.stream)
        );
        let non_json_target = FormatIdentity {
            content_type: Some("text/plain".to_owned()),
            ..FormatIdentity::default()
        };
        let error = context
            .transform_artifact_exporter_registry
            .export(&rendered.output.body, &non_json_target)
            .expect_err("CEM-QL result exporter requires an explicit JSON target");
        assert!(error.contains("JSON encoding requires"), "{error}");
    }

    #[test]
    fn cql_expression_render_source_has_no_json_materialization() {
        let source = include_str!("lib.rs");
        let render = source
            .split("fn render_cem_ql_expression_payload")
            .nth(1)
            .and_then(|source| source.split("fn target_is_cem_tree").next())
            .expect("CEM-QL expression render source");
        for forbidden in ["item_stream_json", "serde_json", "to_value"] {
            assert!(
                !render.contains(forbidden),
                "CEM-QL expression rendering must not materialize `{forbidden}`"
            );
        }
    }

    #[test]
    fn transform_parameter_handoff_uses_compiled_typed_arena_without_json_payload() {
        let source = include_str!("lib.rs");
        let payload = source
            .split("struct CemQlCompiledTemplatePayload")
            .nth(1)
            .and_then(|source| {
                source
                    .split("struct CemQlCompiledTemplateModulePayload")
                    .next()
            })
            .expect("compiled adapter payload source");
        assert!(!payload.contains("params:"));
        assert!(!payload.contains("BTreeMap<String, Value>"));

        let bindings = source
            .split("fn expression_policy_bindings")
            .nth(1)
            .and_then(|source| source.split("fn explicit_param_value_to_stream").next())
            .expect("typed parameter binding source");
        assert!(bindings.contains("TransformTemplateParameterArena"));
        for forbidden in ["serde_json", "to_public_json", "BTreeMap<String, Value>"] {
            assert!(
                !bindings.contains(forbidden),
                "typed parameter binding must not use `{forbidden}`"
            );
        }
    }

    fn assert_missing_native_query_view_diagnostic(
        diagnostics: &[Diagnostic],
        representation: &str,
        node: Option<&str>,
    ) {
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.code == TransformTemplateAdapterError::FAILED_CODE
                    && diagnostic.severity == Severity::Fatal
                    && diagnostic.message.contains(representation)
                    && diagnostic
                        .message
                        .contains("has no CEM-QL native query view")
                    && node.is_none_or(|node| diagnostic.node.as_deref() == Some(node))
            }),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn cql_native_cem_ingress_retains_document_identity_and_projects_fields() {
        let document = Arc::new(document_from_cem("{main @id=demo | Hello}"));
        let artifact = TransformTemplateDataArtifact::new(
            "native",
            Some("data.cem".to_owned()),
            Some(FormatIdentity {
                content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            TransformArtifactBody::CemDocument(Arc::clone(&document)),
        );

        let stream = artifact_query_stream(&artifact).expect("native CEM is queryable");
        let root = stream.items.first().expect("native CEM root item");
        let view = root
            .view()
            .and_then(|view| view.downcast_ref::<CemDocumentQueryView>())
            .expect("CEM root remains a native document view");
        assert!(Arc::ptr_eq(&view.document, &document));
        assert_eq!(view.node_id, 0);

        let children = root
            .view()
            .and_then(|view| view.field("children"))
            .expect("document children field");
        assert_eq!(children.len(), 1);
        assert_eq!(
            children[0]
                .view()
                .and_then(|view| view.field("name"))
                .and_then(|items| items.first().and_then(Item::atom)),
            Some(AtomValue::String("main".to_owned()))
        );
        assert!(children[0].source_map().is_some());
    }

    #[test]
    fn cql_explicit_json_ingress_uses_lossless_ast_with_duplicate_members() {
        let artifact = TransformTemplateDataArtifact::encoded(
            "json",
            Some("data.json".to_owned()),
            FormatIdentity {
                content_type: Some(cem_ml::schema::registry::JSON_CONTENT_TYPE.to_owned()),
                schema: Some(cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            },
            TransformEncoding::Json,
            br#"{"name":"first","name":"second","ratio":1.2300}"#.to_vec(),
        )
        .expect("explicit JSON artifact");

        let stream = artifact_query_stream(&artifact).expect("explicit JSON is lifecycle parsed");
        let root = stream.items.first().expect("JSON root item");
        let view = root
            .view()
            .and_then(|view| view.downcast_ref::<JsonValueQueryView>())
            .expect("JSON object remains a native AST view");
        assert!(matches!(
            view.owner.as_ref(),
            LoadedInputAstStream::JsonDocument(_)
        ));

        let names = root
            .view()
            .and_then(|view| view.field("name"))
            .expect("duplicate name field");
        assert_eq!(names.len(), 2);
        assert_eq!(names[0].atom(), Some(AtomValue::String("first".to_owned())));
        assert_eq!(
            names[1].atom(),
            Some(AtomValue::String("second".to_owned()))
        );
        assert_ne!(names[0].identity(), names[1].identity());
        assert!(names.iter().all(|item| item.source_map().is_some()));

        let ratio = root
            .view()
            .and_then(|view| view.field("ratio"))
            .and_then(|items| items.first().and_then(Item::atom));
        assert_eq!(ratio, Some(AtomValue::Decimal("1.2300".to_owned())));
    }

    #[test]
    fn cql_xml_ingress_retains_lifecycle_event_and_attribute_identity() {
        let (document, diagnostics) =
            xml_document_ast_from_source_bytes(XmlSourceValidationRequest {
                bytes: br#"<root xmlns:ex="urn:example" ex:id="one">text</root>"#,
                source_uri: "data.xml",
                content_type: Some(cem_ml::schema::registry::XML_CONTENT_TYPE),
            });
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        let owner = Arc::new(LoadedInputAstStream::XmlDocument(
            document.expect("XML lifecycle document"),
        ));
        let artifact = TransformTemplateDataArtifact::new(
            "xml",
            Some("data.xml".to_owned()),
            Some(FormatIdentity {
                content_type: Some(cem_ml::schema::registry::XML_CONTENT_TYPE.to_owned()),
                schema: Some(cem_ml::schema::registry::XML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            TransformArtifactBody::Lifecycle(Arc::clone(&owner)),
        );

        let stream = artifact_query_stream(&artifact).expect("XML lifecycle AST is queryable");
        let root = stream.items.first().expect("XML document view");
        let document_view = root
            .view()
            .and_then(|view| view.downcast_ref::<XmlDocumentQueryView>())
            .expect("XML document remains a native view");
        assert!(Arc::ptr_eq(&document_view.owner, &owner));
        assert_eq!(root.identity().as_deref(), Some("xml:document"));

        let events = root
            .view()
            .and_then(|view| view.field("events"))
            .expect("XML event sequence");
        let start = events
            .iter()
            .find(|item| {
                item.view()
                    .and_then(|view| view.field("kind"))
                    .and_then(|items| items.first().and_then(Item::atom))
                    == Some(AtomValue::String("start-element".to_owned()))
            })
            .expect("start element event");
        let event_view = start
            .view()
            .and_then(|view| view.downcast_ref::<XmlEventQueryView>())
            .expect("XML event remains a native view");
        assert!(Arc::ptr_eq(&event_view.owner, &owner));
        assert_eq!(
            start.identity(),
            Some(format!("xml:event:{}", event_view.index))
        );
        assert!(start.source_map().is_some());

        let attributes = start
            .view()
            .and_then(|view| view.field("attributes"))
            .and_then(|items| items.first().and_then(Item::members))
            .expect("XML attribute sequence");
        let attribute_view = attributes
            .iter()
            .find_map(|item| {
                item.view()
                    .and_then(|view| view.downcast_ref::<XmlAttributeQueryView>())
                    .filter(|view| {
                        view.attribute()
                            .is_some_and(|attribute| attribute.local_name == "id")
                    })
            })
            .expect("namespaced id attribute view");
        assert!(Arc::ptr_eq(&attribute_view.owner, &owner));
        assert_eq!(
            attribute_view.identity(),
            format!(
                "xml:event:{}:attribute:{}",
                attribute_view.event_index, attribute_view.attribute_index
            )
        );
        assert!(attribute_view.source_map().is_some());
        assert_eq!(
            attribute_view.field("namespaceUri"),
            Some(atom_items("urn:example"))
        );
    }

    #[test]
    fn cql_collection_ingress_retains_collection_and_child_artifact_identity() {
        let document = Arc::new(document_from_cem("{main}"));
        let child = Arc::new(TransformTemplateDataArtifact::new(
            "child",
            Some("child.cem".to_owned()),
            Some(FormatIdentity {
                content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            }),
            TransformArtifactBody::CemDocument(Arc::clone(&document)),
        ));
        let item_source_map = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange::new(11, 5)),
                transform: TransformKind::InterpreterRender,
            }],
        };
        let collection = Arc::new(TransformArtifactCollection {
            mode: TransformArtifactCollectionMode::Collect,
            bindings: BTreeMap::from([("count".to_owned(), "1".to_owned())]),
            items: vec![TransformArtifactCollectionItem {
                input_name: "primary".to_owned(),
                destination: Some("dist/main.html".to_owned()),
                target: Some(FormatIdentity {
                    content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                    schema: Some(HTML_SCHEMA_URI.to_owned()),
                    ..FormatIdentity::default()
                }),
                bindings: BTreeMap::from([("slug".to_owned(), "main".to_owned())]),
                artifact: Arc::clone(&child),
                source_map: Some(item_source_map.clone()),
                output_spans: vec![OutputSpan {
                    output_range: ByteRange::new(3, 4),
                    origin: item_source_map.clone(),
                }],
            }],
        });
        let artifact = TransformTemplateDataArtifact::new(
            "collection",
            None,
            None,
            TransformArtifactBody::Collection(Arc::clone(&collection)),
        );

        let stream = artifact_query_stream(&artifact).expect("collection is queryable");
        let root = stream.items.first().expect("collection view");
        let collection_view = root
            .view()
            .and_then(|view| view.downcast_ref::<TransformCollectionQueryView>())
            .expect("collection remains a native view");
        assert!(Arc::ptr_eq(&collection_view.collection, &collection));
        assert_eq!(
            collection_view.field("bindings"),
            Some(string_map_record_items(&BTreeMap::from([(
                "count".to_owned(),
                "1".to_owned(),
            )])))
        );

        let items = root
            .view()
            .and_then(|view| view.field("items"))
            .and_then(|items| items.first().and_then(Item::members))
            .expect("collection item sequence");
        let item_view = items[0]
            .view()
            .and_then(|view| view.downcast_ref::<TransformCollectionItemQueryView>())
            .expect("collection item remains a native view");
        assert!(Arc::ptr_eq(&item_view.collection, &collection));
        assert_eq!(
            item_view.field("bindings"),
            Some(string_map_record_items(&BTreeMap::from([(
                "slug".to_owned(),
                "main".to_owned(),
            )])))
        );
        assert_eq!(item_view.source_map(), Some(item_source_map.clone()));

        let target = item_view.field("target").expect("typed target identity");
        let target_view = target[0]
            .view()
            .and_then(|view| view.downcast_ref::<TransformCollectionTargetQueryView>())
            .expect("target remains a native identity view");
        assert!(std::ptr::eq(
            target_view.target().expect("target owner"),
            collection.items[0]
                .target
                .as_ref()
                .expect("collection target")
        ));
        assert_eq!(
            target_view.field("contentType"),
            Some(atom_items(HTML_CONTENT_TYPE))
        );

        let source_map = item_view.field("sourceMap").expect("typed source map");
        let source_map_view = source_map[0]
            .view()
            .and_then(|view| view.downcast_ref::<TransformCollectionSourceMapQueryView>())
            .expect("source map remains a native provenance view");
        assert!(std::ptr::eq(
            source_map_view.source_map_ref().expect("source map owner"),
            collection.items[0]
                .source_map
                .as_ref()
                .expect("collection item source map")
        ));

        let output_spans = item_view
            .field("outputSpans")
            .and_then(|items| items.first().and_then(Item::members))
            .expect("typed output spans");
        let output_span_view = output_spans[0]
            .view()
            .and_then(|view| view.downcast_ref::<TransformCollectionOutputSpanQueryView>())
            .expect("output span remains a native provenance view");
        assert!(std::ptr::eq(
            output_span_view.output_span().expect("output span owner"),
            &collection.items[0].output_spans[0]
        ));
        assert_eq!(
            output_span_view.field("outputRange"),
            Some(vec![Item::Record(BTreeMap::from([
                ("start".to_owned(), integer_items(3)),
                ("length".to_owned(), integer_items(4)),
                ("end".to_owned(), integer_items(7)),
            ]))])
        );

        let artifact = item_view.field("artifact").expect("typed child artifact");
        let child_view = artifact[0]
            .view()
            .and_then(|view| view.downcast_ref::<CemDocumentQueryView>())
            .expect("child remains a native CEM view");
        assert!(Arc::ptr_eq(&child_view.document, &document));
        assert!(Arc::ptr_eq(&collection.items[0].artifact, &child));
        assert_eq!(item_view.field("primary"), item_view.field("artifact"));
    }

    #[test]
    fn cql_collection_modes_preserve_order_cardinality_and_owner_identity() {
        for mode in [
            TransformArtifactCollectionMode::Collect,
            TransformArtifactCollectionMode::GroupBy,
            TransformArtifactCollectionMode::MatchBy,
            TransformArtifactCollectionMode::Zip,
        ] {
            let first_document = Arc::new(document_from_cem("{first}"));
            let second_document = Arc::new(document_from_cem("{second}"));
            let first = Arc::new(TransformTemplateDataArtifact::new(
                "first",
                Some("memory:first.cem".to_owned()),
                None,
                TransformArtifactBody::CemDocument(Arc::clone(&first_document)),
            ));
            let second = Arc::new(TransformTemplateDataArtifact::new(
                "second",
                Some("memory:second.cem".to_owned()),
                None,
                TransformArtifactBody::CemDocument(Arc::clone(&second_document)),
            ));
            let collection = Arc::new(TransformArtifactCollection {
                mode,
                bindings: BTreeMap::from([
                    ("count".to_owned(), "2".to_owned()),
                    ("key".to_owned(), mode.as_str().to_owned()),
                ]),
                items: vec![
                    TransformArtifactCollectionItem {
                        input_name: "secondary".to_owned(),
                        destination: None,
                        target: None,
                        bindings: BTreeMap::new(),
                        artifact: Arc::clone(&second),
                        source_map: None,
                        output_spans: Vec::new(),
                    },
                    TransformArtifactCollectionItem {
                        input_name: "primary".to_owned(),
                        destination: None,
                        target: None,
                        bindings: BTreeMap::new(),
                        artifact: Arc::clone(&first),
                        source_map: None,
                        output_spans: Vec::new(),
                    },
                ],
            });
            let artifact = TransformTemplateDataArtifact::new(
                format!("{}-collection", mode.as_str()),
                None,
                None,
                TransformArtifactBody::Collection(Arc::clone(&collection)),
            );

            let stream = artifact_query_stream(&artifact).expect("collection mode is queryable");
            let root = stream.items.first().expect("collection root");
            let view = root
                .view()
                .and_then(|view| view.downcast_ref::<TransformCollectionQueryView>())
                .expect("borrowed collection view");
            assert!(Arc::ptr_eq(&view.collection, &collection));
            assert_eq!(view.field("mode"), Some(atom_items(mode.as_str())));
            assert_eq!(
                view.field("count"),
                Some(vec![Item::Atomic(AtomValue::Integer(2))])
            );
            let items = view
                .field("items")
                .and_then(|items| items.first().and_then(Item::members))
                .expect("ordered collection items");
            let input_names = items
                .iter()
                .map(|item| {
                    item.view()
                        .and_then(|view| view.field("inputName"))
                        .and_then(|items| items.first().and_then(Item::atom))
                })
                .collect::<Vec<_>>();
            assert_eq!(
                input_names,
                vec![
                    Some(AtomValue::String("secondary".to_owned())),
                    Some(AtomValue::String("primary".to_owned())),
                ],
                "{} item order",
                mode.as_str()
            );
            assert!(Arc::ptr_eq(&collection.items[0].artifact, &second));
            assert!(Arc::ptr_eq(&collection.items[1].artifact, &first));
        }
    }

    #[test]
    fn cql_artifact_ingress_source_has_no_generic_json_bridge() {
        let source = include_str!("lib.rs");
        let ingress = source
            .split("fn artifact_query_stream")
            .nth(1)
            .and_then(|source| source.split("struct CemQlQueryResultExporter").next())
            .expect("CEM-QL artifact ingress source");
        for forbidden in [
            "explicit_json_value",
            "serde_json::",
            "to_cemt_subject",
            "value_to_stream",
        ] {
            assert!(
                !ingress.contains(forbidden),
                "CEM-QL artifact ingress must not call `{forbidden}`"
            );
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
        let params = TransformTemplateParameterArena::default();
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
            .encoded_text_value()
            .expect("converter output should be encoded text content")
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact(
            "data",
            Some("data.json"),
            json_object([
                ("label", Value::String("Save".to_owned())),
                ("tone", Value::String("primary".to_owned())),
            ]),
        );
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact(
            "data",
            Some("data.cem"),
            json_object([("kind", Value::String("document".to_owned()))]),
        );
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::from_normalized_values(BTreeMap::from([(
            "title".to_owned(),
            Value::String("Intro".to_owned()),
        )]))
        .expect("typed params");
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
        let primary_input = explicit_json_test_artifact(
            "data",
            Some("data.cem"),
            json_object([("kind", Value::String("document".to_owned()))]),
        );
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let params = TransformTemplateParameterArena::default();
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
        let params = TransformTemplateParameterArena::default();
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
                primary_input: &explicit_json_test_artifact("data", None, Value::Null),
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
        let params = TransformTemplateParameterArena::default();
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
        let params = TransformTemplateParameterArena::default();
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact(
            "data",
            Some("data.json"),
            json_object([("label", Value::String("orders".to_owned()))]),
        );
        let secondary_inputs = BTreeMap::from([(
            "meta".to_owned(),
            Arc::new(explicit_json_test_artifact(
                "meta",
                Some("meta.json"),
                json_object([("count", Value::Number(3.into()))]),
            )),
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact(
            "data",
            None,
            json_object([("enabled", Value::Bool(false))]),
        );
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input =
            explicit_json_test_artifact("data", None, json_object([("title", Value::Null)]));
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact(
            "data",
            None,
            json_object([(
                "sourceSettings",
                json_object([("enabled", Value::Bool(true))]),
            )]),
        );
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
            Value::String("<p>AB</p>".to_owned())
        );
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
        let params = TransformTemplateParameterArena::from_normalized_values(BTreeMap::from([
            ("locale".to_owned(), Value::String("fr-FR".to_owned())),
            ("title".to_owned(), Value::String("Intro".to_owned())),
        ]))
        .expect("typed params");
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::from_normalized_values(BTreeMap::from([
            ("locale".to_owned(), Value::String("fr-FR".to_owned())),
            ("card.title".to_owned(), Value::String("Intro".to_owned())),
        ]))
        .expect("typed params");
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::from_normalized_values(BTreeMap::from([(
            "title".to_owned(),
            Value::Null,
        )]))
        .expect("typed params");
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input =
            explicit_json_test_artifact("data", None, json_object([("title", Value::Null)]));
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact(
            "data",
            None,
            json_object([(
                "sourceSettings",
                json_object([("enabled", Value::Bool(true))]),
            )]),
        );
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact(
            "data",
            None,
            json_object([("sourceCount", Value::Number(7.into()))]),
        );
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
        let primary_input = explicit_json_test_artifact("data", None, Value::Null);
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
            test_output_value(&rendered.output),
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
        let params = TransformTemplateParameterArena::default();
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
            let params = TransformTemplateParameterArena::default();
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
            test_output_value(&rendered.output),
                    Value::String(
                        r#"<?xml-stylesheet href="main.css"?><p class="lead"><![CDATA[Hi <all>]]></p>"#
                            .to_owned()
                    )
                );
            } else {
                assert_eq!(
                    test_output_value(&rendered.output),
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
    fn real_engine_convert_rejects_unmigrated_packaged_cemt_native_tree_ingress() {
        for (uri, source, input_format, target_format, _, _) in [
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

            assert_eq!(response.primary, Value::Null, "{uri}");
            assert_missing_native_query_view_diagnostic(
                &response.diagnostics,
                "cem.tree-ast",
                None,
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
    fn cem_ql_source_output_source_has_no_json_stage_bridge() {
        let source = include_str!("lib.rs");
        let (_, token_tree_source) = source
            .split_once("fn cem_ql_source_token_tree(")
            .expect("typed CEM-QL token-tree builder");
        let (token_tree_source, _) = token_tree_source
            .split_once("fn cem_ql_token_lexeme")
            .expect("token-tree builder boundary");
        assert!(token_tree_source.contains("-> CemQlSourceTokenTreeAst"));
        assert!(!token_tree_source.contains("-> Value"));
        assert!(!token_tree_source.contains("json!("));

        let (_, direct_output_source) = source
            .split_once("fn convert_cem_ql_source_output(")
            .expect("direct CEM-QL output handler");
        let (direct_output_source, _) = direct_output_source
            .split_once("fn cem_ql_output_pipeline(")
            .expect("direct output handler boundary");
        assert!(direct_output_source.contains(
            "execute_conversion_output_pipeline_from_typed_cemt_subject_with_environment"
        ));
        assert!(
            !direct_output_source.contains("execute_conversion_output_pipeline_with_environment(")
        );
    }

    #[test]
    fn real_engine_convert_uses_registered_cem_ql_source_output_handler_preserves_token_ranges() {
        let source = "module \"https://example.test/queries/source-token-ranges\"\n\n// comment\ndeclare let label = \"héllo\"\n\nlabel\n";
        let tree = cem_ql_source_token_tree("source-token-ranges.cemql", source);
        let root = CemtEvaluatorValueRef::Record(CemtEvaluatorRecordRef::Package { record: &tree });
        let tokens = root
            .field("tokens")
            .and_then(|tokens| tokens.as_sequence().cloned())
            .expect("borrowed token AST sequence");
        assert!(tokens.iter().any(|token| {
            token.field("tokenKind").and_then(|value| value.as_str()) == Some("LineComment")
                && token.field("role").and_then(|value| value.as_str()) == Some("syntax.comment")
        }));

        let string_token_index = tree
            .tokens
            .iter()
            .position(|token| token.value.lexeme == "\"héllo\"")
            .expect("string token with non-ASCII payload");
        let string_token = &tree.tokens[string_token_index];
        let string_token_view = tokens
            .item(string_token_index)
            .expect("borrowed string token view");
        let expected_offset = source.find("\"héllo\"").expect("string lexeme offset") as u64;
        let expected_len = "\"héllo\"".len() as u64;
        assert_eq!(
            string_token_view
                .field("value")
                .and_then(|value| value.field("byteOffset"))
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(expected_offset)
        );
        assert_eq!(
            string_token_view
                .field("value")
                .and_then(|value| value.field("byteLength"))
                .and_then(|value| value.as_number())
                .and_then(CemtEvaluatorNumber::as_u64),
            Some(expected_len)
        );
        let evaluator_source_map_value = string_token_view
            .field("sourceMap")
            .expect("borrowed token source map");
        let evaluator_source_map = evaluator_source_map_value
            .as_source_map()
            .expect("source-map evaluator value");
        assert!(std::ptr::eq(evaluator_source_map, &string_token.source_map));
        assert_eq!(string_token.value.token.range.start, expected_offset);
        assert_eq!(u64::from(string_token.value.token.range.len), expected_len);
        assert_eq!(string_token.output_span.origin, string_token.source_map);
        assert_eq!(
            string_token.output_span.output_range.len,
            expected_len as u32
        );
    }

    #[test]
    fn cem_ql_formatter_tab_size_uses_shared_default_and_positive_option() {
        assert_eq!(
            cem_ql_formatter_tab_size(&ScopeConfig::default()),
            DEFAULT_FORMATTER_TAB_SIZE as usize
        );

        let explicit = ScopeConfig {
            cemt_formatter_options: BTreeMap::from([("tabSize".to_owned(), "6".to_owned())]),
            ..ScopeConfig::default()
        };
        assert_eq!(cem_ql_formatter_tab_size(&explicit), 6);

        let invalid = ScopeConfig {
            cemt_formatter_options: BTreeMap::from([("tabSize".to_owned(), "0".to_owned())]),
            ..ScopeConfig::default()
        };
        assert_eq!(
            cem_ql_formatter_tab_size(&invalid),
            DEFAULT_FORMATTER_TAB_SIZE as usize
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
                    cemt_formatter_options: BTreeMap::from([(
                        "tabSize".to_owned(),
                        "6".to_owned(),
                    )]),
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
        assert!(html.starts_with(&cem_ql_html_preview_prefix(6)), "{html}");
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
    fn real_engine_transform_cem_ql_queries_native_cem_ingress() {
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
    fn real_engine_transform_cem_ql_expression_queries_native_cem_ingress() {
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

        assert_eq!(
            response.primary,
            json!({
                "diagnostics": [],
                "error": null,
                "items": [{
                    "kind": "atomic",
                    "type": "string",
                    "value": "document",
                }],
            })
        );
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert!(response.source_map.is_none());
        assert!(response.output_spans.is_empty());
    }

    #[test]
    fn real_engine_transform_graph_exports_native_cem_ql_expression_as_explicit_json() {
        let json_target = FormatIdentity {
            content_type: Some(cem_ml::schema::registry::JSON_CONTENT_TYPE.to_owned()),
            schema: Some(cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
            ..FormatIdentity::default()
        };
        let request = TransformGraphRequest {
            imports: vec![TransformGraphImport {
                id: "data".to_owned(),
                input: EngineInput {
                    uri: "data.cem".to_owned(),
                    bytes: b"{p @id=\"guide\"}".to_vec(),
                    from_format: None,
                    identity: Some(FormatIdentity {
                        content_type: Some("text/cem-ml".to_owned()),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
                scheduler_scope_id: 18,
            }],
            joins: Vec::new(),
            stages: vec![TransformGraphStage {
                id: "result".to_owned(),
                template: TemplateInput {
                    uri: "result.cem-ql".to_owned(),
                    bytes: b"input.kind".to_vec(),
                    identity: Some(FormatIdentity {
                        content_type: Some(
                            cem_ml::schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE.to_owned(),
                        ),
                        schema: Some(
                            cem_ml::schema::registry::CEM_QL_EXPRESSION_SCHEMA_URI.to_owned(),
                        ),
                        ..FormatIdentity::default()
                    }),
                    root_scope: ScopeConfig::default(),
                },
                template_kind: TransformTemplateKind::CemQlExpression,
                template_entrypoint: TransformTemplateEntrypoint::implicit(),
                params: BTreeMap::new(),
                execution_policy: TransformExecutionPolicy {
                    runtime_phase: TransformRuntimePhase::CemQlExpression,
                    ..TransformExecutionPolicy::default()
                },
                target: Some(json_target.clone()),
                primary_input: "data".to_owned(),
                secondary_inputs: BTreeMap::new(),
                scheduler_scope_ids: TransformStageSchedulerScopeIds {
                    template_load: 19,
                    execution: 20,
                },
            }],
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "result-json".to_owned(),
                input: "result".to_owned(),
                destination: Some("dist/result.json".to_owned()),
                target: Some(json_target),
                target_scope: ScopeConfig::default(),
                style_policy: Default::default(),
                scheduler_scope_id: 21,
            }],
            edges: Vec::new(),
            preserve_source_offsets: false,
            context: engine_context_with_cem_ql_template_adapter(),
            execution_policy: TransformExecutionPolicy::default(),
        };

        let response = RealCemMlEngine::new()
            .transform_graph(request)
            .expect("native expression graph should export explicit JSON");

        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
        assert_eq!(response.artifacts.len(), 1);
        assert_eq!(
            response.artifacts[0].primary,
            json!({
                "diagnostics": [],
                "error": null,
                "items": [{
                    "kind": "atomic",
                    "type": "string",
                    "value": "document",
                }],
            })
        );
    }

    #[test]
    fn real_engine_transform_xslt_remains_explicitly_unimplemented() {
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

        assert_eq!(response.primary, Value::Null);
        assert!(response.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == TransformTemplateAdapterError::NOT_IMPLEMENTED_CODE
                && diagnostic.severity == Severity::Fatal
                && diagnostic.message.contains("xslt-template")
                && diagnostic.message.contains("does not implement compile")
        }));
    }

    #[test]
    fn real_engine_transform_params_coexist_with_native_cem_ingress() {
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
            Value::String("<p>fr-FR:Intro</p>".to_owned())
        );
        assert!(
            response.diagnostics.is_empty(),
            "{:?}",
            response.diagnostics
        );
    }

    #[test]
    fn real_engine_transform_graph_routes_native_cem_to_branched_stages() {
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
        assert_eq!(response.artifacts[0].export_id, "main");
        assert_eq!(
            response.artifacts[0].primary,
            Value::String("<article>document</article>".to_owned())
        );
        assert_eq!(response.artifacts[1].export_id, "chart-svg");
        assert_eq!(
            response.artifacts[1].primary,
            Value::String("<svg>document</svg>".to_owned())
        );
    }

    #[test]
    fn real_engine_transform_graph_queries_typed_collection_ingress() {
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
                        template_load: 31,
                        execution: 32,
                    },
                },
                TransformGraphStage {
                    id: "summary".to_owned(),
                    template: TemplateInput {
                        uri: "summary.cem".to_owned(),
                        bytes: br#"{section | {$input.kind}:{$input.count}}"#.to_vec(),
                        identity: Some(identity.clone()),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "collection".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 34,
                        execution: 35,
                    },
                },
            ],
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "joined".to_owned(),
                input: "summary".to_owned(),
                destination: Some("dist/collection.html".to_owned()),
                target: Some(FormatIdentity {
                    content_type: Some("text/html".to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig::default(),
                style_policy: Default::default(),
                scheduler_scope_id: 36,
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
        assert_eq!(response.artifacts[0].export_id, "joined");
        assert_eq!(
            response.artifacts[0].primary,
            Value::String("<section>collection:1</section>".to_owned())
        );
    }

    #[test]
    fn real_engine_transform_graph_queries_multi_input_typed_collection() {
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
                        template_load: 43,
                        execution: 44,
                    },
                },
                TransformGraphStage {
                    id: "collection-summary".to_owned(),
                    template: TemplateInput {
                        uri: "collection-summary.cem".to_owned(),
                        bytes: br#"{section | {$input.kind}:{$input.count}}"#.to_vec(),
                        identity: Some(identity),
                        root_scope: ScopeConfig::default(),
                    },
                    template_kind: TransformTemplateKind::CemNative,
                    template_entrypoint: TransformTemplateEntrypoint::implicit(),
                    params: BTreeMap::new(),
                    execution_policy: TransformExecutionPolicy::default(),
                    target: None,
                    primary_input: "collection".to_owned(),
                    secondary_inputs: BTreeMap::new(),
                    scheduler_scope_ids: TransformStageSchedulerScopeIds {
                        template_load: 46,
                        execution: 47,
                    },
                },
            ],
            importmap_rewrites: Vec::new(),
            exports: vec![TransformGraphExport {
                id: "joined".to_owned(),
                input: "collection-summary".to_owned(),
                destination: Some("dist/collection.html".to_owned()),
                target: Some(FormatIdentity {
                    content_type: Some("text/html".to_owned()),
                    ..FormatIdentity::default()
                }),
                target_scope: ScopeConfig::default(),
                style_policy: Default::default(),
                scheduler_scope_id: 48,
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
        assert_eq!(
            response.artifacts[0].primary,
            Value::String("<section>collection:2</section>".to_owned())
        );
    }

    #[test]
    fn real_engine_routes_all_collection_modes_into_cem_ql_without_json_ingress() {
        for (index, (mode, mode_name)) in [
            (TransformGraphJoinMode::Collect, "collect"),
            (TransformGraphJoinMode::GroupBy, "group-by"),
            (TransformGraphJoinMode::MatchBy, "match-by"),
            (TransformGraphJoinMode::Zip, "zip"),
        ]
        .into_iter()
        .enumerate()
        {
            let base_scope = u32::try_from(index).unwrap_or_default() * 20 + 100;
            let cem_identity = FormatIdentity {
                content_type: Some(CEM_ML_CONTENT_TYPE.to_owned()),
                schema: Some(CEM_ML_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            };
            let json_target = FormatIdentity {
                content_type: Some(cem_ml::schema::registry::JSON_CONTENT_TYPE.to_owned()),
                schema: Some(cem_ml::schema::registry::JSON_VALUE_SCHEMA_URI.to_owned()),
                ..FormatIdentity::default()
            };
            let request = TransformGraphRequest {
                imports: vec![TransformGraphImport {
                    id: "source".to_owned(),
                    input: EngineInput {
                        uri: format!("memory:{mode_name}.cem"),
                        bytes: b"{p @id=source}".to_vec(),
                        from_format: None,
                        identity: Some(cem_identity.clone()),
                        root_scope: ScopeConfig::default(),
                    },
                    scheduler_scope_id: base_scope,
                }],
                joins: vec![TransformGraphJoin {
                    id: "collection".to_owned(),
                    mode,
                    input_names: vec!["secondary".to_owned(), "primary".to_owned()],
                    inputs: vec![
                        TransformGraphJoinInput {
                            input_name: "secondary".to_owned(),
                            artifact_id: "second".to_owned(),
                            bindings: BTreeMap::from([("position".to_owned(), "0".to_owned())]),
                            destination: Some("dist/second.html".to_owned()),
                            target: Some(FormatIdentity {
                                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                                ..FormatIdentity::default()
                            }),
                        },
                        TransformGraphJoinInput {
                            input_name: "primary".to_owned(),
                            artifact_id: "first".to_owned(),
                            bindings: BTreeMap::from([("position".to_owned(), "1".to_owned())]),
                            destination: Some("dist/first.html".to_owned()),
                            target: Some(FormatIdentity {
                                content_type: Some(HTML_CONTENT_TYPE.to_owned()),
                                ..FormatIdentity::default()
                            }),
                        },
                    ],
                    bindings: BTreeMap::from([
                        ("count".to_owned(), "2".to_owned()),
                        ("key".to_owned(), mode_name.to_owned()),
                    ]),
                    scheduler_scope_id: base_scope + 5,
                }],
                stages: vec![
                    TransformGraphStage {
                        id: "first".to_owned(),
                        template: TemplateInput {
                            uri: "first.cem".to_owned(),
                            bytes: b"{first}".to_vec(),
                            identity: Some(cem_identity.clone()),
                            root_scope: ScopeConfig::default(),
                        },
                        template_kind: TransformTemplateKind::CemNative,
                        template_entrypoint: TransformTemplateEntrypoint::implicit(),
                        params: BTreeMap::new(),
                        execution_policy: TransformExecutionPolicy::default(),
                        target: None,
                        primary_input: "source".to_owned(),
                        secondary_inputs: BTreeMap::new(),
                        scheduler_scope_ids: TransformStageSchedulerScopeIds {
                            template_load: base_scope + 1,
                            execution: base_scope + 2,
                        },
                    },
                    TransformGraphStage {
                        id: "second".to_owned(),
                        template: TemplateInput {
                            uri: "second.cem".to_owned(),
                            bytes: b"{second}".to_vec(),
                            identity: Some(cem_identity),
                            root_scope: ScopeConfig::default(),
                        },
                        template_kind: TransformTemplateKind::CemNative,
                        template_entrypoint: TransformTemplateEntrypoint::implicit(),
                        params: BTreeMap::new(),
                        execution_policy: TransformExecutionPolicy::default(),
                        target: None,
                        primary_input: "source".to_owned(),
                        secondary_inputs: BTreeMap::new(),
                        scheduler_scope_ids: TransformStageSchedulerScopeIds {
                            template_load: base_scope + 3,
                            execution: base_scope + 4,
                        },
                    },
                    TransformGraphStage {
                        id: "result".to_owned(),
                        template: TemplateInput {
                            uri: "result.cem-ql".to_owned(),
                            bytes: b"input.bindings.key".to_vec(),
                            identity: Some(FormatIdentity {
                                content_type: Some(
                                    cem_ml::schema::registry::CEM_QL_EXPRESSION_CONTENT_TYPE
                                        .to_owned(),
                                ),
                                schema: Some(
                                    cem_ml::schema::registry::CEM_QL_EXPRESSION_SCHEMA_URI
                                        .to_owned(),
                                ),
                                ..FormatIdentity::default()
                            }),
                            root_scope: ScopeConfig::default(),
                        },
                        template_kind: TransformTemplateKind::CemQlExpression,
                        template_entrypoint: TransformTemplateEntrypoint::implicit(),
                        params: BTreeMap::new(),
                        execution_policy: TransformExecutionPolicy {
                            runtime_phase: TransformRuntimePhase::CemQlExpression,
                            ..TransformExecutionPolicy::default()
                        },
                        target: Some(json_target.clone()),
                        primary_input: "collection".to_owned(),
                        secondary_inputs: BTreeMap::new(),
                        scheduler_scope_ids: TransformStageSchedulerScopeIds {
                            template_load: base_scope + 6,
                            execution: base_scope + 7,
                        },
                    },
                ],
                importmap_rewrites: Vec::new(),
                exports: vec![TransformGraphExport {
                    id: "result-json".to_owned(),
                    input: "result".to_owned(),
                    destination: Some(format!("dist/{mode_name}.json")),
                    target: Some(json_target),
                    target_scope: ScopeConfig::default(),
                    style_policy: Default::default(),
                    scheduler_scope_id: base_scope + 8,
                }],
                edges: Vec::new(),
                preserve_source_offsets: false,
                context: engine_context_with_cem_ql_template_adapter(),
                execution_policy: TransformExecutionPolicy::default(),
            };

            let response = RealCemMlEngine::new()
                .transform_graph(request)
                .unwrap_or_else(|error| panic!("{mode_name} graph failed: {error:?}"));
            assert!(
                response.diagnostics.is_empty(),
                "{mode_name}: {:?}",
                response.diagnostics
            );
            assert_eq!(response.artifacts.len(), 1, "{mode_name}");
            assert_eq!(
                response.artifacts[0].primary,
                json!({
                    "diagnostics": [],
                    "error": null,
                    "items": [{
                        "kind": "atomic",
                        "type": "string",
                        "value": mode_name,
                    }],
                }),
                "{mode_name} CEM-QL result"
            );
        }
    }

    #[test]
    fn real_engine_transform_graph_attributes_render_diagnostics_to_stage() {
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
                && diagnostic.severity == Severity::Fatal
                && diagnostic.node.as_deref() == Some("transform:chart")
        }));
    }

    #[test]
    fn real_engine_transform_graph_queries_encoded_text_secondary_inputs() {
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
        assert_eq!(response.artifacts[0].export_id, "main");
        assert_eq!(
            response.artifacts[0].primary,
            Value::String(
                "<section>document:&lt;span&gt;document&lt;/span&gt;</section>".to_owned()
            )
        );
    }
}
