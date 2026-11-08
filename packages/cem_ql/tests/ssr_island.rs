use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    source_map::TransformKind,
};
use cem_ql::{
    eval::{retained_cem_node, AtomValue, Item, QueryContextScope},
    ssr_island::{prepare_inline_island, PreparedInlineIsland},
    ssr_preparation::{prepare_inline_dce, PreparedInlineDce},
};
use std::{collections::BTreeMap, sync::Arc};
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
const DECL: &str = "<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{p | Hello}</template></cem-element>";
fn prepared(body: &str, control: &OperationControl) -> PreparedInlineDce {
    let source = owner(&format!("{DECL}{body}"));
    prepare_inline_dce(page(&source, "memory:output"), control).unwrap()
}
fn capture(body: &str) -> PreparedInlineIsland {
    prepare_inline_island(prepared(body, &control()), "inline-one", &control()).unwrap()
}
fn field(item: &Item, name: &str) -> String {
    match item.view().unwrap().field(name).unwrap()[0].atom().unwrap() {
        AtomValue::String(s) => s,
        value => panic!("expected string: {value:?}"),
    }
}
fn children(item: &Item) -> Vec<Item> {
    item.view().unwrap().field("children").unwrap()
}
fn attrs(item: &Item) -> BTreeMap<String, String> {
    item.view()
        .unwrap()
        .field("attributes")
        .unwrap()
        .iter()
        .map(|a| (field(a, "name"), field(a, "value")))
        .collect()
}
fn parts(island: &PreparedInlineIsland) -> Vec<Item> {
    children(island.context_root())
}
fn frame(island: &PreparedInlineIsland) -> &BTreeMap<String, Vec<Item>> {
    let [Item::Record(fields)] = island.bindings().bindings["datadom"].items.as_slice() else {
        panic!("native input frame")
    };
    fields
}
fn error(body: &str) -> Vec<cem_ml::diagnostics::Diagnostic> {
    prepare_inline_island(prepared(body, &control()), "inline-one", &control()).unwrap_err()
}
#[test]
fn canonical_topology_is_inert_and_keeps_original_page_and_payload_owners() {
    let source = owner(&format!(
        "{DECL}<x-inline label='Original'><b>Payload</b></x-inline>"
    ));
    let original = source.source_text.clone();
    let initial = prepare_inline_dce(page(&source, "memory:output"), &control()).unwrap();
    let tree = initial.elements().tree.clone();
    let ids = tree.node(initial.instance_node()).unwrap().children.clone();
    let island = prepare_inline_island(initial, "inline-one", &control()).unwrap();
    assert!(Arc::ptr_eq(
        island.prepared().page().source_document(),
        &source
    ));
    assert!(Arc::ptr_eq(&island.prepared().elements().tree, &tree));
    assert_eq!(source.source_text, original);
    assert_eq!(field(island.template(), "name"), "template");
    assert_eq!(
        field(island.template(), "namespace"),
        "http://www.w3.org/1999/xhtml"
    );
    assert_eq!(attrs(island.template())["data-cem-island"], "instance");
    assert_eq!(children(island.template()).len(), 1);
    assert_eq!(field(island.context_root(), "name"), "context-root");
    assert_eq!(attrs(island.context_root())["version"], "0.2.0");
    let names: Vec<_> = parts(&island).iter().map(|p| field(p, "name")).collect();
    assert_eq!(
        names,
        [
            "data",
            "attributes",
            "dataset",
            "payload",
            "slices",
            "resources",
            "form-state",
            "validation-state",
            "event-state"
        ]
    );
    assert_eq!(
        island
            .payload()
            .iter()
            .map(|n| n.node_id())
            .collect::<Vec<_>>(),
        ids
    );
    assert!(island
        .payload()
        .iter()
        .all(|n| Arc::ptr_eq(n.owner(), &tree)));
    assert_eq!(island.prepared().page().usage().render_passes, 0);
    assert!(!island
        .prepared()
        .page()
        .adoption_identity()
        .hydration_ready());
    assert_eq!(island.instance_id(), "inline-one");
}
#[test]
fn live_payload_reference_targets_preserve_source_identity_parents_maps_and_slot_order() {
    let island=capture("<x-inline>Leading<!--note--><b slot='heading'>First</b><span slot='heading'>Second<i slot='nested'>Inner</i></span>Tail</x-inline>");
    let [Item::Record(slots)] = frame(&island)["slots"].as_slice() else {
        panic!()
    };
    assert_eq!(slots["heading"].len(), 2);
    assert!(!slots.contains_key("nested"));
    assert_eq!(slots[""].len(), 3);
    let occurrences = children(&parts(&island)[3]);
    assert_eq!(occurrences.len(), island.payload().len());
    for (occurrence, source) in occurrences.iter().zip(island.payload()) {
        assert_eq!(field(occurrence, "kind"), "reference");
        let target = occurrence
            .view()
            .unwrap()
            .field("targets")
            .unwrap()
            .remove(0);
        let retained = retained_cem_node(&target).unwrap();
        assert!(Arc::ptr_eq(retained.owner(), source.owner()));
        assert_eq!(retained.node_id(), source.node_id());
        assert_eq!(target.identity(), source.query_item().identity());
        let parent = target
            .view()
            .unwrap()
            .parent(QueryContextScope(0))
            .unwrap()
            .unwrap();
        assert_eq!(
            retained_cem_node(&parent).unwrap().node_id(),
            island.prepared().instance_node()
        );
        assert!(target.source_map().unwrap().frames.iter().any(|f|matches!(&f.transform,TransformKind::SourceBoundary{uri} if uri=="file:///source/page.html")));
        assert_eq!(
            target
                .view()
                .unwrap()
                .provenance()
                .unwrap()
                .source_uri
                .as_deref(),
            Some("file:///source/page.html")
        );
    }
}
#[test]
fn explicit_html_envelope_captures_only_content_and_retains_envelope_identity() {
    let island=capture("<x-inline> \n<template><b slot='heading'>Title</b><!--inside-->Text<template><i>Inert</i></template></template>\n </x-inline>");
    let envelope = island.payload_envelope().unwrap();
    assert_eq!(
        envelope
            .owner()
            .node(envelope.node_id())
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name,
        "template"
    );
    let ids = &envelope.owner().node(envelope.node_id()).unwrap().children;
    assert_eq!(
        island
            .payload()
            .iter()
            .map(|n| n.node_id())
            .collect::<Vec<_>>(),
        *ids
    );
    assert_eq!(
        attrs(&parts(&island)[3])["data-cem-explicit-payload"],
        "true"
    );
    assert_eq!(island.payload().len(), 4);
}
#[test]
fn empty_live_and_explicit_payloads_initialize_all_empty_domains() {
    for body in [
        "<x-inline></x-inline>",
        "<x-inline><template></template></x-inline>",
    ] {
        let island = capture(body);
        assert!(island.payload().is_empty());
        for part in &parts(&island)[4..] {
            assert!(children(part).is_empty());
        }
    }
}
#[test]
fn attributes_and_domstringmap_entries_keep_encounter_order_and_literal_values() {
    let island=capture("<x-inline title='A &amp; B' data-fruit-name='apple' data--foo='one' data-foo.bar='two' data-='empty' constructor='literal' __proto__='ordinary'></x-inline>");
    let entries = children(&parts(&island)[1]);
    assert_eq!(
        entries
            .iter()
            .map(|a| attrs(a)["name"].clone())
            .collect::<Vec<_>>(),
        [
            "title",
            "data-fruit-name",
            "data--foo",
            "data-foo.bar",
            "data-",
            "constructor",
            "__proto__"
        ]
    );
    let data = &frame(&island)["dataset"][0];
    for (name, value) in [
        ("fruitName", "apple"),
        ("Foo", "one"),
        ("foo.bar", "two"),
        ("", "empty"),
    ] {
        assert_eq!(
            data.view().unwrap().field(name).unwrap()[0].atom(),
            Some(AtomValue::String(value.into()))
        );
    }
}
#[test]
fn ambiguous_payloads_and_resume_are_refused_without_rewriting_source() {
    for body in [
        "<x-inline><template></template><p>Sibling</p></x-inline>",
        "<x-inline><template></template><template></template></x-inline>",
        "<x-inline><!--meaningful--><template></template></x-inline>",
    ] {
        assert_eq!(error(body)[0].code, "cem-element.instance_payload_mixed");
    }
    let source=owner(&format!("{DECL}<x-inline><template data-cem-island='instance'><p>State</p></template><p>Derived</p></x-inline>"));
    let before = source.source_text.clone();
    assert_eq!(
        prepare_inline_dce(page(&source, "memory:output"), &control()).unwrap_err()[0].code,
        "cem.ssr.resume_unavailable"
    );
    assert_eq!(source.source_text, before);
}
#[test]
fn unsupported_source_payload_adapters_foreign_nodes_and_styles_fail_closed() {
    for body in [
        "<x-inline><template type='application/json'>{}</template></x-inline>",
        "<x-inline><svg><circle></circle></svg></x-inline>",
        "<x-inline><template><style>p {color:red}</style></template></x-inline>",
    ] {
        assert_eq!(error(body)[0].code, "cem.ssr.payload_profile_unsupported");
    }
}
#[test]
fn native_domain_and_sensitive_input_refusals_remain_in_force() {
    for body in [
        "<x-inline><input type='password'></x-inline>",
        "<x-inline autocomplete='CURRENT-PASSWORD'></x-inline>",
        "<x-inline><input autocomplete='one-time-code'></x-inline>",
        "<x-inline datadom='reserved'></x-inline>",
    ] {
        assert_eq!(error(body)[0].code, "cem.ssr.island_admission_refused");
    }
}
#[test]
fn original_control_cancellation_and_completion_are_not_reset_by_capture() {
    for completed in [false, true] {
        let c = control();
        let prepared = prepared("<x-inline><p>Original</p></x-inline>", &c);
        let source = prepared.page().source_document().clone();
        let before = source.source_text.clone();
        if completed {
            c.complete_scope(ROOT_EXECUTION_SCOPE_ID).unwrap()
        } else {
            c.abort_signal().abort()
        };
        assert_eq!(
            prepare_inline_island(prepared, "inline-one", &c).unwrap_err()[0].code,
            if completed {
                "cem.control.scope_completed"
            } else {
                "host-cancellation"
            }
        );
        assert_eq!(source.source_text, before);
    }
}
#[test]
fn native_island_limits_and_instance_identity_are_checked_before_publication() {
    let body = format!("<x-inline>{}</x-inline>", "<i></i>".repeat(16_385));
    assert_eq!(error(&body)[0].code, "cem.ssr.island_bounds");
    for id in ["", &"x".repeat(257)] {
        assert_eq!(
            prepare_inline_island(
                prepared("<x-inline></x-inline>", &control()),
                id,
                &control()
            )
            .unwrap_err()[0]
                .code,
            "cem.ssr.instance_identity_invalid"
        );
    }
}
#[test]
fn output_destination_does_not_change_capture_identity_or_hydration_fields() {
    let source = owner(&format!("{DECL}<x-inline><p>Original</p></x-inline>"));
    let mut fields = vec![];
    for destination in ["file:///one/page.html", "file:///two/page.html"] {
        let p = prepare_inline_dce(page(&source, destination), &control()).unwrap();
        let island = prepare_inline_island(p, "inline-one", &control()).unwrap();
        fields.push(
            children(&parts(&island)[0])
                .iter()
                .map(|f| (attrs(f)["name"].clone(), field(&children(f)[0], "value")))
                .collect::<BTreeMap<_, _>>(),
        );
    }
    assert_eq!(fields[0], fields[1]);
    assert_eq!(fields[0]["instanceId"], "inline-one");
    assert!(!fields[0].contains_key("hydrationReady"));
}

#[test]
fn original_node_provenance_stays_separate_from_deployed_document_metadata() {
    let p = prepared("<x-inline><p>Original</p></x-inline>", &control());
    let tree = p.elements().tree.clone();
    let id = tree.node(p.instance_node()).unwrap().children[0];
    let source = cem_ql::eval::RetainedCemNode::new(tree.clone(), id)
        .unwrap()
        .query_item();
    assert_eq!(
        source
            .view()
            .unwrap()
            .provenance()
            .unwrap()
            .source_uri
            .as_deref(),
        Some("file:///source/page.html")
    );
    assert_eq!(tree.document_uri(), Some("https://ssr.example.test/guide/"));
}

#[test]
fn transformed_payload_origins_survive_and_unmapped_sources_are_not_fabricated() {
    use cem_ml::{
        interpreter::OutputSpan,
        source::{ByteRange, SourceId},
        source_map::{FrameSpan, SourceMapFrame, SourceMapIndex, SourceMapStack},
    };
    let original = owner(&format!("{DECL}<x-inline><p>Original</p></x-inline>"));
    for mapped in [true, false] {
        let mut native = (*original).clone();
        let origin = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange::new(77, 11)),
                transform: TransformKind::CemAstBuilder,
            }],
        }
        .with_source_uri("https://authored.example.test/original.cem");
        native.origin_spans = Some(SourceMapIndex::new(if mapped {
            vec![OutputSpan {
                output_range: ByteRange::new(0, native.source_text.len() as u32),
                origin: origin.clone(),
            }]
        } else {
            vec![]
        }));
        let source = Arc::new(native);
        let p = prepare_inline_dce(page(&source, "memory:output"), &control()).unwrap();
        let result = prepare_inline_island(p, "inline-one", &control());
        if mapped {
            let island = result.unwrap();
            let payload = island.payload()[0].query_item();
            assert_eq!(payload.source_map().unwrap(), origin);
            assert_eq!(
                payload
                    .view()
                    .unwrap()
                    .provenance()
                    .unwrap()
                    .source_uri
                    .as_deref(),
                Some("https://authored.example.test/original.cem")
            );
            assert_eq!(island.context_root().source_map().unwrap(), origin);
            assert_eq!(
                island
                    .context_root()
                    .view()
                    .unwrap()
                    .provenance()
                    .unwrap()
                    .line_number,
                None
            );
        } else {
            assert_eq!(
                result.unwrap_err()[0].code,
                "cem.ssr.island_admission_refused"
            );
            assert!(source
                .source_map_for_range(cem_ml::validation::html::HtmlSourceRange {
                    start: cem_ml::validation::html::HtmlSourcePosition {
                        line: 1,
                        column: 1,
                        byte_offset: 0
                    },
                    byte_length: source.source_text.len() as u64
                })
                .is_none());
        }
    }
}
#[test]
fn caller_memory_and_stack_limits_can_only_reduce_native_capture_limits() {
    use cem_ml::scheduler::{AbortSignal, ScopePolicy};
    for policy in [
        ScopePolicy::host_root().with_memory_bytes(64),
        ScopePolicy::host_root().with_stack_depth(1),
    ] {
        let c = OperationControl::with_root_policy(AbortSignal::new(), policy).unwrap();
        let p = prepared("<x-inline><p>Original</p></x-inline>", &c);
        assert_eq!(
            prepare_inline_island(p, "inline-one", &c).unwrap_err()[0].code,
            "cem.ssr.island_bounds"
        );
    }
}

#[test]
fn shared_browser_capture_vectors_use_the_same_native_payload_nodes_and_shapes() {
    // Explicit test-control input, never a runtime AST handoff.
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/ssr-island-capture.json")).unwrap();
    for vector in vectors.as_array().unwrap() {
        let result = prepare_inline_island(
            prepared(vector["body"].as_str().unwrap(), &control()),
            "inline-one",
            &control(),
        );
        if vector["mode"] == "mixed" {
            assert_eq!(
                result.unwrap_err()[0].code,
                "cem-element.instance_payload_mixed",
                "{}",
                vector["name"]
            );
        } else {
            let island = result.unwrap();
            assert_eq!(
                island.payload_envelope().is_some(),
                vector["mode"] == "html-envelope"
            );
            let names = island
                .payload()
                .iter()
                .map(|n| {
                    let n = n.owner().node(n.node_id()).unwrap();
                    match n.kind {
                        cem_ml::parser::tree::CemTreeNodeKind::Text => "#text".into(),
                        cem_ml::parser::tree::CemTreeNodeKind::Comment => "#comment".into(),
                        _ => n.name.as_ref().unwrap().local_name.clone(),
                    }
                })
                .collect::<Vec<String>>();
            let expected = vector["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_owned())
                .collect::<Vec<_>>();
            assert_eq!(names, expected, "{}", vector["name"]);
        }
    }
}
