//! XSLT template/parameter semantics lowered onto existing CEMT calls.
use super::*;
use cem_ml::validation::xpath::*;

pub(super) const FOCUS_ITEM: &str = "xslt_context";
pub(super) const FOCUS_POSITION: &str = "xslt_position";
pub(super) const FOCUS_SIZE: &str = "xslt_size";
const FOCUS_MODE: &str = "xslt_mode";
const GROUP_BINDINGS: [&str; 4] = [
    "xslt_group_present",
    "xslt_group",
    "xslt_key_present",
    "xslt_key",
];

fn inherited_group() -> Option<[String; 4]> {
    Some(GROUP_BINDINGS.map(str::to_owned))
}
fn group_arguments(scope: &Scope) -> String {
    let absent = ["false".into(), "()".into(), "false".into(), "()".into()];
    GROUP_BINDINGS
        .iter()
        .zip(scope.groups.as_ref().unwrap_or(&absent))
        .map(|(name, value)| with(name, value))
        .collect()
}

#[derive(Clone)]
pub(super) struct Parameter<'a> {
    name: XPathExpandedName,
    value: String,
    supplied: String,
    required: bool,
    node: AuthorNode<'a>,
}
#[derive(Clone)]
pub(super) struct TemplateDeclaration<'a> {
    id: String,
    node: AuthorNode<'a>,
    params: Vec<Parameter<'a>>,
    patterns: Vec<(XPathExpressionAst, patterns::Priority)>,
    mode: String,
    source: usize,
    precedence: usize,
}

fn with(name: &str, expression: &str) -> String {
    format!(
        " @with:{name}={}",
        query_attribute(&format!("{{{expression}}}"))
    )
}

impl<'a> Compiler<'a> {
    fn name(&self, event: &XmlEventAst, attribute: &str) -> CompileResult<XPathExpandedName> {
        let expression = self.authored_context(event);
        xpath::variable_name(self.required(event, attribute)?, &expression)
            .map_err(|message| self.error(event, "cem.xslt.compile_name", message))
    }
    // A compiler-owned literal carries the authoring context for declarations
    // without XPath attributes. Namespace expansion still uses the XPath parser.
    pub(super) fn authored_context(&self, event: &XmlEventAst) -> XPathExpressionAst {
        let mut contexts = Vec::<XPathStaticContext>::new();
        let mut context = XPathStaticContext::default();
        for ancestor in self
            .stylesheet
            .xml_document
            .events
            .iter()
            .filter(|e| is_element(e) && e.index <= event.index)
        {
            contexts.truncate(ancestor.depth);
            context = contexts.last().cloned().unwrap_or_default();
            context
                .namespaces
                .insert("xml".into(), "http://www.w3.org/XML/1998/namespace".into());
            for attribute in &ancestor.attributes {
                if attribute.qualified_name == "xmlns"
                    || attribute.prefix.as_deref() == Some("xmlns")
                {
                    context.namespaces.insert(
                        if attribute.qualified_name == "xmlns" {
                            String::new()
                        } else {
                            attribute.local_name.clone()
                        },
                        attribute.entity_decoded_value.clone().unwrap_or_default(),
                    );
                }
            }
            if ancestor.kind == XmlEventKind::StartElement {
                contexts.push(context.clone());
            }
        }
        xpath_expression_ast_from_source_bytes(
            XPathSourceRequest {
                bytes: b"0",
                source_uri: &self.stylesheet.xml_document.source.uri,
                content_type: Some(XPATH_CONTENT_TYPE),
                source_range_projector: None,
            },
            XPathAttachment::StandaloneStaticContext {
                source_id: 1,
                static_context: context,
            },
        )
    }
    pub(super) fn templates(
        &mut self,
        root: &AuthorNode<'a>,
        linked: &[imports::Declaration<'a>],
        options: &XsltCompileOptions,
    ) -> CompileResult<String> {
        self.modes
            .insert(XPathExpandedName::unqualified(""), String::new());
        for source in 0..self.sources.len() {
            self.select_source(source);
            for event in
                self.stylesheet.xml_document.events.iter().filter(|e| {
                    is_element(e) && e.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
                })
            {
                if let Some(value) = self
                    .attribute(event, "mode")
                    .and_then(|a| a.entity_decoded_value.as_deref())
                {
                    if matches!(value, "#current" | "#default") {
                        continue;
                    }
                    let name = self.name(event, "mode")?;
                    if !self.modes.contains_key(&name) {
                        let id = self.fresh();
                        self.modes.insert(name, id);
                    }
                }
            }
        }
        for declaration in linked {
            self.select_source(declaration.source);
            let node = &declaration.node;
            if node.event.namespace_uri.as_deref() != Some(XSLT_NAMESPACE_URI)
                || node.event.local_name.as_deref() != Some("template")
            {
                return Err(self.error(
                    node.event,
                    "cem.xslt.compile_unsupported",
                    "only template declarations are supported in this dispatch slice",
                ));
            }
            self.attributes(node.event, &["name", "match", "mode", "priority"])?;
            let index = self.templates.len();
            let patterns = if self.attribute(node.event, "match").is_some() {
                self.patterns(node.event)?
            } else {
                Vec::new()
            };
            if patterns.is_empty()
                && (self.attribute(node.event, "mode").is_some()
                    || self.attribute(node.event, "priority").is_some())
            {
                return Err(self.error(
                    node.event,
                    "cem.xslt.compile_attribute",
                    "mode/priority require a match pattern",
                ));
            }
            let mode = self.mode(node.event, None)?;
            if self.attribute(node.event, "name").is_some() {
                let name = self.name(node.event, "name")?;
                if self.names.insert(name, index).is_some_and(|previous| {
                    self.templates[previous].precedence == declaration.precedence
                }) {
                    return Err(self.error(
                        node.event,
                        "cem.xslt.template_duplicate",
                        "duplicate named template",
                    ));
                }
            } else if self.attribute(node.event, "match").is_none() {
                return Err(self.error(
                    node.event,
                    "cem.xslt.compile_attribute",
                    "template requires @name or @match",
                ));
            }
            let mut params = Vec::new();
            let mut names = BTreeSet::new();
            let mut body_seen = false;
            for child in node.children.iter().filter(|node| !ignorable(node)) {
                if child.event.namespace_uri.as_deref() != Some(XSLT_NAMESPACE_URI)
                    || child.event.local_name.as_deref() != Some("param")
                {
                    body_seen = true;
                    continue;
                }
                if body_seen {
                    return Err(self.error(
                        child.event,
                        "cem.xslt.param_order",
                        "template parameters must precede its body",
                    ));
                }
                self.attributes(child.event, &["name", "select", "required"])?;
                self.empty(child)?;
                let name = self.name(child.event, "name")?;
                if !names.insert(name.clone()) {
                    return Err(self.error(
                        child.event,
                        "cem.xslt.param_duplicate",
                        "duplicate template parameter",
                    ));
                }
                let required = match self
                    .attribute(child.event, "required")
                    .and_then(|a| a.entity_decoded_value.as_deref())
                {
                    None | Some("no" | "false" | "0") => false,
                    Some("yes" | "true" | "1") => true,
                    _ => {
                        return Err(self.error(
                            child.event,
                            "cem.xslt.compile_attribute",
                            "invalid @required value",
                        ))
                    }
                };
                if required && self.attribute(child.event, "select").is_some() {
                    return Err(self.error(
                        child.event,
                        "cem.xslt.param_required",
                        "required parameter cannot declare a default",
                    ));
                }
                params.push(Parameter {
                    name,
                    value: self.fresh(),
                    supplied: self.fresh(),
                    required,
                    node: child.clone(),
                });
            }
            let id = self.fresh();
            self.templates.push(TemplateDeclaration {
                id,
                node: node.clone(),
                params,
                patterns,
                mode,
                source: declaration.source,
                precedence: declaration.precedence,
            });
        }
        self.select_source(0);
        let entry = options
            .entrypoint
            .as_ref()
            .map(|name| {
                self.names.get(name).copied().ok_or_else(|| {
                    self.error(
                        root.event,
                        "cem.transform_template.call_unknown",
                        format!(
                            "XSLT template entrypoint `{}` was not found",
                            xpath::expanded(name)
                        ),
                    )
                })
            })
            .transpose()?;
        let mut ranked = Vec::new();
        for (i, declaration) in self.templates.iter().enumerate() {
            for (j, (_, priority)) in declaration.patterns.iter().enumerate() {
                ranked.push((declaration.precedence, priority.clone(), i, j));
            }
        }
        ranked.sort();
        let ranks: BTreeMap<_, _> = ranked
            .into_iter()
            .enumerate()
            .map(|(rank, (_, _, i, j))| ((i, j), rank + 1))
            .collect();
        let mut declarations = String::new();
        for (i, declaration) in self.templates.clone().into_iter().enumerate() {
            self.select_source(declaration.source);
            let mut scope = Scope {
                item: FOCUS_ITEM.into(),
                position: FOCUS_POSITION.into(),
                size: FOCUS_SIZE.into(),
                mode: FOCUS_MODE.into(),
                groups: inherited_group(),
                variables: BTreeMap::new(),
            };
            let mut body = String::new();
            let mut bindings = vec![
                FOCUS_ITEM.to_owned(),
                FOCUS_POSITION.into(),
                FOCUS_SIZE.into(),
                FOCUS_MODE.into(),
            ];
            bindings.extend(GROUP_BINDINGS.map(str::to_owned));
            for param in &declaration.params {
                bindings.extend([param.value.clone(), param.supplied.clone()]);
                let default = if param.required {
                    format!(
                        "report:raise({}, {})",
                        quote("cem.xslt.param_required"),
                        quote(&format!(
                            "required XSLT parameter `{}` is missing",
                            xpath::expanded(&param.name)
                        ))
                    )
                } else if self.attribute(param.node.event, "select").is_some() {
                    self.program(
                        param.node.event,
                        self.expression(param.node.event, "select")?,
                        &scope,
                        xpath::ResultKind::Sequence,
                    )?
                } else {
                    quote("")
                };
                let binding = self.fresh();
                body.push_str(&emit_variable(
                    &binding,
                    &format!(
                        "if {} {{ {} }} else {{ {default} }}",
                        param.supplied, param.value
                    ),
                ));
                scope.variables.insert(param.name.clone(), binding);
            }
            let children: Vec<_> = declaration
                .node
                .children
                .iter()
                .filter(|child| {
                    !(child.event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
                        && child.event.local_name.as_deref() == Some("param"))
                })
                .cloned()
                .collect();
            body.push_str(&self.sequence(&children, scope)?);
            let parameters = bindings
                .iter()
                .map(|binding| format!("{{param @name={}}}", quote(binding)))
                .collect::<String>();
            declarations.push_str(&format!(
                "{{template @name={} |{parameters}{{body |{body}}}}}",
                quote(&declaration.id)
            ));
            for (j, (pattern, _)) in declaration.patterns.iter().enumerate() {
                let scope = Scope {
                    item: "node".into(),
                    position: FOCUS_POSITION.into(),
                    size: FOCUS_SIZE.into(),
                    mode: FOCUS_MODE.into(),
                    groups: None,
                    variables: BTreeMap::new(),
                };
                let test = self.program(
                    declaration.node.event,
                    pattern.clone(),
                    &scope,
                    xpath::ResultKind::Boolean,
                )?;
                declarations.push_str(&format!(
                    "{{template @match={} @mode={} @priority={} |{parameters}{{body |{body}}}}}",
                    query_attribute(&test),
                    quote(&declaration.mode),
                    ranks[&(i, j)]
                ));
            }
        }
        let scope = Scope {
            item: "document".into(),
            position: "1".into(),
            size: "1".into(),
            mode: quote(""),
            groups: None,
            variables: BTreeMap::new(),
        };
        self.select_source(0);
        declarations.push_str(&self.builtins(root.event)?);
        let entry_call = if let Some(entry) = entry {
            self.call(entry, &options.parameters, &scope, root.event)?
        } else if options.parameters.is_empty() {
            self.dispatch("document", &quote(""), &BTreeMap::new(), false, &scope)
        } else {
            return Err(self.error(
                root.event,
                "cem.xslt.param_unknown",
                "external parameters require an explicit named entrypoint in this profile",
            ));
        };
        let policy = self.result_policy(root.event);
        Ok(format!(
            "{{module |{declarations}{{body |{{try |{{result-document {policy} |{entry_call}}}{{catch @test=false |}}}}}}}}"
        ))
    }
    fn mode(&self, event: &XmlEventAst, current: Option<&str>) -> CompileResult<String> {
        match self
            .attribute(event, "mode")
            .and_then(|a| a.entity_decoded_value.as_deref())
        {
            None | Some("#default") => Ok(String::new()),
            Some("#current") => current.map(str::to_owned).ok_or_else(|| {
                self.error(
                    event,
                    "cem.xslt.mode_invalid",
                    "#current is only valid on apply-templates",
                )
            }),
            Some(_) => self
                .modes
                .get(&self.name(event, "mode")?)
                .cloned()
                .ok_or_else(|| self.error(event, "cem.xslt.mode_invalid", "unknown mode")),
        }
    }
    pub(super) fn apply_templates(
        &mut self,
        node: &AuthorNode<'_>,
        scope: &Scope,
    ) -> CompileResult<String> {
        self.attributes(node.event, &["select", "mode"])?;
        let expression = if self.attribute(node.event, "select").is_some() {
            self.expression(node.event, "select")?
        } else {
            self.generated_xpath(node.event, "child::node()")?
        };
        let select = self.program(node.event, expression, scope, xpath::ResultKind::Sequence)?;
        let (sorting, select, children) = self.sorted(node, scope, select, false)?;
        let current = self
            .attribute(node.event, "mode")
            .and_then(|a| a.entity_decoded_value.as_deref())
            == Some("#current");
        let mode = if current {
            scope.mode.clone()
        } else {
            quote(&self.mode(node.event, None)?)
        };
        let arguments = self.arguments(
            &AuthorNode {
                children,
                ..node.clone()
            },
            scope,
        )?;
        Ok(sorting + &self.dispatch(&select, &mode, &arguments, false, scope))
    }
    fn dispatch(
        &mut self,
        select: &str,
        mode: &str,
        arguments: &BTreeMap<XPathExpandedName, String>,
        forward: bool,
        scope: &Scope,
    ) -> String {
        let sequence = self.fresh();
        let size = self.fresh();
        let item = self.fresh();
        let position = self.fresh();
        let mut output = emit_variable(&sequence, select);
        output.push_str(&emit_variable(&size, &format!("seq:count({sequence})")));
        let mut cached = BTreeMap::new();
        for (name, query) in arguments {
            let id = self.fresh();
            output.push_str(&emit_variable(&id, query));
            cached.insert(name, id);
        }
        let mut attributes = with(FOCUS_ITEM, &item)
            + &with(FOCUS_POSITION, &position)
            + &with(FOCUS_SIZE, &size)
            + &with(FOCUS_MODE, mode);
        attributes.push_str(&group_arguments(scope));
        for template in &self.templates {
            for param in &template.params {
                let argument = cached.get(&param.name);
                attributes.push_str(&with(
                    &param.supplied,
                    if forward {
                        &param.supplied
                    } else if argument.is_some() {
                        "true"
                    } else {
                        "false"
                    },
                ));
                attributes.push_str(&with(
                    &param.value,
                    if forward {
                        &param.value
                    } else {
                        argument.map(String::as_str).unwrap_or("()")
                    },
                ));
            }
        }
        output.push_str(&format!("{{for-each @select={} @as={} |{}{{apply-templates @select={} @mode={} {attributes}}}}}", quote(&sequence), quote(&item), emit_variable(&position, "position"), query_attribute(&item), query_attribute(&format!("{{{mode}}}"))));
        output
    }
    fn builtins(&mut self, event: &XmlEventAst) -> CompileResult<String> {
        let scope = Scope {
            item: FOCUS_ITEM.into(),
            position: FOCUS_POSITION.into(),
            size: FOCUS_SIZE.into(),
            mode: FOCUS_MODE.into(),
            groups: inherited_group(),
            variables: BTreeMap::new(),
        };
        let mut program =
            |code, kind| self.program(event, self.generated_xpath(event, code)?, &scope, kind);
        let parent = program(
            ". instance of document-node() or . instance of element()",
            xpath::ResultKind::Boolean,
        )?;
        let children = program("child::node()", xpath::ResultKind::Sequence)?;
        let text = program(
            ". instance of text() or . instance of attribute()",
            xpath::ResultKind::Boolean,
        )?;
        let value = program("string(.)", xpath::ResultKind::Sequence)?;
        let other_node = program(". instance of node()", xpath::ResultKind::Boolean)?;
        let recursion = self.dispatch(&children, FOCUS_MODE, &BTreeMap::new(), true, &scope);
        let body = format!("{{choose |{{when @test={} |{recursion}}}{{when @test={} |{{$ {value}}}}}{{when @test={} |}}{{otherwise |{{$report:raise(\"cem.xslt.dispatch_unsupported\", \"this profile supports node dispatch\")}}}}}}", query_attribute(&parent), query_attribute(&text), query_attribute(&other_node));
        let mut bindings = vec![
            FOCUS_ITEM.to_owned(),
            FOCUS_POSITION.into(),
            FOCUS_SIZE.into(),
            FOCUS_MODE.into(),
        ];
        bindings.extend(GROUP_BINDINGS.map(str::to_owned));
        for template in &self.templates {
            for param in &template.params {
                bindings.extend([param.value.clone(), param.supplied.clone()]);
            }
        }
        let parameters = bindings
            .iter()
            .map(|binding| format!("{{param @name={}}}", quote(binding)))
            .collect::<String>();
        Ok(self
            .modes
            .values()
            .map(|mode| {
                format!(
                    "{{template @match=true @mode={} @priority=-1 |{parameters}{{body |{body}}}}}",
                    quote(mode)
                )
            })
            .collect())
    }
    fn arguments(
        &mut self,
        node: &AuthorNode<'_>,
        scope: &Scope,
    ) -> CompileResult<BTreeMap<XPathExpandedName, String>> {
        let mut arguments = BTreeMap::new();
        for child in node.children.iter().filter(|node| !ignorable(node)) {
            if child.event.namespace_uri.as_deref() != Some(XSLT_NAMESPACE_URI)
                || child.event.local_name.as_deref() != Some("with-param")
            {
                return Err(self.error(
                    child.event,
                    "cem.xslt.compile_content",
                    "template invocation accepts only with-param children",
                ));
            }
            self.attributes(child.event, &["name", "select"])?;
            self.empty(child)?;
            let name = self.name(child.event, "name")?;
            let select = self.program(
                child.event,
                self.expression(child.event, "select")?,
                scope,
                xpath::ResultKind::Sequence,
            )?;
            if arguments.insert(name, select).is_some() {
                return Err(self.error(
                    child.event,
                    "cem.xslt.param_duplicate",
                    "duplicate supplied parameter",
                ));
            }
        }
        Ok(arguments)
    }
    pub(super) fn named_call(
        &mut self,
        node: &AuthorNode<'_>,
        scope: &Scope,
    ) -> CompileResult<String> {
        self.attributes(node.event, &["name"])?;
        let name = self.name(node.event, "name")?;
        let target = self.names.get(&name).copied().ok_or_else(|| {
            self.error(
                node.event,
                "cem.transform_template.call_unknown",
                format!("unknown template `{}`", xpath::expanded(&name)),
            )
        })?;
        let arguments = self.arguments(node, scope)?;
        self.call(target, &arguments, scope, node.event)
    }
    fn call(
        &self,
        target: usize,
        arguments: &BTreeMap<XPathExpandedName, String>,
        scope: &Scope,
        event: &XmlEventAst,
    ) -> CompileResult<String> {
        let declaration = &self.templates[target];
        if arguments
            .keys()
            .any(|name| !declaration.params.iter().any(|param| &param.name == name))
        {
            return Err(self.error(
                event,
                "cem.xslt.param_unknown",
                "supplied parameter is not declared by the named template",
            ));
        }
        let mut output = format!(
            "{{call @template={}{}{}{}",
            quote(&declaration.id),
            with(FOCUS_ITEM, &scope.item),
            with(FOCUS_POSITION, &scope.position),
            with(FOCUS_SIZE, &scope.size)
        );
        output.push_str(&with(FOCUS_MODE, &scope.mode));
        output.push_str(&group_arguments(scope));
        for param in &declaration.params {
            let argument = arguments.get(&param.name);
            if param.required && argument.is_none() {
                return Err(self.error(
                    event,
                    "cem.xslt.param_required",
                    format!(
                        "required XSLT parameter `{}` is missing",
                        xpath::expanded(&param.name)
                    ),
                ));
            }
            output.push_str(&with(
                &param.supplied,
                if argument.is_some() { "true" } else { "false" },
            ));
            output.push_str(&with(
                &param.value,
                argument.map(String::as_str).unwrap_or("()"),
            ));
        }
        output.push('}');
        Ok(output)
    }
}
