//! Side-effect-free resource-use inspection of native compiled expressions.
use crate::ir::{CompiledQuery, IrId};
use cem_ml::source_map::SourceMapStack;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceUseKind {
    ContentRead,
    ModuleUrl,
}
#[derive(Debug, Clone)]
pub struct ResourceUse {
    pub node_id: IrId,
    pub pipeline_step: Option<usize>,
    pub kind: ResourceUseKind,
    pub literal_argument: Option<String>,
    pub literal_referrer: Option<String>,
    pub source: SourceMapStack,
}
#[derive(Debug)]
pub struct ResourceInspection {
    pub uses: Vec<ResourceUse>,
    pub blockers: Vec<String>,
    pub body_analysis_required: bool,
}
impl Default for ResourceInspection {
    fn default() -> Self {
        Self {
            uses: Vec::new(),
            blockers: Vec::new(),
            body_analysis_required: true,
        }
    }
}
/// Inspect the entire compiled arena, including branches and function bodies.
/// No evaluation, constant folding, URI joining or acquisition occurs. Literal
/// arguments are candidates only. The budget counts both IR nodes and pipeline
/// steps, and truncation remains explicit. Absence of findings is not closure.
pub fn inspect_query_resources(query: &CompiledQuery, budget: usize) -> ResourceInspection {
    use crate::ir::{IrNode, IrStep};
    let mut result = ResourceInspection {
        body_analysis_required: true,
        ..Default::default()
    };
    if query.tree.node(query.tree.root).is_none() {
        block(&mut result, "invalid-query-root");
        return result;
    }
    let mut remaining = budget;
    'nodes: for (index, node) in query.tree.nodes.iter().enumerate() {
        if !charge(&mut remaining, &mut result) {
            break;
        }
        let id = IrId(index as u32);
        match node {
            IrNode::StdlibCall { module, name, args } => inspect_call(
                query,
                id,
                None,
                &module.0,
                &name.local,
                args.first().copied(),
                args.get(1).copied(),
                &mut result,
            ),
            IrNode::Pipeline { source, steps } => {
                for (step_index, step) in steps.iter().enumerate() {
                    if !charge(&mut remaining, &mut result) {
                        break 'nodes;
                    }
                    match step {
                        IrStep::NamedStdlib { module, name, args } => inspect_call(
                            query,
                            id,
                            Some(step_index),
                            &module.0,
                            &name.local,
                            (step_index == 0).then_some(*source),
                            args.first().copied(),
                            &mut result,
                        ),
                        IrStep::Method { .. } | IrStep::Named { .. } | IrStep::Lambda(_) => {
                            block(&mut result, "dynamic-or-host-pipeline-analysis-required")
                        }
                    }
                }
            }
            IrNode::Call { .. } | IrNode::FunctionRef(_) | IrNode::Lambda { .. } => {
                block(&mut result, "dynamic-or-function-body-analysis-required")
            }
            _ => {}
        }
    }
    result
}

fn charge(remaining: &mut usize, result: &mut ResourceInspection) -> bool {
    if *remaining == 0 {
        block(result, "inspection-budget-exceeded");
        false
    } else {
        *remaining -= 1;
        true
    }
}

#[allow(clippy::too_many_arguments)]
fn inspect_call(
    query: &CompiledQuery,
    id: IrId,
    step: Option<usize>,
    module: &str,
    name: &str,
    argument: Option<IrId>,
    referrer: Option<IrId>,
    result: &mut ResourceInspection,
) {
    let kind = match (module, name) {
        ("cem:stdlib/content-types", "read") => ResourceUseKind::ContentRead,
        ("cem:stdlib/modules", "module_url") => ResourceUseKind::ModuleUrl,
        // data:read parses supplied content; it is not a URI acquisition slot.
        _ => {
            block(result, "other-call-analysis-required");
            return;
        }
    };
    let literal_argument = literal(query, argument, result);
    if literal_argument.is_none() {
        block(result, "computed-resource-argument");
    }
    let literal_referrer = if kind == ResourceUseKind::ModuleUrl {
        block(result, "module-resolution-context-required");
        let value = literal(query, referrer, result);
        if referrer.is_some() && value.is_none() {
            block(result, "computed-module-referrer");
        }
        value
    } else {
        block(result, "resource-read-policy-required");
        None
    };
    let source = match query.tree.source_maps.get(id.0 as usize) {
        Some(source) => source.clone(),
        None => {
            block(result, "missing-source-provenance");
            SourceMapStack::default()
        }
    };
    result.uses.push(ResourceUse {
        node_id: id,
        pipeline_step: step,
        kind,
        literal_argument,
        literal_referrer,
        source,
    });
}

fn literal(
    query: &CompiledQuery,
    id: Option<IrId>,
    result: &mut ResourceInspection,
) -> Option<String> {
    match id.map(|id| query.tree.node(id)) {
        Some(Some(crate::ir::IrNode::LitString(value))) => Some(value.clone()),
        Some(None) => {
            block(result, "invalid-query-reference");
            None
        }
        _ => None,
    }
}
fn block(result: &mut ResourceInspection, reason: &str) {
    if !result.blockers.iter().any(|value| value == reason) {
        result.blockers.push(reason.into());
    }
}

#[derive(Debug)]
pub struct TemplateExpressionResourceInspection {
    pub template_uri: String,
    pub byte_offset: u64,
    pub host_source: SourceMapStack,
    pub inspection: ResourceInspection,
    // Retain the compiled native arena for later execution-context resolution;
    // never reconstruct IR from the candidate strings or source coordinates.
    query: Option<CompiledQuery>,
}
impl TemplateExpressionResourceInspection {
    pub fn query(&self) -> Option<&CompiledQuery> {
        self.query.as_ref()
    }
}

/// Retain the host embedding separately from expression-local IR provenance.
/// Uncompiled expressions remain blocked and are never reparsed by inspection.
pub fn inspect_template_expression_resources(
    template_uri: &str,
    expression: &crate::render::CompiledTemplateExpression,
    budget: usize,
) -> TemplateExpressionResourceInspection {
    let inspection = match &expression.query {
        Some(query) => inspect_query_resources(query, budget),
        None => ResourceInspection {
            body_analysis_required: true,
            blockers: vec!["uncompiled-template-expression".into()],
            ..Default::default()
        },
    };
    TemplateExpressionResourceInspection {
        template_uri: template_uri.into(),
        byte_offset: expression.byte_offset,
        host_source: expression.source_map.clone(),
        inspection,
        query: expression.query.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{compile_expression, StandaloneExpressionContext};
    fn query(source: &str) -> CompiledQuery {
        compile_expression(source, &StandaloneExpressionContext::default())
            .unwrap()
            .query
    }
    #[test]
    fn dependency_inspection_distinguishes_resource_reads_from_content_parsing() {
        let result =
            inspect_query_resources(&query(r#"ct:read("file:///missing/never-read.xml")"#), 100);
        assert_eq!(result.uses.len(), 1);
        assert_eq!(result.uses[0].kind, ResourceUseKind::ContentRead);
        assert_eq!(
            result.uses[0].literal_argument.as_deref(),
            Some("file:///missing/never-read.xml")
        );
        assert!(!result.uses[0].source.frames.is_empty());
        assert!(result.body_analysis_required);
        assert!(result.blockers.iter().any(|b| b.contains("policy")));
        let parsed =
            inspect_query_resources(&query(r#"data:read("<root/>", "application/xml")"#), 100);
        assert!(parsed.uses.is_empty());
        assert!(parsed.body_analysis_required);
    }
    #[test]
    fn dependency_inspection_records_module_resolution_without_claiming_fetch() {
        let result = inspect_query_resources(
            &query(r#"module:module_url("icons", "https://example.test/page.cem")"#),
            100,
        );
        assert_eq!(result.uses[0].kind, ResourceUseKind::ModuleUrl);
        assert_eq!(result.uses[0].literal_argument.as_deref(), Some("icons"));
        assert_eq!(
            result.uses[0].literal_referrer.as_deref(),
            Some("https://example.test/page.cem")
        );
        assert!(result.blockers.iter().any(|b| b.contains("context")));
    }
    #[test]
    fn dependency_inspection_pipeline_input_is_implicit_and_not_reused_after_transform() {
        let direct = inspect_query_resources(&query(r#""file:///a.xml".ct:read()"#), 100);
        assert_eq!(
            direct.uses[0].literal_argument.as_deref(),
            Some("file:///a.xml")
        );
        assert_eq!(direct.uses[0].pipeline_step, Some(0));
        let transformed =
            inspect_query_resources(&query(r#""file:///a.xml".str:trim().ct:read()"#), 100);
        assert_eq!(transformed.uses[0].literal_argument, None);
        assert!(transformed.blockers.iter().any(|b| b.contains("computed")));
    }
    #[test]
    fn dependency_inspection_visits_both_branches_and_nested_arguments() {
        let result = inspect_query_resources(
            &query(
                r#"if true { ct:read("file:///a.xml") } else { ct:read(module:module_url("b")) }"#,
            ),
            100,
        );
        assert_eq!(result.uses.len(), 3);
        assert_eq!(
            result
                .uses
                .iter()
                .filter(|item| item.kind == ResourceUseKind::ContentRead)
                .count(),
            2
        );
        assert!(result.uses.iter().any(
            |item| item.kind == ResourceUseKind::ContentRead && item.literal_argument.is_none()
        ));
    }
    #[test]
    fn dependency_inspection_budget_never_yields_a_complete_claim() {
        let result = inspect_query_resources(&query(r#"ct:read("file:///a.xml")"#), 0);
        assert!(result.uses.is_empty());
        assert!(result.blockers.iter().any(|b| b.contains("budget")));
        assert!(result.body_analysis_required);
    }
    #[test]
    fn dependency_inspection_preserves_template_host_and_blocks_uncompiled_slots() {
        let compiled = query(r#"ct:read("file:///a.xml")"#);
        let host_source = compiled.tree.source_maps[compiled.tree.root.0 as usize].clone();
        let mut expression = crate::render::CompiledTemplateExpression {
            resolution: None,
            source: r#"ct:read("file:///a.xml")"#.into(),
            query: Some(compiled),
            source_map: host_source.clone(),
            byte_offset: 27,
        };
        let result =
            inspect_template_expression_resources("studio://p/template.cem", &expression, 100);
        assert_eq!(result.template_uri, "studio://p/template.cem");
        assert_eq!(result.byte_offset, 27);
        assert_eq!(result.host_source, host_source);
        assert_eq!(result.inspection.uses.len(), 1);
        expression.query = None;
        let result =
            inspect_template_expression_resources("studio://p/template.cem", &expression, 100);
        assert!(result.inspection.body_analysis_required);
        assert_eq!(
            result.inspection.blockers,
            vec!["uncompiled-template-expression"]
        );
    }

    #[test]
    fn dependency_inspection_blocks_dynamic_calls_and_counts_pipeline_steps() {
        use crate::ir::{IrNode, IrStep};
        let mut compiled = query(r#"ct:read("file:///a.xml")"#);
        let read = compiled.tree.root;
        compiled.tree.nodes.push(IrNode::Call {
            callee: read,
            args: vec![],
        });
        let result = inspect_query_resources(&compiled, 100);
        assert!(result.blockers.iter().any(|b| b.contains("dynamic")));
        let name = crate::parser::QName {
            prefix: None,
            local: "opaque".into(),
            range: cem_ml::source::ByteRange::new(0, 1),
        };
        compiled.tree.nodes = vec![IrNode::Pipeline {
            source: crate::ir::IrId(0),
            steps: vec![
                IrStep::Named {
                    name,
                    binding: None,
                    args: vec![]
                };
                100
            ],
        }];
        compiled.tree.root = crate::ir::IrId(0);
        let result = inspect_query_resources(&compiled, 3);
        assert!(result.blockers.iter().any(|b| b.contains("budget")));
        assert!(result.body_analysis_required);
    }
    #[test]
    fn dependency_inspection_uses_resolved_modules_not_lexical_prefixes() {
        use crate::api::{compile, CompileContext};
        let alias = compile(
            r#"import "cem:stdlib/content-types" as resource
            resource:read("file:///a.xml")"#,
            &CompileContext::default(),
        )
        .unwrap();
        let result = inspect_query_resources(&alias, 100);
        assert_eq!(result.uses.len(), 1);
        assert_eq!(result.uses[0].kind, ResourceUseKind::ContentRead);
        let custom = compile(
            r#"declare function local:read(value as string) { value }
            local:read("file:///a.xml")"#,
            &CompileContext::default(),
        )
        .unwrap();
        let result = inspect_query_resources(&custom, 100);
        assert!(result.uses.is_empty());
        assert!(result.blockers.iter().any(|b| b.contains("function")));

        // Reserved content-type calls currently lower to the standard library
        // even when a declaration has the same QName. Follow executable IR.
        let reserved = compile(
            r#"declare function ct:read(value as string) { value }
            ct:read("file:///a.xml")"#,
            &CompileContext::default(),
        )
        .unwrap();
        let result = inspect_query_resources(&reserved, 100);
        assert_eq!(result.uses.len(), 1);
        assert_eq!(result.uses[0].kind, ResourceUseKind::ContentRead);
    }
}
