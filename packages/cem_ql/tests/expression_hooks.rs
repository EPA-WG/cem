//! CEMT-HOOK-FOCUS: expression hooks retain sequence, lexical and focus boundaries.
use cem_ql::render::{render_template, TemplateData};

fn render(source: &str) -> String {
    let source = source.lines().map(str::trim).collect::<String>();
    let result = render_template(&source, &TemplateData::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.rendered
}

#[test]
fn attribute_hooks_receive_each_expression_sequence_and_leave_literals_alone() {
    assert_eq!(
        render(
            r#"
        {template @on=expression @into=attribute |
            {$context.attribute.name + ":" + dom:text(seq:count(value)) + ":" + dom:text()}}
        {p @title='pre-{(1, 2)}-mid-{()}-post-{3}' @lang=en}
    "#
        ),
        "<p title=\"pre-title:2:12-mid-title:0:-post-title:1:3\" lang=\"en\"></p>"
    );
}

#[test]
fn hook_scope_captures_each_iteration_and_does_not_escape_the_loop() {
    assert_eq!(
        render(
            r#"
        {cem:variable @name=label @select='"outside"'}
        {cem:for-each @select='("one", "two")' @as=label |
            {template @on=expression @into=content | {$label + ":" + dom:text()}}
            {cem:variable @name=label @select='"changed"'}
            {p | {$label}}}
        {p | {$label}}
    "#
        ),
        "<p>one:changed</p><p>two:changed</p><p>outside</p>"
    );
}

#[test]
fn nested_query_focus_restores_the_complete_hook_input() {
    assert_eq!(
        render(
            r#"
        {template @on=expression @into=content |
            {$dom:text(value.where(dom:text() == "ivy")) + ":" + dom:text()}}
        {p | {$("ivy", "saur")}}
    "#
        ),
        "<p>ivy:ivysaur</p>"
    );
}

#[test]
fn active_hook_is_skipped_for_nested_content_and_outer_behavior_can_apply() {
    assert_eq!(
        render(
            r#"
        {template @on=expression @into=content | {$"outer(" + dom:text() + ")"}}
        {section |
            {template @on=expression @into=content | {b | {$value}}}
            {$"first"}{$"second"}}
        {aside | {$"last"}}
    "#
        ),
        "<section><b>outer(first)</b><b>outer(second)</b></section><aside>outer(last)</aside>"
    );
}

#[test]
fn nested_attribute_hook_restores_content_context_and_reserved_bindings() {
    assert_eq!(render(r#"
        {cem:variable @name=value @select='"caller-value"'}
        {cem:variable @name=context @select='"caller-context"'}
        {section |
            {template @on=expression @into=attribute | {$context.into + ":" + context.attribute.name + ":" + dom:text()}}
            {template @on=expression @into=content |
                {b @title="{$value}" | {$value}}
                {$context.into + ":" + dom:text()}}
            {$"ivy"}}
        {aside | {$value}|{$context}}
    "#), "<section><b title=\"attribute:title:ivy\">ivy</b>content:ivy</section><aside>caller-value|caller-context</aside>");
}

#[test]
fn attribute_bodies_and_transparent_controls_use_attribute_hooks() {
    assert_eq!(render(r#"
        {template @on=expression @into=attribute | {$"attribute:" + dom:text()}}
        {template @on=expression @into=content | {$"content:" + dom:text()}}
        {template @name=emit | {$"called"}}
        {template @mode=emit @match=true | {$node}}
        {p |
            {attribute @name=via-value @value='{"one"}'}
            {attribute @name=via-body | {$"two"}}
            {attribute @name=conditional | {cem:if @test=true | {$"four"}}}
            {attribute @name=choice | {cem:choose | {cem:when @test=false | bad}{cem:otherwise | {$"chosen"}}}}
            {attribute @name=loop | {cem:for-each @select='("a", "b")' @as=letter | {$letter}}}
            {attribute @name=call | {call @template=emit}}
            {attribute @name=match | {apply-templates @select='"matched"' @mode=emit}}
            {attribute @name=rich @content-type=text/html | {b | {$"three"}}}}
    "#), "<p via-value=\"attribute:one\" via-body=\"attribute:two\" conditional=\"attribute:four\" choice=\"attribute:chosen\" loop=\"attribute:aattribute:b\" call=\"attribute:called\" match=\"attribute:matched\" rich=\"&lt;b&gt;content:three&lt;/b&gt;\"></p>");
}

#[test]
fn constructed_elements_reset_content_scope_and_restore_attribute_metadata() {
    assert_eq!(render(r#"
        {template @on=expression @into=attribute |
            {$context.attribute.name + ":" + (context.attribute.content_type ?? "plain") + ":" + dom:text()}}
        {template @on=expression @into=content | {$"content:" + dom:text()}}
        {p | {attribute @name=rich @content-type=text/html |
            {$"start"}
            {b @title='{"hint"}' | {$"inside"}}
            {element @name=i | {$"dynamic"}}
            {result-element @name=em | {$"native"}}
            {$"end"}}}
        {aside | {$"after"}}
    "#), "<p rich=\"rich:text/html:start&lt;b title=&quot;title:plain:hint&quot;&gt;content:inside&lt;/b&gt;&lt;i&gt;content:dynamic&lt;/i&gt;&lt;em&gt;content:native&lt;/em&gt;rich:text/html:end\"></p><aside>content:after</aside>");
}

#[test]
fn native_attribute_constructors_use_attribute_hooks_without_changing_text_spacing() {
    assert_eq!(
        render(
            r#"
        {template @on=expression @into=attribute | {$context.attribute.name + ":" + dom:text()}}
        {template @on=expression @into=content | {$"content:" + dom:text()}}
        {result-element @name=p |
            {result-attribute @name=title @value='pre-{"hint"}-post'}
            {result-attribute @name=body | pre-{$"one"}{cem:if @test=true | {$"two"}}-post}
            {$"inside"}}
    "#
        ),
        "<p title=\"pre-title:hint-post\" body=\"pre-body:onebody:two-post\">content:inside</p>"
    );
}

#[test]
fn constructed_text_nodes_use_content_hooks_inside_attribute_payloads() {
    assert_eq!(render(r#"
        {template @on=expression @into=attribute | {$"attribute:" + dom:text()}}
        {template @on=expression @into=content | {$"content:" + dom:text()}}
        {p | {attribute @name=rich @content-type=text/html |
            {comment | {$"note"}}
            {cdata | {$"raw"}}
            {processing-instruction @name=hint | {$"data"}}
            {$"end"}}}
    "#), "<p rich=\"&lt;!--content:note--&gt;content:raw&lt;?hint content:data?&gt;attribute:end\"></p>");
}

#[test]
fn attribute_body_hooks_preserve_atomic_sequences_and_node_identity_after_reload() {
    use cem_ql::template_artifact::{
        compile_template_artifact, TemplateArtifactLoadContext, TemplateArtifactSourceMapMode,
    };
    use cem_ql::{
        api::{compile, evaluate, CompileContext, EvaluationContext},
        eval::{AtomValue, ItemStream},
        render::*,
    };
    let query = compile(
        r#"data:read("<name>ivy<em>saur</em></name>", "xml").root.children"#,
        &CompileContext::default(),
    )
    .unwrap();
    let nodes = evaluate(&query, &EvaluationContext::default());
    assert!(nodes.error.is_none());
    let source = concat!(
        "{template @on=expression @into=attribute | {$value}}",
        "{p | {attribute @name=mixed @type=any | {cem:if @test=true | {$(1, true, node)}}}",
        "{attribute @name=empty @type=any | {$()}}}"
    );
    let options = CompileTemplateOptions {
        host_bindings: vec!["node".into()],
        ..Default::default()
    };
    let artifact = compile_template_artifact(source, &options, TemplateArtifactSourceMapMode::Dev);
    let loaded = artifact
        .reload(&TemplateArtifactLoadContext {
            host_bindings: options.host_bindings,
            expected_source_hash: Some(cem_ml::content_cache::ContentHash::from_blake3(
                source.as_bytes(),
            )),
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        })
        .unwrap();
    let data =
        TemplateData::default().with_binding("node", ItemStream::once(nodes.items[0].clone()));
    let result = render_compiled_template(&loaded, &data);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let RenderPlanNode::Element { attributes, .. } = &result.nodes[0] else {
        panic!()
    };
    let values = &attributes[0].value_stream.items;
    assert_eq!(values.len(), 3);
    assert_eq!(values[0].atom(), Some(AtomValue::Integer(1)));
    assert_eq!(values[1].atom(), Some(AtomValue::Boolean(true)));
    assert_eq!(values[2].identity(), nodes.items[0].identity());
    assert!(attributes[1].value_stream.items.is_empty());
}

#[test]
fn native_module_calls_inherit_attribute_destination_and_type_metadata() {
    use cem_ql::render::*;
    struct Module(TemplateArtifact);
    impl TemplateCallHandler for Module {
        fn handles(&self, tag: &str) -> bool {
            tag == "remote-emit"
        }
        fn call(
            &self,
            _: &[RenderPlanAttribute],
            _: &cem_ml::source_map::SourceMapStack,
            data: &TemplateData,
            protected: bool,
        ) -> TemplateCallResult {
            render_compiled_template_with_calls(&self.0, data, None, self, protected)
        }
    }
    let options = CompileTemplateOptions::default();
    let module = Module(compile_template("{$2}", &options));
    let root = compile_template(
        concat!(
            r#"{template @on=expression @into=attribute @match='context.attribute.name == "count" && context.attribute.type == "integer"' | {$value + 1}}"#,
            r#"{template @on=expression @into=content | {$"content"}}"#,
            "{p | {attribute @name=count @type=integer | {remote-emit}}}{aside | {$9}}"
        ),
        &options,
    );
    let result =
        render_compiled_template_with_calls(&root, &TemplateData::default(), None, &module, false);
    assert!(result.failure.is_none());
    assert!(
        result.plan.diagnostics.is_empty(),
        "{:?}",
        result.plan.diagnostics
    );
    assert_eq!(
        render_plan_to_html(&result.plan),
        "<p count=\"3\"></p><aside>content</aside>"
    );
}

#[test]
fn attribute_body_hook_results_still_obey_destination_constraints() {
    let valid = concat!(
        "{template @on=expression @into=attribute @returns=integer | {$value + 1}}",
        "{p | {attribute @name=count @type=integer @minInclusive=3 | {cem:if @test=true | {$2}}}}"
    );
    assert_eq!(render(valid), "<p count=\"3\"></p>");
    let invalid = valid.replace("@minInclusive=3", "@minInclusive=4");
    let result = render_template(&invalid, &TemplateData::default());
    assert!(result.rendered.is_empty());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.severity == cem_ml::diagnostics::Severity::Error));
}

#[test]
fn failing_or_recursive_attribute_hooks_publish_no_partial_result() {
    for hook in ["{$1 / 0}", "{call @template=loop}"] {
        let source = format!(concat!(
            "{{template @name=loop | {{call @template=loop}}}}",
            "{{template @on=expression @into=attribute | {}}}",
            "{{aside | prefix}}{{p | {{attribute @name=value | {{cem:if @test=true | {{$1}}}}}}}}{{aside | suffix}}"), hook);
        let result = render_template(&source, &TemplateData::default());
        assert!(result.rendered.is_empty(), "{}", result.rendered);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.severity == cem_ml::diagnostics::Severity::Error),
            "{:?}",
            result.diagnostics
        );
    }
}

#[test]
fn cancelled_attribute_hook_discards_the_render_and_releases_memory() {
    use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    use cem_ql::{
        eval::ItemStream,
        native::{NativeQueryFunction, NativeQueryRequest},
        render::*,
    };
    #[derive(Debug)]
    struct Cancel;
    impl NativeQueryFunction for Cancel {
        fn call(&self, request: NativeQueryRequest<'_>) -> ItemStream {
            request.control.abort_signal().abort();
            request.arguments[0].clone()
        }
    }
    let mut data = TemplateData::default();
    data.native_functions
        .register("hook.cancel", 1, Cancel)
        .unwrap();
    let source = concat!(
        r#"{template @on=expression @into=attribute | {$native:call("hook.cancel", value)}}"#,
        "{aside | prefix}{p | {attribute @name=value | {cem:if @test=true | {$1}}}}"
    );
    let control = OperationControl::default();
    let compiled = compile_template(source, &CompileTemplateOptions::default());
    let result =
        render_compiled_template_with_control(&compiled, &data, &control, ROOT_EXECUTION_SCOPE_ID);
    assert!(result.nodes.is_empty(), "{result:?}");
    assert!(!result.diagnostics.is_empty());
    assert_eq!(control.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
