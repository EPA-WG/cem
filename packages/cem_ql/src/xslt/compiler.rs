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
            xslt_stylesheet_ast_from_source_bytes_with_modules,
            XsltAttributeValueTemplateSegmentAst, XsltSourceValidationRequest, XsltStylesheetAst,
        },
    },
};

mod grouping;
mod imports;
mod output;
mod patterns;
mod recovery;
mod sorting;
mod templates;
mod variables;
mod xpath;
use templates::TemplateDeclaration;
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

#[derive(Debug, Default)]
pub struct XsltCompileOptions {
    pub entrypoint: Option<XPathExpandedName>,
    /// Expanded XSLT parameter name to explicit host binding identifier.
    pub parameters: BTreeMap<XPathExpandedName, String>,
    pub modules: Vec<XsltModuleSource>,
}

/// Resolver-preflighted authoring source. The compiler performs no I/O and
/// resolves a declaration only through its exact parent/href edge.
#[derive(Debug, Clone)]
pub struct XsltModuleSource {
    pub parent_uri: String,
    pub href: String,
    pub uri: String,
    pub source: String,
    pub content_hash: ContentHash,
}

/// Compile a stylesheet with default entry selection.
pub fn compile_xslt_bundle(source: &str, source_uri: &str) -> CompileResult<CompiledXsltBundle> {
    compile_xslt_bundle_with_options(source, source_uri, &XsltCompileOptions::default())
}

/// Import/include edges from typed authoring XML, for host resolver preflight.
/// No source document or parser projection crosses this control boundary.
pub fn stylesheet_imports(source: &str, source_uri: &str) -> CompileResult<Vec<String>> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(imports::diagnostic(source_uri, "stylesheet source exceeds limit"));
    }
    let (stylesheet, diagnostics) = xslt_stylesheet_ast_from_source_bytes_with_modules(
        XsltSourceValidationRequest { bytes: source.as_bytes(), source_uri,
            content_type: Some("application/xslt+xml") }, &[]);
    let stylesheet = stylesheet.ok_or(diagnostics)?;
    Ok(stylesheet.xml_document.events.iter()
        .filter(|event| is_element(event) && event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
            && matches!(event.local_name.as_deref(), Some("import" | "include")))
        .filter_map(|event| event.attributes.iter().find(|attr| attr.local_name == "href"
            && attr.namespace_uri.is_none()).and_then(|attr| attr.entity_decoded_value.clone()))
        .collect())
}

/// Compile the bounded template-dispatch profile. The initial native context
/// is supplied through `document`; module sources are preflighted by the host.
pub fn compile_xslt_bundle_with_options(
    source: &str,
    source_uri: &str,
    options: &XsltCompileOptions,
) -> CompileResult<CompiledXsltBundle> {
    if options.parameters.len() > 250 {
        return Err(imports::diagnostic(
            source_uri,
            "parameter binding limit exceeded",
        ));
    }
    for binding in options.parameters.values() {
        if binding.len() > MAX_IDENTIFIER_BYTES {
            return Err(imports::diagnostic(
                source_uri,
                "parameter binding exceeds identifier limit",
            ));
        }
        let tokens = crate::lexer::Lexer::new(binding).scan_all();
        if !matches!(tokens.as_slice(), [token, end] if token.kind == crate::lexer::TokenKind::Ident && end.kind == crate::lexer::TokenKind::EndOfInput)
            || binding.starts_with("xslt_")
            || binding == "document"
        {
            return Err(imports::diagnostic(
                source_uri,
                "parameter host bindings must be identifiers outside reserved document/xslt_ names",
            ));
        }
    }
    if source.len() > MAX_SOURCE_BYTES || source_uri.len() > MAX_IDENTIFIER_BYTES - 32 {
        return Err(vec![Diagnostic {
            uri: Some(source_uri.into()),
            code: "cem.xslt.compile_limit".into(),
            severity: Severity::Error,
            message: "stylesheet source or URI exceeds the compiler limit".into(),
            ..Default::default()
        }]);
    }
    let mut sources = BTreeMap::new();
    sources.insert(
        source_uri.to_owned(),
        (
            source.to_owned(),
            ContentHash::from_blake3(source.as_bytes()),
        ),
    );
    if options.modules.len() > 128 {
        return Err(imports::diagnostic(
            source_uri,
            "stylesheet dependency limit exceeded",
        ));
    }
    for module in &options.modules {
        if module.source.len() > MAX_SOURCE_BYTES
            || [&module.uri, &module.parent_uri, &module.href]
                .iter()
                .any(|value| value.len() > MAX_IDENTIFIER_BYTES - 32)
        {
            return Err(imports::diagnostic(
                &module.uri,
                "stylesheet source or dependency identifier exceeds limits",
            ));
        }
        if ContentHash::from_blake3(module.source.as_bytes()) != module.content_hash {
            return Err(imports::diagnostic(
                &module.uri,
                "preflighted stylesheet hash mismatch",
            ));
        }
        if let Some((previous, _)) = sources.get(&module.uri) {
            if previous != &module.source {
                return Err(imports::diagnostic(
                    &module.uri,
                    "conflicting source for stylesheet URI",
                ));
            }
        } else {
            sources.insert(
                module.uri.clone(),
                (module.source.clone(), module.content_hash.clone()),
            );
        }
    }
    if sources.len() > 64
        || sources.values().map(|(s, _)| s.len()).sum::<usize>() > MAX_SOURCE_BYTES
    {
        return Err(imports::diagnostic(
            source_uri,
            "stylesheet closure exceeds source limits",
        ));
    }
    let root_source = sources.remove(source_uri).expect("root inserted");
    let sources: Vec<_> = std::iter::once((source_uri.to_owned(), root_source))
        .chain(sources)
        .collect();
    let mut stylesheets = Vec::new();
    let mut manifests = Vec::new();
    let mut hashes = Vec::new();
    for (uri, (source, hash)) in &sources {
        if uri.len() > MAX_IDENTIFIER_BYTES - 32 {
            return Err(imports::diagnostic(
                uri,
                "stylesheet URI exceeds the compiler limit",
            ));
        }
        let hrefs: Vec<_> = options
            .modules
            .iter()
            .filter(|module| &module.parent_uri == uri)
            .map(|module| module.href.as_str())
            .collect();
        let (stylesheet, diagnostics) = xslt_stylesheet_ast_from_source_bytes_with_modules(
            XsltSourceValidationRequest {
                bytes: source.as_bytes(),
                source_uri: uri,
                content_type: Some("application/xslt+xml"),
            },
            &hrefs,
        );
        if diagnostics.iter().any(|d| d.severity.is_hard_violation()) {
            return Err(diagnostics);
        }
        let stylesheet = stylesheet
            .ok_or_else(|| imports::diagnostic(uri, "stylesheet source could not be parsed"))?;
        if stylesheet.xml_document.events.len() > 8192 {
            return Err(imports::diagnostic(uri, "stylesheet event limit exceeded"));
        }
        stylesheets.push(stylesheet);
        hashes.push(hash.clone());
        manifests.push(StylesheetSource {
            source: BundleSource::new(uri, source.as_bytes()),
            dependencies: vec![],
        });
    }
    let mut compiler = Compiler {
        stylesheet: &stylesheets[0],
        source_hash: hashes[0].clone(),
        programs: Vec::new(),
        next_binding: 0,
        templates: Vec::new(),
        names: BTreeMap::new(),
        modes: BTreeMap::new(),
        sources: &stylesheets,
        source_hashes: &hashes,
        source_index: 0,
    };
    let mut roots = Vec::new();
    for (index, stylesheet) in stylesheets.iter().enumerate() {
        compiler.select_source(index);
        let mut cursor = 0;
        let nodes = author_nodes(
            &stylesheet.xml_document.events,
            &mut cursor,
            0,
            false,
            &compiler,
        )?;
        let mut elements = nodes.into_iter().filter(|node| is_element(node.event));
        let root = elements.next().ok_or_else(|| {
            imports::diagnostic(
                &stylesheet.xml_document.source.uri,
                "one stylesheet root is required",
            )
        })?;
        if elements.next().is_some() {
            return Err(imports::diagnostic(
                &stylesheet.xml_document.source.uri,
                "one stylesheet root is required",
            ));
        }
        compiler.attributes(root.event, &["version"])?;
        if compiler.required(root.event, "version")? != "3.0" {
            return Err(compiler.error(
                root.event,
                "cem.xslt.compile_unsupported",
                "this runtime profile requires XSLT version 3.0",
            ));
        }
        roots.push(root);
    }
    let declarations = imports::link(&mut compiler, &roots, &options.modules, &mut manifests)?;
    compiler.select_source(0);
    let generated_cemt = compiler.templates(&roots[0], &declarations, options)?;
    let template = compile_template_artifact(
        &generated_cemt,
        &CompileTemplateOptions {
            host_bindings: std::iter::once("document".into())
                .chain(options.parameters.values().cloned())
                .collect(),
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
        &manifests,
        &compiler.programs,
    )
    .map_err(|error| {
        compiler.error(roots[0].event, "cem.xslt.compile_bundle", error.to_string())
    })?;
    Ok(CompiledXsltBundle {
        content_hash: ContentHash::from_blake3(&bytes),
        bytes,
        source_hash: hashes[0].clone(),
        generated_cemt,
    })
}

/// Resolve host-selected template/parameter names in the principal stylesheet's
/// namespace context. This parses authoring declarations, never runtime data.
pub fn resolve_xslt_names(
    source: &str,
    source_uri: &str,
    names: &[&str],
) -> CompileResult<Vec<XPathExpandedName>> {
    use cem_ml::validation::xpath::*;
    if source.len() > MAX_SOURCE_BYTES
        || source_uri.len() > MAX_IDENTIFIER_BYTES - 32
        || names.len() > 251
        || names.iter().any(|name| name.len() > MAX_IDENTIFIER_BYTES)
    {
        return Err(imports::diagnostic(
            source_uri,
            "stylesheet name resolution exceeds compiler limits",
        ));
    }
    let (stylesheet, diagnostics) = xslt_stylesheet_ast_from_source_bytes_with_modules(
        XsltSourceValidationRequest {
            bytes: source.as_bytes(),
            source_uri,
            content_type: Some("application/xslt+xml"),
        },
        &[],
    );
    let stylesheet = stylesheet.ok_or_else(|| {
        if diagnostics.is_empty() {
            imports::diagnostic(source_uri, "stylesheet source could not be parsed")
        } else {
            diagnostics
        }
    })?;
    let root = stylesheet
        .xml_document
        .events
        .iter()
        .find(|event| is_element(event))
        .ok_or_else(|| imports::diagnostic(source_uri, "stylesheet root missing"))?;
    let mut context = XPathStaticContext::default();
    context
        .namespaces
        .insert("xml".into(), "http://www.w3.org/XML/1998/namespace".into());
    for attribute in &root.attributes {
        if attribute.prefix.as_deref() == Some("xmlns") {
            context.namespaces.insert(
                attribute.local_name.clone(),
                attribute.entity_decoded_value.clone().unwrap_or_default(),
            );
        }
    }
    let expression = xpath_expression_ast_from_source_bytes(
        XPathSourceRequest {
            bytes: b"0",
            source_uri,
            content_type: Some(XPATH_CONTENT_TYPE),
            source_range_projector: None,
        },
        XPathAttachment::StandaloneStaticContext {
            source_id: 1,
            static_context: context,
        },
    );
    names
        .iter()
        .map(|name| {
            xpath::variable_name(name, &expression).map_err(|message| {
                vec![Diagnostic {
                    uri: Some(source_uri.into()),
                    code: "cem.xslt.compile_name".into(),
                    severity: Severity::Error,
                    message,
                    source_map: Some(root.source_range.source_map()),
                    ..Default::default()
                }]
            })
        })
        .collect()
}

#[derive(Clone)]
struct AuthorNode<'a> {
    event: &'a XmlEventAst,
    children: Vec<AuthorNode<'a>>,
    text: Option<String>,
    preserve_whitespace: bool,
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
    ) || !node.preserve_whitespace
        && node.text.as_deref().is_some_and(|value| {
            value
                .chars()
                .all(|ch| matches!(ch, ' ' | '\t' | '\r' | '\n'))
        })
}
fn author_nodes<'a>(
    events: &'a [XmlEventAst],
    index: &mut usize,
    depth: usize,
    preserve_whitespace: bool,
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
        let preserve = if let Some(attribute) = event.attributes.iter().find(|a| {
            a.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
                && a.local_name == "space"
        }) {
            match attribute.entity_decoded_value.as_deref() {
                Some("preserve") => true,
                Some("default") => false,
                _ => {
                    return Err(compiler.error(
                        event,
                        "XTSE0020",
                        "xml:space requires preserve or default",
                    ))
                }
            }
        } else {
            preserve_whitespace
        };
        let children = if event.kind == XmlEventKind::StartElement {
            author_nodes(events, index, depth + 1, preserve, compiler)?
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
            preserve_whitespace: preserve,
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
    mode: String,
    groups: Option<[String; 4]>,
    variables: BTreeMap<XPathExpandedName, String>,
}
struct Compiler<'a> {
    stylesheet: &'a XsltStylesheetAst,
    source_hash: ContentHash,
    programs: Vec<BundleProgram>,
    next_binding: usize,
    templates: Vec<TemplateDeclaration<'a>>,
    names: BTreeMap<XPathExpandedName, usize>,
    modes: BTreeMap<XPathExpandedName, String>,
    sources: &'a [XsltStylesheetAst],
    source_hashes: &'a [ContentHash],
    source_index: usize,
}
impl<'a> Compiler<'a> {
    fn select_source(&mut self, index: usize) {
        self.source_index = index;
        self.stylesheet = &self.sources[index];
        self.source_hash = self.source_hashes[index].clone();
    }
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
    fn attribute<'b>(&self, event: &'b XmlEventAst, name: &str) -> Option<&'b XmlAttributeAst> {
        event
            .attributes
            .iter()
            .find(|a| a.namespace_uri.is_none() && a.local_name == name)
    }
    fn required<'b>(&self, event: &'b XmlEventAst, name: &str) -> CompileResult<&'b str> {
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
            if attribute.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
                && attribute.local_name == "space"
            {
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
                let (sorting, select, children) = self.sorted(node, scope, select, false)?;
                let sequence = self.fresh();
                let size = self.fresh();
                let item = self.fresh();
                let position = self.fresh();
                let inner = Scope {
                    item: item.clone(),
                    position: position.clone(),
                    size: size.clone(),
                    mode: scope.mode.clone(),
                    groups: scope.groups.clone(),
                    variables: scope.variables.clone(),
                };
                let body = self.sequence(&children, inner)?;
                Ok(format!(
                    "{sorting}{}{}{{for-each @select={} @as={} | {}{body}}}",
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
            "sequence" | "copy-of" => {
                self.attributes(event, &["select"])?;
                self.empty(node)?;
                self.result_select(event, scope)
            }
            "element" | "attribute" | "document" => self.result_constructor(node, scope),
            "choose" => self.choose(node, scope),
            "try" => self.recover(node, scope),
            "for-each-group" => self.grouping(node, scope),
            "call-template" => self.named_call(node, scope),
            "apply-templates" => self.apply_templates(node, scope),
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
        let referenced = expression
            .syntax_ast
            .as_ref()
            .map(|syntax| variables::referenced(&syntax.root))
            .unwrap_or_default();
        let bindings: BTreeMap<_, _> = scope
            .variables
            .iter()
            .filter(|(name, _)| referenced.contains(*name))
            .collect();
        let XPathAttachment::Host(host) = &mut expression.attachment else {
            return Err(self.error(
                event,
                "cem.xslt.compile_xpath",
                "XPath slot lost its stylesheet owner",
            ));
        };
        host.static_context.variable_bindings = bindings
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
        let groups = scope.groups.as_ref().filter(|_| {
            expression
                .syntax_ast
                .as_ref()
                .is_some_and(|syntax| grouping::group_function(&syntax.root).is_some())
        });
        if let Some(groups) = groups {
            arguments.extend(groups.iter().cloned());
        }
        arguments.extend(bindings.values().map(|value| (*value).clone()));
        self.programs.push(BundleProgram {
            stylesheet: self.source_index,
            focus: BundleFocus::Sequence,
            group_context: groups.is_some(),
            variables: bindings
                .keys()
                .map(|name| (*name).clone())
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
