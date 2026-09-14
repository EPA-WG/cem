use cem_ql::render::{render_template, TemplateData};

fn render(source: &str) -> String {
    let result = render_template(source, &TemplateData::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result.rendered.split_whitespace().collect()
}

#[test]
fn template_recovery_discards_nodes_attributes_and_local_bindings() {
    assert_eq!(
        render(
            r#"
        {variable @name=label @select='"outer"'}
        {div @class=original |
            {try |
                {attribute @name=title @value=partial}
                {b | partial}
                {variable @name=label @select='"inner"'}
                {$report:raise("sample.invalid", "bad input")}
                {i | must-not-render}
                {catch @as=error | {em | {$error.code}:{$label}}}
            }
            {span | {$label}}
        }
    "#
        ),
        "<divclass=\"original\"><em>sample.invalid:outer</em><span>outer</span></div>"
    );
    assert_eq!(
        render(r#"{try | {p | ok}{catch @as=error | {$1 / 0}}}"#),
        "<p>ok</p>"
    );
}

#[test]
fn nested_calls_and_ordered_catches_propagate_to_outer_recovery() {
    assert_eq!(
        render(
            r#"
        {template @name=fail | {b | partial}{$1 / 0}{i | later}}
        {try |
            {try |
                {call @template=fail}
                {catch @as=e @test='e.code == "not-this"' | wrong}
                {catch @as=e | {$report:raise("sample.handler", e.message)}}
                {catch @as=e @test=true | sibling-must-not-catch}
            }
            {catch @as=e | {p | {$e.code}}}
        }
    "#
        ),
        "<p>sample.handler</p>"
    );
}

#[test]
fn unmatched_inner_catch_preserves_original_failure_for_outer_scope() {
    assert_eq!(
        render(
            r#"{try |
        {try | {b | partial}{$1 / 0}{catch @as=e @test=false | wrong}}
        {catch @as=e | {p | {$e.code}}}
    }"#
        ),
        "<p>cem.ql.type_error</p>"
    );
}

#[test]
fn catches_do_not_handle_diagnostic_only_reports_and_validate_structure() {
    let result = render_template(
        r#"{try | {$report:emit("sample.notice", "notice", "fatal")}{p | ok}{catch @as=e | wrong}}"#,
        &TemplateData::default(),
    );
    assert_eq!(result.rendered.trim(), "<p>ok</p>");
    assert!(result.diagnostics.iter().any(|d| d.code == "sample.notice"));
    for source in [
        "{catch @as=e | wrong}",
        "{try | no-handler}",
        "{try | {catch @as=e | handled}{p | after-catch}}",
        "{try | {catch @as='not a name' | wrong}}",
    ] {
        let result = render_template(source, &TemplateData::default());
        assert!(!result.diagnostics.is_empty(), "{source}");
    }
}

#[test]
fn recovery_covers_attribute_tests_and_iteration_expressions() {
    for body in [
        r#"{p @title='{1 / 0}' | partial}"#,
        r#"{if @test='(1 / 0) == 0' | wrong}"#,
        r#"{for-each @select='1 / 0' | wrong}"#,
    ] {
        assert_eq!(
            render(&format!(
                "{{try | {body} {{catch @as=e | {{p | {{$e.code}}}}}}}}"
            )),
            "<p>cem.ql.type_error</p>"
        );
    }
    // Catch predicates fail outward, never into sibling handlers.
    assert_eq!(
        render(
            r#"{try | {try | {$1 / 0}
        {catch @as=e @test='report:raise("sample.predicate", "bad")' | wrong}
        {catch @as=e | wrong}}
        {catch @as=e | {p | {$e.code}}}}"#
        ),
        "<p>sample.predicate</p>"
    );
}

#[test]
fn dynamic_constructor_and_missing_call_failures_are_recoverable() {
    for (body, code) in [
        (
            r#"{element @name='{"not a name"}' | partial}"#,
            "cem.ql.render.dynamic_name_invalid",
        ),
        (
            "{call @template=missing}",
            "cem.transform_template.call_unknown",
        ),
    ] {
        assert_eq!(
            render(&format!(
                "{{try | {{b | partial}}{body}{{catch @as=e | {{p | {{$e.code}}}}}}}}"
            )),
            format!("<p>{code}</p>")
        );
    }
}

#[test]
fn imported_calls_preserve_recovery_after_artifact_serialization() {
    use cem_ql::render::{
        compile_template_module_closure, render_compiled_template, render_plan_to_html,
        CompileTemplateOptions, TemplateModuleClosure, TemplateModuleSource,
    };
    let root = r#"{module | {import @as=base @src="./base.cemt"}
        {body | {try | {call @from=base @template=fail}{catch @as=e | {p | {$e.code}}}}}}"#;
    let base =
        r#"{module | {template @name=fail @visibility=public | {body | {b | partial}{$1 / 0}}}}"#;
    let hash =
        |s: &str| cem_ml::content_cache::ContentHash::from_blake3(s.as_bytes()).header_value();
    let mut artifact = compile_template_module_closure(
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
            ..TemplateModuleClosure::default()
        },
        &CompileTemplateOptions::default(),
    );
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    // Explicit serde artifact boundary; no native input AST is serialized.
    let bytes = serde_json::to_vec(&artifact.nodes).unwrap();
    artifact.nodes = serde_json::from_slice(&bytes).unwrap();
    let plan = render_compiled_template(&artifact, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(
        render_plan_to_html(&plan).trim(),
        "<p>cem.ql.type_error</p>"
    );
}

#[test]
fn recovery_survives_binary_template_artifact_reload() {
    use cem_ql::render::{render_compiled_template, render_plan_to_html, CompileTemplateOptions};
    use cem_ql::template_artifact::{
        compile_template_artifact, CompiledTemplateArtifact, TemplateArtifactLoadContext,
        TemplateArtifactSourceMapMode,
    };
    let source =
        r#"{try | {$report:raise("sample.binary", "bad")}{catch @as=e | {p | {$e.code}}}}"#;
    let artifact = compile_template_artifact(
        source,
        &CompileTemplateOptions::default(),
        TemplateArtifactSourceMapMode::Dev,
    );
    let reloaded = CompiledTemplateArtifact::from_bytes(artifact.bytes)
        .unwrap()
        .reload(&TemplateArtifactLoadContext {
            expected_source_hash: Some(cem_ml::content_cache::ContentHash::from_blake3(
                source.as_bytes(),
            )),
            host_bindings: vec![],
            source_map_mode: TemplateArtifactSourceMapMode::Dev,
        })
        .unwrap();
    let plan = render_compiled_template(&reloaded, &TemplateData::default());
    assert!(plan.diagnostics.is_empty(), "{:?}", plan.diagnostics);
    assert_eq!(render_plan_to_html(&plan), "<p>sample.binary</p>");
}

#[test]
fn template_recovery_cannot_suppress_host_cancellation_or_recursion_limits() {
    use cem_ml::operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID};
    use cem_ql::render::{
        compile_template, render_compiled_template_with_control, CompileTemplateOptions,
    };
    let recursive = r#"{template @name=again | {call @template=again}}
        {try | {call @template=again}{catch @as=e | {p | must-not-catch}}}"#;
    let result = render_template(recursive, &TemplateData::default());
    assert!(result
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.transform_template.recursion_limit"));
    assert!(!result.rendered.contains("must-not-catch"));
    let artifact = compile_template(recursive, &CompileTemplateOptions::default());
    let control = OperationControl::default();
    control.cancel_root(None, None).unwrap();
    let plan = render_compiled_template_with_control(
        &artifact,
        &TemplateData::default(),
        &control,
        ROOT_EXECUTION_SCOPE_ID,
    );
    assert!(plan.nodes.is_empty());
    assert!(plan
        .diagnostics
        .iter()
        .any(|d| d.code == "cem.ql.render.control_failure"));
}

#[test]
fn caught_error_is_a_native_source_mapped_value_at_call_boundaries() {
    use cem_ml::source_map::SourceMapStack;
    use cem_ql::render::{
        compile_template, render_compiled_template_with_calls, CompileTemplateOptions, RenderPlan,
        RenderPlanAttribute, TemplateCallHandler, TemplateCallResult, TemplateData,
    };
    struct Inspect;
    impl TemplateCallHandler for Inspect {
        fn handles(&self, tag: &str) -> bool {
            tag == "inspect-error"
        }
        fn call(
            &self,
            attributes: &[RenderPlanAttribute],
            _: &SourceMapStack,
            _: &TemplateData,
            protected: bool,
        ) -> TemplateCallResult {
            assert!(protected);
            let value = &attributes
                .iter()
                .find(|a| a.name == "with:failure")
                .unwrap()
                .value_stream
                .items[0];
            let view = value.view().expect("caught failure is native");
            assert_eq!(view.representation_id(), "cem.ql.diagnostic");
            let source = value
                .source_map()
                .expect("source maps survive catch and call forwarding");
            assert!(
                source.frames.len() >= 2,
                "template host plus original query frames: {source:?}"
            );
            TemplateCallResult {
                plan: RenderPlan {
                    nodes: vec![],
                    host_attribute_updates: vec![],
                    diagnostics: vec![],
                },
                failure: None,
            }
        }
    }
    let artifact = compile_template(
        r#"{try | {$report:raise("sample.source", "bad")}{catch @as=e | {inspect-error @with:failure='{e}'}}}"#,
        &CompileTemplateOptions::default(),
    );
    let result = render_compiled_template_with_calls(
        &artifact,
        &TemplateData::default(),
        None,
        &Inspect,
        false,
    );
    assert!(result.failure.is_none());
    assert!(
        result.plan.diagnostics.is_empty(),
        "{:?}",
        result.plan.diagnostics
    );
}
