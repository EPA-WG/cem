use cem_ql::eval::{AtomValue, Item, ItemStream};
use cem_ql::render::{render_template, TemplateData};

fn source() -> &'static str {
    "{cem-data @name=data @type=xml @select=source}"
}

fn data() -> TemplateData {
    TemplateData::default().with_binding(
        "source",
        ItemStream::once(Item::Atomic(AtomValue::String(
            "<root><name>ivy<em>saur</em></name><id>2</id></root>".into(),
        ))),
    )
}

fn rendered(body: &str) -> String {
    let body = body
        .lines()
        .map(str::trim)
        .map(|line| {
            if line.starts_with('@') {
                format!(" {line}")
            } else {
                line.to_owned()
            }
        })
        .collect::<String>();
    let result = render_template(&format!("{}{}", source(), body), &data());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.rendered
}

#[test]
fn explicit_text_and_implicit_template_focus() {
    assert_eq!(
        rendered(
            r#"{template @match='node.name == "name"' |
        {span | {$dom:text()}}}
        {apply-templates @select=data.root.children.children}"#
        ),
        "<span>ivysaur</span>"
    );
    assert_eq!(rendered("{$dom:text(data.root)}"), "ivysaur2");
}

#[test]
fn body_reuses_nodes_while_attributes_project_text() {
    assert_eq!(
        rendered(
            r#"{div @title="{$data.root.children.children}" |
        {$data.root.children.children}}"#
        ),
        "<div title=\"ivysaur2\"><name>ivy<em>saur</em></name><id>2</id></div>"
    );
}

#[test]
fn short_hook_receives_the_whole_sequence_and_restores_scope() {
    assert_eq!(
        rendered(
            r#"{div |
        {template @on=expression @into=content | {$dom:text()}}
        {$data.root.children.children}}
        {aside | {$data.root.children.children}}"#
        ),
        "<div>ivysaur2</div><aside><name>ivy<em>saur</em></name><id>2</id></aside>"
    );
}

#[test]
fn empty_hook_focus_does_not_fall_back_to_template_node() {
    assert_eq!(
        rendered(
            r#"{template @match='node.name == "name"' |
        {template @on=expression @into=content | {$dom:text()}}
        {span | {$node.missing}}}
        {apply-templates @select=data.root.children.children}"#
        ),
        "<span></span>"
    );
}

#[test]
fn named_calls_inherit_focus_without_rebinding_from_a_parameter_name() {
    assert_eq!(
        rendered(
            r#"{template @name=label | {span | {$dom:text()}}}
        {template @match='node.name == "name"' |
            {call @template=label @with:node="different"}}
        {apply-templates @select=data.root.children.children}"#
        ),
        "<span>ivysaur</span>"
    );
}

#[test]
fn missing_focus_is_an_error() {
    let result = render_template("{$dom:text()}", &TemplateData::default());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.code == "cem.ql.context_missing"),
        "{:?}",
        result.diagnostics
    );
}

#[test]
fn attribute_hooks_preserve_literals_and_convert_then_validate() {
    assert_eq!(
        rendered(
            r#"{template @on=expression @into=attribute
        @match='context.attribute.name == "count"' @returns=integer | {$value}}
        {output @count='{"002"}' @title="name: {$data.root.children.children}"}"#
        ),
        "<output count=\"2\" title=\"name: ivysaur2\"></output>"
    );
}

#[test]
fn attribute_contracts_cover_number_date_and_regex() {
    assert_eq!(
        rendered(
            r#"{output |
        {attribute @name=count @type=integer @minInclusive=1 @value='{"002"}'}
        {attribute @name=day @type=date @value='{"2024-02-29"}'}
        {attribute @name=code @type=string @pattern="[A-Z]{2}-[0-9]+" @value='{"AB-12"}'}}"#
        ),
        "<output count=\"2\" day=\"2024-02-29\" code=\"AB-12\"></output>"
    );
    for (contract, value) in [
        ("@type=integer @minInclusive=1", "0"),
        ("@type=date", "2023-02-29"),
        ("@type=string @pattern='[A-Z]+'", "abc"),
    ] {
        let result = render_template(
            &format!("{{output | {{attribute @name=value {contract} @value='{value}'}}}}"),
            &TemplateData::default(),
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.severity == cem_ml::diagnostics::Severity::Error),
            "{contract}: {:?}",
            result.diagnostics
        );
        assert!(
            result.rendered.is_empty(),
            "invalid typed output is never published"
        );
    }
}

#[test]
fn explicit_reference_clone_and_element_have_distinct_semantics() {
    assert_eq!(rendered(r#"{cem:variable @name=n @select=seq:first(data.root.children.children)}
        {span | {$dom:reference((n, n))}}
        {$dom:clone(n)}{$dom:element(n)}"#),
        "<span><name>ivy<em>saur</em></name><name>ivy<em>saur</em></name></span><name>ivy<em>saur</em></name><name></name>");
}

#[test]
fn reference_targets_keep_identity_and_clone_gets_independent_identity() {
    assert_eq!(
        rendered(
            r#"{cem:variable @name=n @select=seq:first(data.root.children.children)}
        {cem:variable @name=r @select=dom:reference(n)}
        {cem:variable @name=c @select=dom:clone(n)}
        {$r.targets.id == n.id}|{$c.id == n.id}|{$seq:count(dom:parent(c))}|{$dom:text(c)}"#
        ),
        "true|false|0|ivysaur"
    );
}

#[test]
fn source_text_occurrences_do_not_repeat_semantically_coalesced_neighbours() {
    let data = TemplateData::default().with_binding(
        "source",
        ItemStream::once(Item::Atomic(AtomValue::String(
            "<a>one<![CDATA[<two>]]>&amp;three</a>".into(),
        ))),
    );
    let rendered = render_template(
        &format!("{}{{$data.root.children.children}}", source()),
        &data,
    );
    assert!(
        rendered.diagnostics.is_empty(),
        "{:?}",
        rendered.diagnostics
    );
    assert_eq!(rendered.rendered, "one&lt;two&gt;&amp;three");
}

#[test]
fn clone_preserves_document_and_attribute_kinds() {
    assert_eq!(
        rendered(
            r#"{cem:variable @name=copy @select=dom:clone(data.root)}{$copy.kind}|{$seq:count(dom:parent(copy))}|{$dom:text(copy)}"#
        ),
        "document|0|ivysaur2"
    );
    assert_eq!(
        rendered(
            r#"{cem:variable @name=a @select='data:read("<a id=\"2\"/>", "xml").root.children.attributes'}{cem:variable @name=copy @select=dom:clone(a)}{$copy.kind}|{$seq:count(dom:parent(copy))}|{$dom:text(copy)}"#
        ),
        "attribute|0|2"
    );
}

#[test]
fn imported_hooks_are_module_defaults_below_caller_and_local_scopes() {
    use cem_ql::render::*;
    let base = r#"{module | {template @on=expression @into=content @priority=100 | {result-sequence @select='"base"'}}{template @name=present @visibility=public | {body | {span | {$"value"}}}}{template @name=local @visibility=public | {body | {template @on=expression @into=content | {result-sequence @select='"local"'}}{b | {$"value"}}}}}"#;
    let root = r#"{module | {import @as=base @src="./base.cemt"}{body | {call @from=base @template=present}{i | {$"root"}}{section | {template @on=expression @into=content | {result-sequence @select='"caller"'}}{call @from=base @template=present}{call @from=base @template=local}}}}"#;
    let hash =
        |s: &str| cem_ml::content_cache::ContentHash::from_blake3(s.as_bytes()).header_value();
    let artifact = compile_template_module_closure(
        root,
        &TemplateModuleClosure {
            root_uri: "https://example.test/root.cemt".into(),
            root_content_hash: hash(root),
            modules: vec![TemplateModuleSource {
                alias: "base".into(),
                parent_uri: None,
                uri: "https://example.test/base.cemt".into(),
                content_hash: hash(base),
                source: base.into(),
            }],
            ..Default::default()
        },
        &CompileTemplateOptions::default(),
    );
    let mut artifact = artifact;
    artifact.nodes = serde_json::from_slice(&serde_json::to_vec(&artifact.nodes).unwrap()).unwrap();
    let plan = render_compiled_template(&artifact, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        render_plan_to_html(&plan),
        "<span>base</span><i>root</i><section><span>caller</span><b>local</b></section>"
    );
}

#[test]
fn hook_precedence_and_predicate_failure_are_deterministic() {
    assert_eq!(
        rendered(
            r#"{template @on=expression @into=content @priority=5 | {$"older"}}{template @on=expression @into=content @priority=5 | {$"later"}}{template @on=expression @into=content @priority=1 | {$"low"}}{div | {$1}{section | {template @on=expression @into=content | {$"nearest"}}{$2}}{$3}}"#
        ),
        "<div>later<section>nearest</section>later</div>"
    );
    let result = render_template(
        r#"{template @on=expression @into=content @match='1 / 0 > 1' | {$value}}{p | {$"must not publish"}}"#,
        &TemplateData::default(),
    );
    assert!(!result.diagnostics.is_empty());
    assert!(result.rendered.is_empty(), "{}", result.rendered);
}

#[test]
fn receiver_declarations_validate_native_inputs_and_control_bindings() {
    use cem_ql::{eval::output::output_attribute, render::*};
    let produced = render_compiled_template(
        &compile_template(
            "{child | {attribute @name=count @type=integer @value=2}}",
            &CompileTemplateOptions::default(),
        ),
        &TemplateData::default(),
    );
    let RenderPlanNode::Element { attributes, .. } = &produced.nodes[0] else {
        panic!()
    };
    let mut data = TemplateData::default();
    data.bind_native_attribute(output_attribute(attributes[0].clone()))
        .unwrap();
    let result = render_template(
        "{attribute @name=count @type=integer @minInclusive=3}{p | {$count}}",
        &data,
    );
    assert!(result.rendered.is_empty(), "{}", result.rendered);
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.severity == cem_ml::diagnostics::Severity::Error));
    let good = render_template(
        "{attribute @name=count @type=integer @minInclusive=1}{p | {$count + 1}}",
        &data,
    );
    assert!(good.diagnostics.is_empty(), "{:?}", good.diagnostics);
    assert_eq!(good.rendered, "<p>3</p>");
    let scalar = TemplateData::default().with_binding(
        "count",
        ItemStream::once(Item::Atomic(AtomValue::String("002".into()))),
    );
    let good = render_template("{attribute @name=count @type=integer @minInclusive=1}{p | {$count + 1}|{$datadom.attributes.count + 1}}", &scalar);
    assert!(good.diagnostics.is_empty(), "{:?}", good.diagnostics);
    assert_eq!(good.rendered, "<p>3|3</p>");
}

#[test]
fn receiver_preserves_nodes_and_rejects_invalid_temporal_regex_and_required_values() {
    use cem_ql::render::*;
    let mut data = data();
    let producer = render_compiled_template(&compile_template(&format!("{}{{child | {{attribute @name=label @type=node @value='{{data.root.children.children}}'}}}}", source()), &CompileTemplateOptions { host_bindings: vec!["source".into()], ..Default::default() }), &data);
    let RenderPlanNode::Element { attributes, .. } = &producer.nodes[0] else {
        panic!()
    };
    data.bind_native_attribute(cem_ql::eval::output::output_attribute(
        attributes[0].clone(),
    ))
    .unwrap();
    let values = data.bindings["label"].clone();
    let output = render_template("{attribute @name=label @type=node}{p | {$label}}", &data);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(
        output.rendered,
        "<p><name>ivy<em>saur</em></name><id>2</id></p>"
    );
    assert_eq!(
        data.bindings["label"].items[0].identity(),
        values.items[0].identity()
    );
    for source in [
        "{attribute @name=missing @type=string @required=true}{p | invalid}",
        "{attribute @name=day @type=date | 2023-02-29}{p | invalid}",
        "{attribute @name=code @type=string @pattern='[A-Z]+' | abc}{p | invalid}",
    ] {
        let invalid = render_template(source, &TemplateData::default());
        assert!(
            invalid.rendered.is_empty(),
            "{source}: {}",
            invalid.rendered
        );
        assert!(!invalid.diagnostics.is_empty());
    }
}

#[test]
fn top_level_hooks_activate_only_after_their_declaration_and_capture_bindings() {
    assert_eq!(
        rendered(
            r#"{cem:variable @name=label @select='"first"'}{p | {$label}}{template @on=expression @into=content | {$label}}{cem:variable @name=label @select='"second"'}{p | {$label}}"#
        ),
        "<p>first</p><p>first</p>"
    );
    let failed = render_template(
        r#"{template @on=expression @into=content | {$1 / 0}}{p | {$"bad"}}"#,
        &TemplateData::default(),
    );
    assert!(failed.rendered.is_empty());
    assert!(!failed.diagnostics.is_empty());
}

#[test]
fn receiver_cannot_relax_a_named_schema_type_or_host_input_contract() {
    use cem_ml::schema::document_model::{AttributeModel, AttributeValueContract};
    let contract = AttributeValueContract {
        model: AttributeModel {
            value_type: Some("integer".into()),
            min_inclusive: Some("3".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut data = TemplateData::default().with_binding(
        "count",
        ItemStream::once(Item::Atomic(AtomValue::Integer(2))),
    );
    data.value_types
        .insert("positive-count".into(), contract.clone());
    let invalid = render_template(
        "{attribute @name=count @type=positive-count @minInclusive=1}{p | invalid}",
        &data,
    );
    assert!(invalid.rendered.is_empty());
    assert!(!invalid.diagnostics.is_empty());
    data.input_attribute_contracts
        .insert("count".into(), contract);
    let invalid = render_template(
        "{attribute @name=count @type=integer @minInclusive=1}{p | invalid}",
        &data,
    );
    assert!(invalid.rendered.is_empty());
    assert!(!invalid.diagnostics.is_empty());
}

#[test]
fn query_template_dispatch_returns_native_content_and_restores_focus() {
    assert_eq!(rendered(r#"
        {template @mode=label @match=true | {b | {$dom:text()}}}
        {template @mode=cell @match='node.name == "name"' |
            {i | {$dom:text(cemt:apply_templates(seq:where(dom:children(dom:parent(node)), fn(sibling) => sibling.name == "id"), "label"))}}
            {span | {$dom:text()}}}
        {$cemt:apply_templates(data.root.children.children, "cell")}
        {$cemt:apply_templates((), "cell")}
    "#), "<i>2</i><span>ivysaur</span>");
}

#[test]
fn query_template_dispatch_is_explicitly_hosted_and_bounded() {
    use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
    let query = compile(r#"cemt:apply_templates((), "cell")"#, &CompileContext::default()).unwrap();
    let values = evaluate(&query, &EvaluationContext::default());
    assert!(values.items.is_empty());
    assert!(values.diagnostics.iter().any(|d| d.code == "cem.ql.template_context_missing"), "{:?}", values);
    let result = render_template(r#"
        {template @match=true | {$cemt:apply_templates(node, "")}}
        {p | before}{$cemt:apply_templates("loop", "")}{p | after}
    "#, &TemplateData::default());
    assert!(result.diagnostics.iter().any(|d| d.code == "cem.transform_template.recursion_limit"), "{:?}", result.diagnostics);
    assert!(result.rendered.trim().is_empty(), "{}", result.rendered);
}

#[test]
fn query_template_errors_use_query_recovery_without_leaking_renderer_failure() {
    assert_eq!(rendered(r#"{template @mode=bad @match=true | {$1 / 0}}
        {$try { cemt:apply_templates("value", "bad") } catch (code, message) { "recovered" }}
        {p | after}"#), "recovered<p>after</p>");
}
