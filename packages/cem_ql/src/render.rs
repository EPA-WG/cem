//! Data-bound CEM-ML template rendering.
//!
//! This C2 slice gives the runtime a compile-once/render-many boundary:
//! canonical CEM-ML is tokenized by `cem_ml`, embedded CEM-QL expressions are
//! compiled by this crate, and render turns a host data snapshot into a
//! serializable-style render plan. A convenience HTML renderer remains for
//! Rust tests and CLI-style callers.

use std::collections::{BTreeMap, BTreeSet};

use cem_ml::diagnostics::{Diagnostic, Severity};
use cem_ml::interpreter::{OutputSpan, OutputTarget, TransformOutput};
use cem_ml::module_resolution::{
    CemModuleUrlMapping, CemModuleUrlScopedMap, CemModuleUrlSpecifierMap,
};
use cem_ml::operation_control::{ExecutionScopeId, OperationControl, SafePointPoller};
use cem_ml::scheduler::ScopePolicy;
use cem_ml::source::{ByteRange, BytesSource, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use cem_ml::tokenizer::cem::CemTokenizer;
use cem_ml::tokenizer::{SchemaToken, SchemaTokenKind, SchemaTokenizer};

use crate::api::{CompileContext, EvaluationContext, PreparedTypeChecking};
use crate::eval::{effective_boolean, AtomValue, EvalError, Item, ItemStream, QueryContextScope};
use crate::ir::CompiledQuery;

mod construction;
mod interpolation;
mod references;
mod hooks;
mod dispatch;
mod projection;
pub use projection::{project_attribute_value_with_control, project_render_plan_with_control};
pub use hooks::ExpressionScope;
mod attributes;
pub use attributes::project_attribute_value;
pub use references::expand_reference;
use construction::ResultBuffer;

#[cfg(test)]
mod prepared_tests;
#[cfg(test)]
mod copy_profile_tests;

/// Explicit result instructions survive portable compilation. Unknown instructions
/// are rejected by older artifact readers instead of becoming literal elements.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum ResultInstruction {
    Sequence,
    Element,
    Attribute,
    Document,
}

/// Binding name under which the `/datadom` data document is exposed to expressions.
const DATA_DOCUMENT_BINDING: &str = "datadom";
/// Stable transform primary artifact binding.
const PRIMARY_INPUT_BINDING: &str = "input";
/// Loop-position binding name. The legacy HTML+XSLT bridge rewrites XPath `position()` to
/// `position`; `cem:for-each` binds it to the 1-based iteration index.
const POSITION_BINDING: &str = "position";
const HTML_VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];
/// Browser/runtime-support bounded native template calls. The full transform-template
/// adapter has configurable limits; this render boundary keeps a conservative fixed
/// default until host options are threaded through the WASM API.
const MAX_TEMPLATE_CALL_DEPTH: usize = 32;

#[derive(Debug, Clone, Default)]
pub struct TemplateData {
    pub bindings: BTreeMap<String, ItemStream>,
    /// Host capabilities are not data bindings and never enter the data DOM.
    pub native_functions: crate::native::NativeFunctionRegistry,
    /// Reuse this runtime-only cache across renders to retain XPath XML owners.
    pub data_readers: crate::eval::DataReaderCache,
    /// Resolved schema types and destination attribute contracts supplied by the host.
    pub value_types: BTreeMap<String, cem_ml::schema::document_model::AttributeValueContract>,
    pub attribute_contracts: BTreeMap<String, cem_ml::schema::document_model::AttributeValueContract>,
    /// Runtime-only caller scope, carried by native template calls.
    pub expression_scope: ExpressionScope,
    /// Receiver input schema; independent of output attribute construction.
    pub input_attribute_contracts: BTreeMap<String, cem_ml::schema::document_model::AttributeValueContract>,
}

impl TemplateData {
    /// Lossless component input. The caller imports a native CEM artifact;
    /// neither its graph nor its values are reconstructed from DOM strings.
    pub fn bind_native_attribute(&mut self, attribute: Item) -> Result<(), String> {
        let view = attribute.view().ok_or("A native CEM attribute is required")?;
        let string_field = |name| view.field(name).and_then(|items| items.first().and_then(Item::atom))
            .and_then(|atom| if let AtomValue::String(value) = atom { Some(value) } else { None });
        if string_field("kind").as_deref() != Some("attribute") {
            return Err("A native CEM attribute is required".into());
        }
        let name = string_field("name").filter(|name| !name.is_empty()).ok_or("A native attribute needs a name")?;
        let values = view.field("values").unwrap_or_else(|| view.field("value").unwrap_or_default());

        bind_attribute_values(&mut self.bindings, &name, values);
        Ok(())
    }

    /// Native slice values bypass the JSON control-data channel entirely.
    pub fn bind_native_slice(&mut self, name: &str, values: ItemStream) -> Result<(), String> {
        if name.is_empty() { return Err("A native slice needs a name".into()); }
        let datadom = self.bindings.entry("datadom".into())
            .or_insert_with(|| ItemStream::once(Item::Record(BTreeMap::new())));
        let [Item::Record(fields)] = datadom.items.as_mut_slice() else {
            return Err("Native slices require a data-island control envelope".into());
        };
        let slices = fields.entry("slices".into()).or_insert_with(|| vec![Item::Record(BTreeMap::new())]);
        let [Item::Record(slices)] = slices.as_mut_slice() else {
            return Err("Native slices require a slice control envelope".into());
        };
        slices.insert(name.into(), values.items.clone());
        self.bindings.insert(name.into(), values);
        Ok(())
    }

    pub fn with_binding(mut self, name: impl Into<String>, value: ItemStream) -> Self {
        self.bindings.insert(name.into(), value);
        self
    }

    /// Explicit native binding channel for a loaded resource. JSON control data
    /// cannot manufacture a document by imitating its shape or handle.
    pub fn bind_cem_document(
        &mut self,
        slice: &str,
        tree: std::sync::Arc<cem_ml::parser::tree::RetainedCemTree>,
    ) -> Result<(), String> {
        fn record(items: &mut [Item]) -> Option<&mut BTreeMap<String, Vec<Item>>> {
            match items { [Item::Record(fields)] => Some(fields), _ => None }
        }
        let envelope = self.bindings.get_mut("datadom")
            .and_then(|stream| record(&mut stream.items))
            .and_then(|fields| fields.get_mut("slices"))
            .and_then(|items| record(items))
            .and_then(|fields| fields.get_mut(slice))
            .and_then(|items| record(items))
            .ok_or_else(|| format!("CEM document binding `{slice}` has no resource envelope."))?;
        envelope.insert("data".into(), vec![crate::eval::imported_cem_tree(tree)]);
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct CompileTemplateOptions {
    pub host_bindings: Vec<String>,
    pub skip_cemt_function_bodies: bool,
}

#[derive(Debug, Clone)]
pub struct TemplateArtifact {
    pub nodes: Vec<TemplateNode>,
    pub stylesheets: Vec<TemplateStylesheetArtifact>,
    pub module_map: Option<TemplateModuleMapArtifact>,
    pub diagnostics: Vec<Diagnostic>,
}

/// One resolver-preflighted CEMT dependency. Hosts resolve and load modules; the render engine
/// validates their hashes and compiles the immutable closure without performing I/O.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateModuleSource {
    pub alias: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_uri: Option<String>,
    pub uri: String,
    pub content_hash: String,
    pub source: String,
}

/// Portable identity and resolved dependency graph supplied by browser, CLI, or SSR hosts.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateModuleClosure {
    pub root_uri: String,
    pub root_content_hash: String,
    #[serde(default)]
    pub resolver_policy_stamp: String,
    #[serde(default)]
    pub entrypoint: String,
    #[serde(default)]
    pub parameter_contract: Vec<String>,
    #[serde(default)]
    pub cem_ml_version: String,
    #[serde(default)]
    pub cem_ql_version: String,
    #[serde(default)]
    pub modules: Vec<TemplateModuleSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TemplateStylesheetArtifact {
    pub css: String,
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateModuleMapArtifact {
    pub scopes: Vec<CemModuleUrlScopedMap>,
    pub specifiers: CemModuleUrlSpecifierMap,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TemplateNode {
    Result {
        instruction: ResultInstruction,
        attributes: Vec<TemplateAttribute>,
        children: Vec<TemplateNode>,
        source_map: SourceMapStack,
    },
    Element {
        tag: String,
        attributes: Vec<TemplateAttribute>,
        children: Vec<TemplateNode>,
        source_map: SourceMapStack,
    },
    Text {
        text: String,
        source_map: SourceMapStack,
    },
    Comment {
        text: String,
        source_map: SourceMapStack,
    },
    Expression(CompiledTemplateExpression),
    /// `cem:if` — emits its children only when `test` is truthy.
    If {
        test: Option<CompiledTemplateExpression>,
        children: Vec<TemplateNode>,
        source_map: SourceMapStack,
    },
    /// `cem:choose` — emits the children of the first branch whose `test` is truthy
    /// (a branch with `test: None` is `cem:otherwise`); at most one branch contributes.
    Choose {
        branches: Vec<ChooseBranch>,
        source_map: SourceMapStack,
    },
    /// `cem:for-each` — evaluates `select` to a sequence and renders `children` once per item,
    /// binding the current item to `as` (default `item`). Flattens like the conditionals (no
    /// wrapper element).
    ForEach {
        select: Option<CompiledTemplateExpression>,
        as_name: String,
        children: Vec<TemplateNode>,
        source_map: SourceMapStack,
    },
    /// `cem:project-payload` — materializes the runtime's serialized payload-node records
    /// as render-plan nodes. This is the authoritative bridge for rich declarative payload;
    /// it never carries live DOM identity, JavaScript properties, or event listeners.
    ProjectPayload {
        select: Option<CompiledTemplateExpression>,
        source_map: SourceMapStack,
    },
    /// `cem:variable` — evaluates `select`, binds it to `name`, and emits no output.
    /// The binding is visible to following nodes in the surrounding template scope.
    Variable {
        name: String,
        select: Option<CompiledTemplateExpression>,
        source_map: SourceMapStack,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChooseBranch {
    pub test: Option<CompiledTemplateExpression>,
    pub children: Vec<TemplateNode>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TemplateAttribute {
    pub name: String,
    pub value: Option<TemplateAttributeValue>,
    pub source_map: SourceMapStack,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TemplateAttributeValue {
    Literal(String),
    Template(Vec<TemplateAttributePart>),
    Expression(CompiledTemplateExpression),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TemplateAttributePart {
    Literal(String),
    Expression(CompiledTemplateExpression),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompiledTemplateExpression {
    pub source: String,
    pub query: Option<CompiledQuery>,
    pub source_map: SourceMapStack,
    pub byte_offset: u64,
}

#[derive(Debug, Clone)]
pub struct RenderPlan {
    pub nodes: Vec<RenderPlanNode>,
    pub host_attribute_updates: Vec<HostAttributeUpdate>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostAttributeUpdate {
    pub name: String,
    pub value: String,
}

impl HostAttributeUpdate {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }
}

pub type RenderPlanNode = cem_ml::value::CemValueNode<Item, ItemStream>;
pub type RenderPlanAttribute = cem_ml::value::CemValueAttribute<ItemStream>;

#[derive(Debug, Clone)]
pub struct RenderedTemplate {
    pub rendered: String,
    pub host_attribute_updates: Vec<HostAttributeUpdate>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn compile_template(source: &str, options: &CompileTemplateOptions) -> TemplateArtifact {
    #[cfg(test)]
    let mut profile = crate::compile_profile::Span::new("template/tokenize");
    let mut tokenizer =
        CemTokenizer::from_source(BytesSource::new(SourceId(1), source.as_bytes().to_vec()));
    let mut tokens = Vec::new();
    while let Some(token) = tokenizer.next_token() {
        tokens.push(token);
    }

    #[cfg(test)]
    profile.next("template/bindings");
    let mut declared_bindings: BTreeMap<String, ItemStream> = options
        .host_bindings
        .iter()
        .map(|name| (name.clone(), ItemStream::empty()))
        .collect();
    // The `/datadom` data document is always available to expressions for functional
    // selection (e.g. `datadom.attributes.label`), so declare it at compile time.
    declared_bindings.insert(DATA_DOCUMENT_BINDING.to_owned(), ItemStream::empty());
    declared_bindings.insert(PRIMARY_INPUT_BINDING.to_owned(), ItemStream::empty());
    // Implicit subject of generic match-template dispatch (saved/restored per call).
    declared_bindings.insert("node".to_owned(), ItemStream::empty());
    declared_bindings.insert("value".to_owned(), ItemStream::empty());
    declared_bindings.insert("context".to_owned(), ItemStream::empty());
    // `{attribute @name=X}` / `{slice @name=X}` declarations introduce named bindings, so
    // declare them too. The render engine owns declaration metadata, so the host runtime
    // no longer needs to scan the template to make `{$ X}` compile.
    for name in scan_declaration_names(&tokens) {
        declared_bindings
            .entry(name)
            .or_insert_with(ItemStream::empty);
    }
    let compile_context = CompileContext {
        policy_bindings: declared_bindings,
        ..CompileContext::default()
    };
    let mut compiler = TemplateCompiler {
        tokens: &tokens,
        source_identity: cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes()).header_value(),
        index: 0,
        compile_context,
        type_checking: PreparedTypeChecking::default(),
        diagnostics: tokenizer.take_diagnostics(),
        element_stack: Vec::new(),
        skip_cemt_function_bodies: options.skip_cemt_function_bodies,
    };
    #[cfg(test)]
    profile.next("template/compile-nodes");
    let mut nodes = compiler.compile_all();
    #[cfg(test)]
    profile.next("template/extract-validate");
    let module_map = extract_static_module_map(&mut nodes, &mut compiler.diagnostics);
    let mut stylesheets = Vec::new();
    extract_static_stylesheets(
        &mut nodes,
        false,
        &mut stylesheets,
        &mut compiler.diagnostics,
    );
    validate_module_bodies(&mut nodes, &mut compiler.diagnostics);
    TemplateArtifact {
        nodes,
        stylesheets,
        module_map,
        diagnostics: compiler.diagnostics,
    }
}

/// Compile a root template plus an already-resolved CEMT dependency closure into one renderable
/// artifact. Imported named templates are namespaced by resolved module URI, while their original
/// source maps remain attached to the compiled nodes. Resolution and loading deliberately stay in
/// the host so browser and SSR can apply the same scoped import-map policy.
pub fn compile_template_module_closure(
    source: &str,
    closure: &TemplateModuleClosure,
    options: &CompileTemplateOptions,
) -> TemplateArtifact {
    let expected_root_hash =
        cem_ml::content_cache::ContentHash::from_blake3(source.as_bytes()).header_value();
    let mut root = compile_template(source, options);
    if closure.root_content_hash != expected_root_hash {
        root.diagnostics.push(render_diagnostic(
            "cem.ql.template.module_hash_mismatch",
            format!(
                "root template `{}` content hash `{}` does not match `{expected_root_hash}`",
                closure.root_uri, closure.root_content_hash
            ),
            0,
            SourceMapStack::default(),
        ));
        return root;
    }
    if !closure.entrypoint.is_empty() && closure.entrypoint != "body" {
        root.diagnostics.push(render_diagnostic(
            "cem.ql.template.module_entrypoint_unsupported",
            format!(
                "module closure entrypoint `{}` is unsupported; use the root `body` entrypoint",
                closure.entrypoint
            ),
            0,
            SourceMapStack::default(),
        ));
        return root;
    }
    let mut expected_parameters = options.host_bindings.clone();
    expected_parameters.sort();
    expected_parameters.dedup();
    if !closure.parameter_contract.is_empty() && closure.parameter_contract != expected_parameters {
        root.diagnostics.push(render_diagnostic(
            "cem.ql.template.module_parameter_contract_mismatch",
            format!(
                "module closure parameter contract {:?} does not match host bindings {:?}",
                closure.parameter_contract, expected_parameters
            ),
            0,
            SourceMapStack::default(),
        ));
        return root;
    }
    if !closure.cem_ml_version.is_empty() && closure.cem_ml_version != cem_ml::VERSION {
        root.diagnostics.push(render_diagnostic(
            "cem.ql.template.module_version_mismatch",
            format!(
                "module closure CEM-ML version `{}` does not match runtime `{}`",
                closure.cem_ml_version,
                cem_ml::VERSION
            ),
            0,
            SourceMapStack::default(),
        ));
        return root;
    }
    if !closure.cem_ql_version.is_empty() && closure.cem_ql_version != crate::VERSION {
        root.diagnostics.push(render_diagnostic(
            "cem.ql.template.module_version_mismatch",
            format!(
                "module closure CEM-QL version `{}` does not match runtime `{}`",
                closure.cem_ql_version,
                crate::VERSION
            ),
            0,
            SourceMapStack::default(),
        ));
        return root;
    }

    let mut imports = BTreeMap::new();
    for module in &closure.modules {
        let key = (module.parent_uri.clone(), module.alias.clone());
        if imports.insert(key.clone(), module.uri.clone()).is_some() {
            root.diagnostics.push(render_diagnostic(
                "cem.ql.template.module_alias_duplicate",
                format!(
                    "template module alias `{}` is duplicated for `{}`",
                    module.alias,
                    module.parent_uri.as_deref().unwrap_or("<root>")
                ),
                0,
                SourceMapStack::default(),
            ));
        }
    }
    let mut imported_templates = Vec::new();
    let mut seen_uris = BTreeMap::<String, String>::new();
    let mut public_templates = BTreeMap::<String, BTreeSet<String>>::new();
    let mut compiled_modules = Vec::new();

    for module in &closure.modules {
        let actual_hash = cem_ml::content_cache::ContentHash::from_blake3(module.source.as_bytes())
            .header_value();
        if module.content_hash != actual_hash {
            root.diagnostics.push(render_diagnostic(
                "cem.ql.template.module_hash_mismatch",
                format!(
                    "template module `{}` content hash `{}` does not match `{actual_hash}`",
                    module.uri, module.content_hash
                ),
                0,
                SourceMapStack::default(),
            ));
            continue;
        }
        if let Some(existing_hash) = seen_uris.get(&module.uri) {
            if existing_hash != &actual_hash {
                root.diagnostics.push(render_diagnostic(
                    "cem.ql.template.module_identity_conflict",
                    format!(
                        "template module `{}` has conflicting content hashes in one closure",
                        module.uri
                    ),
                    0,
                    SourceMapStack::default(),
                ));
            }
            continue;
        }
        seen_uris.insert(module.uri.clone(), actual_hash);

        let parsed = cem_ml::transform_template::parse_cem_native_template_module_options(
            cem_ml::transform_template::TransformTemplateModuleParseRequest {
                template: cem_ml::engine::TemplateInput {
                    uri: module.uri.clone(),
                    bytes: module.source.as_bytes().to_vec(),
                    identity: None,
                    root_scope: cem_ml::run_config::ScopeConfig::default(),
                },
            },
        );
        root.diagnostics.extend(parsed.diagnostics);
        public_templates.insert(
            module.uri.clone(),
            parsed
                .module_options
                .entrypoints
                .into_iter()
                .filter(|entrypoint| {
                    entrypoint.visibility
                        == cem_ml::transform_template::TransformTemplateModuleVisibility::Public
                })
                .map(|entrypoint| entrypoint.name)
                .collect(),
        );

        let mut artifact = compile_template(&module.source, options);
        for diagnostic in &mut artifact.diagnostics {
            if diagnostic.uri.is_none() {
                diagnostic.uri = Some(module.uri.clone());
            }
        }
        compiled_modules.push((module.uri.as_str(), artifact));
    }

    rewrite_module_calls(
        &mut root.nodes,
        None,
        &imports,
        &public_templates,
        &mut root.diagnostics,
    );

    for (module_uri, mut artifact) in compiled_modules {
        rewrite_module_calls(
            &mut artifact.nodes,
            Some(module_uri),
            &imports,
            &public_templates,
            &mut artifact.diagnostics,
        );
        root.diagnostics.extend(artifact.diagnostics);
        root.stylesheets.extend(artifact.stylesheets);
        let mut declarations = hooks::module_hook_nodes(&artifact.nodes);
        for (index, mut declaration) in collect_template_declarations(&artifact.nodes)
            .into_iter()
            .enumerate()
        {
            if let TemplateNode::Element {
                attributes,
                source_map,
                ..
            } = &mut declaration
            {
                let name = declaration_name(attributes)
                    .unwrap_or_else(|| format!("anonymous-match-{index}"));
                let name = module_template_name(module_uri, &name);
                if let Some(attribute) = attributes.iter_mut().find(|a| a.name == "name") {
                    attribute.value = Some(TemplateAttributeValue::Literal(name));
                } else {
                    attributes.push(TemplateAttribute {
                        name: "name".into(),
                        value: Some(TemplateAttributeValue::Literal(name)),
                        source_map: source_map.clone(),
                    });
                }
            }
            declarations.push(declaration);
        }
        imported_templates.push(TemplateNode::Element { tag: "module".into(), attributes: vec![TemplateAttribute { name: "__cem-module-uri".into(), value: Some(TemplateAttributeValue::Literal(module_uri.into())), source_map: SourceMapStack::default() }], children: declarations, source_map: SourceMapStack::default() });
    }
    root.nodes.extend(imported_templates);
    root
}

fn extract_static_module_map(
    nodes: &mut Vec<TemplateNode>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TemplateModuleMapArtifact> {
    let mut retained = Vec::with_capacity(nodes.len());
    let mut module_map = None;
    for node in nodes.drain(..) {
        let TemplateNode::Element {
            tag,
            attributes,
            children,
            source_map,
        } = &node
        else {
            retained.push(node);
            continue;
        };
        if local_template_name(tag) != "module-map" {
            retained.push(node);
            continue;
        }
        if module_map.is_some() {
            diagnostics.push(render_diagnostic(
                "cem.ql.template.module_map_duplicate",
                "a template resolution context may declare at most one `module-map` prelude"
                    .to_owned(),
                source_map_start(source_map),
                source_map.clone(),
            ));
            continue;
        }
        if !attributes.is_empty() {
            diagnostics.push(render_diagnostic(
                "cem.ql.template.module_map_invalid",
                "the inline `module-map` prelude does not accept attributes".to_owned(),
                source_map_start(source_map),
                source_map.clone(),
            ));
        }
        module_map = parse_static_module_map(children, diagnostics, source_map);
    }
    *nodes = retained;
    module_map
}

fn parse_static_module_map(
    children: &[TemplateNode],
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMapStack,
) -> Option<TemplateModuleMapArtifact> {
    let mut map = TemplateModuleMapArtifact::default();
    let mut valid = true;
    for child in children {
        let TemplateNode::Element {
            tag,
            attributes,
            children,
            source_map: child_source_map,
        } = child
        else {
            if !matches!(child, TemplateNode::Text { text, .. } if text.trim().is_empty())
                && !matches!(child, TemplateNode::Comment { .. })
            {
                valid = false;
                diagnostics.push(render_diagnostic(
                    "cem.ql.template.module_map_dynamic_unsupported",
                    "`module-map` content must be static `import`, `resource`, or `scope` entries"
                        .to_owned(),
                    source_map_start(source_map),
                    source_map.clone(),
                ));
            }
            continue;
        };
        match local_template_name(tag) {
            "import" | "resource" => {
                valid &= insert_static_module_mapping(
                    &mut map.specifiers,
                    local_template_name(tag),
                    attributes,
                    children,
                    diagnostics,
                    child_source_map,
                );
            }
            "scope" => {
                let Some(prefix) = required_static_attribute(
                    attributes,
                    "prefix",
                    "scope",
                    diagnostics,
                    child_source_map,
                ) else {
                    valid = false;
                    continue;
                };
                let mut specifiers = CemModuleUrlSpecifierMap::default();
                for scoped_child in children {
                    let TemplateNode::Element {
                        tag,
                        attributes,
                        children,
                        source_map,
                    } = scoped_child
                    else {
                        if !matches!(scoped_child, TemplateNode::Text { text, .. } if text.trim().is_empty())
                            && !matches!(scoped_child, TemplateNode::Comment { .. })
                        {
                            valid = false;
                            diagnostics.push(render_diagnostic(
                                "cem.ql.template.module_map_dynamic_unsupported",
                                "a module-map `scope` may contain only static `import` or `resource` entries"
                                    .to_owned(),
                                source_map_start(child_source_map),
                                child_source_map.clone(),
                            ));
                        }
                        continue;
                    };
                    let kind = local_template_name(tag);
                    if kind != "import" && kind != "resource" {
                        valid = false;
                        diagnostics.push(render_diagnostic(
                            "cem.ql.template.module_map_invalid",
                            format!("unsupported module-map scope entry `{kind}`"),
                            source_map_start(source_map),
                            source_map.clone(),
                        ));
                        continue;
                    }
                    valid &= insert_static_module_mapping(
                        &mut specifiers,
                        kind,
                        attributes,
                        children,
                        diagnostics,
                        source_map,
                    );
                }
                map.scopes
                    .push(CemModuleUrlScopedMap { prefix, specifiers });
            }
            kind => {
                valid = false;
                diagnostics.push(render_diagnostic(
                    "cem.ql.template.module_map_invalid",
                    format!("unsupported module-map entry `{kind}`"),
                    source_map_start(child_source_map),
                    child_source_map.clone(),
                ));
            }
        }
    }
    valid.then_some(map)
}

fn insert_static_module_mapping(
    specifiers: &mut CemModuleUrlSpecifierMap,
    kind: &str,
    attributes: &[TemplateAttribute],
    children: &[TemplateNode],
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMapStack,
) -> bool {
    if !children.iter().all(|child| {
        matches!(child, TemplateNode::Text { text, .. } if text.trim().is_empty())
            || matches!(child, TemplateNode::Comment { .. })
    }) {
        diagnostics.push(render_diagnostic(
            "cem.ql.template.module_map_invalid",
            format!("module-map `{kind}` entries cannot have content"),
            source_map_start(source_map),
            source_map.clone(),
        ));
        return false;
    }
    let Some(specifier) =
        required_static_attribute(attributes, "specifier", kind, diagnostics, source_map)
    else {
        return false;
    };
    let Some(target) =
        required_static_attribute(attributes, "target", kind, diagnostics, source_map)
    else {
        return false;
    };
    if specifiers.imports.contains_key(&specifier) || specifiers.resources.contains_key(&specifier)
    {
        diagnostics.push(render_diagnostic(
            "cem.ql.template.module_map_duplicate",
            format!("duplicate module-map specifier `{specifier}`"),
            source_map_start(source_map),
            source_map.clone(),
        ));
        return false;
    }
    let mut mapping = CemModuleUrlMapping::target(target);
    if kind == "resource" {
        mapping.content_type_hint = optional_static_attribute(attributes, "content-type");
        mapping.integrity = optional_static_attribute(attributes, "integrity");
        specifiers.resources.insert(specifier, mapping);
    } else {
        specifiers.imports.insert(specifier, mapping);
    }
    true
}

fn required_static_attribute(
    attributes: &[TemplateAttribute],
    name: &str,
    entry: &str,
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMapStack,
) -> Option<String> {
    let value = optional_static_attribute(attributes, name);
    if value.as_ref().is_some_and(|value| !value.trim().is_empty()) {
        return value;
    }
    diagnostics.push(render_diagnostic(
        "cem.ql.template.module_map_invalid",
        format!("module-map `{entry}` requires a static non-empty `{name}` attribute"),
        source_map_start(source_map),
        source_map.clone(),
    ));
    None
}

fn optional_static_attribute(attributes: &[TemplateAttribute], name: &str) -> Option<String> {
    let attribute = attributes.iter().find(|attribute| attribute.name == name)?;
    match &attribute.value {
        Some(TemplateAttributeValue::Literal(value)) => Some(value.trim().to_owned()),
        _ => None,
    }
}

fn extract_static_stylesheets(
    nodes: &mut Vec<TemplateNode>,
    dynamic_ancestor: bool,
    stylesheets: &mut Vec<TemplateStylesheetArtifact>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut retained = Vec::with_capacity(nodes.len());
    for mut node in nodes.drain(..) {
        match &mut node {
            TemplateNode::Element {
                tag,
                attributes,
                children,
                source_map,
            } if local_template_name(tag) == "style" => {
                let scope = static_stylesheet_scope(attributes);
                let css = static_stylesheet_text(children);
                if dynamic_ancestor || scope.is_err() || css.is_none() {
                    diagnostics.push(render_diagnostic(
                        "cem.ql.template.stylesheet_dynamic_unsupported",
                        "declaration stylesheet content and its `scope` attribute must be static"
                            .to_owned(),
                        source_map_start(source_map),
                        source_map.clone(),
                    ));
                } else if let (Ok(scope), Some(css)) = (scope, css) {
                    stylesheets.push(TemplateStylesheetArtifact { css, scope });
                }
            }
            TemplateNode::Element { children, .. } => {
                extract_static_stylesheets(children, dynamic_ancestor, stylesheets, diagnostics);
                retained.push(node);
            }
            TemplateNode::If { children, .. } | TemplateNode::ForEach { children, .. } => {
                extract_static_stylesheets(children, true, stylesheets, diagnostics);
                retained.push(node);
            }
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    extract_static_stylesheets(
                        &mut branch.children,
                        true,
                        stylesheets,
                        diagnostics,
                    );
                }
                retained.push(node);
            }
            _ => retained.push(node),
        }
    }
    *nodes = retained;
}

fn static_stylesheet_scope(attributes: &[TemplateAttribute]) -> Result<Option<String>, ()> {
    let Some(attribute) = attributes
        .iter()
        .find(|attribute| attribute.name == "scope")
    else {
        return Ok(None);
    };
    match &attribute.value {
        Some(TemplateAttributeValue::Literal(value)) => Ok(Some(value.trim().to_owned())),
        None => Ok(Some(String::new())),
        _ => Err(()),
    }
}

fn static_stylesheet_text(children: &[TemplateNode]) -> Option<String> {
    let mut css = String::new();
    for child in children {
        match child {
            TemplateNode::Text { text, .. } => css.push_str(text),
            TemplateNode::Comment { text, .. } => {
                css.push_str("/*");
                css.push_str(text);
                css.push_str("*/");
            }
            _ => return None,
        }
    }
    Some(css)
}

pub fn render_compiled_template(artifact: &TemplateArtifact, data: &TemplateData) -> RenderPlan {
    render_compiled_template_internal(artifact, data, None, None, false).plan
}

pub fn render_compiled_template_with_control(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> RenderPlan {
    render_compiled_template_internal(artifact, data, Some((control, scope)), None, false).plan
}

/// A native adapter can resolve module calls inside the active rendering scope,
/// so a caller's recovery boundary also protects failures in the callee.
pub trait TemplateCallHandler {
    fn handles(&self, tag: &str) -> bool;
    fn call(
        &self,
        attributes: &[RenderPlanAttribute],
        source_map: &SourceMapStack,
        data: &TemplateData,
        protected: bool,
    ) -> TemplateCallResult;
}

#[derive(Debug, Clone)]
pub struct TemplateFailure {
    pub error: EvalError,
    pub diagnostic: Diagnostic,
}

#[derive(Debug, Clone)]
pub struct TemplateCallResult {
    pub plan: RenderPlan,
    pub failure: Option<TemplateFailure>,
}

pub fn render_compiled_template_with_calls(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    control: Option<(&OperationControl, ExecutionScopeId)>,
    calls: &dyn TemplateCallHandler,
    protected: bool,
) -> TemplateCallResult {
    render_compiled_template_internal(artifact, data, control, Some(calls), protected)
}

fn render_compiled_template_internal(
    artifact: &TemplateArtifact,
    data: &TemplateData,
    control: Option<(&OperationControl, ExecutionScopeId)>,
    calls: Option<&dyn TemplateCallHandler>,
    protected: bool,
) -> TemplateCallResult {
    let owned_control = OperationControl::with_root_policy(Default::default(), ScopePolicy::host_root().with_queue_size(128)).expect("valid renderer default policy");
    let control = Some(control.unwrap_or((&owned_control, cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)));
    let policy = control.and_then(|(c, s)| c.scope_tree().scope(s).map(|s| s.effective_policy))
        .unwrap_or_else(ScopePolicy::host_root);
    #[cfg(test)]
    let mut profile = crate::compile_profile::Span::new("render/input-context");
    let mut policy_bindings = data.bindings.clone();
    let datadom = data_document_with_host_bindings(&data.bindings);
    policy_bindings.insert(DATA_DOCUMENT_BINDING.to_owned(), datadom);
    let mut host_attribute_updates =
        seed_declaration_defaults(&artifact.nodes, &mut policy_bindings);
    #[cfg(test)]
    profile.next("render/index-templates");
    let templates = collect_named_templates(&artifact.nodes);
    let match_rules = collect_match_rules(&artifact.nodes);
    #[cfg(test)]
    profile.next("render/renderer-metadata");
    let mut renderer = PlanRenderer {
        evaluation_context: EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: policy,
            diagnostics: Vec::new(),
            policy_bindings,
            current_item: data.expression_scope.focus.clone(),
            module_resolution: None,
            native_functions: data.native_functions.clone(),
            data_readers: data.data_readers.clone(),
        },
        diagnostics: artifact.diagnostics.clone(),
        templates,
        match_rules,
        call_depth: data.expression_scope.call_depth,
        max_call_depth: MAX_TEMPLATE_CALL_DEPTH.min(policy.stack_depth as usize),
        safe_points: control.map(|(control, scope)| SafePointPoller::new(control.clone(), scope)),
        control: control.map(|(control, scope)| (control.clone(), scope)),
        control_failed: false,
        recovery_depth: usize::from(protected),
        failure: None,
        result_work: 0,
        result_bytes: 0,
        result_depth: 0,
        text_memory: Vec::new(),
        hook_scopes: vec![Vec::new()],
        active_hooks: data.expression_scope.active.clone(),
        capture_depth: None,
        render_scope_depth: 0,
        value_types: data.value_types.clone(),
        attribute_contracts: data.attribute_contracts.clone(),
        expression_target: data.expression_scope.target.clone(),
        calls,
    };
    #[cfg(test)]
    profile.next("render/body");
    let mut nodes = ResultBuffer::default();
    renderer.apply_attribute_declaration_selects(&artifact.nodes, &mut host_attribute_updates);
    renderer.validate_receiver_inputs(&artifact.nodes, &data.input_attribute_contracts);
    let boundary_source = artifact
        .nodes
        .first()
        .map(template_node_source_map)
        .cloned()
        .unwrap_or_default();
    if renderer.force_render(&boundary_source) {
        renderer.register_module_hooks(&artifact.nodes);
        renderer.hook_scopes.extend(data.expression_scope.scopes.clone());
        for node in root_render_nodes(&artifact.nodes) {
            let mut ignored_attributes = Vec::new();
            renderer.render_into(node, &mut nodes, &mut ignored_attributes);
            if renderer.control_failed {
                break;
            }
        }
        renderer.force_render(&boundary_source);
    }
    if renderer.control_failed || renderer.failure.is_some() {
        nodes.clear();
        host_attribute_updates.clear();
    }
    if renderer.control_failed && renderer.failure.is_none() {
        if let Some(diagnostic) = renderer.diagnostics.last().cloned() {
            renderer.failure = Some(TemplateFailure {
                error: EvalError::Unsupported("template execution control failed"),
                diagnostic,
            });
        }
    }
    let nodes = renderer.finish_result_buffer(nodes, &boundary_source);
    TemplateCallResult {
        failure: renderer.failure,
        plan: RenderPlan {
            nodes,
            host_attribute_updates,
            diagnostics: renderer.diagnostics,
        },
    }
}

pub fn render_template(source: &str, data: &TemplateData) -> RenderedTemplate {
    let options = CompileTemplateOptions {
        host_bindings: data.bindings.keys().cloned().collect(),
        ..CompileTemplateOptions::default()
    };
    let artifact = compile_template(source, &options);
    let plan = render_compiled_template(&artifact, data);
    RenderedTemplate {
        rendered: render_plan_to_html(&plan),
        host_attribute_updates: plan.host_attribute_updates,
        diagnostics: plan.diagnostics,
    }
}

/// Build the `/datadom` data document exposed to cem-ql expressions for functional
/// data selection. Host bindings (the attributes/slices the runtime supplies) become
/// `datadom.attributes.<name>`, the functional-parity equivalent of the legacy
/// `/datadom/attributes` XPath model — navigated with cem-ql record/pipeline access
/// (`record_field`) rather than an XPath engine.
fn data_document_with_host_bindings(bindings: &BTreeMap<String, ItemStream>) -> ItemStream {
    let synthesized = build_data_document(bindings);
    let Some(explicit) = bindings.get(DATA_DOCUMENT_BINDING) else {
        return synthesized;
    };
    merge_data_documents(explicit.clone(), synthesized)
}

fn build_data_document(bindings: &BTreeMap<String, ItemStream>) -> ItemStream {
    let attributes: BTreeMap<String, Vec<Item>> = bindings
        .iter()
        .filter(|(name, _)| name.as_str() != DATA_DOCUMENT_BINDING)
        .map(|(name, stream)| (name.clone(), stream.items.clone()))
        .collect();
    let mut datadom = BTreeMap::new();
    for (name, stream) in bindings
        .iter()
        .filter(|(name, _)| name.as_str() != DATA_DOCUMENT_BINDING)
    {
        datadom.insert(name.clone(), stream.items.clone());
    }
    datadom.insert("attributes".to_owned(), vec![Item::Record(attributes)]);
    ItemStream::once(Item::Record(datadom))
}

fn merge_data_documents(mut explicit: ItemStream, synthesized: ItemStream) -> ItemStream {
    let Some(Item::Record(synthesized_fields)) = synthesized.items.first() else {
        return explicit;
    };
    for item in &mut explicit.items {
        let Item::Record(explicit_fields) = item else {
            continue;
        };
        for (name, values) in synthesized_fields {
            explicit_fields
                .entry(name.clone())
                .or_insert_with(|| values.clone());
        }
    }
    explicit
}

fn bind_attribute_values(bindings: &mut BTreeMap<String, ItemStream>, name: &str, values: Vec<Item>) {
    bindings.insert(name.into(), ItemStream::from_items(values.clone()));
    fn fields(stream: &mut ItemStream) -> Option<&mut BTreeMap<String, Vec<Item>>> {
        match stream.items.as_mut_slice() { [Item::Record(fields)] => Some(fields), _ => None }
    }
    if let Some(attributes) = bindings.get_mut("attributes").and_then(fields) {
        attributes.insert(name.into(), values.clone());
    }
    if let Some(datadom) = bindings.get_mut("datadom").and_then(fields) {
        let attributes = datadom.entry("attributes".into()).or_insert_with(|| vec![Item::Record(BTreeMap::new())]);
        if let [Item::Record(attributes)] = attributes.as_mut_slice() { attributes.insert(name.into(), values); }
    }
}

fn set_current_data_document_attribute(
    bindings: &mut BTreeMap<String, ItemStream>,
    name: &str,
    value: &str,
) {
    let Some(datadom) = bindings.get_mut(DATA_DOCUMENT_BINDING) else {
        return;
    };
    for item in &mut datadom.items {
        let Item::Record(fields) = item else {
            continue;
        };
        let Some(attributes) = fields.get_mut("attributes") else {
            continue;
        };
        for attribute_item in attributes {
            let Item::Record(attribute_fields) = attribute_item else {
                continue;
            };
            attribute_fields.insert(
                name.to_owned(),
                vec![Item::Atomic(AtomValue::String(value.to_owned()))],
            );
        }
    }
}

fn set_current_data_document_slice(
    bindings: &mut BTreeMap<String, ItemStream>,
    name: &str,
    value: &Item,
) {
    let Some(datadom) = bindings.get_mut(DATA_DOCUMENT_BINDING) else {
        return;
    };
    for item in &mut datadom.items {
        let Item::Record(fields) = item else {
            continue;
        };
        let slices = fields
            .entry("slices".to_owned())
            .or_insert_with(|| vec![Item::Record(BTreeMap::new())]);
        if !slices.iter().any(|item| matches!(item, Item::Record(_))) {
            *slices = vec![Item::Record(BTreeMap::new())];
        }
        for slice_item in slices {
            let Item::Record(slice_fields) = slice_item else {
                continue;
            };
            slice_fields.insert(name.to_owned(), vec![value.clone()]);
        }
    }
}

pub fn render_plan_to_html(plan: &RenderPlan) -> String {
    render_plan_to_html_with_source_map(plan).rendered
}

pub fn render_plan_to_html_with_source_map(plan: &RenderPlan) -> TransformOutput {
    render_plan_to_html_with_control(plan, &OperationControl::default(), cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
        .unwrap_or_else(|error| projection_failure(plan, OutputTarget::LightDomCustomElements, error))
}

fn projection_failure(plan: &RenderPlan, target: OutputTarget, error: cem_ml::operation_control::ControlError) -> TransformOutput {
    let mut diagnostics = plan.diagnostics.clone();
    diagnostics.push(render_diagnostic(error.code(), error.to_string(), 0, Default::default()));
    TransformOutput { target, rendered: String::new(), diagnostics, source_map: Default::default(), output_spans: vec![] }
}

pub fn render_plan_to_html_with_control(
    plan: &RenderPlan,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<TransformOutput, cem_ml::operation_control::ControlError> {
    let projected = project_render_plan_with_control(plan, QueryContextScope(0), &cem_ml::value::artifact::CemValueArtifactLimits::default(), control, scope)?;
    let plan = &projected;
    let mut renderer = RenderPlanHtmlRenderer::controlled(control, scope);
    renderer.force()?;
    renderer.render_plan(plan);
    renderer.force()?;
    if let Some(error) = renderer.control_error {
        return Err(error);
    }
    let rendered_len = renderer.out.len() as u32;
    Ok(TransformOutput {
        target: OutputTarget::LightDomCustomElements,
        rendered: renderer.out,
        diagnostics: plan.diagnostics.clone(),
        source_map: SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Single(ByteRange::new(0, rendered_len)),
                transform: TransformKind::InterpreterRender,
            }],
        },
        output_spans: renderer.spans,
    })
}

pub fn render_plan_to_xml_with_source_map(plan: &RenderPlan) -> TransformOutput {
    render_plan_to_xml_with_control(plan, &OperationControl::default(), cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
        .unwrap_or_else(|error| projection_failure(plan, OutputTarget::Xml, error))
}

pub fn render_plan_to_xml_with_control(
    plan: &RenderPlan,
    control: &OperationControl,
    scope: ExecutionScopeId,
) -> Result<TransformOutput, cem_ml::operation_control::ControlError> {
    let projected = project_render_plan_with_control(plan, QueryContextScope(0), &cem_ml::value::artifact::CemValueArtifactLimits::default(), control, scope)?;
    let plan = &projected;
    let mut renderer = RenderPlanXmlRenderer::controlled(control, scope);
    renderer.force()?;
    renderer.render_plan(plan);
    renderer.force()?;
    if let Some(error) = renderer.control_error {
        return Err(error);
    }
    let rendered_len = renderer.out.len() as u32;
    Ok(TransformOutput {
        target: OutputTarget::Xml,
        rendered: renderer.out,
        diagnostics: plan.diagnostics.clone(),
        source_map: SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(0),
                span: FrameSpan::Single(ByteRange::new(0, rendered_len)),
                transform: TransformKind::InterpreterRender,
            }],
        },
        output_spans: renderer.spans,
    })
}

#[derive(Default)]
struct RenderPlanHtmlRenderer {
    out: String,
    spans: Vec<OutputSpan>,
    safe_points: Option<SafePointPoller>,
    control_error: Option<cem_ml::operation_control::ControlError>,
    control: Option<(OperationControl, ExecutionScopeId)>,
    output_memory: Vec<cem_ml::operation_control::MemoryPermit>,
    accounted_bytes: usize,
}

impl RenderPlanHtmlRenderer {
    fn controlled(control: &OperationControl, scope: ExecutionScopeId) -> Self {
        Self {
            safe_points: Some(SafePointPoller::new(control.clone(), scope)),
            control: Some((control.clone(), scope)),
            ..Self::default()
        }
    }

    fn account_output(&mut self) -> Result<(), cem_ml::operation_control::ControlError> {
        if let Some((control, scope)) = &self.control {
            let bytes = self.out.len().saturating_sub(self.accounted_bytes);
            if bytes > 0 {
                self.output_memory.push(control.charge_memory(*scope, bytes as u64, None)?);
                self.accounted_bytes = self.out.len();
            }
        }
        Ok(())
    }
    fn poll(&mut self) -> bool {
        if self.out.len().saturating_sub(self.accounted_bytes) >= 1024 {
            if let Err(error) = self.account_output() { self.control_error = Some(error); }
        }
        if self.control_error.is_some() {
            return false;
        }
        let Some(safe_points) = self.safe_points.as_mut() else {
            return true;
        };
        match safe_points.poll_one() {
            Ok(()) => true,
            Err(error) => {
                self.control_error = Some(error);
                false
            }
        }
    }

    fn force(&mut self) -> Result<(), cem_ml::operation_control::ControlError> {
        self.account_output()?;
        if let Some(error) = self.control_error.clone() {
            return Err(error);
        }
        match self
            .safe_points
            .as_mut()
            .map(SafePointPoller::force)
            .transpose()
        {
            Ok(_) => Ok(()),
            Err(error) => {
                self.control_error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn render_plan(&mut self, plan: &RenderPlan) {
        for node in &plan.nodes {
            if !self.poll() {
                break;
            }
            self.render_node(node);
        }
    }

    fn render_node(&mut self, node: &RenderPlanNode) {
        if !self.poll() {
            return;
        }
        match node {
            RenderPlanNode::Reference { reference, .. } => {
                for node in expand_reference(reference) { self.render_node(&node); }
            }
            RenderPlanNode::Element {
                tag,
                namespace,
                qualified_name,
                attributes,
                children,
                source_map,
            } => {
                let tag = qualified_name.as_deref().unwrap_or(tag);
                let open_start = self.out.len() as u64;
                self.out.push('<');
                self.out.push_str(tag);
                for attribute in attributes {
                    if !self.poll() {
                        return;
                    }
                    self.render_attribute(attribute);
                }
                if HTML_VOID_ELEMENTS.contains(&tag) && children.is_empty() {
                    self.out.push('>');
                    self.record_span(open_start, source_map);
                    return;
                }
                self.out.push('>');
                self.record_span(open_start, source_map);
                for child in children {
                    if !self.poll() {
                        return;
                    }
                    if qualified_name.is_some()
                        && matches!(namespace.as_deref(), None | Some("http://www.w3.org/1999/xhtml"))
                        && matches!(tag, "style" | "script")
                    {
                        if let RenderPlanNode::Text { text, source_map } = child {
                            let start = self.out.len() as u64;
                            self.push_raw(text);
                            self.record_span(start, source_map);
                            continue;
                        }
                    }
                    self.render_node(child);
                }
                let close_start = self.out.len() as u64;
                self.out.push_str("</");
                self.out.push_str(tag);
                self.out.push('>');
                self.record_span(close_start, source_map);
            }
            RenderPlanNode::Text { text, source_map } => {
                let start = self.out.len() as u64;
                self.escape_text(text);
                self.record_span(start, source_map);
            }
            RenderPlanNode::Comment { text, source_map } => {
                let start = self.out.len() as u64;
                self.out.push_str("<!--");
                self.push_raw(text);
                self.out.push_str("-->");
                self.record_span(start, source_map);
            }
            RenderPlanNode::Cdata { text, source_map } => {
                let start = self.out.len() as u64;
                self.escape_text(text);
                self.record_span(start, source_map);
            }
            RenderPlanNode::ProcessingInstruction {
                target,
                data,
                source_map,
            } => {
                let start = self.out.len() as u64;
                self.out.push_str("<?");
                self.out.push_str(target);
                if !data.is_empty() {
                    self.out.push(' ');
                    self.push_raw(data);
                }
                self.out.push_str("?>");
                self.record_span(start, source_map);
            }
        }
    }

    fn render_attribute(&mut self, attribute: &RenderPlanAttribute) {
        let value = &attribute.value;
        let start = self.out.len() as u64;
        self.out.push(' ');
        if let Some(namespace) = attribute
            .namespace
            .as_deref()
            .filter(|value| !value.is_empty() && attribute.qualified_name.is_none())
        {
            self.out.push_str(namespace);
            self.out.push(':');
        }
        self.out.push_str(attribute.qualified_name.as_deref().unwrap_or(&attribute.name));
        if !value.is_empty() || attribute.qualified_name.is_some() {
            self.out.push_str("=\"");
            self.escape_attr(&value);
            self.out.push('"');
        }
        self.record_span(start, &attribute.source_map);
    }

    fn push_raw(&mut self, value: &str) {
        for character in value.chars() {
            if !self.poll() {
                break;
            }
            self.out.push(character);
        }
    }

    fn escape_text(&mut self, value: &str) {
        escape_controlled(
            &mut self.out,
            value,
            false,
            &mut self.safe_points,
            &mut self.control_error,
        );
    }

    fn escape_attr(&mut self, value: &str) {
        escape_controlled(
            &mut self.out,
            value,
            true,
            &mut self.safe_points,
            &mut self.control_error,
        );
    }

    fn record_span(&mut self, start: u64, origin: &SourceMapStack) {
        let end = self.out.len() as u64;
        if end <= start {
            return;
        }
        let mut origin = origin.clone();
        origin.push(SourceMapFrame {
            source_id: origin
                .frames
                .last()
                .map(|frame| frame.source_id)
                .unwrap_or(SourceId(0)),
            span: FrameSpan::Single(ByteRange::new(start, (end - start) as u32)),
            transform: TransformKind::InterpreterRender,
        });
        self.spans.push(OutputSpan {
            output_range: ByteRange::new(start, (end - start) as u32),
            origin,
        });
    }
}

#[derive(Default)]
struct RenderPlanXmlRenderer {
    out: String,
    spans: Vec<OutputSpan>,
    safe_points: Option<SafePointPoller>,
    control_error: Option<cem_ml::operation_control::ControlError>,
    control: Option<(OperationControl, ExecutionScopeId)>,
    output_memory: Vec<cem_ml::operation_control::MemoryPermit>,
    accounted_bytes: usize,
}

impl RenderPlanXmlRenderer {
    fn controlled(control: &OperationControl, scope: ExecutionScopeId) -> Self {
        Self {
            safe_points: Some(SafePointPoller::new(control.clone(), scope)),
            control: Some((control.clone(), scope)),
            ..Self::default()
        }
    }

    fn account_output(&mut self) -> Result<(), cem_ml::operation_control::ControlError> {
        if let Some((control, scope)) = &self.control {
            let bytes = self.out.len().saturating_sub(self.accounted_bytes);
            if bytes > 0 {
                self.output_memory.push(control.charge_memory(*scope, bytes as u64, None)?);
                self.accounted_bytes = self.out.len();
            }
        }
        Ok(())
    }
    fn poll(&mut self) -> bool {
        if self.out.len().saturating_sub(self.accounted_bytes) >= 1024 {
            if let Err(error) = self.account_output() { self.control_error = Some(error); }
        }
        if self.control_error.is_some() {
            return false;
        }
        let Some(safe_points) = self.safe_points.as_mut() else {
            return true;
        };
        match safe_points.poll_one() {
            Ok(()) => true,
            Err(error) => {
                self.control_error = Some(error);
                false
            }
        }
    }

    fn force(&mut self) -> Result<(), cem_ml::operation_control::ControlError> {
        self.account_output()?;
        if let Some(error) = self.control_error.clone() {
            return Err(error);
        }
        match self
            .safe_points
            .as_mut()
            .map(SafePointPoller::force)
            .transpose()
        {
            Ok(_) => Ok(()),
            Err(error) => {
                self.control_error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn render_plan(&mut self, plan: &RenderPlan) {
        for node in &plan.nodes {
            if !self.poll() {
                break;
            }
            self.render_node(node);
        }
    }

    fn render_node(&mut self, node: &RenderPlanNode) {
        if !self.poll() {
            return;
        }
        match node {
            RenderPlanNode::Reference { reference, .. } => {
                for node in expand_reference(reference) { self.render_node(&node); }
            }
            RenderPlanNode::Element {
                tag,
                namespace: _,
                qualified_name,
                attributes,
                children,
                source_map,
            } => {
                let tag = qualified_name.as_deref().unwrap_or(tag);
                let open_start = self.out.len() as u64;
                self.out.push('<');
                self.out.push_str(tag);
                for attribute in attributes {
                    if !self.poll() {
                        return;
                    }
                    self.render_attribute(attribute);
                }
                if children.is_empty() {
                    self.out.push_str("/>");
                    self.record_span(open_start, source_map);
                    return;
                }

                self.out.push('>');
                self.record_span(open_start, source_map);
                for child in children {
                    if !self.poll() {
                        return;
                    }
                    self.render_node(child);
                }
                let close_start = self.out.len() as u64;
                self.out.push_str("</");
                self.out.push_str(tag);
                self.out.push('>');
                self.record_span(close_start, source_map);
            }
            RenderPlanNode::Text { text, source_map } => {
                let start = self.out.len() as u64;
                self.escape_text(text);
                self.record_span(start, source_map);
            }
            RenderPlanNode::Comment { text, source_map } => {
                let start = self.out.len() as u64;
                self.out.push_str("<!--");
                self.push_raw(text);
                self.out.push_str("-->");
                self.record_span(start, source_map);
            }
            RenderPlanNode::Cdata { text, source_map } => {
                let start = self.out.len() as u64;
                self.out.push_str("<![CDATA[");
                self.push_raw(text);
                self.out.push_str("]]>");
                self.record_span(start, source_map);
            }
            RenderPlanNode::ProcessingInstruction {
                target,
                data,
                source_map,
            } => {
                let start = self.out.len() as u64;
                self.out.push_str("<?");
                self.out.push_str(target);
                if !data.is_empty() {
                    self.out.push(' ');
                    self.push_raw(data);
                }
                self.out.push_str("?>");
                self.record_span(start, source_map);
            }
        }
    }

    fn render_attribute(&mut self, attribute: &RenderPlanAttribute) {
        let start = self.out.len() as u64;
        self.out.push(' ');
        if let Some(namespace) = attribute
            .namespace
            .as_deref()
            .filter(|value| !value.is_empty() && attribute.qualified_name.is_none())
        {
            self.out.push_str(namespace);
            self.out.push(':');
        }
        self.out.push_str(attribute.qualified_name.as_deref().unwrap_or(&attribute.name));
        self.out.push_str("=\"");
        self.escape_attr(&project_attribute_value(attribute));
        self.out.push('"');
        self.record_span(start, &attribute.source_map);
    }

    fn push_raw(&mut self, value: &str) {
        for character in value.chars() {
            if !self.poll() {
                break;
            }
            self.out.push(character);
        }
    }

    fn escape_text(&mut self, value: &str) {
        escape_controlled(
            &mut self.out,
            value,
            false,
            &mut self.safe_points,
            &mut self.control_error,
        );
    }

    fn escape_attr(&mut self, value: &str) {
        escape_controlled(
            &mut self.out,
            value,
            true,
            &mut self.safe_points,
            &mut self.control_error,
        );
    }

    fn record_span(&mut self, start: u64, origin: &SourceMapStack) {
        let end = self.out.len() as u64;
        if end <= start {
            return;
        }
        let mut origin = origin.clone();
        origin.push(SourceMapFrame {
            source_id: origin
                .frames
                .last()
                .map(|frame| frame.source_id)
                .unwrap_or(SourceId(0)),
            span: FrameSpan::Single(ByteRange::new(start, (end - start) as u32)),
            transform: TransformKind::InterpreterRender,
        });
        self.spans.push(OutputSpan {
            output_range: ByteRange::new(start, (end - start) as u32),
            origin,
        });
    }
}

struct TemplateCompiler<'a> {
    source_identity: String,
    tokens: &'a [SchemaToken],
    index: usize,
    compile_context: CompileContext,
    type_checking: PreparedTypeChecking,
    diagnostics: Vec<Diagnostic>,
    element_stack: Vec<String>,
    skip_cemt_function_bodies: bool,
}

impl TemplateCompiler<'_> {
    fn compile_all(&mut self) -> Vec<TemplateNode> {
        let mut nodes = Vec::new();
        while self.index < self.tokens.len() {
            if matches!(
                self.tokens[self.index].kind,
                SchemaTokenKind::NodeEnd { .. }
            ) {
                self.index += 1;
                continue;
            }
            if let Some(node) = self.compile_node() {
                nodes.push(node);
            }
        }
        nodes
    }

    /// Parse the node at the cursor (advancing it), or skip a stray token (returns `None`).
    fn compile_node(&mut self) -> Option<TemplateNode> {
        match &self.tokens[self.index].kind {
            SchemaTokenKind::NodeStart { name } if name == "$" => {
                Some(TemplateNode::Expression(self.compile_expression_node()))
            }
            SchemaTokenKind::NodeStart { name } if is_if_name(name) => Some(self.compile_if()),
            SchemaTokenKind::NodeStart { name } if local_template_name(name) == "try" => {
                Some(self.compile_try())
            }
            SchemaTokenKind::NodeStart { name } if local_template_name(name) == "catch" => {
                Some(self.compile_catch())
            }
            SchemaTokenKind::NodeStart { name } if is_choose_name(name) => {
                Some(self.compile_choose())
            }
            SchemaTokenKind::NodeStart { name } if is_for_each_name(name) => {
                Some(self.compile_for_each())
            }
            SchemaTokenKind::NodeStart { name } if is_project_payload_name(name) => {
                Some(self.compile_project_payload())
            }
            SchemaTokenKind::NodeStart { name } if is_variable_name(name) => {
                Some(self.compile_variable())
            }
            SchemaTokenKind::NodeStart { name }
                if name == "cem-data" || name == "cem:read-data" =>
            {
                Some(self.compile_data_reader())
            }
            SchemaTokenKind::NodeStart { .. } => Some(self.compile_element()),
            SchemaTokenKind::Text(text) | SchemaTokenKind::Trivia(text) => {
                let text = text.clone();
                let token = self.tokens[self.index].clone();
                self.index += 1;
                Some(TemplateNode::Text {
                    text,
                    source_map: frame_for(&token),
                })
            }
            // Triple-backtick rich content is verbatim text: its body is emitted as-is with
            // braces preserved, so generators can produce output that itself contains literal
            // `{`/`}` (e.g. CSS rule blocks `:root { … }`) without colliding with cem-ml's
            // structural braces. No interpolation happens inside — pair it with sibling
            // `{cem:for-each …}`/`{$…}` nodes for the dynamic parts.
            SchemaTokenKind::RichContent { data } => {
                let text = data.clone();
                let token = self.tokens[self.index].clone();
                self.index += 1;
                Some(TemplateNode::Text {
                    text,
                    source_map: frame_for(&token),
                })
            }
            SchemaTokenKind::Comment(text) => {
                let text = text.clone();
                let token = self.tokens[self.index].clone();
                self.index += 1;
                Some(TemplateNode::Comment {
                    text,
                    source_map: frame_for(&token),
                })
            }
            _ => {
                self.index += 1;
                None
            }
        }
    }

    /// Parse children until the `NodeEnd` matching `tag` (or an unnamed close `}`).
    fn parse_children(&mut self, tag: &str) -> Vec<TemplateNode> {
        let outer_bindings = self.compile_context.policy_bindings.clone();
        let mut children = Vec::new();
        while self.index < self.tokens.len() {
            if let SchemaTokenKind::NodeEnd { name: end } = &self.tokens[self.index].kind {
                let closes = end.as_deref().map(|end| end == tag).unwrap_or(true);
                self.index += 1;
                if closes {
                    break;
                }
                continue;
            }
            if let Some(node) = self.compile_node() {
                children.push(node);
            }
        }
        self.compile_context.policy_bindings = outer_bindings;
        children
    }

    fn parse_attributes(&mut self, tag: &str) -> Vec<TemplateAttribute> {
        let mut attributes = Vec::new();
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let token = self.tokens[self.index].clone();
                    let compiled_value = value.as_ref().map(|value| {
                        if local_template_name(tag) == "attribute" && name == "pattern" {
                            TemplateAttributeValue::Literal(value.clone())
                        } else if (local_template_name(tag) == "attribute" && name == "select")
                            || (local_template_name(tag) == "template" && name == "match")
                            || (local_template_name(tag) == "apply-templates" && name == "select")
                            || (local_template_name(tag) == "result-sequence" && name == "select")
                        {
                            TemplateAttributeValue::Expression(
                                self.compile_expression(value, &token),
                            )
                        } else {
                            self.compile_attribute_value(value, &token)
                        }
                    });
                    attributes.push(TemplateAttribute {
                        name: name.clone(),
                        value: compiled_value,
                        source_map: frame_for(&token),
                    });
                    self.index += 1;
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        attributes
    }

    fn compile_try(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let attributes = self.parse_attributes(&tag);
        let outer = self.compile_context.policy_bindings.clone();
        let mut children = Vec::new();
        let mut saw_catch = false;
        self.element_stack.push(tag.clone());
        while self.index < self.tokens.len() {
            if matches!(
                self.tokens[self.index].kind,
                SchemaTokenKind::NodeEnd { .. }
            ) {
                self.index += 1;
                break;
            }
            let is_catch = matches!(&self.tokens[self.index].kind, SchemaTokenKind::NodeStart { name } if local_template_name(name) == "catch");
            if is_catch {
                saw_catch = true;
                self.compile_context.policy_bindings = outer.clone();
            } else if saw_catch
                && !matches!(
                    &self.tokens[self.index].kind,
                    SchemaTokenKind::Trivia(_) | SchemaTokenKind::Comment(_)
                )
            {
                self.diagnostics.push(render_diagnostic(
                    "cem.ql.render.try_invalid_child",
                    "only catch handlers may follow the first catch".into(),
                    start.byte_range.start,
                    frame_for(&start),
                ));
            }
            if let Some(child) = self.compile_node() {
                children.push(child);
            }
        }
        self.element_stack.pop();
        self.compile_context.policy_bindings = outer;
        if !saw_catch || !attributes.is_empty() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.try_invalid",
                "try requires at least one catch and does not accept attributes".into(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        TemplateNode::Element {
            tag,
            attributes,
            children,
            source_map: frame_for(&start),
        }
    }

    fn compile_catch(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        if self
            .element_stack
            .last()
            .is_none_or(|p| local_template_name(p) != "try")
        {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.catch_outside_try",
                "catch must be a direct child of try".into(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        let mut raw = Vec::new();
        while self.index < self.tokens.len() {
            let token = self.tokens[self.index].clone();
            match &token.kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    raw.push((name.clone(), value.clone().unwrap_or_default(), token))
                }
                SchemaTokenKind::Trivia(_) => {}
                _ => break,
            }
            self.index += 1;
        }
        let name = raw
            .iter()
            .find(|(n, _, _)| n == "as")
            .map(|(_, v, _)| v.clone())
            .unwrap_or_else(|| "error".into());
        let tokens = crate::lexer::Lexer::new(&name).scan_all();
        if !matches!(
            tokens.as_slice(),
            [
                crate::lexer::Token {
                    kind: crate::lexer::TokenKind::Ident,
                    ..
                },
                crate::lexer::Token {
                    kind: crate::lexer::TokenKind::EndOfInput,
                    ..
                }
            ]
        ) {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.catch_invalid_binding",
                "catch @as requires a static query identifier".into(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        let outer = self.compile_context.policy_bindings.clone();
        self.compile_context
            .policy_bindings
            .insert(name.clone(), ItemStream::empty());
        let mut attributes = vec![TemplateAttribute {
            name: "as".into(),
            value: Some(TemplateAttributeValue::Literal(name)),
            source_map: frame_for(&start),
        }];
        let mut seen = BTreeSet::new();
        for (key, value, token) in raw {
            if !seen.insert(key.clone()) || !matches!(key.as_str(), "as" | "test") {
                self.diagnostics.push(render_diagnostic(
                    "cem.ql.render.catch_invalid_attribute",
                    "catch accepts only one static @as and one query @test".into(),
                    token.byte_range.start,
                    frame_for(&token),
                ));
            }
            if key == "test" {
                attributes.push(TemplateAttribute {
                    name: key,
                    value: Some(TemplateAttributeValue::Expression(
                        self.compile_expression(&value, &token),
                    )),
                    source_map: frame_for(&token),
                });
            }
        }
        self.element_stack.push(tag.clone());
        let children = self.parse_children(&tag);
        self.element_stack.pop();
        self.compile_context.policy_bindings = outer;
        TemplateNode::Element {
            tag,
            attributes,
            children,
            source_map: frame_for(&start),
        }
    }

    fn compile_element(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let SchemaTokenKind::NodeStart { name } = &start.kind else {
            unreachable!("compile_element is called only at NodeStart");
        };
        let tag = name.clone();
        self.index += 1;
        let mut attributes = self.parse_attributes(&tag);
        if local_template_name(&tag) == "template" && literal_template_attribute(&attributes, "on").as_deref() == Some("expression") {
            attributes.push(TemplateAttribute { name: "__cem-hook-id".into(), value: Some(TemplateAttributeValue::Literal(format!("{}:{}", self.source_identity, start.byte_range.start))), source_map: frame_for(&start) });
        }
        if local_template_name(&tag) == "template"
            && attributes.iter().any(|attribute| attribute.name == "match" || attribute.name == "on")
        {
            for attribute in &attributes {
                let valid = match attribute.name.as_str() {
                    "on" => matches!(&attribute.value, Some(TemplateAttributeValue::Literal(value)) if value == "expression"),
                    "into" => matches!(&attribute.value, Some(TemplateAttributeValue::Literal(value)) if value == "content" || value == "attribute"),
                    "match" => attribute.value.is_some(),
                    "mode" => matches!(attribute.value, Some(TemplateAttributeValue::Literal(_))),
                    "priority" => {
                        matches!(&attribute.value, Some(TemplateAttributeValue::Literal(value)) if value.parse::<i64>().is_ok())
                    }
                    _ => true,
                };
                if !valid {
                    self.diagnostics.push(render_diagnostic(
                        "cem.ql.render.match_rule_invalid",
                        "match rules require a predicate, a static mode and a signed integer priority".into(),
                        source_map_start(&attribute.source_map), attribute.source_map.clone(),
                    ));
                }
            }
        }
        if local_template_name(&tag) == "apply-templates"
            && !attributes
                .iter()
                .any(|attribute| attribute.name == "select" && attribute.value.is_some())
        {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.match_select_missing",
                "apply-templates requires an explicit select expression".into(),
                source_map_start(&frame_for(&start)),
                frame_for(&start),
            ));
        }
        let mut children = if self.should_skip_cemt_function_body(&tag) {
            self.skip_children(&tag);
            Vec::new()
        } else {
            self.element_stack.push(tag.clone());
            let children = self.parse_children(&tag);
            self.element_stack.pop();
            children
        };
        if local_template_name(&tag) == "template"
            && attributes.iter().any(|attribute| matches!(attribute.name.as_str(), "name" | "match" | "on"))
        {
            let body_count = children.iter().filter(|child| {
                matches!(child, TemplateNode::Element { tag, .. } if local_template_name(tag) == "body")
            }).count();
            let has_direct_content = children.iter().any(|child| match child {
                TemplateNode::Element { tag, .. } if matches!(local_template_name(tag), "param" | "body") => false,
                TemplateNode::Text { text, .. } if text.trim().is_empty() => false,
                _ => true,
            });
            if let Some(message) = cem_ml::transform_template::template_body_layout_error(body_count, has_direct_content) {
                self.diagnostics.push(render_diagnostic(
                    cem_ml::transform_template::TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE,
                    message.into(), start.byte_range.start, frame_for(&start),
                ));
                // Do not silently render only the first body from an invalid declaration.
                children.clear();
            }
        }
        let instruction = match local_template_name(&tag) {
            "result-sequence" => Some(ResultInstruction::Sequence),
            "result-element" => Some(ResultInstruction::Element),
            "result-attribute" => Some(ResultInstruction::Attribute),
            "result-document" => Some(ResultInstruction::Document),
            _ => None,
        };
        if let Some(instruction) = instruction {
            if let Some(message) = construction::validate_instruction(instruction, &attributes, &children) {
                self.diagnostics.push(render_diagnostic("cem.ql.result.instruction", message.into(), source_map_start(&frame_for(&start)), frame_for(&start)));
            }
            TemplateNode::Result { instruction, attributes, children, source_map: frame_for(&start) }
        } else {
            TemplateNode::Element { tag, attributes, children, source_map: frame_for(&start) }
        }
    }

    fn should_skip_cemt_function_body(&self, tag: &str) -> bool {
        self.skip_cemt_function_bodies
            && local_template_name(tag) == "body"
            && self.element_stack.last().is_some_and(|parent| {
                is_cemt_runtime_function_declaration_name(local_template_name(parent))
            })
    }

    fn skip_children(&mut self, tag: &str) {
        let mut depth = 0usize;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::NodeStart { .. } => {
                    depth += 1;
                    self.index += 1;
                }
                SchemaTokenKind::NodeEnd { name } => {
                    let closes_current = name.as_deref().map(|end| end == tag).unwrap_or(true);
                    self.index += 1;
                    if depth == 0 && closes_current {
                        break;
                    }
                    depth = depth.saturating_sub(1);
                }
                _ => self.index += 1,
            }
        }
    }

    fn compile_if(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let parsed_test = self.parse_test_attribute();
        let test = self.require_test_attribute(parsed_test, &start, &tag);
        let children = self.parse_children(&tag);
        TemplateNode::If {
            test,
            children,
            source_map: frame_for(&start),
        }
    }

    fn compile_choose(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        self.skip_attributes();
        let mut branches = Vec::new();
        let mut has_otherwise = false;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::NodeEnd { name: end }
                    if end.as_deref().map(|end| end == tag).unwrap_or(true) =>
                {
                    self.index += 1;
                    break;
                }
                SchemaTokenKind::NodeStart { name } if is_when_name(name) => {
                    branches.push(self.compile_branch(true));
                }
                SchemaTokenKind::NodeStart { name } if is_otherwise_name(name) => {
                    let otherwise = self.tokens[self.index].clone();
                    if has_otherwise {
                        self.diagnostics.push(render_diagnostic(
                            "cem.ql.render.choose_multiple_otherwise",
                            "`cem:choose` must not contain more than one `cem:otherwise` branch"
                                .to_owned(),
                            otherwise.byte_range.start,
                            frame_for(&otherwise),
                        ));
                    }
                    has_otherwise = true;
                    branches.push(self.compile_branch(false));
                }
                SchemaTokenKind::NodeStart { .. } => {
                    let token = self.tokens[self.index].clone();
                    let name = node_start_name(&token);
                    self.diagnostics.push(render_diagnostic(
                        "cem.ql.render.choose_invalid_child",
                        format!(
                            "`cem:choose` direct children must be `cem:when` or `cem:otherwise`; found `{name}`"
                        ),
                        token.byte_range.start,
                        frame_for(&token),
                    ));
                    let _ = self.compile_element();
                }
                _ => self.index += 1,
            }
        }
        TemplateNode::Choose {
            branches,
            source_map: frame_for(&start),
        }
    }

    fn compile_branch(&mut self, is_when: bool) -> ChooseBranch {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let test = if is_when {
            let parsed_test = self.parse_test_attribute();
            self.require_test_attribute(parsed_test, &start, &tag)
        } else {
            self.skip_otherwise_attributes();
            None
        };
        let children = self.parse_children(&tag);
        ChooseBranch { test, children }
    }

    fn compile_for_each(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let (select, as_name) = self.parse_for_each_attributes(&start);
        let loop_name = as_name.unwrap_or_else(|| "item".to_owned());
        // Declare the loop variable so descendant `{$ <name>}` expressions compile; restore the
        // prior declaration state after the block so the binding does not leak out of scope.
        let pre_existing = self
            .compile_context
            .policy_bindings
            .contains_key(&loop_name);
        self.compile_context
            .policy_bindings
            .entry(loop_name.clone())
            .or_insert_with(ItemStream::empty);
        // Also declare `position` (XSLT `position()` parity) so descendant `{$ position}` compiles.
        let position_pre_existing = self
            .compile_context
            .policy_bindings
            .contains_key(POSITION_BINDING);
        self.compile_context
            .policy_bindings
            .entry(POSITION_BINDING.to_owned())
            .or_insert_with(ItemStream::empty);
        let children = self.parse_children(&tag);
        if !pre_existing {
            self.compile_context.policy_bindings.remove(&loop_name);
        }
        if !position_pre_existing {
            self.compile_context
                .policy_bindings
                .remove(POSITION_BINDING);
        }
        TemplateNode::ForEach {
            select,
            as_name: loop_name,
            children,
            source_map: frame_for(&start),
        }
    }

    fn compile_project_payload(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let mut select = None;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let token = self.tokens[self.index].clone();
                    let raw = value.clone().unwrap_or_default();
                    self.index += 1;
                    if name == "select" {
                        select = Some(self.compile_expression(&raw, &token));
                    }
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        if select.is_none() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.project_payload_missing_select",
                "`cem:project-payload` requires a `@select` expression".to_owned(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        let children = self.parse_children(&tag);
        if !children.is_empty() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.project_payload_children_ignored",
                "`cem:project-payload` does not accept template children".to_owned(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        TemplateNode::ProjectPayload {
            select,
            source_map: frame_for(&start),
        }
    }

    /// A declarative reader is a scoped native binding, not a browser AST transport.
    fn compile_data_reader(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let mut name = String::new();
        let mut selected = None;
        let mut content_type = None;
        let mut projection = None;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute {
                    name: key, value, ..
                } => {
                    let raw = value.clone().unwrap_or_default();
                    match key.as_str() {
                        "name" => name = raw.trim().to_owned(),
                        "select" => selected = Some(raw),
                        "type" => content_type = Some(raw),
                        "projection" => projection = Some(raw),
                        _ => {}
                    }
                    self.index += 1;
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        let children = self.parse_children(&tag);
        let valid = valid_variable_name(&name)
            && selected.is_some()
            && content_type.is_some()
            && children
                .iter()
                .all(|n| matches!(n, TemplateNode::Text { text, .. } if text.trim().is_empty()));
        let select = if valid {
            let content_type = content_type.unwrap();
            let type_expression = whole_avt_expression(&content_type)
                .map(normalize_host_expression)
                .map(str::to_owned)
                .unwrap_or_else(|| serde_json::to_string(&content_type).expect("string literal"));
            let projection_argument = projection
                .map(|projection| {
                    let expression = whole_avt_expression(&projection)
                        .map(normalize_host_expression)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            serde_json::to_string(&projection).expect("string literal")
                        });
                    format!(", {expression}")
                })
                .unwrap_or_default();
            Some(self.compile_expression(
                &format!(
                    "data:read({}, {type_expression}{projection_argument})",
                    selected.unwrap()
                ),
                &start,
            ))
        } else {
            self.diagnostics.push(render_diagnostic("cem.ql.render.data_reader_invalid",
                "`cem-data` requires an identifier `name`, a source `select` expression and a `type`; it has no content".into(),
                start.byte_range.start, frame_for(&start)));
            None
        };
        if valid {
            self.compile_context
                .policy_bindings
                .insert(name.clone(), ItemStream::empty());
        }
        TemplateNode::Variable {
            name,
            select,
            source_map: frame_for(&start),
        }
    }

    fn compile_variable(&mut self) -> TemplateNode {
        let start = self.tokens[self.index].clone();
        let tag = node_start_name(&start);
        self.index += 1;
        let mut name = None;
        let mut select = None;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute {
                    name: attribute_name,
                    value,
                    ..
                } => {
                    let token = self.tokens[self.index].clone();
                    let raw = value.clone().unwrap_or_default();
                    self.index += 1;
                    match attribute_name.as_str() {
                        "name" => {
                            let candidate = raw.trim().trim_start_matches('$');
                            if valid_variable_name(candidate) {
                                name = Some(candidate.to_owned());
                            } else {
                                self.diagnostics.push(render_diagnostic(
                                    "cem.ql.render.variable_name_invalid",
                                    "`cem:variable` requires an identifier-valued `@name`"
                                        .to_owned(),
                                    token.byte_range.start,
                                    frame_for(&token),
                                ));
                            }
                        }
                        "select" => select = Some(self.compile_expression(&raw, &token)),
                        _ => {}
                    }
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        let children = self.parse_children(&tag);
        if !children.is_empty() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.variable_children_ignored",
                "`cem:variable` does not accept template children".to_owned(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        if name.is_none() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.variable_name_missing",
                "`cem:variable` requires a valid `@name`".to_owned(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        if select.is_none() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.variable_select_missing",
                "`cem:variable` requires a `@select` expression".to_owned(),
                start.byte_range.start,
                frame_for(&start),
            ));
        }
        let name = name.unwrap_or_default();
        if !name.is_empty() {
            self.compile_context
                .policy_bindings
                .insert(name.clone(), ItemStream::empty());
        }
        TemplateNode::Variable {
            name,
            select,
            source_map: frame_for(&start),
        }
    }

    /// Parse `cem:for-each` attributes: `@select` (the sequence expression, required) and `@as`
    /// (the loop variable name, default `item`; a legacy leading `$` is tolerated). Other
    /// attributes are ignored.
    fn parse_for_each_attributes(
        &mut self,
        start: &SchemaToken,
    ) -> (Option<CompiledTemplateExpression>, Option<String>) {
        let mut select = None;
        let mut as_name = None;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let attr = name.clone();
                    let raw = value.clone().unwrap_or_default();
                    let token = self.tokens[self.index].clone();
                    self.index += 1;
                    match attr.as_str() {
                        "select" => select = Some(self.compile_expression(&raw, &token)),
                        "as" => {
                            let trimmed = raw.trim().trim_start_matches('$').to_owned();
                            if !trimmed.is_empty() {
                                as_name = Some(trimmed);
                            }
                        }
                        _ => {}
                    }
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        if select.is_none() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.for_each_missing_select",
                "`cem:for-each` requires a `@select` expression".to_owned(),
                start.byte_range.start,
                frame_for(start),
            ));
        }
        (select, as_name)
    }

    /// Compile the `@test` whole-expression attribute of a conditional, ignoring others.
    fn parse_test_attribute(&mut self) -> Option<CompiledTemplateExpression> {
        let mut test = None;
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    let is_test = name == "test";
                    let raw = value.clone().unwrap_or_default();
                    let token = self.tokens[self.index].clone();
                    self.index += 1;
                    if is_test {
                        test = Some(self.compile_expression(&raw, &token));
                    }
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
        test
    }

    fn require_test_attribute(
        &mut self,
        test: Option<CompiledTemplateExpression>,
        token: &SchemaToken,
        conditional_name: &str,
    ) -> Option<CompiledTemplateExpression> {
        if test.is_none() {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.conditional_test_missing",
                format!("`{conditional_name}` requires a `@test` attribute"),
                token.byte_range.start,
                frame_for(token),
            ));
        }
        test
    }

    fn skip_otherwise_attributes(&mut self) {
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { name, .. } => {
                    let token = self.tokens[self.index].clone();
                    if name == "test" {
                        self.diagnostics.push(render_diagnostic(
                            "cem.ql.render.otherwise_test_not_allowed",
                            "`cem:otherwise` must not declare a `@test` attribute".to_owned(),
                            token.byte_range.start,
                            frame_for(&token),
                        ));
                    }
                    self.index += 1;
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
    }

    fn skip_attributes(&mut self) {
        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::Attribute { .. } | SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => break,
            }
        }
    }

    fn compile_expression_node(&mut self) -> CompiledTemplateExpression {
        let host = self.tokens[self.index].clone();
        self.index += 1;
        let mut source = String::new();

        while self.index < self.tokens.len() {
            match &self.tokens[self.index].kind {
                SchemaTokenKind::ExpressionNode(body) => {
                    source.push_str(body);
                    self.index += 1;
                }
                SchemaTokenKind::NodeEnd { name } if name.as_deref() == Some("$") => {
                    self.index += 1;
                    break;
                }
                SchemaTokenKind::Trivia(_) => self.index += 1,
                _ => self.index += 1,
            }
        }

        self.compile_expression(&source, &host)
    }

    fn compile_attribute_value(
        &mut self,
        value: &str,
        host: &SchemaToken,
    ) -> TemplateAttributeValue {
        if let Some(source) = whole_avt_expression(value) {
            return TemplateAttributeValue::Expression(self.compile_expression(source, host));
        }

        let parts = split_avt(value)
            .into_iter()
            .map(|part| match part {
                RawAttributePart::Literal(value) => TemplateAttributePart::Literal(value),
                RawAttributePart::Expression(source) => {
                    TemplateAttributePart::Expression(self.compile_expression(&source, host))
                }
            })
            .collect::<Vec<_>>();
        if parts.len() == 1 {
            if let Some(TemplateAttributePart::Literal(value)) = parts.first() {
                return TemplateAttributeValue::Literal(value.clone());
            }
        }
        TemplateAttributeValue::Template(parts)
    }

    fn compile_expression(
        &mut self,
        source: &str,
        host: &SchemaToken,
    ) -> CompiledTemplateExpression {
        let source = normalize_host_expression(source).to_owned();
        let query = match self.type_checking.compile(&source, &self.compile_context) {
            Ok(mut query) => {
                // Keep the enclosing template slot before the query-local frames
                // so caught native diagnostics still identify their source host.
                let host_frames = frame_for(host).frames;
                for source_map in &mut query.tree.source_maps {
                    source_map.frames.splice(0..0, host_frames.iter().cloned());
                }
                Some(query)
            }
            Err(error) => {
                self.diagnostics.push(render_diagnostic(
                    "cem.ql.render.compile_failed",
                    format!("template expression `{source}` failed to compile: {error}"),
                    host.byte_range.start,
                    host.source_map.clone(),
                ));
                None
            }
        };
        CompiledTemplateExpression {
            source,
            query,
            source_map: frame_for(host),
            byte_offset: host.byte_range.start,
        }
    }
}

struct PlanRenderer<'a> {
    evaluation_context: EvaluationContext,
    diagnostics: Vec<Diagnostic>,
    templates: BTreeMap<String, TemplateBody>,
    match_rules: Vec<MatchRule>,
    call_depth: usize,
    max_call_depth: usize,
    safe_points: Option<SafePointPoller>,
    control: Option<(OperationControl, ExecutionScopeId)>,
    control_failed: bool,
    recovery_depth: usize,
    failure: Option<TemplateFailure>,
    calls: Option<&'a dyn TemplateCallHandler>,
    result_work: u64,
    result_bytes: u64,
    result_depth: usize,
    // Account extracted lexical bytes for the duration of this render. This is
    // not total heap accounting or ownership of the returned render plan.
    text_memory: Vec<cem_ml::operation_control::MemoryPermit>,
    hook_scopes: Vec<Vec<hooks::ExpressionHook>>,
    active_hooks: Vec<String>,
    capture_depth: Option<usize>,
    render_scope_depth: usize,
    value_types: BTreeMap<String, cem_ml::schema::document_model::AttributeValueContract>,
    attribute_contracts: BTreeMap<String, cem_ml::schema::document_model::AttributeValueContract>,
    expression_target: hooks::ExpressionTarget,
}

impl PlanRenderer<'_> {
    fn template_failure(&mut self, diagnostic: Diagnostic) {
        if self.recovery_depth > 0 {
            if self.failure.is_some() || self.control_failed {
                return;
            }
            self.failure = Some(TemplateFailure {
                error: EvalError::TypeError("template operation failed"),
                diagnostic: diagnostic.clone(),
            });
        }
        self.diagnostics.push(diagnostic);
    }

    fn apply_attribute_declaration_selects(
        &mut self,
        nodes: &[TemplateNode],
        host_attribute_updates: &mut Vec<HostAttributeUpdate>,
    ) {
        for node in nodes {
            let TemplateNode::Element {
                tag, attributes, ..
            } = node
            else {
                continue;
            };
            if local_template_name(tag) != "attribute" {
                continue;
            }
            let Some(name) = declaration_name(attributes) else {
                continue;
            };
            let Some(select) = declaration_select(attributes) else {
                continue;
            };
            let Some(query) = &select.query else {
                continue;
            };
            let stream = self.evaluate_query(query);
            self.diagnostics.extend(stream.diagnostics.clone());
            if let Some(error) = stream.error {
                self.diagnostics.push(render_diagnostic(
                    "cem.ql.render.attribute_select_failed",
                    format!(
                        "attribute `{name}` select `{}` failed: {error:?}",
                        select.source
                    ),
                    select.byte_offset,
                    select.source_map.clone(),
                ));
                continue;
            }
            let value = self.render_stream_text(&stream, &select.source_map);
            if self.failure.is_some() || self.control_failed {
                return;
            }
            self.evaluation_context
                .policy_bindings
                .insert(name.clone(), stream);
            set_current_data_document_attribute(
                &mut self.evaluation_context.policy_bindings,
                &name,
                &value,
            );
            host_attribute_updates.push(HostAttributeUpdate::new(name, value));
        }
    }

    fn render_nodes_scoped(
        &mut self,
        nodes: &[TemplateNode],
        out: &mut ResultBuffer,
        parent_attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        self.hook_scopes.push(Vec::new());
        self.render_scope_depth += 1;
        #[cfg(test)]
        let profile = crate::compile_profile::Span::new("copy/scoped-variable-snapshot");
        let names = nodes
            .iter()
            .filter_map(|node| match node {
                TemplateNode::Variable { name, .. } if !name.is_empty() => Some(name.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut previous = BTreeMap::new();
        for name in names {
            previous
                .entry(name.clone())
                .or_insert_with(|| self.evaluation_context.policy_bindings.get(&name).cloned());
        }
        #[cfg(test)]
        drop(profile);
        for node in nodes {
            self.render_into(node, out, parent_attributes);
        }
        self.render_scope_depth -= 1;
        self.hook_scopes.pop();
        for (name, value) in previous {
            match value {
                Some(stream) => {
                    self.evaluation_context.policy_bindings.insert(name, stream);
                }
                None => {
                    self.evaluation_context.policy_bindings.remove(&name);
                }
            }
        }
    }

    fn poll_render(&mut self, source_map: &SourceMapStack) -> bool {
        if self.control_failed || self.failure.is_some() {
            return false;
        }
        let result = self
            .safe_points
            .as_mut()
            .map(SafePointPoller::poll_one)
            .transpose();
        self.accept_control_check(result, source_map)
    }

    fn force_render(&mut self, source_map: &SourceMapStack) -> bool {
        if self.control_failed {
            return false;
        }
        let result = self
            .safe_points
            .as_mut()
            .map(SafePointPoller::force)
            .transpose();
        self.accept_control_check(result, source_map)
    }

    fn accept_control_check(
        &mut self,
        result: Result<Option<()>, cem_ml::operation_control::ControlError>,
        source_map: &SourceMapStack,
    ) -> bool {
        let Err(error) = result else {
            return true;
        };
        if !self.control_failed {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.control_failure",
                format!("{}: {error}", error.code()),
                source_map_start(source_map),
                source_map.clone(),
            ));
        }
        self.control_failed = true;
        false
    }

    /// Render a template node, appending zero or more plan nodes to `out`. Conditionals
    /// (`cem:if`/`cem:choose`) contribute the children of the selected branch (or none),
    /// so they flatten into the surrounding sequence rather than emitting a wrapper.
    fn render_into(
        &mut self,
        node: &TemplateNode,
        out: &mut ResultBuffer,
        parent_attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        let source_map = template_node_source_map(node);
        let permitted = if self.result_depth > 0 {
            let bytes = match node {
                TemplateNode::Text { text, .. } | TemplateNode::Comment { text, .. } => text.len(),
                _ => 0,
            };
            self.charge_result(bytes, self.result_depth, source_map)
        } else { self.poll_render(source_map) };
        if !permitted {
            return;
        }
        match node {
            TemplateNode::Result { instruction, attributes, children, source_map } => {
                self.recovery_depth += 1;
                self.result_depth += 1;
                self.render_result(*instruction, attributes, children, source_map, out);
                self.result_depth -= 1;
                self.recovery_depth -= 1;
            }
            TemplateNode::Element {
                tag,
                attributes,
                children,
                source_map,
            } => {
                if let Some(calls) = self.calls.filter(|calls| calls.handles(tag)) {
                    let attributes = attributes
                        .iter()
                        .filter_map(|a| self.render_call_attribute(a))
                        .collect::<Vec<_>>();
                    if self.failure.is_some() || self.control_failed {
                        return;
                    }
                    let result = calls.call(
                        &attributes,
                        source_map,
                        &TemplateData {
                            bindings: self.evaluation_context.policy_bindings.clone(),
                            native_functions: self.evaluation_context.native_functions.clone(),
                            data_readers: self.evaluation_context.data_readers.clone(),
                            value_types: self.value_types.clone(),
                            attribute_contracts: self.attribute_contracts.clone(),
                            input_attribute_contracts: BTreeMap::new(),
                            expression_scope: ExpressionScope { scopes: self.hook_scopes.clone(), active: self.active_hooks.clone(), focus: self.evaluation_context.current_item.clone(), call_depth: self.call_depth + 1, target: self.expression_target.clone() },
                        },
                        self.recovery_depth > 0,
                    );
                    self.diagnostics.extend(result.plan.diagnostics);
                    if let Some(failure) = result.failure {
                        self.control_failed |= !failure.error.is_recoverable();
                        self.failure = Some(failure);
                    } else {
                        out.extend(result.plan.nodes);
                    }
                    return;
                }
                if hooks::is_expression_hook(node) {
                    self.register_hook(node);
                    return;
                }
                if local_template_name(tag) == "try" {
                    self.render_try(children, out, parent_attributes);
                    return;
                }
                if local_template_name(tag) == "catch" {
                    return;
                }
                if local_template_name(tag) == "call" {
                    self.render_call_into(attributes, source_map, out, parent_attributes);
                    return;
                }
                if local_template_name(tag) == "apply-templates" {
                    self.render_matching_templates(attributes, source_map, out, parent_attributes);
                    return;
                }
                if local_template_name(tag) == "body" {
                    self.render_nodes_scoped(children, out, parent_attributes);
                    return;
                }
                if local_template_name(tag) == "param" || is_named_template_declaration(node) {
                    return;
                }
                if local_template_name(tag) == "attribute" {
                    if let Some(attribute) =
                        self.render_constructed_attribute(attributes, children, source_map)
                    {
                        parent_attributes.push(attribute);
                    }
                    return;
                }
                if local_template_name(tag) == "element" {
                    self.render_constructed_element(attributes, children, source_map, out);
                    return;
                }
                if local_template_name(tag) == "comment" {
                    out.push(RenderPlanNode::Comment {
                        text: self.render_constructor_text(attributes, children, "value"),
                        source_map: source_map.clone(),
                    });
                    return;
                }
                if local_template_name(tag) == "cdata" {
                    out.push(RenderPlanNode::Cdata {
                        text: self.render_constructor_text(attributes, children, "value"),
                        source_map: source_map.clone(),
                    });
                    return;
                }
                if local_template_name(tag) == "processing-instruction" {
                    let Some((target, target_source_map)) = self.render_constructor_name_or_alias(
                        attributes,
                        &["target", "name"],
                        source_map,
                        "processing-instruction",
                    ) else {
                        return;
                    };
                    out.push(RenderPlanNode::ProcessingInstruction {
                        target,
                        data: self.render_constructor_text(attributes, children, "value"),
                        source_map: target_source_map,
                    });
                    return;
                }
                let mut attributes = attributes
                    .iter()
                    .filter_map(|attribute| self.render_attribute(attribute))
                    .collect::<Vec<_>>();
                let mut child_nodes = ResultBuffer::default();
                self.render_content_nodes(children, &mut child_nodes, &mut attributes);
                out.push(RenderPlanNode::Element {
                    qualified_name: None,
                    tag: tag.clone(),
                    namespace: None,
                    attributes,
                    children: self.finish_result_buffer(child_nodes, source_map),
                    source_map: source_map.clone(),
                });
            }
            TemplateNode::Text { text, source_map } => out.push(RenderPlanNode::Text {
                text: text.clone(),
                source_map: source_map.clone(),
            }),
            TemplateNode::Comment { text, source_map } => out.push(RenderPlanNode::Comment {
                text: text.clone(),
                source_map: source_map.clone(),
            }),
            TemplateNode::Expression(expression) => {
                let stream = self.evaluate_to_stream(expression);
                if self.capture_depth == Some(self.render_scope_depth) {
                    out.values(stream, &expression.source_map);
                } else {
                    self.insert_expression_values(stream, &expression.source_map, out);
                }
            }
            TemplateNode::Variable { name, select, .. } => {
                if !name.is_empty() {
                    let value = select
                        .as_ref()
                        .map(|expression| self.evaluate_to_stream(expression))
                        .unwrap_or_default();
                    self.evaluation_context
                        .policy_bindings
                        .insert(name.clone(), value);
                }
            }
            TemplateNode::If { test, children, .. } => {
                if self.test_is_truthy(test.as_ref()) {
                    self.render_nodes_scoped(children, out, parent_attributes);
                }
            }
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    let matched = match &branch.test {
                        None => true,
                        Some(test) => self.test_is_truthy(Some(test)),
                    };
                    if matched {
                        self.render_nodes_scoped(&branch.children, out, parent_attributes);
                        break;
                    }
                }
            }
            TemplateNode::ForEach {
                select,
                as_name,
                children,
                ..
            } => {
                let items = self.evaluate_select(select.as_ref());
                let previous = self
                    .evaluation_context
                    .policy_bindings
                    .get(as_name)
                    .cloned();
                // XSLT `position()` parity: bind a 1-based index for the current iteration. Saved
                // and restored alongside the loop variable so nested loops see their own position.
                let previous_position = self
                    .evaluation_context
                    .policy_bindings
                    .get(POSITION_BINDING)
                    .cloned();
                for (offset, item) in items.into_iter().enumerate() {
                    if !self.poll_render(source_map) {
                        break;
                    }
                    self.evaluation_context
                        .policy_bindings
                        .insert(as_name.clone(), ItemStream::once(item));
                    self.evaluation_context.policy_bindings.insert(
                        POSITION_BINDING.to_owned(),
                        ItemStream::once(Item::Atomic(AtomValue::Integer((offset + 1) as i64))),
                    );
                    self.render_nodes_scoped(children, out, parent_attributes);
                }
                // Restore the prior bindings so the loop variables do not leak past the block.
                match previous {
                    Some(prev) => {
                        self.evaluation_context
                            .policy_bindings
                            .insert(as_name.clone(), prev);
                    }
                    None => {
                        self.evaluation_context.policy_bindings.remove(as_name);
                    }
                }
                match previous_position {
                    Some(prev) => {
                        self.evaluation_context
                            .policy_bindings
                            .insert(POSITION_BINDING.to_owned(), prev);
                    }
                    None => {
                        self.evaluation_context
                            .policy_bindings
                            .remove(POSITION_BINDING);
                    }
                }
            }
            TemplateNode::ProjectPayload { select, source_map } => {
                for item in self.evaluate_select(select.as_ref()) {
                    if !self.poll_render(source_map) {
                        break;
                    }
                    match payload_item_to_render_node(&item, source_map) {
                        Some(node) => out.push(node),
                        None => self.template_failure(render_diagnostic(
                            "cem.ql.render.project_payload_invalid_node",
                            "`cem:project-payload` selected a value that is not a serialized payload node"
                                .to_owned(),
                            source_map_start(source_map),
                            source_map.clone(),
                        )),
                    }
                }
            }
        }
    }

    fn render_matching_templates(
        &mut self,
        attributes: &[TemplateAttribute],
        source_map: &SourceMapStack,
        out: &mut ResultBuffer,
        parent_attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        if !self.force_render(source_map) {
            return;
        }
        if self.call_depth >= self.max_call_depth {
            if self.recovery_depth > 0 {
                self.control_failed = true;
            }
            self.diagnostics.push(render_diagnostic(
                "cem.transform_template.recursion_limit",
                "native template match recursion limit exceeded".into(),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return;
        }
        // Selection is a node context, not a text interpolation context.
        let selected = attributes
            .iter()
            .find(|a| a.name == "select")
            .map(|a| self.render_attribute_stream(a))
            .unwrap_or_default();
        let attributes: Vec<_> = attributes
            .iter()
            .filter(|a| a.name != "select")
            .filter_map(|a| self.render_call_attribute(a))
            .collect();
        let mode = attributes
            .iter()
            .find(|a| a.name == "mode")
            .map(|a| a.value.as_str())
            .unwrap_or("");
        let mut previous = BTreeMap::new();
        for attribute in attributes
            .iter()
            .filter(|a| a.name.starts_with("with:") && a.name != "with:node")
        {
            let name = attribute.name.trim_start_matches("with:").to_owned();
            previous.insert(
                name.clone(),
                self.evaluation_context
                    .policy_bindings
                    .insert(name, attribute.value_stream.clone()),
            );
        }
        self.dispatch_values(selected, mode, source_map, out, parent_attributes);
        for (name, value) in previous {
            if let Some(value) = value {
                self.evaluation_context.policy_bindings.insert(name, value);
            } else {
                self.evaluation_context.policy_bindings.remove(&name);
            }
        }
    }

    fn dispatch_values(
        &mut self, selected: ItemStream, mode: &str, source_map: &SourceMapStack,
        out: &mut ResultBuffer, parent_attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        if self.call_depth >= self.max_call_depth {
            self.control_failed = true;
            self.diagnostics.push(render_diagnostic(
                "cem.transform_template.recursion_limit",
                "native template match recursion limit exceeded".into(),
                source_map_start(source_map), source_map.clone(),
            ));
            return;
        }
        let previous_node = self.evaluation_context.policy_bindings.get("node").cloned();
        let rules = {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("copy/match-rules");
            self.match_rules.clone()
        };
        let previous_focus = self.evaluation_context.current_item.clone();
        for item in selected
            .items
            .into_iter()
            .flat_map(|item| item.members().unwrap_or_else(|| vec![item]))
        {
            if !self.poll_render(source_map) {
                break;
            }
            self.evaluation_context.current_item = Some(item.clone());
            self.evaluation_context
                .policy_bindings
                .insert("node".into(), ItemStream::once(item));
            for rule in rules.iter().filter(|rule| rule.mode == mode) {
                if self.test_is_truthy(Some(&rule.test)) {
                    self.call_depth += 1;
                    self.render_template_body(&rule.body, out, parent_attributes);
                    self.call_depth -= 1;
                    break;
                }
            }
        }
        self.evaluation_context.current_item = previous_focus;
        if let Some(previous) = previous_node {
            self.evaluation_context.policy_bindings.insert("node".into(), previous);
        } else {
            self.evaluation_context.policy_bindings.remove("node");
        }
    }

    fn render_call_into(
        &mut self,
        attributes: &[TemplateAttribute],
        source_map: &SourceMapStack,
        out: &mut ResultBuffer,
        parent_attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        if !self.force_render(source_map) {
            return;
        }
        if self.call_depth >= self.max_call_depth {
            if self.recovery_depth > 0 {
                self.control_failed = true;
            }
            self.diagnostics.push(render_diagnostic(
                "cem.transform_template.recursion_limit",
                format!(
                    "native template call recursion limit exceeded at depth {}; max depth is {}",
                    self.call_depth, self.max_call_depth
                ),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return;
        }

        let rendered_attributes = attributes
            .iter()
            .filter_map(|attribute| self.render_call_attribute(attribute))
            .collect::<Vec<_>>();
        let Some(template_name) = rendered_attributes
            .iter()
            .find(|attribute| attribute.name == "template")
            .map(|attribute| attribute.value.clone())
            .filter(|value| !value.is_empty())
        else {
            self.template_failure(render_diagnostic(
                "cem.transform_template.call_unknown",
                "native template call is missing a `template` target".to_owned(),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return;
        };
        let Some(template_nodes) = ({
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("copy/call-template");
            self.templates.get(&template_name).cloned()
        }) else {
            self.template_failure(render_diagnostic(
                "cem.transform_template.call_unknown",
                format!("native template call target `{template_name}` was not compiled"),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return;
        };

        let mut previous = BTreeMap::new();
        for attribute in rendered_attributes
            .iter()
            .filter(|attribute| attribute.name.starts_with("with:"))
        {
            let name = attribute.name.trim_start_matches("with:").to_owned();
            previous.insert(
                name.clone(),
                self.evaluation_context
                    .policy_bindings
                    .insert(name, attribute.value_stream.clone()),
            );
        }

        self.call_depth += 1;
        self.render_template_body(&template_nodes, out, parent_attributes);
        self.call_depth -= 1;

        for (name, value) in previous {
            match value {
                Some(stream) => {
                    self.evaluation_context.policy_bindings.insert(name, stream);
                }
                None => {
                    self.evaluation_context.policy_bindings.remove(&name);
                }
            }
        }
    }

    /// Evaluate a `cem:for-each` `@select` expression to the sequence of items to iterate.
    ///
    /// A selected `Item::Array` is flattened one level into its members, so iterating a
    /// data-document collection (e.g. `datadom.slices.geometry` — the token rows the host
    /// bridge shapes from a `<table>`, delivered as a single array item) yields one iteration
    /// per row, matching legacy XSLT `for-each` node-set iteration.
    /// A bare sequence already iterates per item, so only array items are expanded.
    fn evaluate_select(&mut self, select: Option<&CompiledTemplateExpression>) -> Vec<Item> {
        let Some(select) = select else {
            return Vec::new();
        };
        let Some(query) = &select.query else {
            return Vec::new();
        };
        let stream = self.evaluate_query(query);
        self.diagnostics.extend(stream.diagnostics.clone());
        if let Some(error) = stream.error {
            self.diagnostics.push(render_diagnostic(
                "cem.ql.render.for_each_failed",
                format!(
                    "`cem:for-each` select `{}` failed: {error:?}",
                    select.source
                ),
                select.byte_offset,
                select.source_map.clone(),
            ));
            return Vec::new();
        }
        stream
            .items
            .into_iter()
            .flat_map(|item| item.members().unwrap_or_else(|| vec![item]))
            .collect()
    }

    /// Evaluate a conditional `@test` expression to a cem-ql effective-boolean.
    fn test_is_truthy(&mut self, test: Option<&CompiledTemplateExpression>) -> bool {
        let Some(test) = test else {
            return false;
        };
        let Some(query) = &test.query else {
            return false;
        };
        let stream = self.evaluate_query(query);
        self.diagnostics.extend(stream.diagnostics.clone());
        if let Some(error) = stream.error {
            self.template_failure(render_diagnostic(
                "cem.ql.render.test_failed",
                format!("conditional test `{}` failed: {error:?}", test.source),
                test.byte_offset,
                test.source_map.clone(),
            ));
            return false;
        }
        effective_boolean(&stream.items)
    }

    fn render_attribute(&mut self, attribute: &TemplateAttribute) -> Option<RenderPlanAttribute> {
        let contract = self.attribute_contracts.get(&attribute.name).cloned().map(std::sync::Arc::new);
        let old = std::mem::replace(&mut self.expression_target, hooks::ExpressionTarget::Attribute {
            name: attribute.name.clone(), contract: contract.clone(),
        });
        let mut value_stream = self.output_attribute_stream(attribute);
        if let Some(contract) = &contract {
            value_stream = self.convert_values(value_stream, contract, &attribute.source_map);
        }
        let has_nodes = value_stream.items.iter().any(|item| item.view().is_some_and(|v| v.kind() == crate::eval::QueryItemViewKind::Node));
        let value = if has_nodes { String::new() } else { self.render_stream_text(&value_stream, &attribute.source_map) };
        self.expression_target = old;
        let preserves_empty_value = match &attribute.value {
            None => true,
            Some(TemplateAttributeValue::Literal(value)) => value.is_empty(),
            Some(TemplateAttributeValue::Template(_))
            | Some(TemplateAttributeValue::Expression(_)) => false,
        };
        if value.is_empty() && !has_nodes && !preserves_empty_value && !attribute.name.starts_with("with:") {
            return None;
        }
        Some(RenderPlanAttribute {
            contract,
            qualified_name: None,
            name: attribute.name.clone(),
            namespace: None,
            value,
            value_stream,
            source_map: attribute.source_map.clone(),
        })
    }

    fn render_attribute_value(&mut self, attribute: &TemplateAttribute) -> (String, ItemStream) {
        match &attribute.value {
            None => (String::new(), string_stream(String::new())),
            Some(TemplateAttributeValue::Literal(value)) => {
                (value.clone(), string_stream(value.clone()))
            }
            Some(TemplateAttributeValue::Template(parts)) => {
                let mut value = String::new();
                for part in parts {
                    if !self.poll_render(&attribute.source_map) {
                        break;
                    }
                    match part {
                        TemplateAttributePart::Literal(literal) => value.push_str(literal),
                        TemplateAttributePart::Expression(expression) => {
                            value.push_str(&self.evaluate_to_string(expression));
                        }
                    }
                }
                let value_stream = string_stream(value.clone());
                (value, value_stream)
            }
            Some(TemplateAttributeValue::Expression(expression)) => {
                let value_stream = self.evaluate_to_stream(expression);
                let value = self.render_stream_text(&value_stream, &expression.source_map);
                (value, value_stream)
            }
        }
    }

    fn render_constructed_element(
        &mut self,
        attributes: &[TemplateAttribute],
        children: &[TemplateNode],
        source_map: &SourceMapStack,
        out: &mut ResultBuffer,
    ) {
        let Some((tag, tag_source_map)) =
            self.render_constructor_name(attributes, "name", source_map, "element")
        else {
            return;
        };
        let namespace = self.render_constructor_optional_text(attributes, "namespace");
        let mut rendered_attributes = Vec::new();
        let mut rendered_children = ResultBuffer::default();
        self.render_content_nodes(children, &mut rendered_children, &mut rendered_attributes);
        sort_render_plan_attributes(&mut rendered_attributes);
        out.push(RenderPlanNode::Element {
            qualified_name: None,
            tag,
            namespace,
            attributes: rendered_attributes,
            children: self.finish_result_buffer(rendered_children, source_map),
            source_map: tag_source_map,
        });
    }

    fn render_constructed_attribute(
        &mut self,
        attributes: &[TemplateAttribute],
        children: &[TemplateNode],
        source_map: &SourceMapStack,
    ) -> Option<RenderPlanAttribute> {
        self.constructed_attribute_value(attributes, children, source_map)
    }

    fn render_constructor_text(
        &mut self,
        attributes: &[TemplateAttribute],
        children: &[TemplateNode],
        value_attribute_name: &str,
    ) -> String {
        if let Some(attribute) = attributes
            .iter()
            .find(|attribute| attribute.name == value_attribute_name)
        {
            let (value, _) = self.render_attribute_value(attribute);
            return value;
        }
        let mut ignored_attributes = Vec::new();
        let mut rendered_children = ResultBuffer::default();
        self.render_content_nodes(children, &mut rendered_children, &mut ignored_attributes);
        render_plan_nodes_to_text(&self.finish_result_buffer(rendered_children, &SourceMapStack::default()))
    }

    fn render_constructor_optional_text(
        &mut self,
        attributes: &[TemplateAttribute],
        attribute_name: &str,
    ) -> Option<String> {
        attributes
            .iter()
            .find(|attribute| attribute.name == attribute_name)
            .map(|attribute| self.render_attribute_value(attribute).0)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    }

    fn render_constructor_name_or_alias(
        &mut self,
        attributes: &[TemplateAttribute],
        attribute_names: &[&str],
        source_map: &SourceMapStack,
        construct: &str,
    ) -> Option<(String, SourceMapStack)> {
        for attribute_name in attribute_names {
            if attributes
                .iter()
                .any(|attribute| attribute.name == *attribute_name)
            {
                return self.render_constructor_name(
                    attributes,
                    attribute_name,
                    source_map,
                    construct,
                );
            }
        }
        self.template_failure(render_diagnostic(
            "cem.ql.render.dynamic_name_missing",
            format!(
                "`{construct}` constructor requires one of `{}`",
                attribute_names
                    .iter()
                    .map(|name| format!("@{name}"))
                    .collect::<Vec<_>>()
                    .join("`, `")
            ),
            source_map_start(source_map),
            source_map.clone(),
        ));
        None
    }

    fn render_constructor_name(
        &mut self,
        attributes: &[TemplateAttribute],
        attribute_name: &str,
        source_map: &SourceMapStack,
        construct: &str,
    ) -> Option<(String, SourceMapStack)> {
        let Some(attribute) = attributes
            .iter()
            .find(|attribute| attribute.name == attribute_name)
        else {
            self.template_failure(render_diagnostic(
                "cem.ql.render.dynamic_name_missing",
                format!("`{construct}` constructor requires `@{attribute_name}`"),
                source_map_start(source_map),
                source_map.clone(),
            ));
            return None;
        };
        let (value, _) = self.render_attribute_value(attribute);
        let Some(name) = normalize_constructed_name(&value) else {
            self.template_failure(render_diagnostic(
                "cem.ql.render.dynamic_name_invalid",
                format!("`{construct}` constructor name `{value}` is not a valid output name"),
                source_map_start(&attribute.source_map),
                attribute.source_map.clone(),
            ));
            return None;
        };
        Some((name, attribute.source_map.clone()))
    }

    fn evaluate_to_string(&mut self, expression: &CompiledTemplateExpression) -> String {
        let stream = self.evaluate_to_stream(expression);
        self.render_stream_text(&stream, &expression.source_map)
    }

    fn evaluate_to_stream(&mut self, expression: &CompiledTemplateExpression) -> ItemStream {
        let Some(query) = &expression.query else {
            return ItemStream::empty();
        };
        let stream = self.evaluate_query(query);
        self.diagnostics.extend(stream.diagnostics.clone());
        if let Some(error) = stream.error {
            self.template_failure(render_diagnostic(
                "cem.ql.render.eval_failed",
                format!(
                    "template expression `{}` failed: {error:?}",
                    expression.source
                ),
                expression.byte_offset,
                expression.source_map.clone(),
            ));
            return ItemStream::empty();
        }
        stream
    }

    fn evaluate_query(&mut self, query: &CompiledQuery) -> ItemStream {
        if self.failure.is_some() || self.control_failed {
            return ItemStream::empty();
        }
        let (control, scope) = self.control.clone().unwrap_or_else(|| (OperationControl::default(), cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID));
        let protected = self.recovery_depth > 0;
        // The conservative dependency proof excludes native extensions, CEMT
        // dispatch and unresolved calls, including calls inside function bodies.
        // Proven expressions need no mutable template host, so their context
        // can be borrowed. The evaluator still owns its selected binding values.
        let borrow_context = query.binding_dependencies().is_some();
        #[cfg(test)]
        let borrow_context = borrow_context && !copy_profile_tests::force_context_copy();
        let stream = if borrow_context {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("render/borrowed-evaluation");
            crate::eval::Evaluator::evaluate_internal(
                query, &self.evaluation_context, &control, scope, protected, None,
            )
        } else {
            let context = self.evaluation_context.for_query(query);
            crate::eval::Evaluator::evaluate_internal(
                query, &context, &control, scope, protected, Some(self),
            )
        };
        if self.recovery_depth > 0 {
            if let Some(error) = &stream.error {
                if let Some(diagnostic) = stream
                    .diagnostics
                    .iter()
                    .rev()
                    .find(|d| d.severity.is_hard_violation())
                    .cloned()
                {
                    self.failure = Some(TemplateFailure {
                        error: error.clone(),
                        diagnostic,
                    });
                }
                if !error.is_recoverable() {
                    self.control_failed = true;
                }
            }
        }
        stream
    }

    fn render_try(
        &mut self,
        children: &[TemplateNode],
        out: &mut ResultBuffer,
        parent_attributes: &mut Vec<RenderPlanAttribute>,
    ) {
        let first_catch = children.iter().position(|n| matches!(n, TemplateNode::Element { tag, .. } if local_template_name(tag) == "catch")).unwrap_or(children.len());
        let saved_bindings = {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("copy/try-snapshot");
            self.evaluation_context.policy_bindings.clone()
        };
        let diagnostic_start = self.diagnostics.len();
        let mut buffered = ResultBuffer::default();
        let mut attributes = parent_attributes.clone();
        self.recovery_depth += 1;
        self.render_nodes_scoped(&children[..first_catch], &mut buffered, &mut attributes);
        self.recovery_depth -= 1;
        self.evaluation_context.policy_bindings = {
            #[cfg(test)]
            let _profile = crate::compile_profile::Span::new("copy/try-restore");
            saved_bindings.clone()
        };
        if self.control_failed {
            return;
        }
        let Some(failure) = self.failure.take() else {
            out.extend(buffered);
            *parent_attributes = attributes;
            return;
        };
        if !failure.error.is_recoverable() {
            self.failure = Some(failure);
            return;
        }
        let protected_end = self.diagnostics.len();
        for child in &children[first_catch..] {
            let TemplateNode::Element {
                tag,
                attributes,
                children,
                ..
            } = child
            else {
                continue;
            };
            if local_template_name(tag) != "catch" {
                continue;
            }
            let name = attributes
                .iter()
                .find_map(|a| match (&*a.name, &a.value) {
                    ("as", Some(TemplateAttributeValue::Literal(name))) => Some(name.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "error".into());
            self.evaluation_context.policy_bindings.insert(
                name,
                ItemStream::once(crate::eval::diagnostic_item(failure.diagnostic.clone())),
            );
            let test = attributes.iter().find_map(|a| match (&*a.name, &a.value) {
                ("test", Some(TemplateAttributeValue::Expression(test))) => Some(test),
                _ => None,
            });
            self.recovery_depth += 1;
            let matched = test
                .map(|test| self.test_is_truthy(Some(test)))
                .unwrap_or(true);
            self.recovery_depth -= 1;
            if matched || self.failure.is_some() || self.control_failed {
                // Remove the handled failure and its render wrappers, retaining
                // independent diagnostics emitted by the protected computation.
                let tail = self.diagnostics.split_off(protected_end);
                let protected = self.diagnostics.split_off(diagnostic_start);
                self.diagnostics.extend(protected.into_iter().filter(|d| {
                    let is_caught_failure = d.code == failure.diagnostic.code
                        && d.message == failure.diagnostic.message
                        && d.source_map == failure.diagnostic.source_map;
                    let is_wrapper = matches!(
                        d.code.as_str(),
                        "cem.ql.render.eval_failed"
                            | "cem.ql.render.for_each_failed"
                            | "cem.ql.render.test_failed"
                    );
                    !(is_caught_failure || is_wrapper)
                }));
                self.diagnostics.extend(tail);
                if self.failure.is_none() && !self.control_failed {
                    let mut recovered = ResultBuffer::default();
                    let mut recovered_attributes = parent_attributes.clone();
                    self.recovery_depth += 1;
                    self.render_nodes_scoped(children, &mut recovered, &mut recovered_attributes);
                    self.recovery_depth -= 1;
                    if self.failure.is_none() && !self.control_failed {
                        out.extend(recovered);
                        *parent_attributes = recovered_attributes;
                    }
                }
                self.evaluation_context.policy_bindings = saved_bindings;
                return;
            }
            self.evaluation_context.policy_bindings = {
                #[cfg(test)]
                let _profile = crate::compile_profile::Span::new("copy/try-restore");
                saved_bindings.clone()
            };
        }
        self.evaluation_context.policy_bindings = saved_bindings;
        self.failure = Some(failure);
    }
}

fn string_stream(value: String) -> ItemStream {
    ItemStream::once(Item::Atomic(AtomValue::String(value)))
}

enum RawAttributePart {
    Literal(String),
    Expression(String),
}

fn split_avt(value: &str) -> Vec<RawAttributePart> {
    let mut out = Vec::new();
    let mut chars = value.char_indices().peekable();
    let mut literal_start = 0;
    while let Some((offset, c)) = chars.next() {
        if c != '{' {
            continue;
        }
        if matches!(chars.peek(), Some((_, '{'))) {
            let (_, next) = chars.next().expect("peeked char exists");
            debug_assert_eq!(next, '{');
            if literal_start < offset {
                out.push(RawAttributePart::Literal(
                    value[literal_start..offset].to_owned(),
                ));
            }
            out.push(RawAttributePart::Literal("{".to_owned()));
            literal_start = offset + 2;
            continue;
        }

        let mut depth = 1u32;
        let body_start = offset + 1;
        let mut body_end = None;
        while let Some((inner_offset, inner)) = chars.next() {
            match inner {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        body_end = Some(inner_offset);
                        break;
                    }
                }
                _ => {}
            }
        }

        if let Some(end) = body_end {
            if literal_start < offset {
                out.push(RawAttributePart::Literal(
                    value[literal_start..offset].to_owned(),
                ));
            }
            out.push(RawAttributePart::Expression(
                value[body_start..end].trim().to_owned(),
            ));
            literal_start = end + 1;
        }
    }
    if literal_start < value.len() {
        out.push(RawAttributePart::Literal(value[literal_start..].to_owned()));
    }
    if out.is_empty() {
        out.push(RawAttributePart::Literal(value.to_owned()));
    }
    out
}

pub(crate) fn item_to_string(item: &Item) -> String {
    if let Some(atom) = item.atom() {
        return match atom {
            AtomValue::String(value) => value,
            AtomValue::Integer(value) => value.to_string(),
            AtomValue::Decimal(value) => value,
            AtomValue::Double(value) => value.to_string(),
            AtomValue::Boolean(value) => value.to_string(),
            AtomValue::AnyUri(value) => value,
            AtomValue::Null => String::new(),
        };
    }
    match item {
        Item::Node(value) => value.clone(),
        Item::Record(_)
        | Item::Array(_)
        | Item::Native(_)
        | Item::Lambda(_)
        | Item::Resource(_) => String::new(),
        Item::Atomic(_) => unreachable!("atomic items return above"),
    }
}

fn normalize_host_expression(source: &str) -> &str {
    let trimmed = source.trim();
    if let Some(rest) = trimmed.strip_prefix('$') {
        let is_simple_binding = !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
        if is_simple_binding {
            return rest;
        }
    }
    trimmed
}

fn whole_avt_expression(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        Some(trimmed[1..trimmed.len() - 1].trim())
    } else {
        None
    }
}

/// Top-level `<attribute>` / `<slice>` declarations configure the produced element
/// (declared attributes, slice state) rather than producing visible output, so they are
/// dropped from the render plan — matching the cem-elements projection boundary.
fn is_top_level_declaration(node: &TemplateNode) -> bool {
    match node {
        TemplateNode::Element {
            tag, attributes, ..
        } => match local_template_name(tag) {
            "attribute" | "slice" | "param" => true,
            "template" => {
                declaration_name(attributes).is_some()
                    || attributes.iter().any(|a| a.name == "match" || a.name == "on")
            }
            name => cem_ml::transform_template::is_template_module_declaration(name),
        },
        _ => false,
    }
}

fn is_named_template_declaration(node: &TemplateNode) -> bool {
    matches!(
        node,
        TemplateNode::Element {
            tag,
            attributes,
            ..
        } if local_template_name(tag) == "template" && (declaration_name(attributes).is_some() || attributes.iter().any(|a| a.name == "match"))
    )
}

/// Seed binding values from top-level `{attribute @name=X | default}` / `{slice @name=X | default}`
/// declarations: the declaration's text content is the default for `X` when the host data
/// omits it (host-provided values win). Applying defaults in the render engine means the
/// browser runtime no longer needs to scan declarations to know them.
fn seed_declaration_defaults(
    nodes: &[TemplateNode],
    bindings: &mut BTreeMap<String, ItemStream>,
) -> Vec<HostAttributeUpdate> {
    let mut host_attribute_updates = Vec::new();
    for node in nodes {
        let TemplateNode::Element {
            tag,
            attributes,
            children,
            ..
        } = node
        else {
            continue;
        };
        let declaration_kind = local_template_name(tag);
        if declaration_kind != "attribute" && declaration_kind != "slice" {
            continue;
        }
        let Some(name) = declaration_name(attributes) else {
            continue;
        };
        if bindings.contains_key(&name) {
            continue; // a host-provided value overrides the declared default
        }
        // Always bind a declared attribute/slice so `{$ X}` / `!X` references resolve even when
        // the host left it unset (DCE parity: a declared attribute is always referenceable). An
        // empty declaration binds Null; attribute defaults bind text, while slice booleans
        // retain their type so `datadom.slices.*` conditions do not treat `false` as truthy text.
        let default = declaration_default_text(children);
        let value = if default.is_empty() {
            Item::Atomic(AtomValue::Null)
        } else if declaration_kind == "slice" && default == "true" {
            Item::Atomic(AtomValue::Boolean(true))
        } else if declaration_kind == "slice" && default == "false" {
            Item::Atomic(AtomValue::Boolean(false))
        } else {
            Item::Atomic(AtomValue::String(default))
        };
        let reflected_default = if declaration_kind == "attribute" {
            match &value {
                Item::Atomic(AtomValue::String(default)) => Some(default.clone()),
                _ => None,
            }
        } else {
            None
        };
        bindings.insert(name.clone(), ItemStream::once(value.clone()));
        if let Some(default) = reflected_default {
            set_current_data_document_attribute(bindings, &name, &default);
            host_attribute_updates.push(HostAttributeUpdate::new(name, default));
        } else if declaration_kind == "slice" {
            set_current_data_document_slice(bindings, &name, &value);
        }
    }
    host_attribute_updates
}

/// Collect the `@name` of every `{attribute …}` / `{slice …}` declaration token, so their
/// `name` bindings can be declared at compile time (otherwise embedded `{$ name}` would fail
/// to compile with `unknown_variable`).
fn scan_declaration_names(tokens: &[SchemaToken]) -> Vec<String> {
    let mut names = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let SchemaTokenKind::NodeStart { name } = &tokens[index].kind else {
            index += 1;
            continue;
        };
        if !matches!(local_template_name(name), "attribute" | "slice" | "param") {
            index += 1;
            continue;
        }
        let mut cursor = index + 1;
        while cursor < tokens.len() {
            match &tokens[cursor].kind {
                SchemaTokenKind::Attribute { name, value, .. } => {
                    if name == "name" {
                        if let Some(value) = value {
                            names.push(value.clone());
                        }
                    }
                    cursor += 1;
                }
                SchemaTokenKind::Trivia(_) => cursor += 1,
                _ => break,
            }
        }
        index = cursor;
    }
    names
}

fn validate_module_bodies(nodes: &mut Vec<TemplateNode>, diagnostics: &mut Vec<Diagnostic>) {
    let module_count = nodes.iter().filter(|node| {
        matches!(node, TemplateNode::Element { tag, .. } if local_template_name(tag) == "module")
    }).count();
    if module_count > 1 {
        diagnostics.push(render_diagnostic(
            cem_ml::transform_template::TRANSFORM_TEMPLATE_DECLARATION_DUPLICATE_CODE,
            "CEM-native template schema allows only one top-level `module` node".into(),
            0, SourceMapStack::default(),
        ));
        nodes.clear();
        return;
    }
    if module_count == 1 {
        for node in nodes {
            if let TemplateNode::Element { tag, children, .. } = node {
                if local_template_name(tag) == "module" {
                    validate_module_body(children, diagnostics);
                }
            }
        }
    } else {
        validate_module_body(nodes, diagnostics);
    }
}

fn validate_module_body(nodes: &mut Vec<TemplateNode>, diagnostics: &mut Vec<Diagnostic>) {
    let body_count = nodes.iter().filter(|node| {
        matches!(node, TemplateNode::Element { tag, .. } if local_template_name(tag) == "body")
    }).count();
    let has_direct_content = nodes.iter().any(|node| {
        if is_top_level_declaration(node) { return false; }
        match node {
            TemplateNode::Element { tag, .. } if local_template_name(tag) == "body" => false,
            TemplateNode::Text { text, .. } => !text.trim().is_empty(),
            _ => true,
        }
    });
    if let Some(message) = cem_ml::transform_template::template_body_layout_error(body_count, has_direct_content) {
        let source_map = nodes.first().map(template_node_source_map).cloned().unwrap_or_default();
        diagnostics.push(render_diagnostic(
            cem_ml::transform_template::TRANSFORM_TEMPLATE_DECLARATION_INVALID_CODE,
            message.into(), source_map_start(&source_map), source_map,
        ));
        nodes.clear();
    }
}

fn root_render_nodes(nodes: &[TemplateNode]) -> Vec<&TemplateNode> {
    let mut roots = Vec::new();
    for node in nodes {
        if let TemplateNode::Element { tag, children, .. } = node {
            if local_template_name(tag) == "module" {
                roots.extend(module_body_nodes(children));
                continue;
            }
            if local_template_name(tag) == "body" {
                roots.extend(children);
                continue;
            }
        }
        if !is_top_level_declaration(node) || hooks::is_expression_hook(node) {
            roots.push(node);
        }
    }
    roots
}

fn template_node_source_map(node: &TemplateNode) -> &SourceMapStack {
    match node {
        TemplateNode::Element { source_map, .. }
        | TemplateNode::Result { source_map, .. }
        | TemplateNode::Text { source_map, .. }
        | TemplateNode::Comment { source_map, .. }
        | TemplateNode::If { source_map, .. }
        | TemplateNode::Choose { source_map, .. }
        | TemplateNode::ForEach { source_map, .. }
        | TemplateNode::ProjectPayload { source_map, .. }
        | TemplateNode::Variable { source_map, .. } => source_map,
        TemplateNode::Expression(expression) => &expression.source_map,
    }
}

fn module_body_nodes(nodes: &[TemplateNode]) -> Vec<&TemplateNode> {
    for node in nodes {
        let TemplateNode::Element { tag, children, .. } = node else {
            continue;
        };
        if local_template_name(tag) == "body" {
            return children.iter().collect();
        }
    }
    nodes.iter().filter(|node| !is_top_level_declaration(node) || hooks::is_expression_hook(node)).collect()
}

#[derive(Debug, Clone)]
struct TemplateBody {
    defaults: Vec<TemplateNode>,
    nodes: Vec<TemplateNode>,
}

#[derive(Debug, Clone)]
struct MatchRule {
    test: CompiledTemplateExpression,
    mode: String,
    priority: i64,
    local: bool,
    order: usize,
    body: TemplateBody,
}

fn collect_match_rules(nodes: &[TemplateNode]) -> Vec<MatchRule> {
    fn visit(nodes: &[TemplateNode], defaults: &[TemplateNode], rules: &mut Vec<MatchRule>) {
        for node in nodes {
            let TemplateNode::Element {
                tag,
                attributes,
                children,
                ..
            } = node
            else {
                continue;
            };
            if local_template_name(tag) == "template" && !hooks::is_expression_hook(node) {
                if let Some(TemplateAttributeValue::Expression(test)) = attributes
                    .iter()
                    .find(|a| a.name == "match")
                    .and_then(|a| a.value.as_ref())
                {
                    rules.push(MatchRule {
                        test: test.clone(),
                        mode: literal_template_attribute(attributes, "mode").unwrap_or_default(),
                        priority: literal_template_attribute(attributes, "priority")
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0),
                        local: !declaration_name(attributes)
                            .is_some_and(|name| name.starts_with("__cem_module:")),
                        order: rules.len(),
                        body: TemplateBody { defaults: defaults.to_vec(), nodes: template_body_nodes(children) },
                    });
                }
            }
            let module_defaults = hooks::imported_module_defaults(node);
            visit(children, module_defaults.as_deref().unwrap_or(defaults), rules);
        }
    }
    let mut rules = Vec::new();
    visit(nodes, &[], &mut rules);
    rules.sort_by_key(|rule| std::cmp::Reverse((rule.priority, rule.local, rule.order)));
    rules
}

fn collect_named_templates(nodes: &[TemplateNode]) -> BTreeMap<String, TemplateBody> {
    fn visit(nodes: &[TemplateNode], defaults: &[TemplateNode], templates: &mut BTreeMap<String, TemplateBody>) {
        for node in nodes {
            let TemplateNode::Element { tag, attributes, children, .. } = node else { continue };
            if local_template_name(tag) == "template" && !hooks::is_expression_hook(node) {
                if let Some(name) = declaration_name(attributes) {
                    templates.insert(name, TemplateBody { defaults: defaults.to_vec(), nodes: template_body_nodes(children) });
                }
            }
            let module_defaults = hooks::imported_module_defaults(node);
            visit(children, module_defaults.as_deref().unwrap_or(defaults), templates);
        }
    }
    let mut templates = BTreeMap::new();
    visit(nodes, &[], &mut templates);
    templates
}

fn module_template_name(uri: &str, name: &str) -> String {
    format!("__cem_module:{uri}#{name}")
}

fn collect_template_declarations(nodes: &[TemplateNode]) -> Vec<TemplateNode> {
    let mut declarations = Vec::new();
    for node in nodes {
        if is_named_template_declaration(node) && !hooks::is_expression_hook(node) {
            declarations.push(node.clone());
        }
        if let TemplateNode::Element { children, .. } = node {
            declarations.extend(collect_template_declarations(children));
        }
    }
    declarations
}

fn rewrite_module_calls(
    nodes: &mut [TemplateNode],
    current_module_uri: Option<&str>,
    imports: &BTreeMap<(Option<String>, String), String>,
    public_templates: &BTreeMap<String, BTreeSet<String>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for node in nodes {
        match node {
            TemplateNode::Element {
                tag,
                attributes,
                children,
                source_map,
            } => {
                if local_template_name(tag) == "call" {
                    rewrite_module_call(
                        attributes,
                        current_module_uri,
                        imports,
                        public_templates,
                        diagnostics,
                        source_map,
                    );
                }
                rewrite_module_calls(
                    children,
                    current_module_uri,
                    imports,
                    public_templates,
                    diagnostics,
                );
            }
            TemplateNode::Result { children, .. } | TemplateNode::If { children, .. } | TemplateNode::ForEach { children, .. } => {
                rewrite_module_calls(
                    children,
                    current_module_uri,
                    imports,
                    public_templates,
                    diagnostics,
                );
            }
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    rewrite_module_calls(
                        &mut branch.children,
                        current_module_uri,
                        imports,
                        public_templates,
                        diagnostics,
                    );
                }
            }
            TemplateNode::Text { .. }
            | TemplateNode::Comment { .. }
            | TemplateNode::ProjectPayload { .. }
            | TemplateNode::Variable { .. }
            | TemplateNode::Expression(_) => {}
        }
    }
}

fn rewrite_module_call(
    attributes: &mut Vec<TemplateAttribute>,
    current_module_uri: Option<&str>,
    imports: &BTreeMap<(Option<String>, String), String>,
    public_templates: &BTreeMap<String, BTreeSet<String>>,
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMapStack,
) {
    let template = literal_template_attribute(attributes, "template");
    let from = literal_template_attribute(attributes, "from");
    if attributes.iter().any(|attribute| attribute.name == "from") && from.is_none() {
        diagnostics.push(render_diagnostic(
            "cem.ql.template.module_call_dynamic",
            "a cross-module template call requires a static `from` alias".to_owned(),
            source_map_start(source_map),
            source_map.clone(),
        ));
        return;
    }
    let Some(template) = template else {
        return;
    };
    let is_cross_module = from.is_some();
    let target_uri = match from {
        Some(alias) => {
            let key = (current_module_uri.map(str::to_owned), alias.clone());
            let Some(uri) = imports.get(&key) else {
                diagnostics.push(render_diagnostic(
                    "cem.ql.template.module_import_unresolved",
                    format!(
                        "template module alias `{alias}` from `{}` is absent from the preflighted closure",
                        current_module_uri.unwrap_or("<root>")
                    ),
                    source_map_start(source_map),
                    source_map.clone(),
                ));
                return;
            };
            uri.clone()
        }
        None => {
            let Some(uri) = current_module_uri else {
                return;
            };
            uri.to_owned()
        }
    };
    if is_cross_module
        && !public_templates
            .get(&target_uri)
            .is_some_and(|templates| templates.contains(&template))
    {
        diagnostics.push(render_diagnostic(
            "cem.ql.template.module_template_not_public",
            format!("template `{template}` is not a public entrypoint of module `{target_uri}`"),
            source_map_start(source_map),
            source_map.clone(),
        ));
        return;
    }
    if let Some(attribute) = attributes
        .iter_mut()
        .find(|attribute| attribute.name == "template")
    {
        attribute.value = Some(TemplateAttributeValue::Literal(module_template_name(
            &target_uri,
            &template,
        )));
    }
    attributes.retain(|attribute| attribute.name != "from");
}

fn literal_template_attribute(attributes: &[TemplateAttribute], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name == name)
        .and_then(|attribute| match &attribute.value {
            Some(TemplateAttributeValue::Literal(value)) => Some(value.clone()),
            _ => None,
        })
}

fn template_body_nodes(children: &[TemplateNode]) -> Vec<TemplateNode> {
    for child in children {
        let TemplateNode::Element {
            tag,
            children: body,
            ..
        } = child
        else {
            continue;
        };
        if local_template_name(tag) == "body" {
            return body.clone();
        }
    }
    children
        .iter()
        .filter(|child| !is_top_level_declaration(child) || hooks::is_expression_hook(child))
        .cloned()
        .collect()
}

fn declaration_name(attributes: &[TemplateAttribute]) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name == "name")
        .and_then(|attribute| match &attribute.value {
            Some(TemplateAttributeValue::Literal(value)) => Some(value.clone()),
            _ => None,
        })
}

fn declaration_select(attributes: &[TemplateAttribute]) -> Option<&CompiledTemplateExpression> {
    attributes
        .iter()
        .find(|attribute| attribute.name == "select")
        .and_then(|attribute| match &attribute.value {
            Some(TemplateAttributeValue::Expression(expression)) => Some(expression),
            _ => None,
        })
}

fn declaration_default_text(children: &[TemplateNode]) -> String {
    let mut text = String::new();
    for child in children {
        if let TemplateNode::Text { text: chunk, .. } = child {
            text.push_str(chunk);
        }
    }
    text.trim().to_owned()
}

fn node_start_name(token: &SchemaToken) -> String {
    match &token.kind {
        SchemaTokenKind::NodeStart { name } => name.clone(),
        _ => String::new(),
    }
}

/// Local name of a (possibly `cem:`-prefixed) conditional element, so both the canonical
/// `cem:if`/`cem:choose`/... and the legacy bare `if`/`choose`/... spellings are accepted.
fn conditional_local_name(name: &str) -> &str {
    name.strip_prefix("cem:").unwrap_or(name)
}

fn local_template_name(name: &str) -> &str {
    conditional_local_name(name)
}

fn is_cemt_runtime_function_declaration_name(name: &str) -> bool {
    matches!(
        name,
        "function" | "encoding-function" | "format-function" | "color-function"
    )
}

fn is_if_name(name: &str) -> bool {
    conditional_local_name(name) == "if"
}

fn is_choose_name(name: &str) -> bool {
    conditional_local_name(name) == "choose"
}

fn is_when_name(name: &str) -> bool {
    conditional_local_name(name) == "when"
}

fn is_otherwise_name(name: &str) -> bool {
    conditional_local_name(name) == "otherwise"
}

fn is_for_each_name(name: &str) -> bool {
    conditional_local_name(name) == "for-each"
}

fn is_project_payload_name(name: &str) -> bool {
    conditional_local_name(name) == "project-payload"
}

fn is_variable_name(name: &str) -> bool {
    conditional_local_name(name) == "variable"
}

fn valid_variable_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
}

fn payload_item_to_render_node(item: &Item, source_map: &SourceMapStack) -> Option<RenderPlanNode> {
    let Item::Record(record) = item else {
        return None;
    };
    match record_string(record, "kind")?.as_str() {
        "text" => Some(RenderPlanNode::Text {
            text: record_string(record, "text").unwrap_or_default(),
            source_map: source_map.clone(),
        }),
        "comment" => Some(RenderPlanNode::Comment {
            text: record_string(record, "text").unwrap_or_default(),
            source_map: source_map.clone(),
        }),
        "element" => {
            let tag = record_string(record, "tag")?;
            let namespace = record_string(record, "namespace").filter(|value| !value.is_empty());
            let attributes = record_record(record, "attributes")
                .map(|attributes| {
                    attributes
                        .iter()
                        .map(|(name, values)| RenderPlanAttribute {
                contract: None,
                            qualified_name: None,
                            name: name.clone(),
                            namespace: None,
                            value: values.iter().map(item_to_string).collect::<String>(),
                            value_stream: ItemStream::from_items(values.clone()),
                            source_map: source_map.clone(),
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let children = record_items(record, "children")
                .into_iter()
                .flat_map(|item| item.members().unwrap_or_else(|| vec![item]))
                .filter_map(|item| payload_item_to_render_node(&item, source_map))
                .collect::<Vec<_>>();
            Some(RenderPlanNode::Element {
                qualified_name: None,
                tag,
                namespace,
                attributes,
                children,
                source_map: source_map.clone(),
            })
        }
        _ => None,
    }
}

fn record_items(record: &BTreeMap<String, Vec<Item>>, name: &str) -> Vec<Item> {
    record.get(name).cloned().unwrap_or_default()
}

fn record_record<'a>(
    record: &'a BTreeMap<String, Vec<Item>>,
    name: &str,
) -> Option<&'a BTreeMap<String, Vec<Item>>> {
    record.get(name)?.first().and_then(|item| match item {
        Item::Record(value) => Some(value),
        _ => None,
    })
}

fn record_string(record: &BTreeMap<String, Vec<Item>>, name: &str) -> Option<String> {
    record.get(name)?.first().map(item_to_string)
}

/// Build a per-node source-map stack from a token's real absolute `byte_range`.
///
/// The CEM tokenizer stamps every token's `source_map` with the whole-document
/// base frame, so cloning it loses per-node offsets. The accurate location lives
/// on `token.byte_range`; this rebuilds a single-frame stack from it so render
/// plans (and the WASM `byteOffset`) carry author-byte-exact per-node frames.
fn frame_for(token: &SchemaToken) -> SourceMapStack {
    let source_id = token
        .source_map
        .origin()
        .map(|frame| frame.source_id)
        .unwrap_or(SourceId(1));
    SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id,
            span: FrameSpan::Single(token.byte_range),
            transform: TransformKind::CemTokenizer,
        }],
    }
}

fn render_diagnostic(
    code: &str,
    message: String,
    byte_offset: u64,
    source_map: SourceMapStack,
) -> Diagnostic {
    Diagnostic {
        uri: None,
        line: None,
        column: None,
        byte_offset: Some(byte_offset),
        code: code.to_owned(),
        severity: Severity::Error,
        message,
        node: None,
        details: None,
        source_map: Some(source_map),
    }
}

fn source_map_start(source_map: &SourceMapStack) -> u64 {
    source_map
        .frames
        .last()
        .and_then(|frame| match frame.span {
            FrameSpan::Single(range) => Some(range.start),
            FrameSpan::Multi(_) => None,
        })
        .unwrap_or(0)
}

fn normalize_constructed_name(value: &str) -> Option<String> {
    let name = value.trim();
    if name
        .chars()
        .any(|c| c.is_whitespace() || matches!(c, '<' | '>' | '/' | '=' | '"' | '\''))
    {
        return None;
    }
    Some(name.to_owned())
}

fn render_plan_nodes_to_text(nodes: &[RenderPlanNode]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            RenderPlanNode::Reference { reference, .. } => {
                text.push_str(&render_plan_nodes_to_text(&expand_reference(reference)));
            }
            RenderPlanNode::Element { children, .. } => {
                text.push_str(&render_plan_nodes_to_text(children));
            }
            RenderPlanNode::Text { text: chunk, .. } => text.push_str(chunk),
            RenderPlanNode::Cdata { text: chunk, .. } => text.push_str(chunk),
            RenderPlanNode::Comment { .. } => {}
            RenderPlanNode::ProcessingInstruction { .. } => {}
        }
    }
    text
}

fn sort_render_plan_attributes(attributes: &mut [RenderPlanAttribute]) {
    attributes.sort_by(|left, right| {
        left.namespace
            .cmp(&right.namespace)
            .then_with(|| left.name.cmp(&right.name))
    });
}

fn escape_controlled(
    out: &mut String,
    value: &str,
    attribute: bool,
    safe_points: &mut Option<SafePointPoller>,
    control_error: &mut Option<cem_ml::operation_control::ControlError>,
) {
    for character in value.chars() {
        if control_error.is_some() {
            break;
        }
        if let Some(error) = safe_points
            .as_mut()
            .and_then(|safe_points| safe_points.poll_one().err())
        {
            *control_error = Some(error);
            break;
        }
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            _ => out.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(start: u64, len: u32) -> SourceMapStack {
        SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange::new(start, len)),
                transform: TransformKind::CemTokenizer,
            }],
        }
    }

    fn sample_plan() -> RenderPlan {
        RenderPlan {
            nodes: vec![RenderPlanNode::Element {
                qualified_name: None,
                tag: "p".to_owned(),
                namespace: None,
                attributes: vec![RenderPlanAttribute {
                contract: None,
                    qualified_name: None,
                    name: "title".to_owned(),
                    namespace: None,
                    value: "A&B".to_owned(),
                    value_stream: ItemStream::once(Item::Atomic(AtomValue::String("A&B".into()))),
                    source_map: stack(3, 12),
                }],
                children: vec![RenderPlanNode::Text {
                    text: "Hi <all>".to_owned(),
                    source_map: stack(16, 8),
                }],
                source_map: stack(0, 28),
            }],
            host_attribute_updates: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn record(fields: impl IntoIterator<Item = (&'static str, Vec<Item>)>) -> Item {
        Item::Record(
            fields
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn source_map_render_preserves_html_output() {
        let plan = sample_plan();

        assert_eq!(
            render_plan_to_html(&plan),
            r#"<p title="A&amp;B">Hi &lt;all&gt;</p>"#
        );
        assert_eq!(
            render_plan_to_html_with_source_map(&plan).rendered,
            render_plan_to_html(&plan)
        );
    }

    #[test]
    fn project_payload_materializes_serialized_rich_nodes() {
        let text = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("text".to_owned()))],
            ),
            (
                "key",
                vec![Item::Atomic(AtomValue::String("0/0/0".to_owned()))],
            ),
            (
                "text",
                vec![Item::Atomic(AtomValue::String("Ada".to_owned()))],
            ),
        ]);
        let strong = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "key",
                vec![Item::Atomic(AtomValue::String("0/0".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("strong".to_owned()))],
            ),
            ("namespace", vec![Item::Atomic(AtomValue::Null)]),
            ("attributes", vec![Item::Record(BTreeMap::new())]),
            ("children", vec![Item::Array(vec![text])]),
        ]);
        let comment = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("comment".to_owned()))],
            ),
            (
                "key",
                vec![Item::Atomic(AtomValue::String("0/1".to_owned()))],
            ),
            (
                "text",
                vec![Item::Atomic(AtomValue::String("note".to_owned()))],
            ),
        ]);
        let span = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            ("key", vec![Item::Atomic(AtomValue::String("0".to_owned()))]),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("span".to_owned()))],
            ),
            ("namespace", vec![Item::Atomic(AtomValue::Null)]),
            (
                "attributes",
                vec![Item::Record(BTreeMap::from([(
                    "class".to_owned(),
                    vec![Item::Atomic(AtomValue::String("rich".to_owned()))],
                )]))],
            ),
            ("children", vec![Item::Array(vec![strong, comment])]),
        ]);
        let payload = record([("nodes", vec![Item::Array(vec![span])])]);
        let datadom = record([("payload", vec![payload])]);
        let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

        let rendered = render_template(
            r#"{div | {cem:project-payload @select="datadom.payload.nodes" | }}"#,
            &data,
        );

        assert_eq!(
            rendered.rendered,
            r#"<div><span class="rich"><strong>Ada</strong><!--note--></span></div>"#
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn project_payload_requires_select() {
        let rendered = render_template("{cem:project-payload | }", &TemplateData::default());
        assert!(rendered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.project_payload_missing_select"));
    }

    #[test]
    fn source_map_render_serializes_xml_specific_nodes() {
        let plan = RenderPlan {
            nodes: vec![
                RenderPlanNode::ProcessingInstruction {
                    target: "xml-stylesheet".to_owned(),
                    data: "href=\"main.css\"".to_owned(),
                    source_map: stack(0, 8),
                },
                RenderPlanNode::Element {
                    qualified_name: None,
                    tag: "root".to_owned(),
                    namespace: None,
                    attributes: vec![RenderPlanAttribute {
                contract: None,
                        qualified_name: None,
                        name: "id".to_owned(),
                        namespace: None,
                        value: "a&b".to_owned(),
                        value_stream: ItemStream::once(Item::Atomic(AtomValue::String("a&b".into()))),
                        source_map: stack(8, 8),
                    }],
                    children: vec![
                        RenderPlanNode::Element {
                            qualified_name: None,
                            tag: "empty".to_owned(),
                            namespace: None,
                            attributes: Vec::new(),
                            children: Vec::new(),
                            source_map: stack(16, 8),
                        },
                        RenderPlanNode::Cdata {
                            text: "x < y".to_owned(),
                            source_map: stack(24, 8),
                        },
                    ],
                    source_map: stack(8, 24),
                },
            ],
            host_attribute_updates: Vec::new(),
            diagnostics: Vec::new(),
        };

        assert_eq!(
            render_plan_to_xml_with_source_map(&plan).rendered,
            r#"<?xml-stylesheet href="main.css"?><root id="a&amp;b"><empty/><![CDATA[x < y]]></root>"#
        );
    }

    #[test]
    fn controlled_output_chunks_preserve_success_and_discard_cancelled_output() {
        let plan = RenderPlan {
            nodes: vec![RenderPlanNode::Text {
                text: "<hello & goodbye>".repeat(16),
                source_map: stack(0, 8),
            }],
            host_attribute_updates: Vec::new(),
            diagnostics: Vec::new(),
        };
        let control = OperationControl::default();
        let controlled = render_plan_to_html_with_control(
            &plan,
            &control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        )
        .unwrap();
        assert_eq!(controlled.rendered, render_plan_to_html(&plan));

        control.cancel_root(None, None).unwrap();
        let short_plan = RenderPlan {
            nodes: vec![RenderPlanNode::Text {
                text: "x".to_owned(),
                source_map: stack(0, 1),
            }],
            host_attribute_updates: Vec::new(),
            diagnostics: Vec::new(),
        };
        assert!(render_plan_to_html_with_control(
            &short_plan,
            &control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        )
        .is_err());
    }

    #[test]
    fn controlled_template_render_discards_a_pre_cancelled_plan() {
        let artifact = compile_template("{p | hello}", &CompileTemplateOptions::default());
        let control = OperationControl::default();
        control.cancel_root(None, None).unwrap();

        let plan = render_compiled_template_with_control(
            &artifact,
            &TemplateData::default(),
            &control,
            cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID,
        );
        assert!(plan.nodes.is_empty());
        assert!(plan
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.ql.render.control_failure"));
    }

    #[test]
    fn compile_extracts_static_stylesheets_from_render_nodes() {
        let artifact = compile_template(
            r#"{module |
                {body |
                    {style |```
                        :host { display: block; }
                    ```}
                    {style @scope="abc-lib" |```
                        .shared { color: green; }
                    ```}
                    {button | Save}
                }
            }"#,
            &CompileTemplateOptions::default(),
        );

        assert_eq!(artifact.stylesheets.len(), 2);
        assert_eq!(artifact.stylesheets[0].scope, None);
        assert!(artifact.stylesheets[0]
            .css
            .contains(":host { display: block; }"));
        assert_eq!(artifact.stylesheets[1].scope.as_deref(), Some("abc-lib"));
        assert!(artifact.stylesheets[1]
            .css
            .contains(".shared { color: green; }"));

        let plan = render_compiled_template(&artifact, &TemplateData::default());
        assert!(!render_plan_to_html(&plan).contains("<style"));
        assert!(render_plan_to_html(&plan).contains("<button>Save</button>"));
    }

    #[test]
    fn compile_rejects_dynamic_declaration_stylesheets() {
        let artifact = compile_template(
            r#"{module |
                {slice @name=scope | abc-lib}
                {body |
                    {style @scope="{$scope}" |```
                        :host { color: red; }
                    ```}
                    {cem:if @test=scope |
                        {style |```
                            :host { display: block; }
                        ```}
                    }
                }
            }"#,
            &CompileTemplateOptions::default(),
        );

        assert!(artifact.stylesheets.is_empty());
        assert_eq!(
            artifact
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code
                    == "cem.ql.template.stylesheet_dynamic_unsupported")
                .count(),
            2,
        );
        let plan = render_compiled_template(&artifact, &TemplateData::default());
        assert!(!render_plan_to_html(&plan).contains("<style"));
    }

    #[test]
    fn compile_extracts_static_module_map_prelude_from_render_nodes() {
        let artifact = compile_template(
            r#"{module-map |
                {import @specifier="demo-image" @target="./images/smiley.svg"}
                {resource @specifier="demo-data" @target="./data.json" @content-type="application/json" @integrity="sha256-demo"}
                {scope @prefix="./feature/" |
                    {import @specifier="demo-image" @target="./images/feature.svg"}
                }
            }
            {cem-module-url @slice=imageUrl @src="demo-image"}"#,
            &CompileTemplateOptions::default(),
        );

        let module_map = artifact.module_map.as_ref().expect("module map prelude");
        assert_eq!(
            module_map.specifiers.imports["demo-image"]
                .target
                .as_deref(),
            Some("./images/smiley.svg")
        );
        assert_eq!(
            module_map.specifiers.resources["demo-data"]
                .content_type_hint
                .as_deref(),
            Some("application/json")
        );
        assert_eq!(module_map.scopes[0].prefix, "./feature/");
        assert_eq!(
            module_map.scopes[0].specifiers.imports["demo-image"]
                .target
                .as_deref(),
            Some("./images/feature.svg")
        );

        let plan = render_compiled_template(&artifact, &TemplateData::default());
        let html = render_plan_to_html(&plan);
        assert!(!html.contains("module-map"));
        assert!(html.contains("cem-module-url"));
    }

    #[test]
    fn compile_rejects_duplicate_module_map_keys_across_collections() {
        let artifact = compile_template(
            r#"{module-map |
                {import @specifier="demo-image" @target="./image.js"}
                {resource @specifier="demo-image" @target="./image.svg"}
            }"#,
            &CompileTemplateOptions::default(),
        );

        assert!(artifact.module_map.is_none());
        assert!(artifact
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "cem.ql.template.module_map_duplicate" }));
    }

    #[test]
    fn source_map_render_records_output_boundary_and_spans() {
        let output = render_plan_to_html_with_source_map(&sample_plan());

        assert_eq!(output.source_map.frames.len(), 1);
        assert!(matches!(
            output.source_map.frames[0].transform,
            TransformKind::InterpreterRender
        ));
        assert!(matches!(
            output.source_map.frames[0].span,
            FrameSpan::Single(ByteRange { start: 0, len })
                if len as usize == output.rendered.len()
        ));
        assert!(output.output_spans.iter().all(|span| {
            span.origin
                .frames
                .last()
                .is_some_and(|frame| matches!(frame.transform, TransformKind::InterpreterRender))
        }));

        let text_start = output.rendered.find("Hi").expect("text should render") as u64;
        assert!(output
            .output_spans
            .iter()
            .any(|span| span.output_range.start == text_start));
    }

    #[test]
    fn same_module_template_call_renders_body_with_params() {
        let source = r#"{module |
            {template @name="label" |
                {param @name="node"}
                {body | {span @data-kind="{node.kind}" | {$ node.text}}}
            }
            {body |
                {call @template="label" @with:node="{datadom.payload.nodes}"}
            }
        }"#;
        let mut node = BTreeMap::new();
        node.insert(
            "kind".to_owned(),
            vec![Item::Atomic(AtomValue::String("text".to_owned()))],
        );
        node.insert(
            "text".to_owned(),
            vec![Item::Atomic(AtomValue::String("Leaf".to_owned()))],
        );
        let mut payload = BTreeMap::new();
        payload.insert("nodes".to_owned(), vec![Item::Record(node)]);
        let mut datadom = BTreeMap::new();
        datadom.insert("payload".to_owned(), vec![Item::Record(payload)]);
        let data = TemplateData::default()
            .with_binding("datadom", ItemStream::once(Item::Record(datadom)));

        let rendered = render_template(source, &data);

        assert_eq!(
            rendered.rendered.trim(),
            r#"<span data-kind="text">Leaf</span>"#
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn unnamed_template_element_still_renders_as_html() {
        let rendered = render_template("{template | {span | fallback}}", &TemplateData::default());

        assert_eq!(
            rendered.rendered.trim(),
            "<template><span>fallback</span></template>"
        );
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn native_recursive_template_renders_data_island_tree() {
        let source = r#"{module |
            {template @name="node" |
                {param @name="node"}
                {body |
                    {cem:choose |
                        {cem:when @test='node.kind == "element"' |
                            {details @open=open |
                                {summary |
                                    {b | {$ node.tag}}
                                    {cem:if @test="node.attributes.data-root" | {code | data-root="{$ node.attributes.data-root}"}}
                                    {cem:if @test="node.attributes.data-level" | {code | data-level="{$ node.attributes.data-level}"}}
                                    {cem:if @test="node.attributes.name" | {code | name="{$ node.attributes.name}"}}
                                    {cem:if @test="node.attributes.code" | {code | code="{$ node.attributes.code}"}}
                                }
                                {cem:for-each @select="node.children" @as="child" |
                                    {call @template="node" @with:node="{child}"}
                                }
                            }
                        }
                        {cem:when @test='node.kind == "text"' |
                            {p | {$ node.text}}
                        }
                    }
                }
            }
            {body |
                {article |
                    {h2 | embedded-xsl data island tree}
                    {details @open=open |
                        {summary |
                            {b | datadom}
                            {code | title="{$ datadom.attributes.title}"}
                            {code | data-demo="{$ datadom.attributes.data-demo}"}
                        }
                        {cem:for-each @select="datadom.payload.nodes" @as="node" |
                            {call @template="node" @with:node="{node}"}
                        }
                    }
                }
            }
        }"#;
        let text = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("text".to_owned()))],
            ),
            (
                "text",
                vec![Item::Atomic(AtomValue::String(
                    "Leaf text from cem-elements data island".to_owned(),
                ))],
            ),
        ]);
        let leaf = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("leaf".to_owned()))],
            ),
            (
                "attributes",
                vec![record([(
                    "data-level",
                    vec![Item::Atomic(AtomValue::String("3".to_owned()))],
                )])],
            ),
            ("children", vec![Item::Array(vec![text])]),
        ]);
        let item = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("item".to_owned()))],
            ),
            (
                "attributes",
                vec![record([(
                    "code",
                    vec![Item::Atomic(AtomValue::String("a1".to_owned()))],
                )])],
            ),
            ("children", vec![Item::Array(vec![leaf])]),
        ]);
        let section = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("section".to_owned()))],
            ),
            (
                "attributes",
                vec![record([
                    (
                        "data-level",
                        vec![Item::Atomic(AtomValue::String("1".to_owned()))],
                    ),
                    (
                        "name",
                        vec![Item::Atomic(AtomValue::String("alpha".to_owned()))],
                    ),
                ])],
            ),
            ("children", vec![Item::Array(vec![item])]),
        ]);
        let catalog = record([
            (
                "kind",
                vec![Item::Atomic(AtomValue::String("element".to_owned()))],
            ),
            (
                "tag",
                vec![Item::Atomic(AtomValue::String("catalog".to_owned()))],
            ),
            (
                "attributes",
                vec![record([(
                    "data-root",
                    vec![Item::Atomic(AtomValue::String("cem-elements".to_owned()))],
                )])],
            ),
            ("children", vec![Item::Array(vec![section])]),
        ]);
        let datadom = record([
            (
                "attributes",
                vec![record([
                    (
                        "title",
                        vec![Item::Atomic(AtomValue::String(
                            "Anonymous DCE data island".to_owned(),
                        ))],
                    ),
                    (
                        "data-demo",
                        vec![Item::Atomic(AtomValue::String("cem-elements".to_owned()))],
                    ),
                ])],
            ),
            (
                "payload",
                vec![record([("nodes", vec![Item::Array(vec![catalog])])])],
            ),
        ]);
        let data = TemplateData::default().with_binding("datadom", ItemStream::once(datadom));

        let rendered = render_template(source, &data);

        assert!(rendered.rendered.contains("embedded-xsl data island tree"));
        assert!(rendered
            .rendered
            .contains("title=\"Anonymous DCE data island\""));
        assert!(rendered.rendered.contains("data-root=\"cem-elements\""));
        assert!(rendered.rendered.contains("data-level=\"3\""));
        assert!(rendered
            .rendered
            .contains("Leaf text from cem-elements data island"));
        assert!(
            rendered.diagnostics.is_empty(),
            "{:?}",
            rendered.diagnostics
        );
    }

    #[test]
    fn recursive_template_calls_are_bounded() {
        let source = r#"{module |
            {template @name="loop" | {body | {span | Loop {call @template="loop"}}}}
            {body | {div | {call @template="loop"}}}
        }"#;

        let rendered = render_template(source, &TemplateData::default());

        assert!(rendered.rendered.starts_with("<div><span>Loop "));
        assert!(rendered.rendered.ends_with("</span></div>"));
        assert!(rendered
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "cem.transform_template.recursion_limit" }));
    }

    #[test]
    fn preflighted_module_closure_renders_imported_and_nested_templates() {
        let root_uri = "https://example.test/generator.cemt";
        let docs_uri = "https://example.test/lib/docs.cemt";
        let cells_uri = "https://example.test/lib/cells.cemt";
        let source = r#"{module |
            {import @as="docs" @src="./lib/docs.cemt"}
            {body | {call @from="docs" @template="token-table" @with:label="Geometry"}}
        }"#;
        let docs = r#"{module |
            {import @as="cells" @src="./cells.cemt"}
            {template @name="token-table" @visibility="public" |
                {param @name="label"}
                {body | {table | {caption | {$label}}{call @from="cells" @template="value"}}}
            }
        }"#;
        let cells = r#"{module |
            {template @name="value" @visibility="public" | {body | {td | shared}}}
        }"#;
        let hash = |value: &str| {
            cem_ml::content_cache::ContentHash::from_blake3(value.as_bytes()).header_value()
        };
        let closure = TemplateModuleClosure {
            root_uri: root_uri.to_owned(),
            root_content_hash: hash(source),
            resolver_policy_stamp: "fixture-policy".to_owned(),
            modules: vec![
                TemplateModuleSource {
                    alias: "docs".to_owned(),
                    parent_uri: None,
                    uri: docs_uri.to_owned(),
                    content_hash: hash(docs),
                    source: docs.to_owned(),
                },
                TemplateModuleSource {
                    alias: "cells".to_owned(),
                    parent_uri: Some(docs_uri.to_owned()),
                    uri: cells_uri.to_owned(),
                    content_hash: hash(cells),
                    source: cells.to_owned(),
                },
            ],
            ..TemplateModuleClosure::default()
        };

        let artifact =
            compile_template_module_closure(source, &closure, &CompileTemplateOptions::default());
        let plan = render_compiled_template(&artifact, &TemplateData::default());

        assert_eq!(
            render_plan_to_html(&plan).trim(),
            "<table><caption>Geometry</caption><td>shared</td></table>"
        );
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    }

    #[test]
    fn preflighted_module_closure_renders_shared_browser_native_fixture() {
        let source = include_str!("../tests/fixtures/template-module-closure/root.cemt");
        let module_source = include_str!("../tests/fixtures/template-module-closure/docs.cemt");
        let hash = |value: &str| {
            cem_ml::content_cache::ContentHash::from_blake3(value.as_bytes()).header_value()
        };
        let artifact = compile_template_module_closure(
            source,
            &TemplateModuleClosure {
                root_uri: "https://example.test/fixtures/root.cemt".to_owned(),
                root_content_hash: hash(source),
                resolver_policy_stamp: "fixture-import-map/1".to_owned(),
                entrypoint: "body".to_owned(),
                parameter_contract: Vec::new(),
                cem_ml_version: cem_ml::VERSION.to_owned(),
                cem_ql_version: crate::VERSION.to_owned(),
                modules: vec![TemplateModuleSource {
                    alias: "docs".to_owned(),
                    parent_uri: None,
                    uri: "https://example.test/fixtures/docs.cemt".to_owned(),
                    content_hash: hash(module_source),
                    source: module_source.to_owned(),
                }],
            },
            &CompileTemplateOptions::default(),
        );
        let plan = render_compiled_template(&artifact, &TemplateData::default());

        assert_eq!(
            render_plan_to_html(&plan).trim(),
            "<table><tr><th>--cem-gap</th><td>0.5rem</td></tr></table>"
        );
        assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    }

    #[test]
    fn preflighted_module_closure_rejects_hash_drift() {
        let source = "{module | {body | ok}}";
        let artifact = compile_template_module_closure(
            source,
            &TemplateModuleClosure {
                root_uri: "https://example.test/root.cemt".to_owned(),
                root_content_hash: "cem-bin/1+blake3:stale".to_owned(),
                ..TemplateModuleClosure::default()
            },
            &CompileTemplateOptions::default(),
        );

        assert!(artifact
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "cem.ql.template.module_hash_mismatch" }));
    }

    #[test]
    fn preflighted_module_closure_rejects_private_entrypoint_calls() {
        let source = r#"{module |
            {import @as="docs" @src="./docs.cemt"}
            {body | {call @from="docs" @template="helper"}}
        }"#;
        let module_source = r#"{module |
            {template @name="helper" @visibility="private" | {body | private}}
        }"#;
        let hash = |value: &str| {
            cem_ml::content_cache::ContentHash::from_blake3(value.as_bytes()).header_value()
        };
        let artifact = compile_template_module_closure(
            source,
            &TemplateModuleClosure {
                root_uri: "https://example.test/root.cemt".to_owned(),
                root_content_hash: hash(source),
                resolver_policy_stamp: "fixture-policy".to_owned(),
                modules: vec![TemplateModuleSource {
                    alias: "docs".to_owned(),
                    parent_uri: None,
                    uri: "https://example.test/docs.cemt".to_owned(),
                    content_hash: hash(module_source),
                    source: module_source.to_owned(),
                }],
                ..TemplateModuleClosure::default()
            },
            &CompileTemplateOptions::default(),
        );

        assert!(artifact
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "cem.ql.template.module_template_not_public" }));
    }
}
