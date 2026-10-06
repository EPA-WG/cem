//! Model routing retains placements and original arenas; it never expands source.
use super::*;

pub(super) fn validate<H: InputReferenceHost>(
    report: &mut StructuralInputValidation<H::Node>,
    node_models: &[usize],
    models: &RegionModels<'_>,
    host: &H,
    evaluator: &dyn SchemaBehaviorEvaluator,
) {
    if !report.complete {
        return;
    }
    for node in &mut report.nodes {
        node.declaring_schema = host.declaring_schema(&node.source);
    }
    let mut domains: Vec<Vec<usize>> = (0..models.len()).map(|_| vec![]).collect();
    for (placement, model) in node_models.iter().enumerate() {
        domains[*model].push(placement);
    }
    let mut diagnostics = vec![];
    let mut complete = true;
    for (model_index, placements) in domains.iter().enumerate() {
        let model = models.model(model_index);
        diagnostics.extend(evaluator.compile_model(model));
        let behavior = evaluator.validate_retained_region(
            RetainedBehaviorRegion {
                structure: report.structure(),
                placements,
            },
            model,
        );
        complete &= behavior.complete;
        diagnostics.extend(behavior.diagnostics);
    }
    report.complete &= complete;
    report.diagnostics.extend(diagnostics);
}
