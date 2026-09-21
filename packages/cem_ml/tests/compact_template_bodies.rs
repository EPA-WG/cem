//! CEMT-COMPACT-TEMPLATES: module preflight accepts the renderer's direct bodies.
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
