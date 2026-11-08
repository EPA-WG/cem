use cem_ql::ssr_page::{prepare_inline_page, SsrPageProgress, SsrPageState};
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
fn source(body: &str) -> std::sync::Arc<cem_ml::validation::html::HtmlDocumentAst> {
    owner(&format!("<cem-element tag='x-parent'><template id='x-parent' type='text/cem-ml'>{{section | {{x-child @label=Generated}} {{$ datadom.slots.heading}}}}</template></cem-element><cem-element tag='x-child'><template id='x-child' type='text/cem-ml'>{{b | {{$ datadom.attributes.label}}}}</template></cem-element>{body}"))
}
#[test]
fn existing_dom_projection_is_not_silently_rebuilt_from_original_html() {
    use cem_ml::{
        parser::{document::CemDocument, CemAstNode},
        transform_artifact::TransformArtifactBody,
    };
    let c = control();
    let source = source("<x-parent></x-parent>");
    let policy = page(&source, "memory:output").policy().clone();
    let projected = fixture::page_with_document(
        &source,
        "memory:output",
        policy,
        TransformArtifactBody::DomProjection(std::sync::Arc::new(CemDocument {
            nodes: vec![
                CemAstNode::Document {
                    node_id: 0,
                    root_children: vec![1],
                    source: Default::default(),
                },
                CemAstNode::Text {
                    node_id: 1,
                    data: "Existing projected content".into(),
                    source: Default::default(),
                },
            ],
            ..Default::default()
        })),
    );
    let errors = prepare_inline_page(projected, "page", &c).unwrap_err();
    assert!(errors
        .iter()
        .any(|d| d.code == "cem.ssr.preparation_source_unsupported"));
    assert_eq!(
        c.memory_charged(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
            .unwrap(),
        0
    );
}
#[test]
fn parent_first_discovery_settles_generated_and_source_children_with_distinct_ids() {
    let c = control();
    let source=source("<x-parent><x-child slot='heading' label='Source'></x-child></x-parent><x-child label='Sibling'></x-child>");
    let before = source.source_text.clone();
    let mut coordinator =
        prepare_inline_page(page(&source, "memory:output"), "page-one", &c).unwrap();
    let mut seen = vec![];
    loop {
        match coordinator.advance(&c).unwrap() {
            SsrPageProgress::Rendered {
                tag,
                instance_id,
                depth,
            } => seen.push((tag, instance_id, depth)),
            SsrPageProgress::Quiescent => break,
            SsrPageProgress::Pending(_) => panic!("passive profile cannot request resources"),
        }
    }
    assert_eq!(
        seen.iter()
            .map(|(tag, _, depth)| (tag.as_str(), *depth))
            .collect::<Vec<_>>(),
        vec![
            ("x-parent", 1),
            ("x-child", 2),
            ("x-child", 2),
            ("x-child", 1)
        ]
    );
    assert_eq!(
        seen.iter()
            .map(|(_, id, _)| id.as_str())
            .collect::<Vec<_>>(),
        vec!["page-one-1", "page-one-2", "page-one-3", "page-one-4"]
    );
    let settled = coordinator.finish(&c).unwrap();
    assert_eq!(settled.page().usage().instances, 4);
    assert_eq!(settled.page().usage().render_passes, 4);
    assert_eq!(settled.page().usage().declaration_depth, 2);
    assert_eq!(settled.participants().len(), 4);
    let tree = settled.tree();
    for participant in settled.participants() {
        let host = tree.node(participant.host_node()).unwrap();
        assert_eq!(host.children[0], participant.island_node());
        assert_eq!(
            tree.node(host.children[1]).unwrap().value,
            "cem-render-start"
        );
        assert_eq!(
            tree.node(*host.children.last().unwrap()).unwrap().value,
            "cem-render-end"
        );
    }
    assert_eq!(source.source_text, before);
    assert!(!settled.page().adoption_identity().hydration_ready());
}
#[test]
fn instance_depth_and_render_exhaustion_are_terminal_without_changing_source() {
    for bound in ["instances", "depth", "renders"] {
        let c = control();
        let source = source("<x-parent></x-parent>");
        let before = source.source_text.clone();
        let mut policy = page(&source, "memory:output").policy().clone();
        match bound {
            "instances" => policy.max_instances = 1,
            "depth" => policy.max_declaration_depth = 1,
            _ => policy.max_render_passes = 1,
        };
        let mut coordinator = prepare_inline_page(
            fixture::page_with_policy(&source, "memory:output", policy),
            "page",
            &c,
        )
        .unwrap();
        loop {
            if coordinator.advance(&c).is_err() {
                break;
            }
        }
        assert_eq!(coordinator.state(), SsrPageState::Refused);
        assert!(coordinator.finish(&c).is_err());
        assert_eq!(source.source_text, before);
        assert_eq!(
            c.memory_charged(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
                .unwrap(),
            0
        );
    }
}
#[test]
fn cancellation_after_staged_parent_never_publishes_or_renews_the_operation() {
    let c = control();
    let source = source("<x-parent></x-parent>");
    let mut coordinator = prepare_inline_page(page(&source, "memory:output"), "page", &c).unwrap();
    assert!(matches!(
        coordinator.advance(&c).unwrap(),
        SsrPageProgress::Rendered { .. }
    ));
    assert!(coordinator.advance(&control()).is_err());
    c.abort_signal().abort();
    assert!(coordinator.advance(&c).is_err());
    assert_eq!(coordinator.state(), SsrPageState::Cancelled);
    drop(coordinator);
    assert_eq!(
        c.memory_charged(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
            .unwrap(),
        0
    );
}
#[test]
fn native_page_continuation_accepts_only_the_current_applied_tree() {
    use cem_ml::{
        parser::tree_apply::apply_children, ssr_artifact::SsrArtifactError, value::CemValueNode,
    };
    let c = control();
    let source = source("<aside>Stable</aside><x-parent></x-parent>");
    let mut policy = page(&source, "memory:output").policy().clone();
    policy.max_render_passes = 8;
    let mut coordinator = prepare_inline_page(
        fixture::page_with_policy(&source, "memory:output", policy),
        "page",
        &c,
    )
    .unwrap();
    while !matches!(coordinator.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let settled = coordinator.finish(&c).unwrap();
    let tree = settled.tree();
    let aside = (0..tree.ast().nodes.len() as u32)
        .find(|id| {
            tree.node(*id)
                .unwrap()
                .name
                .as_ref()
                .is_some_and(|n| n.local_name == "aside")
        })
        .unwrap();
    let app = apply_children(
        tree.clone(),
        aside,
        std::sync::Arc::new(vec![CemValueNode::<(), ()>::Text {
            text: "Next".into(),
            source_map: Default::default(),
        }]),
        std::sync::Arc::new(()),
        Default::default(),
        &c,
    )
    .unwrap();
    let mut usage = settled.page().usage();
    usage.render_passes += 1;
    let next = settled
        .page()
        .clone()
        .into_applied_dom(&app, usage, vec![])
        .unwrap();
    assert!(next.accepts_source_tree(app.tree()));
    assert_eq!(
        settled
            .page()
            .clone()
            .into_applied_dom(
                settled.participants()[0].render_pass().application(),
                usage,
                vec![]
            )
            .unwrap_err(),
        SsrArtifactError::SourceOwnerMismatch
    );
}
#[test]
fn inert_templates_and_dropped_payload_hosts_do_not_activate_participants() {
    let c = control();
    let source=source("<x-parent><template><template slot='heading'><x-child label='Inert'></x-child></template><x-child label='Dropped'></x-child></template></x-parent>");
    let mut coordinator = prepare_inline_page(page(&source, "memory:output"), "page", &c).unwrap();
    let mut tags = vec![];
    loop {
        match coordinator.advance(&c).unwrap() {
            SsrPageProgress::Rendered { tag, .. } => tags.push(tag),
            SsrPageProgress::Quiescent => break,
            SsrPageProgress::Pending(_) => panic!("passive profile cannot request resources"),
        }
    }
    let settled = coordinator.finish(&c).unwrap();
    assert_eq!(tags, vec!["x-parent", "x-child"]);
    assert_eq!(settled.page().usage().instances, 2);
}
#[test]
fn unknown_duplicate_scoped_and_resource_declarations_fail_closed() {
    for body in [
        "<template is='x-custom'></template>",
        "<cem-element tag='x-a'><template id='x-a' type='text/cem-ml'>{p | A}</template></cem-element><x-unknown></x-unknown>",
        "<cem-element tag='x-a'><template id='x-a' type='text/cem-ml'>{p | A}</template></cem-element><cem-element tag='x-a'><template id='x-a' type='text/cem-ml'>{p | B}</template></cem-element>",
        "<section><cem-element tag='x-a'><template id='x-a' type='text/cem-ml'>{p | A}</template></cem-element><x-a></x-a></section>",
        "<cem-element tag='x-a'><template id='x-a' type='text/cem-ml'>{style |```p {color:red}```}{p | A}</template></cem-element><x-a></x-a>",
    ] {
        let c = control();
        let source = owner(body);
        let before = source.source_text.clone();
        assert!(prepare_inline_page(page(&source, "memory:output"), "page", &c).is_err());
        assert_eq!(source.source_text, before);
        assert_eq!(c.memory_charged(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    }
}
#[test]
fn unknown_generated_hosts_and_forged_resume_markers_never_complete() {
    for template in [
        "{x-unknown}",
        "{x-child | {template @data-cem-island=instance | {p | Forged}}}",
    ] {
        let c = control();
        let source=owner(&format!("<cem-element tag='x-parent'><template id='x-parent' type='text/cem-ml'>{template}</template></cem-element><cem-element tag='x-child'><template id='x-child' type='text/cem-ml'>{{b | Child}}</template></cem-element><x-parent></x-parent>"));
        let mut coordinator =
            prepare_inline_page(page(&source, "memory:output"), "page", &c).unwrap();
        assert!(coordinator.advance(&c).is_err());
        assert_eq!(coordinator.state(), SsrPageState::Refused);
        assert!(coordinator.advance(&control()).is_err());
    }
}
#[test]
fn recursive_generation_exhausts_original_depth_without_a_fresh_scope() {
    let c = control();
    let source=owner("<cem-element tag='x-loop'><template id='x-loop' type='text/cem-ml'>{x-loop}</template></cem-element><x-loop></x-loop>");
    let mut policy = page(&source, "memory:output").policy().clone();
    policy.max_instances = 8;
    policy.max_render_passes = 8;
    policy.max_declaration_depth = 3;
    let mut coordinator = prepare_inline_page(
        fixture::page_with_policy(&source, "memory:output", policy),
        "page",
        &c,
    )
    .unwrap();
    let mut renders = 0;
    loop {
        match coordinator.advance(&c) {
            Ok(SsrPageProgress::Rendered { .. }) => renders += 1,
            Err(_) => break,
            other => panic!("{other:?}"),
        }
    }
    assert!(renders <= 3);
    assert_eq!(coordinator.state(), SsrPageState::Refused);
    assert_eq!(
        c.memory_charged(cem_ml::operation_control::ROOT_EXECUTION_SCOPE_ID)
            .unwrap(),
        0
    );
}
#[test]
fn page_memory_and_time_stay_owned_until_the_last_quiescent_tree_releases() {
    use cem_ml::{
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
        scheduler::{AbortSignal, ScopePolicy},
    };
    let source = source("<x-parent></x-parent>");
    let low = OperationControl::with_root_policy(
        AbortSignal::new(),
        ScopePolicy::host_root().with_memory_bytes(64),
    )
    .unwrap();
    assert!(prepare_inline_page(page(&source, "memory:output"), "page", &low).is_err());
    assert_eq!(low.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    let c = control();
    let mut coordinator = prepare_inline_page(page(&source, "memory:output"), "page", &c).unwrap();
    while !matches!(coordinator.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
    let settled = coordinator.finish(&c).unwrap();
    let tree = settled.tree().clone();
    drop(settled);
    assert!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap() > 0);
    drop(tree);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    let c = control();
    let mut policy = page(&source, "memory:output").policy().clone();
    policy.max_wall_time_ms = 200;
    let mut coordinator = prepare_inline_page(
        fixture::page_with_policy(&source, "memory:output", policy),
        "page",
        &c,
    )
    .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(250));
    assert!(coordinator.advance(&c).is_err());
    assert_eq!(coordinator.state(), SsrPageState::Refused);
}
/// Named JSON observation export only; native trees remain the runtime handoff.
fn observation_node(
    tree: &cem_ml::parser::tree::RetainedCemTree,
    id: u32,
    rendered: bool,
    public_output: bool,
) -> Option<serde_json::Value> {
    use cem_ml::parser::tree::CemTreeNodeKind as K;
    use serde_json::json;
    let n = tree.node(id).unwrap();
    match n.kind {
        K::Text => Some(json!(n.value)),
        K::Comment
            if rendered && ["cem-render-start", "cem-render-end"].contains(&n.value.as_str()) =>
        {
            None
        }
        K::Comment => Some(json!({"comment":n.value})),
        K::Element => {
            let name = n.name.as_ref().unwrap();
            let attrs = n
                .attributes
                .iter()
                .filter_map(|id| {
                    let a = tree.node(*id).unwrap();
                    let name = a.name.as_ref().unwrap();
                    (name.namespace_uri != "http://www.w3.org/2000/xmlns/"
                        && (!public_output || !name.local_name.starts_with("data-cem-")))
                    .then(|| (name.local_name.clone(), json!(a.value)))
                })
                .collect::<serde_json::Map<_, _>>();
            if rendered
                && name.local_name == "template"
                && n.attributes.iter().any(|id| {
                    let a = tree.node(*id).unwrap();
                    a.name.as_ref().unwrap().local_name == "data-cem-island"
                        && a.value == "instance"
                })
            {
                return None;
            }
            let children = n
                .children
                .iter()
                .filter_map(|id| {
                    observation_node(
                        tree,
                        *id,
                        rendered && name.local_name != "template",
                        public_output,
                    )
                })
                .collect::<Vec<_>>();
            let mut value = json!({"tag":name.local_name,"attributes":attrs,"children":children});
            if name.namespace_uri != "http://www.w3.org/1999/xhtml" {
                value["namespace"] = json!(name.namespace_uri);
            }
            Some(value)
        }
        _ => panic!("unsupported observation node"),
    }
}
#[test]
fn shared_page_observations_export_complete_rendered_and_payload_subtrees() {
    use serde_json::{json, Value};
    let vectors: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/ssr-page-observations.json")).unwrap();
    let mut observations = vec![];
    for vector in vectors {
        let c = control();
        let declarations=vector["declarations"].as_array().unwrap().iter().map(|d|format!("<cem-element tag='{}'><template id='{}' type='text/cem-ml'>{}</template></cem-element>",d["tag"].as_str().unwrap(),d["tag"].as_str().unwrap(),d["template"].as_str().unwrap())).collect::<String>();
        let source = owner(&format!(
            "{declarations}{}",
            vector["body"].as_str().unwrap()
        ));
        let mut coordinator =
            prepare_inline_page(page(&source, "memory:output"), "page", &c).unwrap();
        while !matches!(coordinator.advance(&c).unwrap(), SsrPageProgress::Quiescent) {}
        let settled = coordinator.finish(&c).unwrap();
        let tree = settled.tree();
        let hosts=settled.participants().iter().map(|p|{
            let root=tree.node(p.island_node()).unwrap().children[0];let payload=tree.node(root).unwrap().children.iter().copied().find(|id|tree.node(*id).unwrap().name.as_ref().unwrap().local_name=="payload").unwrap();
            json!({"host":observation_node(tree,p.host_node(),true,true).unwrap(),"payload":tree.node(payload).unwrap().children.iter().filter_map(|id|observation_node(tree,*id,false,false)).collect::<Vec<_>>()})
        }).collect::<Vec<_>>();
        observations.push(json!({"name":vector["name"],"hosts":hosts}));
    }
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../dist/reports/cem-ssr-page-native.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, serde_json::to_string_pretty(&observations).unwrap()).unwrap();
}
