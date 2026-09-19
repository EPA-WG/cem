use super::*;
use crate::{
    eval::{AtomValue, BudgetAxis, EvalError, Item, ItemStream},
    native::{NativeQueryFunction, NativeQueryRequest},
    xpath::functions::{
        bind_argument, invocation_failure, item_has_type, Parameter, XPathQueryItem,
    },
};
use cem_ml::{
    resolver::{ResolverPolicy, ResolverRegistry},
    validation::xpath::{
        XPathDynamicContext, XPathEvaluationRequest, XPathResultItem, XPathResultSequence,
        XPathXsltGroupContext, XsltXPathInvocationAdapter,
    },
};

// XSLT parameters are item sequences. Reuse the existing explicitly typed
// scalar bridge for host control atoms; retained XPath/CEM values keep their
// native owners. Records and CEM arrays are not alternate document models.
fn bind_xslt_argument(
    parameter: &Parameter,
    argument: &ItemStream,
    request: &NativeQueryRequest<'_>,
) -> std::result::Result<Vec<XPathResultItem>, String> {
    if parameter.kind != ParamType::Any {
        return bind_argument(parameter, argument, request);
    }
    let mut values = Vec::new();
    for item in &argument.items {
        let kind = match item {
            Item::Atomic(AtomValue::String(_)) => ParamType::String,
            Item::Atomic(AtomValue::Boolean(_)) => ParamType::Boolean,
            Item::Atomic(AtomValue::Integer(_)) => ParamType::Integer,
            Item::Atomic(AtomValue::Decimal(_) | AtomValue::Double(_)) => ParamType::Number,
            _ => ParamType::Any,
        };
        values.extend(bind_argument(
            &Parameter {
                name: parameter.name.clone(),
                kind,
                nullable: parameter.nullable,
            },
            &ItemStream::once(item.clone()),
            request,
        )?);
    }
    Ok(values)
}

pub(super) fn registry(
    bindings: &[ProgramBinding],
    expressions: &[Arc<XPathExpressionAst>],
    limits: XPathEvaluationLimits,
) -> Result<NativeFunctionRegistry> {
    let mut registry = NativeFunctionRegistry::default();
    for (index, (binding, expression)) in bindings.iter().zip(expressions).enumerate() {
        registry
            .register(
                format!("xslt.program.{index}"),
                binding.context_arity() + binding.variables.len(),
                Installed {
                    binding: binding.clone(),
                    expression: expression.clone(),
                    limits,
                },
            )
            .map_err(BundleError::invalid)?;
    }
    Ok(registry)
}

#[derive(Debug)]
struct Installed {
    binding: ProgramBinding,
    expression: Arc<XPathExpressionAst>,
    limits: XPathEvaluationLimits,
}
impl NativeQueryFunction for Installed {
    fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
        let binding = &self.binding;
        if request.arguments.len() != binding.context_arity() + binding.variables.len() {
            return request.raise("cem.xslt.bundle_argument", "XSLT program arity mismatch");
        }
        let limit = self
            .limits
            .max_sequence_items
            .unwrap_or(request.max_result_items)
            .min(request.max_result_items);
        if request.arguments.iter().fold(0u64, |count, arg| {
            count.saturating_add(arg.items.len() as u64)
        }) > limit
        {
            let mut failure = request.raise(
                "cem.xslt.bundle_argument_limit",
                "XSLT program argument limit exceeded",
            );
            failure.error = Some(EvalError::BudgetExceeded(BudgetAxis::ItemsPerStage));
            return failure;
        }
        let mut dynamic_context = XPathDynamicContext::default();
        let arguments = (|| -> std::result::Result<(), String> {
            let absent = binding.focus == BundleFocus::OptionalSequence
                && request.arguments[0].items.is_empty();
            if absent
                && (!request.arguments[1].items.is_empty()
                    || !request.arguments[2].items.is_empty())
            {
                return Err("absent optional focus requires empty item, position and size".into());
            }
            if binding.focus != BundleFocus::Absent && !absent {
                let values = bind_xslt_argument(
                    &Parameter {
                        name: "context".into(),
                        kind: ParamType::Any,
                        nullable: false,
                    },
                    &request.arguments[0],
                    &request,
                )?;
                let [value] = values.as_slice() else {
                    return Err("XSLT context requires exactly one native item".into());
                };
                if !item_has_type(value, ParamType::Any) {
                    return Err("unsupported XSLT context item".into());
                }
                dynamic_context.context_item = Some(value.clone());
            }
            if matches!(
                binding.focus,
                BundleFocus::Sequence | BundleFocus::OptionalSequence
            ) && !absent
            {
                let coordinate = |index: usize, name: &str| -> std::result::Result<u64, String> {
                    let values = bind_argument(
                        &Parameter {
                            name: name.into(),
                            kind: ParamType::Integer,
                            nullable: false,
                        },
                        &request.arguments[index],
                        &request,
                    )?;
                    let [XPathResultItem::Atomic { value, .. }] = values.as_slice() else {
                        return Err("focus coordinate requires one integer".into());
                    };
                    value
                        .lexical_value
                        .parse::<u64>()
                        .map_err(|_| "focus coordinate is outside the unsigned 64-bit range".into())
                };
                dynamic_context.context_position = Some(coordinate(1, "position")?);
                dynamic_context.context_size = Some(coordinate(2, "size")?);
            }
            if binding.group_context {
                let offset = binding.focus.arity();
                let group_value =
                    |index: usize| -> std::result::Result<Option<XPathResultSequence>, String> {
                        let present = bind_argument(
                            &Parameter {
                                name: "group-present".into(),
                                kind: ParamType::Boolean,
                                nullable: false,
                            },
                            &request.arguments[offset + index],
                            &request,
                        )?;
                        let [XPathResultItem::Atomic { value, .. }] = present.as_slice() else {
                            return Err("XSLT group presence requires one boolean".into());
                        };
                        let present = match value.lexical_value.as_str() {
                            "true" | "1" => true,
                            "false" | "0" => false,
                            _ => return Err("invalid XSLT group presence boolean".into()),
                        };
                        if !present {
                            if !request.arguments[offset + index + 1].items.is_empty() {
                                return Err(
                                    "absent XSLT group context must have an empty payload".into()
                                );
                            }
                            return Ok(None);
                        }
                        let items = bind_xslt_argument(
                            &Parameter {
                                name: "group-value".into(),
                                kind: ParamType::Any,
                                nullable: true,
                            },
                            &request.arguments[offset + index + 1],
                            &request,
                        )?;
                        Ok(Some(XPathResultSequence {
                            sequence_type: "item()*".into(),
                            items,
                        }))
                    };
                dynamic_context.xslt_group = Some(XPathXsltGroupContext {
                    current_group: group_value(0)?,
                    current_grouping_key: group_value(2)?,
                });
            }
            for (variable, argument) in binding
                .variables
                .iter()
                .zip(&request.arguments[binding.context_arity()..])
            {
                let values = bind_xslt_argument(
                    &Parameter {
                        name: variable.name.local_name.clone(),
                        kind: variable.kind,
                        nullable: variable.nullable,
                    },
                    argument,
                    &request,
                )?;
                if values
                    .iter()
                    .any(|value| !item_has_type(value, ParamType::Any))
                {
                    return Err("unsupported XPath variable item".into());
                }
                dynamic_context.variable_bindings.insert(
                    variable.name.clone(),
                    XPathResultSequence {
                        sequence_type: "item()*".into(),
                        items: values,
                    },
                );
            }
            Ok(())
        })();
        if let Err(message) = arguments {
            return request.raise("cem.xslt.bundle_argument", message);
        }
        let XPathAttachment::Host(host) = &self.expression.attachment else {
            unreachable!("validated XSLT program")
        };
        let result = XsltXPathInvocationAdapter.invoke_with_control(
            XPathEvaluationRequest {
                expression: &self.expression,
                invocation_host: XPathInvocationHost::Xslt,
                dynamic_context,
                static_context: host.static_context.clone(),
                expected_result: host.expected_result.clone(),
                resolver_registry: &ResolverRegistry::new(),
                resolver_policy: &ResolverPolicy::new(),
                evaluation_limits: XPathEvaluationLimits {
                    max_sequence_items: Some(limit),
                    ..self.limits
                },
                safety_policy_stamp: "cem-xslt-bundle/1",
                module_resolution: request.module_resolution,
            },
            request.control,
            request.scope,
        );
        match result {
            Err(diagnostics) => invocation_failure(&request, diagnostics),
            Ok(result) => {
                if result
                    .sequence
                    .items
                    .iter()
                    .any(|value| !item_has_type(value, ParamType::Any))
                {
                    return request.raise(
                        "cem.xslt.bundle_result",
                        "function items cannot cross the CEM-QL return boundary",
                    );
                }
                ItemStream::from_items(
                    result
                        .sequence
                        .items
                        .into_iter()
                        .map(XPathQueryItem::wrap)
                        .collect(),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cem_ml::{
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
        validation::xslt::{xslt_stylesheet_ast_from_source_bytes, XsltSourceValidationRequest},
    };

    #[test]
    fn callback_observes_the_callers_cancelled_operation() {
        let (stylesheet, diagnostics) = xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
            bytes: br#"<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/"><xsl:value-of select="1"/></xsl:template></xsl:stylesheet>"#,
            source_uri: "memory:cancel.xslt", content_type: Some("application/xslt+xml"),
        });
        assert!(diagnostics.is_empty());
        let expression = Arc::new(
            stylesheet
                .unwrap()
                .xpath_expressions
                .pop()
                .unwrap()
                .expression,
        );
        let function = Installed {
            binding: ProgramBinding {
                stylesheet: 0,
                focus: BundleFocus::Absent,
                group_context: false,
                variables: vec![],
                hash: String::new(),
            },
            expression,
            limits: Default::default(),
        };
        let control = OperationControl::default();
        control.cancel_root(None, None).unwrap();
        let result = function.call(NativeQueryRequest {
            arguments: &[],
            current_item: None,
            query_scope: crate::eval::QueryContextScope(0),
            source_map: &Default::default(),
            control: &control,
            scope: ROOT_EXECUTION_SCOPE_ID,
            max_result_items: 10,
            module_resolution: None,
        });
        assert!(
            matches!(result.error, Some(EvalError::Unsupported(_))),
            "{result:?}"
        );
        assert!(result.items.is_empty());
        assert_eq!(result.diagnostics[0].code, "cem.xpath.control_failure");
        assert_eq!(
            result.diagnostics[0].uri.as_deref(),
            Some("memory:cancel.xslt")
        );
    }
}
