use super::*;

#[test]
fn directives_prepare_single_original_provider_and_reject_non_singleton_values() {
    for (expression, ready) in [
        ("{#library}", true),
        ("{$ library}", true),
        ("{$ ()}", false),
        ("{$ (library, library)}", false),
        ("{$ (library, other)}", false),
        ("{$ 'urn:coercion'}", false),
        ("{$ 7}", false),
    ] {
        for directive in ["@ns ui =", "@default"] {
            let input = import(&format!("@ns public = urn:provider\n@ns another = urn:other\n{directive} {expression}\n{{item}}"));
            let id = if directive == "@default" {
                elements(&input, "@default")[0]
            } else {
                elements(&input, "@ns")[2]
            };
            let provider = elements(&input, "@ns")[0];
            let mut host = CemQlSchemaDeclarationHost::new();
            let mut ctx = context(&input, "library", &[provider]);
            ctx.bindings
                .extend(context(&input, "other", &[elements(&input, "@ns")[1]]).bindings);
            host.register_scope(input.tree.clone(), Some(ctx), policy());
            host.attach_captured_namespaces(input.captured.clone())
                .unwrap();
            let report = host
                .prepare_namespace_property(node(&input, id), policy().limits)
                .unwrap();
            assert_eq!(
                report.is_ready(),
                ready,
                "{directive} {expression}: {:?}",
                report.issue
            );
            assert!(report.property.is_some(), "original directive must decode");
            if ready {
                let target = report
                    .preparation
                    .as_ref()
                    .unwrap()
                    .target
                    .as_ref()
                    .unwrap();
                assert_eq!(target.binding_declaration().node_id(), provider);
                assert_eq!(target.namespace_uri(), "urn:provider");
                host.publish_namespace_property(&report).unwrap();
            }
            assert_original(&input);
        }
    }
}

#[test]
fn lifecycle_completes_default_alias_and_restores_nested_shadowing() {
    let input = import("@ns public = urn:provider\n@ns ui = {#library}\n@default ui\n{host |\n @ns ui = urn:inner\n {ui:inner}\n} {ui:after} {plain}");
    let provider = elements(&input, "@ns")[0];
    let roots = [
        elements(&input, "host")[0],
        elements(&input, "after")[0],
        elements(&input, "plain")[0],
    ];
    let ctx = context(&input, "library", &[provider]);
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), Some(ctx.clone()), policy());
    let (snapshot, ()) = host
        .with_namespace_lifecycle(
            input.captured.clone(),
            &roots,
            policy().limits,
            |_, _, _, _| (Some(ctx.clone()), Default::default()),
            |host, snapshot| {
                assert!(snapshot.is_complete(), "{:?}", snapshot.incomplete_roots);
                for (local, uri) in [
                    ("host", "urn:provider"),
                    ("inner", "urn:inner"),
                    ("after", "urn:provider"),
                    ("plain", "urn:provider"),
                ] {
                    assert_eq!(
                        host.consuming_expanded_name(&node(&input, elements(&input, local)[0]))
                            .unwrap()
                            .namespace_uri,
                        uri
                    );
                }
            },
        )
        .unwrap();
    assert!(snapshot.is_complete());
    assert!(host
        .consuming_expanded_name(&node(&input, roots[1]))
        .is_none());
    assert_original(&input);
}

#[test]
fn pending_shadow_cannot_borrow_old_binding_and_retry_keeps_earlier_default_alias() {
    let input = import("@ns public = urn:first\n@ns other = urn:second\n@ns ui = {#library}\n@default ui\n@ns ui = urn:later\n{ui:prefixed} {plain}");
    let roots = [
        elements(&input, "prefixed")[0],
        elements(&input, "plain")[0],
    ];
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), Some(Default::default()), policy());
    for selected in [None, Some(0), Some(1), None, Some(0)] {
        let (snapshot, ()) = host
            .with_namespace_lifecycle(
                input.captured.clone(),
                &roots,
                policy().limits,
                |_, _, _, _| {
                    (
                        selected.map(|index| {
                            context(&input, "library", &[elements(&input, "@ns")[index]])
                        }),
                        Default::default(),
                    )
                },
                |_, _| (),
            )
            .unwrap();
        assert_eq!(snapshot.is_complete(), selected.is_some());
        assert_eq!(
            snapshot
                .completion
                .expanded_name(roots[0])
                .unwrap()
                .namespace_uri,
            "urn:later"
        );
        match selected {
            Some(index) => assert_eq!(
                snapshot
                    .completion
                    .expanded_name(roots[1])
                    .unwrap()
                    .namespace_uri,
                if index == 0 {
                    "urn:first"
                } else {
                    "urn:second"
                }
            ),
            None => assert!(snapshot.completion.expanded_name(roots[1]).is_none()),
        }
    }
    assert_original(&input);
}

#[test]
fn empty_original_provider_resets_default_and_wrong_targets_stay_unready() {
    let input = import("@default \"\"\n@default urn:outer\n{host |\n @default {$ library}\n {item}\n}\n{outside} {$ missing()}");
    let directive = elements(&input, "@default")[2];
    let roots = [elements(&input, "host")[0], elements(&input, "outside")[0]];
    let mut host = CemQlSchemaDeclarationHost::new();
    let scope = host.register_scope(input.tree.clone(), Some(Default::default()), policy());
    for (target, ready) in [
        (elements(&input, "@default")[0], true),
        (roots[1], false),
        (elements(&input, "$")[1], false),
    ] {
        let ctx = context(&input, "library", &[target]);
        host.set_context(scope, Some(ctx.clone()));
        let (snapshot, ()) = host
            .with_namespace_lifecycle(
                input.captured.clone(),
                &roots,
                policy().limits,
                |_, _, _, _| (Some(ctx.clone()), Default::default()),
                |_, _| (),
            )
            .unwrap();
        assert_eq!(snapshot.is_complete(), ready);
        assert_eq!(
            snapshot
                .completion
                .expanded_name(roots[1])
                .unwrap()
                .namespace_uri,
            "urn:outer"
        );
        if ready {
            assert_eq!(
                snapshot
                    .completion
                    .expanded_name(elements(&input, "item")[0])
                    .unwrap()
                    .namespace_uri,
                ""
            );
            let target = snapshot
                .properties
                .iter()
                .find(|r| r.declaration.node_id() == directive)
                .unwrap()
                .preparation
                .as_ref()
                .unwrap()
                .target
                .as_ref()
                .unwrap();
            assert_eq!(
                target.binding_declaration().node_id(),
                elements(&input, "@default")[0]
            );
        }
    }
    assert!(host
        .compiled_source_expression(&node(&input, elements(&input, "$")[1]))
        .is_none());
    assert_original(&input);
}

#[test]
fn foreign_providers_require_grants_and_stale_results_cannot_publish_or_activate() {
    use cem_ql::schema_references::NamespacePublicationError;
    let input = import("@ns ui = {$ library}\n{ui:item}");
    let vendor = import("@ns public = urn:vendor");
    let ctx = context(&vendor, "library", &elements(&vendor, "@ns"));
    let mut host = CemQlSchemaDeclarationHost::new();
    let origin = host.register_scope(input.tree.clone(), Some(ctx.clone()), policy());
    let destination = host.register_scope(vendor.tree.clone(), Some(Default::default()), policy());
    host.attach_captured_namespaces(input.captured.clone())
        .unwrap();
    host.attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    let declaration = node(&input, elements(&input, "@ns")[0]);
    let denied = host
        .prepare_namespace_property(declaration.clone(), policy().limits)
        .unwrap();
    assert!(!denied.is_ready());
    assert!(denied
        .preparation
        .unwrap()
        .selection
        .issues
        .iter()
        .any(|i| i.kind
            == cem_ml::value::reference_resolution::ReferenceResolutionIssueKind::ScopeDenied));
    host.allow_scope_crossing(origin, destination);
    let ready = host
        .prepare_namespace_property(declaration.clone(), policy().limits)
        .unwrap();
    assert!(ready.is_ready());
    host.publish_namespace_property(&ready).unwrap();
    host.set_context(origin, Some(ctx));
    assert_eq!(
        host.publish_namespace_property(&ready),
        Err(NamespacePublicationError::DifferentSnapshot)
    );
    assert!(host
        .activate_namespace_properties(
            input.captured.clone(),
            &elements(&input, "item"),
            &[ready],
            |_, _, _, _| panic!("stale activation")
        )
        .is_err());
    let fresh = host
        .prepare_namespace_property(declaration, policy().limits)
        .unwrap();
    assert!(fresh.is_ready());
    host.activate_namespace_properties(
        input.captured.clone(),
        &elements(&input, "item"),
        &[fresh],
        |_, _, _, _| (Some(Default::default()), Default::default()),
    )
    .unwrap();
    assert_original(&input);
}

#[test]
fn dependency_cycles_stall_and_diamonds_share_original_provider_under_one_budget() {
    let input = import("@ns public = urn:provider\n{left |\n @ns l = {#library}\n {l:leftItem}\n}\n{right |\n @ns r = {$ library}\n {r:rightItem}\n}\n{base |\n @ns b = {#library}\n {b:baseItem}\n}");
    let declarations = elements(&input, "@ns");
    let roots = [
        elements(&input, "leftItem")[0],
        elements(&input, "rightItem")[0],
    ];
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), Some(Default::default()), policy());
    let mut full_work = 0;
    for (cycle, bounded) in [(true, false), (false, false), (false, true)] {
        let mut limits = policy().limits;
        if bounded {
            limits.max_work = full_work - 1;
        }
        let (snapshot, ()) = host
            .with_namespace_lifecycle(
                input.captured.clone(),
                &roots,
                limits,
                |source, _, _, _| {
                    let index = declarations
                        .iter()
                        .position(|id| {
                            input
                                .captured
                                .typed_prelude(input.captured.document(), *id)
                                .is_some_and(|slot| slot.value == source.node_id())
                        })
                        .unwrap();
                    let target = if cycle {
                        if index == 1 {
                            2
                        } else {
                            1
                        }
                    } else if index == 3 {
                        0
                    } else {
                        3
                    };
                    (
                        Some(context(&input, "library", &[declarations[target]])),
                        Default::default(),
                    )
                },
                |_, _| (),
            )
            .unwrap();
        assert_eq!(snapshot.is_complete(), !cycle && !bounded);
        assert!(snapshot.work_used <= limits.max_work);
        if snapshot.is_complete() {
            full_work = snapshot.work_used;
            assert_eq!(snapshot.properties.len(), 3);
            for property in snapshot.properties {
                assert_eq!(
                    property
                        .preparation
                        .unwrap()
                        .target
                        .unwrap()
                        .binding_declaration()
                        .node_id(),
                    declarations[0]
                );
            }
        }
    }
    assert_original(&input);
}
