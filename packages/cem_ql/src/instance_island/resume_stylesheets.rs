//! Rebuild resource-free stylesheet occurrences from the admitted native inputs.
use super::{compare, fail, island, whitespace, HTML_NAMESPACE_URI};
use crate::{
    eval::Item,
    render::{DomStylesheetSource, RenderPlanNode, TemplateArtifact},
    retained_template::RetainedTemplate,
    ssr_stylesheets::stylesheet_markup,
};
use cem_ml::{
    css_emission::CssManagedScope,
    css_imports::{CssImportClosure, CssImportLimits},
    module_resolution::{
        CemModuleUrlResolutionCapability, CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    source_map::SourceMapStack,
};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct PayloadStyles {
    sources: Vec<DomStylesheetSource>,
    pub maps: Vec<SourceMapStack>,
}

impl PayloadStyles {
    pub fn push(&mut self, style: &Item) -> Result<(), String> {
        if self.sources.len() >= 64 || !island::attrs(style)?.is_empty() {
            return Err(fail());
        }
        let mut css = String::new();
        for child in island::children(style)? {
            if !matches!(super::kind(&child).as_deref(), Some("text" | "whitespace")) {
                return Err(fail());
            }
            css.push_str(&super::text(&child)?);
        }
        if css.len() > 32_768 {
            return Err(fail());
        }
        self.maps.push(style.source_map().ok_or_else(fail)?);
        self.sources.push(DomStylesheetSource {
            css,
            scope: None,
            content_type: None,
        });
        Ok(())
    }

    pub fn verify(
        &self,
        instance: &str,
        suffix: &[Item],
        c: &OperationControl,
    ) -> Result<bool, String> {
        let mut owner = RetainedTemplate::new_instance_stylesheets(&self.sources, instance)?;
        if owner.artifact().stylesheets.len() != self.sources.len()
            || !owner.artifact().diagnostics.is_empty()
        {
            return Err(fail());
        }
        let expected = emit(&mut owner, instance, false, None, c)?;
        verify(&expected, suffix, c)?;
        Ok(!expected.is_empty())
    }
}

pub(super) fn declaration_scope(declaration: &Item) -> Result<Option<String>, String> {
    Ok(island::attrs(declaration)?
        .remove("scope")
        .filter(|scope| !scope.is_empty()))
}

pub(super) fn verify_declaration(
    declaration: Option<&Item>,
    tag: &str,
    scope: Option<&str>,
    artifact: &TemplateArtifact,
    c: &OperationControl,
) -> Result<(), String> {
    let Some(declaration) = declaration else {
        return if artifact.stylesheets.is_empty() {
            Ok(())
        } else {
            Err(fail())
        };
    };
    let actual = island::elements(declaration)?
        .into_iter()
        .filter(|node| island::is_element(node, "style", HTML_NAMESPACE_URI))
        .collect::<Vec<_>>();
    let mut owner = RetainedTemplate::new(TemplateArtifact {
        nodes: vec![],
        stylesheets: artifact.stylesheets.clone(),
        module_map: None,
        diagnostics: vec![],
    });
    verify(&emit(&mut owner, tag, true, scope, c)?, &actual, c)
}

fn emit(
    owner: &mut RetainedTemplate,
    id: &str,
    declaration: bool,
    shared_scope: Option<&str>,
    c: &OperationControl,
) -> Result<Vec<RenderPlanNode>, String> {
    if owner.artifact().stylesheets.len() > 64 {
        return Err(fail());
    }
    let capability = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new()),
        CemResolutionContextHandle::new("cem-ssr-closed-css"),
    );
    (0..owner.artifact().stylesheets.len())
        .map(|index| {
            c.check_scope(ROOT_EXECUTION_SCOPE_ID).map_err(|_| fail())?;
            let source = &owner.artifact().stylesheets[index];
            let scope = if declaration {
                match source.scope.as_deref() {
                    None => CssManagedScope::Private {
                        tag: id.into(),
                        context: None,
                    },
                    Some(name) if Some(name) == shared_scope => CssManagedScope::Shared {
                        name: name.into(),
                        context: None,
                    },
                    Some(_) => return Err(fail()),
                }
            } else {
                CssManagedScope::Instance
            };
            let closure = CssImportClosure::new(
                source.css_tree().clone(),
                "cem-ssr-css://inline/",
                capability.clone(),
                CssImportLimits {
                    max_sheets: 1,
                    max_depth: 0,
                },
                c.abort_signal().clone(),
            )
            .map_err(|_| fail())?;
            let record = owner.retain_closed_ssr_stylesheet(
                "resume",
                index,
                Arc::new(closure),
                &scope,
                id,
            )?;
            if !record.emission()?.diagnostics.is_empty() {
                return Err(fail());
            }
            stylesheet_markup(&record, index, declaration, &SourceMapStack::default())
        })
        .collect()
}

fn verify(
    expected: &[RenderPlanNode],
    actual: &[Item],
    c: &OperationControl,
) -> Result<(), String> {
    let actual = actual
        .iter()
        .filter(|node| !whitespace(node))
        .cloned()
        .collect::<Vec<_>>();
    compare(expected, &actual, 0, c, None, true)
}
