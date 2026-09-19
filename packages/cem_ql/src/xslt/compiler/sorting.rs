//! XSLT sort orchestration; comparison remains in standard native XPath.
use super::*;

const CODEPOINT: &str = "http://www.w3.org/2005/xpath-functions/collation/codepoint";
const SORT: &str = r#"
let $values := $records?($index)
return let $double := exists($values[. instance of xs:double])
return let $float := exists($values[. instance of xs:float])
return let $input := if ($order = 'descending') then reverse($records) else $records
return let $sorted := sort($input, (), function($record) {
    let $key := $record?($index)
    return if ($double) then $key ! xs:double(.)
           else if ($float) then $key ! xs:float(.) else $key
})
return if ($order = 'descending') then reverse($sorted) else $sorted
"#;
const COMPARABLE: &str = r#"
let $types := distinct-values($records?($index) ! (
    if (. instance of xs:decimal or . instance of xs:float or . instance of xs:double) then 'number'
    else if (. instance of xs:string) then 'string'
    else if (. instance of xs:boolean) then 'boolean' else 'unsupported'))
return count($types) le 1 and not($types = 'unsupported')
"#;

fn is_sort(node: &AuthorNode<'_>) -> bool {
    is_element(node.event)
        && node.event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
        && node.event.local_name.as_deref() == Some("sort")
}

impl Compiler<'_> {
    fn macro_program(
        &mut self,
        event: &XmlEventAst,
        code: &str,
        scope: &Scope,
        bindings: &[(&str, &str)],
    ) -> CompileResult<String> {
        let mut expression = self.generated_xpath(event, "0")?;
        xpath::compiler_macro(&mut expression, code)
            .map_err(|message| self.error(event, "cem.xslt.compile_xpath", message))?;
        let scope = Scope {
            variables: bindings
                .iter()
                .map(|(name, value)| {
                    (
                        XPathExpandedName {
                            namespace_uri: None,
                            local_name: (*name).into(),
                        },
                        (*value).into(),
                    )
                })
                .collect(),
            groups: None,
            ..scope.clone()
        };
        self.program(event, expression, &scope, xpath::ResultKind::Sequence)
    }

    fn sort_error(&self, event: &XmlEventAst, code: &str, message: &str) -> String {
        format!(
            "report:raise({}, {})",
            quote(code),
            quote(&format!(
                "{}:{}:{}: {message}",
                self.stylesheet.xml_document.source.uri,
                event.source_range.start.line,
                event.source_range.start.column,
            ))
        )
    }

    fn sort_avt(
        &mut self,
        event: &XmlEventAst,
        name: &str,
        scope: &Scope,
        default: &str,
    ) -> CompileResult<String> {
        let Some(attribute) = self.attribute(event, name) else {
            return Ok(quote(default));
        };
        let Some(avt) = self
            .stylesheet
            .attribute_value_templates
            .iter()
            .find(|a| a.event_index == event.index && a.attribute_name == attribute.qualified_name)
            .cloned()
        else {
            return Ok(quote(
                attribute
                    .entity_decoded_value
                    .as_deref()
                    .unwrap_or_default(),
            ));
        };
        let mut parts = Vec::new();
        for segment in avt.segments {
            parts.push(match segment {
                XsltAttributeValueTemplateSegmentAst::Literal { effective, .. } => {
                    quote(&effective)
                }
                XsltAttributeValueTemplateSegmentAst::Expression { expression, .. } => self
                    .program(
                        event,
                        *expression,
                        scope,
                        xpath::ResultKind::Text(" ".into()),
                    )?,
                XsltAttributeValueTemplateSegmentAst::EmptyExpression { .. } => quote(""),
                _ => {
                    return Err(self.error(
                        event,
                        "cem.xslt.compile_attribute",
                        "invalid sort attribute value template",
                    ))
                }
            });
        }
        Ok(format!("str:concat(({}))", parts.join(", ")))
    }

    fn sort_control(
        &mut self,
        event: &XmlEventAst,
        name: &str,
        options: (&str, &[&str], &str),
        scope: &Scope,
        output: &mut String,
    ) -> CompileResult<String> {
        let (default, allowed, code) = options;
        if let Some(attribute) = self.attribute(event, name) {
            let avt = self.stylesheet.attribute_value_templates.iter().find(|a| {
                a.event_index == event.index && a.attribute_name == attribute.qualified_name
            });
            let dynamic = avt.is_some_and(|a| {
                a.segments
                    .iter()
                    .any(|s| !matches!(s, XsltAttributeValueTemplateSegmentAst::Literal { .. }))
            });
            if !dynamic && code != "XTDE1035" {
                let value = self.literal_attribute(event, attribute)?;
                let value = value.trim_matches([' ', '\t', '\r', '\n']);
                if !allowed.contains(&value) {
                    return Err(self.error(
                        event,
                        if code == "XTDE0030" {
                            "XTSE0020"
                        } else {
                            "cem.xslt.compile_unsupported"
                        },
                        format!("unsupported xsl:sort @{name}"),
                    ));
                }
            }
        }
        let query = self.sort_avt(event, name, scope, default)?;
        // Sort controls are tokens/recognized URIs; XML whitespace does not
        // include arbitrary Unicode spaces. Key text is never normalized here.
        let query = self.macro_program(
            event,
            "normalize-space($input)",
            scope,
            &[("input", &query)],
        )?;
        let binding = self.fresh();
        let check = allowed
            .iter()
            .map(|value| format!("{binding} == {}", quote(value)))
            .collect::<Vec<_>>()
            .join(" || ");
        let error = self.sort_error(event, code, &format!("unsupported xsl:sort @{name}"));
        output.push_str(&emit_variable(
            &binding,
            &format!(
                "{{ let {binding} = {query}; if {check} {{ {binding} }} else {{ {error} }} }}"
            ),
        ));
        Ok(binding)
    }

    /// Cache outer-focus controls and pre-sort keys, then process the original
    /// native values in sorted order. No document is reconstructed or serialized.
    pub(super) fn sorted<'a>(
        &mut self,
        node: &AuthorNode<'a>,
        scope: &Scope,
        select: String,
        grouped: bool,
    ) -> CompileResult<(String, String, Vec<AuthorNode<'a>>)> {
        let mut sorts = Vec::new();
        let mut body = Vec::new();
        let mut body_started = false;
        let apply = node.event.local_name.as_deref() == Some("apply-templates");
        for child in &node.children {
            if is_sort(child) {
                if body_started && !apply {
                    return Err(self.error(
                        child.event,
                        "cem.xslt.compile_content",
                        "xsl:sort must precede the sequence constructor",
                    ));
                }
                sorts.push(child);
            } else {
                body_started |= !ignorable(child);
                body.push(child.clone());
            }
        }
        if sorts.is_empty() {
            return Ok((String::new(), select, body));
        }
        let population = self.fresh();
        let mut output = emit_variable(&population, &select);
        let record = self.fresh();
        let mut record_scope = scope.clone();
        record_scope.item = record.clone();
        // This focus is used only to extract the cached native record.
        record_scope.position = "1".into();
        record_scope.size = "1".into();
        let item = self.macro_program(node.event, " .?(1)", &record_scope, &[])?;
        let position = self.macro_program(node.event, ".?(2)", &record_scope, &[])?;
        let size = self.macro_program(node.event, ".?(3)", &record_scope, &[])?;
        let key_scope = if grouped {
            Scope {
                item: self.macro_program(node.event, "head(.?(1)?(2))", &record_scope, &[])?,
                groups: Some([
                    "true".into(),
                    self.macro_program(node.event, ".?(1)?(2)", &record_scope, &[])?,
                    "true".into(),
                    self.macro_program(node.event, ".?(1)?(1)", &record_scope, &[])?,
                ]),
                position,
                size,
                ..scope.clone()
            }
        } else {
            Scope {
                item: item.clone(),
                position,
                size,
                ..scope.clone()
            }
        };
        let indexed = self.macro_program(
            node.event,
            "$population ! [., position(), last()]",
            scope,
            &[("population", &population)],
        )?;
        let mut key_bindings = String::new();
        let mut keys = Vec::new();
        let mut orders = Vec::new();
        for (index, sort) in sorts.iter().enumerate() {
            let event = sort.event;
            self.attributes(
                event,
                &["select", "order", "data-type", "stable", "collation"],
            )?;
            if sort.children.iter().any(|n| !ignorable(n)) {
                return Err(self.error(
                    event,
                    if self.attribute(event, "select").is_some() {
                        "XTSE1015"
                    } else {
                        "cem.xslt.compile_unsupported"
                    },
                    "this profile requires a select-based or empty sort key",
                ));
            }
            if index != 0 && self.attribute(event, "stable").is_some() {
                return Err(self.error(
                    event,
                    "XTSE1017",
                    "stable is allowed only on the first sort key",
                ));
            }
            orders.push(self.sort_control(
                event,
                "order",
                ("ascending", &["ascending", "descending"], "XTDE0030"),
                scope,
                &mut output,
            )?);
            let kind = if self.attribute(event, "data-type").is_some() {
                self.sort_control(
                    event,
                    "data-type",
                    (
                        "",
                        &["text", "number"],
                        "cem.xslt.sort_data_type_unsupported",
                    ),
                    scope,
                    &mut output,
                )?
            } else {
                quote("")
            };
            if self.attribute(event, "collation").is_some() {
                self.sort_control(
                    event,
                    "collation",
                    (CODEPOINT, &[CODEPOINT], "XTDE1035"),
                    scope,
                    &mut output,
                )?;
            }
            if self.attribute(event, "stable").is_some() {
                // A stable result is permitted when stable=no as well.
                self.sort_control(
                    event,
                    "stable",
                    ("yes", &["yes", "no", "true", "false", "1", "0"], "XTDE0030"),
                    scope,
                    &mut output,
                )?;
            }
            let expression = if self.attribute(event, "select").is_some() {
                self.expression(event, "select")?
            } else {
                self.generated_xpath(event, ".")?
            };
            let selected =
                self.program(event, expression, &key_scope, xpath::ResultKind::Sequence)?;
            let atomized =
                self.macro_program(event, "data($input)", scope, &[("input", &selected)])?;
            let key = self.fresh();
            let converted = self.macro_program(event,
                "if ($kind = 'text') then string($input) else if ($kind = 'number') then number($input) else $input ! (if (. instance of xs:untypedAtomic or . instance of xs:anyURI) then string(.) else .)",
                scope, &[("input", &key), ("kind", &kind)])?;
            let error = self.sort_error(
                event,
                "XTTE1020",
                "sort key must atomize to zero or one value",
            );
            key_bindings.push_str(&format!("let {key} = {{ let {key} = {atomized}; if seq:count({key}) > 1 {{ {error} }} else {{ {converted} }} }}; "));
            keys.push(key);
        }
        let mut bindings = vec![("item".to_owned(), item)];
        bindings.extend(
            keys.iter()
                .enumerate()
                .map(|(i, key)| (format!("key{i}"), key.clone())),
        );
        let constructor = format!(
            "[{}]",
            bindings
                .iter()
                .map(|(name, _)| format!("${name}"))
                .collect::<Vec<_>>()
                .join(",")
        );
        let bindings: Vec<_> = bindings
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let constructed = self.macro_program(node.event, &constructor, scope, &bindings)?;
        let records = self.fresh();
        output.push_str(&emit_variable(
            &records,
            &format!("seq:map({indexed}, fn({record}) => {{ {key_bindings}{constructed} }})"),
        ));
        // Stable minor-to-major passes implement mixed directions. Reverse on
        // BOTH sides of an ascending pass preserves equal-key order descending.
        let mut sorted = records;
        for (index, sort) in sorts.iter().enumerate().rev() {
            let column = (index + 2).to_string();
            let bindings = [
                ("records", sorted.as_str()),
                ("index", column.as_str()),
                ("order", orders[index].as_str()),
            ];
            let comparable = self.macro_program(sort.event, COMPARABLE, scope, &bindings)?;
            let select = self.macro_program(sort.event, SORT, scope, &bindings)?;
            let error = self.sort_error(
                sort.event,
                "XTDE1030",
                "sort keys must have comparable supported atomic types",
            );
            let next = self.fresh();
            output.push_str(&emit_variable(
                &next,
                &format!("if {comparable} {{ {select} }} else {{ {error} }}"),
            ));
            sorted = next;
        }
        let select = self.macro_program(
            node.event,
            "$records ! .?(1)",
            scope,
            &[("records", &sorted)],
        )?;
        Ok((output, select, body))
    }
}
