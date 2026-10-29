use cem_ql::{
    declaration_closure::{build_declaration_closure, DeclarationBuildSource},
    template_artifact::TemplateArtifactSourceMapMode,
};
fn source(tag: &str, body: &str) -> DeclarationBuildSource {
    DeclarationBuildSource { source: format!("<cem-element tag=\"{tag}\"><template id=\"{tag}\" type=\"text/cem-ml\">{body}</template></cem-element>"), uri: format!("https://app.test/{tag}.xhtml") }
}
#[test]
fn derives_transitive_conditional_and_projected_roots_deterministically() {
    let build = || {
        build_declaration_closure(
            &["cem-app".into(), "cem-projected".into()],
            TemplateArtifactSourceMapMode::Prod,
            |tag| {
                Ok(match tag {
                    "cem-app" => source(tag, "{cem:if @test=true | {cem-child}}"),
                    "cem-child" => source(tag, "{cem-leaf}"),
                    _ => source(tag, "{p | Leaf}"),
                })
            },
        )
        .unwrap()
    };
    let first = build();
    let next = build();
    assert_eq!(first.loader_html, next.loader_html);
    assert_eq!(first.declarations.len(), 4);
    assert!(first.dependencies["cem-child"].contains("cem-leaf"));
    assert!(!first.loader_html.contains("xhtml"));
    assert!(!first.loader_html.contains("<script"));
    for (tag, built) in first.declarations {
        assert!(first
            .loader_html
            .contains(&format!("compiled-hash=\"{}\"", built.metadata_hash)));
        assert!(first.loader_html.contains(&format!("./{tag}.bin")));
    }
}
#[test]
fn bounds_cycles_graph_size_and_duplicate_roots() {
    let mut calls = 0;
    let closure = build_declaration_closure(
        &["cem-loop".into()],
        TemplateArtifactSourceMapMode::Dev,
        |tag| {
            calls += 1;
            Ok(source(tag, "{cem-loop}"))
        },
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(closure.declarations.len(), 1);
    assert!(build_declaration_closure(
        &["cem-a".into(), "cem-a".into()],
        TemplateArtifactSourceMapMode::Dev,
        |_| unreachable!()
    )
    .is_err());
    assert!(build_declaration_closure(
        &["../cem-a".into()],
        TemplateArtifactSourceMapMode::Dev,
        |_| unreachable!()
    )
    .is_err());
    let roots: Vec<_> = (0..65).map(|i| format!("cem-{i}")).collect();
    assert!(build_declaration_closure(
        &roots,
        TemplateArtifactSourceMapMode::Dev,
        |_| unreachable!()
    )
    .is_err());
    let mut i = 0;
    assert!(build_declaration_closure(
        &["cem-first".into()],
        TemplateArtifactSourceMapMode::Prod,
        |tag| {
            i += 1;
            Ok(source(tag, &format!("{{cem-next-{i}}}")))
        }
    )
    .is_err());
    assert_eq!(i, 64);
}
#[test]
fn refuses_missing_mismatched_and_dynamic_declarations() {
    assert!(build_declaration_closure(
        &["cem-a".into()],
        TemplateArtifactSourceMapMode::Prod,
        |_| Err("missing dependency".into())
    )
    .is_err());
    assert!(build_declaration_closure(
        &["cem-a".into()],
        TemplateArtifactSourceMapMode::Prod,
        |_| Ok(source("cem-wrong", "Hello"))
    )
    .is_err());
    assert!(build_declaration_closure(
        &["cem-a".into()],
        TemplateArtifactSourceMapMode::Prod,
        |tag| Ok(source(tag, "{cem-element @tag=cem-dynamic}"))
    )
    .is_err());
}
