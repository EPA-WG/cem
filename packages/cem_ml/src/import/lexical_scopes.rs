//! Opt-in lexical associations use the same imported AST and event correspondence.
use super::{import_xml_ast_tracked, xml, CemTreeSemantics};
use crate::{
    parser::AstNodeId,
    schema::{
        machine::{LexicallyScopedDocument, XmlLexicalCapture},
        vocab::CompiledSchema,
    },
};
use std::sync::Arc;

pub struct ScopedXmlCemImport {
    pub captured: LexicallyScopedDocument,
    pub semantics: CemTreeSemantics,
    pub event_nodes: Vec<Option<AstNodeId>>,
    pub attribute_nodes: Vec<Vec<AstNodeId>>,
}

/// Import once and retain source-position bindings for standalone native XML
/// references and explicitly marked attribute slots. Other attributes remain
/// literals. Schema selection/evaluation,
/// readiness and crossing grants remain explicit consumer lifecycle stages.
pub fn import_xml_ast_with_lexical_scopes(
    document: &xml::XmlDocumentAst,
    schema: CompiledSchema,
) -> Result<ScopedXmlCemImport, String> {
    let mut capture = Some(XmlLexicalCapture::new(schema));
    let imported = import_xml_ast_tracked(document, &mut capture)?;
    let (occurrences, diagnostics, schema_element_forms, namespace_bindings, attribute_namespaces) =
        capture.take().expect("capture installed").finish();
    Ok(ScopedXmlCemImport {
        captured: LexicallyScopedDocument::from_import(
            Arc::new(imported.ast),
            occurrences,
            diagnostics,
            schema_element_forms,
            namespace_bindings,
            attribute_namespaces,
        ),
        semantics: imported.semantics,
        event_nodes: imported.event_nodes,
        attribute_nodes: imported.attribute_nodes,
    })
}

/// One retained import with source-position lexical metadata. Both handles share
/// the same original CEM arena; XML keeps its original input owner as provenance.
#[derive(Debug, Clone)]
pub struct ScopedCemImport {
    pub captured: Arc<LexicallyScopedDocument>,
    pub tree: Arc<crate::parser::tree::RetainedCemTree>,
}

/// Opt-in native CEM/XML byte ingress for consumers needing lexical associations.
/// MIME and decoding remain at this shared import boundary. No selector execution,
/// declaration compilation, scope registration or target writeback occurs here.
pub fn import_bytes_with_lexical_scopes(
    bytes: &[u8],
    content_type: &str,
    source_uri: &str,
    schema: CompiledSchema,
) -> Result<ScopedCemImport, String> {
    import_bytes_with_lexical_scopes_and_profile(bytes, content_type, source_uri, schema, None)
}

/// Explicit enclosing format capability for headerless CEM fragments. Authored
/// leading document constraints take precedence; XML does not consume this option.
pub fn import_bytes_with_lexical_scopes_and_profile(
    bytes: &[u8],
    content_type: &str,
    source_uri: &str,
    schema: CompiledSchema,
    profile: Option<crate::schema::ir::SemVer>,
) -> Result<ScopedCemImport, String> {
    use crate::{
        events::cem::CemEventNormalizer,
        parser::tree::RetainedCemTree,
        schema::machine::CemSchemaMachine,
        source::{BytesSource, SourceId},
        tokenizer::cem::CemTokenizer,
    };
    if bytes.len() > super::MAX_DOCUMENT_BYTES {
        return Err("Source exceeds the 16 MiB document import limit.".into());
    }
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let native_cem = matches!(
        mime.as_str(),
        "application/cem" | "text/cem" | "text/cem-ml"
    ) || mime.ends_with("+cem");
    let (captured, mut semantics, native, text) = if native_cem {
        for parameter in content_type.split(';').skip(1) {
            if let Some((name, value)) = parameter.trim().split_once('=') {
                if name.eq_ignore_ascii_case("charset")
                    && !value.trim_matches('"').eq_ignore_ascii_case("utf-8")
                {
                    return Err("Native CEM byte import requires UTF-8.".into());
                }
            }
        }
        let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
        let source = BytesSource::new(SourceId(1), bytes.to_vec());
        let tokenizer = match profile.clone() {
            Some(profile) => CemTokenizer::from_source_with_format_profile(source, profile)?,
            None => CemTokenizer::from_source(source),
        };
        let events = CemEventNormalizer::new(tokenizer);
        let captured = CemSchemaMachine::new(schema, events).build_with_lexical_scopes();
        if let Some(error) = captured
            .diagnostics()
            .iter()
            .find(|d| d.code.starts_with("cem.doc."))
        {
            return Err(format!(
                "Unsupported CEM document admission: {}",
                error.code
            ));
        }
        if captured.document().nodes.len() > super::MAX_DOCUMENT_VALUES {
            return Err("CEM document node import limit exceeded.".into());
        }
        (captured, CemTreeSemantics::default(), None, text)
    } else {
        if profile.is_some() {
            return Err("CEM format profiles require native CEM input.".into());
        }
        let native = Arc::new(super::parse_bytes(
            bytes,
            content_type,
            source_uri,
            super::MAX_DOCUMENT_VALUES,
        )?);
        let crate::lifecycle::LoadedInputAstStream::XmlDocument(document) = native.as_ref() else {
            return Err("Lexical byte capture requires CEM or XML input.".into());
        };
        let imported = import_xml_ast_with_lexical_scopes(document, schema)?;
        (imported.captured, imported.semantics, Some(native), "")
    };
    let profile_key = profile.map(|p| p.to_string()).unwrap_or_default();
    semantics.source_fingerprint = Some(super::source_fingerprint(
        source_uri,
        bytes,
        &["lexical-import/2", content_type, &profile_key],
    ));
    let captured = Arc::new(captured);
    let tree = RetainedCemTree::from_shared(
        captured.document().clone(),
        source_uri,
        text,
        semantics,
        native.map(|owner| owner as Arc<dyn std::any::Any + Send + Sync>),
    )?;
    Ok(ScopedCemImport { captured, tree })
}
