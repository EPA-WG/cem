//! Native build-only static declaration closure; no browser source parsing.
use crate::{
    declaration_artifact::{
        admit_declaration_metadata, build_declaration_artifact, BuiltDeclarationArtifact,
    },
    render::{ResultInstruction, TemplateNode},
    template_artifact::TemplateArtifactSourceMapMode,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct DeclarationBuildSource {
    pub source: String,
    pub uri: String,
}
pub struct BuiltDeclarationClosure {
    pub declarations: BTreeMap<String, BuiltDeclarationArtifact>,
    pub dependencies: BTreeMap<String, BTreeSet<String>>,
    pub loader_html: String,
    pub requirements: crate::application_requirements::ApplicationRequirements,
}

/// Roots include entry declarations and all custom-element types projected by
/// the host. Dynamic construction cannot be inferred from an arbitrary query.
pub fn build_declaration_closure(
    roots: &[String],
    mode: TemplateArtifactSourceMapMode,
    mut load: impl FnMut(&str) -> Result<DeclarationBuildSource, String>,
) -> Result<BuiltDeclarationClosure, String> {
    build_with_css(roots, mode, &mut load, None)
}

/// Explicit compiled-CSS deployment closure. Full/source builds keep the original API.
pub fn build_declaration_closure_with_css(
    roots: &[String],
    mode: TemplateArtifactSourceMapMode,
    mut load: impl FnMut(&str) -> Result<DeclarationBuildSource, String>,
    mut css: impl FnMut(
        &cem_ml::css_imports::CssImportRequest,
    ) -> Result<cem_ml::resolver::ResolvedRead, String>,
) -> Result<BuiltDeclarationClosure, String> {
    build_with_css(roots, mode, &mut load, Some(&mut css))
}
fn build_with_css(
    roots: &[String],
    mode: TemplateArtifactSourceMapMode,
    load: &mut dyn FnMut(&str) -> Result<DeclarationBuildSource, String>,
    mut css: Option<
        &mut dyn FnMut(
            &cem_ml::css_imports::CssImportRequest,
        ) -> Result<cem_ml::resolver::ResolvedRead, String>,
    >,
) -> Result<BuiltDeclarationClosure, String> {
    if roots.is_empty()
        || roots.len() > 64
        || roots.iter().collect::<BTreeSet<_>>().len() != roots.len()
    {
        return Err("invalid declaration closure roots".into());
    }
    let mut pending: BTreeSet<String> = roots.iter().cloned().collect();
    let mut declarations = BTreeMap::new();
    let mut dependencies = BTreeMap::new();
    let mut total_bytes = 0usize;
    let mut asset_bytes = 0usize;
    let mut requirements = crate::application_requirements::ApplicationRequirements::default();
    requirements.roots = roots.iter().cloned().collect();
    while let Some(tag) = pending.pop_first() {
        if declarations.contains_key(&tag) {
            continue;
        }
        // Validate before handing a tag to a filesystem-backed loader.
        if !tag.starts_with(|c: char| c.is_ascii_lowercase())
            || !tag.contains('-')
            || tag.len() > 256
            || !tag.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'_' | b'.')
            })
            || declarations.len() >= 64
        {
            return Err("invalid/oversized declaration closure".into());
        }
        let input = load(&tag)?;
        total_bytes = total_bytes
            .checked_add(input.source.len())
            .ok_or("declaration closure byte limit")?;
        if total_bytes > 16 * 1024 * 1024 {
            return Err("declaration closure byte limit".into());
        }
        let built = build_declaration_artifact(&input.source, &input.uri, mode, None)?;
        let built = if let Some(css) = css.as_mut() {
            crate::declaration_artifact::build_declaration_imports(built, *css)?
        } else {
            built
        };
        asset_bytes = asset_bytes
            .checked_add(built.template.bytes.len() + built.metadata_json.len())
            .ok_or("declaration closure artifact limit")?;
        if asset_bytes > 64 * 1024 * 1024 {
            return Err("declaration closure artifact limit".into());
        }
        let metadata = admit_declaration_metadata(&built.metadata_json, &built.metadata_hash)?;
        if metadata.tag != tag {
            return Err("declaration dependency/source tag mismatch".into());
        }
        let loaded = built
            .template
            .reload_pinned(&metadata.template)
            .map_err(|e| e.to_string())?;
        requirements.declarations.insert(
            tag.clone(),
            crate::application_requirements::declaration_requirements(
                &loaded,
                &metadata,
                &built.metadata_hash,
            )?,
        );
        let mut refs = BTreeSet::new();
        references(&loaded.nodes, &mut refs)?;
        for dep in &refs {
            if !declarations.contains_key(dep) && dep != &tag {
                pending.insert(dep.clone());
            }
        }
        dependencies.insert(tag.clone(), refs);
        declarations.insert(tag, built);
    }
    requirements.component_dependencies = dependencies.clone();
    // Files have validated tag-only names. Metadata pins originate in this build,
    // independently of downloaded metadata. No source or inline bootstrap JS.
    let loader_html = declarations.iter().map(|(tag, built)| format!(
        "<cem-element tag=\"{tag}\" compiled-src=\"./{tag}.json\" compiled-template-src=\"./{tag}.bin\" compiled-hash=\"{}\"></cem-element>\n", built.metadata_hash,
    )).collect();
    Ok(BuiltDeclarationClosure {
        declarations,
        dependencies,
        loader_html,
        requirements,
    })
}

fn references(nodes: &[TemplateNode], tags: &mut BTreeSet<String>) -> Result<(), String> {
    for node in nodes {
        match node {
            TemplateNode::Element { tag, children, .. } => {
                if tag == "cem-element" {
                    return Err(
                        "runtime declaration construction requires a compiled contract".into(),
                    );
                }
                if tag.contains('-') {
                    tags.insert(tag.clone());
                }
                references(children, tags)?;
            }
            TemplateNode::Result {
                instruction: ResultInstruction::Element,
                ..
            } => return Err("dynamic element construction requires a compiled contract".into()),
            TemplateNode::Result { children, .. }
            | TemplateNode::If { children, .. }
            | TemplateNode::ForEach { children, .. } => references(children, tags)?,
            TemplateNode::Choose { branches, .. } => {
                for branch in branches {
                    references(&branch.children, tags)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
