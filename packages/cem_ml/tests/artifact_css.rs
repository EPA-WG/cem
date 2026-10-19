use cem_ml::artifact_css::remap_artifact_css;
use std::collections::BTreeMap;

#[test]
fn remaps_native_css_resources_without_touching_other_source() {
    let source = r#"/* café */ @import "./base.css" layer(site) supports(display:grid) screen;
.card { background: image-set("./a\20 b.png" 1x, url(../icon.svg#mark) 2x); content: "url(fake)"; --asset: url(./a\20 b.png); filter:url(#local); }"#;
    let plan = remap_artifact_css(source, false, None).unwrap();
    assert_eq!(plan.css, source);
    assert_eq!(
        plan.resources
            .iter()
            .map(|r| (r.specifier.as_str(), r.import))
            .collect::<Vec<_>>(),
        [
            ("./base.css", true),
            ("./a b.png", false),
            ("../icon.svg#mark", false),
            ("./a b.png", false),
            ("#local", false),
        ]
    );
    let mapping = BTreeMap::from([
        ("./base.css".into(), "blob:https://preview.test/base".into()),
        ("./a b.png".into(), "blob:https://preview.test/image".into()),
        (
            "../icon.svg#mark".into(),
            "blob:https://preview.test/icon#mark".into(),
        ),
    ]);
    let output = remap_artifact_css(source, false, Some(&mapping))
        .unwrap()
        .css;
    assert!(output.starts_with("/* café */ @import url(\"blob:https://preview.test/base\") layer(site) supports(display:grid) screen;"), "{output}");
    assert!(output.contains("image-set(url(\"blob:https://preview.test/image\") 1x, url(\"blob:https://preview.test/icon#mark\") 2x)"));
    assert!(output.contains("content: \"url(fake)\"; --asset: url(\"blob:https://preview.test/image\"); filter:url(#local)"));
    assert!(remap_artifact_css(source, false, Some(&BTreeMap::new()))
        .unwrap_err()
        .contains("missing_mapping"));
}

#[test]
fn inline_css_uses_the_native_declaration_list_and_escapes_targets() {
    let output = remap_artifact_css(
        "color:red;background:URL(\"./x.png\");",
        true,
        Some(&BTreeMap::from([(
            "./x.png".into(),
            "blob:quoted\"\\\n".into(),
        )])),
    )
    .unwrap();
    assert_eq!(
        output.css,
        "color:red;background:url(\"blob:quoted\\22 \\5C \\A \");"
    );
    assert!(
        remap_artifact_css(".x { background:image-set(var(--dynamic)); }", false, None).is_err()
    );
}
