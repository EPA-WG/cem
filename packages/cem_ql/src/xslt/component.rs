//! Explicit component state mappings. Only authoring/control metadata is
//! serialized; expressions and retained documents are evaluated natively.
use super::{compiler::*, *};
use crate::{
    api::{
        compile_expression, evaluate, EvaluationContext, StandaloneExpressionBinding,
        StandaloneExpressionContext,
    },
    eval::{Item, ItemStream, QueryItemViewKind},
    ir::CompiledQuery,
};
use cem_ml::diagnostics::{Diagnostic, Severity};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XsltScalarMapping {
    pub name: String,
    pub select: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XsltSourceModule {
    pub parent_uri: String,
    pub href: String,
    pub uri: String,
    pub source: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XsltComponentOptions {
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub parameters: Vec<XsltScalarMapping>,
    #[serde(default)]
    pub modules: Vec<XsltSourceModule>,
}

pub struct XsltComponent {
    pub bundle: XsltBundle,
    mappings: Vec<(String, String, CompiledQuery)>,
    host_bindings: Vec<String>,
    uri: String,
    pub retained_bytes: usize,
    diagnostics: Vec<Diagnostic>,
}

fn diagnostic(uri: &str, message: impl Into<String>) -> Vec<Diagnostic> {
    vec![Diagnostic {
        uri: Some(uri.into()),
        code: "cem.xslt.scalar_mapping".into(),
        severity: Severity::Error,
        message: message.into(),
        ..Default::default()
    }]
}

impl XsltComponent {
    pub fn compile(
        source: &str,
        uri: &str,
        options: &XsltComponentOptions,
        host_bindings: &[String],
    ) -> std::result::Result<Self, Vec<Diagnostic>> {
        if options.parameters.len() > 250
            || options
                .parameters
                .iter()
                .any(|p| p.select.len() > 32 * 1024)
            || host_bindings.len() > 256
        {
            return Err(diagnostic(
                uri,
                "component parameter expressions exceed limits",
            ));
        }
        let mut names: Vec<&str> = options.parameters.iter().map(|p| p.name.as_str()).collect();
        if let Some(entry) = &options.entrypoint {
            names.push(entry);
        }
        let mut names = resolve_xslt_names(source, uri, &names)?;
        let entrypoint = options.entrypoint.as_ref().and_then(|_| names.pop());
        let mut parameters = BTreeMap::new();
        let mut mappings = Vec::new();
        let mut diagnostics = Vec::new();
        let mut context = StandaloneExpressionContext {
            source_uri: Some(uri.into()),
            ..Default::default()
        };
        context.scope_policy = context.scope_policy.with_queue_size(128);
        for name in host_bindings {
            context.bindings.insert(
                name.clone(),
                StandaloneExpressionBinding::any(ItemStream::empty()),
            );
        }
        for (index, (parameter, expanded)) in options.parameters.iter().zip(names).enumerate() {
            let binding = format!("component_param_{index}");
            if parameters.insert(expanded, binding.clone()).is_some() {
                return Err(diagnostic(
                    uri,
                    format!("duplicate XSLT parameter {}", parameter.name),
                ));
            }
            let compiled = compile_expression(&parameter.select, &context)
                .map_err(|error| error.diagnostics)?;
            diagnostics.extend(compiled.diagnostics);
            mappings.push((binding, parameter.name.clone(), compiled.query));
        }
        let modules = options
            .modules
            .iter()
            .map(|module| {
                Ok(XsltModuleSource {
                    parent_uri: module.parent_uri.clone(),
                    href: module.href.clone(),
                    uri: module.uri.clone(),
                    source: module.source.clone(),
                    content_hash: parse_hash(&module.content_hash)
                        .map_err(|error| diagnostic(uri, error.to_string()))?,
                })
            })
            .collect::<std::result::Result<Vec<_>, Vec<Diagnostic>>>()?;
        let compiled = compile_xslt_bundle_with_options(
            source,
            uri,
            &XsltCompileOptions {
                entrypoint,
                parameters,
                modules,
            },
        )?;
        let bundle = XsltBundle::from_bytes(
            &compiled.bytes,
            &compiled.content_hash,
            &compiled.source_hash,
        )
        .map_err(|error| diagnostic(uri, error.to_string()))?;
        let retained_bytes = compiled.bytes.len()
            + options
                .parameters
                .iter()
                .map(|p| p.select.len())
                .sum::<usize>();
        diagnostics.extend(bundle.template().diagnostics.clone());
        Ok(Self {
            bundle,
            mappings,
            host_bindings: host_bindings.to_vec(),
            uri: uri.into(),
            retained_bytes,
            diagnostics,
        })
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn render(&self, input: &TemplateData) -> RenderPlan {
        let mut context = EvaluationContext::default();
        context.scope_policy = context.scope_policy.with_queue_size(128);
        context.data_readers = input.data_readers.clone();
        context.policy_bindings = self
            .host_bindings
            .iter()
            .map(|name| {
                (
                    name.clone(),
                    input.bindings.get(name).cloned().unwrap_or_default(),
                )
            })
            .collect();
        let mut output = TemplateData::default();
        let mut diagnostics = self.diagnostics.clone();
        for binding in self.bundle.host_bindings() {
            output.bindings.insert(binding.clone(), ItemStream::empty());
        }
        // Initial focus is a native-only binding, never inferred from control data.
        if let Some(document) = input.bindings.get("document") {
            output.bindings.insert("document".into(), document.clone());
        }
        for (binding, name, query) in &self.mappings {
            let result = evaluate(query, &context);
            diagnostics.extend(result.diagnostics.iter().cloned());
            if result.error.is_some()
                || result
                    .diagnostics
                    .iter()
                    .any(|d| d.severity.is_hard_violation())
            {
                diagnostics.extend(diagnostic(
                    &self.uri,
                    format!(
                        "XSLT parameter {name} expression failed: {:?}",
                        result.error
                    ),
                ));
                return RenderPlan {
                    diagnostics,
                    nodes: vec![],
                    host_attribute_updates: vec![],
                };
            }
            let scalar = match result.items.as_slice() {
                [] => None,
                [Item::Atomic(value)] => Some(value.clone()),
                [Item::Native(value)]
                    if value.kind() == QueryItemViewKind::Atomic && value.atom().is_some() =>
                {
                    value.atom()
                }
                _ => {
                    diagnostics.extend(diagnostic(&self.uri,
                        format!("XSLT parameter {name} requires zero or one scalar; nodes, records, arrays and functions are not scalar mappings")));
                    return RenderPlan {
                        diagnostics,
                        nodes: vec![],
                        host_attribute_updates: vec![],
                    };
                }
            };
            if let Some(value) = scalar {
                output
                    .bindings
                    .insert(binding.clone(), ItemStream::once(Item::Atomic(value)));
            }
        }
        let mut plan = self.bundle.render(&output);
        for diagnostic in plan.diagnostics {
            if !diagnostics.contains(&diagnostic) {
                diagnostics.push(diagnostic);
            }
        }
        plan.diagnostics = diagnostics;
        plan
    }
}
