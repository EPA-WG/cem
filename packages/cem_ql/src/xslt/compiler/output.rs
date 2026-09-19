//! XSLT owns its error names and authoring rules; construction is shared CEMT.
use super::*;

impl Compiler<'_> {
    pub(super) fn result_policy(&self, event: &XmlEventAst) -> String {
        format!(
            "@error-uri=\"http://www.w3.org/2005/xqt-errors\" @error-attribute_order=XTDE0410 @error-document_attribute=XTDE0420 @error-item=XTDE0450 @error-namespace=XTDE0835 @error-reserved_attribute=XTDE0855 @origin-content-type=\"application/xslt+xml\" @error-name=XTDE0820 @origin-uri={} @origin-line={} @origin-column={} @origin-offset={} @origin-length={}",
            query_attribute(&format!("{{ {} }}", quote(&self.stylesheet.xml_document.source.uri))), event.source_range.start.line, event.source_range.start.column, event.source_range.start.byte_offset, event.source_range.byte_length,
        )
    }

    pub(super) fn result_select(
        &mut self,
        event: &XmlEventAst,
        scope: &Scope,
    ) -> CompileResult<String> {
        let select = self.program(
            event,
            self.expression(event, "select")?,
            scope,
            xpath::ResultKind::Sequence,
        )?;
        Ok(format!(
            "{{result-sequence @select={}}}",
            query_attribute(&select)
        ))
    }

    fn avt(
        &mut self,
        event: &XmlEventAst,
        attribute: &XmlAttributeAst,
        scope: &Scope,
    ) -> CompileResult<String> {
        let avt = self
            .stylesheet
            .attribute_value_templates
            .iter()
            .find(|a| a.event_index == event.index && a.attribute_name == attribute.qualified_name)
            .cloned();
        let Some(avt) = avt else {
            return Ok(emit_text(&self.literal_attribute(event, attribute)?));
        };
        let mut output = String::new();
        for segment in avt.segments {
            match segment {
                XsltAttributeValueTemplateSegmentAst::Literal { effective, .. } => {
                    output.push_str(&emit_text(&effective))
                }
                XsltAttributeValueTemplateSegmentAst::Expression { expression, .. } => {
                    let select = self.program(
                        event,
                        *expression,
                        scope,
                        xpath::ResultKind::Text(" ".into()),
                    )?;
                    output.push_str(&format!("{{$ {select}}}"));
                }
                _ => return Err(self.error(event, "XTSE0370", "invalid attribute value template")),
            }
        }
        Ok(output)
    }

    pub(super) fn literal(
        &mut self,
        node: &AuthorNode<'_>,
        scope: &Scope,
    ) -> CompileResult<String> {
        let event = node.event;
        if event.local_name.as_deref() == Some("script")
            || (event.local_name.as_deref() == Some("style")
                && node.children.iter().any(|n| is_element(n.event)))
        {
            return Err(self.error(event, "cem.xslt.compile_unsupported", "only static result styles are supported; script and dynamic lexical islands remain outside this profile"));
        }
        let mut output = format!(
            "{{result-element @name={} @namespace={} {} |",
            quote(event.qualified_name.as_deref().unwrap_or_default()),
            quote(event.namespace_uri.as_deref().unwrap_or("")),
            self.result_policy(event)
        );
        for attribute in &event.attributes {
            if attribute.qualified_name == "xmlns" || attribute.prefix.as_deref() == Some("xmlns") {
                continue;
            }
            if attribute.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI) {
                return Err(self.error(
                    event,
                    "cem.xslt.compile_unsupported",
                    "XSLT control attributes on literal results are outside this profile",
                ));
            }
            output.push_str(&format!(
                "{{result-attribute @name={} @namespace={} |{}}}",
                quote(&attribute.qualified_name),
                quote(attribute.namespace_uri.as_deref().unwrap_or("")),
                self.avt(event, attribute, scope)?
            ));
        }
        output.push_str(&self.sequence(&node.children, scope.clone())?);
        output.push('}');
        Ok(output)
    }

    pub(super) fn result_constructor(
        &mut self,
        node: &AuthorNode<'_>,
        scope: &Scope,
    ) -> CompileResult<String> {
        let event = node.event;
        let kind = event.local_name.as_deref().unwrap_or_default();
        self.attributes(
            event,
            if kind == "document" {
                &[]
            } else if kind == "attribute" {
                &["name", "namespace", "select"]
            } else {
                &["name", "namespace"]
            },
        )?;
        let policy = if kind == "attribute" {
            self.result_policy(event)
                .replace("@error-item=XTDE0450", "@error-item=FOTY0013")
                .replace("@error-name=XTDE0820", "@error-name=XTDE0850")
                .replace("@error-namespace=XTDE0835", "@error-namespace=XTDE0865")
        } else {
            self.result_policy(event)
        };
        let mut output = format!("{{result-{kind} {policy}");
        if kind != "document" {
            let attribute = self
                .attribute(event, "name")
                .ok_or_else(|| self.error(event, "XTSE0010", "constructor requires name"))?;
            // The bounded profile supports static expanded names. Runtime AVT
            // name resolution and namespace instructions remain explicit errors.
            let mut name = self.literal_attribute(event, attribute)?;
            if !cem_ml::validation::xpath::xpath_is_qname(&name) {
                return Err(self.error(
                    event,
                    "cem.xslt.compile_unsupported",
                    "constructor name must be a static lexical QName",
                ));
            }
            let explicit_namespace = self
                .attribute(event, "namespace")
                .map(|a| self.literal_attribute(event, a))
                .transpose()?;
            let namespace = if let Some(uri) = explicit_namespace {
                uri
            } else {
                let context = self.authored_context(event);
                let XPathAttachment::StandaloneStaticContext { static_context, .. } =
                    context.attachment
                else {
                    unreachable!("authoring context is static");
                };
                let prefix = name.split_once(':').map(|(prefix, _)| prefix);
                if let Some(prefix) = prefix {
                    static_context
                        .namespaces
                        .get(prefix)
                        .cloned()
                        .ok_or_else(|| {
                            self.error(event, "XTSE0280", "constructor prefix is not declared")
                        })?
                } else if kind == "element" {
                    static_context
                        .namespaces
                        .get("")
                        .cloned()
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            };
            if namespace.is_empty() {
                name = name.rsplit(':').next().unwrap_or_default().into();
            }
            output.push_str(&format!(
                " @name={} @namespace={}",
                quote(&name),
                quote(&namespace)
            ));
        }
        output.push_str(" |");
        if kind == "attribute" && self.attribute(event, "select").is_some() {
            self.empty(node)?;
            let selected = self.program(
                event,
                self.expression(event, "select")?,
                scope,
                xpath::ResultKind::Text(" ".into()),
            )?;
            output.push_str(&format!("{{$ {selected}}}"));
        } else {
            output.push_str(&self.sequence(&node.children, scope.clone())?);
        }
        output.push('}');
        Ok(output)
    }
}
