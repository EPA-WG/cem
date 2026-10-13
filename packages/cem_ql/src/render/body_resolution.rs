//! Attach lexical native body authority while compiling the immutable closure.
//! Cloned call, match and hook bodies retain it; serialized expressions do not.
use super::*;
use cem_ml::module_resolution::CemModuleUrlResolutionCapability;

pub(super) fn bind_body(
    artifact: &mut TemplateArtifact,
    uri: &str,
    resolutions: Option<&BTreeMap<String, CemModuleUrlResolutionCapability>>,
) {
    let Some(resolutions) = resolutions else {
        return;
    };
    let Some(capability) = resolutions.get(uri) else {
        artifact.diagnostics.push(render_diagnostic(
            "cem.ql.template.body_context_missing",
            format!("missing native body context for {uri}"),
            0,
            SourceMapStack::default(),
        ));
        return;
    };
    let bind = |expression: &mut CompiledTemplateExpression| {
        expression.resolution = Some(capability.clone());
    };
    let mut pending: Vec<_> = artifact.nodes.iter_mut().collect();
    while let Some(node) = pending.pop() {
        match node {
            TemplateNode::Element {
                attributes,
                children,
                ..
            }
            | TemplateNode::Result {
                attributes,
                children,
                ..
            } => {
                for attribute in attributes {
                    match &mut attribute.value {
                        Some(TemplateAttributeValue::Expression(e)) => bind(e),
                        Some(TemplateAttributeValue::Template(parts)) => {
                            for part in parts {
                                if let TemplateAttributePart::Expression(e) = part {
                                    bind(e);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                pending.extend(children);
            }
            TemplateNode::Expression(e) => bind(e),
            TemplateNode::If { test, children, .. } => {
                if let Some(e) = test {
                    bind(e);
                }
                pending.extend(children);
            }
            TemplateNode::ForEach {
                select, children, ..
            } => {
                if let Some(e) = select {
                    bind(e);
                }
                pending.extend(children);
            }
            TemplateNode::Variable { select, .. } | TemplateNode::ProjectPayload { select, .. } => {
                if let Some(e) = select {
                    bind(e);
                }
            }
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    if let Some(e) = &mut branch.test {
                        bind(e);
                    }
                    pending.extend(&mut branch.children);
                }
            }
            TemplateNode::Text { .. } | TemplateNode::Comment { .. } => {}
        }
    }
}

impl PlanRenderer<'_> {
    pub(super) fn evaluate_expression_query(
        &mut self,
        expression: &CompiledTemplateExpression,
    ) -> ItemStream {
        let Some(query) = &expression.query else {
            return ItemStream::empty();
        };
        let Some(capability) = &expression.resolution else {
            return self.evaluate_query(query);
        };
        let previous = self
            .evaluation_context
            .module_resolution
            .replace(capability.clone());
        let result = self.evaluate_query(query);
        self.evaluation_context.module_resolution = previous;
        result
    }
}
