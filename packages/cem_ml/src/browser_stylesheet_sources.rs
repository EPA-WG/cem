//! Scalar source predicates for the bounded browser observation adapter. Source
//! text comes from retained document values or the exact acquired response, never
//! from a DOM/JSON AST. This is correspondence evidence, not CSS installation.
use crate::{
    html_css_import_snapshot::HtmlExecutionCssImports, stylesheet_execution::StylesheetOwner,
};

#[derive(Debug)]
pub struct BrowserStylesheetSource {
    pub root: usize,
    pub sheet: usize,
    pub owner: u32,
    pub attribute: bool,
    pub parent_sheet: Option<usize>,
    pub import_index: Option<usize>,
    pub url: String,
    pub text: String,
}

pub fn browser_stylesheet_sources(
    css: &HtmlExecutionCssImports,
    max_sheets: usize,
    max_bytes: usize,
) -> Result<Vec<BrowserStylesheetSource>, String> {
    if css.sheet_count() > max_sheets {
        return Err("browser stylesheet occurrence limit exceeded".into());
    }
    let document = &css.document().document().tree;
    let mut result = Vec::new();
    let mut total = 0usize;
    for (root, descriptor) in css.execution_roots().iter().enumerate() {
        let (owner, attribute) = match descriptor.owner {
            StylesheetOwner::Document { owner, source_node } => (owner, owner != source_node),
            StylesheetOwner::Link { owner } => (owner, false),
            _ => {
                return Err(
                    "compiled body installation requires its recorded execution adapter".into(),
                )
            }
        };
        let closure = descriptor.closure.closure();
        for (sheet, item) in closure.sheets().iter().enumerate() {
            let text = if sheet == 0 && root < css.closures().len() {
                let source = css.document().stylesheets()[root].source_node;
                document
                    .text_fragments(source)
                    .ok_or("missing native stylesheet source")?
                    .collect::<String>()
            } else {
                let response = if sheet == 0 {
                    &css.external()[root - css.closures().len()].source
                } else {
                    &descriptor
                        .closure
                        .acquisitions()
                        .iter()
                        .find(|a| a.sheet == sheet)
                        .ok_or("missing exact CSS acquisition")?
                        .source
                };
                // Browser comparison is deliberately a UTF-8 profile. Reject an
                // importer-selected legacy encoding rather than decode it twice.
                let native = item
                    .resources
                    .tree
                    .native_owner()
                    .and_then(|n| n.downcast_ref::<crate::lifecycle::LoadedInputAstStream>());
                let Some(crate::lifecycle::LoadedInputAstStream::CssDocument(doc)) = native else {
                    return Err("native CSS decoding evidence required".into());
                };
                if !doc
                    .encoding_report
                    .normalized_encoding
                    .eq_ignore_ascii_case("utf-8")
                {
                    return Err("browser stylesheet observations require UTF-8 sources".into());
                }
                std::str::from_utf8(&response.bytes)
                    .map_err(|_| "browser stylesheet observations require UTF-8 sources")?
                    .to_owned()
            };
            total = total
                .checked_add(text.len())
                .ok_or("browser stylesheet source byte overflow")?;
            if total > max_bytes {
                return Err("browser stylesheet source byte limit exceeded".into());
            }
            let edge = closure.edges().iter().find(|e| e.child_sheet == sheet);
            let import_index = edge.map(|e| {
                closure
                    .edges()
                    .iter()
                    .filter(|p| p.parent_sheet == e.parent_sheet && p.import_node < e.import_node)
                    .count()
            });
            result.push(BrowserStylesheetSource {
                root,
                sheet,
                owner,
                attribute,
                parent_sheet: edge.map(|e| e.parent_sheet),
                import_index,
                url: item.url.clone(),
                text,
            });
        }
    }
    Ok(result)
}
