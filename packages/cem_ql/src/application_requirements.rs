//! Build-only requirements from native compiled IR. No source scanning, query
//! execution, resource reads, grants or assertion of linked-code removability.
mod query;
#[cfg(test)]
mod tests;

use crate::{
    declaration_artifact::DeclarationArtifactMetadata,
    render::{
        CompiledTemplateExpression, TemplateArtifact, TemplateAttribute, TemplateAttributePart,
        TemplateAttributeValue, TemplateNode,
    },
    stdlib::ModuleRegistry,
};
use cem_ml::{
    css_imports::CssImportClosure,
    css_resources::CssResourceKind,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlFrame, CemModuleUrlResolutionCapability,
        CemResolutionContextHandle, CemScopedModuleUrlResolver,
    },
    scheduler::AbortSignal,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
const MAX_ITEMS: usize = 100_000;
const MAX_DEPTH: usize = 64;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationRequirements {
    pub contract: &'static str,
    /// This inventory does not attest native implementation/ABI closure.
    native_link_closure_verified: bool,
    pub roots: BTreeSet<String>,
    pub component_dependencies: BTreeMap<String, BTreeSet<String>>,
    pub declarations: BTreeMap<String, DeclarationRequirements>,
}
impl Default for ApplicationRequirements {
    fn default() -> Self {
        Self {
            contract: "cem-application-requirements/1",
            native_link_closure_verified: false,
            roots: BTreeSet::new(),
            component_dependencies: BTreeMap::new(),
            declarations: BTreeMap::new(),
        }
    }
}
impl ApplicationRequirements {
    /// Bounded build-control sidecar and its exact content pin.
    pub fn pinned_json(&self) -> Result<(Vec<u8>, String), String> {
        self.validate_graph()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() > 1024 * 1024 {
            return Err("requirements metadata exceeds 1 MiB".into());
        }
        let hash = cem_ml::content_cache::ContentHash::from_blake3(&bytes).header_value();
        Ok((bytes, hash))
    }
    fn validate_graph(&self) -> Result<(), String> {
        if self.declarations.is_empty()
            || self.declarations.len() > 64
            || self.roots.is_empty()
            || self.component_dependencies.len() != self.declarations.len()
            || self
                .roots
                .iter()
                .any(|tag| !self.declarations.contains_key(tag))
            || self
                .declarations
                .keys()
                .any(|tag| !self.component_dependencies.contains_key(tag))
            || self
                .component_dependencies
                .values()
                .flatten()
                .any(|tag| !self.declarations.contains_key(tag))
        {
            return Err("requirements incomplete component/root graph".into());
        }
        Ok(())
    }
    pub fn require_static_inventory(&self) -> Result<(), String> {
        self.validate_graph()?;
        if self.declarations.values().any(|d| !d.unresolved.is_empty()) {
            return Err(
                "application requirements include unresolved dynamic/host/source contracts".into(),
            );
        }
        Ok(())
    }
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationRequirements {
    pub metadata_hash: String,
    pub source_uri: String,
    pub declaration_source_hash: String,
    pub template_hash: String,
    pub template_format: String,
    pub capability: Option<String>,
    pub host_bindings: BTreeSet<String>,
    pub functions: BTreeSet<FunctionRequirement>,
    pub pipeline_operations: BTreeSet<PipelineOperation>,
    pub data_imports: BTreeSet<DataImportRequirement>,
    pub stylesheets: Vec<StylesheetRequirement>,
    pub unresolved: BTreeSet<RequirementGap>,
    pub template_node_count: usize,
    pub expression_count: usize,
    pub ir_node_count: usize,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionRequirement {
    pub module: String,
    pub name: String,
    pub arity: usize,
    /// A pipeline supplies one implicit stream argument.
    pub dispatch: String,
    pub implementation: String,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineOperation {
    pub kind: String,
    pub name: String,
    pub explicit_arity: usize,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataImportRequirement {
    pub operation: String,
    /// Literal demand only, without evaluating options or parsing data.
    pub format: Option<String>,
    pub schema: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StylesheetRequirement {
    pub occurrence: usize,
    pub sheet: usize,
    pub source_uri: String,
    pub source_key: Option<String>,
    pub base_url: String,
    pub resources: Vec<ResourceRequirement>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirement {
    pub kind: String,
    pub specifier: String,
    pub resolved_url: Option<String>,
    pub content_type: Option<String>,
    pub integrity: Option<String>,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RequirementGap {
    pub kind: String,
    /// Stable structural coordinate, never expression text or runtime values.
    pub site: String,
}
struct Inventory {
    output: DeclarationRequirements,
    registry: ModuleRegistry,
    items: usize,
}
impl Inventory {
    fn charge(&mut self, count: usize) -> Result<(), String> {
        self.items = self
            .items
            .checked_add(count)
            .ok_or("requirements item limit")?;
        if self.items > MAX_ITEMS {
            return Err("requirements item limit".into());
        }
        Ok(())
    }
    fn gap(&mut self, kind: &str, site: &str) {
        self.output.unresolved.insert(RequirementGap {
            kind: kind.into(),
            site: site.into(),
        });
    }
    fn attributes(&mut self, attributes: &[TemplateAttribute], site: &str) -> Result<(), String> {
        for (index, attribute) in attributes.iter().enumerate() {
            self.charge(1)?;
            if matches!(
                attribute.name.as_str(),
                "src"
                    | "href"
                    | "xlink:href"
                    | "srcset"
                    | "poster"
                    | "action"
                    | "formaction"
                    | "style"
                    | "data"
            ) {
                self.gap(
                    "html-resource-or-style-contract",
                    &format!("{site}/attribute:{index}"),
                );
            }
            match &attribute.value {
                Some(TemplateAttributeValue::Expression(expression)) => {
                    self.expression(expression)?
                }
                Some(TemplateAttributeValue::Template(parts)) => {
                    self.charge(parts.len())?;
                    for part in parts {
                        if let TemplateAttributePart::Expression(expression) = part {
                            self.expression(expression)?;
                        }
                    }
                }
                Some(TemplateAttributeValue::Literal(_)) | None => (),
            }
        }
        Ok(())
    }
    fn nodes(&mut self, nodes: &[TemplateNode], depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("requirements template depth limit".into());
        }
        self.charge(nodes.len())?;
        for node in nodes {
            let site = format!("template-node:{}", self.output.template_node_count);
            self.output.template_node_count += 1;
            match node {
                TemplateNode::Element {
                    tag,
                    attributes,
                    children,
                    ..
                } => {
                    if matches!(tag.as_str(), "slot" | "cem-element" | "template") {
                        self.gap("dynamic-component-or-payload", &site);
                    }
                    if matches!(
                        tag.as_str(),
                        "script"
                            | "link"
                            | "iframe"
                            | "object"
                            | "embed"
                            | "img"
                            | "image"
                            | "video"
                            | "audio"
                            | "source"
                            | "track"
                            | "base"
                            | "form"
                            | "style"
                    ) {
                        self.gap("html-resource-or-style-contract", &site);
                    }
                    self.attributes(attributes, &site)?;
                    self.nodes(children, depth + 1)?;
                }
                TemplateNode::Result {
                    instruction,
                    attributes,
                    children,
                    ..
                } => {
                    if matches!(
                        instruction,
                        crate::render::ResultInstruction::Element
                            | crate::render::ResultInstruction::Attribute
                    ) {
                        self.gap("constructed-element-or-attribute", &site);
                    }
                    self.attributes(attributes, &site)?;
                    self.nodes(children, depth + 1)?;
                }
                TemplateNode::Expression(expression) => self.expression(expression)?,
                TemplateNode::If { test, children, .. } => {
                    if let Some(test) = test {
                        self.expression(test)?;
                    }
                    self.nodes(children, depth + 1)?;
                }
                TemplateNode::Choose { branches, .. } => {
                    for branch in branches {
                        self.charge(1)?;
                        if let Some(test) = &branch.test {
                            self.expression(test)?;
                        }
                        self.nodes(&branch.children, depth + 1)?;
                    }
                }
                TemplateNode::ForEach {
                    select, children, ..
                } => {
                    if let Some(select) = select {
                        self.expression(select)?;
                    }
                    self.nodes(children, depth + 1)?;
                }
                TemplateNode::Variable { select, .. } => {
                    if let Some(select) = select {
                        self.expression(select)?;
                    }
                }
                TemplateNode::ProjectPayload { select, .. } => {
                    self.gap("projected-payload", &site);
                    if let Some(select) = select {
                        self.expression(select)?;
                    }
                }
                TemplateNode::Text { .. } | TemplateNode::Comment { .. } => (),
            }
        }
        Ok(())
    }
}
pub(crate) fn declaration_requirements(
    artifact: &TemplateArtifact,
    metadata: &DeclarationArtifactMetadata,
    metadata_hash: &str,
) -> Result<DeclarationRequirements, String> {
    let mut inventory = Inventory {
        registry: ModuleRegistry::with_all_known(),
        items: 0,
        output: DeclarationRequirements {
            metadata_hash: metadata_hash.into(),
            source_uri: metadata.source_uri.clone(),
            declaration_source_hash: metadata.declaration_source_hash.clone(),
            template_hash: metadata.template.content_hash.clone(),
            template_format: metadata.template_format.clone(),
            capability: metadata.capability.clone(),
            host_bindings: metadata.template.host_bindings.iter().cloned().collect(),
            functions: BTreeSet::new(),
            pipeline_operations: BTreeSet::new(),
            data_imports: BTreeSet::new(),
            stylesheets: vec![],
            unresolved: BTreeSet::new(),
            template_node_count: 0,
            expression_count: 0,
            ir_node_count: 0,
        },
    };
    if artifact
        .diagnostics
        .iter()
        .any(|d| d.severity.is_hard_violation())
    {
        inventory.gap("template-diagnostics", "template");
    }
    if metadata.capability.is_some() {
        inventory.gap("shared-behavior-closure", "declaration");
    }
    inventory.nodes(&artifact.nodes, 0)?;
    let handle = CemResolutionContextHandle::new("requirements");
    let mut frame = CemModuleUrlFrame::new("declaration", &metadata.resource_base_url);
    if let Some(map) = &artifact.module_map {
        frame.specifiers = map.specifiers.clone();
        frame.scopes = map.scopes.clone();
    }
    let context = CemModuleUrlContext {
        identity: metadata.declaration_source_hash.clone(),
        resolver_identity: "build-requirements/1".into(),
        resource_policy_stamp: "inventory-only/1".into(),
        frames: vec![frame],
    };
    let capability = CemModuleUrlResolutionCapability::new(
        Arc::new(CemScopedModuleUrlResolver::new().with_context(handle.clone(), context)),
        handle,
    );
    for (occurrence, style) in artifact.stylesheets.iter().enumerate() {
        inventory.charge(1)?;
        let site = format!("style:{occurrence}");
        if let Some(imports) = &style.compiled_imports {
            imports.check_base(&metadata.resource_base_url)?;
        }
        let mut closure = match CssImportClosure::new(
            style.css_tree().clone(),
            &metadata.resource_base_url,
            capability.clone(),
            Default::default(),
            AbortSignal::new(),
        ) {
            Ok(closure) => closure,
            Err(error) if style.compiled_imports.is_some() => return Err(error.to_string()),
            Err(_) => {
                inventory.gap("css-resource-contract", &site);
                continue;
            }
        };
        let mut count = 0;
        loop {
            match closure.next_import() {
                Ok(Some(request)) => {
                    if let Some(imports) = &style.compiled_imports {
                        imports
                            .complete(&mut closure, count, &request, &Default::default())
                            .map_err(|e| e.to_string())?;
                        count += 1;
                    } else {
                        inventory.gap("source-css-import", &site);
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) if style.compiled_imports.is_some() => return Err(error.to_string()),
                Err(_) => {
                    inventory.gap("source-css-import", &site);
                    break;
                }
            }
        }
        if style
            .compiled_imports
            .as_ref()
            .is_some_and(|imports| count != imports.len())
        {
            return Err("requirements unused CSS responses".into());
        }
        for (sheet, entry) in closure.sheets().iter().enumerate() {
            inventory.charge(entry.resources.references.len() + 1)?;
            let mut resources = Vec::new();
            for reference in &entry.resources.references {
                let resolution = reference.resolution.as_ref().ok();
                if resolution.is_none() {
                    inventory.gap("css-resource-unresolved", &site);
                }
                resources.push(ResourceRequirement {
                    kind: match &reference.kind {
                        CssResourceKind::Import { .. } => "css-import",
                        CssResourceKind::Url => "css-url",
                    }
                    .into(),
                    specifier: reference.authored_specifier.clone(),
                    resolved_url: resolution.map(|r| r.resolved_url.clone()),
                    content_type: resolution.and_then(|r| r.content_type_hint.clone()),
                    integrity: resolution.and_then(|r| r.integrity.clone()),
                });
            }
            inventory.output.stylesheets.push(StylesheetRequirement {
                occurrence,
                sheet,
                source_uri: entry.resources.tree.source_uri().into(),
                source_key: entry.resources.tree.source_key(0),
                base_url: entry.url.clone(),
                resources,
            });
        }
    }
    Ok(inventory.output)
}

/// Admit the exact pinned build before deriving requirements. The returned
/// report remains build metadata and conveys no native/runtime permissions.
pub fn compiled_declaration_requirements(
    built: &crate::declaration_artifact::BuiltDeclarationArtifact,
) -> Result<DeclarationRequirements, String> {
    let metadata = crate::declaration_artifact::admit_declaration_metadata(
        &built.metadata_json,
        &built.metadata_hash,
    )?;
    if built.template.identity.artifact_version != metadata.template_format {
        return Err("requirements template format/pin mismatch".into());
    }
    let loaded = built
        .template
        .reload_pinned(&metadata.template)
        .map_err(|e| e.to_string())?;
    declaration_requirements(&loaded, &metadata, &built.metadata_hash)
}
