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
/// references. Attribute values remain literals. Schema selection/evaluation,
/// readiness and crossing grants remain explicit consumer lifecycle stages.
pub fn import_xml_ast_with_lexical_scopes(
    document: &xml::XmlDocumentAst,
    schema: CompiledSchema,
) -> Result<ScopedXmlCemImport, String> {
    let mut capture = Some(XmlLexicalCapture::new(schema));
    let imported = import_xml_ast_tracked(document, &mut capture)?;
    let (occurrences, diagnostics, schema_element_forms) =
        capture.take().expect("capture installed").finish();
    Ok(ScopedXmlCemImport {
        captured: LexicallyScopedDocument::from_import(
            Arc::new(imported.ast),
            occurrences,
            diagnostics,
            schema_element_forms,
        ),
        semantics: imported.semantics,
        event_nodes: imported.event_nodes,
        attribute_nodes: imported.attribute_nodes,
    })
}
