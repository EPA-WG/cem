use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    value::artifact::CemValueArtifactLimits,
};
use cem_ql::render::{
    compile_template, project_element_reference_ids, render_compiled_template, render_plan_to_html,
    CompileTemplateOptions, TemplateData,
};
fn plan(extra: &str) -> cem_ql::render::RenderPlan {
    let source = format!("{{cem:variable @name=target @select='data:read(\"<dialog><b>body</b></dialog>\", \"xml\").root.children'}}{extra}");
    let artifact = compile_template(&source, &CompileTemplateOptions::default());
    assert!(
        artifact.diagnostics.is_empty(),
        "{:?}",
        artifact.diagnostics
    );
    let result = render_compiled_template(&artifact, &TemplateData::default());
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    result
}
fn project(
    plan: &cem_ql::render::RenderPlan,
    instance: &str,
) -> Result<cem_ql::render::RenderPlan, cem_ql::render::ElementReferenceProjectionError> {
    project_element_reference_ids(
        plan,
        instance,
        &CemValueArtifactLimits::default(),
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    )
}
#[test]
fn generated_ids_follow_original_targets_and_are_instance_local() {
    let original = plan("{button @commandfor={#target} | Open}{$target}");
    let first = project(&original, "instance-a").unwrap();
    let second = project(&original, "instance-b").unwrap();
    assert_eq!(render_plan_to_html(&first), "<button commandfor=\"instance-a-ref-1\">Open</button><dialog id=\"instance-a-ref-1\"><b>body</b></dialog>");
    assert!(render_plan_to_html(&second).contains("instance-b-ref-1"));
    assert_eq!(
        render_plan_to_html(&project(&original, "instance-a").unwrap()),
        render_plan_to_html(&first)
    );
    assert!(!render_plan_to_html(&original).contains("instance-a"));
}
#[test]
fn explicit_ids_and_literal_local_names_keep_their_contract() {
    let source = "{cem:variable @name=target @select='data:read(\"<dialog id=\\\"authored\\\"/>\", \"xml\").root.children'}{button @commandfor={#target}}{cem-action @command-target='@local'}{$target}";
    let artifact = compile_template(source, &CompileTemplateOptions::default());
    let original = render_compiled_template(&artifact, &TemplateData::default());
    assert!(
        original.diagnostics.is_empty(),
        "{:?}",
        original.diagnostics
    );
    let output = render_plan_to_html(&project(&original, "instance").unwrap());
    assert!(output.contains("commandfor=\"authored\""), "{output}");
    assert!(output.contains("command-target=\"@local\""));
    assert!(!output.contains("instance-ref"));
    let tree = cem_ml::import::import_data(
        "<div><button commandfor=\"authored\"/><dialog id=\"authored\"/></div>",
        "xml",
        "cem",
        "memory:literal-relationships.xml",
    )
    .unwrap();
    let data = TemplateData::default().with_binding(
        "tree",
        cem_ql::eval::ItemStream::once(cem_ql::eval::imported_cem_tree(tree)),
    );
    let artifact = compile_template(
        "{$tree}",
        &CompileTemplateOptions {
            host_bindings: vec!["tree".into()],
            ..Default::default()
        },
    );
    let original = render_compiled_template(&artifact, &data);
    assert!(
        original.diagnostics.is_empty(),
        "{:?}",
        original.diagnostics
    );
    let output = render_plan_to_html(&project(&original, "instance").unwrap());
    assert!(output.contains("commandfor=\"authored\""), "{output}");
    assert!(!output.contains("instance-ref"));
}
#[test]
fn absent_ambiguous_non_element_and_multiple_single_targets_reject() {
    for (source, code) in [
        (
            "{button @commandfor={#target}}",
            "cem.element_reference.target_missing",
        ),
        (
            "{button @commandfor={#target}}{$target}{$target}",
            "cem.element_reference.target_ambiguous",
        ),
        (
            "{button @commandfor={#(target, target)}}{$target}",
            "cem.element_reference.cardinality",
        ),
        (
            "{button @commandfor={#target.children.children}}{$target}",
            "cem.element_reference.target_kind",
        ),
    ] {
        let error = project(&plan(source), "instance").unwrap_err();
        assert_eq!(error.code(), code, "{source}: {error:?}");
    }
}
#[test]
fn interaction_references_use_an_explicit_projection_marker() {
    let result = project(
        &plan("{cem-action @command-target={#target}}{$target}"),
        "instance",
    )
    .unwrap();
    let html = render_plan_to_html(&result);
    assert!(html.contains("command-target=\"instance-ref-1\""), "{html}");
    assert!(html.contains("data-cem-node-ref-command-target"));
}
#[test]
fn focus_geometry_slots_are_single_placement_relationships() {
    for name in ["focus-target", "return-focus", "anchor", "boundary", "editor-for"] {
        let original = plan(&format!("{{surface @{name}={{#target}}}}{{$target}}"));
        let html = render_plan_to_html(&project(&original, "first").unwrap());
        assert!(html.contains(&format!("{name}=\"first-ref-1\"")), "{html}");
        assert!(
            html.contains(&format!("data-cem-node-ref-{name}")),
            "{html}"
        );
        assert!(
            render_plan_to_html(&project(&original, "second").unwrap()).contains("second-ref-1")
        );
        assert!(!render_plan_to_html(&original).contains("first-ref"));
        for (value, placement, code) in [
            (
                "#(target, target)",
                "{$target}",
                "cem.element_reference.cardinality",
            ),
            ("#()", "{$target}", "cem.element_reference.cardinality"),
            ("#target", "", "cem.element_reference.target_missing"),
            (
                "#target",
                "{$target}{$target}",
                "cem.element_reference.target_ambiguous",
            ),
            (
                "#target.children.children",
                "{$target}",
                "cem.element_reference.target_kind",
            ),
        ] {
            let original = plan(&format!("{{surface @{name}={{{value}}}}}{placement}"));
            assert_eq!(
                project(&original, "first").unwrap_err().code(),
                code,
                "{name}: {value}"
            );
        }
        let reserved = plan(&format!(
            "{{surface @data-cem-node-ref-{name}=bad}}{{$target}}"
        ));
        assert_eq!(
            project(&reserved, "first").unwrap_err().code(),
            "cem.element_reference.reserved"
        );
    }
    let literal = plan("{surface @focus-target=auto @return-focus=none @anchor=invoker @boundary=viewport}{$target}");
    let html = render_plan_to_html(&project(&literal, "first").unwrap());
    assert!(html.contains("focus-target=\"auto\""));
    assert!(html.contains("return-focus=\"none\""));
    assert!(!html.contains("data-cem-node-ref-"));
}

#[test]
fn native_dialog_and_tooltip_slots_export_independent_roles_without_inferring_invokers() {
    for tag in ["dialog", "aside"] {
        let original = plan(&format!("{{{tag} @focus-target={{#target.children}} @return-focus={{#target}} @boundary={{#target}} @anchor=pointer}}{{$target}}"));
        let output = render_plan_to_html(&project(&original, "surface").unwrap());
        assert!(
            output.contains("focus-target=\"surface-ref-1-0\""),
            "{output}"
        );
        assert!(
            output.contains("return-focus=\"surface-ref-1\""),
            "{output}"
        );
        assert!(output.contains("boundary=\"surface-ref-1\""), "{output}");
        assert!(output.contains("anchor=\"pointer\""), "{output}");
        assert!(!output.contains("data-cem-node-ref-anchor"));
        assert!(!output.contains("commandfor"));
    }
}
#[test]
fn traversal_is_bounded_and_cancelled_before_publication() {
    let original = plan("{button @commandfor={#target}}{$target}");
    let cancelled = OperationControl::default();
    cancelled.abort_signal().abort();
    assert!(project_element_reference_ids(
        &original,
        "instance",
        &Default::default(),
        &cancelled,
        ROOT_EXECUTION_SCOPE_ID
    )
    .is_err());
    let control = OperationControl::default();
    let limits = CemValueArtifactLimits {
        max_values: 2,
        ..Default::default()
    };
    assert!(project_element_reference_ids(
        &original,
        "instance",
        &limits,
        &control,
        ROOT_EXECUTION_SCOPE_ID
    )
    .is_err());
}

#[test]
fn native_lifecycle_host_evaluates_source_references_with_explicit_crossings() {
    use cem_ml::{
        import::import_data, parser::CemAstNode, schema::reference_policy::ReferenceScopePolicy,
        value::CemReference,
    };
    use cem_ql::{
        eval::{imported_cem_tree, RetainedCemNode},
        render::{
            project_element_reference_ids_with_host, RenderPlan, RenderPlanAttribute,
            RenderPlanNode,
        },
        schema_references::CemQlSchemaDeclarationHost,
    };
    let source = super::template();
    let reference_id = source
        .ast()
        .nodes
        .iter()
        .find_map(|n| {
            if let CemAstNode::Reference { node_id, .. } = n {
                Some(*node_id)
            } else {
                None
            }
        })
        .unwrap();
    let source_reference = RetainedCemNode::new(source.clone(), reference_id)
        .unwrap()
        .query_item();
    let destination = import_data("<dialog/>", "xml", "cem", "memory:target.xml").unwrap();
    let target = imported_cem_tree(destination.clone())
        .view()
        .unwrap()
        .field("children")
        .unwrap()[0]
        .clone();
    let original = RenderPlan {
        nodes: vec![
            RenderPlanNode::Element {
                tag: "button".into(),
                qualified_name: None,
                namespace: None,
                attributes: vec![RenderPlanAttribute {
                    name: "commandfor".into(),
                    qualified_name: None,
                    namespace: None,
                    value: String::new(),
                    value_stream: cem_ql::eval::ItemStream::once(source_reference),
                    contract: None,
                    source_map: Default::default(),
                }],
                children: vec![],
                source_map: Default::default(),
            },
            RenderPlanNode::Reference {
                reference: CemReference::new(vec![target.clone()]),
                source_map: Default::default(),
            },
        ],
        host_attribute_updates: vec![],
        diagnostics: vec![],
    };
    assert_eq!(
        project(&original, "instance").unwrap_err().code(),
        "cem.element_reference.pending"
    );
    for (grant, destination_limit) in [(false, 100000), (true, 100000), (true, 1)] {
        let policy = ReferenceScopePolicy::schema_defaults().unwrap();
        let mut host = CemQlSchemaDeclarationHost::new();
        let request = host.register_scope(
            source.clone(),
            Some(super::context(cem_ql::eval::ItemStream::once(
                target.clone(),
            ))),
            policy.clone(),
        );
        let mut target_policy = policy;
        target_policy.limits.max_work = destination_limit;
        let target_scope =
            host.register_scope(destination.clone(), Some(Default::default()), target_policy);
        if grant {
            host.allow_scope_crossing(request, target_scope);
        }
        let result = project_element_reference_ids_with_host(
            &original,
            "instance",
            &Default::default(),
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            &mut host.element_reference_host(request).unwrap(),
        );
        // One terminal destination node consumes one unit; the destination cap
        // applies without resetting the requesting scope's accumulated work.
        assert_eq!(result.is_ok(), grant, "{result:?}");
        if let Ok(result) = result {
            assert!(render_plan_to_html(&result).contains("commandfor=\"instance-ref-1\""));
        }
    }
    assert!(destination.ast().nodes.iter().all(|n| !matches!(n, CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "id")));
    assert!(matches!(
        source.ast().get(reference_id),
        Some(CemAstNode::Reference { targets: None, .. })
    ));
}
#[test]
fn lists_keep_order_repetitions_and_resolve_nested_constructed_references() {
    let original = plan("{label @aria-controls={#(target, target)}}{$target}");
    let html = render_plan_to_html(&project(&original, "instance").unwrap());
    assert!(
        html.contains("aria-controls=\"instance-ref-1 instance-ref-1\""),
        "{html}"
    );
    assert_eq!(html.matches("id=\"instance-ref-1\"").count(), 1);
    let original = plan("{output @for={#(target, target.children, target)}}{$target}");
    let html = render_plan_to_html(&project(&original, "instance").unwrap());
    assert!(
        html.contains("for=\"instance-ref-1 instance-ref-1-0\""),
        "An output's for is an ID-token set: {html}"
    );
    let original = plan("{td @headers={#(target, target)}}{$target}");
    assert!(
        render_plan_to_html(&project(&original, "instance").unwrap())
            .contains("headers=\"instance-ref-1\"")
    );
    for attribute in ["for", "aria-details", "aria-errormessage"] {
        let original = plan(&format!(
            "{{label @{attribute}={{#(target, target)}}}}{{$target}}"
        ));
        assert_eq!(
            project(&original, "instance").unwrap_err().code(),
            "cem.element_reference.cardinality",
            "{attribute}"
        );
    }
}

#[test]
fn explicit_aria_profiles_preserve_order_identity_and_literal_attributes() {
    use cem_ql::render::{
        project_element_reference_ids_with_options, AriaReferenceProfile,
        ElementReferenceExportOptions,
    };
    for attribute in ["aria-details", "aria-errormessage"] {
        for (operand, draft_value) in [
            ("#target", "instance-ref-1"),
            (
                "#(target.children, target)",
                "instance-ref-1-0 instance-ref-1",
            ),
            ("#(target, target)", "instance-ref-1 instance-ref-1"),
            ("#()", ""),
        ] {
            let original = plan(&format!("{{div @{attribute}={{{operand}}}}}{{$target}}"));
            let before = render_plan_to_html(&original);
            for profile in [
                AriaReferenceProfile::Recommendation12,
                AriaReferenceProfile::Draft13,
            ] {
                let result = project_element_reference_ids_with_options(
                    &original,
                    "instance",
                    &Default::default(),
                    &OperationControl::default(),
                    ROOT_EXECUTION_SCOPE_ID,
                    ElementReferenceExportOptions {
                        aria_profile: profile,
                    },
                );
                if draft_value.is_empty()
                    || (profile == AriaReferenceProfile::Recommendation12 && operand != "#target")
                {
                    assert_eq!(
                        result.unwrap_err().code(),
                        "cem.element_reference.cardinality"
                    );
                } else {
                    let html = render_plan_to_html(&result.unwrap());
                    assert!(
                        html.contains(&format!("{attribute}=\"{draft_value}\"")),
                        "{html}"
                    );
                    assert_eq!(html.matches("id=\"instance-ref-1\"").count(), 1);
                }
                assert_eq!(render_plan_to_html(&original), before);
            }
        }
        let original = plan(&format!("{{div @{attribute}='literal other'}}{{$target}}"));
        for profile in [
            AriaReferenceProfile::Recommendation12,
            AriaReferenceProfile::Draft13,
        ] {
            let output = project_element_reference_ids_with_options(
                &original,
                "instance",
                &Default::default(),
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
                ElementReferenceExportOptions {
                    aria_profile: profile,
                },
            )
            .unwrap();
            assert!(
                render_plan_to_html(&output).contains(&format!("{attribute}=\"literal other\""))
            );
        }
    }
    assert_eq!(
        AriaReferenceProfile::default().identity(),
        "wai-aria-1.2-rec-20230606"
    );
    assert!("wai-aria-1.3".parse::<AriaReferenceProfile>().is_err());
    assert_eq!(
        "wai-aria-1.3-wd-20260604"
            .parse::<AriaReferenceProfile>()
            .unwrap(),
        AriaReferenceProfile::Draft13
    );
}
#[test]
fn constructed_child_identity_is_preserved_for_relationship_targets() {
    let tree_artifact = compile_template(
        "{div | {dialog | body}}",
        &CompileTemplateOptions::default(),
    );
    let tree = render_compiled_template(&tree_artifact, &TemplateData::default());
    let data = TemplateData::default()
        .with_binding("tree", cem_ql::eval::output::output_nodes(tree.nodes));
    let artifact = compile_template(
        "{button @commandfor={#tree.children}}{$tree}",
        &CompileTemplateOptions {
            host_bindings: vec!["tree".into()],
            ..Default::default()
        },
    );
    let original = render_compiled_template(&artifact, &data);
    assert!(
        original.diagnostics.is_empty(),
        "{:?}",
        original.diagnostics
    );
    let html = render_plan_to_html(&project(&original, "instance").unwrap());
    assert!(html.contains("commandfor=\"instance-ref-1-0\""), "{html}");
    assert!(html.contains("<dialog id=\"instance-ref-1-0\">body</dialog>"));
}

#[test]
fn destination_work_is_shared_across_multiple_relationship_slots() {
    use cem_ml::{import::import_data, schema::reference_policy::ReferenceScopePolicy};
    use cem_ql::{
        eval::{imported_cem_tree, ItemStream},
        render::{project_element_reference_ids_with_host, RenderPlanNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let tree = import_data("<dialog/>", "xml", "cem", "memory:limited-target.xml").unwrap();
    let target = imported_cem_tree(tree.clone())
        .view()
        .unwrap()
        .field("children")
        .unwrap()[0]
        .clone();
    let data = TemplateData::default().with_binding("target", ItemStream::once(target));
    let artifact = compile_template(
        "{button @commandfor={#target}}{button @commandfor={#target}}{$target}",
        &CompileTemplateOptions {
            host_bindings: vec!["target".into()],
            ..Default::default()
        },
    );
    let original = render_compiled_template(&artifact, &data);
    assert!(matches!(
        &original.nodes[2],
        RenderPlanNode::Reference { .. }
    ));
    let mut host = CemQlSchemaDeclarationHost::new();
    let request = host.register_scope(
        super::template(),
        Some(Default::default()),
        ReferenceScopePolicy::schema_defaults().unwrap(),
    );
    let mut policy = ReferenceScopePolicy::schema_defaults().unwrap();
    policy.limits.max_work = 1;
    let destination = host.register_scope(tree, Some(Default::default()), policy);
    host.allow_scope_crossing(request, destination);
    let result = project_element_reference_ids_with_host(
        &original,
        "instance",
        &Default::default(),
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
        &mut host.element_reference_host(request).unwrap(),
    );
    assert!(
        result.is_err(),
        "Destination work must not reset per attribute: {result:?}"
    );
}

#[test]
fn copied_source_attribute_references_use_original_lifecycle_contexts() {
    use cem_ml::{
        import::import_bytes_with_lexical_scopes,
        schema::{reference_policy::ReferenceScopePolicy, vocab::CompiledSchema},
        value::CemReference,
    };
    use cem_ql::{
        api::{StandaloneExpressionBinding, StandaloneExpressionContext},
        eval::{imported_cem_tree, ItemStream},
        render::{project_element_reference_ids_with_host, RenderPlan, RenderPlanNode},
        schema_references::CemQlSchemaDeclarationHost,
    };
    let imported = import_bytes_with_lexical_scopes(
        b"{button @commandfor={#target}}{dialog}",
        "text/cem-ml",
        "memory:relationships.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    let input = imported_cem_tree(imported.tree.clone());
    let target = input
        .view()
        .unwrap()
        .field("children")
        .unwrap()
        .into_iter()
        .find(|n| {
            n.view()
                .and_then(|v| v.field("name"))
                .and_then(|v| v.first()?.atom())
                == Some(cem_ql::eval::AtomValue::String("dialog".into()))
        })
        .unwrap();
    let original = RenderPlan {
        nodes: vec![RenderPlanNode::Reference {
            reference: CemReference::new(vec![input]),
            source_map: Default::default(),
        }],
        host_attribute_updates: vec![],
        diagnostics: vec![],
    };
    let pending = project(&original, "instance").unwrap_err();
    assert!(pending.incomplete);
    assert!(pending
        .diagnostics
        .iter()
        .all(|d| d.code != "cem.element_reference.pending"));
    let mut host = CemQlSchemaDeclarationHost::new();
    let context = StandaloneExpressionContext::default().with_binding(
        "target",
        StandaloneExpressionBinding::any(ItemStream::once(target)),
    );
    let policy = ReferenceScopePolicy::schema_defaults().unwrap();
    let request = host.register_scope(imported.tree.clone(), Some(context.clone()), policy.clone());
    host.attach_captured_lexical_scopes(&imported.captured, |_, _, _| {
        (Some(context.clone()), policy.clone())
    })
    .unwrap();
    let result = project_element_reference_ids_with_host(
        &original,
        "instance",
        &Default::default(),
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
        &mut host.element_reference_host(request).unwrap(),
    )
    .unwrap();
    let html = render_plan_to_html(&result);
    assert!(html.contains("commandfor=\"instance-ref-1\""), "{html}");
    assert!(html.contains("<dialog id=\"instance-ref-1\""));
    assert!(!imported.tree.ast().nodes.iter().any(|n| matches!(n, cem_ml::parser::CemAstNode::Attribute { expanded_name, .. } if expanded_name.local_name == "id")));
}
#[test]
fn empty_targets_mixed_text_and_reserved_metadata_never_publish_ids() {
    for (source, code) in [
        (
            "{button @commandfor={#()}}{$target}",
            "cem.element_reference.cardinality",
        ),
        (
            "{button @commandfor='prefix{#target}'}{$target}",
            "cem.element_reference.cardinality",
        ),
        (
            "{button @data-cem-node-ref-command-target=bad}{$target}",
            "cem.element_reference.reserved",
        ),
        (
            "{button @commandfor={#target}}{i @id=instance-ref-2}{$target}",
            "cem.element_reference.id_conflict",
        ),
    ] {
        assert_eq!(
            project(&plan(source), "instance").unwrap_err().code(),
            code,
            "{source}"
        );
    }
}

#[test]
fn nested_constructed_references_obey_reference_depth_without_expanding_target_descendants() {
    use cem_ml::{import::import_data, value::CemReference};
    use cem_ql::eval::{imported_cem_tree, values::ReferenceView, Item, ItemStream};
    let tree = import_data("<dialog/>", "xml", "cem", "memory:nested-target.xml").unwrap();
    let target = imported_cem_tree(tree)
        .view()
        .unwrap()
        .field("children")
        .unwrap()[0]
        .clone();
    let inner = Item::native(ReferenceView(CemReference::new(vec![target.clone()])));
    let outer = Item::native(ReferenceView(CemReference::new(vec![inner])));
    let data = TemplateData::default()
        .with_binding("target", ItemStream::once(target))
        .with_binding("relationship", ItemStream::once(outer));
    let artifact = compile_template(
        "{button @commandfor={relationship}}{$target}",
        &CompileTemplateOptions {
            host_bindings: vec!["target".into(), "relationship".into()],
            ..Default::default()
        },
    );
    let original = render_compiled_template(&artifact, &data);
    assert!(project(&original, "instance").is_ok());
    let failure = project_element_reference_ids(
        &original,
        "instance",
        &CemValueArtifactLimits {
            max_depth: 1,
            ..Default::default()
        },
        &OperationControl::default(),
        ROOT_EXECUTION_SCOPE_ID,
    )
    .unwrap_err();
    assert!(failure.incomplete, "{failure:?}");
}
