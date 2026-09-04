//! Schema-owned CEM-QL slots for embedded XPath expressions.
//!
//! Slot construction is the only parsing boundary. Runtime invocation consumes
//! the retained XPath AST plus native XPath context and variable sequences.

use std::sync::Arc;

use cem_ml::diagnostics::Diagnostic;
use cem_ml::module_resolution::CemModuleUrlResolutionCapability;
use cem_ml::resolver::{ResolverPolicy, ResolverRegistry};
use cem_ml::validation::xpath::{
    validate_xpath_expression_ast, xpath_expression_ast_from_source_bytes,
    CemQlXPathInvocationAdapter, XPathAttachment, XPathDynamicContext, XPathEvaluationLimits,
    XPathEvaluationPhase, XPathEvaluationRequest, XPathExpectedResult, XPathExpressionAst,
    XPathHostAttachment, XPathHostNodeKind, XPathHostOwner, XPathInvocationAdapter,
    XPathInvocationHost, XPathResultArtifact, XPathResultItem, XPathSchemaContractCatalog,
    XPathSourceRange, XPathSourceRequest, XPathStaticContext, XPathVariableBindings,
    XPATH_CONTENT_TYPE,
};

use crate::api::{CEM_QL_EXPRESSION_CONTENT_TYPE, CEM_QL_EXPRESSION_SCHEMA_URI};

pub const CEM_QL_XPATH_SLOT_KIND: &str = "xpath";
pub const CEM_QL_XPATH_HOST_PACKAGE: &str = "cem-ql/1";

#[derive(Debug, Clone)]
pub struct CemQlXPathExpressionSlotRequest<'a> {
    pub bytes: &'a [u8],
    pub source_uri: &'a str,
    pub source_id: u32,
    pub slot_path: &'a str,
    pub owner_range: XPathSourceRange,
    pub expression_range: XPathSourceRange,
    pub static_context: XPathStaticContext,
    pub expected_result: Option<XPathExpectedResult>,
    pub evaluation_phase: XPathEvaluationPhase,
    pub resolver_policy_stamp: Option<&'a str>,
    pub safety_policy_stamp: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct CemQlXPathExpressionSlot {
    pub host_package: String,
    pub slot_kind: String,
    pub slot_path: String,
    pub expression: Arc<XPathExpressionAst>,
}

pub fn cem_ql_xpath_expression_slot_from_source_bytes(
    request: CemQlXPathExpressionSlotRequest<'_>,
) -> (Option<CemQlXPathExpressionSlot>, Vec<Diagnostic>) {
    let slot_path = request.slot_path.to_owned();
    let attachment = XPathAttachment::Host(XPathHostAttachment {
        owner: XPathHostOwner {
            source_id: request.source_id,
            source_uri: request.source_uri.to_owned(),
            content_type: Some(CEM_QL_EXPRESSION_CONTENT_TYPE.to_owned()),
            schema_uri: Some(CEM_QL_EXPRESSION_SCHEMA_URI.to_owned()),
            node_kind: XPathHostNodeKind::CemQlExpressionSlot,
            node_id: Some(slot_path.clone()),
            source_range: request.owner_range,
        },
        expression_range: request.expression_range,
        static_context: request.static_context,
        expected_result: request.expected_result,
        evaluation_phase: request.evaluation_phase,
        resolver_policy_stamp: request.resolver_policy_stamp.map(str::to_owned),
        safety_policy_stamp: request.safety_policy_stamp.map(str::to_owned),
    });
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: request.bytes,
            source_uri: request.source_uri,
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        attachment,
    );
    let diagnostics =
        validate_xpath_expression_ast(&expression, XPathSchemaContractCatalog::from_builtin());
    let compiled = expression.syntax_ast.is_some()
        && !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity.is_hard_violation());
    let slot = compiled.then(|| CemQlXPathExpressionSlot {
        host_package: CEM_QL_XPATH_HOST_PACKAGE.to_owned(),
        slot_kind: CEM_QL_XPATH_SLOT_KIND.to_owned(),
        slot_path,
        expression: Arc::new(expression),
    });
    (slot, diagnostics)
}

#[derive(Debug, Clone, Default)]
pub struct CemQlXPathHostBindings {
    pub context_item: Option<XPathResultItem>,
    pub variable_bindings: XPathVariableBindings,
}

#[derive(Debug, Clone, Copy)]
pub struct CemQlXPathRuntimeContext<'a> {
    pub resolver_registry: &'a ResolverRegistry,
    pub resolver_policy: &'a ResolverPolicy,
    pub module_resolution: Option<&'a CemModuleUrlResolutionCapability>,
}

pub fn invoke_cem_ql_xpath_expression_slot(
    slot: &CemQlXPathExpressionSlot,
    host_bindings: &CemQlXPathHostBindings,
    evaluation_limits: XPathEvaluationLimits,
    runtime: CemQlXPathRuntimeContext<'_>,
) -> Result<XPathResultArtifact, Vec<Diagnostic>> {
    let attachment = cem_ql_xpath_slot_attachment(slot);
    let static_context = attachment
        .map(|attachment| attachment.static_context.clone())
        .unwrap_or_default();
    let expected_result = attachment.and_then(|attachment| attachment.expected_result.clone());
    let safety_policy_stamp = attachment
        .and_then(|attachment| attachment.safety_policy_stamp.as_deref())
        .unwrap_or("xpath-safety/1;cem-ql-expression-slot");

    CemQlXPathInvocationAdapter.invoke(XPathEvaluationRequest {
        invocation_host: XPathInvocationHost::CemQl,
        expression: slot.expression.as_ref(),
        dynamic_context: XPathDynamicContext {
            context_item: host_bindings.context_item.clone(),
            variable_bindings: host_bindings.variable_bindings.clone(),
            ..XPathDynamicContext::default()
        },
        static_context,
        expected_result,
        resolver_registry: runtime.resolver_registry,
        resolver_policy: runtime.resolver_policy,
        evaluation_limits,
        safety_policy_stamp,
        module_resolution: runtime.module_resolution,
    })
}

fn cem_ql_xpath_slot_attachment(slot: &CemQlXPathExpressionSlot) -> Option<&XPathHostAttachment> {
    match &slot.expression.attachment {
        XPathAttachment::Host(attachment)
            if attachment.owner.node_kind == XPathHostNodeKind::CemQlExpressionSlot =>
        {
            Some(attachment)
        }
        _ => None,
    }
}
