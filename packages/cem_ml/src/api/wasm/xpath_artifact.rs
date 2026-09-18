//! Explicit control-plane entrypoints for XPath-owned compiled programs.
//! Handles here are not CEMT handles and do not install query callbacks.
use crate::{
    content_cache::{ContentHash, HASH_SCHEME},
    validation::xpath::{
        artifact::{XPathArtifactLoadContext, XPathCompiledArtifact, XPATH_ARTIFACT_CONTENT_TYPE},
        xpath_expression_ast_from_source_bytes, XPathAttachment, XPathExpressionAst,
        XPathInvocationHost, XPathSourceRequest, XPATH_CONTENT_TYPE, XPATH_SCHEMA_URI,
    },
};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};
use wasm_bindgen::prelude::*;

const MAX_PROGRAMS: usize = 64;
const MAX_RETAINED_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
struct Programs {
    next_id: u32,
    byte_count: usize,
    entries: BTreeMap<u32, (Arc<XPathExpressionAst>, usize)>,
}

thread_local! { static PROGRAMS: RefCell<Programs> = RefCell::new(Programs::default()); }

/// Rust-side host integration can bind this retained native program to an
/// explicitly allowed capability. Nothing is exposed through JSON data.
pub fn retained_xpath_artifact(id: u32) -> Option<Arc<XPathExpressionAst>> {
    PROGRAMS.with(|programs| {
        programs
            .borrow()
            .entries
            .get(&id)
            .map(|(expression, _)| expression.clone())
    })
}

#[wasm_bindgen(js_name = "compileXPathArtifact")]
pub fn compile_xpath_artifact(source: &str, source_uri: &str) -> Result<Vec<u8>, JsValue> {
    if source.len() > crate::validation::xpath::artifact::XPATH_ARTIFACT_MAX_BYTES {
        return Err(error(
            "cem.xpath.artifact_limit",
            "XPath source exceeds artifact input limit",
        ));
    }
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: source.as_bytes(),
            source_uri,
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::Standalone { source_id: 1 },
    );
    let artifact =
        XPathCompiledArtifact::compile(&expression, ContentHash::from_blake3(source.as_bytes()))
            .map_err(|e| error(e.code, &e.message))?;
    Ok(artifact.bytes().to_vec())
}

/// JSON here is artifact-control metadata only, never an expression or data AST.
#[wasm_bindgen(js_name = "importXPathArtifact")]
pub fn import_xpath_artifact(
    bytes: &[u8],
    expected_content_hash: &str,
    expected_source_hash: &str,
    invocation_host: &str,
) -> Result<String, JsValue> {
    if bytes.len() > crate::validation::xpath::artifact::XPATH_ARTIFACT_MAX_BYTES {
        return Err(error(
            "cem.xpath.artifact_limit",
            "XPath artifact exceeds byte limit",
        ));
    }
    let host = match invocation_host {
        "query" => XPathInvocationHost::Query,
        "standalone-transform" => XPathInvocationHost::StandaloneTransform,
        "cemt" => XPathInvocationHost::Cemt,
        "cem-ql" => XPathInvocationHost::CemQl,
        "xslt" => XPathInvocationHost::Xslt,
        _ => {
            return Err(error(
                "cem.xpath.artifact_identity_mismatch",
                "unknown XPath invocation host",
            ))
        }
    };
    PROGRAMS.with(|programs| {
        let mut programs = programs.borrow_mut();
        if programs.entries.len() >= MAX_PROGRAMS
            || bytes.len() > MAX_RETAINED_BYTES.saturating_sub(programs.byte_count) {
            return Err(error("cem.xpath.artifact_limit", "XPath retained-program limit exceeded"));
        }
        let id = programs.next_id.checked_add(1)
            .ok_or_else(|| error("cem.xpath.artifact_limit", "XPath artifact handle space exhausted"))?;
        let artifact = XPathCompiledArtifact::from_bytes(bytes.to_vec(), &parse_hash(expected_content_hash)?)
            .map_err(|e| error(e.code, &e.message))?;
        let expression = artifact.reload(&XPathArtifactLoadContext {
            expected_source_hash: parse_hash(expected_source_hash)?, invocation_host: host,
        }).map_err(|e| error(e.code, &e.message))?;
        programs.next_id = id;
        programs.byte_count += bytes.len();
        programs.entries.insert(id, (Arc::new(expression), bytes.len()));
        Ok(serde_json::json!({
            "artifactId": id, "contentType": XPATH_ARTIFACT_CONTENT_TYPE, "schemaUri": XPATH_SCHEMA_URI,
            "contentHash": artifact.content_hash().header_value(), "invocationHost": invocation_host,
        }).to_string())
    })
}

#[wasm_bindgen(js_name = "disposeXPathArtifact")]
pub fn dispose_xpath_artifact(id: u32) -> bool {
    PROGRAMS.with(|programs| {
        let mut programs = programs.borrow_mut();
        if let Some((_, bytes)) = programs.entries.remove(&id) {
            programs.byte_count -= bytes;
            true
        } else {
            false
        }
    })
}

fn parse_hash(value: &str) -> Result<ContentHash, JsValue> {
    let (scheme, hex) = value.split_once(':').ok_or_else(|| {
        error(
            "cem.xpath.artifact_identity_mismatch",
            "expected a CEM content hash",
        )
    })?;
    if scheme != HASH_SCHEME
        || hex.len() != 64
        || !hex
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(error(
            "cem.xpath.artifact_identity_mismatch",
            "invalid CEM content hash",
        ));
    }
    Ok(ContentHash {
        scheme: scheme.into(),
        hex: hex.into(),
    })
}

fn error(code: &str, message: &str) -> JsValue {
    JsValue::from_str(&format!("{code}: {message}"))
}
