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
    publications: std::cell::RefCell<
        std::collections::BTreeMap<String, (crate::suggestions::SuggestionsView, usize)>,
    >,
    publication_keys: std::cell::RefCell<std::collections::BTreeSet<String>>,
    publication_bytes: std::cell::Cell<usize>,
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
            publications: Default::default(),
            publication_keys: Default::default(),
            publication_bytes: Default::default(),
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
        let mut data = self.data.clone();
        let view = self.bind_suggestions_frame(&mut data, config)?;
        let root = ItemStream::once(view.root());
        let input = match index {
            Some(index) => ItemStream::once(view.row(index)?),
            None => root,
        };
        Ok((data, view, input))
    }
    /// Publish directly into a consumer's native frame without CEMV export/reload.
    /// The frame retains the view and original source edges after this call.
    pub fn bind_suggestions_frame(
        &self,
        data: &mut TemplateData,
        config: &crate::suggestions::SuggestionsConfig,
    ) -> Result<crate::suggestions::SuggestionsView, crate::suggestions::SuggestionsError> {
        let view = self.suggestions()?.view(config)?;
        data.bind_reserved_native_slice("suggestions", ItemStream::once(view.root()))
            .map_err(suggestions_error)?;
        Ok(view)
    }
    /// One immutable view stays on this execution owner across consumer jobs.
    pub fn publish_suggestions(
        &self,
        key: &str,
        config: &crate::suggestions::SuggestionsConfig,
    ) -> Result<crate::suggestions::SuggestionsView, crate::suggestions::SuggestionsError> {
        if key.is_empty() || key.len() > 1024 || key.len() > self.limits.max_bytes {
            return Err(suggestions_error("Invalid native publication identity"));
        }
        if self.publication_keys.borrow().contains(key) {
            return Err(suggestions_error(
                "Native publication identity was already issued",
            ));
        }
        if self.publications.borrow().len() >= self.limits.max_values
            || self.publication_keys.borrow().len() >= self.limits.max_values.min(100_000)
        {
            return Err(suggestions_error("Native publication capacity exceeded"));
        }
        let plan = self.suggestions()?;
        // Retained control strings, match flags and active/retired keys share one byte budget.
        let charge = serde_json::to_vec(config)
            .map_err(|error| suggestions_error(error.to_string()))?
            .len()
            .checked_add(plan.len())
            .and_then(|n| n.checked_add(key.len() + 128))
            .ok_or_else(|| suggestions_error("Native publication byte limit exceeded"))?;
        let total = self
            .publication_bytes
            .get()
            .checked_add(charge)
            .and_then(|n| n.checked_add(key.len()))
            .filter(|n| *n <= self.limits.max_bytes)
            .ok_or_else(|| suggestions_error("Native publication byte limit exceeded"))?;
        let view = plan.view(config)?;
        self.publication_keys.borrow_mut().insert(key.into());
        self.publications
            .borrow_mut()
            .insert(key.into(), (view.clone(), charge));
        self.publication_bytes.set(total);
        Ok(view)
    }
    pub fn suggestions_publication(
        &self,
        key: &str,
    ) -> Result<crate::suggestions::SuggestionsView, crate::suggestions::SuggestionsError> {
        self.publications
            .borrow()
            .get(key)
            .map(|(view, _)| view.clone())
            .ok_or_else(|| {
                suggestions_error("Native publication is not retained; reacquire authority")
            })
    }
    pub fn release_suggestions(&self, key: &str) -> bool {
        if let Some((_, charge)) = self.publications.borrow_mut().remove(key) {
            self.publication_bytes
                .set(self.publication_bytes.get() - charge);
            true
        } else {
            false
        }
    }
    /// Scalar consumer control enters here; retained source edges never leave their owner.
    pub fn render_suggestions_frame(
        &self,
        key: &str,
        template: &TemplateArtifact,
        mut data: TemplateData,
    ) -> Result<RenderPlan, crate::suggestions::SuggestionsError> {
        let view = self.suggestions_publication(key)?;
        data.bind_reserved_native_slice("suggestions", ItemStream::once(view.root()))
            .map_err(suggestions_error)?;
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
