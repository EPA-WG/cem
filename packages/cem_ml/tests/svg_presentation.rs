use cem_ml::document_dependencies::inspect_document_dependencies;
use std::sync::Arc;
#[path = "support/stylesheet_fixture.rs"]
mod fixture;
use fixture::*;
#[test]
fn presentation_values_use_native_css_tokens_and_confirmed_html_base() {
    let (mut host, inventory, cap) = setup(
        r##"<svg xmlns='http://www.w3.org/2000/svg' xml:base='https://wrong.test/'><path fill='URL(&quot;./a\20 b.svg#paint&quot;) red' filter='url(./filter.svg#f)' clip-path='url(#clip)' mask='url(icon)' marker-start='url(./marker.svg#m)' cursor='url(./cursor.svg), auto'/></svg>"##,
    );
    assert_eq!(inventory.svg_presentations().len(), 6);
    let plans = inventory.svg_presentations();
    assert_eq!(
        plans[0].resources.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://page.test/assets/a%20b.svg#paint"
    );
    assert_eq!(
        plans[3].resources.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://maps.test/mapped.svg"
    );
    assert!(Arc::ptr_eq(
        plans[0].source.document(),
        &inventory.document().tree
    ));
    let range = plans[0]
        .source
        .decoded_value_range(plans[0].resources.references[0].range)
        .unwrap();
    assert!(plans[0].source.value()[range].starts_with("URL("));
    assert!(!plans[0].source.source().frames.is_empty());
    assert!(!inventory
        .blockers()
        .iter()
        .any(|b| b == "svg-presentation-analysis-required"));
    host.validate_dependencies(&inventory, &cap, slots, BASE)
        .unwrap();
    host.invalidate().unwrap();
    assert!(host
        .validate_dependencies(&inventory, &cap, slots, BASE)
        .is_err());
}
#[test]
fn injection_malformed_important_and_budget_do_not_publish_partial_analysis() {
    for value in [
        "url(a);stroke:url(b)",
        "url(a);",
        "url('bad)",
        "url(a)!important",
    ] {
        let source =
            format!("<svg xmlns='http://www.w3.org/2000/svg'><path fill=\"{value}\"/></svg>");
        let tree = cem_ml::import::import_data_bytes(
            source.as_bytes(),
            "application/xml",
            "cem",
            "https://example.test/a.svg",
        )
        .unwrap();
        let result = inspect_document_dependencies(tree, 1000);
        assert!(
            result
                .blockers
                .iter()
                .any(|b| b == "svg-presentation-analysis-incomplete"),
            "{value}: {:?}",
            result.blockers
        );
    }
    let tree = cem_ml::import::import_data_bytes(
        b"<svg xmlns='http://www.w3.org/2000/svg'><path fill='url(a.svg)'/></svg>",
        "application/xml",
        "cem",
        "https://example.test/a.svg",
    )
    .unwrap();
    let result = inspect_document_dependencies(tree, 4);
    assert!(result.blockers.iter().any(|b| b.contains("budget")));
}
#[test]
fn namespace_smil_and_non_url_values_keep_correct_boundaries() {
    let (_,inventory,_)=setup("<svg xmlns='http://www.w3.org/2000/svg'><path fill='currentColor' stroke='none'/><animate fill='freeze'/><foreign xmlns='urn:foreign' fill='url(secret)'/></svg>");
    assert_eq!(inventory.svg_presentations().len(), 2);
    assert!(inventory
        .svg_presentations()
        .iter()
        .all(|p| p.resources.references.is_empty()));
    assert!(inventory
        .blockers()
        .iter()
        .any(|b| b == "svg-animation-runtime-analysis-required"));
}
#[test]
fn imported_xml_bases_and_denied_svg_resources_remain_native() {
    let (_, _, cap) = setup("");
    let result=cem_ml::document_stylesheet_dependencies::inspect_xml_document_stylesheets(b"<svg xmlns='http://www.w3.org/2000/svg' xml:base='../paint/'><path fill='url(a.svg)'/></svg>",&cap,"https://page.test/docs/a.svg",1000).unwrap();
    assert_eq!(
        result.presentations[0].resources.references[0]
            .resolution
            .as_ref()
            .unwrap()
            .resolved_url,
        "https://page.test/paint/a.svg"
    );
    let (_,inventory,_)=setup("<svg xmlns='http://www.w3.org/2000/svg'><path fill='url(https://page.test/assets/denied.css)'/></svg>");
    assert!(inventory.svg_presentations()[0].resources.references[0]
        .resolution
        .is_err());
    assert!(inventory
        .blockers()
        .iter()
        .any(|b| b == "svg-presentation-resolution-incomplete"));
}

#[test]
fn direct_presentation_import_requires_owned_slots_and_bounded_values() {
    use cem_ml::import::svg_presentation::import_svg_presentation_attribute;
    let (_, inventory, _) =
        setup("<svg xmlns='http://www.w3.org/2000/svg'><path fill='url(a.svg)'/></svg>");
    let source = &inventory.svg_presentations()[0].source;
    assert!(import_svg_presentation_attribute(
        source.document().clone(),
        0,
        source.attribute(),
        1000
    )
    .is_err());
    assert!(import_svg_presentation_attribute(
        source.document().clone(),
        source.owner(),
        source.attribute(),
        3
    )
    .is_err());
    let copy = import_svg_presentation_attribute(
        source.document().clone(),
        source.owner(),
        source.attribute(),
        1000,
    )
    .unwrap();
    assert!(Arc::ptr_eq(copy.document(), source.document()));
    assert!(!Arc::ptr_eq(copy.css(), source.css()));
}
