//! Import-owned XML events feed the existing lexical/schema frame rules.
use super::{
    CemSchemaMachine, CompiledSchema, EventNormalizer, NormalizedEvent, PendingAttr,
    PendingSchemaElement,
};
use crate::{
    diagnostics::Diagnostic,
    events::SeparatorKind,
    parser::AstNodeId,
    schema::machine::{LexicalScopeSnapshot, SchemaElementForm},
    source::ByteRange,
    validation::xml::{XmlEventAst, XmlEventKind},
};
use std::collections::BTreeMap;
const CEM_NS: &str = "https://cem.dev/ns/cem-ml/1";
struct NoEvents;
impl EventNormalizer for NoEvents {
    fn next_event(&mut self) -> Option<NormalizedEvent> {
        None
    }
}

pub(crate) struct XmlLexicalCapture {
    machine: CemSchemaMachine<NoEvents>,
    occurrences: BTreeMap<AstNodeId, LexicalScopeSnapshot>,
    schema_element_forms: BTreeMap<AstNodeId, SchemaElementForm>,
}
impl XmlLexicalCapture {
    pub(crate) fn new(schema: CompiledSchema) -> Self {
        Self {
            machine: CemSchemaMachine::new(schema, NoEvents),
            occurrences: BTreeMap::new(),
            schema_element_forms: BTreeMap::new(),
        }
    }
    pub(crate) fn open(&mut self, event: &XmlEventAst, node: AstNodeId) {
        if event.namespace_uri.as_deref() == Some("https://cem.dev/ns/core/1")
            && event.local_name.as_deref() == Some("schema")
        {
            self.schema_element_forms.insert(
                node,
                if event.kind == XmlEventKind::StartElement {
                    SchemaElementForm::Wrapping
                } else {
                    SchemaElementForm::Following
                },
            );
        }
        let range = ByteRange::new(
            event.source_range.start.byte_offset,
            event
                .source_range
                .byte_length
                .try_into()
                .unwrap_or(u32::MAX),
        );
        self.machine.on_open(
            event.qualified_name.as_deref().unwrap_or(""),
            range,
            event.source_range.source_map(),
        );
        // XML owns expanded names. Authored aliases and a foreign `cem` prefix
        // cannot select schema behavior merely by their lexical spelling.
        *self
            .machine
            .pending_schema_elements
            .last_mut()
            .expect("opened frame") = (event.namespace_uri.as_deref() == Some(CEM_NS)
            && event.local_name.as_deref() == Some("schema"))
        .then_some(PendingSchemaElement {
            open_byte_range: range,
            source_node: Some(node),
            cem_name: None,
            src: None,
            select: None,
            is_self_closing: true,
        });
        for attr in &event.attributes {
            self.machine.commit_pending_annotation();
            let value = attr
                .entity_decoded_value
                .clone()
                .expect("import validated attribute");
            let source = attr.value_source_range.unwrap_or(event.source_range);
            let range = ByteRange::new(
                source.start.byte_offset,
                source.byte_length.try_into().unwrap_or(u32::MAX),
            );
            if attr.qualified_name == "xmlns" || attr.prefix.as_deref() == Some("xmlns") {
                let prefix = if attr.qualified_name == "xmlns" {
                    ""
                } else {
                    &attr.local_name
                };
                self.machine
                    .ns_contexts
                    .last_mut()
                    .expect("opened namespace frame")
                    .declare(prefix, value, range, range, source.source_map());
            } else {
                let name = if attr.namespace_uri.as_deref() == Some(CEM_NS) {
                    format!("cem:{}", attr.local_name)
                } else if attr.prefix.as_deref() == Some("cem") {
                    format!("xml-foreign:{}", attr.local_name)
                } else {
                    attr.qualified_name.clone()
                };
                self.machine.handle_attribute(
                    PendingAttr {
                        name,
                        name_range: range,
                    },
                    value,
                    range,
                );
            }
        }
        if event.namespace_uri.as_deref() == Some(CEM_NS)
            && event.local_name.as_deref() == Some("expr")
        {
            self.occurrences
                .insert(node, self.machine.lexical_snapshot());
        }
        if event.kind == XmlEventKind::StartElement {
            self.machine.consume(NormalizedEvent::Separator {
                kind: SeparatorKind::ElementBoundary,
                byte_range: range,
            });
        } else {
            self.close(event);
        }
    }
    pub(crate) fn close(&mut self, event: &XmlEventAst) {
        self.machine.commit_pending_annotation();
        self.machine
            .on_close(event.qualified_name.as_deref().unwrap_or(""));
    }
    pub(crate) fn finish(
        mut self,
    ) -> (
        BTreeMap<AstNodeId, LexicalScopeSnapshot>,
        Vec<Diagnostic>,
        BTreeMap<AstNodeId, SchemaElementForm>,
    ) {
        self.machine.finalize();
        (
            self.occurrences,
            self.machine.diagnostics,
            self.schema_element_forms,
        )
    }
}
