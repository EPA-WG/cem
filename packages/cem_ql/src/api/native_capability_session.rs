//! Transient native sources/capture never pass through materialized CEMV.
//! Each immutable session is prepared under fresh host-issued context and grants.
use super::{
    element_references::{
        ElementReferenceBinding, ElementReferenceExecution, ElementReferenceSource,
    },
    evaluate_expression, StandaloneExpressionBinding, StandaloneExpressionContext,
};
use crate::{
    eval::{portable::export_values, ItemStream},
    render::{render_compiled_template, RenderPlan, TemplateArtifact, TemplateData},
    types::Type,
};
use cem_ml::value::artifact::CemValueArtifactLimits;

pub struct NativeCapabilitySession {
    // Retains all original owners, captured lexical scopes and directed grants.
    execution: ElementReferenceExecution,
    data: TemplateData,
    values: ItemStream,
    limits: CemValueArtifactLimits,
    suggestions: std::sync::OnceLock<
        Result<
            std::sync::Arc<crate::suggestions::SuggestionsPlan>,
            crate::suggestions::SuggestionsError,
        >,
    >,
}
impl NativeCapabilitySession {
    pub fn prepare(
        sources: Vec<ElementReferenceSource>,
        requesting: usize,
        bindings: &[ElementReferenceBinding],
        grants: &[(usize, usize)],
        mut data: TemplateData,
        select: &str,
        resolve: bool,
        limits: CemValueArtifactLimits,
    ) -> Result<Self, String> {
        if limits.max_bytes == 0
            || limits.max_values == 0
            || limits.max_depth == 0
            || select.len() > limits.max_bytes
        {
            return Err("Invalid native session bounds".into());
        }
        let source = sources
            .get(requesting)
            .ok_or("Unknown requesting source")?
            .source
            .clone();
        let captured_sources = sources.clone();
        let mut execution =
            ElementReferenceExecution::prepare(sources, requesting, bindings, grants, &mut data)?;
        let context = context(&data);
        let selected = source
            .evaluate(select, &context)
            .map_err(|e| e.message)?
            .result;
        check_values(&selected, limits.max_values)?;
        execution.admit(&selected)?;
        let values = if resolve {
            execution.consume(selected)?
        } else {
            selected
        };
        check_values(&values, limits.max_values)?;
        if values.items.iter().any(|item| {
            item.view()
                .is_none_or(|v| v.kind() != crate::eval::QueryItemViewKind::Node)
        }) {
            return Err("Native source selection must contain typed nodes".into());
        }
        let values = execution.complete_selected_names(&captured_sources, &data, values)?;
        Ok(Self {
            execution,
            data,
            values,
            limits,
            suggestions: Default::default(),
        })
    }
    pub fn len(&self) -> usize {
        self.values.items.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.items.is_empty()
    }
    fn input(&self, index: Option<usize>) -> Result<ItemStream, String> {
        match index {
            None => Ok(self.values.clone()),
            Some(index) => self
                .values
                .items
                .get(index)
                .cloned()
                .map(ItemStream::once)
                .ok_or("Unknown native session source index".into()),
        }
    }
    /// Explicit query over retained selection. Descendant references stay authored.
    pub fn evaluate(&self, expression: &str, index: Option<usize>) -> Result<ItemStream, String> {
        if expression.len() > self.limits.max_bytes {
            return Err("Native query byte limit exceeded".into());
        }
        let context = context(&self.data).with_input(self.input(index)?, Type::Any);
        let values = evaluate_expression(expression, &context)
            .map_err(|e| e.message)?
            .result;
        check_values(&values, self.limits.max_values)?;
        self.execution.admit(&values)?;
        Ok(values)
    }
    /// Materialized presentation boundary. CEMV still rejects executable capture.
    pub fn export(&self, expression: &str, index: Option<usize>) -> Result<Vec<u8>, String> {
        export_values(&self.evaluate(expression, index)?, &self.limits).map_err(|e| e.to_string())
    }
    /// Label input stays native; no export/reimport can sever its scopes/owner.
    pub fn render(
        &self,
        template: &TemplateArtifact,
        index: Option<usize>,
    ) -> Result<RenderPlan, String> {
        let mut data = self.data.clone();
        data.bindings.insert("input".into(), self.input(index)?);
        let plan = render_compiled_template(template, &data);
        if plan
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(plan
                .diagnostics
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
                .join("; "));
        }
        Ok(plan)
    }
    pub fn limits(&self) -> &CemValueArtifactLimits {
        &self.limits
    }
    /// Adapt once per retained source revision, caching even an invalid revision.
    pub fn suggestions(
        &self,
    ) -> Result<
        std::sync::Arc<crate::suggestions::SuggestionsPlan>,
        crate::suggestions::SuggestionsError,
    > {
        self.suggestions
            .get_or_init(|| {
                crate::suggestions::SuggestionsPlan::prepare(&self.values, self.limits.clone())
            })
            .clone()
    }
    fn suggestions_frame(
        &self,
        config: &crate::suggestions::SuggestionsConfig,
        index: Option<usize>,
    ) -> Result<
        (
            TemplateData,
            crate::suggestions::SuggestionsView,
            ItemStream,
        ),
        crate::suggestions::SuggestionsError,
    > {
        let view = self.suggestions()?.view(config)?;
        let mut data = self.data.clone();
        let root = ItemStream::once(view.root());
        data.bind_native_slice("suggestions", root.clone())
            .map_err(suggestions_error)?;
        let input = match index {
            Some(index) => ItemStream::once(view.row(index)?),
            None => root,
        };
        Ok((data, view, input))
    }
    pub fn evaluate_suggestions(
        &self,
        config: &crate::suggestions::SuggestionsConfig,
        expression: &str,
        index: Option<usize>,
    ) -> Result<ItemStream, crate::suggestions::SuggestionsError> {
        if expression.len() > self.limits.max_bytes {
            return Err(suggestions_error("Native query byte limit exceeded"));
        }
        let (data, view, input) = self.suggestions_frame(config, index)?;
        let values = evaluate_expression(expression, &context(&data).with_input(input, Type::Any))
            .map_err(|e| suggestions_error(e.message))?
            .result;
        check_values(&values, self.limits.max_values).map_err(suggestions_error)?;
        for value in &values.items {
            if !view.owns(value) {
                self.execution
                    .admit(&ItemStream::once(value.clone()))
                    .map_err(suggestions_error)?;
            }
        }
        Ok(values)
    }
    pub fn render_suggestion(
        &self,
        config: &crate::suggestions::SuggestionsConfig,
        template: &TemplateArtifact,
        index: Option<usize>,
    ) -> Result<RenderPlan, crate::suggestions::SuggestionsError> {
        let (mut data, _, input) = self.suggestions_frame(config, index)?;
        data.bindings.insert("suggestion".into(), input.clone());
        data.bindings.insert("input".into(), input);
        let plan = render_compiled_template(template, &data);
        if plan
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(suggestions_error(
                plan.diagnostics
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        Ok(plan)
    }
    pub fn render_suggestion_group(
        &self,
        config: &crate::suggestions::SuggestionsConfig,
        template: &TemplateArtifact,
        index: usize,
    ) -> Result<RenderPlan, crate::suggestions::SuggestionsError> {
        let (mut data, view, _) = self.suggestions_frame(config, None)?;
        let input = ItemStream::once(view.group(index)?);
        data.bindings.insert("group".into(), input.clone());
        data.bindings.insert("input".into(), input);
        let plan = render_compiled_template(template, &data);
        if plan
            .diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation())
        {
            return Err(suggestions_error(
                plan.diagnostics
                    .iter()
                    .map(|d| d.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        Ok(plan)
    }
}
fn suggestions_error(message: impl Into<String>) -> crate::suggestions::SuggestionsError {
    crate::suggestions::SuggestionsError {
        code: "cem.suggestions.consumer",
        message: message.into(),
        source: Default::default(),
    }
}
fn context(data: &TemplateData) -> StandaloneExpressionContext {
    StandaloneExpressionContext {
        bindings: data
            .bindings
            .iter()
            .map(|(name, values)| {
                (
                    name.clone(),
                    StandaloneExpressionBinding::any(values.clone()),
                )
            })
            .collect(),
        native_functions: data.native_functions.clone(),
        ..Default::default()
    }
}
fn check_values(values: &ItemStream, ceiling: usize) -> Result<(), String> {
    if let Some(error) = &values.error {
        return Err(format!("Native query failed: {error:?}"));
    }
    if values.items.len() > ceiling {
        return Err("Native selection value limit exceeded".into());
    }
    Ok(())
}
