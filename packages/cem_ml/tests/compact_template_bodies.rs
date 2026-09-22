//! CEMT-COMPACT-TEMPLATES / CEMT-IMPLICIT-MODULES: shared module preflight.
use cem_ml::{
    engine::TemplateInput,
    transform_template::{
        parse_cem_native_template_module_options, TransformTemplateModuleParseRequest,
        TransformTemplateModuleParseResponse, TransformTemplateModuleVisibility,
    },
};

fn parse(source: &str) -> TransformTemplateModuleParseResponse {
    parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
        template: TemplateInput {
            uri: "memory:compact.cemt".into(),
            bytes: source.as_bytes().to_vec(),
            identity: None,
            root_scope: Default::default(),
        },
    })
}

#[test]
fn compact_and_wrapped_bodies_collect_the_same_module_contract() {
    let content = r#"{let @name=title @value=hello}
        {cem:variable @name=label @select=node}
        {section | {call @template=helper} {call @from=ui @template=icon}}
        {$ encode($node.data, { contentType: "text/html", schema: "https://cem.dev/ns/data/html/1", category: "html-text" }) }"#;
    let module = |body: &str| {
        format!(
            r#"{{module |
        {{import @as=ui @src="./ui.cemt"}}
        {{template @name=helper | {{i | helper}}}}
        {{template @name=card @visibility=public @mode=cell @match=true |
            {{param @name=label @type=string @required=true}} {body}}}}}"#
        )
    };
    let compact = parse(&module(content));
    let wrapped = parse(&module(&format!("{{body | {content}}}")));
    assert!(compact.diagnostics.is_empty(), "{:?}", compact.diagnostics);
    assert!(wrapped.diagnostics.is_empty(), "{:?}", wrapped.diagnostics);
    assert_eq!(compact.module_options, wrapped.module_options);
    let options = compact.module_options;
    assert_eq!(
        options.entrypoints[1].visibility,
        TransformTemplateModuleVisibility::Public
    );
    assert_eq!(options.params[0].name, "card.label");
    assert_eq!(options.calls.len(), 2);
    assert_eq!(options.calls[0].owner_entrypoint.as_deref(), Some("card"));
    assert_eq!(
        options.let_bindings[0].owner_entrypoint.as_deref(),
        Some("card")
    );
    assert_eq!(options.encode_expressions[0].owner.as_deref(), Some("card"));
}

#[test]
fn compact_text_empty_and_expression_hook_bodies_are_valid() {
    for body in ["", "text", "```{literal}```", "{$node}", "{span | text}"] {
        let source = format!("{{module | {{template @name=label | {body}}}}}");
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "{source}: {:?}",
            parsed.diagnostics
        );
    }
    let parsed = parse("{module | {template @on=expression @into=content | {$dom:text()}}}");
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(parsed.module_options.entrypoints.is_empty());
}

#[test]
fn explicit_schemas_accept_compact_templates_without_relaxing_function_bodies() {
    use cem_ml::{
        engine::FormatIdentity, schema::registry::CEM_TRANSFORM_SCHEMA_URI,
        transform_template::CEM_NATIVE_TEMPLATE_SCHEMA_URI,
    };
    let parse_schema = |schema: &str, content: &str| {
        parse_cem_native_template_module_options(TransformTemplateModuleParseRequest {
            template: TemplateInput {
                uri: "memory:typed.cemt".into(),
                bytes: format!(
                    "@doc cem-ml 1\n@ns t = \"{schema}\"\n@default t\n{{module | {content}}}"
                )
                .into_bytes(),
                identity: Some(FormatIdentity {
                    schema: Some(schema.into()),
                    ..Default::default()
                }),
                root_scope: Default::default(),
            },
        })
    };
    for schema in [CEM_NATIVE_TEMPLATE_SCHEMA_URI, CEM_TRANSFORM_SCHEMA_URI] {
        let parsed = parse_schema(
            schema,
            "{template @name=label | {param @name=title} {p | {$title}}}",
        );
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert_eq!(parsed.module_options.params[0].name, "label.title");
    }
    let function = parse_schema(
        CEM_TRANSFORM_SCHEMA_URI,
        "{function @name=label @returns=string | {$'text'}}",
    );
    assert!(
        !function.diagnostics.is_empty(),
        "function syntax is unchanged"
    );
}

#[test]
fn mixed_and_duplicate_template_bodies_are_rejected() {
    for body in [
        "{body | wrapped}{span | direct}",
        "text{body | wrapped}",
        "{body | wrapped}{$node}",
        "{body | first}{body | second}",
        "```literal```{body | wrapped}",
        "{body | wrapped}// output comment\n",
    ] {
        let source = format!("{{module | {{template @name=label | {body}}}}}");
        let parsed = parse(&source);
        assert!(
            parsed
                .diagnostics
                .iter()
                .any(|d| d.code == "cem.transform_template.declaration_invalid"),
            "{source}: {:?}",
            parsed.diagnostics
        );
    }
    let parsed = parse(
        "{module | {template @name=label |\n {param @name=node}\n {body | {span | {$node}}}\n}}",
    );
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
}

#[test]
fn compact_calls_keep_validation_and_original_source_positions() {
    let source = "{module |\n {template @name=label |\n  {span | {call @template=missing}}}}";
    let parsed = parse(source);
    let diagnostic = parsed
        .diagnostics
        .iter()
        .find(|d| d.code.ends_with(".call_unknown"))
        .expect("unknown call");
    assert_eq!(diagnostic.uri.as_deref(), Some("memory:compact.cemt"));
    assert_eq!(
        diagnostic.byte_offset,
        Some(source.find("{call").unwrap() as u64)
    );
    let required = parse("{module | {template @name=label | {param @name=title @required=true} {$title}} {template @name=entry | {call @template=label}}}");
    assert!(
        required
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.transform_template.param_required"),
        "{:?}",
        required.diagnostics
    );
}

#[test]
fn implicit_modules_collect_imports_calls_and_distinct_anonymous_rule_scopes() {
    let source = r#"{import @as=ui @src="./ui.cemt"}
{param @name=locale @default=en}
{template @name=helper | {param @name=label} {i | {$label}}}
{template @mode=cell @match=true |
    {param @name=label @default=first}
    {let @name=title @value=first}
    {call @template=helper @with:label='{$label}'}}
{template @mode=other @match=true |
    {param @name=label @default=second}
    {let @name=title @value=second}
    {call @template=helper @with:label='{$label}'}}
{p | before}{call @from=ui @template=icon}{p | after}"#;
    let parsed = parse(source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert!(
        !parsed.module_declared,
        "only an explicit wrapper sets this flag"
    );
    let options = parsed.module_options;
    assert_eq!(options.imports[0].alias, "ui");
    assert_eq!(options.imports[0].uri, "./ui.cemt");
    assert_eq!(options.entrypoints.len(), 1);
    assert_eq!(options.entrypoints[0].name, "helper");
    assert_eq!(options.params.len(), 4);
    assert_eq!(options.params[0].name, "locale");
    assert_ne!(options.params[2].name, options.params[3].name);
    assert_eq!(options.let_bindings.len(), 2);
    assert_ne!(
        options.let_bindings[0].owner_entrypoint,
        options.let_bindings[1].owner_entrypoint
    );
    assert_eq!(options.calls.len(), 3);
    assert_eq!(options.calls[2].owner_entrypoint, None);
    assert_eq!(options.calls[2].from.as_deref(), Some("ui"));
}

#[test]
fn implicit_module_default_bodies_preserve_preflight_and_call_locations() {
    let declarations = "{param @name=title @default=hello}{template @name=label | {b | {$title}}}";
    let content = "{p | before}{call @template=label}{p | after}";
    let direct = parse(&format!("{declarations}{content}"));
    assert!(direct.diagnostics.is_empty(), "{:?}", direct.diagnostics);
    for source in [
        format!("{declarations}{{body | {content}}}"),
        format!("{{module | {declarations}{content}}}"),
        format!("{{module | {declarations}{{body | {content}}}}}"),
    ] {
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "{source}: {:?}",
            parsed.diagnostics
        );
        assert_eq!(parsed.module_options, direct.module_options);
    }
    let source = "{p | before}\n{call @template=missing}";
    let parsed = parse(source);
    let diagnostic = parsed
        .diagnostics
        .iter()
        .find(|d| d.code.ends_with(".call_unknown"))
        .unwrap();
    assert_eq!(diagnostic.uri.as_deref(), Some("memory:compact.cemt"));
    assert_eq!(
        diagnostic.byte_offset,
        Some(source.find("{call").unwrap() as u64)
    );
}

#[test]
fn implicit_and_explicit_module_roots_reject_mixed_or_duplicate_bodies() {
    for content in [
        "{body | first}{b | second}",
        "text{body | wrapped}",
        "{body | first}{body | second}",
        "{body | wrapped}{$node}",
        "{body | wrapped}// output comment\n",
    ] {
        for source in [content.to_owned(), format!("{{module | {content}}}")] {
            let parsed = parse(&source);
            assert!(
                parsed
                    .diagnostics
                    .iter()
                    .any(|d| d.code == "cem.transform_template.declaration_invalid"),
                "{source}: {:?}",
                parsed.diagnostics
            );
        }
    }
}

#[test]
fn anonymous_rules_require_a_match_without_creating_named_entrypoints() {
    for declaration in [
        "{template @match=true | {b | cell}}",
        "{template @on=expression @into=content | {$dom:text()}}",
    ] {
        let parsed = parse(declaration);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        assert!(parsed.module_options.entrypoints.is_empty());
    }
    // An explicit module distinguishes an invalid rule from an HTML template node.
    let parsed = parse("{module | {template @mode=cell | {b | cell}}}");
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.transform_template.declaration_required"),
        "{:?}",
        parsed.diagnostics
    );
}
