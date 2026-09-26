//! Managed emission over a ready native import graph, without loading or parsing.
use super::{
    diagnostic, emit_css_import_conditions, emit_css_scope_wrapper,
    stylesheet::collect,
    subtree::{compose, SubtreeOptions},
    CssEmissionDiagnostic, CssGroupingContext, CssImportConditionEmission, CssKeyframesEmission,
    CssManagedScope, CssRuleMode, CssRuleSubtreeEmission, CssSubtreeFragment,
};
use crate::{
    css_imports::{CssImportClosure, CssImportState},
    css_resources::named,
    parser::AstNodeId,
};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct CssClosureFragment {
    /// Import occurrence index. Node IDs are local to this sheet's retained tree.
    pub sheet: usize,
    pub fragment: CssSubtreeFragment,
}
#[derive(Debug)]
pub struct CssClosureDiagnostic {
    pub sheet: usize,
    pub diagnostic: CssEmissionDiagnostic,
}
#[derive(Debug, Default)]
pub struct CssClosureEmission {
    pub namespace: String,
    pub animation_names: BTreeMap<String, String>,
    pub fragments: Vec<CssClosureFragment>,
    pub diagnostics: Vec<CssClosureDiagnostic>,
}
impl CssClosureEmission {
    pub fn css(&self) -> String {
        self.fragments
            .iter()
            .map(|f| f.fragment.text.as_str())
            .collect()
    }
}

enum Part {
    Rule(usize, AstNodeId),
    Boundary(CssClosureFragment),
}

/// Emit a ready closure under one managed scope. `owner_identity` must identify
/// the effective declaration/stylesheet and resolution context, and remain stable
/// across hydration. Its UTF-8 bytes are hex-encoded, avoiding hash collisions or
/// dependence on delivery timing, sheet indices or process-local addresses.
/// All admitted imported definitions share this namespace, as in an ordinary
/// flattened import tree. Unmapped names remain external. Source trees are reused.
/// This performs no loading, installation, ownership registration or cache lookup.
/// Callers must inspect diagnostics for unsupported grammar before installation.
pub fn emit_css_import_closure(
    closure: &CssImportClosure,
    scope: &CssManagedScope,
    owner_identity: &str,
) -> Result<CssClosureEmission, CssClosureDiagnostic> {
    ready(closure)?;
    let root = root(closure, 0)?;
    if owner_identity.is_empty() {
        return Err(error(
            closure,
            0,
            root,
            "cem.scoped_css.owner_identity_invalid",
            "managed CSS owner identity must not be empty",
        ));
    }
    let wrapper = emit_css_scope_wrapper(scope).map_err(|diagnostic| CssClosureDiagnostic {
        sheet: 0,
        diagnostic,
    })?;
    let namespace = format!(
        "cem-{}",
        owner_identity
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let mut output = CssClosureEmission {
        namespace,
        ..Default::default()
    };
    let mut parts = Vec::new();
    expand(closure, 0, 0, &mut parts, &mut output.diagnostics)?;
    let mut definitions: BTreeMap<usize, BTreeMap<AstNodeId, CssKeyframesEmission>> =
        BTreeMap::new();
    for part in &parts {
        if let Part::Rule(sheet, id) = part {
            collect(
                &closure.sheets()[*sheet].resources,
                *id,
                &output.namespace,
                0,
                definitions.entry(*sheet).or_default(),
            )
            .map_err(|diagnostic| CssClosureDiagnostic {
                sheet: *sheet,
                diagnostic,
            })?;
        }
    }
    output.animation_names = definitions
        .values()
        .flat_map(|defs| defs.values())
        .filter_map(|d| d.rule.as_ref())
        .map(|r| (r.name.clone(), r.scoped_name.clone()))
        .collect();
    let mode = if matches!(scope, CssManagedScope::Instance) {
        CssRuleMode::Instance
    } else {
        CssRuleMode::Declaration
    };
    output
        .fragments
        .push(boundary(closure, 0, root, wrapper.opening));
    for part in parts {
        match part {
            Part::Boundary(fragment) => output.fragments.push(fragment),
            Part::Rule(sheet, id) => {
                let mut body = CssRuleSubtreeEmission::default();
                compose(
                    &closure.sheets()[sheet].resources,
                    id,
                    &SubtreeOptions {
                        mode,
                        names: Some(&output.animation_names),
                        definitions: definitions.get(&sheet),
                    },
                    CssGroupingContext::Stylesheet,
                    0,
                    &mut body,
                )
                .map_err(|diagnostic| CssClosureDiagnostic { sheet, diagnostic })?;
                output.fragments.extend(
                    body.fragments
                        .into_iter()
                        .map(|fragment| CssClosureFragment { sheet, fragment }),
                );
                output.diagnostics.extend(
                    body.diagnostics
                        .into_iter()
                        .map(|diagnostic| CssClosureDiagnostic { sheet, diagnostic }),
                );
            }
        }
    }
    output
        .fragments
        .push(boundary(closure, 0, root, wrapper.closing.into()));
    ready(closure)?;
    Ok(output)
}

fn expand(
    closure: &CssImportClosure,
    sheet: usize,
    depth: usize,
    parts: &mut Vec<Part>,
    diagnostics: &mut Vec<CssClosureDiagnostic>,
) -> Result<(), CssClosureDiagnostic> {
    if depth >= 64 {
        return Err(error(
            closure,
            sheet,
            0,
            "cem.scoped_css.import_depth_exceeded",
            "emission exceeds 64 import levels",
        ));
    }
    let tree = &closure.sheets()[sheet].resources.tree;
    for &id in &tree.node(root(closure, sheet)?).unwrap().children {
        if named(tree, id, "comment") || named(tree, id, "charset") {
            continue;
        }
        if named(tree, id, "rule") {
            parts.push(Part::Rule(sheet, id));
            continue;
        }
        if !named(tree, id, "import") {
            diagnostics.push(error(
                closure,
                sheet,
                id,
                "cem.scoped_css.stylesheet_construct_unsupported",
                "unsupported retained stylesheet child",
            ));
            continue;
        }
        match emit_css_import_conditions(tree, id)
            .map_err(|diagnostic| CssClosureDiagnostic { sheet, diagnostic })?
        {
            CssImportConditionEmission::Suppress(diagnostic) => {
                diagnostics.push(CssClosureDiagnostic { sheet, diagnostic })
            }
            CssImportConditionEmission::Emit {
                wrappers,
                diagnostics: conditions,
            } => {
                diagnostics.extend(
                    conditions
                        .into_iter()
                        .map(|diagnostic| CssClosureDiagnostic { sheet, diagnostic }),
                );
                let edge = closure
                    .edges()
                    .iter()
                    .find(|e| e.parent_sheet == sheet && e.import_node == id)
                    .ok_or_else(|| {
                        error(
                            closure,
                            sheet,
                            id,
                            "cem.scoped_css.import_edge_missing",
                            "ready closure has no matching import occurrence",
                        )
                    })?;
                for wrapper in &wrappers {
                    parts.push(Part::Boundary(CssClosureFragment {
                        sheet,
                        fragment: CssSubtreeFragment {
                            text: wrapper.opening.clone(),
                            node_id: wrapper.node_id,
                            source: wrapper.source.clone(),
                            range: wrapper.range,
                        },
                    }));
                }
                expand(closure, edge.child_sheet, depth + 1, parts, diagnostics)?;
                for wrapper in wrappers.iter().rev() {
                    parts.push(Part::Boundary(CssClosureFragment {
                        sheet,
                        fragment: CssSubtreeFragment {
                            text: "}".into(),
                            node_id: wrapper.node_id,
                            source: wrapper.source.clone(),
                            range: wrapper.range,
                        },
                    }));
                }
            }
        }
    }
    Ok(())
}
fn root(closure: &CssImportClosure, sheet: usize) -> Result<AstNodeId, CssClosureDiagnostic> {
    let tree = &closure.sheets()[sheet].resources.tree;
    tree.node(0)
        .and_then(|n| (n.children.len() == 1).then(|| n.children[0]))
        .filter(|id| named(tree, *id, "stylesheet") || named(tree, *id, "style-block"))
        .ok_or_else(|| {
            error(
                closure,
                sheet,
                0,
                "cem.scoped_css.stylesheet_tree_invalid",
                "expected a retained stylesheet or style-block",
            )
        })
}
fn ready(closure: &CssImportClosure) -> Result<(), CssClosureDiagnostic> {
    if closure.state() != CssImportState::Ready {
        return Err(error(
            closure,
            0,
            0,
            "cem.scoped_css.import_closure_not_ready",
            "CSS import closure must be ready and uncancelled",
        ));
    }
    Ok(())
}
fn error(
    closure: &CssImportClosure,
    sheet: usize,
    id: AstNodeId,
    code: &'static str,
    message: &str,
) -> CssClosureDiagnostic {
    CssClosureDiagnostic {
        sheet,
        diagnostic: diagnostic(&closure.sheets()[sheet].resources.tree, id, code, message),
    }
}
fn boundary(
    closure: &CssImportClosure,
    sheet: usize,
    id: AstNodeId,
    text: String,
) -> CssClosureFragment {
    let node = closure.sheets()[sheet].resources.tree.node(id).unwrap();
    CssClosureFragment {
        sheet,
        fragment: CssSubtreeFragment {
            text,
            node_id: id,
            source: node.source.clone(),
            range: node.range,
        },
    }
}
