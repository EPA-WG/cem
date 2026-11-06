//! One controlled render/application pass over a prepared native inline island.
use crate::{render::RenderPlan, ssr_island::PreparedInlineIsland};
use cem_ml::{
    diagnostics::Diagnostic, operation_control::OperationControl,
    parser::tree_apply::CemTreeApplication, ssr_artifact::CemSsrPageArtifact,
};
use std::sync::Arc;
#[derive(Debug)]
pub struct AppliedInlineDce {
    input: Arc<PreparedInlineIsland>,
    page: CemSsrPageArtifact,
    plan: Arc<RenderPlan>,
    projected: Arc<RenderPlan>,
    application: CemTreeApplication,
    committed: Arc<crate::instance_island::host_commit::CommittedHostIsland>,
}
impl AppliedInlineDce {
    pub fn committed_island(&self) -> &crate::instance_island::host_commit::CommittedHostIsland {
        &self.committed
    }

    pub fn input(&self) -> &Arc<PreparedInlineIsland> {
        &self.input
    }
    pub fn page(&self) -> &CemSsrPageArtifact {
        &self.page
    }
    pub fn plan(&self) -> &Arc<RenderPlan> {
        &self.plan
    }
    pub fn projected_plan(&self) -> &Arc<RenderPlan> {
        &self.projected
    }
    pub fn application(&self) -> &CemTreeApplication {
        &self.application
    }
}
pub fn render_and_apply_inline_dce(
    input: PreparedInlineIsland,
    control: &OperationControl,
) -> Result<AppliedInlineDce, Vec<Diagnostic>> {
    use crate::{
        eval::QueryContextScope,
        render,
        ssr_preparation::{check, problem},
    };
    use cem_ml::{
        operation_control::ROOT_EXECUTION_SCOPE_ID,
        parser::tree_apply::{apply_element, CemElementEdit},
        value::{CemReference, CemValueNode},
    };
    let page = input.prepared().page();
    let source = &input.prepared().elements().tree;
    let instance = input.prepared().instance_node();
    let fail = |code, message| problem(page, Some(source), Some(instance), code, message);
    check(page, control)?;
    input
        .prepared()
        .elements()
        .validate_html_text_projection()
        .map_err(|_| {
            fail(
                "cem.ssr.source_projection_unsupported",
                "Source text requires an unsupported HTML decoding profile",
            )
        })?;
    let mut usage = page.usage();
    usage.render_passes = usage
        .render_passes
        .checked_add(1)
        .ok_or_else(|| fail("cem.ssr.settle_budget", "Render count overflow"))?;
    if usage.render_passes > page.policy().max_render_passes {
        return Err(fail("cem.ssr.settle_budget", "No render pass remains"));
    }
    let plan = Arc::new(render::render_compiled_template_with_control(
        input.prepared().declaration_owner().artifact(),
        input.bindings(),
        control,
        ROOT_EXECUTION_SCOPE_ID,
    ));
    check(page, control)?;
    if plan
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        let mut errors = fail(
            "cem.ssr.render_failed",
            "Native retained template evaluation failed",
        );
        errors.extend(
            plan.diagnostics
                .iter()
                .filter(|d| d.severity.is_hard_violation())
                .take(255)
                .cloned(),
        );
        return Err(errors);
    }
    let anchor = crate::eval::RetainedCemNode::new(source.clone(), instance).unwrap();
    let committed = match crate::instance_island::host_commit::prepare_host_commit(
        input.template(),
        anchor.clone(),
        input.instance_id(),
        &plan.host_attribute_updates,
        control,
    ) {
        Ok(state) => Arc::new(state),
        Err(error) => {
            check(page, control)?;
            return Err(fail(
                if error == crate::instance_island::BOUNDS {
                    "cem.ssr.host_commit_bounds"
                } else {
                    "cem.ssr.host_commit_refused"
                },
                "Shared canonical host/island commit refused",
            ));
        }
    };
    check(page, control)?;
    let limits = crate::instance_island::LIMITS;
    let plan_with_ids = render::project_element_reference_ids(
        &plan,
        input.instance_id(),
        &limits,
        control,
        ROOT_EXECUTION_SCOPE_ID,
    )
    .map_err(|e| e.diagnostics)?;
    let mut projected = render::project_render_plan_with_control(
        &plan_with_ids,
        QueryContextScope(0),
        &limits,
        control,
        ROOT_EXECUTION_SCOPE_ID,
    )
    .map_err(|e| fail(e.code(), "Native render projection refused"))?;
    admit_output(&mut projected.nodes).map_err(|_| {
        fail(
            "cem.ssr.output_profile_unsupported",
            "Generated output requires an unsupported participant, capability or HTML profile",
        )
    })?;
    let anchor = crate::eval::RetainedCemNode::new(source.clone(), instance).unwrap();
    let roots =
        crate::eval::output::output_nodes_with_source(projected.nodes.clone(), anchor).items;
    crate::instance_island::admit_payload_roots(&roots, control).map_err(|_| {
        fail(
            "cem.ssr.output_profile_unsupported",
            "Generated payload/source or sensitive-input admission refused",
        )
    })?;
    check(page, control)?;
    let boundary_source = source.node_source_map(instance).unwrap_or_default();
    let island = render::RenderPlan {
        nodes: vec![CemValueNode::Reference {
            reference: CemReference::new(vec![committed.template().clone()]),
            source_map: boundary_source.clone(),
        }],
        host_attribute_updates: vec![],
        diagnostics: vec![],
    };
    let island = render::project_render_plan_with_control(
        &island,
        QueryContextScope(0),
        &limits,
        control,
        ROOT_EXECUTION_SCOPE_ID,
    )
    .map_err(|e| fail(e.code(), "Canonical island projection refused"))?;
    if island.nodes.len() != 1 {
        return Err(fail(
            "cem.ssr.island_projection_invalid",
            "Expected exactly one projected island template",
        ));
    }
    let mut children = island.nodes;
    children.push(CemValueNode::Comment {
        text: "cem-render-start".into(),
        source_map: boundary_source.clone(),
    });
    children.extend(projected.nodes.iter().cloned());
    children.push(CemValueNode::Comment {
        text: "cem-render-end".into(),
        source_map: boundary_source,
    });
    let page = page.clone();
    let source = source.clone();
    let input = Arc::new(input);
    let projected = Arc::new(projected);
    let retained: Arc<dyn std::any::Any + Send + Sync> = Arc::new((
        input.clone(),
        plan.clone(),
        projected.clone(),
        committed.clone(),
    ));
    let application = apply_element(
        source,
        instance,
        CemElementEdit {
            children: Arc::new(children),
            attributes: committed.writes().clone(),
        },
        retained,
        cem_ml::value::artifact::CemValueArtifactLimits {
            max_bytes: 16 * 1024 * 1024,
            max_values: 65_536,
            max_depth: 64,
        },
        control,
    )
    .map_err(|e| {
        crate::ssr_preparation::problem(
            &page,
            None,
            None,
            &e.code,
            "Native CEM AST replacement refused",
        )
    })?;
    check(&page, control)?;
    let page = page
        .into_applied_dom(&application, usage, plan.diagnostics.clone())
        .map_err(|e| {
            crate::ssr_preparation::problem(
                input.prepared().page(),
                None,
                None,
                e.code(),
                "Native partial artifact refused application",
            )
        })?;
    Ok(AppliedInlineDce {
        input,
        page,
        plan,
        projected,
        application,
        committed,
    })
}
fn admit_output(nodes: &mut [crate::render::RenderPlanNode]) -> Result<(), ()> {
    use cem_ml::schema::registry::HTML_NAMESPACE_URI;
    const XMLNS_NAMESPACE_URI: &str = "http://www.w3.org/2000/xmlns/";
    use cem_ml::value::CemValueNode;
    for node in nodes {
        match node {
            CemValueNode::Element {
                tag,
                namespace,
                qualified_name,
                attributes,
                children,
                ..
            } => {
                if namespace
                    .as_deref()
                    .is_some_and(|ns| ns != HTML_NAMESPACE_URI)
                    || qualified_name
                        .as_ref()
                        .is_some_and(|name| name.contains(':'))
                {
                    return Err(());
                }
                *namespace = Some(HTML_NAMESPACE_URI.into());
                if attributes.iter().any(|a| {
                    a.namespace
                        .as_deref()
                        .is_some_and(|ns| !ns.is_empty() && ns != XMLNS_NAMESPACE_URI)
                        || a.contract.is_some()
                }) {
                    return Err(());
                }
                let pairs = attributes
                    .iter()
                    .map(|a| (a.name.as_str(), a.value.as_str()))
                    .collect::<Vec<_>>();
                cem_ml::validation::html::validate_passive_html_projection(tag, &pairs)
                    .map_err(|_| ())?;
                const VOID: &[&str] = &[
                    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
                    "param", "source", "track", "wbr",
                ];
                if VOID.contains(&tag.as_str()) && !children.is_empty() {
                    return Err(());
                }
                admit_output(children)?;
            }
            CemValueNode::Text { .. } | CemValueNode::Comment { .. } => {}
            _ => return Err(()),
        }
    }
    Ok(())
}
