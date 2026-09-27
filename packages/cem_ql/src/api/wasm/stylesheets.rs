//! Named stylesheet control and byte transport; retained CSS trees never leave WASM.
use super::*;
use crate::retained_template::{
    StylesheetLoadError, StylesheetLoadOptions, StylesheetLoadProgress,
};
use cem_ml::{
    css_emission::CssManagedScope,
    module_resolution::{
        CemModuleUrlContext, CemModuleUrlResolutionCapability, CemResolutionContextHandle,
        CemScopedModuleUrlResolver,
    },
};
use std::sync::Arc;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Begin {
    consumer: String,
    index: usize,
    declaration_identity: String,
    base_url: String,
    context: CemModuleUrlContext,
    scope: Scope,
}
#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
enum Scope {
    Private { tag: String },
    Shared { name: String },
}
fn error(error: impl Into<StylesheetLoadError>) -> String {
    let error = error.into();
    let diagnostics = match &error {
        StylesheetLoadError::Control(_) => Vec::new(),
        StylesheetLoadError::Import(failure) => {
            let mut diagnostic =
                json!({"code":failure.code, "message":failure.message, "severity":"error"});
            if let Some(location) = &failure.location {
                diagnostic["sourceUri"] = json!(location.source_uri);
                diagnostic["stylesheetUrl"] = json!(location.stylesheet_url);
                diagnostic["line"] = json!(location.range.line);
                diagnostic["column"] = json!(location.range.column);
                diagnostic["offset"] = json!(location.range.offset);
                diagnostic["length"] = json!(location.range.length);
            }
            vec![diagnostic]
        }
    };
    json!({"status":"error", "message":error.to_string(), "diagnostics":diagnostics}).to_string()
}

fn progress(id: u32, progress: StylesheetLoadProgress) -> Result<String, StylesheetLoadError> {
    Ok(match progress {
        StylesheetLoadProgress::Pending(request) => json!({"status":"pending", "loadId":id,
            "request":{"id":request.id,"url":request.resolution.resolved_url,
                "contentType":request.resolution.content_type_hint,"integrity":request.resolution.integrity}}).to_string(),
        StylesheetLoadProgress::Ready(record) => {
            let emission = record.emission()?;
            let diagnostics = emission.diagnostics.iter().map(|d| {
                let source = record.source(d.sheet)?;
                Ok(json!({"code":d.diagnostic.code, "message":d.diagnostic.message,
                    "severity":"warning", "sheet":d.sheet, "sourceUri":source.source_uri,
                    "stylesheetUrl":source.stylesheet_url,
                    "line":d.diagnostic.range.line, "column":d.diagnostic.range.column,
                    "offset":d.diagnostic.range.offset, "length":d.diagnostic.range.length}))
            }).collect::<Result<Vec<_>, &'static str>>()?;
            json!({"status":"ready", "loadId":id, "css":emission.css(),
                "identity":{"ownerKey":record.identity().owner_key,"cacheKey":record.identity().cache_key,
                    "contextMarker":record.identity().context_marker},
                "diagnostics":diagnostics}).to_string()
        }
    })
}
fn with_owner(
    id: u32,
    action: impl FnOnce(&mut RetainedTemplate) -> Result<String, StylesheetLoadError>,
) -> String {
    ARTIFACTS.with(|cell| {
        let mut artifacts = cell.borrow_mut();
        let Some(Some(owner)) = id
            .checked_sub(1)
            .and_then(|index| artifacts.get_mut(index as usize))
        else {
            return error("template artifact is stale or unknown");
        };
        action(owner).unwrap_or_else(error)
    })
}

#[wasm_bindgen(js_name = "beginTemplateStylesheet")]
pub fn begin(artifact_id: u32, options_json: &str) -> String {
    if options_json.len() > cem_ml::import::MAX_DOCUMENT_BYTES {
        return error("stylesheet control exceeds byte limit");
    }
    let options: Begin = match serde_json::from_str(options_json) {
        Ok(v) => v,
        Err(e) => return error(e.to_string()),
    };
    with_owner(artifact_id, |owner| {
        let handle = CemResolutionContextHandle::new("stylesheet-context");
        let capability = CemModuleUrlResolutionCapability::new(
            Arc::new(
                CemScopedModuleUrlResolver::new().with_context(handle.clone(), options.context),
            ),
            handle,
        );
        let consumer = options.consumer.clone();
        let id = owner.begin_stylesheet_load(StylesheetLoadOptions {
            consumer: options.consumer,
            index: options.index,
            declaration_identity: options.declaration_identity,
            base_url: options.base_url,
            capability,
            response_policy: Default::default(),
            scope: match options.scope {
                Scope::Private { tag } => CssManagedScope::Private { tag, context: None },
                Scope::Shared { name } => CssManagedScope::Shared {
                    name,
                    context: None,
                },
            },
        })?;
        progress(id, owner.advance_stylesheet_load(id, &consumer)?)
    })
}

#[wasm_bindgen(js_name = "deliverTemplateStylesheet")]
pub fn deliver(
    artifact_id: u32,
    load_id: u32,
    consumer: &str,
    request_id: u32,
    bytes: &[u8],
    final_url: &str,
    content_type: &str,
) -> String {
    with_owner(artifact_id, |owner| {
        progress(
            load_id,
            owner.deliver_stylesheet_response(
                load_id,
                consumer,
                request_id.into(),
                cem_ml::resolver::ResolvedRead {
                    uri: final_url.into(),
                    bytes: bytes.to_vec(),
                    content_type: (!content_type.is_empty()).then(|| content_type.to_owned()),
                },
            )?,
        )
    })
}

#[wasm_bindgen(js_name = "releaseTemplateStylesheets")]
pub fn release(artifact_id: u32, consumer: &str, load_id: u32) -> String {
    with_owner(artifact_id, |owner| {
        Ok(
            json!({"status":"released", "count":if load_id == 0 { owner.release_stylesheet_consumer(consumer) } else { owner.release_stylesheet_generation(consumer, load_id) }})
                .to_string(),
        )
    })
}
