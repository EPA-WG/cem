#[path = "support/html_base_fixture.rs"]
mod fixture;

#[test]
fn html_base_shared_chromium_cases() {
    assert_eq!(fixture::run_cases().len(), 24);
}

#[test]
fn html_base_unevaluated_attribute_is_not_an_empty_href() {
    use cem_ml::html_base::*;
    let tree = cem_ml::import::import_data_bytes(
        br#"<html xmlns="http://www.w3.org/1999/xhtml" xmlns:cem="https://cem.dev/ns/core/1"><base href="{$base}" cem:expression-attributes="href"/></html>"#,
        "application/xml", "cem", "studio://fixture/snapshot.xhtml",
    ).unwrap();
    let result = resolve_html_base_snapshot(
        tree,
        HtmlBaseContext {
            fallback: HtmlFallbackBase::DocumentUrl("https://example.test/page.html".into()),
            encoding: HtmlBaseEncoding::Utf8,
            policy_stamp: "fixture:allow".into(),
        },
        HtmlBaseLimits {
            nodes: 100,
            url_bytes: 4096,
        },
        |_| panic!("unevaluated attribute"),
    );
    assert!(result.unwrap_err().contains("materialized attribute"));
}
