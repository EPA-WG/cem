use super::*;

impl crate::native::TemplateQueryHost for PlanRenderer<'_> {
    fn apply_templates(
        &mut self,
        values: ItemStream,
        mode: &str,
        source: &SourceMapStack,
    ) -> ItemStream {
        let diagnostics_start = self.diagnostics.len();
        self.recovery_depth += 1;
        // A called template constructs ordinary native output. Capture mode only
        // applies to the hook body that declared it, never to its callees.
        let capture = self.capture_depth.take();
        let mut buffer = ResultBuffer::default();
        self.dispatch_values(values, mode, source, &mut buffer, &mut Vec::new());
        self.capture_depth = capture;
        let mut result = self.hook_result_values(buffer, source);
        self.recovery_depth -= 1;
        result.diagnostics = self.diagnostics.split_off(diagnostics_start);
        if self.control_failed || self.failure.is_some() {
            result.items.clear();
            result.error = Some(self.failure.take().map(|f| f.error).unwrap_or(
                EvalError::BudgetExceeded(crate::eval::BudgetAxis::CallDepth),
            ));
        }
        result
    }
}
