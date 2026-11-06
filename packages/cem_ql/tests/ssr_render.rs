use cem_ml::{
    operation_control::ROOT_EXECUTION_SCOPE_ID, parser::tree::CemTreeNodeKind,
    source_map::TransformKind, ssr_artifact::SsrLifecycleState,
};
use cem_ql::{
    eval::retained_cem_node,
    ssr_island::prepare_inline_island,
    ssr_preparation::prepare_inline_dce,
    ssr_render::{render_and_apply_inline_dce, AppliedInlineDce},
};
use std::sync::Arc;
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
fn prepared(template: &str, body: &str) -> cem_ql::ssr_island::PreparedInlineIsland {
    let source=owner(&format!("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{template}</template></cem-element><aside>Unaffected &amp; stable</aside>{body}"));
    let dce = prepare_inline_dce(page(&source, "memory:output"), &control()).unwrap();
    prepare_inline_island(dce, "inline-one", &control()).unwrap()
}
fn apply(template: &str, body: &str) -> AppliedInlineDce {
    render_and_apply_inline_dce(prepared(template, body), &control()).unwrap()
}
fn code(result: Result<AppliedInlineDce, Vec<cem_ml::diagnostics::Diagnostic>>) -> String {
    result.unwrap_err()[0].code.clone()
}
#[test]
fn applies_one_shared_plan_after_inert_island_between_owned_boundaries_without_mutating_source() {
    let input = prepared("{p | Hello}", "<x-inline><b>Original</b></x-inline>");
    let source = input.prepared().page().source_document().clone();
    let bytes = source.source_text.clone();
    let source_tree = input.prepared().elements().tree.clone();
    let instance = input.prepared().instance_node();
    let result = render_and_apply_inline_dce(input, &control()).unwrap();
    assert!(Arc::ptr_eq(result.page().source_document(), &source));
    assert_eq!(source.source_text, bytes);
    assert!(Arc::ptr_eq(result.application().source(), &source_tree));
    let tree = result.application().tree();
    let id = result.application().output_target();
    assert_eq!(result.application().output_for_source(instance), Some(id));
    let children = &tree.node(id).unwrap().children;
    assert_eq!(children.len(), 4);
    assert_eq!(
        tree.node(children[0])
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name,
        "template"
    );
    assert_eq!(
        tree.node(children[1]).unwrap().kind,
        CemTreeNodeKind::Comment
    );
    assert_eq!(tree.node(children[1]).unwrap().value, "cem-render-start");
    assert_eq!(
        tree.node(children[2])
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name,
        "p"
    );
    assert_eq!(tree.node(children[3]).unwrap().value, "cem-render-end");
    assert_eq!(result.page().usage().render_passes, 1);
    assert!(matches!(
        result.page().lifecycle(),
        SsrLifecycleState::Partial { .. }
    ));
    assert!(!result.page().adoption_identity().hydration_ready());
    assert_eq!(
        result.input().payload()[0].query_item().identity(),
        retained_cem_node(&result.input().payload()[0].query_item())
            .unwrap()
            .query_item()
            .identity()
    );
    assert!(tree.native_owner().is_some());
}
#[test]
fn source_entities_surrounding_content_and_page_base_metadata_are_preserved() {
    let r = apply(
        "{span @title={datadom.attributes.label} | {$ datadom.attributes.label}}",
        "<x-inline label='A &amp; B'></x-inline>",
    );
    let tree = r.application().tree();
    assert_eq!(tree.base_uri(), Some("https://ssr.example.test/guide/"));
    assert_eq!(tree.document_uri(), Some("https://ssr.example.test/guide/"));
    let aside = (0..tree.ast().nodes.len())
        .find(|i| {
            tree.node(*i as u32)
                .and_then(|n| n.name.as_ref())
                .is_some_and(|n| n.local_name == "aside")
        })
        .unwrap() as u32;
    assert_eq!(
        tree.text_fragments(aside).unwrap().collect::<String>(),
        "Unaffected & stable"
    );
    let body = r.application().inserted_children()[2];
    let node = tree.node(body).unwrap();
    assert_eq!(tree.node(node.attributes[0]).unwrap().value, "A & B");
    assert_eq!(
        tree.text_fragments(body).unwrap().collect::<String>(),
        "A & B"
    );
}
#[test]
fn empty_output_keeps_island_and_an_empty_owned_range() {
    let r = apply("", "<x-inline>Original</x-inline>");
    assert!(r.plan().nodes.is_empty());
    assert_eq!(r.application().inserted_children().len(), 3);
}
#[test]
fn original_payload_and_source_maps_survive_projection_and_dom_application() {
    let r = apply(
        "{section | {$ datadom.slots.heading}}",
        "<x-inline><template><b slot='heading'>Title &amp; text</b></template></x-inline>",
    );
    let source = r.input().payload()[0].query_item();
    let original = retained_cem_node(&source).unwrap();
    assert!(Arc::ptr_eq(
        original.owner(),
        &r.input().prepared().elements().tree
    ));
    assert_eq!(cem_ql::render::render_plan_to_html(r.projected_plan()),"<section><b xmlns=\"http://www.w3.org/1999/xhtml\" slot=\"heading\">Title &amp; text</b></section>");
    let tree = r.application().tree();
    let id = r.application().inserted_children()[2];
    let b = tree.node(id).unwrap().children[0];
    assert_eq!(
        tree.node(b).unwrap().name.as_ref().unwrap().namespace_uri,
        "http://www.w3.org/1999/xhtml"
    );
    assert!(tree.node_source_map(b).unwrap().frames.iter().any(|f|matches!(&f.transform,TransformKind::SourceBoundary{uri} if uri=="file:///source/page.html")));
    assert_eq!(
        original.owner().source_parent(original.node_id()),
        r.input().payload_envelope().map(|n| n.node_id())
    );
}
#[test]
fn cancellation_and_completed_scope_are_not_restarted() {
    for completed in [false, true] {
        let input = prepared("{p | Hello}", "<x-inline></x-inline>");
        let c = control();
        if completed {
            c.complete_scope(ROOT_EXECUTION_SCOPE_ID).unwrap()
        } else {
            c.abort_signal().abort()
        };
        assert_eq!(
            code(render_and_apply_inline_dce(input, &c)),
            if completed {
                "cem.control.scope_completed"
            } else {
                "host-cancellation"
            }
        );
    }
}
#[test]
fn hard_evaluation_failure_cannot_publish_a_partial_replacement() {
    let p = prepared("{p | {$ 1 / 0}}", "<x-inline></x-inline>");
    let source = p.prepared().page().source_document().clone();
    let before = source.source_text.clone();
    assert!(!code(render_and_apply_inline_dce(p, &control())).is_empty());
    assert_eq!(source.source_text, before);
}
#[test]
fn unsupported_output_and_host_updates_fail_closed_before_dom_application() {
    for template in [
        "{script | alert(1)}",
        "{img @src=https://remote.example.test/a.png}",
        "{button @onclick=run | Unsafe}",
        "{x-child | Nested}",
        "{svg | {circle}}",
        "{a @href=javascript:run | Unsafe}",
        "{input @type=password @value=secret}",
        "{input | Invalid child}",
    ] {
        let p = prepared(template, "<x-inline></x-inline>");
        assert_eq!(
            code(render_and_apply_inline_dce(p, &control())),
            "cem.ssr.output_profile_unsupported",
            "{template}"
        );
    }
}
#[test]
fn physical_output_uri_does_not_change_applied_dom_or_instance_identity() {
    let source=owner("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{p | Hello}</template></cem-element><x-inline></x-inline>");
    let mut plans = vec![];
    for output in ["file:///one/index.html", "file:///two/index.html"] {
        let p = prepare_inline_dce(page(&source, output), &control()).unwrap();
        let island = prepare_inline_island(p, "inline-one", &control()).unwrap();
        let r = render_and_apply_inline_dce(island, &control()).unwrap();
        plans.push(cem_ql::render::render_plan_to_html(r.projected_plan()));
    }
    assert_eq!(plans[0], plans[1]);
}

#[test]
fn host_defaults_publish_through_the_shared_island_commit() {
    let p = prepared(
        "{attribute @name=label | Default}{p | Hello}",
        "<x-inline></x-inline>",
    );
    let r = render_and_apply_inline_dce(p, &control()).unwrap();
    assert_eq!(r.committed_island().writes()[0].name, "label");
    assert_eq!(r.committed_island().writes()[0].value, "Default");
    assert_eq!(r.page().usage().render_passes, 1);
}

#[test]
fn canonical_namespace_metadata_and_applied_owner_survive_native_extension_handoff() {
    use cem_ml::{
        parser::tree_apply::applied_qualified_name, ssr_artifact::CemSsrPageArtifact,
        transform_artifact::TransformArtifactBody,
    };
    let r = apply(
        "{p | Hello}",
        "<x-inline><template><b>Original</b></template></x-inline>",
    );
    let tree = r.application().tree().clone();
    assert!(Arc::ptr_eq(r.page().applied_tree().unwrap(), &tree));
    let template = r.application().inserted_children()[0];
    let root = tree.node(template).unwrap().children[0];
    assert_eq!(
        applied_qualified_name(&tree, root),
        Some("cem-island:context-root")
    );
    assert_eq!(
        tree.node(root)
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .namespace_uri,
        "https://cem.dev/ns/runtime/data-island"
    );
    let arc = r.page().source_document().clone();
    let routed = r.page().clone().into_transform_artifact("applied");
    let TransformArtifactBody::Extension(native) = routed.body else {
        panic!("native envelope")
    };
    let owner = native
        .as_any()
        .downcast_ref::<CemSsrPageArtifact>()
        .unwrap();
    assert!(Arc::ptr_eq(owner.source_document(), &arc));
    assert!(Arc::ptr_eq(owner.applied_tree().unwrap(), &tree));
    drop(r);
    assert_eq!(
        applied_qualified_name(&tree, root),
        Some("cem-island:context-root")
    );
    assert!(!owner.adoption_identity().hydration_ready());
}
#[test]
fn partial_dom_transition_rejects_foreign_sources_and_false_progress() {
    use cem_ml::ssr_artifact::SsrArtifactError;
    let r = apply("{p | Hello}", "<x-inline></x-inline>");
    let foreign = prepared("{p | Foreign}", "<x-inline></x-inline>");
    let mut usage = foreign.prepared().page().usage();
    usage.render_passes += 1;
    assert_eq!(
        foreign
            .prepared()
            .page()
            .clone()
            .into_applied_dom(r.application(), usage, vec![])
            .unwrap_err(),
        SsrArtifactError::SourceOwnerMismatch
    );
    assert_eq!(
        r.input()
            .prepared()
            .page()
            .clone()
            .into_applied_dom(r.application(), r.input().prepared().page().usage(), vec![])
            .unwrap_err(),
        SsrArtifactError::InvalidProgress
    );
}
#[test]
fn artifact_progress_cannot_skip_completed_passes() {
    // A fresh host page can select a single pass; applied usage must reflect it.
    let r = apply("{p | Hello}", "<x-inline></x-inline>");
    let mut usage = r.page().usage();
    usage.render_passes = r.page().policy().max_render_passes + 1;
    assert_eq!(
        r.page()
            .clone()
            .into_applied_dom(r.application(), usage, vec![])
            .unwrap_err(),
        cem_ml::ssr_artifact::SsrArtifactError::InvalidProgress
    );
}

#[test]
fn incomplete_source_text_projection_is_refused_before_derived_page_changes() {
    let source=owner("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{p | Hello}</template></cem-element><aside>Copyright &copy;</aside><x-inline></x-inline>");
    let p = prepare_inline_dce(page(&source, "memory:output"), &control()).unwrap();
    let p = prepare_inline_island(p, "inline-one", &control()).unwrap();
    assert_eq!(
        code(render_and_apply_inline_dce(p, &control())),
        "cem.ssr.source_projection_unsupported"
    );
}

/// Explicit JSON test-observation export, never a native AST handoff.
fn export_observation(tree: &cem_ml::parser::tree::RetainedCemTree, id: u32) -> serde_json::Value {
    use serde_json::json;
    let node = tree.node(id).unwrap();
    match node.kind {
        CemTreeNodeKind::Element => {
            assert_eq!(
                node.name.as_ref().unwrap().namespace_uri,
                "http://www.w3.org/1999/xhtml"
            );
            let attributes = node
                .attributes
                .iter()
                .filter_map(|id| {
                    let attr = tree.node(*id).unwrap();
                    let name = &attr.name.as_ref().unwrap().local_name;
                    (attr.name.as_ref().unwrap().namespace_uri != "http://www.w3.org/2000/xmlns/"
                        && name != "xmlns")
                        .then(|| (name.clone(), json!(attr.value)))
                })
                .collect::<serde_json::Map<String, serde_json::Value>>();
            json!({"tag":node.name.as_ref().unwrap().local_name,"attributes":attributes,
                "children":node.children.iter().map(|id|export_observation(tree,*id)).collect::<Vec<_>>()})
        }
        CemTreeNodeKind::Comment => json!({"comment":node.value}),
        CemTreeNodeKind::Text => json!(node.value),
        _ => panic!("Unsupported observation node"),
    }
}
#[test]
fn shared_render_observations_match_native_application() {
    use serde_json::{json, Value};
    let vectors: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/ssr-render-observations.json")).unwrap();
    let mut observations = vec![];
    for vector in vectors {
        let r = apply(
            vector["template"].as_str().unwrap(),
            vector["body"].as_str().unwrap(),
        );
        let tree = r.application().tree();
        let ids = r.application().inserted_children();
        let rendered = ids[2..ids.len() - 1]
            .iter()
            .map(|id| export_observation(tree, *id))
            .collect::<Vec<_>>();
        let payload = tree
            .ast()
            .nodes
            .iter()
            .enumerate()
            .find_map(|(id, _)| {
                let n = tree.node(id as u32)?;
                n.name
                    .as_ref()
                    .filter(|name| name.local_name == "payload")
                    .map(|_| n)
            })
            .unwrap()
            .children
            .iter()
            .map(|id| export_observation(tree, *id))
            .collect::<Vec<_>>();
        assert_eq!(json!(rendered), vector["rendered"], "{}", vector["name"]);
        assert_eq!(json!(payload), vector["payload"], "{}", vector["name"]);
        assert_eq!(r.page().usage().render_passes, 1);
        assert!(!r.page().adoption_identity().hydration_ready());
        let mut observation = json!({"name":vector["name"],"rendered":rendered,"payload":payload});
        if vector.get("hostAttributes").is_some() {
            let read_attributes = |id: u32| {
                tree.node(id)
                    .unwrap()
                    .attributes
                    .iter()
                    .map(|id| {
                        let a = tree.node(*id).unwrap();
                        (a.name.as_ref().unwrap().local_name.clone(), json!(a.value))
                    })
                    .collect::<serde_json::Map<String, Value>>()
            };
            let host_attributes = read_attributes(r.application().output_target());
            let root = tree.node(ids[0]).unwrap().children[0];
            let parts = &tree.node(root).unwrap().children;
            let entries = |id: u32, key: &str| {
                tree.node(id)
                    .unwrap()
                    .children
                    .iter()
                    .map(|id| {
                        let a = read_attributes(*id);
                        (a[key].as_str().unwrap().to_owned(), a["value"].clone())
                    })
                    .collect::<serde_json::Map<String, Value>>()
            };
            observation["hostAttributes"] = json!(host_attributes);
            observation["islandAttributes"] = json!(entries(parts[1], "name"));
            observation["islandDataset"] = json!(entries(parts[2], "key"));
            for name in ["hostAttributes", "islandAttributes", "islandDataset"] {
                assert_eq!(
                    observation[name], vector[name],
                    "{}: {name}",
                    vector["name"]
                );
            }
        }
        observations.push(observation);
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../dist/reports/cem-ssr-render-native.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_string_pretty(&observations).unwrap()).unwrap();
}

#[test]
fn unsupported_numeric_references_and_attribute_line_endings_are_explicit_refusals() {
    for content in [
        "<aside>&#0;</aside>",
        "<aside>&#x80;</aside>",
        "<aside title='A\rB'>Text</aside>",
    ] {
        let source=owner(&format!("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{{p | Hello}}</template></cem-element>{content}<x-inline></x-inline>"));
        let dce = prepare_inline_dce(page(&source, "memory:output"), &control()).unwrap();
        let island = prepare_inline_island(dce, "inline-one", &control()).unwrap();
        assert_eq!(
            code(render_and_apply_inline_dce(island, &control())),
            "cem.ssr.source_projection_unsupported"
        );
    }
}

#[test]
fn applied_tree_retains_operation_memory_until_its_last_owner_drops() {
    let c = control();
    let p = prepared("{p | Hello}", "<x-inline>Original</x-inline>");
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    let r = render_and_apply_inline_dce(p, &c).unwrap();
    let tree = r.page().applied_tree().unwrap().clone();
    let charged = c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap();
    assert!(charged > 0);
    drop(r);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), charged);
    drop(tree);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}

#[test]
fn native_application_limits_are_reduced_by_the_original_caller_policy() {
    use cem_ml::{
        operation_control::OperationControl,
        parser::tree_apply::apply_children,
        scheduler::{AbortSignal, ScopePolicy},
        value::CemValueNode,
    };
    let p = prepared("{p | Hello}", "<x-inline>Original</x-inline>");
    for policy in [
        ScopePolicy::host_root().with_memory_bytes(64),
        ScopePolicy::host_root().with_stack_depth(1),
    ] {
        let c = OperationControl::with_root_policy(AbortSignal::new(), policy).unwrap();
        let nodes: Arc<Vec<CemValueNode<(), ()>>> = Arc::new(vec![]);
        let result = apply_children(
            p.prepared().elements().tree.clone(),
            p.prepared().instance_node(),
            nodes,
            Arc::new(()),
            Default::default(),
            &c,
        );
        assert_eq!(result.unwrap_err().code, "cem.ast.apply_bounds");
        assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
}
