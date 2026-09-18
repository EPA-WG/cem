//! Bounded, typed stylesheet-to-CEMT lowering. This is an authoring boundary;
//! runtime documents never enter this compiler. XPath syntax stays XPath-owned.
use super::*;
use crate::{render::CompileTemplateOptions, template_artifact::compile_template_artifact};
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    schema::registry::XSLT_NAMESPACE_URI,
    validation::{
        xml::{xml_decode_entity_reference, XmlAttributeAst, XmlEventAst, XmlEventKind},
        xpath::XPathSyntaxNodeKind,
        xslt::{
            xslt_stylesheet_ast_from_source_bytes, XsltAttributeValueTemplateSegmentAst,
            XsltSourceValidationRequest, XsltStylesheetAst,
        },
    },
};

mod xpath;
type CompileResult<T> = std::result::Result<T, Vec<Diagnostic>>;
const MAX_SOURCE_BYTES: usize = 128 * 1024;

#[derive(Debug)]
pub struct CompiledXsltBundle {
    pub bytes: Vec<u8>,
    pub content_hash: ContentHash,
    pub source_hash: ContentHash,
    /// Generated code for inspection, never serialized runtime input/output.
    pub generated_cemt: String,
}

/// Compile one stylesheet with one `match="/"` template. The initial native
/// context is supplied through the `document` host binding. Dispatch, imports,
/// parameters, grouping, sorting and the full output profile are later slices.
pub fn compile_xslt_bundle(source: &str, source_uri: &str) -> CompileResult<CompiledXsltBundle> {
    if source.len() > MAX_SOURCE_BYTES || source_uri.len() > MAX_IDENTIFIER_BYTES - 32 {
        return Err(vec![Diagnostic {
            uri: Some(source_uri.into()),
            code: "cem.xslt.compile_limit".into(),
            severity: Severity::Error,
            message: "stylesheet source or URI exceeds the compiler limit".into(),
            ..Default::default()
        }]);
    }
    let (stylesheet, diagnostics) =
        xslt_stylesheet_ast_from_source_bytes(XsltSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri,
            content_type: Some("application/xslt+xml"),
        });
    if diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
        return Err(diagnostics);
    }
    let stylesheet = stylesheet.ok_or(diagnostics)?;
    let mut compiler = Compiler {
        stylesheet: &stylesheet,
        programs: Vec::new(),
        next_binding: 0,
        source_hash: ContentHash::from_blake3(source.as_bytes()),
    };
    let events = &stylesheet.xml_document.events;
    if events.len() > 8192 {
        return Err(compiler.error(
            &events[0],
            "cem.xslt.compile_limit",
            "stylesheet event limit exceeded",
        ));
    }
    let mut index = 0;
    let roots = author_nodes(events, &mut index, 0, &compiler)?;
    let roots: Vec<_> = roots.iter().filter(|node| is_element(node.event)).collect();
    let [root] = roots.as_slice() else {
        return Err(vec![Diagnostic {
            uri: Some(source_uri.into()),
            code: "cem.xslt.compile_root".into(),
            severity: Severity::Error,
            message: "one stylesheet root is required".into(),
            ..Default::default()
        }]);
    };
    compiler.attributes(root.event, &["version"])?;
    if compiler.required(root.event, "version")? != "3.0" {
        return Err(compiler.error(
            root.event,
            "cem.xslt.compile_unsupported",
            "this runtime profile requires XSLT version 3.0",
        ));
    }
    let declarations: Vec<_> = root
        .children
        .iter()
        .filter(|node| !ignorable(node))
        .collect();
    let [entry] = declarations.as_slice() else {
        return Err(compiler.error(root.event, "cem.xslt.compile_unsupported", "this profile requires exactly one root template; declarations and imports are not supported yet"));
    };
    if entry.event.namespace_uri.as_deref() != Some(XSLT_NAMESPACE_URI)
        || entry.event.local_name.as_deref() != Some("template")
    {
        return Err(compiler.error(
            entry.event,
            "cem.xslt.compile_unsupported",
            "only a root template is supported at stylesheet level",
        ));
    }
    compiler.attributes(entry.event, &["match"])?;
    if compiler.required(entry.event, "match")? != "/" {
        return Err(compiler.error(
            entry.event,
            "cem.xslt.compile_unsupported",
            "template dispatch is not implemented in this profile",
        ));
    }
    let scope = Scope {
        item: "document".into(),
        position: "1".into(),
        size: "1".into(),
        variables: BTreeMap::new(),
    };
    let body = compiler.sequence(&entry.children, scope)?;
    // Existing protected CEMT rendering buffers the whole result. An unmatched
    // handler preserves the original error and discards partial result nodes.
    let generated_cemt = format!("{{try | {body}{{catch @test=\"false\" | }}}}");
    let template = compile_template_artifact(
        &generated_cemt,
        &CompileTemplateOptions {
            host_bindings: vec!["document".into()],
            ..Default::default()
        },
        TemplateArtifactSourceMapMode::Dev,
    );
    let bytes = XsltBundle::compose(
        &template,
        BundleSource::new(
            format!("{source_uri}#generated-cemt"),
            generated_cemt.as_bytes(),
        ),
        &[StylesheetSource {
            source: BundleSource::new(source_uri, source.as_bytes()),
            dependencies: vec![],
        }],
        &compiler.programs,
    )
    .map_err(|error| compiler.error(root.event, "cem.xslt.compile_bundle", error.to_string()))?;
    Ok(CompiledXsltBundle {
        content_hash: ContentHash::from_blake3(&bytes),
        bytes,
        source_hash: compiler.source_hash,
        generated_cemt,
    })
}

struct AuthorNode<'a> {
    event: &'a XmlEventAst,
    children: Vec<AuthorNode<'a>>,
    text: Option<String>,
}
fn is_element(event: &XmlEventAst) -> bool {
    matches!(
        event.kind,
        XmlEventKind::StartElement | XmlEventKind::EmptyElement
    )
}
fn is_text(event: &XmlEventAst) -> bool {
    matches!(
        event.kind,
        XmlEventKind::Text | XmlEventKind::Cdata | XmlEventKind::EntityReference
    )
}
fn ignorable(node: &AuthorNode<'_>) -> bool {
    matches!(
        node.event.kind,
        XmlEventKind::Comment | XmlEventKind::ProcessingInstruction | XmlEventKind::Declaration
    ) || node.text.as_deref().is_some_and(|value| {
        value
            .chars()
            .all(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'))
    })
}
fn author_nodes<'a>(
    events: &'a [XmlEventAst],
    index: &mut usize,
    depth: usize,
    compiler: &Compiler<'_>,
) -> CompileResult<Vec<AuthorNode<'a>>> {
    let mut nodes = Vec::new();
    while let Some(event) = events.get(*index) {
        *index += 1;
        if depth > 64 {
            return Err(compiler.error(
                event,
                "cem.xslt.compile_limit",
                "stylesheet nesting exceeds 64 levels",
            ));
        }
        if event.kind == XmlEventKind::EndElement {
            break;
        }
        let children = if event.kind == XmlEventKind::StartElement {
            author_nodes(events, index, depth + 1, compiler)?
        } else {
            vec![]
        };
        // XML text, CDATA and entity-reference events form one stylesheet
        // text node. Strip whitespace only after joining that complete node.
        let value = if is_text(event) {
            let mut value = text(event);
            while let Some(next) = events.get(*index).filter(|next| is_text(next)) {
                value.push_str(&text(next));
                *index += 1;
            }
            Some(value)
        } else {
            None
        };
        nodes.push(AuthorNode {
            event,
            children,
            text: value,
        });
    }
    Ok(nodes)
}
fn text(event: &XmlEventAst) -> String {
    if event.kind == XmlEventKind::EntityReference {
        event
            .value
            .as_deref()
            .and_then(xml_decode_entity_reference)
            .map(|c| c.to_string())
            .unwrap_or_default()
    } else {
        event
            .value
            .as_deref()
            .unwrap_or_default()
            .replace("\r\n", "\n")
            .replace('\r', "\n")
    }
}
// CEM-QL scalar code literals. Escape CEMT delimiters too: its expression
// scanner counts braces, and its attribute scanner does not unescape quotes.
fn quote(value: &str) -> String {
    let mut output = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => output.push_str("\\\\"),
            '"' => output.push_str("\\\""),
            '\'' | '{' | '}' => output.push_str(&format!("\\u{{{:x}}}", ch as u32)),
            ch if ch.is_control() => output.push_str(&format!("\\u{{{:x}}}", ch as u32)),
            ch => output.push(ch),
        }
    }
    output.push('"');
    output
}
fn query_attribute(query: &str) -> String {
    debug_assert!(!query.contains('\''));
    format!("'{query}'")
}
fn emit_text(value: &str) -> String {
    format!("{{$ {}}}", quote(value))
}
fn emit_variable(name: &str, select: &str) -> String {
    format!(
        "{{variable @name={} @select={}}}",
        quote(name),
        query_attribute(select)
    )
}

#[derive(Clone)]
struct Scope {
    item: String,
    position: String,
    size: String,
    variables: BTreeMap<XPathExpandedName, String>,
}
struct Compiler<'a> {
    stylesheet: &'a XsltStylesheetAst,
    source_hash: ContentHash,
    programs: Vec<BundleProgram>,
    next_binding: usize,
}
impl Compiler<'_> {
    fn error(
        &self,
        event: &XmlEventAst,
        code: &str,
        message: impl Into<String>,
    ) -> Vec<Diagnostic> {
        let range = event.source_range;
        vec![Diagnostic {
            uri: Some(self.stylesheet.xml_document.source.uri.clone()),
            code: code.into(),
            severity: Severity::Error,
            message: message.into(),
            line: Some(range.start.line),
            column: Some(range.start.column),
            byte_offset: Some(range.start.byte_offset),
            source_map: Some(range.source_map()),
            ..Default::default()
        }]
    }
    fn fresh(&mut self) -> String {
        let name = format!("xslt_{}", self.next_binding);
        self.next_binding += 1;
        name
    }
    fn attribute<'a>(&self, event: &'a XmlEventAst, name: &str) -> Option<&'a XmlAttributeAst> {
        event
            .attributes
            .iter()
            .find(|a| a.namespace_uri.is_none() && a.local_name == name)
    }
    fn required<'a>(&self, event: &'a XmlEventAst, name: &str) -> CompileResult<&'a str> {
        self.attribute(event, name)
            .and_then(|a| a.entity_decoded_value.as_deref())
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| {
                self.error(
                    event,
                    "cem.xslt.compile_attribute",
                    format!(
                        "{} requires @{name}",
                        event.qualified_name.as_deref().unwrap_or("instruction")
                    ),
                )
            })
    }
    fn attributes(&self, event: &XmlEventAst, allowed: &[&str]) -> CompileResult<()> {
        for attribute in &event.attributes {
            if attribute.qualified_name == "xmlns" || attribute.prefix.as_deref() == Some("xmlns") {
                continue;
            }
            if attribute.namespace_uri.is_some()
                || !allowed.contains(&attribute.local_name.as_str())
            {
                return Err(self.error(
                    event,
                    "cem.xslt.compile_unsupported",
                    format!(
                        "unsupported attribute @{} on {}",
                        attribute.qualified_name,
                        event.qualified_name.as_deref().unwrap_or("instruction")
                    ),
                ));
            }
        }
        Ok(())
    }
    fn empty(&self, node: &AuthorNode<'_>) -> CompileResult<()> {
        if node.children.iter().any(|child| !ignorable(child)) {
            return Err(self.error(
                node.event,
                "cem.xslt.compile_content",
                "this select-based instruction requires empty content",
            ));
        }
        Ok(())
    }
    fn literal_attribute(
        &self,
        event: &XmlEventAst,
        attribute: &XmlAttributeAst,
    ) -> CompileResult<String> {
        if let Some(avt) =
            self.stylesheet.attribute_value_templates.iter().find(|a| {
                a.event_index == event.index && a.attribute_name == attribute.qualified_name
            })
        {
            let mut value = String::new();
            for segment in &avt.segments {
                if let XsltAttributeValueTemplateSegmentAst::Literal { effective, .. } = segment {
                    value.push_str(effective);
                } else {
                    return Err(self.error(
                        event,
                        "cem.xslt.compile_unsupported",
                        "dynamic attribute value templates belong to the output-profile fixture",
                    ));
                }
            }
            Ok(value)
        } else {
            attribute.entity_decoded_value.clone().ok_or_else(|| {
                self.error(
                    event,
                    "cem.xslt.compile_attribute",
                    "attribute has no decoded authoring value",
                )
            })
        }
    }
    fn sequence(&mut self, nodes: &[AuthorNode<'_>], mut scope: Scope) -> CompileResult<String> {
        let mut output = String::new();
        for node in nodes {
            if is_element(node.event)
                && node.event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
                && node.event.local_name.as_deref() == Some("variable")
            {
                self.attributes(node.event, &["name", "select"])?;
                self.empty(node)?;
                let lexical = self.required(node.event, "name")?;
                let selected = self.expression(node.event, "select")?;
                let name = xpath::variable_name(lexical, &selected).map_err(|message| {
                    self.error(node.event, "cem.xslt.compile_variable", message)
                })?;
                let call =
                    self.program(node.event, selected, &scope, xpath::ResultKind::Sequence)?;
                let binding = self.fresh();
                output.push_str(&emit_variable(&binding, &call));
                scope.variables.insert(name, binding);
            } else {
                output.push_str(&self.node(node, &scope)?);
            }
        }
        Ok(output)
    }
    fn node(&mut self, node: &AuthorNode<'_>, scope: &Scope) -> CompileResult<String> {
        let event = node.event;
        if ignorable(node) {
            return Ok(String::new());
        }
        if let Some(value) = &node.text {
            return Ok(emit_text(value));
        }
        if !is_element(event) {
            return Err(self.error(
                event,
                "cem.xslt.compile_unsupported",
                "unsupported stylesheet source event",
            ));
        }
        if event.namespace_uri.as_deref() != Some(XSLT_NAMESPACE_URI) {
            return self.literal(node, scope);
        }
        match event.local_name.as_deref().unwrap_or_default() {
            "for-each" => {
                self.attributes(event, &["select"])?;
                let select = self.program(
                    event,
                    self.expression(event, "select")?,
                    scope,
                    xpath::ResultKind::Sequence,
                )?;
                let sequence = self.fresh();
                let size = self.fresh();
                let item = self.fresh();
                let position = self.fresh();
                let inner = Scope {
                    item: item.clone(),
                    position: position.clone(),
                    size: size.clone(),
                    variables: scope.variables.clone(),
                };
                let body = self.sequence(&node.children, inner)?;
                Ok(format!(
                    "{}{}{{for-each @select={} @as={} | {}{body}}}",
                    emit_variable(&sequence, &select),
                    emit_variable(&size, &format!("seq:count({sequence})")),
                    quote(&sequence),
                    quote(&item),
                    emit_variable(&position, "position")
                ))
            }
            "if" => {
                self.attributes(event, &["test"])?;
                let test = self.program(
                    event,
                    self.expression(event, "test")?,
                    scope,
                    xpath::ResultKind::Boolean,
                )?;
                Ok(format!(
                    "{{if @test={} | {}}}",
                    query_attribute(&test),
                    self.sequence(&node.children, scope.clone())?
                ))
            }
            "choose" => self.choose(node, scope),
            "value-of" => {
                self.attributes(event, &["select", "separator"])?;
                self.empty(node)?;
                let separator = self
                    .attribute(event, "separator")
                    .map(|a| self.literal_attribute(event, a))
                    .transpose()?
                    .unwrap_or_else(|| " ".into());
                let select = self.program(
                    event,
                    self.expression(event, "select")?,
                    scope,
                    xpath::ResultKind::Text(separator),
                )?;
                Ok(format!("{{$ {select}}}"))
            }
            "text" => {
                self.attributes(event, &[])?;
                let mut value = String::new();
                for child in &node.children {
                    if matches!(
                        child.event.kind,
                        XmlEventKind::Comment | XmlEventKind::ProcessingInstruction
                    ) {
                        continue;
                    }
                    if !matches!(
                        child.event.kind,
                        XmlEventKind::Text | XmlEventKind::Cdata | XmlEventKind::EntityReference
                    ) {
                        return Err(self.error(
                            child.event,
                            "cem.xslt.compile_content",
                            "xsl:text requires text-only content",
                        ));
                    }
                    value.push_str(child.text.as_deref().unwrap_or_default());
                }
                Ok(emit_text(&value))
            }
            _ => Err(self.error(
                event,
                "cem.xslt.compile_unsupported",
                format!(
                    "{} is outside the current runtime lowering profile",
                    event.qualified_name.as_deref().unwrap_or("instruction")
                ),
            )),
        }
    }
    fn choose(&mut self, node: &AuthorNode<'_>, scope: &Scope) -> CompileResult<String> {
        self.attributes(node.event, &[])?;
        let mut output = String::from("{choose |");
        let mut when = false;
        let mut otherwise = false;
        for child in &node.children {
            if ignorable(child) {
                continue;
            }
            if child.event.namespace_uri.as_deref() != Some(XSLT_NAMESPACE_URI) || otherwise {
                return Err(self.error(
                    child.event,
                    "cem.xslt.compile_content",
                    "choose requires when branches followed by at most one otherwise",
                ));
            }
            match child.event.local_name.as_deref() {
                Some("when") => {
                    self.attributes(child.event, &["test"])?;
                    let test = self.program(
                        child.event,
                        self.expression(child.event, "test")?,
                        scope,
                        xpath::ResultKind::Boolean,
                    )?;
                    output.push_str(&format!(
                        "{{when @test={} | {}}}",
                        query_attribute(&test),
                        self.sequence(&child.children, scope.clone())?
                    ));
                    when = true;
                }
                Some("otherwise") if when => {
                    self.attributes(child.event, &[])?;
                    output.push_str(&format!(
                        "{{otherwise | {}}}",
                        self.sequence(&child.children, scope.clone())?
                    ));
                    otherwise = true;
                }
                _ => {
                    return Err(self.error(
                        child.event,
                        "cem.xslt.compile_content",
                        "choose requires at least one when before otherwise",
                    ))
                }
            }
        }
        if !when {
            return Err(self.error(
                node.event,
                "cem.xslt.compile_content",
                "choose requires at least one when",
            ));
        }
        output.push('}');
        Ok(output)
    }
    fn literal(&mut self, node: &AuthorNode<'_>, scope: &Scope) -> CompileResult<String> {
        let event = node.event;
        if event
            .namespace_uri
            .as_deref()
            .is_some_and(|uri| uri != "http://www.w3.org/1999/xhtml")
            || matches!(event.local_name.as_deref(), Some("script" | "style"))
        {
            return Err(self.error(
                event,
                "cem.xslt.compile_unsupported",
                "namespaced results and lexical islands belong to the output-profile fixture",
            ));
        }
        let mut output = format!(
            "{{element @name={} |",
            quote(event.local_name.as_deref().unwrap_or_default())
        );
        for attribute in &event.attributes {
            if attribute.qualified_name == "xmlns" || attribute.prefix.as_deref() == Some("xmlns") {
                continue;
            }
            if attribute.namespace_uri.is_some() {
                return Err(self.error(
                    event,
                    "cem.xslt.compile_unsupported",
                    "namespaced result attributes are not supported yet",
                ));
            }
            let value = self.literal_attribute(event, attribute)?;
            output.push_str(&format!(
                "{{attribute @name={} |{}}}",
                quote(&attribute.local_name),
                emit_text(&value)
            ));
        }
        output.push_str(&self.sequence(&node.children, scope.clone())?);
        output.push('}');
        Ok(output)
    }
    fn expression(
        &self,
        event: &XmlEventAst,
        attribute: &str,
    ) -> CompileResult<XPathExpressionAst> {
        self.required(event, attribute)?;
        self.stylesheet
            .xpath_expressions
            .iter()
            .find(|slot| slot.event_index == event.index && slot.attribute_name == attribute)
            .map(|slot| slot.expression.clone())
            .ok_or_else(|| {
                self.error(
                    event,
                    "cem.xslt.compile_xpath",
                    "instruction has no typed XPath slot",
                )
            })
    }
    fn program(
        &mut self,
        event: &XmlEventAst,
        mut expression: XPathExpressionAst,
        scope: &Scope,
        kind: xpath::ResultKind,
    ) -> CompileResult<String> {
        if self.programs.len() >= 128 {
            return Err(self.error(
                event,
                "cem.xslt.compile_limit",
                "stylesheet exceeds 128 XPath programs",
            ));
        }
        if expression.syntax_ast.as_ref().is_some_and(|syntax| {
            syntax.events.iter().any(|e| {
                matches!(
                    e.node_kind,
                    XPathSyntaxNodeKind::UnsupportedExpression
                        | XPathSyntaxNodeKind::UnsupportedPrimary
                )
            })
        }) {
            return Err(self.error(
                event,
                "cem.xslt.compile_unsupported",
                "unsupported XPath syntax",
            ));
        }
        let XPathAttachment::Host(host) = &mut expression.attachment else {
            return Err(self.error(
                event,
                "cem.xslt.compile_xpath",
                "XPath slot lost its stylesheet owner",
            ));
        };
        host.static_context.variable_bindings = scope
            .variables
            .keys()
            .map(|name| (xpath::expanded(name), "item()*".into()))
            .collect();
        xpath::adapt(&mut expression, kind)
            .map_err(|message| self.error(event, "cem.xslt.compile_xpath", message))?;
        let artifact = XPathCompiledArtifact::compile(&expression, self.source_hash.clone())
            .map_err(|error| self.error(event, "cem.xslt.compile_xpath", error.to_string()))?;
        let id = self.programs.len();
        let mut arguments = vec![
            quote(&format!("xslt.program.{id}")),
            scope.item.clone(),
            scope.position.clone(),
            scope.size.clone(),
        ];
        arguments.extend(scope.variables.values().cloned());
        self.programs.push(BundleProgram {
            stylesheet: 0,
            focus: BundleFocus::Sequence,
            variables: scope
                .variables
                .keys()
                .cloned()
                .map(|name| BundleVariable {
                    name,
                    kind: ParamType::Any,
                    nullable: true,
                })
                .collect(),
            artifact,
        });
        Ok(format!("native:call({})", arguments.join(", ")))
    }
}
