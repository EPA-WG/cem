use cem_ql::{
    ssr_island::prepare_inline_island, ssr_preparation::prepare_inline_dce,
    ssr_render::render_and_apply_inline_dce,
};
#[path = "support/ssr_page.rs"]
mod fixture;
use fixture::{control, owner, page};
fn source(payload: &str) -> std::sync::Arc<cem_ml::validation::html::HtmlDocumentAst> {
    owner(&format!("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{{section | {{$ datadom.slots.heading}}}}</template></cem-element><x-inline>{payload}</x-inline>"))
}
#[test]
fn passive_foreign_payload_preserves_namespaces_case_and_original_owner() {
    let c = control();
    let source=source("<svg slot='heading' viewBox='0 0 24 24'><linearGradient id='paint'></linearGradient><circle r='4'></circle></svg><math slot='heading'><mi>x</mi><mo>+</mo><mn>1</mn></math>");
    let prepared = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let original = prepared.elements().tree.clone();
    let island = prepare_inline_island(prepared, "inline-one", &c).unwrap();
    assert!(std::sync::Arc::ptr_eq(
        island.payload()[0].owner(),
        &original
    ));
    let svg = original.node(island.payload()[0].node_id()).unwrap();
    assert_eq!(
        svg.name.as_ref().unwrap().namespace_uri,
        "http://www.w3.org/2000/svg"
    );
    assert!(svg.attributes.iter().any(|id| original
        .node(*id)
        .unwrap()
        .name
        .as_ref()
        .unwrap()
        .local_name
        == "viewBox"));
    assert_eq!(
        original
            .node(svg.children[0])
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name,
        "linearGradient"
    );
    let result = render_and_apply_inline_dce(island, &c).unwrap();
    let tree = result.application().tree();
    let section = tree
        .node(result.application().inserted_children()[2])
        .unwrap();
    assert_eq!(section.children.len(), 2);
    for (id, ns) in section.children.iter().zip([
        "http://www.w3.org/2000/svg",
        "http://www.w3.org/1998/Math/MathML",
    ]) {
        assert_eq!(
            tree.node(*id).unwrap().name.as_ref().unwrap().namespace_uri,
            ns
        );
        assert!(tree.node_source_map(*id).unwrap().frames.iter().any(|f| matches!(&f.transform,cem_ml::source_map::TransformKind::SourceBoundary{uri} if uri=="file:///source/page.html")));
    }
    assert!(!result.page().adoption_identity().hydration_ready());
}
#[test]
fn explicit_instance_css_is_retained_scoped_and_published_outside_render_range() {
    let c = control();
    let source=source("<template><style>.title { color:red } @keyframes pulse {from {opacity:0} to {opacity:1}}</style><b class='title' slot='heading'>Title</b></template>");
    let before = source.source_text.clone();
    let p = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    let result = render_and_apply_inline_dce(island, &c).unwrap();
    assert_eq!(source.source_text, before);
    let tree = result.application().tree();
    let ids = result.application().inserted_children();
    assert_eq!(
        tree.node(ids[ids.len() - 2]).unwrap().value,
        "cem-render-end"
    );
    let style = tree.node(*ids.last().unwrap()).unwrap();
    assert_eq!(style.name.as_ref().unwrap().local_name, "style");
    assert!(style.attributes.iter().any(|id| tree
        .node(*id)
        .unwrap()
        .name
        .as_ref()
        .unwrap()
        .local_name
        == "data-cem-instance-style"));
    let css = tree
        .text_fragments(*ids.last().unwrap())
        .unwrap()
        .collect::<String>();
    assert!(css.starts_with("@scope to ("), "{css}");
    assert!(css.contains("@keyframes pulse-"));
    assert!(!css.contains("data-cem-render-scope"));
    assert!(!result.page().adoption_identity().hydration_ready());
}
#[test]
fn stylesheet_source_maps_and_lifetime_follow_the_applied_native_tree() {
    use cem_ml::{
        operation_control::ROOT_EXECUTION_SCOPE_ID,
        source_map::{FrameSpan, TransformKind},
    };
    let c = control();
    let source = source(
        "<template><style>\n.title {color:red}</style><b slot='heading'>Title</b></template>",
    );
    let offset = source.source_text.find(".title").unwrap() as u64;
    let p = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    let record = island.instance_stylesheets()[0].clone();
    let css = record.source_tree(0).unwrap();
    assert_eq!(css.source_uri(), "file:///source/page.html");
    assert!(css.ast().nodes.iter().enumerate().any(|(id,_)|css.node_source_map(id as u32).unwrap().frames.iter().any(|f|matches!((&f.transform,&f.span),(TransformKind::SourceBoundary{uri},FrameSpan::Single(range)) if uri=="file:///source/page.html" && range.start==offset))));
    let result = render_and_apply_inline_dce(island, &c).unwrap();
    assert_eq!(result.page().usage().resources, 1);
    let retained = result.application().tree().clone();
    drop(result);
    assert!(record.emission().is_ok());
    assert!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap() > 0);
    drop(retained);
    assert!(record.emission().is_err());
    drop(record);
    assert_eq!(c.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
}
#[test]
fn source_styles_are_removed_from_projected_slots_but_stay_in_the_canonical_payload() {
    let c = control();
    let source=owner("<cem-element tag='x-inline'><template id='x-inline' type='text/cem-ml'>{section | {$ datadom.slots.heading}}</template></cem-element><x-inline><template><div slot='heading'><style>p {color:red}</style><p>Visible</p></div></template></x-inline>");
    let p = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    let result = render_and_apply_inline_dce(island, &c).unwrap();
    let tree = result.application().tree();
    let ids = result.application().inserted_children();
    let section = tree.node(ids[2]).unwrap();
    let div = tree.node(section.children[0]).unwrap();
    assert_eq!(div.children.len(), 1);
    assert_eq!(
        tree.node(div.children[0])
            .unwrap()
            .name
            .as_ref()
            .unwrap()
            .local_name,
        "p"
    );
    let styles = (0..tree.ast().nodes.len() as u32)
        .filter(|id| {
            tree.node(*id)
                .unwrap()
                .name
                .as_ref()
                .is_some_and(|n| n.local_name == "style")
        })
        .count();
    assert_eq!(styles, 2, "one inert source and one managed suffix");
}
#[test]
fn nested_template_css_is_inert_and_does_not_require_a_stylesheet_closure() {
    let c = control();
    let source=source("<template><template slot='heading'><style>@import 'unloaded.css';</style><b>Inert</b></template></template>");
    let p = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    assert!(island.instance_stylesheets().is_empty());
    let result = render_and_apply_inline_dce(island, &c).unwrap();
    assert_eq!(result.page().usage().resources, 0);
}
#[test]
fn active_foreign_profiles_and_implicit_or_resource_bearing_css_are_refused_without_mutation() {
    for body in [
        "<svg><foreignObject><p>Foreign integration</p></foreignObject></svg>",
        "<math><annotation-xml encoding='text/html'><b>Foreign integration</b></annotation-xml></math>",
        "<svg><use href='https://example.test/icon.svg#x'></use></svg>",
        "<svg><animate attributeName='x'></animate></svg>",
        "<style>p {color:red}</style>",
        "<template><style>@import 'other.css';</style></template>",
        "<template><style>p {background: url(other.png)}</style></template>",
        "<template><style scope='shared'>p {color:red}</style></template>",
        "<template><style type='text/less'>p {color:red}</style></template>",
        "<template><style>p {color:red}</style><style>p {color:red}</style><style>p {color:red}</style><style>p {color:red}</style><style>p {color:red}</style></template>",
    ] {
        let c=control(); let source=source(body); let before=source.source_text.clone();
        let result=prepare_inline_dce(page(&source,"memory:output"),&c).and_then(|p|prepare_inline_island(p,"inline-one",&c));
        assert!(result.is_err(),"{body}");
        assert_eq!(source.source_text,before);
    }
}
#[test]
fn css_admission_rejects_replay_cancellation_and_lowered_native_import_limits() {
    use cem_ml::{
        operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
        scheduler::{AbortSignal, ScopePolicy},
        value::artifact::CemValueArtifactLimits,
    };
    let source =
        source("<template><style>p {color:red}</style><b slot='heading'>Title</b></template>");
    let c = control();
    let p = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let tree = p.elements().tree.clone();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    let style = island.payload()[0].node_id();
    let limited = OperationControl::with_root_policy(
        AbortSignal::new(),
        ScopePolicy::host_root().with_memory_bytes(32),
    )
    .unwrap();
    assert!(cem_ml::import::inline_css::retain_inline_css(
        tree.clone(),
        style,
        &limited,
        &Default::default()
    )
    .is_err());
    assert_eq!(limited.memory_charged(ROOT_EXECUTION_SCOPE_ID).unwrap(), 0);
    let small = CemValueArtifactLimits {
        max_bytes: 1,
        max_values: 1,
        max_depth: 1,
    };
    assert!(cem_ml::import::inline_css::retain_inline_css(tree, style, &c, &small).is_err());
    assert_eq!(
        render_and_apply_inline_dce(island, &control()).unwrap_err()[0].code,
        "cem.ssr.css_closure_stale"
    );
    let p = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    c.abort_signal().abort();
    assert!(render_and_apply_inline_dce(island, &c).is_err());
}
#[test]
fn foreign_namespaced_attributes_keep_expanded_identity_through_projection() {
    let c = control();
    let source=source("<svg slot='heading' xml:lang='en'><defs><path id='shape' d='M0 0h1'></path></defs><use xlink:href='#shape'></use></svg>");
    let p = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
    let island = prepare_inline_island(p, "inline-one", &c).unwrap();
    let result = render_and_apply_inline_dce(island, &c).unwrap();
    let tree = result.application().tree();
    let section = tree
        .node(result.application().inserted_children()[2])
        .unwrap();
    let svg = tree.node(section.children[0]).unwrap();
    assert!(svg
        .attributes
        .iter()
        .any(|id| tree
            .node(*id)
            .unwrap()
            .name
            .as_ref()
            .is_some_and(|n| n.local_name == "lang"
                && n.namespace_uri == "http://www.w3.org/XML/1998/namespace")));
    let use_node = tree.node(svg.children[1]).unwrap();
    assert!(use_node.attributes.iter().any(|id| tree
        .node(*id)
        .unwrap()
        .name
        .as_ref()
        .is_some_and(
            |n| n.local_name == "href" && n.namespace_uri == "http://www.w3.org/1999/xlink"
        )));
}
#[test]
fn shared_foreign_admission_never_converts_unknown_namespaces_or_grants_resources() {
    use cem_ml::validation::html::validate_passive_foreign_projection as admit;
    let svg = "http://www.w3.org/2000/svg";
    assert!(admit(
        svg,
        "use",
        &[("http://www.w3.org/1999/xlink", "href", "#shape")],
        false
    )
    .is_ok());
    for (ns, tag, attrs) in [
        ("urn:unregistered", "svg", vec![]),
        (svg, "script", vec![]),
        (svg, "lineargradient", vec![]),
        (
            svg,
            "use",
            vec![("", "href", "data:image/svg+xml,unadmitted")],
        ),
        (
            svg,
            "circle",
            vec![("", "fill", r"u\72l(https://example.test/paint)")],
        ),
        (svg, "circle", vec![("", "oNload", "run()")]),
        (svg, "circle", vec![("urn:unregistered", "title", "x")]),
    ] {
        assert!(admit(ns, tag, &attrs, false).is_err(), "{tag}: {attrs:?}");
    }
}
#[test]
fn html_foreign_text_integration_points_refuse_element_children() {
    for body in [
        "<svg><title><g>Integration</g></title></svg>",
        "<math><mi><mrow>Integration</mrow></mi></math>",
    ] {
        let c = control();
        let source = source(body);
        let p = prepare_inline_dce(page(&source, "memory:output"), &c).unwrap();
        assert!(
            prepare_inline_island(p, "inline-one", &c).is_err(),
            "{body}"
        );
    }
}
